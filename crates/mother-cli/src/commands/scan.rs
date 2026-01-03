//! Scan command: Scan a repository and store in Neo4j
//!
//! This module implements a 3-phase scanning process:
//! 1. Phase 1: Discover files, open in LSP, create in Neo4j
//! 2. Phase 2: Extract symbols from LSP, enrich with hover, store in Neo4j
//! 3. Phase 3: Extract references, create symbol-to-symbol edges

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use mother_core::graph::convert::convert_symbols;
use mother_core::graph::model::{ScanRun, SymbolNode};
use mother_core::graph::neo4j::{Neo4jClient, Neo4jConfig};
use mother_core::lsp::{
    LspClient, LspServerManager, LspSymbol,
    collect_symbol_positions as collect_lsp_symbol_positions,
    flatten_symbols as flatten_lsp_symbols,
};
use mother_core::scanner::{DiscoveredFile, Language, Scanner};
use tracing::info;

// ============================================================================
// Types
// ============================================================================

/// A file that needs symbol extraction (output from Phase 1)
struct FileToProcess {
    path: std::path::PathBuf,
    file_uri: String,
    content_hash: String,
    language: Language,
}

/// Symbol position info for reference extraction (output from Phase 2)
struct SymbolInfo {
    id: String,
    file_uri: String,
    start_line: u32,
    end_line: u32,
    start_col: u32,
    language: Language,
}

/// Results from Phase 1
struct Phase1Result {
    files_to_process: Vec<FileToProcess>,
    new_file_count: usize,
    reused_file_count: usize,
}

/// Results from Phase 2
struct Phase2Result {
    symbols: Vec<SymbolInfo>,
    symbol_count: usize,
}

/// Results from Phase 3
struct Phase3Result {
    reference_count: usize,
}

// ============================================================================
// Main entry point
// ============================================================================

/// Run the scan command
///
/// # Errors
/// Returns an error if scanning or Neo4j operations fail.
pub async fn run(
    path: &Path,
    neo4j_uri: &str,
    neo4j_user: &str,
    neo4j_password: &str,
    version: Option<&str>,
) -> Result<()> {
    info!("Scanning repository: {}", path.display());

    let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let (scan_run, commit_sha) = create_scan_run(&abs_path, version);

    log_scan_run_info(&scan_run, &commit_sha);

    let client = connect_neo4j(neo4j_uri, neo4j_user, neo4j_password).await?;

    if !client.create_scan_run(&scan_run).await? {
        info!("✓ Commit already scanned, linked scan run to existing data");
        return Ok(());
    }

    execute_scan(&abs_path, &client, &commit_sha).await
}

/// Execute the scan workflow after determining a new commit needs scanning
async fn execute_scan(abs_path: &Path, client: &Neo4jClient, commit_sha: &str) -> Result<()> {
    info!("New commit detected, scanning files...");

    let files: Vec<_> = Scanner::new(abs_path).scan().collect();
    info!("Found {} files to process", files.len());

    let mut lsp_manager = LspServerManager::new(abs_path);

    let phase1 = phase1_open_files(&files, client, &mut lsp_manager, commit_sha).await?;
    let phase2 = phase2_extract_symbols(&phase1.files_to_process, client, &mut lsp_manager).await?;
    let phase3 = phase3_extract_references(&phase2.symbols, client, &mut lsp_manager).await?;

    shutdown_lsp(&mut lsp_manager).await;

    log_scan_summary(&phase1, &phase2, &phase3);
    Ok(())
}

fn log_scan_summary(phase1: &Phase1Result, phase2: &Phase2Result, phase3: &Phase3Result) {
    info!(
        "✓ Scan completed: {} new files, {} reused, {} symbols, {} references",
        phase1.new_file_count,
        phase1.reused_file_count,
        phase2.symbol_count,
        phase3.reference_count
    );
}

fn create_scan_run(abs_path: &Path, version: Option<&str>) -> (ScanRun, String) {
    let mut scan_run = ScanRun::new(abs_path.display().to_string()).with_git_info();
    if let Some(v) = version {
        scan_run = scan_run.with_version(v);
    }
    let commit_sha = scan_run.commit_sha.clone().unwrap_or_default();
    (scan_run, commit_sha)
}

fn log_scan_run_info(scan_run: &ScanRun, commit_sha: &str) {
    info!(
        "Created scan run: {} (commit: {}, branch: {:?})",
        scan_run.id,
        if commit_sha.is_empty() {
            "none"
        } else {
            commit_sha
        },
        scan_run.branch
    );
}

async fn connect_neo4j(uri: &str, user: &str, password: &str) -> Result<Neo4jClient> {
    let config = Neo4jConfig::new(uri, user, password);
    Ok(Neo4jClient::connect(&config).await?)
}

async fn shutdown_lsp(lsp_manager: &mut LspServerManager) {
    if let Err(e) = lsp_manager.shutdown_all().await {
        tracing::warn!("Failed to shutdown LSP servers: {}", e);
    }
}

// ============================================================================
// Phase 1: Open files in LSP and create in Neo4j
// ============================================================================

async fn phase1_open_files(
    files: &[DiscoveredFile],
    client: &Neo4jClient,
    lsp_manager: &mut LspServerManager,
    commit_sha: &str,
) -> Result<Phase1Result> {
    info!("Phase 1: Opening files in LSP...");

    let mut files_to_process: Vec<FileToProcess> = Vec::new();
    let mut new_file_count = 0;
    let mut reused_file_count = 0;

    for file in files {
        match process_file_for_phase1(file, client, lsp_manager, commit_sha).await {
            Ok(Some(file_to_process)) => {
                new_file_count += 1;
                files_to_process.push(file_to_process);
            }
            Ok(None) => {
                reused_file_count += 1;
            }
            Err(e) => {
                tracing::debug!("Skipping file {}: {}", file.path.display(), e);
            }
        }
    }

    Ok(Phase1Result {
        files_to_process,
        new_file_count,
        reused_file_count,
    })
}

/// Process a single file for phase 1. Returns Ok(Some) for new files, Ok(None) for reused.
async fn process_file_for_phase1(
    file: &DiscoveredFile,
    client: &Neo4jClient,
    lsp_manager: &mut LspServerManager,
    commit_sha: &str,
) -> Result<Option<FileToProcess>> {
    let hash = file.compute_hash()?;
    let file_path_str = file.path.display().to_string();

    // Check if file already exists in Neo4j
    let content_hash = match client
        .create_file_if_new(
            &file_path_str,
            &hash,
            &file.language.to_string(),
            commit_sha,
        )
        .await?
    {
        Some(h) => h,
        None => return Ok(None), // File reused
    };

    // Get LSP client and open file
    let lsp_client = lsp_manager.get_client(file.language).await?;
    let file_uri = format!("file://{}", file.path.display());
    let file_content = std::fs::read_to_string(&file.path)?;
    lsp_client
        .did_open(&file_uri, &file.language.to_string(), &file_content)
        .await?;

    Ok(Some(FileToProcess {
        path: file.path.clone(),
        file_uri,
        content_hash,
        language: file.language,
    }))
}

// ============================================================================
// Phase 2: Extract symbols from files
// ============================================================================

async fn phase2_extract_symbols(
    files: &[FileToProcess],
    client: &Neo4jClient,
    lsp_manager: &mut LspServerManager,
) -> Result<Phase2Result> {
    info!("Phase 2: Extracting symbols from {} files...", files.len());

    let mut symbol_count = 0;
    let mut all_symbols: Vec<SymbolInfo> = Vec::new();

    for file_info in files {
        match process_file_for_phase2(file_info, client, lsp_manager).await {
            Ok((symbols, count)) => {
                all_symbols.extend(symbols);
                symbol_count += count;
            }
            Err(e) => {
                tracing::debug!("Skipping file {}: {}", file_info.path.display(), e);
            }
        }
    }

    Ok(Phase2Result {
        symbols: all_symbols,
        symbol_count,
    })
}

/// Process a single file for phase 2 (symbol extraction)
async fn process_file_for_phase2(
    file_info: &FileToProcess,
    client: &Neo4jClient,
    lsp_manager: &mut LspServerManager,
) -> Result<(Vec<SymbolInfo>, usize)> {
    let lsp_client = lsp_manager.get_client(file_info.language).await?;
    let lsp_symbols = lsp_client.document_symbols(&file_info.file_uri).await?;

    // Convert LSP symbols to graph nodes
    let mut symbols = convert_symbols(&lsp_symbols, &file_info.path);
    let file_symbol_count = symbols.len();

    // Enrich symbols with hover information
    enrich_symbols_with_hover(&mut symbols, &lsp_symbols, lsp_client, &file_info.file_uri).await;

    log_file_symbols(file_info, file_symbol_count, lsp_symbols.len());

    // Store symbols in Neo4j
    client
        .create_symbols_batch(&symbols, &file_info.content_hash)
        .await?;

    // Collect symbol info for reference extraction
    let mut symbol_infos = Vec::new();
    collect_symbol_info(
        &lsp_symbols,
        &symbols,
        &file_info.file_uri,
        file_info.language,
        &mut symbol_infos,
    );

    Ok((symbol_infos, file_symbol_count))
}

fn log_file_symbols(file_info: &FileToProcess, symbol_count: usize, lsp_count: usize) {
    tracing::info!(
        "  {} → {} symbols (from {} lsp symbols)",
        file_info
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
        symbol_count,
        lsp_count
    );
}

/// Enrich symbols with hover information from LSP
async fn enrich_symbols_with_hover(
    symbols: &mut [SymbolNode],
    lsp_symbols: &[LspSymbol],
    lsp_client: &mut LspClient,
    file_uri: &str,
) {
    let lsp_positions = collect_lsp_symbol_positions(lsp_symbols);

    for (i, symbol) in symbols.iter_mut().enumerate() {
        let col = lsp_positions.get(i).map(|p| p.1).unwrap_or(0);
        // Use 0-indexed line for hover (symbol.start_line is 1-indexed)
        if let Ok(Some(hover_content)) =
            lsp_client.hover(file_uri, symbol.start_line - 1, col).await
        {
            symbol.doc_comment = Some(hover_content);
        }
    }
}

// ============================================================================
// Phase 3: Extract references and create edges
// ============================================================================

async fn phase3_extract_references(
    symbols: &[SymbolInfo],
    client: &Neo4jClient,
    lsp_manager: &mut LspServerManager,
) -> Result<Phase3Result> {
    info!(
        "Phase 3: Extracting references for {} symbols...",
        symbols.len()
    );

    let symbols_by_file = build_symbol_lookup_table(symbols);
    let mut reference_count = 0;

    for symbol_info in symbols {
        reference_count +=
            process_symbol_references(symbol_info, &symbols_by_file, client, lsp_manager).await;
    }

    Ok(Phase3Result { reference_count })
}

/// Process references for a single symbol
async fn process_symbol_references(
    symbol_info: &SymbolInfo,
    symbols_by_file: &HashMap<String, Vec<(String, u32, u32)>>,
    client: &Neo4jClient,
    lsp_manager: &mut LspServerManager,
) -> usize {
    let lsp_client = match lsp_manager.get_client(symbol_info.language).await {
        Ok(c) => c,
        Err(_) => return 0,
    };

    let refs = match lsp_client
        .references(
            &symbol_info.file_uri,
            symbol_info.start_line,
            symbol_info.start_col,
            true,
        )
        .await
    {
        Ok(r) => r,
        Err(_) => return 0,
    };

    create_reference_edges(&refs, symbol_info, symbols_by_file, client).await
}

/// Build a lookup table from file path to symbols in that file
fn build_symbol_lookup_table(symbols: &[SymbolInfo]) -> HashMap<String, Vec<(String, u32, u32)>> {
    let mut symbols_by_file: HashMap<String, Vec<(String, u32, u32)>> = HashMap::new();

    for sym in symbols {
        let file_path = sym
            .file_uri
            .strip_prefix("file://")
            .unwrap_or(&sym.file_uri);
        symbols_by_file
            .entry(file_path.to_string())
            .or_default()
            .push((sym.id.clone(), sym.start_line, sym.end_line));
    }

    symbols_by_file
}

/// Create reference edges for a symbol's references
async fn create_reference_edges(
    refs: &[mother_core::lsp::LspReference],
    symbol_info: &SymbolInfo,
    symbols_by_file: &HashMap<String, Vec<(String, u32, u32)>>,
    client: &Neo4jClient,
) -> usize {
    let mut count = 0;

    for reference in refs {
        if let Some(from_id) = find_containing_symbol(reference, symbols_by_file)
            && from_id != symbol_info.id
            && create_edge(client, &from_id, &symbol_info.id, reference).await
        {
            count += 1;
        }
    }

    count
}

/// Find the symbol that contains a reference location
fn find_containing_symbol(
    reference: &mother_core::lsp::LspReference,
    symbols_by_file: &HashMap<String, Vec<(String, u32, u32)>>,
) -> Option<String> {
    let ref_file = reference.file.display().to_string();
    let ref_line = reference.line;

    symbols_by_file.get(&ref_file).and_then(|symbols| {
        symbols
            .iter()
            .filter(|(_, start, end)| ref_line >= *start && ref_line <= *end)
            .min_by_key(|(_, start, end)| end - start)
            .map(|(id, _, _)| id.clone())
    })
}

/// Create a single reference edge in Neo4j
async fn create_edge(
    client: &Neo4jClient,
    from_id: &str,
    to_id: &str,
    reference: &mother_core::lsp::LspReference,
) -> bool {
    client
        .create_symbol_reference(from_id, to_id, reference.line, reference.start_col)
        .await
        .is_ok()
}

// ============================================================================
// Helper functions
// ============================================================================

/// Collect position info from LSP symbols, matching them to graph nodes by traversal order
fn collect_symbol_info(
    lsp_symbols: &[LspSymbol],
    graph_symbols: &[SymbolNode],
    file_uri: &str,
    language: Language,
    out: &mut Vec<SymbolInfo>,
) {
    let flat_lsp = flatten_lsp_symbols(lsp_symbols);

    for (lsp_sym, graph_sym) in flat_lsp.iter().zip(graph_symbols.iter()) {
        out.push(SymbolInfo {
            id: graph_sym.id.clone(),
            file_uri: file_uri.to_string(),
            start_line: lsp_sym.start_line,
            end_line: lsp_sym.end_line,
            start_col: lsp_sym.start_col,
            language,
        });
    }
}

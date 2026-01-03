//! Scan command: Scan a repository and store in Neo4j

use std::path::Path;

use anyhow::Result;
use mother_core::graph::convert::convert_symbols;
use mother_core::graph::model::SymbolNode;
use mother_core::graph::neo4j::{Neo4jClient, Neo4jConfig};
use mother_core::lsp::LspServerManager;
use mother_core::scanner::{Scanner, compute_file_hash};
use mother_core::version::ScanRun;
use tracing::info;

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

    // Canonicalize the path for LSP
    let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    // Create scan run with git info
    let mut scan_run = ScanRun::new(abs_path.display().to_string()).with_git_info();
    if let Some(v) = version {
        scan_run = scan_run.with_version(v);
    }

    let commit_sha = scan_run.commit_sha.clone().unwrap_or_default();
    info!(
        "Created scan run: {} (commit: {}, branch: {:?})",
        scan_run.id,
        if commit_sha.is_empty() {
            "none"
        } else {
            &commit_sha
        },
        scan_run.branch
    );

    // Connect to Neo4j
    let config = Neo4jConfig::new(neo4j_uri, neo4j_user, neo4j_password);
    let client = Neo4jClient::connect(&config).await?;

    // Create scan run and check if commit is new
    let is_new_commit = client.create_scan_run(&scan_run).await?;

    if !is_new_commit {
        info!("✓ Commit already scanned, linked scan run to existing data");
        return Ok(());
    }

    info!("New commit detected, scanning files...");

    // Scan files
    let scanner = Scanner::new(&abs_path);
    let files: Vec<_> = scanner.scan().collect();

    info!("Found {} files to process", files.len());

    // Initialize LSP manager
    let mut lsp_manager = LspServerManager::new(&abs_path);

    // Phase 1: Create files in Neo4j and open in LSP
    // We track files that need symbol extraction
    info!("Phase 1: Opening files in LSP...");

    struct FileToProcess {
        path: std::path::PathBuf,
        file_uri: String,
        content_hash: String,
        language: mother_core::scanner::Language,
    }

    let mut files_to_process: Vec<FileToProcess> = Vec::new();
    let mut new_file_count = 0;
    let mut reused_file_count = 0;

    for file in &files {
        // Compute file hash
        let hash = match compute_file_hash(&file.path) {
            Ok(h) => h,
            Err(e) => {
                tracing::warn!("Failed to hash {}: {}", file.path.display(), e);
                continue;
            }
        };

        let file_path_str = file.path.display().to_string();

        // Create or link file
        let content_hash = match client
            .create_file_if_new(
                &file_path_str,
                &hash,
                &file.language.to_string(),
                &commit_sha,
            )
            .await?
        {
            Some(hash) => {
                new_file_count += 1;
                hash
            }
            None => {
                reused_file_count += 1;
                tracing::debug!("Reused file: {} (hash: {})", file_path_str, hash);
                continue;
            }
        };

        // Extract symbols via LSP - get client (this starts the server if needed)
        let lsp_client = match lsp_manager.get_client(file.language).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to get LSP client for {:?}: {}", file.language, e);
                continue;
            }
        };

        // Open the file in LSP
        let file_uri = format!("file://{}", file.path.display());
        let file_content = match std::fs::read_to_string(&file.path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to read {}: {}", file.path.display(), e);
                continue;
            }
        };

        if let Err(e) = lsp_client
            .did_open(&file_uri, &file.language.to_string(), &file_content)
            .await
        {
            tracing::warn!("Failed to open file in LSP: {}", e);
            continue;
        }

        files_to_process.push(FileToProcess {
            path: file.path.clone(),
            file_uri,
            content_hash,
            language: file.language,
        });
    }

    // Phase 2: Query symbols for all files
    // Note: Indexing wait is handled by the LSP client via progress notifications
    info!(
        "Phase 2: Extracting symbols from {} files...",
        files_to_process.len()
    );
    let mut symbol_count = 0;

    // Track symbols for reference extraction
    let mut all_symbols: Vec<SymbolInfo> = Vec::new();

    for file_info in &files_to_process {
        let lsp_client = match lsp_manager.get_client(file_info.language).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to get LSP client: {}", e);
                continue;
            }
        };

        // Get document symbols
        let lsp_symbols = match lsp_client.document_symbols(&file_info.file_uri).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(
                    "Failed to get symbols for {}: {}",
                    file_info.path.display(),
                    e
                );
                continue;
            }
        };

        // Convert LSP symbols to graph nodes
        let symbols = convert_symbols(&lsp_symbols, &file_info.path);
        let file_symbol_count = symbols.len();

        tracing::info!(
            "  {} → {} symbols (from {} lsp symbols)",
            file_info
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy(),
            file_symbol_count,
            lsp_symbols.len()
        );

        // Store symbols in Neo4j
        if let Err(e) = client
            .create_symbols_batch(&symbols, &file_info.content_hash)
            .await
        {
            tracing::warn!(
                "Failed to store symbols for {}: {}",
                file_info.path.display(),
                e
            );
            continue;
        }

        // Track symbols for reference extraction
        // We need to collect position info from the original LSP symbols
        collect_symbol_positions(
            &lsp_symbols,
            &symbols,
            &file_info.file_uri,
            file_info.language,
            &mut all_symbols,
        );

        symbol_count += file_symbol_count;
    }

    // Phase 3: Extract references for each symbol and create Symbol→Symbol edges
    info!(
        "Phase 3: Extracting references for {} symbols...",
        all_symbols.len()
    );
    let mut reference_count = 0;

    // Build lookup table: file_path → list of (symbol_id, start_line, end_line)
    // This lets us find which symbol contains a given reference line
    use std::collections::HashMap;
    let mut symbols_by_file: HashMap<String, Vec<(String, u32, u32)>> = HashMap::new();
    for sym in &all_symbols {
        // Convert file:// URI to path
        let file_path = sym
            .file_uri
            .strip_prefix("file://")
            .unwrap_or(&sym.file_uri);
        symbols_by_file
            .entry(file_path.to_string())
            .or_default()
            .push((sym.id.clone(), sym.start_line, sym.end_line));
    }

    for symbol_info in &all_symbols {
        let lsp_client = match lsp_manager.get_client(symbol_info.language).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to get LSP client: {}", e);
                continue;
            }
        };

        // Query references for this symbol
        let refs = match lsp_client
            .references(
                &symbol_info.file_uri,
                symbol_info.start_line,
                symbol_info.start_col,
                true, // include declaration
            )
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::debug!("Failed to get references: {}", e);
                continue;
            }
        };

        // For each reference, find the containing symbol and create an edge
        for reference in &refs {
            let ref_file = reference.file.display().to_string();
            let ref_line = reference.line;

            // Find which symbol in the reference file contains this line
            if let Some(symbols_in_file) = symbols_by_file.get(&ref_file) {
                // Find the smallest (most specific) symbol that contains this line
                let containing_symbol = symbols_in_file
                    .iter()
                    .filter(|(_, start, end)| ref_line >= *start && ref_line <= *end)
                    .min_by_key(|(_, start, end)| end - start);

                if let Some((from_id, _, _)) = containing_symbol {
                    // Skip self-references (LSP may return definition as a reference)
                    if from_id == &symbol_info.id {
                        continue;
                    }

                    // Create edge: from_symbol -[:REFERENCES]-> to_symbol (symbol_info)
                    if let Err(e) = client
                        .create_symbol_reference(
                            from_id,
                            &symbol_info.id,
                            ref_line,
                            reference.start_col,
                        )
                        .await
                    {
                        tracing::debug!("Failed to create reference edge: {}", e);
                        continue;
                    }
                    reference_count += 1;
                }
            }
        }
    }

    // Shutdown LSP servers
    if let Err(e) = lsp_manager.shutdown_all().await {
        tracing::warn!("Failed to shutdown LSP servers: {}", e);
    }

    info!(
        "✓ Scan completed: {} new files, {} reused, {} symbols, {} references",
        new_file_count, reused_file_count, symbol_count, reference_count
    );
    Ok(())
}

/// Collect position info from LSP symbols, matching them to graph nodes by name order
fn collect_symbol_positions(
    lsp_symbols: &[mother_core::lsp::LspSymbol],
    graph_symbols: &[SymbolNode],
    file_uri: &str,
    language: mother_core::scanner::Language,
    out: &mut Vec<SymbolInfo>,
) {
    // The graph_symbols are flattened in the same order as LSP symbols traversal
    // We'll flatten LSP symbols to match
    fn flatten_lsp(symbols: &[mother_core::lsp::LspSymbol]) -> Vec<&mother_core::lsp::LspSymbol> {
        let mut result = Vec::new();
        for sym in symbols {
            result.push(sym);
            result.extend(flatten_lsp(&sym.children));
        }
        result
    }

    let flat_lsp = flatten_lsp(lsp_symbols);

    // Match by index (they should be in the same order)
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

struct SymbolInfo {
    id: String,
    file_uri: String,
    start_line: u32,
    end_line: u32,
    start_col: u32,
    language: mother_core::scanner::Language,
}

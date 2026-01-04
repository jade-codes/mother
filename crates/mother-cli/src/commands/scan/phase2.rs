//! Phase 2: Extract symbols from files

use anyhow::Result;
use mother_core::graph::convert::convert_symbols;
use mother_core::graph::model::SymbolNode;
use mother_core::graph::neo4j::Neo4jClient;
use mother_core::lsp::{
    collect_symbol_positions as collect_lsp_symbol_positions,
    flatten_symbols as flatten_lsp_symbols, LspClient, LspServerManager, LspSymbol,
};
use mother_core::scanner::Language;
use tracing::info;

use super::{FileToProcess, SymbolInfo};

/// Results from Phase 2
pub struct Phase2Result {
    pub(crate) symbols: Vec<SymbolInfo>,
    pub symbol_count: usize,
    pub error_count: usize,
}

/// Run Phase 2: Extract symbols from files
pub async fn run(
    files: &[FileToProcess],
    client: &Neo4jClient,
    lsp_manager: &mut LspServerManager,
) -> Result<Phase2Result> {
    info!("Phase 2: Extracting symbols from {} files...", files.len());

    let mut result = Phase2Result {
        symbols: Vec::new(),
        symbol_count: 0,
        error_count: 0,
    };

    for file_info in files {
        let outcome = process_file(file_info, client, lsp_manager).await;
        handle_file_result(outcome, file_info, &mut result);
    }

    log_phase2_errors(&result);
    Ok(result)
}

/// Handle the result of processing a single file
fn handle_file_result(
    outcome: Result<(Vec<SymbolInfo>, usize)>,
    file_info: &FileToProcess,
    result: &mut Phase2Result,
) {
    match outcome {
        Ok((symbols, count)) => {
            result.symbols.extend(symbols);
            result.symbol_count += count;
        }
        Err(e) => {
            result.error_count += 1;
            tracing::warn!(
                "Failed to extract symbols from {}: {}",
                file_info.path.display(),
                e
            );
        }
    }
}

/// Log error summary for phase 2
fn log_phase2_errors(result: &Phase2Result) {
    if result.error_count > 0 {
        tracing::warn!(
            "Phase 2: {} files failed symbol extraction",
            result.error_count
        );
    }
}

/// Process a single file for phase 2 (symbol extraction)
async fn process_file(
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

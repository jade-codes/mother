//! Phase 3: Extract references and create edges

use std::collections::HashMap;

use anyhow::Result;
use mother_core::graph::model::{Edge, EdgeKind};
use mother_core::graph::neo4j::Neo4jClient;
use mother_core::lsp::LspServerManager;
use tracing::info;

use super::SymbolInfo;

/// Results from Phase 3
pub struct Phase3Result {
    pub reference_count: usize,
    pub error_count: usize,
}

/// Run Phase 3: Extract references and create edges
pub async fn run(
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
    let mut error_count = 0;

    for symbol_info in symbols {
        let (refs, errors) =
            process_symbol_references(symbol_info, &symbols_by_file, client, lsp_manager).await;
        reference_count += refs;
        error_count += errors;
    }

    if error_count > 0 {
        tracing::warn!("Phase 3: {} reference lookups failed", error_count);
    }

    Ok(Phase3Result {
        reference_count,
        error_count,
    })
}

/// Process references for a single symbol
/// Returns (reference_count, error_count)
async fn process_symbol_references(
    symbol_info: &SymbolInfo,
    symbols_by_file: &HashMap<String, Vec<(String, u32, u32)>>,
    client: &Neo4jClient,
    lsp_manager: &mut LspServerManager,
) -> (usize, usize) {
    let lsp_client = match lsp_manager.get_client(symbol_info.language).await {
        Ok(c) => c,
        Err(_) => return (0, 1),
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
        Err(_) => return (0, 1),
    };

    (
        create_reference_edges(&refs, symbol_info, symbols_by_file, client).await,
        0,
    )
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
        if let Some(from_id) = find_containing_symbol(reference, symbols_by_file) {
            if from_id != symbol_info.id
                && create_reference_edge(client, &from_id, &symbol_info.id, reference).await
            {
                count += 1;
            }
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
async fn create_reference_edge(
    client: &Neo4jClient,
    from_id: &str,
    to_id: &str,
    reference: &mother_core::lsp::LspReference,
) -> bool {
    let edge = Edge {
        source_id: from_id.to_string(),
        target_id: to_id.to_string(),
        kind: EdgeKind::References,
        line: Some(reference.line),
        column: Some(reference.start_col),
    };
    client.create_edge(&edge).await.is_ok()
}

#[cfg(test)]
mod tests;

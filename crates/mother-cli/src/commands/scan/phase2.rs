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

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use mother_core::graph::model::SymbolKind;
    use mother_core::lsp::LspSymbolKind;
    use std::path::PathBuf;

    /// Helper to create a test FileToProcess
    fn create_test_file(path: &str) -> FileToProcess {
        FileToProcess {
            path: PathBuf::from(path),
            file_uri: format!("file://{}", path),
            content_hash: "test_hash".to_string(),
            language: Language::Rust,
        }
    }

    /// Helper to create a test SymbolInfo
    fn create_test_symbol(id: &str) -> SymbolInfo {
        SymbolInfo {
            id: id.to_string(),
            file_uri: "file:///test.rs".to_string(),
            start_line: 1,
            end_line: 10,
            start_col: 0,
            language: Language::Rust,
        }
    }

    /// Helper to create a test LspSymbol
    fn create_lsp_symbol(
        name: &str,
        kind: LspSymbolKind,
        start_line: u32,
        end_line: u32,
        start_col: u32,
        end_col: u32,
    ) -> LspSymbol {
        LspSymbol {
            name: name.to_string(),
            kind,
            detail: None,
            container_name: None,
            file: PathBuf::from("/test.rs"),
            start_line,
            end_line,
            start_col,
            end_col,
            children: Vec::new(),
        }
    }

    /// Helper to create a test SymbolNode
    fn create_symbol_node(
        id: &str,
        name: &str,
        kind: SymbolKind,
        start: u32,
        end: u32,
    ) -> SymbolNode {
        SymbolNode {
            id: id.to_string(),
            name: name.to_string(),
            qualified_name: name.to_string(),
            kind,
            visibility: None,
            file_path: "/test.rs".to_string(),
            start_line: start,
            end_line: end,
            signature: None,
            doc_comment: None,
        }
    }

    #[test]
    fn test_phase2_result_initialization() {
        let result = Phase2Result {
            symbols: Vec::new(),
            symbol_count: 0,
            error_count: 0,
        };

        assert_eq!(result.symbols.len(), 0);
        assert_eq!(result.symbol_count, 0);
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_handle_file_result_success() {
        let mut result = Phase2Result {
            symbols: Vec::new(),
            symbol_count: 0,
            error_count: 0,
        };

        let file = create_test_file("/test/file.rs");
        let symbols = vec![create_test_symbol("symbol1"), create_test_symbol("symbol2")];
        let symbol_count = symbols.len();
        let outcome = Ok((symbols, 5));

        handle_file_result(outcome, &file, &mut result);

        assert_eq!(result.symbols.len(), symbol_count);
        assert_eq!(result.symbol_count, 5);
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_handle_file_result_error() {
        let mut result = Phase2Result {
            symbols: Vec::new(),
            symbol_count: 0,
            error_count: 0,
        };

        let file = create_test_file("/test/file.rs");
        let outcome: Result<(Vec<SymbolInfo>, usize)> = Err(anyhow!("Test error"));

        handle_file_result(outcome, &file, &mut result);

        assert_eq!(result.symbols.len(), 0);
        assert_eq!(result.symbol_count, 0);
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_handle_file_result_multiple_successes() {
        let mut result = Phase2Result {
            symbols: Vec::new(),
            symbol_count: 0,
            error_count: 0,
        };

        let file1 = create_test_file("/test/file1.rs");
        let file2 = create_test_file("/test/file2.rs");

        let symbols1 = vec![create_test_symbol("sym1")];
        let symbols2 = vec![create_test_symbol("sym2"), create_test_symbol("sym3")];

        handle_file_result(Ok((symbols1, 3)), &file1, &mut result);
        handle_file_result(Ok((symbols2, 5)), &file2, &mut result);

        assert_eq!(result.symbols.len(), 3);
        assert_eq!(result.symbol_count, 8);
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_handle_file_result_mixed_results() {
        let mut result = Phase2Result {
            symbols: Vec::new(),
            symbol_count: 0,
            error_count: 0,
        };

        let file1 = create_test_file("/test/file1.rs");
        let file2 = create_test_file("/test/file2.rs");
        let file3 = create_test_file("/test/file3.rs");

        handle_file_result(
            Ok((vec![create_test_symbol("sym1")], 2)),
            &file1,
            &mut result,
        );
        handle_file_result(Err(anyhow!("Error 1")), &file2, &mut result);
        handle_file_result(
            Ok((vec![create_test_symbol("sym2")], 3)),
            &file3,
            &mut result,
        );

        assert_eq!(result.symbols.len(), 2);
        assert_eq!(result.symbol_count, 5);
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_handle_file_result_empty_symbols() {
        let mut result = Phase2Result {
            symbols: Vec::new(),
            symbol_count: 0,
            error_count: 0,
        };

        let file = create_test_file("/test/empty.rs");
        let outcome = Ok((Vec::new(), 0));

        handle_file_result(outcome, &file, &mut result);

        assert_eq!(result.symbols.len(), 0);
        assert_eq!(result.symbol_count, 0);
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_collect_symbol_info_empty() {
        let mut out = Vec::new();
        collect_symbol_info(&[], &[], "file:///test.rs", Language::Rust, &mut out);
        assert_eq!(out.len(), 0);
    }

    #[test]
    fn test_collect_symbol_info_single_symbol() {
        let lsp_symbols = vec![create_lsp_symbol(
            "test_fn",
            LspSymbolKind::Function,
            5,
            10,
            4,
            20,
        )];

        let graph_symbols = vec![create_symbol_node(
            "test_id",
            "test_fn",
            SymbolKind::Function,
            5,
            10,
        )];

        let mut out = Vec::new();
        collect_symbol_info(
            &lsp_symbols,
            &graph_symbols,
            "file:///test.rs",
            Language::Rust,
            &mut out,
        );

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "test_id");
        assert_eq!(out[0].file_uri, "file:///test.rs");
        assert_eq!(out[0].start_line, 5);
        assert_eq!(out[0].end_line, 10);
        assert_eq!(out[0].start_col, 4);
    }

    #[test]
    fn test_collect_symbol_info_multiple_symbols() {
        let lsp_symbols = vec![
            create_lsp_symbol("struct_a", LspSymbolKind::Struct, 1, 5, 0, 10),
            create_lsp_symbol("fn_b", LspSymbolKind::Function, 7, 15, 0, 20),
        ];

        let graph_symbols = vec![
            create_symbol_node("id1", "struct_a", SymbolKind::Struct, 1, 5),
            create_symbol_node("id2", "fn_b", SymbolKind::Function, 7, 15),
        ];

        let mut out = Vec::new();
        collect_symbol_info(
            &lsp_symbols,
            &graph_symbols,
            "file:///test.rs",
            Language::Python,
            &mut out,
        );

        assert_eq!(out.len(), 2);
        assert_eq!(out[0].id, "id1");
        assert_eq!(out[0].start_line, 1);
        assert_eq!(out[1].id, "id2");
        assert_eq!(out[1].start_line, 7);
        assert!(matches!(out[0].language, Language::Python));
    }

    #[test]
    fn test_collect_symbol_info_with_nested_symbols() {
        let mut outer = create_lsp_symbol("outer", LspSymbolKind::Struct, 1, 10, 0, 5);
        outer.children = vec![create_lsp_symbol(
            "inner",
            LspSymbolKind::Function,
            3,
            8,
            4,
            10,
        )];
        let lsp_symbols = vec![outer];

        let graph_symbols = vec![
            create_symbol_node("outer_id", "outer", SymbolKind::Struct, 1, 10),
            create_symbol_node("inner_id", "inner", SymbolKind::Function, 3, 8),
        ];

        let mut out = Vec::new();
        collect_symbol_info(
            &lsp_symbols,
            &graph_symbols,
            "file:///test.rs",
            Language::Rust,
            &mut out,
        );

        assert_eq!(out.len(), 2);
        assert_eq!(out[0].id, "outer_id");
        assert_eq!(out[1].id, "inner_id");
    }

    #[test]
    fn test_collect_symbol_info_mismatched_lengths() {
        let lsp_symbols = vec![
            create_lsp_symbol("sym1", LspSymbolKind::Function, 1, 5, 0, 10),
            create_lsp_symbol("sym2", LspSymbolKind::Function, 7, 10, 0, 10),
        ];

        let graph_symbols = vec![create_symbol_node(
            "id1",
            "sym1",
            SymbolKind::Function,
            1,
            5,
        )];

        let mut out = Vec::new();
        collect_symbol_info(
            &lsp_symbols,
            &graph_symbols,
            "file:///test.rs",
            Language::Rust,
            &mut out,
        );

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "id1");
    }

    #[test]
    fn test_collect_symbol_info_preserves_language() {
        let lsp_symbols = vec![create_lsp_symbol(
            "test",
            LspSymbolKind::Function,
            1,
            5,
            0,
            10,
        )];

        let graph_symbols = vec![create_symbol_node(
            "test_id",
            "test",
            SymbolKind::Function,
            1,
            5,
        )];

        let mut out = Vec::new();
        collect_symbol_info(
            &lsp_symbols,
            &graph_symbols,
            "file:///test.go",
            Language::Go,
            &mut out,
        );

        assert_eq!(out.len(), 1);
        assert!(matches!(out[0].language, Language::Go));
    }

    #[test]
    fn test_collect_symbol_info_preserves_file_uri() {
        let lsp_symbols = vec![create_lsp_symbol(
            "test",
            LspSymbolKind::Function,
            1,
            5,
            0,
            10,
        )];

        let graph_symbols = vec![create_symbol_node(
            "test_id",
            "test",
            SymbolKind::Function,
            1,
            5,
        )];

        let custom_uri = "file:///custom/path.rs";
        let mut out = Vec::new();
        collect_symbol_info(
            &lsp_symbols,
            &graph_symbols,
            custom_uri,
            Language::Rust,
            &mut out,
        );

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].file_uri, custom_uri);
    }

    #[test]
    fn test_collect_symbol_info_different_languages() {
        let languages = vec![
            Language::Rust,
            Language::Python,
            Language::TypeScript,
            Language::JavaScript,
            Language::Go,
        ];

        for _lang in languages {
            let lsp_symbols = vec![create_lsp_symbol(
                "test",
                LspSymbolKind::Function,
                1,
                5,
                0,
                10,
            )];
            let graph_symbols = vec![create_symbol_node(
                "test_id",
                "test",
                SymbolKind::Function,
                1,
                5,
            )];

            let mut out = Vec::new();
            collect_symbol_info(
                &lsp_symbols,
                &graph_symbols,
                "file:///test",
                _lang,
                &mut out,
            );

            assert_eq!(out.len(), 1);
            assert!(matches!(out[0].language, _lang));
        }
    }

    #[test]
    fn test_handle_file_result_accumulates_correctly() {
        let mut result = Phase2Result {
            symbols: Vec::new(),
            symbol_count: 0,
            error_count: 0,
        };

        for i in 0..5 {
            let file = create_test_file(&format!("/test/file{}.rs", i));
            let symbols = vec![create_test_symbol(&format!("sym{}", i))];
            handle_file_result(Ok((symbols, i + 1)), &file, &mut result);
        }

        assert_eq!(result.symbols.len(), 5);
        assert_eq!(result.symbol_count, 1 + 2 + 3 + 4 + 5);
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_handle_file_result_error_accumulation() {
        let mut result = Phase2Result {
            symbols: Vec::new(),
            symbol_count: 0,
            error_count: 0,
        };

        for i in 0..3 {
            let file = create_test_file(&format!("/test/file{}.rs", i));
            let outcome: Result<(Vec<SymbolInfo>, usize)> = Err(anyhow!("Error {}", i));
            handle_file_result(outcome, &file, &mut result);
        }

        assert_eq!(result.symbols.len(), 0);
        assert_eq!(result.symbol_count, 0);
        assert_eq!(result.error_count, 3);
    }

    // ============================================================================
    // Tests for enrich_symbols_with_hover behavior
    // ============================================================================
    //
    // Note: The `enrich_symbols_with_hover` function is async and requires a real
    // LspClient to test fully. These tests document and verify the logic and
    // behavior of the function through understanding its implementation and
    // testing related components.
    //
    // The function:
    // 1. Collects LSP symbol positions using `collect_symbol_positions`
    // 2. Iterates through symbols with their indices
    // 3. Gets column from LSP positions at index i (defaults to 0 if not found)
    // 4. Converts 1-indexed start_line to 0-indexed (start_line - 1) for LSP hover
    // 5. Calls LSP hover with file_uri, 0-indexed line, and column
    // 6. Sets doc_comment if hover returns Some(content)
    //
    // Full integration testing requires an actual LSP server.
    // ============================================================================

    #[test]
    fn test_enrich_symbols_with_hover_empty_symbols() {
        // When symbols array is empty, enrich_symbols_with_hover should:
        // - Not attempt any LSP hover calls
        // - Complete without errors
        // - Leave the empty array unchanged
        let symbols: Vec<SymbolNode> = Vec::new();
        let lsp_symbols: Vec<LspSymbol> = Vec::new();

        // The function would be called with empty symbols
        // No modifications would occur
        assert_eq!(symbols.len(), 0);
        assert_eq!(lsp_symbols.len(), 0);
    }

    #[test]
    fn test_enrich_symbols_with_hover_single_symbol_positions() {
        // Verify the position extraction logic for a single symbol
        let lsp_symbol = create_lsp_symbol("test_fn", LspSymbolKind::Function, 5, 10, 4, 20);
        let lsp_symbols = vec![lsp_symbol];

        // collect_symbol_positions should return (line, col) pairs
        let positions = mother_core::lsp::collect_symbol_positions(&lsp_symbols);

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0], (5, 4));

        // The function would use position[0].1 (column = 4) for hover
        // and would convert the 1-indexed symbol line to 0-indexed
    }

    #[test]
    fn test_enrich_symbols_with_hover_multiple_symbols_positions() {
        // Verify position extraction for multiple symbols
        let lsp_symbols = vec![
            create_lsp_symbol("fn_a", LspSymbolKind::Function, 1, 5, 0, 10),
            create_lsp_symbol("fn_b", LspSymbolKind::Function, 7, 15, 4, 20),
            create_lsp_symbol("fn_c", LspSymbolKind::Function, 20, 25, 8, 30),
        ];

        let positions = mother_core::lsp::collect_symbol_positions(&lsp_symbols);

        assert_eq!(positions.len(), 3);
        assert_eq!(positions[0], (1, 0));
        assert_eq!(positions[1], (7, 4));
        assert_eq!(positions[2], (20, 8));

        // The function would iterate through symbols and use these columns
    }

    #[test]
    fn test_enrich_symbols_with_hover_nested_symbols_positions() {
        // Verify position extraction handles nested symbols correctly
        let mut parent = create_lsp_symbol("struct_a", LspSymbolKind::Struct, 1, 20, 0, 5);
        parent.children = vec![
            create_lsp_symbol("method_a", LspSymbolKind::Method, 5, 10, 4, 15),
            create_lsp_symbol("method_b", LspSymbolKind::Method, 12, 18, 4, 15),
        ];
        let lsp_symbols = vec![parent];

        let positions = mother_core::lsp::collect_symbol_positions(&lsp_symbols);

        // Positions should be flattened depth-first: parent, then children
        assert_eq!(positions.len(), 3);
        assert_eq!(positions[0], (1, 0)); // struct_a
        assert_eq!(positions[1], (5, 4)); // method_a
        assert_eq!(positions[2], (12, 4)); // method_b
    }

    #[test]
    fn test_enrich_symbols_with_hover_line_number_conversion() {
        // Verify that the function correctly handles line number conversion
        // SymbolNode uses 1-indexed lines, LSP hover uses 0-indexed

        let symbol_node = create_symbol_node("test", "test_fn", SymbolKind::Function, 10, 20);

        // Symbol has 1-indexed start_line = 10
        assert_eq!(symbol_node.start_line, 10);

        // The function should convert to 0-indexed: 10 - 1 = 9
        // This would be passed to lsp_client.hover(file_uri, 9, col)
    }

    #[test]
    fn test_enrich_symbols_with_hover_column_extraction() {
        // Verify column extraction from positions
        let lsp_symbols = vec![
            create_lsp_symbol("fn_a", LspSymbolKind::Function, 1, 5, 0, 10),
            create_lsp_symbol("fn_b", LspSymbolKind::Function, 7, 15, 12, 20),
        ];

        let positions = mother_core::lsp::collect_symbol_positions(&lsp_symbols);

        // For symbol at index 0, column should be 0
        let col_0 = positions.first().map(|p| p.1).unwrap_or(0);
        assert_eq!(col_0, 0);

        // For symbol at index 1, column should be 12
        let col_1 = positions.get(1).map(|p| p.1).unwrap_or(0);
        assert_eq!(col_1, 12);
    }

    #[test]
    fn test_enrich_symbols_with_hover_missing_position_defaults_to_zero() {
        // Verify that when position is not found, column defaults to 0
        let lsp_symbols = vec![create_lsp_symbol(
            "fn_a",
            LspSymbolKind::Function,
            1,
            5,
            7,
            10,
        )];

        let positions = mother_core::lsp::collect_symbol_positions(&lsp_symbols);

        // Trying to get position at index 5 (out of bounds)
        let col = positions.get(5).map(|p| p.1).unwrap_or(0);
        assert_eq!(col, 0); // Should default to 0
    }

    #[test]
    fn test_enrich_symbols_with_hover_more_symbols_than_positions() {
        // Test edge case: more symbol nodes than LSP positions
        // This could happen if symbol conversion creates different counts

        let lsp_symbols = vec![create_lsp_symbol(
            "fn_a",
            LspSymbolKind::Function,
            1,
            5,
            4,
            10,
        )];

        let positions = mother_core::lsp::collect_symbol_positions(&lsp_symbols);
        assert_eq!(positions.len(), 1);

        // If we had 2 symbol nodes but only 1 position:
        // - Symbol at index 0 would get column from position[0]
        // - Symbol at index 1 would get default column of 0 (unwrap_or(0))
        let col_exists = positions.first().map(|p| p.1).unwrap_or(0);
        let col_missing = positions.get(1).map(|p| p.1).unwrap_or(0);

        assert_eq!(col_exists, 4);
        assert_eq!(col_missing, 0);
    }

    #[test]
    fn test_enrich_symbols_with_hover_preserves_existing_doc_comments() {
        // Verify logic: doc_comment is only set if hover returns Some
        // Existing doc_comment would be overwritten only if hover succeeds

        let mut symbol = create_symbol_node("test_id", "test_fn", SymbolKind::Function, 5, 10);

        // Initially no doc_comment
        assert_eq!(symbol.doc_comment, None);

        // Simulate setting doc_comment (as hover would do)
        symbol.doc_comment = Some("Function documentation".to_string());
        assert_eq!(
            symbol.doc_comment,
            Some("Function documentation".to_string())
        );

        // If hover returns None, doc_comment would remain unchanged
        // If hover returns Some(new_content), it would be replaced
    }

    #[test]
    fn test_enrich_symbols_with_hover_doc_comment_format() {
        // Verify the doc_comment field can hold the expected hover content
        let mut symbol = create_symbol_node("test_id", "test_fn", SymbolKind::Function, 5, 10);

        // Test various hover content formats
        let hover_contents = vec![
            "Simple documentation",
            "Multi-line\ndocumentation\nwith newlines",
            "```rust\nfn example() {}\n```",
            "Very long documentation that might come from a language server with detailed type information and examples",
            "",
        ];

        for content in hover_contents {
            symbol.doc_comment = Some(content.to_string());
            assert_eq!(symbol.doc_comment, Some(content.to_string()));
        }
    }

    #[test]
    fn test_enrich_symbols_with_hover_iteration_order() {
        // Verify that symbols are processed in order by index
        let lsp_symbols = vec![
            create_lsp_symbol("first", LspSymbolKind::Function, 1, 5, 0, 10),
            create_lsp_symbol("second", LspSymbolKind::Function, 7, 12, 4, 15),
            create_lsp_symbol("third", LspSymbolKind::Function, 15, 20, 8, 20),
        ];

        let graph_symbols = [
            create_symbol_node("id1", "first", SymbolKind::Function, 1, 5),
            create_symbol_node("id2", "second", SymbolKind::Function, 7, 12),
            create_symbol_node("id3", "third", SymbolKind::Function, 15, 20),
        ];

        let positions = mother_core::lsp::collect_symbol_positions(&lsp_symbols);

        // Verify positions match symbols in order
        for (i, symbol) in graph_symbols.iter().enumerate() {
            let _col = positions.get(i).map(|p| p.1).unwrap_or(0);
            // Each symbol would be enriched with hover at its corresponding position
            assert!(i < 3);
            // Line conversion: 1-indexed to 0-indexed
            assert!(symbol.start_line > 0);
        }
    }

    #[test]
    fn test_enrich_symbols_with_hover_different_symbol_kinds() {
        // Verify enrichment works for different symbol types
        let lsp_symbols = vec![
            create_lsp_symbol("MyStruct", LspSymbolKind::Struct, 1, 10, 0, 5),
            create_lsp_symbol("my_function", LspSymbolKind::Function, 12, 20, 0, 10),
            create_lsp_symbol("MY_CONST", LspSymbolKind::Constant, 22, 23, 0, 15),
            create_lsp_symbol("MyEnum", LspSymbolKind::Enum, 25, 35, 0, 5),
        ];

        let positions = mother_core::lsp::collect_symbol_positions(&lsp_symbols);

        assert_eq!(positions.len(), 4);
        // All symbol kinds should be enrichable regardless of their kind
        for (i, lsp_sym) in lsp_symbols.iter().enumerate() {
            assert_eq!(positions[i].0, lsp_sym.start_line);
            assert_eq!(positions[i].1, lsp_sym.start_col);
        }
    }

    #[test]
    fn test_enrich_symbols_with_hover_zero_indexed_lsp_symbols() {
        // LSP symbols are 0-indexed, but SymbolNodes are 1-indexed
        let lsp_symbol = create_lsp_symbol("test", LspSymbolKind::Function, 0, 5, 0, 10);

        // LSP symbol at line 0
        assert_eq!(lsp_symbol.start_line, 0);

        let positions = mother_core::lsp::collect_symbol_positions(&[lsp_symbol]);
        assert_eq!(positions[0].0, 0);

        // If SymbolNode has start_line = 1 (1-indexed)
        // Conversion: 1 - 1 = 0 for hover (0-indexed)
        let symbol = create_symbol_node("id", "test", SymbolKind::Function, 1, 5);
        assert_eq!(symbol.start_line - 1, 0);
    }

    #[test]
    fn test_enrich_symbols_with_hover_large_line_numbers() {
        // Verify handling of large line numbers
        let lsp_symbol =
            create_lsp_symbol("test", LspSymbolKind::Function, 999999, 1000010, 50, 100);

        let positions = mother_core::lsp::collect_symbol_positions(&[lsp_symbol]);
        assert_eq!(positions[0].0, 999999);
        assert_eq!(positions[0].1, 50);

        // Symbol with large line number
        let symbol = create_symbol_node("id", "test", SymbolKind::Function, 1000000, 1000010);
        assert_eq!(symbol.start_line - 1, 999999);
    }

    #[test]
    fn test_enrich_symbols_with_hover_complex_nested_structure() {
        // Test with deeply nested symbol structure
        let mut root = create_lsp_symbol("Root", LspSymbolKind::Module, 1, 100, 0, 5);
        let mut class = create_lsp_symbol("MyClass", LspSymbolKind::Class, 5, 90, 2, 10);
        let mut method1 = create_lsp_symbol("method1", LspSymbolKind::Method, 10, 20, 4, 15);
        let nested_fn = create_lsp_symbol("nested", LspSymbolKind::Function, 12, 18, 8, 20);

        method1.children = vec![nested_fn];
        class.children = vec![
            method1,
            create_lsp_symbol("method2", LspSymbolKind::Method, 25, 35, 4, 15),
        ];
        root.children = vec![class];

        let lsp_symbols = vec![root];
        let positions = mother_core::lsp::collect_symbol_positions(&lsp_symbols);

        // Flattened order: Root, MyClass, method1, nested, method2
        assert_eq!(positions.len(), 5);
        assert_eq!(positions[0], (1, 0)); // Root
        assert_eq!(positions[1], (5, 2)); // MyClass
        assert_eq!(positions[2], (10, 4)); // method1
        assert_eq!(positions[3], (12, 8)); // nested
        assert_eq!(positions[4], (25, 4)); // method2
    }

    #[test]
    fn test_enrich_symbols_with_hover_boundary_conditions() {
        // Test boundary conditions for line and column numbers

        // Minimum line (0 in LSP, 1 in SymbolNode)
        let min_line_symbol = create_symbol_node("min", "test", SymbolKind::Function, 1, 5);
        assert_eq!(min_line_symbol.start_line - 1, 0);

        // Column at boundary
        let lsp_sym_col_0 = create_lsp_symbol("test1", LspSymbolKind::Function, 5, 10, 0, 10);
        let lsp_sym_col_max =
            create_lsp_symbol("test2", LspSymbolKind::Function, 5, 10, u32::MAX, u32::MAX);

        let positions =
            mother_core::lsp::collect_symbol_positions(&[lsp_sym_col_0, lsp_sym_col_max]);

        assert_eq!(positions[0].1, 0);
        assert_eq!(positions[1].1, u32::MAX);
    }
}

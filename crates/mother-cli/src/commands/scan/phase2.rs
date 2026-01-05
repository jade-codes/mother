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

    #[test]
    fn test_collect_symbol_info_boundary_conditions() {
        // Test with line/column at 0
        let lsp_symbols = vec![create_lsp_symbol(
            "edge_fn",
            LspSymbolKind::Function,
            0,
            0,
            0,
            0,
        )];

        let graph_symbols = vec![create_symbol_node(
            "edge_id",
            "edge_fn",
            SymbolKind::Function,
            0,
            0,
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
        assert_eq!(out[0].start_line, 0);
        assert_eq!(out[0].end_line, 0);
        assert_eq!(out[0].start_col, 0);
    }

    #[test]
    fn test_collect_symbol_info_large_line_numbers() {
        let lsp_symbols = vec![create_lsp_symbol(
            "large_fn",
            LspSymbolKind::Function,
            999999,
            1000000,
            0,
            50,
        )];

        let graph_symbols = vec![create_symbol_node(
            "large_id",
            "large_fn",
            SymbolKind::Function,
            999999,
            1000000,
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
        assert_eq!(out[0].start_line, 999999);
        assert_eq!(out[0].end_line, 1000000);
    }

    #[test]
    fn test_collect_symbol_info_deeply_nested_symbols() {
        // Create a deeply nested structure: module -> struct -> impl -> method
        let mut level1 = create_lsp_symbol("module", LspSymbolKind::Module, 1, 100, 0, 5);
        let mut level2 = create_lsp_symbol("MyStruct", LspSymbolKind::Struct, 5, 90, 2, 10);
        let mut level3 = create_lsp_symbol("impl", LspSymbolKind::Struct, 10, 80, 4, 8);
        let level4 = create_lsp_symbol("method", LspSymbolKind::Function, 15, 20, 6, 15);

        level3.children = vec![level4];
        level2.children = vec![level3];
        level1.children = vec![level2];
        let lsp_symbols = vec![level1];

        let graph_symbols = vec![
            create_symbol_node("mod_id", "module", SymbolKind::Module, 1, 100),
            create_symbol_node("struct_id", "MyStruct", SymbolKind::Struct, 5, 90),
            create_symbol_node("impl_id", "impl", SymbolKind::Struct, 10, 80),
            create_symbol_node("method_id", "method", SymbolKind::Function, 15, 20),
        ];

        let mut out = Vec::new();
        collect_symbol_info(
            &lsp_symbols,
            &graph_symbols,
            "file:///test.rs",
            Language::Rust,
            &mut out,
        );

        assert_eq!(out.len(), 4);
        assert_eq!(out[0].id, "mod_id");
        assert_eq!(out[1].id, "struct_id");
        assert_eq!(out[2].id, "impl_id");
        assert_eq!(out[3].id, "method_id");
    }

    #[test]
    fn test_collect_symbol_info_multiple_children_per_parent() {
        let mut parent = create_lsp_symbol("Parent", LspSymbolKind::Struct, 1, 50, 0, 5);
        parent.children = vec![
            create_lsp_symbol("child1", LspSymbolKind::Function, 5, 10, 2, 10),
            create_lsp_symbol("child2", LspSymbolKind::Function, 12, 20, 2, 10),
            create_lsp_symbol("child3", LspSymbolKind::Function, 22, 30, 2, 10),
        ];
        let lsp_symbols = vec![parent];

        let graph_symbols = vec![
            create_symbol_node("parent_id", "Parent", SymbolKind::Struct, 1, 50),
            create_symbol_node("child1_id", "child1", SymbolKind::Function, 5, 10),
            create_symbol_node("child2_id", "child2", SymbolKind::Function, 12, 20),
            create_symbol_node("child3_id", "child3", SymbolKind::Function, 22, 30),
        ];

        let mut out = Vec::new();
        collect_symbol_info(
            &lsp_symbols,
            &graph_symbols,
            "file:///test.rs",
            Language::Rust,
            &mut out,
        );

        assert_eq!(out.len(), 4);
        assert_eq!(out[1].id, "child1_id");
        assert_eq!(out[2].id, "child2_id");
        assert_eq!(out[3].id, "child3_id");
    }

    #[test]
    fn test_collect_symbol_info_various_symbol_kinds() {
        let lsp_symbols = vec![
            create_lsp_symbol("MyClass", LspSymbolKind::Class, 1, 10, 0, 5),
            create_lsp_symbol("my_function", LspSymbolKind::Function, 12, 20, 0, 5),
            create_lsp_symbol("MY_CONST", LspSymbolKind::Constant, 22, 22, 0, 10),
            create_lsp_symbol("MyEnum", LspSymbolKind::Enum, 24, 30, 0, 5),
            create_lsp_symbol("my_var", LspSymbolKind::Variable, 32, 32, 0, 5),
        ];

        let graph_symbols = vec![
            create_symbol_node("class_id", "MyClass", SymbolKind::Class, 1, 10),
            create_symbol_node("fn_id", "my_function", SymbolKind::Function, 12, 20),
            create_symbol_node("const_id", "MY_CONST", SymbolKind::Constant, 22, 22),
            create_symbol_node("enum_id", "MyEnum", SymbolKind::Enum, 24, 30),
            create_symbol_node("var_id", "my_var", SymbolKind::Variable, 32, 32),
        ];

        let mut out = Vec::new();
        collect_symbol_info(
            &lsp_symbols,
            &graph_symbols,
            "file:///test.rs",
            Language::Rust,
            &mut out,
        );

        assert_eq!(out.len(), 5);
        assert_eq!(out[0].id, "class_id");
        assert_eq!(out[1].id, "fn_id");
        assert_eq!(out[2].id, "const_id");
        assert_eq!(out[3].id, "enum_id");
        assert_eq!(out[4].id, "var_id");
    }

    #[test]
    fn test_collect_symbol_info_with_large_column_numbers() {
        let lsp_symbols = vec![create_lsp_symbol(
            "wide_line",
            LspSymbolKind::Function,
            1,
            1,
            5000,
            5050,
        )];

        let graph_symbols = vec![create_symbol_node(
            "wide_id",
            "wide_line",
            SymbolKind::Function,
            1,
            1,
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
        assert_eq!(out[0].start_col, 5000);
    }

    #[test]
    fn test_collect_symbol_info_empty_symbol_name() {
        let lsp_symbols = vec![create_lsp_symbol("", LspSymbolKind::Function, 1, 5, 0, 10)];

        let graph_symbols = vec![create_symbol_node(
            "empty_id",
            "",
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
        assert_eq!(out[0].id, "empty_id");
    }

    #[test]
    fn test_collect_symbol_info_unicode_file_uri() {
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

        let unicode_uri = "file:///test/日本語/файл.rs";
        let mut out = Vec::new();
        collect_symbol_info(
            &lsp_symbols,
            &graph_symbols,
            unicode_uri,
            Language::Rust,
            &mut out,
        );

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].file_uri, unicode_uri);
    }

    #[test]
    fn test_handle_file_result_with_zero_symbols_but_nonzero_count() {
        let mut result = Phase2Result {
            symbols: Vec::new(),
            symbol_count: 0,
            error_count: 0,
        };

        let file = create_test_file("/test/file.rs");
        let outcome = Ok((Vec::new(), 0));

        handle_file_result(outcome, &file, &mut result);

        assert_eq!(result.symbols.len(), 0);
        assert_eq!(result.symbol_count, 0);
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_handle_file_result_preserves_symbol_order() {
        let mut result = Phase2Result {
            symbols: Vec::new(),
            symbol_count: 0,
            error_count: 0,
        };

        let file = create_test_file("/test/file.rs");
        let symbols = vec![
            create_test_symbol("first"),
            create_test_symbol("second"),
            create_test_symbol("third"),
        ];

        handle_file_result(Ok((symbols, 3)), &file, &mut result);

        assert_eq!(result.symbols.len(), 3);
        assert_eq!(result.symbols[0].id, "first");
        assert_eq!(result.symbols[1].id, "second");
        assert_eq!(result.symbols[2].id, "third");
    }

    #[test]
    fn test_handle_file_result_with_duplicate_symbol_ids() {
        let mut result = Phase2Result {
            symbols: Vec::new(),
            symbol_count: 0,
            error_count: 0,
        };

        let file1 = create_test_file("/test/file1.rs");
        let file2 = create_test_file("/test/file2.rs");

        let symbols1 = vec![create_test_symbol("duplicate_id")];
        let symbols2 = vec![create_test_symbol("duplicate_id")];

        handle_file_result(Ok((symbols1, 1)), &file1, &mut result);
        handle_file_result(Ok((symbols2, 1)), &file2, &mut result);

        assert_eq!(result.symbols.len(), 2);
        assert_eq!(result.symbols[0].id, "duplicate_id");
        assert_eq!(result.symbols[1].id, "duplicate_id");
    }

    #[test]
    fn test_collect_symbol_info_single_line_symbols() {
        let lsp_symbols = vec![
            create_lsp_symbol("short1", LspSymbolKind::Variable, 5, 5, 0, 10),
            create_lsp_symbol("short2", LspSymbolKind::Variable, 7, 7, 0, 10),
        ];

        let graph_symbols = vec![
            create_symbol_node("short1_id", "short1", SymbolKind::Variable, 5, 5),
            create_symbol_node("short2_id", "short2", SymbolKind::Variable, 7, 7),
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
        assert_eq!(out[0].start_line, 5);
        assert_eq!(out[0].end_line, 5);
        assert_eq!(out[1].start_line, 7);
        assert_eq!(out[1].end_line, 7);
    }

    #[test]
    fn test_collect_symbol_info_reverse_order_lines() {
        // Edge case: symbols where end_line < start_line (malformed input)
        let lsp_symbols = vec![create_lsp_symbol(
            "backwards",
            LspSymbolKind::Function,
            10,
            5,
            0,
            10,
        )];

        let graph_symbols = vec![create_symbol_node(
            "back_id",
            "backwards",
            SymbolKind::Function,
            10,
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
        assert_eq!(out[0].start_line, 10);
        assert_eq!(out[0].end_line, 5);
    }

    #[test]
    fn test_collect_symbol_info_more_graph_symbols_than_lsp() {
        let lsp_symbols = vec![create_lsp_symbol(
            "only_one",
            LspSymbolKind::Function,
            1,
            5,
            0,
            10,
        )];

        let graph_symbols = vec![
            create_symbol_node("id1", "only_one", SymbolKind::Function, 1, 5),
            create_symbol_node("id2", "extra", SymbolKind::Function, 7, 10),
        ];

        let mut out = Vec::new();
        collect_symbol_info(
            &lsp_symbols,
            &graph_symbols,
            "file:///test.rs",
            Language::Rust,
            &mut out,
        );

        // Should only process as many as lsp_symbols (zips stop at shorter)
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "id1");
    }

    #[test]
    fn test_phase2_result_with_nonzero_initial_values() {
        let result = Phase2Result {
            symbols: vec![create_test_symbol("existing")],
            symbol_count: 100,
            error_count: 5,
        };

        assert_eq!(result.symbols.len(), 1);
        assert_eq!(result.symbol_count, 100);
        assert_eq!(result.error_count, 5);
    }

    #[test]
    fn test_collect_symbol_info_all_languages() {
        for test_lang in [
            Language::Rust,
            Language::Python,
            Language::TypeScript,
            Language::JavaScript,
            Language::Go,
        ] {
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
                test_lang,
                &mut out,
            );

            assert_eq!(out.len(), 1);
            assert_eq!(out[0].language, test_lang);
        }
    }

    #[test]
    fn test_collect_symbol_info_complex_nested_with_multiple_branches() {
        // Create a tree structure with multiple branches at each level
        let mut root = create_lsp_symbol("root", LspSymbolKind::Module, 1, 100, 0, 5);

        let mut branch1 = create_lsp_symbol("branch1", LspSymbolKind::Struct, 5, 40, 2, 10);
        branch1.children = vec![
            create_lsp_symbol("leaf1", LspSymbolKind::Function, 10, 20, 4, 10),
            create_lsp_symbol("leaf2", LspSymbolKind::Function, 25, 35, 4, 10),
        ];

        let mut branch2 = create_lsp_symbol("branch2", LspSymbolKind::Struct, 45, 90, 2, 10);
        branch2.children = vec![
            create_lsp_symbol("leaf3", LspSymbolKind::Function, 50, 60, 4, 10),
            create_lsp_symbol("leaf4", LspSymbolKind::Function, 65, 85, 4, 10),
        ];

        root.children = vec![branch1, branch2];
        let lsp_symbols = vec![root];

        let graph_symbols = vec![
            create_symbol_node("root_id", "root", SymbolKind::Module, 1, 100),
            create_symbol_node("b1_id", "branch1", SymbolKind::Struct, 5, 40),
            create_symbol_node("l1_id", "leaf1", SymbolKind::Function, 10, 20),
            create_symbol_node("l2_id", "leaf2", SymbolKind::Function, 25, 35),
            create_symbol_node("b2_id", "branch2", SymbolKind::Struct, 45, 90),
            create_symbol_node("l3_id", "leaf3", SymbolKind::Function, 50, 60),
            create_symbol_node("l4_id", "leaf4", SymbolKind::Function, 65, 85),
        ];

        let mut out = Vec::new();
        collect_symbol_info(
            &lsp_symbols,
            &graph_symbols,
            "file:///complex.rs",
            Language::Rust,
            &mut out,
        );

        assert_eq!(out.len(), 7);
        assert_eq!(out[0].id, "root_id");
        assert_eq!(out[1].id, "b1_id");
        assert_eq!(out[2].id, "l1_id");
        assert_eq!(out[3].id, "l2_id");
        assert_eq!(out[4].id, "b2_id");
        assert_eq!(out[5].id, "l3_id");
        assert_eq!(out[6].id, "l4_id");
    }
}

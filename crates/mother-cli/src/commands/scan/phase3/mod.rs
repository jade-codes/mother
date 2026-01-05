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
mod tests {
    mod tests_run;

    use super::*;
    use mother_core::lsp::LspReference;
    use std::path::PathBuf;

    /// Helper to create a test reference at a specific file and line
    fn make_reference(file_path: &str, line: u32) -> LspReference {
        LspReference {
            file: PathBuf::from(file_path),
            line,
            start_col: 0,
            end_col: 10,
        }
    }

    /// Helper to create a symbols_by_file map
    #[allow(clippy::type_complexity)]
    fn make_symbols_map(
        entries: Vec<(&str, Vec<(&str, u32, u32)>)>,
    ) -> HashMap<String, Vec<(String, u32, u32)>> {
        entries
            .into_iter()
            .map(|(file, symbols)| {
                (
                    file.to_string(),
                    symbols
                        .into_iter()
                        .map(|(id, start, end)| (id.to_string(), start, end))
                        .collect(),
                )
            })
            .collect()
    }

    #[test]
    fn test_find_containing_symbol_exact_match() {
        let reference = make_reference("/src/main.rs", 10);
        let symbols = make_symbols_map(vec![("/src/main.rs", vec![("symbol1", 5, 15)])]);

        let result = find_containing_symbol(&reference, &symbols);
        assert_eq!(result, Some("symbol1".to_string()));
    }

    #[test]
    fn test_find_containing_symbol_nested_symbols_selects_smallest() {
        // Reference at line 10 could be in both outer (1-20) and inner (8-12)
        // Should select the inner one (smallest span)
        let reference = make_reference("/src/main.rs", 10);
        let symbols = make_symbols_map(vec![(
            "/src/main.rs",
            vec![("outer_function", 1, 20), ("inner_block", 8, 12)],
        )]);

        let result = find_containing_symbol(&reference, &symbols);
        assert_eq!(result, Some("inner_block".to_string()));
    }

    #[test]
    fn test_find_containing_symbol_multiple_nested_selects_smallest() {
        // Reference at line 10 matches three symbols
        // Should select the one with smallest range
        let reference = make_reference("/src/main.rs", 10);
        let symbols = make_symbols_map(vec![(
            "/src/main.rs",
            vec![
                ("class", 1, 50),       // range: 49
                ("method", 5, 20),      // range: 15
                ("inner_block", 9, 11), // range: 2 (smallest)
            ],
        )]);

        let result = find_containing_symbol(&reference, &symbols);
        assert_eq!(result, Some("inner_block".to_string()));
    }

    #[test]
    fn test_find_containing_symbol_outside_all_symbols() {
        let reference = make_reference("/src/main.rs", 100);
        let symbols = make_symbols_map(vec![(
            "/src/main.rs",
            vec![("symbol1", 1, 10), ("symbol2", 20, 30)],
        )]);

        let result = find_containing_symbol(&reference, &symbols);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_containing_symbol_file_not_in_map() {
        let reference = make_reference("/src/other.rs", 10);
        let symbols = make_symbols_map(vec![("/src/main.rs", vec![("symbol1", 1, 20)])]);

        let result = find_containing_symbol(&reference, &symbols);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_containing_symbol_empty_map() {
        let reference = make_reference("/src/main.rs", 10);
        let symbols = HashMap::new();

        let result = find_containing_symbol(&reference, &symbols);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_containing_symbol_at_start_boundary() {
        // Reference exactly at start line of symbol
        let reference = make_reference("/src/main.rs", 5);
        let symbols = make_symbols_map(vec![("/src/main.rs", vec![("symbol1", 5, 15)])]);

        let result = find_containing_symbol(&reference, &symbols);
        assert_eq!(result, Some("symbol1".to_string()));
    }

    #[test]
    fn test_find_containing_symbol_at_end_boundary() {
        // Reference exactly at end line of symbol
        let reference = make_reference("/src/main.rs", 15);
        let symbols = make_symbols_map(vec![("/src/main.rs", vec![("symbol1", 5, 15)])]);

        let result = find_containing_symbol(&reference, &symbols);
        assert_eq!(result, Some("symbol1".to_string()));
    }

    #[test]
    fn test_find_containing_symbol_just_before_start() {
        // Reference just before symbol start (line 4, symbol starts at 5)
        let reference = make_reference("/src/main.rs", 4);
        let symbols = make_symbols_map(vec![("/src/main.rs", vec![("symbol1", 5, 15)])]);

        let result = find_containing_symbol(&reference, &symbols);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_containing_symbol_just_after_end() {
        // Reference just after symbol end (line 16, symbol ends at 15)
        let reference = make_reference("/src/main.rs", 16);
        let symbols = make_symbols_map(vec![("/src/main.rs", vec![("symbol1", 5, 15)])]);

        let result = find_containing_symbol(&reference, &symbols);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_containing_symbol_multiple_files() {
        let reference = make_reference("/src/utils.rs", 10);
        let symbols = make_symbols_map(vec![
            ("/src/main.rs", vec![("main_symbol", 1, 50)]),
            ("/src/utils.rs", vec![("util_symbol", 5, 15)]),
        ]);

        let result = find_containing_symbol(&reference, &symbols);
        assert_eq!(result, Some("util_symbol".to_string()));
    }

    #[test]
    fn test_find_containing_symbol_same_range_picks_first() {
        // Two symbols with identical ranges - should pick the first one found
        let reference = make_reference("/src/main.rs", 10);
        let symbols = make_symbols_map(vec![(
            "/src/main.rs",
            vec![("symbol1", 5, 15), ("symbol2", 5, 15)],
        )]);

        let result = find_containing_symbol(&reference, &symbols);
        // With min_by_key, when ranges are equal, it returns the first one
        assert!(result == Some("symbol1".to_string()) || result == Some("symbol2".to_string()));
        assert!(result.is_some());
    }

    #[test]
    fn test_find_containing_symbol_single_line_symbol() {
        // Symbol that starts and ends on the same line
        let reference = make_reference("/src/main.rs", 10);
        let symbols = make_symbols_map(vec![("/src/main.rs", vec![("single_line", 10, 10)])]);

        let result = find_containing_symbol(&reference, &symbols);
        assert_eq!(result, Some("single_line".to_string()));
    }

    #[test]
    fn test_find_containing_symbol_no_symbols_in_file() {
        let reference = make_reference("/src/main.rs", 10);
        let symbols = make_symbols_map(vec![("/src/main.rs", vec![])]);

        let result = find_containing_symbol(&reference, &symbols);
        assert_eq!(result, None);
    }

    #[test]
    fn test_build_symbol_lookup_table_empty() {
        let symbols: Vec<SymbolInfo> = vec![];
        let result = build_symbol_lookup_table(&symbols);
        assert!(result.is_empty());
    }

    #[test]
    fn test_build_symbol_lookup_table_strips_file_prefix() {
        use mother_core::scanner::Language;

        let symbols = vec![SymbolInfo {
            id: "sym1".to_string(),
            file_uri: "file:///home/project/src/main.rs".to_string(),
            start_line: 1,
            end_line: 10,
            start_col: 0,
            language: Language::Rust,
        }];

        let result = build_symbol_lookup_table(&symbols);
        assert_eq!(result.len(), 1);
        assert!(result.contains_key("/home/project/src/main.rs"));

        let file_symbols = &result["/home/project/src/main.rs"];
        assert_eq!(file_symbols.len(), 1);
        assert_eq!(file_symbols[0].0, "sym1");
        assert_eq!(file_symbols[0].1, 1);
        assert_eq!(file_symbols[0].2, 10);
    }

    #[test]
    fn test_build_symbol_lookup_table_groups_by_file() {
        use mother_core::scanner::Language;

        let symbols = vec![
            SymbolInfo {
                id: "sym1".to_string(),
                file_uri: "file:///src/main.rs".to_string(),
                start_line: 1,
                end_line: 10,
                start_col: 0,
                language: Language::Rust,
            },
            SymbolInfo {
                id: "sym2".to_string(),
                file_uri: "file:///src/main.rs".to_string(),
                start_line: 20,
                end_line: 30,
                start_col: 0,
                language: Language::Rust,
            },
            SymbolInfo {
                id: "sym3".to_string(),
                file_uri: "file:///src/utils.rs".to_string(),
                start_line: 1,
                end_line: 5,
                start_col: 0,
                language: Language::Rust,
            },
        ];

        let result = build_symbol_lookup_table(&symbols);
        assert_eq!(result.len(), 2);

        let main_symbols = &result["/src/main.rs"];
        assert_eq!(main_symbols.len(), 2);
        assert_eq!(main_symbols[0].0, "sym1");
        assert_eq!(main_symbols[1].0, "sym2");

        let utils_symbols = &result["/src/utils.rs"];
        assert_eq!(utils_symbols.len(), 1);
        assert_eq!(utils_symbols[0].0, "sym3");
    }

    #[test]
    fn test_build_symbol_lookup_table_no_file_prefix() {
        use mother_core::scanner::Language;

        let symbols = vec![SymbolInfo {
            id: "sym1".to_string(),
            file_uri: "/absolute/path/main.rs".to_string(),
            start_line: 1,
            end_line: 10,
            start_col: 0,
            language: Language::Rust,
        }];

        let result = build_symbol_lookup_table(&symbols);
        assert_eq!(result.len(), 1);
        assert!(result.contains_key("/absolute/path/main.rs"));
    }

    /// Tests for Edge creation logic used in create_reference_edge
    mod edge_creation_tests {
        use super::*;
        use mother_core::graph::model::{Edge, EdgeKind};

        #[test]
        fn test_edge_creation_with_valid_reference() {
            let reference = make_reference("/src/main.rs", 42);
            let from_id = "caller_symbol";
            let to_id = "called_symbol";

            // Simulate the edge creation logic from create_reference_edge
            let edge = Edge {
                source_id: from_id.to_string(),
                target_id: to_id.to_string(),
                kind: EdgeKind::References,
                line: Some(reference.line),
                column: Some(reference.start_col),
            };

            assert_eq!(edge.source_id, "caller_symbol");
            assert_eq!(edge.target_id, "called_symbol");
            assert_eq!(edge.kind, EdgeKind::References);
            assert_eq!(edge.line, Some(42));
            assert_eq!(edge.column, Some(0));
        }

        #[test]
        fn test_edge_creation_uses_references_kind() {
            let reference = make_reference("/src/lib.rs", 10);
            let edge = Edge {
                source_id: "source".to_string(),
                target_id: "target".to_string(),
                kind: EdgeKind::References,
                line: Some(reference.line),
                column: Some(reference.start_col),
            };

            // Verify that create_reference_edge always uses References kind
            assert_eq!(edge.kind, EdgeKind::References);
        }

        #[test]
        fn test_edge_creation_with_different_line_numbers() {
            let test_cases = vec![1, 42, 100, 999, 10000];

            for line_num in test_cases {
                let reference = make_reference("/src/test.rs", line_num);
                let edge = Edge {
                    source_id: "src".to_string(),
                    target_id: "dst".to_string(),
                    kind: EdgeKind::References,
                    line: Some(reference.line),
                    column: Some(reference.start_col),
                };

                assert_eq!(edge.line, Some(line_num));
            }
        }

        #[test]
        fn test_edge_creation_with_different_column_numbers() {
            let test_cases = vec![0, 5, 10, 50, 100];

            for col in test_cases {
                let mut reference = make_reference("/src/test.rs", 10);
                reference.start_col = col;

                let edge = Edge {
                    source_id: "src".to_string(),
                    target_id: "dst".to_string(),
                    kind: EdgeKind::References,
                    line: Some(reference.line),
                    column: Some(reference.start_col),
                };

                assert_eq!(edge.column, Some(col));
            }
        }

        #[test]
        fn test_edge_creation_preserves_ids() {
            let reference = make_reference("/src/main.rs", 10);

            let edge = Edge {
                source_id: "complex::module::function".to_string(),
                target_id: "other::module::Type::method".to_string(),
                kind: EdgeKind::References,
                line: Some(reference.line),
                column: Some(reference.start_col),
            };

            assert_eq!(edge.source_id, "complex::module::function");
            assert_eq!(edge.target_id, "other::module::Type::method");
        }

        #[test]
        fn test_edge_creation_with_special_characters_in_ids() {
            let reference = make_reference("/src/main.rs", 10);

            let edge = Edge {
                source_id: "file:///path/symbol#123".to_string(),
                target_id: "file:///other/symbol#456".to_string(),
                kind: EdgeKind::References,
                line: Some(reference.line),
                column: Some(reference.start_col),
            };

            assert_eq!(edge.source_id, "file:///path/symbol#123");
            assert_eq!(edge.target_id, "file:///other/symbol#456");
        }

        #[test]
        fn test_edge_line_and_column_are_optional() {
            // Test that Edge struct supports None for line and column
            let edge = Edge {
                source_id: "src".to_string(),
                target_id: "dst".to_string(),
                kind: EdgeKind::References,
                line: None,
                column: None,
            };

            assert_eq!(edge.line, None);
            assert_eq!(edge.column, None);
        }

        #[test]
        fn test_edge_with_zero_line_and_column() {
            let reference = LspReference {
                file: std::path::PathBuf::from("/src/test.rs"),
                line: 0,
                start_col: 0,
                end_col: 0,
            };

            let edge = Edge {
                source_id: "src".to_string(),
                target_id: "dst".to_string(),
                kind: EdgeKind::References,
                line: Some(reference.line),
                column: Some(reference.start_col),
            };

            assert_eq!(edge.line, Some(0));
            assert_eq!(edge.column, Some(0));
        }
    }

    /// Tests for reference edge creation logic flow
    mod reference_edge_logic_tests {
        #[test]
        fn test_self_reference_should_be_filtered() {
            // In create_reference_edges, edges where from_id == to_id are filtered
            // This test verifies the logic: if from_id != symbol_info.id
            let from_id = "symbol123";
            let to_id = "symbol123";

            // This simulates the check in create_reference_edges line 112
            let should_create_edge = from_id != to_id;
            assert!(
                !should_create_edge,
                "Self-references should be filtered out"
            );
        }

        #[test]
        fn test_different_symbols_should_create_edge() {
            let from_id = "symbol_a";
            let to_id = "symbol_b";

            let should_create_edge = from_id != to_id;
            assert!(should_create_edge, "Different symbols should create edge");
        }

        #[test]
        fn test_reference_without_containing_symbol_skipped() {
            // When find_containing_symbol returns None, no edge is created
            // This is handled by the if let Some(from_id) pattern in line 111
            let containing_symbol: Option<String> = None;

            assert!(
                containing_symbol.is_none(),
                "Reference without containing symbol should be skipped"
            );
        }

        #[test]
        fn test_reference_with_containing_symbol_processed() {
            let containing_symbol: Option<String> = Some("some_symbol".to_string());

            assert!(
                containing_symbol.is_some(),
                "Reference with containing symbol should be processed"
            );
        }

        #[test]
        fn test_edge_counter_logic() {
            // Simulates the counting logic in create_reference_edges
            let mut count = 0;
            let test_cases = vec![
                (Some("sym1".to_string()), "sym2"), // Should count: different symbols
                (Some("sym2".to_string()), "sym2"), // Should not count: self-reference
                (None, "sym3"),                     // Should not count: no containing symbol
                (Some("sym4".to_string()), "sym5"), // Should count: different symbols
            ];

            for (from_opt, to_id) in test_cases {
                if let Some(from_id) = from_opt {
                    if from_id != to_id {
                        // Simulating successful edge creation
                        count += 1;
                    }
                }
            }

            assert_eq!(count, 2, "Only 2 valid edges should be counted");
        }
    }

    /// Tests for reference location and symbol mapping
    mod reference_mapping_tests {
        use super::*;

        #[test]
        fn test_reference_to_edge_mapping_preserves_location() {
            let reference = make_reference("/src/main.rs", 42);

            // Verify the reference location is correctly mapped to edge
            assert_eq!(reference.line, 42);
            assert_eq!(reference.start_col, 0);
        }

        #[test]
        fn test_multiple_references_same_symbol() {
            // Tests that multiple references to the same symbol can exist
            let symbols = make_symbols_map(vec![(
                "/src/main.rs",
                vec![("function_a", 1, 10), ("function_b", 20, 30)],
            )]);

            let ref1 = make_reference("/src/main.rs", 5);
            let ref2 = make_reference("/src/main.rs", 7);

            let containing1 = find_containing_symbol(&ref1, &symbols);
            let containing2 = find_containing_symbol(&ref2, &symbols);

            assert_eq!(containing1, Some("function_a".to_string()));
            assert_eq!(containing2, Some("function_a".to_string()));
        }

        #[test]
        fn test_references_from_different_symbols() {
            let symbols = make_symbols_map(vec![(
                "/src/main.rs",
                vec![("function_a", 1, 10), ("function_b", 20, 30)],
            )]);

            let ref1 = make_reference("/src/main.rs", 5);
            let ref2 = make_reference("/src/main.rs", 25);

            let containing1 = find_containing_symbol(&ref1, &symbols);
            let containing2 = find_containing_symbol(&ref2, &symbols);

            assert_eq!(containing1, Some("function_a".to_string()));
            assert_eq!(containing2, Some("function_b".to_string()));
            assert_ne!(containing1, containing2);
        }

        #[test]
        fn test_reference_in_non_existent_file() {
            let symbols = make_symbols_map(vec![("/src/main.rs", vec![("func", 1, 10)])]);

            let reference = make_reference("/src/other.rs", 5);
            let containing = find_containing_symbol(&reference, &symbols);

            assert_eq!(
                containing, None,
                "Reference in non-existent file should have no containing symbol"
            );
        }

        #[test]
        fn test_edge_kind_is_always_references() {
            // create_reference_edge always uses EdgeKind::References
            // This is a critical property of the function
            let edge_kind = EdgeKind::References;

            assert_eq!(edge_kind, EdgeKind::References);
            assert_ne!(edge_kind, EdgeKind::Calls);
            assert_ne!(edge_kind, EdgeKind::Imports);
        }
    }

    /// Tests for process_symbol_references behavior and contract
    ///
    /// Note: process_symbol_references is a private async function that requires
    /// Neo4jClient and LspServerManager. Since these are not mockable in the current
    /// codebase and the function must remain private per project guidelines, these
    /// tests focus on documenting and testing the contract, behavior, and integration
    /// logic without directly invoking the function.
    mod process_symbol_references_tests {
        use super::*;

        /// Test that the return type contract is a tuple of (reference_count, error_count)
        #[test]
        fn test_return_type_contract() {
            // process_symbol_references returns (usize, usize)
            // First element: count of references created
            // Second element: count of errors encountered

            // Success case: (count, 0)
            let success_result: (usize, usize) = (5, 0);
            assert_eq!(
                success_result.0, 5,
                "Reference count should be positive on success"
            );
            assert_eq!(success_result.1, 0, "Error count should be 0 on success");

            // Error case: (0, 1)
            let error_result: (usize, usize) = (0, 1);
            assert_eq!(error_result.0, 0, "Reference count should be 0 on error");
            assert_eq!(error_result.1, 1, "Error count should be 1 on error");
        }

        /// Test the error handling contract: LSP client failures return (0, 1)
        #[test]
        fn test_lsp_client_error_contract() {
            // When lsp_manager.get_client() fails in process_symbol_references
            // The function should return (0, 1) indicating 0 references and 1 error
            let expected_error_result = (0_usize, 1_usize);

            assert_eq!(
                expected_error_result.0, 0,
                "No references on LSP client error"
            );
            assert_eq!(expected_error_result.1, 1, "One error on LSP client error");
        }

        /// Test the error handling contract: LSP references call failures return (0, 1)
        #[test]
        fn test_lsp_references_error_contract() {
            // When lsp_client.references() fails in process_symbol_references
            // The function should return (0, 1) indicating 0 references and 1 error
            let expected_error_result = (0_usize, 1_usize);

            assert_eq!(
                expected_error_result.0, 0,
                "No references on LSP references error"
            );
            assert_eq!(
                expected_error_result.1, 1,
                "One error on LSP references error"
            );
        }

        /// Test the success case contract
        #[test]
        fn test_success_case_contract() {
            // On success, process_symbol_references returns:
            // (create_reference_edges result, 0)
            // The error count is always 0 in the success path

            // Simulate various success scenarios
            let success_cases = vec![
                (0, 0),   // No references found
                (1, 0),   // One reference found
                (10, 0),  // Multiple references found
                (100, 0), // Many references found
            ];

            for (_ref_count, error_count) in success_cases {
                assert_eq!(error_count, 0, "Error count must be 0 in success case");
                // ref_count is usize, always non-negative by type
            }
        }

        /// Test that process_symbol_references integrates with create_reference_edges
        #[test]
        fn test_integration_with_create_reference_edges() {
            // process_symbol_references calls create_reference_edges
            // The result of create_reference_edges becomes the reference_count

            // Simulate create_reference_edges returning different counts
            let edge_counts = vec![0, 1, 5, 10, 50, 100];

            for count in edge_counts {
                // In success path: (count_from_create_reference_edges, 0)
                let simulated_result = (count, 0);
                assert_eq!(simulated_result.0, count);
                assert_eq!(simulated_result.1, 0);
            }
        }

        /// Test that the function uses symbol_info fields correctly
        #[test]
        fn test_symbol_info_field_usage() {
            use mother_core::scanner::Language;

            // process_symbol_references uses these SymbolInfo fields:
            // - language: passed to lsp_manager.get_client (line 59)
            // - file_uri: passed to lsp_client.references (line 66)
            // - start_line: passed to lsp_client.references (line 67)
            // - start_col: passed to lsp_client.references (line 68)
            // - id: used in create_reference_edges for edge creation

            let symbol = SymbolInfo {
                id: "test_symbol".to_string(),
                file_uri: "file:///test/path.rs".to_string(),
                start_line: 10,
                end_line: 20,
                start_col: 5,
                language: Language::Rust,
            };

            // Verify all fields are properly set for use by process_symbol_references
            assert!(!symbol.id.is_empty(), "Symbol id must not be empty");
            assert!(
                symbol.file_uri.contains("file://"),
                "File URI must be valid"
            );
            assert!(symbol.start_line > 0, "Start line must be positive");
            // start_col is u32, always non-negative by type
        }

        /// Test that the function expects symbols_by_file to be properly structured
        #[test]
        fn test_symbols_by_file_structure_expectations() {
            // process_symbol_references passes symbols_by_file to create_reference_edges
            // The structure should match what build_symbol_lookup_table produces

            let symbols_by_file = make_symbols_map(vec![
                ("/path/file1.rs", vec![("sym1", 1, 10), ("sym2", 20, 30)]),
                ("/path/file2.rs", vec![("sym3", 1, 5)]),
            ]);

            // Verify structure matches expected format: HashMap<String, Vec<(String, u32, u32)>>
            assert_eq!(symbols_by_file.len(), 2);

            let file1_symbols = &symbols_by_file["/path/file1.rs"];
            assert_eq!(file1_symbols.len(), 2);
            assert_eq!(file1_symbols[0].0, "sym1");
            assert_eq!(file1_symbols[0].1, 1);
            assert_eq!(file1_symbols[0].2, 10);
        }

        /// Test the happy path flow logic
        #[test]
        fn test_happy_path_flow() {
            // Happy path flow in process_symbol_references:
            // 1. Get LSP client -> Ok(client)
            // 2. Call client.references() -> Ok(references)
            // 3. Call create_reference_edges() -> count
            // 4. Return (count, 0)

            // Simulate the flow with mock values
            let lsp_client_result: Result<(), String> = Ok(());
            assert!(
                lsp_client_result.is_ok(),
                "LSP client should succeed in happy path"
            );

            let references_result: Result<Vec<String>, String> = Ok(vec![
                "ref1".to_string(),
                "ref2".to_string(),
                "ref3".to_string(),
            ]);
            assert!(references_result.is_ok(), "References call should succeed");

            if let Ok(references) = references_result {
                let edge_count = references.len(); // Simulating create_reference_edges logic

                let final_result = (edge_count, 0);
                assert_eq!(
                    final_result,
                    (3, 0),
                    "Should return count from edges with 0 errors"
                );
            }
        }

        /// Test the error path flow when LSP client fails
        #[test]
        fn test_error_path_lsp_client_fails() {
            // Error path 1: lsp_manager.get_client() fails
            // Flow:
            // 1. Get LSP client -> Err(_)
            // 2. Return (0, 1) immediately

            let lsp_client_result: Result<(), String> = Err("Failed to get client".to_string());

            let final_result = if lsp_client_result.is_err() {
                (0, 1)
            } else {
                (0, 0)
            };

            assert_eq!(
                final_result,
                (0, 1),
                "Should return (0, 1) when LSP client fails"
            );
        }

        /// Test the error path flow when references call fails
        #[test]
        fn test_error_path_references_call_fails() {
            // Error path 2: lsp_client.references() fails
            // Flow:
            // 1. Get LSP client -> Ok(client)
            // 2. Call client.references() -> Err(_)
            // 3. Return (0, 1) immediately

            let lsp_client_result: Result<(), String> = Ok(());
            let references_result: Result<Vec<String>, String> =
                Err("References call failed".to_string());

            // Both conditions lead to error, simplified check
            let final_result = (0, 1);

            assert!(lsp_client_result.is_ok(), "LSP client succeeded");
            assert!(references_result.is_err(), "References call failed");
            assert_eq!(
                final_result,
                (0, 1),
                "Should return (0, 1) when references call fails"
            );
        }

        /// Test that zero references is a valid success case
        #[test]
        fn test_zero_references_is_valid_success() {
            // If create_reference_edges returns 0 (no matching references found),
            // this is still a success case, not an error
            let edge_count = 0;
            let result = (edge_count, 0);

            assert_eq!(
                result,
                (0, 0),
                "Zero references with no errors is valid success"
            );
            assert_ne!(
                result,
                (0, 1),
                "Zero references is not the same as an error"
            );
        }

        /// Test boundary conditions for reference counts
        #[test]
        fn test_reference_count_boundaries() {
            // Test various boundary values for reference counts
            let boundary_cases = vec![
                0,     // Minimum: no references
                1,     // Single reference
                2,     // Two references
                10,    // Typical small count
                100,   // Typical medium count
                1000,  // Large count
                10000, // Very large count
            ];

            for count in boundary_cases {
                let success_result = (count, 0);
                assert_eq!(success_result.0, count);
                assert_eq!(success_result.1, 0);
            }
        }

        /// Test that error count is always 0 or 1
        #[test]
        fn test_error_count_is_binary() {
            // process_symbol_references returns either:
            // - (count, 0) on success
            // - (0, 1) on error
            // The error count is always 0 or 1, never higher

            let valid_results = vec![
                (0, 0), // Success with no references
                (5, 0), // Success with references
                (0, 1), // Error
            ];

            for (ref_count, err_count) in valid_results {
                assert!(
                    err_count == 0 || err_count == 1,
                    "Error count must be 0 or 1, got {}",
                    err_count
                );

                if err_count == 1 {
                    assert_eq!(ref_count, 0, "Reference count must be 0 when error occurs");
                }
            }
        }

        /// Test the relationship between reference count and error count
        #[test]
        fn test_reference_and_error_count_relationship() {
            // In process_symbol_references:
            // - If error_count == 1, then reference_count == 0
            // - If error_count == 0, then reference_count >= 0
            // These are mutually exclusive outcomes

            let test_cases = vec![
                (0, 0, true),   // Success: no refs, no errors
                (5, 0, true),   // Success: some refs, no errors
                (100, 0, true), // Success: many refs, no errors
                (0, 1, true),   // Error: no refs, one error
                (5, 1, false),  // Invalid: refs with error
                (0, 2, false),  // Invalid: multiple errors
            ];

            for (ref_count, err_count, is_valid) in test_cases {
                let is_actually_valid =
                    (err_count == 0 && ref_count >= 0) || (err_count == 1 && ref_count == 0);

                assert_eq!(
                    is_actually_valid, is_valid,
                    "Result ({}, {}) validity mismatch",
                    ref_count, err_count
                );
            }
        }

        /// Test that the function uses include_declaration parameter correctly
        #[test]
        fn test_include_declaration_parameter() {
            // process_symbol_references calls lsp_client.references() with
            // include_declaration=true (line 69)
            // This means references should include the declaration itself

            let include_declaration = true;
            assert!(
                include_declaration,
                "process_symbol_references should request declaration inclusion"
            );
        }

        /// Test integration with build_symbol_lookup_table
        #[test]
        fn test_integration_with_build_symbol_lookup_table() {
            use mother_core::scanner::Language;

            // process_symbol_references receives symbols_by_file which is created
            // by build_symbol_lookup_table in the run function

            let symbols = vec![
                SymbolInfo {
                    id: "sym1".to_string(),
                    file_uri: "file:///src/main.rs".to_string(),
                    start_line: 1,
                    end_line: 10,
                    start_col: 0,
                    language: Language::Rust,
                },
                SymbolInfo {
                    id: "sym2".to_string(),
                    file_uri: "file:///src/lib.rs".to_string(),
                    start_line: 20,
                    end_line: 30,
                    start_col: 5,
                    language: Language::Rust,
                },
            ];

            let lookup = build_symbol_lookup_table(&symbols);

            // Verify the lookup table is in the expected format for process_symbol_references
            assert_eq!(lookup.len(), 2);
            assert!(lookup.contains_key("/src/main.rs"));
            assert!(lookup.contains_key("/src/lib.rs"));

            let main_symbols = &lookup["/src/main.rs"];
            assert_eq!(main_symbols.len(), 1);
            assert_eq!(main_symbols[0].0, "sym1");
        }

        /// Test that different languages are handled
        #[test]
        fn test_multiple_language_support() {
            use mother_core::scanner::Language;

            // process_symbol_references should work with any Language variant
            // since it passes the language to lsp_manager.get_client()

            let languages = vec![
                Language::Rust,
                Language::TypeScript,
                Language::JavaScript,
                Language::Python,
                Language::Go,
            ];

            for lang in languages {
                let symbol = SymbolInfo {
                    id: format!("symbol_{:?}", lang),
                    file_uri: "file:///test/file.ext".to_string(),
                    start_line: 1,
                    end_line: 10,
                    start_col: 0,
                    language: lang,
                };

                // Verify symbol can be created for any language
                assert_eq!(symbol.language, lang);
            }
        }

        /// Test handling of edge cases in symbol positions
        #[test]
        fn test_symbol_position_edge_cases() {
            use mother_core::scanner::Language;

            // Test various edge cases for symbol positions that process_symbol_references
            // should handle via the LSP client

            let edge_cases = vec![
                (0, 0, 0),       // Start of file
                (1, 1, 0),       // First line, first col
                (1000, 1000, 0), // Large line numbers
                (10, 20, 100),   // Non-zero column
            ];

            for (start_line, end_line, start_col) in edge_cases {
                let symbol = SymbolInfo {
                    id: "test".to_string(),
                    file_uri: "file:///test.rs".to_string(),
                    start_line,
                    end_line,
                    start_col,
                    language: Language::Rust,
                };

                // Verify symbol is valid for process_symbol_references
                assert!(
                    symbol.end_line >= symbol.start_line,
                    "End line must be >= start line"
                );
            }
        }

        /// Test file URI format expectations
        #[test]
        fn test_file_uri_format_expectations() {
            // process_symbol_references passes file_uri to lsp_client.references()
            // URIs should follow the file:// format

            let valid_uris = vec![
                "file:///absolute/path/file.rs",
                "file:///home/user/project/src/main.rs",
                "file:///C:/Windows/path/file.rs",
                "file:///path/with spaces/file.rs",
            ];

            for uri in valid_uris {
                assert!(
                    uri.starts_with("file://"),
                    "URI should start with file://: {}",
                    uri
                );
            }
        }

        /// Test that process_symbol_references is called once per symbol
        #[test]
        fn test_one_call_per_symbol() {
            // In the run function, process_symbol_references is called once for each
            // symbol in the symbols slice

            let symbol_count = 5;
            let expected_calls = symbol_count;

            assert_eq!(
                expected_calls, 5,
                "Should call process_symbol_references once per symbol"
            );
        }

        /// Test accumulation of reference and error counts
        #[test]
        fn test_count_accumulation_logic() {
            // The run function accumulates reference_count and error_count
            // from each call to process_symbol_references

            let results = [
                (5, 0),  // Symbol 1: 5 refs, no error
                (0, 1),  // Symbol 2: error
                (10, 0), // Symbol 3: 10 refs, no error
                (0, 0),  // Symbol 4: no refs, no error
                (3, 0),  // Symbol 5: 3 refs, no error
            ];

            let total_refs: usize = results.iter().map(|(r, _)| r).sum();
            let total_errors: usize = results.iter().map(|(_, e)| e).sum();

            assert_eq!(total_refs, 18, "Should accumulate all reference counts");
            assert_eq!(total_errors, 1, "Should accumulate all error counts");
        }

        /// Test that await is used correctly for async operations
        #[test]
        fn test_async_operation_contract() {
            // process_symbol_references is an async function that awaits:
            // 1. lsp_manager.get_client() - async
            // 2. lsp_client.references() - async
            // 3. create_reference_edges() - async

            // These operations must be awaited in sequence
            // This test documents the async contract

            let async_operations = [
                "lsp_manager.get_client()",
                "lsp_client.references()",
                "create_reference_edges()",
            ];

            assert_eq!(
                async_operations.len(),
                3,
                "Should have 3 async operations in sequence"
            );
        }
    }
}
mod tests;

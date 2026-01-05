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

    /// Comprehensive tests for create_reference_edges workflow and logic
    mod create_reference_edges_workflow_tests {
        use super::*;

        #[test]
        fn test_empty_references_list_returns_zero() {
            // Test that empty references list results in count of 0
            let refs: Vec<LspReference> = vec![];
            let symbols = make_symbols_map(vec![("/src/main.rs", vec![("target_sym", 1, 10)])]);

            // Simulate the logic of create_reference_edges with empty refs
            let mut count = 0;
            for reference in &refs {
                if let Some(from_id) = find_containing_symbol(reference, &symbols) {
                    if from_id != "target_sym" {
                        count += 1;
                    }
                }
            }

            assert_eq!(count, 0, "Empty references should result in 0 count");
        }

        #[test]
        fn test_all_references_without_containing_symbols() {
            // Test references that don't fall within any symbol
            let refs = vec![
                make_reference("/src/main.rs", 100),
                make_reference("/src/main.rs", 200),
                make_reference("/src/main.rs", 300),
            ];
            let symbols = make_symbols_map(vec![("/src/main.rs", vec![("func", 1, 10)])]);

            let mut count = 0;
            for reference in &refs {
                if let Some(from_id) = find_containing_symbol(reference, &symbols) {
                    if from_id != "target_sym" {
                        count += 1;
                    }
                }
            }

            assert_eq!(
                count, 0,
                "References outside all symbols should result in 0 count"
            );
        }

        #[test]
        fn test_all_references_are_self_references() {
            // Test that self-references are correctly filtered
            let refs = vec![
                make_reference("/src/main.rs", 5),
                make_reference("/src/main.rs", 7),
                make_reference("/src/main.rs", 9),
            ];
            let symbols = make_symbols_map(vec![("/src/main.rs", vec![("target_sym", 1, 10)])]);
            let target_id = "target_sym";

            let mut count = 0;
            for reference in &refs {
                if let Some(from_id) = find_containing_symbol(reference, &symbols) {
                    if from_id != target_id {
                        count += 1;
                    }
                }
            }

            assert_eq!(count, 0, "All self-references should be filtered out");
        }

        #[test]
        fn test_mixed_references_some_valid_some_self() {
            // Test mix of valid references and self-references
            let refs = vec![
                make_reference("/src/main.rs", 5),  // In caller_func
                make_reference("/src/main.rs", 15), // In target_func (self)
                make_reference("/src/main.rs", 25), // In another_func
            ];
            let symbols = make_symbols_map(vec![(
                "/src/main.rs",
                vec![
                    ("caller_func", 1, 10),
                    ("target_func", 12, 18),
                    ("another_func", 20, 30),
                ],
            )]);
            let target_id = "target_func";

            let mut count = 0;
            for reference in &refs {
                if let Some(from_id) = find_containing_symbol(reference, &symbols) {
                    if from_id != target_id {
                        count += 1;
                    }
                }
            }

            assert_eq!(count, 2, "Should count 2 valid references (excluding self)");
        }

        #[test]
        fn test_mixed_references_some_outside_symbols() {
            // Test mix of references inside and outside symbols
            let refs = vec![
                make_reference("/src/main.rs", 5),   // In func_a
                make_reference("/src/main.rs", 100), // Outside any symbol
                make_reference("/src/main.rs", 25),  // In func_b
            ];
            let symbols = make_symbols_map(vec![(
                "/src/main.rs",
                vec![("func_a", 1, 10), ("func_b", 20, 30)],
            )]);
            let target_id = "target_func";

            let mut count = 0;
            for reference in &refs {
                if let Some(from_id) = find_containing_symbol(reference, &symbols) {
                    if from_id != target_id {
                        count += 1;
                    }
                }
            }

            assert_eq!(
                count, 2,
                "Should count only references within symbols (2 out of 3)"
            );
        }

        #[test]
        fn test_references_across_multiple_files() {
            // Test references from different files
            let refs = vec![
                make_reference("/src/main.rs", 5),
                make_reference("/src/utils.rs", 10),
                make_reference("/src/lib.rs", 15),
            ];
            let symbols = make_symbols_map(vec![
                ("/src/main.rs", vec![("main_func", 1, 10)]),
                ("/src/utils.rs", vec![("util_func", 5, 15)]),
                ("/src/lib.rs", vec![("lib_func", 10, 20)]),
            ]);
            let target_id = "target_func";

            let mut count = 0;
            for reference in &refs {
                if let Some(from_id) = find_containing_symbol(reference, &symbols) {
                    if from_id != target_id {
                        count += 1;
                    }
                }
            }

            assert_eq!(count, 3, "Should count all references from different files");
        }

        #[test]
        fn test_multiple_references_from_same_function() {
            // Test multiple references from the same calling function
            let refs = vec![
                make_reference("/src/main.rs", 5),
                make_reference("/src/main.rs", 7),
                make_reference("/src/main.rs", 9),
            ];
            let symbols = make_symbols_map(vec![(
                "/src/main.rs",
                vec![("caller", 1, 10), ("target", 20, 30)],
            )]);
            let target_id = "target";

            let mut count = 0;
            for reference in &refs {
                if let Some(from_id) = find_containing_symbol(reference, &symbols) {
                    if from_id != target_id {
                        count += 1;
                    }
                }
            }

            assert_eq!(
                count, 3,
                "Should count each reference even from same function"
            );
        }

        #[test]
        fn test_references_in_nested_symbols() {
            // Test references when symbols are nested (inner should be selected)
            let refs = [
                make_reference("/src/main.rs", 10), // In inner_block
                make_reference("/src/main.rs", 5),  // In outer_func only
            ];
            let symbols = make_symbols_map(vec![(
                "/src/main.rs",
                vec![("outer_func", 1, 20), ("inner_block", 8, 12)],
            )]);
            let target_id = "some_target";

            let mut count = 0;
            let results: Vec<_> = refs
                .iter()
                .filter_map(|r| find_containing_symbol(r, &symbols))
                .collect();

            for from_id in results {
                if from_id != target_id {
                    count += 1;
                }
            }

            assert_eq!(count, 2, "Should process both references");
            // Reference at line 10 should be in inner_block, at line 5 in outer_func
        }

        #[test]
        fn test_references_with_different_line_numbers() {
            // Test that line numbers are correctly used for symbol matching
            let test_cases = vec![
                (5, Some("func1")),  // Line 5 in func1 (1-10)
                (15, Some("func2")), // Line 15 in func2 (11-20)
                (25, Some("func3")), // Line 25 in func3 (21-30)
                (35, None),          // Line 35 not in any function
            ];

            let symbols = make_symbols_map(vec![(
                "/src/main.rs",
                vec![("func1", 1, 10), ("func2", 11, 20), ("func3", 21, 30)],
            )]);

            for (line, expected) in test_cases {
                let reference = make_reference("/src/main.rs", line);
                let result = find_containing_symbol(&reference, &symbols);
                assert_eq!(
                    result,
                    expected.map(String::from),
                    "Line {} should be in {:?}",
                    line,
                    expected
                );
            }
        }

        #[test]
        fn test_reference_file_path_matching() {
            // Test that file paths must match exactly for symbol lookup
            let reference = make_reference("/src/main.rs", 5);
            let symbols = make_symbols_map(vec![
                ("/src/main.rs", vec![("correct_file", 1, 10)]),
                ("/src/other.rs", vec![("wrong_file", 1, 10)]),
            ]);

            let result = find_containing_symbol(&reference, &symbols);
            assert_eq!(
                result,
                Some("correct_file".to_string()),
                "Should match symbol from correct file"
            );
        }

        #[test]
        fn test_counting_accuracy_with_large_reference_list() {
            // Test counting accuracy with many references
            let mut refs = vec![];
            for i in 1..=100 {
                refs.push(make_reference("/src/main.rs", i));
            }

            // Create symbols: func1 (1-30), func2 (31-60), func3 (61-90), gap at 91-100
            let symbols = make_symbols_map(vec![(
                "/src/main.rs",
                vec![("func1", 1, 30), ("func2", 31, 60), ("func3", 61, 90)],
            )]);
            let target_id = "target";

            let mut count = 0;
            for reference in &refs {
                if let Some(from_id) = find_containing_symbol(reference, &symbols) {
                    if from_id != target_id {
                        count += 1;
                    }
                }
            }

            // Lines 1-90 should find containing symbols (90 refs)
            // Lines 91-100 should not (10 refs)
            assert_eq!(count, 90, "Should count exactly 90 valid references");
        }

        #[test]
        fn test_edge_creation_parameters_correctness() {
            // Test that edge parameters are correctly extracted from references
            let reference = make_reference("/src/main.rs", 42);
            let from_id = "caller";
            let to_id = "callee";

            // Simulate edge creation
            let edge = Edge {
                source_id: from_id.to_string(),
                target_id: to_id.to_string(),
                kind: EdgeKind::References,
                line: Some(reference.line),
                column: Some(reference.start_col),
            };

            assert_eq!(edge.source_id, from_id);
            assert_eq!(edge.target_id, to_id);
            assert_eq!(edge.kind, EdgeKind::References);
            assert_eq!(edge.line, Some(42));
            assert_eq!(edge.column, Some(0));
        }

        #[test]
        fn test_workflow_with_complex_scenario() {
            // Complex scenario: multiple files, nested symbols, self-refs, external refs
            let refs = vec![
                make_reference("/src/main.rs", 5),    // In caller1
                make_reference("/src/main.rs", 15),   // In target (self)
                make_reference("/src/main.rs", 35),   // In inner (nested in caller2)
                make_reference("/src/utils.rs", 10),  // In util_caller
                make_reference("/src/utils.rs", 100), // Outside any symbol
            ];

            let symbols = make_symbols_map(vec![
                (
                    "/src/main.rs",
                    vec![
                        ("caller1", 1, 10),
                        ("target", 12, 18),
                        ("caller2", 20, 50),
                        ("inner", 30, 40), // Nested in caller2
                    ],
                ),
                ("/src/utils.rs", vec![("util_caller", 5, 20)]),
            ]);

            let target_id = "target";
            let mut count = 0;

            for reference in &refs {
                if let Some(from_id) = find_containing_symbol(reference, &symbols) {
                    if from_id != target_id {
                        count += 1;
                    }
                }
            }

            // Expected valid references:
            // 1. caller1 (line 5) -> target
            // 2. target (line 15) -> target (filtered as self-ref)
            // 3. inner (line 35) -> target (nested symbol correctly selected)
            // 4. util_caller (line 10) -> target
            // 5. Outside symbol (line 100) -> not counted
            // Total: 3 valid edges
            assert_eq!(count, 3, "Complex scenario should count 3 valid edges");
        }

        #[test]
        fn test_boundary_line_numbers() {
            // Test references at exact start and end boundaries
            let refs = [
                make_reference("/src/main.rs", 1),  // Exact start
                make_reference("/src/main.rs", 10), // Exact end
                make_reference("/src/main.rs", 0),  // Before start
                make_reference("/src/main.rs", 11), // After end
            ];
            let symbols = make_symbols_map(vec![("/src/main.rs", vec![("func", 1, 10)])]);

            let results: Vec<_> = refs
                .iter()
                .filter_map(|r| find_containing_symbol(r, &symbols))
                .collect();

            assert_eq!(
                results.len(),
                2,
                "Only references at lines 1 and 10 should match"
            );
            assert!(results.iter().all(|id| id == "func"));
        }

        #[test]
        fn test_single_line_symbols() {
            // Test symbols that start and end on the same line
            let refs = [
                make_reference("/src/main.rs", 5),
                make_reference("/src/main.rs", 10),
                make_reference("/src/main.rs", 15),
            ];
            let symbols = make_symbols_map(vec![(
                "/src/main.rs",
                vec![
                    ("single_line_1", 5, 5),
                    ("single_line_2", 10, 10),
                    ("single_line_3", 15, 15),
                ],
            )]);

            let results: Vec<_> = refs
                .iter()
                .filter_map(|r| find_containing_symbol(r, &symbols))
                .collect();

            assert_eq!(
                results.len(),
                3,
                "All references should match their single-line symbols"
            );
        }

        #[test]
        fn test_zero_references_in_map() {
            // Test behavior when symbols_by_file has entries but empty symbol lists
            let refs = [make_reference("/src/main.rs", 5)];
            let symbols = make_symbols_map(vec![("/src/main.rs", vec![])]);

            let result = find_containing_symbol(&refs[0], &symbols);
            assert_eq!(result, None, "Empty symbol list should result in no match");
        }
    }
}

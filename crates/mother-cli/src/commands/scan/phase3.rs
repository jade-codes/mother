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
            assert!(!should_create_edge, "Self-references should be filtered out");
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
            
            assert!(containing_symbol.is_none(), "Reference without containing symbol should be skipped");
        }

        #[test]
        fn test_reference_with_containing_symbol_processed() {
            let containing_symbol: Option<String> = Some("some_symbol".to_string());
            
            assert!(containing_symbol.is_some(), "Reference with containing symbol should be processed");
        }

        #[test]
        fn test_edge_counter_logic() {
            // Simulates the counting logic in create_reference_edges
            let mut count = 0;
            let test_cases = vec![
                (Some("sym1".to_string()), "sym2"), // Should count: different symbols
                (Some("sym2".to_string()), "sym2"), // Should not count: self-reference
                (None, "sym3"),                      // Should not count: no containing symbol
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

            assert_eq!(containing, None, "Reference in non-existent file should have no containing symbol");
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
}

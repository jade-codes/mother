//! Tests for build_symbol_lookup_table function

use super::super::{build_symbol_lookup_table, SymbolInfo};
use mother_core::scanner::Language;

#[test]
fn test_build_symbol_lookup_table_empty() {
    let symbols: Vec<SymbolInfo> = vec![];
    let result = build_symbol_lookup_table(&symbols);
    assert!(result.is_empty());
}

#[test]
fn test_build_symbol_lookup_table_strips_file_prefix() {
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

#[test]
fn test_build_symbol_lookup_table_preserves_tuple_order() {
    // Verify tuple order is (id, start_line, end_line)
    let symbols = vec![SymbolInfo {
        id: "test_symbol".to_string(),
        file_uri: "file:///test.rs".to_string(),
        start_line: 5,
        end_line: 20,
        start_col: 0,
        language: Language::Rust,
    }];

    let result = build_symbol_lookup_table(&symbols);
    let file_symbols = &result["/test.rs"];
    assert_eq!(file_symbols[0].0, "test_symbol");
    assert_eq!(file_symbols[0].1, 5);
    assert_eq!(file_symbols[0].2, 20);
}

#[test]
fn test_build_symbol_lookup_table_multiple_files() {
    let symbols = vec![
        SymbolInfo {
            id: "sym1".to_string(),
            file_uri: "file:///src/a.rs".to_string(),
            start_line: 1,
            end_line: 10,
            start_col: 0,
            language: Language::Rust,
        },
        SymbolInfo {
            id: "sym2".to_string(),
            file_uri: "file:///src/b.rs".to_string(),
            start_line: 1,
            end_line: 5,
            start_col: 0,
            language: Language::Rust,
        },
        SymbolInfo {
            id: "sym3".to_string(),
            file_uri: "file:///src/c.rs".to_string(),
            start_line: 1,
            end_line: 15,
            start_col: 0,
            language: Language::Rust,
        },
    ];

    let result = build_symbol_lookup_table(&symbols);
    assert_eq!(result.len(), 3);
    assert!(result.contains_key("/src/a.rs"));
    assert!(result.contains_key("/src/b.rs"));
    assert!(result.contains_key("/src/c.rs"));
}

#[test]
fn test_build_symbol_lookup_table_duplicate_ids_different_files() {
    // Same symbol ID in different files should both be present
    let symbols = vec![
        SymbolInfo {
            id: "main".to_string(),
            file_uri: "file:///src/main.rs".to_string(),
            start_line: 1,
            end_line: 10,
            start_col: 0,
            language: Language::Rust,
        },
        SymbolInfo {
            id: "main".to_string(),
            file_uri: "file:///src/utils.rs".to_string(),
            start_line: 5,
            end_line: 15,
            start_col: 0,
            language: Language::Rust,
        },
    ];

    let result = build_symbol_lookup_table(&symbols);
    assert_eq!(result.len(), 2);

    let main_symbols = &result["/src/main.rs"];
    assert_eq!(main_symbols.len(), 1);
    assert_eq!(main_symbols[0].0, "main");

    let utils_symbols = &result["/src/utils.rs"];
    assert_eq!(utils_symbols.len(), 1);
    assert_eq!(utils_symbols[0].0, "main");
}

#[test]
fn test_build_symbol_lookup_table_mixed_file_uri_formats() {
    // Mix of file:// prefix and absolute paths
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
            file_uri: "/src/utils.rs".to_string(),
            start_line: 1,
            end_line: 5,
            start_col: 0,
            language: Language::Rust,
        },
    ];

    let result = build_symbol_lookup_table(&symbols);
    assert_eq!(result.len(), 2);
    assert!(result.contains_key("/src/main.rs"));
    assert!(result.contains_key("/src/utils.rs"));
}

#[test]
fn test_build_symbol_lookup_table_preserves_insertion_order() {
    // Symbols should be added in the order they appear in the input
    let symbols = vec![
        SymbolInfo {
            id: "third".to_string(),
            file_uri: "file:///test.rs".to_string(),
            start_line: 30,
            end_line: 40,
            start_col: 0,
            language: Language::Rust,
        },
        SymbolInfo {
            id: "first".to_string(),
            file_uri: "file:///test.rs".to_string(),
            start_line: 1,
            end_line: 10,
            start_col: 0,
            language: Language::Rust,
        },
        SymbolInfo {
            id: "second".to_string(),
            file_uri: "file:///test.rs".to_string(),
            start_line: 15,
            end_line: 25,
            start_col: 0,
            language: Language::Rust,
        },
    ];

    let result = build_symbol_lookup_table(&symbols);
    let file_symbols = &result["/test.rs"];

    assert_eq!(file_symbols.len(), 3);
    // Order should be preserved as inserted
    assert_eq!(file_symbols[0].0, "third");
    assert_eq!(file_symbols[1].0, "first");
    assert_eq!(file_symbols[2].0, "second");
}

#[test]
fn test_build_symbol_lookup_table_boundary_line_numbers() {
    // Test with line number 0 and very large line numbers
    let symbols = vec![
        SymbolInfo {
            id: "at_zero".to_string(),
            file_uri: "file:///test.rs".to_string(),
            start_line: 0,
            end_line: 0,
            start_col: 0,
            language: Language::Rust,
        },
        SymbolInfo {
            id: "large_line".to_string(),
            file_uri: "file:///test.rs".to_string(),
            start_line: 999999,
            end_line: 1000000,
            start_col: 0,
            language: Language::Rust,
        },
    ];

    let result = build_symbol_lookup_table(&symbols);
    let file_symbols = &result["/test.rs"];

    assert_eq!(file_symbols.len(), 2);
    assert_eq!(file_symbols[0].1, 0);
    assert_eq!(file_symbols[0].2, 0);
    assert_eq!(file_symbols[1].1, 999999);
    assert_eq!(file_symbols[1].2, 1000000);
}

#[test]
fn test_build_symbol_lookup_table_special_characters_in_path() {
    // Test with special characters in file paths
    let symbols = vec![
        SymbolInfo {
            id: "sym1".to_string(),
            file_uri: "file:///src/my-file.rs".to_string(),
            start_line: 1,
            end_line: 10,
            start_col: 0,
            language: Language::Rust,
        },
        SymbolInfo {
            id: "sym2".to_string(),
            file_uri: "file:///src/file_with_underscore.rs".to_string(),
            start_line: 1,
            end_line: 5,
            start_col: 0,
            language: Language::Rust,
        },
        SymbolInfo {
            id: "sym3".to_string(),
            file_uri: "file:///src/file.test.rs".to_string(),
            start_line: 1,
            end_line: 5,
            start_col: 0,
            language: Language::Rust,
        },
    ];

    let result = build_symbol_lookup_table(&symbols);
    assert_eq!(result.len(), 3);
    assert!(result.contains_key("/src/my-file.rs"));
    assert!(result.contains_key("/src/file_with_underscore.rs"));
    assert!(result.contains_key("/src/file.test.rs"));
}

#[test]
fn test_build_symbol_lookup_table_deep_nested_paths() {
    let symbols = vec![SymbolInfo {
        id: "sym1".to_string(),
        file_uri: "file:///src/deeply/nested/path/to/file.rs".to_string(),
        start_line: 1,
        end_line: 10,
        start_col: 0,
        language: Language::Rust,
    }];

    let result = build_symbol_lookup_table(&symbols);
    assert_eq!(result.len(), 1);
    assert!(result.contains_key("/src/deeply/nested/path/to/file.rs"));
}

#[test]
fn test_build_symbol_lookup_table_single_line_symbols() {
    // Symbols that start and end on the same line
    let symbols = vec![
        SymbolInfo {
            id: "single1".to_string(),
            file_uri: "file:///test.rs".to_string(),
            start_line: 5,
            end_line: 5,
            start_col: 0,
            language: Language::Rust,
        },
        SymbolInfo {
            id: "single2".to_string(),
            file_uri: "file:///test.rs".to_string(),
            start_line: 10,
            end_line: 10,
            start_col: 0,
            language: Language::Rust,
        },
    ];

    let result = build_symbol_lookup_table(&symbols);
    let file_symbols = &result["/test.rs"];

    assert_eq!(file_symbols.len(), 2);
    assert_eq!(file_symbols[0].1, file_symbols[0].2); // start == end
    assert_eq!(file_symbols[1].1, file_symbols[1].2); // start == end
}

#[test]
fn test_build_symbol_lookup_table_large_number_of_symbols() {
    // Stress test with many symbols in the same file
    let mut symbols = Vec::new();
    for i in 0..1000 {
        symbols.push(SymbolInfo {
            id: format!("symbol_{}", i),
            file_uri: "file:///large.rs".to_string(),
            start_line: i * 10,
            end_line: i * 10 + 5,
            start_col: 0,
            language: Language::Rust,
        });
    }

    let result = build_symbol_lookup_table(&symbols);
    assert_eq!(result.len(), 1);

    let file_symbols = &result["/large.rs"];
    assert_eq!(file_symbols.len(), 1000);

    // Verify first and last entries
    assert_eq!(file_symbols[0].0, "symbol_0");
    assert_eq!(file_symbols[999].0, "symbol_999");
}

#[test]
fn test_build_symbol_lookup_table_ignores_start_col() {
    // start_col should not affect the lookup table (not part of tuple)
    let symbols = vec![
        SymbolInfo {
            id: "sym1".to_string(),
            file_uri: "file:///test.rs".to_string(),
            start_line: 1,
            end_line: 10,
            start_col: 0,
            language: Language::Rust,
        },
        SymbolInfo {
            id: "sym2".to_string(),
            file_uri: "file:///test.rs".to_string(),
            start_line: 5,
            end_line: 8,
            start_col: 42,
            language: Language::Rust,
        },
    ];

    let result = build_symbol_lookup_table(&symbols);
    let file_symbols = &result["/test.rs"];

    assert_eq!(file_symbols.len(), 2);
    // Tuple should only have id, start_line, end_line
    assert_eq!(file_symbols[0], ("sym1".to_string(), 1, 10));
    assert_eq!(file_symbols[1], ("sym2".to_string(), 5, 8));
}

#[test]
fn test_build_symbol_lookup_table_different_languages() {
    // Different languages in the same file (edge case but should work)
    let symbols = vec![
        SymbolInfo {
            id: "rust_sym".to_string(),
            file_uri: "file:///test.rs".to_string(),
            start_line: 1,
            end_line: 10,
            start_col: 0,
            language: Language::Rust,
        },
        SymbolInfo {
            id: "python_sym".to_string(),
            file_uri: "file:///test.py".to_string(),
            start_line: 1,
            end_line: 5,
            start_col: 0,
            language: Language::Python,
        },
    ];

    let result = build_symbol_lookup_table(&symbols);
    assert_eq!(result.len(), 2);
    assert!(result.contains_key("/test.rs"));
    assert!(result.contains_key("/test.py"));
}

#[test]
fn test_build_symbol_lookup_table_windows_style_paths() {
    // Test Windows-style paths (if they appear after file://)
    let symbols = vec![SymbolInfo {
        id: "sym1".to_string(),
        file_uri: "file:///C:/Users/project/src/main.rs".to_string(),
        start_line: 1,
        end_line: 10,
        start_col: 0,
        language: Language::Rust,
    }];

    let result = build_symbol_lookup_table(&symbols);
    assert_eq!(result.len(), 1);
    assert!(result.contains_key("/C:/Users/project/src/main.rs"));
}

#[test]
fn test_build_symbol_lookup_table_overlapping_line_ranges() {
    // Symbols with overlapping line ranges in the same file
    let symbols = vec![
        SymbolInfo {
            id: "outer".to_string(),
            file_uri: "file:///test.rs".to_string(),
            start_line: 1,
            end_line: 100,
            start_col: 0,
            language: Language::Rust,
        },
        SymbolInfo {
            id: "inner".to_string(),
            file_uri: "file:///test.rs".to_string(),
            start_line: 20,
            end_line: 30,
            start_col: 0,
            language: Language::Rust,
        },
    ];

    let result = build_symbol_lookup_table(&symbols);
    let file_symbols = &result["/test.rs"];

    assert_eq!(file_symbols.len(), 2);
    assert_eq!(file_symbols[0].0, "outer");
    assert_eq!(file_symbols[1].0, "inner");
}

#[test]
fn test_build_symbol_lookup_table_empty_id() {
    // Edge case: symbol with empty ID string
    let symbols = vec![SymbolInfo {
        id: "".to_string(),
        file_uri: "file:///test.rs".to_string(),
        start_line: 1,
        end_line: 10,
        start_col: 0,
        language: Language::Rust,
    }];

    let result = build_symbol_lookup_table(&symbols);
    let file_symbols = &result["/test.rs"];

    assert_eq!(file_symbols.len(), 1);
    assert_eq!(file_symbols[0].0, "");
}

#[test]
fn test_build_symbol_lookup_table_very_long_id() {
    // Edge case: very long symbol ID
    let long_id = "a".repeat(1000);
    let symbols = vec![SymbolInfo {
        id: long_id.clone(),
        file_uri: "file:///test.rs".to_string(),
        start_line: 1,
        end_line: 10,
        start_col: 0,
        language: Language::Rust,
    }];

    let result = build_symbol_lookup_table(&symbols);
    let file_symbols = &result["/test.rs"];

    assert_eq!(file_symbols.len(), 1);
    assert_eq!(file_symbols[0].0, long_id);
}

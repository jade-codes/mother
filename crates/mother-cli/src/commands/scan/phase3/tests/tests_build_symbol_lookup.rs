//! Tests for build_symbol_lookup_table function

use super::super::build_symbol_lookup_table;
use crate::commands::scan::SymbolInfo;
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

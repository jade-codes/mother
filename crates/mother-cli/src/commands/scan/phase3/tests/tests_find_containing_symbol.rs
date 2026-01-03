//! Tests for find_containing_symbol function

use super::super::{build_symbol_lookup_table, find_containing_symbol};
use mother_core::lsp::LspReference;
use mother_core::scanner::Language;
use std::collections::HashMap;
use std::path::PathBuf;

// Helper to create a test reference
fn make_reference(file: &str, line: u32) -> LspReference {
    LspReference {
        file: PathBuf::from(file),
        line,
        start_col: 0,
        end_col: 10,
    }
}

// Helper to create a symbols_by_file HashMap
#[allow(clippy::type_complexity)]
fn make_symbols_by_file(
    entries: Vec<(&str, Vec<(&str, u32, u32)>)>,
) -> HashMap<String, Vec<(String, u32, u32)>> {
    entries
        .into_iter()
        .map(|(file, symbols)| {
            let symbols_vec = symbols
                .into_iter()
                .map(|(id, start, end)| (id.to_string(), start, end))
                .collect();
            (file.to_string(), symbols_vec)
        })
        .collect()
}

#[test]
fn test_find_containing_symbol_empty_map() {
    let symbols_by_file: HashMap<String, Vec<(String, u32, u32)>> = HashMap::new();
    let reference = make_reference("/path/to/file.rs", 10);

    let result = find_containing_symbol(&reference, &symbols_by_file);

    assert_eq!(result, None);
}

#[test]
fn test_find_containing_symbol_file_not_in_map() {
    let symbols_by_file = make_symbols_by_file(vec![("/path/to/other.rs", vec![("sym1", 1, 20)])]);
    let reference = make_reference("/path/to/file.rs", 10);

    let result = find_containing_symbol(&reference, &symbols_by_file);

    assert_eq!(result, None);
}

#[test]
fn test_find_containing_symbol_outside_all_ranges() {
    let symbols_by_file = make_symbols_by_file(vec![(
        "/path/to/file.rs",
        vec![("sym1", 1, 5), ("sym2", 10, 15), ("sym3", 20, 30)],
    )]);
    let reference = make_reference("/path/to/file.rs", 7);

    let result = find_containing_symbol(&reference, &symbols_by_file);

    assert_eq!(result, None);
}

#[test]
fn test_find_containing_symbol_inside_single_range() {
    let symbols_by_file = make_symbols_by_file(vec![(
        "/path/to/file.rs",
        vec![("sym1", 1, 5), ("sym2", 10, 20), ("sym3", 25, 30)],
    )]);
    let reference = make_reference("/path/to/file.rs", 15);

    let result = find_containing_symbol(&reference, &symbols_by_file);

    assert_eq!(result, Some("sym2".to_string()));
}

#[test]
fn test_find_containing_symbol_at_start_boundary() {
    let symbols_by_file = make_symbols_by_file(vec![("/path/to/file.rs", vec![("sym1", 10, 20)])]);
    let reference = make_reference("/path/to/file.rs", 10);

    let result = find_containing_symbol(&reference, &symbols_by_file);

    assert_eq!(result, Some("sym1".to_string()));
}

#[test]
fn test_find_containing_symbol_at_end_boundary() {
    let symbols_by_file = make_symbols_by_file(vec![("/path/to/file.rs", vec![("sym1", 10, 20)])]);
    let reference = make_reference("/path/to/file.rs", 20);

    let result = find_containing_symbol(&reference, &symbols_by_file);

    assert_eq!(result, Some("sym1".to_string()));
}

#[test]
fn test_find_containing_symbol_smallest_wins() {
    // Test that when multiple symbols contain the reference,
    // the smallest one (by line count) is returned
    let symbols_by_file = make_symbols_by_file(vec![(
        "/path/to/file.rs",
        vec![
            ("outer", 1, 100),  // 100 lines
            ("middle", 10, 50), // 41 lines
            ("inner", 20, 30),  // 11 lines
        ],
    )]);
    let reference = make_reference("/path/to/file.rs", 25);

    let result = find_containing_symbol(&reference, &symbols_by_file);

    assert_eq!(result, Some("inner".to_string()));
}

#[test]
fn test_find_containing_symbol_nested_symbols() {
    // Test deeply nested symbols (e.g., class > method > block)
    let symbols_by_file = make_symbols_by_file(vec![(
        "/path/to/file.rs",
        vec![
            ("class", 1, 100),
            ("method1", 5, 30),
            ("block1", 10, 15),
            ("method2", 40, 60),
        ],
    )]);

    // Reference in block1
    let reference = make_reference("/path/to/file.rs", 12);
    let result = find_containing_symbol(&reference, &symbols_by_file);
    assert_eq!(result, Some("block1".to_string()));

    // Reference in method1 but outside block1
    let reference = make_reference("/path/to/file.rs", 20);
    let result = find_containing_symbol(&reference, &symbols_by_file);
    assert_eq!(result, Some("method1".to_string()));

    // Reference in method2
    let reference = make_reference("/path/to/file.rs", 45);
    let result = find_containing_symbol(&reference, &symbols_by_file);
    assert_eq!(result, Some("method2".to_string()));
}

#[test]
fn test_find_containing_symbol_same_size_symbols() {
    // When multiple symbols have the same size, min_by_key returns the first one
    let symbols_by_file = make_symbols_by_file(vec![(
        "/path/to/file.rs",
        vec![
            ("sym1", 10, 20), // 11 lines
            ("sym2", 10, 20), // 11 lines (same range)
        ],
    )]);
    let reference = make_reference("/path/to/file.rs", 15);

    let result = find_containing_symbol(&reference, &symbols_by_file);

    // Should return sym1 since it appears first
    assert_eq!(result, Some("sym1".to_string()));
}

#[test]
fn test_find_containing_symbol_before_first_symbol() {
    let symbols_by_file = make_symbols_by_file(vec![("/path/to/file.rs", vec![("sym1", 10, 20)])]);
    let reference = make_reference("/path/to/file.rs", 5);

    let result = find_containing_symbol(&reference, &symbols_by_file);

    assert_eq!(result, None);
}

#[test]
fn test_find_containing_symbol_after_last_symbol() {
    let symbols_by_file = make_symbols_by_file(vec![("/path/to/file.rs", vec![("sym1", 10, 20)])]);
    let reference = make_reference("/path/to/file.rs", 25);

    let result = find_containing_symbol(&reference, &symbols_by_file);

    assert_eq!(result, None);
}

#[test]
fn test_find_containing_symbol_multiple_files() {
    let symbols_by_file = make_symbols_by_file(vec![
        ("/path/to/file1.rs", vec![("sym1", 10, 20)]),
        ("/path/to/file2.rs", vec![("sym2", 10, 20)]),
    ]);

    let reference1 = make_reference("/path/to/file1.rs", 15);
    let result1 = find_containing_symbol(&reference1, &symbols_by_file);
    assert_eq!(result1, Some("sym1".to_string()));

    let reference2 = make_reference("/path/to/file2.rs", 15);
    let result2 = find_containing_symbol(&reference2, &symbols_by_file);
    assert_eq!(result2, Some("sym2".to_string()));
}

#[test]
fn test_find_containing_symbol_single_line_symbol() {
    // Test a symbol that spans exactly one line
    let symbols_by_file =
        make_symbols_by_file(vec![("/path/to/file.rs", vec![("single_line", 10, 10)])]);
    let reference = make_reference("/path/to/file.rs", 10);

    let result = find_containing_symbol(&reference, &symbols_by_file);

    assert_eq!(result, Some("single_line".to_string()));
}

#[test]
fn test_find_containing_symbol_adjacent_symbols() {
    // Test symbols that are adjacent but not overlapping
    let symbols_by_file = make_symbols_by_file(vec![(
        "/path/to/file.rs",
        vec![("sym1", 1, 10), ("sym2", 11, 20), ("sym3", 21, 30)],
    )]);

    // At end of sym1
    let reference = make_reference("/path/to/file.rs", 10);
    assert_eq!(
        find_containing_symbol(&reference, &symbols_by_file),
        Some("sym1".to_string())
    );

    // At start of sym2
    let reference = make_reference("/path/to/file.rs", 11);
    assert_eq!(
        find_containing_symbol(&reference, &symbols_by_file),
        Some("sym2".to_string())
    );

    // Between sym1 and sym2 (should be None, but 10 and 11 are covered above)
    // This tests the gap doesn't exist since symbols are inclusive of boundaries
}

#[test]
fn test_build_symbol_lookup_table_empty() {
    let symbols = vec![];
    let result = build_symbol_lookup_table(&symbols);
    assert!(result.is_empty());
}

#[test]
fn test_build_symbol_lookup_table_single_file() {
    use crate::commands::scan::SymbolInfo;

    let symbols = vec![
        SymbolInfo {
            id: "sym1".to_string(),
            file_uri: "file:///path/to/file.rs".to_string(),
            start_line: 10,
            end_line: 20,
            start_col: 0,
            language: Language::Rust,
        },
        SymbolInfo {
            id: "sym2".to_string(),
            file_uri: "file:///path/to/file.rs".to_string(),
            start_line: 30,
            end_line: 40,
            start_col: 0,
            language: Language::Rust,
        },
    ];

    let result = build_symbol_lookup_table(&symbols);

    assert_eq!(result.len(), 1);
    assert!(result.contains_key("/path/to/file.rs"));

    let file_symbols = &result["/path/to/file.rs"];
    assert_eq!(file_symbols.len(), 2);
    assert_eq!(file_symbols[0], ("sym1".to_string(), 10, 20));
    assert_eq!(file_symbols[1], ("sym2".to_string(), 30, 40));
}

#[test]
fn test_build_symbol_lookup_table_multiple_files() {
    use crate::commands::scan::SymbolInfo;

    let symbols = vec![
        SymbolInfo {
            id: "sym1".to_string(),
            file_uri: "file:///path/to/file1.rs".to_string(),
            start_line: 10,
            end_line: 20,
            start_col: 0,
            language: Language::Rust,
        },
        SymbolInfo {
            id: "sym2".to_string(),
            file_uri: "file:///path/to/file2.rs".to_string(),
            start_line: 30,
            end_line: 40,
            start_col: 0,
            language: Language::Rust,
        },
    ];

    let result = build_symbol_lookup_table(&symbols);

    assert_eq!(result.len(), 2);
    assert!(result.contains_key("/path/to/file1.rs"));
    assert!(result.contains_key("/path/to/file2.rs"));

    assert_eq!(result["/path/to/file1.rs"][0], ("sym1".to_string(), 10, 20));
    assert_eq!(result["/path/to/file2.rs"][0], ("sym2".to_string(), 30, 40));
}

#[test]
fn test_build_symbol_lookup_table_without_file_prefix() {
    use crate::commands::scan::SymbolInfo;

    // Test that file URIs without "file://" prefix are handled correctly
    let symbols = vec![SymbolInfo {
        id: "sym1".to_string(),
        file_uri: "/path/to/file.rs".to_string(),
        start_line: 10,
        end_line: 20,
        start_col: 0,
        language: Language::Rust,
    }];

    let result = build_symbol_lookup_table(&symbols);

    assert_eq!(result.len(), 1);
    assert!(result.contains_key("/path/to/file.rs"));
}

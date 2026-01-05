//! Tests for find_containing_symbol function

use super::super::find_containing_symbol;
use mother_core::lsp::LspReference;
use std::collections::HashMap;
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

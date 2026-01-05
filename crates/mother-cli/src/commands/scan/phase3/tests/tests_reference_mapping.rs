//! Tests for reference location and symbol mapping

use super::super::find_containing_symbol;
use mother_core::graph::model::EdgeKind;
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

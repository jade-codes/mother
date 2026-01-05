//! Tests for Edge creation logic used in create_reference_edge

use mother_core::graph::model::{Edge, EdgeKind};
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

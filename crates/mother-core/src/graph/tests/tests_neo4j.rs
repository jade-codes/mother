//! Tests for Neo4j client

#![allow(clippy::expect_used)] // expect is acceptable in tests

use crate::graph::model::{Edge, EdgeKind, ScanRun, SymbolKind, SymbolNode};
use crate::graph::neo4j::{Neo4jConfig, Neo4jError};
use chrono::Utc;

// ============================================================================
// Neo4jConfig Tests
// ============================================================================

#[test]
fn test_neo4j_config_new() {
    let config = Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password");
    assert_eq!(config.uri, "bolt://localhost:7687");
    assert_eq!(config.user, "neo4j");
    assert_eq!(config.password, "password");
    assert_eq!(config.database, None);
}

#[test]
fn test_neo4j_config_with_database() {
    let config =
        Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password").with_database("mydb");
    assert_eq!(config.uri, "bolt://localhost:7687");
    assert_eq!(config.user, "neo4j");
    assert_eq!(config.password, "password");
    assert_eq!(config.database, Some("mydb".to_string()));
}

#[test]
fn test_neo4j_config_with_empty_strings() {
    let config = Neo4jConfig::new("", "", "");
    assert_eq!(config.uri, "");
    assert_eq!(config.user, "");
    assert_eq!(config.password, "");
    assert_eq!(config.database, None);
}

#[test]
fn test_neo4j_config_multiple_with_database_calls() {
    let config = Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password")
        .with_database("db1")
        .with_database("db2");
    // Last call should win
    assert_eq!(config.database, Some("db2".to_string()));
}

#[test]
fn test_neo4j_config_clone() {
    let config1 =
        Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password").with_database("testdb");
    let config2 = config1.clone();
    assert_eq!(config1.uri, config2.uri);
    assert_eq!(config1.user, config2.user);
    assert_eq!(config1.password, config2.password);
    assert_eq!(config1.database, config2.database);
}

// ============================================================================
// Neo4jError Tests
// ============================================================================

#[test]
fn test_neo4j_error_connection_display() {
    let err = Neo4jError::Connection("Failed to connect".to_string());
    assert_eq!(format!("{err}"), "Connection error: Failed to connect");
}

#[test]
fn test_neo4j_error_query_display() {
    let err = Neo4jError::Query("Invalid query".to_string());
    assert_eq!(format!("{err}"), "Query error: Invalid query");
}

// ============================================================================
// Model Tests (supporting structures for Neo4j operations)
// ============================================================================

#[test]
fn test_symbol_node_creation() {
    let symbol = SymbolNode {
        id: "test_id".to_string(),
        name: "test_function".to_string(),
        qualified_name: "module::test_function".to_string(),
        kind: SymbolKind::Function,
        visibility: Some("pub".to_string()),
        file_path: "/path/to/file.rs".to_string(),
        start_line: 10,
        end_line: 20,
        signature: Some("fn test_function() -> ()".to_string()),
        doc_comment: Some("Test doc".to_string()),
    };

    assert_eq!(symbol.id, "test_id");
    assert_eq!(symbol.name, "test_function");
    assert_eq!(symbol.kind, SymbolKind::Function);
    assert_eq!(symbol.start_line, 10);
    assert_eq!(symbol.end_line, 20);
}

#[test]
fn test_symbol_node_optional_fields() {
    let symbol = SymbolNode {
        id: "test_id".to_string(),
        name: "test".to_string(),
        qualified_name: "test".to_string(),
        kind: SymbolKind::Variable,
        visibility: None,
        file_path: "/path/to/file.rs".to_string(),
        start_line: 5,
        end_line: 5,
        signature: None,
        doc_comment: None,
    };

    assert_eq!(symbol.visibility, None);
    assert_eq!(symbol.signature, None);
    assert_eq!(symbol.doc_comment, None);
}

#[test]
fn test_edge_creation() {
    let edge = Edge {
        source_id: "source_1".to_string(),
        target_id: "target_1".to_string(),
        kind: EdgeKind::Calls,
        line: Some(42),
        column: Some(10),
    };

    assert_eq!(edge.source_id, "source_1");
    assert_eq!(edge.target_id, "target_1");
    assert_eq!(edge.kind, EdgeKind::Calls);
    assert_eq!(edge.line, Some(42));
    assert_eq!(edge.column, Some(10));
}

#[test]
fn test_edge_without_location() {
    let edge = Edge {
        source_id: "source_1".to_string(),
        target_id: "target_1".to_string(),
        kind: EdgeKind::References,
        line: None,
        column: None,
    };

    assert_eq!(edge.line, None);
    assert_eq!(edge.column, None);
}

#[test]
fn test_edge_kind_variants() {
    let edge_calls = Edge {
        source_id: "s".to_string(),
        target_id: "t".to_string(),
        kind: EdgeKind::Calls,
        line: None,
        column: None,
    };
    assert_eq!(edge_calls.kind, EdgeKind::Calls);

    let edge_refs = Edge {
        source_id: "s".to_string(),
        target_id: "t".to_string(),
        kind: EdgeKind::References,
        line: None,
        column: None,
    };
    assert_eq!(edge_refs.kind, EdgeKind::References);

    let edge_imports = Edge {
        source_id: "s".to_string(),
        target_id: "t".to_string(),
        kind: EdgeKind::Imports,
        line: None,
        column: None,
    };
    assert_eq!(edge_imports.kind, EdgeKind::Imports);
}

#[test]
fn test_scan_run_creation() {
    let now = Utc::now();
    let scan_run = ScanRun {
        id: "scan_123".to_string(),
        repo_path: "/path/to/repo".to_string(),
        commit_sha: Some("abc123".to_string()),
        branch: Some("main".to_string()),
        scanned_at: now,
        version: Some("v1.0.0".to_string()),
    };

    assert_eq!(scan_run.id, "scan_123");
    assert_eq!(scan_run.repo_path, "/path/to/repo");
    assert_eq!(scan_run.commit_sha, Some("abc123".to_string()));
    assert_eq!(scan_run.branch, Some("main".to_string()));
    assert_eq!(scan_run.version, Some("v1.0.0".to_string()));
}

#[test]
fn test_scan_run_optional_fields() {
    let now = Utc::now();
    let scan_run = ScanRun {
        id: "scan_123".to_string(),
        repo_path: "/path/to/repo".to_string(),
        commit_sha: None,
        branch: None,
        scanned_at: now,
        version: None,
    };

    assert_eq!(scan_run.commit_sha, None);
    assert_eq!(scan_run.branch, None);
    assert_eq!(scan_run.version, None);
}

#[test]
fn test_scan_run_with_empty_commit_sha() {
    let now = Utc::now();
    let scan_run = ScanRun {
        id: "scan_123".to_string(),
        repo_path: "/path/to/repo".to_string(),
        commit_sha: Some("".to_string()),
        branch: Some("main".to_string()),
        scanned_at: now,
        version: Some("v1.0.0".to_string()),
    };

    assert_eq!(scan_run.commit_sha, Some("".to_string()));
}

// ============================================================================
// Symbol Batch Operations Tests
// ============================================================================

#[test]
fn test_empty_symbol_batch() {
    let symbols: Vec<SymbolNode> = vec![];
    assert_eq!(symbols.len(), 0);
    // This tests that an empty batch can be created
    // The actual create_symbols_batch would handle this by iterating over nothing
}

#[test]
fn test_single_symbol_batch() {
    let symbol = SymbolNode {
        id: "sym_1".to_string(),
        name: "func1".to_string(),
        qualified_name: "mod::func1".to_string(),
        kind: SymbolKind::Function,
        visibility: Some("pub".to_string()),
        file_path: "/file.rs".to_string(),
        start_line: 1,
        end_line: 10,
        signature: Some("fn func1()".to_string()),
        doc_comment: None,
    };
    let symbols = [symbol];
    assert_eq!(symbols.len(), 1);
}

#[test]
fn test_multiple_symbol_batch() {
    let symbols = [
        SymbolNode {
            id: "sym_1".to_string(),
            name: "func1".to_string(),
            qualified_name: "mod::func1".to_string(),
            kind: SymbolKind::Function,
            visibility: Some("pub".to_string()),
            file_path: "/file.rs".to_string(),
            start_line: 1,
            end_line: 10,
            signature: Some("fn func1()".to_string()),
            doc_comment: None,
        },
        SymbolNode {
            id: "sym_2".to_string(),
            name: "func2".to_string(),
            qualified_name: "mod::func2".to_string(),
            kind: SymbolKind::Function,
            visibility: Some("pub".to_string()),
            file_path: "/file.rs".to_string(),
            start_line: 15,
            end_line: 25,
            signature: Some("fn func2()".to_string()),
            doc_comment: None,
        },
        SymbolNode {
            id: "sym_3".to_string(),
            name: "MyStruct".to_string(),
            qualified_name: "mod::MyStruct".to_string(),
            kind: SymbolKind::Struct,
            visibility: Some("pub".to_string()),
            file_path: "/file.rs".to_string(),
            start_line: 30,
            end_line: 35,
            signature: None,
            doc_comment: Some("A struct".to_string()),
        },
    ];
    assert_eq!(symbols.len(), 3);
    assert_eq!(symbols[0].kind, SymbolKind::Function);
    assert_eq!(symbols[1].kind, SymbolKind::Function);
    assert_eq!(symbols[2].kind, SymbolKind::Struct);
}

// ============================================================================
// Edge Cases and Boundary Tests
// ============================================================================

#[test]
fn test_symbol_with_zero_line_numbers() {
    let symbol = SymbolNode {
        id: "sym_1".to_string(),
        name: "test".to_string(),
        qualified_name: "test".to_string(),
        kind: SymbolKind::Variable,
        visibility: None,
        file_path: "/file.rs".to_string(),
        start_line: 0,
        end_line: 0,
        signature: None,
        doc_comment: None,
    };
    assert_eq!(symbol.start_line, 0);
    assert_eq!(symbol.end_line, 0);
}

#[test]
fn test_symbol_with_large_line_numbers() {
    let symbol = SymbolNode {
        id: "sym_1".to_string(),
        name: "test".to_string(),
        qualified_name: "test".to_string(),
        kind: SymbolKind::Variable,
        visibility: None,
        file_path: "/file.rs".to_string(),
        start_line: u32::MAX,
        end_line: u32::MAX,
        signature: None,
        doc_comment: None,
    };
    assert_eq!(symbol.start_line, u32::MAX);
    assert_eq!(symbol.end_line, u32::MAX);
}

#[test]
fn test_edge_with_zero_location() {
    let edge = Edge {
        source_id: "s".to_string(),
        target_id: "t".to_string(),
        kind: EdgeKind::Calls,
        line: Some(0),
        column: Some(0),
    };
    assert_eq!(edge.line, Some(0));
    assert_eq!(edge.column, Some(0));
}

#[test]
fn test_edge_with_large_location_values() {
    let edge = Edge {
        source_id: "s".to_string(),
        target_id: "t".to_string(),
        kind: EdgeKind::Calls,
        line: Some(u32::MAX),
        column: Some(u32::MAX),
    };
    assert_eq!(edge.line, Some(u32::MAX));
    assert_eq!(edge.column, Some(u32::MAX));
}

#[test]
fn test_symbol_with_long_strings() {
    let long_string = "a".repeat(10000);
    let symbol = SymbolNode {
        id: long_string.clone(),
        name: long_string.clone(),
        qualified_name: long_string.clone(),
        kind: SymbolKind::Function,
        visibility: Some(long_string.clone()),
        file_path: long_string.clone(),
        start_line: 1,
        end_line: 2,
        signature: Some(long_string.clone()),
        doc_comment: Some(long_string.clone()),
    };
    assert_eq!(symbol.id.len(), 10000);
    assert_eq!(symbol.name.len(), 10000);
}

#[test]
fn test_edge_with_empty_ids() {
    let edge = Edge {
        source_id: "".to_string(),
        target_id: "".to_string(),
        kind: EdgeKind::Calls,
        line: None,
        column: None,
    };
    assert_eq!(edge.source_id, "");
    assert_eq!(edge.target_id, "");
}

#[test]
fn test_scan_run_with_empty_strings() {
    let now = Utc::now();
    let scan_run = ScanRun {
        id: "".to_string(),
        repo_path: "".to_string(),
        commit_sha: Some("".to_string()),
        branch: Some("".to_string()),
        scanned_at: now,
        version: Some("".to_string()),
    };
    assert_eq!(scan_run.id, "");
    assert_eq!(scan_run.repo_path, "");
}

// ============================================================================
// Symbol Kind Tests
// ============================================================================

#[test]
fn test_all_symbol_kinds() {
    let kinds = vec![
        SymbolKind::Module,
        SymbolKind::Class,
        SymbolKind::Struct,
        SymbolKind::Enum,
        SymbolKind::Interface,
        SymbolKind::Trait,
        SymbolKind::Function,
        SymbolKind::Method,
        SymbolKind::Variable,
        SymbolKind::Constant,
        SymbolKind::Field,
        SymbolKind::TypeAlias,
        SymbolKind::Import,
    ];
    assert_eq!(kinds.len(), 13);
}

#[test]
fn test_symbol_kind_display_all() {
    assert_eq!(format!("{}", SymbolKind::Module), "module");
    assert_eq!(format!("{}", SymbolKind::Class), "class");
    assert_eq!(format!("{}", SymbolKind::Struct), "struct");
    assert_eq!(format!("{}", SymbolKind::Enum), "enum");
    assert_eq!(format!("{}", SymbolKind::Interface), "interface");
    assert_eq!(format!("{}", SymbolKind::Trait), "trait");
    assert_eq!(format!("{}", SymbolKind::Function), "function");
    assert_eq!(format!("{}", SymbolKind::Method), "method");
    assert_eq!(format!("{}", SymbolKind::Variable), "variable");
    assert_eq!(format!("{}", SymbolKind::Constant), "constant");
    assert_eq!(format!("{}", SymbolKind::Field), "field");
    assert_eq!(format!("{}", SymbolKind::TypeAlias), "type_alias");
    assert_eq!(format!("{}", SymbolKind::Import), "import");
}

// ============================================================================
// Edge Kind Tests
// ============================================================================

#[test]
fn test_all_edge_kinds() {
    let kinds = [
        EdgeKind::Calls,
        EdgeKind::References,
        EdgeKind::Imports,
        EdgeKind::Inherits,
        EdgeKind::Implements,
        EdgeKind::Contains,
        EdgeKind::DefinedIn,
        EdgeKind::ScannedIn,
    ];
    assert_eq!(kinds.len(), 8);
}

#[test]
fn test_edge_kind_display_all() {
    assert_eq!(format!("{}", EdgeKind::Calls), "CALLS");
    assert_eq!(format!("{}", EdgeKind::References), "REFERENCES");
    assert_eq!(format!("{}", EdgeKind::Imports), "IMPORTS");
    assert_eq!(format!("{}", EdgeKind::Inherits), "INHERITS");
    assert_eq!(format!("{}", EdgeKind::Implements), "IMPLEMENTS");
    assert_eq!(format!("{}", EdgeKind::Contains), "CONTAINS");
    assert_eq!(format!("{}", EdgeKind::DefinedIn), "DEFINED_IN");
    assert_eq!(format!("{}", EdgeKind::ScannedIn), "SCANNED_IN");
}

// ============================================================================
// Type Equality Tests
// ============================================================================

#[test]
fn test_symbol_kind_equality() {
    assert_eq!(SymbolKind::Function, SymbolKind::Function);
    assert_ne!(SymbolKind::Function, SymbolKind::Method);
    assert_ne!(SymbolKind::Class, SymbolKind::Struct);
}

#[test]
fn test_edge_kind_equality() {
    assert_eq!(EdgeKind::Calls, EdgeKind::Calls);
    assert_ne!(EdgeKind::Calls, EdgeKind::References);
    assert_ne!(EdgeKind::Inherits, EdgeKind::Implements);
}

// ============================================================================
// Serialization/Deserialization Tests
// ============================================================================

#[test]
fn test_symbol_kind_serde() {
    use serde_json;

    let kind = SymbolKind::Function;
    let json = serde_json::to_string(&kind).expect("Failed to serialize SymbolKind");
    assert_eq!(json, "\"function\"");

    let deserialized: SymbolKind =
        serde_json::from_str(&json).expect("Failed to deserialize SymbolKind");
    assert_eq!(deserialized, SymbolKind::Function);
}

#[test]
fn test_edge_kind_serde() {
    use serde_json;

    let kind = EdgeKind::Calls;
    let json = serde_json::to_string(&kind).expect("Failed to serialize EdgeKind");
    assert_eq!(json, "\"CALLS\"");

    let deserialized: EdgeKind =
        serde_json::from_str(&json).expect("Failed to deserialize EdgeKind");
    assert_eq!(deserialized, EdgeKind::Calls);
}

#[test]
fn test_symbol_node_serde() {
    use serde_json;

    let symbol = SymbolNode {
        id: "test_id".to_string(),
        name: "test_func".to_string(),
        qualified_name: "mod::test_func".to_string(),
        kind: SymbolKind::Function,
        visibility: Some("pub".to_string()),
        file_path: "/file.rs".to_string(),
        start_line: 10,
        end_line: 20,
        signature: Some("fn test_func()".to_string()),
        doc_comment: Some("Test".to_string()),
    };

    let json = serde_json::to_string(&symbol).expect("Failed to serialize SymbolNode");
    let deserialized: SymbolNode =
        serde_json::from_str(&json).expect("Failed to deserialize SymbolNode");

    assert_eq!(deserialized.id, symbol.id);
    assert_eq!(deserialized.name, symbol.name);
    assert_eq!(deserialized.kind, symbol.kind);
}

#[test]
fn test_edge_serde() {
    use serde_json;

    let edge = Edge {
        source_id: "src".to_string(),
        target_id: "tgt".to_string(),
        kind: EdgeKind::Calls,
        line: Some(42),
        column: Some(10),
    };

    let json = serde_json::to_string(&edge).expect("Failed to serialize Edge");
    let deserialized: Edge = serde_json::from_str(&json).expect("Failed to deserialize Edge");

    assert_eq!(deserialized.source_id, edge.source_id);
    assert_eq!(deserialized.target_id, edge.target_id);
    assert_eq!(deserialized.kind, edge.kind);
    assert_eq!(deserialized.line, edge.line);
}

#[test]
fn test_scan_run_serde() {
    use serde_json;

    let now = Utc::now();
    let scan_run = ScanRun {
        id: "scan_1".to_string(),
        repo_path: "/repo".to_string(),
        commit_sha: Some("abc123".to_string()),
        branch: Some("main".to_string()),
        scanned_at: now,
        version: Some("v1.0.0".to_string()),
    };

    let json = serde_json::to_string(&scan_run).expect("Failed to serialize ScanRun");
    let deserialized: ScanRun = serde_json::from_str(&json).expect("Failed to deserialize ScanRun");

    assert_eq!(deserialized.id, scan_run.id);
    assert_eq!(deserialized.repo_path, scan_run.repo_path);
    assert_eq!(deserialized.commit_sha, scan_run.commit_sha);
}

// ============================================================================
// Clone Tests
// ============================================================================

#[test]
fn test_symbol_node_clone() {
    let symbol = SymbolNode {
        id: "test_id".to_string(),
        name: "test".to_string(),
        qualified_name: "mod::test".to_string(),
        kind: SymbolKind::Function,
        visibility: Some("pub".to_string()),
        file_path: "/file.rs".to_string(),
        start_line: 1,
        end_line: 2,
        signature: Some("fn test()".to_string()),
        doc_comment: None,
    };

    let cloned = symbol.clone();
    assert_eq!(symbol.id, cloned.id);
    assert_eq!(symbol.name, cloned.name);
    assert_eq!(symbol.kind, cloned.kind);
}

#[test]
fn test_edge_clone() {
    let edge = Edge {
        source_id: "s".to_string(),
        target_id: "t".to_string(),
        kind: EdgeKind::Calls,
        line: Some(10),
        column: Some(5),
    };

    let cloned = edge.clone();
    assert_eq!(edge.source_id, cloned.source_id);
    assert_eq!(edge.target_id, cloned.target_id);
    assert_eq!(edge.kind, cloned.kind);
}

#[test]
fn test_scan_run_clone() {
    let now = Utc::now();
    let scan_run = ScanRun {
        id: "scan_1".to_string(),
        repo_path: "/repo".to_string(),
        commit_sha: Some("abc".to_string()),
        branch: Some("main".to_string()),
        scanned_at: now,
        version: Some("v1".to_string()),
    };

    let cloned = scan_run.clone();
    assert_eq!(scan_run.id, cloned.id);
    assert_eq!(scan_run.repo_path, cloned.repo_path);
}

// ============================================================================
// Debug Format Tests
// ============================================================================

#[test]
fn test_symbol_node_debug() {
    let symbol = SymbolNode {
        id: "test_id".to_string(),
        name: "test".to_string(),
        qualified_name: "test".to_string(),
        kind: SymbolKind::Function,
        visibility: None,
        file_path: "/file.rs".to_string(),
        start_line: 1,
        end_line: 2,
        signature: None,
        doc_comment: None,
    };

    let debug_str = format!("{symbol:?}");
    assert!(debug_str.contains("SymbolNode"));
    assert!(debug_str.contains("test_id"));
}

#[test]
fn test_edge_debug() {
    let edge = Edge {
        source_id: "s".to_string(),
        target_id: "t".to_string(),
        kind: EdgeKind::Calls,
        line: None,
        column: None,
    };

    let debug_str = format!("{edge:?}");
    assert!(debug_str.contains("Edge"));
    assert!(debug_str.contains("Calls"));
}

#[test]
fn test_scan_run_debug() {
    let now = Utc::now();
    let scan_run = ScanRun {
        id: "scan_1".to_string(),
        repo_path: "/repo".to_string(),
        commit_sha: None,
        branch: None,
        scanned_at: now,
        version: None,
    };

    let debug_str = format!("{scan_run:?}");
    assert!(debug_str.contains("ScanRun"));
    assert!(debug_str.contains("scan_1"));
}

#[test]
fn test_neo4j_config_debug() {
    let config = Neo4jConfig::new("bolt://localhost:7687", "user", "pass");
    let debug_str = format!("{config:?}");
    assert!(debug_str.contains("Neo4jConfig"));
    assert!(debug_str.contains("bolt://localhost:7687"));
}

#[test]
fn test_neo4j_error_debug() {
    let err = Neo4jError::Connection("test error".to_string());
    let debug_str = format!("{err:?}");
    assert!(debug_str.contains("Connection"));
    assert!(debug_str.contains("test error"));
}

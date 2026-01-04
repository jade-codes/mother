//! Tests for Neo4jClient

use chrono::Utc;
use serial_test::serial;

use crate::graph::model::{Edge, EdgeKind, ScanRun, SymbolKind, SymbolNode};
use crate::graph::neo4j::{Neo4jClient, Neo4jConfig};

/// Helper to create a test Neo4j client connected to the test database
async fn create_test_client() -> Neo4jClient {
    let config = Neo4jConfig::new("bolt://localhost:7687", "neo4j", "mother_dev_password");

    Neo4jClient::connect(&config).await.unwrap()
}

/// Helper to clean up test data after each test
async fn cleanup_test_data(client: &Neo4jClient) {
    use neo4rs::Query;

    // Clean up all test nodes and relationships
    let queries = [
        "MATCH (n:Symbol) DETACH DELETE n",
        "MATCH (n:File) DETACH DELETE n",
        "MATCH (n:ScanRun) DETACH DELETE n",
        "MATCH (n:Commit) DETACH DELETE n",
    ];

    for query_str in queries {
        let _ = client.graph().run(Query::new(query_str.to_string())).await;
    }
}

#[tokio::test]
#[serial]
async fn test_connect_success() {
    let config = Neo4jConfig::new("bolt://localhost:7687", "neo4j", "mother_dev_password");

    let result = Neo4jClient::connect(&config).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[serial]
async fn test_connect_with_database() {
    let config = Neo4jConfig::new("bolt://localhost:7687", "neo4j", "mother_dev_password")
        .with_database("neo4j");

    let result = Neo4jClient::connect(&config).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[serial]
async fn test_connect_invalid_credentials() {
    let config = Neo4jConfig::new("bolt://localhost:7687", "neo4j", "wrong_password");

    let result = Neo4jClient::connect(&config).await;
    assert!(result.is_err());
}

#[tokio::test]
#[serial]
async fn test_connect_invalid_uri() {
    let config = Neo4jConfig::new("bolt://invalid-host:7687", "neo4j", "mother_dev_password");

    let result = Neo4jClient::connect(&config).await;
    assert!(result.is_err());
}

#[tokio::test]
#[serial]
async fn test_graph_accessor() {
    let client = create_test_client().await;

    // The graph() method returns a reference to the Graph
    // We can verify it works by running a simple query
    use neo4rs::Query;
    let query = Query::new("RETURN 1 as num".to_string());
    let result = client.graph().execute(query).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[serial]
async fn test_ensure_indexes_creates_indexes() {
    let client = create_test_client().await;

    // Verify indexes exist by querying the database
    use neo4rs::Query;
    let query = Query::new("SHOW INDEXES".to_string());
    let mut result = client
        .graph()
        .execute(query)
        .await
        .expect("Failed to query indexes");

    let mut index_names = Vec::new();
    while let Ok(Some(row)) = result.next().await {
        if let Ok(name) = row.get::<String>("name") {
            index_names.push(name);
        }
    }

    // Check that expected indexes exist
    assert!(index_names.iter().any(|n| n.contains("commit_sha")));
    assert!(index_names.iter().any(|n| n.contains("file_path_hash")));
    assert!(index_names.iter().any(|n| n.contains("symbol_name")));
    assert!(index_names.iter().any(|n| n.contains("symbol_id")));
    assert!(index_names.iter().any(|n| n.contains("symbol_file_path")));
}

#[tokio::test]
#[serial]
async fn test_create_scan_run_new_commit() {
    let client = create_test_client().await;
    cleanup_test_data(&client).await;

    let scan_run = ScanRun {
        id: "test-scan-1".to_string(),
        repo_path: "/test/repo".to_string(),
        commit_sha: Some("abc123".to_string()),
        branch: Some("main".to_string()),
        scanned_at: Utc::now(),
        version: Some("v1.0.0".to_string()),
    };

    let result = client.create_scan_run(&scan_run).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true); // New commit should return true

    cleanup_test_data(&client).await;
}

#[tokio::test]
#[serial]
async fn test_create_scan_run_existing_commit() {
    let client = create_test_client().await;
    cleanup_test_data(&client).await;

    let scan_run1 = ScanRun {
        id: "test-scan-1".to_string(),
        repo_path: "/test/repo".to_string(),
        commit_sha: Some("abc123".to_string()),
        branch: Some("main".to_string()),
        scanned_at: Utc::now(),
        version: Some("v1.0.0".to_string()),
    };

    // First scan - should create new commit
    let result1 = client.create_scan_run(&scan_run1).await;
    assert!(result1.is_ok());
    assert_eq!(result1.unwrap(), true);

    // Second scan with same commit - should return false
    let scan_run2 = ScanRun {
        id: "test-scan-2".to_string(),
        repo_path: "/test/repo".to_string(),
        commit_sha: Some("abc123".to_string()),
        branch: Some("main".to_string()),
        scanned_at: Utc::now(),
        version: Some("v1.0.0".to_string()),
    };

    let result2 = client.create_scan_run(&scan_run2).await;
    assert!(result2.is_ok());
    assert_eq!(result2.unwrap(), false); // Existing commit should return false

    cleanup_test_data(&client).await;
}

#[tokio::test]
#[serial]
async fn test_create_scan_run_empty_commit_sha() {
    let client = create_test_client().await;
    cleanup_test_data(&client).await;

    let scan_run = ScanRun {
        id: "test-scan-3".to_string(),
        repo_path: "/test/repo".to_string(),
        commit_sha: None,
        branch: Some("main".to_string()),
        scanned_at: Utc::now(),
        version: Some("v1.0.0".to_string()),
    };

    let result = client.create_scan_run(&scan_run).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true); // Empty commit SHA should create new

    cleanup_test_data(&client).await;
}

#[tokio::test]
#[serial]
async fn test_create_file_if_new() {
    let client = create_test_client().await;
    cleanup_test_data(&client).await;

    // First create a scan run and commit
    let scan_run = ScanRun {
        id: "test-scan-file-1".to_string(),
        repo_path: "/test/repo".to_string(),
        commit_sha: Some("file_commit_123".to_string()),
        branch: Some("main".to_string()),
        scanned_at: Utc::now(),
        version: Some("v1.0.0".to_string()),
    };

    client.create_scan_run(&scan_run).await.unwrap();

    // Create new file
    let result = client
        .create_file_if_new("/test/file.rs", "hash123", "rust", "file_commit_123")
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some("hash123".to_string())); // New file returns hash

    cleanup_test_data(&client).await;
}

#[tokio::test]
#[serial]
async fn test_create_file_if_existing() {
    let client = create_test_client().await;
    cleanup_test_data(&client).await;

    // Create scan runs and commits
    let scan_run1 = ScanRun {
        id: "test-scan-file-2".to_string(),
        repo_path: "/test/repo".to_string(),
        commit_sha: Some("file_commit_456".to_string()),
        branch: Some("main".to_string()),
        scanned_at: Utc::now(),
        version: Some("v1.0.0".to_string()),
    };

    client
        .create_scan_run(&scan_run1)
        .await
        .expect("Failed to create scan run 1");

    // Create first file
    client
        .create_file_if_new("/test/file.rs", "hash456", "rust", "file_commit_456")
        .await
        .unwrap();

    // Create another commit
    let scan_run2 = ScanRun {
        id: "test-scan-file-3".to_string(),
        repo_path: "/test/repo".to_string(),
        commit_sha: Some("file_commit_789".to_string()),
        branch: Some("main".to_string()),
        scanned_at: Utc::now(),
        version: Some("v1.0.0".to_string()),
    };

    client
        .create_scan_run(&scan_run2)
        .await
        .expect("Failed to create scan run 2");

    // Try to create same file (same hash) in different commit
    let result = client
        .create_file_if_new("/test/file.rs", "hash456", "rust", "file_commit_789")
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), None); // Existing file returns None

    cleanup_test_data(&client).await;
}

#[tokio::test]
#[serial]
async fn test_create_symbol() {
    let client = create_test_client().await;
    cleanup_test_data(&client).await;

    // Setup: Create commit and file
    let scan_run = ScanRun {
        id: "test-scan-symbol-1".to_string(),
        repo_path: "/test/repo".to_string(),
        commit_sha: Some("symbol_commit_123".to_string()),
        branch: Some("main".to_string()),
        scanned_at: Utc::now(),
        version: Some("v1.0.0".to_string()),
    };

    client.create_scan_run(&scan_run).await.unwrap();
    client
        .create_file_if_new(
            "/test/file.rs",
            "symbol_hash_123",
            "rust",
            "symbol_commit_123",
        )
        .await
        .unwrap();

    // Create symbol
    let symbol = SymbolNode {
        id: "symbol-1".to_string(),
        name: "test_function".to_string(),
        qualified_name: "module::test_function".to_string(),
        kind: SymbolKind::Function,
        visibility: Some("pub".to_string()),
        file_path: "/test/file.rs".to_string(),
        start_line: 10,
        end_line: 20,
        signature: Some("fn test_function()".to_string()),
        doc_comment: Some("Test function".to_string()),
    };

    let result = client.create_symbol(&symbol, "symbol_hash_123").await;
    assert!(result.is_ok());

    cleanup_test_data(&client).await;
}

#[tokio::test]
#[serial]
async fn test_create_symbol_minimal() {
    let client = create_test_client().await;
    cleanup_test_data(&client).await;

    // Setup
    let scan_run = ScanRun {
        id: "test-scan-symbol-2".to_string(),
        repo_path: "/test/repo".to_string(),
        commit_sha: Some("symbol_commit_456".to_string()),
        branch: Some("main".to_string()),
        scanned_at: Utc::now(),
        version: Some("v1.0.0".to_string()),
    };

    client.create_scan_run(&scan_run).await.unwrap();
    client
        .create_file_if_new(
            "/test/file.rs",
            "symbol_hash_456",
            "rust",
            "symbol_commit_456",
        )
        .await
        .unwrap();

    // Create symbol with minimal fields
    let symbol = SymbolNode {
        id: "symbol-2".to_string(),
        name: "test_var".to_string(),
        qualified_name: "test_var".to_string(),
        kind: SymbolKind::Variable,
        visibility: None,
        file_path: "/test/file.rs".to_string(),
        start_line: 5,
        end_line: 5,
        signature: None,
        doc_comment: None,
    };

    let result = client.create_symbol(&symbol, "symbol_hash_456").await;
    assert!(result.is_ok());

    cleanup_test_data(&client).await;
}

#[tokio::test]
#[serial]
async fn test_create_symbols_batch_empty() {
    let client = create_test_client().await;

    // Empty batch should succeed without error
    let symbols: Vec<SymbolNode> = vec![];
    let result = client.create_symbols_batch(&symbols, "any_hash").await;
    assert!(result.is_ok());
}

#[tokio::test]
#[serial]
async fn test_create_symbols_batch_single() {
    let client = create_test_client().await;
    cleanup_test_data(&client).await;

    // Setup
    let scan_run = ScanRun {
        id: "test-scan-batch-1".to_string(),
        repo_path: "/test/repo".to_string(),
        commit_sha: Some("batch_commit_123".to_string()),
        branch: Some("main".to_string()),
        scanned_at: Utc::now(),
        version: Some("v1.0.0".to_string()),
    };

    client.create_scan_run(&scan_run).await.unwrap();
    client
        .create_file_if_new(
            "/test/file.rs",
            "batch_hash_123",
            "rust",
            "batch_commit_123",
        )
        .await
        .unwrap();

    // Create single symbol via batch
    let symbols = vec![SymbolNode {
        id: "batch-symbol-1".to_string(),
        name: "function1".to_string(),
        qualified_name: "module::function1".to_string(),
        kind: SymbolKind::Function,
        visibility: Some("pub".to_string()),
        file_path: "/test/file.rs".to_string(),
        start_line: 10,
        end_line: 20,
        signature: Some("fn function1()".to_string()),
        doc_comment: None,
    }];

    let result = client
        .create_symbols_batch(&symbols, "batch_hash_123")
        .await;
    assert!(result.is_ok());

    cleanup_test_data(&client).await;
}

#[tokio::test]
#[serial]
async fn test_create_symbols_batch_multiple() {
    let client = create_test_client().await;
    cleanup_test_data(&client).await;

    // Setup
    let scan_run = ScanRun {
        id: "test-scan-batch-2".to_string(),
        repo_path: "/test/repo".to_string(),
        commit_sha: Some("batch_commit_456".to_string()),
        branch: Some("main".to_string()),
        scanned_at: Utc::now(),
        version: Some("v1.0.0".to_string()),
    };

    client.create_scan_run(&scan_run).await.unwrap();
    client
        .create_file_if_new(
            "/test/file.rs",
            "batch_hash_456",
            "rust",
            "batch_commit_456",
        )
        .await
        .unwrap();

    // Create multiple symbols
    let symbols = vec![
        SymbolNode {
            id: "batch-symbol-2".to_string(),
            name: "Class1".to_string(),
            qualified_name: "Class1".to_string(),
            kind: SymbolKind::Class,
            visibility: Some("pub".to_string()),
            file_path: "/test/file.rs".to_string(),
            start_line: 1,
            end_line: 10,
            signature: None,
            doc_comment: Some("Class documentation".to_string()),
        },
        SymbolNode {
            id: "batch-symbol-3".to_string(),
            name: "method1".to_string(),
            qualified_name: "Class1::method1".to_string(),
            kind: SymbolKind::Method,
            visibility: Some("pub".to_string()),
            file_path: "/test/file.rs".to_string(),
            start_line: 5,
            end_line: 8,
            signature: Some("fn method1(&self)".to_string()),
            doc_comment: None,
        },
        SymbolNode {
            id: "batch-symbol-4".to_string(),
            name: "CONSTANT".to_string(),
            qualified_name: "CONSTANT".to_string(),
            kind: SymbolKind::Constant,
            visibility: Some("pub".to_string()),
            file_path: "/test/file.rs".to_string(),
            start_line: 15,
            end_line: 15,
            signature: None,
            doc_comment: None,
        },
    ];

    let result = client
        .create_symbols_batch(&symbols, "batch_hash_456")
        .await;
    assert!(result.is_ok());

    cleanup_test_data(&client).await;
}

#[tokio::test]
#[serial]
async fn test_create_edge_calls() {
    let client = create_test_client().await;
    cleanup_test_data(&client).await;

    // Setup: Create commit, file, and symbols
    let scan_run = ScanRun {
        id: "test-scan-edge-1".to_string(),
        repo_path: "/test/repo".to_string(),
        commit_sha: Some("edge_commit_123".to_string()),
        branch: Some("main".to_string()),
        scanned_at: Utc::now(),
        version: Some("v1.0.0".to_string()),
    };

    client.create_scan_run(&scan_run).await.unwrap();
    client
        .create_file_if_new("/test/file.rs", "edge_hash_123", "rust", "edge_commit_123")
        .await
        .unwrap();

    let symbols = vec![
        SymbolNode {
            id: "edge-symbol-1".to_string(),
            name: "caller".to_string(),
            qualified_name: "caller".to_string(),
            kind: SymbolKind::Function,
            visibility: Some("pub".to_string()),
            file_path: "/test/file.rs".to_string(),
            start_line: 1,
            end_line: 5,
            signature: None,
            doc_comment: None,
        },
        SymbolNode {
            id: "edge-symbol-2".to_string(),
            name: "callee".to_string(),
            qualified_name: "callee".to_string(),
            kind: SymbolKind::Function,
            visibility: Some("pub".to_string()),
            file_path: "/test/file.rs".to_string(),
            start_line: 10,
            end_line: 15,
            signature: None,
            doc_comment: None,
        },
    ];

    client
        .create_symbols_batch(&symbols, "edge_hash_123")
        .await
        .unwrap();

    // Create edge
    let edge = Edge {
        source_id: "edge-symbol-1".to_string(),
        target_id: "edge-symbol-2".to_string(),
        kind: EdgeKind::Calls,
        line: Some(3),
        column: Some(10),
    };

    let result = client.create_edge(&edge).await;
    assert!(result.is_ok());

    cleanup_test_data(&client).await;
}

#[tokio::test]
#[serial]
async fn test_create_edge_references() {
    let client = create_test_client().await;
    cleanup_test_data(&client).await;

    // Setup
    let scan_run = ScanRun {
        id: "test-scan-edge-2".to_string(),
        repo_path: "/test/repo".to_string(),
        commit_sha: Some("edge_commit_456".to_string()),
        branch: Some("main".to_string()),
        scanned_at: Utc::now(),
        version: Some("v1.0.0".to_string()),
    };

    client.create_scan_run(&scan_run).await.unwrap();
    client
        .create_file_if_new("/test/file.rs", "edge_hash_456", "rust", "edge_commit_456")
        .await
        .unwrap();

    let symbols = vec![
        SymbolNode {
            id: "edge-symbol-3".to_string(),
            name: "variable".to_string(),
            qualified_name: "variable".to_string(),
            kind: SymbolKind::Variable,
            visibility: None,
            file_path: "/test/file.rs".to_string(),
            start_line: 1,
            end_line: 1,
            signature: None,
            doc_comment: None,
        },
        SymbolNode {
            id: "edge-symbol-4".to_string(),
            name: "function".to_string(),
            qualified_name: "function".to_string(),
            kind: SymbolKind::Function,
            visibility: Some("pub".to_string()),
            file_path: "/test/file.rs".to_string(),
            start_line: 5,
            end_line: 10,
            signature: None,
            doc_comment: None,
        },
    ];

    client
        .create_symbols_batch(&symbols, "edge_hash_456")
        .await
        .unwrap();

    // Create reference edge
    let edge = Edge {
        source_id: "edge-symbol-4".to_string(),
        target_id: "edge-symbol-3".to_string(),
        kind: EdgeKind::References,
        line: Some(7),
        column: Some(5),
    };

    let result = client.create_edge(&edge).await;
    assert!(result.is_ok());

    cleanup_test_data(&client).await;
}

#[tokio::test]
#[serial]
async fn test_create_edge_no_location() {
    let client = create_test_client().await;
    cleanup_test_data(&client).await;

    // Setup
    let scan_run = ScanRun {
        id: "test-scan-edge-3".to_string(),
        repo_path: "/test/repo".to_string(),
        commit_sha: Some("edge_commit_789".to_string()),
        branch: Some("main".to_string()),
        scanned_at: Utc::now(),
        version: Some("v1.0.0".to_string()),
    };

    client.create_scan_run(&scan_run).await.unwrap();
    client
        .create_file_if_new("/test/file.rs", "edge_hash_789", "rust", "edge_commit_789")
        .await
        .unwrap();

    let symbols = vec![
        SymbolNode {
            id: "edge-symbol-5".to_string(),
            name: "Parent".to_string(),
            qualified_name: "Parent".to_string(),
            kind: SymbolKind::Class,
            visibility: Some("pub".to_string()),
            file_path: "/test/file.rs".to_string(),
            start_line: 1,
            end_line: 5,
            signature: None,
            doc_comment: None,
        },
        SymbolNode {
            id: "edge-symbol-6".to_string(),
            name: "Child".to_string(),
            qualified_name: "Child".to_string(),
            kind: SymbolKind::Class,
            visibility: Some("pub".to_string()),
            file_path: "/test/file.rs".to_string(),
            start_line: 10,
            end_line: 15,
            signature: None,
            doc_comment: None,
        },
    ];

    client
        .create_symbols_batch(&symbols, "edge_hash_789")
        .await
        .unwrap();

    // Create edge without location info
    let edge = Edge {
        source_id: "edge-symbol-6".to_string(),
        target_id: "edge-symbol-5".to_string(),
        kind: EdgeKind::Inherits,
        line: None,
        column: None,
    };

    let result = client.create_edge(&edge).await;
    assert!(result.is_ok());

    cleanup_test_data(&client).await;
}

#[tokio::test]
#[serial]
async fn test_create_edge_multiple_kinds() {
    let client = create_test_client().await;
    cleanup_test_data(&client).await;

    // Setup
    let scan_run = ScanRun {
        id: "test-scan-edge-4".to_string(),
        repo_path: "/test/repo".to_string(),
        commit_sha: Some("edge_commit_multi".to_string()),
        branch: Some("main".to_string()),
        scanned_at: Utc::now(),
        version: Some("v1.0.0".to_string()),
    };

    client.create_scan_run(&scan_run).await.unwrap();
    client
        .create_file_if_new(
            "/test/file.rs",
            "edge_hash_multi",
            "rust",
            "edge_commit_multi",
        )
        .await
        .unwrap();

    let symbols = vec![
        SymbolNode {
            id: "edge-multi-1".to_string(),
            name: "module1".to_string(),
            qualified_name: "module1".to_string(),
            kind: SymbolKind::Module,
            visibility: Some("pub".to_string()),
            file_path: "/test/file.rs".to_string(),
            start_line: 1,
            end_line: 1,
            signature: None,
            doc_comment: None,
        },
        SymbolNode {
            id: "edge-multi-2".to_string(),
            name: "module2".to_string(),
            qualified_name: "module2".to_string(),
            kind: SymbolKind::Module,
            visibility: Some("pub".to_string()),
            file_path: "/test/file.rs".to_string(),
            start_line: 5,
            end_line: 5,
            signature: None,
            doc_comment: None,
        },
        SymbolNode {
            id: "edge-multi-3".to_string(),
            name: "Trait1".to_string(),
            qualified_name: "Trait1".to_string(),
            kind: SymbolKind::Trait,
            visibility: Some("pub".to_string()),
            file_path: "/test/file.rs".to_string(),
            start_line: 10,
            end_line: 15,
            signature: None,
            doc_comment: None,
        },
        SymbolNode {
            id: "edge-multi-4".to_string(),
            name: "Struct1".to_string(),
            qualified_name: "Struct1".to_string(),
            kind: SymbolKind::Struct,
            visibility: Some("pub".to_string()),
            file_path: "/test/file.rs".to_string(),
            start_line: 20,
            end_line: 25,
            signature: None,
            doc_comment: None,
        },
    ];

    client
        .create_symbols_batch(&symbols, "edge_hash_multi")
        .await
        .unwrap();

    // Create multiple edge kinds
    let edges = vec![
        Edge {
            source_id: "edge-multi-2".to_string(),
            target_id: "edge-multi-1".to_string(),
            kind: EdgeKind::Imports,
            line: Some(5),
            column: None,
        },
        Edge {
            source_id: "edge-multi-4".to_string(),
            target_id: "edge-multi-3".to_string(),
            kind: EdgeKind::Implements,
            line: Some(20),
            column: None,
        },
    ];

    for edge in edges {
        let result = client.create_edge(&edge).await;
        assert!(result.is_ok());
    }

    cleanup_test_data(&client).await;
}

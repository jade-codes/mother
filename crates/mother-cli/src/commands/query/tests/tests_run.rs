//! Tests for the query run function
//!
//! These tests verify the behavior of the `mother::commands::query::run` function
//! and its interaction with Neo4j through the public API.

use crate::commands::query::run;
use crate::types::QueryCommands;
use mother_core::graph::neo4j::{Neo4jClient, Neo4jConfig};

/// Test that the run function properly handles connection errors with invalid credentials
#[tokio::test]
async fn test_run_with_invalid_neo4j_connection() {
    let cmd = QueryCommands::Stats;
    let result = run(cmd, "bolt://invalid-host:7687", "neo4j", "invalid_password").await;

    // Should fail because the host is invalid
    assert!(
        result.is_err(),
        "Expected error with invalid Neo4j connection"
    );
}

/// Test that run handles Symbols command with empty pattern gracefully
/// Note: This requires a real Neo4j instance, so it will fail without one
#[tokio::test]
#[ignore] // Ignore by default as it requires a running Neo4j instance
async fn test_run_symbols_with_empty_pattern() {
    let cmd = QueryCommands::Symbols {
        pattern: String::new(),
    };

    // This test would need a real Neo4j instance
    // When run against a real instance, it should:
    // - Connect successfully
    // - Execute the query
    // - Return all symbols (or handle empty pattern appropriately)
    let result = run(cmd, "bolt://localhost:7687", "neo4j", "password").await;

    // With a real instance, this should succeed
    assert!(result.is_ok());
}

/// Test that run handles File command with empty database gracefully
#[tokio::test]
#[ignore] // Requires Neo4j instance
async fn test_run_file_command() {
    let cmd = QueryCommands::File {
        path: "test.rs".to_string(),
    };

    let result = run(cmd, "bolt://localhost:7687", "neo4j", "password").await;

    // Should handle empty results gracefully
    assert!(result.is_ok());
}

/// Test that run handles RefsTo command
#[tokio::test]
#[ignore] // Requires Neo4j instance
async fn test_run_refs_to_command() {
    let cmd = QueryCommands::RefsTo {
        symbol: "TestSymbol".to_string(),
    };

    let result = run(cmd, "bolt://localhost:7687", "neo4j", "password").await;

    assert!(result.is_ok());
}

/// Test that run handles RefsFrom command
#[tokio::test]
#[ignore] // Requires Neo4j instance
async fn test_run_refs_from_command() {
    let cmd = QueryCommands::RefsFrom {
        symbol: "TestSymbol".to_string(),
    };

    let result = run(cmd, "bolt://localhost:7687", "neo4j", "password").await;

    assert!(result.is_ok());
}

/// Test that run handles Files command without pattern
#[tokio::test]
#[ignore] // Requires Neo4j instance
async fn test_run_files_without_pattern() {
    let cmd = QueryCommands::Files { pattern: None };

    let result = run(cmd, "bolt://localhost:7687", "neo4j", "password").await;

    assert!(result.is_ok());
}

/// Test that run handles Files command with pattern
#[tokio::test]
#[ignore] // Requires Neo4j instance
async fn test_run_files_with_pattern() {
    let cmd = QueryCommands::Files {
        pattern: Some("*.rs".to_string()),
    };

    let result = run(cmd, "bolt://localhost:7687", "neo4j", "password").await;

    assert!(result.is_ok());
}

/// Test that run handles Stats command
#[tokio::test]
#[ignore] // Requires Neo4j instance
async fn test_run_stats_command() {
    let cmd = QueryCommands::Stats;

    let result = run(cmd, "bolt://localhost:7687", "neo4j", "password").await;

    assert!(result.is_ok());
}

/// Test that run handles Raw command with valid query
#[tokio::test]
#[ignore] // Requires Neo4j instance
async fn test_run_raw_command() {
    let cmd = QueryCommands::Raw {
        query: "MATCH (n) RETURN count(n) as total".to_string(),
    };

    let result = run(cmd, "bolt://localhost:7687", "neo4j", "password").await;

    assert!(result.is_ok());
}

/// Test Neo4j config creation
#[test]
fn test_neo4j_config_creation() {
    let config = Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password");

    assert_eq!(config.uri, "bolt://localhost:7687");
    assert_eq!(config.user, "neo4j");
    assert_eq!(config.password, "password");
    assert!(config.database.is_none());
}

/// Test Neo4j config with database
#[test]
fn test_neo4j_config_with_database() {
    let config =
        Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password").with_database("testdb");

    assert_eq!(config.database, Some("testdb".to_string()));
}

/// Test Neo4j connection failure with invalid URI format
#[tokio::test]
async fn test_neo4j_connect_invalid_uri() {
    let config = Neo4jConfig::new("invalid-uri", "neo4j", "password");
    let result = Neo4jClient::connect(&config).await;

    assert!(result.is_err(), "Expected error with invalid URI format");
}

/// Test Neo4j connection with unreachable host
/// This test is intentionally removed as it takes too long to timeout.
/// The connection error handling is already tested by test_run_with_invalid_neo4j_connection.
/// Test different QueryCommands variants to ensure they're properly constructed
#[test]
fn test_query_commands_variants() {
    // Test Symbols variant
    let symbols_cmd = QueryCommands::Symbols {
        pattern: "test".to_string(),
    };
    if let QueryCommands::Symbols { pattern } = symbols_cmd {
        assert_eq!(pattern, "test");
    } else {
        unreachable!("Expected Symbols variant");
    }

    // Test File variant
    let file_cmd = QueryCommands::File {
        path: "test.rs".to_string(),
    };
    if let QueryCommands::File { path } = file_cmd {
        assert_eq!(path, "test.rs");
    } else {
        unreachable!("Expected File variant");
    }

    // Test RefsTo variant
    let refs_to_cmd = QueryCommands::RefsTo {
        symbol: "TestFn".to_string(),
    };
    if let QueryCommands::RefsTo { symbol } = refs_to_cmd {
        assert_eq!(symbol, "TestFn");
    } else {
        unreachable!("Expected RefsTo variant");
    }

    // Test RefsFrom variant
    let refs_from_cmd = QueryCommands::RefsFrom {
        symbol: "TestStruct".to_string(),
    };
    if let QueryCommands::RefsFrom { symbol } = refs_from_cmd {
        assert_eq!(symbol, "TestStruct");
    } else {
        unreachable!("Expected RefsFrom variant");
    }

    // Test Files variant with pattern
    let files_with_pattern = QueryCommands::Files {
        pattern: Some("*.rs".to_string()),
    };
    if let QueryCommands::Files { pattern } = files_with_pattern {
        assert_eq!(pattern, Some("*.rs".to_string()));
    } else {
        unreachable!("Expected Files variant");
    }

    // Test Files variant without pattern
    let files_without_pattern = QueryCommands::Files { pattern: None };
    if let QueryCommands::Files { pattern } = files_without_pattern {
        assert!(pattern.is_none());
    } else {
        unreachable!("Expected Files variant");
    }

    // Test Stats variant
    let stats_cmd = QueryCommands::Stats;
    assert!(matches!(stats_cmd, QueryCommands::Stats));

    // Test Raw variant
    let raw_cmd = QueryCommands::Raw {
        query: "MATCH (n) RETURN n".to_string(),
    };
    if let QueryCommands::Raw { query } = raw_cmd {
        assert_eq!(query, "MATCH (n) RETURN n");
    } else {
        unreachable!("Expected Raw variant");
    }
}

/// Test edge case: empty string pattern in Symbols command
#[test]
fn test_symbols_command_empty_pattern() {
    let cmd = QueryCommands::Symbols {
        pattern: String::new(),
    };
    if let QueryCommands::Symbols { pattern } = cmd {
        assert_eq!(pattern, "");
    } else {
        unreachable!("Expected Symbols variant");
    }
}

/// Test edge case: empty string path in File command
#[test]
fn test_file_command_empty_path() {
    let cmd = QueryCommands::File {
        path: String::new(),
    };
    if let QueryCommands::File { path } = cmd {
        assert_eq!(path, "");
    } else {
        unreachable!("Expected File variant");
    }
}

/// Test edge case: empty query in Raw command
#[test]
fn test_raw_command_empty_query() {
    let cmd = QueryCommands::Raw {
        query: String::new(),
    };
    if let QueryCommands::Raw { query } = cmd {
        assert_eq!(query, "");
    } else {
        unreachable!("Expected Raw variant");
    }
}

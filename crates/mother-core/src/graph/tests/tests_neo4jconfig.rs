//! Tests for Neo4jConfig

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::graph::neo4j::Neo4jConfig;

// Tests for Neo4jConfig::new

#[test]
fn test_new_with_string_slices() {
    let config = Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password");

    assert_eq!(config.uri, "bolt://localhost:7687");
    assert_eq!(config.user, "neo4j");
    assert_eq!(config.password, "password");
    assert_eq!(config.database, None);
}

#[test]
fn test_new_with_strings() {
    let uri = String::from("bolt://localhost:7687");
    let user = String::from("neo4j");
    let password = String::from("password");

    let config = Neo4jConfig::new(uri, user, password);

    assert_eq!(config.uri, "bolt://localhost:7687");
    assert_eq!(config.user, "neo4j");
    assert_eq!(config.password, "password");
    assert_eq!(config.database, None);
}

#[test]
fn test_new_with_mixed_types() {
    let uri = String::from("bolt://localhost:7687");
    let config = Neo4jConfig::new(uri, "neo4j", String::from("password"));

    assert_eq!(config.uri, "bolt://localhost:7687");
    assert_eq!(config.user, "neo4j");
    assert_eq!(config.password, "password");
    assert_eq!(config.database, None);
}

#[test]
fn test_new_with_empty_strings() {
    let config = Neo4jConfig::new("", "", "");

    assert_eq!(config.uri, "");
    assert_eq!(config.user, "");
    assert_eq!(config.password, "");
    assert_eq!(config.database, None);
}

#[test]
fn test_new_with_special_characters() {
    let config = Neo4jConfig::new(
        "bolt://localhost:7687?encrypted=true",
        "user@domain",
        "p@$$w0rd!#",
    );

    assert_eq!(config.uri, "bolt://localhost:7687?encrypted=true");
    assert_eq!(config.user, "user@domain");
    assert_eq!(config.password, "p@$$w0rd!#");
    assert_eq!(config.database, None);
}

#[test]
fn test_new_with_unicode() {
    let config = Neo4jConfig::new("bolt://localhost:7687", "用户", "密码");

    assert_eq!(config.uri, "bolt://localhost:7687");
    assert_eq!(config.user, "用户");
    assert_eq!(config.password, "密码");
    assert_eq!(config.database, None);
}

#[test]
fn test_new_with_long_strings() {
    let long_uri = "bolt://".to_string() + &"a".repeat(1000);
    let long_user = "u".repeat(500);
    let long_password = "p".repeat(500);

    let config = Neo4jConfig::new(long_uri.clone(), long_user.clone(), long_password.clone());

    assert_eq!(config.uri, long_uri);
    assert_eq!(config.user, long_user);
    assert_eq!(config.password, long_password);
    assert_eq!(config.database, None);
}

#[test]
fn test_new_default_database_is_none() {
    let config = Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password");

    assert!(config.database.is_none());
}

#[test]
fn test_new_clone() {
    let config1 = Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password");
    let config2 = config1.clone();

    assert_eq!(config1.uri, config2.uri);
    assert_eq!(config1.user, config2.user);
    assert_eq!(config1.password, config2.password);
    assert_eq!(config1.database, config2.database);
}

#[test]
fn test_new_debug_trait() {
    let config = Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password");
    let debug_output = format!("{:?}", config);

    assert!(debug_output.contains("Neo4jConfig"));
    assert!(debug_output.contains("bolt://localhost:7687"));
    assert!(debug_output.contains("neo4j"));
    assert!(debug_output.contains("password"));
}

// Tests for Neo4jConfig::with_database

#[test]
fn test_with_database_string_slice() {
    let config =
        Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password").with_database("my_database");

    assert_eq!(config.uri, "bolt://localhost:7687");
    assert_eq!(config.user, "neo4j");
    assert_eq!(config.password, "password");
    assert_eq!(config.database, Some("my_database".to_string()));
}

#[test]
fn test_with_database_string() {
    let db_name = String::from("my_database");
    let config =
        Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password").with_database(db_name);

    assert_eq!(config.database, Some("my_database".to_string()));
}

#[test]
fn test_with_database_empty_string() {
    let config = Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password").with_database("");

    assert_eq!(config.database, Some("".to_string()));
}

#[test]
fn test_with_database_special_characters() {
    let config = Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password")
        .with_database("my-database_123");

    assert_eq!(config.database, Some("my-database_123".to_string()));
}

#[test]
fn test_with_database_unicode() {
    let config =
        Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password").with_database("数据库");

    assert_eq!(config.database, Some("数据库".to_string()));
}

#[test]
fn test_with_database_chaining_last_wins() {
    let config = Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password")
        .with_database("first_db")
        .with_database("second_db")
        .with_database("final_db");

    assert_eq!(config.database, Some("final_db".to_string()));
}

#[test]
fn test_with_database_builder_pattern() {
    let base_config = Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password");
    let config_with_db = base_config.with_database("my_database");

    // The with_database method consumes self, so we can't access base_config anymore
    // This test verifies the builder pattern works correctly
    assert_eq!(config_with_db.database, Some("my_database".to_string()));
    assert_eq!(config_with_db.uri, "bolt://localhost:7687");
    assert_eq!(config_with_db.user, "neo4j");
    assert_eq!(config_with_db.password, "password");
}

#[test]
fn test_with_database_preserves_other_fields() {
    let config =
        Neo4jConfig::new("bolt://custom:1234", "admin", "secret").with_database("production");

    assert_eq!(config.uri, "bolt://custom:1234");
    assert_eq!(config.user, "admin");
    assert_eq!(config.password, "secret");
    assert_eq!(config.database, Some("production".to_string()));
}

#[test]
fn test_with_database_clone_after_setting() {
    let config1 =
        Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password").with_database("my_database");
    let config2 = config1.clone();

    assert_eq!(config1.database, config2.database);
    assert_eq!(config2.database, Some("my_database".to_string()));
}

// Integration tests combining both methods

#[test]
fn test_new_then_with_database_integration() {
    let config =
        Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password").with_database("neo4j");

    assert_eq!(config.uri, "bolt://localhost:7687");
    assert_eq!(config.user, "neo4j");
    assert_eq!(config.password, "password");
    assert_eq!(config.database, Some("neo4j".to_string()));
}

#[test]
fn test_multiple_configs_independent() {
    let config1 = Neo4jConfig::new("bolt://host1:7687", "user1", "pass1").with_database("db1");
    let config2 = Neo4jConfig::new("bolt://host2:7687", "user2", "pass2").with_database("db2");

    assert_eq!(config1.uri, "bolt://host1:7687");
    assert_eq!(config1.database, Some("db1".to_string()));
    assert_eq!(config2.uri, "bolt://host2:7687");
    assert_eq!(config2.database, Some("db2".to_string()));
}

#[test]
fn test_config_without_database_is_valid() {
    let config = Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password");

    // Config without database should be valid for use
    assert_eq!(config.uri, "bolt://localhost:7687");
    assert_eq!(config.user, "neo4j");
    assert_eq!(config.password, "password");
    assert!(config.database.is_none());
}

#[test]
fn test_with_database_long_name() {
    let long_db_name = "db_".to_string() + &"x".repeat(500);
    let config = Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password")
        .with_database(long_db_name.clone());

    assert_eq!(config.database, Some(long_db_name));
}

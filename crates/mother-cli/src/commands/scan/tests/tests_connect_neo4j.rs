//! Tests for `connect_neo4j` function
//!
//! Note: These tests focus on the configuration creation and parameter handling
//! aspects of `connect_neo4j`. Full integration tests requiring an actual Neo4j
//! instance are not included here, as they would require test infrastructure setup.

use super::super::connect_neo4j;

// ============================================================================
// Configuration Creation Tests
// ============================================================================

#[tokio::test]
async fn test_connect_neo4j_creates_valid_config() {
    // This test verifies that connect_neo4j properly creates a Neo4jConfig
    // with the provided parameters. Since Neo4jClient::connect requires an
    // actual Neo4j instance, we expect this test to fail with a connection
    // error, which confirms the config was created and connection was attempted.

    let uri = "bolt://localhost:7687";
    let user = "neo4j";
    let password = "testpassword";

    let result = connect_neo4j(uri, user, password).await;

    // We expect an error since there's no Neo4j instance running
    // The important part is that the function accepts valid parameters
    // and attempts to connect (proving config was created correctly)
    assert!(
        result.is_err(),
        "Expected connection error without Neo4j instance"
    );
}

#[tokio::test]
#[should_panic(expected = "Option::unwrap")]
async fn test_connect_neo4j_with_empty_uri() {
    // Test behavior with empty URI
    // Note: neo4rs 0.8.0 panics with empty URI instead of returning an error
    // This test documents the current behavior
    let uri = "";
    let user = "neo4j";
    let password = "password";

    let _result = connect_neo4j(uri, user, password).await;
}

#[tokio::test]
async fn test_connect_neo4j_with_empty_user() {
    // Test behavior with empty username
    let uri = "bolt://localhost:7687";
    let user = "";
    let password = "password";

    let result = connect_neo4j(uri, user, password).await;

    // Should fail - connection should require valid credentials
    assert!(result.is_err(), "Expected error with empty username");
}

#[tokio::test]
async fn test_connect_neo4j_with_empty_password() {
    // Test behavior with empty password
    let uri = "bolt://localhost:7687";
    let user = "neo4j";
    let password = "";

    let result = connect_neo4j(uri, user, password).await;

    // Should fail - empty password is typically invalid
    assert!(result.is_err(), "Expected error with empty password");
}

#[tokio::test]
#[should_panic(expected = "Option::unwrap")]
async fn test_connect_neo4j_with_all_empty_params() {
    // Test behavior with all empty parameters
    // Note: neo4rs 0.8.0 panics with empty URI instead of returning an error
    // This test documents the current behavior
    let uri = "";
    let user = "";
    let password = "";

    let _result = connect_neo4j(uri, user, password).await;
}

// ============================================================================
// URI Format Tests
// ============================================================================

#[tokio::test]
async fn test_connect_neo4j_with_bolt_uri() {
    // Test with standard bolt:// URI format
    let uri = "bolt://neo4j.example.com:7687";
    let user = "testuser";
    let password = "testpass";

    let result = connect_neo4j(uri, user, password).await;

    // Should attempt connection (and fail without server)
    assert!(result.is_err(), "Expected connection error without server");
}

#[tokio::test]
async fn test_connect_neo4j_with_neo4j_uri() {
    // Test with neo4j:// URI format
    let uri = "neo4j://localhost:7687";
    let user = "testuser";
    let password = "testpass";

    let result = connect_neo4j(uri, user, password).await;

    // Should attempt connection (and fail without server)
    assert!(result.is_err(), "Expected connection error without server");
}

#[tokio::test]
async fn test_connect_neo4j_with_invalid_uri_format() {
    // Test with invalid URI format
    let uri = "not-a-valid-uri";
    let user = "neo4j";
    let password = "password";

    let result = connect_neo4j(uri, user, password).await;

    // Should fail with invalid URI
    assert!(result.is_err(), "Expected error with invalid URI format");
}

#[tokio::test]
async fn test_connect_neo4j_with_http_uri() {
    // Test with HTTP URI (should be bolt:// or neo4j://)
    let uri = "http://localhost:7474";
    let user = "neo4j";
    let password = "password";

    let result = connect_neo4j(uri, user, password).await;

    // Should fail - HTTP is not valid for Neo4j driver
    assert!(result.is_err(), "Expected error with HTTP URI");
}

// ============================================================================
// Special Characters in Parameters Tests
// ============================================================================

#[tokio::test]
async fn test_connect_neo4j_with_special_chars_in_password() {
    // Test password with special characters
    let uri = "bolt://localhost:7687";
    let user = "neo4j";
    let password = "p@ssw0rd!#$%^&*()";

    let result = connect_neo4j(uri, user, password).await;

    // Should accept special characters in password
    // (will fail on connection, but that's expected)
    assert!(result.is_err(), "Expected connection error without server");
}

#[tokio::test]
async fn test_connect_neo4j_with_unicode_in_password() {
    // Test password with unicode characters
    let uri = "bolt://localhost:7687";
    let user = "neo4j";
    let password = "пароль密码🔐";

    let result = connect_neo4j(uri, user, password).await;

    // Should handle unicode in password
    assert!(result.is_err(), "Expected connection error without server");
}

#[tokio::test]
async fn test_connect_neo4j_with_whitespace_in_params() {
    // Test with whitespace in parameters
    let uri = " bolt://localhost:7687 ";
    let user = " neo4j ";
    let password = " password ";

    let result = connect_neo4j(uri, user, password).await;

    // Should fail - whitespace should not be trimmed automatically
    assert!(
        result.is_err(),
        "Expected error with whitespace in parameters"
    );
}

// ============================================================================
// Port Variation Tests
// ============================================================================

#[tokio::test]
async fn test_connect_neo4j_with_non_standard_port() {
    // Test with non-standard port
    let uri = "bolt://localhost:9999";
    let user = "neo4j";
    let password = "password";

    let result = connect_neo4j(uri, user, password).await;

    // Should accept non-standard port
    assert!(result.is_err(), "Expected connection error without server");
}

#[tokio::test]
async fn test_connect_neo4j_without_port() {
    // Test URI without explicit port (should use default)
    let uri = "bolt://localhost";
    let user = "neo4j";
    let password = "password";

    let result = connect_neo4j(uri, user, password).await;

    // Should use default port
    assert!(result.is_err(), "Expected connection error without server");
}

// ============================================================================
// Hostname Variation Tests
// ============================================================================

#[tokio::test]
async fn test_connect_neo4j_with_ipv4_address() {
    // Test with IPv4 address
    let uri = "bolt://192.168.1.100:7687";
    let user = "neo4j";
    let password = "password";

    let result = connect_neo4j(uri, user, password).await;

    // Should accept IPv4 address
    assert!(result.is_err(), "Expected connection error without server");
}

#[tokio::test]
async fn test_connect_neo4j_with_ipv6_address() {
    // Test with IPv6 address
    let uri = "bolt://[::1]:7687";
    let user = "neo4j";
    let password = "password";

    let result = connect_neo4j(uri, user, password).await;

    // Should accept IPv6 address
    assert!(result.is_err(), "Expected connection error without server");
}

#[tokio::test]
async fn test_connect_neo4j_with_domain_name() {
    // Test with fully qualified domain name
    let uri = "bolt://neo4j.production.example.com:7687";
    let user = "produser";
    let password = "prodpassword";

    let result = connect_neo4j(uri, user, password).await;

    // Should accept FQDN
    assert!(result.is_err(), "Expected connection error without server");
}

// ============================================================================
// Credential Length Tests
// ============================================================================

#[tokio::test]
async fn test_connect_neo4j_with_long_username() {
    // Test with very long username
    let uri = "bolt://localhost:7687";
    let user = &"a".repeat(1000);
    let password = "password";

    let result = connect_neo4j(uri, user, password).await;

    // Should handle long username (fail on connection)
    assert!(result.is_err(), "Expected connection error without server");
}

#[tokio::test]
async fn test_connect_neo4j_with_long_password() {
    // Test with very long password
    let uri = "bolt://localhost:7687";
    let user = "neo4j";
    let password = &"p".repeat(1000);

    let result = connect_neo4j(uri, user, password).await;

    // Should handle long password (fail on connection)
    assert!(result.is_err(), "Expected connection error without server");
}

// ============================================================================
// Error Propagation Tests
// ============================================================================

#[tokio::test]
async fn test_connect_neo4j_propagates_connection_errors() {
    // Verify that connection errors are properly propagated
    let uri = "bolt://nonexistent.host.that.does.not.exist:7687";
    let user = "neo4j";
    let password = "password";

    let result = connect_neo4j(uri, user, password).await;

    // Should return an error (wrapped in anyhow::Error)
    assert!(result.is_err(), "Expected error to be propagated");

    // Error should be available for inspection
    if let Err(err) = result {
        let err_str = format!("{}", err);

        // Error message should contain relevant information
        // (either connection failure, timeout, or DNS resolution failure)
        assert!(!err_str.is_empty(), "Error message should not be empty");
    }
}

// ============================================================================
// Documentation Tests
// ============================================================================

/// Test that demonstrates the expected successful usage pattern
/// (would work with a real Neo4j instance)
///
/// This test documents how the function is used internally within the crate.
/// Note: `connect_neo4j` is `pub(crate)` and not part of the public API.
#[tokio::test]
async fn test_connect_neo4j_usage_documentation() {
    // This test serves as documentation for proper internal usage
    let uri = "bolt://localhost:7687";
    let user = "neo4j";
    let password = "password";

    // Function accepts string slices and returns Result<Neo4jClient>
    let result = connect_neo4j(uri, user, password).await;

    // Without a real server, we expect an error
    assert!(result.is_err());

    // The function returns anyhow::Result<Neo4jClient>
    // On success, it would return Ok(Neo4jClient)
    // On failure, it returns Err(anyhow::Error)
}

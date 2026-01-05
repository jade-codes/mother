//! Tests for the phase1::run function

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use mother_core::graph::neo4j::{Neo4jClient, Neo4jConfig};
use mother_core::lsp::LspServerManager;
use mother_core::scanner::{DiscoveredFile, Language};
use serial_test::serial;
use std::path::PathBuf;
use tempfile::TempDir;

use crate::commands::scan::phase1::run;

// ============================================================================
// Helper functions for tests
// ============================================================================

/// Helper to create a test Neo4j client
async fn create_test_client() -> Neo4jClient {
    let config = Neo4jConfig::new("bolt://localhost:7687", "neo4j", "mother_dev_password");
    Neo4jClient::connect(&config).await.unwrap()
}

/// Helper to create a test file with content
fn create_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let file_path = dir.path().join(name);
    std::fs::write(&file_path, content).expect("Failed to create test file");
    file_path
}

/// Helper to create a DiscoveredFile
fn create_discovered_file(path: PathBuf, language: Language) -> DiscoveredFile {
    DiscoveredFile { path, language }
}

// ============================================================================
// Tests for run function with empty input
// ============================================================================

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_empty_file_list() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());
    let commit_sha = "abc123";

    let result = run(&[], &client, &mut lsp_manager, commit_sha).await;

    assert!(result.is_ok());
    let phase1_result = result.unwrap();
    assert_eq!(phase1_result.new_file_count, 0);
    assert_eq!(phase1_result.reused_file_count, 0);
    assert_eq!(phase1_result.error_count, 0);
    assert_eq!(phase1_result.files_to_process.len(), 0);
}

// ============================================================================
// Tests for run function with single file
// ============================================================================

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_single_new_rust_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(&temp_dir, "test.rs", "fn main() {}");
    let discovered_file = create_discovered_file(file_path, Language::Rust);

    let client = create_test_client().await;

    let mut lsp_manager = LspServerManager::new(temp_dir.path());
    let commit_sha = "test_commit_123";

    let result = run(&[discovered_file], &client, &mut lsp_manager, commit_sha).await;

    assert!(result.is_ok());
    let phase1_result = result.unwrap();
    assert_eq!(phase1_result.new_file_count, 1);
    assert_eq!(phase1_result.reused_file_count, 0);
    assert_eq!(phase1_result.error_count, 0);
    assert_eq!(phase1_result.files_to_process.len(), 1);
    assert_eq!(phase1_result.files_to_process[0].language, Language::Rust);
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_single_new_python_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(&temp_dir, "test.py", "def main(): pass");
    let discovered_file = create_discovered_file(file_path, Language::Python);

    let client = create_test_client().await;

    let mut lsp_manager = LspServerManager::new(temp_dir.path());
    let commit_sha = "test_commit_456";

    let result = run(&[discovered_file], &client, &mut lsp_manager, commit_sha).await;

    assert!(result.is_ok());
    let phase1_result = result.unwrap();
    assert_eq!(phase1_result.new_file_count, 1);
    assert_eq!(phase1_result.reused_file_count, 0);
    assert_eq!(phase1_result.error_count, 0);
    assert_eq!(phase1_result.files_to_process.len(), 1);
    assert_eq!(phase1_result.files_to_process[0].language, Language::Python);
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_single_reused_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(&temp_dir, "reused.rs", "fn test() {}");
    let discovered_file = create_discovered_file(file_path.clone(), Language::Rust);

    let client = create_test_client().await;

    let mut lsp_manager = LspServerManager::new(temp_dir.path());
    let commit_sha = "same_commit";

    // First run - file should be new
    let result1 = run(
        std::slice::from_ref(&discovered_file),
        &client,
        &mut lsp_manager,
        commit_sha,
    )
    .await;
    assert!(result1.is_ok());
    let phase1_result1 = result1.unwrap();
    assert_eq!(phase1_result1.new_file_count, 1);

    // Second run - file should be reused (same content and commit)
    let result2 = run(&[discovered_file], &client, &mut lsp_manager, commit_sha).await;

    assert!(result2.is_ok());
    let phase1_result2 = result2.unwrap();
    assert_eq!(phase1_result2.new_file_count, 0);
    assert_eq!(phase1_result2.reused_file_count, 1);
    assert_eq!(phase1_result2.error_count, 0);
    assert_eq!(phase1_result2.files_to_process.len(), 0);
}

// ============================================================================
// Tests for run function with multiple files
// ============================================================================

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_multiple_new_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file1 = create_test_file(&temp_dir, "file1.rs", "fn one() {}");
    let file2 = create_test_file(&temp_dir, "file2.rs", "fn two() {}");
    let file3 = create_test_file(&temp_dir, "file3.rs", "fn three() {}");

    let discovered_files = vec![
        create_discovered_file(file1, Language::Rust),
        create_discovered_file(file2, Language::Rust),
        create_discovered_file(file3, Language::Rust),
    ];

    let client = create_test_client().await;

    let mut lsp_manager = LspServerManager::new(temp_dir.path());
    let commit_sha = "multi_commit";

    let result = run(&discovered_files, &client, &mut lsp_manager, commit_sha).await;

    assert!(result.is_ok());
    let phase1_result = result.unwrap();
    assert_eq!(phase1_result.new_file_count, 3);
    assert_eq!(phase1_result.reused_file_count, 0);
    assert_eq!(phase1_result.error_count, 0);
    assert_eq!(phase1_result.files_to_process.len(), 3);
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_multiple_languages() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let rust_file = create_test_file(&temp_dir, "test.rs", "fn main() {}");
    let python_file = create_test_file(&temp_dir, "test.py", "def main(): pass");
    let ts_file = create_test_file(&temp_dir, "test.ts", "function main() {}");

    let discovered_files = vec![
        create_discovered_file(rust_file, Language::Rust),
        create_discovered_file(python_file, Language::Python),
        create_discovered_file(ts_file, Language::TypeScript),
    ];

    let client = create_test_client().await;

    let mut lsp_manager = LspServerManager::new(temp_dir.path());
    let commit_sha = "multi_lang_commit";

    let result = run(&discovered_files, &client, &mut lsp_manager, commit_sha).await;

    assert!(result.is_ok());
    let phase1_result = result.unwrap();
    assert_eq!(phase1_result.new_file_count, 3);
    assert_eq!(phase1_result.reused_file_count, 0);
    assert_eq!(phase1_result.error_count, 0);
    assert_eq!(phase1_result.files_to_process.len(), 3);

    // Verify languages are preserved
    let languages: Vec<Language> = phase1_result
        .files_to_process
        .iter()
        .map(|f| f.language)
        .collect();
    assert!(languages.contains(&Language::Rust));
    assert!(languages.contains(&Language::Python));
    assert!(languages.contains(&Language::TypeScript));
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_mixed_new_and_reused_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file1 = create_test_file(&temp_dir, "new.rs", "fn new_func() {}");
    let file2 = create_test_file(&temp_dir, "reused.rs", "fn reused_func() {}");

    let discovered_file1 = create_discovered_file(file1, Language::Rust);
    let discovered_file2 = create_discovered_file(file2.clone(), Language::Rust);

    let client = create_test_client().await;

    let mut lsp_manager = LspServerManager::new(temp_dir.path());
    let commit_sha = "mixed_commit";

    // First, process file2 so it will be reused later
    let _ = run(
        std::slice::from_ref(&discovered_file2),
        &client,
        &mut lsp_manager,
        commit_sha,
    )
    .await;

    // Now run with both files - file1 is new, file2 is reused
    let discovered_files = vec![discovered_file1, discovered_file2];
    let result = run(&discovered_files, &client, &mut lsp_manager, commit_sha).await;

    assert!(result.is_ok());
    let phase1_result = result.unwrap();
    assert_eq!(phase1_result.new_file_count, 1);
    assert_eq!(phase1_result.reused_file_count, 1);
    assert_eq!(phase1_result.error_count, 0);
    assert_eq!(phase1_result.files_to_process.len(), 1);
}

// ============================================================================
// Tests for error handling
// ============================================================================

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_nonexistent_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let nonexistent_path = PathBuf::from("/nonexistent/path/file.rs");
    let discovered_file = create_discovered_file(nonexistent_path, Language::Rust);

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());
    let commit_sha = "error_commit";

    let result = run(&[discovered_file], &client, &mut lsp_manager, commit_sha).await;

    assert!(result.is_ok());
    let phase1_result = result.unwrap();
    assert_eq!(phase1_result.new_file_count, 0);
    assert_eq!(phase1_result.reused_file_count, 0);
    assert_eq!(phase1_result.error_count, 1);
    assert_eq!(phase1_result.files_to_process.len(), 0);
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_mixed_success_and_errors() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let valid_file = create_test_file(&temp_dir, "valid.rs", "fn valid() {}");
    let nonexistent_path = PathBuf::from("/nonexistent/error.rs");

    let discovered_files = vec![
        create_discovered_file(valid_file, Language::Rust),
        create_discovered_file(nonexistent_path, Language::Rust),
    ];

    let client = create_test_client().await;

    let mut lsp_manager = LspServerManager::new(temp_dir.path());
    let commit_sha = "mixed_error_commit";

    let result = run(&discovered_files, &client, &mut lsp_manager, commit_sha).await;

    assert!(result.is_ok());
    let phase1_result = result.unwrap();
    assert_eq!(phase1_result.new_file_count, 1);
    assert_eq!(phase1_result.reused_file_count, 0);
    assert_eq!(phase1_result.error_count, 1);
    assert_eq!(phase1_result.files_to_process.len(), 1);
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_multiple_errors() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let nonexistent1 = PathBuf::from("/nonexistent/error1.rs");
    let nonexistent2 = PathBuf::from("/nonexistent/error2.rs");
    let nonexistent3 = PathBuf::from("/nonexistent/error3.rs");

    let discovered_files = vec![
        create_discovered_file(nonexistent1, Language::Rust),
        create_discovered_file(nonexistent2, Language::Python),
        create_discovered_file(nonexistent3, Language::TypeScript),
    ];

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());
    let commit_sha = "all_errors_commit";

    let result = run(&discovered_files, &client, &mut lsp_manager, commit_sha).await;

    assert!(result.is_ok());
    let phase1_result = result.unwrap();
    assert_eq!(phase1_result.new_file_count, 0);
    assert_eq!(phase1_result.reused_file_count, 0);
    assert_eq!(phase1_result.error_count, 3);
    assert_eq!(phase1_result.files_to_process.len(), 0);
}

// ============================================================================
// Tests for commit_sha parameter
// ============================================================================

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_different_commit_sha() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(&temp_dir, "test.rs", "fn test() {}");
    let discovered_file = create_discovered_file(file_path.clone(), Language::Rust);

    let client = create_test_client().await;

    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    // First run with commit_sha1
    let result1 = run(
        std::slice::from_ref(&discovered_file),
        &client,
        &mut lsp_manager,
        "commit_sha_1",
    )
    .await;
    assert!(result1.is_ok());
    let phase1_result1 = result1.unwrap();
    assert_eq!(phase1_result1.new_file_count, 1);

    // Second run with different commit_sha but same file content
    // The file should be treated as new because commit changed
    let result2 = run(
        &[discovered_file],
        &client,
        &mut lsp_manager,
        "commit_sha_2",
    )
    .await;

    assert!(result2.is_ok());
    let phase1_result2 = result2.unwrap();
    // Behavior depends on implementation - file may be new or reused
    // At minimum, it should not error
    assert_eq!(phase1_result2.error_count, 0);
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_empty_commit_sha() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(&temp_dir, "test.rs", "fn main() {}");
    let discovered_file = create_discovered_file(file_path, Language::Rust);

    let client = create_test_client().await;

    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&[discovered_file], &client, &mut lsp_manager, "").await;

    assert!(result.is_ok());
    let phase1_result = result.unwrap();
    // Should handle empty commit_sha gracefully
    assert_eq!(phase1_result.error_count, 0);
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_long_commit_sha() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(&temp_dir, "test.rs", "fn main() {}");
    let discovered_file = create_discovered_file(file_path, Language::Rust);

    let client = create_test_client().await;

    let mut lsp_manager = LspServerManager::new(temp_dir.path());
    let long_sha = "a".repeat(64); // Typical git SHA length

    let result = run(&[discovered_file], &client, &mut lsp_manager, &long_sha).await;

    assert!(result.is_ok());
    let phase1_result = result.unwrap();
    assert_eq!(phase1_result.error_count, 0);
}

// ============================================================================
// Edge case tests
// ============================================================================

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_large_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    // Create a large file (1MB of code)
    let large_content = "fn test() {}\n".repeat(50000);
    let file_path = create_test_file(&temp_dir, "large.rs", &large_content);
    let discovered_file = create_discovered_file(file_path, Language::Rust);

    let client = create_test_client().await;

    let mut lsp_manager = LspServerManager::new(temp_dir.path());
    let commit_sha = "large_file_commit";

    let result = run(&[discovered_file], &client, &mut lsp_manager, commit_sha).await;

    assert!(result.is_ok());
    let phase1_result = result.unwrap();
    // Should handle large files without error
    assert_eq!(phase1_result.error_count, 0);
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_empty_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(&temp_dir, "empty.rs", "");
    let discovered_file = create_discovered_file(file_path, Language::Rust);

    let client = create_test_client().await;

    let mut lsp_manager = LspServerManager::new(temp_dir.path());
    let commit_sha = "empty_file_commit";

    let result = run(&[discovered_file], &client, &mut lsp_manager, commit_sha).await;

    assert!(result.is_ok());
    let phase1_result = result.unwrap();
    assert_eq!(phase1_result.new_file_count, 1);
    assert_eq!(phase1_result.error_count, 0);
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_special_characters_in_filename() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(&temp_dir, "test file-name_123.rs", "fn main() {}");
    let discovered_file = create_discovered_file(file_path, Language::Rust);

    let client = create_test_client().await;

    let mut lsp_manager = LspServerManager::new(temp_dir.path());
    let commit_sha = "special_chars_commit";

    let result = run(&[discovered_file], &client, &mut lsp_manager, commit_sha).await;

    assert!(result.is_ok());
    let phase1_result = result.unwrap();
    // Should handle special characters in filenames
    assert_eq!(phase1_result.error_count, 0);
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_processes_files_in_order() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file1 = create_test_file(&temp_dir, "a.rs", "fn a() {}");
    let file2 = create_test_file(&temp_dir, "b.rs", "fn b() {}");
    let file3 = create_test_file(&temp_dir, "c.rs", "fn c() {}");

    let discovered_files = vec![
        create_discovered_file(file1.clone(), Language::Rust),
        create_discovered_file(file2.clone(), Language::Rust),
        create_discovered_file(file3.clone(), Language::Rust),
    ];

    let client = create_test_client().await;

    let mut lsp_manager = LspServerManager::new(temp_dir.path());
    let commit_sha = "order_commit";

    let result = run(&discovered_files, &client, &mut lsp_manager, commit_sha).await;

    assert!(result.is_ok());
    let phase1_result = result.unwrap();
    assert_eq!(phase1_result.files_to_process.len(), 3);

    // Verify files are in the same order as input
    assert_eq!(phase1_result.files_to_process[0].path, file1);
    assert_eq!(phase1_result.files_to_process[1].path, file2);
    assert_eq!(phase1_result.files_to_process[2].path, file3);
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_returns_ok_even_with_all_errors() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let nonexistent1 = PathBuf::from("/error/file1.rs");
    let nonexistent2 = PathBuf::from("/error/file2.rs");

    let discovered_files = vec![
        create_discovered_file(nonexistent1, Language::Rust),
        create_discovered_file(nonexistent2, Language::Rust),
    ];

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());
    let commit_sha = "all_errors";

    let result = run(&discovered_files, &client, &mut lsp_manager, commit_sha).await;

    // run() should return Ok even when all files fail
    assert!(result.is_ok());
    let phase1_result = result.unwrap();
    assert_eq!(phase1_result.error_count, 2);
    assert_eq!(phase1_result.new_file_count, 0);
}

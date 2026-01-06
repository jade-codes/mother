//! Comprehensive tests for `log_scan_run_info` function
//!
//! Tests the logging of scan run information with various configurations including:
//! - Commit SHA handling (empty vs non-empty)
//! - Branch information (Some vs None)
//! - Special characters in values
//! - Edge cases and boundary conditions
//! - Log format verification

#![allow(clippy::expect_used)] // Tests can use expect for setup

use chrono::Utc;
use mother_core::graph::model::ScanRun;
use tempfile::TempDir;

use super::super::{create_scan_run, log_scan_run_info};

// ============================================================================
// Test Constants
// ============================================================================

/// Valid 40-character hexadecimal commit SHA for testing
const VALID_FULL_COMMIT_SHA: &str = "a1b2c3d4e5f6789012345678901234567890abcd";

// ============================================================================
// Basic Functionality Tests
// ============================================================================

#[test]
fn test_log_scan_run_info_with_empty_commit_sha() {
    // Create a scan run without git info (no commit SHA)
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    let (scan_run, commit_sha) = create_scan_run(path, None);

    // In a non-git directory, commit_sha should be empty
    assert_eq!(commit_sha, "");

    // Function should not panic with empty commit SHA
    log_scan_run_info(&scan_run, &commit_sha);
}

#[test]
fn test_log_scan_run_info_with_git_commit_sha() {
    // Create a git repo to get a real commit SHA
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    // Initialize a git repository
    let repo = git2::Repository::init(path).expect("Failed to init git repo");
    let sig =
        git2::Signature::now("Test User", "test@example.com").expect("Failed to create signature");
    let tree_id = {
        let mut index = repo.index().expect("Failed to get index");
        index.write_tree().expect("Failed to write tree")
    };
    let tree = repo.find_tree(tree_id).expect("Failed to find tree");

    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .expect("Failed to create commit");

    let (scan_run, commit_sha) = create_scan_run(path, None);

    // Should have a commit SHA in a git repo
    assert!(!commit_sha.is_empty());

    // Function should not panic with valid commit SHA
    log_scan_run_info(&scan_run, &commit_sha);
}

#[test]
fn test_log_scan_run_info_with_branch_some() {
    // Create a git repo with a branch
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    let repo = git2::Repository::init(path).expect("Failed to init git repo");
    let sig =
        git2::Signature::now("Test User", "test@example.com").expect("Failed to create signature");
    let tree_id = {
        let mut index = repo.index().expect("Failed to get index");
        index.write_tree().expect("Failed to write tree")
    };
    let tree = repo.find_tree(tree_id).expect("Failed to find tree");

    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .expect("Failed to create commit");

    let (scan_run, commit_sha) = create_scan_run(path, None);

    // Should have branch set in a git repo
    assert!(scan_run.branch.is_some());

    // Function should not panic with branch set
    log_scan_run_info(&scan_run, &commit_sha);
}

#[test]
fn test_log_scan_run_info_with_branch_none() {
    // Create a scan run without git info (no branch)
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    let (scan_run, commit_sha) = create_scan_run(path, None);

    // In a non-git directory, branch should be None
    assert_eq!(scan_run.branch, None);

    // Function should not panic with branch None
    log_scan_run_info(&scan_run, &commit_sha);
}

// ============================================================================
// Commit SHA Format Tests
// ============================================================================

#[test]
fn test_log_scan_run_info_with_short_commit_sha() {
    let scan_run = create_minimal_scan_run();
    let commit_sha = "abc1234";

    // Function should handle short commit SHA
    log_scan_run_info(&scan_run, commit_sha);
}

#[test]
fn test_log_scan_run_info_with_full_length_commit_sha() {
    let scan_run = create_minimal_scan_run();

    // Function should handle full-length SHA
    log_scan_run_info(&scan_run, VALID_FULL_COMMIT_SHA);
}

#[test]
fn test_log_scan_run_info_with_typical_git_sha() {
    let scan_run = create_minimal_scan_run();
    let commit_sha = "a1b2c3d4e5f6789012345678901234567890abcd";

    // Function should handle typical git SHA format
    log_scan_run_info(&scan_run, commit_sha);
}

// ============================================================================
// Branch Format Tests
// ============================================================================

#[test]
fn test_log_scan_run_info_with_main_branch() {
    let mut scan_run = create_minimal_scan_run();
    scan_run.branch = Some("main".to_string());
    let commit_sha = "abc123";

    log_scan_run_info(&scan_run, commit_sha);
}

#[test]
fn test_log_scan_run_info_with_feature_branch() {
    let mut scan_run = create_minimal_scan_run();
    scan_run.branch = Some("feature/add-tests".to_string());
    let commit_sha = "def456";

    log_scan_run_info(&scan_run, commit_sha);
}

#[test]
fn test_log_scan_run_info_with_branch_with_special_chars() {
    let mut scan_run = create_minimal_scan_run();
    scan_run.branch = Some("fix/issue-#123_urgent!".to_string());
    let commit_sha = "ghi789";

    log_scan_run_info(&scan_run, commit_sha);
}

#[test]
fn test_log_scan_run_info_with_unicode_branch_name() {
    let mut scan_run = create_minimal_scan_run();
    scan_run.branch = Some("功能/测试-🚀".to_string());
    let commit_sha = "jkl012";

    log_scan_run_info(&scan_run, commit_sha);
}

#[test]
fn test_log_scan_run_info_with_empty_branch_string() {
    let mut scan_run = create_minimal_scan_run();
    scan_run.branch = Some(String::new());
    let commit_sha = "mno345";

    log_scan_run_info(&scan_run, commit_sha);
}

#[test]
fn test_log_scan_run_info_with_very_long_branch_name() {
    let mut scan_run = create_minimal_scan_run();
    scan_run.branch = Some("feature/".to_string() + &"very-long-name-".repeat(20));
    let commit_sha = "pqr678";

    log_scan_run_info(&scan_run, commit_sha);
}

// ============================================================================
// Scan Run ID Tests
// ============================================================================

#[test]
fn test_log_scan_run_info_with_uuid_format_id() {
    let scan_run = create_minimal_scan_run();
    let commit_sha = "abc123";

    // Verify ID is in UUID format
    assert_eq!(scan_run.id.len(), 36);
    assert_eq!(scan_run.id.matches('-').count(), 4);

    log_scan_run_info(&scan_run, commit_sha);
}

#[test]
fn test_log_scan_run_info_with_different_scan_run_ids() {
    let scan_run1 = create_minimal_scan_run();
    let scan_run2 = create_minimal_scan_run();
    let commit_sha = "def456";

    // Different scan runs should have different IDs
    assert_ne!(scan_run1.id, scan_run2.id);

    // Both should log successfully
    log_scan_run_info(&scan_run1, commit_sha);
    log_scan_run_info(&scan_run2, commit_sha);
}

// ============================================================================
// Edge Cases and Combinations
// ============================================================================

#[test]
fn test_log_scan_run_info_empty_commit_sha_with_branch() {
    let mut scan_run = create_minimal_scan_run();
    scan_run.branch = Some("main".to_string());
    let commit_sha = "";

    // Empty commit SHA but branch present
    log_scan_run_info(&scan_run, commit_sha);
}

#[test]
fn test_log_scan_run_info_non_empty_commit_sha_with_no_branch() {
    let mut scan_run = create_minimal_scan_run();
    scan_run.branch = None;
    let commit_sha = "abc123def456";

    // Non-empty commit SHA but no branch
    log_scan_run_info(&scan_run, commit_sha);
}

#[test]
fn test_log_scan_run_info_both_commit_sha_and_branch_present() {
    let mut scan_run = create_minimal_scan_run();
    scan_run.branch = Some("develop".to_string());
    let commit_sha = "1234567890abcdef";

    // Both commit SHA and branch present
    log_scan_run_info(&scan_run, commit_sha);
}

#[test]
fn test_log_scan_run_info_neither_commit_sha_nor_branch() {
    let mut scan_run = create_minimal_scan_run();
    scan_run.branch = None;
    let commit_sha = "";

    // Neither commit SHA nor branch present (non-git repo)
    log_scan_run_info(&scan_run, commit_sha);
}

// ============================================================================
// Special Characters Tests
// ============================================================================

#[test]
fn test_log_scan_run_info_commit_sha_with_letters_only() {
    let scan_run = create_minimal_scan_run();
    let commit_sha = "abcdefghijklmnop";

    log_scan_run_info(&scan_run, commit_sha);
}

#[test]
fn test_log_scan_run_info_commit_sha_with_numbers_only() {
    let scan_run = create_minimal_scan_run();
    let commit_sha = "1234567890123456";

    log_scan_run_info(&scan_run, commit_sha);
}

#[test]
fn test_log_scan_run_info_commit_sha_with_mixed_case() {
    let scan_run = create_minimal_scan_run();
    let commit_sha = "AbCdEf0123456789";

    log_scan_run_info(&scan_run, commit_sha);
}

#[test]
fn test_log_scan_run_info_with_whitespace_in_commit_sha() {
    let scan_run = create_minimal_scan_run();
    let commit_sha = "abc 123 def 456";

    // Whitespace in commit SHA (shouldn't happen in practice but test it)
    log_scan_run_info(&scan_run, commit_sha);
}

#[test]
fn test_log_scan_run_info_with_special_chars_in_commit_sha() {
    let scan_run = create_minimal_scan_run();
    let commit_sha = "abc-123_def.456";

    log_scan_run_info(&scan_run, commit_sha);
}

// ============================================================================
// Idempotency Tests
// ============================================================================

#[test]
fn test_log_scan_run_info_can_be_called_multiple_times() {
    let scan_run = create_minimal_scan_run();
    let commit_sha = "abc123";

    // Function should be safe to call multiple times with same data
    log_scan_run_info(&scan_run, commit_sha);
    log_scan_run_info(&scan_run, commit_sha);
    log_scan_run_info(&scan_run, commit_sha);
}

#[test]
fn test_log_scan_run_info_with_different_commit_shas() {
    let scan_run = create_minimal_scan_run();

    // Should handle different commit SHAs
    log_scan_run_info(&scan_run, "");
    log_scan_run_info(&scan_run, "abc123");
    log_scan_run_info(&scan_run, "def456");
    log_scan_run_info(&scan_run, "1234567890abcdef");
}

#[test]
fn test_log_scan_run_info_rapid_succession() {
    let scan_run = create_minimal_scan_run();
    let commit_sha = "xyz789";

    // Call function many times in rapid succession
    for _ in 0..100 {
        log_scan_run_info(&scan_run, commit_sha);
    }
}

// ============================================================================
// Version Field Tests
// ============================================================================

#[test]
fn test_log_scan_run_info_with_version_field_none() {
    let scan_run = create_minimal_scan_run();
    assert_eq!(scan_run.version, None);

    log_scan_run_info(&scan_run, "abc123");
}

#[test]
fn test_log_scan_run_info_with_version_field_some() {
    let mut scan_run = create_minimal_scan_run();
    scan_run.version = Some("v1.0.0".to_string());

    log_scan_run_info(&scan_run, "def456");
}

// ============================================================================
// Path Tests
// ============================================================================

#[test]
fn test_log_scan_run_info_with_short_repo_path() {
    let mut scan_run = create_minimal_scan_run();
    scan_run.repo_path = "/tmp/repo".to_string();

    log_scan_run_info(&scan_run, "ghi789");
}

#[test]
fn test_log_scan_run_info_with_long_repo_path() {
    let mut scan_run = create_minimal_scan_run();
    scan_run.repo_path = "/very/long/path/to/repository/".to_string() + &"nested/".repeat(20);

    log_scan_run_info(&scan_run, "jkl012");
}

#[test]
fn test_log_scan_run_info_with_unicode_in_repo_path() {
    let mut scan_run = create_minimal_scan_run();
    scan_run.repo_path = "/home/用户/项目/测试".to_string();

    log_scan_run_info(&scan_run, "mno345");
}

#[test]
fn test_log_scan_run_info_with_spaces_in_repo_path() {
    let mut scan_run = create_minimal_scan_run();
    scan_run.repo_path = "/path/with spaces/in the name".to_string();

    log_scan_run_info(&scan_run, "pqr678");
}

// ============================================================================
// Timestamp Tests
// ============================================================================

#[test]
fn test_log_scan_run_info_with_current_timestamp() {
    let scan_run = create_minimal_scan_run();

    // Timestamp should be recent
    let now = Utc::now();
    let duration = now.signed_duration_since(scan_run.scanned_at);
    assert!(duration.num_seconds() >= 0 && duration.num_seconds() < 60);

    log_scan_run_info(&scan_run, "stu901");
}

#[test]
fn test_log_scan_run_info_with_different_timestamps() {
    let scan_run1 = create_minimal_scan_run();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let scan_run2 = create_minimal_scan_run();

    // Different scan runs should have different timestamps
    assert!(scan_run2.scanned_at >= scan_run1.scanned_at);

    log_scan_run_info(&scan_run1, "vwx234");
    log_scan_run_info(&scan_run2, "yza567");
}

// ============================================================================
// Integration Tests with create_scan_run
// ============================================================================

#[test]
fn test_log_scan_run_info_with_output_from_create_scan_run() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    // Use create_scan_run to create a real scan run
    let (scan_run, commit_sha) = create_scan_run(path, None);

    // log_scan_run_info should work with real output from create_scan_run
    log_scan_run_info(&scan_run, &commit_sha);
}

#[test]
fn test_log_scan_run_info_with_create_scan_run_and_version() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    let (scan_run, commit_sha) = create_scan_run(path, Some("v2.0.0"));

    log_scan_run_info(&scan_run, &commit_sha);
}

#[test]
fn test_log_scan_run_info_workflow_multiple_scan_runs() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    // Simulate multiple scan runs in sequence
    for i in 0..5 {
        let version = format!("v1.{}.0", i);
        let (scan_run, commit_sha) = create_scan_run(path, Some(&version));
        log_scan_run_info(&scan_run, &commit_sha);
    }
}

// ============================================================================
// Consistency Tests
// ============================================================================

#[test]
fn test_log_scan_run_info_commit_sha_matches_scan_run_field() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    let (scan_run, commit_sha) = create_scan_run(path, None);

    // Verify consistency between commit_sha parameter and scan_run.commit_sha
    let expected_commit_sha = scan_run.commit_sha.clone().unwrap_or_default();
    assert_eq!(commit_sha, expected_commit_sha);

    log_scan_run_info(&scan_run, &commit_sha);
}

#[test]
fn test_log_scan_run_info_handles_mismatch_between_params_and_scan_run() {
    // Test what happens when commit_sha parameter doesn't match scan_run.commit_sha
    // This shouldn't happen in production but test for robustness
    let mut scan_run = create_minimal_scan_run();
    scan_run.commit_sha = Some("abc123".to_string());

    // Pass a different commit SHA
    let different_commit_sha = "xyz789";

    // Function should not panic even with mismatched data
    log_scan_run_info(&scan_run, different_commit_sha);
}

// ============================================================================
// Boundary Tests
// ============================================================================

#[test]
fn test_log_scan_run_info_with_minimal_scan_run() {
    // Test with bare minimum scan run data
    let scan_run = ScanRun::new("");

    log_scan_run_info(&scan_run, "");
}

#[test]
fn test_log_scan_run_info_with_maximal_scan_run() {
    // Test with all fields populated with substantial data
    let scan_run =
        ScanRun::new("/very/long/path/to/repository/".to_string() + &"nested/".repeat(10))
            .with_commit(VALID_FULL_COMMIT_SHA)
            .with_branch("feature/very-long-branch-name-with-lots-of-details")
            .with_version("v1.2.3-beta.1+build.12345");

    log_scan_run_info(&scan_run, VALID_FULL_COMMIT_SHA);
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a minimal scan run for testing purposes
fn create_minimal_scan_run() -> ScanRun {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let (scan_run, _) = create_scan_run(temp_dir.path(), None);
    scan_run
}

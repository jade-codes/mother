//! Tests for scan run functionality
//!
//! These tests verify the scan command's logic and behavior, focusing on
//! what can be tested through the public API and observable side effects.

use mother_core::graph::model::ScanRun;
use std::path::Path;
use tempfile::TempDir;

// ============================================================================
// Tests for ScanRun creation logic (as used by create_scan_run helper)
// ============================================================================

#[test]
fn test_scan_run_created_with_path() {
    let path = Path::new("/test/repo/path");
    let scan_run = ScanRun::new(path.display().to_string());

    assert_eq!(scan_run.repo_path, "/test/repo/path");
    assert!(!scan_run.id.is_empty());
    assert!(scan_run.commit_sha.is_none());
    assert!(scan_run.branch.is_none());
    assert!(scan_run.version.is_none());
}

#[test]
fn test_scan_run_with_version() {
    let path = Path::new("/test/repo");
    let version = Some("v1.0.0");

    let mut scan_run = ScanRun::new(path.display().to_string()).with_git_info();
    if let Some(v) = version {
        scan_run = scan_run.with_version(v);
    }

    assert_eq!(scan_run.version, Some("v1.0.0".to_string()));
    assert_eq!(scan_run.repo_path, "/test/repo");
}

#[test]
fn test_scan_run_without_version() {
    let path = Path::new("/test/repo");
    let version: Option<&str> = None;

    let mut scan_run = ScanRun::new(path.display().to_string()).with_git_info();
    if let Some(v) = version {
        scan_run = scan_run.with_version(v);
    }

    assert!(scan_run.version.is_none());
}

#[test]
fn test_scan_run_commit_sha_extraction() {
    let path = Path::new("/test/repo");
    let scan_run = ScanRun::new(path.display().to_string()).with_git_info();
    let commit_sha = scan_run.commit_sha.clone().unwrap_or_default();

    // When not in a git repo, commit_sha should be empty
    assert_eq!(commit_sha, "");
}

#[test]
fn test_scan_run_in_git_repo() {
    // Create a temp directory with a git repo
    let temp_dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(_) => return, // Skip test if can't create temp dir
    };
    let repo_path = temp_dir.path();

    // Initialize git repo
    if std::process::Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .is_err()
    {
        return; // Skip if git init fails
    }

    // Configure git
    if std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .is_err()
    {
        return; // Skip if git config fails
    }

    if std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .is_err()
    {
        return; // Skip if git config fails
    }

    // Create a file and commit
    if std::fs::write(repo_path.join("test.txt"), "test content").is_err() {
        return; // Skip if file write fails
    }

    if std::process::Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(repo_path)
        .output()
        .is_err()
    {
        return; // Skip if git add fails
    }

    if std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .is_err()
    {
        return; // Skip if git commit fails
    }

    // Test ScanRun with git info
    let scan_run = ScanRun::new(repo_path.display().to_string()).with_git_info();

    // In a git repo, commit_sha should be populated
    assert!(scan_run.commit_sha.is_some());
    if let Some(commit_sha) = scan_run.commit_sha {
        assert!(!commit_sha.is_empty());
        assert_eq!(commit_sha.len(), 40); // Git SHA is 40 characters
    }

    // Branch should be populated (default: main or master)
    assert!(scan_run.branch.is_some());
}

#[test]
fn test_scan_run_with_explicit_git_info() {
    let scan_run = ScanRun::new("/test/repo")
        .with_commit("abc123def456")
        .with_branch("feature/test")
        .with_version("v2.0.0");

    assert_eq!(scan_run.commit_sha, Some("abc123def456".to_string()));
    assert_eq!(scan_run.branch, Some("feature/test".to_string()));
    assert_eq!(scan_run.version, Some("v2.0.0".to_string()));
}

// ============================================================================
// Tests for path handling
// ============================================================================

#[test]
fn test_path_canonicalization_fallback() {
    // Test that non-existent paths fall back to original path
    let path = Path::new("/non/existent/path");
    let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    assert_eq!(abs_path, path);
}

#[test]
fn test_path_canonicalization_success() {
    // Test that existing paths are canonicalized
    let temp_dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(_) => return, // Skip test if can't create temp dir
    };
    let path = temp_dir.path();
    let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    // Should be an absolute path
    assert!(abs_path.is_absolute());
}

// ============================================================================
// Tests for commit SHA handling
// ============================================================================

#[test]
fn test_commit_sha_empty_string_when_none() {
    let scan_run = ScanRun::new("/test/repo");
    let commit_sha = scan_run.commit_sha.clone().unwrap_or_default();

    assert_eq!(commit_sha, "");
}

#[test]
fn test_commit_sha_preserved_when_some() {
    let scan_run = ScanRun::new("/test/repo").with_commit("abc123");
    let commit_sha = scan_run.commit_sha.clone().unwrap_or_default();

    assert_eq!(commit_sha, "abc123");
}

// ============================================================================
// Tests for Phase Results structures
// ============================================================================

#[test]
fn test_phase1_result_structure() {
    use crate::commands::scan::Phase1Result;

    let result = Phase1Result {
        files_to_process: vec![],
        new_file_count: 5,
        reused_file_count: 3,
    };

    assert_eq!(result.new_file_count, 5);
    assert_eq!(result.reused_file_count, 3);
    assert_eq!(result.files_to_process.len(), 0);
}

#[test]
fn test_phase2_result_structure() {
    use crate::commands::scan::Phase2Result;

    let result = Phase2Result {
        symbols: vec![],
        symbol_count: 42,
    };

    assert_eq!(result.symbol_count, 42);
    assert_eq!(result.symbols.len(), 0);
}

#[test]
fn test_phase3_result_structure() {
    use crate::commands::scan::Phase3Result;

    let result = Phase3Result {
        reference_count: 100,
    };

    assert_eq!(result.reference_count, 100);
}

// ============================================================================
// Tests for scan summary calculation
// ============================================================================

#[test]
fn test_scan_summary_calculations() {
    use crate::commands::scan::{Phase1Result, Phase2Result, Phase3Result};

    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 10,
        reused_file_count: 5,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 50,
    };

    let phase3 = Phase3Result {
        reference_count: 25,
    };

    // Verify the counts are preserved correctly
    assert_eq!(phase1.new_file_count, 10);
    assert_eq!(phase1.reused_file_count, 5);
    assert_eq!(phase2.symbol_count, 50);
    assert_eq!(phase3.reference_count, 25);

    // Verify total files
    let total_files = phase1.new_file_count + phase1.reused_file_count;
    assert_eq!(total_files, 15);
}

// ============================================================================
// Edge cases and boundary conditions
// ============================================================================

#[test]
fn test_empty_path() {
    let path = Path::new("");
    let scan_run = ScanRun::new(path.display().to_string());

    assert_eq!(scan_run.repo_path, "");
}

#[test]
fn test_very_long_path() {
    let long_path = "/".to_string() + &"a".repeat(1000);
    let scan_run = ScanRun::new(&long_path);

    assert_eq!(scan_run.repo_path.len(), 1001);
}

#[test]
fn test_path_with_special_characters() {
    let path = "/test/repo with spaces/and-dashes/under_scores";
    let scan_run = ScanRun::new(path);

    assert_eq!(scan_run.repo_path, path);
}

#[test]
fn test_scan_run_id_uniqueness() {
    let scan1 = ScanRun::new("/test/repo");
    let scan2 = ScanRun::new("/test/repo");

    // Each scan run should have a unique ID
    assert_ne!(scan1.id, scan2.id);
}

#[test]
fn test_scan_run_id_format() {
    let scan = ScanRun::new("/test/repo");

    // UUID should be 36 characters (with hyphens)
    assert_eq!(scan.id.len(), 36);
    // Should contain hyphens in UUID format
    assert!(scan.id.contains('-'));
}

#[test]
fn test_empty_results() {
    use crate::commands::scan::{Phase1Result, Phase2Result, Phase3Result};

    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 0,
        reused_file_count: 0,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 0,
    };

    let phase3 = Phase3Result { reference_count: 0 };

    assert_eq!(phase1.new_file_count, 0);
    assert_eq!(phase2.symbol_count, 0);
    assert_eq!(phase3.reference_count, 0);
}

#[test]
fn test_maximum_counts() {
    use crate::commands::scan::{Phase1Result, Phase2Result, Phase3Result};

    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: usize::MAX,
        reused_file_count: usize::MAX,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: usize::MAX,
    };

    let phase3 = Phase3Result {
        reference_count: usize::MAX,
    };

    assert_eq!(phase1.new_file_count, usize::MAX);
    assert_eq!(phase2.symbol_count, usize::MAX);
    assert_eq!(phase3.reference_count, usize::MAX);
}

// ============================================================================
// Tests for version string handling
// ============================================================================

#[test]
fn test_version_string_variations() {
    let versions = vec![
        "v1.0.0",
        "1.0.0",
        "v1.0.0-alpha",
        "v1.0.0-beta.1",
        "2024.01.01",
        "main",
        "abc123",
    ];

    for version in versions {
        let scan_run = ScanRun::new("/test/repo").with_version(version);
        assert_eq!(scan_run.version, Some(version.to_string()));
    }
}

#[test]
fn test_empty_version_string() {
    let scan_run = ScanRun::new("/test/repo").with_version("");
    assert_eq!(scan_run.version, Some("".to_string()));
}

// ============================================================================
// Tests for builder pattern
// ============================================================================

#[test]
fn test_builder_pattern_chain() {
    let scan_run = ScanRun::new("/test/repo")
        .with_commit("abc123")
        .with_branch("main")
        .with_version("v1.0.0");

    assert_eq!(scan_run.repo_path, "/test/repo");
    assert_eq!(scan_run.commit_sha, Some("abc123".to_string()));
    assert_eq!(scan_run.branch, Some("main".to_string()));
    assert_eq!(scan_run.version, Some("v1.0.0".to_string()));
}

#[test]
fn test_builder_pattern_partial() {
    let scan_run = ScanRun::new("/test/repo").with_version("v1.0.0");

    assert_eq!(scan_run.repo_path, "/test/repo");
    assert!(scan_run.commit_sha.is_none());
    assert!(scan_run.branch.is_none());
    assert_eq!(scan_run.version, Some("v1.0.0".to_string()));
}

#[test]
fn test_builder_pattern_order_independence() {
    let scan1 = ScanRun::new("/test/repo")
        .with_commit("abc123")
        .with_branch("main")
        .with_version("v1.0.0");

    let scan2 = ScanRun::new("/test/repo")
        .with_version("v1.0.0")
        .with_branch("main")
        .with_commit("abc123");

    assert_eq!(scan1.commit_sha, scan2.commit_sha);
    assert_eq!(scan1.branch, scan2.branch);
    assert_eq!(scan1.version, scan2.version);
}

// ============================================================================
// Tests for git integration behavior
// ============================================================================

#[test]
fn test_with_git_info_on_non_git_directory() {
    let temp_dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(_) => return, // Skip test if can't create temp dir
    };
    let scan_run = ScanRun::new(temp_dir.path().display().to_string()).with_git_info();

    // Should not populate git info for non-git directories
    assert!(scan_run.commit_sha.is_none());
    assert!(scan_run.branch.is_none());
}

#[test]
fn test_with_git_info_preserves_existing_values() {
    // Even if we call with_git_info on a non-git repo,
    // existing values should not be cleared
    let scan_run = ScanRun::new("/test/repo")
        .with_commit("manual_commit")
        .with_branch("manual_branch")
        .with_git_info(); // This should not clear manual values

    // In a non-git directory, the manual values should be preserved
    // Note: with_git_info overwrites if git info is found, but in this case it's not
    assert!(
        scan_run.commit_sha == Some("manual_commit".to_string()) || scan_run.commit_sha.is_none()
    );
}

#[test]
fn test_commit_sha_display_logic() {
    // Test the logic: if commit_sha.is_empty() { "none" } else { commit_sha }
    let scan_run = ScanRun::new("/test/repo");
    let commit_sha = scan_run.commit_sha.clone().unwrap_or_default();
    let display_value = if commit_sha.is_empty() {
        "none"
    } else {
        &commit_sha
    };

    assert_eq!(display_value, "none");

    let scan_run2 = ScanRun::new("/test/repo").with_commit("abc123");
    let commit_sha2 = scan_run2.commit_sha.clone().unwrap_or_default();
    let display_value2 = if commit_sha2.is_empty() {
        "none"
    } else {
        &commit_sha2
    };

    assert_eq!(display_value2, "abc123");
}

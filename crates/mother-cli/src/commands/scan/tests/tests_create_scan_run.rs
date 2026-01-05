//! Comprehensive tests for `create_scan_run` function
//!
//! Tests the creation of scan runs with various configurations including:
//! - Version handling (with/without version)
//! - Path handling (absolute, relative, empty, long, special characters)
//! - Git information extraction (commit SHA, branch)
//! - Unique ID generation
//! - Edge cases and boundary conditions

#![allow(clippy::expect_used)] // Tests can use expect for setup

use std::fs;
use std::path::Path;
use tempfile::TempDir;

use super::super::create_scan_run;

// ============================================================================
// Basic Functionality Tests
// ============================================================================

#[test]
fn test_create_scan_run_basic_no_version() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    let (scan_run, commit_sha) = create_scan_run(path, None);

    // Verify basic fields are populated correctly
    assert_eq!(scan_run.repo_path, path.display().to_string());
    assert!(!scan_run.id.is_empty(), "Scan run ID should not be empty");
    assert!(
        scan_run.version.is_none(),
        "Version should be None when not provided"
    );

    // Commit SHA should match between return value and scan_run field
    assert_eq!(
        commit_sha,
        scan_run.commit_sha.clone().unwrap_or_default(),
        "Returned commit_sha should match scan_run.commit_sha"
    );
}

#[test]
fn test_create_scan_run_basic_with_version() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();
    let version = "v1.2.3";

    let (scan_run, _commit_sha) = create_scan_run(path, Some(version));

    assert_eq!(
        scan_run.version,
        Some(version.to_string()),
        "Version should be set correctly"
    );
    assert_eq!(scan_run.repo_path, path.display().to_string());
    assert!(!scan_run.id.is_empty());
}

// ============================================================================
// Version Handling Tests
// ============================================================================

#[test]
fn test_create_scan_run_with_semantic_versions() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    let versions = vec![
        "1.0.0",
        "v1.0.0",
        "2.0.0-alpha",
        "v2.0.0-beta.1",
        "3.0.0-rc.1",
        "v4.0.0+build.123",
        "5.0.0-alpha+build",
    ];

    for version in versions {
        let (scan_run, _) = create_scan_run(path, Some(version));
        assert_eq!(
            scan_run.version,
            Some(version.to_string()),
            "Version '{}' should be preserved exactly",
            version
        );
    }
}

#[test]
fn test_create_scan_run_with_empty_version_string() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    let (scan_run, _) = create_scan_run(path, Some(""));

    // Empty string version should still be set (not None)
    assert_eq!(
        scan_run.version,
        Some(String::new()),
        "Empty version string should be preserved"
    );
}

#[test]
fn test_create_scan_run_with_whitespace_version() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    let versions = vec![" v1.0.0", "v1.0.0 ", " v1.0.0 ", "\tv1.0.0\n"];

    for version in versions {
        let (scan_run, _) = create_scan_run(path, Some(version));
        assert_eq!(
            scan_run.version,
            Some(version.to_string()),
            "Whitespace in version '{}' should be preserved",
            version
        );
    }
}

#[test]
fn test_create_scan_run_with_long_version() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();
    let long_version = "v".to_string() + &"1.0.0-".repeat(50);

    let (scan_run, _) = create_scan_run(path, Some(&long_version));

    assert_eq!(scan_run.version, Some(long_version));
}

#[test]
fn test_create_scan_run_with_special_characters_in_version() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    let special_versions = vec![
        "v1.0.0-alpha+build.123",
        "v2.0.0-beta.1+20130313144700",
        "v3.0.0-rc.1+build-metadata",
        "latest",
        "main",
        "feature/branch-name",
    ];

    for version in special_versions {
        let (scan_run, _) = create_scan_run(path, Some(version));
        assert_eq!(scan_run.version, Some(version.to_string()));
    }
}

#[test]
fn test_create_scan_run_with_unicode_version() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();
    let unicode_version = "v1.0.0-发布版";

    let (scan_run, _) = create_scan_run(path, Some(unicode_version));

    assert_eq!(scan_run.version, Some(unicode_version.to_string()));
}

// ============================================================================
// Unique ID Generation Tests
// ============================================================================

#[test]
fn test_create_scan_run_generates_unique_ids() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    let (scan_run1, _) = create_scan_run(path, None);
    let (scan_run2, _) = create_scan_run(path, None);

    assert_ne!(
        scan_run1.id, scan_run2.id,
        "Each scan run should have a unique ID"
    );
}

#[test]
fn test_create_scan_run_generates_many_unique_ids() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();
    let mut ids = std::collections::HashSet::new();

    // Generate 100 scan runs and ensure all IDs are unique
    for _ in 0..100 {
        let (scan_run, _) = create_scan_run(path, None);
        ids.insert(scan_run.id.clone());
    }

    assert_eq!(ids.len(), 100, "All 100 IDs should be unique");
}

#[test]
fn test_create_scan_run_id_format() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    let (scan_run, _) = create_scan_run(path, None);

    // UUIDs should be 36 characters (with hyphens)
    assert_eq!(
        scan_run.id.len(),
        36,
        "Scan run ID should be a valid UUID (36 chars)"
    );

    // Should contain hyphens in UUID format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
    assert_eq!(
        scan_run.id.matches('-').count(),
        4,
        "UUID should contain 4 hyphens"
    );
}

// ============================================================================
// Path Handling Tests
// ============================================================================

#[test]
fn test_create_scan_run_with_empty_path() {
    let path = Path::new("");

    let (scan_run, _) = create_scan_run(path, None);

    assert!(!scan_run.id.is_empty());
    assert_eq!(scan_run.repo_path, path.display().to_string());
}

#[test]
fn test_create_scan_run_with_root_path() {
    let path = Path::new("/");

    let (scan_run, _) = create_scan_run(path, None);

    assert_eq!(scan_run.repo_path, "/");
}

#[test]
fn test_create_scan_run_with_current_dir_path() {
    let path = Path::new(".");

    let (scan_run, _) = create_scan_run(path, None);

    assert_eq!(scan_run.repo_path, ".");
}

#[test]
fn test_create_scan_run_with_parent_dir_path() {
    let path = Path::new("..");

    let (scan_run, _) = create_scan_run(path, None);

    assert_eq!(scan_run.repo_path, "..");
}

#[test]
fn test_create_scan_run_with_long_path() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a deeply nested directory structure
    let mut nested_path = temp_dir.path().to_path_buf();
    for i in 0..50 {
        nested_path.push(format!("dir{}", i));
    }

    // Create the directories
    fs::create_dir_all(&nested_path).expect("Failed to create nested dirs");

    let (scan_run, _) = create_scan_run(&nested_path, Some("v1.0.0"));

    assert!(!scan_run.id.is_empty());
    assert_eq!(scan_run.version, Some("v1.0.0".to_string()));
    assert_eq!(scan_run.repo_path, nested_path.display().to_string());
}

#[test]
fn test_create_scan_run_with_path_containing_spaces() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path_with_spaces = temp_dir.path().join("dir with spaces");
    fs::create_dir(&path_with_spaces).expect("Failed to create dir with spaces");

    let (scan_run, _) = create_scan_run(&path_with_spaces, None);

    assert_eq!(scan_run.repo_path, path_with_spaces.display().to_string());
}

#[test]
fn test_create_scan_run_with_path_containing_unicode() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let unicode_path = temp_dir.path().join("目录名称");
    fs::create_dir(&unicode_path).expect("Failed to create unicode dir");

    let (scan_run, _) = create_scan_run(&unicode_path, None);

    assert_eq!(scan_run.repo_path, unicode_path.display().to_string());
}

#[test]
fn test_create_scan_run_with_path_containing_special_chars() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Test with various special characters (that are valid in filenames)
    let special_dirs = vec!["dir-name", "dir_name", "dir.name", "dir@name"];

    for dir_name in special_dirs {
        let special_path = temp_dir.path().join(dir_name);
        fs::create_dir(&special_path).expect("Failed to create dir");

        let (scan_run, _) = create_scan_run(&special_path, None);

        assert_eq!(scan_run.repo_path, special_path.display().to_string());
    }
}

// ============================================================================
// Commit SHA Tests
// ============================================================================

#[test]
fn test_create_scan_run_commit_sha_consistency() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    let (scan_run, commit_sha) = create_scan_run(path, None);

    // The returned commit_sha should match what's in scan_run
    if let Some(sha) = &scan_run.commit_sha {
        assert_eq!(
            commit_sha, *sha,
            "Returned commit_sha should match scan_run.commit_sha when Some"
        );
    } else {
        assert_eq!(
            commit_sha,
            String::new(),
            "Returned commit_sha should be empty when scan_run.commit_sha is None"
        );
    }
}

#[test]
fn test_create_scan_run_empty_commit_sha_when_no_git() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    let (_scan_run, commit_sha) = create_scan_run(path, None);

    // In a non-git directory, commit_sha should be empty
    assert_eq!(
        commit_sha,
        String::new(),
        "Commit SHA should be empty in non-git directory"
    );
}

#[test]
fn test_create_scan_run_multiple_calls_same_path() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    let (scan_run1, commit_sha1) = create_scan_run(path, None);
    let (scan_run2, commit_sha2) = create_scan_run(path, None);

    // Different IDs but same commit SHA (if any)
    assert_ne!(scan_run1.id, scan_run2.id);
    assert_eq!(commit_sha1, commit_sha2);
}

// ============================================================================
// Git Repository Tests
// ============================================================================

#[test]
fn test_create_scan_run_in_git_repo() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    // Initialize a git repository
    let repo = git2::Repository::init(path).expect("Failed to init git repo");

    // Create a commit
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
    assert!(
        !commit_sha.is_empty(),
        "Commit SHA should not be empty in git repo"
    );
    assert_eq!(scan_run.commit_sha, Some(commit_sha.clone()));

    // Should have a branch
    assert!(
        scan_run.branch.is_some(),
        "Branch should be set in git repo"
    );
}

#[test]
fn test_create_scan_run_git_info_with_version() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    // Initialize a git repository with a commit
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

    let version = "v1.0.0";
    let (scan_run, _) = create_scan_run(path, Some(version));

    // Both git info and version should be present
    assert!(scan_run.commit_sha.is_some());
    assert!(scan_run.branch.is_some());
    assert_eq!(scan_run.version, Some(version.to_string()));
}

// ============================================================================
// Timestamp Tests
// ============================================================================

#[test]
fn test_create_scan_run_has_timestamp() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    let (scan_run, _) = create_scan_run(path, None);

    // Just verify that scanned_at is set (it's always set by ScanRun::new)
    // We can't easily test the exact time, but we can verify it's reasonable
    let now = chrono::Utc::now();
    let scan_time = scan_run.scanned_at;

    // The scan should have happened within the last minute
    let duration = now.signed_duration_since(scan_time);
    assert!(
        duration.num_seconds() >= 0 && duration.num_seconds() < 60,
        "Scan timestamp should be recent (within last minute)"
    );
}

#[test]
fn test_create_scan_run_timestamps_increase() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    let (scan_run1, _) = create_scan_run(path, None);

    // Small delay to ensure different timestamp
    std::thread::sleep(std::time::Duration::from_millis(10));

    let (scan_run2, _) = create_scan_run(path, None);

    assert!(
        scan_run2.scanned_at >= scan_run1.scanned_at,
        "Second scan should have equal or later timestamp"
    );
}

// ============================================================================
// Boundary and Edge Cases
// ============================================================================

#[test]
fn test_create_scan_run_idempotency_same_path() {
    // Verify that creating scan runs with the same path still generates unique IDs
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    let mut ids = std::collections::HashSet::new();

    for _ in 0..10 {
        let (scan_run, _) = create_scan_run(path, Some("v1.0.0"));
        ids.insert(scan_run.id.clone());
    }

    // All IDs should be unique
    assert_eq!(ids.len(), 10, "All scan runs should have unique IDs");
}

#[test]
fn test_create_scan_run_different_paths_different_ids() {
    let temp_dir1 = TempDir::new().expect("Failed to create temp dir 1");
    let temp_dir2 = TempDir::new().expect("Failed to create temp dir 2");

    let (scan_run1, _) = create_scan_run(temp_dir1.path(), None);
    let (scan_run2, _) = create_scan_run(temp_dir2.path(), None);

    assert_ne!(scan_run1.id, scan_run2.id);
    assert_ne!(scan_run1.repo_path, scan_run2.repo_path);
}

#[test]
fn test_create_scan_run_all_fields_populated() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();
    let version = "v1.0.0";

    let (scan_run, _) = create_scan_run(path, Some(version));

    // Verify all essential fields are populated
    assert!(!scan_run.id.is_empty(), "ID should be populated");
    assert!(
        !scan_run.repo_path.is_empty(),
        "repo_path should be populated"
    );
    assert_eq!(
        scan_run.version,
        Some(version.to_string()),
        "version should be populated"
    );
    // commit_sha and branch may be None if not a git repo, that's OK
    // scanned_at is always populated
}

#[test]
fn test_create_scan_run_with_symlink() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let target_dir = temp_dir.path().join("target");
    fs::create_dir(&target_dir).expect("Failed to create target dir");

    let symlink_path = temp_dir.path().join("link");

    #[cfg(unix)]
    std::os::unix::fs::symlink(&target_dir, &symlink_path).expect("Failed to create symlink");

    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&target_dir, &symlink_path)
        .expect("Failed to create symlink");

    let (scan_run, _) = create_scan_run(&symlink_path, None);

    assert!(!scan_run.id.is_empty());
    assert_eq!(scan_run.repo_path, symlink_path.display().to_string());
}

#[test]
fn test_create_scan_run_none_vs_some_empty_version() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();

    let (scan_run_none, _) = create_scan_run(path, None);
    let (scan_run_empty, _) = create_scan_run(path, Some(""));

    // None should be None
    assert_eq!(scan_run_none.version, None);

    // Some("") should be Some(empty string), not None
    assert_eq!(scan_run_empty.version, Some(String::new()));
    assert_ne!(scan_run_none.version, scan_run_empty.version);
}

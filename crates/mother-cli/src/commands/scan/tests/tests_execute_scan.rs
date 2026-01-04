//! Tests for execute_scan and related helper functions

use tempfile::TempDir;
use std::path::Path;

// Import the parent module functions through super
use super::super::{create_scan_run, log_scan_summary, log_scan_run_info, shutdown_lsp};
use super::super::{Phase1Result, Phase2Result, Phase3Result};
use mother_core::graph::model::ScanRun;
use mother_core::lsp::LspServerManager;

// ============================================================================
// Tests for create_scan_run
// ============================================================================

#[test]
fn test_create_scan_run_without_version() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();
    
    let (scan_run, commit_sha) = create_scan_run(path, None);
    
    assert_eq!(scan_run.repo_path, path.display().to_string());
    assert!(!scan_run.id.is_empty());
    assert!(scan_run.version.is_none());
    // commit_sha might be empty if not a git repo
    assert_eq!(commit_sha, scan_run.commit_sha.clone().unwrap_or_default());
}

#[test]
fn test_create_scan_run_with_version() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();
    let version = "v1.2.3";
    
    let (scan_run, _commit_sha) = create_scan_run(path, Some(version));
    
    assert_eq!(scan_run.version, Some(version.to_string()));
}

#[test]
fn test_create_scan_run_generates_unique_ids() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();
    
    let (scan_run1, _) = create_scan_run(path, None);
    let (scan_run2, _) = create_scan_run(path, None);
    
    // Each scan run should have a unique ID
    assert_ne!(scan_run1.id, scan_run2.id);
}

#[test]
fn test_create_scan_run_commit_sha_matches() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();
    
    let (scan_run, commit_sha) = create_scan_run(path, None);
    
    // The returned commit_sha should match what's in scan_run
    if let Some(sha) = &scan_run.commit_sha {
        assert_eq!(commit_sha, *sha);
    } else {
        assert_eq!(commit_sha, String::new());
    }
}

#[test]
fn test_create_scan_run_with_different_versions() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();
    
    let versions = vec!["v1.0.0", "v2.0.0-beta", "latest", ""];
    
    for version in versions {
        let (scan_run, _) = create_scan_run(path, Some(version));
        assert_eq!(scan_run.version, Some(version.to_string()));
    }
}

// ============================================================================
// Tests for log_scan_summary
// ============================================================================

#[test]
fn test_log_scan_summary_no_errors() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 10,
        reused_file_count: 5,
        error_count: 0,
    };
    
    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 100,
        error_count: 0,
    };
    
    let phase3 = Phase3Result {
        reference_count: 50,
        error_count: 0,
    };
    
    // Should not panic
    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_with_errors() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 10,
        reused_file_count: 5,
        error_count: 2,
    };
    
    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 100,
        error_count: 3,
    };
    
    let phase3 = Phase3Result {
        reference_count: 50,
        error_count: 1,
    };
    
    // Should not panic with errors
    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_all_zeros() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 0,
        reused_file_count: 0,
        error_count: 0,
    };
    
    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 0,
        error_count: 0,
    };
    
    let phase3 = Phase3Result {
        reference_count: 0,
        error_count: 0,
    };
    
    // Should handle zero counts gracefully
    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_large_counts() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 10000,
        reused_file_count: 5000,
        error_count: 100,
    };
    
    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 50000,
        error_count: 200,
    };
    
    let phase3 = Phase3Result {
        reference_count: 100000,
        error_count: 50,
    };
    
    // Should handle large counts
    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_only_phase1_errors() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 5,
        reused_file_count: 3,
        error_count: 10,
    };
    
    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 20,
        error_count: 0,
    };
    
    let phase3 = Phase3Result {
        reference_count: 15,
        error_count: 0,
    };
    
    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_only_phase2_errors() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 5,
        reused_file_count: 3,
        error_count: 0,
    };
    
    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 20,
        error_count: 8,
    };
    
    let phase3 = Phase3Result {
        reference_count: 15,
        error_count: 0,
    };
    
    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_only_phase3_errors() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 5,
        reused_file_count: 3,
        error_count: 0,
    };
    
    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 20,
        error_count: 0,
    };
    
    let phase3 = Phase3Result {
        reference_count: 15,
        error_count: 12,
    };
    
    log_scan_summary(&phase1, &phase2, &phase3);
}

// ============================================================================
// Tests for log_scan_run_info
// ============================================================================

#[test]
fn test_log_scan_run_info_with_commit() {
    let scan_run = ScanRun::new("/test/repo")
        .with_commit("abc123def456")
        .with_branch("main");
    let commit_sha = "abc123def456";
    
    // Should not panic
    log_scan_run_info(&scan_run, commit_sha);
}

#[test]
fn test_log_scan_run_info_without_commit() {
    let scan_run = ScanRun::new("/test/repo");
    let commit_sha = "";
    
    // Should handle empty commit sha
    log_scan_run_info(&scan_run, commit_sha);
}

#[test]
fn test_log_scan_run_info_with_branch() {
    let scan_run = ScanRun::new("/test/repo")
        .with_branch("feature/test");
    let commit_sha = "abc123";
    
    log_scan_run_info(&scan_run, commit_sha);
}

#[test]
fn test_log_scan_run_info_without_branch() {
    let scan_run = ScanRun::new("/test/repo")
        .with_commit("abc123");
    let commit_sha = "abc123";
    
    // Should handle None branch
    log_scan_run_info(&scan_run, commit_sha);
}

#[test]
fn test_log_scan_run_info_long_commit_sha() {
    let scan_run = ScanRun::new("/test/repo")
        .with_commit("a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0");
    let commit_sha = "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0";
    
    log_scan_run_info(&scan_run, commit_sha);
}

#[test]
fn test_log_scan_run_info_special_branch_names() {
    let branches = vec![
        "main",
        "master",
        "develop",
        "feature/new-feature",
        "hotfix/urgent-fix",
        "release/v1.0.0",
        "user/john/experiment",
    ];
    
    for branch in branches {
        let scan_run = ScanRun::new("/test/repo")
            .with_commit("abc123")
            .with_branch(branch);
        log_scan_run_info(&scan_run, "abc123");
    }
}

// ============================================================================
// Tests for shutdown_lsp
// ============================================================================

#[tokio::test]
async fn test_shutdown_lsp_empty_manager() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mut lsp_manager = LspServerManager::new(temp_dir.path());
    
    // Should not panic even with no servers running
    shutdown_lsp(&mut lsp_manager).await;
}

#[tokio::test]
async fn test_shutdown_lsp_multiple_calls() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mut lsp_manager = LspServerManager::new(temp_dir.path());
    
    // Should handle multiple shutdown calls gracefully
    shutdown_lsp(&mut lsp_manager).await;
    shutdown_lsp(&mut lsp_manager).await;
}

// ============================================================================
// Tests for edge cases and boundaries
// ============================================================================

#[test]
fn test_create_scan_run_with_empty_path() {
    // Path operations should still work with empty components
    let path = Path::new("");
    let (scan_run, _) = create_scan_run(path, None);
    
    assert!(!scan_run.id.is_empty());
}

#[test]
fn test_create_scan_run_with_long_path() {
    let long_path = format!("/tmp/{}", "a/".repeat(100));
    let path = Path::new(&long_path);
    
    let (scan_run, _) = create_scan_run(path, Some("v1.0.0"));
    
    assert!(!scan_run.id.is_empty());
    assert_eq!(scan_run.version, Some("v1.0.0".to_string()));
}

#[test]
fn test_create_scan_run_with_special_characters_in_version() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();
    
    let special_versions = vec![
        "v1.0.0-alpha+build.123",
        "2.0.0-beta.1",
        "v3.0.0-rc.1+20130313144700",
    ];
    
    for version in special_versions {
        let (scan_run, _) = create_scan_run(path, Some(version));
        assert_eq!(scan_run.version, Some(version.to_string()));
    }
}

#[test]
fn test_log_scan_summary_max_values() {
    // Test with large values that won't overflow when summed
    // Using usize::MAX / 4 to avoid overflow when adding 3 error counts
    let large_val = usize::MAX / 4;
    
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: large_val,
        reused_file_count: large_val,
        error_count: large_val,
    };
    
    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: large_val,
        error_count: large_val,
    };
    
    let phase3 = Phase3Result {
        reference_count: large_val,
        error_count: large_val,
    };
    
    // Should handle large values without overflow
    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_run_info_empty_repo_path() {
    let scan_run = ScanRun::new("");
    let commit_sha = "abc123";
    
    log_scan_run_info(&scan_run, commit_sha);
}

#[test]
fn test_log_scan_run_info_very_long_repo_path() {
    let long_path = format!("/very/long/path/{}", "directory/".repeat(50));
    let scan_run = ScanRun::new(&long_path)
        .with_commit("abc123")
        .with_branch("main");
    
    log_scan_run_info(&scan_run, "abc123");
}

#[test]
fn test_create_scan_run_idempotency() {
    // Creating scan runs with same path should still generate unique IDs
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path();
    
    let mut ids = std::collections::HashSet::new();
    
    for _ in 0..10 {
        let (scan_run, _) = create_scan_run(path, Some("v1.0.0"));
        ids.insert(scan_run.id.clone());
    }
    
    // All IDs should be unique
    assert_eq!(ids.len(), 10);
}

#[test]
fn test_log_scan_summary_mixed_success_and_errors() {
    // Test various combinations of success and error counts
    let test_cases = vec![
        (10, 5, 2, 100, 3, 50, 1),  // Some errors in all phases
        (100, 50, 0, 1000, 10, 500, 5), // Some phase1 no errors, others have
        (0, 10, 5, 50, 0, 25, 2),   // No new files
        (20, 0, 1, 200, 8, 100, 0), // No reused files
    ];
    
    for (new, reused, e1, symbols, e2, refs, e3) in test_cases {
        let phase1 = Phase1Result {
            files_to_process: vec![],
            new_file_count: new,
            reused_file_count: reused,
            error_count: e1,
        };
        
        let phase2 = Phase2Result {
            symbols: vec![],
            symbol_count: symbols,
            error_count: e2,
        };
        
        let phase3 = Phase3Result {
            reference_count: refs,
            error_count: e3,
        };
        
        log_scan_summary(&phase1, &phase2, &phase3);
    }
}

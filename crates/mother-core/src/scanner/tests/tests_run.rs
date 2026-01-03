//! Tests for scan run

use crate::graph::model::ScanRun;

#[test]
fn test_scan_run_creation() {
    let scan = ScanRun::new("/path/to/repo");

    assert_eq!(scan.repo_path, "/path/to/repo");
    assert!(!scan.id.is_empty());
    assert!(scan.commit_sha.is_none());
    assert!(scan.branch.is_none());
    assert!(scan.version.is_none());
}

#[test]
fn test_scan_run_builder() {
    let scan = ScanRun::new("/path/to/repo")
        .with_commit("abc123")
        .with_branch("main")
        .with_version("v1.0.0");

    assert_eq!(scan.commit_sha, Some("abc123".to_string()));
    assert_eq!(scan.branch, Some("main".to_string()));
    assert_eq!(scan.version, Some("v1.0.0".to_string()));
}

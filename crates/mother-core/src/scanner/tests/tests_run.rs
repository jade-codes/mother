//! Tests for scan run

use crate::graph::model::ScanRun;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

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

/// Helper function to create a test git repository with a commit
fn create_test_repo_with_commit(dir: &Path, branch_name: &str) -> Result<git2::Oid, git2::Error> {
    let repo = git2::Repository::init(dir)?;
    
    // Create a test file
    let file_path = dir.join("test.txt");
    fs::write(&file_path, "test content").expect("Failed to write test file");
    
    // Configure signature
    let sig = git2::Signature::now("Test User", "test@example.com")?;
    
    // Add file to index
    let mut index = repo.index()?;
    index.add_path(Path::new("test.txt"))?;
    index.write()?;
    
    // Create tree from index
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    
    // Create initial commit
    let commit_oid = repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;
    
    // Create and checkout the specified branch if not on it already
    if branch_name != "master" && branch_name != "main" {
        let commit = repo.find_commit(commit_oid)?;
        repo.branch(branch_name, &commit, false)?;
        repo.set_head(&format!("refs/heads/{}", branch_name))?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    }
    
    Ok(commit_oid)
}

#[test]
fn test_with_git_info_valid_repo() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();
    
    let commit_oid = create_test_repo_with_commit(repo_path, "main")
        .expect("Failed to create test repo");
    
    let scan = ScanRun::new(repo_path.to_str().unwrap()).with_git_info();
    
    assert!(scan.commit_sha.is_some(), "commit_sha should be populated");
    assert_eq!(
        scan.commit_sha.unwrap(),
        commit_oid.to_string(),
        "commit_sha should match the actual commit"
    );
    
    // Branch name might be "master" or "main" depending on git version/config
    assert!(
        scan.branch.is_some(),
        "branch should be populated"
    );
    let branch = scan.branch.unwrap();
    assert!(
        branch == "main" || branch == "master",
        "branch should be main or master, got: {}",
        branch
    );
}

#[test]
fn test_with_git_info_non_existent_directory() {
    let scan = ScanRun::new("/this/path/does/not/exist/anywhere").with_git_info();
    
    assert!(scan.commit_sha.is_none(), "commit_sha should remain None for non-existent directory");
    assert!(scan.branch.is_none(), "branch should remain None for non-existent directory");
}

#[test]
fn test_with_git_info_non_git_directory() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();
    
    // Create a regular directory without git initialization
    let test_file = repo_path.join("test.txt");
    fs::write(test_file, "test content").expect("Failed to write test file");
    
    let scan = ScanRun::new(repo_path.to_str().unwrap()).with_git_info();
    
    assert!(scan.commit_sha.is_none(), "commit_sha should remain None for non-git directory");
    assert!(scan.branch.is_none(), "branch should remain None for non-git directory");
}

#[test]
fn test_with_git_info_subdirectory_discovers_parent_repo() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();
    
    let commit_oid = create_test_repo_with_commit(repo_path, "main")
        .expect("Failed to create test repo");
    
    // Create a subdirectory within the repo
    let subdir = repo_path.join("subdir");
    fs::create_dir(&subdir).expect("Failed to create subdirectory");
    
    // Create ScanRun pointing to subdirectory - git2::Repository::discover should find the parent repo
    let scan = ScanRun::new(subdir.to_str().unwrap()).with_git_info();
    
    assert!(scan.commit_sha.is_some(), "commit_sha should be populated from parent repo");
    assert_eq!(
        scan.commit_sha.unwrap(),
        commit_oid.to_string(),
        "commit_sha should match the parent repo's commit"
    );
    assert!(scan.branch.is_some(), "branch should be populated from parent repo");
}

#[test]
fn test_with_git_info_custom_branch() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();
    
    let commit_oid = create_test_repo_with_commit(repo_path, "feature/test-branch")
        .expect("Failed to create test repo");
    
    let scan = ScanRun::new(repo_path.to_str().unwrap()).with_git_info();
    
    assert!(scan.commit_sha.is_some(), "commit_sha should be populated");
    assert_eq!(
        scan.commit_sha.unwrap(),
        commit_oid.to_string(),
        "commit_sha should match the actual commit"
    );
    assert_eq!(
        scan.branch,
        Some("feature/test-branch".to_string()),
        "branch should match the custom branch name"
    );
}

#[test]
fn test_with_git_info_detached_head() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();
    
    let commit_oid = create_test_repo_with_commit(repo_path, "main")
        .expect("Failed to create test repo");
    
    // Detach HEAD to the commit
    let repo = git2::Repository::open(repo_path).expect("Failed to open repo");
    repo.set_head_detached(commit_oid).expect("Failed to detach HEAD");
    
    let scan = ScanRun::new(repo_path.to_str().unwrap()).with_git_info();
    
    assert!(scan.commit_sha.is_some(), "commit_sha should be populated even in detached HEAD");
    assert_eq!(
        scan.commit_sha.unwrap(),
        commit_oid.to_string(),
        "commit_sha should match the detached commit"
    );
    // In detached HEAD state, shorthand() returns Some("HEAD"), so branch will be "HEAD"
    assert_eq!(
        scan.branch,
        Some("HEAD".to_string()),
        "branch should be 'HEAD' in detached HEAD state"
    );
}

#[test]
fn test_with_git_info_empty_repo() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();
    
    // Initialize an empty git repository without any commits
    git2::Repository::init(repo_path).expect("Failed to init repo");
    
    let scan = ScanRun::new(repo_path.to_str().unwrap()).with_git_info();
    
    // Empty repo has no HEAD, so both should be None
    assert!(scan.commit_sha.is_none(), "commit_sha should be None for empty repo");
    assert!(scan.branch.is_none(), "branch should be None for empty repo");
}

#[test]
fn test_with_git_info_preserves_existing_values() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();
    
    create_test_repo_with_commit(repo_path, "main")
        .expect("Failed to create test repo");
    
    // Create a ScanRun with pre-existing values
    let scan = ScanRun::new(repo_path.to_str().unwrap())
        .with_version("v1.0.0")
        .with_git_info();
    
    // Git info should be populated
    assert!(scan.commit_sha.is_some(), "commit_sha should be populated");
    assert!(scan.branch.is_some(), "branch should be populated");
    
    // Pre-existing version should be preserved
    assert_eq!(
        scan.version,
        Some("v1.0.0".to_string()),
        "version should be preserved after with_git_info"
    );
}

#[test]
fn test_with_git_info_chaining_with_other_methods() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();
    
    let commit_oid = create_test_repo_with_commit(repo_path, "main")
        .expect("Failed to create test repo");
    
    // Chain with_git_info with other builder methods
    let scan = ScanRun::new(repo_path.to_str().unwrap())
        .with_git_info()
        .with_version("v2.0.0");
    
    // All values should be populated
    assert_eq!(
        scan.commit_sha.unwrap(),
        commit_oid.to_string(),
        "commit_sha should be populated from git"
    );
    assert!(scan.branch.is_some(), "branch should be populated from git");
    assert_eq!(
        scan.version,
        Some("v2.0.0".to_string()),
        "version should be set by with_version"
    );
}

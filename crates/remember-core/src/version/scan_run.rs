//! Scan run: A versioned snapshot of a repository scan

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A versioned scan run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRun {
    /// Unique identifier for this scan run
    pub id: String,
    /// Path to the repository
    pub repo_path: String,
    /// Git commit SHA (if available)
    pub commit_sha: Option<String>,
    /// Git branch (if available)
    pub branch: Option<String>,
    /// When the scan was performed
    pub scanned_at: DateTime<Utc>,
    /// User-provided version tag
    pub version: Option<String>,
}

impl ScanRun {
    /// Create a new scan run
    #[must_use]
    pub fn new(repo_path: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            repo_path: repo_path.into(),
            commit_sha: None,
            branch: None,
            scanned_at: Utc::now(),
            version: None,
        }
    }

    /// Set the commit SHA
    #[must_use]
    pub fn with_commit(mut self, sha: impl Into<String>) -> Self {
        self.commit_sha = Some(sha.into());
        self
    }

    /// Set the branch
    #[must_use]
    pub fn with_branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(branch.into());
        self
    }

    /// Set the version tag
    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Try to populate git info from the repository
    #[must_use]
    pub fn with_git_info(mut self) -> Self {
        if let Ok(repo) = git2::Repository::discover(&self.repo_path) {
            // Get HEAD commit
            if let Ok(head) = repo.head() {
                if let Some(oid) = head.target() {
                    self.commit_sha = Some(oid.to_string());
                }
                if let Some(name) = head.shorthand() {
                    self.branch = Some(name.to_string());
                }
            }
        }
        self
    }
}

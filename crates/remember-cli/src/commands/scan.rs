//! Scan command: Scan a repository and store in Neo4j

use std::path::Path;

use anyhow::Result;
use remember_core::graph::neo4j::{Neo4jClient, Neo4jConfig};
use remember_core::scanner::{compute_file_hash, Scanner};
use remember_core::version::ScanRun;
use tracing::info;

/// Run the scan command
///
/// # Errors
/// Returns an error if scanning or Neo4j operations fail.
pub async fn run(
    path: &Path,
    neo4j_uri: &str,
    neo4j_user: &str,
    neo4j_password: &str,
    version: Option<&str>,
) -> Result<()> {
    info!("Scanning repository: {}", path.display());

    // Create scan run with git info
    let mut scan_run = ScanRun::new(path.display().to_string()).with_git_info();
    if let Some(v) = version {
        scan_run = scan_run.with_version(v);
    }

    let commit_sha = scan_run.commit_sha.clone().unwrap_or_default();
    info!(
        "Created scan run: {} (commit: {}, branch: {:?})",
        scan_run.id,
        if commit_sha.is_empty() { "none" } else { &commit_sha },
        scan_run.branch
    );

    // Connect to Neo4j
    let config = Neo4jConfig::new(neo4j_uri, neo4j_user, neo4j_password);
    let client = Neo4jClient::connect(&config).await?;

    // Create scan run and check if commit is new
    let is_new_commit = client.create_scan_run(&scan_run).await?;
    
    if !is_new_commit {
        info!("✓ Commit already scanned, linked scan run to existing data");
        return Ok(());
    }

    info!("New commit detected, scanning files...");

    // Scan files
    let scanner = Scanner::new(path);
    let files: Vec<_> = scanner.scan().collect();

    info!("Found {} files to process", files.len());

    let mut new_file_count = 0;
    let mut reused_file_count = 0;
    let mut symbol_count = 0;

    for file in &files {
        // Compute file hash
        let hash = match compute_file_hash(&file.path) {
            Ok(h) => h,
            Err(e) => {
                tracing::warn!("Failed to hash {}: {}", file.path.display(), e);
                continue;
            }
        };

        let file_path_str = file.path.display().to_string();

        // Create or link file
        match client
            .create_file_if_new(
                &file_path_str,
                &hash,
                &file.language.to_string(),
                &commit_sha,
            )
            .await?
        {
            Some(_content_hash) => {
                new_file_count += 1;
                // TODO: Extract symbols via LSP and store in Neo4j
                tracing::debug!("New file: {} (hash: {})", file_path_str, hash);
            }
            None => {
                reused_file_count += 1;
                tracing::debug!("Reused file: {} (hash: {})", file_path_str, hash);
            }
        }
    }

    info!(
        "✓ Scan completed: {} new files, {} reused (unchanged content), {} symbols extracted",
        new_file_count, reused_file_count, symbol_count
    );
    Ok(())
}

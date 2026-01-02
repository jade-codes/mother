//! Scan command: Scan a repository and store in Neo4j

use std::path::Path;

use anyhow::Result;
use remember_core::graph::convert::convert_symbols;
use remember_core::graph::neo4j::{Neo4jClient, Neo4jConfig};
use remember_core::lsp::LspServerManager;
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

    // Canonicalize the path for LSP
    let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    // Create scan run with git info
    let mut scan_run = ScanRun::new(abs_path.display().to_string()).with_git_info();
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
    let scanner = Scanner::new(&abs_path);
    let files: Vec<_> = scanner.scan().collect();

    info!("Found {} files to process", files.len());

    // Initialize LSP manager
    let mut lsp_manager = LspServerManager::new(&abs_path);

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
        let content_hash = match client
            .create_file_if_new(
                &file_path_str,
                &hash,
                &file.language.to_string(),
                &commit_sha,
            )
            .await?
        {
            Some(hash) => {
                new_file_count += 1;
                hash
            }
            None => {
                reused_file_count += 1;
                tracing::debug!("Reused file: {} (hash: {})", file_path_str, hash);
                continue;
            }
        };

        // Extract symbols via LSP
        let lsp_client = match lsp_manager.get_client(file.language).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    "Failed to get LSP client for {:?}: {}",
                    file.language,
                    e
                );
                continue;
            }
        };

        // Open the file in LSP
        let file_uri = format!("file://{}", file.path.display());
        let file_content = match std::fs::read_to_string(&file.path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to read {}: {}", file.path.display(), e);
                continue;
            }
        };

        if let Err(e) = lsp_client
            .did_open(&file_uri, &file.language.to_string(), &file_content)
            .await
        {
            tracing::warn!("Failed to open file in LSP: {}", e);
            continue;
        }

        // Get document symbols
        let lsp_symbols = match lsp_client.document_symbols(&file_uri).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Failed to get symbols for {}: {}", file.path.display(), e);
                continue;
            }
        };

        // Convert LSP symbols to graph nodes
        let symbols = convert_symbols(&lsp_symbols, &file.path);
        let file_symbol_count = symbols.len();

        // Store symbols in Neo4j
        if let Err(e) = client.create_symbols_batch(&symbols, &content_hash).await {
            tracing::warn!("Failed to store symbols for {}: {}", file.path.display(), e);
            continue;
        }

        symbol_count += file_symbol_count;
        tracing::debug!(
            "Extracted {} symbols from: {}",
            file_symbol_count,
            file_path_str
        );
    }

    // Shutdown LSP servers
    if let Err(e) = lsp_manager.shutdown_all().await {
        tracing::warn!("Failed to shutdown LSP servers: {}", e);
    }

    info!(
        "✓ Scan completed: {} new files, {} reused (unchanged content), {} symbols extracted",
        new_file_count, reused_file_count, symbol_count
    );
    Ok(())
}

//! Scan command: Scan a repository and store in Neo4j
//!
//! This module implements a 3-phase scanning process:
//! 1. Phase 1: Discover files, open in LSP, create in Neo4j
//! 2. Phase 2: Extract symbols from LSP, enrich with hover, store in Neo4j
//! 3. Phase 3: Extract references, create symbol-to-symbol edges

mod phase1;
mod phase2;
mod phase3;

use std::path::Path;

use anyhow::Result;
use mother_core::graph::model::ScanRun;
use mother_core::graph::neo4j::{Neo4jClient, Neo4jConfig};
use mother_core::lsp::LspServerManager;
use mother_core::scanner::{DiscoveredFile, Language, Scanner};
use tracing::info;

pub use phase1::Phase1Result;
pub use phase2::Phase2Result;
pub use phase3::Phase3Result;

// ============================================================================
// Types shared across phases
// ============================================================================

/// A file that needs symbol extraction (output from Phase 1)
pub(crate) struct FileToProcess {
    pub path: std::path::PathBuf,
    pub file_uri: String,
    pub content_hash: String,
    pub language: Language,
}

/// Symbol position info for reference extraction (output from Phase 2)
pub(crate) struct SymbolInfo {
    pub id: String,
    pub file_uri: String,
    pub start_line: u32,
    pub end_line: u32,
    pub start_col: u32,
    pub language: Language,
}

// ============================================================================
// Main entry point
// ============================================================================

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

    let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let (scan_run, commit_sha) = create_scan_run(&abs_path, version);

    log_scan_run_info(&scan_run, &commit_sha);

    let client = connect_neo4j(neo4j_uri, neo4j_user, neo4j_password).await?;

    if !client.create_scan_run(&scan_run).await? {
        info!("✓ Commit already scanned, linked scan run to existing data");
        return Ok(());
    }

    execute_scan(&abs_path, &client, &commit_sha).await
}

/// Execute the scan workflow after determining a new commit needs scanning
async fn execute_scan(abs_path: &Path, client: &Neo4jClient, commit_sha: &str) -> Result<()> {
    info!("New commit detected, scanning files...");

    let files: Vec<DiscoveredFile> = Scanner::new(abs_path).scan().collect();
    info!("Found {} files to process", files.len());

    let mut lsp_manager = LspServerManager::new(abs_path);

    let phase1 = phase1::run(&files, client, &mut lsp_manager, commit_sha).await?;
    let phase2 = phase2::run(&phase1.files_to_process, client, &mut lsp_manager).await?;
    let phase3 = phase3::run(&phase2.symbols, client, &mut lsp_manager).await?;

    shutdown_lsp(&mut lsp_manager).await;

    log_scan_summary(&phase1, &phase2, &phase3);
    Ok(())
}

fn log_scan_summary(phase1: &Phase1Result, phase2: &Phase2Result, phase3: &Phase3Result) {
    info!(
        "✓ Scan completed: {} new files, {} reused, {} symbols, {} references",
        phase1.new_file_count,
        phase1.reused_file_count,
        phase2.symbol_count,
        phase3.reference_count
    );
}

fn create_scan_run(abs_path: &Path, version: Option<&str>) -> (ScanRun, String) {
    let mut scan_run = ScanRun::new(abs_path.display().to_string()).with_git_info();
    if let Some(v) = version {
        scan_run = scan_run.with_version(v);
    }
    let commit_sha = scan_run.commit_sha.clone().unwrap_or_default();
    (scan_run, commit_sha)
}

fn log_scan_run_info(scan_run: &ScanRun, commit_sha: &str) {
    info!(
        "Created scan run: {} (commit: {}, branch: {:?})",
        scan_run.id,
        if commit_sha.is_empty() {
            "none"
        } else {
            commit_sha
        },
        scan_run.branch
    );
}

async fn connect_neo4j(uri: &str, user: &str, password: &str) -> Result<Neo4jClient> {
    let config = Neo4jConfig::new(uri, user, password);
    Ok(Neo4jClient::connect(&config).await?)
}

async fn shutdown_lsp(lsp_manager: &mut LspServerManager) {
    if let Err(e) = lsp_manager.shutdown_all().await {
        tracing::warn!("Failed to shutdown LSP servers: {}", e);
    }
}

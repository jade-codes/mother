//! Phase 1: Open files in LSP and create in Neo4j

use anyhow::Result;
use mother_core::graph::neo4j::Neo4jClient;
use mother_core::lsp::LspServerManager;
use mother_core::scanner::DiscoveredFile;
use tracing::info;

use super::FileToProcess;

/// Results from Phase 1
pub struct Phase1Result {
    pub files_to_process: Vec<FileToProcess>,
    pub new_file_count: usize,
    pub reused_file_count: usize,
    pub error_count: usize,
}

/// Run Phase 1: Open files in LSP and create in Neo4j
pub async fn run(
    files: &[DiscoveredFile],
    client: &Neo4jClient,
    lsp_manager: &mut LspServerManager,
    commit_sha: &str,
) -> Result<Phase1Result> {
    info!("Phase 1: Opening files in LSP...");

    let mut result = Phase1Result {
        files_to_process: Vec::new(),
        new_file_count: 0,
        reused_file_count: 0,
        error_count: 0,
    };

    for file in files {
        let outcome = process_file(file, client, lsp_manager, commit_sha).await;
        handle_file_result(outcome, file, &mut result);
    }

    log_phase1_errors(&result);
    Ok(result)
}

/// Handle the result of processing a single file
fn handle_file_result(
    outcome: Result<Option<FileToProcess>>,
    file: &DiscoveredFile,
    result: &mut Phase1Result,
) {
    match outcome {
        Ok(Some(file_to_process)) => {
            result.new_file_count += 1;
            result.files_to_process.push(file_to_process);
        }
        Ok(None) => {
            result.reused_file_count += 1;
        }
        Err(e) => {
            result.error_count += 1;
            tracing::warn!("Failed to process {}: {}", file.path.display(), e);
        }
    }
}

/// Log error summary for phase 1
fn log_phase1_errors(result: &Phase1Result) {
    if result.error_count > 0 {
        tracing::warn!("Phase 1: {} files failed to process", result.error_count);
    }
}

/// Process a single file for phase 1. Returns Ok(Some) for new files, Ok(None) for reused.
async fn process_file(
    file: &DiscoveredFile,
    client: &Neo4jClient,
    lsp_manager: &mut LspServerManager,
    commit_sha: &str,
) -> Result<Option<FileToProcess>> {
    let hash = file.compute_hash()?;
    let file_path_str = file.path.display().to_string();

    // Check if file already exists in Neo4j
    let content_hash = match client
        .create_file_if_new(
            &file_path_str,
            &hash,
            &file.language.to_string(),
            commit_sha,
        )
        .await?
    {
        Some(h) => h,
        None => return Ok(None), // File reused
    };

    // Get LSP client and open file
    let lsp_client = lsp_manager.get_client(file.language).await?;
    let file_uri = format!("file://{}", file.path.display());
    let file_content = std::fs::read_to_string(&file.path)?;
    lsp_client
        .did_open(&file_uri, &file.language.to_string(), &file_content)
        .await?;

    Ok(Some(FileToProcess {
        path: file.path.clone(),
        file_uri,
        content_hash,
        language: file.language,
    }))
}

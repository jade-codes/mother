//! Diff command: Compare commits or branches

use anyhow::Result;
use tracing::info;

/// Run the diff command
///
/// # Errors
/// Returns an error if the diff operation fails.
pub async fn run(
    from: &str,
    to: &str,
    _neo4j_uri: &str,
    _neo4j_user: &str,
    _neo4j_password: &str,
) -> Result<()> {
    info!("Comparing {} to {}", from, to);

    // TODO: Connect to Neo4j and compare commits/branches
    // TODO: Show symbol changes between versions

    info!("Diff not yet implemented");
    Ok(())
}

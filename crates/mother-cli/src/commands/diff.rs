//! Diff command: Compare two scan versions

use anyhow::Result;
use tracing::info;

/// Run the diff command
///
/// # Errors
/// Returns an error if the diff fails.
pub async fn run(
    from: &str,
    to: &str,
    _neo4j_uri: &str,
    _neo4j_user: &str,
    _neo4j_password: &str,
) -> Result<()> {
    info!("Comparing versions: {} -> {}", from, to);

    // TODO: Connect to Neo4j
    // TODO: Query for differences between versions
    // TODO: Display added/removed/modified symbols

    info!("Diff not yet implemented");
    Ok(())
}

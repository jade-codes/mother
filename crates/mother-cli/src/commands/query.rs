//! Query command: Execute Cypher queries

use anyhow::Result;
use tracing::info;

/// Run the query command
///
/// # Errors
/// Returns an error if the query fails.
pub async fn run(
    query: &str,
    _neo4j_uri: &str,
    _neo4j_user: &str,
    _neo4j_password: &str,
) -> Result<()> {
    info!("Executing query: {}", query);

    // TODO: Connect to Neo4j and execute query
    // TODO: Format and display results

    info!("Query execution not yet implemented");
    Ok(())
}

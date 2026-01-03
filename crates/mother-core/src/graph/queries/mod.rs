//! Neo4j query modules organized by entity

mod file;
mod scan;
mod symbol;

// Re-export Neo4jClient for the impl blocks
pub(super) use super::neo4j::Neo4jClient;

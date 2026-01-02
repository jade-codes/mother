//! Graph module: Data models and Neo4j storage
//!
//! Defines the graph model for storing AST information
//! and provides the Neo4j client for persistence.

pub mod convert;
pub mod model;
pub mod neo4j;

#[cfg(test)]
mod tests;

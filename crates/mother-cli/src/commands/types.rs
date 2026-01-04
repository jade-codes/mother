//! Command types shared between main and library

use clap::Subcommand;

#[derive(Subcommand)]
pub enum QueryCommands {
    /// Find symbols by name pattern
    Symbols {
        /// Pattern to search for (case-insensitive)
        pattern: String,
    },
    /// List symbols in a file
    File {
        /// File path (or partial path)
        path: String,
    },
    /// Find references to a symbol
    RefsTo {
        /// Symbol name to find references to
        symbol: String,
    },
    /// Find what a symbol references
    RefsFrom {
        /// Symbol name to find outgoing references from
        symbol: String,
    },
    /// List files in the graph
    Files {
        /// Optional pattern to filter files
        pattern: Option<String>,
    },
    /// Show graph statistics
    Stats,
    /// Execute raw Cypher query
    Raw {
        /// Cypher query to execute
        query: String,
    },
}

//! remember-core: Core library for AST graph ingestion via LSP
//!
//! This library uses Language Server Protocol to extract rich semantic
//! information from codebases, including fully resolved types, references,
//! and cross-file analysis, then stores results in a Neo4j graph database.
//!
//! # Supported LSP Servers
//!
//! - **rust-analyzer** - Rust
//! - **pyright** - Python
//! - **typescript-language-server** - TypeScript/JavaScript
//! - **syster-lsp** - SysML/KerML

pub mod graph;
pub mod lsp;
pub mod scanner;
pub mod version;

// Re-export commonly used types
pub use graph::convert::convert_symbols;
pub use graph::model::{Edge, EdgeKind, SymbolKind, SymbolNode};
pub use graph::neo4j::Neo4jClient;
pub use lsp::{LspClient, LspServerManager};
pub use scanner::{DiscoveredFile, Scanner, compute_file_hash};
pub use version::ScanRun;
// test

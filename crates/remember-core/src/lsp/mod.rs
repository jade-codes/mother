//! LSP module: Extract semantic info via Language Server Protocol
//!
//! Uses existing LSP servers to get rich semantic information
//! including resolved types, references, and cross-file analysis.

mod client;
mod manager;
mod types;

pub use client::LspClient;
pub use manager::LspServerManager;
pub use types::{LspReference, LspServerConfig, LspSymbol, LspSymbolKind};

#[cfg(test)]
mod tests;

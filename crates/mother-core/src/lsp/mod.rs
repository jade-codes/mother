//! LSP module: Extract semantic info via Language Server Protocol
//!
//! Uses existing LSP servers to get rich semantic information
//! including resolved types, references, and cross-file analysis.

mod client;
mod convert;
mod manager;
mod types;

pub use client::LspClient;
pub use convert::{
    convert_document_symbol, convert_symbol_information, convert_symbol_kind,
    convert_symbol_response, marked_string_to_string,
};
pub use manager::LspServerManager;
pub use types::{
    LspReference, LspServerConfig, LspSymbol, LspSymbolKind, collect_symbol_positions,
    flatten_symbols,
};

#[cfg(test)]
mod tests;

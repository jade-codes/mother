//! LSP module: Extract semantic info via Language Server Protocol
//!
//! Uses existing LSP servers to get rich semantic information
//! including resolved types, references, and cross-file analysis.

mod client;
mod convert;
mod manager;
mod requests;
mod state;
mod types;

pub use client::LspClient;
pub use convert::{
    convert_document_symbol, convert_symbol_information, convert_symbol_kind,
    convert_symbol_response, marked_string_to_string,
};
pub use manager::{LspServerDefaults, LspServerManager};
pub use types::{
    collect_symbol_positions, flatten_symbols, LspReference, LspServerConfig, LspSymbol,
    LspSymbolKind,
};

#[cfg(test)]
mod tests;

//! LSP types for extracted information

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A symbol extracted via LSP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspSymbol {
    /// Symbol name
    pub name: String,
    /// Symbol kind (function, class, etc.)
    pub kind: LspSymbolKind,
    /// Full detail/signature
    pub detail: Option<String>,
    /// Container name (for flat SymbolInformation format)
    pub container_name: Option<String>,
    /// File containing the symbol
    pub file: PathBuf,
    /// Start line (0-indexed)
    pub start_line: u32,
    /// End line (0-indexed)
    pub end_line: u32,
    /// Start column
    pub start_col: u32,
    /// End column
    pub end_col: u32,
    /// Children symbols (for hierarchical document symbols)
    pub children: Vec<LspSymbol>,
}

/// LSP Symbol kinds (mirrors lsp_types::SymbolKind)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LspSymbolKind {
    File,
    Module,
    Namespace,
    Package,
    Class,
    Method,
    Property,
    Field,
    Constructor,
    Enum,
    Interface,
    Function,
    Variable,
    Constant,
    String,
    Number,
    Boolean,
    Array,
    Object,
    Key,
    Null,
    EnumMember,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

/// A reference extracted via LSP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspReference {
    /// File containing the reference
    pub file: PathBuf,
    /// Line number (0-indexed)
    pub line: u32,
    /// Start column
    pub start_col: u32,
    /// End column
    pub end_col: u32,
}

/// Configuration for an LSP server
#[derive(Debug, Clone)]
pub struct LspServerConfig {
    /// Language this server handles
    pub language: crate::scanner::Language,
    /// Command to start the server
    pub command: String,
    /// Arguments to the command
    pub args: Vec<String>,
    /// Working directory
    pub root_path: PathBuf,
    /// Initialization options (JSON)
    pub init_options: Option<serde_json::Value>,
}

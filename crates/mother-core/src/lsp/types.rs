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

// ============================================================================
// Symbol traversal utilities
// ============================================================================

/// Flatten a tree of LSP symbols into a list (depth-first traversal).
///
/// This maintains the same order as symbols are processed during extraction,
/// which is important for matching LSP symbols to graph nodes.
pub fn flatten_symbols(symbols: &[LspSymbol]) -> Vec<&LspSymbol> {
    let mut result = Vec::new();
    for sym in symbols {
        result.push(sym);
        result.extend(flatten_symbols(&sym.children));
    }
    result
}

/// Collect (start_line, start_col) positions from flattened LSP symbols.
///
/// Useful for hover requests where you need the original LSP positions.
pub fn collect_symbol_positions(symbols: &[LspSymbol]) -> Vec<(u32, u32)> {
    fn flatten_positions(symbols: &[LspSymbol], out: &mut Vec<(u32, u32)>) {
        for sym in symbols {
            out.push((sym.start_line, sym.start_col));
            flatten_positions(&sym.children, out);
        }
    }

    let mut result = Vec::new();
    flatten_positions(symbols, &mut result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_symbol(name: &str, start_line: u32, children: Vec<LspSymbol>) -> LspSymbol {
        LspSymbol {
            name: name.to_string(),
            kind: LspSymbolKind::Function,
            detail: None,
            container_name: None,
            file: PathBuf::new(),
            start_line,
            end_line: start_line + 10,
            start_col: 0,
            end_col: 0,
            children,
        }
    }

    #[test]
    fn test_flatten_symbols_empty() {
        let symbols: Vec<LspSymbol> = vec![];
        let result = flatten_symbols(&symbols);
        assert!(result.is_empty());
    }

    #[test]
    fn test_flatten_symbols_flat() {
        let symbols = vec![make_symbol("a", 1, vec![]), make_symbol("b", 2, vec![])];
        let result = flatten_symbols(&symbols);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "a");
        assert_eq!(result[1].name, "b");
    }

    #[test]
    fn test_flatten_symbols_nested() {
        let child = make_symbol("child", 5, vec![]);
        let parent = make_symbol("parent", 1, vec![child]);
        let symbols = vec![parent];

        let result = flatten_symbols(&symbols);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "parent");
        assert_eq!(result[1].name, "child");
    }

    #[test]
    fn test_collect_symbol_positions() {
        let child = make_symbol("child", 5, vec![]);
        let parent = make_symbol("parent", 1, vec![child]);
        let symbols = vec![parent];

        let positions = collect_symbol_positions(&symbols);
        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0], (1, 0)); // parent
        assert_eq!(positions[1], (5, 0)); // child
    }
}

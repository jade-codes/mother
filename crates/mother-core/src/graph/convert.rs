//! Conversion utilities between LSP types and graph model types

use std::path::Path;
use uuid::Uuid;

use super::model::{SymbolKind, SymbolNode};
use crate::lsp::{LspSymbol, LspSymbolKind};

/// Convert an LSP symbol kind to a graph symbol kind
#[must_use]
pub fn convert_symbol_kind(lsp_kind: LspSymbolKind) -> SymbolKind {
    match lsp_kind {
        LspSymbolKind::Module | LspSymbolKind::Namespace | LspSymbolKind::Package => {
            SymbolKind::Module
        }
        LspSymbolKind::Class => SymbolKind::Class,
        LspSymbolKind::Struct => SymbolKind::Struct,
        LspSymbolKind::Enum => SymbolKind::Enum,
        LspSymbolKind::Interface => SymbolKind::Interface,
        LspSymbolKind::Function | LspSymbolKind::Constructor => SymbolKind::Function,
        LspSymbolKind::Method => SymbolKind::Method,
        LspSymbolKind::Variable => SymbolKind::Variable,
        LspSymbolKind::Constant => SymbolKind::Constant,
        LspSymbolKind::Field | LspSymbolKind::Property => SymbolKind::Field,
        LspSymbolKind::TypeParameter => SymbolKind::TypeAlias,
        LspSymbolKind::EnumMember => SymbolKind::Constant,
        _ => SymbolKind::Variable,
    }
}

/// Convert an LSP symbol to a graph symbol node
#[must_use]
pub fn lsp_symbol_to_node(
    symbol: &LspSymbol,
    file_path: &Path,
    parent_qualified_name: Option<&str>,
) -> SymbolNode {
    // Build qualified name from either:
    // 1. Parent qualified name (for nested DocumentSymbol format)
    // 2. Container name (for flat SymbolInformation format)
    // 3. Just the symbol name if neither is available
    let qualified_name = match parent_qualified_name {
        Some(parent) => format!("{}::{}", parent, symbol.name),
        None => match &symbol.container_name {
            Some(container) if !container.is_empty() => {
                format!("{}::{}", container, symbol.name)
            }
            _ => symbol.name.clone(),
        },
    };

    SymbolNode {
        id: Uuid::new_v4().to_string(),
        name: symbol.name.clone(),
        qualified_name,
        kind: convert_symbol_kind(symbol.kind),
        visibility: None, // LSP doesn't provide this directly
        file_path: file_path.display().to_string(),
        start_line: symbol.start_line + 1, // Convert 0-indexed to 1-indexed
        end_line: symbol.end_line + 1,
        signature: symbol.detail.clone(),
        doc_comment: None, // Would need additional LSP request for hover
    }
}

/// Recursively convert LSP symbols and their children to graph nodes
pub fn flatten_symbols(
    symbol: &LspSymbol,
    file_path: &Path,
    parent_qualified_name: Option<&str>,
) -> Vec<SymbolNode> {
    let mut result = Vec::new();

    let node = lsp_symbol_to_node(symbol, file_path, parent_qualified_name);
    let qualified_name = node.qualified_name.clone();
    result.push(node);

    // Recursively process children
    for child in &symbol.children {
        result.extend(flatten_symbols(child, file_path, Some(&qualified_name)));
    }

    result
}

/// Convert a list of top-level LSP symbols to graph nodes
pub fn convert_symbols(symbols: &[LspSymbol], file_path: &Path) -> Vec<SymbolNode> {
    let mut result = Vec::new();

    for symbol in symbols {
        result.extend(flatten_symbols(symbol, file_path, None));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_convert_symbol_kind() {
        assert_eq!(
            convert_symbol_kind(LspSymbolKind::Function),
            SymbolKind::Function
        );
        assert_eq!(convert_symbol_kind(LspSymbolKind::Class), SymbolKind::Class);
        assert_eq!(
            convert_symbol_kind(LspSymbolKind::Module),
            SymbolKind::Module
        );
    }

    #[test]
    fn test_flatten_symbols_with_children() {
        let child = LspSymbol {
            name: "method".to_string(),
            kind: LspSymbolKind::Method,
            detail: Some("fn method()".to_string()),
            file: PathBuf::new(),
            start_line: 5,
            end_line: 10,
            start_col: 0,
            end_col: 0,
            children: vec![],
            container_name: None,
        };

        let parent = LspSymbol {
            name: "MyClass".to_string(),
            kind: LspSymbolKind::Class,
            detail: None,
            file: PathBuf::new(),
            start_line: 0,
            end_line: 15,
            start_col: 0,
            end_col: 0,
            children: vec![child],
            container_name: None,
        };

        let path = PathBuf::from("/test/file.rs");
        let nodes = flatten_symbols(&parent, &path, None);

        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].name, "MyClass");
        assert_eq!(nodes[0].qualified_name, "MyClass");
        assert_eq!(nodes[1].name, "method");
        assert_eq!(nodes[1].qualified_name, "MyClass::method");
    }
}

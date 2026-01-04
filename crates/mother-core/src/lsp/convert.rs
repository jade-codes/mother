//! LSP type conversion utilities
//!
//! Converts between `async_lsp::lsp_types` and our internal `LspSymbol` types.

use std::path::Path;

use async_lsp::lsp_types::{
    DocumentSymbol, DocumentSymbolResponse, MarkedString, SymbolInformation, SymbolKind,
};

use super::types::{LspSymbol, LspSymbolKind};

/// Convert a `DocumentSymbolResponse` to a list of `LspSymbol`.
pub fn convert_symbol_response(response: Option<DocumentSymbolResponse>) -> Vec<LspSymbol> {
    match response {
        Some(DocumentSymbolResponse::Flat(symbols)) => symbols
            .into_iter()
            .map(|s| convert_symbol_information(&s))
            .collect(),
        Some(DocumentSymbolResponse::Nested(symbols)) => symbols
            .into_iter()
            .map(|s| convert_document_symbol(&s))
            .collect(),
        None => vec![],
    }
}

/// Convert a `DocumentSymbol` (hierarchical format) to our `LspSymbol` type.
pub fn convert_document_symbol(symbol: &DocumentSymbol) -> LspSymbol {
    let children = symbol
        .children
        .as_ref()
        .map(|c| c.iter().map(convert_document_symbol).collect())
        .unwrap_or_default();

    LspSymbol {
        name: symbol.name.clone(),
        kind: convert_symbol_kind(symbol.kind),
        detail: symbol.detail.clone(),
        container_name: None, // Nested format uses explicit children instead
        file: std::path::PathBuf::new(), // DocumentSymbol doesn't include file
        start_line: symbol.range.start.line,
        end_line: symbol.range.end.line,
        start_col: symbol.range.start.character,
        end_col: symbol.range.end.character,
        children,
    }
}

/// Convert a `SymbolInformation` (flat format) to our `LspSymbol` type.
pub fn convert_symbol_information(symbol: &SymbolInformation) -> LspSymbol {
    #[allow(deprecated)]
    let container_name = symbol.container_name.clone();
    LspSymbol {
        name: symbol.name.clone(),
        kind: convert_symbol_kind(symbol.kind),
        detail: None,
        container_name,
        file: Path::new(symbol.location.uri.path()).to_path_buf(),
        start_line: symbol.location.range.start.line,
        end_line: symbol.location.range.end.line,
        start_col: symbol.location.range.start.character,
        end_col: symbol.location.range.end.character,
        children: vec![],
    }
}

/// Convert an LSP `SymbolKind` to our `LspSymbolKind` enum.
pub fn convert_symbol_kind(kind: SymbolKind) -> LspSymbolKind {
    // Use a simple mapping - the SymbolKind values are sequential integers
    // so a match is the clearest approach despite the number of arms
    match kind {
        SymbolKind::FILE => LspSymbolKind::File,
        SymbolKind::MODULE => LspSymbolKind::Module,
        SymbolKind::NAMESPACE => LspSymbolKind::Namespace,
        SymbolKind::PACKAGE => LspSymbolKind::Package,
        SymbolKind::CLASS => LspSymbolKind::Class,
        SymbolKind::METHOD => LspSymbolKind::Method,
        SymbolKind::PROPERTY => LspSymbolKind::Property,
        SymbolKind::FIELD => LspSymbolKind::Field,
        SymbolKind::CONSTRUCTOR => LspSymbolKind::Constructor,
        SymbolKind::ENUM => LspSymbolKind::Enum,
        SymbolKind::INTERFACE => LspSymbolKind::Interface,
        SymbolKind::FUNCTION => LspSymbolKind::Function,
        SymbolKind::VARIABLE => LspSymbolKind::Variable,
        SymbolKind::CONSTANT => LspSymbolKind::Constant,
        SymbolKind::STRING => LspSymbolKind::String,
        SymbolKind::NUMBER => LspSymbolKind::Number,
        SymbolKind::BOOLEAN => LspSymbolKind::Boolean,
        SymbolKind::ARRAY => LspSymbolKind::Array,
        SymbolKind::OBJECT => LspSymbolKind::Object,
        SymbolKind::KEY => LspSymbolKind::Key,
        SymbolKind::NULL => LspSymbolKind::Null,
        SymbolKind::ENUM_MEMBER => LspSymbolKind::EnumMember,
        SymbolKind::STRUCT => LspSymbolKind::Struct,
        SymbolKind::EVENT => LspSymbolKind::Event,
        SymbolKind::OPERATOR => LspSymbolKind::Operator,
        SymbolKind::TYPE_PARAMETER => LspSymbolKind::TypeParameter,
        _ => LspSymbolKind::Variable,
    }
}

/// Convert a `MarkedString` to a plain `String`.
///
/// Used for extracting hover content.
pub fn marked_string_to_string(marked: MarkedString) -> String {
    match marked {
        MarkedString::String(s) => s,
        MarkedString::LanguageString(ls) => ls.value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_lsp::lsp_types::{Location, Position, Range, Url};

    #[test]
    fn test_convert_symbol_kind_all_variants() {
        assert_eq!(convert_symbol_kind(SymbolKind::FILE), LspSymbolKind::File);
        assert_eq!(
            convert_symbol_kind(SymbolKind::MODULE),
            LspSymbolKind::Module
        );
        assert_eq!(convert_symbol_kind(SymbolKind::CLASS), LspSymbolKind::Class);
        assert_eq!(
            convert_symbol_kind(SymbolKind::METHOD),
            LspSymbolKind::Method
        );
        assert_eq!(
            convert_symbol_kind(SymbolKind::FUNCTION),
            LspSymbolKind::Function
        );
        assert_eq!(
            convert_symbol_kind(SymbolKind::VARIABLE),
            LspSymbolKind::Variable
        );
        assert_eq!(
            convert_symbol_kind(SymbolKind::STRUCT),
            LspSymbolKind::Struct
        );
    }

    #[test]
    #[allow(deprecated)]
    fn test_convert_document_symbol() {
        let doc_symbol = DocumentSymbol {
            name: "test_func".to_string(),
            detail: Some("fn test_func()".to_string()),
            kind: SymbolKind::FUNCTION,
            tags: None,
            deprecated: None,
            range: Range::new(Position::new(10, 0), Position::new(20, 1)),
            selection_range: Range::new(Position::new(10, 3), Position::new(10, 12)),
            children: None,
        };

        let result = convert_document_symbol(&doc_symbol);

        assert_eq!(result.name, "test_func");
        assert_eq!(result.kind, LspSymbolKind::Function);
        assert_eq!(result.detail, Some("fn test_func()".to_string()));
        assert_eq!(result.start_line, 10);
        assert_eq!(result.end_line, 20);
        assert!(result.children.is_empty());
    }

    #[test]
    #[allow(deprecated)]
    fn test_convert_document_symbol_with_children() {
        let child = DocumentSymbol {
            name: "inner".to_string(),
            detail: None,
            kind: SymbolKind::VARIABLE,
            tags: None,
            deprecated: None,
            range: Range::new(Position::new(12, 4), Position::new(12, 20)),
            selection_range: Range::new(Position::new(12, 8), Position::new(12, 13)),
            children: None,
        };

        let parent = DocumentSymbol {
            name: "outer".to_string(),
            detail: None,
            kind: SymbolKind::FUNCTION,
            tags: None,
            deprecated: None,
            range: Range::new(Position::new(10, 0), Position::new(20, 1)),
            selection_range: Range::new(Position::new(10, 3), Position::new(10, 8)),
            children: Some(vec![child]),
        };

        let result = convert_document_symbol(&parent);

        assert_eq!(result.name, "outer");
        assert_eq!(result.children.len(), 1);
        assert_eq!(result.children[0].name, "inner");
        assert_eq!(result.children[0].kind, LspSymbolKind::Variable);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_convert_symbol_information() {
        #[allow(deprecated)]
        let sym_info = SymbolInformation {
            name: "MyStruct".to_string(),
            kind: SymbolKind::STRUCT,
            tags: None,
            deprecated: None,
            location: Location {
                uri: Url::parse("file:///test/file.rs").unwrap(),
                range: Range::new(Position::new(5, 0), Position::new(15, 1)),
            },
            container_name: Some("my_module".to_string()),
        };

        let result = convert_symbol_information(&sym_info);

        assert_eq!(result.name, "MyStruct");
        assert_eq!(result.kind, LspSymbolKind::Struct);
        assert_eq!(result.container_name, Some("my_module".to_string()));
        assert_eq!(result.start_line, 5);
        assert_eq!(result.end_line, 15);
    }

    #[test]
    fn test_marked_string_to_string() {
        let plain = MarkedString::String("plain text".to_string());
        assert_eq!(marked_string_to_string(plain), "plain text");

        let lang = MarkedString::LanguageString(async_lsp::lsp_types::LanguageString {
            language: "rust".to_string(),
            value: "fn main() {}".to_string(),
        });
        assert_eq!(marked_string_to_string(lang), "fn main() {}");
    }
}

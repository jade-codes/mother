//! Tests for LSP types

use crate::lsp::types::LspSymbolKind;

#[test]
fn test_symbol_kind_serialization() {
    let kind = LspSymbolKind::Function;
    if let Ok(json) = serde_json::to_string(&kind) {
        assert_eq!(json, "\"function\"");
    }
}

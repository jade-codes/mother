//! Tests for LSP types

use crate::lsp::types::LspSymbolKind;

#[test]
fn test_symbol_kind_serialization() {
    let kind = LspSymbolKind::Function;
    let json = serde_json::to_string(&kind).expect("serialize");
    assert_eq!(json, "\"function\"");
}

//! Tests for LSP types

#![allow(clippy::expect_used)]

use crate::lsp::types::LspSymbolKind;

#[test]
#[allow(clippy::expect_used)]
fn test_symbol_kind_serialization() {
    let kind = LspSymbolKind::Function;
    let json = serde_json::to_string(&kind).expect("serialize");
    assert_eq!(json, "\"function\"");
}

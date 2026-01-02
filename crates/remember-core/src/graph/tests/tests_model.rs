//! Tests for graph model types

use crate::graph::model::{EdgeKind, SymbolKind};

#[test]
fn test_symbol_kind_display() {
    assert_eq!(format!("{}", SymbolKind::Function), "function");
    assert_eq!(format!("{}", SymbolKind::Class), "class");
    assert_eq!(format!("{}", SymbolKind::Module), "module");
    assert_eq!(format!("{}", SymbolKind::TypeAlias), "type_alias");
}

#[test]
fn test_edge_kind_display() {
    assert_eq!(format!("{}", EdgeKind::Calls), "CALLS");
    assert_eq!(format!("{}", EdgeKind::Inherits), "INHERITS");
    assert_eq!(format!("{}", EdgeKind::Implements), "IMPLEMENTS");
    assert_eq!(format!("{}", EdgeKind::DefinedIn), "DEFINED_IN");
}

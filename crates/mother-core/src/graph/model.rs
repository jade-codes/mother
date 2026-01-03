//! Graph model types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Kind of symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Module,
    Class,
    Struct,
    Enum,
    Interface,
    Trait,
    Function,
    Method,
    Variable,
    Constant,
    Field,
    TypeAlias,
    Import,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Module => "module",
            Self::Class => "class",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Interface => "interface",
            Self::Trait => "trait",
            Self::Function => "function",
            Self::Method => "method",
            Self::Variable => "variable",
            Self::Constant => "constant",
            Self::Field => "field",
            Self::TypeAlias => "type_alias",
            Self::Import => "import",
        };
        write!(f, "{s}")
    }
}

/// A symbol node in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolNode {
    /// Unique identifier
    pub id: String,
    /// Symbol name
    pub name: String,
    /// Fully qualified name
    pub qualified_name: String,
    /// Kind of symbol
    pub kind: SymbolKind,
    /// Visibility (pub, private, etc.)
    pub visibility: Option<String>,
    /// Source file path
    pub file_path: String,
    /// Start line (1-indexed)
    pub start_line: u32,
    /// End line (1-indexed)
    pub end_line: u32,
    /// Function/method signature
    pub signature: Option<String>,
    /// Documentation comment
    pub doc_comment: Option<String>,
}

/// Kind of edge/relationship
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EdgeKind {
    Calls,
    References,
    Imports,
    Inherits,
    Implements,
    Contains,
    DefinedIn,
    ScannedIn,
}

impl std::fmt::Display for EdgeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Calls => "CALLS",
            Self::References => "REFERENCES",
            Self::Imports => "IMPORTS",
            Self::Inherits => "INHERITS",
            Self::Implements => "IMPLEMENTS",
            Self::Contains => "CONTAINS",
            Self::DefinedIn => "DEFINED_IN",
            Self::ScannedIn => "SCANNED_IN",
        };
        write!(f, "{s}")
    }
}

/// An edge in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// Source node ID
    pub source_id: String,
    /// Target node ID
    pub target_id: String,
    /// Kind of relationship
    pub kind: EdgeKind,
    /// Line where the relationship is defined
    pub line: Option<u32>,
    /// Column where the relationship is defined
    pub column: Option<u32>,
}

/// A scan run representing a versioned snapshot of a repository scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRun {
    /// Unique identifier for this scan run
    pub id: String,
    /// Path to the repository
    pub repo_path: String,
    /// Git commit SHA (if available)
    pub commit_sha: Option<String>,
    /// Git branch (if available)
    pub branch: Option<String>,
    /// When the scan was performed
    pub scanned_at: DateTime<Utc>,
    /// User-provided version tag
    pub version: Option<String>,
}

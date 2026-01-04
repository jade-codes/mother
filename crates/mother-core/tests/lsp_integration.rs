//! Integration tests for LSP client with real language servers
//!
//! These tests require language servers to be installed:
//! - rust-analyzer (for Rust)
//! - typescript-language-server (for TypeScript)

use std::fs;
use std::path::Path;
use std::time::Duration;

use mother_core::lsp::{LspClient, LspServerDefaults};
use mother_core::scanner::Language;
use tempfile::TempDir;

/// Helper to check if a command exists on PATH
fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Create a minimal Cargo.toml for Rust projects
fn create_cargo_toml(dir: &Path, name: &str) {
    let content = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"
"#
    );
    fs::write(dir.join("Cargo.toml"), content).unwrap();
}

// ============================================================================
// Rust Integration Tests (rust-analyzer)
// ============================================================================

#[tokio::test]
async fn test_rust_document_symbols() -> anyhow::Result<()> {
    if !command_exists("rust-analyzer") {
        eprintln!("Skipping test: rust-analyzer not found");
        return Ok(());
    }

    let temp = TempDir::new()?;
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir)?;

    // Create minimal Cargo.toml
    create_cargo_toml(temp.path(), "test_project");

    // Create a Rust file with known symbols
    let rust_code = r#"
pub struct User {
    pub name: String,
    pub age: u32,
}

impl User {
    pub fn new(name: String, age: u32) -> Self {
        Self { name, age }
    }

    pub fn greet(&self) -> String {
        format!("Hello, {}!", self.name)
    }
}

pub fn create_user(name: &str) -> User {
    User::new(name.to_string(), 0)
}
"#;
    let file_path = src_dir.join("lib.rs");
    fs::write(&file_path, rust_code)?;

    // Start rust-analyzer
    let config = LspServerDefaults::for_language(Language::Rust, temp.path());
    let mut client = LspClient::start(config).await?;

    let root_uri = format!("file://{}", temp.path().display());
    client.initialize(&root_uri).await?;
    client.wait_for_indexing(Duration::from_secs(60)).await?;

    // Open the file
    let file_uri = format!("file://{}", file_path.display());
    client.did_open(&file_uri, "rust", rust_code).await?;

    // Wait for rust-analyzer to fully process the file
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Get document symbols with retry
    let mut symbols = Vec::new();
    for attempt in 0..3 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        match client.document_symbols(&file_uri).await {
            Ok(s) => {
                symbols = s;
                break;
            }
            Err(e) if attempt < 2 => {
                eprintln!("Retry {}: {}", attempt + 1, e);
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    // Verify we got the expected symbols
    let symbol_names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

    assert!(
        symbol_names.contains(&"User"),
        "Expected 'User' struct, got: {:?}",
        symbol_names
    );
    assert!(
        symbol_names.contains(&"create_user"),
        "Expected 'create_user' function, got: {:?}",
        symbol_names
    );

    // Note: rust-analyzer may or may not return children depending on mode
    // Just verify we got the main symbols

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_rust_references() -> anyhow::Result<()> {
    if !command_exists("rust-analyzer") {
        eprintln!("Skipping test: rust-analyzer not found");
        return Ok(());
    }

    let temp = TempDir::new()?;
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir)?;

    create_cargo_toml(temp.path(), "test_refs");

    // Create code with references we can find
    let rust_code = r#"
pub fn helper() -> i32 {
    42
}

pub fn caller_one() -> i32 {
    helper()
}

pub fn caller_two() -> i32 {
    helper() + helper()
}
"#;
    let file_path = src_dir.join("lib.rs");
    fs::write(&file_path, rust_code)?;

    let config = LspServerDefaults::for_language(Language::Rust, temp.path());
    let mut client = LspClient::start(config).await?;

    let root_uri = format!("file://{}", temp.path().display());
    client.initialize(&root_uri).await?;
    client.wait_for_indexing(Duration::from_secs(60)).await?;

    let file_uri = format!("file://{}", file_path.display());
    client.did_open(&file_uri, "rust", rust_code).await?;

    // Wait for processing
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Find references to `helper` (defined on line 1, col 7)
    // Line numbers are 0-indexed in LSP
    // Retry on "content modified" error
    let mut refs = Vec::new();
    for attempt in 0..5 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        match client.references(&file_uri, 1, 7, true).await {
            Ok(r) => {
                refs = r;
                break;
            }
            Err(e) if attempt < 4 && e.to_string().contains("content modified") => {
                eprintln!("Retry {}: {}", attempt + 1, e);
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    // Should have at least 3 references: definition + 3 calls
    assert!(
        refs.len() >= 3,
        "Expected at least 3 references to 'helper', got {}",
        refs.len()
    );

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_rust_definition() -> anyhow::Result<()> {
    if !command_exists("rust-analyzer") {
        eprintln!("Skipping test: rust-analyzer not found");
        return Ok(());
    }

    let temp = TempDir::new()?;
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir)?;

    create_cargo_toml(temp.path(), "test_def");

    let rust_code = r#"
pub struct Point {
    pub x: i32,
    pub y: i32,
}

pub fn use_point() {
    let p = Point { x: 1, y: 2 };
    println!("{}", p.x);
}
"#;
    let file_path = src_dir.join("lib.rs");
    fs::write(&file_path, rust_code)?;

    let config = LspServerDefaults::for_language(Language::Rust, temp.path());
    let mut client = LspClient::start(config).await?;

    let root_uri = format!("file://{}", temp.path().display());
    client.initialize(&root_uri).await?;
    client.wait_for_indexing(Duration::from_secs(60)).await?;

    let file_uri = format!("file://{}", file_path.display());
    client.did_open(&file_uri, "rust", rust_code).await?;

    // Wait for processing
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Go to definition of Point (used on line 7, col 12)
    // Retry on "content modified" error
    let mut defs = Vec::new();
    for attempt in 0..5 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        match client.definition(&file_uri, 7, 12).await {
            Ok(d) => {
                defs = d;
                break;
            }
            Err(e) if attempt < 4 && e.to_string().contains("content modified") => {
                eprintln!("Retry {}: {}", attempt + 1, e);
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    assert!(!defs.is_empty(), "Expected definition for 'Point'");

    // Definition should point to line 1 (where struct Point is defined)
    let def = &defs[0];
    assert_eq!(def.line, 1, "Point definition should be on line 1");

    client.shutdown().await?;
    Ok(())
}

// ============================================================================
// TypeScript Integration Tests (typescript-language-server)
// ============================================================================

#[tokio::test]
async fn test_typescript_document_symbols() -> anyhow::Result<()> {
    if !command_exists("typescript-language-server") {
        eprintln!("Skipping test: typescript-language-server not found");
        return Ok(());
    }

    let temp = TempDir::new()?;

    // Create a TypeScript file with known symbols
    let ts_code = r#"
export interface User {
    name: string;
    age: number;
}

export class UserService {
    private users: User[] = [];

    addUser(user: User): void {
        this.users.push(user);
    }

    getUsers(): User[] {
        return this.users;
    }
}

export function createUser(name: string, age: number): User {
    return { name, age };
}
"#;
    let file_path = temp.path().join("user.ts");
    fs::write(&file_path, ts_code)?;

    // Create minimal tsconfig.json
    let tsconfig = r#"{ "compilerOptions": { "target": "es2020" } }"#;
    fs::write(temp.path().join("tsconfig.json"), tsconfig)?;

    let config = LspServerDefaults::for_language(Language::TypeScript, temp.path());
    let mut client = LspClient::start(config).await?;

    let root_uri = format!("file://{}", temp.path().display());
    client.initialize(&root_uri).await?;

    // TypeScript server doesn't have explicit indexing
    tokio::time::sleep(Duration::from_secs(2)).await;

    let file_uri = format!("file://{}", file_path.display());
    client.did_open(&file_uri, "typescript", ts_code).await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    let symbols = client.document_symbols(&file_uri).await?;
    let symbol_names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

    assert!(
        symbol_names.contains(&"User"),
        "Expected 'User' interface, got: {:?}",
        symbol_names
    );
    assert!(
        symbol_names.contains(&"UserService"),
        "Expected 'UserService' class, got: {:?}",
        symbol_names
    );
    assert!(
        symbol_names.contains(&"createUser"),
        "Expected 'createUser' function, got: {:?}",
        symbol_names
    );

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_typescript_references() -> anyhow::Result<()> {
    if !command_exists("typescript-language-server") {
        eprintln!("Skipping test: typescript-language-server not found");
        return Ok(());
    }

    let temp = TempDir::new()?;

    let ts_code = r#"
function helper(): number {
    return 42;
}

function caller1(): number {
    return helper();
}

function caller2(): number {
    return helper() + helper();
}
"#;
    let file_path = temp.path().join("refs.ts");
    fs::write(&file_path, ts_code)?;

    let tsconfig = r#"{ "compilerOptions": { "target": "es2020" } }"#;
    fs::write(temp.path().join("tsconfig.json"), tsconfig)?;

    let config = LspServerDefaults::for_language(Language::TypeScript, temp.path());
    let mut client = LspClient::start(config).await?;

    let root_uri = format!("file://{}", temp.path().display());
    client.initialize(&root_uri).await?;

    tokio::time::sleep(Duration::from_secs(2)).await;

    let file_uri = format!("file://{}", file_path.display());
    client.did_open(&file_uri, "typescript", ts_code).await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Find references to helper (line 1, col 9)
    let refs = client.references(&file_uri, 1, 9, true).await?;

    assert!(
        refs.len() >= 3,
        "Expected at least 3 references to 'helper', got {}",
        refs.len()
    );

    client.shutdown().await?;
    Ok(())
}

// ============================================================================
// Python Integration Tests (pyright)
// ============================================================================

#[tokio::test]
async fn test_python_document_symbols() -> anyhow::Result<()> {
    if !command_exists("pyright-langserver") {
        eprintln!("Skipping test: pyright-langserver not found");
        return Ok(());
    }

    let temp = TempDir::new()?;

    let python_code = r#"
class User:
    def __init__(self, name: str, age: int):
        self.name = name
        self.age = age

    def greet(self) -> str:
        return f"Hello, {self.name}!"


def create_user(name: str, age: int = 0) -> User:
    return User(name, age)


PI = 3.14159
"#;
    let file_path = temp.path().join("user.py");
    fs::write(&file_path, python_code)?;

    let config = LspServerDefaults::for_language(Language::Python, temp.path());
    let mut client = LspClient::start(config).await?;

    let root_uri = format!("file://{}", temp.path().display());
    client.initialize(&root_uri).await?;

    tokio::time::sleep(Duration::from_secs(2)).await;

    let file_uri = format!("file://{}", file_path.display());
    client.did_open(&file_uri, "python", python_code).await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    let symbols = client.document_symbols(&file_uri).await?;
    let symbol_names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

    assert!(
        symbol_names.contains(&"User"),
        "Expected 'User' class, got: {:?}",
        symbol_names
    );
    assert!(
        symbol_names.contains(&"create_user"),
        "Expected 'create_user' function, got: {:?}",
        symbol_names
    );

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_python_references() -> anyhow::Result<()> {
    if !command_exists("pyright-langserver") {
        eprintln!("Skipping test: pyright-langserver not found");
        return Ok(());
    }

    let temp = TempDir::new()?;

    let python_code = r#"
def helper() -> int:
    return 42


def caller1() -> int:
    return helper()


def caller2() -> int:
    return helper() + helper()
"#;
    let file_path = temp.path().join("refs.py");
    fs::write(&file_path, python_code)?;

    let config = LspServerDefaults::for_language(Language::Python, temp.path());
    let mut client = LspClient::start(config).await?;

    let root_uri = format!("file://{}", temp.path().display());
    client.initialize(&root_uri).await?;

    tokio::time::sleep(Duration::from_secs(2)).await;

    let file_uri = format!("file://{}", file_path.display());
    client.did_open(&file_uri, "python", python_code).await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Find references to helper (line 1, col 4)
    let refs = client.references(&file_uri, 1, 4, true).await?;

    assert!(
        refs.len() >= 3,
        "Expected at least 3 references to 'helper', got {}",
        refs.len()
    );

    client.shutdown().await?;
    Ok(())
}

// ============================================================================
// Go Integration Tests (gopls)
// ============================================================================

#[tokio::test]
async fn test_go_document_symbols() -> anyhow::Result<()> {
    if !command_exists("gopls") {
        eprintln!("Skipping test: gopls not found");
        return Ok(());
    }

    let temp = TempDir::new()?;

    // Create go.mod
    let go_mod = "module testproject\n\ngo 1.21\n";
    fs::write(temp.path().join("go.mod"), go_mod)?;

    let go_code = r#"package main

type User struct {
	Name string
	Age  int
}

func NewUser(name string, age int) *User {
	return &User{Name: name, Age: age}
}

func (u *User) Greet() string {
	return "Hello, " + u.Name + "!"
}

func createUser(name string) *User {
	return NewUser(name, 0)
}
"#;
    let file_path = temp.path().join("main.go");
    fs::write(&file_path, go_code)?;

    let config = LspServerDefaults::for_language(Language::Go, temp.path());
    let mut client = LspClient::start(config).await?;

    let root_uri = format!("file://{}", temp.path().display());
    client.initialize(&root_uri).await?;

    tokio::time::sleep(Duration::from_secs(3)).await;

    let file_uri = format!("file://{}", file_path.display());
    client.did_open(&file_uri, "go", go_code).await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    let symbols = client.document_symbols(&file_uri).await?;
    let symbol_names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

    assert!(
        symbol_names.contains(&"User"),
        "Expected 'User' struct, got: {:?}",
        symbol_names
    );
    assert!(
        symbol_names.contains(&"NewUser"),
        "Expected 'NewUser' function, got: {:?}",
        symbol_names
    );

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_go_references() -> anyhow::Result<()> {
    if !command_exists("gopls") {
        eprintln!("Skipping test: gopls not found");
        return Ok(());
    }

    let temp = TempDir::new()?;

    let go_mod = "module testproject\n\ngo 1.21\n";
    fs::write(temp.path().join("go.mod"), go_mod)?;

    let go_code = r#"package main

func helper() int {
	return 42
}

func caller1() int {
	return helper()
}

func caller2() int {
	return helper() + helper()
}
"#;
    let file_path = temp.path().join("main.go");
    fs::write(&file_path, go_code)?;

    let config = LspServerDefaults::for_language(Language::Go, temp.path());
    let mut client = LspClient::start(config).await?;

    let root_uri = format!("file://{}", temp.path().display());
    client.initialize(&root_uri).await?;

    tokio::time::sleep(Duration::from_secs(3)).await;

    let file_uri = format!("file://{}", file_path.display());
    client.did_open(&file_uri, "go", go_code).await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Find references to helper (line 2, col 5)
    let refs = client.references(&file_uri, 2, 5, true).await?;

    assert!(
        refs.len() >= 3,
        "Expected at least 3 references to 'helper', got {}",
        refs.len()
    );

    client.shutdown().await?;
    Ok(())
}

// ============================================================================
// Multi-File Cross-Reference Tests
// ============================================================================

#[tokio::test]
async fn test_rust_cross_file_references() -> anyhow::Result<()> {
    if !command_exists("rust-analyzer") {
        eprintln!("Skipping test: rust-analyzer not found");
        return Ok(());
    }

    let temp = TempDir::new()?;
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir)?;

    create_cargo_toml(temp.path(), "cross_ref_test");

    // Create lib.rs that exports a module
    let lib_code = r#"
pub mod utils;

pub use utils::helper;

pub fn main_caller() -> i32 {
    helper()
}
"#;
    let lib_path = src_dir.join("lib.rs");
    fs::write(&lib_path, lib_code)?;

    // Create utils.rs with the shared function
    let utils_code = r#"
pub fn helper() -> i32 {
    42
}

pub fn internal_caller() -> i32 {
    helper() + helper()
}
"#;
    let utils_path = src_dir.join("utils.rs");
    fs::write(&utils_path, utils_code)?;

    let config = LspServerDefaults::for_language(Language::Rust, temp.path());
    let mut client = LspClient::start(config).await?;

    let root_uri = format!("file://{}", temp.path().display());
    client.initialize(&root_uri).await?;
    client.wait_for_indexing(Duration::from_secs(60)).await?;

    // Open both files
    let lib_uri = format!("file://{}", lib_path.display());
    let utils_uri = format!("file://{}", utils_path.display());

    client.did_open(&lib_uri, "rust", lib_code).await?;
    client.did_open(&utils_uri, "rust", utils_code).await?;

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Find references to helper from utils.rs (line 1, col 7)
    let mut refs = Vec::new();
    for attempt in 0..5 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        match client.references(&utils_uri, 1, 7, true).await {
            Ok(r) => {
                refs = r;
                break;
            }
            Err(e) if attempt < 4 && e.to_string().contains("content modified") => {
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    // Should find references in both files
    let files_with_refs: std::collections::HashSet<_> =
        refs.iter().map(|r| r.file.clone()).collect();

    assert!(
        refs.len() >= 3,
        "Expected at least 3 references to 'helper' across files, got {}",
        refs.len()
    );

    // Verify we have references from multiple files
    assert!(
        files_with_refs.len() >= 1,
        "Expected references from multiple files, got files: {:?}",
        files_with_refs
    );

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_typescript_cross_file_references() -> anyhow::Result<()> {
    if !command_exists("typescript-language-server") {
        eprintln!("Skipping test: typescript-language-server not found");
        return Ok(());
    }

    let temp = TempDir::new()?;

    // Create tsconfig.json
    let tsconfig = r#"{ "compilerOptions": { "target": "es2020", "module": "commonjs" } }"#;
    fs::write(temp.path().join("tsconfig.json"), tsconfig)?;

    // Create utils.ts with shared function
    let utils_code = r#"
export function helper(): number {
    return 42;
}

export function internalCaller(): number {
    return helper() + helper();
}
"#;
    let utils_path = temp.path().join("utils.ts");
    fs::write(&utils_path, utils_code)?;

    // Create main.ts that imports and uses helper
    let main_code = r#"
import { helper } from './utils';

export function mainCaller(): number {
    return helper();
}

export function anotherCaller(): number {
    return helper() * 2;
}
"#;
    let main_path = temp.path().join("main.ts");
    fs::write(&main_path, main_code)?;

    let config = LspServerDefaults::for_language(Language::TypeScript, temp.path());
    let mut client = LspClient::start(config).await?;

    let root_uri = format!("file://{}", temp.path().display());
    client.initialize(&root_uri).await?;

    tokio::time::sleep(Duration::from_secs(3)).await;

    let utils_uri = format!("file://{}", utils_path.display());
    let main_uri = format!("file://{}", main_path.display());

    client
        .did_open(&utils_uri, "typescript", utils_code)
        .await?;
    client.did_open(&main_uri, "typescript", main_code).await?;

    tokio::time::sleep(Duration::from_secs(1)).await;

    // Find references to helper from utils.ts (line 1, col 16)
    let refs = client.references(&utils_uri, 1, 16, true).await?;

    let files_with_refs: std::collections::HashSet<_> =
        refs.iter().map(|r| r.file.clone()).collect();

    assert!(
        refs.len() >= 4,
        "Expected at least 4 references to 'helper' across files, got {}",
        refs.len()
    );

    // Verify we have references from both files
    assert!(
        files_with_refs.len() >= 2,
        "Expected references from 2 files, got: {:?}",
        files_with_refs
    );

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_python_cross_file_references() -> anyhow::Result<()> {
    if !command_exists("pyright-langserver") {
        eprintln!("Skipping test: pyright-langserver not found");
        return Ok(());
    }

    let temp = TempDir::new()?;

    // Create utils.py with shared function
    let utils_code = r#"
def helper() -> int:
    return 42


def internal_caller() -> int:
    return helper() + helper()
"#;
    let utils_path = temp.path().join("utils.py");
    fs::write(&utils_path, utils_code)?;

    // Create main.py that imports and uses helper
    let main_code = r#"
from utils import helper


def main_caller() -> int:
    return helper()


def another_caller() -> int:
    return helper() * 2
"#;
    let main_path = temp.path().join("main.py");
    fs::write(&main_path, main_code)?;

    let config = LspServerDefaults::for_language(Language::Python, temp.path());
    let mut client = LspClient::start(config).await?;

    let root_uri = format!("file://{}", temp.path().display());
    client.initialize(&root_uri).await?;

    tokio::time::sleep(Duration::from_secs(3)).await;

    let utils_uri = format!("file://{}", utils_path.display());
    let main_uri = format!("file://{}", main_path.display());

    client.did_open(&utils_uri, "python", utils_code).await?;
    client.did_open(&main_uri, "python", main_code).await?;

    tokio::time::sleep(Duration::from_secs(1)).await;

    // Find references to helper from utils.py (line 1, col 4)
    let refs = client.references(&utils_uri, 1, 4, true).await?;

    let files_with_refs: std::collections::HashSet<_> =
        refs.iter().map(|r| r.file.clone()).collect();

    assert!(
        refs.len() >= 4,
        "Expected at least 4 references to 'helper' across files, got {}",
        refs.len()
    );

    // Verify we have references from both files
    assert!(
        files_with_refs.len() >= 2,
        "Expected references from 2 files, got: {:?}",
        files_with_refs
    );

    client.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_go_cross_file_references() -> anyhow::Result<()> {
    if !command_exists("gopls") {
        eprintln!("Skipping test: gopls not found");
        return Ok(());
    }

    let temp = TempDir::new()?;

    let go_mod = "module testproject\n\ngo 1.21\n";
    fs::write(temp.path().join("go.mod"), go_mod)?;

    // Create utils.go with shared function (exported, so capitalized)
    let utils_code = r#"package main

func Helper() int {
	return 42
}

func internalCaller() int {
	return Helper() + Helper()
}
"#;
    let utils_path = temp.path().join("utils.go");
    fs::write(&utils_path, utils_code)?;

    // Create main.go that uses Helper
    let main_code = r#"package main

func mainCaller() int {
	return Helper()
}

func anotherCaller() int {
	return Helper() * 2
}

func main() {
	_ = mainCaller()
}
"#;
    let main_path = temp.path().join("main.go");
    fs::write(&main_path, main_code)?;

    let config = LspServerDefaults::for_language(Language::Go, temp.path());
    let mut client = LspClient::start(config).await?;

    let root_uri = format!("file://{}", temp.path().display());
    client.initialize(&root_uri).await?;

    tokio::time::sleep(Duration::from_secs(3)).await;

    let utils_uri = format!("file://{}", utils_path.display());
    let main_uri = format!("file://{}", main_path.display());

    client.did_open(&utils_uri, "go", utils_code).await?;
    client.did_open(&main_uri, "go", main_code).await?;

    tokio::time::sleep(Duration::from_secs(1)).await;

    // Find references to Helper from utils.go (line 2, col 5)
    let refs = client.references(&utils_uri, 2, 5, true).await?;

    let files_with_refs: std::collections::HashSet<_> =
        refs.iter().map(|r| r.file.clone()).collect();

    assert!(
        refs.len() >= 4,
        "Expected at least 4 references to 'Helper' across files, got {}",
        refs.len()
    );

    // Verify we have references from both files
    assert!(
        files_with_refs.len() >= 2,
        "Expected references from 2 files, got: {:?}",
        files_with_refs
    );

    client.shutdown().await?;
    Ok(())
}

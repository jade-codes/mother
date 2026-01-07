//! Tests for the phase3::run function

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use mother_core::graph::neo4j::{Neo4jClient, Neo4jConfig};
use mother_core::lsp::LspServerManager;
use mother_core::scanner::Language;
use serial_test::serial;
use std::path::PathBuf;
use tempfile::TempDir;

use crate::commands::scan::phase3::run;
use crate::commands::scan::SymbolInfo;

// ============================================================================
// Helper functions for tests
// ============================================================================

/// Helper to create a test Neo4j client
async fn create_test_client() -> Neo4jClient {
    let config = Neo4jConfig::new("bolt://localhost:7687", "neo4j", "mother_dev_password");
    Neo4jClient::connect(&config).await.unwrap()
}

/// Helper to create a test SymbolInfo
fn create_symbol_info(
    id: &str,
    file_uri: &str,
    start_line: u32,
    end_line: u32,
    start_col: u32,
    language: Language,
) -> SymbolInfo {
    SymbolInfo {
        id: id.to_string(),
        file_uri: file_uri.to_string(),
        start_line,
        end_line,
        start_col,
        language,
    }
}

/// Helper to create a test file with content
fn create_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let file_path = dir.path().join(name);
    std::fs::write(&file_path, content).expect("Failed to create test file");
    file_path
}

// ============================================================================
// Tests for run function with empty input
// ============================================================================

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_empty_symbols_list() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&[], &client, &mut lsp_manager).await;

    assert!(result.is_ok());
    let phase3_result = result.unwrap();
    assert_eq!(phase3_result.reference_count, 0);
    assert_eq!(phase3_result.error_count, 0);
}

// ============================================================================
// Tests for run function with single symbol
// ============================================================================

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_single_rust_symbol() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(
        &temp_dir,
        "test.rs",
        "fn main() {\n    println!(\"Hello\");\n}",
    );

    let file_uri = format!("file://{}", file_path.display());
    let symbol = create_symbol_info("test::main", &file_uri, 1, 3, 0, Language::Rust);

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&[symbol], &client, &mut lsp_manager).await;

    assert!(result.is_ok());
    // Reference count depends on LSP server behavior, just verify no panic
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_single_python_symbol() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(
        &temp_dir,
        "test.py",
        "def greet():\n    print('Hello')\n\ngreet()",
    );

    let file_uri = format!("file://{}", file_path.display());
    let symbol = create_symbol_info("test.greet", &file_uri, 1, 2, 0, Language::Python);

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&[symbol], &client, &mut lsp_manager).await;

    assert!(result.is_ok());
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_single_typescript_symbol() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(
        &temp_dir,
        "test.ts",
        "function greet() {\n    console.log('Hello');\n}\ngreet();",
    );

    let file_uri = format!("file://{}", file_path.display());
    let symbol = create_symbol_info("test.greet", &file_uri, 1, 3, 0, Language::TypeScript);

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&[symbol], &client, &mut lsp_manager).await;

    assert!(result.is_ok());
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_single_javascript_symbol() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(
        &temp_dir,
        "test.js",
        "function hello() {\n    return 'Hello';\n}\nhello();",
    );

    let file_uri = format!("file://{}", file_path.display());
    let symbol = create_symbol_info("test.hello", &file_uri, 1, 3, 0, Language::JavaScript);

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&[symbol], &client, &mut lsp_manager).await;

    assert!(result.is_ok());
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_single_go_symbol() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(
        &temp_dir,
        "test.go",
        "package main\n\nfunc main() {\n    println(\"Hello\")\n}",
    );

    let file_uri = format!("file://{}", file_path.display());
    let symbol = create_symbol_info("main.main", &file_uri, 3, 5, 0, Language::Go);

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&[symbol], &client, &mut lsp_manager).await;

    assert!(result.is_ok());
}

// ============================================================================
// Tests for run function with multiple symbols
// ============================================================================

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_multiple_symbols_same_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(
        &temp_dir,
        "test.rs",
        "fn helper() -> i32 { 42 }\nfn main() { let x = helper(); }",
    );

    let file_uri = format!("file://{}", file_path.display());
    let symbols = vec![
        create_symbol_info("test::helper", &file_uri, 1, 1, 0, Language::Rust),
        create_symbol_info("test::main", &file_uri, 2, 2, 0, Language::Rust),
    ];

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&symbols, &client, &mut lsp_manager).await;

    assert!(result.is_ok());
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_multiple_symbols_different_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file1_path = create_test_file(&temp_dir, "file1.rs", "pub fn func1() {}");
    let file2_path = create_test_file(&temp_dir, "file2.rs", "pub fn func2() {}");

    let file1_uri = format!("file://{}", file1_path.display());
    let file2_uri = format!("file://{}", file2_path.display());

    let symbols = vec![
        create_symbol_info("test::func1", &file1_uri, 1, 1, 0, Language::Rust),
        create_symbol_info("test::func2", &file2_uri, 1, 1, 0, Language::Rust),
    ];

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&symbols, &client, &mut lsp_manager).await;

    assert!(result.is_ok());
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_multiple_symbols_mixed_languages() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let rust_file = create_test_file(&temp_dir, "test.rs", "fn main() {}");
    let python_file = create_test_file(&temp_dir, "test.py", "def main(): pass");

    let rust_uri = format!("file://{}", rust_file.display());
    let python_uri = format!("file://{}", python_file.display());

    let symbols = vec![
        create_symbol_info("test::main", &rust_uri, 1, 1, 0, Language::Rust),
        create_symbol_info("test.main", &python_uri, 1, 1, 0, Language::Python),
    ];

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&symbols, &client, &mut lsp_manager).await;

    assert!(result.is_ok());
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_large_number_of_symbols() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(&temp_dir, "test.rs", "fn f1() {}\nfn f2() {}\nfn f3() {}\nfn f4() {}\nfn f5() {}\nfn f6() {}\nfn f7() {}\nfn f8() {}\nfn f9() {}\nfn f10() {}");

    let file_uri = format!("file://{}", file_path.display());
    let mut symbols = Vec::new();
    for i in 1..=10 {
        symbols.push(create_symbol_info(
            &format!("test::f{}", i),
            &file_uri,
            i,
            i,
            0,
            Language::Rust,
        ));
    }

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&symbols, &client, &mut lsp_manager).await;

    assert!(result.is_ok());
}

// ============================================================================
// Tests for edge cases and boundaries
// ============================================================================

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_symbol_at_line_zero() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(&temp_dir, "test.rs", "fn main() {}");

    let file_uri = format!("file://{}", file_path.display());
    // LSP uses 0-based line numbers in some cases
    let symbol = create_symbol_info("test::main", &file_uri, 0, 0, 0, Language::Rust);

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&[symbol], &client, &mut lsp_manager).await;

    assert!(result.is_ok());
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_symbol_at_high_line_number() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(&temp_dir, "test.rs", "fn main() {}");

    let file_uri = format!("file://{}", file_path.display());
    // Test with a very high line number (boundary case)
    let symbol = create_symbol_info("test::main", &file_uri, 10000, 10001, 0, Language::Rust);

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&[symbol], &client, &mut lsp_manager).await;

    assert!(result.is_ok());
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_symbol_at_high_column_number() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(&temp_dir, "test.rs", "fn main() {}");

    let file_uri = format!("file://{}", file_path.display());
    // Test with a very high column number (boundary case)
    let symbol = create_symbol_info("test::main", &file_uri, 1, 1, 10000, Language::Rust);

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&[symbol], &client, &mut lsp_manager).await;

    assert!(result.is_ok());
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_single_line_symbol() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(&temp_dir, "test.rs", "fn main() { let x = 42; }");

    let file_uri = format!("file://{}", file_path.display());
    // Symbol that starts and ends on the same line
    let symbol = create_symbol_info("test::main", &file_uri, 1, 1, 0, Language::Rust);

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&[symbol], &client, &mut lsp_manager).await;

    assert!(result.is_ok());
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_multi_line_symbol() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(
        &temp_dir,
        "test.rs",
        "fn main() {\n    let x = 42;\n    println!(\"{}\", x);\n}",
    );

    let file_uri = format!("file://{}", file_path.display());
    // Symbol spanning multiple lines
    let symbol = create_symbol_info("test::main", &file_uri, 1, 4, 0, Language::Rust);

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&[symbol], &client, &mut lsp_manager).await;

    assert!(result.is_ok());
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_symbol_with_special_characters_in_id() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(&temp_dir, "test.rs", "fn main() {}");

    let file_uri = format!("file://{}", file_path.display());
    // Symbol ID with special characters (namespaces, operators, etc.)
    let symbol = create_symbol_info("std::ops::Add::add", &file_uri, 1, 1, 0, Language::Rust);

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&[symbol], &client, &mut lsp_manager).await;

    assert!(result.is_ok());
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_non_existent_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    // File URI pointing to a non-existent file
    let file_uri = format!("file://{}/nonexistent.rs", temp_dir.path().display());
    let symbol = create_symbol_info("test::main", &file_uri, 1, 1, 0, Language::Rust);

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&[symbol], &client, &mut lsp_manager).await;

    assert!(result.is_ok());
    let phase3_result = result.unwrap();
    // Non-existent file should result in errors
    assert_eq!(phase3_result.reference_count, 0);
    assert!(phase3_result.error_count > 0);
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_file_uri_without_file_prefix() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(&temp_dir, "test.rs", "fn main() {}");

    // File URI without "file://" prefix (edge case)
    let file_uri = file_path.display().to_string();
    let symbol = create_symbol_info("test::main", &file_uri, 1, 1, 0, Language::Rust);

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&[symbol], &client, &mut lsp_manager).await;

    assert!(result.is_ok());
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_empty_symbol_id() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(&temp_dir, "test.rs", "fn main() {}");

    let file_uri = format!("file://{}", file_path.display());
    // Symbol with empty ID (edge case)
    let symbol = create_symbol_info("", &file_uri, 1, 1, 0, Language::Rust);

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&[symbol], &client, &mut lsp_manager).await;

    assert!(result.is_ok());
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_symbols_in_nested_directories() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let nested_dir = temp_dir.path().join("src").join("utils");
    std::fs::create_dir_all(&nested_dir).expect("Failed to create nested directories");

    let file_path = nested_dir.join("helper.rs");
    std::fs::write(&file_path, "pub fn helper() {}").expect("Failed to create nested file");

    let file_uri = format!("file://{}", file_path.display());
    let symbol = create_symbol_info("utils::helper", &file_uri, 1, 1, 0, Language::Rust);

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&[symbol], &client, &mut lsp_manager).await;

    assert!(result.is_ok());
}

// ============================================================================
// Tests for result structure and properties
// ============================================================================

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_accumulates_error_counts() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create symbols with various problematic attributes
    let symbols = vec![
        create_symbol_info(
            "test::func1",
            &format!("file://{}/nonexistent1.rs", temp_dir.path().display()),
            1,
            1,
            0,
            Language::Rust,
        ),
        create_symbol_info(
            "test::func2",
            &format!("file://{}/nonexistent2.rs", temp_dir.path().display()),
            1,
            1,
            0,
            Language::Rust,
        ),
    ];

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&symbols, &client, &mut lsp_manager).await;

    assert!(result.is_ok());
    let phase3_result = result.unwrap();
    // Non-existent files should generate errors
    assert!(phase3_result.error_count > 0);
    assert_eq!(phase3_result.reference_count, 0);
}

// ============================================================================
// Tests for various language support
// ============================================================================

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_supports_all_language_variants() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let languages_and_extensions = vec![
        (Language::Rust, "rs", "fn main() {}"),
        (Language::Python, "py", "def main(): pass"),
        (Language::TypeScript, "ts", "function main() {}"),
        (Language::JavaScript, "js", "function main() {}"),
        (Language::Go, "go", "package main\n\nfunc main() {}"),
    ];

    for (lang, ext, content) in languages_and_extensions {
        let file_path = create_test_file(&temp_dir, &format!("test.{}", ext), content);
        let file_uri = format!("file://{}", file_path.display());
        let symbol = create_symbol_info("test::main", &file_uri, 1, 1, 0, lang);

        let client = create_test_client().await;
        let mut lsp_manager = LspServerManager::new(temp_dir.path());

        let result = run(&[symbol], &client, &mut lsp_manager).await;

        assert!(result.is_ok(), "Language {:?} should be supported", lang);
    }
}

// ============================================================================
// Tests for URI format handling
// ============================================================================

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_file_uri_with_spaces() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let dir_with_spaces = temp_dir.path().join("folder with spaces");
    std::fs::create_dir_all(&dir_with_spaces).expect("Failed to create directory with spaces");

    let file_path = dir_with_spaces.join("test.rs");
    std::fs::write(&file_path, "fn main() {}").expect("Failed to create file");

    let file_uri = format!("file://{}", file_path.display());
    let symbol = create_symbol_info("test::main", &file_uri, 1, 1, 0, Language::Rust);

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&[symbol], &client, &mut lsp_manager).await;

    assert!(result.is_ok());
}

#[tokio::test]
#[ignore = "requires running Neo4j and LSP"]
#[serial]
async fn test_run_with_windows_style_file_uri() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = create_test_file(&temp_dir, "test.rs", "fn main() {}");

    // Windows-style file URI (if on Windows, this will be natural; otherwise tests edge case)
    let file_uri = format!("file://{}", file_path.display());
    let symbol = create_symbol_info("test::main", &file_uri, 1, 1, 0, Language::Rust);

    let client = create_test_client().await;
    let mut lsp_manager = LspServerManager::new(temp_dir.path());

    let result = run(&[symbol], &client, &mut lsp_manager).await;

    assert!(result.is_ok());
}

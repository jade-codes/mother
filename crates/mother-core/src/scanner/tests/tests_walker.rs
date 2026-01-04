//! Tests for file walker

#![allow(clippy::expect_used)]

use crate::scanner::{Language, Scanner};
use std::fs;
use tempfile::TempDir;

#[test]
#[allow(clippy::expect_used)]
fn test_scanner_finds_rust_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src dir");

    fs::write(src_dir.join("main.rs"), "fn main() {}").expect("Failed to write file");
    fs::write(src_dir.join("lib.rs"), "pub mod foo;").expect("Failed to write file");
    fs::write(src_dir.join("README.md"), "# Hello").expect("Failed to write file");

    let scanner = Scanner::new(temp_dir.path());
    let files: Vec<_> = scanner.scan().collect();

    assert_eq!(files.len(), 2);
    assert!(files.iter().all(|f| f.language == Language::Rust));
}

#[test]
#[allow(clippy::expect_used)]
fn test_scanner_with_language_filter() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    fs::write(temp_dir.path().join("main.rs"), "fn main() {}").expect("Failed to write file");
    fs::write(temp_dir.path().join("app.py"), "print('hello')").expect("Failed to write file");

    let scanner = Scanner::new(temp_dir.path()).with_languages(vec![Language::Python]);
    let files: Vec<_> = scanner.scan().collect();

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].language, Language::Python);
}

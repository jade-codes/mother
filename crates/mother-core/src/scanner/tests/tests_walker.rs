//! Tests for file walker

use crate::scanner::{Language, Scanner};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_scanner_finds_rust_files() {
    let temp_dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(_) => return, // Skip test if can't create temp dir
    };
    let src_dir = temp_dir.path().join("src");
    if fs::create_dir(&src_dir).is_err() {
        return; // Skip test if can't create dir
    }

    if fs::write(src_dir.join("main.rs"), "fn main() {}").is_err() {
        return;
    }
    if fs::write(src_dir.join("lib.rs"), "pub mod foo;").is_err() {
        return;
    }
    if fs::write(src_dir.join("README.md"), "# Hello").is_err() {
        return;
    }

    let scanner = Scanner::new(temp_dir.path());
    let files: Vec<_> = scanner.scan().collect();

    assert_eq!(files.len(), 2);
    assert!(files.iter().all(|f| f.language == Language::Rust));
}

#[test]
fn test_scanner_with_language_filter() {
    let temp_dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(_) => return, // Skip test if can't create temp dir
    };

    if fs::write(temp_dir.path().join("main.rs"), "fn main() {}").is_err() {
        return;
    }
    if fs::write(temp_dir.path().join("app.py"), "print('hello')").is_err() {
        return;
    }

    let scanner = Scanner::new(temp_dir.path()).with_languages(vec![Language::Python]);
    let files: Vec<_> = scanner.scan().collect();

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].language, Language::Python);
}

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

#[test]
fn test_scanner_root_returns_correct_path() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let scanner = Scanner::new(temp_dir.path());

    assert_eq!(scanner.root(), temp_dir.path());
}

#[test]
fn test_scanner_root_with_relative_path() {
    let scanner = Scanner::new(".");

    assert_eq!(scanner.root(), std::path::Path::new("."));
}

#[test]
fn test_scanner_root_with_absolute_path() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let abs_path = temp_dir
        .path()
        .canonicalize()
        .expect("Failed to canonicalize path");
    let scanner = Scanner::new(&abs_path);

    assert_eq!(scanner.root(), abs_path.as_path());
}

#[test]
fn test_scanner_root_persists_after_with_languages() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let scanner =
        Scanner::new(temp_dir.path()).with_languages(vec![Language::Rust, Language::Python]);

    assert_eq!(scanner.root(), temp_dir.path());
}

#[test]
fn test_scanner_root_with_string_path() {
    let path_str = "/tmp/test";
    let scanner = Scanner::new(path_str);

    assert_eq!(scanner.root(), std::path::Path::new(path_str));
}

#[test]
fn test_scanner_root_with_path_buf() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path_buf = temp_dir.path().to_path_buf();
    let scanner = Scanner::new(path_buf.clone());

    assert_eq!(scanner.root(), path_buf.as_path());
}

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
    let path_str = "test_path";
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

#[test]
fn test_compute_hash_basic() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("test.rs");
    let content = "fn main() { println!(\"Hello, world!\"); }";
    fs::write(&file_path, content).expect("Failed to write file");

    let discovered_file = crate::scanner::DiscoveredFile {
        path: file_path,
        language: Language::Rust,
    };

    let hash = discovered_file
        .compute_hash()
        .expect("Failed to compute hash");

    // SHA-256 hash should be 64 characters (hex representation)
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_compute_hash_same_content_produces_same_hash() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file1_path = temp_dir.path().join("file1.rs");
    let file2_path = temp_dir.path().join("file2.rs");
    let content = "fn test() { println!(\"Same content\"); }";

    fs::write(&file1_path, content).expect("Failed to write file1");
    fs::write(&file2_path, content).expect("Failed to write file2");

    let file1 = crate::scanner::DiscoveredFile {
        path: file1_path,
        language: Language::Rust,
    };
    let file2 = crate::scanner::DiscoveredFile {
        path: file2_path,
        language: Language::Rust,
    };

    let hash1 = file1.compute_hash().expect("Failed to compute hash1");
    let hash2 = file2.compute_hash().expect("Failed to compute hash2");

    assert_eq!(hash1, hash2);
}

#[test]
fn test_compute_hash_different_content_produces_different_hash() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file1_path = temp_dir.path().join("file1.rs");
    let file2_path = temp_dir.path().join("file2.rs");

    fs::write(&file1_path, "fn main() { println!(\"Hello\"); }").expect("Failed to write file1");
    fs::write(&file2_path, "fn main() { println!(\"World\"); }").expect("Failed to write file2");

    let file1 = crate::scanner::DiscoveredFile {
        path: file1_path,
        language: Language::Rust,
    };
    let file2 = crate::scanner::DiscoveredFile {
        path: file2_path,
        language: Language::Rust,
    };

    let hash1 = file1.compute_hash().expect("Failed to compute hash1");
    let hash2 = file2.compute_hash().expect("Failed to compute hash2");

    assert_ne!(hash1, hash2);
}

#[test]
fn test_compute_hash_empty_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("empty.rs");
    fs::write(&file_path, "").expect("Failed to write empty file");

    let discovered_file = crate::scanner::DiscoveredFile {
        path: file_path,
        language: Language::Rust,
    };

    let hash = discovered_file
        .compute_hash()
        .expect("Failed to compute hash");

    // SHA-256 hash of empty string is known
    assert_eq!(
        hash,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn test_compute_hash_nonexistent_file_returns_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let nonexistent_path = temp_dir.path().join("nonexistent.rs");

    let discovered_file = crate::scanner::DiscoveredFile {
        path: nonexistent_path,
        language: Language::Rust,
    };

    let result = discovered_file.compute_hash();

    assert!(result.is_err());
    assert_eq!(
        result
            .expect_err("Expected error for nonexistent file")
            .kind(),
        std::io::ErrorKind::NotFound
    );
}

#[test]
fn test_compute_hash_large_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("large.rs");
    // Create a large file content (10,000 bytes)
    let large_content = "a".repeat(10_000);
    fs::write(&file_path, &large_content).expect("Failed to write large file");

    let discovered_file = crate::scanner::DiscoveredFile {
        path: file_path,
        language: Language::Rust,
    };

    let hash = discovered_file
        .compute_hash()
        .expect("Failed to compute hash");

    // Should still produce a valid 64-character hex hash
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_compute_hash_binary_content() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("binary.rs");
    // Write binary test data (including null bytes and non-ASCII)
    let test_binary_data: Vec<u8> = vec![0, 1, 2, 255, 127, 128];
    fs::write(&file_path, &test_binary_data).expect("Failed to write binary file");

    let discovered_file = crate::scanner::DiscoveredFile {
        path: file_path,
        language: Language::Rust,
    };

    let hash = discovered_file
        .compute_hash()
        .expect("Failed to compute hash");

    // Should handle binary content correctly
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_compute_hash_with_special_characters() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("special.rs");
    let content = "fn main() {\n    let emoji = \"🦀\";\n    println!(\"Rust {}\", emoji);\n}";
    fs::write(&file_path, content).expect("Failed to write file");

    let discovered_file = crate::scanner::DiscoveredFile {
        path: file_path,
        language: Language::Rust,
    };

    let hash = discovered_file
        .compute_hash()
        .expect("Failed to compute hash");

    // Should handle Unicode correctly
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_compute_hash_with_different_line_endings() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file1_path = temp_dir.path().join("unix.rs");
    let file2_path = temp_dir.path().join("windows.rs");

    // Unix line endings (LF)
    fs::write(&file1_path, "line1\nline2\nline3").expect("Failed to write unix file");
    // Windows line endings (CRLF)
    fs::write(&file2_path, "line1\r\nline2\r\nline3").expect("Failed to write windows file");

    let file1 = crate::scanner::DiscoveredFile {
        path: file1_path,
        language: Language::Rust,
    };
    let file2 = crate::scanner::DiscoveredFile {
        path: file2_path,
        language: Language::Rust,
    };

    let hash1 = file1.compute_hash().expect("Failed to compute hash1");
    let hash2 = file2.compute_hash().expect("Failed to compute hash2");

    // Different line endings should produce different hashes
    assert_ne!(hash1, hash2);
}

#[test]
fn test_compute_hash_deterministic() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("deterministic.rs");
    let content = "fn test() { return 42; }";
    fs::write(&file_path, content).expect("Failed to write file");

    let discovered_file = crate::scanner::DiscoveredFile {
        path: file_path,
        language: Language::Rust,
    };

    // Compute hash multiple times
    let hash1 = discovered_file
        .compute_hash()
        .expect("Failed to compute hash1");
    let hash2 = discovered_file
        .compute_hash()
        .expect("Failed to compute hash2");
    let hash3 = discovered_file
        .compute_hash()
        .expect("Failed to compute hash3");

    // All hashes should be identical
    assert_eq!(hash1, hash2);
    assert_eq!(hash2, hash3);
}

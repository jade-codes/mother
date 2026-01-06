//! Tests for file walker

#![allow(clippy::expect_used)]

use crate::scanner::{DiscoveredFile, Language, Scanner};
use std::fs;
use std::path::PathBuf;
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
#[allow(clippy::expect_used)]
fn test_discovered_file_compute_hash_with_known_content() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("test.rs");

    // Write known content to file
    let content = b"fn main() { println!(\"Hello, world!\"); }";
    fs::write(&file_path, content).expect("Failed to write file");

    let discovered_file = DiscoveredFile {
        path: file_path,
        language: Language::Rust,
    };

    let hash = discovered_file
        .compute_hash()
        .expect("Failed to compute hash");

    // Verify hash is 64 characters (SHA-256 hex string)
    assert_eq!(hash.len(), 64);

    // Verify hash is hexadecimal
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

    // We can't verify exact hash without computing it,
    // but we can verify it's consistent
    let hash2 = discovered_file
        .compute_hash()
        .expect("Failed to compute hash");
    assert_eq!(hash, hash2);
}

#[test]
#[allow(clippy::expect_used)]
fn test_discovered_file_compute_hash_consistency() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("test.rs");

    fs::write(&file_path, b"test content").expect("Failed to write file");

    let discovered_file = DiscoveredFile {
        path: file_path,
        language: Language::Rust,
    };

    // Compute hash multiple times
    let hash1 = discovered_file
        .compute_hash()
        .expect("Failed to compute hash");
    let hash2 = discovered_file
        .compute_hash()
        .expect("Failed to compute hash");
    let hash3 = discovered_file
        .compute_hash()
        .expect("Failed to compute hash");

    // All hashes should be identical
    assert_eq!(hash1, hash2);
    assert_eq!(hash2, hash3);
}

#[test]
#[allow(clippy::expect_used)]
fn test_discovered_file_compute_hash_different_content() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file1_path = temp_dir.path().join("file1.rs");
    let file2_path = temp_dir.path().join("file2.rs");

    fs::write(&file1_path, b"content A").expect("Failed to write file1");
    fs::write(&file2_path, b"content B").expect("Failed to write file2");

    let discovered_file1 = DiscoveredFile {
        path: file1_path,
        language: Language::Rust,
    };

    let discovered_file2 = DiscoveredFile {
        path: file2_path,
        language: Language::Rust,
    };

    let hash1 = discovered_file1
        .compute_hash()
        .expect("Failed to compute hash");
    let hash2 = discovered_file2
        .compute_hash()
        .expect("Failed to compute hash");

    // Different content should produce different hashes
    assert_ne!(hash1, hash2);
}

#[test]
#[allow(clippy::expect_used)]
fn test_discovered_file_compute_hash_empty_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("empty.rs");

    // Create empty file
    fs::write(&file_path, b"").expect("Failed to write file");

    let discovered_file = DiscoveredFile {
        path: file_path,
        language: Language::Rust,
    };

    let hash = discovered_file
        .compute_hash()
        .expect("Failed to compute hash");

    // SHA-256 hash of empty string
    // echo -n '' | sha256sum
    // e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
    assert_eq!(
        hash,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn test_discovered_file_compute_hash_nonexistent_file() {
    let file_path = PathBuf::from("/nonexistent/path/to/file.rs");

    let discovered_file = DiscoveredFile {
        path: file_path,
        language: Language::Rust,
    };

    let result = discovered_file.compute_hash();

    // Should return an error for non-existent file
    assert!(result.is_err());
}

#[test]
#[allow(clippy::expect_used)]
fn test_discovered_file_compute_hash_large_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("large.rs");

    // Create a file with 1MB of data
    let large_content = vec![b'A'; 1024 * 1024];
    fs::write(&file_path, &large_content).expect("Failed to write file");

    let discovered_file = DiscoveredFile {
        path: file_path,
        language: Language::Rust,
    };

    let hash = discovered_file
        .compute_hash()
        .expect("Failed to compute hash");

    // Verify hash format is correct
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
#[allow(clippy::expect_used)]
fn test_discovered_file_compute_hash_binary_content() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("binary.rs");

    // Write binary content (non-UTF8)
    let binary_content: Vec<u8> = vec![0x00, 0xFF, 0xFE, 0xFD, 0x80, 0x81, 0x82];
    fs::write(&file_path, &binary_content).expect("Failed to write file");

    let discovered_file = DiscoveredFile {
        path: file_path,
        language: Language::Rust,
    };

    let hash = discovered_file
        .compute_hash()
        .expect("Failed to compute hash");

    // Should handle binary content without issues
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
#[allow(clippy::expect_used)]
fn test_discovered_file_compute_hash_unicode_content() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("unicode.rs");

    // Write Unicode content
    let unicode_content = "fn main() { println!(\"Hello, 世界! 🦀\"); }";
    fs::write(&file_path, unicode_content).expect("Failed to write file");

    let discovered_file = DiscoveredFile {
        path: file_path,
        language: Language::Rust,
    };

    let hash = discovered_file
        .compute_hash()
        .expect("Failed to compute hash");

    // Should handle Unicode content
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

    // Verify consistency
    let hash2 = discovered_file
        .compute_hash()
        .expect("Failed to compute hash");
    assert_eq!(hash, hash2);
}

#[test]
#[allow(clippy::expect_used)]
fn test_discovered_file_compute_hash_newline_variations() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file1_path = temp_dir.path().join("unix.rs");
    let file2_path = temp_dir.path().join("windows.rs");

    // Unix-style newlines
    fs::write(&file1_path, "line1\nline2\n").expect("Failed to write file1");

    // Windows-style newlines
    fs::write(&file2_path, "line1\r\nline2\r\n").expect("Failed to write file2");

    let discovered_file1 = DiscoveredFile {
        path: file1_path,
        language: Language::Rust,
    };

    let discovered_file2 = DiscoveredFile {
        path: file2_path,
        language: Language::Rust,
    };

    let hash1 = discovered_file1
        .compute_hash()
        .expect("Failed to compute hash");
    let hash2 = discovered_file2
        .compute_hash()
        .expect("Failed to compute hash");

    // Different newline styles should produce different hashes
    assert_ne!(hash1, hash2);
}

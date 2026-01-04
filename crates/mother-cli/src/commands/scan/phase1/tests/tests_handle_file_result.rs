//! Tests for handle_file_result function

use anyhow::anyhow;
use mother_core::scanner::{DiscoveredFile, Language};
use std::path::PathBuf;

use crate::commands::scan::phase1::{handle_file_result, Phase1Result};
use crate::commands::scan::FileToProcess;

// ============================================================================
// Helper functions
// ============================================================================

fn create_test_discovered_file(path: &str, language: Language) -> DiscoveredFile {
    DiscoveredFile {
        path: PathBuf::from(path),
        language,
    }
}

fn create_test_file_to_process(path: &str, language: Language) -> FileToProcess {
    FileToProcess {
        path: PathBuf::from(path),
        file_uri: format!("file://{}", path),
        content_hash: "abc123".to_string(),
        language,
    }
}

fn create_empty_result() -> Phase1Result {
    Phase1Result {
        files_to_process: Vec::new(),
        new_file_count: 0,
        reused_file_count: 0,
        error_count: 0,
    }
}

// ============================================================================
// Tests for Ok(Some) - new file processed successfully
// ============================================================================

#[test]
fn test_handle_file_result_ok_some_increments_new_file_count() {
    let mut result = create_empty_result();
    let file = create_test_discovered_file("/test/file.rs", Language::Rust);
    let file_to_process = create_test_file_to_process("/test/file.rs", Language::Rust);

    handle_file_result(Ok(Some(file_to_process)), &file, &mut result);

    assert_eq!(result.new_file_count, 1);
    assert_eq!(result.reused_file_count, 0);
    assert_eq!(result.error_count, 0);
}

#[test]
fn test_handle_file_result_ok_some_adds_to_files_to_process() {
    let mut result = create_empty_result();
    let file = create_test_discovered_file("/test/file.rs", Language::Rust);
    let file_to_process = create_test_file_to_process("/test/file.rs", Language::Rust);

    handle_file_result(Ok(Some(file_to_process)), &file, &mut result);

    assert_eq!(result.files_to_process.len(), 1);
    assert_eq!(
        result.files_to_process[0].path,
        PathBuf::from("/test/file.rs")
    );
}

#[test]
fn test_handle_file_result_ok_some_multiple_files() {
    let mut result = create_empty_result();

    for i in 0..5 {
        let path = format!("/test/file{}.rs", i);
        let file = create_test_discovered_file(&path, Language::Rust);
        let file_to_process = create_test_file_to_process(&path, Language::Rust);
        handle_file_result(Ok(Some(file_to_process)), &file, &mut result);
    }

    assert_eq!(result.new_file_count, 5);
    assert_eq!(result.files_to_process.len(), 5);
    assert_eq!(result.reused_file_count, 0);
    assert_eq!(result.error_count, 0);
}

#[test]
fn test_handle_file_result_ok_some_preserves_file_metadata() {
    let mut result = create_empty_result();
    let file = create_test_discovered_file("/test/main.py", Language::Python);
    let file_to_process = FileToProcess {
        path: PathBuf::from("/test/main.py"),
        file_uri: "file:///test/main.py".to_string(),
        content_hash: "def456".to_string(),
        language: Language::Python,
    };

    handle_file_result(Ok(Some(file_to_process)), &file, &mut result);

    assert_eq!(result.files_to_process.len(), 1);
    let processed = &result.files_to_process[0];
    assert_eq!(processed.path, PathBuf::from("/test/main.py"));
    assert_eq!(processed.file_uri, "file:///test/main.py");
    assert_eq!(processed.content_hash, "def456");
    assert_eq!(processed.language, Language::Python);
}

#[test]
fn test_handle_file_result_ok_some_different_languages() {
    let mut result = create_empty_result();

    let languages = [
        Language::Rust,
        Language::Python,
        Language::TypeScript,
        Language::JavaScript,
    ];

    for (i, lang) in languages.iter().enumerate() {
        let path = format!("/test/file{}", i);
        let file = create_test_discovered_file(&path, *lang);
        let file_to_process = create_test_file_to_process(&path, *lang);
        handle_file_result(Ok(Some(file_to_process)), &file, &mut result);
    }

    assert_eq!(result.new_file_count, 4);
    assert_eq!(result.files_to_process.len(), 4);
}

// ============================================================================
// Tests for Ok(None) - file reused from cache
// ============================================================================

#[test]
fn test_handle_file_result_ok_none_increments_reused_count() {
    let mut result = create_empty_result();
    let file = create_test_discovered_file("/test/file.rs", Language::Rust);

    handle_file_result(Ok(None), &file, &mut result);

    assert_eq!(result.new_file_count, 0);
    assert_eq!(result.reused_file_count, 1);
    assert_eq!(result.error_count, 0);
}

#[test]
fn test_handle_file_result_ok_none_does_not_add_to_files_to_process() {
    let mut result = create_empty_result();
    let file = create_test_discovered_file("/test/file.rs", Language::Rust);

    handle_file_result(Ok(None), &file, &mut result);

    assert_eq!(result.files_to_process.len(), 0);
}

#[test]
fn test_handle_file_result_ok_none_multiple_reused() {
    let mut result = create_empty_result();

    for i in 0..10 {
        let path = format!("/test/file{}.rs", i);
        let file = create_test_discovered_file(&path, Language::Rust);
        handle_file_result(Ok(None), &file, &mut result);
    }

    assert_eq!(result.reused_file_count, 10);
    assert_eq!(result.new_file_count, 0);
    assert_eq!(result.error_count, 0);
    assert_eq!(result.files_to_process.len(), 0);
}

// ============================================================================
// Tests for Err - error processing file
// ============================================================================

#[test]
fn test_handle_file_result_err_increments_error_count() {
    let mut result = create_empty_result();
    let file = create_test_discovered_file("/test/file.rs", Language::Rust);

    handle_file_result(Err(anyhow!("Test error")), &file, &mut result);

    assert_eq!(result.new_file_count, 0);
    assert_eq!(result.reused_file_count, 0);
    assert_eq!(result.error_count, 1);
}

#[test]
fn test_handle_file_result_err_does_not_add_to_files_to_process() {
    let mut result = create_empty_result();
    let file = create_test_discovered_file("/test/file.rs", Language::Rust);

    handle_file_result(Err(anyhow!("Test error")), &file, &mut result);

    assert_eq!(result.files_to_process.len(), 0);
}

#[test]
fn test_handle_file_result_err_multiple_errors() {
    let mut result = create_empty_result();

    for i in 0..7 {
        let path = format!("/test/file{}.rs", i);
        let file = create_test_discovered_file(&path, Language::Rust);
        handle_file_result(Err(anyhow!("Test error")), &file, &mut result);
    }

    assert_eq!(result.error_count, 7);
    assert_eq!(result.new_file_count, 0);
    assert_eq!(result.reused_file_count, 0);
    assert_eq!(result.files_to_process.len(), 0);
}

#[test]
fn test_handle_file_result_err_different_error_messages() {
    let mut result = create_empty_result();

    let errors = [
        "File not found",
        "Permission denied",
        "Invalid UTF-8",
        "IO error",
    ];

    for (i, error_msg) in errors.iter().enumerate() {
        let path = format!("/test/file{}.rs", i);
        let file = create_test_discovered_file(&path, Language::Rust);
        handle_file_result(Err(anyhow!(*error_msg)), &file, &mut result);
    }

    assert_eq!(result.error_count, 4);
}

// ============================================================================
// Tests for mixed outcomes
// ============================================================================

#[test]
fn test_handle_file_result_mixed_new_and_reused() {
    let mut result = create_empty_result();

    // Add 3 new files
    for i in 0..3 {
        let path = format!("/test/new{}.rs", i);
        let file = create_test_discovered_file(&path, Language::Rust);
        let file_to_process = create_test_file_to_process(&path, Language::Rust);
        handle_file_result(Ok(Some(file_to_process)), &file, &mut result);
    }

    // Add 2 reused files
    for i in 0..2 {
        let path = format!("/test/reused{}.rs", i);
        let file = create_test_discovered_file(&path, Language::Rust);
        handle_file_result(Ok(None), &file, &mut result);
    }

    assert_eq!(result.new_file_count, 3);
    assert_eq!(result.reused_file_count, 2);
    assert_eq!(result.error_count, 0);
    assert_eq!(result.files_to_process.len(), 3);
}

#[test]
fn test_handle_file_result_mixed_new_and_errors() {
    let mut result = create_empty_result();

    // Add 4 new files
    for i in 0..4 {
        let path = format!("/test/new{}.rs", i);
        let file = create_test_discovered_file(&path, Language::Rust);
        let file_to_process = create_test_file_to_process(&path, Language::Rust);
        handle_file_result(Ok(Some(file_to_process)), &file, &mut result);
    }

    // Add 2 errors
    for i in 0..2 {
        let path = format!("/test/error{}.rs", i);
        let file = create_test_discovered_file(&path, Language::Rust);
        handle_file_result(Err(anyhow!("Error")), &file, &mut result);
    }

    assert_eq!(result.new_file_count, 4);
    assert_eq!(result.reused_file_count, 0);
    assert_eq!(result.error_count, 2);
    assert_eq!(result.files_to_process.len(), 4);
}

#[test]
fn test_handle_file_result_mixed_reused_and_errors() {
    let mut result = create_empty_result();

    // Add 5 reused files
    for i in 0..5 {
        let path = format!("/test/reused{}.rs", i);
        let file = create_test_discovered_file(&path, Language::Rust);
        handle_file_result(Ok(None), &file, &mut result);
    }

    // Add 3 errors
    for i in 0..3 {
        let path = format!("/test/error{}.rs", i);
        let file = create_test_discovered_file(&path, Language::Rust);
        handle_file_result(Err(anyhow!("Error")), &file, &mut result);
    }

    assert_eq!(result.new_file_count, 0);
    assert_eq!(result.reused_file_count, 5);
    assert_eq!(result.error_count, 3);
    assert_eq!(result.files_to_process.len(), 0);
}

#[test]
fn test_handle_file_result_mixed_all_outcomes() {
    let mut result = create_empty_result();

    // Add 2 new files
    for i in 0..2 {
        let path = format!("/test/new{}.rs", i);
        let file = create_test_discovered_file(&path, Language::Rust);
        let file_to_process = create_test_file_to_process(&path, Language::Rust);
        handle_file_result(Ok(Some(file_to_process)), &file, &mut result);
    }

    // Add 3 reused files
    for i in 0..3 {
        let path = format!("/test/reused{}.rs", i);
        let file = create_test_discovered_file(&path, Language::Rust);
        handle_file_result(Ok(None), &file, &mut result);
    }

    // Add 1 error
    let error_file = create_test_discovered_file("/test/error.rs", Language::Rust);
    handle_file_result(Err(anyhow!("Error")), &error_file, &mut result);

    assert_eq!(result.new_file_count, 2);
    assert_eq!(result.reused_file_count, 3);
    assert_eq!(result.error_count, 1);
    assert_eq!(result.files_to_process.len(), 2);
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_handle_file_result_empty_path() {
    let mut result = create_empty_result();
    let file = create_test_discovered_file("", Language::Rust);
    let file_to_process = create_test_file_to_process("", Language::Rust);

    handle_file_result(Ok(Some(file_to_process)), &file, &mut result);

    assert_eq!(result.new_file_count, 1);
    assert_eq!(result.files_to_process[0].path, PathBuf::from(""));
}

#[test]
fn test_handle_file_result_long_path() {
    let mut result = create_empty_result();
    let long_path = "/very/long/path/".to_string() + &"directory/".repeat(50) + "file.rs";
    let file = create_test_discovered_file(&long_path, Language::Rust);
    let file_to_process = create_test_file_to_process(&long_path, Language::Rust);

    handle_file_result(Ok(Some(file_to_process)), &file, &mut result);

    assert_eq!(result.new_file_count, 1);
}

#[test]
fn test_handle_file_result_special_characters_in_path() {
    let mut result = create_empty_result();
    let special_path = "/test/file with spaces & special-chars_123.rs";
    let file = create_test_discovered_file(special_path, Language::Rust);
    let file_to_process = create_test_file_to_process(special_path, Language::Rust);

    handle_file_result(Ok(Some(file_to_process)), &file, &mut result);

    assert_eq!(result.new_file_count, 1);
    assert_eq!(
        result.files_to_process[0].path.to_string_lossy(),
        special_path
    );
}

#[test]
fn test_handle_file_result_maintains_order() {
    let mut result = create_empty_result();

    let paths = vec!["/a.rs", "/b.rs", "/c.rs", "/d.rs", "/e.rs"];
    for path in &paths {
        let file = create_test_discovered_file(path, Language::Rust);
        let file_to_process = create_test_file_to_process(path, Language::Rust);
        handle_file_result(Ok(Some(file_to_process)), &file, &mut result);
    }

    assert_eq!(result.files_to_process.len(), 5);
    for (i, path) in paths.iter().enumerate() {
        assert_eq!(result.files_to_process[i].path, PathBuf::from(path));
    }
}

#[test]
fn test_handle_file_result_with_existing_counts() {
    let mut result = Phase1Result {
        files_to_process: Vec::new(),
        new_file_count: 10,
        reused_file_count: 5,
        error_count: 2,
    };

    let file = create_test_discovered_file("/test/file.rs", Language::Rust);
    let file_to_process = create_test_file_to_process("/test/file.rs", Language::Rust);
    handle_file_result(Ok(Some(file_to_process)), &file, &mut result);

    assert_eq!(result.new_file_count, 11);
    assert_eq!(result.reused_file_count, 5);
    assert_eq!(result.error_count, 2);
}

#[test]
fn test_handle_file_result_large_number_of_files() {
    let mut result = create_empty_result();

    for i in 0..1000 {
        let path = format!("/test/file{}.rs", i);
        let file = create_test_discovered_file(&path, Language::Rust);
        let file_to_process = create_test_file_to_process(&path, Language::Rust);
        handle_file_result(Ok(Some(file_to_process)), &file, &mut result);
    }

    assert_eq!(result.new_file_count, 1000);
    assert_eq!(result.files_to_process.len(), 1000);
}

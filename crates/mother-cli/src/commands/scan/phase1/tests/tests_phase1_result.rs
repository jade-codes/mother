//! Tests for Phase1Result struct

use crate::commands::scan::phase1::Phase1Result;

// ============================================================================
// Tests for Phase1Result initialization
// ============================================================================

#[test]
fn test_phase1_result_initialization() {
    let result = Phase1Result {
        files_to_process: Vec::new(),
        new_file_count: 0,
        reused_file_count: 0,
        error_count: 0,
    };

    assert_eq!(result.files_to_process.len(), 0);
    assert_eq!(result.new_file_count, 0);
    assert_eq!(result.reused_file_count, 0);
    assert_eq!(result.error_count, 0);
}

#[test]
fn test_phase1_result_with_values() {
    let result = Phase1Result {
        files_to_process: Vec::new(),
        new_file_count: 5,
        reused_file_count: 3,
        error_count: 2,
    };

    assert_eq!(result.new_file_count, 5);
    assert_eq!(result.reused_file_count, 3);
    assert_eq!(result.error_count, 2);
}

#[test]
fn test_phase1_result_files_to_process_empty() {
    let result = Phase1Result {
        files_to_process: Vec::new(),
        new_file_count: 0,
        reused_file_count: 0,
        error_count: 0,
    };

    assert!(result.files_to_process.is_empty());
}

#[test]
fn test_phase1_result_counts_can_be_zero() {
    let result = Phase1Result {
        files_to_process: Vec::new(),
        new_file_count: 0,
        reused_file_count: 0,
        error_count: 0,
    };

    assert_eq!(result.new_file_count, 0);
    assert_eq!(result.reused_file_count, 0);
    assert_eq!(result.error_count, 0);
}

#[test]
fn test_phase1_result_counts_can_be_large() {
    let result = Phase1Result {
        files_to_process: Vec::new(),
        new_file_count: 1000,
        reused_file_count: 2000,
        error_count: 50,
    };

    assert_eq!(result.new_file_count, 1000);
    assert_eq!(result.reused_file_count, 2000);
    assert_eq!(result.error_count, 50);
}

#[test]
fn test_phase1_result_only_new_files() {
    let result = Phase1Result {
        files_to_process: Vec::new(),
        new_file_count: 10,
        reused_file_count: 0,
        error_count: 0,
    };

    assert_eq!(result.new_file_count, 10);
    assert_eq!(result.reused_file_count, 0);
    assert_eq!(result.error_count, 0);
}

#[test]
fn test_phase1_result_only_reused_files() {
    let result = Phase1Result {
        files_to_process: Vec::new(),
        new_file_count: 0,
        reused_file_count: 15,
        error_count: 0,
    };

    assert_eq!(result.new_file_count, 0);
    assert_eq!(result.reused_file_count, 15);
    assert_eq!(result.error_count, 0);
}

#[test]
fn test_phase1_result_only_errors() {
    let result = Phase1Result {
        files_to_process: Vec::new(),
        new_file_count: 0,
        reused_file_count: 0,
        error_count: 7,
    };

    assert_eq!(result.new_file_count, 0);
    assert_eq!(result.reused_file_count, 0);
    assert_eq!(result.error_count, 7);
}

#[test]
fn test_phase1_result_mixed_counts() {
    let result = Phase1Result {
        files_to_process: Vec::new(),
        new_file_count: 12,
        reused_file_count: 8,
        error_count: 3,
    };

    let total_processed = result.new_file_count + result.reused_file_count + result.error_count;
    assert_eq!(total_processed, 23);
}

// ============================================================================
// Tests for Phase1Result field mutation
// ============================================================================

#[test]
fn test_phase1_result_can_increment_new_file_count() {
    let mut result = Phase1Result {
        files_to_process: Vec::new(),
        new_file_count: 0,
        reused_file_count: 0,
        error_count: 0,
    };

    result.new_file_count += 1;
    assert_eq!(result.new_file_count, 1);

    result.new_file_count += 1;
    assert_eq!(result.new_file_count, 2);
}

#[test]
fn test_phase1_result_can_increment_reused_file_count() {
    let mut result = Phase1Result {
        files_to_process: Vec::new(),
        new_file_count: 0,
        reused_file_count: 0,
        error_count: 0,
    };

    result.reused_file_count += 1;
    assert_eq!(result.reused_file_count, 1);

    result.reused_file_count += 5;
    assert_eq!(result.reused_file_count, 6);
}

#[test]
fn test_phase1_result_can_increment_error_count() {
    let mut result = Phase1Result {
        files_to_process: Vec::new(),
        new_file_count: 0,
        reused_file_count: 0,
        error_count: 0,
    };

    result.error_count += 1;
    assert_eq!(result.error_count, 1);

    result.error_count += 2;
    assert_eq!(result.error_count, 3);
}

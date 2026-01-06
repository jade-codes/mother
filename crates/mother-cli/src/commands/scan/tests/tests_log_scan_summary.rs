//! Comprehensive tests for `log_scan_summary` function
//!
//! Tests the summary logging functionality after a scan completes, including:
//! - Correct log message formatting with no errors
//! - Correct log message formatting with errors
//! - Handling of zero counts
//! - Handling of large values
//! - Edge cases with different error distributions across phases
//! - Boundary conditions

#![allow(clippy::expect_used)] // Tests can use expect for setup

use super::super::{log_scan_summary, Phase1Result, Phase2Result, Phase3Result};

// ============================================================================
// Basic Functionality Tests
// ============================================================================

#[test]
fn test_log_scan_summary_no_errors() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 10,
        reused_file_count: 5,
        error_count: 0,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 100,
        error_count: 0,
    };

    let phase3 = Phase3Result {
        reference_count: 50,
        error_count: 0,
    };

    // Should not panic and should log without error count
    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_with_errors() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 10,
        reused_file_count: 5,
        error_count: 2,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 100,
        error_count: 3,
    };

    let phase3 = Phase3Result {
        reference_count: 50,
        error_count: 1,
    };

    // Should not panic and should include total error count (2+3+1=6)
    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_typical_scan_results() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 42,
        reused_file_count: 18,
        error_count: 0,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 523,
        error_count: 0,
    };

    let phase3 = Phase3Result {
        reference_count: 1247,
        error_count: 0,
    };

    log_scan_summary(&phase1, &phase2, &phase3);
}

// ============================================================================
// Zero Count Tests
// ============================================================================

#[test]
fn test_log_scan_summary_all_zeros() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 0,
        reused_file_count: 0,
        error_count: 0,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 0,
        error_count: 0,
    };

    let phase3 = Phase3Result {
        reference_count: 0,
        error_count: 0,
    };

    // Should handle empty scan results (no files, symbols, or references)
    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_zero_new_files() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 0,
        reused_file_count: 20,
        error_count: 0,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 150,
        error_count: 0,
    };

    let phase3 = Phase3Result {
        reference_count: 75,
        error_count: 0,
    };

    // All files were reused, no new files
    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_zero_reused_files() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 25,
        reused_file_count: 0,
        error_count: 0,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 200,
        error_count: 0,
    };

    let phase3 = Phase3Result {
        reference_count: 100,
        error_count: 0,
    };

    // All files were new, no reused files
    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_zero_symbols() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 10,
        reused_file_count: 5,
        error_count: 0,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 0,
        error_count: 0,
    };

    let phase3 = Phase3Result {
        reference_count: 0,
        error_count: 0,
    };

    // Files processed but no symbols found
    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_zero_references() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 10,
        reused_file_count: 5,
        error_count: 0,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 100,
        error_count: 0,
    };

    let phase3 = Phase3Result {
        reference_count: 0,
        error_count: 0,
    };

    // Symbols found but no references between them
    log_scan_summary(&phase1, &phase2, &phase3);
}

// ============================================================================
// Error Distribution Tests
// ============================================================================

#[test]
fn test_log_scan_summary_only_phase1_errors() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 5,
        reused_file_count: 3,
        error_count: 10,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 20,
        error_count: 0,
    };

    let phase3 = Phase3Result {
        reference_count: 15,
        error_count: 0,
    };

    // Only phase 1 had errors
    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_only_phase2_errors() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 5,
        reused_file_count: 3,
        error_count: 0,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 20,
        error_count: 8,
    };

    let phase3 = Phase3Result {
        reference_count: 15,
        error_count: 0,
    };

    // Only phase 2 had errors
    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_only_phase3_errors() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 5,
        reused_file_count: 3,
        error_count: 0,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 20,
        error_count: 0,
    };

    let phase3 = Phase3Result {
        reference_count: 15,
        error_count: 12,
    };

    // Only phase 3 had errors
    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_errors_in_two_phases() {
    // Test all combinations of two phases having errors

    // Phase 1 and 2 errors
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 10,
        reused_file_count: 5,
        error_count: 3,
    };
    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 50,
        error_count: 2,
    };
    let phase3 = Phase3Result {
        reference_count: 25,
        error_count: 0,
    };
    log_scan_summary(&phase1, &phase2, &phase3);

    // Phase 1 and 3 errors
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 10,
        reused_file_count: 5,
        error_count: 4,
    };
    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 50,
        error_count: 0,
    };
    let phase3 = Phase3Result {
        reference_count: 25,
        error_count: 1,
    };
    log_scan_summary(&phase1, &phase2, &phase3);

    // Phase 2 and 3 errors
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 10,
        reused_file_count: 5,
        error_count: 0,
    };
    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 50,
        error_count: 5,
    };
    let phase3 = Phase3Result {
        reference_count: 25,
        error_count: 3,
    };
    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_all_phases_have_errors() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 10,
        reused_file_count: 5,
        error_count: 2,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 50,
        error_count: 3,
    };

    let phase3 = Phase3Result {
        reference_count: 25,
        error_count: 1,
    };

    // All three phases had errors (total: 2+3+1=6)
    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_single_error() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 100,
        reused_file_count: 50,
        error_count: 1,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 500,
        error_count: 0,
    };

    let phase3 = Phase3Result {
        reference_count: 250,
        error_count: 0,
    };

    // Test singular error count
    log_scan_summary(&phase1, &phase2, &phase3);
}

// ============================================================================
// Large Value Tests
// ============================================================================

#[test]
fn test_log_scan_summary_large_counts() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 10000,
        reused_file_count: 5000,
        error_count: 100,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 50000,
        error_count: 200,
    };

    let phase3 = Phase3Result {
        reference_count: 100000,
        error_count: 50,
    };

    // Should handle large repository scan counts
    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_very_large_counts() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 1_000_000,
        reused_file_count: 500_000,
        error_count: 10_000,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 5_000_000,
        error_count: 20_000,
    };

    let phase3 = Phase3Result {
        reference_count: 10_000_000,
        error_count: 5_000,
    };

    // Should handle extremely large codebase counts
    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_max_usize_safe_values() {
    // Use values that won't overflow when summed for error_count
    let safe_val = usize::MAX / 4;

    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: safe_val,
        reused_file_count: safe_val,
        error_count: safe_val,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: safe_val,
        error_count: safe_val,
    };

    let phase3 = Phase3Result {
        reference_count: safe_val,
        error_count: safe_val,
    };

    // Should handle very large values without overflow
    log_scan_summary(&phase1, &phase2, &phase3);
}

// ============================================================================
// Mixed Scenario Tests
// ============================================================================

#[test]
fn test_log_scan_summary_mixed_success_and_errors() {
    // Test various realistic combinations
    let test_cases = vec![
        (10, 5, 2, 100, 3, 50, 1),      // Balanced scan with some errors
        (100, 50, 0, 1000, 10, 500, 5), // Large scan with errors
        (0, 10, 5, 50, 0, 25, 2),       // All files reused with errors
        (20, 0, 1, 200, 8, 100, 0),     // All files new with errors
        (50, 50, 10, 0, 0, 0, 0),       // Files processed but symbol extraction failed
    ];

    for (new, reused, e1, symbols, e2, refs, e3) in test_cases {
        let phase1 = Phase1Result {
            files_to_process: vec![],
            new_file_count: new,
            reused_file_count: reused,
            error_count: e1,
        };

        let phase2 = Phase2Result {
            symbols: vec![],
            symbol_count: symbols,
            error_count: e2,
        };

        let phase3 = Phase3Result {
            reference_count: refs,
            error_count: e3,
        };

        log_scan_summary(&phase1, &phase2, &phase3);
    }
}

#[test]
fn test_log_scan_summary_high_error_rate() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 10,
        reused_file_count: 5,
        error_count: 100,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 5,
        error_count: 200,
    };

    let phase3 = Phase3Result {
        reference_count: 2,
        error_count: 150,
    };

    // More errors than successful operations
    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_all_errors_no_success() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 0,
        reused_file_count: 0,
        error_count: 50,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 0,
        error_count: 75,
    };

    let phase3 = Phase3Result {
        reference_count: 0,
        error_count: 25,
    };

    // Complete failure scenario
    log_scan_summary(&phase1, &phase2, &phase3);
}

// ============================================================================
// Realistic Scenario Tests
// ============================================================================

#[test]
fn test_log_scan_summary_small_project() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 3,
        reused_file_count: 0,
        error_count: 0,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 15,
        error_count: 0,
    };

    let phase3 = Phase3Result {
        reference_count: 8,
        error_count: 0,
    };

    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_medium_project() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 150,
        reused_file_count: 75,
        error_count: 2,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 1250,
        error_count: 5,
    };

    let phase3 = Phase3Result {
        reference_count: 3420,
        error_count: 1,
    };

    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_large_project() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 2500,
        reused_file_count: 1200,
        error_count: 25,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 45000,
        error_count: 100,
    };

    let phase3 = Phase3Result {
        reference_count: 125000,
        error_count: 50,
    };

    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_incremental_scan() {
    // Scenario: Most files are reused, only a few new ones
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 5,
        reused_file_count: 495,
        error_count: 0,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 150,
        error_count: 0,
    };

    let phase3 = Phase3Result {
        reference_count: 75,
        error_count: 0,
    };

    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_first_scan() {
    // Scenario: First scan, everything is new
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 500,
        reused_file_count: 0,
        error_count: 0,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 8500,
        error_count: 0,
    };

    let phase3 = Phase3Result {
        reference_count: 22000,
        error_count: 0,
    };

    log_scan_summary(&phase1, &phase2, &phase3);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_log_scan_summary_more_symbols_than_expected() {
    // Edge case: Very high symbol-to-file ratio
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 5,
        reused_file_count: 0,
        error_count: 0,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 10000,
        error_count: 0,
    };

    let phase3 = Phase3Result {
        reference_count: 50000,
        error_count: 0,
    };

    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_more_references_than_symbols() {
    // Edge case: Very high reference-to-symbol ratio
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 20,
        reused_file_count: 10,
        error_count: 0,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 100,
        error_count: 0,
    };

    let phase3 = Phase3Result {
        reference_count: 10000,
        error_count: 0,
    };

    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_files_but_no_symbols() {
    // Edge case: Files processed but no symbols extracted (e.g., binary files)
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 50,
        reused_file_count: 25,
        error_count: 0,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 0,
        error_count: 0,
    };

    let phase3 = Phase3Result {
        reference_count: 0,
        error_count: 0,
    };

    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_symbols_but_no_references() {
    // Edge case: Symbols found but isolated (no references between them)
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 20,
        reused_file_count: 10,
        error_count: 0,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 300,
        error_count: 0,
    };

    let phase3 = Phase3Result {
        reference_count: 0,
        error_count: 0,
    };

    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_equal_new_and_reused() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 50,
        reused_file_count: 50,
        error_count: 0,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 500,
        error_count: 0,
    };

    let phase3 = Phase3Result {
        reference_count: 250,
        error_count: 0,
    };

    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_one_of_everything() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 1,
        reused_file_count: 1,
        error_count: 1,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 1,
        error_count: 1,
    };

    let phase3 = Phase3Result {
        reference_count: 1,
        error_count: 1,
    };

    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_maximum_success_minimum_errors() {
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 100000,
        reused_file_count: 50000,
        error_count: 1,
    };

    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 1000000,
        error_count: 0,
    };

    let phase3 = Phase3Result {
        reference_count: 5000000,
        error_count: 0,
    };

    log_scan_summary(&phase1, &phase2, &phase3);
}

#[test]
fn test_log_scan_summary_unbalanced_counts() {
    // Test various unbalanced count scenarios
    
    // Many files, few symbols
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 1000,
        reused_file_count: 500,
        error_count: 0,
    };
    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 10,
        error_count: 0,
    };
    let phase3 = Phase3Result {
        reference_count: 5,
        error_count: 0,
    };
    log_scan_summary(&phase1, &phase2, &phase3);

    // Few files, many symbols
    let phase1 = Phase1Result {
        files_to_process: vec![],
        new_file_count: 2,
        reused_file_count: 1,
        error_count: 0,
    };
    let phase2 = Phase2Result {
        symbols: vec![],
        symbol_count: 5000,
        error_count: 0,
    };
    let phase3 = Phase3Result {
        reference_count: 10000,
        error_count: 0,
    };
    log_scan_summary(&phase1, &phase2, &phase3);
}

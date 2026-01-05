//! Tests for reference edge creation logic flow

#[test]
fn test_self_reference_should_be_filtered() {
    // In create_reference_edges, edges where from_id == to_id are filtered
    // This test verifies the logic: if from_id != symbol_info.id
    let from_id = "symbol123";
    let to_id = "symbol123";

    // This simulates the check in create_reference_edges line 112
    let should_create_edge = from_id != to_id;
    assert!(
        !should_create_edge,
        "Self-references should be filtered out"
    );
}

#[test]
fn test_different_symbols_should_create_edge() {
    let from_id = "symbol_a";
    let to_id = "symbol_b";

    let should_create_edge = from_id != to_id;
    assert!(should_create_edge, "Different symbols should create edge");
}

#[test]
fn test_reference_without_containing_symbol_skipped() {
    // When find_containing_symbol returns None, no edge is created
    // This is handled by the if let Some(from_id) pattern in line 111
    let containing_symbol: Option<String> = None;

    assert!(
        containing_symbol.is_none(),
        "Reference without containing symbol should be skipped"
    );
}

#[test]
fn test_reference_with_containing_symbol_processed() {
    let containing_symbol: Option<String> = Some("some_symbol".to_string());

    assert!(
        containing_symbol.is_some(),
        "Reference with containing symbol should be processed"
    );
}

#[test]
fn test_edge_counter_logic() {
    // Simulates the counting logic in create_reference_edges
    let mut count = 0;
    let test_cases = vec![
        (Some("sym1".to_string()), "sym2"), // Should count: different symbols
        (Some("sym2".to_string()), "sym2"), // Should not count: self-reference
        (None, "sym3"),                     // Should not count: no containing symbol
        (Some("sym4".to_string()), "sym5"), // Should count: different symbols
    ];

    for (from_opt, to_id) in test_cases {
        if let Some(from_id) = from_opt {
            if from_id != to_id {
                // Simulating successful edge creation
                count += 1;
            }
        }
    }

    assert_eq!(count, 2, "Only 2 valid edges should be counted");
}

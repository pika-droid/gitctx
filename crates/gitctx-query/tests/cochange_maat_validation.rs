//! Validation of co-change metrics against Code Maat equations.
//!
//! Adam Tornhill's Code Maat computes logical coupling (co-change).
//! The standard formula for coupling between A and B is:
//!   Support: N(A ∩ B)
//!   Degree (Confidence) A -> B: N(A ∩ B) / N(A)
//!   Degree (Confidence) B -> A: N(A ∩ B) / N(B)
//!
//! Our metrics should exactly match these principles when decay is disabled
//! (i.e. decay weight = 1.0 for all events).

use gitctx_query::cochange::config::CoChangeConfig;
use gitctx_query::cochange::extractor::RawCoChangePair;
use gitctx_query::cochange::metrics::MetricsComputer;

#[test]
fn test_code_maat_formulas_without_decay() {
    // Large half-life effectively disables decay over a short window
    let mut config = CoChangeConfig::with_reference_epoch(1_000_000);
    config.decay_half_life_days = 1_000_000.0;
    config.min_support = 1; // Analyze everything

    let mut pairs = Vec::new();
    
    // Scenario:
    // A touches: 5 times total
    // B touches: 10 times total
    // A and B co-change: 4 times

    // 4 co-change events for A and B
    for i in 0..4 {
        pairs.push(RawCoChangePair {
            entity_a: "A".to_string(),
            entity_b: "B".to_string(),
            commit_hash: format!("cochange_{i}"),
            authored_at: 1_000_000, // same epoch => weight 1.0
        });
    }

    // A touches 1 time alone
    pairs.push(RawCoChangePair {
        entity_a: "A".to_string(),
        entity_b: "Other1".to_string(),
        commit_hash: "a_alone_1".to_string(),
        authored_at: 1_000_000,
    });

    // B touches 6 times alone
    for i in 0..6 {
        pairs.push(RawCoChangePair {
            entity_a: "B".to_string(), // canonically first since B < Other2
            entity_b: "Other2".to_string(),
            commit_hash: format!("b_alone_{i}"),
            authored_at: 1_000_000,
        });
    }

    // Total qualifying commits = 4 (co) + 1 (A alone) + 6 (B alone) = 11
    let metrics = MetricsComputer::compute(&pairs, 11, &config);

    // Find the A-B coupling
    let m = metrics.iter().find(|m| m.entity_a == "A" && m.entity_b == "B").unwrap();

    // Support: N(A ∩ B) = 4
    assert_eq!(m.support, 4);
    assert!((m.weighted_support - 4.0).abs() < 1e-10);

    // Code Maat Degree(A->B) = N(A ∩ B) / N(A) = 4 / 5 = 0.8
    assert!((m.confidence_a_to_b - 0.8).abs() < 1e-10, "Expected P(B|A) = 0.8, got {}", m.confidence_a_to_b);

    // Code Maat Degree(B->A) = N(A ∩ B) / N(B) = 4 / 10 = 0.4
    assert!((m.confidence_b_to_a - 0.4).abs() < 1e-10, "Expected P(A|B) = 0.4, got {}", m.confidence_b_to_a);
}

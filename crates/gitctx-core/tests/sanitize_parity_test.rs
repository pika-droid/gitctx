//! Parity integration test asserting Rust sanitization matches shared fixtures.

use gitctx_core::sanitize::sanitize_chunk_text;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct TestCase {
    id: String,
    raw: String,
    expected: String,
    expected_strips: usize,
}

#[test]
fn test_fixture_cases_count() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_path = root.join("../../tests/fixtures/sanitization_cases.json");
    let content = fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {fixture_path:?}: {e}"));
    let cases: Vec<TestCase> = serde_json::from_str(&content).expect("valid JSON fixture");

    assert!(
        cases.len() >= 20,
        "Must have at least 20 test vectors per D-09, D-10. Found: {}",
        cases.len()
    );
}

#[test]
fn test_all_fixture_cases_parity() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_path = root.join("../../tests/fixtures/sanitization_cases.json");
    let content = fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {fixture_path:?}: {e}"));
    let cases: Vec<TestCase> = serde_json::from_str(&content).expect("valid JSON fixture");

    for case in &cases {
        let result = sanitize_chunk_text(&case.raw);

        assert_eq!(
            result.sanitized_text, case.expected,
            "Case '{}' failed sanitized_text match!\nRaw: {:?}\nExpected: {:?}\nActual: {:?}",
            case.id, case.raw, case.expected, result.sanitized_text
        );
        assert_eq!(
            result.strip_count, case.expected_strips,
            "Case '{}' strip_count mismatch!\nExpected: {}\nActual: {}",
            case.id, case.expected_strips, result.strip_count
        );
        if case.expected_strips > 0 {
            assert!(result.modified, "Case '{}' expected modified = true", case.id);
        }
    }
}

#[test]
fn test_empty_and_clean_inputs() {
    let empty_res = sanitize_chunk_text("");
    assert_eq!(empty_res.sanitized_text, "");
    assert_eq!(empty_res.strip_count, 0);
    assert!(!empty_res.modified);

    let clean_str = "Pure commit message without any trailers or references.";
    let clean_res = sanitize_chunk_text(clean_str);
    assert_eq!(clean_res.sanitized_text, clean_str);
    assert_eq!(clean_res.strip_count, 0);
    assert!(!clean_res.modified);
}

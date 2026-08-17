use gitctx_core::lineage::{murmur3_32, AstNormalizer, MinHashFingerprint};
use gitctx_core::parser::AstExtractor;
use gitctx_core::qualname::QualName;
use std::path::Path;

#[test]
fn test_tracer_end_to_end_python_and_rust_lineage() {
    let extractor = AstExtractor::new();
    let normalizer = AstNormalizer::new();

    // 1. Python sample
    let py_code = r#"
class DataProcessor:
    def process_records(self, items: list, threshold: float) -> int:
        """Process incoming records."""
        count = 0
        for item in items:
            score = item.get_score()
            if score > threshold:
                count += 1
        return count
"#;
    let py_res = extractor
        .parse_blob(Path::new("processor.py"), py_code.as_bytes(), None)
        .expect("parse python");
    assert!(!py_res.symbols.is_empty());

    let py_sym = py_res
        .symbols
        .iter()
        .find(|s| s.name == "process_records")
        .expect("found python function");
    let py_qual = QualName::new(&py_sym.qualname);
    // Standard QualName format: DataProcessor::process_records or module::Class::method
    assert!(py_qual.is_method() || py_qual.leaf() == "process_records");

    let py_body = &py_code[py_sym.start_byte..py_sym.end_byte];
    let py_tokens = normalizer.normalize(py_body);
    let (py_fp1, is_short1) = MinHashFingerprint::from_tokens(&py_tokens);
    assert!(!is_short1, "Normal function should not be gated as short");

    // 2. Rust sample
    let rs_code = r"
impl Engine {
    pub fn process_records(&mut self, items: &[Item], threshold: f64) -> usize {
        let mut count = 0;
        for item in items {
            let score = item.get_score();
            if score > threshold {
                count += 1;
            }
        }
        count
    }
}
";
    let rs_res = extractor
        .parse_blob(Path::new("engine.rs"), rs_code.as_bytes(), None)
        .expect("parse rust");
    assert!(!rs_res.symbols.is_empty());

    let rs_sym = rs_res
        .symbols
        .iter()
        .find(|s| s.name == "process_records")
        .expect("found rust function");
    let rs_qual = QualName::new(&rs_sym.qualname);
    assert_eq!(rs_qual.leaf(), "process_records");

    let rs_body = &rs_code[rs_sym.start_byte..rs_sym.end_byte];
    let rs_tokens = normalizer.normalize(rs_body);
    let (rs_fp, is_short_rs) = MinHashFingerprint::from_tokens(&rs_tokens);
    assert!(!is_short_rs);

    // Verify deterministic self-similarity is 1.0
    let (py_fp2, _) = MinHashFingerprint::from_tokens(&py_tokens);
    assert_eq!(py_fp1, py_fp2);
    assert!((py_fp1.similarity(&py_fp2) - 1.0).abs() < f32::EPSILON);
    assert!((rs_fp.similarity(&rs_fp) - 1.0).abs() < f32::EPSILON);
}

#[test]
fn test_m5_reformatting_whitespace_invariance() {
    let normalizer = AstNormalizer::new();

    let orig = r"
    def calculate_total(prices, tax_rate, discount):
        subtotal = sum(prices)
        tax = subtotal * tax_rate
        total = subtotal + tax - discount
        if total < 0:
            return 0.0
        return total
    ";

    let reformatted = r"
    def calculate_total( prices , tax_rate , discount ) :
        # Calculate subtotal first
        subtotal = sum( prices )

        tax = subtotal * tax_rate
        total = subtotal + tax - discount

        if total < 0 :
            return 0.0

        return total
    ";

    let tok1 = normalizer.normalize(orig);
    let tok2 = normalizer.normalize(reformatted);

    assert_eq!(tok1, tok2);

    let (fp1, is_short1) = MinHashFingerprint::from_tokens(&tok1);
    let (fp2, is_short2) = MinHashFingerprint::from_tokens(&tok2);

    assert!(!is_short1);
    assert!(!is_short2);
    assert_eq!(fp1, fp2);
    assert_eq!(fp1.similarity(&fp2), 1.0);
}

#[test]
fn test_m6_local_variable_rename_invariance() {
    let normalizer = AstNormalizer::new();

    let orig = r"
    fn evaluate_metric(values: &[f64], alpha: f64) -> f64 {
        let initial_sum = values.iter().sum::<f64>();
        let scaled_val = initial_sum * alpha;
        let final_result = scaled_val.sqrt() + 1.0;
        final_result
    }
    ";

    let renamed = r"
    fn evaluate_metric(items: &[f64], weight: f64) -> f64 {
        let raw_total = items.iter().sum::<f64>();
        let weighted = raw_total * weight;
        let output = weighted.sqrt() + 1.0;
        output
    }
    ";

    let tok1 = normalizer.normalize(orig);
    let tok2 = normalizer.normalize(renamed);

    assert_eq!(tok1, tok2);

    let (fp1, _) = MinHashFingerprint::from_tokens(&tok1);
    let (fp2, _) = MinHashFingerprint::from_tokens(&tok2);

    assert_eq!(fp1, fp2);
    assert_eq!(fp1.similarity(&fp2), 1.0);
}

#[test]
fn test_short_function_length_gating() {
    let normalizer = AstNormalizer::new();

    let getter = "def get_id(self): return 1";
    let tokens = normalizer.normalize(getter);

    assert!(tokens.len() < 10);
    let (_fp, is_short) = MinHashFingerprint::from_tokens(&tokens);
    assert!(
        is_short,
        "Trivial getter with < 10 tokens must be gated as short function"
    );
}

#[test]
fn test_murmur3_32_known_vectors() {
    assert_eq!(murmur3_32(b"", 0), 0);
    assert_eq!(murmur3_32(b"hello", 0), 0x248bfa47);
    assert_eq!(murmur3_32(b"hello world", 42), 3926694905);
}

#[test]
fn test_minhash_determinism_and_reproducibility() {
    let normalizer = AstNormalizer::new();
    let code = "fn compute(x: i32, y: i32) -> i32 { let z = x * y + 10; return z * 2; }";
    let tokens = normalizer.normalize(code);

    let (fp1, _) = MinHashFingerprint::from_tokens(&tokens);
    let (fp2, _) = MinHashFingerprint::from_tokens(&tokens);
    let (fp3, _) = MinHashFingerprint::from_tokens(&tokens);

    assert_eq!(fp1, fp2);
    assert_eq!(fp2, fp3);
    assert_eq!(fp1.similarity(&fp2), 1.0);
    assert_eq!(fp2.similarity(&fp3), 1.0);
}

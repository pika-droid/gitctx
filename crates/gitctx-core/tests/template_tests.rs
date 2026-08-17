use gitctx_core::model::{CommitId, FunctionTouch, LineageId, ParseStatus};
use gitctx_core::template::{
    D4TemplateRenderer, FunctionTemplateContext, IdentifierDelta, IdentifierExtractor,
    ModuleTemplateContext,
};

#[test]
fn test_symmetric_identifier_extraction_python() {
    let diff = r"
--- a/calculator.py
+++ b/calculator.py
@@ -1,5 +1,5 @@
-def compute_discount(subtotal, user_role):
-    base_rate = lookup_role_rate(user_role)
-    return subtotal * base_rate
+def compute_discount(subtotal, user_role, coupon_code):
+    base_rate = lookup_role_rate(user_role)
+    discount_multiplier = apply_coupon(coupon_code)
+    return subtotal * base_rate * discount_multiplier
";

    let delta = IdentifierExtractor::extract_from_diff_text(diff);

    // Common tokens (compute_discount, subtotal, user_role, base_rate, lookup_role_rate) cancelled.
    // Keywords (def, return) stripped.
    assert_eq!(
        delta.added_identifiers,
        vec!["apply_coupon", "coupon_code", "discount_multiplier"]
    );
    assert!(delta.removed_identifiers.is_empty());
}

#[test]
fn test_symmetric_identifier_extraction_multi_language() {
    let diff_rust = r"
--- a/src/service.rs
+++ b/src/service.rs
@@ -10,3 +10,3 @@
-    pub async fn process_job(queue: &Queue, job_id: u64) -> Result<Output> {
-        let worker = find_idle_worker(queue).await?;
+    pub async fn process_job(queue: &Queue, job_id: u64, priority: Priority) -> Result<Output> {
+        let worker = find_priority_worker(queue, priority).await?;
";

    let delta = IdentifierExtractor::extract_from_diff_text(diff_rust);
    assert_eq!(
        delta.added_identifiers,
        vec!["Priority", "find_priority_worker", "priority"]
    );
    assert_eq!(delta.removed_identifiers, vec!["find_idle_worker"]);
}

#[test]
fn test_d4_template_function_full() {
    let added = vec!["apply_coupon".to_string(), "coupon_code".to_string()];
    let removed = vec!["legacy_discount".to_string()];
    let ctx = FunctionTemplateContext {
        qualname: "billing::compute_discount",
        path: "src/billing.py",
        author: "Dev Jane",
        date: "2026-08-17",
        raw_message:
            "Add coupon support to discount calculation\n\nCloses: #123\nCo-authored-by: Bob",
        added_identifiers: &added,
        removed_identifiers: &removed,
        signature: Some("def compute_discount(subtotal: float, coupon_code: str) -> float:"),
        docstring: Some("Calculate total discount considering promotional coupon codes."),
    };

    let rendered = D4TemplateRenderer::render_function(&ctx);
    let expected = "\
Function `billing::compute_discount` in `src/billing.py` was modified by Dev Jane on 2026-08-17.
Commit message: Add coupon support to discount calculation
Added: apply_coupon, coupon_code
Removed: legacy_discount
Signature: def compute_discount(subtotal: float, coupon_code: str) -> float:
Surrounding docstring: Calculate total discount considering promotional coupon codes.";

    assert_eq!(rendered, expected);
}

#[test]
fn test_d4_template_empty_section_omission() {
    let ctx = FunctionTemplateContext {
        qualname: "health_check",
        path: "src/main.rs",
        author: "Dev John",
        date: "2026-08-17",
        raw_message: "",
        added_identifiers: &[],
        removed_identifiers: &[],
        signature: None,
        docstring: None,
    };

    let rendered = D4TemplateRenderer::render_function(&ctx);
    assert_eq!(
        rendered,
        "Function `health_check` in `src/main.rs` was modified by Dev John on 2026-08-17."
    );
}

#[test]
fn test_d4_template_module_touch() {
    let symbols = vec!["API_BASE_URL".to_string(), "MAX_RETRIES".to_string()];
    let ctx = ModuleTemplateContext {
        path: "src/constants.ts",
        author: "Alex",
        date: "2026-08-17",
        raw_message: "Configure retry limits and api endpoint\n\nResolves #555",
        modified_symbols: &symbols,
    };

    let rendered = D4TemplateRenderer::render_module(&ctx);
    let expected = "\
Module `src/constants.ts` was modified by Alex on 2026-08-17.
Commit message: Configure retry limits and api endpoint
Modified top-level symbols: API_BASE_URL, MAX_RETRIES";

    assert_eq!(rendered, expected);
}

#[test]
fn test_d4_template_touch_router() {
    // 1. Function touch
    let touch_fn = FunctionTouch {
        commit_id: CommitId::new("1111222233334444"),
        lineage_id: LineageId::new(42),
        path: "crates/core/src/lib.rs".to_string(),
        qualname: "initialize".to_string(),
        signature: "pub fn initialize()".to_string(),
        docstring: Some("Initialize engine state.".to_string()),
        start_line: 10,
        end_line: 25,
        parse_status: ParseStatus::Ok,
    };
    let delta = IdentifierDelta {
        added_identifiers: vec!["init_config".to_string()],
        removed_identifiers: vec![],
    };
    let rendered_fn = D4TemplateRenderer::render_touch(
        &touch_fn,
        "Engineer",
        "2026-08-17",
        "Setup initialization flow",
        &delta,
    );
    assert!(
        rendered_fn.starts_with("Function `initialize` in `crates/core/src/lib.rs` was modified")
    );
    assert!(rendered_fn.contains("Added: init_config"));
    assert!(!rendered_fn.contains("Removed:"));

    // 2. Module touch
    let touch_mod = FunctionTouch {
        commit_id: CommitId::new("1111222233334444"),
        lineage_id: LineageId::new(0),
        path: "crates/core/src/lib.rs".to_string(),
        qualname: "<module>".to_string(),
        signature: String::new(),
        docstring: None,
        start_line: 1,
        end_line: 10,
        parse_status: ParseStatus::Ok,
    };
    let rendered_mod = D4TemplateRenderer::render_touch(
        &touch_mod,
        "Engineer",
        "2026-08-17",
        "Update module exports",
        &delta,
    );
    assert!(rendered_mod.starts_with("Module `crates/core/src/lib.rs` was modified"));
    assert!(rendered_mod.contains("Modified top-level symbols: init_config"));

    // 3. Degraded file-level touch
    let touch_degraded = FunctionTouch {
        commit_id: CommitId::new("1111222233334444"),
        lineage_id: LineageId::new(0),
        path: "broken.rs".to_string(),
        qualname: "<file_level>".to_string(),
        signature: String::new(),
        docstring: None,
        start_line: 1,
        end_line: 50,
        parse_status: ParseStatus::Degraded,
    };
    let rendered_degraded = D4TemplateRenderer::render_touch(
        &touch_degraded,
        "Engineer",
        "2026-08-17",
        "Fix syntax error",
        &delta,
    );
    assert!(rendered_degraded.starts_with("File `broken.rs` was modified"));
    assert!(rendered_degraded.contains("Added: init_config"));
}

#[test]
fn test_secret_redaction_inside_template() {
    let added = vec!["auth_header".to_string()];
    let ctx = FunctionTemplateContext {
        qualname: "connect_service",
        path: "service.go",
        author: "Dev",
        date: "2026-08-17",
        raw_message: "Use API key sk-proj-1234567890abcdefghijklmnopqrstuv for connection",
        added_identifiers: &added,
        removed_identifiers: &[],
        signature: Some("func connectService(token: \"ghp_abcdefghijklmnopqrstuvwxyz1234567890\")"),
        docstring: Some("Connects using secret wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY."),
    };

    let rendered = D4TemplateRenderer::render_function(&ctx);
    assert!(rendered.contains("[REDACTED_SECRET: openai_api_key]"));
    assert!(rendered.contains("[REDACTED_SECRET: github_token]"));
    assert!(rendered.contains("[REDACTED_SECRET: high_entropy_string]"));
    assert!(!rendered.contains("sk-proj-"));
    assert!(!rendered.contains("ghp_"));
    assert!(!rendered.contains("wJalrXUtnFEMI"));
}

#[test]
fn test_template_determinism() {
    let added = vec!["gamma".to_string(), "alpha".to_string(), "beta".to_string()];
    let removed = vec!["delta".to_string()];
    let ctx = FunctionTemplateContext {
        qualname: "calculate_metrics",
        path: "metrics.rs",
        author: "Engineer",
        date: "2026-08-17",
        raw_message: "Compute metrics with gamma factor (#999)",
        added_identifiers: &added,
        removed_identifiers: &removed,
        signature: Some("pub fn calculate_metrics()"),
        docstring: Some("Computes metrics."),
    };

    let first = D4TemplateRenderer::render_function(&ctx);
    for _ in 0..100 {
        let repeated = D4TemplateRenderer::render_function(&ctx);
        assert_eq!(
            first, repeated,
            "D4 template output must be strictly deterministic"
        );
    }
}

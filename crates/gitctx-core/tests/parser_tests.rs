use git2::Oid;
use gitctx_core::model::ParseStatus;
use gitctx_core::parser::{AstExtractor, LanguageId, LanguageRegistry};
use std::path::Path;

#[test]
fn test_python_symbol_extraction() {
    let extractor = AstExtractor::new();
    let code = r#"
"""Module docstring."""

def standalone(x: int) -> str:
    """Helper function."""
    return str(x)

class Calculator:
    """Calculates stuff."""

    def add(self, a: int, b: int) -> int:
        """Add two numbers."""
        return a + b
"#;
    let res = extractor
        .parse_blob(Path::new("calc.py"), code.as_bytes(), None)
        .expect("parse python");

    assert_eq!(res.status, ParseStatus::Ok);
    assert_eq!(res.symbols.len(), 2);

    let standalone = &res.symbols[0];
    assert_eq!(standalone.name, "standalone");
    assert_eq!(standalone.qualname, "standalone");
    assert_eq!(standalone.signature, "def standalone(x: int) -> str");
    assert_eq!(standalone.docstring.as_deref(), Some("Helper function."));
    assert!(!standalone.is_method);

    let add = &res.symbols[1];
    assert_eq!(add.name, "add");
    assert_eq!(add.qualname, "Calculator::add");
    assert_eq!(add.signature, "def add(self, a: int, b: int) -> int");
    assert_eq!(add.docstring.as_deref(), Some("Add two numbers."));
    assert!(add.is_method);
}

#[test]
fn test_rust_symbol_extraction() {
    let extractor = AstExtractor::new();
    let code = r#"
/// Standalone function documentation.
pub fn calculate_hash(data: &[u8]) -> u64 {
    42
}

struct Engine;

impl Engine {
    /// Engine startup.
    pub fn start(&mut self) {
        println!("engine started");
    }
}
"#;
    let res = extractor
        .parse_blob(Path::new("engine.rs"), code.as_bytes(), None)
        .expect("parse rust");

    assert_eq!(res.status, ParseStatus::Ok);
    assert_eq!(res.symbols.len(), 2);

    let hash_fn = &res.symbols[0];
    assert_eq!(hash_fn.name, "calculate_hash");
    assert_eq!(hash_fn.qualname, "calculate_hash");
    assert_eq!(
        hash_fn.signature,
        "pub fn calculate_hash(data: &[u8]) -> u64"
    );
    assert_eq!(
        hash_fn.docstring.as_deref(),
        Some("Standalone function documentation.")
    );

    let start_fn = &res.symbols[1];
    assert_eq!(start_fn.name, "start");
    assert_eq!(start_fn.qualname, "Engine::start");
    assert_eq!(start_fn.signature, "pub fn start(&mut self)");
    assert_eq!(start_fn.docstring.as_deref(), Some("Engine startup."));
}

#[test]
fn test_all_8_languages_smoke_test() {
    let extractor = AstExtractor::new();

    let cases = vec![
        (
            "index.js",
            "function runServer(port) { return port; }",
            "runServer",
        ),
        (
            "app.ts",
            "function processRequest<T>(req: T): boolean { return true; }",
            "processRequest",
        ),
        (
            "main.go",
            "package main\nfunc StartService(cfg Config) error { return nil }",
            "StartService",
        ),
        (
            "driver.c",
            "int init_driver(void) { return 0; }",
            "init_driver",
        ),
        (
            "renderer.cpp",
            "namespace gfx { void render_frame() {} }",
            "gfx::render_frame",
        ),
        (
            "Service.java",
            "public class Service { public void executeTask() {} }",
            "Service::executeTask",
        ),
        (
            "index.php",
            "<?php function handleRequest($req) { return 1; }",
            "handleRequest",
        ),
    ];

    for (filename, code, expected_qualname) in cases {
        let res = extractor
            .parse_blob(Path::new(filename), code.as_bytes(), None)
            .unwrap_or_else(|_| panic!("Failed to parse {filename}"));
        assert_eq!(res.status, ParseStatus::Ok, "Expected Ok for {filename}");
        assert!(
            res.symbols.iter().any(|s| s.qualname == expected_qualname),
            "Expected {expected_qualname} in {filename} symbols: {:?}",
            res.symbols
        );
    }
}

#[test]
fn test_shebang_language_detection() {
    let registry = LanguageRegistry::new();

    let python_script = "#!/usr/bin/env python3\nprint('hello')";
    let node_script = "#!/usr/bin/env node\nconsole.log('hello');";
    let php_script = "#!/usr/bin/php\n<?php echo 1;";

    let py_behavior = registry.detect(Path::new("bin/task"), Some(python_script.as_bytes()));
    assert_eq!(
        py_behavior.map(|b| b.language_id()),
        Some(LanguageId::Python)
    );

    let js_behavior = registry.detect(Path::new("bin/serve"), Some(node_script.as_bytes()));
    assert_eq!(
        js_behavior.map(|b| b.language_id()),
        Some(LanguageId::JavaScript)
    );

    let php_behavior = registry.detect(Path::new("bin/cron"), Some(php_script.as_bytes()));
    assert_eq!(php_behavior.map(|b| b.language_id()), Some(LanguageId::Php));
}

#[test]
fn test_blob_oid_lru_cache() {
    let extractor = AstExtractor::new();
    let dummy_oid = Oid::from_str("1111222233334444555566667777888899990000").unwrap();
    let code = "def cached_func():\n    return 42\n";

    // 1. First parse: not in cache -> parses and inserts
    let res1 = extractor
        .parse_blob(Path::new("script.py"), code.as_bytes(), Some(dummy_oid))
        .expect("parse 1");
    assert_eq!(res1.symbols.len(), 1);
    assert_eq!(extractor.cache().len(), 1);

    // 2. Second parse: cache hit
    let res2 = extractor
        .parse_blob(Path::new("script.py"), code.as_bytes(), Some(dummy_oid))
        .expect("parse 2");
    assert_eq!(res2.symbols.len(), 1);
    assert_eq!(res2.symbols[0].name, "cached_func");

    let (ok, _degraded, _failed) = extractor.telemetry().snapshot();
    assert!(ok >= 2);
}

#[test]
fn test_graceful_degradation_on_syntax_error() {
    let extractor = AstExtractor::new();
    // Python code with a valid function followed by a broken syntax error block
    let broken_code = r#"
def working_fn():
    """This function is intact."""
    return 100

def broken_syntax(
    # missing closing paren, colon, and body
class Invalid :::
"#;
    let res = extractor
        .parse_blob(Path::new("broken.py"), broken_code.as_bytes(), None)
        .expect("parse broken python");

    assert_eq!(res.status, ParseStatus::Degraded);
    assert!(!res.error_byte_ranges.is_empty());

    // Clean function extracted
    assert!(res.symbols.iter().any(|s| s.name == "working_fn"));

    // Degraded <file_level> fallback chunk created
    assert!(res.symbols.iter().any(|s| s.qualname == "<file_level>"));

    let (_ok, degraded, _failed) = extractor.telemetry().snapshot();
    assert_eq!(degraded, 1);
}

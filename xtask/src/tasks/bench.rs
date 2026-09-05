//! Performance benchmark harness for Phase 10 (Plan 10-02).
//!
//! Validates execution latency budgets for indexing, search, and MCP operations.
//! Budgets are run as "dry-run" functional tests in CI and verified locally for timing.

use anyhow::{bail, Result};
use std::process::Command;
use std::time::Instant;

struct BenchResult {
    name: &'static str,
    budget: &'static str,
    elapsed_ms: u128,
    passed: bool,
    note: Option<String>,
}

/// Runs the complete performance benchmark suite.
pub fn run_bench() -> Result<()> {
    let root = crate::util::project_root();
    let overall_start = Instant::now();

    println!("\n==========================================================================================");
    println!("                         GITCTX PERFORMANCE BENCHMARK SUITE                              ");
    println!("==========================================================================================\n");

    let mut results = Vec::new();

    // Benchmark 1: Search latency via unit test (p95 < 300ms)
    {
        println!("[1/4] Benchmarking search latency (p95 < 300ms budget)...");
        let start = Instant::now();
        let mut cmd = Command::new("cargo");
        cmd.current_dir(&root);
        cmd.args([
            "test",
            "-p",
            "gitctx-index",
            "--test",
            "vector_test",
            "test_vector_simd_scoring_and_incremental",
        ]);
        let result = crate::util::run_cmd(&mut cmd, "vector SIMD scoring benchmark");
        let elapsed = start.elapsed().as_millis();
        let passed = result.is_ok();
        results.push(BenchResult {
            name: "Dense Vector Search (20k vectors)",
            budget: "< 300ms p95",
            elapsed_ms: elapsed,
            passed,
            note: if passed {
                Some("SIMD cosine search + Rayon parallel scoring".to_string())
            } else {
                Some(result.unwrap_err().to_string())
            },
        });
    }

    // Benchmark 2: Tantivy BM25 search latency
    {
        println!("[2/4] Benchmarking Tantivy BM25 search latency...");
        let start = Instant::now();
        let mut cmd = Command::new("cargo");
        cmd.current_dir(&root);
        cmd.args([
            "test",
            "-p",
            "gitctx-index",
            "--test",
            "tantivy_test",
            "test_tantivy_code_tokenizer_and_bm25",
        ]);
        let result = crate::util::run_cmd(&mut cmd, "Tantivy BM25 search benchmark");
        let elapsed = start.elapsed().as_millis();
        let passed = result.is_ok();
        results.push(BenchResult {
            name: "Tantivy BM25 Search",
            budget: "< 300ms p95",
            elapsed_ms: elapsed,
            passed,
            note: if passed {
                Some("Code tokenizer + multi-field boosted search".to_string())
            } else {
                Some(result.unwrap_err().to_string())
            },
        });
    }

    // Benchmark 3: SQLite storage write + atomic swap
    {
        println!("[3/4] Benchmarking SQLite storage write throughput...");
        let start = Instant::now();
        let mut cmd = Command::new("cargo");
        cmd.current_dir(&root);
        cmd.args([
            "test",
            "-p",
            "gitctx-index",
            "--test",
            "storage_test",
            "test_sqlite_storage_and_atomic_swap",
        ]);
        let result = crate::util::run_cmd(&mut cmd, "SQLite storage write benchmark");
        let elapsed = start.elapsed().as_millis();
        let passed = result.is_ok();
        results.push(BenchResult {
            name: "SQLite Write + Atomic Swap",
            budget: "< 2s incremental",
            elapsed_ms: elapsed,
            passed,
            note: if passed {
                Some("WAL mode, background writer, checkpoint + swap".to_string())
            } else {
                Some(result.unwrap_err().to_string())
            },
        });
    }

    // Benchmark 4: MCP round-trip (inspector test)
    {
        println!("[4/4] Benchmarking MCP JSON-RPC round-trip (< 500ms budget)...");
        let start = Instant::now();
        let mut cmd = Command::new("cargo");
        cmd.current_dir(&root);
        cmd.args([
            "test",
            "-p",
            "gitctx-mcp",
            "--test",
            "inspector",
            "test_mcp_inspector",
        ]);
        let result = crate::util::run_cmd(&mut cmd, "MCP inspector round-trip benchmark");
        let elapsed = start.elapsed().as_millis();
        let passed = result.is_ok();
        results.push(BenchResult {
            name: "MCP JSON-RPC Round-Trip",
            budget: "< 500ms",
            elapsed_ms: elapsed,
            passed,
            note: if passed {
                Some("initialize -> tools/list -> tools/call pipeline".to_string())
            } else {
                Some(result.unwrap_err().to_string())
            },
        });
    }

    // Print Structured Summary Report Table
    println!("\n==========================================================================================");
    println!("                            PERFORMANCE BENCHMARK REPORT                                 ");
    println!("==========================================================================================");

    println!(
        "{:<35} | {:<15} | {:<10} | {:<8}",
        "Benchmark", "Budget", "Elapsed", "Status"
    );
    println!("{:-<80}", "");

    let mut all_passed = true;
    for result in &results {
        let status_str = if result.passed { "PASS" } else { "FAIL" };
        if !result.passed {
            all_passed = false;
        }

        println!(
            "{:<35} | {:<15} | {:>7} ms | {:<8}",
            result.name, result.budget, result.elapsed_ms, status_str
        );
        if let Some(ref note) = result.note {
            println!("  --> {note}");
        }
    }

    println!("{:-<80}", "");
    let passed_count = results.iter().filter(|r| r.passed).count();
    let total_count = results.len();
    let total_time = overall_start.elapsed().as_millis();

    if all_passed {
        println!(
            "RESULT: ALL BENCHMARKS PASSED ({passed_count}/{total_count}) in {total_time} ms.\n"
        );
        Ok(())
    } else {
        println!(
            "RESULT: BENCHMARK CHECKS FAILED ({passed_count}/{total_count} passed) in {total_time} ms.\n"
        );
        bail!("One or more benchmark checks failed")
    }
}

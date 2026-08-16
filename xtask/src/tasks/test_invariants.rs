//! Unified invariant verification test runner per D-16.
//!
//! Orchestrates offline execution assertions, fixture determinism tests,
//! mutation manifest validation, and configuration hash verification in a unified test run.

use anyhow::{bail, Result};
use std::process::Command;
use std::time::Instant;

struct InvariantCheck {
    id: &'static str,
    name: &'static str,
    target: &'static str,
    passed: bool,
    duration_ms: u128,
    error_msg: Option<String>,
}

/// Runs the complete project invariants verification suite.
#[allow(clippy::too_many_lines)]
pub fn run_test_invariants() -> Result<()> {
    let root = crate::util::project_root();
    let overall_start = Instant::now();

    println!("\n==========================================================================================");
    println!("                           GITCTX INVARIANTS VERIFICATION SUITE                           ");
    println!("==========================================================================================\n");

    let mut checks = Vec::new();

    // Check 1: Invariant I1 - Zero Network Egress (Offline Execution)
    {
        println!("[1/5] Running Invariant I1 Check (Zero Network Egress)...");
        let start = Instant::now();
        let result = crate::tasks::test_offline::run_test_offline();
        let duration = start.elapsed().as_millis();
        let (passed, err) = match result {
            Ok(()) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };
        checks.push(InvariantCheck {
            id: "I1",
            name: "Zero Network Socket Egress (Offline)",
            target: "xtask test-offline",
            passed,
            duration_ms: duration,
            error_msg: err,
        });
    }

    // Check 2: Invariant I2 - Synthetic Fixture Determinism
    {
        println!("[2/5] Running Invariant I2 Check (Fixture Deterministic Hashes)...");
        let start = Instant::now();
        let mut cmd = Command::new("cargo");
        cmd.current_dir(&root);
        cmd.args([
            "test",
            "-p",
            "gitctx-eval",
            "--test",
            "fixture_tests",
            "test_deterministic_hashes",
        ]);
        let result = crate::util::run_cmd(&mut cmd, "fixture_tests test_deterministic_hashes");
        let duration = start.elapsed().as_millis();
        let (passed, err) = match result {
            Ok(()) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };
        checks.push(InvariantCheck {
            id: "I2",
            name: "Synthetic Fixture Hash Determinism",
            target: "fixture_tests",
            passed,
            duration_ms: duration,
            error_msg: err,
        });
    }

    // Check 3: Invariant I3 - Mutation Oracle & Ground Truth Manifest
    {
        println!("[3/5] Running Invariant I3 Check (Mutation Engine M1-M10 & Manifests)...");
        let start = Instant::now();
        let mut cmd = Command::new("cargo");
        cmd.current_dir(&root);
        cmd.args(["test", "-p", "gitctx-eval", "--test", "mutation_tests"]);
        let result = crate::util::run_cmd(&mut cmd, "mutation_tests");
        let duration = start.elapsed().as_millis();
        let (passed, err) = match result {
            Ok(()) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };
        checks.push(InvariantCheck {
            id: "I3",
            name: "Mutation Oracle & Manifest Schema",
            target: "mutation_tests",
            passed,
            duration_ms: duration,
            error_msg: err,
        });
    }

    // Check 4: Invariant I4 - Configuration Hash BLAKE3 Determinism
    {
        println!("[4/5] Running Invariant I4 Check (Config Hash BLAKE3 Determinism)...");
        let start = Instant::now();
        let mut cmd = Command::new("cargo");
        cmd.current_dir(&root);
        cmd.args(["test", "-p", "gitctx-core", "--lib", "config"]);
        let result = crate::util::run_cmd(&mut cmd, "gitctx-core config unit tests");
        let duration = start.elapsed().as_millis();
        let (passed, err) = match result {
            Ok(()) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };
        checks.push(InvariantCheck {
            id: "I4",
            name: "Config Hash BLAKE3 Determinism",
            target: "gitctx-core::config",
            passed,
            duration_ms: duration,
            error_msg: err,
        });
    }

    // Check 5: Invariant I6 - Pinned Evaluation Corpus SHA Enforcement
    {
        println!("[5/6] Running Invariant I6 Check (Pinned Corpus SHA Verification)...");
        let start = Instant::now();
        let mut cmd = Command::new("cargo");
        cmd.current_dir(&root);
        cmd.args(["test", "-p", "gitctx-eval", "--test", "invariant_i6_test"]);
        let result = crate::util::run_cmd(&mut cmd, "invariant_i6_test");
        let duration = start.elapsed().as_millis();
        let (passed, err) = match result {
            Ok(()) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };
        checks.push(InvariantCheck {
            id: "I6",
            name: "Pinned Evaluation Corpus SHA Lock (I6)",
            target: "invariant_i6_test",
            passed,
            duration_ms: duration,
            error_msg: err,
        });
    }

    // Check 6: Ground Truth Cryptographic Manifest Verification (D-08, D-15)
    {
        println!("[6/6] Running Ground Truth Cryptographic Integrity Check...");
        let start = Instant::now();
        let result = crate::tasks::verify_ground_truth::run_verify_ground_truth();
        let duration = start.elapsed().as_millis();
        let (passed, err) = match result {
            Ok(()) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };
        checks.push(InvariantCheck {
            id: "GT",
            name: "Ground Truth Cryptographic Integrity",
            target: "xtask verify-ground-truth",
            passed,
            duration_ms: duration,
            error_msg: err,
        });
    }

    // Print Structured Summary Report Table
    println!("\n==========================================================================================");
    println!("                               INVARIANTS SUMMARY REPORT                                  ");
    println!("==========================================================================================");

    println!(
        "{:<4} | {:<42} | {:<20} | {:<8} | Time",
        "ID", "Invariant Description", "Target / Harness", "Status"
    );
    println!("{:-<90}", "");

    let mut all_passed = true;
    for check in &checks {
        let status_str = if check.passed { "PASS" } else { "FAIL" };
        if !check.passed {
            all_passed = false;
        }

        println!(
            "{:<4} | {:<42} | {:<20} | {:<8} | {} ms",
            check.id, check.name, check.target, status_str, check.duration_ms
        );
        if let Some(ref err) = check.error_msg {
            println!("     --> ERROR: {err}");
        }
    }

    println!("{:-<90}", "");
    let passed_count = checks.iter().filter(|c| c.passed).count();
    let total_count = checks.len();
    let total_time = overall_start.elapsed().as_millis();

    if all_passed {
        println!(
            "RESULT: ALL INVARIANTS PASSED ({passed_count}/{total_count}) in {total_time} ms.\n"
        );
        Ok(())
    } else {
        println!(
            "RESULT: INVARIANT CHECKS FAILED ({passed_count}/{total_count} passed) in {total_time} ms.\n"
        );
        bail!("One or more invariant checks failed");
    }
}

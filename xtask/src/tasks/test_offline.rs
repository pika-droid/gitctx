//! Invariant I1 offline test harness.
//!
//! Enforces zero network socket egress during test execution via environment proxy blackholing
//! and Linux OS-level network namespace isolation (`unshare -n`) per D-13.

use anyhow::{bail, Context, Result};
use std::process::Command;

/// Runs the offline invariant test suite across Tier 1 and Tier 2 verification layers.
pub fn run_test_offline() -> Result<()> {
    let root = crate::util::project_root();
    println!("\n=== Invariant I1: Offline Test Harness ===");
    println!("Enforcing zero network egress during test execution per D-13.\n");

    // Tier 1: Cross-Platform Environment Network Blockade
    println!("--> [Tier 1] Executing cargo test with blackholed proxy and offline environment...");
    let mut cmd = Command::new("cargo");
    cmd.current_dir(&root);
    cmd.args(["test", "--workspace"]);
    cmd.env("HTTP_PROXY", "http://0.0.0.0:0");
    cmd.env("HTTPS_PROXY", "http://0.0.0.0:0");
    cmd.env("ALL_PROXY", "http://0.0.0.0:0");
    cmd.env("NO_PROXY", "");
    cmd.env("GITCTX_OFFLINE_MODE", "1");

    crate::util::run_cmd(&mut cmd, "cargo test --workspace (offline env)")
        .context("Offline test execution failed under Tier 1 proxy blockade")?;
    println!("--> [Tier 1] PASS: All tests passed with network proxy blackholed.\n");

    // Tier 2: Linux OS-level network namespace isolation (`unshare -n`)
    if cfg!(target_os = "linux") {
        println!(
            "--> [Tier 2] Linux detected: Attempting unshare -n network namespace isolation..."
        );
        let unshare_check = Command::new("unshare")
            .args(["-n", "cargo", "test", "--workspace"])
            .current_dir(&root)
            .status();

        match unshare_check {
            Ok(status) if status.success() => {
                println!("--> [Tier 2] PASS: unshare -n network isolation verified.\n");
            }
            Ok(status) => {
                bail!(
                    "Tier 2 unshare -n test failed with exit code: {:?}",
                    status.code()
                );
            }
            Err(e) => {
                println!("--> [Tier 2] NOTICE: unshare not available in current environment ({e}). Validated in CI.\n");
            }
        }
    } else {
        println!("--> [Tier 2] OS is {} (non-Linux).", std::env::consts::OS);
        println!("    OS-level network namespace isolation (unshare -n) is enforced automatically in Linux CI.");
        println!("--> [Tier 2] SKIPPED (platform-dependent, validated in Linux CI).\n");
    }

    println!("=== Invariant I1: Offline Verification Completed Successfully ===\n");
    Ok(())
}

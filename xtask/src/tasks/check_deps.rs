use crate::util::{project_root, run_tool};
use anyhow::Result;

/// Runs all dependency health checks: unused dependencies, licenses, and security advisories.
pub fn run_check_deps(allow_missing: bool) -> Result<()> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let root = project_root();

    println!("==> [1/3] Checking unused dependencies with cargo-machete...");
    run_tool(
        &cargo,
        "machete",
        &[],
        "cargo-machete",
        allow_missing,
        Some(&root),
    )?;

    println!("==> [2/3] Checking licenses, bans & advisories with cargo-deny...");
    run_tool(
        &cargo,
        "deny",
        &["check"],
        "cargo-deny",
        allow_missing,
        Some(&root),
    )?;

    println!("==> [3/3] Auditing dependency vulnerabilities with cargo-audit...");
    run_tool(
        &cargo,
        "audit",
        &[],
        "cargo-audit",
        allow_missing,
        Some(&root),
    )?;

    println!("==> All dependency health checks completed successfully!");
    Ok(())
}

/// Runs security vulnerability audit and license checks (`cargo-audit` + `cargo-deny`).
pub fn run_audit(allow_missing: bool) -> Result<()> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let root = project_root();

    println!("==> [1/2] Checking licenses, bans & advisories with cargo-deny...");
    run_tool(
        &cargo,
        "deny",
        &["check"],
        "cargo-deny",
        allow_missing,
        Some(&root),
    )?;

    println!("==> [2/2] Auditing dependency vulnerabilities with cargo-audit...");
    run_tool(
        &cargo,
        "audit",
        &[],
        "cargo-audit",
        allow_missing,
        Some(&root),
    )?;

    println!("==> All security and license audit checks completed successfully!");
    Ok(())
}

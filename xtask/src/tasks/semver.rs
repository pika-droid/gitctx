use crate::util::{project_root, run_tool};
use anyhow::Result;

/// Runs semantic versioning checks against a baseline using `cargo-semver-checks`.
pub fn run_semver(
    baseline_rev: Option<&str>,
    package: Option<&str>,
    allow_missing: bool,
) -> Result<()> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let root = project_root();

    let mut args: Vec<String> = vec!["check-release".to_string()];
    if let Some(rev) = baseline_rev {
        args.push(format!("--baseline-rev={rev}"));
    }
    if let Some(pkg) = package {
        args.push(format!("-p={pkg}"));
    }

    let str_args: Vec<&str> = args.iter().map(AsRef::as_ref).collect();

    println!("==> Checking SemVer compatibility with cargo-semver-checks...");
    run_tool(
        &cargo,
        "semver-checks",
        &str_args,
        "cargo-semver-checks",
        allow_missing,
        Some(&root),
    )?;

    println!("==> SemVer check completed successfully!");
    Ok(())
}

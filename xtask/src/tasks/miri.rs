use crate::util::{project_root, run_tool};
use anyhow::Result;

pub fn run_miri(package: Option<&str>, allow_missing: bool) -> Result<()> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let root = project_root();

    let target_pkg = package.unwrap_or("gitctx-core");
    let pkg_arg = format!("-p={target_pkg}");

    println!("==> Running Miri UB verification on package '{target_pkg}'...");
    run_tool(
        &cargo,
        "miri",
        &["test", &pkg_arg],
        "cargo-miri",
        allow_missing,
        Some(&root),
    )?;

    println!("==> Miri check completed successfully!");
    Ok(())
}

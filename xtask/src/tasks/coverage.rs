use crate::util::{project_root, run_tool};
use anyhow::Result;

pub fn run_coverage(html: bool, lcov: bool, allow_missing: bool) -> Result<()> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let root = project_root();

    let mut args = vec!["--workspace"];
    if html {
        args.push("--html");
    }
    if lcov {
        args.push("--lcov");
        args.push("--output-path=lcov.info");
    }

    println!("==> Running code coverage with cargo-llvm-cov...");
    run_tool(
        &cargo,
        "llvm-cov",
        &args,
        "cargo-llvm-cov",
        allow_missing,
        Some(&root),
    )?;

    if html {
        println!("==> HTML coverage report generated in target/llvm-cov/html/index.html");
    } else if lcov {
        println!("==> LCOV coverage report generated in lcov.info");
    }

    Ok(())
}

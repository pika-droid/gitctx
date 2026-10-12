use crate::util::{project_root, run_cmd};
use anyhow::Result;
use std::process::Command;

pub fn run_test(use_cargo_test: bool, test_filter: Option<&str>) -> Result<()> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let root = project_root();

    if !use_cargo_test {
        // Check if cargo nextest is installed
        let mut check_nextest = Command::new(&cargo);
        check_nextest.arg("nextest").arg("--help");
        let nextest_available = check_nextest
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if nextest_available {
            println!("==> Running tests with cargo-nextest...");
            let mut args = vec!["run", "--workspace"];
            if let Some(filter) = test_filter {
                args.push(filter);
            }
            let mut cmd = Command::new(&cargo);
            cmd.arg("nextest").args(&args).current_dir(&root);
            return run_cmd(&mut cmd, &format!("cargo nextest {}", args.join(" ")));
        } else {
            println!("==> Note: 'cargo-nextest' is not installed, falling back to 'cargo test'.");
            println!("    (Tip: install cargo-nextest with: cargo install cargo-nextest --locked)");
        }
    }

    println!("==> Running tests with standard cargo test...");
    let mut cmd = Command::new(&cargo);
    cmd.arg("test").arg("--workspace").current_dir(&root);
    if let Some(filter) = test_filter {
        cmd.arg(filter);
    }
    run_cmd(&mut cmd, "cargo test --workspace")
}

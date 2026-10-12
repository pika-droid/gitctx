use crate::tasks::check_deps::run_check_deps;
use crate::util::run_cmd;
use anyhow::Result;
use std::process::Command;

pub fn run_lint(fix: bool, check_deps: bool) -> Result<()> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    if fix {
        let mut fmt_cmd = Command::new(&cargo);
        fmt_cmd.args(["fmt", "--all"]);
        run_cmd(&mut fmt_cmd, "cargo fmt --all")?;

        let mut clippy_cmd = Command::new(&cargo);
        clippy_cmd.args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--fix",
            "--allow-dirty",
            "--allow-staged",
            "--",
            "-D",
            "warnings",
        ]);
        run_cmd(&mut clippy_cmd, "cargo clippy --fix")?;
    } else {
        let mut fmt_cmd = Command::new(&cargo);
        fmt_cmd.args(["fmt", "--all", "--", "--check"]);
        run_cmd(&mut fmt_cmd, "cargo fmt --check")?;

        let mut clippy_cmd = Command::new(&cargo);
        clippy_cmd.args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ]);
        run_cmd(
            &mut clippy_cmd,
            "cargo clippy --workspace --all-targets -- -D warnings",
        )?;
    }

    if check_deps {
        run_check_deps(true)?;
    }

    println!("All lint and format checks passed successfully!");
    Ok(())
}

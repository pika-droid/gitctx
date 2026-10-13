use crate::tasks::check_deps::run_check_deps;
use crate::util::{project_root, run_cmd};
use anyhow::Result;
use std::process::Command;

pub fn run_lint(fix: bool, check_deps: bool) -> Result<()> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let root = project_root();

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

        // Format TOML files if taplo is available
        if Command::new("taplo").arg("--version").output().is_ok() {
            let mut taplo_cmd = Command::new("taplo");
            taplo_cmd.arg("fmt").current_dir(&root);
            let _ = run_cmd(&mut taplo_cmd, "taplo fmt");
        }
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

        // Check TOML files if taplo is available
        if Command::new("taplo").arg("--version").output().is_ok() {
            let mut taplo_cmd = Command::new("taplo");
            taplo_cmd.args(["fmt", "--check"]).current_dir(&root);
            let _ = run_cmd(&mut taplo_cmd, "taplo fmt --check");
        }
    }

    if check_deps {
        run_check_deps(true)?;
    }

    println!("All lint and format checks passed successfully!");
    Ok(())
}

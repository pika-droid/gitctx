use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Returns the root directory of the cargo workspace.
pub fn project_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("Failed to find workspace root directory")
        .to_path_buf()
}

/// Executes a command and streams output, returning an error if the command fails.
pub fn run_cmd(cmd: &mut Command, desc: &str) -> Result<()> {
    println!("==> Running: {desc}");
    let status = cmd
        .status()
        .with_context(|| format!("Failed to execute command for '{desc}'"))?;

    if !status.success() {
        bail!(
            "Command '{desc}' failed with exit code: {:?}",
            status.code()
        );
    }

    Ok(())
}

/// Runs an external cargo extension tool, providing clear installation instructions if missing.
pub fn run_tool(
    cargo: &str,
    tool: &str,
    args: &[&str],
    install_crate: &str,
    allow_missing: bool,
    cwd: Option<&Path>,
) -> Result<bool> {
    // Check if the cargo extension is installed
    let mut check_cmd = Command::new(cargo);
    check_cmd.arg(tool).arg("--help");
    if let Some(dir) = cwd {
        check_cmd.current_dir(dir);
    }

    let is_installed = match check_cmd.output() {
        Ok(out) => out.status.success(),
        Err(_) => false,
    };

    if !is_installed {
        let msg = format!(
            "'cargo-{tool}' is not installed.\nTo install: cargo install {install_crate} --locked"
        );
        if allow_missing {
            println!("==> ⚠️  {msg} (skipping)");
            return Ok(false);
        }
        bail!("{msg}");
    }

    let mut cmd = Command::new(cargo);
    cmd.arg(tool);
    for arg in args {
        cmd.arg(arg);
    }
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    let desc = if args.is_empty() {
        format!("cargo {tool}")
    } else {
        format!("cargo {tool} {}", args.join(" "))
    };

    run_cmd(&mut cmd, &desc)?;
    Ok(true)
}

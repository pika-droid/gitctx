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

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Runs synthetic mutation generation or evaluation tasks.
pub fn run_eval_mutate(
    class: Option<&str>,
    chain_depth: usize,
    out_dir: Option<&Path>,
) -> Result<()> {
    let root = crate::util::project_root();

    let mut cmd = Command::new("cargo");
    cmd.current_dir(&root);
    cmd.args(["run", "-p", "gitctx-eval", "--", "mutate"]);

    if let Some(c) = class {
        cmd.args(["--class", c]);
    }
    cmd.args(["--chain-depth", &chain_depth.to_string()]);

    if let Some(dir) = out_dir {
        cmd.arg("--out");
        cmd.arg(dir);
    }

    crate::util::run_cmd(&mut cmd, "gitctx-eval mutate")
        .context("Failed running synthetic mutation generator")?;

    Ok(())
}

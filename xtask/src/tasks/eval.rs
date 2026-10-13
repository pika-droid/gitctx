use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Runs synthetic mutation generation.
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

/// Runs synthetic lineage benchmark evaluation suite.
pub fn run_eval_bench_lineage(format: Option<&str>) -> Result<()> {
    let root = crate::util::project_root();

    let mut cmd = Command::new("cargo");
    cmd.current_dir(&root);
    cmd.args(["run", "-p", "gitctx-eval", "--", "bench-lineage"]);

    if let Some(f) = format {
        cmd.args(["--format", f]);
    }

    crate::util::run_cmd(&mut cmd, "gitctx-eval bench-lineage")
        .context("Failed running lineage benchmark evaluation")?;

    Ok(())
}

/// Runs threshold calibration tuning sweep.
pub fn run_eval_tune_thresholds(out_path: Option<&Path>) -> Result<()> {
    let root = crate::util::project_root();

    let mut cmd = Command::new("cargo");
    cmd.current_dir(&root);
    cmd.args(["run", "-p", "gitctx-eval", "--", "tune-thresholds"]);

    if let Some(out) = out_path {
        cmd.arg("--out");
        cmd.arg(out);
    }

    crate::util::run_cmd(&mut cmd, "gitctx-eval tune-thresholds")
        .context("Failed running threshold tuning sweep")?;

    Ok(())
}

/// Runs git log -L oracle comparison evaluation.
pub fn run_eval_oracle(repo_path: &Path) -> Result<()> {
    let root = crate::util::project_root();

    let mut cmd = Command::new("cargo");
    cmd.current_dir(&root);
    cmd.args(["run", "-p", "gitctx-eval", "--", "eval-oracle", "--repo"]);
    cmd.arg(repo_path);

    crate::util::run_cmd(&mut cmd, "gitctx-eval eval-oracle")
        .context("Failed running git log -L oracle evaluation")?;

    Ok(())
}

/// Generates the comprehensive evaluation report (Phase 11, Plan 11-01).
pub fn run_eval_report(eval_dir: Option<&Path>, output: Option<&Path>) -> Result<()> {
    let root = crate::util::project_root();

    let mut cmd = Command::new("cargo");
    cmd.current_dir(&root);
    cmd.args(["run", "-p", "gitctx-eval", "--", "report"]);

    if let Some(dir) = eval_dir {
        cmd.arg("--eval-dir");
        cmd.arg(dir);
    }

    if let Some(out) = output {
        cmd.arg("--output");
        cmd.arg(out);
    }

    crate::util::run_cmd(&mut cmd, "gitctx-eval report")
        .context("Failed running evaluation report generator")?;

    Ok(())
}

/// Runs 5x5 retrieval ablation grid evaluation across D1-D5 and R1-R5.
pub fn run_eval_ablation(eval_dir: Option<&Path>) -> Result<()> {
    let root = crate::util::project_root();

    let mut cmd = Command::new("cargo");
    cmd.current_dir(&root);
    cmd.args(["run", "-p", "gitctx-eval", "--", "ablation"]);

    if let Some(dir) = eval_dir {
        cmd.arg("--eval-dir");
        cmd.arg(dir);
    }

    crate::util::run_cmd(&mut cmd, "gitctx-eval ablation")
        .context("Failed running retrieval ablation grid evaluation")?;

    Ok(())
}


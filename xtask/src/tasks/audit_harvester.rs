//! Harvester precision audit automation per D-07.
//!
//! Invokes `eval/harvest.py --audit <samples>` to sample harvested issue-commit pairs,
//! run human or batch inspection, compute 95% Wilson Score CI, and enforce precision >= 0.85.

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Runs the precision audit CLI via `eval/harvest.py --audit`.
pub fn run_audit_harvester(
    samples: usize,
    batch_file: Option<&Path>,
    synthetic: bool,
    non_interactive: bool,
) -> Result<()> {
    let root = crate::util::project_root();
    let script_path = root.join("eval/harvest.py");

    // Check for virtualenv python
    let venv_python_win = root.join(".venv/Scripts/python.exe");
    let venv_python_nix = root.join(".venv/bin/python");
    let python_bin = if venv_python_win.exists() {
        venv_python_win
    } else if venv_python_nix.exists() {
        venv_python_nix
    } else {
        PathBuf::from("python")
    };

    println!("==========================================================================================");
    println!("                           RUNNING HARVESTER PRECISION AUDIT                              ");
    println!("==========================================================================================\n");

    let mut cmd = Command::new(python_bin);
    cmd.current_dir(&root);
    cmd.arg(&script_path);
    cmd.args(["--audit", &samples.to_string()]);

    if let Some(bf) = batch_file {
        cmd.args(["--audit-batch", bf.to_str().unwrap_or_default()]);
    }

    if synthetic {
        cmd.arg("--synthetic");
    }

    if non_interactive {
        cmd.arg("--non-interactive");
    }

    crate::util::run_cmd(&mut cmd, "eval/harvest.py --audit")?;
    println!("\nHarvester precision audit and Wilson CI calculation complete.");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_audit_harvester_script_exists() {
        let root = crate::util::project_root();
        let script = root.join("eval/harvest.py");
        assert!(script.exists(), "eval/harvest.py must exist");
    }
}

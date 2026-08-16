//! Ground truth harvesting automation per D-05, D-06, and D-08.
//!
//! Invokes `eval/harvest.py` to extract Set A (issue-closing pairs) and revert pairs,
//! apply bot filtering and label capping, and serialize canonical datasets into `eval/ground_truth/`.

use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

/// Runs the ground truth harvesting engine via `eval/harvest.py`.
pub fn run_harvest(repo: Option<&str>, offline: bool) -> Result<()> {
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
    println!("                           RUNNING GROUND TRUTH HARVESTER                                 ");
    println!("==========================================================================================\n");

    let mut cmd = Command::new(python_bin);
    cmd.current_dir(&root);
    cmd.arg(&script_path);

    if let Some(r) = repo {
        cmd.args(["--repo", r]);
    }

    if offline {
        cmd.arg("--offline");
    }

    crate::util::run_cmd(&mut cmd, "eval/harvest.py")?;
    println!("\nGround truth harvesting and dataset serialization complete.");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_harvest_script_exists() {
        let root = crate::util::project_root();
        let script = root.join("eval/harvest.py");
        assert!(script.exists(), "eval/harvest.py must exist");
    }
}

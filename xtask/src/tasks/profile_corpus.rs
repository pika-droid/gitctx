//! Repository profiling automation per D-01 and D-02.
//!
//! Invokes `eval/profile.py` to scan candidate repositories, compute structural metrics,
//! generate `eval/corpus_selection.md`, and lock exact commit SHAs into `eval/corpus.lock.toml`.

use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

/// Runs the repository profiler engine via `eval/profile.py`.
pub fn run_profile_corpus(candidates: Option<&str>, offline: bool) -> Result<()> {
    let root = crate::util::project_root();
    let script_path = root.join("eval/profile.py");

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
    println!("                           RUNNING MULTI-LANGUAGE PROFILER                                ");
    println!("==========================================================================================\n");

    let mut cmd = Command::new(python_bin);
    cmd.current_dir(&root);
    cmd.arg(&script_path);

    if let Some(cands) = candidates {
        cmd.args(["--candidates", cands]);
    }

    if offline {
        cmd.arg("--offline");
    }

    crate::util::run_cmd(&mut cmd, "eval/profile.py")?;
    println!("\nCorpus profiling and SHA locking complete.");
    Ok(())
}

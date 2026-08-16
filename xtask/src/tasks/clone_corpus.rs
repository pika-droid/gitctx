//! Corpus cloning automation per D-03.
//!
//! Clones pinned repositories defined in `eval/corpus.lock.toml` into `eval/corpus/<owner>__<repo>`
//! and checks out the exact pinned commit SHA, enforcing Invariant I6.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::process::Command;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct CorpusLock {
    pub version: u32,
    pub locked_at: String,
    pub profiler_script_sha: String,
    #[serde(default)]
    pub corpus: Vec<CorpusEntry>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct CorpusEntry {
    pub id: String,
    pub owner: String,
    pub repo: String,
    pub url: String,
    pub sha: String,
    pub language: String,
    pub size_tier: String,
}

/// Runs the corpus cloning and SHA-pinning task.
pub fn run_clone_corpus(lockfile: Option<&Path>, target_dir: Option<&Path>) -> Result<()> {
    let root = crate::util::project_root();
    let lock_path = lockfile.map_or_else(|| root.join("eval/corpus.lock.toml"), Path::to_path_buf);
    let corpus_dir = target_dir.map_or_else(|| root.join("eval/corpus"), Path::to_path_buf);

    if !lock_path.exists() {
        bail!(
            "Corpus lockfile not found at: {}\nPlease run `cargo xtask profile-corpus` first.",
            lock_path.display()
        );
    }

    let lock_content = fs::read_to_string(&lock_path)
        .with_context(|| format!("Failed to read lockfile at {}", lock_path.display()))?;
    let lock: CorpusLock = toml::from_str(&lock_content)
        .with_context(|| format!("Failed to parse TOML lockfile at {}", lock_path.display()))?;

    fs::create_dir_all(&corpus_dir)
        .with_context(|| format!("Failed to create corpus dir at {}", corpus_dir.display()))?;

    println!("==========================================================================================");
    println!("                           CLONING EVALUATION CORPUS (INVARIANT I6)                       ");
    println!("==========================================================================================");
    println!("Lockfile: {}", lock_path.display());
    println!("Corpus directory: {}", corpus_dir.display());
    println!("Pinned repositories: {}\n", lock.corpus.len());

    for entry in &lock.corpus {
        let repo_dir = corpus_dir.join(format!("{}__{}", entry.owner, entry.repo));
        println!(
            "--> Processing [{}] {}/{} (SHA: {})...",
            entry.language, entry.owner, entry.repo, entry.sha
        );

        if repo_dir.exists() {
            println!(
                "    Repository directory already exists at {}",
                repo_dir.display()
            );
        } else {
            println!("    Cloning from {} ...", entry.url);
            let mut clone_cmd = Command::new("git");
            clone_cmd.args(["clone", &entry.url, repo_dir.to_str().unwrap()]);
            crate::util::run_cmd(&mut clone_cmd, "git clone repository")?;
        }

        // Fetch to ensure we have the commit
        let mut fetch_cmd = Command::new("git");
        fetch_cmd.current_dir(&repo_dir);
        fetch_cmd.args(["fetch", "--all", "--tags"]);
        let _ = crate::util::run_cmd(&mut fetch_cmd, "git fetch");

        // Checkout exact commit SHA (D-03)
        let mut checkout_cmd = Command::new("git");
        checkout_cmd.current_dir(&repo_dir);
        checkout_cmd.args(["checkout", "--force", &entry.sha]);
        crate::util::run_cmd(&mut checkout_cmd, "git checkout locked SHA")?;

        // Verify checked out HEAD SHA
        let head_out = Command::new("git")
            .current_dir(&repo_dir)
            .args(["rev-parse", "HEAD"])
            .output()
            .with_context(|| "Failed to run git rev-parse HEAD")?;

        let head_sha = String::from_utf8_lossy(&head_out.stdout).trim().to_string();
        if !head_sha.eq_ignore_ascii_case(&entry.sha) {
            bail!(
                "Invariant I6 Violation: repository {} HEAD SHA ({}) does not match locked SHA ({})",
                repo_dir.display(),
                head_sha,
                entry.sha
            );
        }

        println!("    Verified HEAD matches locked SHA: {head_sha}\n");
    }

    println!("RESULT: All pinned repositories cloned and verified under Invariant I6.\n");
    Ok(())
}

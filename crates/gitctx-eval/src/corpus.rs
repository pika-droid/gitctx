//! Corpus lockfile parsing and Invariant I6 runtime SHA verification.

use gitctx_core::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Pinned evaluation corpus lockfile schema (Schema Version 1 per Invariant I6).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CorpusLock {
    pub version: u32,
    pub locked_at: String,
    pub profiler_script_sha: String,
    #[serde(default)]
    pub corpus: Vec<CorpusEntry>,
}

/// Metadata and pinned commit SHA for an evaluation repository.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CorpusEntry {
    pub id: String,
    pub owner: String,
    pub repo: String,
    pub url: String,
    pub sha: String,
    pub language: String,
    pub size_tier: String,
    pub commit_count: usize,
    pub first_parent_count: usize,
    pub traceable_issues: usize,
    pub merge_fraction: f64,
    pub bot_fraction: f64,
    pub harvest_date: String,
}

/// Verification report item for a corpus repository.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CorpusVerificationReport {
    pub repo_id: String,
    pub owner: String,
    pub repo: String,
    pub path: PathBuf,
    pub expected_sha: String,
    pub actual_sha: Option<String>,
    pub matched: bool,
    pub error: Option<String>,
}

impl CorpusLock {
    /// Load and deserialize `eval/corpus.lock.toml` from a file path.
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(CoreError::Io)?;
        toml::from_str(&content).map_err(|e| CoreError::ParseError(e.to_string()))
    }
}

/// Asserts that a local repository exists at `repo_path` and its HEAD matches `expected_sha` exactly (Invariant I6).
pub fn assert_corpus_locked_sha(repo_path: &Path, expected_sha: &str) -> Result<()> {
    let repo = git2::Repository::open(repo_path).map_err(|e| CoreError::RepositoryOpen {
        path: repo_path.to_path_buf(),
        message: e.to_string(),
    })?;

    let head = repo
        .head()
        .and_then(|h| h.peel_to_commit())
        .map_err(|e| CoreError::GitInspection(e.to_string()))?;

    let actual_sha = head.id().to_string();
    if actual_sha.to_lowercase() != expected_sha.to_lowercase() {
        return Err(CoreError::CorpusShaMismatch {
            expected: expected_sha.to_string(),
            actual: actual_sha,
            repo: repo_path.to_path_buf(),
        });
    }

    Ok(())
}

/// Verifies all locked repositories in `eval/corpus.lock.toml` against local directories in `corpus_base_dir`.
pub fn verify_all_corpus_locks(
    lockfile_path: &Path,
    corpus_base_dir: &Path,
) -> Result<Vec<CorpusVerificationReport>> {
    let lock = CorpusLock::load_from_path(lockfile_path)?;
    let mut reports = Vec::new();

    for entry in &lock.corpus {
        let repo_dir = corpus_base_dir.join(format!("{}__{}", entry.owner, entry.repo));
        match assert_corpus_locked_sha(&repo_dir, &entry.sha) {
            Ok(()) => {
                reports.push(CorpusVerificationReport {
                    repo_id: entry.id.clone(),
                    owner: entry.owner.clone(),
                    repo: entry.repo.clone(),
                    path: repo_dir,
                    expected_sha: entry.sha.clone(),
                    actual_sha: Some(entry.sha.clone()),
                    matched: true,
                    error: None,
                });
            }
            Err(CoreError::CorpusShaMismatch {
                expected, actual, ..
            }) => {
                reports.push(CorpusVerificationReport {
                    repo_id: entry.id.clone(),
                    owner: entry.owner.clone(),
                    repo: entry.repo.clone(),
                    path: repo_dir,
                    expected_sha: expected,
                    actual_sha: Some(actual),
                    matched: false,
                    error: Some("Commit SHA mismatch".to_string()),
                });
            }
            Err(err) => {
                reports.push(CorpusVerificationReport {
                    repo_id: entry.id.clone(),
                    owner: entry.owner.clone(),
                    repo: entry.repo.clone(),
                    path: repo_dir,
                    expected_sha: entry.sha.clone(),
                    actual_sha: None,
                    matched: false,
                    error: Some(err.to_string()),
                });
            }
        }
    }

    Ok(reports)
}

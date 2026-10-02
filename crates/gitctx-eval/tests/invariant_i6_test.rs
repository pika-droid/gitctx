//! Integration tests for Invariant I6: Pinned Evaluation Corpus SHA verification.

use gitctx_core::error::CoreError;
use gitctx_eval::corpus::{assert_corpus_locked_sha, verify_all_corpus_locks, CorpusLock};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn get_workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Helper creating a synthetic git repository with a single commit, returning its TempDir and HEAD SHA.
fn create_test_git_repo() -> (TempDir, String) {
    let tmp_dir = TempDir::new().expect("Failed to create tempdir");
    let tmp_path = tmp_dir.path();

    let run_git = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(tmp_path)
            .args(args)
            .output()
            .expect("Failed to execute git command");
        assert!(
            output.status.success(),
            "Git command failed: {:?}\nStderr: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    };

    run_git(&["init", "-b", "main"]);
    run_git(&["config", "user.name", "Test Author"]);
    run_git(&["config", "user.email", "author@example.com"]);

    let test_file = tmp_path.join("README.md");
    fs::write(&test_file, "# Test Repo\n").expect("Failed to write README.md");

    run_git(&["add", "README.md"]);
    run_git(&["commit", "-m", "Initial commit"]);

    let head_sha = run_git(&["rev-parse", "HEAD"]);
    (tmp_dir, head_sha)
}

#[test]
fn test_corpus_lockfile_deserialization() {
    let root = get_workspace_root();
    let lockfile_path = root.join("eval/corpus.lock.toml");
    assert!(
        lockfile_path.exists(),
        "eval/corpus.lock.toml must exist under Invariant I6"
    );

    let lock =
        CorpusLock::load_from_path(&lockfile_path).expect("Failed to parse eval/corpus.lock.toml");

    assert_eq!(lock.version, 1, "Schema version must be 1");
    assert!(
        !lock.locked_at.is_empty(),
        "locked_at timestamp must be present"
    );
    assert!(
        !lock.profiler_script_sha.is_empty(),
        "profiler_script_sha must be present"
    );
    assert!(
        lock.corpus.len() >= 3,
        "Corpus lock must contain at least 3 pinned repositories (D-02)"
    );

    let mut languages = std::collections::HashSet::new();
    let mut size_tiers = std::collections::HashSet::new();

    for entry in &lock.corpus {
        assert_ne!(entry.id, "", "id must not be empty");
        assert_ne!(entry.owner, "", "owner must not be empty");
        assert_ne!(entry.repo, "", "repo must not be empty");
        assert!(
            entry.url.starts_with("https://github.com/"),
            "url must be valid GitHub HTTPS url: {}",
            entry.url
        );
        assert_eq!(
            entry.sha.len(),
            40,
            "Pinned SHA must be 40-char hex string: {}",
            entry.sha
        );
        assert!(
            entry.sha.chars().all(|c| c.is_ascii_hexdigit()),
            "SHA must be valid hexadecimal: {}",
            entry.sha
        );
        assert!(entry.commit_count > 0, "commit_count must be > 0");
        assert!(
            entry.first_parent_count > 0,
            "first_parent_count must be > 0"
        );
        assert!(
            entry.traceable_issues >= 150,
            "traceable_issues must meet >= 150 gate (D-02)"
        );
        assert!(
            entry.merge_fraction < 0.40,
            "merge_fraction must meet < 0.40 gate (D-02)"
        );
        assert!(
            entry.bot_fraction <= 0.25,
            "bot_fraction must meet <= 0.25 gate (D-02)"
        );

        languages.insert(entry.language.clone());
        size_tiers.insert(entry.size_tier.clone());
    }

    // Must cover 3 distinct languages and 3 size tiers
    assert!(
        languages.len() >= 3,
        "Corpus must span >= 3 distinct languages"
    );
    assert!(
        size_tiers.contains("small")
            && size_tiers.contains("medium")
            && size_tiers.contains("large"),
        "Corpus must span small, medium, and large size tiers"
    );
}

#[test]
fn test_invariant_i6_matching_sha() {
    let (repo_dir, head_sha) = create_test_git_repo();

    // Verification succeeds when actual HEAD matches expected SHA
    let result = assert_corpus_locked_sha(repo_dir.path(), &head_sha);
    assert!(
        result.is_ok(),
        "assert_corpus_locked_sha must succeed on matching SHA"
    );
}

#[test]
fn test_invariant_i6_mismatch_sha_rejection() {
    let (repo_dir, head_sha) = create_test_git_repo();
    let wrong_sha = "0123456789abcdef0123456789abcdef01234567";

    // Verification must fail with CorpusShaMismatch error on mismatched SHA
    let result = assert_corpus_locked_sha(repo_dir.path(), wrong_sha);
    match result {
        Err(CoreError::CorpusShaMismatch {
            expected,
            actual,
            repo,
        }) => {
            assert_eq!(expected, wrong_sha);
            assert_eq!(actual, head_sha);
            assert_eq!(repo, repo_dir.path());
        }
        other => panic!("Expected CorpusShaMismatch error, got: {other:?}"),
    }
}

#[test]
fn test_invariant_i6_nonexistent_repo_error() {
    let fake_path = PathBuf::from("nonexistent_directory_for_test");
    let result = assert_corpus_locked_sha(&fake_path, "0123456789abcdef0123456789abcdef01234567");
    assert!(
        matches!(result, Err(CoreError::RepositoryOpen { .. })),
        "Must return RepositoryOpen error on invalid path"
    );
}

#[test]
fn test_verify_all_corpus_locks_execution() {
    let (repo_dir, head_sha) = create_test_git_repo();
    let temp_corpus_dir = TempDir::new().expect("Failed to create tempdir");
    let test_repo_subdir = temp_corpus_dir.path().join("gin-gonic__gin");

    // Copy fixture git dir over to subdir
    let _ = fs::rename(repo_dir.path(), &test_repo_subdir);

    let temp_lock_file = temp_corpus_dir.path().join("test.lock.toml");
    let lock_content = format!(
        "version = 1\nlocked_at = \"2026-08-16T21:00:00Z\"\nprofiler_script_sha = \"0123456789abcdef0123456789abcdef01234567\"\n\n[[corpus]]\nid = \"gin\"\nowner = \"gin-gonic\"\nrepo = \"gin\"\nurl = \"https://github.com/gin-gonic/gin.git\"\nsha = \"{head_sha}\"\nlanguage = \"go\"\nsize_tier = \"small\"\ncommit_count = 100\nfirst_parent_count = 100\ntraceable_issues = 150\nmerge_fraction = 0.1\nbot_fraction = 0.01\nharvest_date = \"2026-08-16\"\n"
    );
    fs::write(&temp_lock_file, lock_content).unwrap();

    let reports = verify_all_corpus_locks(&temp_lock_file, temp_corpus_dir.path())
        .expect("verify_all_corpus_locks must succeed");
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].repo_id, "gin");
}

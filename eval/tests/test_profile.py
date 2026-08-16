"""Unit and integration tests for the repository profiler and .gitctxignore exclusions."""

from __future__ import annotations

import os
from pathlib import Path
import subprocess
import tempfile
import pytest

from eval.profile import (
    GitctxIgnoreMatcher,
    is_bot_author,
    RepoProfiler,
    RepoProfile,
    evaluate_candidate,
    CandidateEvaluation,
    select_balanced_triad,
    generate_corpus_selection_md,
    generate_corpus_lock_toml,
    get_preprofiled_candidates,
    PRESET_CANDIDATES,
)


def test_gitctxignore_path_exclusions():
    """Verify built-in .gitctxignore rules flag vendored, generated, and lock files (D-04)."""
    matcher = GitctxIgnoreMatcher()

    # Should match excluded patterns
    assert matcher.is_excluded("node_modules/express/package.json")
    assert matcher.is_excluded("vendor/bundle/ruby/2.7.0/gems")
    assert matcher.is_excluded("src/api/service_pb2.py")
    assert matcher.is_excluded("src/api/service_pb2_grpc.py")
    assert matcher.is_excluded("package-lock.json")
    assert matcher.is_excluded("subpkg/yarn.lock")
    assert matcher.is_excluded("Cargo.lock")
    assert matcher.is_excluded("poetry.lock")
    assert matcher.is_excluded("Pipfile.lock")
    assert matcher.is_excluded("go.sum")
    assert matcher.is_excluded("static/bundle.min.js")
    assert matcher.is_excluded("assets/styles.min.css")
    assert matcher.is_excluded("dist/index.js")
    assert matcher.is_excluded("build/output.o")

    # Should not match standard source files
    assert not matcher.is_excluded("src/main.rs")
    assert not matcher.is_excluded("eval/profile.py")
    assert not matcher.is_excluded("pkg/handler/auth.go")
    assert not matcher.is_excluded("frontend/src/App.tsx")
    assert not matcher.is_excluded("Cargo.toml")
    assert not matcher.is_excluded("package.json")


def test_bot_author_filtering():
    """Verify bot author detection isolates known bot accounts per D-04."""
    assert is_bot_author("dependabot[bot]")
    assert is_bot_author("renovate[bot]")
    assert is_bot_author("pre-commit-ci[bot]")
    assert is_bot_author("github-actions[bot]")
    assert is_bot_author("snyk-bot")
    assert is_bot_author("John Doe", "support+dependabot@github.com")

    # Real human authors should not be flagged
    assert not is_bot_author("Alice Developer", "alice@example.com")
    assert not is_bot_author("BurntSushi", "burntsushi@example.com")
    assert not is_bot_author("Guido van Rossum", "guido@python.org")


def create_synthetic_git_repo(tmp_path: Path) -> Path:
    """Helper creating a synthetic git repository with known structural properties."""
    def run_git(args: list[str]):
        subprocess.run(["git"] + args, cwd=str(tmp_path), check=True, capture_output=True, text=True)

    run_git(["init", "-b", "main"])
    run_git(["config", "user.name", "Test Author"])
    run_git(["config", "user.email", "author@example.com"])

    # Commit 1: Initial source file
    src_file = tmp_path / "main.py"
    src_file.write_text("print('hello')", encoding="utf-8")
    run_git(["add", "main.py"])
    run_git(["commit", "-m", "Initial commit"])

    # Commit 2: Feature with issue trailer
    src_file.write_text("print('hello world')\n", encoding="utf-8")
    run_git(["add", "main.py"])
    run_git(["commit", "-m", "Implement feature foo\n\nFixes #101\nCloses gh-102"])

    # Commit 3: Squash PR commit
    lock_file = tmp_path / "poetry.lock"
    lock_file.write_text("# lock file\n", encoding="utf-8")
    run_git(["add", "poetry.lock"])
    run_git(["commit", "-m", "Update dependencies (#103)"])

    # Commit 4: Bot commit
    run_git(["config", "user.name", "dependabot[bot]"])
    run_git(["config", "user.email", "dependabot[bot]@users.noreply.github.com"])
    src_file.write_text("print('hello dependabot')\n", encoding="utf-8")
    run_git(["add", "main.py"])
    run_git(["commit", "-m", "Bump security dependency"])

    # Reset author back
    run_git(["config", "user.name", "Test Author"])
    run_git(["config", "user.email", "author@example.com"])

    # Commit 5: Revert commit
    run_git(["commit", "--allow-empty", "-m", 'Revert "Bump security dependency"\n\nThis reverts commit 1234567.'])

    return tmp_path


def test_profile_metrics(tmp_path: Path):
    """Test RepoProfiler computes 11 structural metrics correctly on synthetic repository."""
    repo_dir = create_synthetic_git_repo(tmp_path)
    profiler = RepoProfiler()

    profile = profiler.profile_git_repo(
        repo_path=repo_dir,
        repo_id="test_repo",
        owner="test_org",
        repo_name="test_repo",
        language="python",
    )

    assert profile.commit_count == 5
    assert profile.first_parent_count == 5
    assert profile.merge_commit_count == 0
    assert profile.merge_fraction == 0.0
    assert profile.revert_count == 1
    assert profile.bot_commit_count == 1
    assert profile.bot_fraction == 0.2
    assert profile.squash_count == 1
    assert profile.squash_fraction == 0.2
    # Issues 101, 102 from trailers, 103 from squash title
    assert profile.traceable_issues >= 3
    assert len(profile.head_sha) == 40
    assert profile.target_lang_fraction > 0.0
    assert profile.excluded_fraction > 0.0  # poetry.lock touched


def test_candidate_evaluation_gates():
    """Verify hard gating rules (D-02) accept qualified repos and disqualify violators."""
    # Qualified profile
    qual_profile = RepoProfile(
        repo_id="test_qual",
        owner="org",
        repo="repo",
        language="python",
        commit_count=5000,
        first_parent_count=4500,
        merge_commit_count=500,
        merge_fraction=0.10,
        issue_ref_count=1500,
        issue_ref_fraction=0.30,
        traceable_issues=200,
        squash_count=400,
        squash_fraction=0.08,
        mean_files_per_commit=2.5,
        median_files_per_commit=1.0,
        reformat_storms=5,
        target_lang_fraction=0.95,
        excluded_fraction=0.05,
        revert_count=10,
        rename_count=25,
        bot_commit_count=150,
        bot_fraction=0.03,
        head_sha="0123456789abcdef0123456789abcdef01234567",
        harvest_date="2026-08-16",
    )
    ev_qual = evaluate_candidate(qual_profile)
    assert ev_qual.is_qualified
    assert ev_qual.size_tier == "medium"
    assert len(ev_qual.disqualification_reasons) == 0

    # Disqualified profile: High merge fraction (>40%) and low traceable issues (<150)
    disqual_profile = RepoProfile(
        repo_id="test_disqual",
        owner="org",
        repo="disqual",
        language="typescript",
        commit_count=3000,
        first_parent_count=1500,
        merge_commit_count=1500,
        merge_fraction=0.50,
        issue_ref_count=300,
        issue_ref_fraction=0.10,
        traceable_issues=50,
        squash_count=200,
        squash_fraction=0.067,
        mean_files_per_commit=3.0,
        median_files_per_commit=2.0,
        reformat_storms=10,
        target_lang_fraction=0.90,
        excluded_fraction=0.10,
        revert_count=5,
        rename_count=15,
        bot_commit_count=300,
        bot_fraction=0.10,
        head_sha="abcdef0123456789abcdef0123456789abcdef01",
        harvest_date="2026-08-16",
    )
    ev_disqual = evaluate_candidate(disqual_profile)
    assert not ev_disqual.is_qualified
    assert not ev_disqual.passed_traceable
    assert not ev_disqual.passed_merge_fraction
    assert len(ev_disqual.disqualification_reasons) == 2


def test_balanced_triad_selection():
    """Verify triad selection outputs 3 size tiers across 3 distinct languages (D-02)."""
    candidates = get_preprofiled_candidates()
    assert len(candidates) == 15

    evaluations = [evaluate_candidate(c) for c in candidates]
    triad = select_balanced_triad(evaluations)

    assert len(triad) == 3
    tiers = {t.size_tier for t in triad}
    langs = {t.profile.language for t in triad}

    assert tiers == {"small", "medium", "large"}
    assert len(langs) == 3


def test_generate_corpus_lock_toml_schema():
    """Verify generated corpus.lock.toml adheres to Schema Version 1 and Invariant I6."""
    candidates = get_preprofiled_candidates()
    evaluations = [evaluate_candidate(c) for c in candidates]
    triad = select_balanced_triad(evaluations)

    lock_toml = generate_corpus_lock_toml(triad, script_sha="1234567890abcdef1234567890abcdef12345678")

    assert 'version = 1' in lock_toml
    assert 'profiler_script_sha = "1234567890abcdef1234567890abcdef12345678"' in lock_toml
    assert '[[corpus]]' in lock_toml

    # Verify each locked repo has 40-hex SHA and required attributes
    for t in triad:
        p = t.profile
        assert f'id = "{p.repo_id}"' in lock_toml
        assert f'sha = "{p.head_sha}"' in lock_toml
        assert len(p.head_sha) == 40
        assert f'language = "{p.language}"' in lock_toml
        assert f'size_tier = "{t.size_tier}"' in lock_toml


def test_generate_corpus_selection_md_content():
    """Verify generated corpus_selection.md includes comparison matrix and selection sign-off."""
    candidates = get_preprofiled_candidates()
    evaluations = [evaluate_candidate(c) for c in candidates]
    triad = select_balanced_triad(evaluations)

    md = generate_corpus_selection_md(evaluations, triad)

    assert "# Evaluation Corpus Selection & Profiler Report" in md
    assert "Hard Gating Criteria (D-02)" in md
    assert "| Repository | Language | Commits |" in md
    assert "Pre-Registration Sign-Off" in md
    assert len(evaluations) == 15
    for c in PRESET_CANDIDATES:
        assert c["owner"] in md
        assert c["repo"] in md

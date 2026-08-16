"""Unit and integration tests for the Ground Truth Harvester, trailer parsing, and Wilson Score CI."""

from __future__ import annotations

import json
from pathlib import Path
import pytest
import tempfile

from eval.harvest import (
    ACTION_TRAILER_RE,
    GITHUB_URL_RE,
    SQUASH_SUFFIX_RE,
    REVERT_SUBJECT_RE,
    REVERT_BODY_COMMIT_RE,
    is_bot_author,
    detect_has_traceback,
    detect_has_symbol_hint,
    RawCommitInfo,
    extract_trailers_from_commit,
    extract_revert_info,
    compute_wilson_ci,
    SetARecord,
    RevertRecord,
    write_jsonl_dataset,
    update_manifest_toml,
    generate_synthetic_ground_truth,
    run_precision_audit,
    generate_harvester_audit_md,
)


def test_trailer_extraction():
    """Verify regex patterns extract action trailers, GitHub URLs, and squash PR suffixes (D-05, D-06)."""
    # 1. Action trailers
    commit_1 = RawCommitInfo(
        sha="1111111111111111111111111111111111111111",
        author_name="Dev User",
        author_email="dev@example.com",
        date="2026-08-16T12:00:00Z",
        subject="Fix connection pool race",
        body="Fix connection pool race\n\nFixes #123\nCloses: gh-456\nResolves #789\nRef #101\nAddresses #202\nRelated-to: #303",
    )
    trailers_1 = extract_trailers_from_commit(commit_1)
    issue_nums_1 = [num for num, verb, src in trailers_1]
    assert 123 in issue_nums_1
    assert 456 in issue_nums_1
    assert 789 in issue_nums_1
    assert 101 in issue_nums_1
    assert 202 in issue_nums_1
    assert 303 in issue_nums_1

    # Verify source classification
    for num, verb, src in trailers_1:
        if num in (123, 456, 789):
            assert src == "explicit_trailer"
        elif num in (101, 202, 303):
            assert src == "ref"

    # 2. GitHub URLs
    commit_2 = RawCommitInfo(
        sha="2222222222222222222222222222222222222222",
        author_name="Dev User",
        author_email="dev@example.com",
        date="2026-08-16T12:00:00Z",
        subject="Update auth logic",
        body="Details in https://github.com/psf/requests/issues/999 and https://github.com/psf/requests/pull/1000",
    )
    trailers_2 = extract_trailers_from_commit(commit_2)
    issue_nums_2 = [num for num, verb, src in trailers_2]
    assert 999 in issue_nums_2
    assert 1000 in issue_nums_2

    # 3. Squash PR suffix
    commit_3 = RawCommitInfo(
        sha="3333333333333333333333333333333333333333",
        author_name="Dev User",
        author_email="dev@example.com",
        date="2026-08-16T12:00:00Z",
        subject="Add streaming decompression support (#4821)",
        body="Add streaming decompression support (#4821)",
    )
    trailers_3 = extract_trailers_from_commit(commit_3)
    issue_nums_3 = [num for num, verb, src in trailers_3]
    assert 4821 in issue_nums_3


def test_revert_extraction():
    """Verify revert commit regex extracts revert commit SHA and reverted target SHA (D-06)."""
    commit_rev = RawCommitInfo(
        sha="aaaa1111bbbb2222cccc3333dddd4444eeee5555",
        author_name="Maintainer",
        author_email="maint@example.com",
        date="2026-08-16T14:00:00Z",
        subject='Revert "Fix unicode case-folding in inverted character classes"',
        body='Revert "Fix unicode case-folding in inverted character classes"\n\nThis reverts commit 9999888877776666555544443333222211110000.\nReason: Broke literal prefixes.',
    )
    rev_pair = extract_revert_info(commit_rev)
    assert rev_pair is not None
    revert_sha, reverted_target = rev_pair
    assert revert_sha == "aaaa1111bbbb2222cccc3333dddd4444eeee5555"
    assert reverted_target == "9999888877776666555544443333222211110000"

    # Non-revert commit should return None
    commit_norm = RawCommitInfo(
        sha="bbbb1111cccc2222dddd3333eeee4444ffff5555",
        author_name="Maintainer",
        author_email="maint@example.com",
        date="2026-08-16T14:00:00Z",
        subject="Refactor glob parser logic",
        body="Standard refactoring of regex tokens.",
    )
    assert extract_revert_info(commit_norm) is None


def test_bot_author_filtering():
    """Verify bot authors are excluded from harvested ground truth (D-04, D-06)."""
    assert is_bot_author("dependabot[bot]")
    assert is_bot_author("renovate[bot]")
    assert is_bot_author("pre-commit-ci[bot]")
    assert is_bot_author("github-actions[bot]")
    assert is_bot_author("snyk-bot")
    assert is_bot_author("bors[bot]")
    assert is_bot_author("John Doe", "support+dependabot@github.com")

    # Legitimate human developers
    assert not is_bot_author("BurntSushi", "burntsushi@example.com")
    assert not is_bot_author("Kenneth Reitz", "kenneth@example.com")
    assert not is_bot_author("Alice Smith", "alice@example.com")


def test_query_metadata_detection():
    """Verify detection of has_traceback and has_symbol_hint boolean flags (D-12)."""
    # Traceback examples
    python_tb = "Session crashed with error:\nTraceback (most recent call last):\n  File 'a.py', line 12\nTypeError: expected str"
    rust_tb = "Panic occurred in test runner:\nstack backtrace:\n   0: std::panicking::begin_panic"
    plain_text = "The download speed seems rather slow when fetching large files over HTTPS."

    assert detect_has_traceback(python_tb)
    assert detect_has_traceback(rust_tb)
    assert not detect_has_traceback(plain_text)

    # Symbol hint examples
    code_func = "Calling `session.mount('https://', adapter)` fails to register adapter."
    code_path = "Look at src/handler/router.go for routing definitions."
    code_attr = "Check `Response.content` property caching."
    conceptual = "Why is the retry mechanism backing off exponentially?"

    assert detect_has_symbol_hint(code_func)
    assert detect_has_symbol_hint(code_path)
    assert detect_has_symbol_hint(code_attr)
    assert not detect_has_symbol_hint(conceptual)


def test_wilson_ci():
    """Verify Wilson Score Confidence Interval against statistical reference benchmarks (D-07)."""
    # Reference benchmark: n=30, k=26 (p = 0.8667)
    # Expected: p ~ 0.8667, lower bound in [0.70, 0.72], upper bound in [0.93, 0.96]
    p, lower, upper = compute_wilson_ci(k=26, n=30, confidence=0.95)
    assert pytest.approx(p, abs=1e-4) == 26 / 30
    assert 0.70 <= lower <= 0.72
    assert 0.93 <= upper <= 0.96
    assert lower < p < upper

    # Boundary cases
    p_zero, lower_zero, upper_zero = compute_wilson_ci(k=0, n=30, confidence=0.95)
    assert p_zero == 0.0
    assert lower_zero == 0.0
    assert upper_zero > 0.0

    p_full, lower_full, upper_full = compute_wilson_ci(k=30, n=30, confidence=0.95)
    assert p_full == 1.0
    assert lower_full < 1.0
    assert upper_full == 1.0

    # Empty sample
    p_empty, lower_empty, upper_empty = compute_wilson_ci(k=0, n=0)
    assert p_empty == 0.0
    assert lower_empty == 0.0
    assert upper_empty == 0.0


def test_dataset_serialization_and_manifest(tmp_path: Path):
    """Verify JSONL dataset writing, BLAKE3 and SHA256 integrity manifest generation (D-08)."""
    set_a, reverts = generate_synthetic_ground_truth()
    assert len(set_a) > 0
    assert len(reverts) > 0

    set_a_file = tmp_path / "set_a_issues.jsonl"
    reverts_file = tmp_path / "revert_pairs.jsonl"
    manifest_file = tmp_path / "MANIFEST.toml"

    count_a, b3_a, s256_a = write_jsonl_dataset(set_a_file, set_a)
    count_r, b3_r, s256_r = write_jsonl_dataset(reverts_file, reverts)

    assert count_a == len(set_a)
    assert count_r == len(reverts)
    assert len(b3_a) == 64
    assert len(s256_a) == 64

    # Verify JSONL lines parse
    with open(set_a_file, "r", encoding="utf-8") as f:
        for line in f:
            rec = json.loads(line)
            assert "qid" in rec
            assert "query_set" in rec
            assert rec["query_set"] == "A"
            assert "labels" in rec
            assert len(rec["labels"]) <= 3  # Label capping <= 3

    # Generate and verify MANIFEST.toml
    manifest_content = update_manifest_toml(
        manifest_file,
        {
            "set_a_issues.jsonl": (count_a, b3_a, s256_a),
            "revert_pairs.jsonl": (count_r, b3_r, s256_r),
        },
    )

    assert 'version = 1' in manifest_content
    assert '[files."set_a_issues.jsonl"]' in manifest_content
    assert f'count = {count_a}' in manifest_content
    assert f'blake3 = "{b3_a}"' in manifest_content
    assert f'sha256 = "{s256_a}"' in manifest_content


def test_precision_audit_execution(tmp_path: Path):
    """Verify precision audit CLI execution and markdown report generation (D-07)."""
    set_a, _ = generate_synthetic_ground_truth()
    audit_md_path = tmp_path / "harvester_audit.md"

    passed, p_est, ci_l, ci_u, records = run_precision_audit(
        records=set_a,
        sample_size=30,
        batch_file=None,
        non_interactive=True,
        audit_md_path=audit_md_path,
        auditor_id="Test-Auditor",
    )

    assert passed
    assert p_est >= 0.85
    assert ci_l >= 0.70
    assert len(records) == 30
    assert audit_md_path.exists()

    md_content = audit_md_path.read_text(encoding="utf-8")
    assert "# Ground Truth Harvester Precision Audit Report" in md_content
    assert "Pre-Registered Decision Gate:" in md_content
    assert "**PASS**" in md_content
    assert "Test-Auditor" in md_content
    assert "| # | QID | Repository | Commit SHA | Judgment | Auditor Notes |" in md_content

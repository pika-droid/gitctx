"""Tests and validation suite for hand-authored Set B why-query dataset."""

from __future__ import annotations

import json
import re
from pathlib import Path

import pytest

HEX_SHA_REGEX = re.compile(r"^[0-9a-f]{40}$")


@pytest.fixture
def set_b_records() -> list[dict]:
    dataset_path = Path(__file__).resolve().parent.parent / "ground_truth" / "set_b_why.jsonl"
    assert dataset_path.exists(), f"Missing Set B dataset file: {dataset_path}"
    records = []
    with open(dataset_path, "r", encoding="utf-8") as f:
        for line_num, line in enumerate(f, start=1):
            line = line.strip()
            if not line:
                continue
            try:
                record = json.loads(line)
                records.append(record)
            except json.JSONDecodeError as e:
                pytest.fail(f"Invalid JSON at line {line_num}: {e}")
    return records


def test_set_b_total_count(set_b_records: list[dict]) -> None:
    """Ensure total query count is between 25 and 30 per D-13, D-16."""
    assert 25 <= len(set_b_records) <= 30, f"Expected 25..30 records, found {len(set_b_records)}"


def test_set_b_repo_distribution(set_b_records: list[dict]) -> None:
    """Ensure all 3 locked repositories have 8-10 queries each per D-16."""
    repo_counts = {}
    for r in set_b_records:
        repo = r["repo"]
        repo_counts[repo] = repo_counts.get(repo, 0) + 1

    expected_repos = {"psf/requests", "tokio-rs/tokio", "gin-gonic/gin"}
    assert set(repo_counts.keys()) == expected_repos, f"Unexpected repo set: {set(repo_counts.keys())}"

    for repo, count in repo_counts.items():
        assert 8 <= count <= 10, f"Repo '{repo}' has {count} queries (expected 8..10)"


def test_set_b_category_distribution(set_b_records: list[dict]) -> None:
    """Ensure all 4 categories are represented with at least 5 queries each per D-13."""
    cat_counts = {}
    for r in set_b_records:
        cat = r["category"]
        cat_counts[cat] = cat_counts.get(cat, 0) + 1

    expected_cats = {
        "architectural_decision",
        "bug_root_cause",
        "api_evolution",
        "performance_motivation",
    }
    assert set(cat_counts.keys()) == expected_cats, f"Unexpected category set: {set(cat_counts.keys())}"

    for cat, count in cat_counts.items():
        assert count >= 5, f"Category '{cat}' has {count} queries (expected >= 5)"


def test_set_b_dev_test_split(set_b_records: list[dict]) -> None:
    """Ensure 60/40 Dev/Test split ratio (55%-65% dev) per D-14, D-16."""
    dev_count = sum(1 for r in set_b_records if r["split"] == "dev")
    test_count = sum(1 for r in set_b_records if r["split"] == "test")

    assert dev_count + test_count == len(set_b_records)
    dev_ratio = dev_count / len(set_b_records)
    assert 0.55 <= dev_ratio <= 0.65, f"Dev ratio is {dev_ratio:.3f} (expected 0.55..0.65)"


def test_set_b_record_schema_and_provenance(set_b_records: list[dict]) -> None:
    """Assert rich provenance and valid fields on all records per D-14."""
    seen_qids = set()
    for idx, r in enumerate(set_b_records):
        qid = r.get("qid")
        assert qid, f"Record {idx} missing 'qid'"
        assert qid not in seen_qids, f"Duplicate qid: {qid}"
        seen_qids.add(qid)

        assert r.get("query_set") == "B"
        assert r.get("repo") in {"psf/requests", "tokio-rs/tokio", "gin-gonic/gin"}
        assert r.get("category") in {
            "architectural_decision",
            "bug_root_cause",
            "api_evolution",
            "performance_motivation",
        }

        query = r.get("query", "")
        assert len(query.strip()) > 10, f"Record {qid} has too short query: {query!r}"

        disc = r.get("discussion_source", "")
        assert disc.startswith("https://github.com/"), f"Record {qid} invalid discussion_source: {disc!r}"

        rationale = r.get("gold_rationale", "")
        assert len(rationale.strip()) > 20, f"Record {qid} rationale too short"

        gold_commits = r.get("gold_commits")
        assert isinstance(gold_commits, list) and len(gold_commits) >= 1, f"Record {qid} missing gold_commits"
        for gc in gold_commits:
            commit_sha = gc.get("commit", "")
            assert HEX_SHA_REGEX.match(commit_sha), f"Record {qid} invalid commit SHA: {commit_sha}"
            assert gc.get("confidence") in {"high", "medium"}

        assert r.get("split") in {"dev", "test"}
        assert isinstance(r.get("has_symbol_hint"), bool)
        assert r.get("curator_date")

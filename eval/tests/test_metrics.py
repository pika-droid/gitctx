"""Tests for evaluation metrics: Recall@K, MRR, bootstrap CIs."""

from __future__ import annotations

import pytest

from eval.ablation.metrics import (
    BootstrapCIResult,
    QueryResult,
    aggregate_metrics,
    compute_query_metrics,
    deduplicate_by_commit,
    mrr,
    paired_bootstrap_ci,
    recall_at_k,
)


# ──────────────────────────────────────────────────────────────────────────────
# Recall@K
# ──────────────────────────────────────────────────────────────────────────────


class TestRecallAtK:
    def test_perfect_recall(self):
        retrieved = ["a", "b", "c", "d", "e"]
        relevant = {"a", "b", "c"}
        assert recall_at_k(retrieved, relevant, 5) == 1.0

    def test_partial_recall(self):
        retrieved = ["a", "x", "b", "y", "z"]
        relevant = {"a", "b", "c"}
        # Found 2 out of 3 in top 5
        assert recall_at_k(retrieved, relevant, 5) == pytest.approx(2 / 3)

    def test_zero_recall(self):
        retrieved = ["x", "y", "z"]
        relevant = {"a", "b"}
        assert recall_at_k(retrieved, relevant, 3) == 0.0

    def test_empty_relevant(self):
        retrieved = ["a", "b", "c"]
        assert recall_at_k(retrieved, set(), 3) == 0.0

    def test_empty_retrieved(self):
        relevant = {"a", "b"}
        assert recall_at_k([], relevant, 5) == 0.0

    def test_k_larger_than_retrieved(self):
        retrieved = ["a", "b"]
        relevant = {"a", "b", "c"}
        assert recall_at_k(retrieved, relevant, 10) == pytest.approx(2 / 3)

    def test_k_smaller_than_relevant(self):
        retrieved = ["a", "b", "c", "d", "e"]
        relevant = {"a", "b", "c", "d"}
        # Only look at top 2
        assert recall_at_k(retrieved, relevant, 2) == pytest.approx(2 / 4)


# ──────────────────────────────────────────────────────────────────────────────
# MRR
# ──────────────────────────────────────────────────────────────────────────────


class TestMRR:
    def test_first_position(self):
        retrieved = ["a", "b", "c"]
        relevant = {"a"}
        assert mrr(retrieved, relevant) == 1.0

    def test_second_position(self):
        retrieved = ["x", "a", "b"]
        relevant = {"a"}
        assert mrr(retrieved, relevant) == 0.5

    def test_third_position(self):
        retrieved = ["x", "y", "a"]
        relevant = {"a"}
        assert mrr(retrieved, relevant) == pytest.approx(1 / 3)

    def test_no_relevant_found(self):
        retrieved = ["x", "y", "z"]
        relevant = {"a"}
        assert mrr(retrieved, relevant) == 0.0

    def test_empty_relevant(self):
        assert mrr(["a", "b"], set()) == 0.0

    def test_multiple_relevant_returns_first(self):
        retrieved = ["x", "a", "b"]
        relevant = {"a", "b"}
        # First relevant at position 2
        assert mrr(retrieved, relevant) == 0.5


# ──────────────────────────────────────────────────────────────────────────────
# Deduplication
# ──────────────────────────────────────────────────────────────────────────────


class TestDeduplication:
    def test_no_duplicates(self):
        doc_ids = ["repo:aaa", "repo:bbb", "repo:ccc"]
        assert deduplicate_by_commit(doc_ids) == doc_ids

    def test_removes_duplicates(self):
        doc_ids = ["repo:aaa", "repo:bbb", "repo:aaa", "repo:ccc"]
        result = deduplicate_by_commit(doc_ids)
        assert result == ["repo:aaa", "repo:bbb", "repo:ccc"]

    def test_preserves_order(self):
        doc_ids = ["repo:ccc", "repo:aaa", "repo:bbb"]
        assert deduplicate_by_commit(doc_ids) == doc_ids

    def test_empty_list(self):
        assert deduplicate_by_commit([]) == []


# ──────────────────────────────────────────────────────────────────────────────
# Query Metrics
# ──────────────────────────────────────────────────────────────────────────────


class TestQueryMetrics:
    def test_compute_query_metrics(self):
        retrieved = ["repo:a", "repo:b", "repo:c", "repo:d", "repo:e"]
        relevant = {"repo:a", "repo:c"}
        qr = compute_query_metrics(retrieved, relevant, "q1")
        assert qr.query_id == "q1"
        assert qr.recall_at_5 == 1.0
        assert qr.mrr == 1.0

    def test_aggregate_metrics(self):
        results = [
            QueryResult("q1", [], set(), 1.0, 1.0, 1.0, 1.0),
            QueryResult("q2", [], set(), 0.0, 0.5, 1.0, 0.5),
        ]
        agg = aggregate_metrics(results)
        assert agg["recall@5"] == pytest.approx(0.5)
        assert agg["recall@10"] == pytest.approx(0.75)
        assert agg["mrr"] == pytest.approx(0.75)

    def test_aggregate_empty(self):
        agg = aggregate_metrics([])
        assert agg["recall@10"] == 0.0


# ──────────────────────────────────────────────────────────────────────────────
# Bootstrap CI
# ──────────────────────────────────────────────────────────────────────────────


class TestBootstrapCI:
    def test_identical_scores_no_significance(self):
        scores = [0.5, 0.6, 0.7, 0.8, 0.5, 0.6, 0.7, 0.8]
        ci = paired_bootstrap_ci(scores, scores, n_iterations=1000)
        assert ci.delta_mean == pytest.approx(0.0)
        # CI should include 0
        assert ci.ci_lower <= 0 <= ci.ci_upper
        assert ci.significant is False

    def test_clearly_different_scores(self):
        scores_a = [0.9, 0.95, 0.85, 0.92, 0.88, 0.91, 0.93, 0.87, 0.9, 0.89]
        scores_b = [0.1, 0.15, 0.05, 0.12, 0.08, 0.11, 0.13, 0.07, 0.1, 0.09]
        ci = paired_bootstrap_ci(scores_a, scores_b, n_iterations=1000)
        assert ci.delta_mean > 0.5
        assert ci.significant is True
        assert ci.ci_lower > 0

    def test_unequal_lengths_raises(self):
        with pytest.raises(ValueError, match="equal length"):
            paired_bootstrap_ci([0.5, 0.6], [0.5])

    def test_empty_scores_raises(self):
        with pytest.raises(ValueError, match="not be empty"):
            paired_bootstrap_ci([], [])

    def test_reproducibility(self):
        scores_a = [0.5, 0.6, 0.7, 0.8]
        scores_b = [0.3, 0.4, 0.5, 0.6]
        ci1 = paired_bootstrap_ci(scores_a, scores_b, seed=42)
        ci2 = paired_bootstrap_ci(scores_a, scores_b, seed=42)
        assert ci1.delta_mean == ci2.delta_mean
        assert ci1.ci_lower == ci2.ci_lower
        assert ci1.ci_upper == ci2.ci_upper

"""Integration test: runs a small ablation grid on synthetic data."""

from __future__ import annotations

import json
import tempfile
from pathlib import Path

import pytest

from eval.ablation.gate import evaluate_gate
from eval.ablation.metrics import paired_bootstrap_ci
from eval.ablation.runner import (
    GridResult,
    generate_synthetic_documents,
    generate_synthetic_queries,
    run_cell,
    run_grid,
    save_grid_results,
)
from eval.ablation.report import generate_report


# ──────────────────────────────────────────────────────────────────────────────
# Synthetic Data Generation
# ──────────────────────────────────────────────────────────────────────────────


class TestSyntheticData:
    def test_generate_documents(self):
        docs = generate_synthetic_documents(num_docs=10)
        assert len(docs) == 10
        for doc in docs:
            assert "hash" in doc
            assert "message" in doc
            assert "diff" in doc
            assert "author" in doc
            assert len(doc["hash"]) > 0

    def test_generate_queries(self):
        docs = generate_synthetic_documents(num_docs=10)
        queries = generate_synthetic_queries(docs, num_queries=5)
        assert len(queries) == 5
        for q in queries:
            assert "qid" in q
            assert "query" in q
            assert "gold_commits" in q
            assert isinstance(q["gold_commits"], set)
            assert len(q["gold_commits"]) > 0

    def test_reproducibility(self):
        docs1 = generate_synthetic_documents(num_docs=10, seed=42)
        docs2 = generate_synthetic_documents(num_docs=10, seed=42)
        assert docs1 == docs2


# ──────────────────────────────────────────────────────────────────────────────
# Single Cell Execution
# ──────────────────────────────────────────────────────────────────────────────


def BM25Available() -> bool:
    """Check if rank_bm25 is available."""
    try:
        import rank_bm25  # noqa: F401
        return True
    except ImportError:
        return False


class TestRunCell:
    def test_keyword_d1(self):
        docs = generate_synthetic_documents(num_docs=20)
        queries = generate_synthetic_queries(docs, num_queries=5)
        result = run_cell("D1", "R1", docs, queries)
        assert result.representation == "D1"
        assert result.retriever == "R1"
        assert "recall@10" in result.metrics
        assert "mrr" in result.metrics
        assert result.elapsed_seconds >= 0

    def test_keyword_d4(self):
        docs = generate_synthetic_documents(num_docs=20)
        queries = generate_synthetic_queries(docs, num_queries=5)
        result = run_cell("D4", "R1", docs, queries)
        assert result.representation == "D4"
        assert "recall@10" in result.metrics

    @pytest.mark.skipif(
        not BM25Available(),
        reason="rank_bm25 not installed",
    )
    def test_bm25_d3(self):
        docs = generate_synthetic_documents(num_docs=20)
        queries = generate_synthetic_queries(docs, num_queries=5)
        result = run_cell("D3", "R2", docs, queries)
        assert result.representation == "D3"
        assert result.retriever == "R2"
        assert "recall@10" in result.metrics



# ──────────────────────────────────────────────────────────────────────────────
# Full Grid
# ──────────────────────────────────────────────────────────────────────────────


class TestRunGrid:
    def test_small_grid(self):
        """Run a minimal grid with R1 only (always available)."""
        docs = generate_synthetic_documents(num_docs=15)
        queries = generate_synthetic_queries(docs, num_queries=5)
        result = run_grid(
            documents=docs,
            queries=queries,
            representations=["D1", "D4"],
            retrievers=["R1"],
        )
        assert isinstance(result, GridResult)
        assert len(result.cells) == 2  # 2 reps × 1 retriever
        assert result.total_documents == 15
        assert result.total_queries == 5

    def test_save_and_load_results(self):
        docs = generate_synthetic_documents(num_docs=10)
        queries = generate_synthetic_queries(docs, num_queries=3)
        result = run_grid(
            documents=docs,
            queries=queries,
            representations=["D1", "D4"],
            retrievers=["R1"],
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            out_path = save_grid_results(result, Path(tmpdir))
            assert out_path.exists()

            with open(out_path) as f:
                loaded = json.load(f)

            assert loaded["total_documents"] == 10
            assert loaded["total_queries"] == 3
            assert len(loaded["cells"]) == 2


# ──────────────────────────────────────────────────────────────────────────────
# Decision Gate
# ──────────────────────────────────────────────────────────────────────────────


class TestGateIntegration:
    def test_gate_with_synthetic_data(self):
        """Test gate evaluation with known synthetic scores."""
        grid_results = {
            "D1": {"R1": {"recall@10": 0.30, "recall@5": 0.20, "recall@20": 0.40, "mrr": 0.25}},
            "D3": {"R1": {"recall@10": 0.35, "recall@5": 0.25, "recall@20": 0.45, "mrr": 0.30}},
            "D4": {"R1": {"recall@10": 0.50, "recall@5": 0.40, "recall@20": 0.60, "mrr": 0.45}},
            "D5": {"R1": {"recall@10": 0.45, "recall@5": 0.35, "recall@20": 0.55, "mrr": 0.40}},
        }

        per_query_scores = {
            "D3+R1": {"recall@10": [0.3, 0.4, 0.3, 0.4, 0.3]},
            "D4+R1": {"recall@10": [0.5, 0.6, 0.4, 0.5, 0.5]},
        }

        gate = evaluate_gate(grid_results, per_query_scores)
        assert gate.best_template_config == "D4+R1"
        assert gate.best_baseline_config == "D3+R1"
        assert gate.delta_recall_10 == pytest.approx(0.15)

    def test_gate_fails_when_no_improvement(self):
        """Gate should fail if templates don't beat baselines."""
        grid_results = {
            "D1": {"R1": {"recall@10": 0.50}},
            "D4": {"R1": {"recall@10": 0.50}},
        }
        per_query_scores = {
            "D1+R1": {"recall@10": [0.5, 0.5, 0.5]},
            "D4+R1": {"recall@10": [0.5, 0.5, 0.5]},
        }

        gate = evaluate_gate(grid_results, per_query_scores)
        assert gate.passed is False
        assert gate.delta_recall_10 == pytest.approx(0.0)


# ──────────────────────────────────────────────────────────────────────────────
# Report Generation
# ──────────────────────────────────────────────────────────────────────────────


class TestReportGeneration:
    def test_report_from_grid(self):
        """End-to-end: run grid → save → generate report."""
        docs = generate_synthetic_documents(num_docs=10)
        queries = generate_synthetic_queries(docs, num_queries=3)
        result = run_grid(
            documents=docs,
            queries=queries,
            representations=["D1", "D4"],
            retrievers=["R1"],
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            tmppath = Path(tmpdir)
            json_path = save_grid_results(result, tmppath)
            report_path = tmppath / "ABLATION_REPORT.md"
            report = generate_report(
                grid_results_path=json_path,
                output_path=report_path,
            )

            assert report_path.exists()
            assert "Ablation Report" in report
            assert "Decision Gate" in report
            assert "Recall@10" in report

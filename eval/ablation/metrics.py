"""Evaluation metrics: Recall@K, MRR, and paired bootstrap confidence intervals.

Implements the statistical evaluation harness per THESIS-03:
- Recall@K (K = 5, 10, 20)
- Mean Reciprocal Rank (MRR)
- Paired bootstrap 95% confidence intervals
- Max-pooling over function-touches with duplicate commit deduplication
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Dict, List, Optional, Set, Tuple

import numpy as np

from eval.ablation.config import BOOTSTRAP_ALPHA, BOOTSTRAP_ITERATIONS, RANDOM_SEED


@dataclass(frozen=True)
class MetricResult:
    """Result of a single metric computation."""

    metric_name: str
    value: float
    k: Optional[int] = None  # For Recall@K


@dataclass(frozen=True)
class BootstrapCIResult:
    """Result of paired bootstrap confidence interval analysis."""

    metric_name: str
    delta_mean: float
    ci_lower: float
    ci_upper: float
    alpha: float
    n_iterations: int
    significant: bool  # True if CI excludes 0


@dataclass(frozen=True)
class QueryResult:
    """Per-query evaluation result for a single (D, R) combination."""

    query_id: str
    retrieved_ids: List[str]
    relevant_ids: Set[str]
    recall_at_5: float
    recall_at_10: float
    recall_at_20: float
    mrr: float


# ──────────────────────────────────────────────────────────────────────────────
# Core Metrics
# ──────────────────────────────────────────────────────────────────────────────


def recall_at_k(retrieved: List[str], relevant: Set[str], k: int) -> float:
    """Compute Recall@K: fraction of relevant documents found in top-K results.

    Args:
        retrieved: Ordered list of retrieved document IDs (most relevant first).
        relevant: Set of ground-truth relevant document IDs.
        k: Cutoff rank.

    Returns:
        Recall@K as a float in [0, 1]. Returns 0.0 if no relevant docs exist.
    """
    if not relevant:
        return 0.0
    top_k = set(retrieved[:k])
    found = top_k & relevant
    return len(found) / len(relevant)


def mrr(retrieved: List[str], relevant: Set[str]) -> float:
    """Compute Mean Reciprocal Rank for a single query.

    Args:
        retrieved: Ordered list of retrieved document IDs (most relevant first).
        relevant: Set of ground-truth relevant document IDs.

    Returns:
        Reciprocal rank of the first relevant document, or 0.0 if none found.
    """
    if not relevant:
        return 0.0
    for rank, doc_id in enumerate(retrieved, start=1):
        if doc_id in relevant:
            return 1.0 / rank
    return 0.0


def deduplicate_by_commit(doc_ids: List[str]) -> List[str]:
    """Deduplicate retrieved doc_ids by commit hash.

    Doc IDs have format "repo:commit_hash". When the same commit appears
    under different function touches, keep only the first occurrence.

    Args:
        doc_ids: List of document IDs potentially containing duplicates.

    Returns:
        Deduplicated list preserving original order.
    """
    seen_commits: Set[str] = set()
    deduped: List[str] = []
    for doc_id in doc_ids:
        # Extract commit hash from "repo:hash" format
        commit = doc_id.split(":")[-1] if ":" in doc_id else doc_id
        if commit not in seen_commits:
            seen_commits.add(commit)
            deduped.append(doc_id)
    return deduped


# ──────────────────────────────────────────────────────────────────────────────
# Aggregation
# ──────────────────────────────────────────────────────────────────────────────


def compute_query_metrics(
    retrieved: List[str],
    relevant: Set[str],
    query_id: str,
) -> QueryResult:
    """Compute all metrics for a single query.

    Applies commit deduplication before metric computation.

    Args:
        retrieved: Raw retrieved doc IDs.
        relevant: Ground-truth relevant doc IDs.
        query_id: Unique query identifier.

    Returns:
        QueryResult with all metric values.
    """
    deduped = deduplicate_by_commit(retrieved)
    return QueryResult(
        query_id=query_id,
        retrieved_ids=deduped,
        relevant_ids=relevant,
        recall_at_5=recall_at_k(deduped, relevant, 5),
        recall_at_10=recall_at_k(deduped, relevant, 10),
        recall_at_20=recall_at_k(deduped, relevant, 20),
        mrr=mrr(deduped, relevant),
    )


def aggregate_metrics(results: List[QueryResult]) -> Dict[str, float]:
    """Aggregate per-query metrics into mean scores.

    Args:
        results: List of per-query results.

    Returns:
        Dict mapping metric names to mean values.
    """
    if not results:
        return {
            "recall@5": 0.0,
            "recall@10": 0.0,
            "recall@20": 0.0,
            "mrr": 0.0,
        }
    n = len(results)
    return {
        "recall@5": sum(r.recall_at_5 for r in results) / n,
        "recall@10": sum(r.recall_at_10 for r in results) / n,
        "recall@20": sum(r.recall_at_20 for r in results) / n,
        "mrr": sum(r.mrr for r in results) / n,
    }


# ──────────────────────────────────────────────────────────────────────────────
# Paired Bootstrap Confidence Intervals
# ──────────────────────────────────────────────────────────────────────────────


def paired_bootstrap_ci(
    scores_a: List[float],
    scores_b: List[float],
    n_iterations: int = BOOTSTRAP_ITERATIONS,
    alpha: float = BOOTSTRAP_ALPHA,
    seed: int = RANDOM_SEED,
) -> BootstrapCIResult:
    """Compute paired bootstrap confidence interval for the difference in means.

    Tests whether system A significantly differs from system B.
    Delta = mean(scores_a) - mean(scores_b).
    A positive delta means A outperforms B.

    Args:
        scores_a: Per-query metric scores for system A.
        scores_b: Per-query metric scores for system B.
        n_iterations: Number of bootstrap iterations.
        alpha: Significance level (0.05 for 95% CI).
        seed: Random seed for reproducibility.

    Returns:
        BootstrapCIResult with delta, CI bounds, and significance.

    Raises:
        ValueError: If score lists have different lengths or are empty.
    """
    if len(scores_a) != len(scores_b):
        raise ValueError(
            f"Score lists must have equal length: {len(scores_a)} != {len(scores_b)}"
        )
    if not scores_a:
        raise ValueError("Score lists must not be empty")

    rng = np.random.RandomState(seed)
    n = len(scores_a)
    arr_a = np.array(scores_a, dtype=np.float64)
    arr_b = np.array(scores_b, dtype=np.float64)

    # Compute observed delta
    observed_delta = float(np.mean(arr_a) - np.mean(arr_b))

    # Bootstrap resampling
    deltas = np.empty(n_iterations, dtype=np.float64)
    for i in range(n_iterations):
        indices = rng.randint(0, n, size=n)
        boot_a = arr_a[indices]
        boot_b = arr_b[indices]
        deltas[i] = np.mean(boot_a) - np.mean(boot_b)

    # Compute CI bounds (percentile method)
    lower_pct = (alpha / 2) * 100
    upper_pct = (1 - alpha / 2) * 100
    ci_lower = float(np.percentile(deltas, lower_pct))
    ci_upper = float(np.percentile(deltas, upper_pct))

    # Significant if CI excludes 0
    significant = (ci_lower > 0) or (ci_upper < 0)

    return BootstrapCIResult(
        metric_name="delta",
        delta_mean=observed_delta,
        ci_lower=ci_lower,
        ci_upper=ci_upper,
        alpha=alpha,
        n_iterations=n_iterations,
        significant=significant,
    )

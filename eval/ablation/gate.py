"""Decision gate evaluation per THESIS-04.

Evaluates whether the templated pseudo-document representations (D4/D5)
significantly outperform baseline representations (D1/D2/D3) on Recall@10
by at least 0.05, with 95% CI excluding 0.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Dict, List, Optional, Tuple

from eval.ablation.config import GATE_DELTA_THRESHOLD
from eval.ablation.metrics import BootstrapCIResult, paired_bootstrap_ci


@dataclass(frozen=True)
class GateDecision:
    """Formal Phase 0 decision gate result."""

    passed: bool
    reasoning: str
    best_template_config: str  # e.g., "D4+R5"
    best_baseline_config: str  # e.g., "D3+R2"
    template_recall_10: float
    baseline_recall_10: float
    delta_recall_10: float
    bootstrap_ci: BootstrapCIResult
    threshold: float


def evaluate_gate(
    grid_results: Dict[str, Dict[str, Dict[str, float]]],
    per_query_scores: Dict[str, Dict[str, List[float]]],
) -> GateDecision:
    """Evaluate the Phase 0 decision gate.

    The gate PASSES if:
    1. ΔRecall@10 ≥ GATE_DELTA_THRESHOLD (0.05)
    2. The 95% bootstrap CI excludes 0 (statistically significant)

    Args:
        grid_results: Nested dict [representation][retriever] -> metrics dict.
            Each metrics dict has keys like "recall@10", "mrr", etc.
        per_query_scores: Nested dict [config_key][metric] -> list of per-query scores.
            config_key is "D?+R?", metric is "recall@10".

    Returns:
        GateDecision with pass/fail result and detailed reasoning.
    """
    # Find best template config (D4 or D5)
    template_reps = ["D4", "D5"]
    baseline_reps = ["D1", "D2", "D3"]

    best_template_score = -1.0
    best_template_config = ""
    for rep in template_reps:
        if rep not in grid_results:
            continue
        for ret, metrics in grid_results[rep].items():
            r10 = metrics.get("recall@10", 0.0)
            if r10 > best_template_score:
                best_template_score = r10
                best_template_config = f"{rep}+{ret}"

    # Find best baseline config (D1, D2, or D3)
    best_baseline_score = -1.0
    best_baseline_config = ""
    for rep in baseline_reps:
        if rep not in grid_results:
            continue
        for ret, metrics in grid_results[rep].items():
            r10 = metrics.get("recall@10", 0.0)
            if r10 > best_baseline_score:
                best_baseline_score = r10
                best_baseline_config = f"{rep}+{ret}"

    # Compute delta
    delta = best_template_score - best_baseline_score

    # Run bootstrap CI if we have per-query scores
    if (
        best_template_config in per_query_scores
        and best_baseline_config in per_query_scores
        and "recall@10" in per_query_scores[best_template_config]
        and "recall@10" in per_query_scores[best_baseline_config]
    ):
        template_scores = per_query_scores[best_template_config]["recall@10"]
        baseline_scores = per_query_scores[best_baseline_config]["recall@10"]

        if len(template_scores) == len(baseline_scores) and len(template_scores) > 0:
            ci = paired_bootstrap_ci(template_scores, baseline_scores)
        else:
            ci = BootstrapCIResult(
                metric_name="delta",
                delta_mean=delta,
                ci_lower=delta - 0.01,
                ci_upper=delta + 0.01,
                alpha=0.05,
                n_iterations=0,
                significant=False,
            )
    else:
        ci = BootstrapCIResult(
            metric_name="delta",
            delta_mean=delta,
            ci_lower=delta - 0.01,
            ci_upper=delta + 0.01,
            alpha=0.05,
            n_iterations=0,
            significant=False,
        )

    # Decision logic
    threshold_met = delta >= GATE_DELTA_THRESHOLD
    ci_significant = ci.significant and ci.ci_lower > 0

    passed = threshold_met and ci_significant

    # Build reasoning
    reasons = []
    reasons.append(
        f"Best template: {best_template_config} "
        f"(Recall@10 = {best_template_score:.4f})"
    )
    reasons.append(
        f"Best baseline: {best_baseline_config} "
        f"(Recall@10 = {best_baseline_score:.4f})"
    )
    reasons.append(f"ΔRecall@10 = {delta:.4f} (threshold: ≥ {GATE_DELTA_THRESHOLD})")
    reasons.append(
        f"Bootstrap 95% CI: [{ci.ci_lower:.4f}, {ci.ci_upper:.4f}]"
    )

    if passed:
        reasons.append(
            "GATE PASSED: Template representation significantly outperforms baselines. "
            "Proceed with Rust retrieval implementation."
        )
    else:
        if not threshold_met:
            reasons.append(
                f"GATE FAILED: ΔRecall@10 ({delta:.4f}) < threshold ({GATE_DELTA_THRESHOLD}). "
                "Template does not sufficiently improve over baselines."
            )
        elif not ci_significant:
            reasons.append(
                "GATE FAILED: Bootstrap CI includes 0 — improvement is not statistically significant."
            )

    return GateDecision(
        passed=passed,
        reasoning="\n".join(reasons),
        best_template_config=best_template_config,
        best_baseline_config=best_baseline_config,
        template_recall_10=best_template_score,
        baseline_recall_10=best_baseline_score,
        delta_recall_10=delta,
        bootstrap_ci=ci,
        threshold=GATE_DELTA_THRESHOLD,
    )

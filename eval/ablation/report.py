"""Markdown report generator for ablation results.

Produces eval/ABLATION_REPORT.md with:
- Full D×R grid table (Recall@5/10/20, MRR)
- Bootstrap CI analysis
- Per-repo breakdown
- Top miss adjudication
- Gate decision with formal reasoning
"""

from __future__ import annotations

import json
import logging
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional

from eval.ablation.config import (
    ABLATION_REPORT_PATH,
    ABLATION_RESULTS_DIR,
    GATE_DELTA_THRESHOLD,
)
from eval.ablation.gate import GateDecision, evaluate_gate
from eval.ablation.metrics import paired_bootstrap_ci

logger = logging.getLogger(__name__)


def generate_report(
    grid_results_path: Optional[Path] = None,
    output_path: Optional[Path] = None,
) -> str:
    """Generate the ablation report from saved grid results.

    Args:
        grid_results_path: Path to grid_results.json. Defaults to standard location.
        output_path: Path for the output markdown. Defaults to ABLATION_REPORT_PATH.

    Returns:
        The report content as a string.
    """
    results_path = grid_results_path or ABLATION_RESULTS_DIR / "grid_results.json"
    out_path = output_path or ABLATION_REPORT_PATH

    with open(results_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    cells = data["cells"]
    reps = data["representations_used"]
    rets = data["retrievers_used"]

    # Build grid lookup
    grid: Dict[str, Dict[str, Dict[str, float]]] = {}
    per_query_scores: Dict[str, Dict[str, List[float]]] = {}

    for cell in cells:
        rep = cell["representation"]
        ret = cell["retriever"]
        grid.setdefault(rep, {})[ret] = cell["metrics"]

        config_key = f"{rep}+{ret}"
        per_query_scores[config_key] = {
            "recall@5": [q.get("recall@5", 0) for q in cell.get("per_query", [])],
            "recall@10": [q.get("recall@10", 0) for q in cell.get("per_query", [])],
            "recall@20": [q.get("recall@20", 0) for q in cell.get("per_query", [])],
            "mrr": [q.get("mrr", 0) for q in cell.get("per_query", [])],
        }

    # Evaluate decision gate
    gate = evaluate_gate(grid, per_query_scores)

    # Build report
    sections = []

    # Header
    sections.append("# Phase 3: Retrieval Thesis Validation — Ablation Report\n")
    sections.append(f"**Generated:** {datetime.now(timezone.utc).strftime('%Y-%m-%d %H:%M UTC')}\n")
    sections.append(f"**Documents indexed:** {data['total_documents']}")
    sections.append(f"**Queries evaluated:** {data['total_queries']}")
    sections.append(f"**Random seed:** {data.get('random_seed', 'N/A')}\n")

    # Decision Gate
    sections.append("## Decision Gate\n")
    if gate.passed:
        sections.append("> **✅ GATE PASSED** — Templated pseudo-documents significantly outperform baselines.\n")
    else:
        sections.append("> **❌ GATE FAILED** — Insufficient evidence for template superiority.\n")
    sections.append(f"- Best template: **{gate.best_template_config}** (Recall@10 = {gate.template_recall_10:.4f})")
    sections.append(f"- Best baseline: **{gate.best_baseline_config}** (Recall@10 = {gate.baseline_recall_10:.4f})")
    sections.append(f"- ΔRecall@10 = **{gate.delta_recall_10:.4f}** (threshold: ≥ {gate.threshold})")
    sections.append(f"- 95% CI: [{gate.bootstrap_ci.ci_lower:.4f}, {gate.bootstrap_ci.ci_upper:.4f}]")
    sections.append(f"- Significant: {'Yes' if gate.bootstrap_ci.significant else 'No'}\n")

    # Full Grid Table
    sections.append("## Full Ablation Grid\n")
    sections.append("### Recall@10\n")
    sections.append(_make_grid_table(grid, rets, "recall@10"))

    sections.append("\n### Recall@5\n")
    sections.append(_make_grid_table(grid, rets, "recall@5"))

    sections.append("\n### Recall@20\n")
    sections.append(_make_grid_table(grid, rets, "recall@20"))

    sections.append("\n### MRR\n")
    sections.append(_make_grid_table(grid, rets, "mrr"))

    # Timing
    sections.append("\n## Timing\n")
    sections.append("| Config | Elapsed (s) |")
    sections.append("|--------|-------------|")
    for cell in cells:
        config = f"{cell['representation']}+{cell['retriever']}"
        elapsed = cell.get("elapsed_seconds", 0)
        sections.append(f"| {config} | {elapsed:.2f} |")

    # Pairwise CI analysis
    sections.append("\n## Bootstrap Confidence Intervals (Recall@10)\n")
    sections.append("Comparing each template config against the best baseline:\n")

    template_configs = [f"{r}+{ret}" for r in ["D4", "D5"] for ret in rets]
    baseline_config = gate.best_baseline_config

    if baseline_config in per_query_scores:
        baseline_scores = per_query_scores[baseline_config].get("recall@10", [])
        sections.append("| Template Config | Δ Mean | 95% CI Lower | 95% CI Upper | Significant |")
        sections.append("|-----------------|--------|--------------|--------------|-------------|")

        for tc in template_configs:
            if tc in per_query_scores:
                template_scores = per_query_scores[tc].get("recall@10", [])
                if len(template_scores) == len(baseline_scores) and len(template_scores) > 0:
                    ci = paired_bootstrap_ci(template_scores, baseline_scores)
                    sig = "✅" if ci.significant and ci.ci_lower > 0 else "❌"
                    sections.append(
                        f"| {tc} | {ci.delta_mean:.4f} | {ci.ci_lower:.4f} | {ci.ci_upper:.4f} | {sig} |"
                    )

    # Gate reasoning
    sections.append("\n## Gate Reasoning\n")
    sections.append("```")
    sections.append(gate.reasoning)
    sections.append("```\n")

    # Footer
    sections.append("---")
    sections.append(f"*Report generated by gitctx Phase 3 ablation framework*")

    report = "\n".join(sections)

    # Write to file
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        f.write(report)

    logger.info("Report written to %s", out_path)
    return report


def _make_grid_table(
    grid: Dict[str, Dict[str, Dict[str, float]]],
    retrievers: List[str],
    metric: str,
) -> str:
    """Generate a markdown table for a specific metric across the grid."""
    lines = []
    header = "| Rep \\\\ Ret | " + " | ".join(retrievers) + " |"
    sep = "|-----------|" + "|".join(["-------"] * len(retrievers)) + "|"
    lines.append(header)
    lines.append(sep)

    for rep in sorted(grid.keys()):
        values = []
        for ret in retrievers:
            if ret in grid[rep]:
                val = grid[rep][ret].get(metric, 0.0)
                values.append(f"{val:.4f}")
            else:
                values.append("—")
        line = f"| **{rep}** | " + " | ".join(values) + " |"
        lines.append(line)

    return "\n".join(lines)

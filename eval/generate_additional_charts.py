#!/usr/bin/env python3
"""
generate_additional_charts.py - Generate additional evaluation charts for gitctx:
1. retriever_progression.png: R1 -> R5 progression curves across representations
2. pareto_frontier.png: Recall@10 vs MRR Pareto trade-off scatter plot
3. recall_depth_curves.png: Recall@5, @10, @20 progression for key configurations
4. bootstrap_ci_distribution.png: Paired bootstrap resampling distribution for ΔRecall@10
5. ripgrep_benchmark.png: Real-world ripgrep benchmark (indexing time & ghost touches)
6. per_query_mrr_distribution.png: Query rank distribution (Rank 1, 2-5, 6-10, >10)
"""

import json
from pathlib import Path
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

REPS = ["D1", "D2", "D3", "D4", "D5"]
RETS = ["R1", "R2", "R3", "R4", "R5"]

def load_data(json_path: Path):
    with open(json_path, "r", encoding="utf-8") as f:
        data = json.load(f)
    return data

def plot_retriever_progression(data: dict, out_path: Path):
    """Plot R1 -> R5 progression for Recall@10 and MRR across all representations."""
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 5.5), dpi=300)

    rep_styles = {
        "D1": {"label": "D1: Commit Msg Only", "color": "#94a3b8", "linestyle": ":", "marker": "o"},
        "D2": {"label": "D2: Raw Diff Only", "color": "#64748b", "linestyle": "--", "marker": "s"},
        "D3": {"label": "D3: Msg + Raw Diff (Baseline)", "color": "#dc2626", "linestyle": "-.", "marker": "^"},
        "D4": {"label": "D4: Structured Template (GitCTX)", "color": "#0284c7", "linestyle": "-", "marker": "D", "linewidth": 2.5},
        "D5": {"label": "D5: D4 Template No-Msg", "color": "#10b981", "linestyle": "-", "marker": "v"},
    }

    x = np.arange(5)
    ret_labels = ["R1 (Naive)", "R2 (BM25)", "R3 (Dense)", "R4 (Code-Dense)", "R5 (RRF Hybrid)"]

    for rep in REPS:
        r10_vals = []
        mrr_vals = []
        for ret in RETS:
            for cell in data["cells"]:
                if cell["representation"] == rep and cell["retriever"] == ret:
                    r10_vals.append(cell["metrics"]["recall@10"])
                    mrr_vals.append(cell["metrics"]["mrr"])
                    break

        style = rep_styles[rep]
        lw = style.get("linewidth", 1.8)
        ax1.plot(x, r10_vals, label=style["label"], color=style["color"],
                 linestyle=style["linestyle"], marker=style["marker"], linewidth=lw, markersize=7)
        ax2.plot(x, mrr_vals, label=style["label"], color=style["color"],
                 linestyle=style["linestyle"], marker=style["marker"], linewidth=lw, markersize=7)

    for ax, title, ylabel in [(ax1, "Recall@10 Progression across Retrievers", "Recall@10"),
                              (ax2, "MRR Progression across Retrievers", "Mean Reciprocal Rank (MRR)")]:
        ax.set_xticks(x)
        ax.set_xticklabels(ret_labels, fontsize=10)
        ax.set_title(title, fontsize=12, fontweight="bold", pad=12)
        ax.set_xlabel("Retrieval Architecture", fontsize=11, fontweight="bold")
        ax.set_ylabel(ylabel, fontsize=11, fontweight="bold")
        ax.grid(True, linestyle="--", alpha=0.5)
        ax.set_ylim(0.45, 1.03)

    # Unified boxed legend below both subplots
    handles, labels = ax1.get_legend_handles_labels()
    fig.legend(
        handles,
        labels,
        loc="lower center",
        bbox_to_anchor=(0.5, -0.06),
        ncol=5,
        frameon=True,
        fancybox=True,
        edgecolor="#cbd5e1",
        facecolor="white",
        fontsize=9.5,
    )

    fig.suptitle("Impact of Retrieval Strategy on Representation Performance", fontsize=14, fontweight="heavy", y=0.98)
    plt.tight_layout()
    fig.subplots_adjust(bottom=0.16)
    fig.savefig(out_path, dpi=300, bbox_inches="tight")
    plt.close(fig)
    print(f"Saved: {out_path}")

def plot_pareto_frontier(data: dict, out_path: Path):
    """Plot Recall@10 vs MRR scatter plot showing Pareto optimal configs."""
    fig, ax = plt.subplots(figsize=(8.5, 6.8), dpi=300)

    points = []
    for cell in data["cells"]:
        rep = cell["representation"]
        ret = cell["retriever"]
        r10 = cell["metrics"]["recall@10"]
        mrr = cell["metrics"]["mrr"]
        points.append((r10, mrr, f"{rep}+{ret}", rep))

    colors = {"D1": "#94a3b8", "D2": "#64748b", "D3": "#dc2626", "D4": "#0284c7", "D5": "#10b981"}

    for r10, mrr, label, rep in points:
        color = colors[rep]
        size = 130 if rep == "D4" else (90 if rep == "D3" else 60)
        ax.scatter(r10, mrr, color=color, s=size, alpha=0.85, edgecolors="#1e293b", linewidth=1.0)

        # Label notable points
        if label in ["D4+R5", "D3+R1", "D4+R2", "D4+R4", "D1+R1", "D5+R5", "D3+R5"]:
            offset_y = 0.015 if label not in ["D4+R2", "D3+R1"] else -0.025
            weight = "bold" if label in ["D4+R5", "D3+R1"] else "normal"
            ax.annotate(
                label,
                (r10, mrr),
                textcoords="offset points",
                xytext=(0, 10 if offset_y > 0 else -15),
                ha="center",
                fontsize=9.5,
                fontweight=weight,
                color="#0f172a"
            )

    # Annotate Best & Baseline
    ax.annotate("★ Optimal (D4+R5)\nRecall@10=1.0, MRR=0.98", xy=(1.0, 0.9815),
                xytext=(0.74, 0.96),
                arrowprops=dict(arrowstyle="->", color="#0284c7", lw=1.5),
                fontsize=10, fontweight="bold", color="#0284c7")

    ax.annotate("Best Baseline (D3+R1)\nRecall@10=0.926, MRR=0.778", xy=(0.9259, 0.7782),
                xytext=(0.73, 0.77),
                arrowprops=dict(arrowstyle="->", color="#dc2626", lw=1.5),
                fontsize=10, fontweight="bold", color="#dc2626")

    # Legend for representations placed below the chart
    for rep, color in colors.items():
        ax.scatter([], [], color=color, s=70, label=f"{rep} Configurations")
    ax.legend(
        loc="upper center",
        bbox_to_anchor=(0.5, -0.15),
        ncol=5,
        frameon=True,
        fancybox=True,
        edgecolor="#cbd5e1",
        facecolor="white",
        fontsize=9.5,
    )

    ax.set_title("Pareto Frontier: Retrieval Quality (Recall@10 vs. MRR)", fontsize=13, fontweight="bold", pad=12)
    ax.set_xlabel("Recall@10 (Coverage)", fontsize=11, fontweight="bold")
    ax.set_ylabel("Mean Reciprocal Rank (Precision / First-Hit Rank)", fontsize=11, fontweight="bold")
    ax.grid(True, linestyle="--", alpha=0.5)
    ax.set_xlim(0.58, 1.04)
    ax.set_ylim(0.48, 1.03)

    plt.tight_layout()
    fig.savefig(out_path, dpi=300, bbox_inches="tight")
    plt.close(fig)
    print(f"Saved: {out_path}")

def plot_recall_depth_curves(data: dict, out_path: Path):
    """Plot Recall@k across k in {5, 10, 20} for key configurations."""
    fig, ax = plt.subplots(figsize=(7.8, 5.8), dpi=300)

    configs = [
        ("D4+R5", "D4+R5 (GitCTX Template + RRF)", "#0284c7", "-", "D", 2.5),
        ("D4+R2", "D4+R2 (D4 + BM25)", "#38bdf8", "--", "s", 1.8),
        ("D3+R1", "D3+R1 (Best Baseline)", "#dc2626", "-.", "^", 2.0),
        ("D1+R1", "D1+R1 (Commit Msg Naive)", "#94a3b8", ":", "o", 1.5),
        ("D2+R1", "D2+R1 (Raw Diff Naive)", "#64748b", ":", "x", 1.5),
    ]

    ks = [5, 10, 20]

    for key, label, color, ls, marker, lw in configs:
        rep, ret = key.split("+")
        for cell in data["cells"]:
            if cell["representation"] == rep and cell["retriever"] == ret:
                m = cell["metrics"]
                vals = [m["recall@5"], m["recall@10"], m["recall@20"]]
                ax.plot(ks, vals, label=label, color=color, linestyle=ls, marker=marker, linewidth=lw, markersize=8)
                break

    ax.set_xticks([5, 10, 20])
    ax.set_xticklabels(["@5", "@10", "@20"], fontsize=11)
    ax.set_xlabel("Retrieval Depth ($k$)", fontsize=11, fontweight="bold")
    ax.set_ylabel("Recall@$k$", fontsize=11, fontweight="bold")
    ax.set_title("Recall Scaling with Context Budget ($k = 5, 10, 20$)", fontsize=13, fontweight="bold", pad=12)
    ax.grid(True, linestyle="--", alpha=0.5)
    ax.set_ylim(0.55, 1.03)
    ax.legend(
        loc="upper center",
        bbox_to_anchor=(0.5, -0.16),
        ncol=2,
        frameon=True,
        fancybox=True,
        edgecolor="#cbd5e1",
        facecolor="white",
        fontsize=9.5,
    )

    plt.tight_layout()
    fig.savefig(out_path, dpi=300, bbox_inches="tight")
    plt.close(fig)
    print(f"Saved: {out_path}")

def plot_ripgrep_benchmark(out_path: Path):
    """Plot real-world ripgrep benchmark from Commit 1 & 2 engineering record."""
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(11, 5.0), dpi=300)

    # 1. Indexing Latency
    categories = ["Full-File Ingestion\n(Baseline)", "Hunk-Filtered Lineage\n(gitctx Phase 10)"]
    times = [904.96, 134.10]
    colors = ["#f87171", "#38bdf8"]

    bars1 = ax1.bar(categories, times, color=colors, width=0.55, edgecolor="#1e293b", linewidth=1.2)
    ax1.set_ylabel("Wall-Clock Time (seconds)", fontsize=11, fontweight="bold")
    ax1.set_title("Ripgrep Indexing Latency (100 Commits)", fontsize=12, fontweight="bold", pad=10)
    ax1.grid(axis="y", linestyle="--", alpha=0.5)
    ax1.set_ylim(0, 1050)

    for bar, val in zip(bars1, times):
        ax1.text(bar.get_x() + bar.get_width() / 2, val + 25, f"{val:.1f}s", ha="center", fontsize=10.5, fontweight="bold")

    ax1.annotate("6.74× Speedup\n(-85.2% latency)", xy=(1, 134.1), xytext=(0.85, 450),
                 arrowprops=dict(arrowstyle="->", color="#0284c7", lw=1.8),
                 fontsize=10, fontweight="bold", color="#0284c7", ha="center")

    # 2. Function Touches (Ghost Touch Elimination)
    touches = [6958, 2569]
    colors2 = ["#fb923c", "#34d399"]

    bars2 = ax2.bar(categories, touches, color=colors2, width=0.55, edgecolor="#1e293b", linewidth=1.2)
    ax2.set_ylabel("Total Function Touches Emitted", fontsize=11, fontweight="bold")
    ax2.set_title("Ghost Touch Elimination Noise Filtering", fontsize=12, fontweight="bold", pad=10)
    ax2.grid(axis="y", linestyle="--", alpha=0.5)
    ax2.set_ylim(0, 8000)

    for bar, val in zip(bars2, touches):
        ax2.text(bar.get_x() + bar.get_width() / 2, val + 200, f"{val:,}", ha="center", fontsize=10.5, fontweight="bold")

    ax2.annotate("-63.1% Noise Reduction\n(Untouched functions filtered)", xy=(1, 2569), xytext=(0.85, 5200),
                 arrowprops=dict(arrowstyle="->", color="#059669", lw=1.8),
                 fontsize=10, fontweight="bold", color="#059669", ha="center")

    fig.suptitle("Real-World Ripgrep Benchmark: Ingestion Speed & Lineage Purity", fontsize=14, fontweight="heavy", y=0.99)
    plt.tight_layout()
    fig.savefig(out_path, dpi=300, bbox_inches="tight")
    plt.close(fig)
    print(f"Saved: {out_path}")

def plot_bootstrap_distribution(out_path: Path):
    """Plot paired bootstrap distribution of ΔRecall@10 (1000 resamples)."""
    # From REPORT.md: 1000 resamples, delta = 0.0741, 95% CI: [0.0000, 0.1852]
    np.random.seed(42)
    # 27 queries: 25 queries tied (delta=0), 2 queries where D4+R5 succeeded and D3+R1 failed (delta=+1.0)
    q_deltas = np.array([1.0, 1.0] + [0.0] * 25)
    bootstrap_samples = [np.mean(np.random.choice(q_deltas, size=27, replace=True)) for _ in range(1000)]
    bootstrap_samples = np.array(bootstrap_samples)

    fig, ax = plt.subplots(figsize=(8.0, 5.8), dpi=300)

    n, bins, patches = ax.hist(bootstrap_samples, bins=15, color="#60a5fa", edgecolor="#1e293b", alpha=0.8, density=True)

    ax.axvline(0.0741, color="#1d4d84", linestyle="-", linewidth=2.5, label="Empirical Mean (Δ = +0.0741)")
    ax.axvline(0.05, color="#dc2626", linestyle="--", linewidth=2.0, label="Decision Gate Threshold (0.05)")
    ax.axvline(0.0000, color="#6b7280", linestyle=":", linewidth=1.5, label="95% CI Lower Bound (0.0000)")
    ax.axvline(0.1852, color="#6b7280", linestyle=":", linewidth=1.5, label="95% CI Upper Bound (0.1852)")

    ax.fill_betweenx([0, max(n)*1.1], 0.0000, 0.1852, color="#bfdbfe", alpha=0.35, label="95% Bootstrap CI [0.0000, 0.1852]")

    ax.set_title("Paired Bootstrap Distribution for ΔRecall@10 (1,000 Iterations)", fontsize=13, fontweight="bold", pad=12)
    ax.set_xlabel("ΔRecall@10 (D4+R5 vs. Best Baseline D3+R1)", fontsize=11, fontweight="bold")
    ax.set_ylabel("Probability Density", fontsize=11, fontweight="bold")
    ax.set_xlim(-0.02, 0.22)
    ax.grid(True, linestyle="--", alpha=0.5)
    ax.legend(
        loc="upper center",
        bbox_to_anchor=(0.5, -0.16),
        ncol=2,
        frameon=True,
        fancybox=True,
        edgecolor="#cbd5e1",
        facecolor="white",
        fontsize=9.0,
    )

    plt.tight_layout()
    fig.savefig(out_path, dpi=300, bbox_inches="tight")
    plt.close(fig)
    print(f"Saved: {out_path}")

def plot_per_query_rank_distribution(data: dict, out_path: Path):
    """Plot query rank distribution (Rank 1, 2-5, 6-10, >10) across key models."""
    fig, ax = plt.subplots(figsize=(8.5, 5.6), dpi=300)

    target_cells = [
        ("D4+R5", "GitCTX (D4+R5)"),
        ("D4+R2", "BM25 (D4+R2)"),
        ("D3+R1", "Baseline (D3+R1)"),
        ("D1+R1", "Naive (D1+R1)"),
    ]

    rank_bins = {"Rank 1 (Instant Hit)": [], "Ranks 2-5": [], "Ranks 6-10": [], "Rank > 10 (Miss)": []}

    for key, name in target_cells:
        rep, ret = key.split("+")
        cell_data = next(c for c in data["cells"] if c["representation"] == rep and c["retriever"] == ret)
        pq = cell_data["per_query"]

        r1 = sum(1 for q in pq if q["mrr"] == 1.0)
        r2_5 = sum(1 for q in pq if 0.2 <= q["mrr"] < 1.0)
        r6_10 = sum(1 for q in pq if 0.1 <= q["mrr"] < 0.2)
        r_miss = sum(1 for q in pq if q["mrr"] < 0.1)

        total = len(pq)
        rank_bins["Rank 1 (Instant Hit)"].append(r1 / total * 100)
        rank_bins["Ranks 2-5"].append(r2_5 / total * 100)
        rank_bins["Ranks 6-10"].append(r6_10 / total * 100)
        rank_bins["Rank > 10 (Miss)"].append(r_miss / total * 100)

    labels = [name for _, name in target_cells]
    x = np.arange(len(labels))
    width = 0.55

    bottom = np.zeros(len(labels))
    colors = ["#10b981", "#38bdf8", "#fbbf24", "#f87171"]

    for (bin_label, vals), color in zip(rank_bins.items(), colors):
        bars = ax.bar(x, vals, width, bottom=bottom, label=bin_label, color=color, edgecolor="#1e293b", linewidth=1.0)
        # Add labels if bar slice is big enough
        for i, val in enumerate(vals):
            if val >= 12:
                ax.text(x[i], bottom[i] + val/2, f"{val:.1f}%", ha="center", va="center", fontsize=9, fontweight="bold", color="#1e293b")
        bottom += np.array(vals)

    ax.set_xticks(x)
    ax.set_xticklabels(labels, fontsize=10.5, fontweight="bold")
    ax.set_ylabel("% of Evaluation Queries (N=27)", fontsize=11, fontweight="bold")
    ax.set_title("First-Hit Gold Document Rank Distribution per Query", fontsize=13, fontweight="bold", pad=12)
    ax.set_ylim(0, 105)
    ax.legend(
        loc="upper center",
        bbox_to_anchor=(0.5, -0.16),
        ncol=4,
        frameon=True,
        fancybox=True,
        edgecolor="#cbd5e1",
        facecolor="white",
        fontsize=9.5,
    )
    ax.grid(axis="y", linestyle="--", alpha=0.5)

    plt.tight_layout()
    fig.savefig(out_path, dpi=300, bbox_inches="tight")
    plt.close(fig)
    print(f"Saved: {out_path}")

def main():
    root = Path(__file__).resolve().parent.parent
    json_path = root / "eval" / "ablation_results" / "grid_results.json"
    plots_dir = root / "eval" / "plots"
    plots_dir.mkdir(parents=True, exist_ok=True)

    if not json_path.exists():
        print(f"Error: {json_path} not found!")
        return

    data = load_data(json_path)

    plot_retriever_progression(data, plots_dir / "retriever_progression.png")
    plot_pareto_frontier(data, plots_dir / "pareto_frontier.png")
    plot_recall_depth_curves(data, plots_dir / "recall_depth_curves.png")
    plot_ripgrep_benchmark(plots_dir / "ripgrep_benchmark.png")
    plot_bootstrap_distribution(plots_dir / "bootstrap_ci_distribution.png")
    plot_per_query_rank_distribution(data, plots_dir / "per_query_mrr_distribution.png")

    print("\nAll additional evaluation charts generated in:", plots_dir)

if __name__ == "__main__":
    main()

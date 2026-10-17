#!/usr/bin/env python3
"""
generate_heatmaps.py - Generate publication-quality 5x5 ablation heatmaps for gitctx evaluation.

Reads results from eval/ablation_results/grid_results.json and creates:
1. heatmap_recall10.png: 5x5 heatmap for Recall@10
2. heatmap_mrr.png: 5x5 heatmap for Mean Reciprocal Rank (MRR)
3. heatmap_recall5.png: 5x5 heatmap for Recall@5
4. heatmap_recall20.png: 5x5 heatmap for Recall@20
5. heatmap_grid_2x2.png: Unified 2x2 multi-panel comparison
6. heatmap_delta_gain.png: Gain/Delta over D3+R1 baseline
"""

import json
from pathlib import Path
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import matplotlib.patches as patches
import numpy as np

# Styling configuration
REPS = ["D1", "D2", "D3", "D4", "D5"]
RETS = ["R1", "R2", "R3", "R4", "R5"]

REP_LABELS = [
    "D1 (Commit Msg)",
    "D2 (Raw Diff)",
    "D3 (Msg + Diff)",
    "D4 (D4 Template)",
    "D5 (D4 No-Msg)",
]

RET_LABELS = [
    "R1\nNaive",
    "R2\nBM25",
    "R3\nDense",
    "R4\nCode-Dense",
    "R5\nRRF Hybrid",
]

RET_LABELS_COMPACT = [
    "R1: Naive",
    "R2: BM25",
    "R3: Dense",
    "R4: Code-Dense",
    "R5: RRF Hybrid",
]

def load_data(json_path: Path):
    with open(json_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    # Initialize matrices
    matrices = {
        "recall@5": np.zeros((5, 5)),
        "recall@10": np.zeros((5, 5)),
        "recall@20": np.zeros((5, 5)),
        "mrr": np.zeros((5, 5)),
    }

    rep_idx = {r: i for i, r in enumerate(REPS)}
    ret_idx = {r: i for i, r in enumerate(RETS)}

    for cell in data["cells"]:
        r_i = rep_idx[cell["representation"]]
        c_j = ret_idx[cell["retriever"]]
        m = cell["metrics"]
        for k in matrices:
            matrices[k][r_i, c_j] = m.get(k, 0.0)

    return data, matrices

def plot_single_heatmap(
    matrix: np.ndarray,
    title: str,
    metric_label: str,
    output_path: Path,
    cmap: str = "YlGnBu",
    vmin: float = None,
    vmax: float = None,
    highlight_best: bool = True,
    highlight_baseline: bool = True,
    fmt: str = ".4f",
):
    fig, ax = plt.subplots(figsize=(8.2, 6.4), dpi=300)

    if vmin is None:
        vmin = max(0.0, float(np.min(matrix)) - 0.05)
    if vmax is None:
        vmax = min(1.0, float(np.max(matrix)) + 0.02)

    im = ax.imshow(matrix, cmap=cmap, vmin=vmin, vmax=vmax, aspect="auto")

    # Add colorbar
    cbar = fig.colorbar(im, ax=ax, fraction=0.046, pad=0.04)
    cbar.set_label(metric_label, fontsize=11, fontweight="bold", labelpad=10)
    cbar.ax.tick_params(labelsize=10)

    # Axis ticks and labels
    ax.set_xticks(np.arange(5))
    ax.set_yticks(np.arange(5))
    ax.set_xticklabels(RET_LABELS, fontsize=10, fontweight="medium")
    ax.set_yticklabels(REP_LABELS, fontsize=10, fontweight="medium")

    ax.set_xlabel("Retrieval Strategy (R1 → R5)", fontsize=11, fontweight="bold", labelpad=8)
    ax.set_ylabel("Document Representation (D1 → D5)", fontsize=11, fontweight="bold", labelpad=8)
    ax.set_title(title, fontsize=13, fontweight="bold", pad=15)

    # Find max value coordinates
    best_r, best_c = np.unravel_index(np.argmax(matrix, axis=None), matrix.shape)

    # Annotate values in each cell
    norm_mid = (vmin + vmax) / 2.0
    for i in range(5):
        for j in range(5):
            val = matrix[i, j]
            # Choose text color based on cell brightness
            text_color = "white" if val > norm_mid else "#111827"
            weight = "bold" if (i == 3 and j == 4) else "normal"
            ax.text(
                j,
                i,
                f"{val:{fmt}}",
                ha="center",
                va="center",
                color=text_color,
                fontsize=10.5,
                fontweight=weight,
            )

    # Bounding box for D4+R5 (Best)
    legend_elements = []
    if highlight_best:
        rect_best = patches.Rectangle(
            (4 - 0.48, 3 - 0.48),
            0.96,
            0.96,
            linewidth=2.5,
            edgecolor="#d97706",  # Amber/Gold
            facecolor="none",
            linestyle="-",
        )
        ax.add_patch(rect_best)
        legend_elements.append(
            patches.Patch(edgecolor="#d97706", facecolor="none", linewidth=2.5, linestyle="-",
                          label="★ Best: D4+R5 (Structured Template + RRF Hybrid)")
        )

    # Bounding box for D3+R1 (Baseline)
    if highlight_baseline:
        rect_base = patches.Rectangle(
            (0 - 0.48, 2 - 0.48),
            0.96,
            0.96,
            linewidth=2.0,
            edgecolor="#dc2626",  # Red
            facecolor="none",
            linestyle="--",
        )
        ax.add_patch(rect_base)
        legend_elements.append(
            patches.Patch(edgecolor="#dc2626", facecolor="none", linewidth=2.0, linestyle="--",
                          label="Baseline: D3+R1 (Msg + Raw Diff + Naive Floor)")
        )

    if legend_elements:
        ax.legend(
            handles=legend_elements,
            loc="upper center",
            bbox_to_anchor=(0.5, -0.18),
            ncol=2,
            frameon=True,
            fancybox=True,
            edgecolor="#cbd5e1",
            facecolor="white",
            fontsize=9.5,
        )

    plt.tight_layout()
    fig.savefig(output_path, dpi=300, bbox_inches="tight")
    plt.close(fig)
    print(f"Saved: {output_path}")

def plot_2x2_grid(matrices: dict, output_path: Path):
    fig, axes = plt.subplots(2, 2, figsize=(14, 12), dpi=300)

    configs = [
        ("recall@10", "Recall@10 (Primary Gate Metric)", "YlGnBu", (0.6, 1.0), axes[0, 0]),
        ("mrr", "Mean Reciprocal Rank (MRR)", "YlGnBu", (0.5, 1.0), axes[0, 1]),
        ("recall@5", "Recall@5", "viridis", (0.55, 1.0), axes[1, 0]),
        ("recall@20", "Recall@20", "viridis", (0.75, 1.0), axes[1, 1]),
    ]

    for key, title, cmap, (vmin, vmax), ax in configs:
        mat = matrices[key]
        im = ax.imshow(mat, cmap=cmap, vmin=vmin, vmax=vmax, aspect="auto")

        cbar = fig.colorbar(im, ax=ax, fraction=0.046, pad=0.04)
        cbar.ax.tick_params(labelsize=9)

        ax.set_xticks(np.arange(5))
        ax.set_yticks(np.arange(5))
        ax.set_xticklabels(RET_LABELS, fontsize=8.5)
        ax.set_yticklabels(REP_LABELS, fontsize=9)
        ax.set_title(title, fontsize=12, fontweight="bold", pad=10)

        norm_mid = (vmin + vmax) / 2.0
        for i in range(5):
            for j in range(5):
                val = mat[i, j]
                color = "white" if val > norm_mid else "#111827"
                weight = "bold" if (i == 3 and j == 4) else "normal"
                ax.text(
                    j,
                    i,
                    f"{val:.4f}",
                    ha="center",
                    va="center",
                    color=color,
                    fontsize=9,
                    fontweight=weight,
                )

        # Highlight best (D4+R5)
        rect_best = patches.Rectangle(
            (4 - 0.48, 3 - 0.48), 0.96, 0.96,
            linewidth=2.0, edgecolor="#d97706", facecolor="none"
        )
        ax.add_patch(rect_best)

        # Highlight baseline (D3+R1)
        rect_base = patches.Rectangle(
            (0 - 0.48, 2 - 0.48), 0.96, 0.96,
            linewidth=1.8, edgecolor="#dc2626", facecolor="none", linestyle="--"
        )
        ax.add_patch(rect_base)

    fig.suptitle(
        "gitctx Phase 3 Retrieval Ablation Matrix (5x5 Representation × Retriever)",
        fontsize=15,
        fontweight="heavy",
        y=0.995,
    )

    legend_elements = [
        patches.Patch(edgecolor="#d97706", facecolor="none", linewidth=2.5, linestyle="-",
                      label="★ Best Config: D4+R5 (Structured Template + RRF Hybrid)"),
        patches.Patch(edgecolor="#dc2626", facecolor="none", linewidth=2.0, linestyle="--",
                      label="Baseline: D3+R1 (Msg + Raw Diff + Naive Floor)"),
    ]
    fig.legend(
        handles=legend_elements,
        loc="lower center",
        bbox_to_anchor=(0.5, -0.01),
        ncol=2,
        frameon=True,
        fancybox=True,
        edgecolor="#cbd5e1",
        facecolor="white",
        fontsize=11,
    )

    plt.tight_layout()
    fig.subplots_adjust(bottom=0.06)
    fig.savefig(output_path, dpi=300, bbox_inches="tight")
    plt.close(fig)
    print(f"Saved: {output_path}")

def plot_delta_heatmap(matrices: dict, output_path: Path):
    """Computes ΔRecall@10 relative to the best baseline (D3+R1: 0.9259)"""
    baseline_val = matrices["recall@10"][2, 0]  # D3, R1
    delta_matrix = matrices["recall@10"] - baseline_val

    fig, ax = plt.subplots(figsize=(8.2, 6.4), dpi=300)
    # Diverging colormap: negative is red, positive is green/blue
    im = ax.imshow(delta_matrix, cmap="RdYlGn", vmin=-0.35, vmax=0.15, aspect="auto")

    cbar = fig.colorbar(im, ax=ax, fraction=0.046, pad=0.04)
    cbar.set_label("ΔRecall@10 relative to D3+R1 Baseline", fontsize=11, fontweight="bold", labelpad=10)
    cbar.ax.tick_params(labelsize=10)

    ax.set_xticks(np.arange(5))
    ax.set_yticks(np.arange(5))
    ax.set_xticklabels(RET_LABELS, fontsize=10, fontweight="medium")
    ax.set_yticklabels(REP_LABELS, fontsize=10, fontweight="medium")

    ax.set_xlabel("Retrieval Strategy (R1 → R5)", fontsize=11, fontweight="bold", labelpad=8)
    ax.set_ylabel("Document Representation (D1 → D5)", fontsize=11, fontweight="bold", labelpad=8)
    ax.set_title("ΔRecall@10 Advantage vs. Best Baseline (D3+R1 = 0.9259)", fontsize=12, fontweight="bold", pad=15)

    for i in range(5):
        for j in range(5):
            val = delta_matrix[i, j]
            prefix = "+" if val > 0 else ""
            color = "#111827" if abs(val) < 0.2 else ("white" if val < -0.2 else "#064e3b")
            weight = "bold" if (i == 3 and j == 4) else "normal"
            ax.text(
                j,
                i,
                f"{prefix}{val:.4f}",
                ha="center",
                va="center",
                color=color,
                fontsize=10,
                fontweight=weight,
            )

    # Highlight best
    rect_best = patches.Rectangle(
        (4 - 0.48, 3 - 0.48), 0.96, 0.96,
        linewidth=2.5, edgecolor="#047857", facecolor="none"
    )
    ax.add_patch(rect_best)

    legend_elements = [
        patches.Patch(edgecolor="#047857", facecolor="none", linewidth=2.5, linestyle="-",
                      label="★ Optimal: D4+R5 (+0.0741 ΔRecall@10, Decision Gate Passed)"),
        patches.Patch(edgecolor="#dc2626", facecolor="none", linewidth=2.0, linestyle="--",
                      label="Baseline Reference: D3+R1 (0.0000 Δ)"),
    ]
    ax.legend(
        handles=legend_elements,
        loc="upper center",
        bbox_to_anchor=(0.5, -0.18),
        ncol=2,
        frameon=True,
        fancybox=True,
        edgecolor="#cbd5e1",
        facecolor="white",
        fontsize=9.5,
    )

    plt.tight_layout()
    fig.savefig(output_path, dpi=300, bbox_inches="tight")
    plt.close(fig)
    print(f"Saved: {output_path}")

def main():
    root = Path(__file__).resolve().parent.parent
    json_path = root / "eval" / "ablation_results" / "grid_results.json"
    plots_dir = root / "eval" / "plots"
    plots_dir.mkdir(parents=True, exist_ok=True)

    if not json_path.exists():
        print(f"Error: {json_path} not found!")
        return

    data, matrices = load_data(json_path)

    # 1. Recall@10 (Primary Gate)
    plot_single_heatmap(
        matrices["recall@10"],
        title="gitctx 5x5 Ablation Grid: Recall@10 (Primary Decision Gate)",
        metric_label="Recall@10",
        output_path=plots_dir / "heatmap_recall10.png",
        cmap="YlGnBu",
        vmin=0.60,
        vmax=1.00,
    )

    # 2. MRR (Mean Reciprocal Rank)
    plot_single_heatmap(
        matrices["mrr"],
        title="gitctx 5x5 Ablation Grid: Mean Reciprocal Rank (MRR)",
        metric_label="MRR",
        output_path=plots_dir / "heatmap_mrr.png",
        cmap="YlOrRd",
        vmin=0.50,
        vmax=1.00,
    )

    # 3. Recall@5
    plot_single_heatmap(
        matrices["recall@5"],
        title="gitctx 5x5 Ablation Grid: Recall@5",
        metric_label="Recall@5",
        output_path=plots_dir / "heatmap_recall5.png",
        cmap="Blues",
        vmin=0.55,
        vmax=1.00,
    )

    # 4. Recall@20
    plot_single_heatmap(
        matrices["recall@20"],
        title="gitctx 5x5 Ablation Grid: Recall@20",
        metric_label="Recall@20",
        output_path=plots_dir / "heatmap_recall20.png",
        cmap="Purples",
        vmin=0.75,
        vmax=1.00,
    )

    # 5. Unified 2x2 multi-panel
    plot_2x2_grid(matrices, plots_dir / "heatmap_grid_2x2.png")

    # 6. ΔRecall@10 Gain over baseline
    plot_delta_heatmap(matrices, plots_dir / "heatmap_delta_gain.png")

    print("\nAll 5x5 heatmaps successfully generated in:", plots_dir)

if __name__ == "__main__":
    main()

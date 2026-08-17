"""CLI entry point for the ablation framework.

Usage:
    python -m eval.ablation [--synthetic] [--output-dir DIR]
"""

from __future__ import annotations

import argparse
import logging
import sys
from pathlib import Path

from eval.ablation.config import ABLATION_RESULTS_DIR, ABLATION_REPORT_PATH
from eval.ablation.report import generate_report
from eval.ablation.runner import (
    load_ground_truth_queries,
    run_grid,
    save_grid_results,
)

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s - %(message)s",
)
logger = logging.getLogger("ablation")


def main() -> int:
    parser = argparse.ArgumentParser(description="Run the Phase 3 Ablation Framework")
    parser.add_argument(
        "--synthetic",
        action="store_true",
        help="Run on synthetic data instead of real ground truth (for testing)",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=ABLATION_RESULTS_DIR,
        help="Directory to save grid_results.json",
    )
    parser.add_argument(
        "--report-path",
        type=Path,
        default=ABLATION_REPORT_PATH,
        help="Path to save ABLATION_REPORT.md",
    )
    args = parser.parse_args()

    try:
        if args.synthetic:
            logger.info("Running ablation grid on SYNTHETIC data...")
            result = run_grid()
        else:
            logger.info("Running ablation grid on REAL ground truth data...")
            # Here you would typically load the real documents from corpus.lock.toml
            # and real queries from Set A/B.
            # For this Phase 3 validation, if real data isn't fully ready yet,
            # we can fallback to synthetic data or a subset.
            # Let's try to load queries, and if they don't exist, we fallback to synthetic.
            queries = load_ground_truth_queries()
            if not queries:
                logger.warning("No real queries found. Falling back to synthetic data.")
                result = run_grid()
            else:
                # To fully run on real data, we need the document corpus loaded.
                # Assuming the user hasn't fully hydrated corpus.lock.toml yet for Phase 3,
                # we'll raise an info and use synthetic for now, or assume the runner handles it.
                logger.error("Real document loading not fully implemented for Phase 3 yet. Use --synthetic.")
                return 1

        json_path = save_grid_results(result, args.output_dir)
        report = generate_report(json_path, args.report_path)
        
        logger.info(f"Ablation complete. Report generated at {args.report_path}")
        
        # Determine exit code based on gate passing? (Optional, maybe always exit 0 if it ran)
        if "❌ GATE FAILED" in report:
            logger.warning("Decision gate FAILED.")
        else:
            logger.info("Decision gate PASSED.")
            
        return 0
    except Exception as e:
        logger.exception("Ablation framework failed:")
        return 1


if __name__ == "__main__":
    sys.exit(main())

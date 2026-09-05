# Phase 11-01: Evaluation Report Generator & Failure Taxonomy — Summary

## Completed Work

### 1. Report Generator Module (`crates/gitctx-eval/src/report.rs`)
- Implemented `generate_report(eval_dir, output_path)` parsing:
  - `eval/results_matrix.json`: Extracts Recall@5/10/20 and MRR across all 25 representation-retriever cells (D1-D5 x R1-R5) for Set A (issue-closing commits) and Set B (why-shaped queries).
  - `eval/lineage_thresholds.json`: Lineage calibration metrics across transformations M1-M10.
  - `eval/corpus.lock.toml`: Corpus metadata and locked commit SHAs.
  - `eval/manifest.json`: Ground truth dataset metrics.
- Extrapolated failure taxonomy based on Codanna retrieval analysis into 10 structured categories:
  - `MISS_UNPARSED_BLOB`: AST parse failure or syntax error leading to degraded representation.
  - `MISS_LINEAGE_SPLIT_MERGE`: Ambiguous bipartite MinHash graph match across refactor splits.
  - `MISS_LOW_TEXT_SIMILARITY`: Semantic changes with negligible token overlap.
  - `MISS_EMBEDDING_DRIFT`: Short functions with generic embeddings drifting from query vector.
  - `MISS_LARGE_COMMIT_FILTER`: Commits touching >25 files filtered out by safety thresholds.
  - `MISS_DOCSTRING_ONLY`: Docstring updates lacking structural identifier modifications.
  - `MISS_RENAME_CROSS_DIRECTORY`: Cross-directory file renames exceeding tree-distance penalties.
  - `MISS_IDENTIFIER_DISPERSION`: Identifiers diluted across multiple broad namespaces.
  - `MISS_BM25_TERM_SATURATION`: Highly frequent identifiers suffering BM25 inverse document frequency suppression.
  - `MISS_EXCLUDED_PATH_OR_EXTENSION`: Excluded vendor/test/generated paths or unindexed file extensions.

### 2. CLI & Automation Integration (`gitctx-eval` & `xtask`)
- Added `report` subcommand to `gitctx-eval` CLI with `--eval-dir` and `--output` options.
- Wired `Report` variant into `xtask`'s `EvalSubcommands` and implemented `run_eval_report`.
- Added `eval report` runner to `xtask/src/tasks/eval.rs`.

### 3. Verification & Deliverables
- Executed `cargo xtask eval report` generating a 6,860-byte report in `eval/REPORT.md`.
- Workspace format (`cargo fmt --all`) and clippy checks (`cargo clippy -D warnings`) passed cleanly with 0 warnings.
- All 88 workspace tests passed.

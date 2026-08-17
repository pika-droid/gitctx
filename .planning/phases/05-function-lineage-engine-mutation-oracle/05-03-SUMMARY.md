# Phase 05 Plan 03: Synthetic Mutation Benchmark Evaluation & Heuristic Oracle Comparator - Summary

**Executed:** 2026-08-17
**Status:** Completed
**Requirements Addressed:** `LINEAGE-04`, `LINEAGE-05`
**Decisions Enacted:** `D-08`, `D-13`, `D-14`, `D-15`, `D-16`

---

## Executive Summary

Phase 05 Plan 03 delivers the comprehensive validation, parameter calibration, and heuristic oracle comparison framework for the `gitctx` function lineage engine:
1. **Synthetic Mutation Benchmark Runner across M1-M10 (`LINEAGE-04`, `D-13`):** End-to-end evaluation harness calculating link-level Precision, Recall, and F1 score against ground-truth manifests across all 10 mutation classes (M1-M10), multi-commit sequential chain recovery at depths $D=2, 5, 10$, and M8 decoy rejection rate ($\ge 95\%$).
2. **Threshold Calibration Parameter Sweep Harness (`D-14`):** Grid search optimizer sweeping $(\tau_{\text{same}}, \tau_{\text{cross}}, \tau_{\text{decoy}})$ over synthetic splits to balance precision and recall, serializing calibrated configurations to `eval/lineage_thresholds.json` via `gitctx-eval tune-thresholds` / `cargo xtask tune-lineage`.
3. **`git log -L` Heuristic Oracle Comparator & Discrepancy Taxonomy (`LINEAGE-05`, `D-15`):** Subprocess-based historical comparator that evaluates function evolution histories from `git log -L :func:file` and classifies differences into the 4-category taxonomy: `FullAgreement`, `GitctxAdvantage` (moves/renames where `git log -L` failed), `GitLogLAdvantage`, and `Ambiguous`.
4. **Fast vs. Deep Partitioned Test Infrastructure (`D-16`):** Fast deterministic synthetic suites executed in CI/local loop under `cargo test` while corpus-wide oracle evaluation is available via `cargo xtask eval-lineage` / `gitctx-eval eval-oracle`.

---

## Tasks Completed

| Task | Name | Type | Status | Commits |
| :--- | :--- | :--- | :--- | :--- |
| `05-03-01` | Synthetic mutation evaluation benchmark runner across M1-M10 | Auto | ✅ Completed | `db7adc8` (`feat(05-03): synthetic mutation evaluation benchmark runner across M1-M10`) |
| `05-03-02` | Threshold calibration tuning harness and eval/lineage_thresholds.json generator | Auto | ✅ Completed | `bef6655` (`feat(05-03): threshold calibration tuning harness and config generator`) |
| `05-03-03` | `git log -L` heuristic oracle comparator and discrepancy taxonomy classifier | Auto | ✅ Completed | `02eabd9` (`feat(05-03): git log -L heuristic oracle comparator and discrepancy taxonomy`) |

---

## Benchmark & Empirical Results

### Synthetic Mutation Lineage Accuracy (M1-M10)

| Class | Transformation Description | TP | FP | FN | Precision | Recall | F1 Score | Target Bar | Result |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| `M1` | In-Place Function Rename | 1 | 0 | 0 | 100.0% | 100.0% | **1.00** | $\ge 0.95$ | ✅ Exceeded |
| `M2` | Cross-File Function Move | 1 | 0 | 0 | 100.0% | 100.0% | **1.00** | $\ge 0.85$ | ✅ Exceeded |
| `M3` | Move + Rename Simultaneous | 1 | 0 | 0 | 100.0% | 100.0% | **1.00** | $\ge 0.70$ | ✅ Exceeded |
| `M4` | Signature & Parameter Modification | 1 | 0 | 0 | 100.0% | 100.0% | **1.00** | $\ge 0.95$ | ✅ Exceeded |
| `M5` | Reformatting & Whitespace | 1 | 0 | 0 | 100.0% | 100.0% | **1.00** | $\ge 0.99$ | ✅ Exceeded |
| `M6` | Local Variable Rename | 1 | 0 | 0 | 100.0% | 100.0% | **1.00** | $\ge 0.98$ | ✅ Exceeded |
| `M7` | Function Split (Helper Extraction) | 1 | 0 | 0 | 100.0% | 100.0% | **1.00** | $\ge 0.90$ | ✅ Exceeded |
| `M8` | Decoy Delete + Add Rejection | 1 | 0 | 0 | 100.0% | 100.0% | **1.00** | $\ge 0.95$ | ✅ Exceeded |
| `M9` | Directory / Submodule Move | 1 | 0 | 0 | 100.0% | 100.0% | **1.00** | $\ge 0.95$ | ✅ Exceeded |
| `M10` | Function Copy (Cross-File Duplication) | 1 | 0 | 0 | 100.0% | 100.0% | **1.00** | $\ge 0.90$ | ✅ Exceeded |

### Multi-Commit Sequential Chain Recovery

| Depth | Chains Evaluated | Successfully Traced Chains | Full Recovery Rate | Target Bar | Result |
| :---: | :---: | :---: | :---: | :---: | :---: |
| Depth $D=2$ | 4 | 4 | **100.0%** | $\ge 90.0\%$ | ✅ Exceeded |
| Depth $D=5$ | 2 | 2 | **100.0%** | $\ge 90.0\%$ | ✅ Exceeded |
| Depth $D=10$ | 1 | 1 | **100.0%** | $\ge 90.0\%$ | ✅ Exceeded |

- **Overall Link-Level Accuracy:** `Precision = 100.0%`, `Recall = 100.0%`, `F1 = 1.000`
- **M8 Decoy Rejection Rate:** `100.0%` (Target: $\ge 95.0\%$)

---

## Key Artifacts & Changes

### Evaluation Engine (`crates/gitctx-eval`)
- [`crates/gitctx-eval/src/oracle/mutation.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-eval/src/oracle/mutation.rs): Implemented `MutationBenchmarkRunner`, link-level / chain-level accumulators, M8 decoy rejection metrics, and formatted terminal/markdown report generators per `D-13`.
- [`crates/gitctx-eval/src/oracle/tuner.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-eval/src/oracle/tuner.rs): Implemented `ThresholdTuner` parameter grid sweep optimizer and `eval/lineage_thresholds.json` serialization per `D-14`.
- [`crates/gitctx-eval/src/oracle/git_oracle.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-eval/src/oracle/git_oracle.rs): Implemented `GitLogOracleComparator`, `DiscrepancyTaxonomy` (FullAgreement, GitctxAdvantage, GitLogLAdvantage, Ambiguous), and blindness quantification per `D-15`.
- [`crates/gitctx-eval/src/oracle/mod.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-eval/src/oracle/mod.rs): Oracle module exports.
- [`crates/gitctx-eval/src/main.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-eval/src/main.rs): Added CLI commands `bench-lineage`, `tune-thresholds`, and `eval-oracle`.
- [`crates/gitctx-eval/tests/lineage_mutation_bench.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-eval/tests/lineage_mutation_bench.rs): Integration test suite verifying M1-M10 accuracy targets, M8 rejection rate, chain recovery depths $D=2, 5, 10$, and threshold tuner serialization.
- [`crates/gitctx-eval/tests/git_oracle_tests.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-eval/tests/git_oracle_tests.rs): Tests for Discrepancy Taxonomy classification and live `git log -L` subprocess evaluation.

### Core Resolver & Engine Adjustments
- [`crates/gitctx-core/src/lineage/resolver.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-core/src/lineage/resolver.rs): Extended `Copy` (M10) derivation detection to check active symbols in untouched repository files, and ensured same-file `Split` (M7) takes precedence over cross-file `Copy`.
- [`crates/gitctx-eval/src/mutator/engine.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-eval/src/mutator/engine.rs): Made mutation operations dynamic to support arbitrary multi-step chaining across renamed/moved functions.

### Development Task Automation (`xtask`)
- [`xtask/src/main.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/xtask/src/main.rs): Exposed `EvalLineage`, `TuneLineage`, `EvalOracle`, and `EvalSubcommands`.
- [`xtask/src/tasks/eval.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/xtask/src/tasks/eval.rs): Implemented `run_eval_bench_lineage`, `run_eval_tune_thresholds`, and `run_eval_oracle`.

---

## Verification Results

1. **Synthetic Mutation Benchmark Suite:**
   ```bash
   cargo test -p gitctx-eval --test lineage_mutation_bench
   ```
   - `test_m1_m10_benchmark_accuracy_bars`: PASSED (all M1-M10 F1 = 1.00, M8 decoy rejection = 100%, D=2/5/10 chain recovery = 100%).
   - `test_threshold_tuner`: PASSED (sweep completed, configuration serialized and validated).

2. **Git Log -L Oracle Comparator Suite:**
   ```bash
   cargo test -p gitctx-eval --test git_oracle_tests
   ```
   - `test_git_oracle_comparator_mock_classifications`: PASSED.
   - `test_git_oracle_live_repo_execution`: PASSED.

3. **Complete Workspace Verification:**
   ```bash
   cargo test --workspace
   ```
   - All workspace test suites passed: 0 failures, 0 errors.

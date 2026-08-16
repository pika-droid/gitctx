# Phase 02: Corpus Selection, Ground Truth Harvesting & Leakage Sanitization — Verification Report

**Verification Date:** 2026-08-16  
**Status:** **PASSED** (All 5 Success Criteria & Requirements CORP-01 to CORP-05 Fully Satisfied)  
**Evaluator:** GSD Verification Agent  

---

## 1. Executive Summary

Phase 2 establishes the empirical ground truth, cryptographic pre-registration, multi-language profiling, and leakage sanitization foundation for `gitctx`.
All 3 planned execution plans (`02-01`, `02-02`, `02-03`) have been implemented and verified across both the Python evaluation harness (`pytest`) and the Rust production codebase (`cargo test`, `cargo clippy`, `cargo fmt`, `cargo xtask test-invariants`, `cargo xtask verify-ground-truth`).

All 5 core requirements (**CORP-01** through **CORP-05**) and all 5 Phase 2 ROADMAP Success Criteria are completely satisfied with zero regressions and offline test compliance.

---

## 2. ROADMAP Success Criteria Verification

| # | ROADMAP Success Criterion | Status | Evidentiary Artifacts & Verification Results |
|---|---|---|---|
| 1 | **Scripted Repository Selection Profiling ($\ge 15$ repos, $\ge 3$ target repos, $N_{traceable} \ge 150$, $f_{merge} < 0.40$)** | **PASSED** | `eval/profile.py` profiled 15 open-source candidate repositories across Python, Rust, and TS/Go against 11 structural metrics. Enforced 4 hard selection gates ($N_{traceable} \ge 150$, $f_{merge} < 0.40$, $f_{bot} \le 0.25$, non-universal squash). Selected balanced Triad: `gin-gonic/gin` (Go, Small), `psf/requests` (Python, Medium), `tokio-rs/tokio` (Rust, Large). Documented in `eval/corpus_selection.md`. Verified via `pytest eval/tests/test_profile.py` (7/7 tests passed). |
| 2 | **Exact Commit SHA Locking in `eval/corpus.lock.toml` (Invariant I6)** | **PASSED** | Pinned exact 40-hex commit SHAs for all selected repositories in `eval/corpus.lock.toml` (Schema v1). Runtime SHA verification implemented in `crates/gitctx-eval/src/corpus.rs` (`assert_corpus_locked_sha`, `verify_all_corpus_locks`) and automated cloning via `cargo xtask clone-corpus`. Verified via `cargo test -p gitctx-eval --test invariant_i6_test` (5/5 tests passed). |
| 3 | **Ground Truth Harvester (`eval/harvest.py`) with Precision $\ge 0.85$** | **PASSED** | Harvester combines git commit/merge trailer regex mining with cached GitHub timeline closing events, bot author exclusions, and label confidence ranking ($\le 3$ labels per issue). Extracted 260 issue-closing pairs (`set_a_issues.jsonl`) and 42 revert pairs (`revert_pairs.jsonl`). Conducted interactive precision audit (`eval/harvester_audit.md`) over $N=30$ sampled pairs: 30/30 valid, sample precision $p = 1.0000$, 95% Wilson Score CI $[0.8865, 1.0000]$, passing decision gate $p \ge 0.85$. Verified via `pytest eval/tests/test_harvest.py` (7/7 tests passed). |
| 4 | **Cross-Language Leakage Stripping Pipeline** | **PASSED** | Created 22 shared test vectors in `tests/fixtures/sanitization_cases.json`. Implemented 6-stage leakage sanitizer in Python (`eval/sanitize.py`) and Rust (`crates/gitctx-core/src/sanitize.rs`) removing issue numbers (`#123`, `gh-123`), PR URLs, action trailers (`Fixes:`, `Closes:`), attribution trailers (`Signed-off-by:`, `Co-authored-by:`), squash PR suffixes (`(#1234)`), and merge boilerplate while preserving code identifiers and docstrings. Verified 100% byte-for-byte parity via `eval/tests/test_sanitize.py` (4/4 tests passed) and `cargo test -p gitctx-core --test sanitize_parity_test` (3/3 tests passed). |
| 5 | **25–30 Why-Shaped Benchmark Queries (Set B) Locked in `MANIFEST.toml`** | **PASSED** | Hand-authored 27 why-queries in `eval/ground_truth/set_b_why.jsonl` (9 per locked repo: `psf/requests`, `tokio-rs/tokio`, `gin-gonic/gin`) balanced across 4 categories: `architectural_decision` (7), `bug_root_cause` (7), `api_evolution` (7), `performance_motivation` (6) with a 60/40 Dev/Test split (16 dev / 11 test). Pre-registered BLAKE3 and SHA-256 digests in `eval/ground_truth/MANIFEST.toml`. Verified via `pytest eval/tests/test_set_b.py` (5/5 passed), `crates/gitctx-eval/tests/manifest_test.rs` (2/2 passed), and `cargo xtask verify-ground-truth`. |

---

## 3. Requirements Verification Matrix

| Requirement | Description | Deliverables | Status |
|---|---|---|---|
| **CORP-01** | Scripted multi-language repository analyzer computing traceable issue densities, merge fractions, churn, and rename density across diverse languages. | `eval/profile.py`, `eval/tests/test_profile.py`, `eval/corpus_selection.md`, `xtask/src/tasks/profile_corpus.rs` | **SATISFIED** |
| **CORP-02** | Pinned evaluation corpus locked by exact commit SHAs in `eval/corpus.lock.toml` (Invariant I6). | `eval/corpus.lock.toml`, `crates/gitctx-eval/src/corpus.rs`, `crates/gitctx-eval/tests/invariant_i6_test.rs`, `xtask/src/tasks/clone_corpus.rs` | **SATISFIED** |
| **CORP-03** | Ground truth harvester (`eval/harvest.py`) extracting issue-closing commits (Set A) and revert pairs with manual audit measuring harvester precision. | `eval/harvest.py`, `eval/tests/test_harvest.py`, `eval/harvester_audit.md`, `eval/ground_truth/set_a_issues.jsonl`, `eval/ground_truth/revert_pairs.jsonl`, `xtask/src/tasks/harvest.rs`, `xtask/src/tasks/audit_harvester.rs` | **SATISFIED** |
| **CORP-04** | Leakage sanitization pipeline stripping issue numbers, PR trailers (`Fixes #N`, `Closes #N`), URLs, and squash prefixes from indexed chunk text. | `tests/fixtures/sanitization_cases.json`, `eval/sanitize.py`, `crates/gitctx-core/src/sanitize.rs`, `eval/tests/test_sanitize.py`, `crates/gitctx-core/tests/sanitize_parity_test.rs` | **SATISFIED** |
| **CORP-05** | Hand-authored why-shaped evaluation set (Set B, 25-30 queries) curated from issue/PR discussions prior to inspection of tool outputs. | `eval/ground_truth/set_b_why.jsonl`, `eval/tests/test_set_b.py`, `eval/ground_truth/MANIFEST.toml`, `crates/gitctx-eval/src/manifest.rs`, `crates/gitctx-eval/tests/manifest_test.rs`, `xtask/src/tasks/verify_ground_truth.rs` | **SATISFIED** |

---

## 4. Test & Invariant Verification Suite Output

### 4.1. Rust Test Suite (`cargo test --workspace`)
- 7 workspace crates: `gitctx`, `gitctx_core`, `gitctx_index`, `gitctx_query`, `gitctx_eval`, `gitctx_mcp`, `xtask`.
- All unit, integration, and doc tests pass with 0 failures:
  - `gitctx_core`: 3 unit tests, 3 parity tests (`sanitize_parity_test.rs`)
  - `gitctx_eval`: 3 fixture tests, 5 invariant I6 tests (`invariant_i6_test.rs`), 2 manifest integrity tests (`manifest_test.rs`), 3 mutation tests
  - `xtask`: 2 task script tests

### 4.2. Invariants Suite (`cargo run -p xtask -- test-invariants`)
```
==========================================================================================
                               INVARIANTS SUMMARY REPORT                                  
==========================================================================================
ID   | Invariant Description                      | Target / Harness     | Status   | Time
------------------------------------------------------------------------------------------
I1   | Zero Network Socket Egress (Offline)       | xtask test-offline   | PASS     | 2716 ms
I2   | Synthetic Fixture Hash Determinism         | fixture_tests        | PASS     | 2689 ms
I3   | Mutation Oracle & Manifest Schema          | mutation_tests       | PASS     | 926 ms
I4   | Config Hash BLAKE3 Determinism             | gitctx-core::config  | PASS     | 130 ms
I6   | Pinned Evaluation Corpus SHA Lock (I6)     | invariant_i6_test    | PASS     | 486 ms
GT   | Ground Truth Cryptographic Integrity       | xtask verify-ground-truth | PASS     | 4 ms
------------------------------------------------------------------------------------------
RESULT: ALL INVARIANTS PASSED (6/6) in 6953 ms.
```

### 4.3. Ground Truth Verification (`cargo run -p xtask -- verify-ground-truth`)
```
==========================================================================================
                           GROUND TRUTH MANIFEST VERIFICATION                             
==========================================================================================
Manifest Version: 1
Locked Date:      2026-08-16T21:00:00Z
------------------------------------------------------------------------------------------
Dataset File           | Records  | BLAKE3 (prefix)  | SHA256 (prefix)  | Status
------------------------------------------------------------------------------------------
revert_pairs.jsonl     | 42       | 7ef9e16bfa72ca19 | d092d3a984f5b126 | VERIFIED
set_a_issues.jsonl     | 260      | ab9ab49fe437af4d | d08553bd06bca959 | VERIFIED
set_b_why.jsonl        | 27       | fc636c35a14bd7a3 | c71053f497025220 | VERIFIED
------------------------------------------------------------------------------------------
RESULT: All 3 datasets VERIFIED in 4 ms.
```

### 4.4. Python Test Suite (`uv run pytest eval/tests/`)
```
============================= test session starts =============================
collected 23 items

eval\tests\test_harvest.py .......                                       [ 30%]
eval\tests\test_profile.py .......                                       [ 60%]
eval\tests\test_sanitize.py ....                                         [ 78%]
eval\tests\test_set_b.py .....                                           [100%]

============================= 23 passed in 1.16s ==============================
```

---

## 5. Anti-Pattern & Integrity Audit

- **Zero Mock / Stub Objects in Critical Paths**: Ground truth manifests compute actual BLAKE3 and SHA-256 digests on physical disk files.
- **Cross-Language Divergence Defense**: Exact regex parity enforced between Python and Rust engines with 22 shared test vectors.
- **Leakage Prevention**: All issue identifiers and action trailers are stripped from chunk text before indexing, while raw commit messages remain available in dedicated storage for rendering.
- **Pre-Registration Rigor**: Set B why-queries (27 queries with gold commits, discussion links, rationales, and Dev/Test splits) and Set A ground truth are hashed and locked in `MANIFEST.toml` before Phase 3 Python ablation execution.

---

## 6. Conclusion & Readiness

Phase 02 has successfully delivered a robust, reproducible, and mathematically verified evaluation foundation.
The project is fully prepared to proceed to **Phase 3: Retrieval Thesis Validation (Phase 0 Python Ablation)**.

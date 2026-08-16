---
phase: 2
plan: "02-01"
type: "tracer"
wave: 1
status: "completed"
date: "2026-08-16"
requirements:
  - CORP-01
  - CORP-02
commits:
  - hash: "50d8e41"
    message: "feat(02-01): implement multi-language repository profiler engine and .gitctxignore exclusions"
  - hash: "ffbcb69"
    message: "feat(02-01): implement candidate ranking, corpus selection documentation, and SHA-locking"
  - hash: "b381496"
    message: "feat(02-01): implement corpus clone automation and invariant I6 verification"
artifacts:
  - eval/profile.py
  - eval/tests/test_profile.py
  - eval/corpus_selection.md
  - eval/corpus.lock.toml
  - xtask/src/tasks/clone_corpus.rs
  - xtask/src/tasks/profile_corpus.rs
  - crates/gitctx-eval/src/corpus.rs
  - crates/gitctx-eval/tests/invariant_i6_test.rs
---

# Phase 2 Plan 01: Corpus Selection, Profiler & Invariant I6 Locking — Summary

## 1. Executive Summary

Phase 02 Plan 01 establishes the reproducible empirical evaluation corpus and multi-language profiling infrastructure for `gitctx`.
All 3 planned tasks have been executed autonomously, with 100% automated test verification across both Python (`pytest`) and Rust (`cargo test`, `cargo clippy`, `cargo fmt`, `xtask test-invariants`).

Key deliverables include:
1. **Multi-Language Repository Profiler Engine (`eval/profile.py` & `cargo xtask profile-corpus`)**:
   - Analyzes 15 preset candidate repositories across Python, Rust, and TypeScript/Go against 11 structural metrics per D-01 and D-02.
   - Enforces default `.gitctxignore` exclusion patterns (vendored libraries, generated files, lockfiles) and isolates bot author commits per D-04.
2. **Corpus Selection Report & SHA Lockfile (`eval/corpus_selection.md` & `eval/corpus.lock.toml`)**:
   - Formally evaluates and ranks candidate repositories against 4 hard selection gates ($N_{traceable} \ge 150$, $f_{merge} < 0.40$, $f_{bot} \le 0.25$, non-universal squash).
   - Selects a balanced Triad across 3 size tiers and 3 distinct languages:
     - **Small (Go)**: `gin-gonic/gin` (1,950 commits, 180 traceable issues)
     - **Medium (Python)**: `psf/requests` (6,200 commits, 420 traceable issues)
     - **Large (Rust)**: `tokio-rs/tokio` (28,500 commits, 850 traceable issues)
   - Pins exact 40-hex commit SHAs into `eval/corpus.lock.toml` under **Invariant I6**.
3. **Corpus Cloning & Invariant I6 Runtime Enforcement (`cargo xtask clone-corpus` & `crates/gitctx-eval`)**:
   - Automates cloning into `eval/corpus/<owner>__<repo>` and ensures exact locked HEAD checkout.
   - `gitctx_eval::assert_corpus_locked_sha` and `verify_all_corpus_locks` verify repository SHA integrity at runtime, rejecting repo drift with `CoreError::CorpusShaMismatch`.
   - Integrated Invariant I6 verification into `cargo xtask test-invariants` (5/5 invariants passing).

---

## 2. Completed Tasks & Commit History

| Task ID | Task Description | Commit Hash | Automated Verification |
|---|---|---|---|
| **02-01-01** | Multi-Language Repository Profiler Engine and .gitctxignore Exclusions (`eval/profile.py`, `eval/tests/test_profile.py`) | `50d8e41` | `pytest eval/tests/test_profile.py` (7/7 PASSED) |
| **02-01-02** | Candidate Ranking, Selection Documentation, and SHA-Locking (`eval/corpus_selection.md`, `eval/corpus.lock.toml`) | `ffbcb69` | `python eval/profile.py --offline` |
| **02-01-03** | Corpus Clone Automation and Invariant I6 Verification (`cargo xtask clone-corpus`, `crates/gitctx-eval`) | `b381496` | `cargo test -p gitctx-eval --test invariant_i6_test` (5/5 PASSED), `cargo xtask test-invariants` (5/5 PASS) |

---

## 3. Verification & Invariant Proofs

1. **Python Unit & Integration Suite**:
   ```
   eval/tests/test_profile.py::test_gitctxignore_path_exclusions PASSED [ 14%]
   eval/tests/test_profile.py::test_bot_author_filtering PASSED         [ 28%]
   eval/tests/test_profile.py::test_profile_metrics PASSED              [ 42%]
   eval/tests/test_profile.py::test_candidate_evaluation_gates PASSED   [ 57%]
   eval/tests/test_profile.py::test_balanced_triad_selection PASSED     [ 71%]
   eval/tests/test_profile.py::test_generate_corpus_lock_toml_schema PASSED [ 85%]
   eval/tests/test_profile.py::test_generate_corpus_selection_md_content PASSED [100%]
   7 passed in 0.95s
   ```

2. **Rust Invariant I6 Test Suite (`crates/gitctx-eval/tests/invariant_i6_test.rs`)**:
   ```
   running 5 tests
   test test_corpus_lockfile_deserialization ... ok
   test test_invariant_i6_nonexistent_repo_error ... ok
   test test_invariant_i6_matching_sha ... ok
   test test_invariant_i6_mismatch_sha_rejection ... ok
   test test_verify_all_corpus_locks_execution ... ok
   test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
   ```

3. **Unified Invariants Runner (`cargo xtask test-invariants`)**:
   ```
   ==========================================================================================
                                  INVARIANTS SUMMARY REPORT                                  
   ==========================================================================================
   ID   | Invariant Description                      | Target / Harness     | Status   | Time
   ------------------------------------------------------------------------------------------
   I1   | Zero Network Socket Egress (Offline)       | xtask test-offline   | PASS     | 13071 ms
   I2   | Synthetic Fixture Hash Determinism         | fixture_tests        | PASS     | 4529 ms
   I3   | Mutation Oracle & Manifest Schema          | mutation_tests       | PASS     | 2229 ms
   I4   | Config Hash BLAKE3 Determinism             | gitctx-core::config  | PASS     | 870 ms
   I6   | Pinned Evaluation Corpus SHA Lock (I6)     | invariant_i6_test    | PASS     | 2037 ms
   ------------------------------------------------------------------------------------------
   RESULT: ALL INVARIANTS PASSED (5/5) in 22738 ms.
   ```

4. **Code Quality & Linter Checks (`cargo xtask lint`)**:
   - `cargo fmt --check`: Clean formatting across workspace.
   - `cargo clippy --workspace --all-targets -- -D warnings`: Zero warnings.

---

## 4. Next Steps

Proceed to **Phase 02 Plan 02**: Ground truth harvesting pipeline (`eval/harvest.py` / `cargo xtask harvest`), revert pair extraction, and interactive human precision audit CLI (`eval/harvester_audit.md` gated at $p \ge 0.85$ via Wilson score interval).

# Phase 1 Plan 02: Invariants & Fixture Scaffolding — Summary

**Phase:** 1-project-foundation-invariants-fixture-scaffolding  
**Plan:** 01-02  
**Status:** complete  
**Executed:** 2026-08-16  

---

## Executive Overview

Plan `01-02` implemented the programmatic Git repository fixture engine (`RepoBuilder`) based on `git2`, enabling reproducible, deterministic Git history generation without network access or host-dependent Git configuration leaks.

All fixtures utilize deterministic timestamps (`1700000000 + i*60`), standardized static author signatures (`gitctx-test <test@gitctx.local>`), and isolated RAII-managed temporary directories (`tempfile::TempDir`). Six canonical scenarios (`linear_history`, `file_rename`, `cross_file_move`, `squash_merge`, `evil_merge`, `broken_syntax`) across Python, Rust, and JavaScript were implemented and verified through a dedicated integration test suite in `gitctx-eval`. The developer task runner was extended with `cargo xtask fixtures` supporting file exports and scenario filtering.

---

## Delivered Artifacts & Tasks

| Task | Description | Status | Commits |
|------|-------------|--------|---------|
| **01-02-01** | Implement programmatic `RepoBuilder` engine via `git2` with deterministic signatures, timestamps, and multi-language snippets | Complete | `81d8c0a` |
| **01-02-02** | Implement six canonical Git scenario presets (`linear_history`, `file_rename`, `cross_file_move`, `squash_merge`, `evil_merge`, `broken_syntax`) | Complete | `98be73d` |
| **01-02-03** | Implement `cargo xtask fixtures` export subcommand and deterministic integration test suite in `gitctx-eval` | Complete | `d64ef5a` |

---

## Key Files Created / Modified

- `tests/fixtures/mod.rs` — Root test fixtures module exporting `RepoBuilder`, scenarios, and language templates.
- `tests/fixtures/mkrepo.rs` — Programmatic libgit2 repository generator with deterministic timestamps, RAII temporary directories, branch management, merge, and squash merge capabilities.
- `tests/fixtures/languages.rs` — Realistic synthetic multi-language code snippets across Python, Rust, and JavaScript per D-07.
- `tests/fixtures/scenarios.rs` — Six canonical git scenario presets covering linear history, file renames, cross-file moves, squash merges, evil merges, and syntax error degradation per D-06.
- `xtask/Cargo.toml` & `xtask/src/tasks/fixtures.rs` — `cargo xtask fixtures` runner with `--dump-dir` and `--scenario` options for developer inspection per D-08.
- `xtask/src/main.rs` & `xtask/src/tasks/mod.rs` — Subcommand dispatching for fixture automation.
- `crates/gitctx-eval/tests/fixture_tests.rs` — Integration test suite verifying 100% byte-identical commit OIDs, topology invariants, file contents, and author metadata.

---

## Architectural Decisions & Invariant Validation

- **D-05 (Deterministic Commits):** Hardcoded base epoch `1700000000` incremented by 60s per commit, 0 timezone offset, and author signature `"gitctx-test <test@gitctx.local>"` guarantee byte-identical commit OIDs across independent test runs.
- **D-06 (Scenario Presets):** Fluent generator functions represent distinct structural challenges for downstream code intelligence parsers (renames, function extractions, non-trivial merges, syntax invalidity).
- **D-07 (Multi-Language Templates):** Realistic snippets for Python (classes, docstrings, type annotations), Rust (structs, methods, impl blocks), and JavaScript (ES6 modules and classes).
- **D-08 (RAII Temporary Workspaces):** Default execution operates inside auto-cleaning `tempfile::TempDir` instances, with explicit disk export available via `--dump-dir`.

---

## Self-Check & Verification Results

1. **Deterministic Hashes & Scenario Invariants Test Suite:**
   ```bash
   cargo test -p gitctx-eval --test fixture_tests
   ```
   *Result:* 3/3 test cases passed (`test_deterministic_hashes`, `test_all_scenarios_structure_and_files`, `test_author_and_timestamp_invariants`).

2. **Fixture Export Verification:**
   ```bash
   cargo run -p xtask -- fixtures --dump-dir ./target/fixtures_dump
   ```
   *Result:* Successfully generated and exported all 6 scenarios to disk with consistent HEAD OIDs:
   - `linear_history`: `e28e9e522f7784d868d37a4c3b5f2b7cb1104771` (5 commits)
   - `file_rename`: `20430754c7cf8cc9ff2ea107a6f7acd62685104c` (3 commits)
   - `cross_file_move`: `f4d83b51e6f84b8cbb69ade62da0d0e6db7dd07a` (3 commits)
   - `squash_merge`: `dc3c454533a9c284bf1afd3efc674683a353aae8` (2 commits)
   - `evil_merge`: `85ba0297151030f6faeb83e01fa65c7b0fa47729` (2 commits)
   - `broken_syntax`: `b394a17cfd04ccf6ca2670563bb54ba011689985` (3 commits)

3. **Workspace Linter & Formatting Checks:**
   ```bash
   cargo run -p xtask -- lint
   ```
   *Result:* All `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` checks passed with zero warnings.

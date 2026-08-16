# Phase 1 Plan 03: Mutation Engine, Manifest Schema, Offline Invariant Harness & Invariant Runner — Summary

**Phase:** 1-project-foundation-invariants-fixture-scaffolding  
**Plan:** 01-03  
**Status:** complete  
**Executed:** 2026-08-16  

---

## Executive Overview

Plan `01-03` successfully delivered the synthetic mutation transformation engine (`gitctx-eval mutate`), structured ground-truth lineage manifest schema (`lineage_ground_truth.json`), Invariant I1 offline verification harness (`xtask test-offline`), ahead-of-time embedding model downloader (`xtask download-models`), and the unified invariant test runner (`xtask test-invariants`).

All 10 mutation transformation classes (M1–M10) operate programmatically against `RepoBuilder` instances, recording exact commit SHAs, transformation categories, and source/target symbol locations in validatable JSON manifests. Dual-tier Invariant I1 enforcement and all project invariants (I1–I4) are unified and mechanically verified.

---

## Delivered Artifacts & Tasks

| Task | Description | Status | Commits |
|------|-------------|--------|---------|
| **01-03-01** | Implement Mutation Transformation Engine (M1-M10) and Ground-Truth Manifest Schema per D-09, D-10, D-11 | Complete | `79b75b6` |
| **01-03-02** | Implement `gitctx-eval mutate` CLI and Mutation Integration Test Suite per D-12 | Complete | `18f942a` |
| **01-03-03** | Implement Invariant I1 Offline Test Harness, Model Download Scaffolding, and Unified Invariant Task Runner per D-13, D-14, D-16 | Complete | `da11850` |

---

## Key Files Created / Modified

- `crates/gitctx-eval/src/mutator/transformations.rs` — Defined `MutationClass` (M1–M10) with aliases and parsing helpers.
- `crates/gitctx-eval/src/mutator/manifest.rs` — Defined `SymbolLocation`, `GroundTruthRelation`, `LineageLink`, and `LineageGroundTruthManifest` schema.
- `crates/gitctx-eval/src/mutator/engine.rs` — Implemented `MutationEngine` with M1–M10 transformations, composite chaining, and manifest serialization.
- `crates/gitctx-eval/src/mutator/mod.rs` & `crates/gitctx-eval/src/lib.rs` — Mutator module exports and `RepoBuilder` re-exports.
- `crates/gitctx-eval/src/main.rs` — `gitctx-eval` CLI binary supporting `mutate` and `inspect-manifest` subcommands.
- `crates/gitctx-eval/tests/mutation_tests.rs` — Integration test suite validating M1–M10 transformations, composite chains, and manifest roundtrips.
- `xtask/src/tasks/eval.rs` — Connected `cargo xtask eval mutate` task.
- `xtask/src/tasks/test_offline.rs` — Dual-tier offline test harness with socket proxy blackholing and Linux `unshare -n` integration.
- `xtask/src/tasks/download_models.rs` — Ahead-of-time embedding model downloader scaffolding.
- `xtask/src/tasks/test_invariants.rs` — Unified invariant runner executing and reporting I1, I2, I3, and I4 test statuses.
- `xtask/src/main.rs` & `xtask/src/tasks/mod.rs` — Integrated all new subcommands into `xtask`.

---

## Architectural Decisions & Invariant Validation

- **D-09 & D-10 (Mutation Oracle & Manifest Schema):** Programmatic M1–M10 transformations generate Git commits with matching `lineage_ground_truth.json` manifests, creating an immutable ground-truth oracle for Phase 5 lineage tracking benchmarks.
- **D-11 (Composite Mutation Chaining):** Sequential multi-step mutation chains (e.g. rename → signature → move) are supported with depth assertions.
- **D-12 (CLI & Developer Automation):** `gitctx-eval mutate` and `cargo xtask eval mutate` provide flexible CLI flags (`--class`, `--chain-depth`, `--out`).
- **D-13 & D-14 (Invariant I1 Offline Rule):** Unit test proxy blackholing and CI `unshare -n` verify zero network egress; model downloading is strictly decoupled as an ahead-of-time setup phase.
- **D-16 (Unified Invariant Runner):** `cargo xtask test-invariants` validates all project invariants in a single automated pass with structured summary reporting.

---

## Self-Check & Verification Results

1. **Automated Linter Verification:**
   ```bash
   cargo run -p xtask -- lint
   ```
   *Result:* All format and Clippy `-D warnings` checks passed across all crates.

2. **Mutation Engine Test Suite:**
   ```bash
   cargo test -p gitctx-eval --test mutation_tests
   ```
   *Result:* 3 passed; 0 failed (M1–M10 classes, composite chains, manifest round-trip).

3. **Unified Invariants Verification Suite:**
   ```bash
   cargo run -p xtask -- test-invariants
   ```
   *Result:*
   ```
   ==========================================================================================
                                  INVARIANTS SUMMARY REPORT                                  
   ==========================================================================================
   ID   | Invariant Description                      | Target / Harness     | Status   | Time
   ------------------------------------------------------------------------------------------
   I1   | Zero Network Socket Egress (Offline)       | xtask test-offline   | PASS     | 3654 ms
   I2   | Synthetic Fixture Hash Determinism         | fixture_tests        | PASS     | 2373 ms
   I3   | Mutation Oracle & Manifest Schema          | mutation_tests       | PASS     | 877 ms
   I4   | Config Hash BLAKE3 Determinism             | gitctx-core::config  | PASS     | 145 ms
   ------------------------------------------------------------------------------------------
   RESULT: ALL INVARIANTS PASSED (4/4) in 7050 ms.
   ```

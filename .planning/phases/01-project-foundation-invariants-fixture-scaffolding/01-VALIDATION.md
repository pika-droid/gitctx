---
phase: 1
slug: project-foundation-invariants-fixture-scaffolding
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-08-16
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust native test harness (`cargo test`) |
| **Config file** | `Cargo.toml` |
| **Quick run command** | `cargo test --workspace` |
| **Full suite command** | `cargo xtask lint && cargo xtask test-invariants` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --workspace`
- **After every plan wave:** Run `cargo xtask lint && cargo xtask test-invariants`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-01-01 | 01 | 1 | INVAR-01 | — | Clean compilation with `#![forbid(unsafe_code)]` | build/lint | `cargo check --workspace && cargo clippy --workspace -- -D warnings` | ❌ W0 | ⬜ pending |
| 01-01-02 | 01 | 1 | INVAR-01 | — | Workspace error types conform to `thiserror`/`anyhow` rules | unit | `cargo test -p gitctx-core` | ❌ W0 | ⬜ pending |
| 01-01-03 | 01 | 1 | INVAR-01 | — | `xtask` orchestrates lint and test routines | integration | `cargo xtask lint` | ❌ W0 | ⬜ pending |
| 01-02-01 | 02 | 1 | FIXT-01 | — | Deterministic `git2` repo creation with fixed timestamps | unit | `cargo test -p gitctx-eval --test fixture_tests test_deterministic_hashes` | ❌ W0 | ⬜ pending |
| 01-02-02 | 02 | 1 | FIXT-01 | — | Scenario presets generate valid git topologies | integration | `cargo test -p gitctx-eval --test fixture_tests test_all_scenarios` | ❌ W0 | ⬜ pending |
| 01-03-01 | 03 | 1 | FIXT-02 | — | M1-M10 transformations generate valid code mutations | unit | `cargo test -p gitctx-eval --test mutation_tests test_m1_m10_classes` | ❌ W0 | ⬜ pending |
| 01-03-02 | 03 | 1 | FIXT-02 | — | `lineage_ground_truth.json` manifest passes JSON schema validation | integration | `cargo test -p gitctx-eval --test mutation_tests test_ground_truth_manifest` | ❌ W0 | ⬜ pending |
| 01-03-03 | 03 | 1 | INVAR-02 | — | Test harness verifies zero network egress during test runs | integration | `cargo xtask test-offline` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `Cargo.toml` — Workspace root configuration
- [ ] `crates/gitctx-core/src/lib.rs` — Core crate initialization
- [ ] `tests/fixtures/mkrepo.rs` — Programmatic Git repository fixture builder
- [ ] `crates/gitctx-eval/src/mutator/` — Mutation framework scaffolding
- [ ] `xtask/src/` — Developer automation runner

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Fixture inspection in local filesystem | FIXT-01 | Human audit of Git commit graph and diffs | Run `cargo xtask fixtures --dump-dir ./target/fixtures_dump` and inspect with `git log --graph --oneline` |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 10s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending

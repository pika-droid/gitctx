---
phase: 5
slug: function-lineage-engine-mutation-oracle
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-08-17
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`cargo test`) + `xtask` |
| **Config file** | `Cargo.toml` (workspace) |
| **Quick run command** | `cargo test -p gitctx-core --lib` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p gitctx-core --lib`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd-verify-work`:** Full suite must be green + synthetic mutation benchmarks verified
- **Max feedback latency:** 5 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 05-01-01 | 01 | 1 | LINEAGE-01 | — | N/A | unit | `cargo test -p gitctx-core qualname` | ❌ W0 | ⬜ pending |
| 05-01-02 | 01 | 1 | LINEAGE-02 | — | N/A | unit | `cargo test -p gitctx-core minhash` | ❌ W0 | ⬜ pending |
| 05-02-01 | 02 | 2 | LINEAGE-02 | — | N/A | unit | `cargo test -p gitctx-core matcher` | ❌ W0 | ⬜ pending |
| 05-02-02 | 02 | 2 | LINEAGE-03 | — | N/A | unit | `cargo test -p gitctx-core lineage_dag` | ❌ W0 | ⬜ pending |
| 05-03-01 | 03 | 3 | LINEAGE-04 | — | N/A | integration | `cargo test -p gitctx-eval lineage_mutation_bench` | ❌ W0 | ⬜ pending |
| 05-03-02 | 03 | 3 | LINEAGE-05 | — | N/A | integration | `cargo test -p gitctx-eval git_oracle_tests` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/gitctx-core/tests/qualname_tests.rs` — stubs for LINEAGE-01
- [ ] `crates/gitctx-core/tests/minhash_tests.rs` — stubs for LINEAGE-02 MinHash
- [ ] `crates/gitctx-core/tests/lineage_resolver_tests.rs` — stubs for LINEAGE-02 multi-tier resolver
- [ ] `crates/gitctx-core/tests/lineage_dag_tests.rs` — stubs for LINEAGE-03 DAG edges & uniqueness
- [ ] `crates/gitctx-eval/tests/lineage_mutation_bench.rs` — stubs for LINEAGE-04 M1-M10 benchmark
- [ ] `crates/gitctx-eval/tests/git_oracle_tests.rs` — stubs for LINEAGE-05 `git log -L` comparison

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| None | — | — | All phase behaviors have automated verification. |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 5s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** 2026-08-17

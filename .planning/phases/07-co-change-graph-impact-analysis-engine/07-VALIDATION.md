---
phase: 07
slug: co-change-graph-impact-analysis-engine
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-08-19
---

# Phase 07 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in harness) |
| **Config file** | Cargo.toml / crates/gitctx-query/Cargo.toml |
| **Quick run command** | `cargo test -p gitctx-query` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p gitctx-query`
- **After every plan wave:** Run `cargo test --workspace`
- **Before verification:** Full suite must be green (`cargo test --workspace`)
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 07-01-01 | 01 | 1 | GRAPH-01 | T-07-01 | Filter bulk commits (>25 files) & bot author patterns | unit | `cargo test -p gitctx-query --test cochange_test test_commit_size_filtering` | ✅ | ✅ green |
| 07-01-02 | 01 | 1 | GRAPH-01 | T-07-02 | Bot email pattern filtering (`[bot]`, `noreply`, etc.) | unit | `cargo test -p gitctx-query --test cochange_test test_bot_filtering` | ✅ | ✅ green |
| 07-01-03 | 01 | 1 | GRAPH-02 | T-07-03 | Time-decay exponential weighting & metric computation | unit | `cargo test -p gitctx-query --test cochange_test test_metric_correctness` | ✅ | ✅ green |
| 07-01-04 | 01 | 1 | GRAPH-04 | T-07-04 | In-memory `petgraph` UnGraph creation & neighborhood traversals | unit | `cargo test -p gitctx-query --test cochange_test test_graph_construction` | ✅ | ✅ green |
| 07-01-05 | 01 | 1 | GRAPH-03 | T-07-05 | 7-day follow-up window fix counting & churn detection | unit | `cargo test -p gitctx-query --test cochange_test test_impact_scoring` | ✅ | ✅ green |
| 07-01-06 | 01 | 1 | GRAPH-05 | T-07-06 | Code Maat logical coupling formula verification ($P(B\|A)$) | integration | `cargo test -p gitctx-query --test cochange_maat_validation` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Validation Sign-Off

- [x] All tasks have automated verification
- [x] Sampling continuity: zero failing or unverified tasks
- [x] No watch-mode flags
- [x] Feedback latency < 10s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** verified

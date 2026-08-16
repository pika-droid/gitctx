---
phase: 2
slug: corpus-selection-ground-truth-harvesting-leakage-sanitization
status: ready
nyquist_compliant: true
wave_0_complete: false
created: 2026-08-16
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust 1.82+) + `pytest` (Python 3.10+) |
| **Config file** | `Cargo.toml` (workspace) / `pytest.ini` |
| **Quick run command** | `cargo test -p gitctx-core -p gitctx-eval --lib` |
| **Full suite command** | `cargo xtask test-offline && pytest eval/tests/` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p gitctx-core -p gitctx-eval --lib` or `pytest eval/tests/test_<module>.py -q`
- **After every plan wave:** Run `cargo xtask test-offline && pytest eval/tests/`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 02-01-01 | 01 | 1 | CORP-01 | T-02-01 | Validate candidate repo metrics & .gitctxignore exclusions | unit | `pytest eval/tests/test_profile.py -k test_profile_metrics` | ❌ W0 | ⬜ pending |
| 02-01-02 | 01 | 1 | CORP-02 | T-02-02 | Evaluate hard selection gates and generate corpus.lock.toml | unit | `python eval/profile.py --offline` | ❌ W0 | ⬜ pending |
| 02-01-03 | 01 | 1 | CORP-02 | T-02-03 | Assert exact commit SHA lock & Invariant I6 enforcement | integration | `cargo test -p gitctx-eval --test invariant_i6_test` | ❌ W0 | ⬜ pending |
| 02-02-01 | 02 | 2 | CORP-03 | T-02-03 | Safe token handling, bot exclusion, & label capping | unit | `pytest eval/tests/test_harvest.py -k test_trailer_extraction` | ❌ W0 | ⬜ pending |
| 02-02-02 | 02 | 2 | CORP-03 | T-02-04 | Wilson Score CI calculation ($p \ge 0.85$) | unit | `pytest eval/tests/test_harvest.py -k test_wilson_ci` | ❌ W0 | ⬜ pending |
| 02-02-03 | 02 | 2 | CORP-03 | T-02-05 | JSONL dataset generation & xtask command wiring | unit | `pytest eval/tests/test_harvest.py` | ❌ W0 | ⬜ pending |
| 02-03-01 | 03 | 3 | CORP-04 | T-02-06 | 100% Python/Rust leakage sanitization parity | unit | `cargo test -p gitctx-core --test sanitize_parity_test` | ❌ W0 | ⬜ pending |
| 02-03-02 | 03 | 3 | CORP-05 | T-02-07 | Set B 4-category taxonomy & rich provenance schema | unit | `pytest eval/tests/test_set_b.py` | ❌ W0 | ⬜ pending |
| 02-03-03 | 03 | 3 | CORP-05 | T-02-08 | MANIFEST.toml BLAKE3 & SHA256 integrity check | integration | `cargo test -p gitctx-eval --test manifest_test` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/fixtures/sanitization_cases.json` — Shared cross-language test vectors for leakage stripping
- [ ] `eval/tests/test_profile.py` — Test suite for repository profiler and metric computation
- [ ] `eval/tests/test_harvest.py` — Test suite for ground truth extraction, bot filtering, and Wilson CI
- [ ] `eval/tests/test_sanitize.py` — Python test suite for `eval/sanitize.py`
- [ ] `eval/tests/test_set_b.py` — Python test suite for `eval/ground_truth/set_b_why.jsonl`
- [ ] `crates/gitctx-eval/tests/invariant_i6_test.rs` — Rust test for Invariant I6 locked SHA assertion
- [ ] `crates/gitctx-core/tests/sanitize_parity_test.rs` — Rust test for cross-language sanitization parity
- [ ] `crates/gitctx-eval/tests/manifest_test.rs` — Rust test for cryptographic ground truth manifest validation

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Harvester Precision Audit (30 samples) | CORP-03 | Requires human judgement to confirm commit diff fixes issue query | Run `python eval/harvest.py --audit 30` or `cargo xtask audit-harvester` to evaluate 30 pairs and record judgments in `eval/harvester_audit.md`. Confirm $p \ge 0.85$. |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 15s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** verified 2026-08-16

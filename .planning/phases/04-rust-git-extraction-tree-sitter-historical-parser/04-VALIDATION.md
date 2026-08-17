---
phase: 4
slug: rust-git-extraction-tree-sitter-historical-parser
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-08-17
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust native test harness (`cargo test`) |
| **Config file** | `Cargo.toml` |
| **Quick run command** | `cargo test -p gitctx-core` |
| **Full suite command** | `cargo xtask lint && cargo test --workspace` |
| **Estimated runtime** | ~8 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p gitctx-core`
- **After every plan wave:** Run `cargo xtask lint && cargo test --workspace`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 04-01-01 | 01 | 1 | PARSE-01 | — | First-parent commit walker orders topological commits & skips side branches | unit | `cargo test -p gitctx-core --test git_walker_tests test_first_parent_walk` | ❌ W0 | ⬜ pending |
| 04-01-02 | 01 | 1 | PARSE-01 | — | `git2` tree diffing identifies file deltas, additions, deletions, and hunks | unit | `cargo test -p gitctx-core --test git_walker_tests test_tree_diff_extraction` | ❌ W0 | ⬜ pending |
| 04-01-03 | 01 | 1 | PARSE-01 | — | Rename and copy detection properly associates old and new file paths | integration | `cargo test -p gitctx-core --test git_walker_tests test_rename_detection` | ❌ W0 | ⬜ pending |
| 04-02-01 | 02 | 1 | PARSE-02 | — | `LanguageRegistry` and `LanguageBehavior` extract functions, sigs, and docstrings across Python, Rust, JS/TS, Go, C/C++, Java, PHP | unit | `cargo test -p gitctx-core --test parser_tests test_multi_language_extraction` | ❌ W0 | ⬜ pending |
| 04-02-02 | 02 | 1 | PARSE-04 | — | Blob OID LRU cache prevents redundant AST re-parsing of identical blobs | unit | `cargo test -p gitctx-core --test parser_tests test_blob_oid_cache_hit` | ❌ W0 | ⬜ pending |
| 04-02-03 | 02 | 1 | PARSE-03 | — | Syntax errors degrade gracefully to `<file_level>` with `ParseStatus::Degraded` without panics | integration | `cargo test -p gitctx-core --test parser_tests test_broken_syntax_graceful_degradation` | ❌ W0 | ⬜ pending |
| 04-03-01 | 03 | 1 | PARSE-05 | — | Symmetric token diff extracts keyword-filtered `Added:` and `Removed:` identifier sets | unit | `cargo test -p gitctx-core --test template_tests test_symmetric_identifier_extraction` | ❌ W0 | ⬜ pending |
| 04-03-02 | 03 | 1 | PARSE-06 | — | Known secret prefixes and Shannon high-entropy tokens are redacted with typed placeholders | unit | `cargo test -p gitctx-core --test redact_tests test_secret_and_entropy_redaction` | ❌ W0 | ⬜ pending |
| 04-03-03 | 03 | 1 | PARSE-05 | — | D4 pseudo-document generator renders deterministic template chunks for functions and modules | integration | `cargo test -p gitctx-core --test template_tests test_d4_template_rendering` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/gitctx-core/Cargo.toml` — Add `tree-sitter`, language crates, `lru`, and `regex`
- [ ] `crates/gitctx-core/tests/git_walker_tests.rs` — Integration test harness for Git extraction
- [ ] `crates/gitctx-core/tests/parser_tests.rs` — Multi-language AST and degradation test harness
- [ ] `crates/gitctx-core/tests/template_tests.rs` — Template generation and identifier diff tests
- [ ] `crates/gitctx-core/tests/redact_tests.rs` — Secret redaction and Shannon entropy tests

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Multi-language parse inspection | PARSE-02 | Visual verification of AST node boundaries and signatures | Inspect sample rendered D4 chunks in test output across all 8 languages |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 10s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending

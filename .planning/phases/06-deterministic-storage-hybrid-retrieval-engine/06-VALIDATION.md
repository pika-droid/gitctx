---
phase: 06
slug: deterministic-storage-hybrid-retrieval-engine
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-08-18
---

# Phase 06 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in harness) |
| **Config file** | Cargo.toml / crates/gitctx-index/Cargo.toml / crates/gitctx-query/Cargo.toml |
| **Quick run command** | `cargo test -p gitctx-index -p gitctx-query` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p gitctx-index` or `cargo test -p gitctx-query`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd-verify-work`:** Full suite must be green (`cargo test --workspace`)
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 06-01-01 | 01 | 1 | INDEX-01 | T-06-01 | SQLite WAL mode, schema DDL, atomic .tmp swap | unit | `cargo test -p gitctx-index test_sqlite_storage_and_atomic_swap` | ❌ W0 | ⬜ pending |
| 06-01-02 | 01 | 1 | INDEX-02 | T-06-02 | Isolated explain_cache table, zero index leakage (I5) | unit | `cargo test -p gitctx-index test_explain_cache_isolation` | ❌ W0 | ⬜ pending |
| 06-01-03 | 01 | 1 | INVAR-03 | T-06-03 | Config hash & schema version mismatch refusal | unit | `cargo test -p gitctx-index test_config_hash_mismatch_refusal` | ❌ W0 | ⬜ pending |
| 06-02-01 | 02 | 2 | INDEX-03 | T-06-04 | Tantivy CodeTokenizer, camelCase/snake_case subtokenization | unit | `cargo test -p gitctx-index test_tantivy_code_tokenizer_and_bm25` | ❌ W0 | ⬜ pending |
| 06-02-02 | 02 | 2 | INDEX-03 | T-06-05 | Tantivy multi-field boosting & directory swap | unit | `cargo test -p gitctx-index test_tantivy_multi_field_boosts` | ❌ W0 | ⬜ pending |
| 06-03-01 | 03 | 2 | INDEX-04 | T-06-06 | FastEmbed CPU embeddings, vectors.bin layout & memmap2 | unit | `cargo test -p gitctx-index test_mmap_vector_storage_and_simd_scoring` | ❌ W0 | ⬜ pending |
| 06-03-02 | 03 | 2 | INVAR-04 | T-06-07 | Byte-identical double indexing determinism (I2) | integration | `cargo test -p gitctx-index test_double_indexing_determinism` | ❌ W0 | ⬜ pending |
| 06-03-03 | 03 | 2 | INVAR-05 | T-06-08 | Incremental vs cold index equivalence (I3) | integration | `cargo test -p gitctx-index test_incremental_vs_full_equivalence` | ❌ W0 | ⬜ pending |
| 06-04-01 | 04 | 3 | INDEX-05 | T-06-09 | Exact symbol fast-path & RRF k=60 fusion | unit | `cargo test -p gitctx-query test_rrf_hybrid_fusion_and_exact_symbol_fastpath` | ❌ W0 | ⬜ pending |
| 06-04-02 | 04 | 3 | INDEX-06 | T-06-10 | Deterministic lineage chain assembly by topo_order ASC | unit | `cargo test -p gitctx-query test_deterministic_chain_assembly` | ❌ W0 | ⬜ pending |
| 06-04-03 | 04 | 3 | INDEX-07 | T-06-11 | Structured Citation payload attached to all retrieved facts (I4) | unit | `cargo test -p gitctx-query test_citation_payload_completeness` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/gitctx-index/tests/storage_test.rs` — stubs for INDEX-01, INDEX-02, INVAR-03
- [ ] `crates/gitctx-index/tests/tantivy_test.rs` — stubs for INDEX-03
- [ ] `crates/gitctx-index/tests/vector_test.rs` — stubs for INDEX-04, INVAR-04, INVAR-05
- [ ] `crates/gitctx-query/tests/retrieval_test.rs` — stubs for INDEX-05, INDEX-06, INDEX-07

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 15s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending

# Phase 2 Plan 03: Leakage Sanitization Parity, Set B Benchmark, and Cryptographic Ground Truth Manifest Summary

**Execution Date:** 2026-08-16  
**Status:** Completed  
**Requirements Satisfied:** CORP-04, CORP-05  

---

## 1. Executive Summary

Phase 2 Plan 03 completed the empirical foundation and leakage defense architecture for `gitctx`. We delivered:
1. **Cross-Language Leakage Sanitization Parity**: Implemented deterministic 6-stage sanitization in Python (`eval/sanitize.py`) and Rust (`crates/gitctx-core/src/sanitize.rs`), verified against 22 shared test vectors in `tests/fixtures/sanitization_cases.json` ensuring 100% byte-for-byte output identity per D-09, D-10, and D-11.
2. **Hand-Authored Set B Why-Query Benchmark**: Curated 27 high-quality why-queries (`eval/ground_truth/set_b_why.jsonl`) distributed equally across the 3 pinned repositories (9 each for `psf/requests`, `tokio-rs/tokio`, and `gin-gonic/gin`) across all 4 pre-registered categories (`architectural_decision`, `bug_root_cause`, `api_evolution`, `performance_motivation`) with a 60/40 Dev/Test split (16 dev / 11 test) per D-13, D-14, D-15, and D-16.
3. **Cryptographic Ground Truth Manifest & Verification Engine**: Implemented `GroundTruthManifest` integrity checking in `crates/gitctx-eval/src/manifest.rs` and `cargo xtask verify-ground-truth`, locking BLAKE3 and SHA-256 digests in `eval/ground_truth/MANIFEST.toml` and integrating manifest verification into the invariant suite (`xtask test-invariants`).

---

## 2. Key Deliverables & Artifacts

### A. Shared Sanitization Vectors & Engines (CORP-04, D-09, D-10, D-11)
- **`tests/fixtures/sanitization_cases.json`**: 22 comprehensive test vectors covering:
  - Squash PR suffixes (`(#412)`, `(gh-987)`)
  - Action trailers (`Fixes:`, `Closes:`, `Resolves:`, `Ref:`, `Addresses:`, `Related-to:`, `Issue:`) with and without colons
  - Attribution trailers (`Signed-off-by:`, `Co-authored-by:`, `Reviewed-by:`, `Acked-by:`, `Suggested-by:`, `Reported-by:`, `Tested-by:`, `Approved-by:`, `Cc:`)
  - GitHub issue/PR URLs (`https://github.com/.../issues/123`, `/pull/456`)
  - Inline issue references (`#123`, `gh-456`)
  - Merge headers (`Merge pull request #123...`, `Merge branch...`)
  - Code identifiers and docstring preservation
  - Whitespace collapsing and normalization
- **`eval/sanitize.py`**: Python `sanitize_chunk_text(text: str) -> SanitizedChunk` for evaluation and ablation preprocessing.
- **`crates/gitctx-core/src/sanitize.rs`**: Rust `sanitize_chunk_text(text: &str) -> SanitizedChunk` for production indexing.
- **`eval/tests/test_sanitize.py` & `crates/gitctx-core/tests/sanitize_parity_test.rs`**: Dual test suites asserting cross-language parity.

### B. Hand-Authored Set B Why-Query Dataset (CORP-05, D-13, D-14, D-16)
- **`eval/ground_truth/set_b_why.jsonl`**: 27 hand-curated queries with rich schema (`qid`, `repo`, `query_set`, `category`, `query`, `discussion_source`, `gold_commits`, `gold_rationale`, `split`, `has_symbol_hint`, `curator_date`):
  - **`psf/requests`** (Python, Medium): 9 queries (e.g. Session connection pool decoupling, PreparedRequest intermediate serialization, socket timeout handling, cookie jar extraction, redirect auth stripping).
  - **`tokio-rs/tokio`** (Rust, Large): 9 queries (e.g. Cooperative task budgeting, work-stealing scheduler, intrusive task queues, lifo_slot cache optimization, non-reentrant async mutex, std::future transition).
  - **`gin-gonic/gin`** (Go, Small): 9 queries (e.g. `sync.Pool` context pooling, Radix tree routing vs regex, `Context.Copy()` async goroutine safety, `ShouldBind` decoupling, panic recovery buffering).
- **Distribution**:
  - `architectural_decision`: 7 queries
  - `bug_root_cause`: 7 queries
  - `api_evolution`: 7 queries
  - `performance_motivation`: 6 queries
  - Dev/Test Split: 16 Dev (59.3%) / 11 Test (40.7%)
- **`eval/tests/test_set_b.py`**: Automated validation test suite.

### C. Cryptographic Manifest & Offline Verification (D-08, D-15)
- **`eval/ground_truth/MANIFEST.toml`**: Pre-registered cryptographic hashes (BLAKE3 and SHA-256) and record counts for all three datasets (`set_a_issues.jsonl`, `revert_pairs.jsonl`, `set_b_why.jsonl`).
- **`crates/gitctx-eval/src/manifest.rs`**: Core integrity verification module computing BLAKE3 & SHA256 checksums and asserting zero tampering.
- **`crates/gitctx-eval/tests/manifest_test.rs`**: Integration test suite validating manifest integrity and detecting corrupted datasets.
- **`xtask/src/tasks/verify_ground_truth.rs`**: CLI task `cargo xtask verify-ground-truth` reporting verification status in a structured table.
- **`xtask/src/tasks/test_invariants.rs`**: Integrated Check 6 into project invariants verification suite.

---

## 3. Verification & Acceptance Criteria Results

| Verification Suite | Target | Status | Notes |
|---|---|---|---|
| Sanitization Parity (Python) | `eval/tests/test_sanitize.py` | Complete | Passes 22 test vectors with 100% byte identity and matching strip counts. |
| Sanitization Parity (Rust) | `crates/gitctx-core/tests/sanitize_parity_test.rs` | Complete | Passes 22 test vectors in Rust. |
| Set B Schema & Taxonomy | `eval/tests/test_set_b.py` | Complete | Asserts 27 records, 9 per repo, $\ge 5$ per category, 60/40 Dev/Test split. |
| Manifest Integrity (Rust) | `crates/gitctx-eval/tests/manifest_test.rs` | Complete | Verifies checksums and tests corruption detection. |
| CLI Verification Task | `cargo xtask verify-ground-truth` | Complete | Verifies all 3 datasets against `MANIFEST.toml`. |
| Invariants Suite | `cargo xtask test-invariants` | Complete | Integrated Check 6 (Ground Truth Cryptographic Integrity). |

---

## 4. Threat Mitigations Applied

| Threat ID | Threat Type | Component | Mitigation Implemented |
|---|---|---|---|
| `T-02-06` | Information Disclosure / Repudiation | Search Index / Retriever | Stripped squash suffixes, trailers, issue URLs, and `#123`/`gh-123` numbers across Python & Rust engines per D-10, D-11. |
| `T-02-07` | Tampering / Repudiation | Set B Why-Queries | Pre-registered all 27 Set B why-queries with gold rationales and BLAKE3/SHA256 digests in `MANIFEST.toml` before Phase 3 Python ablation per D-15. |
| `T-02-08` | Tampering | Cross-Language Divergence | Shared 22-vector test suite in `tests/fixtures/sanitization_cases.json` enforcing 100% byte-for-byte parity per D-09. |

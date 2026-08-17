# Phase 05 Plan 01: Universal QualName Engine & 128-Permutation MinHash Fingerprinting - Summary

**Executed:** 2026-08-17
**Status:** Completed
**Requirements Addressed:** `LINEAGE-01`, `LINEAGE-02`
**Decisions Enacted:** `D-01`, `D-02`, `D-03`, `D-04`, `D-12`

---

## Executive Summary

Phase 05 Plan 01 delivers the fundamental building blocks of function evolutionary lineage tracking for `gitctx`:
1. **Standardized Cross-Language `QualName` Engine (`LINEAGE-01`, `D-12`):** Universal symbol formatting across all 8 languages using double-colon `::` notation for namespaces/classes/methods (`Class::method`, `ReceiverType::Method`), angle-bracket scopes `::<nested_func>` for nested definitions, and `::<closure@Lline>` for anonymous closures/lambdas.
2. **Positional Lexical AST Body Normalizer (`D-01`):** Robust tokenizer that strips line/block/HTML comments, docstrings, and formatting whitespace, canonicalizes string/number literals into `<str>`/`<num>`, preserves language control keywords and structural operators, and maps local variable identifiers to positional placeholders (`var_0`, `var_1`, ...) in appearance order.
3. **Deterministic 128-Permutation MinHash Fingerprinting (`LINEAGE-02`, `D-02`, `D-03`, `D-04`):** Pure-Rust zero-dependency Murmur3-32 hashing with an immutable 128-seed table, $k=3$ token shingle sliding window, short function length gating (`< 10` tokens flagged as `is_short`), and fast bitwise Jaccard similarity estimation in $[0.0, 1.0]$.
4. **Complete Invariance Verification:** Unit and integration test suites validating 100% invariance (MinHash similarity = 1.0) on M5 (whitespace/formatting) and M6 (local variable renaming) transformations across Python, Rust, JavaScript, TypeScript, Go, C, C++, Java, and PHP.

---

## Tasks Completed

| Task | Name | Type | Status | Commits |
| :--- | :--- | :--- | :--- | :--- |
| `05-01-01` | End-to-end QualName parsing and MinHash fingerprinting tracer across Python and Rust | Tracer | ✅ Completed | `bac4144` (`feat(05-01): tracer for QualName parsing and MinHash fingerprinting`) |
| `05-01-02` | Implement standardized multi-language QualName conventions across all 8 languages | Auto | ✅ Completed | `e8d8e4c` (`feat(05-01): standardized QualName conventions across 8 languages`) |
| `05-01-03` | Implement Positional AST normalizer, Murmur3-32 hasher, 128-permutation MinHash generator, and invariance tests | Auto | ✅ Completed | `e1e16bf` (`feat(05-01): positional AST normalizer and 128-permutation MinHash generator`) |

---

## Key Artifacts & Changes

### Core Library (`crates/gitctx-core`)
- [`crates/gitctx-core/src/qualname.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-core/src/qualname.rs): Added `is_method()`, `is_nested_or_closure()`, `is_closure()`, parent/leaf extraction with double-colon and angle-bracket scope awareness.
- [`crates/gitctx-core/src/lineage/seeds.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-core/src/lineage/seeds.rs): Immutable static array of 128 deterministic `u32` hash seeds (`MINHASH_SEEDS`).
- [`crates/gitctx-core/src/lineage/normalizer.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-core/src/lineage/normalizer.rs): Positional AST normalizer mapping comments/docstrings/whitespace away, canonicalizing literals, preserving keywords, and replacing local variables with `var_0`, `var_1`.
- [`crates/gitctx-core/src/lineage/minhash.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-core/src/lineage/minhash.rs): Pure-Rust `murmur3_32` and `MinHashFingerprint` (`[u32; 128]`) with $k=3$ shingling, short function gating, and bitwise Jaccard estimation.
- [`crates/gitctx-core/src/lineage/mod.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-core/src/lineage/mod.rs): Lineage module exports.
- [`crates/gitctx-core/src/lib.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-core/src/lib.rs): Re-exported `AstNormalizer`, `MinHashFingerprint`, `murmur3_32`.
- [`crates/gitctx-core/src/parser/languages/*`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-core/src/parser/languages/): Standardized qualified name and closure extraction across Python, Rust, JavaScript, TypeScript, Go, C, C++, Java, and PHP.

### Tests
- [`crates/gitctx-core/tests/minhash_tests.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-core/tests/minhash_tests.rs): Tests for Murmur3 test vectors, short function gating (< 10 tokens), M5 formatting invariance, M6 variable renaming invariance, and deterministic fingerprint reproducibility.
- [`crates/gitctx-core/tests/qualname_tests.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-core/tests/qualname_tests.rs): Integration tests asserting correct qualified name extraction, parent/leaf resolution, and closure marker formatting across all 8 supported languages.

---

## Verification Results

1. **Unit and Integration Test Suite:**
   ```bash
   cargo test --workspace
   ```
   - `gitctx_core`: 24 library unit tests passed, 4 git walker tests passed, 6 minhash tests passed, 6 parser tests passed, 6 qualname tests passed, 10 redact tests passed, 3 sanitize tests passed, 8 template tests passed.
   - `gitctx_eval`: 3 fixture tests passed, 5 invariant I6 tests passed, 2 manifest tests passed, 3 mutation benchmark tests passed.
   - `xtask`: 2 audit harvester tests passed.
   - Total: 82 tests passed, 0 failed, 0 ignored.

2. **Invariance Validation:**
   - **M5 (Whitespace/Formatting):** Token stream and MinHash similarity = `1.0` (exact match).
   - **M6 (Local Variable Rename):** Token stream and MinHash similarity = `1.0` (exact match).
   - **Short Function Gating:** `< 10` normalized tokens flagged `is_short = true`.
   - **Determinism:** Identical 128-vector fingerprints across multiple independent runs.

---

## Deviations & Discoveries

- None. Implementation fully adhered to D-01, D-02, D-03, D-04, and D-12 as planned.

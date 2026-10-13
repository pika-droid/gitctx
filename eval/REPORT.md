# gitctx Comprehensive Evaluation Report

**Generated:** 2026-10-13T06:16:30Z
**Corpus:** 3 repositories pinned in `eval/corpus.lock.toml`
**Ground Truth:** Set A (260 issue-closing pairs), Set B (27 why-queries)
**Random Seed:** 42

---

## 1. Retrieval Ablation Grid

Full grid over 5 document representations × 5 retrievers, evaluated on 27 queries against 100 indexed documents.

### Recall@5

| Rep \\ Ret | R1 | R2 | R3 | R4 | R5 |
|-----------|-------|-------|-------|-------|-------|
| **D1** | 0.5556 | 0.5926 | 0.5926 | 0.5926 | 0.5926 |
| **D2** | 0.7778 | 0.8148 | 0.7778 | 0.8148 | 0.7778 |
| **D3** | 0.4815 | 0.7037 | 0.5556 | 0.5185 | 0.5926 |
| **D4** | 0.9259 | 0.9259 | 0.8889 | 0.8889 | 0.8889 |
| **D5** | 0.9630 | 0.8889 | 0.8889 | 0.8889 | 0.8889 |

### Recall@10

| Rep \\ Ret | R1 | R2 | R3 | R4 | R5 |
|-----------|-------|-------|-------|-------|-------|
| **D1** | 0.8148 | 0.8148 | 0.8148 | 0.8148 | 0.8148 |
| **D2** | 0.8889 | 0.8148 | 0.8519 | 0.8519 | 0.8148 |
| **D3** | 0.8519 | 0.8148 | 0.8148 | 0.8519 | 0.8148 |
| **D4** | 1.0000 | 0.9259 | 0.9259 | 0.9259 | 0.9630 |
| **D5** | 1.0000 | 0.9259 | 0.9259 | 0.9259 | 0.9630 |

### Recall@20

| Rep \\ Ret | R1 | R2 | R3 | R4 | R5 |
|-----------|-------|-------|-------|-------|-------|
| **D1** | 0.8519 | 0.8148 | 0.8148 | 0.8148 | 0.8148 |
| **D2** | 0.9259 | 0.8519 | 0.8519 | 0.8519 | 0.8519 |
| **D3** | 0.9259 | 0.8519 | 0.8519 | 0.8519 | 0.8519 |
| **D4** | 1.0000 | 0.9630 | 0.9630 | 0.9630 | 0.9630 |
| **D5** | 1.0000 | 0.9630 | 0.9630 | 0.9630 | 0.9630 |

### MRR (Mean Reciprocal Rank)

| Rep \\ Ret | R1 | R2 | R3 | R4 | R5 |
|-----------|-------|-------|-------|-------|-------|
| **D1** | 0.4582 | 0.5553 | 0.5144 | 0.5423 | 0.5736 |
| **D2** | 0.6058 | 0.7523 | 0.6164 | 0.7022 | 0.7383 |
| **D3** | 0.4335 | 0.6784 | 0.4852 | 0.4827 | 0.5739 |
| **D4** | 0.9342 | 0.9108 | 0.8966 | 0.8790 | 0.8608 |
| **D5** | 0.9424 | 0.8975 | 0.8966 | 0.8790 | 0.8608 |

---

## 2. Phase 0 Decision Gate

| Metric | Value |
|--------|-------|
| Best template config | D4+R1 (Recall@10 = 1.0000) |
| Best baseline config | D2+R1 (Recall@10 = 0.8889) |
| ΔRecall@10 | 0.1111 |
| Threshold | ≥ 0.05 |
| Gate | ✅ PASSED |

---

## 3. Function Lineage Engine Calibration

Thresholds calibrated via grid sweep over synthetic mutations M1-M10.

| Parameter | Value |
|-----------|-------|
| τ_same (same-file MinHash) | 0.55 |
| τ_cross (cross-file MinHash) | 0.65 |
| τ_decoy (decoy rejection) | 0.20 |
| Overall F1 | 1.000 |
| M8 Decoy Rejection Rate | 100.0% |
| Chain Recovery (D=5) | 100.0% |
| Grid Points Evaluated | 66 |
| Calibrated At | 2026-08-17T12:00:00Z |

---

## 4. Pre-Registered Failure Taxonomy

All retrieval misses (Recall@10 < 1.0 in any configuration) are classified into the following taxonomy, extrapolated from Codanna's failure classification patterns for code intelligence systems.

| Bucket | Count | Affected Queries |
|--------|-------|------------------|
| Cross-File Rename Miss | 2 | why-req-003, why-gin-008 |

### Taxonomy Reference

| Category | Description |
|----------|-------------|
| AST Parse Degraded | Tree-sitter returned `has_error()` on a historical blob, preventing function boundary extraction. Chunk emitted at file level with `parse_status = degraded`. |
| Cross-File Rename Miss | A function was moved to a different file but the 4-tier lineage matcher failed to bridge the MinHash fingerprint across the git rename boundary. |
| Directory Move Miss | A function was relocated to a different directory tree. The scoped cross-file MinHash search (Tier 4) did not reach the target directory. |
| Leakage Residual | An issue reference (`Fixes #N`, PR URL, or squash suffix) survived the leakage stripping pipeline and appeared in indexed chunk text. |
| Tokenization Boundary | The camelCase/snake_case tokenizer produced boundary splits that prevented exact symbol matching (e.g., `getValue` split as `get` + `Value` vs `get_value`). |
| Sparse History Gap | The commit touched fewer than 2 functions, producing insufficient retrieval signal for BM25 and dense scoring to surface the correct result. |
| Decoy False Positive | A delete+add pair was incorrectly accepted as lineage continuation despite the decoy rejection threshold (τ_decoy). The new function body was semantically distinct. |
| Dense Retrieval Misalignment | The FastEmbed dense embedding scored an unrelated function higher than the true match due to domain vocabulary overlap in the embedding space. |
| Vocabulary Gap | The natural-language query used terminology not present in any indexed template pseudo-document, causing zero BM25 matches. |
| Large Commit Corruption | A merge or bulk-reformat commit exceeding the 25-file threshold was not properly filtered, corrupting co-change coupling statistics. |

---

## 5. Evaluation Corpus

Pinned repositories from `eval/corpus.lock.toml` (Invariant I6):

| ID | Repository | Language | Size | Commits | Traceable Issues | Merge % |
|----|------------|----------|------|---------|------------------|---------|
| gin | gin | go | small | 2023 | 1159 | 11% |
| tokio | tokio | rust | medium | 4783 | 4299 | 4% |
| requests | requests | python | large | 6495 | 333 | 25% |

---

## 6. Ground Truth Integrity

Cryptographic hashes from `eval/ground_truth/MANIFEST.toml`:

```toml
# Cryptographic Ground Truth Manifest
# Locks dataset integrity before Phase 3 Python ablation (D-08, D-15)
version = 1
locked_date = "2026-08-16T21:00:00Z"

[files."revert_pairs.jsonl"]
count = 42
blake3 = "7ef9e16bfa72ca199ae5a470c60eaf9c604eda0b97b5745f28300e81bd422b33"
sha256 = "d092d3a984f5b1265886baed7cd0331d296d4e4a9990aedbc2fc9f9e38090bda"

[files."set_a_issues.jsonl"]
count = 260
blake3 = "ab9ab49fe437af4d46584ec7fbfa6e3ae19c21577c7679a9db4e984218a7743e"
sha256 = "d08553bd06bca95937481f8f696468380307c062b1a9e0f6cd6b5d4d2676842e"

[files."set_b_why.jsonl"]
count = 27
blake3 = "fc636c35a14bd7a3c583b9ea90e6c18b13248f085d9e5f204ac9c450a0a3276c"
sha256 = "c71053f4970252209a62c10bcd752c83227a217b57dc276cee349f52c3adf0b7"
```

---

## 7. Reproducibility

To reproduce this evaluation from scratch:

```bash
# 1. Clone and pin the evaluation corpus
cargo xtask clone-corpus

# 2. Verify ground truth integrity
cargo xtask verify-ground-truth

# 3. Run lineage benchmark evaluation
cargo xtask eval bench-lineage --format markdown

# 4. Run threshold calibration sweep
cargo xtask eval tune-thresholds

# 5. Generate this evaluation report
cargo xtask eval report

# 6. Run full invariant and determinism suite
cargo xtask test-invariants
cargo xtask bench
```

All commands execute fully offline with zero network dependencies (Invariant I1).

---
*Report generated by `cargo xtask eval report` — gitctx Phase 11*

---
phase: 06
plan: "06-03"
title: "FastEmbed Offline CPU Embeddings, Contiguous Mmap Vector Storage, Rayon SIMD Cosine Scoring, and Invariant Verification"
status: complete
completed_at: "2026-08-18T13:28:00+05:30"
duration: "14m"
tasks_completed: 3
total_tasks: 3
commits:
  - hash: "c2af153"
    message: "feat(06-03): implement fastembed offline embedding generation, mmap vector store, rayon SIMD scoring, and invariant tests"
    type: "feat"
requirements_covered:
  - INDEX-04
  - INVAR-04
  - INVAR-05
key_files:
  created:
    - crates/gitctx-index/src/vector/mod.rs
    - crates/gitctx-index/src/vector/format.rs
    - crates/gitctx-index/src/vector/mmap.rs
    - crates/gitctx-index/src/vector/simd.rs
    - crates/gitctx-index/src/vector/embed.rs
    - crates/gitctx-index/tests/vector_test.rs
  modified:
    - Cargo.toml
    - crates/gitctx-index/Cargo.toml
    - crates/gitctx-index/src/lib.rs
verification_results:
  - command: "cargo test -p gitctx-index --test vector_test"
    passed: 5
    failed: 0
  - command: "cargo test -p gitctx-index"
    passed: 12
    failed: 0
---

# Phase 06 Plan 03: Flat Dense Vector Storage & SIMD Retrieval Summary

Implemented the dense vector storage engine in `gitctx-index` (`INDEX-04`), featuring offline CPU embedding generation (`D-09`), contiguous binary vector file layout with zero-copy `memmap2` reader (`D-10`), write-time unit L2 normalization and Rayon parallel SIMD cosine dot product search (`D-11`), chunk batching and parse status filtering (`D-12`), and cross-cutting invariant test suites for double-indexing determinism (`INVAR-04` / Invariant I2) and incremental vs. cold equivalence (`INVAR-05` / Invariant I3).

## Key Deliverables & Architecture

1. **FastEmbed Offline CPU Embedding Generator (`D-09`, `D-12`, `INDEX-04`):**
   - Implemented [`FastEmbedGenerator`](file:///c:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-index/src/vector/embed.rs) powered by `fastembed-rs` (pinned to 5.6.0) utilizing local ONNX Runtime CPU execution.
   - Built batched inference pipeline (`generate_embeddings_batched`) processing 128–256 chunks per batch.
   - Implemented parse status filtering per `D-12`: generates embeddings for `ParseStatus::Ok` and `ParseStatus::Degraded`, while skipping `ParseStatus::Failed`.

2. **Contiguous Binary Vector Format & Atomic Swap (`D-10`, `INDEX-04`):**
   - Implemented 16-byte fixed binary header (`VectorHeader` in [`format.rs`](file:///c:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-index/src/vector/format.rs)) containing `magic: b"GVEC"`, `version: 1`, `dimension: 384`, and `count: N` in little-endian IEEE 754 format.
   - Built atomic replacement serialization (`write_vector_file_atomic`) staging to `.gitctx/vectors.bin.tmp` before swap.

3. **Zero-Copy Memory-Mapped Reader (`D-10`, `INDEX-04`):**
   - Implemented [`MmapVectorStorage`](file:///c:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-index/src/vector/mmap.rs) using `memmap2::MmapOptions`.
   - Performs strict validation on open: magic bytes, version, dimension, total file size, and cross-checks vector count with SQLite `embeddings` table.
   - Provides zero-copy slice retrieval (`get_vector(row_offset) -> &[f32]`).

4. **Write-Time Unit L2 Normalization & Rayon SIMD Cosine Search (`D-11`, `INDEX-04`):**
   - Vectors are unit L2-normalized at write time ($\hat{\mathbf{v}} = \mathbf{v} / \|\mathbf{v}\|_2$) via [`normalize_l2`](file:///c:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-index/src/vector/simd.rs).
   - At query time, unit query vector dot product over `&[f32]` is computed with unrolled 4-way SIMD (`dot_product_simd`) parallelized across CPU cores via `rayon::into_par_iter()`.
   - Returns top-$K$ scored matches (`search_cosine_top_k`) with deterministic tie-breaking.

5. **Cross-Cutting Invariant Verification (`INVAR-04`, `INVAR-05`):**
   - **Invariant I2 / INVAR-04 (`test_double_indexing_determinism`):** Re-indexing identical repository state across independent directories produces byte-for-byte identical SQLite `index.db` (after WAL truncate checkpoint) and `vectors.bin` files, with exact identical query rankings.
   - **Invariant I3 / INVAR-05 (`test_incremental_vs_full_equivalence`):** Incremental indexing across sequential commit batches produces identical relational rows, embedding offsets, and binary vector payloads to a full cold index.

## Verification

```bash
cargo test -p gitctx-index --test vector_test
```
Results:
- `test_mmap_vector_storage_and_simd_scoring`: passed
- `test_double_indexing_determinism`: passed (Invariant I2)
- `test_incremental_vs_full_equivalence`: passed (Invariant I3)
- `test_vector_simd_scoring_and_incremental`: passed
- `test_fastembed_batch_embedding_and_status_filtering`: passed

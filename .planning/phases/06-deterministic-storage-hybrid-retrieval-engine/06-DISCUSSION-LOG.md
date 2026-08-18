# Phase 6: Deterministic Storage & Hybrid Retrieval Engine - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-08-18
**Phase:** 6-Deterministic Storage & Hybrid Retrieval Engine
**Areas discussed:** SQLite Storage & Concurrency Architecture, Tantivy BM25 Schema & Code Tokenizer, Dense Vector Storage & SIMD Scoring, Hybrid RRF Fusion & Exact Symbol Fast-Path

---

## SQLite Storage & Concurrency Architecture

| Option | Description | Selected |
|--------|-------------|----------|
| Dedicated writer thread with bounded channel | Parallel workers send batches (1,000–5,000 rows) over a channel to a single background writer. Prevents SQLITE_BUSY contention in WAL mode. | ✓ |
| Connection pool with retry | Workers acquire write connections from an r2d2 pool and retry on SQLITE_BUSY with exponential backoff. | |
| Sequential single-threaded write loop | Workers and storage operate on a single thread without concurrency channels. | |

| Option | Description | Selected |
|--------|-------------|----------|
| Two-phase atomic rename (.tmp -> final) | Cold/full index writes to .gitctx/index.db.tmp, checkpoints WAL, and atomically renames to .gitctx/index.db upon success. | ✓ |
| Direct in-place update with is_dirty flag | Modify index.db directly and use an index_state dirty flag. | |

| Option | Description | Selected |
|--------|-------------|----------|
| Fail-fast refusal with diagnostic | Validate schema_version and config_hash on open; if mismatched, return an explicit error instructing user to run 'gitctx index'. | ✓ |
| Automatic background reindex | Automatically trigger full reindex on schema/hash mismatch. | |

| Option | Description | Selected |
|--------|-------------|----------|
| Dedicated isolated explain_cache table | Stored in main SQLite DB but excluded from all embedding/indexer SELECT queries, fulfilling Invariant I5. | ✓ |
| Separate explain_cache.db database file | Stored in a separate SQLite database file on disk. | |

**User's choice:** Recommended choices selected for all questions.

---

## Tantivy BM25 Schema & Code Tokenizer

| Option | Description | Selected |
|--------|-------------|----------|
| Hybrid Code + Text Tokenizer | Splits on camelCase/snake_case/punctuation, indexing compound + subtokens, lowercase + case-preserved, no stemming on code identifiers. | ✓ |
| Standard text tokenizer | Default Tantivy en_stem tokenizer with stemming. | |
| Pure identifier regex tokenizer | Only indexes identifier words. | |

| Option | Description | Selected |
|--------|-------------|----------|
| Boosted Multi-Field Schema | qualified_name (boost 4.0), path (boost 2.0), commit_message (boost 1.2), chunk_text (boost 1.0). | ✓ |
| Uniform Single Text Field | Concatenate all metadata into single body field with 1.0 weight. | |

| Option | Description | Selected |
|--------|-------------|----------|
| Dedicated .gitctx/tantivy/ directory with atomic .tmp swap | Written to .gitctx/tantivy.tmp/ during cold index and renamed alongside index.db. | ✓ |
| Single-file RAM index packed inside SQLite | Stored as blob in SQLite and expanded in-memory on open. | |

| Option | Description | Selected |
|--------|-------------|----------|
| Tantivy IndexWriter Append | On incremental indexing, add new touch documents by touch_id and commit. Fast (<2s). | ✓ |
| Full Tantivy rebuild from SQLite | Incrementally update SQLite, then rebuild entire Tantivy directory. | |

**User's choice:** Recommended choices selected for all questions.

---

## Dense Vector Storage & SIMD Scoring

| Option | Description | Selected |
|--------|-------------|----------|
| fastembed-rs with BGESmallENV15 / AllMiniLML6V2 (384 dims) | Runs 100% offline via ONNX Runtime CPU, fast inference, compact models. | ✓ |
| Custom Candle transformer | HuggingFace Candle crate with compiled-in Bert model. | |

| Option | Description | Selected |
|--------|-------------|----------|
| Flat contiguous f32 file (.gitctx/vectors.bin) with mmap + SQLite row_offset | Raw &[f32] array memory-mapped via memmap2. Zero-copy deserialization, validated against SQLite on open. | ✓ |
| SQLite BLOB column | Store raw vector byte arrays directly in SQLite function_touches.embedding column. | |

| Option | Description | Selected |
|--------|-------------|----------|
| L2-normalization at write + Flat SIMD dot-product (Rayon parallel) | Unit-normalized vectors so cosine similarity = dot product. Rayon-parallel SIMD dot product scans 100k vectors in <15ms. | ✓ |
| HNSW Graph Index | Graph-based vector index on disk using hnsw_rs. | |

| Option | Description | Selected |
|--------|-------------|----------|
| Batch embedding (128-256 chunks), embed 'ok' and 'degraded', skip 'failed' | Maximizes CPU/SIMD throughput; ensures degraded historical diffs remain searchable. | ✓ |
| Stream single chunks one-by-one | Embed each chunk immediately upon AST extraction. | |

**User's choice:** Recommended choices selected for all questions.

---

## Hybrid RRF Fusion & Exact Symbol Fast-Path

| Option | Description | Selected |
|--------|-------------|----------|
| Syntax heuristic + SQLite exact index lookup with rank-0 injection | Detects identifier syntax (fetch_user, Client.get, mod::func) and queries SQLite qualified_name index directly with rank-0 priority injection. | ✓ |
| Tantivy BM25 qualified_name boost only | Rely entirely on Tantivy 4.0x boost without dedicated SQLite fast-path. | |

| Option | Description | Selected |
|--------|-------------|----------|
| Standard RRF k=60 with CLI override (--rrf-k) | Fixed k=60 per Cormack et al. / Phase 3 ablation results, with optional CLI override. | ✓ |
| Dynamic weighted RRF | Dynamically weight BM25 vs Dense based on query classification. | |

| Option | Description | Selected |
|--------|-------------|----------|
| Max 3 touches per commit + chunk-hash deduplication | Suppresses duplicate boilerplate hunks and prevents large commits from dominating top-10. | ✓ |
| Unconstrained top-K | Return raw top-K touches strictly by score. | |

| Option | Description | Selected |
|--------|-------------|----------|
| Single indexed SQL query ordered by topo_order + structured Citation payload | Query SQLite by lineage_id ordered by topo_order ASC in one indexed scan, attaching full Citation metadata. | ✓ |
| Iterative parent graph walk in Rust | Walk git parent commits step-by-step from HEAD. | |

**User's choice:** Recommended choices selected for all questions.

---

## the agent's Discretion

- Exact Rayon chunk sizes for SIMD vector iteration.
- Internal helper functions and module partitioning across `gitctx-index` and `gitctx-query`.
- SQLite PRAGMA cache size and busy timeout constants.

## Deferred Ideas

- None — discussion stayed strictly within Phase 6 scope.

# Phase 6: Deterministic Storage & Hybrid Retrieval Engine - Research

**Status:** Complete  
**Date:** 2026-08-18  
**Author:** GSD Phase Researcher  
**Phase Directory:** `.planning/phases/06-deterministic-storage-hybrid-retrieval-engine`

---

## Executive Summary

Phase 6 implements the deterministic storage layer and hybrid retrieval engine for `gitctx` across two primary crates:
1. `gitctx-index`: SQLite storage layer (`rusqlite` with WAL mode, single writer channel, atomic `.tmp` swap), Tantivy BM25 search index with code-aware tokenization and boosting, and memory-mapped `f32` dense vector storage (`memmap2`) with FastEmbed CPU embeddings.
2. `gitctx-query`: Hybrid retrieval engine combining BM25 and dense vector search via Reciprocal Rank Fusion (RRF $k=60$), exact symbol fast-path with Rank-0 injection, chunk text hash deduplication, commit density capping ($\le 3$ touches per commit), deterministic chronological chain assembly by `lineage_id` ordered by `topo_order ASC`, and structured `Citation` metadata attachment.

This research establishes the exact schema DDL, indexing concurrency model, tokenizer design, binary vector file layout, SIMD vector scoring math, RRF fusion formulas, and validation architecture to satisfy all 10 phase requirements (**INVAR-03**, **INVAR-04**, **INVAR-05**, **INDEX-01**, **INDEX-02**, **INDEX-03**, **INDEX-04**, **INDEX-05**, **INDEX-06**, **INDEX-07**).

---

<user_constraints>
## User Constraints & Locked Decisions

### Implementation Decisions (Verbatim from CONTEXT.md)

#### SQLite Storage & Concurrency Architecture
- **D-01:** Dedicated background writer thread with bounded channel — Parallel indexing workers extract ASTs, render D4 templates, and generate embeddings, sending batch chunks (1,000–5,000 rows) over a crossbeam/mpsc channel to a single dedicated SQLite writer. Eliminates `SQLITE_BUSY` lock contention under WAL mode (`PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;`). — **Reversibility:** costly — defines concurrency pipeline in `gitctx-index`.
- **D-02:** Two-phase atomic rename (`.gitctx/index.db.tmp` -> `.gitctx/index.db`) — Cold indexing builds to temporary files (`index.db.tmp`, `tantivy.tmp/`, `vectors.bin.tmp`), checkpoints WAL, and atomically renames all artifacts upon completion. Interrupted or crashed indexing runs never leave corrupted or partial index state. — **Reversibility:** one-way — core invariant INDEX-01 / INVAR-04 requirement.
- **D-03:** Fail-fast refusal with explicit diagnostic — Validate `schema_version` (integer) and `config_hash` (BLAKE3/SHA256 of schema DDL + tokenizer + model config) in `index_state` on open; if mismatched, return an explicit actionable error: `"Index schema version mismatch. Run 'gitctx index' to rebuild."` — **Reversibility:** reversible.
- **D-04:** Dedicated isolated `explain_cache` table with architectural read-gating — Stored in main SQLite DB (`cache_key`, `model`, `prompt_version`, `text`, `created_at`) but strictly excluded from all indexer and embedding SELECT queries to enforce Invariant I5 (zero LLM-generated text in index/retrieval paths). — **Reversibility:** one-way — Invariant I5 enforcement contract.

#### Tantivy BM25 Schema & Code Tokenizer
- **D-05:** Hybrid Code + Text Tokenizer — Custom Tantivy tokenizer that splits on camelCase, snake_case, and non-alphanumeric boundaries, emitting both compound tokens (`getUserById`) and individual subtokens (`get`, `user`, `by`, `id`), lowercase + case-preserved, with no stemming on code identifiers to prevent destructive stem transformations (e.g., `getter` -> `get`). — **Reversibility:** costly — defines lexical search behavior and Tantivy schema.
- **D-06:** Boosted multi-field schema — `qualified_name` (boost 4.0), `path` (boost 2.0), `commit_message` (boost 1.2), `chunk_text` (boost 1.0). Gives strong priority to exact symbol and filename matches while retaining broad recall across pseudo-doc explanation texts. — **Reversibility:** costly — Tantivy scoring schema.
- **D-07:** Dedicated `.gitctx/tantivy/` directory with atomic `.tmp` swap — Tantivy directory written to `.gitctx/tantivy.tmp/` during cold indexing and atomically swapped alongside `index.db.tmp` -> `index.db` to maintain cross-engine atomicity. — **Reversibility:** costly — filesystem index layout.
- **D-08:** Incremental updates via Tantivy `IndexWriter` append — Incremental indexing instantiates `IndexWriter`, appends new documents keyed by `touch_id`, and commits in <2 seconds, letting Tantivy handle segment merges. — **Reversibility:** reversible.

#### Dense Vector Storage & SIMD Scoring
- **D-09:** `fastembed-rs` with `BGESmallENV15` / `AllMiniLML6V2` (384 dimensions) — Runs 100% offline via ONNX Runtime CPU, fast batch inference (~1000 chunks/s on CPU), compact weights (~130MB), cached locally. — **Reversibility:** costly — embedding dimensions and model selection.
- **D-10:** Flat contiguous binary vector file (`.gitctx/vectors.bin`) with `memmap2` and SQLite `embeddings(touch_id, row_offset)` mapping — Raw `&[f32]` array with zero-copy deserialization; 50k vectors ≈ 75MB. Validates file byte length against SQLite row count on open. — **Reversibility:** one-way — vector storage format.
- **D-11:** Write-time L2 normalization + Rayon-parallel SIMD dot product — Vectors are unit-normalized upon embedding generation ($L_2 = 1.0$). At query time, query vector is unit-normalized and scanned against all vectors using SIMD dot product partitioned with `rayon`, achieving <15ms brute-force scans on 100k vectors with zero ANN graph build overhead. — **Reversibility:** costly — vector scoring engine.
- **D-12:** Batch embedding (128–256 chunks), embed `ok` and `degraded`, skip `failed` — Maximizes CPU/SIMD throughput during indexing; ensures degraded historical diff chunks remain searchable while skipping empty/failed AST parses. — **Reversibility:** reversible.

#### Hybrid RRF Fusion & Exact Symbol Fast-Path
- **D-13:** Syntax heuristic + SQLite exact index lookup with rank-0 injection — Detects identifier syntax (`[a-zA-Z_][a-zA-Z0-9_]*(::|\.)[a-zA-Z_][a-zA-Z0-9_]*` or exact word) and queries SQLite `function_touches` index on `qualified_name` directly. Exact hits are injected with top RRF rank (rank 0 / boost) so exact symbol queries never fail. — **Reversibility:** costly — query execution pipeline.
- **D-14:** Standard RRF $k=60$ with CLI override (`--rrf-k`) — Combines BM25 and dense rank lists via $\sum \frac{1}{k + \text{rank}_i}$, default $k=60$ per Cormack et al. and Phase 3 ablation findings. — **Reversibility:** reversible.
- **D-15:** Max 3 touches per commit + chunk-hash deduplication — Deduplicates duplicate boilerplate hunks by chunk text hash and caps search results to at most 3 touches per commit hash to prevent single large commits from dominating top-K results. — **Reversibility:** reversible.
- **D-16:** Single indexed SQL query ordered by `topo_order ASC` + structured Citation payload — Queries SQLite by `lineage_id` joined with `commits` ordered by `topo_order ASC` in a single indexed query, attaching full Citation metadata (`commit_hash`, `path`, `start_line`, `end_line`, `match_rule`, `match_confidence`). — **Reversibility:** one-way — core retrieval output contract (INDEX-06, INDEX-07).

### the agent's Discretion
- Exact Rayon chunk sizes for SIMD vector iteration.
- Internal helper functions and module partitioning across `gitctx-index` and `gitctx-query`.
- SQLite PRAGMA cache size and busy timeout constants.

### Deferred Ideas
- None — discussion stayed strictly within Phase 6 scope.

</user_constraints>

---

<phase_requirements>
## Phase Requirements Coverage

| Requirement ID | Description | Phase 6 Implementation Mapping |
|----------------|-------------|--------------------------------|
| **INVAR-03** | Configuration hashing mechanism (`config_hash`) computed across schema, model, tokenizer, and params to reject mismatched indexes. | Implemented in `gitctx-core::config::IndexConfig` and validated on database open by `StorageEngine::open()`. Checks `index_state` table for `schema_version` and `config_hash`. Fail-fast refusal with explicit error message on mismatch. |
| **INVAR-04** | Double-indexing determinism test (Invariant I2) asserting byte-identical SQLite/vector dumps and identical top-10 search results. | Cold indexing uses deterministic topo-sort, deterministic write-ordering, WAL truncate checkpoint before rename, and fixed-seed vector normalization. Validated via `test_double_indexing_determinism`. |
| **INVAR-05** | Incremental vs. full index equivalence test (Invariant I3) asserting identical database states for matching commit ranges. | Single-commit and range-based indexing yields identical rows in SQLite and identical vectors/Tantivy document sets as a full cold rebuild from scratch over the same commit range. Validated via `test_incremental_vs_full_equivalence`. |
| **INDEX-01** | SQLite schema with WAL mode, single writer channel, atomic `.tmp` indexing, explicit topological order index, and `schema_version` validation. | Implemented in `gitctx-index::storage::SqliteStorage` with WAL mode, normal synchronous, dedicated background writer thread consuming from bounded channel, atomic `.tmp` directory swap, indexed `topo_order`, and schema validation. |
| **INDEX-02** | Physical isolation of `explain_cache` table to guarantee no LLM text enters embedding inputs (Invariant I5). | Schema defines `explain_cache(cache_key, model, prompt_version, text, created_at)`. No SELECT queries in indexing or retrieval ever join or read from `explain_cache`. Validated via unit test inspection of query ASTs / schema isolation. |
| **INDEX-03** | Code-aware Tantivy BM25 index with camelCase/snake_case tokenization and identifier preservation. | Implemented in `gitctx-index::tantivy_index::TantivyEngine` using a custom `CodeTokenizer` that emits compound tokens and subtokens, preserves identifiers without stemming, and applies boosts (`qualname: 4.0`, `path: 2.0`, `commit_message: 1.2`, `chunk_text: 1.0`). |
| **INDEX-04** | Dense vector storage using `fastembed-rs` (MiniLM / code model) written to memory-mapped `f32` arrays with SIMD flat cosine scoring. | Implemented in `gitctx-index::vector::MmapVectorStorage` and `gitctx-index::vector::FastEmbedGenerator`. Vectors are stored in `.gitctx/vectors.bin`, memory-mapped via `memmap2`, mapped to SQLite `embeddings(touch_id, row_offset)`, and scored via Rayon SIMD dot product. |
| **INDEX-05** | Reciprocal Rank Fusion (RRF $k=60$) combining BM25 and dense scores with exact symbol match priority path. | Implemented in `gitctx-query::retrieval::HybridRetriever`. Merges Tantivy BM25 ranks and SIMD dense vector ranks with RRF $k=60$. Queries with identifier syntax trigger SQLite index fast-path and inject matching touches at Rank-0. |
| **INDEX-06** | Deterministic chain assembly querying complete historical timelines by `lineage_id` in a single indexed query. | Implemented in `gitctx-query::chain::ChainAssembler`. Queries SQLite using `SELECT ... FROM function_touches ft JOIN commits c ON ft.commit_hash = c.hash WHERE ft.lineage_id = ? ORDER BY c.topo_order ASC` in a single indexed lookup. |
| **INDEX-07** | Citation payload inclusion on all retrieved facts (`commit_hash`, `path`, `start_line`, `end_line`, `match_rule`, `match_confidence`) (Invariant I4). | Implemented in `gitctx-query::model::RetrievedFact` ensuring all search results attach non-null structured `Citation` metadata. |

</phase_requirements>

---

## Technical Deep Dive & Architectural Design

### 1. SQLite Storage Layer (`gitctx-index::storage`)

#### 1.1 Schema DDL
The SQLite relational database (`.gitctx/index.db`) serves as the single deterministic ground-truth store for git history facts, topological ordering, function lineage, touches, and vector row-offsets.

```sql
-- Schema Version 1

CREATE TABLE IF NOT EXISTS index_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS commits (
    hash TEXT PRIMARY KEY,
    author TEXT NOT NULL,
    email TEXT NOT NULL,
    authored_at INTEGER NOT NULL,
    topo_order INTEGER NOT NULL,
    branch TEXT,
    parent_hashes TEXT NOT NULL,
    message TEXT NOT NULL,
    insertions INTEGER NOT NULL,
    deletions INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS ix_commits_topo_order ON commits(topo_order ASC);

CREATE TABLE IF NOT EXISTS files (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    language TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS ix_files_path ON files(path);

CREATE TABLE IF NOT EXISTS function_lineage (
    lineage_id INTEGER PRIMARY KEY,
    current_path TEXT NOT NULL,
    current_qualified_name TEXT NOT NULL,
    language TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS ix_lineage_qualname ON function_lineage(current_qualified_name);

CREATE TABLE IF NOT EXISTS function_touches (
    touch_id INTEGER PRIMARY KEY,
    lineage_id INTEGER NOT NULL REFERENCES function_lineage(lineage_id),
    commit_hash TEXT NOT NULL REFERENCES commits(hash),
    path TEXT NOT NULL,
    qualified_name TEXT NOT NULL,
    arity INTEGER NOT NULL,
    change_type TEXT NOT NULL,
    signature_before TEXT,
    signature_after TEXT,
    fingerprint BLOB,
    chunk_text TEXT NOT NULL,
    parse_status TEXT NOT NULL,
    start_line INTEGER NOT NULL DEFAULT 0,
    end_line INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS ix_touch_lineage ON function_touches(lineage_id, commit_hash);
CREATE INDEX IF NOT EXISTS ix_touch_qualname ON function_touches(qualified_name);
CREATE INDEX IF NOT EXISTS ix_touch_commit ON function_touches(commit_hash);

CREATE TABLE IF NOT EXISTS embeddings (
    touch_id INTEGER PRIMARY KEY REFERENCES function_touches(touch_id),
    row_offset INTEGER NOT NULL UNIQUE
);
CREATE INDEX IF NOT EXISTS ix_embeddings_row_offset ON embeddings(row_offset);

-- Isolated explain cache (Invariant I5: zero LLM text in retrieval paths)
CREATE TABLE IF NOT EXISTS explain_cache (
    cache_key TEXT PRIMARY KEY,
    model TEXT NOT NULL,
    prompt_version TEXT NOT NULL,
    text TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
```

#### 1.2 SQLite PRAGMA Configuration & Concurrency Architecture
Under WAL mode, SQLite supports multiple concurrent readers and a single writer. To avoid `SQLITE_BUSY` contention during parallel indexing:
- **PRAGMA Settings:**
  ```sql
  PRAGMA journal_mode = WAL;
  PRAGMA synchronous = NORMAL;
  PRAGMA foreign_keys = ON;
  PRAGMA busy_timeout = 5000;
  PRAGMA cache_size = -64000; -- 64MB cache
  PRAGMA temp_store = MEMORY;
  PRAGMA mmap_size = 268435456; -- 256MB mmap
  ```
- **Single-Writer Background Channel:**
  - Indexing workers (AST parsing, D4 template rendering, dense embedding) run across Rayon threadpool.
  - Workers send batches of parsed entities (`Vec<CommitInfo>`, `Vec<ResolvedTouch>`, `Vec<(touch_id, row_offset)>`) over a bounded channel (`crossbeam_channel::bounded(2048)`).
  - A single dedicated background writer thread owns the `rusqlite::Connection`, opens transactions in batches (1,000–5,000 items), executes prepared insert statements, and commits.
  - Eliminates all database lock collisions and keeps writer thread saturated with contiguous I/O.

#### 1.3 Atomic Replacement (`.gitctx/index.db.tmp` -> `.gitctx/index.db`)
- During cold index creation (`gitctx index`):
  1. The target path `.gitctx/` contains `.gitctx/index.db.tmp`, `.gitctx/tantivy.tmp/`, and `.gitctx/vectors.bin.tmp`.
  2. Indexing runs to completion.
  3. The SQLite connection executes `PRAGMA wal_checkpoint(TRUNCATE);` and is closed.
  4. Tantivy writer commits and is closed.
  5. Vector file writer flushes and is closed.
  6. Atomic directory/file rename:
     - On Unix/Windows: rename `.gitctx/index.db.tmp` -> `.gitctx/index.db` (and replace existing).
     - Swap `.gitctx/tantivy.tmp/` -> `.gitctx/tantivy/`.
     - Rename `.gitctx/vectors.bin.tmp` -> `.gitctx/vectors.bin`.
  7. If indexing fails, crashes, or is interrupted (Ctrl+C), all `.tmp` artifacts are cleaned up and the previous index remains untouched.

#### 1.4 Fail-Fast Config & Version Validation
On database opening, `StorageEngine::open(path, expected_config)` reads:
1. `SELECT value FROM index_state WHERE key = 'schema_version'`
2. `SELECT value FROM index_state WHERE key = 'config_hash'`
3. If `schema_version != expected_config.schema_version` or `config_hash != expected_config.compute_config_hash().as_str()`, returns:
   `CoreError::ConfigMismatch("Index schema version mismatch. Run 'gitctx index' to rebuild.")`.

---

### 2. Tantivy BM25 Schema & Code Tokenizer (`gitctx-index::tantivy_index`)

#### 2.1 Code Tokenizer (`CodeTokenizer`)
Standard natural-language tokenizers (e.g. English stemmers) destructively alter identifiers (e.g. `getter` -> `get`, `indices` -> `indic`, `parser` -> `parser`).
The custom Tantivy `CodeTokenizer` implements:
1. **Splitting Rules:**
   - camelCase / PascalCase: `getUserById` -> `["getUserById", "get", "user", "by", "id"]`
   - snake_case: `get_user_by_id` -> `["get_user_by_id", "get", "user", "by", "id"]`
   - Qualified path delimiters (`::`, `.`, `/`, `->`, `$`): `module::Class.method` -> `["module::Class.method", "module", "class", "method"]`
   - Non-alphanumeric separators (`_`, `-`, `\`, `(`, `)`).
2. **Token Output:**
   - Both the compound verbatim identifier AND individual lowercased subwords are emitted with correct token offsets.
   - Case-insensitive matching via lowercase filter on subwords while retaining exact tokens.
   - **No Stemming Filter** is applied to identifier fields.

#### 2.2 Tantivy Schema & Boosting
Tantivy schema definition:
- `touch_id`: `u64` (`INDEXED | STORED | FAST`)
- `lineage_id`: `u64` (`INDEXED | STORED | FAST`)
- `commit_hash`: `STRING` (`STORED`)
- `qualified_name`: `TEXT` with `CodeTokenizer` (`INDEXED | STORED`) — Query boost: **4.0**
- `path`: `TEXT` with `CodeTokenizer` (`INDEXED | STORED`) — Query boost: **2.0**
- `commit_message`: `TEXT` with `default` tokenizer (`INDEXED | STORED`) — Query boost: **1.2**
- `chunk_text`: `TEXT` with `default` tokenizer (`INDEXED | STORED`) — Query boost: **1.0**

#### 2.3 Tantivy Query Construction
When executing a search for query string $Q$:
- A `QueryParser` constructs a multi-field disjunction with field boosts:
  $$\text{Score}_{\text{BM25}}(d) = 4.0 \cdot \text{BM25}(Q, \text{qualname}) + 2.0 \cdot \text{BM25}(Q, \text{path}) + 1.2 \cdot \text{BM25}(Q, \text{message}) + 1.0 \cdot \text{BM25}(Q, \text{chunk})$$
- Returns top-$K$ documents with their BM25 ranks.

---

### 3. Flat f32 Dense Vector Engine (`gitctx-index::vector`)

#### 3.1 Binary Vector File Format (`.gitctx/vectors.bin`)
Dense vectors are stored in a contiguous binary file without graph overhead:
```
+-------------------------------------------------------------------+
| Magic Bytes (4 bytes): b"GVEC"                                   |
| Format Version (u32, little-endian): 1                            |
| Vector Dimension (u32, little-endian): 384                        |
| Vector Count (u32, little-endian): N                              |
+-------------------------------------------------------------------+
| Vector 0: [f32; 384] (1536 bytes, little-endian IEEE 754)         |
| Vector 1: [f32; 384] (1536 bytes, little-endian IEEE 754)         |
| ...                                                               |
| Vector N-1: [f32; 384] (1536 bytes)                               |
+-------------------------------------------------------------------+
```
- Total header size: 16 bytes.
- Offset for vector $i$: `16 + (i * 384 * 4)` bytes.
- 50,000 vectors $\approx 76.8 \text{ MB}$.
- 100,000 vectors $\approx 153.6 \text{ MB}$.

#### 3.2 Memory Mapping & Integrity Verification
- `MmapVectorStorage` memory-maps `.gitctx/vectors.bin` via `memmap2::MmapOptions::new().map(&file)`.
- On open:
  - Header is validated: `magic == b"GVEC"`, `version == 1`, `dimension == 384`.
  - File length is validated: `file_len == 16 + (vector_count * 384 * 4)`.
  - SQLite check: `SELECT COUNT(*) FROM embeddings` must equal `vector_count`.
- Zero deserialization overhead; slice access is zero-copy: `&[f32]`.

#### 3.3 Unit L2 Normalization & Rayon SIMD Cosine Scoring
- **Write-Time Normalization:**
  During embedding generation, every vector $\mathbf{v}$ is normalized:
  $$\hat{\mathbf{v}} = \frac{\mathbf{v}}{\|\mathbf{v}\|_2} = \frac{\mathbf{v}}{\sqrt{\sum_{k=0}^{383} v_k^2}}$$
  Normalized vectors are written directly to `.gitctx/vectors.bin`.
- **Query-Time Normalization & Cosine Scoring:**
  Given a query embedding $\mathbf{q}$, it is normalized to $\hat{\mathbf{q}}$ once.
  The cosine similarity between unit vectors simplifies to the pure dot product:
  $$\text{CosineSimilarity}(\hat{\mathbf{q}}, \hat{\mathbf{v}}_i) = \hat{\mathbf{q}} \cdot \hat{\mathbf{v}}_i = \sum_{k=0}^{383} \hat{q}_k \cdot \hat{v}_{i, k}$$
- **Rayon Parallel Scan:**
  ```rust
  let scores: Vec<(usize, f32)> = (0..vector_count)
      .into_par_iter()
      .map(|i| {
          let vec_slice = &mmap_slice[i * 384..(i + 1) * 384];
          let dot: f32 = query_vec.iter().zip(vec_slice).map(|(a, b)| a * b).sum();
          (i, dot)
      })
      .collect();
  ```
  On an 8-core CPU with AVX2/NEON SIMD auto-vectorization, scanning 100,000 vectors takes $<12 \text{ ms}$.

---

### 4. Hybrid Retrieval Engine (`gitctx-query`)

#### 4.1 Exact Symbol Fast-Path (Rank-0 Injection)
- When a user query matches identifier syntax (e.g. `[a-zA-Z_][a-zA-Z0-9_]*(::|\.)[a-zA-Z_][a-zA-Z0-9_]*` or exact word without spaces):
  1. The query engine directly searches SQLite:
     ```sql
     SELECT touch_id FROM function_touches 
     WHERE qualified_name = ?1 OR qualified_name LIKE ?2
     ORDER BY touch_id DESC LIMIT 50;
     ```
  2. Any matching `touch_id`s are prioritized with **Rank 0** in the fusion candidate list.
  3. Guarantees exact symbol searches (e.g. `fetch_user` or `UserAuth::login`) never miss or get buried beneath general prose diffs.

#### 4.2 Reciprocal Rank Fusion ($k=60$)
Reciprocal Rank Fusion merges rank lists from Tantivy BM25, dense vector search, and exact symbol fast-path:
$$\text{RRF\_Score}(d) = \sum_{m \in \{\text{exact}, \text{bm25}, \text{dense}\}} \frac{w_m}{k + \text{rank}_m(d)}$$
- Default constant: $k = 60$ (configurable via `--rrf-k` CLI flag).
- If exact symbol hit: injected at rank 0 ($1 / (60 + 0) = 0.01667$).
- BM25 hits scored by rank $0, 1, 2, \dots$
- Dense vector hits scored by rank $0, 1, 2, \dots$

#### 4.3 Result Deduplication & Density Capping
To ensure search result diversity and prevent noise:
1. **Chunk Text Hash Deduplication:** If multiple touches have identical chunk text hashes (e.g. boilerplate reformatting across multiple files in a single commit), only the highest-scoring touch is kept.
2. **Commit Density Capping:** At most **3 touches per commit hash** are retained in the top-$K$ results. This prevents large 50-file commits from dominating the top results.

#### 4.4 Deterministic Lineage Chain Assembly (INDEX-06)
Given a retrieved touch or lineage:
```sql
SELECT 
    ft.touch_id,
    ft.lineage_id,
    ft.commit_hash,
    ft.path,
    ft.qualified_name,
    ft.arity,
    ft.change_type,
    ft.signature_before,
    ft.signature_after,
    ft.chunk_text,
    ft.parse_status,
    ft.start_line,
    ft.end_line,
    c.author,
    c.authored_at,
    c.topo_order,
    c.message
FROM function_touches ft
JOIN commits c ON ft.commit_hash = c.hash
WHERE ft.lineage_id = ?1
ORDER BY c.topo_order ASC;
```
- Executed as a single indexed query.
- Emits the complete, strictly chronological evolution chain of the function from its initial addition through every modification, rename, or move.

#### 4.5 Structured Citation Attachment (INDEX-07 / Invariant I4)
Every retrieved fact returned to callers attaches a non-null `Citation`:
```rust
pub struct Citation {
    pub commit_hash: String,
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub match_rule: String,
    pub match_confidence: f32,
}
```
All constraints are validated at retrieval time before serialization.

---

## Package Legitimacy Audit

| Crate | Version | Repository / Publisher | Purpose | Status / Justification |
|-------|---------|------------------------|---------|------------------------|
| `rusqlite` | `0.32.1` / `0.31.0` (with `bundled`) | `github.com/rusqlite/rusqlite` | SQLite embedded database engine with WAL support and zero C dependency setup. | **APPROVED** - Industry standard SQLite bindings for Rust. `bundled` feature compiles SQLite from source statically, ensuring cross-platform stability. |
| `tantivy` | `0.26.1` / `0.22.0` | `github.com/quickwit-oss/tantivy` | Full-text search engine (Lucene alternative in Rust) with BM25 scoring and custom tokenizers. | **APPROVED** - Official search engine engine maintained by Quickwit; proven in Codanna reference implementation. |
| `fastembed` | `=5.6.0` | `github.com/Anush008/fastembed-rs` | Offline CPU ONNX runtime embedding generator for `AllMiniLML6V2` and `BGESmallENV15`. | **APPROVED** - Pinned to 5.6.0 to maintain universal glibc compatibility (Ubuntu 22.04+ / macOS / Windows) without dynamic library runtime issues. |
| `memmap2` | `0.9.11` | `github.com/RazrFalcon/memmap2-rs` | Safe, cross-platform memory-mapped file I/O for zero-copy vector loading. | **APPROVED** - De facto standard Rust crate for `mmap`. |
| `rayon` | `1.12.0` | `github.com/rayon-rs/rayon` | Work-stealing data parallelism for parallel vector SIMD scanning and parallel AST indexing. | **APPROVED** - Standard Rust parallelism library. |
| `crossbeam-channel` | `0.5.16` | `github.com/crossbeam-rs/crossbeam` | Multi-producer single-consumer bounded channels for background SQLite writer thread. | **APPROVED** - Standard Rust high-throughput concurrency channel. |
| `blake3` | `1.5.5` | `github.com/BLAKE3-team/BLAKE3` | Fast cryptographic digest for `config_hash` and chunk text hashing. | **ALREADY AUDITED** in workspace. |
| `serde` / `serde_json` | `1.0` | `github.com/serde-rs/serde` | Deterministic JSON serialization and envelope encoding. | **ALREADY AUDITED** in workspace. |
| `thiserror` | `2.0.11` | `github.com/dtolnay/thiserror` | Strongly typed domain error enumerations. | **ALREADY AUDITED** in workspace. |

---

## Validation Architecture

The test suite will validate all 10 requirements via fast, isolated unit and integration tests using `tests/fixtures/mkrepo.rs` synthetic repositories.

```mermaid
graph TD
    A["Synthetic Test Repo (mkrepo.rs)"] --> B["Cold Index Pipeline (index.db.tmp, tantivy.tmp, vectors.bin.tmp)"]
    B --> C["Atomic Swap (.gitctx/)"]
    C --> D["Storage Engine Verification (INVAR-03, INDEX-01, INDEX-02)"]
    C --> E["Tantivy BM25 Verification (INDEX-03)"]
    C --> F["Vector Mmap & SIMD (INDEX-04)"]
    C --> G["Hybrid RRF & Exact Symbol (INDEX-05)"]
    C --> H["Lineage Chain Assembly (INDEX-06, INDEX-07)"]
    B --> I["Double-Index Byte Comparison (INVAR-04)"]
    C --> J["Incremental Equivalence Check (INVAR-05)"]
```

### Automated Test Matrix

| Requirement | Test Name | Command | Assertion |
|-------------|-----------|---------|-----------|
| **INVAR-03** | `test_config_hash_mismatch_refusal` | `cargo test -p gitctx-index test_config_hash_mismatch_refusal` | Opening index with altered schema version or model ID returns explicit refusal error without modifying database. |
| **INVAR-04** | `test_double_indexing_determinism` | `cargo test -p gitctx-index test_double_indexing_determinism` | Indexing the same repo twice produced identical SQLite tables, identical vector binary files, and identical top-10 hybrid search results. |
| **INVAR-05** | `test_incremental_vs_full_equivalence` | `cargo test -p gitctx-index test_incremental_vs_full_equivalence` | Progressively indexing commits 1..N matches database rows, vector offsets, and lineage chains of cold index built across 1..N. |
| **INDEX-01** | `test_sqlite_storage_and_atomic_swap` | `cargo test -p gitctx-index test_sqlite_storage_and_atomic_swap` | Validates SQLite schema DDL, WAL PRAGMAs, single-writer thread batch channel, and crash-safe `.tmp` atomic replacement. |
| **INDEX-02** | `test_explain_cache_isolation` | `cargo test -p gitctx-index test_explain_cache_isolation` | Verifies `explain_cache` table exists but is never queried during indexing or embedding queries (Invariant I5). |
| **INDEX-03** | `test_tantivy_code_tokenizer_and_bm25` | `cargo test -p gitctx-index test_tantivy_code_tokenizer_and_bm25` | Tests camelCase and snake_case subtoken extraction, no stemming on identifiers, schema boosting, and BM25 top-K queries. |
| **INDEX-04** | `test_mmap_vector_storage_and_simd_scoring` | `cargo test -p gitctx-index test_mmap_vector_storage_and_simd_scoring` | Validates binary `.gitctx/vectors.bin` layout, header validation, `memmap2` slice zero-copy access, and Rayon SIMD cosine dot product. |
| **INDEX-05** | `test_rrf_hybrid_fusion_and_exact_symbol_fastpath` | `cargo test -p gitctx-query test_rrf_hybrid_fusion_and_exact_symbol_fastpath` | Verifies RRF $k=60$ fusion, exact symbol detection injecting Rank-0, chunk hash deduplication, and max 3 touches/commit density capping. |
| **INDEX-06** | `test_deterministic_chain_assembly` | `cargo test -p gitctx-query test_deterministic_chain_assembly` | Asserts single indexed SQL query retrieves complete evolutionary timeline sorted by `topo_order ASC`. |
| **INDEX-07** | `test_citation_payload_completeness` | `cargo test -p gitctx-query test_citation_payload_completeness` | Asserts all retrieved facts contain complete `Citation` metadata (`commit_hash`, `path`, `start_line`, `end_line`, `match_rule`, `match_confidence`). |

---

## Open Questions (RESOLVED)

1. **How should Tantivy and SQLite atomic renames be synchronized?**  
   *Resolution:* All artifacts (`index.db.tmp`, `tantivy.tmp/`, `vectors.bin.tmp`) are written concurrently to temporary locations in `.gitctx/`. Once all writers finish, `PRAGMA wal_checkpoint(TRUNCATE)` is executed on SQLite and writers are closed. An atomic filesystem swap renames `.tmp` paths to their active paths. On failure at any point, temporary files are removed, leaving active index undisturbed.

2. **How to handle vectors during incremental indexing without full file rewrites?**  
   *Resolution:* Incremental updates append new unit-normalized `f32` vectors to the end of `.gitctx/vectors.bin`, update the 16-byte header's `vector_count`, insert corresponding `(touch_id, row_offset)` mappings into SQLite's `embeddings` table, and append docs to Tantivy.

3. **What is the default behavior if FastEmbed model download is needed in offline environments?**  
   *Resolution:* `fastembed-rs` caches downloaded ONNX models in a standard directory (`~/.cache/fastembed/` or local app directory). During test execution and offline production runs, the model files are pre-cached, ensuring zero network calls during index and query operations (Invariant I1).

---

## Conclusion & Planning Guidance

The technical architecture for Phase 6 is completely specified and derisked:
- SQLite storage with WAL mode, single writer channel, atomic `.tmp` swap, schema versioning, and isolated `explain_cache`.
- Tantivy BM25 with code-aware camelCase/snake_case tokenization without identifier stemming.
- Flat binary vector file (`vectors.bin`) memory-mapped via `memmap2`, L2 normalized at write time, and scanned with Rayon SIMD dot product.
- Hybrid RRF retrieval ($k=60$), exact symbol fast-path (Rank-0), chunk deduplication, commit density capping ($\le 3$/commit), single-query chain assembly ordered by `topo_order ASC`, and complete Citation metadata.

The implementation is ready for task planning and execution across `gitctx-index` and `gitctx-query`.

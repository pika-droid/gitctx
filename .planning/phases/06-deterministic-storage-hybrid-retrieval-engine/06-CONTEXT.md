# Phase 6: Deterministic Storage & Hybrid Retrieval Engine - Context

**Gathered:** 2026-08-18
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 6 implements the unified persistence and hybrid retrieval engine (`gitctx-index` and `gitctx-query`). It delivers:
- SQLite storage engine with WAL mode, single dedicated background writer channel, transactional batching (1,000–5,000 rows), and two-phase atomic replacement (`.gitctx/index.db.tmp` -> `.gitctx/index.db`).
- Strict schema versioning and `config_hash` validation (INVAR-03) on open, with explicit refusal diagnostics.
- Architectural physical isolation of `explain_cache` table (INVAR-05 / INDEX-02).
- Code-aware Tantivy BM25 indexing pipeline with camelCase/snake_case subtoken preservation, no identifier stemming, boosted schema (`qualified_name` 4.0, `path` 2.0, `commit_message` 1.2, `chunk_text` 1.0), and dedicated `.gitctx/tantivy/` directory with atomic `.tmp` swap.
- Dense embedding generator with `fastembed-rs` (384-dim BGESmallENV15 / AllMiniLML6V2, 100% offline CPU), flat contiguous `f32` binary vector storage (`.gitctx/vectors.bin`), memory-mapped via `memmap2`, mapped to SQLite `embeddings(touch_id, row_offset)`, with write-time L2 normalization and Rayon-parallelized SIMD dot-product cosine scoring.
- Exact symbol fast-path (syntax heuristic + SQLite index lookup on `qualified_name` injected with top priority / rank-0).
- Reciprocal Rank Fusion (RRF $k=60$) combining BM25 and dense scores with result deduplication by chunk text hash and max 3 touches per commit density capping.
- Deterministic lineage chain assembly (INDEX-06) querying full chronological history in a single indexed query ordered by `topo_order ASC`.
- Structured citation metadata payload (INDEX-07 / Invariant I4) attached to all retrieved facts (`commit_hash`, `path`, `start_line`, `end_line`, `match_rule`, `match_confidence`).

</domain>

<decisions>
## Implementation Decisions

### SQLite Storage & Concurrency Architecture
- **D-01:** Dedicated background writer thread with bounded channel — Parallel indexing workers extract ASTs, render D4 templates, and generate embeddings, sending batch chunks (1,000–5,000 rows) over a crossbeam/mpsc channel to a single dedicated SQLite writer. Eliminates `SQLITE_BUSY` lock contention under WAL mode (`PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;`). — **Reversibility:** costly — defines concurrency pipeline in `gitctx-index`.
- **D-02:** Two-phase atomic rename (`.gitctx/index.db.tmp` -> `.gitctx/index.db`) — Cold indexing builds to temporary files (`index.db.tmp`, `tantivy.tmp/`, `vectors.bin.tmp`), checkpoints WAL, and atomically renames all artifacts upon completion. Interrupted or crashed indexing runs never leave corrupted or partial index state. — **Reversibility:** one-way — core invariant INDEX-01 / INVAR-04 requirement.
- **D-03:** Fail-fast refusal with explicit diagnostic — Validate `schema_version` (integer) and `config_hash` (BLAKE3/SHA256 of schema DDL + tokenizer + model config) in `index_state` on open; if mismatched, return an explicit actionable error: `"Index schema version mismatch. Run 'gitctx index' to rebuild."` — **Reversibility:** reversible.
- **D-04:** Dedicated isolated `explain_cache` table with architectural read-gating — Stored in main SQLite DB (`cache_key`, `model`, `prompt_version`, `text`, `created_at`) but strictly excluded from all indexer and embedding SELECT queries to enforce Invariant I5 (zero LLM-generated text in index/retrieval paths). — **Reversibility:** one-way — Invariant I5 enforcement contract.

### Tantivy BM25 Schema & Code Tokenizer
- **D-05:** Hybrid Code + Text Tokenizer — Custom Tantivy tokenizer that splits on camelCase, snake_case, and non-alphanumeric boundaries, emitting both compound tokens (`getUserById`) and individual subtokens (`get`, `user`, `by`, `id`), lowercase + case-preserved, with no stemming on code identifiers to prevent destructive stem transformations (e.g., `getter` -> `get`). — **Reversibility:** costly — defines lexical search behavior and Tantivy schema.
- **D-06:** Boosted multi-field schema — `qualified_name` (boost 4.0), `path` (boost 2.0), `commit_message` (boost 1.2), `chunk_text` (boost 1.0). Gives strong priority to exact symbol and filename matches while retaining broad recall across pseudo-doc explanation texts. — **Reversibility:** costly — Tantivy scoring schema.
- **D-07:** Dedicated `.gitctx/tantivy/` directory with atomic `.tmp` swap — Tantivy directory written to `.gitctx/tantivy.tmp/` during cold indexing and atomically swapped alongside `index.db.tmp` -> `index.db` to maintain cross-engine atomicity. — **Reversibility:** costly — filesystem index layout.
- **D-08:** Incremental updates via Tantivy `IndexWriter` append — Incremental indexing instantiates `IndexWriter`, appends new documents keyed by `touch_id`, and commits in <2 seconds, letting Tantivy handle segment merges. — **Reversibility:** reversible.

### Dense Vector Storage & SIMD Scoring
- **D-09:** `fastembed-rs` with `BGESmallENV15` / `AllMiniLML6V2` (384 dimensions) — Runs 100% offline via ONNX Runtime CPU, fast batch inference (~1000 chunks/s on CPU), compact weights (~130MB), cached locally. — **Reversibility:** costly — embedding dimensions and model selection.
- **D-10:** Flat contiguous binary vector file (`.gitctx/vectors.bin`) with `memmap2` and SQLite `embeddings(touch_id, row_offset)` mapping — Raw `&[f32]` array with zero-copy deserialization; 50k vectors ≈ 75MB. Validates file byte length against SQLite row count on open. — **Reversibility:** one-way — vector storage format.
- **D-11:** Write-time L2 normalization + Rayon-parallel SIMD dot product — Vectors are unit-normalized upon embedding generation ($L_2 = 1.0$). At query time, query vector is unit-normalized and scanned against all vectors using SIMD dot product partitioned with `rayon`, achieving <15ms brute-force scans on 100k vectors with zero ANN graph build overhead. — **Reversibility:** costly — vector scoring engine.
- **D-12:** Batch embedding (128–256 chunks), embed `ok` and `degraded`, skip `failed` — Maximizes CPU/SIMD throughput during indexing; ensures degraded historical diff chunks remain searchable while skipping empty/failed AST parses. — **Reversibility:** reversible.

### Hybrid RRF Fusion & Exact Symbol Fast-Path
- **D-13:** Syntax heuristic + SQLite exact index lookup with rank-0 injection — Detects identifier syntax (`[a-zA-Z_][a-zA-Z0-9_]*(::|\.)[a-zA-Z_][a-zA-Z0-9_]*` or exact word) and queries SQLite `function_touches` index on `qualified_name` directly. Exact hits are injected with top RRF rank (rank 0 / boost) so exact symbol queries never fail. — **Reversibility:** costly — query execution pipeline.
- **D-14:** Standard RRF $k=60$ with CLI override (`--rrf-k`) — Combines BM25 and dense rank lists via $sum rac{1}{k + 	ext{rank}_i}$, default $k=60$ per Cormack et al. and Phase 3 ablation findings. — **Reversibility:** reversible.
- **D-15:** Max 3 touches per commit + chunk-hash deduplication — Deduplicates duplicate boilerplate hunks by chunk text hash and caps search results to at most 3 touches per commit hash to prevent single large commits from dominating top-K results. — **Reversibility:** reversible.
- **D-16:** Single indexed SQL query ordered by `topo_order ASC` + structured Citation payload — Queries SQLite by `lineage_id` joined with `commits` ordered by `topo_order ASC` in a single indexed query, attaching full Citation metadata (`commit_hash`, `path`, `start_line`, `end_line`, `match_rule`, `match_confidence`). — **Reversibility:** one-way — core retrieval output contract (INDEX-06, INDEX-07).

### the agent's Discretion
- Exact Rayon chunk sizes for SIMD vector iteration.
- Internal helper functions and module partitioning across `gitctx-index` and `gitctx-query`.
- SQLite PRAGMA cache size and busy timeout constants.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Architecture & Specification
- `reference_docs/gpai-20260816-gitctx-Complete-Step-by-Step-Implementation-Plan-w.md` — Complete step-by-step implementation plan for gitctx
- `reference_docs/20260816-Implementation-Plan-Review-Suggestions.md` §2 (Architecture), §3 (Data model & SQLite schema), §6 (Retrieval: Tantivy + dense + RRF), §8 (CLI JSON envelopes)
- `.planning/PROJECT.md` — Project definition, core values, active requirements, and constraints
- `.planning/REQUIREMENTS.md` §INDEX (INDEX-01 through INDEX-07), §INVAR (INVAR-03, INVAR-04, INVAR-05)
- `.planning/ROADMAP.md` §Phase 6 — Phase 6 goals, plans, and success criteria

### Reference Implementations
- `reference_repos/codanna/src/storage/` — SQLite connection management, PRAGMAs, WAL mode, transaction batching
- `reference_repos/codanna/src/indexing/` — Tantivy schema, custom tokenizers, multi-field boosting
- `reference_repos/codanna/src/vector/` & `src/semantic/` — FastEmbed embedding, vector mmap, hybrid fusion
- `crates/gitctx-core/src/lineage/` — Stable LineageId, MinHash, and LineageDag definitions
- `crates/gitctx-core/src/template/` — D4 pseudo-document template generator

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/gitctx-core/src/model.rs` — `CommitId`, `LineageId`, `FunctionTouch`, `Citation`, `ParseStatus`.
- `crates/gitctx-core/src/config.rs` — `IndexConfig`, `ConfigHash`.
- `crates/gitctx-core/src/template/` — `D4TemplateRenderer` for generating pseudo-documents.
- `crates/gitctx-core/src/lineage/` — `LineageResolver`, `MinHashFingerprint`, `LineageDag`.
- `crates/gitctx-core/src/git/` — `GitWalker`, `CommitDiff`, `DeltaStatus`.
- `crates/gitctx-core/src/redact/` — `SecretRedactor` for sanitizing indexed chunks.

### Established Patterns
- Strongly typed newtypes with Serde support.
- Zero network, 100% local offline execution (Invariant I1).
- Clean error handling with `thiserror` and `Result<T, GitctxError>`.

### Integration Points
- `crates/gitctx-index/` — SQLite storage engine, writer channel, Tantivy BM25 indexer, FastEmbed dense embedding pipeline, vector mmap writer.
- `crates/gitctx-query/` — Search engine, exact symbol fast-path, RRF rank fusion, deterministic lineage chain assembler.

</code_context>

<specifics>
## Specific Ideas

- SQLite Schema:
```sql
CREATE TABLE commits (
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
CREATE TABLE files (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    language TEXT NOT NULL
);
CREATE TABLE function_lineage (
    lineage_id INTEGER PRIMARY KEY,
    current_path TEXT NOT NULL,
    current_qualified_name TEXT NOT NULL,
    language TEXT NOT NULL
);
CREATE TABLE function_touches (
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
    parse_status TEXT NOT NULL
);
CREATE INDEX ix_touch_lineage ON function_touches(lineage_id, commit_hash);
CREATE INDEX ix_touch_qualname ON function_touches(qualified_name);
CREATE TABLE embeddings (
    touch_id INTEGER PRIMARY KEY REFERENCES function_touches(touch_id),
    row_offset INTEGER NOT NULL
);
CREATE TABLE explain_cache (
    cache_key TEXT PRIMARY KEY,
    model TEXT NOT NULL,
    prompt_version TEXT NOT NULL,
    text TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE TABLE index_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

</specifics>

<deferred>
## Deferred Ideas

- None — discussion stayed strictly within Phase 6 scope.

</deferred>

---

*Phase: 06-Deterministic Storage & Hybrid Retrieval Engine*
*Context gathered: 2026-08-18*

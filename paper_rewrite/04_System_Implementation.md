# Chapter IV: System Implementation and Development Lifecycle

Following the architectural blueprints formulated in Chapter III, this chapter documents the concrete engineering implementation of **gitctx**. We describe the phased development lifecycle, the specific algorithms and data structures chosen, performance optimizations that prevent query-time bottlenecks, and the production-hardening mechanisms enforced across heterogeneous environments.

---

## 4.1 Development Lifecycle and Phased Milestones

To maintain rigorous engineering standards and prevent architectural regressions, the implementation of `gitctx` was organized into twelve structured development milestones. Each phase defined a strict boundary of responsibility, implemented dedicated acceptance tests, and produced verifiable artifacts.

Phase 03 was deliberately reserved as an empirical decision gate for validating the retrieval thesis before building out downstream user-facing interfaces.

| Phase | Milestone Title | Primary Deliverable | Core Function and Verification Checkpoint |
| :--- | :--- | :--- | :--- |
| **01** | **Foundation and Invariant Harness** | Multi-crate workspace & invariant suite | Strict compilation under `#![forbid(unsafe_code)]`; automated verification of offline Invariant I1 via local proxy/namespace blackholing; synthetic Git topologies. |
| **02** | **Corpus and Ground-Truth Harvesting** | Locked corpus manifest & ground truth | Profiling of 15 candidate repositories via `profile.py`; harvesting 260 issue-closing pairs (Set A) and 27 intent queries (Set B); 6-stage leakage sanitization pipeline. |
| **03** | **Retrieval Thesis Decision Gate** | $5 \times 5$ Ablation evaluation report | Experimental execution of 25 representation $\times$ retriever configurations; verification of pre-registered decision margin ($\Delta\text{Recall@10} \ge 0.05$). |
| **04** | **Git Traversal and AST Extraction** | `GitWalker`, Tree-sitter registry | Native `libgit2` first-parent commit stream; in-memory blob LRU caching; 9-language Tree-sitter registry with graceful degradation for syntax anomalies. |
| **05** | **Lineage Engine and Mutation Oracle** | `LineageResolver`, $M_1$–$M_{10}$ test suite | Four-tier bipartite matching cascade; 128-permutation Murmur3 MinHash engine; threshold calibration via 66-point grid sweep. |
| **06** | **Storage and Hybrid Retrieval** | `IndexSink`, Tantivy, ONNX FastEmbed | Multi-modal storage: SQLite WAL database, Tantivy BM25 inverted index, and 384-dimensional dense vectors in `vectors.bin` with atomic two-phase rotation. |
| **07** | **Co-Change Graph and Decay Engine** | `CoChangeGraph`, association metrics | Asymmetric association rule mining (support, confidence, lift) over historical commits; 25-file bulk filter; exponential decay ($h = 180\text{ days}$). |
| **08** | **CLI Interface and Formatting** | `gitctx-cli` executable binary | Interactive human terminal interface featuring colored status tables, unified diff formatting, and shell-pipeable JSON envelopes. |
| **09** | **Model Context Protocol Server** | `gitctx-mcp` stdio JSON-RPC server | Local integration surface exposing five analytical tools to AI coding assistants; stream isolation routing all diagnostics to `stderr`; adaptive token budgeting. |
| **10** | **Benchmarking and Diagnostics** | `cargo xtask bench` performance suite | Latency auditing confirming warm search $< 50\text{ ms}$, cold startup $< 300\text{ ms}$, and memory consumption strictly within defined budgets. |
| **11** | **Comprehensive Evaluation Suite** | Evaluation report & failure taxonomy | Verification of eight core system invariants (I1–I8); formal classification of retrieval misses into a ten-category failure taxonomy. |
| **12** | **Production Readiness & Packaging** | Cross-platform CI matrix & installers | Multi-OS build automation (Linux, macOS, Windows); bundled dynamic library management (ONNX Runtime, libgit2); release packaging. |

---

## 4.2 High-Throughput Indexing Pipeline and Asynchronous Storage

The core orchestration module responsible for ingesting version control history is the `IndexPipeline` (located in `crates/gitctx-index/src/pipeline.rs`), operating in concert with the `IndexSink` deep module (`crates/gitctx-index/src/sink.rs`).

Ingesting large Git repositories requires managing high write throughput while preventing disk I/O bottlenecks from starving AST parsers. The pipeline implements five key engineering decisions:

### 1. Batched Asynchronous Dual-Write Architecture
Rather than writing each function touch individually to disk, `IndexSink` buffers extracted records into batches of up to 128 items. Dense embeddings are generated using a single batched tensor forward pass via `FastEmbedGenerator::generate_embeddings_batched`. Relational metadata and touch rows are dispatched across a non-blocking `crossbeam` channel to a background `BackgroundWriter` thread that commits transactions to SQLite in Write-Ahead Logging (WAL) mode. Concurrently, textual pseudo-documents are indexed into Tantivy within a 32 MB heap budget, and 384-dimensional float vectors are appended directly to the binary storage file (`vectors.bin`).

### 2. Comprehensive Relational Schema (Schema v2)
For each function touch, the SQLite database records complete architectural provenance:
* Target function qualified name (`qualname`) and enclosing module path.
* Parameter arity, return type, and modification category (`change_type`).
* Complete post-change function signature and preceding docstring.
* The 128-element MinHash fingerprint serialized as an exact 512-byte little-endian binary BLOB.
* Repository HEAD state and extraction metadata for incremental validation.

### 3. Chronological First-Parent Ingestion
To maintain correct causal lineage, the commit walker collects the topological sequence of first-parent commits and reverses it, ingesting changes chronologically from the initial root commit forward to the repository HEAD. This guarantees that parent-child relationships and function renames are resolved forward in time.

### 4. Dynamic Identifier Delta Computation
Using `IdentifierExtractor::extract_from_lines`, the pipeline tokenizes lines added and deleted within the unified diff hunks. By filtering out language keywords and punctuation, the engine isolates newly introduced domain identifiers and deprecated identifiers, feeding them directly into the D4 template generator.

### 5. Staged Two-Phase Atomic Index Rotation
To ensure crash consistency—particularly on Windows filesystems where active file locks prevent in-place overwriting—the pipeline writes all index files to a temporary staging directory (`.staged/`). Upon successful indexing completion, `AtomicStorageManager` executes a two-phase rotation: existing database and vector files are moved to `.bak` paths, staged files are promoted into the production paths, and the backup files are purged. If any failure occurs during promotion, the previous valid index state is automatically restored.

---

## 4.3 MinHash Fingerprinting and Scoped LSH Inverted Index

Detecting near-duplicate function bodies across complex edits requires computing token similarity without incurring prohibitive $O(N^2)$ pairwise comparisons.

### 4.3.1 128-Permutation MinHash Fingerprinting
In `crates/gitctx-core/src/lineage/minhash.rs`, function bodies are normalized (stripping whitespace and comments) and partitioned into overlapping 3-shingles (token trigrams). The engine generates 128 permutation hash values using a pure-Rust implementation of Murmur3-32 initialized with fixed seed constants:

$$h_i(s) = \text{Murmur3}(s, \text{seed}_i), \quad i \in [0, 127]$$

The MinHash signature of function $A$ is the vector of minimum hash values observed across all constituent shingles:

$$H(A) = [\min_{s \in A} h_0(s), \min_{s \in A} h_1(s), \dots, \min_{s \in A} h_{127}(s)]$$

The Jaccard similarity between two functions $A$ and $B$ is estimated directly by counting the fraction of matching hash slots:

$$\hat{J}(A, B) = \frac{1}{128} \sum_{i=0}^{127} \mathbb{I}[H(A)_i = H(B)_i]$$

Any function containing fewer than 10 tokens is flagged (`is_short = true`) and excluded from MinHash calculations, preventing false-positive matches on trivial getters, setters, or standard boilerplate.

### 4.3.2 LSH Band Inverted Index
While same-file matching (Tier 2) evaluates a small set of candidate functions, cross-file tracking (Tier 4) must search across thousands of active symbols. To accelerate this search, `crates/gitctx-core/src/lineage/lsh.rs` implements a Locality-Sensitive Hashing (LSH) inverted index.

The 128-element MinHash signature is partitioned into $b = 16$ bands, each containing $r = 8$ rows ($16 \times 8 = 128$). Each 8-integer band vector is hashed into a 64-bit integer using FNV-1a. The `LshIndex` maintains an inverted posting list mapping each 64-bit band hash to the set of function identifiers exhibiting that hash.

```
MinHash Signature (128 Permutations)
[ h_0, h_1, ... h_7 | h_8, ... h_15 | ... | h_120, ... h_127 ]
         |                   |                        |
      Band 1              Band 2                   Band 16
   (8 elements)        (8 elements)             (8 elements)
         |                   |                        |
     FNV-1a hash         FNV-1a hash              FNV-1a hash
         v                   v                        v
    [Bucket ID]         [Bucket ID]              [Bucket ID]
         |                   |                        |
         +-------------------+------------------------+
                             |
                             v
               Inverted Posting Candidate Set
             (Evaluated against tau_cross = 0.65)
```

At query time, the engine computes the 16 band hashes of the target function and retrieves all candidate functions sharing at least one common bucket. Only these retrieved candidates undergo full 128-slot Jaccard verification. This reduces candidate comparisons by several orders of magnitude, making cross-file function tracking computationally feasible within tight indexing windows.

---

## 4.4 Resolving Function Identity in Practice

The `LineageResolver::resolve_commit` method tracks code entities across successive revisions using a deterministic bipartite matching cascade:

1. **Tier 1 (Exact Match):** Matches functions exhibiting identical qualified names, file paths, and AST signatures.
2. **Tier 2 (Same-File MinHash):** When Tier 1 fails, evaluates functions remaining within the same file path against $\tau_{\text{same}} = 0.55$.
3. **Tier 3 (Git Rename Map):** Cross-references Git's internal tree diff rename events, tracking functions across file relocations when the qualified identifier is preserved.
4. **Tier 4 (Cross-File LSH MinHash):** Locates relocated and extracted functions across different file paths using the LSH inverted index, requiring estimated Jaccard similarity $\ge \tau_{\text{cross}} = 0.65$ and enforcing a decoy rejection threshold ($\tau_{\text{decoy}} = 0.20$).

Matches are resolved greedily by descending confidence score. Unmatched previous symbols are marked as deleted in the lineage graph, while unmatched new symbols receive freshly minted `LineageId` integers.

---

## 4.5 Reciprocal Rank Fusion Implementation and Determinism

In `crates/gitctx-query/src/rrf.rs`, lexical and semantic result streams are combined using Reciprocal Rank Fusion:

```rust
// Exact mathematical implementation from rrf.rs
let mut fused_scores: HashMap<TouchId, f64> = HashMap::new();

for (rank, hit) in lexical_hits.iter().enumerate() {
    *fused_scores.entry(hit.touch_id).or_default() += 1.0 / (60.0 + rank as f64);
}

for (rank, hit) in dense_hits.iter().enumerate() {
    *fused_scores.entry(hit.touch_id).or_default() += 1.0 / (60.0 + rank as f64);
}
```

### Deterministic Tie-Breaking
To enforce Invariant I2 across all execution environments, tie-breaking is strictly governed by sorting on internal touch identifiers:

```rust
results.sort_by(|a, b| {
    b.score
        .partial_cmp(&a.score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| a.touch_id.cmp(&b.touch_id)) // Invariant I2 tie-breaker
});
```

Because `touch_id` is assigned monotonically in insertion order during first-parent ingestion, sorting ties by ascending `touch_id` guarantees bitwise identical output rankings across all platforms. Furthermore, when a query string exactly matches a known function symbol or commit hash, the record triggers rank-zero injection, ensuring exact lookups dominate semantic approximations.

---

## 4.6 Co-Change Analytics and Precomputed Schema v2

In early development prototypes, computing change impact scores required executing thousands of relational queries at query time ($N+1$ query bottleneck), yielding latencies exceeding 2 seconds per request.

To eliminate this overhead, `gitctx` redesigned its co-change engine to precompute coupling matrices during repository indexing:
1. **Schema v2 Analytical Tables:** The SQLite database introduces `cochange_file_metrics` and `cochange_function_metrics` with composite primary keys (`(entity_a, entity_b)`), backed by B-tree indexes for sub-millisecond retrieval.
2. **Bulk Ingestion Computation:** Upon completing commit ingestion, `compute_and_persist_cochange()` executes a single pass over the historical touch stream, grouping changes by commit, pruning bulk commits ($> 25$ files), and calculating Support, Confidence, Lift, and time-decay weighted support ($h = 180\text{ days}$).
3. **Sub-Millisecond Impact Scoring:** At query time, `ImpactService` (`crates/gitctx-query/src/impact.rs`) queries the precomputed tables directly, returning blast-radius risk rankings in under $1\text{ ms}$.

---

## 4.7 Model Context Protocol (MCP) Server Architecture

The `gitctx-mcp` executable provides the integration surface for autonomous AI coding agents over standard I/O using newline-delimited JSON-RPC 2.0.

### 1. Warm In-Memory Embedder Caching
Re-instantiating the ONNX neural runtime for each individual tool call incurs an initialization overhead of 200–500 ms. To prevent this latency penalty, `gitctx-mcp` wraps the embedding engine in a shared `Arc<Mutex<FastEmbedGenerator>>`. The ONNX session is warmed during server startup and retained across requests, reducing query-time vector generation to sub-5ms latencies.

### 2. Strict Stream Isolation
MCP clients rely on pristine standard output streams to parse JSON-RPC envelopes. `gitctx-mcp` enforces strict stream separation: all internal diagnostic traces, warnings, and log messages are redirected exclusively to `stderr`, guaranteeing that `stdout` contains only well-formed JSON-RPC frames.

### 3. Adaptive Token Budgeting
To optimize context window consumption for host AI agents, every retrieval tool supports a configurable `mode` parameter:
* `compact`: Returns structured citation metadata (commit SHA, author, date, line ranges, and concise intent summary), consuming $< 150$ tokens per result.
* `full`: Emits the complete rendered D4 pseudo-document containing raw identifier deltas, signatures, and surrounding docstrings for deep contextual reasoning.

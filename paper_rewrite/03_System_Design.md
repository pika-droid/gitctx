# III.   SYSTEM DESIGN AND METHODOLOGY

This chapter details the architectural design and technical methodology of the **gitctx** engine. To transform raw version control logs into a high-precision, semantic retrieval index, the system couples deterministic static analysis with hybrid information retrieval, resilient function lineage tracking, and statistical co-change modeling. The entire architecture is implemented in idiomatic Rust and structured into modular crates designed to operate strictly offline without external cloud dependencies.

---

### 3.1  Overview and Design Invariants

The `gitctx` engine is organized as a Cargo workspace comprising six modular crates that isolate concerns across the data pipeline:

* **`gitctx-core`:** Core domain models, native Git repository traversal, Tree-sitter abstract syntax tree (AST) extraction, secret sanitization, and the multi-tier function lineage resolver.
* **`gitctx-index`:** High-throughput storage orchestration, managing Tantivy-based BM25 inverted indexes, dense float vector binary storage (`vectors.bin`), and SQLite relational persistence.
* **`gitctx-query`:** Hybrid retrieval engine executing Reciprocal Rank Fusion, exact-match fast paths, and statistical co-change impact analysis.
* **`gitctx-cli`:** Native command-line interface providing interactive query capabilities, lineage inspection, and impact reporting for developers.
* **`gitctx-mcp`:** Model Context Protocol (MCP) server exposing history retrieval tools to autonomous coding agents via standard input/output (`stdio`) JSON-RPC.
* **`gitctx-eval`:** Experimental evaluation harness, benchmarking automation, and statistical report generator enforcing pre-registered decision gates.

```
+--------------------------------------------------------------------------------------------------+
|                                    gitctx System Architecture                                    |
+--------------------------------------------------------------------------------------------------+
|                                                                                                  |
|  [ Git Repository ]                                                                              |
|          |                                                                                       |
|          v                                                                                       |
|  +--------------------+                                                                          |
|  | First-Parent Walker| (Invariant I8: libgit2 first-parent commit stream)                        |
|  +--------------------+                                                                          |
|          |                                                                                       |
|          v                                                                                       |
|  +--------------------+       Diff-Hunk Range Check                                              |
|  | Tree-sitter Parser | ---------------------------------+                                       |
|  +--------------------+                                  |                                       |
|          | (Active AST Function Touches)                 v (Suppresses 63.1% ghost touches)      |
|          +-----------------------+-----------------------+                                       |
|                                  |                                                               |
|        +-------------------------+-------------------------+                                     |
|        |                                                   |                                     |
|        v                                                   v                                     |
|  +---------------------------+               +---------------------------+                       |
|  | 4-Tier Lineage Engine     |               | D4 Template Renderer      |                       |
|  |  Tier 1: Exact Key        |               |  - Qualified Name & File  |                       |
|  |  Tier 2: Same-File MinHash|               |  - Sanitized Commit Log   |                       |
|  |  Tier 3: Rename Map Cross |               |  - Diff Identifiers (+/-) |                       |
|  |  Tier 4: Cross-File LSH   |               |  - AST Post-Change Sig    |                       |
|  +---------------------------+               |  - Contextual Docstring   |                       |
|        |                                     +---------------------------+                       |
|        |                                                   |                                     |
|        v                                                   v                                     |
|  +--------------------+                       +---------------------------+                      |
|  | SQLite Storage     |                       | Dual-Index Storage Tier   |                      |
|  |  - Commits & Touches                       |  - Tantivy BM25 (Lexical) |                      |
|  |  - Lineage DAGs    |                       |  - ONNX BGE-Small (Dense) |                      |
|  |  - Co-Change Graph |                       +---------------------------+                      |
|  +--------------------+                                    |                                     |
|        |                                                   |                                     |
|        +-------------------------+-------------------------+                                     |
|                                  |                                                               |
|                                  v                                                               |
|  +-------------------------------------------------------+                                       |
|  | Hybrid Query Engine & Reciprocal Rank Fusion (k = 60) |                                       |
|  |  - Rank-0 Exact Match Fast Path                       |                                       |
|  |  - Half-Life Exponential Co-Change Decay Scorer       |                                       |
|  +-------------------------------------------------------+                                       |
|                                  |                                                               |
|                 +----------------+----------------+                                              |
|                 v                                 v                                              |
|         [ gitctx-cli ]                     [ gitctx-mcp ]                                        |
|      (Human Terminal UI)             (Local stdio JSON-RPC for LLMs)                             |
|                                                                                                  |
+--------------------------------------------------------------------------------------------------+
```

To eliminate non-deterministic behavior and guarantee research-grade reproducibility, the system enforces eight mechanically audited design invariants:

1. **Invariant I1 (Zero Network Activity):** All indexing, parsing, vector embedding, and query retrieval operations execute strictly offline. Embedding model weights are packaged locally, and CI pipelines verify this isolation in an un-routed network namespace.
2. **Invariant I2 (Deterministic Output):** Ingesting the same repository commit sequence yields identical database states and checksums, enforced by deterministic tie-breaking across all ranking and lineage-matching algorithms.
3. **Invariant I3 (Incremental Equivalence):** Progressively indexing commits one by one produces an identical persistent state to a cold full-index run evaluated over the same terminal commit.
4. **Invariant I4 (Non-Null Citations):** Every record returned by the retrieval engine includes complete, validated provenance—specifically the commit SHA, author identity, timestamp, and target file path.
5. **Invariant I5 (Isolated Explain Cache):** Optional natural-language summarization or LLM annotation layers reside in an isolated SQLite table disabled by default, preventing stochastic outputs from altering core retrieval data.
6. **Invariant I6 (Pinned Evaluation Corpus):** Benchmark repositories are pinned at immutable commit hashes in `eval/corpus.lock.toml` and verified using BLAKE3 and SHA-256 digests before evaluation execution.
7. **Invariant I7 (Stated Baselines and Statistical Uncertainty):** Any empirical metric reported by the evaluation harness must include an explicit baseline delta and a 95% bootstrap confidence interval.
8. **Invariant I8 (First-Parent Traversal Enforcement):** The Git extraction engine strictly follows first-parent commit edges, preventing double-counting of squash-merged and feature-branch diffs.

These invariants are validated automatically via `cargo xtask test-invariants`, which executes continuous integration checks against synthetic and real repository fixtures.

---

### 3.2  Git Extraction Layer and Commit Traversal

The Git extraction layer serves as the primary data ingestion pipeline for the system. Repository access is implemented using `libgit2` bindings provided by the `git2` Rust crate. To eliminate external runtime environment dependencies, the underlying C library is statically bundled and compiled during the build process, preventing discrepancies across different host operating systems.

#### 3.2.1  First-Parent Traversal Strategy

A foundational architectural decision in the extraction layer is the strict enforcement of first-parent traversal (Invariant I8). In modern collaborative workflows, feature branches are routinely integrated into mainline branches via merge commits. Traversing all topological paths in a branched commit graph leads to severe data contamination: modifications made inside a branch would be ingested once during the branch commits and a second time when analyzing the composite merge commit. This duplicate indexing artificially inflates function modification counts and introduces false co-change coupling. Under Invariant I8, `gitctx` enforces a strict first-parent walk, treating merge commits as single integration snapshots reflecting what actually landed on the release branch.

#### 3.2.2  Structural Differencing and Rename Detection

For each commit node, the system computes a structural tree diff against its first parent. Using Git's similarity metrics, file transitions are categorized as additions, modifications, deletions, or renames. While conventional tools stop at file-level diffs, `gitctx` feeds modified file buffers directly into Tree-sitter parsers supporting Go, Rust, and Python.

To eliminate *ghost touches*—instances where a file is modified but specific functions within that file remain untouched—the extractor calculates the exact line ranges of all diff hunks. Only AST function boundaries that directly intersect modified lines are recorded as genuine function touches. On large codebases, this hunk-filtering mechanism prunes over $63\%$ of redundant AST operations.

#### 3.2.3  Graceful Degradation and Fault Tolerance

Historical version control repositories inevitably contain transient syntax errors, incomplete transitions, or unsupported legacy language constructs. To ensure continuous processing without fatal crashes, the AST extraction pipeline implements a graceful degradation mechanism. Each processed touch record includes a `parse_status` flag (`ok`, `degraded`, or `failed`). When Tree-sitter encounters unparseable syntax, the engine logs a degraded status and emits a coarse file-level chunk, maintaining indexing continuity without crashing.

---

### 3.3  Pseudo-Document Template Rendering (D4)

A central methodological contribution of `gitctx` is the construction of structured D4 pseudo-documents. Rather than indexing raw unified diffs directly, the system generates structured natural-language text chunks deterministically from function-level commit touches. This approach bridges the semantic gap between conversational developer queries and rigid diff syntax.

For every function modified in a commit, a dedicated `D4TemplateRenderer` produces a document comprising six canonical sections:
1. **Header Metadata:** Normalizes the function's fully qualified identifier, file path, author name, and ISO timestamp.
2. **Sanitized Commit Message:** The original commit log message, cleaned of issue-tracker boilerplate and verified against regex credential patterns.
3. **Introduced Identifiers:** High-signal programming tokens added to the function body, extracted by splitting diff lines on camelCase and snake_case boundaries while discarding language-specific keywords.
4. **Deleted Identifiers:** Programming tokens removed from the function body, capturing deprecated logic or obsolete dependencies.
5. **Function Signature:** The updated function prototype, parameter list, and return types derived directly from the post-commit AST.
6. **Contextual Docstring:** Inline comments or documentation blocks immediately preceding the function declaration.

The renderer cleanly omits any empty sections to optimize storage. The resulting pseudo-documents closely resemble the explanatory prose an engineer would write to document a code change.

#### 3.3.1  Credential Sanitation

Before rendering the final text, the pipeline applies a `SecretRedactor` pass to all free-text fields. This module utilizes high-entropy string detection and common API key prefix patterns to scrub credentials from commit messages and variable names. Although this operates strictly as a defense-in-depth measure without absolute cryptographic security guarantees, it effectively prevents the accidental exposure of sensitive secrets within plain-text query results.

---

### 3.4  Function Lineage Engine

The function lineage engine represents the core algorithmic contribution of the system. Its primary purpose is to maintain stable and continuous identities for functions across their entire evolutionary lifetime, preserving tracking continuity across function renames, file moves, and signature migrations.

Function identity is represented as a `LineageId`, a monotonically increasing integer that uniquely identifies a logical function across all its historical manifestations. The lineage engine processes commits in strict chronological order, maintaining a table of active symbols indexed by file path. For each new commit, candidate matches are generated and evaluated through a four-tier resolution cascade:

* **Tier 1 (Exact Triplet Match):** Compares the triplet `(normalized_path, qualified_name, signature)`. If identical, the function represents an in-place modification and preserves its `LineageId`.
* **Tier 2 (Same-File MinHash):** When Tier 1 fails but the entity remains within the same file path, the engine tokenizes the function body into overlapping 3-shingles (trigrams) and computes a 128-permutation MinHash signature using Murmur3. If the estimated Jaccard similarity exceeds the calibrated threshold $\tau_{\text{same}} = 0.55$, continuity is established. Trivial boilerplate functions ($< 10$ tokens) are bypassed.
* **Tier 3 (Git Rename Cross-Referencing):** If Git's structural diff detects that a file was renamed and the function's qualified name is preserved, the engine maps the lineage across the file boundary.
* **Tier 4 (Cross-File MinHash with LSH Banding):** When a function is relocated across modules or extracted into a utility file, the engine searches active symbols. To avoid pairwise comparison overhead, candidate pairs are pre-filtered into hash buckets using Locality-Sensitive Hashing (LSH) banding before computing full MinHash similarity against $\tau_{\text{cross}} = 0.65$.

Candidate pairings are evaluated using greedy bipartite matching sorted by confidence score. Any candidate matching below the decoy rejection threshold ($\tau_{\text{decoy}} = 0.20$) is rejected, preventing accidental linkage of unrelated boilerplate functions. Unmatched previous symbols are marked as deleted in the lineage graph, while unmatched new symbols receive freshly minted `LineageId` integers. The complete resolution history is recorded as a directed acyclic graph to enable queries that trace the full ancestry of any given function.

---

### 3.5  Storage Orchestration and Hybrid Retrieval

The storage layer is engineered to maintain high write throughput during repository indexing while enabling low-latency queries during developer workflows. All structured relational data is persisted to a local SQLite database configured in Write-Ahead Logging (WAL) mode, residing directly within the target repository. Database writes are serialized asynchronously via a dedicated worker thread over crossbeam channels to prevent disk contention during AST extraction.

#### 3.5.1  Dual-Engine Document Indexing

The generated D4 pseudo-documents are simultaneously indexed by two complementary retrieval engines:
* **Lexical Retrieval Engine:** Uses Tantivy to construct an incremental BM25 index with a custom code-aware tokenizer that splits strings on camelCase boundaries, underscores, and hyphens to accurately isolate programming identifiers.
* **Semantic Retrieval Engine:** Encodes each pseudo-document into a 384-dimensional dense vector using the `BGE-Small-EN-v1.5` embedding model. Executed locally via ONNX Runtime dynamic bindings, the resulting embedding vectors are stored directly as memory-mapped floating-point arrays in `vectors.bin`. This memory-mapping strategy avoids the massive startup overhead associated with loading neural inference sessions, allowing rapid nearest-neighbor cosine scanning.

#### 3.5.2  Query Execution and Rank Fusion

During query execution, the system retrieves independent document rankings from both the Tantivy BM25 index and the dense vector store. These two result sets are fused into a single unified ranking using Reciprocal Rank Fusion (RRF):

$$RRF(d) = \sum_{r \in \mathcal{R}} \frac{1}{k + \text{rank}(r, d)}$$

where the constant $k = 60$ prevents top-ranked outliers from disproportionately dominating the fused results.

To satisfy the strict deterministic output requirements of Invariant I2, the retrieval engine breaks any RRF score ties systematically by sorting tied results in ascending order of their internal touch identifiers (`touch_id`). Furthermore, when a query exactly matches a known function qualified name or commit hash prefix, the engine retrieves the specific record directly from the SQLite index and injects it at rank zero, ensuring exact lookups mathematically dominate the final output ranking.

---

### 3.6  Co-Change Coupling and Impact Analysis

The co-change analysis engine computes statistical coupling metrics over the indexed commit history to identify implicit architectural dependencies. To prevent mass-formatting passes, automated dependency bumps, and large refactorings from injecting spurious coupling edges, the engine strictly filters out any commit modifying more than 25 files.

For every qualifying commit, the engine calculates three association metrics for each pair of modified entities:
* **Support:** The raw co-occurrence count representing the number of qualifying commits in which both entities were modified together.
* **Confidence:** The conditional probability that one entity changes given that the corresponding entity also changes ($P(B \mid A)$).
* **Lift:** The ratio of the observed joint probability to the expected joint probability assuming statistical independence:

$$\text{Lift}(A, B) = \frac{P(A \land B)}{P(A) \cdot P(B)}$$

Lift values substantially above 1.0 indicate architecturally significant coupling rather than incidental coincidence.

To ensure recent modifications carry greater diagnostic significance than historical changes, the system applies an exponential half-life decay function:

$$w(t) = 2^{-\frac{t_{\text{ref}} - t}{h}}$$

where $t_{\text{ref}}$ represents the reference epoch at index time, $t$ is the commit timestamp, and $h$ defines the half-life in days (defaulting to 180 days). Under this decay model, coupling from 180 days ago contributes half its original weight ($0.50$), while changes from two years ago decay to approximately $6.25\%$, preventing obsolete architectural topologies from distorting current risk assessments.

The system stores these weighted pairs in precomputed SQLite tables (`cochange_file_metrics`, `cochange_function_metrics`). During query operations, an impact scorer traverses this graph to generate ranked risk assessments for any specific file or function.

---

### 3.7  Output Layer and Agent Integration

The final stage of the system pipeline exposes the processed semantic knowledge base to both human developers and autonomous AI agents.

#### 3.7.1  Human-Facing Command Interface

The `gitctx-cli` module provides a native terminal application for manual developer queries. It receives ranked Reciprocal Rank Fusion scores directly from the query engine and formats the retrieved D4 pseudo-documents, structural metadata, and temporal decay metrics into human-readable console tables and diffs. Additionally, it supports shell-pipeable JSON output envelopes, allowing developers to investigate function lineage and assess the impact of proposed changes natively within terminal scripts and IDE tasks.

#### 3.7.2  Model Context Protocol (MCP) Server

To facilitate advanced reasoning over repository history, the system implements a Model Context Protocol server via the `gitctx-mcp` binary. This component acts as a bridge between the local semantic knowledge base and autonomous Large Language Models, communicating over standard input and output using newline-delimited JSON-RPC 2.0.

The server exposes five dedicated analytical tools to AI agents:
1. `search_history`: Executes hybrid search over historical D4 pseudo-documents.
2. `function_evolution`: Retrieves the unbroken chronological transformation history of a specific function across all refactorings.
3. `change_impact`: Computes decay-weighted co-change risk and lists coupled files for a proposed modification.
4. `who_knows_this`: Identifies domain experts by computing historical modification frequency and recency.
5. `commit_detail`: Returns full contextual metadata and AST changes for a specific commit hash.

Each tool supports a configurable `mode` parameter (`compact` vs. `full`) that controls record verbosity, allowing host AI agents to manage context window consumption effectively. The server enforces strict output isolation: all diagnostic tracing is directed exclusively to `stderr`, guaranteeing that `stdout` contains only well-formed JSON-RPC frames. Furthermore, the server maintains a warm in-memory ONNX session to avoid reload overhead while executing each query statelessly against the underlying local index.

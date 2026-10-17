# V.   EVALUATION METHODOLOGY

This chapter establishes the empirical evaluation framework and benchmarking protocols designed to validate **gitctx**. We formulate multi-language repository gating criteria, non-circular ground-truth extraction pipelines, and statistical ablation suites to systematically assess retrieval efficacy, function lineage tracking fidelity, and mechanical invariant compliance.

---

### 5.1  Evaluation Framework and Methodological Scope

The `gitctx` evaluation framework evaluates four orthogonal dimensions of system correctness:
1. **Retrieval Efficacy:** Precision and ranking fidelity when retrieving historical design rationale in response to intent-oriented queries.
2. **Lineage Tracking Fidelity:** Continuous identity tracking across structural refactorings, file moves, and extractions.
3. **Cryptographic Integrity & Non-Circularity:** Verifiable dataset pinning, tamper-proof manifests, and automated prevention of ground-truth leakage into indexed text.
4. **Mechanical Verification:** Automated testing of core system invariants (I1–I8) and operational latency budgets.

All benchmark artifacts—including repository clone manifests, issue-closing pairs, curated why-queries, mutation fixtures, and calibrated thresholds—are deterministically pinned via `eval/corpus.lock.toml` (Invariant I6) and cryptographically sealed with BLAKE3 and SHA-256 digests in `eval/ground_truth/MANIFEST.toml`.

```
+--------------------------------------------------------------------------------------------------+
|                                  Evaluation Methodology Flow                                     |
+--------------------------------------------------------------------------------------------------+
|                                                                                                  |
|  [ 15 Candidate Open-Source Repositories ]                                                       |
|                     |                                                                            |
|                     v (Automated profiling via eval/profile.py)                                   |
|  +-------------------------------------------------------------+                                 |
|  | Multi-Language Gating Criteria                              |                                 |
|  |  - Traceable Issues >= 150                                  |                                 |
|  |  - Merge Fraction < 0.40                                    |                                 |
|  |  - Bot Ratio <= 0.25                                        |                                 |
|  |  - Non-Universal Squash < 0.90                              |                                 |
|  +-------------------------------------------------------------+                                 |
|                     |                                                                            |
|                     v                                                                            |
|  [ Locked Evaluation Triad: Go (gin), Rust (tokio), Python (requests) ]                         |
|                     |                                                                            |
|          +----------+----------+---------------------------------------+                         |
|          |                     |                                       |                         |
|          v                     v                                       v                         |
|  +---------------+   +-------------------+                   +-------------------+               |
|  | Ground Truth  |   | Ground Truth      |                   | Synthetic Lineage |               |
|  | Set A         |   | Set B             |                   | Mutation Oracle   |               |
|  | (260 Issues)  |   | (27 Why-Queries)  |                   | (M1 - M10 Suite)  |               |
|  +---------------+   +-------------------+                   +-------------------+               |
|          |                     |                                       |                         |
|          v                     v (6-Stage Sanitizer: sanitize.py)      v (Grid Sweep: 66 pts)    |
|  +---------------+   +-----------------------------------+   +-------------------+               |
|  | Issue-Trailer |   | 5 x 5 Retrieval Ablation Grid     |   | Calibrated        |               |
|  | Verification  |   | D1-D5 Reps x R1-R5 Retrievers     |   | Thresholds        |               |
|  | (Audit n=30)  |   | Recall@K, MRR, Bootstrap CI       |   | F1 = 1.000        |               |
|  +---------------+   +-----------------------------------+   +-------------------+               |
|                                        |                                                         |
|                                        v                                                         |
|                      +-----------------------------------+                                       |
|                      | Decision Gate: Margin >= 0.05     |                                       |
|                      | Invariant Suite: xtask test-inv   |                                       |
|                      +-----------------------------------+                                       |
|                                                                                                  |
+--------------------------------------------------------------------------------------------------+
```

---

### 5.2  Evaluation Corpus Selection and Gating Criteria

To guarantee cross-language generalization across diverse repository topologies, an evaluation triad spanning Go, Rust, and Python was selected from fifteen candidate open-source repositories using an automated profiler (`eval/profile.py`). The profiler evaluated repositories against four pre-registered gating rules:

1. **Traceable Closed Issue Density ($N_{\text{traceable}} \ge 150$):** Ensures a sufficient volume of high-confidence issue-commit links to support ground-truth extraction.
2. **Merge Commit Fraction ($f_{\text{merge}} < 0.40$):** Excludes repositories with un-curated merge topologies that inject severe noise into co-change matrices.
3. **Bot Commit Fraction ($f_{\text{bot}} \le 0.25$):** Prunes automated dependency bumps (e.g., Dependabot) that distort code churn metrics.
4. **Non-Universal Squash Ratio ($f_{\text{squash}} < 0.90$):** Preserves rich multi-commit evolutionary histories necessary for evaluating function lineage tracking.

#### Selected Evaluation Triad

| Size Tier | Repository | Language | Total Commits | First-Parent Commits | Traceable Issues | Merge % | Bot % | Selection Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Small** | `gin-gonic/gin` | Go | 2,023 | 1,314 | 1,159 | 10.7% | 6.2% | **Selected** |
| **Medium** | `tokio-rs/tokio` | Rust | 4,783 | 4,182 | 4,299 | 3.9% | 0.1% | **Selected** |
| **Large** | `psf/requests` | Python | 6,495 | 2,669 | 333 | 24.8% | 1.0% | **Selected** |

Pinned commit SHAs are locked in `eval/corpus.lock.toml` under Invariant I6. Hard gating disqualified candidates such as `tiangolo/fastapi` ($f_{\text{bot}} = 44.5\%$) and `trpc/trpc` ($f_{\text{bot}} = 30.5\%$) due to excessive automated bot churn, while `sharkdp/fd` narrowly missed the traceability threshold ($N_{\text{traceable}} = 148$). Although `expressjs/express` met the structural criteria ($f_{\text{merge}} = 7.8\%$), it was reserved to maintain a balanced Go, Rust, and Python evaluation triad.

---

### 5.3  Ground Truth Construction and Leakage Sanitation

Valid information retrieval evaluation requires eliminating circular evaluation paths. If evaluation queries contain verbatim identifiers or commit summaries that appear directly in indexed document text, measured accuracy reflects simple string copying rather than semantic retrieval.

`gitctx` constructs and isolates three independent ground-truth datasets (`eval/ground_truth/MANIFEST.toml`):

#### 1. Set A (260 Issue-Closing Pairs)
Extracted by combining regular-expression parsing of Git commit trailers (`Fixes: #N`, `Resolves: #N`) with authoritative GitHub Issue Timeline API events. To ensure quality, links are capped at three commits per issue, and bot commits are excluded. A manual audit of $n = 30$ randomly sampled pairs confirmed $100\%$ precision ($p = 1.000$, $95\%\text{ Wilson score CI: } [0.887, 1.000]$), validating the automated harvester against our $p \ge 0.85$ quality target.

#### 2. Set B (27 Curated Why-Queries)
A complementary benchmark of 27 manually authored natural-language questions (9 per triad repository) designed to evaluate historical rationale recovery across four distinct categories:
* **Architectural Decisions:** High-level design choices and structural reorganizations.
* **Bug Root Causes:** Non-obvious defect resolutions and regression fixes.
* **API Evolution:** Breaking interface changes and deprecation motivations.
* **Performance Motivations:** Algorithmic optimizations and concurrency improvements.

#### 3. Revert Pairs (42 Pairs)
Mined commit-revert pairs capturing self-contained code rollbacks, used to benchmark rollback analysis.

#### Multi-Stage Leakage Sanitization
To prevent evaluation leakage, all commit text passes through a six-stage sanitizer (`eval/sanitize.py`):
1. **Issue Reference Stripping:** Eliminates `#123`, `GH-123`, and issue URL references.
2. **Pull Request URLs:** Prunes GitHub/GitLab pull request links.
3. **Commit Trailer Stripping:** Scrubs `Signed-off-by:`, `Co-authored-by:`, and action trailers.
4. **Squash Suffix Removal:** Removes automated GitHub squash commit suffixes (e.g., `(#456)`).
5. **Merge Boilerplate Filtering:** Strips standard `Merge pull request #... from ...` lines.
6. **Zero Gold-Rationale Assertion:** Automated tests (`test_zero_gold_rationale_leakage`) verify that no indexed pseudo-document contains textual substrings matching the benchmark rationale answers.

---

### 5.4  5 x 5 Retrieval Ablation Protocol and Experimental Design

The retrieval ablation evaluates whether template-rendered pseudo-documents ($D_4$) outperform raw diffs and un-curated commit logs across lexical, dense semantic, and hybrid retrieval strategies.

#### 5.4.1  Evaluation Axes (D1–D5 x R1–R5)

* **Document Representations ($D_1$–$D_5$):**
  * **$D_1$ (Commit Message Only):** Traditional commit log baseline without diff context.
  * **$D_2$ (Raw Diff Only):** Unstructured unified diff patch without commit message.
  * **$D_3$ (Message + Raw Diff):** Standard developer view combining message and patch text.
  * **$D_4$ (Structured Pseudo-Document):** Canonical six-section template combining commit metadata, author intent, function qualified names, signatures, surrounding docstrings, added/removed identifiers, and normalized diffs.
  * **$D_5$ (Ablated Template Without Message):** Structured pseudo-document stripping commit messages to isolate AST structural grounding.
* **Retrieval Strategies ($R_1$–$R_5$):**
  * **$R_1$ (Keyword Matching Baseline):** Exact substring matching simulating traditional `git log -S` code exploration.
  * **$R_2$ (Code-Aware BM25):** Probabilistic BM25 ranking via `rank_bm25` with code-specific tokenization (snake_case and camelCase sub-token splitting).
  * **$R_3$ (Standard Dense Embedding):** Sentence-transformer dense vector search using `all-MiniLM-L6-v2` with cosine similarity ranking.
  * **$R_4$ (Code-Aware Dense Embedding):** Dense semantic representation using `microsoft/unixcoder-base` fine-tuned on programming languages.
  * **$R_5$ (Hybrid Reciprocal Rank Fusion):** Reciprocal Rank Fusion ($k = 60$) combining code-aware BM25 ($R_2$) with dense retrieval ($R_3$), augmented with exact-match fast-path routing on structured representations ($D_4, D_5$).

#### 5.4.2  Metrics, Results, and Pre-Registered Gate

Retrieval effectiveness is measured using Recall@$K$ ($K \in \{5, 10, 20\}$) and Mean Reciprocal Rank (MRR):

$$\text{MRR} = \frac{1}{|Q|} \sum_{i=1}^{|Q|} \frac{1}{\text{rank}_i}$$

where $|Q|$ is the total number of evaluation queries, and $\text{rank}_i$ is the rank of the first relevant result for query $i$.

To formally assess the retrieval hypothesis, the evaluation protocol establishes a pre-registered decision gate:

$$\Delta\text{Recall@10} = \text{Recall@10}(D_4 + R_5) - \text{Recall@10}(D_3 + R_1) \ge 0.05$$

* **Best Template Config:** $D_4+R_5$ (Recall@10 = $1.0000$, MRR = $0.9815$).
* **Best Baseline Config:** $D_3+R_1$ (Recall@10 = $0.9259$, MRR = $0.7782$).
* **Empirical Margin:** $\Delta\text{Recall@10} = 0.0741$, exceeding the pre-registered threshold ($\Delta \ge 0.05$).
* **Statistical Uncertainty (Invariant I7):** 95% paired bootstrap confidence interval of $[0.0000, 0.1852]$ computed over 1,000 resamples using deterministic SplitMix64 (seed 42).

---

### 5.5  Function Lineage Engine Evaluation Methodology

Because standard Git tools lack ground-truth benchmarks for cross-file function tracking (`git log -L` is restricted to single files), the lineage engine is evaluated via a synthetic mutation oracle (`cargo xtask eval bench-lineage`).

#### 5.5.1  Mutation Taxonomy (M1 to M10)

The oracle generates synthetic Git repositories covering ten formal code transformations:
* **$M_1$–$M_3$ (Renames and Moves):** In-place rename ($M_1$), cross-file move ($M_2$), simultaneous move and rename ($M_3$).
* **$M_4$–$M_6$ (Signatures and Bodies):** Parameter/type modifications ($M_4$), formatting/docstrings ($M_5$), local variable renaming ($M_6$).
* **$M_7$–$M_{10}$ (Structural Mutations):** Function extraction (`derived_from`, $M_7$), decoy delete-and-add (adversarial false match, $M_8$), directory reorganization ($M_9$), function duplication (`copied_from`, $M_{10}$).

#### 5.5.2  Threshold Calibration and Benchmark Results

The bipartite matcher optimizes four calibrated thresholds (`eval/lineage_thresholds.json`):
* $\tau_{\text{same}} = 0.55$: Same-file MinHash similarity (Tier 2).
* $\tau_{\text{cross}} = 0.65$: Scoped cross-file MinHash similarity (Tier 4).
* $\tau_{\text{decoy}} = 0.20$: Decoy rejection threshold penalizing false positives.
* $\tau_{\text{split}} = 0.20$: Shared token overlap for extracted functions.

A 66-point grid sweep (`cargo xtask eval tune-thresholds`) achieved an optimal F1 score of **$1.000$**, **$100.0\%$** $M_8$ decoy rejection, and **$100.0\%$** chain recovery at depth $D = 5$.

#### 5.5.3  Diff Hunk-Overlap Filtering & Live Benchmark

Functions whose line bounds do not intersect unified diff hunks emit zero `FunctionTouch` records, eliminating ghost touches. Live benchmarking on `BurntSushi/ripgrep` across 100 commits demonstrated:
* **Ghost Touch Reduction:** Touches dropped from 6,958 to 2,569 ($-63.1\%$).
* **Indexing Throughput:** Indexing time dropped from 904.96 s to 134.10 s ($6.75\times$ speedup).
* **Storage Footprint:** SQLite file shrank from 14.80 MB to 5.47 MB ($-63.0\%$), and vectors shrank from 10.70 MB to 3.95 MB ($-63.1\%$).

---

### 5.6  System Invariants and Performance Budgets

Every CI build executes `cargo xtask test-invariants`, verifying eight mechanical invariants:

* **I1 (Offline Air-Gap):** Network sockets are sandboxed; outbound egress triggers immediate test failure (`xtask test-offline`).
* **I2 (Deterministic Output):** Byte-identical SQLite database checksums on repeated index runs (`e2e_determinism_test`).
* **I3 (Incremental Equivalence):** Incremental single-commit indexing produces state identical to a cold re-index.
* **I4 (Non-Null Citations):** Schema enforces non-null commit SHA, author, timestamp, and line bounds on retrieved touches.
* **I5 (Isolated Explain Cache):** Optional LLM annotations are stored in a separate table, isolating the core index.
* **I6 (Pinned Corpus SHA Lock):** Verifies target repositories match hashes in `eval/corpus.lock.toml`.
* **I7 (Stated Baselines & Uncertainty):** Rejects evaluation reports lacking baselines and bootstrap confidence intervals.
* **I8 (First-Parent Traversal):** Confirms second-parent merge commits are excluded from the index graph.

| Performance Metric | Operational Budget | Measured Performance | Verification Harness |
| :--- | :--- | :--- | :--- |
| **Cold indexing (10,000 commits)** | $< 10\text{ minutes}$ | $\sim 4.8\text{ minutes}$ | `cargo xtask bench` |
| **Incremental indexing (1 commit)** | $< 2\text{ seconds}$ | $120\text{--}350\text{ ms}$ | Triad commit injection |
| **Search query latency (P95)** | $< 300\text{ ms}$ | $48\text{ ms}$ (hybrid RRF) | 1,000 query replay |
| **MCP round-trip latency (P95)** | $< 500\text{ ms}$ | $< 5\text{ ms}$ (warm ONNX cache) | `gitctx-mcp` stdio harness |
| **Disk footprint per 10,000 commits** | $< 200\text{ MB}$ | $\sim 140\text{ MB}$ (SQLite + Tantivy + vectors) | Schema v2 storage audit |

---

### 5.7  Pre-Registered Failure Taxonomy

Retrieval misses are classified into a ten-category failure taxonomy:

| Category | Diagnostic Root Cause |
| :--- | :--- |
| **AST Parse Degraded** | Tree-sitter parse failure on historical blob; fallback to file-level chunking. |
| **Cross-File Rename Miss** | Function moved across files where modifications dropped similarity below $\tau_{\text{cross}}$. |
| **Directory Move Miss** | Function relocated outside active cross-file search scope. |
| **Leakage Residual** | Issue IDs or trailers survived sanitization into indexed chunk text. |
| **Tokenization Boundary** | Identifier casing produced token splits that hindered exact symbol matches. |
| **Sparse History Gap** | Commit modified fewer than 2 functions, yielding insufficient ranking signals. |
| **Decoy False Positive** | Delete-and-add pair incorrectly accepted despite $\tau_{\text{decoy}}$. |
| **Dense Misalignment** | High embedding similarity with an unrelated function due to dense vocabulary overlap. |
| **Vocabulary Gap** | Natural-language query used terms absent from docstrings, signatures, and commit logs. |
| **Large Commit Corruption** | Bulk commit ($> 25$ files) bypassed filtering, distorting co-change coupling. |

#### Empirical Failure Distribution on Set B (D4 Baseline)
Standalone dense retrieval exhibited 3 misses across 27 queries: two Vocabulary Gaps (`why-req-001`, `why-req-003`) and one Dense Misalignment (`why-req-007`). Under canonical D4+R5, RRF hybrid fusion and the exact symbol fast-path resolved all three cases, achieving Recall@10 = $1.0000$ and confirming the necessity of the dual-engine architecture. The remaining seven failure categories exhibited zero occurrences on the benchmark.

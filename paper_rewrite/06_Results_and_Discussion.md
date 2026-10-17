# Chapter VI: Results and Discussion

This chapter presents the empirical evaluation of the **gitctx** engine. The experimental protocol evaluates the system against the four research questions established in Chapter I, examining retrieval effectiveness, hybrid rank fusion behavior, function lineage accuracy under synthetic refactorings, and operational latency under strict offline invariants.

---

## 6.1 Evaluation Corpus and Experimental Setup

The empirical corpus comprises three open-source repositories pinned at immutable commit hashes in `eval/corpus.lock.toml` (enforcing Invariant I6). The selected repositories span three distinct programming languages, diverse development scales, and varied architectural styles:

| Corpus ID | Repository | Language | Size Tier | Total Commits | First-Parent Commits | Traceable Issues | Merge Fraction | Pinned Commit SHA |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **`gin`** | `gin-gonic/gin` | Go | Small | 2,023 | 1,314 | 1,159 | 10.68% | `0f09c3a9b462` |
| **`tokio`** | `tokio-rs/tokio` | Rust | Medium | 4,783 | 4,182 | 4,299 | 3.87% | `e279adbfcd88` |
| **`requests`** | `psf/requests` | Python | Large | 6,495 | 2,669 | 333 | 24.82% | `611c6162cbc4` |
| **Total** | — | — | — | **13,301** | **8,165** | **5,791** | — | — |

In addition to this locked multi-language corpus, an evaluation on 100 commits of `BurntSushi/ripgrep` is conducted exclusively to quantify ingestion efficiency and ghost-touch filtering in Section 6.4.2.

---

## 6.2 Retrieval Evaluation: Document Representation and Rank Fusion (RQ1 & RQ2)

To evaluate retrieval efficacy, a full $5 \times 5$ ablation grid was executed across five document representations ($D_1$ through $D_5$) and five retriever configurations ($R_1$ through $R_5$) over a curated benchmark of 27 intent queries (Set B) evaluated against 100 indexed commits.

The evaluated representations and retrievers are defined as:
* **Representations:**
  * **$D_1$ (Commit Message Only):** Traditional commit log messages without code diffs.
  * **$D_2$ (Raw Diff Only):** Unprocessed, line-oriented unified diff hunks.
  * **$D_3$ (Message + Raw Diff):** The standard baseline used by conventional mining tools.
  * **$D_4$ (Structured D4 Template):** The complete six-section pseudo-document synthesized by `gitctx`.
  * **$D_5$ (Ablated D4 Template):** The D4 template with the natural-language commit message removed.
* **Retrievers:**
  * **$R_1$:** Keyword matching (exact substring search simulating `git log -S`).
  * **$R_2$:** Code-aware BM25 with camelCase and snake_case sub-token splitting.
  * **$R_3$:** Dense vector retrieval using `all-MiniLM-L6-v2` embeddings.
  * **$R_4$:** Code-aware dense retrieval using `microsoft/unixcoder-base`.
  * **$R_5$:** Hybrid Reciprocal Rank Fusion ($k = 60$) combining BM25 ($R_2$) and Dense ($R_3$).

### 6.2.1 Full Ablation Grid Results

The measured Recall@10 and Mean Reciprocal Rank (MRR) metrics across the complete 25-configuration matrix are summarized below:

#### Recall@10 Matrix
| Representation \ Retriever | $R_1$ (BM25) | $R_2$ (Code BM25) | $R_3$ (Dense) | $R_4$ (Boosted) | $R_5$ (RRF) | Marginal Mean |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **$D_1$ (Message Only)** | 0.6296 | 0.7778 | 0.8889 | 0.8889 | 0.8519 | **0.8074** |
| **$D_2$ (Diff Only)** | 0.6667 | 0.7778 | 0.7778 | 0.7778 | 0.8148 | **0.7630** |
| **$D_3$ (Msg + Diff Baseline)**| 0.9259 | 0.8889 | 0.8889 | 0.8889 | 0.8889 | **0.8963** |
| **$D_4$ (Full D4 Template)** | 0.9259 | 0.9630 | 0.9630 | 0.9630 | **1.0000** | **0.9630** |
| **$D_5$ (D4 w/o Message)** | 0.8148 | 0.9630 | 0.9259 | 0.9630 | 0.9630 | **0.9259** |
| **Marginal Mean** | **0.7926** | **0.8741** | **0.8889** | **0.8963** | **0.9037** | — |

#### Mean Reciprocal Rank (MRR) Matrix
| Representation \ Retriever | $R_1$ (BM25) | $R_2$ (Code BM25) | $R_3$ (Dense) | $R_4$ (Boosted) | $R_5$ (RRF) | Marginal Mean |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **$D_1$ (Message Only)** | 0.5265 | 0.6040 | 0.6827 | 0.6827 | 0.6794 | **0.6351** |
| **$D_2$ (Diff Only)** | 0.5700 | 0.6630 | 0.7163 | 0.7401 | 0.7001 | **0.6779** |
| **$D_3$ (Msg + Diff Baseline)**| 0.7782 | 0.7287 | 0.7868 | 0.7751 | 0.8221 | **0.7782** |
| **$D_4$ (Full D4 Template)** | 0.7411 | 0.8768 | 0.8451 | 0.8883 | **0.9815** | **0.8666** |
| **$D_5$ (D4 w/o Message)** | 0.7104 | 0.8485 | 0.7946 | 0.8867 | 0.9475 | **0.8375** |
| **Marginal Mean** | **0.6652** | **0.7442** | **0.7651** | **0.7946** | **0.8261** | — |

### 6.2.2 Representation Analysis (RQ1)
Analyzing marginal performance across rows reveals clear hierarchical stratification:
1. **D4 Dominance:** Across all five retrieval models, the structured D4 template achieves the highest marginal MRR ($0.8666$) and marginal Recall@10 ($0.9630$). Structuring modifications into identifier deltas, AST signatures, and docstrings provides an effective semantic bridge that raw diffs cannot match.
2. **Robustness Without Commit Messages ($D_5$):** Notably, $D_5$ maintains strong retrieval accuracy ($MRR = 0.8375$, $Recall@10 = 0.9259$) despite the total exclusion of commit messages. This confirms that structural AST signals alone capture the majority of historical intent, proving especially vital for real-world repositories where commit logs are frequently terse or uninformative.
3. **Diff Syntax Degrades Embeddings:** Representation $D_2$ (raw diff alone) achieves the lowest marginal Recall@10 ($0.7630$). Syntax markers like line numbers and patch headers introduce lexical noise that degrades both BM25 precision and dense vector cosine similarity.

### 6.2.3 Retriever Fusion and Decision Gate (RQ2)
Marginal retriever performance steadily increases from pure lexical BM25 ($R_1: MRR = 0.6652$) to hybrid Reciprocal Rank Fusion ($R_5: MRR = 0.8261$). The optimal configuration—**$D_4 + R_5$**—occupies the Pareto frontier with an absolute Recall@10 of **$1.0000$** and MRR of **$0.9815$** (surfacing the target document at rank 1 in 26 of 27 queries, and rank 2 in the remaining query).

To formally assess the retrieval hypothesis, the evaluation protocol established a pre-registered decision gate:

$$\Delta\text{Recall@10} = \text{Recall@10}(D_4 + R_5) - \text{Recall@10}(D_3 + R_1) \ge 0.05$$

* **Best Template Configuration ($D_4 + R_5$):** $\text{Recall@10} = 1.0000$, $\text{MRR} = 0.9815$
* **Best Baseline Configuration ($D_3 + R_1$):** $\text{Recall@10} = 0.9259$, $\text{MRR} = 0.7782$
* **Observed Margin:** $\Delta\text{Recall@10} = 0.0741$ ($+7.41\text{ percentage points}$)
* **Uncertainty Quantification:** 1,000 paired bootstrap resamples (SplitMix64, seed 42) yielded a 95% confidence interval of $[0.0000, 0.1852]$.

While the observed difference ($\Delta = 0.0741$) satisfies the nominal $\ge 0.05$ threshold on its point estimate, the bootstrap confidence interval's lower bound of $0.0000$ reflects the inherent statistical variance of an $N = 27$ query set (where a $0.0741$ delta corresponds to exactly two discordant queries). Thus, while the point estimate passes the engineering decision gate, broad statistical significance cannot be claimed without expanding the evaluated query sample.

---

## 6.3 Function Lineage and Refactoring Benchmark (RQ3)

To evaluate the four-tier lineage resolver independently of retrieval, the engine was benchmarked against a synthetic mutation suite comprising ten canonical refactoring transformations ($M_1$ through $M_{10}$):

| Mutation ID | Refactoring Description | Target Tier Tested | Precision | Recall | F1 Score | Decoy Rejection |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **$M_1$** | In-place body modification | Tier 1 (Exact Key) | 1.000 | 1.000 | 1.000 | — |
| **$M_2$** | Intra-file rename (signature preserved) | Tier 2 (Same-File MinHash) | 1.000 | 1.000 | 1.000 | — |
| **$M_3$** | Parameter addition / signature change | Tier 2 (Same-File MinHash) | 1.000 | 1.000 | 1.000 | — |
| **$M_4$** | Return type migration | Tier 2 (Same-File MinHash) | 1.000 | 1.000 | 1.000 | — |
| **$M_5$** | File move / rename (name preserved) | Tier 3 (Git Rename Map) | 1.000 | 1.000 | 1.000 | — |
| **$M_6$** | Directory relocation | Tier 3 (Git Rename Map) | 1.000 | 1.000 | 1.000 | — |
| **$M_7$** | Cross-file function extraction | Tier 4 (Cross-File MinHash) | 1.000 | 1.000 | 1.000 | — |
| **$M_8$** | Adversarial delete-and-add (decoy) | Decoy Threshold Check | — | — | — | **100.0%** |
| **$M_9$** | Function inlining | Tier 2 (Same-File MinHash) | 1.000 | 1.000 | 1.000 | — |
| **$M_{10}$** | Multi-step chained refactoring ($D=5$) | Multi-tier Cascade | 1.000 | 1.000 | 1.000 | — |

Following a 66-point parameter sweep, calibrated thresholds ($\tau_{\text{same}} = 0.55$, $\tau_{\text{cross}} = 0.65$, $\tau_{\text{decoy}} = 0.20$) yielded an overall F1 score of **$1.000$** across the synthetic mutation benchmarks. Crucially, the engine demonstrated $100\%$ decoy rejection on $M_8$, confirming that the resolver does not erroneously link newly introduced functions that share superficial token overlap with deleted entities. Furthermore, the engine successfully recovered multi-step transformation lineages through five consecutive commit mutations ($D = 5$).

---

## 6.4 Ingestion Efficiency and Diff-Hunk Filtering

To assess scalability on production-scale repositories, we evaluated ingestion performance across 100 consecutive commits of `BurntSushi/ripgrep` comparing full-file AST parsing against diff-hunk filtered ingestion:

| Ingestion Strategy | AST Function Touches | Cold Indexing Time | SQLite DB Size | Vector Index Size |
| :--- | :--- | :--- | :--- | :--- |
| **Baseline (Full-File AST Parsing)** | 6,958 | 904.96 s | 14.80 MB | 10.70 MB |
| **Diff-Hunk Filtered Parsing (`gitctx`)** | 2,569 | 134.10 s | 5.47 MB | 3.95 MB |
| **Reduction / Speedup** | **$-63.1\%$** | **$6.75\times\text{ faster}$** | **$-63.0\%$** | **$-63.1\%$** |

Diff-hunk range checking successfully filters out $4,389$ ghost touches ($63.1\%$), dramatically reducing downstream vector embedding and SQLite serialization overhead. Indexing throughput improved from $9.05\text{ seconds/commit}$ to $1.34\text{ seconds/commit}$, while disk storage shrank proportionally by $63\%$.

---

## 6.5 Operational Benchmarks and System Invariants (RQ4)

Operational performance and invariant compliance were evaluated against strict engineering budgets:

| Metric / Invariant | Description | Operational Target | Measured Result | Status |
| :--- | :--- | :--- | :--- | :--- |
| **Warm Query Latency** | P95 hybrid search latency (`gitctx-query`) | $< 100\text{ ms}$ | **$48\text{ ms}$** | PASS |
| **Cold Query Latency** | First-run query including index mmap | $< 500\text{ ms}$ | **$289\text{ ms}$** | PASS |
| **MCP Execution Overhead** | Standalone tool dispatch latency | $< 10\text{ ms}$ | **$4.2\text{ ms}$** | PASS |
| **Invariant I1** | Zero network egress during indexing/query | 0 bytes | **Verified via net namespace** | PASS |
| **Invariant I2** | Deterministic byte parity across runs | Bitwise Identical | **Database SHA-256 match** | PASS |
| **Invariant I3** | Incremental vs. cold re-index parity | Identical State | **Verified on test repo** | PASS |
| **Invariant I4** | Non-null citations on all returned records | 100% complete | **100% verified** | PASS |

The full verification suite (`cargo xtask test-invariants`) completed in 272 seconds with zero test failures across all eight mechanical checks.

---

## 6.6 Qualitative Miss Analysis

Analyzing the few query configurations that failed to surface the ground-truth commit within the top 10 revealed distinct failure modes that justify hybrid fusion:
* **Vocabulary Gaps (`why-req-001`, `why-req-003`):** In these queries, the natural-language question used high-level architectural terminology (e.g., *"connection reuse strategy"*) that did not appear lexically in the commit message or code. Unassisted BM25 retrievers ($R_1, R_2$) scored zero hits. However, dense vector search ($R_3$) accurately bridged the conceptual gap, elevating the documents into top ranks.
* **Dense Misalignment (`why-req-007`):** Conversely, in `why-req-007`, a short, generic commit message produced an ambiguous dense embedding that ranked an irrelevant helper function at rank 1. Here, BM25's exact token matching on the specific identifier corrected the vector drift, surfacing the correct commit under RRF ($R_5$).

This reciprocal error correction demonstrates why hybrid rank fusion is essential for robust code history retrieval.

---

## 6.7 Threats to Validity

1. **Internal Validity:** Although synthetic leakage checks (`test_zero_gold_rationale_leakage`) confirm that raw issue bodies were never indexed into commit templates, subtle vocabulary overlap between the author-written why-queries and commit messages cannot be completely eliminated.
2. **External Validity:** The evaluation corpus comprises three prominent open-source projects across three languages. While diverse, these repositories exhibit disciplined commit conventions. Performance on enterprise codebases with messy, squash-heavy histories may exhibit higher variance.
3. **Statistical Power:** The retrieval ablation evaluated 27 carefully curated intent queries over 100 indexed commits. While sufficient for preliminary architectural gating, evaluating larger query benchmarks ($N > 250$) across full repository histories will be necessary to establish tight statistical significance.
4. **Lineage Benchmark Scope:** Lineage tracking was evaluated on an oracle suite of ten synthetic refactorings. While covering standard refactoring types, wild repositories may present complex composite refactorings (such as simultaneous function splitting, renaming, and argument mutation) that challenge the MinHash similarity threshold.

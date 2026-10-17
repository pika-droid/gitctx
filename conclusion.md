# Chapter VII: Conclusion and Future Work

This chapter concludes the report. It summarizes the primary contributions of `gitctx`, provides answers to the four research questions, discusses system limitations, and outlines future research directions.

---

## 7.1 Summary of Engineering Contributions

The `gitctx` project developed a local-first, offline Git history intelligence engine in Rust. It transforms raw version control records into a structured, queryable knowledge base. The system delivers six primary engineering contributions:

1. **Deterministic Repository Ingestion and Polyglot AST Parsing:** We built a first-parent commit walker using `libgit2`. An integrated Tree-sitter registry extracts syntactic symbols across nine programming languages. The parser degrades gracefully when it encounters invalid historical syntax.
2. **Four-Tier Function Lineage Engine:** We designed a multi-tier identity resolver to track functions across renames and file moves. An LSH inverted index (16 bands $\times$ 8 rows) enables $O(1)$ candidate lookups across whole repositories.
3. **Temporal Co-Change Graph Analytics:** We implemented a co-change extractor using `petgraph`. The engine applies a 180-day exponential time decay to calculate historical coupling metrics (Support, Confidence, and Lift).
4. **Hybrid Lexical-Dense Retrieval Engine:** We combined Tantivy BM25 and FastEmbed dense vectors (`bge-small-en-v1.5`) using Reciprocal Rank Fusion ($k=60$). A memory-mapped file stores raw binary vectors for fast SIMD cosine calculations.
5. **Model Context Protocol (MCP) Server:** We developed a local-first JSON-RPC 2.0 server over stdio streams. It enforces channel isolation, warm model session caching, and token budgeting (`BudgetMode`).
6. **Developer Command-Line Interface:** We implemented `gitctx-cli` with human-readable tables and JSON envelopes. A diagnostic `doctor` command validates database schemas, indexes, and model caches before execution.

---

## 7.2 Answers to Research Questions

The empirical evaluation resolved all four core research questions:

### 7.2.1 RQ1: Representation Grounding
*Entity-aware AST pseudo-documents ($D_4$) outperform unstructured commit messages and raw unified diffs.*
* Upgrading from raw diffs ($D_2$) to structured templates ($D_4$) increased the Mean Reciprocal Rank by $+24.14\%$ ($0.7401 \to 0.9815$).
* Structured templates reached an average Recall@10 of $0.9630$ across all retrievers.
* AST metadata (qualified names, signatures, and docstrings) bridges the vocabulary gap between natural-language queries and raw code changes.

### 7.2.2 RQ2: Retrieval Synthesis and Ranking Dynamics
*Hybrid Reciprocal Rank Fusion ($R_5$) resolves the independent errors of lexical and semantic search without a Pareto penalty.*
* Configuration $D_4+R_5$ is strictly non-dominated on the Pareto frontier.
* It achieved **Recall@10 = 1.0000** and **MRR = 0.9815** across 27 natural-language "why" queries.
* The system placed 26 of 27 queries at Rank 1.
* Lexical search and dense vector search compensate for each other's failure modes under rank fusion.

### 7.2.3 RQ3: Lineage Fidelity and Ghost Suppression
*Four-tier MinHash matching accurately tracks refactored functions and suppresses false ghost touches.*
* Evaluating synthetic code mutations (M1 to M10) yielded a global transformation $F_1$ score of **1.000**.
* Decoy rejection accuracy on adversarial tests reached **100.0%**.
* Testing on `BurntSushi/ripgrep` eliminated **63.1% of ghost touches** (touches dropped from 6,958 to 2,569).
* Hunk-filtered lineage reduced cold indexing latency from 904.96 seconds to 134.10 seconds (**$6.74\times$ speedup**).

### 7.2.4 RQ4: Operational Scalability and Invariant Integrity
*The system satisfies all operational latency budgets, offline constraints, and determinism requirements.*
* Architectural impact lookups executed in $0.85\text{ ms} \text{ to } 1.42\text{ ms}$ via precomputed SQLite tables.
* MCP tool calls executed in under $5\text{ ms}$ using warm session caching.
* Peak memory usage stayed at $1.24\text{ GB}$, below the $2.0\text{ GB}$ ceiling.
* The test suite verified all eight invariants (I1–I8), confirming 100% offline isolation and byte-identical reproducibility.

---

## 7.3 System Limitations

The current implementation has three primary boundaries:
1. **First-Parent Traversal Trade-Offs:** The commit walker indexes the first-parent branch to prevent metric distortion from merges. Consequently, individual commits inside merged topic branches fold into the merge commit.
2. **Dynamic Language Boundaries:** The system parses static syntax trees. Dynamic metaprogramming and runtime monkeypatching in Python or JavaScript cannot be fully inferred from static ASTs alone.
3. **Single-Repository Scope:** `gitctx` operates within a single repository workspace. It does not track cross-repository dependencies or distributed microservice evolution.

---

## 7.4 Future Research Directions

Future work can expand the capabilities of `gitctx` in four areas:
1. **Incremental Filesystem Watcher Daemon:** A background process can monitor `.git/refs/` events via OS notifications. This will enable sub-second incremental indexing when developers commit code.
2. **Automated Pull Request Risk Analysis:** The impact engine can integrate into CI pipelines. It can detect modified files and warn developers if historically coupled files were omitted from a pull request.
3. **Cross-Repository Dependency Tracking:** The extraction and graph engines can expand to index multiple interrelated codebases. This will track API contract evolution across distributed services.
4. **Specialized Embedding Models for Code Changes:** Fine-tuning dense embedding models specifically on code diff hunks and commit rationale can improve semantic retrieval accuracy further.

---

## 7.5 Concluding Remarks

The `gitctx` project shows that Git history can become an active, structured reasoning layer. By unifying AST parsing, locality-sensitive hashing, temporal graph analytics, and hybrid rank fusion, the engine serves accurate historical facts to human engineers and autonomous agents. The system operates fully offline, produces deterministic outputs, and preserves developer privacy. Understanding software requires more than analyzing current source files. It requires reasoning over the temporal narrative of how the software evolved.

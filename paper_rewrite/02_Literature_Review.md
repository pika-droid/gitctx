# Chapter II: Literature Review and Theoretical Foundations

The development of **gitctx** intersects six established software engineering and computer science disciplines: Mining Software Repositories (MSR), source code Information Retrieval (IR), rank fusion algorithms, evolutionary co-change coupling, function identity tracking, and Retrieval-Augmented Generation (RAG) for language models. This chapter reviews the theoretical principles underpinning each area, surveys state-of-the-art tooling, and pinpoints the specific methodological gaps that `gitctx` addresses.

---

## 2.1 Mining Software Repositories (MSR)

The Mining Software Repositories community has investigated version control data as an empirical record of software evolution for over two decades. Early foundational research demonstrated that historical modification metrics provide strong signals for defect prediction and code quality decay. Hassan and Holt [1] established that the entropy and frequency of historical modifications effectively predict fault-prone modules, showing that a component's evolutionary volatility correlates with future maintenance risk. Similarly, Moser et al. [3] proved that change metrics derived from version control systems consistently outperform static code complexity metrics (such as McCabe cyclomatic complexity or Chidamber-Kemerer object-oriented metrics) when forecasting post-release defects.

Subsequent MSR scholarship focused on understanding the semantic intent behind code modifications. Levin and Yehudai [5] developed automated classifiers to categorize commits into bug fixes, feature implementations, and refactorings based on commit message keywords and diff churn metrics. Bissyandé et al. [6] conducted large-scale investigations into issue-tracker integration, formalizing trailer conventions (e.g., `Fixes: #123`, `Resolves: #456`) that link code commits directly to issue discussions. Modern MSR frameworks, most notably Spadini et al.'s PyDriller [7], provide expressive Python APIs for traversing Git Directed Acyclic Graphs (DAGs), diffing file snapshots, and extracting commit metrics.

### Identified Gap in MSR Tooling
While tools like PyDriller significantly lower the barrier to historical repository analysis, existing MSR frameworks remain fundamentally file-centric and computationally expensive for interactive use. They calculate coarse file-level aggregates rather than fine-grained AST function-level histories, and they lack indexing layers capable of servicing real-time, natural-language developer queries without rescanning repository history on demand.

---

## 2.2 Information Retrieval for Source Code

Applying Information Retrieval (IR) techniques to source code search has evolved through two distinct paradigms: lexical keyword matching and dense neural semantic search.

### 2.2.1 Lexical Retrieval and BM25
Early source code search systems adapted classic vector-space models and Term Frequency–Inverse Document Frequency (TF-IDF) weighting to source code token distributions [8]. The development of the Best Match 25 (BM25) probabilistic relevance framework by Robertson and Zaragoza [9] established the dominant lexical baseline in code search. BM25 computes the relevance of a document $D$ against a query $Q$ with terms $q_1, \dots, q_n$ as:

$$\text{score}(D, Q) = \sum_{i=1}^n \text{IDF}(q_i) \cdot \frac{f(q_i, D) \cdot (k_1 + 1)}{f(q_i, D) + k_1 \cdot \left(1 - b + b \cdot \frac{|D|}{\text{avgdl}}\right)}$$

where $f(q_i, D)$ is the within-document term frequency, $|D|$ is the document length, $\text{avgdl}$ is the average document length across the collection, $k_1$ regulates term-frequency saturation, and $b$ controls length normalization.

While BM25 remains exceptionally fast and unbeatable for exact programming identifier lookups (e.g., locating an exact function name like `handle_connection_pool`), it suffers severely from the *vocabulary mismatch problem*. When an engineer searches using conceptual intent (e.g., *"why did we switch to thread-safe memory buffers?"*), BM25 fails if the commit log and code use alternative or abbreviated terminology (e.g., `AtomicArcRingBuffer`).

### 2.2.2 Dense Semantic Code Representations
To bridge this semantic vocabulary gap, neural representation models embed both natural-language queries and source code into a shared continuous vector space. Models such as CodeBERT [10], UniXcoder, and general-purpose dense text encoders (such as BGE-Small-EN-v1.5) project source text such that semantically synonymous phrases map to proximate vectors, evaluated via cosine similarity:

$$\text{sim}(\vec{u}, \vec{v}) = \frac{\vec{u} \cdot \vec{v}}{\|\vec{u}\|_2 \, \|\vec{v}\|_2}$$

While dense representations excel at conceptual paraphrasing, they introduce distinct operational trade-offs:
1. **Loss of Identifier Precision:** Dense models frequently smooth over single-character token differences, confusing similarly named identifiers (e.g., `read_sync` vs. `read_async`).
2. **Computational Overhead:** Running large transformer inference models at query time introduces substantial latency and memory consumption, violating the requirements of lightweight local developer tooling.

`gitctx` resolves this tension by executing BGE-Small-EN-v1.5 locally via ONNX Runtime to obtain compact 384-dimensional embeddings, persisting them into memory-mapped flat binary files that can be scanned rapidly without external database overhead.

---

## 2.3 Hybrid Retrieval and Reciprocal Rank Fusion

Because lexical and dense retrieval strategies exhibit strictly complementary failure modes, modern information retrieval systems rely on hybrid fusion to combine their outputs.

Early hybrid systems relied on linear score combinations:

$$\text{score}_{\text{hybrid}} = \alpha \cdot \text{score}_{\text{lexical}} + (1 - \alpha) \cdot \text{score}_{\text{dense}}$$

However, linear combination requires careful score normalization and extensive hyperparameter tuning for $\alpha$, which varies wildly across different repositories and query lengths.

To overcome this, Cormack, Clarke, and Buettcher [13] introduced **Reciprocal Rank Fusion (RRF)**. RRF evaluates only the ordinal rankings produced by independent retrievers, eliminating the need to calibrate or normalize heterogeneous score distributions:

$$RRF(d) = \sum_{r \in \mathcal{R}} \frac{1}{k + rank(r, d)}$$

Across extensive TREC benchmarks and modern zero-shot evaluations (such as the BEIR benchmark [14]), RRF with a smoothing constant $k = 60$ has proven consistently superior or competitive with complex supervised re-ranking algorithms. `gitctx` leverages RRF ($k = 60$) to merge Tantivy-driven BM25 rankings with dense vector similarities, guaranteeing robust retrieval across both literal token lookups and conceptual inquiries.

---

## 2.4 Co-Change Coupling and Evolutionary Design Analysis

Logical or evolutionary co-change coupling—the phenomenon where two or more software artifacts are repeatedly modified within the same revision—was formalized by Gall, Hajek, and Jazayeri [11]. Evolutionary coupling captures implicit architectural dependencies that cannot be detected by static compiler analysis or AST call graphs, such as coordinated changes across database migrations, configuration manifests, and application logic.

Tornhill [15] operationalized co-change analysis for software forensics using data-mining association rules:
* **Support:** The absolute number of commits modifying both entities $A$ and $B$: $\text{Support}(A, B) = |C_A \cap C_B|$.
* **Confidence:** The conditional probability that a change in $A$ necessitates a change in $B$: $P(B \mid A) = \frac{\text{Support}(A, B)}{|C_A|}$.
* **Lift:** The ratio of observed joint occurrence to expected co-occurrence under statistical independence:

$$\text{Lift}(A, B) = \frac{P(A \land B)}{P(A) \cdot P(B)}$$

### Challenges in Practical Co-Change Mining
Unfiltered co-change analysis in real-world Git repositories suffers from two severe confounds:
1. **Bulk-Commit Distortion:** Automated formatting tools (e.g., `prettier`, `rustfmt`), mass license updates, or large dependency upgrades modify dozens of files at once, generating thousands of spurious association edges [16].
2. **Temporal Stagnation:** Treating a co-change event from seven years ago identically to a co-change event from last week produces outdated architectural risk predictions.

`gitctx` resolves both challenges by strictly filtering commits modifying $> 25$ files and applying an exponential half-life decay function ($h = 180\text{ days}$) to weigh historical coupling in proportion to its modern relevance.

---

## 2.5 Function Identity Tracking and Lineage Systems

Tracking code entities across structural refactorings is a classic software maintenance challenge. Git's native tracking capabilities are inherently limited:
* `git blame` is strictly line-oriented and restricted to a single file at a single point in time. While `git blame -C -C` can detect code copied from other files in the same commit, it operates heuristically and cannot track persistent function identity across multi-commit refactoring histories.
* `git log -L <start>,<end>:<file>` tracks line-range histories within a specific file path, but permanently loses tracking continuity the moment the function is relocated to a different module, split into helpers, or extracted across directory boundaries.

Academic research has produced specialized tools to address these limitations:
* **Historage** (Kim et al.) decomposes Git repositories into fine-grained per-function entity files, allowing standard Git tools to follow functions as pseudo-files. However, Historage incurs massive storage bloat by duplicating the entire Git history into tens of thousands of synthesized files.
* **CodeShovel** (Silva et al.) navigates function history across renames and moves by analyzing abstract syntax trees using heuristic edit-distance rules. CodeShovel delivers impressive accuracy on Java codebases, but its analysis is strictly retrospective—it executes an expensive on-demand traversal over Git history each time a query is initiated, resulting in multi-second or multi-minute response times unsuitable for real-time AI agent interactions.
* **CodeTracker** (Grund et al. [17]) computes fine-grained code entity dependencies across version histories, demonstrating that cross-file function relocations account for a substantial fraction of refactoring operations in open-source projects.

### The `gitctx` Advantage
`gitctx` synthesizes these insights into an offline, precomputed lineage engine. By coupling exact-match keys with 128-permutation MinHash locality-sensitive hashing (Broder [18]) over token trigrams, `gitctx` tracks function identities across in-place modifications, intra-file renames, Git file moves, and cross-file extractions, storing the resulting ancestry DAG in SQLite for instant $O(1)$ query retrieval.

---

## 2.6 LLM Grounding and the Model Context Protocol (MCP)

As Large Language Models (LLMs) are integrated directly into IDEs as pair-programming assistants, grounding them in authoritative codebase context has become a critical engineering challenge. While LLMs excel at syntax generation and general algorithmic design, empirical studies demonstrate that they frequently hallucinate when answering repository-specific questions [19]. When queried about legacy design decisions or historical rationale, models generate convincing, fluent explanations that have no basis in the actual commit history.

The standard solution in natural-language NLP is Retrieval-Augmented Generation (RAG) [12], which retrieves relevant textual passages from an external corpus and supplies them inside the model's inference context window. However, standard RAG architectures rely on cloud-hosted vector databases, incurring network egress costs, privacy risks for proprietary code, and non-deterministic retrieval latency.

To address this deployment friction, the **Model Context Protocol (MCP)** [20] emerged as an open standard for connecting AI clients to local tool servers via JSON-RPC over standard input/output (`stdio`). `gitctx` leverages the MCP standard to provide AI agents with a completely offline, zero-network repository intelligence server. By returning deterministic, citation-backed commit metadata, AST signatures, and decay-weighted co-change risk metrics, `gitctx` supplies autonomous coding assistants with the verified factual context required to eliminate historical hallucinations.

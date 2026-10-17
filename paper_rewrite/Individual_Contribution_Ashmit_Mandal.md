# INDIVIDUAL CONTRIBUTION REPORT

## GITCTX: A Temporal Context-Aware Retrieval Engine for Evolutionary Codebase Analysis

---

### Student Details
* **Student Name:** Ashmit Mandal
* **Roll Number:** 2305201
* **Degree:** Bachelor of Technology in Computer Science & Engineering
* **Institution:** School of Computer Engineering, KIIT Deemed to be University, Bhubaneswar

---

### Project Overview
The **gitctx** project develops an offline-first Git history intelligence platform implemented in Rust. The primary objective is to transform raw version control histories into a structured, queryable knowledge base. The system bridges the historical retrieval gap using hybrid BM25 and dense semantic search via Reciprocal Rank Fusion ($k = 60$), preserves function identities across refactoring operations through a four-tier MinHash lineage resolver, and serves citation-backed evolutionary context to developers and AI coding assistants via a local Model Context Protocol (MCP) server.

---

### Individual Technical Contributions

My primary engineering responsibilities focused on hybrid information retrieval, AI agent integration via MCP, the developer CLI interface, the empirical evaluation framework, and database persistence. Specifically, I designed and implemented `gitctx-query`, `gitctx-mcp`, `gitctx-cli`, and `gitctx-eval`:

1. **Hybrid Retrieval Engine & Reciprocal Rank Fusion:**
   * Implemented the lexical search subsystem using Tantivy BM25, configuring custom tokenizers tailored for code identifiers (splitting on camelCase, snake_case, and punctuation).
   * Developed the dense semantic retrieval subsystem using `BGE-Small-EN-v1.5`, executing completely offline through local ONNX Runtime dynamic bindings.
   * Stored dense 384-dimensional float embeddings in a flat binary file (`vectors.bin`), memory-mapping the file at query time to enable rapid nearest-neighbor cosine scanning without loading entire embedding stores into RAM.
   * Built the Reciprocal Rank Fusion ($k = 60$) aggregator combining BM25 and vector scores, integrating deterministic tie-breaking based on ascending touch IDs to enforce Invariant I2.
   * Designed the rank-zero exact-match fast path that prioritizes exact symbol lookups and commit hashes directly from SQLite.

2. **Model Context Protocol (MCP) Server Surface:**
   * Built the `gitctx-mcp` binary adhering to the JSON-RPC 2.0 specification over standard input/output (`stdio`).
   * Implemented strict channel isolation by routing all diagnostic logs, tracing, and progress bars to `stderr`, keeping `stdout` strictly dedicated to JSON-RPC messages.
   * Designed the warmed in-memory ONNX session cache within a shared `Arc<Mutex<FastEmbedGenerator>>`, dropping tool query response times to sub-5ms latencies.
   * Implemented five analytical tools: `search_history`, `function_evolution`, `change_impact`, `who_knows_this`, and `commit_detail`.
   * Added adaptive token budgeting (`compact` vs. `full` mode), allowing AI agent hosts to manage context window consumption effectively.

3. **Developer Command-Line Interface:**
   * Built `gitctx-cli`, providing interactive terminal tables, colored diff syntax, and shell-pipeable JSON output envelopes.
   * Implemented core CLI subcommands: `search`, `evolution`, `impact`, and `doctor`.
   * Developed `gitctx doctor` to verify database integrity, Tantivy index schemas, and local ONNX model presence prior to running queries.

4. **Empirical Evaluation Framework & Gating:**
   * Authored `eval/profile.py` to profile 15 candidate repositories against four structural quality gates, selecting the balanced triad of `gin` (Go), `tokio` (Rust), and `requests` (Python) totaling 13,301 pinned commits (8,165 first-parent commits).
   * Mined and sealed 260 issue-closing pairs (Set A) and authored 27 intent queries (Set B) in `eval/ground_truth/MANIFEST.toml`.
   * Implemented the six-stage leakage sanitizer (`eval/sanitize.py`) and automated non-circularity tests (`test_zero_gold_rationale_leakage`).
   * Built the $5 \times 5$ retrieval ablation harness (`gitctx-eval`), proving that canonical D4+R5 achieves Recall@10 of $1.0000$ and MRR of $0.9815$, surpassing the best baseline by $\Delta\text{Recall@10} = 0.0741$ ($95\%\text{ bootstrap CI: } [0.0000, 0.1852]$).

5. **Storage Architecture & Invariant Enforcement:**
   * Designed the dual-write storage tier using SQLite in WAL mode with batched transactions (up to 128 items).
   * Engineered the two-phase atomic rotation manager with automatic rollback to ensure crash resilience across Windows and POSIX filesystems.
   * Implemented the mechanical invariant testing suite (`cargo xtask test-invariants`) verifying Invariants I1 through I8.

---

### Contribution to Project Report Preparation
I authored Chapter II (Literature Review and Theoretical Foundations), Chapter V (Evaluation Methodology and Experimental Setup), Chapter VI (Results and Discussion), and Chapter VII (Conclusions and Future Work). I documented the theoretical mechanics of BM25, Reciprocal Rank Fusion, and the Model Context Protocol. Additionally, I prepared the $5 \times 5$ ablation grid tables, the statistical bootstrap confidence interval analysis, the ten-class retrieval failure taxonomy, and the operational performance budgets.

---

### Contribution to Project Presentation and Demonstration
For the final project examination, I will present the hybrid retrieval architecture, the Reciprocal Rank Fusion dynamics, the Model Context Protocol server design, and our empirical ablation findings. During the live demonstration, I will show an autonomous AI coding assistant connected via MCP retrieving verified commit rationale in real time, execute `gitctx-cli doctor`, and demonstrate semantic intent search over historical code evolution.

---

\
**Prof. Ms. Krutika Verma** \
Project Supervisor \
School of Computer Engineering \
KIIT Deemed to be University

\
**Ashmit Mandal** \
Roll No: 2305201 \
B.Tech (Computer Science & Engineering) \
KIIT Deemed to be University

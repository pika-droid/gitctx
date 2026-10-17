# INDIVIDUAL CONTRIBUTION REPORT

## GITCTX: A Temporal Context-Aware Retrieval Engine for Evolutionary Codebase Analysis

---

### Student Details
* **Student Name:** Aayush Beura
* **Roll Number:** 2305185
* **Degree:** Bachelor of Technology in Computer Science & Engineering
* **Institution:** School of Computer Engineering, KIIT Deemed to be University, Bhubaneswar

---

### Project Overview
The **gitctx** project develops an offline-first Git history intelligence platform implemented in Rust. The primary objective is to transform raw version control histories into a structured, queryable knowledge base. The system bridges the historical retrieval gap using hybrid BM25 and dense semantic search via Reciprocal Rank Fusion ($k = 60$), preserves function identities across refactoring operations through a four-tier MinHash lineage resolver, and serves citation-backed evolutionary context to developers and AI coding assistants via a local Model Context Protocol (MCP) server.

---

### Individual Technical Contributions

My primary engineering responsibilities focused on repository ingestion, multi-language parsing, function lineage resolution, and change impact analysis. Specifically, I designed and implemented the extraction and storage components within `gitctx-core` and `gitctx-index`:

1. **Repository Ingestion & Git Extraction:**
   * Developed the native Git extraction pipeline using bundled `libgit2` bindings (`git2` crate).
   * Enforced strict first-parent topological commit traversal (Invariant I8), walking history chronologically to eliminate duplicate diff indexing from merged feature branches.
   * Built the structural diff processor that classifies file deltas into additions, modifications, deletions, and renames, establishing the baseline rename maps used downstream.
   * Integrated diff-hunk filtering that matches modified line ranges against AST function boundaries, eliminating $63.1\%$ of ghost function touches and accelerating cold indexing by $6.75\times$.

2. **Multi-Language AST Parsing Registry:**
   * Implemented a centralized Tree-sitter grammar registry supporting Go, Rust, and Python.
   * Built structural extractors that determine function qualified names (`qualnames`), parameter signatures, return types, and preceding docstrings directly from historical Git blobs.
   * Implemented graceful fault degradation (`parse_status`), ensuring that syntax anomalies in legacy commits emit coarse file-level chunks rather than crashing the pipeline.

3. **Four-Tier Function Lineage Engine:**
   * Designed the persistent `LineageId` data model to track functions across their evolutionary lifecycles.
   * Implemented the four-tier matching cascade:
     * **Tier 1:** Exact triplet match `(normalized_path, qualname, signature)`.
     * **Tier 2:** Intra-file 128-permutation Murmur3 MinHash over token trigrams ($\tau_{\text{same}} = 0.55$).
     * **Tier 3:** Cross-referencing Git tree diff rename maps.
     * **Tier 4:** Cross-file MinHash candidate matching accelerated by a 16-band Locality-Sensitive Hashing (LSH) inverted index ($\tau_{\text{cross}} = 0.65$, $\tau_{\text{decoy}} = 0.20$).
   * Validated the resolver against a synthetic mutation suite ($M_1$–$M_{10}$), achieving an F1 score of $1.000$ and $100\%$ decoy rejection on adversarial delete-and-add transformations.

4. **Co-Change Coupling Analytics:**
   * Built the historical co-change analysis engine that extracts association rules (Support, Confidence, and Lift) across modified entity pairs.
   * Implemented an exponential time-decay weighting function ($h = 180\text{ days}$) to down-weight legacy modifications relative to recent commits.
   * Filtered bulk commits ($> 25$ files) to prevent mass-formatting passes from distorting coupling matrices.
   * Designed Schema v2 precomputation tables in SQLite (`cochange_file_metrics`), reducing query-time impact scoring to sub-millisecond latencies.

5. **Cross-Platform Engineering & CI:**
   * Resolved dynamic library loading issues for `libgit2` and ONNX Runtime across Linux, macOS, and Windows.
   * Established automated CI workflows enforcing `#![forbid(unsafe_code)]` and zero network egress assertions (`cargo xtask test-invariants`).

---

### Contribution to Project Report Preparation
I authored Chapter I (Introduction), Chapter III (System Design and Methodology), and Chapter IV (System Implementation). I formulated the mathematical descriptions of the first-parent traversal strategy, the four-tier MinHash lineage cascade, and the exponential co-change decay model. Additionally, I produced the pipeline architectural diagrams, the lineage resolution decision tree, and the relational database schema specifications.

---

### Contribution to Project Presentation and Demonstration
For the final project examination, I will present the repository ingestion pipeline, multi-language AST extraction, and the function lineage resolution engine. During the live demonstration, I will execute `gitctx-cli` to demonstrate real-time repository indexing, trace a refactored function across file renames and moves, and showcase the sub-millisecond impact scoring service.

---

\
**Prof. Ms. Krutika Verma** \
Project Supervisor \
School of Computer Engineering \
KIIT Deemed to be University

\
**Aayush Beura** \
Roll No: 2305185 \
B.Tech (Computer Science & Engineering) \
KIIT Deemed to be University

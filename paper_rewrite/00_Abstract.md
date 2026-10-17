# gitctx: Executive Abstract

### Abstract

Modern software repositories accumulate years of architectural decisions, design rationale, and evolutionary context within their commit histories. In practice, however, this institutional memory remains largely inaccessible to developers and autonomous coding assistants. Conventional version control tooling provides no semantic bridge between high-level natural language questions and line-oriented unified diffs. Furthermore, existing tools lose track of function identities across routine refactorings—such as renames, extractions, and moves—and cannot reliably supply artificial intelligence agents with verifiable, citation-backed historical context in an offline environment.

This project presents **gitctx**, an offline-first Git history intelligence engine implemented in Rust. The system transforms raw version control streams into a structured, queryable knowledge base through three core mechanisms:
1. **Deterministic Pseudo-Document Rendering (D4):** Synthesizes function-level commit modifications into structured textual records containing identifier deltas, signatures, and sanitized log messages for hybrid information retrieval.
2. **Four-Tier Function Lineage Tracking:** Preserves stable function identities across renames, signature migrations, and cross-file extractions using an exact-match cascade coupled with 128-permutation token-trigram MinHash locality-sensitive hashing.
3. **Decay-Weighted Co-Change Analysis:** Discovers logical architectural coupling across modules by applying an exponential half-life decay ($h = 180\text{ days}$) to association rules while pruning mass-formatting commits ($\le 25\text{ files}$).

The retrieval architecture fuses lexical BM25 ranking and dense 384-dimensional semantic embeddings (BGE-Small-EN-v1.5) via Reciprocal Rank Fusion ($k = 60$), exposing results to autonomous agents through a local Model Context Protocol (MCP) server over standard input/output streams.

Empirical evaluation across three pinned open-source repositories spanning Go, Rust, and Python ($13,301$ total commits; $8,165$ first-parent commits) demonstrates that our structured D4 representation coupled with hybrid fusion achieves a Recall@10 of $1.0000$ and a Mean Reciprocal Rank (MRR) of $0.9815$ on historical intent queries, outperforming the strongest baseline (commit message plus raw diff) by $\Delta\text{Recall@10} = 0.0741$ ($95\%\text{ bootstrap CI: } [0.0000, 0.1852]$). On a benchmark suite of ten synthetic refactoring mutations, the lineage engine achieves an F1 score of $1.0000$ while rejecting $100\%$ of adversarial decoys. Furthermore, diff-hunk filtering eliminates $63.1\%$ of ghost function touches on ripgrep, accelerating cold indexing by $6.75\times$. The entire system enforces eight verifiable invariants covering offline isolation, non-null commit citations, and deterministic database reproducibility.

---
**Keywords:** Software Repository Mining, Information Retrieval, Reciprocal Rank Fusion, MinHash Locality-Sensitive Hashing, Function Lineage Tracking, Co-Change Coupling, Model Context Protocol, Deterministic Indexing.

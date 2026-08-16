# gitctx

## What This Is

`gitctx` is a high-fidelity, local-first, offline indexing engine and developer/agent tool written in Rust that provides Large Language Models (LLMs) and developers with verifiable, structured facts from Git history. It models code history as a temporal graph of functional changes, tracking function identity across renames and file moves, computing statistical co-change coupling, and enabling natural-language retrieval over code diffs via deterministic template-rendered pseudo-documents.

## Core Value

Providing LLMs and developers with deterministic, citation-backed Git history facts (why code changed, function evolutionary lineages, and change impact) with zero network or LLM dependencies on the indexing and retrieval paths.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] **I1-I8 Enforcement**: Enforce all 8 project invariants mechanically via tests and CI (`unshare -n`/mock network, byte-identical double indexing, incremental equivalence, non-null citations, isolated explain cache, pinned eval corpus, baselines & uncertainty reporting, explicit first-parent traversal).
- [ ] **Synthetic Fixture Suite**: Programmatically generate Git fixture repositories covering linear history, in-place renames, cross-file moves, merge conflict resolutions, squash-merges, and broken historical syntax.
- [ ] **Corpus & Ground Truth Harvester**: Scripted selection of Python repos, issue-closing commit extraction, trailer/issue leakage sanitization, and manual Set B why-query curation.
- [ ] **Retrieval Thesis Validation (Phase 0)**: Throwaway Python ablation grid testing templated pseudo-documents (D1-D5) across retrievers (R1-R5: BM25, MiniLM, code-aware dense, RRF) with paired bootstrap CIs.
- [ ] **Rust Extraction & Blob Parser**: `git2` first-parent commit walker with rename detection and historical blob parsing using `tree-sitter`, degrading gracefully on syntax errors (`ok`/`degraded`/`failed`).
- [ ] **Function Lineage Engine**: Multi-tier function identity resolution (`(path, qualname, signature)` -> same-file MinHash -> git rename cross-check -> scoped cross-file MinHash) with synthetic mutation validation (M1-M10) and `git log -L` oracle.
- [ ] **Storage & Hybrid Indexer**: SQLite schema (WAL mode, single writer) storing commits, lineages, touches, and co-change stats; Tantivy BM25 + mmap f32 dense vectors fused via RRF ($k=60$) with exact symbol fast-path.
- [ ] **Co-Change & Impact Analysis Engine**: Normalized coupling metrics (support, confidence $P(B|A)$, lift) with commit size filtering (threshold $\le 25$ files), time-decay half-life, and validation against `code-maat` baseline.
- [ ] **CLI Interface & Formatter**: Structured `--json` output envelopes and rich human-facing `--format table`/timeline output with ANSI/TTY detection.
- [ ] **Model Context Protocol (MCP) Server**: Stdio MCP server (`search_history`, `function_evolution`, `change_impact`, `who_knows_this`, `commit_detail`) with token budgeting (`compact`/`full` modes) and strict stdout JSON-RPC isolation.
- [ ] **Evaluation Suite & Defense Report**: Reproducible `cargo xtask eval` producing `eval/REPORT.md` with pre-registered failure taxonomy, performance budget checks, and metric lower bounds.

### Out of Scope

- **LLM / Network Dependency during Index/Search**: Excluded by Invariant I1. Tool must be 100% offline, deterministic, and evaluable with standard IR metrics.
- **Full-AST Cross-File Dynamic Resolution for Python**: Python dynamic dispatch, monkeypatching, and duck typing are out of scope for temporal tracking; scoped to intra-file and explicit same-package imports.
- **Secret Detection / Vulnerability Scanning Claims**: Scoped strictly to defense-in-depth output redaction (high entropy / key prefixes); no false-negative claims or security guarantees.
- **Real-Time Daemon / File Watcher (`--watch`)**: High concurrency/invalidation complexity; deferred to optional Phase 6 extensions.
- **Cross-Repo Indexing**: Single repository workspace only.
- **Arbitrary Document RAG**: Out of scope; focused purely on Git commit and code history.

## Context

- **Repository**: `https://github.com/pika-droid/gitctx.git`
- **Positioning**: Complementary to current-state tools (like codanna). Where codanna models *what* the code looks like now, `gitctx` provides *why* it changed over time, who changed it, and what historical coupling risks exist.
- **Reference Repositories Policy**: Vetted reference implementations are cloned in `reference_repos/` (via `cargo xtask clone-references`) covering `codanna` (AST parsing, Tantivy, SQLite, FastEmbed, MCP), `code-maat` (co-change coupling formulas), `pydriller` (Git mining and trailer extraction), and `tree-sitter-python` (grammar definitions). Planners and executors inspect and adapt these implementations to avoid reinventing the wheel.
- **Core Thesis Elements**:
  1. Deterministic template-rendered pseudo-documents bridge the cross-modal gap between natural-language queries and unified diff syntax.
  2. Function-level lineage tracking across renames and moves produces complete, deterministic chronological history chains.
  3. Methodologically sound evaluation with non-circular ground truth (linked issues, reverts, manual why-set) and stated baselines (`git log -S`, BM25 alone, `code-maat`).

## Constraints

- **Tech Stack**: Rust (workspace: `gitctx-core`, `gitctx-index`, `gitctx-query`, `gitctx-cli`, `gitctx-mcp`, `gitctx-eval`, `xtask`), SQLite (`rusqlite`), Tantivy, `fastembed-rs`, `tree-sitter`, `petgraph`, `rmcp`.
- **Target OS**: Windows, Linux, macOS.
- **Performance Budgets**:
  - Cold index 10k commits: $< 10$ minutes
  - Incremental index 1 commit: $< 2$ seconds
  - Search p95 latency: $< 300$ ms
  - MCP round-trip: $< 500$ ms
  - Database size: $< 100$ MB per 10k commits
  - Peak RSS during index: $< 2$ GB

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Structured Output Pivot (No LLM in indexing/retrieval) | Eliminates API cost/network dependency, enables deterministic reproducible IR evaluations, keeps retrieval local and fast. | ✓ Good |
| Template-rendered pseudo-documents for diffs | Raw diff hunks are syntax-heavy; templates create natural-language-shaped text with signatures, identifiers, and docstrings. | — Pending |
| Multi-tier MinHash Lineage Engine | Solves function identity across file renames, moves, and signature modifications. | — Pending |
| Synthetic Mutation Testing Oracle (M1-M10) | `git log -L` is blind to cross-file moves and syntax changes; synthetic fixtures provide true ground truth per transformation class. | — Pending |
| First-parent traversal as default | Avoids diff duplication and co-change double-counting across branch merges. | — Pending |
| Commit-size filtering ($\le 25$ files) & normalized co-change | Prevents reformat storms (e.g. `black`) from corrupting logical coupling statistics. | — Pending |
| Stdio MCP server with compact/full token budgeting | Prevents LLM context blowout when returning code histories. | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-08-16 after initialization*

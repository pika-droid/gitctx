# Requirements: gitctx

**Defined:** 2026-08-16
**Core Value:** Providing LLMs and developers with deterministic, citation-backed Git history facts (why code changed, function evolutionary lineages, and change impact) with zero network or LLM dependencies on the indexing and retrieval paths.

## v1 Requirements

### Invariant & Scaffolding (INVAR)

- [x] **INVAR-01**: Rust workspace initialized (`gitctx-core`, `gitctx-index`, `gitctx-query`, `gitctx-cli`, `gitctx-mcp`, `gitctx-eval`, `xtask`) with CI enforcing `fmt`, `clippy -D warnings`, and fast unit tests.
- [x] **INVAR-02**: Offline enforcement (Invariant I1) verified via network-isolated test harness.
- [x] **INVAR-03**: Configuration hashing mechanism (`config_hash`) computed across schema, model, tokenizer, and params to reject mismatched indexes.
- [ ] **INVAR-04**: Double-indexing determinism test (Invariant I2) asserting byte-identical SQLite/vector dumps and identical top-10 search results.
- [ ] **INVAR-05**: Incremental vs. full index equivalence test (Invariant I3) asserting identical database states for matching commit ranges.

### Synthetic Fixtures (FIXT)

- [x] **FIXT-01**: Synthetic repository generator (`tests/fixtures/mkrepo.rs`) producing isolated Git histories (linear commits, merges, evil merges, squash-merges, renames, moves, syntax errors).
- [x] **FIXT-02**: Mutation testing harness (`gitctx-eval mutate`) implementing transformation classes M1-M10 (in-place rename, cross-file move, simultaneous move+rename, signature change, reformatting, local var rename, function split, decoy delete+add, directory move, function copy).

### Corpus Selection & Ground Truth (CORP)

- [x] **CORP-01**: Scripted multi-language repository analyzer computing traceable issue densities, merge fractions, churn, and rename density across diverse languages (Python, Rust, TypeScript/JavaScript, Go, etc.).
- [x] **CORP-02**: Pinned evaluation corpus locked by exact commit SHAs in `eval/corpus.lock.toml` (Invariant I6).
- [x] **CORP-03**: Ground truth harvester (`eval/harvest.py`) extracting issue-closing commits (Set A) and revert pairs with manual audit measuring harvester precision.
- [x] **CORP-04**: Leakage sanitization pipeline stripping issue numbers, PR trailers (`Fixes #N`, `Closes #N`), URLs, and squash prefixes from indexed chunk text.
- [x] **CORP-05**: Hand-authored why-shaped evaluation set (Set B, 25-30 queries) curated from issue/PR discussions prior to inspection of tool outputs.

### Thesis Validation & Ablation (THESIS)

- [ ] **THESIS-01**: Standalone Python evaluation prototype implementing document representations D1-D5 (message only, raw diff, message+raw diff, templated pseudo-doc, templated without message).
- [ ] **THESIS-02**: Evaluation grid across retrievers R1-R5 (keyword `git log -S`, BM25, MiniLM dense, code-aware dense, RRF hybrid).
- [ ] **THESIS-03**: Statistical evaluation harness computing Recall@5/10/20, MRR, paired bootstrap 95% CIs, and adjudicated-miss rates.
- [ ] **THESIS-04**: Formal Phase 0 decision gate executed to validate $\Delta\text{Recall@10} \ge 0.05$ over baselines before committing to retrieval architecture.

### Git Extraction & Parsing (PARSE)

- [ ] **PARSE-01**: `libgit2` / `git2` walker implementing first-parent traversal and diff extraction with Git rename detection enabled.
- [ ] **PARSE-02**: `tree-sitter` historical blob parser with multi-language grammar registry (Python, Rust, JavaScript, TypeScript, Go, C/C++, Java, PHP, etc., leveraging `codanna`'s parser patterns) extracting function definitions, signatures, docstrings, and identifiers as of each historical commit.
- [ ] **PARSE-03**: Graceful degradation handler emitting file-level chunks with `parse_status = 'degraded'` on syntax errors without crashing indexer.
- [ ] **PARSE-04**: Blob-level OID parse cache to accelerate historical parsing across recurring unchanged blobs.
- [ ] **PARSE-05**: Deterministic template generator producing structured pseudo-documents per (commit, function) touch across all supported languages.
- [ ] **PARSE-06**: Defense-in-depth secret redaction replacing high-entropy strings and recognized API key prefixes in chunks and indexes.

### Function Lineage Engine (LINEAGE)

- [ ] **LINEAGE-01**: Formal qualified name convention for module-level functions, class methods, trait/impl blocks, nested functions, closures, decorated functions, and async definitions across supported programming languages.
- [ ] **LINEAGE-02**: Multi-stage lineage resolver per commit (1. Exact key match, 2. Same-file MinHash fingerprint, 3. Git file rename mapping, 4. Scoped cross-file MinHash search).
- [ ] **LINEAGE-03**: Ambiguity and conflict resolution (maximum-weight bipartite matching, function split inheritance with `derived_from` edges, decoy rejection).
- [ ] **LINEAGE-04**: Lineage oracle verification measuring precision and recall across M1-M10 synthetic transformation classes.
- [ ] **LINEAGE-05**: Agreement evaluation against `git log -L :func:file` heuristic oracle with quantified blindness reporting.

### Storage & Hybrid Retrieval (INDEX)

- [x] **INDEX-01**: SQLite schema with WAL mode, single writer channel, atomic `.tmp` indexing, explicit topological order index, and `schema_version` validation.
- [x] **INDEX-02**: Physical isolation of `explain_cache` table to guarantee no LLM text enters embedding inputs (Invariant I5).
- [ ] **INDEX-03**: Code-aware Tantivy BM25 index with camelCase/snake_case tokenization and identifier preservation.
- [ ] **INDEX-04**: Dense vector storage using `fastembed-rs` (MiniLM / code model) written to memory-mapped `f32` arrays with SIMD flat cosine scoring.
- [ ] **INDEX-05**: Reciprocal Rank Fusion (RRF $k=60$) combining BM25 and dense scores with exact symbol match priority path.
- [ ] **INDEX-06**: Deterministic chain assembly querying complete historical timelines by `lineage_id` in a single indexed query.
- [ ] **INDEX-07**: Citation payload inclusion on all retrieved facts (`commit_hash`, `path`, `start_line`, `end_line`, `match_rule`, `match_confidence`) (Invariant I4).

### Co-Change & Impact Analysis (GRAPH)

- [ ] **GRAPH-01**: Co-change extractor filtering out reformat commits touching $>25$ files and bot commits.
- [ ] **GRAPH-02**: Normalized association metrics computation (support, confidence $P(B|A)$, lift) with exponential time-decay half-life weighting.
- [ ] **GRAPH-03**: Operational fix classification linking commits to bug issues or revert pairs to compute `followup_fix_count`.
- [ ] **GRAPH-04**: In-memory `petgraph` multigraph constructed lazily from SQLite for temporal traversals.
- [ ] **GRAPH-05**: `impact` command evaluating risk, coupled files, historical fix frequency, and top authors against `code-maat` baseline.

### CLI & Formatter (CLI)

- [ ] **CLI-01**: Core CLI commands: `gitctx index`, `search`, `evolution`, `show`, `impact`, `blame`, `doctor`.
- [ ] **CLI-02**: Structured `--json` output envelopes with schema version, config hash, warnings, and result citations.
- [ ] **CLI-03**: Rich `--format table` terminal rendering with chronological evolution timelines, diff stats, pager integration, and `NO_COLOR` support.
- [ ] **CLI-04**: `gitctx doctor` diagnostic tool checking index freshness, config hashes, vector consistency, and parse status distributions.

### MCP Server (MCP)

- [ ] **MCP-01**: Stdio Model Context Protocol server exposing `search_history`, `function_evolution`, `change_impact`, `who_knows_this`, and `commit_detail`.
- [ ] **MCP-02**: Dual-mode token budgeting (`compact` citations by default, `full` diffs on demand) to protect LLM context windows.
- [ ] **MCP-03**: Strict stdout JSON-RPC isolation directing all internal logging and diagnostics to stderr.
- [ ] **MCP-04**: Stale index warning mechanism alerting agents when HEAD has advanced past `head_at_index`.
- [ ] **MCP-05**: `gitctx init` command generating idempotent AGENTS.md instructions and skill profile files.

### Evaluation & Defense Suite (EVAL)

- [ ] **EVAL-01**: Automated evaluation runner `cargo xtask eval` producing comprehensive, reproducible `eval/REPORT.md`.
- [ ] **EVAL-02**: Pre-registered failure taxonomy categorizing retrieval misses (vocabulary mismatch, granularity, lineage break, uninformative commit, label incomplete).
- [ ] **EVAL-03**: Automated performance benchmark suite validating cold index (<10m/10k commits), incremental (<2s), search latency (<300ms p95), and MCP round-trip (<500ms).
- [ ] **EVAL-04**: Final documentation suite (README with positioning, architecture diagram, asciinema walkthrough, non-goals section, reproduction guide).

## v2 Requirements

### Extended Code Intelligence (Deferred / Slip-Absorber)

- **V2-CALLS**: Intra-file and explicit same-package call graph edges stored with temporal validity (`valid_from_commit`).
- **V2-SYMBOLS**: `symbols find` and `symbols callers` commands querying symbol call graphs.
- **V2-MMAP**: Dedicated memory-mapped symbol cache for low-latency symbol lookup if SQLite benchmark indicates necessity.
- **V2-EXPLAIN**: Opt-in LLM summary generation via `gitctx <cmd> --explain` storing cached prompts in isolated `explain_cache`.
- **V2-WATCH**: Background filesystem / Git HEAD watcher with incremental index trigger.
- **V2-NPX**: Cross-platform precompiled binary distribution wrapped via npm.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Network/LLM during indexing or search | Violates Invariant I1; destroys reproducibility, offline guarantees, and speed |
| Whole-AST dynamic Python resolution | Dynamic dispatch/monkeypatching is intractable without full runtime analysis; causes false certainty |
| Security vulnerability/secret detection claims | Scanner is scoped solely as defense-in-depth output redaction, not security auditing |
| Real-time file system daemon in v1 | High complexity in cache invalidation and concurrent access; deferred to v2 |
| Cross-repository federation | Single git repository scope in v1 |
| Unstructured Document RAG | Diverges from git history intelligence mission; codanna / standard RAG covers current state |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| INVAR-01 | Phase 1 | Complete |
| INVAR-02 | Phase 1 | Complete |
| FIXT-01 | Phase 1 | Complete |
| FIXT-02 | Phase 1 | Complete |
| CORP-01 | Phase 2 | Complete |
| CORP-02 | Phase 2 | Complete |
| CORP-03 | Phase 2 | Complete |
| CORP-04 | Phase 2 | Complete |
| CORP-05 | Phase 2 | Complete |
| THESIS-01 | Phase 3 | Pending |
| THESIS-02 | Phase 3 | Pending |
| THESIS-03 | Phase 3 | Pending |
| THESIS-04 | Phase 3 | Pending |
| PARSE-01 | Phase 4 | Pending |
| PARSE-02 | Phase 4 | Pending |
| PARSE-03 | Phase 4 | Pending |
| PARSE-04 | Phase 4 | Pending |
| PARSE-05 | Phase 4 | Pending |
| PARSE-06 | Phase 4 | Pending |
| LINEAGE-01 | Phase 5 | Pending |
| LINEAGE-02 | Phase 5 | Pending |
| LINEAGE-03 | Phase 5 | Pending |
| LINEAGE-04 | Phase 5 | Pending |
| LINEAGE-05 | Phase 5 | Pending |
| INVAR-03 | Phase 6 | Complete |
| INVAR-04 | Phase 6 | Pending |
| INVAR-05 | Phase 6 | Pending |
| INDEX-01 | Phase 6 | Complete |
| INDEX-02 | Phase 6 | Complete |
| INDEX-03 | Phase 6 | Pending |
| INDEX-04 | Phase 6 | Pending |
| INDEX-05 | Phase 6 | Pending |
| INDEX-06 | Phase 6 | Pending |
| INDEX-07 | Phase 6 | Pending |
| GRAPH-01 | Phase 7 | Pending |
| GRAPH-02 | Phase 7 | Pending |
| GRAPH-03 | Phase 7 | Pending |
| GRAPH-04 | Phase 7 | Pending |
| GRAPH-05 | Phase 7 | Pending |
| CLI-01 | Phase 8 | Pending |
| CLI-02 | Phase 8 | Pending |
| CLI-03 | Phase 8 | Pending |
| CLI-04 | Phase 8 | Pending |
| MCP-01 | Phase 9 | Pending |
| MCP-02 | Phase 9 | Pending |
| MCP-03 | Phase 9 | Pending |
| MCP-04 | Phase 9 | Pending |
| MCP-05 | Phase 9 | Pending |
| EVAL-01 | Phase 10 | Pending |
| EVAL-02 | Phase 10 | Pending |
| EVAL-03 | Phase 10 | Pending |
| EVAL-04 | Phase 11 | Pending |

**Coverage:**

- v1 requirements: 48 total
- Mapped to phases: 48
- Unmapped: 0 ✓

---
*Requirements defined: 2026-08-16*
*Last updated: 2026-08-16 after initial definition*

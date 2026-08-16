# Roadmap: gitctx

## Overview

`gitctx` is constructed systematically in 11 structured phases (plus a deferred Phase 12 slip-absorber), strictly prioritizing thesis verification and methodological rigor. We first build synthetic fixture tools and non-circular ground truth pipelines, execute a Python ablation study to validate the retrieval thesis, construct the Rust extraction and MinHash lineage engines, build deterministic hybrid storage (SQLite + Tantivy + dense embeddings with RRF), implement statistical co-change coupling, expose polished CLI and MCP agent surfaces, and produce an exhaustive, reproducible evaluation and defense report.

### Reference Codebases & Pattern Reuse
To avoid reinventing the wheel and ensure architectural alignment, all phases leverage local reference implementations in `reference_repos/` (synchronized via `cargo xtask clone-references`):
- **`reference_repos/codanna`**: Patterns for `libgit2` walkers, `tree-sitter` AST parsers, Tantivy BM25 schema, SQLite connection pooling, FastEmbed vector scoring, and Stdio MCP servers.
- **`reference_repos/code-maat`**: Mathematical specifications for VCS co-change coupling ($P(B|A)$, support, lift), churn, and impact analysis.
- **`reference_repos/pydriller`**: Python Git mining, commit iteration, diff analysis, and issue-closing trailer parsing.
- **`reference_repos/tree-sitter-python`**: Official Python grammar AST node types and query patterns.

## Phases

- [x] **Phase 1: Project Foundation, Invariants & Fixture Scaffolding** - Initialize Rust workspace, establish invariant enforcement harness, and build synthetic git fixture and mutation generators. (completed 2026-08-16)
- [ ] **Phase 2: Corpus Selection, Ground Truth Harvesting & Leakage Sanitization** - Scripted repository profiling, SHA-locked corpus selection, issue-closing ground truth harvester, and leakage stripper.
- [ ] **Phase 3: Retrieval Thesis Validation (Phase 0 Python Ablation)** - Run throwaway Python ablation grid testing document representations (D1-D5) and retrievers (R1-R5) to validate retrieval quality and decision gate.
- [ ] **Phase 4: Rust Git Extraction & Tree-Sitter Historical Parser** - Build `libgit2` first-parent commit walker with rename detection, historical `tree-sitter` parser with graceful degradation, and template chunk renderer.
- [ ] **Phase 5: Function Lineage Engine & Mutation Oracle** - Implement multi-tier MinHash lineage resolution across renames/moves, validated against synthetic mutation suite (M1-M10) and `git log -L`.
- [ ] **Phase 6: Deterministic Storage & Hybrid Retrieval Engine** - SQLite database layer, Tantivy BM25, mmap f32 dense vectors, RRF fusion ($k=60$), exact symbol fast-path, and deterministic chain assembly.
- [ ] **Phase 7: Co-Change Graph & Impact Analysis Engine** - Commit-size filtering ($\le 25$), normalized co-change metrics (support, confidence, lift), time-decay weighting, and `impact` risk scoring against `code-maat`.
- [ ] **Phase 8: CLI Interface & Human Output Formatter** - Build core CLI commands (`index`, `search`, `evolution`, `show`, `impact`, `doctor`) with `--json` envelopes and `--format table` terminal timelines.
- [ ] **Phase 9: Model Context Protocol (MCP) Server Surface** - Stdio MCP server with compact/full token budgeting, JSON-RPC stdout isolation, stale-index detection, and `gitctx init` skill generators.
- [ ] **Phase 10: Cross-Cutting Verification, Determinism & Performance Benchmarking** - Verify double-indexing byte-identity, incremental index equivalence, offline execution, and pass/fail performance budgets.
- [ ] **Phase 11: Comprehensive Evaluation Report, Failure Taxonomy & Defense Artifacts** - Generate `eval/REPORT.md` via `cargo xtask eval`, classify misses using pre-registered failure taxonomy, and compile README with architecture diagrams.
- [ ] **Phase 12: Current-State Extensions (Deferred / Slip-Absorber)** - Secondary language grammars, temporal call graph edges, symbol lookup commands, and optional `--explain` cache.

## Phase Details

### Phase 1: Project Foundation, Invariants & Fixture Scaffolding

**Goal**: Establish the Rust workspace, CI checks, invariant enforcement harnesses, and synthetic git repo/mutation generators to enable rapid offline testing.
**Depends on**: Nothing (first phase)
**Requirements**: INVAR-01, INVAR-02, FIXT-01, FIXT-02
**Success Criteria** (what must be TRUE):

  1. Rust workspace (`gitctx-core`, `gitctx-index`, `gitctx-query`, `gitctx-cli`, `gitctx-mcp`, `gitctx-eval`, `xtask`) compiles cleanly under `cargo check` and `cargo test` in $<10$s.
  2. Offline test runner confirms zero network requests are made during test execution.
  3. `tests/fixtures/mkrepo.rs` programmatically generates miniature Git repos for linear commits, file renames, cross-file moves, squash merges, conflict merges, and syntax errors.
  4. Mutation test generator can apply transformations M1-M10 to benchmark lineage tracking.

**Plans**: 3/3 plans executed

Plans:

- [x] 01-01-PLAN.md
- [x] 01-02-PLAN.md
- [x] 01-03-PLAN.md

**Wave 1**

- [x] 01-01: Cargo workspace layout, crate definitions, CI configuration (`fmt`, `clippy -D warnings`), and `xtask` scaffolding.

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 01-02: Synthetic Git fixture repository builder (`mkrepo.rs`) covering edge-case commit histories and syntax failures.

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 01-03: Synthetic mutation generator (`gitctx-eval mutate`) implementing transformation classes M1-M10 and invariant verification harness.

### Phase 2: Corpus Selection, Ground Truth Harvesting & Leakage Sanitization

**Goal**: Select and pin real-world evaluation repositories across diverse languages (Python, Rust, TypeScript, Go), harvest issue-closing ground truth with leakage stripping, and curate why-shaped queries.
**Depends on**: Phase 1
**Requirements**: CORP-01, CORP-02, CORP-03, CORP-04, CORP-05
**Reference Repos**: `reference_repos/pydriller` (`pydriller/repository.py`, `pydriller/domain/commit.py` for issue/trailer parsing, `pydriller/domain/modification.py` for diff classification)
**Success Criteria** (what must be TRUE):

  1. Scripted repository selection profiles $\ge 15$ multi-language repos, selecting $\ge 3$ diverse target repos (covering Python, Rust, and TypeScript/Go) with $\ge 150$ traceable issues and $<40\%$ merge fraction.
  2. Selected repositories and exact commit SHAs are locked in `eval/corpus.lock.toml`.
  3. Ground truth harvester (`eval/harvest.py`) extracts issue-closing commit pairs (Set A) and revert pairs with audited precision $\ge 0.85$.
  4. Leakage stripping pipeline verifiably removes issue numbers, `Fixes #N` trailers, PR URLs, and squash suffixes from chunk text across languages.
  5. 25-30 why-shaped queries (Set B) are authored from issue/PR discussions prior to inspecting search outputs.

**Plans**: 3 plans

Plans:

- [ ] 02-01: Scripted repository profiler and corpus selection criteria documentation (`eval/corpus.lock.toml`).
- [ ] 02-02: Issue-closing and revert-pair ground truth harvester (`eval/harvest.py`) with manual precision audit logging.
- [ ] 02-03: Leakage sanitization module and Set B why-query curation protocol.

### Phase 3: Retrieval Thesis Validation (Phase 0 Python Ablation)

**Goal**: Validate the core thesis in disposable Python: confirm that deterministic template-rendered diffs outperform BM25 and raw diffs before investing in Rust retrieval.
**Depends on**: Phase 2
**Requirements**: THESIS-01, THESIS-02, THESIS-03, THESIS-04
**Reference Repos**: `reference_repos/pydriller` (commit dataset processing), `reference_repos/codanna/src/indexing/` (Tantivy/BM25 comparison baseline)
**Success Criteria** (what must be TRUE):

  1. Full ablation grid running D1-D5 representations across R1-R5 retrievers against Set A and Set B queries.
  2. Max-pooling over function-touches with duplicate commit deduplication applied across all metric computations.
  3. Statistical harness reports Recall@5/10/20, MRR, paired bootstrap 95% CIs, and top-5 adjudicated miss rates per repo and pooled.
  4. Pre-registered decision gate evaluates $\Delta\text{Recall@10} \ge 0.05$ (CI excluding 0) to officially validate templated pseudo-documents.

**Plans**: 2 plans

Plans:

- [ ] 03-01: Standalone Python ablation framework implementing representations D1-D5, retrievers R1-R5, and max-pooled evaluation metrics.
- [ ] 03-02: Evaluation execution, bootstrap confidence interval analysis, adjudicated miss auditing, and Phase 0 gate decision report.

### Phase 4: Rust Git Extraction & Tree-Sitter Historical Parser

**Goal**: Build the core extraction engine in Rust: `git2` first-parent commit walker with rename tracking, multi-language `tree-sitter` parser registry (Python, Rust, JavaScript, TypeScript, Go, C/C++, Java, PHP), and templated pseudo-doc generation.
**Depends on**: Phase 3
**Requirements**: PARSE-01, PARSE-02, PARSE-03, PARSE-04, PARSE-05, PARSE-06
**Reference Repos**: `reference_repos/codanna/src/git.rs` (libgit2 repo access, ignore matching), `reference_repos/codanna/src/parsing/` (multi-language parser registry, AST traversal, behavior definitions), `reference_repos/tree-sitter-python/` (grammar & AST node types)
**Success Criteria** (what must be TRUE):

  1. `git2` extractor correctly walks first-parent commit graph, tracks file insertions/deletions, and detects renames across all file types.
  2. Multi-language `tree-sitter` registry parses historical blobs across Python, Rust, JavaScript, TypeScript, Go, C/C++, Java, and PHP, extracting function/method signatures, docstrings, and identifier deltas.
  3. Syntactically broken commits emit `parse_status = 'degraded'` file chunks without panicking or halting the indexer.
  4. Blob OID parse cache eliminates redundant AST parses across unchanged historical blobs.
  5. Deterministic chunk templater formats pseudo-documents matching the Phase 0 specification with defense-in-depth secret redaction.

**Plans**: 3 plans

Plans:

- [ ] 04-01: `gitctx-core` Git walker, topological commit ordering, first-parent filter, and diff/rename extractor.
- [ ] 04-02: Historical blob AST parser using `tree-sitter` with OID cache and graceful degradation error handling.
- [ ] 04-03: Templated pseudo-document generator, identifier delta extractor, and secret redaction filter.

### Phase 5: Function Lineage Engine & Mutation Oracle

**Goal**: Implement the multi-tier function identity resolution engine across renames and moves, proven against synthetic mutation testing and `git log -L`.
**Depends on**: Phase 4
**Requirements**: LINEAGE-01, LINEAGE-02, LINEAGE-03, LINEAGE-04, LINEAGE-05
**Reference Repos**: `reference_repos/codanna/src/symbol/` (qualified naming conventions, symbol hashing), `reference_repos/codanna/src/relationship/` (graph relationship mapping)
**Success Criteria** (what must be TRUE):

  1. Standardized qualified name rules implemented for module functions, class methods, nested functions, closures, and decorators.
  2. 4-tier lineage matcher (exact key $\to$ same-file MinHash $\to$ git rename mapping $\to$ scoped cross-file MinHash) assigns stable `lineage_id`s.
  3. Conflict resolution handles function splits (`derived_from` edges), merges, and rejects decoy delete+add false positives.
  4. Synthetic mutation evaluation achieves quantified precision/recall per transformation class M1-M10 with threshold $\tau$ tuned on held-out split.
  5. Cross-checking against `git log -L` runs with quantified reporting of cases the heuristic oracle cannot express.

**Plans**: 3 plans

Plans:

- [ ] 05-01: Qualified name convention engine and normalized body MinHash fingerprint generator.
- [ ] 05-02: Multi-stage lineage resolver with bipartite matching, split derivation, and uniqueness constraints.
- [ ] 05-03: Lineage evaluation harness against synthetic mutation classes (M1-M10) and `git log -L` oracle.

### Phase 6: Deterministic Storage & Hybrid Retrieval Engine

**Goal**: Build the unified storage and retrieval engine in Rust combining SQLite, Tantivy BM25, memory-mapped dense vectors, and RRF fusion.
**Depends on**: Phase 5
**Requirements**: INVAR-03, INVAR-04, INVAR-05, INDEX-01, INDEX-02, INDEX-03, INDEX-04, INDEX-05, INDEX-06, INDEX-07
**Reference Repos**: `reference_repos/codanna/src/storage/` (SQLite WAL mode connection management & PRAGMAs), `reference_repos/codanna/src/indexing/` (Tantivy schema & custom tokenizers), `reference_repos/codanna/src/vector/` & `src/semantic/` (FastEmbed dense vector scoring & hybrid fusion)
**Success Criteria** (what must be TRUE):

  1. SQLite schema with WAL mode, single writer channel, topological commit ordering, and isolated `explain_cache` table.
  2. Tantivy index with camelCase/snake_case tokenization and boosted symbol indexing.
  3. `fastembed-rs` dense embeddings written to memory-mapped `f32` vectors scored via flat SIMD cosine similarity.
  4. RRF fusion ($k=60$) seamlessly combines BM25 and dense scores with exact symbol fast-path lookup.
  5. Deterministic chain assembly reconstructs complete evolutionary history in a single indexed query by `lineage_id`.
  6. All search results return structured citation metadata (`commit_hash`, `path`, `start_line`, `end_line`, `match_rule`, `match_confidence`).

**Plans**: 4 plans

Plans:

- [ ] 06-01: SQLite storage engine (`gitctx-index`), schema migrations, WAL mode writer channel, and atomic database replacement.
- [ ] 06-02: Code-aware Tantivy BM25 indexing pipeline with custom tokenizers.
- [ ] 06-03: Dense embedding generation (`fastembed-rs`), memory-mapped vector file layout, and SIMD flat cosine search.
- [ ] 06-04: RRF hybrid ranker, exact symbol fast-path, and deterministic lineage chain assembler.

### Phase 7: Co-Change Graph & Impact Analysis Engine

**Goal**: Build statistical file and function coupling analysis with commit size filtering, normalization, and time-decay weighting.
**Depends on**: Phase 6
**Requirements**: GRAPH-01, GRAPH-02, GRAPH-03, GRAPH-04, GRAPH-05
**Reference Repos**: `reference_repos/code-maat/src/code_maat/analysis/` (`logical_coupling.clj` coupling formulas $P(B|A)$, `coupling_algos.clj`, `churn.clj`)
**Success Criteria** (what must be TRUE):

  1. Co-change accounting filters out reformat commits ($>25$ files) and bots, logging excluded commit metrics.
  2. Normalized coupling metrics (support, confidence $P(B|A)$ / $P(A|B)$, lift) computed with exponential time-decay half-life.
  3. Operational fix classification computes `followup_fix_count` for coupled files.
  4. In-memory `petgraph` multigraph supports efficient temporal and structural traversals.
  5. `gitctx impact` output benchmarked against established `code-maat` tool baseline across temporal holdout splits.

**Plans**: 2 plans

Plans:

- [ ] 07-01: Co-change extraction engine with commit-size filtering, time-decay weighting, and normalized association metrics.
- [ ] 07-02: `petgraph` in-memory graph construction, `impact` risk scoring, and `code-maat` baseline validation.

### Phase 8: CLI Interface & Human Output Formatter

**Goal**: Implement the user-facing CLI commands with structured `--json` envelopes, rich `--format table` terminal rendering, and diagnostic tooling.
**Depends on**: Phase 7
**Requirements**: CLI-01, CLI-02, CLI-03, CLI-04
**Reference Repos**: `reference_repos/codanna/src/cli/` (Clap command hierarchy, colored console rendering, formatted output)
**Success Criteria** (what must be TRUE):

  1. CLI commands `gitctx index`, `search`, `evolution`, `show`, `impact`, `blame`, and `doctor` operational.
  2. All commands support versioned, typed `--json` output matching the schema specification.
  3. Table formatter produces clean chronological evolution timelines, diff summaries, and respects TTY/`NO_COLOR`/pager settings.
  4. `gitctx doctor` successfully diagnoses index health, schema version, config hash match, and parse status distributions.

**Plans**: 2 plans

Plans:

- [ ] 08-01: CLI command parser (`gitctx-cli`), argument validation, and structured `--json` envelope formatting.
- [ ] 08-02: Human-readable terminal formatters (`--format table`, timelines, diffs) and `gitctx doctor` diagnostic tool.

### Phase 9: Model Context Protocol (MCP) Server Surface

**Goal**: Build the stdio MCP server for agent integration with token budgeting, stdout isolation, and automated initialization skills.
**Depends on**: Phase 8
**Requirements**: MCP-01, MCP-02, MCP-03, MCP-04, MCP-05
**Reference Repos**: `reference_repos/codanna/src/mcp/` (Stdio JSON-RPC protocol implementation, MCP tool schemas, stdout isolation)
**Success Criteria** (what must be TRUE):

  1. Stdio MCP server exposes `search_history`, `function_evolution`, `change_impact`, `who_knows_this`, and `commit_detail`.
  2. MCP return types share identical serde data structures with `--json` CLI output.
  3. Token budgeting supports `compact` citation-only mode (default) and `full` diff mode.
  4. Stdout remains 100% clean JSON-RPC, with all internal logging routed to stderr.
  5. `gitctx init` generates idempotent AGENTS.md sections and skill profiles with negative usage guidance.

**Plans**: 2 plans

Plans:

- [ ] 09-01: `gitctx-mcp` stdio server implementation using `rmcp` with dual `compact`/`full` token budgeting and stdout isolation.
- [ ] 09-02: `gitctx init` skill profile generator, AGENTS.md configuration, and MCP inspector test suite.

### Phase 10: Cross-Cutting Verification, Determinism & Performance Benchmarking

**Goal**: Run end-to-end invariant validation, double-indexing byte-identity tests, incremental equivalence tests, and pass/fail performance benchmarks.
**Depends on**: Phase 9
**Requirements**: EVAL-01, EVAL-02, EVAL-03
**Success Criteria** (what must be TRUE):

  1. Double-indexing test confirms byte-identical SQLite and vector dumps for identical repository inputs (Invariant I2).
  2. Incremental indexing of commits $N-10\dots N$ matches full indexing state byte-for-byte (Invariant I3).
  3. Cold index on 10k commits finishes in $<10$ minutes; incremental commit indexing finishes in $<2$ seconds.
  4. Search latency p95 is $<300$ ms and MCP round-trip is $<500$ ms.

**Plans**: 2 plans

Plans:

- [ ] 10-01: End-to-end invariant validation suite (double-indexing determinism, incremental equivalence, network denial).
- [ ] 10-02: Automated performance benchmark harness executing against stated latency, memory, and database size budgets.

### Phase 11: Comprehensive Evaluation Report, Failure Taxonomy & Defense Artifacts

**Goal**: Produce the reproducible evaluation report (`eval/REPORT.md`), classify misses via pre-registered failure taxonomy, and compile complete defense documentation.
**Depends on**: Phase 10
**Requirements**: EVAL-04
**Success Criteria** (what must be TRUE):

  1. `cargo xtask eval` reproduces full evaluation grid, lineage precision/recall, and impact metrics directly from pinned corpus.
  2. Pre-registered failure taxonomy categorizes all retrieval misses into explicit failure buckets.
  3. Complete README published with positioning statement, architecture diagrams, asciinema walkthrough demo, and non-goals section.

**Plans**: 2 plans

Plans:

- [ ] 11-01: `cargo xtask eval` report generator producing `eval/REPORT.md` with ablation tables, bootstrap CIs, and failure taxonomy breakdown.
- [ ] 11-02: Project README, architectural documentation, defense slides/diagrams, and reproducibility walkthrough.

### Phase 12: Current-State Extensions (Deferred / Slip-Absorber)

**Goal**: Optional v2 extensions: temporal call graph edges, symbol lookup commands, and cached LLM explanation layer.
**Depends on**: Phase 11
**Requirements**: V2-CALLS, V2-SYMBOLS, V2-MMAP, V2-EXPLAIN, V2-WATCH, V2-NPX
**Success Criteria** (what must be TRUE):

  1. Temporal call graph edges recorded with intra-file / same-package resolution precision reported.
  2. `symbols find` and `symbols callers` CLI/MCP tools operational.
  3. Opt-in `--explain` generates cached summaries in physically isolated `explain_cache`.

**Plans**: TBD

## Progress

**Execution Order:**
Phases execute sequentially in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10 → 11 (→ 12)

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Project Foundation, Invariants & Fixtures | 3/3 | Complete    | 2026-08-16 |
| 2. Corpus Selection & Ground Truth | 0/3 | Not started | - |
| 3. Retrieval Thesis Validation (Phase 0) | 0/2 | Not started | - |
| 4. Rust Git Extraction & Parser | 0/3 | Not started | - |
| 5. Function Lineage Engine | 0/3 | Not started | - |
| 6. Storage & Hybrid Retrieval | 0/4 | Not started | - |
| 7. Co-Change Graph & Impact | 0/2 | Not started | - |
| 8. CLI Interface & Formatter | 0/2 | Not started | - |
| 9. Model Context Protocol (MCP) | 0/2 | Not started | - |
| 10. Verification & Benchmarking | 0/2 | Not started | - |
| 11. Evaluation Report & Defense | 0/2 | Not started | - |
| 12. Current-State Extensions (Deferred) | 0/0 | Deferred | - |

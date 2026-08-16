# Reference Repositories & Reuse Catalog

This directory holds local clones of vetted reference codebases and algorithms. Use these reference implementations to inspect, adapt, reuse, and improve existing solutions across `gitctx` phases rather than reinventing the wheel.

> **Note:** Cloned repositories in this directory are git-ignored by the root `.gitignore` to keep the main `gitctx` repository lean and clean. To clone or update them at any time, run:
> ```bash
> cargo xtask clone-references
> ```

---

## Cloned Repositories

| Repository | Path | Upstream | Description |
|---|---|---|---|
| **`codanna`** | `reference_repos/codanna/` | `https://github.com/bartolli/codanna.git` | Production Rust-based local code intelligence tool, Tantivy BM25 schema, SQLite storage, tree-sitter AST parser, FastEmbed vector embeddings, and persistent MCP server. |
| **`code-maat`** | `reference_repos/code-maat/` | `https://github.com/adamtornhill/code-maat.git` | Benchmark VCS temporal analysis tool by Adam Tornhill. Defines mathematical formulas and baseline implementations for co-change coupling, churn, and entity ownership. |
| **`pydriller`** | `reference_repos/pydriller/` | `https://github.com/ishepard/pydriller.git` | Mature Python repository mining framework with commit walking, modified file tracking, diff/hunk parsing, and issue/developer extraction. |
| **`tree-sitter-python`** | `reference_repos/tree-sitter-python/` | `https://github.com/tree-sitter/tree-sitter-python.git` | Official Tree-sitter grammar and node definitions for Python ASTs. |

---

## Phase-by-Phase Reference & Code Reuse Map

### Phase 2: Corpus Selection, Ground Truth Harvesting & Leakage Sanitization
- **Reference**: `reference_repos/pydriller/pydriller/`
  - `repository.py`: Git commit iterator and repo filtering logic.
  - `domain/commit.py`: Issue reference parsing, trailer detection (`Fixes #N`), commit metadata extraction.
  - `domain/modification.py`: File-level diffs, added/deleted line classification, rename tracking.
- **Application**: Reuse regexes and commit filtering logic for `eval/harvest.py` and the leakage stripping pipeline.

### Phase 3: Retrieval Thesis Validation (Python Ablation)
- **Reference**: `reference_repos/pydriller/` & `reference_repos/codanna/src/indexing/`
  - Corpus loading, tokenization, and BM25 index comparison.
- **Application**: Ground truth evaluation script structure and metric computation.

### Phase 4: Rust Git Extraction & Tree-Sitter Historical Parser
- **Reference**: `reference_repos/codanna/src/`
  - `git.rs`: `libgit2` repository opening, branch/commit enumeration, ignore list matching (`.gitignore`, `.gitctxignore`).
  - `parsing/`: Tree-sitter parser loading, AST node traversal, docstring extraction, symbol queries across language grammars.
- **Reference**: `reference_repos/tree-sitter-python/`
  - `grammar.js` & `src/node-types.json`: Node types (`function_definition`, `class_definition`, `decorated_definition`, `parameters`, `return_statement`) for AST chunking.
- **Application**: Adapt `git.rs` and `parsing/` patterns in `gitctx-core` for historical blob parsing with error degradation (`ParseStatus::Degraded`).

### Phase 5: Function Lineage Engine & Mutation Oracle
- **Reference**: `reference_repos/codanna/src/`
  - `symbol/`: Qualified name hierarchies (`module::class::method`), symbol representation, symbol hashing.
  - `relationship/`: Graph relationship types between symbols and callers.
- **Application**: Build the 4-tier MinHash lineage resolver in `gitctx-core` / `gitctx-index`, reusing naming conventions and bipartite matching strategies.

### Phase 6: Deterministic Storage & Hybrid Retrieval Engine
- **Reference**: `reference_repos/codanna/src/`
  - `storage/`: SQLite database setup, connection pool configuration, PRAGMA tuning (`journal_mode = WAL`, `synchronous = NORMAL`, `busy_timeout = 5000`), schema migrations.
  - `indexing/`: Tantivy schema builder, custom code analyzers (camelCase/snake_case splitters), inverted index writer, commit/symbol doc fields.
  - `vector/` & `semantic/`: `fastembed-rs` integration, dense vector normalization, SIMD cosine distance calculation, ONNX model caching.
- **Application**: Reuse SQLite connection patterns and Tantivy indexing pipeline directly in `gitctx-index` and `gitctx-query`.

### Phase 7: Co-Change Graph & Impact Analysis Engine
- **Reference**: `reference_repos/code-maat/src/code_maat/analysis/`
  - `logical_coupling.clj`: Mathematical formulas for support count, degree of coupling ($P(A \cap B) / P(A)$), and average revisions.
  - `coupling_algos.clj`: Pairwise entity coupling matrix generation.
  - `churn.clj`: Code churn metrics (lines added/deleted per entity).
- **Application**: Implement co-change coupling formulas in `gitctx-query` and `gitctx-cli` using `petgraph`, benchmarking against `code-maat` outputs for identical Git commits.

### Phase 8: CLI Interface & Human Output Formatter
- **Reference**: `reference_repos/codanna/src/cli/`
  - `clap` command setup, colored terminal table rendering, human-friendly search result formatters.
- **Application**: Reuse console formatting patterns in `gitctx-cli` for chronological evolution timelines.

### Phase 9: Model Context Protocol (MCP) Server Surface
- **Reference**: `reference_repos/codanna/src/mcp/`
  - JSON-RPC 2.0 stdio transport handling, tool schema registration, parameter validation, prompt generation, and stdout/stderr separation.
- **Application**: Build `gitctx-mcp` stdio server on `rmcp` using `codanna`'s proven MCP tool patterns and token budgeting strategies (`compact` vs. `full`).

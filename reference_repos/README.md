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
| **`codanna`** | `reference_repos/codanna/` | `https://github.com/bartolli/codanna.git` | Production Rust-based local code intelligence tool, Tantivy BM25 schema, SQLite storage, Tree-sitter AST parsers for 15 languages, FastEmbed vector embeddings, and persistent MCP server. |
| **`code-maat`** | `reference_repos/code-maat/` | `https://github.com/adamtornhill/code-maat.git` | Benchmark VCS temporal analysis tool by Adam Tornhill. Defines mathematical formulas and baseline implementations for co-change coupling, churn, and entity ownership. |
| **`pydriller`** | `reference_repos/pydriller/` | `https://github.com/ishepard/pydriller.git` | Mature Python repository mining framework with commit walking, modified file tracking, diff/hunk parsing, and issue/developer extraction. |
| **`tree-sitter-python`** | `reference_repos/tree-sitter-python/` | `https://github.com/tree-sitter/tree-sitter-python.git` | Official Tree-sitter grammar and node definitions for Python ASTs. |

---

## Deep-Dive: Tree-Sitter Multi-Language Parser Architecture & Reuse (from `codanna`)

`reference_repos/codanna/src/parsing/` contains production-tested Tree-sitter parsers and AST traversal engines for **15 programming languages**. In Phase 4, `gitctx` will directly adapt and build upon this subsystem rather than writing language parsers from scratch.

### 1. Existing Tree-Sitter Parsers in `codanna`

| Language | `codanna` Source Location | Grammar Crate | Key AST Node Types Extracted |
|---|---|---|---|
| **Python** | `reference_repos/codanna/src/parsing/python/` | `tree-sitter-python` | `function_definition`, `class_definition`, `decorated_definition`, `parameters`, docstrings (`expression_statement > string`) |
| **Rust** | `reference_repos/codanna/src/parsing/rust/` | `tree-sitter-rust` | `function_item`, `impl_item`, `trait_item`, `macro_definition`, `doc_comment` (`///`, `//!`) |
| **JavaScript** | `reference_repos/codanna/src/parsing/javascript/` | `tree-sitter-javascript` | `function_declaration`, `method_definition`, `arrow_function`, `class_declaration`, JSDoc comments |
| **TypeScript** | `reference_repos/codanna/src/parsing/typescript/` | `tree-sitter-typescript` | `function_declaration`, `method_definition`, `interface_declaration`, `type_alias_declaration`, TS decorators |
| **Go** | `reference_repos/codanna/src/parsing/go/` | `tree-sitter-go` | `function_declaration`, `method_declaration`, `type_declaration`, doc comments |
| **C / C++** | `reference_repos/codanna/src/parsing/c/`, `cpp/` | `tree-sitter-c`, `tree-sitter-cpp` | `function_definition`, `declaration`, `class_specifier`, `struct_specifier`, Doxygen comments |
| **C#** | `reference_repos/codanna/src/parsing/csharp/` | `tree-sitter-c-sharp` | `method_declaration`, `class_declaration`, `interface_declaration`, XML doc comments |
| **Java** | `reference_repos/codanna/src/parsing/java/` | `tree-sitter-java` | `method_declaration`, `constructor_declaration`, `class_declaration`, Javadoc comments |
| **Kotlin** | `reference_repos/codanna/src/parsing/kotlin/` | `tree-sitter-kotlin-codanna` | `function_declaration`, `class_declaration`, KDoc comments |
| **PHP** | `reference_repos/codanna/src/parsing/php/` | `tree-sitter-php` | `function_definition`, `method_declaration`, `class_declaration`, PHPDoc comments |
| **Swift** | `reference_repos/codanna/src/parsing/swift/` | `tree-sitter-swift` | `function_declaration`, `class_declaration`, `protocol_declaration`, doc comments |
| **Lua** | `reference_repos/codanna/src/parsing/lua/` | `tree-sitter-lua` | `function_declaration`, `local_function`, EmmyLua doc comments |
| **Clojure** | `reference_repos/codanna/src/parsing/clojure/` | `tree-sitter-clojure-orchard` | `defn`, `defn-`, `defmulti`, `defmethod` forms |
| **GDScript** | `reference_repos/codanna/src/parsing/gdscript/` | `tree-sitter-gdscript` | `function_definition`, `class_definition` |

### 2. Architectural Adaptation for `gitctx`

While `codanna` parses files directly from disk to index the current workspace, `gitctx` adapts this parsing pipeline for **historical version control graphs**:

1. **Blob-Stream Parsing (`git2::Blob`)**:
   Instead of `std::fs::read`, `gitctx` streams bytes from `git2::Blob` into Tree-sitter's parser `parser.parse(blob_bytes, None)`.
2. **Function Touch Emission**:
   Adapt `LanguageParser::parse` to emit `gitctx_core::model::FunctionTouch`:
   - `qualname`: Hierarchical qualified name (`module::class::function`) using `codanna`'s scope resolution.
   - `signature`: Extracted parameter list and return type.
   - `docstring`: Extracted comment/docstring node text.
   - `body_span`: `(start_line, end_line, start_byte, end_byte)`.
   - `body_tokens`: Normalized token stream for MinHash calculation (Phase 5).
3. **Blob OID Deduplication Cache**:
   Wrap the multi-language parser factory with an in-memory or LRU cache keyed by Git blob OID (`git2::Oid`), completely bypassing Tree-sitter parsing for unchanged blobs across historical commits.
4. **Graceful Error Degradation**:
   When Tree-sitter encounters syntax errors (`has_error()`), `gitctx` uses `codanna`'s node recovery logic to extract valid enclosing functions, and tags unparseable files as `ParseStatus::Degraded` rather than failing the index run.

---

## Phase-by-Phase Reference & Code Reuse Map

### Phase 2: Corpus Selection, Ground Truth Harvesting & Leakage Sanitization
- **Reference**: `reference_repos/pydriller/pydriller/`
  - `repository.py`: Git commit iterator and repo filtering logic.
  - `domain/commit.py`: Issue reference parsing, trailer detection (`Fixes #N`), commit metadata extraction.
  - `domain/modification.py`: File-level diffs, added/deleted line classification, rename tracking.
- **Application**: Reuse regexes and commit filtering logic for `eval/harvest.py` and the leakage stripping pipeline across multi-language repos.

### Phase 3: Retrieval Thesis Validation (Phase 0 Ablation)
- **Reference**: `reference_repos/pydriller/` & `reference_repos/codanna/src/indexing/`
  - Corpus loading, tokenization, and BM25 index comparison.
- **Application**: Ground truth evaluation script structure and metric computation.

### Phase 4: Rust Git Extraction & Tree-Sitter Historical Parser
- **Reference**: `reference_repos/codanna/src/`
  - `git.rs`: `libgit2` repository opening, branch/commit enumeration, ignore list matching (`.gitignore`, `.gitctxignore`).
  - `parsing/mod.rs`, `factory.rs`, `registry.rs`, `language.rs`, `language_behavior.rs`: Multi-language parser factory and registry traits.
  - `parsing/{python, rust, javascript, typescript, go, c, cpp, csharp, java, kotlin, php, swift, lua}/parser.rs`: Specific AST visitor and extraction code.
- **Reference**: `reference_repos/tree-sitter-python/`: Grammar and AST node specification.
- **Application**: Port and adapt `codanna`'s `LanguageParser` traits and language-specific AST visitor implementations to parse `git2::Blob` historical snapshots and produce `FunctionTouch` records.

### Phase 5: Function Lineage Engine & Mutation Oracle
- **Reference**: `reference_repos/codanna/src/`
  - `symbol/`: Qualified name hierarchies (`module::class::method`), symbol representation, symbol hashing.
  - `relationship/`: Graph relationship types between symbols and callers.
- **Application**: Build the 4-tier MinHash lineage resolver in `gitctx-core` / `gitctx-index`, reusing naming conventions and bipartite matching strategies across all supported languages.

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

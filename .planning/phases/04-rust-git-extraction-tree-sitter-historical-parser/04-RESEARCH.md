# Phase 4: Rust Git Extraction & Tree-Sitter Historical Parser - Research

**Researched:** 2026-08-17
**Domain:** Rust Git extraction (`git2`), multi-language AST parsing (`tree-sitter`), and deterministic D4 pseudo-document templating
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01 (Multi-Language Coverage):** Full 8-language suite (Python, Rust, JavaScript, TypeScript, Go, C/C++, Java, PHP) bundled directly in `gitctx-core`, establishing complete parity with the `codanna` reference parser registry and ROADMAP.md §Phase 4.
- **D-02 (Grammar Dependencies):** Official tree-sitter language crates (`tree-sitter-python`, `tree-sitter-rust`, `tree-sitter-javascript`, `tree-sitter-typescript`, `tree-sitter-go`, `tree-sitter-c`, `tree-sitter-cpp`, `tree-sitter-java`, `tree-sitter-php`) compiled directly into `gitctx-core`.
- **D-03 (Imperative AST Traversal):** Adapted `LanguageBehavior` trait pattern from `reference_repos/codanna/src/parsing/language_behavior.rs` for fine-grained extraction of function signatures, docstrings, decorators, and nested functions across diverse grammars.
- **D-04 (Extension & Shebang Registry):** Extension registry with shebang fallback to resolve language identity for historical blobs (mapping standard file extensions and inspecting shebang lines for extensionless executable scripts).
- **D-05 (Historical Degradation Strategy):** Single file-level degraded chunk (`qualname = "<file_level>"`, `parse_status = ParseStatus::Degraded`) covering the file's diff changes when syntax errors prevent full AST function extraction, ensuring the indexer never crashes or drops commits on broken historical code.
- **D-06 (In-Memory Blob OID Cache):** In-memory LRU cache keyed by `(git2::Oid, LanguageId)` with capacity ~50,000 entries storing parsed symbol definitions to eliminate redundant AST parsing across unchanged historical blobs.
- **D-07 (Partial Extraction on Syntax Errors):** Best-effort partial extraction on syntax errors: extract cleanly parsed function nodes with `ParseStatus::Ok`, and emit a companion degraded chunk only if unparsed `ERROR` regions contain diff changes.
- **D-08 (Parse Telemetry):** Aggregate parse telemetry recording `ok`, `degraded`, and `failed` counts in index metadata for health reporting in `gitctx doctor` and CI invariant tests.
- **D-09 (D4 Template Generation):** Standard D4 template format with compact empty-section omission: render 6 canonical sections (Header, Sanitized Commit Message, Added Identifiers, Removed Identifiers, Function Signature, Surrounding Docstring), omitting empty sections when not applicable.
- **D-10 (Module-Level Touches):** Dedicated module-level pseudo-documents: emit a module touch with `qualname = "<module>"` summarizing top-level constants, imports, and global configuration changes outside function/method boundaries.
- **D-11 (Symmetric Token Diffing):** Extract and diff tokens between added (`+`) and removed (`-`) diff lines, filter language keywords, and categorize into distinct `Added:` and `Removed:` identifier sets.
- **D-12 (Chunk Granularity):** Index each function touch as an independent searchable chunk (with `lineage_id` and exact citations) plus module chunks for file-level edits.
- **D-13 (Credential Prefix Suite):** Redact GitHub tokens (`ghp_`, `gho_`, `ghu_`, `ghs_`, `ghr_`), AWS access keys (`AKIA`, `ASIA`), OpenAI keys (`sk-`), Slack tokens (`xoxb-`, `xoxp-`), Private Key headers (`-----BEGIN (RSA|OPENSSH|EC|DSA|PRIVATE KEY)`), and JWT bearer tokens.
- **D-14 (Shannon Entropy Filter):** Calculate Shannon entropy on contiguous base64/hex character strings $\ge 24$ chars with threshold $\ge 4.5$ bits/char, exempting UUIDs, git commit SHAs, and common test fixture hashes.
- **D-15 (Typed Semantic Placeholders):** Replace redacted secrets with `[REDACTED_SECRET: <type>]` (e.g. `[REDACTED_SECRET: github_token]`, `[REDACTED_SECRET: high_entropy_string]`) preserving syntactic position and informing search that credentials were sanitized.
- **D-16 (Universal Redaction Boundary):** Execute secret redaction across diffs, commit messages, extracted identifiers, and rendered chunk pseudo-documents at the initial extraction boundary in `gitctx-core`.

### the agent's Discretion
- Internal buffer capacities and helper module partitioning across `gitctx-core`.
- Exact regex definitions for language keyword filtering and secondary token normalizations.

### Deferred Ideas (OUT OF SCOPE)
- None — all requirements strictly bounded to Phase 4 scope.
</user_constraints>

<architectural_responsibility_map>
## Architectural Responsibility Map

| Module / Component | Primary Responsibility | Key Inputs / Dependencies | Output Artifacts / Types |
| :--- | :--- | :--- | :--- |
| `gitctx_core::git` | First-parent commit walking, tree diffing, rename/copy detection, blob extraction | `git2::Repository`, `git2::Revwalk`, `git2::Diff` | `GitCommit`, `FileDelta`, `DiffHunk`, `RawBlob` |
| `gitctx_core::parser::registry` | Language identification via extension & shebang, behavior routing | File path, shebang bytes | `LanguageId`, `Arc<dyn LanguageDefinition>` |
| `gitctx_core::parser::behavior` | Per-language AST navigation, node extraction, signature & docstring parsing | `tree_sitter::Node`, source code byte slice | `RawFunctionSymbol`, `Docstring`, `Signature` |
| `gitctx_core::parser::cache` | LRU caching of parsed symbol definitions per blob OID | `(git2::Oid, LanguageId)`, parsed AST symbols | `Arc<Vec<RawFunctionSymbol>>` |
| `gitctx_core::parser::degrade` | Syntax error recovery, `ERROR` node interception, fallback `<file_level>` touch | AST parse tree, diff hunks | `Vec<FunctionTouch>` with `ParseStatus` (`Ok`/`Degraded`) |
| `gitctx_core::template` | D4 template formatting, token extraction, symmetric keyword-filtered identifier diffing | `FunctionTouch`, `DiffHunk`, `SanitizedChunk` | Rendered D4 pseudo-document `String` |
| `gitctx_core::redact` | Defense-in-depth secret filtering (API key prefixes & Shannon entropy) | Unsanitized text / diffs / identifiers | Sanitized string with `[REDACTED_SECRET: <type>]` |
| `gitctx_core::telemetry` | Aggregation of parse health metrics across commits and files | `ParseStatus` occurrences | `ParseTelemetry` (`ok_count`, `degraded_count`, `failed_count`) |
</architectural_responsibility_map>

<research_summary>
## Summary

Phase 4 constructs the foundational Rust extraction and historical parsing pipeline in `crates/gitctx-core`. It converts raw Git commit graphs and historical blob trees into structured, citation-rich, deterministic search chunks without network or LLM dependencies.

The architecture is partitioned into three decoupled layers:
1. **Extraction (`gitctx-core::git`):** A `libgit2` first-parent commit walker with topological sorting and `find_similar()` rename detection.
2. **Historical Parsing (`gitctx-core::parser`):** An 8-language `tree-sitter` parser registry using `LanguageBehavior` traits, an in-memory blob OID LRU cache, and graceful degradation for syntax errors.
3. **Template & Redaction (`gitctx-core::template` & `gitctx-core::redact`):** Symmetric identifier delta extraction, universal secret redaction (API key prefixes + Shannon entropy $\ge 4.5$), and D4 template formatting.

**Primary recommendation:** Implement `LanguageBehavior` traits for Python, Rust, JavaScript, TypeScript, Go, C/C++, Java, and PHP with zero-panic AST traversal, caching symbol results per `(Oid, LanguageId)` in an LRU cache, and routing broken syntax regions to `<file_level>` degraded touches.
</research_summary>

<standard_stack>
## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `git2` | 0.19.0 | Git repository object database access, tree diffs, rename detection | Standard Rust binding for libgit2; offline, fast, memory-safe |
| `tree-sitter` | 0.24.7 | Incremental AST parsing across historical blobs | Fast C-based parser generator with zero panics on broken syntax |
| `tree-sitter-python` | 0.23.6 | Python grammar AST extraction | Official tree-sitter grammar for Python |
| `tree-sitter-rust` | 0.23.2 | Rust grammar AST extraction | Official tree-sitter grammar for Rust |
| `tree-sitter-javascript` | 0.23.1 | JavaScript grammar AST extraction | Official tree-sitter grammar for JS / JSX |
| `tree-sitter-typescript` | 0.23.2 | TypeScript grammar AST extraction | Official tree-sitter grammar for TS / TSX |
| `tree-sitter-go` | 0.23.4 | Go grammar AST extraction | Official tree-sitter grammar for Go |
| `tree-sitter-c` | 0.23.4 | C grammar AST extraction | Official tree-sitter grammar for C |
| `tree-sitter-cpp` | 0.23.4 | C++ grammar AST extraction | Official tree-sitter grammar for C++ |
| `tree-sitter-java` | 0.23.5 | Java grammar AST extraction | Official tree-sitter grammar for Java |
| `tree-sitter-php` | 0.23.11 | PHP grammar AST extraction | Official tree-sitter grammar for PHP |
| `lru` | 0.13.0 | OID parse cache | High-performance LRU cache in pure Rust |
| `regex` | 1.11.1 | Secret prefix pattern matching and identifier tokenization | Pre-compiled regex engine with guaranteed linear-time matching |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `LanguageBehavior` Trait | Tree-sitter `.scm` query files | S-expression query files break across minor grammar changes; imperative AST traversal gives finer control over docstrings, decorators, and recovery |
| Official tree-sitter crates | Vendored C grammar sources in `build.rs` | Official crates are maintained by upstream language teams and provide standard Rust bindings |
</standard_stack>

<architecture_patterns>
## Architecture Patterns

### System Architecture Diagram

```
+---------------------------------------------------------------------------------------------------+
|                                      gitctx-core Pipeline                                         |
+---------------------------------------------------------------------------------------------------+
                                                  |
                          +-----------------------v-----------------------+
                          |      libgit2 First-Parent Git Walker          |
                          |  - revwalk.simplify_first_parent()            |
                          |  - revwalk.set_sorting(TOPOLOGICAL | TIME)    |
                          |  - diff_tree_to_tree() + find_similar()       |
                          +-----------------------+-----------------------+
                                                  |
                         Commit + Changed File Deltas (Path, Old/New OIDs)
                                                  |
                          +-----------------------v-----------------------+
                          |        Extension & Shebang Resolver           |
                          |  - ".py", ".rs", ".ts", ".go", etc.           |
                          |  - Shebang fallback ("#!/usr/bin/env python") |
                          +-----------------------+-----------------------+
                                                  |
                          +-----------------------v-----------------------+
                          |       Blob OID Parse Cache (LRU)              |
                          |  Key: (git2::Oid, LanguageId) -> Cache Hit?   |
                          +-----------+-----------------------+-----------+
                                      | No                    | Yes
                                      v                       |
                          +-----------------------+           |
                          | Tree-Sitter Parser    |           |
                          | (LanguageBehavior)    |           |
                          |  - Functions, Methods |           |
                          |  - Docstrings, Sigs   |           |
                          |  - Error Recovery     |           |
                          +-----------+-----------+           |
                                      |                       |
                                      +-----------> + <-------+
                                                    |
                                    RawFunctionSymbols & Module Touches
                                                    |
                          +-------------------------v-------------------------+
                          |             Touch & Diff Correlator               |
                          |  - Intersect function byte spans with diff hunks  |
                          |  - Non-function diffs -> <module> touches         |
                          |  - Unresolved ERROR diffs -> <file_level> degraded|
                          +-------------------------+-------------------------+
                                                    |
                          +-------------------------v-------------------------+
                          |   Symmetric Identifier Diff & Redactor            |
                          |  - Added / Removed tokens filtered by keywords    |
                          |  - Prefix & Shannon entropy secret filter         |
                          |  - Leakage sanitizer (issue #, PR trailers)       |
                          +-------------------------+-------------------------+
                                                    |
                          +-------------------------v-------------------------+
                          |          Deterministic D4 Templater               |
                          |  - Render Header, Message, Identifiers, Sig, Doc  |
                          |  - Omit empty sections cleanly                    |
                          +-------------------------+-------------------------+
                                                    |
                                                    v
                                    Vector of Indexed FunctionTouches
```

### Recommended Project Structure
```
crates/gitctx-core/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── model.rs                 # FunctionTouch, CommitId, LineageId, ParseStatus, Citation
    ├── error.rs                 # CoreError, GitError, ParseError
    ├── config.rs                # IndexConfig, ConfigHash
    ├── qualname.rs              # QualName helpers
    ├── sanitize.rs              # Leakage sanitization pipeline (D-09..D-11)
    ├── git/
    │   ├── mod.rs               # GitExtractor facade
    │   ├── walker.rs            # First-parent Revwalk with topological sort
    │   ├── diff.rs              # Tree diffing, rename detection, hunks
    │   └── repository.rs        # Safe wrapper around git2::Repository
    ├── parser/
    │   ├── mod.rs               # AstExtractor, ParseTelemetry
    │   ├── registry.rs          # LanguageRegistry, LanguageDefinition, LanguageId
    │   ├── behavior.rs          # LanguageBehavior trait definition
    │   ├── cache.rs             # BlobParseCache (LRU for (Oid, LanguageId))
    │   ├── degrade.rs           # Graceful degradation & error node handling
    │   └── languages/           # LanguageBehavior implementations
    │       ├── mod.rs
    │       ├── python.rs
    │       ├── rust.rs
    │       ├── javascript.rs
    │       ├── typescript.rs
    │       ├── go.rs
    │       ├── c.rs
    │       ├── cpp.rs
    │       ├── java.rs
    │       └── php.rs
    ├── template/
    │   ├── mod.rs               # D4 template renderer
    │   ├── identifiers.rs       # Symmetric token extraction & keyword filter
    │   └── keywords.rs          # Cross-language keyword exclusion sets
    └── redact/
        ├── mod.rs               # SecretRedactor facade
        ├── prefixes.rs          # Known API key prefixes (ghp_, AKIA, sk-, etc.)
        └── entropy.rs           # Shannon entropy calculation & exemptions
```
</architecture_patterns>

<dont_hand_roll>
## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Commit Graph Traversal | Custom recursive Git commit walking | `git2::Revwalk` with `simplify_first_parent()` and `Sort::TOPOLOGICAL \| Sort::TIME` | Handles merges, octopus commits, and cycles correctly without stack overflow. |
| Rename & Copy Tracking | Custom file content similarity matcher | `git2::Diff::find_similar()` with `DiffFindOptions` | `libgit2`'s C implementation is battle-tested against Git test suites. |
| AST Parsing | Custom regexes or parser combinators | Official `tree-sitter` grammars | C-based grammars parse robustly in milliseconds and generate parse trees even on syntax errors. |
| Blob OID Caching | Custom `HashMap` with home-grown eviction | `lru::LruCache` | Bounded memory with correct eviction order and fast $O(1)$ lookups. |
</dont_hand_roll>

<common_pitfalls>
## Common Pitfalls

### Pitfall 1: Root Commits & Detached HEAD in libgit2
**What goes wrong:** `revwalk.push_head()` fails on unborn branches; initial commits have 0 parents, causing `tree_to_tree` diffs against parent to panic.
**How to avoid:** Explicitly check parent count: for root commits, diff against an empty tree (`None` as old tree).

### Pitfall 2: Discarding Files on `node.has_error()`
**What goes wrong:** A syntax error on line 500 causes `node.has_error()` to be true on the entire root node, dropping all valid functions on lines 1-499.
**How to avoid:** Traverse child nodes and extract cleanly parsed `function_definition` nodes with `ParseStatus::Ok`. Only emit a `<file_level>` degraded chunk for spans intersecting `ERROR` / `MISSING` nodes.

### Pitfall 3: Multi-Byte UTF-8 String Slicing Panics
**What goes wrong:** Tree-sitter byte ranges may fall in the middle of UTF-8 characters if source code contains multi-byte emojis or unicode identifiers, causing Rust panic.
**How to avoid:** Always use `code.get(start..end)` or validate UTF-8 character boundaries before slicing.

### Pitfall 4: False Positive Shannon Entropy on Valid Code
**What goes wrong:** Shannon entropy flags UUIDs, Git commit SHAs, SHA256 test constants, or base64 image data in fixtures as secrets.
**How to avoid:** Require candidate token length $\ge 24$, enforce base64/hex alphabet purity, and exempt exact UUID and 40/64-char hex patterns.
</common_pitfalls>

<code_examples>
## Code Examples

### 1. First-Parent Revwalk & Diff
```rust
let mut revwalk = repo.revwalk()?;
revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
revwalk.simplify_first_parent()?;
revwalk.push_head()?;

let mut diff_opts = git2::DiffOptions::new();
diff_opts.context_lines(3);

let mut find_opts = git2::DiffFindOptions::new();
find_opts.renames(true);
find_opts.copies(true);
```

### 2. LanguageBehavior Trait
```rust
pub trait LanguageBehavior: Send + Sync {
    fn language_id(&self) -> LanguageId;
    fn extensions(&self) -> &'static [&'static str];
    fn matches_shebang(&self, first_line: &str) -> bool;
    fn extract_symbols(&self, node: tree_sitter::Node, code: &str) -> Vec<RawFunctionSymbol>;
    fn extract_signature(&self, node: tree_sitter::Node, code: &str) -> String;
    fn extract_docstring(&self, node: tree_sitter::Node, code: &str) -> Option<String>;
    fn format_qualname(&self, parent_scope: Option<&str>, name: &str) -> String;
}
```

### 3. Shannon Entropy Formula
$$H = -\sum_{i=1}^{n} p_i \log_2(p_i)$$
Where $p_i$ is the empirical byte frequency of each character in candidate token slice.
</code_examples>

<sota_updates>
## State of the Art (2024-2026)

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `tree-sitter = "0.20"` with raw C pointers | `tree-sitter = "0.24"` with typed `Language::from(tree_sitter_lang::LANGUAGE)` | 2024 | Modern typed API, standard ABI-14 compatibility, safe Rust idioms |
| LLM summarization for diffs | Deterministic D4 template rendering | 2026 (Phase 3 validation) | 100% offline, zero latency/cost, higher Recall@10 on why-queries |
</sota_updates>

<open_questions>
## Open Questions

1. **Secondary file extensions for C/C++ header files (.h, .hpp, .hxx):**
   - Recommendation: Default `.h` to C or C++ parser using C++ behavior (which parses valid C as well).
</open_questions>

<validation_architecture>
## Validation Architecture (Nyquist Test Strategy)

### Nyquist Test Matrix

| Test Category | Target Subsystem | Method / Fixture | Success Criteria |
| :--- | :--- | :--- | :--- |
| **Git First-Parent Walker** | `gitctx_core::git::walker` | `RepoBuilder` linear commits & branch merges | Only first-parent commits returned; topological order strictly maintained. |
| **Git Rename & Copy** | `gitctx_core::git::diff` | `RepoBuilder` rename commit with modifications | `git2::Delta::Renamed` detected; old and new paths accurately mapped. |
| **Tree-Sitter Language Parity** | `gitctx_core::parser::languages` | Multi-language fixture corpus (Python, Rust, TS, JS, Go, C, C++, Java, PHP) | Function names, parameter signatures, and docstrings accurately extracted across all 8 languages. |
| **Graceful Degradation** | `gitctx_core::parser::degrade` | Synthetic commit with broken Python/Rust syntax | Zero panics; valid functions extracted with `ParseStatus::Ok`; broken span emits `ParseStatus::Degraded` `<file_level>` touch. |
| **Blob OID LRU Cache** | `gitctx_core::parser::cache` | Repetitive commit diffs on unchanged blobs | AST parsed exactly once per `(Oid, LanguageId)`; subsequent lookups hit LRU cache. |
| **Symmetric Identifier Delta** | `gitctx_core::template::identifiers` | Synthetic diff with additions/deletions and keywords | Keywords (`def`, `fn`, `return`, `pub`) stripped; identifiers categorized into distinct `Added:` and `Removed:` sets. |
| **Deterministic D4 Templater** | `gitctx_core::template` | Unit tests with varied function/module inputs | Byte-identical output on repeated runs; empty docstring/signature sections cleanly omitted. |
| **Defense-in-Depth Redaction** | `gitctx_core::redact` | Synthetic strings with GitHub tokens, AWS keys, JWTs, high entropy base64, UUIDs | Known keys replaced with typed placeholder; high-entropy tokens redacted; UUIDs & commit SHAs preserved intact. |
</validation_architecture>

<sources>
## Sources

### Primary (HIGH confidence)
- `reference_repos/codanna/src/git.rs` — Git repository operations and commit resolution
- `reference_repos/codanna/src/parsing/` (`registry.rs`, `language_behavior.rs`, `language.rs`, `python/`, `rust/`, `typescript/`, `go/`)
- `eval/ablation/representations.py` — Phase 3 D1-D5 template generators and identifier extraction
- `crates/gitctx-core/src/model.rs`, `sanitize.rs`, `qualname.rs`, `error.rs`
- `tests/fixtures/mkrepo.rs` — Programmatic synthetic Git repository generator
- `reference_docs/20260816-Implementation-Plan-Review-Suggestions.md` §1, §5, §6.1
- `.planning/REQUIREMENTS.md` §PARSE (PARSE-01 through PARSE-06)
- `.planning/ROADMAP.md` §Phase 4
</sources>

<metadata>
## Metadata

**Research scope:**
- Core technology: Rust `git2`, `tree-sitter` (8 languages), D4 templating, Shannon entropy secret redaction
- Confidence: HIGH across all modules

**Research date:** 2026-08-17
**Valid until:** 2026-09-17
</metadata>

---

*Phase: 04-rust-git-extraction-tree-sitter-historical-parser*
*Research completed: 2026-08-17*
*Ready for planning: yes*

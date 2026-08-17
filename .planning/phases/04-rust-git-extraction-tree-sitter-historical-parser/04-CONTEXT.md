# Phase 4: Rust Git Extraction & Tree-Sitter Historical Parser - Context

**Gathered:** 2026-08-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 4 builds the foundational Rust extraction and historical parsing pipeline (`gitctx-core` and `gitctx-index`). It delivers:
- `libgit2` first-parent commit walker with topological ordering, commit size filtering, and rename/copy detection.
- Multi-language `tree-sitter` historical blob parser registry supporting 8 languages (Python, Rust, JavaScript, TypeScript, Go, C/C++, Java, PHP) using the `LanguageBehavior` trait pattern.
- In-memory blob OID parse cache keyed by `(Oid, Language)` to eliminate redundant AST parsing across unchanged historical blobs.
- Graceful degradation error handling emitting `<file_level>` touches with `parse_status = ParseStatus::Degraded` on syntax-broken historical commits.
- Deterministic template chunk generator producing D4 pseudo-documents per (commit, function) touch and `<module>` touch.
- Universal defense-in-depth secret redaction replacing known credential prefixes and high-entropy strings with typed placeholders.

</domain>

<decisions>
## Implementation Decisions

### Multi-Language Grammar Coverage
- **D-01:** Full 8-language suite (Python, Rust, JavaScript, TypeScript, Go, C/C++, Java, PHP) bundled in `gitctx-core`, establishing complete parity with `codanna` reference parser registry and ROADMAP.md §Phase 4. — **Reversibility:** costly — defines language registry and parser dependencies.
- **D-02:** Official tree-sitter language crates (`tree-sitter-python`, `tree-sitter-rust`, `tree-sitter-javascript`, `tree-sitter-typescript`, `tree-sitter-go`, `tree-sitter-c`, `tree-sitter-cpp`, `tree-sitter-java`, `tree-sitter-php`) compiled directly into `gitctx-core`. — **Reversibility:** costly — dependency management and build pipeline.
- **D-03:** Imperative AST traversal using `LanguageBehavior` trait pattern (adapted from `reference_repos/codanna/src/parsing/language_behavior.rs`), providing fine-grained extraction of function signatures, docstrings, decorators, and nested functions across diverse grammars. — **Reversibility:** costly — core AST extraction architecture.
- **D-04:** Extension registry with shebang fallback to resolve language identity for historical blobs (mapping standard file extensions and inspecting shebang lines for extensionless executable scripts). — **Reversibility:** reversible.

### Historical Blob Degradation Strategy
- **D-05:** Single file-level degraded chunk (`qualname = "<file_level>"`, `parse_status = ParseStatus::Degraded`) covering the file's diff changes when syntax errors prevent full AST function extraction, ensuring the indexer never crashes or drops commits on broken historical code. — **Reversibility:** costly — error handling contract and schema expectation.
- **D-06:** In-memory LRU cache keyed by `(git2::Oid, Language)` (capacity ~50,000 entries) storing parsed symbol definitions to avoid redundant AST parsing across unchanged historical blobs. — **Reversibility:** reversible.
- **D-07:** Best-effort partial extraction on syntax errors: extract all cleanly parsed function nodes with `ParseStatus::Ok`, and emit a companion degraded chunk only if unparsed `ERROR` regions contain diff changes. — **Reversibility:** costly — parser error resilience logic.
- **D-08:** Aggregate parse telemetry recording `ok`, `degraded`, and `failed` counts in index metadata for health reporting in `gitctx doctor` and CI invariant tests. — **Reversibility:** reversible.

### Template Chunk Rendering & Module-Level Changes
- **D-09:** Standard D4 template format with compact empty-section omission: render the 6 canonical sections (Header, Sanitized Commit Message, Added Identifiers, Removed Identifiers, Function Signature, Surrounding Docstring), omitting empty sections when not applicable. — **Reversibility:** one-way — template format determines search index document representations.
- **D-10:** Dedicated module-level pseudo-documents: emit a module touch with `qualname = "<module>"` summarizing top-level constants, imports, and global configuration changes outside function/method boundaries. — **Reversibility:** costly — chunk schema and lineage tracking for non-function diffs.
- **D-11:** Symmetric token difference: extract and diff tokens between added (`+`) and removed (`-`) diff lines, filter language keywords, and categorize into distinct `Added:` and `Removed:` identifier sets. — **Reversibility:** costly — identifier delta extractor implementation.
- **D-12:** Per-(commit, function) chunk granularity: index each function touch as an independent searchable chunk (with `lineage_id` and exact citations) plus module chunks for file-level edits. — **Reversibility:** one-way — core index and retrieval unit.

### Secret Redaction Policy & Entropy Thresholds
- **D-13:** Comprehensive credential prefix suite: redact GitHub tokens (`ghp_`, `gho_`, `ghu_`, `ghs_`, `ghr_`), AWS access keys (`AKIA`, `ASIA`), OpenAI keys (`sk-`), Slack tokens (`xoxb-`, `xoxp-`), Private Key headers (`-----BEGIN (RSA|OPENSSH|EC|DSA|PRIVATE KEY)`), and JWT bearer tokens. — **Reversibility:** costly — secret filtering rules across all indexing paths.
- **D-14:** Shannon entropy filter with code exceptions: calculate Shannon entropy on contiguous base64/hex character strings $\ge 24$ chars with threshold $\ge 4.5$ bits/char, exempting UUIDs, git commit SHAs, and common test fixture hashes. — **Reversibility:** costly — entropy calculation and exemption rules.
- **D-15:** Typed semantic placeholder: replace redacted secrets with `[REDACTED_SECRET: <type>]` (e.g. `[REDACTED_SECRET: github_token]`, `[REDACTED_SECRET: high_entropy_string]`) preserving syntactic position and informing search that credentials were sanitized. — **Reversibility:** reversible.
- **D-16:** Universal end-to-end redaction: execute secret redaction across diffs, commit messages, extracted identifiers, and rendered chunk pseudo-documents at the initial extraction boundary. — **Reversibility:** costly — security boundary enforcement in `gitctx-core`.

### the agent's Discretion
- Internal buffer capacities and helper module partitioning across `gitctx-core`.
- Exact regex definitions for language keyword filtering and secondary token normalizations.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Architecture & Specification
- `reference_docs/gpai-20260816-gitctx-Complete-Step-by-Step-Implementation-Plan-w.md` — Complete step-by-step implementation plan for gitctx
- `reference_docs/20260816-Implementation-Plan-Review-Suggestions.md` §1 (Templated pseudo-doc), §5 (Historical blob parsing & degradation), §6.1 (Invariants I1, I2, I6)
- `.planning/PROJECT.md` — Project definition, core values, active requirements, and constraints
- `.planning/REQUIREMENTS.md` §PARSE — Requirements PARSE-01 through PARSE-06
- `.planning/ROADMAP.md` §Phase 4 — Phase 4 goals, plans, and success criteria

### Reference Implementations & Grammar Specs
- `eval/ablation/representations.py` — Phase 3 reference implementation for D1-D5 template rendering and identifier extraction
- `reference_repos/codanna/src/git.rs` — Reference `libgit2` repository access, first-parent walker, ignore matching
- `reference_repos/codanna/src/parsing/` (`registry.rs`, `language_behavior.rs`, `language.rs`, `paths.rs`) — Multi-language parser registry, AST traversal, behavior definitions for 8+ languages
- `reference_repos/tree-sitter-python/` — Python AST grammar and node types

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/gitctx-core/src/model.rs` — Core domain models (`CommitId`, `LineageId`, `ParseStatus`, `Citation`, `FunctionTouch`).
- `crates/gitctx-core/src/sanitize.rs` — Production leakage sanitization pipeline (`sanitize_chunk_text`, `SanitizedChunk`).
- `crates/gitctx-core/src/qualname.rs` — Qualified name container (`QualName`).
- `tests/fixtures/mkrepo.rs` — Synthetic Git repository generator supporting linear commits, renames, moves, and broken syntax.

### Established Patterns
- Strongly typed domain identifiers (`CommitId`, `LineageId`) and error enums with `thiserror`.
- Zero-panic parsing with explicit `ParseStatus` states (`Ok`, `Degraded`, `Failed`).
- Isolated offline test execution enforcing Invariant I1.

### Integration Points
- `crates/gitctx-core/src/` — New modules for `git.rs` (walker/diff), `parser/` (tree-sitter registry/behaviors), `template.rs` (D4 renderer), and `redact.rs` (secret filter).
- `crates/gitctx-core/Cargo.toml` — Add `tree-sitter` and language grammar dependencies.

</code_context>

<specifics>
## Specific Ideas

- D4 pseudo-document template structure:
  ```text
  Function `{qualname}` in `{path}` was modified by {author} on {date}.
  Commit message: {sanitized_message}
  Added: {added_identifiers}
  Removed: {removed_identifiers}
  Signature: {signature}
  Surrounding docstring: {docstring}
  ```
- Module-level touch template:
  ```text
  Module `{path}` was modified by {author} on {date}.
  Commit message: {sanitized_message}
  Modified top-level symbols: {identifiers}
  ```
- Redaction placeholder format: `[REDACTED_SECRET: <type>]`.

</specifics>

<deferred>
## Deferred Ideas

- None — discussion stayed strictly within Phase 4 scope.

</deferred>

---

*Phase: 04-Rust Git Extraction & Tree-Sitter Historical Parser*
*Context gathered: 2026-08-17*

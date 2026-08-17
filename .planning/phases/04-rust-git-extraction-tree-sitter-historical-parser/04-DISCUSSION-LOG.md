# Phase 4: Rust Git Extraction & Tree-Sitter Historical Parser - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-08-17
**Phase:** 04-Rust Git Extraction & Tree-Sitter Historical Parser
**Areas discussed:** Multi-Language Grammar Coverage, Historical Blob Degradation Strategy, Template Chunk Rendering & Module-Level Changes, Secret Redaction Policy & Entropy Thresholds

---

## Multi-Language Grammar Coverage

| Option | Description | Selected |
|--------|-------------|----------|
| Full 8-language suite | Python, Rust, JavaScript, TypeScript, Go, C/C++, Java, and PHP (full parity with codanna parser registry and ROADMAP.md). | ✓ |
| Core 4 corpus languages only | Python, Rust, JavaScript/TypeScript, and Go (the languages present in the pinned evaluation corpus; defer others). | |
| You decide | Agent discretion. | |

**User's choice:** Full 8-language suite
**Notes:** Support the complete 8-language set upfront to avoid deferred grammar debt.

| Option | Description | Selected |
|--------|-------------|----------|
| Official tree-sitter crates | Use standard crates (tree-sitter-python, tree-sitter-rust, tree-sitter-javascript, tree-sitter-typescript, etc.) compiled into gitctx-core. | ✓ |
| Vendored C grammar sources | Compile grammar sources via cc in build.rs for maximum build-time control. | |
| You decide | Agent discretion. | |

**User's choice:** Official tree-sitter crates

| Option | Description | Selected |
|--------|-------------|----------|
| Imperative AST traversal with LanguageBehavior traits | Mirrors reference_repos/codanna/src/parsing/ for robust extraction of signatures, docstrings, decorators, and nested functions. | ✓ |
| Declarative tree-sitter query (.scm) files | Use tree-sitter S-expression query files to match function definitions. | |
| You decide | Agent discretion. | |

**User's choice:** Imperative AST traversal with LanguageBehavior traits

| Option | Description | Selected |
|--------|-------------|----------|
| Extension registry with shebang fallback | Map standard file extensions with shebang inspection for extensionless executable scripts. | ✓ |
| Extension-only mapping | Map strictly by file extension; treat unmapped/extensionless files as plain text. | |
| You decide | Agent discretion. | |

**User's choice:** Extension registry with shebang fallback

---

## Historical Blob Degradation Strategy

| Option | Description | Selected |
|--------|-------------|----------|
| File-level degraded chunk | Emit a single FunctionTouch with qualname="<file_level>" and parse_status=ParseStatus::Degraded covering the file's diff changes. | ✓ |
| Hunk-level degraded chunks | Emit separate degraded touches for each unified diff hunk within the unparseable file. | |
| You decide | Agent discretion. | |

**User's choice:** File-level degraded chunk

| Option | Description | Selected |
|--------|-------------|----------|
| LRU Cache by (Oid, Language) | In-memory LRU cache (e.g., 50k entries) storing parsed symbol definitions to avoid redundant AST parses across commit history. | ✓ |
| Unbounded HashMap cache | Cache all unique blob OIDs in memory during indexing for maximum throughput. | |
| You decide | Agent discretion. | |

**User's choice:** LRU Cache by (Oid, Language)

| Option | Description | Selected |
|--------|-------------|----------|
| Best-effort partial extraction | Extract all cleanly parsed function nodes with ParseStatus::Ok, and emit a companion degraded chunk only if unparsed ERROR regions contain diff changes. | ✓ |
| All-or-nothing per file | If tree-sitter AST contains ERROR nodes, mark the entire file and its touches as ParseStatus::Degraded. | |
| You decide | Agent discretion. | |

**User's choice:** Best-effort partial extraction

| Option | Description | Selected |
|--------|-------------|----------|
| Aggregate parse telemetry | Collect and record ok/degraded/failed counts in index metadata for health reporting in `gitctx doctor` and CI invariant tests. | ✓ |
| Log warnings only | Emit tracing warnings on degraded parses without tracking persistent counters in index metadata. | |
| You decide | Agent discretion. | |

**User's choice:** Aggregate parse telemetry

---

## Template Chunk Rendering & Module-Level Changes

| Option | Description | Selected |
|--------|-------------|----------|
| Standard D4 template with compact empty-section omission | Render the 6 standard sections (Header, Sanitized Message, Added Identifiers, Removed Identifiers, Signature, Docstring), cleanly omitting empty sections when not applicable. | ✓ |
| Fixed 6-section template with explicit 'None' placeholders | Always include all 6 section headers even if a section has no content. | |
| You decide | Agent discretion. | |

**User's choice:** Standard D4 template with compact empty-section omission

| Option | Description | Selected |
|--------|-------------|----------|
| Dedicated Module-level pseudo-document | Emit a module touch with qualname="<module>" summarizing top-level constants, imports, and global configuration changes. | ✓ |
| File-level fallback chunk | Treat top-level/non-function changes under the <file_level> catch-all touch. | |
| You decide | Agent discretion. | |

**User's choice:** Dedicated Module-level pseudo-document

| Option | Description | Selected |
|--------|-------------|----------|
| Symmetric token difference | Parse added (+) vs removed (-) diff tokens, filter keywords, and categorize into distinct Added and Removed identifier lists. | ✓ |
| Merged identifier set | Extract a single combined list of modified identifiers without separating added vs removed. | |
| You decide | Agent discretion. | |

**User's choice:** Symmetric token difference

| Option | Description | Selected |
|--------|-------------|----------|
| Per-(commit, function) granularity | Index each function touch as an independent searchable chunk (with lineage_id and exact citations) plus module chunks for file-level edits. | ✓ |
| Single combined commit document | Combine all function changes in a commit into one monolithic chunk. | |
| You decide | Agent discretion. | |

**User's choice:** Per-(commit, function) granularity

---

## Secret Redaction Policy & Entropy Thresholds

| Option | Description | Selected |
|--------|-------------|----------|
| Comprehensive prefix suite | Redact GitHub tokens (ghp_, gho_, ghu_, etc.), AWS keys (AKIA/ASIA), OpenAI (sk-), Slack (xoxb/xoxp), Private Keys (-----BEGIN), and JWTs. | ✓ |
| Basic prefix suite | Redact GitHub and AWS API keys only. | |
| You decide | Agent discretion. | |

**User's choice:** Comprehensive prefix suite

| Option | Description | Selected |
|--------|-------------|----------|
| Shannon entropy filter with code exceptions | Entropy threshold ≥ 4.5 on strings ≥ 24 chars, with exemptions for UUIDs, git commit SHAs, and common test fixture hashes. | ✓ |
| Regex/prefix matching only | Rely strictly on known API key and credential patterns without Shannon entropy to avoid false positives. | |
| You decide | Agent discretion. | |

**User's choice:** Shannon entropy filter with code exceptions

| Option | Description | Selected |
|--------|-------------|----------|
| Typed semantic placeholder | Replace secrets with `[REDACTED_SECRET: <type>]` (e.g., `[REDACTED_SECRET: github_token]`) to preserve context without leaking tokens. | ✓ |
| Generic placeholder | Replace all matched secrets with a simple `[REDACTED]` token. | |
| You decide | Agent discretion. | |

**User's choice:** Typed semantic placeholder

| Option | Description | Selected |
|--------|-------------|----------|
| Universal end-to-end redaction | Redact diffs, commit messages, identifiers, and rendered chunk pseudo-documents at the extraction boundary. | ✓ |
| Render-time redaction only | Redact only the final assembled chunk text right before indexing. | |
| You decide | Agent discretion. | |

**User's choice:** Universal end-to-end redaction

---

## the agent's Discretion

- Internal buffer capacities and helper module partitioning in `gitctx-core`.
- Exact regex definitions for secondary keyword filtering.

## Deferred Ideas

- None — discussion stayed strictly within Phase 4 scope.

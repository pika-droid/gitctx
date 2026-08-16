# Phase 1: Project Foundation, Invariants & Fixture Scaffolding - Context

**Gathered:** 2026-08-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 1 establishes the Rust workspace, invariant enforcement infrastructure, and programmatic fixture/mutation test generators. It delivers:
- Multi-crate Cargo workspace layout (`gitctx-core`, `gitctx-index`, `gitctx-query`, `gitctx-cli`, `gitctx-mcp`, `gitctx-eval`, `xtask`) with strict CI checks (`fmt`, `clippy -D warnings`, fast tests).
- Offline test harness verifying Invariant I1 (zero network calls during indexing/search) with socket interception and Linux network namespace isolation.
- Programmatic synthetic Git repository builder (`tests/fixtures/mkrepo.rs`) supporting edge-case Git topologies (linear commits, file renames, cross-file moves, squash merges, evil merges, syntax errors).
- Synthetic mutation testing framework (`gitctx-eval mutate`) implementing transformation classes M1-M10 with structured ground-truth lineage manifests.
- Configuration hashing foundation (`config_hash`) using BLAKE3 for index version verification.

</domain>

<decisions>
## Implementation Decisions

### Workspace Layout & Error Handling
- **D-01:** Use `thiserror` in internal crates (`gitctx-core`, `gitctx-index`, `gitctx-query`, `gitctx-mcp`, `gitctx-eval`) with domain-specific typed error enums, and `anyhow` for top-level binaries (`gitctx-cli`, `xtask`) for rich error context. — **Reversibility:** costly — changing error representations later touches error handling across all crate APIs.
- **D-02:** Target Rust 2021 edition with MSRV 1.82+, workspace-level `[workspace.lints.clippy]` enforcing `-D warnings`, `clippy::all`, and scoped `pedantic` rules, with `#![forbid(unsafe_code)]` on safe crates. — **Reversibility:** reversible.
- **D-03:** Strict layered crate hierarchy: `gitctx-core` (domain types, AST models, traits — no DB/search deps) → `gitctx-index` (SQLite, Tantivy, vector storage) → `gitctx-query` (RRF, co-change, ranking) → `gitctx-cli` / `gitctx-mcp` (entry points), plus `gitctx-eval` (benchmarks, mutations, evaluation). — **Reversibility:** costly — defines core data isolation and compile boundaries.
- **D-04:** Pure Rust `xtask` binary using `clap` to provide cross-platform runner commands (`cargo xtask lint`, `cargo xtask test-offline`, `cargo xtask test-invariants`, `cargo xtask fixtures`, `cargo xtask eval`). — **Reversibility:** reversible.

### Synthetic Fixture Generator (`mkrepo`)
- **D-05:** Programmatic `git2` builder using fixed deterministic timestamps (starting at a base epoch `1700000000 + i*60`), consistent author signatures, and isolated temporary directories via `tempfile`. — **Reversibility:** costly — test determinism and fixture generation depend on timestamp/author generation.
- **D-06:** Fluent `RepoBuilder` API with built-in scenario presets: `linear_history()`, `file_rename()`, `cross_file_move()`, `squash_merge()`, `evil_merge()`, and `broken_syntax()`. — **Reversibility:** reversible.
- **D-07:** Multi-language synthetic fixtures (Python + Rust + JavaScript) to test multi-grammar handling upfront. — **Reversibility:** costly — fixture generators will generate realistic code in Python, Rust, and JavaScript.
- **D-08:** Fresh RAII-managed `tempfile::TempDir` per test run (auto-deleted on drop), with `cargo xtask fixtures --dump-dir <path>` to export them for debugging. — **Reversibility:** reversible.

### Mutation Framework (M1-M10)
- **D-09:** Programmatic transformation engine in `gitctx-eval` that applies AST/text modifications for each class (M1-M10) directly to target functions and creates Git commits with known ground-truth lineage links. — **Reversibility:** costly — foundational for Phase 5 lineage validation.
- **D-10:** Structured JSON manifest (`lineage_ground_truth.json`) recording commit SHA, transformation class (M1-M10), source/target symbols (path, qualname, line range), and expected relation (`EXACT`, `RENAMED`, `MOVED`, `SPLIT_FROM`, `DECOY`). — **Reversibility:** one-way — ground truth schema consumed by evaluation tools and benchmarks.
- **D-11:** Support both isolated single-mutation fixtures (unit tests) and multi-step deterministic composite chains (e.g., rename → signature tweak → cross-file move across 3 sequential commits). — **Reversibility:** reversible.
- **D-12:** Expose subcommands in `gitctx-eval` binary (`gitctx-eval mutate --class <M1-M10|all> --chain-depth <N> --out <dir>`) with matching `cargo xtask eval mutate` task. — **Reversibility:** reversible.

### Offline Invariant (I1) Enforcement
- **D-13:** Dual-tier enforcement: Cross-platform socket/network interception in unit/integration tests (fails if any outbound connection is attempted) + OS-level network namespace isolation (`unshare -n`) in Linux CI runners. — **Reversibility:** costly — core testing harness for Invariant I1 across all phases.
- **D-14:** Two-phase embedding model rule: Explicit ahead-of-time setup command (`cargo xtask download-models` or pre-pinned local model directory) with runtime embedding initialization strictly configured in offline mode, failing with actionable error if model files are missing. — **Reversibility:** costly — prevents silent network calls at runtime.
- **D-15:** Canonical canonicalized JSON representation of all indexing parameters (`schema_version`, model ID, tokenizer settings, commit size thresholds, parser version) hashed using BLAKE3 and embedded in SQLite metadata, rejecting queries against mismatched indexes. — **Reversibility:** one-way — index format and invalidation contract.
- **D-16:** Dedicated `cargo xtask test-invariants` command orchestrating offline execution tests, socket traps, and fixture consistency checks with structured summary reporting. — **Reversibility:** reversible.

### the agent's Discretion
- Exact directory naming under `tests/fixtures/` and internal helper module partitioning.
- CLI argument flags and formatting details for `xtask` subcommands.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Architecture & Specification
- `reference_docs/gpai-20260816-gitctx-Complete-Step-by-Step-Implementation-Plan-w.md` — Complete step-by-step implementation plan and architectural specification for gitctx
- `reference_docs/20260816-Implementation-Plan-Review-Suggestions.md` — Architectural reviews, invariant definitions (I1-I8), and implementation guidelines

### Project Requirements & Roadmap
- `.planning/PROJECT.md` — Project definition, core values, active requirements, and constraints
- `.planning/REQUIREMENTS.md` §INVAR, §FIXT — Requirements INVAR-01, INVAR-02, FIXT-01, FIXT-02
- `.planning/ROADMAP.md` §Phase 1 — Phase 1 goals, plans, and success criteria

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- Fresh repository: No prior source code exists yet.

### Established Patterns
- Cargo workspace architecture with dedicated `xtask` crate pattern.
- Deterministic commit history construction using `git2` and `tempfile`.

### Integration Points
- Root `Cargo.toml` workspace declaring `members = ["crates/*", "xtask"]`.
- `.github/workflows/ci.yml` configuring `fmt`, `clippy -D warnings`, and `test` runners.

</code_context>

<specifics>
## Specific Ideas

- Invariant I1 (Offline): Zero network socket creation during indexing/retrieval paths.
- Multi-language synthetic fixtures: Python, Rust, and JavaScript examples in fixture repos.
- Mutation classes M1-M10: In-place rename, cross-file move, move+rename, signature change, reformatting, local var rename, function split, decoy delete+add, directory move, function copy.

</specifics>

<deferred>
## Deferred Ideas

- None — discussion stayed strictly within Phase 1 scope.

</deferred>

---

*Phase: 1-Project Foundation, Invariants & Fixture Scaffolding*
*Context gathered: 2026-08-16*

# Phase 1: Project Foundation, Invariants & Fixture Scaffolding - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-08-16
**Phase:** 1-Project Foundation, Invariants & Fixture Scaffolding
**Areas discussed:** Workspace layout & architecture, Synthetic fixture generator (mkrepo), Mutation framework (M1-M10), Offline invariant (I1) enforcement

---

## Workspace Layout, Crate Boundaries, and Error Handling Architecture

| Option | Description | Selected |
|--------|-------------|----------|
| `thiserror` + `anyhow` | `thiserror` in internal crates (`gitctx-core`, `gitctx-index`, etc.) with domain-specific typed enums, and `anyhow` for top-level binaries (`gitctx-cli`, `xtask`) | ✓ |
| Universal `anyhow` | `anyhow` universally across all crates | |
| Universal `thiserror` | `thiserror` across all crates including binaries | |
| the agent decides | Standard idiomatic Rust error handling | |

**User's choice:** `thiserror` in internal crates with typed error enums and `anyhow` in CLI/xtask binaries.

| Option | Description | Selected |
|--------|-------------|----------|
| Strict 2021 lints | Rust 2021 edition, MSRV 1.82+, workspace-level `[workspace.lints.clippy]` enforcing `-D warnings`, `clippy::all`, and scoped `pedantic` rules with `#![forbid(unsafe_code)]` on high-level crates | ✓ |
| Standard clippy | Rust 2021 with standard `clippy::all` and `-D warnings`, keeping pedantic lints off | |
| Rust 2024 edition | Rust 2024 edition (nightly/latest features) with strict linting | |
| the agent decides | Idiomatic strict workspace configuration | |

**User's choice:** Rust 2021 edition, MSRV 1.82+, workspace `clippy` lints with `-D warnings` and `#![forbid(unsafe_code)]`.

| Option | Description | Selected |
|--------|-------------|----------|
| Layered hierarchy | Strict layered hierarchy: `gitctx-core` → `gitctx-index` → `gitctx-query` → `gitctx-cli` / `gitctx-mcp`, plus `gitctx-eval` | ✓ |
| Two-crate layout | `gitctx-lib` and `gitctx-bin` | |
| Core-centric model | `gitctx-core` bundles `git2` and `tree-sitter` extraction directly | |
| the agent decides | Modularity and fast compile times | |

**User's choice:** Strict layered hierarchy (`gitctx-core` → `gitctx-index` → `gitctx-query` → `gitctx-cli`/`gitctx-mcp`, plus `gitctx-eval`).

| Option | Description | Selected |
|--------|-------------|----------|
| Rust `xtask` binary | Pure Rust `xtask` crate using `clap` to provide cross-platform runner commands | ✓ |
| Shell scripts | Shell/PowerShell script suite without Rust `xtask` | |
| Justfile | Use `just` (`Justfile`) as task runner | |
| the agent decides | Cross-platform ergonomics | |

**User's choice:** Pure Rust `xtask` crate using `clap`.

---

## Synthetic Fixture Generator (mkrepo) Design

| Option | Description | Selected |
|--------|-------------|----------|
| Programmatic `git2` + fixed timestamps | Programmatic `git2` builder using fixed deterministic timestamps (`1700000000 + i*60`), consistent author signatures, and isolated temporary directories via `tempfile` | ✓ |
| Spawn `git` CLI | Subprocess calls to system `git` with environment variables | |
| In-memory bare repos | In-memory bare repositories only (`git2::Repository::init_bare`) | |
| the agent decides | Fast, deterministic fixture generation | |

**User's choice:** Programmatic `git2` builder with fixed timestamps and `tempfile`.

| Option | Description | Selected |
|--------|-------------|----------|
| Fluent `RepoBuilder` presets | Fluent `RepoBuilder` API with built-in scenario presets: `linear_history()`, `file_rename()`, `cross_file_move()`, `squash_merge()`, `evil_merge()`, `broken_syntax()` | ✓ |
| Low-level imperative API | Low-level builder without presets | |
| Declarative scenario files | TOML/YAML scenario files | |
| the agent decides | Ergonomic and reusable fixture definitions | |

**User's choice:** Fluent `RepoBuilder` API with named scenario presets.

| Option | Description | Selected |
|--------|-------------|----------|
| Multi-language fixtures | Multi-language synthetic fixtures (Python + Rust + JavaScript) to test multi-grammar handling upfront | ✓ |
| Python-only | Realistic synthetic Python modules matching v1 parser targets | |
| Abstract pseudocode | Minimal abstract pseudocode snippets | |
| the agent decides | Representative synthetic code | |

**User's choice:** Multi-language synthetic fixtures (Python + Rust + JavaScript).

| Option | Description | Selected |
|--------|-------------|----------|
| RAII TempDir + xtask export | Fresh RAII-managed `tempfile::TempDir` per test run (auto-deleted on drop), with `cargo xtask fixtures --dump-dir <path>` to export for debugging | ✓ |
| Pre-generated tar.gz | Pre-generated `.tar.gz` bundle committed to version control | |
| Shared persistent cache | Shared persistent cache under `target/fixtures/` | |
| the agent decides | Balance between isolation, test speed, and debuggability | |

**User's choice:** Fresh RAII `tempfile::TempDir` with xtask dump-dir export option.

---

## Mutation Framework (M1-M10) Execution & Ground Truth

| Option | Description | Selected |
|--------|-------------|----------|
| Programmatic mutator | Programmatic transformation engine in `gitctx-eval` applying AST/text modifications for M1-M10 and creating Git commits with ground-truth lineage links | ✓ |
| Regex mutator | Regex-based search-and-replace mutator | |
| Pre-recorded patches | Pre-recorded `.patch` / `.bundle` files replayed via `git am` | |
| the agent decides | Robust automated mutation generation | |

**User's choice:** Programmatic transformation engine in `gitctx-eval`.

| Option | Description | Selected |
|--------|-------------|----------|
| Structured JSON manifest | Structured JSON manifest (`lineage_ground_truth.json`) recording commit SHA, transformation class (M1-M10), source/target symbols, and expected relations | ✓ |
| Commit message trailers | Git commit message trailers parsed during evaluation | |
| In-memory assertions only | Rust test assertions only without file serialization | |
| the agent decides | Standardized ground-truth schema | |

**User's choice:** Structured JSON manifest (`lineage_ground_truth.json`).

| Option | Description | Selected |
|--------|-------------|----------|
| Single + Composite chains | Support both isolated single-mutation fixtures and multi-step deterministic composite chains | ✓ |
| Single-step only | Single-mutation fixtures only | |
| Fuzz-driven random chains | Random mutation chains with fixed seed | |
| the agent decides | Balance between unit test clarity and realistic history | |

**User's choice:** Support both isolated single-mutation fixtures and multi-step composite chains.

| Option | Description | Selected |
|--------|-------------|----------|
| `gitctx-eval` CLI + xtask | Subcommands in `gitctx-eval` binary with matching `cargo xtask eval mutate` task | ✓ |
| Internal library only | No dedicated CLI interface | |
| Python script | `eval/mutate.py` | |
| the agent decides | Best developer experience | |

**User's choice:** `gitctx-eval` CLI subcommand with `cargo xtask eval mutate` task.

---

## Offline Invariant (I1) Enforcement

| Option | Description | Selected |
|--------|-------------|----------|
| Dual-tier socket + namespace | Cross-platform socket/network interception in tests + OS-level network namespace isolation (`unshare -n`) in Linux CI | ✓ |
| Crate dependency audit | Static check asserting zero networking crates exist | |
| Linux-only runner | `unshare -n cargo test` skipping Windows/macOS | |
| the agent decides | Ironclad offline verification | |

**User's choice:** Dual-tier socket/network interception and Linux network namespace isolation.

| Option | Description | Selected |
|--------|-------------|----------|
| Explicit pre-download + offline runtime | Explicit setup command (`cargo xtask download-models`) with runtime embedding strictly configured offline, failing if weights missing | ✓ |
| Bundle weights | Bundle quantized model weights in repository assets | |
| Mock vectors in test | Deterministic dummy vectors in test, requiring `--model-path` for CLI | |
| the agent decides | Clean offline compliance | |

**User's choice:** Two-phase model weight acquisition: explicit `xtask` pre-download and strict offline runtime.

| Option | Description | Selected |
|--------|-------------|----------|
| BLAKE3 JSON config hash | Canonical canonicalized JSON representation of parameters hashed with BLAKE3 and embedded in SQLite metadata | ✓ |
| Simple schema_version | Numeric integer column only | |
| External sidecar file | `.gitctx/index.meta` TOML file | |
| the agent decides | Robust determinism | |

**User's choice:** Canonical JSON parameter representation hashed via BLAKE3 in index metadata.

| Option | Description | Selected |
|--------|-------------|----------|
| `cargo xtask test-invariants` | Dedicated xtask command orchestrating offline tests, socket traps, fixture consistency checks with structured summary | ✓ |
| Standard `cargo test` | Inline Rust `#[test]` assertions without xtask wrapper | |
| Shell scripts | `tests/verify_invariants.sh` | |
| the agent decides | Clear CI and local testing ergonomics | |

**User's choice:** Dedicated `cargo xtask test-invariants` command.

---

## the agent's Discretion

- Internal module structure and helper distribution within `mkrepo.rs` and `gitctx-eval`.
- Specific CLI flag aliases for `xtask` subcommands.

## Deferred Ideas

- None.

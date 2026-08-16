# Phase 1: Project Foundation, Invariants & Fixture Scaffolding - Research

**Researched:** 2026-08-16
**Domain:** Rust workspace scaffolding, libgit2 fixture synthesis, AST mutation testing, offline invariant enforcement
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Use `thiserror` in internal crates (`gitctx-core`, `gitctx-index`, `gitctx-query`, `gitctx-mcp`, `gitctx-eval`) with domain-specific typed error enums, and `anyhow` for top-level binaries (`gitctx-cli`, `xtask`) for rich error context. — **Reversibility:** costly
- **D-02:** Target Rust 2021 edition with MSRV 1.82+, workspace-level `[workspace.lints.clippy]` enforcing `-D warnings`, `clippy::all`, and scoped `pedantic` rules, with `#![forbid(unsafe_code)]` on safe crates. — **Reversibility:** reversible
- **D-03:** Strict layered crate hierarchy: `gitctx-core` (domain types, AST models, traits — no DB/search deps) → `gitctx-index` (SQLite, Tantivy, vector storage) → `gitctx-query` (RRF, co-change, ranking) → `gitctx-cli` / `gitctx-mcp` (entry points), plus `gitctx-eval` (benchmarks, mutations, evaluation). — **Reversibility:** costly
- **D-04:** Pure Rust `xtask` binary using `clap` to provide cross-platform runner commands (`cargo xtask lint`, `cargo xtask test-offline`, `cargo xtask test-invariants`, `cargo xtask fixtures`, `cargo xtask eval`). — **Reversibility:** reversible
- **D-05:** Programmatic `git2` builder using fixed deterministic timestamps (starting at a base epoch `1700000000 + i*60`), consistent author signatures, and isolated temporary directories via `tempfile`. — **Reversibility:** costly
- **D-06:** Fluent `RepoBuilder` API with built-in scenario presets: `linear_history()`, `file_rename()`, `cross_file_move()`, `squash_merge()`, `evil_merge()`, and `broken_syntax()`. — **Reversibility:** reversible
- **D-07:** Multi-language synthetic fixtures (Python + Rust + JavaScript) to test multi-grammar handling upfront. — **Reversibility:** costly
- **D-08:** Fresh RAII-managed `tempfile::TempDir` per test run (auto-deleted on drop), with `cargo xtask fixtures --dump-dir <path>` to export them for debugging. — **Reversibility:** reversible
- **D-09:** Programmatic transformation engine in `gitctx-eval` that applies AST/text modifications for each class (M1-M10) directly to target functions and creates Git commits with known ground-truth lineage links. — **Reversibility:** costly
- **D-10:** Structured JSON manifest (`lineage_ground_truth.json`) recording commit SHA, transformation class (M1-M10), source/target symbols (path, qualname, line range), and expected relation (`EXACT`, `RENAMED`, `MOVED`, `SPLIT_FROM`, `DECOY`). — **Reversibility:** one-way
- **D-11:** Support both isolated single-mutation fixtures (unit tests) and multi-step deterministic composite chains (e.g., rename → signature tweak → cross-file move across 3 sequential commits). — **Reversibility:** reversible
- **D-12:** Expose subcommands in `gitctx-eval` binary (`gitctx-eval mutate --class <M1-M10|all> --chain-depth <N> --out <dir>`) with matching `cargo xtask eval mutate` task. — **Reversibility:** reversible
- **D-13:** Dual-tier enforcement: Cross-platform socket/network interception in unit/integration tests (fails if any outbound connection is attempted) + OS-level network namespace isolation (`unshare -n`) in Linux CI runners. — **Reversibility:** costly
- **D-14:** Two-phase embedding model rule: Explicit ahead-of-time setup command (`cargo xtask download-models` or pre-pinned local model directory) with runtime embedding initialization strictly configured in offline mode, failing with actionable error if model files are missing. — **Reversibility:** costly
- **D-15:** Canonical canonicalized JSON representation of all indexing parameters (`schema_version`, model ID, tokenizer settings, commit size thresholds, parser version) hashed using BLAKE3 and embedded in SQLite metadata, rejecting queries against mismatched indexes. — **Reversibility:** one-way
- **D-16:** Dedicated `cargo xtask test-invariants` command orchestrating offline execution tests, socket traps, and fixture consistency checks with structured summary reporting. — **Reversibility:** reversible

### the agent's Discretion
- Exact directory naming under `tests/fixtures/` and internal helper module partitioning.
- CLI argument flags and formatting details for `xtask` subcommands.

### Deferred Ideas (OUT OF SCOPE)
- None — discussion stayed strictly within Phase 1 scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| **INVAR-01** | Rust workspace initialized (`gitctx-core`, `gitctx-index`, `gitctx-query`, `gitctx-cli`, `gitctx-mcp`, `gitctx-eval`, `xtask`) with CI enforcing `fmt`, `clippy -D warnings`, and fast unit tests. | Multi-crate Cargo workspace layout, centralized dependency table, workspace clippy configuration, and GitHub Actions CI workflow. |
| **INVAR-02** | Offline enforcement (Invariant I1) verified via network-isolated test harness. | Socket-trapping mock interceptor in tests + Linux `unshare -n` test task in xtask + dependency audit asserting zero networking crates. |
| **FIXT-01** | Synthetic repository generator (`tests/fixtures/mkrepo.rs`) producing isolated Git histories (linear commits, merges, evil merges, squash-merges, renames, moves, syntax errors). | Programmatic `libgit2` repo builder with deterministic timestamps (`1700000000 + i*60`), consistent signatures, fluent API, and scenario presets. |
| **FIXT-02** | Mutation testing harness (`gitctx-eval mutate`) implementing transformation classes M1-M10. | Programmatic mutation engine in `gitctx-eval` applying M1-M10 transformations and serializing `lineage_ground_truth.json` manifests. |
</phase_requirements>

## Summary

Phase 1 establishes the foundational engineering scaffolding for the entire `gitctx` project. It provides:
1. A multi-crate Rust workspace configured with strict compiler lints, clean layering, and cross-platform developer tooling (`xtask`).
2. An offline verification harness ensuring Invariant I1 (100% offline indexing and search) is mechanically verified in local development and CI.
3. A programmatic synthetic Git fixture engine (`mkrepo.rs`) capable of producing deterministic Git histories covering linear commits, renames, moves, merges, evil merges, and syntax failures across Python, Rust, and JavaScript.
4. A synthetic mutation testing engine (`gitctx-eval mutate`) implementing transformation classes M1 through M10 with structured ground-truth lineage manifests to serve as an immutable oracle for subsequent phase validation.

**Primary recommendation:** Build pure, layered crates with zero unneeded dependencies in `gitctx-core`, vendored `git2` for reliable cross-platform compilation, and explicit xtask automation.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Workspace Layout & Lints | Cargo Workspace (`Cargo.toml`) | CI (`.github/workflows/ci.yml`) | Enforces compilation, MSRV 1.82+, and lint hygiene at build time |
| Domain Models & AST Types | `gitctx-core` | `gitctx-index`, `gitctx-query` | Pure data types and traits without heavy I/O or storage dependencies |
| Fixture Synthesis | `tests/fixtures/mkrepo.rs` | `xtask fixtures` | Programmatic in-memory/tempfile Git repository construction via `git2` |
| Mutation Testing Engine | `gitctx-eval` (`mutator`) | `xtask eval mutate` | Generates M1-M10 transformations and canonical JSON ground-truth manifests |
| Invariant Verification | `xtask` (`test-offline`, `test-invariants`) | CI Runner (`unshare -n`) | Mechanically asserts zero network egress and config hash determinism |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `git2` | `0.19.0` | libgit2 bindings for commit walking, trees, blobs, diffs | Official libgit2 bindings; static vendoring ensures reproducible cross-platform builds |
| `tempfile` | `3.16.0` | RAII temporary directory management | De facto standard for safe temporary directories in test execution |
| `clap` | `4.5.26` | Command-line argument parsing in `gitctx-cli` and `xtask` | Industry standard declarative and derive-based CLI parsing in Rust |
| `thiserror` | `2.0.11` | Typed domain error enums in library crates | Standard Rust error derivation library by dtolnay |
| `anyhow` | `1.0.95` | Contextual application error handling in binaries | Idiomatic Rust binary error reporting |
| `blake3` | `1.5.5` | Cryptographic configuration hashing (`config_hash`) | Ultra-fast SIMD-accelerated cryptographic hash function |
| `serde` / `serde_json` | `1.0.217` / `1.0.137` | JSON serialization for manifests and config hashes | De facto Rust serialization standard |
| `tracing` / `tracing-subscriber` | `0.1.41` / `0.3.19` | Structured diagnostic logging routed strictly to stderr | Standard Tokio tracing framework |

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `git2` | crates.io | 10 yrs | > 40M | github.com/rust-lang/git2-rs | [OK] | Approved |
| `tempfile` | crates.io | 9 yrs | > 100M | github.com/Stebalien/tempfile | [OK] | Approved |
| `clap` | crates.io | 9 yrs | > 150M | github.com/clap-rs/clap | [OK] | Approved |
| `thiserror` | crates.io | 5 yrs | > 120M | github.com/dtolnay/thiserror | [OK] | Approved |
| `anyhow` | crates.io | 5 yrs | > 140M | github.com/dtolnay/anyhow | [OK] | Approved |
| `blake3` | crates.io | 5 yrs | > 35M | github.com/BLAKE3-team/BLAKE3 | [OK] | Approved |
| `serde` | crates.io | 10 yrs | > 250M | github.com/serde-rs/serde | [OK] | Approved |
| `serde_json` | crates.io | 10 yrs | > 200M | github.com/serde-rs/json | [OK] | Approved |
| `tracing` | crates.io | 5 yrs | > 180M | github.com/tokio-rs/tracing | [OK] | Approved |

## Architecture Patterns

### Recommended Project Structure
```
gitctx/
├── Cargo.toml                      # Workspace root configuration & lints
├── .github/
│   └── workflows/
│       └── ci.yml                  # CI: fmt, clippy -D warnings, test, xtask test-invariants
├── crates/
│   ├── gitctx-core/                # Pure domain models, AST types, errors
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── model.rs            # Commit, FunctionTouch, LineageId, Citation
│   │       ├── error.rs            # CoreError (thiserror)
│   │       ├── qualname.rs         # Qualified name conventions
│   │       └── config.rs           # ConfigHash & canonical serialization
│   ├── gitctx-index/               # SQLite storage, Tantivy BM25, mmap vectors (scaffolding)
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── gitctx-query/               # Hybrid retrieval, co-change graph (scaffolding)
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── gitctx-cli/                 # Main CLI binary
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   ├── gitctx-mcp/                 # Model Context Protocol stdio server
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   └── gitctx-eval/                # Evaluation runner, mutator (M1-M10), benchmark metrics
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── main.rs
│           ├── mutator/            # M1-M10 transformation engine
│           │   ├── mod.rs
│           │   ├── transformations.rs
│           │   └── manifest.rs     # lineage_ground_truth.json schema
│           └── metrics/            # Recall@k, MRR, bootstrap CI
├── tests/
│   └── fixtures/
│       ├── mod.rs
│       ├── mkrepo.rs               # Programmatic git2 repository generator
│       └── scenarios.rs            # Named scenario presets (linear, rename, move, merges, syntax)
└── xtask/                          # Developer automation runner
    ├── Cargo.toml
    └── src/
        ├── main.rs
        ├── tasks/
        │   ├── lint.rs
        │   ├── test_offline.rs
        │   ├── test_invariants.rs
        │   ├── fixtures.rs
        │   └── download_models.rs
        └── util.rs
```

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Git Object Manipulation | Custom git CLI subprocess wrappers or packfile parsers | `git2` (`libgit2`) | Fast in-process execution, safe memory models, avoids shell quoting and subprocess overhead |
| Error Derivations | Manual `impl std::fmt::Display` and `std::error::Error` boilerplate | `thiserror` | Eliminates error formatting boilerplate while guaranteeing standard trait compliance |
| CLI Argument Parsing | Manual `std::env::args()` token parsing | `clap` (derive API) | Robust flag validation, typed arguments, `--help` generation, and shell completions |
| Configuration Hashing | Custom string concatenation and ad-hoc hashing | `blake3` over canonical JSON | Cryptographically secure, deterministic byte order, SIMD-accelerated performance |

## Common Pitfalls

### Pitfall 1: Non-Deterministic Commit Hashes in Fixtures
**What goes wrong:** Repeated test runs generate different commit OIDs, breaking cached fixture assertions.
**Why it happens:** Relying on current system time (`std::time::SystemTime::now()`) or host git configuration (`user.name`/`user.email`).
**How to avoid:** Use fixed sequential timestamps (`1700000000 + i*60`) and static author signatures (`"gitctx-test <test@gitctx.local>"`).

### Pitfall 2: Silent Network Downloads in Sub-dependencies
**What goes wrong:** FastEmbed or tokenizer crates attempt to fetch missing model weights from HuggingFace at runtime.
**Why it happens:** Default library configurations enabling auto-download fallback.
**How to avoid:** Enforce offline mode in library initialization and provide explicit ahead-of-time download tasks (`cargo xtask download-models`).

### Pitfall 3: Platform Incompatibilities with libgit2
**What goes wrong:** Missing system libgit2 or CMake errors during `cargo build` on Windows.
**Why it happens:** Dynamic linking against system C libraries that vary by host.
**How to avoid:** Enable `features = ["vendored-libgit2"]` in `git2` Cargo dependency.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust native test harness (`cargo test`) |
| Config file | `Cargo.toml` |
| Quick run command | `cargo test --workspace` |
| Full suite command | `cargo xtask test-invariants` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| **INVAR-01** | Workspace compiles cleanly, fmt and clippy pass | unit/lint | `cargo xtask lint && cargo test --workspace` | ❌ Wave 0 |
| **INVAR-02** | Test suite runs with zero network egress | integration | `cargo xtask test-offline` | ❌ Wave 0 |
| **FIXT-01** | `mkrepo.rs` generates 6 scenario presets deterministically | integration | `cargo test -p gitctx-eval --test fixture_tests` | ❌ Wave 0 |
| **FIXT-02** | `gitctx-eval mutate` produces M1-M10 repos with valid manifests | integration | `cargo test -p gitctx-eval --test mutation_tests` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --workspace`
- **Per wave merge:** `cargo xtask lint && cargo xtask test-invariants`
- **Phase gate:** Full suite green before phase sign-off.

### Wave 0 Gaps
- [ ] `Cargo.toml` workspace root configuration
- [ ] `crates/gitctx-core/src/lib.rs`, `model.rs`, `error.rs`, `config.rs`
- [ ] `crates/gitctx-eval/src/mutator/` (M1-M10 engine and manifest schema)
- [ ] `tests/fixtures/mkrepo.rs` (deterministic git2 repository generator)
- [ ] `xtask/src/` (`lint`, `test_offline`, `test_invariants`, `fixtures`)
- [ ] `.github/workflows/ci.yml` CI workflow

## Security Domain

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V5 Input Validation | Yes | Strongly typed Rust structs with Serde deserialization validation |
| V6 Cryptography | Yes | BLAKE3 hashing for configuration verification; no custom crypto |

---

*Phase: 1-Project Foundation, Invariants & Fixture Scaffolding*
*Research date: 2026-08-16*

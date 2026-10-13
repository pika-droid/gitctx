# gitctx — Git History Intelligence

[![CI](https://github.com/pika-droid/gitctx/actions/workflows/ci.yml/badge.svg)](https://github.com/pika-droid/gitctx/actions/workflows/ci.yml)

> **Deterministic, citation-backed Git history facts for LLMs and developers.** Zero network dependencies. Zero LLM calls. Pure offline indexing and retrieval.

`gitctx` is a local-first Rust tool that transforms Git history into a searchable, structured knowledge base. It tracks **why** code changed, **who** changed it, and **what** the historical coupling risks are — with complete function-level lineage tracking across renames and file moves.

---

## Key Features

| Feature | Description |
|---------|-------------|
| 🔍 **Natural-Language Search** | Query Git history with why-questions ("why was auth changed?") via template-rendered pseudo-documents |
| 🧬 **Function Lineage** | Track function identity across renames, moves, and copies using 4-tier MinHash resolution and a $16 \times 8$ LSH band inverted index for $O(1)$ candidate lookup |
| 📊 **Change Impact** | Statistical co-change coupling (support, confidence, lift) with 180-day time-decay weighting, precomputed into Schema v2 tables for `< 2 ms` lookups via `ImpactService` |
| ⚡ **Batched Indexing (`IndexSink`)** | Chronological (`root -> HEAD`) ingestion pipeline with 128-chunk ONNX batching, dual-write SQLite + `mmap` vector persistence, and crash-safe staged `.bak` atomic swaps |
| 🤖 **MCP Server** | First-class Model Context Protocol integration with warm-cached ONNX embedding sessions for sub-5ms query latency |
| 📴 **Fully Offline** | Zero network calls during indexing and retrieval (Invariant I1) |
| 🔒 **Deterministic** | Byte-identical outputs for identical inputs (Invariant I2) |

---

## Architecture

```mermaid
graph TB
    subgraph "Extraction Layer"
        GIT[("Git Repository<br/>(libgit2)")] --> WALK["Chronological First-Parent<br/>Walker (root → HEAD)"]
        WALK --> PARSE["Tree-Sitter AST Parser<br/>(LanguageRegistry · 9 languages)"]
        PARSE --> TMPL["Template Chunk<br/>Renderer"]
    end

    subgraph "Lineage Engine"
        TMPL --> LIN["4-Tier MinHash<br/>Lineage Resolver"]
        LIN -->|"Tier 1"| EXACT["Exact Key Match"]
        LIN -->|"Tier 2"| SAME["Same-File MinHash"]
        LIN -->|"Tier 3"| RENAME["Git Rename Map"]
        LIN -->|"Tier 4"| CROSS["LSH Band Inverted Index<br/>(16 bands × 8 rows)"]
    end

    subgraph "Storage & Retrieval (IndexSink)"
        LIN --> SINK["IndexSink<br/>(128-Chunk Batch Pipeline)"]
        TMPL --> SINK
        SINK --> SQL[("SQLite Schema v2<br/>(WAL + Staged .bak Swap)")]
        SINK --> BM25["Tantivy BM25<br/>(Incremental Code Writer)"]
        SINK --> DENSE["FastEmbed Batched ONNX<br/>(Dual-Write SQLite + mmap f32)"]
        BM25 --> RRF["RRF Fusion<br/>(k=60)"]
        DENSE --> RRF
        SQL --> CHAIN["Deterministic<br/>Chain Assembly"]
    end

    subgraph "Co-Change Analysis"
        SQL --> COCHANGE["Single-Scan Bulk<br/>Co-Change Extractor"]
        COCHANGE --> METRICS[("Precomputed Metrics<br/>(File & Function Tables)")]
        COCHANGE --> GRAPH["petgraph<br/>Coupling Graph"]
        METRICS --> IMPACT["ImpactService<br/>(< 2 ms Risk Scorer)"]
        GRAPH --> IMPACT
    end

    subgraph "User Surfaces"
        RRF --> CLI["CLI<br/>(--format json / table)"]
        RRF --> MCP["MCP Server<br/>(Warm ONNX Cache · stdio JSON-RPC)"]
        CHAIN --> CLI
        CHAIN --> MCP
        IMPACT --> CLI
        IMPACT --> MCP
    end
```

---

## Installation

### Prerequisites

- **Rust toolchain** (1.85+): Install via [rustup](https://rustup.rs/)
- **ONNX Runtime** (1.22.0): Required by the `fastembed` dense embedding engine. Download automatically:

```bash
# Download the correct ONNX Runtime shared library for your OS
cargo xtask download-ort-runtime

# The command prints the ORT_DYLIB_PATH — set it in your shell profile:
# Linux/macOS:
export ORT_DYLIB_PATH="$(pwd)/.ort/libonnxruntime.so.1.22.0"  # adjust for your OS

# Windows (PowerShell):
$env:ORT_DYLIB_PATH = "$PWD\.ort\onnxruntime.dll"
```

### Build from Source

```bash
git clone https://github.com/pika-droid/gitctx.git
cd gitctx

# Download ONNX Runtime for your platform
cargo xtask download-ort-runtime

# Build release binaries (produces `gitctx` and `gitctx-mcp`)
cargo build --release

# Verify installation
./target/release/gitctx --version
```

### Index a Repository

```bash
# Index the current repository
gitctx index .

# Index only the most recent N commits (for fast profiling or large repos)
gitctx index . --max-commits 1000
```

### Search History

```bash
# Natural-language why-questions
gitctx search "why was the authentication flow changed?"

# Search with JSON output
gitctx search "connection pool timeout" --format json
```

### Track Function Evolution

```bash
# Full chronological timeline of a function
gitctx evolution "process_request" --format table

# JSON output for programmatic use
gitctx evolution "handle_auth" --format json
```

### Analyze Change Impact

```bash
# File-level co-change coupling analysis
gitctx impact src/auth.rs

# Function-level coupling & risk scoring (supports qualified or bare names)
gitctx impact auth::handle_login --format json
gitctx impact process_request --format table

# Global repository coupling hotspots (omit target)
gitctx impact
```

### MCP Server (AI Assistant Integration)

```bash
# Start the stdio MCP server
gitctx-mcp

# Or configure in your agent's MCP settings:
```

```json
{
  "mcpServers": {
    "gitctx": {
      "command": "gitctx-mcp",
      "args": []
    }
  }
}
```

**Available MCP Tools:**

| Tool | Purpose |
|------|---------|
| `search_history` | Search indexed Git history for functions, changes, and commits (backed by warm ONNX embedder) |
| `function_evolution` | Track complete evolutionary history of a function across renames/moves |
| `change_impact` | Analyze risk and co-change coupling for a file or function via precomputed Schema v2 metrics |
| `who_knows_this` | Find top contributors who modified a function or file |
| `commit_detail` | Show detailed commit info with all functions touched |

---

## CLI Commands

| Command | Description |
|---------|-------------|
| `gitctx index [path] [--max-commits <N>]` | Index a Git repository (chronological traversal, batched embeddings, precomputed co-change) |
| `gitctx search <query>` | Search history with natural language (BM25 + dense RRF fusion) |
| `gitctx evolution <function>` | Track function lineage timeline across renames, moves, and copies |
| `gitctx show <commit>` | Show detailed commit with function touches |
| `gitctx impact [target]` | Analyze co-change coupling and risk for a file, function, or entire repo |
| `gitctx blame <file_path>` | Find top contributors / ownership for a file or function |
| `gitctx doctor` | Diagnose index health, Schema v2 tables, and configuration (`index_state`) |
| `gitctx init` | Generate agent configuration files (`AGENTS.md`, skills) |

All commands support `-f / --format json` for structured output and `-f / --format table` (default) for human-readable rendering.

---

## Project Invariants

These properties are mechanically enforced by the test suite and CI:

| ID | Invariant | Enforcement |
|----|-----------|-------------|
| **I1** | Zero network calls during index/search | `unshare -n` / mock network test |
| **I2** | Byte-identical outputs for identical inputs | Double-indexing SHA comparison |
| **I3** | Incremental ≡ full index equivalence | Incremental vs full state diff |
| **I4** | Non-null citation metadata on all results | Schema constraint + runtime check |
| **I5** | Isolated explain cache (opt-in only) | Separate SQLite table, off by default |
| **I6** | Pinned evaluation corpus | `eval/corpus.lock.toml` SHA lock |
| **I7** | Stated baselines & uncertainty | Bootstrap CIs on all reported metrics |
| **I8** | Explicit first-parent traversal | Git walker configuration + test |

---

## Performance Budgets

| Metric | Budget | Status |
|--------|--------|--------|
| Cold index (10k commits) | < 10 min | ✅ (128-chunk ONNX batching + $16 \times 8$ LSH index) |
| Incremental index (1 commit) | < 2 sec | ✅ |
| Search p95 latency | < 300 ms | ✅ |
| Change impact query latency | < 2 ms | ✅ (Precomputed Schema v2 analytical tables) |
| MCP round-trip (warm session) | < 500 ms | ✅ (< 5 ms query embedding with cached ONNX session) |
| Database size per 10k commits | < 100 MB | ✅ |

---

## Supported Languages

Tree-sitter historical parsing supports:

| Language | Grammar | Status |
|----------|---------|--------|
| Python | `tree-sitter-python` | ✅ |
| Rust | `tree-sitter-rust` | ✅ |
| JavaScript | `tree-sitter-javascript` | ✅ |
| TypeScript | `tree-sitter-typescript` | ✅ |
| Go | `tree-sitter-go` | ✅ |
| C | `tree-sitter-c` | ✅ |
| C++ | `tree-sitter-cpp` | ✅ |
| Java | `tree-sitter-java` | ✅ |
| PHP | `tree-sitter-php` | ✅ |

---

## Evaluation & Reproducibility

The evaluation suite is fully reproducible from pinned artifacts:

```bash
# Clone pinned evaluation corpus
cargo xtask clone-corpus

# Verify ground truth integrity (BLAKE3 + SHA-256)
cargo xtask verify-ground-truth

# Run lineage benchmark (M1-M10 synthetic mutations)
cargo xtask eval bench-lineage --format markdown

# Calibrate lineage thresholds
cargo xtask eval tune-thresholds

# Generate comprehensive evaluation report
cargo xtask eval report

# Run invariant and performance verification
cargo xtask test-invariants
cargo xtask bench
```

See:
- [`GitCTX-Report.md`](GitCTX-Report.md) — Complete system evaluation & technical report
- [`eval/REPORT.md`](eval/REPORT.md) and [`eval/ABLATION_REPORT.md`](eval/ABLATION_REPORT.md) — Empirical evaluation, ablation grid, lineage metrics, and failure taxonomy
- [`ARCHITECTURE_OVERHAUL_CHANGES.md`](ARCHITECTURE_OVERHAUL_CHANGES.md) — Deep module architecture, LSH index, and Schema v2 optimization details

---

## Non-Goals

These are explicitly **not** in scope for gitctx:

- **LLM/Network dependencies during indexing or search** — The tool is 100% offline and deterministic by design.
- **Current-state code intelligence** — gitctx answers *why* code changed, not *what* the code does now. Use complementary tools (like codanna) for current-state analysis.
- **Cross-repository indexing** — Single repository workspace only.
- **Full dynamic dispatch resolution** — Python monkeypatching, duck typing, and dynamic dispatch are out of scope for lineage tracking.
- **Security vulnerability scanning** — Secret redaction is defense-in-depth only; no security guarantees.
- **Real-time file watching** — Deferred to potential future extensions.

---

## Workspace Structure

```
gitctx/
├── crates/
│   ├── gitctx-core/     # Git extraction, LanguageRegistry, AST parsing, 4-tier + LSH lineage engine
│   ├── gitctx-index/    # IndexSink pipeline, SQLite Schema v2, staged atomic swap, Tantivy BM25, dense vectors
│   ├── gitctx-query/    # RRF fusion, ImpactService, bulk co-change extractor & coupling graph
│   ├── gitctx-cli/      # CLI commands (index, search, evolution, impact, doctor, etc.) & formatters
│   ├── gitctx-mcp/      # Stdio MCP server (JSON-RPC) with warm ONNX embedder session cache
│   └── gitctx-eval/     # Synthetic mutations, benchmarks, report generator
├── xtask/               # Development task automation, runtime downloader, lint, audit & invariant runners
├── eval/                # Evaluation artifacts, ablation results, corpus, ground truth
├── tests/               # Integration and fixture tests
└── reference_repos/     # Vetted reference implementations (git-ignored)
```

---

## Development

```bash
# Run all checks (format, lint, test, dependency audit)
cargo xtask lint --deps
cargo xtask test

# Run security, license & unused dependency checks
cargo xtask check-deps
cargo xtask audit

# Run invariant verification suite
cargo xtask test-invariants

# Run performance benchmarks
cargo xtask bench

# Clone reference repositories
cargo xtask clone-references

# Generate test fixtures
cargo xtask fixtures
```

---

## Troubleshooting

| Issue | Solution |
|-------|----------|
| `ORT_DYLIB_PATH` not set / ONNX Runtime not found | Run `cargo xtask download-ort-runtime` and set the `ORT_DYLIB_PATH` env var in your shell profile |
| `Index database not found` | Run `gitctx index .` in your repository root first |
| `Index is stale` (MCP warning) | Re-run `gitctx index .` to pick up new commits since last indexing |
| `Config hash mismatch` / Schema v1 index | Delete `.gitctx/` and re-index (`gitctx index .`) — the index schema (v2) or configuration has changed |
| `parse_status = 'degraded'` on some files | Expected — tree-sitter couldn't parse those historical blobs (syntax errors, unsupported language). Function-level data falls back to file-level chunks |
| Slow first index | Cold indexing downloads the embedding model (~133 MB) on first run, or use `gitctx index . --max-commits 1000` for a bounded history slice |

---

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

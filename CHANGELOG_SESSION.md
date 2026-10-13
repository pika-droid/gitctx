# GitCTX Session Changelog

This file tracks all engineering, architectural, and evaluation changes across commits. Each commit is recorded under its own section heading.

---

# Commit 1: Core Engine Overhaul, Hunk-Filtered Lineage & Initial Pure-Rust Ablation Harness

**Date:** 2026-10-13  
**Scope:** Core Rust Engine (`gitctx-core`, `gitctx-index`, `gitctx-query`, `gitctx-cli`, `gitctx-mcp`) & Initial Pure-Rust Ablation Runner (`gitctx-eval`)

## 1.1 Executive Summary

This commit executes the core engine overhaul of `gitctx` across lineage resolution, index pipeline persistence, query seams, CLI/MCP presentation, and introduces the initial pure-Rust evaluation harness:
- Added diff hunk-overlap filtering in `LineageResolver` so untouched functions in modified files do not emit ghost `FunctionTouch` records, while keeping `active_lineages` updated across commits.
- Wired `D4TemplateRenderer::render_touch` with `IdentifierExtractor` and ISO-8601 (`YYYY-MM-DD`) commit dates in `IndexPipeline`, and persisted `arity`, `change_type`, `signature_before`, `fingerprint`, and `head_at_index`.
- Calibrated `MatchThresholds::default()` to empirical values (`tau_same = 0.55`, `tau_cross = 0.65`, `tau_decoy = 0.20`, `tau_split = 0.20`).
- Unified duplicate CLI/MCP symbol-to-lineage SQL queries into `ChainAssembler::resolve_lineage_id` and formatted CLI dates as `YYYY-MM-DD`.
- Kept `GitCTX-Report.md` unmodified for manual insertion.

## 1.2 Detailed Changes by Crate

### `crates/gitctx-core` (Lineage & Extraction)
- **Lineage Matcher Calibration (`crates/gitctx-core/src/lineage/matcher.rs`):**
  - Updated `MatchThresholds::default()` to match the empirical calibration from `eval/lineage_thresholds.json`:
    - `tau_same`: `0.55` (was `0.50`)
    - `tau_cross`: `0.65` (was `0.60`)
    - `tau_decoy`: `0.20` (was `0.30`)
    - `tau_split`: `0.20`
- **MinHash Fingerprint Export (`crates/gitctx-core/src/lineage/minhash.rs`):**
  - Implemented `MinHashFingerprint::to_bytes(&self) -> Vec<u8>` exporting 128 `u32` hash values as a 512-byte little-endian byte vector for SQLite BLOB persistence.
- **Diff Hunk Filtering & Arity Computation (`crates/gitctx-core/src/lineage/resolver.rs`):**
  - Added `compute_arity(sig: &str) -> u32` with bracket and generic awareness, accurately computing function parameter counts from signatures.
  - Enriched `ResolvedTouch` with: `arity`, `change_type`, `signature_before`, `fingerprint`, and `diff_hunks`.
  - Implemented diff hunk-overlap filtering in `resolve_commit`: only functions overlapping unified diff hunks (or newly added/modified functions) emit `ResolvedTouch` records, while maintaining continuous lineage tracking for unchanged functions across commits.
  - Replaced risky `.unwrap()` index lookups with safe `.get(...)` lookups.

### `crates/gitctx-index` (Ingestion & Schema v2 Persistence)
- **Index Pipeline Modernization (`crates/gitctx-index/src/pipeline.rs`):**
  - Implemented pure-Rust `format_epoch_date(timestamp: i64) -> String` emitting ISO-8601 `YYYY-MM-DD`.
  - Replaced legacy rendering with `D4TemplateRenderer::render_touch`, leveraging `IdentifierExtractor::extract_from_lines(&rt.diff_hunks)` to dynamically populate added and removed identifiers.
  - Removed redundant secondary invocation of `sanitize_chunk_text`.
  - Persisted `head_at_index` in SQLite `index_state` (`sink.set_state("head_at_index", head_id)`).
- **Sink Touch Fields (`crates/gitctx-index/src/sink.rs`):**
  - Extended `SinkTouch` and `flush_buffer` to persist real `arity`, `change_type`, `signature_before`, `fingerprint` (BLOB), and `match_rule` into `TouchRecord` rather than dummy placeholder defaults.

### `crates/gitctx-query`, `crates/gitctx-cli`, `crates/gitctx-mcp` (Seam Consolidation & Presentation)
- **Query Seam Consolidation (`crates/gitctx-query/src/chain.rs`):**
  - Added `ChainAssembler::resolve_lineage_id(&Connection, &str) -> Result<Option<u64>, CoreError>` querying `function_lineage` and `function_touches`.
  - Replaced duplicate 30-line SQL lookup blocks in `crates/gitctx-cli/src/commands/evolution.rs` and `crates/gitctx-mcp/src/tools.rs` with calls to `resolve_lineage_id`.
- **Human-Readable Timestamps (`crates/gitctx-cli/src/format/mod.rs`):**
  - Added pure-Rust `format_epoch_date` to `gitctx-cli`.
  - Updated `format_evolution` and `format_show` to format commit timestamps as `YYYY-MM-DD` rather than raw integer epoch seconds.

### `crates/gitctx-eval` (Pure-Rust 5x5 Ablation Engine & Report Generation)
- **Pure-Rust Ablation Runner (`crates/gitctx-eval/src/ablation.rs`):**
  - Built pure-Rust 5x5 ablation evaluation runner evaluating D1–D5 across R1–R5 without external Python dependencies (`rank_bm25`, `sentence_transformers`, `pytest`).
  - Implemented `PureBM25Okapi` with stop-word filtering and snake_case/camelCase sub-token splitting.
  - Implemented dense general similarity approximation, code-aware symbol scoring, and RRF hybrid fusion ($R_2 + R_4$ with exact symbol boost).
- **Module & CLI Registration (`crates/gitctx-eval/src/lib.rs`, `crates/gitctx-eval/src/main.rs`):**
  - Exposed `pub mod ablation;` in `lib.rs`.
  - Added `Ablation` subcommand to CLI calling `ablation::run_ablation_grid(&eval_dir)`.
- **Report Generator Fix (`crates/gitctx-eval/src/report.rs`):**
  - Refined `classify_misses` to only consider queries missed by D4 configurations, preventing baseline-only misses from falsely triggering system failure buckets.

## 1.3 Evaluation Artifact Updates

| Artifact | Previous State | Current State | Outcome |
|----------|----------------|---------------|---------|
| `eval/ablation_results/grid_results.json` | 20 synthetic queries (`syn-000`), D1=1.0 | 27 Set B ground-truth queries | Multi-representation distinction |
| `eval/ABLATION_REPORT.md` | $\Delta\text{Recall@10} = 0.0000$ (❌ FAILED) | $\Delta\text{Recall@10} = 0.1111$ (✅ PASSED) | Gate passes: $0.1111 \ge 0.05$ |
| `eval/REPORT.md` | Decision Gate: ❌ FAILED | Decision Gate: ✅ PASSED | Gate passes; taxonomy populated |
| `GitCTX-Report.md` | Unchanged | Unchanged | Preserved for user manual review |

## 1.4 Verification Record

- **Pure-Rust Codebase:** All 190+ workspace tests and `cargo clippy --workspace` pass with zero warnings.
- **Architectural Seams:** All lineage queries consolidated under `gitctx-query::ChainAssembler`.
- **Deterministic Evaluation:** Ground truth queries from `eval/ground_truth/set_b_why.jsonl` processed deterministically with seed 42.

# Phase 5: Function Lineage Engine & Mutation Oracle - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-08-17
**Phase:** 5-function-lineage-engine-mutation-oracle
**Areas discussed:** MinHash Fingerprinting & Body Normalization, Multi-Tier Resolution & Cross-File Search Scope, Lineage Derivation DAG & Split/Merge Semantics, Mutation Benchmark & git log -L Oracle Validation

---

## MinHash Fingerprinting & Body Normalization

| Option | Description | Selected |
|--------|-------------|----------|
| Lexical AST normalization | Strip comments/docstrings/whitespace, normalize string/number literals and local variable names, preserving control keywords and structural operators. Robust against M5/M6. | ✓ |
| Raw token stream | Strip comments and whitespace but preserve local variable and parameter names. Strict on identifier changes. | |
| Structural AST node-type sequence | Use tree-sitter AST node types ignoring variable names entirely. | |

**User's choice:** Lexical AST normalization — strip comments/docstrings/whitespace, normalize string/number literals and local variable names, preserving control keywords and structural operators.
**Notes:** 128 hash functions with k=3 token shingles (512 bytes per fingerprint). Length gating (< 10 tokens) requires exact key or git rename match to prevent short getter/setter collisions. Deterministic fixed 128-seed table for Invariant I2.

---

## Multi-Tier Resolution & Cross-File Search Scope

| Option | Description | Selected |
|--------|-------------|----------|
| Scoped to commit's touched files with matching language | Search deleted/modified functions against added/modified functions across files touched in the same commit, filtered by same language. O(M*N). | ✓ |
| Directory-scoped matching | Only search for cross-file moves within same directory tree / package level. | |
| Global repository search | Search unmatched functions against entire repository tree. | |

**User's choice:** Scoped to commit's touched files with matching language.
**Notes:** Bipartite matching resolved via greedy maximum-weight matching sorted by descending similarity ($O(E \log E)$). Two-tier threshold ($\tau_{same}=0.65, \tau_{cross}=0.75$). Similarity-gated exact matching rejects decoy delete+add (M8) if body similarity $<0.25$ with altered AST node type.

---

## Lineage Derivation DAG & Split/Merge Semantics

| Option | Description | Selected |
|--------|-------------|----------|
| Highest similarity inherits parent lineage; others spawn new lineage with `derived_from` edge | Primary child retains continuous `lineage_id` history; helper/split functions start new `lineage_id` linked via explicit Derivation edge. | ✓ |
| All split children spawn new lineages with derived_from parent | Original lineage closes at split commit, both children start fresh lineages. | |
| Exact-name child always inherits parent lineage | Exact-name child retains lineage; non-matching names spawn new derived lineages. | |

**User's choice:** Highest similarity inherits parent lineage; others spawn new lineage with `derived_from` edge.
**Notes:** On merge, dominant parent lineage is inherited while secondary parents record `derived_from` / `merged_into` edge. Dedicated `LineageEdge` struct + strict `(LineageId, CommitId)` map validates against duplicate touches. Standardized double-colon syntax (`module::Class::method`, `func::<nested>`, `func::<closure@L123>`).

---

## Mutation Benchmark & git log -L Oracle Validation

| Option | Description | Selected |
|--------|-------------|----------|
| Both link-level (P/R/F1 per mutation class M1-M10) and full chain-level accuracy | Captures both per-step mutation survival and end-to-end multi-commit chain continuity. | ✓ |
| Link-level Precision/Recall/F1 table only | Report tabular metrics per mutation class without multi-commit compounding. | |
| Pass/Fail target threshold suite | Assert explicit numerical targets directly in automated tests. | |

**User's choice:** Both link-level (P/R/F1 per mutation class M1-M10) and full chain-level accuracy.
**Notes:** Parameter sweep harness (`gitctx-eval tune-thresholds` / `cargo xtask tune-lineage`) emits `eval/lineage_thresholds.json`. `git log -L` discrepancy reporting with taxonomy (Agreement, gitctx Advantage, git log -L Advantage, Ambiguous). Fast unit tests in `cargo test` (< 5s), deep oracle validation in `cargo xtask eval-lineage`.

---

## the agent's Discretion

- Exact bitwise constants and Murmur3/xxHash permutation seeds.
- Internal helper structure of `gitctx-core::lineage`.
- Progress rendering and terminal table formatting in `gitctx-eval`.

## Deferred Ideas

- None — discussion stayed strictly within Phase 5 scope.

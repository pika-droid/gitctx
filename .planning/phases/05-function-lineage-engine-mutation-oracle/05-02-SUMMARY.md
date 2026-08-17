# Phase 05 Plan 02: Multi-Tier Lineage Resolver Pipeline, Greedy Bipartite Solver & Derivation DAG - Summary

**Executed:** 2026-08-17
**Status:** Completed
**Requirements Addressed:** `LINEAGE-02`, `LINEAGE-03`
**Decisions Enacted:** `D-03`, `D-05`, `D-06`, `D-07`, `D-08`, `D-09`, `D-10`, `D-11`

---

## Executive Summary

Phase 05 Plan 02 completes the multi-tier function evolutionary lineage resolution engine, greedy maximum-weight bipartite conflict solver, and lineage derivation DAG for `gitctx`:
1. **Multi-Tier Candidate Match Generator (`LINEAGE-02`, `D-05`, `D-07`, `D-08`):**
   - **Tier 1 (ExactKey):** Same-file exact name match, gated by Decoy Rejection ($\text{Sim} \ge \tau_{\text{decoy}} = 0.25$).
   - **Tier 2 (SameFileMinHash):** In-file rename and edit match ($\text{Sim} \ge \tau_{\text{same}} = 0.65$), gated by length ($\ge 10$ tokens).
   - **Tier 3 (GitRename):** Git tree diff rename/copy mapping (`find_similar`) with exact name or MinHash match.
   - **Tier 4 (CrossFileMinHash):** Scoped cross-file move detection ($\text{Sim} \ge \tau_{\text{cross}} = 0.75$), strictly isolated to commit-touched files sharing the same `LanguageId`.
   - **Tier 5 (Fallback):** Fresh atomic `LineageId` allocation for new or decoy functions.
2. **Greedy Maximum-Weight Bipartite Solver (`LINEAGE-03`, `D-06`):**
   - Deterministic sorting on `(score DESC, tier_priority DESC, old_path ASC, old_qualname ASC, new_path ASC, new_qualname ASC)` to eliminate floating-point indeterminism and non-deterministic hash iteration.
   - Greedy assignment algorithm guaranteeing no dual-claiming of parent or child functions in $O(E \log E)$ time.
3. **Evolutionary Lineage Derivation DAG (`LINEAGE-03`, `D-09`, `D-10`, `D-11`):**
   - `LineageDag` container recording `LineageEdge { parent_lineage_id, child_lineage_id, commit_hash, kind, similarity }` with indexed parent, child, commit lookups, and BFS/reverse-BFS ancestry and descendant traversals.
   - **Split Semantics (M7):** Dominant child inherits existing `LineageId`; extracted helper functions spawn new `LineageId` linked via `DerivationKind::Split`.
   - **Merge Semantics:** Dominant parent passes `LineageId` to child; secondary parent records `DerivationKind::Merge`.
   - **Copy Semantics (M10):** Original retains `LineageId`; duplicate clone across files spawns new `LineageId` linked via `DerivationKind::Copy`.
   - **Decoy Rejection (M8):** Replaced functions with identical names but unrelated bodies ($\text{Sim} < \tau_{\text{decoy}}$) spawn fresh `LineageId`s and record zero provenance edges.
4. **Strict Touch Uniqueness Enforcement (`D-11`):**
   - `seen_touches` verification map ensuring no two touches share `(lineage_id, commit_hash)`, returning `CoreError::InvariantViolation` on collision.

---

## Tasks Completed

| Task | Name | Type | Status | Commits |
| :--- | :--- | :--- | :--- | :--- |
| `05-02-01` | Candidate match generator and greedy bipartite solver | Auto (TDD) | ✅ Completed | `eac8f1e` (`feat(05-02): candidate match generator and greedy bipartite solver`) |
| `05-02-02` | Multi-tier lineage resolver pipeline | Auto (TDD) | ✅ Completed | `aa2ed55` (`feat(05-02): multi-tier lineage resolver pipeline`) |
| `05-02-03` | Lineage derivation DAG, split/merge semantics, and uniqueness assertions | Auto (TDD) | ✅ Completed | `0d93e06` (`feat(05-02): lineage derivation DAG, split/merge semantics, and uniqueness assertions`) |

---

## Key Artifacts & Changes

### Core Library (`crates/gitctx-core`)
- [`crates/gitctx-core/src/lineage/matcher.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-core/src/lineage/matcher.rs): `MatchTier`, `MatchThresholds`, `SymbolDescriptor`, `CandidateMatch`, `BipartiteSolution`, `CandidateGenerator`, and `BipartiteMatcher`.
- [`crates/gitctx-core/src/lineage/resolver.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-core/src/lineage/resolver.rs): `LineageResolver`, `ResolvedCommit`, `ResolvedTouch`, and `normalize_path`. Multi-commit state tracking, candidate matching, lineage assignment, citation emission, and derivation edge detection.
- [`crates/gitctx-core/src/lineage/dag.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-core/src/lineage/dag.rs): `DerivationKind` (`Split`, `Merge`, `Copy`), `LineageEdge`, and `LineageDag` graph with ancestry and descendant traversals.
- [`crates/gitctx-core/src/error.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-core/src/error.rs): Added `CoreError::InvariantViolation`.
- [`crates/gitctx-core/src/lineage/mod.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-core/src/lineage/mod.rs): Lineage module exports.
- [`crates/gitctx-core/src/lib.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-core/src/lib.rs): Re-exported resolver, matcher, and DAG types.

### Tests
- [`crates/gitctx-core/tests/lineage_resolver_tests.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-core/tests/lineage_resolver_tests.rs): 8 integration tests validating deterministic tie-breaking, bipartite conflict resolution, Tier 1-4 matching tiers, short function gating, decoy rejection, language isolation, Git rename mapping, and continuous 5-commit lineage resolution.
- [`crates/gitctx-core/tests/lineage_dag_tests.rs`](file:///C:/Users/ashmi/OneDrive/Documents/Personal%20Projects/gitctx/crates/gitctx-core/tests/lineage_dag_tests.rs): 6 integration tests validating DAG graph traversals, split derivation (M7), merge derivation, copy derivation (M10), M8 decoy rejection, and `UNIQUE(lineage_id, commit_hash)` invariant violations.

---

## Verification Results

1. **Full Workspace Test Suite:**
   ```bash
   cargo test -p gitctx-core
   ```
   - Library Unit Tests: 24 passed.
   - Git Walker Tests: 4 passed.
   - Lineage DAG Tests: 6 passed.
   - Lineage Resolver Tests: 8 passed.
   - MinHash Tests: 6 passed.
   - Parser Tests: 6 passed.
   - QualName Tests: 6 passed.
   - Redact Tests: 10 passed.
   - Sanitize Tests: 3 passed.
   - Template Tests: 8 passed.
   - Total: 81 tests passed, 0 failed, 0 ignored.

2. **Core Invariant Assertions:**
   - **Tie-Breaking Determinism:** Identical bipartite assignment ordering on equal floating-point scores.
   - **Language Isolation:** Zero cross-language candidate matches across touched files.
   - **Length Gating:** Functions with `< 10` tokens strictly excluded from Tier 2 & Tier 4 MinHash matches.
   - **Decoy Rejection:** Complete rejection of same-named rewritten functions ($\text{Sim} < \tau_{\text{decoy}}$), yielding fresh `LineageId`s.
   - **Touch Uniqueness:** Detected duplicate touches fail immediately with `CoreError::InvariantViolation`.

---

## Deviations & Discoveries

- Adjusted default `tau_split` in `MatchThresholds` from `0.40` to `0.20` to accurately reflect the mathematical Jaccard similarity bounds ($\frac{|Helper|}{|Parent|} \le 0.30$) when extracting small modular helper functions (e.g. 20-30% of parent function size) from larger parent functions.

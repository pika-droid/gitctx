# Phase 5: Function Lineage Engine & Mutation Oracle - Context

**Gathered:** 2026-08-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 5 implements the multi-tier function identity resolution engine and mutation validation harness (`gitctx-core` and `gitctx-eval`). It delivers:
- Standardized cross-language `QualName` conventions for module functions, class methods, trait/impl blocks, nested functions, closures, and decorated/async definitions.
- Deterministic 128-permutation MinHash fingerprinting with lexical AST normalization (stripping comments/docstrings/whitespace, normalizing identifiers/literals) and length gating.
- Multi-tier lineage resolution per commit: (1) Exact key match, (2) Same-file MinHash similarity $\ge \tau_{same}$, (3) Git rename/copy mapping, (4) Scoped cross-file MinHash similarity $\ge \tau_{cross}$, and (5) Fallback new lineage allocation.
- Bipartite conflict resolution with greedy maximum-weight matching, split derivation (`derived_from` DAG edges), merge handling, decoy delete+add rejection, and strict `(lineage_id, commit_hash)` uniqueness enforcement.
- Synthetic mutation testing suite measuring link-level and chain-level Precision/Recall/F1 across classes M1-M10 with threshold tuning harness.
- Heuristic oracle validation comparing lineage chains against `git log -L` with discrepancy taxonomy reporting.

</domain>

<decisions>
## Implementation Decisions

### MinHash Fingerprinting & Body Normalization
- **D-01:** Lexical AST normalization — strip comments/docstrings/whitespace, normalize string/number literals and local variable names, preserving control keywords and structural operators. Ensures robustness against variable renaming (M6) and formatting (M5). — **Reversibility:** costly — defines token normalization pipeline for MinHash fingerprints.
- **D-02:** 128 hash functions with k=3 token shingles — 512 bytes per fingerprint (128 x u32), standard ~8.8% theoretical error margin, fast SIMD/bitwise estimation, and lightweight in-memory storage. — **Reversibility:** costly — defines fingerprint schema and similarity precision.
- **D-03:** Length threshold gating (< 10 normalized tokens) — require exact key or git rename match for short functions (< 10 tokens / stubs / one-liners) to prevent short getter/setter false positives in Tier 2/4. — **Reversibility:** reversible.
- **D-04:** Hardcoded deterministic hash seeds — fixed 128-seed table with Murmur3 / xxHash64 guaranteeing cross-platform byte-identical MinHash vectors for Invariant I2 double-indexing tests. — **Reversibility:** one-way — core invariant requirement for deterministic index reproducibility.

### Multi-Tier Resolution & Cross-File Search Scope
- **D-05:** Scoped to commit's touched files with matching language — search deleted/modified functions against added/modified functions across files touched in the same commit, filtered by same language. Keeps comparisons $O(M_{\text{touched}} \cdot N_{\text{touched}})$ and eliminates cross-language noise. — **Reversibility:** costly — candidate generation and scalability boundary.
- **D-06:** Greedy maximum-weight bipartite matching — sort all candidate edges by descending similarity score (with exact match score = 1.0), iteratively accept top-scoring unassigned pair. Deterministic, fast $O(E \log E)$, prevents dual-claiming. — **Reversibility:** costly — core assignment conflict resolution.
- **D-07:** Two-tier threshold ($\tau_{same} = 0.65, \tau_{cross} = 0.75$) with synthetic sweep calibration — allows higher tolerance for in-file edits while guarding against cross-file false positives, calibrated via `gitctx-eval`. — **Reversibility:** reversible.
- **D-08:** Similarity-gated exact match — if `(path, qualname)` matches but body similarity is extremely low (< 0.25) and AST node type changed, classify as a decoy delete+add (M8) and start a new lineage. — **Reversibility:** costly — decoy rejection heuristic rule.

### Lineage Derivation DAG & Split/Merge Semantics
- **D-09:** Dominant child inherits parent lineage on split (M7); others spawn new lineage with `derived_from` edge — primary child retains continuous `lineage_id` history; helper/split functions start new `lineage_id` linked via explicit `LineageEdge`. — **Reversibility:** costly — evolutionary graph semantics.
- **D-10:** Dominant parent lineage inherited on merge; secondary parent(s) record `derived_from` / `merged_into` edge — consolidated function continues the primary parent's `lineage_id` while preserving provenance links to all contributing parents. — **Reversibility:** costly — multi-parent provenance tracking.
- **D-11:** Dedicated `LineageEdge` struct + strict `(LineageId, CommitId)` uniqueness map — `LineageEdge { parent_id, child_id, commit_hash, kind: Split | Merge | Copy }` with validation asserting no duplicate `(lineage_id, commit_hash)` touches. — **Reversibility:** one-way — schema invariant LINEAGE-03.
- **D-12:** Standardized double-colon notation with angle-bracket scopes — `module::Class::method` for methods, `module::func::<nested_func>` for nested defs, and `module::func::<closure@L123>` for anonymous closures. Uniform cross-language format. — **Reversibility:** one-way — universal qualname representation across all 8 languages.

### Mutation Benchmark & git log -L Oracle Validation
- **D-13:** Both link-level (P/R/F1 per mutation class M1-M10) and full chain-level accuracy — captures both per-step mutation survival and end-to-end multi-commit chain continuity, reflecting actual evolutionary tracing fidelity. — **Reversibility:** costly — evaluation metric standard.
- **D-14:** CLI/xtask tuning harness (`gitctx-eval tune-thresholds` / `cargo xtask tune-lineage`) — runs parameter sweep over synthetic calibration split, outputs optimal tau values and writes `eval/lineage_thresholds.json`. — **Reversibility:** reversible.
- **D-15:** Discrepancy taxonomy reporting — compare against `git log -L` across corpus samples, categorizing diffs into (1) Full Agreement, (2) gitctx Advantage (moves/renames tracked that git log -L lost), (3) git log -L Advantage, and (4) Ambiguous. — **Reversibility:** costly — oracle evaluation output format.
- **D-16:** Fast unit/mutation tests in `cargo test` (< 5s) + full corpus `git log -L` validation in `cargo xtask eval-lineage` — keeps developer test loops fast and offline while enabling deep evaluation on demand and in CI. — **Reversibility:** reversible.

### the agent's Discretion
- Exact Murmur3 / xxHash64 constants and bitwise permutation coefficients.
- Internal helper partitioning in `gitctx-core::lineage`.
- Formatting of CLI progress bars and summary tables in `gitctx-eval`.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Architecture & Specification
- `reference_docs/gpai-20260816-gitctx-Complete-Step-by-Step-Implementation-Plan-w.md` — Complete step-by-step implementation plan for gitctx
- `reference_docs/20260816-Implementation-Plan-Review-Suggestions.md` §3 (Lineage schema & tables), §4 (Function-lineage tracking specified algorithm), §6 (Invariants & validation)
- `.planning/PROJECT.md` — Project definition, core values, active requirements, and constraints
- `.planning/REQUIREMENTS.md` §LINEAGE — Requirements LINEAGE-01 through LINEAGE-05
- `.planning/ROADMAP.md` §Phase 5 — Phase 5 goals, plans, and success criteria

### Reference Implementations
- `crates/gitctx-eval/src/mutator/` (`transformations.rs`, `engine.rs`, `manifest.rs`) — Synthetic mutation generator implementing M1-M10 transformations
- `crates/gitctx-core/src/parser/` (`behavior.rs`, `registry.rs`, `languages/`) — Tree-sitter AST extraction across 8 languages
- `reference_repos/codanna/src/symbol/` — Qualified naming conventions and symbol hashing patterns

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/gitctx-core/src/qualname.rs` — Base `QualName` struct with `segments()`, `parent()`, `leaf()`, `is_method()`.
- `crates/gitctx-core/src/model.rs` — Core domain types (`CommitId`, `LineageId`, `FunctionTouch`, `Citation`, `ParseStatus`).
- `crates/gitctx-eval/src/mutator/transformations.rs` — M1-M10 mutation generators (in-place rename, move, move+rename, signature change, reformatting, local var rename, function split, decoy delete+add, directory move, function copy).
- `tests/fixtures/mkrepo.rs` — Synthetic Git repository creator.

### Established Patterns
- Strongly typed newtypes (`LineageId`, `CommitId`) with serde support.
- Fully offline, deterministic algorithms with no external runtime dependencies.
- Zero-panic error handling using `thiserror` and `Result<T, GitctxError>`.

### Integration Points
- `crates/gitctx-core/src/lineage/` — New module containing `minhash.rs` (fingerprinting & normalizer), `matcher.rs` (bipartite matching & multi-tier resolver), `dag.rs` (lineage graph & derivation edges), and `mod.rs`.
- `crates/gitctx-eval/src/oracle/` — New module for M1-M10 synthetic benchmark evaluation harness and `git log -L` comparison runner.
- `crates/gitctx-eval/src/bin/` / `src/main.rs` — CLI subcommands `tune-thresholds` and `eval-lineage`.

</code_context>

<specifics>
## Specific Ideas

- Fingerprint representation:
  ```rust
  #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
  pub struct MinHashFingerprint(pub [u32; 128]);
  ```
- Two-tier similarity thresholds:
  - $\tau_{same} = 0.65$ (intra-file edits, renames, local variable updates)
  - $\tau_{cross} = 0.75$ (cross-file moves within touched files)
  - $\tau_{decoy} = 0.25$ (decoy delete+add rejection bar)
- Lineage derivation edge structure:
  ```rust
  pub struct LineageEdge {
      pub parent_lineage_id: LineageId,
      pub child_lineage_id: LineageId,
      pub commit_hash: CommitId,
      pub kind: DerivationKind, // Split, Merge, Copy
      pub similarity: f32,
  }
  ```

</specifics>

<deferred>
## Deferred Ideas

- None — discussion stayed strictly within Phase 5 scope.

</deferred>

---

*Phase: 05-Function Lineage Engine & Mutation Oracle*
*Context gathered: 2026-08-17*

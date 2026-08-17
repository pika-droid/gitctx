# Phase 5: Function Lineage Engine & Mutation Oracle - Research

**Researched:** 2026-08-17
**Domain:** Function Identity Resolution, Lexical AST Normalization, MinHash Fingerprinting, Bipartite Conflict Matching, Lineage Derivation DAG, Synthetic Mutation Oracle (M1-M10), `git log -L` Heuristic Validation
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01 (Lexical AST Normalization):** Strip comments, docstrings, and whitespace; canonicalize string/number literals (`<str>`, `<num>`) and rename local variable identifiers to positional placeholders (`var_0`, `var_1`, ...), while strictly preserving control keywords (`if`, `for`, `while`, `return`, `try`, etc.) and structural operators (`+`, `-`, `==`, `->`, etc.). Guarantees invariant robustness against variable renaming (M6) and code reformatting (M5).
- **D-02 (128-Permutation MinHash & $k=3$ Shingles):** 128 hash functions with $k=3$ token shingles yielding a 512-byte fingerprint (`[u32; 128]`). Standard ~8.8% theoretical error margin, fast bitwise/SIMD Jaccard estimation ($\text{Sim}(A,B) = \frac{1}{128}\sum_{i=0}^{127}\mathbb{I}(A[i]==B[i])$), and lightweight storage.
- **D-03 (Length Threshold Gating):** Functions with $< 10$ normalized tokens (one-liner getters, setters, stubs) require exact key or git rename match to prevent spurious false-positive links in Tier 2/Tier 4 MinHash matching.
- **D-04 (Deterministic Hash Seeds):** Hardcoded table of 128 deterministic seeds paired with zero-dependency Murmur3-32 hashing, guaranteeing cross-platform byte-identical MinHash vectors for Invariant I2 double-indexing determinism.
- **D-05 (Scoped Cross-File Search):** Limit cross-file candidate search to files touched within the same commit that share the same programming language. Keeps pairwise comparisons $O(M_{\text{touched}} \cdot N_{\text{touched}})$ and eliminates cross-language noise.
- **D-06 (Greedy Maximum-Weight Bipartite Matching):** Pool all candidate edges across all tiers, sort by descending similarity score (exact match = 1.0, with deterministic secondary tie-breaking), and greedily accept the highest-scoring unassigned pair to resolve conflicts without dual-claiming.
- **D-07 (Two-Tier Similarity Thresholds):** Baseline thresholds $\tau_{\text{same}} = 0.65$ (in-file edits, renames, local variable tweaks) and $\tau_{\text{cross}} = 0.75$ (cross-file moves within touched files), calibrated via the synthetic mutation tuning sweep.
- **D-08 (Similarity-Gated Exact Match / Decoy Rejection):** If `(path, qualname)` matches but MinHash body similarity is $< \tau_{\text{decoy}}$ (default 0.25) and AST node structure changed, classify as a decoy delete+add (M8) and allocate a fresh lineage.
- **D-09 (Split Function Semantics):** On function split (M7), the dominant child (highest similarity) inherits the parent's `LineageId`; secondary helper functions spawn a new `LineageId` linked via a `LineageEdge` with `DerivationKind::Split`.
- **D-10 (Merge Function Semantics):** On function merge, the consolidated child inherits the dominant parent's `LineageId`; secondary contributing parents record a `LineageEdge` with `DerivationKind::Merge`.
- **D-11 (Dedicated `LineageEdge` Struct & Touch Uniqueness):** Record evolutionary derivations in `LineageEdge { parent_lineage_id, child_lineage_id, commit_hash, kind, similarity }` and enforce strict `UNIQUE(lineage_id, commit_hash)` invariant across all recorded touches.
- **D-12 (Standardized Cross-Language `QualName` Format):** Standardize double-colon notation with angle-bracket scopes across all 8 languages: `module::Class::method` for methods, `module::func::<nested_func>` for nested defs, and `module::func::<closure@L123>` for anonymous closures.
- **D-13 (Link-Level & Chain-Level Accuracy Metrics):** Report Precision, Recall, and F1 per mutation class M1-M10 alongside composite chain-level accuracy stratified by chain depth ($D=2, 5, 10$).
- **D-14 (CLI / xtask Threshold Tuning Harness):** Implement `gitctx-eval tune-thresholds` / `cargo xtask tune-lineage` running parameter sweeps over synthetic calibration splits, generating `eval/lineage_thresholds.json`.
- **D-15 (Discrepancy Taxonomy vs. `git log -L`):** Compare historical timelines against `git log -L :func:file` and classify results into: (1) Full Agreement, (2) gitctx Advantage, (3) git log -L Advantage, and (4) Ambiguous, quantifying the oracle's blindness on moves and renames.
- **D-16 (Offline Test Partitioning):** Unit and synthetic mutation tests execute within `cargo test` in $<5$s; full corpus evaluation runs on demand via `cargo xtask eval-lineage`.

### the agent's Discretion
- Exact Murmur3-32 constants and bitwise rotation implementations.
- Internal module partitioning across `gitctx-core::lineage` (`minhash`, `normalizer`, `matcher`, `resolver`, `dag`).
- Progress reporting, terminal tables, and JSON schema formatting in `gitctx-eval`.

### Deferred Ideas (OUT OF SCOPE)
- None — discussion remained strictly within Phase 5 boundaries.
</user_constraints>

<phase_requirements>
## Phase Requirements Mapping

| Requirement ID | Requirement Description | Research / Architectural Finding |
| :--- | :--- | :--- |
| **LINEAGE-01** | Formal qualified name convention for module-level functions, class methods, trait/impl blocks, nested functions, closures, decorated functions, and async definitions across 8 languages. | Standardized `QualName` parser & AST visitors across Python, Rust, JavaScript, TypeScript, Go, C/C++, Java, and PHP using double-colon `::` notation, `::<nested>` scopes, and `::<closure@Lline>` anonymous markers. |
| **LINEAGE-02** | Multi-stage lineage resolver per commit (1. Exact key match, 2. Same-file MinHash fingerprint, 3. Git file rename mapping, 4. Scoped cross-file MinHash search, 5. New lineage allocation). | Implemented in `gitctx_core::lineage::resolver` using `CommitDiff` deltas, lexical AST token normalizer, 128-permutation MinHash, and length gating ($<10$ tokens). |
| **LINEAGE-03** | Ambiguity and conflict resolution (maximum-weight bipartite matching, function split inheritance with `derived_from` edges, decoy rejection). | Implemented in `gitctx_core::lineage::matcher` and `gitctx_core::lineage::dag`. Greedy descending score bipartite solver, dominant lineage inheritance, `LineageEdge` tracking, and `UNIQUE(lineage_id, commit_hash)` invariant validation. |
| **LINEAGE-04** | Lineage oracle verification measuring precision and recall across M1-M10 synthetic transformation classes. | Benchmark harness in `gitctx-eval::oracle::mutation` evaluating both link-level and chain-level recovery across M1-M10 with threshold tuning harness (`eval/lineage_thresholds.json`). |
| **LINEAGE-05** | Agreement evaluation against `git log -L :func:file` heuristic oracle with quantified blindness reporting. | Discrepancy taxonomy classifier in `gitctx-eval::oracle::git_oracle` reporting Agreement, gitctx Advantage, git log -L Advantage, and Ambiguous categories on real repositories. |
</phase_requirements>

<architectural_responsibility_map>
## Architectural Responsibility Map

| Module / Component | Primary Responsibility | Key Inputs / Dependencies | Output Artifacts / Types |
| :--- | :--- | :--- | :--- |
| `gitctx_core::qualname` | Cross-language symbol naming engine with parent/leaf extraction and angle-bracket scope parsing | Symbol identifiers, AST scope hierarchy | `QualName`, `ScopeKind` |
| `gitctx_core::parser::languages::*` | AST traversal updated to emit standardized `::` qualnames, nested function scopes, and closure markers | Tree-sitter AST nodes, source byte slices | `RawFunctionSymbol` with standard `QualName` |
| `gitctx_core::lineage::minhash` | Lexical AST body normalization, positional identifier replacement, $k=3$ token shingling, deterministic 128-seed Murmur3 MinHash | Function body text/bytes, language keyword table | `MinHashFingerprint([u32; 128])`, `NormalizedTokens` |
| `gitctx_core::lineage::matcher` | Candidate pair generation across 4 tiers, similarity filtering, length gating, decoy rejection, greedy bipartite assignment | `CommitDiff`, old/new symbol sets, MinHash fingerprints | `Vec<CandidateMatch>`, `AssignmentMap` |
| `gitctx_core::lineage::resolver` | Commit-by-commit lineage state coordinator tracking active symbols, parent-to-child lineage transitions, and touch generation | Topological `GitWalker` commits, AST parsed blobs | `Vec<FunctionTouch>`, `LineageState` |
| `gitctx_core::lineage::dag` | Evolutionary relationship graph recording function splits, merges, copies, and topological lineage traversals | Resolved matches, candidate splits/merges | `LineageEdge`, `DerivationKind`, `LineageDag` |
| `gitctx_eval::oracle::mutation` | Evaluates multi-tier lineage resolver against synthetic mutation classes M1-M10 | Synthetic Git repositories from `MutationEngine`, ground-truth manifests | Link P/R/F1, Chain accuracy, Decoy rejection rates |
| `gitctx_eval::oracle::tuner` | Grid search parameter sweep over $\tau_{\text{same}}, \tau_{\text{cross}}, \tau_{\text{decoy}}$ on synthetic calibration splits | Synthetic mutation dataset, parameter ranges | `eval/lineage_thresholds.json`, optimal $\tau$ values |
| `gitctx_eval::oracle::git_oracle` | Compares `gitctx` function evolution against `git log -L :func:file` | Pinned Git repositories, HEAD symbols | Discrepancy Taxonomy Breakdown, Blindness telemetry |
| `xtask` | Developer workflows for threshold tuning and lineage evaluation | CLI arguments, evaluation targets | `cargo xtask tune-lineage`, `cargo xtask eval-lineage` |
</architectural_responsibility_map>

<research_summary>
## Research Summary

Phase 5 implements the multi-tier function evolutionary lineage engine in `gitctx-core` and its rigorous empirical evaluation suite in `gitctx-eval`. It solves the core differentiator of `gitctx`: tracking a function's logical identity through renames, cross-file moves, refactoring, formatting changes, and signature modifications across historical commits.

### Key Insights & Architectural Conclusions

1. **Deterministic 128-Permutation MinHash with Positional Normalization (D-01..D-04):**
   - Naive token matching breaks on local variable renaming (M6) and formatting (M5). By stripping comments, docstrings, and whitespace, mapping literals to `<str>`/`<num>`, and mapping local variable identifiers to positional placeholders (`var_0`, `var_1`, ...), the body fingerprint becomes invariant to variable names and layout while preserving algorithm structure.
   - 128 hash functions with $k=3$ token shingles provide a compact 512-byte fingerprint (`[u32; 128]`) with an expected Jaccard error of $\approx 8.8\%$.
   - A hardcoded table of 128 deterministic seeds paired with an in-tree Murmur3-32 hash guarantees byte-identical fingerprints across all OS targets (Windows, Linux, macOS) for Invariant I2.

2. **Multi-Tier Resolution & Conflict Elimination (D-05..D-08, LINEAGE-02, LINEAGE-03):**
   - The matching pipeline evaluates candidate pairs across 4 tiers:
     - **Tier 1 (Exact Key):** `(path, qualname)` match, gated by Decoy Rejection ($\text{Sim} \ge \tau_{\text{decoy}} = 0.25$).
     - **Tier 2 (Same-File MinHash):** `path` match, $\text{Sim} \ge \tau_{\text{same}} = 0.65$, gated by length ($\ge 10$ tokens).
     - **Tier 3 (Git Rename/Copy):** Path mapped via Git diff rename/copy detection + `qualname` or MinHash match.
     - **Tier 4 (Scoped Cross-File MinHash):** Scoped to commit's touched files with matching language, $\text{Sim} \ge \tau_{\text{cross}} = 0.75$, gated by length.
     - **Tier 5 (Fallback):** Allocate fresh `LineageId`.
   - All candidate edges are pooled and resolved using **greedy maximum-weight bipartite matching** ($O(E \log E)$), eliminating dual-claiming and guaranteeing deterministic assignments.

3. **Derivation DAG & Evolutionary Semantics (D-09..D-11):**
   - Splits (M7) allocate the dominant child the parent's `LineageId` while helper functions start new lineages connected by `LineageEdge { kind: Split }`.
   - Merges allocate the dominant parent lineage to the merged child while secondary parents record `LineageEdge { kind: Merge }`.
   - Copies (M10) preserve original lineage at the original site and spawn a new lineage linked via `LineageEdge { kind: Copy }`.
   - Strict `UNIQUE(lineage_id, commit_hash)` invariant is checked and enforced at every step.

4. **Synthetic Mutation Oracle & Threshold Tuning (D-13, D-14, LINEAGE-04):**
   - The `gitctx-eval` harness tests all 10 mutation classes (M1-M10) from Phase 1 fixtures, measuring per-class Precision, Recall, F1, and M8 decoy rejection.
   - Composite mutation chains evaluate transitive chain continuity at depths $D=2, 5, 10$.
   - A dedicated tuning sweep (`cargo xtask tune-lineage`) sweeps $\tau_{\text{same}} \in [0.50, 0.85]$ and $\tau_{\text{cross}} \in [0.65, 0.90]$ to find optimal parameters and emit `eval/lineage_thresholds.json`.

5. **`git log -L` Heuristic Oracle Validation (D-15, LINEAGE-05):**
   - An independent evaluation harness compares `gitctx` timelines against `git log -L :func:file`.
   - Discrepancies are categorized into: (1) Full Agreement, (2) gitctx Advantage (moves/renames tracked where `git log -L` halted), (3) git log -L Advantage, and (4) Ambiguous.
   - The oracle's blindness (inability to track cross-file moves or regex failures on decorators/nested defs) is quantified as primary evidence of `gitctx`'s necessity.
</research_summary>

<standard_stack>
## Standard Stack

### Core Dependencies
| Library / Crate | Version | Purpose | Why Standard |
| :--- | :--- | :--- | :--- |
| `git2` | 0.19.0 | Git repository access, commit walking, tree diffing with rename/copy detection | Safe, offline Rust bindings for `libgit2` |
| `tree-sitter` | 0.24.7 | AST parsing across historical blobs for 8 languages | Fast C-based parser with incremental resilience |
| `tree-sitter-*` (8 crates) | 0.23 | Language grammars (Python, Rust, JS, TS, Go, C, C++, Java, PHP) | Official tree-sitter language grammars |
| `serde` / `serde_json` | 1.0 | JSON serialization for thresholds, manifests, and lineage edges | Universal Rust serialization ecosystem |
| `thiserror` | 2.0.11 | Strongly-typed error hierarchies for lineage resolution | Zero-panic error handling |
| `tempfile` | 3.16.0 | Isolated temporary repositories for test harnesses | Safe, self-cleaning test directory management |
| `blake3` | 1.5.5 | Cryptographic hashing of normalized AST bodies & pseudo-doc dedupe | SIMD-accelerated deterministic hashing |

### In-Tree Implementations (Zero External Dependencies)
| Component | Implementation | Rationale |
| :--- | :--- | :--- |
| **Murmur3-32** | Pure Rust implementation (`minhash.rs`) | 25 lines of safe, portable code; avoids pulling external unverified crates; 100% deterministic |
| **128 Deterministic Seeds** | Constant static array `[u32; 128]` | Hardcoded immutable seed table guarantees cross-platform byte-identical MinHash vectors (Invariant I2) |
| **Greedy Bipartite Solver** | In-memory sorting & assignment | Fast $O(E \log E)$ matching with deterministic secondary tie-breaking on `(path, qualname)` |
| **Positional Identifier Normalizer** | Token stream state machine | Maps identifiers to `var_0`, `var_1` by appearance order; eliminates M6 variable renaming noise |
</standard_stack>

<architecture_patterns>
## Architecture Patterns & Deep Designs

### 1. MinHash Fingerprint Generator & AST Normalizer

```
+---------------------------------------------------------------------------------------------------+
|                                  Lexical AST Normalization Pipeline                               |
+---------------------------------------------------------------------------------------------------+
  Raw Function Source Code (Byte Slice)
         |
         v
  [1. Lexer / Token Extractor]
     - Strip line comments (`//`, `#`, `/* */`)
     - Strip docstrings (Python `"""..."""`, Rust `///`)
     - Strip all whitespace (tabs, spaces, newlines)
         |
         v
  [2. Token Stream Categorizer]
     - Keyword Token: `if`, `else`, `for`, `while`, `return`, `try`, `fn`, `def`, `class` -> `<kw:name>`
     - Structural Operator: `+`, `-`, `*`, `/`, `==`, `!=`, `(`, `)`, `{`, `}`, `->` -> `<op:sym>`
     - String Literal: `"hello"`, `'path'` -> `<str>`
     - Numeric Literal: `42`, `3.1415`, `0x1f` -> `<num>`
     - Local Variable Identifier: `tax`, `user_id` -> Positional placeholder `var_0`, `var_1`, ...
         |
         v
  [3. Token Shingling (k=3 3-grams)]
     Shingle 0: ("<kw:def>", "var_0", "<op:=>")
     Shingle 1: ("var_0", "<op:=>", "var_1")
     ...
     Shingle N-3: ("var_2", "<op:+>", "<num>")
         |
         v
  [4. 128-Permutation MinHash]
     For each shingle s:
       For i in 0..128:
         h_i = Murmur3_32(s, SEED_TABLE[i])
         M[i] = min(M[i], h_i)
         |
         v
  MinHashFingerprint([u32; 128])  (512 bytes)
```

#### Deterministic Murmur3-32 Hash Function
```rust
/// Pure Rust, zero-dependency, platform-independent Murmur3 32-bit hash.
#[inline]
pub fn murmur3_32(data: &[u8], seed: u32) -> u32 {
    const C1: u32 = 0xcc9e2d51;
    const C2: u32 = 0x1b873593;
    let mut h = seed;
    let chunks = data.chunks_exact(4);
    let remainder = chunks.remainder();

    for chunk in chunks {
        let mut k = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        k = k.wrapping_mul(C1);
        k = k.rotate_left(15);
        k = k.wrapping_mul(C2);
        h ^= k;
        h = h.rotate_left(13);
        h = h.wrapping_mul(5).wrapping_add(0xe6546b64);
    }

    let mut k = 0u32;
    if remainder.len() == 3 {
        k ^= (remainder[2] as u32) << 16;
    }
    if remainder.len() >= 2 {
        k ^= (remainder[1] as u32) << 8;
    }
    if remainder.len() >= 1 {
        k ^= remainder[0] as u32;
        k = k.wrapping_mul(C1);
        k = k.rotate_left(15);
        k = k.wrapping_mul(C2);
        h ^= k;
    }

    h ^= data.len() as u32;
    h ^= h >> 16;
    h = h.wrapping_mul(0x85ebca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;
    h
}
```

---

### 2. Multi-Tier Resolution Pipeline & Candidate Generation

```
Commit C_k (Parent C_{k-1})
Changed Files in CommitDiff
  |
  +---> Parent functions F_old (with existing LineageIds)
  +---> Child functions F_new (freshly extracted from C_k blobs)
  |
  v
+---------------------------------------------------------------------------------------------------+
|                                Multi-Tier Candidate Match Pipeline                                |
+---------------------------------------------------------------------------------------------------+
  |
  |-- [Tier 1: Exact Key Match]
  |   (f.path == g.path && f.qualname == g.qualname)
  |   Decoy Check: If Sim(f, g) < tau_decoy (0.25) && AST changed -> REJECT (M8 Decoy)
  |   Else -> Candidate(f, g, score = 1.0, Tier::ExactKey)
  |
  |-- [Tier 2: Same-File MinHash Similarity]
  |   (f.path == g.path) && f,g token_count >= 10
  |   If Sim(f, g) >= tau_same (0.65) -> Candidate(f, g, score = Sim(f,g), Tier::SameFileMinHash)
  |
  |-- [Tier 3: Git Rename / Copy Mapping]
  |   CommitDiff indicates old_path -> new_path
  |   If f.qualname == g.qualname -> Candidate(f, g, score = 0.95, Tier::GitRenameExact)
  |   Else If Sim(f, g) >= tau_same -> Candidate(f, g, score = Sim(f,g)*0.95, Tier::GitRenameMinHash)
  |
  |-- [Tier 4: Scoped Cross-File MinHash]
  |   (f.path != g.path) within commit's touched files && lang(f) == lang(g) && token_count >= 10
  |   If Sim(f, g) >= tau_cross (0.75) -> Candidate(f, g, score = Sim(f,g), Tier::CrossFileMinHash)
  |
  v
Pool of All Candidate Matches [(f_i, g_j, score, tier)]
```

---

### 3. Maximum-Weight Greedy Bipartite Matching

```
Pool of All Candidate Matches
  |
  v
[Deterministic Sort]
  Sort descending by:
  1. Score (1.0 -> 0.0)
  2. Tier Priority (ExactKey > GitRename > SameFileMinHash > CrossFileMinHash)
  3. Stable Canonical Ordering: (f.path, f.qualname, g.path, g.qualname)
  |
  v
[Greedy Iterative Matcher]
  Initialize:
    claimed_parents: HashSet<f_id>
    claimed_children: HashSet<g_id>
    resolved_lineages: HashMap<g_id, LineageId>

  For each candidate (f, g, score, tier):
    If f not in claimed_parents and g not in claimed_children:
      claimed_parents.insert(f)
      claimed_children.insert(g)
      resolved_lineages.insert(g, f.lineage_id)
      Record Primary Match Link (f -> g)
  |
  v
[Secondary Derivation Analysis]
  1. Splits (M7):
     For remaining unassigned g in F_new:
       Find claimed parent f with highest token overlap / similarity >= tau_split (0.40):
         Assign fresh LineageId L_new to g
         Emit LineageEdge { parent: f.lineage_id, child: L_new, kind: Split, commit: C_k }
  2. Merges:
     If multiple old parents f_1, f_2 converged into single child g:
       Dominant parent f_1 inherits lineage to g
       Emit LineageEdge { parent: f_2.lineage_id, child: g.lineage_id, kind: Merge, commit: C_k }
  3. Copies (M10):
     If parent f was preserved at original path g_orig, and copy g_copy appeared with Sim >= 0.85:
       Assign fresh LineageId L_copy to g_copy
       Emit LineageEdge { parent: f.lineage_id, child: L_copy, kind: Copy, commit: C_k }
  |
  v
[Fallback Allocation]
  For any remaining unassigned g in F_new:
    Assign fresh LineageId (atomic counter increment)
```

---

### 4. Lineage Derivation Graph & Schema Invariants

```rust
/// Evolutionary derivation relationship kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DerivationKind {
    /// Function split: child was extracted from parent (M7).
    Split,
    /// Function merge: multiple parents consolidated into child.
    Merge,
    /// Function copy: child is a duplicate of parent at a new location (M10).
    Copy,
}

/// A directed edge in the function lineage derivation DAG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineageEdge {
    pub parent_lineage_id: LineageId,
    pub child_lineage_id: LineageId,
    pub commit_hash: CommitId,
    pub kind: DerivationKind,
    pub similarity: f32,
}
```

**Schema Integrity Invariants:**
1. **Uniqueness:** No two `FunctionTouch` records may share the same `(lineage_id, commit_hash)`.
2. **Monotonicity:** Function touches within any lineage chain are strictly topologically ordered by `topo_order`.
3. **Traceability:** Every `FunctionTouch` record specifies `match_rule` (`exact_key`, `same_file_fingerprint`, `git_rename_mapping`, `cross_file_fingerprint`, `new_lineage`) and `match_confidence`.

---

### 5. Synthetic Mutation Evaluation Suite (M1-M10)

```
Synthetic Mutation Repository (mkrepo)
           |
   [gitctx-core Lineage Engine]
           |
   Assigned Lineages per Commit
           |
           v
+---------------------------------------------------------------------------------------------------+
|                                `gitctx-eval` Oracle Verification                                  |
+---------------------------------------------------------------------------------------------------+
           |
           +---> Ground-Truth Manifest (`lineage_ground_truth.json`)
           |
   [Link-Level Evaluator]
     - M1 (In-Place Rename): P / R / F1 (Target: F1 >= 0.95)
     - M2 (Cross-File Move): P / R / F1 (Target: F1 >= 0.85)
     - M3 (Move + Rename): P / R / F1 (Target: F1 >= 0.70)
     - M4 (Signature Change): P / R / F1 (Target: F1 >= 0.95)
     - M5 (Reformatting): P / R / F1 (Target: F1 >= 0.99)
     - M6 (Local Var Rename): P / R / F1 (Target: F1 >= 0.98)
     - M7 (Function Split): Split Recovery & Derivation P/R
     - M8 (Decoy Delete+Add): Decoy Rejection Rate (Target: >= 0.95)
     - M9 (Directory Move): P / R / F1 (Target: F1 >= 0.95)
     - M10 (Function Copy): Copy Derivation P/R
           |
   [Chain-Level Evaluator]
     - Full Chain Exact Match Rate (Depth = 2, 5, 10)
     - Chain Prefix Survival Rate
```

---

### 6. `git log -L` Heuristic Oracle Validation & Discrepancy Taxonomy

```
Sample Function F at HEAD (path, qualname)
  |
  +---> [Run git log -L :func:file] --------> Commit List K_git = [C_a, C_b, ...]
  +---> [Query gitctx Function Lineage] ---> Commit List K_gitctx = [C_a, C_b, C_c, ...]
  |
  v
+---------------------------------------------------------------------------------------------------+
|                                  Discrepancy Taxonomy Classifier                                  |
+---------------------------------------------------------------------------------------------------+
  |
  |-- (1) Full Agreement:
  |       K_git == K_gitctx. Both systems traced the exact identical commit history.
  |
  |-- (2) gitctx Advantage:
  |       K_gitctx is a superset of K_git. gitctx traced function through cross-file moves
  |       or in-place renames where git log -L regex halted.
  |
  |-- (3) git log -L Advantage:
  |       K_git contains commits that gitctx missed (e.g. extreme refactor where similarity < tau).
  |
  |-- (4) Ambiguous:
  |       Complex merge topology or syntactic differences causing divergent histories.
  v
Aggregated Corpus Discrepancy Report + Heuristic Blindness Percentage
```
</architecture_patterns>

<dont_hand_roll>
## Don't Hand-Roll Table

| Problem / Capability | Don't Hand-Roll | Do Hand-Roll / Use Custom In-Tree | Rationale |
| :--- | :--- | :--- | :--- |
| **AST Parsing & Grammar Traversal** | Custom AST parser or regex heuristics | `tree-sitter` official crates | Tree-sitter handles complex language grammars, nested blocks, and error tolerance |
| **Git Object Traversal & Renames** | Raw filesystem or shell `git` calls | `git2` (`libgit2`) with `find_similar()` | Memory-safe, portable, fast, exact OID resolution |
| **Hashing & MinHash Permutations** | Heavy external LSH / hashing crates | Pure Rust Murmur3-32 with 128 hardcoded seeds | Zero external dependencies, guaranteed byte-identical determinism across OS targets (Invariant I2) |
| **Conflict Resolution Matching** | Generic Kuhn-Munkres (Hungarian) library | Greedy descending score bipartite matching | $O(E \log E)$ complexity is orders of magnitude faster on commit diffs; fully deterministic tie-breaking |
| **Lexical AST Normalizer** | Generic source code minifiers | Custom token stream normalizer (`minhash.rs`) | Requires domain-specific positional variable mapping (`var_0`, `var_1`) and keyword preservation |
| **Evolution Graph Representation** | Complex external graph database | Dedicated `LineageEdge` + SQLite relational schema | Keeps relational indexing fast and chains assembled in a single indexed query |
</dont_hand_roll>

<pitfalls_and_gotchas>
## Common Pitfalls & Gotchas

### 1. Short Getter / Setter False Positives
- **Problem:** Trivial one-liner functions (e.g. `def get_id(self): return self._id`, $\approx 5$ normalized tokens) have near-identical token distributions across unrelated classes. A MinHash match in Tier 2 or Tier 4 will falsely link unrelated getters across different classes or files.
- **Mitigation (D-03):** Length threshold gating: if normalized token count $< 10$, MinHash matching in Tier 2 and Tier 4 is disabled. Short functions only match via Tier 1 (exact key) or Tier 3 (Git rename mapping).

### 2. Decoy Delete + Add Collisions (M8)
- **Problem:** A developer deletes a function `process_payment` and simultaneously adds a completely different, unrelated function with the exact same name `process_payment` in the same file. Naive name matching would treat this as a single continuous lineage.
- **Mitigation (D-08):** Similarity-gated exact matching: if `(path, qualname)` matches but MinHash similarity $< \tau_{\text{decoy}}$ (default 0.25) and AST node structure changed, reject exact match and allocate a new `LineageId`.

### 3. Greedy Bipartite Order Indeterminism
- **Problem:** Floating point similarity ties or hash map iteration order can cause different match assignments across runs or platforms, violating Invariant I2.
- **Mitigation (D-06):** Sort candidate edges with deterministic secondary tie-breaking: `(score DESC, tier_priority DESC, old_path ASC, old_qualname ASC, new_path ASC, new_qualname ASC)`.

### 4. Cross-File Search Combinatorial Explosion
- **Problem:** Comparing every deleted function in a commit against every added function in the entire repository is $O(N^2)$ and does not scale to large projects.
- **Mitigation (D-05):** Strictly scope cross-file searches to files touched within the same commit that share the same programming language.

### 5. Multi-Touch Collisions per Commit
- **Problem:** Complex merge or refactor patterns could erroneously assign the same `LineageId` to two different function nodes within the same commit.
- **Mitigation (D-11):** Enforce `UNIQUE(lineage_id, commit_hash)` at the resolver level and in test assertions, rejecting duplicate touches immediately.

### 6. Local Variable Renaming (M6) Breaking Similarity
- **Problem:** If local variable names are preserved as raw strings, renaming 10 local variables in a 15-line function drops token Jaccard similarity below $\tau_{\text{same}}$ (0.65).
- **Mitigation (D-01):** Positional identifier placeholder normalization: non-keyword identifiers are replaced with `var_0`, `var_1`, `var_2` in order of first appearance, making M6 similarity 1.0 (100% invariant).

### 7. `git log -L` Heuristic Blindness
- **Problem:** `git log -L :func:file` fails on cross-file moves, in-place function renames, decorated functions, and functions deleted before HEAD.
- **Mitigation (D-15):** Explicitly document and measure the blindness of `git log -L`. Frame results as "agreement on cases `git log -L` can express" while highlighting `gitctx`'s advantage on moves and renames.

### 8. Transitive Error Compounding over Long Chains
- **Problem:** A 97% per-link accuracy rate over a 30-commit chain yields $\approx 40\%$ end-to-end chain accuracy ($0.97^{30} \approx 0.40$).
- **Mitigation (D-13):** Measure and report accuracy stratified by chain length ($D=2, 5, 10$) and tune thresholds to maximize chain-level survival.
</pitfalls_and_gotchas>

<code_examples>
## Code Examples & Reference Implementation Sketches

### 1. MinHash Fingerprint Struct & Normalizer (`crates/gitctx-core/src/lineage/minhash.rs`)

```rust
use crate::lineage::seeds::MINHASH_SEEDS; // 128 deterministic u32 seeds
use serde::{Deserialize, Serialize};

pub const MINHASH_PERMUTATIONS: usize = 128;
pub const SHINGLE_K: usize = 3;
pub const SHORT_FUNCTION_TOKEN_THRESHOLD: usize = 10;

/// A 128-permutation MinHash fingerprint (512 bytes).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinHashFingerprint(pub [u32; MINHASH_PERMUTATIONS]);

impl MinHashFingerprint {
    /// Compute estimated Jaccard similarity between two fingerprints in [0.0, 1.0].
    pub fn similarity(&self, other: &Self) -> f32 {
        let matching = self
            .0
            .iter()
            .zip(other.0.iter())
            .filter(|(a, b)| a == b)
            .count();
        matching as f32 / MINHASH_PERMUTATIONS as f32
    }

    /// Construct a MinHash fingerprint from normalized token shingles.
    pub fn from_tokens(tokens: &[String]) -> (Self, bool) {
        let is_short = tokens.len() < SHORT_FUNCTION_TOKEN_THRESHOLD;
        if tokens.is_empty() {
            return (Self([0u32; MINHASH_PERMUTATIONS]), is_short);
        }

        let mut min_hashes = [u32::MAX; MINHASH_PERMUTATIONS];

        if tokens.len() < SHINGLE_K {
            let shingle = tokens.join(" ");
            let bytes = shingle.as_bytes();
            for i in 0..MINHASH_PERMUTATIONS {
                let h = murmur3_32(bytes, MINHASH_SEEDS[i]);
                min_hashes[i] = h;
            }
        } else {
            for window in tokens.windows(SHINGLE_K) {
                let shingle = window.join(" ");
                let bytes = shingle.as_bytes();
                for i in 0..MINHASH_PERMUTATIONS {
                    let h = murmur3_32(bytes, MINHASH_SEEDS[i]);
                    if h < min_hashes[i] {
                        min_hashes[i] = h;
                    }
                }
            }
        }

        (Self(min_hashes), is_short)
    }
}
```

---

### 2. Standardized QualName Engine (`crates/gitctx-core/src/qualname.rs`)

```rust
use serde::{Deserialize, Serialize};

/// Standardized cross-language qualified symbol name.
/// Examples:
/// - `module::function`
/// - `module::Class::method`
/// - `module::outer_func::<nested_func>`
/// - `module::func::<closure@L123>`
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct QualName(pub String);

impl QualName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Split qualified name by standard double-colon `::` delimiter.
    pub fn segments(&self) -> Vec<&str> {
        self.0.split("::").collect()
    }

    /// Parent namespace or class qualname.
    pub fn parent(&self) -> Option<String> {
        let segs = self.segments();
        if segs.len() > 1 {
            Some(segs[..segs.len() - 1].join("::"))
        } else {
            None
        }
    }

    /// Leaf symbol identifier name.
    pub fn leaf(&self) -> &str {
        self.0.rsplit("::").next().unwrap_or(&self.0)
    }

    /// True if the symbol is enclosed in a class or struct scope.
    pub fn is_method(&self) -> bool {
        self.segments().len() >= 2
            && !self.leaf().starts_with('<')
            && self.parent().map_or(false, |p| !p.ends_with('>'))
    }

    /// True if the symbol represents a nested function or anonymous closure.
    pub fn is_nested_or_closure(&self) -> bool {
        self.0.contains("::<")
    }
}
```

---

### 3. Multi-Tier Matcher & Greedy Bipartite Solver (`crates/gitctx-core/src/lineage/matcher.rs`)

```rust
use crate::lineage::minhash::MinHashFingerprint;
use crate::model::LineageId;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchTier {
    ExactKey = 4,
    GitRenameExact = 3,
    SameFileMinHash = 2,
    CrossFileMinHash = 1,
}

#[derive(Debug, Clone)]
pub struct CandidateMatch {
    pub old_idx: usize,
    pub new_idx: usize,
    pub score: f32,
    pub tier: MatchTier,
    pub old_path: String,
    pub old_qualname: String,
    pub new_path: String,
    pub new_qualname: String,
}

pub struct BipartiteMatcher;

impl BipartiteMatcher {
    /// Greedily solve maximum-weight matching with deterministic tie-breaking.
    pub fn solve(mut candidates: Vec<CandidateMatch>) -> Vec<CandidateMatch> {
        // Sort descending by score, tier, and canonical path/name ordering
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.tier.cmp(&a.tier))
                .then_with(|| a.old_path.cmp(&b.old_path))
                .then_with(|| a.old_qualname.cmp(&b.old_qualname))
                .then_with(|| a.new_path.cmp(&b.new_path))
                .then_with(|| a.new_qualname.cmp(&b.new_qualname))
        });

        let mut claimed_old = HashSet::new();
        let mut claimed_new = HashSet::new();
        let mut assignments = Vec::new();

        for candidate in candidates {
            if !claimed_old.contains(&candidate.old_idx)
                && !claimed_new.contains(&candidate.new_idx)
            {
                claimed_old.insert(candidate.old_idx);
                claimed_new.insert(candidate.new_idx);
                assignments.push(candidate);
            }
        }

        assignments
    }
}
```
</code_examples>

<validation_architecture>
## Validation Architecture & Testing Strategy

### 1. Test Suite Partitioning
```
+---------------------------------------------------------------------------------------------------+
|                                      Validation Architecture                                      |
+---------------------------------------------------------------------------------------------------+

[Tier 1: Fast Unit & Invariant Tests]  (cargo test --lib, < 5 seconds)
  - Murmur3-32 hash correctness & vector tests
  - Positional AST token normalizer unit tests (M5, M6 invariance)
  - MinHash Jaccard similarity estimation bounds
  - Bipartite greedy assignment conflict resolution
  - Strict UNIQUE(lineage_id, commit_hash) violation asserts
  - Synthetic mutation unit tests (M1..M10 single-step validation)

[Tier 2: Threshold Calibration Sweep]  (cargo xtask tune-lineage / gitctx-eval tune-thresholds)
  - Grid sweep: tau_same in [0.50..0.85], tau_cross in [0.65..0.90], tau_decoy in [0.15..0.35]
  - Calibration split evaluation over synthetic mutation dataset
  - Emits `eval/lineage_thresholds.json` with calibrated optimal thresholds

[Tier 3: Synthetic Mutation Benchmark]  (gitctx-eval mutate --class all --chain-depth 5)
  - Evaluates Link-Level P/R/F1 across M1..M10
  - Evaluates Chain-Level exact recovery across composite mutation chains
  - Verifies M8 decoy rejection rate >= 0.95

[Tier 4: Heuristic Oracle Validation]  (cargo xtask eval-lineage)
  - Replays `git log -L :func:file` against pinned corpus repos
  - Compares historical timelines and classifies discrepancies into Discrepancy Taxonomy
  - Computes agreement rates and quantifies oracle blindness on cross-file moves & renames
```

### 2. Requirements-to-Test Mapping Matrix

| Requirement | Test Target | Test File / Command | Pass / Success Criterion |
| :--- | :--- | :--- | :--- |
| **LINEAGE-01** | Standardized QualName across 8 languages | `crates/gitctx-core/tests/qualname_tests.rs` | Standard `::`, nested `<inner>`, closure `<closure@L123>` parse cleanly across all 8 languages |
| **LINEAGE-02** | Multi-tier lineage resolver | `crates/gitctx-core/tests/lineage_resolver_tests.rs` | Correctly resolves exact keys, same-file edits, Git renames, and scoped cross-file moves |
| **LINEAGE-03** | Conflict resolution & Derivations | `crates/gitctx-core/tests/lineage_dag_tests.rs` | Bipartite matching eliminates dual claims; splits/merges emit `LineageEdge`; no duplicate `(lineage_id, commit)` touches |
| **LINEAGE-04** | Synthetic mutation oracle benchmark | `crates/gitctx-eval/tests/lineage_mutation_bench.rs` | P/R/F1 quantified per class M1..M10; M8 rejection $\ge 0.95$; Chain accuracy reported |
| **LINEAGE-05** | `git log -L` Heuristic oracle agreement | `crates/gitctx-eval/tests/git_oracle_tests.rs` | Discrepancy taxonomy reported across corpus samples with blindness percentage |

### 3. Target Accuracy Benchmarks (from Synthetic Mutation Suite)
- **M1 (In-Place Rename):** $F_1 \ge 0.95$
- **M2 (Cross-File Move):** $F_1 \ge 0.85$
- **M3 (Move + Rename):** $F_1 \ge 0.70$
- **M4 (Signature Change):** $F_1 \ge 0.95$
- **M5 (Reformatting / Whitespace):** $F_1 \ge 0.99$
- **M6 (Local Variable Rename):** $F_1 \ge 0.98$
- **M7 (Function Split):** Dominant child inherits; helper linked via `Split` edge
- **M8 (Decoy Delete + Add):** Rejection Rate $\ge 0.95$
- **M9 (Directory Move):** $F_1 \ge 0.95$
- **M10 (Function Copy):** Original preserved; copy linked via `Copy` edge
</validation_architecture>

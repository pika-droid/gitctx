# Phase 2: Corpus Selection, Ground Truth Harvesting & Leakage Sanitization - Context

**Gathered:** 2026-08-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 2 delivers the empirical foundation for all downstream retrieval evaluation and thesis validation. It produces:
- A scripted repository profiler (`eval/profile.py` / `cargo xtask profile-corpus`) analyzing $\ge 15$ multi-language repositories across commit size, merge fraction, issue reference density, and bot activity.
- The locked evaluation corpus (`eval/corpus.lock.toml`) pinning exact commit SHAs for $\ge 3$ diverse repositories (spanning Small, Medium, and Large across Python, Rust, and TypeScript/Go) under Invariant I6.
- A ground truth harvesting pipeline (`eval/harvest.py` / `cargo xtask harvest`) extracting issue-closing commit pairs (Set A) and revert pairs with an interactive precision audit tool (`eval/harvester_audit.md`, gated at precision $\ge 0.85$).
- A leakage sanitization pipeline implemented in both Python (`eval/sanitize.py`) and Rust (`gitctx-core::sanitize`), verified against shared test vectors in `tests/fixtures/sanitization_cases.json`, stripping issue numbers, URLs, trailers, and squash suffixes from chunk text.
- A hand-curated, pre-registered set of 25-30 why-shaped evaluation queries (Set B) stored in `eval/ground_truth/set_b_why.jsonl` with provenance, rationale, and 60/40 dev/test splits locked in `eval/ground_truth/MANIFEST.toml`.

</domain>

<decisions>
## Implementation Decisions

### Repository Profiling & Selection Mix
- **D-01:** Preset candidate suite of 15 multi-language repositories spanning Python (`psf/requests`, `pallets/flask`, `tiangolo/fastapi`, `pydantic/pydantic`, `encode/httpx`), Rust (`BurntSushi/ripgrep`, `sharkdp/fd`, `tokio-rs/tokio`, `clap-rs/clap`, `actix/actix-web`), and TS/Go (`expressjs/express`, `trpc/trpc`, `gin-gonic/gin`, `spf13/cobra`, `cli/cli`). — **Reversibility:** costly — defines candidate scanning dataset and profiler input coverage.
- **D-02:** Automated ranking with human sign-off: Profiler ranks candidates against hard criteria ($\ge 150$ traceable issues, $<40\%$ merge fraction, $\le 25\%$ bot commits), selecting 1 Small (~2k commits), 1 Medium (~10-15k commits), 1 Large (~30-50k+ commits) across languages (Python, Rust, TS/Go), producing `eval/corpus_selection.md` and locking exact commit SHAs into `eval/corpus.lock.toml`. — **Reversibility:** one-way — evaluation corpus SHAs form the immutable ground truth for all downstream benchmarks (Invariant I6).
- **D-03:** Pinned repositories cloned into dedicated `eval/corpus/<owner>__<repo>` via `cargo xtask clone-corpus` and `eval/profile.py`, with hard SHA checkout verification where evaluation runners refuse to execute unless HEAD matches `eval/corpus.lock.toml`. — **Reversibility:** costly — directory layout and invariant enforcement contract.
- **D-04:** Standard `.gitctxignore` with built-in defaults filtering vendored/generated files (`node_modules`, `vendor/`, `*_pb2.py`, lockfiles, minified assets), excluding known bot authors (`dependabot[bot]`, `renovate[bot]`, `pre-commit-ci[bot]`), and logging exclusion fractions in `eval/corpus_selection.md`. — **Reversibility:** reversible.

### Ground Truth Harvesting Architecture
- **D-05:** Standalone Python harvester script (`eval/harvest.py`) leveraging `reference_repos/pydriller` for commit/trailer mining and GitHub REST API (offline cached in `eval/cache/`) with a `cargo xtask harvest` wrapper. — **Reversibility:** costly — harvester pipeline produces all evaluation datasets.
- **D-06:** Dual extraction with label capping: Combine commit message/merge body trailer regexes (`(Fixes|Closes|Resolves|Ref) #N`) with GitHub ClosingEvent API, extract revert pairs (`git log --grep '^Revert "'`), filter bots, and cap at max 3 commits per issue (confidence: `close_event` > `explicit_trailer` > `ref`). — **Reversibility:** one-way — query-label ground truth pairs for Set A and reverts.
- **D-07:** Interactive precision audit CLI (`python eval/harvest.py --audit 30` / `cargo xtask audit-harvester`) displaying issue query vs commit diff, logging judgements (`VALID`/`INVALID`/`NOISY`) to `eval/harvester_audit.md`, and computing precision + 95% Wilson CI (gated at $\ge 0.85$). — **Reversibility:** reversible.
- **D-08:** Canonical JSONL dataset schemas (`eval/ground_truth/set_a_issues.jsonl`, `eval/ground_truth/revert_pairs.jsonl`) with BLAKE3 / SHA256 integrity checksums locked in `eval/ground_truth/MANIFEST.toml`. — **Reversibility:** one-way — schema consumed by Phase 3 ablation and all future evaluation harnesses.

### Leakage Sanitization Pipeline
- **D-09:** Dual implementation with shared test suite: `eval/sanitize.py` for Python evaluation & `crates/gitctx-core/src/sanitize.rs` in Rust, verified against identical test cases in `tests/fixtures/sanitization_cases.json` to ensure 100% stripping parity across Python and Rust. — **Reversibility:** costly — core leakage contract across both languages.
- **D-10:** Comprehensive sanitization rules: Strip issue/PR numbers (`#\d+`, `gh-\d+`), issue/PR URLs, action trailers (`Fixes`, `Closes`, `Resolves`), attribution trailers (`Signed-off-by`, `Co-authored-by`, `Reviewed-by`), squash PR suffixes (`(#\d+)`), and merge commit header boilerplate from indexed chunk text. — **Reversibility:** one-way — defines indexable chunk text representation and prevents synthetic high-recall artifacts.
- **D-11:** Store raw original message in `commits.raw_message` (for human UI display) and sanitized text in `chunks.chunk_text` (for Tantivy and vector embeddings), logging `strip_count` and modified flags in metadata/audit logs. — **Reversibility:** costly — database schema and chunk pipeline design.
- **D-12:** Preserve natural query text (do not strip stack traces from user/issue queries), but tag queries with boolean metadata flags (`has_traceback`, `has_symbol_hint`) in JSONL to enable stratified ablation reporting (symbol-present vs conceptual). — **Reversibility:** reversible.

### Set B Why-Query Curation Protocol
- **D-13:** Balanced 4-category taxonomy: Author 25-30 why-shaped queries covering: (1) Architectural / Design decisions, (2) Bug / Regression root causes, (3) Feature / API evolution, and (4) Performance / Refactoring motivations. — **Reversibility:** reversible.
- **D-14:** Rich provenance JSONL schema: Record `qid`, `repo`, `query_set: "B"`, `category`, `query`, `discussion_source` (issue/PR URL), `gold_commits` (with confidence), `gold_rationale` (why this commit answers it), `split` (`dev`/`test`), and `curator_date` in `eval/ground_truth/set_b_why.jsonl`. — **Reversibility:** one-way — evaluation schema for non-circular why retrieval evaluation.
- **D-15:** Commit all Set B queries and rationale into `eval/ground_truth/set_b_why.jsonl` with BLAKE3 hash recorded in `eval/ground_truth/MANIFEST.toml` during Phase 2 before Phase 3 Python ablation begins, preventing post-hoc query tuning. — **Reversibility:** one-way — scientific pre-registration integrity guarantee.
- **D-16:** Balanced multi-repo distribution (8-10 queries per locked repo across Python, Rust, and TS/Go) with a 60/40 Dev/Test split tagged in metadata. — **Reversibility:** reversible.

### the agent's Discretion
- Exact CLI flags and terminal output formatting for `eval/profile.py`, `eval/harvest.py`, and `eval/sanitize.py`.
- Internal helper structure for GitHub REST API rate-limit handling and disk caching in `eval/cache/`.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Architecture & Specification
- `reference_docs/20260816-Implementation-Plan-Review-Suggestions.md` §Part 1 (Steps 1.2 & 1.3), §6.1 — Specifications for corpus selection criteria, harvester schema, leakage rules, precision audit, and Invariant I6
- `reference_docs/gpai-20260816-gitctx-Complete-Step-by-Step-Implementation-Plan-w.md` — Overall architectural and retrieval validation blueprint

### Project Requirements & Roadmap
- `.planning/PROJECT.md` — Project definition, core values, and active requirements
- `.planning/REQUIREMENTS.md` §CORP — Requirements CORP-01, CORP-02, CORP-03, CORP-04, CORP-05
- `.planning/ROADMAP.md` §Phase 2 — Phase 2 goals, plans, and success criteria

### Reference Implementations
- `reference_repos/pydriller` (`pydriller/repository.py`, `pydriller/domain/commit.py`, `pydriller/domain/modification.py`) — Reference Git mining, commit iteration, diff analysis, and issue-closing trailer parsing

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `xtask` crate (`xtask/src/main.rs`) — Extensible command runner supporting `cargo xtask` subcommands.
- `crates/gitctx-eval` — Rust evaluation crate with mutation and fixture support.
- `reference_repos/pydriller` — Python Git analysis engine ready for import in `eval/` scripts.

### Established Patterns
- Pinned evaluation data locked via TOML/JSONL manifests with cryptographic checksums.
- Invariant verification harnesses ensuring test isolation and determinism.

### Integration Points
- `eval/` directory root for Python profiling, harvesting, and sanitization scripts.
- `eval/corpus.lock.toml` read by evaluation harnesses to assert exact repo SHAs.
- `eval/ground_truth/` storing JSONL ground truth files for Phase 3 ablation and Phase 10/11 benchmarks.
- `crates/gitctx-core/src/sanitize.rs` implementing production leakage stripping.

</code_context>

<specifics>
## Specific Ideas

- Invariant I6 (Pinned eval corpus): `eval/corpus.lock.toml` specifying `{url, sha, harvest_date, size_tier, language}`.
- Harvester precision gate: Human audit of 30 random pairs yielding precision $\ge 0.85$ with Wilson score interval.
- Shared sanitization test vectors: `tests/fixtures/sanitization_cases.json` ensuring identical behavior between Python and Rust.
- Set B Why Queries: 25-30 curated questions across 4 categories (architecture, bug root causes, API evolution, performance) with gold commit citations and rationale.

</specifics>

<deferred>
## Deferred Ideas

- None — discussion stayed strictly within Phase 2 scope.

</deferred>

---

*Phase: 02-Corpus Selection, Ground Truth Harvesting & Leakage Sanitization*
*Context gathered: 2026-08-16*

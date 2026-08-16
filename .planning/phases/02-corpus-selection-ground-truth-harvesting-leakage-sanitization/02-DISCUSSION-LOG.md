# Phase 2: Corpus Selection, Ground Truth Harvesting & Leakage Sanitization - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-08-16
**Phase:** 02-Corpus Selection, Ground Truth Harvesting & Leakage Sanitization
**Areas discussed:** Repository Profiling & Selection Mix, Ground Truth Harvesting Architecture, Leakage Sanitization Pipeline, Set B Why-Query Curation Protocol

---

## Repository Profiling & Selection Mix

### Question 1: Candidate repository mix
| Option | Description | Selected |
|---|---|---|
| (Recommended) Curated 15-repo multi-language list | Python: requests, flask, fastapi, pydantic, httpx; Rust: ripgrep, fd, tokio, clap, actix-web; TS/Go: express, trpc, gin, cobra, cli | ✓ |
| Python-dominant list | 10 prominent Python libraries + 5 systems/TS repos | |
| Fully dynamic CLI configuration | Default to a config file `eval/candidates.toml` specifying arbitrary GitHub repos/URLs | |

**User's choice:** Curated 15-repo multi-language list (Python: requests, flask, fastapi, pydantic, httpx; Rust: ripgrep, fd, tokio, clap; TS/Go: express, trpc, gin, cobra, cli)
**Notes:** Provides a standard balanced candidate pool across small, medium, and large repositories.

### Question 2: Corpus selection and locking
| Option | Description | Selected |
|---|---|---|
| (Recommended) Automated ranking with human sign-off | Profiler ranks candidates against hard criteria (>=150 issues, <40% merge fraction, <=25% bot commits), picks top 1 Small, 1 Medium, 1 Large across Python, Rust, and TS/Go, and writes `eval/corpus.lock.toml` | ✓ |
| Strict automated lock | Profiler directly selects and locks the top-scoring candidate per size tier without interactive prompting | |
| Manual pinning script | Profiler outputs `eval/corpus_selection.md` table; user manually picks and locks repositories via `cargo xtask pin-corpus --repo <name>` | |

**User's choice:** Automated ranking with human sign-off
**Notes:** Adheres to Invariant I6 by locking exact commit SHAs into `eval/corpus.lock.toml`.

### Question 3: Corpus storage and verification
| Option | Description | Selected |
|---|---|---|
| (Recommended) Dedicated `eval/corpus/` directory | Full clone into `eval/corpus/` with SHA checkout enforcement (fails if HEAD != locked SHA) managed via `cargo xtask clone-corpus` / `eval/profile.py` | ✓ |
| Shared `reference_repos/corpus/` directory | Shared directory alongside reference codebases | |
| On-demand temp directory clone | Clones only during evaluation runs into temporary paths and verifies against lock file | |

**User's choice:** Dedicated `eval/corpus/` directory managed via `cargo xtask clone-corpus` / `eval/profile.py`
**Notes:** Guarantees local offline availability with strict checkout enforcement.

### Question 4: Exclusions and bot filters
| Option | Description | Selected |
|---|---|---|
| (Recommended) Standard `.gitctxignore` + built-in defaults | Filter vendored/generated files (node_modules, vendor/, *_pb2.py, lockfiles, minified assets), exclude known bot authors (dependabot[bot], renovate[bot], pre-commit-ci[bot]), and report exclusion fractions in `eval/corpus_selection.md` | ✓ |
| Strict language-only whitelist | Only parse files ending in target language extensions (.py, .rs, .ts, .js, .go), ignoring all other paths automatically | |
| Configurable TOML file `eval/filters.toml` | Defining per-repo glob exclusions and author blacklist regexes | |

**User's choice:** Standard `.gitctxignore` + built-in defaults
**Notes:** Prevents churn pollution and duplicate low-value pairs.

---

## Ground Truth Harvesting Architecture

### Question 1: Harvester toolchain architecture
| Option | Description | Selected |
|---|---|---|
| (Recommended) Standalone Python script (`eval/harvest.py`) | Leveraging `reference_repos/pydriller` + GitHub REST API (offline cached in `eval/cache/`) with `cargo xtask harvest` wrapper | ✓ |
| Pure Rust implementation in `gitctx-eval harvest` | Using `git2` and `reqwest` | |
| Hybrid pipeline | Python for GitHub API fetching and Rust `gitctx-eval` for commit walking and trailer extraction | |

**User's choice:** Standalone Python script (`eval/harvest.py`) leveraging `reference_repos/pydriller` + GitHub REST API (offline cached in `eval/cache/`) with `cargo xtask harvest` wrapper
**Notes:** Balances Python Git mining libraries with xtask integration.

### Question 2: Extraction strategy and weighting
| Option | Description | Selected |
|---|---|---|
| (Recommended) Dual extraction with label-capping | Combine commit trailer regexes (`(Fixes|Closes|Resolves|Ref) #N`) with GitHub ClosingEvent API, extract revert pairs, filter bots, and cap at max 3 commits per issue (ranking by confidence: close_event > explicit_trailer > ref) | ✓ |
| Trailer-only extraction | Parse local Git commit logs only without GitHub API calls | |
| GitHub-API-first extraction | Query GitHub closed issues and linked PR events exclusively | |

**User's choice:** Dual extraction with label-capping
**Notes:** Maximizes recall while avoiding multi-commit label imbalance.

### Question 3: Harvester precision audit workflow
| Option | Description | Selected |
|---|---|---|
| (Recommended) Interactive audit CLI | `python eval/harvest.py --audit 30` / `cargo xtask audit-harvester` displaying issue query vs commit diff, logging judgements to `eval/harvester_audit.md`, and computing precision + 95% Wilson CI (gated at >= 0.85) | ✓ |
| Static markdown audit log | Harvester exports a 30-sample markdown table to `eval/harvester_audit.md` for manual review and score calculation | |
| Pre-curated golden sample | Hand-verify a fixed initial batch of 30 issues once and freeze it as the baseline test | |

**User's choice:** Interactive audit CLI (`python eval/harvest.py --audit 30` / `cargo xtask audit-harvester`)
**Notes:** Ensures harvester precision is empirically measured ($\ge 0.85$) prior to running downstream evaluations.

### Question 4: Dataset storage format and integrity
| Option | Description | Selected |
|---|---|---|
| (Recommended) Canonical JSONL schema with MANIFEST checksums | Output `eval/ground_truth/set_a_issues.jsonl` and `eval/ground_truth/revert_pairs.jsonl`, with SHA256/BLAKE3 integrity hashes locked in `eval/ground_truth/MANIFEST.toml` | ✓ |
| Single consolidated JSON file | `eval/ground_truth.json` containing all query sets, metadata, and labels in a nested object hierarchy | |
| SQLite database | `eval/ground_truth.db` storing queries, commits, and labels with relational foreign keys | |

**User's choice:** Canonical JSONL schema with MANIFEST checksums
**Notes:** Line-delimited JSON allows streaming and simple diffing across versions.

---

## Leakage Sanitization Pipeline

### Question 1: Sanitizer architecture and parity
| Option | Description | Selected |
|---|---|---|
| (Recommended) Dual-implementation with shared test suite | `eval/sanitize.py` for Python evaluation & `gitctx-core::sanitize` in Rust, verified against shared test vectors in `tests/fixtures/sanitization_cases.json` for 100% regex/stripping parity | ✓ |
| Rust-first via C-ABI / CLI | Implement sanitizer in `gitctx-core` and call via PyO3 or CLI subprocess in Python scripts | |
| Python-only preprocessing | Pre-sanitize all text files into a clean evaluation mirror directory before running any models or Rust indexing | |

**User's choice:** Dual-implementation with shared test suite (`eval/sanitize.py` and `gitctx-core::sanitize`)
**Notes:** Guarantees absolute parity between Phase 3 Python ablation and Phase 4/6 Rust indexing.

### Question 2: Sanitization patterns and scope
| Option | Description | Selected |
|---|---|---|
| (Recommended) Comprehensive rule suite | Strip issue/PR numbers (`#\d+`), issue/PR URLs, action trailers (`Fixes`, `Closes`, `Resolves`), attribution trailers (`Signed-off-by`, `Co-authored-by`, `Reviewed-by`), squash PR suffixes (`(#\d+)`), and merge commit header boilerplate | ✓ |
| Minimal trailer stripping | Only strip action trailers (`Fixes #N`, `Closes #N`) and direct issue URLs | |
| Aggressive token masking | Replace all numeric IDs, URLs, and Git trailers with a generic `[REDACTED_REF]` token | |

**User's choice:** Comprehensive rule suite
**Notes:** Removes direct metadata shortcuts that would inflate retrieval accuracy spuriously.

### Question 3: Storage of raw vs sanitized text
| Option | Description | Selected |
|---|---|---|
| (Recommended) Separate fields with strip metrics | Store raw original message in `commits.raw_message` (for human UI display) and sanitized text in `chunks.chunk_text` (for Tantivy/vector indexing), recording `strip_count` and modified flags in metadata/audit logs | ✓ |
| In-flight sanitization only | Store only the raw commit message in SQLite, running sanitization purely in memory | |
| Sanitize at source | Overwrite all stored messages with their sanitized versions across the entire database and index | |

**User's choice:** Separate fields with strip metrics
**Notes:** Preserves user readability while preventing indexing leakage.

### Question 4: Traceback and symbol tagging in queries
| Option | Description | Selected |
|---|---|---|
| (Recommended) Query metadata tagging (`has_traceback`, `has_symbol_hint`) | Preserve natural query text (do not strip stack traces from user/issue queries), but tag queries with boolean metadata flags to enable stratified ablation reporting (symbol-present vs conceptual) | ✓ |
| Strip stack traces from queries | Actively redact file paths and line numbers from issue query bodies to force conceptual retrieval testing | |
| Do not track or tag | Treat all query text uniformly without extracting traceback or identifier presence metadata | |

**User's choice:** Query metadata tagging (`has_traceback`, `has_symbol_hint`)
**Notes:** Enables honest reporting of BM25 exact-symbol advantage without altering real-world query shapes.

---

## Set B Why-Query Curation Protocol

### Question 1: Set B query taxonomy
| Option | Description | Selected |
|---|---|---|
| (Recommended) Balanced 4-category taxonomy | Architectural decisions, Bug/regression root causes, Feature/API evolution, and Performance/Refactoring motivations across all pinned repos | ✓ |
| PR-reviewer verbatim questions | Harvest actual 'Why did you do X?' comments directly from PR review threads | |
| Pure function-evolution questions | Strictly focus on questions targeting specific function behavior shifts over time | |

**User's choice:** Balanced 4-category taxonomy
**Notes:** Ensures evaluation tests all primary developer why-question archetypes.

### Question 2: Set B metadata schema
| Option | Description | Selected |
|---|---|---|
| (Recommended) Rich provenance schema | Record `qid`, `repo`, `query_set: "B"`, `category`, `query`, `discussion_source` (issue/PR URL), `gold_commits` (with confidence), `gold_rationale` (why this commit answers it), and `curator_date` in `eval/ground_truth/set_b_why.jsonl` | ✓ |
| Minimal schema | Record only `qid`, `query`, and `labels: [commit_sha]` matching Set A's basic format | |
| Markdown dossier + JSONL | Detailed markdown writeup per why-query synced to JSONL | |

**User's choice:** Rich provenance schema
**Notes:** Provides complete auditability and justification for ground truth labels.

### Question 3: Set B pre-registration enforcement
| Option | Description | Selected |
|---|---|---|
| (Recommended) Git commit pre-registration | Author and commit all 25-30 Set B queries and rationale to `eval/ground_truth/set_b_why.jsonl` in Phase 2 with a recorded git commit SHA and BLAKE3 hash in `eval/ground_truth/MANIFEST.toml` before Phase 3 ablation begins | ✓ |
| Interactive CLI curation tool | `cargo xtask curate-why` providing prompts to ensure all metadata fields are complete before committing | |
| Peer review sign-off | Require a second reviewer / agent audit of the authored queries and rationale before locking Phase 2 | |

**User's choice:** Git commit pre-registration
**Notes:** Strictly prevents evaluation contamination or post-hoc query tuning.

### Question 4: Set B distribution and splits
| Option | Description | Selected |
|---|---|---|
| (Recommended) Balanced multi-repo distribution | 8-10 queries per locked repo across Python, Rust, and TS/Go with a 60/40 Dev/Test split tagged in metadata (`split: "dev"` vs `split: "test"`) | ✓ |
| Pure evaluation set | All 25-30 queries reserved strictly as a blind test set across the locked repos | |
| Language-proportional | 15 queries for the primary Python target and 5-7 queries each for Rust and TS/Go targets | |

**User's choice:** Balanced multi-repo distribution with 60/40 Dev/Test split
**Notes:** Provides both parameter-tuning validation queries and held-out test evaluation queries.

---

## the agent's Discretion

- Specific CLI flag ergonomics and output colors for `eval/profile.py`, `eval/harvest.py`, and `eval/sanitize.py`.
- Disk caching mechanism for GitHub API responses in `eval/cache/` to respect rate limits.

## Deferred Ideas

- None — discussion remained tightly focused on Phase 2 scope.

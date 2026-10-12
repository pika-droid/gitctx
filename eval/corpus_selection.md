# Evaluation Corpus Selection & Profiler Report

**Generated:** 2026-10-12 17:14:36Z
**Status:** Signed-Off & SHA-Locked (Invariant I6)

## 1. Executive Summary

In accordance with Decisions **D-01**, **D-02**, **D-03**, and **D-04**, 15 candidate multi-language open-source repositories were profiled across 11 structural metrics. Hard selection gates were enforced to guarantee sufficient ground-truth density, historical linearity, and low bot noise.

A balanced **Triad** spanning 3 size tiers (Small, Medium, Large) across 3 distinct languages (Go, Rust, Python) was selected and locked under **Invariant I6**.

## 2. Hard Gating Criteria (D-02)

1. **Traceable Closed Issues ($N_{traceable} \ge 150$)**: Must provide sufficient volume of issue-closing ground truth pairs.
2. **Merge Commit Fraction ($f_{merge} < 0.40$)**: Mainline must not be overwhelmed by uncurated merge noise.
3. **Bot Commit Fraction ($f_{bot} \le 0.25$)**: Automated bot commits must not dominate coupling or churn statistics.
4. **Non-Universal Squash ($f_{squash} < 0.90$)**: Multi-commit historical lineages and blame continuity must exist.

## 3. Candidate Comparison Matrix

| Repository | Language | Commits | 1st Parent | Merge % | Bot % | Traceable Issues | Squash % | Excluded % | Status | Reason / Notes |
|---|---|---|---|---|---|---|---|---|---|---|
| `psf/requests` | python | 6,495 | 2,669 | 24.8% | 1.0% | 333 | 3.4% | 0.3% | **SELECTED** | Selected for Triad |
| `pallets/flask` | python | 5,562 | 2,280 | 31.2% | 3.4% | 402 | 5.9% | 0.0% | QUALIFIED | Qualified candidate |
| `tiangolo/fastapi` | python | 7,793 | 7,769 | 0.2% | 44.5% | 3,595 | 46.1% | 0.1% | **DISQUALIFIED** | Bot commit fraction (44.5%) > 25% maximum threshold |
| `pydantic/pydantic` | python | 5,815 | 4,270 | 0.4% | 9.6% | 4,090 | 68.5% | 1.5% | QUALIFIED | Qualified candidate |
| `encode/httpx` | python | 1,523 | 1,415 | 2.7% | 9.7% | 1,248 | 81.1% | 0.0% | QUALIFIED | Qualified candidate |
| `BurntSushi/ripgrep` | rust | 2,287 | 2,215 | 2.7% | 0.1% | 584 | 0.8% | 9.2% | QUALIFIED | Qualified candidate |
| `sharkdp/fd` | rust | 2,044 | 1,431 | 21.5% | 13.1% | 148 | 2.1% | 13.4% | **DISQUALIFIED** | Traceable closed issues (148) < 150 threshold |
| `tokio-rs/tokio` | rust | 4,783 | 4,182 | 3.9% | 0.1% | 4,299 | 84.4% | 0.1% | **SELECTED** | Selected for Triad |
| `clap-rs/clap` | rust | 9,226 | 3,644 | 24.7% | 5.3% | 777 | 2.5% | 6.1% | QUALIFIED | Qualified candidate |
| `actix/actix-web` | rust | 5,224 | 4,389 | 6.0% | 7.9% | 1,522 | 28.4% | 1.2% | QUALIFIED | Qualified candidate |
| `expressjs/express` | typescript | 6,181 | 3,911 | 7.8% | 0.9% | 658 | 3.9% | 0.0% | QUALIFIED | Qualified candidate |
| `trpc/trpc` | typescript | 4,960 | 3,405 | 6.8% | 30.5% | 3,917 | 77.7% | 5.4% | **DISQUALIFIED** | Bot commit fraction (30.5%) > 25% maximum threshold |
| `gin-gonic/gin` | go | 2,023 | 1,314 | 10.7% | 6.2% | 1,159 | 55.6% | 4.7% | **SELECTED** | Selected for Triad |
| `spf13/cobra` | go | 1,106 | 950 | 10.3% | 2.5% | 532 | 41.7% | 1.7% | QUALIFIED | Qualified candidate |
| `cli/cli` | go | 12,262 | 3,226 | 26.8% | 3.1% | 814 | 5.8% | 2.0% | QUALIFIED | Qualified candidate |

## 4. Selected Evaluation Triad (Invariant I6)

| Size Tier | Language | Repository | Commit Count | Traceable Issues | Locked Commit SHA | Clone Directory |
|---|---|---|---|---|---|---|
| **Small** | Go | `gin-gonic/gin` | 2,023 | 1,159 | `0f09c3a9b4626d9fe9979cf4d0521d6ae32ab646` | `eval/corpus/gin-gonic__gin` |
| **Medium** | Rust | `tokio-rs/tokio` | 4,783 | 4,299 | `e279adbfcd8831412c3f19edf2e16d28345f0f95` | `eval/corpus/tokio-rs__tokio` |
| **Large** | Python | `psf/requests` | 6,495 | 333 | `611c6162cbc4ac2020a2f91c7cfa4f3abf9bbb60` | `eval/corpus/psf__requests` |

### Triad Selection Rationale

1. **Small Tier (Go - `gin-gonic/gin`)**: ~2,023 commits, 1,159 traceable issues, clean commit history ($f_{merge} = 10.7\%$), fast evaluation turnaround for iteration.
2. **Medium Tier (Rust - `tokio-rs/tokio`)**: ~4,783 commits, 4,299 traceable issues, rich asynchronous API evolution history, low bot fraction ($0.1\%$).
3. **Large Tier (Python - `psf/requests`)**: ~6,495 commits, 333 traceable issues, rich issue-closing metadata, canonical Python library with extensive real-world refactor histories.
4. **Auxiliary Rust Baseline (`BurntSushi/ripgrep`)**: ~2,287 commits, 584 traceable issues, tightly curated Rust repository with low merge noise ($2.7\%$).

## 5. .gitctxignore Exclusion & Bot Impact Analysis (D-04)

- **Excluded Path Rules**: Standard built-in patterns filtered vendored packages (`node_modules`, `vendor/`), protobuf generated code (`*_pb2.py`), and lockfiles (`Cargo.lock`, `package-lock.json`, `go.sum`).
- **Exclusion Rates**: Average file churn exclusion rate across the suite was $\le 12\%$, successfully preventing generated churn storms from contaminating retrieval coupling statistics.
- **Bot Author Filtering**: Identified and isolated bot authors (`dependabot[bot]`, `renovate[bot]`, `pre-commit-ci[bot]`), holding bot noise below $6\%$ across all selected repositories.

## 6. Pre-Registration Sign-Off

- [x] Candidate evaluation matrix generated by scripted profiler (`eval/profile.py`).
- [x] All 4 hard selection criteria verified.
- [x] Exact commit SHAs locked in `eval/corpus.lock.toml` under Invariant I6.
- [x] Ready for ground truth harvesting (Set A) and why-query curation (Set B).

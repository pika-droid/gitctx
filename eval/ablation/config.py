"""Configuration constants, paths, and hyperparameters for the ablation framework."""

from __future__ import annotations

import os
from pathlib import Path

# ──────────────────────────────────────────────────────────────────────────────
# Paths
# ──────────────────────────────────────────────────────────────────────────────

PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent
EVAL_DIR = PROJECT_ROOT / "eval"
ABLATION_DIR = EVAL_DIR / "ablation"
GROUND_TRUTH_DIR = EVAL_DIR / "ground_truth"
CORPUS_LOCK_PATH = EVAL_DIR / "corpus.lock.toml"
CORPUS_DIR = EVAL_DIR / "corpus"
ABLATION_RESULTS_DIR = EVAL_DIR / "ablation_results"
ABLATION_REPORT_PATH = EVAL_DIR / "ABLATION_REPORT.md"

# ──────────────────────────────────────────────────────────────────────────────
# Ground Truth Files
# ──────────────────────────────────────────────────────────────────────────────

SET_A_PATH = GROUND_TRUTH_DIR / "set_a_issues.jsonl"
SET_B_PATH = GROUND_TRUTH_DIR / "set_b_why.jsonl"
REVERT_PAIRS_PATH = GROUND_TRUTH_DIR / "revert_pairs.jsonl"

# ──────────────────────────────────────────────────────────────────────────────
# Document Representation IDs
# ──────────────────────────────────────────────────────────────────────────────

# D1: Commit message only (sanitized)
# D2: Raw unified diff text
# D3: Message + raw diff concatenation
# D4: Templated pseudo-document (structured: message + file changes + sigs + ids + docs)
# D5: Templated without message (template body only)
REPRESENTATION_IDS = ["D1", "D2", "D3", "D4", "D5"]

# ──────────────────────────────────────────────────────────────────────────────
# Retriever IDs
# ──────────────────────────────────────────────────────────────────────────────

# R1: Keyword (git log -S simulation)
# R2: BM25
# R3: MiniLM dense
# R4: Code-aware dense (UniXcoder / CodeBERT)
# R5: RRF Hybrid (R2 + R3)
RETRIEVER_IDS = ["R1", "R2", "R3", "R4", "R5"]

# ──────────────────────────────────────────────────────────────────────────────
# Evaluation Hyperparameters
# ──────────────────────────────────────────────────────────────────────────────

# Recall@K cutoffs
RECALL_K_VALUES = [5, 10, 20]

# Bootstrap CI
BOOTSTRAP_ITERATIONS = 10_000
BOOTSTRAP_ALPHA = 0.05  # 95% CI

# RRF fusion parameter
RRF_K = 60

# Decision gate threshold: ΔRecall@10 >= 0.05 with CI excluding 0
GATE_DELTA_THRESHOLD = 0.05

# ──────────────────────────────────────────────────────────────────────────────
# Model Configuration
# ──────────────────────────────────────────────────────────────────────────────

# MiniLM for general-purpose dense retrieval (R3)
MINILM_MODEL_NAME = "sentence-transformers/all-MiniLM-L6-v2"

# Code-aware dense retrieval model (R4)
CODE_MODEL_NAME = "microsoft/unixcoder-base"

# Maximum document length for embedding (in tokens, approximate)
MAX_DOC_TOKENS = 512

# ──────────────────────────────────────────────────────────────────────────────
# Operational
# ──────────────────────────────────────────────────────────────────────────────

# Maximum number of commits to index per corpus repo for the ablation
MAX_COMMITS_PER_REPO = int(os.environ.get("GITCTX_ABLATION_MAX_COMMITS", "2000"))

# Batch size for embedding computation
EMBEDDING_BATCH_SIZE = 32

# Random seed for reproducibility
RANDOM_SEED = 42

# Dev/test split for Set B queries
SET_B_DEV_SPLIT = "dev"
SET_B_TEST_SPLIT = "test"

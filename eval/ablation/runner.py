"""Ablation grid runner.

Orchestrates the full D×R ablation study:
1. Loads ground truth datasets (Set A, Set B)
2. Generates synthetic commit documents (or loads from corpus)
3. For each (D, R) combination, indexes and evaluates
4. Collects per-query results for bootstrap analysis
5. Produces JSON output for the report generator
"""

from __future__ import annotations

import json
import logging
import time
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

import numpy as np

from eval.ablation.config import (
    ABLATION_RESULTS_DIR,
    RANDOM_SEED,
    RECALL_K_VALUES,
    REPRESENTATION_IDS,
    RETRIEVER_IDS,
    SET_A_PATH,
    SET_B_PATH,
)
from eval.ablation.metrics import (
    QueryResult,
    aggregate_metrics,
    compute_query_metrics,
)
from eval.ablation.representations import GENERATORS, generate_document
from eval.ablation.retrievers import create_retriever, get_available_retrievers

logger = logging.getLogger(__name__)


@dataclass
class AblationResult:
    """Results from a single (D, R) evaluation cell."""

    representation: str
    retriever: str
    metrics: Dict[str, float]
    per_query: List[Dict[str, Any]]
    elapsed_seconds: float


@dataclass
class GridResult:
    """Complete ablation grid results."""

    cells: List[AblationResult]
    representations_used: List[str]
    retrievers_used: List[str]
    total_queries: int
    total_documents: int
    timestamp: str
    random_seed: int = RANDOM_SEED


def load_ground_truth_queries(
    set_a_path: Optional[Path] = None,
    set_b_path: Optional[Path] = None,
    split: Optional[str] = None,
) -> List[Dict[str, Any]]:
    """Load ground truth queries from JSONL files.

    Args:
        set_a_path: Path to Set A issues JSONL.
        set_b_path: Path to Set B why-queries JSONL.
        split: If given, filter Set B by 'dev' or 'test' split.

    Returns:
        List of query dicts with keys:
            - qid: query ID
            - query: query text
            - gold_commits: set of relevant commit hashes
            - repo: repository identifier
            - query_set: 'A' or 'B'
    """
    queries = []

    # Load Set A
    sa_path = set_a_path or SET_A_PATH
    if sa_path.exists():
        with open(sa_path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                entry = json.loads(line)
                gold = set()
                for label in entry.get("labels", []):
                    gold.add(label["commit"])
                queries.append({
                    "qid": entry["qid"],
                    "query": entry.get("query", entry.get("query_title", "")),
                    "gold_commits": gold,
                    "repo": entry.get("repo", ""),
                    "query_set": "A",
                })

    # Load Set B
    sb_path = set_b_path or SET_B_PATH
    if sb_path.exists():
        with open(sb_path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                entry = json.loads(line)
                if split and entry.get("split") != split:
                    continue
                gold = set()
                for gc in entry.get("gold_commits", []):
                    gold.add(gc["commit"])
                queries.append({
                    "qid": entry["qid"],
                    "query": entry.get("query", ""),
                    "gold_commits": gold,
                    "repo": entry.get("repo", ""),
                    "query_set": "B",
                })

    return queries


def generate_synthetic_documents(
    num_docs: int = 100,
    seed: int = RANDOM_SEED,
) -> List[Dict[str, Any]]:
    """Generate synthetic commit documents for testing when corpus is unavailable.

    Creates plausible commit-like documents with messages, diffs, and authors.
    Used for development testing and CI.

    Args:
        num_docs: Number of synthetic documents to generate.
        seed: Random seed for reproducibility.

    Returns:
        List of commit dicts with keys: hash, message, diff, author, repo.
    """
    rng = np.random.RandomState(seed)

    function_names = [
        "process_request", "validate_input", "handle_error", "parse_response",
        "send_notification", "update_cache", "check_permission", "transform_data",
        "build_query", "render_template", "authenticate_user", "serialize_output",
        "compress_payload", "decode_message", "route_handler", "schedule_task",
        "allocate_buffer", "release_connection", "merge_configs", "filter_results",
    ]

    file_paths = [
        "src/api/handlers.py", "src/core/engine.rs", "lib/utils.ts",
        "pkg/server/router.go", "src/auth/session.py", "src/db/queries.rs",
        "lib/parser.ts", "pkg/cache/store.go", "src/api/middleware.py",
        "src/models/user.rs",
    ]

    change_verbs = [
        "Fix", "Refactor", "Add", "Update", "Improve", "Optimize",
        "Restructure", "Implement", "Remove", "Simplify",
    ]

    docs = []
    for i in range(num_docs):
        commit_hash = "%08x%08x%08x%08x%08x" % tuple(
            int(rng.randint(0, 2**31)) for _ in range(5)
        )
        func = rng.choice(function_names)
        fpath = rng.choice(file_paths)
        verb = rng.choice(change_verbs)

        message = f"{verb} {func} in {fpath.split('/')[-1]}"
        diff = (
            f"diff --git a/{fpath} b/{fpath}\n"
            f"--- a/{fpath}\n"
            f"+++ b/{fpath}\n"
            f"@@ -10,5 +10,8 @@\n"
            f" existing_code()\n"
            f"-def {func}(old_param):\n"
            f"-    return old_logic()\n"
            f"+def {func}(new_param, extra=None):\n"
            f"+    '''Updated {func} with better error handling.'''\n"
            f"+    result = new_logic(new_param)\n"
            f"+    return result\n"
        )

        docs.append({
            "hash": commit_hash,
            "message": message,
            "diff": diff,
            "author": f"dev{rng.randint(1, 10)}",
            "repo": "synthetic/test",
        })

    return docs


def generate_synthetic_queries(
    documents: List[Dict[str, Any]],
    num_queries: int = 20,
    seed: int = RANDOM_SEED,
) -> List[Dict[str, Any]]:
    """Generate synthetic queries aligned with synthetic documents.

    Creates queries that reference specific functions or changes from the
    synthetic documents, with gold_commits pointing to the relevant docs.

    Args:
        documents: List of synthetic commit dicts.
        num_queries: Number of queries to generate.
        seed: Random seed.

    Returns:
        List of query dicts with qid, query, gold_commits, repo, query_set.
    """
    rng = np.random.RandomState(seed)
    queries = []

    query_templates = [
        "Why was {} changed?",
        "What happened to {} recently?",
        "Find changes to {} function",
        "Why did we refactor {}?",
        "Show the evolution of {}",
    ]

    for i in range(min(num_queries, len(documents))):
        doc = documents[i]
        # Extract function name from diff
        func_match = None
        for line in doc["diff"].split("\n"):
            if line.startswith("+def ") or line.startswith("+fn ") or line.startswith("+func "):
                parts = line.split("(")[0].split()
                if len(parts) >= 2:
                    func_match = parts[1]
                    break

        if not func_match:
            func_match = f"function_{i}"

        template = rng.choice(query_templates)
        query_text = template.format(func_match)

        queries.append({
            "qid": f"syn-{i:03d}",
            "query": query_text,
            "gold_commits": {doc["hash"]},
            "repo": doc.get("repo", "synthetic/test"),
            "query_set": "synthetic",
        })

    return queries


def run_cell(
    representation: str,
    retriever_id: str,
    documents: List[Dict[str, Any]],
    queries: List[Dict[str, Any]],
    repo: str = "synthetic/test",
) -> AblationResult:
    """Run a single (D, R) ablation cell.

    Args:
        representation: Document representation ID (D1-D5).
        retriever_id: Retriever ID (R1-R5).
        documents: List of commit dicts.
        queries: List of query dicts with gold_commits.
        repo: Repository identifier.

    Returns:
        AblationResult with metrics and per-query breakdown.
    """
    start = time.time()

    # Generate documents under this representation
    gen = GENERATORS[representation]
    doc_ids = []
    doc_texts = []
    for doc in documents:
        text = gen(doc)
        doc_id = f"{repo}:{doc['hash']}"
        doc_ids.append(doc_id)
        doc_texts.append(text)

    # Create and index retriever
    retriever = create_retriever(retriever_id)
    if not retriever.is_available():
        elapsed = time.time() - start
        return AblationResult(
            representation=representation,
            retriever=retriever_id,
            metrics={"recall@5": 0, "recall@10": 0, "recall@20": 0, "mrr": 0, "skipped": 1},
            per_query=[],
            elapsed_seconds=elapsed,
        )

    retriever.index(doc_ids, doc_texts)

    # Evaluate each query
    per_query_results = []
    for q in queries:
        retrieved = retriever.search(q["query"], k=20)
        # Map gold commits to doc_ids
        gold_doc_ids = {f"{repo}:{c}" for c in q["gold_commits"]}
        qr = compute_query_metrics(retrieved, gold_doc_ids, q["qid"])
        per_query_results.append(qr)

    # Aggregate
    metrics = aggregate_metrics(per_query_results)
    elapsed = time.time() - start

    per_query_dicts = []
    for qr in per_query_results:
        per_query_dicts.append({
            "qid": qr.query_id,
            "recall@5": qr.recall_at_5,
            "recall@10": qr.recall_at_10,
            "recall@20": qr.recall_at_20,
            "mrr": qr.mrr,
            "retrieved_count": len(qr.retrieved_ids),
        })

    return AblationResult(
        representation=representation,
        retriever=retriever_id,
        metrics=metrics,
        per_query=per_query_dicts,
        elapsed_seconds=elapsed,
    )


def run_grid(
    documents: Optional[List[Dict[str, Any]]] = None,
    queries: Optional[List[Dict[str, Any]]] = None,
    representations: Optional[List[str]] = None,
    retrievers: Optional[List[str]] = None,
    repo: str = "synthetic/test",
) -> GridResult:
    """Run the full D×R ablation grid.

    Args:
        documents: Commit documents. If None, generates synthetic data.
        queries: Evaluation queries. If None, generates synthetic queries.
        representations: List of representation IDs. Defaults to all (D1-D5).
        retrievers: List of retriever IDs. Defaults to available retrievers.
        repo: Repository identifier.

    Returns:
        GridResult with all cell results.
    """
    # Defaults
    if documents is None:
        documents = generate_synthetic_documents()
    if queries is None:
        queries = generate_synthetic_queries(documents)

    reps = representations or REPRESENTATION_IDS
    rets = retrievers or get_available_retrievers()

    logger.info(
        "Running ablation grid: %d representations × %d retrievers, "
        "%d documents, %d queries",
        len(reps), len(rets), len(documents), len(queries),
    )

    cells = []
    for rep in reps:
        for ret in rets:
            logger.info("Running cell %s × %s ...", rep, ret)
            cell = run_cell(rep, ret, documents, queries, repo)
            cells.append(cell)
            logger.info(
                "  %s × %s: Recall@10=%.4f, MRR=%.4f (%.1fs)",
                rep, ret,
                cell.metrics.get("recall@10", 0),
                cell.metrics.get("mrr", 0),
                cell.elapsed_seconds,
            )

    import datetime
    return GridResult(
        cells=cells,
        representations_used=reps,
        retrievers_used=rets,
        total_queries=len(queries),
        total_documents=len(documents),
        timestamp=datetime.datetime.now(datetime.timezone.utc).isoformat(),
    )


def save_grid_results(result: GridResult, output_dir: Optional[Path] = None) -> Path:
    """Save grid results as JSON.

    Args:
        result: GridResult to save.
        output_dir: Output directory. Defaults to eval/ablation_results/.

    Returns:
        Path to the saved JSON file.
    """
    out_dir = output_dir or ABLATION_RESULTS_DIR
    out_dir.mkdir(parents=True, exist_ok=True)

    # Serialize
    data = {
        "representations_used": result.representations_used,
        "retrievers_used": result.retrievers_used,
        "total_queries": result.total_queries,
        "total_documents": result.total_documents,
        "timestamp": result.timestamp,
        "random_seed": result.random_seed,
        "cells": [],
    }

    for cell in result.cells:
        data["cells"].append({
            "representation": cell.representation,
            "retriever": cell.retriever,
            "metrics": cell.metrics,
            "per_query": cell.per_query,
            "elapsed_seconds": cell.elapsed_seconds,
        })

    out_path = out_dir / "grid_results.json"
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2)

    logger.info("Saved grid results to %s", out_path)
    return out_path

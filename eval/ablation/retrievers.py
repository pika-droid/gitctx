"""Retriever implementations R1-R5.

All retrievers implement a common protocol:
    - index(documents: List[Document]) -> None
    - search(query: str, k: int) -> List[str]  # returns doc_ids

Retriever Strategies:
    R1: Keyword (git log -S simulation) — exact substring matching
    R2: BM25 — traditional keyword search via rank_bm25
    R3: MiniLM Dense — sentence-transformers/all-MiniLM-L6-v2 cosine similarity
    R4: Code-Aware Dense — microsoft/unixcoder-base embeddings
    R5: RRF Hybrid — Reciprocal Rank Fusion of R2 + R3 (k=60)
"""

from __future__ import annotations

import logging
import re
from abc import ABC, abstractmethod
from typing import Any, Dict, List, Optional, Sequence

import numpy as np

from eval.ablation.config import EMBEDDING_BATCH_SIZE, MAX_DOC_TOKENS, RRF_K

logger = logging.getLogger(__name__)

# ──────────────────────────────────────────────────────────────────────────────
# Base Protocol
# ──────────────────────────────────────────────────────────────────────────────


class Retriever(ABC):
    """Abstract base class for retriever implementations."""

    @property
    @abstractmethod
    def retriever_id(self) -> str:
        """Unique identifier for this retriever (R1-R5)."""

    @abstractmethod
    def index(self, doc_ids: List[str], texts: List[str]) -> None:
        """Index a collection of documents.

        Args:
            doc_ids: Unique document identifiers.
            texts: Document text content, aligned with doc_ids.
        """

    @abstractmethod
    def search(self, query: str, k: int = 10) -> List[str]:
        """Retrieve top-k document IDs for the given query.

        Args:
            query: Natural language query string.
            k: Number of results to return.

        Returns:
            List of doc_ids ranked by relevance (most relevant first).
        """

    @abstractmethod
    def is_available(self) -> bool:
        """Check if this retriever's dependencies are available."""


# ──────────────────────────────────────────────────────────────────────────────
# R1: Keyword Retriever (git log -S simulation)
# ──────────────────────────────────────────────────────────────────────────────


class KeywordRetriever(Retriever):
    """R1: Simulates git log -S exact substring matching.

    For each query, extracts significant terms and finds documents
    containing those exact substrings. Ranks by number of matching terms.
    """

    def __init__(self) -> None:
        self._doc_ids: List[str] = []
        self._texts: List[str] = []

    @property
    def retriever_id(self) -> str:
        return "R1"

    def index(self, doc_ids: List[str], texts: List[str]) -> None:
        self._doc_ids = list(doc_ids)
        self._texts = [t.lower() for t in texts]

    def search(self, query: str, k: int = 10) -> List[str]:
        if not self._doc_ids:
            return []
        # Extract significant terms (3+ chars, not stopwords)
        terms = _extract_search_terms(query)
        if not terms:
            return []

        scores: List[float] = []
        for text in self._texts:
            score = sum(1.0 for term in terms if term in text)
            scores.append(score)

        # Rank by score descending
        ranked = sorted(
            range(len(scores)),
            key=lambda i: scores[i],
            reverse=True,
        )
        results = []
        for idx in ranked[:k]:
            if scores[idx] > 0:
                results.append(self._doc_ids[idx])
        return results

    def is_available(self) -> bool:
        return True


def _extract_search_terms(query: str) -> List[str]:
    """Extract significant search terms from a query string."""
    stopwords = {
        "the", "a", "an", "is", "was", "were", "are", "be", "been", "being",
        "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "shall", "can", "need", "dare", "ought",
        "used", "to", "of", "in", "for", "on", "with", "at", "by", "from",
        "as", "into", "through", "during", "before", "after", "above",
        "below", "between", "out", "off", "over", "under", "again",
        "further", "then", "once", "here", "there", "when", "where", "why",
        "how", "all", "both", "each", "few", "more", "most", "other",
        "some", "such", "no", "nor", "not", "only", "own", "same", "so",
        "than", "too", "very", "just", "because", "but", "and", "or", "if",
        "while", "what", "which", "who", "whom", "this", "that", "these",
        "those", "it", "its",
    }
    tokens = re.findall(r"[a-zA-Z_][a-zA-Z0-9_]*", query.lower())
    return [t for t in tokens if len(t) >= 3 and t not in stopwords]


# ──────────────────────────────────────────────────────────────────────────────
# R2: BM25 Retriever
# ──────────────────────────────────────────────────────────────────────────────


class BM25Retriever(Retriever):
    """R2: BM25 keyword search using rank_bm25."""

    def __init__(self) -> None:
        self._doc_ids: List[str] = []
        self._bm25: Any = None

    @property
    def retriever_id(self) -> str:
        return "R2"

    def index(self, doc_ids: List[str], texts: List[str]) -> None:
        from rank_bm25 import BM25Okapi

        self._doc_ids = list(doc_ids)
        # Tokenize: split on whitespace and punctuation
        tokenized = [_tokenize_for_bm25(text) for text in texts]
        self._bm25 = BM25Okapi(tokenized)

    def search(self, query: str, k: int = 10) -> List[str]:
        if self._bm25 is None or not self._doc_ids:
            return []
        tokenized_query = _tokenize_for_bm25(query)
        scores = self._bm25.get_scores(tokenized_query)

        ranked = np.argsort(scores)[::-1][:k]
        results = []
        for idx in ranked:
            if scores[idx] > 0:
                results.append(self._doc_ids[idx])
        return results

    def is_available(self) -> bool:
        try:
            import rank_bm25  # noqa: F401
            return True
        except ImportError:
            return False


def _tokenize_for_bm25(text: str) -> List[str]:
    """Tokenize text for BM25: split on non-alphanumeric, split camelCase/snake_case, then lowercase."""
    # Extract tokens preserving case for camelCase splitting
    tokens = re.findall(r"[a-zA-Z_][a-zA-Z0-9_]*", text)
    # Also split camelCase and snake_case
    expanded = []
    for tok in tokens:
        # Split snake_case
        parts = tok.split("_")
        for part in parts:
            if part:
                # Split camelCase (must be done before lowercasing)
                subtokens = re.findall(r"[A-Z]?[a-z]+|[A-Z]+(?=[A-Z]|$)", part)
                if subtokens:
                    expanded.extend(s.lower() for s in subtokens)
                else:
                    expanded.append(part.lower())
    return expanded


# ──────────────────────────────────────────────────────────────────────────────
# R3: MiniLM Dense Retriever
# ──────────────────────────────────────────────────────────────────────────────


class DenseRetriever(Retriever):
    """R3: Dense retrieval using sentence-transformers MiniLM.

    Uses cosine similarity between query and document embeddings.
    Gracefully skips if sentence-transformers is not installed.
    """

    def __init__(self, model_name: str = "", retriever_id_str: str = "R3") -> None:
        from eval.ablation.config import MINILM_MODEL_NAME
        self._model_name = model_name or MINILM_MODEL_NAME
        self._retriever_id = retriever_id_str
        self._doc_ids: List[str] = []
        self._doc_embeddings: Optional[np.ndarray] = None
        self._model: Any = None

    @property
    def retriever_id(self) -> str:
        return self._retriever_id

    def _load_model(self) -> None:
        if self._model is not None:
            return
        # Security: only load from HuggingFace Hub with trust_remote_code=False
        from sentence_transformers import SentenceTransformer
        self._model = SentenceTransformer(
            self._model_name,
            trust_remote_code=False,
        )

    def index(self, doc_ids: List[str], texts: List[str]) -> None:
        self._load_model()
        self._doc_ids = list(doc_ids)
        # Truncate texts to MAX_DOC_TOKENS approximate characters
        truncated = [t[:MAX_DOC_TOKENS * 4] for t in texts]
        self._doc_embeddings = self._model.encode(
            truncated,
            batch_size=EMBEDDING_BATCH_SIZE,
            show_progress_bar=False,
            normalize_embeddings=True,
        )

    def search(self, query: str, k: int = 10) -> List[str]:
        if self._model is None or self._doc_embeddings is None:
            return []
        query_emb = self._model.encode(
            [query],
            normalize_embeddings=True,
        )
        # Cosine similarity (embeddings are normalized, so dot product = cosine)
        scores = np.dot(self._doc_embeddings, query_emb.T).flatten()

        ranked = np.argsort(scores)[::-1][:k]
        return [self._doc_ids[idx] for idx in ranked if scores[idx] > 0]

    def is_available(self) -> bool:
        try:
            import sentence_transformers  # noqa: F401
            return True
        except ImportError:
            return False


# ──────────────────────────────────────────────────────────────────────────────
# R4: Code-Aware Dense Retriever
# ──────────────────────────────────────────────────────────────────────────────


class CodeDenseRetriever(DenseRetriever):
    """R4: Dense retrieval using a code-aware embedding model.

    Uses a code-specific model (UniXcoder or CodeBERT) for better
    understanding of code identifiers and programming concepts.
    """

    def __init__(self) -> None:
        from eval.ablation.config import CODE_MODEL_NAME
        super().__init__(model_name=CODE_MODEL_NAME, retriever_id_str="R4")


# ──────────────────────────────────────────────────────────────────────────────
# R5: RRF Hybrid Retriever
# ──────────────────────────────────────────────────────────────────────────────


class RRFHybridRetriever(Retriever):
    """R5: Reciprocal Rank Fusion of BM25 (R2) + Dense (R3).

    Combines sparse and dense retrieval via RRF formula:
        RRF_score(d) = sum(1 / (k + rank_i(d))) for each system i

    Falls back to BM25-only if dense retriever is unavailable.
    """

    def __init__(self, rrf_k: int = RRF_K) -> None:
        self._bm25 = BM25Retriever()
        self._dense = DenseRetriever()
        self._rrf_k = rrf_k
        self._dense_available = False

    @property
    def retriever_id(self) -> str:
        return "R5"

    def index(self, doc_ids: List[str], texts: List[str]) -> None:
        self._bm25.index(doc_ids, texts)
        self._dense_available = self._dense.is_available()
        if self._dense_available:
            try:
                self._dense.index(doc_ids, texts)
            except Exception as e:
                logger.warning("Dense retriever failed to index, falling back to BM25-only: %s", e)
                self._dense_available = False

    def search(self, query: str, k: int = 10) -> List[str]:
        # Get BM25 results (request more to allow fusion)
        fetch_k = min(k * 3, 100)
        bm25_results = self._bm25.search(query, k=fetch_k)

        if not self._dense_available:
            return bm25_results[:k]

        dense_results = self._dense.search(query, k=fetch_k)

        # Compute RRF scores
        rrf_scores: Dict[str, float] = {}

        for rank, doc_id in enumerate(bm25_results):
            rrf_scores[doc_id] = rrf_scores.get(doc_id, 0.0) + 1.0 / (self._rrf_k + rank + 1)

        for rank, doc_id in enumerate(dense_results):
            rrf_scores[doc_id] = rrf_scores.get(doc_id, 0.0) + 1.0 / (self._rrf_k + rank + 1)

        # Sort by RRF score descending
        sorted_docs = sorted(rrf_scores.items(), key=lambda x: x[1], reverse=True)
        return [doc_id for doc_id, _ in sorted_docs[:k]]

    def is_available(self) -> bool:
        return self._bm25.is_available()


# ──────────────────────────────────────────────────────────────────────────────
# Registry
# ──────────────────────────────────────────────────────────────────────────────

RETRIEVER_CLASSES = {
    "R1": KeywordRetriever,
    "R2": BM25Retriever,
    "R3": DenseRetriever,
    "R4": CodeDenseRetriever,
    "R5": RRFHybridRetriever,
}


def create_retriever(retriever_id: str) -> Retriever:
    """Create a retriever instance by ID.

    Args:
        retriever_id: One of 'R1'-'R5'.

    Returns:
        Retriever instance.

    Raises:
        ValueError: If retriever_id is not recognized.
    """
    if retriever_id not in RETRIEVER_CLASSES:
        raise ValueError(
            f"Unknown retriever '{retriever_id}'. "
            f"Expected one of: {sorted(RETRIEVER_CLASSES.keys())}"
        )
    return RETRIEVER_CLASSES[retriever_id]()


def get_available_retrievers() -> List[str]:
    """Return list of retriever IDs whose dependencies are satisfied."""
    available = []
    for rid, cls in RETRIEVER_CLASSES.items():
        try:
            instance = cls()
            if instance.is_available():
                available.append(rid)
        except Exception:
            pass
    return available

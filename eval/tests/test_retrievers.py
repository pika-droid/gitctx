"""Tests for retriever implementations R1-R5."""

from __future__ import annotations

import pytest

from eval.ablation.retrievers import (
    BM25Retriever,
    KeywordRetriever,
    RRFHybridRetriever,
    create_retriever,
    get_available_retrievers,
    _extract_search_terms,
    _tokenize_for_bm25,
)


# ──────────────────────────────────────────────────────────────────────────────
# Fixtures
# ──────────────────────────────────────────────────────────────────────────────


@pytest.fixture
def sample_docs():
    """Small document set for testing retrievers."""
    return {
        "doc_ids": [
            "repo:aaa111",
            "repo:bbb222",
            "repo:ccc333",
            "repo:ddd444",
            "repo:eee555",
        ],
        "texts": [
            "Fix timeout in HTTPAdapter socket handling for keep-alive connections",
            "Refactor authentication middleware to support OAuth2 token refresh",
            "Add connection pool allocator with configurable max connections",
            "Update JSON serialization to handle nested datetime objects properly",
            "Fix race condition in cache invalidation during concurrent requests",
        ],
    }


# ──────────────────────────────────────────────────────────────────────────────
# R1: Keyword Retriever
# ──────────────────────────────────────────────────────────────────────────────


class TestKeywordRetriever:
    def test_basic_search(self, sample_docs):
        r = KeywordRetriever()
        r.index(sample_docs["doc_ids"], sample_docs["texts"])
        results = r.search("timeout socket HTTPAdapter", k=3)
        assert len(results) > 0
        assert results[0] == "repo:aaa111"

    def test_no_match(self, sample_docs):
        r = KeywordRetriever()
        r.index(sample_docs["doc_ids"], sample_docs["texts"])
        results = r.search("quantum entanglement", k=3)
        assert len(results) == 0

    def test_empty_index(self):
        r = KeywordRetriever()
        r.index([], [])
        results = r.search("anything", k=5)
        assert results == []

    def test_retriever_id(self):
        r = KeywordRetriever()
        assert r.retriever_id == "R1"

    def test_is_available(self):
        r = KeywordRetriever()
        assert r.is_available() is True

    def test_case_insensitive(self, sample_docs):
        r = KeywordRetriever()
        r.index(sample_docs["doc_ids"], sample_docs["texts"])
        results_lower = r.search("timeout", k=5)
        results_upper = r.search("TIMEOUT", k=5)
        assert results_lower == results_upper


# ──────────────────────────────────────────────────────────────────────────────
# R2: BM25 Retriever
# ──────────────────────────────────────────────────────────────────────────────


class TestBM25Retriever:
    def test_basic_search(self, sample_docs):
        r = BM25Retriever()
        if not r.is_available():
            pytest.skip("rank_bm25 not installed")
        r.index(sample_docs["doc_ids"], sample_docs["texts"])
        results = r.search("authentication OAuth token", k=3)
        assert len(results) > 0
        # The auth doc should be ranked highly
        assert "repo:bbb222" in results[:2]

    def test_retriever_id(self):
        r = BM25Retriever()
        assert r.retriever_id == "R2"

    def test_empty_query(self, sample_docs):
        r = BM25Retriever()
        if not r.is_available():
            pytest.skip("rank_bm25 not installed")
        r.index(sample_docs["doc_ids"], sample_docs["texts"])
        # Empty query should return empty (no matching tokens)
        results = r.search("", k=5)
        assert isinstance(results, list)


# ──────────────────────────────────────────────────────────────────────────────
# R5: RRF Hybrid Retriever
# ──────────────────────────────────────────────────────────────────────────────


class TestRRFHybridRetriever:
    def test_basic_search_bm25_fallback(self, sample_docs):
        """RRF should fall back to BM25-only when dense is unavailable."""
        r = RRFHybridRetriever()
        if not r.is_available():
            pytest.skip("rank_bm25 not installed")
        r.index(sample_docs["doc_ids"], sample_docs["texts"])
        results = r.search("connection pool allocator", k=3)
        assert len(results) > 0

    def test_retriever_id(self):
        r = RRFHybridRetriever()
        assert r.retriever_id == "R5"


# ──────────────────────────────────────────────────────────────────────────────
# Helper Functions
# ──────────────────────────────────────────────────────────────────────────────


class TestHelpers:
    def test_extract_search_terms(self):
        terms = _extract_search_terms("Why was the HTTPAdapter timeout changed?")
        assert "httpadapter" in terms
        assert "timeout" in terms
        assert "changed" in terms
        # Stopwords removed
        assert "was" not in terms
        assert "the" not in terms

    def test_tokenize_for_bm25_snake_case(self):
        tokens = _tokenize_for_bm25("process_request_handler")
        assert "process" in tokens
        assert "request" in tokens
        assert "handler" in tokens

    def test_tokenize_for_bm25_camel_case(self):
        tokens = _tokenize_for_bm25("processRequestHandler")
        assert "process" in tokens
        assert "request" in tokens
        assert "handler" in tokens


# ──────────────────────────────────────────────────────────────────────────────
# Registry
# ──────────────────────────────────────────────────────────────────────────────


class TestRegistry:
    def test_create_known_retriever(self):
        r = create_retriever("R1")
        assert r.retriever_id == "R1"

    def test_create_invalid_retriever(self):
        with pytest.raises(ValueError, match="Unknown retriever"):
            create_retriever("R99")

    def test_available_retrievers(self):
        available = get_available_retrievers()
        # R1 (keyword) should always be available
        assert "R1" in available

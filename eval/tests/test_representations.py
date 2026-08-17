"""Tests for document representations D1-D5."""

from __future__ import annotations

import pytest

from eval.ablation.representations import (
    Document,
    generate_d1,
    generate_d2,
    generate_d3,
    generate_d4,
    generate_d5,
    generate_document,
    GENERATORS,
    _extract_identifiers_from_diff,
    _extract_function_sigs_from_diff,
    _extract_changed_files,
)


# ──────────────────────────────────────────────────────────────────────────────
# Fixtures
# ──────────────────────────────────────────────────────────────────────────────


@pytest.fixture
def sample_commit():
    """A realistic commit dict with message, diff, author, and hash."""
    return {
        "hash": "abc123def456789",
        "author": "alice",
        "message": "Fix timeout in HTTPAdapter when keep-alive drops (#1234)\n\nFixes #1234\nSigned-off-by: alice <alice@example.com>",
        "diff": (
            "diff --git a/src/adapters.py b/src/adapters.py\n"
            "--- a/src/adapters.py\n"
            "+++ b/src/adapters.py\n"
            "@@ -42,6 +42,10 @@\n"
            " class HTTPAdapter:\n"
            "+def send_with_timeout(self, request, timeout=30):\n"
            "+    '''Send request with configurable timeout handling.'''\n"
            "+    socket_timeout = self._calculate_timeout(timeout)\n"
            "+    return self._send(request, socket_timeout)\n"
        ),
    }


@pytest.fixture
def empty_commit():
    """A commit with no message or diff."""
    return {
        "hash": "0000000000000",
        "author": "",
        "message": "",
        "diff": "",
    }


@pytest.fixture
def rust_commit():
    """A Rust-style commit."""
    return {
        "hash": "rust789abc",
        "author": "bob",
        "message": "Refactor connection pool allocator",
        "diff": (
            "diff --git a/src/pool.rs b/src/pool.rs\n"
            "--- a/src/pool.rs\n"
            "+++ b/src/pool.rs\n"
            "@@ -10,3 +10,6 @@\n"
            "+pub fn allocate_connection(config: &PoolConfig) -> Result<Connection> {\n"
            "+    /// Allocates a new connection from the pool.\n"
            "+    let conn = pool.get_or_create(config)?;\n"
        ),
    }


# ──────────────────────────────────────────────────────────────────────────────
# D1: Message Only
# ──────────────────────────────────────────────────────────────────────────────


class TestD1:
    def test_basic_message(self, sample_commit):
        text = generate_d1(sample_commit)
        assert "timeout" in text.lower()
        assert "HTTPAdapter" in text or "httpadapter" in text.lower()

    def test_sanitization_strips_issue_refs(self, sample_commit):
        text = generate_d1(sample_commit)
        # Issue number #1234 should be stripped
        assert "#1234" not in text
        # Fixes trailer should be stripped
        assert "Fixes" not in text

    def test_sanitization_strips_signoff(self, sample_commit):
        text = generate_d1(sample_commit)
        assert "Signed-off-by" not in text

    def test_empty_message(self, empty_commit):
        text = generate_d1(empty_commit)
        assert text == ""


# ──────────────────────────────────────────────────────────────────────────────
# D2: Raw Diff
# ──────────────────────────────────────────────────────────────────────────────


class TestD2:
    def test_contains_diff(self, sample_commit):
        text = generate_d2(sample_commit)
        assert "diff --git" in text
        assert "+def send_with_timeout" in text

    def test_empty_diff(self, empty_commit):
        text = generate_d2(empty_commit)
        assert text == ""


# ──────────────────────────────────────────────────────────────────────────────
# D3: Message + Diff
# ──────────────────────────────────────────────────────────────────────────────


class TestD3:
    def test_contains_both(self, sample_commit):
        text = generate_d3(sample_commit)
        # Should contain sanitized message
        assert "timeout" in text.lower()
        # Should contain diff
        assert "diff --git" in text

    def test_empty_gives_empty(self, empty_commit):
        text = generate_d3(empty_commit)
        assert text == ""


# ──────────────────────────────────────────────────────────────────────────────
# D4: Templated Pseudo-Document
# ──────────────────────────────────────────────────────────────────────────────


class TestD4:
    def test_structured_output(self, sample_commit):
        text = generate_d4(sample_commit)
        assert "Commit abc123def456" in text
        assert "alice" in text
        assert "Files changed:" in text
        assert "adapters.py" in text

    def test_function_sigs_extracted(self, sample_commit):
        text = generate_d4(sample_commit)
        assert "Functions modified:" in text
        assert "send_with_timeout" in text

    def test_identifiers_extracted(self, sample_commit):
        text = generate_d4(sample_commit)
        assert "Key identifiers:" in text
        assert "socket_timeout" in text

    def test_message_present(self, sample_commit):
        text = generate_d4(sample_commit)
        assert "Change description:" in text

    def test_empty_commit(self, empty_commit):
        text = generate_d4(empty_commit)
        assert "Commit" in text  # Should still have header


# ──────────────────────────────────────────────────────────────────────────────
# D5: Templated Without Message
# ──────────────────────────────────────────────────────────────────────────────


class TestD5:
    def test_no_message_section(self, sample_commit):
        text = generate_d5(sample_commit)
        assert "Change description:" not in text

    def test_has_structural_content(self, sample_commit):
        text = generate_d5(sample_commit)
        assert "Files changed:" in text
        assert "Functions modified:" in text

    def test_differs_from_d4(self, sample_commit):
        d4 = generate_d4(sample_commit)
        d5 = generate_d5(sample_commit)
        assert d4 != d5
        assert len(d4) > len(d5)


# ──────────────────────────────────────────────────────────────────────────────
# Helper Functions
# ──────────────────────────────────────────────────────────────────────────────


class TestExtractors:
    def test_extract_identifiers(self):
        diff = "+    result = process_data(input_buffer)\n+    cache_key = compute_hash(result)"
        ids = _extract_identifiers_from_diff(diff)
        assert "process_data" in ids
        assert "input_buffer" in ids
        assert "cache_key" in ids

    def test_extract_python_sigs(self):
        diff = "+def handle_request(self, req, timeout=30):\n+    pass"
        sigs = _extract_function_sigs_from_diff(diff)
        assert any("handle_request" in s for s in sigs)

    def test_extract_rust_sigs(self, rust_commit):
        sigs = _extract_function_sigs_from_diff(rust_commit["diff"])
        assert any("allocate_connection" in s for s in sigs)

    def test_extract_changed_files(self, sample_commit):
        files = _extract_changed_files(sample_commit["diff"])
        assert "src/adapters.py" in files

    def test_empty_diff_extractors(self):
        assert _extract_identifiers_from_diff("") == []
        assert _extract_function_sigs_from_diff("") == []
        assert _extract_changed_files("") == []


# ──────────────────────────────────────────────────────────────────────────────
# Generator Registry
# ──────────────────────────────────────────────────────────────────────────────


class TestGeneratorRegistry:
    def test_all_representations_registered(self):
        assert set(GENERATORS.keys()) == {"D1", "D2", "D3", "D4", "D5"}

    def test_generate_document_factory(self, sample_commit):
        doc = generate_document(sample_commit, "D4", "test/repo")
        assert isinstance(doc, Document)
        assert doc.doc_id == "test/repo:abc123def456789"
        assert doc.representation == "D4"
        assert len(doc.text) > 0

    def test_invalid_representation_raises(self, sample_commit):
        with pytest.raises(ValueError, match="Unknown representation"):
            generate_document(sample_commit, "D99", "test/repo")

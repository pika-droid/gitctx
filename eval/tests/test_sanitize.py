"""Tests for cross-language leakage sanitization engine (Python)."""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from eval.sanitize import sanitize_chunk_text


@pytest.fixture
def test_cases() -> list[dict]:
    fixture_path = Path(__file__).resolve().parent.parent.parent / "tests" / "fixtures" / "sanitization_cases.json"
    assert fixture_path.exists(), f"Missing fixture file: {fixture_path}"
    with open(fixture_path, "r", encoding="utf-8") as f:
        return json.load(f)


def test_fixture_cases_count(test_cases: list[dict]) -> None:
    """Ensure at least 20 comprehensive test cases are defined per D-09, D-10."""
    assert len(test_cases) >= 20


def test_all_fixture_cases(test_cases: list[dict]) -> None:
    """Assert all shared test vectors match expected outputs and strip counts."""
    for case in test_cases:
        case_id = case["id"]
        raw = case["raw"]
        expected = case["expected"]
        expected_strips = case["expected_strips"]

        result = sanitize_chunk_text(raw)

        assert result.sanitized_text == expected, (
            f"Case '{case_id}' failed!\n"
            f"Raw:      {raw!r}\n"
            f"Expected: {expected!r}\n"
            f"Actual:   {result.sanitized_text!r}"
        )
        assert result.strip_count == expected_strips, (
            f"Case '{case_id}' strip count mismatch!\n"
            f"Expected: {expected_strips}\n"
            f"Actual:   {result.strip_count}"
        )
        if expected_strips > 0:
            assert result.modified is True


def test_empty_string() -> None:
    result = sanitize_chunk_text("")
    assert result.sanitized_text == ""
    assert result.strip_count == 0
    assert result.modified is False


def test_clean_string() -> None:
    text = "Pure commit message without any trailers or references."
    result = sanitize_chunk_text(text)
    assert result.sanitized_text == text
    assert result.strip_count == 0
    assert result.modified is False

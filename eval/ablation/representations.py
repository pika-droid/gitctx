"""Document representation generators D1-D5.

Each generator takes a commit dict and produces a text document suitable
for indexing by a retriever. All generators apply leakage sanitization
via eval.sanitize.sanitize_chunk_text to prevent artificial recall inflation.

Document Representations:
    D1: Commit message only (sanitized)
    D2: Raw unified diff text
    D3: Message + raw diff concatenation
    D4: Templated pseudo-document (structured with file changes, sigs, identifiers)
    D5: Templated without message (template body only)
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Any, Dict, List, Optional

from eval.sanitize import sanitize_chunk_text


@dataclass(frozen=True)
class Document:
    """A single indexed document representing one commit under a specific representation."""

    doc_id: str
    commit_hash: str
    representation: str
    text: str
    repo: str


def _sanitize(text: str) -> str:
    """Apply leakage sanitization, returning cleaned text."""
    if not text:
        return ""
    return sanitize_chunk_text(text).sanitized_text


def _extract_identifiers_from_diff(diff_text: str) -> List[str]:
    """Extract plausible code identifiers from a unified diff.

    Looks for added/modified lines and extracts snake_case, camelCase,
    and PascalCase identifiers that look like function/variable names.
    """
    identifiers = set()
    if not diff_text:
        return []
    # Match lines starting with + (additions)
    for line in diff_text.split("\n"):
        if line.startswith("+") and not line.startswith("+++"):
            # Extract identifiers: word boundaries with underscores or camelCase
            tokens = re.findall(r"\b[a-zA-Z_][a-zA-Z0-9_]{2,}\b", line)
            for tok in tokens:
                # Skip common keywords
                if tok.lower() not in _SKIP_KEYWORDS:
                    identifiers.add(tok)
    return sorted(identifiers)


_SKIP_KEYWORDS = frozenset({
    "def", "class", "return", "import", "from", "if", "else", "elif",
    "for", "while", "try", "except", "finally", "with", "as", "pass",
    "break", "continue", "raise", "yield", "lambda", "and", "or", "not",
    "in", "is", "none", "true", "false", "self", "cls", "async", "await",
    "fn", "let", "mut", "pub", "use", "mod", "struct", "enum", "impl",
    "trait", "where", "match", "const", "static", "ref", "move", "type",
    "crate", "super", "extern", "unsafe", "dyn", "box",
    "function", "var", "const", "export", "default", "interface",
    "string", "int", "float", "bool", "list", "dict", "tuple", "set",
    "the", "this", "that", "these", "those", "are", "was", "were",
    "has", "have", "had", "will", "would", "could", "should", "can",
})


def _extract_function_sigs_from_diff(diff_text: str) -> List[str]:
    """Extract function/method signature lines from a unified diff."""
    sigs = []
    if not diff_text:
        return sigs
    # Patterns for common language function definitions
    patterns = [
        re.compile(r"^\+\s*(def\s+\w+\s*\(.*?\))", re.MULTILINE),
        re.compile(r"^\+\s*(fn\s+\w+\s*(?:<.*?>)?\s*\(.*?\))", re.MULTILINE),
        re.compile(r"^\+\s*((?:pub\s+)?(?:async\s+)?fn\s+\w+)", re.MULTILINE),
        re.compile(r"^\+\s*(func\s+(?:\(\w+\s+\*?\w+\)\s+)?\w+\s*\(.*?\))", re.MULTILINE),
        re.compile(r"^\+\s*((?:export\s+)?(?:async\s+)?function\s+\w+\s*\(.*?\))", re.MULTILINE),
    ]
    for pat in patterns:
        sigs.extend(pat.findall(diff_text))
    return sigs


def _extract_docstrings_from_diff(diff_text: str) -> List[str]:
    """Extract docstring-like content from diff additions."""
    docs = []
    if not diff_text:
        return docs
    # Python triple-quote docstrings
    py_docs = re.findall(
        r'^\+\s*"""(.*?)"""',
        diff_text,
        re.MULTILINE | re.DOTALL,
    )
    docs.extend(d.strip() for d in py_docs if d.strip())
    # Rust /// doc comments
    rust_docs = re.findall(r"^\+\s*///\s*(.*)", diff_text, re.MULTILINE)
    docs.extend(d.strip() for d in rust_docs if d.strip())
    # JSDoc /** ... */
    jsdoc = re.findall(
        r"^\+\s*/\*\*(.*?)\*/",
        diff_text,
        re.MULTILINE | re.DOTALL,
    )
    docs.extend(d.strip() for d in jsdoc if d.strip())
    return docs


def _extract_changed_files(diff_text: str) -> List[str]:
    """Extract file paths mentioned in diff headers."""
    if not diff_text:
        return []
    files = re.findall(r"^(?:\+\+\+|---)\s+[ab]/(.*)", diff_text, re.MULTILINE)
    return sorted(set(files))


# ──────────────────────────────────────────────────────────────────────────────
# Generator Functions
# ──────────────────────────────────────────────────────────────────────────────


def generate_d1(commit: Dict[str, Any]) -> str:
    """D1: Commit message only (sanitized).

    Uses only the commit message, stripped of issue references and trailers.
    This is the weakest baseline — tests whether the message alone carries
    enough semantic signal for retrieval.
    """
    message = commit.get("message", "")
    return _sanitize(message)


def generate_d2(commit: Dict[str, Any]) -> str:
    """D2: Raw unified diff text.

    The full unified diff output. Preserves all diff syntax (+/-/@@).
    Tests whether raw code changes are retrievable without structural help.
    """
    diff = commit.get("diff", "")
    return diff


def generate_d3(commit: Dict[str, Any]) -> str:
    """D3: Message + raw diff concatenation.

    Simple concatenation of sanitized commit message and raw diff.
    Tests whether adding message context improves over diff-only.
    """
    message = _sanitize(commit.get("message", ""))
    diff = commit.get("diff", "")
    parts = []
    if message:
        parts.append(message)
    if diff:
        parts.append(diff)
    return "\n\n".join(parts)


def generate_d4(commit: Dict[str, Any]) -> str:
    """D4: Templated pseudo-document (full structured template).

    The core thesis representation. Structures the commit into natural-language
    sections covering:
    - Sanitized commit message
    - Changed files list
    - Function signatures (added/modified)
    - Extracted identifiers
    - Docstrings

    This bridges the cross-modal gap between NL queries and unified diff syntax.
    """
    message = _sanitize(commit.get("message", ""))
    diff = commit.get("diff", "")
    author = commit.get("author", "")
    commit_hash = commit.get("hash", "")[:12]

    changed_files = _extract_changed_files(diff)
    function_sigs = _extract_function_sigs_from_diff(diff)
    identifiers = _extract_identifiers_from_diff(diff)
    docstrings = _extract_docstrings_from_diff(diff)

    sections = []

    # Header
    sections.append(f"Commit {commit_hash} by {author}")

    # Message section
    if message:
        sections.append(f"Change description: {message}")

    # Files section
    if changed_files:
        file_list = ", ".join(changed_files)
        sections.append(f"Files changed: {file_list}")

    # Function signatures section
    if function_sigs:
        sig_text = "; ".join(function_sigs[:20])
        sections.append(f"Functions modified: {sig_text}")

    # Identifiers section
    if identifiers:
        id_text = ", ".join(identifiers[:40])
        sections.append(f"Key identifiers: {id_text}")

    # Docstrings section
    if docstrings:
        doc_text = " | ".join(d[:200] for d in docstrings[:5])
        sections.append(f"Documentation: {doc_text}")

    return "\n".join(sections)


def generate_d5(commit: Dict[str, Any]) -> str:
    """D5: Templated without message (template body only).

    Same as D4 but without the commit message. Tests whether the structured
    template alone (file changes, signatures, identifiers) carries retrieval
    signal without relying on message text.
    """
    diff = commit.get("diff", "")
    author = commit.get("author", "")
    commit_hash = commit.get("hash", "")[:12]

    changed_files = _extract_changed_files(diff)
    function_sigs = _extract_function_sigs_from_diff(diff)
    identifiers = _extract_identifiers_from_diff(diff)
    docstrings = _extract_docstrings_from_diff(diff)

    sections = []

    # Header (no message)
    sections.append(f"Commit {commit_hash} by {author}")

    # Files section
    if changed_files:
        file_list = ", ".join(changed_files)
        sections.append(f"Files changed: {file_list}")

    # Function signatures section
    if function_sigs:
        sig_text = "; ".join(function_sigs[:20])
        sections.append(f"Functions modified: {sig_text}")

    # Identifiers section
    if identifiers:
        id_text = ", ".join(identifiers[:40])
        sections.append(f"Key identifiers: {id_text}")

    # Docstrings section
    if docstrings:
        doc_text = " | ".join(d[:200] for d in docstrings[:5])
        sections.append(f"Documentation: {doc_text}")

    return "\n".join(sections)


# ──────────────────────────────────────────────────────────────────────────────
# Registry
# ──────────────────────────────────────────────────────────────────────────────

GENERATORS = {
    "D1": generate_d1,
    "D2": generate_d2,
    "D3": generate_d3,
    "D4": generate_d4,
    "D5": generate_d5,
}


def generate_document(
    commit: Dict[str, Any],
    representation: str,
    repo: str,
) -> Document:
    """Generate a Document for the given commit using the specified representation.

    Args:
        commit: Dict with keys 'hash', 'message', 'diff', 'author'.
        representation: One of 'D1'-'D5'.
        repo: Repository identifier.

    Returns:
        Document instance.

    Raises:
        ValueError: If representation is not recognized.
    """
    if representation not in GENERATORS:
        raise ValueError(
            f"Unknown representation '{representation}'. "
            f"Expected one of: {sorted(GENERATORS.keys())}"
        )
    gen = GENERATORS[representation]
    text = gen(commit)
    return Document(
        doc_id=f"{repo}:{commit['hash']}",
        commit_hash=commit["hash"],
        representation=representation,
        text=text,
        repo=repo,
    )

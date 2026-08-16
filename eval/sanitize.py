"""Cross-language leakage sanitization engine (Python implementation).

Strips issue numbers, action trailers, attribution lines, squash PR suffixes,
and merge headers from commit messages and diff chunks to prevent artificial
retrieval recall per D-09, D-10, and D-11.
"""

from __future__ import annotations

import dataclasses
import re
from typing import Tuple


@dataclasses.dataclass(frozen=True)
class SanitizedChunk:
    """Sanitized text chunk with modification metadata."""

    sanitized_text: str
    strip_count: int
    modified: bool


# Precompiled regexes for the 6-stage sanitization pipeline

# Stage 1: Merge commit header lines
_RE_MERGE_HEADER = re.compile(
    r"(?im)^[ \t]*Merge (?:pull request|branch|commit|remote-tracking branch)\b.*(?:\r?\n|$)",
)

# Stage 2: Action trailers (e.g. Fixes: #123, Closes #456, Resolves: gh-789)
_RE_ACTION_TRAILERS = re.compile(
    r"(?im)^[ \t]*(?:Fix(?:es|ed)?|Close[sd]?|Resolve[sd]?|Ref(?:erence[sd]?)?|See-also|Addresses|Related-to|Issues?|See)(?::[ \t]*.*|\s+(?:#|gh-)\d+.*)$(?:\r?\n|$)",
)

# Stage 3: Attribution trailers (e.g. Signed-off-by, Co-authored-by)
_RE_ATTRIBUTION_TRAILERS = re.compile(
    r"(?im)^[ \t]*(?:Signed-off-by|Co-authored-by|Reviewed-by|Acked-by|Suggested-by|Reported-by|Tested-by|Approved-by|Cc):[ \t]*.*$(?:\r?\n|$)",
)

# Stage 4: Squash PR suffixes at end of line (e.g. " (#1234)", " (gh-5678)")
_RE_SQUASH_SUFFIX = re.compile(
    r"(?m)[ \t]*\((?:#|gh-)\d+\)[ \t]*$",
)

# Stage 5: GitHub issue and PR URLs
_RE_GITHUB_URLS = re.compile(
    r"https?://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+/(?:issues|pull)/\d+",
)

# Stage 6: Inline issue references (e.g. #1234, gh-5678)
_RE_INLINE_ISSUE_NUMS = re.compile(
    r"(?:\bgh-|\B#)\d+\b",
)

# Whitespace normalization regexes
_RE_MULTI_SPACES = re.compile(r"(\S)[ \t]{2,}")
_RE_TRAILING_LINE_SPACE = re.compile(r"(?m)[ \t]+$")
_RE_MULTI_NEWLINES = re.compile(r"\n{3,}")


def sanitize_chunk_text(text: str) -> SanitizedChunk:
    """Sanitize raw commit / diff text by stripping issue references and trailers.

    Executes 6 deterministic regex stages:
    1. Merge header lines
    2. Action trailer lines
    3. Attribution trailer lines
    4. Squash PR suffixes
    5. GitHub issue & PR URLs
    6. Inline issue numbers (#123, gh-123)
    followed by whitespace normalization.

    Args:
        text: Raw text to sanitize.

    Returns:
        SanitizedChunk with sanitized text, strip count, and modified flag.
    """
    if not text:
        return SanitizedChunk(sanitized_text="", strip_count=0, modified=False)

    # Normalize CRLF first
    working_text = text.replace("\r\n", "\n")
    total_strips = 0

    # Stage 1: Merge header lines
    working_text, count = _RE_MERGE_HEADER.subn("", working_text)
    total_strips += count

    # Stage 2: Action trailers
    working_text, count = _RE_ACTION_TRAILERS.subn("", working_text)
    total_strips += count

    # Stage 3: Attribution trailers
    working_text, count = _RE_ATTRIBUTION_TRAILERS.subn("", working_text)
    total_strips += count

    # Stage 4: Squash PR suffixes
    working_text, count = _RE_SQUASH_SUFFIX.subn("", working_text)
    total_strips += count

    # Stage 5: GitHub issue and PR URLs
    working_text, count = _RE_GITHUB_URLS.subn("", working_text)
    total_strips += count

    # Stage 6: Inline issue numbers
    working_text, count = _RE_INLINE_ISSUE_NUMS.subn("", working_text)
    total_strips += count

    # Whitespace cleanup:
    # 1. Collapse multiple spaces within each line
    working_text = _RE_MULTI_SPACES.sub(r"\1 ", working_text)
    # 2. Strip trailing spaces from each line
    working_text = _RE_TRAILING_LINE_SPACE.sub("", working_text)
    # 3. Collapse 3+ newlines to 2 newlines
    working_text = _RE_MULTI_NEWLINES.sub("\n\n", working_text)
    # 4. Strip leading and trailing whitespace from the full text
    sanitized = working_text.strip()

    modified = total_strips > 0 or sanitized != text.strip()

    return SanitizedChunk(
        sanitized_text=sanitized,
        strip_count=total_strips,
        modified=modified,
    )

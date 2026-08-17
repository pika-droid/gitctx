//! Cross-language leakage sanitization engine (Rust implementation).
//!
//! Strips issue numbers, action trailers, attribution lines, squash PR suffixes,
//! and merge headers from commit messages and diff chunks to prevent artificial
//! retrieval recall per D-09, D-10, and D-11.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// Sanitized chunk containing cleaned text and modification metrics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SanitizedChunk {
    /// Sanitized text with issue references and trailers stripped.
    pub sanitized_text: String,
    /// Total number of stripped entities across all 6 stages.
    pub strip_count: usize,
    /// Whether the text was modified.
    pub modified: bool,
}

// Stage 1: Merge commit header lines
static RE_MERGE_HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?im)^[ \t]*Merge (?:pull request|branch|commit|remote-tracking branch)\b.*(?:\r?\n|$)",
    )
    .expect("valid regex")
});

// Stage 2: Action trailers (e.g. Fixes: #123, Closes #456, Resolves: gh-789)
static RE_ACTION_TRAILERS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?im)^[ \t]*(?:Fix(?:es|ed)?|Close[sd]?|Resolve[sd]?|Ref(?:erence[sd]?)?|See-also|Addresses|Related-to|Issues?|See)(?::[ \t]*.*|\s+(?:#|gh-)\d+.*)$(?:\r?\n|$)")
        .expect("valid regex")
});

// Stage 3: Attribution trailers (e.g. Signed-off-by, Co-authored-by)
static RE_ATTRIBUTION_TRAILERS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?im)^[ \t]*(?:Signed-off-by|Co-authored-by|Reviewed-by|Acked-by|Suggested-by|Reported-by|Tested-by|Approved-by|Cc):[ \t]*.*$(?:\r?\n|$)")
        .expect("valid regex")
});

// Stage 4: Squash PR suffixes at end of line (e.g. " (#1234)", " (gh-5678)")
static RE_SQUASH_SUFFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)[ \t]*\((?:#|gh-)\d+\)[ \t]*$").expect("valid regex"));

// Stage 5: GitHub issue and PR URLs
static RE_GITHUB_URLS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https?://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+/(?:issues|pull)/\d+")
        .expect("valid regex")
});

// Stage 6: Inline issue references (e.g. #1234, gh-5678)
static RE_INLINE_ISSUE_NUMS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:\bgh-|\B#)\d+\b").expect("valid regex"));

// Whitespace normalization regexes
static RE_MULTI_SPACES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\S)[ \t]{2,}").expect("valid regex"));

static RE_TRAILING_LINE_SPACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)[ \t]+$").expect("valid regex"));

static RE_MULTI_NEWLINES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\n{3,}").expect("valid regex"));

fn apply_sub(re: &Regex, text: &str, replacement: &str) -> (String, usize) {
    let count = re.find_iter(text).count();
    if count > 0 {
        (re.replace_all(text, replacement).into_owned(), count)
    } else {
        (text.to_string(), 0)
    }
}

/// Sanitize raw commit / diff text by stripping issue references and trailers.
///
/// Executes 6 deterministic regex stages:
/// 1. Merge header lines
/// 2. Action trailer lines
/// 3. Attribution trailer lines
/// 4. Squash PR suffixes
/// 5. GitHub issue & PR URLs
/// 6. Inline issue numbers (#123, gh-123)
///    followed by whitespace normalization.
#[must_use]
pub fn sanitize_chunk_text(text: &str) -> SanitizedChunk {
    if text.is_empty() {
        return SanitizedChunk {
            sanitized_text: String::new(),
            strip_count: 0,
            modified: false,
        };
    }

    let mut working = text.replace("\r\n", "\n");
    let mut total_strips = 0;

    // Stage 1: Merge header lines
    let (res, c1) = apply_sub(&RE_MERGE_HEADER, &working, "");
    working = res;
    total_strips += c1;

    // Stage 2: Action trailers
    let (res, c2) = apply_sub(&RE_ACTION_TRAILERS, &working, "");
    working = res;
    total_strips += c2;

    // Stage 3: Attribution trailers
    let (res, c3) = apply_sub(&RE_ATTRIBUTION_TRAILERS, &working, "");
    working = res;
    total_strips += c3;

    // Stage 4: Squash PR suffixes
    let (res, c4) = apply_sub(&RE_SQUASH_SUFFIX, &working, "");
    working = res;
    total_strips += c4;

    // Stage 5: GitHub issue and PR URLs
    let (res, c5) = apply_sub(&RE_GITHUB_URLS, &working, "");
    working = res;
    total_strips += c5;

    // Stage 6: Inline issue numbers
    let (res, c6) = apply_sub(&RE_INLINE_ISSUE_NUMS, &working, "");
    working = res;
    total_strips += c6;

    // Whitespace cleanup
    working = RE_MULTI_SPACES.replace_all(&working, "$1 ").into_owned();
    working = RE_TRAILING_LINE_SPACE
        .replace_all(&working, "")
        .into_owned();
    working = RE_MULTI_NEWLINES.replace_all(&working, "\n\n").into_owned();

    let sanitized = working.trim().to_string();
    let modified = total_strips > 0 || sanitized != text.trim();

    SanitizedChunk {
        sanitized_text: sanitized,
        strip_count: total_strips,
        modified,
    }
}

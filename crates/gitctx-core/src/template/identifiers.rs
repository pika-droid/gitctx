//! Symmetric identifier delta extractor for code diffs.
//!
//! Extracts added and removed identifiers from unified diff lines and applies
//! cross-language keyword filtering per D-11.

use crate::git::DiffHunk;
use crate::template::keywords::is_keyword;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};
use std::sync::LazyLock;

static RE_IDENTIFIER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b[a-zA-Z_][a-zA-Z0-9_]{2,}\b").expect("valid identifier regex")
});

/// The extracted symmetric token delta representing added and removed code identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IdentifierDelta {
    /// Identifiers that appear in added lines but not in removed lines.
    pub added_identifiers: Vec<String>,
    /// Identifiers that appear in removed lines but not in added lines.
    pub removed_identifiers: Vec<String>,
}

impl IdentifierDelta {
    /// Returns true if both added and removed identifier lists are empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.added_identifiers.is_empty() && self.removed_identifiers.is_empty()
    }
}

/// Extractor for symmetric code identifier deltas.
pub struct IdentifierExtractor;

impl IdentifierExtractor {
    /// Extract identifier tokens from a single text line, filtering out language keywords.
    #[must_use]
    pub fn extract_tokens(line: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        for mat in RE_IDENTIFIER.find_iter(line) {
            let tok = mat.as_str();
            if !is_keyword(tok) {
                tokens.push(tok.to_string());
            }
        }
        tokens
    }

    /// Extract symmetric identifier delta from a raw unified diff string.
    #[must_use]
    pub fn extract_from_diff_text(diff_text: &str) -> IdentifierDelta {
        Self::extract_from_lines(diff_text.lines())
    }

    /// Extract symmetric identifier delta from an iterator of diff lines.
    pub fn extract_from_lines<I, S>(lines: I) -> IdentifierDelta
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut added_raw: HashSet<String> = HashSet::new();
        let mut removed_raw: HashSet<String> = HashSet::new();

        for line_ref in lines {
            let line = line_ref.as_ref();
            if line.starts_with('+') && !line.starts_with("+++") {
                let content = &line[1..];
                for tok in Self::extract_tokens(content) {
                    added_raw.insert(tok);
                }
            } else if line.starts_with('-') && !line.starts_with("---") {
                let content = &line[1..];
                for tok in Self::extract_tokens(content) {
                    removed_raw.insert(tok);
                }
            }
        }

        // Symmetric difference: added_only = added - removed, removed_only = removed - added
        let added_only: BTreeSet<String> = added_raw
            .difference(&removed_raw)
            .cloned()
            .collect();
        let removed_only: BTreeSet<String> = removed_raw
            .difference(&added_raw)
            .cloned()
            .collect();

        IdentifierDelta {
            added_identifiers: added_only.into_iter().collect(),
            removed_identifiers: removed_only.into_iter().collect(),
        }
    }

    /// Extract symmetric identifier delta from a slice of `DiffHunk` records.
    #[must_use]
    pub fn extract_from_hunks(hunks: &[DiffHunk]) -> IdentifierDelta {
        let lines: Vec<&str> = hunks
            .iter()
            .flat_map(|h| h.lines.iter().map(String::as_str))
            .collect();
        Self::extract_from_lines(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_symmetric_diff() {
        let diff = r#"
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -10,4 +10,4 @@
-    let old_cache_key = calculate_old_hash(user_id, session_id);
+    let new_cache_key = calculate_new_hash(user_id, session_id, ttl_seconds);
"#;
        let delta = IdentifierExtractor::extract_from_diff_text(diff);

        // 'user_id' and 'session_id' are in both + and -, so they are filtered by symmetric difference.
        // 'let' is a keyword, so filtered.
        assert_eq!(
            delta.added_identifiers,
            vec!["calculate_new_hash", "new_cache_key", "ttl_seconds"]
        );
        assert_eq!(
            delta.removed_identifiers,
            vec!["calculate_old_hash", "old_cache_key"]
        );
    }
}

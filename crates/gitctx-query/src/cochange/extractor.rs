//! Co-change pair extraction engine with commit-size filtering and bot exclusion per GRAPH-01.
//!
//! Scans all indexed commits from `StorageEngine`, filters out bulk reformats (>25 files)
//! and bot-authored commits, and emits ordered pairwise co-change relationships at both
//! file and function granularity.

use crate::cochange::config::CoChangeConfig;
use gitctx_index::storage::StorageEngine;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

/// A raw co-change pair extracted from a single qualifying commit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawCoChangePair {
    /// First entity identifier (file path or qualified name), lexicographically smaller.
    pub entity_a: String,
    /// Second entity identifier, lexicographically larger.
    pub entity_b: String,
    /// Commit hash where both entities were touched.
    pub commit_hash: String,
    /// Authored timestamp (unix seconds) for time-decay weighting.
    pub authored_at: i64,
}

/// Statistics from the extraction pass, logged for observability per GRAPH-01.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractionStats {
    /// Total commits examined.
    pub total_commits: usize,
    /// Total commits that passed all filters.
    pub qualifying_commits: usize,
    /// Commits excluded by file-count threshold.
    pub size_filtered: usize,
    /// Commits excluded by bot email pattern.
    pub bot_filtered: usize,
    /// Total file-level co-change pairs emitted.
    pub total_file_pairs: usize,
    /// Total function-level co-change pairs emitted.
    pub total_function_pairs: usize,
}

struct CommitGroup {
    email: String,
    authored_at: i64,
    paths: BTreeSet<String>,
    functions: BTreeSet<String>,
}

/// Extracts co-change pairs from the indexed commit history.
pub struct CoChangeExtractor<'a> {
    engine: &'a StorageEngine,
    config: &'a CoChangeConfig,
}

impl<'a> CoChangeExtractor<'a> {
    pub fn new(engine: &'a StorageEngine, config: &'a CoChangeConfig) -> Self {
        Self { engine, config }
    }

    /// Check if a commit email matches any bot pattern.
    fn is_bot_email(&self, email: &str) -> bool {
        let email_lower = email.to_lowercase();
        self.config
            .bot_patterns
            .iter()
            .any(|pattern| email_lower.contains(&pattern.to_lowercase()))
    }

    /// Generate all unordered pairs from a list of entities, with canonical ordering (a < b).
    fn generate_pairs(entities: &[String]) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        for i in 0..entities.len() {
            for j in (i + 1)..entities.len() {
                let (a, b) = if entities[i] <= entities[j] {
                    (entities[i].clone(), entities[j].clone())
                } else {
                    (entities[j].clone(), entities[i].clone())
                };
                pairs.push((a, b));
            }
        }
        pairs
    }

    /// Run the full extraction pipeline, returning file-level and function-level pairs.
    pub fn extract(
        &self,
    ) -> Result<(Vec<RawCoChangePair>, Vec<RawCoChangePair>, ExtractionStats), String> {
        let mut stats = ExtractionStats::default();
        let mut file_pairs = Vec::new();
        let mut function_pairs = Vec::new();

        // 1. Single bulk scan joining function_touches and commits
        let touch_events = self
            .engine
            .query_cochange_touch_events()
            .map_err(|e| format!("Failed to query cochange touch events: {e}"))?;

        let mut commit_order: Vec<String> = Vec::new();
        let mut commit_map: HashMap<String, CommitGroup> = HashMap::new();

        for (commit_hash, email, authored_at, path, qualname) in touch_events {
            let entry = commit_map.entry(commit_hash.clone()).or_insert_with(|| {
                commit_order.push(commit_hash);
                CommitGroup {
                    email,
                    authored_at,
                    paths: BTreeSet::new(),
                    functions: BTreeSet::new(),
                }
            });
            entry.paths.insert(path);
            entry.functions.insert(qualname);
        }

        stats.total_commits = commit_order.len();

        for commit_hash in commit_order {
            let Some(group) = commit_map.remove(&commit_hash) else {
                continue;
            };

            // 2. Apply commit-size filter (GRAPH-01)
            if group.paths.len() > self.config.max_commit_files {
                stats.size_filtered += 1;
                tracing::debug!(
                    commit = %commit_hash,
                    files = group.paths.len(),
                    threshold = self.config.max_commit_files,
                    "Excluding commit: too many files"
                );
                continue;
            }

            // 3. Apply bot filter (GRAPH-01)
            if self.is_bot_email(&group.email) {
                stats.bot_filtered += 1;
                tracing::debug!(
                    commit = %commit_hash,
                    email = %group.email,
                    "Excluding commit: bot author"
                );
                continue;
            }

            stats.qualifying_commits += 1;

            // 4. Extract file-level pairs
            let paths: Vec<String> = group.paths.into_iter().collect();
            if paths.len() >= 2 {
                for (a, b) in Self::generate_pairs(&paths) {
                    file_pairs.push(RawCoChangePair {
                        entity_a: a,
                        entity_b: b,
                        commit_hash: commit_hash.clone(),
                        authored_at: group.authored_at,
                    });
                }
            }

            // 5. Extract function-level pairs (by qualified name)
            let func_names: Vec<String> = group.functions.into_iter().collect();
            if func_names.len() >= 2 {
                for (a, b) in Self::generate_pairs(&func_names) {
                    function_pairs.push(RawCoChangePair {
                        entity_a: a,
                        entity_b: b,
                        commit_hash: commit_hash.clone(),
                        authored_at: group.authored_at,
                    });
                }
            }
        }

        stats.total_file_pairs = file_pairs.len();
        stats.total_function_pairs = function_pairs.len();

        tracing::info!(
            total = stats.total_commits,
            qualifying = stats.qualifying_commits,
            size_filtered = stats.size_filtered,
            bot_filtered = stats.bot_filtered,
            file_pairs = stats.total_file_pairs,
            function_pairs = stats.total_function_pairs,
            "Co-change extraction complete"
        );

        Ok((file_pairs, function_pairs, stats))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_pairs_empty() {
        let pairs = CoChangeExtractor::generate_pairs(&[]);
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_generate_pairs_single() {
        let pairs = CoChangeExtractor::generate_pairs(&["a.rs".to_string()]);
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_generate_pairs_two() {
        let pairs = CoChangeExtractor::generate_pairs(&["b.rs".to_string(), "a.rs".to_string()]);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("a.rs".to_string(), "b.rs".to_string()));
    }

    #[test]
    fn test_generate_pairs_three() {
        let entities = vec!["c.rs".to_string(), "a.rs".to_string(), "b.rs".to_string()];
        let pairs = CoChangeExtractor::generate_pairs(&entities);
        assert_eq!(pairs.len(), 3);
        // All pairs should be canonically ordered (a < b)
        for (a, b) in &pairs {
            assert!(a <= b, "Pair ({a}, {b}) is not canonically ordered");
        }
    }

    #[test]
    fn test_is_bot_email() {
        let config = CoChangeConfig::with_reference_epoch(0);
        let engine_placeholder = || -> bool { false };
        // Test bot patterns directly
        let email_bot = "dependabot[bot]@users.noreply.github.com";
        let email_human = "developer@example.com";

        let lower_bot = email_bot.to_lowercase();
        let lower_human = email_human.to_lowercase();

        let has_bot = config
            .bot_patterns
            .iter()
            .any(|p| lower_bot.contains(&p.to_lowercase()));
        let has_human = config
            .bot_patterns
            .iter()
            .any(|p| lower_human.contains(&p.to_lowercase()));

        let _ = engine_placeholder;
        assert!(has_bot, "Bot email should match bot patterns");
        assert!(!has_human, "Human email should not match bot patterns");
    }
}

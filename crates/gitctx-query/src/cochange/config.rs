//! Co-change analysis configuration: commit-size filtering thresholds, bot exclusion patterns,
//! minimum support requirements, and exponential time-decay parameters per GRAPH-01, GRAPH-02.

use serde::{Deserialize, Serialize};

/// Default maximum files per commit before the commit is considered a bulk reformat (GRAPH-01).
pub const DEFAULT_MAX_COMMIT_FILES: usize = 25;

/// Default exponential half-life in days for time-decay weighting (GRAPH-02).
pub const DEFAULT_DECAY_HALF_LIFE_DAYS: f64 = 180.0;

/// Default minimum raw support count to emit a coupling edge.
pub const DEFAULT_MIN_SUPPORT: u32 = 2;

/// Default bot author email substring patterns.
pub const DEFAULT_BOT_PATTERNS: &[&str] = &["[bot]", "noreply", "dependabot", "renovate"];

/// Configuration for co-change extraction and metric computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoChangeConfig {
    /// Maximum files touched in a commit to be included (GRAPH-01).
    /// Commits touching more files are considered reformats/bulk changes and are excluded.
    pub max_commit_files: usize,

    /// Bot author email substring patterns to exclude (GRAPH-01).
    /// Commits whose `email` field contains any of these substrings are filtered out.
    pub bot_patterns: Vec<String>,

    /// Minimum raw support count to emit a coupling edge.
    pub min_support: u32,

    /// Exponential half-life in days for time-decay weighting (GRAPH-02).
    /// Co-changes from `half_life_days` ago contribute 50% weight.
    pub decay_half_life_days: f64,

    /// Reference epoch (unix timestamp) for decay computation.
    /// Typically set to the `authored_at` of the most recent commit (HEAD).
    pub reference_epoch: i64,
}

impl CoChangeConfig {
    /// Create a new config with the given reference epoch and all other fields at defaults.
    pub fn with_reference_epoch(reference_epoch: i64) -> Self {
        Self {
            max_commit_files: DEFAULT_MAX_COMMIT_FILES,
            bot_patterns: DEFAULT_BOT_PATTERNS
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            min_support: DEFAULT_MIN_SUPPORT,
            decay_half_life_days: DEFAULT_DECAY_HALF_LIFE_DAYS,
            reference_epoch,
        }
    }
}

impl Default for CoChangeConfig {
    fn default() -> Self {
        Self::with_reference_epoch(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_values() {
        let config = CoChangeConfig::with_reference_epoch(1_700_000_000);
        assert_eq!(config.max_commit_files, 25);
        assert_eq!(config.min_support, 2);
        assert!((config.decay_half_life_days - 180.0).abs() < f64::EPSILON);
        assert_eq!(config.reference_epoch, 1_700_000_000);
        assert!(config.bot_patterns.contains(&"[bot]".to_string()));
        assert!(config.bot_patterns.contains(&"dependabot".to_string()));
    }
}

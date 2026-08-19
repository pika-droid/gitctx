//! Normalized co-change association metrics per GRAPH-02: support, confidence P(B|A), and lift.
//!
//! Aggregates raw co-change pairs into per-pair metrics with exponential time-decay weighting,
//! then normalizes into directional confidence and lift measures.

use crate::cochange::config::CoChangeConfig;
use crate::cochange::decay::time_decay_weight;
use crate::cochange::extractor::RawCoChangePair;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Normalized co-change metrics for a single entity pair per GRAPH-02.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoChangeMetrics {
    /// Entity A identifier (file path or qualified name).
    pub entity_a: String,
    /// Entity B identifier.
    pub entity_b: String,
    /// Raw co-occurrence count (unweighted).
    pub support: u32,
    /// Time-decay-weighted support sum.
    pub weighted_support: f64,
    /// P(B|A) — directional confidence from A to B.
    pub confidence_a_to_b: f64,
    /// P(A|B) — directional confidence from B to A.
    pub confidence_b_to_a: f64,
    /// Lift = P(A∩B) / (P(A) * P(B)) — association strength above random chance.
    pub lift: f64,
    /// Total qualifying commits used as denominator N.
    pub total_qualifying_commits: usize,
}

/// Aggregator key for a pair (canonical ordering: entity_a < entity_b).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PairKey {
    entity_a: String,
    entity_b: String,
}

/// Accumulator for raw pair aggregation.
#[derive(Debug, Default)]
struct PairAccumulator {
    raw_count: u32,
    weighted_sum: f64,
}

/// Computes normalized co-change metrics from raw pairs.
pub struct MetricsComputer;

impl MetricsComputer {
    /// Compute metrics for a set of raw co-change pairs.
    ///
    /// # Arguments
    /// * `pairs` — raw co-change pairs from the extractor
    /// * `qualifying_commit_count` — total number of qualifying commits (N)
    /// * `config` — configuration with decay parameters and min_support threshold
    pub fn compute(
        pairs: &[RawCoChangePair],
        qualifying_commit_count: usize,
        config: &CoChangeConfig,
    ) -> Vec<CoChangeMetrics> {
        if pairs.is_empty() || qualifying_commit_count == 0 {
            return Vec::new();
        }

        // 1. Aggregate pairs with time-decay weights
        let mut pair_accumulators: HashMap<PairKey, PairAccumulator> = HashMap::new();
        let mut entity_weighted_counts: HashMap<String, f64> = HashMap::new();
        let mut entity_raw_counts: HashMap<String, u32> = HashMap::new();

        // Track which entities appear in which commits (for per-entity occurrence counts)
        let mut entity_commits: HashMap<String, std::collections::HashSet<String>> = HashMap::new();

        for pair in pairs {
            let weight = time_decay_weight(
                pair.authored_at,
                config.reference_epoch,
                config.decay_half_life_days,
            );

            let key = PairKey {
                entity_a: pair.entity_a.clone(),
                entity_b: pair.entity_b.clone(),
            };

            let acc = pair_accumulators.entry(key).or_default();
            acc.raw_count += 1;
            acc.weighted_sum += weight;

            // Track entity-level occurrences
            entity_commits
                .entry(pair.entity_a.clone())
                .or_default()
                .insert(pair.commit_hash.clone());
            entity_commits
                .entry(pair.entity_b.clone())
                .or_default()
                .insert(pair.commit_hash.clone());
        }

        // 2. Compute per-entity weighted counts from commit-level occurrences
        for (entity, commits) in &entity_commits {
            entity_raw_counts.insert(entity.clone(), commits.len() as u32);
            // For weighted count, we'd need per-commit timestamps, but we approximate
            // by using the raw count (each commit gets weight from its pairs).
            // A more precise approach: iterate pairs again per entity.
        }

        // Precise weighted entity counts: sum decay weights for each entity across its commits
        // We need to track (entity -> [(commit_hash, authored_at)]) for proper weighting
        let mut entity_commit_epochs: HashMap<String, HashMap<String, i64>> = HashMap::new();
        for pair in pairs {
            entity_commit_epochs
                .entry(pair.entity_a.clone())
                .or_default()
                .insert(pair.commit_hash.clone(), pair.authored_at);
            entity_commit_epochs
                .entry(pair.entity_b.clone())
                .or_default()
                .insert(pair.commit_hash.clone(), pair.authored_at);
        }

        for (entity, commit_epochs) in &entity_commit_epochs {
            let weighted: f64 = commit_epochs
                .values()
                .map(|epoch| {
                    time_decay_weight(*epoch, config.reference_epoch, config.decay_half_life_days)
                })
                .sum();
            entity_weighted_counts.insert(entity.clone(), weighted);
        }

        // 3. Compute normalized metrics
        let n = qualifying_commit_count as f64;
        let mut results = Vec::new();

        for (key, acc) in &pair_accumulators {
            if acc.raw_count < config.min_support {
                continue;
            }

            let w_a = entity_weighted_counts
                .get(&key.entity_a)
                .copied()
                .unwrap_or(1.0);
            let w_b = entity_weighted_counts
                .get(&key.entity_b)
                .copied()
                .unwrap_or(1.0);

            let confidence_a_to_b = if w_a > 0.0 {
                acc.weighted_sum / w_a
            } else {
                0.0
            };
            let confidence_b_to_a = if w_b > 0.0 {
                acc.weighted_sum / w_b
            } else {
                0.0
            };

            let lift = if w_a > 0.0 && w_b > 0.0 {
                (acc.weighted_sum * n) / (w_a * w_b)
            } else {
                0.0
            };

            results.push(CoChangeMetrics {
                entity_a: key.entity_a.clone(),
                entity_b: key.entity_b.clone(),
                support: acc.raw_count,
                weighted_support: acc.weighted_sum,
                confidence_a_to_b,
                confidence_b_to_a,
                lift,
                total_qualifying_commits: qualifying_commit_count,
            });
        }

        // Sort deterministically by (support desc, entity_a asc, entity_b asc)
        results.sort_by(|a, b| {
            b.support
                .cmp(&a.support)
                .then_with(|| a.entity_a.cmp(&b.entity_a))
                .then_with(|| a.entity_b.cmp(&b.entity_b))
        });

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pair(a: &str, b: &str, hash: &str, epoch: i64) -> RawCoChangePair {
        let (ea, eb) = if a <= b {
            (a.to_string(), b.to_string())
        } else {
            (b.to_string(), a.to_string())
        };
        RawCoChangePair {
            entity_a: ea,
            entity_b: eb,
            commit_hash: hash.to_string(),
            authored_at: epoch,
        }
    }

    #[test]
    fn test_empty_pairs() {
        let config = CoChangeConfig::with_reference_epoch(1_000_000);
        let results = MetricsComputer::compute(&[], 10, &config);
        assert!(results.is_empty());
    }

    #[test]
    fn test_single_pair_below_min_support() {
        let config = CoChangeConfig::with_reference_epoch(1_000_000);
        let pairs = vec![make_pair("a.rs", "b.rs", "abc123", 1_000_000)];
        let results = MetricsComputer::compute(&pairs, 10, &config);
        // min_support default is 2, so single occurrence is filtered
        assert!(results.is_empty());
    }

    #[test]
    fn test_pair_at_min_support() {
        let ref_epoch = 1_000_000;
        let mut config = CoChangeConfig::with_reference_epoch(ref_epoch);
        config.min_support = 2;

        let pairs = vec![
            make_pair("a.rs", "b.rs", "abc123", ref_epoch),
            make_pair("a.rs", "b.rs", "def456", ref_epoch),
        ];
        let results = MetricsComputer::compute(&pairs, 5, &config);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].support, 2);
        assert_eq!(results[0].entity_a, "a.rs");
        assert_eq!(results[0].entity_b, "b.rs");
    }

    #[test]
    fn test_confidence_symmetry_with_equal_counts() {
        let ref_epoch = 1_000_000;
        let mut config = CoChangeConfig::with_reference_epoch(ref_epoch);
        config.min_support = 1;

        // a.rs and b.rs always appear together (in 3 commits), never alone
        let pairs = vec![
            make_pair("a.rs", "b.rs", "c1", ref_epoch),
            make_pair("a.rs", "b.rs", "c2", ref_epoch),
            make_pair("a.rs", "b.rs", "c3", ref_epoch),
        ];
        let results = MetricsComputer::compute(&pairs, 5, &config);
        assert_eq!(results.len(), 1);

        // When both appear only together, P(B|A) = P(A|B) = 1.0
        assert!(
            (results[0].confidence_a_to_b - 1.0).abs() < 1e-10,
            "Expected confidence_a_to_b ≈ 1.0, got {}",
            results[0].confidence_a_to_b
        );
        assert!(
            (results[0].confidence_b_to_a - 1.0).abs() < 1e-10,
            "Expected confidence_b_to_a ≈ 1.0, got {}",
            results[0].confidence_b_to_a
        );
    }

    #[test]
    fn test_time_decay_reduces_weighted_support() {
        let ref_epoch = 1_000_000;
        let half_life = 180.0;
        let mut config = CoChangeConfig::with_reference_epoch(ref_epoch);
        config.decay_half_life_days = half_life;
        config.min_support = 1;

        let old_epoch = ref_epoch - (half_life * 86_400.0) as i64; // exactly 1 half-life ago
        let pairs = vec![
            make_pair("a.rs", "b.rs", "c1", ref_epoch), // weight 1.0
            make_pair("a.rs", "b.rs", "c2", old_epoch), // weight 0.5
        ];
        let results = MetricsComputer::compute(&pairs, 5, &config);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].support, 2);
        // weighted_support should be approximately 1.0 + 0.5 = 1.5
        assert!(
            (results[0].weighted_support - 1.5).abs() < 0.01,
            "Expected weighted_support ≈ 1.5, got {}",
            results[0].weighted_support
        );
    }
}

//! Deep service for querying precomputed co-change coupling and impact metrics in <2ms.

use crate::cochange::graph::{CoChangeGraph, CoupledEntity, EntityType};
use crate::cochange::impact::{ImpactAnalyzer, ImpactResult};
use crate::cochange::metrics::CoChangeMetrics;
use gitctx_core::error::{CoreError, Result};
use gitctx_index::storage::{CoChangeMetricRow, StorageEngine};
use serde::{Deserialize, Serialize};

/// Summary of global co-change impact metrics across the repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalImpactSummary {
    pub top_file_couplings: Vec<CoChangeMetrics>,
    pub top_function_couplings: Vec<CoChangeMetrics>,
}

/// Deep service providing <2ms impact analysis using precomputed SQLite tables.
pub struct ImpactService<'a> {
    storage: &'a StorageEngine,
}

impl<'a> ImpactService<'a> {
    pub fn new(storage: &'a StorageEngine) -> Self {
        Self { storage }
    }

    /// Analyze impact for target (file or function) directly querying precomputed SQLite metrics in <2ms.
    pub fn analyze(&self, target: &str, limit: usize) -> Result<ImpactResult> {
        let likely_file = target.contains('/')
            || target.contains('\\')
            || (target.contains('.') && !target.contains("::"));

        let (rows, entity_type) = if likely_file {
            let file_rows = self
                .storage
                .query_cochange_file_metrics_for(target, limit)
                .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;
            if file_rows.is_empty() {
                let func_rows = self
                    .storage
                    .query_cochange_function_metrics_for(target, limit)
                    .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;
                if func_rows.is_empty() {
                    (Vec::new(), EntityType::File)
                } else {
                    (func_rows, EntityType::Function)
                }
            } else {
                (file_rows, EntityType::File)
            }
        } else {
            let func_rows = self
                .storage
                .query_cochange_function_metrics_for(target, limit)
                .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;
            if func_rows.is_empty() {
                let file_rows = self
                    .storage
                    .query_cochange_file_metrics_for(target, limit)
                    .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;
                if file_rows.is_empty() {
                    (Vec::new(), EntityType::Function)
                } else {
                    (file_rows, EntityType::File)
                }
            } else {
                (func_rows, EntityType::Function)
            }
        };

        if !rows.is_empty() {
            // Build coupled entities from precomputed rows
            let mut coupled = Vec::with_capacity(rows.len());
            for r in &rows {
                let (coupled_name, confidence) = if r.entity_a == target {
                    (r.entity_b.clone(), r.confidence_a_to_b)
                } else {
                    (r.entity_a.clone(), r.confidence_b_to_a)
                };
                coupled.push(CoupledEntity {
                    name: coupled_name,
                    entity_type,
                    confidence,
                    lift: r.lift,
                    support: r.support,
                    weighted_support: r.weighted_support,
                });
            }

            let coupled_names: Vec<String> = coupled.iter().map(|c| c.name.clone()).collect();
            let target_commits = match entity_type {
                EntityType::Function => self
                    .storage
                    .query_commits_touching_function(target)
                    .unwrap_or_default(),
                EntityType::File => self
                    .storage
                    .query_commits_touching_path(target)
                    .unwrap_or_default(),
            };

            let mut followup_fix_count: u32 = 0;
            if !target_commits.is_empty() && !coupled_names.is_empty() {
                let mut seen_coupled_fixes = std::collections::HashSet::new();
                for coupled_name in &coupled_names {
                    let coupled_commits = match entity_type {
                        EntityType::Function => self
                            .storage
                            .query_commits_touching_function(coupled_name)
                            .unwrap_or_default(),
                        EntityType::File => self
                            .storage
                            .query_commits_touching_path(coupled_name)
                            .unwrap_or_default(),
                    };

                    for (c_hash, c_epoch, c_msg) in &coupled_commits {
                        if !ImpactAnalyzer::is_fix_commit(c_msg) {
                            continue;
                        }
                        for (_, t_epoch, _) in &target_commits {
                            let delta = c_epoch - t_epoch;
                            if delta > 0 && delta <= 7 * 86_400 {
                                if seen_coupled_fixes.insert((coupled_name.clone(), c_hash.clone()))
                                {
                                    followup_fix_count += 1;
                                }
                                break;
                            }
                        }
                    }
                }
            }

            let max_confidence = coupled.iter().map(|c| c.confidence).fold(0.0_f64, f64::max);
            let target_touches = target_commits.len();
            let normalized_fix_ratio = if target_touches > 0 {
                (f64::from(followup_fix_count) / target_touches as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let normalized_complexity =
                (coupled.len() as f64 / limit.max(1) as f64).clamp(0.0, 1.0);
            let risk_score =
                (0.5 * max_confidence + 0.3 * normalized_fix_ratio + 0.2 * normalized_complexity)
                    .clamp(0.0, 1.0);

            return Ok(ImpactResult {
                target: target.to_string(),
                entity_type,
                risk_score,
                followup_fix_count,
                coupled_entities: coupled,
            });
        }

        // Fallback: If precomputed tables are empty (e.g. unindexed db or fresh memory fixture),
        // run ad-hoc computation via Extractor & Graph
        let config = crate::cochange::config::CoChangeConfig::default();
        let extractor = crate::cochange::extractor::CoChangeExtractor::new(self.storage, &config);
        let (file_pairs, func_pairs, stats) =
            extractor.extract().map_err(CoreError::InvariantViolation)?;

        let is_file = if likely_file {
            true
        } else if target.contains("::")
            || !self
                .storage
                .query_commits_touching_function(target)
                .unwrap_or_default()
                .is_empty()
        {
            false
        } else {
            !self
                .storage
                .query_commits_touching_path(target)
                .unwrap_or_default()
                .is_empty()
        };

        let (metrics, e_type) = if is_file {
            let m = crate::cochange::metrics::MetricsComputer::compute(
                &file_pairs,
                stats.qualifying_commits,
                &config,
            );
            (m, EntityType::File)
        } else {
            let m = crate::cochange::metrics::MetricsComputer::compute(
                &func_pairs,
                stats.qualifying_commits,
                &config,
            );
            (m, EntityType::Function)
        };

        let graph = CoChangeGraph::from_metrics(&metrics, e_type);
        let analyzer = ImpactAnalyzer::new(self.storage, &graph);
        Ok(analyzer.analyze(target, limit))
    }

    /// Return global co-change couplings from precomputed tables.
    pub fn global_metrics(&self, limit: usize) -> Result<GlobalImpactSummary> {
        let file_rows = self
            .storage
            .query_global_file_cochange_metrics(limit)
            .unwrap_or_default();
        let func_rows = self
            .storage
            .query_global_function_cochange_metrics(limit)
            .unwrap_or_default();

        let row_to_metric = |r: CoChangeMetricRow| CoChangeMetrics {
            entity_a: r.entity_a,
            entity_b: r.entity_b,
            support: r.support,
            weighted_support: r.weighted_support,
            confidence_a_to_b: r.confidence_a_to_b,
            confidence_b_to_a: r.confidence_b_to_a,
            lift: r.lift,
            total_qualifying_commits: 0,
        };

        Ok(GlobalImpactSummary {
            top_file_couplings: file_rows.into_iter().map(row_to_metric).collect(),
            top_function_couplings: func_rows.into_iter().map(row_to_metric).collect(),
        })
    }
}

//! Impact risk scoring engine per GRAPH-03 and GRAPH-05.
//!
//! Computes composite risk scores for entities based on coupling strength,
//! follow-up fix frequency, and code churn.

use crate::cochange::graph::{CoChangeGraph, CoupledEntity, EntityType};
use gitctx_index::storage::StorageEngine;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// Regex pattern for identifying fix commits from commit messages.
static FIX_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(fix|bug|patch|hotfix|bugfix)\b").expect("Invalid fix pattern regex")
});

/// Follow-up fix detection window in seconds (7 days).
const FOLLOWUP_WINDOW_SECONDS: i64 = 7 * 86_400;

/// Result of impact analysis for a target entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactResult {
    /// The target entity being analyzed.
    pub target: String,
    /// Entity type.
    pub entity_type: EntityType,
    /// Composite risk score in [0.0, 1.0].
    pub risk_score: f64,
    /// Number of follow-up fix commits for coupled entities (GRAPH-03).
    pub followup_fix_count: u32,
    /// Top coupled entities sorted by weighted_support descending.
    pub coupled_entities: Vec<CoupledEntity>,
}

/// Impact risk scoring engine.
pub struct ImpactAnalyzer<'a> {
    engine: &'a StorageEngine,
    graph: &'a CoChangeGraph,
}

impl<'a> ImpactAnalyzer<'a> {
    pub fn new(engine: &'a StorageEngine, graph: &'a CoChangeGraph) -> Self {
        Self { engine, graph }
    }

    /// Check if a commit message indicates a fix.
    pub fn is_fix_commit(message: &str) -> bool {
        FIX_PATTERN.is_match(message)
    }

    /// Determine entity type for a given identifier using graph and storage knowledge.
    pub fn resolve_entity_type(&self, entity: &str) -> EntityType {
        if let Some(t) = self.graph.entity_type(entity) {
            return t;
        }
        if entity.contains("::") {
            return EntityType::Function;
        }
        if entity.contains('/') || entity.contains('\\') {
            return EntityType::File;
        }
        if let Ok(func_commits) = self.engine.query_commits_touching_function(entity) {
            if !func_commits.is_empty() {
                return EntityType::Function;
            }
        }
        if let Ok(path_commits) = self.engine.query_commits_touching_path(entity) {
            if !path_commits.is_empty() {
                return EntityType::File;
            }
        }
        if entity.contains('.') {
            EntityType::File
        } else {
            EntityType::Function
        }
    }

    /// Helper to get commits touching an entity (path or function qualified name).
    fn get_commits_touching(
        &self,
        entity: &str,
        entity_type: EntityType,
    ) -> Vec<(String, i64, String)> {
        match entity_type {
            EntityType::Function => self
                .engine
                .query_commits_touching_function(entity)
                .unwrap_or_default(),
            EntityType::File => self
                .engine
                .query_commits_touching_path(entity)
                .unwrap_or_default(),
        }
    }

    /// Count follow-up fix commits per GRAPH-03.
    ///
    /// For each coupled entity, counts how many unique "fix" commits touched the coupled entity
    /// within `FOLLOWUP_WINDOW_SECONDS` of a commit that touched the target.
    fn count_followup_fixes(
        &self,
        target: &str,
        target_type: EntityType,
        coupled_names: &[String],
    ) -> u32 {
        let target_commits = self.get_commits_touching(target, target_type);
        if target_commits.is_empty() || coupled_names.is_empty() {
            return 0;
        }

        let mut fix_count: u32 = 0;
        let mut seen_coupled_fixes = std::collections::HashSet::new();

        for coupled in coupled_names {
            let coupled_type = self.resolve_entity_type(coupled);
            let coupled_commits = self.get_commits_touching(coupled, coupled_type);

            for (coupled_hash, coupled_epoch, coupled_msg) in &coupled_commits {
                if !Self::is_fix_commit(coupled_msg) {
                    continue;
                }
                for (_, target_epoch, _) in &target_commits {
                    let time_delta = coupled_epoch - target_epoch;
                    if time_delta > 0 && time_delta <= FOLLOWUP_WINDOW_SECONDS {
                        if seen_coupled_fixes.insert((coupled.clone(), coupled_hash.clone())) {
                            fix_count += 1;
                        }
                        break;
                    }
                }
            }
        }

        fix_count
    }

    /// Analyze impact for a target entity.
    ///
    /// Returns the composite risk score combining coupling strength,
    /// follow-up fix ratio, and churn.
    pub fn analyze(&self, target: &str, top_k: usize) -> ImpactResult {
        let target_type = self.resolve_entity_type(target);
        let coupled = self.graph.top_coupled(target, top_k);

        let coupled_names: Vec<String> = coupled.iter().map(|c| c.name.clone()).collect();

        // Compute follow-up fix count
        let followup_fix_count = self.count_followup_fixes(target, target_type, &coupled_names);

        // Compute risk score components
        let max_confidence = coupled.iter().map(|c| c.confidence).fold(0.0_f64, f64::max);

        // Get target's total touches for fix ratio normalization
        let target_touches = self.get_commits_touching(target, target_type).len();

        let normalized_fix_ratio = if target_touches > 0 {
            (f64::from(followup_fix_count) / target_touches as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Simplified churn proxy: use the count of coupled entities as a complexity signal
        let normalized_complexity = (coupled.len() as f64 / top_k.max(1) as f64).clamp(0.0, 1.0);

        // Composite risk score
        let risk_score =
            (0.5 * max_confidence + 0.3 * normalized_fix_ratio + 0.2 * normalized_complexity)
                .clamp(0.0, 1.0);

        let entity_type = if coupled.is_empty() {
            target_type
        } else {
            coupled[0].entity_type
        };

        ImpactResult {
            target: target.to_string(),
            entity_type,
            risk_score,
            followup_fix_count,
            coupled_entities: coupled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_fix_commit() {
        assert!(ImpactAnalyzer::is_fix_commit("fix: resolve null pointer"));
        assert!(ImpactAnalyzer::is_fix_commit("Bug in parser handling"));
        assert!(ImpactAnalyzer::is_fix_commit("Hotfix for production crash"));
        assert!(ImpactAnalyzer::is_fix_commit("PATCH: memory leak"));
        assert!(ImpactAnalyzer::is_fix_commit("bugfix: off-by-one error"));
        assert!(!ImpactAnalyzer::is_fix_commit("feat: add new endpoint"));
        assert!(!ImpactAnalyzer::is_fix_commit("refactor: clean up imports"));
        assert!(!ImpactAnalyzer::is_fix_commit("docs: update README"));
        // Should not match partial words like "prefix" or "suffix"
        assert!(!ImpactAnalyzer::is_fix_commit("Add prefix handling logic"));
    }
}

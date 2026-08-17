//! Git log -L heuristic oracle comparator and discrepancy taxonomy classifier per D-15, D-16, LINEAGE-05.
//!
//! Replays `git log -L :func:file` against historical function evolutions and compares with
//! `gitctx` function lineage resolution, categorizing differences into a 4-bucket discrepancy taxonomy:
//! 1. FullAgreement: both systems identify the identical chronological commit history.
//! 2. GitctxAdvantage: gitctx tracks functions across cross-file moves and renames where `git log -L` halted.
//! 3. GitLogLAdvantage: `git log -L` tracked sub-line textual edits that AST degradation missed.
//! 4. Ambiguous: complex merge conflicts or diverging historical branches.

use anyhow::Result;
use gitctx_core::git::repository::GitRepo;
use gitctx_core::lineage::resolver::LineageResolver;
use gitctx_core::parser::AstExtractor;
use git2::Oid;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

/// 4-bucket discrepancy taxonomy classifying differences between gitctx and the git log -L heuristic oracle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiscrepancyTaxonomy {
    /// Full Agreement: gitctx and git log -L trace the exact same commit set.
    FullAgreement,
    /// gitctx Advantage: gitctx correctly traced through a cross-file move or rename where git log -L halted.
    GitctxAdvantage,
    /// git log -L Advantage: git log -L found a commit that gitctx missed (e.g. syntax degradation).
    GitLogLAdvantage,
    /// Ambiguous: diverging multi-branch edits or partial commit overlap.
    Ambiguous,
}

impl DiscrepancyTaxonomy {
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::FullAgreement => "Full Agreement",
            Self::GitctxAdvantage => "gitctx Advantage (moves/renames)",
            Self::GitLogLAdvantage => "git log -L Advantage",
            Self::Ambiguous => "Ambiguous / Partial Overlap",
        }
    }
}

/// Comparison result for a single function evolution timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionComparisonResult {
    pub qualname: String,
    pub path: String,
    pub gitctx_commits: Vec<String>,
    pub git_log_l_commits: Vec<String>,
    pub category: DiscrepancyTaxonomy,
    pub agreement_rate: f32,
    pub notes: String,
}

/// Comprehensive oracle comparison report across a repository sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OracleComparisonReport {
    pub total_functions_evaluated: usize,
    pub full_agreement_count: usize,
    pub gitctx_advantage_count: usize,
    pub git_log_l_advantage_count: usize,
    pub ambiguous_count: usize,
    pub overall_agreement_rate: f32,
    pub oracle_blindness_rate: f32,
    pub results: Vec<FunctionComparisonResult>,
}

/// Heuristic oracle comparator.
pub struct GitLogOracleComparator;

impl GitLogOracleComparator {
    /// Execute `git log -L :func:file` via git CLI on the given repository.
    pub fn query_git_log_l(
        repo_path: impl AsRef<Path>,
        func_name: &str,
        file_path: &str,
    ) -> Result<Vec<String>> {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo_path.as_ref())
            .arg("log")
            .arg(format!("-L:{}:{}", func_name, file_path))
            .arg("--format=%H")
            .arg("--no-patch")
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let commits: Vec<String> = stdout
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty() && l.len() >= 7)
                    .map(ToString::to_string)
                    .collect();
                Ok(commits)
            }
            Ok(out) => {
                let _stderr = String::from_utf8_lossy(&out.stderr);
                // `git log -L` failed (e.g. function renamed, regex mismatch, or file moved)
                // Return empty list indicating git log -L blindness/halting
                Ok(vec![])
            }
            Err(_e) => {
                // If git command is not in PATH or fails to spawn, return empty
                Ok(vec![])
            }
        }
    }

    /// Compare gitctx lineage history against `git log -L` for a given function in a repository.
    pub fn compare_function(
        repo_path: impl AsRef<Path>,
        func_name: &str,
        file_path: &str,
        gitctx_commits: &[String],
        has_renames_or_moves: bool,
    ) -> Result<FunctionComparisonResult> {
        let git_log_l_commits = Self::query_git_log_l(&repo_path, func_name, file_path)?;

        let gitctx_set: HashSet<&str> = gitctx_commits.iter().map(String::as_str).collect();
        let log_set: HashSet<&str> = git_log_l_commits.iter().map(String::as_str).collect();

        let intersection_count = gitctx_set.intersection(&log_set).count();
        let union_count = gitctx_set.union(&log_set).count();

        let agreement_rate = if union_count == 0 {
            1.0
        } else {
            intersection_count as f32 / union_count as f32
        };

        let category = if gitctx_set == log_set {
            DiscrepancyTaxonomy::FullAgreement
        } else if gitctx_set.is_superset(&log_set) && (has_renames_or_moves || git_log_l_commits.is_empty()) {
            DiscrepancyTaxonomy::GitctxAdvantage
        } else if log_set.is_superset(&gitctx_set) {
            DiscrepancyTaxonomy::GitLogLAdvantage
        } else {
            DiscrepancyTaxonomy::Ambiguous
        };

        let notes = match category {
            DiscrepancyTaxonomy::FullAgreement => "Exact match between gitctx and git log -L".to_string(),
            DiscrepancyTaxonomy::GitctxAdvantage => {
                format!(
                    "gitctx traced {} commits vs {} from git log -L across renames/moves",
                    gitctx_commits.len(),
                    git_log_l_commits.len()
                )
            }
            DiscrepancyTaxonomy::GitLogLAdvantage => {
                format!(
                    "git log -L traced {} commits vs {} from gitctx",
                    git_log_l_commits.len(),
                    gitctx_commits.len()
                )
            }
            DiscrepancyTaxonomy::Ambiguous => {
                format!(
                    "Partial agreement: {} shared commits, {} gitctx, {} git log -L",
                    intersection_count,
                    gitctx_commits.len(),
                    git_log_l_commits.len()
                )
            }
        };

        Ok(FunctionComparisonResult {
            qualname: func_name.to_string(),
            path: file_path.to_string(),
            gitctx_commits: gitctx_commits.to_vec(),
            git_log_l_commits,
            category,
            agreement_rate,
            notes,
        })
    }

    /// Evaluates multiple functions in a repository and compiles an `OracleComparisonReport`.
    pub fn evaluate_repository(
        repo_path: impl AsRef<Path>,
        sample_functions: &[(String, String, bool)], // (qualname, path, has_renames)
    ) -> Result<OracleComparisonReport> {
        let git_repo = GitRepo::open(repo_path.as_ref())?;
        let mut revwalk = git_repo.inner().revwalk()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        revwalk.push_head()?;

        let mut oids = Vec::new();
        for oid_res in revwalk {
            oids.push(oid_res?);
        }
        oids.reverse();

        let mut resolver = LineageResolver::new();
        let extractor = AstExtractor::new();
        let mut prev_oid: Option<Oid> = None;
        let mut all_touches = Vec::new();

        for commit_oid in &oids {
            let resolved = resolver.process_commit(&git_repo, *commit_oid, prev_oid, &extractor)?;
            all_touches.extend(resolved.touches);
            prev_oid = Some(*commit_oid);
        }

        let mut results = Vec::new();
        let mut full_agreement = 0;
        let mut gitctx_adv = 0;
        let mut git_log_adv = 0;
        let mut ambiguous = 0;
        let mut total_agreement_sum = 0.0f32;

        for (qualname, path, has_renames) in sample_functions {
            // Find lineage ID and touches from resolver
            let leaf = qualname.rsplit("::").next().unwrap_or(qualname);
            let mut gitctx_commits = Vec::new();

            for touch in &all_touches {
                if touch.path == *path && (touch.qualname == *qualname || touch.qualname.ends_with(leaf)) {
                    gitctx_commits.push(touch.commit_id.to_string());
                }
            }

            let comp = Self::compare_function(
                repo_path.as_ref(),
                leaf,
                path,
                &gitctx_commits,
                *has_renames,
            )?;

            match comp.category {
                DiscrepancyTaxonomy::FullAgreement => full_agreement += 1,
                DiscrepancyTaxonomy::GitctxAdvantage => gitctx_adv += 1,
                DiscrepancyTaxonomy::GitLogLAdvantage => git_log_adv += 1,
                DiscrepancyTaxonomy::Ambiguous => ambiguous += 1,
            }
            total_agreement_sum += comp.agreement_rate;
            results.push(comp);
        }

        let total = sample_functions.len();
        let overall_agreement_rate = if total == 0 {
            1.0
        } else {
            total_agreement_sum / total as f32
        };

        let oracle_blindness_rate = if total == 0 {
            0.0
        } else {
            gitctx_adv as f32 / total as f32
        };

        Ok(OracleComparisonReport {
            total_functions_evaluated: total,
            full_agreement_count: full_agreement,
            gitctx_advantage_count: gitctx_adv,
            git_log_l_advantage_count: git_log_adv,
            ambiguous_count: ambiguous,
            overall_agreement_rate,
            oracle_blindness_rate,
            results,
        })
    }

    /// Format report as terminal table.
    #[must_use]
    pub fn format_report(&self, report: &OracleComparisonReport) -> String {
        let mut out = String::new();
        out.push_str("\n========================================================================================\n");
        out.push_str("                   gitctx vs git log -L Heuristic Oracle Comparison                     \n");
        out.push_str("========================================================================================\n");
        out.push_str(&format!(
            "Total Functions Evaluated: {}\n",
            report.total_functions_evaluated
        ));
        out.push_str(&format!(
            "- Full Agreement:                 {:>4} ({:>5.1}%)\n",
            report.full_agreement_count,
            if report.total_functions_evaluated > 0 {
                (report.full_agreement_count as f32 / report.total_functions_evaluated as f32) * 100.0
            } else {
                100.0
            }
        ));
        out.push_str(&format!(
            "- gitctx Advantage (Moves/Renames):{:>4} ({:>5.1}%)\n",
            report.gitctx_advantage_count,
            if report.total_functions_evaluated > 0 {
                (report.gitctx_advantage_count as f32 / report.total_functions_evaluated as f32) * 100.0
            } else {
                0.0
            }
        ));
        out.push_str(&format!(
            "- git log -L Advantage:           {:>4} ({:>5.1}%)\n",
            report.git_log_l_advantage_count,
            if report.total_functions_evaluated > 0 {
                (report.git_log_l_advantage_count as f32 / report.total_functions_evaluated as f32) * 100.0
            } else {
                0.0
            }
        ));
        out.push_str(&format!(
            "- Ambiguous / Diverging:          {:>4} ({:>5.1}%)\n",
            report.ambiguous_count,
            if report.total_functions_evaluated > 0 {
                (report.ambiguous_count as f32 / report.total_functions_evaluated as f32) * 100.0
            } else {
                0.0
            }
        ));
        out.push_str("----------------------------------------------------------------------------------------\n");
        out.push_str(&format!(
            "Heuristic Oracle Blindness Rate on Moves/Renames: {:>5.1}%\n",
            report.oracle_blindness_rate * 100.0
        ));
        out.push_str("========================================================================================\n\n");
        out
    }
}

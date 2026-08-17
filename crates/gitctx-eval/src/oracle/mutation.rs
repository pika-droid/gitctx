//! Synthetic mutation benchmark runner and accuracy evaluator across M1-M10 per LINEAGE-04, D-08, D-13.
//!
//! Evaluates link-level Precision, Recall, and F1 across all 10 synthetic mutation classes (M1-M10),
//! M8 decoy rejection rate, and multi-commit transitive chain recovery at depths D=2, 5, 10.

use anyhow::{Context, Result};
use git2::Oid;
use gitctx_core::git::repository::GitRepo;
use gitctx_core::lineage::dag::DerivationKind;
use gitctx_core::lineage::matcher::MatchThresholds;
use gitctx_core::lineage::resolver::LineageResolver;
use gitctx_core::model::LineageId;
use gitctx_core::parser::AstExtractor;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::fixtures::mkrepo::RepoBuilder;
use crate::mutator::engine::MutationEngine;
use crate::mutator::manifest::{GroundTruthRelation, LineageGroundTruthManifest, LineageLink};
use crate::mutator::transformations::MutationClass;

/// Accuracy metrics for a specific mutation class.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassMetrics {
    pub mutation_class: MutationClass,
    pub true_positives: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
    pub precision: f32,
    pub recall: f32,
    pub f1: f32,
    pub total_samples: usize,
    pub decoy_rejection_rate: Option<f32>,
}

impl ClassMetrics {
    /// Compute derived Precision, Recall, and F1 scores.
    #[must_use]
    pub fn new(
        mutation_class: MutationClass,
        tp: usize,
        fp: usize,
        fn_count: usize,
        decoy_rejections: Option<(usize, usize)>,
    ) -> Self {
        let precision = if tp + fp == 0 {
            1.0
        } else {
            tp as f32 / (tp + fp) as f32
        };

        let recall = if tp + fn_count == 0 {
            1.0
        } else {
            tp as f32 / (tp + fn_count) as f32
        };

        let f1 = if precision + recall == 0.0 {
            0.0
        } else {
            (2.0 * precision * recall) / (precision + recall)
        };

        let decoy_rejection_rate = decoy_rejections.map(|(rej, total)| {
            if total == 0 {
                1.0
            } else {
                rej as f32 / total as f32
            }
        });

        Self {
            mutation_class,
            true_positives: tp,
            false_positives: fp,
            false_negatives: fn_count,
            precision,
            recall,
            f1,
            total_samples: tp + fn_count,
            decoy_rejection_rate,
        }
    }
}

/// Recovery rate metric for multi-commit sequential mutation chains.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChainMetric {
    pub depth: usize,
    pub total_chains: usize,
    pub successful_chains: usize,
    pub recovery_rate: f32,
}

/// Comprehensive benchmark report across all mutation classes and chain depths.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub per_class: HashMap<MutationClass, ClassMetrics>,
    pub overall_precision: f32,
    pub overall_recall: f32,
    pub overall_f1: f32,
    pub m8_decoy_rejection_rate: f32,
    pub chain_metrics: HashMap<usize, ChainMetric>,
}

/// Evaluation result for a single link verification.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkEvaluationResult {
    pub link: LineageLink,
    pub is_correct: bool,
    pub detected_source_lineage: Option<LineageId>,
    pub detected_target_lineage: Option<LineageId>,
    pub notes: String,
}

/// Synthetic mutation evaluation runner per LINEAGE-04.
pub struct MutationBenchmarkRunner {
    thresholds: MatchThresholds,
}

impl Default for MutationBenchmarkRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl MutationBenchmarkRunner {
    /// Create a benchmark runner with default match thresholds.
    #[must_use]
    pub fn new() -> Self {
        Self::with_thresholds(MatchThresholds::default())
    }

    /// Create a benchmark runner with customized match thresholds.
    #[must_use]
    pub fn with_thresholds(thresholds: MatchThresholds) -> Self {
        Self { thresholds }
    }

    /// Access configured thresholds.
    #[must_use]
    pub fn thresholds(&self) -> &MatchThresholds {
        &self.thresholds
    }

    /// Evaluates a synthetic repository against its ground-truth manifest.
    pub fn evaluate_repo(
        &self,
        builder: &RepoBuilder,
        manifest: &LineageGroundTruthManifest,
    ) -> Result<Vec<LinkEvaluationResult>> {
        let git_repo = GitRepo::open(builder.path())
            .with_context(|| format!("Failed to open repo at {}", builder.path().display()))?;

        // Collect all commits from root to HEAD in chronological order
        let mut revwalk = git_repo.inner().revwalk()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        revwalk.push_head()?;

        let mut oids = Vec::new();
        for oid_res in revwalk {
            oids.push(oid_res?);
        }
        oids.reverse(); // Chronological: [root, commit_1, ..., HEAD]

        let mut resolver = LineageResolver::with_thresholds(self.thresholds);
        let extractor = AstExtractor::new();

        // Maps commit_sha -> Map of (path, qualname) -> LineageId
        let mut commit_symbol_lineages: HashMap<String, HashMap<(String, String), LineageId>> =
            HashMap::new();

        let mut prev_oid: Option<Oid> = None;

        for commit_oid in &oids {
            let commit_sha = commit_oid.to_string();
            let _resolved = resolver
                .process_commit(&git_repo, *commit_oid, prev_oid, &extractor)
                .with_context(|| format!("Resolver failed on commit {commit_sha}"))?;

            let mut symbol_map = HashMap::new();
            for (path, syms) in resolver.active_symbols() {
                for sym in syms {
                    if let Some(id) = sym.lineage_id {
                        symbol_map.insert((path.replace('\\', "/"), sym.qualname.clone()), id);
                    }
                }
            }
            commit_symbol_lineages.insert(commit_sha, symbol_map);
            prev_oid = Some(*commit_oid);
        }

        // Helper to find symbol lineage supporting both raw and module-prefixed qualnames
        let find_lineage = |commit_sha: &str, path: &str, qualname: &str| -> Option<LineageId> {
            let syms = commit_symbol_lineages.get(commit_sha)?;
            let norm_path = path.replace('\\', "/");
            if let Some(&id) = syms.get(&(norm_path.clone(), qualname.to_string())) {
                return Some(id);
            }
            let leaf = qualname.rsplit('.').next().unwrap_or(qualname);
            let leaf_colon = qualname.rsplit("::").next().unwrap_or(qualname);
            for ((p, q), &id) in syms {
                let p_norm = p.replace('\\', "/");
                if (p_norm == norm_path
                    || p_norm.ends_with(&norm_path)
                    || norm_path.ends_with(&p_norm))
                    && (q == qualname
                        || q == leaf
                        || q == leaf_colon
                        || qualname.ends_with(q)
                        || q.ends_with(qualname))
                {
                    return Some(id);
                }
            }
            None
        };

        // Evaluate each ground truth link
        let mut results = Vec::with_capacity(manifest.links.len());

        for link in &manifest.links {
            // Find commit index in chronological order
            let current_commit_idx = oids
                .iter()
                .position(|o| o.to_string() == link.commit_sha)
                .unwrap_or(oids.len().saturating_sub(1));

            // Target lineage is in link.commit_sha
            let target_lineage =
                find_lineage(&link.commit_sha, &link.target.path, &link.target.qualname);

            // Source lineage is from the parent commit (commit before current_commit_idx)
            let parent_commit_sha = if current_commit_idx > 0 {
                oids.get(current_commit_idx - 1).map(ToString::to_string)
            } else {
                None
            };

            let source_lineage = parent_commit_sha
                .and_then(|p_sha| find_lineage(&p_sha, &link.source.path, &link.source.qualname));

            let (is_correct, notes) = match link.relation {
                GroundTruthRelation::Exact
                | GroundTruthRelation::Renamed
                | GroundTruthRelation::Moved => {
                    if let (Some(src_id), Some(tgt_id)) = (source_lineage, target_lineage) {
                        if src_id == tgt_id {
                            (true, format!("Lineage preserved: {src_id}"))
                        } else {
                            (
                                false,
                                format!("Lineage mismatch: expected {src_id}, got {tgt_id}"),
                            )
                        }
                    } else {
                        (
                            false,
                            format!(
                                "Missing lineage: source={source_lineage:?}, target={target_lineage:?}"
                            ),
                        )
                    }
                }
                GroundTruthRelation::Decoy => {
                    // Decoy must reject the old lineage and allocate a new one (or have no lineage link)
                    if let (Some(src_id), Some(tgt_id)) = (source_lineage, target_lineage) {
                        if src_id == tgt_id {
                            (
                                false,
                                format!("Decoy falsely accepted: same lineage {src_id}"),
                            )
                        } else {
                            (
                                true,
                                format!("Decoy successfully rejected: {src_id} != {tgt_id}"),
                            )
                        }
                    } else if target_lineage.is_some() {
                        (true, "Decoy allocated new lineage".to_string())
                    } else {
                        (false, "Decoy target symbol missing".to_string())
                    }
                }
                GroundTruthRelation::SplitFrom => {
                    // Check if DAG contains a Split derivation edge linking source -> target
                    let has_split_edge =
                        if let (Some(src_id), Some(tgt_id)) = (source_lineage, target_lineage) {
                            resolver.dag().edges().iter().any(|edge| {
                                edge.parent_lineage_id == src_id
                                    && edge.child_lineage_id == tgt_id
                                    && edge.kind == DerivationKind::Split
                            })
                        } else {
                            false
                        };

                    if has_split_edge {
                        (true, "Split derivation edge detected".to_string())
                    } else {
                        (
                            false,
                            format!(
                                "Missing Split edge from source {source_lineage:?} to target {target_lineage:?}. Existing DAG edges: {:?}",
                                resolver.dag().edges()
                            ),
                        )
                    }
                }
                GroundTruthRelation::Copied => {
                    // Check if DAG contains a Copy derivation edge linking source -> target
                    let has_copy_edge =
                        if let (Some(src_id), Some(tgt_id)) = (source_lineage, target_lineage) {
                            resolver.dag().edges().iter().any(|edge| {
                                edge.parent_lineage_id == src_id
                                    && edge.child_lineage_id == tgt_id
                                    && edge.kind == DerivationKind::Copy
                            })
                        } else {
                            false
                        };

                    if has_copy_edge {
                        (true, "Copy derivation edge detected".to_string())
                    } else {
                        (
                            false,
                            format!(
                                "Missing Copy edge from source {source_lineage:?} to target {target_lineage:?}"
                            ),
                        )
                    }
                }
            };

            results.push(LinkEvaluationResult {
                link: link.clone(),
                is_correct,
                detected_source_lineage: source_lineage,
                detected_target_lineage: target_lineage,
                notes,
            });
        }

        Ok(results)
    }

    /// Evaluates a suite of single-class and multi-class repositories across M1-M10 and chain depths.
    pub fn run_benchmark_suite(&self) -> Result<BenchmarkReport> {
        let mut class_results: HashMap<MutationClass, (usize, usize, usize, usize, usize)> =
            HashMap::new();

        // 1. Evaluate all 10 mutation classes individually (depth=1)
        for &class in MutationClass::all() {
            let mut builder = RepoBuilder::new()?;
            let link = MutationEngine::apply_mutation(&mut builder, class)
                .map_err(|e| anyhow::anyhow!("Mutation application failed: {e}"))?;

            let mut manifest = LineageGroundTruthManifest::new(1);
            manifest.add_link(link);

            let eval_results = self.evaluate_repo(&builder, &manifest)?;

            for res in eval_results {
                let entry = class_results
                    .entry(res.link.mutation_class)
                    .or_insert((0, 0, 0, 0, 0)); // (tp, fp, fn, decoy_rej, decoy_total)

                if res.link.mutation_class == MutationClass::M8_DecoyDeleteAdd {
                    entry.4 += 1;
                    if res.is_correct {
                        entry.0 += 1; // TP
                        entry.3 += 1; // Decoy rejected
                    } else {
                        entry.1 += 1; // FP
                    }
                } else if res.is_correct {
                    entry.0 += 1; // TP
                } else {
                    entry.2 += 1; // FN
                }
            }
        }

        // Build per-class metrics
        let mut per_class = HashMap::new();
        let mut total_tp = 0;
        let mut total_fp = 0;
        let mut total_fn = 0;
        let mut m8_rej = 0;
        let mut m8_total = 0;

        for &class in MutationClass::all() {
            let (tp, fp, fn_count, decoy_rej, decoy_total) = class_results
                .get(&class)
                .copied()
                .unwrap_or((0, 0, 0, 0, 0));

            total_tp += tp;
            total_fp += fp;
            total_fn += fn_count;

            let decoy_info = if class == MutationClass::M8_DecoyDeleteAdd {
                m8_rej += decoy_rej;
                m8_total += decoy_total;
                Some((decoy_rej, decoy_total))
            } else {
                None
            };

            let metrics = ClassMetrics::new(class, tp, fp, fn_count, decoy_info);
            per_class.insert(class, metrics);
        }

        // 2. Evaluate composite chains at depths D=2, 5, 10
        let chain_metrics = self.evaluate_chain_depths(&[2, 5, 10])?;

        let overall_precision = if total_tp + total_fp == 0 {
            1.0
        } else {
            total_tp as f32 / (total_tp + total_fp) as f32
        };

        let overall_recall = if total_tp + total_fn == 0 {
            1.0
        } else {
            total_tp as f32 / (total_tp + total_fn) as f32
        };

        let overall_f1 = if overall_precision + overall_recall == 0.0 {
            0.0
        } else {
            (2.0 * overall_precision * overall_recall) / (overall_precision + overall_recall)
        };

        let m8_decoy_rejection_rate = if m8_total == 0 {
            1.0
        } else {
            m8_rej as f32 / m8_total as f32
        };

        Ok(BenchmarkReport {
            per_class,
            overall_precision,
            overall_recall,
            overall_f1,
            m8_decoy_rejection_rate,
            chain_metrics,
        })
    }

    /// Evaluates multi-commit mutation chains at specified depths D.
    pub fn evaluate_chain_depths(&self, depths: &[usize]) -> Result<HashMap<usize, ChainMetric>> {
        let mut results = HashMap::new();

        for &depth in depths {
            let sample_chains = Self::generate_sample_chains(depth);
            let total_chains = sample_chains.len();
            let mut successful_chains = 0;

            for chain in &sample_chains {
                let mut builder = RepoBuilder::new()?;
                let manifest = MutationEngine::apply_chain(&mut builder, chain)
                    .map_err(|e| anyhow::anyhow!("Apply chain failed: {e}"))?;

                let eval_results = self.evaluate_repo(&builder, &manifest)?;
                let all_correct = eval_results.iter().all(|r| r.is_correct);
                if all_correct {
                    successful_chains += 1;
                }
            }

            let recovery_rate = if total_chains == 0 {
                1.0
            } else {
                successful_chains as f32 / total_chains as f32
            };

            results.insert(
                depth,
                ChainMetric {
                    depth,
                    total_chains,
                    successful_chains,
                    recovery_rate,
                },
            );
        }

        Ok(results)
    }

    /// Generate deterministic representative mutation chains for a given depth.
    fn generate_sample_chains(depth: usize) -> Vec<Vec<MutationClass>> {
        match depth {
            2 => vec![
                vec![
                    MutationClass::M1_InPlaceRename,
                    MutationClass::M4_SignatureChange,
                ],
                vec![
                    MutationClass::M5_Reformatting,
                    MutationClass::M6_LocalVarRename,
                ],
                vec![
                    MutationClass::M4_SignatureChange,
                    MutationClass::M2_CrossFileMove,
                ],
                vec![
                    MutationClass::M6_LocalVarRename,
                    MutationClass::M1_InPlaceRename,
                ],
            ],
            5 => vec![
                vec![
                    MutationClass::M1_InPlaceRename,
                    MutationClass::M5_Reformatting,
                    MutationClass::M6_LocalVarRename,
                    MutationClass::M4_SignatureChange,
                    MutationClass::M2_CrossFileMove,
                ],
                vec![
                    MutationClass::M5_Reformatting,
                    MutationClass::M6_LocalVarRename,
                    MutationClass::M1_InPlaceRename,
                    MutationClass::M4_SignatureChange,
                    MutationClass::M9_DirectoryMove,
                ],
            ],
            10 => vec![vec![
                MutationClass::M1_InPlaceRename,
                MutationClass::M5_Reformatting,
                MutationClass::M6_LocalVarRename,
                MutationClass::M4_SignatureChange,
                MutationClass::M7_FunctionSplit,
                MutationClass::M10_FunctionCopy,
                MutationClass::M8_DecoyDeleteAdd,
                MutationClass::M2_CrossFileMove,
                MutationClass::M3_MoveAndRename,
                MutationClass::M9_DirectoryMove,
            ]],
            _ => vec![vec![MutationClass::M1_InPlaceRename; depth]],
        }
    }

    /// Format benchmark report as terminal text table.
    #[must_use]
    pub fn format_terminal_table(&self, report: &BenchmarkReport) -> String {
        let mut out = String::new();
        out.push_str("\n========================================================================================\n");
        out.push_str("                       GitCtx Function Lineage Mutation Benchmark                       \n");
        out.push_str("========================================================================================\n");
        out.push_str(&format!(
            "{:<6} | {:<24} | {:>4} | {:>4} | {:>4} | {:>7} | {:>7} | {:>7} |\n",
            "Class", "Description", "TP", "FP", "FN", "Prec", "Recall", "F1"
        ));
        out.push_str("----------------------------------------------------------------------------------------\n");

        for &class in MutationClass::all() {
            let label = class.code();
            let name = class.description();
            if let Some(m) = report.per_class.get(&class) {
                out.push_str(&format!(
                    "{:<6} | {:<24} | {:>4} | {:>4} | {:>4} | {:>6.1}% | {:>6.1}% | {:>6.1}% |\n",
                    label,
                    name,
                    m.true_positives,
                    m.false_positives,
                    m.false_negatives,
                    m.precision * 100.0,
                    m.recall * 100.0,
                    m.f1 * 100.0,
                ));
            }
        }

        out.push_str("----------------------------------------------------------------------------------------\n");
        out.push_str(&format!(
            "Overall Link-Level Accuracy: Precision: {:>5.1}% | Recall: {:>5.1}% | F1: {:>5.1}%\n",
            report.overall_precision * 100.0,
            report.overall_recall * 100.0,
            report.overall_f1 * 100.0
        ));
        out.push_str(&format!(
            "M8 Decoy Rejection Rate:    {:>5.1}% (Target: >= 95.0%)\n",
            report.m8_decoy_rejection_rate * 100.0
        ));
        out.push_str("----------------------------------------------------------------------------------------\n");
        out.push_str("Sequential Chain Recovery Rates:\n");
        for depth in [2, 5, 10] {
            if let Some(c) = report.chain_metrics.get(&depth) {
                out.push_str(&format!(
                    "  Depth D={:<2}: {:>3}/{} chains fully recovered ({:>5.1}%)\n",
                    depth,
                    c.successful_chains,
                    c.total_chains,
                    c.recovery_rate * 100.0
                ));
            }
        }
        out.push_str("========================================================================================\n\n");
        out
    }

    /// Format benchmark report as GitHub-Flavored Markdown table.
    #[must_use]
    pub fn format_markdown_table(&self, report: &BenchmarkReport) -> String {
        let mut out = String::new();
        out.push_str("### Synthetic Mutation Lineage Accuracy (M1-M10)\n\n");
        out.push_str("| Class | Transformation | TP | FP | FN | Precision | Recall | F1 Score |\n");
        out.push_str("| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: |\n");

        for &class in MutationClass::all() {
            let label = class.code();
            let name = class.description();
            if let Some(m) = report.per_class.get(&class) {
                out.push_str(&format!(
                    "| `{label}` | {name} | {tp} | {fp} | {fn_count} | {p:.1}% | {r:.1}% | **{f1:.2}** |\n",
                    label = label,
                    name = name,
                    tp = m.true_positives,
                    fp = m.false_positives,
                    fn_count = m.false_negatives,
                    p = m.precision * 100.0,
                    r = m.recall * 100.0,
                    f1 = m.f1
                ));
            }
        }

        out.push_str("\n#### Summary Performance Metrics\n\n");
        out.push_str(&format!(
            "- **Overall Link F1 Score:** `{:.3}`\n",
            report.overall_f1
        ));
        out.push_str(&format!(
            "- **M8 Decoy Rejection Rate:** `{:.1}%` (Benchmark requirement: $\\ge 95\\%$)\n",
            report.m8_decoy_rejection_rate * 100.0
        ));
        out.push_str("\n#### Transitive Chain Recovery\n\n");
        out.push_str("| Chain Depth | Total Chains | Successful Chains | Full Recovery Rate |\n");
        out.push_str("| :--- | :---: | :---: | :---: |\n");
        for depth in [2, 5, 10] {
            if let Some(c) = report.chain_metrics.get(&depth) {
                out.push_str(&format!(
                    "| Depth $D={}$ | {} | {} | **{:.1}%** |\n",
                    depth,
                    c.total_chains,
                    c.successful_chains,
                    c.recovery_rate * 100.0
                ));
            }
        }
        out
    }
}

//! Candidate match generation and greedy maximum-weight bipartite matching solver.
//!
//! Implements multi-tier candidate generation across Tier 1 (ExactKey with decoy check),
//! Tier 2 (SameFileMinHash), Tier 3 (GitRename), Tier 4 (CrossFileMinHash), and deterministic
//! greedy bipartite conflict resolution per LINEAGE-02, LINEAGE-03, D-05, D-06, D-07, D-08.

use crate::lineage::minhash::MinHashFingerprint;
use crate::model::{LineageId, ParseStatus};
use crate::parser::LanguageId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Match tier priority for candidate lineage edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchTier {
    /// Tier 1: Exact `(path, qualname)` match gated by decoy check.
    ExactKey,
    /// Tier 3: Git file rename mapping with exact qualname.
    GitRenameExact,
    /// Tier 2: Same file MinHash fingerprint similarity >= tau_same.
    SameFileMinHash,
    /// Tier 3: Git file rename mapping with MinHash fingerprint similarity >= tau_same.
    GitRenameMinHash,
    /// Tier 4: Scoped cross-file MinHash fingerprint similarity >= tau_cross.
    CrossFileMinHash,
}

impl MatchTier {
    /// Canonical tier priority (lower value = higher priority).
    #[must_use]
    pub const fn priority(&self) -> u8 {
        match self {
            Self::ExactKey => 0,
            Self::GitRenameExact => 1,
            Self::SameFileMinHash => 2,
            Self::GitRenameMinHash => 3,
            Self::CrossFileMinHash => 4,
        }
    }

    /// Return the string rule name for Citation / FunctionTouch metadata.
    #[must_use]
    pub const fn as_rule_name(&self) -> &'static str {
        match self {
            Self::ExactKey => "exact_key",
            Self::GitRenameExact => "git_rename_exact",
            Self::SameFileMinHash => "same_file_fingerprint",
            Self::GitRenameMinHash => "git_rename_fingerprint",
            Self::CrossFileMinHash => "cross_file_fingerprint",
        }
    }
}

/// Similarity thresholds for multi-tier matching and decoy rejection per D-07, D-08.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MatchThresholds {
    /// Similarity threshold for in-file modifications and renames (default 0.65).
    pub tau_same: f32,
    /// Similarity threshold for scoped cross-file moves (default 0.75).
    pub tau_cross: f32,
    /// Decoy rejection threshold on exact key matches (default 0.25).
    pub tau_decoy: f32,
    /// Split derivation token overlap threshold (default 0.40).
    pub tau_split: f32,
}

impl Default for MatchThresholds {
    fn default() -> Self {
        Self {
            tau_same: 0.65,
            tau_cross: 0.75,
            tau_decoy: 0.25,
            tau_split: 0.40,
        }
    }
}

/// Full descriptor of an active or extracted function symbol for matching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolDescriptor {
    pub lineage_id: Option<LineageId>,
    pub path: String,
    pub qualname: String,
    pub language: LanguageId,
    pub fingerprint: MinHashFingerprint,
    pub token_count: usize,
    pub is_short: bool,
    pub signature: String,
    pub docstring: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub parse_status: ParseStatus,
}

/// A candidate match between a parent symbol and a child symbol.
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateMatch {
    pub parent_index: usize,
    pub child_index: usize,
    pub tier: MatchTier,
    pub score: f32,
    pub old_path: String,
    pub old_qualname: String,
    pub new_path: String,
    pub new_qualname: String,
}

impl CandidateMatch {
    /// Compare candidates for deterministic greedy bipartite matching per D-06.
    ///
    /// Primary sort: score descending.
    /// Secondary sort: tier priority (ExactKey > GitRenameExact > ...).
    /// Tie-breaking: lexical `(old_path, old_qualname, new_path, new_qualname)`.
    #[must_use]
    pub fn cmp_deterministic(&self, other: &Self) -> std::cmp::Ordering {
        other
            .score
            .total_cmp(&self.score)
            .then_with(|| self.tier.priority().cmp(&other.tier.priority()))
            .then_with(|| self.old_path.cmp(&other.old_path))
            .then_with(|| self.old_qualname.cmp(&other.old_qualname))
            .then_with(|| self.new_path.cmp(&other.new_path))
            .then_with(|| self.new_qualname.cmp(&other.new_qualname))
            .then_with(|| self.parent_index.cmp(&other.parent_index))
            .then_with(|| self.child_index.cmp(&other.child_index))
    }
}

/// Result of greedy bipartite conflict resolution.
#[derive(Debug, Clone, PartialEq)]
pub struct BipartiteSolution {
    pub matches: Vec<CandidateMatch>,
    pub unmatched_parents: Vec<usize>,
    pub unmatched_children: Vec<usize>,
}

/// Candidate match generator implementing multi-tier matching rules.
pub struct CandidateGenerator;

impl CandidateGenerator {
    /// Generate all candidate matches between parent and child symbols.
    ///
    /// Evaluates:
    /// - Tier 1: Exact `(path, qualname)` match with decoy rejection (D-08).
    /// - Tier 2: Same-file MinHash match with length threshold gating (D-03, D-07).
    /// - Tier 3: Git rename mapping match with exact or MinHash match.
    /// - Tier 4: Cross-file MinHash match scoped to commit files and matching language (D-05, D-07).
    #[must_use]
    pub fn generate_candidates(
        parents: &[SymbolDescriptor],
        children: &[SymbolDescriptor],
        renamed_paths: &HashMap<PathBuf, PathBuf>,
        thresholds: &MatchThresholds,
    ) -> Vec<CandidateMatch> {
        let mut candidates = Vec::new();

        for (p_idx, p) in parents.iter().enumerate() {
            let p_path_buf = PathBuf::from(&p.path);
            let rename_target = renamed_paths.get(&p_path_buf);

            for (c_idx, c) in children.iter().enumerate() {
                let is_same_path = p.path == c.path;
                let is_same_name = p.qualname == c.qualname;
                let is_renamed_path = rename_target
                    .map(|target| target == Path::new(&c.path))
                    .unwrap_or(false);

                // Tier 1: Exact Key (same path, same qualname)
                if is_same_path && is_same_name {
                    let sim = p.fingerprint.similarity(&c.fingerprint);
                    // Decoy Check (D-08): if sim < tau_decoy and AST structure changed, reject
                    let is_exact_identical =
                        p.fingerprint == c.fingerprint && p.signature == c.signature;
                    if sim >= thresholds.tau_decoy || is_exact_identical {
                        candidates.push(CandidateMatch {
                            parent_index: p_idx,
                            child_index: c_idx,
                            tier: MatchTier::ExactKey,
                            score: 1.0,
                            old_path: p.path.clone(),
                            old_qualname: p.qualname.clone(),
                            new_path: c.path.clone(),
                            new_qualname: c.qualname.clone(),
                        });
                        continue;
                    }
                    // If decoy rejected, do not add Tier 1. Fall through for other tiers if applicable.
                }

                // Tier 3: Git Rename / Copy Mapping
                if is_renamed_path {
                    if is_same_name {
                        candidates.push(CandidateMatch {
                            parent_index: p_idx,
                            child_index: c_idx,
                            tier: MatchTier::GitRenameExact,
                            score: 0.95,
                            old_path: p.path.clone(),
                            old_qualname: p.qualname.clone(),
                            new_path: c.path.clone(),
                            new_qualname: c.qualname.clone(),
                        });
                        continue;
                    } else if !p.is_short && !c.is_short {
                        let sim = p.fingerprint.similarity(&c.fingerprint);
                        if sim >= thresholds.tau_same {
                            candidates.push(CandidateMatch {
                                parent_index: p_idx,
                                child_index: c_idx,
                                tier: MatchTier::GitRenameMinHash,
                                score: sim * 0.95,
                                old_path: p.path.clone(),
                                old_qualname: p.qualname.clone(),
                                new_path: c.path.clone(),
                                new_qualname: c.qualname.clone(),
                            });
                            continue;
                        }
                    }
                }

                // Tier 2: Same-file MinHash (same path, different qualname)
                if is_same_path && !is_same_name && !p.is_short && !c.is_short {
                    let sim = p.fingerprint.similarity(&c.fingerprint);
                    if sim >= thresholds.tau_same {
                        candidates.push(CandidateMatch {
                            parent_index: p_idx,
                            child_index: c_idx,
                            tier: MatchTier::SameFileMinHash,
                            score: sim,
                            old_path: p.path.clone(),
                            old_qualname: p.qualname.clone(),
                            new_path: c.path.clone(),
                            new_qualname: c.qualname.clone(),
                        });
                        continue;
                    }
                }

                // Tier 4: Scoped Cross-File MinHash (different path, same language, length >= 10)
                if !is_same_path
                    && !is_renamed_path
                    && p.language == c.language
                    && !p.is_short
                    && !c.is_short
                {
                    let sim = p.fingerprint.similarity(&c.fingerprint);
                    if sim >= thresholds.tau_cross {
                        candidates.push(CandidateMatch {
                            parent_index: p_idx,
                            child_index: c_idx,
                            tier: MatchTier::CrossFileMinHash,
                            score: sim,
                            old_path: p.path.clone(),
                            old_qualname: p.qualname.clone(),
                            new_path: c.path.clone(),
                            new_qualname: c.qualname.clone(),
                        });
                    }
                }
            }
        }

        candidates
    }
}

/// Greedy maximum-weight bipartite matching solver per D-06.
pub struct BipartiteMatcher;

impl BipartiteMatcher {
    /// Solve the assignment problem greedily with deterministic tie-breaking.
    #[must_use]
    pub fn solve(
        mut candidates: Vec<CandidateMatch>,
        parent_count: usize,
        child_count: usize,
    ) -> BipartiteSolution {
        // Deterministic sort: score DESC, tier DESC, canonical names ASC
        candidates.sort_by(CandidateMatch::cmp_deterministic);

        let mut claimed_parents = HashSet::new();
        let mut claimed_children = HashSet::new();
        let mut matches = Vec::new();

        for candidate in candidates {
            if !claimed_parents.contains(&candidate.parent_index)
                && !claimed_children.contains(&candidate.child_index)
            {
                claimed_parents.insert(candidate.parent_index);
                claimed_children.insert(candidate.child_index);
                matches.push(candidate);
            }
        }

        let unmatched_parents = (0..parent_count)
            .filter(|idx| !claimed_parents.contains(idx))
            .collect();
        let unmatched_children = (0..child_count)
            .filter(|idx| !claimed_children.contains(idx))
            .collect();

        BipartiteSolution {
            matches,
            unmatched_parents,
            unmatched_children,
        }
    }
}

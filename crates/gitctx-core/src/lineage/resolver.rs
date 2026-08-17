//! Multi-tier lineage resolver pipeline per LINEAGE-02, LINEAGE-03, D-05..D-08, D-11.
//!
//! Tracks function evolutionary identity across sequential commits, allocating atomic `LineageId`s,
//! executing candidate generation and greedy bipartite matching, and emitting `FunctionTouch` records.

use crate::error::{CoreError, Result};
use crate::git::diff::{CommitDiff, DeltaStatus, TreeDiffExtractor};
use crate::git::repository::GitRepo;
use crate::lineage::matcher::{
    BipartiteMatcher, BipartiteSolution, CandidateGenerator, MatchThresholds, SymbolDescriptor,
};
use crate::lineage::minhash::MinHashFingerprint;
use crate::lineage::normalizer::AstNormalizer;
use crate::model::{Citation, CommitId, FunctionTouch, LineageId};
use crate::parser::{AstExtractor, LanguageRegistry, ParsedFileSymbols};
use git2::Oid;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Normalize filesystem paths to forward slashes for cross-platform deterministic matching.
pub fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Enriched touch record containing lineage metadata and match confidence.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedTouch {
    pub touch: FunctionTouch,
    pub match_rule: String,
    pub match_confidence: f32,
}

/// Result of resolving function lineages for a single commit transition.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedCommit {
    pub commit_id: CommitId,
    pub touches: Vec<FunctionTouch>,
    pub citations: Vec<Citation>,
    pub resolved_touches: Vec<ResolvedTouch>,
    pub solution: BipartiteSolution,
    pub parent_symbols_count: usize,
    pub child_symbols_count: usize,
}

/// Multi-tier lineage resolver maintaining state across sequential commits.
pub struct LineageResolver {
    thresholds: MatchThresholds,
    normalizer: AstNormalizer,
    registry: LanguageRegistry,
    next_lineage_id: u64,
    /// Active symbols in the repository indexed by normalized file path.
    active_symbols: HashMap<String, Vec<SymbolDescriptor>>,
    /// Enforces UNIQUE(lineage_id, commit_hash) per D-11.
    seen_touches: HashSet<(LineageId, CommitId)>,
}

impl Default for LineageResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl LineageResolver {
    /// Create a new lineage resolver with default thresholds.
    #[must_use]
    pub fn new() -> Self {
        Self::with_thresholds(MatchThresholds::default())
    }

    /// Create a lineage resolver with custom similarity thresholds.
    #[must_use]
    pub fn with_thresholds(thresholds: MatchThresholds) -> Self {
        Self {
            thresholds,
            normalizer: AstNormalizer::new(),
            registry: LanguageRegistry::new(),
            next_lineage_id: 1,
            active_symbols: HashMap::new(),
            seen_touches: HashSet::new(),
        }
    }


    /// Access current thresholds.
    #[must_use]
    pub fn thresholds(&self) -> &MatchThresholds {
        &self.thresholds
    }

    /// Access active symbols currently tracked in the working state.
    #[must_use]
    pub fn active_symbols(&self) -> &HashMap<String, Vec<SymbolDescriptor>> {
        &self.active_symbols
    }

    /// Return next lineage ID counter value.
    #[must_use]
    pub fn next_lineage_id(&self) -> u64 {
        self.next_lineage_id
    }

    /// Allocate a fresh atomic `LineageId`.
    pub fn alloc_lineage_id(&mut self) -> u64 {
        let id = self.next_lineage_id;
        self.next_lineage_id += 1;
        id
    }

    /// Resolve lineage for a commit given pre-parsed symbols and file contents.
    pub fn resolve_commit(
        &mut self,
        commit_id: &CommitId,
        diff: &CommitDiff,
        new_file_symbols: &HashMap<PathBuf, ParsedFileSymbols>,
        file_contents: &HashMap<PathBuf, Vec<u8>>,
    ) -> Result<ResolvedCommit> {
        let mut touched_old_paths = HashSet::new();
        let mut renamed_paths = HashMap::new();

        for delta in &diff.deltas {
            if let Some(old_p) = &delta.old_path {
                touched_old_paths.insert(normalize_path(old_p));
            }
            if let (Some(old_p), Some(new_p)) = (&delta.old_path, &delta.new_path) {
                if delta.status == DeltaStatus::Renamed || delta.status == DeltaStatus::Copied {
                    renamed_paths.insert(old_p.clone(), new_p.clone());
                }
            }
        }

        // If root commit or empty deltas, all new files are treated as touched
        if diff.deltas.is_empty() {
            for path in new_file_symbols.keys() {
                touched_old_paths.insert(normalize_path(path));
            }
        }

        // 1. Collect parent symbols from all touched old paths
        let mut parent_symbols = Vec::new();
        for old_path in &touched_old_paths {
            if let Some(syms) = self.active_symbols.remove(old_path) {
                parent_symbols.extend(syms);
            }
        }

        // 2. Build child symbols for all newly parsed files in this commit
        let mut child_symbols = Vec::new();
        for (path_buf, parsed_file) in new_file_symbols {
            let norm_path = normalize_path(path_buf);
            let content = file_contents
                .get(path_buf)
                .map(Vec::as_slice)
                .unwrap_or(&[]);

            let language_id = self
                .registry
                .detect(path_buf, Some(content))
                .map(|b| b.language_id())
                .unwrap_or(crate::parser::LanguageId::Rust);

            for sym in &parsed_file.symbols {
                let snippet = if sym.start_byte <= sym.end_byte && sym.end_byte <= content.len() {
                    &content[sym.start_byte..sym.end_byte]
                } else {
                    &[]
                };
                let source_str = std::str::from_utf8(snippet).unwrap_or("");
                let tokens = self.normalizer.normalize(source_str);
                let token_count = tokens.len();
                let (fingerprint, is_short) = MinHashFingerprint::from_tokens(&tokens);

                child_symbols.push(SymbolDescriptor {
                    lineage_id: None,
                    path: norm_path.clone(),
                    qualname: sym.qualname.clone(),
                    language: language_id,
                    fingerprint,
                    token_count,
                    is_short,
                    signature: sym.signature.clone(),
                    docstring: sym.docstring.clone(),
                    start_line: sym.start_line as usize,
                    end_line: sym.end_line as usize,
                    parse_status: parsed_file.status,
                });
            }
        }

        // 3. Multi-tier candidate generation
        let candidates = CandidateGenerator::generate_candidates(
            &parent_symbols,
            &child_symbols,
            &renamed_paths,
            &self.thresholds,
        );

        // 4. Greedy maximum-weight bipartite matching
        let solution = BipartiteMatcher::solve(
            candidates,
            parent_symbols.len(),
            child_symbols.len(),
        );

        // 5. Lineage assignment and match rule attribution
        let mut match_rules: HashMap<usize, (String, f32)> = HashMap::new();

        for m in &solution.matches {
            let parent_id = parent_symbols[m.parent_index]
                .lineage_id
                .expect("parent symbol must have lineage_id");
            child_symbols[m.child_index].lineage_id = Some(parent_id);
            match_rules.insert(
                m.child_index,
                (m.tier.as_rule_name().to_string(), m.score),
            );
        }

        for child_idx in &solution.unmatched_children {
            let new_id = LineageId::new(self.alloc_lineage_id());
            child_symbols[*child_idx].lineage_id = Some(new_id);
            match_rules.insert(*child_idx, ("new_lineage".to_string(), 1.0));
        }

        // 6. Update active symbols with new state
        for child in &child_symbols {
            self.active_symbols
                .entry(child.path.clone())
                .or_default()
                .push(child.clone());
        }

        // 7. Emit FunctionTouch, Citation, and ResolvedTouch records
        let mut touches = Vec::with_capacity(child_symbols.len());
        let mut citations = Vec::with_capacity(child_symbols.len());
        let mut resolved_touches = Vec::with_capacity(child_symbols.len());

        for (c_idx, child) in child_symbols.iter().enumerate() {
            let lineage_id = child.lineage_id.expect("all children must have lineage_id");

            // Enforce UNIQUE(lineage_id, commit_hash) per D-11
            if !self.seen_touches.insert((lineage_id, commit_id.clone())) {
                return Err(CoreError::InvariantViolation(format!(
                    "Duplicate touch for lineage {} in commit {}",
                    lineage_id, commit_id
                )));
            }

            let (match_rule, match_confidence) = match_rules
                .remove(&c_idx)
                .unwrap_or_else(|| ("unknown".to_string(), 0.0));

            let touch = FunctionTouch {
                commit_id: commit_id.clone(),
                lineage_id,
                path: child.path.clone(),
                qualname: child.qualname.clone(),
                signature: child.signature.clone(),
                docstring: child.docstring.clone(),
                start_line: child.start_line,
                end_line: child.end_line,
                parse_status: child.parse_status,
            };

            let citation = Citation {
                commit_hash: commit_id.to_string(),
                path: child.path.clone(),
                start_line: child.start_line,
                end_line: child.end_line,
                match_rule: match_rule.clone(),
                match_confidence,
            };

            resolved_touches.push(ResolvedTouch {
                touch: touch.clone(),
                match_rule,
                match_confidence,
            });
            touches.push(touch);
            citations.push(citation);
        }

        Ok(ResolvedCommit {
            commit_id: commit_id.clone(),
            touches,
            citations,
            resolved_touches,
            solution,
            parent_symbols_count: parent_symbols.len(),
            child_symbols_count: child_symbols.len(),
        })
    }

    /// Process a Git commit directly from a `GitRepo` instance.
    pub fn process_commit(
        &mut self,
        repo: &GitRepo,
        commit_oid: Oid,
        parent_oid: Option<Oid>,
        extractor: &AstExtractor,
    ) -> Result<ResolvedCommit> {
        let diff = TreeDiffExtractor::extract(repo, commit_oid, parent_oid)?;
        let commit_id = CommitId::new(commit_oid.to_string());

        let mut new_file_symbols = HashMap::new();
        let mut file_contents = HashMap::new();

        for delta in &diff.deltas {
            if delta.status == DeltaStatus::Deleted {
                continue;
            }
            if let Some(new_path) = &delta.new_path {
                if let Some(new_oid_hex) = &delta.new_oid {
                    if let Ok(oid) = Oid::from_str(new_oid_hex) {
                        if let Ok(blob) = repo.inner().find_blob(oid) {
                            let content = blob.content().to_vec();
                            let parsed = extractor.parse_blob(new_path, &content, Some(oid))?;
                            new_file_symbols.insert(new_path.clone(), parsed);
                            file_contents.insert(new_path.clone(), content);
                        }
                    }
                }
            }
        }

        self.resolve_commit(&commit_id, &diff, &new_file_symbols, &file_contents)
    }
}

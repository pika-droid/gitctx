//! Multi-tier lineage resolver pipeline per LINEAGE-02, LINEAGE-03, D-05..D-08, D-11.
//!
//! Tracks function evolutionary identity across sequential commits, allocating atomic `LineageId`s,
//! executing candidate generation and greedy bipartite matching, and emitting `FunctionTouch` records.

use crate::error::{CoreError, Result};
use crate::git::diff::{CommitDiff, DeltaStatus, DiffHunk, TreeDiffExtractor};
use crate::git::repository::GitRepo;
use crate::lineage::dag::{DerivationKind, LineageDag, LineageEdge};
use crate::lineage::matcher::{
    BipartiteMatcher, BipartiteSolution, CandidateGenerator, MatchThresholds, SymbolDescriptor,
};
use crate::lineage::minhash::MinHashFingerprint;
use crate::lineage::normalizer::AstNormalizer;
use crate::model::{Citation, CommitId, FunctionTouch, LineageId};
use crate::parser::{AstExtractor, LanguageId, LanguageRegistry, ParsedFileSymbols};
use git2::Oid;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Normalize filesystem paths to forward slashes for cross-platform deterministic matching.
pub fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Compute function arity (parameter count) by parsing the signature.
pub fn compute_arity(sig: &str) -> u32 {
    let Some(start) = sig.find('(') else {
        return 0;
    };
    let rest = &sig[start + 1..];
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_angle = 0usize;
    let mut param_chars = 0usize;
    let mut count = 0u32;

    for ch in rest.chars() {
        match ch {
            '(' => depth_paren += 1,
            ')' if depth_paren > 0 => depth_paren -= 1,
            ')' => {
                if param_chars > 0 {
                    count += 1;
                }
                return count;
            }
            '[' => depth_bracket += 1,
            ']' if depth_bracket > 0 => depth_bracket -= 1,
            '<' => depth_angle += 1,
            '>' if depth_angle > 0 => depth_angle -= 1,
            ',' if depth_paren == 0 && depth_bracket == 0 && depth_angle == 0 => {
                if param_chars > 0 {
                    count += 1;
                    param_chars = 0;
                }
            }
            c if !c.is_whitespace() => {
                param_chars += 1;
            }
            _ => {}
        }
    }
    if param_chars > 0 {
        count += 1;
    }
    count
}

/// Enriched touch record containing lineage metadata and match confidence.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedTouch {
    pub touch: FunctionTouch,
    pub match_rule: String,
    pub match_confidence: f32,
    pub arity: u32,
    pub change_type: String,
    pub signature_before: Option<String>,
    pub fingerprint: Vec<u8>,
    pub diff_hunks: Vec<String>,
}

/// Result of resolving function lineages for a single commit transition.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedCommit {
    pub commit_id: CommitId,
    pub touches: Vec<FunctionTouch>,
    pub citations: Vec<Citation>,
    pub resolved_touches: Vec<ResolvedTouch>,
    pub solution: BipartiteSolution,
    pub derivation_edges: Vec<LineageEdge>,
    pub parent_symbols_count: usize,
    pub child_symbols_count: usize,
}

/// An entry in the LSH band index for copy candidate matching.
#[derive(Debug, Clone)]
pub struct LshBucketEntry {
    pub lineage_id: LineageId,
    pub path: String,
    pub qualname: String,
    pub language: LanguageId,
    pub fingerprint: MinHashFingerprint,
}

/// Locality-Sensitive Hashing (LSH) band inverted index for copy derivation candidates.
/// Partitions 128 MinHash permutations into 16 bands of 8 rows each.
#[derive(Debug, Clone, Default)]
pub struct LshIndex {
    buckets: HashMap<(LanguageId, usize, u64), Vec<LshBucketEntry>>,
}

impl LshIndex {
    /// Create a new empty LSH index.
    pub fn new() -> Self {
        Self {
            buckets: HashMap::new(),
        }
    }

    /// Insert a symbol descriptor into all 16 band buckets.
    pub fn insert(&mut self, sym: &SymbolDescriptor) {
        if sym.is_short {
            return;
        }
        let Some(lineage_id) = sym.lineage_id else {
            return;
        };
        for band in 0..16 {
            let hash = sym.fingerprint.band_hash(band, 8);
            self.buckets
                .entry((sym.language, band, hash))
                .or_default()
                .push(LshBucketEntry {
                    lineage_id,
                    path: sym.path.clone(),
                    qualname: sym.qualname.clone(),
                    language: sym.language,
                    fingerprint: sym.fingerprint,
                });
        }
    }

    /// Remove a symbol descriptor from the band buckets.
    pub fn remove(&mut self, sym: &SymbolDescriptor) {
        if sym.is_short {
            return;
        }
        let Some(lineage_id) = sym.lineage_id else {
            return;
        };
        for band in 0..16 {
            let hash = sym.fingerprint.band_hash(band, 8);
            if let Some(entries) = self.buckets.get_mut(&(sym.language, band, hash)) {
                entries.retain(|e| e.lineage_id != lineage_id);
                if entries.is_empty() {
                    self.buckets.remove(&(sym.language, band, hash));
                }
            }
        }
    }

    /// Retrieve candidates colliding in at least one band bucket.
    pub fn get_candidates(
        &self,
        language: LanguageId,
        fingerprint: &MinHashFingerprint,
    ) -> Vec<&LshBucketEntry> {
        let mut seen = HashSet::new();
        let mut candidates = Vec::new();
        for band in 0..16 {
            let hash = fingerprint.band_hash(band, 8);
            if let Some(entries) = self.buckets.get(&(language, band, hash)) {
                for entry in entries {
                    if seen.insert(entry.lineage_id) {
                        candidates.push(entry);
                    }
                }
            }
        }
        candidates
    }
}

/// Multi-tier lineage resolver maintaining state across sequential commits.
pub struct LineageResolver {
    thresholds: MatchThresholds,
    normalizer: AstNormalizer,
    registry: LanguageRegistry,
    next_lineage_id: u64,
    /// Evolutionary lineage derivation DAG per D-09, D-10, D-11.
    dag: LineageDag,
    /// Active symbols in the repository indexed by normalized file path.
    active_symbols: HashMap<String, Vec<SymbolDescriptor>>,
    /// LSH band index for active symbols across untouched repository files.
    lsh_index: LshIndex,
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
            dag: LineageDag::new(),
            active_symbols: HashMap::new(),
            lsh_index: LshIndex::new(),
            seen_touches: HashSet::new(),
        }
    }

    /// Access current thresholds.
    #[must_use]
    pub fn thresholds(&self) -> &MatchThresholds {
        &self.thresholds
    }

    /// Access the lineage derivation DAG.
    #[must_use]
    pub fn dag(&self) -> &LineageDag {
        &self.dag
    }

    /// Mutable access to the lineage derivation DAG.
    pub fn dag_mut(&mut self) -> &mut LineageDag {
        &mut self.dag
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
                for s in &syms {
                    self.lsh_index.remove(s);
                }
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
                .map_or(crate::parser::LanguageId::Rust, |b| b.language_id());

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
        let solution =
            BipartiteMatcher::solve(candidates, parent_symbols.len(), child_symbols.len());

        // 5. Lineage assignment and match rule attribution
        let mut match_rules: HashMap<usize, (String, f32)> = HashMap::new();
        let mut matched_parents: HashMap<usize, usize> = HashMap::new();

        for m in &solution.matches {
            if let Some(parent) = parent_symbols.get(m.parent_index) {
                let parent_id = parent
                    .lineage_id
                    .expect("parent symbol must have lineage_id");
                child_symbols[m.child_index].lineage_id = Some(parent_id);
                matched_parents.insert(m.child_index, m.parent_index);
                match_rules.insert(m.child_index, (m.tier.as_rule_name().to_string(), m.score));
            }
        }

        for child_idx in &solution.unmatched_children {
            let new_id = LineageId::new(self.alloc_lineage_id());
            child_symbols[*child_idx].lineage_id = Some(new_id);
            match_rules.insert(*child_idx, ("new_lineage".to_string(), 1.0));
        }

        // 6. Secondary derivation analysis: Split (M7), Merge, and Copy (M10) per D-09, D-10, D-11
        let mut derivation_edges = Vec::new();

        // 6a. Copies (M10) & Splits (M7) for unmatched children
        for &child_idx in &solution.unmatched_children {
            let child = &child_symbols[child_idx];
            let child_lineage_id = child.lineage_id.expect("child has lineage_id");

            if child.is_short {
                continue;
            }

            let mut best_copy: Option<(LineageId, f32)> = None;
            let mut best_split: Option<(LineageId, f32)> = None;

            for m in &solution.matches {
                let Some(parent) = parent_symbols.get(m.parent_index) else {
                    continue;
                };
                if parent.language != child.language || parent.is_short {
                    continue;
                }

                let sim = parent.fingerprint.similarity(&child.fingerprint);
                let parent_lineage_id = parent.lineage_id.expect("parent has lineage_id");

                let is_same_file = child.path == parent.path;

                if is_same_file {
                    if sim >= self.thresholds.tau_split
                        && best_split.as_ref().is_none_or(|(_, s)| sim > *s)
                    {
                        best_split = Some((parent_lineage_id, sim));
                    }
                } else if sim >= 0.80 {
                    if best_copy.as_ref().is_none_or(|(_, s)| sim > *s) {
                        best_copy = Some((parent_lineage_id, sim));
                    }
                } else if sim >= self.thresholds.tau_split
                    && best_split.as_ref().is_none_or(|(_, s)| sim > *s)
                {
                    best_split = Some((parent_lineage_id, sim));
                }
            }

            // Also check active symbols in untouched files across the repository for Copy (M10) via LSH index
            if best_split.is_none() {
                let candidates = self
                    .lsh_index
                    .get_candidates(child.language, &child.fingerprint);
                for active_sym in candidates {
                    if active_sym.path == child.path {
                        continue;
                    }

                    let sim = active_sym.fingerprint.similarity(&child.fingerprint);
                    if sim >= 0.80 {
                        match &best_copy {
                            None => best_copy = Some((active_sym.lineage_id, sim)),
                            Some((best_id, best_s)) => {
                                if sim > *best_s
                                    || (sim == *best_s && active_sym.lineage_id < *best_id)
                                {
                                    best_copy = Some((active_sym.lineage_id, sim));
                                }
                            }
                        }
                    }
                }
            }

            if let Some((parent_id, sim)) = best_split {
                let edge = LineageEdge {
                    parent_lineage_id: parent_id,
                    child_lineage_id,
                    commit_hash: commit_id.clone(),
                    kind: DerivationKind::Split,
                    similarity: sim,
                };
                self.dag.add_edge(edge.clone());
                derivation_edges.push(edge);
            } else if let Some((parent_id, sim)) = best_copy {
                let edge = LineageEdge {
                    parent_lineage_id: parent_id,
                    child_lineage_id,
                    commit_hash: commit_id.clone(),
                    kind: DerivationKind::Copy,
                    similarity: sim,
                };
                self.dag.add_edge(edge.clone());
                derivation_edges.push(edge);
            }
        }

        // 6b. Merges for unmatched parents
        for &parent_idx in &solution.unmatched_parents {
            let Some(parent) = parent_symbols.get(parent_idx) else {
                continue;
            };
            let parent_lineage_id = parent.lineage_id.expect("parent has lineage_id");

            if parent.is_short {
                continue;
            }

            let mut best_merge: Option<(LineageId, f32)> = None;

            for m in &solution.matches {
                let Some(child) = child_symbols.get(m.child_index) else {
                    continue;
                };
                if parent.language != child.language || child.is_short {
                    continue;
                }

                let sim = parent.fingerprint.similarity(&child.fingerprint);
                let child_lineage_id = child.lineage_id.expect("child has lineage_id");

                if sim >= self.thresholds.tau_split
                    && best_merge.as_ref().is_none_or(|(_, s)| sim > *s)
                {
                    best_merge = Some((child_lineage_id, sim));
                }
            }

            if let Some((child_id, sim)) = best_merge {
                let edge = LineageEdge {
                    parent_lineage_id,
                    child_lineage_id: child_id,
                    commit_hash: commit_id.clone(),
                    kind: DerivationKind::Merge,
                    similarity: sim,
                };
                self.dag.add_edge(edge.clone());
                derivation_edges.push(edge);
            }
        }

        // 7. Update active symbols and LSH index with new state
        for child in &child_symbols {
            self.lsh_index.insert(child);
            self.active_symbols
                .entry(child.path.clone())
                .or_default()
                .push(child.clone());
        }

        // 8. Emit FunctionTouch, Citation, and ResolvedTouch records
        let mut touches = Vec::with_capacity(child_symbols.len());
        let mut citations = Vec::with_capacity(child_symbols.len());
        let mut resolved_touches = Vec::with_capacity(child_symbols.len());

        for (c_idx, child) in child_symbols.iter().enumerate() {
            let parent_match = matched_parents
                .get(&c_idx)
                .and_then(|&p_idx| parent_symbols.get(p_idx));

            let is_added_file = diff.deltas.is_empty()
                || diff.deltas.iter().any(|d| {
                    d.status == DeltaStatus::Added
                        && d.new_path
                            .as_ref()
                            .is_some_and(|p| normalize_path(p) == child.path)
                });

            let file_has_hunks_entry = diff
                .hunks_by_path
                .iter()
                .any(|(p, _)| normalize_path(p) == child.path);

            let file_hunks: Vec<&DiffHunk> = diff
                .hunks_by_path
                .iter()
                .filter(|(p, _)| normalize_path(p) == child.path)
                .flat_map(|(_, hunks)| hunks.iter())
                .collect();

            let mut overlapping_hunk_lines = Vec::new();
            let mut has_hunk_overlap = false;

            for hunk in &file_hunks {
                let hunk_start = hunk.new_start as usize;
                let hunk_end = if hunk.new_lines == 0 {
                    hunk_start
                } else {
                    hunk_start + hunk.new_lines as usize - 1
                };

                if hunk_start <= child.end_line && child.start_line <= hunk_end {
                    has_hunk_overlap = true;
                    overlapping_hunk_lines.extend(hunk.lines.clone());
                }
            }

            let is_new = parent_match.is_none();
            let has_structural_change = match parent_match {
                Some(p) => {
                    p.signature != child.signature
                        || p.fingerprint != child.fingerprint
                        || p.path != child.path
                        || p.qualname != child.qualname
                }
                None => true,
            };

            let is_touched = is_added_file
                || has_hunk_overlap
                || !file_has_hunks_entry
                || is_new
                || has_structural_change;

            if !is_touched {
                continue;
            }

            let lineage_id = child.lineage_id.expect("all children must have lineage_id");

            // Enforce UNIQUE(lineage_id, commit_hash) per D-11
            if !self.seen_touches.insert((lineage_id, commit_id.clone())) {
                return Err(CoreError::InvariantViolation(format!(
                    "Duplicate touch for lineage {lineage_id} in commit {commit_id}"
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

            let change_type = match parent_match {
                None => "add".to_string(),
                Some(p) => {
                    if p.path != child.path {
                        "move".to_string()
                    } else if p.qualname != child.qualname {
                        "rename".to_string()
                    } else {
                        "modify".to_string()
                    }
                }
            };

            let signature_before = parent_match.map(|p| p.signature.clone());
            let arity = compute_arity(&child.signature);
            let fingerprint = child.fingerprint.to_bytes();

            resolved_touches.push(ResolvedTouch {
                touch: touch.clone(),
                match_rule,
                match_confidence,
                arity,
                change_type,
                signature_before,
                fingerprint,
                diff_hunks: overlapping_hunk_lines,
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
            derivation_edges,
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

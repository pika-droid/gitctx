//! Lineage Derivation DAG per LINEAGE-03, D-09, D-10, D-11.
//!
//! Models evolutionary derivation edges (Split, Merge, Copy) between function lineages
//! across historical commit transitions.

use crate::model::{CommitId, LineageId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// Evolutionary derivation relationship kind per D-09, D-10, D-11.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DerivationKind {
    /// Function split: child was extracted from parent (M7).
    Split,
    /// Function merge: multiple parents consolidated into child.
    Merge,
    /// Function copy: child is a duplicate of parent at a new location (M10).
    Copy,
}

impl std::fmt::Display for DerivationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Split => write!(f, "split"),
            Self::Merge => write!(f, "merge"),
            Self::Copy => write!(f, "copy"),
        }
    }
}

/// A directed edge in the function lineage derivation DAG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineageEdge {
    pub parent_lineage_id: LineageId,
    pub child_lineage_id: LineageId,
    pub commit_hash: CommitId,
    pub kind: DerivationKind,
    pub similarity: f32,
}

/// In-memory graph container for evolutionary lineage derivations.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LineageDag {
    edges: Vec<LineageEdge>,
    by_parent: HashMap<LineageId, Vec<usize>>,
    by_child: HashMap<LineageId, Vec<usize>>,
    by_commit: HashMap<CommitId, Vec<usize>>,
}

impl LineageDag {
    /// Create an empty Lineage DAG.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a derivation edge to the DAG.
    pub fn add_edge(&mut self, edge: LineageEdge) {
        let idx = self.edges.len();
        self.by_parent
            .entry(edge.parent_lineage_id)
            .or_default()
            .push(idx);
        self.by_child
            .entry(edge.child_lineage_id)
            .or_default()
            .push(idx);
        self.by_commit
            .entry(edge.commit_hash.clone())
            .or_default()
            .push(idx);
        self.edges.push(edge);
    }

    /// Access all recorded derivation edges.
    #[must_use]
    pub fn edges(&self) -> &[LineageEdge] {
        &self.edges
    }

    /// Return total number of derivation edges.
    #[must_use]
    pub fn len(&self) -> usize {
        self.edges.len()
    }

    /// Check if DAG has no derivation edges.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    /// Get all derivation edges where `parent_id` is the parent.
    #[must_use]
    pub fn get_children(&self, parent_id: LineageId) -> Vec<&LineageEdge> {
        self.by_parent
            .get(&parent_id)
            .map(|indices| indices.iter().map(|&idx| &self.edges[idx]).collect())
            .unwrap_or_default()
    }

    /// Get all derivation edges where `child_id` is the child.
    #[must_use]
    pub fn get_parents(&self, child_id: LineageId) -> Vec<&LineageEdge> {
        self.by_child
            .get(&child_id)
            .map(|indices| indices.iter().map(|&idx| &self.edges[idx]).collect())
            .unwrap_or_default()
    }

    /// Get all derivation edges introduced in a given commit.
    #[must_use]
    pub fn get_edges_for_commit(&self, commit_id: &CommitId) -> Vec<&LineageEdge> {
        self.by_commit
            .get(commit_id)
            .map(|indices| indices.iter().map(|&idx| &self.edges[idx]).collect())
            .unwrap_or_default()
    }

    /// Collect all ancestor lineage IDs reachable from `lineage_id` via reverse BFS.
    #[must_use]
    pub fn ancestors(&self, lineage_id: LineageId) -> HashSet<LineageId> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(lineage_id);

        while let Some(current) = queue.pop_front() {
            if let Some(parent_indices) = self.by_child.get(&current) {
                for &idx in parent_indices {
                    let parent = self.edges[idx].parent_lineage_id;
                    if visited.insert(parent) {
                        queue.push_back(parent);
                    }
                }
            }
        }

        visited
    }

    /// Collect all descendant lineage IDs reachable from `lineage_id` via BFS.
    #[must_use]
    pub fn descendants(&self, lineage_id: LineageId) -> HashSet<LineageId> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(lineage_id);

        while let Some(current) = queue.pop_front() {
            if let Some(child_indices) = self.by_parent.get(&current) {
                for &idx in child_indices {
                    let child = self.edges[idx].child_lineage_id;
                    if visited.insert(child) {
                        queue.push_back(child);
                    }
                }
            }
        }

        visited
    }
}

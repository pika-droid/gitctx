//! Lineage ground-truth manifest schema and serialization.
//!
//! Provides the structured JSON schema (`lineage_ground_truth.json`) recording
//! commit SHAs, mutation classes, source/target symbol locations, and expected
//! lineage relationships per D-10.

use crate::mutator::transformations::MutationClass;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Precise location of a source code symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolLocation {
    /// Relative path within the repository root.
    pub path: String,
    /// Fully qualified symbol identifier (e.g. `order_service.calculate_tax`).
    pub qualname: String,
    /// Starting 1-indexed line number.
    pub start_line: usize,
    /// Ending 1-indexed line number (inclusive).
    pub end_line: usize,
}

impl SymbolLocation {
    /// Creates a new symbol location.
    #[must_use]
    pub fn new(
        path: impl Into<String>,
        qualname: impl Into<String>,
        start_line: usize,
        end_line: usize,
    ) -> Self {
        Self {
            path: path.into(),
            qualname: qualname.into(),
            start_line,
            end_line,
        }
    }
}

/// Expected ground truth relationship category between two symbol revisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GroundTruthRelation {
    /// Exact match with minor signature, formatting, or internal variable tweaks.
    #[serde(alias = "Exact", alias = "exact")]
    Exact,
    /// Renamed symbol within the same file or across files.
    #[serde(alias = "Renamed", alias = "renamed")]
    Renamed,
    /// Relocated symbol to a new file or directory location.
    #[serde(alias = "Moved", alias = "moved")]
    Moved,
    /// Sub-function or helper extracted from the original symbol.
    #[serde(alias = "SplitFrom", alias = "split_from", alias = "SPLIT_FROM")]
    SplitFrom,
    /// Decoy symbol (deleted and replaced with unrelated code).
    #[serde(alias = "Decoy", alias = "decoy")]
    Decoy,
    /// Duplicated or copied symbol retaining the original instance.
    #[serde(alias = "Copied", alias = "copied")]
    Copied,
}

impl std::fmt::Display for GroundTruthRelation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exact => write!(f, "EXACT"),
            Self::Renamed => write!(f, "RENAMED"),
            Self::Moved => write!(f, "MOVED"),
            Self::SplitFrom => write!(f, "SPLIT_FROM"),
            Self::Decoy => write!(f, "DECOY"),
            Self::Copied => write!(f, "COPIED"),
        }
    }
}

/// A single ground-truth lineage link recorded across a Git commit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineageLink {
    /// Full 40-character Git commit hash.
    pub commit_sha: String,
    /// Applied transformation class (M1-M10).
    pub mutation_class: MutationClass,
    /// Symbol location prior to the mutation commit.
    pub source: SymbolLocation,
    /// Symbol location following the mutation commit.
    pub target: SymbolLocation,
    /// Expected semantic relationship.
    pub relation: GroundTruthRelation,
    /// Expected confidence or similarity benchmark score in `[0.0, 1.0]`.
    pub confidence_expected: f32,
}

impl LineageLink {
    /// Constructs a new lineage ground truth link entry.
    #[must_use]
    pub fn new(
        commit_sha: impl Into<String>,
        mutation_class: MutationClass,
        source: SymbolLocation,
        target: SymbolLocation,
        relation: GroundTruthRelation,
        confidence_expected: f32,
    ) -> Self {
        Self {
            commit_sha: commit_sha.into(),
            mutation_class,
            source,
            target,
            relation,
            confidence_expected,
        }
    }
}

/// Structured manifest containing all synthetic ground-truth links for an evaluation run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineageGroundTruthManifest {
    /// Version of the manifest schema.
    pub schema_version: u32,
    /// UTC timestamp of manifest generation.
    pub created_at: String,
    /// Number of sequential mutation commits applied.
    pub chain_depth: usize,
    /// Sequential list of ground truth lineage links.
    pub links: Vec<LineageLink>,
    /// Optional metadata key-value dictionary.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl Default for LineageGroundTruthManifest {
    fn default() -> Self {
        Self::new(0)
    }
}

impl LineageGroundTruthManifest {
    /// Creates a fresh manifest for a given chain depth.
    #[must_use]
    pub fn new(chain_depth: usize) -> Self {
        Self {
            schema_version: 1,
            created_at: "2026-08-16T00:00:00Z".to_string(),
            chain_depth,
            links: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Appends a new lineage link to the manifest.
    pub fn add_link(&mut self, link: LineageLink) {
        self.links.push(link);
    }

    /// Serializes manifest to pretty JSON string.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserializes manifest from JSON string.
    pub fn from_json_str(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    /// Writes manifest to a target file path.
    pub fn save_to_file(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(path, json)
    }

    /// Reads and deserializes manifest from a file path.
    pub fn load_from_file(path: &Path) -> std::io::Result<Self> {
        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

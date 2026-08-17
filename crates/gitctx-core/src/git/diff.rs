use crate::error::{CoreError, Result};
use crate::git::repository::GitRepo;
use crate::model::CommitId;
use git2::{Delta, Diff, DiffFindOptions, DiffOptions, Oid};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Status of a changed file.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DeltaStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Ignored,
    Untracked,
    Typechange,
    Unreadable,
    Conflicted,
}

impl From<Delta> for DeltaStatus {
    fn from(d: Delta) -> Self {
        match d {
            Delta::Added => DeltaStatus::Added,
            Delta::Deleted => DeltaStatus::Deleted,
            Delta::Unmodified | Delta::Modified => DeltaStatus::Modified,
            Delta::Renamed => DeltaStatus::Renamed,
            Delta::Copied => DeltaStatus::Copied,
            Delta::Ignored => DeltaStatus::Ignored,
            Delta::Untracked => DeltaStatus::Untracked,
            Delta::Typechange => DeltaStatus::Typechange,
            Delta::Unreadable => DeltaStatus::Unreadable,
            Delta::Conflicted => DeltaStatus::Conflicted,
        }
    }
}

/// A file-level modification record.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FileDelta {
    pub old_path: Option<PathBuf>,
    pub new_path: Option<PathBuf>,
    pub status: DeltaStatus,
    pub old_oid: Option<String>,
    pub new_oid: Option<String>,
    pub similarity: u16,
    pub is_binary: bool,
}

/// A unified diff hunk.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
    pub lines: Vec<String>,
}

/// Full extracted diff for a single commit.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CommitDiff {
    pub commit_id: CommitId,
    pub parent_id: Option<CommitId>,
    pub deltas: Vec<FileDelta>,
    pub hunks_by_path: HashMap<PathBuf, Vec<DiffHunk>>,
    pub raw_diff: String,
}

/// Extractor for tree diffs and hunks.
pub struct TreeDiffExtractor;

impl TreeDiffExtractor {
    /// Extract diff between a commit and its first parent (or against an empty tree for root commits).
    pub fn extract(repo: &GitRepo, commit_oid: Oid, parent_oid: Option<Oid>) -> Result<CommitDiff> {
        let commit = repo.find_commit(commit_oid)?;
        let new_tree = commit.tree().map_err(CoreError::from)?;

        let old_tree = match parent_oid {
            Some(pid) => {
                let parent = repo.find_commit(pid)?;
                Some(parent.tree().map_err(CoreError::from)?)
            }
            None => None,
        };

        let mut diff_opts = DiffOptions::new();
        diff_opts.context_lines(3);

        let mut diff = repo
            .inner()
            .diff_tree_to_tree(old_tree.as_ref(), Some(&new_tree), Some(&mut diff_opts))
            .map_err(CoreError::from)?;

        let mut find_opts = DiffFindOptions::new();
        find_opts.renames(true);
        find_opts.copies(true);
        find_opts.exact_match_only(false);
        diff.find_similar(Some(&mut find_opts))
            .map_err(CoreError::from)?;

        let deltas = Self::extract_deltas(&diff)?;
        let (raw_diff, hunks_by_path) = Self::extract_hunks_and_raw(&diff)?;

        Ok(CommitDiff {
            commit_id: CommitId::new(commit_oid.to_string()),
            parent_id: parent_oid.map(|p| CommitId::new(p.to_string())),
            deltas,
            hunks_by_path,
            raw_diff,
        })
    }

    fn extract_deltas(diff: &Diff) -> Result<Vec<FileDelta>> {
        let mut deltas = Vec::with_capacity(diff.deltas().len());
        for i in 0..diff.deltas().len() {
            let delta = diff.get_delta(i).ok_or_else(|| {
                CoreError::GitInspection(format!("Missing diff delta at index {i}"))
            })?;

            let old_file = delta.old_file();
            let new_file = delta.new_file();

            let old_path = old_file.path().map(PathBuf::from);
            let new_path = new_file.path().map(PathBuf::from);

            let old_oid = if old_file.id().is_zero() {
                None
            } else {
                Some(old_file.id().to_string())
            };

            let new_oid = if new_file.id().is_zero() {
                None
            } else {
                Some(new_file.id().to_string())
            };

            let status = DeltaStatus::from(delta.status());
            let is_binary = delta.flags().contains(git2::DiffFlags::BINARY);
            let similarity = if status == DeltaStatus::Renamed || status == DeltaStatus::Copied {
                100
            } else {
                0
            };

            deltas.push(FileDelta {
                old_path,
                new_path,
                status,
                old_oid,
                new_oid,
                similarity,
                is_binary,
            });
        }
        Ok(deltas)
    }

    fn extract_hunks_and_raw(diff: &Diff) -> Result<(String, HashMap<PathBuf, Vec<DiffHunk>>)> {
        let mut raw_diff = String::new();
        let mut hunks_by_path: HashMap<PathBuf, Vec<DiffHunk>> = HashMap::new();
        let mut current_file_path: Option<PathBuf> = None;
        let mut current_hunk: Option<DiffHunk> = None;

        diff.print(git2::DiffFormat::Patch, |delta, maybe_hunk, line| {
            let line_content = String::from_utf8_lossy(line.content()).to_string();
            let origin = line.origin();

            raw_diff.push(origin);
            raw_diff.push_str(&line_content);

            let file_key = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(PathBuf::from);

            if let Some(target_path) = file_key {
                if current_file_path.as_ref() != Some(&target_path) {
                    if let Some(hunk) = current_hunk.take() {
                        if let Some(prev_path) = current_file_path.take() {
                            hunks_by_path.entry(prev_path).or_default().push(hunk);
                        }
                    }
                    current_file_path = Some(target_path);
                }

                if let Some(h) = maybe_hunk {
                    let hunk_header = String::from_utf8_lossy(h.header()).trim().to_string();
                    let is_new_hunk = match &current_hunk {
                        Some(cur) => {
                            cur.old_start != h.old_start() || cur.new_start != h.new_start()
                        }
                        None => true,
                    };

                    if is_new_hunk {
                        if let Some(prev) = current_hunk.take() {
                            if let Some(ref path) = current_file_path {
                                hunks_by_path.entry(path.clone()).or_default().push(prev);
                            }
                        }
                        current_hunk = Some(DiffHunk {
                            old_start: h.old_start(),
                            old_lines: h.old_lines(),
                            new_start: h.new_start(),
                            new_lines: h.new_lines(),
                            header: hunk_header,
                            lines: Vec::new(),
                        });
                    }
                }

                if let Some(ref mut cur) = current_hunk {
                    let mut formatted = String::with_capacity(line_content.len() + 1);
                    formatted.push(origin);
                    formatted.push_str(&line_content);
                    cur.lines.push(formatted);
                }
            }

            true
        })
        .map_err(CoreError::from)?;

        if let Some(hunk) = current_hunk {
            if let Some(path) = current_file_path {
                hunks_by_path.entry(path).or_default().push(hunk);
            }
        }

        Ok((raw_diff, hunks_by_path))
    }
}

use crate::error::{CoreError, Result};
use crate::git::repository::GitRepo;
use crate::model::CommitId;
use git2::{Oid, Revwalk, Sort};

/// Metadata for an extracted Git commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitInfo {
    pub commit_id: CommitId,
    pub oid: Oid,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: i64,
    pub message: String,
    pub parent_ids: Vec<CommitId>,
}

/// First-parent commit walker with topological and chronological ordering.
pub struct GitWalker<'a> {
    repo: &'a GitRepo,
    revwalk: Revwalk<'a>,
}

impl<'a> GitWalker<'a> {
    /// Create a walker that traverses all first-parent commits from HEAD.
    pub fn new(repo: &'a GitRepo) -> Result<Self> {
        let mut revwalk = repo.inner().revwalk().map_err(CoreError::from)?;
        revwalk
            .set_sorting(Sort::TOPOLOGICAL | Sort::TIME)
            .map_err(CoreError::from)?;
        revwalk.simplify_first_parent().map_err(CoreError::from)?;
        revwalk.push_head().map_err(CoreError::from)?;
        Ok(Self { repo, revwalk })
    }

    /// Create a walker that traverses from an explicit tip commit to an optional base.
    pub fn from_tip(repo: &'a GitRepo, tip_oid: Oid, stop_oid: Option<Oid>) -> Result<Self> {
        let mut revwalk = repo.inner().revwalk().map_err(CoreError::from)?;
        revwalk
            .set_sorting(Sort::TOPOLOGICAL | Sort::TIME)
            .map_err(CoreError::from)?;
        revwalk.simplify_first_parent().map_err(CoreError::from)?;
        revwalk.push(tip_oid).map_err(CoreError::from)?;
        if let Some(stop) = stop_oid {
            revwalk.hide(stop).map_err(CoreError::from)?;
        }
        Ok(Self { repo, revwalk })
    }

    /// Collect all commits from this walker into a vector.
    pub fn collect_all(self) -> Result<Vec<CommitInfo>> {
        self.collect()
    }
}

impl Iterator for GitWalker<'_> {
    type Item = Result<CommitInfo>;

    fn next(&mut self) -> Option<Self::Item> {
        let oid = match self.revwalk.next()? {
            Ok(oid) => oid,
            Err(e) => return Some(Err(CoreError::from(e))),
        };

        let commit = match self.repo.find_commit(oid) {
            Ok(c) => c,
            Err(e) => return Some(Err(e)),
        };

        let author = commit.author();
        let author_name = author.name().unwrap_or("Unknown").to_string();
        let author_email = author.email().unwrap_or("").to_string();
        let timestamp = commit.time().seconds();
        let message = commit.message().unwrap_or("").to_string();

        let parent_ids = commit
            .parent_ids()
            .map(|id| CommitId::new(id.to_string()))
            .collect();

        Some(Ok(CommitInfo {
            commit_id: CommitId::new(oid.to_string()),
            oid,
            author_name,
            author_email,
            timestamp,
            message,
            parent_ids,
        }))
    }
}

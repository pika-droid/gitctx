//! Programmatic synthetic Git repository builder engine using `git2`.
//!
//! Provides deterministic commit timestamps (`1700000000 + i*60`), consistent author signatures,
//! and RAII-managed temporary directories per D-05 and D-08.

use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[cfg(windows)]
#[link(name = "advapi32")]
extern "C" {}

/// Programmatic Git repository builder for deterministic fixture creation.
pub struct RepoBuilder {
    temp_dir: Option<TempDir>,
    explicit_path: Option<PathBuf>,
    repo: git2::Repository,
    commit_count: u64,
    author_name: String,
    author_email: String,
    base_time: i64,
}

impl RepoBuilder {
    /// Creates a new `RepoBuilder` in an isolated RAII-managed temporary directory.
    pub fn new() -> Result<Self, git2::Error> {
        let temp_dir = TempDir::new().map_err(|e| {
            git2::Error::from_str(&format!("Failed to create temporary directory: {}", e))
        })?;
        let repo = git2::Repository::init(temp_dir.path())?;
        Ok(Self {
            temp_dir: Some(temp_dir),
            explicit_path: None,
            repo,
            commit_count: 0,
            author_name: "gitctx-test".to_string(),
            author_email: "test@gitctx.local".to_string(),
            base_time: 1_700_000_000,
        })
    }

    /// Creates a new `RepoBuilder` in an explicit target directory.
    pub fn new_in(path: &Path) -> Result<Self, git2::Error> {
        std::fs::create_dir_all(path).map_err(|e| {
            git2::Error::from_str(&format!("Failed to create directory at {:?}: {}", path, e))
        })?;
        let repo = git2::Repository::init(path)?;
        Ok(Self {
            temp_dir: None,
            explicit_path: Some(path.to_path_buf()),
            repo,
            commit_count: 0,
            author_name: "gitctx-test".to_string(),
            author_email: "test@gitctx.local".to_string(),
            base_time: 1_700_000_000,
        })
    }

    /// Returns the filesystem path to the working tree root.
    pub fn path(&self) -> &Path {
        if let Some(ref temp) = self.temp_dir {
            temp.path()
        } else if let Some(ref exp) = self.explicit_path {
            exp.as_path()
        } else {
            self.repo.workdir().unwrap_or_else(|| Path::new("."))
        }
    }

    /// Returns a reference to the underlying `git2::Repository`.
    pub fn repo(&self) -> &git2::Repository {
        &self.repo
    }

    /// Returns a mutable reference to the underlying `git2::Repository`.
    pub fn repo_mut(&mut self) -> &mut git2::Repository {
        &mut self.repo
    }

    /// Returns the number of commits made so far.
    pub fn commit_count(&self) -> u64 {
        self.commit_count
    }

    /// Returns the OID of HEAD if available.
    pub fn head_oid(&self) -> Result<git2::Oid, git2::Error> {
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;
        Ok(commit.id())
    }

    /// Returns the short name of the current HEAD branch (e.g. "master" or "main").
    pub fn current_branch_name(&self) -> Result<String, git2::Error> {
        let head = self.repo.head()?;
        if let Some(name) = head.shorthand() {
            Ok(name.to_string())
        } else {
            Ok("master".to_string())
        }
    }

    /// Writes content to a file relative to the repository working directory.
    pub fn write_file(&mut self, rel_path: &str, content: &str) -> Result<&mut Self, std::io::Error> {
        let full_path = self.path().join(rel_path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&full_path, content)?;
        Ok(self)
    }

    /// Removes a file relative to the repository working directory if it exists.
    pub fn remove_file(&mut self, rel_path: &str) -> Result<&mut Self, std::io::Error> {
        let full_path = self.path().join(rel_path);
        if full_path.exists() {
            std::fs::remove_file(&full_path)?;
        }
        Ok(self)
    }

    /// Helper to stage all changes (additions, modifications, deletions) into the index and write tree.
    fn stage_and_write_tree(&self) -> Result<git2::Oid, git2::Error> {
        let mut index = self.repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;

        // Remove deleted files from index
        let mut to_remove = Vec::new();
        for entry in index.iter() {
            let entry_path = String::from_utf8_lossy(&entry.path).to_string();
            if !self.path().join(&entry_path).exists() {
                to_remove.push(entry_path);
            }
        }
        for path in to_remove {
            index.remove_path(Path::new(&path))?;
        }

        index.write()?;
        index.write_tree()
    }

    /// Creates a deterministic commit with the current working tree state.
    ///
    /// Timestamp is computed as `base_time + commit_count * 60` with zero offset.
    pub fn commit(&mut self, message: &str) -> Result<git2::Oid, git2::Error> {
        let tree_oid = self.stage_and_write_tree()?;
        let tree = self.repo.find_tree(tree_oid)?;

        let time = git2::Time::new(self.base_time + (self.commit_count as i64) * 60, 0);
        let sig = git2::Signature::new(&self.author_name, &self.author_email, &time)?;

        let parent_commit = match self.repo.head() {
            Ok(head) => Some(head.peel_to_commit()?),
            Err(_) => None,
        };

        let oid = match parent_commit {
            Some(ref parent) => {
                self.repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[parent])?
            }
            None => {
                self.repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])?
            }
        };

        self.commit_count += 1;
        Ok(oid)
    }

    /// Creates a new named branch pointing at the current HEAD commit.
    pub fn create_branch(&self, name: &str) -> Result<(), git2::Error> {
        let head_commit = self.repo.head()?.peel_to_commit()?;
        self.repo.branch(name, &head_commit, true)?;
        Ok(())
    }

    /// Checks out the specified named branch and updates the working tree.
    pub fn checkout_branch(&self, name: &str) -> Result<(), git2::Error> {
        let refname = format!("refs/heads/{}", name);
        let obj = self.repo.revparse_single(&refname)?;
        self.repo.checkout_tree(
            &obj,
            Some(git2::build::CheckoutBuilder::new().force()),
        )?;
        self.repo.set_head(&refname)?;
        Ok(())
    }

    /// Merges `source_branch` into the current branch, creating a merge commit with 2 parents.
    pub fn merge(&mut self, source_branch: &str, message: &str) -> Result<git2::Oid, git2::Error> {
        let head_commit = self.repo.head()?.peel_to_commit()?;
        let branch_ref = format!("refs/heads/{}", source_branch);
        let branch_commit = self.repo.find_reference(&branch_ref)?.peel_to_commit()?;

        let tree_oid = self.stage_and_write_tree()?;
        let tree = self.repo.find_tree(tree_oid)?;

        let time = git2::Time::new(self.base_time + (self.commit_count as i64) * 60, 0);
        let sig = git2::Signature::new(&self.author_name, &self.author_email, &time)?;

        let oid = self.repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &[&head_commit, &branch_commit],
        )?;

        self.commit_count += 1;
        Ok(oid)
    }

    /// Performs a squash merge of `source_branch` into current HEAD, producing a single parent commit.
    pub fn squash_merge(&mut self, source_branch: &str, message: &str) -> Result<git2::Oid, git2::Error> {
        let head_commit = self.repo.head()?.peel_to_commit()?;
        let branch_ref = format!("refs/heads/{}", source_branch);
        let branch_commit = self.repo.find_reference(&branch_ref)?.peel_to_commit()?;

        // Checkout the source branch tree onto disk without moving HEAD
        self.repo.checkout_tree(
            branch_commit.as_object(),
            Some(git2::build::CheckoutBuilder::new().force()),
        )?;

        let tree_oid = self.stage_and_write_tree()?;
        let tree = self.repo.find_tree(tree_oid)?;

        let time = git2::Time::new(self.base_time + (self.commit_count as i64) * 60, 0);
        let sig = git2::Signature::new(&self.author_name, &self.author_email, &time)?;

        let oid = self.repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &[&head_commit],
        )?;

        self.commit_count += 1;
        Ok(oid)
    }
}

use crate::error::{CoreError, Result};
use git2::{Blob, Commit, Oid, Repository};
use std::path::{Path, PathBuf};

/// Safe wrapper around `git2::Repository`.
pub struct GitRepo {
    repo: Repository,
    path: PathBuf,
}

impl GitRepo {
    /// Open a Git repository at the specified path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        let repo = Repository::open(&path_buf).map_err(|e| CoreError::RepositoryOpen {
            path: path_buf.clone(),
            message: e.message().to_string(),
        })?;
        Ok(Self {
            repo,
            path: path_buf,
        })
    }

    /// Access the underlying `git2::Repository`.
    pub fn inner(&self) -> &Repository {
        &self.repo
    }

    /// Path to the repository.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Working directory of the repository (if not bare).
    pub fn workdir(&self) -> Option<&Path> {
        self.repo.workdir()
    }

    /// Get the OID of HEAD.
    pub fn head_oid(&self) -> Result<Oid> {
        let head = self.repo.head().map_err(CoreError::from)?;
        head.target()
            .ok_or_else(|| CoreError::GitInspection("HEAD is a symbolic ref without target".to_string()))
    }

    /// Find a commit by its OID.
    pub fn find_commit(&self, oid: Oid) -> Result<Commit<'_>> {
        self.repo.find_commit(oid).map_err(CoreError::from)
    }

    /// Find a blob by its OID.
    pub fn find_blob(&self, oid: Oid) -> Result<Blob<'_>> {
        self.repo.find_blob(oid).map_err(CoreError::from)
    }
}

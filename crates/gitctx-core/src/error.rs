use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Configuration mismatch: expected {expected}, actual {actual}")]
    ConfigMismatch { expected: String, actual: String },

    #[error("Corpus SHA mismatch for {repo:?}: expected {expected}, actual {actual}")]
    CorpusShaMismatch {
        expected: String,
        actual: String,
        repo: PathBuf,
    },

    #[error("Repository open error for {path:?}: {message}")]
    RepositoryOpen { path: PathBuf, message: String },

    #[error("Git inspection error: {0}")]
    GitInspection(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Invalid identifier: {0}")]
    InvalidIdentifier(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, CoreError>;

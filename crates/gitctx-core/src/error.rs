use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Configuration mismatch: expected {expected}, actual {actual}")]
    ConfigMismatch { expected: String, actual: String },

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

#![forbid(unsafe_code)]

pub mod config;
pub mod error;
pub mod model;
pub mod qualname;
pub mod sanitize;

pub use config::{ConfigHash, IndexConfig};
pub use error::{CoreError, Result};
pub use model::{Citation, CommitId, FunctionTouch, LineageId, ParseStatus};
pub use qualname::QualName;
pub use sanitize::{sanitize_chunk_text, SanitizedChunk};

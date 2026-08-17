#![forbid(unsafe_code)]

pub mod config;
pub mod error;
pub mod git;
pub mod model;
pub mod parser;
pub mod qualname;
pub mod redact;
pub mod sanitize;
pub mod template;

pub use config::{ConfigHash, IndexConfig};
pub use error::{CoreError, Result};
pub use git::{CommitDiff, CommitInfo, DeltaStatus, DiffHunk, FileDelta, GitRepo, GitWalker, TreeDiffExtractor};
pub use model::{Citation, CommitId, FunctionTouch, LineageId, ParseStatus};
pub use parser::{
    AstExtractor, BlobParseCache, DegradationHandler, LanguageBehavior, LanguageId,
    LanguageRegistry, ParseTelemetry, ParsedFileSymbols, RawFunctionSymbol,
};
pub use qualname::QualName;
pub use redact::{
    is_exempt, redact_high_entropy_tokens, redact_known_prefixes, shannon_entropy, SecretRedactor,
    MIN_ENTROPY_TOKEN_LEN, SHANNON_ENTROPY_THRESHOLD,
};
pub use sanitize::{sanitize_chunk_text, SanitizedChunk};
pub use template::{
    is_keyword, D4TemplateRenderer, FunctionTemplateContext, IdentifierDelta, IdentifierExtractor,
    ModuleTemplateContext,
};



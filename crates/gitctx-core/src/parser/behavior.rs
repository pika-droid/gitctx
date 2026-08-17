use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Supported programming language identifier.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum LanguageId {
    Python,
    Rust,
    JavaScript,
    TypeScript,
    Go,
    C,
    Cpp,
    Java,
    Php,
}

impl LanguageId {
    /// Return the canonical string name of the language.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::Rust => "rust",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Go => "go",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Java => "java",
            Self::Php => "php",
        }
    }
}

/// Extracted raw function/method symbol from a tree-sitter AST.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RawFunctionSymbol {
    pub name: String,
    pub qualname: String,
    pub signature: String,
    pub docstring: Option<String>,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: u32,
    pub end_line: u32,
    pub is_method: bool,
}

/// Language-specific behavior abstraction for AST symbol extraction.
pub trait LanguageBehavior: Send + Sync {
    /// Canonical language ID.
    fn language_id(&self) -> LanguageId;

    /// File extensions associated with this language.
    fn extensions(&self) -> &'static [&'static str];

    /// Test if the shebang line matches this language.
    fn matches_shebang(&self, first_line: &str) -> bool;

    /// Return the tree-sitter language grammar.
    fn tree_sitter_language(&self) -> tree_sitter::Language;

    /// Extract all function and method symbols from the AST.
    fn extract_symbols(&self, code: &[u8], tree: &tree_sitter::Tree) -> Result<Vec<RawFunctionSymbol>>;
}

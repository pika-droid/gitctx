//! Deterministic D4 pseudo-document templater.
//!
//! Generates structured, natural-language chunks for function touches and module-level
//! changes per D-09, D-10, D-11, D-12, and D-16.

pub mod identifiers;
pub mod keywords;

pub use identifiers::{IdentifierDelta, IdentifierExtractor};
pub use keywords::is_keyword;

use crate::model::FunctionTouch;
use crate::redact::SecretRedactor;
use crate::sanitize::sanitize_chunk_text;
use std::collections::BTreeSet;

/// Context for rendering a function-level D4 pseudo-document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionTemplateContext<'a> {
    pub qualname: &'a str,
    pub path: &'a str,
    pub author: &'a str,
    pub date: &'a str,
    pub raw_message: &'a str,
    pub added_identifiers: &'a [String],
    pub removed_identifiers: &'a [String],
    pub signature: Option<&'a str>,
    pub docstring: Option<&'a str>,
}

/// Context for rendering a module-level D4 pseudo-document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleTemplateContext<'a> {
    pub path: &'a str,
    pub author: &'a str,
    pub date: &'a str,
    pub raw_message: &'a str,
    pub modified_symbols: &'a [String],
}

/// Deterministic D4 template pseudo-document renderer.
pub struct D4TemplateRenderer;

impl D4TemplateRenderer {
    /// Renders a canonical function-level D4 pseudo-document with empty section omission.
    ///
    /// The template renders the 6 canonical sections in deterministic order:
    /// 1. Header (`Function \`{qualname}\` in \`{path}\` was modified by {author} on {date}.`)
    /// 2. Sanitized Commit Message (`Commit message: {sanitized_message}`)
    /// 3. Added Identifiers (`Added: {id1, id2, ...}`)
    /// 4. Removed Identifiers (`Removed: {id1, id2, ...}`)
    /// 5. Function Signature (`Signature: {sig}`)
    /// 6. Surrounding Docstring (`Surrounding docstring: {doc}`)
    ///
    /// Empty sections are cleanly omitted, and secret redaction is universally applied.
    #[must_use]
    pub fn render_function(ctx: &FunctionTemplateContext<'_>) -> String {
        let mut sections = Vec::new();

        // 1. Header
        sections.push(format!(
            "Function `{}` in `{}` was modified by {} on {}.",
            ctx.qualname, ctx.path, ctx.author, ctx.date
        ));

        // 2. Sanitized commit message
        let sanitized = sanitize_chunk_text(ctx.raw_message);
        if !sanitized.sanitized_text.is_empty() {
            sections.push(format!("Commit message: {}", sanitized.sanitized_text));
        }

        // 3. Added identifiers
        if !ctx.added_identifiers.is_empty() {
            sections.push(format!("Added: {}", ctx.added_identifiers.join(", ")));
        }

        // 4. Removed identifiers
        if !ctx.removed_identifiers.is_empty() {
            sections.push(format!("Removed: {}", ctx.removed_identifiers.join(", ")));
        }

        // 5. Signature
        if let Some(sig) = ctx.signature {
            let trimmed = sig.trim();
            if !trimmed.is_empty() {
                sections.push(format!("Signature: {trimmed}"));
            }
        }

        // 6. Docstring
        if let Some(doc) = ctx.docstring {
            let trimmed = doc.trim();
            if !trimmed.is_empty() {
                sections.push(format!("Surrounding docstring: {trimmed}"));
            }
        }

        let joined = sections.join("\n");
        SecretRedactor::redact(&joined)
    }

    /// Renders a module-level D4 pseudo-document for top-level non-function changes per D-10.
    #[must_use]
    pub fn render_module(ctx: &ModuleTemplateContext<'_>) -> String {
        let mut sections = Vec::new();

        // 1. Header
        sections.push(format!(
            "Module `{}` was modified by {} on {}.",
            ctx.path, ctx.author, ctx.date
        ));

        // 2. Sanitized commit message
        let sanitized = sanitize_chunk_text(ctx.raw_message);
        if !sanitized.sanitized_text.is_empty() {
            sections.push(format!("Commit message: {}", sanitized.sanitized_text));
        }

        // 3. Modified top-level symbols
        if !ctx.modified_symbols.is_empty() {
            sections.push(format!(
                "Modified top-level symbols: {}",
                ctx.modified_symbols.join(", ")
            ));
        }

        let joined = sections.join("\n");
        SecretRedactor::redact(&joined)
    }

    /// Renders a D4 pseudo-document for an extracted `FunctionTouch`.
    ///
    /// Automatically routes `<module>` touches, `<file_level>` degraded touches,
    /// and standard function touches to their respective canonical representations.
    #[must_use]
    pub fn render_touch(
        touch: &FunctionTouch,
        author: &str,
        date: &str,
        raw_message: &str,
        delta: &IdentifierDelta,
    ) -> String {
        if touch.qualname == "<module>" {
            let mut symbols = BTreeSet::new();
            for id in &delta.added_identifiers {
                symbols.insert(id.as_str());
            }
            for id in &delta.removed_identifiers {
                symbols.insert(id.as_str());
            }
            let symbols_vec: Vec<String> = symbols.into_iter().map(String::from).collect();

            Self::render_module(&ModuleTemplateContext {
                path: &touch.path,
                author,
                date,
                raw_message,
                modified_symbols: &symbols_vec,
            })
        } else if touch.qualname == "<file_level>" {
            let mut sections = Vec::new();
            sections.push(format!(
                "File `{}` was modified by {} on {}.",
                touch.path, author, date
            ));

            let sanitized = sanitize_chunk_text(raw_message);
            if !sanitized.sanitized_text.is_empty() {
                sections.push(format!("Commit message: {}", sanitized.sanitized_text));
            }

            if !delta.added_identifiers.is_empty() {
                sections.push(format!("Added: {}", delta.added_identifiers.join(", ")));
            }

            if !delta.removed_identifiers.is_empty() {
                sections.push(format!("Removed: {}", delta.removed_identifiers.join(", ")));
            }

            let joined = sections.join("\n");
            SecretRedactor::redact(&joined)
        } else {
            Self::render_function(&FunctionTemplateContext {
                qualname: &touch.qualname,
                path: &touch.path,
                author,
                date,
                raw_message,
                added_identifiers: &delta.added_identifiers,
                removed_identifiers: &delta.removed_identifiers,
                signature: if touch.signature.is_empty() {
                    None
                } else {
                    Some(&touch.signature)
                },
                docstring: touch.docstring.as_deref(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CommitId, LineageId, ParseStatus};

    #[test]
    fn test_render_function_full() {
        let added = vec!["cache_store".to_string(), "session_token".to_string()];
        let removed = vec!["legacy_session".to_string()];
        let ctx = FunctionTemplateContext {
            qualname: "auth::authenticate_user",
            path: "src/auth.rs",
            author: "Alice",
            date: "2026-08-17",
            raw_message: "Refactor session handling\n\nFixes: #42\nSigned-off-by: Alice",
            added_identifiers: &added,
            removed_identifiers: &removed,
            signature: Some("pub fn authenticate_user(token: &str) -> Result<User>"),
            docstring: Some("Authenticate an incoming user session token."),
        };

        let rendered = D4TemplateRenderer::render_function(&ctx);
        let expected = "\
Function `auth::authenticate_user` in `src/auth.rs` was modified by Alice on 2026-08-17.
Commit message: Refactor session handling
Added: cache_store, session_token
Removed: legacy_session
Signature: pub fn authenticate_user(token: &str) -> Result<User>
Surrounding docstring: Authenticate an incoming user session token.";

        assert_eq!(rendered, expected);
    }

    #[test]
    fn test_render_function_empty_sections_omitted() {
        let ctx = FunctionTemplateContext {
            qualname: "calculate_tax",
            path: "finance/tax.py",
            author: "Bob",
            date: "2026-08-17",
            raw_message: "",
            added_identifiers: &[],
            removed_identifiers: &[],
            signature: None,
            docstring: None,
        };

        let rendered = D4TemplateRenderer::render_function(&ctx);
        assert_eq!(
            rendered,
            "Function `calculate_tax` in `finance/tax.py` was modified by Bob on 2026-08-17."
        );
    }

    #[test]
    fn test_render_touch_module() {
        let touch = FunctionTouch {
            commit_id: CommitId::new("abcd1234ef5678"),
            lineage_id: LineageId::new(1),
            path: "src/config.rs".to_string(),
            qualname: "<module>".to_string(),
            signature: String::new(),
            docstring: None,
            start_line: 1,
            end_line: 20,
            parse_status: ParseStatus::Ok,
        };

        let delta = IdentifierDelta {
            added_identifiers: vec!["DEFAULT_TIMEOUT".to_string()],
            removed_identifiers: vec!["OLD_TIMEOUT".to_string()],
        };

        let rendered = D4TemplateRenderer::render_touch(
            &touch,
            "Charlie",
            "2026-08-17",
            "Update timeout constants",
            &delta,
        );

        let expected = "\
Module `src/config.rs` was modified by Charlie on 2026-08-17.
Commit message: Update timeout constants
Modified top-level symbols: DEFAULT_TIMEOUT, OLD_TIMEOUT";

        assert_eq!(rendered, expected);
    }
}

//! Universal defense-in-depth secret redactor.
//!
//! Combines vendor prefix regex matching (GitHub, AWS, OpenAI, Slack, Private Keys, JWTs)
//! and Shannon entropy scanning ($H \ge 4.5$, length $\ge 24$) with code artifact exemptions
//! (UUIDs, Git SHAs) per D-13, D-14, D-15, and D-16.

pub mod entropy;
pub mod prefixes;

pub use entropy::{is_exempt, redact_high_entropy_tokens, shannon_entropy, MIN_ENTROPY_TOKEN_LEN, SHANNON_ENTROPY_THRESHOLD};
pub use prefixes::redact_known_prefixes;

use crate::sanitize::{sanitize_chunk_text, SanitizedChunk};

/// Universal defense-in-depth secret redactor facade.
pub struct SecretRedactor;

impl SecretRedactor {
    /// Redacts all known credentials and high-entropy secret tokens from text.
    ///
    /// Applies two sequential defense-in-depth stages:
    /// 1. Known vendor credential prefixes (`ghp_`, `AKIA`, `sk-`, `xoxb-`, Private Keys, JWTs)
    /// 2. Shannon entropy filtering on candidate tokens $\ge 24$ chars with $H \ge 4.5$ bits/char
    #[must_use]
    pub fn redact(text: &str) -> String {
        let after_prefixes = redact_known_prefixes(text);
        redact_high_entropy_tokens(&after_prefixes)
    }

    /// Redacts all secret tokens and applies leakage sanitization (stripping PR trailers and issue references).
    #[must_use]
    pub fn redact_and_sanitize(text: &str) -> SanitizedChunk {
        let redacted = Self::redact(text);
        sanitize_chunk_text(&redacted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_redactor_full_pipeline() {
        let raw = r#"
        Fixes: #1024
        let token = "ghp_1234567890abcdefghijklmnopqrstuvwxyzAB";
        let secret = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let commit = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4";
        let id = "550e8400-e29b-41d4-a716-446655440000";
        "#;

        let res = SecretRedactor::redact_and_sanitize(raw);
        assert!(res.sanitized_text.contains("[REDACTED_SECRET: github_token]"));
        assert!(res.sanitized_text.contains("[REDACTED_SECRET: high_entropy_string]"));
        assert!(res.sanitized_text.contains("e3b0c44298fc1c149afbf4c8996fb92427ae41e4"));
        assert!(res.sanitized_text.contains("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!res.sanitized_text.contains("#1024"));
    }
}

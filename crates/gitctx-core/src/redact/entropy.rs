//! Shannon entropy secret filtering with exemptions for code artifacts.
//!
//! Calculates Shannon entropy $H = -\sum p_i \log_2(p_i)$ on candidate tokens
//! ($\ge 24$ chars, $H \ge 4.5$) and redacts high-entropy secrets while exempting
//! UUIDs and Git commit hashes per D-14 and D-15.

use regex::Regex;
use std::sync::LazyLock;

/// Minimum character length for candidate high-entropy secret tokens.
pub const MIN_ENTROPY_TOKEN_LEN: usize = 24;

/// Shannon entropy threshold in bits per character.
pub const SHANNON_ENTROPY_THRESHOLD: f64 = 4.5;

static RE_CANDIDATE_TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[A-Za-z0-9+/=_-]{24,}").expect("valid candidate token regex")
});

static RE_UUID: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$")
        .expect("valid uuid regex")
});

static RE_HEX_HASH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[0-9a-fA-F]{40}$|^[0-9a-fA-F]{64}$").expect("valid hex hash regex")
});

/// Calculates the empirical Shannon entropy of a string in bits per character.
///
/// Formula: $H = -\sum_{i=1}^{n} p_i \log_2(p_i)$ over byte frequencies.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn shannon_entropy(token: &str) -> f64 {
    if token.is_empty() {
        return 0.0;
    }

    let mut counts = [0usize; 256];
    for &b in token.as_bytes() {
        counts[b as usize] += 1;
    }

    let len = token.len() as f64;
    let mut entropy = 0.0;

    for &count in &counts {
        if count > 0 {
            let p = (count as f64) / len;
            entropy -= p * p.log2();
        }
    }

    entropy
}

/// Checks if a candidate token is exempt from entropy redaction.
///
/// Exemptions:
/// - UUIDs (standard 36-char hyphenated UUIDs)
/// - Git commit SHAs and cryptographic hashes (40-char SHA1, 64-char SHA256 hex strings)
/// - Already redacted secret placeholders
#[must_use]
pub fn is_exempt(token: &str) -> bool {
    if token.contains("REDACTED_SECRET") {
        return true;
    }
    if RE_UUID.is_match(token) {
        return true;
    }
    if RE_HEX_HASH.is_match(token) {
        return true;
    }
    false
}

/// Redacts high-entropy secret tokens from text while preserving exempt identifiers.
#[must_use]
pub fn redact_high_entropy_tokens(text: &str) -> String {
    if text.len() < MIN_ENTROPY_TOKEN_LEN {
        return text.to_string();
    }

    RE_CANDIDATE_TOKEN
        .replace_all(text, |caps: &regex::Captures| {
            let matched = caps.get(0).map_or("", |m| m.as_str());
            if matched.len() < MIN_ENTROPY_TOKEN_LEN {
                return matched.to_string();
            }

            if is_exempt(matched) {
                return matched.to_string();
            }

            let entropy = shannon_entropy(matched);
            if entropy >= SHANNON_ENTROPY_THRESHOLD {
                "[REDACTED_SECRET: high_entropy_string]".to_string()
            } else {
                matched.to_string()
            }
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shannon_entropy_calculation() {
        // Uniform repetitive string has 0 entropy
        assert_eq!(shannon_entropy("AAAAAAAAAAAAAAAAAAAAAAAA"), 0.0);

        // Standard English identifier has moderate entropy (< 4.2)
        let identifier = "AuthenticationMiddlewareConfigurationService";
        assert!(shannon_entropy(identifier) < 4.5);

        // Random base64 string has high entropy (>= 4.5)
        let high_entropy = "dGhpcy1pcy1hLXZlcnktc2VjcmV0LWtleS0xMjM0NTY=";
        assert!(shannon_entropy(high_entropy) >= 4.5);
    }

    #[test]
    fn test_exempt_uuid() {
        let uuid = "123e4567-e89b-12d3-a456-426614174000";
        assert!(is_exempt(uuid));
        let text = format!("session_id = \"{uuid}\"");
        assert_eq!(redact_high_entropy_tokens(&text), text);
    }

    #[test]
    fn test_exempt_git_sha() {
        let sha = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4";
        assert!(is_exempt(sha));
        let text = format!("commit_sha = \"{sha}\"");
        assert_eq!(redact_high_entropy_tokens(&text), text);
    }

    #[test]
    fn test_redact_high_entropy_secret() {
        let secret = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let text = format!("secret_key: {secret}");
        let redacted = redact_high_entropy_tokens(&text);
        assert_eq!(
            redacted,
            "secret_key: [REDACTED_SECRET: high_entropy_string]"
        );
    }
}

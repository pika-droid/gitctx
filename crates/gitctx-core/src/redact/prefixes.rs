//! Known credential prefix matching and redaction rules.
//!
//! Redacts API keys and credentials matching known vendor prefix patterns per D-13 and D-15.

use regex::Regex;
use std::sync::LazyLock;

static RE_GITHUB_TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{40,})\b")
        .expect("valid regex")
});

static RE_AWS_ACCESS_KEY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:AKIA|ASIA|AROA|AIDA)[0-9A-Z]{16}\b").expect("valid regex")
});

static RE_OPENAI_KEY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bsk-(?:proj-|org-|admin-|svcacct-)?[A-Za-z0-9_-]{20,}\b").expect("valid regex")
});

static RE_SLACK_TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bxox[baprs]-[0-9A-Za-z-]{10,}\b").expect("valid regex")
});

static RE_PRIVATE_KEY_BLOCK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)-----BEGIN (?:[A-Z0-9_-]+ )?PRIVATE KEY-----.*?-----END (?:[A-Z0-9_-]+ )?PRIVATE KEY-----")
        .expect("valid regex")
});

static RE_PRIVATE_KEY_HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"-----BEGIN (?:[A-Z0-9_-]+ )?PRIVATE KEY-----").expect("valid regex")
});

static RE_JWT_TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b")
        .expect("valid regex")
});

/// Redacts known credential patterns using vendor prefix signatures.
#[must_use]
pub fn redact_known_prefixes(text: &str) -> String {
    let mut res = text.to_string();

    // 1. Private keys (blocks first, then standalone header)
    res = RE_PRIVATE_KEY_BLOCK
        .replace_all(&res, "[REDACTED_SECRET: private_key]")
        .into_owned();
    res = RE_PRIVATE_KEY_HEADER
        .replace_all(&res, "[REDACTED_SECRET: private_key]")
        .into_owned();

    // 2. GitHub tokens
    res = RE_GITHUB_TOKEN
        .replace_all(&res, "[REDACTED_SECRET: github_token]")
        .into_owned();

    // 3. AWS keys
    res = RE_AWS_ACCESS_KEY
        .replace_all(&res, "[REDACTED_SECRET: aws_access_key]")
        .into_owned();

    // 4. OpenAI keys
    res = RE_OPENAI_KEY
        .replace_all(&res, "[REDACTED_SECRET: openai_api_key]")
        .into_owned();

    // 5. Slack tokens
    res = RE_SLACK_TOKEN
        .replace_all(&res, "[REDACTED_SECRET: slack_token]")
        .into_owned();

    // 6. JWT tokens
    res = RE_JWT_TOKEN
        .replace_all(&res, "[REDACTED_SECRET: jwt_token]")
        .into_owned();

    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_github_token() {
        let input = "export GITHUB_TOKEN=ghp_1234567890abcdefghijklmnopqrstuvwxyzAB";
        let output = redact_known_prefixes(input);
        assert_eq!(output, "export GITHUB_TOKEN=[REDACTED_SECRET: github_token]");
    }

    #[test]
    fn test_redact_aws_key() {
        let input = "aws_key = \"AKIAIOSFODNN7EXAMPLE\"";
        let output = redact_known_prefixes(input);
        assert_eq!(output, "aws_key = \"[REDACTED_SECRET: aws_access_key]\"");
    }

    #[test]
    fn test_redact_openai_key() {
        let input = "OPENAI_API_KEY=sk-proj-abc123def456ghi789jkl012mno345pqr678stu901";
        let output = redact_known_prefixes(input);
        assert_eq!(output, "OPENAI_API_KEY=[REDACTED_SECRET: openai_api_key]");
    }

    #[test]
    fn test_redact_slack_token() {
        let input = "SLACK_BOT_TOKEN=xoxb-mock-sample-slack-bot-token-12345";
        let output = redact_known_prefixes(input);
        assert_eq!(output, "SLACK_BOT_TOKEN=[REDACTED_SECRET: slack_token]");
    }

    #[test]
    fn test_redact_private_key() {
        let input = "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA...\n-----END RSA PRIVATE KEY-----";
        let output = redact_known_prefixes(input);
        assert_eq!(output, "[REDACTED_SECRET: private_key]");
    }

    #[test]
    fn test_redact_jwt() {
        let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let output = redact_known_prefixes(input);
        assert_eq!(output, "Authorization: Bearer [REDACTED_SECRET: jwt_token]");
    }
}

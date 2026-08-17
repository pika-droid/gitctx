use gitctx_core::redact::{
    is_exempt, redact_high_entropy_tokens, redact_known_prefixes, shannon_entropy, SecretRedactor,
};


#[test]
fn test_github_token_redaction() {
    let inputs = [
        "ghp_1234567890abcdefghijklmnopqrstuvwxyzAB",
        "gho_abcdef1234567890abcdefghijklmnopqrstuv",
        "ghu_9876543210zyxwvutsrqponmlkjihgfedcba01",
        "ghs_11223344556677889900aabbccddeeff001122",
        "ghr_aabbccddeeff0011223344556677889900aabb",
        "github_pat_11A2B3C4D5E6F7G8H9I0J1K2L3M4N5O6P7Q8R9S0T1U2V3W4X5Y6Z7",
    ];

    for tok in inputs {
        let text = format!("auth_token = '{tok}'");
        let redacted = SecretRedactor::redact(&text);
        assert_eq!(
            redacted, "auth_token = '[REDACTED_SECRET: github_token]'",
            "Failed for token: {tok}"
        );
    }
}

#[test]
fn test_aws_access_key_redaction() {
    let text = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE and ASIAIOSFODNN7EXAMPLE";
    let redacted = SecretRedactor::redact(text);
    assert_eq!(
        redacted,
        "AWS_ACCESS_KEY_ID=[REDACTED_SECRET: aws_access_key] and [REDACTED_SECRET: aws_access_key]"
    );
}

#[test]
fn test_openai_api_key_redaction() {
    let legacy = "OPENAI_KEY=sk-1234567890abcdefghijklmnopqrstuvwx";
    let proj = "OPENAI_KEY=sk-proj-abc123def456ghi789jkl012mno345pqr678stu901vwx";
    assert_eq!(
        SecretRedactor::redact(legacy),
        "OPENAI_KEY=[REDACTED_SECRET: openai_api_key]"
    );
    assert_eq!(
        SecretRedactor::redact(proj),
        "OPENAI_KEY=[REDACTED_SECRET: openai_api_key]"
    );
}

#[test]
fn test_slack_token_redaction() {
    let bot_token = "SLACK_BOT=xoxb-mock-sample-slack-bot-token-12345";
    let user_token = "SLACK_USER=xoxp-mock-sample-slack-user-token-67890";
    assert_eq!(
        SecretRedactor::redact(bot_token),
        "SLACK_BOT=[REDACTED_SECRET: slack_token]"
    );
    assert_eq!(
        SecretRedactor::redact(user_token),
        "SLACK_USER=[REDACTED_SECRET: slack_token]"
    );
}

#[test]
fn test_private_key_redaction() {
    let pem = "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA0Y7f9...\n-----END RSA PRIVATE KEY-----";
    let single_header = "line with -----BEGIN OPENSSH PRIVATE KEY----- inside";

    assert_eq!(
        SecretRedactor::redact(pem),
        "[REDACTED_SECRET: private_key]"
    );
    assert_eq!(
        SecretRedactor::redact(single_header),
        "line with [REDACTED_SECRET: private_key] inside"
    );
}

#[test]
fn test_jwt_token_redaction() {
    let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    let text = format!("Bearer {jwt}");
    assert_eq!(
        SecretRedactor::redact(&text),
        "Bearer [REDACTED_SECRET: jwt_token]"
    );
}

#[test]
fn test_entropy_calculation_and_redaction() {
    let high_entropy = "c29tZS12ZXJ5LXJhbmRvbS1zZWNyZXQta2V5LXZhbHVlLTEyMzQ1Ng==";
    assert!(shannon_entropy(high_entropy) >= 4.5);

    let text = format!("api_secret = \"{high_entropy}\"");
    assert_eq!(
        redact_high_entropy_tokens(&text),
        "api_secret = \"[REDACTED_SECRET: high_entropy_string]\""
    );
    assert_eq!(
        SecretRedactor::redact(&text),
        "api_secret = \"[REDACTED_SECRET: high_entropy_string]\""
    );
}

#[test]
fn test_direct_prefix_redaction() {
    let raw = "ghp_1234567890abcdefghijklmnopqrstuvwxyzAB";
    assert_eq!(
        redact_known_prefixes(raw),
        "[REDACTED_SECRET: github_token]"
    );
}


#[test]
fn test_exemptions_uuid_and_git_sha() {
    let uuid_lower = "c4b4c73a-4a7b-4d4c-9f6e-6c8f9b7c8a1b";
    let uuid_upper = "C4B4C73A-4A7B-4D4C-9F6E-6C8F9B7C8A1B";
    let git_sha1 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4";
    let git_sha256 = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    assert!(is_exempt(uuid_lower));
    assert!(is_exempt(uuid_upper));
    assert!(is_exempt(git_sha1));
    assert!(is_exempt(git_sha256));

    let body = format!(
        "uuid1: {uuid_lower}, uuid2: {uuid_upper}, commit: {git_sha1}, full_sha: {git_sha256}"
    );
    let result = SecretRedactor::redact(&body);
    assert_eq!(result, body);
}

#[test]
fn test_no_false_positives_on_long_identifiers() {
    let long_code = "class AuthenticationMiddlewareConfigurationServiceManager {\n  fn handle_request() {}\n}";
    let result = SecretRedactor::redact(long_code);
    assert_eq!(result, long_code);
}

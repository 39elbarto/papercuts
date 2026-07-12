use crate::error::{AppError, AppResult};
use crate::policy::{SensitiveCategory, SensitivePolicy};
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

pub const POLICY_VERSION: u8 = 1;
pub const MAX_TEXT_BYTES: usize = 10_000;
pub const MAX_NOTE_BYTES: usize = 2_000;
pub const MAX_TAG_BYTES: usize = 64;
pub const MAX_TAGS: usize = 16;
pub const MAX_AGENT_BYTES: usize = 128;
pub const MAX_TOTAL_BYTES: usize = 16_384;
pub const PATTERN_COUNT: usize = 11;
const _: () = assert!(PATTERN_COUNT <= 128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensitiveField {
    Text,
    Tag,
    Agent,
    ResolutionNote,
}

impl SensitiveField {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Tag => "tag",
            Self::Agent => "agent",
            Self::ResolutionNote => "resolution_note",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentDecision {
    Clean,
    Warn,
    Override,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentPolicy {
    pub version: u8,
    pub mode: SensitivePolicy,
    pub decision: ContentDecision,
    pub categories: Vec<SensitiveCategory>,
    pub fields: Vec<SensitiveField>,
}

#[derive(Clone, Copy)]
struct ScanInput<'a> {
    field: SensitiveField,
    value: &'a str,
}

#[derive(Default)]
struct Findings {
    by_category: BTreeMap<SensitiveCategory, BTreeSet<SensitiveField>>,
}

impl Findings {
    fn record(&mut self, category: SensitiveCategory, field: SensitiveField) {
        self.by_category.entry(category).or_default().insert(field);
    }

    fn categories(&self) -> Vec<SensitiveCategory> {
        let mut categories: Vec<_> = self.by_category.keys().copied().collect();
        categories.sort_by_key(|category| category.as_str());
        categories
    }

    fn fields_for<'a>(
        &self,
        categories: impl IntoIterator<Item = &'a SensitiveCategory>,
    ) -> Vec<SensitiveField> {
        let mut fields: Vec<_> = categories
            .into_iter()
            .filter_map(|category| self.by_category.get(category))
            .flatten()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        fields.sort_by_key(|field| field.as_str());
        fields
    }
}

struct Catalog {
    private_key: Regex,
    authorization_header: Regex,
    credential_url: Regex,
    assignment: Regex,
    github_token: Regex,
    slack_token: Regex,
    stripe_secret_key: Regex,
    aws_access_key: Regex,
    email_address: Regex,
    filesystem_path: Regex,
    config_line: Regex,
}

impl Catalog {
    fn compile() -> Self {
        Self {
            private_key: pattern(
                r"(?i)-----[ ]*(?:BEGIN|END)[ ]+(?:(?:RSA|EC|DSA|OPENSSH|ENCRYPTED)[ ]+)?(?:PRIVATE KEY|PGP PRIVATE KEY BLOCK)[ ]*-----",
            ),
            authorization_header: pattern(
                r#"(?i)\bauthorization[\t ]*:[\t ]*(?:bearer|basic)[\t ]+(?P<value>[^\s"'\\]+)"#,
            ),
            credential_url: pattern(
                r"(?i)\b[a-z][a-z0-9+.-]{1,20}://(?P<user>[^/@:\s]+):(?P<password>[^/@\s]+)@",
            ),
            assignment: pattern(
                r#"(?im)(?:^|[,{;\n][\t ]*)[\t ]*(?:export[\t ]+)?["']?(?P<key>[A-Za-z][A-Za-z0-9_. -]{0,48})["']?[\t ]*(?:=|:)[\t ]*(?P<value>\$\{[A-Za-z_][A-Za-z0-9_]*\}|\$[A-Za-z_][A-Za-z0-9_]*|"[^"\r\n]*"|'[^'\r\n]*'|[^,;\r\n}]+)"#,
            ),
            github_token: pattern(
                r"\b(?:ghp_[A-Za-z0-9_]{8,255}|github_pat_[A-Za-z0-9_]{8,255}|(?:gho|ghu|ghs|ghr)_[A-Za-z0-9_]{8,255})\b",
            ),
            slack_token: pattern(r"\b(?:xoxb|xoxp|xwfp|xapp)-[A-Za-z0-9-]{8,255}\b"),
            stripe_secret_key: pattern(r"\b(?:sk|rk)_(?:test|live)_[A-Za-z0-9]{8,255}\b"),
            aws_access_key: pattern(r"\b(?:AKIA|ASIA)[A-Z0-9]{16}\b"),
            email_address: pattern(
                r"(?i)(?:^|[^A-Z0-9.!#$%&'*+/=?^_`{|}~:-])[A-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[A-Z0-9](?:[A-Z0-9-]{0,61}[A-Z0-9])?(?:\.[A-Z0-9](?:[A-Z0-9-]{0,61}[A-Z0-9])?)+\b",
            ),
            filesystem_path: pattern(
                r#"(?i)(?:^|[\s"'(=])(?:/(?:home|users|root|etc|var|tmp|opt|usr|srv|data)(?:/[^\s"',;)}]+)+|/(?:[^/\s"',;)}]+/){1,}[^/\s"',;)}]+|[A-Z]:\\[^\s"',;)}]+|\\\\[A-Z0-9._-]+\\[^\s"',;)}]+)"#,
            ),
            config_line: pattern(
                r#"(?i)^[\t ]*(?:export[\t ]+)?["']?[A-Z_][A-Z0-9_.-]*["']?[\t ]*(?:=|:)[\t ]*\S+"#,
            ),
        }
    }
}

fn pattern(source: &str) -> Regex {
    RegexBuilder::new(source)
        .size_limit(1 << 20)
        .build()
        .expect("the versioned sensitive-data pattern catalog must compile")
}

fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(Catalog::compile)
}

pub fn preflight_add(
    mode: SensitivePolicy,
    allowed: &[SensitiveCategory],
    text: &str,
    tags: &[String],
    agent: &str,
) -> AppResult<ContentPolicy> {
    validate_add_bounds(text, tags, agent)?;
    let mut inputs = Vec::with_capacity(tags.len() + 2);
    inputs.push(ScanInput {
        field: SensitiveField::Text,
        value: text,
    });
    inputs.extend(tags.iter().map(|tag| ScanInput {
        field: SensitiveField::Tag,
        value: tag,
    }));
    inputs.push(ScanInput {
        field: SensitiveField::Agent,
        value: agent,
    });
    evaluate(mode, allowed, &inputs)
}

pub fn preflight_resolve(
    mode: SensitivePolicy,
    allowed: &[SensitiveCategory],
    note: Option<&str>,
    agent: &str,
) -> AppResult<ContentPolicy> {
    validate_resolve_bounds(note, agent)?;
    let mut inputs = Vec::with_capacity(2);
    if let Some(note) = note {
        inputs.push(ScanInput {
            field: SensitiveField::ResolutionNote,
            value: note,
        });
    }
    inputs.push(ScanInput {
        field: SensitiveField::Agent,
        value: agent,
    });
    evaluate(mode, allowed, &inputs)
}

fn validate_add_bounds(text: &str, tags: &[String], agent: &str) -> AppResult<()> {
    validate_size("text", text.len(), MAX_TEXT_BYTES)?;
    if tags.len() > MAX_TAGS {
        return Err(AppError::invalid_input(
            format!("tag count is {}; the maximum is {MAX_TAGS}", tags.len()),
            format!("Pass at most {MAX_TAGS} --tag values."),
        ));
    }
    for tag in tags {
        validate_size("tag", tag.len(), MAX_TAG_BYTES)?;
    }
    validate_size("agent", agent.len(), MAX_AGENT_BYTES)?;
    let total = text.len() + tags.iter().map(String::len).sum::<usize>() + agent.len();
    validate_size("total scan payload", total, MAX_TOTAL_BYTES)
}

fn validate_resolve_bounds(note: Option<&str>, agent: &str) -> AppResult<()> {
    if let Some(note) = note {
        validate_size("resolution_note", note.len(), MAX_NOTE_BYTES)?;
    }
    validate_size("agent", agent.len(), MAX_AGENT_BYTES)?;
    let total = note.map_or(0, str::len) + agent.len();
    validate_size("total scan payload", total, MAX_TOTAL_BYTES)
}

fn validate_size(field: &str, actual: usize, maximum: usize) -> AppResult<()> {
    if actual <= maximum {
        return Ok(());
    }
    Err(AppError::invalid_input(
        format!("{field} is {actual} bytes; the maximum is {maximum}"),
        format!("Shorten {field} to at most {maximum} UTF-8 bytes."),
    ))
}

fn evaluate(
    mode: SensitivePolicy,
    allowed: &[SensitiveCategory],
    inputs: &[ScanInput<'_>],
) -> AppResult<ContentPolicy> {
    let findings = scan(inputs);
    let categories = findings.categories();
    let refusing: Vec<_> = categories
        .iter()
        .copied()
        .filter(|category| category.is_high_confidence() || mode == SensitivePolicy::Strict)
        .collect();
    let allowed: BTreeSet<_> = allowed.iter().copied().collect();
    let refusing_set: BTreeSet<_> = refusing.iter().copied().collect();
    let mut unused: Vec<_> = allowed.difference(&refusing_set).copied().collect();
    unused.sort_by_key(|category| category.as_str());
    if !unused.is_empty() {
        return Err(AppError::unused_sensitive_override(unused));
    }
    if !refusing_set.is_subset(&allowed) {
        let fields = findings.fields_for(&refusing);
        return Err(AppError::sensitive_input(mode, refusing, fields));
    }

    let decision = if !refusing.is_empty() {
        ContentDecision::Override
    } else if categories.is_empty() {
        ContentDecision::Clean
    } else {
        ContentDecision::Warn
    };
    let fields = findings.fields_for(&categories);
    Ok(ContentPolicy {
        version: POLICY_VERSION,
        mode,
        decision,
        categories,
        fields,
    })
}

fn scan(inputs: &[ScanInput<'_>]) -> Findings {
    let catalog = catalog();
    let mut findings = Findings::default();
    let mut aws_access_fields = BTreeSet::new();
    let mut aws_secret_fields = BTreeSet::new();

    for input in inputs {
        let value = input.value;
        if catalog.private_key.is_match(value) {
            findings.record(SensitiveCategory::PrivateKey, input.field);
        }
        for capture in catalog.authorization_header.captures_iter(value) {
            if capture
                .name("value")
                .is_some_and(|candidate| !is_exempt(candidate.as_str()))
            {
                findings.record(SensitiveCategory::AuthorizationHeader, input.field);
            }
        }
        for capture in catalog.credential_url.captures_iter(value) {
            let user = capture.name("user").map(|part| part.as_str());
            let password = capture.name("password").map(|part| part.as_str());
            if user.is_some_and(|part| !is_exempt(part))
                && password.is_some_and(|part| !is_exempt(part))
            {
                findings.record(SensitiveCategory::CredentialUrl, input.field);
            }
        }
        for capture in catalog.assignment.captures_iter(value) {
            let Some(key) = capture.name("key").map(|part| normalize_key(part.as_str())) else {
                continue;
            };
            let Some(candidate) = capture.name("value").map(|part| part.as_str()) else {
                continue;
            };
            if secret_key(&key) && !is_exempt(candidate) {
                findings.record(SensitiveCategory::SecretAssignment, input.field);
                if matches!(key.as_str(), "awssecretaccesskey" | "secretaccesskey") {
                    aws_secret_fields.insert(input.field);
                }
            }
            if personal_key(&key) && !is_exempt(candidate) {
                findings.record(SensitiveCategory::PersonalIdentifier, input.field);
            }
        }
        if catalog.github_token.is_match(value) {
            findings.record(SensitiveCategory::GithubToken, input.field);
        }
        if catalog.slack_token.is_match(value) {
            findings.record(SensitiveCategory::SlackToken, input.field);
        }
        if catalog.stripe_secret_key.is_match(value) {
            findings.record(SensitiveCategory::StripeSecretKey, input.field);
        }
        if catalog.aws_access_key.is_match(value) {
            aws_access_fields.insert(input.field);
        }
        if catalog.email_address.is_match(value) {
            findings.record(SensitiveCategory::EmailAddress, input.field);
        }
        if catalog.filesystem_path.is_match(value) {
            findings.record(SensitiveCategory::FilesystemPath, input.field);
        }
        if matches!(
            input.field,
            SensitiveField::Text | SensitiveField::ResolutionNote
        ) && value
            .lines()
            .filter(|line| catalog.config_line.is_match(line))
            .take(2)
            .count()
            >= 2
        {
            findings.record(SensitiveCategory::ConfigBlock, input.field);
        }
    }

    if !aws_access_fields.is_empty() && !aws_secret_fields.is_empty() {
        for field in aws_access_fields.union(&aws_secret_fields) {
            findings.record(SensitiveCategory::AwsCredentialPair, *field);
        }
    }
    findings
}

fn normalize_key(key: &str) -> String {
    key.bytes()
        .filter(u8::is_ascii_alphanumeric)
        .map(|byte| byte.to_ascii_lowercase() as char)
        .collect()
}

fn secret_key(key: &str) -> bool {
    matches!(
        key,
        "password"
            | "passwd"
            | "pwd"
            | "token"
            | "authtoken"
            | "accesstoken"
            | "refreshtoken"
            | "secret"
            | "apisecret"
            | "apikey"
            | "accesskey"
            | "clientsecret"
            | "privatekey"
            | "secretaccesskey"
            | "awssecretaccesskey"
    )
}

fn personal_key(key: &str) -> bool {
    matches!(
        key,
        "email"
            | "emailaddress"
            | "phone"
            | "phonenumber"
            | "customerid"
            | "patientid"
            | "userid"
            | "accountid"
    )
}

fn is_exempt(value: &str) -> bool {
    let candidate = trim_candidate(value);
    if variable_reference(candidate) {
        return true;
    }
    matches!(
        candidate.to_ascii_lowercase().as_str(),
        "example"
            | "placeholder"
            | "redacted"
            | "[redacted]"
            | "xxxxx"
            | "changeme"
            | "not-a-real-secret"
            | "test-token"
            | "dummy"
            | "your_token_here"
    )
}

fn trim_candidate(value: &str) -> &str {
    let mut candidate = value.trim_matches(|char: char| char.is_ascii_whitespace());
    if candidate.len() >= 2 {
        let first = candidate.as_bytes()[0];
        let last = candidate.as_bytes()[candidate.len() - 1];
        if matches!(first, b'\'' | b'"') && first == last {
            candidate = &candidate[1..candidate.len() - 1];
        }
    }
    candidate.trim_matches(|char: char| char.is_ascii_whitespace())
}

fn variable_reference(value: &str) -> bool {
    let bytes = value.as_bytes();
    if let Some(name) = bytes
        .strip_prefix(b"${")
        .and_then(|rest| rest.strip_suffix(b"}"))
    {
        return valid_variable_name(name);
    }
    bytes.strip_prefix(b"$").is_some_and(valid_variable_name)
}

fn valid_variable_name(name: &[u8]) -> bool {
    name.first()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
        && name
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn audit(mode: SensitivePolicy, value: &str) -> AppResult<ContentPolicy> {
        preflight_add(mode, &[], value, &[], "tester")
    }

    #[test]
    fn catalog_is_bounded_and_clean_is_deterministic() {
        let first = audit(SensitivePolicy::Balanced, "ordinary workflow friction").unwrap();
        let second = audit(SensitivePolicy::Balanced, "ordinary workflow friction").unwrap();
        assert_eq!(first, second);
        assert_eq!(first.decision, ContentDecision::Clean);
        assert!(first.categories.is_empty());
        assert!(first.fields.is_empty());
    }

    #[test]
    fn high_confidence_categories_refuse_without_echoing_values() {
        let cases = [
            (
                concat!("-----BEGIN OPENSSH ", "PRIVATE KEY-----"),
                "private_key",
            ),
            (
                "Authorization: Bearer syntheticBearerMaterial99",
                "authorization_header",
            ),
            (
                "https://synthetic-user:synthetic-pass@example.invalid/db",
                "credential_url",
            ),
            ("password=synthetic-literal-value", "secret_assignment"),
            ("ghp_syntheticTokenBody99", "github_token"),
            ("xoxb-synthetic-token-body-99", "slack_token"),
            (
                concat!("sk_", "test_syntheticKeyBody99"),
                "stripe_secret_key",
            ),
            (
                concat!(
                    "AKIA",
                    "ABCDEFGHIJKLMNOP\nAWS_SECRET_ACCESS_KEY=synthetic-secret-material"
                ),
                "aws_credential_pair",
            ),
        ];
        for (value, category) in cases {
            let error = audit(SensitivePolicy::Balanced, value).unwrap_err();
            assert_eq!(error.code, "sensitive_input", "{category}");
            let rendered = format!("{error:?}");
            assert!(rendered.contains(category), "{category}: {rendered}");
            assert!(!rendered.contains("synthetic"), "{category}: {rendered}");
        }
    }

    #[test]
    fn medium_categories_warn_in_balanced_and_refuse_in_strict() {
        let cases = [
            (
                "write alice@example.invalid",
                SensitiveCategory::EmailAddress,
            ),
            (
                "customer_id=customer-2048",
                SensitiveCategory::PersonalIdentifier,
            ),
            (
                "open /data/projects/example/file.txt",
                SensitiveCategory::FilesystemPath,
            ),
            ("alpha=one\nbeta=two", SensitiveCategory::ConfigBlock),
        ];
        for (value, category) in cases {
            let balanced = audit(SensitivePolicy::Balanced, value).unwrap();
            assert_eq!(balanced.decision, ContentDecision::Warn, "{category:?}");
            assert!(balanced.categories.contains(&category), "{category:?}");
            assert_eq!(
                audit(SensitivePolicy::Strict, value).unwrap_err().code,
                "sensitive_input",
                "{category:?}"
            );
        }
    }

    #[test]
    fn exact_placeholders_variables_and_benign_identifiers_are_allowed() {
        for value in [
            "password=example",
            "token='[redacted]'",
            "api_key=$TOKEN",
            "client_secret=${CLIENT_SECRET}",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "550e8400-e29b-41d4-a716-446655440000",
            "br-hardened-papercuts-fork-x30.9",
            "pk_live_publishableControl99",
            concat!("AKIA", "ABCDEFGHIJKLMNOP"),
        ] {
            let result = audit(SensitivePolicy::Strict, value).unwrap();
            assert_eq!(result.decision, ContentDecision::Clean, "{value}");
        }
        assert_eq!(
            audit(SensitivePolicy::Balanced, "token=production-test-token-123")
                .unwrap_err()
                .code,
            "sensitive_input"
        );
    }

    #[test]
    fn override_must_exactly_cover_refusing_categories() {
        let value = "ghp_syntheticTokenBody99 and alice@example.invalid";
        let partial = preflight_add(
            SensitivePolicy::Strict,
            &[SensitiveCategory::GithubToken],
            value,
            &[],
            "tester",
        )
        .unwrap_err();
        assert_eq!(partial.code, "sensitive_input");

        let full = preflight_add(
            SensitivePolicy::Strict,
            &[
                SensitiveCategory::EmailAddress,
                SensitiveCategory::GithubToken,
                SensitiveCategory::GithubToken,
            ],
            value,
            &[],
            "tester",
        )
        .unwrap();
        assert_eq!(full.decision, ContentDecision::Override);
        assert_eq!(
            full.categories,
            [
                SensitiveCategory::EmailAddress,
                SensitiveCategory::GithubToken
            ]
        );

        let unused = preflight_add(
            SensitivePolicy::Balanced,
            &[SensitiveCategory::EmailAddress],
            "alice@example.invalid",
            &[],
            "tester",
        )
        .unwrap_err();
        assert_eq!(unused.code, "invalid_input");
    }

    #[test]
    fn cross_field_aws_pair_and_field_order_are_stable() {
        let tags = vec!["AWS_SECRET_ACCESS_KEY=synthetic-secret-material".into()];
        let error = preflight_add(
            SensitivePolicy::Balanced,
            &[],
            concat!("AKIA", "ABCDEFGHIJKLMNOP"),
            &tags,
            "tester",
        )
        .unwrap_err();
        assert_eq!(error.code, "sensitive_input");
        assert_eq!(error.details["fields"], serde_json::json!(["tag", "text"]));
    }

    #[test]
    fn byte_and_count_bounds_are_exact() {
        assert!(audit(SensitivePolicy::Balanced, &"x".repeat(MAX_TEXT_BYTES)).is_ok());
        assert_eq!(
            audit(SensitivePolicy::Balanced, &"x".repeat(MAX_TEXT_BYTES + 1))
                .unwrap_err()
                .code,
            "invalid_input"
        );
        let tags = vec!["x".repeat(MAX_TAG_BYTES); MAX_TAGS];
        assert!(preflight_add(SensitivePolicy::Balanced, &[], "x", &tags, "a").is_ok());
        let too_many = vec!["x".into(); MAX_TAGS + 1];
        assert_eq!(
            preflight_add(SensitivePolicy::Balanced, &[], "x", &too_many, "a")
                .unwrap_err()
                .code,
            "invalid_input"
        );
        assert!(
            preflight_resolve(
                SensitivePolicy::Balanced,
                &[],
                Some(&"x".repeat(MAX_NOTE_BYTES)),
                &"a".repeat(MAX_AGENT_BYTES),
            )
            .is_ok()
        );
    }

    #[test]
    fn catalog_variants_cover_every_version_one_family() {
        let cases = [
            (concat!("-----BEGIN ", "PRIVATE KEY-----"), "private_key"),
            (concat!("-----END RSA ", "PRIVATE KEY-----"), "private_key"),
            (concat!("-----BEGIN EC ", "PRIVATE KEY-----"), "private_key"),
            (
                concat!("-----BEGIN OPENSSH ", "PRIVATE KEY-----"),
                "private_key",
            ),
            (
                concat!("-----BEGIN PGP ", "PRIVATE KEY BLOCK-----"),
                "private_key",
            ),
            (
                "aUtHoRiZaTiOn: bEaReR SyntheticBearerMaterial99",
                "authorization_header",
            ),
            (
                "Authorization: Basic SyntheticBasicMaterial99",
                "authorization_header",
            ),
            (
                "ssh://synthetic-user:synthetic-pass@example.invalid/repo",
                "credential_url",
            ),
            (
                "postgresql://synthetic-user:synthetic-pass@example.invalid/db",
                "credential_url",
            ),
            ("password: synthetic-literal-value", "secret_assignment"),
            (
                "export API_KEY=synthetic-literal-value",
                "secret_assignment",
            ),
            (
                r#"{"client_secret":"synthetic-literal-value"}"#,
                "secret_assignment",
            ),
            ("ghp_SyntheticTokenBody99", "github_token"),
            ("github_pat_SyntheticTokenBody99", "github_token"),
            ("gho_SyntheticTokenBody99", "github_token"),
            ("ghu_SyntheticTokenBody99", "github_token"),
            ("ghs_SyntheticTokenBody99", "github_token"),
            ("ghr_SyntheticTokenBody99", "github_token"),
            ("xoxb-synthetic-token-body-99", "slack_token"),
            ("xoxp-synthetic-token-body-99", "slack_token"),
            ("xwfp-synthetic-token-body-99", "slack_token"),
            ("xapp-synthetic-token-body-99", "slack_token"),
            (
                concat!("sk_", "test_SyntheticKeyBody99"),
                "stripe_secret_key",
            ),
            (
                concat!("sk_", "live_SyntheticKeyBody99"),
                "stripe_secret_key",
            ),
            (
                concat!("rk_", "test_SyntheticKeyBody99"),
                "stripe_secret_key",
            ),
            (
                concat!("rk_", "live_SyntheticKeyBody99"),
                "stripe_secret_key",
            ),
            ("email=alice@example.invalid", "personal_identifier"),
            ("phone=synthetic-555-0100", "personal_identifier"),
            ("customer_id=customer-2048", "personal_identifier"),
            ("patient_id=patient-2048", "personal_identifier"),
            ("user_id=user-2048", "personal_identifier"),
            ("account_id=account-2048", "personal_identifier"),
            ("open /home/example/project/file", "filesystem_path"),
            ("open /Users/example/project/file", "filesystem_path"),
            (r"open C:\Users\example\file.txt", "filesystem_path"),
            (r"open \\server\share\file.txt", "filesystem_path"),
            ("alpha=one\r\nbeta=two\r\n", "config_block"),
        ];
        for (fixture, (value, category)) in cases.into_iter().enumerate() {
            let result = audit(SensitivePolicy::Strict, value);
            assert!(
                result.is_err(),
                "fixture {fixture} for {category} was not detected"
            );
            let error = result.unwrap_err();
            assert_eq!(error.code, "sensitive_input", "{value}");
            assert!(
                error.details["categories"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|actual| actual == category),
                "{value}: {:?}",
                error.details
            );
        }
    }

    #[test]
    fn controls_and_documented_misses_remain_allowed() {
        for placeholder in [
            "example",
            "placeholder",
            "redacted",
            "[redacted]",
            "xxxxx",
            "changeme",
            "not-a-real-secret",
            "test-token",
            "dummy",
            "your_token_here",
        ] {
            for assignment in [
                format!("password={placeholder}"),
                format!("password: '{placeholder}'"),
                format!(r#"{{"password":"{placeholder}"}}"#),
            ] {
                assert_eq!(
                    audit(SensitivePolicy::Strict, &assignment)
                        .unwrap()
                        .decision,
                    ContentDecision::Clean,
                    "{assignment}"
                );
            }
        }
        for value in [
            "Authorization: Bearer example",
            "https://user:placeholder@example.invalid/db",
            "ghp_short",
            "xoxb-short",
            "sk_test_short",
            "pk_test_SyntheticPublishable99",
            "pk_live_SyntheticPublishable99",
            "relative/path/to/file",
            "/isolated",
            "alpha=one",
            "Z2hwX1N5bnRoZXRpY1Rva2VuQm9keTk5",
            "Unicode prose: секрет 🔐 éxample",
        ] {
            assert_eq!(
                audit(SensitivePolicy::Strict, value).unwrap().decision,
                ContentDecision::Clean,
                "{value}"
            );
        }
        let split = preflight_add(
            SensitivePolicy::Strict,
            &[],
            "ghp_",
            &["SyntheticTokenBody99".into()],
            "tester",
        )
        .unwrap();
        assert_eq!(split.decision, ContentDecision::Clean);
    }
}

use assert_cmd::Command;
use papercuts::commands::add::AddData;
use papercuts::commands::doctor::DoctorData;
use papercuts::commands::list::ListData;
use papercuts::commands::resolve::ResolveData;
use papercuts::error::{ERROR_CONTRACT, exit_code_map};
use papercuts::output::{ErrorEnvelope, SuccessEnvelope};
use papercuts::policy::SensitiveCategory;
use papercuts::sensitive::{ContentDecision, SensitiveField};
use papercuts::{
    CutRecord, ItemStatus, PathEncoding, RecordPathPolicy, ResolveRecord, Severity, compute_id,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::TempDir;

const NOW: &str = "2026-07-09T18:30:00.123456Z";

fn command() -> Command {
    let mut command = assert_cmd::cargo::cargo_bin_cmd!("papercuts");
    command
        .env("PAPERCUTS_NOW", NOW)
        .env("PAPERCUTS_PROFILE", "committed")
        .env_remove("PAPERCUTS_FILE")
        .env_remove("PAPERCUTS_AGENT")
        .env_remove("PAPERCUTS_READ_ONLY")
        .env_remove("PAPERCUTS_SENSITIVE_POLICY")
        .env_remove("PAPERCUTS_ALLOW_SENSITIVE")
        .env_remove("CLAUDECODE");
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("CODEX_")
            || key.to_string_lossy().starts_with("CURSOR_")
        {
            command.env_remove(key);
        }
    }
    command
}

fn run(args: &[&str]) -> std::process::Output {
    command().args(args).output().unwrap()
}

fn run_file(file: &Path, args: &[&str]) -> std::process::Output {
    command()
        .arg("--file")
        .arg(file)
        .args(args)
        .output()
        .unwrap()
}

fn temp_has_git_ancestor(temp: &TempDir) -> bool {
    temp.path()
        .ancestors()
        .any(|ancestor| ancestor.join(".git").exists())
}

fn init_git(path: &Path) {
    std::fs::create_dir_all(path).unwrap();
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("init")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git(path: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn success<T: DeserializeOwned>(output: &std::process::Output) -> SuccessEnvelope<T> {
    assert!(
        output.status.success(),
        "status={:?}\nstderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}

fn error(output: &std::process::Output, exit: i32, code: &str) -> ErrorEnvelope {
    assert_eq!(output.status.code(), Some(exit));
    assert!(output.stdout.is_empty());
    let envelope: ErrorEnvelope = serde_json::from_slice(&output.stderr).unwrap();
    assert!(!envelope.ok);
    assert_eq!(envelope.error.code, code);
    assert!(!envelope.error.suggested_fix.is_empty());
    assert_eq!(envelope.meta.contract, 2);
    envelope
}

fn add(file: &Path, text: &str) -> SuccessEnvelope<AddData> {
    let output = run_file(file, &["add", text, "--agent", "tester"]);
    success(&output)
}

#[test]
fn every_command_success_envelope_deserializes() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("cuts.jsonl");
    let added = add(&file, "first cut");
    assert!(added.ok);
    assert!(added.data.changed);
    assert_eq!(added.data.record.ts, "2026-07-09T18:30:00.123Z");
    assert_eq!(added.meta.agent_source.as_deref(), Some("flag"));

    let listed: SuccessEnvelope<ListData> = success(&run_file(&file, &["list", "--status", "all"]));
    assert_eq!(listed.data.count, 1);

    let resolved: SuccessEnvelope<ResolveData> = success(&run_file(
        &file,
        &[
            "resolve",
            &added.data.record.id,
            "--agent",
            "fixer",
            "--note",
            "fixed",
        ],
    ));
    assert!(resolved.data.changed);
    assert_eq!(resolved.data.record.status, ItemStatus::Resolved);
    assert_eq!(
        resolved.data.record.resolution.unwrap().note.as_deref(),
        Some("fixed")
    );

    let doctor_output = run_file(&file, &["doctor"]);
    let doctor: SuccessEnvelope<DoctorData> = success(&doctor_output);
    assert!(doctor.data.healthy);
    assert_eq!(doctor.data.checked_lines, 2);

    let schema: SuccessEnvelope<Value> = success(&run(&["schema"]));
    assert_eq!(schema.data["contract"], 2);
    assert_eq!(
        schema.data["implementation_status"]["storage_policy"],
        "implemented by x30.7"
    );
    assert!(
        schema.data["implementation_status"]["sensitive_preflight"]
            .as_str()
            .unwrap()
            .contains("implemented by x30.9")
    );
    assert_eq!(
        schema.data["implementation_status"]["adversarial_acceptance"],
        "implemented by x30.11"
    );
    assert_eq!(
        schema.data["metadata"]["private_add_resolve"]["contract"],
        2
    );
    let record_schema: SuccessEnvelope<Value> = success(&run(&["schema", "record"]));
    assert_eq!(
        record_schema.data["records"]["exact_examples"]["private_clean_cut"]["cwd"],
        "."
    );
    assert_eq!(schema.data["exit_codes"]["74"], "I/O error");
    assert_eq!(schema.data["commands"]["doctor"]["read_only"], true);

    let expected = serde_json::to_value(exit_code_map()).unwrap();
    assert_eq!(schema.data["exit_codes"], expected);
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct V01CutRecord {
    kind: String,
    id: String,
    ts: String,
    agent: String,
    text: String,
    tags: Vec<String>,
    severity: Severity,
    cwd: String,
    repo: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct V01ResolveRecord {
    kind: String,
    id: String,
    ts: String,
    agent: String,
    note: Option<String>,
}

#[test]
fn schema_contract2_is_static_exact_and_runtime_sourced() {
    let output = command()
        .env("PAPERCUTS_PROFILE", "rejected-profile-sentinel")
        .env("PAPERCUTS_READ_ONLY", "rejected-bool-sentinel")
        .env("PAPERCUTS_SENSITIVE_POLICY", "rejected-policy-sentinel")
        .env("PAPERCUTS_ALLOW_SENSITIVE", "rejected-gate-sentinel")
        .env("PAPERCUTS_AGENT", "rejected-agent-sentinel")
        .env("PAPERCUTS_NOW", "rejected-clock-sentinel")
        .env("PAPERCUTS_FILE", "/rejected/path/sentinel")
        .env_remove("HOME")
        .arg("schema")
        .output()
        .unwrap();
    let schema: SuccessEnvelope<Value> = success(&output);
    assert_eq!(schema.meta.contract, 2);
    assert_eq!(schema.meta.storage_profile, None);
    let rendered = String::from_utf8(output.stdout).unwrap();
    for sentinel in [
        "rejected-profile-sentinel",
        "rejected-bool-sentinel",
        "rejected-policy-sentinel",
        "rejected-gate-sentinel",
        "rejected-agent-sentinel",
        "rejected-clock-sentinel",
        "/rejected/path/sentinel",
    ] {
        assert!(!rendered.contains(sentinel));
    }

    let categories = serde_json::to_value(SensitiveCategory::ALL).unwrap();
    assert_eq!(schema.data["sensitive_content"]["categories"], categories);
    let catalog = schema.data["errors"]["catalog"].as_array().unwrap();
    assert_eq!(catalog.len(), ERROR_CONTRACT.len());
    for contract in ERROR_CONTRACT {
        let row = catalog
            .iter()
            .find(|row| row["code"] == contract.code)
            .unwrap();
        assert_eq!(row["exit_code"], contract.exit_code);
        assert_eq!(row["retryable"], contract.retryable());
        assert_eq!(row["description"], contract.description);
    }
    assert_eq!(
        schema.data["exit_codes"],
        serde_json::to_value(exit_code_map()).unwrap()
    );
    assert_eq!(schema.data["commands"]["add"]["appends"], true);
    assert_eq!(schema.data["commands"]["list"]["may_create"], false);
    assert_eq!(
        schema.data["plaintext_exceptions"]
            .as_array()
            .unwrap()
            .len(),
        3
    );

    let private_value = schema.data["records"]["exact_examples"]["private_clean_cut"].clone();
    let private: CutRecord = serde_json::from_value(private_value.clone()).unwrap();
    assert_eq!(private.id, "pc_94f5df71022d");
    assert_eq!(
        compute_id(
            &private.ts,
            &private.agent,
            &private.text,
            private.severity,
            &private.tags,
        ),
        private.id
    );
    let committed: CutRecord = serde_json::from_value(
        schema.data["records"]["exact_examples"]["committed_clean_cut"].clone(),
    )
    .unwrap();
    assert_eq!(committed.id, private.id);
    let resolve_value = schema.data["records"]["exact_examples"]["resolve_event"].clone();
    let resolve: ResolveRecord = serde_json::from_value(resolve_value.clone()).unwrap();
    assert_eq!(resolve.id, private.id);

    let v01_private: V01CutRecord = serde_json::from_value(private_value).unwrap();
    let v01_resolve: V01ResolveRecord = serde_json::from_value(resolve_value).unwrap();
    assert_eq!(v01_private.cwd, ".");
    assert_eq!(v01_private.repo, None);
    assert_eq!(v01_resolve.id, private.id);

    for target in ["record", "error", "exit-codes"] {
        let targeted: SuccessEnvelope<Value> = success(&run(&["schema", target]));
        assert_eq!(targeted.data["contract"], 2);
    }
}

#[test]
fn legacy_unscanned_warnings_and_content_policy_mismatch_are_explicit() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("mixed.jsonl");
    let ts = "2026-07-12T00:00:00.000Z";
    let legacy_id = compute_id(ts, "tester", "legacy", Severity::Minor, &[]);
    let invalid_id = compute_id(ts, "tester", "invalid audit", Severity::Minor, &[]);
    let legacy = json!({"kind":"cut","id":legacy_id,"ts":ts,"agent":"tester","text":"legacy","tags":[],"severity":"minor","cwd":"/legacy/source","repo":null});
    let invalid = json!({"kind":"cut","id":invalid_id,"ts":ts,"agent":"tester","text":"invalid audit","tags":[],"severity":"minor","cwd":".","repo":null,"path_policy":"omitted","path_encoding":"omitted","content_policy":{"version":2,"mode":"balanced","decision":"clean","categories":[],"fields":[]}});
    std::fs::write(&file, format!("{legacy}\n{invalid}\n")).unwrap();

    let listed: SuccessEnvelope<ListData> = success(
        &command()
            .arg("--profile")
            .arg("private")
            .arg("--file")
            .arg(&file)
            .args(["list", "--status", "all"])
            .output()
            .unwrap(),
    );
    assert!(
        listed
            .meta
            .warnings
            .iter()
            .any(|warning| warning == "legacy_unscanned_records:1")
    );
    assert!(
        listed
            .data
            .items
            .iter()
            .any(|item| item.cut.content_policy.is_none())
    );

    let doctor_output = command()
        .arg("--profile")
        .arg("private")
        .arg("--file")
        .arg(&file)
        .arg("doctor")
        .output()
        .unwrap();
    assert_eq!(doctor_output.status.code(), Some(1));
    assert!(doctor_output.stderr.is_empty());
    let doctor: SuccessEnvelope<DoctorData> =
        serde_json::from_slice(&doctor_output.stdout).unwrap();
    assert!(
        doctor
            .data
            .findings
            .iter()
            .any(|finding| finding.kind == "content_policy_mismatch")
    );
    assert!(
        doctor
            .meta
            .warnings
            .iter()
            .any(|warning| warning == "legacy_unscanned_records:1")
    );
}

#[test]
fn add_stdin_validation_duplicate_and_exact_id() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("cuts.jsonl");
    let mut stdin = command();
    let output = stdin
        .arg("--file")
        .arg(&file)
        .args([
            "add",
            "-",
            "--agent",
            "tester",
            "--severity",
            "major",
            "--tag",
            "z",
            "--tag",
            "a",
        ])
        .write_stdin("ouch\n")
        .output()
        .unwrap();
    let first: SuccessEnvelope<AddData> = success(&output);
    assert_eq!(first.data.record.id, "pc_6d26611bad4c");
    assert_eq!(first.data.record.tags, ["a", "z"]);

    let second: SuccessEnvelope<AddData> = success(
        &command()
            .arg("--file")
            .arg(&file)
            .args([
                "add",
                "ouch",
                "--agent",
                "tester",
                "--severity",
                "major",
                "--tag",
                "z",
                "--tag",
                "a",
            ])
            .output()
            .unwrap(),
    );
    assert!(!second.data.changed);
    assert!(
        second
            .meta
            .warnings
            .iter()
            .any(|warning| warning == "duplicate papercut; existing record returned")
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap().lines().count(), 1);

    let blank = command()
        .arg("--file")
        .arg(&file)
        .arg("add")
        .write_stdin(" \n")
        .output()
        .unwrap();
    error(&blank, 65, "invalid_input");
    let large = "x".repeat(10_001);
    error(&run_file(&file, &["add", &large]), 65, "invalid_input");
}

#[test]
fn list_filters_sorts_limits_since_and_markdown() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("cuts.jsonl");
    let cases = [
        ("2026-07-01T00:00:00Z", "old blocker", "blocker", "ops"),
        ("2026-07-09T17:00:00Z", "new minor", "minor", "shell"),
        ("2026-07-09T18:00:00Z", "new major", "major", "ops"),
    ];
    for (now, text, severity, tag) in cases {
        let output = command()
            .env("PAPERCUTS_NOW", now)
            .arg("--file")
            .arg(&file)
            .args([
                "add",
                text,
                "--agent",
                "tester",
                "--severity",
                severity,
                "--tag",
                tag,
            ])
            .output()
            .unwrap();
        success::<AddData>(&output);
    }
    let limited: SuccessEnvelope<ListData> = success(&run_file(&file, &["list", "--limit", "1"]));
    assert_eq!(limited.data.items[0].cut.text, "old blocker");
    assert_eq!(limited.data.total, 3);
    assert!(limited.data.truncated);

    let since: SuccessEnvelope<ListData> = success(
        &command()
            .env("PAPERCUTS_NOW", "2026-07-09T19:00:00Z")
            .arg("--file")
            .arg(&file)
            .args(["list", "--since", "2h", "--tag", "ops"])
            .output()
            .unwrap(),
    );
    assert_eq!(since.data.items.len(), 1);
    assert_eq!(since.data.items[0].cut.text, "new major");

    let markdown = run_file(&file, &["list", "--format", "md", "--severity", "major"]);
    assert!(markdown.status.success());
    assert!(markdown.stderr.is_empty());
    let markdown = String::from_utf8(markdown.stdout).unwrap();
    assert!(markdown.starts_with("## Major\n"));
    assert!(markdown.contains("new major — tester"));
    assert!(serde_json::from_str::<Value>(&markdown).is_err());
    error(
        &run_file(&file, &["list", "--since", "2026-07-09"]),
        2,
        "invalid_argument",
    );
}

#[test]
fn list_sorts_rfc3339_offsets_by_instant_not_text() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("offsets.jsonl");
    let earlier = json!({"kind":"cut","id":"pc_111111111111","ts":"2026-07-09T10:00:00+02:00","agent":"a","text":"earlier","tags":[],"severity":"minor","cwd":"/tmp","repo":null});
    let later = json!({"kind":"cut","id":"pc_222222222222","ts":"2026-07-09T09:00:00Z","agent":"a","text":"later","tags":[],"severity":"minor","cwd":"/tmp","repo":null});
    std::fs::write(&file, format!("{earlier}\n{later}\n")).unwrap();
    let listed: SuccessEnvelope<ListData> = success(&run_file(&file, &["list"]));
    assert_eq!(listed.data.items[0].cut.text, "later");
}

#[test]
fn resolve_prefix_errors_and_idempotence_are_structured() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("cuts.jsonl");
    let added = add(&file, "resolve me");
    let id = added.data.record.id;
    let prefix = &id[3..7];
    let first: SuccessEnvelope<ResolveData> = success(&run_file(
        &file,
        &["resolve", &prefix.to_ascii_uppercase(), "--agent", "fixer"],
    ));
    assert!(first.data.changed);
    let second: SuccessEnvelope<ResolveData> =
        success(&run_file(&file, &["resolve", &id, "--agent", "fixer"]));
    assert!(!second.data.changed);
    assert!(
        second
            .meta
            .warnings
            .iter()
            .any(|warning| warning == "already resolved")
    );

    error(&run_file(&file, &["resolve", "abc"]), 2, "invalid_argument");
    error(&run_file(&file, &["resolve", "deadbeef"]), 66, "not_found");

    let ambiguous = temp.path().join("ambiguous.jsonl");
    let lines = ["pc_abcd00000000", "pc_abcd11111111"]
        .map(|id| {
            json!({"kind":"cut","id":id,"ts":"2026-07-09T00:00:00.000Z","agent":"a","text":id,"tags":[],"severity":"minor","cwd":"/tmp","repo":null}).to_string()
        })
        .join("\n")
        + "\n";
    std::fs::write(&ambiguous, lines).unwrap();
    let envelope = error(
        &run_file(&ambiguous, &["resolve", "abcd"]),
        65,
        "ambiguous_id",
    );
    assert_eq!(
        envelope.error.details["candidates"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn structured_error_exit_matrix_and_help_exceptions() {
    let temp = TempDir::new().unwrap();
    let missing = temp.path().join("missing.jsonl");
    error(&run_file(&missing, &["list"]), 66, "not_found");
    error(&run(&["list", "--format", "jsonl"]), 2, "invalid_argument");
    let schema_ignores_clock: SuccessEnvelope<Value> = success(
        &command()
            .env("PAPERCUTS_NOW", "not-a-time")
            .args(["schema"])
            .output()
            .unwrap(),
    );
    assert_eq!(schema_ignores_clock.data["contract"], 2);
    error(
        &run_file(&missing, &["add", " ", "--agent", "tester"]),
        65,
        "invalid_input",
    );
    let invalid_utf8 = command()
        .arg("--file")
        .arg(&missing)
        .args(["add", "-", "--agent", "tester"])
        .write_stdin(vec![0xff])
        .output()
        .unwrap();
    error(&invalid_utf8, 65, "invalid_input");
    let directory_error = run_file(temp.path(), &["list"]);
    error(&directory_error, 74, "io_error");

    let help = run(&["--help"]);
    assert!(help.status.success());
    assert!(help.stderr.is_empty());
    assert!(String::from_utf8_lossy(&help.stdout).contains("Usage:"));
    let version = run(&["--version"]);
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8_lossy(&version.stdout),
        "papercuts 0.1.0\n"
    );
}

#[test]
fn rejected_values_never_echo_and_suggested_fixes_never_weaken_policy() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("cuts.jsonl");
    add(&file, "existing");
    let cases = [
        (
            "rejected-profile-argv-sentinel",
            command()
                .args(["--profile", "rejected-profile-argv-sentinel", "list"])
                .output()
                .unwrap(),
        ),
        (
            "rejected-profile-env-sentinel",
            command()
                .env("PAPERCUTS_PROFILE", "rejected-profile-env-sentinel")
                .arg("list")
                .output()
                .unwrap(),
        ),
        (
            "rejected-since-sentinel",
            command()
                .arg("--file")
                .arg(&file)
                .args(["list", "--since", "rejected-since-sentinel"])
                .output()
                .unwrap(),
        ),
        (
            "rejected-id-sentinel",
            command()
                .arg("--file")
                .arg(&file)
                .args(["resolve", "rejected-id-sentinel"])
                .output()
                .unwrap(),
        ),
        (
            "rejected-wildcard-sentinel",
            command()
                .arg("--file")
                .arg(&file)
                .args([
                    "add",
                    "x",
                    "--allow-sensitive",
                    "rejected-wildcard-sentinel",
                ])
                .output()
                .unwrap(),
        ),
    ];
    for (sentinel, output) in cases {
        assert!(!output.status.success(), "{sentinel}");
        assert!(output.stdout.is_empty(), "{sentinel}");
        let stderr = String::from_utf8(output.stderr.clone()).unwrap();
        assert!(!stderr.contains(sentinel), "{sentinel}: {stderr}");
        let envelope: ErrorEnvelope = serde_json::from_slice(&output.stderr).unwrap();
        let fix = envelope.error.suggested_fix.to_ascii_lowercase();
        for forbidden in [
            "--profile committed",
            "papercuts_allow_sensitive",
            "--allow-sensitive",
            "disable read-only",
            "read_only=false",
        ] {
            assert!(!fix.contains(forbidden), "{sentinel}: {fix}");
        }
    }
}

#[test]
fn agent_resolution_order_and_sources_are_pinned() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("unused.jsonl");
    let invoke = |command: &mut Command| -> SuccessEnvelope<AddData> {
        success(
            &command
                .arg("--file")
                .arg(&file)
                .args(["add", "x", "--dry-run"])
                .output()
                .unwrap(),
        )
    };

    let default = invoke(&mut command());
    assert_eq!(default.data.record.agent, "unknown");
    assert_eq!(default.meta.agent_source.as_deref(), Some("default"));

    let claude = invoke(command().env("CLAUDECODE", "1"));
    assert_eq!(claude.data.record.agent, "claude-code");
    assert_eq!(claude.meta.agent_source.as_deref(), Some("detected"));

    let codex = invoke(command().env("CODEX_TEST", "1").env("CURSOR_TEST", "1"));
    assert_eq!(codex.data.record.agent, "codex");

    let cursor = invoke(command().env("CURSOR_TEST", "1"));
    assert_eq!(cursor.data.record.agent, "cursor");

    let env = invoke(
        command()
            .env("PAPERCUTS_AGENT", "from-env")
            .env("CLAUDECODE", "1"),
    );
    assert_eq!(env.data.record.agent, "from-env");
    assert_eq!(env.meta.agent_source.as_deref(), Some("env"));

    let flag: SuccessEnvelope<AddData> = success(
        &command()
            .env("PAPERCUTS_AGENT", "from-env")
            .arg("--file")
            .arg(&file)
            .args(["add", "x", "--agent", "from-flag", "--dry-run"])
            .output()
            .unwrap(),
    );
    assert_eq!(flag.data.record.agent, "from-flag");
    assert_eq!(flag.meta.agent_source.as_deref(), Some("flag"));
    assert!(!file.exists());
}

#[test]
fn mutation_dry_runs_do_not_write() {
    let temp = TempDir::new().unwrap();
    let dry_add = temp.path().join("nested/cuts.jsonl");
    let added: SuccessEnvelope<AddData> = success(&run_file(
        &dry_add,
        &["add", "preview", "--agent", "a", "--dry-run"],
    ));
    assert!(!added.data.changed);
    assert!(!dry_add.exists());

    let file = temp.path().join("cuts.jsonl");
    let id = add(&file, "resolve preview").data.record.id;
    let before = std::fs::read(&file).unwrap();
    let resolved: SuccessEnvelope<ResolveData> = success(&run_file(
        &file,
        &["resolve", &id, "--agent", "a", "--dry-run"],
    ));
    assert!(!resolved.data.changed);
    assert_eq!(resolved.data.record.status, ItemStatus::Resolved);
    assert_eq!(std::fs::read(&file).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn permission_denied_is_exit_77() {
    use std::os::unix::fs::PermissionsExt;
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("cuts.jsonl");
    std::fs::write(&file, "{}\n").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).unwrap();
    let output = run_file(&file, &["list"]);
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    error(&output, 77, "permission_denied");
}

#[test]
fn lock_timeout_is_retryable_exit_75() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("customer-secret-lock.jsonl");
    add(&file, "locked");
    let locked = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&file)
        .unwrap();
    locked.lock().unwrap();
    let output = command()
        .arg("--profile")
        .arg("private")
        .arg("--file")
        .arg(&file)
        .arg("list")
        .output()
        .unwrap();
    locked.unlock().unwrap();
    assert!(!String::from_utf8_lossy(&output.stderr).contains("customer-secret"));
    let envelope = error(&output, 75, "lock_timeout");
    assert!(envelope.error.retryable);
    assert_eq!(envelope.error.details["location"], "explicit_journal");
    assert_eq!(envelope.meta.storage_profile.as_deref(), Some("private"));
    assert_eq!(envelope.meta.file, None);
}

#[test]
fn doctor_reports_all_core_findings_and_recomputed_ids() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("cuts.jsonl");
    let good = add(&file, "valid").data.record;
    let good_line = std::fs::read_to_string(&file).unwrap();
    let bad_id = json!({"kind":"cut","id":"pc_000000000000","ts":good.ts,"agent":"tester","text":"bad","tags":[],"severity":"minor","cwd":"/tmp","repo":null});
    let mut writer = OpenOptions::new().append(true).open(&file).unwrap();
    writeln!(writer, "{good_line}{}", bad_id).unwrap();
    writeln!(writer, "{{\"kind\":\"future\"}}").unwrap();
    writeln!(writer, "{{\"kind\":\"resolve\",\"id\":\"pc_deadbeef0000\",\"ts\":\"2026-07-09T00:00:00.000Z\",\"agent\":\"a\",\"note\":null}}").unwrap();
    writeln!(writer, "<<<<<<< HEAD").unwrap();
    write!(writer, "{{\"kind\":").unwrap();
    drop(writer);
    let output = run_file(&file, &["doctor"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let envelope: SuccessEnvelope<DoctorData> = serde_json::from_slice(&output.stdout).unwrap();
    let kinds: Vec<_> = envelope
        .data
        .findings
        .iter()
        .map(|finding| finding.kind.as_str())
        .collect();
    for kind in [
        "duplicate_cut",
        "id_conflict",
        "unknown_kind",
        "orphan_resolve",
        "conflict_marker",
        "torn_line",
    ] {
        assert!(kinds.contains(&kind), "missing {kind}: {kinds:?}");
    }
    assert!(!envelope.data.healthy);
}

#[test]
fn torn_tail_self_heals_on_add() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("cuts.jsonl");
    std::fs::write(&file, b"{\"kind\":\"cut\"").unwrap();
    let added = add(&file, "after tear");
    assert!(added.data.changed);
    let bytes = std::fs::read(&file).unwrap();
    assert!(bytes.ends_with(b"\n"));
    assert_eq!(bytes.split(|byte| *byte == b'\n').count(), 3);
    let listed: SuccessEnvelope<ListData> = success(&run_file(&file, &["list"]));
    assert_eq!(listed.data.items.len(), 1);
    assert_eq!(listed.data.items[0].cut.text, "after tear");
    assert!(
        listed
            .meta
            .warnings
            .iter()
            .any(|warning| warning.contains("malformed"))
    );
}

#[test]
fn doctor_finding_counts_match_fold_bytes_warning_counts() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("cuts.jsonl");
    let valid_id = compute_id(
        "2026-07-09T00:00:00.000Z",
        "a",
        "valid",
        Severity::Minor,
        &[],
    );
    let malformed = json!({
        "kind": "cut",
        "id": "pc_000000000000",
        "ts": "not-a-time",
        "agent": "a",
        "text": "malformed",
        "tags": [],
        "severity": "minor",
        "cwd": "/tmp",
        "repo": null
    })
    .to_string();
    let valid = json!({
        "kind": "cut",
        "id": valid_id,
        "ts": "2026-07-09T00:00:00.000Z",
        "agent": "a",
        "text": "valid",
        "tags": [],
        "severity": "minor",
        "cwd": "/tmp",
        "repo": null
    })
    .to_string();
    let orphan = json!({
        "kind": "resolve",
        "id": "pc_deadbeef0000",
        "ts": "2026-07-09T00:00:00.000Z",
        "agent": "a",
        "note": null
    })
    .to_string();
    let unknown = json!({"kind": "future"}).to_string();
    let fixture = format!("{malformed}\n{valid}\n{orphan}\n{valid}\n{unknown}\n{{\"kind\":");
    std::fs::write(&file, fixture).unwrap();

    let folded = papercuts::store::fold_bytes(&std::fs::read(&file).unwrap());
    let doctor_output = run_file(&file, &["doctor"]);
    assert_eq!(doctor_output.status.code(), Some(1));
    assert!(doctor_output.stderr.is_empty());
    let doctor: SuccessEnvelope<DoctorData> =
        serde_json::from_slice(&doctor_output.stdout).unwrap();

    let fold_counts = fold_warning_counts(&folded.warnings);
    let doctor_counts = doctor_finding_counts(&doctor.data.findings);
    let expected: HashMap<String, usize> = [
        ("malformed", 1),
        ("unknown", 1),
        ("duplicate_cut", 1),
        ("orphan_resolve", 1),
        ("torn", 1),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    assert_eq!(
        fold_counts, expected,
        "fold warnings: {:?}",
        folded.warnings
    );
    assert_eq!(
        doctor_counts, expected,
        "doctor findings: {:?}",
        doctor.data.findings
    );
}

fn fold_warning_counts(warnings: &[String]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for warning in warnings {
        let parts: Vec<_> = warning.splitn(3, ' ').collect();
        let count: usize = parts[1].parse().unwrap();
        let label = parts[2].trim_end_matches('s');
        let key = if label.starts_with("malformed line") {
            "malformed"
        } else if label.starts_with("torn final line") {
            "torn"
        } else if label.starts_with("unknown event") {
            "unknown"
        } else if label.starts_with("duplicate cut") {
            "duplicate_cut"
        } else if label.starts_with("duplicate resolve") {
            "duplicate_resolve"
        } else if label.starts_with("orphan resolve") {
            "orphan_resolve"
        } else {
            panic!("unknown fold warning label: {label}")
        };
        counts.insert(key.to_string(), count);
    }
    counts
}

fn doctor_finding_counts(
    findings: &[papercuts::commands::doctor::Finding],
) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for finding in findings {
        let key = match finding.kind.as_str() {
            "malformed" => "malformed",
            "torn_line" => "torn",
            "unknown_kind" => "unknown",
            "duplicate_cut" => "duplicate_cut",
            "orphan_resolve" => "orphan_resolve",
            _ => continue,
        };
        *counts.entry(key.to_string()).or_insert(0) += 1;
    }
    counts
}

#[test]
fn discovery_precedence_virtual_empty_and_valid_git_root() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("repo");
    let nested = root.join("a/b");
    std::fs::create_dir_all(&nested).unwrap();
    assert!(
        std::process::Command::new("git")
            .arg("-C")
            .arg(&root)
            .arg("init")
            .output()
            .unwrap()
            .status
            .success()
    );
    let env_file = temp.path().join("env.jsonl");
    let flag_file = temp.path().join("flag.jsonl");

    let walk: SuccessEnvelope<AddData> = success(
        &command()
            .current_dir(&nested)
            .args(["add", "x", "--agent", "a", "--dry-run"])
            .output()
            .unwrap(),
    );
    let canonical_root = root.canonicalize().unwrap();
    assert_eq!(
        walk.meta.file.as_deref(),
        Some(canonical_root.join(".papercuts.jsonl").to_str().unwrap())
    );
    let empty_env: SuccessEnvelope<AddData> = success(
        &command()
            .current_dir(&nested)
            .env("PAPERCUTS_FILE", "")
            .args(["add", "x", "--agent", "a", "--dry-run"])
            .output()
            .unwrap(),
    );
    assert_eq!(empty_env.meta.file, walk.meta.file);

    let env: SuccessEnvelope<AddData> = success(
        &command()
            .current_dir(&nested)
            .env("PAPERCUTS_FILE", &env_file)
            .args(["add", "x", "--agent", "a", "--dry-run"])
            .output()
            .unwrap(),
    );
    assert_eq!(env.meta.file.as_deref(), Some(env_file.to_str().unwrap()));

    let flag: SuccessEnvelope<AddData> = success(
        &command()
            .current_dir(&nested)
            .env("PAPERCUTS_FILE", &env_file)
            .arg("--file")
            .arg(&flag_file)
            .args(["add", "x", "--agent", "a", "--dry-run"])
            .output()
            .unwrap(),
    );
    assert_eq!(flag.meta.file.as_deref(), Some(flag_file.to_str().unwrap()));

    let empty: SuccessEnvelope<ListData> =
        success(&command().current_dir(&nested).arg("list").output().unwrap());
    assert!(empty.data.items.is_empty());
    assert!(
        empty
            .meta
            .warnings
            .iter()
            .any(|warning| warning.contains("no papercuts file"))
    );

    if !temp_has_git_ancestor(&temp) {
        let outside = temp.path().join("outside");
        let home = temp.path().join("home");
        std::fs::create_dir_all(&outside).unwrap();
        let home_result: SuccessEnvelope<AddData> = success(
            &command()
                .current_dir(&outside)
                .env("HOME", &home)
                .args(["add", "x", "--agent", "a", "--dry-run"])
                .output()
                .unwrap(),
        );
        assert_eq!(
            home_result.meta.file.as_deref(),
            Some(home.join(".papercuts/log.jsonl").to_str().unwrap())
        );
        assert!(
            !home.exists(),
            "dry run must not create the home fallback directory"
        );
        let no_home = command()
            .current_dir(&outside)
            .env_remove("HOME")
            .arg("list")
            .output()
            .unwrap();
        error(&no_home, 78, "config_error");
    } else {
        eprintln!(
            "skipping home-fallback assertions because the temporary directory is inside a git checkout"
        );
    }
}

#[test]
fn fixed_clock_fresh_state_is_byte_deterministic_and_retry_is_duplicate_safe() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("cuts.jsonl");
    let first = run_file(&file, &["add", "same", "--agent", "tester"]);
    assert!(first.status.success());
    std::fs::remove_file(&file).unwrap();
    let fresh = run_file(&file, &["add", "same", "--agent", "tester"]);
    assert_eq!(first.stdout, fresh.stdout);
    let retry: SuccessEnvelope<AddData> =
        success(&run_file(&file, &["add", "same", "--agent", "tester"]));
    assert!(!retry.data.changed);
}

#[test]
fn eight_way_distinct_add_race_loses_no_lines() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("cuts.jsonl");
    let barrier = Arc::new(Barrier::new(8));
    let handles: Vec<_> = (0..8)
        .map(|thread_id| {
            let file = file.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                for item in 0..4 {
                    let text = format!("thread-{thread_id}-item-{item}");
                    let output = run_file(&file, &["add", &text, "--agent", "race"]);
                    assert!(
                        output.status.success(),
                        "{}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    let contents = std::fs::read_to_string(&file).unwrap();
    assert_eq!(contents.lines().count(), 32);
    for line in contents.lines() {
        serde_json::from_str::<Value>(line).unwrap();
    }
}

#[test]
fn eight_way_identical_add_race_appends_once() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("cuts.jsonl");
    let barrier = Arc::new(Barrier::new(8));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let file = file.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                let envelope: SuccessEnvelope<AddData> =
                    success(&run_file(&file, &["add", "identical", "--agent", "race"]));
                envelope.data.changed
            })
        })
        .collect();
    let changed = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .filter(|changed| *changed)
        .count();
    assert_eq!(changed, 1);
    assert_eq!(std::fs::read_to_string(&file).unwrap().lines().count(), 1);
}

#[test]
fn eight_way_resolve_race_appends_once() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("cuts.jsonl");
    let id = add(&file, "resolve race").data.record.id;
    let barrier = Arc::new(Barrier::new(8));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let file = file.clone();
            let id = id.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                let envelope: SuccessEnvelope<ResolveData> =
                    success(&run_file(&file, &["resolve", &id, "--agent", "race"]));
                envelope.data.changed
            })
        })
        .collect();
    let changed = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .filter(|changed| *changed)
        .count();
    assert_eq!(changed, 1);
    assert_eq!(std::fs::read_to_string(&file).unwrap().lines().count(), 2);
}

#[test]
fn hash_length_prefix_and_tag_sort_are_pinned() {
    let a = compute_id(
        "2026-07-09T18:30:00.123Z",
        "tester",
        "ouch",
        Severity::Major,
        &["a".into(), "z".into()],
    );
    let b = compute_id(
        "2026-07-09T18:30:00.123Z",
        "tester",
        "ouc",
        Severity::Major,
        &["z".into(), "ha".into()],
    );
    let unsorted = compute_id(
        "2026-07-09T18:30:00.123Z",
        "tester",
        "ouch",
        Severity::Major,
        &["z".into(), "a".into()],
    );
    assert_eq!(a, "pc_6d26611bad4c");
    assert_eq!(a, unsorted);
    assert_ne!(a, b);
}

#[test]
fn env_papercuts_file_nonexistent_returns_not_found() {
    let temp = TempDir::new().unwrap();
    let missing = temp.path().join("missing.jsonl");
    let output = command()
        .env("PAPERCUTS_FILE", &missing)
        .arg("list")
        .output()
        .unwrap();
    error(&output, 66, "not_found");
}

#[test]
fn relative_file_resolves_against_cwd() {
    let temp = TempDir::new().unwrap();
    let output = command()
        .current_dir(temp.path())
        .arg("--file")
        .arg("rel/path.jsonl")
        .args(["add", "x", "--agent", "a", "--dry-run"])
        .output()
        .unwrap();
    let envelope: SuccessEnvelope<AddData> = success(&output);
    let temp_canonical = temp.path().canonicalize().unwrap();
    assert!(
        Path::new(envelope.meta.file.as_deref().unwrap()).starts_with(&temp_canonical),
        "meta.file = {:?}",
        envelope.meta.file
    );
}

#[test]
fn markdown_format_is_byte_deterministic() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("cuts.jsonl");
    let added = add(&file, "determinism");
    let first = run_file(&file, &["list", "--format", "md"]);
    assert!(first.status.success());
    assert!(!first.stdout.is_empty());
    let first_text = String::from_utf8_lossy(&first.stdout);
    assert!(first_text.contains("determinism"));
    assert!(first_text.contains(&added.data.record.id));
    let second = run_file(&file, &["list", "--format", "md"]);
    assert!(second.status.success());
    assert_eq!(first.stdout, second.stdout);
}

#[test]
fn doctor_reports_gitignored_finding() {
    let git_available = std::process::Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success());
    if !git_available {
        return;
    }

    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    assert!(
        std::process::Command::new("git")
            .arg("-C")
            .arg(&repo)
            .arg("init")
            .output()
            .unwrap()
            .status
            .success()
    );
    std::fs::write(repo.join(".gitignore"), ".papercuts.jsonl\n").unwrap();

    let output = command()
        .current_dir(&repo)
        .args(["add", "gitignored cut", "--agent", "a"])
        .output()
        .unwrap();
    success::<AddData>(&output);

    let doctor_output = command().current_dir(&repo).arg("doctor").output().unwrap();
    assert_eq!(doctor_output.status.code(), Some(1));
    assert!(doctor_output.stderr.is_empty());
    let doctor: SuccessEnvelope<DoctorData> =
        serde_json::from_slice(&doctor_output.stdout).unwrap();
    assert!(!doctor.data.healthy);
    assert!(
        doctor
            .data
            .findings
            .iter()
            .any(|finding| finding.kind == "gitignored")
    );
}

#[test]
fn error_envelope_matrix() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("cuts.jsonl");
    let missing = temp.path().join("missing.jsonl");
    let outside = temp.path().join("outside");
    std::fs::create_dir_all(&outside).unwrap();

    let ambiguous = temp.path().join("ambiguous.jsonl");
    let lines = ["pc_abcd00000000", "pc_abcd11111111"]
        .map(|id| {
            json!({"kind":"cut","id":id,"ts":"2026-07-09T00:00:00.000Z","agent":"a","text":id,"tags":[],"severity":"minor","cwd":"/tmp","repo":null}).to_string()
        })
        .join("\n")
        + "\n";
    std::fs::write(&ambiguous, lines).unwrap();

    error(&run(&["list", "--format", "jsonl"]), 2, "invalid_argument");
    error(
        &run_file(&file, &["add", " ", "--agent", "tester"]),
        65,
        "invalid_input",
    );
    error(&run_file(&missing, &["list"]), 66, "not_found");
    if temp_has_git_ancestor(&temp) {
        eprintln!(
            "skipping HOME/config-78 assertion because the temporary directory is inside a git checkout"
        );
    } else {
        error(
            &command()
                .current_dir(&outside)
                .env("HOME", "")
                .arg("list")
                .output()
                .unwrap(),
            78,
            "config_error",
        );
    }
    error(
        &run_file(&ambiguous, &["resolve", "abcd"]),
        65,
        "ambiguous_id",
    );
}

#[test]
fn contract2_policy_and_target_precedence_are_explicit() {
    let temp = TempDir::new().unwrap();
    let env_file = temp.path().join("env.jsonl");
    let flag_file = temp.path().join("flag.jsonl");
    let output = command()
        .env("PAPERCUTS_PROFILE", "committed")
        .env("PAPERCUTS_FILE", &env_file)
        .arg("--profile")
        .arg("private")
        .arg("--file")
        .arg(&flag_file)
        .args(["add", "preview", "--agent", "tester", "--dry-run"])
        .output()
        .unwrap();
    let envelope: SuccessEnvelope<AddData> = success(&output);
    assert_eq!(envelope.meta.contract, 2);
    assert_eq!(envelope.meta.storage_profile.as_deref(), Some("private"));
    assert_eq!(
        envelope.meta.profile_source.as_deref(),
        Some("flag-profile")
    );
    assert_eq!(envelope.meta.storage_source.as_deref(), Some("flag-file"));
    assert_eq!(envelope.meta.write_policy.as_deref(), Some("normal"));
    assert_eq!(envelope.meta.path_policy.as_deref(), Some("omitted"));
    assert_eq!(envelope.meta.sensitive_policy.as_deref(), Some("balanced"));
    assert_eq!(
        envelope.meta.sensitive_policy_source.as_deref(),
        Some("profile-default")
    );
    assert_eq!(envelope.meta.sensitive_policy_version, Some(1));
    assert_eq!(envelope.meta.file, None, "private metadata must hide paths");
    assert_eq!(envelope.data.record.cwd, ".");
    assert_eq!(envelope.data.record.repo, None);
    assert!(!env_file.exists());
    assert!(!flag_file.exists());

    let committed: SuccessEnvelope<AddData> = success(
        &command()
            .env("PAPERCUTS_PROFILE", "committed")
            .arg("--file")
            .arg(&flag_file)
            .args(["add", "preview", "--agent", "tester", "--dry-run"])
            .output()
            .unwrap(),
    );
    assert_eq!(committed.meta.storage_profile.as_deref(), Some("committed"));
    assert_eq!(
        committed.meta.profile_source.as_deref(),
        Some("env-profile")
    );
    assert_eq!(
        committed.meta.path_policy.as_deref(),
        Some("legacy-absolute")
    );
    assert_eq!(committed.meta.sensitive_policy.as_deref(), Some("strict"));
    assert_eq!(
        committed.meta.file.as_deref(),
        Some(flag_file.to_str().unwrap())
    );

    let private_default: SuccessEnvelope<AddData> = success(
        &command()
            .env_remove("PAPERCUTS_PROFILE")
            .env("PAPERCUTS_FILE", &env_file)
            .args(["add", "preview", "--agent", "tester", "--dry-run"])
            .output()
            .unwrap(),
    );
    assert_eq!(
        private_default.meta.storage_profile.as_deref(),
        Some("private")
    );
    assert_eq!(
        private_default.meta.profile_source.as_deref(),
        Some("default")
    );
    assert_eq!(
        private_default.meta.storage_source.as_deref(),
        Some("env-file")
    );
    assert_eq!(private_default.meta.file, None);
}

#[test]
fn read_only_is_monotonic_and_precedes_clock_storage_and_stdin() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("never-created.jsonl");
    let env_guard = command()
        .env("PAPERCUTS_READ_ONLY", "true")
        .env("PAPERCUTS_NOW", "invalid-clock")
        .arg("--file")
        .arg(&file)
        .args(["add", "-", "--agent", "tester"])
        .write_stdin("not consumed")
        .output()
        .unwrap();
    error(&env_guard, 78, "writes_disabled");
    assert!(!file.exists());

    let flag_guard = command()
        .env("PAPERCUTS_READ_ONLY", "false")
        .arg("--read-only")
        .arg("--file")
        .arg(&file)
        .args(["add", "blocked", "--agent", "tester"])
        .output()
        .unwrap();
    error(&flag_guard, 78, "writes_disabled");
    assert!(!file.exists());

    let invalid_env = command()
        .env("PAPERCUTS_READ_ONLY", "sometimes")
        .arg("--read-only")
        .arg("--file")
        .arg(&file)
        .args(["add", "blocked", "--agent", "tester"])
        .output()
        .unwrap();
    error(&invalid_env, 78, "config_error");

    let preview: SuccessEnvelope<AddData> = success(
        &command()
            .env("PAPERCUTS_READ_ONLY", "true")
            .arg("--file")
            .arg(&file)
            .args(["add", "preview", "--agent", "tester", "--dry-run"])
            .output()
            .unwrap(),
    );
    assert_eq!(preview.meta.write_policy.as_deref(), Some("read-only"));
    assert!(!file.exists());

    let resolve_guard = command()
        .env("PAPERCUTS_READ_ONLY", "true")
        .arg("--file")
        .arg(&file)
        .args(["resolve", "not-an-id"])
        .output()
        .unwrap();
    error(&resolve_guard, 78, "writes_disabled");

    let agent_guard = command()
        .env("PAPERCUTS_READ_ONLY", "true")
        .arg("--file")
        .arg(&file)
        .args(["add", "blocked", "--agent", " "])
        .output()
        .unwrap();
    error(&agent_guard, 78, "writes_disabled");

    let invalid_agent = command()
        .arg("--file")
        .arg(&file)
        .args(["add", "preview", "--agent", " ", "--dry-run"])
        .output()
        .unwrap();
    error(&invalid_agent, 65, "invalid_input");
}

#[test]
fn commands_read_only_the_environment_that_can_affect_them() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("cuts.jsonl");
    add(&file, "existing");

    let ignored = command()
        .arg("--file")
        .arg(&file)
        .env("PAPERCUTS_AGENT", "ignored")
        .env("PAPERCUTS_SENSITIVE_POLICY", "invalid")
        .env("PAPERCUTS_ALLOW_SENSITIVE", "invalid")
        .env("PAPERCUTS_NOW", "invalid")
        .arg("list")
        .output()
        .unwrap();
    let listed: SuccessEnvelope<ListData> = success(&ignored);
    assert_eq!(listed.data.count, 1);
    assert_eq!(listed.meta.sensitive_policy, None);

    let absolute: SuccessEnvelope<ListData> = success(
        &command()
            .arg("--file")
            .arg(&file)
            .env("PAPERCUTS_NOW", "invalid")
            .args(["list", "--since", "2026-07-01T00:00:00Z"])
            .output()
            .unwrap(),
    );
    assert_eq!(absolute.data.count, 1);

    let relative = command()
        .arg("--file")
        .arg(&file)
        .env("PAPERCUTS_NOW", "invalid")
        .args(["list", "--since", "1d"])
        .output()
        .unwrap();
    error(&relative, 78, "config_error");

    let doctor: SuccessEnvelope<DoctorData> = success(
        &command()
            .arg("--file")
            .arg(&file)
            .env("PAPERCUTS_AGENT", "ignored")
            .env("PAPERCUTS_SENSITIVE_POLICY", "invalid")
            .env("PAPERCUTS_ALLOW_SENSITIVE", "invalid")
            .env("PAPERCUTS_NOW", "invalid")
            .arg("doctor")
            .output()
            .unwrap(),
    );
    assert!(doctor.data.healthy);

    let schema: SuccessEnvelope<Value> = success(
        &command()
            .env("PAPERCUTS_PROFILE", "invalid")
            .env("PAPERCUTS_READ_ONLY", "invalid")
            .env("PAPERCUTS_SENSITIVE_POLICY", "invalid")
            .env("PAPERCUTS_ALLOW_SENSITIVE", "invalid")
            .env("PAPERCUTS_AGENT", "ignored")
            .env("PAPERCUTS_NOW", "invalid")
            .env_remove("HOME")
            .arg("schema")
            .output()
            .unwrap(),
    );
    assert_eq!(schema.data["contract"], 2);
    assert_eq!(schema.meta.storage_profile, None);
}

#[cfg(unix)]
#[test]
fn private_default_uses_common_git_storage_with_user_only_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    init_git(&repo);
    let nested = repo.join("nested/work");
    std::fs::create_dir_all(&nested).unwrap();

    let added: SuccessEnvelope<AddData> = success(
        &command()
            .current_dir(&nested)
            .arg("--profile")
            .arg("private")
            .args(["add", "private cut", "--agent", "tester"])
            .output()
            .unwrap(),
    );
    let private_dir = repo.join(".git/papercuts");
    let private_file = private_dir.join("log.jsonl");
    assert!(private_file.is_file());
    assert!(!repo.join(".papercuts.jsonl").exists());
    assert_eq!(added.meta.file, None);
    assert_eq!(added.data.record.cwd, ".");
    assert_eq!(added.data.record.repo, None);
    assert_eq!(
        std::fs::metadata(&private_dir)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        std::fs::metadata(&private_file)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );

    let listed: SuccessEnvelope<ListData> = success(
        &command()
            .current_dir(&repo)
            .arg("--profile")
            .arg("private")
            .args(["list", "--status", "all"])
            .output()
            .unwrap(),
    );
    assert_eq!(listed.data.count, 1);
    assert_eq!(listed.meta.file, None);

    let committed: SuccessEnvelope<AddData> = success(
        &command()
            .current_dir(&repo)
            .arg("--profile")
            .arg("committed")
            .args(["add", "visible cut", "--agent", "tester"])
            .output()
            .unwrap(),
    );
    assert!(repo.join(".papercuts.jsonl").is_file());
    assert_eq!(
        committed.meta.file.as_deref(),
        Some(repo.join(".papercuts.jsonl").to_str().unwrap())
    );
}

#[test]
fn private_non_git_and_legacy_migration_states_are_explicit() {
    let temp = TempDir::new().unwrap();
    if temp_has_git_ancestor(&temp) {
        return;
    }
    let outside = temp.path().join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    let virtual_empty: SuccessEnvelope<ListData> = success(
        &command()
            .current_dir(&outside)
            .arg("--profile")
            .arg("private")
            .arg("list")
            .output()
            .unwrap(),
    );
    assert_eq!(virtual_empty.data.count, 0);
    assert!(
        virtual_empty
            .meta
            .warnings
            .iter()
            .any(|warning| warning == "storage_required_for_writes")
    );
    let no_storage = command()
        .current_dir(&outside)
        .arg("--profile")
        .arg("private")
        .args(["add", "preview", "--dry-run"])
        .output()
        .unwrap();
    error(&no_storage, 78, "storage_required");

    let repo = temp.path().join("repo");
    init_git(&repo);
    let legacy_add: SuccessEnvelope<AddData> = success(
        &command()
            .current_dir(&repo)
            .arg("--profile")
            .arg("committed")
            .args(["add", "legacy", "--agent", "tester"])
            .output()
            .unwrap(),
    );
    let legacy = repo.join(".papercuts.jsonl");
    let private = repo.join(".git/papercuts/log.jsonl");
    assert!(legacy.is_file());
    assert!(!private.exists());

    let private_list: SuccessEnvelope<ListData> = success(
        &command()
            .current_dir(&repo)
            .arg("--profile")
            .arg("private")
            .arg("list")
            .output()
            .unwrap(),
    );
    assert_eq!(private_list.data.count, 0);
    assert!(
        private_list
            .meta
            .warnings
            .iter()
            .any(|warning| warning == "legacy_journal_detected")
    );

    let blocked = command()
        .current_dir(&repo)
        .arg("--profile")
        .arg("private")
        .args(["add", "new", "--agent", "tester"])
        .output()
        .unwrap();
    error(&blocked, 78, "migration_required");
    assert!(!private.exists());

    let preview: SuccessEnvelope<AddData> = success(
        &command()
            .current_dir(&repo)
            .arg("--profile")
            .arg("private")
            .args(["add", "new", "--agent", "tester", "--dry-run"])
            .output()
            .unwrap(),
    );
    assert!(
        preview
            .meta
            .warnings
            .iter()
            .any(|warning| warning == "legacy_journal_detected")
    );
    let resolve_preview = command()
        .current_dir(&repo)
        .arg("--profile")
        .arg("private")
        .args(["resolve", &legacy_add.data.record.id, "--dry-run"])
        .output()
        .unwrap();
    error(&resolve_preview, 78, "migration_required");

    std::fs::create_dir_all(private.parent().unwrap()).unwrap();
    std::fs::copy(&legacy, &private).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            private.parent().unwrap(),
            std::fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let dual: SuccessEnvelope<ListData> = success(
        &command()
            .current_dir(&repo)
            .arg("--profile")
            .arg("private")
            .args(["list", "--status", "all"])
            .output()
            .unwrap(),
    );
    assert_eq!(dual.data.count, 1);
    assert!(
        dual.meta
            .warnings
            .iter()
            .any(|warning| warning == "legacy_journal_retained")
    );
}

#[cfg(unix)]
#[test]
fn insecure_implicit_private_permissions_block_mutation_and_doctor_reports() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    init_git(&repo);
    let private_dir = repo.join(".git/papercuts");
    let private_file = private_dir.join("log.jsonl");
    std::fs::create_dir_all(&private_dir).unwrap();
    std::fs::write(&private_file, b"").unwrap();
    std::fs::set_permissions(&private_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::set_permissions(&private_file, std::fs::Permissions::from_mode(0o644)).unwrap();

    let blocked = command()
        .current_dir(&repo)
        .arg("--profile")
        .arg("private")
        .args(["add", "blocked", "--agent", "tester"])
        .output()
        .unwrap();
    error(&blocked, 77, "insecure_private_permissions");
    assert!(std::fs::read(&private_file).unwrap().is_empty());

    let doctor_output = command()
        .current_dir(&repo)
        .arg("--profile")
        .arg("private")
        .arg("doctor")
        .output()
        .unwrap();
    assert_eq!(doctor_output.status.code(), Some(1));
    let doctor: SuccessEnvelope<DoctorData> =
        serde_json::from_slice(&doctor_output.stdout).unwrap();
    assert!(
        doctor
            .data
            .findings
            .iter()
            .any(|finding| finding.kind == "insecure_private_permissions")
    );
}

#[cfg(unix)]
#[test]
fn private_profile_rejects_final_and_implicit_directory_symlinks() {
    use std::os::unix::fs::{PermissionsExt, symlink};

    let temp = TempDir::new().unwrap();
    let target = temp.path().join("target.jsonl");
    let link = temp.path().join("private-link.jsonl");
    add(&target, "existing");
    let before = std::fs::read(&target).unwrap();
    symlink(&target, &link).unwrap();

    for args in [
        vec!["add", "blocked", "--agent", "tester"],
        vec!["add", "preview", "--agent", "tester", "--dry-run"],
        vec!["list"],
    ] {
        let output = command()
            .arg("--profile")
            .arg("private")
            .arg("--file")
            .arg(&link)
            .args(args)
            .output()
            .unwrap();
        error(&output, 78, "unsafe_journal_symlink");
        assert_eq!(std::fs::read(&target).unwrap(), before);
    }

    let committed: SuccessEnvelope<ListData> = success(
        &command()
            .arg("--profile")
            .arg("committed")
            .arg("--file")
            .arg(&link)
            .args(["list", "--status", "all"])
            .output()
            .unwrap(),
    );
    assert_eq!(committed.data.count, 1);

    let repo = temp.path().join("repo");
    init_git(&repo);
    let redirected = temp.path().join("redirected-private");
    std::fs::create_dir_all(&redirected).unwrap();
    std::fs::set_permissions(&redirected, std::fs::Permissions::from_mode(0o700)).unwrap();
    symlink(&redirected, repo.join(".git/papercuts")).unwrap();
    let implicit = command()
        .current_dir(&repo)
        .arg("--profile")
        .arg("private")
        .args(["add", "blocked", "--agent", "tester"])
        .output()
        .unwrap();
    error(&implicit, 78, "unsafe_journal_symlink");
    assert!(!redirected.join("log.jsonl").exists());
}

#[test]
fn linked_worktrees_share_one_private_journal() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let linked = temp.path().join("linked");
    init_git(&repo);
    git(
        &repo,
        &["config", "user.email", "papercuts@example.invalid"],
    );
    git(&repo, &["config", "user.name", "Papercuts Test"]);
    std::fs::write(repo.join("README"), "fixture\n").unwrap();
    git(&repo, &["add", "README"]);
    git(&repo, &["commit", "-m", "fixture"]);
    git(
        &repo,
        &["worktree", "add", "--detach", linked.to_str().unwrap()],
    );

    let added: SuccessEnvelope<AddData> = success(
        &command()
            .current_dir(&repo)
            .arg("--profile")
            .arg("private")
            .args(["add", "shared", "--agent", "tester"])
            .output()
            .unwrap(),
    );
    let listed: SuccessEnvelope<ListData> = success(
        &command()
            .current_dir(&linked)
            .arg("--profile")
            .arg("private")
            .args(["list", "--status", "all"])
            .output()
            .unwrap(),
    );
    assert_eq!(listed.data.count, 1);
    assert_eq!(listed.data.items[0].cut.id, added.data.record.id);
    assert_eq!(
        std::fs::read_to_string(repo.join(".git/papercuts/log.jsonl"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}

#[test]
fn malformed_nearest_git_marker_never_falls_back_to_outer_repository() {
    let temp = TempDir::new().unwrap();
    let outer = temp.path().join("outer");
    init_git(&outer);
    let nested = outer.join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join(".git"), "gitdir: missing-admin-dir\n").unwrap();
    let output = command()
        .current_dir(&nested)
        .arg("--profile")
        .arg("private")
        .arg("list")
        .output()
        .unwrap();
    error(&output, 78, "invalid_repository");
    assert!(!outer.join(".git/papercuts/log.jsonl").exists());

    let invalid_id = command()
        .current_dir(&nested)
        .arg("--profile")
        .arg("private")
        .args(["resolve", "not-an-id"])
        .output()
        .unwrap();
    error(&invalid_id, 2, "invalid_argument");
}

#[test]
fn sensitive_policy_floor_and_override_gate_are_centralized() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("cuts.jsonl");
    let weak = command()
        .arg("--profile")
        .arg("committed")
        .arg("--sensitive-policy")
        .arg("balanced")
        .arg("--file")
        .arg(&file)
        .args(["add", "x", "--dry-run"])
        .output()
        .unwrap();
    error(&weak, 78, "config_error");

    let no_gate = command()
        .arg("--profile")
        .arg("private")
        .arg("--file")
        .arg(&file)
        .args([
            "add",
            "x",
            "--dry-run",
            "--allow-sensitive",
            "email_address",
        ])
        .output()
        .unwrap();
    error(&no_gate, 78, "config_error");

    let gated: SuccessEnvelope<AddData> = success(
        &command()
            .env("PAPERCUTS_ALLOW_SENSITIVE", "true")
            .arg("--profile")
            .arg("private")
            .arg("--sensitive-policy")
            .arg("strict")
            .arg("--file")
            .arg(&file)
            .args([
                "add",
                "alice@example.invalid",
                "--dry-run",
                "--allow-sensitive",
                "email_address",
                "--allow-sensitive",
                "email_address",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(gated.meta.sensitive_policy.as_deref(), Some("strict"));
    assert_eq!(gated.meta.sensitive_policy_source.as_deref(), Some("flag"));
    assert_eq!(
        gated.data.record.content_policy.unwrap().decision,
        papercuts::sensitive::ContentDecision::Override
    );
}

#[cfg(unix)]
#[test]
fn path_environment_keeps_native_encoding_while_text_policy_requires_utf8() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let temp = TempDir::new().unwrap();
    let mut bytes = temp.path().as_os_str().as_encoded_bytes().to_vec();
    bytes.extend_from_slice(b"/native-");
    bytes.push(0xff);
    bytes.extend_from_slice(b".jsonl");
    let native = OsString::from_vec(bytes);
    let preview: SuccessEnvelope<AddData> = success(
        &command()
            .env("PAPERCUTS_PROFILE", "private")
            .env("PAPERCUTS_FILE", &native)
            .args(["add", "preview", "--dry-run"])
            .output()
            .unwrap(),
    );
    assert_eq!(preview.meta.storage_source.as_deref(), Some("env-file"));
    assert_eq!(preview.meta.file, None);

    let invalid_text = command()
        .env(
            "PAPERCUTS_PROFILE",
            OsString::from_vec(vec![b'p', 0xff, b'r']),
        )
        .arg("schema")
        .output()
        .unwrap();
    success::<Value>(&invalid_text);

    let invalid_text = command()
        .env(
            "PAPERCUTS_PROFILE",
            OsString::from_vec(vec![b'p', 0xff, b'r']),
        )
        .arg("list")
        .output()
        .unwrap();
    error(&invalid_text, 78, "config_error");
}

#[test]
fn contract2_private_and_committed_records_are_exact_and_share_the_same_id() {
    let temp = TempDir::new().unwrap();
    if temp_has_git_ancestor(&temp) {
        return;
    }
    let private_file = temp.path().join("private.jsonl");
    let committed_file = temp.path().join("committed.jsonl");
    let text = "same path policy";
    let tags = vec!["a".to_string(), "z".to_string()];
    let id = compute_id(
        "2026-07-09T18:30:00.123Z",
        "tester",
        text,
        Severity::Major,
        &tags,
    );

    let private: SuccessEnvelope<AddData> = success(
        &command()
            .current_dir(temp.path())
            .arg("--profile")
            .arg("private")
            .arg("--file")
            .arg(&private_file)
            .args([
                "add",
                text,
                "--agent",
                "tester",
                "--severity",
                "major",
                "--tag",
                "z",
                "--tag",
                "a",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(private.data.record.id, id);
    assert_eq!(
        private.data.record.path_policy,
        Some(RecordPathPolicy::Omitted)
    );
    assert_eq!(
        private.data.record.path_encoding,
        Some(PathEncoding::Omitted)
    );
    let expected_private = format!(
        "{{\"kind\":\"cut\",\"id\":\"{id}\",\"ts\":\"2026-07-09T18:30:00.123Z\",\"agent\":\"tester\",\"text\":\"same path policy\",\"tags\":[\"a\",\"z\"],\"severity\":\"major\",\"cwd\":\".\",\"repo\":null,\"path_policy\":\"omitted\",\"path_encoding\":\"omitted\",\"content_policy\":{{\"version\":1,\"mode\":\"balanced\",\"decision\":\"clean\",\"categories\":[],\"fields\":[]}}}}\n"
    );
    assert_eq!(
        std::fs::read_to_string(&private_file).unwrap(),
        expected_private
    );

    let committed: SuccessEnvelope<AddData> = success(
        &command()
            .current_dir(temp.path())
            .arg("--profile")
            .arg("committed")
            .arg("--file")
            .arg(&committed_file)
            .args([
                "add",
                text,
                "--agent",
                "tester",
                "--severity",
                "major",
                "--tag",
                "z",
                "--tag",
                "a",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(committed.data.record.id, id);
    assert_eq!(
        committed.data.record.path_policy,
        Some(RecordPathPolicy::LegacyAbsolute)
    );
    assert_eq!(
        committed.data.record.path_encoding,
        Some(PathEncoding::Utf8)
    );
    let cwd = temp.path().to_string_lossy();
    let cwd_json = serde_json::to_string(cwd.as_ref()).unwrap();
    let expected_committed = format!(
        "{{\"kind\":\"cut\",\"id\":\"{id}\",\"ts\":\"2026-07-09T18:30:00.123Z\",\"agent\":\"tester\",\"text\":\"same path policy\",\"tags\":[\"a\",\"z\"],\"severity\":\"major\",\"cwd\":{cwd_json},\"repo\":null,\"path_policy\":\"legacy-absolute\",\"path_encoding\":\"utf8\",\"content_policy\":{{\"version\":1,\"mode\":\"strict\",\"decision\":\"clean\",\"categories\":[],\"fields\":[]}}}}\n"
    );
    assert_eq!(
        std::fs::read_to_string(&committed_file).unwrap(),
        expected_committed
    );
    assert!(
        committed
            .meta
            .warnings
            .iter()
            .any(|warning| warning == "legacy_absolute_path_exposure")
    );
}

#[test]
fn private_projection_redacts_legacy_records_without_rewriting_source_bytes() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("mixed.jsonl");
    let forbidden = "/forbidden/customer/worktree";
    let text = "legacy cut";
    let id = compute_id(
        "2026-07-09T18:30:00.123Z",
        "tester",
        text,
        Severity::Minor,
        &[],
    );
    let legacy = json!({
        "kind":"cut","id":id,"ts":"2026-07-09T18:30:00.123Z",
        "agent":"tester","text":text,"tags":[],"severity":"minor",
        "cwd":forbidden,"repo":"/forbidden/customer"
    });
    std::fs::write(&file, format!("{legacy}\n")).unwrap();
    let original = std::fs::read(&file).unwrap();

    let list_output = command()
        .arg("--profile")
        .arg("private")
        .arg("--file")
        .arg(&file)
        .args(["list", "--status", "all"])
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&list_output.stdout).contains(forbidden));
    let listed: SuccessEnvelope<ListData> = success(&list_output);
    assert_eq!(listed.data.items[0].cut.cwd, ".");
    assert_eq!(listed.data.items[0].cut.repo, None);
    assert_eq!(
        listed.data.items[0].cut.path_policy,
        Some(RecordPathPolicy::Omitted)
    );
    assert!(
        listed
            .meta
            .warnings
            .iter()
            .any(|warning| warning == "legacy_path_records_retained:1")
    );
    assert_eq!(std::fs::read(&file).unwrap(), original);

    let duplicate_output = command()
        .arg("--profile")
        .arg("private")
        .arg("--file")
        .arg(&file)
        .args(["add", text, "--agent", "tester"])
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&duplicate_output.stdout).contains(forbidden));
    let duplicate: SuccessEnvelope<AddData> = success(&duplicate_output);
    assert!(!duplicate.data.changed);
    assert_eq!(duplicate.data.record.cwd, ".");
    assert_eq!(std::fs::read(&file).unwrap(), original);

    let resolve_output = command()
        .arg("--profile")
        .arg("private")
        .arg("--file")
        .arg(&file)
        .args(["resolve", &id, "--agent", "fixer"])
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&resolve_output.stdout).contains(forbidden));
    let resolved: SuccessEnvelope<ResolveData> = success(&resolve_output);
    assert_eq!(resolved.data.record.cut.cwd, ".");
    assert_eq!(resolved.data.record.cut.repo, None);
    let after_resolve = std::fs::read_to_string(&file).unwrap();
    assert!(after_resolve.starts_with(&String::from_utf8(original.clone()).unwrap()));
    assert!(after_resolve.contains(forbidden));
    assert_eq!(after_resolve.lines().count(), 2);

    let already_output = command()
        .arg("--profile")
        .arg("private")
        .arg("--file")
        .arg(&file)
        .args(["resolve", &id, "--agent", "fixer"])
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&already_output.stdout).contains(forbidden));
    success::<ResolveData>(&already_output);

    let doctor_output = command()
        .arg("--profile")
        .arg("private")
        .arg("--file")
        .arg(&file)
        .arg("doctor")
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&doctor_output.stdout).contains(forbidden));
    let doctor: SuccessEnvelope<DoctorData> = success(&doctor_output);
    assert!(doctor.data.healthy);
    assert!(
        doctor
            .meta
            .warnings
            .iter()
            .any(|warning| warning == "legacy_path_records_retained:1")
    );

    let markdown = command()
        .arg("--profile")
        .arg("private")
        .arg("--file")
        .arg(&file)
        .args(["list", "--status", "all", "--format", "md"])
        .output()
        .unwrap();
    assert!(markdown.status.success());
    assert!(!String::from_utf8_lossy(&markdown.stdout).contains(forbidden));

    let committed_output = command()
        .arg("--profile")
        .arg("committed")
        .arg("--file")
        .arg(&file)
        .args(["list", "--status", "all"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&committed_output.stdout).contains(forbidden));
    success::<ListData>(&committed_output);
}

#[test]
fn doctor_reports_path_policy_mismatch_without_echoing_stored_path() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("mismatch.jsonl");
    let forbidden = "/forbidden/mismatched/path";
    let text = "mismatch";
    let id = compute_id(
        "2026-07-09T00:00:00.000Z",
        "tester",
        text,
        Severity::Minor,
        &[],
    );
    let record = json!({
        "kind":"cut","id":id,"ts":"2026-07-09T00:00:00.000Z",
        "agent":"tester","text":text,"tags":[],"severity":"minor",
        "cwd":forbidden,"repo":null,"path_policy":"omitted","path_encoding":"omitted"
    });
    std::fs::write(&file, format!("{record}\n")).unwrap();
    let before = std::fs::read(&file).unwrap();
    let output = command()
        .arg("--profile")
        .arg("private")
        .arg("--file")
        .arg(&file)
        .arg("doctor")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&output.stdout).contains(forbidden));
    let doctor: SuccessEnvelope<DoctorData> = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        doctor
            .data
            .findings
            .iter()
            .any(|finding| finding.kind == "path_policy_mismatch")
    );
    assert_eq!(std::fs::read(&file).unwrap(), before);
}

#[test]
fn private_doctor_never_echoes_values_from_malformed_records() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("journal.jsonl");
    let forbidden = "/forbidden/customer/doctor-value";
    let records = format!(
        "{{\"kind\":\"cut\",\"id\":\"pc_000000000000\",\"ts\":\"2026-07-12T00:00:00.000Z\",\"agent\":\"tester\",\"text\":\"safe\",\"tags\":[],\"severity\":\"minor\",\"cwd\":\".\",\"repo\":null,\"path_policy\":{forbidden_json},\"path_encoding\":\"omitted\"}}\n{{\"kind\":{forbidden_json}}}\n{{\"kind\":\"resolve\",\"id\":{forbidden_json},\"ts\":\"2026-07-12T00:00:00.000Z\",\"agent\":\"tester\",\"note\":null}}\n",
        forbidden_json = serde_json::to_string(forbidden).unwrap(),
    );
    std::fs::write(&file, records.as_bytes()).unwrap();
    let before = std::fs::read(&file).unwrap();

    let output = command()
        .args(["--profile", "private", "--file"])
        .arg(&file)
        .arg("doctor")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&output.stdout).contains(forbidden));
    assert!(!String::from_utf8_lossy(&output.stderr).contains(forbidden));
    let envelope: SuccessEnvelope<DoctorData> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope.data.findings.len(), 3);
    assert_eq!(std::fs::read(&file).unwrap(), before);
}

#[test]
fn private_errors_use_opaque_locations_and_policy_metadata() {
    let temp = TempDir::new().unwrap();
    let missing = temp.path().join("customer-secret-missing.jsonl");
    let output = command()
        .arg("--profile")
        .arg("private")
        .arg("--file")
        .arg(&missing)
        .arg("list")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("customer-secret-missing"));
    assert!(!stderr.contains(temp.path().to_string_lossy().as_ref()));
    let envelope = error(&output, 66, "not_found");
    assert_eq!(
        envelope.error.details["location"],
        Value::String("explicit_journal".into())
    );
    assert_eq!(envelope.meta.storage_profile.as_deref(), Some("private"));
    assert_eq!(envelope.meta.storage_source.as_deref(), Some("flag-file"));
    assert_eq!(envelope.meta.path_policy.as_deref(), Some("omitted"));
    assert_eq!(envelope.meta.file, None);

    let outer = temp.path().join("outer-secret");
    init_git(&outer);
    let nested = outer.join("customer-secret-nested");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join(".git"), "gitdir: missing-secret-admin\n").unwrap();
    let invalid = command()
        .current_dir(&nested)
        .arg("--profile")
        .arg("private")
        .arg("list")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&invalid.stderr);
    assert!(!stderr.contains("customer-secret"));
    assert!(!stderr.contains("missing-secret-admin"));
    let invalid = error(&invalid, 78, "invalid_repository");
    assert_eq!(
        invalid.error.details["location"],
        Value::String("repository_marker".into())
    );
    assert_eq!(invalid.meta.storage_profile.as_deref(), Some("private"));
    assert_eq!(
        invalid.meta.storage_source.as_deref(),
        Some("profile-default")
    );
    assert_eq!(invalid.meta.file, None);

    std::fs::write(&missing, b"").unwrap();
    for output in [
        command()
            .arg("--profile")
            .arg("private")
            .arg("--file")
            .arg(&missing)
            .args(["resolve", "/customer-secret-invalid-id"])
            .output()
            .unwrap(),
        command()
            .arg("--profile")
            .arg("private")
            .arg("--file")
            .arg(&missing)
            .args(["list", "--since", "customer-secret-since"])
            .output()
            .unwrap(),
        command()
            .arg("--profile")
            .arg("private")
            .args(["list", "--format", "customer-secret-format"])
            .output()
            .unwrap(),
    ] {
        assert!(!String::from_utf8_lossy(&output.stderr).contains("customer-secret"));
    }
}

#[cfg(unix)]
#[test]
fn non_utf8_paths_are_omitted_in_private_and_labeled_in_committed() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let temp = TempDir::new().unwrap();
    let mut name = b"repo-nonutf8-".to_vec();
    name.push(0xff);
    let repo = temp.path().join(OsString::from_vec(name));
    init_git(&repo);

    let private_output = command()
        .current_dir(&repo)
        .arg("--profile")
        .arg("private")
        .args(["add", "private", "--agent", "tester"])
        .output()
        .unwrap();
    assert!(
        !private_output
            .stdout
            .windows(3)
            .any(|bytes| bytes == [0xef, 0xbf, 0xbd])
    );
    let private: SuccessEnvelope<AddData> = success(&private_output);
    assert_eq!(
        private.data.record.path_encoding,
        Some(PathEncoding::Omitted)
    );
    let private_bytes = std::fs::read(repo.join(".git/papercuts/log.jsonl")).unwrap();
    assert!(
        !private_bytes
            .windows(3)
            .any(|bytes| bytes == [0xef, 0xbf, 0xbd])
    );

    let committed_output = command()
        .current_dir(&repo)
        .arg("--profile")
        .arg("committed")
        .args(["add", "committed", "--agent", "tester"])
        .output()
        .unwrap();
    let committed: SuccessEnvelope<AddData> = success(&committed_output);
    assert_eq!(
        committed.data.record.path_encoding,
        Some(PathEncoding::LossyUtf8)
    );
    assert!(
        committed
            .meta
            .warnings
            .iter()
            .any(|warning| warning == "lossy_legacy_path_encoding")
    );
}

#[test]
fn relative_gitdir_resolution_works_without_git_on_path() {
    let temp = TempDir::new().unwrap();
    let worktree = temp.path().join("submodule-like");
    let admin = temp.path().join("admin");
    std::fs::create_dir_all(&worktree).unwrap();
    std::fs::create_dir_all(admin.join("objects")).unwrap();
    std::fs::write(admin.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(admin.join("config"), "[core]\n\tbare = false\n").unwrap();
    std::fs::write(worktree.join(".git"), "gitdir: ../admin\n").unwrap();

    let output = command()
        .current_dir(&worktree)
        .env("PATH", "")
        .arg("--profile")
        .arg("private")
        .args(["add", "native resolver", "--agent", "tester"])
        .output()
        .unwrap();
    let added: SuccessEnvelope<AddData> = success(&output);
    assert_eq!(added.data.record.cwd, ".");
    assert!(admin.join("papercuts/log.jsonl").is_file());
    assert!(!worktree.join(".papercuts.jsonl").exists());
}

#[test]
fn repository_metadata_grammar_refuses_malformed_nearest_markers() {
    let temp = TempDir::new().unwrap();
    let cases: [(&str, &[u8]); 4] = [
        ("empty-marker", b""),
        ("wrong-prefix", b"directory: ../admin\n"),
        ("nul-marker", b"gitdir: ../admin\0suffix\n"),
        ("extra-line", b"gitdir: ../admin\nsecond-line\n"),
    ];
    for (name, marker) in cases {
        let worktree = temp.path().join(name);
        std::fs::create_dir_all(&worktree).unwrap();
        std::fs::write(worktree.join(".git"), marker).unwrap();
        let output = command()
            .current_dir(&worktree)
            .arg("--profile")
            .arg("private")
            .arg("list")
            .output()
            .unwrap();
        assert!(!String::from_utf8_lossy(&output.stderr).contains(name));
        error(&output, 78, "invalid_repository");
    }

    let target_is_file = temp.path().join("target-is-file");
    std::fs::create_dir_all(&target_is_file).unwrap();
    std::fs::write(target_is_file.join("admin-file"), "not a directory").unwrap();
    std::fs::write(target_is_file.join(".git"), "gitdir: admin-file\n").unwrap();
    let output = command()
        .current_dir(&target_is_file)
        .arg("--profile")
        .arg("private")
        .arg("list")
        .output()
        .unwrap();
    error(&output, 78, "invalid_repository");

    for (name, create_config, create_objects) in [
        ("missing-head", true, true),
        ("missing-config", false, true),
        ("missing-objects", true, false),
    ] {
        let repo = temp.path().join(name);
        let git_dir = repo.join(".git");
        std::fs::create_dir_all(&git_dir).unwrap();
        if name != "missing-head" {
            std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        }
        if create_config {
            std::fs::write(git_dir.join("config"), "[core]\n").unwrap();
        }
        if create_objects {
            std::fs::create_dir_all(git_dir.join("objects")).unwrap();
        }
        let output = command()
            .current_dir(&repo)
            .arg("--profile")
            .arg("private")
            .arg("list")
            .output()
            .unwrap();
        error(&output, 78, "invalid_repository");
    }

    let bad_commondir = temp.path().join("bad-commondir");
    let admin = temp.path().join("bad-commondir-admin");
    std::fs::create_dir_all(&bad_commondir).unwrap();
    std::fs::create_dir_all(&admin).unwrap();
    std::fs::write(admin.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(admin.join("commondir"), "../common\nextra\n").unwrap();
    std::fs::write(
        bad_commondir.join(".git"),
        format!("gitdir: {}\n", admin.display()),
    )
    .unwrap();
    let output = command()
        .current_dir(&bad_commondir)
        .arg("--profile")
        .arg("private")
        .arg("list")
        .output()
        .unwrap();
    error(&output, 78, "invalid_repository");
}

#[cfg(unix)]
#[test]
fn metadata_paths_preserve_symlink_parent_traversal_until_canonicalization() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();

    let worktree = temp.path().join("gitdir-worktree");
    let routed_parent = temp.path().join("gitdir-routed-parent");
    let routed_nested = routed_parent.join("nested");
    let routed_git = routed_parent.join("admin");
    let lexical_git = worktree.join("admin");
    std::fs::create_dir_all(&worktree).unwrap();
    std::fs::create_dir_all(&routed_nested).unwrap();
    for git_dir in [&routed_git, &lexical_git] {
        std::fs::create_dir_all(git_dir.join("objects")).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(git_dir.join("config"), "[core]\n").unwrap();
    }
    symlink(&routed_nested, worktree.join("route")).unwrap();
    std::fs::write(worktree.join(".git"), "gitdir: route/../admin\n").unwrap();
    let output = command()
        .current_dir(&worktree)
        .args(["--profile", "private", "add", "gitdir traversal"])
        .output()
        .unwrap();
    success::<AddData>(&output);
    assert!(routed_git.join("papercuts/log.jsonl").exists());
    assert!(!lexical_git.join("papercuts/log.jsonl").exists());

    let common_worktree = temp.path().join("commondir-worktree");
    let git_dir = temp.path().join("commondir-admin");
    let common_parent = temp.path().join("commondir-routed-parent");
    let common_nested = common_parent.join("nested");
    let routed_common = common_parent.join("common");
    let lexical_common = git_dir.join("common");
    std::fs::create_dir_all(&common_worktree).unwrap();
    std::fs::create_dir_all(&git_dir).unwrap();
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    for common in [&routed_common, &lexical_common] {
        std::fs::create_dir_all(common.join("objects")).unwrap();
        std::fs::write(common.join("config"), "[core]\n").unwrap();
    }
    std::fs::create_dir_all(&common_nested).unwrap();
    symlink(&common_nested, git_dir.join("route")).unwrap();
    std::fs::write(git_dir.join("commondir"), "route/../common\n").unwrap();
    std::fs::write(
        common_worktree.join(".git"),
        format!("gitdir: {}\n", git_dir.display()),
    )
    .unwrap();
    let output = command()
        .current_dir(&common_worktree)
        .args(["--profile", "private", "add", "commondir traversal"])
        .output()
        .unwrap();
    success::<AddData>(&output);
    assert!(routed_common.join("papercuts/log.jsonl").exists());
    assert!(!lexical_common.join("papercuts/log.jsonl").exists());
}

#[cfg(unix)]
#[test]
fn symlink_git_marker_and_non_utf8_gitdir_behave_deterministically() {
    use std::ffi::OsString;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();
    let symlink_repo = temp.path().join("symlink-marker-secret");
    let marker_target = temp.path().join("marker-target-secret");
    std::fs::create_dir_all(&symlink_repo).unwrap();
    std::fs::create_dir_all(&marker_target).unwrap();
    symlink(&marker_target, symlink_repo.join(".git")).unwrap();
    let output = command()
        .current_dir(&symlink_repo)
        .arg("--profile")
        .arg("private")
        .arg("list")
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&output.stderr).contains("secret"));
    error(&output, 78, "invalid_repository");

    let worktree = temp.path().join("native-gitdir-worktree");
    std::fs::create_dir_all(&worktree).unwrap();
    let mut admin_name = b"native-admin-".to_vec();
    admin_name.push(0xff);
    let admin_name = OsString::from_vec(admin_name);
    let admin = temp.path().join(&admin_name);
    std::fs::create_dir_all(admin.join("objects")).unwrap();
    std::fs::write(admin.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(admin.join("config"), "[core]\n").unwrap();
    let mut marker = b"gitdir: ../".to_vec();
    marker.extend_from_slice(admin_name.as_os_str().as_bytes());
    marker.extend_from_slice(b"\r\n");
    std::fs::write(worktree.join(".git"), marker).unwrap();
    let output = command()
        .current_dir(&worktree)
        .arg("--profile")
        .arg("private")
        .args(["add", "native metadata", "--agent", "tester"])
        .output()
        .unwrap();
    assert!(
        !output
            .stdout
            .windows(3)
            .any(|bytes| bytes == [0xef, 0xbf, 0xbd])
    );
    success::<AddData>(&output);
    assert!(admin.join("papercuts/log.jsonl").is_file());
}

#[test]
fn bare_repository_uses_private_non_git_semantics() {
    let temp = TempDir::new().unwrap();
    let bare = temp.path().join("bare.git");
    std::fs::create_dir_all(&bare).unwrap();
    git(&bare, &["init", "--bare"]);
    let output = command()
        .current_dir(&bare)
        .arg("--profile")
        .arg("private")
        .args(["add", "no implicit bare storage", "--dry-run"])
        .output()
        .unwrap();
    error(&output, 78, "storage_required");
    assert!(!bare.join("papercuts/log.jsonl").exists());
}

#[cfg(unix)]
#[test]
fn private_explicit_parent_symlink_is_allowed_without_path_echo() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();
    let real_parent = temp.path().join("customer-secret-real-parent");
    let linked_parent = temp.path().join("customer-secret-linked-parent");
    std::fs::create_dir_all(&real_parent).unwrap();
    symlink(&real_parent, &linked_parent).unwrap();
    let file = linked_parent.join("cuts.jsonl");
    let output = command()
        .arg("--profile")
        .arg("private")
        .arg("--file")
        .arg(&file)
        .args(["add", "through parent", "--agent", "tester"])
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&output.stdout).contains("customer-secret"));
    success::<AddData>(&output);
    assert!(real_parent.join("cuts.jsonl").is_file());
}

#[test]
fn sensitive_preflight_warns_refuses_overrides_and_precedes_clock_and_duplicate_lookup() {
    let temp = TempDir::new().unwrap();
    let private_file = temp.path().join("private/warn.jsonl");
    let warning: SuccessEnvelope<AddData> = success(
        &command()
            .arg("--profile")
            .arg("private")
            .arg("--file")
            .arg(&private_file)
            .args(["add", "contact alice@example.invalid", "--agent", "tester"])
            .output()
            .unwrap(),
    );
    let warning_policy = warning.data.record.content_policy.unwrap();
    assert_eq!(warning_policy.decision, ContentDecision::Warn);
    assert_eq!(warning_policy.categories, [SensitiveCategory::EmailAddress]);
    assert_eq!(warning_policy.fields, [SensitiveField::Text]);
    assert!(
        std::fs::read_to_string(&private_file)
            .unwrap()
            .contains("alice@example.invalid")
    );

    let sentinel = "SYNTHETIC_NO_ECHO_BEARER_7Q9";
    let suspect = format!("Authorization: Bearer {sentinel}");
    let committed_file = temp.path().join("committed.jsonl");
    let accepted: SuccessEnvelope<AddData> = success(
        &command()
            .env("PAPERCUTS_ALLOW_SENSITIVE", "true")
            .arg("--file")
            .arg(&committed_file)
            .args([
                "add",
                &suspect,
                "--agent",
                "tester",
                "--allow-sensitive",
                "authorization_header",
            ])
            .output()
            .unwrap(),
    );
    let accepted_policy = accepted.data.record.content_policy.unwrap();
    assert_eq!(accepted_policy.decision, ContentDecision::Override);
    assert_eq!(
        accepted_policy.categories,
        [SensitiveCategory::AuthorizationHeader]
    );
    let before = std::fs::read(&committed_file).unwrap();

    let refused = command()
        .env("PAPERCUTS_NOW", "invalid-clock-must-not-win")
        .arg("--file")
        .arg(&committed_file)
        .args(["add", &suspect, "--agent", "tester"])
        .output()
        .unwrap();
    let refusal = error(&refused, 65, "sensitive_input");
    assert_eq!(refusal.meta.file, None);
    assert_eq!(
        refusal.error.details["categories"],
        json!(["authorization_header"])
    );
    assert_eq!(refusal.error.details["fields"], json!(["text"]));
    assert!(!String::from_utf8_lossy(&refused.stderr).contains(sentinel));
    assert_eq!(std::fs::read(&committed_file).unwrap(), before);

    let dry_target = temp.path().join("missing-parent/refused.jsonl");
    let dry_refusal = command()
        .arg("--file")
        .arg(&dry_target)
        .args(["add", &suspect, "--agent", "tester", "--dry-run"])
        .output()
        .unwrap();
    error(&dry_refusal, 65, "sensitive_input");
    assert!(!String::from_utf8_lossy(&dry_refusal.stderr).contains(sentinel));
    assert!(!dry_target.exists());
    assert!(!dry_target.parent().unwrap().exists());
}

#[test]
fn sensitive_preflight_covers_tags_agents_and_resolution_notes() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("private.jsonl");

    let tag_warning: SuccessEnvelope<AddData> = success(
        &command()
            .arg("--profile")
            .arg("private")
            .arg("--file")
            .arg(&file)
            .args([
                "add",
                "tag field",
                "--agent",
                "tester",
                "--tag",
                "alice@example.invalid",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(
        tag_warning.data.record.content_policy.unwrap().fields,
        [SensitiveField::Tag]
    );

    let agent_warning: SuccessEnvelope<AddData> = success(
        &command()
            .arg("--profile")
            .arg("private")
            .arg("--file")
            .arg(&file)
            .args(["add", "agent field", "--agent", "agent@example.invalid"])
            .output()
            .unwrap(),
    );
    assert_eq!(
        agent_warning.data.record.content_policy.unwrap().fields,
        [SensitiveField::Agent]
    );

    let clean = add(&file, "resolution field");
    let resolved: SuccessEnvelope<ResolveData> = success(
        &command()
            .arg("--profile")
            .arg("private")
            .arg("--file")
            .arg(&file)
            .args([
                "resolve",
                &clean.data.record.id,
                "--agent",
                "fixer",
                "--note",
                "follow up with owner@example.invalid",
            ])
            .output()
            .unwrap(),
    );
    let resolution_policy = resolved
        .data
        .record
        .resolution
        .unwrap()
        .content_policy
        .unwrap();
    assert_eq!(resolution_policy.decision, ContentDecision::Warn);
    assert_eq!(resolution_policy.fields, [SensitiveField::ResolutionNote]);

    let sentinel = "ghp_SyntheticTagSentinel99";
    let refused = command()
        .arg("--file")
        .arg(temp.path().join("refused.jsonl"))
        .args(["add", "tag refusal", "--agent", "tester", "--tag", sentinel])
        .output()
        .unwrap();
    let envelope = error(&refused, 65, "sensitive_input");
    assert_eq!(envelope.error.details["fields"], json!(["tag"]));
    assert!(!String::from_utf8_lossy(&refused.stderr).contains(sentinel));
}

#[test]
fn sensitive_preflight_enforces_bounded_stdin_fields_and_notes() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("bounds.jsonl");

    let exact_stdin: SuccessEnvelope<AddData> = success(
        &command()
            .arg("--file")
            .arg(&file)
            .args(["add", "-", "--agent", "tester", "--dry-run"])
            .write_stdin(format!("{}\n", "x".repeat(10_000)))
            .output()
            .unwrap(),
    );
    assert_eq!(exact_stdin.data.record.text.len(), 10_000);
    assert!(!file.exists());

    let exact_crlf: SuccessEnvelope<AddData> = success(
        &command()
            .arg("--file")
            .arg(&file)
            .args(["add", "-", "--agent", "tester", "--dry-run"])
            .write_stdin(format!("{}\r\n", "x".repeat(10_000)))
            .output()
            .unwrap(),
    );
    assert_eq!(exact_crlf.data.record.text.len(), 10_000);
    assert!(!file.exists());

    for input in ["x".repeat(10_001), "x".repeat(10_002)] {
        let oversized = command()
            .arg("--file")
            .arg(&file)
            .args(["add", "-", "--agent", "tester", "--dry-run"])
            .write_stdin(input)
            .output()
            .unwrap();
        error(&oversized, 65, "invalid_input");
        assert!(!file.exists());
    }

    let long_agent = "a".repeat(129);
    error(
        &run_file(&file, &["add", "x", "--agent", &long_agent]),
        65,
        "invalid_input",
    );
    let long_tag = "t".repeat(65);
    error(
        &run_file(
            &file,
            &["add", "x", "--agent", "tester", "--tag", &long_tag],
        ),
        65,
        "invalid_input",
    );

    let mut too_many = command();
    too_many
        .arg("--file")
        .arg(&file)
        .args(["add", "x", "--agent", "tester"]);
    for index in 0..17 {
        too_many.arg("--tag").arg(format!("tag-{index}"));
    }
    let too_many = too_many.output().unwrap();
    error(&too_many, 65, "invalid_input");
    assert!(!file.exists());

    let added = add(&file, "note bounds");
    let exact_note = "n".repeat(2_000);
    let preview: SuccessEnvelope<ResolveData> = success(
        &command()
            .arg("--file")
            .arg(&file)
            .args([
                "resolve",
                &added.data.record.id,
                "--agent",
                "tester",
                "--note",
                &exact_note,
                "--dry-run",
            ])
            .output()
            .unwrap(),
    );
    assert!(!preview.data.changed);
    let before = std::fs::read(&file).unwrap();
    let long_note = "n".repeat(2_001);
    let rejected = command()
        .arg("--file")
        .arg(&file)
        .args([
            "resolve",
            &added.data.record.id,
            "--agent",
            "tester",
            "--note",
            &long_note,
        ])
        .output()
        .unwrap();
    error(&rejected, 65, "invalid_input");
    assert_eq!(std::fs::read(&file).unwrap(), before);
}

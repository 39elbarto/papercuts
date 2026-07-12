use assert_cmd::Command;
use papercuts::commands::add::AddData;
use papercuts::commands::doctor::DoctorData;
use papercuts::commands::list::ListData;
use papercuts::output::{ErrorEnvelope, SuccessEnvelope};
use papercuts::sensitive::ContentDecision;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::fs;
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

fn success<T: DeserializeOwned>(
    output: &std::process::Output,
    fixture: &str,
) -> SuccessEnvelope<T> {
    assert!(output.status.success(), "{fixture}: expected success");
    assert!(output.stderr.is_empty(), "{fixture}: stderr was not empty");
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|_| panic!("{fixture}: stdout was not a success envelope"))
}

fn error(output: &std::process::Output, exit: i32, code: &str, fixture: &str) -> ErrorEnvelope {
    assert_eq!(output.status.code(), Some(exit), "{fixture}: wrong exit");
    assert!(output.stdout.is_empty(), "{fixture}: stdout was not empty");
    let envelope: ErrorEnvelope = serde_json::from_slice(&output.stderr)
        .unwrap_or_else(|_| panic!("{fixture}: stderr was not an error envelope"));
    assert_eq!(envelope.error.code, code, "{fixture}: wrong error code");
    envelope
}

fn assert_absent(haystack: &[u8], sentinel: &str, fixture: &str, surface: &str) {
    assert!(
        !String::from_utf8_lossy(haystack).contains(sentinel),
        "{fixture}: sentinel reached {surface}"
    );
}

fn assert_sensitive_refusal(
    temp: &TempDir,
    fixture: &str,
    value: &str,
    sentinel: &str,
    category: &str,
) {
    for dry_run in [false, true] {
        let parent = temp.path().join(format!("{fixture}-{dry_run}"));
        let target = parent.join("journal.jsonl");
        let mut cmd = command();
        cmd.args(["--sensitive-policy", "strict", "--file"])
            .arg(&target)
            .args(["add", "--agent", "acceptance-agent"]);
        if dry_run {
            cmd.arg("--dry-run");
        }
        cmd.arg("--").arg(value);
        let output = cmd.output().expect("acceptance command must start");
        let envelope = error(&output, 65, "sensitive_input", fixture);
        assert_absent(&output.stdout, sentinel, fixture, "stdout");
        assert_absent(&output.stderr, sentinel, fixture, "stderr");
        let categories = envelope.error.details["categories"]
            .as_array()
            .unwrap_or_else(|| panic!("{fixture}: categories were not an array"));
        assert!(
            categories.iter().any(|actual| actual == category),
            "{fixture}: expected category was absent"
        );
        assert!(!target.exists(), "{fixture}: refusal created a journal");
        assert!(!parent.exists(), "{fixture}: refusal created a parent");
    }
}

#[test]
fn real_binary_refuses_every_catalog_category_without_echo_or_write() {
    let temp = TempDir::new().expect("temporary root");
    let fixtures = [
        (
            "private-key",
            concat!("-----BEGIN OPENSSH ", "PRIVATE KEY----- ACCEPTANCE_PK_41"),
            "ACCEPTANCE_PK_41",
            "private_key",
        ),
        (
            "authorization",
            "Authorization: Bearer ACCEPTANCE_AUTH_42_MATERIAL",
            "ACCEPTANCE_AUTH_42",
            "authorization_header",
        ),
        (
            "credential-url",
            "https://acceptance-user:ACCEPTANCE_URL_43@example.invalid/db",
            "ACCEPTANCE_URL_43",
            "credential_url",
        ),
        (
            "assignment",
            "client_\x73ecret=ACCEPTANCE_ASSIGN_44_MATERIAL",
            "ACCEPTANCE_ASSIGN_44",
            "secret_assignment",
        ),
        (
            "github-token",
            concat!("ghp_", "ACCEPTANCEGH45TOKEN"),
            "ACCEPTANCEGH45",
            "github_token",
        ),
        (
            "slack-token",
            concat!("xoxb-", "ACCEPTANCESL46TOKEN"),
            "ACCEPTANCESL46",
            "slack_token",
        ),
        (
            "stripe-key",
            concat!("sk_", "test_ACCEPTANCEST47TOKEN"),
            "ACCEPTANCEST47",
            "stripe_secret_key",
        ),
        (
            "aws-pair",
            concat!(
                "AKIA",
                "ABCDEFGHIJKLMNOP\nAWS_SECRET_ACCESS_KEY=ACCEPTANCE_AWS_48"
            ),
            "ACCEPTANCE_AWS_48",
            "aws_credential_pair",
        ),
        (
            "email",
            "contact ACCEPTANCE_EMAIL_49@example.invalid",
            "ACCEPTANCE_EMAIL_49",
            "email_address",
        ),
        (
            "personal-id",
            "patient_id=ACCEPTANCE_PATIENT_50",
            "ACCEPTANCE_PATIENT_50",
            "personal_identifier",
        ),
        (
            "filesystem-path",
            "inspect /home/ACCEPTANCE_PATH_51/project/file",
            "ACCEPTANCE_PATH_51",
            "filesystem_path",
        ),
        (
            "config-block",
            "ALPHA=ACCEPTANCE_CONFIG_52\nBETA=ordinary",
            "ACCEPTANCE_CONFIG_52",
            "config_block",
        ),
    ];

    for (fixture, value, sentinel, category) in fixtures {
        assert_sensitive_refusal(&temp, fixture, value, sentinel, category);
    }
}

#[test]
fn real_binary_scans_every_persisted_field_and_cross_field_pairs() {
    let temp = TempDir::new().expect("temporary root");

    let tag_sentinel = "ACCEPTANCE_TAG_61";
    let tag_target = temp.path().join("tag/journal.jsonl");
    let tag = concat!("ghp_", "ACCEPTANCE_TAG_61_TOKEN");
    let tag_output = command()
        .arg("--file")
        .arg(&tag_target)
        .args(["add", "tag fixture", "--agent", "agent", "--tag", tag])
        .output()
        .expect("tag command");
    let tag_error = error(&tag_output, 65, "sensitive_input", "tag-field");
    assert_eq!(
        tag_error.error.details["fields"],
        serde_json::json!(["tag"])
    );
    assert_absent(&tag_output.stderr, tag_sentinel, "tag-field", "stderr");
    assert!(!tag_target.exists());

    let agent_sentinel = "ACCEPTANCE_AGENT_62";
    let agent_target = temp.path().join("agent/journal.jsonl");
    let agent_output = command()
        .env(
            "PAPERCUTS_AGENT",
            concat!("ghp_", "ACCEPTANCE_AGENT_62_TOKEN"),
        )
        .arg("--file")
        .arg(&agent_target)
        .args(["add", "agent fixture"])
        .output()
        .expect("agent command");
    let agent_error = error(&agent_output, 65, "sensitive_input", "agent-field");
    assert_eq!(
        agent_error.error.details["fields"],
        serde_json::json!(["agent"])
    );
    assert_absent(
        &agent_output.stderr,
        agent_sentinel,
        "agent-field",
        "stderr",
    );
    assert!(!agent_target.exists());

    let pair_sentinel = "ACCEPTANCE_PAIR_63";
    let pair_target = temp.path().join("pair/journal.jsonl");
    let pair_output = command()
        .arg("--file")
        .arg(&pair_target)
        .args([
            "add",
            concat!("AKIA", "ABCDEFGHIJKLMNOP"),
            "--agent",
            "agent",
            "--tag",
            "AWS_SECRET_ACCESS_KEY=ACCEPTANCE_PAIR_63",
        ])
        .output()
        .expect("cross-field command");
    let pair_error = error(&pair_output, 65, "sensitive_input", "cross-field");
    assert_eq!(
        pair_error.error.details["fields"],
        serde_json::json!(["tag", "text"])
    );
    assert_absent(&pair_output.stderr, pair_sentinel, "cross-field", "stderr");
    assert!(!pair_target.exists());

    let journal = temp.path().join("resolve.jsonl");
    let add_output = command()
        .arg("--file")
        .arg(&journal)
        .args(["add", "clean resolution fixture", "--agent", "agent"])
        .output()
        .expect("clean add");
    let added: SuccessEnvelope<AddData> = success(&add_output, "resolve-setup");
    let before = fs::read(&journal).expect("journal bytes");
    let note_sentinel = "ACCEPTANCE_NOTE_64";
    let note = concat!("ghp_", "ACCEPTANCE_NOTE_64_TOKEN");
    let note_output = command()
        .arg("--file")
        .arg(&journal)
        .args([
            "resolve",
            &added.data.record.id,
            "--agent",
            "agent",
            "--note",
            note,
        ])
        .output()
        .expect("resolve command");
    let note_error = error(&note_output, 65, "sensitive_input", "resolution-note");
    assert_eq!(
        note_error.error.details["fields"],
        serde_json::json!(["resolution_note"])
    );
    assert_absent(
        &note_output.stderr,
        note_sentinel,
        "resolution-note",
        "stderr",
    );
    assert_eq!(fs::read(&journal).expect("journal bytes"), before);
}

#[test]
fn override_gate_is_exact_and_audited_by_the_real_binary() {
    let temp = TempDir::new().expect("temporary root");
    let target = temp.path().join("override.jsonl");
    let value = concat!("ghp_", "ACCEPTANCE_OVERRIDE_71_TOKEN");

    let gate_only = command()
        .env("PAPERCUTS_ALLOW_SENSITIVE", "true")
        .arg("--file")
        .arg(&target)
        .args(["add", value, "--agent", "agent"])
        .output()
        .expect("gate-only command");
    error(&gate_only, 65, "sensitive_input", "gate-only");
    assert_absent(
        &gate_only.stderr,
        "ACCEPTANCE_OVERRIDE_71",
        "gate-only",
        "stderr",
    );
    assert!(!target.exists());

    let flag_only = command()
        .arg("--file")
        .arg(&target)
        .args([
            "add",
            value,
            "--agent",
            "agent",
            "--allow-sensitive",
            "github_token",
        ])
        .output()
        .expect("flag-only command");
    error(&flag_only, 78, "config_error", "flag-only");
    assert_absent(
        &flag_only.stderr,
        "ACCEPTANCE_OVERRIDE_71",
        "flag-only",
        "stderr",
    );
    assert!(!target.exists());

    let accepted_output = command()
        .env("PAPERCUTS_ALLOW_SENSITIVE", "true")
        .arg("--file")
        .arg(&target)
        .args([
            "add",
            value,
            "--agent",
            "agent",
            "--allow-sensitive",
            "github_token",
            "--allow-sensitive",
            "github_token",
        ])
        .output()
        .expect("full override command");
    let accepted: SuccessEnvelope<AddData> = success(&accepted_output, "full-override");
    let policy = accepted
        .data
        .record
        .content_policy
        .expect("content policy audit");
    assert_eq!(policy.decision, ContentDecision::Override);
    assert_eq!(policy.categories.len(), 1);

    let before = fs::read(&target).expect("override journal");
    let unknown = "ACCEPTANCE_UNKNOWN_CATEGORY_72";
    let unknown_output = command()
        .env("PAPERCUTS_ALLOW_SENSITIVE", "true")
        .arg("--file")
        .arg(&target)
        .args(["add", "clean", "--allow-sensitive", unknown])
        .output()
        .expect("unknown override command");
    error(&unknown_output, 2, "invalid_argument", "unknown-category");
    assert_absent(
        &unknown_output.stderr,
        unknown,
        "unknown-category",
        "stderr",
    );
    assert_eq!(fs::read(&target).expect("override journal"), before);
}

#[test]
fn documented_false_positive_controls_and_dry_run_modes_are_real_binary_clean() {
    let temp = TempDir::new().expect("temporary root");
    let controls = [
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "0123456789abcdef0123456789abcdef01234567",
        "550e8400-e29b-41d4-a716-446655440000",
        "br-hardened-papercuts-fork-x30.11",
        "issue #12345",
        "password=example",
        "token='[redacted]'",
        "api_key=$TOKEN",
        "client_secret=${CLIENT_SECRET}",
        "the words test example redacted are documentation labels",
        "pk_test_ACCEPTANCEPUBLISHABLE99",
        "pk_live_ACCEPTANCEPUBLISHABLE99",
        concat!("AKIA", "ABCDEFGHIJKLMNOP"),
        concat!("ASIA", "ABCDEFGHIJKLMNOP"),
        "relative/path/to/file",
        "alpha=one",
        "Unicode prose: секрет 🔐 éxample раураl",
        "Z2hwX1N5bnRoZXRpY1Rva2VuQm9keTk5",
        "ghp_short xoxb-short sk_test_short",
    ];
    for (index, value) in controls.iter().enumerate() {
        let parent = temp.path().join(format!("control-{index}"));
        let target = parent.join("journal.jsonl");
        let output = command()
            .args(["--sensitive-policy", "strict", "--file"])
            .arg(&target)
            .args(["add", "--agent", "agent", "--dry-run", "--"])
            .arg(value)
            .output()
            .expect("control command");
        let accepted: SuccessEnvelope<AddData> = success(&output, "false-positive-control");
        assert_eq!(
            accepted
                .data
                .record
                .content_policy
                .expect("content audit")
                .decision,
            ContentDecision::Clean,
            "control-{index}: not clean"
        );
        assert!(!parent.exists(), "control-{index}: dry run wrote storage");
    }

    let warning_parent = temp.path().join("warning-dry-run");
    let warning_output = command()
        .args(["--profile", "private", "--file"])
        .arg(warning_parent.join("journal.jsonl"))
        .args([
            "add",
            "contact warning@example.invalid",
            "--agent",
            "agent",
            "--dry-run",
        ])
        .output()
        .expect("warning dry run");
    let warning: SuccessEnvelope<AddData> = success(&warning_output, "warning-dry-run");
    assert_eq!(
        warning
            .data
            .record
            .content_policy
            .expect("content audit")
            .decision,
        ContentDecision::Warn
    );
    assert!(!warning_parent.exists());

    let override_parent = temp.path().join("override-dry-run");
    let override_output = command()
        .env("PAPERCUTS_ALLOW_SENSITIVE", "true")
        .arg("--file")
        .arg(override_parent.join("journal.jsonl"))
        .args([
            "add",
            concat!("ghp_", "ACCEPTANCEDRYRUN121TOKEN"),
            "--agent",
            "agent",
            "--allow-sensitive",
            "github_token",
            "--dry-run",
        ])
        .output()
        .expect("override dry run");
    let override_envelope: SuccessEnvelope<AddData> = success(&override_output, "override-dry-run");
    assert_eq!(
        override_envelope
            .data
            .record
            .content_policy
            .expect("content audit")
            .decision,
        ContentDecision::Override
    );
    assert!(!override_parent.exists());
}

#[cfg(unix)]
#[test]
fn submodule_private_identity_is_distinct_and_committed_symlink_is_legacy_compatible() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().expect("temporary root");
    let root = temp.path().join("repo");
    let common = root.join(".git");
    fs::create_dir_all(common.join("objects")).expect("outer objects");
    fs::write(common.join("HEAD"), "ref: refs/heads/main\n").expect("outer HEAD");
    fs::write(common.join("config"), "[core]\n\tbare = false\n").expect("outer config");

    let submodule = root.join("vendor/submodule");
    let admin = common.join("modules/submodule");
    fs::create_dir_all(&submodule).expect("submodule worktree");
    fs::create_dir_all(admin.join("objects")).expect("submodule objects");
    fs::write(admin.join("HEAD"), "ref: refs/heads/main\n").expect("submodule HEAD");
    fs::write(admin.join("config"), "[core]\n\tbare = false\n").expect("submodule config");
    fs::write(
        submodule.join(".git"),
        "gitdir: ../../.git/modules/submodule\n",
    )
    .expect("submodule marker");

    let outer_output = command()
        .current_dir(&root)
        .args(["--profile", "private", "add", "outer identity"])
        .output()
        .expect("outer add");
    success::<AddData>(&outer_output, "outer-private");
    let submodule_output = command()
        .current_dir(&submodule)
        .args(["--profile", "private", "add", "submodule identity"])
        .output()
        .expect("submodule add");
    success::<AddData>(&submodule_output, "submodule-private");
    let outer_journal = common.join("papercuts/log.jsonl");
    let submodule_journal = admin.join("papercuts/log.jsonl");
    assert!(outer_journal.is_file());
    assert!(submodule_journal.is_file());
    assert_ne!(
        fs::read(&outer_journal).expect("outer journal"),
        fs::read(&submodule_journal).expect("submodule journal")
    );

    let real = temp.path().join("legacy-real.jsonl");
    let link = temp.path().join("legacy-link.jsonl");
    fs::write(&real, b"").expect("legacy real file");
    symlink(&real, &link).expect("legacy symlink");
    let committed_output = command()
        .current_dir(&root)
        .arg("--file")
        .arg(&link)
        .args(["add", "legacy symlink compatibility", "--agent", "agent"])
        .output()
        .expect("committed symlink add");
    success::<AddData>(&committed_output, "committed-symlink");
    assert!(!fs::read(&real).expect("legacy target").is_empty());
}

#[test]
fn private_lifecycle_never_projects_storage_fragments() {
    let temp = TempDir::new().expect("temporary root");
    let hidden = temp
        .path()
        .join("Users")
        .join("ACCEPTANCE_USER_81")
        .join("ACCEPTANCE_REPO_82")
        .join("ACCEPTANCE_COMMON_83")
        .join("journal.jsonl");
    let fragments = [
        temp.path().to_string_lossy().into_owned(),
        "ACCEPTANCE_USER_81".into(),
        "ACCEPTANCE_REPO_82".into(),
        "ACCEPTANCE_COMMON_83".into(),
        "journal.jsonl".into(),
    ];

    let add_output = command()
        .args(["--profile", "private", "--file"])
        .arg(&hidden)
        .args(["add", "private lifecycle", "--agent", "agent"])
        .output()
        .expect("private add");
    let added: SuccessEnvelope<AddData> = success(&add_output, "private-add");
    assert_eq!(added.meta.file, None);

    let operations = [
        command()
            .args(["--profile", "private", "--file"])
            .arg(&hidden)
            .args(["add", "private lifecycle", "--agent", "agent"])
            .output()
            .expect("duplicate add"),
        command()
            .args(["--profile", "private", "--file"])
            .arg(&hidden)
            .args(["add", "private dry run", "--agent", "agent", "--dry-run"])
            .output()
            .expect("dry add"),
        command()
            .args(["--profile", "private", "--file"])
            .arg(&hidden)
            .args(["list", "--status", "all"])
            .output()
            .expect("private list"),
        command()
            .args(["--profile", "private", "--file"])
            .arg(&hidden)
            .args(["resolve", &added.data.record.id, "--agent", "agent"])
            .output()
            .expect("private resolve"),
        command()
            .args(["--profile", "private", "--file"])
            .arg(&hidden)
            .args(["resolve", &added.data.record.id, "--agent", "agent"])
            .output()
            .expect("already resolved"),
        command()
            .args(["--profile", "private", "--file"])
            .arg(&hidden)
            .arg("doctor")
            .output()
            .expect("private doctor"),
    ];
    for (index, output) in operations.iter().enumerate() {
        assert!(output.status.success(), "private-operation-{index}: failed");
        for fragment in &fragments {
            assert_absent(&output.stdout, fragment, "private-lifecycle", "stdout");
            assert_absent(&output.stderr, fragment, "private-lifecycle", "stderr");
        }
    }
    let journal = fs::read(&hidden).expect("private journal");
    for fragment in &fragments {
        assert_absent(&journal, fragment, "private-lifecycle", "journal");
    }

    let missing = hidden.with_file_name("ACCEPTANCE_MISSING_84.jsonl");
    let missing_output = command()
        .args(["--profile", "private", "--file"])
        .arg(&missing)
        .arg("list")
        .output()
        .expect("missing private list");
    error(&missing_output, 66, "not_found", "private-error");
    assert_absent(
        &missing_output.stderr,
        "ACCEPTANCE_MISSING_84",
        "private-error",
        "stderr",
    );
}

#[test]
fn storage_fallback_and_no_side_effect_matrix_uses_disposable_roots() {
    let temp = TempDir::new().expect("temporary root");
    let workspace = temp.path().join("workspace");
    let home = temp.path().join("home");
    fs::create_dir_all(&workspace).expect("workspace");
    fs::create_dir_all(&home).expect("home");

    let committed = command()
        .current_dir(&workspace)
        .env("HOME", &home)
        .args(["--profile", "committed", "add", "home fallback"])
        .output()
        .expect("committed fallback");
    let committed: SuccessEnvelope<AddData> = success(&committed, "committed-home");
    assert_eq!(
        committed.meta.storage_source.as_deref(),
        Some("profile-default")
    );
    assert!(home.join(".papercuts/log.jsonl").is_file());

    let missing_home = command()
        .current_dir(&workspace)
        .env_remove("HOME")
        .args(["--profile", "committed", "add", "missing home"])
        .output()
        .expect("missing home command");
    error(&missing_home, 78, "config_error", "missing-home");

    let dry_root = temp.path().join("dry-root");
    fs::create_dir_all(&dry_root).expect("dry root");
    let dry_output = command()
        .current_dir(&dry_root)
        .env("HOME", temp.path().join("unused-home"))
        .args(["--profile", "committed", "add", "dry fallback", "--dry-run"])
        .output()
        .expect("dry fallback command");
    success::<AddData>(&dry_output, "dry-home");
    assert!(!temp.path().join("unused-home").exists());

    for subcommand in ["list", "doctor"] {
        let target = temp
            .path()
            .join(format!("missing-{subcommand}/journal.jsonl"));
        let output = command()
            .arg("--file")
            .arg(&target)
            .arg(subcommand)
            .output()
            .expect("missing read command");
        error(&output, 66, "not_found", subcommand);
        assert!(!target.exists());
        assert!(!target.parent().expect("parent").exists());
    }
}

#[test]
fn invalid_utf8_and_parser_values_are_redacted_and_write_nothing() {
    let temp = TempDir::new().expect("temporary root");
    let target = temp.path().join("invalid/journal.jsonl");
    let invalid_stdin = command()
        .arg("--file")
        .arg(&target)
        .args(["add", "-", "--agent", "agent"])
        .write_stdin(vec![b'a', 0xff, b'b'])
        .output()
        .expect("invalid stdin command");
    error(&invalid_stdin, 65, "invalid_input", "invalid-utf8");
    assert!(!target.exists());
    assert!(!target.parent().expect("parent").exists());

    let invalid_profile = "ACCEPTANCE_INVALID_PROFILE_91";
    let profile_output = command()
        .env("PAPERCUTS_PROFILE", invalid_profile)
        .arg("list")
        .output()
        .expect("invalid profile command");
    error(&profile_output, 78, "config_error", "invalid-profile");
    assert_absent(
        &profile_output.stderr,
        invalid_profile,
        "invalid-profile",
        "stderr",
    );

    let invalid_id = "ACCEPTANCE_INVALID_ID_92";
    let id_output = command()
        .arg("--file")
        .arg(&target)
        .args(["resolve", invalid_id])
        .output()
        .expect("invalid id command");
    error(&id_output, 2, "invalid_argument", "invalid-id");
    assert_absent(&id_output.stderr, invalid_id, "invalid-id", "stderr");
    assert!(!target.exists());
}

#[test]
fn mixed_journal_is_read_only_deterministic_and_safely_diagnosed() {
    let temp = TempDir::new().expect("temporary root");
    let journal = temp.path().join("mixed.jsonl");
    let hidden_path = "/home/ACCEPTANCE_LEGACY_PATH_101/repo";
    let legacy = serde_json::json!({
        "kind":"cut","id":"pc_000000000001","ts":"2026-07-09T00:00:00.000Z",
        "agent":"legacy","text":"legacy","tags":[],"severity":"minor",
        "cwd":hidden_path,"repo":hidden_path
    });
    let current = serde_json::json!({
        "kind":"cut","id":"pc_000000000002","ts":"2026-07-10T00:00:00.000Z",
        "agent":"current","text":"current","tags":[],"severity":"major",
        "cwd":".","repo":null,"path_policy":"omitted","path_encoding":"omitted",
        "content_policy":{"version":1,"mode":"balanced","decision":"clean","categories":[],"fields":[]}
    });
    let orphan = serde_json::json!({
        "kind":"resolve","id":"pc_deadbeef0000","ts":"2026-07-11T00:00:00.000Z",
        "agent":"resolver","note":null
    });
    let bytes =
        format!("{legacy}\n{current}\n{current}\n{orphan}\n{{\"kind\":\"future\"}}\n{{\"kind\":");
    fs::write(&journal, bytes.as_bytes()).expect("mixed journal");
    let before = fs::read(&journal).expect("mixed bytes");

    let list_output = command()
        .args(["--profile", "private", "--file"])
        .arg(&journal)
        .args(["list", "--status", "all"])
        .output()
        .expect("mixed list");
    let listed: SuccessEnvelope<ListData> = success(&list_output, "mixed-list");
    assert_eq!(listed.data.count, 2);
    assert_eq!(listed.data.items[0].cut.id, "pc_000000000002");
    assert_absent(
        &list_output.stdout,
        "ACCEPTANCE_LEGACY_PATH_101",
        "mixed-list",
        "stdout",
    );

    let doctor_output = command()
        .args(["--profile", "private", "--file"])
        .arg(&journal)
        .arg("doctor")
        .output()
        .expect("mixed doctor");
    assert_eq!(doctor_output.status.code(), Some(1));
    let doctor: SuccessEnvelope<DoctorData> = serde_json::from_slice(&doctor_output.stdout)
        .expect("doctor success envelope with findings");
    let kinds: Vec<_> = doctor
        .data
        .findings
        .iter()
        .map(|finding| finding.kind.as_str())
        .collect();
    for expected in [
        "duplicate_cut",
        "orphan_resolve",
        "unknown_kind",
        "torn_line",
    ] {
        assert!(kinds.contains(&expected), "mixed-doctor: missing finding");
    }
    assert_absent(
        &doctor_output.stdout,
        "ACCEPTANCE_LEGACY_PATH_101",
        "mixed-doctor",
        "stdout",
    );
    assert_eq!(fs::read(&journal).expect("mixed bytes"), before);
}

#[test]
fn schema_and_read_commands_ignore_unrelated_hostile_environment() {
    let temp = TempDir::new().expect("temporary root");
    let schema_output = command()
        .env("PAPERCUTS_PROFILE", "ACCEPTANCE_SCHEMA_PROFILE_111")
        .env("PAPERCUTS_FILE", temp.path().join("schema-hidden"))
        .env("PAPERCUTS_NOW", "ACCEPTANCE_SCHEMA_CLOCK_112")
        .env("PAPERCUTS_AGENT", "ACCEPTANCE_SCHEMA_AGENT_113")
        .env("PAPERCUTS_READ_ONLY", "ACCEPTANCE_SCHEMA_READONLY_114")
        .env("PAPERCUTS_SENSITIVE_POLICY", "ACCEPTANCE_SCHEMA_POLICY_115")
        .env("PAPERCUTS_ALLOW_SENSITIVE", "ACCEPTANCE_SCHEMA_GATE_116")
        .arg("schema")
        .output()
        .expect("schema command");
    let schema: SuccessEnvelope<Value> = success(&schema_output, "static-schema");
    assert_eq!(schema.data["contract"], 2);
    for sentinel in [
        "ACCEPTANCE_SCHEMA_PROFILE_111",
        "ACCEPTANCE_SCHEMA_CLOCK_112",
        "ACCEPTANCE_SCHEMA_AGENT_113",
        "ACCEPTANCE_SCHEMA_READONLY_114",
        "ACCEPTANCE_SCHEMA_POLICY_115",
        "ACCEPTANCE_SCHEMA_GATE_116",
    ] {
        assert_absent(&schema_output.stdout, sentinel, "static-schema", "stdout");
    }

    let journal = temp.path().join("read.jsonl");
    let added_output = command()
        .arg("--file")
        .arg(&journal)
        .args(["add", "read environment fixture", "--agent", "agent"])
        .output()
        .expect("read setup");
    success::<AddData>(&added_output, "read-setup");
    let before = fs::read(&journal).expect("read journal");
    for subcommand in ["list", "doctor"] {
        let output = command()
            .env("PAPERCUTS_AGENT", "ACCEPTANCE_IGNORED_AGENT_117")
            .env(
                "PAPERCUTS_SENSITIVE_POLICY",
                "ACCEPTANCE_IGNORED_POLICY_118",
            )
            .env("PAPERCUTS_ALLOW_SENSITIVE", "ACCEPTANCE_IGNORED_GATE_119")
            .arg("--file")
            .arg(&journal)
            .arg(subcommand)
            .output()
            .expect("read command");
        if subcommand == "list" {
            success::<ListData>(&output, "read-list");
        } else {
            success::<DoctorData>(&output, "read-doctor");
        }
    }
    assert_eq!(fs::read(&journal).expect("read journal"), before);
}

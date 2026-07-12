use crate::cli::SchemaTarget;
use crate::error::{self, ERROR_CONTRACT};
use crate::policy::SensitiveCategory;
use crate::sensitive;
use serde_json::{Value, json};

pub fn contract(target: SchemaTarget) -> Value {
    let records = records_contract();
    let errors = errors_contract();
    let exit_codes: Value = json!(error::exit_code_map());
    let compatibility = compatibility_contract();
    match target {
        SchemaTarget::Record => json!({
            "contract": 2,
            "records": records,
            "id": id_contract(),
            "compatibility": compatibility,
        }),
        SchemaTarget::Error => json!({
            "contract": 2,
            "errors": errors,
            "exit_codes": exit_codes,
            "diagnostics": diagnostics_contract(),
        }),
        SchemaTarget::ExitCodes => json!({
            "contract": 2,
            "exit_codes": exit_codes,
        }),
        SchemaTarget::All => json!({
            "contract": 2,
            "implementation_status": {
                "storage_policy": "implemented by x30.7",
                "path_projection": "implemented by x30.8",
                "sensitive_preflight": "implemented by x30.9",
                "contract_surface": "implemented by x30.10",
                "adversarial_acceptance": "implemented by x30.11",
                "security_claim": "bounded guardrails implemented; hardened release claim withheld until remaining acceptance and release gates pass"
            },
            "security_claim": {
                "may_say": "Papercuts applies a monotonic write guard, keeps private automatic storage outside worktrees, omits automatic path fields in private records, and checks bounded caller-authored fields for policy-v1 sensitive-data shapes before append.",
                "must_also_say": "Private does not mean encrypted. Detection is incomplete. Balanced warnings and deliberate overrides persist original input. Legacy journals can retain paths and unscanned text.",
                "must_not_say": ["all secrets are detected", "accepted input is sanitized", "the private profile is encrypted", "the fork is hardened before all gates pass"]
            },
            "plaintext_exceptions": ["--help", "--version", "list --format md"],
            "commands": commands_contract(),
            "global_flags": {
                "--file": "PATH; target override only, never changes profile",
                "--pretty": "pretty-print JSON envelopes",
                "--profile": "private|committed",
                "--read-only": "monotonic write restriction",
                "--sensitive-policy": "balanced|strict; cannot weaken profile floor"
            },
            "environment": environment_contract(),
            "precedence": {
                "profile": ["--profile", "non-empty PAPERCUTS_PROFILE", "private default"],
                "target": ["--file", "non-empty PAPERCUTS_FILE", "profile default"],
                "write_policy": "--read-only OR truthy PAPERCUTS_READ_ONLY",
                "sensitive_policy": ["--sensitive-policy", "non-empty PAPERCUTS_SENSITIVE_POLICY", "profile floor"],
                "agent": ["--agent", "non-empty PAPERCUTS_AGENT", "known harness detection", "unknown default"]
            },
            "evaluation_order": {
                "mutation": ["parse CLI", "validate relevant non-clock configuration", "enforce write guard", "validate non-consuming arguments", "resolve logical profile and target", "resolve migration/storage prerequisites", "read bounded stdin", "validate UTF-8 and field bounds", "scan persisted caller-authored fields", "refuse without journal I/O when required", "resolve clock and construct event", "create/open/lock/read/fold/append", "project and serialize once"],
                "add_dry_run": "same validation, scan, clock, record, and projection; no directory creation, journal open, lock, read, or append",
                "resolve_dry_run": "same validation and scan; shared journal read is required to identify the item",
                "schema": "static; reads no profile, storage, repository, clock, agent, read-only, or sensitive-policy environment"
            },
            "profiles": {
                "private": {"default":true,"content_floor":"balanced","path_policy":"omitted","automatic_storage":"validated GIT_COMMON_DIR/papercuts/log.jsonl","outside_git":"explicit target required for mutation","meta_file":"omitted"},
                "committed": {"default":false,"content_floor":"strict","path_policy":"legacy-absolute","automatic_storage":"validated repository root/.papercuts.jsonl, otherwise HOME/.papercuts/log.jsonl","meta_file":"included","warning":"legacy_absolute_path_exposure"}
            },
            "storage": storage_contract(),
            "repository": repository_contract(),
            "records": records,
            "metadata": metadata_contract(),
            "warnings": warnings_contract(),
            "id": id_contract(),
            "sensitive_content": sensitive_contract(),
            "errors": errors,
            "exit_codes": exit_codes,
            "diagnostics": diagnostics_contract(),
            "compatibility": compatibility,
            "limitations": ["local filesystems only for advisory-lock guarantees", "no fsync durability promise per append", "no network, server, telemetry, runtime pattern download, entropy scan, decoding, or Unicode normalization", "no automatic migration, merge, rewrite, deletion, or retroactive clean label", "no multi-project discovery or aggregation in contract 2"]
        }),
    }
}

fn commands_contract() -> Value {
    json!({
        "add": {"alias":["log"],"positional":"TEXT or -; stdin when omitted and non-TTY","flags":{"--agent":"NAME","--tag":"TAG repeatable","--severity":"minor|major|blocker","--dry-run":"boolean","--allow-sensitive":"CATEGORY repeatable"},"output":"{changed,record}","read_only":false,"appends":true,"destructive":false,"dry_run":true,"may_create":true},
        "list": {"flags":{"--status":"open|resolved|all","--agent":"NAME","--tag":"TAG","--severity":"minor|major|blocker","--since":"RFC3339|Nd|Nh","--limit":"N default 50","--format":"json|md"},"output":"{items,count,total,truncated}","read_only":true,"appends":false,"destructive":false,"dry_run":false,"may_create":false},
        "resolve": {"positional":"optional pc_ plus at least 4 hexadecimal digits","flags":{"--note":"TEXT","--agent":"NAME","--dry-run":"boolean","--allow-sensitive":"CATEGORY repeatable"},"output":"{changed,record-with-resolution}","read_only":false,"appends":true,"destructive":false,"dry_run":true,"may_create":false},
        "schema": {"positional":"all|record|error|exit-codes","read_only":true,"appends":false,"destructive":false,"dry_run":false,"may_create":false},
        "doctor": {"output":"{healthy,findings,checked_lines}","exit_codes":{"0":"healthy","1":"findings"},"read_only":true,"appends":false,"destructive":false,"dry_run":false,"may_create":false}
    })
}

fn environment_contract() -> Value {
    json!({
        "PAPERCUTS_FILE":{"type":"native OS path","commands":["add","list","resolve","doctor"],"empty":"unset"},
        "PAPERCUTS_AGENT":{"type":"UTF-8 text","commands":["add","resolve"],"empty":"unset"},
        "PAPERCUTS_NOW":{"type":"full RFC3339 timestamp","commands":["add","resolve","list with relative --since"],"empty":"unset","lazy":true},
        "PAPERCUTS_PROFILE":{"type":"private|committed ASCII case-insensitive","commands":["add","list","resolve","doctor"],"empty":"unset"},
        "PAPERCUTS_READ_ONLY":{"type":"0|1|false|true ASCII case-insensitive","commands":["add","list","resolve","doctor"],"empty":"unset"},
        "PAPERCUTS_SENSITIVE_POLICY":{"type":"balanced|strict ASCII case-insensitive","commands":["add","resolve"],"empty":"unset"},
        "PAPERCUTS_ALLOW_SENSITIVE":{"type":"0|1|false|true ASCII case-insensitive","commands":["add","resolve"],"empty":"unset","gate_alone":"inert"},
        "HOME":{"type":"native OS path","commands":["add","list","resolve","doctor"],"relevance":"committed non-Git profile default only"}
    })
}

fn records_contract() -> Value {
    let private_cut = json!({
        "kind":"cut","id":"pc_94f5df71022d","ts":"2026-07-12T00:00:00.000Z","agent":"codex","text":"workspace tests only pass from apps/web","tags":["tooling"],"severity":"minor","cwd":".","repo":null,"path_policy":"omitted","path_encoding":"omitted","content_policy":{"version":1,"mode":"balanced","decision":"clean","categories":[],"fields":[]}
    });
    let committed_cut = json!({
        "kind":"cut","id":"pc_94f5df71022d","ts":"2026-07-12T00:00:00.000Z","agent":"codex","text":"workspace tests only pass from apps/web","tags":["tooling"],"severity":"minor","cwd":"/Users/alice/work/papercuts/apps/web","repo":"/Users/alice/work/papercuts","path_policy":"legacy-absolute","path_encoding":"utf8","content_policy":{"version":1,"mode":"strict","decision":"clean","categories":[],"fields":[]}
    });
    let resolve = json!({
        "kind":"resolve","id":"pc_94f5df71022d","ts":"2026-07-12T01:00:00.000Z","agent":"codex","note":"fixed in br-example","content_policy":{"version":1,"mode":"balanced","decision":"clean","categories":[],"fields":[]}
    });
    json!({
        "field_order":{"cut":["kind","id","ts","agent","text","tags","severity","cwd","repo","path_policy","path_encoding","content_policy"],"resolve":["kind","id","ts","agent","note","content_policy"]},
        "exact_examples":{"private_clean_cut":private_cut,"committed_clean_cut":committed_cut,"resolve_event":resolve},
        "list_item":{"shape":"cut fields plus status and optional resolution","status":"open|resolved","resolution":"{ts,agent,note,content_policy}|omitted"},
        "legacy_inference":{"missing_path_policy":"contract-1 legacy path semantics","missing_content_policy":"legacy-unscanned","reads_rewrite_source":false},
        "path_fields":{"private":{"cwd":".","repo":null,"path_policy":"omitted","path_encoding":"omitted"},"committed":{"cwd":"absolute path","repo":"absolute path|null","path_policy":"legacy-absolute","path_encoding":"utf8|lossy-utf8"}}
    })
}

fn metadata_contract() -> Value {
    json!({
        "private_add_resolve":{"contract":2,"storage_profile":"private","profile_source":"default|flag-profile|env-profile","storage_source":"profile-default|flag-file|env-file","write_policy":"normal|read-only","path_policy":"omitted","sensitive_policy":"balanced|strict","sensitive_policy_source":"profile-default|flag|env","sensitive_policy_version":1,"file":"omitted"},
        "committed_add_resolve":{"path_policy":"legacy-absolute","file":"included","warning":"legacy_absolute_path_exposure"},
        "list_doctor":{"sensitive_policy_fields":"omitted because no ingestion decision is made"},
        "schema":{"contract":2,"all_other_meta":"omitted"},
        "set_like_arrays":"lexicographically sorted and deduplicated"
    })
}

fn warnings_contract() -> Value {
    json!({
        "ordering":"lexicographically sorted and deduplicated before JSON serialization",
        "stable":[
            {"pattern":"storage_required_for_writes","meaning":"private non-Git read is virtual empty but mutation needs an explicit target"},
            {"pattern":"legacy_journal_detected","meaning":"legacy committed journal exists while private default is absent"},
            {"pattern":"legacy_journal_retained","meaning":"private and legacy journals both exist; private is authoritative"},
            {"pattern":"legacy_path_records_retained:N","meaning":"N source cuts retain contract-1 or legacy absolute paths"},
            {"pattern":"legacy_unscanned_records:N","meaning":"N source cut or resolve events have no content_policy audit"},
            {"pattern":"legacy_absolute_path_exposure","meaning":"committed profile can expose automatic absolute paths"},
            {"pattern":"lossy_legacy_path_encoding","meaning":"a committed automatic path required lossy UTF-8 projection"},
            {"pattern":"dry run; no record appended","meaning":"add preview did not write"},
            {"pattern":"dry run; no resolve event appended","meaning":"resolve preview did not write"},
            {"pattern":"duplicate papercut; existing record returned","meaning":"first cut for the deterministic ID won"},
            {"pattern":"already resolved","meaning":"first resolve won; no new event appended"},
            {"pattern":"no papercuts file yet; papercuts add creates it","meaning":"discovered list target is virtual empty"},
            {"pattern":"no papercuts file yet; healthy empty state","meaning":"discovered doctor target is virtual healthy empty"},
            {"pattern":"no papercuts matched; try --status all or broader filters","meaning":"filters returned an empty successful result"},
            {"pattern":"skipped N <torn|malformed|unknown|duplicate|orphan> event(s)","meaning":"fold skipped compatible journal anomalies"}
        ],
        "policy_state":"content_policy.decision and categories, not warning prose, describe clean/warn/override"
    })
}

fn id_contract() -> Value {
    json!({"prefix":"pc_","hex_digits":12,"hash":"first 6 bytes of SHA-256","encoding":"u32 little-endian UTF-8 byte length before each field","included_fields":["ts","agent","text","severity","sorted tags joined with comma"],"excluded_fields":["cwd","repo","path_policy","path_encoding","content_policy"]})
}

fn sensitive_contract() -> Value {
    json!({
        "version": sensitive::POLICY_VERSION,
        "modes":{"balanced":"refuse high; warn medium","strict":"refuse high and medium"},
        "profile_floors":{"private":"balanced","committed":"strict"},
        "categories": SensitiveCategory::ALL,
        "fields":["text","tag","agent","resolution_note"],
        "decisions":["clean","override","warn"],
        "bounds":{"text_bytes":sensitive::MAX_TEXT_BYTES,"stdin_retained_byte_ceiling":sensitive::MAX_TEXT_BYTES+1,"resolution_note_bytes":sensitive::MAX_NOTE_BYTES,"tag_bytes":sensitive::MAX_TAG_BYTES,"tag_count":sensitive::MAX_TAGS,"agent_bytes":sensitive::MAX_AGENT_BYTES,"total_scan_bytes":sensitive::MAX_TOTAL_BYTES,"compiled_patterns_max":128,"compiled_patterns_actual":sensitive::PATTERN_COUNT},
        "override":{"environment_gate":"truthy PAPERCUTS_ALLOW_SENSITIVE","command_gate":"exact repeated --allow-sensitive for every refusing category","wildcard":false,"partial":"invalid_input 65","unused":"invalid_input 65","flags_without_gate":"config_error 78"},
        "audit":{"shape":"{version,mode,decision,categories,fields}","excluded_from_id":true,"accepted_warn_override_is_redacted":false},
        "known_misses":["encoded or compressed values","Unicode-normalized or homoglyph-obfuscated labels","unmarked key bodies","deliberately split signatures","vendors and identifiers outside policy-v1 catalog","referenced files, clipboard, history, existing journals, and unrelated environment"]
    })
}

fn errors_contract() -> Value {
    let catalog: Vec<_> = ERROR_CONTRACT
        .iter()
        .map(|entry| json!({"code":entry.code,"exit_code":entry.exit_code,"retryable":entry.retryable(),"description":entry.description}))
        .collect();
    json!({
        "shape":{"ok":false,"error":{"code":"catalog code","message":"safe string","details":"code-specific safe object","retryable":false,"suggested_fix":"non-destructive remediation"},"meta":{"contract":2}},
        "catalog":catalog,
        "sensitive_input_details":["policy_version","policy","categories","fields"],
        "ambiguous_id_details":"sorted full product-generated candidate IDs; caller prefix omitted",
        "suggested_fix_policy":"never teaches disabling read-only, selecting committed, or authorizing sensitive content merely to pass"
    })
}

fn diagnostics_contract() -> Value {
    json!({
        "rejected_values":"never echoed for parser, configuration, ID, since, sensitive, or bounded-input errors",
        "private_path_values":"never emitted in records, meta.file, errors, or doctor findings",
        "private_locations":["current_working_directory","repository_marker","git_directory","git_common_directory","private_journal","explicit_journal","stdin","stdout"],
        "doctor_findings":["torn_line","malformed","unknown_kind","orphan_resolve","duplicate_cut","id_conflict","conflict_marker","gitignored","path_policy_mismatch","content_policy_mismatch","insecure_private_permissions"]
    })
}

fn storage_contract() -> Value {
    json!({
        "format":"append-only JSONL event journal",
        "locking":"local filesystems; 50 try-lock attempts x 100 ms; timeout is retryable exit 75",
        "append":"one serialized buffer with write_all; torn-tail newline healing; truncate rollback on failed write",
        "durability":"best effort; no per-append fsync",
        "private_permissions":{"implicit_directory":"0700 on Unix","implicit_file":"0600 on Unix","explicit_target":"operator-owned; final symlink still rejected"},
        "migration_states":{"none":"use selected private state","legacy_only":"reads are virtual private empty; real mutation is migration_required","dual":"use private only and warn legacy retained"},
        "migration":"copy-and-verify only; never automatic move, merge, delete, or rewrite"
    })
}

fn repository_contract() -> Value {
    json!({
        "search":"physical cwd ancestors; nearest .git marker wins and malformed nearest marker never falls back",
        "marker":"regular directory or one-line gitdir file; symlink rejected",
        "gitdir":"absolute or marker-parent-relative native path; LF/CRLF; NUL and extra logical lines rejected",
        "commondir":"regular one-line absolute or gitdir-relative native path",
        "required":{"git_dir":["regular HEAD"],"common_dir":["regular config","objects directory"]},
        "bare_repository":"does not imply a worktree root",
        "git_executable":"not required for discovery",
        "symlink_semantics":"metadata path parent traversal is preserved until canonicalization"
    })
}

fn compatibility_contract() -> Value {
    json!({
        "contract_1":{"readable":true,"missing_path_policy":"legacy path semantics","missing_content_policy":"legacy-unscanned","rewritten":false},
        "v0_1_reader":{"new_records_parse":true,"unknown_new_fields":"ignored by default serde behavior","private_cwd":".","private_repo":null,"id_recomputation":"unchanged","level":"parse compatibility only","not_understood":["contract 2", "profile/path sentinel semantics", "content policy", "private discovery", "new limits and safe diagnostics"]},
        "committed_profile":{"preserves":["repository-visible/default-home storage locations","automatic absolute path meaning","explicit target behavior","append/fold/lock semantics"],"diverges_on":["default profile","strict Git validation","input bounds","content refusal","audit and path fields","schema/meta contract","new errors"]},
        "rollback":"binary selection only; retain journal bytes unchanged; never strip audit fields or rewrite history"
    })
}

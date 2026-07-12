use crate::cli::SchemaTarget;
use crate::error;
use serde_json::{Value, json};

pub fn contract(target: SchemaTarget) -> Value {
    let implementation_status = json!({
        "storage_policy": "implemented by x30.7",
        "path_projection": "implemented by x30.8 for new records, mixed-journal reads, command outputs, and private path diagnostics",
        "sensitive_preflight": "policy and override inputs are resolved, but content scanning and enforcement are pending x30.9",
        "security_claim": "none until implementation, adversarial, documentation, and release gates pass"
    });
    let records = json!({
        "cut": {"kind":"cut","id":"pc_<12 lowercase hex>","ts":"RFC3339 UTC milliseconds","agent":"string","text":"string <= 10000 bytes","tags":["string"],"severity":"minor|major|blocker","cwd":". under omitted; absolute path under legacy-absolute","repo":"null under omitted; absolute path|null under legacy-absolute","path_policy":"omitted|legacy-absolute; missing means contract-1 legacy","path_encoding":"omitted|utf8|lossy-utf8; missing with path_policy means contract-1 legacy"},
        "resolve": {"kind":"resolve","id":"pc_<12 lowercase hex>","ts":"RFC3339 UTC milliseconds","agent":"string","note":"string|null"},
        "list_item": {"cut":"all cut fields projected through the active profile without rewriting source bytes","status":"open|resolved","resolution":"{ts,agent,note}|omitted"},
        "private_path_fields": {"cwd":".","repo":null,"path_policy":"omitted","path_encoding":"omitted"},
        "committed_path_fields": {"cwd":"absolute path","repo":"absolute path|null","path_policy":"legacy-absolute","path_encoding":"utf8|lossy-utf8"}
    });
    let errors = json!({
        "shape": {"ok":false,"error":{"code":"string","message":"string","details":{},"retryable":false,"suggested_fix":"string"},"meta":{"contract":2}},
        "codes": error::error_codes()
    });
    let exit_codes: Value = json!(error::exit_code_map());
    match target {
        SchemaTarget::Record => {
            json!({"contract":2,"implementation_status":implementation_status,"records":records})
        }
        SchemaTarget::Error => json!({"contract":2,"errors":errors}),
        SchemaTarget::ExitCodes => json!({"contract":2,"exit_codes":exit_codes}),
        SchemaTarget::All => json!({
            "contract": 2,
            "implementation_status": implementation_status,
            "success_envelope": {"ok":true,"data":"command-specific object","meta":{"contract":2,"storage_profile":"private|committed","profile_source":"flag-profile|env-profile|default","storage_source":"flag-file|env-file|profile-default","write_policy":"normal|read-only","path_policy":"omitted|legacy-absolute","file":"committed profile only","agent_source":"flag|env|detected|default where relevant","sensitive_policy":"balanced|strict on add/resolve","sensitive_policy_source":"flag|env|profile-default on add/resolve","sensitive_policy_version":"1 on add/resolve; resolution only until x30.9","warnings":["sorted unique string; omitted when empty"]}},
            "commands": {
                "add": {"alias":["log"],"positional":"TEXT or -; optional when stdin is piped","flags":{"--agent":"NAME","--tag":"TAG; repeatable","--severity":"minor|major|blocker; default minor","--dry-run":"boolean","--allow-sensitive":"CATEGORY; repeatable; requires environment gate"},"output":"{changed,record}","read_only":false,"appends":true,"destructive":false},
                "list": {"flags":{"--status":"open|resolved|all; default open","--agent":"NAME","--tag":"TAG","--severity":"minor|major|blocker","--since":"full RFC3339|Nd|Nh","--limit":"N; default 50","--format":"json|md; default json"},"output":"{items,count,total,truncated}; md is raw markdown","read_only":true,"appends":false,"destructive":false},
                "resolve": {"positional":"ID; optional pc_ plus at least 4 hex digits","flags":{"--note":"TEXT","--agent":"NAME","--dry-run":"boolean","--allow-sensitive":"CATEGORY; repeatable; requires environment gate"},"output":"{changed,record-with-resolution}","read_only":false,"appends":true,"destructive":false},
                "schema": {"positional":"all|record|error|exit-codes; default all","read_only":true,"appends":false,"destructive":false},
                "doctor": {"flags":{},"output":"{healthy,findings,checked_lines}","exit_codes":{"0":"healthy","1":"findings"},"read_only":true,"appends":false,"destructive":false}
            },
            "global_flags": ["--file <PATH>","--pretty","--profile private|committed","--read-only","--sensitive-policy balanced|strict"],
            "env": {
                "PAPERCUTS_FILE":"log-file override",
                "PAPERCUTS_AGENT":"agent-name fallback",
                "PAPERCUTS_NOW":"full RFC3339 clock override; read lazily only when relevant",
                "PAPERCUTS_PROFILE":"private|committed",
                "PAPERCUTS_READ_ONLY":"0|1|false|true",
                "PAPERCUTS_SENSITIVE_POLICY":"balanced|strict",
                "PAPERCUTS_ALLOW_SENSITIVE":"0|1|false|true; inert without exact category flags"
            },
            "records": records,
            "id": {"prefix":"pc_","hex_digits":12,"hash":"SHA-256 first 6 bytes","fields_in_order":["ts","agent","text","severity","sorted tags joined with comma"],"encoding":"u32 little-endian UTF-8 byte length before each field"},
            "discovery": {"target_precedence":["--file","PAPERCUTS_FILE","profile default"],"private_default":"validated GIT_COMMON_DIR/papercuts/log.jsonl; explicit storage required outside Git","committed_default":"validated repository root/.papercuts.jsonl or $HOME/.papercuts/log.jsonl"},
            "path_contract": {"id_excludes":["cwd","repo","path_policy","path_encoding"],"private_projection":"contract-1 and legacy-absolute records return omitted sentinels without source rewrite","committed_projection":"stored legacy paths remain visible; omitted paths are never reconstructed","private_error_locations":["current_working_directory","repository_marker","git_directory","git_common_directory","private_journal","explicit_journal","stdin","stdout"],"limitations":"agent-authored text, tags, and notes can still contain paths; sensitive preflight is pending x30.9"},
            "errors": errors,
            "exit_codes": exit_codes,
            "storage": {"format":"append-only JSONL","private_permissions":"implicit directory 0700 and file 0600 on Unix","migration":"legacy-only private default requires explicit copy-and-verify migration","locking":"local filesystems only; 50 retries x 100ms","durability":"best effort; no fsync per append"}
        }),
    }
}

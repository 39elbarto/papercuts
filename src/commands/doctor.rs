use crate::error::{AppError, AppResult};
use crate::output::{self, Meta};
use crate::policy::{PolicyContext, StorageProfile};
use crate::store;
use crate::{CutRecord, PathEncoding, RecordPathPolicy, ResolveRecord, compute_id};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::process::Command;
use std::process::Stdio;

#[derive(Debug, Serialize, Deserialize)]
pub struct DoctorData {
    pub healthy: bool,
    pub findings: Vec<Finding>,
    pub checked_lines: usize,
    #[serde(skip)]
    pub legacy_path_records: usize,
    #[serde(skip)]
    pub legacy_unscanned_records: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Finding {
    pub line: usize,
    pub kind: String,
    pub message: String,
}

pub fn run(context: &PolicyContext, pretty: bool) -> AppResult<i32> {
    let resolved = &context.storage;
    let mut warnings = resolved.warnings.clone();
    let empty = || DoctorData {
        healthy: true,
        findings: Vec::new(),
        checked_lines: 0,
        legacy_path_records: 0,
        legacy_unscanned_records: 0,
    };
    let (mut data, file_existed) = if let Some(path) = resolved.path.as_deref() {
        match store::with_shared_resolved(resolved, |log| {
            let bytes = store::read_bytes(log, path)?;
            Ok(inspect(&bytes))
        }) {
            Ok(data) => (data, true),
            Err(error) if error.code == "not_found" && error.exit_code == 66 => {
                if resolved.explicit {
                    return Err(AppError::not_found(
                        if context.profile == StorageProfile::Private {
                            "selected private papercuts file was not found".into()
                        } else {
                            format!("papercuts file not found: {}", path.display())
                        },
                        "Pass an existing --file PATH or omit --file to inspect discovered state.",
                    ));
                }
                warnings.push("no papercuts file yet; healthy empty state".into());
                (empty(), false)
            }
            Err(error) => return Err(error),
        }
    } else {
        (empty(), false)
    };
    if !store::private_permissions_secure(resolved)? {
        data.findings.push(Finding {
            line: 0,
            kind: "insecure_private_permissions".into(),
            message: "implicit private storage is accessible beyond the current user".into(),
        });
        data.healthy = false;
    }
    if context.profile == StorageProfile::Private && data.legacy_path_records > 0 {
        warnings.push(format!(
            "legacy_path_records_retained:{}",
            data.legacy_path_records
        ));
    }
    if data.legacy_unscanned_records > 0 {
        warnings.push(format!(
            "legacy_unscanned_records:{}",
            data.legacy_unscanned_records
        ));
    }
    if file_existed
        && context.profile == StorageProfile::Committed
        && let Some(repo) = resolved.repo.as_ref()
        && let Some(path) = resolved.path.as_ref()
        && path.starts_with(repo)
        && Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["check-ignore", "-q", "--"])
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    {
        data.findings.push(Finding {
            line: 0,
            kind: "gitignored".into(),
            message: "papercuts file is gitignored; papercuts will not appear in diffs".into(),
        });
        data.healthy = false;
    }
    if context.profile == StorageProfile::Private {
        for finding in &mut data.findings {
            finding.message = private_finding_message(&finding.kind).into();
        }
    }
    let exit = i32::from(!data.healthy);
    let mut meta = Meta::from_policy(context, false);
    meta.warnings = warnings;
    output::write_success(data, pretty, meta)
        .map_err(|error| AppError::from_io(error, std::path::Path::new("stdout")))?;
    Ok(exit)
}

fn inspect(bytes: &[u8]) -> DoctorData {
    let mut findings = Vec::new();
    let mut cuts = HashMap::<String, Vec<u8>>::new();
    let mut cut_ids = HashSet::new();
    let mut resolves = Vec::<(usize, String)>::new();
    let mut checked_lines = 0;
    let mut legacy_path_records = 0;
    let mut legacy_unscanned_records = 0;
    let torn = !bytes.is_empty() && !bytes.ends_with(b"\n");
    let line_count = bytes.split(|byte| *byte == b'\n').count();
    for (index, raw) in bytes.split(|byte| *byte == b'\n').enumerate() {
        if raw.is_empty() && index + 1 == line_count {
            continue;
        }
        checked_lines += 1;
        let line = index + 1;
        if torn && index + 1 == line_count {
            findings.push(Finding {
                line,
                kind: "torn_line".into(),
                message: "final physical line is not newline-terminated".into(),
            });
            continue;
        }
        if raw.starts_with(b"<<<<<<< ") || raw.starts_with(b">>>>>>> ") {
            findings.push(Finding {
                line,
                kind: "conflict_marker".into(),
                message: "complete git conflict-marker line found".into(),
            });
            continue;
        }
        let Ok(value) = serde_json::from_slice::<Value>(raw) else {
            findings.push(Finding {
                line,
                kind: "malformed".into(),
                message: "line is not valid JSON".into(),
            });
            continue;
        };
        match value.get("kind").and_then(Value::as_str) {
            Some("cut") => match serde_json::from_value::<CutRecord>(value) {
                Ok(cut) => {
                    if cut.ts.parse::<jiff::Timestamp>().is_err() {
                        findings.push(Finding {
                            line,
                            kind: "malformed".into(),
                            message: "cut ts is not a full RFC3339 timestamp".into(),
                        });
                        continue;
                    }
                    match (cut.path_policy, cut.path_encoding) {
                        (None, None) => legacy_path_records += 1,
                        (Some(RecordPathPolicy::Omitted), Some(PathEncoding::Omitted))
                            if cut.cwd == "." && cut.repo.is_none() => {}
                        (
                            Some(RecordPathPolicy::LegacyAbsolute),
                            Some(PathEncoding::Utf8 | PathEncoding::LossyUtf8),
                        ) if looks_absolute(&cut.cwd)
                            && cut.repo.as_deref().is_none_or(looks_absolute) => {}
                        _ => findings.push(Finding {
                            line,
                            kind: "path_policy_mismatch".into(),
                            message: "cut path fields do not match the declared path policy".into(),
                        }),
                    }
                    match cut.content_policy.as_ref() {
                        None => legacy_unscanned_records += 1,
                        Some(policy) if !policy.is_valid_v1() => findings.push(Finding {
                            line,
                            kind: "content_policy_mismatch".into(),
                            message: "cut content policy violates version-1 invariants".into(),
                        }),
                        Some(_) => {}
                    }
                    let mut tags = cut.tags.clone();
                    tags.sort();
                    let expected = compute_id(&cut.ts, &cut.agent, &cut.text, cut.severity, &tags);
                    if cut.id != expected {
                        findings.push(Finding {
                            line,
                            kind: "id_conflict".into(),
                            message: format!("cut ID {} does not recompute to {expected}", cut.id),
                        });
                    }
                    if let Some(first) = cuts.get(&cut.id) {
                        let (kind, message) = if first == raw {
                            (
                                "duplicate_cut",
                                format!("byte-identical duplicate cut {}", cut.id),
                            )
                        } else {
                            (
                                "id_conflict",
                                format!(
                                    "cut {} has a different payload than its first occurrence",
                                    cut.id
                                ),
                            )
                        };
                        findings.push(Finding {
                            line,
                            kind: kind.into(),
                            message,
                        });
                    } else {
                        cuts.insert(cut.id.clone(), raw.to_vec());
                    }
                    cut_ids.insert(cut.id);
                }
                Err(error) => findings.push(Finding {
                    line,
                    kind: "malformed".into(),
                    message: format!("invalid cut record: {error}"),
                }),
            },
            Some("resolve") => match serde_json::from_value::<ResolveRecord>(value) {
                Ok(resolve) => {
                    if resolve.ts.parse::<jiff::Timestamp>().is_err() {
                        findings.push(Finding {
                            line,
                            kind: "malformed".into(),
                            message: "resolve ts is not a full RFC3339 timestamp".into(),
                        });
                        continue;
                    }
                    match resolve.content_policy.as_ref() {
                        None => legacy_unscanned_records += 1,
                        Some(policy) if !policy.is_valid_v1() => findings.push(Finding {
                            line,
                            kind: "content_policy_mismatch".into(),
                            message: "resolve content policy violates version-1 invariants".into(),
                        }),
                        Some(_) => {}
                    }
                    resolves.push((line, resolve.id));
                }
                Err(error) => findings.push(Finding {
                    line,
                    kind: "malformed".into(),
                    message: format!("invalid resolve record: {error}"),
                }),
            },
            Some(kind) => findings.push(Finding {
                line,
                kind: "unknown_kind".into(),
                message: format!("unknown event kind '{kind}'"),
            }),
            None => findings.push(Finding {
                line,
                kind: "unknown_kind".into(),
                message: "event has no string kind field".into(),
            }),
        }
    }
    for (line, id) in resolves {
        if !cut_ids.contains(&id) {
            findings.push(Finding {
                line,
                kind: "orphan_resolve".into(),
                message: format!("resolve references unknown cut {id}"),
            });
        }
    }
    DoctorData {
        healthy: findings.is_empty(),
        findings,
        checked_lines,
        legacy_path_records,
        legacy_unscanned_records,
    }
}

fn looks_absolute(value: &str) -> bool {
    std::path::Path::new(value).is_absolute()
        || value
            .as_bytes()
            .get(1..3)
            .is_some_and(|bytes| bytes[0] == b':' && matches!(bytes[1], b'/' | b'\\'))
        || value.starts_with("\\\\")
}

fn private_finding_message(kind: &str) -> &'static str {
    match kind {
        "torn_line" => "final physical line is not newline-terminated",
        "malformed" => "record does not match the expected schema",
        "unknown_kind" => "event kind is not recognized",
        "orphan_resolve" => "resolve references an unknown cut",
        "duplicate_cut" => "byte-identical duplicate cut found",
        "id_conflict" => "cut identifier conflict found",
        "conflict_marker" => "complete git conflict-marker line found",
        "path_policy_mismatch" => "cut path fields do not match the declared path policy",
        "content_policy_mismatch" => "record content policy violates version-1 invariants",
        "insecure_private_permissions" => {
            "implicit private storage is accessible beyond the current user"
        }
        _ => "journal diagnostic finding",
    }
}

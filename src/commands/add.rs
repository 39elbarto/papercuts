use crate::cli::AddArgs;
use crate::error::{AppError, AppResult};
use crate::output::{self, Meta};
use crate::policy::{PolicyContext, StorageProfile};
use crate::store;
use crate::{CutRecord, PathEncoding, RecordPathPolicy, compute_id, format_timestamp};
use serde::{Deserialize, Serialize};
use std::io::{IsTerminal, Read};

#[derive(Debug, Serialize, Deserialize)]
pub struct AddData {
    pub changed: bool,
    pub record: CutRecord,
}

pub fn run(args: AddArgs, context: &PolicyContext, pretty: bool) -> AppResult<i32> {
    let resolved = &context.storage;
    let text = read_text(args.text)?;
    if text.trim().is_empty() {
        return Err(AppError::invalid_input(
            "papercut text cannot be empty or whitespace-only",
            "Pass non-empty TEXT or pipe it on stdin.",
        ));
    }
    if text.len() > 10_000 {
        return Err(AppError::invalid_input(
            format!(
                "papercut text is {} bytes; the maximum is 10000",
                text.len()
            ),
            "Shorten the papercut text to at most 10000 UTF-8 bytes.",
        ));
    }
    let identity = context
        .agent
        .as_ref()
        .ok_or_else(|| AppError::internal("add policy omitted agent identity"))?;
    if identity.value.trim().is_empty() {
        return Err(AppError::invalid_input(
            "agent name cannot be empty or whitespace-only",
            "Pass a non-empty --agent NAME, set PAPERCUTS_AGENT, or omit both.",
        ));
    }
    let agent = identity.value.clone();
    let mut tags = args.tags;
    tags.sort();
    let now = context.effective_now()?;
    let ts = format_timestamp(now);
    let (cwd, repo, path_policy, path_encoding, lossy_paths) =
        if context.profile == StorageProfile::Private {
            (
                ".".into(),
                None,
                RecordPathPolicy::Omitted,
                PathEncoding::Omitted,
                false,
            )
        } else {
            let cwd_path = std::env::current_dir()
                .map_err(|error| AppError::from_io(error, std::path::Path::new(".")))?;
            let lossy = cwd_path.to_str().is_none()
                || resolved
                    .repo
                    .as_ref()
                    .is_some_and(|path| path.to_str().is_none());
            (
                cwd_path.to_string_lossy().into_owned(),
                resolved
                    .repo
                    .as_ref()
                    .map(|path| path.to_string_lossy().into_owned()),
                RecordPathPolicy::LegacyAbsolute,
                if lossy {
                    PathEncoding::LossyUtf8
                } else {
                    PathEncoding::Utf8
                },
                lossy,
            )
        };
    let record = CutRecord {
        kind: "cut".into(),
        id: compute_id(&ts, &agent, &text, args.severity, &tags),
        ts,
        agent,
        text,
        tags,
        severity: args.severity,
        cwd,
        repo,
        path_policy: Some(path_policy),
        path_encoding: Some(path_encoding),
    };

    let mut warnings = Vec::new();
    if lossy_paths {
        warnings.push("lossy_legacy_path_encoding".into());
    }
    let (changed, record) = if args.dry_run {
        warnings.push("dry run; no record appended".into());
        (false, record)
    } else {
        let path = resolved.path()?.to_path_buf();
        store::with_exclusive_resolved(resolved, true, |log| {
            let bytes = store::read_bytes(log, &path)?;
            if let Some(existing) = store::fold_bytes(&bytes)
                .items
                .into_iter()
                .find(|item| item.cut.id == record.id)
            {
                return Ok((false, existing.cut));
            }
            store::append_json(log, &path, &bytes, &record)?;
            Ok((true, record))
        })?
    };
    if !changed && !args.dry_run {
        warnings.push("duplicate papercut; existing record returned".into());
    }
    let (record, retained_legacy) = context.project_cut(record);
    if context.profile == StorageProfile::Private && retained_legacy {
        warnings.push("legacy_path_records_retained:1".into());
    }
    let mut meta = Meta::from_policy(context, true);
    meta.agent_source = Some(identity.source.into());
    meta.warnings.extend(warnings);
    output::write_success(AddData { changed, record }, pretty, meta)
        .map_err(|error| AppError::from_io(error, std::path::Path::new("stdout")))?;
    Ok(0)
}

fn read_text(text: Option<String>) -> AppResult<String> {
    let use_stdin =
        text.as_deref() == Some("-") || (text.is_none() && !std::io::stdin().is_terminal());
    let mut text = if use_stdin {
        let mut input = Vec::new();
        std::io::stdin()
            .lock()
            .read_to_end(&mut input)
            .map_err(|error| AppError::from_io(error, std::path::Path::new("stdin")))?;
        String::from_utf8(input).map_err(|_| {
            AppError::invalid_input(
                "papercut text from stdin is not valid UTF-8",
                "Pipe UTF-8 text to `papercuts add -`.",
            )
        })?
    } else {
        text.ok_or_else(|| {
            AppError::invalid_argument(
                "add requires TEXT when stdin is a terminal",
                "Run `papercuts add \"what went wrong\"` or pipe text to `papercuts add -`.",
            )
        })?
    };
    while text.ends_with('\n') || text.ends_with('\r') {
        text.pop();
    }
    Ok(text)
}

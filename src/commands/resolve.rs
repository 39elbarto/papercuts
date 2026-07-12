use crate::cli::ResolveArgs;
use crate::error::{AppError, AppResult};
use crate::output::{self, Meta};
use crate::policy::{PolicyContext, StorageProfile};
use crate::sensitive;
use crate::store;
use crate::{ItemStatus, ListItem, Resolution, ResolveRecord, format_timestamp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ResolveData {
    pub changed: bool,
    pub record: ListItem,
}

pub fn run(args: ResolveArgs, context: &PolicyContext, pretty: bool) -> AppResult<i32> {
    let prefix = normalize_prefix(&args.id)?;
    let resolved = &context.storage;
    let identity = context
        .agent
        .as_ref()
        .ok_or_else(|| AppError::internal("resolve policy omitted agent identity"))?;
    if identity.value.trim().is_empty() {
        return Err(AppError::invalid_input(
            "agent name cannot be empty or whitespace-only",
            "Pass a non-empty --agent NAME, set PAPERCUTS_AGENT, or omit both.",
        ));
    }
    let agent = identity.value.clone();
    let note = args.note;
    let content_policy = sensitive::preflight_resolve(
        context
            .sensitive_policy
            .ok_or_else(|| AppError::internal("resolve policy omitted sensitive policy"))?,
        &context.allow_sensitive,
        note.as_deref(),
        &agent,
    )?;
    let path = resolved.path()?.to_path_buf();
    let now = context.effective_now()?;
    let ts = format_timestamp(now);
    let action = |log: &mut std::fs::File| -> AppResult<(bool, bool, ListItem)> {
        let bytes = store::read_bytes(log, &path)?;
        let folded = store::fold_bytes(&bytes);
        let id = match_id(&prefix, &folded.items)?;
        let mut item = folded
            .items
            .into_iter()
            .find(|item| item.cut.id == id)
            .ok_or_else(|| AppError::internal("matched papercut disappeared during resolution"))?;
        if item.status == ItemStatus::Resolved {
            return Ok((false, true, item));
        }
        item.status = ItemStatus::Resolved;
        item.resolution = Some(Resolution {
            ts: ts.clone(),
            agent: agent.clone(),
            note: note.clone(),
            content_policy: Some(content_policy.clone()),
        });
        if !args.dry_run {
            let event = ResolveRecord {
                kind: "resolve".into(),
                id,
                ts: ts.clone(),
                agent: agent.clone(),
                note: note.clone(),
                content_policy: Some(content_policy.clone()),
            };
            store::append_json(log, &path, &bytes, &event)?;
        }
        Ok((!args.dry_run, false, item))
    };
    let (changed, already_resolved, record) = match if args.dry_run {
        store::with_shared_resolved(resolved, action)
    } else {
        store::with_exclusive_resolved(resolved, false, action)
    } {
        Ok(result) => result,
        Err(error) if error.code == "not_found" && error.exit_code == 66 => {
            return Err(AppError::not_found(
                if context.profile == StorageProfile::Private {
                    "selected private papercuts file was not found".into()
                } else {
                    format!("papercuts file not found: {}", path.display())
                },
                "Run `papercuts list --status all` to find an ID after adding a papercut.",
            ));
        }
        Err(error) => return Err(error),
    };
    let (record, retained_legacy) = context.project_item(record);
    let mut meta = Meta::from_policy(context, true);
    meta.agent_source = Some(identity.source.into());
    if already_resolved {
        meta.warnings.push("already resolved".into());
    } else if args.dry_run {
        meta.warnings
            .push("dry run; no resolve event appended".into());
    }
    if context.profile == StorageProfile::Private && retained_legacy {
        meta.warnings.push("legacy_path_records_retained:1".into());
    }
    output::write_success(ResolveData { changed, record }, pretty, meta)
        .map_err(|error| AppError::from_io(error, std::path::Path::new("stdout")))?;
    Ok(0)
}

pub(crate) fn validate_id(input: &str) -> AppResult<()> {
    normalize_prefix(input).map(|_| ())
}

fn normalize_prefix(input: &str) -> AppResult<String> {
    let prefix = input
        .get(..3)
        .filter(|prefix| prefix.eq_ignore_ascii_case("pc_"))
        .map_or(input, |_| &input[3..]);
    if prefix.len() < 4 || !prefix.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::invalid_argument(
            "invalid papercut ID prefix",
            "Use `papercuts list --status all` and pass at least 4 hexadecimal digits, with optional pc_ prefix.",
        ));
    }
    Ok(prefix.to_ascii_lowercase())
}

fn match_id(prefix: &str, items: &[ListItem]) -> AppResult<String> {
    let mut candidates: Vec<_> = items
        .iter()
        .map(|item| item.cut.id.clone())
        .filter(|id| {
            id.get(..3)
                .filter(|id_prefix| id_prefix.eq_ignore_ascii_case("pc_"))
                .and_then(|_| id.get(3..))
                .is_some_and(|hex| hex.to_ascii_lowercase().starts_with(prefix))
        })
        .collect();
    candidates.sort();
    match candidates.as_slice() {
        [] => Err(AppError::not_found(
            "no papercut matches the ID prefix",
            "Run `papercuts list --status all` and retry with a listed ID.",
        )),
        [id] => Ok(id.clone()),
        _ => Err(AppError::ambiguous_id(candidates)),
    }
}

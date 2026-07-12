use crate::error::{AppError, AppResult};
use crate::policy::{StorageIntent, StorageProfile};
use crate::{CutRecord, ItemStatus, ListItem, Resolution, ResolveRecord};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use std::thread;
use std::time::Duration;

const LOCK_ATTEMPTS: usize = 50;
const LOCK_DELAY: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageSource {
    FlagFile,
    EnvFile,
    ProfileDefault,
}

impl StorageSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FlagFile => "flag-file",
            Self::EnvFile => "env-file",
            Self::ProfileDefault => "profile-default",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationState {
    None,
    LegacyOnly,
    Dual,
}

#[derive(Debug, Clone)]
pub struct ResolvedFile {
    pub path: Option<PathBuf>,
    pub profile: StorageProfile,
    pub explicit: bool,
    pub repo: Option<PathBuf>,
    pub source: StorageSource,
    pub private_implicit: bool,
    pub migration: MigrationState,
    pub warnings: Vec<String>,
}

impl ResolvedFile {
    pub fn path(&self) -> AppResult<&Path> {
        self.path.as_deref().ok_or_else(AppError::storage_required)
    }
}

#[derive(Debug, Default)]
pub struct FoldResult {
    pub items: Vec<ListItem>,
    pub warnings: Vec<String>,
}

#[derive(Default)]
struct WarningCounts {
    torn: usize,
    malformed: usize,
    unknown: usize,
    duplicate_cuts: usize,
    duplicate_resolves: usize,
    orphans: usize,
}

#[derive(Debug, Clone)]
pub struct Repository {
    pub root: PathBuf,
    pub git_dir: PathBuf,
    pub common_dir: PathBuf,
}

pub fn discover(
    explicit_target: Option<(PathBuf, StorageSource)>,
    profile: StorageProfile,
    intent: StorageIntent,
) -> AppResult<ResolvedFile> {
    let cwd = std::env::current_dir().map_err(|error| AppError::from_io(error, Path::new(".")))?;
    let repository = resolve_repository(&cwd)?;
    if let Some((path, source)) = explicit_target {
        if path.as_os_str().is_empty() {
            return Err(AppError::invalid_argument(
                "--file requires a non-empty path",
                "Pass a non-empty --file PATH or omit the flag.",
            ));
        }
        return Ok(ResolvedFile {
            path: Some(absolute(&cwd, path)),
            profile,
            explicit: true,
            repo: repository.as_ref().map(|repo| repo.root.clone()),
            source,
            private_implicit: false,
            migration: MigrationState::None,
            warnings: exposure_warnings(profile),
        });
    }

    match profile {
        StorageProfile::Private => discover_private(repository, intent),
        StorageProfile::Committed => discover_committed(&cwd, repository),
    }
}

fn discover_private(
    repository: Option<Repository>,
    intent: StorageIntent,
) -> AppResult<ResolvedFile> {
    let Some(repository) = repository else {
        if !matches!(intent, StorageIntent::Read) {
            return Err(AppError::storage_required());
        }
        return Ok(ResolvedFile {
            path: None,
            profile: StorageProfile::Private,
            explicit: false,
            repo: None,
            source: StorageSource::ProfileDefault,
            private_implicit: true,
            migration: MigrationState::None,
            warnings: vec!["storage_required_for_writes".into()],
        });
    };
    let path = repository.common_dir.join("papercuts/log.jsonl");
    let legacy = repository.root.join(".papercuts.jsonl");
    let private_exists = try_exists(&path)?;
    let legacy_exists = try_exists(&legacy)?;
    let (migration, warnings) = match (private_exists, legacy_exists) {
        (false, true) => (
            MigrationState::LegacyOnly,
            vec!["legacy_journal_detected".into()],
        ),
        (true, true) => (MigrationState::Dual, vec!["legacy_journal_retained".into()]),
        _ => (MigrationState::None, Vec::new()),
    };
    if migration == MigrationState::LegacyOnly
        && matches!(
            intent,
            StorageIntent::Add | StorageIntent::Resolve | StorageIntent::ResolveDryRun
        )
    {
        return Err(AppError::migration_required());
    }
    Ok(ResolvedFile {
        path: Some(path),
        profile: StorageProfile::Private,
        explicit: false,
        repo: Some(repository.root),
        source: StorageSource::ProfileDefault,
        private_implicit: true,
        migration,
        warnings,
    })
}

fn discover_committed(cwd: &Path, repository: Option<Repository>) -> AppResult<ResolvedFile> {
    let (path, repo) = if let Some(repository) = repository {
        let path = repository.root.join(".papercuts.jsonl");
        (path, Some(repository.root))
    } else {
        let home = std::env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| {
                AppError::config(
                    "cannot resolve HOME for committed profile storage",
                    "Set HOME or pass an explicit --file PATH.",
                )
            })?;
        (absolute(cwd, home).join(".papercuts/log.jsonl"), None)
    };
    Ok(ResolvedFile {
        path: Some(path),
        profile: StorageProfile::Committed,
        explicit: false,
        repo,
        source: StorageSource::ProfileDefault,
        private_implicit: false,
        migration: MigrationState::None,
        warnings: exposure_warnings(StorageProfile::Committed),
    })
}

fn exposure_warnings(profile: StorageProfile) -> Vec<String> {
    if profile == StorageProfile::Committed {
        vec!["legacy_absolute_path_exposure".into()]
    } else {
        Vec::new()
    }
}

fn try_exists(path: &Path) -> AppResult<bool> {
    path.try_exists()
        .map_err(|error| AppError::from_io(error, path))
}

pub fn resolve_repository(start: &Path) -> AppResult<Option<Repository>> {
    let physical =
        std::fs::canonicalize(start).map_err(|error| AppError::from_io(error, Path::new(".")))?;
    for candidate in physical.ancestors() {
        let marker = candidate.join(".git");
        let metadata = match std::fs::symlink_metadata(&marker) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(AppError::from_io(error, &marker)),
        };
        if metadata.file_type().is_symlink() {
            return Err(AppError::invalid_repository(
                "the nearest Git marker must not be a symlink",
            ));
        }
        let git_dir = if metadata.is_dir() {
            marker
        } else if metadata.is_file() {
            let value = read_metadata_path(&marker, "gitdir:")?;
            metadata_target(candidate, value)
        } else {
            return Err(AppError::invalid_repository(
                "the nearest Git marker is neither a directory nor a gitdir file",
            ));
        };
        let git_dir = canonical_directory(&git_dir, "Git directory")?;
        require_regular_file(&git_dir.join("HEAD"), "Git HEAD")?;
        let commondir_file = git_dir.join("commondir");
        let common_dir = match std::fs::symlink_metadata(&commondir_file) {
            Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
                let value = read_single_path(&commondir_file)?;
                canonical_directory(&metadata_target(&git_dir, value), "Git common directory")?
            }
            Ok(_) => {
                return Err(AppError::invalid_repository(
                    "Git commondir metadata must be a regular file",
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => git_dir.clone(),
            Err(error) => return Err(AppError::from_io(error, &commondir_file)),
        };
        require_regular_file(&common_dir.join("config"), "Git config")?;
        require_directory(&common_dir.join("objects"), "Git objects directory")?;
        return Ok(Some(Repository {
            root: candidate.to_path_buf(),
            git_dir,
            common_dir,
        }));
    }
    Ok(None)
}

fn read_metadata_path(path: &Path, prefix: &str) -> AppResult<PathBuf> {
    let bytes = std::fs::read(path).map_err(|error| AppError::from_io(error, path))?;
    let line = single_logical_line(&bytes)?;
    let value = line
        .strip_prefix(prefix.as_bytes())
        .ok_or_else(|| AppError::invalid_repository("Git metadata has an unknown path prefix"))?;
    let value = trim_ascii_start(value);
    if value.is_empty() {
        return Err(AppError::invalid_repository(
            "Git metadata path must not be empty",
        ));
    }
    metadata_path(value)
}

fn read_single_path(path: &Path) -> AppResult<PathBuf> {
    let bytes = std::fs::read(path).map_err(|error| AppError::from_io(error, path))?;
    let line = single_logical_line(&bytes)?;
    if line.is_empty() {
        return Err(AppError::invalid_repository(
            "Git common-directory path must not be empty",
        ));
    }
    metadata_path(line)
}

fn single_logical_line(bytes: &[u8]) -> AppResult<&[u8]> {
    if bytes.contains(&0) {
        return Err(AppError::invalid_repository(
            "Git metadata must not contain NUL bytes",
        ));
    }
    let mut lines = bytes.split(|byte| *byte == b'\n');
    let first = lines.next().unwrap_or_default();
    let first = first.strip_suffix(b"\r").unwrap_or(first);
    if lines.any(|line| !line.iter().all(u8::is_ascii_whitespace)) {
        return Err(AppError::invalid_repository(
            "Git metadata must contain exactly one logical line",
        ));
    }
    Ok(first)
}

fn trim_ascii_start(mut bytes: &[u8]) -> &[u8] {
    while bytes.first().is_some_and(u8::is_ascii_whitespace) {
        bytes = &bytes[1..];
    }
    bytes
}

#[cfg(unix)]
fn metadata_path(bytes: &[u8]) -> AppResult<PathBuf> {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    Ok(PathBuf::from(OsString::from_vec(bytes.to_vec())))
}

#[cfg(not(unix))]
fn metadata_path(bytes: &[u8]) -> AppResult<PathBuf> {
    let value = std::str::from_utf8(bytes).map_err(|_| {
        AppError::invalid_repository("Git metadata path is not valid on this platform")
    })?;
    Ok(PathBuf::from(value))
}

fn metadata_target(base: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        // Preserve the kernel's symlink/`..` traversal semantics until
        // `canonicalize`; lexical normalization can select a different target.
        base.join(path)
    }
}

fn canonical_directory(path: &Path, label: &str) -> AppResult<PathBuf> {
    let canonical = std::fs::canonicalize(path)
        .map_err(|_| AppError::invalid_repository(format!("{label} is missing or unreadable")))?;
    require_directory(&canonical, label)?;
    Ok(canonical)
}

fn require_regular_file(path: &Path, label: &str) -> AppResult<()> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| AppError::invalid_repository(format!("{label} is missing or unreadable")))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(AppError::invalid_repository(format!(
            "{label} must be a regular file"
        )));
    }
    Ok(())
}

fn require_directory(path: &Path, label: &str) -> AppResult<()> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| AppError::invalid_repository(format!("{label} is missing or unreadable")))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(AppError::invalid_repository(format!(
            "{label} must be a directory"
        )));
    }
    Ok(())
}

fn absolute(cwd: &Path, path: PathBuf) -> PathBuf {
    let joined = if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

pub fn with_shared<T>(path: &Path, action: impl FnOnce(&mut File) -> AppResult<T>) -> AppResult<T> {
    let mut file = File::open(path).map_err(|error| AppError::from_log_open(error, path))?;
    lock(&file, path, false)?;
    let result = action(&mut file);
    let unlock = file
        .unlock()
        .map_err(|error| AppError::from_io(error, path));
    match (result, unlock) {
        (Err(error), _) | (Ok(_), Err(error)) => Err(error),
        (Ok(value), Ok(())) => Ok(value),
    }
}

pub fn with_shared_resolved<T>(
    resolved: &ResolvedFile,
    action: impl FnOnce(&mut File) -> AppResult<T>,
) -> AppResult<T> {
    validate_private_journal(resolved)?;
    with_shared(resolved.path()?, action)
}

pub fn with_exclusive<T>(
    path: &Path,
    create: bool,
    action: impl FnOnce(&mut File) -> AppResult<T>,
) -> AppResult<T> {
    if create && let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| AppError::from_io(error, parent))?;
    }
    let mut file = OpenOptions::new()
        .read(true)
        .append(true)
        .create(create)
        .open(path)
        .map_err(|error| AppError::from_log_open(error, path))?;
    lock(&file, path, true)?;
    let result = action(&mut file);
    let unlock = file
        .unlock()
        .map_err(|error| AppError::from_io(error, path));
    match (result, unlock) {
        (Err(error), _) | (Ok(_), Err(error)) => Err(error),
        (Ok(value), Ok(())) => Ok(value),
    }
}

pub fn with_exclusive_resolved<T>(
    resolved: &ResolvedFile,
    create: bool,
    action: impl FnOnce(&mut File) -> AppResult<T>,
) -> AppResult<T> {
    let path = resolved.path()?;
    validate_private_journal(resolved)?;
    if !resolved.private_implicit {
        return with_exclusive(path, create, action);
    }
    validate_private_permissions(resolved)?;
    if create {
        create_private_parent(path)?;
    }
    let mut options = OpenOptions::new();
    options.read(true).append(true).create(create);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| AppError::from_log_open(error, path))?;
    validate_private_permissions(resolved)?;
    lock(&file, path, true)?;
    let result = action(&mut file);
    let unlock = file
        .unlock()
        .map_err(|error| AppError::from_io(error, path));
    match (result, unlock) {
        (Err(error), _) | (Ok(_), Err(error)) => Err(error),
        (Ok(value), Ok(())) => Ok(value),
    }
}

pub fn validate_private_journal(resolved: &ResolvedFile) -> AppResult<()> {
    if resolved.profile != StorageProfile::Private {
        return Ok(());
    }
    let Some(path) = resolved.path.as_deref() else {
        return Ok(());
    };
    if resolved.private_implicit
        && let Some(parent) = path.parent()
    {
        reject_symlink(parent)?;
    }
    reject_symlink(path)
}

fn reject_symlink(path: &Path) -> AppResult<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(AppError::unsafe_journal_symlink())
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::from_io(error, path)),
    }
}

fn create_private_parent(path: &Path) -> AppResult<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.exists() {
        return Ok(());
    }
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder
        .create(parent)
        .map_err(|error| AppError::from_io(error, parent))?;
    Ok(())
}

pub fn private_permissions_secure(resolved: &ResolvedFile) -> AppResult<bool> {
    if !resolved.private_implicit {
        return Ok(true);
    }
    let Some(path) = resolved.path.as_deref() else {
        return Ok(true);
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for candidate in [path.parent(), Some(path)].into_iter().flatten() {
            match std::fs::symlink_metadata(candidate) {
                Ok(metadata) if metadata.permissions().mode() & 0o077 != 0 => return Ok(false),
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(AppError::from_io(error, candidate)),
            }
        }
    }
    Ok(true)
}

fn validate_private_permissions(resolved: &ResolvedFile) -> AppResult<()> {
    if private_permissions_secure(resolved)? {
        Ok(())
    } else {
        Err(AppError::insecure_private_permissions())
    }
}

fn lock(file: &File, path: &Path, exclusive: bool) -> AppResult<()> {
    for attempt in 0..LOCK_ATTEMPTS {
        let result = if exclusive {
            file.try_lock()
        } else {
            file.try_lock_shared()
        };
        match result {
            Ok(()) => return Ok(()),
            Err(error) => {
                let error: std::io::Error = error.into();
                if error.kind() != std::io::ErrorKind::WouldBlock {
                    return Err(AppError::from_io(error, path));
                }
                if attempt + 1 < LOCK_ATTEMPTS {
                    thread::sleep(LOCK_DELAY);
                }
            }
        }
    }
    Err(AppError::lock_timeout(path))
}

pub fn read_bytes(file: &mut File, path: &Path) -> AppResult<Vec<u8>> {
    file.seek(SeekFrom::Start(0))
        .and_then(|_| {
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).map(|_| bytes)
        })
        .map_err(|error| AppError::from_io(error, path))
}

pub fn append_json<T: serde::Serialize>(
    file: &mut File,
    path: &Path,
    prior: &[u8],
    record: &T,
) -> AppResult<()> {
    let original_len = file
        .metadata()
        .map_err(|error| AppError::from_io(error, path))?
        .len();
    let mut bytes = Vec::new();
    if !prior.is_empty() && !prior.ends_with(b"\n") {
        bytes.push(b'\n');
    }
    serde_json::to_writer(&mut bytes, record)
        .map_err(|error| AppError::internal(error.to_string()))?;
    bytes.push(b'\n');
    // If the write fails, roll back to the pre-write length; if rollback also fails, surface both.
    if let Err(error) = file.write_all(&bytes) {
        if let Err(rollback) = file.set_len(original_len) {
            return Err(AppError {
                code: "io_error",
                message: format!(
                    "append failed: {error}; rollback to original length {original_len} failed: {rollback}"
                ),
                details: json!({}),
                retryable: false,
                suggested_fix: "Check the papercuts file and filesystem, then retry.".into(),
                exit_code: 74,
                policy_meta: None,
            });
        }
        return Err(AppError::from_io(error, path));
    }
    Ok(())
}

pub fn fold_bytes(bytes: &[u8]) -> FoldResult {
    let mut cuts = BTreeMap::<String, CutRecord>::new();
    let mut resolves = HashMap::<String, ResolveRecord>::new();
    let mut counts = WarningCounts::default();
    let complete_len = if bytes.is_empty() || bytes.ends_with(b"\n") {
        bytes.len()
    } else {
        counts.torn += 1;
        bytes
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |i| i + 1)
    };

    let complete = &bytes[..complete_len];
    let complete = complete.strip_suffix(b"\n").unwrap_or(complete);
    for raw in complete.split(|byte| *byte == b'\n') {
        if complete.is_empty() {
            break;
        }
        let Ok(value) = serde_json::from_slice::<Value>(raw) else {
            counts.malformed += 1;
            continue;
        };
        match value.get("kind").and_then(Value::as_str) {
            Some("cut") => match serde_json::from_value::<CutRecord>(value) {
                Ok(mut cut) => {
                    if cut.ts.parse::<jiff::Timestamp>().is_err() {
                        counts.malformed += 1;
                        continue;
                    }
                    cut.tags.sort();
                    if cuts.contains_key(&cut.id) {
                        counts.duplicate_cuts += 1;
                    } else {
                        cuts.insert(cut.id.clone(), cut);
                    }
                }
                Err(_) => counts.malformed += 1,
            },
            Some("resolve") => match serde_json::from_value::<ResolveRecord>(value) {
                Ok(resolve) => {
                    if resolve.ts.parse::<jiff::Timestamp>().is_err() {
                        counts.malformed += 1;
                        continue;
                    }
                    if resolves.contains_key(&resolve.id) {
                        counts.duplicate_resolves += 1;
                    } else {
                        resolves.insert(resolve.id.clone(), resolve);
                    }
                }
                Err(_) => counts.malformed += 1,
            },
            _ => counts.unknown += 1,
        }
    }

    for id in resolves.keys() {
        if !cuts.contains_key(id) {
            counts.orphans += 1;
        }
    }
    let mut items: Vec<_> = cuts
        .into_values()
        .map(|cut| {
            let resolution = resolves.get(&cut.id).map(|resolve| Resolution {
                ts: resolve.ts.clone(),
                agent: resolve.agent.clone(),
                note: resolve.note.clone(),
                content_policy: resolve.content_policy.clone(),
            });
            ListItem {
                status: if resolution.is_some() {
                    ItemStatus::Resolved
                } else {
                    ItemStatus::Open
                },
                cut,
                resolution,
            }
        })
        .collect();
    items.sort_by(|left, right| {
        let left_ts = left.cut.ts.parse::<jiff::Timestamp>().ok();
        let right_ts = right.cut.ts.parse::<jiff::Timestamp>().ok();
        right
            .cut
            .severity
            .rank()
            .cmp(&left.cut.severity.rank())
            .then_with(|| right_ts.cmp(&left_ts))
            .then_with(|| left.cut.id.cmp(&right.cut.id))
    });

    let mut warnings = Vec::new();
    warning(&mut warnings, counts.torn, "torn final line");
    warning(&mut warnings, counts.malformed, "malformed line");
    warning(&mut warnings, counts.unknown, "unknown event");
    warning(&mut warnings, counts.duplicate_cuts, "duplicate cut");
    warning(
        &mut warnings,
        counts.duplicate_resolves,
        "duplicate resolve",
    );
    warning(&mut warnings, counts.orphans, "orphan resolve");
    FoldResult { items, warnings }
}

fn warning(warnings: &mut Vec<String>, count: usize, label: &str) {
    if count > 0 {
        warnings.push(format!(
            "skipped {count} {label}{}",
            if count == 1 { "" } else { "s" }
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Severity, compute_id};

    fn cut(id: &str) -> String {
        cut_with_text(id, "x")
    }

    fn cut_with_text(id: &str, text: &str) -> String {
        serde_json::json!({
            "kind":"cut", "id":id, "ts":"2026-07-09T00:00:00.000Z",
            "agent":"a", "text":text, "tags":[], "severity":"minor",
            "cwd":"/tmp", "repo":null
        })
        .to_string()
    }

    fn resolve(id: &str) -> String {
        serde_json::json!({
            "kind":"resolve", "id":id, "ts":"2026-07-10T00:00:00.000Z",
            "agent":"a", "note":null
        })
        .to_string()
    }

    #[test]
    fn fold_matrix() {
        let id = compute_id("2026-07-09T00:00:00.000Z", "a", "x", Severity::Minor, &[]);
        let cases = [
            ("cut", format!("{}\n", cut(&id)), 1, ItemStatus::Open, 0),
            (
                "resolve before cut",
                format!("{}\n{}\n", resolve(&id), cut(&id)),
                1,
                ItemStatus::Resolved,
                0,
            ),
            (
                "duplicates",
                format!(
                    "{}\n{}\n{}\n{}\n",
                    cut(&id),
                    cut(&id),
                    resolve(&id),
                    resolve(&id)
                ),
                1,
                ItemStatus::Resolved,
                2,
            ),
            (
                "unknown malformed orphan",
                format!(
                    "{{\"kind\":\"future\"}}\nnope\n{}\n{}\n",
                    resolve("pc_deadbeef0000"),
                    cut(&id)
                ),
                1,
                ItemStatus::Open,
                3,
            ),
            (
                "torn tail",
                format!("{}\n{{\"kind\":", cut(&id)),
                1,
                ItemStatus::Open,
                1,
            ),
            (
                "all adversarial orderings interleaved",
                format!(
                    "{}\n{{\"kind\":\"future\"}}\n{}\n{}\n{}\n{}\n{}\nnope\n{{\"kind\":",
                    resolve(&id),
                    cut(&id),
                    cut(&id),
                    cut_with_text(&id, "conflicting payload"),
                    resolve(&id),
                    resolve("pc_deadbeef0000"),
                ),
                1,
                ItemStatus::Resolved,
                6,
            ),
        ];
        for (name, input, item_count, status, warning_count) in cases {
            let folded = fold_bytes(input.as_bytes());
            assert_eq!(folded.items.len(), item_count, "{name}");
            if !folded.items.is_empty() {
                assert_eq!(folded.items[0].status, status, "{name}");
                assert_eq!(folded.items[0].cut.text, "x", "{name}");
            }
            assert_eq!(folded.warnings.len(), warning_count, "{name}");
        }
    }
}

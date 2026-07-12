use crate::error::{AppError, AppResult};
use crate::store::{self, ResolvedFile, StorageSource};
use crate::{CutRecord, ListItem, PathEncoding, RecordPathPolicy};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum, PartialOrd, Ord,
)]
#[serde(rename_all = "lowercase")]
pub enum StorageProfile {
    Private,
    Committed,
}

impl StorageProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Committed => "committed",
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum, PartialOrd, Ord,
)]
#[serde(rename_all = "lowercase")]
pub enum SensitivePolicy {
    Balanced,
    Strict,
}

impl SensitivePolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Balanced => "balanced",
            Self::Strict => "strict",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Balanced => 0,
            Self::Strict => 1,
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "snake_case")]
pub enum SensitiveCategory {
    PrivateKey,
    AuthorizationHeader,
    CredentialUrl,
    SecretAssignment,
    GithubToken,
    SlackToken,
    StripeSecretKey,
    AwsCredentialPair,
    EmailAddress,
    PersonalIdentifier,
    FilesystemPath,
    ConfigBlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileSource {
    Flag,
    Env,
    Default,
}

impl ProfileSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Flag => "flag-profile",
            Self::Env => "env-profile",
            Self::Default => "default",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensitivePolicySource {
    Flag,
    Env,
    ProfileDefault,
}

impl SensitivePolicySource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Flag => "flag",
            Self::Env => "env",
            Self::ProfileDefault => "profile-default",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritePolicy {
    Normal,
    ReadOnly,
}

impl WritePolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::ReadOnly => "read-only",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPolicy {
    Omitted,
    LegacyAbsolute,
}

impl PathPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Omitted => "omitted",
            Self::LegacyAbsolute => "legacy-absolute",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageIntent {
    Read,
    AddDryRun,
    Add,
    ResolveDryRun,
    Resolve,
}

impl StorageIntent {
    pub fn is_actual_mutation(self) -> bool {
        matches!(self, Self::Add | Self::Resolve)
    }
}

#[derive(Debug)]
pub enum Operation {
    List,
    Doctor,
    Add {
        dry_run: bool,
        agent: Option<String>,
        allow_sensitive: Vec<SensitiveCategory>,
    },
    Resolve {
        dry_run: bool,
        agent: Option<String>,
        allow_sensitive: Vec<SensitiveCategory>,
    },
}

impl Operation {
    fn intent(&self) -> StorageIntent {
        match self {
            Self::List | Self::Doctor => StorageIntent::Read,
            Self::Add { dry_run: true, .. } => StorageIntent::AddDryRun,
            Self::Add { dry_run: false, .. } => StorageIntent::Add,
            Self::Resolve { dry_run: true, .. } => StorageIntent::ResolveDryRun,
            Self::Resolve { dry_run: false, .. } => StorageIntent::Resolve,
        }
    }

    fn ingestion(&self) -> bool {
        matches!(self, Self::Add { .. } | Self::Resolve { .. })
    }

    fn agent_flag(&self) -> Option<String> {
        match self {
            Self::Add { agent, .. } | Self::Resolve { agent, .. } => agent.clone(),
            Self::List | Self::Doctor => None,
        }
    }

    fn allowed_categories(&self) -> Vec<SensitiveCategory> {
        match self {
            Self::Add {
                allow_sensitive, ..
            }
            | Self::Resolve {
                allow_sensitive, ..
            } => allow_sensitive.clone(),
            Self::List | Self::Doctor => Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentIdentity {
    pub value: String,
    pub source: &'static str,
}

#[derive(Debug)]
pub struct PolicyContext {
    pub profile: StorageProfile,
    pub profile_source: ProfileSource,
    pub storage: ResolvedFile,
    pub write_policy: WritePolicy,
    pub path_policy: PathPolicy,
    pub sensitive_policy: Option<SensitivePolicy>,
    pub sensitive_policy_source: Option<SensitivePolicySource>,
    pub allow_sensitive: Vec<SensitiveCategory>,
    pub agent: Option<AgentIdentity>,
}

impl PolicyContext {
    pub fn effective_now(&self) -> AppResult<Timestamp> {
        crate::effective_now()
    }

    pub fn project_cut(&self, mut cut: CutRecord) -> (CutRecord, bool) {
        let retained_legacy = cut.path_policy != Some(RecordPathPolicy::Omitted);
        if self.profile == StorageProfile::Private {
            cut.cwd = ".".into();
            cut.repo = None;
            cut.path_policy = Some(RecordPathPolicy::Omitted);
            cut.path_encoding = Some(PathEncoding::Omitted);
        }
        (cut, retained_legacy)
    }

    pub fn project_item(&self, item: ListItem) -> (ListItem, bool) {
        let (cut, retained_legacy) = self.project_cut(item.cut);
        (
            ListItem {
                cut,
                status: item.status,
                resolution: item.resolution,
            },
            retained_legacy,
        )
    }
}

pub fn resolve(
    file: Option<PathBuf>,
    profile_flag: Option<StorageProfile>,
    read_only_flag: bool,
    sensitive_policy_flag: Option<SensitivePolicy>,
    operation: Operation,
) -> AppResult<PolicyContext> {
    resolve_with_preflight(
        file,
        profile_flag,
        read_only_flag,
        sensitive_policy_flag,
        operation,
        || Ok(()),
    )
}

pub fn resolve_with_preflight(
    file: Option<PathBuf>,
    profile_flag: Option<StorageProfile>,
    read_only_flag: bool,
    sensitive_policy_flag: Option<SensitivePolicy>,
    operation: Operation,
    preflight: impl FnOnce() -> AppResult<()>,
) -> AppResult<PolicyContext> {
    let (profile, profile_source) = resolve_profile(profile_flag)?;
    let env_read_only = env_bool("PAPERCUTS_READ_ONLY")?.unwrap_or(false);
    let read_only = read_only_flag || env_read_only;
    let write_policy = if read_only {
        WritePolicy::ReadOnly
    } else {
        WritePolicy::Normal
    };
    let (sensitive_policy, sensitive_policy_source, allow_sensitive, agent) = if operation
        .ingestion()
    {
        let (policy, source) = resolve_sensitive_policy(profile, sensitive_policy_flag)?;
        let gate = env_bool("PAPERCUTS_ALLOW_SENSITIVE")?.unwrap_or(false);
        let mut categories = operation.allowed_categories();
        categories.sort();
        categories.dedup();
        if !categories.is_empty() && !gate {
            return Err(AppError::config(
                "--allow-sensitive requires the PAPERCUTS_ALLOW_SENSITIVE gate",
                "Remove --allow-sensitive unless an operator has enabled the matching environment gate.",
            ));
        }
        (
            Some(policy),
            Some(source),
            categories,
            Some(resolve_agent(operation.agent_flag())?),
        )
    } else {
        (None, None, Vec::new(), None)
    };

    let intent = operation.intent();
    if intent.is_actual_mutation() && read_only {
        return Err(AppError::writes_disabled());
    }
    preflight()?;

    let path_policy = match profile {
        StorageProfile::Private => PathPolicy::Omitted,
        StorageProfile::Committed => PathPolicy::LegacyAbsolute,
    };
    let explicit_target = resolve_explicit_target(file)?;
    let storage_source = explicit_target
        .as_ref()
        .map_or(StorageSource::ProfileDefault, |(_, source)| *source);
    let private_error_location = if explicit_target.is_some() {
        "explicit_journal"
    } else {
        "private_journal"
    };
    let storage = store::discover(explicit_target, profile, intent).map_err(|error| {
        if profile == StorageProfile::Private {
            let location = if error.code == "invalid_repository" {
                "repository_marker"
            } else {
                private_error_location
            };
            error.sanitize_private_path(location).with_policy_parts(
                profile,
                profile_source,
                storage_source,
                write_policy,
                path_policy,
                None,
            )
        } else {
            error
        }
    })?;
    store::validate_private_journal(&storage).map_err(|error| {
        if profile == StorageProfile::Private {
            error
                .sanitize_private_path(if storage.explicit {
                    "explicit_journal"
                } else {
                    "private_journal"
                })
                .with_policy_parts(
                    profile,
                    profile_source,
                    storage.source,
                    write_policy,
                    path_policy,
                    None,
                )
        } else {
            error
        }
    })?;
    Ok(PolicyContext {
        profile,
        profile_source,
        storage,
        write_policy,
        path_policy,
        sensitive_policy,
        sensitive_policy_source,
        allow_sensitive,
        agent,
    })
}

fn resolve_explicit_target(flag: Option<PathBuf>) -> AppResult<Option<(PathBuf, StorageSource)>> {
    if let Some(path) = flag {
        if path.as_os_str().is_empty() {
            return Err(AppError::invalid_argument(
                "--file requires a non-empty path",
                "Pass a non-empty --file PATH or omit the flag.",
            ));
        }
        return Ok(Some((path, StorageSource::FlagFile)));
    }
    Ok(std::env::var_os("PAPERCUTS_FILE")
        .filter(|value| !value.is_empty())
        .map(|path| (PathBuf::from(path), StorageSource::EnvFile)))
}

fn resolve_profile(flag: Option<StorageProfile>) -> AppResult<(StorageProfile, ProfileSource)> {
    if let Some(profile) = flag {
        return Ok((profile, ProfileSource::Flag));
    }
    if let Some(value) = env_text("PAPERCUTS_PROFILE")? {
        let profile = match value.to_ascii_lowercase().as_str() {
            "private" => StorageProfile::Private,
            "committed" => StorageProfile::Committed,
            _ => {
                return Err(AppError::config(
                    "PAPERCUTS_PROFILE must be private or committed",
                    "Set PAPERCUTS_PROFILE to private or committed, or unset it.",
                ));
            }
        };
        return Ok((profile, ProfileSource::Env));
    }
    Ok((StorageProfile::Private, ProfileSource::Default))
}

fn resolve_sensitive_policy(
    profile: StorageProfile,
    flag: Option<SensitivePolicy>,
) -> AppResult<(SensitivePolicy, SensitivePolicySource)> {
    let floor = match profile {
        StorageProfile::Private => SensitivePolicy::Balanced,
        StorageProfile::Committed => SensitivePolicy::Strict,
    };
    let (policy, source) = if let Some(policy) = flag {
        (policy, SensitivePolicySource::Flag)
    } else if let Some(value) = env_text("PAPERCUTS_SENSITIVE_POLICY")? {
        let policy = match value.to_ascii_lowercase().as_str() {
            "balanced" => SensitivePolicy::Balanced,
            "strict" => SensitivePolicy::Strict,
            _ => {
                return Err(AppError::config(
                    "PAPERCUTS_SENSITIVE_POLICY must be balanced or strict",
                    "Set PAPERCUTS_SENSITIVE_POLICY to balanced or strict, or unset it.",
                ));
            }
        };
        (policy, SensitivePolicySource::Env)
    } else {
        (floor, SensitivePolicySource::ProfileDefault)
    };
    if policy.rank() < floor.rank() {
        return Err(AppError::config(
            "the requested sensitive policy is weaker than the active profile floor",
            "Use a sensitive policy at least as strict as the selected storage profile requires.",
        ));
    }
    Ok((policy, source))
}

fn resolve_agent(flag: Option<String>) -> AppResult<AgentIdentity> {
    if let Some(value) = flag {
        return Ok(AgentIdentity {
            value,
            source: "flag",
        });
    }
    if let Some(value) = env_text("PAPERCUTS_AGENT")? {
        return Ok(AgentIdentity {
            value,
            source: "env",
        });
    }
    if std::env::var_os("CLAUDECODE").is_some() {
        return Ok(AgentIdentity {
            value: "claude-code".into(),
            source: "detected",
        });
    }
    if std::env::vars_os().any(|(key, _)| key.to_string_lossy().starts_with("CODEX_")) {
        return Ok(AgentIdentity {
            value: "codex".into(),
            source: "detected",
        });
    }
    if std::env::vars_os().any(|(key, _)| key.to_string_lossy().starts_with("CURSOR_")) {
        return Ok(AgentIdentity {
            value: "cursor".into(),
            source: "detected",
        });
    }
    Ok(AgentIdentity {
        value: "unknown".into(),
        source: "default",
    })
}

fn env_text(name: &'static str) -> AppResult<Option<String>> {
    match std::env::var(name) {
        Ok(value) if value.is_empty() => Ok(None),
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(AppError::config(
            format!("{name} must be valid UTF-8"),
            format!("Set {name} to a supported UTF-8 value or unset it."),
        )),
    }
}

fn env_bool(name: &'static str) -> AppResult<Option<bool>> {
    let Some(value) = env_text(name)? else {
        return Ok(None);
    };
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" => Ok(Some(true)),
        "0" | "false" => Ok(Some(false)),
        _ => Err(AppError::config(
            format!("{name} must be 0, 1, false, or true"),
            format!("Set {name} to 0, 1, false, or true, or unset it."),
        )),
    }
}

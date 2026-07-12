use crate::error::AppError;
use crate::policy::{PolicyContext, StorageProfile};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, Write};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub contract: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensitive_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensitive_policy_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensitive_policy_version: Option<u8>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

impl Meta {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_policy(context: &PolicyContext, include_sensitive: bool) -> Self {
        let mut meta = Self {
            storage_profile: Some(context.profile.as_str().into()),
            profile_source: Some(context.profile_source.as_str().into()),
            storage_source: Some(context.storage.source.as_str().into()),
            write_policy: Some(context.write_policy.as_str().into()),
            path_policy: Some(context.path_policy.as_str().into()),
            warnings: context.storage.warnings.clone(),
            ..Self::default()
        };
        if context.profile == StorageProfile::Committed {
            meta.file = context
                .storage
                .path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned());
        }
        if include_sensitive {
            meta.sensitive_policy = context
                .sensitive_policy
                .map(|policy| policy.as_str().into());
            meta.sensitive_policy_source = context
                .sensitive_policy_source
                .map(|source| source.as_str().into());
            meta.sensitive_policy_version = Some(1);
        }
        meta
    }
}

impl Default for Meta {
    fn default() -> Self {
        Self {
            contract: 2,
            file: None,
            agent_source: None,
            storage_profile: None,
            profile_source: None,
            storage_source: None,
            write_policy: None,
            path_policy: None,
            sensitive_policy: None,
            sensitive_policy_source: None,
            sensitive_policy_version: None,
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SuccessEnvelope<T> {
    pub ok: bool,
    pub data: T,
    pub meta: Meta,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorEnvelope {
    pub ok: bool,
    pub error: ErrorBody,
    pub meta: Meta,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    pub details: Value,
    pub retryable: bool,
    pub suggested_fix: String,
}

pub fn write_success<T: Serialize>(data: T, pretty: bool, mut meta: Meta) -> io::Result<()> {
    meta.warnings.sort();
    meta.warnings.dedup();
    let envelope = SuccessEnvelope {
        ok: true,
        data,
        meta,
    };
    let mut output = io::BufWriter::new(io::stdout().lock());
    if pretty {
        serde_json::to_writer_pretty(&mut output, &envelope)?;
    } else {
        serde_json::to_writer(&mut output, &envelope)?;
    }
    writeln!(output)
}

pub fn write_error(error: &AppError) -> i32 {
    let mut meta = Meta::new();
    if let Some(policy) = error.policy_meta.as_deref() {
        meta.storage_profile = Some(policy.storage_profile.clone());
        meta.profile_source = Some(policy.profile_source.clone());
        meta.storage_source = Some(policy.storage_source.clone());
        meta.write_policy = Some(policy.write_policy.clone());
        meta.path_policy = Some(policy.path_policy.clone());
        meta.file = policy.file.clone();
    }
    let envelope = ErrorEnvelope {
        ok: false,
        error: ErrorBody {
            code: error.code.into(),
            message: error.message.clone(),
            details: error.details.clone(),
            retryable: error.retryable,
            suggested_fix: error.suggested_fix.clone(),
        },
        meta,
    };
    let mut output = io::BufWriter::new(io::stderr().lock());
    let _ = serde_json::to_writer(&mut output, &envelope);
    let _ = writeln!(output);
    error.exit_code
}

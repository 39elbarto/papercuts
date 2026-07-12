use serde_json::{Value, json};
use std::collections::BTreeMap;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
#[error("{message}")]
pub struct AppError {
    pub code: &'static str,
    pub message: String,
    pub details: Value,
    pub retryable: bool,
    pub suggested_fix: String,
    pub exit_code: i32,
    pub policy_meta: Option<Box<ErrorPolicyMeta>>,
}

#[derive(Debug, Clone)]
pub struct ErrorPolicyMeta {
    pub storage_profile: String,
    pub profile_source: String,
    pub storage_source: String,
    pub write_policy: String,
    pub path_policy: String,
    pub file: Option<String>,
}

/// Single source of truth for every public error code, its exit code, and the
/// description published in `papercuts schema`.
pub struct ErrorContract {
    pub code: &'static str,
    pub exit_code: i32,
    pub description: &'static str,
}

impl ErrorContract {
    pub fn retryable(&self) -> bool {
        self.code == "lock_timeout"
    }
}

pub const EXIT_CONTRACT: &[(i32, &str)] = &[
    (0, "success or empty result"),
    (1, "doctor findings"),
    (2, "invalid command arguments"),
    (65, "invalid or refused input data"),
    (66, "missing explicit file or unknown ID"),
    (70, "internal error"),
    (74, "I/O error"),
    (75, "temporary lock timeout; retryable"),
    (77, "permission denied or insecure private permissions"),
    (78, "configuration or policy error"),
];

pub const ERROR_CONTRACT: &[ErrorContract] = &[
    ErrorContract {
        code: "invalid_argument",
        exit_code: 2,
        description: "invalid arguments",
    },
    ErrorContract {
        code: "invalid_input",
        exit_code: 65,
        description: "invalid input data",
    },
    ErrorContract {
        code: "sensitive_input",
        exit_code: 65,
        description: "input refused by the sensitive-data guardrail",
    },
    ErrorContract {
        code: "not_found",
        exit_code: 66,
        description: "missing explicit file or unknown ID",
    },
    ErrorContract {
        code: "ambiguous_id",
        exit_code: 65,
        description: "invalid input data including ambiguous ID",
    },
    ErrorContract {
        code: "io_error",
        exit_code: 74,
        description: "I/O error",
    },
    ErrorContract {
        code: "permission_denied",
        exit_code: 77,
        description: "permission denied",
    },
    ErrorContract {
        code: "lock_timeout",
        exit_code: 75,
        description: "lock timeout; retryable",
    },
    ErrorContract {
        code: "config_error",
        exit_code: 78,
        description: "configuration error",
    },
    ErrorContract {
        code: "writes_disabled",
        exit_code: 78,
        description: "writes disabled by monotonic policy",
    },
    ErrorContract {
        code: "storage_required",
        exit_code: 78,
        description: "explicit storage required",
    },
    ErrorContract {
        code: "migration_required",
        exit_code: 78,
        description: "explicit storage migration required",
    },
    ErrorContract {
        code: "invalid_repository",
        exit_code: 78,
        description: "invalid repository metadata",
    },
    ErrorContract {
        code: "unsupported_filesystem",
        exit_code: 78,
        description: "unsupported filesystem",
    },
    ErrorContract {
        code: "unsafe_journal_symlink",
        exit_code: 78,
        description: "unsafe private journal symlink",
    },
    ErrorContract {
        code: "insecure_private_permissions",
        exit_code: 77,
        description: "private storage is not user-only",
    },
    ErrorContract {
        code: "internal",
        exit_code: 70,
        description: "internal error",
    },
];

pub fn exit_code_for(code: &str) -> i32 {
    ERROR_CONTRACT
        .iter()
        .find(|entry| entry.code == code)
        .map_or(70, |entry| entry.exit_code)
}

pub fn error_codes() -> Vec<&'static str> {
    ERROR_CONTRACT.iter().map(|entry| entry.code).collect()
}

pub fn exit_code_map() -> BTreeMap<i32, &'static str> {
    EXIT_CONTRACT.iter().copied().collect()
}

impl AppError {
    pub fn invalid_argument(message: impl Into<String>, fix: impl Into<String>) -> Self {
        Self::new("invalid_argument", message, false, fix)
    }

    pub fn invalid_input(message: impl Into<String>, fix: impl Into<String>) -> Self {
        Self::new("invalid_input", message, false, fix)
    }

    pub fn unused_sensitive_override(categories: Vec<crate::policy::SensitiveCategory>) -> Self {
        let mut error = Self::invalid_input(
            "one or more sensitive-data overrides were not required",
            "Remove unused --allow-sensitive categories and retry.",
        );
        error.details = json!({
            "categories": categories
                .into_iter()
                .map(crate::policy::SensitiveCategory::as_str)
                .collect::<Vec<_>>()
        });
        error
    }

    pub fn sensitive_input(
        policy: crate::policy::SensitivePolicy,
        categories: Vec<crate::policy::SensitiveCategory>,
        fields: Vec<crate::sensitive::SensitiveField>,
    ) -> Self {
        let mut error = Self::new(
            "sensitive_input",
            "input matched the sensitive-data guardrail",
            false,
            "Replace sensitive values with a non-sensitive description, then retry. Review the original value outside Papercuts.",
        );
        error.details = json!({
            "policy_version": crate::sensitive::POLICY_VERSION,
            "policy": policy.as_str(),
            "categories": categories
                .into_iter()
                .map(crate::policy::SensitiveCategory::as_str)
                .collect::<Vec<_>>(),
            "fields": fields
                .into_iter()
                .map(crate::sensitive::SensitiveField::as_str)
                .collect::<Vec<_>>(),
        });
        error
    }

    pub fn not_found(message: impl Into<String>, fix: impl Into<String>) -> Self {
        Self::new("not_found", message, false, fix)
    }

    pub fn ambiguous_id(candidates: Vec<String>) -> Self {
        let mut error = Self::new(
            "ambiguous_id",
            "the ID prefix matches multiple papercuts",
            false,
            "Use one of the full IDs listed in error.details.candidates.",
        );
        error.details = json!({ "candidates": candidates });
        error
    }

    pub fn config(message: impl Into<String>, fix: impl Into<String>) -> Self {
        Self::new("config_error", message, false, fix)
    }

    pub fn writes_disabled() -> Self {
        Self::new(
            "writes_disabled",
            "writes are disabled for this invocation",
            false,
            "Continue without appending, or ask the operator to run an approved write-capable task.",
        )
    }

    pub fn storage_required() -> Self {
        Self::new(
            "storage_required",
            "private storage requires an explicit journal outside a Git working tree",
            false,
            "Choose an approved private location and pass it with --file or PAPERCUTS_FILE.",
        )
    }

    pub fn migration_required() -> Self {
        Self::new(
            "migration_required",
            "a legacy journal exists but private storage has not been selected by migration",
            false,
            "Review and copy the legacy journal with the documented copy-and-verify procedure before writing.",
        )
    }

    pub fn invalid_repository(message: impl Into<String>) -> Self {
        Self::new(
            "invalid_repository",
            message,
            false,
            "Repair the nearest Git metadata or use an explicit approved journal outside that repository.",
        )
    }

    pub fn insecure_private_permissions() -> Self {
        Self::new(
            "insecure_private_permissions",
            "implicit private storage is accessible beyond the current user",
            false,
            "Review the private journal and directory permissions, then restrict them to the current user.",
        )
    }

    pub fn unsafe_journal_symlink() -> Self {
        Self::new(
            "unsafe_journal_symlink",
            "the selected private journal must not be a symlink",
            false,
            "Choose a regular private journal file and review the storage location without pasting it into logs.",
        )
    }

    pub fn lock_timeout(path: &std::path::Path) -> Self {
        Self::new(
            "lock_timeout",
            format!(
                "timed out waiting for the papercuts file lock: {}",
                path.display()
            ),
            true,
            "Retry the same command after the other papercuts process finishes.",
        )
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(
            "internal",
            message,
            false,
            "Run `papercuts doctor`; if the problem persists, report the command and papercuts version.",
        )
    }

    pub fn from_io(error: std::io::Error, path: &std::path::Path) -> Self {
        let kind = error.kind();
        let mut app_error = match kind {
            std::io::ErrorKind::PermissionDenied => Self::new(
                "permission_denied",
                format!("permission denied for {}: {error}", path.display()),
                false,
                "Choose a writable path with --file or correct the file permissions.",
            ),
            _ => Self::new(
                "io_error",
                format!("I/O error for {}: {error}", path.display()),
                false,
                "Check that the path exists and its filesystem is available, then retry.",
            ),
        };
        app_error.details = json!({
            "os_kind": os_kind(kind),
            "location": known_location(path),
        });
        app_error
    }

    /// Error mapping for opening an existing papercuts log file. This is the
    /// only place where `NotFound` is mapped to `not_found` / 66.
    pub fn from_log_open(error: std::io::Error, path: &std::path::Path) -> Self {
        if error.kind() == std::io::ErrorKind::NotFound {
            let mut app_error = Self::new(
                "not_found",
                format!("papercuts file not found: {}", path.display()),
                false,
                "Run `papercuts add` to create the file or pass an existing --file PATH.",
            );
            app_error.details = json!({
                "os_kind": "not-found",
                "location": known_location(path),
            });
            app_error
        } else {
            Self::from_io(error, path)
        }
    }

    pub fn with_policy(mut self, context: &crate::policy::PolicyContext) -> Self {
        let sensitive_diagnostic = self.code == "sensitive_input"
            || (self.code == "invalid_input" && self.details.get("categories").is_some());
        let file = if context.profile == crate::policy::StorageProfile::Committed
            && !sensitive_diagnostic
        {
            context
                .storage
                .path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned())
        } else {
            None
        };
        self = self.with_policy_parts(
            context.profile,
            context.profile_source,
            context.storage.source,
            context.write_policy,
            context.path_policy,
            file,
        );
        if context.profile == crate::policy::StorageProfile::Private {
            let location = self
                .details
                .get("location")
                .and_then(Value::as_str)
                .filter(|location| !location.is_empty())
                .unwrap_or(if context.storage.explicit {
                    "explicit_journal"
                } else {
                    "private_journal"
                })
                .to_string();
            self = self.sanitize_private_path(&location);
        }
        self
    }

    pub fn with_policy_parts(
        mut self,
        profile: crate::policy::StorageProfile,
        profile_source: crate::policy::ProfileSource,
        storage_source: crate::store::StorageSource,
        write_policy: crate::policy::WritePolicy,
        path_policy: crate::policy::PathPolicy,
        file: Option<String>,
    ) -> Self {
        self.policy_meta = Some(Box::new(ErrorPolicyMeta {
            storage_profile: profile.as_str().into(),
            profile_source: profile_source.as_str().into(),
            storage_source: storage_source.as_str().into(),
            write_policy: write_policy.as_str().into(),
            path_policy: path_policy.as_str().into(),
            file,
        }));
        self
    }

    pub fn sanitize_private_path(mut self, default_location: &str) -> Self {
        if !matches!(
            self.code,
            "io_error"
                | "permission_denied"
                | "not_found"
                | "lock_timeout"
                | "invalid_repository"
                | "unsupported_filesystem"
                | "unsafe_journal_symlink"
                | "insecure_private_permissions"
        ) {
            return self;
        }
        let location = self
            .details
            .get("location")
            .and_then(Value::as_str)
            .filter(|location| !location.is_empty())
            .unwrap_or(default_location)
            .to_string();
        let os_kind = self
            .details
            .get("os_kind")
            .and_then(Value::as_str)
            .map(str::to_string);
        self.message = match self.code {
            "permission_denied" => "permission denied for the selected private location",
            "not_found" => "the selected private journal was not found",
            "lock_timeout" => "timed out waiting for the selected private journal lock",
            "invalid_repository" => "the nearest Git metadata is invalid",
            "unsupported_filesystem" => "the selected private filesystem is unsupported",
            "unsafe_journal_symlink" => "the selected private journal must not be a symlink",
            "insecure_private_permissions" => {
                "implicit private storage is accessible beyond the current user"
            }
            _ => "I/O error for the selected private location",
        }
        .into();
        self.details = json!({ "location": location });
        if let Some(os_kind) = os_kind {
            self.details["os_kind"] = json!(os_kind);
        }
        self.suggested_fix =
            "Review the selected private location without pasting its path into logs.".into();
        self
    }

    fn new(
        code: &'static str,
        message: impl Into<String>,
        retryable: bool,
        suggested_fix: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            details: json!({}),
            retryable,
            suggested_fix: suggested_fix.into(),
            exit_code: exit_code_for(code),
            policy_meta: None,
        }
    }
}

fn known_location(path: &std::path::Path) -> Option<&'static str> {
    match path.to_str() {
        Some("stdin") => Some("stdin"),
        Some("stdout") => Some("stdout"),
        Some(".") => Some("current_working_directory"),
        _ => None,
    }
}

fn os_kind(kind: std::io::ErrorKind) -> &'static str {
    match kind {
        std::io::ErrorKind::NotFound => "not-found",
        std::io::ErrorKind::PermissionDenied => "permission-denied",
        std::io::ErrorKind::AlreadyExists => "already-exists",
        std::io::ErrorKind::WouldBlock => "would-block",
        std::io::ErrorKind::InvalidInput => "invalid-input",
        std::io::ErrorKind::InvalidData => "invalid-data",
        std::io::ErrorKind::TimedOut => "timed-out",
        std::io::ErrorKind::WriteZero => "write-zero",
        std::io::ErrorKind::UnexpectedEof => "unexpected-eof",
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::io::ErrorKind;

    #[test]
    fn io_not_found_maps_to_io_error_74() {
        let error = std::io::Error::new(ErrorKind::NotFound, "missing");
        let err = AppError::from_io(error, std::path::Path::new("/tmp/x"));
        assert_eq!(err.code, "io_error");
        assert_eq!(err.exit_code, 74);
    }

    #[test]
    fn log_open_not_found_maps_to_not_found_66() {
        let error = std::io::Error::new(ErrorKind::NotFound, "missing");
        let err = AppError::from_log_open(error, std::path::Path::new("/tmp/x"));
        assert_eq!(err.code, "not_found");
        assert_eq!(err.exit_code, 66);
    }

    #[test]
    fn public_error_and_exit_contracts_are_unique_and_complete() {
        let mut codes = HashSet::new();
        for contract in ERROR_CONTRACT {
            assert!(codes.insert(contract.code), "duplicate {}", contract.code);
            assert_eq!(exit_code_for(contract.code), contract.exit_code);
            assert!(!contract.description.is_empty());
        }
        let exits = exit_code_map();
        assert_eq!(exits.len(), EXIT_CONTRACT.len());
        for (code, description) in EXIT_CONTRACT {
            assert_eq!(exits.get(code), Some(description));
        }
        assert!(ERROR_CONTRACT.iter().any(|entry| entry.retryable()));
        assert!(
            ERROR_CONTRACT
                .iter()
                .filter(|entry| entry.retryable())
                .all(|entry| entry.code == "lock_timeout")
        );
    }
}

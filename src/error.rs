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
}

/// Single source of truth for every public error code, its exit code, and the
/// description published in `papercuts schema`.
pub struct ErrorContract {
    pub code: &'static str,
    pub exit_code: i32,
    pub description: &'static str,
}

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
    let mut map = BTreeMap::new();
    map.insert(0, "success or empty result");
    for entry in ERROR_CONTRACT {
        map.insert(entry.exit_code, entry.description);
    }
    map.insert(1, "doctor findings");
    map
}

impl AppError {
    pub fn invalid_argument(message: impl Into<String>, fix: impl Into<String>) -> Self {
        Self::new("invalid_argument", message, false, fix)
    }

    pub fn invalid_input(message: impl Into<String>, fix: impl Into<String>) -> Self {
        Self::new("invalid_input", message, false, fix)
    }

    pub fn not_found(message: impl Into<String>, fix: impl Into<String>) -> Self {
        Self::new("not_found", message, false, fix)
    }

    pub fn ambiguous_id(prefix: &str, candidates: Vec<String>) -> Self {
        let mut error = Self::new(
            "ambiguous_id",
            format!("ID prefix '{prefix}' matches multiple papercuts"),
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
        match error.kind() {
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
        }
    }

    /// Error mapping for opening an existing papercuts log file. This is the
    /// only place where `NotFound` is mapped to `not_found` / 66.
    pub fn from_log_open(error: std::io::Error, path: &std::path::Path) -> Self {
        if error.kind() == std::io::ErrorKind::NotFound {
            Self::new(
                "not_found",
                format!("papercuts file not found: {}", path.display()),
                false,
                "Run `papercuts add` to create the file or pass an existing --file PATH.",
            )
        } else {
            Self::from_io(error, path)
        }
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}

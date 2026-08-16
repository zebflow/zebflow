use std::fmt::{Display, Formatter};
use std::io;
use std::path::PathBuf;

/// Stable error categories for platform contract boundaries.
#[derive(Debug)]
pub enum ContractError {
    Io { path: PathBuf, source: io::Error },
    Json(serde_json::Error),
    Yaml(serde_yaml_ng::Error),
    Invalid(String),
    Violation { code: &'static str, message: String },
    UnsupportedApiVersion(String),
    UnknownKind(String),
    UnexpectedKind { expected: String, actual: String },
}

impl ContractError {
    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }

    /// Creates a domain contract violation with a stable public error code.
    pub(crate) fn violation(code: &'static str, message: impl Into<String>) -> Self {
        Self::Violation {
            code,
            message: message.into(),
        }
    }

    /// Returns a stable domain error code when the contract defines one.
    pub const fn violation_code(&self) -> Option<&'static str> {
        match self {
            Self::Violation { code, .. } => Some(code),
            _ => None,
        }
    }

    /// Stable error category for platform adapters and public errors.
    pub const fn category(&self) -> &'static str {
        match self {
            Self::Io { .. } => "io",
            Self::Json(_) => "json",
            Self::Yaml(_) => "yaml",
            Self::Invalid(_) => "invalid",
            Self::Violation { .. } => "violation",
            Self::UnsupportedApiVersion(_) => "unsupported_api_version",
            Self::UnknownKind(_) => "unknown_kind",
            Self::UnexpectedKind { .. } => "unexpected_kind",
        }
    }
}

impl Display for ContractError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "contract I/O failed for '{}': {source}", path.display())
            }
            Self::Json(source) => write!(f, "invalid contract JSON: {source}"),
            Self::Yaml(source) => write!(f, "invalid contract YAML: {source}"),
            Self::Invalid(message) => write!(f, "invalid contract: {message}"),
            Self::Violation { code, message } => write!(f, "{code}: {message}"),
            Self::UnsupportedApiVersion(version) => {
                write!(f, "unsupported apiVersion '{version}'")
            }
            Self::UnknownKind(kind) => write!(f, "unknown contract kind '{kind}'"),
            Self::UnexpectedKind { expected, actual } => {
                write!(
                    f,
                    "unexpected contract kind '{actual}'; expected '{expected}'"
                )
            }
        }
    }
}

impl std::error::Error for ContractError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Json(source) => Some(source),
            Self::Yaml(source) => Some(source),
            _ => None,
        }
    }
}

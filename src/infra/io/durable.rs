//! Durable local-file primitives for authoritative Zebflow contracts.
//!
//! Authoritative JSON must not be parsed with permissive fallbacks or written
//! directly over its last valid copy. This module centralizes two rules:
//!
//! 1. Validate a document's identity and version before deserializing it.
//! 2. Replace files through a synchronized same-directory temporary file.
//!
//! Higher-level services still own migrations and business validation. This
//! module owns only durable bytes and root contract-field validation.

use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// One exact root-field value required by a JSON contract.
#[derive(Debug, Clone, Copy)]
pub enum JsonContractValue {
    /// Required string value.
    String(&'static str),
    /// Required unsigned integer value.
    U64(u64),
}

impl Display for JsonContractValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(value) => write!(f, "'{value}'"),
            Self::U64(value) => write!(f, "{value}"),
        }
    }
}

/// One required root field in a versioned JSON document.
#[derive(Debug, Clone, Copy)]
pub struct JsonContractField {
    /// Root object field name.
    pub name: &'static str,
    /// Exact currently supported value.
    pub expected: JsonContractValue,
}

/// Strict identity and version expectations for one JSON contract.
#[derive(Debug, Clone, Copy)]
pub struct JsonContract {
    /// Human-readable contract name used in errors.
    pub name: &'static str,
    /// Root fields that identify the contract and supported version.
    pub fields: &'static [JsonContractField],
}

/// Error returned by strict JSON contract I/O.
#[derive(Debug)]
pub enum DurableJsonError {
    /// Filesystem operation failed.
    Io { path: PathBuf, source: io::Error },
    /// Bytes were not valid JSON.
    Parse {
        contract: &'static str,
        source: serde_json::Error,
    },
    /// JSON did not satisfy the root identity/version contract.
    Contract {
        contract: &'static str,
        message: String,
    },
    /// A typed value could not be serialized.
    Serialize {
        contract: &'static str,
        source: serde_json::Error,
    },
}

impl DurableJsonError {
    /// Stable category suitable for adapter-specific error mapping.
    pub fn category(&self) -> &'static str {
        match self {
            Self::Io { .. } => "io",
            Self::Parse { .. } => "parse",
            Self::Contract { .. } => "contract",
            Self::Serialize { .. } => "serialize",
        }
    }
}

impl Display for DurableJsonError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "failed durable I/O for '{}': {source}", path.display())
            }
            Self::Parse { contract, source } => {
                write!(f, "invalid {contract} JSON: {source}")
            }
            Self::Contract { contract, message } => {
                write!(f, "invalid {contract} contract: {message}")
            }
            Self::Serialize { contract, source } => {
                write!(f, "failed serializing {contract}: {source}")
            }
        }
    }
}

impl std::error::Error for DurableJsonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse { source, .. } | Self::Serialize { source, .. } => Some(source),
            Self::Contract { .. } => None,
        }
    }
}

/// Reads, validates, and deserializes one authoritative JSON document.
pub fn read_versioned_json<T: DeserializeOwned>(
    path: &Path,
    contract: JsonContract,
) -> Result<T, DurableJsonError> {
    let bytes = std::fs::read(path).map_err(|source| DurableJsonError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    parse_versioned_json(&bytes, contract)
}

/// Reads an optional authoritative JSON document.
///
/// Only `NotFound` becomes `None`. Permission failures and all other I/O errors
/// remain visible to the caller.
pub fn read_optional_versioned_json<T: DeserializeOwned>(
    path: &Path,
    contract: JsonContract,
) -> Result<Option<T>, DurableJsonError> {
    match std::fs::read(path) {
        Ok(bytes) => parse_versioned_json(&bytes, contract).map(Some),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(DurableJsonError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// Validates and deserializes versioned JSON bytes.
pub fn parse_versioned_json<T: DeserializeOwned>(
    bytes: &[u8],
    contract: JsonContract,
) -> Result<T, DurableJsonError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|source| DurableJsonError::Parse {
        contract: contract.name,
        source,
    })?;
    validate_contract_value(&value, contract)?;
    serde_json::from_value(value).map_err(|source| DurableJsonError::Parse {
        contract: contract.name,
        source,
    })
}

/// Validates, serializes, and atomically replaces one authoritative JSON file.
pub fn write_atomic_json<T: Serialize>(
    path: &Path,
    value: &T,
    contract: JsonContract,
) -> Result<(), DurableJsonError> {
    let value = serde_json::to_value(value).map_err(|source| DurableJsonError::Serialize {
        contract: contract.name,
        source,
    })?;
    validate_contract_value(&value, contract)?;
    let mut bytes =
        serde_json::to_vec_pretty(&value).map_err(|source| DurableJsonError::Serialize {
            contract: contract.name,
            source,
        })?;
    bytes.push(b'\n');
    atomic_write(path, &bytes).map_err(|source| DurableJsonError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Atomically replaces `path` with `bytes` and synchronizes the durable rename.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    atomic_write_with(path, |file| file.write_all(bytes))
}

fn atomic_write_with<F>(path: &Path, write: F) -> io::Result<()>
where
    F: FnOnce(&mut File) -> io::Result<()>,
{
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;

    let prefix = format!(
        ".{}.zebflow-write-",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("state")
    );
    let mut temporary = tempfile::Builder::new()
        .prefix(&prefix)
        .tempfile_in(parent)?;

    write(temporary.as_file_mut())?;
    temporary.as_file_mut().flush()?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    sync_directory(parent)?;
    Ok(())
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn validate_contract_value(value: &Value, contract: JsonContract) -> Result<(), DurableJsonError> {
    let object = value
        .as_object()
        .ok_or_else(|| DurableJsonError::Contract {
            contract: contract.name,
            message: "root must be a JSON object".to_string(),
        })?;

    for field in contract.fields {
        let actual = object
            .get(field.name)
            .ok_or_else(|| DurableJsonError::Contract {
                contract: contract.name,
                message: format!("missing required root field '{}'", field.name),
            })?;
        let matches = match field.expected {
            JsonContractValue::String(expected) => actual.as_str() == Some(expected),
            JsonContractValue::U64(expected) => actual.as_u64() == Some(expected),
        };
        if !matches {
            return Err(DurableJsonError::Contract {
                contract: contract.name,
                message: format!(
                    "unsupported '{}' value {}; expected {}",
                    field.name, actual, field.expected
                ),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    const TEST_FIELDS: &[JsonContractField] = &[
        JsonContractField {
            name: "kind",
            expected: JsonContractValue::String("example"),
        },
        JsonContractField {
            name: "version",
            expected: JsonContractValue::U64(1),
        },
    ];
    const TEST_CONTRACT: JsonContract = JsonContract {
        name: "test document",
        fields: TEST_FIELDS,
    };

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct TestDocument {
        kind: String,
        version: u64,
        value: String,
    }

    fn document(value: &str) -> TestDocument {
        TestDocument {
            kind: "example".to_string(),
            version: 1,
            value: value.to_string(),
        }
    }

    #[test]
    fn versioned_json_rejects_missing_and_future_contract_fields() {
        let missing = parse_versioned_json::<TestDocument>(
            br#"{"kind":"example","value":"x"}"#,
            TEST_CONTRACT,
        )
        .unwrap_err();
        assert_eq!(missing.category(), "contract");

        let future = parse_versioned_json::<TestDocument>(
            br#"{"kind":"example","version":2,"value":"x"}"#,
            TEST_CONTRACT,
        )
        .unwrap_err();
        assert_eq!(future.category(), "contract");
    }

    #[test]
    fn optional_read_defaults_only_for_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.json");
        assert!(
            read_optional_versioned_json::<TestDocument>(&missing, TEST_CONTRACT)
                .unwrap()
                .is_none()
        );

        let invalid = dir.path().join("invalid.json");
        std::fs::write(&invalid, b"not-json").unwrap();
        assert!(read_optional_versioned_json::<TestDocument>(&invalid, TEST_CONTRACT).is_err());
    }

    #[test]
    fn atomic_json_write_roundtrips_and_replaces() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        write_atomic_json(&path, &document("first"), TEST_CONTRACT).unwrap();
        write_atomic_json(&path, &document("second"), TEST_CONTRACT).unwrap();

        let loaded: TestDocument = read_versioned_json(&path, TEST_CONTRACT).unwrap();
        assert_eq!(loaded, document("second"));
    }

    #[test]
    fn failed_write_keeps_previous_file_and_removes_temporary_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        atomic_write(&path, b"previous").unwrap();

        let result = atomic_write_with(&path, |file| {
            file.write_all(b"partial")?;
            Err(io::Error::other("injected write failure"))
        });
        assert!(result.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"previous");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn invalid_value_is_rejected_before_replacing_previous_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        write_atomic_json(&path, &document("valid"), TEST_CONTRACT).unwrap();

        let invalid = TestDocument {
            kind: "example".to_string(),
            version: 2,
            value: "invalid".to_string(),
        };
        assert!(write_atomic_json(&path, &invalid, TEST_CONTRACT).is_err());
        let loaded: TestDocument = read_versioned_json(&path, TEST_CONTRACT).unwrap();
        assert_eq!(loaded, document("valid"));
    }
}

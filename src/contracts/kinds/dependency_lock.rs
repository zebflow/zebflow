//! Canonical dependency lock contract.
//!
//! This module is the only owner of the persisted `repo/zeb.lock` shape.
//! Platform services use these types directly so an internal runtime model
//! cannot silently drift from the durable transfer contract.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::contracts::{
    ContractDocument, ContractError, ContractKind, ContractMetadata, PlatformContract,
    decode_contract, encode_contract,
};

/// Canonical dependency lock filename inside a project repository.
pub const DEPENDENCY_LOCK_FILE: &str = "zeb.lock";
/// Recovery copy stem retained after converting a pre-v1 lock, combined with
/// the migration date (`project-directory.md` §5) into
/// `data/recovery/zeb-lock-{date}.lock`.
pub const DEPENDENCY_LOCK_BACKUP_FILE: &str = "zeb-lock";
/// Maximum accepted size of one dependency lock document.
pub const MAX_DEPENDENCY_LOCK_BYTES: usize = 4 * 1024 * 1024;
/// Maximum RWE libraries in one project.
pub const MAX_DEPENDENCY_LOCK_LIBRARIES: usize = 1024;
/// Maximum installed node bundles in one project.
pub const MAX_DEPENDENCY_LOCK_NODE_BUNDLES: usize = 1024;
/// Maximum node definitions supplied by one bundle.
pub const MAX_DEPENDENCY_LOCK_BUNDLE_DEFINITIONS: usize = 256;

/// Canonical `zeb.lock` contract.
pub struct DependencyLockContract;

impl PlatformContract for DependencyLockContract {
    type Spec = DependencyLockSpec;
    const KIND: ContractKind = ContractKind::DependencyLock;

    fn validate(metadata: &ContractMetadata, spec: &Self::Spec) -> Result<(), ContractError> {
        validate_logical_name("metadata.name", &metadata.name, false)?;
        if metadata.version.is_some()
            || metadata.digest.is_some()
            || !metadata.annotations.is_empty()
        {
            return Err(ContractError::violation(
                "ZF_DEPENDENCY_LOCK_METADATA",
                "DependencyLock metadata contains only name",
            ));
        }
        spec.validate()
    }
}

/// Exact external artifacts resolved for one project.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DependencyLockSpec {
    #[serde(default)]
    pub rwe: DependencyLockRweSpec,
    #[serde(default)]
    pub nodes: DependencyLockNodesSpec,
}

/// Browser-runtime dependencies.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DependencyLockRweSpec {
    #[serde(default)]
    pub libraries: BTreeMap<String, DependencyLockArtifactSpec>,
}

/// Installed non-native node dependencies.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DependencyLockNodesSpec {
    #[serde(default)]
    pub bundles: BTreeMap<String, DependencyLockNodeBundleSpec>,
}

/// One exact RWE artifact resolution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DependencyLockArtifactSpec {
    pub version: String,
    pub source: DependencyLockSource,
    pub source_id: String,
    pub entry: String,
    pub integrity: String,
}

/// One exact node-bundle resolution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DependencyLockNodeBundleSpec {
    pub version: String,
    pub source: DependencyLockSource,
    pub source_id: String,
    pub entry: String,
    pub integrity: String,
    pub definitions: Vec<String>,
}

/// Durable provenance of a resolved dependency artifact.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependencyLockSource {
    Embedded,
    Hub,
    Project,
}

impl DependencyLockSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Embedded => "embedded",
            Self::Hub => "hub",
            Self::Project => "project",
        }
    }
}

impl DependencyLockSpec {
    fn validate(&self) -> Result<(), ContractError> {
        if self.rwe.libraries.len() > MAX_DEPENDENCY_LOCK_LIBRARIES {
            return Err(ContractError::violation(
                "ZF_DEPENDENCY_LOCK_LIMIT",
                format!(
                    "spec.rwe.libraries exceeds the {MAX_DEPENDENCY_LOCK_LIBRARIES} library limit"
                ),
            ));
        }
        if self.nodes.bundles.len() > MAX_DEPENDENCY_LOCK_NODE_BUNDLES {
            return Err(ContractError::violation(
                "ZF_DEPENDENCY_LOCK_LIMIT",
                format!(
                    "spec.nodes.bundles exceeds the {MAX_DEPENDENCY_LOCK_NODE_BUNDLES} bundle limit"
                ),
            ));
        }

        for (name, entry) in &self.rwe.libraries {
            let path = format!("spec.rwe.libraries[{name:?}]");
            validate_namespaced_name(&path, name)?;
            // Widened deliberately when the local hub gained rwe_library
            // packages: an installed library carries hub provenance and a real
            // digest. `project` stays refused — no channel produces it for a
            // library in zebflow.com/v1.
            if entry.source == DependencyLockSource::Project {
                return Err(ContractError::violation(
                    "ZF_DEPENDENCY_LOCK_SOURCE",
                    format!("{path}.source must be embedded or hub in zebflow.com/v1"),
                ));
            }
            validate_artifact_fields(
                &path,
                &entry.version,
                entry.source,
                &entry.source_id,
                &entry.entry,
                &entry.integrity,
            )?;
        }

        let mut providers = BTreeMap::<&str, &str>::new();
        for (name, entry) in &self.nodes.bundles {
            let path = format!("spec.nodes.bundles[{name:?}]");
            validate_namespaced_name(&path, name)?;
            if entry.source == DependencyLockSource::Embedded {
                return Err(ContractError::violation(
                    "ZF_DEPENDENCY_LOCK_SOURCE",
                    format!("{path}.source must be hub or project in zebflow.com/v1"),
                ));
            }
            validate_artifact_fields(
                &path,
                &entry.version,
                entry.source,
                &entry.source_id,
                &entry.entry,
                &entry.integrity,
            )?;
            if entry.definitions.is_empty()
                || entry.definitions.len() > MAX_DEPENDENCY_LOCK_BUNDLE_DEFINITIONS
            {
                return Err(ContractError::violation(
                    "ZF_DEPENDENCY_LOCK_DEFINITIONS",
                    format!(
                        "{path}.definitions must contain 1 to {MAX_DEPENDENCY_LOCK_BUNDLE_DEFINITIONS} node kinds"
                    ),
                ));
            }
            let unique = entry.definitions.iter().collect::<BTreeSet<_>>();
            if unique.len() != entry.definitions.len()
                || !entry.definitions.windows(2).all(|pair| pair[0] < pair[1])
            {
                return Err(ContractError::violation(
                    "ZF_DEPENDENCY_LOCK_DEFINITIONS",
                    format!("{path}.definitions must be sorted and unique"),
                ));
            }
            for definition in &entry.definitions {
                validate_node_kind(&format!("{path}.definitions"), definition)?;
                if let Some(previous) = providers.insert(definition, name) {
                    return Err(ContractError::violation(
                        "ZF_DEPENDENCY_LOCK_PROVIDER_COLLISION",
                        format!(
                            "node kind '{definition}' is provided by both '{previous}' and '{name}'"
                        ),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Decodes the canonical lock with a fixed document-size bound.
pub fn decode_dependency_lock(
    bytes: &[u8],
) -> Result<ContractDocument<DependencyLockSpec>, ContractError> {
    if bytes.len() > MAX_DEPENDENCY_LOCK_BYTES {
        return Err(ContractError::violation(
            "ZF_DEPENDENCY_LOCK_LIMIT",
            format!("DependencyLock exceeds the {MAX_DEPENDENCY_LOCK_BYTES} byte limit"),
        ));
    }
    decode_contract::<DependencyLockContract>(bytes)
}

/// Validates and encodes one canonical lock document.
pub fn encode_dependency_lock(
    metadata: ContractMetadata,
    spec: DependencyLockSpec,
) -> Result<Vec<u8>, ContractError> {
    let bytes = encode_contract::<DependencyLockContract>(metadata, spec)?;
    if bytes.len() > MAX_DEPENDENCY_LOCK_BYTES {
        return Err(ContractError::violation(
            "ZF_DEPENDENCY_LOCK_LIMIT",
            format!("DependencyLock exceeds the {MAX_DEPENDENCY_LOCK_BYTES} byte limit"),
        ));
    }
    Ok(bytes)
}

fn validate_artifact_fields(
    path: &str,
    version: &str,
    _source: DependencyLockSource,
    source_id: &str,
    entry: &str,
    integrity: &str,
) -> Result<(), ContractError> {
    validate_text(&format!("{path}.version"), version, 128)?;
    validate_logical_name(&format!("{path}.source_id"), source_id, true)?;
    validate_relative_path(&format!("{path}.entry"), entry)?;
    validate_sha256(&format!("{path}.integrity"), integrity)
}

fn validate_namespaced_name(path: &str, value: &str) -> Result<(), ContractError> {
    validate_logical_name(path, value, true)?;
    if !value.contains('/') {
        return Err(ContractError::violation(
            "ZF_DEPENDENCY_LOCK_NAME",
            format!("{path} must include its namespace"),
        ));
    }
    Ok(())
}

/// Validates one locked node kind as a well-formed logical name.
///
/// The lock deliberately does not police the node namespace. Which prefixes an
/// installed node may use is a `NodeBundle` rule, and keeping it there means the
/// namespace is enforced in exactly one place.
fn validate_node_kind(path: &str, value: &str) -> Result<(), ContractError> {
    validate_logical_name(path, value, false)
}

fn validate_logical_name(path: &str, value: &str, allow_slash: bool) -> Result<(), ContractError> {
    let valid = !value.is_empty()
        && value.len() <= 256
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'-' | b'_' | b'.')
                || (allow_slash && byte == b'/')
        })
        && !value
            .split('/')
            .any(|segment| segment.is_empty() || matches!(segment, "." | ".."));
    if !valid {
        return Err(ContractError::violation(
            "ZF_DEPENDENCY_LOCK_NAME",
            format!(
                "{path} must be a lowercase logical name using letters, numbers, dot, underscore, hyphen, and optional namespace separators"
            ),
        ));
    }
    Ok(())
}

fn validate_text(path: &str, value: &str, maximum: usize) -> Result<(), ContractError> {
    if value.trim().is_empty() || value.len() > maximum || value.chars().any(char::is_control) {
        return Err(ContractError::violation(
            "ZF_DEPENDENCY_LOCK_VALUE",
            format!(
                "{path} must be non-empty, at most {maximum} bytes, and contain no control characters"
            ),
        ));
    }
    Ok(())
}

fn validate_relative_path(path: &str, value: &str) -> Result<(), ContractError> {
    validate_text(path, value, 2048)?;
    if value.starts_with('/')
        || value.contains('\\')
        || value
            .split('/')
            .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
    {
        return Err(ContractError::violation(
            "ZF_DEPENDENCY_LOCK_ENTRY",
            format!("{path} must be a normalized relative path"),
        ));
    }
    Ok(())
}

fn validate_sha256(path: &str, value: &str) -> Result<(), ContractError> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(ContractError::violation(
            "ZF_DEPENDENCY_LOCK_INTEGRITY",
            format!("{path} must use sha256:<hex>"),
        ));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ContractError::violation(
            "ZF_DEPENDENCY_LOCK_INTEGRITY",
            format!("{path} must contain exactly 64 lowercase hexadecimal digits"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> &'static [u8] {
        include_bytes!("../../../tests/fixtures/contracts/dependency-lock/v1-complete.json")
    }

    #[test]
    fn golden_v1_fixture_roundtrips_deterministically() {
        let document = decode_dependency_lock(fixture()).expect("valid lock fixture");
        let encoded = encode_dependency_lock(document.metadata, document.spec).unwrap();
        assert_eq!(encoded, fixture());
    }

    #[test]
    fn rejects_unknown_fields_unsafe_entries_and_bad_digests() {
        let mut value: serde_json::Value = serde_json::from_slice(fixture()).unwrap();
        value["spec"]["extra"] = serde_json::json!(true);
        assert!(decode_dependency_lock(&serde_json::to_vec(&value).unwrap()).is_err());

        let mut value: serde_json::Value = serde_json::from_slice(fixture()).unwrap();
        value["spec"]["rwe"]["libraries"]["zeb/deckgl"]["entry"] = serde_json::json!("../secret");
        assert!(decode_dependency_lock(&serde_json::to_vec(&value).unwrap()).is_err());

        let mut value: serde_json::Value = serde_json::from_slice(fixture()).unwrap();
        value["spec"]["nodes"]["bundles"]["zebflow/sim-des"]["integrity"] =
            serde_json::json!("sha256:not-a-digest");
        assert!(decode_dependency_lock(&serde_json::to_vec(&value).unwrap()).is_err());
    }

    #[test]
    fn rejects_future_versions_unsorted_definitions_and_duplicate_providers() {
        let mut value: serde_json::Value = serde_json::from_slice(fixture()).unwrap();
        value["apiVersion"] = serde_json::json!("zebflow.com/v2");
        assert!(decode_dependency_lock(&serde_json::to_vec(&value).unwrap()).is_err());

        let mut value: serde_json::Value = serde_json::from_slice(fixture()).unwrap();
        value["spec"]["nodes"]["bundles"]["zebflow/sim-des"]["definitions"] =
            serde_json::json!(["n.wasm.z", "n.wasm.a"]);
        assert!(decode_dependency_lock(&serde_json::to_vec(&value).unwrap()).is_err());

        let mut value: serde_json::Value = serde_json::from_slice(fixture()).unwrap();
        let mut duplicate = value["spec"]["nodes"]["bundles"]["zebflow/sim-des"].clone();
        duplicate["source_id"] = serde_json::json!("another/provider");
        value["spec"]["nodes"]["bundles"]["zebflow/duplicate"] = duplicate;
        assert!(decode_dependency_lock(&serde_json::to_vec(&value).unwrap()).is_err());
    }

    #[test]
    fn rejects_dependency_sources_without_v1_resolvers() {
        // `hub` became a v1 resolver for libraries when the local hub gained
        // rwe_library packages; `project` remains the source no channel
        // produces for a library.
        let mut value: serde_json::Value = serde_json::from_slice(fixture()).unwrap();
        value["spec"]["rwe"]["libraries"]["zeb/deckgl"]["source"] = serde_json::json!("hub");
        assert!(decode_dependency_lock(&serde_json::to_vec(&value).unwrap()).is_ok());

        let mut value: serde_json::Value = serde_json::from_slice(fixture()).unwrap();
        value["spec"]["rwe"]["libraries"]["zeb/deckgl"]["source"] = serde_json::json!("project");
        assert!(decode_dependency_lock(&serde_json::to_vec(&value).unwrap()).is_err());

        let mut value: serde_json::Value = serde_json::from_slice(fixture()).unwrap();
        value["spec"]["nodes"]["bundles"]["zebflow/sim-des"]["source"] =
            serde_json::json!("embedded");
        assert!(decode_dependency_lock(&serde_json::to_vec(&value).unwrap()).is_err());
    }

    #[test]
    fn writer_orders_dependency_keys_lexically() {
        let mut spec = DependencyLockSpec::default();
        for name in ["zeb/z-last", "zeb/a-first"] {
            spec.rwe.libraries.insert(
                name.to_string(),
                DependencyLockArtifactSpec {
                    version: "1".to_string(),
                    source: DependencyLockSource::Embedded,
                    source_id: format!("zebflow/{name}"),
                    entry: "dist/main.mjs".to_string(),
                    integrity: format!("sha256:{}", "a".repeat(64)),
                },
            );
        }
        let text = String::from_utf8(
            encode_dependency_lock(ContractMetadata::named("project"), spec).unwrap(),
        )
        .unwrap();
        assert!(text.find("zeb/a-first").unwrap() < text.find("zeb/z-last").unwrap());
    }
}

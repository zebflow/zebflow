//! Canonical project export/import archive envelope.
//!
//! This module is the only owner of the `manifest.json` shape at the root of
//! every whole-project movement archive — transfer archive, hub
//! `project_bundle` asset, platform-scope install
//! (`docs/contracts/kinds/project-bundle/README.md`). The manifest declares
//! which classes the archive carries; the platform transfer service verifies
//! each declared class as a unit before any swap.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::contracts::kinds::DependencyLockSource;
use crate::contracts::{ContractError, ContractKind, ContractMetadata, PlatformContract};

/// Canonical manifest filename at the archive root.
pub const PROJECT_BUNDLE_MANIFEST_FILE: &str = "manifest.json";
/// Archive directory holding the bytes of `direct.*` lock entries, which
/// cannot be fetched again from their recorded source
/// (`distribution.md` §5 reproducibility table).
pub const PROJECT_BUNDLE_CARRIED_DEPENDENCIES_DIR: &str = "carried-dependencies";

/// Canonical archive-root envelope for every whole-project movement.
pub struct ProjectBundleContract;

impl PlatformContract for ProjectBundleContract {
    type Spec = ProjectBundleSpec;
    const KIND: ContractKind = ContractKind::ProjectBundle;

    fn validate(metadata: &ContractMetadata, spec: &Self::Spec) -> Result<(), ContractError> {
        // `metadata.name` is `owner/project` provenance — recorded and shown,
        // never authority: no import gates on it.
        validate_provenance_name(&metadata.name)?;
        if metadata.version.is_some()
            || metadata.digest.is_some()
            || !metadata.annotations.is_empty()
        {
            return Err(ContractError::violation(
                "ZF_PROJECT_BUNDLE_METADATA",
                "ProjectBundle metadata contains only name",
            ));
        }
        spec.validate()
    }
}

/// One movable class of project content.
///
/// | Class | Content |
/// | --- | --- |
/// | `repo` | `repo/` — source, `zebflow.yaml`, `zeb.lock`, schema/initial-data files |
/// | `store` | `data/store/` — applied database state |
/// | `files` | `files/` — objects + `.zebfs/acl.json` |
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ProjectBundleClass {
    Repo,
    Store,
    Files,
}

impl ProjectBundleClass {
    /// Stable serialized name; also the class directory at the archive root.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Repo => "repo",
            Self::Store => "store",
            Self::Files => "files",
        }
    }

    /// Parses a canonical class name. Aliases are intentionally unsupported.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "repo" => Some(Self::Repo),
            "store" => Some(Self::Store),
            "files" => Some(Self::Files),
            _ => None,
        }
    }
}

impl std::fmt::Display for ProjectBundleClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Declared content of one project movement archive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectBundleSpec {
    /// Carried classes, sorted `repo`, `store`, `files`, without duplicates.
    pub classes: Vec<ProjectBundleClass>,
    /// Unix timestamp seconds when the archive was produced.
    pub exported_at: i64,
    /// Office that produced the archive, when one is identified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_office_id: Option<String>,
    /// One tree digest per carried class — the class swaps as a unit, so it
    /// verifies as a unit, at staging before any swap.
    pub class_digests: BTreeMap<ProjectBundleClass, String>,
    /// Every `direct.*` lock entry travelling with its bytes; `hub.*` entries
    /// regenerate from the lock and are never listed here.
    pub carried_dependencies: Vec<ProjectBundleCarriedDependency>,
    /// Per-class file counts plus the total staged payload bytes.
    pub counts: ProjectBundleCounts,
}

/// One `direct.*` dependency whose bytes travel inside the archive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectBundleCarriedDependency {
    /// The lock entry's logical name (`zeb/my-chart`, `publisher.package`).
    pub name: String,
    /// The lock entry's provenance; only `direct.*` bytes are carried.
    pub source: DependencyLockSource,
    /// The lock entry's integrity digest, verified against the carried bytes.
    pub integrity: String,
}

/// File counts per carried class plus total staged payload bytes.
///
/// A count is present exactly for the carried classes: a count without a
/// class, or a carried class without a count, is refused.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectBundleCounts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<u64>,
    /// Total exported payload bytes before archive packaging.
    pub total_bytes: u64,
}

impl ProjectBundleCounts {
    /// The recorded count for one class, when one is recorded.
    pub fn class_count(&self, class: ProjectBundleClass) -> Option<u64> {
        match class {
            ProjectBundleClass::Repo => self.repo,
            ProjectBundleClass::Store => self.store,
            ProjectBundleClass::Files => self.files,
        }
    }
}

impl ProjectBundleSpec {
    /// True when the archive declares this class.
    pub fn carries(&self, class: ProjectBundleClass) -> bool {
        self.classes.contains(&class)
    }

    fn validate(&self) -> Result<(), ContractError> {
        if self.classes.is_empty() {
            return Err(ContractError::violation(
                "ZF_PROJECT_BUNDLE_CLASSES",
                "spec.classes must declare at least one carried class",
            ));
        }
        if !self.classes.is_sorted() || self.classes.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ContractError::violation(
                "ZF_PROJECT_BUNDLE_CLASSES",
                "spec.classes must be sorted repo, store, files without duplicates",
            ));
        }
        // State without the source that explains it is not a project.
        if self.carries(ProjectBundleClass::Store) && !self.carries(ProjectBundleClass::Repo) {
            return Err(ContractError::violation(
                "ZF_PROJECT_BUNDLE_STORE_WITHOUT_REPO",
                "spec.classes carrying store without repo is refused",
            ));
        }
        if self.exported_at <= 0 {
            return Err(ContractError::violation(
                "ZF_PROJECT_BUNDLE_EXPORTED_AT",
                "spec.exported_at must be a positive unix timestamp",
            ));
        }
        if let Some(office) = &self.source_office_id
            && (office.trim().is_empty() || office.chars().any(char::is_control))
        {
            return Err(ContractError::violation(
                "ZF_PROJECT_BUNDLE_SOURCE_OFFICE",
                "spec.source_office_id must be non-empty and contain no control characters",
            ));
        }
        for class in [
            ProjectBundleClass::Repo,
            ProjectBundleClass::Store,
            ProjectBundleClass::Files,
        ] {
            match (self.carries(class), self.class_digests.get(&class)) {
                (true, Some(digest)) => {
                    validate_sha256_digest(&format!("spec.class_digests.{class}"), digest)?;
                }
                (true, None) => {
                    return Err(ContractError::violation(
                        "ZF_PROJECT_BUNDLE_DIGESTS",
                        format!("carried class '{class}' has no spec.class_digests entry"),
                    ));
                }
                (false, Some(_)) => {
                    return Err(ContractError::violation(
                        "ZF_PROJECT_BUNDLE_DIGESTS",
                        format!(
                            "spec.class_digests names '{class}', which spec.classes does not carry"
                        ),
                    ));
                }
                (false, None) => {}
            }
            match (self.carries(class), self.counts.class_count(class)) {
                (true, None) => {
                    return Err(ContractError::violation(
                        "ZF_PROJECT_BUNDLE_COUNTS",
                        format!("carried class '{class}' has no spec.counts entry"),
                    ));
                }
                (false, Some(_)) => {
                    return Err(ContractError::violation(
                        "ZF_PROJECT_BUNDLE_COUNTS",
                        format!("spec.counts names '{class}', which spec.classes does not carry"),
                    ));
                }
                _ => {}
            }
        }
        let mut previous: Option<&str> = None;
        for entry in &self.carried_dependencies {
            let name = entry.name.as_str();
            if name.trim().is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
                return Err(ContractError::violation(
                    "ZF_PROJECT_BUNDLE_CARRIED",
                    "spec.carried_dependencies entry names must be non-empty, at most 256 \
                     bytes, and contain no control characters",
                ));
            }
            if previous.is_some_and(|prior| prior >= name) {
                return Err(ContractError::violation(
                    "ZF_PROJECT_BUNDLE_CARRIED",
                    "spec.carried_dependencies must be sorted by name without duplicates",
                ));
            }
            previous = Some(name);
            // `hub.*` and `project` entries regenerate from the lock; only a
            // dependency that cannot be fetched again travels with bytes.
            if !matches!(
                entry.source,
                DependencyLockSource::DirectFile | DependencyLockSource::DirectNpm
            ) {
                return Err(ContractError::violation(
                    "ZF_PROJECT_BUNDLE_CARRIED",
                    format!(
                        "spec.carried_dependencies[{name:?}].source must be direct.file or \
                         direct.npm; '{}' regenerates from the lock",
                        entry.source.as_str()
                    ),
                ));
            }
            validate_sha256_digest(
                &format!("spec.carried_dependencies[{name:?}].integrity"),
                &entry.integrity,
            )?;
        }
        Ok(())
    }
}

/// `owner/project` — exactly one separator, both segments non-empty.
fn validate_provenance_name(name: &str) -> Result<(), ContractError> {
    let mut segments = name.split('/');
    let owner = segments.next().unwrap_or_default();
    let project = segments.next().unwrap_or_default();
    if owner.trim().is_empty() || project.trim().is_empty() || segments.next().is_some() {
        return Err(ContractError::violation(
            "ZF_PROJECT_BUNDLE_NAME",
            format!("metadata.name '{name}' must be 'owner/project' provenance"),
        ));
    }
    Ok(())
}

/// `sha256:` + exactly 64 lowercase hexadecimal characters.
fn validate_sha256_digest(path: &str, digest: &str) -> Result<(), ContractError> {
    let hex = digest.strip_prefix("sha256:").ok_or_else(|| {
        ContractError::violation(
            "ZF_PROJECT_BUNDLE_DIGEST",
            format!("{path} must use the sha256:<hex> form"),
        )
    })?;
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ContractError::violation(
            "ZF_PROJECT_BUNDLE_DIGEST",
            format!("{path} must carry exactly 64 lowercase hexadecimal digest characters"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;
    use crate::contracts::{decode_contract, decode_contract_value, encode_contract};

    const COMPLETE_V1: &[u8] =
        include_bytes!("../../../tests/fixtures/contracts/project-bundle/v1-complete.json");

    fn valid_value() -> Value {
        serde_json::from_slice(COMPLETE_V1).expect("the fixture is JSON")
    }

    #[test]
    fn complete_v1_fixture_is_canonical_and_roundtrips_byte_for_byte() {
        let document = decode_contract::<ProjectBundleContract>(COMPLETE_V1).unwrap();
        assert_eq!(document.kind, "ProjectBundle");
        assert_eq!(document.metadata.name, "superadmin/spatial-blog");
        assert_eq!(
            document.spec.classes,
            vec![
                ProjectBundleClass::Repo,
                ProjectBundleClass::Store,
                ProjectBundleClass::Files
            ]
        );
        assert_eq!(document.spec.exported_at, 1_787_175_600);
        assert_eq!(document.spec.source_office_id.as_deref(), Some("office-a"));
        assert_eq!(document.spec.class_digests.len(), 3);
        assert_eq!(document.spec.carried_dependencies.len(), 1);
        assert_eq!(document.spec.carried_dependencies[0].name, "zeb/my-chart");
        assert_eq!(
            document.spec.carried_dependencies[0].source,
            DependencyLockSource::DirectFile
        );
        assert_eq!(document.spec.counts.repo, Some(41));
        assert_eq!(document.spec.counts.store, Some(7));
        assert_eq!(document.spec.counts.files, Some(128));
        assert_eq!(document.spec.counts.total_bytes, 5_242_880);
        assert_eq!(
            encode_contract::<ProjectBundleContract>(document.metadata, document.spec).unwrap(),
            COMPLETE_V1
        );
    }

    #[test]
    fn malformed_bytes_and_unknown_fields_are_refused() {
        assert!(decode_contract::<ProjectBundleContract>(b"{not json").is_err());

        let mut unknown_root = valid_value();
        unknown_root["status"] = json!({});
        assert!(decode_contract_value::<ProjectBundleContract>(unknown_root).is_err());

        let mut unknown_spec = valid_value();
        unknown_spec["spec"]["artifact_kind"] = json!("bundle");
        assert!(decode_contract_value::<ProjectBundleContract>(unknown_spec).is_err());

        let mut unknown_counts = valid_value();
        unknown_counts["spec"]["counts"]["data"] = json!(3);
        assert!(decode_contract_value::<ProjectBundleContract>(unknown_counts).is_err());

        let mut unknown_carried = valid_value();
        unknown_carried["spec"]["carried_dependencies"][0]["entry"] = json!("x/y.mjs");
        assert!(decode_contract_value::<ProjectBundleContract>(unknown_carried).is_err());
    }

    #[test]
    fn a_wrong_kind_and_a_future_api_version_are_refused() {
        let mut wrong_kind = valid_value();
        wrong_kind["kind"] = json!("Pipeline");
        assert_eq!(
            decode_contract_value::<ProjectBundleContract>(wrong_kind)
                .unwrap_err()
                .category(),
            "unexpected_kind"
        );

        let mut future = valid_value();
        future["apiVersion"] = json!("zebflow.com/v2");
        assert_eq!(
            decode_contract_value::<ProjectBundleContract>(future)
                .unwrap_err()
                .category(),
            "unsupported_api_version"
        );
    }

    #[test]
    fn empty_classes_and_store_without_repo_are_refused() {
        let mut empty = valid_value();
        empty["spec"]["classes"] = json!([]);
        empty["spec"]["class_digests"] = json!({});
        empty["spec"]["counts"] = json!({ "total_bytes": 0 });
        assert!(decode_contract_value::<ProjectBundleContract>(empty).is_err());

        let mut store_only = valid_value();
        store_only["spec"]["classes"] = json!(["store"]);
        store_only["spec"]["class_digests"] = json!({
            "store": valid_value()["spec"]["class_digests"]["store"].clone()
        });
        store_only["spec"]["counts"] = json!({ "store": 7, "total_bytes": 100 });
        assert!(decode_contract_value::<ProjectBundleContract>(store_only).is_err());

        let mut unsorted = valid_value();
        unsorted["spec"]["classes"] = json!(["store", "repo", "files"]);
        assert!(decode_contract_value::<ProjectBundleContract>(unsorted).is_err());

        let mut duplicated = valid_value();
        duplicated["spec"]["classes"] = json!(["repo", "repo", "store", "files"]);
        assert!(decode_contract_value::<ProjectBundleContract>(duplicated).is_err());
    }

    #[test]
    fn class_digests_must_cover_exactly_the_carried_classes() {
        // A carried class with no digest is refused.
        let mut missing = valid_value();
        missing["spec"]["class_digests"]
            .as_object_mut()
            .unwrap()
            .remove("store");
        assert!(decode_contract_value::<ProjectBundleContract>(missing).is_err());

        // A digest for a class the archive does not carry is refused.
        let mut extra = valid_value();
        extra["spec"]["classes"] = json!(["repo", "files"]);
        extra["spec"]["counts"] = json!({ "repo": 41, "files": 128, "total_bytes": 5242880 });
        assert!(decode_contract_value::<ProjectBundleContract>(extra).is_err());

        for refused in [
            json!(""),
            json!("aa11"),
            json!("beef".repeat(16)),
            json!(format!("sha256:{}", "G".repeat(64))),
            json!(format!("md5:{}", "a".repeat(64))),
        ] {
            let mut bad = valid_value();
            bad["spec"]["class_digests"]["repo"] = refused.clone();
            assert!(
                decode_contract_value::<ProjectBundleContract>(bad).is_err(),
                "accepted digest {refused}"
            );
        }
    }

    #[test]
    fn counts_must_cover_exactly_the_carried_classes() {
        let mut missing = valid_value();
        missing["spec"]["counts"]
            .as_object_mut()
            .unwrap()
            .remove("files");
        assert!(decode_contract_value::<ProjectBundleContract>(missing).is_err());

        let mut extra = valid_value();
        extra["spec"]["classes"] = json!(["repo"]);
        extra["spec"]["class_digests"] = json!({
            "repo": valid_value()["spec"]["class_digests"]["repo"].clone()
        });
        assert!(decode_contract_value::<ProjectBundleContract>(extra).is_err());
    }

    #[test]
    fn carried_dependencies_accept_only_direct_sources_with_real_digests() {
        for refused in [
            "hub.local",
            "hub.public",
            "hub.static",
            "project",
            "embedded",
        ] {
            let mut bad = valid_value();
            bad["spec"]["carried_dependencies"][0]["source"] = json!(refused);
            assert!(
                decode_contract_value::<ProjectBundleContract>(bad).is_err(),
                "accepted carried source '{refused}'"
            );
        }
        let mut npm = valid_value();
        npm["spec"]["carried_dependencies"][0]["source"] = json!("direct.npm");
        assert!(decode_contract_value::<ProjectBundleContract>(npm).is_ok());

        let mut bad_digest = valid_value();
        bad_digest["spec"]["carried_dependencies"][0]["integrity"] = json!("sha256:beef");
        assert!(decode_contract_value::<ProjectBundleContract>(bad_digest).is_err());

        // Carrying nothing is a valid state: every lock entry regenerates.
        let mut none = valid_value();
        none["spec"]["carried_dependencies"] = json!([]);
        assert!(decode_contract_value::<ProjectBundleContract>(none).is_ok());
    }

    #[test]
    fn metadata_name_must_be_owner_project_provenance() {
        for refused in ["spatial-blog", "a/b/c", "/spatial-blog", "superadmin/"] {
            let mut bad = valid_value();
            bad["metadata"]["name"] = json!(refused);
            assert!(
                decode_contract_value::<ProjectBundleContract>(bad).is_err(),
                "accepted metadata.name '{refused}'"
            );
        }
    }

    #[test]
    fn exported_at_must_be_positive() {
        for refused in [json!(0), json!(-5)] {
            let mut bad = valid_value();
            bad["spec"]["exported_at"] = refused;
            assert!(decode_contract_value::<ProjectBundleContract>(bad).is_err());
        }
    }
}

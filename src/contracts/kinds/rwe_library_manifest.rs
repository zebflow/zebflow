use std::path::{Component, Path};

use crate::contracts::{ContractError, ContractKind, ContractMetadata, PlatformContract};
use crate::platform::services::library::RweLibraryManifestSpec;

/// Canonical manifest for one RWE library: an opaque pre-built runtime bundle
/// plus typed wrappers, loaded at runtime and never compiled by the RWE
/// compiler. If the compiler compiles it, it is source, not a library.
pub struct RweLibraryManifestContract;

impl PlatformContract for RweLibraryManifestContract {
    type Spec = RweLibraryManifestSpec;
    const KIND: ContractKind = ContractKind::RweLibraryManifest;

    fn validate(metadata: &ContractMetadata, spec: &Self::Spec) -> Result<(), ContractError> {
        if metadata.name != spec.name {
            return Err(ContractError::invalid(format!(
                "metadata.name '{}' must match library name '{}'",
                metadata.name, spec.name
            )));
        }
        if spec.name.trim().is_empty() {
            return Err(ContractError::invalid("library name must not be empty"));
        }
        if spec.exports.is_empty()
            || spec
                .exports
                .iter()
                .any(|symbol| symbol.trim().is_empty() || symbol.chars().any(char::is_control))
        {
            return Err(ContractError::invalid(format!(
                "library '{}' must export at least one symbol, and every export must be a \
                 non-empty symbol name",
                spec.name
            )));
        }
        if spec.versions.is_empty() {
            return Err(ContractError::invalid(format!(
                "library '{}' must define at least one version",
                spec.name
            )));
        }
        for (key, version) in &spec.versions {
            if key.trim().is_empty() {
                return Err(ContractError::invalid(format!(
                    "library '{}' has an empty version key",
                    spec.name
                )));
            }
            validate_descending_entry(&spec.name, key, &version.entry)?;
            // `online` — a bare download URL with no digest and no review — is
            // not a source. "Host it yourself" is the static repository
            // channel, which is digest-pinned and reviewed.
            if version.source != "offline" && version.source != "hub" {
                return Err(ContractError::invalid(format!(
                    "library '{}' version '{key}' has unsupported source '{}': \
                     a library entry loads from 'offline' or 'hub'",
                    spec.name, version.source
                )));
            }
            validate_integrity(&spec.name, key, &version.integrity)?;
            if version.size_bytes == 0 {
                return Err(ContractError::invalid(format!(
                    "library '{}' version '{key}' must declare the non-zero decoded size of \
                     its entry bundle",
                    spec.name
                )));
            }
        }
        Ok(())
    }
}

/// The entry path is joined onto the library's own directory, so it may only
/// ever descend. `..`, an absolute path, and a backslash are each a way to make
/// the entry address bytes the library directory does not contain.
fn validate_descending_entry(name: &str, key: &str, entry: &str) -> Result<(), ContractError> {
    let entry_path = Path::new(entry);
    let ok = !entry.is_empty()
        && !entry.contains('\\')
        && !entry.contains("//")
        && !entry.chars().any(char::is_control)
        && !entry_path.is_absolute()
        && entry_path.components().all(|component| {
            !matches!(
                component,
                Component::ParentDir
                    | Component::CurDir
                    | Component::RootDir
                    | Component::Prefix(_)
            )
        });
    if !ok {
        return Err(ContractError::invalid(format!(
            "library '{name}' version '{key}' entry '{entry}' must be a normalized relative \
             file path that only descends"
        )));
    }
    Ok(())
}

/// `sha256:` + 64 lowercase hex over the entry bundle. Required — empty
/// refused. Filled by the publisher or seeder, verified at install and by the
/// dependency report.
fn validate_integrity(name: &str, key: &str, integrity: &str) -> Result<(), ContractError> {
    let hex = integrity.strip_prefix("sha256:").ok_or_else(|| {
        ContractError::invalid(format!(
            "library '{name}' version '{key}' must declare integrity as sha256:<hex>"
        ))
    })?;
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ContractError::invalid(format!(
            "library '{name}' version '{key}' integrity must carry exactly 64 lowercase \
             hexadecimal digest characters"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;
    use crate::contracts::{decode_contract, decode_contract_value, encode_contract};

    const COMPLETE_V1: &[u8] =
        include_bytes!("../../../tests/fixtures/contracts/rwe-library-manifest/v1-complete.json");

    fn valid_value() -> Value {
        serde_json::from_slice(COMPLETE_V1).expect("the fixture is JSON")
    }

    /// Mutates one version entry of the otherwise-valid fixture.
    fn with_version_field(field: &str, value: Value) -> Value {
        let mut document = valid_value();
        document["spec"]["versions"]["full-1.41"][field] = value;
        document
    }

    #[test]
    fn complete_v1_fixture_is_canonical_and_roundtrips_byte_for_byte() {
        let document = decode_contract::<RweLibraryManifestContract>(COMPLETE_V1).unwrap();
        assert_eq!(document.kind, "RweLibraryManifest");
        assert_eq!(document.metadata.name, "zeb/prosemirror");
        assert_eq!(document.spec.exports.len(), 3);
        let release = &document.spec.versions["full-1.41"];
        assert_eq!(release.entry, "0.1/runtime/prosemirror.bundle.mjs");
        assert_eq!(release.source, "offline");
        assert_eq!(release.size_bytes, 243_123);
        assert_eq!(
            encode_contract::<RweLibraryManifestContract>(document.metadata, document.spec)
                .unwrap(),
            COMPLETE_V1
        );
    }

    #[test]
    fn malformed_bytes_and_unknown_fields_are_refused() {
        assert!(decode_contract::<RweLibraryManifestContract>(b"{not json").is_err());

        let mut unknown_root = valid_value();
        unknown_root["status"] = json!({});
        assert!(decode_contract_value::<RweLibraryManifestContract>(unknown_root).is_err());

        let mut unknown_spec = valid_value();
        unknown_spec["spec"]["registry_url"] = json!("https://cdn.example/x.mjs");
        assert!(decode_contract_value::<RweLibraryManifestContract>(unknown_spec).is_err());

        let unknown_version =
            with_version_field("registry_url", json!("https://cdn.example/x.mjs"));
        assert!(decode_contract_value::<RweLibraryManifestContract>(unknown_version).is_err());
    }

    #[test]
    fn a_wrong_kind_and_a_future_api_version_are_refused() {
        let mut wrong_kind = valid_value();
        wrong_kind["kind"] = json!("Pipeline");
        assert_eq!(
            decode_contract_value::<RweLibraryManifestContract>(wrong_kind)
                .unwrap_err()
                .category(),
            "unexpected_kind"
        );

        // The pre-rename name is gone, not aliased: a document still carrying
        // it is unknown, never silently read.
        let mut old_kind = valid_value();
        old_kind["kind"] = json!("LibraryManifest");
        assert_eq!(
            decode_contract_value::<RweLibraryManifestContract>(old_kind)
                .unwrap_err()
                .category(),
            "unknown_kind"
        );

        let mut future = valid_value();
        future["apiVersion"] = json!("zebflow.com/v2");
        assert_eq!(
            decode_contract_value::<RweLibraryManifestContract>(future)
                .unwrap_err()
                .category(),
            "unsupported_api_version"
        );
    }

    #[test]
    fn integrity_is_required_and_must_be_a_sha256_digest() {
        for refused in [
            json!(""),
            json!("9c1e385bc9918e55ca7214dd995efbfc73bff14806ba13ee5c3f6ffcb4761eae"),
            json!("sha256:beef"),
            json!(format!("sha256:{}", "G".repeat(64))),
        ] {
            assert!(
                decode_contract_value::<RweLibraryManifestContract>(with_version_field(
                    "integrity",
                    refused.clone()
                ))
                .is_err(),
                "accepted integrity {refused}"
            );
        }
        // Missing entirely is refused, not defaulted to empty.
        let mut missing = valid_value();
        missing["spec"]["versions"]["full-1.41"]
            .as_object_mut()
            .unwrap()
            .remove("integrity");
        assert!(decode_contract_value::<RweLibraryManifestContract>(missing).is_err());
    }

    #[test]
    fn a_zero_size_is_refused() {
        assert!(
            decode_contract_value::<RweLibraryManifestContract>(with_version_field(
                "size_bytes",
                json!(0)
            ))
            .is_err()
        );
    }

    #[test]
    fn an_entry_may_only_descend() {
        for refused in [
            "",
            "../outside.mjs",
            "0.1/../../outside.mjs",
            "/etc/passwd",
            "0.1\\runtime\\entry.mjs",
            "0.1//runtime/entry.mjs",
            "./entry.mjs",
        ] {
            assert!(
                decode_contract_value::<RweLibraryManifestContract>(with_version_field(
                    "entry",
                    json!(refused)
                ))
                .is_err(),
                "accepted entry '{refused}'"
            );
        }
    }

    #[test]
    fn unknown_sources_are_refused_and_online_is_gone() {
        for refused in ["online", "cdn", "embedded", ""] {
            assert!(
                decode_contract_value::<RweLibraryManifestContract>(with_version_field(
                    "source",
                    json!(refused)
                ))
                .is_err(),
                "accepted source '{refused}'"
            );
        }
        // `hub` — an installed copy — is the one other loading form.
        assert!(
            decode_contract_value::<RweLibraryManifestContract>(with_version_field(
                "source",
                json!("hub")
            ))
            .is_ok()
        );
    }

    #[test]
    fn empty_exports_and_empty_versions_are_refused() {
        let mut no_exports = valid_value();
        no_exports["spec"]["exports"] = json!([]);
        assert!(decode_contract_value::<RweLibraryManifestContract>(no_exports).is_err());

        let mut no_versions = valid_value();
        no_versions["spec"]["versions"] = json!({});
        assert!(decode_contract_value::<RweLibraryManifestContract>(no_versions).is_err());
    }

    #[test]
    fn metadata_name_must_match_the_library_name() {
        let mut mismatched = valid_value();
        mismatched["metadata"]["name"] = json!("zeb/other");
        assert!(decode_contract_value::<RweLibraryManifestContract>(mismatched).is_err());
    }
}

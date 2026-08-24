use std::collections::HashSet;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::contracts::{ContractError, ContractKind, ContractMetadata, PlatformContract};

/// The file name a static repository serves its index at.
///
/// A repository is discovered by one URL and nothing else, so the index has a
/// fixed name under it rather than a configurable one: `<base>/` plus this.
pub const HUB_REPOSITORY_INDEX_FILE: &str = "zebflow-repository.json";

/// Maximum serialized size of one `HubRepositoryIndex` document.
///
/// The index is fetched before anything is chosen, so it is the one document a
/// repository can make a reader download without the reader having asked for a
/// package. It is metadata only -- no bytes of any package travel in it -- so
/// the ceiling is small enough that a hostile index cannot be a download.
pub const MAX_HUB_REPOSITORY_INDEX_BYTES: usize = 8 * 1024 * 1024;

/// Maximum packages one index may list.
pub const MAX_HUB_REPOSITORY_INDEX_PACKAGES: usize = 4096;

/// Maximum releases one package may list.
pub const MAX_HUB_REPOSITORY_INDEX_RELEASES: usize = 512;

/// Maximum length of one single-line index text field.
pub const MAX_HUB_REPOSITORY_INDEX_TEXT_BYTES: usize = 4096;

/// Maximum length of a package's one-line description.
pub const MAX_HUB_REPOSITORY_INDEX_DESCRIPTION_BYTES: usize = 64 * 1024;

/// Decode one bounded canonical HubRepositoryIndex document.
pub fn decode_hub_repository_index(
    bytes: &[u8],
) -> Result<crate::contracts::ContractDocument<HubRepositoryIndexSpec>, ContractError> {
    if bytes.len() > MAX_HUB_REPOSITORY_INDEX_BYTES {
        return Err(ContractError::invalid(format!(
            "HubRepositoryIndex exceeds the {MAX_HUB_REPOSITORY_INDEX_BYTES} byte limit"
        )));
    }
    crate::contracts::decode_contract::<HubRepositoryIndexContract>(bytes)
}

/// Encode one bounded canonical HubRepositoryIndex document.
pub fn encode_hub_repository_index(
    metadata: ContractMetadata,
    spec: HubRepositoryIndexSpec,
) -> Result<Vec<u8>, ContractError> {
    let bytes = crate::contracts::encode_contract::<HubRepositoryIndexContract>(metadata, spec)?;
    if bytes.len() > MAX_HUB_REPOSITORY_INDEX_BYTES {
        return Err(ContractError::invalid(format!(
            "HubRepositoryIndex exceeds the {MAX_HUB_REPOSITORY_INDEX_BYTES} byte limit"
        )));
    }
    Ok(bytes)
}

/// What one static repository offers, and where each release document sits.
///
/// A static repository is a directory: an index plus a set of `HubPackage`
/// documents plus content-addressed artifacts. This is the index, and it is the
/// only part of that directory whose format the repository itself owns --
/// everything it points at is a document format that already exists.
///
/// **It carries no package content.** A reader fetches the index to discover
/// what is available and to learn the path and digest of one release, and then
/// fetches that release. That split is what lets a repository be a plain web
/// server: discovery costs one file, and nothing serving it has to run code.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HubRepositoryIndexSpec {
    /// Every package this repository offers.
    ///
    /// A package this list omits is not obtainable from this repository even
    /// if its document is present: the index is the repository, and a reader
    /// never guesses a path.
    #[serde(default)]
    pub packages: Vec<HubRepositoryIndexPackage>,
}

/// One package, and the releases of it this repository holds.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HubRepositoryIndexPackage {
    /// The package identity, `{publisher}.{slug}` by convention.
    pub package_id: String,
    /// What kind of package this is, so a listing can be filtered before any
    /// release document is fetched.
    ///
    /// **Advisory.** The fetched `HubPackage` carries its own `asset_kind` and
    /// that one decides what the install does, so an index that misdescribes a
    /// package changes what a listing shows and nothing else.
    pub asset_kind: String,
    /// What this package is called.
    pub title: String,
    /// One line saying what it does.
    #[serde(default)]
    pub description: String,
    /// Which release a reference with no version resolves to.
    ///
    /// It must name one of `releases`, so an index cannot point a bare
    /// reference at a release it does not carry.
    pub latest_version: String,
    /// Every release of this package, newest first by convention and in no
    /// order this format enforces.
    pub releases: Vec<HubRepositoryIndexRelease>,
}

/// One release: where its document is, and which bytes are the right ones.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HubRepositoryIndexRelease {
    /// The release version, matching the document's `metadata.version`.
    pub version: String,
    /// Where the `HubPackage` document sits, relative to the repository base.
    pub path: String,
    /// The digest of that document's bytes.
    ///
    /// **Required.** A static file can be replaced and a git tag can be moved,
    /// so a version string alone is not an identity. Pinning the document here
    /// means a release swapped underneath a version fails closed at the fetch,
    /// and a reader never has to trust that a path still holds what it held.
    pub sha256: String,
    /// The document's size, so a reader bounds the read before it starts.
    pub size_bytes: usize,
}

impl HubRepositoryIndexSpec {
    /// The release a reference resolves to, or `None`.
    ///
    /// An absent version means `latest_version`, which validation has already
    /// proven names a release this index carries.
    pub fn find_release(
        &self,
        package_id: &str,
        version: Option<&str>,
    ) -> Option<(&HubRepositoryIndexPackage, &HubRepositoryIndexRelease)> {
        let package = self
            .packages
            .iter()
            .find(|item| item.package_id == package_id)?;
        let wanted = version.unwrap_or(package.latest_version.as_str());
        let release = package
            .releases
            .iter()
            .find(|item| item.version == wanted)?;
        Some((package, release))
    }
}

/// Canonical static repository index contract.
pub struct HubRepositoryIndexContract;

impl PlatformContract for HubRepositoryIndexContract {
    type Spec = HubRepositoryIndexSpec;
    const KIND: ContractKind = ContractKind::HubRepositoryIndex;

    fn validate(_metadata: &ContractMetadata, spec: &Self::Spec) -> Result<(), ContractError> {
        validate_limit(
            "spec.packages",
            spec.packages.len(),
            MAX_HUB_REPOSITORY_INDEX_PACKAGES,
        )?;
        let mut seen_packages = HashSet::new();
        for (index, package) in spec.packages.iter().enumerate() {
            let at = format!("spec.packages[{index}]");
            validate_package_id(&format!("{at}.package_id"), &package.package_id)?;
            // One id offered twice cannot say which release it means, and a
            // reader that took the first would resolve by file order.
            if !seen_packages.insert(package.package_id.as_str()) {
                return Err(ContractError::invalid(format!(
                    "spec.packages lists '{}' more than once",
                    package.package_id
                )));
            }
            validate_asset_kind(&format!("{at}.asset_kind"), &package.asset_kind)?;
            validate_single_line(&format!("{at}.title"), &package.title)?;
            validate_length(
                &format!("{at}.description"),
                &package.description,
                MAX_HUB_REPOSITORY_INDEX_DESCRIPTION_BYTES,
            )?;
            if package.releases.is_empty() {
                return Err(ContractError::invalid(format!(
                    "{at}.releases must name at least one release"
                )));
            }
            validate_limit(
                &format!("{at}.releases"),
                package.releases.len(),
                MAX_HUB_REPOSITORY_INDEX_RELEASES,
            )?;
            let mut seen_versions = HashSet::new();
            for (release_index, release) in package.releases.iter().enumerate() {
                let release_at = format!("{at}.releases[{release_index}]");
                validate_version(&format!("{release_at}.version"), &release.version)?;
                if !seen_versions.insert(release.version.as_str()) {
                    return Err(ContractError::invalid(format!(
                        "{at}.releases lists version '{}' more than once",
                        release.version
                    )));
                }
                validate_relative_file(&format!("{release_at}.path"), &release.path)?;
                validate_sha256(&format!("{release_at}.sha256"), &release.sha256)?;
                if release.size_bytes == 0
                    || release.size_bytes > super::hub_package::MAX_HUB_PACKAGE_BYTES
                {
                    return Err(ContractError::invalid(format!(
                        "{release_at}.size_bytes must be between 1 and {}",
                        super::hub_package::MAX_HUB_PACKAGE_BYTES
                    )));
                }
            }
            validate_version(&format!("{at}.latest_version"), &package.latest_version)?;
            if !seen_versions.contains(package.latest_version.as_str()) {
                return Err(ContractError::invalid(format!(
                    "{at}.latest_version '{}' names no release this index carries",
                    package.latest_version
                )));
            }
        }
        Ok(())
    }
}

fn validate_limit(path: &str, actual: usize, limit: usize) -> Result<(), ContractError> {
    if actual > limit {
        return Err(ContractError::invalid(format!(
            "{path} exceeds the limit of {limit}"
        )));
    }
    Ok(())
}

fn validate_length(path: &str, value: &str, limit: usize) -> Result<(), ContractError> {
    if value.len() > limit {
        return Err(ContractError::invalid(format!(
            "{path} exceeds the {limit} byte limit"
        )));
    }
    Ok(())
}

fn validate_single_line(path: &str, value: &str) -> Result<(), ContractError> {
    validate_length(path, value, MAX_HUB_REPOSITORY_INDEX_TEXT_BYTES)?;
    if value.chars().any(char::is_control) {
        return Err(ContractError::invalid(format!(
            "{path} must not contain control characters"
        )));
    }
    Ok(())
}

/// A package id is a bare listing name, never a location.
///
/// It is checked here rather than where it is used because the same id becomes
/// a project slug at install and a listing key in a client, and a name that can
/// hold a separator would be a path in one of those places.
fn validate_package_id(path: &str, value: &str) -> Result<(), ContractError> {
    let ok = !value.is_empty()
        && value.len() <= 256
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || byte == b'.'
                || byte == b'-'
                || byte == b'_'
        })
        && !value.starts_with('.')
        && !value.ends_with('.');
    if !ok {
        return Err(ContractError::invalid(format!(
            "{path} '{value}' must be lowercase letters, digits, '.', '-', or '_'"
        )));
    }
    Ok(())
}

fn validate_asset_kind(path: &str, value: &str) -> Result<(), ContractError> {
    let ok = !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
    if !ok {
        return Err(ContractError::invalid(format!(
            "{path} '{value}' must be a lowercase token of letters, digits, or '_'"
        )));
    }
    Ok(())
}

/// A version is an opaque single-line token that is never a path segment.
fn validate_version(path: &str, value: &str) -> Result<(), ContractError> {
    let ok = !value.is_empty()
        && value.len() <= 256
        && !value.contains('/')
        && !value.contains('\\')
        && !value.chars().any(char::is_control);
    if !ok {
        return Err(ContractError::invalid(format!(
            "{path} '{value}' must be a non-empty single-line token containing no path separator"
        )));
    }
    Ok(())
}

fn validate_sha256(path: &str, value: &str) -> Result<(), ContractError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ContractError::invalid(format!(
            "{path} must be 64 lowercase hexadecimal digits"
        )));
    }
    Ok(())
}

/// A document path is joined onto a URL the user named, so it may only ever
/// descend.
///
/// `..`, an absolute path, and a backslash are each a way to make one fetch
/// address something the repository base does not contain, and a reader that
/// accepted them would let an index reach outside the location the user trusted.
fn validate_relative_file(path: &str, value: &str) -> Result<(), ContractError> {
    let value_path = Path::new(value);
    if value.is_empty()
        || value.len() > MAX_HUB_REPOSITORY_INDEX_TEXT_BYTES
        || value.contains('\\')
        || value.contains("//")
        || value.chars().any(char::is_control)
        || value_path.is_absolute()
        || value_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir
                    | Component::CurDir
                    | Component::RootDir
                    | Component::Prefix(_)
            )
        })
    {
        return Err(ContractError::invalid(format!(
            "{path} must be a normalized relative file path"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const V1_INDEX: &[u8] =
        include_bytes!("../../../tests/fixtures/contracts/hub-repository-index/v1-complete.json");

    fn mutate(bytes: &[u8], mutation: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
        let mut value: serde_json::Value = serde_json::from_slice(bytes).expect("json");
        mutation(&mut value);
        serde_json::to_vec(&value).expect("json bytes")
    }

    fn release_field<'a>(value: &'a mut serde_json::Value) -> &'a mut serde_json::Value {
        &mut value["spec"]["packages"][0]["releases"][0]
    }

    #[test]
    fn the_reference_index_decodes_and_resolves_its_latest() {
        let document = decode_hub_repository_index(V1_INDEX).expect("decode");
        assert_eq!(document.metadata.name, "zebflow-hub");
        let (package, release) = document
            .spec
            .find_release("zebflow.kids-educational-games", None)
            .expect("latest resolves");
        assert_eq!(package.asset_kind, "project_bundle");
        assert_eq!(release.version, "1.0.0");
        assert_eq!(
            release.path,
            "packages/zebflow.kids-educational-games/1.0.0/package.json"
        );
    }

    #[test]
    fn an_unknown_field_is_refused_rather_than_ignored() {
        let bytes = mutate(V1_INDEX, |value| {
            value["spec"]["packages"][0]["mirror"] = serde_json::json!("https://elsewhere");
        });
        decode_hub_repository_index(&bytes).expect_err("unknown field");
    }

    #[test]
    fn a_release_path_may_not_leave_the_repository_base() {
        for escape in ["../../etc/passwd", "/etc/passwd", "a\\b", "a//b"] {
            let bytes = mutate(V1_INDEX, |value| {
                release_field(value)["path"] = serde_json::json!(escape);
            });
            decode_hub_repository_index(&bytes).expect_err(escape);
        }
    }

    #[test]
    fn a_release_without_a_usable_digest_is_refused() {
        for digest in ["", "abc", &"A".repeat(64)] {
            let bytes = mutate(V1_INDEX, |value| {
                release_field(value)["sha256"] = serde_json::json!(digest);
            });
            decode_hub_repository_index(&bytes).expect_err(digest);
        }
    }

    #[test]
    fn latest_version_must_name_a_release_the_index_carries() {
        let bytes = mutate(V1_INDEX, |value| {
            value["spec"]["packages"][0]["latest_version"] = serde_json::json!("9.9.9");
        });
        let error = decode_hub_repository_index(&bytes).expect_err("dangling latest");
        assert!(error.to_string().contains("names no release"), "{error}");
    }

    #[test]
    fn one_id_offered_twice_is_refused() {
        let bytes = mutate(V1_INDEX, |value| {
            let first = value["spec"]["packages"][0].clone();
            value["spec"]["packages"]
                .as_array_mut()
                .expect("packages")
                .push(first);
        });
        let error = decode_hub_repository_index(&bytes).expect_err("duplicate id");
        assert!(error.to_string().contains("more than once"), "{error}");
    }

    #[test]
    fn a_package_with_no_releases_is_refused() {
        let bytes = mutate(V1_INDEX, |value| {
            value["spec"]["packages"][0]["releases"] = serde_json::json!([]);
        });
        decode_hub_repository_index(&bytes).expect_err("no releases");
    }

    #[test]
    fn another_kinds_document_is_not_read_as_an_index() {
        let bytes = mutate(V1_INDEX, |value| {
            value["kind"] = serde_json::json!("HubPackage");
        });
        decode_hub_repository_index(&bytes).expect_err("wrong kind");
    }

    #[test]
    fn a_future_api_version_is_refused_rather_than_guessed() {
        let bytes = mutate(V1_INDEX, |value| {
            value["apiVersion"] = serde_json::json!("zebflow.com/v2");
        });
        decode_hub_repository_index(&bytes).expect_err("wrong api version");
    }

    #[test]
    fn the_reference_index_round_trips() {
        let document = decode_hub_repository_index(V1_INDEX).expect("decode");
        let bytes = encode_hub_repository_index(document.metadata.clone(), document.spec.clone())
            .expect("encode");
        let again = decode_hub_repository_index(&bytes).expect("decode again");
        assert_eq!(again.spec, document.spec);
    }
}

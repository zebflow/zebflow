//! Asset marketplace service.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::Engine as _;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    CreateMarketplaceTokenRequest, CreateProjectRequest, MarketplaceAssetPackage,
    MarketplaceAssetVersion, MarketplaceAuthority, MarketplacePublisher, MarketplaceToken,
    PlatformMarketplaceRepository, PlatformServiceInstance, ProjectFileLayout,
    ProjectMarketplaceRepository, ProjectRuntimeSelectionRequest, ZebflowJson, now_ts,
    slug_segment,
};
use crate::platform::services::ProjectService;
use crate::platform::services::tsx_outline::extract_import_sources;

pub struct MarketplaceService {
    control_data: Arc<dyn DataAdapter>,
    marketplace_data: Arc<dyn DataAdapter>,
    projects: Arc<ProjectService>,
    data_root: PathBuf,
}

pub const DEFAULT_MARKETPLACE_SERVICE_INSTANCE_ID: &str = "marketplace-default";
pub const MARKETPLACE_SERVICE_KIND: &str = "marketplace";
const MARKETPLACE_SERVICE_SCOPE_OWNER: &str = "marketplace-service";
const MARKETPLACE_SERVICE_SCOPE_PROJECT: &str = "marketplace-default";
const MAX_REMOTE_MARKETPLACE_ARTIFACT_BYTES: u64 = 25 * 1024 * 1024;
const DEFAULT_PUBLISHER_MAX_PACKAGES: i64 = 20;
const DEFAULT_PUBLISHER_MAX_PACKAGE_BYTES: i64 = 10 * 1024 * 1024;
const DEFAULT_PUBLISHER_MAX_MEDIA_FILES: i64 = 8;
const DEFAULT_PUBLISHER_MAX_IMAGE_BYTES: i64 = 2 * 1024 * 1024;

fn default_marketplace_base_url() -> String {
    std::env::var("ZEBFLOW_MARKETPLACE_DEFAULT_BASE_URL")
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "https://market.zebflow.com/api".to_string())
}

fn preserve_or_replace_token(existing: Option<&str>, incoming: &str) -> String {
    let trimmed = incoming.trim();
    if trimmed.is_empty() {
        existing.unwrap_or_default().to_string()
    } else {
        trimmed.to_string()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MarketplacePublishSourceItem {
    pub source_type: String,
    pub source_ref: String,
    pub name: String,
    pub description: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceExportEntry {
    pub rel_path: String,
    pub kind: String,
    pub size_bytes: usize,
    pub reason: String,
    #[serde(default)]
    pub encoding: String,
    #[serde(default)]
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MarketplaceExportPreview {
    pub asset_kind: String,
    pub source_type: String,
    pub source_ref: String,
    pub name: String,
    pub description: String,
    pub entries: Vec<MarketplaceExportEntry>,
    pub warnings: Vec<String>,
    pub total_files: usize,
    pub total_bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct MarketplaceInstallResult {
    pub package_id: String,
    pub version: String,
    pub install_root: String,
    pub files_written: usize,
    pub pipelines_registered: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MarketplaceRemotePackRow {
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
    pub visibility: String,
    pub tags: Vec<String>,
    pub latest_version: String,
    pub updated_at: i64,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RemoteMarketplaceListResponse {
    items: Vec<RemoteMarketplaceAssetItem>,
}

#[derive(Debug, Clone, Deserialize)]
struct RemoteMarketplaceAssetItem {
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
    visibility: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    latest_version: String,
    #[serde(default)]
    updated_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct RemoteMarketplaceArtifactResponse {
    version: RemoteMarketplaceAssetVersion,
    #[serde(default)]
    artifact_sha256: String,
    #[serde(default)]
    artifact_size_bytes: u64,
    artifact: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct RemoteMarketplaceAssetVersion {
    #[serde(default)]
    artifact_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MarketplaceArtifact {
    schema: String,
    asset_kind: String,
    #[serde(default)]
    source_type: String,
    #[serde(default)]
    source_owner: String,
    #[serde(default)]
    source_project: String,
    #[serde(default)]
    source_ref: String,
    #[serde(default)]
    publisher_id: String,
    #[serde(default)]
    publisher_display_name: String,
    #[serde(default)]
    publisher_url: String,
    #[serde(default)]
    publisher_email: String,
    title: String,
    description: String,
    files: Vec<MarketplaceExportEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteMarketplacePublishRequest {
    pub package_id: String,
    pub version: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub visibility: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub source_owner: String,
    pub source_project: String,
    pub source_kind: String,
    pub source_ref: String,
    pub artifact: Value,
}

impl MarketplaceService {
    pub fn new(
        control_data: Arc<dyn DataAdapter>,
        marketplace_data: Arc<dyn DataAdapter>,
        projects: Arc<ProjectService>,
        data_root: PathBuf,
    ) -> Self {
        Self {
            control_data,
            marketplace_data,
            projects,
            data_root,
        }
    }

    pub fn get_authority(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Option<MarketplaceAuthority>, PlatformError> {
        self.marketplace_data
            .get_marketplace_authority(&slug_segment(owner), &slug_segment(project))
    }

    pub fn get_default_service_instance(
        &self,
    ) -> Result<Option<PlatformServiceInstance>, PlatformError> {
        self.control_data
            .get_platform_service_instance(DEFAULT_MARKETPLACE_SERVICE_INSTANCE_ID)
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
                "MARKETPLACE_SERVICE_HOST_REQUIRED",
                "marketplace service host office is required",
            ));
        }
        if self
            .control_data
            .get_platform_office(host_office_id)?
            .is_none()
        {
            return Err(PlatformError::new(
                "MARKETPLACE_SERVICE_HOST_MISSING",
                "marketplace service host office was not found",
            ));
        }
        let public_base_url = public_base_url.trim().trim_end_matches('/');
        let public_base_url = if public_base_url.is_empty() {
            default_marketplace_base_url()
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
            service_instance_id: DEFAULT_MARKETPLACE_SERVICE_INSTANCE_ID.to_string(),
            service_kind: MARKETPLACE_SERVICE_KIND.to_string(),
            display_label: "Marketplace".to_string(),
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
    ) -> Result<MarketplaceAuthority, PlatformError> {
        let _ = (owner, project);
        let authority = self.ensure_service_authority()?;
        let now = now_ts();
        let next = MarketplaceAuthority {
            enabled,
            updated_at: now,
            ..authority
        };
        self.marketplace_data.put_marketplace_authority(&next)?;
        Ok(next)
    }

    pub fn list_asset_packages(&self) -> Result<Vec<MarketplaceAssetPackage>, PlatformError> {
        self.require_enabled()?;
        self.marketplace_data.list_marketplace_asset_packages()
    }

    pub fn list_asset_packages_by_owner(
        &self,
        owner: &str,
    ) -> Result<Vec<MarketplaceAssetPackage>, PlatformError> {
        let owner = slug_segment(owner);
        self.require_enabled()?;
        let mut items = self.marketplace_data.list_marketplace_asset_packages()?;
        items.retain(|item| item.publisher_owner == owner);
        Ok(items)
    }

    pub fn list_asset_versions(
        &self,
        package_id: &str,
    ) -> Result<Vec<MarketplaceAssetVersion>, PlatformError> {
        self.require_enabled()?;
        self.marketplace_data
            .list_marketplace_asset_versions(package_id)
    }

    pub fn list_publishers(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<MarketplacePublisher>, PlatformError> {
        let _ = (owner, project);
        self.require_enabled()?;
        self.marketplace_data.list_marketplace_publishers(
            MARKETPLACE_SERVICE_SCOPE_OWNER,
            MARKETPLACE_SERVICE_SCOPE_PROJECT,
        )
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
    ) -> Result<MarketplacePublisher, PlatformError> {
        let _ = (owner, project);
        self.require_enabled()?;
        let authority = self.ensure_service_authority()?;
        let publisher_id = slug_segment(publisher_id);
        if publisher_id.is_empty() {
            return Err(PlatformError::new(
                "MARKETPLACE_PUBLISHER_INVALID",
                "publisher id must not be empty",
            ));
        }
        let now = now_ts();
        let existing = self.marketplace_data.get_marketplace_publisher(
            MARKETPLACE_SERVICE_SCOPE_OWNER,
            MARKETPLACE_SERVICE_SCOPE_PROJECT,
            &publisher_id,
        )?;
        let row = MarketplacePublisher {
            authority_id: authority.authority_id,
            publisher_pk: existing
                .as_ref()
                .map(|v| v.publisher_pk.clone())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| format!("mpub_{}", random_hex(8))),
            owner: MARKETPLACE_SERVICE_SCOPE_OWNER.to_string(),
            project: MARKETPLACE_SERVICE_SCOPE_PROJECT.to_string(),
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
        self.marketplace_data.put_marketplace_publisher(&row)?;
        Ok(row)
    }

    pub fn delete_publisher(
        &self,
        owner: &str,
        project: &str,
        publisher_id: &str,
    ) -> Result<(), PlatformError> {
        let _ = (owner, project);
        self.marketplace_data.delete_marketplace_publisher(
            MARKETPLACE_SERVICE_SCOPE_OWNER,
            MARKETPLACE_SERVICE_SCOPE_PROJECT,
            &slug_segment(publisher_id),
        )
    }

    pub fn create_token(
        &self,
        owner: &str,
        project: &str,
        req: &CreateMarketplaceTokenRequest,
    ) -> Result<(MarketplaceToken, String), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        self.require_enabled()?;
        let authority = self.ensure_service_authority()?;
        if owner.is_empty() || project.is_empty() {
            return Err(PlatformError::new(
                "MARKETPLACE_TOKEN_INVALID",
                "owner/project must not be empty",
            ));
        }
        let publisher_id = slug_segment(&req.publisher_id);
        let title = req.title.trim();
        if title.is_empty() || publisher_id.is_empty() {
            return Err(PlatformError::new(
                "MARKETPLACE_TOKEN_INVALID",
                "token title and publisher id must not be empty",
            ));
        }
        let Some(publisher) = self.marketplace_data.get_marketplace_publisher(
            MARKETPLACE_SERVICE_SCOPE_OWNER,
            MARKETPLACE_SERVICE_SCOPE_PROJECT,
            &publisher_id,
        )?
        else {
            return Err(PlatformError::new(
                "MARKETPLACE_PUBLISHER_MISSING",
                "publisher not found",
            ));
        };
        if !publisher.enabled {
            return Err(PlatformError::new(
                "MARKETPLACE_PUBLISHER_DISABLED",
                "publisher is disabled",
            ));
        }
        let scopes = normalize_scopes(&req.scopes);
        validate_publisher_scopes(&publisher, &scopes)?;
        let now = now_ts();
        let token_id = format!("mkt_{}", random_hex(8));
        let secret = random_hex(24);
        let plain = format!("zfmt_{token_id}_{secret}");
        let token = MarketplaceToken {
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
        self.marketplace_data.put_marketplace_token(&token)?;
        Ok((token, plain))
    }

    pub fn list_tokens(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<MarketplaceToken>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        self.require_enabled()?;
        self.marketplace_data
            .list_marketplace_tokens(&owner, &project)
    }

    pub fn list_all_tokens(&self) -> Result<Vec<MarketplaceToken>, PlatformError> {
        self.require_enabled()?;
        self.marketplace_data.list_all_marketplace_tokens()
    }

    pub fn revoke_token(
        &self,
        owner: &str,
        project: &str,
        token_id: &str,
    ) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let Some(mut token) = self.marketplace_data.get_marketplace_token(token_id)? else {
            return Ok(());
        };
        if token.owner != owner || token.project != project {
            return Err(PlatformError::new(
                "MARKETPLACE_TOKEN_FORBIDDEN",
                "token does not belong to this marketplace",
            ));
        }
        token.revoked_at = Some(now_ts());
        token.updated_at = now_ts();
        self.marketplace_data.put_marketplace_token(&token)?;
        Ok(())
    }

    pub fn revoke_token_any(&self, token_id: &str) -> Result<(), PlatformError> {
        self.require_enabled()?;
        let Some(mut token) = self.marketplace_data.get_marketplace_token(token_id)? else {
            return Ok(());
        };
        token.revoked_at = Some(now_ts());
        token.updated_at = now_ts();
        self.marketplace_data.put_marketplace_token(&token)?;
        Ok(())
    }

    pub fn list_repositories(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectMarketplaceRepository>, PlatformError> {
        self.control_data
            .list_project_marketplace_repositories(&slug_segment(owner), &slug_segment(project))
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
        let existing = self
            .control_data
            .list_project_marketplace_repositories(&owner, &project)?;
        if existing
            .iter()
            .any(|item| item.repository_id == "zebflow-com")
        {
            return Ok(());
        }
        let now = now_ts();
        self.control_data
            .put_project_marketplace_repository(&ProjectMarketplaceRepository {
                owner,
                project,
                repository_id: "zebflow-com".to_string(),
                title: "Zebflow Marketplace".to_string(),
                base_url: default_marketplace_base_url(),
                remote_owner: String::new(),
                remote_project: String::new(),
                read_token: String::new(),
                enabled: true,
                created_at: now,
                updated_at: now,
            })?;
        Ok(())
    }

    pub fn list_platform_repositories(
        &self,
        owner: &str,
    ) -> Result<Vec<PlatformMarketplaceRepository>, PlatformError> {
        self.control_data
            .list_platform_marketplace_repositories(&slug_segment(owner))
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
    ) -> Result<PlatformMarketplaceRepository, PlatformError> {
        let owner = slug_segment(owner);
        let repository_id = slug_segment(repository_id);
        if owner.is_empty() || repository_id.is_empty() || base_url.trim().is_empty() {
            return Err(PlatformError::new(
                "MARKETPLACE_REPOSITORY_INVALID",
                "repository fields must not be empty",
            ));
        }
        validate_remote_marketplace_url(base_url.trim().trim_end_matches('/'))?;
        let uses_direct_base = is_direct_marketplace_base(base_url);
        if !uses_direct_base && (remote_owner.trim().is_empty() || remote_project.trim().is_empty())
        {
            return Err(PlatformError::new(
                "MARKETPLACE_REPOSITORY_INVALID",
                "remote owner and project are required unless base_url already points to /api/projects/{owner}/{project}/marketplace",
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
            .list_platform_marketplace_repositories(&owner)?
            .into_iter()
            .find(|item| item.repository_id == repository_id);
        let row = PlatformMarketplaceRepository {
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
                "Remote Marketplace".to_string()
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
        self.control_data
            .put_platform_marketplace_repository(&row)?;
        Ok(row)
    }

    pub fn delete_platform_repository(
        &self,
        owner: &str,
        repository_id: &str,
    ) -> Result<(), PlatformError> {
        self.control_data.delete_platform_marketplace_repository(
            &slug_segment(owner),
            &slug_segment(repository_id),
        )
    }

    pub fn ensure_default_platform_repository(&self, owner: &str) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        if owner.is_empty() {
            return Ok(());
        }
        let existing = self
            .control_data
            .list_platform_marketplace_repositories(&owner)?;
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
            .put_platform_marketplace_repository(&PlatformMarketplaceRepository {
                source_id: format!("pmr_{}", random_hex(8)),
                owner_user_id,
                owner,
                repository_id: "zebflow-com".to_string(),
                title: "Zebflow Marketplace".to_string(),
                base_url: default_marketplace_base_url(),
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
    ) -> Result<ProjectMarketplaceRepository, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let repository_id = slug_segment(repository_id);
        if owner.is_empty()
            || project.is_empty()
            || repository_id.is_empty()
            || base_url.trim().is_empty()
        {
            return Err(PlatformError::new(
                "MARKETPLACE_REPOSITORY_INVALID",
                "repository fields must not be empty",
            ));
        }
        validate_remote_marketplace_url(base_url.trim().trim_end_matches('/'))?;
        let uses_direct_base = is_direct_marketplace_base(base_url);
        if !uses_direct_base && (remote_owner.trim().is_empty() || remote_project.trim().is_empty())
        {
            return Err(PlatformError::new(
                "MARKETPLACE_REPOSITORY_INVALID",
                "remote owner and project are required unless base_url already points to /api/projects/{owner}/{project}/marketplace",
            ));
        }
        let now = now_ts();
        let existing = self
            .control_data
            .list_project_marketplace_repositories(&owner, &project)?
            .into_iter()
            .find(|item| item.repository_id == repository_id);
        let row = ProjectMarketplaceRepository {
            owner,
            project,
            repository_id,
            title: if title.trim().is_empty() {
                "Remote Marketplace".to_string()
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
        self.control_data.put_project_marketplace_repository(&row)?;
        Ok(row)
    }

    pub fn delete_repository(
        &self,
        owner: &str,
        project: &str,
        repository_id: &str,
    ) -> Result<(), PlatformError> {
        self.control_data.delete_project_marketplace_repository(
            &slug_segment(owner),
            &slug_segment(project),
            &slug_segment(repository_id),
        )
    }

    pub fn authenticate_token(
        &self,
        bearer_token: &str,
        required_scope: &str,
    ) -> Result<MarketplaceToken, PlatformError> {
        self.require_enabled()?;
        let token_value = bearer_token.trim();
        if token_value.is_empty() {
            return Err(PlatformError::new(
                "MARKETPLACE_TOKEN_INVALID",
                "token missing",
            ));
        }
        let token_hash = sha256_hex(token_value.as_bytes());
        let matched = self
            .marketplace_data
            .list_all_marketplace_tokens()?
            .into_iter()
            .find(|token| token.secret_hash == token_hash);
        let Some(mut token) = matched else {
            return Err(PlatformError::new(
                "MARKETPLACE_TOKEN_INVALID",
                "token not found",
            ));
        };
        if token.revoked_at.is_some() {
            return Err(PlatformError::new(
                "MARKETPLACE_TOKEN_REVOKED",
                "token revoked",
            ));
        }
        if let Some(expires_at) = token.expires_at
            && expires_at > 0
            && expires_at <= now_ts()
        {
            return Err(PlatformError::new(
                "MARKETPLACE_TOKEN_EXPIRED",
                "token expired",
            ));
        }
        if !token.grants_scope(required_scope) {
            return Err(PlatformError::new(
                "MARKETPLACE_TOKEN_FORBIDDEN",
                "scope missing",
            ));
        }
        let Some(publisher) = self.marketplace_data.get_marketplace_publisher(
            MARKETPLACE_SERVICE_SCOPE_OWNER,
            MARKETPLACE_SERVICE_SCOPE_PROJECT,
            &token.publisher_id,
        )?
        else {
            return Err(PlatformError::new(
                "MARKETPLACE_PUBLISHER_MISSING",
                "publisher not found",
            ));
        };
        if !publisher.enabled {
            return Err(PlatformError::new(
                "MARKETPLACE_PUBLISHER_DISABLED",
                "publisher is disabled",
            ));
        }
        validate_publisher_scopes(&publisher, &[required_scope.to_string()])?;
        token.last_used_at = Some(now_ts());
        token.updated_at = now_ts();
        self.marketplace_data.put_marketplace_token(&token)?;
        Ok(token)
    }

    pub fn list_publish_sources(
        &self,
        owner: &str,
        project: &str,
        source_type: &str,
    ) -> Result<Vec<MarketplacePublishSourceItem>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let source_type = normalize_source_type(source_type);
        match source_type.as_str() {
            "pipeline_with_dependencies" => {
                let rows = self.projects.list_pipeline_meta_rows(&owner, &project)?;
                Ok(rows
                    .into_iter()
                    .map(|item| MarketplacePublishSourceItem {
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
                    .map(|item| MarketplacePublishSourceItem {
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
                    .map(|path| MarketplacePublishSourceItem {
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
            "project_files" => Ok(vec![MarketplacePublishSourceItem {
                source_type,
                source_ref: ".".to_string(),
                name: format!("{project} project files"),
                description: "Entire repo workspace export".to_string(),
                path: "/".to_string(),
            }]),
            _ => Err(PlatformError::new(
                "MARKETPLACE_SOURCE_INVALID",
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
    ) -> Result<MarketplaceExportPreview, PlatformError> {
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
                "MARKETPLACE_SOURCE_INVALID",
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
        visibility: &str,
        tags: Vec<String>,
    ) -> Result<(MarketplaceAssetPackage, MarketplaceAssetVersion), PlatformError> {
        let _ = (authority_owner, authority_project);
        self.require_enabled()?;
        let authority_owner = MARKETPLACE_SERVICE_SCOPE_OWNER.to_string();
        let authority_project = MARKETPLACE_SERVICE_SCOPE_PROJECT.to_string();
        let publisher_owner = slug_segment(publisher_owner);
        let publisher_id = slug_segment(publisher_id);
        let source_owner = slug_segment(source_owner);
        let source_project = slug_segment(source_project);
        let source_type = normalize_source_type(source_type);
        let package_id = slug_segment(package_id);
        let version = version.trim();
        if authority_owner.is_empty()
            || authority_project.is_empty()
            || publisher_owner.is_empty()
            || publisher_id.is_empty()
            || source_owner.is_empty()
            || source_project.is_empty()
        {
            return Err(PlatformError::new(
                "MARKETPLACE_PUBLISH_INVALID",
                "authority, publisher, and source must not be empty",
            ));
        }
        let Some(publisher) = self.marketplace_data.get_marketplace_publisher(
            MARKETPLACE_SERVICE_SCOPE_OWNER,
            MARKETPLACE_SERVICE_SCOPE_PROJECT,
            &publisher_id,
        )?
        else {
            return Err(PlatformError::new(
                "MARKETPLACE_PUBLISHER_MISSING",
                "publisher not found",
            ));
        };
        if !publisher.enabled {
            return Err(PlatformError::new(
                "MARKETPLACE_PUBLISHER_DISABLED",
                "publisher is disabled",
            ));
        }
        if !publisher.can_publish {
            return Err(PlatformError::new(
                "MARKETPLACE_PUBLISHER_FORBIDDEN",
                "publisher does not have publish permission",
            ));
        }
        let authority = self.ensure_service_authority()?;
        if package_id.is_empty() || version.is_empty() {
            return Err(PlatformError::new(
                "MARKETPLACE_PUBLISH_INVALID",
                "package id and version must not be empty",
            ));
        }
        validate_marketplace_version(version)?;
        let mut preview =
            self.preview_publish_source(&source_owner, &source_project, &source_type, source_ref)?;
        if preview.entries.is_empty() {
            return Err(PlatformError::new(
                "MARKETPLACE_PUBLISH_EMPTY",
                "nothing to publish for this source",
            ));
        }
        sanitize_marketplace_export_entries(&mut preview.entries)?;
        let artifact_rel = format!(
            "services/{}/packages/{}/versions/{}/artifact.json",
            DEFAULT_MARKETPLACE_SERVICE_INSTANCE_ID, package_id, version
        );
        let artifact_abs = self.data_root.join(&artifact_rel);
        if let Some(parent) = artifact_abs.parent() {
            fs::create_dir_all(parent)?;
        }
        let now = now_ts();
        let manifest = MarketplaceArtifact {
            schema: "zebflow.asset-pack.v1".to_string(),
            asset_kind: preview.asset_kind.clone(),
            source_type: source_type.clone(),
            source_owner: source_owner.clone(),
            source_project: source_project.clone(),
            source_ref: source_ref.to_string(),
            publisher_id: publisher_id.clone(),
            publisher_display_name: if publisher_display_name.trim().is_empty() {
                publisher.display_name.clone()
            } else {
                publisher_display_name.trim().to_string()
            },
            publisher_url: if publisher_url.trim().is_empty() {
                publisher.publisher_url.clone()
            } else {
                normalize_publisher_url(&publisher_id, publisher_url)
            },
            publisher_email: if publisher_email.trim().is_empty() {
                publisher.email.clone()
            } else {
                publisher_email.trim().to_string()
            },
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
            files: preview.entries.clone(),
        };
        let artifact_bytes = serde_json::to_vec_pretty(&manifest)
            .map_err(|err| PlatformError::new("MARKETPLACE_PUBLISH", err.to_string()))?;
        let artifact_sha256 = sha256_hex(&artifact_bytes);
        let existing_package = self
            .marketplace_data
            .get_marketplace_asset_package(&package_id)?;
        self.enforce_publisher_package_quota(
            &publisher,
            &package_id,
            existing_package.as_ref(),
            artifact_bytes.len(),
        )?;
        fs::write(&artifact_abs, &artifact_bytes)?;
        let package = MarketplaceAssetPackage {
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
            publisher_display_name: manifest.publisher_display_name.clone(),
            publisher_url: manifest.publisher_url.clone(),
            publisher_email: manifest.publisher_email.clone(),
            asset_kind: preview.asset_kind.clone(),
            title: manifest.title.clone(),
            description: manifest.description.clone(),
            visibility: normalize_visibility(visibility),
            tags,
            created_at: existing_package.map(|item| item.created_at).unwrap_or(now),
            updated_at: now,
        };
        let version_row = MarketplaceAssetVersion {
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
                .map_err(|err| PlatformError::new("MARKETPLACE_PUBLISH", err.to_string()))?,
            created_at: now,
        };
        self.marketplace_data
            .put_marketplace_asset_package(&package)?;
        self.marketplace_data
            .put_marketplace_asset_version(&version_row)?;
        Ok((package, version_row))
    }

    pub fn install_asset(
        &self,
        target_owner: &str,
        target_project: &str,
        package_id: &str,
        version: &str,
    ) -> Result<MarketplaceInstallResult, PlatformError> {
        let target_owner = slug_segment(target_owner);
        let target_project = slug_segment(target_project);
        self.require_enabled()?;
        let Some(version_row) = self
            .marketplace_data
            .get_marketplace_asset_version(package_id, version)?
        else {
            return Err(PlatformError::new(
                "MARKETPLACE_ASSET_MISSING",
                "asset version not found",
            ));
        };
        let artifact_abs = self.data_root.join(&version_row.artifact_rel_path);
        let raw = fs::read_to_string(&artifact_abs)?;
        let payload: MarketplaceArtifact = serde_json::from_str(&raw)
            .map_err(|err| PlatformError::new("MARKETPLACE_INSTALL", err.to_string()))?;
        self.install_artifact_payload(target_owner, target_project, package_id, version, payload)
    }

    fn install_artifact_payload(
        &self,
        target_owner: String,
        target_project: String,
        package_id: &str,
        version: &str,
        payload: MarketplaceArtifact,
    ) -> Result<MarketplaceInstallResult, PlatformError> {
        let layout = self
            .projects
            .project_layout(&target_owner, &target_project)?;

        let mut pipelines_registered = Vec::new();
        for entry in &payload.files {
            let install_rel = install_rel_path(package_id, &entry.rel_path);
            let dest_abs = layout.repo_dir.join(&install_rel);
            if let Some(parent) = dest_abs.parent() {
                fs::create_dir_all(parent)?;
            }
            write_entry_content(&dest_abs, entry)?;
            if install_rel.ends_with(".zf.json") && install_rel.starts_with("pipelines/") {
                let source = fs::read_to_string(&dest_abs)?;
                let (title, trigger_kind) = infer_pipeline_meta(&source, &install_rel);
                let meta = self.projects.upsert_pipeline_definition(
                    &target_owner,
                    &target_project,
                    &install_rel,
                    &title,
                    "",
                    &trigger_kind,
                    &source,
                )?;
                pipelines_registered.push(meta.file_rel_path);
            }
        }

        Ok(MarketplaceInstallResult {
            package_id: package_id.to_string(),
            version: version.to_string(),
            install_root: install_root_for(package_id, &payload.asset_kind),
            files_written: payload.files.len(),
            pipelines_registered,
        })
    }

    pub fn get_asset_version_artifact(
        &self,
        package_id: &str,
        version: &str,
    ) -> Result<(MarketplaceAssetVersion, Value), PlatformError> {
        let (version_row, artifact, _) =
            self.get_asset_version_install_artifact(package_id, version)?;
        Ok((version_row, artifact))
    }

    pub fn get_asset_version_install_artifact(
        &self,
        package_id: &str,
        version: &str,
    ) -> Result<(MarketplaceAssetVersion, Value, u64), PlatformError> {
        self.require_enabled()?;
        let Some(version_row) = self
            .marketplace_data
            .get_marketplace_asset_version(package_id, version)?
        else {
            return Err(PlatformError::new(
                "MARKETPLACE_ASSET_MISSING",
                "asset version not found",
            ));
        };
        let artifact_abs = self.marketplace_artifact_path(&version_row.artifact_rel_path)?;
        let artifact_size_bytes = fs::metadata(&artifact_abs)?.len();
        if artifact_size_bytes > MAX_REMOTE_MARKETPLACE_ARTIFACT_BYTES {
            return Err(PlatformError::new(
                "MARKETPLACE_ARTIFACT_TOO_LARGE",
                "marketplace artifact exceeds maximum read size",
            ));
        }
        let raw = fs::read_to_string(&artifact_abs)?;
        let artifact = serde_json::from_str::<Value>(&raw)
            .map_err(|err| PlatformError::new("MARKETPLACE_INSTALL", err.to_string()))?;
        Ok((version_row, artifact, artifact_size_bytes))
    }

    pub fn import_remote_asset(
        &self,
        authority_owner: &str,
        authority_project: &str,
        token: &MarketplaceToken,
        req: &RemoteMarketplacePublishRequest,
    ) -> Result<(MarketplaceAssetPackage, MarketplaceAssetVersion), PlatformError> {
        let _ = (authority_owner, authority_project);
        self.require_enabled()?;
        let authority_owner = MARKETPLACE_SERVICE_SCOPE_OWNER.to_string();
        let authority_project = MARKETPLACE_SERVICE_SCOPE_PROJECT.to_string();
        let publisher_owner = slug_segment(&token.owner);
        let publisher_id = slug_segment(&token.publisher_id);
        let package_id = slug_segment(&req.package_id);
        let version = req.version.trim().to_string();
        if authority_owner.is_empty()
            || authority_project.is_empty()
            || publisher_owner.is_empty()
            || publisher_id.is_empty()
            || package_id.is_empty()
            || version.is_empty()
        {
            return Err(PlatformError::new(
                "MARKETPLACE_REMOTE_INVALID",
                "authority, publisher, package id, and version must not be empty",
            ));
        }
        validate_marketplace_version(&version)?;
        let Some(publisher) = self.marketplace_data.get_marketplace_publisher(
            MARKETPLACE_SERVICE_SCOPE_OWNER,
            MARKETPLACE_SERVICE_SCOPE_PROJECT,
            &publisher_id,
        )?
        else {
            return Err(PlatformError::new(
                "MARKETPLACE_PUBLISHER_MISSING",
                "publisher not found",
            ));
        };
        if !publisher.enabled {
            return Err(PlatformError::new(
                "MARKETPLACE_PUBLISHER_DISABLED",
                "publisher is disabled",
            ));
        }
        if !publisher.can_publish {
            return Err(PlatformError::new(
                "MARKETPLACE_PUBLISHER_FORBIDDEN",
                "publisher does not have publish permission",
            ));
        }
        let source_owner = slug_segment(&req.source_owner);
        let source_project = slug_segment(&req.source_project);
        let mut artifact: MarketplaceArtifact = serde_json::from_value(req.artifact.clone())
            .map_err(|err| PlatformError::new("MARKETPLACE_REMOTE_INVALID", err.to_string()))?;
        if artifact.files.is_empty() {
            return Err(PlatformError::new(
                "MARKETPLACE_REMOTE_INVALID",
                "artifact must contain at least one file",
            ));
        }
        sanitize_marketplace_export_entries(&mut artifact.files)?;
        let artifact_value = serde_json::to_value(&artifact)
            .map_err(|err| PlatformError::new("MARKETPLACE_REMOTE_INVALID", err.to_string()))?;
        let artifact_rel = format!(
            "services/{}/packages/{}/versions/{}/artifact.json",
            DEFAULT_MARKETPLACE_SERVICE_INSTANCE_ID, package_id, version
        );
        let artifact_abs = self.data_root.join(&artifact_rel);
        if let Some(parent) = artifact_abs.parent() {
            fs::create_dir_all(parent)?;
        }
        let artifact_bytes = serde_json::to_vec_pretty(&artifact)
            .map_err(|err| PlatformError::new("MARKETPLACE_REMOTE_INVALID", err.to_string()))?;
        let now = now_ts();
        let authority = self.ensure_service_authority()?;
        let existing_package = self
            .marketplace_data
            .get_marketplace_asset_package(&package_id)?;
        self.enforce_publisher_package_quota(
            &publisher,
            &package_id,
            existing_package.as_ref(),
            artifact_bytes.len(),
        )?;
        fs::write(&artifact_abs, &artifact_bytes)?;
        let package = MarketplaceAssetPackage {
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
            asset_kind: artifact.asset_kind.clone(),
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
            visibility: normalize_visibility(&req.visibility),
            tags: req.tags.clone(),
            created_at: existing_package.map(|item| item.created_at).unwrap_or(now),
            updated_at: now,
        };
        let version_row = MarketplaceAssetVersion {
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
            created_at: now,
        };
        self.marketplace_data
            .put_marketplace_asset_package(&package)?;
        self.marketplace_data
            .put_marketplace_asset_version(&version_row)?;
        Ok((package, version_row))
    }

    pub async fn fetch_remote_pack_rows(
        &self,
        http_client: &reqwest::Client,
        owner: &str,
        project: &str,
    ) -> Result<Vec<MarketplaceRemotePackRow>, PlatformError> {
        let repos = self.list_repositories(owner, project)?;
        let mut out = Vec::new();
        for repo in repos.into_iter().filter(|item| item.enabled) {
            let url = remote_marketplace_url(&repo, "remote/assets");
            validate_remote_marketplace_url(&url)?;
            let mut req = http_client.get(url);
            if !repo.read_token.trim().is_empty() {
                req = req.bearer_auth(repo.read_token.trim());
            }
            let response = req
                .send()
                .await
                .map_err(|err| PlatformError::new("MARKETPLACE_REMOTE_FETCH", err.to_string()))?;
            if !response.status().is_success() {
                continue;
            }
            let payload: RemoteMarketplaceListResponse = response
                .json()
                .await
                .map_err(|err| PlatformError::new("MARKETPLACE_REMOTE_FETCH", err.to_string()))?;
            out.extend(
                payload
                    .items
                    .into_iter()
                    .map(|item| MarketplaceRemotePackRow {
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

    pub async fn fetch_platform_remote_app_rows(
        &self,
        http_client: &reqwest::Client,
        owner: &str,
    ) -> Result<Vec<MarketplaceRemotePackRow>, PlatformError> {
        let repos = self.list_platform_repositories(owner)?;
        self.fetch_platform_remote_app_rows_from_repositories(http_client, repos)
            .await
    }

    pub async fn fetch_platform_remote_app_rows_from_repositories(
        &self,
        http_client: &reqwest::Client,
        repos: Vec<PlatformMarketplaceRepository>,
    ) -> Result<Vec<MarketplaceRemotePackRow>, PlatformError> {
        let mut out = Vec::new();
        for repo in repos.into_iter().filter(|item| item.enabled) {
            let url = remote_marketplace_url_for_platform(&repo, "remote/assets");
            validate_remote_marketplace_url(&url)?;
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
            let payload: RemoteMarketplaceListResponse = match response.json().await {
                Ok(payload) => payload,
                Err(_) => continue,
            };
            out.extend(
                payload
                    .items
                    .into_iter()
                    .filter(|item| item.asset_kind == "project_bundle")
                    .map(|item| MarketplaceRemotePackRow {
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
    ) -> Result<MarketplaceInstallResult, PlatformError> {
        let repo = self
            .list_repositories(target_owner, target_project)?
            .into_iter()
            .find(|item| item.repository_id == repository_id)
            .ok_or_else(|| {
                PlatformError::new("MARKETPLACE_REPOSITORY_MISSING", "repository not found")
            })?;
        let url = remote_marketplace_url(
            &repo,
            &format!("remote/assets/{}/{}/artifact", package_id, version),
        );
        validate_remote_marketplace_url(&url)?;
        let mut req = http_client.get(url);
        if !repo.read_token.trim().is_empty() {
            req = req.bearer_auth(repo.read_token.trim());
        }
        let response = req
            .send()
            .await
            .map_err(|err| PlatformError::new("MARKETPLACE_REMOTE_FETCH", err.to_string()))?;
        if !response.status().is_success() {
            return Err(PlatformError::new(
                "MARKETPLACE_REMOTE_FETCH",
                format!("remote fetch failed with {}", response.status()),
            ));
        }
        let payload: RemoteMarketplaceArtifactResponse = response
            .json()
            .await
            .map_err(|err| PlatformError::new("MARKETPLACE_REMOTE_FETCH", err.to_string()))?;
        verify_remote_artifact_hash(&payload)?;
        let artifact: MarketplaceArtifact = serde_json::from_value(payload.artifact)
            .map_err(|err| PlatformError::new("MARKETPLACE_REMOTE_INVALID", err.to_string()))?;
        self.install_artifact_payload(
            slug_segment(target_owner),
            slug_segment(target_project),
            package_id,
            version,
            artifact,
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
            .ok_or_else(|| {
                PlatformError::new("MARKETPLACE_REPOSITORY_MISSING", "repository not found")
            })?;
        let url = remote_marketplace_url_for_platform(
            &repo,
            &format!("remote/assets/{}/{}/artifact", package_id, version),
        );
        validate_remote_marketplace_url(&url)?;
        let mut req = http_client.get(url);
        if !repo.read_token.trim().is_empty() {
            req = req.bearer_auth(repo.read_token.trim());
        }
        let response = req
            .send()
            .await
            .map_err(|err| PlatformError::new("MARKETPLACE_REMOTE_FETCH", err.to_string()))?;
        if !response.status().is_success() {
            return Err(PlatformError::new(
                "MARKETPLACE_REMOTE_FETCH",
                format!("remote fetch failed with {}", response.status()),
            ));
        }
        let payload: RemoteMarketplaceArtifactResponse = response
            .json()
            .await
            .map_err(|err| PlatformError::new("MARKETPLACE_REMOTE_FETCH", err.to_string()))?;
        verify_remote_artifact_hash(&payload)?;
        let artifact: MarketplaceArtifact = serde_json::from_value(payload.artifact.clone())
            .map_err(|err| PlatformError::new("MARKETPLACE_REMOTE_INVALID", err.to_string()))?;
        if artifact.asset_kind != "project_bundle" {
            return Err(PlatformError::new(
                "MARKETPLACE_REMOTE_INVALID",
                "platform marketplace install only supports project bundles",
            ));
        }
        let base_project = slug_segment(package_id);
        if base_project.is_empty() {
            return Err(PlatformError::new(
                "MARKETPLACE_REMOTE_INVALID",
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
        clear_repo_worktree_preserving_git(&layout.repo_dir)?;
        for entry in &artifact.files {
            let dest_abs = sanitize_install_repo_path(&layout, &entry.rel_path)?;
            if let Some(parent) = dest_abs.parent() {
                fs::create_dir_all(parent)?;
            }
            write_entry_content(&dest_abs, entry)?;
        }
        Ok((target_owner, project))
    }

    fn preview_pipeline(
        &self,
        owner: &str,
        project: &str,
        layout: &ProjectFileLayout,
        source_ref: &str,
    ) -> Result<MarketplaceExportPreview, PlatformError> {
        let Some(meta) = self
            .projects
            .get_pipeline_meta_by_file_id(owner, project, source_ref)?
        else {
            return Err(PlatformError::new(
                "MARKETPLACE_PUBLISH_MISSING",
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
            .map_err(|err| PlatformError::new("MARKETPLACE_PREVIEW", err.to_string()))?;
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
            "pipeline_bundle".to_string(),
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
    ) -> Result<MarketplaceExportPreview, PlatformError> {
        let listing = self.projects.list_template_workspace(owner, project)?;
        let selected = listing
            .items
            .into_iter()
            .find(|item| item.kind != "folder" && item.rel_path == source_ref)
            .ok_or_else(|| {
                PlatformError::new(
                    "MARKETPLACE_PUBLISH_MISSING",
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
            "template_bundle".to_string(),
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
    ) -> Result<MarketplaceExportPreview, PlatformError> {
        let mut entries = collect_tree_entries(layout, source_ref)?;
        entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        let name = Path::new(source_ref)
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or(source_ref)
            .to_string();
        Ok(build_preview(
            "folder_bundle".to_string(),
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
    ) -> Result<MarketplaceExportPreview, PlatformError> {
        let mut entries = collect_tree_entries(layout, ".")?;
        entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        Ok(build_preview(
            "project_bundle".to_string(),
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
        entries: &mut Vec<MarketplaceExportEntry>,
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
            Some(entry.content.clone())
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

impl MarketplaceService {
    fn require_enabled(&self) -> Result<PlatformServiceInstance, PlatformError> {
        match self.get_default_service_instance()? {
            Some(service) if service.enabled => Ok(service),
            _ => Err(PlatformError::new(
                "MARKETPLACE_SERVICE_DISABLED",
                "marketplace service is not enabled",
            )),
        }
    }

    fn ensure_service_authority(&self) -> Result<MarketplaceAuthority, PlatformError> {
        if let Some(authority) = self.marketplace_data.get_marketplace_authority(
            MARKETPLACE_SERVICE_SCOPE_OWNER,
            MARKETPLACE_SERVICE_SCOPE_PROJECT,
        )? {
            return Ok(authority);
        }
        let service = self.require_enabled()?;
        let now = now_ts();
        let authority = MarketplaceAuthority {
            authority_id: DEFAULT_MARKETPLACE_SERVICE_INSTANCE_ID.to_string(),
            host_project_id: service.host_office_id,
            owner: MARKETPLACE_SERVICE_SCOPE_OWNER.to_string(),
            project: MARKETPLACE_SERVICE_SCOPE_PROJECT.to_string(),
            enabled: true,
            public_base_url: service.public_base_url,
            created_at: now,
            updated_at: now,
        };
        self.marketplace_data
            .put_marketplace_authority(&authority)?;
        Ok(authority)
    }

    fn marketplace_artifact_path(&self, rel_path: &str) -> Result<PathBuf, PlatformError> {
        let rel_path = rel_path.trim().replace('\\', "/");
        if rel_path.is_empty() || rel_path.contains('\0') {
            return Err(PlatformError::new(
                "MARKETPLACE_ARTIFACT_PATH_INVALID",
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
                "MARKETPLACE_ARTIFACT_PATH_INVALID",
                "artifact path must be a contained relative path",
            ));
        }
        let service_rel_prefix = format!("services/{DEFAULT_MARKETPLACE_SERVICE_INSTANCE_ID}/");
        if !rel_path.starts_with(&service_rel_prefix) {
            return Err(PlatformError::new(
                "MARKETPLACE_ARTIFACT_PATH_INVALID",
                "artifact path must live under the marketplace service root",
            ));
        }
        let service_root = self
            .data_root
            .join("services")
            .join(DEFAULT_MARKETPLACE_SERVICE_INSTANCE_ID);
        let abs = self.data_root.join(rel);
        let service_root = fs::canonicalize(&service_root)?;
        let abs = fs::canonicalize(&abs)?;
        if !abs.starts_with(&service_root) {
            return Err(PlatformError::new(
                "MARKETPLACE_ARTIFACT_PATH_INVALID",
                "artifact path escapes marketplace service root",
            ));
        }
        Ok(abs)
    }

    fn enforce_publisher_package_quota(
        &self,
        publisher: &MarketplacePublisher,
        package_id: &str,
        existing_package: Option<&MarketplaceAssetPackage>,
        artifact_bytes: usize,
    ) -> Result<(), PlatformError> {
        let max_package_bytes = normalize_limit(
            publisher.max_package_bytes,
            DEFAULT_PUBLISHER_MAX_PACKAGE_BYTES,
        );
        let max_packages = normalize_limit(publisher.max_packages, DEFAULT_PUBLISHER_MAX_PACKAGES);
        if artifact_bytes as i64 > max_package_bytes {
            return Err(PlatformError::new(
                "MARKETPLACE_PUBLISHER_QUOTA_EXCEEDED",
                "publisher artifact byte limit exceeded",
            ));
        }
        if let Some(existing) = existing_package {
            if existing.publisher_id != publisher.publisher_id
                || existing.publisher_pk != publisher.publisher_pk
            {
                return Err(PlatformError::new(
                    "MARKETPLACE_PACKAGE_FORBIDDEN",
                    "package id already belongs to another publisher",
                ));
            }
            return Ok(());
        }
        let package_count = self
            .marketplace_data
            .list_marketplace_asset_packages()?
            .into_iter()
            .filter(|item| {
                item.publisher_id == publisher.publisher_id
                    && item.publisher_pk == publisher.publisher_pk
            })
            .count() as i64;
        if package_count >= max_packages {
            return Err(PlatformError::new(
                "MARKETPLACE_PUBLISHER_QUOTA_EXCEEDED",
                "publisher package count limit exceeded",
            ));
        }
        if package_id.is_empty() {
            return Err(PlatformError::new(
                "MARKETPLACE_PUBLISH_INVALID",
                "package id must not be empty",
            ));
        }
        Ok(())
    }
}

fn remote_marketplace_api_base(base_url: &str) -> String {
    base_url.trim().trim_end_matches('/').to_string()
}

fn is_direct_marketplace_base(base_url: &str) -> bool {
    let base = remote_marketplace_api_base(base_url).to_lowercase();
    is_legacy_project_marketplace_base(&base) || is_ownerless_marketplace_base(&base)
}

fn is_legacy_project_marketplace_base(base_url: &str) -> bool {
    let base = remote_marketplace_api_base(base_url).to_lowercase();
    base.contains("/api/projects/") && base.ends_with("/marketplace")
}

fn is_ownerless_marketplace_base(base_url: &str) -> bool {
    let base = remote_marketplace_api_base(base_url).to_lowercase();
    base.ends_with("/api") || base.ends_with("/api/marketplace")
}

fn ownerless_marketplace_url(api_base: &str, suffix: &str) -> String {
    if api_base.to_lowercase().ends_with("/api/marketplace") {
        format!("{}/{}", api_base, suffix)
    } else {
        format!("{}/marketplace/{}", api_base, suffix)
    }
}

fn remote_marketplace_url(repo: &ProjectMarketplaceRepository, suffix: &str) -> String {
    let api_base = remote_marketplace_api_base(&repo.base_url);
    let suffix = suffix.trim_start_matches('/');
    if is_legacy_project_marketplace_base(&api_base) {
        format!("{}/{}", api_base, suffix)
    } else if is_ownerless_marketplace_base(&api_base) {
        ownerless_marketplace_url(&api_base, suffix)
    } else {
        format!(
            "{}/projects/{}/{}/marketplace/{}",
            api_base, repo.remote_owner, repo.remote_project, suffix
        )
    }
}

fn remote_marketplace_url_for_platform(
    repo: &PlatformMarketplaceRepository,
    suffix: &str,
) -> String {
    let api_base = remote_marketplace_api_base(&repo.base_url);
    let suffix = suffix.trim_start_matches('/');
    if is_legacy_project_marketplace_base(&api_base) {
        format!("{}/{}", api_base, suffix)
    } else if is_ownerless_marketplace_base(&api_base) {
        ownerless_marketplace_url(&api_base, suffix)
    } else {
        format!(
            "{}/projects/{}/{}/marketplace/{}",
            api_base, repo.remote_owner, repo.remote_project, suffix
        )
    }
}

fn validate_remote_marketplace_url(url: &str) -> Result<(), PlatformError> {
    if marketplace_localhost_remote_allowed(url) {
        return Ok(());
    }
    crate::pipeline::security::validate_outbound_http_url(url, "marketplace.remote")
        .map_err(|err| PlatformError::new(err.code, err.message))
}

fn marketplace_localhost_remote_allowed(url: &str) -> bool {
    // Test/dev escape hatch for two local Zebflow instances. Production must
    // leave this unset so marketplace remotes follow the normal egress policy.
    if std::env::var("ZEBFLOW_MARKETPLACE_ALLOW_LOCALHOST_REMOTE")
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
    let entries = fs::read_dir(repo_dir)
        .map_err(|err| PlatformError::new("MARKETPLACE_INSTALL", err.to_string()))?;
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
                .map_err(|err| PlatformError::new("MARKETPLACE_INSTALL", err.to_string()))?;
        } else {
            fs::remove_file(&path)
                .map_err(|err| PlatformError::new("MARKETPLACE_INSTALL", err.to_string()))?;
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
            "MARKETPLACE_INSTALL",
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
    entries: Vec<MarketplaceExportEntry>,
    warnings: Vec<String>,
) -> MarketplaceExportPreview {
    let total_bytes = entries.iter().map(|item| item.size_bytes).sum();
    let total_files = entries.len();
    MarketplaceExportPreview {
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
        .filter(|item| {
            matches!(
                item.as_str(),
                "marketplace:read" | "marketplace:publish" | "marketplace:manage"
            )
        })
        .collect::<Vec<_>>();
    out.sort();
    out.dedup();
    out
}

fn validate_publisher_scopes(
    publisher: &MarketplacePublisher,
    scopes: &[String],
) -> Result<(), PlatformError> {
    for scope in scopes {
        let allowed = match scope.as_str() {
            "marketplace:read" => publisher.can_read,
            "marketplace:publish" => publisher.can_publish,
            "marketplace:manage" => publisher.can_manage,
            _ => false,
        };
        if !allowed {
            return Err(PlatformError::new(
                "MARKETPLACE_PUBLISHER_SCOPE_DENIED",
                "publisher is not allowed to create a token with the requested scope",
            ));
        }
    }
    Ok(())
}

fn apply_scope_flags(mut token: MarketplaceToken) -> MarketplaceToken {
    token.scope_read = token.scopes.iter().any(|scope| scope == "marketplace:read");
    token.scope_publish = token
        .scopes
        .iter()
        .any(|scope| scope == "marketplace:publish");
    token.scope_manage = token
        .scopes
        .iter()
        .any(|scope| scope == "marketplace:manage");
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

fn normalize_platform_source_visibility(input: &str) -> String {
    match input.trim().to_ascii_lowercase().as_str() {
        "private" => "private".to_string(),
        _ => "public".to_string(),
    }
}

fn normalize_limit(value: i64, default_value: i64) -> i64 {
    if value <= 0 { default_value } else { value }
}

fn validate_marketplace_version(version: &str) -> Result<(), PlatformError> {
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
            "MARKETPLACE_PUBLISH_INVALID",
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
) -> Result<MarketplaceExportEntry, PlatformError> {
    let rel_path = normalize_repo_rel(rel_path);
    let abs = layout.repo_dir.join(&rel_path);
    if !abs.starts_with(&layout.repo_dir) || !abs.is_file() {
        return Err(PlatformError::new(
            "MARKETPLACE_ENTRY_MISSING",
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
    Ok(MarketplaceExportEntry {
        rel_path,
        kind: file_kind_from_path(&abs),
        size_bytes,
        reason,
        encoding,
        content,
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
) -> Result<Vec<MarketplaceExportEntry>, PlatformError> {
    let rel_root = normalize_repo_rel(rel_root);
    let base = if rel_root.is_empty() || rel_root == "." {
        layout.repo_dir.clone()
    } else {
        layout.repo_dir.join(&rel_root)
    };
    if !base.starts_with(&layout.repo_dir) || !base.exists() {
        return Err(PlatformError::new(
            "MARKETPLACE_SOURCE_INVALID",
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
        let rel = current.strip_prefix(root).map_err(|_| {
            PlatformError::new("MARKETPLACE_SOURCE_INVALID", "path escaped repo root")
        })?;
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
            let rel = path.strip_prefix(root).map_err(|_| {
                PlatformError::new("MARKETPLACE_SOURCE_INVALID", "path escaped repo root")
            })?;
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

fn install_root_for(package_id: &str, asset_kind: &str) -> String {
    if matches!(asset_kind, "pipeline_bundle" | "template_bundle") {
        format!("pipelines/marketplace/{package_id}")
    } else {
        format!("marketplace/{package_id}")
    }
}

fn install_rel_path(package_id: &str, rel_path: &str) -> String {
    let rel = normalize_repo_rel(rel_path);
    if let Some(rest) = rel.strip_prefix("pipelines/") {
        format!("pipelines/marketplace/{package_id}/{rest}")
    } else {
        format!("marketplace/{package_id}/{rel}")
    }
}

fn write_entry_content(
    dest_abs: &Path,
    entry: &MarketplaceExportEntry,
) -> Result<(), PlatformError> {
    let bytes = if entry.encoding == "base64" {
        base64::engine::general_purpose::STANDARD
            .decode(&entry.content)
            .map_err(|err| PlatformError::new("MARKETPLACE_INSTALL", err.to_string()))?
    } else {
        entry.content.as_bytes().to_vec()
    };
    fs::write(dest_abs, bytes)?;
    Ok(())
}

fn verify_remote_artifact_hash(
    payload: &RemoteMarketplaceArtifactResponse,
) -> Result<(), PlatformError> {
    if payload.artifact_size_bytes > MAX_REMOTE_MARKETPLACE_ARTIFACT_BYTES {
        return Err(PlatformError::new(
            "MARKETPLACE_ARTIFACT_TOO_LARGE",
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
            "MARKETPLACE_REMOTE_HASH_MISSING",
            "remote artifact response is missing artifact hash",
        ));
    }
    let artifact: MarketplaceArtifact = serde_json::from_value(payload.artifact.clone())
        .map_err(|err| PlatformError::new("MARKETPLACE_REMOTE_INVALID", err.to_string()))?;
    let bytes = serde_json::to_vec_pretty(&artifact)
        .map_err(|err| PlatformError::new("MARKETPLACE_REMOTE_INVALID", err.to_string()))?;
    let actual = sha256_hex(&bytes);
    if actual != expected {
        return Err(PlatformError::new(
            "MARKETPLACE_REMOTE_HASH_MISMATCH",
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

fn sanitize_marketplace_export_entries(
    entries: &mut [MarketplaceExportEntry],
) -> Result<(), PlatformError> {
    for entry in entries.iter_mut() {
        if normalize_repo_rel(&entry.rel_path) != "zebflow.json" || entry.encoding == "base64" {
            continue;
        }
        let Ok(mut cfg) = serde_json::from_str::<ZebflowJson>(&entry.content) else {
            continue;
        };
        cfg.distribution.marketplace.producer_enabled = false;
        let content = serde_json::to_string_pretty(&cfg)
            .map_err(|err| PlatformError::new("MARKETPLACE_PUBLISH", err.to_string()))?;
        entry.size_bytes = content.len();
        entry.content = content;
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
    let Ok(value) = serde_json::from_str::<Value>(source) else {
        return (fallback_title, "webhook".to_string());
    };
    let trigger_kind = value
        .get("nodes")
        .and_then(Value::as_array)
        .and_then(|nodes| {
            nodes.iter().find_map(|node| {
                node.get("kind")
                    .and_then(Value::as_str)
                    .filter(|kind| kind.starts_with("n.trigger."))
                    .map(|kind| kind.trim_start_matches("n.trigger.").to_string())
            })
        })
        .unwrap_or_else(|| "webhook".to_string());
    (fallback_title, trigger_kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_scopes_only_keeps_known_marketplace_scopes() {
        let scopes = normalize_scopes(&[
            "marketplace:publish".to_string(),
            " marketplace:read ".to_string(),
            "marketplace:publish".to_string(),
            "custom:other".to_string(),
        ]);
        assert_eq!(
            scopes,
            vec![
                "marketplace:publish".to_string(),
                "marketplace:read".to_string()
            ]
        );
    }

    #[test]
    fn apply_scope_flags_sets_explicit_permissions() {
        let token = apply_scope_flags(MarketplaceToken {
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
            scopes: vec![
                "marketplace:read".to_string(),
                "marketplace:publish".to_string(),
            ],
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
        assert!(token.grants_scope("marketplace:read"));
        assert!(token.grants_scope("marketplace:publish"));
        assert!(!token.grants_scope("marketplace:manage"));
    }

    #[test]
    fn publisher_scopes_respect_current_permissions() {
        let publisher = MarketplacePublisher {
            authority_id: DEFAULT_MARKETPLACE_SERVICE_INSTANCE_ID.to_string(),
            publisher_pk: "mpub_1".to_string(),
            owner: MARKETPLACE_SERVICE_SCOPE_OWNER.to_string(),
            project: MARKETPLACE_SERVICE_SCOPE_PROJECT.to_string(),
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
        assert!(validate_publisher_scopes(&publisher, &["marketplace:read".to_string()]).is_ok());
        let err = validate_publisher_scopes(&publisher, &["marketplace:publish".to_string()])
            .expect_err("publish scope should be denied");
        assert_eq!(err.code, "MARKETPLACE_PUBLISHER_SCOPE_DENIED");
    }

    #[test]
    fn marketplace_versions_are_single_safe_path_segments() {
        assert!(validate_marketplace_version("1.0.0").is_ok());
        assert!(validate_marketplace_version("1.0.0-beta_1").is_ok());
        assert!(validate_marketplace_version("../secret").is_err());
        assert!(validate_marketplace_version("1/2").is_err());
        assert!(validate_marketplace_version("release candidate").is_err());
    }
}

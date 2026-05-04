//! Platform domain models and configuration.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::infra::cluster::config::ClusterSettings;
use crate::infra::cluster::registry::WorkerHeartbeat;
use crate::infra::execution::placement::{ProjectRuntimePlacement, ProjectRuntimeProfile};
use crate::infra::execution::runner::RunnerCapabilities;
use crate::infra::execution::sync::{ProjectBootstrapPlan, ProjectRuntimeBundle};

/// Data adapter selection.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DataAdapterKind {
    /// SQLite-backed platform catalog (WAL mode, bundled SQLite 3.47).
    #[default]
    Sqlite,
    /// Placeholder adapter.
    DynamoDb,
    /// Placeholder adapter.
    Firebase,
}

/// File adapter selection.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FileAdapterKind {
    /// Local filesystem tree. Git-sync friendly.
    #[default]
    Filesystem,
}

/// Platform bootstrap/runtime config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    /// Root data directory where platform metadata + project files are stored.
    pub data_root: PathBuf,
    /// Selected metadata adapter.
    pub data_adapter: DataAdapterKind,
    /// Selected file adapter.
    pub file_adapter: FileAdapterKind,
    /// Default superadmin username created on first bootstrap.
    pub default_owner: String,
    /// Initial superadmin password created on first bootstrap.
    ///
    /// This should be supplied explicitly by the host (for example from
    /// `ZEBFLOW_PLATFORM_DEFAULT_PASSWORD`) rather than relying on a baked-in
    /// repository default.
    pub default_password: String,
    /// Default project slug created on first bootstrap.
    pub default_project: String,
    /// Unix timestamp seconds; platform-issued secrets created before this time
    /// are invalidated on startup/lookup.
    #[serde(default)]
    pub secret_rotation_epoch: i64,
    /// Cluster/controller-office role and connectivity settings.
    pub cluster: ClusterSettings,
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            data_root: PathBuf::from(".zebflow-platform-data"),
            data_adapter: DataAdapterKind::Sqlite,
            file_adapter: FileAdapterKind::Filesystem,
            default_owner: "superadmin".to_string(),
            default_password: String::new(),
            default_project: "default".to_string(),
            secret_rotation_epoch: 0,
            cluster: ClusterSettings::default(),
        }
    }
}

/// User profile stored by platform metadata adapter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformUser {
    /// Stable internal user id.
    #[serde(default)]
    pub user_id: String,
    /// User identifier.
    pub owner: String,
    /// Role string (`superadmin`, `member`, ...).
    pub role: String,
    /// Display name for git commits (e.g. "Alice Smith").
    #[serde(default)]
    pub git_name: String,
    /// Email for git commits (e.g. "alice@example.com").
    #[serde(default)]
    pub git_email: String,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
}

/// User record with auth secret, used internally by auth service.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredUser {
    /// Public profile fields.
    pub profile: PlatformUser,
    /// Local platform auth material.
    pub auth: PlatformUserLocalAuth,
}

/// Local auth record for one user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformUserLocalAuth {
    /// Stable internal user id.
    pub user_id: String,
    /// Stored password hash.
    pub password_hash: String,
    /// Password hash algorithm identifier.
    pub password_alg: String,
    /// Unix timestamp seconds.
    pub password_updated_at: i64,
}

/// Project profile stored by metadata adapter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformProject {
    /// Stable internal project id.
    #[serde(default)]
    pub project_id: String,
    /// Stable internal owner user id.
    #[serde(default)]
    pub owner_user_id: String,
    /// Owner identifier.
    pub owner: String,
    /// Project slug.
    pub project: String,
    /// Display title.
    pub title: String,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
}

/// Stored project credential record used by runtime nodes and management APIs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectCredential {
    /// Owner identifier.
    pub owner: String,
    /// Project slug.
    pub project: String,
    /// Stable credential id.
    pub credential_id: String,
    /// Display title.
    pub title: String,
    /// Driver/kind (`postgres`, `openai`, ...).
    pub kind: String,
    /// Secret payload owned by the project.
    pub secret: serde_json::Value,
    /// Optional freeform notes.
    pub notes: String,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
}

/// One project credential summary row safe to return in list responses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectCredentialListItem {
    /// Stable credential id.
    pub credential_id: String,
    /// Display title.
    pub title: String,
    /// Driver/kind (`postgres`, `openai`, ...).
    pub kind: String,
    /// Whether the credential currently stores a secret payload.
    pub has_secret: bool,
    /// Optional freeform notes.
    pub notes: String,
    /// Roles registered in this credential (JWT only). Non-sensitive — safe to expose in list responses.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub auth_roles: Vec<String>,
    /// Safe variable definitions exposed by `secure_request` credentials.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secure_request_vars: Vec<SecureRequestVariableDefinition>,
    /// OAuth2 token status. Derived at list-time, not stored.
    /// Values: "" (non-oauth2), "not_configured", "authorized", "expired".
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub oauth2_status: String,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
}

/// Safe variable definition exposed from a `secure_request` credential.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecureRequestVariableDefinition {
    /// Stable variable name used in the request template.
    pub name: String,
    /// Human-readable label displayed in UI surfaces.
    pub label: String,
    /// Value kind hint such as `string`, `secret`, or `header`.
    #[serde(default)]
    pub value_type: String,
    /// Whether this variable is required at request time.
    #[serde(default)]
    pub required: bool,
    /// Optional default expression evaluated client-side or by later request tooling.
    #[serde(default)]
    pub default_expr: String,
    /// Optional human-readable explanation.
    #[serde(default)]
    pub description: String,
}

/// Stored project DB connection record used by DB suite and runtime nodes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectDbConnection {
    /// Owner identifier.
    pub owner: String,
    /// Project slug.
    pub project: String,
    /// Stable connection id.
    pub connection_id: String,
    /// Stable route slug.
    pub connection_slug: String,
    /// Display label.
    pub connection_label: String,
    /// Database kind (`sqlite`, `postgresql`, ...).
    pub database_kind: String,
    /// Optional linked credential id.
    pub credential_id: Option<String>,
    /// Optional kind-specific config payload.
    pub config: serde_json::Value,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
}

/// One project DB connection summary row safe to return in list responses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectDbConnectionListItem {
    /// Stable connection id.
    pub connection_id: String,
    /// Stable route slug.
    pub connection_slug: String,
    /// Display label.
    pub connection_label: String,
    /// Database kind (`sqlite`, `postgresql`, ...).
    pub database_kind: String,
    /// Optional linked credential id.
    pub credential_id: Option<String>,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
}

/// One project-scoped marketplace repository source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectMarketplaceRepository {
    /// Owner identifier.
    pub owner: String,
    /// Project slug.
    pub project: String,
    /// Stable repository id.
    pub repository_id: String,
    /// Display title.
    pub title: String,
    /// Base URL of the remote marketplace, e.g. `http://127.0.0.1:10612`.
    pub base_url: String,
    /// Remote owner slug on the target marketplace.
    pub remote_owner: String,
    /// Remote project slug on the target marketplace.
    pub remote_project: String,
    /// Optional bearer token for read access.
    pub read_token: String,
    /// Whether this repository should be queried in Packs.
    pub enabled: bool,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
}

/// One platform-scoped marketplace repository source used from Home.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformMarketplaceRepository {
    /// Stable internal source id.
    #[serde(default)]
    pub source_id: String,
    /// Stable internal owner user id.
    #[serde(default)]
    pub owner_user_id: String,
    /// Owner identifier for the local platform user who manages this source.
    pub owner: String,
    /// Stable repository id.
    pub repository_id: String,
    /// Display title.
    pub title: String,
    /// Base URL of the remote marketplace, e.g. `http://127.0.0.1:10612/api/projects/superadmin/default/marketplace`.
    pub base_url: String,
    /// Remote owner slug on the target marketplace when using a proxy base.
    pub remote_owner: String,
    /// Remote project slug on the target marketplace when using a proxy base.
    pub remote_project: String,
    /// Optional bearer token for read access.
    pub read_token: String,
    /// Source visibility in Platform Home marketplace explorer: public or private.
    #[serde(default)]
    pub visibility: String,
    /// Whether this repository should be queried in Home marketplace.
    pub enabled: bool,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
}

/// One explicit marketplace authority row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketplaceAuthority {
    /// Stable authority id.
    pub authority_id: String,
    /// Host project id.
    pub host_project_id: String,
    /// Marketplace authority owner slug.
    pub owner: String,
    /// Marketplace authority project slug.
    pub project: String,
    /// Whether producer mode is currently enabled.
    pub enabled: bool,
    /// Public marketplace API base URL when exposed.
    pub public_base_url: String,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
}

/// One durable office row in the platform control plane.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformOffice {
    /// Stable office id.
    pub office_id: String,
    /// Human-readable / URL-safe office slug.
    pub office_slug: String,
    /// Display label.
    pub label: String,
    /// Office kind (`controller`, `office`, `standalone`).
    pub office_kind: String,
    /// Base URL advertised for control/runtime traffic.
    pub base_url: String,
    /// Current status summary.
    pub status: String,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
}

/// One platform-level service instance hosted by an office.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformServiceInstance {
    /// Stable service instance id, e.g. `marketplace-default`.
    pub service_instance_id: String,
    /// Service kind, e.g. `marketplace`.
    pub service_kind: String,
    /// Display label.
    pub display_label: String,
    /// Office that serves this service runtime.
    pub host_office_id: String,
    /// Office that owns this service's operational state.
    pub state_office_id: String,
    /// Public API/base URL for this service, when exposed.
    pub public_base_url: String,
    /// Whether the service is enabled.
    pub enabled: bool,
    /// Current service status summary.
    pub status: String,
    /// Monotonic placement/config generation.
    pub placement_generation: i64,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
}

/// One registered runtime node under an office.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformOfficeNode {
    /// Stable office id.
    pub office_id: String,
    /// Stable node id.
    pub node_id: String,
    /// Display label.
    pub label: String,
    /// Advertised base URL.
    pub base_url: String,
    /// Current status summary.
    pub status: String,
    /// Declared capabilities.
    #[serde(default)]
    pub capabilities: RunnerCapabilities,
    /// Unix timestamp seconds.
    pub registered_at: i64,
    /// Unix timestamp seconds.
    pub last_heartbeat_at: i64,
}

/// One stable publisher identity inside one marketplace service authority.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketplacePublisher {
    /// Stable authority id.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub authority_id: String,
    /// Stable publisher row id.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub publisher_pk: String,
    /// Internal marketplace scope owner.
    pub owner: String,
    /// Internal marketplace scope project.
    pub project: String,
    /// Stable immutable public publisher identity.
    pub publisher_id: String,
    /// Human-facing publisher name.
    pub display_name: String,
    /// Stable public publisher URL.
    pub publisher_url: String,
    /// Publisher contact email.
    pub email: String,
    /// Optional human-readable description.
    pub description: String,
    /// Optional icon URL.
    pub icon_url: String,
    /// Optional website URL.
    pub website_url: String,
    /// Whether the publisher is active.
    pub enabled: bool,
    /// Whether this publisher may read private marketplace artifacts.
    #[serde(default = "default_true")]
    pub can_read: bool,
    /// Whether this publisher may publish package versions.
    #[serde(default = "default_true")]
    pub can_publish: bool,
    /// Whether this publisher may manage publisher-scoped settings/tokens.
    #[serde(default)]
    pub can_manage: bool,
    /// Maximum number of package ids this publisher may own. Zero means default.
    #[serde(default)]
    pub max_packages: i64,
    /// Maximum raw artifact bytes per package version. Zero means default.
    #[serde(default)]
    pub max_package_bytes: i64,
    /// Maximum media files per package version. Zero means default.
    #[serde(default)]
    pub max_media_files: i64,
    /// Maximum bytes per image media file. Zero means default.
    #[serde(default)]
    pub max_image_bytes: i64,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
}

/// One published marketplace asset package stored by the marketplace service.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketplaceAssetPackage {
    /// Stable package row id.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub package_pk: String,
    /// Stable authority id.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub authority_id: String,
    /// Stable publisher row id.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub publisher_pk: String,
    /// Stable package id, unique inside one marketplace instance.
    pub package_id: String,
    /// Internal marketplace authority owner. Public APIs must not expose this.
    pub authority_owner: String,
    /// Internal marketplace authority project. Public APIs must not expose this.
    pub authority_project: String,
    /// Publisher owner id.
    pub publisher_owner: String,
    /// Stable public publisher identity inside this marketplace authority.
    pub publisher_id: String,
    /// Human-facing publisher name.
    pub publisher_display_name: String,
    /// Stable public publisher URL for attribution and discovery.
    pub publisher_url: String,
    /// Publisher contact email.
    pub publisher_email: String,
    /// Asset kind (`pipeline`, `template_pack`, ...).
    pub asset_kind: String,
    /// Display title.
    pub title: String,
    /// Optional human-readable description.
    pub description: String,
    /// Visibility (`public`, `private`, `unlisted`).
    pub visibility: String,
    /// Search tags.
    pub tags: Vec<String>,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
}

/// One immutable version of a marketplace asset package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketplaceAssetVersion {
    /// Stable package row id.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub package_pk: String,
    /// Stable package id.
    pub package_id: String,
    /// Version string.
    pub version: String,
    /// Internal marketplace authority owner. Public APIs must not expose this.
    pub authority_owner: String,
    /// Internal marketplace authority project. Public APIs must not expose this.
    pub authority_project: String,
    /// Publisher owner id.
    pub publisher_owner: String,
    /// Stable public publisher identity.
    pub publisher_id: String,
    /// Source owner from which the asset was exported.
    pub source_owner: String,
    /// Source project from which the asset was exported.
    pub source_project: String,
    /// Source kind (`pipeline` for v1).
    pub source_kind: String,
    /// Source ref such as `pipelines/foo.zf.json`.
    pub source_ref: String,
    /// Artifact relative path under platform storage.
    pub artifact_rel_path: String,
    /// SHA-256 of artifact bytes.
    pub artifact_sha256: String,
    /// Serialized manifest/payload metadata.
    pub manifest: serde_json::Value,
    /// Unix timestamp seconds.
    pub created_at: i64,
}

/// One marketplace access token used for publish/read/manage operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketplaceToken {
    /// Stable token id.
    pub token_id: String,
    /// Stable authority id.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub authority_id: String,
    /// Stable publisher row id.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub publisher_pk: String,
    /// Owner id.
    pub owner: String,
    /// Client project slug that owns this token/profile.
    pub project: String,
    /// Stable public publisher identity this token acts as.
    pub publisher_id: String,
    /// Human-facing publisher name.
    pub publisher_display_name: String,
    /// Stable public publisher URL.
    pub publisher_url: String,
    /// Publisher contact email.
    pub publisher_email: String,
    /// Display title.
    pub title: String,
    /// SHA-256 hash of the secret, never returned to clients.
    pub secret_hash: String,
    /// Granted scopes.
    pub scopes: Vec<String>,
    /// Whether this token grants marketplace read access.
    #[serde(default, skip_serializing)]
    pub scope_read: bool,
    /// Whether this token grants marketplace publish access.
    #[serde(default, skip_serializing)]
    pub scope_publish: bool,
    /// Whether this token grants marketplace manage access.
    #[serde(default, skip_serializing)]
    pub scope_manage: bool,
    /// Optional unix timestamp seconds.
    pub expires_at: Option<i64>,
    /// Optional unix timestamp seconds.
    pub last_used_at: Option<i64>,
    /// Optional unix timestamp seconds.
    pub revoked_at: Option<i64>,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
}

impl MarketplaceToken {
    /// Returns whether the token grants the required marketplace scope.
    pub fn grants_scope(&self, required_scope: &str) -> bool {
        match required_scope.trim() {
            "marketplace:read" => self.scope_read,
            "marketplace:publish" => self.scope_publish,
            "marketplace:manage" => self.scope_manage,
            _ => false,
        }
    }
}

/// Request to create one marketplace token.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateMarketplaceTokenRequest {
    /// Stable public publisher identity.
    pub publisher_id: String,
    /// Display title.
    pub title: String,
    /// Granted scopes.
    #[serde(default)]
    pub scopes: Vec<String>,
    /// Optional unix timestamp seconds.
    pub expires_at: Option<i64>,
}

/// Project-scoped assistant runtime configuration (used by Zebtune/chat assistant).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectAssistantConfig {
    /// Owner identifier.
    pub owner: String,
    /// Project slug.
    pub project: String,
    /// Credential id for high-level reasoning/planning model.
    pub llm_high_credential_id: Option<String>,
    /// Credential id for general/cheap model.
    pub llm_general_credential_id: Option<String>,
    /// Max execution steps per request.
    pub max_steps: u32,
    /// Max replan attempts.
    pub max_replans: u32,
    /// Whether assistant is enabled for this project.
    pub enabled: bool,
    /// Number of user+assistant pairs to persist as server-side chat history (default 10).
    pub chat_history_pairs: u32,
    /// Unix timestamp seconds.
    pub updated_at: i64,
}

/// Atomic project-scoped permission used by REST, MCP, and internal assistants.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProjectCapability {
    ProjectRead,
    MembersRead,
    MembersWrite,
    CredentialsRead,
    CredentialsWrite,
    TemplatesRead,
    TemplatesWrite,
    TemplatesCreate,
    TemplatesDelete,
    TemplatesMove,
    TemplatesDiagnostics,
    PipelinesRead,
    PipelinesWrite,
    PipelinesCreate,
    PipelinesDelete,
    PipelinesMove,
    PipelinesExecute,
    FilesRead,
    FilesWrite,
    FilesDelete,
    TablesRead,
    TablesWrite,
    LibrariesRead,
    LibrariesInstall,
    LibrariesRemove,
    SettingsRead,
    SettingsWrite,
    McpSessionCreate,
    McpSessionRevoke,
}

impl ProjectCapability {
    /// Stable string id used by policy payloads and UI.
    pub fn key(self) -> &'static str {
        match self {
            Self::ProjectRead => "project.read",
            Self::MembersRead => "members.read",
            Self::MembersWrite => "members.write",
            Self::CredentialsRead => "credentials.read",
            Self::CredentialsWrite => "credentials.write",
            Self::TemplatesRead => "templates.read",
            Self::TemplatesWrite => "templates.write",
            Self::TemplatesCreate => "templates.create",
            Self::TemplatesDelete => "templates.delete",
            Self::TemplatesMove => "templates.move",
            Self::TemplatesDiagnostics => "templates.diagnostics",
            Self::PipelinesRead => "pipelines.read",
            Self::PipelinesWrite => "pipelines.write",
            Self::PipelinesCreate => "pipelines.create",
            Self::PipelinesDelete => "pipelines.delete",
            Self::PipelinesMove => "pipelines.move",
            Self::PipelinesExecute => "pipelines.execute",
            Self::FilesRead => "files.read",
            Self::FilesWrite => "files.write",
            Self::FilesDelete => "files.delete",
            Self::TablesRead => "tables.read",
            Self::TablesWrite => "tables.write",
            Self::LibrariesRead => "libraries.read",
            Self::LibrariesInstall => "libraries.install",
            Self::LibrariesRemove => "libraries.remove",
            Self::SettingsRead => "settings.read",
            Self::SettingsWrite => "settings.write",
            Self::McpSessionCreate => "mcp.session.create",
            Self::McpSessionRevoke => "mcp.session.revoke",
        }
    }

    /// Parse from stable string key.
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "project.read" => Some(Self::ProjectRead),
            "members.read" => Some(Self::MembersRead),
            "members.write" => Some(Self::MembersWrite),
            "credentials.read" => Some(Self::CredentialsRead),
            "credentials.write" => Some(Self::CredentialsWrite),
            "templates.read" => Some(Self::TemplatesRead),
            "templates.write" => Some(Self::TemplatesWrite),
            "templates.create" => Some(Self::TemplatesCreate),
            "templates.delete" => Some(Self::TemplatesDelete),
            "templates.move" => Some(Self::TemplatesMove),
            "templates.diagnostics" => Some(Self::TemplatesDiagnostics),
            "pipelines.read" => Some(Self::PipelinesRead),
            "pipelines.write" => Some(Self::PipelinesWrite),
            "pipelines.create" => Some(Self::PipelinesCreate),
            "pipelines.delete" => Some(Self::PipelinesDelete),
            "pipelines.move" => Some(Self::PipelinesMove),
            "pipelines.execute" => Some(Self::PipelinesExecute),
            "files.read" => Some(Self::FilesRead),
            "files.write" => Some(Self::FilesWrite),
            "files.delete" => Some(Self::FilesDelete),
            "tables.read" => Some(Self::TablesRead),
            "tables.write" => Some(Self::TablesWrite),
            "libraries.read" => Some(Self::LibrariesRead),
            "libraries.install" => Some(Self::LibrariesInstall),
            "libraries.remove" => Some(Self::LibrariesRemove),
            "settings.read" => Some(Self::SettingsRead),
            "settings.write" => Some(Self::SettingsWrite),
            "mcp.session.create" => Some(Self::McpSessionCreate),
            "mcp.session.revoke" => Some(Self::McpSessionRevoke),
            _ => None,
        }
    }

    /// All capabilities — used when auto-creating an MCP session on first activation.
    pub fn all() -> Vec<Self> {
        vec![
            Self::ProjectRead,
            Self::MembersRead,
            Self::MembersWrite,
            Self::CredentialsRead,
            Self::CredentialsWrite,
            Self::TemplatesRead,
            Self::TemplatesWrite,
            Self::TemplatesCreate,
            Self::TemplatesDelete,
            Self::TemplatesMove,
            Self::TemplatesDiagnostics,
            Self::PipelinesRead,
            Self::PipelinesWrite,
            Self::PipelinesCreate,
            Self::PipelinesDelete,
            Self::PipelinesMove,
            Self::PipelinesExecute,
            Self::FilesRead,
            Self::FilesWrite,
            Self::FilesDelete,
            Self::TablesRead,
            Self::TablesWrite,
            Self::LibrariesRead,
            Self::LibrariesInstall,
            Self::LibrariesRemove,
            Self::SettingsRead,
            Self::SettingsWrite,
            Self::McpSessionCreate,
            Self::McpSessionRevoke,
        ]
    }
}

/// Product-facing managed role presets for project membership.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectAccessRolePreset {
    Guest,
    #[default]
    Reporter,
    Developer,
    Maintainer,
    Owner,
}

impl ProjectAccessRolePreset {
    /// Stable preset id used for storage and API transport.
    pub const fn key(self) -> &'static str {
        match self {
            Self::Guest => "guest",
            Self::Reporter => "reporter",
            Self::Developer => "developer",
            Self::Maintainer => "maintainer",
            Self::Owner => "owner",
        }
    }

    /// Human-readable title for settings and invite UI.
    pub const fn title(self) -> &'static str {
        match self {
            Self::Guest => "Guest",
            Self::Reporter => "Reporter",
            Self::Developer => "Developer",
            Self::Maintainer => "Maintainer",
            Self::Owner => "Owner",
        }
    }

    /// Managed policy id emitted by the role preset layer.
    pub const fn policy_id(self) -> &'static str {
        self.key()
    }
}

/// One explicit project member row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectMember {
    /// Stable internal project id.
    #[serde(default)]
    pub project_id: String,
    /// Owner identifier.
    pub owner: String,
    /// Project slug.
    pub project: String,
    /// Public user slug receiving membership.
    pub user_id: String,
    /// Stable internal user id receiving membership.
    #[serde(default)]
    pub member_user_id: String,
    /// Managed role preset.
    pub role_preset: ProjectAccessRolePreset,
    /// Extra policy ids layered on top of the role preset.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_policy_ids: Vec<String>,
    /// Optional MCP capability ceiling for future per-user MCP sessions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_capability_ceiling: Vec<ProjectCapability>,
    /// Acting user that created or last updated the row.
    #[serde(default)]
    pub created_by: String,
    /// Stable internal actor user id.
    #[serde(default)]
    pub created_by_user_id: String,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
}

/// Invite lifecycle state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectInviteStatus {
    #[default]
    Pending,
    Accepted,
    Revoked,
    Expired,
}

/// One pending or historical invite row for project collaboration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectInvite {
    /// Owner identifier.
    pub owner: String,
    /// Project slug.
    pub project: String,
    /// Stable invite id.
    pub invite_id: String,
    /// User id being invited.
    pub target_user: String,
    /// Managed role preset to grant on acceptance.
    pub role_preset: ProjectAccessRolePreset,
    /// Extra policy ids to apply on acceptance.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_policy_ids: Vec<String>,
    /// Optional MCP capability ceiling to apply on acceptance.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_capability_ceiling: Vec<ProjectCapability>,
    /// Optional human note attached to the invite.
    #[serde(default)]
    pub note: String,
    /// Inviting actor user id.
    #[serde(default)]
    pub invited_by: String,
    /// Invite lifecycle state.
    #[serde(default)]
    pub status: ProjectInviteStatus,
    /// Optional expiry timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
}

/// Create/update payload for one explicit project member.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpsertProjectMemberRequest {
    /// User id receiving membership.
    pub user_id: String,
    /// Managed role preset.
    pub role_preset: ProjectAccessRolePreset,
    /// Extra policy ids beyond the managed role preset.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_policy_ids: Vec<String>,
    /// Optional MCP capability ceiling for this member.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_capability_ceiling: Vec<ProjectCapability>,
}

/// Create payload for one project invite.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateProjectInviteRequest {
    /// Target user id.
    pub target_user: String,
    /// Managed role preset proposed by the invite.
    pub role_preset: ProjectAccessRolePreset,
    /// Extra policy ids to layer on top of the role preset.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_policy_ids: Vec<String>,
    /// Optional MCP capability ceiling for the invited user.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_capability_ceiling: Vec<ProjectCapability>,
    /// Optional note shown in invite listings.
    #[serde(default)]
    pub note: String,
    /// Optional invite expiry timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
}

/// Project policy bundle stored in metadata and reused by users, MCP sessions, and assistants.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectPolicy {
    /// Stable internal project id.
    #[serde(default)]
    pub project_id: String,
    /// Owner identifier.
    pub owner: String,
    /// Project slug.
    pub project: String,
    /// Stable policy id (`viewer`, `editor`, `owner`, ...).
    pub policy_id: String,
    /// Display label.
    pub title: String,
    /// Capability bundle.
    pub capabilities: Vec<ProjectCapability>,
    /// Whether this policy is platform-managed.
    pub managed: bool,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
}

/// Subject kind bound to one project policy.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProjectSubjectKind {
    User,
    McpSession,
    AssistantProfile,
}

impl ProjectSubjectKind {
    /// Stable string id for storage and transport.
    pub fn key(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::McpSession => "mcp_session",
            Self::AssistantProfile => "assistant_profile",
        }
    }
}

/// One project-level subject -> policy binding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectPolicyBinding {
    /// Stable internal project id.
    #[serde(default)]
    pub project_id: String,
    /// Owner identifier.
    pub owner: String,
    /// Project slug.
    pub project: String,
    /// Subject kind (`user`, `mcp_session`, `assistant_profile`).
    pub subject_kind: ProjectSubjectKind,
    /// Stable subject id.
    pub subject_id: String,
    /// Bound policy id.
    pub policy_id: String,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
}

/// Runtime subject passed into authorization checks so REST, MCP, and assistant paths
/// share the same policy gate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectAccessSubject {
    /// Subject kind.
    pub kind: ProjectSubjectKind,
    /// Stable subject id.
    pub id: String,
}

impl ProjectAccessSubject {
    /// Creates a user subject.
    pub fn user(owner: &str) -> Self {
        Self {
            kind: ProjectSubjectKind::User,
            id: slug_segment(owner),
        }
    }

    /// Creates an MCP session subject.
    pub fn mcp_session(token: &str) -> Self {
        Self {
            kind: ProjectSubjectKind::McpSession,
            id: token.to_string(),
        }
    }
}

/// Pipeline metadata catalog entry stored in platform-level metadata DB.
///
/// The pipeline source file is stored under one project `app/` workspace
/// (git-sync friendly). This metadata is the fast index used by platform UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineMeta {
    /// Owner identifier.
    pub owner: String,
    /// Project slug.
    pub project: String,
    /// Pipeline id/name.
    pub name: String,
    /// Optional display title.
    pub title: String,
    /// Logical virtual folder path (for registry hierarchy), canonicalized.
    pub virtual_path: String,
    /// Relative source file path under project `app/` root.
    pub file_rel_path: String,
    /// Optional human-readable description.
    pub description: String,
    /// Trigger kind (`webhook`, `schedule`, `function`, ...).
    pub trigger_kind: String,
    /// Stable content hash for change tracking.
    pub hash: String,
    /// Activated production hash. When this differs from `hash`, the working
    /// tree has draft changes that are not promoted to runtime yet.
    pub active_hash: Option<String>,
    /// Unix timestamp seconds when the current active hash was promoted.
    pub activated_at: Option<i64>,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
}

/// One project doc file/folder in app/docs (ERD, README.md, AGENTS.md, use cases, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectDocItem {
    /// Relative path under app/docs.
    pub path: String,
    /// Display name (file or folder name).
    pub name: String,
    /// "file" or "folder".
    pub kind: String,
}

/// One agent doc entry (AGENTS.md, SOUL.md, MEMORY.md).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentDocItem {
    /// File name (e.g. "AGENTS.md").
    pub name: String,
    /// Whether the user can edit this doc via REST (false for MEMORY.md — agent-only).
    pub user_editable: bool,
}

/// One breadcrumb segment in pipeline registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineBreadcrumb {
    /// Display name.
    pub name: String,
    /// Link to this level.
    pub path: String,
    /// Whether a slash separator should appear before this segment.
    pub show_divider: bool,
}

/// One child folder shown in pipeline registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineFolderItem {
    /// Folder segment name.
    pub name: String,
    /// Link to drill-down into this folder.
    pub path: String,
    /// True for reserved folders (assets, styles) — pinned at the bottom of the registry list.
    pub is_special: bool,
}

/// One template/script/style file shown in the pipeline registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryFileItem {
    /// File name including extension (e.g. `home.tsx`).
    pub name: String,
    /// Path relative to `repo/` root (e.g. `pipelines/pages/home.tsx`).
    pub rel_path: String,
    /// File kind: `"template"` (.tsx), `"script"` (.ts), `"style"` (.css).
    pub kind: String,
    /// URL to open this file in the template editor.
    pub edit_href: String,
    /// Git status code if dirty.
    pub git_status: Option<String>,
}

/// One pipeline item shown at one registry level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineRegistryItem {
    /// Pipeline name/id.
    pub name: String,
    /// Optional title.
    pub title: String,
    /// Description.
    pub description: String,
    /// Trigger kind.
    pub trigger_kind: String,
    /// Source file path under `app/`.
    pub file_rel_path: String,
    /// True when pipeline has an active hash matching current hash.
    pub is_active: bool,
    /// True when pipeline has an active hash but it differs from current hash (draft changes).
    pub has_draft: bool,
    /// Git status code (e.g. "M", "??", "D") if file is dirty, otherwise None.
    pub git_status: Option<String>,
}

/// Pipeline registry listing payload for one project + folder path.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineRegistryListing {
    /// Canonical current virtual path (`/` or `/a/b`).
    pub current_path: String,
    /// Breadcrumbs from root to current path.
    pub breadcrumbs: Vec<PipelineBreadcrumb>,
    /// Immediate child folders (sorted: normal first, special last).
    pub folders: Vec<PipelineFolderItem>,
    /// Pipeline entries located exactly at `current_path`.
    pub pipelines: Vec<PipelineRegistryItem>,
    /// Template/script/style files physically at `current_path`.
    pub files: Vec<RegistryFileItem>,
}

/// API payload used to create/update one pipeline definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpsertPipelineDefinitionRequest {
    /// Stable file-relative path under `repo/` (e.g. `"pipelines/api/my-hook.zf.json"`).
    /// This is the single canonical identifier — name and virtual_path are derived from it.
    pub file_rel_path: String,
    /// Optional display title.
    #[serde(default)]
    pub title: String,
    /// Optional human-readable description.
    #[serde(default)]
    pub description: String,
    /// Trigger kind (`webhook`, `schedule`, `function`, ...).
    #[serde(default)]
    pub trigger_kind: String,
    /// Full pipeline source (`*.zf.json`).
    pub source: String,
}

/// API payload used to delete one pipeline by its stable file_rel_path.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeletePipelineRequest {
    /// Relative path of the pipeline source file under `repo/`.
    pub file_rel_path: String,
}

/// API payload for committing repo changes via git.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitCommitRequest {
    /// File paths relative to `repo/` to stage and commit.
    pub files: Vec<String>,
    /// Commit message.
    pub message: String,
    /// Whether to push after committing.
    #[serde(default)]
    pub push: bool,
    /// Credential ID to use for authenticated push (optional).
    pub credential_id: Option<String>,
    /// Remote repository URL for authenticated push (optional).
    pub repo_url: Option<String>,
    /// Branch to push to (optional, defaults to current branch).
    pub branch: Option<String>,
}

/// Request body for `PUT /api/projects/{owner}/{project}/settings/{section}`.
///
/// Wraps the section-specific data alongside a git commit message.
/// The handler writes the section to `zebflow.json` then commits the file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSettingsSectionRequest {
    /// Git commit message. Shown in the commit dialog before save.
    pub commit_message: String,
    /// Section-specific payload. Deserialized per `{section}`.
    pub data: serde_json::Value,
}

/// API payload used to target one pipeline by its stable file path.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineLocateRequest {
    /// Stable file-relative path under `repo/` (e.g. `"pipelines/api/my-hook.zf.json"`).
    pub file_rel_path: String,
}

/// Trigger type used for explicit pipeline execution calls.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PipelineExecuteTrigger {
    Webhook,
    Schedule,
    Manual,
}

/// API payload used to execute one active pipeline with explicit trigger context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutePipelineRequest {
    /// Stable file-relative path under `repo/` (e.g. `"pipelines/api/my-hook.zf.json"`).
    pub file_rel_path: String,
    /// Trigger mode to validate against active trigger nodes.
    pub trigger: PipelineExecuteTrigger,
    /// Optional webhook path matcher.
    #[serde(default)]
    pub webhook_path: Option<String>,
    /// Optional webhook method matcher.
    #[serde(default)]
    pub webhook_method: Option<String>,
    /// Optional schedule cron matcher.
    #[serde(default)]
    pub schedule_cron: Option<String>,
    /// Request input payload passed to pipeline execution.
    #[serde(default)]
    pub input: serde_json::Value,
}

/// Payload used to create/update one project doc file under `app/docs`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpsertProjectDocRequest {
    /// Relative path under `app/docs`.
    pub path: String,
    /// Full file content.
    #[serde(default)]
    pub content: String,
}

/// Payload used to create one project docs folder under `repo/docs`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateProjectDocFolderRequest {
    /// Relative folder path under `repo/docs`.
    pub path: String,
}

/// Payload used to move one project docs file or folder.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectDocMoveRequest {
    /// Existing relative path under `repo/docs`.
    pub from_path: String,
    /// Destination parent folder under `repo/docs`.
    pub to_parent_path: String,
}

/// One template tree row for the templates workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateTreeItem {
    /// Display name.
    pub name: String,
    /// Relative path under `app/templates`.
    pub rel_path: String,
    /// `folder` or `file`.
    pub kind: String,
    /// Nesting depth from template root.
    pub depth: usize,
    /// File classification for icon/behavior hints.
    pub file_kind: String,
    /// Whether the entry is protected from destructive actions.
    pub is_protected: bool,
}

/// Template workspace listing for one project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateWorkspaceListing {
    /// Relative path of the preferred initial file.
    pub default_file: Option<String>,
    /// Flattened tree rows in display order.
    pub items: Vec<TemplateTreeItem>,
}

/// One file status row from the project git repository for templates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateGitStatusItem {
    /// Relative path under `app/templates`.
    pub rel_path: String,
    /// Short git porcelain status such as `M`, `A`, `D`, `??`, or `R`.
    pub code: String,
}

/// Payload used to save one template file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateSaveRequest {
    /// Relative path under `app/templates`.
    pub rel_path: String,
    /// Full file content.
    pub content: String,
}

/// Supported controlled template creation kinds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TemplateCreateKind {
    /// `templates/pages/*.tsx`
    Page,
    /// `templates/components/*.tsx`
    Component,
    /// `templates/scripts/*.ts`
    Script,
    /// arbitrary folder inside `templates/`
    Folder,
}

/// Payload used to create one controlled template entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateCreateRequest {
    /// Creation kind.
    pub kind: TemplateCreateKind,
    /// Human-entered base name.
    pub name: String,
    /// Optional parent folder under `app/templates`.
    pub parent_rel_path: Option<String>,
}

/// Payload used to move one template entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateMoveRequest {
    /// Existing relative path under `app/templates`.
    pub from_rel_path: String,
    /// Destination parent folder under `app/templates`.
    pub to_parent_rel_path: String,
}

/// Basic template file response used by the web editor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateFilePayload {
    /// Relative path under `app/templates`.
    pub rel_path: String,
    /// Display filename.
    pub name: String,
    /// File classification.
    pub file_kind: String,
    /// Full file content.
    pub content: String,
    /// Line count.
    pub line_count: usize,
    /// Whether the entry is protected from destructive actions.
    pub is_protected: bool,
}

/// Request payload used to compile one current template buffer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateCompileRequest {
    /// Relative path under `app/templates`.
    pub rel_path: String,
    /// Unsaved editor content to compile.
    pub content: String,
}

/// One platform-facing template compile diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateDiagnostic {
    /// Stable diagnostic code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// `error` or `warning`.
    pub severity: String,
    /// Optional zero-based source start offset.
    pub from: Option<usize>,
    /// Optional zero-based source end offset.
    pub to: Option<usize>,
}

/// Compile result returned to the web editor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateCompileResponse {
    /// Whether compile completed without a hard failure.
    pub ok: bool,
    /// Diagnostics emitted by the compile path.
    pub diagnostics: Vec<TemplateDiagnostic>,
}

/// One attribute definition in a Simple Table collection schema.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionAttribute {
    /// Field name (slug).
    pub name: String,
    /// Data kind: `string` | `number` | `boolean` | `text` | `json` | `vector` | `geo`.
    pub kind: String,
    /// Active index types: `hash` | `range` | `fulltext` | `vector` | `spatial`.
    #[serde(default)]
    pub index_types: Vec<String>,
}

/// One managed Simple Table definition stored inside the project runtime DB.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimpleTableDefinition {
    /// Stable table slug.
    pub table: String,
    /// Display title.
    pub title: String,
    /// Backing collection name.
    pub collection: String,
    /// Attribute schema definitions.
    #[serde(default)]
    pub attributes: Vec<CollectionAttribute>,
    /// Hash indexed payload fields (exact equality).
    #[serde(default)]
    pub hash_indexed_fields: Vec<String>,
    /// Range indexed payload fields (ordered scans).
    #[serde(default)]
    pub range_indexed_fields: Vec<String>,
    /// Full-text indexed payload fields.
    #[serde(default)]
    pub fulltext_fields: Vec<String>,
    /// Vector indexed payload fields (semantic similarity).
    #[serde(default)]
    pub vector_fields: Vec<String>,
    /// Spatial indexed payload fields (geo queries).
    #[serde(default)]
    pub spatial_fields: Vec<String>,
    /// Live row count.
    pub row_count: usize,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
}

/// Create payload for one project credential.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpsertProjectCredentialRequest {
    /// Stable credential id.
    pub credential_id: String,
    /// Display title.
    pub title: String,
    /// Driver/kind (`postgres`, `openai`, ...).
    pub kind: String,
    /// Secret payload.
    #[serde(default)]
    pub secret: serde_json::Value,
    /// Optional freeform notes.
    #[serde(default)]
    pub notes: String,
}

/// Create/update payload for one project DB connection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpsertProjectDbConnectionRequest {
    /// Stable route slug.
    pub connection_slug: String,
    /// Display label.
    pub connection_label: String,
    /// Database kind (`sqlite`, `postgresql`, ...).
    pub database_kind: String,
    /// Optional linked credential id.
    pub credential_id: Option<String>,
    /// Optional kind-specific config payload.
    #[serde(default)]
    pub config: serde_json::Value,
}

/// Create/update payload for one project assistant config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpsertProjectAssistantConfigRequest {
    /// Credential id for high-level reasoning/planning model.
    pub llm_high_credential_id: Option<String>,
    /// Credential id for general/cheap model.
    pub llm_general_credential_id: Option<String>,
    /// Max execution steps per request.
    pub max_steps: Option<u32>,
    /// Max replan attempts.
    pub max_replans: Option<u32>,
    /// Whether assistant is enabled for this project.
    pub enabled: Option<bool>,
    /// Number of user+assistant pairs to persist as server-side chat history.
    pub chat_history_pairs: Option<u32>,
}

/// Test payload for one project DB connection (existing or draft).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestProjectDbConnectionRequest {
    /// Existing connection slug to test.
    pub connection_slug: Option<String>,
    /// Optional draft database kind (used when slug is not provided).
    pub database_kind: Option<String>,
    /// Optional draft credential id (used when slug is not provided).
    pub credential_id: Option<String>,
    /// Optional draft config payload.
    #[serde(default)]
    pub config: serde_json::Value,
}

/// Describe payload for one DB connection runtime endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DescribeProjectDbConnectionRequest {
    /// Describe scope (`tree`, `schemas`, `tables`, `functions`, `columns`).
    pub scope: Option<String>,
    /// Optional schema filter.
    pub schema: Option<String>,
    /// Filter to a specific table for column-level detail.
    /// Format: "schema.table" (e.g. "academic.staff") or just "table" for public schema.
    /// When set, scope is treated as "columns" automatically.
    #[serde(default)]
    pub table: Option<String>,
    /// Whether system schemas should be included when supported.
    pub include_system: Option<bool>,
}

/// One node in DB object tree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DbObjectNode {
    /// Object kind (`schema`, `table`, `function`).
    pub kind: String,
    /// Object name.
    pub name: String,
    /// Optional schema name.
    pub schema: Option<String>,
    /// Optional children.
    #[serde(default)]
    pub children: Vec<DbObjectNode>,
    /// Optional metadata.
    #[serde(default)]
    pub meta: serde_json::Value,
}

/// Describe result for one DB connection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectDbConnectionDescribeResult {
    /// Stable connection id.
    pub connection_id: String,
    /// Stable route slug.
    pub connection_slug: String,
    /// Database kind (`sqlite`, `postgresql`, ...).
    pub database_kind: String,
    /// Effective scope.
    pub scope: String,
    /// Object tree/list payload.
    #[serde(default)]
    pub nodes: Vec<DbObjectNode>,
}

/// Query payload for one DB connection runtime endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct QueryProjectDbConnectionRequest {
    /// SQL text (for SQL engines like PostgreSQL).
    #[serde(default)]
    pub sql: String,
    /// Positional bind parameters.
    #[serde(default)]
    pub params: Vec<serde_json::Value>,
    /// Optional table identifier for engines that do not use SQL directly.
    pub table: Option<String>,
    /// Optional max rows to return.
    pub limit: Option<usize>,
    /// Whether write statements are blocked (defaults true).
    pub read_only: Option<bool>,
}

/// One query result column.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DbQueryColumn {
    /// Column name.
    pub name: String,
    /// Optional engine-native data type.
    pub data_type: Option<String>,
}

/// Query result for one DB connection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectDbConnectionQueryResult {
    /// Stable connection id.
    pub connection_id: String,
    /// Stable route slug.
    pub connection_slug: String,
    /// Database kind (`sqlite`, `postgresql`, ...).
    pub database_kind: String,
    /// Returned columns.
    #[serde(default)]
    pub columns: Vec<DbQueryColumn>,
    /// Returned rows as column-aligned vectors.
    #[serde(default)]
    pub rows: Vec<Vec<serde_json::Value>>,
    /// Number of rows in this payload.
    pub row_count: usize,
    /// Whether payload was truncated due to row limit.
    pub truncated: bool,
    /// Optional affected rows for write statements.
    pub affected_rows: Option<u64>,
    /// Query execution duration in milliseconds.
    pub duration_ms: u64,
}

/// Result payload returned by DB connection test endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectDbConnectionTestResult {
    /// Whether test passed.
    pub ok: bool,
    /// Human-friendly message.
    pub message: String,
    /// Optional details.
    pub details: serde_json::Value,
}

/// Create payload for one Simple Table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateSimpleTableRequest {
    /// Stable table slug.
    pub table: String,
    /// Optional display title.
    pub title: Option<String>,
    /// Attribute schema definitions.
    #[serde(default)]
    pub attributes: Vec<CollectionAttribute>,
    /// Hash indexed payload fields.
    #[serde(default)]
    pub hash_indexed_fields: Vec<String>,
    /// Range indexed payload fields.
    #[serde(default)]
    pub range_indexed_fields: Vec<String>,
}

/// Upsert payload for one Simple Table row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpsertSimpleTableRowRequest {
    /// Target table slug.
    pub table: String,
    /// Stable row id within the table.
    pub row_id: String,
    /// Row payload.
    #[serde(default)]
    pub data: serde_json::Value,
}

/// Query payload for one Simple Table read.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimpleTableQueryRequest {
    /// Target table slug.
    pub table: String,
    /// Optional equality field name.
    pub where_field: Option<String>,
    /// Equality field value.
    pub where_value: Option<serde_json::Value>,
    /// Maximum rows to return.
    #[serde(default = "default_simple_table_limit")]
    pub limit: usize,
}

fn default_simple_table_limit() -> usize {
    100
}

/// Query result returned by project Simple Table management and nodes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimpleTableQueryResult {
    /// Table definition.
    pub table: SimpleTableDefinition,
    /// Returned rows.
    pub rows: Vec<serde_json::Value>,
}

/// File-system tree returned for one project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFileLayout {
    /// `{data_root}/users/{owner}/{project}`
    pub root: PathBuf,
    /// `.../data`
    pub data_dir: PathBuf,
    /// `.../data/runtime`
    pub data_runtime_dir: PathBuf,
    /// `.../data/runtime/pipelines`
    pub data_runtime_pipelines_dir: PathBuf,
    /// `.../files`
    pub files_dir: PathBuf,
    /// `.../repo` (git-sync workspace root).
    pub repo_dir: PathBuf,
    /// `.../repo/.git`
    pub repo_git_dir: PathBuf,
    /// `.../repo/pipelines` — unified source root: *.zf.json pipelines + *.tsx templates + *.ts scripts.
    /// Also the @/ root for the RWE compiler.
    pub repo_pipelines_dir: PathBuf,
    /// `.../repo/docs` (project docs: ERD, README.md, AGENTS.md, use cases, etc.; UI label may be "Schema")
    pub repo_docs_dir: PathBuf,
    /// `.../repo/zebflow.json` (Layer 2 non-sensitive project config, git-synced).
    pub zebflow_json_file: PathBuf,
    /// `.../data/runtime/agent_docs` (AGENTS.md, SOUL.md, MEMORY.md — agent context)
    pub agent_docs_dir: PathBuf,
}

/// Request payload for user creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    /// Target owner/user id.
    pub owner: String,
    /// Password.
    pub password: String,
    /// Role.
    #[serde(default = "default_member_role")]
    pub role: String,
    /// Display name for git commits.
    #[serde(default)]
    pub git_name: String,
    /// Email for git commits.
    #[serde(default)]
    pub git_email: String,
}

/// Self-service user settings payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UpdateUserSettingsRequest {
    /// Display name for git commits.
    #[serde(default)]
    pub git_name: String,
    /// Email for git commits.
    #[serde(default)]
    pub git_email: String,
}

fn default_member_role() -> String {
    "member".to_string()
}

/// Source used to resolve one git author identity.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GitIdentitySource {
    UserProfile,
    ProjectSettings,
    Fallback,
}

/// Concrete git identity chosen for one write action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedGitIdentity {
    /// Resolved git author/committer name.
    pub name: String,
    /// Resolved git author/committer email.
    pub email: String,
    /// Source from which the identity was resolved.
    pub source: GitIdentitySource,
}

/// Layer 2 project config stored in `repo/zebflow.json` (git-synced, non-sensitive).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZebflowJson {
    #[serde(default = "default_zebflow_json_version")]
    pub version: String,
    #[serde(default)]
    pub metadata: ZebflowJsonMetadata,
    #[serde(default)]
    pub configs: ZebflowJsonConfigs,
    #[serde(default)]
    pub distribution: ZebflowJsonDistribution,
}

fn default_zebflow_json_version() -> String {
    "1.0".to_string()
}

fn default_max_asset_size_mb() -> u32 {
    10
}

fn default_webhook_body_max_mb() -> u32 {
    100
}

pub fn default_pipeline_node_timeout_secs() -> u64 {
    30
}

impl Default for ZebflowJson {
    fn default() -> Self {
        Self {
            version: default_zebflow_json_version(),
            metadata: ZebflowJsonMetadata::default(),
            configs: ZebflowJsonConfigs::default(),
            distribution: ZebflowJsonDistribution::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZebflowJsonMetadata {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZebflowJsonConfigs {
    #[serde(default)]
    pub rwe: ZebflowJsonRwe,
    #[serde(default)]
    pub pipelines: ZebflowJsonPipelines,
    #[serde(default)]
    pub runtime: ProjectRuntimeProfile,
    #[serde(default)]
    pub bootstrap: ProjectBootstrapPlan,
    #[serde(default)]
    pub git: ZebflowJsonGit,
    #[serde(default)]
    pub assistant: ZebflowJsonAssistant,
    #[serde(default)]
    pub locks: ZebflowJsonLocks,
    #[serde(default)]
    pub data: ZebflowJsonData,
    #[serde(default)]
    pub files: ZebflowJsonFiles,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZebflowJsonPipelines {
    #[serde(default)]
    pub logging: ZebflowJsonLogging,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_timeout_secs: Option<u64>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub nodes: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZebflowJsonData {}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZebflowJsonFiles {
    #[serde(default)]
    pub uploads: ZebflowJsonUploads,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZebflowJsonUploads {
    /// Max allowed size in MB for a single uploaded asset file (5–50, default 10).
    #[serde(default = "default_max_asset_size_mb")]
    pub max_asset_size_mb: u32,
    /// Max allowed size in MB for one webhook request body.
    ///
    /// This controls the per-project logical limit enforced by `/wh/...` handlers.
    /// Route-level Axum buffering still uses a higher fallback ceiling so project
    /// overrides can be enforced inside the request handler.
    #[serde(default = "default_webhook_body_max_mb")]
    pub webhook_body_max_mb: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_file_size_mb: Option<u32>,
}

impl Default for ZebflowJsonUploads {
    fn default() -> Self {
        Self {
            max_asset_size_mb: default_max_asset_size_mb(),
            webhook_body_max_mb: default_webhook_body_max_mb(),
            max_file_size_mb: None,
        }
    }
}

impl ZebflowJsonUploads {
    pub fn effective_max_asset_size_mb(&self) -> u32 {
        self.max_asset_size_mb.clamp(5, 100)
    }

    pub fn effective_webhook_body_max_mb(&self) -> u32 {
        self.webhook_body_max_mb
            .clamp(default_webhook_body_max_mb(), 512)
    }
}

impl ZebflowJsonPipelines {
    pub fn effective_node_timeout_secs(&self) -> u64 {
        self.node_timeout_secs
            .unwrap_or_else(default_pipeline_node_timeout_secs)
            .clamp(5, 3600)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZebflowJsonDistribution {
    #[serde(default)]
    pub marketplace: ZebflowJsonDistributionMarketplace,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZebflowJsonDistributionMarketplace {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub entry_url: String,
    #[serde(default)]
    pub as_app: bool,
    #[serde(default)]
    pub producer_enabled: bool,
}

/// Lock settings stored in `zebflow.json` — controls which resources agents cannot access.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZebflowJsonLocks {
    /// Template rel_paths (files or folder prefixes) blocked from agent read/write.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub templates: Vec<String>,
}

/// Git settings section of `zebflow.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZebflowJsonGit {
    #[serde(default)]
    pub remote: ZebflowJsonGitRemote,
}

/// Git remote section of `zebflow.json`.
/// Stores remote repository configuration for push/sync operations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZebflowJsonGitRemote {
    /// Credential ID for authenticated push (references `ProjectCredential.credential_id`).
    #[serde(default)]
    pub credential_id: String,
    /// Remote repository URL (e.g. `"https://gitlab.com/user/repo.git"`).
    #[serde(default)]
    pub repo_url: String,
    /// Default branch to push to (e.g. `"main"`).
    #[serde(default)]
    pub branch: String,
}

/// RWE settings section of `zebflow.json`.
///
/// Controls project-level compile/render behaviour for all `n.web.response` template nodes.
/// Values are merged into [`crate::rwe::ReactiveWebOptions`] at execution time,
/// before each pipeline run. Node-level `--load-scripts` is appended on top.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZebflowJsonRwe {
    /// URL patterns applied to the RWE resource allow-list (scripts and CSS).
    ///
    /// Each entry is a glob-style prefix, e.g. `https://cdnjs.cloudflare.com/*`.
    /// Blessed libraries (imported via `zeb/*`) are always allowed and do not
    /// appear here. Node-level `--load-scripts` is blocked at save time if any
    /// URL does not match an entry in this list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow_list: Vec<String>,
    /// Enable HTML minification on rendered output. Default: false.
    #[serde(default)]
    pub minify_html: bool,
    /// Enable strict compile-time checks. Default: true.
    #[serde(default = "default_rwe_strict_mode")]
    pub strict_mode: bool,
    /// Per-project enabled `zeb/*` library declarations.
    ///
    /// Keyed by library name (e.g. `"zeb/threejs"`). An entry here locks
    /// the version for reproducibility and surfaces the library in editor
    /// tooling. All libraries are served unconditionally at
    /// `/assets/libraries/zeb/…` regardless of this map.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub libraries: ZebflowJsonRweLibraries,
    /// Base URL prefix used when generating `<script src="...">` paths for
    /// compiled RWE scripts in production.
    ///
    /// Default (`None`): `/assets/{owner}/{project}/rwe/scripts/{hash}`
    ///
    /// When set, the prefix replaces `/assets/{owner}/{project}`, so the
    /// resulting path becomes `{deployment_asset_base}/rwe/scripts/{hash}`.
    ///
    /// Useful when the app is served behind a reverse proxy that rewrites
    /// asset paths, or when static assets are hosted at a custom sub-path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deployment_asset_base: Option<String>,
}

/// Enabled library map stored under `rwe.libraries` in `zebflow.json`.
pub type ZebflowJsonRweLibraries = HashMap<String, ZebflowJsonRweLibraryEntry>;

/// One enabled library entry stored in `zebflow.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZebflowJsonRweLibraryEntry {
    /// Pinned packed version string (e.g. `"bridge-0.1"`).
    pub version: String,
    /// Source kind: `"offline"` (embedded binary) or `"online"` (CDN fetch).
    pub source: String,
}

/// Lock file model — stored at `repo/zeb.lock` (git-tracked).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZebLock {
    /// Schema version; current is 1.
    pub version: u32,
    /// Locked library entries keyed by library name.
    #[serde(default)]
    pub libraries: HashMap<String, ZebLockEntry>,
}

/// One locked library entry in `zeb.lock`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZebLockEntry {
    /// Pinned packed version string (e.g. `"bridge-0.1"`).
    pub version: String,
    /// Source kind: `"offline"` or `"online"`.
    pub source: String,
    /// Relative runtime entry path (e.g. `"runtime/threejs.bundle.mjs"`).
    pub entry: String,
    /// SHA-256 integrity hash of the bundle file; `None` for embedded bridge bundles.
    pub integrity: Option<String>,
}

pub fn default_rwe_strict_mode() -> bool {
    true
}

/// API request to update the project-level RWE settings in `zebflow.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertRweSettingsRequest {
    #[serde(default)]
    pub allow_list: Vec<String>,
    #[serde(default)]
    pub minify_html: bool,
    #[serde(default = "default_rwe_strict_mode")]
    pub strict_mode: bool,
}

/// Logging settings section of zebflow.json.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZebflowJsonLogging {
    /// Max invocation entries to retain per pipeline. Defaults to 10.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_invocations: Option<u32>,
}

impl ZebflowJsonLogging {
    pub fn effective_max_invocations(&self) -> usize {
        self.max_invocations.unwrap_or(20).max(1) as usize
    }
}

/// One recorded pipeline invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineInvocationEntry {
    /// Stable run identifier for this execution.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub run_id: String,
    /// Unix timestamp (seconds).
    pub at: i64,
    /// Wall-clock duration of the execution in milliseconds.
    pub duration_ms: u64,
    /// `"ok"` or `"error"`.
    pub status: String,
    /// Trigger source: `"webhook"`, `"manual"`, `"schedule"`, etc.
    pub trigger: String,
    /// Short error message, present only when `status == "error"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Per-node execution trace for this invocation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trace: Vec<crate::pipeline::model::NodeTraceEntry>,
}

/// Assistant settings section of zebflow.json.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZebflowJsonAssistant {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high_model_credential: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub general_model_credential: Option<String>,
    #[serde(default)]
    pub max_steps: Option<u32>,
    #[serde(default)]
    pub max_replans: Option<u32>,
    #[serde(default)]
    pub chat_history_pairs: Option<u32>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

/// Request payload for project creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    /// Project slug.
    pub project: String,
    /// Optional title.
    pub title: Option<String>,
    /// Optional initial local branch name. Defaults to "main" when None or empty.
    #[serde(default)]
    pub local_branch: Option<String>,
    /// Environment-owned runtime selection captured at create/clone time.
    #[serde(default)]
    pub runtime: ProjectRuntimeSelectionRequest,
}

/// Environment-owned runtime selection supplied during create/clone flows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ProjectRuntimeSelectionRequest {
    /// Optional explicit runtime mode override for the project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_mode: Option<crate::infra::execution::placement::ProjectRuntimeMode>,
    /// Selected runtime host id, or `None`/`local` for the current office.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement_worker_id: Option<String>,
}

/// Internal control-plane payload used to materialize a project onto another office.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ProjectRuntimeMaterializationRequest {
    /// Portable project runtime bundle.
    #[serde(default)]
    pub bundle: ProjectRuntimeBundle,
    /// Runtime credentials to materialize locally on the destination office.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub credentials: Vec<ProjectCredential>,
    /// Runtime DB connections to materialize locally on the destination office.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub db_connections: Vec<ProjectDbConnection>,
}

/// Internal office registration request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ClusterWorkerRegisterRequest {
    /// Stable office id.
    pub node_id: String,
    /// Human-readable office label.
    #[serde(default)]
    pub label: String,
    /// Advertised base URL reachable by the controller or peers.
    #[serde(default)]
    pub base_url: String,
    /// Declared runtime capabilities.
    #[serde(default)]
    pub capabilities: RunnerCapabilities,
}

/// Internal office heartbeat request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ClusterWorkerHeartbeatRequest {
    /// Stable office id.
    pub node_id: String,
    /// Current status summary.
    #[serde(default)]
    pub status: String,
    /// Refreshed advertised base URL.
    #[serde(default)]
    pub base_url: String,
    /// Refreshed runtime capabilities.
    #[serde(default)]
    pub capabilities: RunnerCapabilities,
}

/// Internal office heartbeat response returned by the controller.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterWorkerHeartbeatResponse {
    /// Whether the heartbeat update was accepted.
    pub ok: bool,
    /// Normalized heartbeat snapshot.
    pub heartbeat: WorkerHeartbeat,
}

/// One runtime target option shown in project create/clone UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ClusterRuntimeTargetOption {
    /// Stable value posted back by forms.
    pub value: String,
    /// Human-readable office label.
    pub label: String,
    /// Short description line for selection UI.
    #[serde(default)]
    pub description: String,
}

/// Export/import artifact kind used by first-class project portability flows.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectTransferArtifactKind {
    /// Repository workspace plus project runtime data.
    Bundle,
    /// Public/private file storage trees.
    Files,
}

impl ProjectTransferArtifactKind {
    /// Stable API/storage key.
    pub fn key(self) -> &'static str {
        match self {
            Self::Bundle => "bundle",
            Self::Files => "files",
        }
    }

    /// Stable archive filename used for downloads and operation artifacts.
    pub fn archive_name(self) -> &'static str {
        match self {
            Self::Bundle => "project.bundle.tar",
            Self::Files => "project.files.tar",
        }
    }
}

/// Versioned manifest embedded in exported project portability archives.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectTransferManifest {
    /// Manifest schema version.
    pub schema_version: String,
    /// Stable owner id.
    pub owner: String,
    /// Stable project slug.
    pub project: String,
    /// Exported artifact type.
    pub artifact_kind: ProjectTransferArtifactKind,
    /// Office that produced the archive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_office_id: Option<String>,
    /// Controller that instructed the export, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_controller_id: Option<String>,
    /// Unix timestamp seconds.
    pub exported_at: i64,
    /// Repo-owned runtime profile at export time.
    #[serde(default)]
    pub runtime_profile: ProjectRuntimeProfile,
    /// Environment-owned placement snapshot at export time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<ProjectRuntimePlacement>,
    /// File count inside `repo/` when exporting a bundle.
    #[serde(default)]
    pub repo_file_count: u64,
    /// File count inside `data/` when exporting a bundle.
    #[serde(default)]
    pub data_file_count: u64,
    /// File count inside `files/public/` when exporting file storage.
    #[serde(default)]
    pub public_file_count: u64,
    /// File count inside `files/private/` when exporting file storage.
    #[serde(default)]
    pub private_file_count: u64,
    /// Total exported payload bytes before tar packaging.
    #[serde(default)]
    pub total_bytes: u64,
}

/// Durable controller-side operation kind for project portability and sync flows.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectOperationKind {
    /// Export `repo/` + `data/`.
    ExportBundle,
    /// Export `files/public` + `files/private`.
    ExportFiles,
    /// Import `repo/` + `data/`.
    ImportBundle,
    /// Import `files/public` + `files/private`.
    ImportFiles,
}

impl ProjectOperationKind {
    /// Stable storage key.
    pub fn key(self) -> &'static str {
        match self {
            Self::ExportBundle => "export_bundle",
            Self::ExportFiles => "export_files",
            Self::ImportBundle => "import_bundle",
            Self::ImportFiles => "import_files",
        }
    }
}

/// Durable controller-side operation state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectOperationStatus {
    Pending,
    Running,
    Failed,
    Completed,
}

impl ProjectOperationStatus {
    /// Stable storage key.
    pub fn key(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Failed => "failed",
            Self::Completed => "completed",
        }
    }
}

/// One durable controller-tracked project operation record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectOperationRecord {
    /// Stable operation id.
    pub operation_id: String,
    /// Stable project id.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub project_id: String,
    /// Stable owner id.
    pub owner: String,
    /// Stable project slug.
    pub project: String,
    /// Operation type.
    pub kind: ProjectOperationKind,
    /// Current status.
    pub status: ProjectOperationStatus,
    /// Human-readable current step.
    #[serde(default)]
    pub current_step: String,
    /// Source office id when relevant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_office_id: Option<String>,
    /// Target office id when relevant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_office_id: Option<String>,
    /// Relative artifact path under controller operation storage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_rel_path: Option<String>,
    /// Archive SHA-256 when an artifact exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_sha256: Option<String>,
    /// Artifact byte size when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_bytes: Option<u64>,
    /// Last error message when the operation failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Retry count for resumed operations.
    #[serde(default)]
    pub retry_count: u32,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
    /// Unix timestamp seconds when operation reached a terminal success state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<i64>,
}

/// Request payload for platform login page/form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    /// Username/owner id.
    pub identifier: String,
    /// Password.
    pub password: String,
}

/// Minimal auth session value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    /// Authenticated owner id.
    pub owner: String,
}

/// Returns unix timestamp seconds.
pub fn now_ts() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs() as i64,
        Err(_) => 0,
    }
}

/// Slug-normalize a segment for ids/paths.
pub fn slug_segment(raw: &str) -> String {
    raw.trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Canonicalize one virtual path used for pipeline registry hierarchy.
///
/// Rules:
/// - root is `/`
/// - removes empty segments
/// - disallows `.` and `..`
/// - each segment is slug-normalized
pub fn normalize_virtual_path(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "/" {
        return "/".to_string();
    }
    let mut parts = Vec::new();
    for seg in trimmed.split('/') {
        let seg = seg.trim();
        if seg.is_empty() || seg == "." || seg == ".." {
            continue;
        }
        let slug = slug_segment(seg);
        if !slug.is_empty() {
            parts.push(slug);
        }
    }
    if parts.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", parts.join("/"))
    }
}

fn default_true() -> bool {
    true
}

/// MCP session record (in-memory and persisted per-project).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSession {
    /// Owner identifier.
    pub owner: String,
    /// Project slug.
    pub project: String,
    /// Capabilities allowed for this session.
    pub capabilities: Vec<ProjectCapability>,
    /// Opaque session token.
    pub token: String,
    /// Unix timestamp seconds when session was created.
    #[serde(default)]
    pub created_at: i64,
    /// Optional seconds after which this session auto-expires.
    #[serde(default)]
    pub auto_reset_seconds: Option<u64>,
    /// Whether this session is active. Token persists even when disabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Request to create an MCP session for a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSessionCreateRequest {
    /// Capabilities to allow for this session (can be specified as strings or capability keys).
    pub capabilities: Vec<String>,
    /// Optional seconds after which this session auto-expires (None = no expiry).
    #[serde(default)]
    pub auto_reset_seconds: Option<u64>,
}

/// Request to toggle an MCP session enabled/disabled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSessionToggleRequest {
    /// Whether to enable or disable the session.
    pub enabled: bool,
}

/// Response after creating an MCP session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSessionResponse {
    /// Opaque session token for Authorization header.
    pub token: String,
    /// Full MCP endpoint URL for this project.
    pub mcp_url: String,
    /// Allowed capabilities echoed back.
    pub capabilities: Vec<String>,
    /// Whether the session is currently accepted by the MCP handler.
    pub enabled: bool,
    /// Session creation timestamp.
    pub created_at: i64,
    /// Optional seconds after which this session auto-expires.
    #[serde(default)]
    pub auto_reset_seconds: Option<u64>,
    /// Global minimum creation timestamp required for persisted MCP sessions.
    pub rotation_epoch: i64,
}

/// Maps MCP tool name to required project capability.
pub fn mcp_tool_capability(tool_name: &str) -> Option<ProjectCapability> {
    match tool_name {
        "list_pipelines" => Some(ProjectCapability::PipelinesRead),
        "get_pipeline" => Some(ProjectCapability::PipelinesRead),
        "upsert_pipeline" => Some(ProjectCapability::PipelinesWrite),
        "activate_pipeline" => Some(ProjectCapability::PipelinesWrite),
        "deactivate_pipeline" => Some(ProjectCapability::PipelinesWrite),
        "execute_pipeline" => Some(ProjectCapability::PipelinesExecute),
        "list_templates" => Some(ProjectCapability::TemplatesRead),
        "get_template" => Some(ProjectCapability::TemplatesRead),
        "save_template" => Some(ProjectCapability::TemplatesWrite),
        "create_template" => Some(ProjectCapability::TemplatesCreate),
        "delete_template" => Some(ProjectCapability::TemplatesDelete),
        "get_credential" => Some(ProjectCapability::CredentialsRead),
        "upsert_credential" => Some(ProjectCapability::CredentialsWrite),
        "list_db_connections" => Some(ProjectCapability::TablesRead),
        "get_db_connection" => Some(ProjectCapability::TablesRead),
        "upsert_db_connection" => Some(ProjectCapability::TablesWrite),
        "update_db_connection" => Some(ProjectCapability::TablesWrite),
        "delete_db_connection" => Some(ProjectCapability::TablesWrite),
        "test_db_connection" => Some(ProjectCapability::TablesRead),
        "describe_db_connection" => Some(ProjectCapability::TablesRead),
        "query_db_connection" => Some(ProjectCapability::TablesRead),
        "list_db_connection_schemas" => Some(ProjectCapability::TablesRead),
        "list_db_connection_tables" => Some(ProjectCapability::TablesRead),
        "list_db_connection_functions" => Some(ProjectCapability::TablesRead),
        "preview_db_connection_table" => Some(ProjectCapability::TablesRead),
        "list_connections" => Some(ProjectCapability::TablesRead),
        "describe_connection" => Some(ProjectCapability::TablesRead),
        "list_project_docs" => Some(ProjectCapability::ProjectRead),
        "read_project_doc" => Some(ProjectCapability::ProjectRead),
        "create_project_doc" => Some(ProjectCapability::FilesWrite),

        "list_credentials" => Some(ProjectCapability::CredentialsRead),
        // execute_pipeline_dsl temporarily disabled; keeping mapping for future re-enable
        "execute_pipeline_dsl" => Some(ProjectCapability::PipelinesExecute),
        "describe_pipeline" => Some(ProjectCapability::PipelinesRead),
        "register_pipeline" => Some(ProjectCapability::PipelinesWrite),
        "patch_pipeline" => Some(ProjectCapability::PipelinesWrite),
        "run_ephemeral" => Some(ProjectCapability::PipelinesExecute),
        "git_command" => Some(ProjectCapability::PipelinesWrite),
        "write_template" => Some(ProjectCapability::TemplatesWrite),
        "write_doc" => Some(ProjectCapability::TemplatesWrite),
        "list_agent_docs" => Some(ProjectCapability::SettingsRead),
        "read_agent_doc" => Some(ProjectCapability::SettingsRead),
        "write_agent_doc" => Some(ProjectCapability::SettingsWrite),
        // New domain-prefixed tool names
        "pipeline_list" => Some(ProjectCapability::PipelinesRead),
        "pipeline_get" => Some(ProjectCapability::PipelinesRead),
        "pipeline_register" => Some(ProjectCapability::PipelinesWrite),
        "pipeline_describe" => Some(ProjectCapability::PipelinesRead),
        "pipeline_patch" => Some(ProjectCapability::PipelinesWrite),
        "pipeline_activate" => Some(ProjectCapability::PipelinesWrite),
        "pipeline_deactivate" => Some(ProjectCapability::PipelinesWrite),
        "pipeline_execute" => Some(ProjectCapability::PipelinesExecute),
        "pipeline_run" => Some(ProjectCapability::PipelinesExecute),
        "pipeline_get_invocations" => Some(ProjectCapability::PipelinesRead),
        "template_list" => Some(ProjectCapability::TemplatesRead),
        "template_get" => Some(ProjectCapability::TemplatesRead),
        "template_create" => Some(ProjectCapability::TemplatesCreate),
        "template_write" => Some(ProjectCapability::TemplatesWrite),
        "template_search" => Some(ProjectCapability::TemplatesRead),
        "template_edit" => Some(ProjectCapability::TemplatesWrite),
        "template_outline" => Some(ProjectCapability::TemplatesRead),
        "template_deps" => Some(ProjectCapability::TemplatesRead),
        "template_batch_edit" => Some(ProjectCapability::TemplatesWrite),
        "pipeline_search" => Some(ProjectCapability::PipelinesRead),
        "connection_list" => Some(ProjectCapability::TablesRead),
        "connection_describe" => Some(ProjectCapability::TablesRead),
        "credential_list" => Some(ProjectCapability::CredentialsRead),
        "docs_project_list" => Some(ProjectCapability::ProjectRead),
        "docs_project_read" => Some(ProjectCapability::ProjectRead),
        "docs_project_write" => Some(ProjectCapability::TemplatesWrite),
        "docs_agent_list" => Some(ProjectCapability::SettingsRead),
        "docs_agent_read" => Some(ProjectCapability::SettingsRead),
        "docs_agent_write" => Some(ProjectCapability::SettingsWrite),
        // Help and orientation tools
        "start_here" => Some(ProjectCapability::ProjectRead),
        "help" => Some(ProjectCapability::ProjectRead),
        "help_search" => Some(ProjectCapability::ProjectRead),
        // UI catalog install tools
        "list_ui_catalog" => Some(ProjectCapability::PipelinesRead),
        "install_ui_components" => Some(ProjectCapability::PipelinesWrite),
        // Move resource (requires both pipelines write + templates write — use higher capability)
        "move_resource" => Some(ProjectCapability::PipelinesWrite),
        _ => None,
    }
}

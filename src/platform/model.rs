//! Platform domain models and configuration.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Data adapter selection.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DataAdapterKind {
    /// Sekejap-backed metadata store.
    #[default]
    Sekejap,
    /// Placeholder adapter.
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
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            data_root: PathBuf::from(".zebflow-platform-data"),
            data_adapter: DataAdapterKind::Sekejap,
            file_adapter: FileAdapterKind::Filesystem,
            default_owner: "superadmin".to_string(),
            default_password: String::new(),
            default_project: "default".to_string(),
        }
    }
}

/// User profile stored by platform metadata adapter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformUser {
    /// User identifier.
    pub owner: String,
    /// Role string (`superadmin`, `member`, ...).
    pub role: String,
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
    /// Plain password for prototype bootstrap.
    pub password: String,
}

/// Project profile stored by metadata adapter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformProject {
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
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
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
    /// Database kind (`sjtable`, `postgresql`, ...).
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
    /// Database kind (`sjtable`, `postgresql`, ...).
    pub database_kind: String,
    /// Optional linked credential id.
    pub credential_id: Option<String>,
    /// Unix timestamp seconds.
    pub created_at: i64,
    /// Unix timestamp seconds.
    pub updated_at: i64,
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
}

/// Project policy bundle stored in metadata and reused by users, MCP sessions, and assistants.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectPolicy {
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
    /// Immediate child folders.
    pub folders: Vec<PipelineFolderItem>,
    /// Pipeline entries located exactly at `current_path`.
    pub pipelines: Vec<PipelineRegistryItem>,
}

/// API payload used to create/update one pipeline definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpsertPipelineDefinitionRequest {
    /// Logical virtual folder path (`/` or `/a/b`).
    #[serde(default)]
    pub virtual_path: String,
    /// Pipeline id/name.
    pub name: String,
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
}

/// API payload used to target one pipeline by path and name.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineLocateRequest {
    /// Logical virtual folder path (`/` or `/a/b`).
    #[serde(default)]
    pub virtual_path: String,
    /// Pipeline id/name.
    pub name: String,
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
    /// Logical virtual folder path (`/` or `/a/b`).
    #[serde(default)]
    pub virtual_path: String,
    /// Pipeline id/name.
    pub name: String,
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
    /// Backing Sekejap collection name.
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
    /// Database kind (`sjtable`, `postgresql`, ...).
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
    /// Describe scope (`tree`, `schemas`, `tables`, `functions`).
    pub scope: Option<String>,
    /// Optional schema filter.
    pub schema: Option<String>,
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
    /// Database kind (`sjtable`, `postgresql`, ...).
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
    /// Database kind (`sjtable`, `postgresql`, ...).
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
    /// `.../data/sekejap` (project runtime db — general-purpose blank DB for user business data).
    pub data_sekejap_dir: PathBuf,
    /// `.../files`
    pub files_dir: PathBuf,
    /// `.../repo` (git-sync workspace root).
    pub repo_dir: PathBuf,
    /// `.../repo/.git`
    pub repo_git_dir: PathBuf,
    /// `.../repo/pipelines`
    pub repo_pipelines_dir: PathBuf,
    /// `.../repo/templates`
    pub repo_templates_dir: PathBuf,
    /// `.../repo/components`
    pub repo_components_dir: PathBuf,
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
}

fn default_member_role() -> String {
    "member".to_string()
}

/// Layer 2 project config stored in `repo/zebflow.json` (git-synced, non-sensitive).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZebflowJson {
    #[serde(default)]
    pub project: ZebflowJsonProject,
    #[serde(default)]
    pub assistant: ZebflowJsonAssistant,
    #[serde(default)]
    pub ui: ZebflowJsonUi,
}

/// Project identity section of zebflow.json.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZebflowJsonProject {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
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

/// UI preferences section of zebflow.json.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZebflowJsonUi {
    #[serde(default)]
    pub theme: String,
}

/// Request payload for project creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    /// Project slug.
    pub project: String,
    /// Optional title.
    pub title: Option<String>,
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

/// Response after creating an MCP session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSessionResponse {
    /// Opaque session token for Authorization header.
    pub token: String,
    /// Full MCP endpoint URL for this project.
    pub mcp_url: String,
    /// Allowed capabilities echoed back.
    pub capabilities: Vec<String>,
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
        "list_credentials" => Some(ProjectCapability::CredentialsRead),
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
        "list_tables" => Some(ProjectCapability::TablesRead),
        "list_project_docs" => Some(ProjectCapability::ProjectRead),
        "read_project_doc" => Some(ProjectCapability::ProjectRead),
        "create_project_doc" => Some(ProjectCapability::FilesWrite),
        "list_skills" => Some(ProjectCapability::ProjectRead),
        "read_skill" => Some(ProjectCapability::ProjectRead),
        "execute_pipeline_dsl" => Some(ProjectCapability::PipelinesExecute),
        _ => None,
    }
}

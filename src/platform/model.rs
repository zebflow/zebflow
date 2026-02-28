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

/// Atomic project-scoped permission used by REST, MCP, and internal assistants.
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
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

/// One managed Simple Table definition stored inside the project runtime DB.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimpleTableDefinition {
    /// Stable table slug.
    pub table: String,
    /// Display title.
    pub title: String,
    /// Backing Sekejap collection name.
    pub collection: String,
    /// Hash indexed payload fields.
    pub hash_indexed_fields: Vec<String>,
    /// Range indexed payload fields.
    pub range_indexed_fields: Vec<String>,
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

/// Create payload for one Simple Table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateSimpleTableRequest {
    /// Stable table slug.
    pub table: String,
    /// Optional display title.
    pub title: Option<String>,
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
    /// `.../data/sekejap` (project runtime db for SimpleTable node).
    pub data_sekejap_dir: PathBuf,
    /// `.../data/sqlite/project.db` (project sqlite runtime db).
    pub data_sqlite_file: PathBuf,
    /// `.../files`
    pub files_dir: PathBuf,
    /// `.../app` (git-sync workspace root).
    pub app_dir: PathBuf,
    /// `.../app/.git`
    pub app_git_dir: PathBuf,
    /// `.../app/pipelines`
    pub app_pipelines_dir: PathBuf,
    /// `.../app/templates`
    pub app_templates_dir: PathBuf,
    /// `.../app/components`
    pub app_components_dir: PathBuf,
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

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

/// File-system tree returned for one project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFileLayout {
    /// `{data_root}/users/{owner}/{project}`
    pub root: PathBuf,
    /// `.../data`
    pub data_dir: PathBuf,
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

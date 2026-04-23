//! SQLite-backed platform catalog adapter (WAL mode, bundled SQLite 3.47).
//!
//! Uses a single `catalog.db` file under `{data_root}/platform/` with proper
//! WAL journaling. Safe across K8s restarts — no unsafe mmap code.

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, params};
use serde_json::Value;

use crate::infra::cluster::registry::WorkerRegistryRecord;
use crate::infra::execution::placement::{
    ProjectRuntimeMode, ProjectRuntimePlacement, ProjectRuntimePlacementTarget,
};
use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    MarketplaceAssetPackage, MarketplaceAssetVersion, MarketplacePublisher, MarketplaceToken, McpSession,
    PipelineInvocationEntry, PipelineMeta, PlatformMarketplaceRepository, PlatformProject,
    PlatformUser, ProjectAccessRolePreset, ProjectCapability, ProjectCredential,
    ProjectDbConnection, ProjectInvite, ProjectInviteStatus, ProjectMarketplaceRepository,
    ProjectMember, ProjectOperationKind, ProjectOperationRecord, ProjectOperationStatus,
    ProjectPolicy, ProjectPolicyBinding, ProjectSubjectKind, StoredUser,
};

const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS users (
    owner         TEXT PRIMARY KEY,
    role          TEXT NOT NULL DEFAULT 'owner',
    git_name      TEXT NOT NULL DEFAULT '',
    git_email     TEXT NOT NULL DEFAULT '',
    password      TEXT NOT NULL DEFAULT '',
    created_at    INTEGER NOT NULL DEFAULT 0,
    updated_at    INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS projects (
    owner         TEXT NOT NULL,
    project       TEXT NOT NULL,
    created_at    INTEGER NOT NULL DEFAULT 0,
    updated_at    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project)
);
CREATE TABLE IF NOT EXISTS project_credentials (
    owner         TEXT NOT NULL,
    project       TEXT NOT NULL,
    credential_id TEXT NOT NULL,
    title         TEXT NOT NULL DEFAULT '',
    kind          TEXT NOT NULL DEFAULT '',
    secret_json   TEXT NOT NULL DEFAULT 'null',
    notes         TEXT NOT NULL DEFAULT '',
    created_at    INTEGER NOT NULL DEFAULT 0,
    updated_at    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, credential_id)
);
CREATE TABLE IF NOT EXISTS project_db_connections (
    owner            TEXT NOT NULL,
    project          TEXT NOT NULL,
    connection_id    TEXT NOT NULL,
    connection_slug  TEXT NOT NULL DEFAULT '',
    connection_label TEXT NOT NULL DEFAULT '',
    database_kind    TEXT NOT NULL DEFAULT '',
    credential_id    TEXT,
    config_json      TEXT NOT NULL DEFAULT 'null',
    created_at       INTEGER NOT NULL DEFAULT 0,
    updated_at       INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, connection_id)
);
CREATE TABLE IF NOT EXISTS project_marketplace_repositories (
    owner         TEXT NOT NULL,
    project       TEXT NOT NULL,
    repository_id TEXT NOT NULL,
    title         TEXT NOT NULL DEFAULT '',
    base_url      TEXT NOT NULL DEFAULT '',
    remote_owner  TEXT NOT NULL DEFAULT '',
    remote_project TEXT NOT NULL DEFAULT '',
    read_token    TEXT NOT NULL DEFAULT '',
    enabled       INTEGER NOT NULL DEFAULT 1,
    created_at    INTEGER NOT NULL DEFAULT 0,
    updated_at    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, repository_id)
);
CREATE TABLE IF NOT EXISTS platform_marketplace_repositories (
    owner         TEXT NOT NULL,
    repository_id TEXT NOT NULL,
    title         TEXT NOT NULL DEFAULT '',
    base_url      TEXT NOT NULL DEFAULT '',
    remote_owner  TEXT NOT NULL DEFAULT '',
    remote_project TEXT NOT NULL DEFAULT '',
    read_token    TEXT NOT NULL DEFAULT '',
    enabled       INTEGER NOT NULL DEFAULT 1,
    created_at    INTEGER NOT NULL DEFAULT 0,
    updated_at    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, repository_id)
);
CREATE TABLE IF NOT EXISTS marketplace_publishers (
    owner          TEXT NOT NULL,
    project        TEXT NOT NULL,
    publisher_id   TEXT NOT NULL,
    display_name   TEXT NOT NULL DEFAULT '',
    publisher_url  TEXT NOT NULL DEFAULT '',
    email          TEXT NOT NULL DEFAULT '',
    description    TEXT NOT NULL DEFAULT '',
    icon_url       TEXT NOT NULL DEFAULT '',
    website_url    TEXT NOT NULL DEFAULT '',
    enabled        INTEGER NOT NULL DEFAULT 1,
    created_at     INTEGER NOT NULL DEFAULT 0,
    updated_at     INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, publisher_id)
);
CREATE TABLE IF NOT EXISTS marketplace_asset_packages (
    package_id       TEXT PRIMARY KEY,
    authority_owner  TEXT NOT NULL DEFAULT '',
    authority_project TEXT NOT NULL DEFAULT '',
    publisher_owner  TEXT NOT NULL DEFAULT '',
    publisher_id     TEXT NOT NULL DEFAULT '',
    publisher_display_name TEXT NOT NULL DEFAULT '',
    publisher_url    TEXT NOT NULL DEFAULT '',
    publisher_email  TEXT NOT NULL DEFAULT '',
    asset_kind       TEXT NOT NULL DEFAULT '',
    title            TEXT NOT NULL DEFAULT '',
    description      TEXT NOT NULL DEFAULT '',
    visibility       TEXT NOT NULL DEFAULT 'private',
    tags_json        TEXT NOT NULL DEFAULT '[]',
    created_at       INTEGER NOT NULL DEFAULT 0,
    updated_at       INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS marketplace_asset_versions (
    package_id        TEXT NOT NULL,
    version           TEXT NOT NULL,
    authority_owner   TEXT NOT NULL DEFAULT '',
    authority_project TEXT NOT NULL DEFAULT '',
    publisher_owner   TEXT NOT NULL DEFAULT '',
    publisher_id      TEXT NOT NULL DEFAULT '',
    source_owner      TEXT NOT NULL DEFAULT '',
    source_project    TEXT NOT NULL DEFAULT '',
    source_kind       TEXT NOT NULL DEFAULT '',
    source_ref        TEXT NOT NULL DEFAULT '',
    artifact_rel_path TEXT NOT NULL DEFAULT '',
    artifact_sha256   TEXT NOT NULL DEFAULT '',
    manifest_json     TEXT NOT NULL DEFAULT 'null',
    created_at        INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (package_id, version)
);
CREATE TABLE IF NOT EXISTS marketplace_tokens (
    token_id       TEXT PRIMARY KEY,
    owner          TEXT NOT NULL DEFAULT '',
    project        TEXT NOT NULL DEFAULT '',
    publisher_id   TEXT NOT NULL DEFAULT '',
    publisher_display_name TEXT NOT NULL DEFAULT '',
    publisher_url  TEXT NOT NULL DEFAULT '',
    publisher_email TEXT NOT NULL DEFAULT '',
    title          TEXT NOT NULL DEFAULT '',
    secret_hash    TEXT NOT NULL DEFAULT '',
    scopes_json    TEXT NOT NULL DEFAULT '[]',
    expires_at     INTEGER,
    last_used_at   INTEGER,
    revoked_at     INTEGER,
    created_at     INTEGER NOT NULL DEFAULT 0,
    updated_at     INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS pipeline_meta (
    owner         TEXT NOT NULL,
    project       TEXT NOT NULL,
    file_rel_path TEXT NOT NULL,
    name          TEXT NOT NULL DEFAULT '',
    title         TEXT NOT NULL DEFAULT '',
    virtual_path  TEXT NOT NULL DEFAULT '',
    description   TEXT NOT NULL DEFAULT '',
    trigger_kind  TEXT NOT NULL DEFAULT '',
    hash          TEXT NOT NULL DEFAULT '',
    active_hash   TEXT,
    activated_at  INTEGER,
    created_at    INTEGER NOT NULL DEFAULT 0,
    updated_at    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, file_rel_path)
);
CREATE TABLE IF NOT EXISTS project_policies (
    owner             TEXT NOT NULL,
    project           TEXT NOT NULL,
    policy_id         TEXT NOT NULL,
    title             TEXT NOT NULL DEFAULT '',
    capabilities_json TEXT NOT NULL DEFAULT '[]',
    managed           INTEGER NOT NULL DEFAULT 0,
    created_at        INTEGER NOT NULL DEFAULT 0,
    updated_at        INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, policy_id)
);
CREATE TABLE IF NOT EXISTS project_policy_bindings (
    owner        TEXT NOT NULL,
    project      TEXT NOT NULL,
    subject_kind TEXT NOT NULL,
    subject_id   TEXT NOT NULL,
    policy_id    TEXT NOT NULL,
    created_at   INTEGER NOT NULL DEFAULT 0,
    updated_at   INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, subject_id, policy_id)
);
CREATE TABLE IF NOT EXISTS project_members (
    owner                  TEXT NOT NULL,
    project                TEXT NOT NULL,
    user_id                TEXT NOT NULL,
    role_preset            TEXT NOT NULL DEFAULT 'reporter',
    custom_policy_ids_json TEXT NOT NULL DEFAULT '[]',
    mcp_capabilities_json  TEXT NOT NULL DEFAULT '[]',
    created_by             TEXT NOT NULL DEFAULT '',
    created_at             INTEGER NOT NULL DEFAULT 0,
    updated_at             INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, user_id)
);
CREATE TABLE IF NOT EXISTS project_invites (
    owner                  TEXT NOT NULL,
    project                TEXT NOT NULL,
    invite_id              TEXT NOT NULL,
    target_user            TEXT NOT NULL DEFAULT '',
    role_preset            TEXT NOT NULL DEFAULT 'reporter',
    custom_policy_ids_json TEXT NOT NULL DEFAULT '[]',
    mcp_capabilities_json  TEXT NOT NULL DEFAULT '[]',
    note                   TEXT NOT NULL DEFAULT '',
    invited_by             TEXT NOT NULL DEFAULT '',
    status                 TEXT NOT NULL DEFAULT 'pending',
    expires_at             INTEGER,
    created_at             INTEGER NOT NULL DEFAULT 0,
    updated_at             INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, invite_id)
);
CREATE TABLE IF NOT EXISTS worker_registry (
    node_id             TEXT PRIMARY KEY,
    label               TEXT NOT NULL DEFAULT '',
    base_url            TEXT NOT NULL DEFAULT '',
    status              TEXT NOT NULL DEFAULT '',
    capabilities_json   TEXT NOT NULL DEFAULT '{}',
    registered_at       INTEGER NOT NULL DEFAULT 0,
    last_heartbeat_at   INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS project_runtime_placements (
    owner               TEXT NOT NULL,
    project             TEXT NOT NULL,
    mode                TEXT NOT NULL DEFAULT 'shared',
    target              TEXT NOT NULL DEFAULT 'local',
    worker_id           TEXT,
    created_at          INTEGER NOT NULL DEFAULT 0,
    updated_at          INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project)
);
CREATE TABLE IF NOT EXISTS project_operations (
    owner               TEXT NOT NULL,
    project             TEXT NOT NULL,
    operation_id        TEXT NOT NULL,
    kind                TEXT NOT NULL DEFAULT '',
    status              TEXT NOT NULL DEFAULT 'pending',
    current_step        TEXT NOT NULL DEFAULT '',
    source_office_id    TEXT,
    target_office_id    TEXT,
    artifact_rel_path   TEXT,
    artifact_sha256     TEXT,
    artifact_bytes      INTEGER,
    error_message       TEXT,
    retry_count         INTEGER NOT NULL DEFAULT 0,
    created_at          INTEGER NOT NULL DEFAULT 0,
    updated_at          INTEGER NOT NULL DEFAULT 0,
    completed_at        INTEGER,
    PRIMARY KEY (owner, project, operation_id)
);
CREATE INDEX IF NOT EXISTS idx_project_operations_project
    ON project_operations (owner, project, updated_at DESC, operation_id ASC);
CREATE TABLE IF NOT EXISTS mcp_sessions (
    token              TEXT PRIMARY KEY,
    owner              TEXT NOT NULL DEFAULT '',
    project            TEXT NOT NULL DEFAULT '',
    capabilities_json  TEXT NOT NULL DEFAULT '[]',
    created_at         INTEGER NOT NULL DEFAULT 0,
    auto_reset_seconds INTEGER,
    enabled            INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE IF NOT EXISTS pipeline_invocations (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    owner         TEXT NOT NULL,
    project       TEXT NOT NULL,
    file_rel_path TEXT NOT NULL,
    run_id        TEXT NOT NULL DEFAULT '',
    at            INTEGER NOT NULL DEFAULT 0,
    duration_ms   INTEGER NOT NULL DEFAULT 0,
    status        TEXT NOT NULL DEFAULT '',
    trigger       TEXT NOT NULL DEFAULT '',
    error         TEXT,
    trace_json    TEXT NOT NULL DEFAULT '[]'
);
CREATE INDEX IF NOT EXISTS idx_pipeline_invocations_pipeline
    ON pipeline_invocations (owner, project, file_rel_path, at DESC);
";

/// Platform catalog adapter backed by a single WAL-mode SQLite file.
pub struct SqliteDataAdapter {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteDataAdapter {
    /// Opens or creates `{data_root}/platform/catalog.db` with WAL mode and
    /// runs DDL migrations (`CREATE TABLE IF NOT EXISTS`).
    pub fn new(data_root: &Path) -> Result<Self, PlatformError> {
        std::fs::create_dir_all(data_root.join("platform"))?;
        let path = data_root.join("platform").join("catalog.db");
        let conn = Connection::open(&path)
            .map_err(|e| PlatformError::new("PLATFORM_SQLITE_OPEN", e.to_string()))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| PlatformError::new("PLATFORM_SQLITE_PRAGMA", e.to_string()))?;
        conn.execute_batch(SCHEMA_SQL)
            .map_err(|e| PlatformError::new("PLATFORM_SQLITE_SCHEMA", e.to_string()))?;
        Self::ensure_pipeline_invocation_schema(&conn)?;
        Self::ensure_marketplace_schema(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn qe(e: rusqlite::Error) -> PlatformError {
        PlatformError::new("PLATFORM_SQLITE", e.to_string())
    }

    fn json_error(e: serde_json::Error) -> PlatformError {
        PlatformError::new("PLATFORM_SQLITE_JSON", e.to_string())
    }

    fn ensure_table_column(
        conn: &Connection,
        table: &str,
        column: &str,
        definition: &str,
    ) -> Result<(), PlatformError> {
        let pragma_sql = format!("PRAGMA table_info({table})");
        let mut stmt = conn.prepare(&pragma_sql).map_err(Self::qe)?;
        let exists = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(Self::qe)?
            .filter_map(|row| row.ok())
            .any(|name| name == column);
        if exists {
            return Ok(());
        }
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
            [],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn ensure_pipeline_invocation_schema(conn: &Connection) -> Result<(), PlatformError> {
        Self::ensure_table_column(
            conn,
            "pipeline_invocations",
            "run_id",
            "TEXT NOT NULL DEFAULT ''",
        )
    }

    fn ensure_marketplace_schema(conn: &Connection) -> Result<(), PlatformError> {
        Self::ensure_table_column(
            conn,
            "marketplace_asset_packages",
            "authority_owner",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        Self::ensure_table_column(
            conn,
            "marketplace_asset_packages",
            "authority_project",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        Self::ensure_table_column(
            conn,
            "marketplace_asset_packages",
            "publisher_id",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        Self::ensure_table_column(
            conn,
            "marketplace_asset_packages",
            "publisher_display_name",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        Self::ensure_table_column(
            conn,
            "marketplace_asset_packages",
            "publisher_url",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        Self::ensure_table_column(
            conn,
            "marketplace_asset_packages",
            "publisher_email",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        Self::ensure_table_column(
            conn,
            "marketplace_asset_versions",
            "authority_owner",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        Self::ensure_table_column(
            conn,
            "marketplace_asset_versions",
            "authority_project",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        Self::ensure_table_column(
            conn,
            "marketplace_asset_versions",
            "publisher_id",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        Self::ensure_table_column(
            conn,
            "marketplace_tokens",
            "project",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        Self::ensure_table_column(
            conn,
            "marketplace_tokens",
            "publisher_id",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        Self::ensure_table_column(
            conn,
            "marketplace_tokens",
            "publisher_display_name",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        Self::ensure_table_column(
            conn,
            "marketplace_tokens",
            "publisher_url",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        Self::ensure_table_column(
            conn,
            "marketplace_tokens",
            "publisher_email",
            "TEXT NOT NULL DEFAULT ''",
        )
    }

    fn decode_project_operation_kind(raw: &str) -> Result<ProjectOperationKind, PlatformError> {
        match raw {
            "export_bundle" => Ok(ProjectOperationKind::ExportBundle),
            "export_files" => Ok(ProjectOperationKind::ExportFiles),
            "import_bundle" => Ok(ProjectOperationKind::ImportBundle),
            "import_files" => Ok(ProjectOperationKind::ImportFiles),
            other => Err(PlatformError::new(
                "PLATFORM_SQLITE_DECODE",
                format!("unknown project operation kind '{other}'"),
            )),
        }
    }

    fn decode_project_operation_status(raw: &str) -> Result<ProjectOperationStatus, PlatformError> {
        match raw {
            "pending" => Ok(ProjectOperationStatus::Pending),
            "running" => Ok(ProjectOperationStatus::Running),
            "failed" => Ok(ProjectOperationStatus::Failed),
            "completed" => Ok(ProjectOperationStatus::Completed),
            other => Err(PlatformError::new(
                "PLATFORM_SQLITE_DECODE",
                format!("unknown project operation status '{other}'"),
            )),
        }
    }
}

impl DataAdapter for SqliteDataAdapter {
    fn id(&self) -> &'static str {
        "data.sqlite"
    }

    // ──────────────────────── Users ────────────────────────

    fn get_user_auth(&self, owner: &str) -> Result<Option<StoredUser>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let result = conn.query_row(
            "SELECT owner, role, git_name, git_email, password, created_at, updated_at
             FROM users WHERE owner = ?1",
            params![owner],
            |row| {
                Ok(StoredUser {
                    profile: PlatformUser {
                        owner: row.get(0)?,
                        role: row.get(1)?,
                        git_name: row.get(2)?,
                        git_email: row.get(3)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    },
                    password: row.get(4)?,
                })
            },
        );
        match result {
            Ok(u) => Ok(Some(u)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Self::qe(e)),
        }
    }

    fn put_user(&self, user: &StoredUser) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT OR REPLACE INTO users
             (owner, role, git_name, git_email, password, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &user.profile.owner,
                &user.profile.role,
                &user.profile.git_name,
                &user.profile.git_email,
                &user.password,
                user.profile.created_at,
                user.profile.updated_at,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn list_users(&self) -> Result<Vec<PlatformUser>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT owner, role, git_name, git_email, created_at, updated_at
                 FROM users ORDER BY owner ASC",
            )
            .map_err(Self::qe)?;
        let users = stmt
            .query_map([], |row| {
                Ok(PlatformUser {
                    owner: row.get(0)?,
                    role: row.get(1)?,
                    git_name: row.get(2)?,
                    git_email: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })
            .map_err(Self::qe)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(users)
    }

    // ──────────────────────── Projects ────────────────────────

    fn get_project(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Option<PlatformProject>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let result = conn.query_row(
            "SELECT owner, project, created_at, updated_at
             FROM projects WHERE owner = ?1 AND project = ?2",
            params![owner, project],
            |row| {
                Ok(PlatformProject {
                    owner: row.get(0)?,
                    project: row.get(1)?,
                    title: String::new(), // populated from zebflow.json by ProjectService
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            },
        );
        match result {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Self::qe(e)),
        }
    }

    fn put_project(&self, project: &PlatformProject) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT OR REPLACE INTO projects (owner, project, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                &project.owner,
                &project.project,
                project.created_at,
                project.updated_at
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn list_projects(&self, owner: &str) -> Result<Vec<PlatformProject>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT owner, project, created_at, updated_at
                 FROM projects WHERE owner = ?1 ORDER BY project ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map(params![owner], |row| {
                Ok(PlatformProject {
                    owner: row.get(0)?,
                    project: row.get(1)?,
                    title: String::new(),
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            })
            .map_err(Self::qe)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(items)
    }

    fn delete_project(&self, owner: &str, project: &str) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "DELETE FROM projects WHERE owner = ?1 AND project = ?2",
            params![owner, project],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    // ──────────────────────── Credentials ────────────────────────

    fn get_project_credential(
        &self,
        owner: &str,
        project: &str,
        credential_id: &str,
    ) -> Result<Option<ProjectCredential>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let result = conn.query_row(
            "SELECT owner, project, credential_id, title, kind, secret_json, notes, created_at, updated_at
             FROM project_credentials WHERE owner = ?1 AND project = ?2 AND credential_id = ?3",
            params![owner, project, credential_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                ))
            },
        );
        match result {
            Ok((
                owner,
                project,
                credential_id,
                title,
                kind,
                secret_json,
                notes,
                created_at,
                updated_at,
            )) => {
                let secret = serde_json::from_str::<Value>(&secret_json).unwrap_or(Value::Null);
                Ok(Some(ProjectCredential {
                    owner,
                    project,
                    credential_id,
                    title,
                    kind,
                    secret,
                    notes,
                    created_at,
                    updated_at,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Self::qe(e)),
        }
    }

    fn put_project_credential(&self, cred: &ProjectCredential) -> Result<(), PlatformError> {
        let secret_json = serde_json::to_string(&cred.secret)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT OR REPLACE INTO project_credentials
             (owner, project, credential_id, title, kind, secret_json, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                &cred.owner,
                &cred.project,
                &cred.credential_id,
                &cred.title,
                &cred.kind,
                &secret_json,
                &cred.notes,
                cred.created_at,
                cred.updated_at,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn list_project_credentials(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectCredential>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT owner, project, credential_id, title, kind, secret_json, notes, created_at, updated_at
                 FROM project_credentials WHERE owner = ?1 AND project = ?2
                 ORDER BY credential_id ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map(params![owner, project], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                ))
            })
            .map_err(Self::qe)?
            .filter_map(|r| r.ok())
            .map(
                |(
                    owner,
                    project,
                    credential_id,
                    title,
                    kind,
                    secret_json,
                    notes,
                    created_at,
                    updated_at,
                )| {
                    let secret = serde_json::from_str::<Value>(&secret_json).unwrap_or(Value::Null);
                    ProjectCredential {
                        owner,
                        project,
                        credential_id,
                        title,
                        kind,
                        secret,
                        notes,
                        created_at,
                        updated_at,
                    }
                },
            )
            .collect();
        Ok(items)
    }

    fn delete_project_credential(
        &self,
        owner: &str,
        project: &str,
        credential_id: &str,
    ) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "DELETE FROM project_credentials WHERE owner = ?1 AND project = ?2 AND credential_id = ?3",
            params![owner, project, credential_id],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    // ──────────────────────── DB Connections ────────────────────────

    fn get_project_db_connection(
        &self,
        owner: &str,
        project: &str,
        connection_slug: &str,
    ) -> Result<Option<ProjectDbConnection>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let result = conn.query_row(
            "SELECT owner, project, connection_id, connection_slug, connection_label,
                    database_kind, credential_id, config_json, created_at, updated_at
             FROM project_db_connections
             WHERE owner = ?1 AND project = ?2 AND connection_slug = ?3",
            params![owner, project, connection_slug],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                ))
            },
        );
        match result {
            Ok((
                owner,
                project,
                connection_id,
                connection_slug,
                connection_label,
                database_kind,
                credential_id,
                config_json,
                created_at,
                updated_at,
            )) => {
                let config = serde_json::from_str::<Value>(&config_json).unwrap_or(Value::Null);
                Ok(Some(ProjectDbConnection {
                    owner,
                    project,
                    connection_id,
                    connection_slug,
                    connection_label,
                    database_kind,
                    credential_id,
                    config,
                    created_at,
                    updated_at,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Self::qe(e)),
        }
    }

    fn put_project_db_connection(
        &self,
        conn_rec: &ProjectDbConnection,
    ) -> Result<(), PlatformError> {
        let config_json = serde_json::to_string(&conn_rec.config)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT OR REPLACE INTO project_db_connections
             (owner, project, connection_id, connection_slug, connection_label,
              database_kind, credential_id, config_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                &conn_rec.owner,
                &conn_rec.project,
                &conn_rec.connection_id,
                &conn_rec.connection_slug,
                &conn_rec.connection_label,
                &conn_rec.database_kind,
                conn_rec.credential_id.as_deref(),
                &config_json,
                conn_rec.created_at,
                conn_rec.updated_at,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn list_project_db_connections(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectDbConnection>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT owner, project, connection_id, connection_slug, connection_label,
                        database_kind, credential_id, config_json, created_at, updated_at
                 FROM project_db_connections WHERE owner = ?1 AND project = ?2
                 ORDER BY connection_slug ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map(params![owner, project], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                ))
            })
            .map_err(Self::qe)?
            .filter_map(|r| r.ok())
            .map(
                |(
                    owner,
                    project,
                    connection_id,
                    connection_slug,
                    connection_label,
                    database_kind,
                    credential_id,
                    config_json,
                    created_at,
                    updated_at,
                )| {
                    let config = serde_json::from_str::<Value>(&config_json).unwrap_or(Value::Null);
                    ProjectDbConnection {
                        owner,
                        project,
                        connection_id,
                        connection_slug,
                        connection_label,
                        database_kind,
                        credential_id,
                        config,
                        created_at,
                        updated_at,
                    }
                },
            )
            .collect();
        Ok(items)
    }

    fn delete_project_db_connection(
        &self,
        owner: &str,
        project: &str,
        connection_slug: &str,
    ) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "DELETE FROM project_db_connections
             WHERE owner = ?1 AND project = ?2 AND connection_slug = ?3",
            params![owner, project, connection_slug],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn put_project_marketplace_repository(
        &self,
        repository: &ProjectMarketplaceRepository,
    ) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT OR REPLACE INTO project_marketplace_repositories
             (owner, project, repository_id, title, base_url, remote_owner, remote_project, read_token, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                &repository.owner,
                &repository.project,
                &repository.repository_id,
                &repository.title,
                &repository.base_url,
                &repository.remote_owner,
                &repository.remote_project,
                &repository.read_token,
                if repository.enabled { 1 } else { 0 },
                repository.created_at,
                repository.updated_at,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn list_project_marketplace_repositories(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectMarketplaceRepository>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT owner, project, repository_id, title, base_url, remote_owner, remote_project, read_token, enabled, created_at, updated_at
                 FROM project_marketplace_repositories WHERE owner = ?1 AND project = ?2
                 ORDER BY title ASC, repository_id ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map(params![owner, project], |row| {
                Ok(ProjectMarketplaceRepository {
                    owner: row.get(0)?,
                    project: row.get(1)?,
                    repository_id: row.get(2)?,
                    title: row.get(3)?,
                    base_url: row.get(4)?,
                    remote_owner: row.get(5)?,
                    remote_project: row.get(6)?,
                    read_token: row.get(7)?,
                    enabled: row.get::<_, i64>(8)? != 0,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(Self::qe)?
            .filter_map(Result::ok)
            .collect();
        Ok(items)
    }

    fn delete_project_marketplace_repository(
        &self,
        owner: &str,
        project: &str,
        repository_id: &str,
    ) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "DELETE FROM project_marketplace_repositories
             WHERE owner = ?1 AND project = ?2 AND repository_id = ?3",
            params![owner, project, repository_id],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn put_platform_marketplace_repository(
        &self,
        repository: &PlatformMarketplaceRepository,
    ) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT OR REPLACE INTO platform_marketplace_repositories
             (owner, repository_id, title, base_url, remote_owner, remote_project, read_token, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                &repository.owner,
                &repository.repository_id,
                &repository.title,
                &repository.base_url,
                &repository.remote_owner,
                &repository.remote_project,
                &repository.read_token,
                if repository.enabled { 1 } else { 0 },
                repository.created_at,
                repository.updated_at,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn list_platform_marketplace_repositories(
        &self,
        owner: &str,
    ) -> Result<Vec<PlatformMarketplaceRepository>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT owner, repository_id, title, base_url, remote_owner, remote_project, read_token, enabled, created_at, updated_at
                 FROM platform_marketplace_repositories WHERE owner = ?1
                 ORDER BY title ASC, repository_id ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map(params![owner], |row| {
                Ok(PlatformMarketplaceRepository {
                    owner: row.get(0)?,
                    repository_id: row.get(1)?,
                    title: row.get(2)?,
                    base_url: row.get(3)?,
                    remote_owner: row.get(4)?,
                    remote_project: row.get(5)?,
                    read_token: row.get(6)?,
                    enabled: row.get::<_, i64>(7)? != 0,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })
            .map_err(Self::qe)?
            .filter_map(Result::ok)
            .collect();
        Ok(items)
    }

    fn delete_platform_marketplace_repository(
        &self,
        owner: &str,
        repository_id: &str,
    ) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "DELETE FROM platform_marketplace_repositories
             WHERE owner = ?1 AND repository_id = ?2",
            params![owner, repository_id],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn put_marketplace_publisher(
        &self,
        publisher: &MarketplacePublisher,
    ) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT OR REPLACE INTO marketplace_publishers
             (owner, project, publisher_id, display_name, publisher_url, email, description, icon_url, website_url, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                &publisher.owner,
                &publisher.project,
                &publisher.publisher_id,
                &publisher.display_name,
                &publisher.publisher_url,
                &publisher.email,
                &publisher.description,
                &publisher.icon_url,
                &publisher.website_url,
                if publisher.enabled { 1 } else { 0 },
                publisher.created_at,
                publisher.updated_at,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn list_marketplace_publishers(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<MarketplacePublisher>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT owner, project, publisher_id, display_name, publisher_url, email, description, icon_url, website_url, enabled, created_at, updated_at
                 FROM marketplace_publishers WHERE owner = ?1 AND project = ?2
                 ORDER BY display_name ASC, publisher_id ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map(params![owner, project], |row| {
                Ok(MarketplacePublisher {
                    owner: row.get(0)?,
                    project: row.get(1)?,
                    publisher_id: row.get(2)?,
                    display_name: row.get(3)?,
                    publisher_url: row.get(4)?,
                    email: row.get(5)?,
                    description: row.get(6)?,
                    icon_url: row.get(7)?,
                    website_url: row.get(8)?,
                    enabled: row.get::<_, i64>(9)? != 0,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .map_err(Self::qe)?
            .filter_map(Result::ok)
            .collect();
        Ok(items)
    }

    fn get_marketplace_publisher(
        &self,
        owner: &str,
        project: &str,
        publisher_id: &str,
    ) -> Result<Option<MarketplacePublisher>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT owner, project, publisher_id, display_name, publisher_url, email, description, icon_url, website_url, enabled, created_at, updated_at
                 FROM marketplace_publishers WHERE owner = ?1 AND project = ?2 AND publisher_id = ?3",
            )
            .map_err(Self::qe)?;
        match stmt.query_row(params![owner, project, publisher_id], |row| {
            Ok(MarketplacePublisher {
                owner: row.get(0)?,
                project: row.get(1)?,
                publisher_id: row.get(2)?,
                display_name: row.get(3)?,
                publisher_url: row.get(4)?,
                email: row.get(5)?,
                description: row.get(6)?,
                icon_url: row.get(7)?,
                website_url: row.get(8)?,
                enabled: row.get::<_, i64>(9)? != 0,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        }) {
            Ok(item) => Ok(Some(item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Self::qe(e)),
        }
    }

    fn delete_marketplace_publisher(
        &self,
        owner: &str,
        project: &str,
        publisher_id: &str,
    ) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "DELETE FROM marketplace_publishers WHERE owner = ?1 AND project = ?2 AND publisher_id = ?3",
            params![owner, project, publisher_id],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn put_marketplace_asset_package(
        &self,
        package: &MarketplaceAssetPackage,
    ) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let tags_json = serde_json::to_string(&package.tags).map_err(Self::json_error)?;
        conn.execute(
            "INSERT OR REPLACE INTO marketplace_asset_packages
             (package_id, authority_owner, authority_project, publisher_owner, publisher_id, publisher_display_name, publisher_url, publisher_email, asset_kind, title, description, visibility, tags_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                &package.package_id,
                &package.authority_owner,
                &package.authority_project,
                &package.publisher_owner,
                &package.publisher_id,
                &package.publisher_display_name,
                &package.publisher_url,
                &package.publisher_email,
                &package.asset_kind,
                &package.title,
                &package.description,
                &package.visibility,
                &tags_json,
                package.created_at,
                package.updated_at,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn list_marketplace_asset_packages(&self) -> Result<Vec<MarketplaceAssetPackage>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT package_id, authority_owner, authority_project, publisher_owner, publisher_id, publisher_display_name, publisher_url, publisher_email, asset_kind, title, description, visibility, tags_json, created_at, updated_at
                 FROM marketplace_asset_packages
                 ORDER BY updated_at DESC, package_id ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, i64>(13)?,
                    row.get::<_, i64>(14)?,
                ))
            })
            .map_err(Self::qe)?
            .filter_map(|r| r.ok())
            .map(|(package_id, authority_owner, authority_project, publisher_owner, publisher_id, publisher_display_name, publisher_url, publisher_email, asset_kind, title, description, visibility, tags_json, created_at, updated_at)| MarketplaceAssetPackage {
                package_id,
                authority_owner,
                authority_project,
                publisher_owner,
                publisher_id,
                publisher_display_name,
                publisher_url,
                publisher_email,
                asset_kind,
                title,
                description,
                visibility,
                tags: serde_json::from_str::<Vec<String>>(&tags_json).unwrap_or_default(),
                created_at,
                updated_at,
            })
            .collect();
        Ok(items)
    }

    fn get_marketplace_asset_package(
        &self,
        package_id: &str,
    ) -> Result<Option<MarketplaceAssetPackage>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT package_id, authority_owner, authority_project, publisher_owner, publisher_id, publisher_display_name, publisher_url, publisher_email, asset_kind, title, description, visibility, tags_json, created_at, updated_at
                 FROM marketplace_asset_packages WHERE package_id = ?1",
            )
            .map_err(Self::qe)?;
        match stmt.query_row(params![package_id], |row| {
            Ok(MarketplaceAssetPackage {
                package_id: row.get(0)?,
                authority_owner: row.get(1)?,
                authority_project: row.get(2)?,
                publisher_owner: row.get(3)?,
                publisher_id: row.get(4)?,
                publisher_display_name: row.get(5)?,
                publisher_url: row.get(6)?,
                publisher_email: row.get(7)?,
                asset_kind: row.get(8)?,
                title: row.get(9)?,
                description: row.get(10)?,
                visibility: row.get(11)?,
                tags: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(12)?).unwrap_or_default(),
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        }) {
            Ok(item) => Ok(Some(item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Self::qe(e)),
        }
    }

    fn put_marketplace_asset_version(
        &self,
        version: &MarketplaceAssetVersion,
    ) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let manifest_json = serde_json::to_string(&version.manifest).map_err(Self::json_error)?;
        conn.execute(
            "INSERT OR REPLACE INTO marketplace_asset_versions
             (package_id, version, authority_owner, authority_project, publisher_owner, publisher_id, source_owner, source_project, source_kind, source_ref, artifact_rel_path, artifact_sha256, manifest_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                &version.package_id,
                &version.version,
                &version.authority_owner,
                &version.authority_project,
                &version.publisher_owner,
                &version.publisher_id,
                &version.source_owner,
                &version.source_project,
                &version.source_kind,
                &version.source_ref,
                &version.artifact_rel_path,
                &version.artifact_sha256,
                &manifest_json,
                version.created_at,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn list_marketplace_asset_versions(
        &self,
        package_id: &str,
    ) -> Result<Vec<MarketplaceAssetVersion>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT package_id, version, authority_owner, authority_project, publisher_owner, publisher_id, source_owner, source_project, source_kind, source_ref, artifact_rel_path, artifact_sha256, manifest_json, created_at
                 FROM marketplace_asset_versions WHERE package_id = ?1
                 ORDER BY created_at DESC, version DESC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map(params![package_id], |row| {
                Ok(MarketplaceAssetVersion {
                    package_id: row.get(0)?,
                    version: row.get(1)?,
                    authority_owner: row.get(2)?,
                    authority_project: row.get(3)?,
                    publisher_owner: row.get(4)?,
                    publisher_id: row.get(5)?,
                    source_owner: row.get(6)?,
                    source_project: row.get(7)?,
                    source_kind: row.get(8)?,
                    source_ref: row.get(9)?,
                    artifact_rel_path: row.get(10)?,
                    artifact_sha256: row.get(11)?,
                    manifest: serde_json::from_str::<Value>(&row.get::<_, String>(12)?).unwrap_or(Value::Null),
                    created_at: row.get(13)?,
                })
            })
            .map_err(Self::qe)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(items)
    }

    fn get_marketplace_asset_version(
        &self,
        package_id: &str,
        version: &str,
    ) -> Result<Option<MarketplaceAssetVersion>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT package_id, version, authority_owner, authority_project, publisher_owner, publisher_id, source_owner, source_project, source_kind, source_ref, artifact_rel_path, artifact_sha256, manifest_json, created_at
                 FROM marketplace_asset_versions WHERE package_id = ?1 AND version = ?2",
            )
            .map_err(Self::qe)?;
        match stmt.query_row(params![package_id, version], |row| {
            Ok(MarketplaceAssetVersion {
                package_id: row.get(0)?,
                version: row.get(1)?,
                authority_owner: row.get(2)?,
                authority_project: row.get(3)?,
                publisher_owner: row.get(4)?,
                publisher_id: row.get(5)?,
                source_owner: row.get(6)?,
                source_project: row.get(7)?,
                source_kind: row.get(8)?,
                source_ref: row.get(9)?,
                artifact_rel_path: row.get(10)?,
                artifact_sha256: row.get(11)?,
                manifest: serde_json::from_str::<Value>(&row.get::<_, String>(12)?).unwrap_or(Value::Null),
                created_at: row.get(13)?,
            })
        }) {
            Ok(item) => Ok(Some(item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Self::qe(e)),
        }
    }

    fn put_marketplace_token(&self, token: &MarketplaceToken) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let scopes_json = serde_json::to_string(&token.scopes).map_err(Self::json_error)?;
        conn.execute(
            "INSERT OR REPLACE INTO marketplace_tokens
             (token_id, owner, project, publisher_id, publisher_display_name, publisher_url, publisher_email, title, secret_hash, scopes_json, expires_at, last_used_at, revoked_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                &token.token_id,
                &token.owner,
                &token.project,
                &token.publisher_id,
                &token.publisher_display_name,
                &token.publisher_url,
                &token.publisher_email,
                &token.title,
                &token.secret_hash,
                &scopes_json,
                token.expires_at,
                token.last_used_at,
                token.revoked_at,
                token.created_at,
                token.updated_at,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn get_marketplace_token(&self, token_id: &str) -> Result<Option<MarketplaceToken>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT token_id, owner, project, publisher_id, publisher_display_name, publisher_url, publisher_email, title, secret_hash, scopes_json, expires_at, last_used_at, revoked_at, created_at, updated_at
                 FROM marketplace_tokens WHERE token_id = ?1",
            )
            .map_err(Self::qe)?;
        match stmt.query_row(params![token_id], |row| {
            Ok(MarketplaceToken {
                token_id: row.get(0)?,
                owner: row.get(1)?,
                project: row.get(2)?,
                publisher_id: row.get(3)?,
                publisher_display_name: row.get(4)?,
                publisher_url: row.get(5)?,
                publisher_email: row.get(6)?,
                title: row.get(7)?,
                secret_hash: row.get(8)?,
                scopes: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(9)?).unwrap_or_default(),
                expires_at: row.get(10)?,
                last_used_at: row.get(11)?,
                revoked_at: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        }) {
            Ok(item) => Ok(Some(item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Self::qe(e)),
        }
    }

    fn list_marketplace_tokens(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<MarketplaceToken>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT token_id, owner, project, publisher_id, publisher_display_name, publisher_url, publisher_email, title, secret_hash, scopes_json, expires_at, last_used_at, revoked_at, created_at, updated_at
                 FROM marketplace_tokens WHERE owner = ?1 AND project = ?2
                 ORDER BY updated_at DESC, token_id ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map(params![owner, project], |row| {
                Ok(MarketplaceToken {
                    token_id: row.get(0)?,
                    owner: row.get(1)?,
                    project: row.get(2)?,
                    publisher_id: row.get(3)?,
                    publisher_display_name: row.get(4)?,
                    publisher_url: row.get(5)?,
                    publisher_email: row.get(6)?,
                    title: row.get(7)?,
                    secret_hash: row.get(8)?,
                    scopes: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(9)?).unwrap_or_default(),
                    expires_at: row.get(10)?,
                    last_used_at: row.get(11)?,
                    revoked_at: row.get(12)?,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                })
            })
            .map_err(Self::qe)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(items)
    }

    fn delete_marketplace_token(&self, token_id: &str) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "DELETE FROM marketplace_tokens WHERE token_id = ?1",
            params![token_id],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    // ──────────────────────── Pipeline Metadata ────────────────────────

    fn put_pipeline_meta(&self, meta: &PipelineMeta) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT OR REPLACE INTO pipeline_meta
             (owner, project, file_rel_path, name, title, virtual_path, description,
              trigger_kind, hash, active_hash, activated_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                &meta.owner,
                &meta.project,
                &meta.file_rel_path,
                &meta.name,
                &meta.title,
                &meta.virtual_path,
                &meta.description,
                &meta.trigger_kind,
                &meta.hash,
                meta.active_hash.as_deref(),
                meta.activated_at,
                meta.created_at,
                meta.updated_at,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn delete_pipeline_meta(
        &self,
        owner: &str,
        project: &str,
        file_rel_path: &str,
    ) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "DELETE FROM pipeline_meta WHERE owner = ?1 AND project = ?2 AND file_rel_path = ?3",
            params![owner, project, file_rel_path],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn list_pipeline_meta(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<PipelineMeta>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT owner, project, file_rel_path, name, title, virtual_path, description,
                        trigger_kind, hash, active_hash, activated_at, created_at, updated_at
                 FROM pipeline_meta WHERE owner = ?1 AND project = ?2
                 ORDER BY file_rel_path ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map(params![owner, project], |row| {
                Ok(PipelineMeta {
                    owner: row.get(0)?,
                    project: row.get(1)?,
                    file_rel_path: row.get(2)?,
                    name: row.get(3)?,
                    title: row.get(4)?,
                    virtual_path: row.get(5)?,
                    description: row.get(6)?,
                    trigger_kind: row.get(7)?,
                    hash: row.get(8)?,
                    active_hash: row.get(9)?,
                    activated_at: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            })
            .map_err(Self::qe)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(items)
    }

    // ──────────────────────── Policies ────────────────────────

    fn put_project_policy(&self, policy: &ProjectPolicy) -> Result<(), PlatformError> {
        let caps_json = serde_json::to_string(&policy.capabilities)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT OR REPLACE INTO project_policies
             (owner, project, policy_id, title, capabilities_json, managed, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &policy.owner,
                &policy.project,
                &policy.policy_id,
                &policy.title,
                &caps_json,
                policy.managed as i64,
                policy.created_at,
                policy.updated_at,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn list_project_policies(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectPolicy>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT owner, project, policy_id, title, capabilities_json, managed, created_at, updated_at
                 FROM project_policies WHERE owner = ?1 AND project = ?2
                 ORDER BY policy_id ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map(params![owner, project], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            })
            .map_err(Self::qe)?
            .filter_map(|r| r.ok())
            .filter_map(
                |(owner, project, policy_id, title, caps_json, managed, created_at, updated_at)| {
                    let capabilities: Vec<ProjectCapability> =
                        serde_json::from_str(&caps_json).unwrap_or_default();
                    Some(ProjectPolicy {
                        owner,
                        project,
                        policy_id,
                        title,
                        capabilities,
                        managed: managed != 0,
                        created_at,
                        updated_at,
                    })
                },
            )
            .collect();
        Ok(items)
    }

    fn delete_project_policy(
        &self,
        owner: &str,
        project: &str,
        policy_id: &str,
    ) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "DELETE FROM project_policies WHERE owner = ?1 AND project = ?2 AND policy_id = ?3",
            params![owner, project, policy_id],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn put_project_policy_binding(
        &self,
        binding: &ProjectPolicyBinding,
    ) -> Result<(), PlatformError> {
        let subject_kind = binding.subject_kind.key();
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT OR REPLACE INTO project_policy_bindings
             (owner, project, subject_kind, subject_id, policy_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &binding.owner,
                &binding.project,
                subject_kind,
                &binding.subject_id,
                &binding.policy_id,
                binding.created_at,
                binding.updated_at,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn list_project_policy_bindings(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectPolicyBinding>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT owner, project, subject_kind, subject_id, policy_id, created_at, updated_at
                 FROM project_policy_bindings WHERE owner = ?1 AND project = ?2
                 ORDER BY subject_kind ASC, subject_id ASC, policy_id ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map(params![owner, project], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                ))
            })
            .map_err(Self::qe)?
            .filter_map(|r| r.ok())
            .filter_map(
                |(
                    owner,
                    project,
                    subject_kind_str,
                    subject_id,
                    policy_id,
                    created_at,
                    updated_at,
                )| {
                    let subject_kind: ProjectSubjectKind =
                        serde_json::from_value(Value::String(subject_kind_str)).ok()?;
                    Some(ProjectPolicyBinding {
                        owner,
                        project,
                        subject_kind,
                        subject_id,
                        policy_id,
                        created_at,
                        updated_at,
                    })
                },
            )
            .collect();
        Ok(items)
    }

    fn delete_project_policy_binding(
        &self,
        owner: &str,
        project: &str,
        subject_id: &str,
    ) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "DELETE FROM project_policy_bindings
             WHERE owner = ?1 AND project = ?2 AND subject_id = ?3",
            params![owner, project, subject_id],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn get_project_member(
        &self,
        owner: &str,
        project: &str,
        user_id: &str,
    ) -> Result<Option<ProjectMember>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let result = conn.query_row(
            "SELECT owner, project, user_id, role_preset, custom_policy_ids_json, mcp_capabilities_json, created_by, created_at, updated_at
             FROM project_members WHERE owner = ?1 AND project = ?2 AND user_id = ?3",
            params![owner, project, user_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                ))
            },
        );
        match result {
            Ok((
                owner,
                project,
                user_id,
                role_preset,
                custom_policy_ids_json,
                mcp_capabilities_json,
                created_by,
                created_at,
                updated_at,
            )) => {
                let role_preset: ProjectAccessRolePreset =
                    serde_json::from_value(Value::String(role_preset)).unwrap_or_default();
                let custom_policy_ids: Vec<String> =
                    serde_json::from_str(&custom_policy_ids_json).unwrap_or_default();
                let mcp_capability_ceiling: Vec<ProjectCapability> =
                    serde_json::from_str(&mcp_capabilities_json).unwrap_or_default();
                Ok(Some(ProjectMember {
                    owner,
                    project,
                    user_id,
                    role_preset,
                    custom_policy_ids,
                    mcp_capability_ceiling,
                    created_by,
                    created_at,
                    updated_at,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Self::qe(e)),
        }
    }

    fn put_project_member(&self, member: &ProjectMember) -> Result<(), PlatformError> {
        let custom_policy_ids_json = serde_json::to_string(&member.custom_policy_ids)?;
        let mcp_capabilities_json = serde_json::to_string(&member.mcp_capability_ceiling)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT OR REPLACE INTO project_members
             (owner, project, user_id, role_preset, custom_policy_ids_json, mcp_capabilities_json, created_by, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                &member.owner,
                &member.project,
                &member.user_id,
                member.role_preset.key(),
                custom_policy_ids_json,
                mcp_capabilities_json,
                &member.created_by,
                member.created_at,
                member.updated_at,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn list_project_members(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectMember>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT owner, project, user_id, role_preset, custom_policy_ids_json, mcp_capabilities_json, created_by, created_at, updated_at
                 FROM project_members WHERE owner = ?1 AND project = ?2 ORDER BY user_id ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map(params![owner, project], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                ))
            })
            .map_err(Self::qe)?
            .filter_map(|r| r.ok())
            .filter_map(
                |(
                    owner,
                    project,
                    user_id,
                    role_preset,
                    custom_policy_ids_json,
                    mcp_capabilities_json,
                    created_by,
                    created_at,
                    updated_at,
                )| {
                    let role_preset: ProjectAccessRolePreset =
                        serde_json::from_value(Value::String(role_preset)).ok()?;
                    let custom_policy_ids: Vec<String> =
                        serde_json::from_str(&custom_policy_ids_json).unwrap_or_default();
                    let mcp_capability_ceiling: Vec<ProjectCapability> =
                        serde_json::from_str(&mcp_capabilities_json).unwrap_or_default();
                    Some(ProjectMember {
                        owner,
                        project,
                        user_id,
                        role_preset,
                        custom_policy_ids,
                        mcp_capability_ceiling,
                        created_by,
                        created_at,
                        updated_at,
                    })
                },
            )
            .collect();
        Ok(items)
    }

    fn delete_project_member(
        &self,
        owner: &str,
        project: &str,
        user_id: &str,
    ) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "DELETE FROM project_members WHERE owner = ?1 AND project = ?2 AND user_id = ?3",
            params![owner, project, user_id],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn get_project_invite(
        &self,
        owner: &str,
        project: &str,
        invite_id: &str,
    ) -> Result<Option<ProjectInvite>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let result = conn.query_row(
            "SELECT owner, project, invite_id, target_user, role_preset, custom_policy_ids_json, mcp_capabilities_json, note, invited_by, status, expires_at, created_at, updated_at
             FROM project_invites WHERE owner = ?1 AND project = ?2 AND invite_id = ?3",
            params![owner, project, invite_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, Option<i64>>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, i64>(12)?,
                ))
            },
        );
        match result {
            Ok((
                owner,
                project,
                invite_id,
                target_user,
                role_preset,
                custom_policy_ids_json,
                mcp_capabilities_json,
                note,
                invited_by,
                status,
                expires_at,
                created_at,
                updated_at,
            )) => {
                let role_preset: ProjectAccessRolePreset =
                    serde_json::from_value(Value::String(role_preset)).unwrap_or_default();
                let status: ProjectInviteStatus = serde_json::from_str(&status).unwrap_or_default();
                let custom_policy_ids: Vec<String> =
                    serde_json::from_str(&custom_policy_ids_json).unwrap_or_default();
                let mcp_capability_ceiling: Vec<ProjectCapability> =
                    serde_json::from_str(&mcp_capabilities_json).unwrap_or_default();
                Ok(Some(ProjectInvite {
                    owner,
                    project,
                    invite_id,
                    target_user,
                    role_preset,
                    custom_policy_ids,
                    mcp_capability_ceiling,
                    note,
                    invited_by,
                    status,
                    expires_at,
                    created_at,
                    updated_at,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Self::qe(e)),
        }
    }

    fn put_project_invite(&self, invite: &ProjectInvite) -> Result<(), PlatformError> {
        let custom_policy_ids_json = serde_json::to_string(&invite.custom_policy_ids)?;
        let mcp_capabilities_json = serde_json::to_string(&invite.mcp_capability_ceiling)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT OR REPLACE INTO project_invites
             (owner, project, invite_id, target_user, role_preset, custom_policy_ids_json, mcp_capabilities_json, note, invited_by, status, expires_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                &invite.owner,
                &invite.project,
                &invite.invite_id,
                &invite.target_user,
                invite.role_preset.key(),
                custom_policy_ids_json,
                mcp_capabilities_json,
                &invite.note,
                &invite.invited_by,
                serde_json::to_string(&invite.status)?,
                invite.expires_at,
                invite.created_at,
                invite.updated_at,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn list_project_invites(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectInvite>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT owner, project, invite_id, target_user, role_preset, custom_policy_ids_json, mcp_capabilities_json, note, invited_by, status, expires_at, created_at, updated_at
                 FROM project_invites WHERE owner = ?1 AND project = ?2 ORDER BY created_at DESC, invite_id ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map(params![owner, project], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, Option<i64>>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, i64>(12)?,
                ))
            })
            .map_err(Self::qe)?
            .filter_map(|r| r.ok())
            .filter_map(
                |(
                    owner,
                    project,
                    invite_id,
                    target_user,
                    role_preset,
                    custom_policy_ids_json,
                    mcp_capabilities_json,
                    note,
                    invited_by,
                    status,
                    expires_at,
                    created_at,
                    updated_at,
                )| {
                    let role_preset: ProjectAccessRolePreset =
                        serde_json::from_value(Value::String(role_preset)).ok()?;
                    let status: ProjectInviteStatus = serde_json::from_str(&status).ok()?;
                    let custom_policy_ids: Vec<String> =
                        serde_json::from_str(&custom_policy_ids_json).unwrap_or_default();
                    let mcp_capability_ceiling: Vec<ProjectCapability> =
                        serde_json::from_str(&mcp_capabilities_json).unwrap_or_default();
                    Some(ProjectInvite {
                        owner,
                        project,
                        invite_id,
                        target_user,
                        role_preset,
                        custom_policy_ids,
                        mcp_capability_ceiling,
                        note,
                        invited_by,
                        status,
                        expires_at,
                        created_at,
                        updated_at,
                    })
                },
            )
            .collect();
        Ok(items)
    }

    fn delete_project_invite(
        &self,
        owner: &str,
        project: &str,
        invite_id: &str,
    ) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "DELETE FROM project_invites WHERE owner = ?1 AND project = ?2 AND invite_id = ?3",
            params![owner, project, invite_id],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn get_worker_registry_record(
        &self,
        node_id: &str,
    ) -> Result<Option<WorkerRegistryRecord>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let result = conn.query_row(
            "SELECT node_id, label, base_url, status, capabilities_json, registered_at, last_heartbeat_at
             FROM worker_registry
             WHERE node_id = ?1",
            params![node_id],
            |row| {
                let capabilities_json: String = row.get(4)?;
                let capabilities = serde_json::from_str(&capabilities_json).unwrap_or_default();
                Ok(WorkerRegistryRecord {
                    node_id: row.get(0)?,
                    label: row.get(1)?,
                    base_url: row.get(2)?,
                    status: row.get(3)?,
                    capabilities,
                    registered_at: row.get(5)?,
                    last_heartbeat_at: row.get(6)?,
                })
            },
        );
        match result {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(Self::qe(err)),
        }
    }

    fn put_worker_registry_record(
        &self,
        record: &WorkerRegistryRecord,
    ) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let capabilities_json =
            serde_json::to_string(&record.capabilities).unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "INSERT OR REPLACE INTO worker_registry
             (node_id, label, base_url, status, capabilities_json, registered_at, last_heartbeat_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &record.node_id,
                &record.label,
                &record.base_url,
                &record.status,
                capabilities_json,
                record.registered_at,
                record.last_heartbeat_at,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn list_worker_registry_records(&self) -> Result<Vec<WorkerRegistryRecord>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT node_id, label, base_url, status, capabilities_json, registered_at, last_heartbeat_at
                 FROM worker_registry
                 ORDER BY node_id ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map([], |row| {
                let capabilities_json: String = row.get(4)?;
                let capabilities = serde_json::from_str(&capabilities_json).unwrap_or_default();
                Ok(WorkerRegistryRecord {
                    node_id: row.get(0)?,
                    label: row.get(1)?,
                    base_url: row.get(2)?,
                    status: row.get(3)?,
                    capabilities,
                    registered_at: row.get(5)?,
                    last_heartbeat_at: row.get(6)?,
                })
            })
            .map_err(Self::qe)?
            .filter_map(|row| row.ok())
            .collect();
        Ok(items)
    }

    fn delete_worker_registry_record(&self, node_id: &str) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "DELETE FROM worker_registry WHERE node_id = ?1",
            params![node_id],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn get_project_runtime_placement(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Option<ProjectRuntimePlacement>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let result = conn.query_row(
            "SELECT owner, project, mode, target, worker_id, created_at, updated_at
             FROM project_runtime_placements
             WHERE owner = ?1 AND project = ?2",
            params![owner, project],
            |row| {
                let mode: String = row.get(2)?;
                let target: String = row.get(3)?;
                Ok(ProjectRuntimePlacement {
                    owner: row.get(0)?,
                    project: row.get(1)?,
                    mode: serde_json::from_value(Value::String(mode))
                        .unwrap_or(ProjectRuntimeMode::Shared),
                    target: serde_json::from_value(Value::String(target))
                        .unwrap_or(ProjectRuntimePlacementTarget::Local),
                    worker_id: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        );
        match result {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(Self::qe(err)),
        }
    }

    fn put_project_runtime_placement(
        &self,
        placement: &ProjectRuntimePlacement,
    ) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mode = serde_json::to_value(placement.mode)
            .ok()
            .and_then(|value| value.as_str().map(ToString::to_string))
            .unwrap_or_else(|| "shared".to_string());
        let target = serde_json::to_value(placement.target)
            .ok()
            .and_then(|value| value.as_str().map(ToString::to_string))
            .unwrap_or_else(|| "local".to_string());
        conn.execute(
            "INSERT OR REPLACE INTO project_runtime_placements
             (owner, project, mode, target, worker_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &placement.owner,
                &placement.project,
                mode,
                target,
                &placement.worker_id,
                placement.created_at,
                placement.updated_at,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn list_project_runtime_placements(
        &self,
    ) -> Result<Vec<ProjectRuntimePlacement>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT owner, project, mode, target, worker_id, created_at, updated_at
                 FROM project_runtime_placements
                 ORDER BY owner ASC, project ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map([], |row| {
                let mode: String = row.get(2)?;
                let target: String = row.get(3)?;
                Ok(ProjectRuntimePlacement {
                    owner: row.get(0)?,
                    project: row.get(1)?,
                    mode: serde_json::from_value(Value::String(mode))
                        .unwrap_or(ProjectRuntimeMode::Shared),
                    target: serde_json::from_value(Value::String(target))
                        .unwrap_or(ProjectRuntimePlacementTarget::Local),
                    worker_id: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(Self::qe)?
            .filter_map(|row| row.ok())
            .collect();
        Ok(items)
    }

    fn delete_project_runtime_placement(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "DELETE FROM project_runtime_placements WHERE owner = ?1 AND project = ?2",
            params![owner, project],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn get_project_operation(
        &self,
        owner: &str,
        project: &str,
        operation_id: &str,
    ) -> Result<Option<ProjectOperationRecord>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let result = conn.query_row(
            "SELECT owner, project, operation_id, kind, status, current_step, source_office_id, target_office_id, artifact_rel_path, artifact_sha256, artifact_bytes, error_message, retry_count, created_at, updated_at, completed_at
             FROM project_operations
             WHERE owner = ?1 AND project = ?2 AND operation_id = ?3",
            params![owner, project, operation_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<u64>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, u32>(12)?,
                    row.get::<_, i64>(13)?,
                    row.get::<_, i64>(14)?,
                    row.get::<_, Option<i64>>(15)?,
                ))
            },
        );
        match result {
            Ok((
                owner,
                project,
                operation_id,
                kind,
                status,
                current_step,
                source_office_id,
                target_office_id,
                artifact_rel_path,
                artifact_sha256,
                artifact_bytes,
                error_message,
                retry_count,
                created_at,
                updated_at,
                completed_at,
            )) => Ok(Some(ProjectOperationRecord {
                owner,
                project,
                operation_id,
                kind: Self::decode_project_operation_kind(&kind)?,
                status: Self::decode_project_operation_status(&status)?,
                current_step,
                source_office_id,
                target_office_id,
                artifact_rel_path,
                artifact_sha256,
                artifact_bytes,
                error_message,
                retry_count,
                created_at,
                updated_at,
                completed_at,
            })),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(Self::qe(err)),
        }
    }

    fn put_project_operation(&self, record: &ProjectOperationRecord) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT OR REPLACE INTO project_operations
             (owner, project, operation_id, kind, status, current_step, source_office_id, target_office_id, artifact_rel_path, artifact_sha256, artifact_bytes, error_message, retry_count, created_at, updated_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                &record.owner,
                &record.project,
                &record.operation_id,
                record.kind.key(),
                record.status.key(),
                &record.current_step,
                &record.source_office_id,
                &record.target_office_id,
                &record.artifact_rel_path,
                &record.artifact_sha256,
                record.artifact_bytes,
                &record.error_message,
                record.retry_count,
                record.created_at,
                record.updated_at,
                record.completed_at,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn list_project_operations(
        &self,
        owner: &str,
        project: &str,
        limit: usize,
    ) -> Result<Vec<ProjectOperationRecord>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT owner, project, operation_id, kind, status, current_step, source_office_id, target_office_id, artifact_rel_path, artifact_sha256, artifact_bytes, error_message, retry_count, created_at, updated_at, completed_at
                 FROM project_operations
                 WHERE owner = ?1 AND project = ?2
                 ORDER BY updated_at DESC, operation_id DESC
                 LIMIT ?3",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map(params![owner, project, limit as i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<u64>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, u32>(12)?,
                    row.get::<_, i64>(13)?,
                    row.get::<_, i64>(14)?,
                    row.get::<_, Option<i64>>(15)?,
                ))
            })
            .map_err(Self::qe)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Self::qe)?;
        let mut out = Vec::with_capacity(items.len());
        for (
            owner,
            project,
            operation_id,
            kind,
            status,
            current_step,
            source_office_id,
            target_office_id,
            artifact_rel_path,
            artifact_sha256,
            artifact_bytes,
            error_message,
            retry_count,
            created_at,
            updated_at,
            completed_at,
        ) in items
        {
            out.push(ProjectOperationRecord {
                owner,
                project,
                operation_id,
                kind: Self::decode_project_operation_kind(&kind)?,
                status: Self::decode_project_operation_status(&status)?,
                current_step,
                source_office_id,
                target_office_id,
                artifact_rel_path,
                artifact_sha256,
                artifact_bytes,
                error_message,
                retry_count,
                created_at,
                updated_at,
                completed_at,
            });
        }
        Ok(out)
    }

    // ──────────────────────── MCP Sessions ────────────────────────

    fn list_all_mcp_sessions(&self) -> Result<Vec<McpSession>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT token, owner, project, capabilities_json, created_at, auto_reset_seconds, enabled
                 FROM mcp_sessions",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                    row.get::<_, i64>(6)?,
                ))
            })
            .map_err(Self::qe)?
            .filter_map(|r| r.ok())
            .filter_map(
                |(token, owner, project, caps_json, created_at, auto_reset_seconds, enabled)| {
                    let capabilities: Vec<ProjectCapability> =
                        serde_json::from_str(&caps_json).unwrap_or_default();
                    Some(McpSession {
                        token,
                        owner,
                        project,
                        capabilities,
                        created_at,
                        auto_reset_seconds: auto_reset_seconds.map(|s| s as u64),
                        enabled: enabled != 0,
                    })
                },
            )
            .collect();
        Ok(items)
    }

    fn put_mcp_session(&self, session: &McpSession) -> Result<(), PlatformError> {
        let caps_json = serde_json::to_string(&session.capabilities)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT OR REPLACE INTO mcp_sessions
             (token, owner, project, capabilities_json, created_at, auto_reset_seconds, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &session.token,
                &session.owner,
                &session.project,
                &caps_json,
                session.created_at,
                session.auto_reset_seconds.map(|s| s as i64),
                session.enabled as i64,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn delete_mcp_session(&self, token: &str) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute("DELETE FROM mcp_sessions WHERE token = ?1", params![token])
            .map_err(Self::qe)?;
        Ok(())
    }

    // ──────────────────────── Admin ────────────────────────

    fn admin_list_collections(&self) -> Result<Vec<(String, usize)>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let tables = [
            "users",
            "projects",
            "project_credentials",
            "project_db_connections",
            "pipeline_meta",
            "project_policies",
            "project_policy_bindings",
            "mcp_sessions",
            "pipeline_invocations",
        ];
        let mut out = Vec::with_capacity(tables.len());
        for table in &tables {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap_or(0);
            out.push((table.to_string(), count as usize));
        }
        Ok(out)
    }

    // ──────────────────────── Invocations ────────────────────────

    fn log_pipeline_invocation(
        &self,
        owner: &str,
        project: &str,
        file_rel_path: &str,
        entry: &PipelineInvocationEntry,
        max_n: usize,
    ) -> Result<(), PlatformError> {
        let trace_json = serde_json::to_string(&entry.trace)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO pipeline_invocations
             (owner, project, file_rel_path, run_id, at, duration_ms, status, trigger, error, trace_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                owner,
                project,
                file_rel_path,
                &entry.run_id,
                entry.at,
                entry.duration_ms as i64,
                &entry.status,
                &entry.trigger,
                entry.error.as_deref(),
                &trace_json,
            ],
        )
        .map_err(Self::qe)?;
        // Trim rows beyond max_n (keep the most-recent by `at`)
        conn.execute(
            "DELETE FROM pipeline_invocations
             WHERE owner = ?1 AND project = ?2 AND file_rel_path = ?3
               AND id NOT IN (
                 SELECT id FROM pipeline_invocations
                 WHERE owner = ?1 AND project = ?2 AND file_rel_path = ?3
                 ORDER BY at DESC LIMIT ?4
               )",
            params![owner, project, file_rel_path, max_n as i64],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn get_pipeline_invocations(
        &self,
        owner: &str,
        project: &str,
        file_rel_path: &str,
    ) -> Result<Vec<PipelineInvocationEntry>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT run_id, at, duration_ms, status, trigger, error, trace_json
                 FROM pipeline_invocations
                 WHERE owner = ?1 AND project = ?2 AND file_rel_path = ?3
                 ORDER BY at DESC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map(params![owner, project, file_rel_path], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })
            .map_err(Self::qe)?
            .filter_map(|r| r.ok())
            .map(
                |(run_id, at, duration_ms, status, trigger, error, trace_json)| {
                    let trace = serde_json::from_str(&trace_json).unwrap_or_default();
                    PipelineInvocationEntry {
                        run_id,
                        at,
                        duration_ms: duration_ms as u64,
                        status,
                        trigger,
                        error,
                        trace,
                    }
                },
            )
            .collect();
        Ok(items)
    }
}

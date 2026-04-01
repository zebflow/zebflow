//! SQLite-backed platform catalog adapter (WAL mode, bundled SQLite 3.47).
//!
//! Uses a single `catalog.db` file under `{data_root}/platform/` with proper
//! WAL journaling. Safe across K8s restarts — no unsafe mmap code.

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, params};
use serde_json::Value;

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    McpSession, PipelineInvocationEntry, PipelineMeta, PlatformProject, PlatformUser,
    ProjectCapability, ProjectCredential, ProjectDbConnection, ProjectPolicy, ProjectPolicyBinding,
    ProjectSubjectKind, StoredUser,
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
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn qe(e: rusqlite::Error) -> PlatformError {
        PlatformError::new("PLATFORM_SQLITE", e.to_string())
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
            Ok((owner, project, credential_id, title, kind, secret_json, notes, created_at, updated_at)) => {
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
                |(owner, project, credential_id, title, kind, secret_json, notes, created_at, updated_at)| {
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
                |(owner, project, subject_kind_str, subject_id, policy_id, created_at, updated_at)| {
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
        conn.execute(
            "DELETE FROM mcp_sessions WHERE token = ?1",
            params![token],
        )
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
             (owner, project, file_rel_path, at, duration_ms, status, trigger, error, trace_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                owner,
                project,
                file_rel_path,
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
                "SELECT at, duration_ms, status, trigger, error, trace_json
                 FROM pipeline_invocations
                 WHERE owner = ?1 AND project = ?2 AND file_rel_path = ?3
                 ORDER BY at DESC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map(params![owner, project, file_rel_path], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .map_err(Self::qe)?
            .filter_map(|r| r.ok())
            .map(|(at, duration_ms, status, trigger, error, trace_json)| {
                let trace = serde_json::from_str(&trace_json).unwrap_or_default();
                PipelineInvocationEntry {
                    at,
                    duration_ms: duration_ms as u64,
                    status,
                    trigger,
                    error,
                    trace,
                }
            })
            .collect();
        Ok(items)
    }
}

//! SQLite-backed platform catalog adapter (WAL mode, bundled SQLite 3.47).
//!
//! Uses a single `catalog.db` file under `{data_root}/platform/` with proper
//! WAL journaling. Safe across K8s restarts — no unsafe mmap code.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, Transaction, params};
use serde_json::Value;

use crate::infra::cluster::registry::WorkerRegistryRecord;
use crate::infra::execution::placement::{
    ProjectRuntimeMode, ProjectRuntimePlacement, ProjectRuntimePlacementTarget,
};
use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    MarketplaceAssetPackage, MarketplaceAssetVersion, MarketplaceAuthority, MarketplacePublisher,
    MarketplaceToken, McpSession, PipelineInvocationEntry, PipelineMeta, PlatformMarketplaceRepository,
    PlatformOffice, PlatformOfficeNode, PlatformProject, PlatformUser, PlatformUserLocalAuth,
    ProjectAccessRolePreset, ProjectCapability, ProjectCredential, ProjectDbConnection,
    ProjectInvite, ProjectInviteStatus, ProjectMarketplaceRepository, ProjectMember,
    ProjectOperationKind, ProjectOperationRecord, ProjectOperationStatus, ProjectPolicy,
    ProjectPolicyBinding, ProjectSubjectKind, StoredUser,
};

const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS users (
    owner         TEXT PRIMARY KEY,
    user_id       TEXT NOT NULL UNIQUE DEFAULT '',
    role          TEXT NOT NULL DEFAULT 'owner',
    git_name      TEXT NOT NULL DEFAULT '',
    git_email     TEXT NOT NULL DEFAULT '',
    created_at    INTEGER NOT NULL DEFAULT 0,
    updated_at    INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS user_local_auth (
    user_id              TEXT PRIMARY KEY,
    password_hash        TEXT NOT NULL DEFAULT '',
    password_alg         TEXT NOT NULL DEFAULT 'sha256',
    password_updated_at  INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (user_id) REFERENCES users(user_id)
        ON UPDATE CASCADE
        ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS projects (
    owner         TEXT NOT NULL,
    project       TEXT NOT NULL,
    project_id    TEXT NOT NULL UNIQUE DEFAULT '',
    owner_user_id TEXT NOT NULL DEFAULT '',
    created_at    INTEGER NOT NULL DEFAULT 0,
    updated_at    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project),
    FOREIGN KEY (owner_user_id) REFERENCES users(user_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
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
    source_id     TEXT NOT NULL UNIQUE DEFAULT '',
    owner_user_id TEXT NOT NULL DEFAULT '',
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
    PRIMARY KEY (owner, repository_id),
    FOREIGN KEY (owner_user_id) REFERENCES users(user_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS marketplace_authorities (
    authority_id     TEXT PRIMARY KEY,
    host_project_id  TEXT NOT NULL UNIQUE DEFAULT '',
    owner            TEXT NOT NULL DEFAULT '',
    project          TEXT NOT NULL DEFAULT '',
    enabled          INTEGER NOT NULL DEFAULT 0,
    public_base_url  TEXT NOT NULL DEFAULT '',
    created_at       INTEGER NOT NULL DEFAULT 0,
    updated_at       INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (host_project_id) REFERENCES projects(project_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS marketplace_publishers (
    authority_id    TEXT NOT NULL DEFAULT '',
    publisher_pk    TEXT NOT NULL UNIQUE DEFAULT '',
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
    PRIMARY KEY (owner, project, publisher_id),
    FOREIGN KEY (authority_id) REFERENCES marketplace_authorities(authority_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS marketplace_asset_packages (
    package_pk       TEXT NOT NULL UNIQUE DEFAULT '',
    authority_id     TEXT NOT NULL DEFAULT '',
    publisher_pk     TEXT NOT NULL DEFAULT '',
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
    updated_at       INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (authority_id) REFERENCES marketplace_authorities(authority_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (publisher_pk) REFERENCES marketplace_publishers(publisher_pk)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS marketplace_asset_versions (
    package_pk        TEXT NOT NULL DEFAULT '',
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
    PRIMARY KEY (package_id, version),
    FOREIGN KEY (package_pk) REFERENCES marketplace_asset_packages(package_pk)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS marketplace_tokens (
    token_id       TEXT PRIMARY KEY,
    authority_id   TEXT NOT NULL DEFAULT '',
    publisher_pk   TEXT NOT NULL DEFAULT '',
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
    updated_at     INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (authority_id) REFERENCES marketplace_authorities(authority_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (publisher_pk) REFERENCES marketplace_publishers(publisher_pk)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
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
    project_id        TEXT NOT NULL DEFAULT '',
    owner             TEXT NOT NULL,
    project           TEXT NOT NULL,
    policy_id         TEXT NOT NULL,
    title             TEXT NOT NULL DEFAULT '',
    capabilities_json TEXT NOT NULL DEFAULT '[]',
    managed           INTEGER NOT NULL DEFAULT 0,
    created_at        INTEGER NOT NULL DEFAULT 0,
    updated_at        INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, policy_id),
    UNIQUE (project_id, policy_id),
    FOREIGN KEY (project_id) REFERENCES projects(project_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS project_policy_bindings (
    project_id   TEXT NOT NULL DEFAULT '',
    owner        TEXT NOT NULL,
    project      TEXT NOT NULL,
    subject_kind TEXT NOT NULL,
    subject_id   TEXT NOT NULL,
    policy_id    TEXT NOT NULL,
    created_at   INTEGER NOT NULL DEFAULT 0,
    updated_at   INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, subject_id, policy_id),
    FOREIGN KEY (project_id) REFERENCES projects(project_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (project_id, policy_id) REFERENCES project_policies(project_id, policy_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS project_members (
    project_id              TEXT NOT NULL DEFAULT '',
    owner                  TEXT NOT NULL,
    project                TEXT NOT NULL,
    user_id                TEXT NOT NULL,
    member_user_id         TEXT NOT NULL DEFAULT '',
    role_preset            TEXT NOT NULL DEFAULT 'reporter',
    custom_policy_ids_json TEXT NOT NULL DEFAULT '[]',
    mcp_capabilities_json  TEXT NOT NULL DEFAULT '[]',
    created_by             TEXT NOT NULL DEFAULT '',
    created_by_user_id     TEXT NOT NULL DEFAULT '',
    created_at             INTEGER NOT NULL DEFAULT 0,
    updated_at             INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, user_id),
    FOREIGN KEY (project_id) REFERENCES projects(project_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (member_user_id) REFERENCES users(user_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (created_by_user_id) REFERENCES users(user_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
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
CREATE TABLE IF NOT EXISTS offices (
    office_id     TEXT PRIMARY KEY,
    office_slug   TEXT NOT NULL UNIQUE DEFAULT '',
    label         TEXT NOT NULL DEFAULT '',
    office_kind   TEXT NOT NULL DEFAULT 'office',
    base_url      TEXT NOT NULL DEFAULT '',
    status        TEXT NOT NULL DEFAULT '',
    created_at    INTEGER NOT NULL DEFAULT 0,
    updated_at    INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS office_nodes (
    node_id             TEXT PRIMARY KEY,
    office_id           TEXT NOT NULL DEFAULT '',
    label               TEXT NOT NULL DEFAULT '',
    base_url            TEXT NOT NULL DEFAULT '',
    status              TEXT NOT NULL DEFAULT '',
    capabilities_json   TEXT NOT NULL DEFAULT '{}',
    registered_at       INTEGER NOT NULL DEFAULT 0,
    last_heartbeat_at   INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (office_id) REFERENCES offices(office_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS worker_registry (
    office_id           TEXT NOT NULL DEFAULT '',
    office_slug         TEXT NOT NULL DEFAULT '',
    node_id             TEXT PRIMARY KEY,
    label               TEXT NOT NULL DEFAULT '',
    base_url            TEXT NOT NULL DEFAULT '',
    status              TEXT NOT NULL DEFAULT '',
    capabilities_json   TEXT NOT NULL DEFAULT '{}',
    registered_at       INTEGER NOT NULL DEFAULT 0,
    last_heartbeat_at   INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS project_runtime_placements (
    project_id           TEXT NOT NULL DEFAULT '',
    owner               TEXT NOT NULL,
    project             TEXT NOT NULL,
    mode                TEXT NOT NULL DEFAULT 'shared',
    target              TEXT NOT NULL DEFAULT 'local',
    target_office_id    TEXT,
    target_node_id      TEXT,
    worker_id           TEXT,
    resource_profile    TEXT NOT NULL DEFAULT '',
    desired_replicas    INTEGER NOT NULL DEFAULT 1,
    effective_state     TEXT NOT NULL DEFAULT '',
    created_at          INTEGER NOT NULL DEFAULT 0,
    updated_at          INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project),
    FOREIGN KEY (project_id) REFERENCES projects(project_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (target_office_id) REFERENCES offices(office_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (target_node_id) REFERENCES office_nodes(node_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS project_operations (
    project_id          TEXT NOT NULL DEFAULT '',
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
    PRIMARY KEY (owner, project, operation_id),
    FOREIGN KEY (project_id) REFERENCES projects(project_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (source_office_id) REFERENCES offices(office_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (target_office_id) REFERENCES offices(office_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
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

const SCHEMA_MIGRATIONS_SQL: &str = "
CREATE TABLE IF NOT EXISTS schema_migrations (
    version     INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    applied_at  INTEGER NOT NULL DEFAULT 0
);
";

#[derive(Clone, Copy)]
struct MigrationDef {
    version: i64,
    name: &'static str,
    apply: fn(&Transaction<'_>) -> Result<(), PlatformError>,
}

/// Platform catalog adapter backed by a single WAL-mode SQLite file.
pub struct SqliteDataAdapter {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteDataAdapter {
    /// Opens or creates `{data_root}/platform/catalog.db` with WAL mode and
    /// applies ordered platform schema migrations.
    pub fn new(data_root: &Path) -> Result<Self, PlatformError> {
        std::fs::create_dir_all(data_root.join("platform"))?;
        let path = data_root.join("platform").join("catalog.db");
        let mut conn = Connection::open(&path)
            .map_err(|e| PlatformError::new("PLATFORM_SQLITE_OPEN", e.to_string()))?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;",
        )
            .map_err(|e| PlatformError::new("PLATFORM_SQLITE_PRAGMA", e.to_string()))?;
        Self::run_migrations(&mut conn)?;
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

    fn sha256_hex(input: &str) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let digest = hasher.finalize();
        digest.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn marketplace_token_scopes(
        scope_read: bool,
        scope_publish: bool,
        scope_manage: bool,
    ) -> Vec<String> {
        let mut scopes = Vec::new();
        if scope_read {
            scopes.push("marketplace:read".to_string());
        }
        if scope_publish {
            scopes.push("marketplace:publish".to_string());
        }
        if scope_manage {
            scopes.push("marketplace:manage".to_string());
        }
        scopes
    }

    fn table_has_column<C>(conn: &C, table: &str, column: &str) -> Result<bool, PlatformError>
    where
        C: std::ops::Deref<Target = Connection>,
    {
        let pragma_sql = format!("PRAGMA table_info({table})");
        let mut stmt = conn.prepare(&pragma_sql).map_err(Self::qe)?;
        let exists = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(Self::qe)?
            .filter_map(|row| row.ok())
            .any(|name| name == column);
        Ok(exists)
    }

    fn migrations() -> [MigrationDef; 11] {
        [
            MigrationDef {
                version: 1,
                name: "initial_catalog",
                apply: Self::apply_migration_0001_initial_catalog,
            },
            MigrationDef {
                version: 2,
                name: "pipeline_invocation_run_id",
                apply: Self::apply_migration_0002_pipeline_invocation_run_id,
            },
            MigrationDef {
                version: 3,
                name: "marketplace_columns_and_publishers",
                apply: Self::apply_migration_0003_marketplace_columns_and_publishers,
            },
            MigrationDef {
                version: 4,
                name: "stable_ids_and_user_local_auth",
                apply: Self::apply_migration_0004_stable_ids_and_user_local_auth,
            },
            MigrationDef {
                version: 5,
                name: "authorities_offices_and_runtime_normalization",
                apply: Self::apply_migration_0005_authorities_offices_and_runtime_normalization,
            },
            MigrationDef {
                version: 6,
                name: "marketplace_and_operations_internal_ids",
                apply: Self::apply_migration_0006_marketplace_and_operations_internal_ids,
            },
            MigrationDef {
                version: 7,
                name: "ownership_and_access_internal_ids",
                apply: Self::apply_migration_0007_ownership_and_access_internal_ids,
            },
            MigrationDef {
                version: 8,
                name: "constraint_and_uniqueness_hardening",
                apply: Self::apply_migration_0008_constraint_and_uniqueness_hardening,
            },
            MigrationDef {
                version: 9,
                name: "core_foreign_key_enforcement",
                apply: Self::apply_migration_0009_core_foreign_key_enforcement,
            },
            MigrationDef {
                version: 10,
                name: "project_access_normalization",
                apply: Self::apply_migration_0010_project_access_normalization,
            },
            MigrationDef {
                version: 11,
                name: "marketplace_token_scope_flags",
                apply: Self::apply_migration_0011_marketplace_token_scope_flags,
            },
        ]
    }

    fn run_migrations(conn: &mut Connection) -> Result<(), PlatformError> {
        conn.execute_batch(SCHEMA_MIGRATIONS_SQL)
            .map_err(|e| PlatformError::new("PLATFORM_SQLITE_SCHEMA", e.to_string()))?;

        let applied_versions = {
            let mut stmt = conn
                .prepare("SELECT version FROM schema_migrations ORDER BY version ASC")
                .map_err(Self::qe)?;
            stmt.query_map([], |row| row.get::<_, i64>(0))
                .map_err(Self::qe)?
                .collect::<Result<BTreeSet<_>, _>>()
                .map_err(Self::qe)?
        };

        for migration in Self::migrations() {
            if applied_versions.contains(&migration.version) {
                continue;
            }
            let tx = conn.transaction().map_err(Self::qe)?;
            (migration.apply)(&tx)?;
            tx.execute(
                "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, strftime('%s','now'))",
                params![migration.version, migration.name],
            )
            .map_err(Self::qe)?;
            tx.commit().map_err(Self::qe)?;
        }
        Ok(())
    }

    fn apply_migration_0001_initial_catalog(tx: &Transaction<'_>) -> Result<(), PlatformError> {
        tx.execute_batch(SCHEMA_SQL)
            .map_err(|e| PlatformError::new("PLATFORM_SQLITE_SCHEMA", e.to_string()))
    }

    fn apply_migration_0002_pipeline_invocation_run_id(
        tx: &Transaction<'_>,
    ) -> Result<(), PlatformError> {
        Self::ensure_table_column(
            tx,
            "pipeline_invocations",
            "run_id",
            "TEXT NOT NULL DEFAULT ''",
        )
    }

    fn apply_migration_0003_marketplace_columns_and_publishers(
        tx: &Transaction<'_>,
    ) -> Result<(), PlatformError> {
        Self::ensure_marketplace_schema(tx)
    }

    fn apply_migration_0004_stable_ids_and_user_local_auth(
        tx: &Transaction<'_>,
    ) -> Result<(), PlatformError> {
        tx.execute_batch(
            "
CREATE TABLE IF NOT EXISTS user_local_auth (
    user_id              TEXT PRIMARY KEY,
    password_hash        TEXT NOT NULL DEFAULT '',
    password_alg         TEXT NOT NULL DEFAULT 'sha256',
    password_updated_at  INTEGER NOT NULL DEFAULT 0
);
",
        )
        .map_err(|e| PlatformError::new("PLATFORM_SQLITE_SCHEMA", e.to_string()))?;

        Self::ensure_table_column(tx, "users", "user_id", "TEXT NOT NULL DEFAULT ''")?;
        Self::ensure_table_column(tx, "projects", "project_id", "TEXT NOT NULL DEFAULT ''")?;

        if Self::table_has_column(tx, "users", "password")? {
            let mut stmt = tx
                .prepare(
                    "SELECT owner, created_at FROM users WHERE COALESCE(user_id, '') = '' ORDER BY owner ASC",
                )
                .map_err(Self::qe)?;
            let items = stmt
                .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
                .map_err(Self::qe)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(Self::qe)?;
            drop(stmt);
            for (owner, created_at) in items {
                let user_id = format!("usr_{}", uuid::Uuid::new_v4().simple());
                tx.execute(
                    "UPDATE users SET user_id = ?1 WHERE owner = ?2",
                    params![user_id, owner],
                )
                .map_err(Self::qe)?;
                let password = tx
                    .query_row(
                        "SELECT password FROM users WHERE owner = ?1",
                        params![owner],
                        |row| row.get::<_, String>(0),
                    )
                    .map_err(Self::qe)?;
                tx.execute(
                    "INSERT OR IGNORE INTO user_local_auth (user_id, password_hash, password_alg, password_updated_at)
                     VALUES (?1, ?2, 'sha256', ?3)",
                    params![user_id, Self::sha256_hex(&password), created_at],
                )
                .map_err(Self::qe)?;
            }
            let mut stmt = tx
                .prepare(
                    "SELECT user_id, owner, password, created_at FROM users
                     WHERE COALESCE(user_id, '') <> '' ORDER BY owner ASC",
                )
                .map_err(Self::qe)?;
            let items = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                })
                .map_err(Self::qe)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(Self::qe)?;
            drop(stmt);
            for (user_id, _owner, password, created_at) in items {
                tx.execute(
                    "INSERT OR IGNORE INTO user_local_auth (user_id, password_hash, password_alg, password_updated_at)
                     VALUES (?1, ?2, 'sha256', ?3)",
                    params![user_id, Self::sha256_hex(&password), created_at],
                )
                .map_err(Self::qe)?;
            }
        }

        {
            let mut stmt = tx
                .prepare(
                    "SELECT owner, project FROM projects WHERE COALESCE(project_id, '') = '' ORDER BY owner ASC, project ASC",
                )
                .map_err(Self::qe)?;
            let items = stmt
                .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
                .map_err(Self::qe)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(Self::qe)?;
            drop(stmt);
            for (owner, project) in items {
                let project_id = format!("prj_{}", uuid::Uuid::new_v4().simple());
                tx.execute(
                    "UPDATE projects SET project_id = ?1 WHERE owner = ?2 AND project = ?3",
                    params![project_id, owner, project],
                )
                .map_err(Self::qe)?;
            }
        }

        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_users_user_id ON users(user_id)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_projects_project_id ON projects(project_id)",
            [],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn apply_migration_0005_authorities_offices_and_runtime_normalization(
        tx: &Transaction<'_>,
    ) -> Result<(), PlatformError> {
        tx.execute_batch(
            "
CREATE TABLE IF NOT EXISTS marketplace_authorities (
    authority_id     TEXT PRIMARY KEY,
    host_project_id  TEXT NOT NULL UNIQUE DEFAULT '',
    owner            TEXT NOT NULL DEFAULT '',
    project          TEXT NOT NULL DEFAULT '',
    enabled          INTEGER NOT NULL DEFAULT 0,
    public_base_url  TEXT NOT NULL DEFAULT '',
    created_at       INTEGER NOT NULL DEFAULT 0,
    updated_at       INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS offices (
    office_id     TEXT PRIMARY KEY,
    office_slug   TEXT NOT NULL UNIQUE DEFAULT '',
    label         TEXT NOT NULL DEFAULT '',
    office_kind   TEXT NOT NULL DEFAULT 'office',
    base_url      TEXT NOT NULL DEFAULT '',
    status        TEXT NOT NULL DEFAULT '',
    created_at    INTEGER NOT NULL DEFAULT 0,
    updated_at    INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS office_nodes (
    node_id             TEXT PRIMARY KEY,
    office_id           TEXT NOT NULL DEFAULT '',
    label               TEXT NOT NULL DEFAULT '',
    base_url            TEXT NOT NULL DEFAULT '',
    status              TEXT NOT NULL DEFAULT '',
    capabilities_json   TEXT NOT NULL DEFAULT '{}',
    registered_at       INTEGER NOT NULL DEFAULT 0,
    last_heartbeat_at   INTEGER NOT NULL DEFAULT 0
);
",
        )
        .map_err(|e| PlatformError::new("PLATFORM_SQLITE_SCHEMA", e.to_string()))?;

        Self::ensure_table_column(tx, "worker_registry", "office_id", "TEXT NOT NULL DEFAULT ''")?;
        Self::ensure_table_column(tx, "worker_registry", "office_slug", "TEXT NOT NULL DEFAULT ''")?;
        Self::ensure_table_column(tx, "project_runtime_placements", "project_id", "TEXT NOT NULL DEFAULT ''")?;
        Self::ensure_table_column(tx, "project_runtime_placements", "target_office_id", "TEXT")?;
        Self::ensure_table_column(tx, "project_runtime_placements", "target_node_id", "TEXT")?;
        Self::ensure_table_column(tx, "project_runtime_placements", "resource_profile", "TEXT NOT NULL DEFAULT ''")?;
        Self::ensure_table_column(tx, "project_runtime_placements", "desired_replicas", "INTEGER NOT NULL DEFAULT 1")?;
        Self::ensure_table_column(tx, "project_runtime_placements", "effective_state", "TEXT NOT NULL DEFAULT ''")?;

        tx.execute(
            "UPDATE worker_registry
             SET office_id = node_id
             WHERE COALESCE(office_id, '') = ''",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "UPDATE worker_registry
             SET office_slug = office_id
             WHERE COALESCE(office_slug, '') = ''",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "INSERT OR REPLACE INTO offices
             (office_id, office_slug, label, office_kind, base_url, status, created_at, updated_at)
             SELECT office_id,
                    CASE WHEN COALESCE(office_slug, '') = '' THEN office_id ELSE office_slug END,
                    label,
                    'office',
                    base_url,
                    status,
                    registered_at,
                    last_heartbeat_at
             FROM worker_registry",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "INSERT OR REPLACE INTO office_nodes
             (node_id, office_id, label, base_url, status, capabilities_json, registered_at, last_heartbeat_at)
             SELECT node_id, office_id, label, base_url, status, capabilities_json, registered_at, last_heartbeat_at
             FROM worker_registry",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "UPDATE project_runtime_placements
             SET project_id = (
                 SELECT projects.project_id
                 FROM projects
                 WHERE projects.owner = project_runtime_placements.owner
                   AND projects.project = project_runtime_placements.project
             )
             WHERE COALESCE(project_id, '') = ''",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "UPDATE project_runtime_placements
             SET target_node_id = worker_id
             WHERE COALESCE(target_node_id, '') = '' AND COALESCE(worker_id, '') <> ''",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "UPDATE project_runtime_placements
             SET target_office_id = target_node_id
             WHERE COALESCE(target_office_id, '') = '' AND COALESCE(target_node_id, '') <> ''",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "UPDATE project_runtime_placements
             SET effective_state = CASE
                 WHEN target = 'worker' AND COALESCE(target_node_id, '') <> '' THEN 'assigned'
                 ELSE 'local'
             END
             WHERE COALESCE(effective_state, '') = ''",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "INSERT OR IGNORE INTO marketplace_authorities
             (authority_id, host_project_id, owner, project, enabled, public_base_url, created_at, updated_at)
             SELECT 'mka_' || project_id,
                    project_id,
                    owner,
                    project,
                    0,
                    '',
                    created_at,
                    updated_at
             FROM projects
             WHERE COALESCE(project_id, '') <> ''",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_marketplace_authorities_host_project_id
             ON marketplace_authorities(host_project_id)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_office_nodes_office_id
             ON office_nodes(office_id)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_project_runtime_placements_project_id
             ON project_runtime_placements(project_id)",
            [],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn apply_migration_0006_marketplace_and_operations_internal_ids(
        tx: &Transaction<'_>,
    ) -> Result<(), PlatformError> {
        Self::ensure_table_column(tx, "marketplace_publishers", "authority_id", "TEXT NOT NULL DEFAULT ''")?;
        Self::ensure_table_column(tx, "marketplace_publishers", "publisher_pk", "TEXT NOT NULL DEFAULT ''")?;
        Self::ensure_table_column(tx, "marketplace_tokens", "authority_id", "TEXT NOT NULL DEFAULT ''")?;
        Self::ensure_table_column(tx, "marketplace_tokens", "publisher_pk", "TEXT NOT NULL DEFAULT ''")?;
        Self::ensure_table_column(tx, "marketplace_asset_packages", "package_pk", "TEXT NOT NULL DEFAULT ''")?;
        Self::ensure_table_column(tx, "marketplace_asset_packages", "authority_id", "TEXT NOT NULL DEFAULT ''")?;
        Self::ensure_table_column(tx, "marketplace_asset_packages", "publisher_pk", "TEXT NOT NULL DEFAULT ''")?;
        Self::ensure_table_column(tx, "marketplace_asset_versions", "package_pk", "TEXT NOT NULL DEFAULT ''")?;
        Self::ensure_table_column(tx, "project_operations", "project_id", "TEXT NOT NULL DEFAULT ''")?;

        tx.execute(
            "UPDATE marketplace_publishers
             SET authority_id = (
                 SELECT authority_id
                 FROM marketplace_authorities
                 WHERE marketplace_authorities.owner = marketplace_publishers.owner
                   AND marketplace_authorities.project = marketplace_publishers.project
             )
             WHERE COALESCE(authority_id, '') = ''",
            [],
        )
        .map_err(Self::qe)?;

        {
            let mut stmt = tx
                .prepare(
                    "SELECT owner, project, publisher_id
                     FROM marketplace_publishers
                     WHERE COALESCE(publisher_pk, '') = ''
                     ORDER BY owner ASC, project ASC, publisher_id ASC",
                )
                .map_err(Self::qe)?;
            let items = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(Self::qe)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(Self::qe)?;
            drop(stmt);
            for (owner, project, publisher_id) in items {
                let publisher_pk = format!("mpub_{}", uuid::Uuid::new_v4().simple());
                tx.execute(
                    "UPDATE marketplace_publishers
                     SET publisher_pk = ?1
                     WHERE owner = ?2 AND project = ?3 AND publisher_id = ?4",
                    params![publisher_pk, owner, project, publisher_id],
                )
                .map_err(Self::qe)?;
            }
        }

        tx.execute(
            "UPDATE marketplace_tokens
             SET authority_id = (
                 SELECT authority_id
                 FROM marketplace_authorities
                 WHERE marketplace_authorities.owner = marketplace_tokens.owner
                   AND marketplace_authorities.project = marketplace_tokens.project
             )
             WHERE COALESCE(authority_id, '') = ''",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "UPDATE marketplace_tokens
             SET publisher_pk = (
                 SELECT publisher_pk
                 FROM marketplace_publishers
                 WHERE marketplace_publishers.owner = marketplace_tokens.owner
                   AND marketplace_publishers.project = marketplace_tokens.project
                   AND marketplace_publishers.publisher_id = marketplace_tokens.publisher_id
             )
             WHERE COALESCE(publisher_pk, '') = ''",
            [],
        )
        .map_err(Self::qe)?;

        {
            let mut stmt = tx
                .prepare(
                    "SELECT package_id
                     FROM marketplace_asset_packages
                     WHERE COALESCE(package_pk, '') = ''
                     ORDER BY package_id ASC",
                )
                .map_err(Self::qe)?;
            let items = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(Self::qe)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(Self::qe)?;
            drop(stmt);
            for package_id in items {
                let package_pk = format!("mpkg_{}", uuid::Uuid::new_v4().simple());
                tx.execute(
                    "UPDATE marketplace_asset_packages SET package_pk = ?1 WHERE package_id = ?2",
                    params![package_pk, package_id],
                )
                .map_err(Self::qe)?;
            }
        }

        tx.execute(
            "UPDATE marketplace_asset_packages
             SET authority_id = (
                 SELECT authority_id
                 FROM marketplace_authorities
                 WHERE marketplace_authorities.owner = marketplace_asset_packages.authority_owner
                   AND marketplace_authorities.project = marketplace_asset_packages.authority_project
             )
             WHERE COALESCE(authority_id, '') = ''",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "UPDATE marketplace_asset_packages
             SET publisher_pk = (
                 SELECT publisher_pk
                 FROM marketplace_publishers
                 WHERE marketplace_publishers.owner = marketplace_asset_packages.authority_owner
                   AND marketplace_publishers.project = marketplace_asset_packages.authority_project
                   AND marketplace_publishers.publisher_id = marketplace_asset_packages.publisher_id
             )
             WHERE COALESCE(publisher_pk, '') = ''",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "UPDATE marketplace_asset_versions
             SET package_pk = (
                 SELECT package_pk
                 FROM marketplace_asset_packages
                 WHERE marketplace_asset_packages.package_id = marketplace_asset_versions.package_id
             )
             WHERE COALESCE(package_pk, '') = ''",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "UPDATE project_operations
             SET project_id = (
                 SELECT projects.project_id
                 FROM projects
                 WHERE projects.owner = project_operations.owner
                   AND projects.project = project_operations.project
             )
             WHERE COALESCE(project_id, '') = ''",
            [],
        )
        .map_err(Self::qe)?;

        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_marketplace_publishers_publisher_pk
             ON marketplace_publishers(publisher_pk)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_marketplace_asset_packages_package_pk
             ON marketplace_asset_packages(package_pk)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_marketplace_tokens_publisher_pk
             ON marketplace_tokens(publisher_pk)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_marketplace_asset_versions_package_pk
             ON marketplace_asset_versions(package_pk)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_project_operations_project_id
             ON project_operations(project_id, updated_at DESC, operation_id ASC)",
            [],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn apply_migration_0007_ownership_and_access_internal_ids(
        tx: &Transaction<'_>,
    ) -> Result<(), PlatformError> {
        Self::ensure_table_column(tx, "projects", "owner_user_id", "TEXT NOT NULL DEFAULT ''")?;
        Self::ensure_table_column(tx, "platform_marketplace_repositories", "source_id", "TEXT NOT NULL DEFAULT ''")?;
        Self::ensure_table_column(tx, "platform_marketplace_repositories", "owner_user_id", "TEXT NOT NULL DEFAULT ''")?;
        Self::ensure_table_column(tx, "project_policies", "project_id", "TEXT NOT NULL DEFAULT ''")?;
        Self::ensure_table_column(tx, "project_policy_bindings", "project_id", "TEXT NOT NULL DEFAULT ''")?;
        Self::ensure_table_column(tx, "project_members", "project_id", "TEXT NOT NULL DEFAULT ''")?;
        Self::ensure_table_column(tx, "project_members", "created_by_user_id", "TEXT NOT NULL DEFAULT ''")?;

        tx.execute(
            "UPDATE projects
             SET owner_user_id = (
                 SELECT users.user_id FROM users WHERE users.owner = projects.owner
             )
             WHERE COALESCE(owner_user_id, '') = ''",
            [],
        )
        .map_err(Self::qe)?;

        {
            let mut stmt = tx
                .prepare(
                    "SELECT owner, repository_id
                     FROM platform_marketplace_repositories
                     WHERE COALESCE(source_id, '') = ''
                     ORDER BY owner ASC, repository_id ASC",
                )
                .map_err(Self::qe)?;
            let items = stmt
                .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
                .map_err(Self::qe)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(Self::qe)?;
            drop(stmt);
            for (owner, repository_id) in items {
                let source_id = format!("pmr_{}", uuid::Uuid::new_v4().simple());
                tx.execute(
                    "UPDATE platform_marketplace_repositories
                     SET source_id = ?1
                     WHERE owner = ?2 AND repository_id = ?3",
                    params![source_id, owner, repository_id],
                )
                .map_err(Self::qe)?;
            }
        }

        tx.execute(
            "UPDATE platform_marketplace_repositories
             SET owner_user_id = (
                 SELECT users.user_id FROM users WHERE users.owner = platform_marketplace_repositories.owner
             )
             WHERE COALESCE(owner_user_id, '') = ''",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "UPDATE project_policies
             SET project_id = (
                 SELECT projects.project_id
                 FROM projects
                 WHERE projects.owner = project_policies.owner
                   AND projects.project = project_policies.project
             )
             WHERE COALESCE(project_id, '') = ''",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "UPDATE project_policy_bindings
             SET project_id = (
                 SELECT projects.project_id
                 FROM projects
                 WHERE projects.owner = project_policy_bindings.owner
                   AND projects.project = project_policy_bindings.project
             )
             WHERE COALESCE(project_id, '') = ''",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "UPDATE project_members
             SET project_id = (
                 SELECT projects.project_id
                 FROM projects
                 WHERE projects.owner = project_members.owner
                   AND projects.project = project_members.project
             )
             WHERE COALESCE(project_id, '') = ''",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "UPDATE project_members
             SET created_by_user_id = (
                 SELECT users.user_id
                 FROM users
                 WHERE users.owner = project_members.created_by
             )
             WHERE COALESCE(created_by_user_id, '') = ''",
            [],
        )
        .map_err(Self::qe)?;

        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_platform_marketplace_repositories_source_id
             ON platform_marketplace_repositories(source_id)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_projects_owner_user_id
             ON projects(owner_user_id)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_project_members_project_id
             ON project_members(project_id, user_id)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_project_policies_project_id
             ON project_policies(project_id, policy_id)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_project_policy_bindings_project_id
             ON project_policy_bindings(project_id, subject_id, policy_id)",
            [],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn apply_migration_0008_constraint_and_uniqueness_hardening(
        tx: &Transaction<'_>,
    ) -> Result<(), PlatformError> {
        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_project_db_connections_slug
             ON project_db_connections(owner, project, connection_slug)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_project_members_project_user
             ON project_members(project_id, user_id)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_project_policies_project_policy
             ON project_policies(project_id, policy_id)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_project_policy_bindings_project_subject_policy
             ON project_policy_bindings(project_id, subject_kind, subject_id, policy_id)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_marketplace_publishers_authority_publisher
             ON marketplace_publishers(authority_id, publisher_id)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_marketplace_publishers_authority_url
             ON marketplace_publishers(authority_id, publisher_url)
             WHERE publisher_url <> ''",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_marketplace_asset_packages_authority_package
             ON marketplace_asset_packages(authority_id, package_id)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_marketplace_asset_versions_packagepk_version
             ON marketplace_asset_versions(package_pk, version)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_platform_marketplace_repositories_owner_user
             ON platform_marketplace_repositories(owner_user_id, enabled, title)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_pipeline_meta_project_file
             ON pipeline_meta(owner, project, file_rel_path)",
            [],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn apply_migration_0009_core_foreign_key_enforcement(
        tx: &Transaction<'_>,
    ) -> Result<(), PlatformError> {
        tx.execute_batch("PRAGMA defer_foreign_keys = ON;")
            .map_err(Self::qe)?;

        Self::assert_non_empty_column(tx, "users", "user_id")?;
        Self::assert_non_empty_column(tx, "projects", "project_id")?;
        Self::assert_non_empty_column(tx, "projects", "owner_user_id")?;
        Self::assert_non_empty_column(tx, "platform_marketplace_repositories", "source_id")?;
        Self::assert_non_empty_column(tx, "platform_marketplace_repositories", "owner_user_id")?;
        Self::assert_non_empty_column(tx, "marketplace_authorities", "authority_id")?;
        Self::assert_non_empty_column(tx, "marketplace_authorities", "host_project_id")?;
        Self::assert_non_empty_column(tx, "marketplace_publishers", "authority_id")?;
        Self::assert_non_empty_column(tx, "marketplace_publishers", "publisher_pk")?;
        Self::assert_non_empty_column(tx, "marketplace_tokens", "authority_id")?;
        Self::assert_non_empty_column(tx, "marketplace_tokens", "publisher_pk")?;
        Self::assert_non_empty_column(tx, "marketplace_asset_packages", "package_pk")?;
        Self::assert_non_empty_column(tx, "marketplace_asset_packages", "authority_id")?;
        Self::assert_non_empty_column(tx, "marketplace_asset_packages", "publisher_pk")?;
        Self::assert_non_empty_column(tx, "marketplace_asset_versions", "package_pk")?;
        Self::assert_non_empty_column(tx, "office_nodes", "office_id")?;
        Self::assert_non_empty_column(tx, "project_runtime_placements", "project_id")?;
        Self::assert_non_empty_column(tx, "project_operations", "project_id")?;

        Self::rebuild_table(
            tx,
            "user_local_auth",
            "
CREATE TABLE user_local_auth (
    user_id              TEXT PRIMARY KEY,
    password_hash        TEXT NOT NULL DEFAULT '',
    password_alg         TEXT NOT NULL DEFAULT 'sha256',
    password_updated_at  INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (user_id) REFERENCES users(user_id)
        ON UPDATE CASCADE
        ON DELETE CASCADE
)",
            &["user_id", "password_hash", "password_alg", "password_updated_at"],
        )?;
        Self::rebuild_table(
            tx,
            "projects",
            "
CREATE TABLE projects (
    owner         TEXT NOT NULL,
    project       TEXT NOT NULL,
    project_id    TEXT NOT NULL UNIQUE DEFAULT '',
    owner_user_id TEXT NOT NULL DEFAULT '',
    created_at    INTEGER NOT NULL DEFAULT 0,
    updated_at    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project),
    FOREIGN KEY (owner_user_id) REFERENCES users(user_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
)",
            &["owner", "project", "project_id", "owner_user_id", "created_at", "updated_at"],
        )?;
        Self::rebuild_table(
            tx,
            "platform_marketplace_repositories",
            "
CREATE TABLE platform_marketplace_repositories (
    source_id      TEXT NOT NULL UNIQUE DEFAULT '',
    owner_user_id  TEXT NOT NULL DEFAULT '',
    owner          TEXT NOT NULL,
    repository_id  TEXT NOT NULL,
    title          TEXT NOT NULL DEFAULT '',
    base_url       TEXT NOT NULL DEFAULT '',
    remote_owner   TEXT NOT NULL DEFAULT '',
    remote_project TEXT NOT NULL DEFAULT '',
    read_token     TEXT NOT NULL DEFAULT '',
    enabled        INTEGER NOT NULL DEFAULT 1,
    created_at     INTEGER NOT NULL DEFAULT 0,
    updated_at     INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, repository_id),
    FOREIGN KEY (owner_user_id) REFERENCES users(user_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
)",
            &[
                "source_id",
                "owner_user_id",
                "owner",
                "repository_id",
                "title",
                "base_url",
                "remote_owner",
                "remote_project",
                "read_token",
                "enabled",
                "created_at",
                "updated_at",
            ],
        )?;
        Self::rebuild_table(
            tx,
            "marketplace_authorities",
            "
CREATE TABLE marketplace_authorities (
    authority_id     TEXT PRIMARY KEY,
    host_project_id  TEXT NOT NULL UNIQUE DEFAULT '',
    owner            TEXT NOT NULL DEFAULT '',
    project          TEXT NOT NULL DEFAULT '',
    enabled          INTEGER NOT NULL DEFAULT 0,
    public_base_url  TEXT NOT NULL DEFAULT '',
    created_at       INTEGER NOT NULL DEFAULT 0,
    updated_at       INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (host_project_id) REFERENCES projects(project_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
)",
            &[
                "authority_id",
                "host_project_id",
                "owner",
                "project",
                "enabled",
                "public_base_url",
                "created_at",
                "updated_at",
            ],
        )?;
        Self::rebuild_table(
            tx,
            "marketplace_publishers",
            "
CREATE TABLE marketplace_publishers (
    authority_id    TEXT NOT NULL DEFAULT '',
    publisher_pk    TEXT NOT NULL UNIQUE DEFAULT '',
    owner           TEXT NOT NULL,
    project         TEXT NOT NULL,
    publisher_id    TEXT NOT NULL,
    display_name    TEXT NOT NULL DEFAULT '',
    publisher_url   TEXT NOT NULL DEFAULT '',
    email           TEXT NOT NULL DEFAULT '',
    description     TEXT NOT NULL DEFAULT '',
    icon_url        TEXT NOT NULL DEFAULT '',
    website_url     TEXT NOT NULL DEFAULT '',
    enabled         INTEGER NOT NULL DEFAULT 1,
    created_at      INTEGER NOT NULL DEFAULT 0,
    updated_at      INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, publisher_id),
    FOREIGN KEY (authority_id) REFERENCES marketplace_authorities(authority_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
)",
            &[
                "authority_id",
                "publisher_pk",
                "owner",
                "project",
                "publisher_id",
                "display_name",
                "publisher_url",
                "email",
                "description",
                "icon_url",
                "website_url",
                "enabled",
                "created_at",
                "updated_at",
            ],
        )?;
        Self::rebuild_table(
            tx,
            "marketplace_tokens",
            "
CREATE TABLE marketplace_tokens (
    token_id                TEXT PRIMARY KEY,
    authority_id            TEXT NOT NULL DEFAULT '',
    publisher_pk            TEXT NOT NULL DEFAULT '',
    owner                   TEXT NOT NULL DEFAULT '',
    project                 TEXT NOT NULL DEFAULT '',
    publisher_id            TEXT NOT NULL DEFAULT '',
    publisher_display_name  TEXT NOT NULL DEFAULT '',
    publisher_url           TEXT NOT NULL DEFAULT '',
    publisher_email         TEXT NOT NULL DEFAULT '',
    title                   TEXT NOT NULL DEFAULT '',
    secret_hash             TEXT NOT NULL DEFAULT '',
    scopes_json             TEXT NOT NULL DEFAULT '[]',
    expires_at              INTEGER,
    last_used_at            INTEGER,
    revoked_at              INTEGER,
    created_at              INTEGER NOT NULL DEFAULT 0,
    updated_at              INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (authority_id) REFERENCES marketplace_authorities(authority_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (publisher_pk) REFERENCES marketplace_publishers(publisher_pk)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
)",
            &[
                "token_id",
                "authority_id",
                "publisher_pk",
                "owner",
                "project",
                "publisher_id",
                "publisher_display_name",
                "publisher_url",
                "publisher_email",
                "title",
                "secret_hash",
                "scopes_json",
                "expires_at",
                "last_used_at",
                "revoked_at",
                "created_at",
                "updated_at",
            ],
        )?;
        Self::rebuild_table(
            tx,
            "marketplace_asset_packages",
            "
CREATE TABLE marketplace_asset_packages (
    package_pk              TEXT NOT NULL UNIQUE DEFAULT '',
    authority_id            TEXT NOT NULL DEFAULT '',
    publisher_pk            TEXT NOT NULL DEFAULT '',
    package_id              TEXT PRIMARY KEY,
    authority_owner         TEXT NOT NULL DEFAULT '',
    authority_project       TEXT NOT NULL DEFAULT '',
    publisher_owner         TEXT NOT NULL DEFAULT '',
    publisher_id            TEXT NOT NULL DEFAULT '',
    publisher_display_name  TEXT NOT NULL DEFAULT '',
    publisher_url           TEXT NOT NULL DEFAULT '',
    publisher_email         TEXT NOT NULL DEFAULT '',
    asset_kind              TEXT NOT NULL DEFAULT '',
    title                   TEXT NOT NULL DEFAULT '',
    description             TEXT NOT NULL DEFAULT '',
    visibility              TEXT NOT NULL DEFAULT 'private',
    tags_json               TEXT NOT NULL DEFAULT '[]',
    created_at              INTEGER NOT NULL DEFAULT 0,
    updated_at              INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (authority_id) REFERENCES marketplace_authorities(authority_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (publisher_pk) REFERENCES marketplace_publishers(publisher_pk)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
)",
            &[
                "package_pk",
                "authority_id",
                "publisher_pk",
                "package_id",
                "authority_owner",
                "authority_project",
                "publisher_owner",
                "publisher_id",
                "publisher_display_name",
                "publisher_url",
                "publisher_email",
                "asset_kind",
                "title",
                "description",
                "visibility",
                "tags_json",
                "created_at",
                "updated_at",
            ],
        )?;
        Self::rebuild_table(
            tx,
            "marketplace_asset_versions",
            "
CREATE TABLE marketplace_asset_versions (
    package_pk         TEXT NOT NULL DEFAULT '',
    package_id         TEXT NOT NULL,
    version            TEXT NOT NULL,
    authority_owner    TEXT NOT NULL DEFAULT '',
    authority_project  TEXT NOT NULL DEFAULT '',
    publisher_owner    TEXT NOT NULL DEFAULT '',
    publisher_id       TEXT NOT NULL DEFAULT '',
    source_owner       TEXT NOT NULL DEFAULT '',
    source_project     TEXT NOT NULL DEFAULT '',
    source_kind        TEXT NOT NULL DEFAULT '',
    source_ref         TEXT NOT NULL DEFAULT '',
    artifact_rel_path  TEXT NOT NULL DEFAULT '',
    artifact_sha256    TEXT NOT NULL DEFAULT '',
    manifest_json      TEXT NOT NULL DEFAULT 'null',
    created_at         INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (package_id, version),
    FOREIGN KEY (package_pk) REFERENCES marketplace_asset_packages(package_pk)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
)",
            &[
                "package_pk",
                "package_id",
                "version",
                "authority_owner",
                "authority_project",
                "publisher_owner",
                "publisher_id",
                "source_owner",
                "source_project",
                "source_kind",
                "source_ref",
                "artifact_rel_path",
                "artifact_sha256",
                "manifest_json",
                "created_at",
            ],
        )?;
        Self::rebuild_table(
            tx,
            "office_nodes",
            "
CREATE TABLE office_nodes (
    node_id             TEXT PRIMARY KEY,
    office_id           TEXT NOT NULL DEFAULT '',
    label               TEXT NOT NULL DEFAULT '',
    base_url            TEXT NOT NULL DEFAULT '',
    status              TEXT NOT NULL DEFAULT '',
    capabilities_json   TEXT NOT NULL DEFAULT '{}',
    registered_at       INTEGER NOT NULL DEFAULT 0,
    last_heartbeat_at   INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (office_id) REFERENCES offices(office_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
)",
            &[
                "node_id",
                "office_id",
                "label",
                "base_url",
                "status",
                "capabilities_json",
                "registered_at",
                "last_heartbeat_at",
            ],
        )?;
        Self::rebuild_table(
            tx,
            "project_runtime_placements",
            "
CREATE TABLE project_runtime_placements (
    project_id         TEXT NOT NULL DEFAULT '',
    owner              TEXT NOT NULL,
    project            TEXT NOT NULL,
    mode               TEXT NOT NULL DEFAULT 'shared',
    target             TEXT NOT NULL DEFAULT 'local',
    target_office_id   TEXT,
    target_node_id     TEXT,
    worker_id          TEXT,
    resource_profile   TEXT NOT NULL DEFAULT '',
    desired_replicas   INTEGER NOT NULL DEFAULT 1,
    effective_state    TEXT NOT NULL DEFAULT '',
    created_at         INTEGER NOT NULL DEFAULT 0,
    updated_at         INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project),
    FOREIGN KEY (project_id) REFERENCES projects(project_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (target_office_id) REFERENCES offices(office_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (target_node_id) REFERENCES office_nodes(node_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
)",
            &[
                "project_id",
                "owner",
                "project",
                "mode",
                "target",
                "target_office_id",
                "target_node_id",
                "worker_id",
                "resource_profile",
                "desired_replicas",
                "effective_state",
                "created_at",
                "updated_at",
            ],
        )?;
        Self::rebuild_table(
            tx,
            "project_operations",
            "
CREATE TABLE project_operations (
    project_id          TEXT NOT NULL DEFAULT '',
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
    PRIMARY KEY (owner, project, operation_id),
    FOREIGN KEY (project_id) REFERENCES projects(project_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (source_office_id) REFERENCES offices(office_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (target_office_id) REFERENCES offices(office_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
)",
            &[
                "project_id",
                "owner",
                "project",
                "operation_id",
                "kind",
                "status",
                "current_step",
                "source_office_id",
                "target_office_id",
                "artifact_rel_path",
                "artifact_sha256",
                "artifact_bytes",
                "error_message",
                "retry_count",
                "created_at",
                "updated_at",
                "completed_at",
            ],
        )?;

        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_office_nodes_office_id
             ON office_nodes(office_id)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_project_runtime_placements_project_id
             ON project_runtime_placements(project_id)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_project_operations_project
             ON project_operations(owner, project, updated_at DESC, operation_id ASC)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_project_operations_project_id
             ON project_operations(project_id, updated_at DESC, operation_id ASC)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_marketplace_publishers_publisher_pk
             ON marketplace_publishers(publisher_pk)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_marketplace_asset_packages_package_pk
             ON marketplace_asset_packages(package_pk)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_marketplace_tokens_publisher_pk
             ON marketplace_tokens(publisher_pk)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_marketplace_asset_versions_package_pk
             ON marketplace_asset_versions(package_pk)",
            [],
        )
        .map_err(Self::qe)?;
        Self::apply_migration_0008_constraint_and_uniqueness_hardening(tx)?;
        let violations = {
            let mut stmt = tx
                .prepare("PRAGMA foreign_key_check")
                .map_err(Self::qe)?;
            stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(Self::qe)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Self::qe)?
        };
        if let Some((table, rowid, parent, fk)) = violations.into_iter().next() {
            return Err(PlatformError::new(
                "PLATFORM_SQLITE_FK_CHECK_FAILED",
                format!(
                    "foreign key violation after rebuild: table={table} rowid={rowid} parent={parent} fk={fk}"
                ),
            ));
        }
        Ok(())
    }

    fn apply_migration_0010_project_access_normalization(
        tx: &Transaction<'_>,
    ) -> Result<(), PlatformError> {
        tx.execute_batch("PRAGMA defer_foreign_keys = ON;")
            .map_err(Self::qe)?;
        Self::ensure_table_column(tx, "project_members", "member_user_id", "TEXT NOT NULL DEFAULT ''")?;

        tx.execute(
            "UPDATE project_members
             SET member_user_id = (
                 SELECT users.user_id
                 FROM users
                 WHERE users.owner = project_members.user_id
             )
             WHERE COALESCE(member_user_id, '') = ''",
            [],
        )
        .map_err(Self::qe)?;

        Self::assert_non_empty_column(tx, "project_policies", "project_id")?;
        Self::assert_non_empty_column(tx, "project_policy_bindings", "project_id")?;
        Self::assert_non_empty_column(tx, "project_members", "project_id")?;
        Self::assert_non_empty_column(tx, "project_members", "member_user_id")?;
        Self::assert_non_empty_column(tx, "project_members", "created_by_user_id")?;

        Self::rebuild_table(
            tx,
            "project_policies",
            "
CREATE TABLE project_policies (
    project_id        TEXT NOT NULL DEFAULT '',
    owner             TEXT NOT NULL,
    project           TEXT NOT NULL,
    policy_id         TEXT NOT NULL,
    title             TEXT NOT NULL DEFAULT '',
    capabilities_json TEXT NOT NULL DEFAULT '[]',
    managed           INTEGER NOT NULL DEFAULT 0,
    created_at        INTEGER NOT NULL DEFAULT 0,
    updated_at        INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, policy_id),
    UNIQUE (project_id, policy_id),
    FOREIGN KEY (project_id) REFERENCES projects(project_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
)",
            &[
                "project_id",
                "owner",
                "project",
                "policy_id",
                "title",
                "capabilities_json",
                "managed",
                "created_at",
                "updated_at",
            ],
        )?;
        Self::rebuild_table(
            tx,
            "project_policy_bindings",
            "
CREATE TABLE project_policy_bindings (
    project_id    TEXT NOT NULL DEFAULT '',
    owner         TEXT NOT NULL,
    project       TEXT NOT NULL,
    subject_kind  TEXT NOT NULL,
    subject_id    TEXT NOT NULL,
    policy_id     TEXT NOT NULL,
    created_at    INTEGER NOT NULL DEFAULT 0,
    updated_at    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, subject_id, policy_id),
    FOREIGN KEY (project_id) REFERENCES projects(project_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (project_id, policy_id) REFERENCES project_policies(project_id, policy_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
)",
            &[
                "project_id",
                "owner",
                "project",
                "subject_kind",
                "subject_id",
                "policy_id",
                "created_at",
                "updated_at",
            ],
        )?;
        Self::rebuild_table(
            tx,
            "project_members",
            "
CREATE TABLE project_members (
    project_id              TEXT NOT NULL DEFAULT '',
    owner                   TEXT NOT NULL,
    project                 TEXT NOT NULL,
    user_id                 TEXT NOT NULL,
    member_user_id          TEXT NOT NULL DEFAULT '',
    role_preset             TEXT NOT NULL DEFAULT 'reporter',
    custom_policy_ids_json  TEXT NOT NULL DEFAULT '[]',
    mcp_capabilities_json   TEXT NOT NULL DEFAULT '[]',
    created_by              TEXT NOT NULL DEFAULT '',
    created_by_user_id      TEXT NOT NULL DEFAULT '',
    created_at              INTEGER NOT NULL DEFAULT 0,
    updated_at              INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner, project, user_id),
    FOREIGN KEY (project_id) REFERENCES projects(project_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (member_user_id) REFERENCES users(user_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (created_by_user_id) REFERENCES users(user_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
)",
            &[
                "project_id",
                "owner",
                "project",
                "user_id",
                "member_user_id",
                "role_preset",
                "custom_policy_ids_json",
                "mcp_capabilities_json",
                "created_by",
                "created_by_user_id",
                "created_at",
                "updated_at",
            ],
        )?;

        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_project_policies_project_policy
             ON project_policies(project_id, policy_id)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_project_policy_bindings_project_subject_policy
             ON project_policy_bindings(project_id, subject_kind, subject_id, policy_id)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_project_members_project_user
             ON project_members(project_id, user_id)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_project_members_project_member_user_id
             ON project_members(project_id, member_user_id)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_project_members_member_user_id
             ON project_members(member_user_id)",
            [],
        )
        .map_err(Self::qe)?;

        let violations = {
            let mut stmt = tx
                .prepare("PRAGMA foreign_key_check")
                .map_err(Self::qe)?;
            stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(Self::qe)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Self::qe)?
        };
        if let Some((table, rowid, parent, fk)) = violations.into_iter().next() {
            return Err(PlatformError::new(
                "PLATFORM_SQLITE_FK_CHECK_FAILED",
                format!(
                    "foreign key violation after access rebuild: table={table} rowid={rowid} parent={parent} fk={fk}"
                ),
            ));
        }
        Ok(())
    }

    fn apply_migration_0011_marketplace_token_scope_flags(
        tx: &Transaction<'_>,
    ) -> Result<(), PlatformError> {
        tx.execute_batch("PRAGMA defer_foreign_keys = ON;")
            .map_err(Self::qe)?;
        Self::ensure_table_column(tx, "marketplace_tokens", "scope_read", "INTEGER NOT NULL DEFAULT 0")?;
        Self::ensure_table_column(tx, "marketplace_tokens", "scope_publish", "INTEGER NOT NULL DEFAULT 0")?;
        Self::ensure_table_column(tx, "marketplace_tokens", "scope_manage", "INTEGER NOT NULL DEFAULT 0")?;

        if Self::table_has_column(tx, "marketplace_tokens", "scopes_json")? {
            tx.execute(
                "UPDATE marketplace_tokens
                 SET scope_read = CASE
                        WHEN COALESCE(scopes_json, '[]') LIKE '%\"marketplace:read\"%' THEN 1
                        ELSE 0
                     END,
                     scope_publish = CASE
                        WHEN COALESCE(scopes_json, '[]') LIKE '%\"marketplace:publish\"%' THEN 1
                        ELSE 0
                     END,
                     scope_manage = CASE
                        WHEN COALESCE(scopes_json, '[]') LIKE '%\"marketplace:manage\"%' THEN 1
                        ELSE 0
                     END",
                [],
            )
            .map_err(Self::qe)?;
        }

        let old_table = "marketplace_tokens__old";
        tx.execute_batch(&format!("ALTER TABLE marketplace_tokens RENAME TO {old_table};"))
            .map_err(Self::qe)?;
        tx.execute_batch(
            "
CREATE TABLE marketplace_tokens (
    token_id                TEXT PRIMARY KEY,
    authority_id            TEXT NOT NULL DEFAULT '',
    publisher_pk            TEXT NOT NULL DEFAULT '',
    owner                   TEXT NOT NULL DEFAULT '',
    project                 TEXT NOT NULL DEFAULT '',
    publisher_id            TEXT NOT NULL DEFAULT '',
    publisher_display_name  TEXT NOT NULL DEFAULT '',
    publisher_url           TEXT NOT NULL DEFAULT '',
    publisher_email         TEXT NOT NULL DEFAULT '',
    title                   TEXT NOT NULL DEFAULT '',
    secret_hash             TEXT NOT NULL DEFAULT '',
    scope_read              INTEGER NOT NULL DEFAULT 0,
    scope_publish           INTEGER NOT NULL DEFAULT 0,
    scope_manage            INTEGER NOT NULL DEFAULT 0,
    expires_at              INTEGER,
    last_used_at            INTEGER,
    revoked_at              INTEGER,
    created_at              INTEGER NOT NULL DEFAULT 0,
    updated_at              INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (authority_id) REFERENCES marketplace_authorities(authority_id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    FOREIGN KEY (publisher_pk) REFERENCES marketplace_publishers(publisher_pk)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
)",
        )
        .map_err(Self::qe)?;
        let old_has_scope_columns = Self::table_has_column(tx, old_table, "scope_read")?;
        let old_has_scopes_json = Self::table_has_column(tx, old_table, "scopes_json")?;
        let insert_sql = if old_has_scope_columns {
            format!(
                "INSERT INTO marketplace_tokens
                 (token_id, authority_id, publisher_pk, owner, project, publisher_id, publisher_display_name, publisher_url, publisher_email, title, secret_hash, scope_read, scope_publish, scope_manage, expires_at, last_used_at, revoked_at, created_at, updated_at)
                 SELECT token_id, authority_id, publisher_pk, owner, project, publisher_id, publisher_display_name, publisher_url, publisher_email, title, secret_hash, scope_read, scope_publish, scope_manage, expires_at, last_used_at, revoked_at, created_at, updated_at
                 FROM {old_table}"
            )
        } else if old_has_scopes_json {
            format!(
                "INSERT INTO marketplace_tokens
                 (token_id, authority_id, publisher_pk, owner, project, publisher_id, publisher_display_name, publisher_url, publisher_email, title, secret_hash, scope_read, scope_publish, scope_manage, expires_at, last_used_at, revoked_at, created_at, updated_at)
                 SELECT token_id, authority_id, publisher_pk, owner, project, publisher_id, publisher_display_name, publisher_url, publisher_email, title, secret_hash,
                        CASE WHEN COALESCE(scopes_json, '[]') LIKE '%\"marketplace:read\"%' THEN 1 ELSE 0 END,
                        CASE WHEN COALESCE(scopes_json, '[]') LIKE '%\"marketplace:publish\"%' THEN 1 ELSE 0 END,
                        CASE WHEN COALESCE(scopes_json, '[]') LIKE '%\"marketplace:manage\"%' THEN 1 ELSE 0 END,
                        expires_at, last_used_at, revoked_at, created_at, updated_at
                 FROM {old_table}"
            )
        } else {
            return Err(PlatformError::new(
                "PLATFORM_SQLITE_SCOPE_MIGRATION",
                "marketplace_tokens has neither scope flags nor scopes_json",
            ));
        };
        tx.execute(&insert_sql, []).map_err(Self::qe)?;
        tx.execute_batch(&format!("DROP TABLE {old_table};"))
            .map_err(Self::qe)?;

        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_marketplace_tokens_publisher_pk
             ON marketplace_tokens(publisher_pk)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_marketplace_tokens_scope_publish
             ON marketplace_tokens(scope_publish, revoked_at, expires_at)",
            [],
        )
        .map_err(Self::qe)?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_marketplace_tokens_scope_read
             ON marketplace_tokens(scope_read, revoked_at, expires_at)",
            [],
        )
        .map_err(Self::qe)?;

        let violations = {
            let mut stmt = tx
                .prepare("PRAGMA foreign_key_check")
                .map_err(Self::qe)?;
            stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(Self::qe)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Self::qe)?
        };
        if let Some((table, rowid, parent, fk)) = violations.into_iter().next() {
            return Err(PlatformError::new(
                "PLATFORM_SQLITE_FK_CHECK_FAILED",
                format!(
                    "foreign key violation after token scope rebuild: table={table} rowid={rowid} parent={parent} fk={fk}"
                ),
            ));
        }
        Ok(())
    }

    fn ensure_table_column<C>(
        conn: &C,
        table: &str,
        column: &str,
        definition: &str,
    ) -> Result<(), PlatformError>
    where
        C: std::ops::Deref<Target = Connection>,
    {
        let exists = Self::table_has_column(conn, table, column)?;
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

    fn assert_non_empty_column(
        tx: &Transaction<'_>,
        table: &str,
        column: &str,
    ) -> Result<(), PlatformError> {
        let sql = format!(
            "SELECT COUNT(1) FROM {table} WHERE COALESCE(TRIM({column}), '') = ''"
        );
        let count: i64 = tx
            .query_row(&sql, [], |row| row.get(0))
            .map_err(Self::qe)?;
        if count > 0 {
            return Err(PlatformError::new(
                "PLATFORM_SQLITE_FK_REBUILD_BLOCKED",
                format!("{table}.{column} has {count} blank rows"),
            ));
        }
        Ok(())
    }

    fn rebuild_table(
        tx: &Transaction<'_>,
        table: &str,
        create_sql: &str,
        columns: &[&str],
    ) -> Result<(), PlatformError> {
        let old_table = format!("{table}__old");
        tx.execute_batch(&format!("ALTER TABLE {table} RENAME TO {old_table};"))
            .map_err(Self::qe)?;
        tx.execute_batch(create_sql).map_err(Self::qe)?;
        let cols = columns.join(", ");
        tx.execute(
            &format!("INSERT INTO {table} ({cols}) SELECT {cols} FROM {old_table}"),
            [],
        )
        .map_err(Self::qe)?;
        tx.execute_batch(&format!("DROP TABLE {old_table};"))
            .map_err(Self::qe)?;
        Ok(())
    }

    fn ensure_marketplace_schema<C>(conn: &C) -> Result<(), PlatformError>
    where
        C: std::ops::Deref<Target = Connection>,
    {
        conn.execute_batch(
            "
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
",
        )
        .map_err(|e| PlatformError::new("PLATFORM_SQLITE_SCHEMA", e.to_string()))?;
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
            "SELECT u.user_id, u.owner, u.role, u.git_name, u.git_email, u.created_at, u.updated_at,
                    a.password_hash, a.password_alg, a.password_updated_at
             FROM users u
             LEFT JOIN user_local_auth a ON a.user_id = u.user_id
             WHERE u.owner = ?1",
            params![owner],
            |row| {
                Ok(StoredUser {
                    profile: PlatformUser {
                        user_id: row.get(0)?,
                        owner: row.get(1)?,
                        role: row.get(2)?,
                        git_name: row.get(3)?,
                        git_email: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    },
                    auth: PlatformUserLocalAuth {
                        user_id: row.get(0)?,
                        password_hash: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                        password_alg: row
                            .get::<_, Option<String>>(8)?
                            .unwrap_or_else(|| "sha256".to_string()),
                        password_updated_at: row.get::<_, Option<i64>>(9)?.unwrap_or_default(),
                    },
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
            "INSERT INTO users
             (owner, user_id, role, git_name, git_email, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(owner) DO UPDATE SET
                 user_id = excluded.user_id,
                 role = excluded.role,
                 git_name = excluded.git_name,
                 git_email = excluded.git_email,
                 created_at = excluded.created_at,
                 updated_at = excluded.updated_at",
            params![
                &user.profile.owner,
                &user.profile.user_id,
                &user.profile.role,
                &user.profile.git_name,
                &user.profile.git_email,
                user.profile.created_at,
                user.profile.updated_at,
            ],
        )
        .map_err(Self::qe)?;
        conn.execute(
            "INSERT INTO user_local_auth
             (user_id, password_hash, password_alg, password_updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(user_id) DO UPDATE SET
                 password_hash = excluded.password_hash,
                 password_alg = excluded.password_alg,
                 password_updated_at = excluded.password_updated_at",
            params![
                &user.auth.user_id,
                &user.auth.password_hash,
                &user.auth.password_alg,
                user.auth.password_updated_at,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn list_users(&self) -> Result<Vec<PlatformUser>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT user_id, owner, role, git_name, git_email, created_at, updated_at
                 FROM users ORDER BY owner ASC",
            )
            .map_err(Self::qe)?;
        let users = stmt
            .query_map([], |row| {
                Ok(PlatformUser {
                    user_id: row.get(0)?,
                    owner: row.get(1)?,
                    role: row.get(2)?,
                    git_name: row.get(3)?,
                    git_email: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
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
            "SELECT owner, project, project_id, owner_user_id, created_at, updated_at
             FROM projects WHERE owner = ?1 AND project = ?2",
            params![owner, project],
            |row| {
                Ok(PlatformProject {
                    owner: row.get(0)?,
                    project: row.get(1)?,
                    project_id: row.get(2)?,
                    owner_user_id: row.get(3)?,
                    title: String::new(), // populated from zebflow.json by ProjectService
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
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
            "INSERT INTO projects (owner, project, project_id, owner_user_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(owner, project) DO UPDATE SET
                 project_id = excluded.project_id,
                 owner_user_id = excluded.owner_user_id,
                 created_at = excluded.created_at,
                 updated_at = excluded.updated_at",
            params![
                &project.owner,
                &project.project,
                &project.project_id,
                &project.owner_user_id,
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
                "SELECT owner, project, project_id, owner_user_id, created_at, updated_at
                 FROM projects WHERE owner = ?1 ORDER BY project ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map(params![owner], |row| {
                Ok(PlatformProject {
                    owner: row.get(0)?,
                    project: row.get(1)?,
                    project_id: row.get(2)?,
                    owner_user_id: row.get(3)?,
                    title: String::new(),
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
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
            "INSERT INTO platform_marketplace_repositories
             (source_id, owner_user_id, owner, repository_id, title, base_url, remote_owner, remote_project, read_token, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(owner, repository_id) DO UPDATE SET
                 source_id = excluded.source_id,
                 owner_user_id = excluded.owner_user_id,
                 title = excluded.title,
                 base_url = excluded.base_url,
                 remote_owner = excluded.remote_owner,
                 remote_project = excluded.remote_project,
                 read_token = excluded.read_token,
                 enabled = excluded.enabled,
                 created_at = excluded.created_at,
                 updated_at = excluded.updated_at",
            params![
                &repository.source_id,
                &repository.owner_user_id,
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
                "SELECT source_id, owner_user_id, owner, repository_id, title, base_url, remote_owner, remote_project, read_token, enabled, created_at, updated_at
                 FROM platform_marketplace_repositories WHERE owner = ?1
                 ORDER BY title ASC, repository_id ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map(params![owner], |row| {
                Ok(PlatformMarketplaceRepository {
                    source_id: row.get(0)?,
                    owner_user_id: row.get(1)?,
                    owner: row.get(2)?,
                    repository_id: row.get(3)?,
                    title: row.get(4)?,
                    base_url: row.get(5)?,
                    remote_owner: row.get(6)?,
                    remote_project: row.get(7)?,
                    read_token: row.get(8)?,
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
            "INSERT INTO marketplace_publishers
             (authority_id, publisher_pk, owner, project, publisher_id, display_name, publisher_url, email, description, icon_url, website_url, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(owner, project, publisher_id) DO UPDATE SET
                 authority_id = excluded.authority_id,
                 publisher_pk = excluded.publisher_pk,
                 display_name = excluded.display_name,
                 publisher_url = excluded.publisher_url,
                 email = excluded.email,
                 description = excluded.description,
                 icon_url = excluded.icon_url,
                 website_url = excluded.website_url,
                 enabled = excluded.enabled,
                 created_at = excluded.created_at,
                 updated_at = excluded.updated_at",
            params![
                &publisher.authority_id,
                &publisher.publisher_pk,
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
                "SELECT authority_id, publisher_pk, owner, project, publisher_id, display_name, publisher_url, email, description, icon_url, website_url, enabled, created_at, updated_at
                 FROM marketplace_publishers WHERE owner = ?1 AND project = ?2
                 ORDER BY display_name ASC, publisher_id ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map(params![owner, project], |row| {
                Ok(MarketplacePublisher {
                    authority_id: row.get(0)?,
                    publisher_pk: row.get(1)?,
                    owner: row.get(2)?,
                    project: row.get(3)?,
                    publisher_id: row.get(4)?,
                    display_name: row.get(5)?,
                    publisher_url: row.get(6)?,
                    email: row.get(7)?,
                    description: row.get(8)?,
                    icon_url: row.get(9)?,
                    website_url: row.get(10)?,
                    enabled: row.get::<_, i64>(11)? != 0,
                    created_at: row.get(12)?,
                    updated_at: row.get(13)?,
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
                "SELECT authority_id, publisher_pk, owner, project, publisher_id, display_name, publisher_url, email, description, icon_url, website_url, enabled, created_at, updated_at
                 FROM marketplace_publishers WHERE owner = ?1 AND project = ?2 AND publisher_id = ?3",
            )
            .map_err(Self::qe)?;
        match stmt.query_row(params![owner, project, publisher_id], |row| {
            Ok(MarketplacePublisher {
                authority_id: row.get(0)?,
                publisher_pk: row.get(1)?,
                owner: row.get(2)?,
                project: row.get(3)?,
                publisher_id: row.get(4)?,
                display_name: row.get(5)?,
                publisher_url: row.get(6)?,
                email: row.get(7)?,
                description: row.get(8)?,
                icon_url: row.get(9)?,
                website_url: row.get(10)?,
                enabled: row.get::<_, i64>(11)? != 0,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
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
            "INSERT INTO marketplace_asset_packages
             (package_pk, authority_id, publisher_pk, package_id, authority_owner, authority_project, publisher_owner, publisher_id, publisher_display_name, publisher_url, publisher_email, asset_kind, title, description, visibility, tags_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
             ON CONFLICT(package_id) DO UPDATE SET
                 package_pk = excluded.package_pk,
                 authority_id = excluded.authority_id,
                 publisher_pk = excluded.publisher_pk,
                 authority_owner = excluded.authority_owner,
                 authority_project = excluded.authority_project,
                 publisher_owner = excluded.publisher_owner,
                 publisher_id = excluded.publisher_id,
                 publisher_display_name = excluded.publisher_display_name,
                 publisher_url = excluded.publisher_url,
                 publisher_email = excluded.publisher_email,
                 asset_kind = excluded.asset_kind,
                 title = excluded.title,
                 description = excluded.description,
                 visibility = excluded.visibility,
                 tags_json = excluded.tags_json,
                 created_at = excluded.created_at,
                 updated_at = excluded.updated_at",
            params![
                &package.package_pk,
                &package.authority_id,
                &package.publisher_pk,
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
                "SELECT package_pk, authority_id, publisher_pk, package_id, authority_owner, authority_project, publisher_owner, publisher_id, publisher_display_name, publisher_url, publisher_email, asset_kind, title, description, visibility, tags_json, created_at, updated_at
                 FROM marketplace_asset_packages
                 ORDER BY updated_at DESC, package_id ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map([], |row| {
                Ok(MarketplaceAssetPackage {
                    package_pk: row.get(0)?,
                    authority_id: row.get(1)?,
                    publisher_pk: row.get(2)?,
                    package_id: row.get(3)?,
                    authority_owner: row.get(4)?,
                    authority_project: row.get(5)?,
                    publisher_owner: row.get(6)?,
                    publisher_id: row.get(7)?,
                    publisher_display_name: row.get(8)?,
                    publisher_url: row.get(9)?,
                    publisher_email: row.get(10)?,
                    asset_kind: row.get(11)?,
                    title: row.get(12)?,
                    description: row.get(13)?,
                    visibility: row.get(14)?,
                    tags: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(15)?).unwrap_or_default(),
                    created_at: row.get(16)?,
                    updated_at: row.get(17)?,
                })
            })
            .map_err(Self::qe)?
            .filter_map(Result::ok)
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
                "SELECT package_pk, authority_id, publisher_pk, package_id, authority_owner, authority_project, publisher_owner, publisher_id, publisher_display_name, publisher_url, publisher_email, asset_kind, title, description, visibility, tags_json, created_at, updated_at
                 FROM marketplace_asset_packages WHERE package_id = ?1",
            )
            .map_err(Self::qe)?;
        match stmt.query_row(params![package_id], |row| {
            Ok(MarketplaceAssetPackage {
                package_pk: row.get(0)?,
                authority_id: row.get(1)?,
                publisher_pk: row.get(2)?,
                package_id: row.get(3)?,
                authority_owner: row.get(4)?,
                authority_project: row.get(5)?,
                publisher_owner: row.get(6)?,
                publisher_id: row.get(7)?,
                publisher_display_name: row.get(8)?,
                publisher_url: row.get(9)?,
                publisher_email: row.get(10)?,
                asset_kind: row.get(11)?,
                title: row.get(12)?,
                description: row.get(13)?,
                visibility: row.get(14)?,
                tags: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(15)?).unwrap_or_default(),
                created_at: row.get(16)?,
                updated_at: row.get(17)?,
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
            "INSERT INTO marketplace_asset_versions
             (package_pk, package_id, version, authority_owner, authority_project, publisher_owner, publisher_id, source_owner, source_project, source_kind, source_ref, artifact_rel_path, artifact_sha256, manifest_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT(package_id, version) DO UPDATE SET
                 package_pk = excluded.package_pk,
                 authority_owner = excluded.authority_owner,
                 authority_project = excluded.authority_project,
                 publisher_owner = excluded.publisher_owner,
                 publisher_id = excluded.publisher_id,
                 source_owner = excluded.source_owner,
                 source_project = excluded.source_project,
                 source_kind = excluded.source_kind,
                 source_ref = excluded.source_ref,
                 artifact_rel_path = excluded.artifact_rel_path,
                 artifact_sha256 = excluded.artifact_sha256,
                 manifest_json = excluded.manifest_json,
                 created_at = excluded.created_at",
            params![
                &version.package_pk,
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
                "SELECT package_pk, package_id, version, authority_owner, authority_project, publisher_owner, publisher_id, source_owner, source_project, source_kind, source_ref, artifact_rel_path, artifact_sha256, manifest_json, created_at
                 FROM marketplace_asset_versions WHERE package_id = ?1
                 ORDER BY created_at DESC, version DESC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map(params![package_id], |row| {
                Ok(MarketplaceAssetVersion {
                    package_pk: row.get(0)?,
                    package_id: row.get(1)?,
                    version: row.get(2)?,
                    authority_owner: row.get(3)?,
                    authority_project: row.get(4)?,
                    publisher_owner: row.get(5)?,
                    publisher_id: row.get(6)?,
                    source_owner: row.get(7)?,
                    source_project: row.get(8)?,
                    source_kind: row.get(9)?,
                    source_ref: row.get(10)?,
                    artifact_rel_path: row.get(11)?,
                    artifact_sha256: row.get(12)?,
                    manifest: serde_json::from_str::<Value>(&row.get::<_, String>(13)?).unwrap_or(Value::Null),
                    created_at: row.get(14)?,
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
                "SELECT package_pk, package_id, version, authority_owner, authority_project, publisher_owner, publisher_id, source_owner, source_project, source_kind, source_ref, artifact_rel_path, artifact_sha256, manifest_json, created_at
                 FROM marketplace_asset_versions WHERE package_id = ?1 AND version = ?2",
            )
            .map_err(Self::qe)?;
        match stmt.query_row(params![package_id, version], |row| {
            Ok(MarketplaceAssetVersion {
                package_pk: row.get(0)?,
                package_id: row.get(1)?,
                version: row.get(2)?,
                authority_owner: row.get(3)?,
                authority_project: row.get(4)?,
                publisher_owner: row.get(5)?,
                publisher_id: row.get(6)?,
                source_owner: row.get(7)?,
                source_project: row.get(8)?,
                source_kind: row.get(9)?,
                source_ref: row.get(10)?,
                artifact_rel_path: row.get(11)?,
                artifact_sha256: row.get(12)?,
                manifest: serde_json::from_str::<Value>(&row.get::<_, String>(13)?).unwrap_or(Value::Null),
                created_at: row.get(14)?,
            })
        }) {
            Ok(item) => Ok(Some(item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Self::qe(e)),
        }
    }

    fn put_marketplace_token(&self, token: &MarketplaceToken) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO marketplace_tokens
             (token_id, authority_id, publisher_pk, owner, project, publisher_id, publisher_display_name, publisher_url, publisher_email, title, secret_hash, scope_read, scope_publish, scope_manage, expires_at, last_used_at, revoked_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
             ON CONFLICT(token_id) DO UPDATE SET
                 authority_id = excluded.authority_id,
                 publisher_pk = excluded.publisher_pk,
                 owner = excluded.owner,
                 project = excluded.project,
                 publisher_id = excluded.publisher_id,
                 publisher_display_name = excluded.publisher_display_name,
                 publisher_url = excluded.publisher_url,
                 publisher_email = excluded.publisher_email,
                 title = excluded.title,
                 secret_hash = excluded.secret_hash,
                 scope_read = excluded.scope_read,
                 scope_publish = excluded.scope_publish,
                 scope_manage = excluded.scope_manage,
                 expires_at = excluded.expires_at,
                 last_used_at = excluded.last_used_at,
                 revoked_at = excluded.revoked_at,
                 created_at = excluded.created_at,
                 updated_at = excluded.updated_at",
            params![
                &token.token_id,
                &token.authority_id,
                &token.publisher_pk,
                &token.owner,
                &token.project,
                &token.publisher_id,
                &token.publisher_display_name,
                &token.publisher_url,
                &token.publisher_email,
                &token.title,
                &token.secret_hash,
                if token.scope_read { 1 } else { 0 },
                if token.scope_publish { 1 } else { 0 },
                if token.scope_manage { 1 } else { 0 },
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
                "SELECT token_id, authority_id, publisher_pk, owner, project, publisher_id, publisher_display_name, publisher_url, publisher_email, title, secret_hash, scope_read, scope_publish, scope_manage, expires_at, last_used_at, revoked_at, created_at, updated_at
                 FROM marketplace_tokens WHERE token_id = ?1",
            )
            .map_err(Self::qe)?;
        match stmt.query_row(params![token_id], |row| {
            {
                let scope_read = row.get::<_, i64>(11)? != 0;
                let scope_publish = row.get::<_, i64>(12)? != 0;
                let scope_manage = row.get::<_, i64>(13)? != 0;
                Ok(MarketplaceToken {
                    token_id: row.get(0)?,
                    authority_id: row.get(1)?,
                    publisher_pk: row.get(2)?,
                    owner: row.get(3)?,
                    project: row.get(4)?,
                    publisher_id: row.get(5)?,
                    publisher_display_name: row.get(6)?,
                    publisher_url: row.get(7)?,
                    publisher_email: row.get(8)?,
                    title: row.get(9)?,
                    secret_hash: row.get(10)?,
                    scopes: Self::marketplace_token_scopes(
                        scope_read,
                        scope_publish,
                        scope_manage,
                    ),
                    scope_read,
                    scope_publish,
                    scope_manage,
                    expires_at: row.get(14)?,
                    last_used_at: row.get(15)?,
                    revoked_at: row.get(16)?,
                    created_at: row.get(17)?,
                    updated_at: row.get(18)?,
                })
            }
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
                "SELECT token_id, authority_id, publisher_pk, owner, project, publisher_id, publisher_display_name, publisher_url, publisher_email, title, secret_hash, scope_read, scope_publish, scope_manage, expires_at, last_used_at, revoked_at, created_at, updated_at
                 FROM marketplace_tokens WHERE owner = ?1 AND project = ?2
                 ORDER BY updated_at DESC, token_id ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map(params![owner, project], |row| {
                {
                    let scope_read = row.get::<_, i64>(11)? != 0;
                    let scope_publish = row.get::<_, i64>(12)? != 0;
                    let scope_manage = row.get::<_, i64>(13)? != 0;
                    Ok(MarketplaceToken {
                        token_id: row.get(0)?,
                        authority_id: row.get(1)?,
                        publisher_pk: row.get(2)?,
                        owner: row.get(3)?,
                        project: row.get(4)?,
                        publisher_id: row.get(5)?,
                        publisher_display_name: row.get(6)?,
                        publisher_url: row.get(7)?,
                        publisher_email: row.get(8)?,
                        title: row.get(9)?,
                        secret_hash: row.get(10)?,
                        scopes: Self::marketplace_token_scopes(
                            scope_read,
                            scope_publish,
                            scope_manage,
                        ),
                        scope_read,
                        scope_publish,
                        scope_manage,
                        expires_at: row.get(14)?,
                        last_used_at: row.get(15)?,
                        revoked_at: row.get(16)?,
                        created_at: row.get(17)?,
                        updated_at: row.get(18)?,
                    })
                }
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
            "INSERT INTO project_policies
             (project_id, owner, project, policy_id, title, capabilities_json, managed, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(owner, project, policy_id) DO UPDATE SET
                 project_id = excluded.project_id,
                 title = excluded.title,
                 capabilities_json = excluded.capabilities_json,
                 managed = excluded.managed,
                 created_at = excluded.created_at,
                 updated_at = excluded.updated_at",
            params![
                &policy.project_id,
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
                "SELECT project_id, owner, project, policy_id, title, capabilities_json, managed, created_at, updated_at
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
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                ))
            })
            .map_err(Self::qe)?
            .filter_map(|r| r.ok())
            .filter_map(
                |(project_id, owner, project, policy_id, title, caps_json, managed, created_at, updated_at)| {
                    let capabilities: Vec<ProjectCapability> =
                        serde_json::from_str(&caps_json).unwrap_or_default();
                    Some(ProjectPolicy {
                        project_id,
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
            "INSERT INTO project_policy_bindings
             (project_id, owner, project, subject_kind, subject_id, policy_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(owner, project, subject_id, policy_id) DO UPDATE SET
                 project_id = excluded.project_id,
                 subject_kind = excluded.subject_kind,
                 created_at = excluded.created_at,
                 updated_at = excluded.updated_at",
            params![
                &binding.project_id,
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
                "SELECT project_id, owner, project, subject_kind, subject_id, policy_id, created_at, updated_at
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
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            })
            .map_err(Self::qe)?
            .filter_map(|r| r.ok())
            .filter_map(
                |(
                    project_id,
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
                        project_id,
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
            "SELECT project_id, owner, project, user_id, member_user_id, role_preset, custom_policy_ids_json, mcp_capabilities_json, created_by, created_by_user_id, created_at, updated_at
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
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, i64>(11)?,
                ))
            },
        );
        match result {
            Ok((
                project_id,
                owner,
                project,
                user_id,
                member_user_id,
                role_preset,
                custom_policy_ids_json,
                mcp_capabilities_json,
                created_by,
                created_by_user_id,
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
                    project_id,
                    owner,
                    project,
                    user_id,
                    member_user_id,
                    role_preset,
                    custom_policy_ids,
                    mcp_capability_ceiling,
                    created_by,
                    created_by_user_id,
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
            "INSERT INTO project_members
             (project_id, owner, project, user_id, member_user_id, role_preset, custom_policy_ids_json, mcp_capabilities_json, created_by, created_by_user_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(owner, project, user_id) DO UPDATE SET
                 project_id = excluded.project_id,
                 member_user_id = excluded.member_user_id,
                 role_preset = excluded.role_preset,
                 custom_policy_ids_json = excluded.custom_policy_ids_json,
                 mcp_capabilities_json = excluded.mcp_capabilities_json,
                 created_by = excluded.created_by,
                 created_by_user_id = excluded.created_by_user_id,
                 created_at = excluded.created_at,
                 updated_at = excluded.updated_at",
            params![
                &member.project_id,
                &member.owner,
                &member.project,
                &member.user_id,
                &member.member_user_id,
                member.role_preset.key(),
                custom_policy_ids_json,
                mcp_capabilities_json,
                &member.created_by,
                &member.created_by_user_id,
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
                "SELECT project_id, owner, project, user_id, member_user_id, role_preset, custom_policy_ids_json, mcp_capabilities_json, created_by, created_by_user_id, created_at, updated_at
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
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, i64>(11)?,
                ))
            })
            .map_err(Self::qe)?
            .filter_map(|r| r.ok())
            .filter_map(
                |(
                    project_id,
                    owner,
                    project,
                    user_id,
                    member_user_id,
                    role_preset,
                    custom_policy_ids_json,
                    mcp_capabilities_json,
                    created_by,
                    created_by_user_id,
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
                        project_id,
                        owner,
                        project,
                        user_id,
                        member_user_id,
                        role_preset,
                        custom_policy_ids,
                        mcp_capability_ceiling,
                        created_by,
                        created_by_user_id,
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

    fn get_marketplace_authority(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Option<MarketplaceAuthority>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let result = conn.query_row(
            "SELECT authority_id, host_project_id, owner, project, enabled, public_base_url, created_at, updated_at
             FROM marketplace_authorities
             WHERE owner = ?1 AND project = ?2",
            params![owner, project],
            |row| {
                Ok(MarketplaceAuthority {
                    authority_id: row.get(0)?,
                    host_project_id: row.get(1)?,
                    owner: row.get(2)?,
                    project: row.get(3)?,
                    enabled: row.get::<_, i64>(4)? != 0,
                    public_base_url: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            },
        );
        match result {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(Self::qe(err)),
        }
    }

    fn put_marketplace_authority(
        &self,
        authority: &MarketplaceAuthority,
    ) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO marketplace_authorities
             (authority_id, host_project_id, owner, project, enabled, public_base_url, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(authority_id) DO UPDATE SET
                 host_project_id = excluded.host_project_id,
                 owner = excluded.owner,
                 project = excluded.project,
                 enabled = excluded.enabled,
                 public_base_url = excluded.public_base_url,
                 created_at = excluded.created_at,
                 updated_at = excluded.updated_at",
            params![
                &authority.authority_id,
                &authority.host_project_id,
                &authority.owner,
                &authority.project,
                if authority.enabled { 1 } else { 0 },
                &authority.public_base_url,
                authority.created_at,
                authority.updated_at,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn list_marketplace_authorities(&self) -> Result<Vec<MarketplaceAuthority>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT authority_id, host_project_id, owner, project, enabled, public_base_url, created_at, updated_at
                 FROM marketplace_authorities
                 ORDER BY owner ASC, project ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map([], |row| {
                Ok(MarketplaceAuthority {
                    authority_id: row.get(0)?,
                    host_project_id: row.get(1)?,
                    owner: row.get(2)?,
                    project: row.get(3)?,
                    enabled: row.get::<_, i64>(4)? != 0,
                    public_base_url: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })
            .map_err(Self::qe)?
            .filter_map(|row| row.ok())
            .collect();
        Ok(items)
    }

    fn get_platform_office(&self, office_id: &str) -> Result<Option<PlatformOffice>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let result = conn.query_row(
            "SELECT office_id, office_slug, label, office_kind, base_url, status, created_at, updated_at
             FROM offices
             WHERE office_id = ?1",
            params![office_id],
            |row| {
                Ok(PlatformOffice {
                    office_id: row.get(0)?,
                    office_slug: row.get(1)?,
                    label: row.get(2)?,
                    office_kind: row.get(3)?,
                    base_url: row.get(4)?,
                    status: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            },
        );
        match result {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(Self::qe(err)),
        }
    }

    fn put_platform_office(&self, office: &PlatformOffice) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO offices
             (office_id, office_slug, label, office_kind, base_url, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(office_id) DO UPDATE SET
                 office_slug = excluded.office_slug,
                 label = excluded.label,
                 office_kind = excluded.office_kind,
                 base_url = excluded.base_url,
                 status = excluded.status,
                 created_at = excluded.created_at,
                 updated_at = excluded.updated_at",
            params![
                &office.office_id,
                &office.office_slug,
                &office.label,
                &office.office_kind,
                &office.base_url,
                &office.status,
                office.created_at,
                office.updated_at,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn list_platform_offices(&self) -> Result<Vec<PlatformOffice>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT office_id, office_slug, label, office_kind, base_url, status, created_at, updated_at
                 FROM offices
                 ORDER BY office_slug ASC, office_id ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map([], |row| {
                Ok(PlatformOffice {
                    office_id: row.get(0)?,
                    office_slug: row.get(1)?,
                    label: row.get(2)?,
                    office_kind: row.get(3)?,
                    base_url: row.get(4)?,
                    status: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })
            .map_err(Self::qe)?
            .filter_map(|row| row.ok())
            .collect();
        Ok(items)
    }

    fn get_platform_office_node(
        &self,
        node_id: &str,
    ) -> Result<Option<PlatformOfficeNode>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let result = conn.query_row(
            "SELECT office_id, node_id, label, base_url, status, capabilities_json, registered_at, last_heartbeat_at
             FROM office_nodes
             WHERE node_id = ?1",
            params![node_id],
            |row| {
                let capabilities_json: String = row.get(5)?;
                Ok(PlatformOfficeNode {
                    office_id: row.get(0)?,
                    node_id: row.get(1)?,
                    label: row.get(2)?,
                    base_url: row.get(3)?,
                    status: row.get(4)?,
                    capabilities: serde_json::from_str(&capabilities_json).unwrap_or_default(),
                    registered_at: row.get(6)?,
                    last_heartbeat_at: row.get(7)?,
                })
            },
        );
        match result {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(Self::qe(err)),
        }
    }

    fn put_platform_office_node(&self, node: &PlatformOfficeNode) -> Result<(), PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let capabilities_json =
            serde_json::to_string(&node.capabilities).unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "INSERT INTO office_nodes
             (office_id, node_id, label, base_url, status, capabilities_json, registered_at, last_heartbeat_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(node_id) DO UPDATE SET
                 office_id = excluded.office_id,
                 label = excluded.label,
                 base_url = excluded.base_url,
                 status = excluded.status,
                 capabilities_json = excluded.capabilities_json,
                 registered_at = excluded.registered_at,
                 last_heartbeat_at = excluded.last_heartbeat_at",
            params![
                &node.office_id,
                &node.node_id,
                &node.label,
                &node.base_url,
                &node.status,
                capabilities_json,
                node.registered_at,
                node.last_heartbeat_at,
            ],
        )
        .map_err(Self::qe)?;
        Ok(())
    }

    fn list_platform_office_nodes(&self) -> Result<Vec<PlatformOfficeNode>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT office_id, node_id, label, base_url, status, capabilities_json, registered_at, last_heartbeat_at
                 FROM office_nodes
                 ORDER BY office_id ASC, node_id ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map([], |row| {
                let capabilities_json: String = row.get(5)?;
                Ok(PlatformOfficeNode {
                    office_id: row.get(0)?,
                    node_id: row.get(1)?,
                    label: row.get(2)?,
                    base_url: row.get(3)?,
                    status: row.get(4)?,
                    capabilities: serde_json::from_str(&capabilities_json).unwrap_or_default(),
                    registered_at: row.get(6)?,
                    last_heartbeat_at: row.get(7)?,
                })
            })
            .map_err(Self::qe)?
            .filter_map(|row| row.ok())
            .collect();
        Ok(items)
    }

    fn get_worker_registry_record(
        &self,
        node_id: &str,
    ) -> Result<Option<WorkerRegistryRecord>, PlatformError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let result = conn.query_row(
            "SELECT office_id, office_slug, node_id, label, base_url, status, capabilities_json, registered_at, last_heartbeat_at
             FROM worker_registry
             WHERE node_id = ?1",
            params![node_id],
            |row| {
                let capabilities_json: String = row.get(6)?;
                let capabilities = serde_json::from_str(&capabilities_json).unwrap_or_default();
                Ok(WorkerRegistryRecord {
                    office_id: row.get(0)?,
                    office_slug: row.get(1)?,
                    node_id: row.get(2)?,
                    label: row.get(3)?,
                    base_url: row.get(4)?,
                    status: row.get(5)?,
                    capabilities,
                    registered_at: row.get(7)?,
                    last_heartbeat_at: row.get(8)?,
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
             (office_id, office_slug, node_id, label, base_url, status, capabilities_json, registered_at, last_heartbeat_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                &record.office_id,
                &record.office_slug,
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
                "SELECT office_id, office_slug, node_id, label, base_url, status, capabilities_json, registered_at, last_heartbeat_at
                 FROM worker_registry
                 ORDER BY node_id ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map([], |row| {
                let capabilities_json: String = row.get(6)?;
                let capabilities = serde_json::from_str(&capabilities_json).unwrap_or_default();
                Ok(WorkerRegistryRecord {
                    office_id: row.get(0)?,
                    office_slug: row.get(1)?,
                    node_id: row.get(2)?,
                    label: row.get(3)?,
                    base_url: row.get(4)?,
                    status: row.get(5)?,
                    capabilities,
                    registered_at: row.get(7)?,
                    last_heartbeat_at: row.get(8)?,
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
            "SELECT project_id, owner, project, mode, target, target_office_id, target_node_id, worker_id, resource_profile, desired_replicas, effective_state, created_at, updated_at
             FROM project_runtime_placements
             WHERE owner = ?1 AND project = ?2",
            params![owner, project],
            |row| {
                let mode: String = row.get(3)?;
                let target: String = row.get(4)?;
                Ok(ProjectRuntimePlacement {
                    project_id: row.get(0)?,
                    owner: row.get(1)?,
                    project: row.get(2)?,
                    mode: serde_json::from_value(Value::String(mode))
                        .unwrap_or(ProjectRuntimeMode::Shared),
                    target: serde_json::from_value(Value::String(target))
                        .unwrap_or(ProjectRuntimePlacementTarget::Local),
                    target_office_id: row.get(5)?,
                    target_node_id: row.get(6)?,
                    worker_id: row.get::<_, Option<String>>(7)?.or_else(|| row.get::<_, Option<String>>(6).ok().flatten()),
                    resource_profile: row.get(8)?,
                    desired_replicas: row.get::<_, i64>(9)?.max(0) as u32,
                    effective_state: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
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
            "INSERT INTO project_runtime_placements
             (project_id, owner, project, mode, target, target_office_id, target_node_id, worker_id, resource_profile, desired_replicas, effective_state, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(owner, project) DO UPDATE SET
                 project_id = excluded.project_id,
                 mode = excluded.mode,
                 target = excluded.target,
                 target_office_id = excluded.target_office_id,
                 target_node_id = excluded.target_node_id,
                 worker_id = excluded.worker_id,
                 resource_profile = excluded.resource_profile,
                 desired_replicas = excluded.desired_replicas,
                 effective_state = excluded.effective_state,
                 created_at = excluded.created_at,
                 updated_at = excluded.updated_at",
            params![
                &placement.project_id,
                &placement.owner,
                &placement.project,
                mode,
                target,
                &placement.target_office_id,
                &placement.target_node_id,
                &placement.worker_id,
                &placement.resource_profile,
                i64::from(placement.desired_replicas),
                &placement.effective_state,
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
                "SELECT project_id, owner, project, mode, target, target_office_id, target_node_id, worker_id, resource_profile, desired_replicas, effective_state, created_at, updated_at
                 FROM project_runtime_placements
                 ORDER BY owner ASC, project ASC",
            )
            .map_err(Self::qe)?;
        let items = stmt
            .query_map([], |row| {
                let mode: String = row.get(3)?;
                let target: String = row.get(4)?;
                Ok(ProjectRuntimePlacement {
                    project_id: row.get(0)?,
                    owner: row.get(1)?,
                    project: row.get(2)?,
                    mode: serde_json::from_value(Value::String(mode))
                        .unwrap_or(ProjectRuntimeMode::Shared),
                    target: serde_json::from_value(Value::String(target))
                        .unwrap_or(ProjectRuntimePlacementTarget::Local),
                    target_office_id: row.get(5)?,
                    target_node_id: row.get(6)?,
                    worker_id: row.get::<_, Option<String>>(7)?.or_else(|| row.get::<_, Option<String>>(6).ok().flatten()),
                    resource_profile: row.get(8)?,
                    desired_replicas: row.get::<_, i64>(9)?.max(0) as u32,
                    effective_state: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
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
            "SELECT project_id, owner, project, operation_id, kind, status, current_step, source_office_id, target_office_id, artifact_rel_path, artifact_sha256, artifact_bytes, error_message, retry_count, created_at, updated_at, completed_at
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
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<u64>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, u32>(13)?,
                    row.get::<_, i64>(14)?,
                    row.get::<_, i64>(15)?,
                    row.get::<_, Option<i64>>(16)?,
                ))
            },
        );
        match result {
            Ok((
                project_id,
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
                project_id,
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
            "INSERT INTO project_operations
             (project_id, owner, project, operation_id, kind, status, current_step, source_office_id, target_office_id, artifact_rel_path, artifact_sha256, artifact_bytes, error_message, retry_count, created_at, updated_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
             ON CONFLICT(owner, project, operation_id) DO UPDATE SET
                 project_id = excluded.project_id,
                 kind = excluded.kind,
                 status = excluded.status,
                 current_step = excluded.current_step,
                 source_office_id = excluded.source_office_id,
                 target_office_id = excluded.target_office_id,
                 artifact_rel_path = excluded.artifact_rel_path,
                 artifact_sha256 = excluded.artifact_sha256,
                 artifact_bytes = excluded.artifact_bytes,
                 error_message = excluded.error_message,
                 retry_count = excluded.retry_count,
                 created_at = excluded.created_at,
                 updated_at = excluded.updated_at,
                 completed_at = excluded.completed_at",
            params![
                &record.project_id,
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
                "SELECT project_id, owner, project, operation_id, kind, status, current_step, source_office_id, target_office_id, artifact_rel_path, artifact_sha256, artifact_bytes, error_message, retry_count, created_at, updated_at, completed_at
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
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<u64>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, u32>(13)?,
                    row.get::<_, i64>(14)?,
                    row.get::<_, i64>(15)?,
                    row.get::<_, Option<i64>>(16)?,
                ))
            })
            .map_err(Self::qe)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Self::qe)?;
        let mut out = Vec::with_capacity(items.len());
        for (
            project_id,
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
                project_id,
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

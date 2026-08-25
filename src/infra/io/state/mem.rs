//! In-memory [`StateBus`] implementation backed by the existing [`crate::infra::mem::MemHub`],
//! with optional SQLite-backed durable KV storage.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use serde_json::Value;

use crate::infra::mem::MemHub;

use super::interface::{
    StateBus, StateBusCapabilities, StateBusError, StateBusStats, StateSubscription,
};

const KV_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS kv (
    namespace TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    expires_at INTEGER,
    PRIMARY KEY (namespace, key)
);";

/// Durable KV storage: one SQLite file per project namespace.
///
/// Each project's durable `n.kv.*` state lives at
/// `{data_root}/users/{owner}/{project}/data/store/kv.db` — the `store` tier
/// of `docs/contracts/project-directory.md` — so a project backup, transfer,
/// or removal carries its durable state with the project instead of leaving
/// it behind in an instance-global file. The path shape mirrors
/// `ProjectFileLayout::data_store_kv_db_file`; it is rebuilt here because this
/// module is infra and cannot see platform types, and `project-directory.md`
/// §5 names both sites so a layout change updates both.
struct DurableKv {
    /// `{data_root}/users` — per-project files live below this root.
    users_root: PathBuf,
    /// `{data_root}/kv_durable.db` — the pre-tier global file every project's
    /// rows used to share. Never created by this code: it is drained one
    /// namespace at a time on first touch and removed once no rows remain.
    legacy_path: PathBuf,
    /// Open per-namespace connections, keyed `{owner}/{project}`.
    conns: Mutex<HashMap<String, Arc<Mutex<Connection>>>>,
}

impl DurableKv {
    fn new(data_root: PathBuf) -> Self {
        Self {
            users_root: data_root.join("users"),
            legacy_path: data_root.join("kv_durable.db"),
            conns: Mutex::new(HashMap::new()),
        }
    }

    /// `users/{owner}/{project}/data/store/kv.db`.
    ///
    /// `owner` and `project` have already passed namespace validation, which
    /// admits only ASCII alphanumerics, `_`, `-`, and `.`, and refuses
    /// segments made solely of dots — so neither can traverse out of
    /// `users_root`.
    fn project_kv_path(&self, owner: &str, project: &str) -> PathBuf {
        self.users_root
            .join(owner)
            .join(project)
            .join("data")
            .join("store")
            .join("kv.db")
    }

    /// Get or open this namespace's durable connection, migrating its rows
    /// out of the pre-tier global file first if any are still there.
    fn connection(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Arc<Mutex<Connection>>, StateBusError> {
        let ns = MemStateBus::namespace(owner, project);
        let mut conns = self.conns.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(conn) = conns.get(&ns) {
            return Ok(conn.clone());
        }
        let path = self.project_kv_path(owner, project);
        self.migrate_legacy_namespace(&ns, &path)?;
        let conn = open_kv_database(&path)?;
        let conn = Arc::new(Mutex::new(conn));
        conns.insert(ns, conn.clone());
        Ok(conn)
    }

    /// Moves this namespace's rows out of the pre-tier global `kv_durable.db`.
    ///
    /// The copy into the per-project file and the delete from the global file
    /// are one transaction across both databases (SQLite `ATTACH`), so a crash
    /// leaves the namespace's rows wholly in one file or wholly in the other —
    /// never split, never doubled. An interrupted attempt can leave an *empty*
    /// per-project file behind (`ATTACH` creates it before the transaction
    /// commits); a file with zero rows holds no state, so proceeding into it
    /// is completion, not a guess. A per-project file that already **holds
    /// rows** while the global file still has rows for the same namespace is
    /// genuinely ambiguous and refused, the same rule `migrate_tier_entry`
    /// applies to directory moves.
    ///
    /// Once the global file holds no rows at all it is removed. Rows whose
    /// project no longer exists keep it alive untouched: deleting them would
    /// destroy their only copy, and moving them would resurrect a removed
    /// project's directory.
    fn migrate_legacy_namespace(&self, ns: &str, dest: &Path) -> Result<(), StateBusError> {
        if !self.legacy_path.exists() {
            return Ok(());
        }
        let migrate_err =
            |e: rusqlite::Error| StateBusError::new("STATE_BUS_DURABLE_MIGRATE", e.to_string());
        let mut legacy = Connection::open(&self.legacy_path).map_err(migrate_err)?;
        let pending = count_kv_rows(&legacy, Some(ns)).map_err(migrate_err)?;
        if pending > 0 {
            if dest.exists() && count_kv_rows_in_file(dest)? > 0 {
                return Err(StateBusError::new(
                    "STATE_BUS_DURABLE_MIGRATE",
                    format!(
                        "both '{}' (rows for '{ns}') and '{}' hold state; refusing to \
                         guess which is current — remove or drain the stale one by hand \
                         once its content is confirmed",
                        self.legacy_path.display(),
                        dest.display()
                    ),
                ));
            }
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    StateBusError::new(
                        "STATE_BUS_DURABLE_MIGRATE",
                        format!("failed to create dir: {e}"),
                    )
                })?;
            }
            let dest_str = dest.to_str().ok_or_else(|| {
                StateBusError::new(
                    "STATE_BUS_DURABLE_MIGRATE",
                    format!("non-UTF-8 destination path: {}", dest.display()),
                )
            })?;
            legacy
                .execute("ATTACH DATABASE ?1 AS dest", [dest_str])
                .map_err(migrate_err)?;
            let moved = (|| -> Result<(), rusqlite::Error> {
                legacy.execute_batch(
                    "CREATE TABLE IF NOT EXISTS dest.kv (
                        namespace TEXT NOT NULL,
                        key TEXT NOT NULL,
                        value TEXT NOT NULL,
                        expires_at INTEGER,
                        PRIMARY KEY (namespace, key)
                    );",
                )?;
                let tx = legacy.transaction()?;
                tx.execute(
                    "INSERT INTO dest.kv (namespace, key, value, expires_at)
                     SELECT namespace, key, value, expires_at FROM kv WHERE namespace = ?1",
                    [ns],
                )?;
                tx.execute("DELETE FROM kv WHERE namespace = ?1", [ns])?;
                tx.commit()
            })();
            let _ = legacy.execute_batch("DETACH DATABASE dest;");
            moved.map_err(migrate_err)?;
        }
        let remaining = count_kv_rows(&legacy, None).map_err(migrate_err)?;
        drop(legacy);
        if remaining == 0 {
            // Fully drained: the global file may go. WAL siblings follow it.
            let _ = std::fs::remove_file(&self.legacy_path);
            for suffix in ["-wal", "-shm"] {
                let mut sibling = self.legacy_path.as_os_str().to_owned();
                sibling.push(suffix);
                let _ = std::fs::remove_file(PathBuf::from(sibling));
            }
        }
        Ok(())
    }
}

/// Counts `kv` rows, for one namespace or in total. A database without the
/// `kv` table has zero rows.
fn count_kv_rows(conn: &Connection, ns: Option<&str>) -> Result<i64, rusqlite::Error> {
    let has_table: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'kv'",
        [],
        |row| row.get(0),
    )?;
    if has_table == 0 {
        return Ok(0);
    }
    match ns {
        Some(ns) => conn.query_row(
            "SELECT COUNT(*) FROM kv WHERE namespace = ?1",
            [ns],
            |row| row.get(0),
        ),
        None => conn.query_row("SELECT COUNT(*) FROM kv", [], |row| row.get(0)),
    }
}

fn count_kv_rows_in_file(path: &Path) -> Result<i64, StateBusError> {
    let conn = Connection::open(path).map_err(|e| {
        StateBusError::new(
            "STATE_BUS_DURABLE_MIGRATE",
            format!("failed to open db: {e}"),
        )
    })?;
    count_kv_rows(&conn, None)
        .map_err(|e| StateBusError::new("STATE_BUS_DURABLE_MIGRATE", e.to_string()))
}

/// Opens (creating if absent) one per-project durable KV database.
fn open_kv_database(path: &Path) -> Result<Connection, StateBusError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            StateBusError::new(
                "STATE_BUS_DURABLE_INIT",
                format!("failed to create dir: {e}"),
            )
        })?;
    }
    let conn = Connection::open(path).map_err(|e| {
        StateBusError::new("STATE_BUS_DURABLE_INIT", format!("failed to open db: {e}"))
    })?;
    conn.execute_batch(&format!("PRAGMA journal_mode=WAL;\n{KV_SCHEMA}"))
        .map_err(|e| {
            StateBusError::new(
                "STATE_BUS_DURABLE_INIT",
                format!("failed to init schema: {e}"),
            )
        })?;
    Ok(conn)
}

/// `StateBus` adapter over the existing in-process `MemHub`, with optional durable SQLite KV.
#[derive(Clone)]
pub struct MemStateBus {
    hub: MemHub,
    /// Per-project durable KV storage. If set, durable operations are available.
    /// Shared across clones so every clone sees one connection per namespace.
    durable: Option<Arc<DurableKv>>,
}

impl Default for MemStateBus {
    fn default() -> Self {
        Self::new()
    }
}

impl MemStateBus {
    /// Create a new in-memory state bus (no durable storage).
    pub fn new() -> Self {
        Self {
            hub: MemHub::new(),
            durable: None,
        }
    }

    /// Create a new state bus with per-project durable SQLite storage under
    /// the given data root (`users/{owner}/{project}/data/store/kv.db`).
    pub fn new_with_durable(data_root: PathBuf) -> Self {
        Self {
            hub: MemHub::new(),
            durable: Some(Arc::new(DurableKv::new(data_root))),
        }
    }

    /// Wrap an existing `MemHub` with optional durable path.
    pub fn from_hub(hub: MemHub) -> Self {
        Self { hub, durable: None }
    }

    /// Wrap an existing `MemHub` with per-project durable storage under the
    /// given data root.
    pub fn from_hub_with_durable(hub: MemHub, data_root: PathBuf) -> Self {
        Self {
            hub,
            durable: Some(Arc::new(DurableKv::new(data_root))),
        }
    }

    /// Borrow the inner `MemHub`.
    pub fn hub(&self) -> &MemHub {
        &self.hub
    }

    /// Get or open the durable SQLite connection for one project namespace.
    fn db(&self, owner: &str, project: &str) -> Result<Arc<Mutex<Connection>>, StateBusError> {
        let durable = self.durable.as_deref().ok_or_else(|| {
            StateBusError::new(
                "STATE_BUS_NO_DURABLE",
                "durable storage path not configured",
            )
        })?;
        durable.connection(owner, project)
    }

    fn namespace(owner: &str, project: &str) -> String {
        format!("{}/{}", owner, project)
    }

    fn now_unix() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }
}

impl StateBus for MemStateBus {
    fn id(&self) -> &'static str {
        "state.mem"
    }

    fn capabilities(&self) -> StateBusCapabilities {
        StateBusCapabilities {
            ttl: true,
            atomic_incr: true,
            pubsub: true,
            durable: self.durable.is_some(),
            shared_coordination: false,
        }
    }

    fn stats(&self) -> Result<StateBusStats, StateBusError> {
        let stats = self.hub.stats();
        Ok(StateBusStats {
            implementation: self.id().to_string(),
            entry_count: stats.entry_count,
            channel_count: stats.channel_count,
            subscriber_count: stats.subscriber_count,
        })
    }

    fn get(&self, owner: &str, project: &str, key: &str) -> Result<Option<Value>, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        Ok(self.hub.get(owner, project, key))
    }

    fn set(
        &self,
        owner: &str,
        project: &str,
        key: &str,
        value: Value,
        ttl_secs: Option<u64>,
    ) -> Result<(), StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        self.hub.set(owner, project, key, value, ttl_secs);
        Ok(())
    }

    fn exists(&self, owner: &str, project: &str, key: &str) -> Result<bool, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        Ok(self.hub.exists(owner, project, key))
    }

    fn expire(
        &self,
        owner: &str,
        project: &str,
        key: &str,
        ttl_secs: Option<u64>,
    ) -> Result<bool, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        Ok(self.hub.expire(owner, project, key, ttl_secs))
    }

    fn incr(
        &self,
        owner: &str,
        project: &str,
        key: &str,
        amount: i64,
    ) -> Result<i64, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        Ok(self.hub.incr(owner, project, key, amount))
    }

    fn del(&self, owner: &str, project: &str, key: &str) -> Result<bool, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        Ok(self.hub.del(owner, project, key))
    }

    fn publish(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        message: Value,
    ) -> Result<usize, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_channel(channel)?;
        Ok(self.hub.publish(owner, project, channel, message))
    }

    fn subscribe(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> Result<StateSubscription, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_channel(channel)?;
        Ok(self.hub.subscribe(owner, project, channel))
    }

    // ── Durable KV (SQLite-backed) ─────────────────────────────────────────

    fn durable_get(
        &self,
        owner: &str,
        project: &str,
        key: &str,
    ) -> Result<Option<Value>, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        let db = self.db(owner, project)?;
        let conn = db
            .lock()
            .map_err(|e| StateBusError::new("STATE_BUS_DURABLE", e.to_string()))?;
        let ns = Self::namespace(owner, project);
        let now = Self::now_unix();
        let result: Result<String, _> = conn.query_row(
            "SELECT value FROM kv WHERE namespace = ?1 AND key = ?2 AND (expires_at IS NULL OR expires_at > ?3)",
            rusqlite::params![ns, key, now],
            |row| row.get(0),
        );
        match result {
            Ok(json_str) => {
                let val = serde_json::from_str(&json_str).unwrap_or(Value::Null);
                Ok(Some(val))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StateBusError::new("STATE_BUS_DURABLE", e.to_string())),
        }
    }

    fn durable_set(
        &self,
        owner: &str,
        project: &str,
        key: &str,
        value: Value,
        ttl_secs: Option<u64>,
    ) -> Result<(), StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        let db = self.db(owner, project)?;
        let conn = db
            .lock()
            .map_err(|e| StateBusError::new("STATE_BUS_DURABLE", e.to_string()))?;
        let ns = Self::namespace(owner, project);
        let json_str = serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string());
        let expires_at: Option<i64> = ttl_secs
            .filter(|&t| t > 0)
            .map(|t| Self::now_unix() + t as i64);
        conn.execute(
            "INSERT OR REPLACE INTO kv (namespace, key, value, expires_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![ns, key, json_str, expires_at],
        )
        .map_err(|e| StateBusError::new("STATE_BUS_DURABLE", e.to_string()))?;
        Ok(())
    }

    fn durable_exists(&self, owner: &str, project: &str, key: &str) -> Result<bool, StateBusError> {
        Ok(self.durable_get(owner, project, key)?.is_some())
    }

    fn durable_expire(
        &self,
        owner: &str,
        project: &str,
        key: &str,
        ttl_secs: Option<u64>,
    ) -> Result<bool, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        let db = self.db(owner, project)?;
        let conn = db
            .lock()
            .map_err(|e| StateBusError::new("STATE_BUS_DURABLE", e.to_string()))?;
        let ns = Self::namespace(owner, project);
        let now = Self::now_unix();
        let new_expires: Option<i64> = ttl_secs.filter(|&t| t > 0).map(|t| now + t as i64);
        let affected = conn.execute(
            "UPDATE kv SET expires_at = ?1 WHERE namespace = ?2 AND key = ?3 AND (expires_at IS NULL OR expires_at > ?4)",
            rusqlite::params![new_expires, ns, key, now],
        ).map_err(|e| StateBusError::new("STATE_BUS_DURABLE", e.to_string()))?;
        Ok(affected > 0)
    }

    fn durable_incr(
        &self,
        owner: &str,
        project: &str,
        key: &str,
        amount: i64,
    ) -> Result<i64, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        let db = self.db(owner, project)?;
        let conn = db
            .lock()
            .map_err(|e| StateBusError::new("STATE_BUS_DURABLE", e.to_string()))?;
        let ns = Self::namespace(owner, project);
        let now = Self::now_unix();
        // Read current value
        let current: i64 = conn.query_row(
            "SELECT value FROM kv WHERE namespace = ?1 AND key = ?2 AND (expires_at IS NULL OR expires_at > ?3)",
            rusqlite::params![ns, key, now],
            |row| {
                let s: String = row.get(0)?;
                Ok(serde_json::from_str::<i64>(&s).unwrap_or(0))
            },
        ).unwrap_or(0);
        let new_val = current + amount;
        let json_str = new_val.to_string();
        conn.execute(
            "INSERT OR REPLACE INTO kv (namespace, key, value, expires_at) VALUES (?1, ?2, ?3, NULL)",
            rusqlite::params![ns, key, json_str],
        ).map_err(|e| StateBusError::new("STATE_BUS_DURABLE", e.to_string()))?;
        Ok(new_val)
    }

    fn durable_del(&self, owner: &str, project: &str, key: &str) -> Result<bool, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        let db = self.db(owner, project)?;
        let conn = db
            .lock()
            .map_err(|e| StateBusError::new("STATE_BUS_DURABLE", e.to_string()))?;
        let ns = Self::namespace(owner, project);
        let affected = conn
            .execute(
                "DELETE FROM kv WHERE namespace = ?1 AND key = ?2",
                rusqlite::params![ns, key],
            )
            .map_err(|e| StateBusError::new("STATE_BUS_DURABLE", e.to_string()))?;
        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::infra::io::state::StateBus;

    use super::MemStateBus;

    #[tokio::test]
    async fn enforces_validation_and_exposes_stats() {
        let bus = MemStateBus::new();
        bus.set(
            "superadmin",
            "default",
            "session:user@example.com",
            json!({"ok": true}),
            Some(60),
        )
        .expect("set");
        let stats = bus.stats().expect("stats");
        assert_eq!(stats.entry_count, 1);
        assert!(
            bus.set("bad owner", "default", "safe", json!(1), None)
                .is_err()
        );
        assert!(
            bus.publish("superadmin", "default", "bad channel", json!(1))
                .is_err()
        );
    }

    #[tokio::test]
    async fn supports_full_kv_feature_surface() {
        let bus = MemStateBus::new();
        bus.set("superadmin", "default", "counter", json!(1), Some(1))
            .expect("set");
        assert_eq!(
            bus.get("superadmin", "default", "counter").expect("get"),
            Some(json!(1))
        );
        assert!(
            bus.exists("superadmin", "default", "counter")
                .expect("exists")
        );
        assert_eq!(
            bus.incr("superadmin", "default", "counter", 2)
                .expect("incr"),
            3
        );
        assert!(
            bus.expire("superadmin", "default", "counter", None)
                .expect("expire")
        );
        assert!(bus.del("superadmin", "default", "counter").expect("del"));
        assert!(
            !bus.exists("superadmin", "default", "counter")
                .expect("exists after del")
        );
    }

    #[tokio::test]
    async fn publish_and_subscribe_are_project_scoped() {
        let bus = MemStateBus::new();
        let mut rx = bus
            .subscribe("superadmin", "default", "events/orders")
            .expect("subscribe");
        bus.publish("superadmin", "default", "events/orders", json!({"id": 42}))
            .expect("publish");
        let message = rx.recv().await.expect("message");
        assert_eq!(message, json!({"id": 42}));

        let mut other = bus
            .subscribe("superadmin", "other", "events/orders")
            .expect("other subscribe");
        let _ = bus
            .publish("superadmin", "default", "events/orders", json!({"id": 43}))
            .expect("publish again");
        let message = rx.recv().await.expect("second message");
        assert_eq!(message, json!({"id": 43}));
        assert!(
            other.try_recv().is_err(),
            "other project must not receive the message"
        );
    }

    #[tokio::test]
    async fn durable_kv_roundtrip() {
        let dir = tempfile::tempdir().expect("temp dir");
        let bus = MemStateBus::new_with_durable(dir.path().to_path_buf());

        // Set and get
        bus.durable_set(
            "superadmin",
            "default",
            "persist-key",
            json!({"hello": "world"}),
            None,
        )
        .expect("durable set");
        let val = bus
            .durable_get("superadmin", "default", "persist-key")
            .expect("durable get");
        assert_eq!(val, Some(json!({"hello": "world"})));

        // Exists
        assert!(
            bus.durable_exists("superadmin", "default", "persist-key")
                .expect("durable exists")
        );

        // Incr
        bus.durable_set("superadmin", "default", "counter", json!(10), None)
            .expect("set counter");
        let new_val = bus
            .durable_incr("superadmin", "default", "counter", 5)
            .expect("durable incr");
        assert_eq!(new_val, 15);

        // Del
        assert!(
            bus.durable_del("superadmin", "default", "persist-key")
                .expect("durable del")
        );
        assert!(
            !bus.durable_exists("superadmin", "default", "persist-key")
                .expect("after del")
        );

        // Project scoped
        assert!(
            bus.durable_get("superadmin", "other", "counter")
                .expect("other project")
                .is_none()
        );
    }

    #[tokio::test]
    async fn durable_kv_lives_in_the_project_store_tier() {
        let dir = tempfile::tempdir().expect("temp dir");
        let bus = MemStateBus::new_with_durable(dir.path().to_path_buf());
        bus.durable_set("superadmin", "default", "k", json!("v"), None)
            .expect("durable set");
        assert!(
            dir.path()
                .join("users/superadmin/default/data/store/kv.db")
                .is_file(),
            "durable KV must live at the project's store tier"
        );
        assert!(
            !dir.path().join("kv_durable.db").exists(),
            "a fresh instance must never create the pre-tier global file"
        );
    }

    /// Builds a pre-tier global `kv_durable.db` with the schema the old
    /// implementation wrote.
    fn seed_legacy_global(path: &std::path::Path, rows: &[(&str, &str, &str)]) {
        let conn = rusqlite::Connection::open(path).expect("open legacy");
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS kv (
                 namespace TEXT NOT NULL,
                 key TEXT NOT NULL,
                 value TEXT NOT NULL,
                 expires_at INTEGER,
                 PRIMARY KEY (namespace, key)
             );",
        )
        .expect("legacy schema");
        for (ns, key, value) in rows {
            conn.execute(
                "INSERT INTO kv (namespace, key, value, expires_at) VALUES (?1, ?2, ?3, NULL)",
                rusqlite::params![ns, key, value],
            )
            .expect("seed row");
        }
    }

    fn raw_kv_value(path: &std::path::Path, ns: &str, key: &str) -> Option<String> {
        let conn = rusqlite::Connection::open(path).expect("open db");
        conn.query_row(
            "SELECT value FROM kv WHERE namespace = ?1 AND key = ?2",
            rusqlite::params![ns, key],
            |row| row.get(0),
        )
        .ok()
    }

    #[tokio::test]
    async fn legacy_global_kv_drains_per_namespace_and_keeps_orphan_rows() {
        let dir = tempfile::tempdir().expect("temp dir");
        let legacy = dir.path().join("kv_durable.db");
        seed_legacy_global(
            &legacy,
            &[
                ("superadmin/default", "greeting", "{\"hello\":\"world\"}"),
                ("superadmin/other", "count", "41"),
                ("ghost/gone", "orphan", "\"left behind\""),
            ],
        );

        let bus = MemStateBus::new_with_durable(dir.path().to_path_buf());

        // First touch of one namespace moves only that namespace's rows.
        assert_eq!(
            bus.durable_get("superadmin", "default", "greeting")
                .expect("get after migration"),
            Some(json!({"hello": "world"}))
        );
        let default_db = dir.path().join("users/superadmin/default/data/store/kv.db");
        assert_eq!(
            raw_kv_value(&default_db, "superadmin/default", "greeting").as_deref(),
            Some("{\"hello\":\"world\"}"),
            "stored TEXT must survive the move byte-exact"
        );
        assert!(legacy.exists(), "other namespaces still hold rows");
        assert_eq!(
            raw_kv_value(&legacy, "superadmin/default", "greeting"),
            None
        );
        assert_eq!(
            raw_kv_value(&legacy, "superadmin/other", "count").as_deref(),
            Some("41")
        );

        // Second namespace drains on its own first touch.
        assert_eq!(
            bus.durable_incr("superadmin", "other", "count", 1)
                .expect("incr after migration"),
            42
        );

        // Rows for a project that no longer exists stay in the global file,
        // which therefore stays too: deleting them would destroy their only
        // copy, and moving them would resurrect a removed project's directory.
        assert!(legacy.exists(), "orphan rows keep the global file alive");
        assert_eq!(
            raw_kv_value(&legacy, "ghost/gone", "orphan").as_deref(),
            Some("\"left behind\"")
        );
    }

    #[tokio::test]
    async fn legacy_global_kv_file_is_removed_once_fully_drained() {
        let dir = tempfile::tempdir().expect("temp dir");
        let legacy = dir.path().join("kv_durable.db");
        seed_legacy_global(
            &legacy,
            &[
                ("superadmin/default", "a", "1"),
                ("superadmin/other", "b", "2"),
            ],
        );

        let bus = MemStateBus::new_with_durable(dir.path().to_path_buf());
        assert_eq!(
            bus.durable_get("superadmin", "default", "a").expect("get"),
            Some(json!(1))
        );
        assert!(legacy.exists());
        assert_eq!(
            bus.durable_get("superadmin", "other", "b").expect("get"),
            Some(json!(2))
        );
        assert!(
            !legacy.exists(),
            "the global file goes only after its last row has moved"
        );

        // A later bus on the same root is a no-op migration: values stay put.
        let second = MemStateBus::new_with_durable(dir.path().to_path_buf());
        assert_eq!(
            second
                .durable_get("superadmin", "default", "a")
                .expect("get on second run"),
            Some(json!(1))
        );
        assert!(!legacy.exists());
    }

    #[tokio::test]
    async fn legacy_migration_refuses_when_both_files_hold_rows() {
        let dir = tempfile::tempdir().expect("temp dir");
        let legacy = dir.path().join("kv_durable.db");
        seed_legacy_global(&legacy, &[("superadmin/default", "k", "\"legacy\"")]);
        let dest = dir.path().join("users/superadmin/default/data/store/kv.db");
        std::fs::create_dir_all(dest.parent().expect("parent")).expect("mkdir");
        seed_legacy_global(&dest, &[("superadmin/default", "k", "\"per-project\"")]);

        let bus = MemStateBus::new_with_durable(dir.path().to_path_buf());
        let err = bus
            .durable_get("superadmin", "default", "k")
            .expect_err("both-paths-present must refuse, not guess");
        assert_eq!(err.code, "STATE_BUS_DURABLE_MIGRATE");

        // Nothing moved: both files keep exactly what they had.
        assert_eq!(
            raw_kv_value(&legacy, "superadmin/default", "k").as_deref(),
            Some("\"legacy\"")
        );
        assert_eq!(
            raw_kv_value(&dest, "superadmin/default", "k").as_deref(),
            Some("\"per-project\"")
        );
    }
}

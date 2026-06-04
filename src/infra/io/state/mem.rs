//! In-memory [`StateBus`] implementation backed by the existing [`crate::infra::mem::MemHub`],
//! with optional SQLite-backed durable KV storage.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use rusqlite::Connection;
use serde_json::Value;

use crate::infra::mem::MemHub;

use super::interface::{
    StateBus, StateBusCapabilities, StateBusError, StateBusStats, StateSubscription,
};

/// `StateBus` adapter over the existing in-process `MemHub`, with optional durable SQLite KV.
pub struct MemStateBus {
    hub: MemHub,
    /// Path to durable KV database. If set, durable operations are available.
    durable_path: Option<PathBuf>,
    /// Lazy-initialized SQLite connection for durable KV.
    durable_db: OnceLock<Arc<Mutex<Connection>>>,
}

impl Clone for MemStateBus {
    fn clone(&self) -> Self {
        Self {
            hub: self.hub.clone(),
            durable_path: self.durable_path.clone(),
            durable_db: OnceLock::new(),
        }
    }
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
            durable_path: None,
            durable_db: OnceLock::new(),
        }
    }

    /// Create a new state bus with durable SQLite storage at the given data root.
    pub fn new_with_durable(data_root: PathBuf) -> Self {
        let db_path = data_root.join("kv_durable.db");
        Self {
            hub: MemHub::new(),
            durable_path: Some(db_path),
            durable_db: OnceLock::new(),
        }
    }

    /// Wrap an existing `MemHub` with optional durable path.
    pub fn from_hub(hub: MemHub) -> Self {
        Self {
            hub,
            durable_path: None,
            durable_db: OnceLock::new(),
        }
    }

    /// Wrap an existing `MemHub` with durable storage.
    pub fn from_hub_with_durable(hub: MemHub, data_root: PathBuf) -> Self {
        let db_path = data_root.join("kv_durable.db");
        Self {
            hub,
            durable_path: Some(db_path),
            durable_db: OnceLock::new(),
        }
    }

    /// Borrow the inner `MemHub`.
    pub fn hub(&self) -> &MemHub {
        &self.hub
    }

    /// Get or initialize the durable SQLite connection.
    fn db(&self) -> Result<&Arc<Mutex<Connection>>, StateBusError> {
        // Use get_or_init with a panic-on-error wrapper since get_or_try_init is unstable.
        // The OnceLock guarantees this closure runs at most once.
        if let Some(db) = self.durable_db.get() {
            return Ok(db);
        }
        let path = self.durable_path.as_ref().ok_or_else(|| {
            StateBusError::new("STATE_BUS_NO_DURABLE", "durable storage path not configured")
        })?;
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                StateBusError::new("STATE_BUS_DURABLE_INIT", format!("failed to create dir: {e}"))
            })?;
        }
        let conn = Connection::open(path).map_err(|e| {
            StateBusError::new("STATE_BUS_DURABLE_INIT", format!("failed to open db: {e}"))
        })?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS kv (
                 namespace TEXT NOT NULL,
                 key TEXT NOT NULL,
                 value TEXT NOT NULL,
                 expires_at INTEGER,
                 PRIMARY KEY (namespace, key)
             );"
        ).map_err(|e| {
            StateBusError::new("STATE_BUS_DURABLE_INIT", format!("failed to init schema: {e}"))
        })?;
        let arc = Arc::new(Mutex::new(conn));
        // If another thread raced us, get() will return their value; ours is dropped.
        let _ = self.durable_db.set(arc);
        Ok(self.durable_db.get().expect("just set"))
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
            durable: self.durable_path.is_some(),
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

    fn durable_get(&self, owner: &str, project: &str, key: &str) -> Result<Option<Value>, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        let db = self.db()?;
        let conn = db.lock().map_err(|e| StateBusError::new("STATE_BUS_DURABLE", e.to_string()))?;
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
        let db = self.db()?;
        let conn = db.lock().map_err(|e| StateBusError::new("STATE_BUS_DURABLE", e.to_string()))?;
        let ns = Self::namespace(owner, project);
        let json_str = serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string());
        let expires_at: Option<i64> = ttl_secs
            .filter(|&t| t > 0)
            .map(|t| Self::now_unix() + t as i64);
        conn.execute(
            "INSERT OR REPLACE INTO kv (namespace, key, value, expires_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![ns, key, json_str, expires_at],
        ).map_err(|e| StateBusError::new("STATE_BUS_DURABLE", e.to_string()))?;
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
        let db = self.db()?;
        let conn = db.lock().map_err(|e| StateBusError::new("STATE_BUS_DURABLE", e.to_string()))?;
        let ns = Self::namespace(owner, project);
        let now = Self::now_unix();
        let new_expires: Option<i64> = ttl_secs
            .filter(|&t| t > 0)
            .map(|t| now + t as i64);
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
        let db = self.db()?;
        let conn = db.lock().map_err(|e| StateBusError::new("STATE_BUS_DURABLE", e.to_string()))?;
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
        let db = self.db()?;
        let conn = db.lock().map_err(|e| StateBusError::new("STATE_BUS_DURABLE", e.to_string()))?;
        let ns = Self::namespace(owner, project);
        let affected = conn.execute(
            "DELETE FROM kv WHERE namespace = ?1 AND key = ?2",
            rusqlite::params![ns, key],
        ).map_err(|e| StateBusError::new("STATE_BUS_DURABLE", e.to_string()))?;
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
        bus.durable_set("superadmin", "default", "persist-key", json!({"hello": "world"}), None)
            .expect("durable set");
        let val = bus.durable_get("superadmin", "default", "persist-key").expect("durable get");
        assert_eq!(val, Some(json!({"hello": "world"})));

        // Exists
        assert!(bus.durable_exists("superadmin", "default", "persist-key").expect("durable exists"));

        // Incr
        bus.durable_set("superadmin", "default", "counter", json!(10), None).expect("set counter");
        let new_val = bus.durable_incr("superadmin", "default", "counter", 5).expect("durable incr");
        assert_eq!(new_val, 15);

        // Del
        assert!(bus.durable_del("superadmin", "default", "persist-key").expect("durable del"));
        assert!(!bus.durable_exists("superadmin", "default", "persist-key").expect("after del"));

        // Project scoped
        assert!(bus.durable_get("superadmin", "other", "counter").expect("other project").is_none());
    }
}

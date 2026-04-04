//! Per-project in-memory key-value store with pub/sub channels.
//!
//! # Design
//!
//! - **Ephemeral**: all data is lost on server restart.
//! - **Per-project scoped**: keys/channels are namespaced as `{owner}/{project}/{name}`.
//! - **TTL**: lazy expiry checked on every read — no background cleanup thread.
//! - **Pub/sub**: `tokio::sync::broadcast` channels per named channel.
//!
//! # Quick use in pipelines
//!
//! ```text
//! | n.trigger.webhook --path /counter --method POST
//! | n.mem.incr --key visits --out-key count
//! | n.script -- "return { count: input.count };"
//! ```
//!
//! ```text
//! | n.trigger.webhook --path /notify --method POST
//! | n.mem.publish --channel alerts
//! ```
//!
//! ```text
//! | n.trigger.memsubscribe --channel alerts
//! | n.script -- "return { received: input.message };"
//! ```

pub mod subscriber;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;

const BROADCAST_CAPACITY: usize = 64;

struct MemEntry {
    value: Value,
    expires_at: Option<Instant>,
}

impl MemEntry {
    fn is_expired(&self) -> bool {
        self.expires_at.map(|e| Instant::now() > e).unwrap_or(false)
    }
}

/// Global in-memory hub — KV store + pub/sub channels, per-project namespaced.
///
/// Held as `pub mem_hub: Arc<MemHub>` in [`crate::platform::services::PlatformService`].
/// Cheap to clone (all state behind `Arc`).
#[derive(Clone)]
pub struct MemHub {
    entries: Arc<Mutex<HashMap<String, MemEntry>>>,
    channels: Arc<Mutex<HashMap<String, tokio::sync::broadcast::Sender<Value>>>>,
}

impl MemHub {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            channels: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn scoped(owner: &str, project: &str, name: &str) -> String {
        format!("{}/{}/{}", owner, project, name)
    }

    /// Get a stored value. Returns `None` if missing or expired (lazy eviction).
    pub fn get(&self, owner: &str, project: &str, key: &str) -> Option<Value> {
        let fk = Self::scoped(owner, project, key);
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let entry = entries.get(&fk)?;
        if entry.is_expired() {
            entries.remove(&fk);
            return None;
        }
        Some(entry.value.clone())
    }

    /// Set a key to `value` with an optional TTL in seconds.
    /// Passing `ttl_secs = Some(0)` is treated as no expiry.
    pub fn set(&self, owner: &str, project: &str, key: &str, value: Value, ttl_secs: Option<u64>) {
        let fk = Self::scoped(owner, project, key);
        let expires_at = ttl_secs
            .filter(|&t| t > 0)
            .map(|t| Instant::now() + Duration::from_secs(t));
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        entries.insert(fk, MemEntry { value, expires_at });
    }

    /// Delete a key. Returns `true` if it existed and was removed.
    pub fn del(&self, owner: &str, project: &str, key: &str) -> bool {
        let fk = Self::scoped(owner, project, key);
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&fk)
            .is_some()
    }

    /// Atomically increment (or decrement with negative `amount`) an integer key.
    /// Non-existent or expired keys start from 0. Non-integer values are reset to 0.
    /// Returns the new value. TTL is NOT preserved after an incr.
    pub fn incr(&self, owner: &str, project: &str, key: &str, amount: i64) -> i64 {
        let fk = Self::scoped(owner, project, key);
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let current = entries
            .get(&fk)
            .filter(|e| !e.is_expired())
            .and_then(|e| e.value.as_i64())
            .unwrap_or(0);
        let new_val = current + amount;
        entries.insert(
            fk,
            MemEntry {
                value: Value::Number(new_val.into()),
                expires_at: None,
            },
        );
        new_val
    }

    /// Check whether a key exists and is not expired. Does not modify the store.
    pub fn exists(&self, owner: &str, project: &str, key: &str) -> bool {
        self.get(owner, project, key).is_some()
    }

    /// Update the TTL of an existing key without changing its value.
    /// Pass `ttl_secs = Some(0)` or `None` to remove the expiry (persist forever).
    /// Returns `true` if the key existed (and was updated), `false` if the key was missing or expired.
    pub fn expire(&self, owner: &str, project: &str, key: &str, ttl_secs: Option<u64>) -> bool {
        let fk = Self::scoped(owner, project, key);
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = entries.get_mut(&fk) {
            if entry.is_expired() {
                entries.remove(&fk);
                return false;
            }
            entry.expires_at = ttl_secs
                .filter(|&t| t > 0)
                .map(|t| Instant::now() + Duration::from_secs(t));
            true
        } else {
            false
        }
    }

    /// Publish `message` to a named channel.
    /// Returns the number of live receivers that received the message.
    /// Silently no-ops if no subscriber has called `subscribe()` yet.
    pub fn publish(&self, owner: &str, project: &str, channel: &str, message: Value) -> usize {
        let full_ch = Self::scoped(owner, project, channel);
        let channels = self.channels.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(tx) = channels.get(&full_ch) {
            tx.send(message).unwrap_or(0)
        } else {
            0
        }
    }

    /// Subscribe to a named channel. Creates the broadcast sender if it doesn't exist.
    /// Each caller gets an independent receiver — messages are delivered to all subscribers.
    pub fn subscribe(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> tokio::sync::broadcast::Receiver<Value> {
        let full_ch = Self::scoped(owner, project, channel);
        let mut channels = self.channels.lock().unwrap_or_else(|e| e.into_inner());
        let tx = channels.entry(full_ch).or_insert_with(|| {
            let (tx, _) = tokio::sync::broadcast::channel(BROADCAST_CAPACITY);
            tx
        });
        tx.subscribe()
    }
}

impl Default for MemHub {
    fn default() -> Self {
        Self::new()
    }
}

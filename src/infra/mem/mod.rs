//! Per-project in-memory key-value store with pub/sub channels.
//!
//! # Design
//!
//! - **Ephemeral**: all data is lost on server restart.
//! - **Per-project scoped**: keys/channels are namespaced as `{owner}/{project}/{name}`.
//! - **TTL**: expiry is checked on every read, and [`MemHub::start_sweeper`] removes
//!   expired keys in the background, so an unread key does not live forever.
//! - **Pub/sub**: `tokio::sync::broadcast` channels per named channel.
//!
//! # Quick use in pipelines
//!
//! ```text
//! | trigger.webhook --path /counter --method POST
//! | kv.entry.increment --key visits --out-key count
//! | script.result.run -- "return { count: input.count };"
//! ```
//!
//! ```text
//! | trigger.webhook --path /notify --method POST
//! | kv.message.publish --channel alerts
//! ```
//!
//! ```text
//! | trigger.topic --channel alerts
//! | script.result.run -- "return { received: input.message };"
//! ```

pub mod subscriber;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use serde_json::Value;

const BROADCAST_CAPACITY: usize = 64;

/// Lightweight operational stats for the in-process mem hub.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MemHubStats {
    /// Number of currently live KV entries.
    pub entry_count: usize,
    /// Number of known pub/sub channels.
    pub channel_count: usize,
    /// Total live subscribers across all channels.
    pub subscriber_count: usize,
}

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
    entries: Arc<RwLock<HashMap<String, MemEntry>>>,
    channels: Arc<RwLock<HashMap<String, tokio::sync::broadcast::Sender<Value>>>>,
}

impl MemHub {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn scoped(owner: &str, project: &str, name: &str) -> String {
        format!("{}/{}/{}", owner, project, name)
    }

    /// Get a stored value. Returns `None` if missing or expired (lazy eviction).
    pub fn get(&self, owner: &str, project: &str, key: &str) -> Option<Value> {
        let fk = Self::scoped(owner, project, key);
        {
            let entries = self.entries.read().unwrap_or_else(|e| e.into_inner());
            let entry = entries.get(&fk)?;
            if !entry.is_expired() {
                return Some(entry.value.clone());
            }
        }
        let mut entries = self.entries.write().unwrap_or_else(|e| e.into_inner());
        if entries
            .get(&fk)
            .map(|entry| entry.is_expired())
            .unwrap_or(false)
        {
            entries.remove(&fk);
        }
        None
    }

    /// Set a key to `value` with an optional TTL in seconds.
    /// Passing `ttl_secs = Some(0)` is treated as no expiry.
    pub fn set(&self, owner: &str, project: &str, key: &str, value: Value, ttl_secs: Option<u64>) {
        let fk = Self::scoped(owner, project, key);
        let expires_at = ttl_secs
            .filter(|&t| t > 0)
            .map(|t| Instant::now() + Duration::from_secs(t));
        let mut entries = self.entries.write().unwrap_or_else(|e| e.into_inner());
        entries.insert(fk, MemEntry { value, expires_at });
    }

    /// Delete a key. Returns `true` if it existed and was removed.
    pub fn del(&self, owner: &str, project: &str, key: &str) -> bool {
        let fk = Self::scoped(owner, project, key);
        self.entries
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&fk)
            .is_some()
    }

    /// Atomically increment (or decrement with negative `amount`) an integer key.
    /// Non-existent or expired keys start from 0. Non-integer values are reset to 0.
    /// Returns the new value. A live key keeps its TTL — an incr used to clear it,
    /// turning a counter with an expiry into one that never expired.
    pub fn incr(&self, owner: &str, project: &str, key: &str, amount: i64) -> i64 {
        let fk = Self::scoped(owner, project, key);
        let mut entries = self.entries.write().unwrap_or_else(|e| e.into_inner());
        let live = entries.get(&fk).filter(|e| !e.is_expired());
        let current = live.and_then(|e| e.value.as_i64()).unwrap_or(0);
        let expires_at = live.and_then(|e| e.expires_at);
        let new_val = current + amount;
        entries.insert(
            fk,
            MemEntry {
                value: Value::Number(new_val.into()),
                expires_at,
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
        let mut entries = self.entries.write().unwrap_or_else(|e| e.into_inner());
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
        let tx = {
            let channels = self.channels.read().unwrap_or_else(|e| e.into_inner());
            channels.get(&full_ch).cloned()
        };
        if let Some(tx) = tx {
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
        if let Some(tx) = self
            .channels
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&full_ch)
            .cloned()
        {
            return tx.subscribe();
        }
        let mut channels = self.channels.write().unwrap_or_else(|e| e.into_inner());
        let tx = channels.entry(full_ch).or_insert_with(|| {
            let (tx, _) = tokio::sync::broadcast::channel(BROADCAST_CAPACITY);
            tx
        });
        tx.subscribe()
    }

    /// Return lightweight runtime stats and opportunistically purge expired entries.
    pub fn stats(&self) -> MemHubStats {
        self.purge_expired();
        let entry_count = self.entries.read().unwrap_or_else(|e| e.into_inner()).len();
        let channels = self.channels.read().unwrap_or_else(|e| e.into_inner());
        let channel_count = channels.len();
        let subscriber_count = channels
            .values()
            .map(tokio::sync::broadcast::Sender::receiver_count)
            .sum();
        MemHubStats {
            entry_count,
            channel_count,
            subscriber_count,
        }
    }

    /// Remove expired entries every `every`, in the background, for as long as a
    /// Tokio runtime is running. Without a runtime (plain unit tests) it does nothing.
    pub fn start_sweeper(&self, every: Duration) {
        let Ok(rt) = tokio::runtime::Handle::try_current() else { return };
        let hub = self.clone();
        rt.spawn(async move {
            let mut tick = tokio::time::interval(every);
            loop {
                tick.tick().await;
                hub.purge_expired();
            }
        });
    }

    /// Remove expired entries from the in-memory store.
    pub fn purge_expired(&self) -> usize {
        let mut entries = self.entries.write().unwrap_or_else(|e| e.into_inner());
        let before = entries.len();
        entries.retain(|_, entry| !entry.is_expired());
        before.saturating_sub(entries.len())
    }
}

impl Default for MemHub {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incr_keeps_a_live_keys_ttl() {
        let hub = MemHub::new();
        hub.set("o", "p", "hits", Value::from(1), Some(60));
        assert_eq!(hub.incr("o", "p", "hits", 2), 3);
        let entries = hub.entries.read().unwrap();
        assert!(entries.get("o/p/hits").unwrap().expires_at.is_some(), "incr must not make the key permanent");
    }

    #[test]
    fn incr_on_a_new_key_has_no_ttl() {
        let hub = MemHub::new();
        assert_eq!(hub.incr("o", "p", "n", 1), 1);
        assert!(hub.entries.read().unwrap().get("o/p/n").unwrap().expires_at.is_none());
    }

    #[tokio::test]
    async fn the_sweeper_removes_unread_expired_keys() {
        let hub = MemHub::new();
        hub.entries.write().unwrap().insert(
            "o/p/gone".into(),
            MemEntry { value: Value::from(1), expires_at: Some(Instant::now() - Duration::from_secs(1)) },
        );
        hub.start_sweeper(Duration::from_millis(20));
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(hub.entries.read().unwrap().is_empty(), "an expired key nobody reads is still removed");
    }
}

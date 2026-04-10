//! In-process cache implementation.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::Value;

use super::interface::CacheStore;

#[derive(Debug, Clone)]
struct CacheEntry {
    value: Value,
    expires_at: Option<Instant>,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        self.expires_at.map(|v| Instant::now() > v).unwrap_or(false)
    }
}

/// Simple in-process cache suitable for standalone or per-worker local caches.
#[derive(Debug, Default)]
pub struct MemoryCacheStore {
    entries: Mutex<HashMap<String, CacheEntry>>,
}

impl MemoryCacheStore {
    fn scoped(namespace: &str, key: &str) -> String {
        format!("{namespace}::{key}")
    }
}

impl CacheStore for MemoryCacheStore {
    fn id(&self) -> &'static str {
        "cache.memory"
    }

    fn get_json(&self, namespace: &str, key: &str) -> Option<Value> {
        let scoped = Self::scoped(namespace, key);
        let mut guard = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let entry = guard.get(&scoped)?;
        if entry.is_expired() {
            guard.remove(&scoped);
            return None;
        }
        Some(entry.value.clone())
    }

    fn put_json(&self, namespace: &str, key: &str, value: Value, ttl_secs: Option<u64>) {
        let scoped = Self::scoped(namespace, key);
        let expires_at = ttl_secs
            .filter(|ttl| *ttl > 0)
            .map(|ttl| Instant::now() + Duration::from_secs(ttl));
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(scoped, CacheEntry { value, expires_at });
    }

    fn delete(&self, namespace: &str, key: &str) -> bool {
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&Self::scoped(namespace, key))
            .is_some()
    }

    fn clear_namespace(&self, namespace: &str) {
        let prefix = format!("{namespace}::");
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|key, _| !key.starts_with(&prefix));
    }
}

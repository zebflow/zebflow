//! Trait definitions for generic cache storage.

use std::sync::Arc;

use serde_json::Value;

/// Generic JSON-oriented cache contract.
pub trait CacheStore: Send + Sync {
    /// Stable implementation identifier.
    fn id(&self) -> &'static str;

    /// Read one cache entry from a namespace.
    fn get_json(&self, namespace: &str, key: &str) -> Option<Value>;

    /// Write one cache entry into a namespace.
    fn put_json(&self, namespace: &str, key: &str, value: Value, ttl_secs: Option<u64>);

    /// Delete one cache entry.
    fn delete(&self, namespace: &str, key: &str) -> bool;

    /// Clear a whole namespace.
    fn clear_namespace(&self, namespace: &str);
}

/// Shared cache trait object.
pub type DynCacheStore = Arc<dyn CacheStore>;

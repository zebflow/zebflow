//! Trait definitions for Zebflow's shared state bus.
//!
//! The state bus is the seam between Zebflow runtime features and the concrete mechanism used to
//! coordinate project state. It intentionally covers the full current `n.kv.*` feature
//! set so the existing in-memory behavior can move behind one contract before Redis or another
//! shared backend is introduced.
//!
//! Design goals:
//!
//! - preserve the current low-latency local mem behavior
//! - make namespace and key validation explicit early
//! - keep TTL, atomic increment, and pub/sub semantics visible in the contract
//! - expose capabilities and limits so future UI/cluster code can reason about backends

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Maximum owner segment length accepted by the default state-bus validators.
pub const MAX_STATE_OWNER_LEN: usize = 96;
/// Maximum project segment length accepted by the default state-bus validators.
pub const MAX_STATE_PROJECT_LEN: usize = 96;
/// Maximum key length accepted by the default state-bus validators.
pub const MAX_STATE_KEY_LEN: usize = 256;
/// Maximum channel length accepted by the default state-bus validators.
pub const MAX_STATE_CHANNEL_LEN: usize = 256;

/// Shared subscription type used by pub/sub backends.
pub type StateSubscription = tokio::sync::broadcast::Receiver<Value>;

/// Typed state-bus error.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateBusError {
    /// Stable error code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

impl StateBusError {
    /// Build a new state-bus error.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    /// Invalid namespace segment helper.
    pub fn invalid_namespace(field: &str, message: impl Into<String>) -> Self {
        Self::new(
            "STATE_BUS_INVALID_NAMESPACE",
            format!("{field}: {}", message.into()),
        )
    }

    /// Invalid key helper.
    pub fn invalid_key(message: impl Into<String>) -> Self {
        Self::new("STATE_BUS_INVALID_KEY", message)
    }

    /// Invalid channel helper.
    pub fn invalid_channel(message: impl Into<String>) -> Self {
        Self::new("STATE_BUS_INVALID_CHANNEL", message)
    }

    /// Not implemented helper for placeholder backends.
    pub fn not_implemented(backend_id: &str) -> Self {
        Self::new(
            "STATE_BUS_NOT_IMPLEMENTED",
            format!("state bus backend '{backend_id}' is not implemented yet"),
        )
    }
}

impl std::fmt::Display for StateBusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for StateBusError {}

/// Stable capability summary for one state-bus implementation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateBusCapabilities {
    /// Whether TTL/expiry is supported.
    pub ttl: bool,
    /// Whether atomic integer increment is supported.
    pub atomic_incr: bool,
    /// Whether pub/sub is supported.
    pub pubsub: bool,
    /// Whether the backend is durable across process restarts.
    pub durable: bool,
    /// Whether the backend supports shared coordination across multiple runtimes.
    pub shared_coordination: bool,
}

impl Default for StateBusCapabilities {
    fn default() -> Self {
        Self {
            ttl: true,
            atomic_incr: true,
            pubsub: true,
            durable: false,
            shared_coordination: false,
        }
    }
}

/// Implementation limits exposed by the state bus.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateBusLimits {
    /// Maximum owner segment length.
    pub max_owner_len: usize,
    /// Maximum project segment length.
    pub max_project_len: usize,
    /// Maximum key length.
    pub max_key_len: usize,
    /// Maximum channel length.
    pub max_channel_len: usize,
}

impl Default for StateBusLimits {
    fn default() -> Self {
        Self {
            max_owner_len: MAX_STATE_OWNER_LEN,
            max_project_len: MAX_STATE_PROJECT_LEN,
            max_key_len: MAX_STATE_KEY_LEN,
            max_channel_len: MAX_STATE_CHANNEL_LEN,
        }
    }
}

/// Lightweight implementation stats.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct StateBusStats {
    /// Stable implementation id.
    pub implementation: String,
    /// Number of live entries currently held.
    pub entry_count: usize,
    /// Number of known pub/sub channels.
    pub channel_count: usize,
    /// Number of live subscribers across all channels.
    pub subscriber_count: usize,
}

/// Shared project-scoped KV + pub/sub contract.
///
/// This is intentionally close to the current `MemHub` behavior so the first migration can
/// be mechanical. Later implementations may add stronger lease or delivery guarantees.
pub trait StateBus: Send + Sync {
    /// Stable implementation identifier.
    fn id(&self) -> &'static str;

    /// Backend capability summary.
    fn capabilities(&self) -> StateBusCapabilities {
        StateBusCapabilities::default()
    }

    /// Backend validation and size limits.
    fn limits(&self) -> StateBusLimits {
        StateBusLimits::default()
    }

    /// Return lightweight implementation stats.
    fn stats(&self) -> Result<StateBusStats, StateBusError>;

    /// Validate one project namespace.
    fn validate_namespace(&self, owner: &str, project: &str) -> Result<(), StateBusError> {
        validate_namespace(owner, project, &self.limits())
    }

    /// Validate one key.
    fn validate_key(&self, key: &str) -> Result<(), StateBusError> {
        validate_state_key(key, self.limits().max_key_len)
    }

    /// Validate one channel.
    fn validate_channel(&self, channel: &str) -> Result<(), StateBusError> {
        validate_state_channel(channel, self.limits().max_channel_len)
    }

    /// Read one namespaced key.
    fn get(&self, owner: &str, project: &str, key: &str) -> Result<Option<Value>, StateBusError>;

    /// Set one namespaced key with an optional TTL in seconds.
    fn set(
        &self,
        owner: &str,
        project: &str,
        key: &str,
        value: Value,
        ttl_secs: Option<u64>,
    ) -> Result<(), StateBusError>;

    /// Check whether one namespaced key exists and is still live.
    fn exists(&self, owner: &str, project: &str, key: &str) -> Result<bool, StateBusError>;

    /// Update the TTL of an existing key.
    fn expire(
        &self,
        owner: &str,
        project: &str,
        key: &str,
        ttl_secs: Option<u64>,
    ) -> Result<bool, StateBusError>;

    /// Atomically increment one integer key.
    fn incr(
        &self,
        owner: &str,
        project: &str,
        key: &str,
        amount: i64,
    ) -> Result<i64, StateBusError>;

    /// Delete one namespaced key.
    fn del(&self, owner: &str, project: &str, key: &str) -> Result<bool, StateBusError>;

    /// Publish a message onto a namespaced channel.
    fn publish(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        message: Value,
    ) -> Result<usize, StateBusError>;

    /// Subscribe to a namespaced channel.
    fn subscribe(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> Result<StateSubscription, StateBusError>;

    // ── Durable KV operations (SQLite-backed, survive restart) ─────────────

    /// Read one namespaced key from durable storage.
    fn durable_get(
        &self,
        owner: &str,
        project: &str,
        key: &str,
    ) -> Result<Option<Value>, StateBusError> {
        let _ = (owner, project, key);
        Err(StateBusError::new(
            "STATE_BUS_NO_DURABLE",
            "durable storage not available",
        ))
    }

    /// Set one namespaced key in durable storage with an optional TTL in seconds.
    fn durable_set(
        &self,
        owner: &str,
        project: &str,
        key: &str,
        value: Value,
        ttl_secs: Option<u64>,
    ) -> Result<(), StateBusError> {
        let _ = (owner, project, key, value, ttl_secs);
        Err(StateBusError::new(
            "STATE_BUS_NO_DURABLE",
            "durable storage not available",
        ))
    }

    /// Check whether one namespaced key exists in durable storage.
    fn durable_exists(&self, owner: &str, project: &str, key: &str) -> Result<bool, StateBusError> {
        let _ = (owner, project, key);
        Err(StateBusError::new(
            "STATE_BUS_NO_DURABLE",
            "durable storage not available",
        ))
    }

    /// Update the TTL of an existing key in durable storage.
    fn durable_expire(
        &self,
        owner: &str,
        project: &str,
        key: &str,
        ttl_secs: Option<u64>,
    ) -> Result<bool, StateBusError> {
        let _ = (owner, project, key, ttl_secs);
        Err(StateBusError::new(
            "STATE_BUS_NO_DURABLE",
            "durable storage not available",
        ))
    }

    /// Atomically increment one integer key in durable storage.
    fn durable_incr(
        &self,
        owner: &str,
        project: &str,
        key: &str,
        amount: i64,
    ) -> Result<i64, StateBusError> {
        let _ = (owner, project, key, amount);
        Err(StateBusError::new(
            "STATE_BUS_NO_DURABLE",
            "durable storage not available",
        ))
    }

    /// Delete one namespaced key from durable storage.
    fn durable_del(&self, owner: &str, project: &str, key: &str) -> Result<bool, StateBusError> {
        let _ = (owner, project, key);
        Err(StateBusError::new(
            "STATE_BUS_NO_DURABLE",
            "durable storage not available",
        ))
    }
}

/// Shared trait object used by schedulers, workers, and runtime services.
pub type DynStateBus = Arc<dyn StateBus>;

fn validate_namespace(
    owner: &str,
    project: &str,
    limits: &StateBusLimits,
) -> Result<(), StateBusError> {
    validate_namespace_segment("owner", owner, limits.max_owner_len)?;
    validate_namespace_segment("project", project, limits.max_project_len)?;
    Ok(())
}

fn validate_namespace_segment(
    field: &str,
    value: &str,
    max_len: usize,
) -> Result<(), StateBusError> {
    if value.is_empty() {
        return Err(StateBusError::invalid_namespace(field, "must not be empty"));
    }
    if value.len() > max_len {
        return Err(StateBusError::invalid_namespace(
            field,
            format!("must be at most {max_len} bytes"),
        ));
    }
    if value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
    {
        Ok(())
    } else {
        Err(StateBusError::invalid_namespace(
            field,
            "contains unsupported characters; allowed: ASCII letters, digits, '_', '-', '.'",
        ))
    }
}

fn validate_state_key(key: &str, max_len: usize) -> Result<(), StateBusError> {
    validate_state_name(key, max_len, StateBusError::invalid_key)
}

fn validate_state_channel(channel: &str, max_len: usize) -> Result<(), StateBusError> {
    validate_state_name(channel, max_len, StateBusError::invalid_channel)
}

fn validate_state_name<F>(value: &str, max_len: usize, err: F) -> Result<(), StateBusError>
where
    F: Fn(String) -> StateBusError,
{
    if value.is_empty() {
        return Err(err("must not be empty".to_string()));
    }
    if value.len() > max_len {
        return Err(err(format!("must be at most {max_len} bytes")));
    }
    if value.bytes().all(is_valid_state_name_byte) {
        Ok(())
    } else {
        Err(err(
            "contains unsupported characters; allowed: ASCII letters, digits, '-', '_', '.', ':', '/', '@', '+', '='".to_string(),
        ))
    }
}

fn is_valid_state_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/' | b'@' | b'+' | b'=')
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_STATE_CHANNEL_LEN, MAX_STATE_KEY_LEN, MAX_STATE_OWNER_LEN, MAX_STATE_PROJECT_LEN,
        StateBusLimits, validate_namespace, validate_state_channel, validate_state_key,
    };

    #[test]
    fn validates_namespace_segments() {
        let limits = StateBusLimits::default();
        validate_namespace("superadmin", "default", &limits).expect("valid namespace");
        assert!(validate_namespace("bad/owner", "default", &limits).is_err());
        assert!(validate_namespace("bad owner", "default", &limits).is_err());
        assert!(validate_namespace("", "default", &limits).is_err());
    }

    #[test]
    fn validates_keys_and_channels() {
        validate_state_key("session:user@example.com", MAX_STATE_KEY_LEN).expect("valid key");
        validate_state_channel("events/order.created", MAX_STATE_CHANNEL_LEN)
            .expect("valid channel");
        assert!(validate_state_key("bad key", MAX_STATE_KEY_LEN).is_err());
        assert!(validate_state_channel("bad\tchannel", MAX_STATE_CHANNEL_LEN).is_err());
    }

    #[test]
    fn enforces_length_limits() {
        let owner = "o".repeat(MAX_STATE_OWNER_LEN + 1);
        let project = "p".repeat(MAX_STATE_PROJECT_LEN + 1);
        let key = "k".repeat(MAX_STATE_KEY_LEN + 1);
        let channel = "c".repeat(MAX_STATE_CHANNEL_LEN + 1);
        let limits = StateBusLimits::default();
        assert!(validate_namespace(&owner, "default", &limits).is_err());
        assert!(validate_namespace("superadmin", &project, &limits).is_err());
        assert!(validate_state_key(&key, MAX_STATE_KEY_LEN).is_err());
        assert!(validate_state_channel(&channel, MAX_STATE_CHANNEL_LEN).is_err());
    }
}

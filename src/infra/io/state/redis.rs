//! Placeholder for a future Redis-backed [`StateBus`] implementation.
//!
//! This file exists early so cluster-aware code has a stable module path even before Redis
//! support is implemented. The first clustered release should use `MemStateBus` with pinned
//! single-runner placement.

use serde_json::Value;

use super::interface::{
    StateBus, StateBusCapabilities, StateBusError, StateBusStats, StateSubscription,
};

/// Future Redis-backed state bus.
#[derive(Debug, Clone, Default)]
pub struct RedisStateBus;

impl StateBus for RedisStateBus {
    fn id(&self) -> &'static str {
        "state.redis"
    }

    fn capabilities(&self) -> StateBusCapabilities {
        StateBusCapabilities {
            ttl: true,
            atomic_incr: true,
            pubsub: true,
            durable: false,
            shared_coordination: true,
        }
    }

    fn stats(&self) -> Result<StateBusStats, StateBusError> {
        Err(StateBusError::not_implemented(self.id()))
    }

    fn get(&self, owner: &str, project: &str, key: &str) -> Result<Option<Value>, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        Err(StateBusError::not_implemented(self.id()))
    }

    fn set(
        &self,
        owner: &str,
        project: &str,
        key: &str,
        _value: Value,
        _ttl_secs: Option<u64>,
    ) -> Result<(), StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        Err(StateBusError::not_implemented(self.id()))
    }

    fn exists(&self, owner: &str, project: &str, key: &str) -> Result<bool, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        Err(StateBusError::not_implemented(self.id()))
    }

    fn expire(
        &self,
        owner: &str,
        project: &str,
        key: &str,
        _ttl_secs: Option<u64>,
    ) -> Result<bool, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        Err(StateBusError::not_implemented(self.id()))
    }

    fn incr(
        &self,
        owner: &str,
        project: &str,
        key: &str,
        _amount: i64,
    ) -> Result<i64, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        Err(StateBusError::not_implemented(self.id()))
    }

    fn del(&self, owner: &str, project: &str, key: &str) -> Result<bool, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_key(key)?;
        Err(StateBusError::not_implemented(self.id()))
    }

    fn publish(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        _message: Value,
    ) -> Result<usize, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_channel(channel)?;
        Err(StateBusError::not_implemented(self.id()))
    }

    fn subscribe(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> Result<StateSubscription, StateBusError> {
        self.validate_namespace(owner, project)?;
        self.validate_channel(channel)?;
        Err(StateBusError::not_implemented(self.id()))
    }
}

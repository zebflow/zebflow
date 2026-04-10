//! State bus contracts: KV, pubsub, and lease-oriented coordination.
//!
//! This is the first high-value abstraction for clustered Zebflow.
//!
//! The current `infra/mem` implementation already provides project-scoped KV and pubsub.
//! This module wraps that behavior behind an explicit interface so the runtime can later swap
//! between:
//!
//! - `MemStateBus` for standalone and pinned single-runner deployments
//! - `RedisStateBus` or similar for shared multi-runner coordination
//!
//! Long-term uses:
//!
//! - `n.mem.*` nodes
//! - mem-subscribe triggers
//! - cache eviction fanout
//! - coordination leases / leader election
//!
//! Current implementation notes:
//!
//! - namespaces, keys, and channels are validated explicitly
//! - capabilities and limits are surfaced to callers
//! - the first backend remains ultra-lightweight and in-process
//! - Redis or another shared backend can later implement the same contract

pub mod interface;
pub mod lease;
pub mod mem;
pub mod redis;

pub use interface::{
    DynStateBus, StateBus, StateBusCapabilities, StateBusError, StateBusLimits, StateBusStats,
    StateSubscription,
};
pub use lease::{LeaseScope, scoped_lease_key};
pub use mem::MemStateBus;
pub use redis::RedisStateBus;

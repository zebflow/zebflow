//! Cache contracts and implementations.
//!
//! This layer is for generic cache semantics that can be shared across render, execution,
//! or orchestration features. The current codebase still contains specialized in-memory caches
//! in feature modules such as `rwe` and `pipeline`; this module is the place to converge them later.

pub mod interface;
pub mod memory;
pub mod redis;

pub use interface::{CacheStore, DynCacheStore};
pub use memory::MemoryCacheStore;
pub use redis::RedisCacheStore;

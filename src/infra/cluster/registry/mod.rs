//! Worker registry and heartbeat models.
//!
//! The registry layer is the authoritative description of which execution-plane nodes are known,
//! what they can do, and whether they are currently healthy enough to receive work.
//!
//! Later, this module is expected to back:
//!
//! - worker list APIs
//! - Infrastructure UI pages
//! - placement validation
//! - scheduling and drain logic

pub mod heartbeat;
pub mod worker_registry;

pub use heartbeat::WorkerHeartbeat;
pub use worker_registry::{WorkerRegistryRecord, WorkerRegistrySnapshot};

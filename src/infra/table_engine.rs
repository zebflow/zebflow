//! The one DataFusion runtime every SQL query over a file runs on: the
//! `table.*` nodes and the map server's GeoParquet layers.
//!
//! `SessionContext::new()` has no memory limit, never spills, and runs one
//! partition per core of the machine, not of the container. One heavy query
//! over a large GeoParquet file took a whole office down that way: every
//! project on it stopped answering until the container restarted.
//!
//! Here every session shares one memory pool, a fixed share of the memory this
//! process runs under (`instance_settings::table_query_memory`). An operator
//! that would pass it spills to disk, or the query fails with
//! `ResourcesExhausted`, which the nodes answer as their own error. The
//! partitions of one query are capped, and at most
//! [`MAX_CONCURRENT_QUERIES`] table queries run at once.
//!
//! The pool counts only the operators that hold memory in proportion to their
//! input: grouping, sorting, joins. Reading a file is not counted (DataFusion's
//! `memory_pool` documentation says so), which is why the partitions and the
//! number of queries at once are bounded as well.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use datafusion::error::DataFusionError;
use datafusion::execution::memory_pool::{FairSpillPool, TrackConsumersPool};
use datafusion::execution::runtime_env::{RuntimeEnv, RuntimeEnvBuilder};
use datafusion::prelude::{SessionConfig, SessionContext};
use tokio::sync::{Semaphore, SemaphorePermit};

/// The most partitions one query is split into, whatever the machine's cores.
pub const MAX_PARTITIONS: usize = 4;
/// The most table queries running at once in this process.
pub const MAX_CONCURRENT_QUERIES: usize = 2;
/// How long a table query waits for its turn before it is answered as busy.
pub const QUEUE_WAIT: Duration = Duration::from_secs(10);
/// How many of the largest memory users an exhausted-memory error names.
const NAMED_CONSUMERS: usize = 5;

/// A runtime whose sessions share one pool of `pool_bytes`, spilling to the
/// system's temporary directory.
pub fn runtime_with(pool_bytes: usize) -> Arc<RuntimeEnv> {
    let pool = TrackConsumersPool::new(
        FairSpillPool::new(pool_bytes),
        std::num::NonZeroUsize::new(NAMED_CONSUMERS).expect("a positive count"),
    );
    RuntimeEnvBuilder::new()
        .with_memory_pool(Arc::new(pool))
        .build_arc()
        .expect("a runtime with a memory pool and the default disk manager")
}

/// This process's runtime, made on first use.
pub fn runtime() -> Arc<RuntimeEnv> {
    static RUNTIME: OnceLock<Arc<RuntimeEnv>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            let bytes = crate::platform::services::instance_settings::table_query_memory();
            runtime_with(usize::try_from(bytes).unwrap_or(usize::MAX))
        })
        .clone()
}

/// `config` with its partitions capped at [`MAX_PARTITIONS`].
pub fn bounded(config: SessionConfig) -> SessionConfig {
    let partitions = config.target_partitions().clamp(1, MAX_PARTITIONS);
    config.with_target_partitions(partitions)
}

/// A session on this process's runtime.
pub fn session(config: SessionConfig) -> SessionContext {
    SessionContext::new_with_config_rt(bounded(config), runtime())
}

/// A turn to run one table query, held until the query's result is collected.
/// `None` when every turn stayed taken for [`QUEUE_WAIT`].
pub async fn admit() -> Option<SemaphorePermit<'static>> {
    static TURNS: Semaphore = Semaphore::const_new(MAX_CONCURRENT_QUERIES);
    admit_on(&TURNS, QUEUE_WAIT).await
}

async fn admit_on(turns: &Semaphore, wait: Duration) -> Option<SemaphorePermit<'_>> {
    tokio::time::timeout(wait, turns.acquire()).await.ok()?.ok()
}

/// Whether `err` is a query that needed more memory than the pool holds.
pub fn is_out_of_memory(err: &DataFusionError) -> bool {
    matches!(err.find_root(), DataFusionError::ResourcesExhausted(_))
}

/// The pool's size in whole MiB, for the error a node answers.
pub fn pool_mib() -> u64 {
    crate::platform::services::instance_settings::table_query_memory() >> 20
}

#[cfg(test)]
#[path = "table_engine_tests.rs"]
mod tests;

//! Sandboxed JavaScript engine backed by Deno.
//!
//! This module is designed for Zebflow script execution with a safe-by-default
//! configuration model:
//!
//! 1. Platform patch
//! 2. Project patch
//! 3. Optional run patch
//!
//! The resulting config is normalized and enforced both in Rust (process limits)
//! and in the Deno runner (operation/time budget, fetch policy, blocked globals).

mod analyze;
mod config;
mod engine;
pub(crate) mod heap;
mod pool;
mod runner;

pub use config::{
    DenoSandboxAllowList, DenoSandboxAllowListPatch, DenoSandboxConfig, DenoSandboxConfigPatch,
    DenoSandboxDangerZone, DenoSandboxDangerZonePatch,
};
pub use analyze::{ScriptDiagnostic, ScriptPolicy, compile_body, static_property_name};
pub use engine::{CompiledDenoSandboxScript, DenoSandboxEngine};
pub use heap::{runtime_heap_limit, set_runtime_heap_limit};
pub use pool::sandbox_runtime_count;

/// Starts V8 with as many helper threads as the device has cores, one fewer
/// as V8 itself would choose. Left to itself V8 counts the host's processors,
/// which a container's CPU limit does not change: a 4-core device ran 10 V8
/// threads, each soon with an allocator arena of its own. Called once at
/// process start, before any runtime is made; a runtime made first starts V8
/// its own way and this is then ignored.
pub fn init_v8_platform() {
    let cores = std::thread::available_parallelism().map(std::num::NonZeroUsize::get).unwrap_or(1);
    let workers = cores.saturating_sub(1).max(1) as u32;
    deno_core::JsRuntime::init_platform(Some(deno_core::v8::new_default_platform(workers, false).make_shared()));
}

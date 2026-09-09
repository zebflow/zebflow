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
mod pool;
mod runner;

pub use config::{
    DenoSandboxAllowList, DenoSandboxAllowListPatch, DenoSandboxConfig, DenoSandboxConfigPatch,
    DenoSandboxDangerZone, DenoSandboxDangerZonePatch,
};
pub use analyze::{ScriptDiagnostic, ScriptPolicy, compile_body};
pub use engine::{CompiledDenoSandboxScript, DenoSandboxEngine};

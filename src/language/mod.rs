//! Language module for parsing, compiling, and running user scripts.
//!
//! This layer is engine-pluggable and currently includes:
//!
//! - `language.deno_sandbox` for sandboxed JavaScript
//! - `language.noop` as a reference implementation

pub mod engines;
pub mod interface;
pub mod model;
pub mod registry;

pub use engines::{
    CompiledDenoSandboxScript, DenoSandboxAllowList, DenoSandboxAllowListPatch, DenoSandboxConfig,
    DenoSandboxConfigPatch, DenoSandboxDangerZone, DenoSandboxDangerZonePatch, DenoSandboxEngine,
    NoopLanguageEngine,
};
pub use interface::LanguageEngine;
pub use model::{
    COMPILE_TARGET_BACKEND, COMPILE_TARGET_FRONTEND, COMPILE_TARGET_PIPELINE, CompileOptions,
    CompiledProgram, ExecutionContext, ExecutionOutput, LanguageError, ModuleSource, ProgramIr,
    SourceKind, TSX_EXTENSION, ZF_JSON_EXTENSION,
};
pub use registry::LanguageEngineRegistry;

//! Deno sandbox language engine implementation.
//!
//! This file owns:
//!
//! - compile-time policy merge and instrumentation
//! - runtime artifact serialization/deserialization
//! - `LanguageEngine` integration

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::language::interface::LanguageEngine;
use crate::language::model::{
    CompileOptions, CompiledProgram, ExecutionContext, ExecutionOutput, LanguageError,
    ModuleSource, ProgramIr,
};

use super::config::{DenoSandboxConfig, DenoSandboxConfigPatch, apply_patch, normalize_limits};
use super::instrument::{forbid_patterns, inject_loop_guards};
use super::runner::run_compiled_script;

const RUNNER_REL_PATH: &str = "runtime/secure_js_runner.js";
const TOOL_INIT: &str = include_str!("../../../language/runtime/tool_init.js");

/// Serialized artifact produced by `DenoSandboxEngine::compile_script`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledDenoSandboxScript {
    /// Stable hash id of the transformed module + critical limits.
    pub script_id: String,
    /// Original source submitted by caller.
    pub original_source: String,
    /// Wrapped module source loaded by the JS runner.
    pub module_source: String,
    /// Resolved, normalized runtime policy.
    pub resolved_config: DenoSandboxConfig,
}

/// Sandboxed Deno language engine with layered policy patches.
///
/// Layer merge order:
///
/// 1. strict defaults
/// 2. platform patch
/// 3. project patch
/// 4. optional run patch
#[derive(Clone, Debug, Default)]
pub struct DenoSandboxEngine {
    /// Global platform patch.
    pub platform_patch: DenoSandboxConfigPatch,
    /// Per-project patch.
    pub project_patch: DenoSandboxConfigPatch,
    runner_path: Option<PathBuf>,
}

impl DenoSandboxEngine {
    /// Creates a new engine with explicit platform and project patches.
    pub fn new(
        platform_patch: DenoSandboxConfigPatch,
        project_patch: DenoSandboxConfigPatch,
    ) -> Self {
        Self {
            platform_patch,
            project_patch,
            runner_path: None,
        }
    }

    /// Overrides the runner path (mainly for integration tests).
    pub fn with_runner_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.runner_path = Some(path.into());
        self
    }

    /// Compiles plain JS source into a sandbox-ready artifact.
    pub fn compile_script(
        &self,
        source: &str,
        run_patch: Option<&DenoSandboxConfigPatch>,
    ) -> Result<CompiledDenoSandboxScript, LanguageError> {
        let mut cfg = DenoSandboxConfig::default();
        apply_patch(&mut cfg, &self.platform_patch);
        apply_patch(&mut cfg, &self.project_patch);
        if let Some(patch) = run_patch {
            apply_patch(&mut cfg, patch);
        }
        normalize_limits(&mut cfg);

        if source.len() > cfg.max_source_bytes {
            return Err(LanguageError::new(
                "LANG_DENO_SOURCE_TOO_LARGE",
                format!(
                    "source too large ({} > {} bytes)",
                    source.len(),
                    cfg.max_source_bytes
                ),
            ));
        }

        forbid_patterns(source, &cfg).map_err(|e| {
            LanguageError::new("LANG_DENO_POLICY", format!("policy violation: {e}"))
        })?;

        let body = if cfg.danger_zone.disable_loop_guards {
            source.to_string()
        } else {
            inject_loop_guards(source)
        };

        let module_source =
            format!("{TOOL_INIT}\nexport default async function(input, n, ctx) {{\n{body}\n}}\n");

        let mut hasher = DefaultHasher::new();
        module_source.hash(&mut hasher);
        cfg.timeout_ms.hash(&mut hasher);
        cfg.max_ops.hash(&mut hasher);
        let script_id = format!("tj_{:x}", hasher.finish());

        Ok(CompiledDenoSandboxScript {
            script_id,
            original_source: source.to_string(),
            module_source,
            resolved_config: cfg,
        })
    }

    /// Runs an already compiled script with JSON input.
    pub fn run_compiled(
        &self,
        compiled: &CompiledDenoSandboxScript,
        input: &Value,
    ) -> Result<Value, LanguageError> {
        run_compiled_script(&self.runner_path(), compiled, input).map_err(|e| {
            LanguageError::new(
                "LANG_DENO_RUN",
                format!("sandbox execution failed for '{}': {e}", compiled.script_id),
            )
        })
    }

    /// Convenience path for compile + run in one call.
    pub fn run_script(
        &self,
        source: &str,
        input: &Value,
        run_patch: Option<&DenoSandboxConfigPatch>,
    ) -> Result<Value, LanguageError> {
        let compiled = self.compile_script(source, run_patch)?;
        self.run_compiled(&compiled, input)
    }

    fn runner_path(&self) -> PathBuf {
        self.runner_path.clone().unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(RUNNER_REL_PATH)
                .to_path_buf()
        })
    }
}

impl LanguageEngine for DenoSandboxEngine {
    fn id(&self) -> &'static str {
        "language.deno_sandbox"
    }

    fn parse(&self, module: &ModuleSource) -> Result<ProgramIr, LanguageError> {
        Ok(ProgramIr {
            source_id: module.id.clone(),
            kind: module.kind,
            body: json!({
                "source": module.code,
                "source_kind": format!("{:?}", module.kind),
            }),
        })
    }

    fn compile(
        &self,
        ir: &ProgramIr,
        options: &CompileOptions,
    ) -> Result<CompiledProgram, LanguageError> {
        let source = ir
            .body
            .get("source")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                LanguageError::new(
                    "LANG_DENO_COMPILE_INPUT",
                    format!("missing source text in ir '{}'", ir.source_id),
                )
            })?;

        let compiled = self.compile_script(source, None)?;
        let artifact = serde_json::to_vec(&compiled).map_err(|e| {
            LanguageError::new(
                "LANG_DENO_COMPILE_ENCODE",
                format!(
                    "failed to serialize compiled artifact '{}': {e}",
                    ir.source_id
                ),
            )
        })?;

        Ok(CompiledProgram {
            engine_id: self.id().to_string(),
            source_id: ir.source_id.clone(),
            artifact,
            metadata: json!({
                "target": options.target,
                "optimize_level": options.optimize_level,
                "emit_trace_hints": options.emit_trace_hints,
            }),
        })
    }

    fn run(
        &self,
        compiled: &CompiledProgram,
        input: Value,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionOutput, LanguageError> {
        let mut script: CompiledDenoSandboxScript = serde_json::from_slice(&compiled.artifact)
            .map_err(|e| {
                LanguageError::new(
                    "LANG_DENO_ARTIFACT_DECODE",
                    format!(
                        "failed to decode compiled artifact '{}': {e}",
                        compiled.source_id
                    ),
                )
            })?;

        if let Some(run_patch) = extract_run_patch(&ctx.metadata)? {
            script = self.compile_script(&script.original_source, Some(&run_patch))?;
        }

        let value = self.run_compiled(&script, &input)?;

        Ok(ExecutionOutput {
            value,
            trace: vec![
                format!("engine={}", self.id()),
                format!("script_id={}", script.script_id),
                format!("project={}", ctx.project),
                format!("pipeline={}", ctx.pipeline),
                format!("request_id={}", ctx.request_id),
            ],
        })
    }
}

/// Extracts an optional per-run sandbox patch from execution metadata.
///
/// Accepted keys:
///
/// - `languageRunPatch`
/// - `denoSandboxRunPatch` (backward compatibility)
fn extract_run_patch(metadata: &Value) -> Result<Option<DenoSandboxConfigPatch>, LanguageError> {
    let raw = metadata
        .get("languageRunPatch")
        .or_else(|| metadata.get("denoSandboxRunPatch"));
    let Some(raw) = raw else {
        return Ok(None);
    };
    let patch = serde_json::from_value(raw.clone()).map_err(|e| {
        LanguageError::new(
            "LANG_DENO_RUN_PATCH",
            format!("invalid metadata.languageRunPatch|denoSandboxRunPatch: {e}"),
        )
    })?;
    Ok(Some(patch))
}

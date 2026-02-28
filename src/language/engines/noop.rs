//! Minimal language engine used for testing and scaffolding.
//!
//! The noop engine:
//!
//! - parses `.zf.json` as JSON
//! - wraps `.tsx` as plain template payload
//! - serializes/deserializes artifacts without optimization

use serde_json::{Value, json};

use crate::language::interface::LanguageEngine;
use crate::language::model::{
    COMPILE_TARGET_PIPELINE, CompileOptions, CompiledProgram, ExecutionContext, ExecutionOutput,
    LanguageError, ModuleSource, ProgramIr, SourceKind,
};

/// Reference language engine with no sandboxing and no execution optimizations.
#[derive(Default)]
pub struct NoopLanguageEngine;

impl LanguageEngine for NoopLanguageEngine {
    fn id(&self) -> &'static str {
        "language.noop"
    }

    fn parse(&self, module: &ModuleSource) -> Result<ProgramIr, LanguageError> {
        let body = match module.kind {
            SourceKind::ZfJson => serde_json::from_str::<Value>(&module.code).map_err(|err| {
                LanguageError::new(
                    "LANG_PARSE_TPJSON",
                    format!("invalid .zf.json source '{}': {err}", module.id),
                )
            })?,
            SourceKind::Tsx => json!({ "template": module.code }),
        };

        Ok(ProgramIr {
            source_id: module.id.clone(),
            kind: module.kind,
            body,
        })
    }

    fn compile(
        &self,
        ir: &ProgramIr,
        options: &CompileOptions,
    ) -> Result<CompiledProgram, LanguageError> {
        let metadata = json!({
            "target": options.target,
            "optimize_level": options.optimize_level,
            "emit_trace_hints": options.emit_trace_hints,
        });

        let artifact = serde_json::to_vec(&ir.body).map_err(|err| {
            LanguageError::new(
                "LANG_COMPILE",
                format!(
                    "failed to encode compiled artifact '{}': {err}",
                    ir.source_id
                ),
            )
        })?;

        Ok(CompiledProgram {
            engine_id: self.id().to_string(),
            source_id: ir.source_id.clone(),
            artifact,
            metadata,
        })
    }

    fn run(
        &self,
        compiled: &CompiledProgram,
        input: Value,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionOutput, LanguageError> {
        let decoded = serde_json::from_slice::<Value>(&compiled.artifact).map_err(|err| {
            LanguageError::new(
                "LANG_RUN_DECODE",
                format!(
                    "failed to decode compiled artifact '{}': {err}",
                    compiled.source_id
                ),
            )
        })?;

        let trace = vec![
            format!("engine={}", self.id()),
            format!("project={}", ctx.project),
            format!("pipeline={}", ctx.pipeline),
            format!("request_id={}", ctx.request_id),
            format!(
                "target={}",
                compiled
                    .metadata
                    .get("target")
                    .and_then(Value::as_str)
                    .unwrap_or(COMPILE_TARGET_PIPELINE)
            ),
        ];

        Ok(ExecutionOutput {
            value: json!({
                "compiled": decoded,
                "input": input,
            }),
            trace,
        })
    }
}

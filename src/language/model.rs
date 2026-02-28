//! Shared language-layer domain model.

use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Extension for Zebflow reactive templates.
pub const TSX_EXTENSION: &str = "tsx";
/// Extension for Zebflow pipeline contracts.
pub const ZF_JSON_EXTENSION: &str = "zf.json";

/// Compile target id for pipeline execution.
pub const COMPILE_TARGET_PIPELINE: &str = "pipeline";
/// Compile target id for backend script execution.
pub const COMPILE_TARGET_BACKEND: &str = "backend";
/// Compile target id for frontend script execution.
pub const COMPILE_TARGET_FRONTEND: &str = "frontend";

/// Source module type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceKind {
    /// Template source (`*.tsx`).
    Tsx,
    /// Pipeline contract source (`*.zf.json`).
    ZfJson,
}

/// Raw source module submitted to a language engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSource {
    /// Logical source id.
    pub id: String,
    /// Optional filesystem path.
    pub source_path: Option<PathBuf>,
    /// Module kind.
    pub kind: SourceKind,
    /// Source text body.
    pub code: String,
}

/// Engine-specific intermediate representation payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramIr {
    /// Source id for traceability.
    pub source_id: String,
    /// Source kind.
    pub kind: SourceKind,
    /// Opaque IR body.
    pub body: Value,
}

/// Compile options shared across engines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileOptions {
    /// Target id (`pipeline`, `backend`, `frontend`).
    pub target: String,
    /// Optimization level hint.
    pub optimize_level: u8,
    /// Whether to include trace hints in artifact metadata.
    pub emit_trace_hints: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            target: COMPILE_TARGET_PIPELINE.to_string(),
            optimize_level: 1,
            emit_trace_hints: true,
        }
    }
}

/// Compiled executable artifact returned by language engines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledProgram {
    /// Engine id that produced this artifact.
    pub engine_id: String,
    /// Source id used for compilation.
    pub source_id: String,
    /// Serialized executable payload.
    pub artifact: Vec<u8>,
    /// Engine metadata (optimization level, target, etc.).
    pub metadata: Value,
}

/// Runtime context supplied when running a compiled program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Project id.
    pub project: String,
    /// Pipeline id.
    pub pipeline: String,
    /// Request/run id.
    pub request_id: String,
    /// Additional metadata envelope.
    pub metadata: Value,
}

/// Program execution output envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOutput {
    /// Program return value.
    pub value: Value,
    /// Trace entries emitted by engine.
    pub trace: Vec<String>,
}

/// Language layer error model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageError {
    /// Stable error code.
    pub code: &'static str,
    /// Human-readable error message.
    pub message: String,
}

impl LanguageError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl Display for LanguageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for LanguageError {}

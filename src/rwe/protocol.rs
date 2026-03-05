//! Language-agnostic RWE protocol contracts.
//!
//! These types are intended as stable JSON envelopes for adapters in other
//! runtimes (Python/FastAPI, Node, Go, etc.). External services can call a
//! Zebflow RWE host through HTTP/IPC using these request/response payloads.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::model::{
    CompiledTemplate, ReactiveWebOptions, RenderContext, RenderOutput, TemplateSource,
};

/// Current protocol version identifier.
pub const RWE_PROTOCOL_VERSION: &str = "rwe.v1";

/// Shared request metadata used by all protocol operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMeta {
    /// Protocol version expected by client/server.
    pub version: String,
    /// Target RWE engine id (for example `rwe`).
    pub rwe_engine: String,
    /// Target language engine id used for control script compile/run.
    pub language_engine: String,
}

impl Default for ProtocolMeta {
    fn default() -> Self {
        Self {
            version: RWE_PROTOCOL_VERSION.to_string(),
            rwe_engine: "rwe".to_string(),
            language_engine: "language.deno_sandbox".to_string(),
        }
    }
}

/// Compile operation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileTemplateRequest {
    /// Protocol metadata.
    pub meta: ProtocolMeta,
    /// Source template payload.
    pub template: TemplateSource,
    /// Compile options.
    #[serde(default)]
    pub options: ReactiveWebOptions,
}

/// Compile operation success response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileTemplateResponse {
    /// Protocol metadata echoed by server.
    pub meta: ProtocolMeta,
    /// Compiled template artifact used for render operation.
    pub compiled: CompiledTemplate,
}

/// Render operation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderTemplateRequest {
    /// Protocol metadata.
    pub meta: ProtocolMeta,
    /// Previously compiled template artifact.
    pub compiled: CompiledTemplate,
    /// Request state/input payload.
    pub state: Value,
    /// Render context (route/request id/metadata).
    pub ctx: RenderContext,
}

/// Render operation success response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderTemplateResponse {
    /// Protocol metadata echoed by server.
    pub meta: ProtocolMeta,
    /// Final render output.
    pub output: RenderOutput,
}

/// Standard protocol error envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolError {
    /// Stable error code.
    pub code: String,
    /// Human-readable error message.
    pub message: String,
    /// Optional structured details.
    #[serde(default)]
    pub details: Option<Value>,
}

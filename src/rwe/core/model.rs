use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::config::{CompileOptions, RuntimeMode};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum HydrateMode {
    Off,
    Onload,
    Onview,
    Oninteract,
}

impl Default for HydrateMode {
    fn default() -> Self {
        Self::Onload
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportEdge {
    pub source: String,
    pub resolved: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub line: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledTemplate {
    pub engine: String,
    pub source_path: Option<String>,
    pub runtime_mode: RuntimeMode,
    pub deno_timeout_ms: u64,
    pub server_module_source: String,
    pub client_module_source: String,
    pub imports: Vec<ImportEdge>,
    pub diagnostics: Vec<Diagnostic>,
    pub hydrate_mode: HydrateMode,
    pub compile_options: CompileOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RenderMeta {
    pub html_bytes: usize,
    pub js_bytes: usize,
    pub css_bytes: usize,
    pub ssr_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderOutput {
    pub html: String,
    pub js: String,
    pub css: String,
    pub hydration_payload: Value,
    pub meta: RenderMeta,
}

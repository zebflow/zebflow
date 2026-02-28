//! Shared RWE model used by compile/render interfaces.

use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::language::CompiledProgram;

/// Raw template source consumed by RWE engines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSource {
    /// Logical template id.
    pub id: String,
    /// Optional source file path.
    pub source_path: Option<PathBuf>,
    /// Template markup payload.
    pub markup: String,
}

/// Compile/render options for RWE engines.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReactiveWebOptions {
    /// Enables HTML minification hints.
    pub minify_html: bool,
    /// Enables stricter compile-time checks.
    pub strict_mode: bool,
    /// Style processing mode.
    pub style_engine: StyleEngineMode,
    /// Runtime bundle flavor.
    pub runtime_mode: RuntimeMode,
    /// Reactive attribute scanning mode.
    pub reactive_mode: ReactiveMode,
    /// External resource allow-list.
    pub allow_list: ResourceAllowList,
    /// Explicitly allowed script URLs to keep in rendered HTML.
    pub load_scripts: Vec<String>,
    /// Component registry used during compile expansion.
    pub components: ComponentOptions,
    /// Language runtime options passed to script engine.
    pub language: LanguageOptions,
    /// Template import resolution options for this compile call.
    #[serde(default)]
    pub templates: TemplateOptions,
    /// Optional compile processor pipeline (for example `["tailwind", "markdown"]`).
    ///
    /// Behavior:
    ///
    /// - empty list: legacy defaults apply (`style_engine` drives Tailwind-like processing)
    /// - non-empty list: only listed processors are executed, in listed order
    #[serde(default)]
    pub processors: Vec<String>,
}

impl Default for ReactiveWebOptions {
    fn default() -> Self {
        Self {
            minify_html: false,
            strict_mode: true,
            style_engine: StyleEngineMode::TailwindLike,
            runtime_mode: RuntimeMode::Prod,
            reactive_mode: ReactiveMode::Bindings,
            allow_list: ResourceAllowList::default(),
            load_scripts: Vec::new(),
            components: ComponentOptions::default(),
            language: LanguageOptions::default(),
            templates: TemplateOptions::default(),
            processors: Vec::new(),
        }
    }
}

/// Compile-scoped template resolution settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplateOptions {
    /// Filesystem root used for `@/` and boundary-checked relative imports.
    #[serde(default)]
    pub template_root: Option<PathBuf>,
    /// Optional explicit stylesheet entry paths, relative to `template_root`.
    ///
    /// When empty, RWE probes deterministic defaults:
    ///
    /// - `styles/main.css`
    #[serde(default)]
    pub style_entries: Vec<String>,
}

/// Component registry settings for compile-time expansion.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComponentOptions {
    /// Map from component name to raw markup snippet.
    #[serde(default)]
    pub registry: BTreeMap<String, String>,
    /// When `true`, missing component names fail compilation.
    #[serde(default)]
    pub strict: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeMode {
    /// Development runtime bundle with debug helpers.
    Dev,
    /// Production runtime bundle.
    #[default]
    Prod,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum StyleEngineMode {
    /// Disable style preprocessing.
    Off,
    /// Lightweight tailwind-like token expansion.
    #[default]
    TailwindLike,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ReactiveMode {
    /// Disable reactive attribute scanning.
    Off,
    /// Enable reactive binding attribute scanning.
    #[default]
    Bindings,
}

/// Resource allow-list for HTML compile filtering.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceAllowList {
    /// Allowed stylesheet URL rules.
    #[serde(default)]
    pub css: Vec<String>,
    /// Allowed script URL rules.
    #[serde(default)]
    pub scripts: Vec<String>,
    /// Shared URL rules used by css/script checks.
    #[serde(default)]
    pub urls: Vec<String>,
}

/// Language-related options for template control scripts.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LanguageOptions {
    /// Optional runtime patch forwarded to language engine as metadata.
    #[serde(default)]
    pub run_patch: Option<Value>,
}

/// Runtime JavaScript bundle descriptor injected by renderer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeBundle {
    /// Bundle logical name.
    pub name: String,
    /// Bundle source text.
    pub source: String,
}

/// Collected reactive binding entry from template markup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactiveBinding {
    /// Binding kind (event/bind namespace).
    pub kind: String,
    /// Bound key/path/action string.
    pub key: String,
}

/// Diagnostic entry emitted during template compilation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactiveWebDiagnostic {
    /// Stable diagnostic code.
    pub code: String,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Compiled template artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledTemplate {
    /// RWE engine id that produced this artifact.
    pub engine_id: String,
    /// Source template id.
    pub template_id: String,
    /// Compiled HTML intermediate representation.
    pub html_ir: String,
    /// Optional extracted control-script source.
    #[serde(default)]
    pub control_script_source: Option<String>,
    /// Optional compiled logic artifact from language engine.
    pub compiled_logic: Option<CompiledProgram>,
    /// Runtime JS bundle to inject at render time.
    pub runtime_bundle: RuntimeBundle,
    /// Reactive bindings discovered during compile stage.
    #[serde(default)]
    pub reactive_bindings: Vec<ReactiveBinding>,
    /// Compile diagnostics.
    #[serde(default)]
    pub diagnostics: Vec<ReactiveWebDiagnostic>,
    /// `true` when template markup still contains dynamic class placeholders
    /// that require Tailwind-like CSS regeneration at render time.
    ///
    /// This keeps render-time style rebuild in `auto` mode and avoids paying
    /// that cost for fully static class templates.
    #[serde(default)]
    pub needs_runtime_tailwind_rebuild: bool,
    /// Aggregated exact dynamic-tailwind hint tokens declared via `tw-variants`.
    ///
    /// These tokens are compiled into static CSS once and reused per request.
    #[serde(default)]
    pub tailwind_variant_exact_tokens: Vec<String>,
    /// Aggregated wildcard pattern hints declared via `tw-variants`
    /// (for example `bg-[*]`, `text-[*]`).
    ///
    /// Wildcard patterns imply a lightweight runtime transform hook.
    #[serde(default)]
    pub tailwind_variant_patterns: Vec<String>,
    /// Effective compile/render options.
    pub options: ReactiveWebOptions,
}

/// Request-level render context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderContext {
    /// Route path currently rendered.
    pub route: String,
    /// Request/run id.
    pub request_id: String,
    /// Additional metadata for SSR/runtime bootstrap.
    pub metadata: Value,
}

/// Render result returned by RWE engines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderOutput {
    /// Final HTML output.
    pub html: String,
    /// Hydration/bootstrap payload for client runtime.
    pub hydration_payload: Value,
    /// Render trace entries.
    pub trace: Vec<String>,
}

/// RWE layer error model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactiveWebError {
    /// Stable error code.
    pub code: &'static str,
    /// Human-readable error message.
    pub message: String,
}

impl ReactiveWebError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl Display for ReactiveWebError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ReactiveWebError {}

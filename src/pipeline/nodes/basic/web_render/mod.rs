//! `n.web.render` — compile and serve an RWE page.
//!
//! Takes the upstream payload as template state, compiles the specified TSX
//! through the Reactive Web Engine, and emits rendered HTML + client scripts.
//!
//! # Pipeline position
//!
//! `n.web.render` is **never** a standalone node. It always sits downstream of
//! a trigger that owns the HTTP route. The route is the trigger's concern —
//! this node only knows about templates.
//!
//! ```text
//! | n.trigger.webhook --path /blog --method GET
//! | pg.query --credential main-db -- "SELECT * FROM posts ORDER BY created_at DESC"
//! | n.web.render --template pages/blog-home
//! ```
//!
//! # User-facing config (node level)
//!
//! | Field | Type | Required | Description |
//! |---|---|---|---|
//! | `template_path` | string | ✓ | TSX file path relative to `templates/`, e.g. `pages/blog-home` — omit `.tsx` extension |
//! | `load_scripts` | string | — | External script URLs, comma- or newline-separated. Each must match project `allow_list`. |
//!
//! Everything else — route, markup, options — is either derived by the platform
//! or configured project-wide in `zebflow.json`.
//!
//! # Config schema
//!
//! ```json
//! {
//!   "type": "object",
//!   "required": ["template_path"],
//!   "properties": {
//!     "template_path": {
//!       "type": "string",
//!       "description": "Path to the TSX page file relative to templates/. Example: 'pages/blog-home', 'pages/blog/post-detail'. Omit the .tsx extension — RWE resolves it automatically. The UI should show a file browser with TSX preview."
//!     },
//!     "load_scripts": {
//!       "type": "string",
//!       "description": "External script URLs to inject into the rendered page. Comma- or newline-separated. Each URL is validated against the project allow_list at save time — URLs not in the allow_list show a danger indicator and block saving."
//!     }
//!   }
//! }
//! ```
//!
//! # Studio UI hint
//!
//! ```text
//! ┌─ Web Render ──────────────────────────────────────────────────────┐
//! │ Template                                                          │
//! │ ┌───────────────────────────────────────┐  [Browse]              │
//! │ │ pages/blog-home                       │  (TSX file browser     │
//! │ └───────────────────────────────────────┘   with preview pane)   │
//! │                                                                   │
//! │ External Scripts  (optional)                                      │
//! │ ┌───────────────────────────────────────────────────────────────┐ │
//! │ │ https://cdn.example.com/widget.js,                            │ │
//! │ │ https://cdn.other.com/analytics.js                            │ │
//! │ └───────────────────────────────────────────────────────────────┘ │
//! │  ⚠ cdn.other.com/analytics.js not in project allow_list           │
//! │    Go to Settings → Policy → RWE → Allow List to add it          │
//! │    [Save disabled until all URLs are trusted]                     │
//! └───────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # TSX authoring rules
//!
//! - `@/` resolves to the project `templates/` root at compile time.
//! - `import { useState, useEffect, ... } from "rwe"` — stripped at compile,
//!   these are injected as `globalThis` by the RWE runtime. Never use
//!   `"npm:preact/hooks"`.
//! - `import Button from "@/components/ui/button"` — resolved at compile time.
//! - Never call `render()` manually. Never import from `"npm:preact"`.
//!
//! # RWE hook globals (available in every TSX without import)
//!
//! | Hook | Description |
//! |---|---|
//! | `useState` | Local component state |
//! | `useEffect` | Side effects and lifecycle |
//! | `useRef` | DOM / stable value ref |
//! | `useMemo` | Memoized derived value |
//! | `useDebounce(value, ms)` | Debounced value |
//! | `useLocalStorage(key, init)` | State persisted in localStorage |
//! | `useWindowSize()` | Reactive `{ width, height }` |
//! | `useOnClickOutside(ref, fn)` | Click-outside handler |
//! | `useInterval(fn, ms)` | Safe interval with cleanup |
//! | `usePrevious(value)` | Value from previous render |
//!
//! # Blessed visualization libraries (zeb/ namespace)
//!
//! Heavy libraries are imported directly in TSX via the `zeb/` namespace.
//! They are pre-trusted and do not require `allow_list` entries or `load_scripts`.
//!
//! ```tsx
//! import { Scene, PerspectiveCamera, WebGLRenderer } from "zeb/threejs";
//! import { DeckGL, ScatterplotLayer } from "zeb/deckgl";
//! import { select, scaleLinear } from "zeb/d3";
//! import { Map } from "zeb/mapbox";
//! ```
//!
//! # Generated client scripts
//!
//! After render, RWE emits `compiled_scripts: Vec<CompiledScript>` — each with
//! a deterministic `content_hash`. The platform persists these at:
//!
//! ```text
//! data/generated/scripts/{hash}.js     ← not git-synced, content-addressed
//! ```
//!
//! Served at (configurable in project settings, default auto-derived):
//!
//! ```text
//! /assets/{owner}/{project}/generated/scripts/{hash}.js
//! ```
//!
//! With `Cache-Control: immutable` — safe for CDN caching indefinitely.
//! Same component used across 10 pages = one file on disk (free deduplication).
//!
//! # Project-level RWE settings (`zebflow.json` → `rwe` section)
//!
//! Configured in **Settings → Policy → Reactive Web Engine**.
//! Applies to all `n.web.render` nodes in the project.
//!
//! | Key | Type | Default | Description |
//! |---|---|---|---|
//! | `scripts_route` | `string \| null` | `null` | URL prefix for generated scripts. `null` = auto-derive from owner/project |
//! | `minify_html` | bool | `true` | Strip whitespace from HTML output |
//! | `strict_mode` | bool | `true` | Fail on template compile warnings |
//! | `style_engine` | enum | `"tailwind-like"` | CSS generation: `"tailwind-like"` or `"off"` |
//! | `runtime_mode` | enum | `"prod"` | Bundle flavor: `"dev"` (source maps) or `"prod"` |
//! | `allow_list` | object | `{}` | URL glob rules for scripts and CSS |
//!
//! Example `zebflow.json`:
//!
//! ```json
//! {
//!   "rwe": {
//!     "scripts_route": null,
//!     "minify_html": true,
//!     "strict_mode": true,
//!     "style_engine": "tailwind-like",
//!     "runtime_mode": "prod",
//!     "allow_list": {
//!       "scripts": ["cdnjs.cloudflare.com/*", "cdn.jsdelivr.net/*"],
//!       "css": [],
//!       "urls": []
//!     }
//!   }
//! }
//! ```
//!
//! # Config struct
//!
//! User-facing fields stored in pipeline JSON:
//!
//! - `template_path` — the TSX file to render.
//! - `load_scripts` — comma/newline-separated external script URLs.
//!
//! Internal fields managed by the platform (not set by users):
//!
//! - `markup` — populated at request time from `template_path` by
//!   `hydrate_web_render_markup_from_templates`. Never stored in pipeline JSON.
//! - `options` — merged from project `zebflow.json → rwe` settings at
//!   runtime. Node config does not expose individual option fields.
//! - `route` — derived from the upstream trigger's path at execution time.
//!   Not a node config field.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput};
use crate::pipeline::{PipelineError, NodeDefinition};
use crate::pipeline::model::{DslFlag, DslFlagKind};
use crate::language::LanguageEngine;
use crate::rwe::{CompiledTemplate, ReactiveWebEngine, ReactiveWebOptions, TemplateSource};

/// Node kind identifier.
pub const NODE_KIND: &str = "n.web.render";
/// Standard input pin.
pub const INPUT_PIN_IN: &str = "in";
/// Success output pin.
pub const OUTPUT_PIN_OUT: &str = "out";
/// Error output pin.
pub const OUTPUT_PIN_ERROR: &str = "error";

/// Unified node-definition metadata for `n.web.render`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Web Render".to_string(),
        description: "Render RWE template into HTML using upstream payload as template input."
            .to_string(),
        input_schema: serde_json::json!({
            "type":"object",
            "description":"Template input object."
        }),
        output_schema: serde_json::json!({
            "type":"object",
            "properties":{
                "html":{"type":"string"},
                "compiled_scripts":{"type":"array"},
                "hydration_payload":{"type":"object"}
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string(), OUTPUT_PIN_ERROR.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: serde_json::json!({
            "type": "object",
            "required": ["template_path"],
            "properties": {
                "template_path": {
                    "type": "string",
                    "description": "Path to the TSX page file relative to templates/. Example: 'pages/blog-home', 'pages/blog/post-detail'. Omit the .tsx extension — RWE resolves it automatically."
                },
                "load_scripts": {
                    "type": "string",
                    "description": "External script URLs to inject into the rendered page. Comma- or newline-separated. Each URL must match a pattern in the project allow_list or the node cannot be saved."
                }
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--template-path".to_string(),
                config_key: "template_path".to_string(),
                description: "TSX page file relative to templates/, e.g. pages/blog-home. Omit the .tsx extension.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--load-scripts".to_string(),
                config_key: "load_scripts".to_string(),
                description: "External script URLs to inject, comma-separated. Each URL must match the project allow_list.".to_string(),
                kind: DslFlagKind::CommaSeparatedList,
                required: false,
            },
        ],
        ai_tool: Default::default(),
    }
}

/// Static configuration for `n.web.render`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// TSX file path relative to `templates/`, e.g. `pages/blog-home`.
    /// This is the primary user-facing field set via DSL `--template-path`.
    #[serde(default)]
    pub template_path: String,
    /// Template id for traceability. Derived from `template_path` at runtime when empty.
    #[serde(default)]
    pub template_id: String,
    /// Route passed to render context. Overridden at runtime from `metadata["route"]`.
    #[serde(default)]
    pub route: String,
    /// Inline TSX/template markup used when executing directly from a graph node.
    #[serde(default)]
    pub markup: Option<String>,
    /// RWE compile/render options.
    #[serde(default)]
    pub options: ReactiveWebOptions,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            template_path: String::new(),
            template_id: String::new(),
            route: "/".to_string(),
            markup: None,
            options: ReactiveWebOptions::default(),
        }
    }
}

/// Compiled node artifact persisted by the framework.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Compiled {
    /// Runtime node id.
    pub node_id: String,
    /// Effective node config.
    pub config: Config,
    /// Compiled RWE template.
    pub template: CompiledTemplate,
}

/// Runtime node wrapper using a precompiled [`Compiled`] payload.
pub struct Node {
    compiled: Compiled,
}

impl Node {
    /// Compiles a `web.render` node with provided engines and template source.
    pub fn compile(
        node_id: &str,
        config: &Config,
        template: &TemplateSource,
        rwe: &dyn ReactiveWebEngine,
        language: &dyn LanguageEngine,
    ) -> Result<Compiled, PipelineError> {
        let compiled_template = rwe
            .compile_template(template, language, &config.options)
            .map_err(|e| {
                PipelineError::new(
                    "FW_NODE_WEB_RENDER_COMPILE",
                    format!("failed compiling node '{}': {}", node_id, e),
                )
            })?;

        Ok(Compiled {
            node_id: node_id.to_string(),
            config: config.clone(),
            template: compiled_template,
        })
    }

    /// Creates a node instance from compiled artifact.
    pub fn new(compiled: Compiled) -> Self {
        Self { compiled }
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        NODE_KIND
    }

    fn input_pins(&self) -> &'static [&'static str] {
        &[INPUT_PIN_IN]
    }

    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT, OUTPUT_PIN_ERROR]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        if input.input_pin != INPUT_PIN_IN {
            return Err(PipelineError::new(
                "FW_NODE_WEB_RENDER_INPUT_PIN",
                format!(
                    "node '{}' received unsupported input pin '{}' (expected '{}')",
                    self.compiled.node_id, input.input_pin, INPUT_PIN_IN
                ),
            ));
        }

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({
                "node_id": self.compiled.node_id,
                "template_id": self.compiled.config.template_id,
                "route": self.compiled.config.route,
                "state": input.payload,
                "metadata": input.metadata,
            }),
            trace: vec![
                format!("node={}", self.compiled.node_id),
                format!("node_kind={}", NODE_KIND),
                format!("output_pin={}", OUTPUT_PIN_OUT),
            ],
        })
    }
}

/// Runs full render phase for a previously compiled node.
///
/// This helper is intentionally separate from [`NodeHandler::execute`] so
/// orchestration layers can choose between lightweight planning output and full
/// HTML render output.
pub fn render_with_engines(
    compiled: &Compiled,
    state: Value,
    metadata: Value,
    rwe: &dyn ReactiveWebEngine,
    language: &dyn LanguageEngine,
    request_id: &str,
) -> Result<NodeExecutionOutput, PipelineError> {
    // Route comes from metadata["route"] (injected by the webhook handler via PipelineContext).
    // Fall back to the config field for inline / test execution paths.
    let route = metadata
        .get("route")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(&compiled.config.route)
        .to_string();

    let rendered = rwe
        .render(
            &compiled.template,
            state,
            language,
            &crate::rwe::RenderContext {
                route,
                request_id: request_id.to_string(),
                metadata,
            },
        )
        .map_err(|e| {
            PipelineError::new(
                "FW_NODE_WEB_RENDER_RUN",
                format!("failed rendering node '{}': {}", compiled.node_id, e),
            )
        })?;

    let mut trace = vec![
        format!("node={}", compiled.node_id),
        format!("node_kind={}", NODE_KIND),
        format!("output_pin={}", OUTPUT_PIN_OUT),
    ];
    trace.extend(rendered.trace);

    Ok(NodeExecutionOutput {
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        payload: json!({
            "html": rendered.html,
            "compiled_scripts": rendered.compiled_scripts,
            "hydration_payload": rendered.hydration_payload,
        }),
        trace,
    })
}

/// Compiles and renders directly from inline node config markup.
pub fn render_from_config(
    node_id: &str,
    config: &Config,
    state: Value,
    metadata: Value,
    rwe: &dyn ReactiveWebEngine,
    language: &dyn LanguageEngine,
    request_id: &str,
) -> Result<NodeExecutionOutput, PipelineError> {
    let markup = config.markup.clone().ok_or_else(|| {
        PipelineError::new(
            "FW_NODE_WEB_RENDER_CONFIG",
            format!(
                "node '{}' requires config.markup for inline execution",
                node_id
            ),
        )
    })?;
    let compiled = Node::compile(
        node_id,
        config,
        &TemplateSource {
            id: config.template_id.clone(),
            source_path: None,
            markup,
        },
        rwe,
        language,
    )?;
    render_with_engines(&compiled, state, metadata, rwe, language, request_id)
}

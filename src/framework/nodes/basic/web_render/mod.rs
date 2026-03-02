//! Built-in web render framework node.
//!
//! This node composes:
//!
//! - an RWE engine for template compile/render
//! - a language engine for script compilation/execution hooks
//!
//! and exposes the result through framework pin contracts.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::framework::nodes::{FrameworkNode, NodeExecutionInput, NodeExecutionOutput};
use crate::framework::{FrameworkError, NodeDefinition};
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
        ai_tool: Default::default(),
    }
}

/// Static configuration for `n.web.render`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Template id for traceability.
    pub template_id: String,
    /// Route passed to render context.
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
            template_id: "home".to_string(),
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
    ) -> Result<Compiled, FrameworkError> {
        let compiled_template = rwe
            .compile_template(template, language, &config.options)
            .map_err(|e| {
                FrameworkError::new(
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
impl FrameworkNode for Node {
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
    ) -> Result<NodeExecutionOutput, FrameworkError> {
        if input.input_pin != INPUT_PIN_IN {
            return Err(FrameworkError::new(
                "FW_NODE_WEB_RENDER_INPUT_PIN",
                format!(
                    "node '{}' received unsupported input pin '{}' (expected '{}')",
                    self.compiled.node_id, input.input_pin, INPUT_PIN_IN
                ),
            ));
        }

        Ok(NodeExecutionOutput {
            output_pin: OUTPUT_PIN_OUT.to_string(),
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
/// This helper is intentionally separate from [`FrameworkNode::execute`] so
/// orchestration layers can choose between lightweight planning output and full
/// HTML render output.
pub fn render_with_engines(
    compiled: &Compiled,
    state: Value,
    metadata: Value,
    rwe: &dyn ReactiveWebEngine,
    language: &dyn LanguageEngine,
    request_id: &str,
) -> Result<NodeExecutionOutput, FrameworkError> {
    let rendered = rwe
        .render(
            &compiled.template,
            state,
            language,
            &crate::rwe::RenderContext {
                route: compiled.config.route.clone(),
                request_id: request_id.to_string(),
                metadata,
            },
        )
        .map_err(|e| {
            FrameworkError::new(
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
        output_pin: OUTPUT_PIN_OUT.to_string(),
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
) -> Result<NodeExecutionOutput, FrameworkError> {
    let markup = config.markup.clone().ok_or_else(|| {
        FrameworkError::new(
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

//! Script execution node backed by the language engine.

use crate::pipeline::{
    PipelineError, NodeDefinition,
    nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput},
};
use crate::pipeline::model::{LayoutItem, NodeFieldDef, NodeFieldType, SidebarSection, SidebarItem};
use crate::language::{
    COMPILE_TARGET_BACKEND, CompileOptions, CompiledProgram, ExecutionContext, LanguageEngine,
    ModuleSource, SourceKind,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub const NODE_KIND: &str = "n.script";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

/// Unified node-definition metadata for `n.script`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Script".to_string(),
        description:
            "Execute sandboxed Deno logic with runtime signature async function(input, n, ctx)."
                .to_string(),
        input_schema: serde_json::json!({
            "type":"object",
            "description":"Upstream payload available as `input`."
        }),
        output_schema: serde_json::json!({
            "type":"any",
            "description":"Script return value forwarded downstream."
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: Default::default(),
        fields: vec![
            NodeFieldDef { name: "title".to_string(), label: "Title".to_string(), field_type: NodeFieldType::Text, help: Some("Override display title for this node.".to_string()), ..Default::default() },
            NodeFieldDef {
                name: "source".to_string(),
                label: "Source".to_string(),
                field_type: NodeFieldType::CodeEditor,
                language: Some("javascript".to_string()),
                span: Some("full".to_string()),
                help: Some("Deno JavaScript expression/body. Must return next payload.".to_string()),
                default_value: Some(serde_json::json!("return input;")),
                sidebar: vec![
                    SidebarSection {
                        title: "Input".to_string(),
                        items: vec![
                            SidebarItem { label: "input".to_string(), type_hint: Some("any".to_string()), description: Some("Upstream payload passed into the script.".to_string()) },
                        ],
                    },
                    SidebarSection {
                        title: "Return".to_string(),
                        items: vec![
                            SidebarItem { label: "payload".to_string(), type_hint: Some("object".to_string()), description: Some("Value returned becomes the downstream payload.".to_string()) },
                        ],
                    },
                    SidebarSection {
                        title: "Built-ins".to_string(),
                        items: vec![
                            SidebarItem { label: "console.log(...)".to_string(), type_hint: Some("void".to_string()), description: Some("Log to pipeline trace output.".to_string()) },
                            SidebarItem { label: "n.pg.query({...})".to_string(), type_hint: Some("Promise<rows>".to_string()), description: Some("Execute a Postgres query inline.".to_string()) },
                            SidebarItem { label: "n.http.request({...})".to_string(), type_hint: Some("Promise<response>".to_string()), description: Some("Make an HTTP request inline.".to_string()) },
                            SidebarItem { label: "ctx.pipeline".to_string(), type_hint: Some("string".to_string()), description: Some("Current pipeline id.".to_string()) },
                            SidebarItem { label: "ctx.request_id".to_string(), type_hint: Some("string".to_string()), description: Some("Unique execution request id.".to_string()) },
                        ],
                    },
                ],
                ..Default::default()
            },
        ],
        layout: vec![
            LayoutItem::Field("title".to_string()),
            LayoutItem::Field("source".to_string()),
        ],
        ai_tool: Default::default(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub source: String,
}

pub struct Node {
    node_id: String,
    compiled: CompiledProgram,
    language: std::sync::Arc<dyn LanguageEngine>,
}

impl Node {
    pub fn new(
        node_id: &str,
        config: Config,
        language: std::sync::Arc<dyn LanguageEngine>,
    ) -> Result<Self, PipelineError> {
        if config.source.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_SCRIPT_CONFIG",
                format!("node '{}' requires config.source", node_id),
            ));
        }
        let module = ModuleSource {
            id: format!("pipeline:{node_id}"),
            source_path: None,
            kind: SourceKind::Tsx,
            code: config.source,
        };
        let ir = language.parse(&module).map_err(|err| {
            PipelineError::new(
                "FW_NODE_SCRIPT_PARSE",
                format!("node '{}': {}", node_id, err),
            )
        })?;
        let compiled = language
            .compile(
                &ir,
                &CompileOptions {
                    target: COMPILE_TARGET_BACKEND.to_string(),
                    optimize_level: 1,
                    emit_trace_hints: true,
                },
            )
            .map_err(|err| {
                PipelineError::new(
                    "FW_NODE_SCRIPT_COMPILE",
                    format!("node '{}': {}", node_id, err),
                )
            })?;
        Ok(Self {
            node_id: node_id.to_string(),
            compiled,
            language,
        })
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
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        if input.input_pin != INPUT_PIN_IN {
            return Err(PipelineError::new(
                "FW_NODE_SCRIPT_INPUT_PIN",
                format!("unsupported input pin '{}'", input.input_pin),
            ));
        }

        let ctx = ExecutionContext {
            project: input
                .metadata
                .get("project")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            pipeline: input
                .metadata
                .get("pipeline")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            request_id: input
                .metadata
                .get("request_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            metadata: input.metadata.clone(),
        };

        let out = self
            .language
            .run(&self.compiled, input.payload, &ctx)
            .map_err(|err| {
                PipelineError::new(
                    "FW_NODE_SCRIPT_RUN",
                    format!("node '{}': {}", self.node_id, err),
                )
            })?;

        let mut trace = vec![format!("node_kind={NODE_KIND}")];
        trace.extend(out.trace);

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: out.value,
            trace,
        })
    }
}

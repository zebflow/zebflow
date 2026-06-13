//! Script execution node backed by the language engine.

use crate::language::{
    COMPILE_TARGET_BACKEND, CompileOptions, CompiledProgram, ExecutionContext, LanguageEngine,
    ModuleSource, SourceKind,
};
use crate::pipeline::model::{
    LayoutItem, NodeFieldDef, NodeFieldType, SidebarItem, SidebarSection,
};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
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
            "Execute sandboxed Deno logic with runtime signature async function(input, n, ctx). \
            Signals: to emit real-time signals (progress, status, custom events), include a \
            __signal key in the return value. The engine strips it from the downstream payload \
            and routes it through the ExecutionBus. Supports string (\"processing…\"), \
            object ({kind, message, data}), or array of either. Example: \
            return { result: 42, __signal: {kind: \"progress\", message: \"step 2 done\"} };"
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
        dsl_flags: vec![
            crate::pipeline::model::DslFlag {
                flag: "--lang".to_string(),
                config_key: "language".to_string(),
                description: "Script language (default: js). Supported: js, ts. Source is provided via -- body.".to_string(),
                kind: crate::pipeline::model::DslFlagKind::Scalar,
                required: false,
            },
            crate::pipeline::model::DslFlag {
                flag: "--source-expr".to_string(),
                config_key: "source_expr".to_string(),
                description: "JS expression returning the script source string. Overrides the body source at runtime.".to_string(),
                kind: crate::pipeline::model::DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "language".to_string(),
                label: "Language".to_string(),
                field_type: NodeFieldType::Select,
                options: vec![
                    crate::pipeline::model::SelectOptionDef { value: "js".to_string(), label: "JavaScript".to_string() },
                    crate::pipeline::model::SelectOptionDef { value: "ts".to_string(), label: "TypeScript".to_string() },
                ],
                default_value: Some(serde_json::json!("js")),
                help: Some("Script language (default: js).".to_string()),
                ..Default::default()
            },
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
                            SidebarItem { label: "input".to_string(), type_hint: Some("any".to_string()), description: Some("Upstream payload from the previous node.".to_string()) },
                        ],
                    },
                    SidebarSection {
                        title: "Context".to_string(),
                        items: vec![
                            SidebarItem { label: "ctx.pipeline".to_string(), type_hint: Some("string".to_string()), description: Some("Current pipeline id.".to_string()) },
                            SidebarItem { label: "ctx.request_id".to_string(), type_hint: Some("string".to_string()), description: Some("Unique execution request id.".to_string()) },
                            SidebarItem { label: "ctx.nodes".to_string(), type_hint: Some("object".to_string()), description: Some("Map of all previous node outputs. Access by node id: ctx.nodes['a'].".to_string()) },
                            SidebarItem { label: "ctx.placeholder".to_string(), type_hint: Some("object".to_string()), description: Some("Resolved placeholder values (credentials, config).".to_string()) },
                        ],
                    },
                    SidebarSection {
                        title: "Trigger".to_string(),
                        items: vec![
                            SidebarItem { label: "ctx.trigger".to_string(), type_hint: Some("object|null".to_string()), description: Some("Full trigger event snapshot from the entry node.".to_string()) },
                            SidebarItem { label: "ctx.trigger.auth".to_string(), type_hint: Some("object|null".to_string()), description: Some("Verified JWT claims — immutable across the pipeline.".to_string()) },
                            SidebarItem { label: "ctx.trigger.params".to_string(), type_hint: Some("object".to_string()), description: Some("URL path params (:id etc) from the request.".to_string()) },
                            SidebarItem { label: "ctx.trigger.query".to_string(), type_hint: Some("object".to_string()), description: Some("Query string params from the request.".to_string()) },
                            SidebarItem { label: "ctx.trigger.headers".to_string(), type_hint: Some("object".to_string()), description: Some("Safe subset of request headers.".to_string()) },
                        ],
                    },
                    SidebarSection {
                        title: "Return".to_string(),
                        items: vec![
                            SidebarItem { label: "return value".to_string(), type_hint: Some("any".to_string()), description: Some("Returned value becomes the downstream payload.".to_string()) },
                            SidebarItem { label: "__signal".to_string(), type_hint: Some("string|object|array".to_string()), description: Some("Optional key in return value for real-time signals. Stripped before downstream.".to_string()) },
                        ],
                    },
                    SidebarSection {
                        title: "Built-ins".to_string(),
                        items: vec![
                            SidebarItem { label: "console.log(...)".to_string(), type_hint: Some("void".to_string()), description: Some("Log to pipeline trace output.".to_string()) },
                            SidebarItem { label: "n.time.now()".to_string(), type_hint: Some("number".to_string()), description: Some("Current Unix timestamp in milliseconds.".to_string()) },
                            SidebarItem { label: "n.pg.query({...})".to_string(), type_hint: Some("Promise<rows>".to_string()), description: Some("Execute a Postgres query inline.".to_string()) },
                            SidebarItem { label: "n.http.request({...})".to_string(), type_hint: Some("Promise<response>".to_string()), description: Some("Make an HTTP request inline.".to_string()) },
                        ],
                    },
                ],
                ..Default::default()
            },
            NodeFieldDef {
                name: "source_expr".to_string(),
                label: "Source Expr".to_string(),
                field_type: NodeFieldType::Textarea,
                rows: Some(3),
                help: Some("JS expression returning the script source string. Overrides the source editor above.".to_string()),
                ..Default::default()
            },
        ],
        layout: vec![
            LayoutItem::Field("language".to_string()),
            LayoutItem::Field("source".to_string()),
            LayoutItem::Field("source_expr".to_string()),
        ],
        ai_tool: crate::pipeline::model::NodeAiToolDefinition {
            registered: true,
            tool_name: "run_script".to_string(),
            tool_description: "Execute a JavaScript snippet in the Deno sandbox. Args: code (required).".to_string(),
            tool_input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "code": { "type": "string", "description": "JavaScript code to execute" }
                },
                "required": ["code"]
            }),
        },
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub source_expr: Option<String>,
}

pub struct Node {
    node_id: String,
    compiled: Option<CompiledProgram>,
    source_expr: Option<String>,
    language: std::sync::Arc<dyn LanguageEngine>,
}

impl Node {
    pub fn new(
        node_id: &str,
        config: Config,
        language: std::sync::Arc<dyn LanguageEngine>,
    ) -> Result<Self, PipelineError> {
        let has_source = !config.source.trim().is_empty();
        let has_expr = config
            .source_expr
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .len()
            > 0;
        if !has_source && !has_expr {
            return Err(PipelineError::new(
                "FW_NODE_SCRIPT_CONFIG",
                format!("node '{}' requires source or source_expr", node_id),
            ));
        }
        if has_source && has_expr {
            return Err(PipelineError::new(
                "FW_NODE_SCRIPT_CONFIG",
                "set either source or source_expr, not both",
            ));
        }
        if has_expr {
            return Ok(Self {
                node_id: node_id.to_string(),
                compiled: None,
                source_expr: config.source_expr,
                language,
            });
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
            compiled: Some(compiled),
            source_expr: None,
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

        let compiled = if let Some(ref pre) = self.compiled {
            std::borrow::Cow::Borrowed(pre)
        } else if let Some(ref expr) = self.source_expr {
            let source_val = super::util::eval_deno_expr(
                self.language.as_ref(),
                expr,
                &input.payload,
                &input.metadata,
            )?;
            let source = source_val
                .as_str()
                .ok_or_else(|| {
                    PipelineError::new("FW_NODE_SCRIPT_BINDING", "source_expr must return a string")
                })?
                .to_string();
            let module = ModuleSource {
                id: format!("pipeline:{}:dynamic", self.node_id),
                source_path: None,
                kind: SourceKind::Tsx,
                code: source,
            };
            let ir = self.language.parse(&module).map_err(|err| {
                PipelineError::new(
                    "FW_NODE_SCRIPT_PARSE",
                    format!("node '{}': {}", self.node_id, err),
                )
            })?;
            let c = self
                .language
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
                        format!("node '{}': {}", self.node_id, err),
                    )
                })?;
            std::borrow::Cow::Owned(c)
        } else {
            return Err(PipelineError::new(
                "FW_NODE_SCRIPT_CONFIG",
                "no source or source_expr configured",
            ));
        };

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
            trigger: input
                .metadata
                .get("trigger")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
            metadata: input.metadata.clone(),
        };

        let out = self
            .language
            .run(&compiled, input.payload, &ctx)
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

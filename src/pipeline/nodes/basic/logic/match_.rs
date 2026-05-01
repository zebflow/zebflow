//! `n.logic.match` — multi-case routing node.
//!
//! Evaluates a DSL expression to get a string value.
//! Routes to the matching case pin, or the default pin if no case matches.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::language::{
    COMPILE_TARGET_BACKEND, CompileOptions, CompiledProgram, LanguageEngine, ModuleSource,
    SourceKind,
};
use crate::pipeline::expr::build_expression_scope_input;
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "n.logic.match";
pub const INPUT_PIN_IN: &str = "in";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Match".to_string(),
        description:
            "Evaluates a DSL expression using $input/$trigger/$nodes and routes to matching case pin, or default."
                .to_string(),
        input_schema: serde_json::json!({ "type": "object" }),
        output_schema: serde_json::json!({ "type": "object" }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag {
                flag: "--expr".to_string(),
                config_key: "expression".to_string(),
                description: "JS expression returning a string to match against case pins."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--cases".to_string(),
                config_key: "cases".to_string(),
                description: "Comma-separated case values; each becomes an output pin.".to_string(),
                kind: DslFlagKind::CommaSeparatedList,
                required: false,
            },
            DslFlag {
                flag: "--default".to_string(),
                config_key: "default".to_string(),
                description: "Pin name to route to when no case matches.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType};
            vec![
                NodeFieldDef {
                    name: "title".to_string(),
                    label: "Title".to_string(),
                    field_type: NodeFieldType::Text,
                    help: Some("Override display title for this node.".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "expression".to_string(),
                    label: "Expression".to_string(),
                    field_type: NodeFieldType::Textarea,
                    rows: Some(4),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "cases".to_string(),
                    label: "Cases (one per line)".to_string(),
                    field_type: NodeFieldType::Textarea,
                    rows: Some(5),
                    help: Some("Each case becomes an output pin.".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "default".to_string(),
                    label: "Default Pin".to_string(),
                    field_type: NodeFieldType::Text,
                    default_value: Some(serde_json::json!("default")),
                    ..Default::default()
                },
            ]
        },
        layout: vec![
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("title".to_string()),
                    LayoutItem::Field("default".to_string()),
                ],
            },
            LayoutItem::Field("expression".to_string()),
            LayoutItem::Field("cases".to_string()),
        ],
        ai_tool: Default::default(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub expression: String,
    #[serde(default)]
    pub cases: Vec<String>,
    #[serde(default = "default_case")]
    pub default: String,
}

fn default_case() -> String {
    "default".to_string()
}

pub struct Node {
    node_id: String,
    config: Config,
    compiled: CompiledProgram,
    language: std::sync::Arc<dyn LanguageEngine>,
}

impl Node {
    pub fn new(
        node_id: &str,
        config: Config,
        language: std::sync::Arc<dyn LanguageEngine>,
    ) -> Result<Self, PipelineError> {
        let source = format!(
            "var $input = input.$input;\n\
             var $item = input.$item;\n\
             var $index = input.$index;\n\
             var $count = input.$count;\n\
             var $trigger = input.$trigger || null;\n\
             var $nodes = input.$nodes || {{}};\n\
             return String({});",
            config.expression
        );
        let module = ModuleSource {
            id: format!("logic.match:{node_id}"),
            source_path: None,
            kind: SourceKind::Tsx,
            code: source,
        };
        let ir = language.parse(&module).map_err(|e| {
            PipelineError::new(
                "FW_NODE_LOGIC_MATCH_PARSE",
                format!("node '{}': {}", node_id, e),
            )
        })?;
        let compiled = language
            .compile(
                &ir,
                &CompileOptions {
                    target: COMPILE_TARGET_BACKEND.to_string(),
                    optimize_level: 1,
                    emit_trace_hints: false,
                },
            )
            .map_err(|e| {
                PipelineError::new(
                    "FW_NODE_LOGIC_MATCH_COMPILE",
                    format!("node '{}': {}", node_id, e),
                )
            })?;
        Ok(Self {
            node_id: node_id.to_string(),
            config,
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
        &[]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let out = self
            .language
            .run(
                &self.compiled,
                build_expression_scope_input(&input.payload, &input.metadata),
                &crate::language::ExecutionContext {
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
                },
            )
            .map_err(|e| {
                PipelineError::new(
                    "FW_NODE_LOGIC_MATCH_RUN",
                    format!("node '{}': {}", self.node_id, e),
                )
            })?;

        let value = out.value.as_str().unwrap_or("").to_string();
        let pin = if self.config.cases.contains(&value) {
            value.clone()
        } else {
            self.config.default.clone()
        };

        Ok(NodeExecutionOutput {
            output_pins: vec![pin.clone()],
            payload: input.payload,
            trace: vec![format!("node_kind={NODE_KIND}"), format!("matched={pin}")],
        })
    }
}

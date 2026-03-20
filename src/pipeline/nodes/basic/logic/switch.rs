//! `n.logic.switch` — multi-case routing node.
//!
//! Evaluates a JS expression to get a string value.
//! Routes to the matching case pin, or the default pin if no case matches.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::pipeline::{
    PipelineError, NodeDefinition,
    nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput},
};
use crate::pipeline::model::LayoutItem;
use crate::language::{
    COMPILE_TARGET_BACKEND, CompileOptions, CompiledProgram, LanguageEngine, ModuleSource,
    SourceKind,
};

pub const NODE_KIND: &str = "n.logic.switch";
pub const INPUT_PIN_IN: &str = "in";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Switch".to_string(),
        description: "Evaluates expression to a string and routes to matching case pin, or default.".to_string(),
        input_schema: serde_json::json!({ "type": "object" }),
        output_schema: serde_json::json!({ "type": "object" }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![], // dynamic — defined per instance in the graph
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: Default::default(),
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType};
            vec![
                NodeFieldDef { name: "title".to_string(), label: "Title".to_string(), field_type: NodeFieldType::Text, help: Some("Override display title for this node.".to_string()), ..Default::default() },
                NodeFieldDef { name: "expression".to_string(), label: "Expression".to_string(), field_type: NodeFieldType::Textarea, rows: Some(4), ..Default::default() },
                NodeFieldDef { name: "cases".to_string(), label: "Cases (one per line)".to_string(), field_type: NodeFieldType::Textarea, rows: Some(5), help: Some("Each case becomes an output pin.".to_string()), ..Default::default() },
                NodeFieldDef { name: "default".to_string(), label: "Default Pin".to_string(), field_type: NodeFieldType::Text, default_value: Some(serde_json::json!("default")), ..Default::default() },
            ]
        },
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("title".to_string()), LayoutItem::Field("default".to_string())] },
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

fn default_case() -> String { "default".to_string() }

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
        let source = format!("return String({});", config.expression);
        let module = ModuleSource {
            id: format!("logic.switch:{node_id}"),
            source_path: None,
            kind: SourceKind::Tsx,
            code: source,
        };
        let ir = language.parse(&module).map_err(|e| {
            PipelineError::new("FW_NODE_LOGIC_SWITCH_PARSE", format!("node '{}': {}", node_id, e))
        })?;
        let compiled = language
            .compile(&ir, &CompileOptions {
                target: COMPILE_TARGET_BACKEND.to_string(),
                optimize_level: 1,
                emit_trace_hints: false,
            })
            .map_err(|e| {
                PipelineError::new("FW_NODE_LOGIC_SWITCH_COMPILE", format!("node '{}': {}", node_id, e))
            })?;
        Ok(Self { node_id: node_id.to_string(), config, compiled, language })
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str { NODE_KIND }
    fn input_pins(&self) -> &'static [&'static str] { &[INPUT_PIN_IN] }
    fn output_pins(&self) -> &'static [&'static str] { &[] }

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        let out = self.language.run(&self.compiled, input.payload.clone(), &crate::language::ExecutionContext { project: String::new(), pipeline: String::new(), request_id: String::new(), metadata: serde_json::Value::Null })
            .map_err(|e| PipelineError::new("FW_NODE_LOGIC_SWITCH_RUN", format!("node '{}': {}", self.node_id, e)))?;

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

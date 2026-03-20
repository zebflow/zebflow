//! `n.logic.if` — binary branch node.
//!
//! Evaluates a JS expression against the input payload.
//! Emits to the `true` pin when truthy, `false` pin otherwise.

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

pub const NODE_KIND: &str = "n.logic.if";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_TRUE: &str = "true";
pub const OUTPUT_PIN_FALSE: &str = "false";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "If".to_string(),
        description: "Evaluates a JS expression. Routes to `true` pin when truthy, `false` otherwise.".to_string(),
        input_schema: serde_json::json!({ "type": "object" }),
        output_schema: serde_json::json!({ "type": "object" }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_TRUE.to_string(), OUTPUT_PIN_FALSE.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: Default::default(),
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType};
            vec![
                NodeFieldDef { name: "title".to_string(), label: "Title".to_string(), field_type: NodeFieldType::Text, help: Some("Override display title for this node.".to_string()), ..Default::default() },
                NodeFieldDef { name: "expression".to_string(), label: "Condition".to_string(), field_type: NodeFieldType::Textarea, rows: Some(5), help: Some("JS expression returning truthy/falsy. Routes to 'true'/'false' pin.".to_string()), ..Default::default() },
            ]
        },
        layout: vec![
            LayoutItem::Field("title".to_string()),
            LayoutItem::Field("expression".to_string()),
        ],
        ai_tool: Default::default(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub expression: String,
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
        let source = format!("return Boolean({});", config.expression);
        let module = ModuleSource {
            id: format!("logic.if:{node_id}"),
            source_path: None,
            kind: SourceKind::Tsx,
            code: source,
        };
        let ir = language.parse(&module).map_err(|e| {
            PipelineError::new("FW_NODE_LOGIC_IF_PARSE", format!("node '{}': {}", node_id, e))
        })?;
        let compiled = language
            .compile(&ir, &CompileOptions {
                target: COMPILE_TARGET_BACKEND.to_string(),
                optimize_level: 1,
                emit_trace_hints: false,
            })
            .map_err(|e| {
                PipelineError::new("FW_NODE_LOGIC_IF_COMPILE", format!("node '{}': {}", node_id, e))
            })?;
        Ok(Self { node_id: node_id.to_string(), compiled, language })
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str { NODE_KIND }
    fn input_pins(&self) -> &'static [&'static str] { &[INPUT_PIN_IN] }
    fn output_pins(&self) -> &'static [&'static str] { &[OUTPUT_PIN_TRUE, OUTPUT_PIN_FALSE] }

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        let out = self.language.run(&self.compiled, input.payload.clone(), &crate::language::ExecutionContext { project: String::new(), pipeline: String::new(), request_id: String::new(), metadata: serde_json::Value::Null })
            .map_err(|e| PipelineError::new("FW_NODE_LOGIC_IF_RUN", format!("node '{}': {}", self.node_id, e)))?;

        let is_true = match &out.value {
            serde_json::Value::Bool(b) => *b,
            serde_json::Value::Null => false,
            serde_json::Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
            serde_json::Value::String(s) => !s.is_empty(),
            _ => true,
        };

        let pin = if is_true { OUTPUT_PIN_TRUE } else { OUTPUT_PIN_FALSE };
        Ok(NodeExecutionOutput {
            output_pins: vec![pin.to_string()],
            payload: input.payload,
            trace: vec![format!("node_kind={NODE_KIND}"), format!("result={pin}")],
        })
    }
}

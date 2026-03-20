//! `n.logic.branch` — fan-out or expression-based routing node.
//!
//! mode=fanout:         emits to ALL output pins simultaneously (parallel fan-out).
//! mode=by_expression:  evaluates expression to a string, emits to that pin.

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

pub const NODE_KIND: &str = "n.logic.branch";
pub const INPUT_PIN_IN: &str = "in";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Branch".to_string(),
        description: "Fan-out to all pins (mode=fanout) or route to one pin by expression (mode=by_expression).".to_string(),
        input_schema: serde_json::json!({ "type": "object" }),
        output_schema: serde_json::json!({ "type": "object" }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![], // dynamic — defined per instance in the graph
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: Default::default(),
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType, SelectOptionDef};
            vec![
                NodeFieldDef { name: "title".to_string(), label: "Title".to_string(), field_type: NodeFieldType::Text, help: Some("Override display title for this node.".to_string()), ..Default::default() },
                NodeFieldDef { name: "mode".to_string(), label: "Mode".to_string(), field_type: NodeFieldType::Select, options: vec![
                    SelectOptionDef { value: "expression".to_string(), label: "Expression — route to one branch".to_string() },
                    SelectOptionDef { value: "parallel".to_string(), label: "Parallel — route to all branches".to_string() },
                ], ..Default::default() },
                NodeFieldDef { name: "branches".to_string(), label: "Branches (one per line)".to_string(), field_type: NodeFieldType::Textarea, rows: Some(5), ..Default::default() },
                NodeFieldDef { name: "expression".to_string(), label: "Expression".to_string(), field_type: NodeFieldType::Textarea, rows: Some(4), ..Default::default() },
            ]
        },
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("title".to_string()), LayoutItem::Field("mode".to_string())] },
            LayoutItem::Field("expression".to_string()),
            LayoutItem::Field("branches".to_string()),
        ],
        ai_tool: Default::default(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BranchMode {
    Fanout,
    ByExpression,
}

impl Default for BranchMode {
    fn default() -> Self { Self::Fanout }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub mode: BranchMode,
    #[serde(default)]
    pub branches: Vec<String>,
    pub expression: Option<String>,
}

pub struct Node {
    node_id: String,
    config: Config,
    compiled: Option<CompiledProgram>,
    language: std::sync::Arc<dyn LanguageEngine>,
}

impl Node {
    pub fn new(
        node_id: &str,
        config: Config,
        language: std::sync::Arc<dyn LanguageEngine>,
    ) -> Result<Self, PipelineError> {
        let compiled = if config.mode == BranchMode::ByExpression {
            let expr = config.expression.as_deref().unwrap_or("''");
            let source = format!("return String({});", expr);
            let module = ModuleSource {
                id: format!("logic.branch:{node_id}"),
                source_path: None,
                kind: SourceKind::Tsx,
                code: source,
            };
            let ir = language.parse(&module).map_err(|e| {
                PipelineError::new("FW_NODE_LOGIC_BRANCH_PARSE", format!("node '{}': {}", node_id, e))
            })?;
            Some(language.compile(&ir, &CompileOptions {
                target: COMPILE_TARGET_BACKEND.to_string(),
                optimize_level: 1,
                emit_trace_hints: false,
            }).map_err(|e| {
                PipelineError::new("FW_NODE_LOGIC_BRANCH_COMPILE", format!("node '{}': {}", node_id, e))
            })?)
        } else {
            None
        };

        Ok(Self { node_id: node_id.to_string(), config, compiled, language })
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str { NODE_KIND }
    fn input_pins(&self) -> &'static [&'static str] { &[INPUT_PIN_IN] }
    fn output_pins(&self) -> &'static [&'static str] { &[] }

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        let output_pins = match self.config.mode {
            BranchMode::Fanout => self.config.branches.clone(),
            BranchMode::ByExpression => {
                let compiled = self.compiled.as_ref().ok_or_else(|| {
                    PipelineError::new("FW_NODE_LOGIC_BRANCH_NO_COMPILED", "by_expression mode requires expression")
                })?;
                let out = self.language.run(compiled, input.payload.clone(), &crate::language::ExecutionContext { project: String::new(), pipeline: String::new(), request_id: String::new(), metadata: serde_json::Value::Null })
                    .map_err(|e| PipelineError::new("FW_NODE_LOGIC_BRANCH_RUN", format!("node '{}': {}", self.node_id, e)))?;
                let pin = out.value.as_str().unwrap_or("").to_string();
                vec![pin]
            }
        };

        Ok(NodeExecutionOutput {
            output_pins: output_pins.clone(),
            payload: input.payload,
            trace: vec![
                format!("node_kind={NODE_KIND}"),
                format!("mode={:?}", self.config.mode),
                format!("pins={}", output_pins.join(",")),
            ],
        })
    }
}

//! `n.logic.reduce` — ordered accumulation over a foreach-emitted series.
//!
//! The node evaluates:
//!
//! - `init_expr` once to create the initial accumulator
//! - `step_expr` for each arrival to produce the next accumulator
//!
//! Scope inside both expressions:
//!
//! - `$input` = current arriving payload
//! - `$acc`   = current accumulator (`null` for init)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::language::{
    COMPILE_TARGET_BACKEND, CompileOptions, CompiledProgram, LanguageEngine, ModuleSource,
    SourceKind,
};
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "n.logic.reduce";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Reduce".to_string(),
        description: "Accumulates an ordered emitted series into one final result.".to_string(),
        input_schema: serde_json::json!({ "type": "object" }),
        output_schema: serde_json::json!({ "type": "object" }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag {
                flag: "--init-expr".to_string(),
                config_key: "init_expr".to_string(),
                description: "Expression producing the initial accumulator.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--step-expr".to_string(),
                config_key: "step_expr".to_string(),
                description: "Expression producing the next accumulator from $acc and $input."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType};
            vec![
                NodeFieldDef {
                    name: "init_expr".to_string(),
                    label: "Init Expression".to_string(),
                    field_type: NodeFieldType::Textarea,
                    rows: Some(3),
                    help: Some("Expression that creates the initial accumulator before processing inputs.".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "step_expr".to_string(),
                    label: "Step Expression".to_string(),
                    field_type: NodeFieldType::Textarea,
                    rows: Some(5),
                    help: Some("Expression that returns the next accumulator from $acc and the current $input.".to_string()),
                    ..Default::default()
                },
            ]
        },
        layout: vec![
            LayoutItem::Field("init_expr".to_string()),
            LayoutItem::Field("step_expr".to_string()),
        ],
        ai_tool: Default::default(),
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub init_expr: String,
    pub step_expr: String,
}

pub struct Node {
    node_id: String,
    init_compiled: CompiledProgram,
    step_compiled: CompiledProgram,
    language: std::sync::Arc<dyn LanguageEngine>,
}

impl Node {
    pub fn new(
        node_id: &str,
        config: Config,
        language: std::sync::Arc<dyn LanguageEngine>,
    ) -> Result<Self, PipelineError> {
        let init_compiled = compile_expr(
            &language,
            &format!("logic.reduce:init:{node_id}"),
            &config.init_expr,
        )?;
        let step_compiled = compile_expr(
            &language,
            &format!("logic.reduce:step:{node_id}"),
            &config.step_expr,
        )?;
        Ok(Self {
            node_id: node_id.to_string(),
            init_compiled,
            step_compiled,
            language,
        })
    }
}

fn compile_expr(
    language: &std::sync::Arc<dyn LanguageEngine>,
    id: &str,
    expr: &str,
) -> Result<CompiledProgram, PipelineError> {
    let source = format!(
        "const $input = input.$input;\nconst $acc = input.$acc;\nreturn ({});",
        expr
    );
    let module = ModuleSource {
        id: id.to_string(),
        source_path: None,
        kind: SourceKind::Tsx,
        code: source,
    };
    let ir = language.parse(&module).map_err(|e| {
        PipelineError::new("FW_NODE_LOGIC_REDUCE_PARSE", format!("module '{id}': {e}"))
    })?;
    language
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
                "FW_NODE_LOGIC_REDUCE_COMPILE",
                format!("module '{id}': {e}"),
            )
        })
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
        let acc = input
            .metadata
            .get("reduce_acc")
            .cloned()
            .unwrap_or(Value::Null);
        let exec_ctx = crate::language::ExecutionContext {
            project: input
                .metadata
                .get("project")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            pipeline: input
                .metadata
                .get("pipeline")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            request_id: input
                .metadata
                .get("request_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            trigger: input
                .metadata
                .get("trigger")
                .cloned()
                .unwrap_or(Value::Null),
            metadata: input.metadata.clone(),
        };

        let out = if acc.is_null() {
            let init_out = self
                .language
                .run(
                    &self.init_compiled,
                    json!({
                        "$input": input.payload.clone(),
                        "$acc": Value::Null,
                    }),
                    &exec_ctx,
                )
                .map_err(|e| {
                    PipelineError::new(
                        "FW_NODE_LOGIC_REDUCE_RUN",
                        format!("node '{}': {}", self.node_id, e),
                    )
                })?;
            self.language
                .run(
                    &self.step_compiled,
                    json!({
                        "$input": input.payload,
                        "$acc": init_out.value,
                    }),
                    &exec_ctx,
                )
                .map_err(|e| {
                    PipelineError::new(
                        "FW_NODE_LOGIC_REDUCE_RUN",
                        format!("node '{}': {}", self.node_id, e),
                    )
                })?
        } else {
            self.language
                .run(
                    &self.step_compiled,
                    json!({
                        "$input": input.payload,
                        "$acc": acc,
                    }),
                    &exec_ctx,
                )
                .map_err(|e| {
                    PipelineError::new(
                        "FW_NODE_LOGIC_REDUCE_RUN",
                        format!("node '{}': {}", self.node_id, e),
                    )
                })?
        };

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: out.value,
            trace: vec![
                format!("node_kind={NODE_KIND}"),
                format!("phase={}", if acc.is_null() { "init+step" } else { "step" }),
            ],
        })
    }
}

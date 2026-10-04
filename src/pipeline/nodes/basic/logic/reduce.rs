//! `logic.reduce` — ordered accumulation over a foreach-emitted series.
//!
//! The node evaluates:
//!
//! - `--initial` once to create the initial accumulator
//! - `--step` for each arrival to produce the next accumulator
//!
//! Scope inside both expressions:
//!
//! - `$input` = current arriving payload
//! - `$acc`   = current accumulator (`null` for init)
//!
//! It answers one key, `reduce`: the accumulator, added to the arriving
//! payload. The engine carries the accumulator between arrivals by reading
//! that key back ([`accumulator`]).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::language::{
    COMPILE_TARGET_BACKEND, CompileOptions, CompiledProgram, LanguageEngine, ModuleSource,
    SourceKind,
};
use crate::pipeline::model::NodeCapability;
use crate::pipeline::nodes::shared::util::with_answer;
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "logic.reduce";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Process],
        title: "Reduce".to_string(),
        description: "Folds the per-item answers of a `logic.foreach` into one value. Placed after `logic.foreach` (directly or further down the branch), it runs `--initial` once and \
             `--step` for every emission, with `$acc` the accumulator so far and `$input` the arriving payload, then fires \
             `out` once when the series is complete. Expressions are JavaScript values without `{{ }}`. \
             Answers one key, `reduce`: the final `$acc` (`input.reduce.total`)."
            .to_string(),
        input_schema: serde_json::json!({ "type": "object" }),
        output_schema: serde_json::json!({ "type": "object" }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag {
                flag: "--initial".to_string(),
                config_key: "initial".to_string(),
                description: "The initial accumulator: a JavaScript value (`{ total: 0 }`).".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
                value: "expression".to_string(),
                ..Default::default()
            },
            DslFlag {
                flag: "--step".to_string(),
                config_key: "step".to_string(),
                description: "The next accumulator from `$acc` and `$input`: a JavaScript value."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
                value: "expression".to_string(),
                ..Default::default()
            },
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType};
            vec![
                NodeFieldDef {
                    name: "initial".to_string(),
                    label: "Initial".to_string(),
                    field_type: NodeFieldType::Textarea,
                    rows: Some(3),
                    help: Some("Expression that creates the initial accumulator before processing inputs.".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "step".to_string(),
                    label: "Step".to_string(),
                    field_type: NodeFieldType::Textarea,
                    rows: Some(5),
                    help: Some("Expression that returns the next accumulator from $acc and the current $input.".to_string()),
                    ..Default::default()
                },
            ]
        },
        layout: vec![
            LayoutItem::Field("initial".to_string()),
            LayoutItem::Field("step".to_string()),
        ],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Sum a column", r#"logic.reduce --initial "{ total: 0, n: 0 }" --step "{ total: $acc.total + $input.item.amount, n: $acc.n + 1 }""#)
                .output(serde_json::json!({ "item": { "amount": 12.5 }, "index": 2, "count": 3, "reduce": { "total": 42.5, "n": 3 } }))
                .note("After `logic.foreach --from \"input.query.rows\"` over three rows: the last arrival, with the sum under `reduce`."),
        ],
        ..Default::default()
    }
}

/// The key `logic.reduce` answers under.
pub const ANSWER_KEY: &str = "reduce";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// `--initial`: the first accumulator.
    #[serde(deserialize_with = "super::expression_text")]
    pub initial: String,
    /// `--step`: the next accumulator from `$acc` and `$input`.
    #[serde(deserialize_with = "super::expression_text")]
    pub step: String,
}

/// The accumulator a reduce answered: what the engine hands back as `$acc`
/// on the next arrival.
pub fn accumulator(payload: &Value) -> Value {
    payload.get(ANSWER_KEY).cloned().unwrap_or(Value::Null)
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
        let initial = super::required_expression(&config.initial, "--initial", "FW_NODE_LOGIC_REDUCE_CONFIG")?;
        let step = super::required_expression(&config.step, "--step", "FW_NODE_LOGIC_REDUCE_CONFIG")?;
        let init_compiled = compile_expr(&language, &format!("logic.reduce:initial:{node_id}"), initial)?;
        let step_compiled = compile_expr(&language, &format!("logic.reduce:step:{node_id}"), step)?;
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

        let arriving = input.payload.clone();
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
            payload: with_answer(&arriving, json!({ ANSWER_KEY: out.value })),
            trace: vec![
                format!("node_kind={NODE_KIND}"),
                format!("phase={}", if acc.is_null() { "init+step" } else { "step" }),
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(initial: &str, step: &str) -> Result<Node, PipelineError> {
        Node::new(
            "r",
            Config { initial: initial.to_string(), step: step.to_string() },
            std::sync::Arc::new(crate::language::DenoSandboxEngine::default()),
        )
    }

    fn run(node: &Node, payload: Value, metadata: Value) -> NodeExecutionOutput {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(node.execute_async(NodeExecutionInput {
                node_id: "r".to_string(),
                input_pin: INPUT_PIN_IN.to_string(),
                payload,
                metadata,
                bus: None,
            }))
            .expect("run")
    }

    /// The flags are `--initial` and `--step`, both expressions.
    #[test]
    fn the_flags_are_initial_and_step() {
        let def = definition();
        let flags: Vec<(&str, &str)> = def.dsl_flags.iter().map(|f| (f.flag.as_str(), f.value.as_str())).collect();
        assert_eq!(flags, [("--initial", "expression"), ("--step", "expression")]);
    }

    /// The accumulator is answered under `reduce`, the arriving payload kept,
    /// and read back as the next `$acc`.
    #[test]
    fn the_accumulator_is_answered_under_reduce() {
        let node = node("{ total: 0 }", "{ total: $acc.total + $input.item }").expect("node");
        let first = run(&node, json!({ "item": 10, "index": 0, "count": 2 }), json!({}));
        assert_eq!(first.payload["reduce"], json!({ "total": 10 }));
        assert_eq!(first.payload["item"], 10, "the arriving payload is kept");
        let acc = accumulator(&first.payload);
        let second = run(&node, json!({ "item": 5, "index": 1, "count": 2 }), json!({ "reduce_acc": acc }));
        assert_eq!(second.payload["reduce"], json!({ "total": 15 }));
    }

    /// Empty is not a value: either expression empty is refused at build.
    #[test]
    fn an_empty_expression_is_refused() {
        for (initial, step, flag) in [("", "$acc", "--initial"), ("0", " ", "--step")] {
            let err = node(initial, step).err().expect("refused");
            assert_eq!(err.code, "FW_NODE_LOGIC_REDUCE_CONFIG");
            assert!(err.message.contains(flag), "{}", err.message);
        }
    }
}

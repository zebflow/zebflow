//! `logic.reduce` — ordered accumulation over a foreach-emitted series.
//!
//! The node evaluates:
//!
//! - `--initial` once to create the initial accumulator
//! - `--step` for each delivered item to produce the next accumulator
//!
//! Scope inside both expressions:
//!
//! - `$input` = the item (for `--initial`: the payload the loop started from)
//! - `$acc`   = current accumulator (`null` for init)
//!
//! It runs once, when the loop it closes has run every item
//! (`node-conventions.md` §4, Loops): the engine hands it the delivered items
//! under [`super::LOOP_ITEMS_METADATA_KEY`] and the payload the loop started
//! from. It answers one key, `reduce`: the folded accumulator, added to that
//! payload. An empty or all-skipped series answers `--initial`.

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
        description: "Folds the per-item answers of a `logic.foreach` into one value. It closes the loop: placed after `logic.foreach` (directly or further down the item branch), it runs once, after every item has run — `--initial` once, then \
             `--step` for every item that reached it, in item order, with `$acc` the accumulator so far and `$input` that item's payload (an item whose branch was skipped is left out). \
             An empty list, or a series where every item was skipped, answers `--initial`. Expressions are JavaScript values without `{{ }}`. \
             Answers one key, `reduce`: the final `$acc` (`input.reduce.total`), on top of the payload the foreach received; the loop's own nodes are not in `$nodes` after it."
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
                .output(serde_json::json!({ "query": { "rows": [{ "amount": 10 }, { "amount": 20 }, { "amount": 12.5 }] }, "reduce": { "total": 42.5, "n": 3 } }))
                .note("After `logic.foreach --from \"input.query.rows\"` over three rows: the payload the loop started from, with the sum under `reduce`."),
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

impl Node {
    fn run(
        &self,
        program: &CompiledProgram,
        input: Value,
        acc: Value,
        ctx: &crate::language::ExecutionContext,
    ) -> Result<Value, PipelineError> {
        self.language
            .run(program, json!({ "$input": input, "$acc": acc }), ctx)
            .map(|out| out.value)
            .map_err(|e| PipelineError::new("FW_NODE_LOGIC_REDUCE_RUN", format!("node '{}': {}", self.node_id, e)))
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

    /// One run folds the whole series: `--initial` once (with `$input` the
    /// payload the loop started from), then `--step` per delivered item in
    /// item order. Outside a loop the payload is the one item.
    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let meta = |key: &str| input.metadata.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
        let exec_ctx = crate::language::ExecutionContext {
            project: meta("project"),
            pipeline: meta("pipeline"),
            request_id: meta("request_id"),
            trigger: input.metadata.get("trigger").cloned().unwrap_or(Value::Null),
            metadata: input.metadata.clone(),
        };
        let items = match input.metadata.get(super::LOOP_ITEMS_METADATA_KEY) {
            Some(Value::Array(items)) => items.clone(),
            _ => vec![input.payload.clone()],
        };
        let mut acc = self.run(&self.init_compiled, input.payload.clone(), Value::Null, &exec_ctx)?;
        for item in &items {
            acc = self.run(&self.step_compiled, item.clone(), acc, &exec_ctx)?;
        }
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: with_answer(&input.payload, json!({ ANSWER_KEY: acc })),
            trace: vec![format!("node_kind={NODE_KIND}"), format!("items={}", items.len())],
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

    /// One run folds every item the loop delivered, in order, over the
    /// payload the loop started from; outside a loop the payload is one item.
    #[test]
    fn the_series_is_folded_once_under_reduce() {
        let node = node("{ total: 0, from: $input.start }", "{ total: $acc.total + $input.item, from: $acc.from }").expect("node");
        let items = json!({ "loop_items": [{ "item": 10 }, { "item": 5 }] });
        let out = run(&node, json!({ "start": "s" }), items);
        assert_eq!(out.payload["reduce"], json!({ "total": 15, "from": "s" }));
        assert_eq!(out.payload["start"], "s", "the loop's own payload is kept");

        let empty = run(&node, json!({ "start": "s" }), json!({ "loop_items": [] }));
        assert_eq!(empty.payload["reduce"], json!({ "total": 0, "from": "s" }), "an empty series answers --initial");

        let alone = run(&node, json!({ "item": 3, "start": "x" }), json!({}));
        assert_eq!(alone.payload["reduce"], json!({ "total": 3, "from": "x" }));
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

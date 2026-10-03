//! `logic.foreach` — explicit ordered multi-emission node.
//!
//! Evaluates `--from` to an array and emits one downstream
//! run per item on the `item` pin. Emitted payloads are item-only by default:
//!
//! - `item`
//! - `index`
//! - `count`
//!
//! This deliberately avoids cloning a large parent payload into every emitted
//! item. Use `--keep-input` only when the downstream node truly needs the full
//! upstream payload alongside each item.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::language::{
    COMPILE_TARGET_BACKEND, CompileOptions, CompiledProgram, LanguageEngine, ModuleSource,
    SourceKind,
};
use crate::pipeline::expr::build_expression_scope_input;
use crate::pipeline::model::NodeCapability;
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "logic.foreach";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_ITEM: &str = "item";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Process],
        title: "Foreach".to_string(),
        description: "Loop. Evaluates `--from` (JavaScript over `input`) to an array and runs everything wired to its `item` pin once per element, in order. \
             Each run receives `{ item, index, count }` — the element is `input.item`, not `input`; the upstream payload is \
             not carried unless `--keep-input` (then it is merged in beside `item`). `$item`, `$index` and `$count` name the \
             run's element anywhere down the branch, even after a node replaced the payload. Emissions are sequential; to \
             fold the results back into one value, end the branch in `logic.reduce`. `--batch-size N` emits arrays of up to N elements \
             instead of single elements. A non-array expression fails the node."
            .to_string(),
        input_schema: serde_json::json!({ "type": "object" }),
        output_schema: serde_json::json!({ "type": "object" }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_ITEM.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag {
                flag: "--from".to_string(),
                config_key: "from".to_string(),
                description: "The list: JavaScript over `input` (and `$trigger`, `$nodes`) returning the array to emit.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
                value: "expression".to_string(),
                ..Default::default()
            },
            DslFlag {
                flag: "--dispatch".to_string(),
                config_key: "dispatch".to_string(),
                description: "How the emissions run: `sequential` (the default, and the only policy today)."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                choices: vec![DISPATCH_SEQUENTIAL.to_string()],
                ..Default::default()
            },
            DslFlag {
                flag: "--batch-size".to_string(),
                config_key: "batch_size".to_string(),
                description: "Emit arrays of up to N elements instead of single elements (at least 1)."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                value: "number".to_string(),
                ..Default::default()
            },
            DslFlag {
                flag: "--keep-input".to_string(),
                config_key: "keep_input".to_string(),
                description: "Include the full upstream payload in every emitted item. Off by default to avoid fan-out amplification.".to_string(),
                kind: DslFlagKind::Bool,
                required: false,
                ..Default::default()
            },
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType, SelectOptionDef};
            vec![
                NodeFieldDef {
                    name: "from".to_string(),
                    label: "From".to_string(),
                    field_type: NodeFieldType::Text,
                    placeholder: Some("input.query.rows".to_string()),
                    help: Some("JS expression returning the array to emit.".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "dispatch".to_string(),
                    label: "Dispatch".to_string(),
                    field_type: NodeFieldType::Select,
                    options: vec![SelectOptionDef {
                        value: DISPATCH_SEQUENTIAL.to_string(),
                        label: "Sequential".to_string(),
                    }],
                    default_value: Some(json!(DISPATCH_SEQUENTIAL)),
                    help: Some("Dispatch policy for emitted items. Current runtime supports sequential dispatch.".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "batch_size".to_string(),
                    label: "Batch size".to_string(),
                    field_type: NodeFieldType::Text,
                    help: Some("Optional: emit arrays of up to this many elements instead of single elements.".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "keep_input".to_string(),
                    label: "Keep Input".to_string(),
                    field_type: NodeFieldType::Checkbox,
                    help: Some("When enabled, each emitted payload includes the full upstream input. Leave off for large payloads.".to_string()),
                    default_value: Some(json!(false)),
                    ..Default::default()
                },
            ]
        },
        layout: vec![
            LayoutItem::Field("dispatch".to_string()),
            LayoutItem::Field("from".to_string()),
            LayoutItem::Field("batch_size".to_string()),
            LayoutItem::Field("keep_input".to_string()),
        ],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("One run per row", r#"logic.foreach --from "input.query.rows""#)
                .input(serde_json::json!({ "query": { "rows": [{ "email": "a@example.com" }, { "email": "b@example.com" }] } }))
                .output(serde_json::json!({ "item": { "email": "a@example.com" }, "index": 0, "count": 2 }))
                .note("The first of two emissions on the `item` pin; the next node reads `input.item.email`."),
        ],
        ..Default::default()
    }
}

/// The one dispatch policy the runtime has.
pub const DISPATCH_SEQUENTIAL: &str = "sequential";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// `--from`: the list, JavaScript over `input`.
    #[serde(deserialize_with = "super::expression_text")]
    pub from: String,
    #[serde(default)]
    pub dispatch: String,
    /// `--batch-size`: a whole number of 1 or more; the dialog saves it as text.
    #[serde(default)]
    pub batch_size: Value,
    #[serde(default)]
    pub keep_input: bool,
}

const CONFIG_CODE: &str = "FW_NODE_LOGIC_FOREACH_CONFIG";

/// `--batch-size` read once at build: unset is `None`; anything but a whole
/// number of 1 or more is refused.
fn batch_size(value: &Value) -> Result<Option<usize>, PipelineError> {
    let refuse = |shown: String| PipelineError::new(CONFIG_CODE, format!("--batch-size {shown} must be a whole number of 1 or more"));
    let n = match value {
        Value::Null => return Ok(None),
        Value::String(text) if text.trim().is_empty() => return Ok(None),
        Value::String(text) => text.trim().parse::<usize>().map_err(|_| refuse(format!("'{}'", text.trim())))?,
        Value::Number(n) => n.as_u64().map(|n| n as usize).ok_or_else(|| refuse(n.to_string()))?,
        other => return Err(refuse(other.to_string())),
    };
    if n == 0 {
        return Err(refuse("0".to_string()));
    }
    Ok(Some(n))
}

pub struct Node {
    config: Config,
    dispatch: &'static str,
    batch_size: Option<usize>,
    compiled_from: CompiledProgram,
    language: std::sync::Arc<dyn LanguageEngine>,
}

impl Node {
    pub fn new(
        config: Config,
        language: std::sync::Arc<dyn LanguageEngine>,
    ) -> Result<Self, PipelineError> {
        let dispatch = crate::pipeline::nodes::shared::limits::choice(
            &config.dispatch,
            &[DISPATCH_SEQUENTIAL],
            DISPATCH_SEQUENTIAL,
            "--dispatch",
            CONFIG_CODE,
        )?;
        let batch_size = batch_size(&config.batch_size)?;
        let from = super::required_expression(&config.from, "--from", CONFIG_CODE)?;
        let compiled_from = compile_from(language.as_ref(), from)?;
        Ok(Self {
            config,
            dispatch,
            batch_size,
            compiled_from,
            language,
        })
    }
}

fn compile_from(
    language: &dyn LanguageEngine,
    expr: &str,
) -> Result<CompiledProgram, PipelineError> {
    let source = format!(
        // `input` is the payload here, as it is in every `{{ }}` block, in
            // javascript.script.run, and in every document that teaches either. Binding only
            // `$input` left `input` pointing at the scope object, so
            // `input.rows` silently evaluated to undefined and a guard took the
            // wrong branch with no error anywhere. Same defect, same fix as
            // `expr/resolver.rs`.
            "var __scope = input;\n\
             var input = __scope.$input;\n\
             var $input = input;\n\
         var $item = __scope.$item;\n\
         var $index = __scope.$index;\n\
         var $count = __scope.$count;\n\
         var $trigger = __scope.$trigger || null;\n\
         var $nodes = __scope.$nodes || {{}};\n\
         return ({expr});"
    );
    let module = ModuleSource {
        id: format!("pipeline:logic.foreach:from:{expr}"),
        source_path: None,
        kind: SourceKind::Tsx,
        code: source,
    };
    let ir = language
        .parse(&module)
        .map_err(|err| PipelineError::new("FW_NODE_LOGIC_FOREACH_PARSE", err.to_string()))?;
    language
        .compile(
            &ir,
            &CompileOptions {
                target: COMPILE_TARGET_BACKEND.to_string(),
                optimize_level: 1,
                emit_trace_hints: false,
            },
        )
        .map_err(|err| PipelineError::new("FW_NODE_LOGIC_FOREACH_COMPILE", err.to_string()))
}

fn build_emission_payload(
    base: &Value,
    item: Value,
    index: usize,
    count: usize,
    keep_input: bool,
) -> Value {
    if !keep_input {
        return json!({
            "item": item,
            "index": index,
            "count": count,
        });
    }

    match base {
        Value::Object(map) => {
            let mut next = map.clone();
            next.insert("item".to_string(), item);
            next.insert("index".to_string(), json!(index));
            next.insert("count".to_string(), json!(count));
            Value::Object(next)
        }
        _ => json!({
            "input": base,
            "item": item,
            "index": index,
            "count": count,
        }),
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
        &[OUTPUT_PIN_ITEM]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let outputs = self.execute_many_async(input).await?;
        outputs.into_iter().next().ok_or_else(|| {
            PipelineError::new(
                "FW_NODE_LOGIC_FOREACH_EMPTY",
                "foreach produced no outputs; use execute_many_async",
            )
        })
    }

    async fn execute_many_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<Vec<NodeExecutionOutput>, PipelineError> {
        let ctx = crate::language::ExecutionContext {
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
        let source = self
            .language
            .run(
                &self.compiled_from,
                build_expression_scope_input(&input.payload, &input.metadata),
                &ctx,
            )
            .map_err(|err| PipelineError::new("FW_NODE_LOGIC_FOREACH_RUN", err.to_string()))?
            .value;
        let items = source.as_array().ok_or_else(|| {
            PipelineError::new(
                "FW_NODE_LOGIC_FOREACH_TYPE",
                format!("--from '{}' did not resolve to an array", self.config.from.trim()),
            )
        })?;

        let emitted_items: Vec<Value> = if let Some(batch_size) = self.batch_size {
            items
                .chunks(batch_size)
                .map(|chunk| Value::Array(chunk.to_vec()))
                .collect()
        } else {
            items.to_vec()
        };

        let count = emitted_items.len();
        let outputs = emitted_items
            .into_iter()
            .enumerate()
            .map(|(index, item)| NodeExecutionOutput {
                output_pins: vec![OUTPUT_PIN_ITEM.to_string()],
                payload: build_emission_payload(
                    &input.payload,
                    item,
                    index,
                    count,
                    self.config.keep_input,
                ),
                trace: vec![
                    format!("node_kind={NODE_KIND}"),
                    format!("dispatch={}", self.dispatch),
                    format!("keep_input={}", self.config.keep_input),
                    format!("index={index}"),
                    format!("count={count}"),
                ],
            })
            .collect();

        Ok(outputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(config: Value) -> Result<Node, PipelineError> {
        let config: Config = serde_json::from_value(config).expect("config");
        Node::new(config, std::sync::Arc::new(crate::language::DenoSandboxEngine::default()))
    }

    fn run(node: &Node, payload: Value) -> Vec<NodeExecutionOutput> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(node.execute_many_async(NodeExecutionInput {
                node_id: "b".to_string(),
                input_pin: INPUT_PIN_IN.to_string(),
                payload,
                metadata: json!({}),
                bus: None,
            }))
            .expect("run")
    }

    /// `--from` is the list, `--batch-size` a number, `--dispatch` a closed choice.
    #[test]
    fn the_flags_are_from_batch_size_and_a_closed_dispatch() {
        let def = definition();
        let flag = |name: &str| def.dsl_flags.iter().find(|f| f.flag == name).expect(name);
        assert_eq!(flag("--from").value, "expression");
        assert!(flag("--from").required);
        assert_eq!(flag("--batch-size").value, "number");
        assert_eq!(flag("--dispatch").choices, vec![DISPATCH_SEQUENTIAL.to_string()]);
        assert!(def.dsl_flags.iter().all(|f| !f.flag.ends_with("-expr") && f.flag != "--chunk-size"));
    }

    /// One emission per element: `{ item, index, count }`, the upstream
    /// payload not carried.
    #[test]
    fn it_emits_one_item_per_element() {
        let outs = run(&node(json!({ "from": "input.rows" })).expect("node"), json!({ "rows": [1, 2, 3], "big": "x" }));
        assert_eq!(outs.len(), 3);
        assert_eq!(outs[1].payload, json!({ "item": 2, "index": 1, "count": 3 }));
    }

    /// `--batch-size 2` emits arrays of up to two elements; the dialog's text
    /// spelling reads the same.
    #[test]
    fn a_batch_size_emits_arrays() {
        for size in [json!(2), json!("2")] {
            let outs = run(&node(json!({ "from": "input.rows", "batch_size": size })).expect("node"), json!({ "rows": [1, 2, 3] }));
            let items: Vec<Value> = outs.iter().map(|o| o.payload["item"].clone()).collect();
            assert_eq!(items, vec![json!([1, 2]), json!([3])]);
        }
    }

    /// A batch of 0, an unknown dispatch word and an empty list expression
    /// are refused at build, each naming its flag.
    #[test]
    fn bad_settings_are_refused_at_build() {
        for (config, flag) in [
            (json!({ "from": "input.rows", "batch_size": 0 }), "--batch-size"),
            (json!({ "from": "input.rows", "batch_size": "many" }), "--batch-size"),
            (json!({ "from": "input.rows", "dispatch": "seq" }), "--dispatch"),
            (json!({ "from": " " }), "--from"),
        ] {
            let err = node(config).err().expect("refused");
            assert_eq!(err.code, CONFIG_CODE);
            assert!(err.message.contains(flag), "{}", err.message);
        }
        assert!(node(json!({ "from": "input.rows", "dispatch": "sequential" })).is_ok());
    }
}

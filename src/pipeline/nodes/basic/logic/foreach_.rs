//! `n.logic.foreach` — explicit ordered multi-emission node.
//!
//! Evaluates `items_expr` to an array and emits one downstream
//! run per item on the `item` pin. Emitted payloads preserve the original object payload and add:
//!
//! - `item`
//! - `index`
//! - `count`

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

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

pub const NODE_KIND: &str = "n.logic.foreach";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_ITEM: &str = "item";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Foreach".to_string(),
        description: "Emits one ordered downstream run per item from an input collection."
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
                flag: "--items-expr".to_string(),
                config_key: "items_expr".to_string(),
                description: "JS expression returning the array to emit.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--dispatch".to_string(),
                config_key: "dispatch".to_string(),
                description: "Dispatch policy. Current runtime supports sequential dispatch only."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--chunk-size".to_string(),
                config_key: "chunk_size".to_string(),
                description: "Optional chunk size. When set, each emitted item is an array chunk."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType, SelectOptionDef};
            vec![
                NodeFieldDef {
                    name: "title".to_string(),
                    label: "Title".to_string(),
                    field_type: NodeFieldType::Text,
                    help: Some("Override display title for this node.".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "items_expr".to_string(),
                    label: "Items".to_string(),
                    field_type: NodeFieldType::Text,
                    placeholder: Some("$input.rows".to_string()),
                    help: Some("JS expression returning the array to emit.".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "dispatch".to_string(),
                    label: "Dispatch".to_string(),
                    field_type: NodeFieldType::Select,
                    options: vec![SelectOptionDef {
                        value: "seq".to_string(),
                        label: "Sequential".to_string(),
                    }],
                    default_value: Some(json!("seq")),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "chunk_size".to_string(),
                    label: "Chunk Size".to_string(),
                    field_type: NodeFieldType::Text,
                    help: Some("Optional chunk size for large collections.".to_string()),
                    ..Default::default()
                },
            ]
        },
        layout: vec![
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("title".to_string()),
                    LayoutItem::Field("dispatch".to_string()),
                ],
            },
            LayoutItem::Field("items_expr".to_string()),
            LayoutItem::Field("chunk_size".to_string()),
        ],
        ai_tool: Default::default(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub items_expr: String,
    #[serde(default = "default_dispatch")]
    pub dispatch: String,
    #[serde(default)]
    pub chunk_size: Option<usize>,
}

fn default_dispatch() -> String {
    "seq".to_string()
}

pub struct Node {
    config: Config,
    compiled_items_expr: CompiledProgram,
    language: std::sync::Arc<dyn LanguageEngine>,
}

impl Node {
    pub fn new(
        config: Config,
        language: std::sync::Arc<dyn LanguageEngine>,
    ) -> Result<Self, PipelineError> {
        if config.dispatch.trim() != "seq" {
            return Err(PipelineError::new(
                "FW_NODE_LOGIC_FOREACH_CONFIG",
                "foreach dispatch currently supports only 'seq'",
            ));
        }
        let compiled_items_expr = compile_items_expr(language.as_ref(), &config.items_expr)?;
        Ok(Self {
            config,
            compiled_items_expr,
            language,
        })
    }
}

fn compile_items_expr(
    language: &dyn LanguageEngine,
    expr: &str,
) -> Result<CompiledProgram, PipelineError> {
    let expr = expr.trim();
    if expr.is_empty() {
        return Err(PipelineError::new(
            "FW_NODE_LOGIC_FOREACH_CONFIG",
            "items_expr must not be empty",
        ));
    }
    let source = format!(
        "var $input = input.$input;\n\
         var $item = input.$item;\n\
         var $index = input.$index;\n\
         var $count = input.$count;\n\
         var $trigger = input.$trigger || null;\n\
         var $nodes = input.$nodes || {{}};\n\
         return ({expr});"
    );
    let module = ModuleSource {
        id: format!("pipeline:logic.foreach:items:{expr}"),
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

fn build_emission_payload(base: &Value, item: Value, index: usize, count: usize) -> Value {
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
                &self.compiled_items_expr,
                build_expression_scope_input(&input.payload, &input.metadata),
                &ctx,
            )
            .map_err(|err| PipelineError::new("FW_NODE_LOGIC_FOREACH_RUN", err.to_string()))?
            .value;
        let items = source.as_array().ok_or_else(|| {
            PipelineError::new(
                "FW_NODE_LOGIC_FOREACH_TYPE",
                format!(
                    "items_expr '{}' did not resolve to an array",
                    self.config.items_expr
                ),
            )
        })?;

        let emitted_items: Vec<Value> = if let Some(chunk_size) = self.config.chunk_size {
            if chunk_size == 0 {
                return Err(PipelineError::new(
                    "FW_NODE_LOGIC_FOREACH_CHUNK",
                    "chunk_size must be greater than 0",
                ));
            }
            items
                .chunks(chunk_size)
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
                payload: build_emission_payload(&input.payload, item, index, count),
                trace: vec![
                    format!("node_kind={NODE_KIND}"),
                    format!("dispatch={}", self.config.dispatch),
                    format!("index={index}"),
                    format!("count={count}"),
                ],
            })
            .collect();

        Ok(outputs)
    }
}

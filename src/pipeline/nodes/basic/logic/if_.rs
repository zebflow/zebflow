//! `logic.if` — binary branch node.
//!
//! Evaluates `--when` (JavaScript over `input`) against the current execution
//! scope. Emits to the `true` pin when truthy, `false` pin otherwise; the
//! payload passes on unchanged — a control node adds no key.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

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

pub const NODE_KIND: &str = "logic.if";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_TRUE: &str = "true";
pub const OUTPUT_PIN_FALSE: &str = "false";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Process],
        title: "If".to_string(),
        description:
            "Sends the payload down `true` or `false` by a condition. Evaluates `--when` (JavaScript over `input`, `$trigger`, `$nodes`) and sends the payload, unchanged, \
             down the `true` pin or the `false` pin. This is how a pipeline validates, guards and answers 404/400: wire \
             `[b]:true -> [c]` and `[b]:false -> [e]` in graph mode — in pipe mode only `true` continues and `false` ends the run silently. \
             The pin not taken is skipped, and so is every node only it feeds (`$nodes` of a skipped node is `null`). \
             The expression sees the payload as `input` (right after a webhook, `input.webhook.body.x`; anywhere, `$trigger.body.x`), not `$input`."
                .to_string(),
        input_schema: serde_json::json!({ "type": "object" }),
        output_schema: serde_json::json!({ "type": "object" }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_TRUE.to_string(), OUTPUT_PIN_FALSE.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![DslFlag {
            flag: "--when".to_string(),
            config_key: "when".to_string(),
            description: "JavaScript over `input` (and `$trigger`, `$nodes`): truthy routes to the `true` pin, falsy to `false`."
                .to_string(),
            kind: DslFlagKind::Scalar,
            required: true,
            value: "expression".to_string(),
            ..Default::default()
        }],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType};
            vec![
                NodeFieldDef {
                    name: "when".to_string(),
                    label: "When".to_string(),
                    field_type: NodeFieldType::Textarea,
                    rows: Some(5),
                    help: Some(
                        "JS expression returning truthy/falsy. Routes to 'true'/'false' pin."
                            .to_string(),
                    ),
                    ..Default::default()
                },
            ]
        },
        layout: vec![
            LayoutItem::Field("when".to_string()),
        ],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Guard a form field", r#"logic.if --when "typeof $trigger.body?.title === 'string' && $trigger.body.title.length > 0""#)
                .note("`true` carries the same payload on; wire `false` to a `web.response.send --status 400`."),
            crate::pipeline::model::NodeExample::dsl("Found or not found", r#"logic.if --when "input.query.rows.length > 0""#)
                .input(serde_json::json!({ "query": { "rows": [] } }))
                .note("Fires the `false` pin; the payload is unchanged."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// `--when`: the condition, JavaScript over `input`.
    #[serde(deserialize_with = "super::expression_text")]
    pub when: String,
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
        let when = &crate::pipeline::expr::optional_nodes_paths(super::required_expression(&config.when, "--when", "FW_NODE_LOGIC_IF_CONFIG")?);
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
             return Boolean({when});"
        );
        let module = ModuleSource {
            id: format!("logic.if:{node_id}"),
            source_path: None,
            kind: SourceKind::Tsx,
            code: source,
        };
        let ir = language.parse(&module).map_err(|e| {
            PipelineError::new(
                "FW_NODE_LOGIC_IF_PARSE",
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
                    "FW_NODE_LOGIC_IF_COMPILE",
                    format!("node '{}': {}", node_id, e),
                )
            })?;
        Ok(Self {
            node_id: node_id.to_string(),
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
        &[OUTPUT_PIN_TRUE, OUTPUT_PIN_FALSE]
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
                    "FW_NODE_LOGIC_IF_RUN",
                    format!("node '{}': {}", self.node_id, e),
                )
            })?;

        let is_true = match &out.value {
            serde_json::Value::Bool(b) => *b,
            serde_json::Value::Null => false,
            serde_json::Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
            serde_json::Value::String(s) => !s.is_empty(),
            _ => true,
        };

        let pin = if is_true {
            OUTPUT_PIN_TRUE
        } else {
            OUTPUT_PIN_FALSE
        };
        Ok(NodeExecutionOutput {
            output_pins: vec![pin.to_string()],
            payload: input.payload,
            trace: vec![format!("node_kind={NODE_KIND}"), format!("result={pin}")],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn node(when: serde_json::Value) -> Result<Node, PipelineError> {
        let config: Config = serde_json::from_value(json!({ "when": when })).expect("config");
        Node::new("b", config, std::sync::Arc::new(crate::language::DenoSandboxEngine::default()))
    }

    fn run(node: &Node, payload: serde_json::Value) -> NodeExecutionOutput {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(node.execute_async(NodeExecutionInput {
                node_id: "b".to_string(),
                input_pin: INPUT_PIN_IN.to_string(),
                payload,
                metadata: json!({}),
                bus: None,
            }))
            .expect("run")
    }

    /// `--when` is the one flag: an expression, required.
    #[test]
    fn the_condition_is_when_an_expression() {
        let def = definition();
        assert_eq!(def.dsl_flags.len(), 1);
        let flag = &def.dsl_flags[0];
        assert_eq!((flag.flag.as_str(), flag.config_key.as_str(), flag.value.as_str()), ("--when", "when", "expression"));
        assert!(flag.required);
    }

    /// The pin follows the condition and the payload passes on unchanged.
    #[test]
    fn it_routes_and_passes_the_payload_on() {
        let node = node(json!("input.query.rows.length > 0")).expect("node");
        let payload = json!({ "query": { "rows": [{ "id": 1 }] } });
        let out = run(&node, payload.clone());
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_TRUE.to_string()]);
        assert_eq!(out.payload, payload);
        let out = run(&node, json!({ "query": { "rows": [] } }));
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_FALSE.to_string()]);
    }

    /// The DSL reads `--when true` as a JSON boolean; it is the literal.
    #[test]
    fn a_bare_literal_is_its_javascript_text() {
        let out = run(&node(json!(true)).expect("node"), json!({}));
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_TRUE.to_string()]);
    }

    /// Empty is not a value: an empty condition is refused at build.
    #[test]
    fn an_empty_condition_is_refused() {
        let err = node(json!("  ")).err().expect("refused");
        assert_eq!(err.code, "FW_NODE_LOGIC_IF_CONFIG");
        assert!(err.message.contains("--when"), "{}", err.message);
    }
}

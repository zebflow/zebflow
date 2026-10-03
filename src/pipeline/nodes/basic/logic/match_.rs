//! `logic.match` — multi-case routing node.
//!
//! Evaluates `--from` (JavaScript over `input`) to a string and routes to the
//! matching `--case` pin, or the `--default` pin if no case matches. The
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

pub const NODE_KIND: &str = "logic.match";
pub const INPUT_PIN_IN: &str = "in";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Process],
        title: "Match".to_string(),
        description:
            "Many-way branch. Evaluates `--from` (JavaScript over `input`) to a string and sends the payload, unchanged, down the pin \
             of the matching `--case` (repeat it, one value each), or down the `--default` pin (`default` unless named) when nothing \
             matches. Each case is an output pin you wire in graph mode (`[b]:create -> [c]`). For a yes/no decision use `logic.if`."
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
                flag: "--from".to_string(),
                config_key: "from".to_string(),
                description: "The value matched: JavaScript over `input` (and `$trigger`, `$nodes`), read as a string."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
                value: "expression".to_string(),
                ..Default::default()
            },
            DslFlag {
                flag: "--case".to_string(),
                config_key: "cases".to_string(),
                description: "One case value, repeated (`--case created --case updated`); each becomes an output pin."
                    .to_string(),
                kind: DslFlagKind::RepeatedList,
                required: false,
                value: "text".to_string(),
                ..Default::default()
            },
            DslFlag {
                flag: "--default".to_string(),
                config_key: "default".to_string(),
                description: "The pin taken when no case matches; `default` unless named.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                value: "text".to_string(),
                ..Default::default()
            },
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType};
            vec![
                NodeFieldDef {
                    name: "from".to_string(),
                    label: "Match on".to_string(),
                    field_type: NodeFieldType::Textarea,
                    rows: Some(4),
                    help: Some("JS expression over input/$trigger/$nodes; its string result selects a route pin.".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "match_routes".to_string(),
                    label: "Routes".to_string(),
                    field_type: NodeFieldType::MatchCases,
                    help: Some("Each route creates one output pin. The default route is used when no value matches.".to_string()),
                    ..Default::default()
                },
            ]
        },
        layout: vec![
            LayoutItem::Field("from".to_string()),
            LayoutItem::Field("match_routes".to_string()),
        ],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Route by event type", r#"logic.match --from "$trigger.body.type" --case created --case updated --case deleted --default other"#)
                .note("Pins: `created`, `updated`, `deleted`, `other`. Wire each: `[b]:created -> [c]`."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchCase {
    pub value: String,
    #[serde(default)]
    pub pin: String,
    #[serde(default)]
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchDefault {
    pub pin: String,
    #[serde(default)]
    pub label: String,
}

impl Default for MatchDefault {
    fn default() -> Self {
        Self {
            pin: default_case(),
            label: "Default".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// `--from`: the value matched, JavaScript over `input`.
    #[serde(deserialize_with = "super::expression_text")]
    pub from: String,
    #[serde(default, deserialize_with = "deserialize_match_cases")]
    pub cases: Vec<MatchCase>,
    #[serde(default, deserialize_with = "deserialize_match_default")]
    pub default: MatchDefault,
}

fn default_case() -> String {
    "default".to_string()
}

fn pin_from_value(value: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in value.trim().chars() {
        let next = if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            Some(ch.to_ascii_lowercase())
        } else if ch.is_whitespace() || matches!(ch, '.' | '/' | ':' | ',' | ';') {
            Some('-')
        } else {
            None
        };
        if let Some(ch) = next {
            if ch == '-' {
                if !last_dash && !out.is_empty() {
                    out.push(ch);
                    last_dash = true;
                }
            } else {
                out.push(ch);
                last_dash = false;
            }
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "case".to_string()
    } else {
        trimmed
    }
}

fn normalize_match_case(mut item: MatchCase) -> MatchCase {
    item.value = item.value.trim().to_string();
    item.pin = item.pin.trim().to_string();
    item.label = item.label.trim().to_string();
    if item.pin.is_empty() {
        item.pin = pin_from_value(&item.value);
    }
    if item.label.is_empty() {
        item.label = item.value.clone();
    }
    item
}

fn normalize_default(mut item: MatchDefault) -> MatchDefault {
    item.pin = item.pin.trim().to_string();
    item.label = item.label.trim().to_string();
    if item.pin.is_empty() {
        item.pin = default_case();
    }
    if item.label.is_empty() {
        item.label = "Default".to_string();
    }
    item
}

fn deserialize_match_cases<'de, D>(deserializer: D) -> Result<Vec<MatchCase>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    let mut out = Vec::new();
    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                match item {
                    serde_json::Value::String(value) => out.push(normalize_match_case(MatchCase {
                        value,
                        pin: String::new(),
                        label: String::new(),
                    })),
                    // `--case 404` is the text `404`: the value matched is a string.
                    serde_json::Value::Number(_) | serde_json::Value::Bool(_) => {
                        out.push(normalize_match_case(MatchCase {
                            value: item.to_string(),
                            pin: String::new(),
                            label: String::new(),
                        }))
                    }
                    serde_json::Value::Object(_) => {
                        let parsed: MatchCase =
                            serde_json::from_value(item).map_err(serde::de::Error::custom)?;
                        let normalized = normalize_match_case(parsed);
                        if !normalized.value.is_empty() {
                            out.push(normalized);
                        }
                    }
                    _ => {}
                }
            }
        }
        serde_json::Value::String(raw) => {
            for line in raw.lines() {
                let value = line.trim();
                if !value.is_empty() {
                    out.push(normalize_match_case(MatchCase {
                        value: value.to_string(),
                        pin: String::new(),
                        label: String::new(),
                    }));
                }
            }
        }
        _ => {}
    }
    Ok(out)
}

fn deserialize_match_default<'de, D>(deserializer: D) -> Result<MatchDefault, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    let parsed = match value {
        serde_json::Value::String(pin) => MatchDefault {
            pin,
            label: String::new(),
        },
        serde_json::Value::Number(_) | serde_json::Value::Bool(_) => MatchDefault {
            pin: value.to_string(),
            label: String::new(),
        },
        serde_json::Value::Object(_) => {
            serde_json::from_value(value).map_err(serde::de::Error::custom)?
        }
        _ => MatchDefault::default(),
    };
    Ok(normalize_default(parsed))
}

/// The output pins one `logic.match` declares, from its config: each case's
/// pin in order, then the default's. The parser and the node read the same
/// cases the same way, so a case `New York` is the pin `new-york` on both.
pub fn output_pins(config: &serde_json::Value) -> Vec<String> {
    #[derive(Deserialize, Default)]
    struct Routes {
        #[serde(default, deserialize_with = "deserialize_match_cases")]
        cases: Vec<MatchCase>,
        #[serde(default, deserialize_with = "deserialize_match_default")]
        default: MatchDefault,
    }
    let routes: Routes = serde_json::from_value(config.clone()).unwrap_or_default();
    let mut pins: Vec<String> = Vec::new();
    for pin in routes.cases.into_iter().map(|case| case.pin).chain(std::iter::once(routes.default.pin)) {
        if !pins.contains(&pin) {
            pins.push(pin);
        }
    }
    pins
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
        let from = super::required_expression(&config.from, "--from", "FW_NODE_LOGIC_MATCH_CONFIG")?.to_string();
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
             return String({from});"
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
        let pin = self
            .config
            .cases
            .iter()
            .find(|case| case.value == value)
            .map(|case| case.pin.clone())
            .unwrap_or_else(|| self.config.default.pin.clone());

        Ok(NodeExecutionOutput {
            output_pins: vec![pin.clone()],
            payload: input.payload,
            trace: vec![format!("node_kind={NODE_KIND}"), format!("matched={pin}")],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::model::DslFlagKind;
    use serde_json::json;

    fn node(config: serde_json::Value) -> Result<Node, PipelineError> {
        let config: Config = serde_json::from_value(config).expect("config");
        Node::new("m", config, std::sync::Arc::new(crate::language::DenoSandboxEngine::default()))
    }

    fn run(node: &Node, payload: serde_json::Value) -> NodeExecutionOutput {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(node.execute_async(NodeExecutionInput {
                node_id: "m".to_string(),
                input_pin: INPUT_PIN_IN.to_string(),
                payload,
                metadata: json!({}),
                bus: None,
            }))
            .expect("run")
    }

    /// `--from` is the value matched; `--case` repeats, one value each.
    #[test]
    fn the_flags_are_from_case_and_default() {
        let def = definition();
        let flags: Vec<&str> = def.dsl_flags.iter().map(|f| f.flag.as_str()).collect();
        assert_eq!(flags, ["--from", "--case", "--default"]);
        assert_eq!(def.dsl_flags[0].value, "expression");
        assert!(matches!(def.dsl_flags[1].kind, DslFlagKind::RepeatedList));
    }

    /// The matching case's pin, else the default's; the payload unchanged.
    #[test]
    fn it_routes_to_the_matching_case_or_the_default() {
        let node = node(json!({ "from": "input.kind", "cases": ["created", "New York", 404], "default": "other" })).expect("node");
        let payload = json!({ "kind": "New York" });
        let out = run(&node, payload.clone());
        assert_eq!(out.output_pins, vec!["new-york".to_string()]);
        assert_eq!(out.payload, payload);
        assert_eq!(run(&node, json!({ "kind": 404 })).output_pins, vec!["404".to_string()]);
        assert_eq!(run(&node, json!({ "kind": "gone" })).output_pins, vec!["other".to_string()]);
    }

    /// The pins the parser declares are the pins the node routes to.
    #[test]
    fn the_declared_pins_are_the_routed_pins() {
        assert_eq!(output_pins(&json!({ "cases": ["a", "New York", "a"] })), vec!["a", "new-york", "default"]);
        assert_eq!(output_pins(&json!({ "cases": ["done"], "default": "retry" })), vec!["done", "retry"]);
        assert_eq!(output_pins(&json!({})), vec!["default"]);
    }

    /// Empty is not a value: an empty `--from` is refused at build.
    #[test]
    fn an_empty_from_is_refused() {
        let err = node(json!({ "from": "", "cases": ["a"] })).err().expect("refused");
        assert_eq!(err.code, "FW_NODE_LOGIC_MATCH_CONFIG");
        assert!(err.message.contains("--from"), "{}", err.message);
    }
}

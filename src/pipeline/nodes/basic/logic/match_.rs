//! `n.logic.match` — multi-case routing node.
//!
//! Evaluates a DSL expression to get a string value.
//! Routes to the matching case pin, or the default pin if no case matches.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

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

pub const NODE_KIND: &str = "n.logic.match";
pub const INPUT_PIN_IN: &str = "in";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Match".to_string(),
        description:
            "Evaluates a DSL expression using $input/$trigger/$nodes and routes to matching case pin, or default."
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
                flag: "--expr".to_string(),
                config_key: "expression".to_string(),
                description: "JS expression returning a string to match against case pins."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--cases".to_string(),
                config_key: "cases".to_string(),
                description: "Comma-separated case values; each becomes an output pin.".to_string(),
                kind: DslFlagKind::CommaSeparatedList,
                required: false,
            },
            DslFlag {
                flag: "--default".to_string(),
                config_key: "default".to_string(),
                description: "Pin name to route to when no case matches.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType};
            vec![
                NodeFieldDef {
                    name: "expression".to_string(),
                    label: "Expression".to_string(),
                    field_type: NodeFieldType::Textarea,
                    rows: Some(4),
                    help: Some("JS expression evaluated against $input/$trigger/$nodes; its string result selects a route pin.".to_string()),
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
            LayoutItem::Field("expression".to_string()),
            LayoutItem::Field("match_routes".to_string()),
        ],
        ai_tool: Default::default(),
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
    pub expression: String,
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
        serde_json::Value::Object(_) => {
            serde_json::from_value(value).map_err(serde::de::Error::custom)?
        }
        _ => MatchDefault::default(),
    };
    Ok(normalize_default(parsed))
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
        let source = format!(
            "var $input = input.$input;\n\
             var $item = input.$item;\n\
             var $index = input.$index;\n\
             var $count = input.$count;\n\
             var $trigger = input.$trigger || null;\n\
             var $nodes = input.$nodes || {{}};\n\
             return String({});",
            config.expression
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

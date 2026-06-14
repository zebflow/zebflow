//! `n.trigger.function` — marks a pipeline as a callable function unit.
//!
//! # Pipeline position
//!
//! Always the first (and only trigger) node in a function pipeline.
//! Function pipelines are reusable callable units invoked from other pipelines
//! via `n.function.call` or exposed as Project Operator tools.
//!
//! # User-facing config
//! | Field | Type | Required | Description |
//! |---|---|---|---|
//! | `description` | string | yes | What the function does and when to use it |
//! | `input_schema` | object | yes | JSON Schema object describing function inputs |
//! | `output_schema` | object | yes | JSON Schema object describing function output |
//! | `examples` | array | no | Optional input/output examples for tool callers |
//!
//! # DSL
//! ```text
//! | trigger.function --title "Lookup user" --description "Looks up one user." \
//!     --input user_id:string! "User id." --output ok:boolean! "Whether lookup succeeded."
//! | script -- return { greeting: "hello " + input.user_id }
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "n.trigger.function";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// What this function does and when callers should use it.
    #[serde(default)]
    pub description: String,
    /// Full JSON Schema object defining function input.
    #[serde(default)]
    pub input_schema: Value,
    /// Full JSON Schema object defining function output.
    #[serde(default)]
    pub output_schema: Value,
    /// Optional examples: `[{"input": {...}, "output": {...}}]`.
    #[serde(default)]
    pub examples: Value,
    /// Deprecated compatibility alias for `input_schema.properties`.
    #[serde(default)]
    pub params: Value,
}

pub struct Node {
    #[allow(dead_code)]
    config: Config,
}

impl Node {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Function Trigger".to_string(),
        description: "Marks this pipeline as a callable function. The pipeline can be invoked \
            from other pipelines via n.function.call or exposed as a Project Operator tool. \
            Declare title, description, input_schema, and output_schema so every caller sees \
            the same typed contract."
            .to_string(),
        input_pins: vec![],
        output_pins: vec!["out".to_string()],
        config_schema: serde_json::json!({
            "type": "object",
            "required": ["description", "input_schema", "output_schema"],
            "properties": {
                "description": {
                    "type": "string",
                    "description": "What this function does and when callers should use it."
                },
                "input_schema": {
                    "type": "object",
                    "description": "JSON Schema object describing the function input."
                },
                "output_schema": {
                    "type": "object",
                    "description": "JSON Schema object describing the function output."
                },
                "examples": {
                    "type": "array",
                    "description": "Optional examples with input and output objects.",
                    "items": { "type": "object" }
                },
                "params": {
                    "type": "object",
                    "description": "Deprecated compatibility alias for input_schema.properties."
                }
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--description".to_string(),
                config_key: "description".to_string(),
                description: "What this function does and when callers should use it.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--input".to_string(),
                config_key: "input_schema".to_string(),
                description: "Input field declaration: name:type! plus optional description. \
                    Example: --input columns:string[]! \"Source column names.\""
                    .to_string(),
                kind: DslFlagKind::SchemaField,
                required: false,
            },
            DslFlag {
                flag: "--output".to_string(),
                config_key: "output_schema".to_string(),
                description: "Output field declaration: name:type! plus optional description. \
                    Example: --output ok:boolean! \"Whether the function succeeded.\""
                    .to_string(),
                kind: DslFlagKind::SchemaField,
                required: false,
            },
            DslFlag {
                flag: "--input-schema".to_string(),
                config_key: "input_schema".to_string(),
                description: "Full JSON Schema object for this function's input.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--output-schema".to_string(),
                config_key: "output_schema".to_string(),
                description: "Full JSON Schema object for this function's output.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--example".to_string(),
                config_key: "examples".to_string(),
                description: "Optional JSON example with input and output. Repeat for 1-3 examples."
                    .to_string(),
                kind: DslFlagKind::RepeatedList,
                required: false,
            },
            DslFlag {
                flag: "--params".to_string(),
                config_key: "params".to_string(),
                description: "Deprecated input properties object. Prefer --input or --input-schema."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "description".to_string(),
                label: "Description".to_string(),
                field_type: NodeFieldType::Textarea,
                help: Some("What this function does and when callers should use it.".to_string()),
                rows: Some(3),
                span: Some("full".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "input_schema".to_string(),
                label: "Input Schema".to_string(),
                field_type: NodeFieldType::ParamsBuilder,
                help: Some(
                    "Define the input arguments this function accepts. Required fields are enforced before execution.".to_string(),
                ),
                default_value: Some(json!({ "type": "object", "required": [], "properties": {} })),
                span: Some("full".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "output_schema".to_string(),
                label: "Output Schema".to_string(),
                field_type: NodeFieldType::ParamsBuilder,
                help: Some(
                    "Define the structured output this function returns. Include ok:boolean for tool-friendly results.".to_string(),
                ),
                default_value: Some(json!({
                    "type": "object",
                    "required": ["ok"],
                    "properties": {
                        "ok": {
                            "type": "boolean",
                            "description": "Whether the function succeeded."
                        }
                    }
                })),
                span: Some("full".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "examples".to_string(),
                label: "Examples".to_string(),
                field_type: NodeFieldType::CodeEditor,
                language: Some("json".to_string()),
                help: Some(
                    "Optional JSON array of examples: [{\"input\": {...}, \"output\": {...}}].".to_string(),
                ),
                default_value: Some(json!("[]")),
                span: Some("full".to_string()),
                ..Default::default()
            },
        ],
        ..Default::default()
    }
}

pub fn input_schema_from_config(config: &Value) -> Value {
    schema_from_config(config, "input_schema")
        .or_else(|| params_schema_from_config(config))
        .unwrap_or_else(empty_object_schema)
}

pub fn output_schema_from_config(config: &Value) -> Value {
    schema_from_config(config, "output_schema").unwrap_or_else(|| {
        json!({
            "type": "object",
            "required": ["ok"],
            "properties": {
                "ok": {
                    "type": "boolean",
                    "description": "Whether the function succeeded."
                }
            }
        })
    })
}

pub fn examples_from_config(config: &Value) -> Value {
    let value = config
        .get("examples")
        .and_then(parse_json_value)
        .filter(|v| v.is_array())
        .unwrap_or_else(|| json!([]));
    let items = value
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|item| parse_json_value(item).unwrap_or_else(|| item.clone()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Value::Array(items)
}

pub fn tool_description_from_config(slug: &str, config: &Value) -> String {
    let title = config
        .get("title")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(slug);
    let description = config
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let output_schema = output_schema_from_config(config);
    let examples = examples_from_config(config);

    let mut parts = vec![format!("{}: {}", title.trim(), description)];
    if let Ok(pretty) = serde_json::to_string_pretty(&output_schema) {
        parts.push(format!("Output schema:\n{pretty}"));
    }
    if examples.as_array().map(|a| !a.is_empty()).unwrap_or(false)
        && let Ok(pretty) = serde_json::to_string_pretty(&examples)
    {
        parts.push(format!("Examples:\n{pretty}"));
    }
    parts.join("\n\n")
}

pub fn validate_function_input(schema: &Value, input: &Value) -> Result<(), Value> {
    let Some(obj) = input.as_object() else {
        return Err(validation_error(
            "invalid_input_type",
            None,
            "Function input must be a JSON object.",
        ));
    };

    if let Some(required) = schema.get("required").and_then(Value::as_array) {
        for name in required.iter().filter_map(Value::as_str) {
            if !obj.contains_key(name) {
                return Err(validation_error(
                    "missing_required_arg",
                    Some(name),
                    &format!("Call this function again with `{name}`."),
                ));
            }
        }
    }

    let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
        return Ok(());
    };
    for (name, prop_schema) in properties {
        if let Some(value) = obj.get(name)
            && !schema_type_matches(prop_schema, value)
        {
            return Err(validation_error(
                "invalid_arg_type",
                Some(name),
                &format!("Argument `{name}` does not match the declared schema."),
            ));
        }
    }
    Ok(())
}

fn schema_from_config(config: &Value, key: &str) -> Option<Value> {
    config
        .get(key)
        .and_then(parse_json_value)
        .map(normalize_schema_object)
}

fn params_schema_from_config(config: &Value) -> Option<Value> {
    let props = config.get("params").and_then(parse_json_value)?;
    if !props.is_object() {
        return None;
    }
    Some(json!({
        "type": "object",
        "properties": props,
        "required": required_from_properties(&props)
    }))
}

fn parse_json_value(value: &Value) -> Option<Value> {
    match value {
        Value::String(s) => serde_json::from_str::<Value>(s).ok(),
        other => Some(other.clone()),
    }
}

fn normalize_schema_object(schema: Value) -> Value {
    if schema.get("properties").is_some() {
        let mut schema = schema;
        if let Some(obj) = schema.as_object_mut() {
            obj.entry("type".to_string())
                .or_insert_with(|| json!("object"));
            if !obj.contains_key("required")
                && let Some(props) = obj.get("properties")
            {
                obj.insert("required".to_string(), required_from_properties(props));
            }
        }
        schema
    } else if schema.is_object() {
        json!({
            "type": "object",
            "properties": schema,
            "required": required_from_properties(&schema)
        })
    } else {
        empty_object_schema()
    }
}

fn required_from_properties(properties: &Value) -> Value {
    let required = properties
        .as_object()
        .map(|props| {
            props
                .iter()
                .filter_map(|(name, prop)| {
                    prop.get("required")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                        .then(|| json!(name))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Value::Array(required)
}

fn empty_object_schema() -> Value {
    json!({ "type": "object", "properties": {}, "required": [] })
}

fn schema_type_matches(schema: &Value, value: &Value) -> bool {
    if let Some(zeb_type) = schema.get("x-zebflow-type").and_then(Value::as_str) {
        return match zeb_type {
            "file" | "blob" => {
                value.is_object() && value.get("uri").and_then(Value::as_str).is_some()
            }
            "bytes" => {
                value.is_object()
                    && value.get("encoding").and_then(Value::as_str) == Some("base64")
                    && value.get("data").and_then(Value::as_str).is_some()
            }
            _ => true,
        };
    }
    match schema.get("type").and_then(Value::as_str) {
        None => true,
        Some("string") => value.is_string(),
        Some("number") => value.is_number(),
        Some("integer") => value.as_i64().is_some() || value.as_u64().is_some(),
        Some("boolean") => value.is_boolean(),
        Some("object") => value.is_object(),
        Some("array") => {
            let Some(arr) = value.as_array() else {
                return false;
            };
            if let Some(item_schema) = schema.get("items") {
                arr.iter()
                    .all(|item| schema_type_matches(item_schema, item))
            } else {
                true
            }
        }
        _ => true,
    }
}

fn validation_error(code: &str, arg: Option<&str>, retry_hint: &str) -> Value {
    let mut obj = Map::new();
    obj.insert("ok".to_string(), json!(false));
    obj.insert("code".to_string(), json!(code));
    if let Some(arg) = arg {
        obj.insert("arg".to_string(), json!(arg));
    }
    obj.insert("retry_hint".to_string(), json!(retry_hint));
    Value::Object(obj)
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        NODE_KIND
    }

    fn input_pins(&self) -> &'static [&'static str] {
        &[]
    }

    fn output_pins(&self) -> &'static [&'static str] {
        &["out"]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        // Passthrough — the caller's payload is injected before dispatch.
        Ok(NodeExecutionOutput {
            output_pins: vec!["out".to_string()],
            payload: input.payload,
            trace: vec![format!("node_kind={NODE_KIND}: passthrough")],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{input_schema_from_config, validate_function_input};
    use serde_json::json;

    #[test]
    fn function_input_validation_checks_required_and_special_types() {
        let config = json!({
            "input_schema": {
                "type": "object",
                "required": ["source", "image"],
                "properties": {
                    "source": { "type": "object", "x-zebflow-type": "file" },
                    "image": { "type": "object", "x-zebflow-type": "bytes" },
                    "options": {}
                }
            }
        });
        let schema = input_schema_from_config(&config);

        let missing = validate_function_input(
            &schema,
            &json!({
                "source": { "uri": "zebfs://project/uploads/a.csv" }
            }),
        )
        .expect_err("missing bytes should fail");
        assert_eq!(missing["code"], json!("missing_required_arg"));
        assert_eq!(missing["arg"], json!("image"));

        let invalid_file = validate_function_input(
            &schema,
            &json!({
                "source": { "name": "a.csv" },
                "image": { "encoding": "base64", "data": "AA==" }
            }),
        )
        .expect_err("file ref without uri should fail");
        assert_eq!(invalid_file["code"], json!("invalid_arg_type"));
        assert_eq!(invalid_file["arg"], json!("source"));

        validate_function_input(
            &schema,
            &json!({
                "source": { "uri": "zebfs://project/uploads/a.csv", "name": "a.csv" },
                "image": { "encoding": "base64", "data": "AA==", "mime": "image/png" },
                "options": ["any", { "json": true }]
            }),
        )
        .expect("valid input");
    }
}

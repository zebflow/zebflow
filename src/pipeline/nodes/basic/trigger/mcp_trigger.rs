//! `trigger.mcp` — expose a pipeline as an MCP tool.
//!
//! This node is a **routing declaration**, not an active processor.  At
//! compile time the pipeline runtime extracts [`McpTriggerSpec`] from the
//! node config.  When the MCP server receives `tools/list`, dynamic tools
//! from active pipelines are merged with the static tool set.  When an
//! agent calls the tool, the pipeline executes with the tool arguments as
//! input.  The node answers one key, `mcp`: `{ tool_name, arguments }`.
//!
//! # Config flags
//!
//! | Flag | Type | Required | Description |
//! |---|---|---|---|
//! | `--name` | string | yes | MCP tool name (e.g. `greet_user`) |
//! | `--description` | string | no | Human-readable tool description |
//! | `--params` | csv | no | Comma-separated `name:type` pairs (e.g. `name:string,age:number`) |
//!
//! # The answer: `mcp: { … }`
//!
//! The MCP dispatch handler builds the envelope before execution:
//!
//! | Field | Type | Description |
//! |---|---|---|
//! | `tool_name` | string | The MCP tool name that was called |
//! | `arguments` | object | The arguments passed by the AI agent |
//!
//! # Example pipelines
//!
//! **Greeting tool:**
//! ```text
//! | trigger.mcp --name greet_user --description "Greet a user by name" --params name:string
//! | javascript.script.run -- "return { greeting: 'Hello, ' + input.mcp.arguments.name + '!' };"
//! | web.response.send
//! ```
//!
//! **Database lookup tool:**
//! ```text
//! | trigger.mcp --name lookup_user --description "Look up user by email" --params email:string
//! | pg.query.run --credential main-db --param "1={{ input.mcp.arguments.email }}" -- "SELECT * FROM users WHERE email = $1"
//! | web.response.send
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "trigger.mcp";
const OUTPUT_PIN_OUT: &str = "out";
/// The key this trigger answers under.
pub const ANSWER_KEY: &str = "mcp";

/// Return the [`NodeDefinition`] for `trigger.mcp`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "MCP Tool Trigger".to_string(),
        description: "Makes this pipeline a tool on the project's MCP endpoint: once active, `--name` appears in `tools/list` and an \
            agent calling it runs the pipeline. Answers one key, `mcp`: `{ tool_name, arguments }` — an argument is \
            `input.mcp.arguments.<param>` (`$trigger.arguments.<param>`). `--params` declares the \
            arguments as `name:type` pairs (`string`, `number`, `boolean`, `object`, `array`); `--description` is what the agent \
            reads to decide when to call it — write it as a trigger, not a label. The answer the agent receives is the last node's \
            payload as JSON; do not end in a page."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "tool_name": {
                    "type": "string",
                    "description": "The MCP tool name that was called."
                },
                "arguments": {
                    "type": "object",
                    "description": "The arguments passed by the AI agent."
                }
            }
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "mcp": {
                    "type": "object",
                    "properties": {
                        "tool_name": { "type": "string" },
                        "arguments": { "type": "object" }
                    }
                }
            }
        }),
        input_pins: vec![],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": {
                    "type": "string",
                    "description": "MCP tool name. Must be a valid identifier (letters, digits, underscores). This is how AI agents will call this tool."
                },
                "description": {
                    "type": "string",
                    "description": "Human-readable description shown to AI agents explaining what this tool does."
                },
                "parameters": {
                    "type": "string",
                    "description": "Comma-separated name:type pairs defining the tool input schema. Types: string, number, integer, boolean, object, array. Example: name:string,age:number,active:boolean"
                }
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--name".to_string(),
                config_key: "name".to_string(),
                description: "MCP tool name (e.g. greet_user). Must be a valid identifier."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
                value: "text".to_string(),
                ..Default::default()
            },
            DslFlag {
                flag: "--description".to_string(),
                config_key: "description".to_string(),
                description: "What the tool does and when an agent should call it.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                value: "text".to_string(),
                ..Default::default()
            },
            DslFlag {
                flag: "--params".to_string(),
                config_key: "parameters".to_string(),
                description:
                    "Comma-separated name:type pairs. Types: string, number, integer, boolean, object, array. Example: name:string,age:number"
                        .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                value: "text".to_string(),
                ..Default::default()
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "name".to_string(),
                label: "Tool Name".to_string(),
                field_type: NodeFieldType::Text,
                help: Some(
                    "MCP tool identifier. AI agents will call this tool by this name.".to_string(),
                ),
                ..Default::default()
            },
            NodeFieldDef {
                name: "description".to_string(),
                label: "Description".to_string(),
                field_type: NodeFieldType::Textarea,
                help: Some("Human-readable description shown to AI agents.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "parameters".to_string(),
                label: "Parameters".to_string(),
                field_type: NodeFieldType::Text,
                help: Some(
                    "Comma-separated name:type pairs (e.g. name:string,age:number).".to_string(),
                ),
                ..Default::default()
            },
        ],
        layout: vec![
            LayoutItem::Field("name".to_string()),
            LayoutItem::Field("description".to_string()),
            LayoutItem::Field("parameters".to_string()),
        ],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("A tool that looks up stock", r#"trigger.mcp --name stock_lookup --description "Current stock level for one SKU. Use before promising availability." --params sku:string"#)
                .input(serde_json::json!({ "tool_name": "stock_lookup", "arguments": { "sku": "MUG-01" } }))
                .output(serde_json::json!({ "mcp": { "tool_name": "stock_lookup", "arguments": { "sku": "MUG-01" } } }))
                .note("Then `| sekejap.query.run --param \"1={{ input.mcp.arguments.sku }}\" -- \"SELECT sku, on_hand FROM stock WHERE sku = $1\"`."),
        ],
        ..Default::default()
    }
}

/// Configuration for `trigger.mcp`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// MCP tool name — the identifier used in `tools/list` and `tools/call`.
    pub name: String,

    /// Human-readable tool description shown to AI agents.
    #[serde(default)]
    pub description: String,

    /// Comma-separated `name:type` pairs defining the input schema.
    /// Example: `"name:string,age:number,active:boolean"`
    #[serde(default)]
    pub parameters: String,
}

/// `trigger.mcp` node instance.
pub struct Node {
    #[allow(dead_code)]
    config: Config,
}

impl Node {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
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
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        // The MCP dispatch handler built `{ tool_name, arguments }`.
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: super::answer_under(ANSWER_KEY, input.payload),
            trace: vec![format!("trigger.mcp: tool={}", self.config.name)],
        })
    }
}

/// Parse a `name:type` parameter string into a JSON Schema object.
///
/// Input: `"name:string,age:number,active:boolean"`
/// Output:
/// ```json
/// {
///   "type": "object",
///   "properties": {
///     "name": {"type": "string"},
///     "age": {"type": "number"},
///     "active": {"type": "boolean"}
///   },
///   "required": ["name", "age", "active"]
/// }
/// ```
pub fn params_to_json_schema(params: &str) -> serde_json::Value {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();

    for pair in params.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let parts: Vec<&str> = pair.splitn(2, ':').collect();
        let name = parts[0].trim();
        if name.is_empty() {
            continue;
        }
        let type_str = if parts.len() > 1 {
            parts[1].trim()
        } else {
            "string"
        };
        // Validate JSON Schema type
        let json_type = match type_str {
            "string" | "number" | "integer" | "boolean" | "object" | "array" => type_str,
            _ => "string",
        };
        properties.insert(name.to_string(), json!({"type": json_type}));
        required.push(serde_json::Value::String(name.to_string()));
    }

    json!({
        "type": "object",
        "properties": properties,
        "required": required
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_to_json_schema_parses_name_type_pairs() {
        let schema = params_to_json_schema("name:string,age:number,active:boolean");
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["name"]["type"], "string");
        assert_eq!(schema["properties"]["age"]["type"], "number");
        assert_eq!(schema["properties"]["active"]["type"], "boolean");
        let required = schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 3);
    }

    #[test]
    fn params_to_json_schema_handles_empty_string() {
        let schema = params_to_json_schema("");
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].as_object().unwrap().is_empty());
        assert!(schema["required"].as_array().unwrap().is_empty());
    }

    #[test]
    fn params_to_json_schema_defaults_missing_type_to_string() {
        let schema = params_to_json_schema("name");
        assert_eq!(schema["properties"]["name"]["type"], "string");
    }
}

//! `trigger.mcp` — publish a pipeline as one tool of an MCP server on an app
//! route (`docs/contracts/published-mcp.md`).
//!
//! This node is a **routing declaration**. At activation the pipeline runtime
//! extracts a `McpTriggerSpec` from it; every active `trigger.mcp` with the
//! same `--route` in one project is one tool of the MCP server that route
//! serves, on the `mcp` surface (`/_mcp/ROUTE` on the project's hosts,
//! `/mcp/{owner}/{project}/ROUTE` on the platform; off by default). The
//! project's dev MCP
//! (`/api/projects/{o}/{p}/mcp`) never lists or calls these tools.
//!
//! # Config flags
//!
//! | Flag | Required | Meaning |
//! |---|---|---|
//! | `--route` | yes | the path the MCP server answers on; one route, one server |
//! | `--name` | yes | the tool name, unique on its route |
//! | `--description` | no | what the agent reads to decide when to call the tool |
//! | `--parameter` | no | `name:type[!] "doc"`, repeated — the tool's arguments, as on `trigger.function` |
//! | `--auth` | yes | `none`, `jwt` or `api_key` — the same on every tool of a route |
//! | `--credential` | with `jwt` / `api_key` | the credential that verifies `--auth` |
//! | `--role` | no | a JWT role allowed in, repeated |
//! | `--errors` | no | `show` or `hide`: what a failed run reveals in the tool error |
//!
//! # The answer: `mcp: { route, tool_name, arguments }`
//!
//! The route's server builds the envelope before the run; `$trigger` is the
//! same envelope. The tool result is what `web.response.send` answers (its
//! `--body`), or the run's value without one; a status of 400 or more is a
//! tool error with that body.
//!
//! 0.11 seals the declaration — flags, answer, the save and activation
//! checks. Serving the routes (the server, its auth at the door) lands in
//! 0.11.1; until then the `mcp` surface answers 404.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::webhook;
use crate::pipeline::model::{
    DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType, SelectOptionDef,
};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "trigger.mcp";
const OUTPUT_PIN_OUT: &str = "out";
/// The key this trigger answers under.
pub const ANSWER_KEY: &str = "mcp";
/// `--auth`: the closed words. `hmac` signs one request body for one sender,
/// which no MCP client does, so a published route does not take it.
pub const AUTH_MODES: [&str; 3] = ["none", "jwt", "api_key"];
/// Raised when the flags of a `trigger.mcp` are refused at activation.
pub const CONFIG_CODE: &str = "FW_NODE_TRIGGER_MCP_CONFIG";

/// Return the [`NodeDefinition`] for `trigger.mcp`.
pub fn definition() -> NodeDefinition {
    let mut dsl_flags = vec![
        DslFlag {
            flag: "--route".to_string(),
            config_key: "route".to_string(),
            description: "The server's path under the mcp surface: `--route /shop` answers at /_mcp/shop on the project's hosts. Every active trigger.mcp with this route is one tool of that one server.".to_string(),
            kind: DslFlagKind::Scalar,
            required: true,
            value: "text".to_string(),
            ..Default::default()
        },
        DslFlag {
            flag: "--name".to_string(),
            config_key: "name".to_string(),
            description: "The tool name agents call (letters, digits, `_`, `-`, `.`), unique on its route.".to_string(),
            kind: DslFlagKind::Scalar,
            required: true,
            value: "text".to_string(),
            ..Default::default()
        },
        DslFlag {
            flag: "--description".to_string(),
            config_key: "description".to_string(),
            description: "What the tool does and when an agent should call it — the agent reads this to decide.".to_string(),
            kind: DslFlagKind::Scalar,
            value: "text".to_string(),
            ..Default::default()
        },
        DslFlag {
            flag: "--parameter".to_string(),
            config_key: "schema".to_string(),
            description: "One argument of the tool, repeated: name:type! plus an optional description, as on trigger.function. \
                Example: --parameter q:string! \"The words to look for.\""
                .to_string(),
            kind: DslFlagKind::SchemaField,
            value: "text".to_string(),
            ..Default::default()
        },
    ];
    let mut guard = webhook::auth_flags("the MCP server");
    if let Some(auth) = guard.iter_mut().find(|f| f.flag == "--auth") {
        auth.description = "Who may call the server: none (open — must be written), jwt or api_key (each needs --credential). Every trigger.mcp on one route declares the same.".to_string();
        auth.required = true;
        auth.choices = AUTH_MODES.iter().map(|w| w.to_string()).collect();
    }
    dsl_flags.extend(guard);
    dsl_flags.push(DslFlag {
        flag: "--errors".to_string(),
        config_key: "errors".to_string(),
        description: "show or hide: what a failed run reveals in the tool error, overriding the project's errors switch (Settings → Addressing). Absent: the project decides.".to_string(),
        kind: DslFlagKind::Scalar,
        value: "text".to_string(),
        choices: webhook::ERRORS_MODES.iter().map(|w| w.to_string()).collect(),
        ..Default::default()
    });

    let mut fields = vec![
        NodeFieldDef { name: "route".to_string(), label: "Route".to_string(), field_type: NodeFieldType::Text, help: Some("The server's path under /_mcp, e.g. /shop.".to_string()), ..Default::default() },
        NodeFieldDef { name: "__webhook_public_url".to_string(), label: "Server URL".to_string(), field_type: NodeFieldType::CopyUrl, help: Some("The URL an MCP client connects to.".to_string()), ..Default::default() },
        NodeFieldDef { name: "name".to_string(), label: "Tool Name".to_string(), field_type: NodeFieldType::Text, help: Some("The tool name agents call; unique on its route.".to_string()), ..Default::default() },
        NodeFieldDef { name: "description".to_string(), label: "Description".to_string(), field_type: NodeFieldType::Textarea, help: Some("What the tool does and when an agent should call it.".to_string()), rows: Some(3), span: Some("full".to_string()), ..Default::default() },
        NodeFieldDef {
            name: "schema".to_string(),
            label: "Arguments".to_string(),
            field_type: NodeFieldType::ParamsBuilder,
            help: Some("The arguments the tool takes. Required ones are enforced before the run.".to_string()),
            default_value: Some(json!({ "type": "object", "required": [], "properties": {} })),
            span: Some("full".to_string()),
            ..Default::default()
        },
    ];
    for mut field in webhook::auth_fields("Refused before any run: 401, or 403 for a missing role.") {
        if field.name == "auth" {
            field.options.retain(|o| AUTH_MODES.contains(&o.value.as_str()));
        }
        fields.push(field);
    }
    fields.push(NodeFieldDef {
        name: "errors".to_string(),
        label: "Errors".to_string(),
        field_type: NodeFieldType::Select,
        options: vec![
            SelectOptionDef { value: String::new(), label: "Project decides".to_string() },
            SelectOptionDef { value: "show".to_string(), label: "show".to_string() },
            SelectOptionDef { value: "hide".to_string(), label: "hide".to_string() },
        ],
        help: Some("What a failed run reveals in the tool error. Empty: the project's errors switch decides.".to_string()),
        ..Default::default()
    });

    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "MCP Tool Trigger".to_string(),
        description: "Publishes this pipeline as one tool of an MCP server on a route of its own, for outside agents (ChatGPT, Claude, any MCP client). \
            Every active trigger.mcp with the same `--route` is one tool of that route's server; another route is another server. \
            The server answers on the mcp surface, off by default — `/_mcp/ROUTE` on the project's hosts, `/mcp/{owner}/{project}/ROUTE` on the platform — \
            and never on the project's dev MCP. `--auth` is required (`none` publishes openly) and is the app's own — a key or a token from its credentials, never a Zebflow account or session. \
            Answers one key, `mcp`: `{ route, tool_name, arguments }` — an argument is `input.mcp.arguments.<name>` \
            (`$trigger.arguments.<name>` later). The tool result is what `web.response.send` answers (its `--body`), or the run's value \
            without one; a status of 400 or more is a tool error with that body. \
            In 0.11 the declaration is checked at save and activation; the route is served from 0.11.1."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "description": "The envelope the route's server builds: route, tool_name, arguments."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "mcp": {
                    "type": "object",
                    "properties": {
                        "route": { "type": "string" },
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
            "required": ["route", "name", "auth"],
            "properties": {
                "route": { "type": "string", "description": "The path the MCP server answers on. Starts with /; never /_…." },
                "name": { "type": "string", "description": "The tool name, unique on its route." },
                "description": { "type": "string", "description": "What the tool does and when an agent should call it." },
                "schema": { "type": "object", "description": "JSON Schema object of the tool's arguments." },
                "auth": { "type": "string", "enum": AUTH_MODES, "description": "none, jwt or api_key; the same on every tool of a route." },
                "credential_id": { "type": "string", "description": "The credential that verifies --auth. Required unless auth is none." },
                "role": { "type": "array", "items": { "type": "string" }, "description": "Roles allowed in; one entry of the JWT 'roles' claim must match." },
                "errors": { "type": "string", "enum": webhook::ERRORS_MODES, "description": "What a failed run reveals in the tool error: show or hide. Absent: the project decides." }
            }
        }),
        dsl_flags,
        fields,
        layout: vec![
            LayoutItem::Field("route".to_string()),
            LayoutItem::Field("__webhook_public_url".to_string()),
            LayoutItem::Field("name".to_string()),
            LayoutItem::Field("description".to_string()),
            LayoutItem::Field("schema".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("auth".to_string()), LayoutItem::Field("credential_id".to_string())] },
            LayoutItem::Field("role".to_string()),
            LayoutItem::Field("errors".to_string()),
        ],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl(
                "A tool on a protected server",
                r#"trigger.mcp --route /shop --name stock_lookup --description "Current stock level for one SKU. Use before promising availability." --parameter sku:string! "The SKU to look up." --auth api_key --credential shop-mcp-key"#,
            )
            .input(json!({ "route": "/shop", "tool_name": "stock_lookup", "arguments": { "sku": "MUG-01" } }))
            .output(json!({ "mcp": { "route": "/shop", "tool_name": "stock_lookup", "arguments": { "sku": "MUG-01" } } }))
            .note("Then `| sekejap.query.run --param \"1={{ input.mcp.arguments.sku }}\" -- \"SELECT sku, on_hand FROM stock WHERE sku = $1\" | web.response.send --body \"{{ input.query.rows }}\"`. An MCP client will connect to `/_mcp/shop` on the project's host with an `X-API-Key` header (served from 0.11.1)."),
        ],
        ..Default::default()
    }
}

/// Configuration for `trigger.mcp`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub route: String,
    /// The tool name in `tools/list` and `tools/call`.
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// JSON Schema object of the arguments (`--parameter`).
    #[serde(default)]
    pub schema: Value,
    #[serde(default)]
    pub auth: String,
    #[serde(default)]
    pub credential_id: String,
    #[serde(default)]
    pub role: Vec<String>,
    #[serde(default)]
    pub errors: String,
}

/// The tool's input schema: the declared `--parameter`s, an open object when
/// none is declared — the same reading `trigger.function` gives its own.
pub fn input_schema_from_config(config: &Value) -> Value {
    super::function::input_schema_from_config(config)
}

/// `true` for a tool name MCP clients accept: 1–128 of letters, digits, `_`,
/// `-` and `.`.
pub fn valid_tool_name(name: &str) -> bool {
    (1..=128).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}

/// A route as the server compares it: a leading `/`, no trailing one.
pub fn normalize_route(route: &str) -> String {
    let trimmed = route.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

/// `trigger.mcp` node instance.
pub struct Node {
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
        // The route's server built `{ route, tool_name, arguments }`.
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: super::answer_under(ANSWER_KEY, input.payload),
            trace: vec![format!(
                "trigger.mcp: route={} tool={}",
                self.config.route, self.config.name
            )],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters_become_the_tool_input_schema() {
        let config = json!({
            "schema": {
                "type": "object",
                "properties": { "q": { "type": "string", "description": "words" }, "limit": { "type": "integer" } },
                "required": ["q"]
            }
        });
        let schema = input_schema_from_config(&config);
        assert_eq!(schema["properties"]["q"]["type"], "string");
        assert_eq!(schema["required"], json!(["q"]));
        assert_eq!(input_schema_from_config(&json!({}))["properties"], json!({}));
    }

    #[test]
    fn tool_names_and_routes_are_read_one_way() {
        assert!(valid_tool_name("search"));
        assert!(valid_tool_name("catalog.search-v2"));
        assert!(!valid_tool_name(""));
        assert!(!valid_tool_name("two words"));
        assert_eq!(normalize_route("/mcp/shop/"), "/mcp/shop");
        assert_eq!(normalize_route("mcp/shop"), "/mcp/shop");
        assert_eq!(normalize_route("/"), "/");
    }

    #[test]
    fn auth_is_required_and_closed_without_hmac() {
        let def = definition();
        let auth = def.dsl_flags.iter().find(|f| f.flag == "--auth").expect("--auth");
        assert!(auth.required);
        assert_eq!(auth.choices, vec!["none", "jwt", "api_key"]);
        assert!(def.dsl_flags.iter().all(|f| f.flag != "--params"));
    }
}

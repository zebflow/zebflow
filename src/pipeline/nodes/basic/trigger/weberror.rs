//! `n.trigger.weberror` — trigger a pipeline when an HTTP error occurs.
//!
//! This is a **routing declaration**, not an active processor.  The platform
//! checks for matching weberror pipelines when:
//!
//! - A route is not found (404)
//! - A pipeline sets `_status: 4xx/5xx` in its output
//! - A pipeline execution fails (500)
//!
//! The node itself is a passthrough — the error context flows downstream unchanged.
//!
//! # Config flags
//!
//! | Flag | Type | Default | Description |
//! |---|---|---|---|
//! | `--code` | string | `"*"` | Error code pattern to match |
//!
//! # Code patterns
//!
//! | Pattern | Matches |
//! |---|---|
//! | `"404"` | Exactly HTTP 404 |
//! | `"401"` | Exactly HTTP 401 |
//! | `"4xx"` | Any 400–499 |
//! | `"5xx"` | Any 500–599 |
//! | `"*"` or `""` | Any error (catch-all) |
//!
//! Exact matches take priority over ranges, which take priority over catch-all.
//!
//! # Injected payload fields
//!
//! | Field | Type | Description |
//! |---|---|---|
//! | `error_code` | integer | HTTP error code (e.g. 404) |
//! | `error_message` | string | Standard HTTP reason phrase |
//! | `original_path` | string | The path that triggered the error |
//! | `method` | string | HTTP method of the original request |
//!
//! # Example pipelines
//!
//! **Custom 404 page:**
//! ```text
//! | n.trigger.weberror --code 404
//! | n.web.render --template pages/error-404
//! ```
//!
//! **Custom unauthorized page:**
//! ```text
//! | n.trigger.weberror --code 401
//! | n.web.render --template pages/error-unauthorized
//! ```
//!
//! **Catch-all error page (5xx):**
//! ```text
//! | n.trigger.weberror --code 5xx
//! | n.web.render --template pages/error-server
//! ```
//!
//! **Catch-all fallback:**
//! ```text
//! | n.trigger.weberror
//! | n.web.render --template pages/error-generic
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::pipeline::{
    PipelineError, NodeDefinition,
    nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput},
};
use crate::pipeline::model::{LayoutItem, NodeFieldDef, NodeFieldType, SelectOptionDef};

pub const NODE_KIND: &str = "n.trigger.weberror";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

/// Return the [`NodeDefinition`] for `n.trigger.weberror`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Web Error Trigger".to_string(),
        description: "Triggers a pipeline when an HTTP error occurs. \
            Use --code to scope to a specific error code (404, 401), \
            a range (4xx, 5xx), or leave empty for a catch-all. \
            The pipeline receives error_code, error_message, original_path, and method. \
            Pair with n.web.render to serve custom HTML error pages."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "error_code":    { "type": "integer", "description": "HTTP error code (e.g. 404)." },
                "error_message": { "type": "string",  "description": "Standard HTTP reason phrase." },
                "original_path": { "type": "string",  "description": "Path that triggered the error." },
                "method":        { "type": "string",  "description": "HTTP method of the original request." }
            }
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "error_code":    { "type": "integer" },
                "error_message": { "type": "string" },
                "original_path": { "type": "string" },
                "method":        { "type": "string" }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: Default::default(),
        fields: vec![
            NodeFieldDef { name: "title".to_string(), label: "Title".to_string(), field_type: NodeFieldType::Text, help: Some("Override display title for this node.".to_string()), ..Default::default() },
            NodeFieldDef { name: "code".to_string(), label: "Error Code Pattern".to_string(), field_type: NodeFieldType::Select, options: vec![
                SelectOptionDef { value: "404".to_string(), label: "404 — Not Found".to_string() },
                SelectOptionDef { value: "4xx".to_string(), label: "4xx — Any Client Error".to_string() },
                SelectOptionDef { value: "5xx".to_string(), label: "5xx — Any Server Error".to_string() },
                SelectOptionDef { value: "*".to_string(), label: "* — All errors (catch-all)".to_string() },
            ], help: Some("Which HTTP error code(s) this pipeline handles.".to_string()), ..Default::default() },
        ],
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("title".to_string()), LayoutItem::Field("code".to_string())] },
        ],
        ai_tool: Default::default(),
    }
}

/// Configuration for `n.trigger.weberror`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Error code pattern to match.
    ///
    /// - Exact: `"404"`, `"401"`, `"500"`
    /// - Range: `"4xx"` (400–499), `"5xx"` (500–599)
    /// - Catch-all: `""` or `"*"` (default, matches any error)
    #[serde(default)]
    pub code: String,
}

/// Match specificity — used to select the most specific weberror pipeline.
///
/// Higher value = higher priority.
pub fn match_specificity(code_pattern: &str, error_code: u16) -> Option<u8> {
    if code_pattern.is_empty() || code_pattern == "*" {
        return Some(0); // catch-all
    }
    if code_pattern.eq_ignore_ascii_case("4xx") && (400..500).contains(&error_code) {
        return Some(1);
    }
    if code_pattern.eq_ignore_ascii_case("5xx") && (500..600).contains(&error_code) {
        return Some(1);
    }
    if let Ok(exact) = code_pattern.parse::<u16>() {
        if exact == error_code {
            return Some(2); // exact match — highest priority
        }
    }
    None // no match
}

/// `n.trigger.weberror` node instance.
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
        &[INPUT_PIN_IN]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        // Passthrough — error context was injected by the platform before dispatch.
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: input.payload,
            trace: vec!["n.trigger.weberror: passthrough".to_string()],
        })
    }
}

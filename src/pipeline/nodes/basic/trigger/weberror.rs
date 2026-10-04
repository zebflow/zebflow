//! `trigger.error` — trigger a pipeline when an HTTP error occurs.
//!
//! This is a **routing declaration**, not an active processor.  The platform
//! checks for matching weberror pipelines when:
//!
//! - A route is not found (404)
//! - A route's `--auth` refuses the visitor (401/403)
//! - A pipeline execution fails (500)
//!
//! It answers through `web.response.send`; one that answers nothing leaves
//! the platform's own error answer — its run's value is never sent.
//!
//! The node answers one key, `error`, holding the error context the platform
//! built (`node-conventions.md` §6).
//!
//! # Config flags
//!
//! | Flag | Type | Default | Description |
//! |---|---|---|---|
//! | `--status` | number or range | `"*"` | HTTP status pattern to match |
//!
//! # Status patterns
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
//! # The answer: `error: { … }`
//!
//! | Field | Type | Description |
//! |---|---|---|
//! | `error_code` | integer | HTTP error code (e.g. 404) |
//! | `error_message` | string | Standard HTTP reason phrase |
//! | `original_path` | string | The path that triggered the error |
//! | `method` | string | HTTP method of the original request |
//! | `request_id` | string | The run id of the failed request |
//!
//! # Example pipelines
//!
//! **Custom 404 page:**
//! ```text
//! | trigger.error --status 404
//! | web.response.send --template pages/error-404.tsx
//! ```
//!
//! **Custom unauthorized page:**
//! ```text
//! | trigger.error --status 401
//! | web.response.send --template pages/error-unauthorized.tsx
//! ```
//!
//! **Catch-all error page (5xx):**
//! ```text
//! | trigger.error --status 5xx
//! | web.response.send --template pages/error-server.tsx
//! ```
//!
//! **Catch-all fallback:**
//! ```text
//! | trigger.error
//! | web.response.send --template pages/error-generic.tsx
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::pipeline::model::{
    DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType, SelectOptionDef,
};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "trigger.error";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";
/// The key this trigger answers under.
pub const ANSWER_KEY: &str = "error";

/// Return the [`NodeDefinition`] for `trigger.error`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Web Error Trigger".to_string(),
        description: "Runs when a request to this project ends in an HTTP error that no pipeline answered. It covers a `/wh/…` path nobody \
            registered (404), a refused auth (401/403), a failed node (500). `--status` picks which: `404`, `4xx`, `5xx`, or empty for \
            all. Answers one key, `error`: `{ error_code, error_message, original_path, method, request_id }` \
            (`input.error.error_code`, `$trigger.error_code`) — there is no `body`. End in \
            `web.response.send --template pages/not-found.tsx --status 404` to serve a designed error page; without `--status` the page \
            answers 200 and browsers and crawlers treat the error as a success. One pipeline per code range; the most specific wins."
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
                "error": {
                    "type": "object",
                    "properties": {
                        "error_code":    { "type": "integer" },
                        "error_message": { "type": "string" },
                        "original_path": { "type": "string" },
                        "method":        { "type": "string" },
                        "request_id":    { "type": "string" }
                    }
                }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![DslFlag {
            flag: "--status".to_string(),
            config_key: "status".to_string(),
            description:
                "The HTTP status this pipeline answers: an exact status (404), a range (4xx, 5xx), or empty for every error."
                    .to_string(),
            kind: DslFlagKind::Scalar,
            required: false,
            value: "text".to_string(),
            ..Default::default()
        }],
        fields: vec![NodeFieldDef {
            name: "status".to_string(),
            label: "Status".to_string(),
            field_type: NodeFieldType::Select,
            options: vec![
                SelectOptionDef {
                    value: "404".to_string(),
                    label: "404 — Not Found".to_string(),
                },
                SelectOptionDef {
                    value: "4xx".to_string(),
                    label: "4xx — Any Client Error".to_string(),
                },
                SelectOptionDef {
                    value: "5xx".to_string(),
                    label: "5xx — Any Server Error".to_string(),
                },
                SelectOptionDef {
                    value: "*".to_string(),
                    label: "* — All errors (catch-all)".to_string(),
                },
            ],
            help: Some("Which HTTP error code(s) this pipeline handles.".to_string()),
            ..Default::default()
        }],
        layout: vec![LayoutItem::Field("status".to_string())],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("A designed 404", "trigger.error --status 404")
                .output(serde_json::json!({ "error": { "error_code": 404, "error_message": "Not Found", "original_path": "/blog/old-post", "method": "GET", "request_id": "weberror-404" } }))
                .note("Then `| web.response.send --template pages/not-found.tsx --status 404`."),
        ],
        ..Default::default()
    }
}

/// Configuration for `trigger.error`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// HTTP status pattern to match.
    ///
    /// - Exact: `"404"`, `"401"`, `"500"`
    /// - Range: `"4xx"` (400–499), `"5xx"` (500–599)
    /// - Catch-all: `""` or `"*"` (default, matches any error)
    ///
    /// The DSL types an unquoted `--status 404` as a number, so the field
    /// accepts a number and keeps it as its digits.
    #[serde(default, deserialize_with = "status_from_string_or_number")]
    pub status: String,
}

fn status_from_string_or_number<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let value = serde_json::Value::deserialize(d)?;
    Ok(match value {
        serde_json::Value::String(s) => s,
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Null => String::new(),
        other => return Err(serde::de::Error::custom(format!("status must be a string or a number, got {other}"))),
    })
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

/// `trigger.error` node instance.
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
        &[INPUT_PIN_IN]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        // The error context was injected by the platform before dispatch.
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: super::answer_under(ANSWER_KEY, input.payload),
            trace: vec![format!("trigger.error: status={}", self.config.status)],
        })
    }
}

#[cfg(test)]
mod code_tests {
    use super::*;
    #[test]
    fn status_accepts_the_number_the_dsl_types_it_as() {
        let numeric: Config = serde_json::from_value(serde_json::json!({ "status": 404 })).unwrap();
        assert_eq!(numeric.status, "404");
        let text: Config = serde_json::from_value(serde_json::json!({ "status": "4xx" })).unwrap();
        assert_eq!(text.status, "4xx");
        let missing: Config = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(missing.status, "");
    }
}

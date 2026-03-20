//! HTTP request node for pulling external/internal data into pipeline flow.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::{
    PipelineError, NodeDefinition,
    nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput},
};
use crate::language::LanguageEngine;

use crate::pipeline::model::LayoutItem;
use super::util::{eval_deno_expr, resolve_path_cloned};

pub const NODE_KIND: &str = "n.http.request";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

/// Unified node-definition metadata for `n.http.request`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "HTTP Request".to_string(),
        description: "Perform HTTP call and return normalized response envelope.".to_string(),
        input_schema: serde_json::json!({
            "type":"object",
            "description":"Input context used for optional body binding."
        }),
        output_schema: serde_json::json!({
            "type":"object",
            "properties":{
                "request":{"type":"object"},
                "response":{"type":"object"}
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: true,
        script_bridge: Some(crate::pipeline::NodeScriptBridge {
            name: "n.http.request".to_string(),
            enabled: false,
        }),
        config_schema: Default::default(),
        dsl_flags: Default::default(),
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType, SelectOptionDef};
            vec![
                NodeFieldDef { name: "title".to_string(), label: "Title".to_string(), field_type: NodeFieldType::Text, help: Some("Override display title for this node.".to_string()), ..Default::default() },
                NodeFieldDef { name: "url".to_string(), label: "URL".to_string(), field_type: NodeFieldType::Text, help: Some("Fallback URL when url_expr is empty.".to_string()), default_value: Some(serde_json::json!("https://example.com")), ..Default::default() },
                NodeFieldDef { name: "method".to_string(), label: "Method".to_string(), field_type: NodeFieldType::Select, options: vec!["GET","POST","PUT","PATCH","DELETE"].iter().map(|m| SelectOptionDef { value: m.to_string(), label: m.to_string() }).collect(), help: Some("Fallback HTTP method when method_expr is empty.".to_string()), ..Default::default() },
                NodeFieldDef { name: "timeout_ms".to_string(), label: "Timeout (ms)".to_string(), field_type: NodeFieldType::Text, help: Some("Request timeout in milliseconds.".to_string()), ..Default::default() },
                NodeFieldDef { name: "url_expr".to_string(), label: "URL Expr".to_string(), field_type: NodeFieldType::Textarea, rows: Some(3), help: Some("Optional JS expression returning string URL.".to_string()), ..Default::default() },
                NodeFieldDef { name: "method_expr".to_string(), label: "Method Expr".to_string(), field_type: NodeFieldType::Textarea, rows: Some(3), help: Some("Optional JS expression returning string method.".to_string()), ..Default::default() },
                NodeFieldDef { name: "body_path".to_string(), label: "Body Path".to_string(), field_type: NodeFieldType::Text, help: Some("Payload path used as request body when body_expr is empty.".to_string()), ..Default::default() },
                NodeFieldDef { name: "headers_expr".to_string(), label: "Headers Expr".to_string(), field_type: NodeFieldType::Textarea, rows: Some(4), help: Some("JS expression returning header object.".to_string()), ..Default::default() },
                NodeFieldDef { name: "body_expr".to_string(), label: "Body Expr".to_string(), field_type: NodeFieldType::Textarea, rows: Some(4), help: Some("JS expression returning request body value.".to_string()), ..Default::default() },
            ]
        },
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("title".to_string()), LayoutItem::Field("method".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("url".to_string()), LayoutItem::Field("timeout_ms".to_string())] },
            LayoutItem::Field("body_path".to_string()),
            LayoutItem::Field("url_expr".to_string()),
            LayoutItem::Field("method_expr".to_string()),
            LayoutItem::Field("headers_expr".to_string()),
            LayoutItem::Field("body_expr".to_string()),
        ],
        ai_tool: Default::default(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub url: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub body_path: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub url_expr: Option<String>,
    #[serde(default)]
    pub method_expr: Option<String>,
    #[serde(default)]
    pub headers_expr: Option<String>,
    #[serde(default)]
    pub body_expr: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            url: String::new(),
            method: default_method(),
            headers: BTreeMap::new(),
            body_path: None,
            timeout_ms: None,
            url_expr: None,
            method_expr: None,
            headers_expr: None,
            body_expr: None,
        }
    }
}

fn default_method() -> String {
    "GET".to_string()
}

pub struct Node {
    config: Config,
    language: Arc<dyn LanguageEngine>,
}

impl Node {
    pub fn new(config: Config, language: Arc<dyn LanguageEngine>) -> Result<Self, PipelineError> {
        let url = config.url.trim();
        let url_expr_empty = config
            .url_expr
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty();
        if url.is_empty() && url_expr_empty {
            return Err(PipelineError::new(
                "FW_NODE_HTTP_REQUEST_CONFIG",
                "config.url must not be empty",
            ));
        }
        if !url.is_empty() && !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(PipelineError::new(
                "FW_NODE_HTTP_REQUEST_CONFIG",
                "config.url must start with http:// or https://",
            ));
        }
        Ok(Self { config, language })
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
        if input.input_pin != INPUT_PIN_IN {
            return Err(PipelineError::new(
                "FW_NODE_HTTP_REQUEST_INPUT_PIN",
                format!("unsupported input pin '{}'", input.input_pin),
            ));
        }

        let timeout_ms = self.config.timeout_ms.unwrap_or(10_000).clamp(100, 120_000);
        let agent: ureq::Agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_millis(timeout_ms))
            .build();

        let url = resolve_http_string_binding(
            self.language.as_ref(),
            &input.payload,
            &input.metadata,
            self.config.url_expr.as_deref(),
            &self.config.url,
            "url",
        )?;
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(PipelineError::new(
                "FW_NODE_HTTP_REQUEST_CONFIG",
                "resolved url must start with http:// or https://",
            ));
        }
        let method = resolve_http_string_binding(
            self.language.as_ref(),
            &input.payload,
            &input.metadata,
            self.config.method_expr.as_deref(),
            &self.config.method,
            "method",
        )?
        .to_uppercase();
        let mut request = agent.request(&method, &url);
        let headers = if let Some(expr) = self.config.headers_expr.as_deref() {
            let value = eval_deno_expr(
                self.language.as_ref(),
                expr,
                &input.payload,
                &input.metadata,
            )?;
            parse_headers(value)?
        } else {
            self.config.headers.clone()
        };
        for (key, value) in &headers {
            request = request.set(key, value);
        }

        let body_value = if let Some(expr) = self.config.body_expr.as_deref() {
            Some(eval_deno_expr(
                self.language.as_ref(),
                expr,
                &input.payload,
                &input.metadata,
            )?)
        } else {
            resolve_path_cloned(&input.payload, self.config.body_path.as_deref())
        };
        let response = match body_value {
            Some(body) if !matches!(method.as_str(), "GET" | "HEAD" | "DELETE" | "OPTIONS") => {
                request.send_string(&body.to_string())
            }
            _ => request.call(),
        };

        let response = match response {
            Ok(resp) => resp,
            Err(ureq::Error::Status(_status, resp)) => resp,
            Err(ureq::Error::Transport(err)) => {
                return Err(PipelineError::new(
                    "FW_NODE_HTTP_REQUEST_TRANSPORT",
                    err.to_string(),
                ));
            }
        };

        let status = response.status();
        let content_type = response
            .header("content-type")
            .unwrap_or("application/octet-stream")
            .to_string();
        let body_text = response.into_string().map_err(|err| {
            PipelineError::new("FW_NODE_HTTP_REQUEST_READ_BODY", err.to_string())
        })?;
        let body = serde_json::from_str::<Value>(&body_text).unwrap_or(Value::String(body_text));

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({
                "request": {
                    "url": url,
                    "method": method,
                    "timeout_ms": timeout_ms
                },
                "response": {
                    "status": status,
                    "ok": (200..400).contains(&status),
                    "content_type": content_type,
                    "body": body
                }
            }),
            trace: vec![format!("node_kind={NODE_KIND}")],
        })
    }
}

fn resolve_http_string_binding(
    language: &dyn LanguageEngine,
    input: &Value,
    metadata: &Value,
    expr: Option<&str>,
    fallback: &str,
    field: &str,
) -> Result<String, PipelineError> {
    if let Some(expr) = expr {
        let value = eval_deno_expr(language, expr, input, metadata)?;
        return value.as_str().map(ToString::to_string).ok_or_else(|| {
            PipelineError::new(
                "FW_NODE_HTTP_REQUEST_BINDING",
                format!("binding expression for '{field}' must return string"),
            )
        });
    }
    let out = fallback.trim();
    if out.is_empty() {
        return Err(PipelineError::new(
            "FW_NODE_HTTP_REQUEST_BINDING",
            format!("resolved '{field}' must not be empty"),
        ));
    }
    Ok(out.to_string())
}

fn parse_headers(value: Value) -> Result<BTreeMap<String, String>, PipelineError> {
    let mut out = BTreeMap::new();
    let Value::Object(map) = value else {
        return Err(PipelineError::new(
            "FW_NODE_HTTP_REQUEST_BINDING",
            "headers_expr must return object",
        ));
    };
    for (k, v) in map {
        let Some(s) = v.as_str() else {
            return Err(PipelineError::new(
                "FW_NODE_HTTP_REQUEST_BINDING",
                "headers_expr values must be strings",
            ));
        };
        out.insert(k, s.to_string());
    }
    Ok(out)
}

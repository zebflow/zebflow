//! Webhook trigger node.
//!
//! # Pipeline position
//!
//! Always the first node in a webhook-triggered pipeline. Owns the HTTP route.
//! The request path flows as `PipelineContext.route` → node metadata `"route"` to
//! downstream nodes (specifically `n.web.render`).
//!
//! ```text
//! | n.trigger.webhook --path /blog --method GET
//! | pg.query --credential main-db -- "SELECT ..."
//! | n.web.render --template-path pages/blog-home
//! ```

use crate::pipeline::{
    PipelineError, NodeDefinition,
    nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput},
};
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType, NodeFieldDataSource, SelectOptionDef};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub const NODE_KIND: &str = "n.trigger.webhook";
pub const OUTPUT_PIN_OUT: &str = "out";

/// Unified node-definition metadata for `n.trigger.webhook`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Webhook Trigger".to_string(),
        description: "Start pipeline run from inbound HTTP path + method. \
            Use --auth-type jwt/hmac/api_key and --auth-credential <id> to protect the route. \
            jwt auth checks Authorization: Bearer header first, then Cookie: zebflow_session fallback — \
            verified claims are injected into input.auth. \
            Output with _status sets HTTP response status code. \
            Output with _set_cookie sets an HttpOnly cookie in the response.".to_string(),
        input_schema: serde_json::json!({
            "type":"object",
            "description":"Request payload forwarded from webhook ingress."
        }),
        output_schema: serde_json::json!({
            "type":"object",
            "description":"Unmodified request payload for downstream nodes."
        }),
        input_pins: vec![],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: serde_json::json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": {
                    "type": "string",
                    "description": "HTTP path this webhook listens on, e.g. '/blog' or '/api/users/:id'. Must start with /."
                },
                "method": {
                    "type": "string",
                    "enum": ["GET", "POST", "PUT", "PATCH", "DELETE"],
                    "description": "HTTP method. Defaults to GET."
                },
                "auth_type": {
                    "type": "string",
                    "enum": ["none", "jwt", "hmac", "api_key"],
                    "description": "Authentication mode. none = open (default). jwt/hmac/api_key require auth_credential."
                },
                "auth_credential": {
                    "type": "string",
                    "description": "Credential ID used for auth verification. Required when auth_type is not none."
                }
            }
        }),
        fields: vec![
            NodeFieldDef { name: "title".to_string(), label: "Title".to_string(), field_type: NodeFieldType::Text, help: Some("Override display title for this node.".to_string()), ..Default::default() },
            NodeFieldDef { name: "method".to_string(), label: "Method".to_string(), field_type: NodeFieldType::MethodButtons, options: vec!["GET","POST","PUT","PATCH","DELETE"].iter().map(|m| SelectOptionDef { value: m.to_string(), label: m.to_string() }).collect(), help: Some("HTTP method accepted by webhook trigger.".to_string()), ..Default::default() },
            NodeFieldDef { name: "path".to_string(), label: "Path".to_string(), field_type: NodeFieldType::Text, help: Some("Webhook relative path under /wh/{owner}/{project}.".to_string()), ..Default::default() },
            NodeFieldDef { name: "__webhook_public_url".to_string(), label: "Public URL".to_string(), field_type: NodeFieldType::CopyUrl, help: Some("Copy-ready URL for this trigger.".to_string()), ..Default::default() },
            NodeFieldDef { name: "auth_type".to_string(), label: "Auth Type".to_string(), field_type: NodeFieldType::Select, options: vec![
                SelectOptionDef { value: "none".to_string(), label: "None (public)".to_string() },
                SelectOptionDef { value: "jwt".to_string(), label: "JWT Bearer".to_string() },
                SelectOptionDef { value: "hmac".to_string(), label: "HMAC-SHA256 (X-Hub-Signature-256)".to_string() },
                SelectOptionDef { value: "api_key".to_string(), label: "API Key (X-API-Key)".to_string() },
            ], help: Some("Trigger-level auth. On failure returns 401.".to_string()), ..Default::default() },
            NodeFieldDef { name: "auth_credential".to_string(), label: "Auth Credential".to_string(), field_type: NodeFieldType::Select, data_source: Some(NodeFieldDataSource::CredentialsJwt), help: Some("Credential for signing key / secret / api_key.".to_string()), ..Default::default() },
        ],
        dsl_flags: vec![
            DslFlag {
                flag: "--path".to_string(),
                config_key: "path".to_string(),
                description: "HTTP path this webhook listens on. Must start with /. Examples: /blog, /api/users/:id.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--method".to_string(),
                config_key: "method".to_string(),
                description: "HTTP method: GET (default), POST, PUT, PATCH, DELETE.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--auth-type".to_string(),
                config_key: "auth_type".to_string(),
                description: "Authentication mode: none (default), jwt, hmac, api_key.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--auth-credential".to_string(),
                config_key: "auth_credential".to_string(),
                description: "Credential ID for auth verification. Required when auth_type != none.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("title".to_string()), LayoutItem::Field("path".to_string())] },
            LayoutItem::Field("method".to_string()),
            LayoutItem::Field("__webhook_public_url".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("auth_type".to_string()), LayoutItem::Field("auth_credential".to_string())] },
        ],
        ai_tool: Default::default(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub path: String,
    #[serde(default = "default_method")]
    pub method: String,
    /// Auth type: `"none"` (default), `"jwt"`, `"hmac"`, `"api_key"`.
    ///
    /// - `jwt`     — verifies `Authorization: Bearer <token>` against a `jwt_signing_key` credential.
    ///               Verified claims are injected into `payload.auth`.
    /// - `hmac`    — verifies `X-Hub-Signature-256: sha256=<hex>` (GitHub-style) against a credential.
    /// - `api_key` — verifies `X-API-Key: <key>` or `Authorization: ApiKey <key>` against a credential.
    /// - `none`    — no authentication (default).
    #[serde(default)]
    pub auth_type: String,
    /// Credential ID to use for auth verification (required when `auth_type != "none"`).
    #[serde(default)]
    pub auth_credential: String,
}

fn default_method() -> String {
    "GET".to_string()
}

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
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: input.payload,
            trace: vec![
                format!("node_kind={NODE_KIND}"),
                format!("method={}", self.config.method),
                format!("path={}", self.config.path),
            ],
        })
    }
}

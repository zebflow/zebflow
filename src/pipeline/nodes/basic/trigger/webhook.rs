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
use crate::pipeline::model::{DslFlag, DslFlagKind};
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
            JWT claims are injected into payload.auth. \
            Output with _status sets the HTTP response status code.".to_string(),
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

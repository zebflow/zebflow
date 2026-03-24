//! `n.browser.run` — execute a Playwright/Puppeteer script via a Browserless-compatible HTTP endpoint.
//!
//! # Pipeline position
//! Middleware node. Requires an upstream trigger and produces a downstream payload.
//!
//! # User-facing config
//! | Field | Type | Required | Description |
//! |---|---|---|---|
//! | `credential_id` | string | ✓ | ID of a `browser_*` credential (kind prefix `browser_`) |
//! | `code` | string | ✓ | `async ({ page }) => { ... }` function body sent to the browser endpoint |
//!
//! # How it works
//! 1. Resolves the credential by `credential_id`.
//! 2. Reads `secret.url` (Browserless root URL) and optional `secret.token`.
//! 3. POST `<url>/function?token=<token>` with `{ "code": "<user code>" }`.
//! 4. Returns the JSON response as the node output payload.
//!
//! # DSL
//! ```text
//! | n.trigger.webhook --path /scrape
//! | n.browser.run --credential browserless-local
//! ```

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::{
    PipelineError, NodeDefinition,
    nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput},
};
use crate::platform::services::CredentialService;
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType, NodeFieldDataSource, SidebarSection, SidebarItem};

pub const NODE_KIND: &str = "n.browser.run";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

const BROWSER_KIND_PREFIX: &str = "browser_";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Browser Run".to_string(),
        description: "Execute a Playwright script via a Browserless-compatible HTTP endpoint and return the result.".to_string(),
        input_schema: json!({ "type": "object", "description": "Upstream payload available as context." }),
        output_schema: json!({ "type": "object", "description": "JSON result returned by the browser script." }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag { flag: "--credential".to_string(), config_key: "credential_id".to_string(), description: "Credential ID of the browser connection (kind: browser_*).".to_string(), kind: DslFlagKind::Scalar, required: true },
        ],
        fields: vec![
            NodeFieldDef { name: "title".to_string(), label: "Title".to_string(), field_type: NodeFieldType::Text, help: Some("Override display title for this node.".to_string()), ..Default::default() },
            NodeFieldDef { name: "credential_id".to_string(), label: "Credential".to_string(), field_type: NodeFieldType::Select, data_source: Some(NodeFieldDataSource::CredentialsBrowser), help: Some("Browser credential (kind: browser_browserless or similar).".to_string()), ..Default::default() },
            NodeFieldDef {
                name: "code".to_string(),
                label: "Script".to_string(),
                field_type: NodeFieldType::CodeEditor,
                language: Some("javascript".to_string()),
                span: Some("full".to_string()),
                help: Some("async ({ page }) => { ... } — Playwright/Puppeteer page function. Return value becomes the node output.".to_string()),
                default_value: Some(json!("async ({ page }) => {\n  await page.goto('https://example.com');\n  return { title: await page.title() };\n}")),
                sidebar: vec![
                    SidebarSection {
                        title: "API".to_string(),
                        items: vec![
                            SidebarItem { label: "page".to_string(), type_hint: Some("Page".to_string()), description: Some("Playwright/Puppeteer page object.".to_string()) },
                        ],
                    },
                    SidebarSection {
                        title: "Return".to_string(),
                        items: vec![
                            SidebarItem { label: "any JSON".to_string(), type_hint: Some("object | array | string | number".to_string()), description: Some("Returned value becomes the downstream payload.".to_string()) },
                        ],
                    },
                ],
                ..Default::default()
            },
            NodeFieldDef { name: "timeout_ms".to_string(), label: "Timeout (ms)".to_string(), field_type: NodeFieldType::Text, help: Some("HTTP request timeout in milliseconds. Default 60000.".to_string()), ..Default::default() },
        ],
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("title".to_string()), LayoutItem::Field("credential_id".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("timeout_ms".to_string())] },
            LayoutItem::Field("code".to_string()),
        ],
        ai_tool: Default::default(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub credential_id: String,
    pub code: String,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

pub struct Node {
    config: Config,
    credentials: Arc<CredentialService>,
}

impl Node {
    pub fn new(config: Config, credentials: Arc<CredentialService>) -> Result<Self, PipelineError> {
        if config.credential_id.trim().is_empty() {
            return Err(PipelineError::new("FW_NODE_BROWSER_RUN_CONFIG", "config.credential_id must not be empty"));
        }
        if config.code.trim().is_empty() {
            return Err(PipelineError::new("FW_NODE_BROWSER_RUN_CONFIG", "config.code must not be empty"));
        }
        Ok(Self { config, credentials })
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str { NODE_KIND }
    fn input_pins(&self) -> &'static [&'static str] { &[INPUT_PIN_IN] }
    fn output_pins(&self) -> &'static [&'static str] { &[OUTPUT_PIN_OUT] }

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        if input.input_pin != INPUT_PIN_IN {
            return Err(PipelineError::new("FW_NODE_BROWSER_RUN_PIN", format!("unsupported input pin '{}'", input.input_pin)));
        }

        let owner = input.metadata.get("owner").and_then(Value::as_str).unwrap_or_default();
        let project = input.metadata.get("project").and_then(Value::as_str).unwrap_or_default();

        let credential = self.credentials
            .get_project_credential(owner, project, &self.config.credential_id)
            .map_err(|e| PipelineError::new("FW_NODE_BROWSER_RUN_CREDENTIAL", e.to_string()))?
            .ok_or_else(|| PipelineError::new("FW_NODE_BROWSER_RUN_CREDENTIAL_MISSING", format!("credential '{}' not found", self.config.credential_id)))?;

        if !credential.kind.starts_with(BROWSER_KIND_PREFIX) {
            return Err(PipelineError::new(
                "FW_NODE_BROWSER_RUN_CREDENTIAL_KIND",
                format!("credential '{}' has kind '{}' — expected kind starting with '{}'", credential.credential_id, credential.kind, BROWSER_KIND_PREFIX),
            ));
        }

        let base_url = credential.secret.get("url").and_then(Value::as_str).unwrap_or_default().trim_end_matches('/');
        if base_url.is_empty() {
            return Err(PipelineError::new("FW_NODE_BROWSER_RUN_SECRET", "credential secret.url is required"));
        }
        let token = credential.secret.get("token").and_then(Value::as_str).unwrap_or("").trim();

        let endpoint = if token.is_empty() {
            format!("{}/function", base_url)
        } else {
            format!("{}/function?token={}", base_url, token)
        };

        let timeout_ms = self.config.timeout_ms.unwrap_or(60_000).clamp(1_000, 300_000);
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_millis(timeout_ms))
            .build();

        let body = json!({ "code": self.config.code });

        let response = agent
            .post(&endpoint)
            .set("Content-Type", "application/json")
            .send_string(&body.to_string())
            .map_err(|e| PipelineError::new("FW_NODE_BROWSER_RUN_HTTP", e.to_string()))?;

        let status = response.status();
        let body_text = response.into_string()
            .map_err(|e| PipelineError::new("FW_NODE_BROWSER_RUN_READ", e.to_string()))?;

        let payload = serde_json::from_str::<Value>(&body_text).unwrap_or(Value::String(body_text));

        if !(200..400).contains(&status) {
            return Err(PipelineError::new(
                "FW_NODE_BROWSER_RUN_STATUS",
                format!("browser endpoint returned status {}: {}", status, serde_json::to_string(&payload).unwrap_or_default()),
            ));
        }

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload,
            trace: vec![format!("node_kind={NODE_KIND} credential={} status={}", self.config.credential_id, status)],
        })
    }
}

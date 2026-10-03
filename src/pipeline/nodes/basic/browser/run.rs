//! `browser.page.run` — execute a Playwright/Puppeteer script via a Browserless-compatible HTTP endpoint.
//!
//! # Pipeline position
//! Middleware node. Requires an upstream trigger and produces a downstream payload.
//!
//! # User-facing config
//! | Field | Type | Required | Description |
//! |---|---|---|---|
//! | `credential_id` | string | ✓ | ID of a `browser_*` credential (kind prefix `browser_`) |
//! | `code` | string | ✓ | `export default async ({ page }) => { ... }` ESM function sent to the browser endpoint |
//!
//! # How it works
//! 1. Resolves the credential by `credential_id`.
//! 2. Reads `secret.url` (Browserless root URL) and optional `secret.token`.
//! 3. POST `<url>/function?token=<token>` with `{ "code": "<user code>" }`.
//! 4. Adds what the script returned as `page`, keeping the rest of the payload.
//!
//! The call has no timeout of its own: the engine's `--timeout` bounds the
//! node, and dropping the request closes the connection.
//!
//! # DSL
//! ```text
//! | trigger.webhook --route /scrape
//! | browser.page.run --credential browserless-local --timeout 60s -- "export default async ({ page }) => …"
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::model::NodeCapability;
use crate::pipeline::model::{
    DslFlag, DslFlagKind, LayoutItem, NodeFieldDataSource, NodeFieldDef, NodeFieldType,
    SidebarItem, SidebarSection,
};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::CredentialService;

pub const NODE_KIND: &str = "browser.page.run";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

const BROWSER_KIND_PREFIX: &str = "browser_";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Network, NodeCapability::Credential, NodeCapability::Process],
        title: "Browser Run".to_string(),
        description: "Runs a Playwright script in a headless browser reached through a `browser_*` credential (a Browserless-compatible \
            endpoint the owner configured) — screenshots, PDF of a live page, scraping a page that needs JavaScript. The body after \
            `--` is the script, an ESM `export default async ({ page }) => { … }`; put payload values into it with `{{ expr }}`. \
            Whatever it returns is added to the payload as `page`, and the rest is kept. The engine's `--timeout` bounds the run \
            (e.g. `--timeout 2m`). This is not for verifying your own pages during development — that is the agent's own browser \
            (`zebflow-verify`)."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `page` is added." }),
        output_schema: json!({ "type": "object", "properties": { "page": { "description": "What the browser script returned (JSON, or text)." } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag { flag: "--credential".to_string(), config_key: "credential_id".to_string(), description: "The browser connection's credential (kind browser_*).".to_string(), kind: DslFlagKind::Scalar, required: true, value: "text".to_string(), ..Default::default() },
        ],
        fields: vec![
            NodeFieldDef { name: "credential_id".to_string(), label: "Credential".to_string(), field_type: NodeFieldType::Select, data_source: Some(NodeFieldDataSource::CredentialsBrowser), help: Some("Browser credential (kind: browser_browserless or similar).".to_string()), ..Default::default() },
            NodeFieldDef {
                name: "code".to_string(),
                label: "Script".to_string(),
                field_type: NodeFieldType::CodeEditor,
                language: Some("javascript".to_string()),
                span: Some("full".to_string()),
                help: Some("export default async ({ page }) => { ... } — Browserless /function script. Return value becomes the node output.".to_string()),
                default_value: Some(json!("export default async ({ page }) => {\n  await page.goto('https://example.com');\n  return {\n    data: { title: await page.title() },\n    type: 'application/json'\n  };\n};")),
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
                            SidebarItem { label: "any JSON".to_string(), type_hint: Some("object | array | string | number".to_string()), description: Some("Returned value becomes `input.page` downstream.".to_string()) },
                        ],
                    },
                ],
                ..Default::default()
            },
        ],
        layout: vec![
            LayoutItem::Field("credential_id".to_string()),
            LayoutItem::Field("code".to_string()),
        ],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Title of a rendered page", r#"browser.page.run --credential browserless_main --timeout 20s -- "export default async ({ page }) => { await page.goto('{{ $trigger.body.url }}'); return { title: await page.title() }; }""#)
                .output(serde_json::json!({ "page": { "title": "Example Domain" } }))
                .note("Read it downstream as `input.page.title`."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub credential_id: String,
    pub code: String,
}

pub struct Node {
    config: Config,
    credentials: Arc<CredentialService>,
    /// Declared hosts of the node bundle this run happens inside, if any.
    bundle_egress: Option<Arc<crate::pipeline::security::BundleEgress>>,
}

impl Node {
    pub fn new(
        config: Config,
        credentials: Arc<CredentialService>,
        bundle_egress: Option<Arc<crate::pipeline::security::BundleEgress>>,
    ) -> Result<Self, PipelineError> {
        if config.credential_id.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_BROWSER_PAGE_RUN_CONFIG",
                "config.credential_id must not be empty",
            ));
        }
        if config.code.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_BROWSER_PAGE_RUN_CONFIG",
                "config.code must not be empty",
            ));
        }
        Ok(Self {
            config,
            credentials,
            bundle_egress,
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
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        if input.input_pin != INPUT_PIN_IN {
            return Err(PipelineError::new(
                "FW_NODE_BROWSER_PAGE_RUN_PIN",
                format!("unsupported input pin '{}'", input.input_pin),
            ));
        }

        let owner = input
            .metadata
            .get("owner")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let project = input
            .metadata
            .get("project")
            .and_then(Value::as_str)
            .unwrap_or_default();

        let credential = self
            .credentials
            .get_project_credential(owner, project, &self.config.credential_id)
            .map_err(|e| PipelineError::new("FW_NODE_BROWSER_PAGE_RUN_CREDENTIAL", e.to_string()))?
            .ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_BROWSER_PAGE_RUN_CREDENTIAL_MISSING",
                    format!("credential '{}' not found", self.config.credential_id),
                )
            })?;

        if !credential.kind.starts_with(BROWSER_KIND_PREFIX) {
            return Err(PipelineError::new(
                "FW_NODE_BROWSER_PAGE_RUN_CREDENTIAL_KIND",
                format!(
                    "credential '{}' has kind '{}' — expected kind starting with '{}'",
                    credential.credential_id, credential.kind, BROWSER_KIND_PREFIX
                ),
            ));
        }

        let base_url = credential
            .secret
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim_end_matches('/');
        if base_url.is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_BROWSER_PAGE_RUN_SECRET",
                "credential secret.url is required",
            ));
        }
        if let Some(egress) = &self.bundle_egress {
            egress.check_url(base_url, NODE_KIND)?;
        }
        crate::pipeline::security::validate_outbound_http_url(base_url, NODE_KIND)?;
        let token = credential
            .secret
            .get("token")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();

        let endpoint = if token.is_empty() {
            format!("{}/function", base_url)
        } else {
            format!("{}/function?token={}", base_url, token)
        };

        // Async, with no timeout of its own: the engine's `--timeout` drops
        // this future, which a blocking client could not be made to notice.
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| PipelineError::new("FW_NODE_BROWSER_PAGE_RUN_HTTP", e.to_string()))?;
        let response = client
            .post(&endpoint)
            .json(&json!({ "code": self.config.code }))
            .send()
            .await
            .map_err(|e| {
                // The token rides in the query string; an error that quotes the URL must not repeat it.
                let message = e.to_string();
                let message = if token.is_empty() { message } else { message.replace(token, "••••••") };
                PipelineError::new("FW_NODE_BROWSER_PAGE_RUN_HTTP", message)
            })?;

        let status = response.status().as_u16();
        let raw = crate::pipeline::nodes::basic::http::request::read_body_capped(response, "FW_NODE_BROWSER_PAGE_RUN_READ").await?;
        let body_text = String::from_utf8_lossy(&raw).into_owned();

        let payload = serde_json::from_str::<Value>(&body_text).unwrap_or(Value::String(body_text));

        if !(200..400).contains(&status) {
            return Err(PipelineError::new(
                "FW_NODE_BROWSER_PAGE_RUN_STATUS",
                format!(
                    "browser endpoint returned status {}: {}",
                    status,
                    serde_json::to_string(&payload).unwrap_or_default()
                ),
            ));
        }

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: crate::pipeline::nodes::shared::util::with_answer(&input.payload, json!({ "page": payload })),
            trace: vec![format!(
                "node_kind={NODE_KIND} credential={} status={}",
                self.config.credential_id, status
            )],
        })
    }
}

#[cfg(test)]
mod tests {
    /// `--timeout-ms` is gone (the engine's `--timeout` bounds the run) and the
    /// answer is `page`.
    #[test]
    fn the_flags_and_answer_are_the_0_11_ones() {
        let def = super::definition();
        let flags: Vec<&str> = def.dsl_flags.iter().map(|f| f.flag.as_str()).collect();
        assert_eq!(flags, ["--credential"]);
        assert!(def.output_schema["properties"].get("page").is_some());
        assert!(!def.fields.iter().any(|f| f.name == "timeout_ms"));
    }
}

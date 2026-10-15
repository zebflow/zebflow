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
        description: "Runs a Playwright script in a remote headless browser — screenshots, PDFs, scraping. The browser is reached through a `browser_*` credential (a Browserless-compatible \
            endpoint the owner configured) — screenshots, PDF of a live page, scraping a page that needs JavaScript. The body after \
            `--` is the script, an ESM `export default async ({ page, context }) => { … }`; read upstream payload values from `context.input` \
            (and trigger data from `context.trigger`) rather than interpolating strings into JS source. \
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
                help: Some("export default async ({ page, context }) => { ... } — Browserless /function script. Read upstream data from context.input. Return value becomes the node output.".to_string()),
                default_value: Some(json!("export default async ({ page, context }) => {\n  await page.goto('https://example.com');\n  return {\n    data: { title: await page.title() },\n    type: 'application/json'\n  };\n};")),
                sidebar: vec![
                    SidebarSection {
                        title: "API".to_string(),
                        items: vec![
                            SidebarItem { label: "page".to_string(), type_hint: Some("Page".to_string()), description: Some("Playwright/Puppeteer page object.".to_string()) },
                            SidebarItem { label: "context.input".to_string(), type_hint: Some("object".to_string()), description: Some("Structured upstream pipeline payload.".to_string()) },
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
            crate::pipeline::model::NodeExample::dsl("Title of a rendered page", r#"browser.page.run --credential browserless_main --timeout 20s -- "export default async ({ page, context }) => { await page.goto(context.input.url); return { title: await page.title() }; }""#)
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

fn build_browserless_endpoint(base_url: &str, token: &str) -> Result<reqwest::Url, PipelineError> {
    let raw = format!("{}/function", base_url.trim_end_matches('/'));
    let mut url = reqwest::Url::parse(&raw)
        .map_err(|e| PipelineError::new("FW_NODE_BROWSER_PAGE_RUN_SECRET", format!("invalid browser url: {e}")))?;
    if !token.is_empty() {
        url.query_pairs_mut().append_pair("token", token);
    }
    Ok(url)
}

fn build_browserless_request_body(code: &str, input: &NodeExecutionInput) -> Value {
    json!({
        "code": code,
        "context": {
            "input": input.payload,
            "trigger": input.metadata.get("trigger").cloned().unwrap_or(Value::Null),
        }
    })
}

fn mask_browser_token(message: &str, token: &str) -> String {
    if token.is_empty() {
        return message.to_string();
    }
    let mut masked = message.replace(token, "••••••");
    let mut probe = reqwest::Url::parse("https://example.invalid/function").expect("static url");
    probe.query_pairs_mut().append_pair("token", token);
    if let Some(q) = probe.query().and_then(|q| q.strip_prefix("token=")) {
        if !q.is_empty() {
            masked = masked.replace(q, "••••••");
        }
    }
    masked
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

        let endpoint = build_browserless_endpoint(base_url, token)?;

        // Async, with no timeout of its own: the engine's `--timeout` bounds
        // this future, and HTTP redirects are disabled so a validated endpoint
        // cannot redirect to an internal address or leak `?token=` to a third party.
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| PipelineError::new("FW_NODE_BROWSER_PAGE_RUN_HTTP", e.to_string()))?;
        let req_body = build_browserless_request_body(&self.config.code, &input);
        let response = client
            .post(endpoint)
            .json(&req_body)
            .send()
            .await
            .map_err(|e| {
                // The token rides in the query string; an error that quotes the URL must not repeat it.
                let message = mask_browser_token(&e.to_string(), token);
                PipelineError::new("FW_NODE_BROWSER_PAGE_RUN_HTTP", message)
            })?;

        let status = response.status().as_u16();
        let raw = crate::pipeline::security::read_response_capped(
            response,
            crate::pipeline::nodes::shared::project_store::MAX_NODE_OBJECT_BYTES,
            "FW_NODE_BROWSER_PAGE_RUN_READ",
        )
        .await?;
        let body_text = String::from_utf8_lossy(&raw).into_owned();

        let payload = serde_json::from_str::<Value>(&body_text).unwrap_or(Value::String(body_text));

        if !(200..300).contains(&status) {
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
    use super::*;

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

    #[test]
    fn browser_page_run_url_encodes_token_disables_redirects_and_passes_structured_context() {
        let token = format!("{}&inject=1#frag", uuid::Uuid::new_v4());
        let url = build_browserless_endpoint("https://browserless.example.com/", &token).expect("endpoint");
        assert_eq!(url.path(), "/function");
        assert!(url.fragment().is_none(), "fragment must be percent-encoded inside token param");
        let pairs: Vec<(String, String)> = url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        assert_eq!(pairs, vec![("token".to_string(), token.clone())]);

        let masked = mask_browser_token(&format!("request to {url} failed"), &token);
        assert!(!masked.contains(&token));
        assert!(!masked.contains("inject=1"));

        let input = NodeExecutionInput {
            node_id: "b".to_string(),
            input_pin: INPUT_PIN_IN.to_string(),
            payload: json!({ "url": "https://example.com/?q=';alert(1)//" }),
            metadata: json!({ "trigger": { "kind": "webhook" } }),
            bus: None,
        };
        let body = build_browserless_request_body(
            "export default async ({ page, context }) => { await page.goto(context.input.url); }",
            &input,
        );
        assert_eq!(
            body["context"]["input"]["url"],
            json!("https://example.com/?q=';alert(1)//")
        );
        assert_eq!(body["context"]["trigger"]["kind"], json!("webhook"));
    }
}

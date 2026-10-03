//! HTTP request node for pulling external/internal data into pipeline flow.
//!
//! For general node authoring rules, read `src/pipeline/nodes/mod.rs`; for
//! FileRef IR and backend/lifecycle rules, read
//! `src/pipeline/nodes/shared/file_ref.rs`.
//!
//! Binary responses (`--response-type bytes`) are stored as temporary FileRefs.
//! Multipart request bodies can send FileRefs as file parts without re-encoding
//! the bytes into JSON.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::language::LanguageEngine;
use crate::pipeline::model::NodeCapability;
use crate::pipeline::nodes::shared::file_ref::{
    FileRefInput, is_file_ref, read_file_ref_bytes, write_tmp_file_ref,
};
use crate::pipeline::security::{BundleEgress, OutboundHttpPolicy};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::CredentialService;
use crate::platform::services::PlatformService;

use crate::pipeline::nodes::shared::util::{eval_deno_expr, metadata_scope};
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem};
use crate::pipeline::nodes::shared::limits::choice;

pub const NODE_KIND: &str = "http.response.fetch";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

/// The methods `--method` takes; GET when it is not given.
pub const METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
/// How the response body is read (`--parse`): parsed JSON (text when it is
/// not JSON), text, or the bytes stored as a FileRef.
pub const PARSE_WORDS: &[&str] = &["json", "text", "bytes"];
/// How the request body is written (`--format`).
pub const FORMAT_WORDS: &[&str] = &["json", "text", "form-data"];

/// A flag set wrong: a choice not on its list, a URL that is not http(s).
pub const CONFIG_CODE: &str = "FW_NODE_HTTP_RESPONSE_FETCH_CONFIG";

/// Unified node-definition metadata for `http.response.fetch`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Network, NodeCapability::Filesystem, NodeCapability::Credential],
        title: "HTTP Request".to_string(),
        description: "Calls another server over HTTP — the only way a pipeline reaches the outside (a script's `fetch` is blocked). \
            `--url`, `--method` (GET default), `--body \"{{ expr }}\"` written as `--format json` (default), `text` or `form-data` \
            (a FileRef is a file part), `--header K=V` repeatable (a repeated name is sent twice), `--credential <id>` for a \
            secure_request or oauth2 credential so secrets never sit in the pipeline, with `--argument NAME=value` filling a \
            secure_request profile's variables. Adds `response: { status, ok, headers, content_type, body, request }` and keeps the \
            rest of the payload — the answer is `input.response.body`, read as `--parse json` (default; text when it is not JSON), \
            `text`, or `bytes` (stored as a FileRef). A non-2xx does not fail the node: check `input.response.ok` with `logic.if`. \
            The engine's `--timeout` bounds the call."
            .to_string(),
        input_schema: serde_json::json!({ "type": "object", "description": "Any payload; it is kept and `response` is added." }),
        output_schema: serde_json::json!({ "type": "object", "properties": { "response": { "type": "object", "properties": {
            "status": { "type": "integer" },
            "ok": { "type": "boolean", "description": "status is 2xx or 3xx" },
            "headers": { "type": "object", "description": "Response headers, lower-case names; a repeated header is a list." },
            "content_type": { "type": "string" },
            "body": { "description": "Parsed JSON, text, or a FileRef with --parse bytes." },
            "request": { "type": "object", "description": "The method and URL called (masked when a secure_request profile owns them)." }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            flag("--url", "url", "The URL to call (http or https). Leave it out when a secure_request credential owns the URL.", "text"),
            DslFlag { choices: words(METHODS), ..flag("--method", "method", "The HTTP method (default GET).", "") },
            flag("--body", "body", "The request body — a literal or {{ expr }}; a whole {{ }} keeps its type, so an object stays an object. With a secure_request profile it is sent when the profile's Body Template is blank.", "json"),
            DslFlag { choices: words(FORMAT_WORDS), ..flag("--format", "format", "How the body is written: json (default), text, or form-data (multipart; a FileRef is a file part).", "") },
            DslFlag { kind: DslFlagKind::KeyValuePairs, ..flag("--header", "headers", "A request header, repeated: --header \"Accept=application/json\". A repeated name is sent twice.", "text") },
            DslFlag { choices: words(PARSE_WORDS), ..flag("--parse", "parse", "How the response body is read: json (default; text when it is not JSON), text, or bytes (stored as a FileRef).", "") },
            flag("--credential", "credential_id", "A secure_request credential (the profile owns URL, method, headers and secrets) or an oauth2 credential (a Bearer token, refreshed).", "text"),
            DslFlag { kind: DslFlagKind::KeyValuePairs, ..flag("--argument", "argument", "A secure_request profile variable, repeated: --argument \"USER_ID={{ input.player_id }}\". Unset, the profile's default is used.", "expression") },
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType, SelectOptionDef};
            let select = |name: &str, label: &str, words: &[&str], help: &str| NodeFieldDef {
                name: name.to_string(),
                label: label.to_string(),
                field_type: NodeFieldType::Select,
                options: words.iter().map(|w| SelectOptionDef { value: w.to_string(), label: w.to_string() }).collect(),
                default_value: words.first().map(|w| json!(w)),
                help: Some(help.to_string()),
                ..Default::default()
            };
            vec![
                NodeFieldDef { name: "credential_id".to_string(), label: "Request Profile".to_string(), field_type: NodeFieldType::Select, data_source: Some(crate::pipeline::model::NodeFieldDataSource::CredentialsHttpAuth), help: Some("Credential for HTTP authentication. secure_request: template-driven request. oauth2: Bearer token auto-refresh.".to_string()), ..Default::default() },
                NodeFieldDef { name: "argument".to_string(), label: "Profile Arguments".to_string(), field_type: NodeFieldType::SecureRequestBindings, help: Some("A value for each variable the selected secure_request profile declares: a literal or {{ expr }}, e.g. {{ input.player_id }}. Empty: the profile's default.".to_string()), span: Some("full".to_string()), ..Default::default() },
                NodeFieldDef { name: "url".to_string(), label: "URL".to_string(), field_type: NodeFieldType::Text, placeholder: Some("https://api.example.com/v1/…".to_string()), help: Some("Target URL — a literal or {{ expr }}. Leave empty when a secure_request profile owns the URL.".to_string()), ..Default::default() },
                select("method", "Method", METHODS, "The HTTP method."),
                select("parse", "Parse Response", PARSE_WORDS, "How the response body is read. 'bytes' stores it as a FileRef."),
                select("format", "Body Format", FORMAT_WORDS, "How the request body is written. 'form-data' sends FileRef and __zf_bytes objects as file parts."),
                NodeFieldDef { name: "headers".to_string(), label: "Headers".to_string(), field_type: NodeFieldType::KeyValuePairs, help: Some("Request headers. Each value may be a literal or {{ expr }}.".to_string()), ..Default::default() },
                NodeFieldDef { name: "body".to_string(), label: "Body".to_string(), field_type: NodeFieldType::Textarea, help: Some("Request body — a literal or {{ expr }}. A whole {{ }} carries its typed value.".to_string()), ..Default::default() },
            ]
        },
        layout: vec![
            LayoutItem::Field("credential_id".to_string()),
            LayoutItem::Field("argument".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("method".to_string()), LayoutItem::Field("parse".to_string()), LayoutItem::Field("format".to_string())] },
            LayoutItem::Field("url".to_string()),
            LayoutItem::Field("headers".to_string()),
            LayoutItem::Field("body".to_string()),
        ],
        ai_tool: crate::pipeline::model::NodeAiToolDefinition {
            registered: true,
            tool_name: "http_request".to_string(),
            tool_description: "Make an HTTP request. Args: url (required), method (GET/POST/etc.), body, headers (object), parse (json/text/bytes where bytes returns a FileRef), format (json/text/form-data).".to_string(),
            tool_input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "url":     { "type": "string", "description": "Target URL" },
                    "method":  { "type": "string", "enum": METHODS, "description": "HTTP method (default GET)" },
                    "body":    { "description": "Request body (optional)" },
                    "headers": { "type": "object", "description": "Additional headers (optional)" },
                    "parse":   { "type": "string", "enum": PARSE_WORDS, "description": "How the response is read (default json)" },
                    "format":  { "type": "string", "enum": FORMAT_WORDS, "description": "How the body is written (default json)" }
                },
                "required": ["url"]
            }),
        },
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Fetch JSON", r#"http.response.fetch --url "https://api.example.com/rates?base={{ $trigger.params.currency }}""#)
                .output(serde_json::json!({ "response": { "status": 200, "ok": true, "headers": { "content-type": "application/json" }, "content_type": "application/json", "body": { "AUD": 1, "USD": 0.65 }, "request": { "url": "https://api.example.com/rates?base=AUD", "method": "GET" } } })),
            crate::pipeline::model::NodeExample::dsl("POST with a credential", r#"http.response.fetch --url https://hooks.example.com/notify --method POST --credential notify_key --body "{{ { text: 'New order ' + input.query.rows[0]._key } }}""#),
            crate::pipeline::model::NodeExample::dsl("Download a file", r#"http.response.fetch --url https://example.com/report.pdf --parse bytes"#)
                .note("`input.response.body` is a FileRef; `fs.file.put --from \"{{ input.response.body }}\"` keeps it."),
        ],
        ..Default::default()
    }
}

/// A scalar flag with its 0.11 metadata.
fn flag(name: &str, key: &str, description: &str, value: &str) -> DslFlag {
    DslFlag {
        flag: name.to_string(),
        config_key: key.to_string(),
        description: description.to_string(),
        kind: DslFlagKind::Scalar,
        value: value.to_string(),
        ..Default::default()
    }
}

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|w| w.to_string()).collect()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub credential_id: Option<String>,
    /// `--argument NAME=value`: a secure_request profile's variables, each
    /// a literal or `{{ expr }}` arriving final — or one whole `{{ object }}`.
    #[serde(default)]
    pub argument: Value,
    #[serde(default)]
    pub url: String,
    /// One of [`METHODS`]; empty is GET.
    #[serde(default)]
    pub method: String,
    /// Request headers: a value is a string, or a list when the name repeats.
    #[serde(default)]
    pub headers: Map<String, Value>,
    /// The request body — a literal or `{{ expr }}`, arriving final. A whole
    /// `{{ }}` carries its typed value, so an object stays an object.
    #[serde(default)]
    pub body: Value,
    /// One of [`PARSE_WORDS`]; empty is json.
    #[serde(default)]
    pub parse: String,
    /// One of [`FORMAT_WORDS`]; empty is json.
    #[serde(default)]
    pub format: String,
}

fn default_method() -> String {
    "GET".to_string()
}

#[derive(Debug, Clone, Deserialize, Default)]
struct SecureRequestVariableDef {
    name: String,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    default_expr: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct SecureRequestTemplate {
    #[serde(default = "default_method")]
    method: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    #[serde(default)]
    body: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct SecureRequestEgressPolicy {
    #[serde(default)]
    allow_private: bool,
    #[serde(default)]
    allowed_hosts: Vec<String>,
    #[serde(default)]
    allowed_methods: Vec<String>,
    #[serde(default)]
    allowed_paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct SecureRequestSecret {
    #[serde(default)]
    request: SecureRequestTemplate,
    #[serde(default)]
    variables: Vec<SecureRequestVariableDef>,
    #[serde(default)]
    secrets: BTreeMap<String, String>,
    #[serde(default)]
    egress: SecureRequestEgressPolicy,
}

#[derive(Debug)]
struct PreparedRequest {
    url: String,
    visible_url: String,
    method: String,
    visible_method: String,
    /// In order; a name may repeat.
    headers: Vec<(String, String)>,
    body: Option<Value>,
    redact_tokens: Vec<String>,
    credential_id: Option<String>,
    egress_policy: Option<OutboundHttpPolicy>,
}

pub struct Node {
    config: Config,
    language: Arc<dyn LanguageEngine>,
    credentials: Option<Arc<CredentialService>>,
    platform: Option<Arc<PlatformService>>,
    /// Declared hosts of the node bundle this request runs inside, if any.
    bundle_egress: Option<Arc<BundleEgress>>,
}

impl Node {
    pub fn new(
        config: Config,
        language: Arc<dyn LanguageEngine>,
        credentials: Option<Arc<CredentialService>>,
        platform: Option<Arc<PlatformService>>,
        bundle_egress: Option<Arc<BundleEgress>>,
    ) -> Result<Self, PipelineError> {
        let url = config.url.trim();
        let has_credential = !config
            .credential_id
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty();
        // `--url` may be a literal or a `{{ expr }}`; either way it is a
        // non-empty string here, because resolution happens before the node
        // is built. A credential can supply the URL instead.
        if url.is_empty() && !has_credential {
            return Err(PipelineError::new(
                "FW_NODE_HTTP_RESPONSE_FETCH_CONFIG",
                "--url is empty and there is no --credential to supply one",
            ));
        }
        if !url.is_empty() && !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(PipelineError::new(
                "FW_NODE_HTTP_RESPONSE_FETCH_CONFIG",
                format!("--url '{url}' must start with http:// or https://"),
            ));
        }
        // Every choice is checked before anything is sent; an unknown word is
        // refused, never read as the default.
        let method = choice(&config.method, METHODS, "GET", "--method", CONFIG_CODE)?.to_string();
        choice(&config.parse, PARSE_WORDS, "json", "--parse", CONFIG_CODE)?;
        choice(&config.format, FORMAT_WORDS, "json", "--format", CONFIG_CODE)?;
        argument_map(&config.argument)?;
        let mut config = config;
        config.method = method;
        Ok(Self {
            config,
            language,
            credentials,
            platform,
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
                "FW_NODE_HTTP_RESPONSE_FETCH_INPUT_PIN",
                format!("unsupported input pin '{}'", input.input_pin),
            ));
        }

        let prepared = if let Some(credential_id) = self
            .config
            .credential_id
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            let Some(credentials) = &self.credentials else {
                return Err(PipelineError::new(
                    "FW_NODE_HTTP_RESPONSE_FETCH_CREDENTIALS_UNAVAILABLE",
                    "credential service is not configured on this pipeline engine",
                ));
            };
            let (owner, project, _, _) = metadata_scope(&input.metadata)?;
            let credential = credentials
                .get_project_credential(owner, project, credential_id)
                .map_err(|err| {
                    PipelineError::new("FW_NODE_HTTP_RESPONSE_FETCH_CREDENTIAL", err.to_string())
                })?
                .ok_or_else(|| {
                    PipelineError::new(
                        "FW_NODE_HTTP_RESPONSE_FETCH_CREDENTIAL_MISSING",
                        format!("credential '{}' not found", credential_id),
                    )
                })?;
            match credential.kind.as_str() {
                "secure_request" => build_request_from_secure_credential(
                    credential_id,
                    self.language.as_ref(),
                    &self.config,
                    &credential.secret,
                    &input.payload,
                    &input.metadata,
                )?,
                "oauth2" => {
                    // Get a valid access token, auto-refreshing if expired.
                    let token = credentials
                        .get_valid_oauth2_token(owner, project, credential_id)
                        .await
                        .map_err(|err| {
                            PipelineError::new("FW_NODE_HTTP_RESPONSE_FETCH_OAUTH2", err.to_string())
                        })?;
                    // Resolve URL/method/headers/body from node config (not from credential template).
                    // Arrives final — `{{ }}` resolved engine-side before this ran.
            let url = self.config.url.trim().to_string();
                    if !url.starts_with("http://") && !url.starts_with("https://") {
                        return Err(PipelineError::new(
                            "FW_NODE_HTTP_RESPONSE_FETCH_CONFIG",
                            "resolved url must start with http:// or https://",
                        ));
                    }
                    let method = self.config.method.clone();
                    let mut headers = header_pairs(&self.config.headers);
                    // The Bearer token is the credential's: it replaces any
                    // Authorization the node set.
                    headers.retain(|(name, _)| !name.eq_ignore_ascii_case("authorization"));
                    headers.push(("Authorization".to_string(), format!("Bearer {token}")));
                    // The body arrives final; null means "no body".
            let body_value = match self.config.body.clone() {
                Value::Null => None,
                other => Some(other),
            };
                    PreparedRequest {
                        visible_url: url.clone(),
                        url,
                        visible_method: method.clone(),
                        method,
                        headers,
                        body: body_value,
                        redact_tokens: vec![token],
                        credential_id: Some(credential_id.to_string()),
                        egress_policy: None,
                    }
                }
                other => {
                    return Err(PipelineError::new(
                        "FW_NODE_HTTP_RESPONSE_FETCH_CREDENTIAL_KIND",
                        format!(
                            "credential '{}' has kind '{}' — expected secure_request or oauth2",
                            credential.credential_id, other
                        ),
                    ));
                }
            }
        } else {
            // Arrives final — `{{ }}` resolved engine-side before this ran.
            let url = self.config.url.trim().to_string();
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(PipelineError::new(
                    "FW_NODE_HTTP_RESPONSE_FETCH_CONFIG",
                    "resolved url must start with http:// or https://",
                ));
            }
            let method = self.config.method.clone();
            let headers = header_pairs(&self.config.headers);
            // The body arrives final; null means "no body".
            let body_value = match self.config.body.clone() {
                Value::Null => None,
                other => Some(other),
            };
            PreparedRequest {
                visible_url: url.clone(),
                url,
                visible_method: method.clone(),
                method,
                headers,
                body: body_value,
                redact_tokens: Vec::new(),
                credential_id: None,
                egress_policy: None,
            }
        };

        let request_visible_url = prepared.visible_url.clone();
        let request_method = prepared.visible_method.clone();
        let request_credential_id = prepared.credential_id.clone();
        // The bundle's declaration is checked first, so a refusal names the
        // undeclared host rather than whatever the network guard finds there.
        if let Some(egress) = &self.bundle_egress {
            egress.check_url(&prepared.url, NODE_KIND)?;
        }
        if let Some(policy) = &prepared.egress_policy {
            crate::pipeline::security::validate_outbound_http_url_with_policy(
                &prepared.url,
                NODE_KIND,
                policy,
            )?;
        } else {
            crate::pipeline::security::validate_outbound_http_url(&prepared.url, NODE_KIND)?;
        }

        // ── Build reqwest client + request ───────────────────────────────────────
        // No timeout of its own: the engine's `--timeout` bounds the node, and
        // dropping this future closes the connection.
        let mut client_builder = reqwest::Client::builder();
        if let Some(egress) = self.bundle_egress.clone() {
            // A checked URL that redirects is still a connection to wherever it
            // lands, so every hop answers to the same declaration.
            client_builder =
                client_builder.redirect(reqwest::redirect::Policy::custom(move |attempt| {
                    match egress.check_url(attempt.url().as_str(), NODE_KIND) {
                        Ok(()) if attempt.previous().len() >= 10 => attempt.stop(),
                        Ok(()) => attempt.follow(),
                        Err(error) => attempt.error(error.message),
                    }
                }));
        }
        let client = client_builder
            .build()
            .map_err(|e| PipelineError::new("FW_NODE_HTTP_RESPONSE_FETCH_CLIENT", e.to_string()))?;

        let method_parsed = reqwest::Method::from_bytes(prepared.method.as_bytes())
            .map_err(|e| PipelineError::new("FW_NODE_HTTP_RESPONSE_FETCH_METHOD", e.to_string()))?;

        let mut req = client.request(method_parsed.clone(), &prepared.url);
        for (key, value) in &prepared.headers {
            req = req.header(key.as_str(), value.as_str());
        }
        // The run's id travels with every call the run makes
        // (`kinds/node-io`, Request id across the boundary), so a failure in
        // the other service traces back to the run that caused it. An author
        // who set the header themselves wins.
        if !prepared.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("x-request-id")) {
            if let Some(run_id) = input.metadata.get("request_id").and_then(Value::as_str).filter(|s| !s.is_empty()) {
                req = req.header("x-request-id", run_id);
            }
        }

        // ── The body, written as `--format` says ─────────────────────────────────
        let body_type = choice(&self.config.format, FORMAT_WORDS, "json", "--format", CONFIG_CODE)?;
        let has_body = !matches!(
            method_parsed.as_str(),
            "GET" | "HEAD" | "DELETE" | "OPTIONS"
        );

        if has_body {
            if let Some(body) = &prepared.body {
                match body_type {
                    "form-data" => {
                        req = build_multipart_request(
                            req,
                            body,
                            self.platform.as_ref(),
                            &input.metadata,
                        )?;
                    }
                    "text" => {
                        let body_str = match body {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        // Only set content-type if not already in headers
                        if !prepared.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-type")) {
                            req = req.header("content-type", "text/plain");
                        }
                        req = req.body(body_str);
                    }
                    _ => {
                        // "json" (default)
                        let body_str = match body {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        if !prepared.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-type")) {
                            req = req.header("content-type", "application/json");
                        }
                        req = req.body(body_str);
                    }
                }
            }
        }

        // ── Execute request ──────────────────────────────────────────────────────
        let transport_visible_url = prepared.visible_url.clone();
        let transport_raw_url = prepared.url.clone();

        let resp = req.send().await.map_err(|err| {
            let mut message = err.to_string();
            if transport_raw_url != transport_visible_url && !transport_visible_url.is_empty() {
                message = message.replace(&transport_raw_url, &transport_visible_url);
            }
            PipelineError::new("FW_NODE_HTTP_RESPONSE_FETCH_TRANSPORT", message)
        })?;

        let status = resp.status().as_u16();
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        let response_headers = headers_answer(resp.headers());

        // ── The response, read as `--parse` says ─────────────────────────────────
        let response_type = choice(&self.config.parse, PARSE_WORDS, "json", "--parse", CONFIG_CODE)?;
        // The body is capped while it arrives, not after it is all in memory.
        let raw = read_body_capped(resp, "FW_NODE_HTTP_RESPONSE_FETCH_READ_BODY").await?;
        let body: Value = match response_type {
            "bytes" => {
                let bytes = raw;
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        "FW_NODE_HTTP_RESPONSE_FETCH_FILE_REF",
                        "platform service is required for --parse bytes",
                    ));
                };
                let (owner, project, _, request_id) = metadata_scope(&input.metadata)?;
                write_tmp_file_ref(
                    platform,
                    FileRefInput {
                        owner,
                        project,
                        request_id,
                        bytes: &bytes,
                        filename: filename_from_content_type(&content_type).as_deref(),
                        mime: Some(&content_type),
                        origin: "http.response",
                        trust: "untrusted",
                    },
                )?
            }
            "text" => Value::String(String::from_utf8_lossy(&raw).into_owned()),
            _ => {
                // `json`: parsed, or the text when it is not JSON.
                let text = String::from_utf8_lossy(&raw).into_owned();
                serde_json::from_str::<Value>(&text).unwrap_or(Value::String(text))
            }
        };

        // ── Build output ─────────────────────────────────────────────────────────
        let request_obj = if let Some(credential_id) = request_credential_id {
            json!({
                "credential_id": credential_id,
                "secured": true,
                "url": request_visible_url,
                "method": request_method
            })
        } else {
            json!({
                "url": request_visible_url,
                "method": request_method
            })
        };
        let response_obj = json!({
            "status": status,
            "ok": (200..400).contains(&status),
            "headers": response_headers,
            "content_type": content_type,
            "body": body,
            "request": request_obj
        });
        let mut payload = json!({ "response": response_obj });
        if !prepared.redact_tokens.is_empty() {
            if let Value::Object(map) = &mut payload {
                map.insert(
                    "__zf_private_redact".to_string(),
                    Value::Array(
                        prepared
                            .redact_tokens
                            .iter()
                            .map(|item| Value::String(item.clone()))
                            .collect(),
                    ),
                );
                map.insert(
                    "__zf_private_redact_except_paths".to_string(),
                    Value::Array(vec![Value::String("response.body".to_string())]),
                );
            }
        }
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: crate::pipeline::nodes::shared::util::with_answer(&input.payload, payload),
            trace: vec![format!("node_kind={NODE_KIND}")],
        })
    }
}

/// The response body, refused under the caller's `code` once it passes the
/// node read cap (`node-conventions.md` §5) — counted as it streams, so a
/// large or endless body never sits whole in memory.
pub(crate) async fn read_body_capped(mut resp: reqwest::Response, code: &'static str) -> Result<Vec<u8>, PipelineError> {
    let max = crate::pipeline::nodes::shared::project_store::MAX_NODE_OBJECT_BYTES;
    if resp.content_length().is_some_and(|len| len > max) {
        return Err(PipelineError::new(
            code,
            format!("the response declares {} bytes, over the {max} a node reads", resp.content_length().unwrap_or(0)),
        ));
    }
    let mut out = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(|err| PipelineError::new(code, err.to_string()))? {
        if out.len() as u64 + chunk.len() as u64 > max {
            return Err(PipelineError::new(code, format!("the response is over the {max} bytes a node reads")));
        }
        out.extend_from_slice(&chunk);
    }
    Ok(out)
}

/// Build a multipart/form-data request from a JSON value.
/// - Scalar fields become text parts.
/// - FileRef and objects with `__zf_bytes` become file parts.
fn build_multipart_request(
    req: reqwest::RequestBuilder,
    body: &Value,
    platform: Option<&Arc<PlatformService>>,
    metadata: &Value,
) -> Result<reqwest::RequestBuilder, PipelineError> {
    let map = match body {
        Value::Object(m) => m,
        _ => {
            return Err(PipelineError::new(
                "FW_NODE_HTTP_RESPONSE_FETCH_BODY",
                "--format form-data needs --body to be an object",
            ));
        }
    };

    let mut form = reqwest::multipart::Form::new();
    for (key, value) in map {
        match value {
            Value::Object(obj) if is_file_ref(value) => {
                let Some(platform) = platform else {
                    return Err(PipelineError::new(
                        "FW_NODE_HTTP_RESPONSE_FETCH_BODY",
                        format!("form-data: FileRef field '{key}' requires platform service"),
                    ));
                };
                let (owner, project, _, _) = metadata_scope(metadata)?;
                let bytes = read_file_ref_bytes(platform, owner, project, value)?;
                let mime = obj
                    .get("mime")
                    .or_else(|| obj.get("content_type"))
                    .and_then(Value::as_str)
                    .unwrap_or("application/octet-stream");
                let filename = obj
                    .get("filename")
                    .or_else(|| obj.get("name"))
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .unwrap_or_else(|| derive_filename_from_mime(mime));
                let part = reqwest::multipart::Part::bytes(bytes)
                    .file_name(filename)
                    .mime_str(mime)
                    .map_err(|e| {
                        PipelineError::new(
                            "FW_NODE_HTTP_RESPONSE_FETCH_BODY",
                            format!("form-data: invalid mime for FileRef field '{key}': {e}"),
                        )
                    })?;
                form = form.part(key.clone(), part);
            }
            Value::Object(obj) if obj.contains_key("__zf_bytes") => {
                let b64 = obj
                    .get("__zf_bytes")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let mime = obj
                    .get("__zf_mime")
                    .and_then(|v| v.as_str())
                    .unwrap_or("application/octet-stream");
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .map_err(|e| {
                        PipelineError::new(
                            "FW_NODE_HTTP_RESPONSE_FETCH_BODY",
                            format!("form-data: base64 decode error for field '{key}': {e}"),
                        )
                    })?;
                let filename = derive_filename_from_mime(mime);
                let part = reqwest::multipart::Part::bytes(bytes)
                    .file_name(filename)
                    .mime_str(mime)
                    .map_err(|e| {
                        PipelineError::new(
                            "FW_NODE_HTTP_RESPONSE_FETCH_BODY",
                            format!("form-data: invalid mime for field '{key}': {e}"),
                        )
                    })?;
                form = form.part(key.clone(), part);
            }
            Value::String(s) => {
                form = form.text(key.clone(), s.clone());
            }
            Value::Number(n) => {
                form = form.text(key.clone(), n.to_string());
            }
            Value::Bool(b) => {
                form = form.text(key.clone(), b.to_string());
            }
            Value::Null => {}
            other => {
                form = form.text(key.clone(), other.to_string());
            }
        }
    }
    Ok(req.multipart(form))
}

fn filename_from_content_type(content_type: &str) -> Option<String> {
    Some(derive_filename_from_mime(content_type))
}

fn derive_filename_from_mime(mime: &str) -> String {
    let base = mime.split(';').next().unwrap_or("").trim();
    let ext = match base {
        "audio/mpeg" => "mp3",
        "audio/wav" | "audio/x-wav" => "wav",
        "audio/ogg" => "ogg",
        "audio/mp4" | "audio/m4a" => "m4a",
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "application/pdf" => "pdf",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        "application/octet-stream" => "bin",
        _ => "bin",
    };
    format!("download.{ext}")
}


fn build_request_from_secure_credential(
    credential_id: &str,
    language: &dyn LanguageEngine,
    config: &Config,
    credential_secret: &Value,
    input: &Value,
    metadata: &Value,
) -> Result<PreparedRequest, PipelineError> {
    let secret: SecureRequestSecret =
        serde_json::from_value(credential_secret.clone()).map_err(|err| {
            PipelineError::new(
                "FW_NODE_HTTP_RESPONSE_FETCH_SECURE_REQUEST",
                format!("invalid secure_request secret: {err}"),
            )
        })?;

    if secret.request.url.trim().is_empty() {
        return Err(PipelineError::new(
            "FW_NODE_HTTP_RESPONSE_FETCH_SECURE_REQUEST",
            "secure_request credential requires secret.request.url",
        ));
    }

    let bindings = resolve_secure_request_bindings(language, config, &secret, input, metadata)?;
    let mut tokens = secret.secrets.clone();
    for (key, value) in bindings {
        tokens.insert(key, value);
    }
    let redact_tokens = tokens
        .values()
        .filter_map(|item| {
            let trimmed = item.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect::<Vec<_>>();

    let url = render_secure_request_template(&secret.request.url, &tokens);
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(PipelineError::new(
            "FW_NODE_HTTP_RESPONSE_FETCH_SECURE_REQUEST",
            "resolved secure_request url must start with http:// or https://",
        ));
    }

    let method = render_secure_request_template(&secret.request.method, &tokens).to_uppercase();
    let egress_policy = secure_request_egress_policy(&secret.egress, &url, &method)?;
    let headers = secret
        .request
        .headers
        .iter()
        .map(|(key, value)| (key.clone(), render_secure_request_template(value, &tokens)))
        .collect::<Vec<_>>();
    // The profile owns the body when it declares one. A blank Body Template
    // leaves the body to the node — `--body "{{ expr }}"` arrives final and
    // typed, so a provider call carries the profile's secret and the run's
    // payload without the payload's shape living in the credential.
    let body = if secret.request.body.trim().is_empty() {
        match config.body.clone() {
            Value::Null => None,
            other => Some(other),
        }
    } else {
        Some(Value::String(render_secure_request_template(
            &secret.request.body,
            &tokens,
        )))
    };
    Ok(PreparedRequest {
        visible_url: "••••••".to_string(),
        url,
        method,
        visible_method: "••••••".to_string(),
        headers,
        body,
        redact_tokens,
        credential_id: Some(credential_id.to_string()),
        egress_policy,
    })
}

fn secure_request_egress_policy(
    egress: &SecureRequestEgressPolicy,
    url: &str,
    method: &str,
) -> Result<Option<OutboundHttpPolicy>, PipelineError> {
    if egress.allow_private
        && egress
            .allowed_hosts
            .iter()
            .map(|host| host.trim())
            .all(str::is_empty)
    {
        return Err(PipelineError::new(
            "FW_NODE_HTTP_RESPONSE_FETCH_EGRESS",
            "secure_request egress.allow_private requires at least one egress.allowed_hosts entry",
        ));
    }

    if !egress.allowed_methods.is_empty() {
        let method = method.trim().to_ascii_uppercase();
        let allowed = egress.allowed_methods.iter().any(|allowed| {
            let allowed = allowed.trim();
            !allowed.is_empty() && allowed.eq_ignore_ascii_case(&method)
        });
        if !allowed {
            return Err(PipelineError::new(
                "FW_NODE_HTTP_RESPONSE_FETCH_EGRESS",
                format!("secure_request egress does not allow method {method}"),
            ));
        }
    }

    if !egress.allowed_paths.is_empty() {
        let parsed = reqwest::Url::parse(url).map_err(|err| {
            PipelineError::new(
                "FW_NODE_HTTP_RESPONSE_FETCH_EGRESS",
                format!("secure_request resolved URL is invalid: {err}"),
            )
        })?;
        let path = parsed.path();
        let allowed = egress
            .allowed_paths
            .iter()
            .map(|allowed| allowed.trim())
            .any(|allowed| !allowed.is_empty() && allowed == path);
        if !allowed {
            return Err(PipelineError::new(
                "FW_NODE_HTTP_RESPONSE_FETCH_EGRESS",
                format!("secure_request egress does not allow path {path}"),
            ));
        }
    }

    if egress.allow_private {
        return Ok(Some(OutboundHttpPolicy {
            allow_private: true,
            allowed_hosts: egress
                .allowed_hosts
                .iter()
                .map(|host| host.trim().to_string())
                .filter(|host| !host.is_empty())
                .collect(),
        }));
    }
    Ok(None)
}

fn resolve_secure_request_bindings(
    language: &dyn LanguageEngine,
    config: &Config,
    credential: &SecureRequestSecret,
    input: &Value,
    metadata: &Value,
) -> Result<BTreeMap<String, String>, PipelineError> {
    // `--argument` values arrive final (a literal, or a `{{ }}` the engine
    // resolved). A variable the node leaves unset takes the profile's own
    // default, which is the profile author's expression, run here.
    let arguments = argument_map(&config.argument)?;
    let mut out = BTreeMap::new();
    for item in &credential.variables {
        if let Some(value) = arguments.get(&item.name).filter(|value| !value.is_empty()) {
            out.insert(item.name.clone(), value.clone());
            continue;
        }
        let fallback = item.default_expr.trim();
        if fallback.is_empty() {
            if item.required {
                return Err(PipelineError::new(
                    "FW_NODE_HTTP_RESPONSE_FETCH_ARGUMENT",
                    format!("the secure_request profile needs --argument {}=…", item.name),
                ));
            }
            continue;
        }
        out.insert(item.name.clone(), eval_binding_to_string(language, fallback, input, metadata)?);
    }
    for (key, value) in arguments {
        if !value.is_empty() {
            out.entry(key).or_insert(value);
        }
    }
    Ok(out)
}

/// `--argument` as name → text: repeated pairs, or one `{{ object }}` that
/// arrived as the object (or as its JSON text). A value that is not text is
/// written as JSON.
fn argument_map(argument: &Value) -> Result<BTreeMap<String, String>, PipelineError> {
    let text = |value: &Value| match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    match argument {
        Value::Null => Ok(BTreeMap::new()),
        Value::Object(map) => Ok(map.iter().map(|(k, v)| (k.clone(), text(v))).collect()),
        Value::String(raw) if raw.trim().is_empty() => Ok(BTreeMap::new()),
        Value::String(raw) => match serde_json::from_str::<Value>(raw.trim()) {
            Ok(parsed @ Value::Object(_)) => argument_map(&parsed),
            _ => Err(PipelineError::new(CONFIG_CODE, "--argument takes NAME=value pairs or one {{ object }}")),
        },
        _ => Err(PipelineError::new(CONFIG_CODE, "--argument takes NAME=value pairs or one {{ object }}")),
    }
}

/// `--header` as name/value pairs in order: a list value (a name given
/// twice) is sent once per item.
fn header_pairs(headers: &Map<String, Value>) -> Vec<(String, String)> {
    let text = |value: &Value| match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    let mut out = Vec::new();
    for (name, value) in headers {
        match value {
            Value::Null => {}
            Value::Array(items) => out.extend(items.iter().map(|item| (name.clone(), text(item)))),
            other => out.push((name.clone(), text(other))),
        }
    }
    out
}

/// The response headers as the answer gives them: lower-case names, a
/// header that came more than once as a list.
fn headers_answer(headers: &reqwest::header::HeaderMap) -> Value {
    let mut out = Map::new();
    for name in headers.keys() {
        let values: Vec<Value> = headers
            .get_all(name)
            .iter()
            .map(|value| Value::String(String::from_utf8_lossy(value.as_bytes()).into_owned()))
            .collect();
        let value = if values.len() == 1 { values.into_iter().next().unwrap_or(Value::Null) } else { Value::Array(values) };
        out.insert(name.as_str().to_string(), value);
    }
    Value::Object(out)
}

fn eval_binding_to_string(
    language: &dyn LanguageEngine,
    expr: &str,
    input: &Value,
    metadata: &Value,
) -> Result<String, PipelineError> {
    let value = eval_deno_expr(language, expr, input, metadata)?;
    match value {
        Value::Null => Ok(String::new()),
        Value::String(value) => Ok(value),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Number(value) => Ok(value.to_string()),
        other => Ok(other.to_string()),
    }
}

fn render_secure_request_template(template: &str, tokens: &BTreeMap<String, String>) -> String {
    let mut out = template.to_string();
    for (key, value) in tokens {
        let placeholder = format!("<{key}>");
        out = out.replace(&placeholder, value);
    }
    out
}


#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use axum::{Json, Router, routing::get};
    use reqwest::StatusCode;
    use serde_json::{Value, json};
    use tokio::net::TcpListener;
    use tokio::time::timeout;

    use crate::language::NoopLanguageEngine;
    use crate::pipeline::nodes::{NodeExecutionInput, NodeHandler};
    use crate::pipeline::security::validate_outbound_http_url_with_policy;

    use super::{Config, INPUT_PIN_IN, NODE_KIND, Node, argument_map, build_request_from_secure_credential, header_pairs};

    fn build(config: Value) -> Result<Node, crate::pipeline::PipelineError> {
        Node::new(serde_json::from_value(config).expect("config"), Arc::new(NoopLanguageEngine), None, None, None)
    }

    /// The 0.11 words, and a closed choice refused rather than defaulted.
    #[test]
    fn the_flags_are_the_0_11_words_and_choices_are_closed() {
        let def = super::definition();
        let flags: Vec<&str> = def.dsl_flags.iter().map(|f| f.flag.as_str()).collect();
        assert_eq!(flags, ["--url", "--method", "--body", "--format", "--header", "--parse", "--credential", "--argument"]);
        for (config, flag) in [
            (json!({ "url": "https://example.com", "method": "FETCH" }), "--method"),
            (json!({ "url": "https://example.com", "parse": "xml" }), "--parse"),
            (json!({ "url": "https://example.com", "format": "yaml" }), "--format"),
        ] {
            let err = build(config).err().expect("refused");
            assert_eq!(err.code, "FW_NODE_HTTP_RESPONSE_FETCH_CONFIG");
            assert!(err.message.contains(flag), "{}", err.message);
        }
        assert!(build(json!({ "url": "https://example.com", "method": "post", "parse": "bytes", "format": "form-data" })).is_ok());
        assert_eq!(build(json!({ "url": "ftp://example.com" })).err().expect("refused").code, "FW_NODE_HTTP_RESPONSE_FETCH_CONFIG");
    }

    /// The answer is `response`, the payload is kept, and the response's
    /// headers are in it. The server is local, so a secure_request profile
    /// that allows exactly this host is what lets the call through.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_answer_is_response_and_the_payload_is_kept() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let app = Router::new().route(
            "/rates",
            get(|| async { ([("x-tag", "a")], Json(json!({ "USD": 0.65 }))) }),
        );
        let server = tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });

        let platform = crate::pipeline::nodes::shared::test_platform::test_platform();
        platform
            .credentials
            .upsert_project_credential(
                "superadmin",
                "default",
                &crate::platform::model::UpsertProjectCredentialRequest {
                    credential_id: "local".to_string(),
                    title: "Local".to_string(),
                    kind: "secure_request".to_string(),
                    secret: json!({
                        "request": { "method": "GET", "url": format!("http://{addr}/rates") },
                        "egress": { "allow_private": true, "allowed_hosts": ["127.0.0.1"] }
                    }),
                    notes: String::new(),
                },
            )
            .expect("credential");
        let node = Node::new(
            serde_json::from_value(json!({ "credential_id": "local" })).unwrap(),
            Arc::new(NoopLanguageEngine),
            Some(platform.credentials.clone()),
            None,
            None,
        )
        .expect("node");
        let out = node
            .execute_async(NodeExecutionInput {
                node_id: "n1".to_string(),
                input_pin: INPUT_PIN_IN.to_string(),
                payload: json!({ "kept": 1 }),
                metadata: json!({ "owner": "superadmin", "project": "default", "pipeline": "t", "request_id": "r" }),
                bus: None,
            })
            .await
            .expect("fetches");
        server.abort();
        assert_eq!(out.payload["kept"], 1);
        let response = &out.payload["response"];
        assert_eq!(response["status"], 200);
        assert_eq!(response["ok"], true);
        assert_eq!(response["body"], json!({ "USD": 0.65 }));
        assert_eq!(response["headers"]["x-tag"], "a");
        assert_eq!(response["request"]["secured"], true);
    }

    /// A header given twice is sent twice; `--argument` is pairs or one object.
    #[test]
    fn repeated_headers_and_arguments_keep_what_was_written() {
        let headers = serde_json::from_value(json!({ "Accept": "application/json", "X-Tag": ["a", "b"] })).unwrap();
        assert_eq!(
            header_pairs(&headers),
            vec![
                ("Accept".to_string(), "application/json".to_string()),
                ("X-Tag".to_string(), "a".to_string()),
                ("X-Tag".to_string(), "b".to_string()),
            ]
        );
        let args = argument_map(&json!({ "USER_ID": 7, "NAME": "ana" })).unwrap();
        assert_eq!(args.get("USER_ID").map(String::as_str), Some("7"));
        assert_eq!(argument_map(&json!("{\"A\":\"1\"}")).unwrap().get("A").map(String::as_str), Some("1"));
        assert!(argument_map(&json!("not an object")).is_err());
    }

    /// `--argument` fills a profile variable as written; an unset one falls
    /// back to the profile's own default expression.
    #[test]
    fn secure_request_variables_come_from_argument() {
        let config: Config = serde_json::from_value(json!({ "argument": { "USER_ID": "u-7" } })).unwrap();
        let prepared = build_request_from_secure_credential(
            "profile",
            &NoopLanguageEngine,
            &config,
            &json!({
                "request": { "method": "GET", "url": "https://api.example.com/accounts/<USER_ID>" },
                "variables": [{ "name": "USER_ID", "required": true }]
            }),
            &json!({}),
            &json!({}),
        )
        .expect("prepared");
        assert_eq!(prepared.url, "https://api.example.com/accounts/u-7");

        let err = build_request_from_secure_credential(
            "profile",
            &NoopLanguageEngine,
            &Config::default(),
            &json!({
                "request": { "method": "GET", "url": "https://api.example.com/accounts/<USER_ID>" },
                "variables": [{ "name": "USER_ID", "required": true }]
            }),
            &json!({}),
            &json!({}),
        )
        .expect_err("a required variable with no argument and no default");
        assert_eq!(err.code, "FW_NODE_HTTP_RESPONSE_FETCH_ARGUMENT");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn self_call_to_private_network_is_blocked_from_same_server_request_handler() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let addr = listener.local_addr().expect("resolve test server addr");
        let echo_url = format!("http://{addr}/echo");

        let app = Router::new()
            .route("/echo", get(|| async { Json(json!({ "ok": true })) }))
            .route(
                "/self",
                get({
                    let echo_url = echo_url.clone();
                    move || {
                        let echo_url = echo_url.clone();
                        async move {
                            let node = Node::new(
                                Config {
                                    url: echo_url,
                                    ..Default::default()
                                },
                                Arc::new(NoopLanguageEngine),
                                None,
                                None,
                                None,
                            )
                            .expect("build http.response.fetch node");

                            let err = node
                                .execute_async(NodeExecutionInput {
                                    node_id: "n0".to_string(),
                                    input_pin: INPUT_PIN_IN.to_string(),
                                    payload: json!({}),
                                    metadata: json!({}),
                                    bus: None,
                                })
                                .await
                                .expect_err("self-call should be blocked");

                            Json(json!({ "code": err.code, "message": err.message }))
                        }
                    }
                }),
            );

        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve test app");
        });

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .expect("build client");
        let response = timeout(
            Duration::from_secs(2),
            client.get(format!("http://{addr}/self")).send(),
        )
        .await
        .expect("outer timeout waiting for self handler")
        .expect("send request to self handler");

        assert_eq!(response.status(), StatusCode::OK);
        let payload: Value = response.json().await.expect("decode response payload");
        assert_eq!(payload["code"], "FW_EGRESS_DENIED");

        server.abort();
    }

    #[test]
    fn secure_request_masks_credential_owned_request_fields() {
        let prepared = build_request_from_secure_credential(
            "salam-login-request",
            &NoopLanguageEngine,
            &Config::default(),
            &json!({
                "request": {
                    "method": "POST",
                    "url": "https://api.university.example/portal/v1/auth/login",
                    "headers": {
                        "Content-Type": "application/x-www-form-urlencoded"
                    },
                    "body": "username=<USERNAME>&password=<PASSWORD>"
                },
                "variables": [],
                "secrets": {}
            }),
            &json!({}),
            &json!({}),
        )
        .expect("build prepared request");

        assert_eq!(prepared.visible_url, "••••••");
        assert_eq!(prepared.visible_method, "••••••");
        assert_eq!(
            prepared.credential_id.as_deref(),
            Some("salam-login-request")
        );
        assert_eq!(
            prepared.url,
            "https://api.university.example/portal/v1/auth/login"
        );
        assert_eq!(prepared.method, "POST");
        assert_eq!(
            prepared.body,
            Some(json!("username=<USERNAME>&password=<PASSWORD>")),
            "a profile that declares a body owns it"
        );
    }

    /// A provider profile (method, URL, `Authorization: Bearer <API_KEY>`)
    /// with a blank Body Template: the node's own `--body`, already resolved
    /// and typed, is what travels, and the key is a redact token.
    #[test]
    fn secure_request_with_a_blank_body_template_sends_the_nodes_body() {
        let config = Config {
            body: json!([{ "taskType": "imageInference", "positivePrompt": "a red bicycle" }]),
            ..Default::default()
        };
        let profile = json!({
            "request": {
                "method": "POST",
                "url": "https://api.runware.ai/v1",
                "headers": {
                    "Authorization": "Bearer <API_KEY>",
                    "Content-Type": "application/json"
                },
                "body": ""
            },
            "variables": [],
            "secrets": { "API_KEY": "rw-test-key-4f1e9c2b7a60" }
        });
        let prepared = build_request_from_secure_credential(
            "runware",
            &NoopLanguageEngine,
            &config,
            &profile,
            &json!({}),
            &json!({}),
        )
        .expect("build prepared request");

        assert_eq!(prepared.body, Some(config.body.clone()), "the node's typed body is sent as is");
        assert!(prepared.headers.iter().any(|(k, v)| k == "Authorization" && v == "Bearer rw-test-key-4f1e9c2b7a60"));
        assert!(prepared.redact_tokens.contains(&"rw-test-key-4f1e9c2b7a60".to_string()));
        assert_eq!(prepared.visible_url, "••••••");

        // Nothing on the node: no body at all.
        let prepared = build_request_from_secure_credential(
            "runware",
            &NoopLanguageEngine,
            &Config::default(),
            &profile,
            &json!({}),
            &json!({}),
        )
        .expect("build prepared request");
        assert_eq!(prepared.body, None);
    }

    #[test]
    fn secure_request_private_egress_allows_exact_host() {
        let prepared = build_request_from_secure_credential(
            "qwen-embedding-service",
            &NoopLanguageEngine,
            &Config::default(),
            &json!({
                "request": {
                    "method": "POST",
                    "url": "http://10.0.0.5/embed"
                },
                "egress": {
                    "allow_private": true,
                    "allowed_hosts": ["10.0.0.5"],
                    "allowed_methods": ["POST"],
                    "allowed_paths": ["/embed"]
                }
            }),
            &json!({}),
            &json!({}),
        )
        .expect("build prepared request");

        let policy = prepared.egress_policy.expect("egress policy");
        validate_outbound_http_url_with_policy(&prepared.url, NODE_KIND, &policy)
            .expect("private host explicitly allowed by credential");
    }

    #[test]
    fn secure_request_private_egress_requires_allowed_hosts() {
        let err = build_request_from_secure_credential(
            "bad-internal-service",
            &NoopLanguageEngine,
            &Config::default(),
            &json!({
                "request": {
                    "method": "POST",
                    "url": "http://10.0.0.5/embed"
                },
                "egress": {
                    "allow_private": true
                }
            }),
            &json!({}),
            &json!({}),
        )
        .expect_err("missing allowed_hosts should fail");

        assert_eq!(err.code, "FW_NODE_HTTP_RESPONSE_FETCH_EGRESS");
    }

    #[test]
    fn secure_request_private_egress_enforces_allowed_path() {
        let err = build_request_from_secure_credential(
            "qwen-embedding-service",
            &NoopLanguageEngine,
            &Config::default(),
            &json!({
                "request": {
                    "method": "POST",
                    "url": "http://10.0.0.5/not-embed"
                },
                "egress": {
                    "allow_private": true,
                    "allowed_hosts": ["10.0.0.5"],
                    "allowed_paths": ["/embed"]
                }
            }),
            &json!({}),
            &json!({}),
        )
        .expect_err("path outside allowlist should fail");

        assert_eq!(err.code, "FW_NODE_HTTP_RESPONSE_FETCH_EGRESS");
    }
}

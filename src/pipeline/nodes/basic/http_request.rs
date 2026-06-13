//! HTTP request node for pulling external/internal data into pipeline flow.
//!
//! For general node authoring rules, read `src/pipeline/nodes/mod.rs`; for
//! FileRef IR and backend/lifecycle rules, read
//! `src/pipeline/nodes/basic/file_ref.rs`.
//!
//! Binary responses (`--response-type bytes`) are stored as temporary FileRefs.
//! Multipart request bodies can send FileRefs as file parts without re-encoding
//! the bytes into JSON.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::language::LanguageEngine;
use crate::pipeline::nodes::basic::file_ref::{
    FileRefInput, is_file_ref, read_file_ref_bytes, write_tmp_file_ref,
};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::CredentialService;
use crate::platform::services::PlatformService;

use super::util::{eval_deno_expr, metadata_scope, resolve_path_cloned};
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem};

pub const NODE_KIND: &str = "n.http.request";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

fn default_response_type() -> String {
    "json".to_string()
}
fn default_body_type() -> String {
    "json".to_string()
}

/// Unified node-definition metadata for `n.http.request`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "HTTP Request".to_string(),
        description: "Perform HTTP call with support for JSON, text, and binary (bytes) responses. \
            Supports JSON, raw text, and multipart form-data request bodies including FileRef and __zf_bytes file parts. \
            Returns normalized { request, response } envelope."
            .to_string(),
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
        dsl_flags: vec![
            DslFlag { flag: "--credential".to_string(), config_key: "credential_id".to_string(), description: "Optional credential for HTTP auth. secure_request: template-driven. oauth2: auto-refresh Bearer token.".to_string(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--bind".to_string(), config_key: "request_bindings".to_string(), description: "Binding expression for one secure_request variable. Repeatable: --bind USER_ID=input.player_id".to_string(), kind: DslFlagKind::KeyValuePairs, required: false },
            DslFlag { flag: "--url".to_string(), config_key: "url".to_string(), description: "Target URL for the HTTP request.".to_string(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--url-expr".to_string(), config_key: "url_expr".to_string(), description: "JS expression returning the target URL. Evaluated against input payload at runtime.".to_string(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--method".to_string(), config_key: "method".to_string(), description: "HTTP method: GET (default), POST, PUT, PATCH, DELETE.".to_string(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--method-expr".to_string(), config_key: "method_expr".to_string(), description: "JS expression returning the HTTP method.".to_string(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--body-path".to_string(), config_key: "body_path".to_string(), description: "Dot-notation path into input payload whose value is sent as the request body.".to_string(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--body-expr".to_string(), config_key: "body_expr".to_string(), description: "JS expression returning the request body value. Evaluated against input payload.".to_string(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--header".to_string(), config_key: "headers".to_string(), description: "Static request header. Repeat for each header. Format: Header-Name=value. e.g. --header Content-Type=application/json".to_string(), kind: DslFlagKind::KeyValuePairs, required: false },
            DslFlag { flag: "--headers-expr".to_string(), config_key: "headers_expr".to_string(), description: "JS expression returning an object of request headers. Evaluated against input payload.".to_string(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--timeout-ms".to_string(), config_key: "timeout_ms".to_string(), description: "Request timeout in milliseconds (default 10000, max 120000).".to_string(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--response-type".to_string(), config_key: "response_type".to_string(), description: "How to read the response: json (default, parses body as JSON), text (raw string), bytes (stores body as FileRef).".to_string(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--body-type".to_string(), config_key: "body_type".to_string(), description: "How to send the request body: json (default), text (raw string), form-data (multipart with FileRef/__zf_bytes support).".to_string(), kind: DslFlagKind::Scalar, required: false },
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType, SelectOptionDef};
            vec![
                NodeFieldDef { name: "credential_id".to_string(), label: "Request Profile".to_string(), field_type: NodeFieldType::Select, data_source: Some(crate::pipeline::model::NodeFieldDataSource::CredentialsHttpAuth), help: Some("Credential for HTTP authentication. secure_request: template-driven request. oauth2: Bearer token auto-refresh.".to_string()), ..Default::default() },
                NodeFieldDef { name: "request_bindings".to_string(), label: "Profile Bindings".to_string(), field_type: NodeFieldType::SecureRequestBindings, help: Some("JS expressions for the variables declared by the selected secure_request profile. Example: input.player_id or ctx.nodes.n3.unit.code".to_string()), span: Some("full".to_string()), ..Default::default() },
                NodeFieldDef { name: "url".to_string(), label: "URL".to_string(), field_type: NodeFieldType::Text, help: Some("Fallback URL when url_expr is empty.".to_string()), default_value: Some(serde_json::json!("https://example.com")), ..Default::default() },
                NodeFieldDef { name: "method".to_string(), label: "Method".to_string(), field_type: NodeFieldType::Select, options: vec!["GET","POST","PUT","PATCH","DELETE"].iter().map(|m| SelectOptionDef { value: m.to_string(), label: m.to_string() }).collect(), help: Some("Fallback HTTP method when method_expr is empty.".to_string()), ..Default::default() },
                NodeFieldDef { name: "timeout_ms".to_string(), label: "Timeout (ms)".to_string(), field_type: NodeFieldType::Text, help: Some("Request timeout in milliseconds.".to_string()), ..Default::default() },
                NodeFieldDef {
                    name: "response_type".to_string(),
                    label: "Response Type".to_string(),
                    field_type: NodeFieldType::Select,
                    options: vec![
                        SelectOptionDef { value: "json".to_string(), label: "JSON (parse body)".to_string() },
                        SelectOptionDef { value: "text".to_string(), label: "Text (raw string)".to_string() },
                        SelectOptionDef { value: "bytes".to_string(), label: "Bytes (FileRef)".to_string() },
                    ],
                    help: Some("How to read the response body. 'bytes' stores content in ZebFS tmp and returns FileRef metadata.".to_string()),
                    default_value: Some(json!("json")),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "body_type".to_string(),
                    label: "Body Type".to_string(),
                    field_type: NodeFieldType::Select,
                    options: vec![
                        SelectOptionDef { value: "json".to_string(), label: "JSON".to_string() },
                        SelectOptionDef { value: "text".to_string(), label: "Text (raw string)".to_string() },
                        SelectOptionDef { value: "form-data".to_string(), label: "Form Data (multipart)".to_string() },
                    ],
                    help: Some("How to send the request body. 'form-data' supports FileRef and __zf_bytes objects as file parts.".to_string()),
                    default_value: Some(json!("json")),
                    ..Default::default()
                },
                NodeFieldDef { name: "headers".to_string(), label: "Static Headers".to_string(), field_type: NodeFieldType::KeyValuePairs, help: Some("Static request headers. Overridden by headers_expr if set.".to_string()), ..Default::default() },
                NodeFieldDef { name: "url_expr".to_string(), label: "URL Expr".to_string(), field_type: NodeFieldType::Textarea, rows: Some(3), help: Some("Optional JS expression returning string URL.".to_string()), ..Default::default() },
                NodeFieldDef { name: "method_expr".to_string(), label: "Method Expr".to_string(), field_type: NodeFieldType::Textarea, rows: Some(3), help: Some("Optional JS expression returning string method.".to_string()), ..Default::default() },
                NodeFieldDef { name: "body_path".to_string(), label: "Body Path".to_string(), field_type: NodeFieldType::Text, help: Some("Payload path used as request body when body_expr is empty.".to_string()), ..Default::default() },
                NodeFieldDef { name: "headers_expr".to_string(), label: "Headers Expr".to_string(), field_type: NodeFieldType::Textarea, rows: Some(4), help: Some("JS expression returning header object. Overrides static headers.".to_string()), ..Default::default() },
                NodeFieldDef { name: "body_expr".to_string(), label: "Body Expr".to_string(), field_type: NodeFieldType::Textarea, rows: Some(4), help: Some("JS expression returning request body value.".to_string()), ..Default::default() },
            ]
        },
        layout: vec![
            LayoutItem::Field("credential_id".to_string()),
            LayoutItem::Field("request_bindings".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("method".to_string()), LayoutItem::Field("timeout_ms".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("response_type".to_string()), LayoutItem::Field("body_type".to_string())] },
            LayoutItem::Field("url".to_string()),
            LayoutItem::Field("headers".to_string()),
            LayoutItem::Field("body_path".to_string()),
            LayoutItem::Field("url_expr".to_string()),
            LayoutItem::Field("method_expr".to_string()),
            LayoutItem::Field("headers_expr".to_string()),
            LayoutItem::Field("body_expr".to_string()),
        ],
        ai_tool: crate::pipeline::model::NodeAiToolDefinition {
            registered: true,
            tool_name: "http_request".to_string(),
            tool_description: "Make an HTTP request. Args: url (required), method (GET/POST/etc.), body (string), headers (object), response_type (json/text/bytes where bytes returns FileRef), body_type (json/text/form-data).".to_string(),
            tool_input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "url":     { "type": "string", "description": "Target URL" },
                    "method":  { "type": "string", "description": "HTTP method (default GET)" },
                    "body":    { "type": "string", "description": "Request body (optional)" },
                    "headers": { "type": "object", "description": "Additional headers (optional)" },
                    "response_type": { "type": "string", "description": "json (default), text, or bytes" },
                    "body_type": { "type": "string", "description": "json (default), text, or form-data" }
                },
                "required": ["url"]
            }),
        },
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub credential_id: Option<String>,
    #[serde(default)]
    pub request_bindings: BTreeMap<String, String>,
    #[serde(default)]
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
    /// How to read the response: "json" (default), "text", "bytes".
    #[serde(default = "default_response_type")]
    pub response_type: String,
    /// How to send the request body: "json" (default), "text", "form-data".
    #[serde(default = "default_body_type")]
    pub body_type: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            credential_id: None,
            request_bindings: BTreeMap::new(),
            url: String::new(),
            method: default_method(),
            headers: BTreeMap::new(),
            body_path: None,
            timeout_ms: None,
            url_expr: None,
            method_expr: None,
            headers_expr: None,
            body_expr: None,
            response_type: default_response_type(),
            body_type: default_body_type(),
        }
    }
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
struct SecureRequestSecret {
    #[serde(default)]
    request: SecureRequestTemplate,
    #[serde(default)]
    variables: Vec<SecureRequestVariableDef>,
    #[serde(default)]
    secrets: BTreeMap<String, String>,
}

struct PreparedRequest {
    url: String,
    visible_url: String,
    method: String,
    visible_method: String,
    headers: BTreeMap<String, String>,
    body: Option<Value>,
    redact_tokens: Vec<String>,
    credential_id: Option<String>,
}

pub struct Node {
    config: Config,
    language: Arc<dyn LanguageEngine>,
    credentials: Option<Arc<CredentialService>>,
    platform: Option<Arc<PlatformService>>,
}

impl Node {
    pub fn new(
        config: Config,
        language: Arc<dyn LanguageEngine>,
        credentials: Option<Arc<CredentialService>>,
        platform: Option<Arc<PlatformService>>,
    ) -> Result<Self, PipelineError> {
        let url = config.url.trim();
        let has_credential = !config
            .credential_id
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty();
        let url_expr_empty = config
            .url_expr
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty();
        if url.is_empty() && url_expr_empty && !has_credential {
            return Err(PipelineError::new(
                "FW_NODE_HTTP_REQUEST_CONFIG",
                "config.url or config.credential_id must not be empty",
            ));
        }
        if !url.is_empty() && !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(PipelineError::new(
                "FW_NODE_HTTP_REQUEST_CONFIG",
                "config.url must start with http:// or https://",
            ));
        }
        Ok(Self {
            config,
            language,
            credentials,
            platform,
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
                "FW_NODE_HTTP_REQUEST_INPUT_PIN",
                format!("unsupported input pin '{}'", input.input_pin),
            ));
        }

        let timeout_ms = self.config.timeout_ms.unwrap_or(10_000).clamp(100, 120_000);

        let prepared = if let Some(credential_id) = self
            .config
            .credential_id
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            let Some(credentials) = &self.credentials else {
                return Err(PipelineError::new(
                    "FW_NODE_HTTP_REQUEST_CREDENTIALS_UNAVAILABLE",
                    "credential service is not configured on this pipeline engine",
                ));
            };
            let (owner, project, _, _) = metadata_scope(&input.metadata)?;
            let credential = credentials
                .get_project_credential(owner, project, credential_id)
                .map_err(|err| {
                    PipelineError::new("FW_NODE_HTTP_REQUEST_CREDENTIAL", err.to_string())
                })?
                .ok_or_else(|| {
                    PipelineError::new(
                        "FW_NODE_HTTP_REQUEST_CREDENTIAL_MISSING",
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
                            PipelineError::new("FW_NODE_HTTP_REQUEST_OAUTH2", err.to_string())
                        })?;
                    // Resolve URL/method/headers/body from node config (not from credential template).
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
                    let mut headers = if let Some(expr) = self.config.headers_expr.as_deref() {
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
                    // Inject Bearer token as Authorization header.
                    headers.insert("Authorization".to_string(), format!("Bearer {token}"));
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
                    PreparedRequest {
                        visible_url: url.clone(),
                        url,
                        visible_method: method.clone(),
                        method,
                        headers,
                        body: body_value,
                        redact_tokens: vec![token],
                        credential_id: Some(credential_id.to_string()),
                    }
                }
                other => {
                    return Err(PipelineError::new(
                        "FW_NODE_HTTP_REQUEST_CREDENTIAL_KIND",
                        format!(
                            "credential '{}' has kind '{}' — expected secure_request or oauth2",
                            credential.credential_id, other
                        ),
                    ));
                }
            }
        } else {
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
            PreparedRequest {
                visible_url: url.clone(),
                url,
                visible_method: method.clone(),
                method,
                headers,
                body: body_value,
                redact_tokens: Vec::new(),
                credential_id: None,
            }
        };

        let request_visible_url = prepared.visible_url.clone();
        let request_method = prepared.visible_method.clone();
        let request_credential_id = prepared.credential_id.clone();
        crate::pipeline::security::validate_outbound_http_url(&prepared.url, NODE_KIND)?;

        // ── Build reqwest client + request ───────────────────────────────────────
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|e| PipelineError::new("FW_NODE_HTTP_REQUEST_CLIENT", e.to_string()))?;

        let method_parsed = reqwest::Method::from_bytes(prepared.method.as_bytes())
            .map_err(|e| PipelineError::new("FW_NODE_HTTP_REQUEST_METHOD", e.to_string()))?;

        let mut req = client.request(method_parsed.clone(), &prepared.url);
        for (key, value) in &prepared.headers {
            req = req.header(key.as_str(), value.as_str());
        }

        // ── Send body based on body_type ─────────────────────────────────────────
        let body_type = self.config.body_type.trim().to_lowercase();
        let has_body = !matches!(
            method_parsed.as_str(),
            "GET" | "HEAD" | "DELETE" | "OPTIONS"
        );

        if has_body {
            if let Some(body) = &prepared.body {
                match body_type.as_str() {
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
                        if !prepared
                            .headers
                            .keys()
                            .any(|k| k.to_lowercase() == "content-type")
                        {
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
                        if !prepared
                            .headers
                            .keys()
                            .any(|k| k.to_lowercase() == "content-type")
                        {
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
            PipelineError::new("FW_NODE_HTTP_REQUEST_TRANSPORT", message)
        })?;

        let status = resp.status().as_u16();
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        // ── Read response based on response_type ─────────────────────────────────
        let response_type = self.config.response_type.trim().to_lowercase();
        let body: Value = match response_type.as_str() {
            "bytes" => {
                let bytes = resp.bytes().await.map_err(|err| {
                    PipelineError::new("FW_NODE_HTTP_REQUEST_READ_BODY", err.to_string())
                })?;
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        "FW_NODE_HTTP_REQUEST_FILE_REF",
                        "platform service is required for --response-type bytes",
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
            "text" => {
                let text = resp.text().await.map_err(|err| {
                    PipelineError::new("FW_NODE_HTTP_REQUEST_READ_BODY", err.to_string())
                })?;
                Value::String(text)
            }
            _ => {
                // "json" (default): try parse, fall back to string
                let text = resp.text().await.map_err(|err| {
                    PipelineError::new("FW_NODE_HTTP_REQUEST_READ_BODY", err.to_string())
                })?;
                serde_json::from_str::<Value>(&text).unwrap_or(Value::String(text))
            }
        };

        // ── Build output ─────────────────────────────────────────────────────────
        let request_obj = if let Some(credential_id) = request_credential_id {
            json!({
                "credential_id": credential_id,
                "secured": true,
                "url": request_visible_url,
                "method": request_method,
                "timeout_ms": timeout_ms
            })
        } else {
            json!({
                "url": request_visible_url,
                "method": request_method,
                "timeout_ms": timeout_ms
            })
        };
        let response_obj = json!({
            "status": status,
            "ok": (200..400).contains(&status),
            "content_type": content_type,
            "body": body
        });
        let mut payload = json!({ "request": request_obj, "response": response_obj });
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
            payload,
            trace: vec![format!("node_kind={NODE_KIND}")],
        })
    }
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
                "FW_NODE_HTTP_REQUEST_BODY",
                "form-data body_type requires an object value (from body_path or body_expr)",
            ));
        }
    };

    let mut form = reqwest::multipart::Form::new();
    for (key, value) in map {
        match value {
            Value::Object(obj) if is_file_ref(value) => {
                let Some(platform) = platform else {
                    return Err(PipelineError::new(
                        "FW_NODE_HTTP_REQUEST_BODY",
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
                            "FW_NODE_HTTP_REQUEST_BODY",
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
                            "FW_NODE_HTTP_REQUEST_BODY",
                            format!("form-data: base64 decode error for field '{key}': {e}"),
                        )
                    })?;
                let filename = derive_filename_from_mime(mime);
                let part = reqwest::multipart::Part::bytes(bytes)
                    .file_name(filename)
                    .mime_str(mime)
                    .map_err(|e| {
                        PipelineError::new(
                            "FW_NODE_HTTP_REQUEST_BODY",
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
                "FW_NODE_HTTP_REQUEST_SECURE_REQUEST",
                format!("invalid secure_request secret: {err}"),
            )
        })?;

    if secret.request.url.trim().is_empty() {
        return Err(PipelineError::new(
            "FW_NODE_HTTP_REQUEST_SECURE_REQUEST",
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
            "FW_NODE_HTTP_REQUEST_SECURE_REQUEST",
            "resolved secure_request url must start with http:// or https://",
        ));
    }

    let method = render_secure_request_template(&secret.request.method, &tokens).to_uppercase();
    let headers = secret
        .request
        .headers
        .iter()
        .map(|(key, value)| (key.clone(), render_secure_request_template(value, &tokens)))
        .collect::<BTreeMap<_, _>>();
    let body = if secret.request.body.trim().is_empty() {
        None
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
    })
}

fn resolve_secure_request_bindings(
    language: &dyn LanguageEngine,
    config: &Config,
    credential: &SecureRequestSecret,
    input: &Value,
    metadata: &Value,
) -> Result<BTreeMap<String, String>, PipelineError> {
    let mut out = BTreeMap::new();
    for item in &credential.variables {
        let expr = config
            .request_bindings
            .get(&item.name)
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .or_else(|| {
                let fallback = item.default_expr.trim();
                if fallback.is_empty() {
                    None
                } else {
                    Some(fallback.to_string())
                }
            });
        let Some(expr) = expr else {
            if item.required {
                return Err(PipelineError::new(
                    "FW_NODE_HTTP_REQUEST_BINDING",
                    format!("missing required secure_request binding '{}'", item.name),
                ));
            }
            continue;
        };
        out.insert(
            item.name.clone(),
            eval_binding_to_string(language, &expr, input, metadata)?,
        );
    }
    for (key, expr) in &config.request_bindings {
        if out.contains_key(key) {
            continue;
        }
        let expr = expr.trim();
        if expr.is_empty() {
            continue;
        }
        out.insert(
            key.clone(),
            eval_binding_to_string(language, expr, input, metadata)?,
        );
    }
    Ok(out)
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

    use super::{Config, INPUT_PIN_IN, Node, build_request_from_secure_credential};

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
                                    method: "GET".to_string(),
                                    timeout_ms: Some(1_000),
                                    ..Default::default()
                                },
                                Arc::new(NoopLanguageEngine),
                                None,
                                None,
                            )
                            .expect("build http.request node");

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
                    "url": "https://api.uinsgd.ac.id/salam/v1/index.php/auth/login",
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
            "https://api.uinsgd.ac.id/salam/v1/index.php/auth/login"
        );
        assert_eq!(prepared.method, "POST");
    }
}

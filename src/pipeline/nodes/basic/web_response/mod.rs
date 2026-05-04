//! `n.web.response` — terminate the HTTP request with an explicit response.
//!
//! Single unified node for all web response concerns. Without `--template` it
//! serves the pipeline payload as JSON (or a redirect / plain-text message).
//! With `--template` it renders a TSX page through the RWE engine.
//!
//! # Decision matrix for agents
//!
//! | Intent | DSL |
//! |---|---|
//! | Serve pipeline output as JSON | `\| web.response` |
//! | Serve specific field as JSON | `\| web.response --body $.rows` |
//! | Render HTML page | `\| web.response --template pages/home.tsx` |
//! | Redirect | `\| web.response --location /somewhere` |
//! | Error with message | `\| web.response --status 403 --message "Access denied"` |
//! | Error page | `\| web.response --template pages/404.tsx --status 404` |
//! | Set session cookie | `\| web.response --template pages/home.tsx --set-cookie name=session,value=$.token,http-only` |
//!
//! # Cookie spec format (`--set-cookie`)
//!
//! Comma-separated key=value pairs (or boolean flags):
//! ```text
//! name=session,value=$.token,http-only,max-age=86400,secure,same-site=Strict,path=/
//! ```
//! - `name=<NAME>` — cookie name (required)
//! - `value=<PATH>` — cookie value; `$.field` resolves from upstream payload
//! - `http-only` — sets HttpOnly flag
//! - `secure` — sets Secure flag
//! - `max-age=<SECS>` — Max-Age directive (default 900)
//! - `same-site=<Lax|Strict|None>` — SameSite directive (default Lax)
//! - `path=<PATH>` — cookie Path (default /)

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::language::LanguageEngine;
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem};
use crate::pipeline::nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler};
use crate::pipeline::{NodeDefinition, PipelineError};
use crate::rwe::{CompiledTemplate, ReactiveWebEngine, ReactiveWebOptions, TemplateSource};

pub const NODE_KIND: &str = "n.web.response";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

// ── Definition ────────────────────────────────────────────────────────────────

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Web Response".to_string(),
        description:
            "Terminate the HTTP request with an explicit response. \
             Without --template: serves pipeline payload as JSON (default 200). \
             With --template: renders a TSX page via the RWE engine. \
             Use --status, --set-cookie, --header to control HTTP metadata."
                .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "__zf_response": { "type": "object" }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({
            "type": "object",
            "properties": {
                "template":    { "type": "string",  "description": "TSX template path (RWE mode)." },
                "status":      { "type": "integer", "description": "HTTP status code." },
                "location":    { "type": "string",  "description": "Redirect URL." },
                "message":     { "type": "string",  "description": "Plain text body." },
                "body_path":   { "type": "string",  "description": "JSON path into payload for body." },
                "set_cookie":  { "type": "string",  "description": "Cookie spec string." },
                "headers":     { "type": "object",  "description": "Extra response headers." },
                "load_scripts":{ "type": "string",  "description": "External scripts (template mode)." }
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--template".to_string(),
                config_key: "template".to_string(),
                description: "TSX page file relative to templates/, e.g. pages/home.tsx. \
                    Activates RWE mode — upstream payload becomes template state."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--status".to_string(),
                config_key: "status".to_string(),
                description: "HTTP status code (default 200, or 302 when --location is set)."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--location".to_string(),
                config_key: "location".to_string(),
                description:
                    "Redirect target URL. Implies --status 302 unless --status is also set."
                        .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--message".to_string(),
                config_key: "message".to_string(),
                description: "Short plain-text response body.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--body".to_string(),
                config_key: "body_path".to_string(),
                description:
                    "JSON path into the pipeline payload to use as the response body, e.g. $.rows."
                        .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--set-cookie".to_string(),
                config_key: "set_cookie".to_string(),
                description:
                    "Cookie spec: name=NAME,value=$.path,http-only,max-age=SECS,secure,same-site=Lax"
                        .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--header".to_string(),
                config_key: "headers".to_string(),
                description: "Extra response header. Repeatable: --header X-Custom=hello --header X-Other=world".to_string(),
                kind: DslFlagKind::KeyValuePairs,
                required: false,
            },
            DslFlag {
                flag: "--load-scripts".to_string(),
                config_key: "load_scripts".to_string(),
                description:
                    "External script URLs to inject (template mode only). Comma-separated."
                        .to_string(),
                kind: DslFlagKind::CommaSeparatedList,
                required: false,
            },
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType, NodeFieldDataSource};
            vec![
                NodeFieldDef {
                    name: "title".to_string(),
                    label: "Title".to_string(),
                    field_type: NodeFieldType::Text,
                    help: Some("Override display title.".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "template".to_string(),
                    label: "Template".to_string(),
                    field_type: NodeFieldType::Datalist,
                    data_source: Some(NodeFieldDataSource::TemplatesPages),
                    placeholder: Some("pages/home.tsx".to_string()),
                    help: Some("TSX template to render (optional — omit for JSON response).".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "status".to_string(),
                    label: "Status Code".to_string(),
                    field_type: NodeFieldType::Text,
                    placeholder: Some("200".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "location".to_string(),
                    label: "Redirect URL".to_string(),
                    field_type: NodeFieldType::Text,
                    placeholder: Some("/auth/login".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "body_path".to_string(),
                    label: "Body Path".to_string(),
                    field_type: NodeFieldType::Text,
                    placeholder: Some("$.rows".to_string()),
                    help: Some("JSON path into the pipeline payload to serve as response body (JSON mode only).".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "message".to_string(),
                    label: "Message".to_string(),
                    field_type: NodeFieldType::Text,
                    placeholder: Some("Access denied".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "set_cookie".to_string(),
                    label: "Set-Cookie".to_string(),
                    field_type: NodeFieldType::Text,
                    placeholder: Some("name=session,value=$.token,http-only,max-age=86400".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "headers".to_string(),
                    label: "Extra Response Headers".to_string(),
                    field_type: NodeFieldType::KeyValuePairs,
                    help: Some("Static response headers added to every response from this node.".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "load_scripts".to_string(),
                    label: "Load Scripts".to_string(),
                    field_type: NodeFieldType::Text,
                    placeholder: Some("https://cdn.example.com/app.js".to_string()),
                    help: Some("Comma-separated external script URLs to inject (template mode only).".to_string()),
                    ..Default::default()
                },
            ]
        },
        layout: vec![
            LayoutItem::Field("title".to_string()),
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("status".to_string()),
                    LayoutItem::Field("location".to_string()),
                ],
            },
            LayoutItem::Field("template".to_string()),
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("message".to_string()),
                    LayoutItem::Field("body_path".to_string()),
                ],
            },
            LayoutItem::Field("set_cookie".to_string()),
            LayoutItem::Field("headers".to_string()),
            LayoutItem::Field("load_scripts".to_string()),
        ],
        ai_tool: Default::default(),
    }
}

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// TSX template path (RWE mode). When set, upstream payload is template state.
    #[serde(default)]
    pub template: Option<String>,
    /// Inline template markup — injected at request time from `template` path.
    /// Never stored in pipeline JSON.
    #[serde(default)]
    pub markup: Option<String>,
    /// HTTP status code.
    #[serde(default)]
    pub status: Option<u16>,
    /// Redirect URL. Implies 302 when no `status` is set.
    #[serde(default)]
    pub location: Option<String>,
    /// Plain-text response body.
    #[serde(default)]
    pub message: Option<String>,
    /// JSON path into payload to use as response body (e.g. `$.rows`).
    #[serde(default)]
    pub body_path: Option<String>,
    /// Cookie spec string (see module docs for format).
    #[serde(default)]
    pub set_cookie: Option<String>,
    /// Extra response headers.
    #[serde(default)]
    pub headers: Map<String, Value>,
    /// External script URLs for template mode (comma-separated).
    #[serde(default)]
    pub load_scripts: Option<String>,
}

// ── Node (non-template path only) ────────────────────────────────────────────
// Template path is handled by BasicPipelineEngine via InlineWebResponse.

pub struct Node {
    config: Config,
}

impl Node {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait::async_trait]
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
        // Resolve location — supports $.field references into the payload.
        let location = self.config.location.as_deref().map(|loc| {
            if loc.starts_with("$.") || loc == "$" {
                resolve_json_path_string(&input.payload, loc).unwrap_or_else(|| loc.to_string())
            } else {
                loc.to_string()
            }
        });

        let status = self
            .config
            .status
            .or_else(|| if location.is_some() { Some(302) } else { None });

        let cookie = self
            .config
            .set_cookie
            .as_deref()
            .and_then(|spec| parse_cookie_spec(spec, &input.payload));

        let body = self
            .config
            .body_path
            .as_deref()
            .and_then(|p| resolve_json_path(&input.payload, p))
            .or_else(|| {
                if self.config.template.is_none()
                    && location.is_none()
                    && self.config.message.is_none()
                {
                    Some(input.payload.clone())
                } else {
                    None
                }
            });

        let envelope = json!({
            "status": status,
            "location": location,
            "message": self.config.message,
            "body": body,
            "set_cookie": cookie,
            "headers": self.config.headers,
        });

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({ "__zf_response": envelope }),
            trace: vec![
                format!("node_kind={NODE_KIND}"),
                format!("status={:?}", status),
            ],
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Parse a cookie spec string into a JSON object with resolved values.
///
/// Format: `name=session,value=$.token,http-only,max-age=86400,secure,same-site=Strict,path=/`
pub fn parse_cookie_spec(spec: &str, payload: &Value) -> Option<Value> {
    let mut name = String::new();
    let mut value = String::new();
    let mut max_age: i64 = 900;
    let mut http_only = true;
    let mut secure = false;
    let mut same_site = "Lax".to_string();
    let mut path = "/".to_string();

    for part in spec.split(',') {
        let part = part.trim();
        if let Some(v) = part.strip_prefix("name=") {
            name = v.to_string();
        } else if let Some(v) = part.strip_prefix("value=") {
            value = if v.starts_with("$.") || v == "$" {
                resolve_json_path_string(payload, v).unwrap_or_default()
            } else {
                v.to_string()
            };
        } else if let Some(v) = part.strip_prefix("max-age=") {
            max_age = v.parse().unwrap_or(900);
        } else if let Some(v) = part.strip_prefix("same-site=") {
            same_site = v.to_string();
        } else if let Some(v) = part.strip_prefix("path=") {
            path = v.to_string();
        } else if part == "http-only" {
            http_only = true;
        } else if part == "no-http-only" {
            http_only = false;
        } else if part == "secure" {
            secure = true;
        }
    }

    if name.is_empty() || value.is_empty() {
        return None;
    }

    Some(json!({
        "name": name,
        "value": value,
        "max_age": max_age,
        "http_only": http_only,
        "secure": secure,
        "same_site": same_site,
        "path": path,
    }))
}

/// Resolve a `$.field.sub` path against a JSON value, returning a string.
pub fn resolve_json_path_string(payload: &Value, path: &str) -> Option<String> {
    resolve_json_path(payload, path).map(|v| {
        v.as_str()
            .map(str::to_string)
            .unwrap_or_else(|| v.to_string())
    })
}

/// Resolve a `$.field.sub` path against a JSON value.
pub fn resolve_json_path(payload: &Value, path: &str) -> Option<Value> {
    let stripped = if let Some(s) = path.strip_prefix("$.") {
        s
    } else if path == "$" {
        return Some(payload.clone());
    } else {
        path
    };

    if stripped.is_empty() {
        return Some(payload.clone());
    }

    let mut current = payload;
    for segment in stripped.split('.') {
        current = current.get(segment)?;
    }
    Some(current.clone())
}

// ── Internal page compile/render ─────────────────────────────────────────────
// Used by BasicPipelineEngine for the template rendering path.

/// A compiled TSX page held in the engine's render cache.
pub struct CompiledPage {
    pub node_id: String,
    pub template: CompiledTemplate,
}

/// Compile a TSX template into a cached page artifact.
pub fn compile_page(
    node_id: &str,
    template: &TemplateSource,
    options: &ReactiveWebOptions,
    rwe: &dyn ReactiveWebEngine,
    language: &dyn LanguageEngine,
) -> Result<CompiledPage, PipelineError> {
    let compiled_template = rwe
        .compile_template(template, language, options)
        .map_err(|e| {
            PipelineError::new(
                "WEB_RESPONSE_COMPILE",
                format!("failed compiling node '{}': {}", node_id, e),
            )
        })?;
    Ok(CompiledPage {
        node_id: node_id.to_string(),
        template: compiled_template,
    })
}

/// Strip private JWT claims from `payload["auth"]` before it reaches the browser.
///
/// Only keys listed in `_zf_public` survive. If no keys are marked public,
/// `auth` is set to `null` (secure by default). Pipeline nodes upstream still
/// see the full claims — this filtering only applies at the render boundary.
fn strip_private_auth_claims(mut payload: Value) -> Value {
    let auth = match payload.get("auth") {
        Some(Value::Object(m)) => m.clone(),
        _ => return payload,
    };

    let public_keys: Vec<String> = match auth.get("_zf_public") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => vec![],
    };

    if let Some(obj) = payload.as_object_mut() {
        if public_keys.is_empty() {
            obj.insert("auth".to_string(), Value::Null);
        } else {
            let mut public_auth = Map::new();
            for key in &public_keys {
                if let Some(v) = auth.get(key) {
                    public_auth.insert(key.clone(), v.clone());
                }
            }
            obj.insert("auth".to_string(), Value::Object(public_auth));
        }
    }

    payload
}

/// Inject trigger-context fields into state so templates always have
/// `ctx.auth`, `ctx.params`, `ctx.query` regardless of what upstream nodes
/// did to the payload.
fn inject_trigger_fields(mut state: Value, metadata: &Value) -> Value {
    let Some(trigger) = metadata.get("trigger") else {
        return state;
    };
    let Value::Object(ref mut map) = state else {
        return state;
    };

    // params / query / headers: inject only when absent in state
    for key in &["params", "query", "headers"] {
        if !map.contains_key(*key) {
            if let Some(v) = trigger.get(*key) {
                map.insert(key.to_string(), v.clone());
            }
        }
    }
    // auth: always prefer trigger.auth — it carries _zf_public for correct
    // filtering by strip_private_auth_claims which runs right after.
    if !map.contains_key("auth") || map.get("auth") == Some(&Value::Null) {
        if let Some(auth) = trigger.get("auth") {
            map.insert("auth".to_string(), auth.clone());
        }
    }
    state
}

/// Render a previously compiled page artifact.
pub fn render_compiled_page(
    compiled: &CompiledPage,
    state: Value,
    metadata: Value,
    rwe: &dyn ReactiveWebEngine,
    language: &dyn LanguageEngine,
    request_id: &str,
    enabled_libraries: Vec<String>,
) -> Result<NodeExecutionOutput, PipelineError> {
    let route = metadata
        .get("route")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("/")
        .to_string();

    // Inject trigger fields before filtering — ensures ctx.auth/params/query are
    // always available in templates even when upstream nodes replaced the payload.
    let state = inject_trigger_fields(state, &metadata);
    // Strip private JWT claims before the payload reaches the browser DOM.
    let state = strip_private_auth_claims(state);

    let rendered = rwe
        .render(
            &compiled.template,
            state,
            language,
            &crate::rwe::RenderContext {
                route,
                request_id: request_id.to_string(),
                metadata,
                enabled_libraries,
            },
        )
        .map_err(|e| {
            PipelineError::new(
                "WEB_RESPONSE_RENDER",
                format!("failed rendering node '{}': {}", compiled.node_id, e),
            )
        })?;

    let mut trace = vec![
        format!("node={}", compiled.node_id),
        format!("node_kind={NODE_KIND}"),
        format!("output_pin={OUTPUT_PIN_OUT}"),
    ];
    trace.extend(rendered.trace);

    Ok(NodeExecutionOutput {
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        payload: json!({
            "html": rendered.html,
            "compiled_scripts": rendered.compiled_scripts,
            "hydration_payload": rendered.hydration_payload,
        }),
        trace,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::pipeline::nodes::{NodeExecutionInput, NodeHandler};

    use super::{Config, INPUT_PIN_IN, Node};

    #[tokio::test]
    async fn default_json_response_uses_upstream_payload_as_body() {
        let node = Node::new(Config::default());
        let input_payload = json!({
            "ok": true,
            "rows": [{ "id": 1, "name": "Alpha" }]
        });

        let output = node
            .execute_async(NodeExecutionInput {
                node_id: "n0".to_string(),
                input_pin: INPUT_PIN_IN.to_string(),
                payload: input_payload.clone(),
                metadata: json!({}),
                step_tx: None,
            })
            .await
            .expect("execute web.response");

        assert_eq!(output.payload["__zf_response"]["body"], input_payload);
    }

    #[test]
    fn render_boundary_injects_trigger_fields_and_filters_auth_claims() {
        let state = json!({
            "title": "Hello",
            "params": { "slug": "keep-existing" },
            "auth": null
        });
        let metadata = json!({
            "trigger": {
                "params": { "slug": "from-trigger" },
                "query": { "page": "1" },
                "headers": { "x-request-id": "req-123" },
                "auth": {
                    "sub": "user-1",
                    "role": "admin",
                    "secret": "internal-only",
                    "_zf_public": ["sub", "role"]
                }
            }
        });

        let injected = super::inject_trigger_fields(state, &metadata);
        assert_eq!(injected["params"]["slug"], "keep-existing");
        assert_eq!(injected["query"]["page"], "1");
        assert_eq!(injected["headers"]["x-request-id"], "req-123");
        assert_eq!(injected["auth"]["secret"], "internal-only");

        let filtered = super::strip_private_auth_claims(injected);
        assert_eq!(
            filtered["auth"],
            json!({
                "sub": "user-1",
                "role": "admin"
            })
        );
        assert!(filtered["auth"].get("secret").is_none());
    }
}

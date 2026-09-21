//! Webhook trigger node.
//!
//! # Pipeline position
//!
//! Always the first node in a webhook-triggered pipeline. Owns the HTTP route.
//! The request path flows as `PipelineContext.route` → node metadata `"route"` to
//! downstream nodes via `PipelineContext.route`.
//!
//! ```text
//! | n.trigger.webhook --path /blog --method GET
//! | pg.query --credential main-db -- "SELECT ..."
//! | n.web.response --template pages/blog-home.tsx
//! ```
//!
//! # SSE streaming
//!
//! Clients can request execution progress as a Server-Sent Events stream by
//! sending `Accept: text/event-stream`. The pipeline definition is identical
//! for both modes — no config changes needed.
//!
//! ```text
//! Normal:    curl -X POST /wh/owner/project/blog
//!            → waits for full pipeline, returns final response
//!
//! Streaming: curl -X POST /wh/owner/project/blog -H "Accept: text/event-stream"
//!            → event: step  {"step":"...","description":"...","at":"..."}
//!            → event: step  ...
//!            → event: done  {"ok":true,"value":{...}}
//!            (or event: error on failure)
//! ```
//!
//! # Multipart files
//!
//! Files are emitted as FileRef metadata under `input.files.<field>`. If a client
//! repeats a field name or uses common frontend array names (`photos[]`,
//! `photos[0]`), the value becomes an array and individual files can be addressed
//! by dot path (`files.photos.0`). See `src/pipeline/nodes/shared/file_ref.rs` for
//! the FileRef shape.

use crate::pipeline::model::{
    DslFlag, DslFlagKind, LayoutItem, NodeFieldDataSource, NodeFieldDef, NodeFieldType,
    SelectOptionDef,
};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const NODE_KIND: &str = "n.trigger.webhook";
pub const OUTPUT_PIN_OUT: &str = "out";

/// Unified node-definition metadata for `n.trigger.webhook`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Webhook Trigger".to_string(),
        description: "Start pipeline run from inbound HTTP path + method. \
            User-submitted data is namespaced under input.body to prevent collisions with request context: \
            application/json → parsed value at input.body; \
            application/x-www-form-urlencoded → form fields at input.body (percent-decoded); \
            multipart/form-data → text fields at input.body, files under input.files.{field} as FileRef metadata; repeated fields, field[], and field[0] become arrays. \
            GET requests → input.body is null. \
            Request context is at root: input.query (URL query params), input.params (path params), \
            input.path (request path), input.method (HTTP method). \
            Also available via $trigger: $trigger.query, $trigger.params, $trigger.auth, $trigger.headers. \
            Use --auth-type jwt/hmac/api_key and --auth-credential <id> to protect the route. \
            jwt auth checks Authorization: Bearer header first, then the cookie the credential names (`cookie_name`, default zebflow_session) — \
            verified claims are injected into input.auth. Add --auth-optional for a public page that only wants to know \
            who is signed in: a valid token fills input.auth, anything else leaves it null and the page renders for the guest. \
            Streaming: clients that send Accept: text/event-stream receive an SSE stream \
            instead of a single response. Nodes that emit signals via the ExecutionBus \
            (or return __signal in their output) are forwarded as event: signal messages. \
            The final pipeline result is sent as event: done (or event: error) and the \
            stream closes. The pipeline definition is identical for both modes — the \
            Accept header is the only switch.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "description": "Structured request payload. User body at input.body (JSON/form/multipart text). Request context at root: input.query, input.params, input.path, input.method. FileRef uploads at input.files.{field}; repeated fields and field[]/field[0] names become arrays. JWT claims at input.auth."
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
                },
                "auth_required_role": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Required roles for this route. One entry of the JWT 'roles' array claim must match. Empty = any authenticated user."
                },
                "auth_optional": {
                    "type": "boolean",
                    "description": "With auth_type jwt: a public route that knows who is signed in. A valid token fills input.auth; no token, an expired one or a missing role leaves input.auth null and the run proceeds — never 401 or 403."
                },
                "errors": {
                    "type": "string",
                    "enum": ["show", "hide"],
                    "description": "What this route's 5xx reveals, overriding the project's errors switch: show (code, message, node id, run link) or hide (the error page with a reference only). Absent: the project decides. Status codes never change."
                }
            }
        }),
        fields: vec![
            NodeFieldDef { name: "method".to_string(), label: "Method".to_string(), field_type: NodeFieldType::MethodButtons, options: vec!["GET","POST","PUT","PATCH","DELETE"].iter().map(|m| SelectOptionDef { value: m.to_string(), label: m.to_string() }).collect(), help: Some("HTTP method accepted by webhook trigger.".to_string()), ..Default::default() },
            NodeFieldDef { name: "path".to_string(), label: "Path".to_string(), field_type: NodeFieldType::Text, help: Some("Webhook relative path under /wh/{owner}/{project}.".to_string()), ..Default::default() },
            NodeFieldDef { name: "__webhook_public_url".to_string(), label: "Public URL".to_string(), field_type: NodeFieldType::CopyUrl, help: Some("Copy-ready URL for this trigger.".to_string()), ..Default::default() },
            NodeFieldDef { name: "auth_type".to_string(), label: "Auth Type".to_string(), field_type: NodeFieldType::Select, options: vec![
                SelectOptionDef { value: "none".to_string(), label: "None (public)".to_string() },
                SelectOptionDef { value: "jwt".to_string(), label: "JWT Bearer".to_string() },
                SelectOptionDef { value: "hmac".to_string(), label: "HMAC Signature".to_string() },
                SelectOptionDef { value: "api_key".to_string(), label: "API Key (X-API-Key)".to_string() },
            ], help: Some("Trigger-level auth. On failure returns 401.".to_string()), ..Default::default() },
            NodeFieldDef { name: "auth_credential".to_string(), label: "Auth Credential".to_string(), field_type: NodeFieldType::Select, data_source: Some(NodeFieldDataSource::CredentialsWebhookAuth), help: Some("Credential for signing key / secret / api_key.".to_string()), ..Default::default() },
            NodeFieldDef { name: "auth_required_role".to_string(), label: "Required Role".to_string(), field_type: NodeFieldType::MultiCheckbox, data_source: Some(NodeFieldDataSource::CredentialJwtRoles), help: Some("Roles allowed to access this route. Populated from the selected JWT credential's registered roles. Empty = any authenticated user.".to_string()), ..Default::default() },
            NodeFieldDef { name: "auth_optional".to_string(), label: "Auth Optional".to_string(), field_type: NodeFieldType::Checkbox, default_value: Some(serde_json::json!(false)), help: Some("Public route that knows who is signed in: a valid token fills input.auth, anything else leaves it null and the page still renders. Never 401.".to_string()), ..Default::default() },
            NodeFieldDef { name: "errors".to_string(), label: "Errors".to_string(), field_type: NodeFieldType::Text, placeholder: Some("show | hide".to_string()), help: Some("What a failure on this route reveals: show or hide. Empty: the project's errors switch decides.".to_string()), ..Default::default() },
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
            DslFlag {
                flag: "--auth-required-role".to_string(),
                config_key: "auth_required_role".to_string(),
                description: "Comma-separated roles required for this route. One entry of the JWT 'roles' array claim must match. E.g. lecturer,student. Empty = any authenticated user.".to_string(),
                kind: DslFlagKind::CommaSeparatedList,
                required: false,
            },
            DslFlag {
                flag: "--auth-optional".to_string(),
                config_key: "auth_optional".to_string(),
                description: "With --auth-type jwt: the route stays public; a valid token fills input.auth, no token or a bad one leaves input.auth null instead of answering 401. For a public page that greets a signed-in user.".to_string(),
                kind: DslFlagKind::Bool,
                required: false,
            },
            DslFlag {
                flag: "--errors".to_string(),
                config_key: "errors".to_string(),
                description: "show or hide: what a failure on this route reveals to the caller, overriding the project's errors switch (Settings → Addressing). Absent: the project decides. The status code is the same either way.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        layout: vec![
            LayoutItem::Field("path".to_string()),
            LayoutItem::Field("method".to_string()),
            LayoutItem::Field("__webhook_public_url".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("auth_type".to_string()), LayoutItem::Field("auth_credential".to_string())] },
            LayoutItem::Field("auth_required_role".to_string()),
            LayoutItem::Field("auth_optional".to_string()),
            LayoutItem::Field("errors".to_string()),
        ],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("A public page", "trigger.webhook --path /blog --method GET")
                .output(serde_json::json!({ "body": null, "params": {}, "query": { "page": "2" }, "path": "/blog", "method": "GET" }))
                .note("Served at `/wh/{owner}/{project}/blog`. Then a query, then `web.response --template pages/blog.tsx`."),
            crate::pipeline::model::NodeExample::dsl("A form POST", "trigger.webhook --path /contact --method POST")
                .output(serde_json::json!({ "body": { "email": "a@x.io", "message": "Hi" }, "params": {}, "query": {}, "path": "/contact", "method": "POST" }))
                .note("`<input name=\"email\">` arrives as `input.body.email`; a file field is `input.files.<name>`."),
            crate::pipeline::model::NodeExample::dsl("A protected admin route", "trigger.webhook --path /admin/posts/:id --method GET --auth-type jwt --auth-credential jwt_main --auth-required-role editor")
                .output(serde_json::json!({ "body": null, "params": { "id": "42" }, "query": {}, "path": "/admin/posts/42", "method": "GET", "auth": { "sub": "u_1", "roles": ["editor"] } }))
                .note("Without a valid token the request is refused before any node runs, and the credential's `auth_redirect` decides where a browser goes."),
        ],
        ..Default::default()
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
    /// Required roles for this route. JWT claim `role` must match one of these.
    /// Empty = any authenticated user may access. Comma-separated in DSL: `lecturer,student`.
    #[serde(default)]
    pub auth_required_role: Vec<String>,
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

/// How deep the sweep for sensitive values goes. Deep enough for a form posted
/// inside `body`, a JSON envelope inside that, and a few layers of nesting
/// under it; bounded so a hostile payload cannot turn trace redaction into the
/// expensive part of a request.
const SENSITIVE_SCAN_MAX_DEPTH: usize = 12;

/// Key names whose value is a secret wherever it appears.
const SENSITIVE_KEYS: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "passphrase",
    "secret",
    "client_secret",
    "token",
    "access_token",
    "refresh_token",
    "id_token",
    "session_token",
    "api_key",
    "apikey",
    "private_key",
    "authorization",
    "credit_card",
    "card_number",
    "cvv",
    "otp",
];

/// True when `key` names a secret.
///
/// Comparison is on letters and digits only, so `apiKey`, `api-key`, `api_key`,
/// and `API KEY` are one name. A trailing `s` is also dropped, because a field
/// holding several secrets is spelled `tokens` at least as often as `token` and
/// a list of secrets is not less secret than one.
pub(crate) fn is_sensitive_payload_key(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    let singular = normalized.strip_suffix('s').unwrap_or(&normalized);
    SENSITIVE_KEYS.iter().any(|candidate| {
        let candidate = candidate.replace('_', "");
        normalized == candidate || singular == candidate
    })
}

/// Collects the literal secret values a request carries, so the engine can
/// strike them out of every trace the run writes.
///
/// This walks the whole payload. It used to read top-level keys only, which
/// made it dead code in production: the ingress always nests the request body
/// under `body` (`build_structured_payload`), so the top-level keys of a real
/// webhook payload are `body`/`query`/`params`/`path`/`method`/`files` and
/// never a field name. A login form posted to a webhook therefore wrote its
/// plaintext password into `trace.input` and onto disk.
fn collect_trace_private_tokens(payload: &Value) -> Vec<String> {
    let mut out = Vec::new();
    collect_into(payload, false, 0, &mut out);
    out
}

fn collect_into(value: &Value, under_sensitive_key: bool, depth: usize, out: &mut Vec<String>) {
    if depth > SENSITIVE_SCAN_MAX_DEPTH {
        return;
    }
    match value {
        Value::Object(map) => {
            for (key, item) in map {
                collect_into(item, is_sensitive_payload_key(key), depth + 1, out);
            }
        }
        // An array under a sensitive key is a list of secrets -- `token: [..]`
        // -- so the flag carries through it rather than being reset.
        Value::Array(items) => {
            for item in items {
                collect_into(item, under_sensitive_key, depth + 1, out);
            }
        }
        Value::String(text) if under_sensitive_key => {
            let trimmed = text.trim();
            if trimmed.is_empty() || out.iter().any(|existing| existing == trimmed) {
                return;
            }
            out.push(trimmed.to_string());
        }
        _ => {}
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
        let mut payload = input.payload;
        let trace_private_tokens = collect_trace_private_tokens(&payload);
        if !trace_private_tokens.is_empty() {
            if let Value::Object(map) = &mut payload {
                map.insert(
                    "__zf_private_trace_redact".to_string(),
                    Value::Array(
                        trace_private_tokens
                            .into_iter()
                            .map(Value::String)
                            .collect(),
                    ),
                );
            }
        }
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload,
            trace: vec![
                format!("node_kind={NODE_KIND}"),
                format!("method={}", self.config.method),
                format!("path={}", self.config.path),
            ],
        })
    }
}

/// The collector, reachable from the engine's own tests so that "the ingress
/// publishes tokens the engine then applies" can be asserted end to end.
#[cfg(test)]
pub(crate) fn collect_trace_private_tokens_for_test(payload: &Value) -> Vec<String> {
    collect_trace_private_tokens(payload)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{collect_trace_private_tokens, is_sensitive_payload_key};

    #[test]
    fn collects_common_sensitive_form_fields_for_trace_redaction() {
        let tokens = collect_trace_private_tokens(&json!({
            "username": "wawan dot wan",
            "password": "toryoto",
            "api_key": "abc123"
        }));

        assert_eq!(tokens, vec!["toryoto", "abc123"]);
    }

    /// The shape the ingress actually produces. A flat payload is not one:
    /// `build_structured_payload` always nests the request body under `body`,
    /// so a collector that read top-level keys only returned nothing for every
    /// real request while its test passed on a payload no request can produce.
    #[test]
    fn collects_secrets_from_the_nested_shape_the_ingress_produces() {
        let tokens = collect_trace_private_tokens(&json!({
            "body": { "username": "wawan", "password": "toryoto" },
            "query": { "api_key": "abc123" },
            "params": {},
            "path": "/login",
            "method": "POST"
        }));

        assert!(tokens.contains(&"toryoto".to_string()));
        assert!(tokens.contains(&"abc123".to_string()));
        assert!(!tokens.contains(&"wawan".to_string()));
    }

    #[test]
    fn reaches_secrets_nested_below_the_body() {
        let tokens = collect_trace_private_tokens(&json!({
            "body": {
                "account": { "credentials": { "clientSecret": "deep-secret" } },
                "tokens": ["t-one", "t-two"]
            }
        }));

        assert!(tokens.contains(&"deep-secret".to_string()));
        assert!(tokens.contains(&"t-one".to_string()));
        assert!(tokens.contains(&"t-two".to_string()));
    }

    #[test]
    fn key_spelling_does_not_decide_whether_a_secret_is_seen() {
        for key in ["apiKey", "api-key", "API_KEY", "Api Key"] {
            assert!(is_sensitive_payload_key(key), "'{key}' should be sensitive");
        }
        assert!(
            is_sensitive_payload_key("tokens"),
            "a list of secrets is still secret"
        );
        assert!(!is_sensitive_payload_key("username"));
        assert!(!is_sensitive_payload_key("tokenizer"));
    }

    #[test]
    fn a_hostile_depth_does_not_run_away() {
        let mut value = json!({ "password": "bottom" });
        for _ in 0..500 {
            value = json!({ "wrap": value });
        }
        // Bounded: it stops rather than recursing 500 deep.
        assert!(collect_trace_private_tokens(&value).is_empty());
    }
}

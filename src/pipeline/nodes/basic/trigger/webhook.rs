//! Webhook trigger node.
//!
//! # Pipeline position
//!
//! Always the first node in a webhook-triggered pipeline. Owns the HTTP route.
//! The request path flows as `PipelineContext.route` → node metadata `"route"` to
//! downstream nodes via `PipelineContext.route`.
//!
//! ```text
//! | trigger.webhook --route /blog --method GET
//! | postgres.query.run --credential main-db -- "SELECT ..."
//! | web.response.send --template pages/blog-home.tsx
//! ```
//!
//! # Answer
//!
//! One key, `webhook` (`node-conventions.md` §6):
//! `webhook: { body, query, params, headers, files, method, path, auth }` —
//! the same envelope `$trigger` holds for the whole run.
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
//! Files are emitted as FileRef metadata under `webhook.files.<field>`. If a
//! client repeats a field name or uses common frontend array names
//! (`photos[]`, `photos[0]`), the value becomes an array and individual files
//! can be addressed by dot path (`webhook.files.photos.0`). See
//! `src/pipeline/nodes/shared/file_ref.rs` for the FileRef shape.

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

pub const NODE_KIND: &str = "trigger.webhook";
pub const OUTPUT_PIN_OUT: &str = "out";
/// The key this trigger answers under.
pub const ANSWER_KEY: &str = "webhook";

/// `--method`: the closed words.
pub const METHODS: [&str; 5] = ["GET", "POST", "PUT", "PATCH", "DELETE"];
/// `--auth`: the closed words, shared with `trigger.room`.
pub const AUTH_MODES: [&str; 4] = ["none", "jwt", "hmac", "api_key"];
/// `--errors`: the closed words.
pub const ERRORS_MODES: [&str; 2] = ["show", "hide"];
/// Raised when a webhook's flags are refused at activation.
pub const CONFIG_CODE: &str = "FW_NODE_TRIGGER_WEBHOOK_CONFIG";

/// `--auth`, `--credential`, `--role`: the guard `trigger.webhook` and
/// `trigger.room` share, one meaning in both.
pub fn auth_flags(guarded: &str) -> Vec<DslFlag> {
    vec![
        DslFlag {
            flag: "--auth".to_string(),
            config_key: "auth".to_string(),
            description: format!("How {guarded} is guarded: none (default), jwt, hmac, api_key. Anything but none needs --credential."),
            kind: DslFlagKind::Scalar,
            value: "text".to_string(),
            choices: AUTH_MODES.iter().map(|w| w.to_string()).collect(),
            ..Default::default()
        },
        DslFlag {
            flag: "--credential".to_string(),
            config_key: "credential_id".to_string(),
            description: "The credential (its id) that verifies --auth: a JWT signing key, an HMAC secret or an API key.".to_string(),
            kind: DslFlagKind::Scalar,
            value: "text".to_string(),
            ..Default::default()
        },
        DslFlag {
            flag: "--role".to_string(),
            config_key: "role".to_string(),
            description: "A role allowed in, repeated (`--role editor --role admin`): one entry of the JWT `roles` claim must match. None: any authenticated caller.".to_string(),
            kind: DslFlagKind::RepeatedList,
            value: "text".to_string(),
            ..Default::default()
        },
    ]
}

/// The editor fields for [`auth_flags`].
pub fn auth_fields(on_failure: &str) -> Vec<NodeFieldDef> {
    vec![
        NodeFieldDef { name: "auth".to_string(), label: "Auth".to_string(), field_type: NodeFieldType::Select, options: vec![
            SelectOptionDef { value: "none".to_string(), label: "None (public)".to_string() },
            SelectOptionDef { value: "jwt".to_string(), label: "JWT Bearer".to_string() },
            SelectOptionDef { value: "hmac".to_string(), label: "HMAC Signature".to_string() },
            SelectOptionDef { value: "api_key".to_string(), label: "API Key (X-API-Key)".to_string() },
        ], help: Some(format!("Trigger-level auth. {on_failure}")), ..Default::default() },
        NodeFieldDef { name: "credential_id".to_string(), label: "Credential".to_string(), field_type: NodeFieldType::Select, data_source: Some(NodeFieldDataSource::CredentialsWebhookAuth), help: Some("Credential for signing key / secret / api_key.".to_string()), ..Default::default() },
        NodeFieldDef { name: "role".to_string(), label: "Role".to_string(), field_type: NodeFieldType::MultiCheckbox, data_source: Some(NodeFieldDataSource::CredentialJwtRoles), help: Some("Roles allowed in. Populated from the selected JWT credential's registered roles. None: any authenticated caller.".to_string()), ..Default::default() },
    ]
}

/// Unified node-definition metadata for `trigger.webhook`.
pub fn definition() -> NodeDefinition {
    let mut dsl_flags = vec![
        DslFlag {
            flag: "--route".to_string(),
            config_key: "route".to_string(),
            description: "The path this project serves, under /wh/{owner}/{project}. Starts with /; `:name` is a path param. Examples: /blog, /api/users/:id.".to_string(),
            kind: DslFlagKind::Scalar,
            required: true,
            value: "text".to_string(),
            ..Default::default()
        },
        DslFlag {
            flag: "--method".to_string(),
            config_key: "method".to_string(),
            description: "HTTP method: GET (default), POST, PUT, PATCH, DELETE.".to_string(),
            kind: DslFlagKind::Scalar,
            value: "text".to_string(),
            choices: METHODS.iter().map(|w| w.to_string()).collect(),
            ..Default::default()
        },
    ];
    dsl_flags.extend(auth_flags("the route"));
    dsl_flags.extend([
        DslFlag {
            flag: "--auth-optional".to_string(),
            config_key: "auth_optional".to_string(),
            description: "With --auth jwt: the route stays public; a valid token fills webhook.auth, no token or a bad one leaves it null instead of answering 401. For a public page that greets a signed-in user.".to_string(),
            kind: DslFlagKind::Bool,
            ..Default::default()
        },
        DslFlag {
            flag: "--errors".to_string(),
            config_key: "errors".to_string(),
            description: "show or hide: what a failure on this route reveals to the caller, overriding the project's errors switch (Settings → Addressing). Absent: the project decides. The status code is the same either way.".to_string(),
            kind: DslFlagKind::Scalar,
            value: "text".to_string(),
            choices: ERRORS_MODES.iter().map(|w| w.to_string()).collect(),
            ..Default::default()
        },
    ]);
    let mut fields = vec![
        NodeFieldDef { name: "method".to_string(), label: "Method".to_string(), field_type: NodeFieldType::MethodButtons, options: METHODS.iter().map(|m| SelectOptionDef { value: m.to_string(), label: m.to_string() }).collect(), help: Some("HTTP method accepted by webhook trigger.".to_string()), ..Default::default() },
        NodeFieldDef { name: "route".to_string(), label: "Route".to_string(), field_type: NodeFieldType::Text, help: Some("Webhook relative path under /wh/{owner}/{project}.".to_string()), ..Default::default() },
        NodeFieldDef { name: "__webhook_public_url".to_string(), label: "Public URL".to_string(), field_type: NodeFieldType::CopyUrl, help: Some("Copy-ready URL for this trigger.".to_string()), ..Default::default() },
    ];
    fields.extend(auth_fields("On failure returns 401."));
    fields.extend([
        NodeFieldDef { name: "auth_optional".to_string(), label: "Auth Optional".to_string(), field_type: NodeFieldType::Checkbox, default_value: Some(serde_json::json!(false)), help: Some("Public route that knows who is signed in: a valid token fills webhook.auth, anything else leaves it null and the page still renders. Never 401.".to_string()), ..Default::default() },
        NodeFieldDef { name: "errors".to_string(), label: "Errors".to_string(), field_type: NodeFieldType::Select, options: vec![
            SelectOptionDef { value: String::new(), label: "Project decides".to_string() },
            SelectOptionDef { value: "show".to_string(), label: "show".to_string() },
            SelectOptionDef { value: "hide".to_string(), label: "hide".to_string() },
        ], help: Some("What a failure on this route reveals: show or hide. Empty: the project's errors switch decides.".to_string()), ..Default::default() },
    ]);
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Webhook Trigger".to_string(),
        description: "Runs the pipeline for every HTTP request on `--route` and `--method`. Answers one key, `webhook`: \
            `webhook: { body, query, params, headers, files, method, path, auth }`, and the same envelope is `$trigger` for the whole run. \
            What the caller submitted is `webhook.body`: \
            application/json → the parsed value; \
            application/x-www-form-urlencoded → the form fields (percent-decoded); \
            multipart/form-data → the text fields, with files under webhook.files.{field} as FileRef metadata; \
            in both, a repeated field (a checkbox group sharing one name), field[] and field[0] become arrays. \
            GET requests → webhook.body is null. \
            Request context: webhook.query (URL query params), webhook.params (path params), webhook.headers (a safe subset), \
            webhook.path (request path), webhook.method (HTTP method). \
            Use --auth jwt|hmac|api_key and --credential <id> to protect the route, --role to require a role. \
            jwt auth checks Authorization: Bearer header first, then the cookie the credential names (`cookie_name`, default zebflow_session) — \
            verified claims are webhook.auth. Add --auth-optional for a public page that only wants to know \
            who is signed in: a valid token fills webhook.auth, anything else leaves it null and the page renders for the guest. \
            Streaming: clients that send Accept: text/event-stream receive an SSE stream \
            instead of a single response. Nodes that emit signals via the ExecutionBus \
            (or return __signal in their output) are forwarded as event: signal messages. \
            The final pipeline result is sent as event: done (or event: error) and the \
            stream closes. The pipeline definition is identical for both modes — the \
            Accept header is the only switch.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "description": "The request envelope the ingress builds: body, query, params, headers, files, method, path, auth."
        }),
        output_schema: serde_json::json!({
            "type": "object",
            "description": "The request envelope under `webhook`.",
            "properties": {
                "webhook": {
                    "type": "object",
                    "properties": {
                        "body": {}, "query": { "type": "object" }, "params": { "type": "object" },
                        "headers": { "type": "object" }, "files": { "type": "object" },
                        "method": { "type": "string" }, "path": { "type": "string" }, "auth": {}
                    }
                }
            }
        }),
        input_pins: vec![],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: serde_json::json!({
            "type": "object",
            "required": ["route"],
            "properties": {
                "route": {
                    "type": "string",
                    "description": "The path this webhook serves, e.g. '/blog' or '/api/users/:id'. Must start with /."
                },
                "method": {
                    "type": "string",
                    "enum": METHODS,
                    "description": "HTTP method. Defaults to GET."
                },
                "auth": {
                    "type": "string",
                    "enum": AUTH_MODES,
                    "description": "Authentication mode. none = open (default). jwt/hmac/api_key require credential."
                },
                "credential_id": {
                    "type": "string",
                    "description": "Credential ID used for auth verification. Required when auth is not none."
                },
                "role": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Roles allowed on this route. One entry of the JWT 'roles' array claim must match. Empty = any authenticated user."
                },
                "auth_optional": {
                    "type": "boolean",
                    "description": "With auth jwt: a public route that knows who is signed in. A valid token fills webhook.auth; no token, an expired one or a missing role leaves webhook.auth null and the run proceeds — never 401 or 403."
                },
                "errors": {
                    "type": "string",
                    "enum": ERRORS_MODES,
                    "description": "What this route's 5xx reveals, overriding the project's errors switch: show (code, message, node id, run link) or hide (the error page with a reference only). Absent: the project decides. Status codes never change."
                }
            }
        }),
        fields,
        dsl_flags,
        layout: vec![
            LayoutItem::Field("route".to_string()),
            LayoutItem::Field("method".to_string()),
            LayoutItem::Field("__webhook_public_url".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("auth".to_string()), LayoutItem::Field("credential_id".to_string())] },
            LayoutItem::Field("role".to_string()),
            LayoutItem::Field("auth_optional".to_string()),
            LayoutItem::Field("errors".to_string()),
        ],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("A public page", "trigger.webhook --route /blog --method GET")
                .output(serde_json::json!({ "webhook": { "body": null, "params": {}, "query": { "page": "2" }, "headers": {}, "path": "/blog", "method": "GET" } }))
                .note("Served at `/wh/{owner}/{project}/blog`. Then a query, then `web.response.send --template pages/blog.tsx`."),
            crate::pipeline::model::NodeExample::dsl("A form POST", "trigger.webhook --route /contact --method POST")
                .output(serde_json::json!({ "webhook": { "body": { "email": "a@example.com", "message": "Hi" }, "params": {}, "query": {}, "headers": {}, "path": "/contact", "method": "POST" } }))
                .note("`<input name=\"email\">` arrives as `input.webhook.body.email` (`$trigger.body.email` anywhere later); a file field is `input.webhook.files.<name>`."),
            crate::pipeline::model::NodeExample::dsl("A protected admin route", "trigger.webhook --route /admin/posts/:id --method GET --auth jwt --credential jwt_main --role editor")
                .output(serde_json::json!({ "webhook": { "body": null, "params": { "id": "42" }, "query": {}, "headers": {}, "path": "/admin/posts/42", "method": "GET", "auth": { "sub": "u_1", "roles": ["editor"] } } }))
                .note("Without a valid token the request is refused before any node runs, and the credential's `auth_redirect` decides where a browser goes."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub route: String,
    #[serde(default = "default_method")]
    pub method: String,
    /// `"none"` (default), `"jwt"`, `"hmac"`, `"api_key"`.
    ///
    /// - `jwt`     — verifies `Authorization: Bearer <token>` against a `jwt_signing_key` credential.
    ///               Verified claims are `webhook.auth`.
    /// - `hmac`    — verifies `X-Hub-Signature-256: sha256=<hex>` (GitHub-style) against a credential.
    /// - `api_key` — verifies `X-API-Key: <key>` or `Authorization: ApiKey <key>` against a credential.
    /// - `none`    — no authentication (default).
    #[serde(default)]
    pub auth: String,
    /// Credential ID to use for auth verification (required when `auth != "none"`).
    #[serde(default)]
    pub credential_id: String,
    /// Roles allowed on this route; one entry of the JWT `roles` claim must
    /// match. Empty = any authenticated user may access.
    #[serde(default)]
    pub role: Vec<String>,
    #[serde(default)]
    pub auth_optional: bool,
    #[serde(default)]
    pub errors: String,
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
            payload: super::answer_under(ANSWER_KEY, payload),
            trace: vec![
                format!("node_kind={NODE_KIND}"),
                format!("method={}", self.config.method),
                format!("route={}", self.config.route),
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

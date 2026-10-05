//! `web.response.send` — answer the caller.
//!
//! One node for every response: without a source it answers the payload as
//! JSON; `--body` answers a value (a string as text, anything else JSON);
//! `--template` renders a TSX page; `--file` answers a project file, or a
//! stored file when it is a FileRef; `--path` answers a store key (`stored.rs`,
//! streamed by the HTTP layer with `Range`, `HEAD` and `304`). `--status`
//! and `--header` (repeat; a repeated name is sent twice) shape any of them.
//! It passes its payload on unchanged: what it answers travels to the HTTP
//! layer beside the payload, never in it.
//!
//! # Decision matrix for agents
//!
//! | Intent | DSL |
//! |---|---|
//! | Serve pipeline output as JSON | `\| web.response.send` |
//! | Serve specific field as JSON | `\| web.response.send --body "{{ input.query.rows }}"` |
//! | Render HTML page | `\| web.response.send --template pages/home.tsx` |
//! | Redirect | `\| web.response.send --status 303 --header "Location=/somewhere"` |
//! | Error with message | `\| web.response.send --status 403 --body "Access denied"` |
//! | Error page | `\| web.response.send --template pages/404.tsx --status 404` |
//! | Set session cookie | `\| web.response.send --status 303 --header "Location=/home" --header "Set-Cookie=zebflow_session={{ input.token.access_token }}; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly"` |
//! | Serve a project file (manifest, robots, icon) | `\| web.response.send --file pwa/manifest.webmanifest` |
//! | Serve a service worker | `\| web.response.send --file pwa/site.sw.ts --header Service-Worker-Allowed=/` |
//! | Serve one file out of a folder by URL parameter | `\| web.response.send --root pwa/icons --file "{{ $trigger.params.file }}"` |
//! | Serve a stored file (streamed, Range, 304) | `\| web.response.send --file "{{ input.file }}"` |
//! | Serve a stored file as a download | `\| web.response.send --file "{{ input.file }}" --filename "report.pdf"` |
//!
//! Every header is sent as written, `Set-Cookie` included: a session cookie
//! carries `Path=/; SameSite=Lax; HttpOnly` (and `Secure` behind HTTPS)
//! because its author wrote them.

mod answer;
mod file;
mod render;
mod stored;

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem, NodeCapability, NodeFieldDataSource, NodeFieldDef, NodeFieldType};
use crate::pipeline::nodes::shared::project_store::store_flag;
use crate::pipeline::nodes::shared::util::{metadata_scope, with_answer};
use crate::pipeline::nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler};
use crate::pipeline::{NodeDefinition, PipelineError};
use crate::platform::services::PlatformService;

pub use answer::{Head, body_envelope};
pub use file::{FileBody, FileResponse, resolve_file_rel_path};
pub use render::{CompiledPage, compile_page, render_compiled_page};
pub use stored::{CODE_NOT_FOUND, STORED_KEY, content_disposition};

pub const NODE_KIND: &str = "web.response.send";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";
/// Where the answer travels from this node to the HTTP layer. The engine
/// lifts it out of the payload (`PipelineOutput::response`), so it is never
/// part of what the next node receives.
pub const ENVELOPE_KEY: &str = "__zf_response";
pub const CODE_FILE: &str = "FW_NODE_WEB_RESPONSE_SEND_FILE";
pub const CODE_CONFIG: &str = answer::CODE_CONFIG;

fn flag(name: &str, key: &str, value: &str, description: &str) -> DslFlag {
    DslFlag {
        flag: name.to_string(),
        config_key: key.to_string(),
        description: description.to_string(),
        kind: DslFlagKind::Scalar,
        value: value.to_string(),
        ..Default::default()
    }
}

fn field(name: &str, label: &str, field_type: NodeFieldType, placeholder: &str, help: &str) -> NodeFieldDef {
    NodeFieldDef {
        name: name.to_string(),
        label: label.to_string(),
        field_type,
        placeholder: Some(placeholder.to_string()).filter(|p| !p.is_empty()),
        help: Some(help.to_string()),
        ..Default::default()
    }
}

// ── Definition ────────────────────────────────────────────────────────────────

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem, NodeCapability::Process],
        title: "Web Response".to_string(),
        description:
            "Answers the request — the caller receives it the moment this node runs — and passes its payload on unchanged: nodes after it keep \
             running, and a failure after it is recorded on the run without changing the answer. The first response of a run wins; a second one \
             sends nothing (recorded `empty`). No source: the payload as JSON, \
             200. At most one source: `--body VALUE` (a string answers text/plain, anything else JSON, unless a `Content-Type` header says otherwise); \
             `--template pages/x.tsx` (exact `file_list` path, `.tsx` required): render the page with the payload as its `input`; \
             `--file pwa/manifest.webmanifest`: a project file, content type by extension, a `.ts` compiled to JavaScript \
             (a service worker: `--file pwa/site.sw.ts` behind `trigger.webhook --route /sw.js`); `--root pwa/icons --file \"{{ $trigger.params.file }}\"`: \
             one file out of a folder, the name from the route, never outside it. \
             A stored file: `--file \"{{ input.file }}\"` with a FileRef (fs.file.put, fs.file.get --return file, an upload, one kept in a record), \
             or `--path KEY` (`--store`) with a store key — streamed from its store, never read into the run, typed by the FileRef's mime, \
             with `Range` (206/416), `HEAD`, `ETag`/`Last-Modified` and `304`, `Cache-Control: private, no-cache` unless a header says otherwise, \
             shown inline; `--filename NAME` makes it a download under that name. A stored file that is not there — or a `--file` / `--path` that resolved to null \
             — fails FW_NODE_WEB_RESPONSE_SEND_NOT_FOUND, which the route answers 404 (the run keeps the reason) unless an `:error` edge takes it. \
             A rendered page never carries the request into the browser: its hydration state has no request headers, cookies or trigger auth \
             (SSR still reads them; copy a value into a key of your own upstream to show it). \
             `--status N`, `--header K=V` (repeat; a repeated name is sent twice). \
             A redirect is `--status 303 --header \"Location=/home\"` — a Location without a 3xx, or a 3xx without a Location, is refused. \
             Every header is sent exactly as written, `Set-Cookie` too: nothing is added, so a session cookie writes its own attributes — \
             `--header \"Set-Cookie=zebflow_session={{ input.token.access_token }}; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly\"`, plus `; Secure` behind HTTPS. There is no `--route`; the route is the trigger's `--route`. \
             A 404 or 400 is a `logic.if` whose `false` pin reaches a second `web.response.send` with that `--status` — a script cannot set one."
                .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({
            "type": "object",
            "description": "The payload it was given, unchanged; the answer goes to the caller."
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            flag("--status", "status", "number", "HTTP status code (default 200). A 3xx needs a Location header."),
            DslFlag {
                kind: DslFlagKind::KeyValuePairs,
                ..flag("--header", "headers", "text", "A response header, e.g. --header \"Location=/home\". Sent exactly as written. Repeat for several; a repeated name is sent twice (two Set-Cookie headers are two cookies).")
            },
            flag("--body", "body", "json", "The answer: a string answers text/plain, anything else JSON, e.g. \"{{ input.query.rows }}\"."),
            flag("--template", "template", "text", "A TSX page under the source root, ending .tsx, e.g. pages/home.tsx. The payload is its input."),
            flag("--file", "file", "text", "A project file to answer, e.g. pwa/manifest.webmanifest or pwa/site.sw.ts (compiled); with --root, a bare filename inside it. A FileRef (\"{{ input.file }}\") answers that stored file, streamed."),
            flag("--root", "root", "text", "The project folder --file must be directly inside (no subfolders, no ..), e.g. pwa/icons for a /pwa/{file} route."),
            flag("--path", "path", "text", "A store key to answer, streamed from --store, e.g. \"reports/{{ input.query.rows[0].key }}\"."),
            DslFlag {
                description: "The store --path is read from: `local`, or the id of an `s3` credential. Saved explicitly when the pipeline is registered. A FileRef names its own store.".to_string(),
                ..store_flag()
            },
            flag("--filename", "filename", "text", "Answer the file as a download under this name (Content-Disposition: attachment). Default: inline."),
            DslFlag {
                kind: DslFlagKind::RepeatedList,
                ..flag("--script", "scripts", "text", "An external script URL for the page (with --template); repeat for several. Each must match the project's RWE allow-list.")
            },
        ],
        fields: vec![
            field("status", "Status Code", NodeFieldType::Text, "200", "HTTP status code (default 200). A 3xx needs a Location header."),
            field("headers", "Headers", NodeFieldType::KeyValuePairs, "", "Response headers: Location for a redirect, Set-Cookie for a cookie, Content-Type, Cache-Control."),
            field("body", "Body", NodeFieldType::Text, "{{ input.query.rows }}", "The answer: a string answers text, anything else JSON. Empty: the payload as JSON."),
            NodeFieldDef {
                data_source: Some(NodeFieldDataSource::TemplatesPages),
                ..field("template", "Template", NodeFieldType::Datalist, "pages/home.tsx", "A TSX page to render with the payload as its input.")
            },
            field("file", "Project file", NodeFieldType::Text, "pwa/site.sw.ts", "Answer this project file; content type by extension, .ts compiled. With a root, a bare filename."),
            field("root", "Root", NodeFieldType::Text, "pwa/icons", "The file must be directly inside this project folder."),
            field("path", "Store key", NodeFieldType::Text, "reports/a1.pdf", "Answer this stored file, streamed from the store."),
            field("store", "Store", NodeFieldType::Text, "", "`local`, or the id of an s3 credential. Empty: the project's default, saved when the pipeline is registered."),
            field("filename", "Download name", NodeFieldType::Text, "report.pdf", "Answer the file as a download under this name. Empty: shown inline."),
            field("scripts", "Scripts", NodeFieldType::Text, "https://cdn.example.com/app.js", "External script URLs for the page (template only)."),
        ],
        layout: vec![
            LayoutItem::Field("status".to_string()),
            LayoutItem::Field("headers".to_string()),
            LayoutItem::Field("body".to_string()),
            LayoutItem::Field("template".to_string()),
            LayoutItem::Row {
                row: vec![LayoutItem::Field("root".to_string()), LayoutItem::Field("file".to_string())],
            },
            LayoutItem::Row {
                row: vec![LayoutItem::Field("path".to_string()), LayoutItem::Field("store".to_string())],
            },
            LayoutItem::Field("filename".to_string()),
            LayoutItem::Field("scripts".to_string()),
        ],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Render a page", "web.response.send --template pages/posts.tsx")
                .note("The payload (e.g. `{ rows }`) is the page's `input`; the body must show no `RWE component error`."),
            crate::pipeline::model::NodeExample::dsl("Redirect after a form POST", r#"web.response.send --status 303 --header "Location=/admin/posts""#),
            crate::pipeline::model::NodeExample::dsl("JSON with a status", r#"web.response.send --status 400 --body "{{ { error: 'title is required' } }}""#),
            crate::pipeline::model::NodeExample::dsl("Plain text with a status", r#"web.response.send --status 401 --body "invalid credentials""#),
            crate::pipeline::model::NodeExample::dsl("Set the session cookie and go home", r#"web.response.send --status 303 --header "Location=/home" --header "Set-Cookie=zebflow_session={{ input.token.access_token }}; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly""#)
                .note("The header is sent as written: the cookie carries the attributes it names and no others. Add `; Secure` behind HTTPS. Logout: `--header \"Set-Cookie=zebflow_session=; Path=/; Max-Age=0\"`."),
            crate::pipeline::model::NodeExample::dsl("The web-app manifest, a file in the project", "web.response.send --file pwa/manifest.webmanifest")
                .note("Behind `trigger.webhook --route /manifest.webmanifest`; pages link it with `head.links: [{ rel: \"manifest\", href: \"/manifest.webmanifest\" }]`."),
            crate::pipeline::model::NodeExample::dsl("The service worker, compiled from TypeScript", "web.response.send --file pwa/site.sw.ts")
                .note("Behind `--path /sw.js`. The file starts with `self.__ZF = { version, source }`; the page registers it once with `navigator.serviceWorker.register(\"/sw.js\")`."),
            crate::pipeline::model::NodeExample::dsl("One icon out of a folder", r#"web.response.send --root pwa/icons --file "{{ $trigger.params.file }}""#)
                .note("Behind `--path /pwa/{file}`. The name may not leave the folder, so this is safe to expose."),
            crate::pipeline::model::NodeExample::dsl("A member's own file, or 404", r#"web.response.send --path "{{ input.query.rows[0]?.path ?? null }}""#)
                .note("After `trigger.webhook --auth jwt` and `sekejap.query.run … WHERE _key = $1 AND owner = $2` with `--param \"2={{ input.webhook.auth.sub }}\"`: the owner gets the bytes, anyone else finds no row and gets 404."),
            crate::pipeline::model::NodeExample::dsl("A stored key as a download", r#"web.response.send --path "exports/{{ $trigger.params.id }}.csv" --filename "export.csv""#),
        ],
        failure_semantics: vec![
            crate::pipeline::model::NodeFailureSemantic {
                code: CODE_FILE.to_string(),
                description: "`--file` is missing, leaves the project or its `--root`, or does not exist; a stored file's store cannot be read, or the object no longer matches its FileRef.".to_string(),
                ..Default::default()
            },
            crate::pipeline::model::NodeFailureSemantic {
                code: CODE_NOT_FOUND.to_string(),
                description: "The stored file `--file` or `--path` names is not in its store, or `--file` resolved to null. Unless an `:error` edge takes it, the route answers 404.".to_string(),
                ..Default::default()
            },
            crate::pipeline::model::NodeFailureSemantic {
                code: "FW_NODE_WEB_RESPONSE_SEND_COMPILE".to_string(),
                description: "The `.ts` named by `--file`, or the `--template` page, did not compile; the message names the file.".to_string(),
                ..Default::default()
            },
            crate::pipeline::model::NodeFailureSemantic {
                code: answer::CODE_REDIRECT.to_string(),
                description: "A Location header without a 3xx `--status`, or a 3xx (other than 304) without a Location.".to_string(),
                ..Default::default()
            },
            crate::pipeline::model::NodeFailureSemantic {
                code: answer::CODE_CONFIG.to_string(),
                description: "More than one of `--body`, `--template`, `--file`, `--path`; `--root` without `--file`; `--filename` without `--file` or `--path`.".to_string(),
                ..Default::default()
            },
        ],
        ..Default::default()
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
    /// The response body — arrives final (a whole `{{ }}` is its typed value).
    #[serde(default)]
    pub body: Option<Value>,
    /// Response headers: a value is a string, or a list when the name repeats.
    #[serde(default)]
    pub headers: Map<String, Value>,
    /// External script URLs for the page (`--script`, repeat).
    #[serde(default)]
    pub scripts: Vec<String>,
    /// A project path (`file.rs`), or a FileRef — a stored file (`stored.rs`).
    /// `Some(Value::Null)` is a `{{ }}` that resolved to nothing: a 404.
    #[serde(default, deserialize_with = "present")]
    pub file: Option<Value>,
    /// The folder `file` must be directly inside, when the name comes from the route.
    #[serde(default)]
    pub root: Option<String>,
    /// A store key to answer (`stored.rs`); `Some(Value::Null)`, as for
    /// `file`, is a `{{ }}` that resolved to nothing — a 404, never the
    /// payload answered in its place.
    #[serde(default, deserialize_with = "present")]
    pub path: Option<Value>,
    /// The store `path` is read from; saved explicitly at registration.
    #[serde(default)]
    pub store: Option<String>,
    /// A download name: `Content-Disposition: attachment`.
    #[serde(default)]
    pub filename: Option<String>,
}

/// A key that is present is `Some`, `null` included — absent stays `None`
/// through `#[serde(default)]`.
fn present<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<Value>, D::Error> {
    Value::deserialize(deserializer).map(Some)
}

impl Config {
    /// `--file` as a project path, when it is one.
    fn repo_file(&self) -> Option<&str> {
        self.file.as_ref().and_then(Value::as_str).filter(|_| self.path.is_none())
    }

    /// `--filename`, trimmed; empty is none.
    fn download_name(&self) -> Option<&str> {
        self.filename.as_deref().map(str::trim).filter(|n| !n.is_empty())
    }
}

// ── Node (non-template path only) ────────────────────────────────────────────
// Template path is handled by BasicPipelineEngine via InlineWebResponse.

pub struct Node {
    config: Config,
    /// The project source root `--file` resolves against; `None` outside a project.
    template_root: Option<PathBuf>,
    /// The project's stores, for a stored file; `None` outside the platform.
    platform: Option<Arc<PlatformService>>,
}

impl Node {
    pub fn new(config: Config, template_root: Option<PathBuf>) -> Self {
        Self { config, template_root, platform: None }
    }

    /// The node with the platform its stored files are read through.
    pub fn with_platform(mut self, platform: Option<Arc<PlatformService>>) -> Self {
        self.platform = platform;
        self
    }

    /// The `--file` FileRef or `--path` response: decided here, streamed by
    /// the HTTP layer (`stored.rs`).
    fn stored_envelope(&self, head: Head, metadata: &Value) -> Result<Value, PipelineError> {
        let Some(platform) = &self.platform else {
            return Err(PipelineError::new(CODE_FILE, "a stored file needs the platform; this engine has none"));
        };
        let (owner, project, _, _) = metadata_scope(metadata)?;
        let source = match (&self.config.path, &self.config.file) {
            (Some(Value::Null), _) => Value::Null,
            (Some(Value::String(path)), _) => Value::String(path.clone()),
            (Some(_), _) => return Err(PipelineError::new(CODE_FILE, "--path is a store key, a string")),
            (None, Some(file)) => file.clone(),
            (None, None) => Value::Null,
        };
        stored::stored_envelope(platform, owner, project, &source, self.config.store.as_deref(), self.config.download_name(), head)
    }

    /// The `--file` response: bytes read from the project, typed by extension,
    /// a script compiled and given its prelude.
    fn file_envelope(&self, mut head: Head) -> Result<Value, PipelineError> {
        let rel = resolve_file_rel_path(self.config.root.as_deref(), self.config.repo_file().unwrap_or_default())?;
        let Some(root) = self.template_root.as_deref() else {
            return Err(PipelineError::new(CODE_FILE, "template_root is not configured on this pipeline engine"));
        };
        // Through the repository reader: no `..`, no link on the way, capped.
        let bytes = crate::pipeline::nodes::shared::project_store::read_repo_file(root, &rel, CODE_FILE)?;
        let served = FileResponse::from_bytes(&rel, bytes)?;
        head.default_header("Content-Type", served.content_type);
        head.default_header("Cache-Control", "no-cache");
        if let Some(name) = self.config.download_name() {
            head.default_header("Content-Disposition", &content_disposition("attachment", name));
        }
        let mut envelope = head.envelope();
        match served.body {
            FileBody::Text(text) => envelope.insert("text".to_string(), Value::String(text)),
            FileBody::Bytes(b64) => envelope.insert("body_base64".to_string(), Value::String(b64)),
        };
        Ok(Value::Object(envelope))
    }
}

/// The payload passed on, with the answer beside it for the engine to lift.
pub fn with_envelope(payload: &Value, envelope: Value) -> Value {
    with_answer(payload, json!({ ENVELOPE_KEY: envelope }))
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
        // Every value arrives final: {{ }} resolution happened engine-side
        // before this node ran (docs/contracts/kinds/node-io).
        let head = self.config.check()?;
        let status = head.status;
        let (envelope, what) = if let Some(rel) = self.config.repo_file() {
            (self.file_envelope(head)?, format!("file={rel}"))
        } else if self.config.file.is_some() || self.config.path.is_some() {
            let envelope = self.stored_envelope(head, &input.metadata)?;
            let what = format!("stored={}", envelope[STORED_KEY]["key"].as_str().unwrap_or_default());
            (envelope, what)
        } else {
            (body_envelope(&self.config, &head, &input.payload), format!("status={status:?}"))
        };
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: with_envelope(&input.payload, envelope),
            trace: vec![format!("node_kind={NODE_KIND}"), what],
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::pipeline::nodes::{NodeExecutionInput, NodeHandler};

    use super::{Config, ENVELOPE_KEY, INPUT_PIN_IN, Node};

    async fn run(config: serde_json::Value, payload: serde_json::Value) -> serde_json::Value {
        let config: Config = serde_json::from_value(config).expect("config");
        Node::new(config, None)
            .execute_async(NodeExecutionInput {
                node_id: "n0".to_string(),
                input_pin: INPUT_PIN_IN.to_string(),
                payload,
                metadata: json!({}),
                bus: None,
            })
            .await
            .expect("execute web.response.send")
            .payload
    }

    #[tokio::test]
    async fn default_json_response_uses_upstream_payload_as_body() {
        let input_payload = json!({ "ok": true, "rows": [{ "id": 1, "name": "Alpha" }] });
        let output = run(json!({}), input_payload.clone()).await;
        assert_eq!(output[ENVELOPE_KEY]["json"], input_payload);
    }

    #[tokio::test]
    async fn the_payload_passes_on_unchanged_beside_the_answer() {
        let input_payload = json!({ "user": { "id": 7 } });
        let mut output = run(json!({ "status": 303, "headers": { "Location": "/home" } }), input_payload.clone()).await;
        let envelope = output.as_object_mut().unwrap().remove(ENVELOPE_KEY).expect("envelope");
        assert_eq!(output, input_payload, "no key added");
        assert_eq!(envelope["status"], 303);
        assert_eq!(envelope["headers"], json!([["Location", "/home"]]));
    }

    #[tokio::test]
    async fn a_redirect_is_refused_at_run_without_its_other_half() {
        let config: Config = serde_json::from_value(json!({ "headers": { "Location": "/home" } })).unwrap();
        let err = Node::new(config, None)
            .execute_async(NodeExecutionInput {
                node_id: "n0".to_string(),
                input_pin: INPUT_PIN_IN.to_string(),
                payload: json!({}),
                metadata: json!({}),
                bus: None,
            })
            .await
            .unwrap_err();
        assert_eq!(err.code, "FW_NODE_WEB_RESPONSE_SEND_REDIRECT");
    }

    #[tokio::test]
    async fn a_file_is_confined_to_its_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("pwa/icons")).unwrap();
        std::fs::write(dir.path().join("pwa/icons/a.txt"), "inside").unwrap();
        std::fs::write(dir.path().join("pwa/secret.txt"), "outside").unwrap();
        let node = |file: &str| {
            let config: Config = serde_json::from_value(json!({ "root": "pwa/icons", "file": file })).unwrap();
            Node::new(config, Some(dir.path().to_path_buf()))
        };
        let input = || NodeExecutionInput {
            node_id: "n0".to_string(),
            input_pin: INPUT_PIN_IN.to_string(),
            payload: json!({}),
            metadata: json!({}),
            bus: None,
        };
        let out = node("a.txt").execute_async(input()).await.expect("inside the root").payload;
        assert_eq!(out[ENVELOPE_KEY]["text"], "inside");
        assert_eq!(out[ENVELOPE_KEY]["headers"][0], json!(["Content-Type", "text/plain; charset=utf-8"]));
        for escape in ["../secret.txt", "sub/a.txt"] {
            let err = node(escape).execute_async(input()).await.unwrap_err();
            assert_eq!(err.code, super::CODE_FILE, "{escape}");
        }
    }
}

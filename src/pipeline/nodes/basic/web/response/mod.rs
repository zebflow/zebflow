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
//! | Serve specific field as JSON | `\| web.response --body "{{ input.rows }}"` |
//! | Render HTML page | `\| web.response --template pages/home.tsx` |
//! | Redirect | `\| web.response --location /somewhere` |
//! | Error with message | `\| web.response --status 403 --message "Access denied"` |
//! | Error page | `\| web.response --template pages/404.tsx --status 404` |
//! | Set session cookie | `\| web.response --template pages/home.tsx --set-cookie "name=session,value={{ input.token }},http-only"` |
//! | Serve a project file (manifest, robots, icon) | `\| web.response --file pwa/manifest.webmanifest` |
//! | Serve a service worker | `\| web.response --file pwa/site.sw.ts --header Service-Worker-Allowed=/` |
//! | Serve one file out of a folder by URL parameter | `\| web.response --folder pwa/icons --file "{{ input.params.file }}"` |
//!
//! # `--file` — a project file as the response
//!
//! The content type follows the extension (`.webmanifest`, `.json`, `.js`,
//! `.css`, `.txt`, `.xml`, `.svg`, `.png`, `.jpg`, `.webp`, `.ico`, `.woff2`,
//! `.pdf`, `.md`, `.html`). A `.ts` file is compiled to JavaScript first. Every
//! script (`.ts`, `.js`) starts with one line the platform adds,
//! `self.__ZF = Object.freeze({ version, source })` — the build and the file's
//! hash — so a service worker can key its cache on them. Everything else is
//! served byte for byte. `Cache-Control: no-cache` unless `--header` says
//! otherwise. A worker controls only pages under the directory it is served
//! from, so the site's worker answers at `/sw.js`, and needs
//! `Service-Worker-Allowed: /` only when served from deeper.
//!
//! With `--folder`, `--file` is a bare filename — usually a `{{ }}` from the
//! route — and may only name a file directly inside that folder: no slashes,
//! no `..`. That is what makes `/pwa/{file}` safe to expose.
//!
//! # Cookie spec format (`--set-cookie`)
//!
//! Comma-separated key=value pairs (or boolean flags):
//! ```text
//! name=session,value={{ input.token }},http-only,max-age=86400,secure,same-site=Strict,path=/
//! ```
//! - `name=<NAME>` — cookie name (required)
//! - `value=<VALUE>` — cookie value; a literal or `{{ expr }}` (resolved engine-side before this node runs)
//! - `http-only` — sets HttpOnly flag
//! - `secure` — sets Secure flag
//! - `max-age=<SECS>` — Max-Age directive (default 900)
//! - `same-site=<Lax|Strict|None>` — SameSite directive (default Lax)
//! - `path=<PATH>` — cookie Path (default /)

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::language::LanguageEngine;
use crate::pipeline::model::NodeCapability;
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
        capabilities: vec![NodeCapability::Filesystem, NodeCapability::Process],
        title: "Web Response".to_string(),
        description:
            "Ends the request with the response you mean — always the last node of a webhook pipeline. No flags: the payload as JSON, \
             200. `--template pages/x.tsx` (exact `file_list` path, `.tsx` required): render the page with the payload as its `input`. \
             `--location /path`: redirect (302 unless `--status 303`). `--status`, `--set-cookie \"name=…,value=…,http-only,max-age=…\"`, \
             `--header K=V`, `--message \"text\"`, `--body \"{{ expr }}\"`. There is no `--route`; the route is the trigger's `--path`. \
             A 404 or 400 is a `logic.if` whose `false` pin reaches a second `web.response` with that `--status` — a script cannot set one. \
             `--file pwa/manifest.webmanifest`: answer a project file, content type by extension, a `.ts` compiled to JavaScript \
             (a service worker: `--file pwa/site.sw.ts` behind `--path /sw.js`). `--folder pwa/icons --file \"{{ input.params.file }}\"`: \
             one file out of a folder, the name from the route, never outside it."
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
                "body":        { "description": "Response body — literal or {{ expr }}; a whole {{ }} carries its typed value (object, array)." },
                "set_cookie":  { "type": "string",  "description": "Cookie spec string." },
                "headers":     { "type": "object",  "description": "Extra response headers." },
                "load_scripts":{ "type": "string",  "description": "External scripts (template mode)." },
                "file":        { "type": "string",  "description": "Project file to answer (or, with folder, a bare filename inside it)." },
                "folder":      { "type": "string",  "description": "Project folder that file must be directly inside." }
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--template".to_string(),
                config_key: "template".to_string(),
                description: "TSX page file relative to the project source root, e.g. pages/home.tsx (the .tsx is required). \
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
                config_key: "body".to_string(),
                description:
                    "Response body — literal or {{ expr }}, e.g. \"{{ input.rows }}\" to answer with that value."
                        .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--set-cookie".to_string(),
                config_key: "set_cookie".to_string(),
                description:
                    "Cookie spec: name=NAME,value={{ expr }},http-only,max-age=SECS,secure,same-site=Lax"
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
            DslFlag {
                flag: "--file".to_string(),
                config_key: "file".to_string(),
                description: "A project file to answer as the response, e.g. pwa/manifest.webmanifest or pwa/site.sw.ts (compiled). \
                    Content type by extension. With --folder, a bare filename inside that folder — usually \"{{ input.params.file }}\"."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--folder".to_string(),
                config_key: "folder".to_string(),
                description: "Project folder --file must be directly inside (no subfolders, no ..), e.g. pwa/icons for a /pwa/{file} route."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType, NodeFieldDataSource};
            vec![
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
                    help: Some("HTTP status code to return. Defaults to 200 when omitted.".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "location".to_string(),
                    label: "Redirect URL".to_string(),
                    field_type: NodeFieldType::Text,
                    placeholder: Some("/auth/login".to_string()),
                    help: Some("Redirect target used with 3xx status codes.".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "body".to_string(),
                    label: "Body Path".to_string(),
                    field_type: NodeFieldType::Text,
                    placeholder: Some("{{ input.rows }}".to_string()),
                    help: Some("JSON path into the pipeline payload to serve as response body (JSON mode only).".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "message".to_string(),
                    label: "Message".to_string(),
                    field_type: NodeFieldType::Text,
                    placeholder: Some("Access denied".to_string()),
                    help: Some("Plain fallback response message when no template/body path is used.".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "set_cookie".to_string(),
                    label: "Set-Cookie".to_string(),
                    field_type: NodeFieldType::Text,
                    placeholder: Some("name=session,value={{ input.token }},http-only,max-age=86400".to_string()),
                    help: Some("Cookie specification string for a Set-Cookie header.".to_string()),
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
                    name: "file".to_string(),
                    label: "Project file".to_string(),
                    field_type: NodeFieldType::Text,
                    placeholder: Some("pwa/site.sw.ts".to_string()),
                    help: Some("Answer this project file; content type by extension, .ts compiled. With a folder, a bare filename.".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "folder".to_string(),
                    label: "Folder".to_string(),
                    field_type: NodeFieldType::Text,
                    placeholder: Some("pwa/icons".to_string()),
                    help: Some("The file must be directly inside this project folder.".to_string()),
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
                    LayoutItem::Field("body".to_string()),
                ],
            },
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("folder".to_string()),
                    LayoutItem::Field("file".to_string()),
                ],
            },
            LayoutItem::Field("set_cookie".to_string()),
            LayoutItem::Field("headers".to_string()),
            LayoutItem::Field("load_scripts".to_string()),
        ],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Render a page", "web.response --template pages/posts.tsx")
                .note("The payload (e.g. `{ rows }`) is the page's `input`; the body must show no `RWE component error`."),
            crate::pipeline::model::NodeExample::dsl("Redirect after a form POST", "web.response --location /admin/posts --status 303"),
            crate::pipeline::model::NodeExample::dsl("JSON with a status", r#"web.response --status 400 --body "{{ { error: 'title is required' } }}""#),
            crate::pipeline::model::NodeExample::dsl("Set the session cookie and go home", r#"web.response --location /home --set-cookie "name=zebflow_session,value={{ input.access_token }},http-only,max-age=86400,same-site=Lax""#),
            crate::pipeline::model::NodeExample::dsl("The web-app manifest, a file in the project", "web.response --file pwa/manifest.webmanifest")
                .note("Behind `trigger.webhook --path /manifest.webmanifest`; pages link it with `head.links: [{ rel: \"manifest\", href: \"/manifest.webmanifest\" }]`."),
            crate::pipeline::model::NodeExample::dsl("The service worker, compiled from TypeScript", "web.response --file pwa/site.sw.ts")
                .note("Behind `--path /sw.js`. The file starts with `self.__ZF = { version, source }`; the page registers it once with `navigator.serviceWorker.register(\"/sw.js\")`."),
            crate::pipeline::model::NodeExample::dsl("One icon out of a folder", r#"web.response --folder pwa/icons --file "{{ input.params.file }}""#)
                .note("Behind `--path /pwa/{file}`. The name may not leave the folder, so this is safe to expose."),
        ],
        failure_semantics: vec![
            crate::pipeline::model::NodeFailureSemantic {
                code: "WEB_RESPONSE_FILE".to_string(),
                description: "`--file` is missing, leaves the project or its `--folder`, or does not exist.".to_string(),
                ..Default::default()
            },
            crate::pipeline::model::NodeFailureSemantic {
                code: "WEB_RESPONSE_COMPILE".to_string(),
                description: "The `.ts` named by `--file` did not parse; the message names the file.".to_string(),
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
    /// Redirect URL. Implies 302 when no `status` is set.
    #[serde(default)]
    pub location: Option<String>,
    /// Plain-text response body.
    #[serde(default)]
    pub message: Option<String>,
    /// The response body — arrives final (a whole `{{ }}` is its typed value).
    #[serde(default)]
    pub body: Option<Value>,
    /// Cookie spec string (see module docs for format).
    #[serde(default)]
    pub set_cookie: Option<String>,
    /// Extra response headers.
    #[serde(default)]
    pub headers: Map<String, Value>,
    /// External script URLs for template mode (comma-separated).
    #[serde(default)]
    pub load_scripts: Option<String>,
    /// A project file to answer (module docs, `--file`).
    #[serde(default)]
    pub file: Option<String>,
    /// The folder `file` must be directly inside, when the name comes from the route.
    #[serde(default)]
    pub folder: Option<String>,
}

// ── Node (non-template path only) ────────────────────────────────────────────
// Template path is handled by BasicPipelineEngine via InlineWebResponse.

pub struct Node {
    config: Config,
    /// The project source root `--file` resolves against; `None` outside a project.
    template_root: Option<PathBuf>,
}

impl Node {
    pub fn new(config: Config, template_root: Option<PathBuf>) -> Self {
        Self { config, template_root }
    }

    /// The `--file` response: bytes read from the project, typed by extension,
    /// a script compiled and given its prelude.
    fn file_envelope(&self) -> Result<Value, PipelineError> {
        let rel = resolve_file_rel_path(self.config.folder.as_deref(), self.config.file.as_deref().unwrap_or_default())?;
        let Some(root) = self.template_root.as_deref() else {
            return Err(PipelineError::new("WEB_RESPONSE_FILE", "template_root is not configured on this pipeline engine"));
        };
        let abs = root.join(&rel);
        if !abs.starts_with(root) || !abs.is_file() {
            return Err(PipelineError::new("WEB_RESPONSE_FILE", format!("file '{rel}' not found in the project")));
        }
        let bytes = std::fs::read(&abs).map_err(|e| PipelineError::new("WEB_RESPONSE_FILE", format!("failed reading '{rel}': {e}")))?;
        let served = FileResponse::from_bytes(&rel, bytes)?;
        let mut headers = self.config.headers.clone();
        headers.entry("Content-Type".to_string()).or_insert_with(|| json!(served.content_type));
        headers.entry("Cache-Control".to_string()).or_insert_with(|| json!("no-cache"));
        let status = self.config.status.unwrap_or(200);
        Ok(match served.body {
            FileBody::Text(text) => json!({ "status": status, "message": text, "headers": headers, "set_cookie": self.config.set_cookie.as_deref().and_then(parse_cookie_spec) }),
            FileBody::Bytes(b64) => json!({ "status": status, "body_base64": b64, "headers": headers, "set_cookie": self.config.set_cookie.as_deref().and_then(parse_cookie_spec) }),
        })
    }
}

/// What `--file` answers with, worked out from the name and the bytes. Pure,
/// so a test can hand it a name and a source without a project.
#[derive(Debug)]
pub struct FileResponse {
    pub content_type: &'static str,
    pub body: FileBody,
}

#[derive(Debug)]
pub enum FileBody {
    Text(String),
    /// Base64 — the response envelope travels as JSON.
    Bytes(String),
}

impl FileResponse {
    pub fn from_bytes(rel: &str, bytes: Vec<u8>) -> Result<Self, PipelineError> {
        let ext = rel.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        let (content_type, text) = match ext.as_str() {
            "webmanifest" => ("application/manifest+json; charset=utf-8", true),
            "json" | "map" => ("application/json; charset=utf-8", true),
            "js" | "mjs" | "ts" => ("text/javascript; charset=utf-8", true),
            "css" => ("text/css; charset=utf-8", true),
            "txt" => ("text/plain; charset=utf-8", true),
            "md" => ("text/markdown; charset=utf-8", true),
            "html" | "htm" => ("text/html; charset=utf-8", true),
            "xml" => ("application/xml; charset=utf-8", true),
            "svg" => ("image/svg+xml", true),
            "csv" => ("text/csv; charset=utf-8", true),
            "png" => ("image/png", false),
            "jpg" | "jpeg" => ("image/jpeg", false),
            "gif" => ("image/gif", false),
            "webp" => ("image/webp", false),
            "ico" => ("image/x-icon", false),
            "woff2" => ("font/woff2", false),
            "woff" => ("font/woff", false),
            "pdf" => ("application/pdf", false),
            "wasm" => ("application/wasm", false),
            _ => ("application/octet-stream", false),
        };
        if !text {
            use base64::Engine as _;
            return Ok(Self { content_type, body: FileBody::Bytes(base64::engine::general_purpose::STANDARD.encode(bytes)) });
        }
        let source = String::from_utf8(bytes).map_err(|_| PipelineError::new("WEB_RESPONSE_FILE", format!("'{rel}' is not UTF-8 text")))?;
        let body = match ext.as_str() {
            "ts" => {
                let compiled = crate::rwe::core::deno_worker::transpile_ts(&source)
                    .map_err(|e| PipelineError::new("WEB_RESPONSE_COMPILE", format!("{rel}: {}", e.message)))?;
                format!("{}{compiled}", script_prelude(&source))
            }
            "js" | "mjs" => format!("{}{source}", script_prelude(&source)),
            _ => source,
        };
        Ok(Self { content_type, body: FileBody::Text(body) })
    }
}

/// `self.__ZF = Object.freeze({ version, source })` — the platform build and the
/// file's hash, so a worker keys its cache on both.
fn script_prelude(source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    format!("self.__ZF = Object.freeze({{ version: {}, source: {} }});\n", json!(crate::version::APP_VERSION), json!(&digest[..16]))
}

/// The project-relative path `--file` (and `--folder`) name, or why not. With
/// a folder the file is a bare name and stays directly inside it; without one
/// it is a project path that never climbs out. Pure.
pub fn resolve_file_rel_path(folder: Option<&str>, file: &str) -> Result<String, PipelineError> {
    fn clean(raw: &str, what: &str) -> Result<Vec<String>, PipelineError> {
        let mut parts = Vec::new();
        for part in raw.trim().replace('\\', "/").split('/') {
            match part.trim() {
                "" | "." => continue,
                ".." => return Err(PipelineError::new("WEB_RESPONSE_FILE", format!("{what} '{raw}' escapes the project"))),
                p => parts.push(p.to_string()),
            }
        }
        Ok(parts)
    }
    let file = file.trim();
    if file.is_empty() {
        return Err(PipelineError::new("WEB_RESPONSE_FILE", "web.response --file needs a path, e.g. pwa/manifest.webmanifest"));
    }
    match folder.map(str::trim).filter(|f| !f.is_empty()) {
        Some(folder) => {
            if file.contains('/') || file.contains('\\') || file == ".." || file == "." {
                return Err(PipelineError::new("WEB_RESPONSE_FILE", format!("with --folder, --file must be a bare filename; got '{file}'")));
            }
            let mut parts = clean(folder, "folder")?;
            if parts.is_empty() {
                return Err(PipelineError::new("WEB_RESPONSE_FILE", "--folder is empty"));
            }
            parts.push(file.to_string());
            Ok(parts.join("/"))
        }
        None => {
            let parts = clean(file, "file")?;
            if parts.is_empty() {
                return Err(PipelineError::new("WEB_RESPONSE_FILE", "web.response --file needs a path"));
            }
            Ok(parts.join("/"))
        }
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
        if self.config.file.is_some() || self.config.folder.is_some() {
            let envelope = self.file_envelope()?;
            return Ok(NodeExecutionOutput {
                output_pins: vec![OUTPUT_PIN_OUT.to_string()],
                payload: json!({ "__zf_response": envelope }),
                trace: vec![format!("node_kind={NODE_KIND}"), format!("file={}", self.config.file.as_deref().unwrap_or_default())],
            });
        }
        // Location arrives final: {{ }} resolution happened engine-side
        // before this node ran (docs/contracts/kinds/node-io).
        let location = self.config.location.clone();

        let status = self
            .config
            .status
            .or_else(|| if location.is_some() { Some(302) } else { None });

        let cookie = self
            .config
            .set_cookie
            .as_deref()
            .and_then(parse_cookie_spec);

        let body = self
            .config
            .body
            .clone()
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
/// Format: `name=session,value={{ input.token }},http-only,max-age=86400,secure,same-site=Strict,path=/`
pub fn parse_cookie_spec(spec: &str) -> Option<Value> {
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
            // The spec string arrives with {{ }} already interpolated
            // engine-side, so the value here is the value.
            value = v.to_string();
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

    // An empty value is how a cookie is cleared (`value=,max-age=0` on logout);
    // only a missing name makes the spec meaningless.
    if name.is_empty() {
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
/// `ctx.auth`, `ctx.params`, `ctx.query` and raw URL context regardless of what
/// upstream nodes did to the payload. Explicit page routes take precedence;
/// otherwise the browser pathname wins over the pipeline-relative route.
fn inject_trigger_fields(mut state: Value, metadata: &Value) -> Value {
    // The route the request arrived on. `usePathname` reads this during server
    // rendering; without it the server has no idea what path it is rendering
    // and the browser's `location.pathname` disagrees on the first client
    // render — which is a hydration tear, not a cosmetic difference.
    let route = metadata
        .get("trigger")
        .and_then(|trigger| trigger.get("pathname"))
        .and_then(Value::as_str)
        .or_else(|| metadata.get("route").and_then(Value::as_str));
    if let Some(route) = route {
        if let Value::Object(ref mut map) = state {
            if !map.contains_key("route") {
                map.insert("route".to_string(), Value::String(route.to_string()));
            }
        }
    }

    let Some(trigger) = metadata.get("trigger") else {
        return state;
    };
    let Value::Object(ref mut map) = state else {
        return state;
    };

    // Preserve raw search separately from the legacy query object, including
    // repeated parameters and percent escapes needed by SSR URL hooks.
    // Explicit page-state fields retain their existing precedence.
    for key in &["params", "query", "search", "headers"] {
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
        let node = Node::new(Config::default(), None);
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
                bus: None,
            })
            .await
            .expect("execute web.response");

        assert_eq!(output.payload["__zf_response"]["body"], input_payload);
    }

    #[test]
    fn page_route_uses_browser_path_without_changing_pipeline_route() {
        let metadata = json!({
            "route": "/probe",
            "trigger": {"pathname": "/wh/owner/project/probe"}
        });
        let injected = super::inject_trigger_fields(json!({}), &metadata);
        assert_eq!(injected["route"], "/wh/owner/project/probe");
        assert_eq!(metadata["route"], "/probe");
        let explicit = super::inject_trigger_fields(json!({"route": "/custom"}), &metadata);
        assert_eq!(explicit["route"], "/custom");
        let fallback = super::inject_trigger_fields(json!({}), &json!({"route": "/preview"}));
        assert_eq!(fallback["route"], "/preview");
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
                "search": "?page=1&tag=one&tag=two",
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
        assert_eq!(injected["search"], "?page=1&tag=one&tag=two");
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

#[cfg(test)]
mod file_mode_tests {
    use super::*;

    #[test]
    fn a_manifest_is_typed_by_its_extension_and_served_as_written() {
        let r = FileResponse::from_bytes("pwa/manifest.webmanifest", br#"{"name":"RESEARCHSITE"}"#.to_vec()).unwrap();
        assert_eq!(r.content_type, "application/manifest+json; charset=utf-8");
        assert!(matches!(r.body, FileBody::Text(ref t) if t == r#"{"name":"RESEARCHSITE"}"#));
    }

    #[test]
    fn a_typescript_worker_is_compiled_and_gets_the_prelude() {
        let src = "const SHELL: string[] = [\"/\"];\nself.addEventListener(\"install\", (e: any) => e.waitUntil(caches.open(self.__ZF.version)));\n";
        let r = FileResponse::from_bytes("pwa/site.sw.ts", src.as_bytes().to_vec()).unwrap();
        assert_eq!(r.content_type, "text/javascript; charset=utf-8");
        let FileBody::Text(js) = r.body else { panic!("text") };
        assert!(js.starts_with("self.__ZF = Object.freeze({ version: \""), "{js}");
        assert!(!js.contains(": string[]"), "types stripped: {js}");
        assert!(js.contains("addEventListener(\"install\""));
    }

    #[test]
    fn a_png_travels_as_base64_with_its_type() {
        let r = FileResponse::from_bytes("pwa/icons/icon-192.png", vec![0x89, b'P', b'N', b'G']).unwrap();
        assert_eq!(r.content_type, "image/png");
        assert!(matches!(r.body, FileBody::Bytes(ref b) if b == "iVBORw=="));
    }

    #[test]
    fn a_broken_typescript_names_the_file() {
        let err = FileResponse::from_bytes("pwa/bad.sw.ts", b"const = ;".to_vec()).unwrap_err();
        assert_eq!(err.code, "WEB_RESPONSE_COMPILE");
        assert!(err.message.starts_with("pwa/bad.sw.ts"));
    }

    #[test]
    fn a_file_path_never_climbs_out_of_the_project() {
        assert_eq!(resolve_file_rel_path(None, "pwa/manifest.webmanifest").unwrap(), "pwa/manifest.webmanifest");
        assert_eq!(resolve_file_rel_path(None, "./pwa//site.sw.ts").unwrap(), "pwa/site.sw.ts");
        assert_eq!(resolve_file_rel_path(None, "../secrets").unwrap_err().code, "WEB_RESPONSE_FILE");
        assert_eq!(resolve_file_rel_path(None, "").unwrap_err().code, "WEB_RESPONSE_FILE");
    }

    #[test]
    fn with_a_folder_the_name_from_the_route_stays_inside_it() {
        assert_eq!(resolve_file_rel_path(Some("pwa/icons"), "icon-192.png").unwrap(), "pwa/icons/icon-192.png");
        for bad in ["../site.sw.ts", "sub/icon.png", "..", "", "pwa/icons/x.png"] {
            assert_eq!(resolve_file_rel_path(Some("pwa/icons"), bad).unwrap_err().code, "WEB_RESPONSE_FILE", "{bad}");
        }
        assert_eq!(resolve_file_rel_path(Some("../"), "x.png").unwrap_err().code, "WEB_RESPONSE_FILE");
    }
}

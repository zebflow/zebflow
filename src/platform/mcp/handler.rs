//! Zebflow MCP handler exposing project-scoped tools.
//!
//! Thin adapter over `PlatformOps`. Each `#[tool]` method:
//! 1. Extracts + validates the MCP session
//! 2. Checks tool capability authorization
//! 3. Delegates to `PlatformOps`
//! 4. Wraps the text result in `CallToolResult` (navigate field is ignored for MCP)

use std::sync::Arc;

use axum::body::{Body, Bytes, to_bytes};
use axum::extract::OriginalUri;
use axum::http::{self, HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use base64::Engine as _;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::handler::server::{ServerHandler, tool::Extension};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, GetPromptRequestParams, GetPromptResult,
    ListPromptsResult, ListToolsResult, PaginatedRequestParams, Prompt, PromptMessage,
    PromptMessageContent, PromptMessageRole, ServerCapabilities, ServerInfo, Tool,
};
use rmcp::schemars::JsonSchema;
use rmcp::service::RequestContext;
use rmcp::{ErrorData as McpError, RoleServer, schemars, tool, tool_router};

use crate::infra::execution::placement::ProjectRuntimePlacementTarget;
use crate::platform::model::{McpSession, mcp_tool_capability};
use crate::platform::services::{PlatformOps, PlatformService};

const INTERNAL_CLUSTER_TOKEN_HEADER: &str = "x-zebflow-cluster-token";
const INTERNAL_MCP_SESSION_HEADER: &str = "x-zebflow-mcp-session";
const MCP_PROXY_BODY_LIMIT: usize = 16 * 1024 * 1024;

// ── Parameter structs (MCP schema only) ──────────────────────────────────────

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineGetParams {
    /// Source-relative path of the pipeline (e.g. "my-pipeline.zf.json").
    file_rel_path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineListParams {
    /// Optional semantic filter across path, title, description, trigger kind, and trigger summary.
    #[schemars(with = "String")]
    query: Option<String>,
    /// Optional glob to filter pipeline files (e.g. "api/*.zf.json").
    #[schemars(with = "String")]
    glob: Option<String>,
    /// Optional status filter: "active", "stale" (active, but changed since activation), "draft", or "all".
    #[schemars(with = "String")]
    status: Option<String>,
    /// Optional trigger kind filter: "webhook", "schedule", "function", or full "n.trigger.webhook".
    #[schemars(with = "String")]
    trigger_kind: Option<String>,
    /// Optional cap on returned rows. Compact defaults to a small cap.
    #[schemars(with = "u32")]
    limit: Option<u32>,
    /// Output format: "compact" (default), "json" for full legacy metadata, or "tree".
    #[schemars(with = "String")]
    format: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct FileReadParams {
    /// Relative path to the template file (e.g. "pages/home.tsx").
    rel_path: String,
    /// 1-based starting line number. Omit to read the full file.
    #[schemars(with = "u32")]
    offset: Option<u32>,
    /// Number of lines to return. Omit to read to the end.
    #[schemars(with = "u32")]
    limit: Option<u32>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct FileListParams {
    /// Optional glob to filter files (e.g. "pages/*.tsx", "**/*.tsx"). Omit to list all files.
    #[schemars(with = "String")]
    glob: Option<String>,
    /// Optional semantic filter across path, kind, title, description, and keywords.
    #[schemars(with = "String")]
    query: Option<String>,
    /// Optional template kind filter: "page", "component", "script", "style", "other", or "all".
    #[schemars(with = "String")]
    kind: Option<String>,
    /// Optional cap on returned rows. Compact defaults to a small cap.
    #[schemars(with = "u32")]
    limit: Option<u32>,
    /// Output format: "compact" (default), "json" for full legacy workspace data, or "tree".
    #[schemars(with = "String")]
    format: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct HelpParams {
    /// Help path to load (e.g. "pipeline", "web/hooks", "pipeline/nodes", "tool").
    /// Pass empty string or omit for the full index.
    #[schemars(with = "String")]
    topic: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct FileWriteParams {
    /// Path relative to the project source root (e.g. "pages/blog-home.tsx", "components/card.tsx", "docs/schema.md").
    rel_path: String,
    /// Full file content to write.
    content: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct FileSearchParams {
    /// Pattern to search for (case-insensitive substring).
    pattern: String,
    /// Optional glob to filter files (e.g. "pages/*.tsx", "**/*.tsx"). Omit to search all files.
    #[schemars(with = "String")]
    glob: Option<String>,
    /// Number of context lines to include before and after each match. Default 0 (match line only).
    #[schemars(with = "u32")]
    context: Option<u32>,
    /// Optional cap on returned matches.
    #[schemars(with = "u32")]
    head_limit: Option<u32>,
    /// Output mode: "content" (default) or "files_with_matches".
    #[schemars(with = "String")]
    output_mode: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineSearchParams {
    /// Pattern to search for (case-insensitive substring).
    pattern: String,
    /// Optional glob to filter pipeline files (e.g. "api/*.zf.json"). Omit to search all .zf.json files.
    #[schemars(with = "String")]
    glob: Option<String>,
    /// Number of context lines to include before and after each match. Default 0 (match line only).
    #[schemars(with = "u32")]
    context: Option<u32>,
    /// Optional cap on returned matches.
    #[schemars(with = "u32")]
    head_limit: Option<u32>,
    /// Output mode: "content" (default) or "files_with_matches".
    #[schemars(with = "String")]
    output_mode: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct FileEditParams {
    /// Relative path to the template file (e.g. "pages/home.tsx").
    rel_path: String,
    /// Exact string to find. Must match exactly once — provide enough context to be unique.
    old_string: String,
    /// Replacement string.
    new_string: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct FileOutlineParams {
    /// Relative path to the template file (e.g. "pages/home.tsx").
    rel_path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct FileDepsParams {
    /// Relative path to the template file (e.g. "pages/home.tsx").
    rel_path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct TemplateBatchEditItem {
    /// Relative path to the template file (e.g. "pages/home.tsx").
    rel_path: String,
    /// Exact string to find. Must match exactly once in the file.
    old_string: String,
    /// Replacement string.
    new_string: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct FileBatchEditParams {
    /// Edits to apply. Fails fast on the first edit error.
    edits: Vec<TemplateBatchEditItem>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct FileCreateParams {
    /// Kind of entry to create: "page", "component", "script", or "folder".
    kind: String,
    /// Base name for the file or folder (e.g. "blog-home", "user-card").
    name: String,
    /// Optional parent folder path under templates/ (e.g. "components/ui").
    #[schemars(with = "String")]
    parent_rel_path: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct DocsAgentReadParams {
    /// Agent doc name: "AGENTS.md", "SOUL.md", or "MEMORY.md".
    name: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct DocsAgentWriteParams {
    /// Agent doc name: "AGENTS.md" (project instructions), "SOUL.md" (agent personality),
    /// or "MEMORY.md" (persistent memory across sessions).
    name: String,
    /// Full file content to write.
    content: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineRegisterParams {
    /// Source-relative path of the pipeline (e.g. "api/blog-home").
    /// The .zf.json extension is added automatically if omitted.
    /// Preferred over name+path. If omitted, derived from name and path fields.
    #[serde(default)]
    #[schemars(with = "String")]
    file_rel_path: Option<String>,
    /// Pipeline name slug (e.g. "blog-home", "process-order"). Used when file_rel_path is not set.
    #[serde(default)]
    #[schemars(with = "String")]
    name: Option<String>,
    /// Virtual path for grouping (e.g. "/pages", "/api", "/jobs"). Defaults to "/". Used when file_rel_path is not set.
    #[serde(default)]
    #[schemars(with = "String")]
    path: Option<String>,
    /// Optional human-readable display title.
    #[schemars(with = "String")]
    title: Option<String>,
    /// Optional human-readable description of what this pipeline does.
    /// Stored in the pipeline graph (.zf.json) and indexed in the catalog for search.
    #[serde(default)]
    #[schemars(with = "String")]
    description: Option<String>,
    /// Pipeline body: pipe-chained nodes starting with |.
    /// Example: "| trigger.webhook --path /blog --method GET | pg.query --credential main-db -- \"SELECT * FROM posts\""
    /// Use help("pipeline/dsl") for the full node catalog and syntax.
    body: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineDescribeParams {
    /// Source-relative path of the pipeline (e.g. "api/blog-home.zf.json").
    /// Also accepted as "name" for backward compatibility.
    #[serde(alias = "name")]
    file_rel_path: String,
    /// When true, shows one compact line per node (id | kind | key flags) without body content.
    /// Use for orientation when pipelines have long SQL or script bodies.
    #[serde(default)]
    compact: bool,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelinePatchParams {
    /// Source-relative path of the pipeline (e.g. "api/blog-home.zf.json").
    /// Also accepted as "name" for backward compatibility.
    #[serde(alias = "name")]
    file_rel_path: String,
    /// Node ID to patch — get IDs from pipeline_describe output (e.g. "n0", "b", "trigger").
    node_id: String,
    /// Space-separated --flag value pairs to update in the node config.
    /// Example: "--credential new-db --path /updated"
    #[schemars(with = "String")]
    flags: Option<String>,
    /// Body content for the node (SQL for pg.query, JS source for script nodes).
    #[schemars(with = "String")]
    body: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineActivateParams {
    /// Source-relative path of the pipeline to activate (e.g. "api/blog-home.zf.json").
    /// Also accepted as "name" for backward compatibility.
    /// Ignored when glob is set.
    #[serde(alias = "name", default)]
    file_rel_path: String,
    /// Glob pattern to bulk-activate matching pipelines (e.g. "modules/manage/**").
    /// When set, activates all pipelines whose file_rel_path matches. file_rel_path is ignored.
    #[serde(default)]
    #[schemars(with = "String")]
    glob: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineDeactivateParams {
    /// Source-relative path of the pipeline to deactivate (e.g. "api/blog-home.zf.json").
    /// Also accepted as "name" for backward compatibility.
    #[serde(alias = "name")]
    file_rel_path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineExecuteParams {
    /// Source-relative path of the registered active pipeline to execute (e.g. "api/blog-home.zf.json").
    /// Also accepted as "name" for backward compatibility.
    #[serde(alias = "name")]
    file_rel_path: String,
    /// Optional JSON input payload string (e.g. "{\"order_id\": 42}").
    #[serde(default)]
    input: Option<serde_json::Value>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineRunParams {
    /// Pipe-chained node body to execute inline — NOT saved, NOT logged.
    /// Starts with | followed by nodes: "| pg.query --credential main-db -- \"SELECT count(*) FROM users\""
    /// Auto-prepends trigger.manual if no trigger node is specified.
    /// Use this for testing queries, one-off scripts, or data exploration.
    body: String,
    /// Optional JSON input payload (object or JSON string).
    /// Passed as the initial pipeline payload to trigger nodes.
    /// Example: {"message": "hello"} or "{\"message\": \"hello\"}"
    #[serde(default)]
    input: Option<serde_json::Value>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineGetInvocationsParams {
    /// Source-relative path of the pipeline (e.g. "api/blog-home.zf.json").
    file_rel_path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct GitCommandParams {
    /// Git subcommand: status, log, diff, add, commit
    subcommand: String,
    /// Additional arguments as a space-separated string (e.g. "path/to/file" for add/diff, "--limit 10" for log).
    #[schemars(with = "String")]
    args: Option<String>,
    /// Commit message — only used when subcommand is "commit".
    #[schemars(with = "String")]
    message: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct SkillReadParams {
    /// Skill name as listed by skill_list (e.g. "zebflow-pipeline").
    name: String,
    /// Optional file inside the skill folder (e.g. "references/verify.md"). Omit for SKILL.md.
    #[schemars(with = "String")]
    path: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct ConnectionDescribeParams {
    /// Connection slug — get slugs from connection_list (e.g. "main-db", "default").
    slug: String,
    /// Scope to inspect: "tables", "schemas", "functions", or omit for full tree.
    #[schemars(with = "String")]
    scope: Option<String>,
    /// Filter to a specific schema name (e.g. "public"). Only meaningful with scope="tables".
    #[schemars(with = "String")]
    schema: Option<String>,
    /// Filter to a specific table for column-level detail.
    /// Format: "schema.table" (e.g. "academic.staff") or just "table" for public schema.
    /// Use scope="tables" first to discover table names, then table=<name> for columns.
    #[schemars(with = "String")]
    table: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct HelpSearchParams {
    /// Search query — node name, DSL flag, concept, or keyword.
    query: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct RouteFetchParams {
    /// The route under this project, e.g. "/book" or "/api/slots?date=2026-09-14". Not a full URL.
    path: String,
    /// GET (default), POST, PUT, PATCH, DELETE.
    #[serde(default)]
    method: Option<String>,
    /// JSON body (an object) or raw text (a string). Ignored when `form` is given.
    #[serde(default)]
    body: Option<serde_json::Value>,
    /// Fields to post as application/x-www-form-urlencoded, like a browser <form>.
    #[serde(default)]
    form: Option<serde_json::Map<String, serde_json::Value>>,
    /// Extra request headers.
    #[serde(default)]
    headers: Option<serde_json::Map<String, serde_json::Value>>,
    /// Cookie header value, e.g. "zebflow_session=eyJ…" — from a login response's set_cookie.
    #[serde(default)]
    cookie: Option<String>,
    /// Follow up to 5 redirects. Default false, so a 302/303 and its location are visible.
    #[serde(default)]
    #[schemars(with = "bool")]
    follow_redirects: Option<bool>,
    /// Cap on returned body characters (200–60000, default 6000).
    #[serde(default)]
    #[schemars(with = "u32")]
    max_body_chars: Option<usize>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct HubSearchParams {
    /// A word to match in the package id, title, description or tags. Empty lists everything.
    #[serde(default)]
    query: Option<String>,
    /// Restrict to one asset kind: skill, rwe_library, pipeline, template_pack, folder_bundle, project_bundle, node_bundle.
    #[serde(default)]
    kind: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct HubPackageParams {
    /// The package id from `hub_search`, e.g. `zebflow.skill-procedural-assets`.
    package_id: String,
    /// A release version from `hub_search`; omitted, the latest live release.
    #[serde(default)]
    version: Option<String>,
    /// Where a bundle lands, relative to the source root. Ignored for skills and libraries, which have fixed homes.
    #[serde(default)]
    target_folder: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct InstallUiComponentsParams {
    /// Component names to install, e.g. ["button", "card", "dialog"].
    names: Vec<String>,
    /// If true, overwrite existing files. Default: false.
    #[serde(default)]
    #[schemars(with = "bool")]
    overwrite: Option<bool>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct MoveParams {
    /// Source path to move from.
    /// Pipelines: file_rel_path e.g. "api/old-name.zf.json" or just "old-name".
    /// Templates: rel_path e.g. "pages/old-name.tsx" or "components/old-card.tsx".
    from_path: String,
    /// Destination path to move to. Same domain as from_path.
    /// Parent folders are created automatically.
    to_path: String,
}

// ── Handler ───────────────────────────────────────────────────────────────────

/// Zebflow MCP handler with project-scoped tools.
///
/// Sessions are injected via HTTP request extensions by the middleware layer.
/// Tools access the session via `Extension<http::request::Parts>` and extract
/// the `McpSession` from `parts.extensions`.
#[derive(Clone)]
pub struct ZebflowMcpHandler {
    platform: Arc<PlatformService>,
    template_cache: crate::pipeline::engines::basic::TemplateCache,
    tool_router: rmcp::handler::server::tool::ToolRouter<Self>,
}

#[tool_router]
impl ZebflowMcpHandler {
    pub fn new(
        platform: Arc<PlatformService>,
        template_cache: crate::pipeline::engines::basic::TemplateCache,
    ) -> Self {
        Self {
            platform,
            template_cache,
            tool_router: Self::tool_router(),
        }
    }

    // ── Orientation ──────────────────────────────────────────────────────────

    #[tool(
        description = "Call this first. Returns Zebflow platform overview, project name, \
        project docs list, AGENTS.md/MEMORY.md content, DB connections, template tree, \
        and the next MCP tools to call. Your orientation before building anything."
    )]
    async fn start_here(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.start_here().await;
        Ok(CallToolResult::success(vec![Content::text(result.text)]))
    }

    // ── Version ──────────────────────────────────────────────────────────────

    #[tool(description = "Returns the running platform version string. \
        Use this to verify the deployed binary matches the expected Docker image tag.")]
    async fn version(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(
            crate::version::APP_VERSION.to_string(),
        )]))
    }

    // ── Help / Knowledge ─────────────────────────────────────────────────────

    #[tool(description = "Hierarchical docs browser. No topic = full index. \
        Paths: 'pipeline' (DSL + web patterns), 'pipeline/dsl', 'pipeline/authoring', 'pipeline/web', \
        'pipeline/nodes' (live catalog), 'pipeline/nodes/{kind}' (one node), \
        'pipeline/examples' (index), 'pipeline/examples/{slug}' (full recipe), \
        'web' (TSX pages), 'web/hooks', 'web/tailwind', 'web/design-system', 'web/libraries', \
        'tool' (Tool.time/arr/stat/geo), 'db', \
        'platform', 'platform/agent', 'platform/api', 'platform/operations', 'platform/workflow'. \
        Call help() before writing pipelines or templates.")]
    async fn help(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<HelpParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "help")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.help(params.topic.as_deref().unwrap_or(""));
        if result.text.starts_with("Error:") {
            Err(McpError::invalid_params(result.text, None))
        } else {
            Ok(CallToolResult::success(vec![Content::text(result.text)]))
        }
    }

    #[tool(
        description = "Search Zebflow docs. Returns matching chunks from pipeline docs, \
        web template docs, node catalog, and all help files. Use for any concept, node name, DSL flag, \
        or syntax question. Example: query='jwt', query='sqlite query', query='web.response'."
    )]
    async fn help_search(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<HelpSearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "help_search")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.help_search(&params.query);
        Ok(CallToolResult::success(vec![Content::text(result.text)]))
    }

    // ── Skills ───────────────────────────────────────────────────────────────

    #[tool(
        description = "List the skills this project's agent can load — name and one-line trigger each. \
                       A skill is the procedure for one kind of task (building a page, a pipeline, auth, verification…). \
                       When a task matches a skill, read it with skill_read before acting. \
                       Blessed skills ship with the platform; a project's own skills/<name>/SKILL.md shadows one by name."
    )]
    async fn skill_list(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "skill_list")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.skill_list();
        Ok(CallToolResult::success(vec![Content::text(result.text)]))
    }

    #[tool(
        description = "Read a skill's SKILL.md (the procedure, gates and checks for one kind of task), \
                       or with `path` a reference file beside it. Read a skill when its trigger matches \
                       the task at hand; do not read them all."
    )]
    async fn skill_read(
        &self,
        Parameters(params): Parameters<SkillReadParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "skill_read")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.skill_read(&params.name, params.path.as_deref());
        if result.text.starts_with("Error: ") {
            return Err(McpError::invalid_params(result.text, None));
        }
        Ok(CallToolResult::success(vec![Content::text(result.text)]))
    }

    // ── Pipelines ────────────────────────────────────────────────────────────

    #[tool(
        description = "List pipelines as a lean semantic index. Default compact rows are: \
                       file_rel_path | trigger summary | status | description. \
                       Use format='json' for full legacy metadata."
    )]
    async fn pipeline_list(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineListParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_list")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.pipeline_list(crate::platform::services::ops::PipelineListOptions {
            query: params.query.as_deref(),
            glob: params.glob.as_deref(),
            status: params.status.as_deref(),
            trigger_kind: params.trigger_kind.as_deref(),
            limit: params.limit,
            format: params.format.as_deref(),
        });
        ok_or_err(result)
    }

    #[tool(
        description = "Search pipeline .zf.json files for a pattern. Returns file:line matches. \
                       Use glob to narrow scope (e.g. \"api/*.zf.json\"). \
                       Equivalent to Grep across pipelines — find which pipelines use a credential, path, or node kind."
    )]
    async fn pipeline_search(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineSearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_search")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        ok_or_err(ops.pipeline_search(
            &params.pattern,
            params.glob.as_deref(),
            params.context.unwrap_or(0) as usize,
            params.head_limit,
            params.output_mode.as_deref(),
        ))
    }

    #[tool(description = "Get a specific pipeline by file-relative path")]
    async fn pipeline_get(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineGetParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_get")?;
        if self.pipeline_locked(&session.owner, &session.project, &params.file_rel_path) {
            return Err(McpError::invalid_params(
                "This pipeline is locked by the project owner and cannot be accessed by agents. Ask the owner to unlock it.",
                None,
            ));
        }
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.pipeline_get(&params.file_rel_path, None);
        ok_or_err(result)
    }

    #[tool(
        description = "Register (create or update) a pipeline at file_rel_path from a DSL body. \
                       The body is the nodes only — pipe mode '| trigger.webhook --path /x | sekejap.query -- \"SQL\" | web.response --template pages/x.tsx' \
                       or graph mode '[a] trigger.webhook … [b] … [a] -> [b]' — with no leading 'register …' line (that is the console form). \
                       It is saved as a draft; call pipeline_activate to make it live. Re-registering a live pipeline makes it stale until activated. \
                       help(\"pipeline/dsl\") for the syntax, help(\"pipeline/nodes/<kind>\") for a node's flags."
    )]
    async fn pipeline_register(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineRegisterParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_register")?;
        // Check lock on existing pipeline (by resolved file_rel_path if provided)
        if let Some(ref frp) = params.file_rel_path {
            let frp = self
                .platform
                .projects
                .pipeline_identity(&session.owner, &session.project, frp)
                .map_err(|error| McpError::internal_error(error.to_string(), None))?;
            if self.pipeline_locked(&session.owner, &session.project, &frp) {
                return Err(McpError::invalid_params(
                    "This pipeline is locked by the project owner and cannot be accessed by agents. Ask the owner to unlock it.",
                    None,
                ));
            }
        }
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops
            .pipeline_register(
                &params.body,
                params.file_rel_path.as_deref(),
                params.name.as_deref(),
                params.path.as_deref(),
                params.title.as_deref(),
                params.description.as_deref(),
            )
            .await;
        // navigate is ignored for MCP
        Ok(CallToolResult::success(vec![Content::text(result.text)]))
    }

    #[tool(
        description = "Inspect a pipeline — returns its nodes, edges, status, and hit stats. \
                       Node IDs from this output are required for pipeline_patch. \
                       Set compact=true to show one line per node (id | kind | key flags) \
                       without body content — useful for pipelines with long SQL or scripts."
    )]
    async fn pipeline_describe(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineDescribeParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_describe")?;
        if self.pipeline_locked(&session.owner, &session.project, &params.file_rel_path) {
            return Err(McpError::invalid_params(
                "This pipeline is locked by the project owner and cannot be accessed by agents. Ask the owner to unlock it.",
                None,
            ));
        }
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops
            .pipeline_describe(&params.file_rel_path, params.compact)
            .await;
        Ok(CallToolResult::success(vec![Content::text(result.text)]))
    }

    #[tool(
        description = "Patch one node in a saved pipeline without rewriting the full graph. \
                       node_id accepts: opaque ID (e.g. 'n0'), node kind (e.g. 'trigger.webhook', 'pg.query'), \
                       or kind+index (e.g. 'pg.query[1]') when multiple nodes share the same kind. \
                       Pipeline status becomes stale after patching — call pipeline_activate to make it live again."
    )]
    async fn pipeline_patch(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelinePatchParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_patch")?;
        if self.pipeline_locked(&session.owner, &session.project, &params.file_rel_path) {
            return Err(McpError::invalid_params(
                "This pipeline is locked by the project owner and cannot be accessed by agents. Ask the owner to unlock it.",
                None,
            ));
        }
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops
            .pipeline_patch(
                &params.file_rel_path,
                &params.node_id,
                params.flags.as_deref(),
                params.body.as_deref(),
            )
            .await;
        Ok(CallToolResult::success(vec![Content::text(result.text)]))
    }

    #[tool(
        description = "Activate a pipeline — makes it live so it can serve traffic and be executed. \
                       Must be called after pipeline_register or after patching. \
                       A pipeline must be active before pipeline_execute will run it. \
                       Set glob (e.g. \"modules/manage/**\") to bulk-activate all matching pipelines \
                       instead of activating one at a time."
    )]
    async fn pipeline_activate(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineActivateParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_activate")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);

        // Bulk mode: glob provided → activate all matching pipelines (no lock check per pipeline).
        if let Some(ref glob) = params.glob {
            let result = ops.pipeline_activate_glob(glob).await;
            return Ok(CallToolResult::success(vec![Content::text(result.text)]));
        }

        // Single mode: check lock and activate exact path.
        if self.pipeline_locked(&session.owner, &session.project, &params.file_rel_path) {
            return Err(McpError::invalid_params(
                "This pipeline is locked by the project owner and cannot be accessed by agents. Ask the owner to unlock it.",
                None,
            ));
        }
        let result = ops.pipeline_activate(&params.file_rel_path).await;
        // navigate is ignored for MCP
        Ok(CallToolResult::success(vec![Content::text(result.text)]))
    }

    #[tool(
        description = "Deactivate a pipeline — takes it offline. Traffic stops being served. \
                       Pipeline source is retained and can be re-activated later."
    )]
    async fn pipeline_deactivate(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineDeactivateParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_deactivate")?;
        if self.pipeline_locked(&session.owner, &session.project, &params.file_rel_path) {
            return Err(McpError::invalid_params(
                "This pipeline is locked by the project owner and cannot be accessed by agents. Ask the owner to unlock it.",
                None,
            ));
        }
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.pipeline_deactivate(&params.file_rel_path).await;
        Ok(CallToolResult::success(vec![Content::text(result.text)]))
    }

    #[tool(
        description = "Execute a registered active pipeline by name. Records execution hits. \
                       Pipeline must be activated first — use pipeline_activate if status is draft or stale. \
                       Use pipeline_list to see pipeline names and activation status. \
                       For function pipelines (n.trigger.function) always pass `input` to test with real data; \
                       without it the pipeline receives an empty payload {}."
    )]
    async fn pipeline_execute(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineExecuteParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_execute")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        // Normalize input: accept JSON object/array or string, serialize to JSON string for DSL.
        let input_str = params.input.as_ref().and_then(|v| match v {
            serde_json::Value::String(s) if !s.is_empty() => Some(s.clone()),
            serde_json::Value::Null => None,
            other => serde_json::to_string(other).ok(),
        });
        let result = ops
            .pipeline_execute(&params.file_rel_path, input_str.as_deref())
            .await;
        Ok(CallToolResult::success(vec![Content::text(result.text)]))
    }

    #[tool(
        description = "Run a pipe-chained node body EPHEMERALLY — not saved, not logged, no hit recording. \
                       Use this to test queries, explore data, or prototype before registering. \
                       Example body: '| pg.query --credential main-db -- \"SELECT count(*) FROM users\"'. \
                       Auto-prepends trigger.manual if no trigger node specified."
    )]
    async fn pipeline_run(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineRunParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_run")?;
        let initial_input = params.input.and_then(|v| match v {
            serde_json::Value::Object(_) | serde_json::Value::Array(_) => Some(v),
            serde_json::Value::String(s) if !s.is_empty() => serde_json::from_str(&s).ok(),
            _ => None,
        });
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.pipeline_run(&params.body, initial_input).await;
        Ok(CallToolResult::success(vec![Content::text(result.text)]))
    }

    #[tool(
        description = "Get recent execution history for a pipeline. Returns stored invocations \
                           with timestamp, duration, status (ok/error), trigger source, error message, \
                           and per-node trace. Use this to inspect past runs, debug failures on scheduled \
                           pipelines, or verify that a pipeline is executing correctly."
    )]
    async fn pipeline_get_invocations(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineGetInvocationsParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_get_invocations")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        ok_or_err(ops.pipeline_get_invocations(&params.file_rel_path))
    }

    // ── Templates ────────────────────────────────────────────────────────────

    #[tool(description = "List templates in the project workspace. Optional glob filters files.")]
    async fn file_list(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<FileListParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "file_list")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.file_list(crate::platform::services::ops::FileListOptions {
            query: params.query.as_deref(),
            glob: params.glob.as_deref(),
            kind: params.kind.as_deref(),
            limit: params.limit,
            format: params.format.as_deref(),
        });
        ok_or_err(result)
    }

    #[tool(description = "Get a specific template by relative path. \
                       Use offset and limit to read a line-numbered slice instead of the full file.")]
    async fn file_read(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<FileReadParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "file_read")?;
        if self.template_locked(&session.owner, &session.project, &params.rel_path) {
            return Err(McpError::invalid_params(
                "This template is locked by the project owner and cannot be accessed by agents. Ask the owner to unlock it.",
                None,
            ));
        }
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.file_read(&params.rel_path, params.offset, params.limit);
        ok_or_err(result)
    }

    #[tool(description = "Create a new source file with scaffolding. \
                       Kind must be one of: page (.tsx), component (.tsx), script (.ts), style (.css), doc (.md), folder. \
                       The file lands at parent_rel_path/name.<ext> relative to the source root — pass \
                       parent_rel_path=\"pages\" to get pages/<name>.tsx; without it the file is created at the root. \
                       Returns the scaffolded content — use file_write to customise it after.")]
    async fn file_create(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<FileCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "file_create")?;
        // Check lock on parent folder if provided
        if let Some(ref parent) = params.parent_rel_path {
            if self.template_locked(&session.owner, &session.project, parent) {
                return Err(McpError::invalid_params(
                    "This template folder is locked by the project owner and cannot be accessed by agents. Ask the owner to unlock it.",
                    None,
                ));
            }
        }
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.file_create(
            &params.kind,
            &params.name,
            params.parent_rel_path.as_deref(),
        );
        // navigate is ignored for MCP
        ok_or_err(result)
    }

    #[tool(description = "Write (create or overwrite) a template file. \
                       Use file_create first to scaffold with boilerplate, then file_write to fill in content. \
                       Path is relative to templates/ (e.g. 'pages/blog-home.tsx', 'components/ui/card.tsx', 'scripts/format-address.ts'). \
                       Use help(\"web\") for TSX conventions or help(\"web/custom-scripts\") for TypeScript module rules before writing.")]
    async fn file_write(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<FileWriteParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "file_write")?;
        if self.template_locked(&session.owner, &session.project, &params.rel_path) {
            return Err(McpError::invalid_params(
                "This template is locked by the project owner and cannot be accessed by agents. Ask the owner to unlock it.",
                None,
            ));
        }
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.file_write(&params.rel_path, &params.content);
        if !result.text.starts_with("Error:") {
            if let Ok(abs) = self.platform.projects.resolve_template_abs_path(
                &session.owner,
                &session.project,
                &params.rel_path,
            ) {
                crate::pipeline::engines::basic::evict_template_cache_by_path(
                    &self.template_cache,
                    &abs.to_string_lossy(),
                );
            }
        }
        // navigate is ignored for MCP
        ok_or_err(result)
    }

    #[tool(
        description = "Search template files for a pattern. Returns file:line matches. \
                       Use glob to narrow scope (e.g. \"pages/*.tsx\", \"**/*.tsx\"). \
                       Equivalent to Grep across templates — find which files use an import, component, or value."
    )]
    async fn file_search(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<FileSearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "file_search")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        ok_or_err(ops.file_search(
            &params.pattern,
            params.glob.as_deref(),
            params.context.unwrap_or(0) as usize,
            params.head_limit,
            params.output_mode.as_deref(),
        ))
    }

    #[tool(description = "Surgical string replacement in a template file. \
                       Equivalent to Edit — no need to read the full file first. \
                       Fails if old_string is not found or matches more than once (provide more context). \
                       Returns the line number of the replacement.")]
    async fn file_edit(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<FileEditParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "file_edit")?;
        if self.template_locked(&session.owner, &session.project, &params.rel_path) {
            return Err(McpError::invalid_params(
                "This template is locked by the project owner and cannot be accessed by agents. Ask the owner to unlock it.",
                None,
            ));
        }
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.file_edit(&params.rel_path, &params.old_string, &params.new_string);
        if !result.text.starts_with("Error:") {
            if let Ok(abs) = self.platform.projects.resolve_template_abs_path(
                &session.owner,
                &session.project,
                &params.rel_path,
            ) {
                crate::pipeline::engines::basic::evict_template_cache_by_path(
                    &self.template_cache,
                    &abs.to_string_lossy(),
                );
            }
        }
        ok_or_err(result)
    }

    #[tool(
        description = "Parse a template file and return its code outline: imports, exports, \
                       functions, classes, types, interfaces, and line numbers. \
                       Use this before file_read when orienting on a large TSX/TS file."
    )]
    async fn file_outline(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<FileOutlineParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "file_outline")?;
        if self.template_locked(&session.owner, &session.project, &params.rel_path) {
            return Err(McpError::invalid_params(
                "This template is locked by the project owner and cannot be accessed by agents. Ask the owner to unlock it.",
                None,
            ));
        }
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.file_outline(&params.rel_path);
        ok_or_err(result)
    }

    #[tool(
        description = "Show a template dependency graph: imports used by this file and \
                       other project templates that import it. Use before refactoring shared UI."
    )]
    async fn file_deps(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<FileDepsParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "file_deps")?;
        if self.template_locked(&session.owner, &session.project, &params.rel_path) {
            return Err(McpError::invalid_params(
                "This template is locked by the project owner and cannot be accessed by agents. Ask the owner to unlock it.",
                None,
            ));
        }
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.file_deps(&params.rel_path);
        ok_or_err(result)
    }

    #[tool(
        description = "Apply multiple exact string edits across template files in one call. \
                       Each edit is rel_path + old_string + new_string. \
                       Evicts template cache for edited files when the batch succeeds."
    )]
    async fn file_batch_edit(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<FileBatchEditParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "file_batch_edit")?;
        for edit in &params.edits {
            if self.template_locked(&session.owner, &session.project, &edit.rel_path) {
                return Err(McpError::invalid_params(
                    format!("Template '{}' is locked", edit.rel_path),
                    None,
                ));
            }
        }
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let edits: Vec<(String, String, String)> = params
            .edits
            .iter()
            .map(|edit| {
                (
                    edit.rel_path.clone(),
                    edit.old_string.clone(),
                    edit.new_string.clone(),
                )
            })
            .collect();
        let result = ops.file_batch_edit(&edits);
        if result.text.contains("ERROR:") {
            return Err(McpError::invalid_params(result.text, None));
        }
        if !result.text.starts_with("Error:") {
            for edit in &params.edits {
                if let Ok(abs) = self.platform.projects.resolve_template_abs_path(
                    &session.owner,
                    &session.project,
                    &edit.rel_path,
                ) {
                    crate::pipeline::engines::basic::evict_template_cache_by_path(
                        &self.template_cache,
                        &abs.to_string_lossy(),
                    );
                }
            }
        }
        ok_or_err(result)
    }

    // ── Agent Docs ───────────────────────────────────────────────────────────

    #[tool(
        description = "List the three agent doc files: AGENTS.md (project instructions for agents), \
                       SOUL.md (agent personality and behavior config), MEMORY.md (persistent memory across sessions). \
                       Always read AGENTS.md first when starting work on a project."
    )]
    async fn docs_agent_list(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "docs_agent_list")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.docs_agent_list();
        ok_or_err(result)
    }

    #[tool(description = "Read one agent doc: AGENTS.md (project instructions), \
                       SOUL.md (agent personality), or MEMORY.md (persistent memory). \
                       Read AGENTS.md at the start of every session to understand the project.")]
    async fn docs_agent_read(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<DocsAgentReadParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "docs_agent_read")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.docs_agent_read(&params.name);
        ok_or_err(result)
    }

    #[tool(
        description = "Write an agent doc. AGENTS.md: project-specific instructions for all agents. \
                       SOUL.md: agent personality, tone, and behavioral config. \
                       MEMORY.md: persistent notes the agent writes to remember things across sessions. \
                       Agents should update MEMORY.md after completing significant work."
    )]
    async fn docs_agent_write(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<DocsAgentWriteParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "docs_agent_write")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.docs_agent_write(&params.name, &params.content);
        ok_or_err(result)
    }

    // ── Connections & Credentials ─────────────────────────────────────────────

    #[tool(
        description = "List all DB connections for this project — returns slug, label, and kind (postgresql, sekejap, sqlite, ...). Every project has `default` (sqlite) and `default-multimodel` (sekejap). Use the slug with connection_describe. (`--credential` flags take a credential id from credential_list, not a connection slug.)"
    )]
    async fn connection_list(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "connection_list")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.connection_list();
        ok_or_err(result)
    }

    #[tool(
        description = "Describe a DB connection's schema — tables, columns, types, constraints. Use scope='tables' for a quick overview, scope='schemas' to list schemas, or omit scope for the full tree. Always run this before writing SQL queries. Use table='schema.table' (e.g. table='academic.staff') to get column detail for a specific table."
    )]
    async fn connection_describe(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<ConnectionDescribeParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "connection_describe")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops
            .connection_describe(
                &params.slug,
                params.scope.as_deref(),
                params.schema.as_deref(),
                params.table.as_deref(),
            )
            .await;
        ok_or_err(result)
    }

    #[tool(
        description = "List credentials for this project — returns id, title, and kind only. Values are never exposed. Use the id in pipeline nodes that require authentication."
    )]
    async fn credential_list(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "credential_list")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.credential_list();
        ok_or_err(result)
    }

    // ── Git ──────────────────────────────────────────────────────────────────

    #[tool(description = "Run a git command on the project repository. \
                       Allowed subcommands: status, log, diff, add, commit. \
                       Destructive operations (reset, rebase, force, checkout) are blocked. \
                       Always commit after registering or patching pipelines.")]
    async fn git_command(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<GitCommandParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "git_command")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops
            .git_command(
                &params.subcommand,
                params.args.as_deref(),
                params.message.as_deref(),
            )
            .await;
        Ok(CallToolResult::success(vec![Content::text(result.text)]))
    }

    // ── UI Catalog ───────────────────────────────────────────────────────────

    #[tool(
        description = "List all available shadcn-compatible Zeb React UI components \
        that can be installed into shared/ui/. Returns name, category, description, \
        filename, and whether each component is already installed in this project."
    )]
    async fn list_ui_catalog(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "list_ui_catalog")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.list_ui_catalog();
        ok_or_err(result)
    }

    #[tool(
        description = "Install shadcn-compatible UI components into shared/ui/. \
        Pass names like [\"button\",\"card\",\"dialog\"]. \
        Set overwrite=true to replace existing files. \
        Returns installed and skipped lists."
    )]
    async fn install_ui_components(
        &self,
        Parameters(params): Parameters<InstallUiComponentsParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "install_ui_components")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.install_ui_components(params.names, params.overwrite);
        ok_or_err(result)
    }

    // ── Route fetch ───────────────────────────────────────────────────────────

    #[tool(
        description = "Fetch one of this project's own routes the way a browser would — the verification step. \
        `path` is the route under the project (`/`, `/book`, `/api/slots?date=2026-09-14`); the tool prepends \
        `/wh/{owner}/{project}` and goes through the real ingress, so auth, cookies, redirects and page rendering \
        happen. Returns status, content_type, location, set_cookie, length, rwe_component_errors (every \
        `<!-- RWE component error -->` in the body — a 200 with one of these is a broken page) and the body \
        (capped, `max_body_chars`). `method` GET|POST|PUT|DELETE; `form` posts url-encoded fields like a <form>; \
        `body` sends JSON (an object) or raw text (a string); `cookie` is a Cookie header value — copy it from a \
        login's set_cookie (`name=value`) to reach protected routes; `follow_redirects` is off by default so you \
        see the 302/303 and its location. Fetch every route you built, and every failure path, before saying done."
    )]
    async fn route_fetch(
        &self,
        Parameters(params): Parameters<RouteFetchParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "route_fetch")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        ok_or_err(
            ops.route_fetch(
                params.path,
                params.method,
                params.body,
                params.form,
                params.headers,
                params.cookie,
                params.follow_redirects,
                params.max_body_chars,
            )
            .await,
        )
    }

    // ── Hub ───────────────────────────────────────────────────────────────────

    #[tool(
        description = "List what this project can add from the Hub shelf — optional skills, zeb/* libraries, \
        pipeline and template bundles, node bundles. Filter with `query` (a word in the id, title, description \
        or tags) and `kind` (skill, rwe_library, pipeline, template_pack, folder_bundle, project_bundle, node_bundle). \
        Each row carries `package_id` and `latest_version`, which `hub_review` and `hub_add` take. \
        The core zebflow-* skills are not here: every project already has them (`skill_list`)."
    )]
    async fn hub_search(
        &self,
        Parameters(params): Parameters<HubSearchParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "hub_search")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        ok_or_err(ops.hub_search(params.query, params.kind))
    }

    #[tool(
        description = "Review a Hub package before adding it: the files it would add or overwrite, pipelines it \
        registers, nodes it uses, credentials it needs, outbound hosts, public endpoints, schedules, and any \
        violation that makes it uninstallable. `version` defaults to the latest live release. Read the review, \
        then `hub_add` — never the other way round."
    )]
    async fn hub_review(
        &self,
        Parameters(params): Parameters<HubPackageParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "hub_review")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        ok_or_err(ops.hub_review(params.package_id, params.version, params.target_folder))
    }

    #[tool(
        description = "Add a Hub package to this project (the Studio's Add button). A skill lands at \
        `skills/<name>/` and appears in `skill_list`; a zeb/* library under `shared/`; a pipeline or template \
        bundle under `target_folder` (default: the package's own folder). Pipelines arrive as drafts — \
        `pipeline_activate` them. `version` defaults to the latest live release. Returns files written and \
        pipelines registered; the dependency is recorded in zeb.lock."
    )]
    async fn hub_add(
        &self,
        Parameters(params): Parameters<HubPackageParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "hub_add")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        ok_or_err(ops.hub_add(params.package_id, params.version, params.target_folder))
    }

    // ── Move Resource ─────────────────────────────────────────────────────────

    #[tool(description = "Rename or reorganize a pipeline or template file. \
            Domain detected automatically from path: .zf.json = pipeline, anything else = template. \
            For pipelines: deactivate → move → re-activate lifecycle handled automatically. \
            Parent folders created automatically. No cross-domain moves (pipeline ↔ template).")]
    async fn move_resource(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<MoveParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "move_resource")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.move_resource(&params.from_path, &params.to_path).await;
        ok_or_err(result)
    }

    // ── Lock helpers ──────────────────────────────────────────────────────────

    /// Returns true if the pipeline at `file_rel_path` is locked.
    fn pipeline_locked(&self, owner: &str, project: &str, file_rel_path: &str) -> bool {
        self.platform
            .projects
            .read_pipeline_source(owner, project, file_rel_path)
            .map(|source| {
                let Ok(value) = serde_json::from_str::<serde_json::Value>(&source) else {
                    return false;
                };
                value
                    .get("metadata")
                    .and_then(|m| m.get("locked"))
                    .and_then(serde_json::Value::as_bool)
                    .or_else(|| value.get("locked").and_then(serde_json::Value::as_bool))
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    /// Returns true if the template at `rel_path` is locked.
    fn template_locked(&self, owner: &str, project: &str, rel_path: &str) -> bool {
        self.platform
            .zebflow_cfg
            .is_template_locked(owner, project, rel_path)
            .unwrap_or(false)
    }

    // ── Auth helpers ──────────────────────────────────────────────────────────

    fn get_session_from_http_parts(
        &self,
        parts: &http::request::Parts,
    ) -> Result<McpSession, McpError> {
        parts
            .extensions
            .get::<McpSession>()
            .cloned()
            .ok_or_else(|| {
                McpError::invalid_params(
                    "No active MCP session; ensure Authorization header is set with valid session token",
                    None,
                )
            })
    }

    // ── MCP trigger helpers ────────────────────────────────────────────────

    /// Extract session from RequestContext extensions (used by manual ServerHandler impl).
    fn session_from_context(&self, context: &RequestContext<RoleServer>) -> Option<McpSession> {
        context
            .extensions
            .get::<http::request::Parts>()
            .and_then(|parts| parts.extensions.get::<McpSession>().cloned())
    }

    /// Convert an McpTriggerSpec into an rmcp Tool for tools/list.
    fn mcp_trigger_spec_to_tool(
        &self,
        spec: &crate::platform::services::pipeline_runtime::McpTriggerSpec,
    ) -> Tool {
        let input_schema = spec.input_schema.as_object().cloned().unwrap_or_default();
        Tool {
            name: spec.tool_name.clone().into(),
            title: None,
            description: if spec.tool_description.is_empty() {
                None
            } else {
                Some(spec.tool_description.clone().into())
            },
            input_schema: std::sync::Arc::new(input_schema),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        }
    }

    /// Dispatch an MCP tool call to a pipeline with a matching mcp_trigger.
    async fn mcp_trigger_dispatch(
        &self,
        tool_name: &str,
        arguments: Option<serde_json::Map<String, serde_json::Value>>,
        context: &RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.session_from_context(context).ok_or_else(|| {
            McpError::invalid_params("No active MCP session for dynamic tool dispatch", None)
        })?;

        // Find the pipeline containing this tool.
        let pipelines = self
            .platform
            .pipeline_runtime
            .list_project(&session.owner, &session.project);

        let (compiled, _spec) = pipelines
            .iter()
            .find_map(|p| {
                p.mcp_triggers
                    .iter()
                    .find(|s| s.tool_name == tool_name)
                    .map(|s| (p, s))
            })
            .ok_or_else(|| McpError::invalid_params(format!("Unknown tool '{tool_name}'"), None))?;

        // Build the pipeline engine (same pattern as webhook/manual dispatch).
        let credentials = self.platform.credentials.clone();
        let engine = crate::pipeline::engines::basic::BasicPipelineEngine::new(
            std::sync::Arc::new(
                self.platform
                    .project_sandbox(&session.owner, &session.project),
            ),
            crate::rwe::resolve_engine_or_default(None),
            Some(credentials),
        )
        .with_platform(self.platform.clone())
        .with_template_cache(self.template_cache.clone())
        .with_project_layout(
            self.platform
                .projects
                .project_layout(&session.owner, &session.project)
                .ok(),
        )
        .with_ws_hub(self.platform.ws_hub.clone())
        .with_state_bus(self.platform.state_bus.clone())
        .with_data_root(self.platform.config.data_root.clone());

        let request_id = format!("mcp-{}", uuid::Uuid::new_v4());
        let input_payload = serde_json::json!({
            "tool_name": tool_name,
            "arguments": arguments.unwrap_or_default(),
        });

        let ctx = crate::pipeline::model::PipelineContext {
            owner: session.owner.clone(),
            project: session.project.clone(),
            pipeline: compiled.graph.id.clone(),
            request_id: request_id.clone(),
            route: Default::default(),
            input: input_payload.clone(),
            trigger: Some(serde_json::json!({
                "kind": "mcp",
                "source": format!("tools/call:{tool_name}"),
            })),
            placeholder: None,
        };

        let exec_start = std::time::Instant::now();

        match crate::pipeline::interface::PipelineEngine::execute_async(
            &engine,
            &compiled.graph,
            &ctx,
        )
        .await
        {
            Ok(output) => {
                self.platform.pipeline_hits.record_success(
                    &session.owner,
                    &session.project,
                    &compiled.file_rel_path,
                );
                let _ = self.platform.data.log_pipeline_invocation(
                    &session.owner,
                    &session.project,
                    &compiled.file_rel_path,
                    &crate::platform::model::PipelineInvocationEntry {
                        run_id: request_id,
                        at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64,
                        duration_ms: exec_start.elapsed().as_millis() as u64,
                        status: "ok".to_string(),
                        trigger: format!("mcp:{tool_name}"),
                        error: None,
                        trace: output.node_trace.clone(),
                    },
                    50,
                    Some(86400),
                );

                let result_text = serde_json::to_string_pretty(&output.value)
                    .unwrap_or_else(|_| format!("{:?}", output.value));
                Ok(CallToolResult::success(vec![Content::text(result_text)]))
            }
            Err(err) => {
                self.platform.pipeline_hits.record_failure(
                    &session.owner,
                    &session.project,
                    &compiled.file_rel_path,
                    &format!("mcp:{tool_name}"),
                    err.code,
                    &err.message,
                );
                let _ = self.platform.data.log_pipeline_invocation(
                    &session.owner,
                    &session.project,
                    &compiled.file_rel_path,
                    &crate::platform::model::PipelineInvocationEntry {
                        run_id: request_id,
                        at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64,
                        duration_ms: exec_start.elapsed().as_millis() as u64,
                        status: "error".to_string(),
                        trigger: format!("mcp:{tool_name}"),
                        error: Some(err.message.clone()),
                        trace: err.node_trace.clone(),
                    },
                    50,
                    Some(86400),
                );
                Err(McpError::internal_error(
                    format!("Pipeline execution failed: {}", err.message),
                    None,
                ))
            }
        }
    }

    fn check_tool_capability(&self, session: &McpSession, tool_name: &str) -> Result<(), McpError> {
        let required_capability = mcp_tool_capability(tool_name)
            .ok_or_else(|| McpError::invalid_params(format!("Unknown tool '{tool_name}'"), None))?;

        if session.capabilities.contains(&required_capability) {
            return Ok(());
        }
        Err(McpError::invalid_params(
            format!(
                "Tool '{tool_name}' requires capability '{}' which is not allowed in this session",
                required_capability.key()
            ),
            None,
        ))
    }
}

impl ServerHandler for ZebflowMcpHandler {
    fn get_info(&self) -> ServerInfo {
        let instructions = crate::platform::help::get_help_content("platform/agent")
            .unwrap_or_else(|| {
                "Zebflow project management tools. Call start_here first. \
                 Use help(topic) for docs — no topic = full index. \
                 Use help_search(query) to search across all docs."
                    .to_string()
            });
        ServerInfo {
            instructions: Some(instructions.into()),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .build(),
            ..Default::default()
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let mut tools = self.tool_router.list_all();

        // Merge dynamic MCP trigger tools from active pipelines.
        if let Some(session) = self.session_from_context(&context) {
            let pipelines = self
                .platform
                .pipeline_runtime
                .list_project(&session.owner, &session.project);
            for compiled in &pipelines {
                for spec in &compiled.mcp_triggers {
                    tools.push(self.mcp_trigger_spec_to_tool(spec));
                }
            }
        }

        Ok(ListToolsResult {
            tools,
            meta: None,
            next_cursor: None,
        })
    }

    /// Every skill is also an MCP prompt, so a client with slash commands
    /// (Claude Code, Cursor, …) gets `/zebflow-page` for free. The body a
    /// prompt returns is exactly what `skill_read` returns; the listing is
    /// the same tier-1 name + description.
    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        let Some(session) = self.session_from_context(&context) else {
            return Ok(ListPromptsResult::default());
        };
        let Ok(layout) = self.platform.projects.project_layout(&session.owner, &session.project) else {
            return Ok(ListPromptsResult::default());
        };
        let prompts = crate::platform::skills::list(&layout.repo_source_dir())
            .into_iter()
            .map(|skill| Prompt::new(skill.name, Some(skill.description), None))
            .collect();
        Ok(ListPromptsResult::with_all_items(prompts))
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        let session = self
            .session_from_context(&context)
            .ok_or_else(|| McpError::invalid_request("no MCP session", None))?;
        let layout = self
            .platform
            .projects
            .project_layout(&session.owner, &session.project)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let Some((_, text)) = crate::platform::skills::read(&layout.repo_source_dir(), &request.name, "") else {
            return Err(McpError::invalid_params(format!("no skill '{}'", request.name), None));
        };
        let description = crate::platform::skills::parse_frontmatter(&text).description;
        Ok(GetPromptResult {
            description,
            messages: vec![PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::Text { text },
            }],
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        // Static tools — delegate to macro-generated tool router.
        if self.tool_router.has_route(&request.name) {
            let tcc = rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
            return self.tool_router.call(tcc).await;
        }

        // Dynamic MCP trigger tools — find matching pipeline and execute.
        self.mcp_trigger_dispatch(&request.name, request.arguments, &context)
            .await
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        // Check static router first.
        if let Some(tool) = self.tool_router.get(name) {
            return Some(tool.clone());
        }

        // Check dynamic MCP triggers — we don't have session context here,
        // so scan all active pipelines. This is fine for validation purposes.
        let all = self.platform.pipeline_runtime.list_all();
        for compiled in &all {
            for spec in &compiled.mcp_triggers {
                if spec.tool_name == name {
                    return Some(self.mcp_trigger_spec_to_tool(spec));
                }
            }
        }

        None
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Convert an `OpsResult` to `CallToolResult`, treating "Error: ..." text as an MCP error.
fn ok_or_err(
    result: crate::platform::services::ops::OpsResult,
) -> Result<CallToolResult, McpError> {
    if result.text.starts_with("Error:") {
        Err(McpError::internal_error(result.text, None))
    } else {
        Ok(CallToolResult::success(vec![Content::text(result.text)]))
    }
}

/// Build the MCP service with token validation using rmcp's StreamableHttpService.
pub fn build_mcp_service<S: Clone + Send + Sync + 'static>(
    platform: Arc<PlatformService>,
    template_cache: crate::pipeline::engines::basic::TemplateCache,
) -> axum::Router<S> {
    use axum::middleware;
    use rmcp::transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    };
    use tokio_util::sync::CancellationToken;

    let session_manager = Arc::new(LocalSessionManager::default());
    let cancellation_token = CancellationToken::new();

    let config = StreamableHttpServerConfig {
        sse_keep_alive: Some(std::time::Duration::from_secs(30)),
        sse_retry: Some(std::time::Duration::from_secs(5)),
        stateful_mode: false,
        json_response: true,
        cancellation_token: cancellation_token.clone(),
    };

    let platform_for_factory = platform.clone();
    let cache_for_factory = template_cache.clone();
    let service = StreamableHttpService::new(
        move || {
            let platform = platform_for_factory.clone();
            let cache = cache_for_factory.clone();
            let handler = ZebflowMcpHandler::new(platform, cache);
            Ok(handler)
        },
        session_manager,
        config,
    );

    let platform_for_middleware = platform.clone();
    axum::Router::new()
        .route_service("/", service)
        .layer(middleware::from_fn(
            move |mut req: axum::extract::Request, next: middleware::Next| {
                let platform = platform_for_middleware.clone();
                async move {
                    let Some((owner, project)) = mcp_project_scope_from_request(&req) else {
                        return StatusCode::BAD_REQUEST.into_response();
                    };

                    let session = match mcp_session_from_request(&platform, req.headers()) {
                        Some(session) => session,
                        None => return StatusCode::UNAUTHORIZED.into_response(),
                    };

                    if crate::platform::model::slug_segment(&session.owner)
                        != crate::platform::model::slug_segment(&owner)
                        || crate::platform::model::slug_segment(&session.project)
                            != crate::platform::model::slug_segment(&project)
                    {
                        return StatusCode::FORBIDDEN.into_response();
                    }

                    match mcp_remote_project_worker_id(&platform, &owner, &project) {
                        Ok(Some(worker_id)) => {
                            return match forward_mcp_request_to_worker(
                                &platform, req, &worker_id, &session,
                            )
                            .await
                            {
                                Ok(response) => response,
                                Err(response) => response,
                            };
                        }
                        Ok(None) => {}
                        Err(response) => return response,
                    }

                    req.extensions_mut().insert(session);

                    next.run(req).await
                }
            },
        ))
}

fn mcp_session_from_request(platform: &PlatformService, headers: &HeaderMap) -> Option<McpSession> {
    if let Some(token) = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(parse_bearer_token)
        && let Some(session) = platform.mcp_sessions.lookup(token)
    {
        return Some(session);
    }

    if !is_controller_call(platform, headers) {
        return None;
    }
    let encoded = headers
        .get(INTERNAL_MCP_SESSION_HEADER)
        .and_then(|h| h.to_str().ok())?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .ok()?;
    let session: McpSession = serde_json::from_slice(&bytes).ok()?;
    if session.enabled { Some(session) } else { None }
}

/// Whether this request is **this office's controller** proxying an MCP call.
///
/// The header below carries a whole session, so what authenticates it decides
/// who that session may be. It used to accept any active office's join token on
/// a controller, which let one office hand a controller any session it liked.
/// One direction only now: an office accepting its controller's proxied call.
fn is_controller_call(platform: &PlatformService, headers: &HeaderMap) -> bool {
    let Some(presented) = headers
        .get(INTERNAL_CLUSTER_TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    platform
        .cluster_join_tokens
        .controller_call_authenticates(presented)
}

fn mcp_remote_project_worker_id(
    platform: &PlatformService,
    owner: &str,
    project: &str,
) -> Result<Option<String>, Response> {
    let placement = platform
        .cluster_placement
        .get(owner, project)
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed reading project placement: {}", err.message),
            )
                .into_response()
        })?;
    let worker_id = match placement {
        Some(record) if record.target == ProjectRuntimePlacementTarget::Worker => record.worker_id,
        _ => None,
    };
    let local_id = platform.cluster_bootstrap.node_id();
    if worker_id.as_deref() == Some(local_id.as_str()) {
        return Ok(None);
    }
    Ok(worker_id)
}

async fn forward_mcp_request_to_worker(
    platform: &PlatformService,
    req: axum::extract::Request,
    worker_id: &str,
    session: &McpSession,
) -> Result<Response, Response> {
    let worker = platform
        .cluster_registry
        .get_worker(worker_id)
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed reading worker registry: {}", err.message),
            )
                .into_response()
        })?
        .ok_or_else(|| {
            (
                StatusCode::BAD_GATEWAY,
                format!("office '{worker_id}' not found"),
            )
                .into_response()
        })?;
    // Derived per office from that office's own token digest, so this
    // controller proves itself to exactly one office (`offices.md` §8).
    let office_id = if worker.office_id.trim().is_empty() {
        worker.node_id.as_str()
    } else {
        worker.office_id.as_str()
    };
    let token = platform
        .cluster_join_tokens
        .controller_call_header_for(office_id)
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("{}: {}", err.code, err.message),
            )
                .into_response()
        })?;

    let (parts, body) = req.into_parts();
    let body = to_bytes(body, MCP_PROXY_BODY_LIMIT).await.map_err(|err| {
        (
            StatusCode::BAD_REQUEST,
            format!("failed reading MCP request body: {err}"),
        )
            .into_response()
    })?;

    let original_uri = parts.extensions.get::<OriginalUri>();
    let path_and_query = original_uri
        .and_then(|uri| uri.path_and_query().map(|pq| pq.as_str().to_string()))
        .or_else(|| parts.uri.path_and_query().map(|pq| pq.as_str().to_string()))
        .unwrap_or_else(|| parts.uri.path().to_string());
    let url = format!(
        "{}{}",
        worker.base_url.trim_end_matches('/'),
        path_and_query
    );
    let method = reqwest::Method::from_bytes(parts.method.as_str().as_bytes()).map_err(|err| {
        (
            StatusCode::BAD_REQUEST,
            format!("unsupported MCP method '{}': {err}", parts.method),
        )
            .into_response()
    })?;

    let mut request = reqwest::Client::new()
        .request(method, url)
        .header(INTERNAL_CLUSTER_TOKEN_HEADER, token);
    for (name, value) in parts.headers.iter() {
        let header_name = name.as_str();
        if header_name.eq_ignore_ascii_case("host")
            || header_name.eq_ignore_ascii_case("content-length")
            || header_name.eq_ignore_ascii_case("cookie")
            || header_name.eq_ignore_ascii_case(INTERNAL_CLUSTER_TOKEN_HEADER)
            || header_name.eq_ignore_ascii_case(INTERNAL_MCP_SESSION_HEADER)
        {
            continue;
        }
        request = request.header(name, value);
    }

    let session_bytes = serde_json::to_vec(session).map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed serializing MCP session: {err}"),
        )
            .into_response()
    })?;
    let session_header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(session_bytes);
    let response = request
        .header(INTERNAL_MCP_SESSION_HEADER, session_header)
        .body(body)
        .send()
        .await
        .map_err(|err| {
            (
                StatusCode::BAD_GATEWAY,
                format!("MCP worker proxy failed: {err}"),
            )
                .into_response()
        })?;
    Ok(reqwest_response_to_axum(response).await)
}

async fn reqwest_response_to_axum(response: reqwest::Response) -> Response {
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.bytes().await.unwrap_or_else(|_| Bytes::new());
    let mut builder = Response::builder().status(status);
    for (name, value) in headers.iter() {
        if name.as_str().eq_ignore_ascii_case("connection")
            || name.as_str().eq_ignore_ascii_case("transfer-encoding")
        {
            continue;
        }
        builder = builder.header(name, value);
    }
    builder
        .body(Body::from(body))
        .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
}

fn parse_bearer_token(header_value: &str) -> Option<&str> {
    let token = header_value.trim().strip_prefix("Bearer ")?.trim();
    if token.is_empty() { None } else { Some(token) }
}

fn mcp_project_scope_from_request(req: &axum::extract::Request) -> Option<(String, String)> {
    if let Some(original_uri) = req.extensions().get::<OriginalUri>()
        && let Some(scope) = mcp_project_scope_from_path(original_uri.path())
    {
        return Some(scope);
    }
    mcp_project_scope_from_path(req.uri().path())
}

fn mcp_project_scope_from_path(path: &str) -> Option<(String, String)> {
    let segments = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.len() < 5 {
        return None;
    }
    for index in 0..=(segments.len() - 5) {
        if segments[index] == "api"
            && segments[index + 1] == "projects"
            && segments[index + 4] == "mcp"
        {
            return Some((
                segments[index + 2].to_string(),
                segments[index + 3].to_string(),
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{mcp_project_scope_from_path, parse_bearer_token};

    #[test]
    fn parse_bearer_token_rejects_missing_or_empty_tokens() {
        assert_eq!(parse_bearer_token("Bearer abc123"), Some("abc123"));
        assert_eq!(parse_bearer_token("Bearer   abc123  "), Some("abc123"));
        assert_eq!(parse_bearer_token("Bearer "), None);
        assert_eq!(parse_bearer_token("Basic abc123"), None);
        assert_eq!(parse_bearer_token(""), None);
    }

    #[test]
    fn mcp_project_scope_is_extracted_from_project_url() {
        assert_eq!(
            mcp_project_scope_from_path("/api/projects/alice/my-app/mcp"),
            Some(("alice".to_string(), "my-app".to_string()))
        );
        assert_eq!(
            mcp_project_scope_from_path("/api/projects/alice/my-app/mcp/"),
            Some(("alice".to_string(), "my-app".to_string()))
        );
        assert_eq!(mcp_project_scope_from_path("/api/projects/alice"), None);
        assert_eq!(mcp_project_scope_from_path("/mcp"), None);
    }
}

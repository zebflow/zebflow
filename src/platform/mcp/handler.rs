//! Zebflow MCP handler exposing project-scoped tools.
//!
//! Thin adapter over `PlatformOps`. Each `#[tool]` method:
//! 1. Extracts + validates the MCP session
//! 2. Checks tool capability authorization
//! 3. Delegates to `PlatformOps`
//! 4. Wraps the text result in `CallToolResult` (navigate field is ignored for MCP)

use std::sync::Arc;

use axum::http;
use rmcp::handler::server::{ServerHandler, tool::Extension};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ServerCapabilities, ServerInfo};
use rmcp::schemars::JsonSchema;
use rmcp::{ErrorData as McpError, schemars, tool, tool_handler, tool_router};

use crate::platform::model::{McpSession, ProjectAccessSubject, mcp_tool_capability};
use crate::platform::services::{PlatformOps, PlatformService};

// ── Parameter structs (MCP schema only) ──────────────────────────────────────

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineGetParams {
    /// File-relative path of the pipeline (e.g. "pipelines/my-pipeline.zf.json").
    file_rel_path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct TemplateGetParams {
    /// Relative path to the template file (e.g. "pages/home.tsx").
    rel_path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct DocsProjectReadParams {
    /// Relative path to the doc file under repo/docs (e.g. "README.md").
    path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct HelpParams {
    /// Help path to load (e.g. "pipeline", "web/hooks", "pipeline/nodes", "tool").
    /// Omit for the full index.
    topic: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct TemplateWriteParams {
    /// Relative path under templates/ (e.g. "pages/blog-home.tsx", "components/ui/card.tsx").
    rel_path: String,
    /// Full file content to write.
    content: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct TemplateSearchParams {
    /// Pattern to search for (case-insensitive substring).
    pattern: String,
    /// Optional glob to filter files (e.g. "pages/*.tsx", "**/*.tsx"). Omit to search all files.
    glob: Option<String>,
    /// Number of context lines to include before and after each match. Default 0 (match line only).
    context: Option<u32>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineSearchParams {
    /// Pattern to search for (case-insensitive substring).
    pattern: String,
    /// Optional glob to filter pipeline files (e.g. "pipelines/api/*.zf.json"). Omit to search all .zf.json files.
    glob: Option<String>,
    /// Number of context lines to include before and after each match. Default 0 (match line only).
    context: Option<u32>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct TemplateEditParams {
    /// Relative path to the template file (e.g. "pages/home.tsx").
    rel_path: String,
    /// Exact string to find. Must match exactly once — provide enough context to be unique.
    old_string: String,
    /// Replacement string.
    new_string: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct TemplateCreateParams {
    /// Kind of entry to create: "page", "component", "script", or "folder".
    kind: String,
    /// Base name for the file or folder (e.g. "blog-home", "user-card").
    name: String,
    /// Optional parent folder path under templates/ (e.g. "components/ui").
    parent_rel_path: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct DocsProjectWriteParams {
    /// Relative path under repo/docs/ (e.g. "README.md", "architecture.md", "erd.md").
    path: String,
    /// Full file content to write.
    content: String,
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
    /// File-relative path of the pipeline under repo/ (e.g. "pipelines/api/blog-home").
    /// The .zf.json extension is added automatically if omitted.
    /// Preferred over name+path. If omitted, derived from name and path fields.
    #[serde(default)]
    file_rel_path: Option<String>,
    /// Pipeline name slug (e.g. "blog-home", "process-order"). Used when file_rel_path is not set.
    #[serde(default)]
    name: Option<String>,
    /// Virtual path for grouping (e.g. "/pages", "/api", "/jobs"). Defaults to "/". Used when file_rel_path is not set.
    #[serde(default)]
    path: Option<String>,
    /// Optional human-readable display title.
    title: Option<String>,
    /// Pipeline body: pipe-chained nodes starting with |.
    /// Example: "| trigger.webhook --path /blog --method GET | pg.query --credential main-db -- \"SELECT * FROM posts\""
    /// Use help("pipeline/dsl") for the full node catalog and syntax.
    body: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineDescribeParams {
    /// File-relative path of the pipeline (e.g. "pipelines/api/blog-home.zf.json").
    /// Also accepted as "name" for backward compatibility.
    #[serde(alias = "name")]
    file_rel_path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelinePatchParams {
    /// File-relative path of the pipeline (e.g. "pipelines/api/blog-home.zf.json").
    /// Also accepted as "name" for backward compatibility.
    #[serde(alias = "name")]
    file_rel_path: String,
    /// Node ID to patch — get IDs from pipeline_describe output (e.g. "n0", "b", "trigger").
    node_id: String,
    /// Space-separated --flag value pairs to update in the node config.
    /// Example: "--credential new-db --path /updated"
    flags: Option<String>,
    /// Body content for the node (SQL for pg.query, JS source for script nodes).
    body: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineActivateParams {
    /// File-relative path of the pipeline to activate (e.g. "pipelines/api/blog-home.zf.json").
    /// Also accepted as "name" for backward compatibility.
    #[serde(alias = "name")]
    file_rel_path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineDeactivateParams {
    /// File-relative path of the pipeline to deactivate (e.g. "pipelines/api/blog-home.zf.json").
    /// Also accepted as "name" for backward compatibility.
    #[serde(alias = "name")]
    file_rel_path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineExecuteParams {
    /// File-relative path of the registered active pipeline to execute (e.g. "pipelines/api/blog-home.zf.json").
    /// Also accepted as "name" for backward compatibility.
    #[serde(alias = "name")]
    file_rel_path: String,
    /// Optional JSON input payload string (e.g. "{\"order_id\": 42}").
    input: Option<String>,
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
struct GitCommandParams {
    /// Git subcommand: status, log, diff, add, commit
    subcommand: String,
    /// Additional arguments as a space-separated string (e.g. "path/to/file" for add/diff, "--limit 10" for log).
    args: Option<String>,
    /// Commit message — only used when subcommand is "commit".
    message: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct ConnectionDescribeParams {
    /// Connection slug — get slugs from connection_list (e.g. "main-db", "default").
    slug: String,
    /// Scope to inspect: "tables", "schemas", "functions", or omit for full tree.
    scope: Option<String>,
    /// Filter to a specific schema name (e.g. "public"). Only meaningful with scope="tables".
    schema: Option<String>,
    /// Filter to a specific table for column-level detail.
    /// Format: "schema.table" (e.g. "academic.staff") or just "table" for public schema.
    /// Use scope="tables" first to discover table names, then table=<name> for columns.
    table: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct HelpSearchParams {
    /// Search query — node name, DSL flag, concept, or keyword.
    query: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct InstallUiComponentsParams {
    /// Component names to install, e.g. ["button", "card", "dialog"].
    names: Vec<String>,
    /// If true, overwrite existing files. Default: false.
    #[serde(default)]
    overwrite: Option<bool>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct MoveParams {
    /// Source path to move from.
    /// Pipelines: file_rel_path e.g. "pipelines/api/old-name.zf.json" or just "old-name".
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
    tool_router: rmcp::handler::server::tool::ToolRouter<Self>,
}

#[tool_router]
impl ZebflowMcpHandler {
    pub fn new(platform: Arc<PlatformService>) -> Self {
        Self {
            platform,
            tool_router: Self::tool_router(),
        }
    }

    // ── Orientation ──────────────────────────────────────────────────────────

    #[tool(description = "Call this first. Returns Zebflow platform overview, project name, \
        project docs list, AGENTS.md content (if exists), DB connections, template tree, \
        and which help tools to call next. Your orientation before building anything.")]
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
    async fn version(
        &self,
    ) -> Result<CallToolResult, McpError> {
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
        'tool' (Tool.time/arr/stat/geo), 'db', 'db/sekejap', \
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

    #[tool(description = "Search Zebflow docs. Returns matching chunks from pipeline docs, \
        web template docs, node catalog, and all help files. Use for any concept, node name, DSL flag, \
        or syntax question. Example: query='jwt', query='sekejap upsert', query='web.response'.")]
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

    // ── Pipelines ────────────────────────────────────────────────────────────

    #[tool(description = "List all pipelines in the project")]
    async fn pipeline_list(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_list")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.pipeline_list();
        ok_or_err(result)
    }

    #[tool(
        description = "Search pipeline .zf.json files for a pattern. Returns file:line matches. \
                       Use glob to narrow scope (e.g. \"pipelines/api/*.zf.json\"). \
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
        ok_or_err(ops.pipeline_search(&params.pattern, params.glob.as_deref(), params.context.unwrap_or(0) as usize))
    }

    #[tool(description = "Get a specific pipeline by file-relative path")]
    async fn pipeline_get(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineGetParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_get")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.pipeline_get(&params.file_rel_path);
        ok_or_err(result)
    }

    #[tool(
        description = "Register (create or update) a pipeline by name and pipe-chained node body. \
                       Body format: '| trigger.webhook --path /x | pg.query --credential db -- \"SQL\"'. \
                       After registering, call pipeline_activate to make it live. \
                       Use help(\"pipeline/dsl\") for the full node catalog and syntax."
    )]
    async fn pipeline_register(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineRegisterParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_register")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.pipeline_register(
            &params.body,
            params.file_rel_path.as_deref(),
            params.name.as_deref(),
            params.path.as_deref(),
            params.title.as_deref(),
        ).await;
        // navigate is ignored for MCP
        Ok(CallToolResult::success(vec![Content::text(result.text)]))
    }

    #[tool(
        description = "Inspect a pipeline — returns its nodes, edges, status, and hit stats. \
                       Node IDs from this output are required for pipeline_patch."
    )]
    async fn pipeline_describe(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineDescribeParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_describe")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.pipeline_describe(&params.file_rel_path).await;
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
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.pipeline_patch(
            &params.file_rel_path,
            &params.node_id,
            params.flags.as_deref(),
            params.body.as_deref(),
        ).await;
        Ok(CallToolResult::success(vec![Content::text(result.text)]))
    }

    #[tool(
        description = "Activate a pipeline — makes it live so it can serve traffic and be executed. \
                       Must be called after pipeline_register or after patching. \
                       A pipeline must be active before pipeline_execute will run it."
    )]
    async fn pipeline_activate(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineActivateParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_activate")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
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
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.pipeline_deactivate(&params.file_rel_path).await;
        Ok(CallToolResult::success(vec![Content::text(result.text)]))
    }

    #[tool(
        description = "Execute a registered active pipeline by name. Records execution hits. \
                       Pipeline must be activated first — use pipeline_activate if status is draft or stale. \
                       Use pipeline_list to see pipeline names and activation status."
    )]
    async fn pipeline_execute(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineExecuteParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_execute")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.pipeline_execute(&params.file_rel_path, params.input.as_deref()).await;
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

    // ── Templates ────────────────────────────────────────────────────────────

    #[tool(description = "List all templates in the project workspace")]
    async fn template_list(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "template_list")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.template_list();
        ok_or_err(result)
    }

    #[tool(description = "Get a specific template by relative path")]
    async fn template_get(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<TemplateGetParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "template_get")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.template_get(&params.rel_path);
        ok_or_err(result)
    }

    #[tool(
        description = "Create a new template file with scaffolding. \
                       Kind must be one of: page (pages/*.tsx), component (components/*.tsx), \
                       script (scripts/*.ts), folder. \
                       Returns the scaffolded content — use template_write to customise it after."
    )]
    async fn template_create(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<TemplateCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "template_create")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.template_create(&params.kind, &params.name, params.parent_rel_path.as_deref());
        // navigate is ignored for MCP
        ok_or_err(result)
    }

    #[tool(
        description = "Write (create or overwrite) a template file. \
                       Use template_create first to scaffold with boilerplate, then template_write to fill in content. \
                       Path is relative to templates/ (e.g. 'pages/blog-home.tsx', 'components/ui/card.tsx'). \
                       Use help(\"web\") for TSX conventions before writing."
    )]
    async fn template_write(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<TemplateWriteParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "template_write")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.template_write(&params.rel_path, &params.content);
        // navigate is ignored for MCP
        ok_or_err(result)
    }

    #[tool(
        description = "Search template files for a pattern. Returns file:line matches. \
                       Use glob to narrow scope (e.g. \"pages/*.tsx\", \"**/*.tsx\"). \
                       Equivalent to Grep across templates — find which files use an import, component, or value."
    )]
    async fn template_search(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<TemplateSearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "template_search")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        ok_or_err(ops.template_search(&params.pattern, params.glob.as_deref(), params.context.unwrap_or(0) as usize))
    }

    #[tool(
        description = "Surgical string replacement in a template file. \
                       Equivalent to Edit — no need to read the full file first. \
                       Fails if old_string is not found or matches more than once (provide more context). \
                       Returns the line number of the replacement."
    )]
    async fn template_edit(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<TemplateEditParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "template_edit")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        ok_or_err(ops.template_edit(&params.rel_path, &params.old_string, &params.new_string))
    }

    // ── Project Docs ─────────────────────────────────────────────────────────

    #[tool(description = "List project doc files (ERD, README.md, architecture docs, use cases) under repo/docs")]
    async fn docs_project_list(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "docs_project_list")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.docs_project_list();
        ok_or_err(result)
    }

    #[tool(description = "Read one project doc by path (e.g. README.md, AGENTS.md, architecture.md)")]
    async fn docs_project_read(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<DocsProjectReadParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "docs_project_read")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.docs_project_read(&params.path);
        ok_or_err(result)
    }

    #[tool(
        description = "Write (create or update) a project doc file under repo/docs/. \
                       Use for specs, architecture docs, ERDs, API contracts, README, CHANGELOG. \
                       These files are git-synced. Always commit after writing with git_command."
    )]
    async fn docs_project_write(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<DocsProjectWriteParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "docs_project_write")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.docs_project_write(&params.path, &params.content);
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

    #[tool(
        description = "Read one agent doc: AGENTS.md (project instructions), \
                       SOUL.md (agent personality), or MEMORY.md (persistent memory). \
                       Read AGENTS.md at the start of every session to understand the project."
    )]
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

    #[tool(description = "List all DB connections for this project — returns slug, label, and kind (postgres, mysql, sekejap). Use the slug with connection_describe and in --credential flags.")]
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

    #[tool(description = "Describe a DB connection's schema — tables, columns, types, constraints. Use scope='tables' for a quick overview, scope='schemas' to list schemas, or omit scope for the full tree. Always run this before writing SQL queries. Use table='schema.table' (e.g. table='academic.staff') to get column detail for a specific table.")]
    async fn connection_describe(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<ConnectionDescribeParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "connection_describe")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.connection_describe(
            &params.slug,
            params.scope.as_deref(),
            params.schema.as_deref(),
            params.table.as_deref(),
        ).await;
        ok_or_err(result)
    }

    #[tool(description = "List credentials for this project — returns id, title, and kind only. Values are never exposed. Use the id in pipeline nodes that require authentication.")]
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

    #[tool(
        description = "Run a git command on the project repository. \
                       Allowed subcommands: status, log, diff, add, commit. \
                       Destructive operations (reset, rebase, force, checkout) are blocked. \
                       Always commit after registering or patching pipelines."
    )]
    async fn git_command(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<GitCommandParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "git_command")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.git_command(
            &params.subcommand,
            params.args.as_deref(),
            params.message.as_deref(),
        ).await;
        Ok(CallToolResult::success(vec![Content::text(result.text)]))
    }

    // ── UI Catalog ───────────────────────────────────────────────────────────

    #[tool(description = "List all available shadcn-compatible Zeb React UI components \
        that can be installed into shared/ui/. Returns name, category, description, \
        filename, and whether each component is already installed in this project.")]
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

    #[tool(description = "Install shadcn-compatible UI components into shared/ui/. \
        Pass names like [\"button\",\"card\",\"dialog\"]. \
        Set overwrite=true to replace existing files. \
        Returns installed and skipped lists.")]
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

    // ── Move Resource ─────────────────────────────────────────────────────────

    #[tool(
        description = "Rename or reorganize a pipeline or template file. \
            Domain detected automatically from path: .zf.json = pipeline, anything else = template. \
            For pipelines: deactivate → move → re-activate lifecycle handled automatically. \
            Parent folders created automatically. No cross-domain moves (pipeline ↔ template). \
            node_id can be: opaque ID (e.g. n0), kind (e.g. trigger.webhook), or kind+index (e.g. pg.query[1])."
    )]
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

    fn check_tool_capability(&self, session: &McpSession, tool_name: &str) -> Result<(), McpError> {
        let required_capability = mcp_tool_capability(tool_name).ok_or_else(|| {
            McpError::invalid_params(format!("Unknown tool '{tool_name}'"), None)
        })?;

        let subject = ProjectAccessSubject::mcp_session(&session.token);
        match self.platform.authz.ensure_project_capability(
            &subject,
            &session.owner,
            &session.project,
            required_capability,
        ) {
            Ok(()) => Ok(()),
            Err(_) => Err(McpError::invalid_params(
                format!(
                    "Tool '{tool_name}' requires capability '{}' which is not allowed in this session",
                    required_capability.key()
                ),
                None,
            )),
        }
    }
}

#[tool_handler]
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
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Convert an `OpsResult` to `CallToolResult`, treating "Error: ..." text as an MCP error.
fn ok_or_err(result: crate::platform::services::ops::OpsResult) -> Result<CallToolResult, McpError> {
    if result.text.starts_with("Error:") {
        Err(McpError::internal_error(result.text, None))
    } else {
        Ok(CallToolResult::success(vec![Content::text(result.text)]))
    }
}

/// Build the MCP service with token validation using rmcp's StreamableHttpService.
pub fn build_mcp_service<S: Clone + Send + Sync + 'static>(
    platform: Arc<PlatformService>,
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
    let service = StreamableHttpService::new(
        move || {
            let platform = platform_for_factory.clone();
            let handler = ZebflowMcpHandler::new(platform);
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
                    let token = req
                        .headers()
                        .get("authorization")
                        .and_then(|h| h.to_str().ok())
                        .and_then(|s| s.strip_prefix("Bearer "))
                        .unwrap_or("");

                    if !token.is_empty() {
                        if let Some(session) = platform.mcp_sessions.lookup(token) {
                            req.extensions_mut().insert(session);
                        }
                    }

                    next.run(req).await
                }
            },
        ))
}

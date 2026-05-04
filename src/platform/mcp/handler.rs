//! Zebflow MCP handler exposing project-scoped tools.
//!
//! Thin adapter over `PlatformOps`. Each `#[tool]` method:
//! 1. Extracts + validates the MCP session
//! 2. Checks tool capability authorization
//! 3. Delegates to `PlatformOps`
//! 4. Wraps the text result in `CallToolResult` (navigate field is ignored for MCP)

use std::sync::Arc;

use axum::extract::OriginalUri;
use axum::http::{self, StatusCode, header};
use axum::response::IntoResponse;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::handler::server::{ServerHandler, tool::Extension};
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
    /// 1-based starting line number. Omit to read the full file.
    #[schemars(with = "u32")]
    offset: Option<u32>,
    /// Number of lines to return. Omit to read to the end.
    #[schemars(with = "u32")]
    limit: Option<u32>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct TemplateListParams {
    /// Optional glob to filter files (e.g. "pages/*.tsx", "**/*.tsx"). Omit to list all files.
    #[schemars(with = "String")]
    glob: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct DocsProjectReadParams {
    /// Relative path to the doc file under repo/docs (e.g. "README.md").
    path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct HelpParams {
    /// Help path to load (e.g. "pipeline", "web/hooks", "pipeline/nodes", "tool").
    /// Pass empty string or omit for the full index.
    #[schemars(with = "String")]
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
    /// Optional glob to filter pipeline files (e.g. "pipelines/api/*.zf.json"). Omit to search all .zf.json files.
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
struct TemplateEditParams {
    /// Relative path to the template file (e.g. "pages/home.tsx").
    rel_path: String,
    /// Exact string to find. Must match exactly once — provide enough context to be unique.
    old_string: String,
    /// Replacement string.
    new_string: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct TemplateOutlineParams {
    /// Relative path to the template file (e.g. "pages/home.tsx").
    rel_path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct TemplateDepsParams {
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
struct TemplateBatchEditParams {
    /// Edits to apply. Fails fast on the first edit error.
    edits: Vec<TemplateBatchEditItem>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct TemplateCreateParams {
    /// Kind of entry to create: "page", "component", "script", or "folder".
    kind: String,
    /// Base name for the file or folder (e.g. "blog-home", "user-card").
    name: String,
    /// Optional parent folder path under templates/ (e.g. "components/ui").
    #[schemars(with = "String")]
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
    /// File-relative path of the pipeline (e.g. "pipelines/api/blog-home.zf.json").
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
    /// File-relative path of the pipeline (e.g. "pipelines/api/blog-home.zf.json").
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
    /// File-relative path of the pipeline to activate (e.g. "pipelines/api/blog-home.zf.json").
    /// Also accepted as "name" for backward compatibility.
    /// Ignored when glob is set.
    #[serde(alias = "name", default)]
    file_rel_path: String,
    /// Glob pattern to bulk-activate matching pipelines (e.g. "pipelines/modules/manage/**").
    /// When set, activates all pipelines whose file_rel_path matches. file_rel_path is ignored.
    #[serde(default)]
    #[schemars(with = "String")]
    glob: Option<String>,
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
    /// File-relative path of the pipeline (e.g. "pipelines/api/blog-home.zf.json").
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
        // Check lock on existing pipeline (by resolved file_rel_path if provided)
        if let Some(ref frp) = params.file_rel_path {
            let frp = crate::platform::services::project::normalize_pipeline_file_rel_path(frp);
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
                       Set glob (e.g. \"pipelines/modules/manage/**\") to bulk-activate all matching pipelines \
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
    async fn template_list(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<TemplateListParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "template_list")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.template_list(params.glob.as_deref());
        ok_or_err(result)
    }

    #[tool(description = "Get a specific template by relative path. \
                       Use offset and limit to read a line-numbered slice instead of the full file.")]
    async fn template_get(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<TemplateGetParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "template_get")?;
        if self.template_locked(&session.owner, &session.project, &params.rel_path) {
            return Err(McpError::invalid_params(
                "This template is locked by the project owner and cannot be accessed by agents. Ask the owner to unlock it.",
                None,
            ));
        }
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.template_get(&params.rel_path, params.offset, params.limit);
        ok_or_err(result)
    }

    #[tool(description = "Create a new template file with scaffolding. \
                       Kind must be one of: page (pages/*.tsx), component (components/*.tsx), \
                       script (scripts/*.ts), folder. \
                       Returns the scaffolded content — use template_write to customise it after.")]
    async fn template_create(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<TemplateCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "template_create")?;
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
        let result = ops.template_create(
            &params.kind,
            &params.name,
            params.parent_rel_path.as_deref(),
        );
        // navigate is ignored for MCP
        ok_or_err(result)
    }

    #[tool(description = "Write (create or overwrite) a template file. \
                       Use template_create first to scaffold with boilerplate, then template_write to fill in content. \
                       Path is relative to templates/ (e.g. 'pages/blog-home.tsx', 'components/ui/card.tsx'). \
                       Use help(\"web\") for TSX conventions before writing.")]
    async fn template_write(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<TemplateWriteParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "template_write")?;
        if self.template_locked(&session.owner, &session.project, &params.rel_path) {
            return Err(McpError::invalid_params(
                "This template is locked by the project owner and cannot be accessed by agents. Ask the owner to unlock it.",
                None,
            ));
        }
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.template_write(&params.rel_path, &params.content);
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
    async fn template_search(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<TemplateSearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "template_search")?;
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        ok_or_err(ops.template_search(
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
    async fn template_edit(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<TemplateEditParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "template_edit")?;
        if self.template_locked(&session.owner, &session.project, &params.rel_path) {
            return Err(McpError::invalid_params(
                "This template is locked by the project owner and cannot be accessed by agents. Ask the owner to unlock it.",
                None,
            ));
        }
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.template_edit(&params.rel_path, &params.old_string, &params.new_string);
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
                       Use this before template_get when orienting on a large TSX/TS file."
    )]
    async fn template_outline(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<TemplateOutlineParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "template_outline")?;
        if self.template_locked(&session.owner, &session.project, &params.rel_path) {
            return Err(McpError::invalid_params(
                "This template is locked by the project owner and cannot be accessed by agents. Ask the owner to unlock it.",
                None,
            ));
        }
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.template_outline(&params.rel_path);
        ok_or_err(result)
    }

    #[tool(
        description = "Show a template dependency graph: imports used by this file and \
                       other project templates that import it. Use before refactoring shared UI."
    )]
    async fn template_deps(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<TemplateDepsParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "template_deps")?;
        if self.template_locked(&session.owner, &session.project, &params.rel_path) {
            return Err(McpError::invalid_params(
                "This template is locked by the project owner and cannot be accessed by agents. Ask the owner to unlock it.",
                None,
            ));
        }
        let ops = PlatformOps::new(self.platform.clone(), &session.owner, &session.project);
        let result = ops.template_deps(&params.rel_path);
        ok_or_err(result)
    }

    #[tool(
        description = "Apply multiple exact string edits across template files in one call. \
                       Each edit is rel_path + old_string + new_string. \
                       Evicts template cache for edited files when the batch succeeds."
    )]
    async fn template_batch_edit(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<TemplateBatchEditParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "template_batch_edit")?;
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
        let result = ops.template_batch_edit(&edits);
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

    // ── Project Docs ─────────────────────────────────────────────────────────

    #[tool(
        description = "List project doc files (ERD, README.md, architecture docs, use cases) under repo/docs"
    )]
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

    #[tool(
        description = "Read one project doc by path (e.g. README.md, AGENTS.md, architecture.md)"
    )]
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
        description = "List all DB connections for this project — returns slug, label, and kind (postgres, mysql, sqlite). Use the slug with connection_describe and in --credential flags."
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

    // ── Move Resource ─────────────────────────────────────────────────────────

    #[tool(description = "Rename or reorganize a pipeline or template file. \
            Domain detected automatically from path: .zf.json = pipeline, anything else = template. \
            For pipelines: deactivate → move → re-activate lifecycle handled automatically. \
            Parent folders created automatically. No cross-domain moves (pipeline ↔ template). \
            node_id can be: opaque ID (e.g. n0), kind (e.g. trigger.webhook), or kind+index (e.g. pg.query[1]).")]
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

    fn check_tool_capability(&self, session: &McpSession, tool_name: &str) -> Result<(), McpError> {
        let required_capability = mcp_tool_capability(tool_name)
            .ok_or_else(|| McpError::invalid_params(format!("Unknown tool '{tool_name}'"), None))?;

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
                    let Some(token) = req
                        .headers()
                        .get(header::AUTHORIZATION)
                        .and_then(|h| h.to_str().ok())
                        .and_then(parse_bearer_token)
                    else {
                        return StatusCode::UNAUTHORIZED.into_response();
                    };

                    let Some(session) = platform.mcp_sessions.lookup(token) else {
                        return StatusCode::UNAUTHORIZED.into_response();
                    };

                    let Some((owner, project)) = mcp_project_scope_from_request(&req) else {
                        return StatusCode::BAD_REQUEST.into_response();
                    };
                    if crate::platform::model::slug_segment(&session.owner)
                        != crate::platform::model::slug_segment(&owner)
                        || crate::platform::model::slug_segment(&session.project)
                            != crate::platform::model::slug_segment(&project)
                    {
                        return StatusCode::FORBIDDEN.into_response();
                    }

                    req.extensions_mut().insert(session);

                    next.run(req).await
                }
            },
        ))
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

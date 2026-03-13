//! Zebflow MCP handler exposing project-scoped tools.

use std::sync::Arc;

use axum::http;
use rmcp::handler::server::{ServerHandler, tool::Extension};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ServerCapabilities, ServerInfo};
use rmcp::schemars::JsonSchema;
use rmcp::{ErrorData as McpError, schemars, tool, tool_handler, tool_router};
use serde_json::json;

use crate::platform::model::{
    DescribeProjectDbConnectionRequest, McpSession, ProjectAccessSubject, TemplateCreateKind,
    TemplateCreateRequest, TemplateSaveRequest, mcp_tool_capability,
};
use crate::platform::services::PlatformService;

#[derive(serde::Deserialize, JsonSchema)]
struct GetPipelineParams {
    /// File-relative path of the pipeline (e.g. "pipelines/my-pipeline.zf.json").
    file_rel_path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct GetTemplateParams {
    /// Relative path to the template file (e.g. "pages/home.tsx").
    rel_path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct ReadProjectDocParams {
    /// Relative path to the doc file under app/docs (e.g. "README.md").
    path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct ReadSkillParams {
    /// Skill name to read (e.g. "pipeline-authoring", "rwe-templates").
    name: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct WriteTemplateParams {
    /// Relative path under templates/ (e.g. "pages/blog-home.tsx", "components/ui/card.tsx").
    rel_path: String,
    /// Full file content to write.
    content: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct CreateTemplateParams {
    /// Kind of entry to create: "page", "component", "script", or "folder".
    kind: String,
    /// Base name for the file or folder (e.g. "blog-home", "user-card").
    name: String,
    /// Optional parent folder path under templates/ (e.g. "components/ui").
    parent_rel_path: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct WriteDocParams {
    /// Relative path under repo/docs/ (e.g. "README.md", "architecture.md", "erd.md").
    path: String,
    /// Full file content to write.
    content: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct ReadAgentDocParams {
    /// Agent doc name: "AGENTS.md", "SOUL.md", or "MEMORY.md".
    name: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct WriteAgentDocParams {
    /// Agent doc name: "AGENTS.md" (project instructions), "SOUL.md" (agent personality),
    /// or "MEMORY.md" (persistent memory across sessions).
    name: String,
    /// Full file content to write.
    content: String,
}

// execute_pipeline_dsl is temporarily disabled in favour of focused pipeline tools.
// #[derive(serde::Deserialize, JsonSchema)]
// struct ExecutePipelineDslParams { dsl: String }

#[derive(serde::Deserialize, JsonSchema)]
struct RegisterPipelineParams {
    /// Pipeline name slug (e.g. "blog-home", "process-order").
    name: String,
    /// Virtual path for grouping (e.g. "/pages", "/api", "/jobs"). Defaults to "/".
    path: Option<String>,
    /// Optional human-readable display title.
    title: Option<String>,
    /// Pipeline body: pipe-chained nodes starting with |.
    /// Example: "| trigger.webhook --path /blog --method GET | pg.query --credential main-db -- \"SELECT * FROM posts\""
    /// Use read_skill with name "pipeline-dsl" for the full node catalog and syntax.
    body: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct DescribePipelineParams {
    /// Pipeline name to inspect (e.g. "blog-home").
    name: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PatchPipelineParams {
    /// Pipeline name (e.g. "blog-home").
    name: String,
    /// Node ID to patch — get IDs from describe_pipeline output (e.g. "n0", "b", "trigger").
    node_id: String,
    /// Space-separated --flag value pairs to update in the node config.
    /// Example: "--credential new-db --path /updated"
    flags: Option<String>,
    /// Body content for the node (SQL for pg.query, JS source for script nodes).
    body: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct ActivatePipelineParams {
    /// Name of the pipeline to activate (make live).
    name: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct DeactivatePipelineParams {
    /// Name of the pipeline to deactivate (take offline).
    name: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct ExecutePipelineParams {
    /// Name of the registered active pipeline to execute.
    name: String,
    /// Optional JSON input payload string (e.g. "{\"order_id\": 42}").
    input: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct RunEphemeralParams {
    /// Pipe-chained node body to execute inline — NOT saved, NOT logged.
    /// Starts with | followed by nodes: "| pg.query --credential main-db -- \"SELECT count(*) FROM users\""
    /// Auto-prepends trigger.manual if no trigger node is specified.
    /// Use this for testing queries, one-off scripts, or data exploration.
    body: String,
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
struct DescribeConnectionParams {
    /// Connection slug — get slugs from list_connections (e.g. "main-db", "default").
    slug: String,
    /// Scope to inspect: "tables", "schemas", "functions", or omit for full tree.
    scope: Option<String>,
    /// Filter to a specific schema name (e.g. "public"). Only meaningful with scope="tables".
    schema: Option<String>,
}

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

    #[tool(description = "List all pipelines in the project")]
    async fn list_pipelines(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "list_pipelines")?;

        match self
            .platform
            .projects
            .list_pipeline_meta_rows(&session.owner, &session.project)
        {
            Ok(pipelines) => {
                let content = json!({
                    "pipelines": pipelines,
                    "count": pipelines.len()
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&content).unwrap(),
                )]))
            }
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(description = "List all templates in the project workspace")]
    async fn list_templates(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "list_templates")?;

        match self
            .platform
            .projects
            .list_template_workspace(&session.owner, &session.project)
        {
            Ok(workspace) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&workspace).unwrap(),
            )])),
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(description = "Get a specific pipeline by file-relative path")]
    async fn get_pipeline(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<GetPipelineParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "get_pipeline")?;

        let meta = self
            .platform
            .projects
            .get_pipeline_meta_by_file_id(&session.owner, &session.project, &params.file_rel_path)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let Some(meta) = meta else {
            return Err(McpError::invalid_params(
                format!("Pipeline '{}' not found", params.file_rel_path),
                None,
            ));
        };
        let source = self
            .platform
            .projects
            .read_pipeline_source(&session.owner, &session.project, &meta.file_rel_path)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let content = json!({ "meta": meta, "source": source });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&content).unwrap(),
        )]))
    }

    #[tool(description = "Get a specific template by relative path")]
    async fn get_template(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<GetTemplateParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "get_template")?;

        match self
            .platform
            .projects
            .read_template_file(&session.owner, &session.project, &params.rel_path)
        {
            Ok(content) => Ok(CallToolResult::success(vec![Content::text(content)])),
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(
        description = "Create a new template file with scaffolding. \
                       Kind must be one of: page (pages/*.tsx), component (components/*.tsx), \
                       script (scripts/*.ts), folder. \
                       Returns the scaffolded content — use write_template to customise it after."
    )]
    async fn create_template(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<CreateTemplateParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "create_template")?;

        let kind = match params.kind.as_str() {
            "page" => TemplateCreateKind::Page,
            "component" => TemplateCreateKind::Component,
            "script" => TemplateCreateKind::Script,
            "folder" => TemplateCreateKind::Folder,
            other => return Err(McpError::invalid_params(
                format!("Invalid kind '{}'. Must be: page, component, script, folder", other),
                None,
            )),
        };

        let req = TemplateCreateRequest {
            kind,
            name: params.name,
            parent_rel_path: params.parent_rel_path,
        };

        match self.platform.projects.create_template_entry(&session.owner, &session.project, &req) {
            Ok(payload) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&payload).unwrap(),
            )])),
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(
        description = "Write (create or overwrite) a template file. \
                       Use create_template first to scaffold with boilerplate, then write_template to fill in content. \
                       Path is relative to templates/ (e.g. 'pages/blog-home.tsx', 'components/ui/card.tsx'). \
                       Use read_skill 'rwe-templates' for TSX conventions before writing."
    )]
    async fn write_template(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<WriteTemplateParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "write_template")?;

        let req = TemplateSaveRequest {
            rel_path: params.rel_path,
            content: params.content,
        };

        match self.platform.projects.write_template_file(&session.owner, &session.project, &req) {
            Ok(payload) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&payload).unwrap(),
            )])),
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(
        description = "Write (create or update) a project doc file under repo/docs/. \
                       Use for specs, architecture docs, ERDs, API contracts, README, CHANGELOG. \
                       These files are git-synced. Always commit after writing with git_command."
    )]
    async fn write_doc(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<WriteDocParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "write_doc")?;

        match self.platform.projects.upsert_project_doc(
            &session.owner,
            &session.project,
            &params.path,
            &params.content,
        ) {
            Ok(doc) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&doc).unwrap(),
            )])),
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(
        description = "List the three agent doc files: AGENTS.md (project instructions for agents), \
                       SOUL.md (agent personality and behavior config), MEMORY.md (persistent memory across sessions). \
                       Always read AGENTS.md first when starting work on a project."
    )]
    async fn list_agent_docs(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "list_agent_docs")?;

        match self.platform.projects.list_agent_docs(&session.owner, &session.project) {
            Ok(docs) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&docs).unwrap(),
            )])),
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(
        description = "Read one agent doc: AGENTS.md (project instructions), \
                       SOUL.md (agent personality), or MEMORY.md (persistent memory). \
                       Read AGENTS.md at the start of every session to understand the project."
    )]
    async fn read_agent_doc(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<ReadAgentDocParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "read_agent_doc")?;

        match self.platform.projects.read_agent_doc(&session.owner, &session.project, &params.name) {
            Ok(content) => Ok(CallToolResult::success(vec![Content::text(content)])),
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(
        description = "Write an agent doc. AGENTS.md: project-specific instructions for all agents. \
                       SOUL.md: agent personality, tone, and behavioral config. \
                       MEMORY.md: persistent notes the agent writes to remember things across sessions. \
                       Agents should update MEMORY.md after completing significant work."
    )]
    async fn write_agent_doc(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<WriteAgentDocParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "write_agent_doc")?;

        match self.platform.projects.upsert_agent_doc(
            &session.owner,
            &session.project,
            &params.name,
            &params.content,
        ) {
            Ok(()) => Ok(CallToolResult::success(vec![Content::text(
                format!("{} written successfully.", params.name),
            )])),
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(description = "List all DB connections for this project — returns slug, label, and kind (postgres, mysql, sjtable). Use the slug with describe_connection and in --credential flags.")]
    async fn list_connections(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "list_connections")?;

        match self
            .platform
            .db_connections
            .list_project_connections(&session.owner, &session.project)
        {
            Ok(items) => {
                let content = json!({
                    "connections": items.iter().map(|c| json!({
                        "slug": c.connection_slug,
                        "label": c.connection_label,
                        "kind": c.database_kind,
                    })).collect::<Vec<_>>(),
                    "count": items.len(),
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&content).unwrap(),
                )]))
            }
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(description = "Describe a DB connection's schema — tables, columns, types, constraints. Use scope='tables' for a quick overview, scope='schemas' to list schemas, or omit scope for the full tree. Always run this before writing SQL queries.")]
    async fn describe_connection(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<DescribeConnectionParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "describe_connection")?;

        let conn = self
            .platform
            .db_connections
            .get_project_connection(&session.owner, &session.project, &params.slug)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .ok_or_else(|| {
                McpError::invalid_params(
                    format!("Connection '{}' not found", params.slug),
                    None,
                )
            })?;

        let req = DescribeProjectDbConnectionRequest {
            scope: params.scope,
            schema: params.schema,
            include_system: Some(false),
        };

        match self
            .platform
            .db_runtime
            .describe_connection(&session.owner, &session.project, &conn.connection_id, &req)
            .await
        {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap(),
            )])),
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(description = "List credentials for this project — returns id, title, and kind only. Values are never exposed. Use the id in pipeline nodes that require authentication.")]
    async fn list_credentials(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "list_credentials")?;

        match self
            .platform
            .credentials
            .list_project_credentials(&session.owner, &session.project)
        {
            Ok(items) => {
                let content = json!({
                    "credentials": items.iter().map(|c| json!({
                        "id": c.credential_id,
                        "title": c.title,
                        "kind": c.kind,
                        "notes": c.notes,
                    })).collect::<Vec<_>>(),
                    "count": items.len(),
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&content).unwrap(),
                )]))
            }
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(
        description = "List project doc files (ERD, README.md, AGENTS.md, use cases) under app/docs"
    )]
    async fn list_project_docs(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "list_project_docs")?;

        match self
            .platform
            .projects
            .list_project_docs(&session.owner, &session.project)
        {
            Ok(docs) => {
                let content = json!({ "docs": docs, "count": docs.len() });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&content).unwrap(),
                )]))
            }
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(description = "Read one project doc by path (e.g. README.md, AGENTS.md)")]
    async fn read_project_doc(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<ReadProjectDocParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "read_project_doc")?;

        match self
            .platform
            .projects
            .read_project_doc(&session.owner, &session.project, &params.path)
        {
            Ok(content) => Ok(CallToolResult::success(vec![Content::text(content)])),
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(description = "List all available Zebflow platform skills (operational knowledge docs)")]
    async fn list_skills(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "list_skills")?;

        let skills = crate::platform::skills::all_skills();
        let items: Vec<_> = skills
            .iter()
            .map(|s| {
                json!({
                    "name": s.name,
                    "title": s.title,
                    "summary": s.summary(),
                })
            })
            .collect();
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&json!({"skills": items, "count": items.len()})).unwrap(),
        )]))
    }

    #[tool(description = "Read the full content of a Zebflow platform skill by name")]
    async fn read_skill(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<ReadSkillParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "read_skill")?;

        match crate::platform::skills::get_skill(&params.name) {
            Some(skill) => Ok(CallToolResult::success(vec![Content::text(skill.content)])),
            None => Err(McpError::invalid_params(
                format!(
                    "Skill '{}' not found. Use list_skills to see available skills.",
                    params.name
                ),
                None,
            )),
        }
    }

    // execute_pipeline_dsl is temporarily disabled in favour of focused pipeline tools.
    // async fn execute_pipeline_dsl(...) { ... }

    #[tool(
        description = "Register (create or update) a pipeline by name and pipe-chained node body. \
                       Body format: '| trigger.webhook --path /x | pg.query --credential db -- \"SQL\"'. \
                       After registering, call activate_pipeline to make it live. \
                       Use read_skill with name 'pipeline-dsl' for the full node catalog and syntax."
    )]
    async fn register_pipeline(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<RegisterPipelineParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "register_pipeline")?;

        let mut dsl = format!("register {}", params.name);
        if let Some(path) = &params.path {
            dsl.push_str(&format!(" --path {}", path));
        }
        if let Some(title) = &params.title {
            dsl.push_str(&format!(" --title \"{}\"", title));
        }
        dsl.push(' ');
        dsl.push_str(&params.body);

        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &session.owner,
            &session.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        let formatted = output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
        Ok(CallToolResult::success(vec![Content::text(formatted)]))
    }

    #[tool(
        description = "Inspect a pipeline — returns its nodes, edges, status, and hit stats. \
                       Node IDs from this output are required for patch_pipeline."
    )]
    async fn describe_pipeline(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<DescribePipelineParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "describe_pipeline")?;

        let dsl = format!("describe pipeline {}", params.name);
        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &session.owner,
            &session.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        let formatted = output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
        Ok(CallToolResult::success(vec![Content::text(formatted)]))
    }

    #[tool(
        description = "Patch one node in a saved pipeline without rewriting the full graph. \
                       Call describe_pipeline first to get node IDs. \
                       Pipeline status becomes stale after patching — call activate_pipeline to make it live again."
    )]
    async fn patch_pipeline(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PatchPipelineParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "patch_pipeline")?;

        let mut dsl = format!("patch pipeline {} node {}", params.name, params.node_id);
        if let Some(flags) = &params.flags {
            dsl.push(' ');
            dsl.push_str(flags);
        }
        if let Some(body) = &params.body {
            dsl.push_str(&format!(" -- {}", body));
        }

        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &session.owner,
            &session.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        let formatted = output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
        Ok(CallToolResult::success(vec![Content::text(formatted)]))
    }

    #[tool(
        description = "Activate a pipeline — makes it live so it can serve traffic and be executed. \
                       Must be called after register_pipeline or after patching. \
                       A pipeline must be active before execute_pipeline will run it."
    )]
    async fn activate_pipeline(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<ActivatePipelineParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "activate_pipeline")?;

        let dsl = format!("activate pipeline {}", params.name);
        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &session.owner,
            &session.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        let formatted = output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
        Ok(CallToolResult::success(vec![Content::text(formatted)]))
    }

    #[tool(
        description = "Deactivate a pipeline — takes it offline. Traffic stops being served. \
                       Pipeline source is retained and can be re-activated later."
    )]
    async fn deactivate_pipeline(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<DeactivatePipelineParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "deactivate_pipeline")?;

        let dsl = format!("deactivate pipeline {}", params.name);
        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &session.owner,
            &session.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        let formatted = output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
        Ok(CallToolResult::success(vec![Content::text(formatted)]))
    }

    #[tool(
        description = "Execute a registered active pipeline by name. Records execution hits. \
                       Pipeline must be activated first — use activate_pipeline if status is draft or stale. \
                       Use list_pipelines to see pipeline names and activation status."
    )]
    async fn execute_pipeline(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<ExecutePipelineParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "execute_pipeline")?;

        let mut dsl = format!("execute pipeline {}", params.name);
        if let Some(input) = &params.input {
            dsl.push_str(&format!(" --input {}", input));
        }

        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &session.owner,
            &session.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        let formatted = output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
        Ok(CallToolResult::success(vec![Content::text(formatted)]))
    }

    #[tool(
        description = "Run a pipe-chained node body EPHEMERALLY — not saved, not logged, no hit recording. \
                       Use this to test queries, explore data, or prototype before registering. \
                       Example body: '| pg.query --credential main-db -- \"SELECT count(*) FROM users\"'. \
                       Auto-prepends trigger.manual if no trigger node specified."
    )]
    async fn run_ephemeral(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<RunEphemeralParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "run_ephemeral")?;

        let dsl = format!("run {}", params.body);
        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &session.owner,
            &session.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        let formatted = output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
        Ok(CallToolResult::success(vec![Content::text(formatted)]))
    }

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

        let mut dsl = format!("git {}", params.subcommand);
        if let Some(args) = &params.args {
            dsl.push(' ');
            dsl.push_str(args);
        }
        if let Some(msg) = &params.message {
            dsl.push_str(&format!(" -- {}", msg));
        }

        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &session.owner,
            &session.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        let formatted = output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
        Ok(CallToolResult::success(vec![Content::text(formatted)]))
    }

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
            McpError::invalid_params(format!("Unknown tool '{}'", tool_name), None)
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
                    "Tool '{}' requires capability '{}' which is not allowed in this session",
                    tool_name,
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
        let instructions = crate::platform::skills::get_skill("agent-core")
            .map(|s| s.content.to_string())
            .unwrap_or_else(|| {
                "Zebflow project management tools. Use list_skills then read_skill to load \
                 operational knowledge docs before working with pipelines, templates, or data."
                    .to_string()
            });
        ServerInfo {
            instructions: Some(instructions.into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
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

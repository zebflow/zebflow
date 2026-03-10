//! Zebflow MCP handler exposing project-scoped tools.

use std::sync::Arc;

use axum::http;
use rmcp::handler::server::{ServerHandler, tool::Extension};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ServerCapabilities, ServerInfo};
use rmcp::schemars::JsonSchema;
use rmcp::{ErrorData as McpError, schemars, tool, tool_handler, tool_router};
use serde_json::json;

use crate::platform::model::{McpSession, ProjectAccessSubject, mcp_tool_capability};
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
struct ExecutePipelineDslParams {
    /// Pipeline DSL string. Supports: get, describe, register, patch, activate,
    /// deactivate, execute, run, git, delete. Use && to chain commands and \ for line
    /// continuations. See the pipeline-dsl skill for full reference.
    dsl: String,
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

    #[tool(description = "List all tables in the project")]
    async fn list_tables(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "list_tables")?;

        match self
            .platform
            .simple_tables
            .list_tables(&session.owner, &session.project)
        {
            Ok(tables) => {
                let content = json!({
                    "tables": tables,
                    "count": tables.len()
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

    #[tool(
        description = "Execute Pipeline DSL commands (get, describe, register, patch, activate, \
                       deactivate, execute, run, git). Returns terminal-style line output. \
                       Supports && chaining. See pipeline-dsl skill for full syntax reference."
    )]
    async fn execute_pipeline_dsl(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<ExecutePipelineDslParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "execute_pipeline_dsl")?;

        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &session.owner,
            &session.project,
        );
        let output = executor.execute_dsl(&params.dsl).await;
        let formatted = output
            .lines
            .iter()
            .map(|l| l.text.clone())
            .collect::<Vec<_>>()
            .join("\n");
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
        ServerInfo {
            instructions: Some(
                "Zebflow project management tools. Requires per-project session token.\n\n\
                 Available tools: list_pipelines, get_pipeline, list_templates, get_template, \
                 list_tables, list_project_docs, read_project_doc, list_skills, read_skill.\n\n\
                 To understand how to use Zebflow effectively, call list_skills first to see \
                 available knowledge docs, then read_skill to get detailed operational guidance \
                 on pipelines, templates, tables, and the API."
                    .into(),
            ),
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

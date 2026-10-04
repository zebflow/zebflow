//! One route's MCP server for one request: rmcp's streamable-HTTP server,
//! stateless, listing and running exactly the route's tools — no project
//! tool, resource, prompt or skill.

use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::http::{HeaderMap, Method, Uri};
use axum::response::Response;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler};
use serde_json::Value;

use super::{PlatformAppState, PublishedTool, run};
use crate::pipeline::nodes::basic::trigger::function;
use crate::platform::services::pipeline_runtime::McpTriggerSpec;

/// rmcp's streamable-HTTP server for this one request: stateless (no session
/// outlives the request, so none is shared), answering JSON.
pub(super) async fn serve(server: PublishedServer, method: Method, uri: Uri, headers: HeaderMap, body: Bytes) -> Response {
    use rmcp::transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    };
    let config = StreamableHttpServerConfig {
        sse_keep_alive: None,
        sse_retry: None,
        stateful_mode: false,
        json_response: true,
        cancellation_token: tokio_util::sync::CancellationToken::new(),
    };
    let service = StreamableHttpService::new(move || Ok(server.clone()), Arc::new(LocalSessionManager::default()), config);
    let mut request = axum::http::Request::new(Body::from(body));
    *request.method_mut() = method;
    *request.uri_mut() = uri;
    *request.headers_mut() = headers;
    service.handle(request).await.map(Body::new)
}

/// One route's server, for one request: what it lists and runs, and nothing
/// else — no project tool, resource, prompt or skill.
#[derive(Clone)]
pub(super) struct PublishedServer {
    pub(super) state: PlatformAppState,
    pub(super) owner: String,
    pub(super) project: String,
    pub(super) route: String,
    pub(super) tools: Arc<Vec<PublishedTool>>,
}

fn tool_of(spec: &McpTriggerSpec) -> Tool {
    let input_schema = spec.input_schema.as_object().cloned().unwrap_or_default();
    Tool {
        name: spec.tool_name.clone().into(),
        title: None,
        description: (!spec.tool_description.is_empty()).then(|| spec.tool_description.clone().into()),
        input_schema: Arc::new(input_schema),
        output_schema: None,
        annotations: None,
        execution: None,
        icons: None,
        meta: None,
    }
}

impl ServerHandler for PublishedServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: format!("zebflow:{}", self.route),
                title: None,
                version: env!("CARGO_PKG_VERSION").to_string(),
                description: None,
                icons: None,
                website_url: None,
            },
            instructions: None,
            ..Default::default()
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult { tools: self.tools.iter().map(|t| tool_of(&t.spec)).collect(), meta: None, next_cursor: None })
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tools.iter().find(|t| t.spec.tool_name == name).map(|t| tool_of(&t.spec))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let Some(tool) = self.tools.iter().find(|t| t.spec.tool_name == request.name) else {
            return Err(McpError::invalid_params(format!("Unknown tool '{}'", request.name), None));
        };
        let arguments = Value::Object(request.arguments.unwrap_or_default());
        // A wrong argument is the agent's to fix: a tool error it can read
        // and retry, before anything runs.
        if let Err(problem) = function::validate_function_input(&tool.spec.input_schema, &arguments) {
            return Ok(CallToolResult::error(vec![Content::text(problem.to_string())]));
        }
        Ok(run::run(self, tool, arguments).await)
    }
}

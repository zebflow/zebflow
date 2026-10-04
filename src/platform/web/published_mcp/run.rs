//! One `tools/call` of a published route: the tool's pipeline run with
//! `mcp: { route, tool_name, arguments }`, recorded as every run is, and its
//! answer as a tool result (`published-mcp.md`, "The tool result").

use std::sync::Arc;

use rmcp::model::{CallToolResult, Content};
use serde_json::{Value, json};

use super::PublishedTool;
use super::server::PublishedServer;
use crate::pipeline::model::{ExecuteOptions, PipelineError, PipelineOutput, Responder};
use crate::pipeline::{BasicPipelineEngine, PipelineContext, PipelineEngine};
use crate::platform::model::PipelineInvocationEntry;
use crate::platform::services::addressing::ErrorDetail;

/// Runs the tool's pipeline and answers what it answered.
pub(super) async fn run(server: &PublishedServer, tool: &PublishedTool, arguments: Value) -> CallToolResult {
    let state = &server.state;
    let (owner, project) = (server.owner.clone(), server.project.clone());
    let request_id = uuid::Uuid::new_v4().simple().to_string();
    let file_rel_path = tool.compiled.file_rel_path.clone();
    let project_cfg = state.platform.zebflow_cfg.read_or_default(&owner, &project).unwrap_or_default();
    let retention = crate::platform::model::resolve_invocation_retention(&project_cfg, Some(&tool.compiled.graph));
    let error_bounds = project_cfg.configs.pipelines.logging.error_group_bounds();

    let mut graph = tool.compiled.graph.clone();
    if let Err(err) = super::super::hydrate_template_markup(state, &owner, &project, &mut graph)
        .and_then(|_| super::super::apply_rwe_project_options(state, &owner, &project, &mut graph))
    {
        return tool_error(&json!({ "ok": false, "error": { "code": err.code, "message": err.message } }));
    }
    let envelope = json!({ "route": server.route, "tool_name": tool.spec.tool_name, "arguments": arguments });
    let ctx = PipelineContext {
        owner: owner.clone(),
        project: project.clone(),
        pipeline: graph.id.clone(),
        request_id: request_id.clone(),
        route: server.route.clone(),
        input: envelope.clone(),
        trigger: Some(envelope),
        placeholder: None,
    };
    let engine = BasicPipelineEngine::new(
        Arc::new(state.platform.project_sandbox(&owner, &project)),
        state.frontend.rwe.clone(),
        Some(state.platform.credentials.clone()),
    )
    .with_platform(state.platform.clone())
    .with_template_cache(state.template_cache.clone())
    .with_project_layout(state.platform.projects.project_layout(&owner, &project).ok())
    .with_ws_hub(state.platform.ws_hub.clone())
    .with_ws_client_manager(state.ws_client_manager.clone())
    .with_state_bus(state.platform.state_bus.clone())
    .with_data_root(state.platform.config.data_root.clone());

    // `web.response.send` answers at once and the run goes on to its end on
    // its own task, recorded there (`node-conventions.md` §4).
    let (responder, early_response) = Responder::channel();
    let options = ExecuteOptions { responder: Some(responder), ..Default::default() };
    let platform = state.platform.clone();
    let exec_start = std::time::Instant::now();
    let scope = (owner.clone(), project.clone(), file_rel_path, request_id.clone());
    let mut run_task = tokio::spawn(async move {
        let run = engine.execute_with_options_async(&graph, &ctx, &options).await;
        let (owner, project, file_rel_path, request_id) = &scope;
        crate::pipeline::nodes::shared::file_ref::remove_run_temporary_files(&platform, owner, project, request_id);
        let entry = PipelineInvocationEntry {
            run_id: request_id.clone(),
            at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64,
            duration_ms: exec_start.elapsed().as_millis() as u64,
            status: if run.is_ok() { "ok" } else { "error" }.to_string(),
            trigger: "mcp".to_string(),
            error: run.as_ref().err().map(|e| e.message.clone()),
            trace: match &run {
                Ok(output) => output.node_trace.clone(),
                Err(err) => err.node_trace.clone(),
            },
        };
        match &run {
            Ok(_) => platform.pipeline_hits.record_success(owner, project, file_rel_path),
            Err(err) => {
                platform.pipeline_hits.record_failure(owner, project, file_rel_path, "mcp.ingress", err.code, &err.message);
                if let Err(e) = platform.data.record_pipeline_error(owner, project, file_rel_path, &entry, error_bounds) {
                    eprintln!("warning: error group not recorded for {owner}/{project}: {}", e.message);
                }
            }
        }
        let _ = platform.data.log_pipeline_invocation(owner, project, file_rel_path, &entry, retention.max_invocations, retention.max_age_secs);
        run
    });
    let run = tokio::select! {
        biased;
        Ok(envelope) = early_response => return response_result(&envelope),
        joined = &mut run_task => joined.unwrap_or_else(|join_error| {
            Err(PipelineError::new("FW_ENGINE_RUNTIME", format!("the run's task ended early: {join_error}")))
        }),
    };
    match run {
        Ok(PipelineOutput { response: Some(envelope), .. }) => response_result(&envelope),
        // The 204 rule for a tool: no answering node, nothing answered.
        Ok(_) => empty_result(),
        Err(err) => failure_result(server, tool, &err, &request_id),
    }
}

/// An uncaught failure, as the route's `--errors` (else the project's
/// `errors` switch) allows it to be seen — the webhook's rule.
fn failure_result(server: &PublishedServer, tool: &PublishedTool, err: &PipelineError, request_id: &str) -> CallToolResult {
    let shown = match tool.spec.errors.as_str() {
        "show" => true,
        "hide" => false,
        _ => server
            .state
            .platform
            .addressing
            .read(&server.owner, &server.project)
            .map(|a| a.errors == ErrorDetail::Shown)
            .unwrap_or(false),
    };
    let body = if shown {
        let node_id = crate::platform::model::failing_trace_entry(&err.node_trace).map(|t| t.node_id.clone());
        json!({ "ok": false, "error": { "code": err.code, "message": err.message, "node_id": node_id, "request_id": request_id } })
    } else {
        json!({ "ok": false, "error": { "code": "internal", "request_id": request_id } })
    };
    tool_error(&body)
}

/// What `web.response.send` answered, as a tool result: its body, a tool
/// error when the status is 400 or more.
fn response_result(envelope: &Value) -> CallToolResult {
    let status = envelope.get("status").and_then(Value::as_u64).unwrap_or(200);
    let text = if let Some(json) = envelope.get("json") {
        json_text(json)
    } else if let Some(text) = envelope.get("text").and_then(Value::as_str) {
        text.to_string()
    } else if let Some(html) = envelope.get("html").and_then(Value::as_str) {
        html.to_string()
    } else if envelope.get("body_base64").is_some() {
        "(binary body — a tool answers text or JSON)".to_string()
    } else {
        String::new()
    };
    if status >= 400 {
        CallToolResult::error(vec![Content::text(text)])
    } else {
        CallToolResult::success(vec![Content::text(text)])
    }
}

/// A run that reached no `web.response.send`: an empty result,
/// `{ content: [], isError: false }` — the run's value is never sent on its
/// own (`published-mcp.md`, the webhook's 204 rule).
fn empty_result() -> CallToolResult {
    CallToolResult::success(Vec::new())
}

fn tool_error(body: &Value) -> CallToolResult {
    CallToolResult::error(vec![Content::text(json_text(body))])
}

fn json_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::{empty_result, response_result};
    use serde_json::json;

    #[test]
    fn a_response_is_the_result_and_a_4xx_is_a_tool_error() {
        let ok = response_result(&json!({ "status": 200, "json": { "rows": [1] } }));
        assert_eq!(ok.is_error, Some(false));
        assert_eq!(ok.content[0].as_text().map(|t| t.text.as_str()), Some(r#"{"rows":[1]}"#));
        let refused = response_result(&json!({ "status": 404, "json": { "error": "no such sku" } }));
        assert_eq!(refused.is_error, Some(true));
        assert_eq!(refused.content[0].as_text().map(|t| t.text.as_str()), Some(r#"{"error":"no such sku"}"#));
        let text = response_result(&json!({ "text": "hello" }));
        assert_eq!(text.content[0].as_text().map(|t| t.text.as_str()), Some("hello"));
    }

    #[test]
    fn no_answering_node_is_an_empty_result() {
        let out = serde_json::to_value(empty_result()).expect("serialises");
        assert_eq!(out, json!({ "content": [], "isError": false }));
    }
}

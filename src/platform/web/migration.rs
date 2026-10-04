//! The 0.11 migration over HTTP, for a project's owner (or a superadmin):
//!
//! - `GET  /api/projects/{owner}/{project}/migration/0.11/plan` — the plan:
//!   JSON, or its report alone with `?format=markdown`. Reads only.
//! - `POST /api/projects/{owner}/{project}/migration/0.11/apply` with
//!   `{ "fingerprint": "<the plan's>" }` — applies that plan; refused when
//!   the project changed since, or the plan is not ready.

use async_trait::async_trait;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, Method, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::body::Bytes;
use serde::Deserialize;
use serde_json::json;

use std::sync::Arc;

use super::{PlatformAppState, internal_error, maybe_forward_project_api_to_worker, require_project_api_capability, run_lifecycle_hooks};
use crate::infra::mem::subscriber::KvSubscriber;
use crate::infra::scheduler::PipelineScheduler;
use crate::infra::ws_client::WsClientManager;
use crate::platform::services::PlatformService;
use crate::platform::error::PlatformError;
use crate::platform::model::ProjectCapability;
use crate::platform::services::migration::{MigrationService, PipelineSwitch};

/// Activation as the pipeline API does it: the runtime registry, the
/// schedules, subscriptions and socket clients, and the lifecycle hooks.
/// Attached to the project service when the router is built, so every door
/// that switches pipelines (HTTP, MCP) switches them the same way.
pub struct WebSwitch {
    pub platform: Arc<PlatformService>,
    pub scheduler: Arc<PipelineScheduler>,
    pub kv_subscriber: Arc<KvSubscriber>,
    pub ws_client_manager: Arc<WsClientManager>,
}

#[async_trait]
impl PipelineSwitch for WebSwitch {
    async fn activate(&self, owner: &str, project: &str, file_rel_path: &str) -> Result<(), PlatformError> {
        self.platform.projects.activate_pipeline_definition(owner, project, file_rel_path)?;
        self.platform.pipeline_runtime.refresh_pipeline(owner, project, file_rel_path)?;
        self.scheduler.sync_pipeline(owner, project, file_rel_path).await;
        self.kv_subscriber.sync_pipeline(owner, project, file_rel_path).await;
        self.ws_client_manager.sync_pipeline(owner, project, file_rel_path).await;
        run_lifecycle_hooks(&self.platform, owner, project, file_rel_path, "on_activate").await;
        Ok(())
    }

    async fn deactivate(&self, owner: &str, project: &str, file_rel_path: &str) -> Result<(), PlatformError> {
        self.platform.projects.deactivate_pipeline_definition(owner, project, file_rel_path)?;
        run_lifecycle_hooks(&self.platform, owner, project, file_rel_path, "on_deactivate").await;
        self.platform.pipeline_runtime.evict(owner, project, file_rel_path);
        self.scheduler.sync_pipeline(owner, project, file_rel_path).await;
        self.kv_subscriber.sync_pipeline(owner, project, file_rel_path).await;
        self.ws_client_manager.sync_pipeline(owner, project, file_rel_path).await;
        Ok(())
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct PlanQuery {
    #[serde(default)]
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApplyRequest {
    pub fingerprint: String,
}

/// Owner-only: the capability only a project's owner (and a superadmin)
/// holds.
const CAPABILITY: ProjectCapability = ProjectCapability::ProjectDelete;

pub async fn api_migration_plan(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Query(query): Query<PlanQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(&state, &headers, &owner, &project, CAPABILITY) {
        return response;
    }
    match maybe_forward_project_api_to_worker(&state, &uri, &Method::GET, &headers, Bytes::new(), &owner, &project).await {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let service = MigrationService::new(state.platform.clone());
    match service.plan_project(&owner, &project) {
        Ok(plan) if query.format.as_deref() == Some("markdown") => {
            ([(header::CONTENT_TYPE, "text/markdown; charset=utf-8")], plan.report).into_response()
        }
        Ok(plan) => Json(json!({ "ok": true, "plan": plan })).into_response(),
        Err(err) => internal_error(err),
    }
}

pub async fn api_migration_apply(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(request): Json<ApplyRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(&state, &headers, &owner, &project, CAPABILITY) {
        return response;
    }
    let body = Bytes::from(serde_json::to_vec(&json!({ "fingerprint": request.fingerprint })).unwrap_or_default());
    match maybe_forward_project_api_to_worker(&state, &uri, &Method::POST, &headers, body, &owner, &project).await {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let service = MigrationService::new(state.platform.clone());
    match service.apply_plan(&owner, &project, &request.fingerprint).await {
        Ok(report) => {
            let status = if report.ok { StatusCode::OK } else { StatusCode::MULTI_STATUS };
            (status, Json(json!({ "ok": report.ok, "report": report }))).into_response()
        }
        Err(err) if err.code == "PLATFORM_MIGRATION_STALE" || err.code == "PLATFORM_MIGRATION_BLOCKED" => (
            StatusCode::CONFLICT,
            Json(json!({ "ok": false, "error": { "code": err.code, "message": err.message } })),
        )
            .into_response(),
        Err(err) => internal_error(err),
    }
}

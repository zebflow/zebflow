//! Project placement orchestration service.
//!
//! This service owns environment-owned runtime placement records. The repo may declare that a
//! project wants `shared`, `pinned`, or `dedicated` runtime behavior, but the actual worker
//! assignment remains platform-controlled and belongs here.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::infra::execution::placement::{
    PlacementPolicy, PlacementPolicyKind, ProjectRuntimeMode, ProjectRuntimePlacement,
    ProjectRuntimePlacementTarget, ProjectRuntimeProfile,
};
use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{ProjectRuntimeSelectionRequest, now_ts, slug_segment};

/// Product-facing placement policy helper.
#[derive(Clone)]
pub struct ClusterPlacementService {
    data: Arc<dyn DataAdapter>,
    /// Every webhook asks where its project runs. The answer is read from
    /// the catalog at most once per `STAMP_TRUST` per project, not on each
    /// request through the catalog's one connection; a placement written
    /// here drops its entry at once.
    recent: Arc<RwLock<HashMap<String, (Instant, Option<ProjectRuntimePlacement>)>>>,
}

impl ClusterPlacementService {
    /// Create a new placement service backed by the platform catalog adapter.
    pub fn new(data: Arc<dyn DataAdapter>) -> Self {
        Self { data, recent: Arc::default() }
    }

    /// Default placement for a fresh standalone-style project.
    pub fn default_policy(&self) -> PlacementPolicy {
        PlacementPolicy::local()
    }

    /// Return the persisted placement record for one project, if any.
    pub fn get(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Option<ProjectRuntimePlacement>, PlatformError> {
        let key = format!("{owner}/{project}");
        let trust = crate::platform::services::project_config::STAMP_TRUST;
        if let Some((read, placement)) = self.recent.read().unwrap_or_else(|e| e.into_inner()).get(&key)
            && read.elapsed() < trust
        {
            return Ok(placement.clone());
        }
        let placement = self.data.get_project_runtime_placement(owner, project)?;
        self.recent.write().unwrap_or_else(|e| e.into_inner()).insert(key, (Instant::now(), placement.clone()));
        Ok(placement)
    }

    /// Resolve and persist placement for a freshly created or cloned project.
    pub fn assign_for_project(
        &self,
        owner: &str,
        project: &str,
        runtime_profile: &ProjectRuntimeProfile,
        selection: &ProjectRuntimeSelectionRequest,
    ) -> Result<ProjectRuntimePlacement, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let project_row = self
            .data
            .get_project(&owner, &project)?
            .ok_or_else(|| PlatformError::new("PROJECT_NOT_FOUND", "project not found"))?;
        let worker_id = selection
            .placement_worker_id
            .as_deref()
            .map(slug_segment)
            .filter(|value| !value.is_empty() && value != "local");
        let target_worker = if let Some(ref worker_id) = worker_id {
            Some(
                self.data
                    .get_worker_registry_record(worker_id)?
                    .ok_or_else(|| {
                        PlatformError::new(
                            "CLUSTER_WORKER_UNKNOWN",
                            format!("worker '{}' is not registered", worker_id),
                        )
                    })?,
            )
        } else {
            None
        };
        let now = now_ts();
        let existing = self.data.get_project_runtime_placement(&owner, &project)?;
        let placement = ProjectRuntimePlacement {
            project_id: project_row.project_id,
            owner,
            project,
            mode: selection.runtime_mode.unwrap_or(runtime_profile.mode),
            target: if worker_id.is_some() {
                ProjectRuntimePlacementTarget::Worker
            } else {
                ProjectRuntimePlacementTarget::Local
            },
            target_office_id: target_worker.as_ref().map(|worker| {
                if worker.office_id.trim().is_empty() {
                    worker.node_id.clone()
                } else {
                    worker.office_id.clone()
                }
            }),
            target_node_id: worker_id.clone(),
            worker_id,
            resource_profile: serde_json::to_value(runtime_profile.resource_profile)
                .ok()
                .and_then(|value| value.as_str().map(ToString::to_string))
                .unwrap_or_else(|| "small".to_string()),
            desired_replicas: runtime_profile.min_replicas,
            effective_state: if target_worker.is_some() {
                "assigned".to_string()
            } else {
                "local".to_string()
            },
            created_at: existing
                .as_ref()
                .map(|value| value.created_at)
                .unwrap_or(now),
            updated_at: now,
        };
        self.data.put_project_runtime_placement(&placement)?;
        self.recent.write().unwrap_or_else(|e| e.into_inner()).remove(&format!("{}/{}", placement.owner, placement.project));
        Ok(placement)
    }

    /// Translate a persisted placement record into a runtime dispatch policy.
    pub fn dispatch_policy(&self, placement: Option<&ProjectRuntimePlacement>) -> PlacementPolicy {
        match placement {
            Some(record) if record.target == ProjectRuntimePlacementTarget::Worker => {
                PlacementPolicy {
                    kind: PlacementPolicyKind::Pinned,
                    pinned_runner_id: record.worker_id.clone(),
                    required_tags: Vec::new(),
                }
            }
            _ => PlacementPolicy::local(),
        }
    }

    /// Convenience helper for callers that only care whether a project is local or remote.
    pub fn is_remote(placement: Option<&ProjectRuntimePlacement>) -> bool {
        matches!(
            placement,
            Some(ProjectRuntimePlacement {
                target: ProjectRuntimePlacementTarget::Worker,
                ..
            })
        )
    }

    /// Resolve a human-readable placement summary for UI surfaces.
    pub fn describe(&self, placement: Option<&ProjectRuntimePlacement>) -> String {
        match placement {
            Some(ProjectRuntimePlacement {
                mode: ProjectRuntimeMode::Dedicated,
                worker_id: Some(worker_id),
                ..
            }) => format!("Dedicated on {worker_id}"),
            Some(ProjectRuntimePlacement {
                mode: ProjectRuntimeMode::Pinned,
                worker_id: Some(worker_id),
                ..
            }) => format!("Pinned to {worker_id}"),
            Some(ProjectRuntimePlacement {
                target: ProjectRuntimePlacementTarget::Worker,
                worker_id: Some(worker_id),
                ..
            }) => format!("Remote on {worker_id}"),
            Some(_) => "Local".to_string(),
            None => "Local".to_string(),
        }
    }
}

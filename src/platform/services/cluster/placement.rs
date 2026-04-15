//! Project placement orchestration service.
//!
//! This service owns environment-owned runtime placement records. The repo may declare that a
//! project wants `shared`, `pinned`, or `dedicated` runtime behavior, but the actual worker
//! assignment remains platform-controlled and belongs here.

use std::sync::Arc;

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
}

impl ClusterPlacementService {
    /// Create a new placement service backed by the platform catalog adapter.
    pub fn new(data: Arc<dyn DataAdapter>) -> Self {
        Self { data }
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
        self.data.get_project_runtime_placement(owner, project)
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
        let worker_id = selection
            .placement_worker_id
            .as_deref()
            .map(slug_segment)
            .filter(|value| !value.is_empty() && value != "local");
        if let Some(ref worker_id) = worker_id {
            if self.data.get_worker_registry_record(worker_id)?.is_none() {
                return Err(PlatformError::new(
                    "CLUSTER_WORKER_UNKNOWN",
                    format!("worker '{}' is not registered", worker_id),
                ));
            }
        }
        let now = now_ts();
        let existing = self.data.get_project_runtime_placement(&owner, &project)?;
        let placement = ProjectRuntimePlacement {
            owner,
            project,
            mode: selection.runtime_mode.unwrap_or(runtime_profile.mode),
            target: if worker_id.is_some() {
                ProjectRuntimePlacementTarget::Worker
            } else {
                ProjectRuntimePlacementTarget::Local
            },
            worker_id,
            created_at: existing
                .as_ref()
                .map(|value| value.created_at)
                .unwrap_or(now),
            updated_at: now,
        };
        self.data.put_project_runtime_placement(&placement)?;
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

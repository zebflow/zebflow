//! Durable controller-side operation tracking for project portability flows.
//!
//! This service stores export/import progress in the platform catalog so the controller can show
//! incomplete work, surface failures, and keep artifact metadata attached to a stable operation id.

use std::sync::Arc;

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    ProjectOperationKind, ProjectOperationRecord, ProjectOperationStatus, now_ts, slug_segment,
};

/// Controller-side project operation persistence helper.
pub struct ProjectOperationService {
    data: Arc<dyn DataAdapter>,
}

impl ProjectOperationService {
    /// Create a new operation service backed by the platform catalog adapter.
    pub fn new(data: Arc<dyn DataAdapter>) -> Self {
        Self { data }
    }

    /// Create and persist a fresh pending operation record.
    pub fn create(
        &self,
        owner: &str,
        project: &str,
        kind: ProjectOperationKind,
        source_office_id: Option<String>,
        target_office_id: Option<String>,
    ) -> Result<ProjectOperationRecord, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let project_row = self
            .data
            .get_project(&owner, &project)?
            .ok_or_else(|| PlatformError::new("PROJECT_NOT_FOUND", "project not found"))?;
        let now = now_ts();
        let operation = ProjectOperationRecord {
            operation_id: format!("op-{}-{now}", kind.key()),
            project_id: project_row.project_id,
            owner,
            project,
            kind,
            status: ProjectOperationStatus::Pending,
            current_step: "pending".to_string(),
            source_office_id,
            target_office_id,
            artifact_rel_path: None,
            artifact_sha256: None,
            artifact_bytes: None,
            error_message: None,
            retry_count: 0,
            created_at: now,
            updated_at: now,
            completed_at: None,
        };
        self.data.put_project_operation(&operation)?;
        Ok(operation)
    }

    /// Persist the current running step for an operation.
    pub fn mark_running(
        &self,
        record: &ProjectOperationRecord,
        step: impl Into<String>,
    ) -> Result<ProjectOperationRecord, PlatformError> {
        let mut next = record.clone();
        next.status = ProjectOperationStatus::Running;
        next.current_step = step.into();
        next.error_message = None;
        next.updated_at = now_ts();
        self.data.put_project_operation(&next)?;
        Ok(next)
    }

    /// Persist a terminal failure state and keep the failed step visible.
    pub fn mark_failed(
        &self,
        record: &ProjectOperationRecord,
        step: impl Into<String>,
        error_message: impl Into<String>,
    ) -> Result<ProjectOperationRecord, PlatformError> {
        let mut next = record.clone();
        next.status = ProjectOperationStatus::Failed;
        next.current_step = step.into();
        next.error_message = Some(error_message.into());
        next.updated_at = now_ts();
        next.completed_at = None;
        self.data.put_project_operation(&next)?;
        Ok(next)
    }

    /// Persist a completed state and attach the generated artifact metadata when available.
    pub fn mark_completed(
        &self,
        record: &ProjectOperationRecord,
        step: impl Into<String>,
        artifact_rel_path: Option<String>,
        artifact_sha256: Option<String>,
        artifact_bytes: Option<u64>,
    ) -> Result<ProjectOperationRecord, PlatformError> {
        let now = now_ts();
        let mut next = record.clone();
        next.status = ProjectOperationStatus::Completed;
        next.current_step = step.into();
        next.artifact_rel_path = artifact_rel_path;
        next.artifact_sha256 = artifact_sha256;
        next.artifact_bytes = artifact_bytes;
        next.error_message = None;
        next.updated_at = now;
        next.completed_at = Some(now);
        self.data.put_project_operation(&next)?;
        Ok(next)
    }

    /// Read one operation by id.
    pub fn get(
        &self,
        owner: &str,
        project: &str,
        operation_id: &str,
    ) -> Result<Option<ProjectOperationRecord>, PlatformError> {
        self.data
            .get_project_operation(owner, project, operation_id)
    }

    /// List the most recent operations for one project.
    pub fn list(
        &self,
        owner: &str,
        project: &str,
        limit: usize,
    ) -> Result<Vec<ProjectOperationRecord>, PlatformError> {
        self.data.list_project_operations(owner, project, limit)
    }
}

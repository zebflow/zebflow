//! Swappable metadata adapters for Zebflow platform.

mod dynamodb;
mod firebase;
mod sqlite;

use std::path::Path;
use std::sync::Arc;

use crate::infra::cluster::registry::WorkerRegistryRecord;
use crate::infra::execution::placement::ProjectRuntimePlacement;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    DataAdapterKind, McpSession, PipelineInvocationEntry, PipelineMeta, PlatformProject,
    PlatformUser, ProjectCredential, ProjectDbConnection, ProjectInvite, ProjectMember,
    ProjectOperationRecord, ProjectPolicy, ProjectPolicyBinding, StoredUser,
};

/// Metadata adapter contract used by platform services.
pub trait DataAdapter: Send + Sync {
    /// Stable adapter id.
    fn id(&self) -> &'static str;
    /// Load a user auth record by owner id.
    fn get_user_auth(&self, owner: &str) -> Result<Option<StoredUser>, PlatformError>;
    /// Upsert one user auth record.
    fn put_user(&self, user: &StoredUser) -> Result<(), PlatformError>;
    /// List users.
    fn list_users(&self) -> Result<Vec<PlatformUser>, PlatformError>;
    /// Get one project.
    fn get_project(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Option<PlatformProject>, PlatformError>;
    /// Upsert one project.
    fn put_project(&self, project: &PlatformProject) -> Result<(), PlatformError>;
    /// List projects by owner.
    fn list_projects(&self, owner: &str) -> Result<Vec<PlatformProject>, PlatformError>;
    /// Delete one project metadata record.
    fn delete_project(&self, owner: &str, project: &str) -> Result<(), PlatformError>;
    /// Load one project credential.
    fn get_project_credential(
        &self,
        owner: &str,
        project: &str,
        credential_id: &str,
    ) -> Result<Option<ProjectCredential>, PlatformError>;
    /// Upsert one project credential.
    fn put_project_credential(&self, credential: &ProjectCredential) -> Result<(), PlatformError>;
    /// List project credentials.
    fn list_project_credentials(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectCredential>, PlatformError>;
    /// Delete one project credential.
    fn delete_project_credential(
        &self,
        owner: &str,
        project: &str,
        credential_id: &str,
    ) -> Result<(), PlatformError>;
    /// Load one project DB connection.
    fn get_project_db_connection(
        &self,
        owner: &str,
        project: &str,
        connection_slug: &str,
    ) -> Result<Option<ProjectDbConnection>, PlatformError>;
    /// Upsert one project DB connection.
    fn put_project_db_connection(
        &self,
        connection: &ProjectDbConnection,
    ) -> Result<(), PlatformError>;
    /// List project DB connections.
    fn list_project_db_connections(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectDbConnection>, PlatformError>;
    /// Delete one project DB connection.
    fn delete_project_db_connection(
        &self,
        owner: &str,
        project: &str,
        connection_slug: &str,
    ) -> Result<(), PlatformError>;
    /// Upsert one pipeline metadata row.
    fn put_pipeline_meta(&self, meta: &PipelineMeta) -> Result<(), PlatformError>;
    /// Delete one pipeline metadata row by owner/project/file_rel_path.
    fn delete_pipeline_meta(
        &self,
        owner: &str,
        project: &str,
        file_rel_path: &str,
    ) -> Result<(), PlatformError>;
    /// List pipeline metadata rows by owner/project.
    fn list_pipeline_meta(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<PipelineMeta>, PlatformError>;
    /// Upsert one project policy.
    fn put_project_policy(&self, policy: &ProjectPolicy) -> Result<(), PlatformError>;
    /// List project policies by owner/project.
    fn list_project_policies(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectPolicy>, PlatformError>;
    /// Upsert one project policy binding.
    fn put_project_policy_binding(
        &self,
        binding: &ProjectPolicyBinding,
    ) -> Result<(), PlatformError>;
    /// List project policy bindings by owner/project.
    fn list_project_policy_bindings(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectPolicyBinding>, PlatformError>;
    /// Delete one project policy.
    fn delete_project_policy(
        &self,
        owner: &str,
        project: &str,
        policy_id: &str,
    ) -> Result<(), PlatformError>;
    /// Delete one project policy binding.
    fn delete_project_policy_binding(
        &self,
        owner: &str,
        project: &str,
        subject_id: &str,
    ) -> Result<(), PlatformError>;
    /// Get one explicit project member row.
    fn get_project_member(
        &self,
        owner: &str,
        project: &str,
        user_id: &str,
    ) -> Result<Option<ProjectMember>, PlatformError>;
    /// Upsert one project member row.
    fn put_project_member(&self, member: &ProjectMember) -> Result<(), PlatformError>;
    /// List explicit project member rows.
    fn list_project_members(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectMember>, PlatformError>;
    /// Delete one explicit project member row.
    fn delete_project_member(
        &self,
        owner: &str,
        project: &str,
        user_id: &str,
    ) -> Result<(), PlatformError>;
    /// Get one stored project invite.
    fn get_project_invite(
        &self,
        owner: &str,
        project: &str,
        invite_id: &str,
    ) -> Result<Option<ProjectInvite>, PlatformError>;
    /// Upsert one project invite.
    fn put_project_invite(&self, invite: &ProjectInvite) -> Result<(), PlatformError>;
    /// List project invites.
    fn list_project_invites(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectInvite>, PlatformError>;
    /// Delete one project invite.
    fn delete_project_invite(
        &self,
        owner: &str,
        project: &str,
        invite_id: &str,
    ) -> Result<(), PlatformError>;
    /// Get one registered worker record.
    fn get_worker_registry_record(
        &self,
        node_id: &str,
    ) -> Result<Option<WorkerRegistryRecord>, PlatformError>;
    /// Upsert one registered worker record.
    fn put_worker_registry_record(
        &self,
        record: &WorkerRegistryRecord,
    ) -> Result<(), PlatformError>;
    /// List registered worker records.
    fn list_worker_registry_records(&self) -> Result<Vec<WorkerRegistryRecord>, PlatformError>;
    /// Delete one registered worker record.
    fn delete_worker_registry_record(&self, node_id: &str) -> Result<(), PlatformError>;
    /// Get one environment-owned project placement record.
    fn get_project_runtime_placement(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Option<ProjectRuntimePlacement>, PlatformError>;
    /// Upsert one environment-owned project placement record.
    fn put_project_runtime_placement(
        &self,
        placement: &ProjectRuntimePlacement,
    ) -> Result<(), PlatformError>;
    /// List all environment-owned project placement records.
    fn list_project_runtime_placements(
        &self,
    ) -> Result<Vec<ProjectRuntimePlacement>, PlatformError>;
    /// Delete one environment-owned project placement record.
    fn delete_project_runtime_placement(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<(), PlatformError>;
    /// Get one durable controller-side project operation record.
    fn get_project_operation(
        &self,
        owner: &str,
        project: &str,
        operation_id: &str,
    ) -> Result<Option<ProjectOperationRecord>, PlatformError> {
        let _ = (owner, project, operation_id);
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "project operations are not supported by this adapter",
        ))
    }
    /// Upsert one durable controller-side project operation record.
    fn put_project_operation(&self, record: &ProjectOperationRecord) -> Result<(), PlatformError> {
        let _ = record;
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "project operations are not supported by this adapter",
        ))
    }
    /// List durable controller-side project operation records for one project.
    fn list_project_operations(
        &self,
        owner: &str,
        project: &str,
        limit: usize,
    ) -> Result<Vec<ProjectOperationRecord>, PlatformError> {
        let _ = (owner, project, limit);
        Ok(vec![])
    }
    /// List all persisted MCP sessions.
    fn list_all_mcp_sessions(&self) -> Result<Vec<McpSession>, PlatformError>;
    /// Persist one MCP session.
    fn put_mcp_session(&self, session: &McpSession) -> Result<(), PlatformError>;
    /// Delete a persisted MCP session by token.
    fn delete_mcp_session(&self, token: &str) -> Result<(), PlatformError>;
    /// Admin: list collection names with counts. Default impl returns unsupported error.
    fn admin_list_collections(&self) -> Result<Vec<(String, usize)>, PlatformError> {
        Err(PlatformError::new(
            "ADMIN_DB_UNAVAILABLE",
            "Admin DB access not supported by this adapter",
        ))
    }
    /// Admin: run a raw query pipeline JSON. Default impl returns unsupported error.
    fn admin_raw_query(
        &self,
        pipeline_json: &str,
    ) -> Result<Vec<serde_json::Value>, PlatformError> {
        let _ = pipeline_json;
        Err(PlatformError::new(
            "ADMIN_DB_UNAVAILABLE",
            "Admin DB access not supported by this adapter",
        ))
    }
    /// Admin: get a raw node by slug. Default impl returns unsupported error.
    fn admin_get_node(&self, slug: &str) -> Result<Option<String>, PlatformError> {
        let _ = slug;
        Err(PlatformError::new(
            "ADMIN_DB_UNAVAILABLE",
            "Admin DB access not supported by this adapter",
        ))
    }
    /// Admin: delete a raw node by slug. Returns true if the node existed.
    fn admin_delete_node(&self, slug: &str) -> Result<bool, PlatformError> {
        let _ = slug;
        Err(PlatformError::new(
            "ADMIN_DB_UNAVAILABLE",
            "Admin DB access not supported by this adapter",
        ))
    }

    /// Record a pipeline invocation, retaining at most `max_n` entries.
    /// Default implementation is a no-op (logging not supported).
    fn log_pipeline_invocation(
        &self,
        _owner: &str,
        _project: &str,
        _file_rel_path: &str,
        _entry: &PipelineInvocationEntry,
        _max_n: usize,
    ) -> Result<(), PlatformError> {
        Ok(())
    }

    /// Return stored invocation log for a pipeline (most-recent first).
    /// Default implementation returns an empty list.
    fn get_pipeline_invocations(
        &self,
        _owner: &str,
        _project: &str,
        _file_rel_path: &str,
    ) -> Result<Vec<PipelineInvocationEntry>, PlatformError> {
        Ok(vec![])
    }
}

/// Builds selected metadata adapter.
pub fn build_data_adapter(
    kind: DataAdapterKind,
    data_root: &Path,
) -> Result<Arc<dyn DataAdapter>, PlatformError> {
    match kind {
        DataAdapterKind::Sqlite => Ok(Arc::new(sqlite::SqliteDataAdapter::new(data_root)?)),
        DataAdapterKind::DynamoDb => Ok(Arc::new(dynamodb::DynamoDbDataAdapter::default())),
        DataAdapterKind::Firebase => Ok(Arc::new(firebase::FirebaseDataAdapter::default())),
    }
}

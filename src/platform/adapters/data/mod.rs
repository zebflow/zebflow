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
    DataAdapterKind, HubAssetPackage, HubAssetVersion, HubAuthority, HubPublisher, HubToken,
    McpSession, PipelineInvocationEntry, PipelineInvocationLogStats, PipelineMeta,
    PlatformHubRepository, PlatformOffice, PlatformOfficeNode, PlatformProject,
    PlatformServiceInstance, PlatformUser, ProjectCredential, ProjectDbConnection,
    ProjectHubRepository, ProjectInvite, ProjectMember, ProjectOperationRecord, ProjectPolicy,
    ProjectPolicyBinding, StoredUser,
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
    /// Upsert one hub repository source.
    fn put_project_hub_repository(
        &self,
        repository: &ProjectHubRepository,
    ) -> Result<(), PlatformError> {
        let _ = repository;
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "hub repositories are not supported by this adapter",
        ))
    }
    /// List hub repository sources for one project.
    fn list_project_hub_repositories(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectHubRepository>, PlatformError> {
        let _ = (owner, project);
        Ok(vec![])
    }
    /// Delete one hub repository source.
    fn delete_project_hub_repository(
        &self,
        owner: &str,
        project: &str,
        repository_id: &str,
    ) -> Result<(), PlatformError> {
        let _ = (owner, project, repository_id);
        Ok(())
    }
    /// Upsert one platform-scoped hub repository source.
    fn put_platform_hub_repository(
        &self,
        repository: &PlatformHubRepository,
    ) -> Result<(), PlatformError> {
        let _ = repository;
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "platform hub repositories are not supported by this adapter",
        ))
    }
    /// List platform hub repository sources for one owner.
    fn list_platform_hub_repositories(
        &self,
        owner: &str,
    ) -> Result<Vec<PlatformHubRepository>, PlatformError> {
        let _ = owner;
        Ok(vec![])
    }
    /// Delete one platform hub repository source.
    fn delete_platform_hub_repository(
        &self,
        owner: &str,
        repository_id: &str,
    ) -> Result<(), PlatformError> {
        let _ = (owner, repository_id);
        Ok(())
    }
    /// Upsert one hub publisher identity.
    fn put_hub_publisher(&self, publisher: &HubPublisher) -> Result<(), PlatformError> {
        let _ = publisher;
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "hub publishers are not supported by this adapter",
        ))
    }
    /// List hub publishers for one project authority.
    fn list_hub_publishers(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<HubPublisher>, PlatformError> {
        let _ = (owner, project);
        Ok(vec![])
    }
    /// Get one hub publisher.
    fn get_hub_publisher(
        &self,
        owner: &str,
        project: &str,
        publisher_id: &str,
    ) -> Result<Option<HubPublisher>, PlatformError> {
        let _ = (owner, project, publisher_id);
        Ok(None)
    }
    /// Delete one hub publisher.
    fn delete_hub_publisher(
        &self,
        owner: &str,
        project: &str,
        publisher_id: &str,
    ) -> Result<(), PlatformError> {
        let _ = (owner, project, publisher_id);
        Ok(())
    }
    /// Upsert one hub asset package.
    fn put_hub_asset_package(&self, package: &HubAssetPackage) -> Result<(), PlatformError> {
        let _ = package;
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "hub packages are not supported by this adapter",
        ))
    }
    /// List hub asset packages.
    fn list_hub_asset_packages(&self) -> Result<Vec<HubAssetPackage>, PlatformError> {
        Ok(vec![])
    }
    /// Get one hub asset package.
    fn get_hub_asset_package(
        &self,
        package_id: &str,
    ) -> Result<Option<HubAssetPackage>, PlatformError> {
        let _ = package_id;
        Ok(None)
    }
    /// Upsert one hub asset version.
    fn put_hub_asset_version(&self, version: &HubAssetVersion) -> Result<(), PlatformError> {
        let _ = version;
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "hub package versions are not supported by this adapter",
        ))
    }
    /// List versions for one package.
    fn list_hub_asset_versions(
        &self,
        package_id: &str,
    ) -> Result<Vec<HubAssetVersion>, PlatformError> {
        let _ = package_id;
        Ok(vec![])
    }
    /// Get one specific hub asset version.
    fn get_hub_asset_version(
        &self,
        package_id: &str,
        version: &str,
    ) -> Result<Option<HubAssetVersion>, PlatformError> {
        let _ = (package_id, version);
        Ok(None)
    }
    /// Upsert one hub token.
    fn put_hub_token(&self, token: &HubToken) -> Result<(), PlatformError> {
        let _ = token;
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "hub tokens are not supported by this adapter",
        ))
    }
    /// Get one hub token by id.
    fn get_hub_token(&self, token_id: &str) -> Result<Option<HubToken>, PlatformError> {
        let _ = token_id;
        Ok(None)
    }
    /// List hub tokens by owner/project authority.
    fn list_hub_tokens(&self, owner: &str, project: &str) -> Result<Vec<HubToken>, PlatformError> {
        let _ = (owner, project);
        Ok(vec![])
    }
    /// List all hub tokens in this hub catalog.
    fn list_all_hub_tokens(&self) -> Result<Vec<HubToken>, PlatformError> {
        Ok(vec![])
    }
    /// Delete one hub token.
    fn delete_hub_token(&self, token_id: &str) -> Result<(), PlatformError> {
        let _ = token_id;
        Ok(())
    }
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
    /// Get one explicit hub authority row for a host project.
    fn get_hub_authority(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Option<HubAuthority>, PlatformError> {
        let _ = (owner, project);
        Ok(None)
    }
    /// Upsert one explicit hub authority row.
    fn put_hub_authority(&self, authority: &HubAuthority) -> Result<(), PlatformError> {
        let _ = authority;
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "hub authorities are not supported by this adapter",
        ))
    }
    /// List hub authorities.
    fn list_hub_authorities(&self) -> Result<Vec<HubAuthority>, PlatformError> {
        Ok(vec![])
    }
    /// Get one office row.
    fn get_platform_office(
        &self,
        office_id: &str,
    ) -> Result<Option<PlatformOffice>, PlatformError> {
        let _ = office_id;
        Ok(None)
    }
    /// Upsert one office row.
    fn put_platform_office(&self, office: &PlatformOffice) -> Result<(), PlatformError> {
        let _ = office;
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "platform offices are not supported by this adapter",
        ))
    }
    /// List offices.
    fn list_platform_offices(&self) -> Result<Vec<PlatformOffice>, PlatformError> {
        Ok(vec![])
    }
    /// Get one platform service instance.
    fn get_platform_service_instance(
        &self,
        service_instance_id: &str,
    ) -> Result<Option<PlatformServiceInstance>, PlatformError> {
        let _ = service_instance_id;
        Ok(None)
    }
    /// Upsert one platform service instance.
    fn put_platform_service_instance(
        &self,
        service: &PlatformServiceInstance,
    ) -> Result<(), PlatformError> {
        let _ = service;
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "platform service instances are not supported by this adapter",
        ))
    }
    /// List platform service instances.
    fn list_platform_service_instances(
        &self,
    ) -> Result<Vec<PlatformServiceInstance>, PlatformError> {
        Ok(vec![])
    }
    /// Get one office node row.
    fn get_platform_office_node(
        &self,
        node_id: &str,
    ) -> Result<Option<PlatformOfficeNode>, PlatformError> {
        let _ = node_id;
        Ok(None)
    }
    /// Upsert one office node row.
    fn put_platform_office_node(&self, node: &PlatformOfficeNode) -> Result<(), PlatformError> {
        let _ = node;
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "platform office nodes are not supported by this adapter",
        ))
    }
    /// List office nodes.
    fn list_platform_office_nodes(&self) -> Result<Vec<PlatformOfficeNode>, PlatformError> {
        Ok(vec![])
    }
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
        _max_age_secs: Option<i64>,
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
        _max_age_secs: Option<i64>,
    ) -> Result<Vec<PipelineInvocationEntry>, PlatformError> {
        Ok(vec![])
    }

    /// Return project-level invocation log storage statistics.
    fn get_pipeline_invocation_log_stats(
        &self,
        _owner: &str,
        _project: &str,
    ) -> Result<PipelineInvocationLogStats, PlatformError> {
        Ok(PipelineInvocationLogStats::default())
    }

    /// Clear invocation log rows for a whole project or one pipeline.
    fn clear_pipeline_invocation_logs(
        &self,
        _owner: &str,
        _project: &str,
        _file_rel_path: Option<&str>,
    ) -> Result<u64, PlatformError> {
        Ok(0)
    }

    /// Re-key all project-scoped records from `(old_owner, project)` to
    /// `(new_owner, project)` in a single atomic transaction.
    /// Used for sovereignty recovery after controller dissolution.
    fn transfer_project_owner(
        &self,
        _old_owner: &str,
        _project: &str,
        _new_owner: &str,
        _new_owner_user_id: &str,
    ) -> Result<(), PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "transfer_project_owner not supported by this adapter",
        ))
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

/// Builds the SQLite-backed hub service catalog at an exact DB path.
///
/// Hub is an office-hosted service in the fresh storage model, so its
/// operational rows live under `{data_root}/services/{service}/hub.db`
/// instead of the platform control-plane catalog.
pub fn build_hub_data_adapter(
    kind: DataAdapterKind,
    db_path: &Path,
) -> Result<Arc<dyn DataAdapter>, PlatformError> {
    match kind {
        DataAdapterKind::Sqlite => Ok(Arc::new(sqlite::SqliteDataAdapter::new_at_db_path(
            db_path,
        )?)),
        DataAdapterKind::DynamoDb | DataAdapterKind::Firebase => Err(PlatformError::new(
            "PLATFORM_HUB_ADAPTER_UNAVAILABLE",
            "office-hosted hub service storage currently requires sqlite",
        )),
    }
}

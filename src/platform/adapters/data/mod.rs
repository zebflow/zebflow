//! Swappable metadata adapters for Zebflow platform.

mod dynamodb;
mod firebase;
mod sekejap;
mod sqlite;

use std::path::Path;
use std::sync::Arc;

use crate::platform::error::PlatformError;
use crate::platform::model::{
    DataAdapterKind, PipelineMeta, PlatformProject, PlatformUser, ProjectCredential,
    ProjectDbConnection, ProjectPolicy, ProjectPolicyBinding, StoredUser,
};

pub use sekejap::SekejapDataAdapter;

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
}

/// Builds selected metadata adapter.
pub fn build_data_adapter(
    kind: DataAdapterKind,
    data_root: &Path,
) -> Result<Arc<dyn DataAdapter>, PlatformError> {
    match kind {
        DataAdapterKind::Sekejap => Ok(Arc::new(SekejapDataAdapter::new(data_root)?)),
        DataAdapterKind::Sqlite => Ok(Arc::new(sqlite::SqliteDataAdapter::default())),
        DataAdapterKind::DynamoDb => Ok(Arc::new(dynamodb::DynamoDbDataAdapter::default())),
        DataAdapterKind::Firebase => Ok(Arc::new(firebase::FirebaseDataAdapter::default())),
    }
}

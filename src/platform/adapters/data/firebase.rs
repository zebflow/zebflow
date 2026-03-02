//! Placeholder Firebase adapter.

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    PipelineMeta, PlatformProject, PlatformUser, ProjectAssistantConfig, ProjectCredential,
    ProjectDbConnection, ProjectPolicy, ProjectPolicyBinding, StoredUser,
};

/// Stub adapter for future Firebase support.
#[derive(Default)]
pub struct FirebaseDataAdapter;

impl DataAdapter for FirebaseDataAdapter {
    fn id(&self) -> &'static str {
        "data.firebase"
    }

    fn get_user_auth(&self, _owner: &str) -> Result<Option<StoredUser>, PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn put_user(&self, _user: &StoredUser) -> Result<(), PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn list_users(&self) -> Result<Vec<PlatformUser>, PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn get_project(
        &self,
        _owner: &str,
        _project: &str,
    ) -> Result<Option<PlatformProject>, PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn put_project(&self, _project: &PlatformProject) -> Result<(), PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn list_projects(&self, _owner: &str) -> Result<Vec<PlatformProject>, PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn get_project_credential(
        &self,
        _owner: &str,
        _project: &str,
        _credential_id: &str,
    ) -> Result<Option<ProjectCredential>, PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn put_project_credential(&self, _credential: &ProjectCredential) -> Result<(), PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn list_project_credentials(
        &self,
        _owner: &str,
        _project: &str,
    ) -> Result<Vec<ProjectCredential>, PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn delete_project_credential(
        &self,
        _owner: &str,
        _project: &str,
        _credential_id: &str,
    ) -> Result<(), PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn get_project_db_connection(
        &self,
        _owner: &str,
        _project: &str,
        _connection_slug: &str,
    ) -> Result<Option<ProjectDbConnection>, PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn put_project_db_connection(
        &self,
        _connection: &ProjectDbConnection,
    ) -> Result<(), PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn list_project_db_connections(
        &self,
        _owner: &str,
        _project: &str,
    ) -> Result<Vec<ProjectDbConnection>, PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn delete_project_db_connection(
        &self,
        _owner: &str,
        _project: &str,
        _connection_slug: &str,
    ) -> Result<(), PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn get_project_assistant_config(
        &self,
        _owner: &str,
        _project: &str,
    ) -> Result<Option<ProjectAssistantConfig>, PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn put_project_assistant_config(
        &self,
        _config: &ProjectAssistantConfig,
    ) -> Result<(), PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn put_pipeline_meta(&self, _meta: &PipelineMeta) -> Result<(), PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn list_pipeline_meta(
        &self,
        _owner: &str,
        _project: &str,
    ) -> Result<Vec<PipelineMeta>, PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn put_project_policy(&self, _policy: &ProjectPolicy) -> Result<(), PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn list_project_policies(
        &self,
        _owner: &str,
        _project: &str,
    ) -> Result<Vec<ProjectPolicy>, PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn put_project_policy_binding(
        &self,
        _binding: &ProjectPolicyBinding,
    ) -> Result<(), PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn list_project_policy_bindings(
        &self,
        _owner: &str,
        _project: &str,
    ) -> Result<Vec<ProjectPolicyBinding>, PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "firebase adapter is not implemented yet",
        ))
    }

    fn delete_project_policy(
        &self,
        _owner: &str,
        _project: &str,
        _policy_id: &str,
    ) -> Result<(), PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_FIREBASE_UNIMPLEMENTED",
            "firebase adapter not implemented",
        ))
    }

    fn delete_project_policy_binding(
        &self,
        _owner: &str,
        _project: &str,
        _subject_id: &str,
    ) -> Result<(), PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_FIREBASE_UNIMPLEMENTED",
            "firebase adapter not implemented",
        ))
    }
}

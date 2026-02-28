//! Placeholder SQLite adapter.

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{PipelineMeta, PlatformProject, PlatformUser, StoredUser};

/// Stub adapter for future SQLite support.
#[derive(Default)]
pub struct SqliteDataAdapter;

impl DataAdapter for SqliteDataAdapter {
    fn id(&self) -> &'static str {
        "data.sqlite"
    }

    fn get_user_auth(&self, _owner: &str) -> Result<Option<StoredUser>, PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "sqlite adapter is not implemented yet",
        ))
    }

    fn put_user(&self, _user: &StoredUser) -> Result<(), PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "sqlite adapter is not implemented yet",
        ))
    }

    fn list_users(&self) -> Result<Vec<PlatformUser>, PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "sqlite adapter is not implemented yet",
        ))
    }

    fn get_project(
        &self,
        _owner: &str,
        _project: &str,
    ) -> Result<Option<PlatformProject>, PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "sqlite adapter is not implemented yet",
        ))
    }

    fn put_project(&self, _project: &PlatformProject) -> Result<(), PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "sqlite adapter is not implemented yet",
        ))
    }

    fn list_projects(&self, _owner: &str) -> Result<Vec<PlatformProject>, PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "sqlite adapter is not implemented yet",
        ))
    }

    fn put_pipeline_meta(&self, _meta: &PipelineMeta) -> Result<(), PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "sqlite adapter is not implemented yet",
        ))
    }

    fn list_pipeline_meta(
        &self,
        _owner: &str,
        _project: &str,
    ) -> Result<Vec<PipelineMeta>, PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_ADAPTER_UNAVAILABLE",
            "sqlite adapter is not implemented yet",
        ))
    }
}

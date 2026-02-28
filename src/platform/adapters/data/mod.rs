//! Swappable metadata adapters for Zebflow platform.

mod dynamodb;
mod firebase;
mod sekejap;
mod sqlite;

use std::path::Path;
use std::sync::Arc;

use crate::platform::error::PlatformError;
use crate::platform::model::{
    DataAdapterKind, PipelineMeta, PlatformProject, PlatformUser, StoredUser,
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
    /// Upsert one pipeline metadata row.
    fn put_pipeline_meta(&self, meta: &PipelineMeta) -> Result<(), PlatformError>;
    /// List pipeline metadata rows by owner/project.
    fn list_pipeline_meta(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<PipelineMeta>, PlatformError>;
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

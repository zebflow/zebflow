use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

use crate::platform::error::PlatformError;
use crate::platform::model::{
    DbCapabilities, DescribeProjectDbConnectionRequest, ProjectDbConnection,
    ProjectDbConnectionDescribeResult, ProjectDbConnectionQueryResult,
    QueryProjectDbConnectionRequest,
};
use crate::platform::services::CredentialService;

/// Shared context passed to one DB driver call.
#[derive(Clone)]
pub struct DbDriverContext {
    pub owner: String,
    pub project: String,
    pub data_root: PathBuf,
    pub connection: ProjectDbConnection,
    pub credentials: Arc<CredentialService>,
}

/// Runtime driver contract for one database kind.
#[async_trait]
pub trait DbDriver: Send + Sync {
    /// Stable kind key (`sqlite`, `postgresql`, ...).
    fn kind(&self) -> &'static str;

    /// What this engine supports.
    ///
    /// The UI renders its panels from this rather than from `kind()`, so an
    /// engine gains a feature by declaring it here and nowhere else. The
    /// default is read-and-query only, which is always safe.
    fn capabilities(&self) -> DbCapabilities {
        DbCapabilities::default()
    }

    /// Describes objects available in one connection.
    async fn describe(
        &self,
        ctx: &DbDriverContext,
        req: &DescribeProjectDbConnectionRequest,
    ) -> Result<ProjectDbConnectionDescribeResult, PlatformError>;

    /// Executes one query against a connection.
    async fn query(
        &self,
        ctx: &DbDriverContext,
        req: &QueryProjectDbConnectionRequest,
    ) -> Result<ProjectDbConnectionQueryResult, PlatformError>;
}

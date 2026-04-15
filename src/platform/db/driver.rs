use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

use crate::platform::error::PlatformError;
use crate::platform::model::{
    DescribeProjectDbConnectionRequest, ProjectDbConnection, ProjectDbConnectionDescribeResult,
    ProjectDbConnectionQueryResult, QueryProjectDbConnectionRequest,
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

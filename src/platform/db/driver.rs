use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

use crate::platform::error::PlatformError;
use crate::platform::model::{
    CreateSimpleTableRequest, DbCapabilities, DescribeProjectDbConnectionRequest,
    ProjectDbConnection, ProjectDbConnectionDescribeResult, ProjectDbConnectionQueryResult,
    QueryProjectDbConnectionRequest, SimpleTableDefinition, UpdateSimpleTableRequest,
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

    /// Which SQL dialect this engine speaks, if any.
    ///
    /// Callers that must write a statement themselves — the studio's table
    /// preview, for one — ask this rather than matching on the kind name, so
    /// identifiers are quoted the way the engine expects. `None` means the
    /// engine is not addressed with quoted SQL identifiers.
    fn sql_dialect(&self) -> Option<crate::platform::db::sql_ddl::SqlDialect> {
        None
    }

    /// The column types this engine offers, for the studio's type picker.
    ///
    /// Empty means the engine does not let a caller choose a type.
    fn type_catalog(&self) -> Vec<crate::platform::model::DbTypeDef> {
        Vec::new()
    }

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

    /// Creates one table in this connection's database.
    ///
    /// Refused unless the driver declares `create_table`, so an engine cannot
    /// half-support table definition: the capability and the implementation
    /// arrive together.
    async fn create_table(
        &self,
        _ctx: &DbDriverContext,
        _req: &CreateSimpleTableRequest,
    ) -> Result<SimpleTableDefinition, PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_DB_DDL_UNSUPPORTED",
            format!("'{}' cannot create tables", self.kind()),
        ))
    }

    /// Changes one table to match a wanted set of attributes.
    ///
    /// Refused unless the driver declares `edit_table_properties`.
    async fn alter_table(
        &self,
        _ctx: &DbDriverContext,
        _table: &str,
        _req: &UpdateSimpleTableRequest,
    ) -> Result<SimpleTableDefinition, PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_DB_DDL_UNSUPPORTED",
            format!("'{}' cannot alter tables", self.kind()),
        ))
    }

    /// Inserts one row with the values given and answers with its identity.
    ///
    /// Values rather than an empty row, because most tables have columns that
    /// cannot be null: an empty insert is rejected by the database on any table
    /// worth having. The studio collects the values first and sends them here.
    ///
    /// The statement differs by engine — sekejap supplies its own `_key`,
    /// PostgreSQL answers with `RETURNING`, MySQL with its last insert id — so
    /// the driver writes it. Update and delete are ordinary SQL once the
    /// identity column is known, so they stay in the caller.
    async fn insert_row(
        &self,
        _ctx: &DbDriverContext,
        _table: &str,
        _values: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<serde_json::Value, PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_DB_ROW_UNSUPPORTED",
            format!("'{}' cannot add rows", self.kind()),
        ))
    }

    /// Drops one table from this connection's database.
    async fn drop_table(&self, _ctx: &DbDriverContext, _table: &str) -> Result<(), PlatformError> {
        Err(PlatformError::new(
            "PLATFORM_DB_DDL_UNSUPPORTED",
            format!("'{}' cannot drop tables", self.kind()),
        ))
    }
}

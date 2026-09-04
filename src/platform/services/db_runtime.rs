//! Project DB runtime service (describe/query) dispatching by database kind.

use std::path::PathBuf;
use std::sync::Arc;

use crate::platform::db::{DbDriverContext, DbDriverRegistry};
use crate::platform::error::PlatformError;
use crate::platform::model::{
    CreateSimpleTableRequest, DbCapabilities, DescribeProjectDbConnectionRequest,
    ProjectDbConnection, ProjectDbConnectionDescribeResult, ProjectDbConnectionQueryResult,
    QueryProjectDbConnectionRequest, SimpleTableDefinition, slug_segment,
};

use super::{CredentialService, DbConnectionService};

pub struct DbRuntimeService {
    db_connections: Arc<DbConnectionService>,
    credentials: Arc<CredentialService>,
    data_root: PathBuf,
    drivers: DbDriverRegistry,
}

impl DbRuntimeService {
    /// Creates runtime service with built-in driver registry.
    pub fn new(
        db_connections: Arc<DbConnectionService>,
        credentials: Arc<CredentialService>,
        data_root: PathBuf,
    ) -> Self {
        Self {
            db_connections,
            credentials,
            data_root,
            drivers: DbDriverRegistry::with_defaults(),
        }
    }

    /// What one database kind supports.
    ///
    /// Page handlers use this to decide which panels to offer before any
    /// query runs. An unknown kind reports the conservative default rather
    /// than failing, so a connection whose driver is missing still renders a
    /// read-only shell instead of an error page.
    pub fn capabilities_for_kind(&self, database_kind: &str) -> DbCapabilities {
        self.drivers
            .get(database_kind)
            .map(|driver| driver.capabilities())
            .unwrap_or_default()
    }

    /// Which SQL dialect one database kind speaks, if any.
    pub fn sql_dialect_for_kind(
        &self,
        database_kind: &str,
    ) -> Option<crate::platform::db::sql_ddl::SqlDialect> {
        self.drivers.get(database_kind)?.sql_dialect()
    }

    /// Whether a runtime driver exists for one database kind.
    pub fn has_driver(&self, database_kind: &str) -> bool {
        self.drivers.get(database_kind).is_some()
    }

    /// Describes one DB connection by immutable connection id.
    pub async fn describe_connection(
        &self,
        owner: &str,
        project: &str,
        connection_id: &str,
        req: &DescribeProjectDbConnectionRequest,
    ) -> Result<ProjectDbConnectionDescribeResult, PlatformError> {
        let (owner, project, connection) =
            self.resolve_connection(owner, project, connection_id)?;
        let driver = self.drivers.get(&connection.database_kind).ok_or_else(|| {
            PlatformError::new(
                "PLATFORM_DB_DRIVER_MISSING",
                format!("no runtime driver for '{}'", connection.database_kind),
            )
        })?;

        let ctx = DbDriverContext {
            owner,
            project,
            data_root: self.data_root.clone(),
            connection,
            credentials: self.credentials.clone(),
        };
        // Stamped here rather than inside each driver's result construction, so
        // a driver reports its capabilities by implementing one method and can
        // never forget to copy them into the payload.
        let mut result = driver.describe(&ctx, req).await?;
        result.capabilities = driver.capabilities();
        Ok(result)
    }

    /// Creates one table in the database this connection points at.
    ///
    /// The capability is checked before the driver is asked, so an engine that
    /// does not declare table definition refuses here rather than part way
    /// through a statement.
    pub async fn create_table(
        &self,
        owner: &str,
        project: &str,
        connection_id: &str,
        req: &CreateSimpleTableRequest,
    ) -> Result<SimpleTableDefinition, PlatformError> {
        let (ctx, driver) = self.driver_context(owner, project, connection_id)?;
        if !driver.capabilities().create_table {
            return Err(PlatformError::new(
                "PLATFORM_DB_DDL_UNSUPPORTED",
                format!("'{}' cannot create tables", driver.kind()),
            ));
        }
        driver.create_table(&ctx, req).await
    }

    /// Drops one table from the database this connection points at.
    pub async fn drop_table(
        &self,
        owner: &str,
        project: &str,
        connection_id: &str,
        table: &str,
    ) -> Result<(), PlatformError> {
        let (ctx, driver) = self.driver_context(owner, project, connection_id)?;
        if !driver.capabilities().drop_table {
            return Err(PlatformError::new(
                "PLATFORM_DB_DDL_UNSUPPORTED",
                format!("'{}' cannot drop tables", driver.kind()),
            ));
        }
        driver.drop_table(&ctx, table).await
    }

    /// Resolves one connection to its driver and call context.
    fn driver_context(
        &self,
        owner: &str,
        project: &str,
        connection_id: &str,
    ) -> Result<(DbDriverContext, std::sync::Arc<dyn crate::platform::db::DbDriver>), PlatformError>
    {
        let (owner, project, connection) =
            self.resolve_connection(owner, project, connection_id)?;
        let driver = self.drivers.get(&connection.database_kind).ok_or_else(|| {
            PlatformError::new(
                "PLATFORM_DB_DRIVER_MISSING",
                format!("no runtime driver for '{}'", connection.database_kind),
            )
        })?;
        Ok((
            DbDriverContext {
                owner,
                project,
                data_root: self.data_root.clone(),
                connection,
                credentials: self.credentials.clone(),
            },
            driver,
        ))
    }

    /// Executes one query against DB connection by immutable connection id.
    pub async fn query_connection(
        &self,
        owner: &str,
        project: &str,
        connection_id: &str,
        req: &QueryProjectDbConnectionRequest,
    ) -> Result<ProjectDbConnectionQueryResult, PlatformError> {
        let (owner, project, connection) =
            self.resolve_connection(owner, project, connection_id)?;
        let driver = self.drivers.get(&connection.database_kind).ok_or_else(|| {
            PlatformError::new(
                "PLATFORM_DB_DRIVER_MISSING",
                format!("no runtime driver for '{}'", connection.database_kind),
            )
        })?;

        let ctx = DbDriverContext {
            owner,
            project,
            data_root: self.data_root.clone(),
            connection,
            credentials: self.credentials.clone(),
        };
        driver.query(&ctx, req).await
    }

    fn resolve_connection(
        &self,
        owner: &str,
        project: &str,
        connection_id: &str,
    ) -> Result<(String, String, ProjectDbConnection), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let connection_id = slug_segment(connection_id);
        if connection_id.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_DB_CONNECTION_INVALID",
                "connection id must not be empty",
            ));
        }

        let items = self
            .db_connections
            .list_project_connections(&owner, &project)?;
        let Some(item) = items
            .into_iter()
            .find(|item| item.connection_id == connection_id)
        else {
            return Err(PlatformError::new(
                "PLATFORM_DB_CONNECTION_MISSING",
                format!("connection id '{}' not found", connection_id),
            ));
        };

        let Some(connection) =
            self.db_connections
                .get_project_connection(&owner, &project, &item.connection_slug)?
        else {
            return Err(PlatformError::new(
                "PLATFORM_DB_CONNECTION_MISSING",
                format!("connection '{}' not found", item.connection_slug),
            ));
        };

        Ok((owner, project, connection))
    }
}

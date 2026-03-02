//! Project DB runtime service (describe/query) dispatching by database kind.

use std::sync::Arc;

use crate::platform::db::{DbDriverContext, DbDriverRegistry};
use crate::platform::error::PlatformError;
use crate::platform::model::{
    DescribeProjectDbConnectionRequest, ProjectDbConnection, ProjectDbConnectionDescribeResult,
    ProjectDbConnectionQueryResult, QueryProjectDbConnectionRequest, slug_segment,
};

use super::{CredentialService, DbConnectionService, SimpleTableService};

pub struct DbRuntimeService {
    db_connections: Arc<DbConnectionService>,
    credentials: Arc<CredentialService>,
    simple_tables: Arc<SimpleTableService>,
    drivers: DbDriverRegistry,
}

impl DbRuntimeService {
    /// Creates runtime service with built-in driver registry.
    pub fn new(
        db_connections: Arc<DbConnectionService>,
        credentials: Arc<CredentialService>,
        simple_tables: Arc<SimpleTableService>,
    ) -> Self {
        Self {
            db_connections,
            credentials,
            simple_tables,
            drivers: DbDriverRegistry::with_defaults(),
        }
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
            connection,
            credentials: self.credentials.clone(),
            simple_tables: self.simple_tables.clone(),
        };
        driver.describe(&ctx, req).await
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
            connection,
            credentials: self.credentials.clone(),
            simple_tables: self.simple_tables.clone(),
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

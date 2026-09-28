use async_trait::async_trait;

use crate::platform::db::driver::{DbDriver, DbDriverContext};
use crate::platform::error::PlatformError;
use crate::platform::model::{
    CreateSimpleTableRequest, DbCapabilities, DbObjectNode, DbRelationStyle, DbTypeDef,
    DbTypeFamily,
    DescribeProjectDbConnectionRequest, ProjectDbConnectionDescribeResult,
    ProjectDbConnectionQueryResult, QueryProjectDbConnectionRequest, SimpleTableDefinition,
    UpdateSimpleTableRequest, slug_segment,
};
use crate::platform::sekejap;

pub struct SekejapDbDriver;

async fn run_blocking<T, F>(f: F) -> Result<T, PlatformError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, PlatformError> + Send + 'static,
{
    tokio::task::spawn_blocking(f).await.map_err(|err| {
        PlatformError::new(
            "PLATFORM_SEKEJAP_TASK_JOIN",
            format!("sekejap blocking task failed: {err}"),
        )
    })?
}

#[async_trait]
impl DbDriver for SekejapDbDriver {
    fn kind(&self) -> &'static str {
        sekejap::DB_KIND
    }

    fn type_catalog(&self) -> Vec<DbTypeDef> {
        // The SQL types a sekejap column can declare, the words `CREATE
        // TABLE` takes. The Studio writes the chosen one as the column's type.
        let def = |name: &str, family: DbTypeFamily, parameterized: bool, note: &str| DbTypeDef {
            name: name.to_string(),
            family,
            parameterized,
            note: note.to_string(),
        };
        vec![
            def("TEXT", DbTypeFamily::Text, false, ""),
            def("INT", DbTypeFamily::Number, false, "whole number"),
            def("BIGINT", DbTypeFamily::Number, false, "whole number"),
            def("REAL", DbTypeFamily::Number, false, ""),
            def("DOUBLE PRECISION", DbTypeFamily::Number, false, ""),
            def("BOOLEAN", DbTypeFamily::Boolean, false, ""),
            def("JSONB", DbTypeFamily::Json, false, ""),
            def("TIMESTAMPTZ", DbTypeFamily::DateTime, false, ""),
            def("DATE", DbTypeFamily::DateTime, false, ""),
            // A shape and an SRID are optional: `GEOMETRY(Point,4326)`.
            def("GEOMETRY", DbTypeFamily::Geometry, true, "optional shape and SRID, Point,4326"),
            // A vector column carries its dimension: `VECTOR(384)`. There is
            // no default, because the number is the embedding model's.
            def("VECTOR", DbTypeFamily::Vector, true, "the dimension, 384"),
        ]
    }

    fn capabilities(&self) -> DbCapabilities {
        DbCapabilities {
            inline_edit: true,
            create_table: true,
            drop_table: true,
            // Health, sync and compact are sekejap's own maintenance surface.
            maintenance: true,
            // Tables live in schemas (`public` and any `CREATE SCHEMA`), and a
            // statement names one outside `public` as `schema.table`.
            schemas: true,
            // Attributes and index kinds are editable after creation.
            edit_table_properties: true,
            // Every sekejap row is addressed by `_key`.
            row_identity: "_key".to_string(),
            // `SELECT *` answers the declared columns; `_key` is named.
            row_identity_hidden: true,
            // CREATE TABLE and ADD COLUMN take NOT NULL, DEFAULT and UNIQUE.
            column_constraints: true,
            key_defaults: vec!["ulid()".to_string(), "uuid4()".to_string()],
            geo: true,
            // Rows are joined by free edges rather than declared keys.
            relations: DbRelationStyle::Graph,
        }
    }

    async fn describe(
        &self,
        ctx: &DbDriverContext,
        req: &DescribeProjectDbConnectionRequest,
    ) -> Result<ProjectDbConnectionDescribeResult, PlatformError> {
        let scope = req
            .scope
            .as_deref()
            .map(slug_segment)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "tree".to_string());

        let data_root = ctx.data_root.clone();
        let owner = ctx.owner.clone();
        let project = ctx.project.clone();
        let table = req.table.clone();
        let scope_for_nodes = scope.clone();
        let nodes = run_blocking(move || match scope_for_nodes.as_str() {
            "schemas" => sekejap::describe_schemas(&data_root, &owner, &project),
            "tables" => sekejap::describe_tables(&data_root, &owner, &project),
            "functions" => Ok(Vec::<DbObjectNode>::new()),
            "columns" => {
                let table = table.as_deref().unwrap_or_default();
                sekejap::describe_columns(&data_root, &owner, &project, table)
            }
            _ => sekejap::describe_tree(&data_root, &owner, &project),
        })
        .await?;

        Ok(ProjectDbConnectionDescribeResult {
            connection_id: ctx.connection.connection_id.clone(),
            connection_slug: ctx.connection.connection_slug.clone(),
            database_kind: ctx.connection.database_kind.clone(),
            scope,
            capabilities: self.capabilities(),
            nodes,
        })
    }

    async fn create_table(
        &self,
        ctx: &DbDriverContext,
        req: &CreateSimpleTableRequest,
    ) -> Result<SimpleTableDefinition, PlatformError> {
        let data_root = ctx.data_root.clone();
        let owner = ctx.owner.clone();
        let project = ctx.project.clone();
        let req = req.clone();
        run_blocking(move || sekejap::create_table(&data_root, &owner, &project, &req)).await
    }

    async fn alter_table(
        &self,
        ctx: &DbDriverContext,
        table: &str,
        req: &UpdateSimpleTableRequest,
    ) -> Result<SimpleTableDefinition, PlatformError> {
        let data_root = ctx.data_root.clone();
        let owner = ctx.owner.clone();
        let project = ctx.project.clone();
        // The name as the tree gave it: `geo.places` is not `places`.
        let table = table.to_string();
        let req = req.clone();
        run_blocking(move || sekejap::update_table(&data_root, &owner, &project, &table, &req)).await
    }

    async fn insert_row(
        &self,
        ctx: &DbDriverContext,
        table: &str,
        values: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<serde_json::Value, PlatformError> {
        let data_root = ctx.data_root.clone();
        let owner = ctx.owner.clone();
        let project = ctx.project.clone();
        let table = table.to_string();
        let values = values.clone();
        run_blocking(move || sekejap::insert_row(&data_root, &owner, &project, &table, &values)).await
    }

    async fn drop_table(&self, ctx: &DbDriverContext, table: &str) -> Result<(), PlatformError> {
        let data_root = ctx.data_root.clone();
        let owner = ctx.owner.clone();
        let project = ctx.project.clone();
        let table = table.to_string();
        run_blocking(move || sekejap::delete_table(&data_root, &owner, &project, &table)).await
    }

    async fn query(
        &self,
        ctx: &DbDriverContext,
        req: &QueryProjectDbConnectionRequest,
    ) -> Result<ProjectDbConnectionQueryResult, PlatformError> {
        let data_root = ctx.data_root.clone();
        let owner = ctx.owner.clone();
        let project = ctx.project.clone();
        let connection_id = ctx.connection.connection_id.clone();
        let connection_slug = ctx.connection.connection_slug.clone();
        let req = req.clone();
        run_blocking(move || {
            sekejap::execute_connection_query(
                &data_root,
                &owner,
                &project,
                &connection_id,
                &connection_slug,
                &req,
            )
        })
        .await
    }
}

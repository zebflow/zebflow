use async_trait::async_trait;

use crate::platform::db::driver::{DbDriver, DbDriverContext};
use crate::platform::error::PlatformError;
use crate::platform::model::{
    CreateSimpleTableRequest, DbCapabilities, DbObjectNode, DbRelationStyle,
    DescribeProjectDbConnectionRequest, ProjectDbConnectionDescribeResult,
    ProjectDbConnectionQueryResult, QueryProjectDbConnectionRequest, SimpleTableDefinition,
    slug_segment,
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

    fn capabilities(&self) -> DbCapabilities {
        DbCapabilities {
            inline_edit: true,
            create_table: true,
            drop_table: true,
            // Health, sync and compact are sekejap's own maintenance surface.
            maintenance: true,
            // Collections live in one flat namespace.
            schemas: false,
            // Attributes and index kinds are editable after creation.
            edit_table_properties: true,
            // Every sekejap row is addressed by `_key`.
            row_identity: "_key".to_string(),
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

    async fn insert_empty_row(
        &self,
        ctx: &DbDriverContext,
        table: &str,
    ) -> Result<serde_json::Value, PlatformError> {
        let bare = table.rsplit('.').next().unwrap_or(table).to_string();
        // Sekejap rows carry a key the caller supplies, so one is minted here
        // and answered back for the studio to select.
        let key = uuid::Uuid::new_v4().to_string();
        let sql = format!("INSERT INTO {bare} (_key) VALUES ('{key}')");
        let req = QueryProjectDbConnectionRequest {
            sql,
            read_only: Some(false),
            ..Default::default()
        };
        self.query(ctx, &req).await?;
        Ok(serde_json::Value::String(key))
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

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

/// One value as sekejap's SQL spells it.
///
/// Sekejap's parser takes literals rather than bind parameters here, so a
/// string is quoted and its quotes doubled.
fn sekejap_literal(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(text) => format!("'{}'", text.replace('\'', "''")),
        other => format!("'{}'", other.to_string().replace('\'', "''")),
    }
}

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
        // Sekejap stores values by kind rather than by SQL type, so these are
        // its own words, not a dialect's.
        let def = |name: &str, family: DbTypeFamily, note: &str| DbTypeDef {
            name: name.to_string(),
            family,
            parameterized: false,
            note: note.to_string(),
        };
        vec![
            def("string", DbTypeFamily::Text, ""),
            def("text", DbTypeFamily::Text, "long form"),
            def("number", DbTypeFamily::Number, ""),
            def("boolean", DbTypeFamily::Boolean, ""),
            def("json", DbTypeFamily::Json, ""),
            def("geo", DbTypeFamily::Geometry, "GeoJSON"),
            def("vector", DbTypeFamily::Vector, "similarity search"),
        ]
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

    async fn alter_table(
        &self,
        ctx: &DbDriverContext,
        table: &str,
        req: &UpdateSimpleTableRequest,
    ) -> Result<SimpleTableDefinition, PlatformError> {
        let data_root = ctx.data_root.clone();
        let owner = ctx.owner.clone();
        let project = ctx.project.clone();
        let table = table.rsplit('.').next().unwrap_or(table).to_string();
        let req = req.clone();
        run_blocking(move || sekejap::update_table(&data_root, &owner, &project, &table, &req)).await
    }

    async fn insert_row(
        &self,
        ctx: &DbDriverContext,
        table: &str,
        values: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<serde_json::Value, PlatformError> {
        let bare = table.rsplit('.').next().unwrap_or(table).to_string();
        // A sekejap row carries the key its caller supplies, so one is minted
        // here unless the caller chose it.
        let key = values
            .get("_key")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let mut columns = vec!["_key".to_string()];
        let mut literals = vec![format!("'{}'", key.replace('\'', "''"))];
        for (name, value) in values {
            if name == "_key" {
                continue;
            }
            columns.push(name.clone());
            literals.push(sekejap_literal(value));
        }
        let sql = format!(
            "INSERT INTO {bare} ({}) VALUES ({})",
            columns.join(", "),
            literals.join(", ")
        );
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

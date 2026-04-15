use async_trait::async_trait;

use crate::platform::db::driver::{DbDriver, DbDriverContext};
use crate::platform::error::PlatformError;
use crate::platform::model::{
    DbObjectNode, DescribeProjectDbConnectionRequest, ProjectDbConnectionDescribeResult,
    ProjectDbConnectionQueryResult, QueryProjectDbConnectionRequest, slug_segment,
};
use crate::platform::sekejap;

pub struct SekejapDbDriver;

#[async_trait]
impl DbDriver for SekejapDbDriver {
    fn kind(&self) -> &'static str {
        sekejap::DB_KIND
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

        let nodes = match scope.as_str() {
            "schemas" => sekejap::describe_schemas(&ctx.data_root, &ctx.owner, &ctx.project)?,
            "tables" => sekejap::describe_tables(&ctx.data_root, &ctx.owner, &ctx.project)?,
            "functions" => Vec::<DbObjectNode>::new(),
            "columns" => {
                let table = req.table.as_deref().unwrap_or_default();
                sekejap::describe_columns(&ctx.data_root, &ctx.owner, &ctx.project, table)?
            }
            _ => sekejap::describe_tree(&ctx.data_root, &ctx.owner, &ctx.project)?,
        };

        Ok(ProjectDbConnectionDescribeResult {
            connection_id: ctx.connection.connection_id.clone(),
            connection_slug: ctx.connection.connection_slug.clone(),
            database_kind: ctx.connection.database_kind.clone(),
            scope,
            nodes,
        })
    }

    async fn query(
        &self,
        ctx: &DbDriverContext,
        req: &QueryProjectDbConnectionRequest,
    ) -> Result<ProjectDbConnectionQueryResult, PlatformError> {
        sekejap::execute_connection_query(
            &ctx.data_root,
            &ctx.owner,
            &ctx.project,
            &ctx.connection.connection_id,
            &ctx.connection.connection_slug,
            req,
        )
    }
}

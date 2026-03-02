use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use async_trait::async_trait;
use futures::TryStreamExt;
use serde_json::{Map, Value, json};
use sqlx::{Column, Row, TypeInfo, postgres::PgConnectOptions, postgres::PgRow};
use sqlx::types::Uuid;
use sqlx::types::chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc};

use crate::platform::db::driver::{DbDriver, DbDriverContext};
use crate::platform::error::PlatformError;
use crate::platform::model::{
    DbObjectNode, DbQueryColumn, DescribeProjectDbConnectionRequest,
    ProjectDbConnectionDescribeResult, ProjectDbConnectionQueryResult,
    QueryProjectDbConnectionRequest, slug_segment,
};

#[derive(Default)]
pub struct PostgresqlDbDriver;

#[async_trait]
impl DbDriver for PostgresqlDbDriver {
    fn kind(&self) -> &'static str {
        "postgresql"
    }

    async fn describe(
        &self,
        ctx: &DbDriverContext,
        req: &DescribeProjectDbConnectionRequest,
    ) -> Result<ProjectDbConnectionDescribeResult, PlatformError> {
        let scope = normalize_scope(req.scope.as_deref())?;
        let include_system = req.include_system.unwrap_or(false);
        let schema_filter = req
            .schema
            .as_deref()
            .map(slug_segment)
            .filter(|value| !value.is_empty());

        let pool = connect_pool(ctx).await?;
        let nodes = match scope.as_str() {
            "schemas" => describe_schemas(&pool, include_system).await?,
            "tables" => describe_tables(&pool, schema_filter.as_deref(), include_system).await?,
            "functions" => {
                describe_functions(&pool, schema_filter.as_deref(), include_system).await?
            }
            _ => describe_tree(&pool, schema_filter.as_deref(), include_system).await?,
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
        let started = Instant::now();
        let sql = req.sql.trim();
        if sql.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_DB_QUERY_INVALID",
                "query.sql must not be empty for postgresql",
            ));
        }

        let read_only = req.read_only.unwrap_or(true);
        let max_rows = req.limit.unwrap_or(200).clamp(1, 5_000);
        let statement_kind = statement_kind(sql);

        if read_only && !matches!(statement_kind, StatementKind::Read) {
            return Err(PlatformError::new(
                "PLATFORM_DB_QUERY_READ_ONLY",
                "write statement rejected in read-only mode",
            ));
        }

        let pool = connect_pool(ctx).await?;

        if matches!(statement_kind, StatementKind::Write) {
            let mut query = sqlx::query(sql);
            for param in &req.params {
                query = bind_json_param(query, param);
            }
            let result = query
                .execute(&pool)
                .await
                .map_err(|err| PlatformError::new("PLATFORM_DB_QUERY_FAILED", err.to_string()))?;
            return Ok(ProjectDbConnectionQueryResult {
                connection_id: ctx.connection.connection_id.clone(),
                connection_slug: ctx.connection.connection_slug.clone(),
                database_kind: ctx.connection.database_kind.clone(),
                columns: Vec::new(),
                rows: Vec::new(),
                row_count: 0,
                truncated: false,
                affected_rows: Some(result.rows_affected()),
                duration_ms: started.elapsed().as_millis() as u64,
            });
        }

        let mut query = sqlx::query(sql);
        for param in &req.params {
            query = bind_json_param(query, param);
        }
        let mut stream = query.fetch(&pool);

        let mut columns = Vec::<DbQueryColumn>::new();
        let mut names = Vec::<String>::new();
        let mut rows = Vec::<Vec<Value>>::new();
        let mut truncated = false;

        while let Some(row) = stream
            .try_next()
            .await
            .map_err(|err| PlatformError::new("PLATFORM_DB_QUERY_FAILED", err.to_string()))?
        {
            if columns.is_empty() {
                columns = row
                    .columns()
                    .iter()
                    .map(|column| DbQueryColumn {
                        name: column.name().to_string(),
                        data_type: Some(column.type_info().name().to_string()),
                    })
                    .collect();
                names = columns.iter().map(|column| column.name.clone()).collect();
            }

            if rows.len() >= max_rows {
                truncated = true;
                break;
            }

            let obj = row_to_json_object(row)?;
            let cells = names
                .iter()
                .map(|name| obj.get(name).cloned().unwrap_or(Value::Null))
                .collect::<Vec<_>>();
            rows.push(cells);
        }

        Ok(ProjectDbConnectionQueryResult {
            connection_id: ctx.connection.connection_id.clone(),
            connection_slug: ctx.connection.connection_slug.clone(),
            database_kind: ctx.connection.database_kind.clone(),
            row_count: rows.len(),
            columns,
            rows,
            truncated,
            affected_rows: None,
            duration_ms: started.elapsed().as_millis() as u64,
        })
    }
}

async fn describe_tree(
    pool: &sqlx::PgPool,
    schema_filter: Option<&str>,
    include_system: bool,
) -> Result<Vec<DbObjectNode>, PlatformError> {
    let schema_nodes = describe_schemas(pool, include_system).await?;
    let table_nodes = describe_tables(pool, schema_filter, include_system).await?;
    let function_nodes = describe_functions(pool, schema_filter, include_system).await?;

    let mut by_schema = BTreeMap::<String, Vec<DbObjectNode>>::new();
    for node in table_nodes.into_iter().chain(function_nodes.into_iter()) {
        let key = node.schema.clone().unwrap_or_else(|| "public".to_string());
        by_schema.entry(key).or_default().push(node);
    }

    let mut out = Vec::new();
    for mut schema_node in schema_nodes {
        let key = schema_node.name.clone();
        let mut children = by_schema.remove(&key).unwrap_or_default();
        children.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.name.cmp(&b.name)));
        schema_node.children = children;
        out.push(schema_node);
    }
    Ok(out)
}

async fn describe_schemas(
    pool: &sqlx::PgPool,
    include_system: bool,
) -> Result<Vec<DbObjectNode>, PlatformError> {
    let rows = sqlx::query(
        "SELECT schema_name FROM information_schema.schemata\n         WHERE ($1::bool OR schema_name NOT IN ('pg_catalog', 'information_schema'))\n         ORDER BY schema_name",
    )
    .bind(include_system)
    .fetch_all(pool)
    .await
    .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;

    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let schema_name = row.get::<String, _>("schema_name");
            if !include_system && schema_name.starts_with('_') {
                return None;
            }
            Some(DbObjectNode {
                kind: "schema".to_string(),
                name: schema_name,
                schema: None,
                children: Vec::new(),
                meta: json!({}),
            })
        })
        .collect())
}

async fn describe_tables(
    pool: &sqlx::PgPool,
    schema_filter: Option<&str>,
    include_system: bool,
) -> Result<Vec<DbObjectNode>, PlatformError> {
    let rows = sqlx::query(
        "SELECT table_schema, table_name\n         FROM information_schema.tables\n         WHERE table_type = 'BASE TABLE'\n           AND ($1::text IS NULL OR table_schema = $1)\n           AND ($2::bool OR table_schema NOT IN ('pg_catalog', 'information_schema'))\n         ORDER BY table_schema, table_name",
    )
    .bind(schema_filter)
    .bind(include_system)
    .fetch_all(pool)
    .await
    .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;

    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let schema: String = row.get("table_schema");
            if !include_system && schema.starts_with('_') {
                return None;
            }
            let table: String = row.get("table_name");
            Some(DbObjectNode {
                kind: "table".to_string(),
                name: table,
                schema: Some(schema),
                children: Vec::new(),
                meta: json!({}),
            })
        })
        .collect())
}

async fn describe_functions(
    pool: &sqlx::PgPool,
    schema_filter: Option<&str>,
    include_system: bool,
) -> Result<Vec<DbObjectNode>, PlatformError> {
    let rows = sqlx::query(
        "SELECT n.nspname AS schema_name, p.proname AS function_name\n         FROM pg_proc p\n         JOIN pg_namespace n ON n.oid = p.pronamespace\n         WHERE ($1::text IS NULL OR n.nspname = $1)\n           AND ($2::bool OR n.nspname NOT IN ('pg_catalog', 'information_schema'))\n         ORDER BY n.nspname, p.proname",
    )
    .bind(schema_filter)
    .bind(include_system)
    .fetch_all(pool)
    .await
    .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;

    let mut seen = BTreeSet::<(String, String)>::new();
    let mut out = Vec::new();
    for row in rows {
        let schema: String = row.get("schema_name");
        if !include_system && schema.starts_with('_') {
            continue;
        }
        let function: String = row.get("function_name");
        if !seen.insert((schema.clone(), function.clone())) {
            continue;
        }
        out.push(DbObjectNode {
            kind: "function".to_string(),
            name: function,
            schema: Some(schema),
            children: Vec::new(),
            meta: json!({}),
        });
    }
    Ok(out)
}

fn normalize_scope(raw: Option<&str>) -> Result<String, PlatformError> {
    let normalized = raw
        .map(slug_segment)
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "tree".to_string());
    match normalized.as_str() {
        "tree" | "schemas" | "tables" | "functions" => Ok(normalized),
        _ => Err(PlatformError::new(
            "PLATFORM_DB_DESCRIBE_SCOPE_INVALID",
            "scope must be one of: tree, schemas, tables, functions",
        )),
    }
}

async fn connect_pool(ctx: &DbDriverContext) -> Result<sqlx::PgPool, PlatformError> {
    let credential_id = ctx.connection.credential_id.as_deref().ok_or_else(|| {
        PlatformError::new(
            "PLATFORM_DB_CONNECTION_INVALID",
            "postgresql connection requires credential_id",
        )
    })?;

    let credential = ctx
        .credentials
        .get_project_credential(&ctx.owner, &ctx.project, credential_id)?
        .ok_or_else(|| {
            PlatformError::new(
                "PLATFORM_DB_CONNECTION_INVALID",
                format!("credential '{}' not found", credential_id),
            )
        })?;

    if credential.kind != "postgres" {
        return Err(PlatformError::new(
            "PLATFORM_DB_CONNECTION_INVALID",
            format!(
                "credential '{}' kind '{}' is not compatible with postgresql",
                credential.credential_id, credential.kind
            ),
        ));
    }

    let options = build_postgres_connect_options(&credential.secret)?;
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(8))
        .connect_with(options)
        .await
        .map_err(|err| PlatformError::new("PLATFORM_DB_CONNECTION_FAILED", err.to_string()))
}

fn build_postgres_connect_options(secret: &Value) -> Result<PgConnectOptions, PlatformError> {
    let host = secret
        .get("host")
        .and_then(Value::as_str)
        .ok_or_else(|| PlatformError::new("PLATFORM_DB_SECRET", "secret.host is required"))?;
    let port = secret
        .get("port")
        .and_then(|value| {
            value.as_u64().or_else(|| {
                value
                    .as_str()
                    .and_then(|raw| raw.trim().parse::<u64>().ok())
            })
        })
        .unwrap_or(5432);
    let port = u16::try_from(port).map_err(|_| {
        PlatformError::new("PLATFORM_DB_SECRET", "secret.port must be in 0..=65535")
    })?;
    let database = secret
        .get("database")
        .and_then(Value::as_str)
        .ok_or_else(|| PlatformError::new("PLATFORM_DB_SECRET", "secret.database is required"))?;
    let user = secret
        .get("user")
        .and_then(Value::as_str)
        .ok_or_else(|| PlatformError::new("PLATFORM_DB_SECRET", "secret.user is required"))?;
    let password = secret.get("password").and_then(Value::as_str).unwrap_or("");

    Ok(PgConnectOptions::new()
        .host(host)
        .port(port)
        .database(database)
        .username(user)
        .password(password))
}

fn bind_json_param<'q>(
    query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    value: &Value,
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    match value {
        Value::Null => query.bind(Option::<String>::None),
        Value::Bool(v) => query.bind(*v),
        Value::Number(n) => query.bind(n.to_string()),
        Value::String(s) => query.bind(s.clone()),
        other => query.bind(other.to_string()),
    }
}

fn row_to_json_object(row: PgRow) -> Result<Map<String, Value>, PlatformError> {
    let mut map = Map::new();
    for (idx, column) in row.columns().iter().enumerate() {
        map.insert(column.name().to_string(), row_cell_to_json(&row, idx));
    }
    Ok(map)
}

fn row_cell_to_json(row: &PgRow, idx: usize) -> Value {
    if let Ok(v) = row.try_get::<Option<serde_json::Value>, _>(idx) {
        return v.unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<String>, _>(idx) {
        return v.map(Value::String).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<Uuid>, _>(idx) {
        return v.map(|value| Value::String(value.to_string())).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<DateTime<Utc>>, _>(idx) {
        return v
            .map(|value| Value::String(value.to_rfc3339()))
            .unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<DateTime<FixedOffset>>, _>(idx) {
        return v
            .map(|value| Value::String(value.to_rfc3339()))
            .unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<NaiveDateTime>, _>(idx) {
        return v
            .map(|value| Value::String(value.format("%Y-%m-%d %H:%M:%S%.f").to_string()))
            .unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<NaiveDate>, _>(idx) {
        return v
            .map(|value| Value::String(value.format("%Y-%m-%d").to_string()))
            .unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<NaiveTime>, _>(idx) {
        return v
            .map(|value| Value::String(value.format("%H:%M:%S%.f").to_string()))
            .unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<bool>, _>(idx) {
        return v.map(Value::Bool).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<i64>, _>(idx) {
        return v.map(|x| json!(x)).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<i32>, _>(idx) {
        return v.map(|x| json!(x)).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<i16>, _>(idx) {
        return v.map(|x| json!(x)).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<f64>, _>(idx) {
        return v
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<f32>, _>(idx) {
        return v
            .and_then(|x| serde_json::Number::from_f64(x as f64))
            .map(Value::Number)
            .unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(idx) {
        return v
            .map(|bytes| Value::String(hex::encode(bytes)))
            .unwrap_or(Value::Null);
    }
    Value::Null
}

enum StatementKind {
    Read,
    Write,
}

fn statement_kind(sql: &str) -> StatementKind {
    let first = sql
        .trim_start()
        .split_whitespace()
        .next()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_default();
    match first.as_str() {
        "select" | "with" | "show" | "explain" => StatementKind::Read,
        _ => StatementKind::Write,
    }
}

//! MySQL runtime driver.
//!
//! Structure is read live from `information_schema` on every call, the same way
//! the PostgreSQL driver reads it, so the studio renders both identically.
//!
//! MySQL calls a namespace a *database* where PostgreSQL calls it a *schema*.
//! The studio speaks in schemas, so that is the word used here; the queries
//! read `TABLE_SCHEMA`, which is the same thing.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use async_trait::async_trait;
use serde_json::{Value, json};
use sqlx::types::chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use sqlx::{Column, Row, TypeInfo, ValueRef, mysql::MySqlConnectOptions, mysql::MySqlRow};

use crate::platform::db::driver::{DbDriver, DbDriverContext};
use crate::platform::db::sql_ddl::{
    ExistingColumn, IDENTITY_COLUMN, SqlDialect, alter_table_statements, create_table_statements,
    drop_table_statement, index_name, validate_identifier,
};
use crate::platform::error::PlatformError;
use crate::platform::model::{
    CollectionAttribute, CreateSimpleTableRequest, DbCapabilities, DbObjectNode, DbQueryColumn,
    DbRelationStyle, DescribeProjectDbConnectionRequest, ProjectDbConnectionDescribeResult,
    ProjectDbConnectionQueryResult, QueryProjectDbConnectionRequest, SimpleTableDefinition,
    UpdateSimpleTableRequest, slug_segment,
};

pub const DB_KIND: &str = "mysql";

/// Namespaces MySQL keeps for itself.
const SYSTEM_SCHEMAS: [&str; 4] = ["information_schema", "mysql", "performance_schema", "sys"];

/// Rows returned for one query, matching the other SQL drivers.
const DEFAULT_ROW_LIMIT: usize = 500;

#[derive(Default)]
pub struct MysqlDbDriver;

fn system_filter(include_system: bool) -> String {
    if include_system {
        String::new()
    } else {
        let list = SYSTEM_SCHEMAS
            .iter()
            .map(|s| format!("'{s}'"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(" AND TABLE_SCHEMA NOT IN ({list})")
    }
}

async fn connect_pool(ctx: &DbDriverContext) -> Result<sqlx::MySqlPool, PlatformError> {
    let credential_id = ctx.connection.credential_id.as_deref().ok_or_else(|| {
        PlatformError::new(
            "PLATFORM_DB_CONNECTION_INVALID",
            "mysql connection requires credential_id",
        )
    })?;
    let credential = ctx
        .credentials
        .get_project_credential(&ctx.owner, &ctx.project, credential_id)?
        .ok_or_else(|| {
            PlatformError::new(
                "PLATFORM_DB_CREDENTIAL_MISSING",
                format!("credential '{credential_id}' not found"),
            )
        })?;
    if credential.kind != DB_KIND {
        return Err(PlatformError::new(
            "PLATFORM_DB_CREDENTIAL_KIND",
            format!(
                "credential '{}' kind '{}' is not compatible with mysql",
                credential.credential_id, credential.kind
            ),
        ));
    }
    let options = build_mysql_connect_options(&credential.secret)?;
    sqlx::MySqlPool::connect_with(options)
        .await
        .map_err(|err| PlatformError::new("PLATFORM_DB_CONNECT_FAILED", err.to_string()))
}

fn build_mysql_connect_options(secret: &Value) -> Result<MySqlConnectOptions, PlatformError> {
    let host = secret
        .get("host")
        .and_then(Value::as_str)
        .ok_or_else(|| PlatformError::new("PLATFORM_DB_SECRET", "secret.host is required"))?;
    let port = secret
        .get("port")
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_str().and_then(|raw| raw.trim().parse::<u64>().ok()))
        })
        .unwrap_or(3306);
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

    Ok(MySqlConnectOptions::new()
        .host(host)
        .port(port)
        .database(database)
        .username(user)
        .password(password))
}

/// Converts one returned cell to JSON.
///
/// The chain mirrors the PostgreSQL driver, ending in a named type rather than
/// a silent null so a value this driver cannot read is never mistaken for an
/// empty column.
fn cell_to_json(row: &MySqlRow, idx: usize) -> Value {
    // MySQL has no boolean type — `BOOLEAN` is an alias for `TINYINT(1)` — so a
    // bool decode succeeds for any small integer and would turn an id of 1 into
    // `true`. The declared type decides, rather than whichever decode happens to
    // work first.
    let declared = row
        .try_get_raw(idx)
        .ok()
        .map(|raw| raw.type_info().name().to_ascii_uppercase())
        .unwrap_or_default();

    if declared == "BOOLEAN" || declared == "BOOL" {
        if let Ok(v) = row.try_get::<Option<bool>, _>(idx) {
            return v.map(Value::Bool).unwrap_or(Value::Null);
        }
    }
    if let Ok(v) = row.try_get::<Option<serde_json::Value>, _>(idx) {
        return v.unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<String>, _>(idx) {
        return v.map(Value::String).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<i64>, _>(idx) {
        return v.map(|x| json!(x)).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<u64>, _>(idx) {
        return v.map(|x| json!(x)).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<sqlx::types::BigDecimal>, _>(idx) {
        // DECIMAL holds more precision than an f64, so exact digits are kept.
        return v.map(|d| Value::String(d.to_string())).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<f64>, _>(idx) {
        return v
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<DateTime<Utc>>, _>(idx) {
        return v.map(|x| json!(x.to_rfc3339())).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<NaiveDateTime>, _>(idx) {
        return v.map(|x| json!(x.to_string())).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<NaiveDate>, _>(idx) {
        return v.map(|x| json!(x.to_string())).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<NaiveTime>, _>(idx) {
        return v.map(|x| json!(x.to_string())).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(idx) {
        return v
            .map(|bytes| Value::String(hex::encode(bytes)))
            .unwrap_or(Value::Null);
    }
    match row.try_get_raw(idx) {
        Ok(raw) if !raw.is_null() => {
            Value::String(format!("<unsupported {}>", raw.type_info().name()))
        }
        _ => Value::Null,
    }
}

/// Reads one text column from `information_schema`.
///
/// MySQL 8 returns these columns binary-collated, so a plain `String` decode
/// fails with a type mismatch. Falling back to bytes keeps the read working
/// whatever collation the server uses, and never panics inside a request.
fn text(row: &MySqlRow, column: &str) -> String {
    if let Ok(value) = row.try_get::<String, _>(column) {
        return value;
    }
    if let Ok(bytes) = row.try_get::<Vec<u8>, _>(column) {
        return String::from_utf8_lossy(&bytes).to_string();
    }
    String::new()
}

/// The same read where the column may be absent or NULL.
fn text_opt(row: &MySqlRow, column: &str) -> Option<String> {
    if let Ok(value) = row.try_get::<Option<String>, _>(column) {
        return value;
    }
    if let Ok(bytes) = row.try_get::<Option<Vec<u8>>, _>(column) {
        return bytes.map(|b| String::from_utf8_lossy(&b).to_string());
    }
    None
}

async fn describe_schemas(
    pool: &sqlx::MySqlPool,
    include_system: bool,
) -> Result<Vec<DbObjectNode>, PlatformError> {
    let sql = format!(
        "SELECT SCHEMA_NAME AS name FROM information_schema.schemata WHERE 1=1{} ORDER BY SCHEMA_NAME",
        system_filter(include_system).replace("TABLE_SCHEMA", "SCHEMA_NAME")
    );
    let rows = sqlx::query(&sql)
        .fetch_all(pool)
        .await
        .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|row| DbObjectNode {
            kind: "schema".to_string(),
            name: text(&row, "name"),
            schema: None,
            children: Vec::new(),
            meta: Value::Null,
        })
        .collect())
}


/// The table's columns as they exist, with whether this platform's index for
/// each one is present.
async fn existing_columns(
    pool: &sqlx::MySqlPool,
    schema: &str,
    table: &str,
) -> Result<Vec<ExistingColumn>, PlatformError> {
    let cols = sqlx::query(
        "SELECT COLUMN_NAME, COLUMN_TYPE FROM information_schema.columns \
         WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? ORDER BY ORDINAL_POSITION",
    )
    .bind(schema)
    .bind(table)
    .fetch_all(pool)
    .await
    .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;

    let indexes = sqlx::query(
        "SELECT DISTINCT INDEX_NAME FROM information_schema.statistics \
         WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?",
    )
    .bind(schema)
    .bind(table)
    .fetch_all(pool)
    .await
    .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;
    let present: Vec<String> = indexes.iter().map(|row| text(row, "INDEX_NAME")).collect();

    Ok(cols
        .into_iter()
        .map(|row| {
            let name = text(&row, "COLUMN_NAME");
            // COLUMN_TYPE keeps the length, so VARCHAR(255) compares equal to
            // what this platform writes rather than to a bare VARCHAR.
            let column_type = text(&row, "COLUMN_TYPE").to_ascii_uppercase();
            let indexed = present.contains(&index_name(table, &name));
            ExistingColumn {
                name,
                column_type,
                indexed,
            }
        })
        .collect())
}

async fn column_lookup(
    pool: &sqlx::MySqlPool,
    schema_filter: Option<&str>,
    include_system: bool,
) -> Result<BTreeMap<(String, String), Vec<Value>>, PlatformError> {
    let mut sql = format!(
        "SELECT TABLE_SCHEMA, TABLE_NAME, COLUMN_NAME, DATA_TYPE, IS_NULLABLE, \
                COLUMN_DEFAULT, COLUMN_KEY \
         FROM information_schema.columns WHERE 1=1{}",
        system_filter(include_system)
    );
    if schema_filter.is_some() {
        sql.push_str(" AND TABLE_SCHEMA = ?");
    }
    sql.push_str(" ORDER BY TABLE_SCHEMA, TABLE_NAME, ORDINAL_POSITION");

    let mut query = sqlx::query(&sql);
    if let Some(schema) = schema_filter {
        query = query.bind(schema);
    }
    let rows = query
        .fetch_all(pool)
        .await
        .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;

    // Foreign keys, so a column can name what it points at.
    let mut fk_sql = format!(
        "SELECT TABLE_SCHEMA, TABLE_NAME, COLUMN_NAME, REFERENCED_TABLE_SCHEMA, \
                REFERENCED_TABLE_NAME, REFERENCED_COLUMN_NAME \
         FROM information_schema.key_column_usage \
         WHERE REFERENCED_TABLE_NAME IS NOT NULL{}",
        system_filter(include_system)
    );
    if schema_filter.is_some() {
        fk_sql.push_str(" AND TABLE_SCHEMA = ?");
    }
    let mut fk_query = sqlx::query(&fk_sql);
    if let Some(schema) = schema_filter {
        fk_query = fk_query.bind(schema);
    }
    let fk_rows = fk_query
        .fetch_all(pool)
        .await
        .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;

    let mut fk_lookup = BTreeMap::<(String, String, String), Value>::new();
    for row in fk_rows {
        fk_lookup.insert(
            (
                text(&row, "TABLE_SCHEMA"),
                text(&row, "TABLE_NAME"),
                text(&row, "COLUMN_NAME"),
            ),
            json!({
                "schema": text_opt(&row, "REFERENCED_TABLE_SCHEMA"),
                "table": text_opt(&row, "REFERENCED_TABLE_NAME"),
                "column": text_opt(&row, "REFERENCED_COLUMN_NAME"),
            }),
        );
    }

    let mut out = BTreeMap::<(String, String), Vec<Value>>::new();
    for row in rows {
        let schema = text(&row, "TABLE_SCHEMA");
        let table = text(&row, "TABLE_NAME");
        let name = text(&row, "COLUMN_NAME");
        let data_type = text(&row, "DATA_TYPE");
        let nullable = text(&row, "IS_NULLABLE");
        let default = text_opt(&row, "COLUMN_DEFAULT");
        let key = text(&row, "COLUMN_KEY");

        let mut col = json!({
            "name": name,
            "type": data_type,
            "nullable": nullable.eq_ignore_ascii_case("YES"),
        });
        if key == "PRI" {
            col["pk"] = json!(true);
        }
        if let Some(fk) = fk_lookup.get(&(schema.clone(), table.clone(), name)) {
            col["fk"] = fk.clone();
        }
        if let Some(d) = default {
            col["default"] = json!(d);
        }
        out.entry((schema, table)).or_default().push(col);
    }
    Ok(out)
}

async fn describe_tables(
    pool: &sqlx::MySqlPool,
    schema_filter: Option<&str>,
    include_system: bool,
) -> Result<Vec<DbObjectNode>, PlatformError> {
    let mut sql = format!(
        "SELECT TABLE_SCHEMA, TABLE_NAME FROM information_schema.tables \
         WHERE TABLE_TYPE = 'BASE TABLE'{}",
        system_filter(include_system)
    );
    if schema_filter.is_some() {
        sql.push_str(" AND TABLE_SCHEMA = ?");
    }
    sql.push_str(" ORDER BY TABLE_SCHEMA, TABLE_NAME");

    let mut query = sqlx::query(&sql);
    if let Some(schema) = schema_filter {
        query = query.bind(schema);
    }
    let rows = query
        .fetch_all(pool)
        .await
        .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;

    let columns = column_lookup(pool, schema_filter, include_system).await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let schema = text(&row, "TABLE_SCHEMA");
            let table = text(&row, "TABLE_NAME");
            let cols = columns
                .get(&(schema.clone(), table.clone()))
                .cloned()
                .unwrap_or_default();
            DbObjectNode {
                kind: "table".to_string(),
                name: table,
                schema: Some(schema),
                children: Vec::new(),
                meta: json!({ "columns": cols }),
            }
        })
        .collect())
}

async fn describe_columns_for_table(
    pool: &sqlx::MySqlPool,
    schema: Option<&str>,
    table: &str,
) -> Result<Vec<DbObjectNode>, PlatformError> {
    let columns = column_lookup(pool, schema, true).await?;
    let matched = columns
        .iter()
        .find(|((s, t), _)| t == table && schema.map(|want| want == s).unwrap_or(true));
    let Some(((schema_name, _), cols)) = matched else {
        return Ok(Vec::new());
    };
    Ok(cols
        .iter()
        .map(|col| DbObjectNode {
            kind: "column".to_string(),
            name: col
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            schema: Some(schema_name.clone()),
            children: Vec::new(),
            meta: col.clone(),
        })
        .collect())
}

async fn describe_tree(
    pool: &sqlx::MySqlPool,
    schema_filter: Option<&str>,
    include_system: bool,
) -> Result<Vec<DbObjectNode>, PlatformError> {
    let schema_nodes = describe_schemas(pool, include_system).await?;
    let table_nodes = describe_tables(pool, schema_filter, include_system).await?;

    let mut by_schema = BTreeMap::<String, Vec<DbObjectNode>>::new();
    for node in table_nodes {
        let key = node.schema.clone().unwrap_or_default();
        by_schema.entry(key).or_default().push(node);
    }
    let mut seen = BTreeSet::<String>::new();
    let mut out = Vec::new();
    for mut schema_node in schema_nodes {
        let key = schema_node.name.clone();
        seen.insert(key.clone());
        schema_node.children = by_schema.remove(&key).unwrap_or_default();
        out.push(schema_node);
    }
    Ok(out)
}

fn row_to_columns(row: &MySqlRow) -> Vec<DbQueryColumn> {
    row.columns()
        .iter()
        .map(|col| DbQueryColumn {
            name: col.name().to_string(),
            data_type: Some(col.type_info().name().to_string()),
        })
        .collect()
}

fn row_to_values(row: &MySqlRow) -> Vec<Value> {
    (0..row.columns().len())
        .map(|idx| cell_to_json(row, idx))
        .collect()
}

fn is_read_statement(sql: &str) -> bool {
    let first = sql
        .trim_start()
        .split_whitespace()
        .next()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_default();
    matches!(first.as_str(), "select" | "show" | "describe" | "desc" | "explain" | "with")
}

#[async_trait]
impl DbDriver for MysqlDbDriver {
    fn kind(&self) -> &'static str {
        DB_KIND
    }

    fn sql_dialect(&self) -> Option<SqlDialect> {
        Some(SqlDialect::MySql)
    }

    fn capabilities(&self) -> DbCapabilities {
        DbCapabilities {
            inline_edit: true,
            create_table: true,
            drop_table: true,
            maintenance: false,
            // MySQL's databases are the namespace the studio calls a schema.
            schemas: true,
            edit_table_properties: true,
            row_identity: IDENTITY_COLUMN.to_string(),
            // Spatial types exist but are not managed by this driver.
            geo: false,
            relations: DbRelationStyle::ForeignKey,
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
            // MySQL routines exist, but nothing in the studio reads them yet.
            "functions" => Vec::new(),
            "columns" => match req.table.as_deref() {
                Some(spec) => {
                    let (schema, table) = match spec.find('.') {
                        Some(dot) => (Some(&spec[..dot]), &spec[dot + 1..]),
                        None => (None, spec),
                    };
                    describe_columns_for_table(&pool, schema, table).await?
                }
                None => Vec::new(),
            },
            _ => describe_tree(&pool, schema_filter.as_deref(), include_system).await?,
        };

        Ok(ProjectDbConnectionDescribeResult {
            connection_id: ctx.connection.connection_id.clone(),
            connection_slug: ctx.connection.connection_slug.clone(),
            database_kind: ctx.connection.database_kind.clone(),
            scope,
            capabilities: self.capabilities(),
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
                "PLATFORM_DB_QUERY_EMPTY",
                "query text is empty",
            ));
        }
        let read_only = req.read_only.unwrap_or(true);
        if read_only && !is_read_statement(sql) {
            return Err(PlatformError::new(
                "PLATFORM_MYSQL_QUERY_READ_ONLY",
                "write statement rejected in read-only mode",
            ));
        }
        let limit = req.limit.unwrap_or(DEFAULT_ROW_LIMIT).max(1);
        let pool = connect_pool(ctx).await?;

        let mut query = sqlx::query(sql);
        for param in &req.params {
            query = match param {
                Value::Null => query.bind(Option::<String>::None),
                Value::Bool(b) => query.bind(*b),
                Value::Number(n) if n.is_i64() => query.bind(n.as_i64().unwrap_or_default()),
                Value::Number(n) => query.bind(n.as_f64().unwrap_or_default()),
                Value::String(text) => query.bind(text.clone()),
                other => query.bind(other.to_string()),
            };
        }

        if !is_read_statement(sql) {
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

        let fetched = query
            .fetch_all(&pool)
            .await
            .map_err(|err| PlatformError::new("PLATFORM_DB_QUERY_FAILED", err.to_string()))?;
        let truncated = fetched.len() > limit;
        let columns = fetched.first().map(row_to_columns).unwrap_or_default();
        let rows: Vec<Vec<Value>> = fetched.iter().take(limit).map(row_to_values).collect();

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

    async fn create_table(
        &self,
        ctx: &DbDriverContext,
        req: &CreateSimpleTableRequest,
    ) -> Result<SimpleTableDefinition, PlatformError> {
        let statements = create_table_statements(&req.table, &req.attributes, SqlDialect::MySql)?;
        let table = validate_identifier(&req.table, "table")?;
        let attributes: Vec<CollectionAttribute> = req.attributes.clone();
        let pool = connect_pool(ctx).await?;
        for statement in statements {
            sqlx::query(&statement)
                .execute(&pool)
                .await
                .map_err(|err| PlatformError::new("PLATFORM_DB_DDL_FAILED", err.to_string()))?;
        }
        Ok(SimpleTableDefinition {
            table: table.clone(),
            collection: table,
            attributes,
            ..Default::default()
        })
    }

    async fn alter_table(
        &self,
        ctx: &DbDriverContext,
        table: &str,
        req: &UpdateSimpleTableRequest,
    ) -> Result<SimpleTableDefinition, PlatformError> {
        let (schema, bare) = match table.find('.') {
            Some(dot) => (Some(&table[..dot]), &table[dot + 1..]),
            None => (None, table),
        };
        let name = validate_identifier(bare, "table")?;
        let pool = connect_pool(ctx).await?;
        // Without a qualifier the connection's own database is the namespace.
        let schema = match schema {
            Some(value) => validate_identifier(value, "schema")?,
            None => sqlx::query_scalar::<_, String>("SELECT DATABASE()")
                .fetch_one(&pool)
                .await
                .map_err(|err| {
                    PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string())
                })?,
        };
        let existing = existing_columns(&pool, &schema, &name).await?;
        let statements =
            alter_table_statements(&name, &existing, &req.attributes, SqlDialect::MySql)?;
        for statement in statements {
            sqlx::query(&statement)
                .execute(&pool)
                .await
                .map_err(|err| PlatformError::new("PLATFORM_DB_DDL_FAILED", err.to_string()))?;
        }
        Ok(SimpleTableDefinition {
            table: name.clone(),
            collection: name,
            attributes: req.attributes.clone(),
            ..Default::default()
        })
    }

    async fn insert_empty_row(
        &self,
        ctx: &DbDriverContext,
        table: &str,
    ) -> Result<serde_json::Value, PlatformError> {
        // The table arrives qualified as the tree named it; each part is
        // validated and quoted rather than pasted in.
        let quoted = table
            .split('.')
            .filter(|part| !part.trim().is_empty())
            .map(|part| validate_identifier(part, "table").map(|name| SqlDialect::MySql.quote(&name)))
            .collect::<Result<Vec<_>, _>>()?
            .join(".");
        let pool = connect_pool(ctx).await?;
        let result = sqlx::query(&format!("INSERT INTO {quoted} () VALUES ()"))
            .execute(&pool)
            .await
            .map_err(|err| PlatformError::new("PLATFORM_DB_ROW_FAILED", err.to_string()))?;
        Ok(serde_json::Value::from(result.last_insert_id()))
    }

    async fn drop_table(&self, ctx: &DbDriverContext, table: &str) -> Result<(), PlatformError> {
        let statement = drop_table_statement(table, SqlDialect::MySql)?;
        let pool = connect_pool(ctx).await?;
        sqlx::query(&statement)
            .execute(&pool)
            .await
            .map_err(|err| PlatformError::new("PLATFORM_DB_DDL_FAILED", err.to_string()))?;
        Ok(())
    }
}

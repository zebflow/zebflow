use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use async_trait::async_trait;
use futures::TryStreamExt;
use serde_json::{Map, Value, json};
use sqlx::types::Uuid;
use sqlx::types::chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use sqlx::{Column, Row, TypeInfo, ValueRef, postgres::PgConnectOptions, postgres::PgRow};

use crate::platform::db::driver::{DbDriver, DbDriverContext};
use crate::platform::db::sql_ddl::{
    ExistingColumn, IDENTITY_COLUMN, SqlDialect, alter_table_statements, create_table_statements,
    drop_table_statement, index_name, type_catalog, validate_identifier,
};
use crate::platform::error::PlatformError;
use crate::platform::model::{
    CollectionAttribute, CreateSimpleTableRequest, DbCapabilities, DbObjectNode, DbQueryColumn,
    DbRelationStyle, DbTypeDef, DescribeProjectDbConnectionRequest, ProjectDbConnectionDescribeResult,
    ProjectDbConnectionQueryResult, QueryProjectDbConnectionRequest, SimpleTableDefinition,
    UpdateSimpleTableRequest, slug_segment,
};

#[derive(Default)]
pub struct PostgresqlDbDriver;

#[async_trait]
impl DbDriver for PostgresqlDbDriver {
    fn kind(&self) -> &'static str {
        "postgresql"
    }

    fn sql_dialect(&self) -> Option<SqlDialect> {
        Some(SqlDialect::Postgres)
    }

    fn type_catalog(&self) -> Vec<DbTypeDef> {
        type_catalog(SqlDialect::Postgres)
    }

    fn capabilities(&self) -> DbCapabilities {
        DbCapabilities {
            inline_edit: true,
            create_table: true,
            drop_table: true,
            // No sekejap-style health/compact surface to offer.
            maintenance: false,
            schemas: true,
            // Columns can be altered, but the studio's editor assumes sekejap's
            // attribute model; nothing maps it onto ALTER TABLE yet.
            edit_table_properties: true,
            row_identity: IDENTITY_COLUMN.to_string(),
            // PostGIS may be absent; the grid asks per column rather than
            // assuming the whole connection can hold geometry.
            geo: true,
            relations: DbRelationStyle::ForeignKey,
        }
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
            "columns" => {
                if let Some(table_spec) = req.table.as_deref() {
                    // Parse "schema.table" or just "table" (defaults to "public").
                    let (tbl_schema, tbl_name) = if let Some(dot) = table_spec.find('.') {
                        (&table_spec[..dot], &table_spec[dot + 1..])
                    } else {
                        ("public", table_spec)
                    };
                    describe_columns_for_table(&pool, tbl_schema, tbl_name).await?
                } else {
                    // No table specified — fall back to full table describe.
                    describe_tables(&pool, schema_filter.as_deref(), include_system).await?
                }
            }
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

    async fn create_table(
        &self,
        ctx: &DbDriverContext,
        req: &CreateSimpleTableRequest,
    ) -> Result<SimpleTableDefinition, PlatformError> {
        let statements =
            create_table_statements(&req.table, &req.attributes, SqlDialect::Postgres)?;
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
            Some(dot) => (&table[..dot], &table[dot + 1..]),
            None => ("public", table),
        };
        let schema = validate_identifier(schema, "schema")?;
        let name = validate_identifier(bare, "table")?;
        let pool = connect_pool(ctx).await?;
        let existing = existing_columns(&pool, &schema, &name).await?;
        let statements =
            alter_table_statements(&name, &existing, &req.attributes, SqlDialect::Postgres)?;
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

    async fn insert_row(
        &self,
        ctx: &DbDriverContext,
        table: &str,
        values: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<serde_json::Value, PlatformError> {
        // The table arrives qualified as the tree named it; each part is
        // validated and quoted rather than pasted in.
        let quoted = table
            .split('.')
            .filter(|part| !part.trim().is_empty())
            .map(|part| validate_identifier(part, "table").map(|name| SqlDialect::Postgres.quote(&name)))
            .collect::<Result<Vec<_>, _>>()?
            .join(".");

        let mut columns = Vec::new();
        let mut bound: Vec<Value> = Vec::new();
        for (column, value) in values {
            columns.push(SqlDialect::Postgres.quote(&validate_identifier(column, "column")?));
            bound.push(value.clone());
        }

        let pool = connect_pool(ctx).await?;
        let identity = SqlDialect::Postgres.quote(IDENTITY_COLUMN);
        let statement = if columns.is_empty() {
            format!("INSERT INTO {quoted} DEFAULT VALUES RETURNING {identity}")
        } else {
            let marks: Vec<String> = (1..=columns.len()).map(|i| format!("${i}")).collect();
            format!(
                "INSERT INTO {quoted} ({}) VALUES ({}) RETURNING {identity}",
                columns.join(", "),
                marks.join(", ")
            )
        };
        let mut query = sqlx::query(&statement);
        for value in &bound {
            query = bind_json_param(query, value);
        }
        let row = query
            .fetch_one(&pool)
            .await
            .map_err(|err| PlatformError::new("PLATFORM_DB_ROW_FAILED", err.to_string()))?;
        Ok(row_cell_to_json(&row, 0))
    }

    async fn drop_table(&self, ctx: &DbDriverContext, table: &str) -> Result<(), PlatformError> {
        let statement = drop_table_statement(table, SqlDialect::Postgres)?;
        let pool = connect_pool(ctx).await?;
        sqlx::query(&statement)
            .execute(&pool)
            .await
            .map_err(|err| PlatformError::new("PLATFORM_DB_DDL_FAILED", err.to_string()))?;
        Ok(())
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


/// The table's columns as they exist, with whether this platform's index for
/// each one is present.
async fn existing_columns(
    pool: &sqlx::PgPool,
    schema: &str,
    table: &str,
) -> Result<Vec<ExistingColumn>, PlatformError> {
    let cols = sqlx::query(
        "SELECT column_name, data_type FROM information_schema.columns \
         WHERE table_schema = $1 AND table_name = $2 ORDER BY ordinal_position",
    )
    .bind(schema)
    .bind(table)
    .fetch_all(pool)
    .await
    .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;

    let indexes = sqlx::query(
        "SELECT indexname FROM pg_indexes WHERE schemaname = $1 AND tablename = $2",
    )
    .bind(schema)
    .bind(table)
    .fetch_all(pool)
    .await
    .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;
    let present: Vec<String> = indexes
        .iter()
        .map(|row| row.get::<String, _>("indexname"))
        .collect();

    Ok(cols
        .into_iter()
        .map(|row| {
            let name: String = row.get("column_name");
            let column_type: String = row.get("data_type");
            let indexed = present.contains(&index_name(table, &name));
            ExistingColumn {
                name,
                column_type,
                indexed,
            }
        })
        .collect())
}

async fn describe_schemas(
    pool: &sqlx::PgPool,
    include_system: bool,
) -> Result<Vec<DbObjectNode>, PlatformError> {
    let rows = sqlx::query(
        // PostgreSQL reserves the `pg_` prefix for its own schemas, so
        // `pg_toast` and friends are excluded by the prefix rather than by
        // naming each one.
        "SELECT schema_name FROM information_schema.schemata\n         WHERE ($1::bool OR (schema_name NOT LIKE 'pg\\_%' AND schema_name <> 'information_schema'))\n         ORDER BY schema_name",
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
    // ── 1. Table list ────────────────────────────────────────────────────────
    let table_rows = sqlx::query(
        "SELECT table_schema, table_name \
         FROM information_schema.tables \
         WHERE table_type = 'BASE TABLE' \
           AND ($1::text IS NULL OR table_schema = $1) \
           AND ($2::bool OR table_schema NOT IN ('pg_catalog', 'information_schema')) \
         ORDER BY table_schema, table_name",
    )
    .bind(schema_filter)
    .bind(include_system)
    .fetch_all(pool)
    .await
    .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;

    // ── 2. Columns + primary key membership ──────────────────────────────────
    let col_rows = sqlx::query(
        "SELECT c.table_schema, c.table_name, c.column_name, c.udt_name AS data_type, \
                c.character_maximum_length, c.numeric_precision, c.numeric_scale, \
                c.is_nullable, c.column_default, \
                (pk.column_name IS NOT NULL) AS is_pk \
         FROM information_schema.columns c \
         LEFT JOIN ( \
             SELECT kcu.table_schema, kcu.table_name, kcu.column_name \
             FROM information_schema.table_constraints tc \
             JOIN information_schema.key_column_usage kcu \
                 ON kcu.constraint_name = tc.constraint_name \
                 AND kcu.table_schema = tc.table_schema \
             WHERE tc.constraint_type = 'PRIMARY KEY' \
         ) pk ON pk.table_schema = c.table_schema \
              AND pk.table_name = c.table_name \
              AND pk.column_name = c.column_name \
         WHERE ($1::text IS NULL OR c.table_schema = $1) \
           AND ($2::bool OR c.table_schema NOT IN ('pg_catalog', 'information_schema')) \
         ORDER BY c.table_schema, c.table_name, c.ordinal_position",
    )
    .bind(schema_filter)
    .bind(include_system)
    .fetch_all(pool)
    .await
    .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;

    // ── 3. Foreign key relations ──────────────────────────────────────────────
    let fk_rows = sqlx::query(
        "SELECT kcu.table_schema, kcu.table_name, kcu.column_name, \
                ccu.table_schema AS ref_schema, ccu.table_name AS ref_table, \
                ccu.column_name AS ref_column \
         FROM information_schema.table_constraints tc \
         JOIN information_schema.key_column_usage kcu \
             ON kcu.constraint_name = tc.constraint_name \
             AND kcu.table_schema = tc.table_schema \
         JOIN information_schema.referential_constraints rc \
             ON rc.constraint_name = tc.constraint_name \
         JOIN information_schema.constraint_column_usage ccu \
             ON ccu.constraint_name = rc.unique_constraint_name \
         WHERE tc.constraint_type = 'FOREIGN KEY' \
           AND ($1::text IS NULL OR kcu.table_schema = $1) \
           AND ($2::bool OR kcu.table_schema NOT IN ('pg_catalog', 'information_schema')) \
         ORDER BY kcu.table_schema, kcu.table_name, kcu.column_name",
    )
    .bind(schema_filter)
    .bind(include_system)
    .fetch_all(pool)
    .await
    .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;

    // ── 4. Build FK lookup: (schema, table, column) → ref object ─────────────
    // key: (schema, table, column)
    let mut fk_lookup = BTreeMap::<(String, String, String), Value>::new();
    for row in fk_rows {
        let schema: String = row.get("table_schema");
        let table: String = row.get("table_name");
        let col: String = row.get("column_name");
        let ref_schema: String = row.get("ref_schema");
        let ref_table: String = row.get("ref_table");
        let ref_col: String = row.get("ref_column");
        fk_lookup.insert(
            (schema, table, col),
            json!({ "schema": ref_schema, "table": ref_table, "column": ref_col }),
        );
    }

    // ── 4b. This platform's indexes, so the properties editor round-trips ────
    let index_rows = sqlx::query(
        "SELECT schemaname, tablename, indexname FROM pg_indexes \
         WHERE ($1::text IS NULL OR schemaname = $1)",
    )
    .bind(schema_filter)
    .fetch_all(pool)
    .await
    .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;
    let mut index_lookup = BTreeSet::<(String, String, String)>::new();
    for row in index_rows {
        index_lookup.insert((
            row.get::<String, _>("schemaname"),
            row.get::<String, _>("tablename"),
            row.get::<String, _>("indexname"),
        ));
    }

    // ── 5. Build columns lookup: (schema, table) → Vec<column object> ─────────
    let mut cols_lookup = BTreeMap::<(String, String), Vec<Value>>::new();
    for row in col_rows {
        let schema: String = row.get("table_schema");
        let table: String = row.get("table_name");
        let col_name: String = row.get("column_name");
        let data_type: String = row.get("data_type");
        // Length and precision, so the studio can show `varchar(20)` the way
        // the engine's own tools do.
        let max_len: Option<i32> = row.try_get("character_maximum_length").ok().flatten();
        let precision: Option<i32> = row.try_get("numeric_precision").ok().flatten();
        let scale: Option<i32> = row.try_get("numeric_scale").ok().flatten();
        let full_type = match (max_len, precision, scale) {
            (Some(len), _, _) => format!("{data_type}({len})"),
            (None, Some(p), Some(sc)) if data_type == "numeric" && sc > 0 => {
                format!("{data_type}({p},{sc})")
            }
            _ => data_type.clone(),
        };
        let is_nullable: String = row.get("is_nullable");
        let default: Option<String> = row.try_get("column_default").ok().flatten();
        let is_pk: bool = row.try_get("is_pk").unwrap_or(false);

        let fk = fk_lookup
            .get(&(schema.clone(), table.clone(), col_name.clone()))
            .cloned();

        let mut col = json!({
            "name": col_name,
            // The bare type name, which the picker matches against.
            "type": data_type,
            // The same type as the engine writes it, arguments included.
            "full_type": full_type,
            "nullable": is_nullable == "YES",
        });
        if is_pk {
            col["pk"] = json!(true);
        }
        if index_lookup.contains(&(
            schema.clone(),
            table.clone(),
            index_name(&table, &col_name),
        )) {
            col["indexed"] = json!(true);
        }
        if let Some(fk_ref) = fk {
            col["fk"] = fk_ref;
        }
        if let Some(d) = default {
            col["default"] = json!(d);
        }

        cols_lookup.entry((schema, table)).or_default().push(col);
    }

    // ── 6. Assemble final nodes ───────────────────────────────────────────────
    Ok(table_rows
        .into_iter()
        .filter_map(|row| {
            let schema: String = row.get("table_schema");
            if !include_system && schema.starts_with('_') {
                return None;
            }
            let table: String = row.get("table_name");
            let columns = cols_lookup
                .get(&(schema.clone(), table.clone()))
                .cloned()
                .unwrap_or_default();
            Some(DbObjectNode {
                kind: "table".to_string(),
                name: table,
                schema: Some(schema),
                children: Vec::new(),
                meta: json!({ "columns": columns }),
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

/// Returns column detail for a single table as DbObjectNode items.
/// Each item represents one column with metadata (type, nullable, pk, default, fk).
async fn describe_columns_for_table(
    pool: &sqlx::PgPool,
    schema: &str,
    table: &str,
) -> Result<Vec<DbObjectNode>, PlatformError> {
    let col_rows = sqlx::query(
        "SELECT c.column_name, c.udt_name AS data_type, c.character_maximum_length, \
                c.numeric_precision, c.numeric_scale, c.is_nullable, c.column_default, \
                (pk.column_name IS NOT NULL) AS is_pk, \
                ccu.table_schema AS ref_schema, ccu.table_name AS ref_table, \
                ccu.column_name AS ref_column \
         FROM information_schema.columns c \
         LEFT JOIN ( \
             SELECT kcu.column_name \
             FROM information_schema.table_constraints tc \
             JOIN information_schema.key_column_usage kcu \
                 ON kcu.constraint_name = tc.constraint_name \
                 AND kcu.table_schema = tc.table_schema \
                 AND kcu.table_name = tc.table_name \
             WHERE tc.constraint_type = 'PRIMARY KEY' \
               AND tc.table_schema = $1 AND tc.table_name = $2 \
         ) pk ON pk.column_name = c.column_name \
         LEFT JOIN information_schema.referential_constraints rc \
             ON rc.constraint_name IN ( \
                 SELECT kcu2.constraint_name \
                 FROM information_schema.key_column_usage kcu2 \
                 WHERE kcu2.table_schema = $1 AND kcu2.table_name = $2 \
                   AND kcu2.column_name = c.column_name \
             ) \
         LEFT JOIN information_schema.constraint_column_usage ccu \
             ON ccu.constraint_name = rc.unique_constraint_name \
         WHERE c.table_schema = $1 AND c.table_name = $2 \
         ORDER BY c.ordinal_position",
    )
    .bind(schema)
    .bind(table)
    .fetch_all(pool)
    .await
    .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;

    let mut out = Vec::new();
    for row in col_rows {
        let col_name: String = row.get("column_name");
        let data_type: String = row.get("data_type");
        let is_nullable: String = row.get("is_nullable");
        let default: Option<String> = row.try_get("column_default").ok().flatten();
        let is_pk: bool = row.try_get("is_pk").unwrap_or(false);
        let ref_schema: Option<String> = row.try_get("ref_schema").ok().flatten();
        let ref_table: Option<String> = row.try_get("ref_table").ok().flatten();
        let ref_col: Option<String> = row.try_get("ref_column").ok().flatten();

        let fk = if ref_schema.is_some() && ref_table.is_some() {
            Some(json!({ "schema": ref_schema, "table": ref_table, "column": ref_col }))
        } else {
            None
        };

        let max_len: Option<i32> = row.try_get("character_maximum_length").ok().flatten();
        let precision: Option<i32> = row.try_get("numeric_precision").ok().flatten();
        let scale: Option<i32> = row.try_get("numeric_scale").ok().flatten();
        let full_type = match (max_len, precision, scale) {
            (Some(len), _, _) => format!("{data_type}({len})"),
            (None, Some(p), Some(sc)) if data_type == "numeric" && sc > 0 => {
                format!("{data_type}({p},{sc})")
            }
            _ => data_type.clone(),
        };

        let mut meta = json!({
            "type": data_type,
            "full_type": full_type,
            "nullable": is_nullable == "YES",
            "pk": is_pk,
        });
        if let Some(d) = default {
            meta["default"] = json!(d);
        }
        if let Some(fk_val) = fk {
            meta["fk"] = fk_val;
        }

        out.push(DbObjectNode {
            kind: "column".to_string(),
            name: col_name,
            schema: Some(schema.to_string()),
            children: Vec::new(),
            meta,
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
        "tree" | "schemas" | "tables" | "functions" | "columns" => Ok(normalized),
        _ => Err(PlatformError::new(
            "PLATFORM_DB_DESCRIBE_SCOPE_INVALID",
            "scope must be one of: tree, schemas, tables, functions, columns",
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
        // Bound as a number, not as its text. PostgreSQL checks a parameter's
        // type, so a text-bound 1 is refused by an integer column.
        Value::Number(n) if n.is_i64() => query.bind(n.as_i64().unwrap_or_default()),
        Value::Number(n) => query.bind(n.as_f64().unwrap_or_default()),
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
        return v
            .map(|value| Value::String(value.to_string()))
            .unwrap_or(Value::Null);
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
    // NUMERIC carries more precision than an f64 can hold, so it is returned
    // as its exact digits rather than rounded into a JSON number.
    if let Ok(v) = row.try_get::<Option<sqlx::types::BigDecimal>, _>(idx) {
        return v
            .map(|d| Value::String(d.to_string()))
            .unwrap_or(Value::Null);
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
    // Nothing above could decode this column. Reporting `null` here would be a
    // lie: the reader cannot tell a value this driver does not understand from
    // a column that is genuinely empty. Say which type went unread instead.
    match row.try_get_raw(idx) {
        Ok(raw) if !raw.is_null() => {
            let type_name = raw.type_info().name().to_string();
            Value::String(format!("<unsupported {type_name}>"))
        }
        _ => Value::Null,
    }
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

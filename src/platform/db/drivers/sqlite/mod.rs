//! SQLite runtime driver for the project's own store.
//!
//! Structure is read live from the engine on every call — `sqlite_master` for
//! the object list and `PRAGMA table_info` / `PRAGMA foreign_key_list` for
//! columns and references. Nothing is mirrored, which is what the database
//! schema contract requires.

use async_trait::async_trait;
use rusqlite::Connection;
use rusqlite::types::ValueRef;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

use crate::platform::db::driver::{DbDriver, DbDriverContext};
use crate::platform::error::PlatformError;
use crate::platform::model::{
    DbCapabilities, DbObjectNode, DbQueryColumn, DbRelationStyle,
    DescribeProjectDbConnectionRequest, ProjectDbConnectionDescribeResult,
    ProjectDbConnectionQueryResult, QueryProjectDbConnectionRequest, slug_segment,
};
use crate::platform::sqlite_schema;

pub const DB_KIND: &str = "sqlite";

/// SQLite keeps every object in one namespace, so the studio shows this name
/// where PostgreSQL would show a schema.
const MAIN_SCHEMA: &str = "main";

/// Rows returned to the studio for one query, matching the PostgreSQL driver.
const DEFAULT_ROW_LIMIT: usize = 500;

#[derive(Default)]
pub struct SqliteDbDriver;

async fn run_blocking<T, F>(f: F) -> Result<T, PlatformError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, PlatformError> + Send + 'static,
{
    tokio::task::spawn_blocking(f).await.map_err(|err| {
        PlatformError::new(
            "PLATFORM_SQLITE_TASK_JOIN",
            format!("sqlite blocking task failed: {err}"),
        )
    })?
}

/// Opens the project's store, creating the file on first use.
///
/// `ensure_local_db_migrated` first, because a project created before the
/// storage tiers existed keeps its database at the older path.
fn open_store(data_root: &PathBuf, owner: &str, project: &str) -> Result<Connection, PlatformError> {
    sqlite_schema::ensure_local_db_migrated(data_root, owner, project)?;
    let path = sqlite_schema::local_db_path(data_root, owner, project);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            PlatformError::new("PLATFORM_SQLITE_OPEN", format!("create store dir: {err}"))
        })?;
    }
    Connection::open(&path)
        .map_err(|err| PlatformError::new("PLATFORM_SQLITE_OPEN", err.to_string()))
}

/// Converts one SQLite cell to JSON.
///
/// SQLite stores a value's type per cell rather than per column, so this reads
/// what is actually there rather than what the column was declared as. A blob
/// has no faithful JSON form, so its length is reported instead of its bytes.
fn cell_to_json(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => json!(i),
        ValueRef::Real(f) => json!(f),
        ValueRef::Text(bytes) => json!(String::from_utf8_lossy(bytes).to_string()),
        ValueRef::Blob(bytes) => json!(format!("<blob {} bytes>", bytes.len())),
    }
}

/// Reads every table with its columns and foreign keys.
fn describe_tables(conn: &Connection) -> Result<Vec<DbObjectNode>, PlatformError> {
    let mut stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master \
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;
    let names = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;

    let mut out = Vec::new();
    for table in names {
        out.push(DbObjectNode {
            kind: "table".to_string(),
            name: table.clone(),
            schema: Some(MAIN_SCHEMA.to_string()),
            children: Vec::new(),
            meta: json!({ "columns": table_columns(conn, &table)? }),
        });
    }
    Ok(out)
}

/// Columns of one table, in the same shape the PostgreSQL driver emits so the
/// studio renders both identically.
fn table_columns(conn: &Connection, table: &str) -> Result<Vec<Value>, PlatformError> {
    // PRAGMA takes no bind parameters, so the name is quoted rather than bound.
    let quoted = table.replace('"', "\"\"");

    let mut fk_lookup = BTreeMap::<String, Value>::new();
    let mut fk_stmt = conn
        .prepare(&format!("PRAGMA foreign_key_list(\"{quoted}\")"))
        .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;
    let fk_rows = fk_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>("from")?,
                row.get::<_, String>("table")?,
                row.get::<_, Option<String>>("to")?,
            ))
        })
        .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;
    for row in fk_rows {
        let (from, ref_table, ref_col) = row
            .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;
        fk_lookup.insert(
            from,
            json!({
                "schema": MAIN_SCHEMA,
                "table": ref_table,
                // A reference without an explicit column targets the primary key.
                "column": ref_col.unwrap_or_default(),
            }),
        );
    }

    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info(\"{quoted}\")"))
        .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>("name")?,
                row.get::<_, String>("type")?,
                row.get::<_, i64>("notnull")?,
                row.get::<_, Option<String>>("dflt_value")?,
                row.get::<_, i64>("pk")?,
            ))
        })
        .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;

    let mut columns = Vec::new();
    for row in rows {
        let (name, data_type, notnull, default, pk) = row
            .map_err(|err| PlatformError::new("PLATFORM_DB_DESCRIBE_FAILED", err.to_string()))?;
        let mut col = json!({
            "name": name,
            // A column declared without a type has none in SQLite; report that
            // rather than inventing one.
            "type": if data_type.is_empty() { Value::Null } else { json!(data_type) },
            "nullable": notnull == 0,
        });
        if pk > 0 {
            col["pk"] = json!(true);
        }
        if let Some(fk_ref) = fk_lookup.get(col["name"].as_str().unwrap_or_default()) {
            col["fk"] = fk_ref.clone();
        }
        if let Some(d) = default {
            col["default"] = json!(d);
        }
        columns.push(col);
    }
    Ok(columns)
}

/// The single namespace, carrying its tables as children.
fn describe_tree(conn: &Connection) -> Result<Vec<DbObjectNode>, PlatformError> {
    Ok(vec![DbObjectNode {
        kind: "schema".to_string(),
        name: MAIN_SCHEMA.to_string(),
        schema: None,
        children: describe_tables(conn)?,
        meta: Value::Null,
    }])
}

#[async_trait]
impl DbDriver for SqliteDbDriver {
    fn kind(&self) -> &'static str {
        DB_KIND
    }

    fn capabilities(&self) -> DbCapabilities {
        DbCapabilities {
            // Row edits travel the connection's own query endpoint, so they
            // reach this engine.
            inline_edit: true,
            // Table definition still travels the project `tables` route, which
            // calls `sekejap::create_table` whatever connection is open. Until
            // DDL moves onto the driver, declaring these would offer a button
            // that silently writes a sekejap collection instead of a SQLite
            // table.
            create_table: false,
            drop_table: false,
            // No health, sync or compact surface to offer.
            maintenance: false,
            // Every object lives in one namespace.
            schemas: false,
            // Geometry needs an extension that is not loaded here.
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

        let data_root = ctx.data_root.clone();
        let owner = ctx.owner.clone();
        let project = ctx.project.clone();
        let table = req.table.clone();
        let scope_for_nodes = scope.clone();

        let nodes = run_blocking(move || {
            let conn = open_store(&data_root, &owner, &project)?;
            match scope_for_nodes.as_str() {
                "schemas" => Ok(vec![DbObjectNode {
                    kind: "schema".to_string(),
                    name: MAIN_SCHEMA.to_string(),
                    schema: None,
                    children: Vec::new(),
                    meta: Value::Null,
                }]),
                "tables" => describe_tables(&conn),
                // SQLite exposes no user function catalog.
                "functions" => Ok(Vec::new()),
                "columns" => {
                    let table = table.as_deref().unwrap_or_default();
                    if table.is_empty() {
                        return Ok(Vec::new());
                    }
                    Ok(table_columns(&conn, table)?
                        .into_iter()
                        .map(|col| DbObjectNode {
                            kind: "column".to_string(),
                            name: col
                                .get("name")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                            schema: Some(MAIN_SCHEMA.to_string()),
                            children: Vec::new(),
                            meta: col,
                        })
                        .collect())
                }
                _ => describe_tree(&conn),
            }
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

    async fn query(
        &self,
        ctx: &DbDriverContext,
        req: &QueryProjectDbConnectionRequest,
    ) -> Result<ProjectDbConnectionQueryResult, PlatformError> {
        let started = Instant::now();
        let sql = req.sql.trim().to_string();
        if sql.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_DB_QUERY_EMPTY",
                "query text is empty",
            ));
        }

        let read_only = req.read_only.unwrap_or(true);
        let limit = req.limit.unwrap_or(DEFAULT_ROW_LIMIT).max(1);
        let params: Vec<Value> = req.params.clone();
        let data_root = ctx.data_root.clone();
        let owner = ctx.owner.clone();
        let project = ctx.project.clone();

        let (columns, rows, truncated, affected) = run_blocking(move || {
            let conn = open_store(&data_root, &owner, &project)?;

            // Bind parameters positionally, matching the PostgreSQL driver.
            let bound: Vec<Box<dyn rusqlite::ToSql>> = params
                .iter()
                .map(|p| -> Box<dyn rusqlite::ToSql> {
                    match p {
                        Value::Null => Box::new(Option::<String>::None),
                        Value::Bool(b) => Box::new(*b),
                        Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                Box::new(i)
                            } else {
                                Box::new(n.as_f64().unwrap_or_default())
                            }
                        }
                        // A JSON string binds as its text. `to_string()` would
                        // serialise it back to JSON and store the quotes.
                        Value::String(text) => Box::new(text.clone()),
                        // Arrays and objects have no SQLite type, so they are
                        // stored as the JSON text the caller sent.
                        other => Box::new(other.to_string()),
                    }
                })
                .collect();
            let refs: Vec<&dyn rusqlite::ToSql> = bound.iter().map(|b| b.as_ref()).collect();

            let mut stmt = conn
                .prepare(&sql)
                .map_err(|err| PlatformError::new("PLATFORM_DB_QUERY_FAILED", err.to_string()))?;

            // `readonly` is the engine's own answer about the prepared
            // statement, so a write is refused by what SQLite parsed rather
            // than by matching words in the text.
            if read_only && !stmt.readonly() {
                return Err(PlatformError::new(
                    "PLATFORM_SQLITE_QUERY_READ_ONLY",
                    "write statement rejected in read-only mode",
                ));
            }

            if stmt.column_count() == 0 {
                let affected = stmt.execute(refs.as_slice()).map_err(|err| {
                    PlatformError::new("PLATFORM_DB_QUERY_FAILED", err.to_string())
                })?;
                return Ok((Vec::new(), Vec::new(), false, Some(affected as u64)));
            }

            let names: Vec<String> = stmt.column_names().into_iter().map(String::from).collect();
            let mut result_rows: Vec<Vec<Value>> = Vec::new();
            let mut truncated = false;
            let mut cursor = stmt
                .query(refs.as_slice())
                .map_err(|err| PlatformError::new("PLATFORM_DB_QUERY_FAILED", err.to_string()))?;
            while let Some(row) = cursor
                .next()
                .map_err(|err| PlatformError::new("PLATFORM_DB_QUERY_FAILED", err.to_string()))?
            {
                if result_rows.len() >= limit {
                    truncated = true;
                    break;
                }
                let mut out = Vec::with_capacity(names.len());
                for idx in 0..names.len() {
                    let value = row.get_ref(idx).map_err(|err| {
                        PlatformError::new("PLATFORM_DB_QUERY_FAILED", err.to_string())
                    })?;
                    out.push(cell_to_json(value));
                }
                result_rows.push(out);
            }

            let columns = names
                .into_iter()
                .map(|name| DbQueryColumn {
                    name,
                    // SQLite types belong to values, not columns, so a result
                    // column carries no single declared type.
                    data_type: None,
                })
                .collect::<Vec<_>>();
            Ok((columns, result_rows, truncated, None))
        })
        .await?;

        Ok(ProjectDbConnectionQueryResult {
            connection_id: ctx.connection.connection_id.clone(),
            connection_slug: ctx.connection.connection_slug.clone(),
            database_kind: ctx.connection.database_kind.clone(),
            row_count: rows.len(),
            columns,
            rows,
            truncated,
            affected_rows: affected,
            duration_ms: started.elapsed().as_millis() as u64,
        })
    }
}

//! `sqlite.query.run` — SQL on the project's built-in SQLite database
//! (connection `default`, `data/store/local.db`).
//!
//! Reads and writes are one kind. Without `--write` the database is opened
//! read-only (`SQLITE_OPEN_READ_ONLY`), so SQLite itself refuses any statement
//! that would change it (`SQLITE_READONLY`) and the node answers
//! `FW_NODE_SQLITE_QUERY_RUN_WRITE`. Values bind from `--param`: `1=…` binds
//! `?1`, a name binds `:name`, `@name` or `$name`. The answer is one key,
//! `query` (`shared/query.rs`).

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use rusqlite::{OpenFlags, types::ValueRef};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::model::{LayoutItem, NodeCapability, NodeExample, NodeFieldDef, NodeFieldType};
use crate::pipeline::nodes::shared::query::{self, Params};
use crate::pipeline::nodes::shared::util::{metadata_scope, with_answer};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "sqlite.query.run";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

/// The statement failed, or the database file could not be opened.
pub const CODE: &str = "FW_NODE_SQLITE_QUERY_RUN";
/// A flag the author set wrong.
pub const CONFIG_CODE: &str = "FW_NODE_SQLITE_QUERY_RUN_CONFIG";
/// The project's database could not be moved to its current path.
const MIGRATE_CODE: &str = "FW_NODE_SQLITE_QUERY_RUN_MIGRATE";
/// `--param` keys that do not fit the statement's placeholders.
const PARAM_CODE: &str = "FW_NODE_SQLITE_QUERY_RUN_PARAM";
/// `--limit` outside `1..=5000`.
const LIMIT_CODE: &str = "FW_NODE_SQLITE_QUERY_RUN_LIMIT";
/// A write without `--write`, refused by the read-only connection.
pub const WRITE_CODE: &str = "FW_NODE_SQLITE_QUERY_RUN_WRITE";

const PLACEHOLDERS: &str = "`1=…` binds ?1, `name=…` binds :name (or @name, $name)";

/// Unified node-definition metadata for `sqlite.query.run`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Database, NodeCapability::Process],
        title: "SQLite Query".to_string(),
        description: "Run SQL on the project's built-in SQLite database (connection `default`; no credential). SQL in the body after `--` \
            (or `--query`); values bind from `--param`: `1=…` binds `?1`, `name=…` binds `:name` — a literal is text, a whole `{{ expr }}` \
            keeps its type. The database is opened read-only unless `--write` is set, so an INSERT/UPDATE/DELETE/CREATE without it is \
            refused. Adds `query: { rows, columns, row_count, truncated }` (`input.query.rows[0].name`), plus `rows_affected` with `--write`; \
            `INSERT … RETURNING id` answers the rows it wrote. `--limit` caps the rows (default 200, at most 5000). Standard SQLite DDL \
            and constraints work (unlike Sekejap)."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "description": "Input context — values reach the SQL only through --param."
        }),
        output_schema: query::output_schema(),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            query::query_flag("SQLite"),
            query::param_flag(PLACEHOLDERS),
            query::write_flag(),
            query::limit_flag(),
        ],
        fields: {
            let mut fields = vec![NodeFieldDef {
                name: "query".to_string(),
                label: "Query".to_string(),
                field_type: NodeFieldType::CodeEditor,
                language: Some("sql".to_string()),
                span: Some("full".to_string()),
                help: Some(
                    "SELECT id, title FROM posts WHERE id = ?1\n\
                     Read-only unless Write is ticked; bind values from Params."
                        .to_string(),
                ),
                default_value: Some(json!("SELECT id\nFROM items\nLIMIT 20")),
                ..Default::default()
            }];
            fields.extend(query::fields("1 binds ?1, a name binds :name"));
            fields
        },
        layout: vec![
            LayoutItem::Field("query".to_string()),
            LayoutItem::Field("param".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("limit".to_string()), LayoutItem::Field("write".to_string())] },
        ],
        ai_tool: crate::pipeline::model::NodeAiToolDefinition {
            registered: true,
            tool_name: "sqlite_query".to_string(),
            tool_description:
                "Run a read-only SQL query against the project's embedded SQLite database. \
                Arg: query (required) — SQL SELECT string."
                    .to_string(),
            tool_input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "SQL SELECT query" }
                },
                "required": ["query"]
            }),
        },
        examples: vec![
            NodeExample::dsl("Read with a bound value", r#"sqlite.query.run --param "1={{ $trigger.body.email }}" -- "SELECT id, name FROM users WHERE email = ?1""#)
                .output(json!({ "query": { "rows": [{ "id": 1, "name": "Ana" }], "columns": ["id", "name"], "row_count": 1, "truncated": false } })),
            NodeExample::dsl("Insert from a form", r#"sqlite.query.run --write --param "email={{ $trigger.body.email }}" --param "name={{ $trigger.body.name }}" -- "INSERT INTO users (email, name) VALUES (:email, :name) RETURNING id""#)
                .output(json!({ "query": { "rows": [{ "id": 2 }], "columns": ["id"], "row_count": 1, "truncated": false, "rows_affected": 1 } })),
            NodeExample::dsl("Create a table", r#"sqlite.query.run --write -- "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, email TEXT UNIQUE NOT NULL, name TEXT, created_at TEXT DEFAULT CURRENT_TIMESTAMP)""#)
                .output(json!({ "query": { "rows": [], "columns": [], "row_count": 0, "truncated": false, "rows_affected": 0 } })),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub query: String,
    /// `{ "1": value }` binds `?1`; `{ "name": value }` binds `:name`.
    #[serde(default)]
    pub param: Value,
    /// Let the statement change data.
    #[serde(default)]
    pub write: bool,
    /// Rows answered.
    #[serde(default)]
    pub limit: Value,
}

pub struct Node {
    config: Config,
    data_root: PathBuf,
}

impl Node {
    pub fn new(config: Config, data_root: PathBuf) -> Result<Self, PipelineError> {
        // A `{{ }}` resolving to empty is caught at run time, where the
        // resolved value is known.
        if config.query.trim().is_empty() {
            return Err(PipelineError::new(CONFIG_CODE, "--query (or the body after --) must not be empty"));
        }
        Ok(Self { config, data_root })
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        NODE_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[INPUT_PIN_IN]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let (owner, project, _pipeline, _request_id) = metadata_scope(&input.metadata)?;
        // Arrives final — `{{ }}` resolved engine-side before this node ran
        // (NodeIO §Value resolution).
        let sql = self.config.query.trim().to_string();
        if sql.is_empty() {
            return Err(PipelineError::new(CONFIG_CODE, "--query resolved empty"));
        }
        let params = query::params(&self.config.param, true, PARAM_CODE)?;
        let limit = query::limit(&self.config.limit, LIMIT_CODE)?;
        let write = self.config.write;
        crate::platform::sqlite_schema::ensure_local_db_migrated(&self.data_root, owner, project)
            .map_err(|err| PipelineError::new(MIGRATE_CODE, err.message))?;
        let db_path = crate::platform::sqlite_schema::local_db_path(&self.data_root, owner, project);
        let answer = tokio::task::spawn_blocking(move || run(&db_path, &sql, &params, write, limit))
            .await
            .map_err(|err| PipelineError::new(CODE, format!("task: {err}")))??;
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: with_answer(&input.payload, answer),
            trace: vec![format!("node_kind={NODE_KIND}")],
        })
    }
}

/// One statement against the database at `db_path`: on a read-only
/// connection unless `write`. Answers the `query` key.
pub fn run(db_path: &Path, sql: &str, params: &Params, write: bool, limit: usize) -> Result<Value, PipelineError> {
    let conn = open(db_path, write)?;
    let mut stmt = conn.prepare(sql).map_err(|err| sqlite_error(&err, "prepare"))?;
    bind(&mut stmt, params)?;
    let columns: Vec<String> = stmt.column_names().into_iter().map(str::to_string).collect();
    let mut rows = Vec::new();
    let mut truncated = false;
    {
        let mut cursor = stmt.raw_query();
        while let Some(row) = cursor.next().map_err(|err| sqlite_error(&err, "run"))? {
            if rows.len() < limit {
                let mut object = serde_json::Map::with_capacity(columns.len());
                for (index, name) in columns.iter().enumerate() {
                    let cell = row.get_ref(index).map_err(|err| sqlite_error(&err, "row"))?;
                    object.insert(name.clone(), cell_to_json(cell));
                }
                rows.push(Value::Object(object));
            } else {
                truncated = true;
                // A read stops here; a write steps to its end so every row
                // it changes is changed and counted.
                if !write {
                    break;
                }
            }
        }
    }
    let affected = write.then(|| conn.changes());
    Ok(query::answer(columns, rows, truncated, affected))
}

/// Read-only unless `write`: SQLite refuses every change on a read-only
/// connection itself, whatever the statement looks like. No connection may
/// attach another database: `ATTACH` names any file the process can open, so
/// a statement could otherwise read or write outside the project's database.
fn open(db_path: &Path, write: bool) -> Result<rusqlite::Connection, PipelineError> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| PipelineError::new(CODE, format!("open db: {err}")))?;
    }
    if write || !db_path.exists() {
        // A first read creates the empty database, so it is not "missing".
        let conn = rusqlite::Connection::open(db_path).map_err(|err| PipelineError::new(CODE, format!("open db: {err}")))?;
        conn.execute_batch("PRAGMA journal_mode=WAL;").map_err(|err| PipelineError::new(CODE, format!("pragma: {err}")))?;
        if write {
            return Ok(no_attach(conn));
        }
    }
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    rusqlite::Connection::open_with_flags(db_path, flags)
        .map(no_attach)
        .map_err(|err| PipelineError::new(CODE, format!("open db: {err}")))
}

fn no_attach(conn: rusqlite::Connection) -> rusqlite::Connection {
    conn.set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_ATTACHED, 0);
    conn
}

/// Binds `--param` to the statement's placeholders, refusing a count or a
/// name that does not match.
fn bind(stmt: &mut rusqlite::Statement<'_>, params: &Params) -> Result<(), PipelineError> {
    let wanted = stmt.parameter_count();
    match params {
        Params::Positional(values) => {
            if values.len() != wanted {
                return Err(PipelineError::new(
                    PARAM_CODE,
                    format!("the statement has {wanted} placeholder(s); --param gives {}", values.len()),
                ));
            }
            for (index, value) in values.iter().enumerate() {
                bind_one(stmt, index + 1, value)?;
            }
        }
        Params::Named(values) => {
            if values.len() != wanted {
                return Err(PipelineError::new(
                    PARAM_CODE,
                    format!("the statement has {wanted} placeholder(s); --param names {}", values.len()),
                ));
            }
            for (name, value) in values {
                let index = [":", "@", "$"]
                    .iter()
                    .find_map(|prefix| stmt.parameter_index(&format!("{prefix}{name}")).ok().flatten())
                    .ok_or_else(|| PipelineError::new(PARAM_CODE, format!("the statement has no :{name} placeholder")))?;
                bind_one(stmt, index, value)?;
            }
        }
    }
    Ok(())
}

fn bind_one(stmt: &mut rusqlite::Statement<'_>, index: usize, value: &Value) -> Result<(), PipelineError> {
    let bound = match value {
        Value::Null => stmt.raw_bind_parameter(index, rusqlite::types::Null),
        Value::Bool(b) => stmt.raw_bind_parameter(index, *b),
        Value::Number(n) => match (n.as_i64(), n.as_f64()) {
            (Some(i), _) => stmt.raw_bind_parameter(index, i),
            (None, Some(f)) => stmt.raw_bind_parameter(index, f),
            _ => stmt.raw_bind_parameter(index, n.to_string()),
        },
        Value::String(s) => stmt.raw_bind_parameter(index, s.as_str()),
        other => stmt.raw_bind_parameter(index, other.to_string()),
    };
    bound.map_err(|err| PipelineError::new(PARAM_CODE, format!("bind {index}: {err}")))
}

/// SQLite's own refusal of a write on a read-only connection is the author's
/// missing `--write`; anything else is the statement's failure.
fn sqlite_error(err: &rusqlite::Error, stage: &str) -> PipelineError {
    if let rusqlite::Error::SqliteFailure(code, _) = err {
        if code.code == rusqlite::ErrorCode::ReadOnly {
            return PipelineError::new(WRITE_CODE, query::write_refusal(NODE_KIND, &err.to_string()));
        }
    }
    PipelineError::new(CODE, format!("{stage}: {err}"))
}

fn cell_to_json(v: ValueRef<'_>) -> Value {
    match v {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => json!(i),
        ValueRef::Real(f) => json!(f),
        ValueRef::Text(t) => Value::String(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => Value::String(hex::encode(b)),
    }
}

#[cfg(test)]
mod tests;

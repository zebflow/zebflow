//! `pg.query.run` — SQL on a PostgreSQL database named by a stored credential.
//!
//! Every run is one transaction. Without `--write` it is `SET TRANSACTION
//! READ ONLY`, so Postgres itself refuses a statement that writes (SQLSTATE
//! 25006) and the node answers `FW_NODE_PG_QUERY_RUN_WRITE`; with `--write`
//! it commits. Values bind as `$1, $2, …` from `--param 1=… 2=…`, typed: a
//! whole `{{ }}` keeps its JSON type. The answer is one key, `query`
//! (`shared/query.rs`).

use std::sync::Arc;

use async_trait::async_trait;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sqlx::{Column, Connection, Executor, Row, postgres::PgConnectOptions, postgres::PgRow};

use crate::pipeline::model::{LayoutItem, NodeCapability, NodeExample};
use crate::pipeline::nodes::shared::query::{self, Params};
use crate::pipeline::nodes::shared::util::{metadata_scope, with_answer};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::CredentialService;

pub const NODE_KIND: &str = "pg.query.run";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

/// The statement failed on the server, or the connection did.
pub const CODE: &str = "FW_NODE_PG_QUERY_RUN";
/// A flag the author set wrong.
pub const CONFIG_CODE: &str = "FW_NODE_PG_QUERY_RUN_CONFIG";
/// The credential is missing, not a postgres one, or incomplete.
const CREDENTIAL_CODE: &str = "FW_NODE_PG_QUERY_RUN_CREDENTIAL";
/// The server could not be reached.
const CONNECT_CODE: &str = "FW_NODE_PG_QUERY_RUN_CONNECT";
/// `--param` keys that are not `1..n`.
const PARAM_CODE: &str = "FW_NODE_PG_QUERY_RUN_PARAM";
/// `--limit` outside `1..=5000`.
const LIMIT_CODE: &str = "FW_NODE_PG_QUERY_RUN_LIMIT";
/// A write without `--write`, refused by the read-only transaction.
pub const WRITE_CODE: &str = "FW_NODE_PG_QUERY_RUN_WRITE";

/// SQLSTATE `read_only_sql_transaction`.
const READ_ONLY_SQLSTATE: &str = "25006";

/// Unified node-definition metadata for `pg.query.run`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Network, NodeCapability::Database, NodeCapability::Credential, NodeCapability::Process],
        title: "Postgres Query".to_string(),
        description: "Run SQL on a PostgreSQL database named by `--credential <credential id>` (from `credential_list`, kind postgres — an id, \
             not a connection slug). SQL goes in the body after `--` (or `--query`); values bind as `$1, $2, …` from `--param 1=… --param 2=…` — \
             a literal is text, a whole `{{ expr }}` keeps its type. The statement runs in a read-only transaction unless `--write` is set, so \
             an INSERT/UPDATE/DELETE/DDL without it is refused. Adds `query: { rows, columns, row_count, truncated }` (`input.query.rows[0].id`), \
             plus `rows_affected` with `--write`; add `RETURNING` to get rows back from a write. `--limit` caps the rows (default 200, at most 5000). \
             Never build SQL text from input; bind it. Inside a DSL body write `concat()` rather than `||`, the parser reads `|` as a node separator."
            .to_string(),
        input_schema: json!({
            "type":"object",
            "description":"Input context for query/parameter bindings."
        }),
        output_schema: query::output_schema(),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            crate::pipeline::model::DslFlag {
                flag: "--credential".to_string(),
                config_key: "credential_id".to_string(),
                description: "Credential id of the PostgreSQL connection (from credential_list, kind postgres).".to_string(),
                kind: crate::pipeline::model::DslFlagKind::Scalar,
                required: true,
                value: "text".to_string(),
                ..Default::default()
            },
            query::query_flag("SQL"),
            query::param_flag("`1=…` binds $1, `2=…` binds $2"),
            query::write_flag(),
            query::limit_flag(),
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType, NodeFieldDataSource, SidebarSection, SidebarItem};
            let mut fields = vec![
                NodeFieldDef { name: "credential_id".to_string(), label: "Credential".to_string(), field_type: NodeFieldType::Select, data_source: Some(NodeFieldDataSource::CredentialsPostgres), help: Some("Loaded from project credentials filtered by kind=postgres.".to_string()), ..Default::default() },
                NodeFieldDef {
                    name: "query".to_string(),
                    label: "Query".to_string(),
                    field_type: NodeFieldType::CodeEditor,
                    language: Some("sql".to_string()),
                    span: Some("full".to_string()),
                    help: Some("SQL. Read-only unless Write is ticked; bind values as $1, $2 from Params.".to_string()),
                    default_value: Some(json!("SELECT 1\nAS ok;")),
                    sidebar: vec![
                        SidebarSection {
                            title: "Input payload".to_string(),
                            items: vec![
                                SidebarItem { label: "input".to_string(), type_hint: Some("object".to_string()), description: Some("Upstream payload — bind params from this.".to_string()) },
                            ],
                        },
                        SidebarSection {
                            title: "Output".to_string(),
                            items: vec![
                                SidebarItem { label: "query.rows".to_string(), type_hint: Some("array".to_string()), description: Some("One object per row, keyed by column".to_string()) },
                                SidebarItem { label: "query.rows_affected".to_string(), type_hint: Some("integer".to_string()), description: Some("Rows changed, with Write".to_string()) },
                            ],
                        },
                    ],
                    ..Default::default()
                },
            ];
            fields.extend(query::fields("1 binds $1, 2 binds $2"));
            fields
        },
        layout: vec![
            LayoutItem::Field("credential_id".to_string()),
            LayoutItem::Field("query".to_string()),
            LayoutItem::Field("param".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("limit".to_string()), LayoutItem::Field("write".to_string())] },
        ],
        ai_tool: crate::pipeline::model::NodeAiToolDefinition {
            registered: true,
            tool_name: "database_query".to_string(),
            tool_description: "Execute a read-only SQL query against the configured PostgreSQL database. Args: query (required).".to_string(),
            tool_input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "SQL query to execute" }
                },
                "required": ["query"]
            }),
        },
        examples: vec![
            NodeExample::dsl("Read with a bound value", r#"pg.query.run --credential pg_main --param "1={{ $trigger.auth.sub }}" -- "SELECT id, email FROM accounts WHERE id = $1""#)
                .output(json!({ "query": { "rows": [{ "id": 7, "email": "a@example.com" }], "columns": ["id", "email"], "row_count": 1, "truncated": false } })),
            NodeExample::dsl("Insert and get the id back", r#"pg.query.run --credential pg_main --write --param "1={{ $trigger.body.email }}" -- "INSERT INTO accounts (email) VALUES ($1) RETURNING id""#)
                .output(json!({ "query": { "rows": [{ "id": 8 }], "columns": ["id"], "row_count": 1, "truncated": false, "rows_affected": 1 } })),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// The credential id — a literal or `{{ expr }}`, arriving final.
    pub credential_id: String,
    /// The SQL — a literal or `{{ expr }}`, arriving final.
    #[serde(default)]
    pub query: String,
    /// `{ "1": value, "2": value }`: the values for `$1`, `$2`, …
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
    credentials: Arc<CredentialService>,
}

impl Node {
    pub fn new(
        config: Config,
        credentials: Arc<CredentialService>,
    ) -> Result<Self, PipelineError> {
        // A `{{ }}` that resolves to empty is caught at run time, where the
        // resolved value is known.
        if config.credential_id.trim().is_empty() {
            return Err(PipelineError::new(CONFIG_CODE, "--credential must not be empty"));
        }
        if config.query.trim().is_empty() {
            return Err(PipelineError::new(CONFIG_CODE, "--query (or the body after --) must not be empty"));
        }
        Ok(Self {
            config,
            credentials,
        })
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
        // Both arrive final: `{{ }}` resolved engine-side before this node
        // ran (NodeIO §Value resolution).
        let credential_id = self.config.credential_id.trim();
        let sql = self.config.query.trim();
        if credential_id.is_empty() || sql.is_empty() {
            return Err(PipelineError::new(CONFIG_CODE, "--credential and --query resolved empty"));
        }
        let params = match query::params(&self.config.param, false, PARAM_CODE)? {
            Params::Positional(values) => values,
            Params::Named(_) => return Err(PipelineError::new(PARAM_CODE, "postgres binds positions only: --param 1=…")),
        };
        let limit = query::limit(&self.config.limit, LIMIT_CODE)?;
        let credential = self
            .credentials
            .get_project_credential(owner, project, credential_id)
            .map_err(|err| PipelineError::new(CODE, err.to_string()))?
            .ok_or_else(|| PipelineError::new(CREDENTIAL_CODE, format!("credential '{credential_id}' not found")))?;
        if credential.kind != "postgres" {
            return Err(PipelineError::new(
                CREDENTIAL_CODE,
                format!("credential '{}' is '{}' not 'postgres'", credential.credential_id, credential.kind),
            ));
        }
        let options = build_postgres_connect_options(&credential.secret)?;
        let answer = run(options, sql, &params, self.config.write, limit).await?;
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: with_answer(&input.payload, answer),
            trace: vec![format!("node_kind={NODE_KIND}")],
        })
    }
}

/// One statement in one transaction: read-only unless `write`, committed
/// only when `write`. Answers the `query` key.
pub async fn run(
    options: PgConnectOptions,
    sql: &str,
    params: &[Value],
    write: bool,
    limit: usize,
) -> Result<Value, PipelineError> {
    let mut conn = sqlx::postgres::PgConnection::connect_with(&options)
        .await
        .map_err(|err| PipelineError::new(CONNECT_CODE, err.to_string()))?;
    let mut tx = conn.begin().await.map_err(query_error)?;
    if !write {
        // Postgres' own read-only mode for this transaction: every write
        // the statement attempts — DML, DDL, a function that writes — is
        // refused by the server, not guessed at from the SQL text.
        (&mut *tx).execute("SET TRANSACTION READ ONLY").await.map_err(query_error)?;
    }
    let mut statement = sqlx::query(sql);
    for param in params {
        statement = bind_json_param(statement, param);
    }
    let mut columns: Option<Vec<String>> = None;
    let mut rows = Vec::new();
    let mut truncated = false;
    let mut affected = 0u64;
    {
        let mut stream = (&mut *tx).fetch_many(statement);
        while let Some(item) = stream.try_next().await.map_err(query_error)? {
            match item {
                sqlx::Either::Left(done) => affected += done.rows_affected(),
                sqlx::Either::Right(row) => {
                    if columns.is_none() {
                        columns = Some(row.columns().iter().map(|c| c.name().to_string()).collect());
                    }
                    if rows.len() < limit {
                        rows.push(row_to_json(row));
                    } else {
                        truncated = true;
                        // A read stops here; a write runs to its end so it
                        // is committed whole and counted.
                        if !write {
                            break;
                        }
                    }
                }
            }
        }
    }
    if write {
        tx.commit().await.map_err(query_error)?;
    } else {
        tx.rollback().await.map_err(query_error)?;
    }
    Ok(query::answer(columns.unwrap_or_default(), rows, truncated, write.then_some(affected)))
}

/// A server refusal of a write in the read-only transaction is the author's
/// missing `--write`; anything else is the statement's own failure.
fn query_error(err: sqlx::Error) -> PipelineError {
    let read_only = err
        .as_database_error()
        .and_then(|db| db.code())
        .is_some_and(|code| code == READ_ONLY_SQLSTATE);
    if read_only {
        return PipelineError::new(WRITE_CODE, query::write_refusal(NODE_KIND, &err.to_string()));
    }
    PipelineError::new(CODE, err.to_string())
}

fn build_postgres_connect_options(secret: &Value) -> Result<PgConnectOptions, PipelineError> {
    let host = secret
        .get("host")
        .and_then(Value::as_str)
        .ok_or_else(|| PipelineError::new(CREDENTIAL_CODE, "secret.host is required"))?;
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
    let port = u16::try_from(port)
        .map_err(|_| PipelineError::new(CREDENTIAL_CODE, "secret.port must be in 0..=65535"))?;
    let database = secret
        .get("database")
        .and_then(Value::as_str)
        .ok_or_else(|| PipelineError::new(CREDENTIAL_CODE, "secret.database is required"))?;
    let user = secret
        .get("user")
        .and_then(Value::as_str)
        .ok_or_else(|| PipelineError::new(CREDENTIAL_CODE, "secret.user is required"))?;
    let password = secret
        .get("password")
        .and_then(Value::as_str)
        .ok_or_else(|| PipelineError::new(CREDENTIAL_CODE, "secret.password is required"))?;
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
    // Bind owned values so query lifetime never depends on input payload references.
    match value {
        Value::Null => query.bind(Option::<String>::None),
        Value::Bool(v) => query.bind(*v),
        // A number keeps its type: an integer binds as int8, anything else
        // as float8; Postgres casts either to the column it meets.
        Value::Number(n) => match n.as_i64() {
            Some(i) => query.bind(i),
            None => query.bind(n.as_f64()),
        },
        Value::String(s) => query.bind(s.clone()),
        other => query.bind(other.to_string()),
    }
}

fn row_to_json(row: PgRow) -> Value {
    let mut map = Map::new();
    for (idx, column) in row.columns().iter().enumerate() {
        map.insert(column.name().to_string(), row_cell_to_json(&row, idx));
    }
    Value::Object(map)
}

fn row_cell_to_json(row: &PgRow, idx: usize) -> Value {
    if let Ok(v) = row.try_get::<Option<serde_json::Value>, _>(idx) {
        return v.unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<String>, _>(idx) {
        return v.map(Value::String).unwrap_or(Value::Null);
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
    // Time, identity, exact numbers, arrays and points — each one used to fall
    // through to Null, so a `created_at` read as "no value" and a page showed
    // nothing where the database had a date.
    if let Ok(v) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(idx) {
        return v.map(|t| Value::String(t.to_rfc3339())).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<chrono::NaiveDateTime>, _>(idx) {
        return v.map(|t| Value::String(t.format("%Y-%m-%dT%H:%M:%S%.f").to_string())).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<chrono::NaiveDate>, _>(idx) {
        return v.map(|d| Value::String(d.to_string())).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<chrono::NaiveTime>, _>(idx) {
        return v.map(|t| Value::String(t.to_string())).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<uuid::Uuid>, _>(idx) {
        return v.map(|u| Value::String(u.to_string())).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<sqlx::types::BigDecimal>, _>(idx) {
        // A numeric keeps its digits as a string; a float would round it.
        return v.map(|n| Value::String(n.to_string())).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<Vec<String>>, _>(idx) {
        return v.map(|xs| Value::Array(xs.into_iter().map(Value::String).collect())).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<Vec<i64>>, _>(idx) {
        return v.map(|xs| json!(xs)).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<Vec<i32>>, _>(idx) {
        return v.map(|xs| json!(xs)).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<Vec<f64>>, _>(idx) {
        return v.map(|xs| json!(xs)).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<sqlx::postgres::types::PgPoint>, _>(idx) {
        return v.map(|p| json!([p.x, p.y])).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(idx) {
        return v
            .map(|bytes| Value::String(hex::encode(bytes)))
            .unwrap_or(Value::Null);
    }
    // A type none of the above decode: say so once in the log rather than
    // hand the page a Null that looks like data.
    eprintln!(
        "[pg.query.run] column {} has a type this node does not decode ({}); cast it in SQL (::text, to_json)",
        row.columns()[idx].name(),
        row.columns()[idx].type_info()
    );
    Value::Null
}

#[cfg(test)]
mod tests;

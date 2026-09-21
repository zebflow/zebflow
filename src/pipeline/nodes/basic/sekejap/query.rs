//! Sekejap query node — SQL against the project's embedded Sekejap store.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::nodes::shared::util::metadata_scope;
use crate::pipeline::model::NodeCapability;
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::sekejap;

pub const NODE_KIND: &str = "n.sekejap.query";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Database, NodeCapability::Process],
        title: "Sekejap Query".to_string(),
        description:
            "Run SQL on the project's built-in database (connection `default-multimodel`; no credential, no setup). SQL goes in the body \
             after `--`, values in `--params` as `$1, $2, …`; writes need `--read-only false`. A read answers \
             `{ columns, rows, row_count, truncated }` with each row an object keyed by column (`input.rows[0].title`); a write answers \
             `{ affected_rows }`. Sekejap's DDL is its own dialect — `CREATE TABLE t (_key TEXT PRIMARY KEY DEFAULT UUIDV4(), name TEXT) WITH (hash: ['name'])`, \
             no NOT NULL/UNIQUE/REFERENCES/DEFAULT NOW(); see help topic `db/sekejap`. An unknown table or column fails at run time, not at register."
                .to_string(),
        input_schema: json!({
            "type": "object",
            "description": "Input context for query and $1/$2 bind parameter expressions."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "columns": { "type": "array" },
                "rows": { "type": "array" },
                "row_count": { "type": "integer" },
                "affected_rows": { "type": ["integer", "null"] },
                "duration_ms": { "type": "integer" }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag {
                flag: "--query".to_string(),
                config_key: "query".to_string(),
                description: "Sekejap SQL (alternative to body `-- \"SELECT ...\"`)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--limit".to_string(),
                config_key: "limit".to_string(),
                description: "Maximum rows to return for read queries.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--read-only".to_string(),
                config_key: "read_only".to_string(),
                description: "Reject write statements when true.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--params".to_string(),
                config_key: "params".to_string(),
                description: "Bind values for $1, $2, … — a literal or {{ expr }}. A whole {{ }} carries its typed value, so \"{{ [$trigger.params.slug] }}\" is a real array.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "query".to_string(),
                label: "Query".to_string(),
                field_type: NodeFieldType::CodeEditor,
                language: Some("sql".to_string()),
                span: Some("full".to_string()),
                help: Some(
                    "SELECT * FROM posts WHERE slug = $1\nINSERT INTO posts (_key, title) VALUES ($1, $2)"
                        .to_string(),
                ),
                default_value: Some(json!("SELECT *\nFROM items\nLIMIT 20")),
                ..Default::default()
            },
            NodeFieldDef {
                name: "params".to_string(),
                label: "Params".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Bind values for $1, $2, … — a literal or {{ expr }}.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "limit".to_string(),
                label: "Limit".to_string(),
                field_type: NodeFieldType::Number,
                help: Some("Maximum rows returned for read queries.".to_string()),
                default_value: Some(json!(200)),
                ..Default::default()
            },
            NodeFieldDef {
                name: "read_only".to_string(),
                label: "Read Only".to_string(),
                field_type: NodeFieldType::Checkbox,
                help: Some("When enabled, INSERT/UPDATE/DELETE/CREATE statements are rejected."
                    .to_string()),
                default_value: Some(json!(false)),
                ..Default::default()
            },
        ],
        layout: vec![
            LayoutItem::Field("query".to_string()),
            LayoutItem::Field("params".to_string()),
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("limit".to_string()),
                    LayoutItem::Field("read_only".to_string()),
                ],
            },
        ],
        ai_tool: crate::pipeline::model::NodeAiToolDefinition {
            registered: true,
            tool_name: "sekejap_query".to_string(),
            tool_description:
                "Execute SQL against the project's embedded Sekejap store. Args: query (required), params for $1/$2 binds, limit (optional), read_only (optional)."
                    .to_string(),
            tool_input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Sekejap SQL query" },
                    "params": { "description": "Bind values for $1, $2, … — a literal or {{ expr }}" },
                    "limit": { "type": "integer", "description": "Maximum rows to return" },
                    "read_only": { "type": "boolean", "description": "Reject write statements when true" }
                }
            }),
        },
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Read with a bound value", r#"sekejap.query --params "{{ [$trigger.params.slug] }}" -- "SELECT _key, title, body FROM posts WHERE slug = $1""#)
                .output(serde_json::json!({ "columns": ["_key", "title", "body"], "rows": [{ "_key": "8c1…", "title": "Hello", "body": "…" }], "row_count": 1, "truncated": false })),
            crate::pipeline::model::NodeExample::dsl("Insert from a form", r#"sekejap.query --read-only false --params "{{ [input.body.title, input.body.slug, new Date().toISOString()] }}" -- "INSERT INTO posts (title, slug, created_at) VALUES ($1, $2, $3)""#)
                .output(serde_json::json!({ "affected_rows": 1 })),
            crate::pipeline::model::NodeExample::dsl("Create a table", r#"sekejap.query --read-only false -- "CREATE TABLE posts (_key TEXT PRIMARY KEY DEFAULT UUIDV4(), title TEXT, slug TEXT, created_at TIMESTAMPTZ) WITH (hash: ['slug'], range: ['created_at'])""#)
                .note("Run once from `pipeline_run` or a `jobs/migrate` function pipeline; keep the SQL in `db/001_posts.sql`."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub query: String,
    /// Bind values for `$1`, `$2`, … A whole `{{ }}` carries its typed value.
    #[serde(default)]
    pub params: Value,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub read_only: bool,
}

fn default_limit() -> usize {
    200
}

pub struct Node {
    config: Config,
    data_root: PathBuf,
}

impl Node {
    pub fn new(
        config: Config,
        data_root: PathBuf,
    ) -> Result<Self, PipelineError> {
        // One flag per thing now, so "set one or the other" has nothing left
        // to adjudicate.
        if config.query.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_SEKEJAP_QUERY_CONFIG",
                "config.query must not be empty",
            ));
        }
        Ok(Self {
            config,
            data_root,
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
        // Arrives final — `{{ }}` resolved engine-side before this ran.
        let query = self.config.query.trim().to_string();
        if query.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_SEKEJAP_QUERY",
                "query must not be empty — use: -- \"SELECT ...\"",
            ));
        }
        // Bind values arrive final: a whole `{{ }}` is a real array.
        let param_values: Vec<Value> = match self.config.params.clone() {
            Value::Null => Vec::new(),
            Value::Array(items) => items,
            other => vec![other],
        };

        let data_root = self.data_root.clone();
        let owner = owner.to_string();
        let project = project.to_string();
        let limit = self.config.limit;
        let read_only = self.config.read_only;
        let sql = query.clone();
        let result = tokio::task::spawn_blocking(move || {
            sekejap::execute_sql(
                &data_root,
                &owner,
                &project,
                &sql,
                &param_values,
                limit,
                read_only,
            )
        })
        .await
        .map_err(|err| {
            PipelineError::new(
                "FW_NODE_SEKEJAP_QUERY_JOIN",
                format!("sekejap query task failed: {err}"),
            )
        })?
        .map_err(|err| PipelineError::new("FW_NODE_SEKEJAP_QUERY", with_ddl_hint(&query, err.to_string())))?;

        // The store answers positionally (columns + value arrays — the shape
        // the DB pages render). A pipeline reads `input.rows[0].title`, as it
        // does after pg.query and sqlite.query, so each row is keyed by its
        // column name here; the positional `columns` list stays beside it.
        let rows = rows_as_objects(&result.columns, &result.rows);
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({
                "columns": result.columns,
                "rows": rows,
                "row_count": result.row_count,
                "truncated": result.truncated,
                "affected_rows": result.affected_rows,
                "duration_ms": result.duration_ms,
            }),
            trace: vec![
                format!("node_kind={NODE_KIND}"),
                format!("row_count={}", result.row_count),
            ],
        })
    }
}

/// A `CREATE TABLE` that fails to parse is nearly always SQL-dialect DDL —
/// `NOT NULL`, `UNIQUE`, `REFERENCES`, `DEFAULT NOW()` — which Sekejap's
/// grammar does not have. Every agent tried each of those before reading the
/// grammar, so the error says where the grammar is.
fn with_ddl_hint(query: &str, message: String) -> String {
    let upper = query.trim_start().to_ascii_uppercase();
    if !upper.starts_with("CREATE TABLE") {
        return message;
    }
    format!(
        "{message} — Sekejap DDL: `CREATE TABLE t (_key TEXT PRIMARY KEY DEFAULT UUIDV4(), name TEXT, created_at TIMESTAMPTZ) WITH (hash: ['name'])`; \
         no NOT NULL / UNIQUE / REFERENCES / DEFAULT NOW() (help topic db/sekejap)"
    )
}

/// One object per row, keyed by column name. A duplicate column name (a join
/// selecting `id` twice) keeps the last value, as pg.query does; alias in SQL
/// when both are wanted.
fn rows_as_objects(
    columns: &[crate::platform::model::DbQueryColumn],
    rows: &[Vec<Value>],
) -> Vec<Value> {
    rows.iter()
        .map(|row| {
            let mut object = serde_json::Map::with_capacity(columns.len());
            for (index, column) in columns.iter().enumerate() {
                object.insert(
                    column.name.clone(),
                    row.get(index).cloned().unwrap_or(Value::Null),
                );
            }
            Value::Object(object)
        })
        .collect()
}

#[cfg(test)]
mod row_shape_tests {
    use super::{rows_as_objects, with_ddl_hint};
    use crate::platform::model::DbQueryColumn;
    use serde_json::json;

    /// The store's positional rows become `input.rows[0].name`, which is what
    /// every pipeline, page and doc reads — and what pg.query and sqlite.query
    /// already deliver.
    #[test]
    fn a_failed_create_table_points_at_the_grammar() {
        let hinted = with_ddl_hint("CREATE TABLE users (email TEXT NOT NULL)", "parse error".into());
        assert!(hinted.contains("db/sekejap"), "{hinted}");
        assert!(hinted.contains("UUIDV4()"), "{hinted}");
        assert_eq!(with_ddl_hint("SELECT 1", "parse error".into()), "parse error");
    }

    #[test]
    fn rows_are_objects_keyed_by_column_name() {
        let columns = vec![
            DbQueryColumn { name: "_key".into(), data_type: None },
            DbQueryColumn { name: "name".into(), data_type: None },
        ];
        let rows = vec![vec![json!("k1"), json!("Alice")], vec![json!("k2")]];
        let out = rows_as_objects(&columns, &rows);
        assert_eq!(out[0], json!({ "_key": "k1", "name": "Alice" }));
        assert_eq!(out[1], json!({ "_key": "k2", "name": null }));
    }
}

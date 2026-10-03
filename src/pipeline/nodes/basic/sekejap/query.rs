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
             `{ affected_rows }`; an `INSERT … RETURNING _key` answers the rows it wrote. Every row has a `_key`: name it in the \
             INSERT, or declare `_key TEXT PRIMARY KEY DEFAULT ulid()` and read the minted key back with `RETURNING _key`. A plain \
             INSERT never overwrites (a taken key is 23505); upsert with `ON CONFLICT (_key) DO UPDATE SET c = EXCLUDED.c`. \
             Sekejap's SQL is PostgreSQL's outside graphs: `LIKE`/`ILIKE`, `UNIQUE`, `REFERENCES`, `DEFAULT`, `NOT NULL`; relations \
             between rows are graph walks, not JOINs — `GRAPH_TABLE (g MATCH (a)-[e:knows]->(b) RETURN b.name AS name)`; pages \
             continue after the last row (`WHERE (name, _key) > ($1, $2) ORDER BY name, _key LIMIT 20`), there is no OFFSET. See help topic \
             `db/sekejap`. An unknown table or column fails at run time, not at register."
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
                ..Default::default()
            },
            DslFlag {
                flag: "--limit".to_string(),
                config_key: "limit".to_string(),
                description: "Maximum rows to return for read queries.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
            DslFlag {
                flag: "--read-only".to_string(),
                config_key: "read_only".to_string(),
                description: "Reject write statements when true.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
            DslFlag {
                flag: "--params".to_string(),
                config_key: "params".to_string(),
                description: "Bind values for $1, $2, … — a literal or {{ expr }}. A whole {{ }} carries its typed value, so \"{{ [$trigger.params.slug] }}\" is a real array.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                ..Default::default()
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
                    "SELECT _key, title FROM posts WHERE slug = $1\nINSERT INTO posts (title) VALUES ($1) RETURNING _key — with _key DEFAULT ulid()"
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
                .output(serde_json::json!({ "columns": ["_key", "title", "body"], "rows": [{ "_key": "hello-world", "title": "Hello", "body": "…" }], "row_count": 1, "truncated": false })),
            crate::pipeline::model::NodeExample::dsl("Insert from a form, learning the new key", r#"sekejap.query --read-only false --params "{{ [input.body.title] }}" -- "INSERT INTO posts (title) VALUES ($1) RETURNING _key""#)
                .output(serde_json::json!({ "columns": ["_key"], "rows": [{ "_key": "01M3JSKZFJ4AZGVR0PG6ZXMDH5" }], "row_count": 1, "truncated": false, "affected_rows": 1 }))
                .note("The table declares `_key TEXT PRIMARY KEY DEFAULT ulid()`, so the INSERT leaves the key out and `RETURNING` hands it back — `input.rows[0]._key` for a redirect."),
            crate::pipeline::model::NodeExample::dsl("Upsert", r#"sekejap.query --read-only false --params "{{ [input.body.slug, input.body.title] }}" -- "INSERT INTO posts (_key, title) VALUES ($1, $2) ON CONFLICT (_key) DO UPDATE SET title = EXCLUDED.title""#)
                .output(serde_json::json!({ "affected_rows": 1 }))
                .note("A plain INSERT of a key that exists is refused with 23505; ON CONFLICT is the upsert."),
            crate::pipeline::model::NodeExample::dsl("Create a table", r#"sekejap.query --read-only false -- "CREATE TABLE posts (_key TEXT PRIMARY KEY DEFAULT ulid(), title TEXT NOT NULL, slug TEXT UNIQUE, status TEXT DEFAULT 'draft', created_at TIMESTAMPTZ DEFAULT now()) WITH (fulltext: [title])""#)
                .note("Run once from `pipeline_run` or a `jobs/migrate` function pipeline; keep the SQL in `db/001_posts.sql`. Scalar columns are indexed automatically; `WITH` declares full-text, spatial and vector indexes."),
            crate::pipeline::model::NodeExample::dsl("Search by part of a name", r#"sekejap.query --params "{{ ['%' + input.query.q + '%'] }}" -- "SELECT _key, name FROM members WHERE name ILIKE $1 ORDER BY name LIMIT 20""#)
                .output(serde_json::json!({ "columns": ["_key", "name"], "rows": [{ "_key": "m1", "name": "Alex Doe" }], "row_count": 1, "truncated": false })),
            crate::pipeline::model::NodeExample::dsl("Newest first, one page at a time", r#"sekejap.query --params "{{ [input.query.after || '~'] }}" -- "SELECT _key, title FROM posts WHERE _key < $1 ORDER BY _key DESC LIMIT 20""#)
                .output(serde_json::json!({ "columns": ["_key", "title"], "rows": [{ "_key": "01M3JSKZFM6MM3GJFF0WKN71DD", "title": "Latest" }], "row_count": 1, "truncated": false }))
                .note("ULID keys sort by time, so `ORDER BY _key DESC` is newest first. The next page passes the last `_key` shown as `after`; there is no OFFSET."),
            crate::pipeline::model::NodeExample::dsl("Add an edge", r#"sekejap.query --read-only false --params "{{ [input.body.member, input.body.institution, input.body.position] }}" -- "INSERT INTO affiliated_with (member_id, institution_id, position) VALUES ($1, $2, $3)""#)
                .output(serde_json::json!({ "affected_rows": 1 }))
                .note("`affiliated_with` is an edge table: a table with two REFERENCES columns declared in `CREATE PROPERTY GRAPH … EDGE TABLES`. UPDATE and DELETE name an end: `DELETE FROM affiliated_with WHERE member_id = $1 AND institution_id = $2`."),
            crate::pipeline::model::NodeExample::dsl("Walk the graph", r#"sekejap.query --params "{{ [$trigger.params.id] }}" -- "SELECT name, position FROM GRAPH_TABLE (network MATCH (m WHERE m._key = $1)-[a:affiliated_with]->(i) RETURN i.name AS name, a.position AS position)""#)
                .output(serde_json::json!({ "columns": ["name", "position"], "rows": [{ "name": "University One", "position": "Lecturer" }], "row_count": 1, "truncated": false }))
                .note("Name the edge (`[a:affiliated_with]`) to read its properties. Each matching path is one row; `RETURN DISTINCT` returns a node once."),
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
            payload: crate::pipeline::nodes::shared::util::with_answer(&input.payload, json!({
                "columns": result.columns,
                "rows": rows,
                "row_count": result.row_count,
                "truncated": result.truncated,
                "affected_rows": result.affected_rows,
                "duration_ms": result.duration_ms,
            })),
            trace: vec![
                format!("node_kind={NODE_KIND}"),
                format!("row_count={}", result.row_count),
            ],
        })
    }
}

/// A failed `CREATE TABLE` points at the grammar: which column clauses
/// sekejap takes, and the one it does not.
fn with_ddl_hint(query: &str, message: String) -> String {
    let upper = query.trim_start().to_ascii_uppercase();
    if !upper.starts_with("CREATE TABLE") {
        return message;
    }
    format!(
        "{message} — Sekejap DDL: `CREATE TABLE t (_key TEXT PRIMARY KEY DEFAULT ulid(), name TEXT NOT NULL, email TEXT UNIQUE, \
         status TEXT DEFAULT 'draft', owner_id TEXT REFERENCES users, embedding VECTOR(384)) WITH (fulltext: [name], vector: [embedding])`; \
         a VECTOR needs its dimension and CHECK is not supported; an edge table is declared with CREATE PROPERTY GRAPH (help topic db/sekejap)"
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
        let hinted = with_ddl_hint("CREATE TABLE users (age INT CHECK (age > 0))", "parse error".into());
        assert!(hinted.contains("db/sekejap"), "{hinted}");
        assert!(hinted.contains("VECTOR(384)"), "{hinted}");
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

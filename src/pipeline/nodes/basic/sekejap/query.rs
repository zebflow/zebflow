//! `sekejap.query.run` — SQL on the project's built-in Sekejap store
//! (connection `default-multimodel`).
//!
//! Without `--write` the statement runs read-only: `execute_sql` refuses a
//! write statement up front and runs the rest on the store's published
//! read-only snapshot, so the engine itself refuses any write that gets that
//! far (`Error::ReadOnly`); either answers `FW_NODE_SEKEJAP_QUERY_RUN_WRITE`.
//! Values bind as `$1, $2, …` from `--param 1=… 2=…`. The answer is one key,
//! `query` (`shared/query.rs`).

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::model::{LayoutItem, NodeCapability, NodeExample, NodeFieldDef, NodeFieldType};
use crate::pipeline::nodes::shared::query::{self, Params};
use crate::pipeline::nodes::shared::util::{metadata_scope, with_answer};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::sekejap;

pub const NODE_KIND: &str = "sekejap.query.run";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

/// The statement failed in the store.
pub const CODE: &str = "FW_NODE_SEKEJAP_QUERY_RUN";
/// A flag the author set wrong.
pub const CONFIG_CODE: &str = "FW_NODE_SEKEJAP_QUERY_RUN_CONFIG";
/// `--param` keys that are not `1..n`.
const PARAM_CODE: &str = "FW_NODE_SEKEJAP_QUERY_RUN_PARAM";
/// `--limit` outside `1..=5000`.
const LIMIT_CODE: &str = "FW_NODE_SEKEJAP_QUERY_RUN_LIMIT";
/// A write without `--write`.
pub const WRITE_CODE: &str = "FW_NODE_SEKEJAP_QUERY_RUN_WRITE";

pub fn definition() -> NodeDefinition {
    let answer = |columns: &[&str], rows: Value, affected: Option<u64>| {
        let n = rows.as_array().map_or(0, Vec::len);
        let mut query = json!({ "rows": rows, "columns": columns, "row_count": n, "truncated": false });
        if let Some(affected) = affected {
            query["rows_affected"] = json!(affected);
        }
        json!({ "query": query })
    };
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Database, NodeCapability::Process],
        title: "Sekejap Query".to_string(),
        description:
            "Run SQL on the project's built-in database (connection `default-multimodel`; no credential, no setup). SQL goes in the body \
             after `--` (or `--query`), values bind as `$1, $2, …` from `--param 1=… --param 2=…` — a literal is text, a whole `{{ expr }}` \
             keeps its type. Reads only unless `--write` is set: a write without it is refused. Adds `query: { rows, columns, row_count, \
             truncated }` with each row an object keyed by column (`input.query.rows[0].title`), plus `rows_affected` with `--write`; an \
             `INSERT … RETURNING _key` answers the rows it wrote. `--limit` caps the rows (default 200, at most 5000). Every row has a \
             `_key`: name it in the INSERT, or declare `_key TEXT PRIMARY KEY DEFAULT ulid()` and read the minted key back with \
             `RETURNING _key`. A plain INSERT never overwrites (a taken key is 23505); upsert with `ON CONFLICT (_key) DO UPDATE SET c = \
             EXCLUDED.c`. Sekejap's SQL is PostgreSQL's outside graphs: `LIKE`/`ILIKE`, `UNIQUE`, `REFERENCES`, `DEFAULT`, `NOT NULL`; \
             relations between rows are graph walks, not JOINs — `GRAPH_TABLE (g MATCH (a)-[e:knows]->(b) RETURN b.name AS name)`; pages \
             continue after the last row (`WHERE (name, _key) > ($1, $2) ORDER BY name, _key LIMIT 20`), there is no OFFSET. See help topic \
             `db/sekejap`. An unknown table or column fails at run time, not at register."
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
            query::query_flag("Sekejap SQL"),
            query::param_flag("`1=…` binds $1, `2=…` binds $2"),
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
                    "SELECT _key, title FROM posts WHERE slug = $1\nINSERT INTO posts (title) VALUES ($1) RETURNING _key — with _key DEFAULT ulid() and Write ticked"
                        .to_string(),
                ),
                default_value: Some(json!("SELECT *\nFROM items\nLIMIT 20")),
                ..Default::default()
            }];
            fields.extend(query::fields("1 binds $1, 2 binds $2"));
            fields
        },
        layout: vec![
            LayoutItem::Field("query".to_string()),
            LayoutItem::Field("param".to_string()),
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("limit".to_string()),
                    LayoutItem::Field("write".to_string()),
                ],
            },
        ],
        ai_tool: crate::pipeline::model::NodeAiToolDefinition {
            registered: true,
            tool_name: "sekejap_query".to_string(),
            tool_description:
                "Execute read-only SQL against the project's embedded Sekejap store. Args: query (required), limit (optional)."
                    .to_string(),
            tool_input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Sekejap SQL query" },
                    "limit": { "type": "integer", "description": "Maximum rows to return" }
                },
                "required": ["query"]
            }),
        },
        examples: vec![
            NodeExample::dsl("Read with a bound value", r#"sekejap.query.run --param "1={{ $trigger.params.slug }}" -- "SELECT _key, title, body FROM posts WHERE slug = $1""#)
                .output(answer(&["_key", "title", "body"], json!([{ "_key": "hello-world", "title": "Hello", "body": "…" }]), None)),
            NodeExample::dsl("Insert from a form, learning the new key", r#"sekejap.query.run --write --param "1={{ $trigger.body.title }}" -- "INSERT INTO posts (title) VALUES ($1) RETURNING _key""#)
                .output(answer(&["_key"], json!([{ "_key": "01M3JSKZFJ4AZGVR0PG6ZXMDH5" }]), Some(1)))
                .note("The table declares `_key TEXT PRIMARY KEY DEFAULT ulid()`, so the INSERT leaves the key out and `RETURNING` hands it back — `input.query.rows[0]._key` for a redirect."),
            NodeExample::dsl("Upsert", r#"sekejap.query.run --write --param "1={{ $trigger.body.slug }}" --param "2={{ $trigger.body.title }}" -- "INSERT INTO posts (_key, title) VALUES ($1, $2) ON CONFLICT (_key) DO UPDATE SET title = EXCLUDED.title""#)
                .output(answer(&[], json!([]), Some(1)))
                .note("A plain INSERT of a key that exists is refused with 23505; ON CONFLICT is the upsert."),
            NodeExample::dsl("Create a table", r#"sekejap.query.run --write -- "CREATE TABLE posts (_key TEXT PRIMARY KEY DEFAULT ulid(), title TEXT NOT NULL, slug TEXT UNIQUE, status TEXT DEFAULT 'draft', created_at TIMESTAMPTZ DEFAULT now()) WITH (fulltext: [title])""#)
                .note("Run once from `pipeline_run` or a `jobs/migrate` function pipeline; keep the SQL in `db/001_posts.sql`. Scalar columns are indexed automatically; `WITH` declares full-text, spatial and vector indexes."),
            NodeExample::dsl("Search by part of a name", r#"sekejap.query.run --param "1={{ '%' + $trigger.query.q + '%' }}" -- "SELECT _key, name FROM members WHERE name ILIKE $1 ORDER BY name LIMIT 20""#)
                .output(answer(&["_key", "name"], json!([{ "_key": "m1", "name": "Alex Doe" }]), None)),
            NodeExample::dsl("Newest first, one page at a time", r#"sekejap.query.run --param "1={{ $trigger.query.after || '~' }}" -- "SELECT _key, title FROM posts WHERE _key < $1 ORDER BY _key DESC LIMIT 20""#)
                .output(answer(&["_key", "title"], json!([{ "_key": "01M3JSKZFM6MM3GJFF0WKN71DD", "title": "Latest" }]), None))
                .note("ULID keys sort by time, so `ORDER BY _key DESC` is newest first. The next page passes the last `_key` shown as `after`; there is no OFFSET."),
            NodeExample::dsl("Add an edge", r#"sekejap.query.run --write --param "1={{ $trigger.body.member }}" --param "2={{ $trigger.body.institution }}" --param "3={{ $trigger.body.position }}" -- "INSERT INTO affiliated_with (member_id, institution_id, position) VALUES ($1, $2, $3)""#)
                .output(answer(&[], json!([]), Some(1)))
                .note("`affiliated_with` is an edge table: a table with two REFERENCES columns declared in `CREATE PROPERTY GRAPH … EDGE TABLES`. UPDATE and DELETE name an end: `DELETE FROM affiliated_with WHERE member_id = $1 AND institution_id = $2`."),
            NodeExample::dsl("Walk the graph", r#"sekejap.query.run --param "1={{ $trigger.params.id }}" -- "SELECT name, position FROM GRAPH_TABLE (network MATCH (m WHERE m._key = $1)-[a:affiliated_with]->(i) RETURN i.name AS name, a.position AS position)""#)
                .output(answer(&["name", "position"], json!([{ "name": "University One", "position": "Lecturer" }]), None))
                .note("Name the edge (`[a:affiliated_with]`) to read its properties. Each matching path is one row; `RETURN DISTINCT` returns a node once."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
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
    data_root: PathBuf,
}

impl Node {
    pub fn new(config: Config, data_root: PathBuf) -> Result<Self, PipelineError> {
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
        // Arrives final — `{{ }}` resolved engine-side before this ran.
        let sql = self.config.query.trim().to_string();
        if sql.is_empty() {
            return Err(PipelineError::new(CONFIG_CODE, "--query resolved empty — use: -- \"SELECT ...\""));
        }
        let params = match query::params(&self.config.param, false, PARAM_CODE)? {
            Params::Positional(values) => values,
            Params::Named(_) => return Err(PipelineError::new(PARAM_CODE, "sekejap binds positions only: --param 1=…")),
        };
        let limit = query::limit(&self.config.limit, LIMIT_CODE)?;
        let write = self.config.write;

        let data_root = self.data_root.clone();
        let owner = owner.to_string();
        let project = project.to_string();
        let statement = sql.clone();
        let result = tokio::task::spawn_blocking(move || {
            sekejap::execute_sql(&data_root, &owner, &project, &statement, &params, limit, !write)
        })
        .await
        .map_err(|err| PipelineError::new(CODE, format!("sekejap query task failed: {err}")))?
        .map_err(|err| {
            if err.code == sekejap::READ_ONLY_CODE {
                PipelineError::new(WRITE_CODE, query::write_refusal(NODE_KIND, &err.message))
            } else {
                PipelineError::new(CODE, with_ddl_hint(&sql, err.to_string()))
            }
        })?;

        // The store answers positionally (columns + value arrays — the shape
        // the DB pages render). A pipeline reads `input.query.rows[0].title`,
        // so each row is keyed by its column name here.
        let rows = rows_as_objects(&result.columns, &result.rows);
        let columns = result.columns.iter().map(|column| column.name.clone()).collect();
        let affected = write.then(|| result.affected_rows.unwrap_or(0));
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: with_answer(&input.payload, query::answer(columns, rows, result.truncated, affected)),
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

/// One object per row, keyed by column name. A duplicate column name keeps
/// the last value, as pg.query.run does; alias in SQL when both are wanted.
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

    #[test]
    fn a_failed_create_table_points_at_the_grammar() {
        let hinted = with_ddl_hint("CREATE TABLE users (age INT CHECK (age > 0))", "parse error".into());
        assert!(hinted.contains("db/sekejap"), "{hinted}");
        assert!(hinted.contains("VECTOR(384)"), "{hinted}");
        assert_eq!(with_ddl_hint("SELECT 1", "parse error".into()), "parse error");
    }

    /// The store's positional rows become `input.query.rows[0].name`, as
    /// pg.query.run and sqlite.query.run deliver them.
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

#[cfg(test)]
mod node_tests {
    use super::*;
    use serde_json::json;

    async fn exec(root: &std::path::Path, config: Value, payload: Value) -> Result<Value, PipelineError> {
        let node = Node::new(serde_json::from_value(config).expect("config"), root.to_path_buf())?;
        let out = node
            .execute_async(NodeExecutionInput {
                node_id: "n0".to_string(),
                input_pin: "in".to_string(),
                payload,
                metadata: json!({ "owner": "demo", "project": "site-a", "pipeline": "t", "request_id": "r" }),
                bus: None,
            })
            .await?;
        Ok(out.payload)
    }

    #[test]
    fn the_signature_is_the_shared_query_shape() {
        assert_eq!(
            crate::pipeline::nodes::node_signature(&definition()),
            "sekejap.query.run [--query TEXT] [--param KEY=EXPR…] [--write] [--limit N] → query"
        );
    }

    /// Without `--write` a write is refused — by the up-front check, or by
    /// the read-only snapshot when the statement does not start with its verb.
    #[tokio::test]
    async fn a_write_needs_write() {
        let tmp = tempfile::tempdir().expect("tempdir");
        exec(tmp.path(), json!({ "query": "CREATE TABLE posts (_key TEXT PRIMARY KEY, n INT)", "write": true }), json!({}))
            .await
            .expect("create");
        for sql in ["INSERT INTO posts (_key, n) VALUES ('a', 1)", "/* seed */ INSERT INTO posts (_key, n) VALUES ('b', 2)"] {
            let err = exec(tmp.path(), json!({ "query": sql }), json!({})).await.unwrap_err();
            assert_eq!(err.code, WRITE_CODE, "{sql}: {}", err.message);
            assert!(err.message.contains("add --write"), "{}", err.message);
        }
        let empty = exec(tmp.path(), json!({ "query": "SELECT count(*) AS n FROM posts" }), json!({})).await.unwrap();
        assert_eq!(empty["query"]["rows"][0]["n"], 0, "nothing was written");
    }

    /// `--param 1=…` binds `$1` with its JSON type; the answer is one
    /// `query` key beside the payload, with `rows_affected` only for a write.
    #[tokio::test]
    async fn params_bind_by_position_and_the_answer_is_one_key() {
        let tmp = tempfile::tempdir().expect("tempdir");
        exec(tmp.path(), json!({ "query": "CREATE TABLE posts (_key TEXT PRIMARY KEY, n INT)", "write": true }), json!({}))
            .await
            .expect("create");
        let wrote = exec(
            tmp.path(),
            json!({ "query": "INSERT INTO posts (_key, n) VALUES ($1, $2) RETURNING _key", "param": { "2": 41, "1": "a" }, "write": true }),
            json!({}),
        )
        .await
        .unwrap();
        assert_eq!(wrote["query"]["rows"], json!([{ "_key": "a" }]));
        assert_eq!(wrote["query"]["rows_affected"], 1);
        let read = exec(
            tmp.path(),
            json!({ "query": "SELECT _key, n FROM posts WHERE n = $1", "param": { "1": 41 } }),
            json!({ "webhook": { "path": "/p" } }),
        )
        .await
        .unwrap();
        assert_eq!(
            read,
            json!({ "webhook": { "path": "/p" }, "query": { "rows": [{ "_key": "a", "n": 41 }], "columns": ["_key", "n"], "row_count": 1, "truncated": false } })
        );
        for param in [json!({ "1": 1, "3": 3 }), json!({ "1": 1, "n": 2 }), json!({ "n": 2 })] {
            let err = exec(tmp.path(), json!({ "query": "SELECT $1", "param": param }), json!({})).await.unwrap_err();
            assert_eq!(err.code, PARAM_CODE, "{}", err.message);
        }
    }
}

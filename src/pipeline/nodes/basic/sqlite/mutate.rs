//! SQLite mutate node — INSERT / UPDATE / DELETE against the project's embedded SQLite database.

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

pub const NODE_KIND: &str = "n.sqlite.mutate";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

/// Unified node-definition metadata for `n.sqlite.mutate`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Database, NodeCapability::Process],
        title: "SQLite Mutate".to_string(),
        description: "Write to the project's built-in SQLite database (connection `default`): INSERT, UPDATE, DELETE, CREATE TABLE, ALTER. \
            SQL in the body after `--`, values in `--params` bound as `?1, ?2, …`. Answers `{ ok: true, affected_rows: N }` and nothing \
            else — the inserted row is not returned; SELECT it afterwards with `sqlite.query`, or keep the values you inserted from \
            the previous node with `$nodes.<id>`."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "description": "Input context — values accessible via {{ $input.* }} in the SQL."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "ok": { "type": "boolean" },
                "affected_rows": { "type": "integer" }
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
                description: "SQL mutation (alternative to body `-- \"INSERT INTO ...\"`)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--params".to_string(),
                config_key: "params".to_string(),
                description: "Bind values for ?1, ?2, … — a literal or {{ expr }}. A whole {{ }} carries its typed value, so \"{{ [input.id, 10] }}\" is a real array.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "params".to_string(),
                label: "Params".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Bind values for ?1, ?2, … — a literal or {{ expr }}.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "query".to_string(),
                label: "Query".to_string(),
                field_type: NodeFieldType::CodeEditor,
                language: Some("sql".to_string()),
                span: Some("full".to_string()),
                help: Some(
                    "INSERT INTO items (id, title) VALUES ($1, $2)\n\
                     Supports: INSERT INTO, UPDATE, DELETE FROM, CREATE TABLE, DROP TABLE."
                        .to_string(),
                ),
                ..Default::default()
            },
        ],
        layout: vec![
            LayoutItem::Field("query".to_string()),
            LayoutItem::Field("params".to_string()),
        ],
        ai_tool: crate::pipeline::model::NodeAiToolDefinition {
            registered: true,
            tool_name: "sqlite_mutate".to_string(),
            tool_description: "Run a SQL mutation against the project's embedded SQLite database. \
                Arg: query (required) — INSERT / UPDATE / DELETE / CREATE TABLE."
                .to_string(),
            tool_input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "SQL mutation string" }
                },
                "required": ["query"]
            }),
        },
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Insert from a form", r#"sqlite.mutate --params "{{ [input.body.email, input.body.name] }}" -- "INSERT INTO users (email, name) VALUES (?1, ?2)""#)
                .output(serde_json::json!({ "ok": true, "affected_rows": 1 })),
            crate::pipeline::model::NodeExample::dsl("Create a table", r#"sqlite.mutate -- "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, email TEXT UNIQUE NOT NULL, name TEXT, created_at TEXT DEFAULT CURRENT_TIMESTAMP)""#)
                .output(serde_json::json!({ "ok": true, "affected_rows": 0 })),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default, alias = "sql")]
    pub query: String,
    /// Bind values for `?1`, `?2`, … A whole `{{ }}` carries its typed value.
    #[serde(default)]
    pub params: Value,
}

pub struct Node {
    config: Config,
    data_root: PathBuf,
}

impl Node {
    pub fn new(config: Config, data_root: PathBuf) -> Result<Self, PipelineError> {
        // One flag per thing now, so "set one or the other, not both" has
        // nothing left to adjudicate.
        if config.query.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_SQLITE_MUTATE_CONFIG",
                "config.query must not be empty",
            ));
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
        // Both arrive final — `{{ }}` resolved engine-side before this ran.
        let sql = self.config.query.trim().to_string();
        if sql.is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_SQLITE_MUTATE",
                "query must not be empty",
            ));
        }
        let param_values: Vec<Value> = match self.config.params.clone() {
            Value::Null => Vec::new(),
            Value::Array(items) => items,
            other => vec![other],
        };
        crate::platform::sqlite_schema::ensure_local_db_migrated(&self.data_root, owner, project)
            .map_err(|err| PipelineError::new("FW_NODE_SQLITE_MUTATE_MIGRATE", err.message))?;
        let db_path = self
            .data_root
            .join("users")
            .join(owner)
            .join(project)
            .join("data")
            .join("store")
            .join("local.db");
        let affected_rows = tokio::task::spawn_blocking(move || -> Result<usize, String> {
            let conn = rusqlite::Connection::open(&db_path).map_err(|e| format!("open db: {e}"))?;
            conn.execute_batch("PRAGMA journal_mode=WAL;")
                .map_err(|e| format!("pragma: {e}"))?;
            let params: Vec<Box<dyn rusqlite::types::ToSql>> = param_values
                .iter()
                .map(|v| -> Box<dyn rusqlite::types::ToSql> {
                    match v {
                        Value::Null => Box::new(Option::<String>::None),
                        Value::Bool(b) => Box::new(*b),
                        Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                Box::new(i)
                            } else if let Some(f) = n.as_f64() {
                                Box::new(f)
                            } else {
                                Box::new(n.to_string())
                            }
                        }
                        Value::String(s) => Box::new(s.clone()),
                        other => Box::new(other.to_string()),
                    }
                })
                .collect();
            let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                params.iter().map(|p| p.as_ref()).collect();
            let n = conn
                .execute(&sql, param_refs.as_slice())
                .map_err(|e| format!("execute: {e}"))?;
            Ok(n)
        })
        .await
        .map_err(|e| PipelineError::new("FW_NODE_SQLITE_MUTATE", format!("task: {e}")))?
        .map_err(|e| PipelineError::new("FW_NODE_SQLITE_MUTATE", e))?;

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({ "ok": true, "affected_rows": affected_rows }),
            trace: vec![format!("node_kind={NODE_KIND}")],
        })
    }
}

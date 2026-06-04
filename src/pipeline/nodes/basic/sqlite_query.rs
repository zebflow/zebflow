//! SQLite query node — SELECT against the project's embedded SQLite database.

use std::path::PathBuf;

use async_trait::async_trait;
use rusqlite::types::ValueRef;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::util::metadata_scope;
use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "n.sqlite.query";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

/// Unified node-definition metadata for `n.sqlite.query`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "SQLite Query".to_string(),
        description: "Run a SQL SELECT query against the project's embedded SQLite database. \
            Write the query in the body using `-- \"SELECT ...\"`. \
            Use `{{ expr }}` placeholders anywhere in the SQL — they are resolved before the node \
            runs. Output: `{ rows: [...] }` — use `input.rows` in downstream nodes or templates."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "description": "Input context — values accessible via {{ $input.* }} in the SQL."
        }),
        output_schema: json!({
            "type": "object",
            "properties": { "rows": { "type": "array" } }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![DslFlag {
            flag: "--sql".to_string(),
            config_key: "sql".to_string(),
            description: "SQL query (alternative to body `-- \"SELECT ...\"`)".to_string(),
            kind: DslFlagKind::Scalar,
            required: false,
        }],
        fields: vec![NodeFieldDef {
            name: "sql".to_string(),
            label: "SQL".to_string(),
            field_type: NodeFieldType::Textarea,
            rows: Some(6),
            help: Some(
                "SELECT id, title FROM posts LIMIT 20\n\
                 Use {{ $input.field }} or {{ $trigger.params.id }} for dynamic values."
                    .to_string(),
            ),
            default_value: Some(json!("SELECT id\nFROM items\nLIMIT 20")),
            ..Default::default()
        }],
        layout: vec![crate::pipeline::model::LayoutItem::Field("sql".to_string())],
        ai_tool: crate::pipeline::model::NodeAiToolDefinition {
            registered: true,
            tool_name: "sqlite_query".to_string(),
            tool_description:
                "Run a SQL SELECT query against the project's embedded SQLite database. \
                Arg: sql (required) — SQL SELECT string."
                    .to_string(),
            tool_input_schema: json!({
                "type": "object",
                "properties": {
                    "sql": { "type": "string", "description": "SQL SELECT query" }
                },
                "required": ["sql"]
            }),
        },
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// SQL query — populated from DSL body `-- "SELECT ..."`.
    /// `{{ expr }}` placeholders are resolved before the node runs.
    #[serde(default)]
    pub sql: String,
}

pub struct Node {
    config: Config,
    data_root: PathBuf,
}

impl Node {
    pub fn new(config: Config, data_root: PathBuf) -> Result<Self, PipelineError> {
        Ok(Self { config, data_root })
    }
}

fn sqlite_value_to_json(v: ValueRef<'_>) -> Value {
    match v {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => json!(i),
        ValueRef::Real(f) => json!(f),
        ValueRef::Text(t) => Value::String(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => Value::String(hex::encode(b)),
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
        let sql = self.config.sql.trim().to_string();
        if sql.is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_SQLITE_QUERY",
                "sql must not be empty — use: -- \"SELECT ...\"",
            ));
        }
        let db_path = self
            .data_root
            .join("users")
            .join(owner)
            .join(project)
            .join("data")
            .join("local.db");
        let rows = tokio::task::spawn_blocking(move || -> Result<Vec<Value>, String> {
            let conn = rusqlite::Connection::open(&db_path).map_err(|e| format!("open db: {e}"))?;
            let mut stmt = conn.prepare(&sql).map_err(|e| format!("prepare: {e}"))?;
            let col_count = stmt.column_count();
            let col_names: Vec<String> = (0..col_count)
                .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
                .collect();
            let rows = stmt
                .query_map([], |row| {
                    let mut obj = serde_json::Map::new();
                    for (i, name) in col_names.iter().enumerate() {
                        let v = sqlite_value_to_json(row.get_ref(i)?);
                        obj.insert(name.clone(), v);
                    }
                    Ok(Value::Object(obj))
                })
                .map_err(|e| format!("query: {e}"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("row: {e}"))?;
            Ok(rows)
        })
        .await
        .map_err(|e| PipelineError::new("FW_NODE_SQLITE_QUERY", format!("task: {e}")))?
        .map_err(|e| PipelineError::new("FW_NODE_SQLITE_QUERY", e))?;

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({ "rows": rows }),
            trace: vec![format!("node_kind={NODE_KIND}")],
        })
    }
}

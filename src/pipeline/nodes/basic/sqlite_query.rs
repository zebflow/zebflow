//! SQLite query node — SELECT against the project's embedded SQLite database.

use std::path::PathBuf;

use async_trait::async_trait;
use rusqlite::types::ValueRef;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::util::metadata_scope;
use crate::pipeline::model::NodeCapability;
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType};
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
        capabilities: vec![NodeCapability::Database, NodeCapability::Process],
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
        dsl_flags: vec![
            DslFlag {
                flag: "--query".to_string(),
                config_key: "query".to_string(),
                description: "SQL query (alternative to body `-- \"SELECT ...\"`)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--params".to_string(),
                config_key: "params".to_string(),
                description:
                    "Bind values for ?1, ?2, … — a literal or {{ expr }}. A whole {{ }} carries \
                     its typed value, so \"{{ [input.id, 10] }}\" is a real array."
                        .to_string(),
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
                    "SELECT id, title FROM posts LIMIT 20\n\
                     Use $1, $2 with params."
                        .to_string(),
                ),
                default_value: Some(json!("SELECT id\nFROM items\nLIMIT 20")),
                ..Default::default()
            },
        ],
        layout: vec![
            LayoutItem::Field("query".to_string()),
            LayoutItem::Field("params".to_string()),
        ],
        ai_tool: crate::pipeline::model::NodeAiToolDefinition {
            registered: true,
            tool_name: "sqlite_query".to_string(),
            tool_description:
                "Run a SQL SELECT query against the project's embedded SQLite database. \
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
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default, alias = "sql")]
    pub query: String,
    /// Bind values for `?1`, `?2`, … A whole `{{ }}` carries its typed value,
    /// so `--params "{{ [input.id, 10] }}"` is a real array; anything else
    /// binds as one parameter.
    #[serde(default)]
    pub params: Value,
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
        // One flag per thing now, so there is no "set one or the other" left
        // to adjudicate. A `{{ }}` resolving to empty is caught at run time,
        // where the resolved value is known.
        if config.query.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_SQLITE_QUERY_CONFIG",
                "config.query must not be empty",
            ));
        }
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
        // Both arrive final — `{{ }}` resolved engine-side before this node
        // ran (NodeIO §Value resolution).
        let sql = self.config.query.trim().to_string();
        if sql.is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_SQLITE_QUERY",
                "query must not be empty",
            ));
        }
        // A whole `{{ }}` carries its typed value, so an array stays an array
        // and anything else binds as one parameter.
        let param_values: Vec<Value> = match self.config.params.clone() {
            Value::Null => Vec::new(),
            Value::Array(items) => items,
            other => vec![other],
        };
        crate::platform::sqlite_schema::ensure_local_db_migrated(&self.data_root, owner, project)
            .map_err(|err| PipelineError::new("FW_NODE_SQLITE_QUERY_MIGRATE", err.message))?;
        let db_path = self
            .data_root
            .join("users")
            .join(owner)
            .join(project)
            .join("data")
            .join("store")
            .join("local.db");
        let rows = tokio::task::spawn_blocking(move || -> Result<Vec<Value>, String> {
            let conn = rusqlite::Connection::open(&db_path).map_err(|e| format!("open db: {e}"))?;
            let mut stmt = conn.prepare(&sql).map_err(|e| format!("prepare: {e}"))?;
            let col_count = stmt.column_count();
            let col_names: Vec<String> = (0..col_count)
                .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
                .collect();
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
            let rows = stmt
                .query_map(param_refs.as_slice(), |row| {
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

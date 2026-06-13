//! SQLite query node — SELECT against the project's embedded SQLite database.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use rusqlite::types::ValueRef;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::util::{eval_deno_expr, metadata_scope, resolve_array_values, resolve_query_binding};
use crate::language::LanguageEngine;
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
                flag: "--query-expr".to_string(),
                config_key: "query_expr".to_string(),
                description:
                    "JS expression returning the SQL query string. Overrides --query at runtime."
                        .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--params-path".to_string(),
                config_key: "params_path".to_string(),
                description: "Dot-notation path into upstream payload for $1/$2 bind params."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--params-expr".to_string(),
                config_key: "params_expr".to_string(),
                description: "JS expression returning an array of bind params.".to_string(),
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
                    "SELECT id, title FROM posts LIMIT 20\n\
                     Use $1, $2 with params."
                        .to_string(),
                ),
                default_value: Some(json!("SELECT id\nFROM items\nLIMIT 20")),
                ..Default::default()
            },
            NodeFieldDef {
                name: "query_expr".to_string(),
                label: "Query Expr".to_string(),
                field_type: NodeFieldType::Textarea,
                rows: Some(3),
                help: Some(
                    "JS expression returning SQL query string. Overrides the query editor above."
                        .to_string(),
                ),
                ..Default::default()
            },
            NodeFieldDef {
                name: "params_path".to_string(),
                label: "Params Path".to_string(),
                field_type: NodeFieldType::Text,
                help: Some(
                    "Dot-notation path into upstream payload for $1/$2 bind params.".to_string(),
                ),
                ..Default::default()
            },
            NodeFieldDef {
                name: "params_expr".to_string(),
                label: "Params Expr".to_string(),
                field_type: NodeFieldType::Textarea,
                rows: Some(3),
                help: Some("JS expression returning array of bind params.".to_string()),
                ..Default::default()
            },
        ],
        layout: vec![
            LayoutItem::Field("query".to_string()),
            LayoutItem::Field("query_expr".to_string()),
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("params_path".to_string()),
                    LayoutItem::Field("params_expr".to_string()),
                ],
            },
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
    #[serde(default)]
    pub query_expr: Option<String>,
    #[serde(default)]
    pub params_path: Option<String>,
    #[serde(default)]
    pub params_expr: Option<String>,
}

pub struct Node {
    config: Config,
    data_root: PathBuf,
    language: Arc<dyn LanguageEngine>,
}

impl Node {
    pub fn new(
        config: Config,
        data_root: PathBuf,
        language: Arc<dyn LanguageEngine>,
    ) -> Result<Self, PipelineError> {
        if config.query.trim().is_empty()
            && config
                .query_expr
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            return Err(PipelineError::new(
                "FW_NODE_SQLITE_QUERY_CONFIG",
                "config.query must not be empty (set query or query_expr)",
            ));
        }
        if !config.query.trim().is_empty()
            && config
                .query_expr
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .len()
                > 0
        {
            return Err(PipelineError::new(
                "FW_NODE_SQLITE_QUERY_CONFIG",
                "set either query or query_expr, not both",
            ));
        }
        Ok(Self {
            config,
            data_root,
            language,
        })
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
        let sql = resolve_query_binding(
            self.language.as_ref(),
            &input.payload,
            &input.metadata,
            self.config.query_expr.as_deref(),
            &self.config.query,
            "FW_NODE_SQLITE_QUERY",
        )?;
        let param_values: Vec<Value> = if let Some(expr) = self.config.params_expr.as_deref() {
            let evaluated = eval_deno_expr(
                self.language.as_ref(),
                expr,
                &input.payload,
                &input.metadata,
            )?;
            match evaluated {
                Value::Array(items) => items,
                other => vec![other],
            }
        } else {
            resolve_array_values(&input.payload, self.config.params_path.as_deref())
        };
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

//! Sekejap query node — SQL against the project's embedded Sekejap store.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::util::metadata_scope;
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
            "Execute SQL against the project's embedded Sekejap multimodel store and return rows or affected count."
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
        let result = tokio::task::spawn_blocking(move || {
            sekejap::execute_sql(
                &data_root,
                &owner,
                &project,
                &query,
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
        .map_err(|err| PipelineError::new("FW_NODE_SEKEJAP_QUERY", err.to_string()))?;

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
    use super::rows_as_objects;
    use crate::platform::model::DbQueryColumn;
    use serde_json::json;

    /// The store's positional rows become `input.rows[0].name`, which is what
    /// every pipeline, page and doc reads — and what pg.query and sqlite.query
    /// already deliver.
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

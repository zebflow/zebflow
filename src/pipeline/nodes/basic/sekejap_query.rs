//! Sekejap query node — SQL against the project's embedded Sekejap store.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::util::metadata_scope;
use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType};
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
        title: "Sekejap Query".to_string(),
        description:
            "Execute SQL against the project's embedded Sekejap multimodel store and return rows or affected count."
                .to_string(),
        input_schema: json!({
            "type": "object",
            "description": "Input context — values accessible via {{ $input.* }} in the SQL."
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
        ],
        fields: vec![
            NodeFieldDef {
                name: "query".to_string(),
                label: "Query".to_string(),
                field_type: NodeFieldType::CodeEditor,
                language: Some("sql".to_string()),
                span: Some("full".to_string()),
                help: Some(
                    "SELECT * FROM posts LIMIT 20\nINSERT INTO posts (_key, title) VALUES ('first', 'Hello')"
                        .to_string(),
                ),
                default_value: Some(json!("SELECT * FROM items LIMIT 20")),
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
            crate::pipeline::model::LayoutItem::Field("query".to_string()),
            crate::pipeline::model::LayoutItem::Row {
                row: vec![
                    crate::pipeline::model::LayoutItem::Field("limit".to_string()),
                    crate::pipeline::model::LayoutItem::Field("read_only".to_string()),
                ],
            },
        ],
        ai_tool: crate::pipeline::model::NodeAiToolDefinition {
            registered: true,
            tool_name: "sekejap_query".to_string(),
            tool_description:
                "Execute SQL against the project's embedded Sekejap store. Args: query (required), limit (optional), read_only (optional)."
                    .to_string(),
            tool_input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Sekejap SQL query" },
                    "limit": { "type": "integer", "description": "Maximum rows to return" },
                    "read_only": { "type": "boolean", "description": "Reject write statements when true" }
                },
                "required": ["query"]
            }),
        },
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub query: String,
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
    pub fn new(config: Config, data_root: PathBuf) -> Result<Self, PipelineError> {
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
        let query = self.config.query.trim().to_string();
        if query.is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_SEKEJAP_QUERY",
                "query must not be empty — use: -- \"SELECT ...\"",
            ));
        }

        let result = sekejap::execute_sql(
            &self.data_root,
            owner,
            project,
            &query,
            self.config.limit,
            self.config.read_only,
        )
        .map_err(|err| PipelineError::new("FW_NODE_SEKEJAP_QUERY", err.to_string()))?;

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({
                "columns": result.columns,
                "rows": result.rows,
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

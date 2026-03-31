//! Sekejap SQL query node.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::pipeline::{
    PipelineError, NodeDefinition,
    nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput},
};
use crate::platform::services::SimpleTableService;

use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType};
use super::util::metadata_scope;

pub const NODE_KIND: &str = "n.sekejap.query";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

/// Unified node-definition metadata for `n.sekejap.query`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Sekejap Query".to_string(),
        description: "Run a SQL SELECT (or TRAVERSE / VECTOR_NEAR) against the project's embedded \
            Sekejap database. Write the query in the body using `-- \"SELECT ...\"`. \
            Use `{{ expr }}` placeholders anywhere in the SQL — they are resolved before the node \
            runs. Output: `{ rows: [...] }` — use `input.rows` in downstream nodes or templates."
            .to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "description": "Input context — values accessible via {{ $input.* }} in the SQL."
        }),
        output_schema: serde_json::json!({
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
                flag: "--sql".to_string(),
                config_key: "sql".to_string(),
                description: "SQL query (alternative to body `-- \"SELECT ...\"`)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "sql".to_string(),
                label: "SQL".to_string(),
                field_type: NodeFieldType::Textarea,
                rows: Some(6),
                help: Some(
                    "SELECT id, title FROM posts LIMIT 20\n\
                     Use {{ $input.field }} or {{ $trigger.params.id }} for dynamic values."
                        .to_string(),
                ),
                default_value: Some(serde_json::json!("SELECT id FROM tasks LIMIT 20")),
                ..Default::default()
            },
        ],
        layout: vec![
            crate::pipeline::model::LayoutItem::Field("sql".to_string()),
        ],
        ai_tool: crate::pipeline::model::NodeAiToolDefinition {
            registered: true,
            tool_name: "table_query".to_string(),
            tool_description: "Run a SQL SELECT query against the embedded Sekejap database. \
                Arg: sql (required) — SQL string."
                .to_string(),
            tool_input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "sql": { "type": "string", "description": "SQL SELECT query" }
                },
                "required": ["sql"]
            }),
        },
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
    simple_tables: Arc<SimpleTableService>,
}

impl Node {
    pub fn new(
        config: Config,
        simple_tables: Arc<SimpleTableService>,
    ) -> Result<Self, PipelineError> {
        Ok(Self { config, simple_tables })
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
        if self.config.sql.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_SJ_QUERY",
                "sql must not be empty — use: -- \"SELECT ...\"",
            ));
        }
        let rows = self
            .simple_tables
            .execute_query_sql(owner, project, &self.config.sql)
            .map_err(|e| PipelineError::new("FW_NODE_SJ_QUERY", e.to_string()))?;
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({ "rows": rows }),
            trace: vec![format!("node_kind={NODE_KIND}")],
        })
    }
}

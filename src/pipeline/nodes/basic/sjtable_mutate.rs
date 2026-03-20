//! Simple Table mutate node — delete and other row-level mutations.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::pipeline::{
    PipelineError, NodeDefinition,
    nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput},
};
use crate::language::LanguageEngine;
use crate::platform::services::SimpleTableService;

use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType, SelectOptionDef};
use super::util::{eval_deno_expr, metadata_scope, resolve_path_cloned};

pub const NODE_KIND: &str = "n.sjtable.mutate";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

/// Unified node-definition metadata for `n.sjtable.mutate`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Simple Table Mutate".to_string(),
        description: "Delete rows from project simple-table collections.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "description": "Input context used for row ID resolution."
        }),
        output_schema: serde_json::json!({
            "type": "object",
            "properties": { "deleted": { "type": "boolean" }, "row_id": { "type": "string" } }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag {
                flag: "--table".to_string(),
                config_key: "table".to_string(),
                description: "Simple table name.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--op".to_string(),
                config_key: "operation".to_string(),
                description: "Mutation operation: delete.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--operation".to_string(),
                config_key: "operation".to_string(),
                description: "Mutation operation: delete.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--id-path".to_string(),
                config_key: "row_id_path".to_string(),
                description: "JSON pointer into input payload for the row ID.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: {
            vec![
                NodeFieldDef { name: "title".to_string(), label: "Title".to_string(), field_type: NodeFieldType::Text, help: Some("Override display title for this node.".to_string()), ..Default::default() },
                NodeFieldDef { name: "table".to_string(), label: "Table".to_string(), field_type: NodeFieldType::Text, help: Some("Simple table slug to mutate.".to_string()), ..Default::default() },
                NodeFieldDef { name: "operation".to_string(), label: "Operation".to_string(), field_type: NodeFieldType::Select, options: vec![
                    SelectOptionDef { value: "delete".to_string(), label: "delete".to_string() },
                ], help: Some("delete removes the row permanently.".to_string()), ..Default::default() },
                NodeFieldDef { name: "row_id_path".to_string(), label: "Row ID Path".to_string(), field_type: NodeFieldType::Text, help: Some("JSON pointer into the payload for the row ID (e.g. params.id).".to_string()), ..Default::default() },
                NodeFieldDef { name: "row_id_expr".to_string(), label: "Row ID Expr".to_string(), field_type: NodeFieldType::Textarea, rows: Some(3), ..Default::default() },
            ]
        },
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("title".to_string()), LayoutItem::Field("operation".to_string())] },
            LayoutItem::Field("table".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("row_id_path".to_string()), LayoutItem::Field("row_id_expr".to_string())] },
        ],
        ai_tool: Default::default(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    #[default]
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub table: String,
    #[serde(default)]
    pub operation: Operation,
    #[serde(default)]
    pub row_id_path: Option<String>,
    #[serde(default)]
    pub row_id_expr: Option<String>,
}

pub struct Node {
    config: Config,
    simple_tables: Arc<SimpleTableService>,
    language: Arc<dyn LanguageEngine>,
}

impl Node {
    pub fn new(
        config: Config,
        simple_tables: Arc<SimpleTableService>,
        language: Arc<dyn LanguageEngine>,
    ) -> Result<Self, PipelineError> {
        if config.table.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_SJTABLE_MUTATE_CONFIG",
                "config.table must not be empty",
            ));
        }
        Ok(Self { config, simple_tables, language })
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

        let row_id = if let Some(expr) = self.config.row_id_expr.as_deref() {
            eval_deno_expr(self.language.as_ref(), expr, &input.payload, &input.metadata)?
                .as_str()
                .map(ToString::to_string)
                .ok_or_else(|| {
                    PipelineError::new(
                        "FW_NODE_SJTABLE_MUTATE",
                        "row_id_expr must return string",
                    )
                })?
        } else {
            resolve_path_cloned(&input.payload, self.config.row_id_path.as_deref())
                .and_then(|v| v.as_str().map(ToString::to_string))
                .ok_or_else(|| {
                    PipelineError::new(
                        "FW_NODE_SJTABLE_MUTATE",
                        "row_id_path must resolve to a non-empty string",
                    )
                })?
        };

        let payload = match self.config.operation {
            Operation::Delete => {
                self.simple_tables
                    .delete_row(owner, project, &self.config.table, &row_id)
                    .map_err(|e| PipelineError::new("FW_NODE_SJTABLE_MUTATE", e.to_string()))?;
                json!({ "deleted": true, "row_id": row_id })
            }
        };

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload,
            trace: vec![format!("node_kind={NODE_KIND}")],
        })
    }
}

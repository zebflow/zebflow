//! Simple Table query/upsert node.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::pipeline::{
    FrameworkError, NodeDefinition,
    nodes::{FrameworkNode, NodeExecutionInput, NodeExecutionOutput},
};
use crate::language::LanguageEngine;
use crate::platform::model::{SimpleTableQueryRequest, UpsertSimpleTableRowRequest};
use crate::platform::services::SimpleTableService;

use super::util::{eval_deno_expr, metadata_scope, resolve_path_cloned};

pub const NODE_KIND: &str = "n.sjtable.query";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

/// Unified node-definition metadata for `n.sjtable.query`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Simple Table Query".to_string(),
        description: "Query or upsert rows on project simple-table collections.".to_string(),
        input_schema: serde_json::json!({
            "type":"object",
            "description":"Input context used for where/upsert bindings."
        }),
        output_schema: serde_json::json!({
            "oneOf":[
                {"type":"object","properties":{"table":{"type":"object"},"rows":{"type":"array"}}},
                {"type":"object","properties":{"row":{"type":"object"}}}
            ]
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: true,
        script_bridge: Some(crate::pipeline::NodeScriptBridge {
            name: "n.sjtable.query".to_string(),
            enabled: false,
        }),
        ai_tool: Default::default(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    #[default]
    Query,
    Upsert,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub table: String,
    #[serde(default)]
    pub operation: Operation,
    #[serde(default)]
    pub where_field: Option<String>,
    #[serde(default)]
    pub where_value_path: Option<String>,
    #[serde(default)]
    pub row_id_path: Option<String>,
    #[serde(default)]
    pub data_path: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub table_expr: Option<String>,
    #[serde(default)]
    pub where_field_expr: Option<String>,
    #[serde(default)]
    pub where_value_expr: Option<String>,
    #[serde(default)]
    pub row_id_expr: Option<String>,
    #[serde(default)]
    pub data_expr: Option<String>,
    #[serde(default)]
    pub limit_expr: Option<String>,
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
    ) -> Result<Self, FrameworkError> {
        if config.table.trim().is_empty()
            && config
                .table_expr
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            return Err(FrameworkError::new(
                "FW_NODE_SJTABLE_CONFIG",
                "config.table must not be empty",
            ));
        }
        Ok(Self {
            config,
            simple_tables,
            language,
        })
    }
}

#[async_trait]
impl FrameworkNode for Node {
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
    ) -> Result<NodeExecutionOutput, FrameworkError> {
        let (owner, project, _pipeline, _request_id) = metadata_scope(&input.metadata)?;
        let table = resolve_string_binding(
            self.language.as_ref(),
            &input.payload,
            &input.metadata,
            self.config.table_expr.as_deref(),
            &self.config.table,
            "table",
        )?;
        let payload = match self.config.operation {
            Operation::Query => {
                let where_field = if let Some(expr) = self.config.where_field_expr.as_deref() {
                    Some(
                        eval_deno_expr(
                            self.language.as_ref(),
                            expr,
                            &input.payload,
                            &input.metadata,
                        )?
                        .as_str()
                        .ok_or_else(|| {
                            FrameworkError::new(
                                "FW_NODE_SJTABLE_QUERY",
                                "where_field_expr must return string",
                            )
                        })?
                        .to_string(),
                    )
                } else {
                    self.config.where_field.clone()
                };
                let where_value = if let Some(expr) = self.config.where_value_expr.as_deref() {
                    Some(eval_deno_expr(
                        self.language.as_ref(),
                        expr,
                        &input.payload,
                        &input.metadata,
                    )?)
                } else {
                    resolve_path_cloned(&input.payload, self.config.where_value_path.as_deref())
                };
                let limit = if let Some(expr) = self.config.limit_expr.as_deref() {
                    let value = eval_deno_expr(
                        self.language.as_ref(),
                        expr,
                        &input.payload,
                        &input.metadata,
                    )?;
                    let as_u64 = value.as_u64().ok_or_else(|| {
                        FrameworkError::new(
                            "FW_NODE_SJTABLE_QUERY",
                            "limit_expr must return integer",
                        )
                    })?;
                    usize::try_from(as_u64).map_err(|_| {
                        FrameworkError::new("FW_NODE_SJTABLE_QUERY", "limit_expr exceeds usize")
                    })?
                } else {
                    self.config.limit.unwrap_or(100)
                };
                let result = self
                    .simple_tables
                    .query_rows(
                        owner,
                        project,
                        &SimpleTableQueryRequest {
                            table,
                            where_field,
                            where_value,
                            limit,
                        },
                    )
                    .map_err(|err| FrameworkError::new("FW_NODE_SJTABLE_QUERY", err.to_string()))?;
                json!({
                    "table": result.table,
                    "rows": result.rows,
                })
            }
            Operation::Upsert => {
                let row_id = if let Some(expr) = self.config.row_id_expr.as_deref() {
                    eval_deno_expr(
                        self.language.as_ref(),
                        expr,
                        &input.payload,
                        &input.metadata,
                    )?
                    .as_str()
                    .map(ToString::to_string)
                    .ok_or_else(|| {
                        FrameworkError::new(
                            "FW_NODE_SJTABLE_UPSERT",
                            "row_id_expr must return string",
                        )
                    })?
                } else {
                    resolve_path_cloned(&input.payload, self.config.row_id_path.as_deref())
                        .and_then(|v| v.as_str().map(ToString::to_string))
                        .ok_or_else(|| {
                            FrameworkError::new(
                                "FW_NODE_SJTABLE_UPSERT",
                                "row_id_path must resolve to a string",
                            )
                        })?
                };
                let data = if let Some(expr) = self.config.data_expr.as_deref() {
                    eval_deno_expr(
                        self.language.as_ref(),
                        expr,
                        &input.payload,
                        &input.metadata,
                    )?
                } else {
                    resolve_path_cloned(&input.payload, self.config.data_path.as_deref())
                        .unwrap_or_else(|| input.payload.clone())
                };
                let row = self
                    .simple_tables
                    .upsert_row(
                        owner,
                        project,
                        &UpsertSimpleTableRowRequest {
                            table,
                            row_id,
                            data,
                        },
                    )
                    .map_err(|err| {
                        FrameworkError::new("FW_NODE_SJTABLE_UPSERT", err.to_string())
                    })?;
                json!({ "row": row })
            }
        };

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload,
            trace: vec![format!("node_kind={NODE_KIND}")],
        })
    }
}

fn resolve_string_binding(
    language: &dyn LanguageEngine,
    input: &serde_json::Value,
    metadata: &serde_json::Value,
    expr: Option<&str>,
    fallback: &str,
    field: &str,
) -> Result<String, FrameworkError> {
    if let Some(expr) = expr {
        let value = eval_deno_expr(language, expr, input, metadata)?;
        return value.as_str().map(ToString::to_string).ok_or_else(|| {
            FrameworkError::new(
                "FW_NODE_SJTABLE_BINDING",
                format!("binding expression for '{field}' must return string"),
            )
        });
    }
    let out = fallback.trim();
    if out.is_empty() {
        return Err(FrameworkError::new(
            "FW_NODE_SJTABLE_BINDING",
            format!("resolved '{field}' must not be empty"),
        ));
    }
    Ok(out.to_string())
}

//! Sekejap query/upsert node (formerly "Simple Table").

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::pipeline::{
    PipelineError, NodeDefinition,
    nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput},
};
use crate::language::LanguageEngine;
use crate::platform::model::{SimpleTableQueryRequest, UpsertSimpleTableRowRequest};
use crate::platform::services::SimpleTableService;

use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem};
use super::util::{eval_deno_expr, metadata_scope, resolve_path_cloned};

pub const NODE_KIND: &str = "n.sekejap.query";
/// Backward-compat alias — old `.zf.json` files using `n.sjtable.query` are still dispatched here.
pub const NODE_KIND_ALIAS: &str = "n.sjtable.query";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

/// Unified node-definition metadata for `n.sekejap.query`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Sekejap Query".to_string(),
        description: "Query or upsert rows in Sekejap — Zebflow's embedded multi-model database. \
            Supports graph, vector, spatial, full-text, and vague temporal queries. \
            Create tables in the UI (Tables page) before querying.".to_string(),
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
            name: "n.sekejap.query".to_string(),
            enabled: false,
        }),
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag {
                flag: "--table".to_string(),
                config_key: "table".to_string(),
                description: "Sekejap table name to query or upsert.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--op".to_string(),
                config_key: "operation".to_string(),
                description: "Operation: query (default) or upsert.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--operation".to_string(),
                config_key: "operation".to_string(),
                description: "Operation: query (default) or upsert.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--id-path".to_string(),
                config_key: "row_id_path".to_string(),
                description: "JSON pointer into input payload for the row ID (upsert).".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--limit".to_string(),
                config_key: "limit".to_string(),
                description: "Maximum rows to return (query).".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--where-field".to_string(),
                config_key: "where_field".to_string(),
                description: "Field name to filter on (query).".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--where-value".to_string(),
                config_key: "where_value_expr".to_string(),
                description: "JS expression for filter value (query).".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType, SelectOptionDef};
            vec![
                NodeFieldDef { name: "title".to_string(), label: "Title".to_string(), field_type: NodeFieldType::Text, help: Some("Override display title for this node.".to_string()), ..Default::default() },
                NodeFieldDef { name: "table".to_string(), label: "Table".to_string(), field_type: NodeFieldType::Text, help: Some("Sekejap table slug to query/upsert (create in UI first).".to_string()), default_value: Some(serde_json::json!("posts")), ..Default::default() },
                NodeFieldDef { name: "operation".to_string(), label: "Operation".to_string(), field_type: NodeFieldType::Select, options: vec![
                    SelectOptionDef { value: "query".to_string(), label: "query".to_string() },
                    SelectOptionDef { value: "upsert".to_string(), label: "upsert".to_string() },
                ], help: Some("query returns rows, upsert writes one row.".to_string()), ..Default::default() },
                NodeFieldDef { name: "table_expr".to_string(), label: "Table Expr".to_string(), field_type: NodeFieldType::Textarea, rows: Some(3), ..Default::default() },
                NodeFieldDef { name: "where_field".to_string(), label: "Where Field".to_string(), field_type: NodeFieldType::Text, ..Default::default() },
                NodeFieldDef { name: "where_field_expr".to_string(), label: "Where Field Expr".to_string(), field_type: NodeFieldType::Textarea, rows: Some(3), ..Default::default() },
                NodeFieldDef { name: "where_value_path".to_string(), label: "Where Value Path".to_string(), field_type: NodeFieldType::Text, ..Default::default() },
                NodeFieldDef { name: "where_value_expr".to_string(), label: "Where Value Expr".to_string(), field_type: NodeFieldType::Textarea, rows: Some(3), ..Default::default() },
                NodeFieldDef { name: "limit".to_string(), label: "Limit".to_string(), field_type: NodeFieldType::Text, ..Default::default() },
                NodeFieldDef { name: "limit_expr".to_string(), label: "Limit Expr".to_string(), field_type: NodeFieldType::Textarea, rows: Some(3), ..Default::default() },
                NodeFieldDef { name: "row_id_path".to_string(), label: "Row ID Path".to_string(), field_type: NodeFieldType::Text, default_value: Some(serde_json::json!("row_id")), ..Default::default() },
                NodeFieldDef { name: "row_id_expr".to_string(), label: "Row ID Expr".to_string(), field_type: NodeFieldType::Textarea, rows: Some(3), ..Default::default() },
                NodeFieldDef { name: "data_path".to_string(), label: "Data Path".to_string(), field_type: NodeFieldType::Text, default_value: Some(serde_json::json!("data")), ..Default::default() },
                NodeFieldDef { name: "data_expr".to_string(), label: "Data Expr".to_string(), field_type: NodeFieldType::Textarea, rows: Some(4), ..Default::default() },
            ]
        },
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("title".to_string()), LayoutItem::Field("operation".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("table".to_string()), LayoutItem::Field("table_expr".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("where_field".to_string()), LayoutItem::Field("where_field_expr".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("where_value_path".to_string()), LayoutItem::Field("where_value_expr".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("row_id_path".to_string()), LayoutItem::Field("row_id_expr".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("limit".to_string()), LayoutItem::Field("limit_expr".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("data_path".to_string()), LayoutItem::Field("data_expr".to_string())] },
        ],
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
    ) -> Result<Self, PipelineError> {
        if config.table.trim().is_empty()
            && config
                .table_expr
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            return Err(PipelineError::new(
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
                            PipelineError::new(
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
                        PipelineError::new(
                            "FW_NODE_SJTABLE_QUERY",
                            "limit_expr must return integer",
                        )
                    })?;
                    usize::try_from(as_u64).map_err(|_| {
                        PipelineError::new("FW_NODE_SJTABLE_QUERY", "limit_expr exceeds usize")
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
                    .map_err(|err| PipelineError::new("FW_NODE_SJTABLE_QUERY", err.to_string()))?;
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
                        PipelineError::new(
                            "FW_NODE_SJTABLE_UPSERT",
                            "row_id_expr must return string",
                        )
                    })?
                } else {
                    resolve_path_cloned(&input.payload, self.config.row_id_path.as_deref())
                        .and_then(|v| v.as_str().map(ToString::to_string))
                        .ok_or_else(|| {
                            PipelineError::new(
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
                        PipelineError::new("FW_NODE_SJTABLE_UPSERT", err.to_string())
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
) -> Result<String, PipelineError> {
    if let Some(expr) = expr {
        let value = eval_deno_expr(language, expr, input, metadata)?;
        return value.as_str().map(ToString::to_string).ok_or_else(|| {
            PipelineError::new(
                "FW_NODE_SJTABLE_BINDING",
                format!("binding expression for '{field}' must return string"),
            )
        });
    }
    let out = fallback.trim();
    if out.is_empty() {
        return Err(PipelineError::new(
            "FW_NODE_SJTABLE_BINDING",
            format!("resolved '{field}' must not be empty"),
        ));
    }
    Ok(out.to_string())
}

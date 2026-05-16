//! Sekejap query node — SQL against the project's embedded Sekejap store.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::util::{eval_deno_expr, metadata_scope, resolve_array_values};
use crate::language::LanguageEngine;
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
                flag: "--params-path".to_string(),
                config_key: "params_path".to_string(),
                description: "Dot-notation path into upstream payload for $1/$2 bind params. e.g. 'body' or 'identifier'. If value is an array each element maps to $1/$2/...".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--params-expr".to_string(),
                config_key: "params_expr".to_string(),
                description: "JS expression returning an array of bind params, evaluated against input. e.g. '[$trigger.params.slug]'.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--query-expr".to_string(),
                config_key: "query_expr".to_string(),
                description: "JS expression returning the Sekejap SQL query string. Overrides the body SQL at runtime.".to_string(),
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
                name: "params_path".to_string(),
                label: "Params Path".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Dot-notation path into upstream payload for $1/$2 bind params. e.g. body or identifier.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "params_expr".to_string(),
                label: "Params Expr".to_string(),
                field_type: NodeFieldType::Textarea,
                rows: Some(3),
                help: Some("JS expression returning array of bind params. e.g. [$trigger.params.slug]. Overrides params_path.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "query_expr".to_string(),
                label: "Query Expr".to_string(),
                field_type: NodeFieldType::Textarea,
                rows: Some(3),
                help: Some("JS expression returning SQL query string. Overrides the query editor above.".to_string()),
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
            LayoutItem::Field("params_path".to_string()),
            LayoutItem::Field("params_expr".to_string()),
            LayoutItem::Field("query_expr".to_string()),
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
                "Execute SQL against the project's embedded Sekejap store. Args: query or query_expr (required), params_path or params_expr for $1/$2 binds, limit (optional), read_only (optional)."
                    .to_string(),
            tool_input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Sekejap SQL query" },
                    "query_expr": { "type": "string", "description": "JS expression returning the Sekejap SQL query string" },
                    "params_path": { "type": "string", "description": "Dot-notation path into upstream payload for $1/$2 bind params" },
                    "params_expr": { "type": "string", "description": "JS expression returning an array of bind params" },
                    "limit": { "type": "integer", "description": "Maximum rows to return" },
                    "read_only": { "type": "boolean", "description": "Reject write statements when true" }
                }
            }),
        },
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub query_expr: Option<String>,
    #[serde(default)]
    pub params_path: Option<String>,
    #[serde(default)]
    pub params_expr: Option<String>,
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
                "FW_NODE_SEKEJAP_QUERY_CONFIG",
                "config.query must not be empty",
            ));
        }
        Ok(Self {
            config,
            data_root,
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
        let query = if let Some(expr) = self.config.query_expr.as_deref() {
            let value = eval_deno_expr(
                self.language.as_ref(),
                expr,
                &input.payload,
                &input.metadata,
            )?;
            value.as_str().map(ToString::to_string).ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_SEKEJAP_QUERY_BINDING",
                    "query_expr must return string",
                )
            })?
        } else {
            self.config.query.trim().to_string()
        };
        if query.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_SEKEJAP_QUERY",
                "query must not be empty — use: -- \"SELECT ...\"",
            ));
        }
        if let Some(path) = self.config.params_path.as_deref()
            && path.starts_with('/')
        {
            return Err(PipelineError::new(
                "SEKEJAP_QUERY_PARAMS_PATH_SYNTAX",
                format!(
                    "params_path uses dot notation, not JSON pointer. Use '{}' not '{}'",
                    &path[1..].replace('/', "."),
                    path
                ),
            ));
        }
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

        let result = sekejap::execute_sql(
            &self.data_root,
            owner,
            project,
            &query,
            &param_values,
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

//! Table query node — SQL over one or more table sources.

use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::execution::options::JsonReadOptions;
use datafusion::prelude::{CsvReadOptions, ParquetReadOptions, SessionContext};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::language::LanguageEngine;
use crate::pipeline::model::{
    DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType, SelectOptionDef,
};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;
use crate::zebfs::{LocalZebFs, normalize_object_path};

use super::table_convert::{
    TableFormat, collect_columns, encode_rows, parse_format, record_batch_to_rows,
};
use super::util::{eval_deno_expr, metadata_scope, resolve_array_values, resolve_query_binding};

pub const NODE_KIND: &str = "n.table.query";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Table Query".to_string(),
        description: "Run SQL over multiple table sources using the selected table engine.".to_string(),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        input_schema: json!({
            "type": "object",
            "description": "Input context for source expressions and $1/$2 bind parameter expressions."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "table": {
                    "type": "object",
                    "properties": {
                        "engine": { "type": "string" },
                        "rows": { "type": "integer" },
                        "columns": { "type": "array", "items": { "type": "string" } },
                        "preview": { "type": "array" },
                        "data": { "type": "array" },
                        "to": { "type": ["string", "null"] },
                        "url": { "type": ["string", "null"] }
                    }
                }
            }
        }),
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag {
                flag: "--engine".to_string(),
                config_key: "engine".to_string(),
                description: "Table query engine. Only geodatafusion is supported.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--from".to_string(),
                config_key: "sources".to_string(),
                description: "Table source binding. Repeat for each source: --from \"datasets/posts.parquet as posts\".".to_string(),
                kind: DslFlagKind::RepeatedList,
                required: true,
            },
            DslFlag {
                flag: "--query".to_string(),
                config_key: "query".to_string(),
                description: "SQL query. Body SQL after -- also writes this field.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--query-expr".to_string(),
                config_key: "query_expr".to_string(),
                description: "JS expression returning the SQL query string. Overrides --query at runtime.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--params-path".to_string(),
                config_key: "params_path".to_string(),
                description: "Dot-notation path into upstream payload for $1/$2 bind params.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--params-expr".to_string(),
                config_key: "params_expr".to_string(),
                description: "Deno expression returning an array of bind params, for example [$trigger.params.slug].".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--to".to_string(),
                config_key: "to_path".to_string(),
                description: "ZebFS path to write query output.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--format".to_string(),
                config_key: "to_format".to_string(),
                description: "Output format for --to: csv, json, ndjson, parquet. Defaults from path extension.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--to-json".to_string(),
                config_key: "to_json".to_string(),
                description: "Emit query rows downstream as table.data.".to_string(),
                kind: DslFlagKind::Bool,
                required: false,
            },
            DslFlag {
                flag: "--preview".to_string(),
                config_key: "preview".to_string(),
                description: "Number of rows to include in table.preview.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--limit".to_string(),
                config_key: "limit".to_string(),
                description: "Maximum rows to keep after query execution. Prefer SQL LIMIT for large data.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "engine".to_string(),
                label: "Engine".to_string(),
                field_type: NodeFieldType::Select,
                options: vec![
                    SelectOptionDef {
                        value: "geodatafusion".to_string(),
                        label: "GeoDataFusion".to_string(),
                    },
                ],
                default_value: Some(json!("geodatafusion")),
                help: Some("GeoDataFusion SQL over table sources, including supported ST_* functions.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "sources".to_string(),
                label: "Sources".to_string(),
                field_type: NodeFieldType::SourceBindings,
                span: Some("full".to_string()),
                help: Some("Bind each ZebFS path or row expression to a SQL table alias.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "query".to_string(),
                label: "Query".to_string(),
                field_type: NodeFieldType::CodeEditor,
                language: Some("sql".to_string()),
                span: Some("full".to_string()),
                help: Some("SQL over the source aliases. Use $1, $2 with params.".to_string()),
                default_value: Some(json!("SELECT *\nFROM posts\nLIMIT 20")),
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
                name: "params_path".to_string(),
                label: "Params Path".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Dot-notation path returning one value or an array for $1/$2 params.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "params_expr".to_string(),
                label: "Params Expr".to_string(),
                field_type: NodeFieldType::Textarea,
                rows: Some(3),
                help: Some("Deno expression returning an array of bind params.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "to_json".to_string(),
                label: "Emit JSON Rows".to_string(),
                field_type: NodeFieldType::Checkbox,
                help: Some("Emit rows under table.data for downstream nodes.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "to_path".to_string(),
                label: "Write To FS".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Optional ZebFS object path to write query rows, e.g. exports/result.parquet.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "to_format".to_string(),
                label: "Output Format".to_string(),
                field_type: NodeFieldType::Select,
                options: vec![
                    SelectOptionDef { value: "".to_string(), label: "Infer from path".to_string() },
                    SelectOptionDef { value: "csv".to_string(), label: "CSV".to_string() },
                    SelectOptionDef { value: "json".to_string(), label: "JSON".to_string() },
                    SelectOptionDef { value: "ndjson".to_string(), label: "NDJSON".to_string() },
                    SelectOptionDef { value: "parquet".to_string(), label: "Parquet".to_string() },
                ],
                help: Some("Optional output format override for Write To FS.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "preview".to_string(),
                label: "Preview Rows".to_string(),
                field_type: NodeFieldType::Number,
                default_value: Some(json!(20)),
                help: Some("Number of rows included in table.preview.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "limit".to_string(),
                label: "Limit Rows".to_string(),
                field_type: NodeFieldType::Number,
                help: Some("Maximum rows materialized after query execution. Prefer SQL LIMIT for large data.".to_string()),
                ..Default::default()
            },
        ],
        layout: vec![
            LayoutItem::Row { row: vec![
                LayoutItem::Field("engine".to_string()),
                LayoutItem::Field("to_json".to_string()),
            ] },
            LayoutItem::Field("sources".to_string()),
            LayoutItem::Field("query".to_string()),
            LayoutItem::Field("query_expr".to_string()),
            LayoutItem::Row { row: vec![
                LayoutItem::Field("params_path".to_string()),
                LayoutItem::Field("params_expr".to_string()),
            ] },
            LayoutItem::Row { row: vec![
                LayoutItem::Field("to_path".to_string()),
                LayoutItem::Field("to_format".to_string()),
            ] },
            LayoutItem::Row { row: vec![
                LayoutItem::Field("preview".to_string()),
                LayoutItem::Field("limit".to_string()),
            ] },
        ],
        ai_tool: Default::default(),
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SourceBindingConfig {
    Dsl(String),
    Ui { source: String, alias: String },
}

impl SourceBindingConfig {
    fn to_binding(&self) -> Result<SourceBinding, PipelineError> {
        match self {
            SourceBindingConfig::Dsl(spec) => parse_source_binding(spec),
            SourceBindingConfig::Ui { source, alias } => SourceBinding::new(source, alias),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            SourceBindingConfig::Dsl(spec) => spec.trim().is_empty(),
            SourceBindingConfig::Ui { source, alias } => {
                source.trim().is_empty() && alias.trim().is_empty()
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_engine")]
    pub engine: String,
    #[serde(default)]
    pub sources: Vec<SourceBindingConfig>,
    #[serde(default, alias = "sql")]
    pub query: String,
    #[serde(default)]
    pub query_expr: Option<String>,
    #[serde(default)]
    pub params_path: Option<String>,
    #[serde(default)]
    pub params_expr: Option<String>,
    #[serde(default)]
    pub to_path: Option<String>,
    #[serde(default)]
    pub to_format: Option<String>,
    #[serde(default)]
    pub to_json: bool,
    #[serde(default)]
    pub preview: Option<usize>,
    #[serde(default)]
    pub limit: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            engine: default_engine(),
            sources: Vec::new(),
            query: String::new(),
            query_expr: None,
            params_path: None,
            params_expr: None,
            to_path: None,
            to_format: None,
            to_json: false,
            preview: None,
            limit: None,
        }
    }
}

fn default_engine() -> String {
    "geodatafusion".to_string()
}

pub struct Node {
    config: Config,
    platform: Arc<PlatformService>,
    language: Arc<dyn LanguageEngine>,
}

impl Node {
    pub fn new(
        config: Config,
        platform: Arc<PlatformService>,
        language: Arc<dyn LanguageEngine>,
    ) -> Result<Self, PipelineError> {
        if config.sources.iter().all(SourceBindingConfig::is_empty) {
            return Err(PipelineError::new(
                "FW_NODE_TABLE_QUERY_CONFIG",
                "config.sources must include at least one --from binding",
            ));
        }
        if config.query.trim().is_empty()
            && config
                .query_expr
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            return Err(PipelineError::new(
                "FW_NODE_TABLE_QUERY_CONFIG",
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
                "FW_NODE_TABLE_QUERY_CONFIG",
                "set either query or query_expr, not both",
            ));
        }
        Ok(Self {
            config,
            platform,
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
        normalize_engine(&self.config.engine)?;
        let raw_sql = resolve_query_binding(
            self.language.as_ref(),
            &input.payload,
            &input.metadata,
            self.config.query_expr.as_deref(),
            &self.config.query,
            "FW_NODE_TABLE_QUERY_SQL",
        )?;
        let sql = normalize_select_sql(&raw_sql)?;
        let params = resolve_params(&self.config, &self.language, &input)?;
        let (owner, project, ..) = metadata_scope(&input.metadata)?;
        let layout = self
            .platform
            .file
            .ensure_project_layout(owner, project)
            .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY", err.to_string()))?;
        let zebfs = LocalZebFs::new(layout.files_dir.clone());
        let QueryRows {
            rows,
            source_labels,
        } = execute_geodatafusion_engine(
            &self.config.sources,
            &sql,
            &params,
            &zebfs,
            &layout.files_dir,
            &input,
            self.language.as_ref(),
        )
        .await?;
        let mut rows = rows;
        if let Some(limit) = self.config.limit {
            rows.truncate(limit);
        }

        let columns = collect_columns(&rows);
        let mut to_path = None;
        let mut url = None;
        let mut to_format_value = None;

        if let Some(path) = non_empty(self.config.to_path.as_deref()) {
            let rel_path = normalize_object_path(path)
                .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY", err.to_string()))?;
            let format = output_format(self.config.to_format.as_deref(), &rel_path)?;
            let bytes = encode_rows(&rows, &columns, format)
                .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY", err.to_string()))?;
            zebfs
                .put(&rel_path, &bytes)
                .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY", err.to_string()))?;
            url = Some(format!("/fs/{owner}/{project}/{rel_path}"));
            to_path = Some(rel_path);
            to_format_value = Some(format.as_str().to_string());
        }

        if to_path.is_none() && !self.config.to_json {
            return Err(PipelineError::new(
                "FW_NODE_TABLE_QUERY",
                "set --to to write a ZebFS object or --to-json to emit rows downstream",
            ));
        }

        let preview_len = self.config.preview.unwrap_or(0).min(rows.len());
        let engine_label = "geodatafusion";
        let mut table = Map::new();
        table.insert("engine".to_string(), json!(engine_label));
        table.insert("sources".to_string(), Value::Array(source_labels));
        table.insert("rows".to_string(), json!(rows.len()));
        table.insert("columns".to_string(), json!(columns));
        table.insert("to".to_string(), option_string(to_path));
        table.insert("url".to_string(), option_string(url));
        table.insert("to_format".to_string(), option_string(to_format_value));
        table.insert(
            "preview".to_string(),
            Value::Array(rows.iter().take(preview_len).cloned().collect()),
        );
        if self.config.to_json {
            table.insert("data".to_string(), Value::Array(rows.clone()));
        }

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({ "table": table }),
            trace: vec![
                format!("node_kind={NODE_KIND}"),
                format!("engine={engine_label} rows={}", rows.len()),
            ],
        })
    }
}

fn normalize_engine(value: &str) -> Result<(), PipelineError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "geodatafusion" => Ok(()),
        other => Err(PipelineError::new(
            "FW_NODE_TABLE_QUERY_ENGINE",
            format!("unsupported table query engine '{other}'"),
        )),
    }
}

struct QueryRows {
    rows: Vec<Value>,
    source_labels: Vec<Value>,
}

async fn execute_geodatafusion_engine(
    sources: &[SourceBindingConfig],
    sql: &str,
    params: &[Value],
    zebfs: &LocalZebFs,
    files_dir: &Path,
    input: &NodeExecutionInput,
    language: &dyn LanguageEngine,
) -> Result<QueryRows, PipelineError> {
    let ctx = SessionContext::new();
    geodatafusion::register(&ctx);
    let mut temps = Vec::new();
    let mut source_labels = Vec::new();

    for source_config in sources {
        if source_config.is_empty() {
            continue;
        }
        let binding = source_config.to_binding()?;
        register_source(
            &ctx, zebfs, files_dir, &binding, input, language, &mut temps,
        )
        .await?;
        source_labels.push(json!({
            "alias": binding.alias,
            "source": binding.source,
        }));
    }

    let batches = execute_geodatafusion_query(&ctx, sql, params).await?;
    let mut rows = Vec::new();
    for batch in batches {
        rows.extend(
            record_batch_to_rows(&batch)
                .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY", err.to_string()))?,
        );
    }

    Ok(QueryRows {
        rows,
        source_labels,
    })
}

#[derive(Debug, Clone)]
struct SourceBinding {
    source: String,
    alias: String,
}

impl SourceBinding {
    fn new(source: &str, alias: &str) -> Result<Self, PipelineError> {
        let source = source.trim();
        let alias = alias.trim();
        if source.is_empty() || alias.is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_TABLE_QUERY_SOURCE",
                "source binding must include both source and alias",
            ));
        }
        if !valid_alias(alias) {
            return Err(PipelineError::new(
                "FW_NODE_TABLE_QUERY_SOURCE",
                format!("source alias must be an identifier: {alias}"),
            ));
        }
        Ok(Self {
            source: source.to_string(),
            alias: alias.to_string(),
        })
    }
}

fn parse_source_binding(spec: &str) -> Result<SourceBinding, PipelineError> {
    let spec = spec.trim();
    let lower = spec.to_ascii_lowercase();
    let Some(pos) = lower.rfind(" as ") else {
        return Err(PipelineError::new(
            "FW_NODE_TABLE_QUERY_SOURCE",
            format!("source binding must use '<source> as <alias>': {spec}"),
        ));
    };
    let source = spec[..pos].trim();
    let alias = spec[pos + 4..].trim();
    SourceBinding::new(source, alias)
}

fn valid_alias(alias: &str) -> bool {
    alias
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        && !alias
            .chars()
            .next()
            .map(|ch| ch.is_ascii_digit())
            .unwrap_or(true)
}

async fn register_source(
    ctx: &SessionContext,
    zebfs: &LocalZebFs,
    files_dir: &Path,
    binding: &SourceBinding,
    input: &NodeExecutionInput,
    language: &dyn LanguageEngine,
    temps: &mut Vec<tempfile::NamedTempFile>,
) -> Result<(), PipelineError> {
    if binding.source.trim_start().starts_with('$') {
        let value = eval_deno_expr(language, &binding.source, &input.payload, &input.metadata)?;
        let rows = rows_from_json_value(value);
        let mut temp = tempfile::Builder::new()
            .suffix(".json")
            .tempfile()
            .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY", err.to_string()))?;
        for row in rows {
            writeln!(
                temp,
                "{}",
                serde_json::to_string(&row)
                    .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY", err.to_string()))?
            )
            .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY", err.to_string()))?;
        }
        ctx.register_json(
            binding.alias.as_str(),
            temp.path().to_string_lossy(),
            JsonReadOptions::default(),
        )
        .await
        .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY_REGISTER", err.to_string()))?;
        temps.push(temp);
        return Ok(());
    }

    let rel = normalize_object_path(&binding.source)
        .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY", err.to_string()))?;
    let _ = zebfs
        .get(&rel)
        .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY", err.to_string()))?;
    let abs = files_dir.join(&rel);
    let path = abs.to_string_lossy();
    match source_format(&rel)? {
        TableFormat::Csv => {
            ctx.register_csv(
                binding.alias.as_str(),
                path,
                CsvReadOptions::new().has_header(true),
            )
            .await
        }
        TableFormat::Json | TableFormat::Ndjson => {
            ctx.register_json(binding.alias.as_str(), path, JsonReadOptions::default())
                .await
        }
        TableFormat::Parquet => {
            ctx.register_parquet(binding.alias.as_str(), path, ParquetReadOptions::default())
                .await
        }
    }
    .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY_REGISTER", err.to_string()))
}

async fn execute_geodatafusion_query(
    ctx: &SessionContext,
    sql: &str,
    params: &[Value],
) -> Result<Vec<RecordBatch>, PipelineError> {
    if params.is_empty() {
        return ctx
            .sql(sql)
            .await
            .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY_SQL", err.to_string()))?
            .collect()
            .await
            .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY_SQL", err.to_string()));
    }

    let prepare_sql = format!("PREPARE zf_table_query AS {sql}");
    ctx.sql(&prepare_sql)
        .await
        .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY_PREPARE", err.to_string()))?
        .collect()
        .await
        .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY_PREPARE", err.to_string()))?;

    let param_sql = params
        .iter()
        .map(datafusion_literal)
        .collect::<Vec<_>>()
        .join(", ");
    let execute_sql = format!("EXECUTE zf_table_query({param_sql})");
    ctx.sql(&execute_sql)
        .await
        .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY_EXECUTE", err.to_string()))?
        .collect()
        .await
        .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY_EXECUTE", err.to_string()))
}

fn resolve_params(
    config: &Config,
    language: &Arc<dyn LanguageEngine>,
    input: &NodeExecutionInput,
) -> Result<Vec<Value>, PipelineError> {
    if let Some(expr) = non_empty(config.params_expr.as_deref()) {
        let evaluated = eval_deno_expr(language.as_ref(), expr, &input.payload, &input.metadata)?;
        return Ok(match evaluated {
            Value::Array(items) => items,
            other => vec![other],
        });
    }
    Ok(resolve_array_values(
        &input.payload,
        config.params_path.as_deref(),
    ))
}

fn normalize_select_sql(sql: &str) -> Result<String, PipelineError> {
    let sql = sql.trim().trim_end_matches(';').trim().to_string();
    let lower = sql.trim_start().to_ascii_lowercase();
    if !(lower.starts_with("select") || lower.starts_with("with")) {
        return Err(PipelineError::new(
            "FW_NODE_TABLE_QUERY_SQL",
            "table.query only accepts SELECT or WITH queries",
        ));
    }
    Ok(sql)
}

fn datafusion_literal(value: &Value) -> String {
    match value {
        Value::Null => "NULL".to_string(),
        Value::Bool(v) => {
            if *v {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::String(s) => quote_sql_string(s),
        other => quote_sql_string(&other.to_string()),
    }
}

fn quote_sql_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn rows_from_json_value(value: Value) -> Vec<Value> {
    match value {
        Value::Array(items) => items.into_iter().map(row_object).collect(),
        Value::Object(mut map) => {
            if let Some(Value::Array(rows)) = map.remove("rows") {
                return rows.into_iter().map(row_object).collect();
            }
            if let Some(Value::Array(rows)) = map.remove("data") {
                return rows.into_iter().map(row_object).collect();
            }
            vec![Value::Object(map)]
        }
        Value::Null => Vec::new(),
        other => vec![json!({ "value": other })],
    }
}

fn row_object(value: Value) -> Value {
    match value {
        Value::Object(_) => value,
        other => json!({ "value": other }),
    }
}

fn source_format(path: &str) -> Result<TableFormat, PipelineError> {
    let ext = Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    parse_format(ext).map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY", err.to_string()))
}

fn output_format(explicit: Option<&str>, path: &str) -> Result<TableFormat, PipelineError> {
    if let Some(value) = non_empty(explicit) {
        return parse_format(value)
            .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY", err.to_string()));
    }
    source_format(path)
}

fn option_string(value: Option<String>) -> Value {
    value.map(Value::String).unwrap_or(Value::Null)
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

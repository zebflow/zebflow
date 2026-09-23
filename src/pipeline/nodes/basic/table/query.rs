//! Table query node — SQL over one or more table sources.

use std::fs;
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
use crate::pipeline::model::NodeCapability;
use crate::pipeline::model::{
    DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType, SelectOptionDef,
};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;
use crate::zebfs::{ZebFs, normalize_object_path};

use crate::pipeline::nodes::shared::file_ref::zebfs_rel_path;
use super::convert::{
    TableFormat, collect_columns, encode_rows, parse_format,
    record_batch_to_rows,
};
use crate::pipeline::nodes::shared::util::{eval_deno_expr, metadata_scope};

pub const NODE_KIND: &str = "n.table.query";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";
const MAX_INLINE_ROWS: usize = 10_000;

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Network, NodeCapability::Filesystem, NodeCapability::Database, NodeCapability::Process],
        title: "Table Query".to_string(),
        description: "Run SQL across files — CSV, JSON, NDJSON, Parquet objects in the project's file store — as if they were tables. \
            Each `--from \"<path> as <alias>\"` binds one file; the SQL in the body queries the aliases; `--params` binds `$1, $2`. \
            Answers `{ table: { engine, rows, columns, preview, data?, to?, url? } }` — rows are in `input.table.data` only with \
            `--to-json`, otherwise they are written to `--to <path>` and only `preview` rows travel in the payload. For database \
            tables use `sekejap.query` / `pg.query`; this node is for files and analytics over them."
            .to_string(),
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
                flag: "--params".to_string(),
                config_key: "params".to_string(),
                description: "Bind values for $1, $2, … — a literal or {{ expr }}. A whole {{ }} carries its typed value, so \"{{ [$trigger.params.slug] }}\" is a real array.".to_string(),
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
                flag: "--preview-rows".to_string(),
                config_key: "preview_rows".to_string(),
                description: "Number of sample rows to include in table.preview. (The canvas preview is --preview <as>.)".to_string(),
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
                name: "params".to_string(),
                label: "Params".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Bind values for $1, $2, … — a literal or {{ expr }}.".to_string()),
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
                name: "preview_rows".to_string(),
                label: "Preview rows".to_string(),
                field_type: NodeFieldType::Number,
                default_value: Some(json!(20)),
                help: Some("Number of sample rows included in table.preview.".to_string()),
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
            LayoutItem::Field("params".to_string()),
            LayoutItem::Row { row: vec![
                LayoutItem::Field("to_path".to_string()),
                LayoutItem::Field("to_format".to_string()),
            ] },
            LayoutItem::Row { row: vec![
                LayoutItem::Field("preview_rows".to_string()),
                LayoutItem::Field("limit".to_string()),
            ] },
        ],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Aggregate a CSV", r#"table.query --from "uploads/sales.csv as sales" --to-json -- "SELECT region, SUM(amount) AS total FROM sales GROUP BY region ORDER BY total DESC""#)
                .output(serde_json::json!({ "table": { "engine": "geodatafusion", "rows": 2, "columns": ["region", "total"], "preview": [{ "region": "AU", "total": 1200 }], "data": [{ "region": "AU", "total": 1200 }, { "region": "NZ", "total": 300 }], "to": null, "url": null } })),
            crate::pipeline::model::NodeExample::dsl("Join two files into Parquet", r#"table.query --from "datasets/orders.ndjson as o" --from "datasets/customers.csv as c" --to datasets/report.parquet --preview-rows 5 -- "SELECT c.name, COUNT(*) AS orders FROM o JOIN c ON o.customer_id = c.id GROUP BY c.name""#)
                .note("The full result is the file at `datasets/report.parquet`; the payload carries `table.preview` (5 sample rows, from `--preview-rows`) and `table.url`."),
        ],
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
    /// Bind values for `$1`, `$2`, … A whole `{{ }}` carries its typed value.
    #[serde(default)]
    pub params: Value,
    #[serde(default)]
    pub to_path: Option<String>,
    #[serde(default)]
    pub to_format: Option<String>,
    #[serde(default)]
    pub to_json: bool,
    /// Number of sample rows carried under `table.preview` (`--preview-rows`).
    #[serde(default)]
    pub preview_rows: Option<usize>,
    #[serde(default)]
    pub limit: Option<usize>,
}

impl Config {
    /// `--preview-rows`; nothing else names it.
    pub fn preview_row_count(&self) -> usize {
        self.preview_rows.unwrap_or(0)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            engine: default_engine(),
            sources: Vec::new(),
            query: String::new(),
            params: Value::Null,
            to_path: None,
            to_format: None,
            to_json: false,
            preview_rows: None,
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
    /// Still needed: `--from "$expr as alias"` evaluates a source expression,
    /// which is a different mechanism from the retired --params-expr twin.
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
        // One flag per thing now, so "set one or the other" has nothing left
        // to adjudicate.
        if config.query.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_TABLE_QUERY_CONFIG",
                "config.query must not be empty",
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
        // Arrives final — `{{ }}` resolved engine-side before this ran.
        let raw_sql = self.config.query.trim().to_string();
        let sql = normalize_select_sql(&raw_sql)?;
        let params = resolve_params(&self.config);
        if non_empty(self.config.to_path.as_deref()).is_none() && !self.config.to_json {
            return Err(PipelineError::new(
                "FW_NODE_TABLE_QUERY",
                "set --to to write a ZebFS object or --to-json to emit rows downstream",
            ));
        }
        let (owner, project, ..) = metadata_scope(&input.metadata)?;
        let layout = self
            .platform
            .file
            .ensure_project_layout(owner, project)
            .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY", err.to_string()))?;
        let zebfs = layout.open_files();
        let QueryRows {
            rows,
            source_labels,
        } = execute_geodatafusion_engine(
            &self.config.sources,
            &sql,
            &params,
            &zebfs,
            &input,
            self.language.as_ref(),
            self.config.limit.unwrap_or(MAX_INLINE_ROWS),
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

        let preview_len = self.config.preview_row_count().min(rows.len());
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
    zebfs: &ZebFs,
    input: &NodeExecutionInput,
    language: &dyn LanguageEngine,
    max_inline_rows: usize,
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
        register_source(&ctx, zebfs, &binding, input, language, &mut temps).await?;
        source_labels.push(json!({
            "alias": binding.alias,
            "source": binding.source,
        }));
    }

    let bounded_sql = bounded_select_sql(sql, max_inline_rows);
    let batches = execute_geodatafusion_query(&ctx, &bounded_sql, params).await?;
    let mut rows = Vec::new();
    for batch in batches {
        if rows.len().saturating_add(batch.num_rows()) > max_inline_rows {
            return Err(PipelineError::new(
                "FW_NODE_TABLE_QUERY_LIMIT",
                format!(
                    "query would materialize more than {max_inline_rows} rows in node JSON; add SQL LIMIT, set --limit, or write a smaller result"
                ),
            ));
        }
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

fn bounded_select_sql(sql: &str, max_inline_rows: usize) -> String {
    let fetch = if max_inline_rows == 0 {
        0
    } else {
        max_inline_rows.saturating_add(1)
    };
    format!("SELECT * FROM ({sql}) AS zf_table_query_limited LIMIT {fetch}")
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
    zebfs: &ZebFs,
    binding: &SourceBinding,
    input: &NodeExecutionInput,
    language: &dyn LanguageEngine,
    temps: &mut Vec<tempfile::NamedTempFile>,
) -> Result<(), PipelineError> {
    if binding.source.trim_start().starts_with('$') {
        let value = eval_deno_expr(language, &binding.source, &input.payload, &input.metadata)?;
        if let Some(path) = zebfs_rel_path(&value)? {
            register_table_path(ctx, zebfs, binding.alias.as_str(), &path).await?;
            return Ok(());
        }
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

    register_table_path(ctx, zebfs, binding.alias.as_str(), &binding.source).await
}

async fn register_table_path(
    ctx: &SessionContext,
    zebfs: &ZebFs,
    alias: &str,
    source: &str,
) -> Result<(), PipelineError> {
    let (format_label, table_path) = if is_external_table_uri(source) {
        (source.to_string(), source.trim().to_string())
    } else {
        let rel = normalize_object_path(source)
            .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY", err.to_string()))?;
        // DataFusion streams from a file path. A bucket has none, and pulling
        // the object down to read it is a design the node does not have yet.
        let abs = zebfs
            .local_path(&rel)
            .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY", err.to_string()))?
            .ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_TABLE_QUERY",
                    format!("'{rel}': this project's files live in a bucket, and table.query streams a source from local disk; an object-store source is not supported yet"),
                )
            })?;
        ensure_local_table_file(&abs)?;
        (rel, abs.to_string_lossy().into_owned())
    };
    match source_format(&format_label)? {
        TableFormat::Csv => {
            ctx.register_csv(alias, table_path, CsvReadOptions::new().has_header(true))
                .await
        }
        TableFormat::Json | TableFormat::Ndjson => {
            ctx.register_json(alias, table_path, JsonReadOptions::default())
                .await
        }
        TableFormat::Parquet => {
            ctx.register_parquet(alias, table_path, ParquetReadOptions::default())
                .await
        }
    }
    .map_err(|err| PipelineError::new("FW_NODE_TABLE_QUERY_REGISTER", err.to_string()))
}

fn ensure_local_table_file(path: &Path) -> Result<(), PipelineError> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(()),
        Ok(_) => Err(PipelineError::new(
            "FW_NODE_TABLE_QUERY",
            "table source path is not a file",
        )),
        Err(_) => Err(PipelineError::new(
            "FW_NODE_TABLE_QUERY",
            "table source object not found",
        )),
    }
}

fn is_external_table_uri(source: &str) -> bool {
    let source = source.trim().to_ascii_lowercase();
    source.starts_with("s3://")
        || source.starts_with("s3a://")
        || source.starts_with("http://")
        || source.starts_with("https://")
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

/// Bind values arrive final: a whole `{{ }}` is a real array; anything else
/// binds as one parameter.
fn resolve_params(config: &Config) -> Vec<Value> {
    match config.params.clone() {
        Value::Null => Vec::new(),
        Value::Array(items) => items,
        other => vec![other],
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_external_table_uris() {
        assert!(is_external_table_uri("s3://bucket/path/data.parquet"));
        assert!(is_external_table_uri("https://example.test/data.csv"));
        assert!(!is_external_table_uri("datasets/local.parquet"));
    }

    #[test]
    fn preview_rows_flag_parses_and_a_canvas_preview_is_not_a_row_count() {
        let graph = crate::platform::shell::parser::build_pipeline_graph(
            "t",
            "[a] trigger.manual\n[b] table.query --from \"datasets/orders.csv as o\" --preview-rows 5 --to-json -- \"SELECT * FROM o\"\n[a] -> [b]\n",
        )
        .expect("graph");
        let config: Config = serde_json::from_value(graph.nodes[1].config.clone()).expect("config");
        assert_eq!(config.preview_rows, Some(5));
        assert_eq!(config.preview_row_count(), 5);

        // A canvas preview under the same key is not a row count.
        let canvas: Config = serde_json::from_value(json!({
            "sources": ["datasets/orders.csv as o"],
            "query": "SELECT * FROM o",
            "preview": { "out": { "as": "table" } }
        }))
        .expect("canvas preview is not an error");
        assert_eq!(canvas.preview_row_count(), 0);
    }

    #[test]
    fn wraps_query_with_materialization_limit() {
        let sql = bounded_select_sql("SELECT * FROM roads", 10);
        assert_eq!(
            sql,
            "SELECT * FROM (SELECT * FROM roads) AS zf_table_query_limited LIMIT 11"
        );
        assert_eq!(
            bounded_select_sql("SELECT * FROM roads", 0),
            "SELECT * FROM (SELECT * FROM roads) AS zf_table_query_limited LIMIT 0"
        );
    }
}

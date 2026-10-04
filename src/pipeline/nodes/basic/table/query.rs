//! `table.query.run` — SQL over files in a project store, as if they were
//! tables (GeoDataFusion: DataFusion with the `ST_*` functions).
//!
//! The query shape is the db nodes' (`shared/query.rs`): `--query` or the body
//! after `--`, `--param 1=…` binding `$1`, `--limit` capping the rows
//! answered. Each `--from "<source> as <name>"` binds one table: a store key,
//! `$expr` giving a FileRef or rows. Only SELECT and WITH run.
//!
//! Without a destination the answer is the db nodes' `query: { rows, columns,
//! row_count, truncated }`. With one (`--folder`, `--filename` or `--path`)
//! the whole result — at most [`MAX_FILE_ROWS`] rows — is written as
//! `--format`, and `query` holds the file's FileRef fields with `row_count`
//! (rows written), `columns` and `format`; `--rows` adds the first `--limit`
//! rows and `truncated`.

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
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem, NodeCapability, NodeExample, NodeFieldDef, NodeFieldType};
use crate::pipeline::nodes::shared::file_ref::zebfs_rel_path;
use crate::pipeline::nodes::shared::limits::choice;
use crate::pipeline::nodes::shared::query::{self, Params};
use crate::pipeline::nodes::shared::util::{eval_deno_expr, metadata_scope, with_answer};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;
use crate::zebfs::normalize_object_path;

use super::FORMAT_WORDS;
use super::convert::{TableFormat, collect_columns, encode_rows, parse_format, record_batch_to_rows};

pub const NODE_KIND: &str = "table.query.run";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

/// Reading a source or writing the result failed.
pub const CODE: &str = "FW_NODE_TABLE_QUERY_RUN";
/// A flag the author set wrong.
pub const CONFIG_CODE: &str = "FW_NODE_TABLE_QUERY_RUN_CONFIG";
/// The engine has no platform to read stores through.
pub const UNAVAILABLE_CODE: &str = "FW_NODE_TABLE_QUERY_RUN_UNAVAILABLE";
/// `--param` keys that are not `1..n`.
const PARAM_CODE: &str = "FW_NODE_TABLE_QUERY_RUN_PARAM";
/// `--limit` out of range, or a result too large to write.
const LIMIT_CODE: &str = "FW_NODE_TABLE_QUERY_RUN_LIMIT";
/// A `--from` binding that names no readable table.
const SOURCE_CODE: &str = "FW_NODE_TABLE_QUERY_RUN_SOURCE";
/// A source DataFusion could not register.
const REGISTER_CODE: &str = "FW_NODE_TABLE_QUERY_RUN_REGISTER";
/// The SQL was refused or failed.
const SQL_CODE: &str = "FW_NODE_TABLE_QUERY_RUN_SQL";
const PREPARE_CODE: &str = "FW_NODE_TABLE_QUERY_RUN_PREPARE";
const EXECUTE_CODE: &str = "FW_NODE_TABLE_QUERY_RUN_EXECUTE";

/// The most rows one run writes to a file; a larger result is refused.
pub const MAX_FILE_ROWS: usize = 10_000;
/// The most `--from` tables one query binds.
const MAX_SOURCES: u32 = 32;

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Network, NodeCapability::Filesystem, NodeCapability::Database, NodeCapability::Process],
        title: "Table Query".to_string(),
        description: "Runs SQL across CSV, JSON, NDJSON and Parquet files in a project store as if they were tables. It has the \
            GeoDataFusion `ST_*` functions. Each `--from \"<source> as <name>\"` binds one table: a store key, or `$expr` giving a FileRef or rows. \
            The SQL is the body after `--` (or `--query`), SELECT or WITH only; `--param 1=…` binds `$1` (a whole `{{ expr }}` keeps its type). \
            Without a destination it adds `query: { rows, columns, row_count, truncated }`, as the db nodes do — `--limit` caps the rows (default 200, \
            at most 5000). With `--folder` / `--filename` / `--path` the whole result (at most 10000 rows) is written as `--format` and `query` holds \
            the file's FileRef fields with `row_count` (rows written), `columns` and `format`; `--rows` adds the first `--limit` rows and `truncated`. \
            For database tables use `postgres.query.run`, `sqlite.query.run` or `sekejap.query.run`."
            .to_string(),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        input_schema: json!({
            "type": "object",
            "description": "Input context for `$expr` sources; values reach the SQL only through --param."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "object",
                    "properties": {
                        "rows": { "type": "array", "description": "One object per row; without a destination, or with --rows" },
                        "columns": { "type": "array", "items": { "type": "string" } },
                        "row_count": { "type": "integer", "description": "Rows answered, or rows written when a file is written" },
                        "truncated": { "type": "boolean", "description": "More rows than --limit answered" },
                        "ref": { "type": "string", "description": "The written file's store key; with a destination" },
                        "format": { "type": "string", "description": "What the written file is; with a destination" }
                    }
                }
            }
        }),
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag {
                flag: "--from".to_string(),
                config_key: "from".to_string(),
                description: "One table, repeated: \"<source> as <name>\" — a store key, or $expr giving a FileRef or rows.".to_string(),
                kind: DslFlagKind::RepeatedList,
                required: true,
                value: "text".to_string(),
                max_repeat: Some(MAX_SOURCES),
                ..Default::default()
            },
            query::query_flag("SQL (SELECT or WITH)"),
            query::param_flag("`1=…` binds $1, `2=…` binds $2"),
            query::limit_flag(),
            super::format_flag(),
            super::rows_flag(),
        ]
        .into_iter()
        .chain(super::destination_flags())
        .collect(),
        fields: vec![
            NodeFieldDef {
                name: "from".to_string(),
                label: "From".to_string(),
                field_type: NodeFieldType::SourceBindings,
                span: Some("full".to_string()),
                help: Some("Bind each store key or $expr to a SQL table name.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "query".to_string(),
                label: "Query".to_string(),
                field_type: NodeFieldType::CodeEditor,
                language: Some("sql".to_string()),
                span: Some("full".to_string()),
                help: Some("SELECT or WITH over the table names; $1, $2 bind from Params.".to_string()),
                default_value: Some(json!("SELECT *\nFROM posts\nLIMIT 20")),
                ..Default::default()
            },
        ]
        .into_iter()
        .chain(query::fields("1 binds $1, 2 binds $2").into_iter().filter(|field| field.name != "write"))
        .chain([
            super::format_field("format", "Format", "What the written file is. Auto: from the destination's extension, else csv."),
            super::rows_field(),
        ])
        .chain(super::destination_fields())
        .collect(),
        layout: vec![
            LayoutItem::Field("from".to_string()),
            LayoutItem::Field("query".to_string()),
            LayoutItem::Field("param".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("limit".to_string()), LayoutItem::Field("rows".to_string())] },
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("folder".to_string()),
                    LayoutItem::Field("filename".to_string()),
                    LayoutItem::Field("path".to_string()),
                ],
            },
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("store".to_string()),
                    LayoutItem::Field("on_conflict".to_string()),
                    LayoutItem::Field("format".to_string()),
                ],
            },
        ],
        ai_tool: Default::default(),
        examples: vec![
            NodeExample::dsl("Aggregate a CSV", r#"table.query.run --from "uploads/sales.csv as sales" -- "SELECT region, SUM(amount) AS total FROM sales GROUP BY region ORDER BY total DESC""#)
                .output(json!({ "query": { "rows": [{ "region": "AU", "total": 1200 }, { "region": "NZ", "total": 300 }], "columns": ["region", "total"], "row_count": 2, "truncated": false } })),
            NodeExample::dsl("Join two files into Parquet", r#"table.query.run --from "datasets/orders.ndjson as o" --from "datasets/customers.csv as c" --path datasets/report.parquet -- "SELECT c.name, COUNT(*) AS orders FROM o JOIN c ON o.customer_id = c.id GROUP BY c.name""#)
                .output(json!({ "query": { "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "datasets/report.parquet", "filename": "report.parquet", "mime": "application/vnd.apache.parquet", "kind": "parquet", "size": 2048, "sha256": "sha256:…", "lifecycle": "durable", "origin": NODE_KIND, "trust": "generated", "row_count": 42, "columns": ["name", "orders"], "format": "parquet" } }))
                .note("The whole result is the file; `input.query` is its FileRef. Add `--rows` to answer the first --limit rows as well."),
            NodeExample::dsl("Filter by a bound value", r#"table.query.run --from "datasets/posts.parquet as posts" --param "1={{ $trigger.params.slug }}" -- "SELECT id, title FROM posts WHERE slug = $1""#),
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
            SourceBindingConfig::Ui { source, alias } => source.trim().is_empty() && alias.trim().is_empty(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// The tables, each `"<source> as <name>"` (or the editor's `{ source, alias }`).
    #[serde(default)]
    pub from: Vec<SourceBindingConfig>,
    /// The statement: SELECT or WITH.
    #[serde(default)]
    pub query: String,
    /// `{ "1": value }` binds `$1`.
    #[serde(default)]
    pub param: Value,
    /// Rows answered.
    #[serde(default)]
    pub limit: Value,
    /// What a written file is: csv, json, ndjson or parquet.
    #[serde(default)]
    pub format: String,
    /// With a destination, answer the rows as well as the file.
    #[serde(default)]
    pub rows: bool,
    /// Store folder for the written file (default: `tables`).
    #[serde(default)]
    pub folder: String,
    /// Name of the written file (default: a UUID with the format's extension).
    #[serde(default)]
    pub filename: Option<String>,
    /// Exact store key; overrides `folder` and `filename`.
    #[serde(default)]
    pub path: Option<String>,
    /// The store keys are read from and a file is written to; saved explicitly at registration.
    #[serde(default)]
    pub store: Option<String>,
    /// `overwrite`, `skip` or `error` (default: error).
    #[serde(default)]
    pub on_conflict: Option<String>,
}

impl Config {
    fn destination(&self) -> super::TableDestination<'_> {
        super::TableDestination { folder: &self.folder, filename: self.filename.as_deref(), path: self.path.as_deref() }
    }
}

pub struct Node {
    config: Config,
    platform: Arc<PlatformService>,
    /// `--from "$expr as name"` evaluates a source expression.
    language: Arc<dyn LanguageEngine>,
}

impl Node {
    pub fn new(config: Config, platform: Arc<PlatformService>, language: Arc<dyn LanguageEngine>) -> Result<Self, PipelineError> {
        if config.from.iter().all(SourceBindingConfig::is_empty) {
            return Err(PipelineError::new(CONFIG_CODE, "--from is required: one \"<source> as <name>\" per table"));
        }
        if config.from.len() > MAX_SOURCES as usize {
            return Err(PipelineError::new(CONFIG_CODE, format!("--from binds at most {MAX_SOURCES} tables")));
        }
        if config.query.trim().is_empty() {
            return Err(PipelineError::new(CONFIG_CODE, "--query (or the body after --) must not be empty"));
        }
        Ok(Self { config, platform, language })
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

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        // Arrives final — `{{ }}` resolved engine-side before this ran.
        let sql = normalize_select_sql(&self.config.query)?;
        let params = match query::params(&self.config.param, false, PARAM_CODE)? {
            Params::Positional(values) => values,
            Params::Named(_) => unreachable!("named keys are refused for this engine"),
        };
        let limit = query::limit(&self.config.limit, LIMIT_CODE)?;
        let format_word = choice(&self.config.format, FORMAT_WORDS, "", "--format", CONFIG_CODE)?;
        let destination = self.config.destination();
        let (owner, project, ..) = metadata_scope(&input.metadata)?;
        let store = crate::pipeline::nodes::shared::project_store::open_store(&self.platform, owner, project, self.config.store.as_deref())?;
        let source_stores = SourceStores { platform: &self.platform, owner, project, store_id: &store.id };

        // Without a file, one row past --limit says whether more matched; a
        // file takes the whole result, one row past its ceiling refused.
        let fetch = if destination.requested() { MAX_FILE_ROWS } else { limit };
        let mut rows =
            execute_geodatafusion_engine(&self.config.from, &sql, &params, &source_stores, &input, self.language.as_ref(), fetch).await?;
        let columns = collect_columns(&rows);

        let mut answer = Map::new();
        if destination.requested() {
            if rows.len() > MAX_FILE_ROWS {
                return Err(PipelineError::new(
                    LIMIT_CODE,
                    format!("the result is over {MAX_FILE_ROWS} rows; aggregate or add a SQL LIMIT before writing it"),
                ));
            }
            let default_word = if format_word.is_empty() { "csv" } else { format_word };
            let key = destination.key(default_word, CONFIG_CODE)?;
            let format = output_format(format_word, &key)?;
            let bytes = encode_rows(&rows, &columns, format, CODE)?;
            let file = super::write_table_file(
                &self.platform,
                owner,
                project,
                self.config.store.as_deref(),
                self.config.on_conflict.as_deref(),
                &key,
                bytes,
                super::table_mime(format.as_str()),
                NODE_KIND,
                CODE,
            )?;
            if let Value::Object(file) = file {
                answer.extend(file);
            }
            answer.insert("row_count".to_string(), json!(rows.len()));
            answer.insert("columns".to_string(), json!(columns));
            answer.insert("format".to_string(), json!(format.as_str()));
            if self.config.rows {
                answer.insert("truncated".to_string(), json!(rows.len() > limit));
                rows.truncate(limit);
                answer.insert("rows".to_string(), Value::Array(rows));
            }
        } else {
            let truncated = rows.len() > limit;
            rows.truncate(limit);
            if let Value::Object(query) = query::answer(columns, rows, truncated, None)["query"].take() {
                answer = query;
            }
        }
        let row_count = answer.get("row_count").cloned().unwrap_or(Value::Null);
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: with_answer(&input.payload, json!({ "query": answer })),
            trace: vec![format!("node_kind={NODE_KIND} rows={row_count}")],
        })
    }
}

/// Binds every source and runs the statement, answering at most `fetch + 1`
/// rows: one past `fetch` tells the caller more matched.
async fn execute_geodatafusion_engine(
    sources: &[SourceBindingConfig],
    sql: &str,
    params: &[Value],
    stores: &SourceStores<'_>,
    input: &NodeExecutionInput,
    language: &dyn LanguageEngine,
    fetch: usize,
) -> Result<Vec<Value>, PipelineError> {
    let ctx = SessionContext::new();
    geodatafusion::register(&ctx);
    let mut temps = Vec::new();
    for source_config in sources.iter().filter(|source| !source.is_empty()) {
        let binding = source_config.to_binding()?;
        register_source(&ctx, stores, &binding, input, language, &mut temps).await?;
    }

    let bounded_sql = bounded_select_sql(sql, fetch);
    let batches = execute_geodatafusion_query(&ctx, &bounded_sql, params).await?;
    let mut rows = Vec::new();
    for batch in batches {
        rows.extend(record_batch_to_rows(&batch, CODE)?);
    }
    Ok(rows)
}

fn bounded_select_sql(sql: &str, fetch: usize) -> String {
    let fetch = fetch.saturating_add(1);
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
                SOURCE_CODE,
                "source binding must include both source and alias",
            ));
        }
        if !valid_alias(alias) {
            return Err(PipelineError::new(
                SOURCE_CODE,
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
            SOURCE_CODE,
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

/// Where table sources are read from: the node's store, or the store a
/// FileRef names, each through the project's mirror.
struct SourceStores<'a> {
    platform: &'a PlatformService,
    owner: &'a str,
    project: &'a str,
    store_id: &'a str,
}

impl SourceStores<'_> {
    fn local_path(&self, store: Option<&str>, key: &str) -> Result<std::path::PathBuf, PipelineError> {
        self.platform
            .file
            .object_local_path(self.owner, self.project, Some(store.unwrap_or(self.store_id)), key)
            .map_err(|err| PipelineError::new(SOURCE_CODE, format!("'{key}': {}", err.message)))
    }
}

async fn register_source(
    ctx: &SessionContext,
    stores: &SourceStores<'_>,
    binding: &SourceBinding,
    input: &NodeExecutionInput,
    language: &dyn LanguageEngine,
    temps: &mut Vec<tempfile::NamedTempFile>,
) -> Result<(), PipelineError> {
    if binding.source.trim_start().starts_with('$') {
        let value = eval_deno_expr(language, &binding.source, &input.payload, &input.metadata)?;
        if let Some(path) = zebfs_rel_path(&value)? {
            // A FileRef is read from the store it names, not the node's.
            let store = value.get("store").and_then(Value::as_str);
            register_table_path(ctx, stores, store, binding.alias.as_str(), &path).await?;
            return Ok(());
        }
        let rows = rows_from_json_value(value);
        let mut temp = tempfile::Builder::new()
            .suffix(".json")
            .tempfile()
            .map_err(|err| PipelineError::new(CODE, err.to_string()))?;
        for row in rows {
            writeln!(
                temp,
                "{}",
                serde_json::to_string(&row)
                    .map_err(|err| PipelineError::new(CODE, err.to_string()))?
            )
            .map_err(|err| PipelineError::new(CODE, err.to_string()))?;
        }
        ctx.register_json(
            binding.alias.as_str(),
            temp.path().to_string_lossy(),
            JsonReadOptions::default(),
        )
        .await
        .map_err(|err| PipelineError::new(REGISTER_CODE, err.to_string()))?;
        temps.push(temp);
        return Ok(());
    }

    register_table_path(ctx, stores, None, binding.alias.as_str(), &binding.source).await
}

async fn register_table_path(
    ctx: &SessionContext,
    stores: &SourceStores<'_>,
    store: Option<&str>,
    alias: &str,
    source: &str,
) -> Result<(), PipelineError> {
    // A table is a file in a project store. A URL is refused: a bucket is
    // read through its registered store, the web through http.response.fetch — never
    // past the egress guard and the store's own credentials.
    if is_external_table_uri(source) {
        return Err(PipelineError::new(
            SOURCE_CODE,
            format!("'{source}' is a URL; register the bucket as a store and pass a key or FileRef, or fetch it with http.response.fetch first"),
        ));
    }
    let (format_label, table_path) = {
        let rel = normalize_object_path(source)
            .map_err(|err| PipelineError::new(CODE, err.to_string()))?;
        // DataFusion streams from a file path: a directory store's own file,
        // or a bucket object's copy in the project's mirror.
        let abs = stores.local_path(store, &rel)?;
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
    .map_err(|err| PipelineError::new(REGISTER_CODE, err.to_string()))
}

fn ensure_local_table_file(path: &Path) -> Result<(), PipelineError> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(()),
        Ok(_) => Err(PipelineError::new(
            CODE,
            "table source path is not a file",
        )),
        Err(_) => Err(PipelineError::new(
            CODE,
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
            .map_err(|err| PipelineError::new(SQL_CODE, err.to_string()))?
            .collect()
            .await
            .map_err(|err| PipelineError::new(SQL_CODE, err.to_string()));
    }

    let prepare_sql = format!("PREPARE zf_table_query AS {sql}");
    ctx.sql(&prepare_sql)
        .await
        .map_err(|err| PipelineError::new(PREPARE_CODE, err.to_string()))?
        .collect()
        .await
        .map_err(|err| PipelineError::new(PREPARE_CODE, err.to_string()))?;

    let param_sql = params
        .iter()
        .map(datafusion_literal)
        .collect::<Vec<_>>()
        .join(", ");
    let execute_sql = format!("EXECUTE zf_table_query({param_sql})");
    ctx.sql(&execute_sql)
        .await
        .map_err(|err| PipelineError::new(EXECUTE_CODE, err.to_string()))?
        .collect()
        .await
        .map_err(|err| PipelineError::new(EXECUTE_CODE, err.to_string()))
}

fn normalize_select_sql(sql: &str) -> Result<String, PipelineError> {
    let sql = sql.trim().trim_end_matches(';').trim().to_string();
    let lower = sql.trim_start().to_ascii_lowercase();
    if !(lower.starts_with("select") || lower.starts_with("with")) {
        return Err(PipelineError::new(
            SQL_CODE,
            "table.query.run only accepts SELECT or WITH queries",
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

/// A source's format, from its key's extension.
fn source_format(path: &str) -> Result<TableFormat, PipelineError> {
    let ext = Path::new(path).extension().and_then(|value| value.to_str()).unwrap_or_default();
    parse_format(ext, SOURCE_CODE)
        .map_err(|_| PipelineError::new(SOURCE_CODE, format!("'{path}' is not a table file: name it .csv, .json, .ndjson or .parquet")))
}

/// What a written file is: `--format`, else the destination's extension.
fn output_format(word: &str, key: &str) -> Result<TableFormat, PipelineError> {
    if !word.is_empty() {
        return parse_format(word, CONFIG_CODE);
    }
    let ext = Path::new(key).extension().and_then(|value| value.to_str()).unwrap_or_default();
    parse_format(ext, CONFIG_CODE)
        .map_err(|_| PipelineError::new(CONFIG_CODE, format!("'{key}' names no table format; set --format csv|json|ndjson|parquet")))
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
    fn the_query_shape_is_the_db_nodes() {
        let graph = crate::platform::shell::parser::build_pipeline_graph(
            "t",
            "[a] trigger.manual\n[b] table.query.run --from \"datasets/orders.csv as o\" --param \"1=7\" --limit 5 --rows --format parquet -- \"SELECT * FROM o WHERE id = $1\"\n[a] -> [b]\n",
        )
        .expect("graph");
        let config: Config = serde_json::from_value(graph.nodes[1].config.clone()).expect("config");
        assert_eq!(config.from.len(), 1);
        assert_eq!(config.query, "SELECT * FROM o WHERE id = $1");
        assert_eq!(query::params(&config.param, false, PARAM_CODE).unwrap(), Params::Positional(vec![json!("7")]), "a literal is text");
        assert_eq!(query::limit(&config.limit, LIMIT_CODE).unwrap(), 5);
        assert!(config.rows);
        assert_eq!(config.format, "parquet");

        let flags: Vec<String> = definition().dsl_flags.into_iter().map(|f| f.flag).collect();
        for gone in ["--engine", "--params", "--to-json", "--preview-rows", "--write"] {
            assert!(!flags.iter().any(|f| f == gone), "{gone} is gone: {flags:?}");
        }
        assert!(query::params(&json!({ "slug": "a" }), false, PARAM_CODE).is_err(), "DataFusion binds positions only");
    }

    #[test]
    fn only_a_read_runs_and_a_written_format_is_named() {
        assert_eq!(normalize_select_sql("  WITH t AS (SELECT 1) SELECT * FROM t; ").unwrap(), "WITH t AS (SELECT 1) SELECT * FROM t");
        assert_eq!(normalize_select_sql("DELETE FROM t").unwrap_err().code, SQL_CODE);
        assert_eq!(output_format("", "out/a.ndjson").unwrap(), TableFormat::Ndjson);
        assert_eq!(output_format("csv", "out/a.parquet").unwrap(), TableFormat::Csv);
        assert_eq!(output_format("", "out/a.xlsx").unwrap_err().code, CONFIG_CODE);
        assert_eq!(source_format("uploads/a.txt").unwrap_err().code, SOURCE_CODE);
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
            "SELECT * FROM (SELECT * FROM roads) AS zf_table_query_limited LIMIT 1"
        );
    }
}

//! `table.data.convert` — rows between formats, and between the payload and a
//! project store.
//!
//! `--from` is the table: a store key, a FileRef (read from the store it
//! names), or the rows themselves through `{{ }}`. `--parse` says how it is
//! read (else from the key's extension, or JSON for rows); `--format` says
//! what a written file is (else from the destination's extension, else csv).
//! A destination (`--folder`, `--filename` or `--path`) writes a file; without
//! one the rows are the answer. CSV to Parquet streams without holding the
//! rows when they are not asked for.
//!
//! The answer is one key, `data`: the written file's FileRef fields (when a
//! file is written) with `row_count`, `columns`, `parse`, `format`, and `rows`
//! when there is no destination or `--rows` is set.

use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use datafusion::arrow::{
    array::{
        Array, ArrayRef, BooleanArray, BooleanBuilder, Float32Array, Float64Array, Float64Builder,
        Int8Array, Int16Array, Int32Array, Int64Array, Int64Builder, LargeStringArray, StringArray,
        StringBuilder, UInt8Array, UInt16Array, UInt32Array, UInt64Array,
    },
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
    util::display::array_value_to_string,
};
use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::prelude::{CsvReadOptions, SessionContext};
use parquet::{
    arrow::{ArrowWriter, arrow_reader::ParquetRecordBatchReaderBuilder},
    basic::Compression,
    file::properties::WriterProperties,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::pipeline::model::NodeCapability;
use crate::pipeline::nodes::shared::file_ref::is_file_ref;
use crate::pipeline::nodes::shared::limits::{choice, whole};
use crate::pipeline::nodes::shared::project_store::{NodeStore, OnConflict, open_from, open_store, read_capped};
use crate::pipeline::nodes::shared::util::{metadata_scope, with_answer};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeExample, NodeFieldDef, NodeFieldType},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;
use crate::zebfs::ZebFs;

use super::FORMAT_WORDS;

pub const NODE_KIND: &str = "table.data.convert";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

/// Reading, converting or writing failed.
pub const CODE: &str = "FW_NODE_TABLE_DATA_CONVERT";
/// A flag the author set wrong.
pub const CONFIG_CODE: &str = "FW_NODE_TABLE_DATA_CONVERT_CONFIG";
/// The streamed CSV could not be read.
const CSV_CODE: &str = "FW_NODE_TABLE_DATA_CONVERT_CSV";
/// A source too large to hold as rows.
const LIMIT_CODE: &str = "FW_NODE_TABLE_DATA_CONVERT_LIMIT";
/// The streamed Parquet could not be written.
const PARQUET_CODE: &str = "FW_NODE_TABLE_DATA_CONVERT_PARQUET";

const MAX_MATERIALIZED_OBJECT_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// The table: a store key, a FileRef, or the rows themselves — typed,
    /// because all three arrive through `{{ }}`.
    #[serde(default)]
    pub from: Value,
    /// How `from` is read: csv, json, ndjson or parquet (else sniffed).
    #[serde(default)]
    pub parse: String,
    /// What a written file is: csv, json, ndjson or parquet (else from the
    /// destination's extension, else csv).
    #[serde(default)]
    pub format: String,
    /// With a destination, answer the rows as well as the file.
    #[serde(default)]
    pub rows: bool,
    /// The most rows converted (default: all).
    #[serde(default)]
    pub limit: Value,
    /// Store folder for the written file (default: `tables`).
    #[serde(default)]
    pub folder: String,
    /// Name of the written file (default: a UUID with the format's extension).
    #[serde(default)]
    pub filename: Option<String>,
    /// Exact store key; overrides `folder` and `filename`.
    #[serde(default)]
    pub path: Option<String>,
    /// The store a key is read from and a file written to; saved explicitly at registration.
    #[serde(default)]
    pub store: Option<String>,
    /// `overwrite`, `skip` or `error` (default: error).
    #[serde(default)]
    pub on_conflict: Option<String>,
}

pub fn definition() -> NodeDefinition {
    let file = json!({ "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "exports/orders.csv", "filename": "orders.csv", "mime": "text/csv", "kind": "csv", "size": 4120, "sha256": "sha256:…", "lifecycle": "durable", "origin": NODE_KIND, "trust": "generated" });
    let mut written = file.clone();
    written["row_count"] = json!(120);
    written["columns"] = json!(["id", "total"]);
    written["parse"] = json!("json");
    written["format"] = json!("csv");
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem, NodeCapability::Database, NodeCapability::Process],
        title: "Table Convert".to_string(),
        description: "Convert table-shaped data between formats, and between the payload and a project store. `--from` is a store key, \
            a FileRef (read from the store it names) or the rows themselves (`{{ input.query.rows }}`); `--parse csv|json|ndjson|parquet` says how \
            it is read (default: from the key's extension, or JSON for rows). `--folder` / `--filename` / `--path` write a file, as `--format` \
            (default: from the destination's extension, else csv); without a destination the rows are the answer. Adds \
            `data: { …FileRef…, row_count, columns, parse, format, rows? }` — the FileRef fields only when a file is written, `rows` when there \
            is no destination or `--rows` is set — and keeps the rest of the payload. `--limit N` converts the first N rows. This is how a query \
            result becomes a downloadable CSV, and an uploaded CSV becomes rows a script can read."
            .to_string(),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        input_schema: json!({
            "type": "object",
            "description": "Any payload; --from \"{{ input.query.rows }}\" takes rows from upstream."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "data": {
                    "type": "object",
                    "description": "The written file's FileRef fields (when a file is written), with what was converted.",
                    "properties": {
                        "ref": { "type": "string", "description": "The written file's store key; present with a destination" },
                        "store": { "type": "string" },
                        "row_count": { "type": "integer", "description": "Rows converted" },
                        "columns": { "type": "array", "items": { "type": "string" } },
                        "parse": { "type": "string", "description": "How --from was read" },
                        "format": { "type": "string", "description": "What the written file is; present with a destination" },
                        "rows": { "type": "array", "description": "The rows; present without a destination or with --rows" }
                    }
                }
            }
        }),
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag {
                flag: "--from".to_string(),
                config_key: "from".to_string(),
                description: "The table: a store key, a FileRef, or the rows themselves through {{ }}.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
                value: "file".to_string(),
                ..Default::default()
            },
            DslFlag {
                flag: "--parse".to_string(),
                config_key: "parse".to_string(),
                description: "How --from is read: csv, json, ndjson or parquet (default: from the key's extension, or JSON for rows).".to_string(),
                kind: DslFlagKind::Scalar,
                choices: FORMAT_WORDS.iter().map(|w| w.to_string()).collect(),
                ..Default::default()
            },
            super::format_flag(),
            super::rows_flag(),
            DslFlag {
                flag: "--limit".to_string(),
                config_key: "limit".to_string(),
                description: "The most rows converted, from the first (default: all).".to_string(),
                kind: DslFlagKind::Scalar,
                value: "number".to_string(),
                ..Default::default()
            },
        ]
        .into_iter()
        .chain(super::destination_flags())
        .collect(),
        fields: vec![
            NodeFieldDef {
                name: "from".to_string(),
                label: "From".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("uploads/data.csv".to_string()),
                help: Some("A store key, a FileRef or rows, e.g. {{ input.query.rows }}.".to_string()),
                ..Default::default()
            },
            super::format_field("parse", "Parse", "How From is read. Auto: from the key's extension, or JSON for rows."),
            super::format_field("format", "Format", "What the written file is. Auto: from the destination's extension, else csv."),
            super::rows_field(),
            NodeFieldDef {
                name: "limit".to_string(),
                label: "Limit".to_string(),
                field_type: NodeFieldType::Number,
                help: Some("The most rows converted. Empty: all.".to_string()),
                ..Default::default()
            },
        ]
        .into_iter()
        .chain(super::destination_fields())
        .collect(),
        layout: vec![
            LayoutItem::Field("from".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("parse".to_string()), LayoutItem::Field("limit".to_string())] },
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("folder".to_string()),
                    LayoutItem::Field("filename".to_string()),
                    LayoutItem::Field("path".to_string()),
                ],
            },
            LayoutItem::Row { row: vec![LayoutItem::Field("store".to_string()), LayoutItem::Field("on_conflict".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("format".to_string()), LayoutItem::Field("rows".to_string())] },
        ],
        script_available: false,
        script_bridge: None,
        ai_tool: Default::default(),
        examples: vec![
            NodeExample::dsl("Query result to a CSV download", r#"table.data.convert --from "{{ input.query.rows }}" --folder exports --filename orders.csv"#)
                .output(json!({ "data": written })),
            NodeExample::dsl("Uploaded CSV to rows", r#"table.data.convert --from "{{ $trigger.files.sheet }}" --parse csv --limit 500"#)
                .output(json!({ "data": { "row_count": 2, "columns": ["name", "email"], "parse": "csv", "rows": [{ "name": "Ana", "email": "ana@example.com" }, { "name": "Ben", "email": "ben@example.com" }] } }))
                .note("After `trigger.webhook` with a multipart field `sheet`; the next node reads `input.data.rows`. A CSV cell is text."),
        ],
        ..Default::default()
    }
}

pub struct Node {
    config: Config,
    platform: Arc<PlatformService>,
}

impl Node {
    pub fn new(config: Config, platform: Arc<PlatformService>) -> Result<Self, PipelineError> {
        Ok(Self { config, platform })
    }

    fn destination(&self) -> super::TableDestination<'_> {
        self.config.destination()
    }
}

/// The flags read and checked once, before anything is opened.
struct Settings {
    /// `--parse`, `None` to sniff.
    parse: Option<TableFormat>,
    /// `--format`, `None` to take it from the destination.
    format: Option<TableFormat>,
    limit: Option<usize>,
    /// The answer carries the rows.
    answer_rows: bool,
}

impl Config {
    fn destination(&self) -> super::TableDestination<'_> {
        super::TableDestination { folder: &self.folder, filename: self.filename.as_deref(), path: self.path.as_deref() }
    }

    fn settings(&self) -> Result<Settings, PipelineError> {
        let word = |value: &str, flag: &str| -> Result<Option<TableFormat>, PipelineError> {
            let word = choice(value, FORMAT_WORDS, "", flag, CONFIG_CODE)?;
            if word.is_empty() { Ok(None) } else { parse_format(word, CONFIG_CODE).map(Some) }
        };
        let limit = match whole(&self.limit, "--limit", CONFIG_CODE)? {
            Some(0) => return Err(PipelineError::new(CONFIG_CODE, "--limit 0 converts nothing; leave it out to convert every row")),
            other => other.map(|n| n as usize),
        };
        let requested = self.destination().requested();
        Ok(Settings {
            parse: word(&self.parse, "--parse")?,
            format: word(&self.format, "--format")?,
            limit,
            answer_rows: self.rows || !requested,
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
        let (owner, project, ..) = metadata_scope(&input.metadata)?;
        let settings = self.config.settings()?;
        let destination = self.destination();

        if let Some(data) = self.try_streaming_file_conversion(owner, project, &settings).await? {
            return Ok(answer(&input.payload, data, "streamed=true".to_string()));
        }

        let source = self.read_source(owner, project, settings.parse)?;
        let parse = source.format;
        let mut rows = rows_from_source(source.value, parse)?;
        if let Some(limit) = settings.limit {
            rows.truncate(limit);
        }
        let columns = collect_columns(&rows);

        let mut data = Map::new();
        if destination.requested() {
            let default_word = settings.format.map(TableFormat::as_str).unwrap_or("csv");
            let key = destination.key(default_word, CONFIG_CODE)?;
            let format = match settings.format {
                Some(format) => format,
                None => normalize_format(&key, "the written file")?,
            };
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
                data.extend(file);
            }
            data.insert("format".to_string(), json!(format.as_str()));
        }
        data.insert("row_count".to_string(), json!(rows.len()));
        data.insert("columns".to_string(), json!(columns));
        data.insert("parse".to_string(), json!(parse.as_str()));
        let row_count = rows.len();
        if settings.answer_rows {
            data.insert("rows".to_string(), Value::Array(rows));
        }
        Ok(answer(&input.payload, data, format!("parse={} rows={row_count}", parse.as_str())))
    }
}

/// `data` added to the payload, the rest kept.
fn answer(payload: &Value, data: Map<String, Value>, trace: String) -> NodeExecutionOutput {
    NodeExecutionOutput {
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        payload: with_answer(payload, json!({ "data": data })),
        trace: vec![format!("node_kind={NODE_KIND} {trace}")],
    }
}

struct SourceData {
    value: SourceValue,
    format: TableFormat,
}

enum SourceValue {
    Bytes(Vec<u8>),
    Json(Value),
}

impl Node {
    /// `--from` arrives final and typed, so its shape says what it is: a
    /// string is a store key, a FileRef names one in its own store, anything
    /// else is the rows themselves.
    fn read_source(&self, owner: &str, project: &str, parse: Option<TableFormat>) -> Result<SourceData, PipelineError> {
        let from = &self.config.from;
        if from.is_null() || from.as_str().is_some_and(|s| s.trim().is_empty()) {
            return Err(PipelineError::new(CONFIG_CODE, "--from is required: a store key, a FileRef, or rows"));
        }
        if from.is_string() || is_file_ref(from) {
            let (store, key) = self.open_source(owner, project)?;
            let format = match parse {
                Some(format) => format,
                None => normalize_format(&key, "--from")?,
            };
            ensure_materialization_safe(&store.fs, &key)?;
            let bytes = read_capped(&store.fs, &key, CODE)?;
            return Ok(SourceData { value: SourceValue::Bytes(bytes), format });
        }
        // Rows: JSON unless --parse names a text format the value holds.
        let format = parse.unwrap_or(TableFormat::Json);
        Ok(SourceData { value: SourceValue::Json(from.clone()), format })
    }

    fn open_source(&self, owner: &str, project: &str) -> Result<(NodeStore, String), PipelineError> {
        open_from(&self.platform, owner, project, &self.config.from, self.config.store.as_deref(), "--from", CONFIG_CODE)
    }

    /// CSV in a store to a Parquet file, streamed by DataFusion without
    /// holding the rows — taken only when nothing asks for them.
    async fn try_streaming_file_conversion(
        &self,
        owner: &str,
        project: &str,
        settings: &Settings,
    ) -> Result<Option<Map<String, Value>>, PipelineError> {
        let destination = self.destination();
        if settings.answer_rows || !destination.requested() {
            return Ok(None);
        }
        let from = &self.config.from;
        if !(from.as_str().is_some_and(|s| !s.trim().is_empty()) || is_file_ref(from)) {
            return Ok(None);
        }
        let to_key = destination.key("parquet", CONFIG_CODE)?;
        let to_format = match settings.format {
            Some(format) => format,
            None => normalize_format(&to_key, "the written file")?,
        };
        let (source_store, source_key) = self.open_source(owner, project)?;
        let from_format = match settings.parse {
            Some(format) => format,
            None => normalize_format(&source_key, "--from")?,
        };
        if from_format != TableFormat::Csv || to_format != TableFormat::Parquet {
            return Ok(None);
        }
        let store = open_store(&self.platform, owner, project, self.config.store.as_deref())?;
        let on_conflict = OnConflict::parse(self.config.on_conflict.as_deref(), OnConflict::Error, CONFIG_CODE)?;
        if store.fs.head(&to_key).is_ok() && on_conflict != OnConflict::Overwrite {
            // skip and error both leave the streamed write; the materialised
            // path answers them with the same rule.
            return Ok(None);
        }

        // DataFusion streams from a local path: a directory store's own
        // file, or a bucket object's copy in the project's mirror.
        let abs_from = self
            .platform
            .file
            .object_local_path(owner, project, Some(&source_store.id), &source_key)
            .map_err(|err| PipelineError::new(CODE, format!("'{source_key}': {}", err.message)))?;
        ensure_local_table_file(&abs_from)?;

        // A directory store is written in place; a bucket gets the file built
        // in scratch and streamed up.
        let scratch = crate::pipeline::nodes::shared::store_scratch::StoreScratch::new(CODE)?;
        let (abs_to, staged) = match store.fs.local_path(&to_key).map_err(|err| PipelineError::new(CODE, err.to_string()))? {
            Some(path) => (path, false),
            None => (scratch.local("out.parquet"), true),
        };
        let columns = stream_csv_to_parquet(&abs_from, &abs_to, settings.limit).await?;
        let row_count = parquet_row_count(&abs_to)?;
        let leaf = to_key.rsplit('/').next().unwrap_or(&to_key).to_string();
        let file = store.file_ref_from_file(&to_key, &leaf, super::table_mime("parquet"), &abs_to, NODE_KIND, "generated", CODE)?;
        if staged {
            store
                .fs
                .put_from_file(&to_key, &abs_to)
                .map_err(|err| PipelineError::new(CODE, format!("write '{to_key}': {err}")))?;
        }

        let mut data = match file {
            Value::Object(file) => file,
            _ => Map::new(),
        };
        data.insert("format".to_string(), json!("parquet"));
        data.insert("row_count".to_string(), json!(row_count));
        data.insert("columns".to_string(), json!(columns));
        data.insert("parse".to_string(), json!("csv"));
        Ok(Some(data))
    }
}

/// The rows a Parquet file holds, read from its footer.
fn parquet_row_count(path: &Path) -> Result<u64, PipelineError> {
    let file = fs::File::open(path).map_err(|err| PipelineError::new(CODE, err.to_string()))?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file).map_err(|err| PipelineError::new(CODE, err.to_string()))?;
    Ok(builder.metadata().file_metadata().num_rows().max(0) as u64)
}

fn ensure_materialization_safe(zebfs: &ZebFs, rel_path: &str) -> Result<(), PipelineError> {
    let stat = zebfs
        .head(rel_path)
        .map_err(|err| PipelineError::new(CODE, err.to_string()))?;
    if stat.size > MAX_MATERIALIZED_OBJECT_BYTES {
        return Err(PipelineError::new(
            LIMIT_CODE,
            format!(
                "source object is {}; table.data.convert would materialize it in node JSON. Use the streamed CSV-to-Parquet path or reduce the input before converting",
                format_bytes(stat.size)
            ),
        ));
    }
    Ok(())
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

async fn stream_csv_to_parquet(
    from_abs: &Path,
    to_abs: &Path,
    limit: Option<usize>,
) -> Result<Vec<String>, PipelineError> {
    if let Some(parent) = to_abs.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| PipelineError::new(CODE, err.to_string()))?;
    }

    let ctx = SessionContext::new();
    let from_path = from_abs.to_string_lossy().into_owned();
    let mut df = ctx
        .read_csv(from_path.as_str(), CsvReadOptions::new().has_header(true))
        .await
        .map_err(|err| PipelineError::new(CSV_CODE, err.to_string()))?;
    if let Some(limit) = limit {
        df = df
            .limit(0, Some(limit))
            .map_err(|err| PipelineError::new(LIMIT_CODE, err.to_string()))?;
    }
    let columns = df
        .schema()
        .fields()
        .iter()
        .map(|field| field.name().to_string())
        .collect::<Vec<_>>();

    let tmp_abs = temp_output_path(to_abs, "parquet");
    let tmp_path = tmp_abs.to_string_lossy().into_owned();
    let write_result = df
        .write_parquet(
            tmp_path.as_str(),
            DataFrameWriteOptions::new().with_single_file_output(true),
            None,
        )
        .await;
    if let Err(err) = write_result {
        let _ = fs::remove_file(&tmp_abs);
        return Err(PipelineError::new(
            PARQUET_CODE,
            err.to_string(),
        ));
    }
    fs::rename(&tmp_abs, to_abs)
        .map_err(|err| PipelineError::new(CODE, err.to_string()))?;
    Ok(columns)
}

fn temp_output_path(path: &Path, extension: &str) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("table");
    path.with_file_name(format!(
        ".{file_name}.{}.tmp.{extension}",
        std::process::id()
    ))
}

fn format_bytes(size: u64) -> String {
    const MIB: u64 = 1024 * 1024;
    if size >= MIB {
        format!("{:.1} MiB", size as f64 / MIB as f64)
    } else {
        format!("{size} bytes")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TableFormat {
    Csv,
    Json,
    Ndjson,
    Parquet,
}

impl TableFormat {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Json => "json",
            Self::Ndjson => "ndjson",
            Self::Parquet => "parquet",
        }
    }
}

/// The format a key's extension names (`.csv`, `.json`, `.ndjson` or
/// `.jsonl`, `.parquet`); refused when it names none, asking for the flag.
fn normalize_format(path: &str, role: &str) -> Result<TableFormat, PipelineError> {
    let ext = Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let flag = if role == "--from" { "--parse" } else { "--format" };
    if ext.is_empty() {
        return Err(PipelineError::new(CONFIG_CODE, format!("the format of {role} is not known from its name; set {flag}")));
    }
    parse_format(ext, CONFIG_CODE)
        .map_err(|_| PipelineError::new(CONFIG_CODE, format!("'.{ext}' on {role} is not a table format; set {flag} csv|json|ndjson|parquet")))
}

/// A format word, raised under the calling node's `code`.
pub(crate) fn parse_format(value: &str, code: &'static str) -> Result<TableFormat, PipelineError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "csv" => Ok(TableFormat::Csv),
        "json" => Ok(TableFormat::Json),
        "ndjson" | "jsonl" => Ok(TableFormat::Ndjson),
        "parquet" => Ok(TableFormat::Parquet),
        other => Err(PipelineError::new(code, format!("unsupported table format '{other}'"))),
    }
}

fn rows_from_source(source: SourceValue, format: TableFormat) -> Result<Vec<Value>, PipelineError> {
    match (source, format) {
        (SourceValue::Json(value), TableFormat::Json) => rows_from_json_value(value),
        (SourceValue::Json(Value::String(text)), TableFormat::Csv) => rows_from_csv_text(&text),
        (SourceValue::Json(Value::String(text)), TableFormat::Ndjson) => {
            rows_from_ndjson_text(&text)
        }
        (SourceValue::Json(value), TableFormat::Csv) => rows_from_json_value(value),
        (SourceValue::Json(value), TableFormat::Ndjson) => rows_from_json_value(value),
        (SourceValue::Bytes(bytes), TableFormat::Csv) => {
            let text = String::from_utf8(bytes).map_err(|err| {
                PipelineError::new(CODE, format!("CSV is not UTF-8: {err}"))
            })?;
            rows_from_csv_text(&text)
        }
        (SourceValue::Bytes(bytes), TableFormat::Json) => {
            let value: Value = serde_json::from_slice(&bytes).map_err(|err| {
                PipelineError::new(CODE, format!("JSON parse error: {err}"))
            })?;
            rows_from_json_value(value)
        }
        (SourceValue::Bytes(bytes), TableFormat::Ndjson) => {
            let text = String::from_utf8(bytes).map_err(|err| {
                PipelineError::new(
                    CODE,
                    format!("NDJSON is not UTF-8: {err}"),
                )
            })?;
            rows_from_ndjson_text(&text)
        }
        (SourceValue::Bytes(bytes), TableFormat::Parquet) => rows_from_parquet_bytes(bytes),
        (SourceValue::Json(value), TableFormat::Parquet) => rows_from_json_value(value),
    }
}

fn rows_from_json_value(value: Value) -> Result<Vec<Value>, PipelineError> {
    match value {
        Value::Array(items) => Ok(items.into_iter().map(row_object).collect()),
        Value::Object(mut map) => {
            if let Some(Value::Array(rows)) = map.remove("rows") {
                return Ok(rows.into_iter().map(row_object).collect());
            }
            Ok(vec![Value::Object(map)])
        }
        Value::Null => Ok(Vec::new()),
        other => Ok(vec![json!({ "value": other })]),
    }
}

fn row_object(value: Value) -> Value {
    match value {
        Value::Object(_) => value,
        other => json!({ "value": other }),
    }
}

fn rows_from_csv_text(text: &str) -> Result<Vec<Value>, PipelineError> {
    let records = parse_csv_records(text)?;
    if records.is_empty() {
        return Ok(Vec::new());
    }
    let headers = records[0]
        .iter()
        .enumerate()
        .map(|(index, header)| {
            let trimmed = header.trim();
            if trimmed.is_empty() {
                format!("column_{}", index + 1)
            } else {
                trimmed.to_string()
            }
        })
        .collect::<Vec<_>>();
    let mut rows = Vec::new();
    for record in records.into_iter().skip(1) {
        if record.iter().all(|cell| cell.is_empty()) {
            continue;
        }
        let mut map = Map::new();
        for (index, header) in headers.iter().enumerate() {
            map.insert(
                header.clone(),
                Value::String(record.get(index).cloned().unwrap_or_default()),
            );
        }
        rows.push(Value::Object(map));
    }
    Ok(rows)
}

fn rows_from_ndjson_text(text: &str) -> Result<Vec<Value>, PipelineError> {
    let mut rows = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line).map_err(|err| {
            PipelineError::new(
                CODE,
                format!("NDJSON parse error on line {}: {err}", index + 1),
            )
        })?;
        rows.push(row_object(value));
    }
    Ok(rows)
}

/// Rows encoded as `format`, failures raised under the calling node's `code`.
pub(crate) fn encode_rows(
    rows: &[Value],
    columns: &[String],
    format: TableFormat,
    code: &'static str,
) -> Result<Vec<u8>, PipelineError> {
    let encoded = match format {
        TableFormat::Csv => Ok(encode_csv(rows, columns).into_bytes()),
        TableFormat::Json => serde_json::to_vec_pretty(rows).map_err(|err| PipelineError::new(CODE, err.to_string())),
        TableFormat::Ndjson => {
            let mut out = String::new();
            for row in rows {
                out.push_str(&serde_json::to_string(row).map_err(|err| PipelineError::new(CODE, err.to_string()))?);
                out.push('\n');
            }
            Ok(out.into_bytes())
        }
        TableFormat::Parquet => encode_parquet(rows, columns),
    };
    encoded.map_err(|err| PipelineError::new(code, err.message))
}

fn rows_from_parquet_bytes(bytes: Vec<u8>) -> Result<Vec<Value>, PipelineError> {
    let mut temp = tempfile::NamedTempFile::new()
        .map_err(|err| PipelineError::new(CODE, err.to_string()))?;
    temp.write_all(&bytes)
        .map_err(|err| PipelineError::new(CODE, err.to_string()))?;
    let file = temp
        .reopen()
        .map_err(|err| PipelineError::new(CODE, err.to_string()))?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|err| PipelineError::new(CODE, err.to_string()))?
        .build()
        .map_err(|err| PipelineError::new(CODE, err.to_string()))?;

    let mut rows = Vec::new();
    for batch in reader {
        let batch =
            batch.map_err(|err| PipelineError::new(CODE, err.to_string()))?;
        rows.extend(batch_rows(&batch)?);
    }
    Ok(rows)
}

fn encode_parquet(rows: &[Value], columns: &[String]) -> Result<Vec<u8>, PipelineError> {
    if columns.is_empty() {
        return Err(PipelineError::new(
            CODE,
            "parquet output needs at least one column",
        ));
    }
    let batch = rows_to_record_batch(rows, columns)?;
    let props = WriterProperties::builder()
        .set_compression(Compression::SNAPPY)
        .build();
    let mut buffer = Vec::new();
    {
        let mut writer = ArrowWriter::try_new(&mut buffer, batch.schema(), Some(props))
            .map_err(|err| PipelineError::new(CODE, err.to_string()))?;
        writer
            .write(&batch)
            .map_err(|err| PipelineError::new(CODE, err.to_string()))?;
        writer
            .close()
            .map_err(|err| PipelineError::new(CODE, err.to_string()))?;
    }
    Ok(buffer)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColumnKind {
    Boolean,
    Int64,
    Float64,
    Utf8,
}

fn rows_to_record_batch(rows: &[Value], columns: &[String]) -> Result<RecordBatch, PipelineError> {
    let kinds = columns
        .iter()
        .map(|column| infer_column_kind(rows, column))
        .collect::<Vec<_>>();
    let fields = columns
        .iter()
        .zip(kinds.iter())
        .map(|(column, kind)| Field::new(column, column_data_type(*kind), true))
        .collect::<Vec<_>>();
    let schema = Arc::new(Schema::new(fields));
    let arrays = columns
        .iter()
        .zip(kinds.iter())
        .map(|(column, kind)| build_arrow_array(rows, column, *kind))
        .collect::<Result<Vec<_>, _>>()?;
    RecordBatch::try_new(schema, arrays)
        .map_err(|err| PipelineError::new(CODE, err.to_string()))
}

fn column_data_type(kind: ColumnKind) -> DataType {
    match kind {
        ColumnKind::Boolean => DataType::Boolean,
        ColumnKind::Int64 => DataType::Int64,
        ColumnKind::Float64 => DataType::Float64,
        ColumnKind::Utf8 => DataType::Utf8,
    }
}

fn infer_column_kind(rows: &[Value], column: &str) -> ColumnKind {
    let mut kind: Option<ColumnKind> = None;
    for row in rows {
        let value = row
            .as_object()
            .and_then(|object| object.get(column))
            .unwrap_or(&Value::Null);
        let Some(next) = value_column_kind(value) else {
            continue;
        };
        kind = Some(match (kind, next) {
            (None, next) => next,
            (Some(ColumnKind::Int64), ColumnKind::Float64)
            | (Some(ColumnKind::Float64), ColumnKind::Int64)
            | (Some(ColumnKind::Float64), ColumnKind::Float64) => ColumnKind::Float64,
            (Some(existing), next) if existing == next => existing,
            _ => ColumnKind::Utf8,
        });
        if kind == Some(ColumnKind::Utf8) {
            break;
        }
    }
    kind.unwrap_or(ColumnKind::Utf8)
}

fn value_column_kind(value: &Value) -> Option<ColumnKind> {
    match value {
        Value::Null => None,
        Value::Bool(_) => Some(ColumnKind::Boolean),
        Value::Number(number) => {
            if number.as_i64().is_some() {
                Some(ColumnKind::Int64)
            } else if let Some(unsigned) = number.as_u64() {
                if i64::try_from(unsigned).is_ok() {
                    Some(ColumnKind::Int64)
                } else {
                    Some(ColumnKind::Float64)
                }
            } else {
                Some(ColumnKind::Float64)
            }
        }
        Value::String(_) | Value::Array(_) | Value::Object(_) => Some(ColumnKind::Utf8),
    }
}

fn build_arrow_array(
    rows: &[Value],
    column: &str,
    kind: ColumnKind,
) -> Result<ArrayRef, PipelineError> {
    match kind {
        ColumnKind::Boolean => {
            let mut builder = BooleanBuilder::with_capacity(rows.len());
            for row in rows {
                match row.as_object().and_then(|object| object.get(column)) {
                    Some(Value::Bool(value)) => builder.append_value(*value),
                    _ => builder.append_null(),
                }
            }
            Ok(Arc::new(builder.finish()) as ArrayRef)
        }
        ColumnKind::Int64 => {
            let mut builder = Int64Builder::with_capacity(rows.len());
            for row in rows {
                match row.as_object().and_then(|object| object.get(column)) {
                    Some(Value::Number(number)) => {
                        if let Some(value) = number.as_i64() {
                            builder.append_value(value);
                        } else if let Some(value) =
                            number.as_u64().and_then(|value| i64::try_from(value).ok())
                        {
                            builder.append_value(value);
                        } else {
                            builder.append_null();
                        }
                    }
                    _ => builder.append_null(),
                }
            }
            Ok(Arc::new(builder.finish()) as ArrayRef)
        }
        ColumnKind::Float64 => {
            let mut builder = Float64Builder::with_capacity(rows.len());
            for row in rows {
                match row.as_object().and_then(|object| object.get(column)) {
                    Some(Value::Number(number)) => {
                        if let Some(value) = number.as_f64() {
                            builder.append_value(value);
                        } else {
                            builder.append_null();
                        }
                    }
                    _ => builder.append_null(),
                }
            }
            Ok(Arc::new(builder.finish()) as ArrayRef)
        }
        ColumnKind::Utf8 => {
            let mut builder = StringBuilder::with_capacity(rows.len(), rows.len() * 16);
            for row in rows {
                match row.as_object().and_then(|object| object.get(column)) {
                    Some(Value::Null) | None => builder.append_null(),
                    Some(value) => builder.append_value(&csv_cell_value(value)),
                }
            }
            Ok(Arc::new(builder.finish()) as ArrayRef)
        }
    }
}

/// One batch as JSON rows, failures raised under the calling node's `code`.
pub(crate) fn record_batch_to_rows(batch: &RecordBatch, code: &'static str) -> Result<Vec<Value>, PipelineError> {
    batch_rows(batch).map_err(|err| PipelineError::new(code, err.message))
}

fn batch_rows(batch: &RecordBatch) -> Result<Vec<Value>, PipelineError> {
    let schema = batch.schema();
    let mut rows = Vec::with_capacity(batch.num_rows());
    for row_index in 0..batch.num_rows() {
        let mut row = Map::new();
        for (column_index, field) in schema.fields().iter().enumerate() {
            let array = batch.column(column_index);
            row.insert(
                field.name().clone(),
                arrow_value_to_json(array.as_ref(), row_index)?,
            );
        }
        rows.push(Value::Object(row));
    }
    Ok(rows)
}

fn arrow_value_to_json(array: &dyn Array, row: usize) -> Result<Value, PipelineError> {
    if array.is_null(row) {
        return Ok(Value::Null);
    }
    match array.data_type() {
        DataType::Boolean => Ok(json!(
            array
                .as_any()
                .downcast_ref::<BooleanArray>()
                .expect("boolean array")
                .value(row)
        )),
        DataType::Int8 => Ok(json!(
            array
                .as_any()
                .downcast_ref::<Int8Array>()
                .expect("int8 array")
                .value(row)
        )),
        DataType::Int16 => Ok(json!(
            array
                .as_any()
                .downcast_ref::<Int16Array>()
                .expect("int16 array")
                .value(row)
        )),
        DataType::Int32 => Ok(json!(
            array
                .as_any()
                .downcast_ref::<Int32Array>()
                .expect("int32 array")
                .value(row)
        )),
        DataType::Int64 => Ok(json!(
            array
                .as_any()
                .downcast_ref::<Int64Array>()
                .expect("int64 array")
                .value(row)
        )),
        DataType::UInt8 => Ok(json!(
            array
                .as_any()
                .downcast_ref::<UInt8Array>()
                .expect("uint8 array")
                .value(row)
        )),
        DataType::UInt16 => Ok(json!(
            array
                .as_any()
                .downcast_ref::<UInt16Array>()
                .expect("uint16 array")
                .value(row)
        )),
        DataType::UInt32 => Ok(json!(
            array
                .as_any()
                .downcast_ref::<UInt32Array>()
                .expect("uint32 array")
                .value(row)
        )),
        DataType::UInt64 => Ok(json!(
            array
                .as_any()
                .downcast_ref::<UInt64Array>()
                .expect("uint64 array")
                .value(row)
        )),
        DataType::Float32 => {
            let value = array
                .as_any()
                .downcast_ref::<Float32Array>()
                .expect("float32 array")
                .value(row);
            Ok(serde_json::Number::from_f64(value as f64)
                .map(Value::Number)
                .unwrap_or(Value::Null))
        }
        DataType::Float64 => {
            let value = array
                .as_any()
                .downcast_ref::<Float64Array>()
                .expect("float64 array")
                .value(row);
            Ok(serde_json::Number::from_f64(value)
                .map(Value::Number)
                .unwrap_or(Value::Null))
        }
        DataType::Utf8 => Ok(Value::String(
            array
                .as_any()
                .downcast_ref::<StringArray>()
                .expect("utf8 array")
                .value(row)
                .to_string(),
        )),
        DataType::LargeUtf8 => Ok(Value::String(
            array
                .as_any()
                .downcast_ref::<LargeStringArray>()
                .expect("large utf8 array")
                .value(row)
                .to_string(),
        )),
        _ => array_value_to_string(array, row)
            .map(Value::String)
            .map_err(|err| PipelineError::new(CODE, err.to_string())),
    }
}

pub(crate) fn collect_columns(rows: &[Value]) -> Vec<String> {
    let mut seen = HashSet::<String>::new();
    let mut columns = Vec::new();
    for row in rows {
        if let Value::Object(map) = row {
            for key in map.keys() {
                if seen.insert(key.clone()) {
                    columns.push(key.clone());
                }
            }
        }
    }
    columns
}

fn encode_csv(rows: &[Value], columns: &[String]) -> String {
    let mut out = String::new();
    write_csv_record(&mut out, columns.iter().map(String::as_str));
    for row in rows {
        let object = row.as_object();
        let cells = columns
            .iter()
            .map(|column| {
                object
                    .and_then(|map| map.get(column))
                    .map(csv_cell_value)
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();
        write_csv_record(&mut out, cells.iter().map(String::as_str));
    }
    out
}

fn write_csv_record<'a, I>(out: &mut String, cells: I)
where
    I: IntoIterator<Item = &'a str>,
{
    let mut first = true;
    for cell in cells {
        if !first {
            out.push(',');
        }
        first = false;
        out.push_str(&escape_csv_cell(cell));
    }
    out.push('\n');
}

fn csv_cell_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn escape_csv_cell(cell: &str) -> String {
    if cell.contains(',') || cell.contains('"') || cell.contains('\n') || cell.contains('\r') {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell.to_string()
    }
}

fn parse_csv_records(text: &str) -> Result<Vec<Vec<String>>, PipelineError> {
    let mut records = Vec::<Vec<String>>::new();
    let mut record = Vec::<String>::new();
    let mut cell = String::new();
    let mut chars = text.chars().peekable();
    let mut in_quotes = false;
    let mut saw_any = false;

    while let Some(char) = chars.next() {
        saw_any = true;
        match char {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                record.push(std::mem::take(&mut cell));
            }
            '\n' if !in_quotes => {
                record.push(std::mem::take(&mut cell));
                records.push(std::mem::take(&mut record));
            }
            '\r' if !in_quotes => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                record.push(std::mem::take(&mut cell));
                records.push(std::mem::take(&mut record));
            }
            other => cell.push(other),
        }
    }

    if in_quotes {
        return Err(PipelineError::new(
            CODE,
            "CSV has an unterminated quoted cell",
        ));
    }

    if saw_any && (!cell.is_empty() || !record.is_empty()) {
        record.push(cell);
        records.push(record);
    }

    Ok(records)
}


#[cfg(test)]
mod tests {
    use super::*;

    fn config(dsl: &str) -> Config {
        let graph = crate::platform::shell::parser::build_pipeline_graph("t", &format!("[a] trigger.manual\n[b] {dsl}\n[a] -> [b]\n"))
            .expect("graph");
        serde_json::from_value(graph.nodes[1].config.clone()).expect("config")
    }

    #[test]
    fn the_flags_are_parse_format_rows_and_limit() {
        let config = config("table.data.convert --from uploads/data.csv --parse csv --format parquet --rows --limit 5 --path out/data.parquet");
        assert_eq!(config.parse, "csv");
        assert_eq!(config.format, "parquet");
        assert!(config.rows);
        assert_eq!(config.limit, json!(5));
        let settings = config.settings().expect("settings");
        assert_eq!(settings.parse, Some(TableFormat::Csv));
        assert_eq!(settings.format, Some(TableFormat::Parquet));
        assert_eq!(settings.limit, Some(5));
        assert!(settings.answer_rows, "--rows with a destination answers the rows too");

        let flags: Vec<String> = definition().dsl_flags.into_iter().map(|f| f.flag).collect();
        for gone in ["--from-format", "--to-format", "--to-json", "--preview-rows"] {
            assert!(!flags.iter().any(|f| f == gone), "{gone} is gone: {flags:?}");
        }
    }

    #[test]
    fn a_choice_is_closed_and_rows_are_the_answer_without_a_destination() {
        let bad = (Config { parse: "xlsx".into(), ..Default::default() }).settings().err().expect("refused");
        assert_eq!(bad.code, CONFIG_CODE);
        assert!(bad.message.contains("--parse"), "{}", bad.message);
        assert!((Config { format: "tsv".into(), ..Default::default() }).settings().is_err());
        assert!((Config { limit: json!(0), ..Default::default() }).settings().is_err());
        let rows_only = (Config { from: json!([{ "a": 1 }]), ..Default::default() }).settings().expect("settings");
        assert!(rows_only.answer_rows);
        let file_only = (Config { from: json!("a.csv"), path: Some("b.parquet".into()), ..Default::default() }).settings().expect("settings");
        assert!(!file_only.answer_rows);
    }

    #[test]
    fn an_extension_that_is_no_format_asks_for_the_flag() {
        let err = normalize_format("uploads/sheet.xlsx", "--from").unwrap_err();
        assert_eq!(err.code, CONFIG_CODE);
        assert!(err.message.contains("--parse"), "{}", err.message);
        assert_eq!(normalize_format("a/b.jsonl", "--from").unwrap(), TableFormat::Ndjson);
        assert!(normalize_format("noext", "the written file").unwrap_err().message.contains("--format"));
    }

    #[test]
    fn parses_quoted_csv_rows() {
        let rows = rows_from_csv_text("name,note\nAda,\"hello, world\"\nBob,\"a \"\"quote\"\"\"\n")
            .expect("csv rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], "Ada");
        assert_eq!(rows[0]["note"], "hello, world");
        assert_eq!(rows[1]["note"], "a \"quote\"");
    }

    #[test]
    fn object_with_rows_is_table_source() {
        let rows = rows_from_json_value(json!({
            "rows": [
                { "id": 1, "title": "A" },
                { "id": 2, "title": "B" }
            ],
            "ignored": true
        }))
        .expect("json rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1]["title"], "B");
    }

    #[test]
    fn encodes_csv_with_escaped_cells() {
        let rows = vec![json!({ "name": "Ada", "note": "hello, world", "score": 42 })];
        let csv = encode_csv(
            &rows,
            &["name".to_string(), "note".to_string(), "score".to_string()],
        );
        assert_eq!(csv, "name,note,score\nAda,\"hello, world\",42\n");
    }

    #[test]
    fn parquet_roundtrip_preserves_basic_types() {
        let rows = vec![
            json!({ "id": 1, "title": "A", "score": 1.5, "active": true }),
            json!({ "id": 2, "title": "B", "score": 2.0, "active": false }),
        ];
        let columns = collect_columns(&rows);
        let parquet = encode_parquet(&rows, &columns).expect("parquet encode");
        let decoded = rows_from_parquet_bytes(parquet).expect("parquet decode");
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0]["id"], 1);
        assert_eq!(decoded[0]["title"], "A");
        assert_eq!(decoded[0]["score"], 1.5);
        assert_eq!(decoded[1]["active"], false);
    }

    #[tokio::test]
    async fn streams_csv_to_parquet_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let source = dir.path().join("input.csv");
        let target = dir.path().join("output.parquet");
        std::fs::write(&source, "id,name\n1,Ada\n2,Bob\n").expect("write csv");

        let columns = stream_csv_to_parquet(&source, &target, None)
            .await
            .expect("stream csv to parquet");

        assert_eq!(columns, vec!["id".to_string(), "name".to_string()]);
        let rows = rows_from_parquet_bytes(std::fs::read(&target).expect("read parquet"))
            .expect("decode parquet");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["id"], 1);
        assert_eq!(rows[1]["name"], "Bob");
    }
}


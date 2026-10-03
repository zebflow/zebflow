//! `geo.dataset.convert` — a spatial dataset from one format to another.
//!
//! Delegates to `geonative_convert::convert()`. Reads `.gdb`, `.shp`,
//! `.parquet`, `.geojson`; writes `.parquet` or `.geojson` (from the
//! destination's extension). `--crs` reprojects mid-stream.
//!
//! The answer is one key, `dataset`: the written file's FileRef fields with
//! the CRS of what was produced, the layer read, the feature count and every
//! file written.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::pipeline::model::NodeCapability;
use crate::pipeline::nodes::shared::limits::{whole, within};
use crate::pipeline::nodes::shared::project_store::{
    OnConflict, on_conflict_flag, open_from, open_store, store_fields, store_flag, target_key,
};
use crate::pipeline::nodes::shared::store_scratch::StoreScratch;
use crate::pipeline::nodes::shared::util::{metadata_scope, with_answer};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeExample, NodeFieldDef, NodeFieldType},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

pub const NODE_KIND: &str = "geo.dataset.convert";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

/// Reading, converting or writing failed.
pub const CODE: &str = "FW_NODE_GEO_DATASET_CONVERT";
/// A flag the author set wrong.
pub const CONFIG_CODE: &str = "FW_NODE_GEO_DATASET_CONVERT_CONFIG";

const DEFAULT_FOLDER: &str = "geo";
const DEFAULT_BATCH_SIZE: u32 = 10_000;
const MAX_BATCH_SIZE: u32 = 1_000_000;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// The source: a store key, or a FileRef through `{{ }}`.
    #[serde(default)]
    pub from: Value,
    /// Store folder for the output (default: `geo`).
    #[serde(default)]
    pub folder: String,
    /// Output name, `.parquet` or `.geojson` (default: `<source>.parquet`).
    #[serde(default)]
    pub filename: Option<String>,
    /// Exact store key; overrides `folder` and `filename`.
    #[serde(default)]
    pub path: Option<String>,
    /// The store a key is read from and the output written to; saved explicitly at registration.
    #[serde(default)]
    pub store: Option<String>,
    /// `overwrite`, `skip` or `error` (default: error).
    #[serde(default)]
    pub on_conflict: Option<String>,
    /// The layer of a multi-layer source (FileGDB).
    #[serde(default)]
    pub layer: String,
    /// The CRS of what is produced: `EPSG:NNNN` or `NNNN` (default: the source's).
    #[serde(default)]
    pub crs: String,
    /// Hilbert-sort the output by bbox centroid (Parquet only).
    #[serde(default)]
    pub hilbert: bool,
    /// Rows per Parquet row group (default 10000).
    #[serde(default)]
    pub batch_size: Value,
}

fn flag(name: &str, key: &str, description: &str, value: &str) -> DslFlag {
    DslFlag {
        flag: name.to_string(),
        config_key: key.to_string(),
        description: description.to_string(),
        kind: DslFlagKind::Scalar,
        value: value.to_string(),
        ..Default::default()
    }
}

fn field(name: &str, label: &str, field_type: NodeFieldType, help: &str) -> NodeFieldDef {
    NodeFieldDef { name: name.to_string(), label: label.to_string(), field_type, help: Some(help.to_string()), ..Default::default() }
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "Geo Convert".to_string(),
        description: "Convert a spatial dataset from one format to another. `--from` is a store key or a FileRef (read from the store it \
            names), with its sidecar files: .gdb, .shp, .parquet, .geojson. Writes .parquet or .geojson, by the destination's extension \
            (`--folder`, default `geo`; `--filename`, default `<source>.parquet`; or `--path`). `--crs EPSG:4326` reprojects mid-stream; \
            `--layer` picks one layer of a FileGDB; `--hilbert` sorts Parquet rows by bbox centroid; `--batch-size` sets the rows per row group. \
            Adds `dataset: { …FileRef…, crs, layer, feature_count, files }` — `crs` is the declared CRS of the output, `files` every file \
            written — and keeps the rest of the payload."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "dataset": {
                    "type": "object",
                    "description": "The written file's FileRef fields, with what was converted.",
                    "properties": {
                        "ref": { "type": "string" },
                        "store": { "type": "string" },
                        "crs": { "type": ["object", "null"], "description": "{ kind: epsg, code } | { kind: wkt, wkt } | { kind: projjson, projjson } | { kind: unknown }; null when the file was kept under --on-conflict skip" },
                        "layer": { "type": ["string", "null"], "description": "The --layer read; null for a single-layer source" },
                        "feature_count": { "type": ["integer", "null"] },
                        "files": { "type": "array", "description": "A FileRef for every file written" }
                    }
                }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag { required: true, ..flag("--from", "from", "The spatial file: a store key, or a FileRef through {{ }}.", "file") },
            flag("--folder", "folder", "Store folder for the output (default: geo).", "text"),
            flag("--filename", "filename", "Output name, .parquet or .geojson (default: <source>.parquet).", "text"),
            flag("--path", "path", "Exact store key for the output; overrides --folder and --filename.", "text"),
            DslFlag { value: "text".to_string(), ..store_flag() },
            DslFlag { choices: ["error", "skip", "overwrite"].iter().map(|w| w.to_string()).collect(), ..on_conflict_flag(OnConflict::Error) },
            flag("--layer", "layer", "The layer of a multi-layer source (FileGDB).", "text"),
            flag("--crs", "crs", "The CRS of what is produced, reprojected mid-stream: EPSG:NNNN or NNNN (default: the source's).", "text"),
            DslFlag { kind: DslFlagKind::Bool, ..flag("--hilbert", "hilbert", "Hilbert-sort the output by bbox centroid (Parquet only).", "") },
            flag("--batch-size", "batch_size", "Rows per Parquet row group (default: 10000).", "number"),
        ],
        fields: vec![
            field("from", "From", NodeFieldType::Text, "The spatial file: a store key, or a FileRef through {{ }}."),
            field("folder", "Folder", NodeFieldType::Text, "Store folder for the output (default: geo)."),
            field("filename", "Filename", NodeFieldType::Text, "Output name, .parquet or .geojson (default: <source>.parquet)."),
            field("path", "Path", NodeFieldType::Text, "Exact store key; overrides folder and filename."),
            field("layer", "Layer", NodeFieldType::Text, "The layer of a multi-layer FileGDB."),
            field("crs", "CRS", NodeFieldType::Text, "The CRS of the output, e.g. EPSG:4326. Empty: the source's."),
            NodeFieldDef { default_value: Some(json!(false)), ..field("hilbert", "Hilbert sort", NodeFieldType::Checkbox, "Sort Parquet rows by bbox centroid.") },
            NodeFieldDef { default_value: Some(json!(DEFAULT_BATCH_SIZE)), ..field("batch_size", "Batch size", NodeFieldType::Number, "Rows per Parquet row group.") },
        ]
        .into_iter()
        .chain(store_fields(OnConflict::Error))
        .collect(),
        layout: vec![
            LayoutItem::Col {
                col: ["from", "folder", "filename", "path", "store", "on_conflict"].iter().map(|n| LayoutItem::Field(n.to_string())).collect(),
            },
            LayoutItem::Col { col: ["layer", "crs", "hilbert", "batch_size"].iter().map(|n| LayoutItem::Field(n.to_string())).collect() },
        ],
        ai_tool: Default::default(),
        examples: vec![
            NodeExample::dsl("Shapefile to GeoParquet in WGS84", "geo.dataset.convert --from uploads/suburbs.shp --folder datasets --filename suburbs.parquet --crs EPSG:4326 --hilbert")
                .output(json!({ "dataset": {
                    "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "datasets/suburbs.parquet", "filename": "suburbs.parquet",
                    "mime": "application/vnd.apache.parquet", "kind": "parquet", "size": 918233, "sha256": "sha256:…", "lifecycle": "durable",
                    "origin": NODE_KIND, "trust": "generated",
                    "crs": { "kind": "epsg", "code": 4326 }, "layer": null, "feature_count": 312, "files": ["…a FileRef for every file written"]
                } }))
                .note("Then `mapserver.layer.publish --name suburbs --route suburbs --from \"{{ input.dataset }}\"`."),
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
        let (owner, project, ..) = metadata_scope(&input.metadata)?;
        let crs = parse_crs(&self.config.crs)?;
        let batch_size = batch_size(&self.config.batch_size)?;
        let layer = Some(self.config.layer.trim()).filter(|layer| !layer.is_empty()).map(str::to_string);

        // GDAL speaks paths and the project's files live in their stores:
        // the input (and its sidecar files) is pulled into a scratch folder
        // from the store that holds it, converted there, and every file
        // written goes into this node's store.
        let (source_store, input_rel) =
            open_from(&self.platform, owner, project, &self.config.from, self.config.store.as_deref(), "--from", CONFIG_CODE)?;
        let stem = input_rel.rsplit('/').next().unwrap_or(&input_rel).split('.').next().unwrap_or("output").to_string();
        let folder = if self.config.folder.trim().is_empty() { DEFAULT_FOLDER } else { self.config.folder.trim() };
        let filename = self
            .config
            .filename
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("{stem}.parquet"));
        let output_rel = target_key(self.config.path.as_deref(), folder, &filename, CONFIG_CODE)?;
        let store = open_store(&self.platform, owner, project, self.config.store.as_deref())?;
        let on_conflict = OnConflict::parse(self.config.on_conflict.as_deref(), OnConflict::Error, CONFIG_CODE)?;
        if !on_conflict.allows(&store.fs, &output_rel, CODE)? {
            // Skipped: the answer is the file already there, as it is.
            let file = store.stored_ref(&output_rel, NODE_KIND, "generated", CODE)?;
            let dataset = dataset(file, Value::Null, layer.as_deref(), Value::Null, Vec::new());
            return Ok(answer(&input.payload, dataset, format!("output={output_rel} skipped=true")));
        }

        let scratch = StoreScratch::new(CODE)?;
        let input_abs = scratch
            .pull_with_siblings(&source_store.fs, &input_rel)
            .map_err(|_| PipelineError::new(CODE, format!("input file not found: {input_rel}")))?;
        let output_dir_local = scratch.path().join(".zf-out");
        let output_leaf = output_rel.rsplit('/').next().unwrap_or(&output_rel).to_string();
        let output_parent_rel = output_rel.rsplit_once('/').map(|(parent, _)| parent.to_string()).unwrap_or_default();
        std::fs::create_dir_all(&output_dir_local)
            .map_err(|err| PipelineError::new(CODE, format!("creating output directory: {err}")))?;
        let output_abs = output_dir_local.join(&output_leaf);

        let hilbert = self.config.hilbert;
        let (input_task, output_task, layer_task, crs_task) = (input_abs.clone(), output_abs.clone(), layer.clone(), crs.clone());
        let (stats, produced_crs) = tokio::task::spawn_blocking(move || {
            let opts = geonative_convert::ConvertOptions {
                layer: layer_task,
                sink: geonative_convert::SinkOptions {
                    batch_size: batch_size as usize,
                    add_bbox_columns: true,
                    hilbert_sort: hilbert,
                    ..geonative_convert::SinkOptions::default()
                },
                to_crs: crs_task.clone(),
                progress: None,
            };
            let stats = geonative_convert::convert(&input_task, &output_task, opts)?;
            Ok::<_, geonative_convert::ConvertError>((stats, output_crs(&output_task, crs_task.as_ref())))
        })
        .await
        .map_err(|err| PipelineError::new(CODE, format!("convert task panicked: {err}")))?
        .map_err(|err| PipelineError::new(CODE, err.to_string()))?;

        // Every file the converter wrote obeys --on-conflict, sidecars as
        // well as the main output: one already in the store is refused under
        // `error` and kept (not pushed) under `skip`.
        if let Ok(entries) = std::fs::read_dir(&output_dir_local) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                let rel = if output_parent_rel.is_empty() { name.clone() } else { format!("{output_parent_rel}/{name}") };
                if rel != output_rel && !on_conflict.allows(&store.fs, &rel, CODE)? {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
        let files = scratch.push_tree_refs(&store, &output_dir_local, &output_parent_rel, NODE_KIND, "generated")?;
        let file = files.iter().find(|file| file.get("ref").and_then(Value::as_str) == Some(output_rel.as_str())).cloned().unwrap_or(Value::Null);
        let features = stats.features;
        let dataset = dataset(file, produced_crs, layer.as_deref(), json!(features), files);
        Ok(answer(&input.payload, dataset, format!("input={input_rel} output={output_rel} features={features}")))
    }
}

/// The answer's object: the file's FileRef fields, then what was converted.
fn dataset(file: Value, crs: Value, layer: Option<&str>, feature_count: Value, files: Vec<Value>) -> Map<String, Value> {
    let mut dataset = match file {
        Value::Object(file) => file,
        _ => Map::new(),
    };
    dataset.insert("crs".to_string(), crs);
    dataset.insert("layer".to_string(), layer.map(Value::from).unwrap_or(Value::Null));
    dataset.insert("feature_count".to_string(), feature_count);
    dataset.insert("files".to_string(), Value::Array(files));
    dataset
}

/// `dataset` added to the payload, the rest kept.
fn answer(payload: &Value, dataset: Map<String, Value>, trace: String) -> NodeExecutionOutput {
    NodeExecutionOutput {
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        payload: with_answer(payload, json!({ "dataset": dataset })),
        trace: vec![format!("node_kind={NODE_KIND} {trace}")],
    }
}

/// The CRS the output declares, as `geo.dataset.inspect` reports one: the
/// requested `--crs`, else what the written file says.
fn output_crs(output: &std::path::Path, requested: Option<&geonative_core::Crs>) -> Value {
    let declared = match requested {
        Some(crs) => serde_json::to_value(geonative_convert::CrsInspection::from(crs)).ok(),
        None => geonative_convert::inspect(output)
            .ok()
            .and_then(|report| report.layers.into_iter().next())
            .and_then(|layer| serde_json::to_value(layer.crs).ok()),
    };
    declared.unwrap_or_else(|| json!({ "kind": "unknown" }))
}

/// `--crs`: `EPSG:NNNN` or `NNNN`; empty keeps the source's.
fn parse_crs(value: &str) -> Result<Option<geonative_core::Crs>, PipelineError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let digits = trimmed.strip_prefix("EPSG:").or_else(|| trimmed.strip_prefix("epsg:")).unwrap_or(trimmed);
    let code: u32 = digits
        .parse()
        .map_err(|_| PipelineError::new(CONFIG_CODE, format!("--crs '{trimmed}' is not EPSG:NNNN or NNNN")))?;
    Ok(Some(geonative_core::Crs::Epsg(code)))
}

/// `--batch-size`: rows per row group, `1..=1000000`, 10000 when unset.
fn batch_size(value: &Value) -> Result<u32, PipelineError> {
    match whole(value, "--batch-size", CONFIG_CODE)? {
        None => Ok(DEFAULT_BATCH_SIZE),
        Some(n) => within(n, 1, MAX_BATCH_SIZE, "--batch-size", CONFIG_CODE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crs_and_batch_size_are_checked_before_anything_runs() {
        assert!(matches!(parse_crs("EPSG:4326").unwrap(), Some(geonative_core::Crs::Epsg(4326))));
        assert!(matches!(parse_crs("7855").unwrap(), Some(geonative_core::Crs::Epsg(7855))));
        assert!(parse_crs("").unwrap().is_none());
        assert_eq!(parse_crs("WGS84").unwrap_err().code, CONFIG_CODE);
        assert_eq!(batch_size(&Value::Null).unwrap(), DEFAULT_BATCH_SIZE);
        assert_eq!(batch_size(&json!("500")).unwrap(), 500);
        assert!(batch_size(&json!(0)).is_err());
        assert!(batch_size(&json!(2_000_000)).is_err());
    }

    #[test]
    fn the_flags_are_0_11() {
        let config: Config = serde_json::from_value(
            crate::platform::shell::parser::build_pipeline_graph(
                "t",
                "[a] trigger.manual\n[b] geo.dataset.convert --from uploads/a.shp --crs EPSG:4326 --hilbert --batch-size 500\n[a] -> [b]\n",
            )
            .expect("graph")
            .nodes[1]
                .config
                .clone(),
        )
        .expect("config");
        assert_eq!(config.crs, "EPSG:4326");
        assert!(config.hilbert);
        assert_eq!(config.batch_size, json!(500));
        assert!(!definition().dsl_flags.iter().any(|f| f.flag == "--to-crs"));
    }

    #[test]
    fn the_answer_is_the_file_with_what_was_converted() {
        let file = json!({ "__zf_type": "file_ref", "ref": "geo/a.parquet", "store": "local" });
        let got = dataset(file, json!({ "kind": "epsg", "code": 4326 }), None, json!(3), vec![]);
        assert_eq!(got["ref"], "geo/a.parquet");
        assert_eq!(got["crs"]["code"], 4326);
        assert_eq!(got["layer"], Value::Null);
        assert_eq!(got["feature_count"], 3);
    }

    #[tokio::test]
    async fn a_geojson_converts_to_parquet_and_inspects_back() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut config = crate::platform::model::PlatformConfig::default();
        config.data_root = tmp.path().join("platform");
        let platform = Arc::new(PlatformService::from_config(config).expect("platform"));
        let store = open_store(&platform, "demo", "demo", None).expect("store");
        store
            .fs
            .put(
                "uploads/points.geojson",
                br#"{"type":"FeatureCollection","features":[
                  {"type":"Feature","properties":{"name":"a"},"geometry":{"type":"Point","coordinates":[144.96,-37.81]}},
                  {"type":"Feature","properties":{"name":"b"},"geometry":{"type":"Point","coordinates":[145.0,-37.9]}}]}"#,
            )
            .expect("put");
        let input = |payload: Value| NodeExecutionInput {
            node_id: "n0".to_string(),
            input_pin: "in".to_string(),
            payload,
            metadata: json!({ "owner": "demo", "project": "demo", "pipeline": "test", "request_id": "r1" }),
            bus: None,
        };

        let convert = Node::new(
            Config { from: json!("uploads/points.geojson"), crs: "EPSG:4326".into(), ..Default::default() },
            platform.clone(),
        )
        .unwrap();
        let out = convert.execute_async(input(json!({ "keep": 1 }))).await.expect("convert").payload;
        assert_eq!(out["keep"], 1, "the payload is kept");
        let dataset = &out["dataset"];
        assert_eq!(dataset["__zf_type"], "file_ref");
        assert_eq!(dataset["ref"], "geo/points.parquet");
        assert_eq!(dataset["feature_count"], 2);
        assert_eq!(dataset["crs"], json!({ "kind": "epsg", "code": 4326 }));
        assert!(store.fs.head("geo/points.parquet").is_ok());

        let again = convert.execute_async(input(json!({}))).await.unwrap_err();
        assert_eq!(again.code, CODE, "a named file that exists is an error by default");

        let inspect = super::super::inspect::Node::new(
            super::super::inspect::Config { from: dataset.clone(), ..Default::default() },
            platform.clone(),
        )
        .unwrap();
        let report = inspect.execute_async(input(json!({}))).await.expect("inspect").payload;
        assert_eq!(report["dataset"]["format"], "geoparquet");
        assert_eq!(report["dataset"]["source"], "geo/points.parquet");
        assert_eq!(report["dataset"]["layers"][0]["crs"]["code"], 4326);
    }
}

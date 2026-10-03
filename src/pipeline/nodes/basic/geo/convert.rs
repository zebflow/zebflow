//! geo.dataset.convert — convert a spatial dataset between formats.
//!
//! Delegates to `geonative_convert::convert()`. Reads `.gdb`, `.shp`,
//! `.parquet`, `.geojson`; writes `.parquet` or `.geojson`. Optionally
//! reprojects mid-stream via `--to-crs`.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::pipeline::nodes::shared::project_store::{OnConflict, on_conflict_flag, open_source, open_store, store_fields, store_flag, target_key};
use crate::pipeline::nodes::shared::store_scratch::StoreScratch;
use crate::pipeline::nodes::shared::util::metadata_scope;
use crate::pipeline::model::NodeCapability;
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

pub const NODE_KIND: &str = "geo.dataset.convert";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

fn default_batch_size() -> usize {
    10_000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// The source: a store key, or a FileRef through `{{ }}`.
    #[serde(default)]
    pub from: serde_json::Value,
    /// Store folder for the output (default: `geo`).
    #[serde(default)]
    pub folder: String,
    /// Output name, `.parquet` or `.geojson` (default: `<source>.parquet`).
    #[serde(default)]
    pub filename: Option<String>,
    /// Exact store key; overrides `folder` and `filename`.
    #[serde(default)]
    pub path: Option<String>,
    /// The store to write to; saved explicitly at registration.
    #[serde(default)]
    pub store: Option<String>,
    /// `overwrite`, `skip` or `error` (default: error).
    #[serde(default)]
    pub on_conflict: Option<String>,
    #[serde(default)]
    pub layer: String,
    #[serde(default)]
    pub to_crs: String,
    #[serde(default)]
    pub hilbert: bool,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            from: serde_json::Value::Null,
            folder: String::new(),
            filename: None,
            path: None,
            store: None,
            on_conflict: None,
            layer: String::new(),
            to_crs: String::new(),
            hilbert: false,
            batch_size: default_batch_size(),
        }
    }
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "Geo Convert".to_string(),
        description: "Convert a spatial dataset from one format to another. \
            Input: .gdb, .shp, .parquet, .geojson. Output: .parquet, .geojson. \
            Optionally reprojects mid-stream via --to-crs."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "description": "Payload optionally contains a project-relative path at the configured input_expr key."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "converted": {
                    "type": "object",
                    "properties": {
                        "input": { "type": "string" },
                        "output": { "type": "string" },
                        "features": { "type": "integer" },
                        "output_bytes": { "type": "integer" },
                        "elapsed_secs": { "type": "number" }
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
            DslFlag {
                flag: "--from".to_string(),
                config_key: "from".to_string(),
                description: "The input spatial file: a store key, or a FileRef through {{ }} (e.g. \"{{ input.file }}\")".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
                ..Default::default()
            },
            DslFlag {
                flag: "--folder".to_string(),
                config_key: "folder".to_string(),
                description: "Store folder for the output (default: geo)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
            DslFlag {
                flag: "--filename".to_string(),
                config_key: "filename".to_string(),
                description: "Output name, .parquet or .geojson (default: <source>.parquet)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
            DslFlag {
                flag: "--path".to_string(),
                config_key: "path".to_string(),
                description: "Exact store key for the output; overrides --folder and --filename".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
            store_flag(),
            on_conflict_flag(OnConflict::Error),
            DslFlag {
                flag: "--layer".to_string(),
                config_key: "layer".to_string(),
                description: "Layer name for multi-layer sources (e.g. FileGDB)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
            DslFlag {
                flag: "--to-crs".to_string(),
                config_key: "to_crs".to_string(),
                description: "Target CRS for mid-stream reprojection (e.g. EPSG:4326 or 4326)"
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
            DslFlag {
                flag: "--hilbert".to_string(),
                config_key: "hilbert".to_string(),
                description: "Hilbert-sort output by bbox centroid (parquet only)".to_string(),
                kind: DslFlagKind::Bool,
                required: false,
                ..Default::default()
            },
            DslFlag {
                flag: "--batch-size".to_string(),
                config_key: "batch_size".to_string(),
                description: "Rows per parquet row group (default: 10000)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "from".to_string(),
                label: "From".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("The input spatial file: a store key, or a FileRef through {{ }}.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "folder".to_string(),
                label: "Folder".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Store folder for the output (default: geo).".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "filename".to_string(),
                label: "Filename".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Output name, .parquet or .geojson (default: <source>.parquet).".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "path".to_string(),
                label: "Path".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Exact store key; overrides folder and filename.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "layer".to_string(),
                label: "Layer".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Layer name for multi-layer FileGDB sources.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "to_crs".to_string(),
                label: "Target CRS".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Target CRS (e.g. EPSG:4326, 3857, 7855).".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "hilbert".to_string(),
                label: "Hilbert Sort".to_string(),
                field_type: NodeFieldType::Checkbox,
                help: Some("Hilbert-sort output by bbox centroid (parquet only).".to_string()),
                default_value: Some(json!(false)),
                ..Default::default()
            },
            NodeFieldDef {
                name: "batch_size".to_string(),
                label: "Batch Size".to_string(),
                field_type: NodeFieldType::Number,
                help: Some("Rows per parquet row group.".to_string()),
                default_value: Some(json!(10000)),
                ..Default::default()
            },
        ].into_iter().chain(store_fields(OnConflict::Error)).collect(),
        layout: vec![
            LayoutItem::Col {
                col: vec![
                    LayoutItem::Field("from".to_string()),
                    LayoutItem::Field("folder".to_string()),
                    LayoutItem::Field("filename".to_string()),
                    LayoutItem::Field("path".to_string()),
                    LayoutItem::Field("store".to_string()),
                    LayoutItem::Field("on_conflict".to_string()),
                ],
            },
            LayoutItem::Col {
                col: vec![
                    LayoutItem::Field("layer".to_string()),
                    LayoutItem::Field("to_crs".to_string()),
                    LayoutItem::Field("hilbert".to_string()),
                    LayoutItem::Field("batch_size".to_string()),
                ],
            },
        ],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Shapefile to GeoParquet in WGS84", "geo.dataset.convert --from uploads/suburbs.zip --folder datasets --filename suburbs.parquet --to-crs EPSG:4326 --hilbert")
                .output(serde_json::json!({ "converted": { "source": "uploads/suburbs.zip", "store": "local", "file": { "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "datasets/suburbs.parquet", "filename": "suburbs.parquet", "mime": "application/vnd.apache.parquet", "kind": "parquet", "size": 918233, "sha256": "sha256:…", "lifecycle": "durable", "origin": "geo.dataset.convert", "trust": "generated" }, "files": ["…every file written, sidecars too"], "features": 312, "elapsed_secs": 0.8 } }))
                .note("Then `ms.layer.publish --name suburbs --route suburbs --from datasets/suburbs.parquet --source-kind geoparquet`."),
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

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let (owner, project, ..) = metadata_scope(&input.metadata)?;

        // GDAL speaks paths and the project's files live in their stores:
        // the input (and its sidecar files) is pulled into a scratch folder
        // from the store that holds it, converted there, and every file
        // written goes into this node's store.
        let (source_store, input_rel) = open_source(&self.platform, owner, project, &self.config.from, self.config.store.as_deref())?
            .ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_GEO_CONVERT",
                    "no input configured — set --from to a store key or a FileRef",
                )
            })?;
        let input_rel = sanitize_rel_path(&input_rel);
        let stem = input_rel
            .rsplit('/')
            .next()
            .unwrap_or(&input_rel)
            .split('.')
            .next()
            .unwrap_or("output")
            .to_string();
        let folder = if self.config.folder.trim().is_empty() { "geo" } else { self.config.folder.trim() };
        let filename = self
            .config
            .filename
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("{stem}.parquet"));
        let output_rel = target_key(self.config.path.as_deref(), folder, &filename, "FW_NODE_GEO_CONVERT")?;
        let store = open_store(&self.platform, owner, project, self.config.store.as_deref())?;
        let on_conflict = OnConflict::parse(self.config.on_conflict.as_deref(), OnConflict::Error, "FW_NODE_GEO_CONVERT")?;
        if !on_conflict.allows(&store.fs, &output_rel, "FW_NODE_GEO_CONVERT")? {
            let file = store.stored_ref(&output_rel, "geo.dataset.convert", "generated", "FW_NODE_GEO_CONVERT")?;
            return Ok(answer(&input.payload, json!({
                "source": input_rel, "store": store.id, "file": file, "files": [], "skipped": true
            }), format!("node_kind={NODE_KIND} input={input_rel} output={output_rel} skipped=true")));
        }

        let scratch = StoreScratch::new("FW_NODE_GEO_CONVERT")?;
        let input_abs = scratch.pull_with_siblings(&source_store.fs, &input_rel).map_err(|_| {
            PipelineError::new(
                "FW_NODE_GEO_CONVERT",
                format!("input file not found: {input_rel}"),
            )
        })?;

        let output_dir_local = scratch.path().join(".zf-out");
        let output_leaf = output_rel.rsplit('/').next().unwrap_or(&output_rel).to_string();
        let output_parent_rel = output_rel
            .rsplit_once('/')
            .map(|(parent, _)| parent.to_string())
            .unwrap_or_default();
        std::fs::create_dir_all(&output_dir_local).map_err(|err| {
            PipelineError::new(
                "FW_NODE_GEO_CONVERT",
                format!("creating output directory: {err}"),
            )
        })?;
        let output_abs = output_dir_local.join(&output_leaf);

        let to_crs = parse_optional_crs(&self.config.to_crs)?;
        let layer = if self.config.layer.trim().is_empty() {
            None
        } else {
            Some(self.config.layer.trim().to_string())
        };
        let batch_size = if self.config.batch_size == 0 {
            default_batch_size()
        } else {
            self.config.batch_size
        };
        let hilbert = self.config.hilbert;

        let input_abs_clone = input_abs.clone();
        let output_abs_clone = output_abs.clone();
        let stats = tokio::task::spawn_blocking(move || {
            let opts = geonative_convert::ConvertOptions {
                layer,
                sink: geonative_convert::SinkOptions {
                    batch_size,
                    add_bbox_columns: true,
                    hilbert_sort: hilbert,
                    ..geonative_convert::SinkOptions::default()
                },
                to_crs,
                progress: None,
            };
            geonative_convert::convert(&input_abs_clone, &output_abs_clone, opts)
        })
        .await
        .map_err(|err| {
            PipelineError::new(
                "FW_NODE_GEO_CONVERT",
                format!("convert task panicked: {err}"),
            )
        })?
        .map_err(|err| PipelineError::new("FW_NODE_GEO_CONVERT", err.to_string()))?;
        // Every file the converter wrote obeys --on-conflict, sidecars as
        // well as the main output: one already in the store is refused under
        // `error` and kept (not pushed) under `skip`.
        if let Ok(entries) = std::fs::read_dir(&output_dir_local) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                let rel = if output_parent_rel.is_empty() { name.clone() } else { format!("{output_parent_rel}/{name}") };
                if rel != output_rel && !on_conflict.allows(&store.fs, &rel, "FW_NODE_GEO_CONVERT")? {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
        let files = scratch.push_tree_refs(&store, &output_dir_local, &output_parent_rel, "geo.dataset.convert", "generated")?;
        let file = files
            .iter()
            .find(|file| file.get("ref").and_then(|value| value.as_str()) == Some(output_rel.as_str()))
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        Ok(answer(
            &input.payload,
            json!({
                "source": input_rel,
                "store": store.id,
                "file": file,
                "files": files,
                "features": stats.features,
                "elapsed_secs": stats.elapsed_secs,
            }),
            format!(
                "node_kind={NODE_KIND} input={input_rel} output={output_rel} features={}",
                stats.features
            ),
        ))
    }
}

/// `converted` added to the payload, the rest kept.
fn answer(input: &serde_json::Value, converted: serde_json::Value, trace: String) -> NodeExecutionOutput {
    NodeExecutionOutput {
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        payload: crate::pipeline::nodes::shared::util::with_answer(input, serde_json::json!({ "converted": converted })),
        trace: vec![trace],
    }
}

fn parse_optional_crs(s: &str) -> Result<Option<geonative_core::Crs>, PipelineError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let digits = trimmed
        .strip_prefix("EPSG:")
        .or_else(|| trimmed.strip_prefix("epsg:"))
        .unwrap_or(trimmed);
    let code: u32 = digits.parse().map_err(|e| {
        PipelineError::new(
            "FW_NODE_GEO_CONVERT",
            format!("--to-crs expects EPSG:NNNN or NNNN, got '{s}': {e}"),
        )
    })?;
    Ok(Some(geonative_core::Crs::Epsg(code)))
}

fn sanitize_rel_path(path: &str) -> String {
    path.split('/')
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .collect::<Vec<_>>()
        .join("/")
}

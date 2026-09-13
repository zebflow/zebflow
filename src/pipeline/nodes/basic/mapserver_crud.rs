//! Mapserver CRUD nodes — dynamic layer management.
//!
//! Manages the layer registry (`files_dir/mapserver/{instance}.layers.json`).
//! Layers published via these nodes are immediately queryable on `/ms/{owner}/{project}/{path}`.
//!
//! | Node              | Purpose                               |
//! |-------------------|---------------------------------------|
//! | `n.ms.publish`    | Upsert a layer in the registry        |
//! | `n.ms.unpublish`  | Remove a layer from the registry      |
//! | `n.ms.get`        | Get layer metadata                    |
//! | `n.ms.list`       | List all published layers             |

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::file_ref::zebfs_rel_path_or_string;
use super::util::metadata_scope;
use crate::contracts::kinds::MapserverLayerRecord as LayerRecord;
use crate::mapserver::publish::registry;
use crate::mapserver::publish::registry::DEFAULT_INSTANCE;
use crate::pipeline::model::NodeCapability;
use crate::pipeline::{
    NodeDefinition, NodeFieldDataSource, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType, SelectOptionDef},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

pub const PUBLISH_KIND: &str = "n.ms.publish";
pub const UNPUBLISH_KIND: &str = "n.ms.unpublish";
pub const GET_KIND: &str = "n.ms.get";
pub const LIST_KIND: &str = "n.ms.list";

const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

// ── Layer record ────────────────────────────────────────────────────────────
//
// There is one definition, the contract's. A hand-mirrored copy lived here and
// had already drifted, which is how two writers ended up disagreeing about the
// same file.

fn layers_path(
    platform: &PlatformService,
    owner: &str,
    project: &str,
) -> Result<std::path::PathBuf, PipelineError> {
    let layout = platform
        .file
        .ensure_project_layout(owner, project)
        .map_err(|e| PipelineError::new("FW_NODE_MS", e.to_string()))?;
    Ok(registry::layers_manifest_path(
        &layout.files_dir,
        DEFAULT_INSTANCE,
    ))
}

fn read_layers(
    platform: &PlatformService,
    owner: &str,
    project: &str,
) -> Result<Vec<LayerRecord>, PipelineError> {
    let path = layers_path(platform, owner, project)?;
    registry::read_layers(&path).map_err(|e| PipelineError::new("FW_NODE_MS_PARSE", e.to_string()))
}

fn write_layers(
    platform: &PlatformService,
    owner: &str,
    project: &str,
    items: &[LayerRecord],
) -> Result<(), PipelineError> {
    let path = layers_path(platform, owner, project)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| PipelineError::new("FW_NODE_MS_WRITE", e.to_string()))?;
    }
    registry::write_layers(&path, DEFAULT_INSTANCE, items)
        .map_err(|e| PipelineError::new("FW_NODE_MS_WRITE", e.to_string()))
}

fn layer_to_json(owner: &str, project: &str, record: &LayerRecord) -> Value {
    json!({
        "layer_id": record.layer_id,
        "path": record.path,
        "url": format!("/ms/{owner}/{project}/{}", record.path),
        "source_path": record.source_path,
        "source_kind": if record.source_kind.is_empty() {
            if record.artifact_manifest_path.is_some() { "geojson_artifact" } else { "geojson_file" }
        } else {
            &record.source_kind
        },
        "mode": record.mode,
        "min_zoom": record.min_zoom,
        "max_zoom": record.max_zoom,
        "bbox_required": record.bbox_required,
        "max_features": record.max_features,
        "allowed_properties": record.allowed_properties,
        "feature_count": record.feature_count,
        "chunk_count": record.chunk_count,
        "style": record.style,
        "filter": record.filter,
        "function_slug": record.function_slug,
        "cache_ttl_secs": record.cache_ttl_secs,
    })
}

// ── Operation enum ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub enum Operation {
    Publish,
    Unpublish,
    Get,
    List,
}

impl Operation {
    fn kind(self) -> &'static str {
        match self {
            Self::Publish => PUBLISH_KIND,
            Self::Unpublish => UNPUBLISH_KIND,
            Self::Get => GET_KIND,
            Self::List => LIST_KIND,
        }
    }
}

// ── Config ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub source_path: String,
    #[serde(default)]
    pub source_kind: String,
    #[serde(default)]
    pub bbox_required: Option<bool>,
    #[serde(default)]
    pub no_bbox_required: Option<bool>,
    #[serde(default)]
    pub max_features: Option<usize>,
    #[serde(default)]
    pub allowed_properties: Option<String>,
    #[serde(default)]
    pub min_zoom: Option<u8>,
    #[serde(default)]
    pub max_zoom: Option<u8>,
    #[serde(default)]
    pub build_artifact: bool,
    // Style fields for tile rendering
    #[serde(default)]
    pub fill: Option<String>,
    #[serde(default)]
    pub stroke: Option<String>,
    #[serde(default)]
    pub stroke_width: Option<String>,
    #[serde(default)]
    pub point_radius: Option<String>,
    #[serde(default)]
    pub point_color: Option<String>,
    /// Style DSL expression (e.g. "cb(population,5,YlOrRd)").
    /// When set, overrides individual fill/stroke/etc flags.
    #[serde(default)]
    pub style_dsl: Option<String>,
    /// Default filter expression (e.g. "category:residential;value>100").
    #[serde(default)]
    pub filter: Option<String>,
    // Spatial optimization
    #[serde(default)]
    pub optimize: bool,
    /// Skip auto-conversion to optimized GeoParquet. Keeps original format.
    #[serde(default)]
    pub no_optimize: bool,
    /// Function pipeline slug for GeoJsonFunction source kind.
    #[serde(default)]
    pub function: Option<String>,
    /// Feature cache TTL in seconds for GeoJsonFunction layers.
    #[serde(default)]
    pub cache_ttl: Option<u64>,
}

// ── Node definitions ─────────────────────────────────────────────────────────

fn scalar_flag(flag: &str, config_key: &str, description: &str) -> DslFlag {
    DslFlag {
        flag: flag.to_string(),
        config_key: config_key.to_string(),
        description: description.to_string(),
        kind: DslFlagKind::Scalar,
        required: false,
    }
}

fn bool_flag(flag: &str, config_key: &str, description: &str) -> DslFlag {
    DslFlag {
        flag: flag.to_string(),
        config_key: config_key.to_string(),
        description: description.to_string(),
        kind: DslFlagKind::Bool,
        required: false,
    }
}

fn text_field(name: &str, label: &str, help: &str) -> NodeFieldDef {
    NodeFieldDef {
        name: name.to_string(),
        label: label.to_string(),
        field_type: NodeFieldType::Text,
        help: Some(help.to_string()),
        ..Default::default()
    }
}

pub fn publish_definition() -> NodeDefinition {
    NodeDefinition {
        kind: PUBLISH_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "MS Publish".to_string(),
        description: "Publish or update a map layer in the project layer registry. \
            The layer becomes immediately queryable on `/ms/{owner}/{project}/{path}`. \
            Supports geojson_file, geojson_artifact, geoparquet, and geojson_function source kinds. \
            Use --function for dynamic layers backed by a function pipeline."
            .to_string(),
        input_schema: json!({"type": "object"}),
        output_schema: json!({
            "type": "object",
            "properties": {
                "ms": {
                    "type": "object",
                    "properties": {
                        "operation": { "type": "string" },
                        "layer": { "type": "object" }
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
            scalar_flag("--name", "name", "Unique layer identifier (required)."),
            scalar_flag(
                "--path",
                "path",
                "URL path under /ms/{owner}/{project}/ (required).",
            ),
            scalar_flag(
                "--source-path",
                "source_path",
                "ZebFS path to source file (required). Can also come from input payload.source_path.",
            ),
            DslFlag {
                flag: "--source-kind".to_string(),
                config_key: "source_kind".to_string(),
                description: "Source type: geojson_file (default), geojson_artifact, geoparquet."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            bool_flag(
                "--bbox-required",
                "bbox_required",
                "Enforce bbox parameter in queries. Default: true.",
            ),
            bool_flag(
                "--no-bbox-required",
                "no_bbox_required",
                "Disable bbox requirement (allow queries without bbox).",
            ),
            scalar_flag(
                "--max-features",
                "max_features",
                "Hard feature cap per query. Default: 1000.",
            ),
            scalar_flag(
                "--allowed-properties",
                "allowed_properties",
                "Comma-separated list of source properties the public may see. Empty means geometry only.",
            ),
            scalar_flag("--min-zoom", "min_zoom", "Minimum zoom visibility."),
            scalar_flag("--max-zoom", "max_zoom", "Maximum zoom visibility."),
            bool_flag(
                "--build-artifact",
                "build_artifact",
                "For geojson: auto-build chunked artifact for large files.",
            ),
            scalar_flag("--fill", "fill", "Polygon fill CSS color (default: rgba(65,105,225,128))."),
            scalar_flag("--stroke", "stroke", "Stroke CSS color (default: #1E3CA0DC)."),
            scalar_flag("--stroke-width", "stroke_width", "Stroke width in pixels (default: 1.0)."),
            scalar_flag("--point-radius", "point_radius", "Point radius in pixels (default: 4.0)."),
            scalar_flag("--point-color", "point_color", "Point fill CSS color (default: #DC3232C8)."),
            scalar_flag("--style", "style_dsl", "Style DSL expression (e.g. 'cb(population,5,YlOrRd)'). Overrides individual fill/stroke flags."),
            scalar_flag("--filter", "filter", "Default filter expression (e.g. 'category:residential;value>100')"),
            bool_flag(
                "--optimize",
                "optimize",
                "Deprecated: optimization is now the default. Use --no-optimize to disable.",
            ),
            bool_flag(
                "--no-optimize",
                "no_optimize",
                "Skip auto-conversion to optimized GeoParquet; keep the original source format.",
            ),
            scalar_flag(
                "--function",
                "function",
                "Function pipeline slug for dynamic GeoJSON source. Mutually exclusive with --source-path.",
            ),
            scalar_flag(
                "--cache-ttl",
                "cache_ttl",
                "Feature cache TTL in seconds for function layers. Default: 60.",
            ),
        ],
        fields: vec![
            text_field("name", "Layer ID", "Unique layer identifier."),
            text_field("path", "URL Path", "Path under /ms/{owner}/{project}/."),
            text_field("source_path", "Source Path", "ZebFS path to source file."),
            NodeFieldDef {
                name: "source_kind".to_string(),
                label: "Source Kind".to_string(),
                field_type: NodeFieldType::Select,
                default_value: Some(json!("geojson_file")),
                options: vec![
                    SelectOptionDef {
                        value: "geojson_file".to_string(),
                        label: "GeoJSON File".to_string(),
                    },
                    SelectOptionDef {
                        value: "geojson_artifact".to_string(),
                        label: "GeoJSON Artifact".to_string(),
                    },
                    SelectOptionDef {
                        value: "geoparquet".to_string(),
                        label: "GeoParquet".to_string(),
                    },
                    SelectOptionDef {
                        value: "geojson_function".to_string(),
                        label: "GeoJSON Function".to_string(),
                    },
                ],
                help: Some("Type of geospatial source. Use 'GeoJSON Function' for dynamic data from a function pipeline.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "bbox_required".to_string(),
                label: "Require BBox".to_string(),
                field_type: NodeFieldType::Checkbox,
                default_value: Some(json!(true)),
                help: Some("Enforce bbox parameter in queries.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "max_features".to_string(),
                label: "Max Features".to_string(),
                field_type: NodeFieldType::Number,
                default_value: Some(json!(1000)),
                help: Some("Hard feature cap per query.".to_string()),
                ..Default::default()
            },
            text_field(
                "allowed_properties",
                "Allowed Properties",
                "Comma-separated list of source properties the public may see. Empty means geometry only — no properties are served.",
            ),
            NodeFieldDef {
                name: "min_zoom".to_string(),
                label: "Min Zoom".to_string(),
                field_type: NodeFieldType::Number,
                help: Some("Minimum zoom level for visibility.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "max_zoom".to_string(),
                label: "Max Zoom".to_string(),
                field_type: NodeFieldType::Number,
                help: Some("Maximum zoom level for visibility.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "build_artifact".to_string(),
                label: "Build Artifact".to_string(),
                field_type: NodeFieldType::Checkbox,
                help: Some(
                    "Deprecated: use default optimize behavior instead.".to_string(),
                ),
                ..Default::default()
            },
            NodeFieldDef {
                name: "no_optimize".to_string(),
                label: "Skip Optimization".to_string(),
                field_type: NodeFieldType::Checkbox,
                default_value: Some(json!(false)),
                help: Some(
                    "Skip GeoParquet conversion. Keep the original source format as-is.".to_string(),
                ),
                ..Default::default()
            },
            NodeFieldDef {
                name: "function".to_string(),
                label: "Function".to_string(),
                field_type: NodeFieldType::Datalist,
                data_source: Some(NodeFieldDataSource::FunctionPipelines),
                placeholder: Some("select or type a function pipeline slug".to_string()),
                help: Some(
                    "Function pipeline slug for dynamic GeoJSON source. The function must return a GeoJSON FeatureCollection.".to_string(),
                ),
                ..Default::default()
            },
            NodeFieldDef {
                name: "cache_ttl".to_string(),
                label: "Cache TTL (s)".to_string(),
                field_type: NodeFieldType::Number,
                default_value: Some(json!(60)),
                help: Some(
                    "Feature cache duration in seconds for function layers. Default: 60.".to_string(),
                ),
                ..Default::default()
            },
        ],
        layout: vec![
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("name".to_string()),
                    LayoutItem::Field("path".to_string()),
                ],
            },
            LayoutItem::Field("source_path".to_string()),
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("bbox_required".to_string()),
                    LayoutItem::Field("max_features".to_string()),
                ],
            },
            LayoutItem::Field("allowed_properties".to_string()),
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("min_zoom".to_string()),
                    LayoutItem::Field("max_zoom".to_string()),
                ],
            },
            LayoutItem::Field("no_optimize".to_string()),
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("function".to_string()),
                    LayoutItem::Field("cache_ttl".to_string()),
                ],
            },
        ],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Publish a GeoParquet layer", "ms.publish --name suburbs --path suburbs --source-path datasets/suburbs.parquet --source-kind geoparquet --min-zoom 8 --max-zoom 14")
                .output(serde_json::json!({ "ms": { "operation": "publish", "layer": { "name": "suburbs", "path": "suburbs", "url": "/ms/acme/shop/suburbs", "source_kind": "geoparquet" } } }))
                .note("A page loads it with `zeb/deckgl` from `/ms/{owner}/{project}/suburbs` (help topic `guide/mapserver`)."),
        ],
        ..Default::default()
    }
}

pub fn unpublish_definition() -> NodeDefinition {
    NodeDefinition {
        kind: UNPUBLISH_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "MS Unpublish".to_string(),
        description: "Take a published map layer offline: removes `--name` from the project's layer registry so `/ms/{owner}/{project}/{path}` \
            stops answering. The source file in the file store is left where it is. Replaces the payload with \
            `{ ms: { operation: \"unpublish\", removed, layer_id } }`; an unknown name answers `removed: false` rather than failing."
            .to_string(),
        input_schema: json!({"type": "object"}),
        output_schema: json!({
            "type": "object",
            "properties": {
                "ms": {
                    "type": "object",
                    "properties": {
                        "operation": { "type": "string" },
                        "removed": { "type": "boolean" },
                        "layer_id": { "type": "string" }
                    }
                }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![scalar_flag(
            "--name",
            "name",
            "Layer identifier to remove (required).",
        )],
        fields: vec![text_field(
            "name",
            "Layer ID",
            "Layer identifier to remove.",
        )],
        layout: vec![LayoutItem::Field("name".to_string())],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Retire a layer", "ms.unpublish --name suburbs")
                .output(serde_json::json!({ "ms": { "operation": "unpublish", "removed": true, "layer_id": "suburbs" } })),
        ],
        ..Default::default()
    }
}

pub fn get_definition() -> NodeDefinition {
    NodeDefinition {
        kind: GET_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "MS Get".to_string(),
        description: "Read one published map layer's registry entry by `--name`: its path, url, source, zoom range, style and cache \
            settings. Replaces the payload with `{ ms: { operation: \"get\", layer } }`. Use it in an admin page's pipeline to show \
            what is live, or before `ms.publish` to decide between create and update; a name that is not published fails the node."
            .to_string(),
        input_schema: json!({"type": "object"}),
        output_schema: json!({
            "type": "object",
            "properties": {
                "ms": {
                    "type": "object",
                    "properties": {
                        "operation": { "type": "string" },
                        "layer": { "type": "object" }
                    }
                }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![scalar_flag(
            "--name",
            "name",
            "Layer identifier to look up (required).",
        )],
        fields: vec![text_field(
            "name",
            "Layer ID",
            "Layer identifier to look up.",
        )],
        layout: vec![LayoutItem::Field("name".to_string())],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Show a layer's settings", "ms.get --name suburbs")
                .output(serde_json::json!({ "ms": { "operation": "get", "layer": { "name": "suburbs", "path": "suburbs", "url": "/ms/acme/shop/suburbs", "source_kind": "geoparquet", "min_zoom": 8, "max_zoom": 14 } } })),
        ],
        ..Default::default()
    }
}

pub fn list_definition() -> NodeDefinition {
    NodeDefinition {
        kind: LIST_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "MS List".to_string(),
        description: "List every map layer this project has published, with the same registry fields `ms.get` returns for one. \
            No flags. Replaces the payload with `{ ms: { operation: \"list\", count, layers } }` — a page reads `input.ms.layers`. \
            This is the registry, not the file store: a GeoParquet file nobody published is not in it."
            .to_string(),
        input_schema: json!({"type": "object"}),
        output_schema: json!({
            "type": "object",
            "properties": {
                "ms": {
                    "type": "object",
                    "properties": {
                        "operation": { "type": "string" },
                        "count": { "type": "integer" },
                        "layers": { "type": "array" }
                    }
                }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![],
        fields: vec![],
        layout: vec![],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Layers for a map picker", "ms.list")
                .output(serde_json::json!({ "ms": { "operation": "list", "count": 1, "layers": [{ "name": "suburbs", "path": "suburbs", "url": "/ms/acme/shop/suburbs" }] } })),
        ],
        ..Default::default()
    }
}

// ── Node ─────────────────────────────────────────────────────────────────────

pub struct Node {
    config: Config,
    platform: Arc<PlatformService>,
    operation: Operation,
}

impl Node {
    pub fn new(
        config: Config,
        platform: Arc<PlatformService>,
        operation: Operation,
    ) -> Result<Self, PipelineError> {
        Ok(Self {
            config,
            platform,
            operation,
        })
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        self.operation.kind()
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

        let payload = match self.operation {
            Operation::Publish => self.exec_publish(owner, project, &input)?,
            Operation::Unpublish => self.exec_unpublish(owner, project)?,
            Operation::Get => self.exec_get(owner, project)?,
            Operation::List => self.exec_list(owner, project)?,
        };

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload,
            trace: vec![format!(
                "node_kind={} operation={}",
                self.operation.kind(),
                match self.operation {
                    Operation::Publish => "publish",
                    Operation::Unpublish => "unpublish",
                    Operation::Get => "get",
                    Operation::List => "list",
                }
            )],
        })
    }
}

impl Node {
    fn exec_publish(
        &self,
        owner: &str,
        project: &str,
        input: &NodeExecutionInput,
    ) -> Result<Value, PipelineError> {
        let name = require_non_empty(&self.config.name, "--name", "FW_NODE_MS_PUBLISH")?;
        let path = require_non_empty(&self.config.path, "--path", "FW_NODE_MS_PUBLISH")?;

        // ── GeoJsonFunction mode: --function set → skip source-path / optimization
        let is_function_mode = self
            .config
            .function
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty());

        // source_path: config takes priority, then input payload (not required for function mode)
        let mut source_path = if is_function_mode {
            String::new()
        } else if !self.config.source_path.trim().is_empty() {
            self.config.source_path.trim().to_string()
        } else {
            source_path_from_payload(&input.payload)?
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    PipelineError::new(
                        "FW_NODE_MS_PUBLISH",
                        "--source-path is required (config or input payload.source_path), or use --function for dynamic sources",
                    )
                })?
        };

        let layout = self
            .platform
            .file
            .ensure_project_layout(owner, project)
            .map_err(|e| PipelineError::new("FW_NODE_MS_PUBLISH", e.to_string()))?;

        if !is_function_mode {
            // Validate source file exists in ZebFS
            let zebfs = layout.open_files();
            zebfs.head(&source_path).map_err(|e| {
                PipelineError::new(
                    "FW_NODE_MS_PUBLISH",
                    format!("source file not found in ZebFS: {e}"),
                )
            })?;
        }

        let bbox_required = if self.config.no_bbox_required.unwrap_or(false) {
            false
        } else {
            self.config.bbox_required.unwrap_or(true)
        };
        let max_features = self.config.max_features.unwrap_or(1000);
        let allowed_properties: Vec<String> = self
            .config
            .allowed_properties
            .as_deref()
            .unwrap_or("")
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let mut artifact_manifest_path: Option<String> = None;
        let mut feature_count: Option<usize> = None;
        let mut chunk_count: Option<usize> = None;
        let mut optimization_info: Option<Value> = None;

        // ── Auto-detect format and optimize ──────────────────────────────
        let source_abs = layout.files_dir.join(source_path.trim_start_matches('/'));
        let source_lower = source_path.to_ascii_lowercase();

        // Determine if we should optimize (default=yes, opt-out with --no-optimize)
        let should_optimize = !self.config.no_optimize && !is_function_mode;

        let effective_kind = if is_function_mode {
            "geojson_function".to_string()
        } else if should_optimize {
            // Auto-detect format from file extension
            let is_geojson = source_lower.ends_with(".geojson") || source_lower.ends_with(".json");
            let is_parquet = source_lower.ends_with(".parquet") || source_lower.ends_with(".pq");

            if is_parquet {
                // Check if already optimized
                if crate::mapserver::resolve::geoparquet_optimize::is_already_optimized(&source_abs)
                {
                    eprintln!("geoparquet already optimized, registering as-is");
                    optimization_info = Some(json!({
                        "applied": false,
                        "reason": "already_optimized",
                        "source_format": "parquet",
                    }));
                    "geoparquet".to_string()
                } else {
                    // Optimize: add bbox columns, Hilbert sort, small row groups
                    let optimized_dir = layout.files_dir.join("mapserver/.optimized");
                    std::fs::create_dir_all(&optimized_dir).map_err(|e| {
                        PipelineError::new("FW_NODE_MS_PUBLISH", format!("mkdir failed: {e}"))
                    })?;
                    let optimized_filename = format!("{name}.spatial.parquet");
                    let optimized_abs = optimized_dir.join(&optimized_filename);
                    let optimized_rel = format!("mapserver/.optimized/{optimized_filename}");

                    let report =
                        crate::mapserver::resolve::geoparquet_optimize::optimize_geoparquet(
                            &source_abs,
                            &optimized_abs,
                        )
                        .map_err(|e| {
                            PipelineError::new(
                                "FW_NODE_MS_PUBLISH",
                                format!("optimize failed: {e}"),
                            )
                        })?;

                    // Write column stats sidecar JSON
                    let stats_filename = format!("{name}.spatial.stats.json");
                    let stats_abs = optimized_dir.join(&stats_filename);
                    let stats_rel = format!("mapserver/.optimized/{stats_filename}");
                    let stats_report =
                        crate::mapserver::resolve::geoparquet_optimize::ColumnStatsReport {
                            row_count: report.rows,
                            columns: report.column_stats.clone(),
                        };
                    if let Ok(stats_json) = serde_json::to_string_pretty(&stats_report) {
                        let _ = std::fs::write(&stats_abs, stats_json);
                    }

                    eprintln!(
                        "geoparquet optimized: {} rows, {} row groups, {:.1}MB → {:.1}MB, {} column stats",
                        report.rows,
                        report.row_groups,
                        report.source_bytes as f64 / 1_048_576.0,
                        report.dest_bytes as f64 / 1_048_576.0,
                        report.column_stats.len(),
                    );

                    source_path = optimized_rel;
                    feature_count = Some(report.rows);
                    optimization_info = Some(json!({
                        "applied": true,
                        "source_format": "parquet",
                        "rows": report.rows,
                        "row_groups": report.row_groups,
                        "column_stats_count": report.column_stats.len(),
                        "column_stats_path": stats_rel,
                    }));
                    "geoparquet".to_string()
                }
            } else if is_geojson {
                // Convert GeoJSON → raw Parquet → optimize
                let optimized_dir = layout.files_dir.join("mapserver/.optimized");
                std::fs::create_dir_all(&optimized_dir).map_err(|e| {
                    PipelineError::new("FW_NODE_MS_PUBLISH", format!("mkdir failed: {e}"))
                })?;

                let temp_raw = optimized_dir.join(format!("{name}.raw.parquet"));
                let optimized_filename = format!("{name}.spatial.parquet");
                let optimized_abs = optimized_dir.join(&optimized_filename);
                let optimized_rel = format!("mapserver/.optimized/{optimized_filename}");

                // Step 1: GeoJSON → raw GeoParquet
                let convert_report =
                    crate::mapserver::resolve::geoparquet_optimize::convert_geojson_to_geoparquet(
                        &source_abs,
                        &temp_raw,
                    )
                    .map_err(|e| {
                        PipelineError::new(
                            "FW_NODE_MS_PUBLISH",
                            format!("GeoJSON conversion failed: {e}"),
                        )
                    })?;

                eprintln!(
                    "geojson converted: {} features, {} columns",
                    convert_report.feature_count, convert_report.column_count,
                );

                // Step 2: optimize the raw parquet
                let opt_report =
                    crate::mapserver::resolve::geoparquet_optimize::optimize_geoparquet(
                        &temp_raw,
                        &optimized_abs,
                    )
                    .map_err(|e| {
                        // Clean up temp file on error
                        let _ = std::fs::remove_file(&temp_raw);
                        PipelineError::new("FW_NODE_MS_PUBLISH", format!("optimize failed: {e}"))
                    })?;

                // Clean up intermediate raw parquet
                let _ = std::fs::remove_file(&temp_raw);

                // Write column stats sidecar JSON
                let stats_filename = format!("{name}.spatial.stats.json");
                let stats_abs = optimized_dir.join(&stats_filename);
                let stats_rel = format!("mapserver/.optimized/{stats_filename}");
                let stats_report =
                    crate::mapserver::resolve::geoparquet_optimize::ColumnStatsReport {
                        row_count: opt_report.rows,
                        columns: opt_report.column_stats.clone(),
                    };
                if let Ok(stats_json) = serde_json::to_string_pretty(&stats_report) {
                    let _ = std::fs::write(&stats_abs, stats_json);
                }

                eprintln!(
                    "geoparquet optimized: {} rows, {} row groups, {:.1}MB, {} column stats",
                    opt_report.rows,
                    opt_report.row_groups,
                    opt_report.dest_bytes as f64 / 1_048_576.0,
                    opt_report.column_stats.len(),
                );

                feature_count = Some(convert_report.feature_count);
                source_path = optimized_rel;
                optimization_info = Some(json!({
                    "applied": true,
                    "source_format": "geojson",
                    "features": convert_report.feature_count,
                    "columns": convert_report.column_count,
                    "rows": opt_report.rows,
                    "row_groups": opt_report.row_groups,
                    "column_stats_count": opt_report.column_stats.len(),
                    "column_stats_path": stats_rel,
                }));
                "geoparquet".to_string()
            } else {
                // Unknown format — cannot auto-optimize
                return Err(PipelineError::new(
                    "FW_NODE_MS_PUBLISH",
                    format!(
                        "cannot auto-detect format for '{}'; \
                         use --no-optimize to publish without conversion, \
                         or rename the file with a .geojson or .parquet extension",
                        source_path
                    ),
                ));
            }
        } else {
            // --no-optimize: legacy behavior
            let source_kind = if self.config.source_kind.trim().is_empty() {
                // Auto-detect source_kind for --no-optimize
                if source_lower.ends_with(".parquet") || source_lower.ends_with(".pq") {
                    "geoparquet".to_string()
                } else {
                    "geojson_file".to_string()
                }
            } else {
                self.config.source_kind.trim().to_ascii_lowercase()
            };

            // Validate source_kind
            if !matches!(
                source_kind.as_str(),
                "geojson_file" | "geojson_artifact" | "geoparquet" | "geojson_function"
            ) {
                return Err(PipelineError::new(
                    "FW_NODE_MS_PUBLISH",
                    format!(
                        "unsupported --source-kind '{source_kind}'; \
                         expected geojson_file, geojson_artifact, geoparquet, or geojson_function"
                    ),
                ));
            }

            // Legacy: build artifact for geojson if requested
            if (source_kind == "geojson_file" && self.config.build_artifact)
                || source_kind == "geojson_artifact"
            {
                // Artifacts live in the cache tier; a pre-tier
                // `files/mapserver/.artifacts` tree migrates on first touch.
                let artifact_home = layout.ensure_mapserver_artifacts_home().map_err(|e| {
                    PipelineError::new(
                        "FW_NODE_MS_PUBLISH",
                        format!("artifact tier migration refused: {e}"),
                    )
                })?;
                let artifact_rel = format!("mapserver-artifacts/{DEFAULT_INSTANCE}/{name}");
                let artifact_abs = artifact_home.join(DEFAULT_INSTANCE).join(name);
                let build_out = crate::mapserver::publish::build::build_geojson_artifact(
                    &source_abs,
                    name,
                    &artifact_abs,
                    &artifact_rel,
                )
                .map_err(|e| PipelineError::new("FW_NODE_MS_PUBLISH", e))?;

                artifact_manifest_path = Some(build_out.manifest_rel_path);
                feature_count = Some(build_out.feature_count);
                chunk_count = Some(build_out.chunk_count);
                "geojson_artifact".to_string()
            } else {
                source_kind
            }
        };

        // Build style: prefer --style DSL over individual fill/stroke flags
        let style = if let Some(ref dsl) = self.config.style_dsl {
            // Validate DSL syntax at publish time
            crate::mapserver::resolve::style_dsl::parse_style_dsl(dsl).map_err(|e| {
                PipelineError::new("MS_PUBLISH_STYLE", format!("invalid style DSL: {e}"))
            })?;
            Some(json!(dsl)) // Store as JSON string value
        } else {
            let mut obj = serde_json::Map::new();
            if let Some(ref v) = self.config.fill {
                obj.insert("fill".into(), json!(v));
            }
            if let Some(ref v) = self.config.stroke {
                obj.insert("stroke".into(), json!(v));
            }
            if let Some(ref v) = self.config.stroke_width {
                if let Ok(f) = v.parse::<f32>() {
                    obj.insert("stroke_width".into(), json!(f));
                }
            }
            if let Some(ref v) = self.config.point_radius {
                if let Ok(f) = v.parse::<f32>() {
                    obj.insert("point_radius".into(), json!(f));
                }
            }
            if let Some(ref v) = self.config.point_color {
                obj.insert("point_color".into(), json!(v));
            }
            if obj.is_empty() {
                None
            } else {
                Some(Value::Object(obj))
            }
        };

        // Validate filter syntax at publish time
        let filter = if let Some(ref f) = self.config.filter {
            crate::mapserver::resolve::filter_dsl::parse_filter(f).map_err(|e| {
                PipelineError::new("MS_PUBLISH_FILTER", format!("invalid filter: {e}"))
            })?;
            Some(f.clone())
        } else {
            None
        };

        // Upsert into registry
        let record = LayerRecord {
            layer_id: name.to_string(),
            path: registry::normalize_layer_path(path).to_string(),
            source_path: source_path.clone(),
            source_kind: effective_kind,
            artifact_manifest_path,
            mode: "features".to_string(),
            min_zoom: self.config.min_zoom,
            max_zoom: self.config.max_zoom,
            bbox_required,
            max_features,
            allowed_properties,
            feature_count,
            chunk_count,
            style,
            filter,
            function_slug: if is_function_mode {
                self.config.function.clone()
            } else {
                None
            },
            cache_ttl_secs: if is_function_mode {
                self.config.cache_ttl
            } else {
                None
            },
        };

        let mut layers = read_layers(&self.platform, owner, project)?;
        if let Some(pos) = layers.iter().position(|l| l.layer_id == name) {
            layers[pos] = record.clone();
        } else {
            layers.push(record.clone());
        }
        write_layers(&self.platform, owner, project, &layers)?;

        let mut result = json!({
            "ms": {
                "operation": "publish",
                "layer": layer_to_json(owner, project, &record)
            }
        });
        if let Some(opt) = optimization_info {
            result["ms"]["optimization"] = opt;
        }
        Ok(result)
    }

    fn exec_unpublish(&self, owner: &str, project: &str) -> Result<Value, PipelineError> {
        let name = require_non_empty(&self.config.name, "--name", "FW_NODE_MS_UNPUBLISH")?;
        let mut layers = read_layers(&self.platform, owner, project)?;
        let removed_record = layers.iter().find(|l| l.layer_id == name).cloned();
        let before = layers.len();
        layers.retain(|l| l.layer_id != name);
        let removed = layers.len() < before;
        write_layers(&self.platform, owner, project, &layers)?;

        // Clean up artifact directory and optimized file if they exist
        if removed {
            if let Ok(layout) = self.platform.file.ensure_project_layout(owner, project) {
                // Move a pre-tier artifact tree to its cache home first so
                // the cleanup hits the tree wherever it actually lives.
                let artifact_home = layout.ensure_mapserver_artifacts_home().ok();
                if let Some(rel) = removed_record
                    .as_ref()
                    .and_then(|record| record.artifact_manifest_path.as_deref())
                {
                    // The record's own rel path covers legacy shapes such as
                    // `mapserver/.artifacts/{name}` (no instance segment).
                    if let Some(dir) = layout.resolve_mapserver_artifact_path(rel).parent() {
                        let _ = std::fs::remove_dir_all(dir);
                    }
                }
                if let Some(artifact_dir) =
                    artifact_home.map(|home| home.join(DEFAULT_INSTANCE).join(name))
                {
                    if artifact_dir.exists() {
                        let _ = std::fs::remove_dir_all(&artifact_dir);
                    }
                }

                let optimized_file = layout
                    .files_dir
                    .join("mapserver")
                    .join(".optimized")
                    .join(format!("{}.spatial.parquet", name));
                if optimized_file.exists() {
                    let _ = std::fs::remove_file(&optimized_file);
                }

                let stats_file = layout
                    .files_dir
                    .join("mapserver")
                    .join(".optimized")
                    .join(format!("{}.spatial.stats.json", name));
                if stats_file.exists() {
                    let _ = std::fs::remove_file(&stats_file);
                }
            }
        }

        Ok(json!({
            "ms": {
                "operation": "unpublish",
                "layer_id": name,
                "removed": removed
            }
        }))
    }

    fn exec_get(&self, owner: &str, project: &str) -> Result<Value, PipelineError> {
        let name = require_non_empty(&self.config.name, "--name", "FW_NODE_MS_GET")?;
        let layers = read_layers(&self.platform, owner, project)?;
        let layer = layers.iter().find(|l| l.layer_id == name);

        match layer {
            Some(record) => Ok(json!({
                "ms": {
                    "operation": "get",
                    "found": true,
                    "layer": layer_to_json(owner, project, record)
                }
            })),
            None => Ok(json!({
                "ms": {
                    "operation": "get",
                    "found": false,
                    "layer_id": name
                }
            })),
        }
    }

    fn exec_list(&self, owner: &str, project: &str) -> Result<Value, PipelineError> {
        let layers = read_layers(&self.platform, owner, project)?;
        let items: Vec<Value> = layers
            .iter()
            .map(|r| layer_to_json(owner, project, r))
            .collect();

        Ok(json!({
            "ms": {
                "operation": "list",
                "count": items.len(),
                "layers": items
            }
        }))
    }
}

fn require_non_empty<'a>(
    value: &'a str,
    flag: &str,
    code: &'static str,
) -> Result<&'a str, PipelineError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(PipelineError::new(code, format!("{flag} is required")));
    }
    Ok(trimmed)
}

fn source_path_from_payload(payload: &Value) -> Result<Option<String>, PipelineError> {
    payload
        .get("source_path")
        .map(zebfs_rel_path_or_string)
        .transpose()
        .map(Option::flatten)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::source_path_from_payload;

    #[test]
    fn publish_source_path_accepts_file_ref_payload() {
        let payload = json!({
            "source_path": {
                "__zf_type": "file_ref",
                "backend": "zebfs",
                "ref": "mapserver/sources/roads.geojson",
                "sha256": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "mime": "application/geo+json",
                "size": 1,
                "lifecycle": "durable"
            }
        });

        assert_eq!(
            source_path_from_payload(&payload).unwrap(),
            Some("mapserver/sources/roads.geojson".to_string())
        );
    }

    /// `ref` is opaque to every node but its own backend
    /// (`kinds/file-ref/README.md`): a remote handle is never joined to a
    /// local path.
    #[test]
    fn publish_source_path_refuses_a_foreign_backend_ref() {
        let payload = json!({
            "source_path": {
                "__zf_type": "file_ref",
                "backend": "s3",
                "ref": "s3://bucket/roads.geojson",
                "filename": "roads.geojson",
                "mime": "application/geo+json",
                "kind": "geojson",
                "size": 1,
                "sha256": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "lifecycle": "durable",
                "origin": "webhook",
                "trust": "untrusted"
            }
        });

        assert_eq!(
            source_path_from_payload(&payload).unwrap_err().code,
            "FW_FILE_REF_BACKEND"
        );
    }
}

//! `ms.layer.*` — the project's map layers.
//!
//! The layer registry (`data/store/mapserver/{instance}.layers.json`) holds
//! one record per layer; a published layer answers on
//! `/ms/{owner}/{project}/{route}` while the project's `ms` surface is on.
//!
//! | Node                 | Does                                   | Answers                       |
//! |----------------------|----------------------------------------|-------------------------------|
//! | `ms.layer.publish`   | upserts a layer by `--name`            | `layer: { …record, serving }` |
//! | `ms.layer.get`       | reads one layer by `--name`            | `layer: { found, …record }`   |
//! | `ms.layer.list`      | every layer                            | `layer: { items, count }`     |
//! | `ms.layer.unpublish` | removes a layer by `--name`            | `layer: { name, removed }`    |
//!
//! Publishing a GeoJSON or GeoParquet source builds an optimized GeoParquet
//! copy (bbox columns, Hilbert order, small row groups) under
//! `mapserver/.optimized/` in this node's store and serves that; with
//! `--skip-optimize` the source is served as it is, and `--build-artifact`
//! then serves a GeoJSON as a chunked artifact in the cache tier instead.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::contracts::kinds::MapserverLayerRecord as LayerRecord;
use crate::mapserver::publish::registry;
use crate::mapserver::publish::registry::DEFAULT_INSTANCE;
use crate::pipeline::model::{NodeCapability, NodeExample};
use crate::pipeline::nodes::shared::file_ref::zebfs_rel_path_or_string;
use crate::pipeline::nodes::shared::limits::{choice, whole, within};
use crate::pipeline::nodes::shared::project_store::{self, NodeStore};
use crate::pipeline::nodes::shared::store_scratch::StoreScratch;
use crate::pipeline::nodes::shared::units;
use crate::pipeline::nodes::shared::util::{metadata_scope, with_answer};
use crate::pipeline::{
    NodeDefinition, NodeFieldDataSource, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType, SelectOptionDef},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

pub const PUBLISH_KIND: &str = "ms.layer.publish";
pub const UNPUBLISH_KIND: &str = "ms.layer.unpublish";
pub const GET_KIND: &str = "ms.layer.get";
pub const LIST_KIND: &str = "ms.layer.list";

/// Publishing failed: the source, the optimized copy or the registry.
pub const PUBLISH_CODE: &str = "FW_NODE_MS_LAYER_PUBLISH";
/// A publish flag the author set wrong.
pub const PUBLISH_CONFIG_CODE: &str = "FW_NODE_MS_LAYER_PUBLISH_CONFIG";
/// `--style` that does not parse.
const PUBLISH_STYLE_CODE: &str = "FW_NODE_MS_LAYER_PUBLISH_STYLE";
/// `--filter` that does not parse.
const PUBLISH_FILTER_CODE: &str = "FW_NODE_MS_LAYER_PUBLISH_FILTER";
pub const UNPUBLISH_CODE: &str = "FW_NODE_MS_LAYER_UNPUBLISH";
pub const UNPUBLISH_CONFIG_CODE: &str = "FW_NODE_MS_LAYER_UNPUBLISH_CONFIG";
pub const GET_CODE: &str = "FW_NODE_MS_LAYER_GET";
pub const GET_CONFIG_CODE: &str = "FW_NODE_MS_LAYER_GET_CONFIG";
pub const LIST_CODE: &str = "FW_NODE_MS_LAYER_LIST";

const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

/// How a source is read: `--parse`.
const PARSE_WORDS: &[&str] = &["geojson", "geoparquet"];
/// Features one query answers when `--max-items` is not set, and the most it may say.
const DEFAULT_MAX_ITEMS: u32 = 1_000;
const MAX_MAX_ITEMS: u32 = 50_000;
/// The most `--field` properties a layer serves.
const MAX_FIELDS: u32 = 256;
/// Web map zoom levels.
const MAX_ZOOM: u32 = 24;
/// The longest a function layer's features are cached.
const MAX_TTL_SECS: u64 = 7 * 86_400;

// ── Registry ─────────────────────────────────────────────────────────────────
//
// There is one record definition, the contract's. A hand-mirrored copy lived
// here and had already drifted, which is how two writers ended up disagreeing
// about the same file.

fn layers_path(platform: &PlatformService, owner: &str, project: &str, code: &'static str) -> Result<std::path::PathBuf, PipelineError> {
    let layout = platform.file.ensure_project_layout(owner, project).map_err(|e| PipelineError::new(code, e.to_string()))?;
    Ok(registry::layers_manifest_path(&layout.data_store_dir(), DEFAULT_INSTANCE))
}

fn read_layers(platform: &PlatformService, owner: &str, project: &str, code: &'static str) -> Result<Vec<LayerRecord>, PipelineError> {
    let path = layers_path(platform, owner, project, code)?;
    registry::read_layers(&path).map_err(|e| PipelineError::new(code, format!("layer registry: {e}")))
}

fn write_layers(platform: &PlatformService, owner: &str, project: &str, items: &[LayerRecord], code: &'static str) -> Result<(), PipelineError> {
    let path = layers_path(platform, owner, project, code)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| PipelineError::new(code, e.to_string()))?;
    }
    registry::write_layers(&path, DEFAULT_INSTANCE, items).map_err(|e| PipelineError::new(code, format!("layer registry: {e}")))
}

/// Streams an optimized GeoParquet and its column stats into the store.
fn store_optimized(
    store: &NodeStore,
    optimized_abs: &std::path::Path,
    optimized_rel: &str,
    stats: &crate::mapserver::resolve::geoparquet_optimize::ColumnStatsReport,
    stats_rel: &str,
) -> Result<(), PipelineError> {
    store
        .fs
        .put_from_file(optimized_rel, optimized_abs)
        .map_err(|e| PipelineError::new(PUBLISH_CODE, format!("write '{optimized_rel}': {e}")))?;
    let stats_json = serde_json::to_vec_pretty(stats).map_err(|e| PipelineError::new(PUBLISH_CODE, e.to_string()))?;
    store.fs.put(stats_rel, &stats_json).map_err(|e| PipelineError::new(PUBLISH_CODE, format!("write '{stats_rel}': {e}")))?;
    Ok(())
}

/// A registry record as the nodes answer it, in the flags' words.
fn layer_to_json(record: &LayerRecord) -> Map<String, Value> {
    let source_kind = if record.source_kind.is_empty() {
        if record.artifact_manifest_path.is_some() { "geojson_artifact" } else { "geojson_file" }
    } else {
        record.source_kind.as_str()
    };
    let layer = json!({
        "name": record.layer_id,
        "route": record.path,
        "store": record.store,
        "source": record.source_path,
        "source_kind": source_kind,
        "min_zoom": record.min_zoom,
        "max_zoom": record.max_zoom,
        "bbox_required": record.bbox_required,
        "max_items": record.max_features,
        "fields": record.allowed_properties,
        "feature_count": record.feature_count,
        "chunk_count": record.chunk_count,
        "style": record.style,
        "filter": record.filter,
        "function": record.function_slug,
        "ttl": record.cache_ttl_secs.map(|secs| format!("{secs}s")),
    });
    match layer {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

// ── Operation ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub enum Operation {
    Publish,
    Unpublish,
    Get,
    List,
}

impl Operation {
    pub fn from_kind(kind: &str) -> Option<Self> {
        match kind {
            PUBLISH_KIND => Some(Self::Publish),
            UNPUBLISH_KIND => Some(Self::Unpublish),
            GET_KIND => Some(Self::Get),
            LIST_KIND => Some(Self::List),
            _ => None,
        }
    }

    fn kind(self) -> &'static str {
        match self {
            Self::Publish => PUBLISH_KIND,
            Self::Unpublish => UNPUBLISH_KIND,
            Self::Get => GET_KIND,
            Self::List => LIST_KIND,
        }
    }

    /// The operation's own failure code.
    pub fn code(self) -> &'static str {
        match self {
            Self::Publish => PUBLISH_CODE,
            Self::Unpublish => UNPUBLISH_CODE,
            Self::Get => GET_CODE,
            Self::List => LIST_CODE,
        }
    }

    /// The code a config the author set wrong is refused under.
    pub fn config_code(self) -> &'static str {
        match self {
            Self::Publish => PUBLISH_CONFIG_CODE,
            Self::Unpublish => UNPUBLISH_CONFIG_CODE,
            Self::Get => GET_CONFIG_CODE,
            Self::List => LIST_CODE,
        }
    }
}

// ── Config ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// The layer's name: 1–64 letters, digits, `-` or `_`.
    #[serde(default)]
    pub name: String,
    /// Where the layer answers, under `/ms/{owner}/{project}/`.
    #[serde(default)]
    pub route: String,
    /// The source: a store key, or a FileRef through `{{ }}`.
    #[serde(default)]
    pub from: Value,
    /// How the source is read: `geojson` or `geoparquet` (else from its extension).
    #[serde(default)]
    pub parse: String,
    /// Serve queries without a bbox (default: a bbox is required).
    #[serde(default)]
    pub bbox_optional: bool,
    /// Features one query answers at most (default 1000).
    #[serde(default)]
    pub max_items: Value,
    /// The source properties the public may see, one per `--field`.
    #[serde(default)]
    pub field: Value,
    #[serde(default)]
    pub min_zoom: Value,
    #[serde(default)]
    pub max_zoom: Value,
    /// With `--skip-optimize`, serve a GeoJSON as a chunked artifact.
    #[serde(default)]
    pub build_artifact: bool,
    #[serde(default)]
    pub fill: Option<String>,
    #[serde(default)]
    pub stroke: Option<String>,
    #[serde(default)]
    pub stroke_width: Value,
    #[serde(default)]
    pub point_radius: Value,
    #[serde(default)]
    pub point_color: Option<String>,
    /// A style expression (`cb(population,5,YlOrRd)`); overrides the colour flags.
    #[serde(default)]
    pub style: Option<String>,
    /// The default filter (`category:residential;value>100`).
    #[serde(default)]
    pub filter: Option<String>,
    /// Serve the source as it is, without the optimized GeoParquet copy.
    #[serde(default)]
    pub skip_optimize: bool,
    /// A function pipeline whose GeoJSON FeatureCollection is the layer.
    #[serde(default)]
    pub function: Option<String>,
    /// How long a function layer's features are cached (default 60s).
    #[serde(default)]
    pub ttl: String,
    /// The store the source is read from and the optimized copy written to;
    /// saved explicitly at registration (`node-conventions.md` §5).
    #[serde(default)]
    pub store: Option<String>,
}

/// The publish flags read and checked before anything is touched.
#[derive(Debug, PartialEq)]
struct PublishSettings {
    /// `--parse`, `None` to sniff from the extension.
    parse: Option<&'static str>,
    max_items: usize,
    fields: Vec<String>,
    min_zoom: Option<u8>,
    max_zoom: Option<u8>,
    stroke_width: Option<f64>,
    point_radius: Option<f64>,
    function: Option<String>,
    ttl_secs: Option<u64>,
}

impl Config {
    fn publish_settings(&self) -> Result<PublishSettings, PipelineError> {
        const CODE: &str = PUBLISH_CONFIG_CODE;
        let parse = Some(choice(&self.parse, PARSE_WORDS, "", "--parse", CODE)?).filter(|word| !word.is_empty());
        let max_items = match whole(&self.max_items, "--max-items", CODE)? {
            None => DEFAULT_MAX_ITEMS,
            Some(n) => within(n, 1, MAX_MAX_ITEMS, "--max-items", CODE)?,
        } as usize;
        let zoom = |value: &Value, flag: &str| -> Result<Option<u8>, PipelineError> {
            whole(value, flag, CODE)?.map(|n| within(n, 0, MAX_ZOOM, flag, CODE).map(|n| n as u8)).transpose()
        };
        let (min_zoom, max_zoom) = (zoom(&self.min_zoom, "--min-zoom")?, zoom(&self.max_zoom, "--max-zoom")?);
        if let (Some(min), Some(max)) = (min_zoom, max_zoom)
            && min > max
        {
            return Err(PipelineError::new(CODE, format!("--min-zoom {min} is above --max-zoom {max}")));
        }
        let function = self.function.as_deref().map(str::trim).filter(|f| !f.is_empty()).map(str::to_string);
        let has_from = !(self.from.is_null() || self.from.as_str().is_some_and(|s| s.trim().is_empty()));
        if function.is_some() && has_from {
            return Err(PipelineError::new(CODE, "--from and --function are two sources; give one"));
        }
        if function.is_none() && !has_from {
            return Err(PipelineError::new(CODE, "--from is required (a store key or a FileRef), or --function for a function layer"));
        }
        let ttl_secs = match self.ttl.trim() {
            "" => None,
            text => {
                if function.is_none() {
                    return Err(PipelineError::new(CODE, "--ttl caches a function layer's features; it needs --function"));
                }
                let secs = units::duration(text, "--ttl", CODE)?.as_secs();
                Some(within(secs, 1, MAX_TTL_SECS, "--ttl (seconds)", CODE)?)
            }
        };
        if self.build_artifact && (!self.skip_optimize || parse == Some("geoparquet")) {
            return Err(PipelineError::new(CODE, "--build-artifact serves a GeoJSON as it is: it needs --skip-optimize and a GeoJSON source"));
        }
        Ok(PublishSettings {
            parse,
            max_items,
            fields: fields(&self.field)?,
            min_zoom,
            max_zoom,
            stroke_width: number(&self.stroke_width, "--stroke-width", 0.0, 100.0)?,
            point_radius: number(&self.point_radius, "--point-radius", 0.0, 100.0)?,
            function,
            ttl_secs,
        })
    }
}

/// `--field`, repeated or one `{{ [list] }}`: property names, never a comma list.
fn fields(value: &Value) -> Result<Vec<String>, PipelineError> {
    let items: Vec<&Value> = match value {
        Value::Null => Vec::new(),
        Value::Array(items) => items.iter().collect(),
        other => vec![other],
    };
    let mut out = Vec::new();
    for item in items {
        let Some(name) = item.as_str().map(str::trim) else {
            return Err(PipelineError::new(PUBLISH_CONFIG_CODE, format!("--field {item} is not a property name")));
        };
        if name.is_empty() {
            continue;
        }
        if name.contains(',') {
            return Err(PipelineError::new(PUBLISH_CONFIG_CODE, format!("--field '{name}' is one property name; repeat --field for each")));
        }
        if !out.iter().any(|seen| seen == name) {
            out.push(name.to_string());
        }
    }
    if out.len() > MAX_FIELDS as usize {
        return Err(PipelineError::new(PUBLISH_CONFIG_CODE, format!("--field names at most {MAX_FIELDS} properties")));
    }
    Ok(out)
}

/// A number flag — a number, or the number as text — inside `min..=max`.
fn number(value: &Value, flag: &str, min: f64, max: f64) -> Result<Option<f64>, PipelineError> {
    let n = match value {
        Value::Null => return Ok(None),
        Value::String(text) if text.trim().is_empty() => return Ok(None),
        Value::Number(n) => n.as_f64(),
        Value::String(text) => text.trim().parse::<f64>().ok(),
        _ => None,
    };
    match n {
        Some(n) if n.is_finite() => within(n, min, max, flag, PUBLISH_CONFIG_CODE).map(Some),
        _ => Err(PipelineError::new(PUBLISH_CONFIG_CODE, format!("{flag} '{value}' is not a number"))),
    }
}

// ── Node definitions ─────────────────────────────────────────────────────────

fn flag(name: &str, config_key: &str, description: &str, value: &str) -> DslFlag {
    DslFlag {
        flag: name.to_string(),
        config_key: config_key.to_string(),
        description: description.to_string(),
        kind: DslFlagKind::Scalar,
        value: value.to_string(),
        ..Default::default()
    }
}

fn switch(name: &str, config_key: &str, description: &str) -> DslFlag {
    DslFlag { kind: DslFlagKind::Bool, ..flag(name, config_key, description, "") }
}

fn name_flag(description: &str) -> DslFlag {
    DslFlag { required: true, ..flag("--name", "name", description, "text") }
}

fn field(name: &str, label: &str, field_type: NodeFieldType, help: &str) -> NodeFieldDef {
    NodeFieldDef { name: name.to_string(), label: label.to_string(), field_type, help: Some(help.to_string()), ..Default::default() }
}

fn text_field(name: &str, label: &str, help: &str) -> NodeFieldDef {
    field(name, label, NodeFieldType::Text, help)
}

/// The record fields every answer carrying a layer has.
fn layer_schema() -> Value {
    json!({
        "name": { "type": "string" },
        "route": { "type": "string", "description": "Where it answers, under /ms/{owner}/{project}/" },
        "store": { "type": "string" },
        "source": { "type": "string", "description": "The store key served (the optimized copy unless --skip-optimize)" },
        "source_kind": { "type": "string", "description": "geoparquet, geojson_file, geojson_artifact or geojson_function" },
        "min_zoom": { "type": ["integer", "null"] },
        "max_zoom": { "type": ["integer", "null"] },
        "bbox_required": { "type": "boolean" },
        "max_items": { "type": "integer" },
        "fields": { "type": "array", "items": { "type": "string" } },
        "feature_count": { "type": ["integer", "null"] },
        "chunk_count": { "type": ["integer", "null"] },
        "style": {},
        "filter": { "type": ["string", "null"] },
        "function": { "type": ["string", "null"] },
        "ttl": { "type": ["string", "null"], "description": "A function layer's cache lifetime, e.g. 60s" }
    })
}

fn answer_schema(properties: Value) -> Value {
    json!({ "type": "object", "properties": { "layer": { "type": "object", "properties": properties } } })
}

pub fn publish_definition() -> NodeDefinition {
    let mut publish_props = layer_schema();
    publish_props["serving"] = json!({ "type": "boolean", "description": "The project's ms surface is on, so the route answers" });
    publish_props["note"] = json!({ "type": "string", "description": "Present when serving is false" });
    publish_props["optimization"] = json!({ "type": "object", "description": "What the optimized copy holds; absent with --skip-optimize or --function" });
    NodeDefinition {
        kind: PUBLISH_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "MS Publish".to_string(),
        description: "Publish or update a map layer, by `--name`, in the project's layer registry. It answers on `/ms/{owner}/{project}/{route}` \
            while the project's `ms` surface is on (off by default; `layer.serving` says which). `--from` is a GeoJSON or GeoParquet store key \
            or FileRef (`--parse geojson|geoparquet`, else from its extension); it is served from an optimized GeoParquet copy unless \
            `--skip-optimize`, and `--skip-optimize --build-artifact` serves a GeoJSON as chunks. `--function` instead names a function \
            pipeline whose GeoJSON FeatureCollection is the layer, its features cached for `--ttl` (default 60s). Each `--field` is a property \
            the public may see (none: geometry only); `--max-items` caps the features one query answers (default 1000); `--min-zoom` / \
            `--max-zoom` bound visibility; `--style` or the colour flags style tiles; `--filter` is the default filter. \
            Adds `layer: { name, route, store, source, source_kind, …, serving }` and keeps the rest of the payload."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: answer_schema(publish_props),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            name_flag("The layer's name: 1–64 letters, digits, - or _. Publishing a name again updates it."),
            DslFlag { required: true, ..flag("--route", "route", "Where the layer answers, under /ms/{owner}/{project}/.", "text") },
            flag("--from", "from", "The source: a GeoJSON or GeoParquet store key, or a FileRef through {{ }}. Required unless --function.", "file"),
            DslFlag {
                choices: PARSE_WORDS.iter().map(|w| w.to_string()).collect(),
                ..flag("--parse", "parse", "How --from is read: geojson or geoparquet (default: from its extension).", "")
            },
            switch("--bbox-optional", "bbox_optional", "Serve queries without a bbox. Default: a bbox is required."),
            flag("--max-items", "max_items", "The most features one query answers (default 1000, at most 50000).", "number"),
            DslFlag {
                kind: DslFlagKind::RepeatedList,
                max_repeat: Some(MAX_FIELDS),
                ..flag("--field", "field", "A source property the public may see (repeat, or one {{ [list] }}). None: geometry only.", "text")
            },
            flag("--min-zoom", "min_zoom", "The lowest zoom the layer shows at (0–24).", "number"),
            flag("--max-zoom", "max_zoom", "The highest zoom the layer shows at (0–24).", "number"),
            switch("--skip-optimize", "skip_optimize", "Serve the source as it is, without the optimized GeoParquet copy."),
            switch(
                "--build-artifact",
                "build_artifact",
                "With --skip-optimize and a GeoJSON source: build a chunked artifact in the cache tier and serve that instead of the file.",
            ),
            flag("--fill", "fill", "Polygon fill colour (default: rgba(65,105,225,128)).", "text"),
            flag("--stroke", "stroke", "Stroke colour (default: #1E3CA0DC).", "text"),
            flag("--stroke-width", "stroke_width", "Stroke width in pixels, 0–100 (default: 1).", "number"),
            flag("--point-radius", "point_radius", "Point radius in pixels, 0–100 (default: 4).", "number"),
            flag("--point-color", "point_color", "Point fill colour (default: #DC3232C8).", "text"),
            flag("--style", "style", "A style expression, e.g. cb(population,5,YlOrRd); overrides the colour flags.", "text"),
            flag("--filter", "filter", "The default filter, e.g. category:residential;value>100.", "text"),
            flag("--function", "function", "A function pipeline whose GeoJSON FeatureCollection is the layer; instead of --from.", "text"),
            flag("--ttl", "ttl", "How long a function layer's features are cached, e.g. 60s or 5m (default 60s).", "duration"),
            DslFlag { value: "text".to_string(), ..project_store::store_flag() },
        ],
        fields: vec![
            text_field("name", "Name", "The layer's name: letters, digits, - or _."),
            text_field("route", "Route", "Where the layer answers, under /ms/{owner}/{project}/."),
            text_field("from", "From", "A GeoJSON or GeoParquet store key, or a FileRef through {{ }}."),
            NodeFieldDef {
                options: [("", "Auto"), ("geojson", "GeoJSON"), ("geoparquet", "GeoParquet")]
                    .iter()
                    .map(|(value, label)| SelectOptionDef { value: value.to_string(), label: label.to_string() })
                    .collect(),
                ..field("parse", "Parse", NodeFieldType::Select, "How From is read. Auto: from its extension.")
            },
            NodeFieldDef {
                default_value: Some(json!(false)),
                ..field("bbox_optional", "BBox optional", NodeFieldType::Checkbox, "Serve queries without a bbox. Off: a bbox is required.")
            },
            NodeFieldDef {
                default_value: Some(json!(DEFAULT_MAX_ITEMS)),
                ..field("max_items", "Max items", NodeFieldType::Number, "The most features one query answers (at most 50000).")
            },
            text_field("field", "Fields", "A property the public may see, e.g. {{ ['name', 'postcode'] }}. Empty: geometry only."),
            field("min_zoom", "Min zoom", NodeFieldType::Number, "The lowest zoom the layer shows at (0–24)."),
            field("max_zoom", "Max zoom", NodeFieldType::Number, "The highest zoom the layer shows at (0–24)."),
            NodeFieldDef {
                default_value: Some(json!(false)),
                ..field("skip_optimize", "Skip optimization", NodeFieldType::Checkbox, "Serve the source as it is, without the GeoParquet copy.")
            },
            field(
                "build_artifact",
                "Build artifact",
                NodeFieldType::Checkbox,
                "With Skip optimization and a GeoJSON source: serve it as a chunked artifact.",
            ),
            text_field("style", "Style", "A style expression, e.g. cb(population,5,YlOrRd); overrides the colours."),
            text_field("filter", "Filter", "The default filter, e.g. category:residential;value>100."),
            NodeFieldDef {
                data_source: Some(NodeFieldDataSource::FunctionPipelines),
                placeholder: Some("select or type a function pipeline".to_string()),
                ..field("function", "Function", NodeFieldType::Datalist, "A function pipeline returning a GeoJSON FeatureCollection; instead of From.")
            },
            text_field("ttl", "TTL", "How long a function layer's features are cached, e.g. 60s (default)."),
        ],
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("name".to_string()), LayoutItem::Field("route".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("from".to_string()), LayoutItem::Field("parse".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("bbox_optional".to_string()), LayoutItem::Field("max_items".to_string())] },
            LayoutItem::Field("field".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("min_zoom".to_string()), LayoutItem::Field("max_zoom".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("skip_optimize".to_string()), LayoutItem::Field("build_artifact".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("style".to_string()), LayoutItem::Field("filter".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("function".to_string()), LayoutItem::Field("ttl".to_string())] },
        ],
        ai_tool: Default::default(),
        examples: vec![
            NodeExample::dsl("Publish a GeoParquet layer", "ms.layer.publish --name suburbs --route suburbs --from datasets/suburbs.parquet --field name --field postcode --min-zoom 8 --max-zoom 14")
                .output(json!({ "layer": {
                    "name": "suburbs", "route": "suburbs", "store": "local", "source": "mapserver/.optimized/suburbs.spatial.parquet",
                    "source_kind": "geoparquet", "min_zoom": 8, "max_zoom": 14, "bbox_required": true, "max_items": 1000,
                    "fields": ["name", "postcode"], "feature_count": 312, "chunk_count": null, "style": null, "filter": null,
                    "function": null, "ttl": null, "serving": true,
                    "optimization": { "applied": true, "source_format": "parquet", "rows": 312 }
                } }))
                .note("A page loads it with `zeb/deckgl` from `/ms/{owner}/{project}/suburbs` (help topic `guide/mapserver`)."),
            NodeExample::dsl("A layer a function pipeline draws", "ms.layer.publish --name live --route live --function sensors/positions --ttl 30s --bbox-optional"),
        ],
        ..Default::default()
    }
}

pub fn unpublish_definition() -> NodeDefinition {
    NodeDefinition {
        kind: UNPUBLISH_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "MS Unpublish".to_string(),
        description: "Take a published map layer offline: removes `--name` from the project's layer registry so its route stops answering, \
            and deletes its optimized copy and artifact; the source file is left where it is. Adds `layer: { name, removed }`; a name that \
            is not published answers `removed: false` rather than failing."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: answer_schema(json!({ "name": { "type": "string" }, "removed": { "type": "boolean" } })),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![name_flag("The layer to remove.")],
        fields: vec![text_field("name", "Name", "The layer to remove.")],
        layout: vec![LayoutItem::Field("name".to_string())],
        ai_tool: Default::default(),
        examples: vec![
            NodeExample::dsl("Retire a layer", "ms.layer.unpublish --name suburbs").output(json!({ "layer": { "name": "suburbs", "removed": true } })),
        ],
        ..Default::default()
    }
}

pub fn get_definition() -> NodeDefinition {
    let mut props = layer_schema();
    props["found"] = json!({ "type": "boolean", "description": "false: the name is not published, and the rest is absent" });
    NodeDefinition {
        kind: GET_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "MS Get".to_string(),
        description: "Read one published map layer's registry record by `--name`: its route, source, zoom range, fields, style and cache. Adds \
            `layer: { found: true, name, route, store, source, source_kind, … }`, or `layer: { found: false, name }` when the name is not \
            published — branch on `input.layer.found` before `ms.layer.publish` to tell a create from an update."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: answer_schema(props),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![name_flag("The layer to read.")],
        fields: vec![text_field("name", "Name", "The layer to read.")],
        layout: vec![LayoutItem::Field("name".to_string())],
        ai_tool: Default::default(),
        examples: vec![
            NodeExample::dsl("Show a layer's settings", "ms.layer.get --name suburbs")
                .output(json!({ "layer": { "found": true, "name": "suburbs", "route": "suburbs", "source_kind": "geoparquet", "min_zoom": 8, "max_zoom": 14 } })),
        ],
        ..Default::default()
    }
}

pub fn list_definition() -> NodeDefinition {
    NodeDefinition {
        kind: LIST_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "MS List".to_string(),
        description: "List every map layer this project has published, each with the record `ms.layer.get` answers for one. No flags. Adds \
            `layer: { items, count }` — a page reads `input.layer.items`. This is the registry, not the file store: a GeoParquet file \
            nobody published is not in it."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: answer_schema(json!({ "items": { "type": "array" }, "count": { "type": "integer" } })),
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
            NodeExample::dsl("Layers for a map picker", "ms.layer.list")
                .output(json!({ "layer": { "items": [{ "name": "suburbs", "route": "suburbs" }], "count": 1 } })),
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
    pub fn new(config: Config, platform: Arc<PlatformService>, operation: Operation) -> Result<Self, PipelineError> {
        Ok(Self { config, platform, operation })
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

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        let (owner, project, ..) = metadata_scope(&input.metadata)?;
        let layer = match self.operation {
            Operation::Publish => self.exec_publish(owner, project)?,
            Operation::Unpublish => self.exec_unpublish(owner, project)?,
            Operation::Get => self.exec_get(owner, project)?,
            Operation::List => self.exec_list(owner, project)?,
        };
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: with_answer(&input.payload, json!({ "layer": layer })),
            trace: vec![format!("node_kind={}", self.operation.kind())],
        })
    }
}

impl Node {
    fn exec_publish(&self, owner: &str, project: &str) -> Result<Map<String, Value>, PipelineError> {
        let settings = self.config.publish_settings()?;
        let name = require_layer_id(&self.config.name, PUBLISH_CONFIG_CODE)?;
        let path = require_non_empty(&self.config.route, "--route", PUBLISH_CONFIG_CODE)?;
        let is_function_mode = settings.function.is_some();

        // `--from` is a store key, or a FileRef that names its own store.
        let (mut source_path, from_store) = if is_function_mode {
            (String::new(), None)
        } else {
            if crate::pipeline::nodes::shared::file_ref::is_file_ref(&self.config.from) {
                crate::pipeline::nodes::shared::file_ref::validate_file_ref(&self.config.from)?;
            }
            let key = zebfs_rel_path_or_string(&self.config.from)?
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| PipelineError::new(PUBLISH_CONFIG_CODE, "--from is not a store key or a FileRef"))?;
            let store = crate::pipeline::nodes::shared::file_ref::is_file_ref(&self.config.from)
                .then(|| self.config.from.get("store").and_then(Value::as_str).map(str::to_string))
                .flatten();
            (key, store)
        };

        let layout = self.platform.file.ensure_project_layout(owner, project).map_err(|e| PipelineError::new(PUBLISH_CODE, e.to_string()))?;

        // The source is read from its own store (a FileRef's, or this
        // node's); the optimized copy is this node's own object in this
        // node's store, under `mapserver/.optimized/`, which no source may name.
        let store = project_store::open_store(&self.platform, owner, project, self.config.store.as_deref())?;
        let source_store = match from_store.as_deref() {
            Some(id) if id != store.id => project_store::open_store(&self.platform, owner, project, Some(id))?,
            _ => project_store::open_store(&self.platform, owner, project, Some(&store.id))?,
        };
        let mut served_store = source_store.id.clone();
        if !is_function_mode {
            if source_path.trim_start_matches('/').starts_with("mapserver/.optimized/") {
                return Err(PipelineError::new(
                    PUBLISH_CONFIG_CODE,
                    "--from may not name mapserver/.optimized/ — that folder holds the optimized copies this node writes",
                ));
            }
            source_store
                .fs
                .head(&source_path)
                .map_err(|e| PipelineError::new(PUBLISH_CODE, format!("source file not found in store '{}': {e}", source_store.id)))?;
        }
        // Optimized copies are built here and streamed into the store.
        let scratch = StoreScratch::new(PUBLISH_CODE)?;

        let mut artifact_manifest_path: Option<String> = None;
        let mut feature_count: Option<usize> = None;
        let mut chunk_count: Option<usize> = None;
        let mut optimization_info: Option<Value> = None;

        // ── Auto-detect format and optimize ──────────────────────────────
        let source_abs = if is_function_mode {
            std::path::PathBuf::new()
        } else {
            self.platform
                .file
                .object_local_path(owner, project, Some(&source_store.id), &source_path)
                .map_err(|e| PipelineError::new(PUBLISH_CODE, e.message))?
        };
        let source_lower = source_path.to_ascii_lowercase();

        let should_optimize = !self.config.skip_optimize && !is_function_mode;

        let effective_kind = if is_function_mode {
            "geojson_function".to_string()
        } else if should_optimize {
            // `--parse`, else the file's extension.
            let is_geojson = settings.parse == Some("geojson")
                || (settings.parse.is_none() && (source_lower.ends_with(".geojson") || source_lower.ends_with(".json")));
            let is_parquet = settings.parse == Some("geoparquet")
                || (settings.parse.is_none() && (source_lower.ends_with(".parquet") || source_lower.ends_with(".pq")));

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
                    let optimized_dir = scratch.path().to_path_buf();
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
                                PUBLISH_CODE,
                                format!("optimize failed: {e}"),
                            )
                        })?;

                    // Write column stats sidecar JSON
                    let stats_filename = format!("{name}.spatial.stats.json");
                    let stats_rel = format!("mapserver/.optimized/{stats_filename}");
                    let stats_report =
                        crate::mapserver::resolve::geoparquet_optimize::ColumnStatsReport {
                            row_count: report.rows,
                            columns: report.column_stats.clone(),
                        };
                    store_optimized(&store, &optimized_abs, &optimized_rel, &stats_report, &stats_rel)?;

                    eprintln!(
                        "geoparquet optimized: {} rows, {} row groups, {:.1}MB → {:.1}MB, {} column stats",
                        report.rows,
                        report.row_groups,
                        report.source_bytes as f64 / 1_048_576.0,
                        report.dest_bytes as f64 / 1_048_576.0,
                        report.column_stats.len(),
                    );

                    source_path = optimized_rel;
                served_store = store.id.clone();
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
                let optimized_dir = scratch.path().to_path_buf();

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
                            PUBLISH_CODE,
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
                        PipelineError::new(PUBLISH_CODE, format!("optimize failed: {e}"))
                    })?;

                // Clean up intermediate raw parquet
                let _ = std::fs::remove_file(&temp_raw);

                // Write column stats sidecar JSON
                let stats_filename = format!("{name}.spatial.stats.json");
                let stats_rel = format!("mapserver/.optimized/{stats_filename}");
                let stats_report =
                    crate::mapserver::resolve::geoparquet_optimize::ColumnStatsReport {
                        row_count: opt_report.rows,
                        columns: opt_report.column_stats.clone(),
                    };
                store_optimized(&store, &optimized_abs, &optimized_rel, &stats_report, &stats_rel)?;

                eprintln!(
                    "geoparquet optimized: {} rows, {} row groups, {:.1}MB, {} column stats",
                    opt_report.rows,
                    opt_report.row_groups,
                    opt_report.dest_bytes as f64 / 1_048_576.0,
                    opt_report.column_stats.len(),
                );

                feature_count = Some(convert_report.feature_count);
                source_path = optimized_rel;
                served_store = store.id.clone();
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
                    PUBLISH_CONFIG_CODE,
                    format!(
                        "cannot tell the format of '{}' from its name; \
                         set --parse geojson|geoparquet, or --skip-optimize to serve it as it is",
                        source_path
                    ),
                ));
            }
        } else {
            // --skip-optimize: the source is served as it is — `--parse`,
            // else its extension, says which reader serves it.
            let parquet = match settings.parse {
                Some(word) => word == "geoparquet",
                None => source_lower.ends_with(".parquet") || source_lower.ends_with(".pq"),
            };
            if parquet && self.config.build_artifact {
                return Err(PipelineError::new(PUBLISH_CONFIG_CODE, "--build-artifact chunks a GeoJSON source; this one is GeoParquet"));
            }
            let source_kind = if parquet { "geoparquet" } else { "geojson_file" }.to_string();

            // --build-artifact: the GeoJSON is served as chunks built in
            // the cache tier.
            if source_kind == "geojson_file" && self.config.build_artifact {
                // Artifacts live in the cache tier; a pre-tier
                // `files/mapserver/.artifacts` tree migrates on first touch.
                let artifact_home = layout.ensure_mapserver_artifacts_home().map_err(|e| {
                    PipelineError::new(
                        PUBLISH_CODE,
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
                .map_err(|e| PipelineError::new(PUBLISH_CODE, e))?;

                artifact_manifest_path = Some(build_out.manifest_rel_path);
                feature_count = Some(build_out.feature_count);
                chunk_count = Some(build_out.chunk_count);
                "geojson_artifact".to_string()
            } else {
                source_kind
            }
        };

        // --style wins over the colour flags.
        let style = if let Some(dsl) = self.config.style.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            crate::mapserver::resolve::style_dsl::parse_style_dsl(dsl)
                .map_err(|e| PipelineError::new(PUBLISH_STYLE_CODE, format!("invalid --style: {e}")))?;
            Some(json!(dsl))
        } else {
            let mut obj = Map::new();
            if let Some(v) = self.config.fill.as_deref().filter(|v| !v.trim().is_empty()) {
                obj.insert("fill".into(), json!(v));
            }
            if let Some(v) = self.config.stroke.as_deref().filter(|v| !v.trim().is_empty()) {
                obj.insert("stroke".into(), json!(v));
            }
            if let Some(v) = settings.stroke_width {
                obj.insert("stroke_width".into(), json!(v as f32));
            }
            if let Some(v) = settings.point_radius {
                obj.insert("point_radius".into(), json!(v as f32));
            }
            if let Some(v) = self.config.point_color.as_deref().filter(|v| !v.trim().is_empty()) {
                obj.insert("point_color".into(), json!(v));
            }
            if obj.is_empty() { None } else { Some(Value::Object(obj)) }
        };

        let filter = match self.config.filter.as_deref().map(str::trim).filter(|f| !f.is_empty()) {
            Some(f) => {
                crate::mapserver::resolve::filter_dsl::parse_filter(f)
                    .map_err(|e| PipelineError::new(PUBLISH_FILTER_CODE, format!("invalid --filter: {e}")))?;
                Some(f.to_string())
            }
            None => None,
        };

        let record = LayerRecord {
            layer_id: name.to_string(),
            store: served_store,
            path: registry::normalize_layer_path(path).to_string(),
            source_path: source_path.clone(),
            source_kind: effective_kind,
            artifact_manifest_path,
            mode: "features".to_string(),
            min_zoom: settings.min_zoom,
            max_zoom: settings.max_zoom,
            bbox_required: !self.config.bbox_optional,
            max_features: settings.max_items,
            allowed_properties: settings.fields.clone(),
            feature_count,
            chunk_count,
            style,
            filter,
            function_slug: settings.function.clone(),
            cache_ttl_secs: settings.ttl_secs,
        };

        let mut layers = read_layers(&self.platform, owner, project, PUBLISH_CODE)?;
        match layers.iter().position(|l| l.layer_id == name) {
            Some(pos) => layers[pos] = record.clone(),
            None => layers.push(record.clone()),
        }
        write_layers(&self.platform, owner, project, &layers, PUBLISH_CODE)?;

        // A layer answers only while the project's `ms` surface is on; say
        // so rather than imply it is already reachable.
        let serving = self
            .platform
            .addressing
            .read(owner, project)
            .map(|a| a.is_enabled(crate::platform::services::addressing::Surface::Ms))
            .unwrap_or(false);
        let mut layer = layer_to_json(&record);
        layer.insert("serving".to_string(), json!(serving));
        if !serving {
            layer.insert(
                "note".to_string(),
                json!("the ms surface is off for this project; switch it on in Settings → Addressing to serve this layer"),
            );
        }
        if let Some(opt) = optimization_info {
            layer.insert("optimization".to_string(), opt);
        }
        Ok(layer)
    }

    fn exec_unpublish(&self, owner: &str, project: &str) -> Result<Map<String, Value>, PipelineError> {
        let name = require_layer_id(&self.config.name, UNPUBLISH_CONFIG_CODE)?;
        let mut layers = read_layers(&self.platform, owner, project, UNPUBLISH_CODE)?;
        let removed_record = layers.iter().find(|l| l.layer_id == name).cloned();
        layers.retain(|l| l.layer_id != name);
        let removed = removed_record.is_some();
        write_layers(&self.platform, owner, project, &layers, UNPUBLISH_CODE)?;

        // Clean up the artifact directory and the optimized copy.
        if let Some(record) = removed_record.as_ref()
            && let Ok(layout) = self.platform.file.ensure_project_layout(owner, project)
        {
            // Move a pre-tier artifact tree to its cache home first so the
            // cleanup hits the tree wherever it actually lives.
            let artifact_home = layout.ensure_mapserver_artifacts_home().ok();
            if let Some(dir) = record
                .artifact_manifest_path
                .as_deref()
                .and_then(|rel| layout.resolve_mapserver_artifact_path(rel))
                .as_deref()
                .and_then(std::path::Path::parent)
            {
                let _ = std::fs::remove_dir_all(dir);
            }
            if let Some(artifact_dir) = artifact_home.map(|home| home.join(DEFAULT_INSTANCE).join(name))
                && artifact_dir.exists()
            {
                let _ = std::fs::remove_dir_all(&artifact_dir);
            }

            // The optimized copy is the node's own object in the store the
            // layer was published from; the source is left where it is.
            if let Ok(store) = project_store::open_store(&self.platform, owner, project, Some(&record.store)) {
                for key in [format!("mapserver/.optimized/{name}.spatial.parquet"), format!("mapserver/.optimized/{name}.spatial.stats.json")] {
                    if store.fs.head(&key).is_ok() {
                        store.delete_named(&self.platform, owner, project, &key, UNPUBLISH_CODE)?;
                    }
                }
            }
        }

        let mut layer = Map::new();
        layer.insert("name".to_string(), json!(name));
        layer.insert("removed".to_string(), json!(removed));
        Ok(layer)
    }

    fn exec_get(&self, owner: &str, project: &str) -> Result<Map<String, Value>, PipelineError> {
        let name = require_layer_id(&self.config.name, GET_CONFIG_CODE)?;
        let layers = read_layers(&self.platform, owner, project, GET_CODE)?;
        let mut layer = Map::new();
        match layers.iter().find(|l| l.layer_id == name) {
            Some(record) => {
                layer.insert("found".to_string(), json!(true));
                layer.extend(layer_to_json(record));
            }
            None => {
                layer.insert("found".to_string(), json!(false));
                layer.insert("name".to_string(), json!(name));
            }
        }
        Ok(layer)
    }

    fn exec_list(&self, owner: &str, project: &str) -> Result<Map<String, Value>, PipelineError> {
        let items: Vec<Value> = read_layers(&self.platform, owner, project, LIST_CODE)?
            .iter()
            .map(|record| Value::Object(layer_to_json(record)))
            .collect();
        let mut layer = Map::new();
        layer.insert("count".to_string(), json!(items.len()));
        layer.insert("items".to_string(), Value::Array(items));
        Ok(layer)
    }
}

/// `--name` is a folder and file name in the cache and the store.
fn require_layer_id<'a>(value: &'a str, code: &'static str) -> Result<&'a str, PipelineError> {
    let name = require_non_empty(value, "--name", code)?;
    if !crate::contracts::kinds::valid_layer_id(name) {
        return Err(PipelineError::new(code, format!("--name '{name}' must be 1–64 letters, digits, '-' or '_'")));
    }
    Ok(name)
}

fn require_non_empty<'a>(value: &'a str, flag: &str, code: &'static str) -> Result<&'a str, PipelineError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(PipelineError::new(code, format!("{flag} is required")));
    }
    Ok(trimmed)
}

#[cfg(test)]
mod tests;

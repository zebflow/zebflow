//! `geo.dataset.inspect` — what a spatial dataset holds: its format, and per
//! layer its feature count, geometry, CRS, declared extent and fields.
//!
//! Delegates to `geonative_convert::inspect()`. Reads `.gdb`, `.shp`,
//! `.parquet` and `.geojson`. The answer is one key, `dataset`.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::model::NodeCapability;
use crate::pipeline::nodes::shared::project_store::{open_from, store_flag};
use crate::pipeline::nodes::shared::store_scratch::StoreScratch;
use crate::pipeline::nodes::shared::util::{metadata_scope, with_answer};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeExample, NodeFieldDef, NodeFieldType},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

pub const NODE_KIND: &str = "geo.dataset.inspect";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

/// Reading or inspecting the file failed.
pub const CODE: &str = "FW_NODE_GEO_DATASET_INSPECT";
/// A flag the author set wrong, or a `--layer` the file does not hold.
pub const CONFIG_CODE: &str = "FW_NODE_GEO_DATASET_INSPECT_CONFIG";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// The file to inspect: a store key, or a FileRef through `{{ }}`.
    #[serde(default)]
    pub from: Value,
    /// Report this layer only.
    #[serde(default)]
    pub layer: String,
    /// The store a bare key is read from; saved explicitly at registration.
    #[serde(default)]
    pub store: Option<String>,
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "Geo Inspect".to_string(),
        description: "Inspects a spatial dataset (.gdb, .shp, .parquet, .geojson) and lists its layers. The dataset is given as a store key or a FileRef (read from the store it \
            names). Adds `dataset: { source, store, format, layers }`, keeping the rest of the payload; each layer is `{ name, feature_count, \
            geometry: { field_name, kind, has_z, has_m, declared_extent }, crs, fields: [{ name, type, nullable, width }] }`. `--layer` reports \
            that layer only. Read `input.dataset.layers[0].crs` before choosing `geo.dataset.convert --crs`."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "dataset": {
                    "type": "object",
                    "properties": {
                        "source": { "type": "string", "description": "The store key inspected" },
                        "store": { "type": "string" },
                        "format": { "type": "string", "description": "filegdb, shapefile, geoparquet or geojson" },
                        "layers": { "type": "array", "description": "One entry per layer: name, feature_count, geometry, crs, fields" }
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
                description: "The spatial file: a store key, or a FileRef through {{ }}.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
                value: "file".to_string(),
                ..Default::default()
            },
            DslFlag {
                flag: "--layer".to_string(),
                config_key: "layer".to_string(),
                description: "Report this layer only (a FileGDB holds several; other formats one, named default).".to_string(),
                kind: DslFlagKind::Scalar,
                value: "text".to_string(),
                ..Default::default()
            },
            DslFlag { value: "text".to_string(), ..store_flag() },
        ],
        fields: vec![
            NodeFieldDef {
                name: "from".to_string(),
                label: "From".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("The spatial file: a store key, or a FileRef through {{ }}.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "layer".to_string(),
                label: "Layer".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Report this layer only. Empty: every layer.".to_string()),
                ..Default::default()
            },
        ],
        layout: vec![LayoutItem::Col { col: vec![LayoutItem::Field("from".to_string()), LayoutItem::Field("layer".to_string())] }],
        ai_tool: Default::default(),
        examples: vec![
            NodeExample::dsl("What is in this file?", "geo.dataset.inspect --from uploads/suburbs.shp")
                .output(json!({ "dataset": { "source": "uploads/suburbs.shp", "store": "local", "format": "shapefile", "layers": [{
                    "name": "default", "feature_count": 312,
                    "geometry": { "field_name": "geometry", "kind": "Polygon", "has_z": false, "has_m": false, "declared_extent": null },
                    "crs": { "kind": "epsg", "code": 7844 },
                    "fields": [{ "name": "name", "type": "String", "nullable": true, "width": 80 }]
                }] } }))
                .note("`input.dataset.layers[0].crs.code` is 7844; `geo.dataset.convert --crs EPSG:4326` reprojects."),
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

        // GDAL reads paths and the project's files live in their stores: the
        // file (and its sidecars) is pulled into a scratch folder from the
        // store that holds it.
        let (source_store, rel_path) =
            open_from(&self.platform, owner, project, &self.config.from, self.config.store.as_deref(), "--from", CONFIG_CODE)?;
        let scratch = StoreScratch::new(CODE)?;
        let abs_path = scratch
            .pull_with_siblings(&source_store.fs, &rel_path)
            .map_err(|_| PipelineError::new(CODE, format!("file not found: {rel_path}")))?;

        let report = tokio::task::spawn_blocking(move || geonative_convert::inspect(&abs_path))
            .await
            .map_err(|err| PipelineError::new(CODE, format!("inspect task panicked: {err}")))?
            .map_err(|err| PipelineError::new(CODE, err.to_string()))?;
        let layers = serde_json::to_value(&report.layers).map_err(|err| PipelineError::new(CODE, format!("serialising report: {err}")))?;
        let layers = only_layer(layers, &self.config.layer)?;

        let dataset = json!({ "source": rel_path, "store": source_store.id, "format": report.format, "layers": layers });
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: with_answer(&input.payload, json!({ "dataset": dataset })),
            trace: vec![format!("node_kind={NODE_KIND} path={rel_path}")],
        })
    }
}

/// Every layer, or with `--layer` that one; a name the file does not hold
/// is refused, naming the ones it does.
fn only_layer(layers: Value, wanted: &str) -> Result<Value, PipelineError> {
    let wanted = wanted.trim();
    if wanted.is_empty() {
        return Ok(layers);
    }
    let all = layers.as_array().cloned().unwrap_or_default();
    let name = |layer: &Value| layer.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
    match all.iter().find(|layer| name(layer) == wanted) {
        Some(layer) => Ok(json!([layer])),
        None => Err(PipelineError::new(
            CONFIG_CODE,
            format!("--layer '{wanted}' is not in the file; it holds {}", all.iter().map(name).collect::<Vec<_>>().join(", ")),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_narrows_the_report_or_names_what_is_there() {
        let layers = json!([{ "name": "roads" }, { "name": "parcels" }]);
        assert_eq!(only_layer(layers.clone(), "").unwrap(), layers);
        assert_eq!(only_layer(layers.clone(), "parcels").unwrap(), json!([{ "name": "parcels" }]));
        let err = only_layer(layers, "rivers").unwrap_err();
        assert_eq!(err.code, CONFIG_CODE);
        assert!(err.message.contains("roads, parcels"), "{}", err.message);
    }
}

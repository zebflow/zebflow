//! n.geo.inspect — report schema, CRS, extent, and fields of a spatial dataset.
//!
//! Delegates to `geonative_convert::inspect()`. Supports `.gdb`, `.shp`,
//! `.parquet`, and `.geojson` inputs.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::pipeline::nodes::shared::project_store::open_source;
use crate::pipeline::nodes::shared::store_scratch::StoreScratch;
use crate::pipeline::nodes::shared::util::metadata_scope;
use crate::pipeline::model::NodeCapability;
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

pub const NODE_KIND: &str = "n.geo.inspect";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// The file to inspect: a store key, or a FileRef through `{{ }}`.
    #[serde(default)]
    pub from: serde_json::Value,
    #[serde(default)]
    pub layer: String,
    /// The store a bare key is read from (default: the project's default).
    #[serde(default)]
    pub store: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            from: serde_json::Value::Null,
            layer: String::new(),
            store: None,
        }
    }
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "Geo Inspect".to_string(),
        description: "Inspect a spatial dataset and return its schema, CRS, geometry type, \
            declared extent, and field definitions. Supports .gdb, .shp, .parquet, .geojson."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "description": "Payload optionally contains a project-relative path at the configured path_expr key."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "inspect": {
                    "type": "object",
                    "description": "DatasetInspection report from geonative"
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
                description: "The spatial file: a store key, or a FileRef through {{ }}".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--layer".to_string(),
                config_key: "layer".to_string(),
                description: "Layer name for multi-layer sources (e.g. FileGDB)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            crate::pipeline::nodes::shared::project_store::store_flag(),
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
                help: Some("Layer name for multi-layer FileGDB sources.".to_string()),
                ..Default::default()
            },
        ],
        layout: vec![LayoutItem::Col {
            col: vec![
                LayoutItem::Field("from".to_string()),
                LayoutItem::Field("layer".to_string()),
            ],
        }],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("What is in this file?", "geo.inspect --from uploads/suburbs.zip")
                .output(serde_json::json!({ "inspect": { "report": { "driver": "ESRI Shapefile", "layers": [{ "name": "suburbs", "geometry": "Polygon", "crs": "EPSG:7844", "features": 312, "fields": ["name", "postcode"] }] }, "source": "uploads/suburbs.zip", "store": "local" } }))
                .note("Read `input.inspect.layers[0].crs` before deciding on `geo.convert --to-crs`."),
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

        // GDAL reads paths and the project's files live in their stores: the
        // file (and its sidecars) is pulled into a scratch folder from the
        // store that holds it.
        let (source_store, rel_path) = open_source(&self.platform, owner, project, &self.config.from, self.config.store.as_deref())?
            .ok_or_else(|| PipelineError::new("FW_NODE_GEO_INSPECT", "no input configured — set --from to a store key or a FileRef"))?;
        let rel_path = sanitize_rel_path(&rel_path);
        let zebfs = &source_store.fs;
        let scratch = StoreScratch::new("FW_NODE_GEO_INSPECT")?;
        let abs_path = scratch.pull_with_siblings(&zebfs, &rel_path).map_err(|_| {
            PipelineError::new(
                "FW_NODE_GEO_INSPECT",
                format!("file not found: {rel_path}"),
            )
        })?;

        let path_for_task = abs_path.clone();
        let report =
            tokio::task::spawn_blocking(move || geonative_convert::inspect(&path_for_task))
                .await
                .map_err(|err| {
                    PipelineError::new(
                        "FW_NODE_GEO_INSPECT",
                        format!("inspect task panicked: {err}"),
                    )
                })?
                .map_err(|err| PipelineError::new("FW_NODE_GEO_INSPECT", err.to_string()))?;

        let report_json = serde_json::to_value(&report).map_err(|err| {
            PipelineError::new("FW_NODE_GEO_INSPECT", format!("serialising report: {err}"))
        })?;

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: {
                let mut payload = match &input.payload {
                    serde_json::Value::Object(map) => map.clone(),
                    _ => serde_json::Map::new(),
                };
                payload.insert("inspect".to_string(), json!({ "report": report_json, "source": rel_path, "store": source_store.id }));
                serde_json::Value::Object(payload)
            },
            trace: vec![format!("node_kind={NODE_KIND} path={rel_path}")],
        })
    }
}


fn sanitize_rel_path(path: &str) -> String {
    path.split('/')
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .collect::<Vec<_>>()
        .join("/")
}

//! `n.trigger.mapserver` — direct spatial feature endpoint.
//!
//! Specialized trigger that owns an HTTP map-facing route and resolves a
//! published spatial source directly without requiring `n.web.response`.

use crate::pipeline::model::{
    DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType, SelectOptionDef,
};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub const NODE_KIND: &str = "n.trigger.mapserver";
pub const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Mapserver Trigger".to_string(),
        description: "Serve spatial feature queries directly over HTTP. \
            V1 supports GeoJSON file sources and bbox-filtered FeatureCollection responses."
            .to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "description": "Normalized spatial request payload for mapserver routes."
        }),
        output_schema: serde_json::json!({
            "type": "object",
            "description": "Passthrough spatial request payload."
        }),
        input_pins: vec![],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: serde_json::json!({
            "type": "object",
            "required": ["path", "source_path"],
            "properties": {
                "path": { "type": "string", "description": "Relative mapserver path under /ms/{owner}/{project}." },
                "mode": { "type": "string", "enum": ["features"], "description": "Serving mode. V1 only supports features." },
                "source_kind": { "type": "string", "enum": ["geojson_file"], "description": "Backing source kind. V1 only supports geojson_file." },
                "source_path": { "type": "string", "description": "Absolute or project-resolved GeoJSON file path." },
                "bbox_required": { "type": "boolean", "description": "Require bbox query parameter before serving data." },
                "min_zoom": { "type": "integer", "minimum": 0, "maximum": 30, "description": "Minimum zoom where this layer is visible." },
                "max_zoom": { "type": "integer", "minimum": 0, "maximum": 30, "description": "Maximum zoom where this layer is visible." },
                "max_features": { "type": "integer", "minimum": 1, "description": "Hard feature cap per request." },
                "allowed_properties": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Property whitelist. Empty returns all properties."
                }
            }
        }),
        fields: vec![
            NodeFieldDef {
                name: "title".to_string(),
                label: "Title".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Override display title for this node.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "path".to_string(),
                label: "Path".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Mapserver relative path under /ms/{owner}/{project}.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "mode".to_string(),
                label: "Mode".to_string(),
                field_type: NodeFieldType::Select,
                options: vec![SelectOptionDef {
                    value: "features".to_string(),
                    label: "Features".to_string(),
                }],
                help: Some("Serving mode. V1 only supports feature queries.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "source_kind".to_string(),
                label: "Source Kind".to_string(),
                field_type: NodeFieldType::Select,
                options: vec![SelectOptionDef {
                    value: "geojson_file".to_string(),
                    label: "GeoJSON File".to_string(),
                }],
                help: Some("Backing source kind.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "source_path".to_string(),
                label: "Source Path".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("GeoJSON file path. V1 expects a FeatureCollection.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "bbox_required".to_string(),
                label: "BBox Required".to_string(),
                field_type: NodeFieldType::Checkbox,
                help: Some("Reject requests without bbox.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "min_zoom".to_string(),
                label: "Min Zoom".to_string(),
                field_type: NodeFieldType::Number,
                help: Some("Hide this layer below the given zoom.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "max_zoom".to_string(),
                label: "Max Zoom".to_string(),
                field_type: NodeFieldType::Number,
                help: Some("Hide this layer above the given zoom.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "max_features".to_string(),
                label: "Max Features".to_string(),
                field_type: NodeFieldType::Number,
                help: Some("Hard cap per request.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "allowed_properties_csv".to_string(),
                label: "Allowed Properties".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Comma-separated property whitelist.".to_string()),
                ..Default::default()
            },
        ],
        dsl_flags: vec![
            DslFlag {
                flag: "--path".to_string(),
                config_key: "path".to_string(),
                description: "Relative mapserver path under /ms/{owner}/{project}.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--source-path".to_string(),
                config_key: "source_path".to_string(),
                description: "GeoJSON source path.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--mode".to_string(),
                config_key: "mode".to_string(),
                description: "Serving mode. V1 only supports features.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--bbox-required".to_string(),
                config_key: "bbox_required".to_string(),
                description: "Require bbox query parameter.".to_string(),
                kind: DslFlagKind::Bool,
                required: false,
            },
            DslFlag {
                flag: "--min-zoom".to_string(),
                config_key: "min_zoom".to_string(),
                description: "Minimum zoom where this layer is visible.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--max-zoom".to_string(),
                config_key: "max_zoom".to_string(),
                description: "Maximum zoom where this layer is visible.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--max-features".to_string(),
                config_key: "max_features".to_string(),
                description: "Hard feature cap per request.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--allowed-properties".to_string(),
                config_key: "allowed_properties".to_string(),
                description: "Comma-separated property whitelist.".to_string(),
                kind: DslFlagKind::CommaSeparatedList,
                required: false,
            },
        ],
        layout: vec![
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("title".to_string()),
                    LayoutItem::Field("path".to_string()),
                ],
            },
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("mode".to_string()),
                    LayoutItem::Field("source_kind".to_string()),
                ],
            },
            LayoutItem::Field("source_path".to_string()),
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("bbox_required".to_string()),
                    LayoutItem::Field("max_features".to_string()),
                ],
            },
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("min_zoom".to_string()),
                    LayoutItem::Field("max_zoom".to_string()),
                ],
            },
            LayoutItem::Field("allowed_properties_csv".to_string()),
        ],
        ai_tool: Default::default(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub path: String,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_source_kind")]
    pub source_kind: String,
    pub source_path: String,
    #[serde(default = "default_bbox_required")]
    pub bbox_required: bool,
    #[serde(default)]
    pub min_zoom: Option<u8>,
    #[serde(default)]
    pub max_zoom: Option<u8>,
    #[serde(default = "default_max_features")]
    pub max_features: usize,
    #[serde(default)]
    pub allowed_properties: Vec<String>,
    #[serde(default)]
    pub allowed_properties_csv: String,
}

fn default_mode() -> String {
    "features".to_string()
}

fn default_source_kind() -> String {
    "geojson_file".to_string()
}

fn default_bbox_required() -> bool {
    true
}

fn default_max_features() -> usize {
    1000
}

pub struct Node {
    config: Config,
}

impl Node {
    pub fn new(mut config: Config) -> Self {
        if config.allowed_properties.is_empty() && !config.allowed_properties_csv.trim().is_empty()
        {
            config.allowed_properties = config
                .allowed_properties_csv
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
                .collect();
        }
        Self { config }
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        NODE_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }
    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        Ok(NodeExecutionOutput {
            payload: input.payload,
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            trace: vec![
                format!("node_kind={NODE_KIND}"),
                format!("path={}", self.config.path),
            ],
        })
    }
}

//! n.geo.inspect — report schema, CRS, extent, and fields of a spatial dataset.
//!
//! Delegates to `geonative_convert::inspect()`. Supports `.gdb`, `.shp`,
//! `.parquet`, and `.geojson` inputs.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::util::{metadata_scope, resolve_path};
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
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub path_expr: String,
    #[serde(default)]
    pub layer: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            path: String::new(),
            path_expr: String::new(),
            layer: String::new(),
        }
    }
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
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
                flag: "--path".to_string(),
                config_key: "path".to_string(),
                description: "Project-relative path to the spatial file".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--path-expr".to_string(),
                config_key: "path_expr".to_string(),
                description: "Dot-path into upstream payload resolving to the file path"
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--layer".to_string(),
                config_key: "layer".to_string(),
                description: "Layer name for multi-layer sources (e.g. FileGDB)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "path".to_string(),
                label: "Path".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Project-relative path to the spatial file.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "path_expr".to_string(),
                label: "Path Expression".to_string(),
                field_type: NodeFieldType::Text,
                help: Some(
                    "Dot-path into upstream payload resolving to the file path.".to_string(),
                ),
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
                LayoutItem::Field("path".to_string()),
                LayoutItem::Field("path_expr".to_string()),
                LayoutItem::Field("layer".to_string()),
            ],
        }],
        ai_tool: Default::default(),
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

        let rel_path = resolve_input_path(&self.config, &input.payload)?;

        let layout = self
            .platform
            .file
            .ensure_project_layout(owner, project)
            .map_err(|err| PipelineError::new("FW_NODE_GEO_INSPECT", err.to_string()))?;

        let abs_path = layout.files_dir.join(&rel_path);
        if !abs_path.exists() {
            return Err(PipelineError::new(
                "FW_NODE_GEO_INSPECT",
                format!("file not found: {rel_path}"),
            ));
        }

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
            payload: json!({
                "inspect": report_json,
                "source": rel_path,
            }),
            trace: vec![format!("node_kind={NODE_KIND} path={rel_path}")],
        })
    }
}

fn resolve_input_path(
    config: &Config,
    payload: &serde_json::Value,
) -> Result<String, PipelineError> {
    // 1. Static --path takes priority.
    if !config.path.trim().is_empty() {
        return Ok(sanitize_rel_path(config.path.trim()));
    }
    // 2. Dynamic --path-expr resolves from upstream payload.
    if !config.path_expr.trim().is_empty() {
        let val = resolve_path(payload, config.path_expr.trim())
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_GEO_INSPECT",
                    format!(
                        "path not found at payload key '{}' — set --path or --path-expr",
                        config.path_expr
                    ),
                )
            })?;
        return Ok(sanitize_rel_path(val));
    }
    Err(PipelineError::new(
        "FW_NODE_GEO_INSPECT",
        "no input path configured — set --path or --path-expr",
    ))
}

fn sanitize_rel_path(path: &str) -> String {
    path.split('/')
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .collect::<Vec<_>>()
        .join("/")
}

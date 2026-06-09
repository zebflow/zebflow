//! n.geo.convert — convert a spatial dataset between formats.
//!
//! Delegates to `geonative_convert::convert()`. Reads `.gdb`, `.shp`,
//! `.parquet`, `.geojson`; writes `.parquet` or `.geojson`. Optionally
//! reprojects mid-stream via `--to-crs`.

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

pub const NODE_KIND: &str = "n.geo.convert";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

fn default_batch_size() -> usize {
    10_000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub input: String,
    #[serde(default)]
    pub input_expr: String,
    #[serde(default)]
    pub output: String,
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
            input: String::new(),
            input_expr: String::new(),
            output: String::new(),
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
                flag: "--input".to_string(),
                config_key: "input".to_string(),
                description: "Project-relative path to the input spatial file".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--input-expr".to_string(),
                config_key: "input_expr".to_string(),
                description: "Dot-path into upstream payload resolving to the input path"
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--output".to_string(),
                config_key: "output".to_string(),
                description: "Project-relative path for the output file (.parquet or .geojson)"
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
            DslFlag {
                flag: "--to-crs".to_string(),
                config_key: "to_crs".to_string(),
                description: "Target CRS for mid-stream reprojection (e.g. EPSG:4326 or 4326)"
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--hilbert".to_string(),
                config_key: "hilbert".to_string(),
                description: "Hilbert-sort output by bbox centroid (parquet only)".to_string(),
                kind: DslFlagKind::Bool,
                required: false,
            },
            DslFlag {
                flag: "--batch-size".to_string(),
                config_key: "batch_size".to_string(),
                description: "Rows per parquet row group (default: 10000)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "input".to_string(),
                label: "Input".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Project-relative path to the input spatial file.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "input_expr".to_string(),
                label: "Input Expression".to_string(),
                field_type: NodeFieldType::Text,
                help: Some(
                    "Dot-path into upstream payload resolving to the input path.".to_string(),
                ),
                ..Default::default()
            },
            NodeFieldDef {
                name: "output".to_string(),
                label: "Output".to_string(),
                field_type: NodeFieldType::Text,
                help: Some(
                    "Project-relative path for the output file (.parquet or .geojson).".to_string(),
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
        ],
        layout: vec![
            LayoutItem::Col {
                col: vec![
                    LayoutItem::Field("input".to_string()),
                    LayoutItem::Field("input_expr".to_string()),
                    LayoutItem::Field("output".to_string()),
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
        ..Default::default()
    }
}

pub struct Node {
    config: Config,
    platform: Arc<PlatformService>,
}

impl Node {
    pub fn new(config: Config, platform: Arc<PlatformService>) -> Result<Self, PipelineError> {
        if config.output.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_GEO_CONVERT",
                "output path is required — set --output",
            ));
        }
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

        let input_rel = resolve_input_path(&self.config, &input.payload)?;
        let output_rel = sanitize_rel_path(self.config.output.trim());

        if output_rel.is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_GEO_CONVERT",
                "output path is empty after sanitization",
            ));
        }

        let layout = self
            .platform
            .file
            .ensure_project_layout(owner, project)
            .map_err(|err| PipelineError::new("FW_NODE_GEO_CONVERT", err.to_string()))?;

        let input_abs = layout.files_dir.join(&input_rel);
        if !input_abs.exists() {
            return Err(PipelineError::new(
                "FW_NODE_GEO_CONVERT",
                format!("input file not found: {input_rel}"),
            ));
        }

        let output_abs = layout.files_dir.join(&output_rel);
        if let Some(parent) = output_abs.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                PipelineError::new(
                    "FW_NODE_GEO_CONVERT",
                    format!("creating output directory: {err}"),
                )
            })?;
        }

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

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({
                "converted": {
                    "input": input_rel,
                    "output": output_rel,
                    "features": stats.features,
                    "output_bytes": stats.output_bytes,
                    "elapsed_secs": stats.elapsed_secs,
                }
            }),
            trace: vec![format!(
                "node_kind={NODE_KIND} input={input_rel} output={output_rel} features={}",
                stats.features
            )],
        })
    }
}

fn resolve_input_path(
    config: &Config,
    payload: &serde_json::Value,
) -> Result<String, PipelineError> {
    if !config.input.trim().is_empty() {
        return Ok(sanitize_rel_path(config.input.trim()));
    }
    if !config.input_expr.trim().is_empty() {
        let val = resolve_path(payload, config.input_expr.trim())
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_GEO_CONVERT",
                    format!(
                        "input path not found at payload key '{}' — set --input or --input-expr",
                        config.input_expr
                    ),
                )
            })?;
        return Ok(sanitize_rel_path(val));
    }
    Err(PipelineError::new(
        "FW_NODE_GEO_CONVERT",
        "no input path configured — set --input or --input-expr",
    ))
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

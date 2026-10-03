//! n.fs.barcode.code128 — a linear barcode for printable ASCII text (a
//! ticket, a certificate or order number), stored as SVG or PNG. The encoder
//! is ours (`encode`, ISO/IEC 15417). The payload gains `barcode`: the stored
//! file's FileRef plus its size, and for SVG the markup itself.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::render::{Drawing, parse_colour};
use super::{field, flag, select};
use crate::pipeline::model::NodeCapability;
use crate::pipeline::nodes::shared::util::metadata_scope;
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlagKind, LayoutItem},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

pub mod encode;

pub const NODE_KIND: &str = "n.fs.barcode.code128";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";
const ERR: &str = "FS_BARCODE_CODE128";

fn default_format() -> String {
    "svg".into()
}
fn default_size() -> u32 {
    300
}
fn default_height() -> u32 {
    80
}
fn default_margin() -> usize {
    10
}
fn default_color() -> String {
    "#000000".into()
}
fn default_background() -> String {
    "#ffffff".into()
}
fn default_folder() -> String {
    "barcodes".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// What the barcode says: printable ASCII only.
    #[serde(default)]
    pub text: String,
    #[serde(default = "default_format")]
    pub format: String,
    /// Target width in pixels; bars keep whole pixels, so it may come out a little smaller.
    #[serde(default = "default_size")]
    pub size: u32,
    /// Bar height in pixels.
    #[serde(default = "default_height")]
    pub height: u32,
    /// Light border in modules; the standard asks for 10 on each side.
    #[serde(default = "default_margin")]
    pub margin: usize,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default = "default_background")]
    pub background: String,
    #[serde(default = "default_folder")]
    pub folder: String,
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
}

impl Default for Config {
    fn default() -> Self {
        serde_json::from_value(json!({})).expect("every field has a default")
    }
}

pub fn definition() -> NodeDefinition {
    let example_ref = json!({ "__zf_type": "file_ref", "backend": "zebfs", "ref": "tickets/codes/T-000481.svg", "filename": "T-000481.svg", "mime": "image/svg+xml", "kind": "image", "size": 1904, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.barcode.code128", "trust": "generated", "format": "svg", "width": 278, "height": 100, "svg": "<svg …>…</svg>" });
    NodeDefinition {
        kind: NODE_KIND.into(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "Barcode (Code 128)".into(),
        description: "Make a Code 128 barcode for `--text` (printable ASCII: a ticket, order or certificate number) and store it in \
            `--folder` as `--format svg|png`. Adds `barcode` to the payload: the file's FileRef with `width` and `height`, and for SVG \
            the markup in `barcode.svg`. Digit runs are packed two to a bar symbol, so long numbers stay short. For a URL or \
            non-ASCII text use `fs.barcode.qr`."
            .into(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `barcode` is added." }),
        output_schema: json!({
            "type": "object",
            "properties": { "barcode": { "type": "object", "description": "A FileRef plus width, height, and svg (SVG only)." } }
        }),
        input_pins: vec![INPUT_PIN_IN.into()],
        output_pins: vec![OUTPUT_PIN_OUT.into()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            flag("--text", "text", "What the barcode says (printable ASCII). Usually a {{ }} expression.", DslFlagKind::Scalar, true),
            flag("--format", "format", "svg|png (default svg).", DslFlagKind::Scalar, false),
            flag("--size", "size", "Target width in px (default 300).", DslFlagKind::Scalar, false),
            flag("--height", "height", "Bar height in px (default 80).", DslFlagKind::Scalar, false),
            flag("--margin", "margin", "Light border in modules (default 10).", DslFlagKind::Scalar, false),
            flag("--color", "color", "Bar colour, #rgb or #rrggbb (default #000000).", DslFlagKind::Scalar, false),
            flag("--background", "background", "Background, #rgb or #rrggbb (default #ffffff).", DslFlagKind::Scalar, false),
            flag("--folder", "folder", "Store folder (default barcodes).", DslFlagKind::Scalar, false),
            flag("--filename", "filename", "File name without extension (default a UUID).", DslFlagKind::Scalar, false),
        ].into_iter().chain(super::destination_flags()).collect(),
        fields: vec![
            field("text", "Text", "Printable ASCII: a ticket, order or certificate number.", None),
            select("format", "Format", "SVG stays sharp at any size; PNG for places that need pixels.", "svg", &[("svg", "SVG"), ("png", "PNG")]),
            field("size", "Width (px)", "Target width in pixels (default 300).", Some(json!("300"))),
            field("height", "Bar height (px)", "Default 80.", Some(json!("80"))),
            field("margin", "Margin (modules)", "Light border; scanners need 10.", Some(json!("10"))),
            field("color", "Colour", "#rgb or #rrggbb.", Some(json!("#000000"))),
            field("background", "Background", "#rgb or #rrggbb.", Some(json!("#ffffff"))),
            field("folder", "Folder", "Store folder (default barcodes).", Some(json!("barcodes"))),
            field("filename", "Filename", "Without extension (default a UUID).", None),
        ].into_iter().chain(super::destination_fields()).collect(),
        layout: ["text", "format", "size", "height", "margin", "color", "background", "folder", "filename", "path", "store", "on_conflict"]
            .into_iter()
            .map(|f| LayoutItem::Field(f.into()))
            .collect(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl(
                "A ticket number under its QR",
                "fs.barcode.code128 --text \"{{ input.ticket }}\" --folder tickets/codes --filename \"{{ input.ticket }}\"",
            )
            .input(json!({ "ticket": "T-000481" }))
            .output(json!({ "ticket": "T-000481", "barcode": example_ref })),
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
        let c = &self.config;
        if c.text.is_empty() {
            return Err(PipelineError::new(ERR, "--text is empty: say what the barcode should carry"));
        }
        let format = c.format.trim().to_ascii_lowercase();
        if format != "svg" && format != "png" {
            return Err(PipelineError::new(ERR, format!("--format '{}' is not svg or png", c.format)));
        }
        let colour = |raw: &str, flag: &str| {
            parse_colour(raw).ok_or_else(|| PipelineError::new(ERR, format!("{flag} '{raw}' is not #rgb or #rrggbb")))
        };
        let (dark, light) = (colour(&c.color, "--color")?, colour(&c.background, "--background")?);
        let values = encode::values(&c.text).map_err(|bad| {
            PipelineError::new(ERR, format!("'{bad}' is not printable ASCII; Code 128 cannot carry it — use fs.barcode.qr"))
        })?;
        let row = vec![encode::modules(&values)];
        let margin = c.margin.min(40);
        let module_px = (c.size / (row[0].len() + 2 * margin) as u32).max(1);
        let drawing = Drawing { rows: &row, margin, module_px, row_px: c.height.clamp(1, 2000), dark, light };
        let svg = (format == "svg").then(|| drawing.svg());
        let bytes = match &svg {
            Some(markup) => markup.clone().into_bytes(),
            None => drawing.png()?,
        };
        let mut barcode = super::save(
            &self.platform,
            super::Output { owner, project, folder: &c.folder, filename: c.filename.as_deref(), path: c.path.as_deref(), store: c.store.as_deref(), on_conflict: c.on_conflict.as_deref(), format: &format, bytes, origin: "fs.barcode.code128" },
        )?;
        if let Some(map) = barcode.as_object_mut() {
            map.insert("format".into(), json!(format));
            map.insert("width".into(), json!(drawing.width_px()));
            map.insert("height".into(), json!(drawing.height_px()));
            if let Some(markup) = svg {
                map.insert("svg".into(), Value::String(markup));
            }
        }
        let trace = format!("node_kind={NODE_KIND} {} symbols {}", values.len(), barcode["ref"]);
        let mut out = match input.payload {
            Value::Object(map) => map,
            _ => serde_json::Map::new(),
        };
        out.insert("barcode".into(), barcode);
        Ok(NodeExecutionOutput { output_pins: vec![OUTPUT_PIN_OUT.into()], payload: Value::Object(out), trace: vec![trace] })
    }
}

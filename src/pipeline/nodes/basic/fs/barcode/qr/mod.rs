//! n.fs.barcode.qr — a QR Code for any text (a URL, a certificate number),
//! stored as SVG or PNG.
//!
//! The encoder is ours (ISO/IEC 18004): `encode` builds the symbol,
//! `reed_solomon` its error correction, `penalty` picks the mask, `tables`
//! holds the standard's capacities. The payload gains `qr`: the stored
//! file's FileRef plus the symbol's facts, and for SVG the markup itself, so
//! a template can place the code inline and stay vector.

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
mod penalty;
pub mod reed_solomon;
pub mod tables;

pub const NODE_KIND: &str = "n.fs.barcode.qr";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";
const ERR: &str = "FW_NODE_FS_BARCODE_QR";

fn default_ecc() -> String {
    "M".into()
}
fn default_format() -> String {
    "svg".into()
}
fn default_size() -> u32 {
    256
}
fn default_margin() -> usize {
    4
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
    /// What the code says: a URL, a number, any text (UTF-8).
    #[serde(default)]
    pub text: String,
    /// Error correction: L (7%), M (15%), Q (25%), H (30%) of the code may be damaged.
    #[serde(default = "default_ecc")]
    pub ecc: String,
    /// `svg` or `png`.
    #[serde(default = "default_format")]
    pub format: String,
    /// Target width in pixels; the code keeps whole modules, so it may come out a little smaller.
    #[serde(default = "default_size")]
    pub size: u32,
    /// Light border in modules; the standard asks for 4.
    #[serde(default = "default_margin")]
    pub margin: usize,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default = "default_background")]
    pub background: String,
    #[serde(default = "default_folder")]
    pub folder: String,
    /// File name without extension; a UUID when unset.
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
    let example_ref = json!({ "__zf_type": "file_ref", "backend": "zebfs", "ref": "certificates/qr/CERT-2026-0412.svg", "filename": "CERT-2026-0412.svg", "mime": "image/svg+xml", "kind": "image", "size": 2841, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.barcode.qr", "trust": "generated", "format": "svg", "width": 232, "height": 232, "version": 3, "ecc": "M", "modules": 29, "svg": "<svg …>…</svg>" });
    NodeDefinition {
        kind: NODE_KIND.into(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "QR Code".into(),
        description: "Make a QR Code for `--text` (a URL, a number, any text) and store it in `--folder` as `--format svg|png`. \
            Adds `qr` to the payload: the file's FileRef (`ref`, `filename`, `size`, …) with `width`, `height`, `version`, `ecc`, \
            `modules`, and for SVG the markup in `qr.svg`, to place inline in another SVG (a certificate) so it stays sharp in print. \
            `--ecc L|M|Q|H` trades size for damage tolerance (default M); `--margin` is the light border in modules (default 4)."
            .into(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `qr` is added." }),
        output_schema: json!({
            "type": "object",
            "properties": { "qr": { "type": "object", "description": "A FileRef plus width, height, version, ecc, modules, and svg (SVG only)." } }
        }),
        input_pins: vec![INPUT_PIN_IN.into()],
        output_pins: vec![OUTPUT_PIN_OUT.into()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            flag("--text", "text", "What the code says. Usually a {{ }} expression.", DslFlagKind::Scalar, true),
            flag("--ecc", "ecc", "Error correction L|M|Q|H (default M).", DslFlagKind::Scalar, false),
            flag("--format", "format", "svg|png (default svg).", DslFlagKind::Scalar, false),
            flag("--size", "size", "Target width in px (default 256).", DslFlagKind::Scalar, false),
            flag("--margin", "margin", "Light border in modules (default 4).", DslFlagKind::Scalar, false),
            flag("--color", "color", "Dark colour, #rgb or #rrggbb (default #000000).", DslFlagKind::Scalar, false),
            flag("--background", "background", "Light colour, #rgb or #rrggbb (default #ffffff).", DslFlagKind::Scalar, false),
            flag("--folder", "folder", "Store folder (default barcodes).", DslFlagKind::Scalar, false),
            flag("--filename", "filename", "File name without extension (default a UUID).", DslFlagKind::Scalar, false),
        ].into_iter().chain(super::destination_flags()).collect(),
        fields: vec![
            field("text", "Text", "What the code says: a URL, a number, any text.", None),
            select("ecc", "Error correction", "How much of the code may be damaged and still read.", "M", &[("L", "L — 7%"), ("M", "M — 15%"), ("Q", "Q — 25%"), ("H", "H — 30%")]),
            select("format", "Format", "SVG stays sharp at any size; PNG for places that need pixels.", "svg", &[("svg", "SVG"), ("png", "PNG")]),
            field("size", "Size (px)", "Target width in pixels (default 256).", Some(json!("256"))),
            field("margin", "Margin (modules)", "Light border; scanners need 4.", Some(json!("4"))),
            field("color", "Colour", "#rgb or #rrggbb.", Some(json!("#000000"))),
            field("background", "Background", "#rgb or #rrggbb.", Some(json!("#ffffff"))),
            field("folder", "Folder", "Store folder (default barcodes).", Some(json!("barcodes"))),
            field("filename", "Filename", "Without extension (default a UUID).", None),
        ].into_iter().chain(super::destination_fields()).collect(),
        layout: ["text", "ecc", "format", "size", "margin", "color", "background", "folder", "filename", "path", "store", "on_conflict"]
            .into_iter()
            .map(|f| LayoutItem::Field(f.into()))
            .collect(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl(
                "A verification link for a certificate",
                "fs.barcode.qr --text \"{{ 'https://example.com/certificates/' + input.number }}\" --folder certificates/qr --filename \"{{ input.number }}\"",
            )
            .input(json!({ "number": "CERT-2026-0412" }))
            .output(json!({ "number": "CERT-2026-0412", "qr": example_ref })),
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
            return Err(PipelineError::new(ERR, "--text is empty: say what the code should carry"));
        }
        let ecc = tables::Ecc::parse(&c.ecc).ok_or_else(|| PipelineError::new(ERR, format!("--ecc '{}' is not L, M, Q or H", c.ecc)))?;
        let format = c.format.trim().to_ascii_lowercase();
        if format != "svg" && format != "png" {
            return Err(PipelineError::new(ERR, format!("--format '{}' is not svg or png", c.format)));
        }
        let colour = |raw: &str, flag: &str| {
            parse_colour(raw).ok_or_else(|| PipelineError::new(ERR, format!("{flag} '{raw}' is not #rgb or #rrggbb")))
        };
        let (dark, light) = (colour(&c.color, "--color")?, colour(&c.background, "--background")?);
        let symbol = encode::encode(&c.text, ecc).ok_or_else(|| {
            PipelineError::new(ERR, format!("{} bytes do not fit a QR Code at ecc {}; shorten the text or lower --ecc", c.text.len(), ecc.letter()))
        })?;

        let margin = c.margin.min(16);
        let module_px = (c.size / (symbol.size + 2 * margin) as u32).max(1);
        let drawing = Drawing { rows: &symbol.modules, margin, module_px, row_px: module_px, dark, light };
        let svg = (format == "svg").then(|| drawing.svg());
        let bytes = match &svg {
            Some(markup) => markup.clone().into_bytes(),
            None => drawing.png()?,
        };
        let mut qr = super::save(
            &self.platform,
            super::Output { owner, project, folder: &c.folder, filename: c.filename.as_deref(), path: c.path.as_deref(), store: c.store.as_deref(), on_conflict: c.on_conflict.as_deref(), format: &format, bytes, origin: "fs.barcode.qr" },
        )?;
        let facts = json!({
            "format": format, "width": drawing.width_px(), "height": drawing.height_px(),
            "version": symbol.version, "ecc": symbol.ecc.letter(), "modules": symbol.size,
        });
        if let (Some(map), Some(extra)) = (qr.as_object_mut(), facts.as_object()) {
            map.extend(extra.clone());
            if let Some(markup) = svg {
                map.insert("svg".into(), Value::String(markup));
            }
        }
        let trace = format!("node_kind={NODE_KIND} v{} {} mask {} {}", symbol.version, symbol.ecc.letter(), symbol.mask, qr["ref"]);
        let payload = crate::pipeline::nodes::shared::util::with_answer(&input.payload, json!({ "qr": qr }));
        Ok(NodeExecutionOutput { output_pins: vec![OUTPUT_PIN_OUT.into()], payload, trace: vec![trace] })
    }
}

#[cfg(test)]
mod tests;

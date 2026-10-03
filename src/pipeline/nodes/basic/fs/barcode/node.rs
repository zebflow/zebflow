//! `fs.barcode.render` — a code a scanner reads, drawn and stored as SVG or
//! PNG. `--symbology` picks the code: `qr` (2D, any text, `qr/`) or
//! `code128` (linear, printable ASCII, `code128/`); `render.rs` draws either.
//!
//! The answer is one key, `barcode`: the durable FileRef of the stored file
//! (`origin: fs.barcode.render`) with `symbology`, `width`, `height` and
//! `format` inside it; the rest of the payload stays.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use super::code128;
use super::qr::{encode as qr_encode, tables::Ecc};
use super::render::{Drawing, parse_colour};
use crate::pipeline::model::NodeCapability;
use crate::pipeline::nodes::shared::limits::{MAX_RASTER_SIDE, choice, raster, within};
use crate::pipeline::nodes::shared::project_store::{OnConflict, on_conflict_flag, open_store, store_fields, store_flag, target_key};
use crate::pipeline::nodes::shared::util::{filename_stem, metadata_scope, with_answer};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeExample, NodeFieldDef, NodeFieldType, SelectOptionDef},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

pub const NODE_KIND: &str = "fs.barcode.render";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

/// The store write and the PNG encoder: the world's side.
pub const CODE: &str = "FW_NODE_FS_BARCODE_RENDER";
/// A flag the author set wrong, or one this symbology does not take.
pub const CONFIG_CODE: &str = "FW_NODE_FS_BARCODE_RENDER_CONFIG";
/// `--text` is empty, or this symbology cannot carry it.
const TEXT_CODE: &str = "FW_NODE_FS_BARCODE_RENDER_TEXT";
/// The drawing is over the shared raster ceiling.
pub const SIZE_CODE: &str = "FW_NODE_FS_BARCODE_RENDER_SIZE";

const SYMBOLOGIES: &[&str] = &["qr", "code128"];
const FORMATS: &[&str] = &["svg", "png"];
const ECC_LEVELS: &[&str] = &["L", "M", "Q", "H"];
const DEFAULT_FOLDER: &str = "barcodes";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// What the code carries. A number is read as its digits.
    #[serde(default)]
    pub text: Value,
    /// `qr` (default) or `code128`.
    #[serde(default)]
    pub symbology: String,
    /// `svg` (default) or `png`.
    #[serde(default)]
    pub format: String,
    /// Target width in pixels (qr 256, code128 300); whole modules, so it may come out a little smaller.
    #[serde(default)]
    pub width: Value,
    /// code128's bar height in pixels (default 80); a QR Code is square and refuses it.
    #[serde(default)]
    pub height: Value,
    /// Light border in modules (qr 4, code128 10).
    #[serde(default)]
    pub margin: Value,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub background: String,
    /// QR error correction L|M|Q|H (default M); code128 refuses it.
    #[serde(default)]
    pub ecc: String,
    #[serde(default)]
    pub store: Option<String>,
    #[serde(default)]
    pub folder: Value,
    #[serde(default)]
    pub filename: Value,
    #[serde(default)]
    pub path: Value,
    #[serde(default)]
    pub on_conflict: Option<String>,
}

/// One symbology's defaults and ceilings.
struct Symbology {
    name: &'static str,
    width: u32,
    margin: u32,
    max_margin: u32,
}

const QR: Symbology = Symbology { name: "qr", width: 256, margin: 4, max_margin: 16 };
const CODE128: Symbology = Symbology { name: "code128", width: 300, margin: 10, max_margin: 40 };
const DEFAULT_BAR_HEIGHT: u32 = 80;

fn flag(name: &str, key: &str, description: &str, value: &str) -> DslFlag {
    DslFlag {
        flag: name.to_string(),
        config_key: key.to_string(),
        description: description.to_string(),
        kind: DslFlagKind::Scalar,
        value: value.to_string(),
        ..Default::default()
    }
}

fn field(name: &str, label: &str, help: &str) -> NodeFieldDef {
    NodeFieldDef { name: name.to_string(), label: label.to_string(), field_type: NodeFieldType::Text, help: Some(help.to_string()), ..Default::default() }
}

fn select(name: &str, label: &str, help: &str, options: &[(&str, &str)]) -> NodeFieldDef {
    NodeFieldDef {
        field_type: NodeFieldType::Select,
        options: options.iter().map(|(v, l)| SelectOptionDef { value: v.to_string(), label: l.to_string() }).collect(),
        ..field(name, label, help)
    }
}

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|word| word.to_string()).collect()
}

pub fn definition() -> NodeDefinition {
    let answer = |r: &str, size: u64, symbology: &str, width: u32, height: u32| {
        json!({ "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": r, "filename": r.rsplit('/').next().unwrap_or(r), "mime": "image/svg+xml", "kind": "image", "size": size, "sha256": "sha256:…", "lifecycle": "durable", "origin": NODE_KIND, "trust": "generated", "symbology": symbology, "format": "svg", "width": width, "height": height })
    };
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "Barcode".to_string(),
        description: "Draw a code a scanner reads for `--text` and store it as `--format svg|png` (default svg). \
            `--symbology qr` (default) carries any text — a URL, a number — with `--ecc L|M|Q|H` (default M) trading size for damage tolerance; \
            `--symbology code128` is a linear barcode for printable ASCII (a ticket or order number), digit runs packed two to a bar, \
            `--height` its bar height (default 80). `--width` is the target width in px (qr 256, code128 300); `--margin` the light border \
            in modules (qr 4, code128 10). Writes under `--folder` (default `barcodes`) as a generated name, as `--filename`, or at an exact \
            `--path`; a named file that exists is an error unless `--on-conflict` says otherwise. Adds `barcode` — the durable FileRef \
            (`ref`, `store`, `size`, …, `origin: fs.barcode.render`) with `symbology`, `format`, `width` and `height` in px — and keeps the rest of the payload."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `barcode` is added." }),
        output_schema: json!({
            "type": "object",
            "properties": { "barcode": {
                "type": "object",
                "description": "The durable FileRef of the stored code, with what was drawn.",
                "properties": {
                    "ref": { "type": "string" },
                    "store": { "type": "string" },
                    "origin": { "type": "string", "const": NODE_KIND },
                    "symbology": { "type": "string", "enum": SYMBOLOGIES },
                    "format": { "type": "string", "enum": FORMATS },
                    "width": { "type": "integer", "description": "Pixels, quiet zone included" },
                    "height": { "type": "integer", "description": "Pixels, quiet zone included" }
                }
            } }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag { required: true, ..flag("--text", "text", "What the code carries; code128 takes printable ASCII only. Usually a {{ }} expression.", "text") },
            DslFlag { choices: words(SYMBOLOGIES), ..flag("--symbology", "symbology", "The code: qr (default) or code128.", "") },
            DslFlag { choices: words(FORMATS), ..flag("--format", "format", "svg (default) or png.", "") },
            flag("--width", "width", "Target width in px (default: qr 256, code128 300).", "number"),
            flag("--height", "height", "code128 bar height in px (default 80); a QR Code is square and refuses it.", "number"),
            flag("--margin", "margin", "Light border in modules (default: qr 4, code128 10).", "number"),
            flag("--color", "color", "Dark colour, #rgb or #rrggbb (default #000000).", "text"),
            flag("--background", "background", "Light colour, #rgb or #rrggbb (default #ffffff).", "text"),
            DslFlag { choices: words(ECC_LEVELS), ..flag("--ecc", "ecc", "QR error correction (default M); code128 refuses it.", "") },
            DslFlag { value: "text".to_string(), ..store_flag() },
            flag("--folder", "folder", "Destination folder (default: barcodes).", "text"),
            flag("--filename", "filename", "Destination name without extension (default: a generated name).", "text"),
            flag("--path", "path", "Exact destination key; overrides --folder and --filename.", "text"),
            DslFlag { choices: words(&["error", "skip", "overwrite"]), ..on_conflict_flag(OnConflict::Error) },
        ],
        fields: vec![
            field("text", "Text", "What the code carries: a URL, a number, any text (code128: printable ASCII)."),
            NodeFieldDef { default_value: Some(json!("qr")), ..select("symbology", "Symbology", "QR for any text; Code 128 for a short ASCII number under a QR.", &[("qr", "QR Code"), ("code128", "Code 128")]) },
            NodeFieldDef { default_value: Some(json!("svg")), ..select("format", "Format", "SVG stays sharp at any size; PNG for places that need pixels.", &[("svg", "SVG"), ("png", "PNG")]) },
            field("width", "Width (px)", "Target width (default: QR 256, Code 128 300)."),
            field("height", "Bar height (px)", "Code 128 only (default 80)."),
            field("margin", "Margin (modules)", "Light border (default: QR 4, Code 128 10); scanners need it."),
            NodeFieldDef { default_value: Some(json!("#000000")), ..field("color", "Colour", "#rgb or #rrggbb.") },
            NodeFieldDef { default_value: Some(json!("#ffffff")), ..field("background", "Background", "#rgb or #rrggbb.") },
            select("ecc", "Error correction", "QR only: how much of the code may be damaged and still read.", &[("", "Default (M)"), ("L", "L — 7%"), ("M", "M — 15%"), ("Q", "Q — 25%"), ("H", "H — 30%")]),
            field("folder", "Folder", "Destination folder (default: barcodes)."),
            field("filename", "Filename", "Without extension (default: a generated name)."),
            field("path", "Path", "Exact destination key; overrides folder and filename."),
        ]
        .into_iter()
        .chain(store_fields(OnConflict::Error))
        .collect(),
        layout: ["text", "symbology", "format", "width", "height", "margin", "color", "background", "ecc", "folder", "filename", "path", "store", "on_conflict"]
            .iter()
            .map(|name| LayoutItem::Field(name.to_string()))
            .collect(),
        examples: vec![
            NodeExample::dsl(
                "A verification link for a certificate",
                "fs.barcode.render --text \"{{ 'https://example.com/certificates/' + input.number }}\" --folder certificates/qr --filename \"{{ input.number }}\"",
            )
            .input(json!({ "number": "CERT-2026-0412" }))
            .output(json!({ "number": "CERT-2026-0412", "barcode": answer("certificates/qr/CERT-2026-0412.svg", 2841, "qr", 232, 232) })),
            NodeExample::dsl(
                "A ticket number as a linear barcode",
                "fs.barcode.render --symbology code128 --text \"{{ input.ticket }}\" --folder tickets/codes --filename \"{{ input.ticket }}\"",
            )
            .input(json!({ "ticket": "T-000481" }))
            .output(json!({ "ticket": "T-000481", "barcode": answer("tickets/codes/T-000481.svg", 1904, "code128", 278, 100) })),
        ],
        ..Default::default()
    }
}

/// A flag that is text to the author: a string, or a number read as its digits.
fn text_of(value: &Value, flag: &str, code: &'static str) -> Result<String, PipelineError> {
    match value {
        Value::Null => Ok(String::new()),
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        _ => Err(PipelineError::new(code, format!("{flag} must be text, not {value}"))),
    }
}

/// A whole number flag, `None` when unset.
fn number_of(value: &Value, flag: &str) -> Result<Option<u32>, PipelineError> {
    let bad = || PipelineError::new(CONFIG_CODE, format!("{flag} '{value}' is not a whole number"));
    match value {
        Value::Null => Ok(None),
        Value::String(s) if s.trim().is_empty() => Ok(None),
        Value::String(s) => s.trim().parse::<u32>().map(Some).map_err(|_| bad()),
        Value::Number(n) => n.as_u64().and_then(|n| u32::try_from(n).ok()).map(Some).ok_or_else(bad),
        _ => Err(bad()),
    }
}

/// Every flag read and checked before anything is drawn.
struct Plan {
    symbology: &'static Symbology,
    text: String,
    format: &'static str,
    width: u32,
    bar_height: u32,
    margin: usize,
    ecc: Ecc,
    dark: [u8; 3],
    light: [u8; 3],
}

impl Plan {
    fn read(c: &Config) -> Result<Self, PipelineError> {
        let symbology = match choice(&c.symbology, SYMBOLOGIES, "qr", "--symbology", CONFIG_CODE)? {
            "code128" => &CODE128,
            _ => &QR,
        };
        let text = text_of(&c.text, "--text", TEXT_CODE)?;
        if text.is_empty() {
            return Err(PipelineError::new(TEXT_CODE, "--text is empty: say what the code should carry"));
        }
        let format = choice(&c.format, FORMATS, "svg", "--format", CONFIG_CODE)?;
        let ecc = choice(&c.ecc, ECC_LEVELS, "M", "--ecc", CONFIG_CODE)?;
        let height = number_of(&c.height, "--height")?;
        if symbology.name == "qr" && height.is_some() {
            return Err(PipelineError::new(CONFIG_CODE, "--height is for code128: a QR Code is square, so set --width only"));
        }
        if symbology.name == "code128" && !c.ecc.trim().is_empty() {
            return Err(PipelineError::new(CONFIG_CODE, "--ecc is for qr: Code 128 has a check symbol, not error correction levels"));
        }
        let width = within(number_of(&c.width, "--width")?.unwrap_or(symbology.width), 1, MAX_RASTER_SIDE, "--width", CONFIG_CODE)?;
        let bar_height = within(height.unwrap_or(DEFAULT_BAR_HEIGHT), 1, MAX_RASTER_SIDE, "--height", CONFIG_CODE)?;
        let margin = within(number_of(&c.margin, "--margin")?.unwrap_or(symbology.margin), 0, symbology.max_margin, "--margin", CONFIG_CODE)?;
        let colour = |raw: &str, default: &str, flag: &str| {
            let raw = if raw.trim().is_empty() { default } else { raw };
            parse_colour(raw).ok_or_else(|| PipelineError::new(CONFIG_CODE, format!("{flag} '{raw}' is not #rgb or #rrggbb")))
        };
        Ok(Self {
            symbology,
            text,
            format,
            width,
            bar_height,
            margin: margin as usize,
            ecc: Ecc::parse(ecc).expect("a listed level"),
            dark: colour(&c.color, "#000000", "--color")?,
            light: colour(&c.background, "#ffffff", "--background")?,
        })
    }

    /// The modules, `rows[y][x]`, and a line for the trace.
    fn modules(&self) -> Result<(Vec<Vec<bool>>, String), PipelineError> {
        if self.symbology.name == "code128" {
            let values = code128::encode::values(&self.text).map_err(|bad| {
                PipelineError::new(TEXT_CODE, format!("'{bad}' is not printable ASCII; Code 128 cannot carry it — use --symbology qr"))
            })?;
            return Ok((vec![code128::encode::modules(&values)], format!("{} symbols", values.len())));
        }
        let symbol = qr_encode::encode(&self.text, self.ecc).ok_or_else(|| {
            PipelineError::new(TEXT_CODE, format!("{} bytes do not fit a QR Code at ecc {}; shorten the text or lower --ecc", self.text.len(), self.ecc.letter()))
        })?;
        Ok((symbol.modules, format!("v{} {} mask {}", symbol.version, symbol.ecc.letter(), symbol.mask)))
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
        let plan = Plan::read(c)?;
        let (rows, facts) = plan.modules()?;
        let columns = rows.first().map_or(0, Vec::len);
        let module_px = (plan.width / (columns + 2 * plan.margin) as u32).max(1);
        let row_px = if plan.symbology.name == "qr" { module_px } else { plan.bar_height };
        let drawing = Drawing { rows: &rows, margin: plan.margin, module_px, row_px, dark: plan.dark, light: plan.light };
        // Either format is held to the raster ceiling, so a long text never
        // draws an SVG no screen or printer could hold.
        raster(drawing.width_px(), drawing.height_px(), SIZE_CODE)?;
        let bytes = if plan.format == "svg" { drawing.svg().into_bytes() } else { drawing.png()? };

        let folder = text_of(&c.folder, "--folder", CONFIG_CODE)?;
        let folder = if folder.trim().is_empty() { DEFAULT_FOLDER.to_string() } else { folder };
        let filename = text_of(&c.filename, "--filename", CONFIG_CODE)?;
        let path = text_of(&c.path, "--path", CONFIG_CODE)?;
        let stem = Some(filename_stem(&filename)).filter(|s| !s.is_empty()).unwrap_or_else(|| Uuid::new_v4().to_string());
        let name = format!("{stem}.{}", plan.format);
        let key = target_key(Some(path.as_str()).filter(|p| !p.trim().is_empty()), &folder, &name, CODE)?;
        let store = open_store(&self.platform, owner, project, c.store.as_deref())?;
        let mut barcode = if OnConflict::parse(c.on_conflict.as_deref(), OnConflict::Error, CODE)?.allows(&store.fs, &key, CODE)? {
            store.fs.put(&key, &bytes).map_err(|e| PipelineError::new(CODE, format!("write '{key}': {e}")))?;
            let mime = if plan.format == "svg" { "image/svg+xml" } else { "image/png" };
            let leaf = key.rsplit('/').next().unwrap_or(&key);
            store.file_ref(&key, leaf, mime, &bytes, NODE_KIND, "generated")
        } else {
            // Skipped: the answer is the file already there, as it is.
            store.stored_ref(&key, NODE_KIND, "generated", CODE)?
        };
        if let Some(map) = barcode.as_object_mut() {
            map.insert("symbology".into(), json!(plan.symbology.name));
            map.insert("format".into(), json!(plan.format));
            map.insert("width".into(), json!(drawing.width_px()));
            map.insert("height".into(), json!(drawing.height_px()));
        }
        let trace = format!("node_kind={NODE_KIND} {} {facts} {}", plan.symbology.name, barcode["ref"]);
        let payload = with_answer(&input.payload, json!({ "barcode": barcode }));
        Ok(NodeExecutionOutput { output_pins: vec![OUTPUT_PIN_OUT.into()], payload, trace: vec![trace] })
    }
}

#[cfg(test)]
mod tests;

//! `fs.barcode.*` — codes a scanner reads, written as a stored file:
//! `fs.barcode.qr` (2D) and `fs.barcode.code128` (linear). Encoders are written here, against the standards,
//! with no barcode crate; each symbology is one folder beside `qr/`.
//! `render` draws any of them as SVG or PNG.

use std::sync::Arc;

use serde_json::{Value, json};
use uuid::Uuid;

use crate::pipeline::PipelineError;
use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType, SelectOptionDef};
use crate::pipeline::nodes::shared::project_store::{OnConflict, on_conflict_flag, open_store, store_fields, store_flag, target_key};
use crate::pipeline::nodes::shared::util::filename_stem;
use crate::platform::services::PlatformService;

pub mod code128;
pub mod qr;
pub mod render;

/// One file a barcode node is about to store.
pub struct Output<'a> {
    pub owner: &'a str,
    pub project: &'a str,
    pub folder: &'a str,
    pub filename: Option<&'a str>,
    /// Exact store key; overrides `folder` and `filename`.
    pub path: Option<&'a str>,
    /// The store to write to; `None` is the project's default.
    pub store: Option<&'a str>,
    /// `overwrite`, `skip` or `error` (default: error).
    pub on_conflict: Option<&'a str>,
    /// `svg` or `png`.
    pub format: &'a str,
    pub bytes: Vec<u8>,
    /// The node kind, recorded as the FileRef's `origin`.
    pub origin: &'a str,
}

/// Write the file into the node's store and answer its FileRef.
pub fn save(platform: &Arc<PlatformService>, out: Output<'_>) -> Result<Value, PipelineError> {
    let err = |m: String| PipelineError::new("FW_NODE_FS_BARCODE", m);
    let folder = out.folder;
    let stem = out.filename.map(filename_stem).filter(|s| !s.is_empty()).unwrap_or_else(|| Uuid::new_v4().to_string());
    let name = format!("{stem}.{}", out.format);
    let rel = target_key(out.path, folder, &name, "FW_NODE_FS_BARCODE")?;
    let store = open_store(platform, out.owner, out.project, out.store)?;
    if !OnConflict::parse(out.on_conflict, OnConflict::Error, "FW_NODE_FS_BARCODE")?.allows(&store.fs, &rel, "FW_NODE_FS_BARCODE")? {
        // Skipped: the answer is the file already there, as it is.
        return store.stored_ref(&rel, out.origin, "generated", "FW_NODE_FS_BARCODE");
    }
    store.fs.put(&rel, &out.bytes).map_err(|e| err(format!("write: {e}")))?;
    let mime = if out.format == "svg" { "image/svg+xml" } else { "image/png" };
    Ok(store.file_ref(&rel, &name, mime, &out.bytes, out.origin, "generated"))
}

/// `--path`, `--store` and `--on-conflict`, the destination flags every
/// symbology takes beside `--folder` and `--filename`.
pub(super) fn destination_flags() -> Vec<DslFlag> {
    vec![
        flag("--path", "path", "Exact store key; overrides --folder and --filename.", DslFlagKind::Scalar, false),
        store_flag(),
        on_conflict_flag(OnConflict::Error),
    ]
}

/// The editor fields for those flags.
pub(super) fn destination_fields() -> Vec<NodeFieldDef> {
    let mut fields = vec![field("path", "Path", "Exact store key; overrides folder and filename.", None)];
    fields.extend(store_fields(OnConflict::Error));
    fields
}

// ── Definition helpers shared by the symbologies ─────────────────────────

pub(super) fn flag(flag: &str, key: &str, description: &str, kind: DslFlagKind, required: bool) -> DslFlag {
    DslFlag { flag: flag.into(), config_key: key.into(), description: description.into(), kind, required }
}

pub(super) fn field(name: &str, label: &str, help: &str, default: Option<Value>) -> NodeFieldDef {
    NodeFieldDef {
        name: name.into(),
        label: label.into(),
        field_type: NodeFieldType::Text,
        help: Some(help.into()),
        default_value: default,
        ..Default::default()
    }
}

pub(super) fn select(name: &str, label: &str, help: &str, default: &str, options: &[(&str, &str)]) -> NodeFieldDef {
    NodeFieldDef {
        field_type: NodeFieldType::Select,
        options: options.iter().map(|(v, l)| SelectOptionDef { value: (*v).into(), label: (*l).into() }).collect(),
        ..field(name, label, help, Some(json!(default)))
    }
}

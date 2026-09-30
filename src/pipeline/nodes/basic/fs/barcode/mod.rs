//! `fs.barcode.*` — codes a scanner reads, written as a stored file:
//! `fs.barcode.qr` (2D) and `fs.barcode.code128` (linear). Encoders are written here, against the standards,
//! with no barcode crate; each symbology is one folder beside `qr/`.
//! `render` draws any of them as SVG or PNG.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::pipeline::PipelineError;
use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType, SelectOptionDef};
use crate::pipeline::nodes::shared::file_ref::{BACKEND_ZEBFS, FILE_REF_TYPE, LIFECYCLE_DURABLE};
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
    /// `svg` or `png`.
    pub format: &'a str,
    pub bytes: Vec<u8>,
    /// The node kind, recorded as the FileRef's `origin`.
    pub origin: &'a str,
}

/// Write the file into the project's store and answer its FileRef.
pub fn save(platform: &PlatformService, out: Output<'_>) -> Result<Value, PipelineError> {
    let err = |m: String| PipelineError::new("FS_BARCODE", m);
    let layout = platform.file.ensure_project_layout(out.owner, out.project).map_err(|e| err(e.to_string()))?;
    let folder = sanitize_folder(out.folder);
    let stem = out.filename.map(filename_stem).filter(|s| !s.is_empty()).unwrap_or_else(|| Uuid::new_v4().to_string());
    let name = format!("{stem}.{}", out.format);
    let rel = if folder.is_empty() { name.clone() } else { format!("{folder}/{name}") };
    let dest = layout.local_files_dir()?.join(&rel);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| err(format!("mkdir: {e}")))?;
    }
    let tmp = dest.with_extension(format!("{}.tmp", out.format));
    std::fs::write(&tmp, &out.bytes).map_err(|e| err(format!("write: {e}")))?;
    std::fs::rename(&tmp, &dest).map_err(|e| err(format!("rename: {e}")))?;
    Ok(json!({
        "__zf_type": FILE_REF_TYPE,
        "backend": BACKEND_ZEBFS,
        "ref": rel,
        "filename": name,
        "mime": if out.format == "svg" { "image/svg+xml" } else { "image/png" },
        "kind": "image",
        "size": out.bytes.len(),
        "sha256": format!("sha256:{:x}", Sha256::digest(&out.bytes)),
        "lifecycle": LIFECYCLE_DURABLE,
        "origin": out.origin,
        "trust": "generated",
    }))
}

fn sanitize_folder(folder: &str) -> String {
    folder
        .split('/')
        .filter(|seg| !seg.is_empty() && *seg != "." && *seg != "..")
        .collect::<Vec<_>>()
        .join("/")
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

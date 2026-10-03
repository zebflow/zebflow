//! The store a node reads from or writes to (`docs/contracts/node-conventions.md` §3).
//!
//! A project may hold several stores — `local`, and one per `s3` credential —
//! and one of them is the default. A node names its store with `--store`,
//! saved explicitly when the pipeline is registered; a FileRef names the store
//! that holds its bytes in `store`. Both resolve here, and every key a node
//! writes passes through [`target_key`] first.

use std::sync::Arc;

use serde_json::Value;

use crate::pipeline::PipelineError;
use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType, SelectOptionDef};
use crate::platform::services::PlatformService;
use crate::zebfs::{FileBackend, ZebFs};

/// One opened store and the words a FileRef needs to name it.
pub struct NodeStore {
    pub id: String,
    pub backend: FileBackend,
    pub fs: ZebFs,
}

impl NodeStore {
    /// A durable FileRef for bytes just written to this store.
    pub fn file_ref(
        &self,
        rel_path: &str,
        filename: &str,
        mime: &str,
        bytes: &[u8],
        origin: &str,
        trust: &str,
    ) -> Value {
        super::file_ref::durable_file_ref(self.backend, &self.id, rel_path, filename, mime, bytes, origin, trust)
    }
}

/// Opens one of the project's stores by id; `None` is the project's default.
pub fn open_store(
    platform: &Arc<PlatformService>,
    owner: &str,
    project: &str,
    store_id: Option<&str>,
) -> Result<NodeStore, PipelineError> {
    let store = platform.file.open_store(owner, project, store_id).map_err(|err| {
        let message = match store_id {
            Some(id) if !id.trim().is_empty() => format!("store '{}': {}", id.trim(), err.message),
            _ => err.message,
        };
        PipelineError::new("FW_NODE_STORE", message)
    })?;
    Ok(NodeStore { id: store.id, backend: store.backend, fs: store.fs })
}

/// The store and key a source value names: a FileRef's own `store` and `ref`,
/// or a bare store key in the node's own store.
pub fn open_source(
    platform: &Arc<PlatformService>,
    owner: &str,
    project: &str,
    value: &Value,
    node_store: Option<&str>,
) -> Result<Option<(NodeStore, String)>, PipelineError> {
    let Some(rel) = super::file_ref::zebfs_rel_path_or_string(value)? else {
        return Ok(None);
    };
    let store_id = if super::file_ref::is_file_ref(value) {
        value.get("store").and_then(Value::as_str)
    } else {
        node_store
    };
    Ok(Some((open_store(platform, owner, project, store_id)?, rel)))
}

/// Where a single-file node writes: `--path` exactly, or `--folder` joined to
/// `--filename`. Templates are already expanded; the key is normalised once
/// and anything escaping the store is refused.
pub fn target_key(
    path: Option<&str>,
    folder: &str,
    filename: &str,
    code: &'static str,
) -> Result<String, PipelineError> {
    let raw = match path.map(str::trim).filter(|p| !p.is_empty()) {
        Some(path) => path.to_string(),
        None => {
            let folder = folder.trim().trim_matches('/');
            if folder.is_empty() {
                filename.to_string()
            } else {
                format!("{folder}/{filename}")
            }
        }
    };
    crate::zebfs::normalize_object_path(raw.trim_start_matches('/'))
        .map_err(|err| PipelineError::new(code, format!("invalid destination '{raw}': {}", err.message)))
}

/// `--on-conflict`: what a write does when its target already exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnConflict {
    Overwrite,
    Skip,
    Error,
}

impl OnConflict {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Overwrite => "overwrite",
            Self::Skip => "skip",
            Self::Error => "error",
        }
    }

    /// Parses the flag; an empty value is the node's own default.
    pub fn parse(raw: Option<&str>, default: OnConflict, code: &'static str) -> Result<Self, PipelineError> {
        match raw.map(str::trim).unwrap_or("") {
            "" => Ok(default),
            "overwrite" => Ok(Self::Overwrite),
            "skip" => Ok(Self::Skip),
            "error" => Ok(Self::Error),
            other => Err(PipelineError::new(
                code,
                format!("--on-conflict '{other}' must be overwrite, skip or error"),
            )),
        }
    }

    /// Decides a tree write into `folder`: the target exists when the
    /// folder holds anything. `Ok(true)` writes, `Ok(false)` skips.
    pub fn allows_tree(self, store: &ZebFs, folder: &str, code: &'static str) -> Result<bool, PipelineError> {
        let exists = store.list(folder).map(|entries| !entries.is_empty()).unwrap_or(false);
        match (exists, self) {
            (false, _) | (true, Self::Overwrite) => Ok(true),
            (true, Self::Skip) => Ok(false),
            (true, Self::Error) => Err(PipelineError::new(
                code,
                format!("'{folder}' is not empty; set --on-conflict overwrite or skip"),
            )),
        }
    }

    /// Decides one write. `Ok(true)` writes, `Ok(false)` skips.
    pub fn allows(self, store: &ZebFs, key: &str, code: &'static str) -> Result<bool, PipelineError> {
        let exists = store.head(key).is_ok();
        match (exists, self) {
            (false, _) | (true, Self::Overwrite) => Ok(true),
            (true, Self::Skip) => Ok(false),
            (true, Self::Error) => Err(PipelineError::new(
                code,
                format!("'{key}' already exists; set --on-conflict overwrite or skip"),
            )),
        }
    }
}

/// The `--store` flag every file-writing node takes.
pub fn store_flag() -> DslFlag {
    DslFlag {
        flag: "--store".to_string(),
        config_key: "store".to_string(),
        description: "The project store to write to: `local`, or the id of an `s3` credential. Saved explicitly when the pipeline is registered (default: the project's default store at that moment)".to_string(),
        kind: DslFlagKind::Scalar,
        required: false,
    }
}

/// The `--on-conflict` flag, with the node's own default.
pub fn on_conflict_flag(default: OnConflict) -> DslFlag {
    DslFlag {
        flag: "--on-conflict".to_string(),
        config_key: "on_conflict".to_string(),
        description: format!(
            "When the target already exists: overwrite, skip or error (default: {})",
            default.as_str()
        ),
        kind: DslFlagKind::Scalar,
        required: false,
    }
}

/// The editor fields for `store` and `on_conflict`.
pub fn store_fields(default: OnConflict) -> Vec<NodeFieldDef> {
    vec![
        NodeFieldDef {
            name: "store".to_string(),
            label: "Store".to_string(),
            field_type: NodeFieldType::Text,
            help: Some("`local`, or the id of an s3 credential. Empty: the project's default, saved when the pipeline is registered.".to_string()),
            ..Default::default()
        },
        NodeFieldDef {
            name: "on_conflict".to_string(),
            label: "If the target exists".to_string(),
            field_type: NodeFieldType::Select,
            help: Some("error stops the run, skip keeps the existing file, overwrite replaces it.".to_string()),
            default_value: Some(serde_json::json!(default.as_str())),
            options: ["error", "skip", "overwrite"]
                .iter()
                .map(|value| SelectOptionDef { value: value.to_string(), label: value.to_string() })
                .collect(),
            ..Default::default()
        },
    ]
}

/// The node kinds that write files, whose `store` is saved explicitly when a
/// pipeline is registered (`node-conventions.md` §3).
pub const FILE_WRITING_NODE_KINDS: &[&str] = &[
    "n.fs.save",
    "n.fs.put",
    "n.fs.copy",
    "n.fs.move",
    "n.fs.mkdir",
    "n.fs.compress",
    "n.fs.decompress",
    "n.fs.pdf.convert",
    "n.fs.svg.convert",
    "n.fs.image.thumbnail",
    "n.fs.image.chromakey",
    "n.fs.barcode.qr",
    "n.fs.barcode.code128",
    "n.table.convert",
    "n.table.query",
    "n.ai.tts",
    "n.geo.convert",
    "n.web.static.generate",
    "n.web.docs.generate",
    "n.ms.publish",
];

#[cfg(test)]
mod tests {
    use super::{OnConflict, target_key};
    use crate::zebfs::{LocalZebFs, ZebFs};

    #[test]
    fn a_path_wins_and_every_key_is_normalised_once() {
        assert_eq!(target_key(None, "thumbnails", "a.jpg", "T").unwrap(), "thumbnails/a.jpg");
        assert_eq!(target_key(Some("/exact/b.jpg"), "thumbnails", "a.jpg", "T").unwrap(), "exact/b.jpg");
        assert_eq!(target_key(None, "", "a.jpg", "T").unwrap(), "a.jpg");
        assert!(target_key(Some("../escape.jpg"), "x", "y", "T").is_err());
        assert!(target_key(None, "a/../../b", "c", "T").is_err());
    }

    #[test]
    fn on_conflict_decides_an_existing_target() {
        let dir = tempfile::tempdir().unwrap();
        let store = ZebFs::Local(LocalZebFs::new(dir.path().to_path_buf()));
        store.put("a.txt", b"1").unwrap();
        assert!(OnConflict::Overwrite.allows(&store, "a.txt", "T").unwrap());
        assert!(!OnConflict::Skip.allows(&store, "a.txt", "T").unwrap());
        assert!(OnConflict::Error.allows(&store, "a.txt", "T").is_err());
        assert!(OnConflict::Error.allows(&store, "b.txt", "T").unwrap());
        assert_eq!(OnConflict::parse(None, OnConflict::Error, "T").unwrap(), OnConflict::Error);
        assert!(OnConflict::parse(Some("replace"), OnConflict::Error, "T").is_err());
    }
}

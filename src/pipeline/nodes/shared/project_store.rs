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

    /// A durable FileRef for bytes in a local file, digested by streaming.
    pub fn file_ref_from_file(
        &self,
        rel_path: &str,
        filename: &str,
        mime: &str,
        path: &std::path::Path,
        origin: &str,
        trust: &str,
        code: &'static str,
    ) -> Result<Value, PipelineError> {
        super::file_ref::durable_file_ref_from_file(self.backend, &self.id, rel_path, filename, mime, path, origin, trust)
            .map_err(|err| PipelineError::new(code, format!("digest '{rel_path}': {err}")))
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

/// The store and key a source value names: a FileRef's own `store` and `ref`
/// (the FileRef validated first — one without `store` is refused, never read
/// from the default store), or a bare store key in the node's own store.
pub fn open_source(
    platform: &Arc<PlatformService>,
    owner: &str,
    project: &str,
    value: &Value,
    node_store: Option<&str>,
) -> Result<Option<(NodeStore, String)>, PipelineError> {
    if super::file_ref::is_file_ref(value) {
        super::file_ref::validate_file_ref(value)?;
    }
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

/// One repository file's bytes, for a node that serves or draws from the
/// project's source (`web.response --file`, `svg.convert repo://`, a site's
/// static assets). The key is normalised (`..`, absolute and empty segments
/// refused), no segment may be a link, and the read is capped
/// (`node-conventions.md` §3).
pub fn read_repo_file(root: &std::path::Path, rel: &str, code: &'static str) -> Result<Vec<u8>, PipelineError> {
    let rel = crate::zebfs::normalize_object_path(rel.trim_start_matches('/'))
        .map_err(|err| PipelineError::new(code, format!("'{rel}': {}", err.message)))?;
    let mut path = root.to_path_buf();
    for segment in rel.split('/') {
        path.push(segment);
        let meta = std::fs::symlink_metadata(&path)
            .map_err(|_| PipelineError::new(code, format!("'{rel}' is not in the project")))?;
        if meta.file_type().is_symlink() {
            return Err(PipelineError::new(code, format!("'{rel}' passes through a link; repository reads never follow one")));
        }
    }
    let meta = std::fs::metadata(&path).map_err(|err| PipelineError::new(code, format!("'{rel}': {err}")))?;
    if !meta.is_file() {
        return Err(PipelineError::new(code, format!("'{rel}' is not a file")));
    }
    if meta.len() > MAX_NODE_OBJECT_BYTES {
        return Err(PipelineError::new(code, format!("'{rel}' is {} bytes, over the {MAX_NODE_OBJECT_BYTES} a node reads at once", meta.len())));
    }
    std::fs::read(&path).map_err(|err| PipelineError::new(code, format!("'{rel}': {err}")))
}

/// [`NodeStore::read_capped`] for a store held without its id.
pub fn read_capped(fs: &ZebFs, key: &str, code: &'static str) -> Result<Vec<u8>, PipelineError> {
    let stat = fs.head(key).map_err(|err| PipelineError::new(code, format!("'{key}': {err}")))?;
    if stat.size > MAX_NODE_OBJECT_BYTES {
        return Err(PipelineError::new(
            code,
            format!("'{key}' is {} bytes, over the {MAX_NODE_OBJECT_BYTES} a node reads at once", stat.size),
        ));
    }
    fs.get(key).map(|object| object.bytes).map_err(|err| PipelineError::new(code, format!("'{key}': {err}")))
}

/// The most a node reads into memory at once (`node-conventions.md` §3).
/// Anything larger streams through a scratch file.
pub const MAX_NODE_OBJECT_BYTES: u64 = 128 * 1024 * 1024;

impl NodeStore {
    /// One object's bytes, refused above [`MAX_NODE_OBJECT_BYTES`] before any
    /// byte is read.
    pub fn read_capped(&self, key: &str, code: &'static str) -> Result<Vec<u8>, PipelineError> {
        read_capped(&self.fs, key, code)
    }

    /// A durable FileRef for an object already in this store, digested by
    /// streaming it rather than holding it.
    pub fn stored_ref(&self, key: &str, origin: &str, trust: &str, code: &'static str) -> Result<Value, PipelineError> {
        let leaf = key.rsplit('/').next().unwrap_or(key).to_string();
        let mime = super::file_ref::mime_for_filename(&leaf);
        let local = self.fs.local_path(key).map_err(|err| PipelineError::new(code, err.to_string()))?;
        let scratch;
        let path = match local {
            Some(path) => path,
            None => {
                scratch = super::store_scratch::StoreScratch::new(code)?;
                let path = scratch.local("object");
                self.fs.get_to_file(key, &path).map_err(|err| PipelineError::new(code, err.to_string()))?;
                path
            }
        };
        self.file_ref_from_file(key, &leaf, mime, &path, origin, trust, code)
    }

    /// Deletes one object the run names (`--delete-source`) and forgets its
    /// exposure rule, so a later object at that key starts private. A failure
    /// fails the node.
    pub fn delete_named(
        &self,
        platform: &PlatformService,
        owner: &str,
        project: &str,
        key: &str,
        code: &'static str,
    ) -> Result<(), PipelineError> {
        self.fs.delete(key).map_err(|err| PipelineError::new(code, format!("delete '{key}': {err}")))?;
        let layout = platform
            .file
            .ensure_project_layout(owner, project)
            .map_err(|err| PipelineError::new(code, err.to_string()))?;
        if layout.store_id() == self.id {
            crate::platform::services::zebfs_acl::forget(&layout.data_store_dir(), key)
                .map_err(|err| PipelineError::new(code, err.to_string()))?;
        }
        Ok(())
    }
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
    // An absolute key is refused, not made relative (`node-conventions.md` §3).
    if raw.starts_with('/') {
        return Err(PipelineError::new(code, format!("invalid destination '{raw}': a store key is relative, without a leading '/'")));
    }
    crate::zebfs::normalize_object_path(&raw)
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
        // Only "not there" means absent; any other store error is an error,
        // never a licence to write.
        let exists = match store.list(folder) {
            Ok(entries) => !entries.is_empty(),
            Err(err) if err.code == "ZEBFS_NOT_FOUND" || err.code == "ZEBFS_NOT_PREFIX" => false,
            Err(err) => return Err(PipelineError::new(code, format!("'{folder}': {err}"))),
        };
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
        let exists = match store.head(key) {
            Ok(_) => true,
            Err(err) if err.code == "ZEBFS_NOT_FOUND" => false,
            Err(err) => return Err(PipelineError::new(code, format!("'{key}': {err}"))),
        };
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
        ..Default::default()
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
        ..Default::default()
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

/// The node kinds whose `store` is saved explicitly when a pipeline is
/// registered: every node that declares `--store` — readers and deleters as
/// well as writers (`node-conventions.md` §3). Read off the definitions, so a
/// new node is pinned by declaring the flag, not by joining a list.
pub fn store_node_kinds() -> &'static std::collections::BTreeSet<String> {
    static KINDS: std::sync::OnceLock<std::collections::BTreeSet<String>> = std::sync::OnceLock::new();
    KINDS.get_or_init(|| {
        crate::pipeline::nodes::builtin_node_definitions()
            .into_iter()
            .filter(|def| def.dsl_flags.iter().any(|flag| flag.flag == "--store"))
            .map(|def| def.kind)
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::{OnConflict, target_key};
    use crate::zebfs::{LocalZebFs, ZebFs};

    #[cfg(unix)]
    #[test]
    fn a_repository_read_never_follows_a_link_or_climbs_out() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("static")).unwrap();
        std::fs::write(dir.path().join("static/a.txt"), b"a").unwrap();
        std::os::unix::fs::symlink("/etc", dir.path().join("static/out")).unwrap();
        assert_eq!(super::read_repo_file(dir.path(), "static/a.txt", "T").unwrap(), b"a");
        assert!(super::read_repo_file(dir.path(), "static/out/hosts", "T").is_err());
        assert!(super::read_repo_file(dir.path(), "../etc/hosts", "T").is_err());
        assert!(super::read_repo_file(dir.path(), "static", "T").is_err());
    }

    #[test]
    fn a_path_wins_and_every_key_is_normalised_once() {
        assert_eq!(target_key(None, "thumbnails", "a.jpg", "T").unwrap(), "thumbnails/a.jpg");
        assert!(target_key(Some("/exact/b.jpg"), "thumbnails", "a.jpg", "T").is_err(), "an absolute key is refused");
        assert_eq!(target_key(Some("exact/b.jpg"), "thumbnails", "a.jpg", "T").unwrap(), "exact/b.jpg");
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

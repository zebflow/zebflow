//! `table.*` — tabular files (CSV, Parquet, …): `table.convert`, `table.query`.

use crate::pipeline::NodeDefinition;

pub mod convert;
pub mod query;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![convert::definition(), query::definition()]
}

use std::sync::Arc;

use serde_json::Value;

use crate::pipeline::PipelineError;
use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType};
use crate::pipeline::nodes::shared::project_store::{
    OnConflict, on_conflict_flag, open_store, store_fields, store_flag, target_key,
};
use crate::platform::services::PlatformService;

/// Where a table node writes its rows (`docs/contracts/node-conventions.md`
/// §1): `--folder` (default `tables`), `--filename` (default a UUID with the
/// format's extension) or an exact `--path`. Any of the three asks for a file.
pub(crate) struct TableDestination<'a> {
    pub folder: &'a str,
    pub filename: Option<&'a str>,
    pub path: Option<&'a str>,
}

impl TableDestination<'_> {
    pub fn requested(&self) -> bool {
        !self.folder.trim().is_empty()
            || self.filename.is_some_and(|name| !name.trim().is_empty())
            || self.path.is_some_and(|path| !path.trim().is_empty())
    }

    /// The store key, given the format word used when no name was set.
    pub fn key(&self, default_format: &str, code: &'static str) -> Result<String, PipelineError> {
        let folder = if self.folder.trim().is_empty() { "tables" } else { self.folder.trim() };
        let filename = match self.filename.map(str::trim).filter(|name| !name.is_empty()) {
            Some(name) => name.to_string(),
            None => format!("{}.{default_format}", uuid::Uuid::new_v4()),
        };
        target_key(self.path, folder, &filename, code)
    }
}

/// Writes encoded rows to the node's store under its collision rule and
/// answers the file's FileRef.
#[allow(clippy::too_many_arguments)]
pub(crate) fn write_table_file(
    platform: &Arc<PlatformService>,
    owner: &str,
    project: &str,
    store: Option<&str>,
    on_conflict: Option<&str>,
    key: &str,
    bytes: Vec<u8>,
    mime: &str,
    origin: &str,
    code: &'static str,
) -> Result<Value, PipelineError> {
    let store = open_store(platform, owner, project, store)?;
    let bytes = if OnConflict::parse(on_conflict, OnConflict::Error, code)?.allows(&store.fs, key, code)? {
        store.fs.put(key, &bytes).map_err(|err| PipelineError::new(code, err.to_string()))?;
        bytes
    } else {
        store.fs.get(key).map_err(|err| PipelineError::new(code, err.to_string()))?.bytes
    };
    let leaf = key.rsplit('/').next().unwrap_or(key).to_string();
    Ok(store.file_ref(key, &leaf, mime, &bytes, origin, "generated"))
}

/// The MIME of a table format word.
pub(crate) fn table_mime(format: &str) -> &'static str {
    match format {
        "csv" => "text/csv",
        "json" => "application/json",
        "ndjson" => "application/x-ndjson",
        "parquet" => "application/vnd.apache.parquet",
        _ => "application/octet-stream",
    }
}

/// `--folder`, `--filename`, `--path`, `--store`, `--on-conflict`.
pub(crate) fn destination_flags() -> Vec<DslFlag> {
    let flag = |flag: &str, key: &str, description: &str| DslFlag {
        flag: flag.to_string(),
        config_key: key.to_string(),
        description: description.to_string(),
        kind: DslFlagKind::Scalar,
        required: false,
    };
    vec![
        flag("--folder", "folder", "Store folder for the written file (default: tables)."),
        flag("--filename", "filename", "Name of the written file (default: a UUID with the format's extension)."),
        flag("--path", "path", "Exact store key for the written file; overrides --folder and --filename."),
        store_flag(),
        on_conflict_flag(OnConflict::Error),
    ]
}

pub(crate) fn destination_fields() -> Vec<NodeFieldDef> {
    let text = |name: &str, label: &str, help: &str| NodeFieldDef {
        name: name.to_string(),
        label: label.to_string(),
        field_type: NodeFieldType::Text,
        help: Some(help.to_string()),
        ..Default::default()
    };
    vec![
        text("folder", "Folder", "Store folder for the written file (default: tables). Set any of folder, filename or path to write a file."),
        text("filename", "Filename", "Default: a UUID with the format's extension."),
        text("path", "Path", "Exact store key; overrides folder and filename."),
    ]
    .into_iter()
    .chain(store_fields(OnConflict::Error))
    .collect()
}

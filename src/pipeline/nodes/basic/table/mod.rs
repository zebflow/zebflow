//! `table.*` — tabular files (CSV, Parquet, …): `table.data.convert`, `table.query.run`.

use crate::pipeline::NodeDefinition;

pub mod convert;
pub mod query;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![convert::definition(), query::definition()]
}

use std::sync::Arc;

use serde_json::Value;

use crate::pipeline::PipelineError;
use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType, SelectOptionDef};
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
    if !OnConflict::parse(on_conflict, OnConflict::Error, code)?.allows(&store.fs, key, code)? {
        // Skipped: the answer is the file already there, as it is.
        return store.stored_ref(key, origin, "generated", code);
    }
    store.fs.put(key, &bytes).map_err(|err| PipelineError::new(code, err.to_string()))?;
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

/// The table formats a node reads and writes, as `--parse` and `--format` list them.
pub(crate) const FORMAT_WORDS: &[&str] = &["csv", "json", "ndjson", "parquet"];

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|word| word.to_string()).collect()
}

/// `--format`: what a written file is.
pub(crate) fn format_flag() -> DslFlag {
    DslFlag {
        flag: "--format".to_string(),
        config_key: "format".to_string(),
        description: "What the written file is: csv, json, ndjson or parquet (default: from the destination's extension, else csv).".to_string(),
        kind: DslFlagKind::Scalar,
        choices: words(FORMAT_WORDS),
        ..Default::default()
    }
}

/// `--rows`: the answer carries the rows as well as the file.
pub(crate) fn rows_flag() -> DslFlag {
    DslFlag {
        flag: "--rows".to_string(),
        config_key: "rows".to_string(),
        description: "With a destination, the answer carries the rows as JSON as well as the file (without one it always does).".to_string(),
        kind: DslFlagKind::Bool,
        ..Default::default()
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
        value: "text".to_string(),
        ..Default::default()
    };
    vec![
        flag("--folder", "folder", "Store folder for the written file (default: tables)."),
        flag("--filename", "filename", "Name of the written file (default: a UUID with the format's extension)."),
        flag("--path", "path", "Exact store key for the written file; overrides --folder and --filename."),
        DslFlag { value: "text".to_string(), ..store_flag() },
        DslFlag { choices: words(&["error", "skip", "overwrite"]), ..on_conflict_flag(OnConflict::Error) },
    ]
}

/// A format select: Auto (empty) and the four format words.
pub(crate) fn format_field(name: &str, label: &str, help: &str) -> NodeFieldDef {
    let mut options = vec![SelectOptionDef { value: String::new(), label: "Auto".to_string() }];
    options.extend(FORMAT_WORDS.iter().map(|word| SelectOptionDef { value: word.to_string(), label: word.to_string() }));
    NodeFieldDef {
        name: name.to_string(),
        label: label.to_string(),
        field_type: NodeFieldType::Select,
        options,
        help: Some(help.to_string()),
        ..Default::default()
    }
}

/// The `rows` checkbox.
pub(crate) fn rows_field() -> NodeFieldDef {
    NodeFieldDef {
        name: "rows".to_string(),
        label: "Answer rows".to_string(),
        field_type: NodeFieldType::Checkbox,
        help: Some("With a destination, answer the rows as JSON as well as the file. Without one the rows are always answered.".to_string()),
        default_value: Some(serde_json::json!(false)),
        ..Default::default()
    }
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

//! `fs.archive.create` — files and folders from a project store, bundled
//! into one archive in the node's store.
//!
//! `--from` names each source — a FileRef, an upload, or a store key (a file
//! or a folder) — repeated, or given once as a `{{ [list] }}`; at most
//! [`MAX_SOURCES`]. Every source is read from the store that holds it into
//! one scratch folder, `tar` bundles them there, and the archive is written
//! with the whole destination set. Only `tar.gz` is written.
//!
//! The answer is one key, `archive`: the durable FileRef of the archive
//! (`origin: fs.archive.create`); the rest of the payload stays.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::model::NodeCapability;
use crate::pipeline::nodes::shared::limits::choice;
use crate::pipeline::nodes::shared::project_store::{OnConflict, on_conflict_flag, open_from, open_store, store_fields, store_flag, target_key};
use crate::pipeline::nodes::shared::store_scratch::StoreScratch;
use crate::pipeline::nodes::shared::util::{metadata_scope, with_answer};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeExample, NodeFieldDef, NodeFieldType, SelectOptionDef},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

pub const NODE_KIND: &str = "fs.archive.create";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

/// The store, `tar` and the archive write: the world's side.
pub const CODE: &str = "FW_NODE_FS_ARCHIVE_CREATE";
/// A flag the author set wrong.
pub const CONFIG_CODE: &str = "FW_NODE_FS_ARCHIVE_CREATE_CONFIG";
/// `--from` is missing, over the ceiling, or names nothing usable.
const SOURCE_CODE: &str = "FW_NODE_FS_ARCHIVE_CREATE_SOURCE";

/// The most sources one archive bundles.
pub const MAX_SOURCES: usize = 64;
const FORMATS: &[&str] = &["tar.gz"];
const DEFAULT_FOLDER: &str = "archives";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// The sources: one value, or a list (each a FileRef, an upload or a store key).
    #[serde(default)]
    pub from: Value,
    /// `tar.gz`, the only one written.
    #[serde(default)]
    pub format: String,
    /// The store to write to; saved explicitly at registration.
    #[serde(default)]
    pub store: Option<String>,
    /// Destination folder (default: `archives`).
    #[serde(default)]
    pub folder: String,
    /// Archive name (default: the first source's name); `.tar.gz` is added when missing.
    #[serde(default)]
    pub filename: Option<String>,
    /// Exact store key; overrides `folder` and `filename`.
    #[serde(default)]
    pub path: Option<String>,
    /// `error` (default), `skip` or `overwrite`.
    #[serde(default)]
    pub on_conflict: Option<String>,
}

fn flag(name: &str, key: &str, description: &str, value: &str) -> DslFlag {
    DslFlag {
        flag: name.to_string(),
        config_key: key.to_string(),
        description: description.to_string(),
        kind: DslFlagKind::Scalar,
        required: false,
        value: value.to_string(),
        ..Default::default()
    }
}

fn field(name: &str, label: &str, help: &str) -> NodeFieldDef {
    NodeFieldDef { name: name.to_string(), label: label.to_string(), field_type: NodeFieldType::Text, help: Some(help.to_string()), ..Default::default() }
}

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|word| word.to_string()).collect()
}

pub fn definition() -> NodeDefinition {
    let archive = json!({ "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "archives/export.tar.gz", "filename": "export.tar.gz", "mime": "application/gzip", "kind": "archive", "size": 40211, "sha256": "sha256:…", "lifecycle": "durable", "origin": NODE_KIND, "trust": "generated" });
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem, NodeCapability::Process],
        title: "Archive Create".to_string(),
        description: format!(
            "Bundle files and folders into one tar.gz archive. `--from` names each source — a FileRef, an upload, or a store key (a file or a whole folder) — \
             repeated, or once as a `{{{{ [list] }}}}`; at most {MAX_SOURCES}. Writes under `--folder` (default `archives`) as `--filename` (default: the first \
             source's name; `.tar.gz` is added when missing) or at an exact `--path`; an archive that exists is an error unless `--on-conflict` says otherwise. \
             Adds `archive` — the durable FileRef of the archive (`ref`, `store`, `size`, …, `origin: fs.archive.create`) — and keeps the rest of the payload."
        ),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "archive": {
                    "type": "object",
                    "description": "The durable FileRef of the archive; the store path is `ref`.",
                    "properties": {
                        "ref": { "type": "string" },
                        "store": { "type": "string" },
                        "mime": { "type": "string", "const": "application/gzip" },
                        "kind": { "type": "string", "const": "archive" },
                        "origin": { "type": "string", "const": NODE_KIND }
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
                kind: DslFlagKind::RepeatedList,
                required: true,
                max_repeat: Some(MAX_SOURCES as u32),
                ..flag("--from", "from", "A file or folder to bundle: a FileRef, an upload or a store key (repeat, or one {{ [list] }}).", "file")
            },
            DslFlag { choices: words(FORMATS), ..flag("--format", "format", "The archive format: tar.gz (default, the only one).", "") },
            DslFlag { value: "text".to_string(), ..store_flag() },
            flag("--folder", "folder", "Destination folder (default: archives).", "text"),
            flag("--filename", "filename", "Archive name; .tar.gz is added when missing (default: the first source's name).", "text"),
            flag("--path", "path", "Exact destination key; overrides --folder and --filename.", "text"),
            DslFlag { choices: words(&["error", "skip", "overwrite"]), ..on_conflict_flag(OnConflict::Error) },
        ],
        fields: vec![
            field("from", "From", "What to bundle: {{ input.file }}, a store key such as exports/2026-09, or {{ [input.a, input.b] }}."),
            NodeFieldDef {
                field_type: NodeFieldType::Select,
                default_value: Some(json!("tar.gz")),
                options: FORMATS.iter().map(|v| SelectOptionDef { value: v.to_string(), label: v.to_string() }).collect(),
                ..field("format", "Format", "The archive format.")
            },
            field("folder", "Folder", "Destination folder (default: archives)."),
            field("filename", "Filename", "Archive name (default: the first source's name)."),
            field("path", "Path", "Exact destination key; overrides folder and filename."),
        ]
        .into_iter()
        .chain(store_fields(OnConflict::Error))
        .collect(),
        layout: ["from", "format", "folder", "filename", "path", "store", "on_conflict"].iter().map(|name| LayoutItem::Field(name.to_string())).collect(),
        examples: vec![
            NodeExample::dsl("Bundle an export folder", "fs.archive.create --from exports/2026-09 --filename export")
                .input(json!({ "month": "2026-09" }))
                .output(json!({ "month": "2026-09", "archive": archive.clone() })),
            NodeExample::dsl("Bundle two stored files", "fs.archive.create --from \"{{ input.report }}\" --from \"{{ input.file }}\" --filename export.tar.gz")
                .note("Each --from is a FileRef (read from the store it names) or a store key in --store; `--from \"{{ [input.report, input.file] }}\"` is the same."),
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

    /// Every `--from` value, a list given as one value opened out.
    fn sources(&self) -> Result<Vec<&Value>, PipelineError> {
        let mut sources = Vec::new();
        let given: Vec<&Value> = match &self.config.from {
            Value::Array(items) => items.iter().collect(),
            Value::Null => Vec::new(),
            one => vec![one],
        };
        for value in given {
            match value {
                Value::Array(items) => sources.extend(items.iter()),
                Value::Null => {}
                Value::String(s) if s.trim().is_empty() => {}
                one => sources.push(one),
            }
        }
        if sources.is_empty() {
            return Err(PipelineError::new(SOURCE_CODE, "--from is required: a file or folder to bundle"));
        }
        if sources.len() > MAX_SOURCES {
            return Err(PipelineError::new(SOURCE_CODE, format!("--from names {} sources; one archive takes at most {MAX_SOURCES}", sources.len())));
        }
        Ok(sources)
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
        choice(&self.config.format, FORMATS, "tar.gz", "--format", CONFIG_CODE)?;
        let on_conflict = OnConflict::parse(self.config.on_conflict.as_deref(), OnConflict::Error, CONFIG_CODE)?;
        let sources = self.sources()?;

        // Every source is read from the store that holds it (a FileRef's own
        // `store`, or this node's store for a bare key) into one scratch
        // folder; tar bundles them there and the archive goes to this node's store.
        let scratch = StoreScratch::new(CODE)?;
        let mut source_paths: Vec<String> = Vec::new();
        let mut source_stores: Vec<String> = Vec::new();
        for value in sources {
            let (source_store, rel) = open_from(&self.platform, owner, project, value, self.config.store.as_deref(), "--from", SOURCE_CODE)?;
            match source_paths.iter().position(|existing| *existing == rel) {
                // The same file named twice is bundled once.
                Some(at) if source_stores[at] == source_store.id => {}
                // One archive path cannot hold two stores' files.
                Some(at) => {
                    return Err(PipelineError::new(
                        SOURCE_CODE,
                        format!("'{rel}' comes from both store '{}' and store '{}'; one archive path holds one file", source_stores[at], source_store.id),
                    ));
                }
                None => {
                    scratch.pull(&source_store.fs, &rel)?;
                    source_paths.push(rel);
                    source_stores.push(source_store.id.clone());
                }
            }
        }

        let archive_name = archive_filename(self.config.filename.as_deref(), &source_paths[0]);
        let folder = if self.config.folder.trim().is_empty() { DEFAULT_FOLDER } else { self.config.folder.trim() };
        let archive_rel = target_key(self.config.path.as_deref(), folder, &archive_name, CONFIG_CODE)?;
        let store = open_store(&self.platform, owner, project, self.config.store.as_deref())?;

        let archive = if on_conflict.allows(&store.fs, &archive_rel, CODE)? {
            let archive_abs = scratch.path().join(".zf-archive").join(&archive_name);
            if let Some(parent) = archive_abs.parent() {
                std::fs::create_dir_all(parent).map_err(|err| PipelineError::new(CODE, format!("create archive parent dir: {err}")))?;
            }
            let archive_abs_for_task = archive_abs.clone();
            let files_root_for_task = scratch.path().to_path_buf();
            let source_rels_for_task = source_paths.clone();
            tokio::task::spawn_blocking(move || compress_tar_gz(&files_root_for_task, &source_rels_for_task, &archive_abs_for_task))
                .await
                .map_err(|err| PipelineError::new(CODE, format!("archive task failed: {err}")))??;
            scratch.push_file(&store.fs, &archive_abs, &archive_rel)?;
            let leaf = archive_rel.rsplit('/').next().unwrap_or(&archive_rel).to_string();
            store.file_ref_from_file(&archive_rel, &leaf, "application/gzip", &archive_abs, NODE_KIND, "generated", CODE)?
        } else {
            // Skipped: the answer is the archive already there, as it is.
            store.stored_ref(&archive_rel, NODE_KIND, "generated", CODE)?
        };
        let size = archive["size"].as_u64().unwrap_or(0);
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: with_answer(&input.payload, json!({ "archive": archive })),
            trace: vec![format!("node_kind={NODE_KIND} srcs={} archive={archive_rel} store={} size={size}", source_paths.join(","), store.id)],
        })
    }
}

fn compress_tar_gz(
    files_root: &Path,
    source_rels: &[String],
    archive_abs: &Path,
) -> Result<(), PipelineError> {
    if source_rels.is_empty() {
        return Err(PipelineError::new(
            CODE,
            "no source paths provided to archive",
        ));
    }
    let mut command = Command::new("tar");
    command
        .arg("-czf")
        .arg(archive_abs)
        .arg("-C")
        .arg(files_root)
        // Keys after `--` are names, never options, whatever they start with.
        .arg("--");
    for source_rel in source_rels {
        command.arg(source_rel);
    }
    let output = command.output().map_err(|err| {
        PipelineError::new(
            CODE,
            format!("failed running tar: {err}"),
        )
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(PipelineError::new(
            CODE,
            format!("tar failed: {stderr}"),
        ));
    }
    Ok(())
}

fn sanitize_filename_stem(name: &str) -> String {
    let stem = Path::new(name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(name);
    let sanitized: String = stem
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    sanitized
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// The archive's name: `--filename` (with `.tar.gz` added when missing), or
/// the first source's name.
fn archive_filename(configured: Option<&str>, source_rel_path: &str) -> String {
    if let Some(name) = configured.map(str::trim).filter(|name| !name.is_empty()) {
        let name = name.rsplit('/').next().unwrap_or(name);
        return if name.ends_with(".tar.gz") { name.to_string() } else { format!("{name}.tar.gz") };
    }
    let source_leaf = Path::new(source_rel_path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("bundle");
    let stem = sanitize_filename_stem(source_leaf);
    format!("{}.tar.gz", if stem.is_empty() { "bundle" } else { &stem })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::nodes::shared::project_store::NodeStore;

    fn platform() -> Arc<PlatformService> {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut config = crate::platform::model::PlatformConfig::default();
        config.data_root = tmp.keep().join("platform");
        Arc::new(PlatformService::from_config(config).expect("platform"))
    }

    fn store(platform: &Arc<PlatformService>) -> NodeStore {
        open_store(platform, "demo", "demo", None).expect("store")
    }

    async fn run(platform: &Arc<PlatformService>, config: Value, payload: Value) -> Result<Value, PipelineError> {
        let node = Node::new(serde_json::from_value(config).expect("config"), platform.clone())?;
        let out = node
            .execute_async(NodeExecutionInput {
                node_id: "n0".to_string(),
                input_pin: "in".to_string(),
                payload,
                metadata: json!({ "owner": "demo", "project": "demo", "pipeline": "test", "request_id": "r1" }),
                bus: None,
            })
            .await?;
        Ok(out.payload)
    }

    #[test]
    fn derives_default_archive_leaf() {
        assert_eq!(archive_filename(None, "pdf/My Paper"), "my-paper.tar.gz");
        assert_eq!(archive_filename(Some("export"), "pdf/x"), "export.tar.gz");
        assert_eq!(archive_filename(Some("export.tar.gz"), "pdf/x"), "export.tar.gz");
    }

    #[tokio::test]
    async fn from_is_required_and_bounded() {
        let p = platform();
        for from in [json!(null), json!([]), json!([""])] {
            let err = run(&p, json!({ "from": from }), json!({})).await.unwrap_err();
            assert_eq!(err.code, SOURCE_CODE);
            assert!(err.message.contains("--from is required"), "{}", err.message);
        }
        let many: Vec<String> = (0..=MAX_SOURCES).map(|n| format!("f/{n}.txt")).collect();
        assert_eq!(run(&p, json!({ "from": many }), json!({})).await.unwrap_err().code, SOURCE_CODE);
        assert_eq!(run(&p, json!({ "from": [{ "name": "x" }] }), json!({})).await.unwrap_err().code, SOURCE_CODE);
        assert_eq!(run(&p, json!({ "from": ["a.txt"], "format": "zip" }), json!({})).await.unwrap_err().code, CONFIG_CODE);
    }

    #[tokio::test]
    async fn repeated_and_listed_sources_answer_one_archive() {
        let p = platform();
        store(&p).fs.put("exports/a.csv", b"a").unwrap();
        store(&p).fs.put("exports/b.csv", b"b").unwrap();
        let file = store(&p).stored_ref("exports/b.csv", "fs.file.put", "generated", "T").unwrap();
        let out = run(&p, json!({ "from": ["exports/a.csv", file], "filename": "two" }), json!({ "keep": true })).await.unwrap();
        let mut keys: Vec<&str> = out.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["archive", "keep"]);
        crate::pipeline::nodes::shared::file_ref::validate_file_ref(&out["archive"]).expect("a contract FileRef");
        assert_eq!((out["archive"]["ref"].as_str(), out["archive"]["origin"].as_str()), (Some("archives/two.tar.gz"), Some(NODE_KIND)));

        // One {{ [list] }} arrives as a list inside the repeated flag's list.
        let out = run(&p, json!({ "from": [["exports/a.csv", "exports/b.csv"]], "path": "out/list.tar.gz" }), json!({})).await.unwrap();
        assert_eq!(out["archive"]["ref"], "out/list.tar.gz");
        // A folder key bundles the folder; an existing archive is an error unless told.
        let out = run(&p, json!({ "from": "exports" }), json!({})).await.unwrap();
        assert_eq!(out["archive"]["ref"], "archives/exports.tar.gz");
        assert!(run(&p, json!({ "from": "exports" }), json!({})).await.unwrap_err().message.contains("already exists"));
    }

    #[test]
    fn the_signature_repeats_from() {
        assert_eq!(
            crate::pipeline::nodes::node_signature(&definition()),
            "fs.archive.create --from FILE… [--format tar.gz] [--store TEXT] [--folder TEXT] [--filename TEXT] [--path TEXT] \
             [--on-conflict error|skip|overwrite] → archive"
        );
    }
}

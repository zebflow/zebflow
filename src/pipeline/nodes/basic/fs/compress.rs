//! n.fs.compress — archive one project file or folder into a compressed bundle.
//!
//! First slice supports only `tar.gz`. The answer, `compressed`, is a bare
//! durable FileRef for the archive — the eleven contract fields and nothing
//! else; the store path is `compressed.ref`.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::pipeline::nodes::shared::project_store::{OnConflict, on_conflict_flag, open_source, open_store, store_fields, store_flag, target_key};
use crate::pipeline::nodes::shared::store_scratch::StoreScratch;

use crate::pipeline::nodes::shared::util::{metadata_scope, resolve_path};
use crate::pipeline::model::NodeCapability;
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType, SelectOptionDef},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

pub const NODE_KIND: &str = "n.fs.compress";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

fn default_source_key() -> String {
    "saved".to_string()
}

fn default_format() -> String {
    "tar.gz".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_source_key")]
    pub source_key: String,
    #[serde(default)]
    pub extra_source_keys: Vec<String>,
    /// Store folder for the archive (default: `archives`).
    #[serde(default)]
    pub folder: String,
    /// Archive name (default: `<source>.tar.gz`).
    #[serde(default)]
    pub filename: Option<String>,
    /// Exact store key; overrides `folder` and `filename`.
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
    /// The store to write to; saved explicitly at registration.
    #[serde(default)]
    pub store: Option<String>,
    /// `overwrite`, `skip` or `error` (default: error).
    #[serde(default)]
    pub on_conflict: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            source_key: default_source_key(),
            extra_source_keys: Vec::new(),
            folder: String::new(),
            filename: None,
            path: None,
            format: default_format(),
            store: None,
            on_conflict: None,
        }
    }
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem, NodeCapability::Process],
        title: "File Compress".to_string(),
        description: "Archive one project file or folder into a compressed bundle. \
            Reads the source at `input.saved` by default (a FileRef or a store path string; `--source-key` names another). \
            Answers `compressed`, a bare durable FileRef for the archive (`ref` is its store path). First slice supports only tar.gz."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "description": "Payload must contain a project-relative file or folder path at the configured source_key."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "compressed": {
                    "type": "object",
                    "description": "A durable FileRef for the archive: the eleven contract fields and nothing else.",
                    "additionalProperties": false,
                    "required": ["__zf_type", "backend", "ref", "filename", "mime", "kind", "size", "sha256", "lifecycle", "origin", "trust"],
                    "properties": {
                        "__zf_type": { "type": "string", "const": "file_ref" },
                        "backend": { "type": "string", "const": "zebfs" },
                        "ref": { "type": "string", "description": "The archive's store path" },
                        "filename": { "type": "string" },
                        "mime": { "type": "string", "const": "application/gzip" },
                        "kind": { "type": "string", "const": "archive" },
                        "size": { "type": "integer" },
                        "sha256": { "type": "string" },
                        "lifecycle": { "type": "string", "const": "durable" },
                        "origin": { "type": "string", "const": "fs.compress" },
                        "trust": { "type": "string", "const": "generated" }
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
                flag: "--source-key".to_string(),
                config_key: "source_key".to_string(),
                description: "Dot-path to the source in the payload: a FileRef or a store path string (default: `saved`, what `fs.save` answers)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--extra-source-keys".to_string(),
                config_key: "extra_source_keys".to_string(),
                description: "Comma-separated extra payload keys to include in the same archive".to_string(),
                kind: DslFlagKind::CommaSeparatedList,
                required: false,
            },
            DslFlag {
                flag: "--folder".to_string(),
                config_key: "folder".to_string(),
                description: "Store folder for the archive (default: archives)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--filename".to_string(),
                config_key: "filename".to_string(),
                description: "Archive name; `.tar.gz` is added when missing (default: <source>.tar.gz)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--path".to_string(),
                config_key: "path".to_string(),
                description: "Exact store key for the archive; overrides --folder and --filename".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--format".to_string(),
                config_key: "format".to_string(),
                description: "Archive format. First slice supports only tar.gz".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            store_flag(),
            on_conflict_flag(OnConflict::Error),
        ],
        fields: vec![
            NodeFieldDef {
                name: "source_key".to_string(),
                label: "Source Key".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Dot-path to the project-relative source path in the input payload.".to_string()),
                default_value: Some(json!("saved")),
                ..Default::default()
            },
            NodeFieldDef {
                name: "extra_source_keys".to_string(),
                label: "Extra Source Keys".to_string(),
                field_type: NodeFieldType::Text,
                help: Some(
                    "Comma-separated extra payload keys whose file/folder paths should be included in the same archive."
                        .to_string(),
                ),
                ..Default::default()
            },
            NodeFieldDef {
                name: "folder".to_string(),
                label: "Folder".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Store folder for the archive (default: archives).".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "filename".to_string(),
                label: "Filename".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Archive name (default: <source>.tar.gz).".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "format".to_string(),
                label: "Format".to_string(),
                field_type: NodeFieldType::Select,
                default_value: Some(json!("tar.gz")),
                options: vec![SelectOptionDef {
                    label: "tar.gz".to_string(),
                    value: "tar.gz".to_string(),
                }],
                help: Some("Archive format to write. Current runtime supports tar.gz.".to_string()),
                ..Default::default()
            },
        ].into_iter().chain(store_fields(OnConflict::Error)).collect(),
        layout: vec![LayoutItem::Col {
            col: vec![
                LayoutItem::Field("source_key".to_string()),
                LayoutItem::Field("extra_source_keys".to_string()),
                LayoutItem::Field("folder".to_string()),
                LayoutItem::Field("filename".to_string()),
                LayoutItem::Field("format".to_string()),
                LayoutItem::Field("store".to_string()),
                LayoutItem::Field("on_conflict".to_string()),
            ],
        }],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Archive an export folder", "fs.compress --source-key export.folder --filename export.tar.gz")
                .input(serde_json::json!({ "export": { "folder": "exports/2026-09" } }))
                .output(serde_json::json!({ "export": { "folder": "exports/2026-09" }, "compressed": { "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "archives/export.tar.gz", "filename": "export.tar.gz", "mime": "application/gzip", "kind": "archive", "size": 40211, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.compress", "trust": "generated" } })),
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

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let (owner, project, ..) = metadata_scope(&input.metadata)?;
        validate_format(&self.config.format)?;

        let source_key = if self.config.source_key.trim().is_empty() {
            "saved"
        } else {
            self.config.source_key.trim()
        };

        // Every source is read from the store that holds it (a FileRef's own
        // `store`, or this node's store for a bare key) into one scratch
        // folder; tar archives there and the archive goes to this node's store.
        let mut keys = vec![source_key.to_string()];
        keys.extend(
            self.config
                .extra_source_keys
                .iter()
                .map(|key| key.trim().to_string())
                .filter(|key| !key.is_empty()),
        );
        let scratch = StoreScratch::new("FW_NODE_FILE_COMPRESS")?;
        let mut source_paths: Vec<String> = Vec::new();
        for key in &keys {
            let value = resolve_path(&input.payload, key).ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_FILE_COMPRESS",
                    format!("source path not found at payload key '{key}' — chain after n.fs.save or set --source-key"),
                )
            })?;
            let (source_store, rel) = open_source(&self.platform, owner, project, value, self.config.store.as_deref())?
                .ok_or_else(|| {
                    PipelineError::new(
                        "FW_NODE_FILE_COMPRESS",
                        format!("payload key '{key}' must be a FileRef or a store path string"),
                    )
                })?;
            let rel = sanitize_rel_path(&rel);
            if rel.is_empty() {
                return Err(PipelineError::new(
                    "FW_NODE_FILE_COMPRESS",
                    format!("resolved source path is empty after sanitization for key '{key}'"),
                ));
            }
            if !source_paths.contains(&rel) {
                scratch.pull(&source_store.fs, &rel)?;
                source_paths.push(rel);
            }
        }

        let archive_name = archive_filename(self.config.filename.as_deref(), &source_paths[0]);
        let folder = if self.config.folder.trim().is_empty() { "archives" } else { self.config.folder.trim() };
        let archive_rel = target_key(self.config.path.as_deref(), folder, &archive_name, "FW_NODE_FILE_COMPRESS")?;
        let store = open_store(&self.platform, owner, project, self.config.store.as_deref())?;
        let on_conflict = OnConflict::parse(self.config.on_conflict.as_deref(), OnConflict::Error, "FW_NODE_FILE_COMPRESS")?;

        let archive_bytes = if on_conflict.allows(&store.fs, &archive_rel, "FW_NODE_FILE_COMPRESS")? {
            let archive_abs = scratch.path().join(".zf-archive").join(&archive_name);
            if let Some(parent) = archive_abs.parent() {
                std::fs::create_dir_all(parent).map_err(|err| {
                    PipelineError::new("FW_NODE_FILE_COMPRESS", format!("create archive parent dir: {err}"))
                })?;
            }
            let archive_abs_for_task = archive_abs.clone();
            let files_root_for_task = scratch.path().to_path_buf();
            let source_rels_for_task = source_paths.clone();
            tokio::task::spawn_blocking(move || {
                compress_tar_gz(&files_root_for_task, &source_rels_for_task, &archive_abs_for_task)
            })
            .await
            .map_err(|err| PipelineError::new("FW_NODE_FILE_COMPRESS", format!("archive task failed: {err}")))??;
            scratch.push_file(&store.fs, &archive_abs, &archive_rel)?
        } else {
            store
                .fs
                .get(&archive_rel)
                .map_err(|err| PipelineError::new("FW_NODE_FILE_COMPRESS", err.to_string()))?
                .bytes
        };
        let size = archive_bytes.len() as u64;

        // `compressed` is a bare durable FileRef for the archive, so `fs.copy
        // --from "{{ input.compressed }}"` or a `--preview` takes it as it is.
        let leaf = archive_rel.rsplit('/').next().unwrap_or(&archive_rel).to_string();
        let compressed = store.file_ref(&archive_rel, &leaf, "application/gzip", &archive_bytes, "fs.compress", "generated");
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: crate::pipeline::nodes::shared::util::with_answer(&input.payload, serde_json::json!({ "compressed": compressed })),
            trace: vec![format!(
                "node_kind={NODE_KIND} srcs={} archive={} store={} size={}",
                source_paths.join(","),
                archive_rel,
                store.id,
                size
            )],
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
            "FW_NODE_FILE_COMPRESS",
            "no source paths provided to archive",
        ));
    }
    let mut command = Command::new("tar");
    command
        .arg("-czf")
        .arg(archive_abs)
        .arg("-C")
        .arg(files_root);
    for source_rel in source_rels {
        command.arg(source_rel);
    }
    let output = command.output().map_err(|err| {
        PipelineError::new(
            "FW_NODE_FILE_COMPRESS",
            format!("failed running tar: {err}"),
        )
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(PipelineError::new(
            "FW_NODE_FILE_COMPRESS",
            format!("tar failed: {stderr}"),
        ));
    }
    Ok(())
}

fn validate_format(format: &str) -> Result<(), PipelineError> {
    if format.trim().eq_ignore_ascii_case("tar.gz") {
        Ok(())
    } else {
        Err(PipelineError::new(
            "FW_NODE_FILE_COMPRESS",
            format!(
                "unsupported archive format '{}'; first slice supports only tar.gz",
                format
            ),
        ))
    }
}

fn sanitize_rel_path(path: &str) -> String {
    path.split('/')
        .filter(|segment| !segment.is_empty() && *segment != "." && *segment != "..")
        .collect::<Vec<_>>()
        .join("/")
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
    use super::{archive_filename, sanitize_rel_path};

    #[test]
    fn sanitizes_source_relative_paths() {
        assert_eq!(sanitize_rel_path("../pdf/./paper"), "pdf/paper");
    }

    #[test]
    fn derives_default_archive_leaf() {
        assert_eq!(archive_filename(None, "pdf/My Paper"), "my-paper.tar.gz");
        assert_eq!(archive_filename(Some("export"), "pdf/x"), "export.tar.gz");
        assert_eq!(archive_filename(Some("export.tar.gz"), "pdf/x"), "export.tar.gz");
    }
}

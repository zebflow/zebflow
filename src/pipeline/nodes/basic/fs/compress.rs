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

use crate::pipeline::nodes::shared::file_ref::{durable_file_ref, zebfs_rel_path_or_string};
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
    #[serde(default)]
    pub output_path: String,
    #[serde(default = "default_format")]
    pub format: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            source_key: default_source_key(),
            extra_source_keys: Vec::new(),
            output_path: String::new(),
            format: default_format(),
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
                flag: "--output-path".to_string(),
                config_key: "output_path".to_string(),
                description: "Destination ZebFS object path. Defaults to archives/<source>.tar.gz".to_string(),
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
                name: "output_path".to_string(),
                label: "Output Path".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Destination ZebFS object path. Leave blank to use archives/<source>.tar.gz.".to_string()),
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
        ],
        layout: vec![LayoutItem::Col {
            col: vec![
                LayoutItem::Field("source_key".to_string()),
                LayoutItem::Field("extra_source_keys".to_string()),
                LayoutItem::Field("output_path".to_string()),
                LayoutItem::Field("format".to_string()),
            ],
        }],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Archive an export folder", "fs.compress --source-key export.folder --output-path archives/export.tar.gz")
                .input(serde_json::json!({ "export": { "folder": "exports/2026-09" } }))
                .output(serde_json::json!({ "compressed": { "__zf_type": "file_ref", "backend": "zebfs", "ref": "archives/export.tar.gz", "filename": "export.tar.gz", "mime": "application/gzip", "kind": "archive", "size": 40211, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.compress", "trust": "generated" } })),
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

        let primary_source_rel = resolve_path(&input.payload, source_key)
            .map(zebfs_rel_path_or_string)
            .transpose()?
            .flatten()
            .ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_FILE_COMPRESS",
                    format!(
                        "source path not found at payload key '{source_key}' — chain after n.fs.save or set --source-key"
                    ),
                )
            })?;

        let primary_source_rel = sanitize_rel_path(&primary_source_rel);
        if primary_source_rel.is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_FILE_COMPRESS",
                "resolved source path is empty after sanitization",
            ));
        }

        let mut source_paths = vec![primary_source_rel.clone()];
        for extra_key in &self.config.extra_source_keys {
            let extra_key = extra_key.trim();
            if extra_key.is_empty() {
                continue;
            }
            let extra_rel = resolve_path(&input.payload, extra_key)
                .map(zebfs_rel_path_or_string)
                .transpose()?
                .flatten()
                .ok_or_else(|| {
                    PipelineError::new(
                        "FW_NODE_FILE_COMPRESS",
                        format!("extra source path not found at payload key '{extra_key}'"),
                    )
                })?;
            let extra_rel = sanitize_rel_path(&extra_rel);
            if extra_rel.is_empty() {
                return Err(PipelineError::new(
                    "FW_NODE_FILE_COMPRESS",
                    format!(
                        "resolved extra source path is empty after sanitization for key '{extra_key}'"
                    ),
                ));
            }
            if !source_paths.iter().any(|existing| existing == &extra_rel) {
                source_paths.push(extra_rel);
            }
        }

        let layout = self
            .platform
            .file
            .ensure_project_layout(owner, project)
            .map_err(|err| PipelineError::new("FW_NODE_FILE_COMPRESS", err.to_string()))?;

        let mut source_rels_for_task = Vec::with_capacity(source_paths.len());
        for source_rel in &source_paths {
            let source_abs = layout.files_dir.join(source_rel);
            if !source_abs.exists() {
                return Err(PipelineError::new(
                    "FW_NODE_FILE_COMPRESS",
                    format!("source path not found: {source_rel}"),
                ));
            }
            source_rels_for_task.push(source_rel.clone());
        }

        let archive_rel = resolve_archive_leaf(&self.config.output_path, &primary_source_rel);
        let archive_abs = layout.files_dir.join(&archive_rel);
        if let Some(parent) = archive_abs.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                PipelineError::new(
                    "FW_NODE_FILE_COMPRESS",
                    format!("create archive parent dir: {err}"),
                )
            })?;
        }

        let archive_abs_for_task = archive_abs.clone();
        let files_root_for_task = layout.files_dir.clone();
        tokio::task::spawn_blocking(move || {
            compress_tar_gz(
                &files_root_for_task,
                &source_rels_for_task,
                &archive_abs_for_task,
            )
        })
        .await
        .map_err(|err| {
            PipelineError::new(
                "FW_NODE_FILE_COMPRESS",
                format!("archive task failed: {err}"),
            )
        })??;

        let size = std::fs::metadata(&archive_abs)
            .map_err(|err| {
                PipelineError::new(
                    "FW_NODE_FILE_COMPRESS",
                    format!("read archive metadata: {err}"),
                )
            })?
            .len();

        // `compressed` is a bare durable FileRef for the archive, so `fs.copy
        // --from "{{ input.compressed }}"` or a `--preview` takes it as it is.
        let compressed = {
            let archive_bytes = std::fs::read(&archive_abs).map_err(|err| {
                PipelineError::new(
                    "FW_NODE_FILE_COMPRESS",
                    format!("read archive for digest: {err}"),
                )
            })?;
            let archive_name = archive_rel.rsplit('/').next().unwrap_or(&archive_rel).to_string();
            durable_file_ref(
                &archive_rel,
                &archive_name,
                "application/gzip",
                &archive_bytes,
                "fs.compress",
                "generated",
            )
        };

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({ "compressed": compressed }),
            trace: vec![format!(
                "node_kind={NODE_KIND} srcs={} archive={} size={}",
                source_paths.join(","),
                archive_rel,
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

fn resolve_archive_leaf(configured: &str, source_rel_path: &str) -> String {
    let configured = sanitize_rel_path(configured);
    if !configured.is_empty() {
        if configured.ends_with(".tar.gz") {
            return configured;
        }
        return format!("{configured}.tar.gz");
    }
    let source_leaf = Path::new(source_rel_path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("bundle");
    let stem = sanitize_filename_stem(source_leaf);
    format!(
        "archives/{}.tar.gz",
        if stem.is_empty() { "bundle" } else { &stem }
    )
}

#[cfg(test)]
mod tests {
    use super::{resolve_archive_leaf, sanitize_rel_path};

    #[test]
    fn sanitizes_source_relative_paths() {
        assert_eq!(sanitize_rel_path("../pdf/./paper"), "pdf/paper");
    }

    #[test]
    fn derives_default_archive_leaf() {
        assert_eq!(
            resolve_archive_leaf("", "pdf/My Paper"),
            "archives/my-paper.tar.gz"
        );
    }
}

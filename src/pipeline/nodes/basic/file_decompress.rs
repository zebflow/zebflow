//! n.file.decompress — extract one archive into project file storage.
//!
//! First slice supports only `tar.gz`.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::util::{metadata_scope, resolve_path};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType, SelectOptionDef},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

pub const NODE_KIND: &str = "n.file.decompress";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

fn default_source_key() -> String {
    "saved.path".to_string()
}

fn default_access() -> String {
    "private".to_string()
}

fn default_format() -> String {
    "tar.gz".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_source_key")]
    pub source_key: String,
    #[serde(default)]
    pub output_dir: String,
    #[serde(default = "default_access")]
    pub access: String,
    #[serde(default = "default_format")]
    pub format: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            source_key: default_source_key(),
            output_dir: String::new(),
            access: default_access(),
            format: default_format(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompressOutput {
    pub source_path: String,
    pub output_dir: String,
    pub format: String,
    pub entries: Vec<String>,
    pub extracted_count: usize,
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "File Decompress".to_string(),
        description: "Extract one archive into project file storage. \
            Reads the source path from `input.saved.path` by default. \
            First slice supports only tar.gz."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "description": "Payload must contain a project-relative archive path at the configured source_key."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "decompressed": {
                    "type": "object",
                    "properties": {
                        "source_path": { "type": "string" },
                        "output_dir": { "type": "string" },
                        "format": { "type": "string" },
                        "extracted_count": { "type": "integer" },
                        "entries": { "type": "array" }
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
                description: "Dot-path to the source archive path in payload (default: saved.path)"
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--output-dir".to_string(),
                config_key: "output_dir".to_string(),
                description:
                    "Destination directory under files/{access}/. Defaults to extracted/<archive>"
                        .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--access".to_string(),
                config_key: "access".to_string(),
                description: "public | private (default: private)".to_string(),
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
                help: Some(
                    "Dot-path to the project-relative archive path in the input payload."
                        .to_string(),
                ),
                default_value: Some(json!("saved.path")),
                ..Default::default()
            },
            NodeFieldDef {
                name: "output_dir".to_string(),
                label: "Output Dir".to_string(),
                field_type: NodeFieldType::Text,
                help: Some(
                    "Destination under files/{access}/. Leave blank to use extracted/<archive>."
                        .to_string(),
                ),
                ..Default::default()
            },
            NodeFieldDef {
                name: "access".to_string(),
                label: "Access".to_string(),
                field_type: NodeFieldType::Select,
                default_value: Some(json!("private")),
                options: vec![
                    SelectOptionDef {
                        label: "Private".to_string(),
                        value: "private".to_string(),
                    },
                    SelectOptionDef {
                        label: "Public".to_string(),
                        value: "public".to_string(),
                    },
                ],
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
                ..Default::default()
            },
        ],
        layout: vec![LayoutItem::Col {
            col: vec![
                LayoutItem::Field("source_key".to_string()),
                LayoutItem::Field("output_dir".to_string()),
                LayoutItem::Field("access".to_string()),
                LayoutItem::Field("format".to_string()),
            ],
        }],
        ai_tool: Default::default(),
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
            "saved.path"
        } else {
            self.config.source_key.trim()
        };

        let source_rel = resolve_path(&input.payload, source_key)
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_FILE_DECOMPRESS",
                    format!(
                        "source path not found at payload key '{source_key}' — chain after n.file.save or set --source-key"
                    ),
                )
            })?;

        let source_rel = sanitize_rel_path(source_rel);
        if source_rel.is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_FILE_DECOMPRESS",
                "resolved source path is empty after sanitization",
            ));
        }

        let layout = self
            .platform
            .file
            .ensure_project_layout(owner, project)
            .map_err(|err| PipelineError::new("FW_NODE_FILE_DECOMPRESS", err.to_string()))?;

        let source_abs = layout.files_dir.join(&source_rel);
        if !source_abs.is_file() {
            return Err(PipelineError::new(
                "FW_NODE_FILE_DECOMPRESS",
                format!("source archive not found: {source_rel}"),
            ));
        }

        let access = normalize_access(&self.config.access);
        let output_leaf = resolve_output_dir(&self.config.output_dir, &source_rel);
        let output_rel = format!("{access}/{output_leaf}");
        let output_abs = layout.files_dir.join(&output_rel);
        std::fs::create_dir_all(&output_abs).map_err(|err| {
            PipelineError::new(
                "FW_NODE_FILE_DECOMPRESS",
                format!("create extract dir: {err}"),
            )
        })?;

        let source_abs_for_task = source_abs.clone();
        let output_abs_for_task = output_abs.clone();
        let entries = tokio::task::spawn_blocking(move || {
            let entries = list_tar_gz_entries(&source_abs_for_task)?;
            extract_tar_gz(&source_abs_for_task, &output_abs_for_task)?;
            Ok::<Vec<String>, PipelineError>(entries)
        })
        .await
        .map_err(|err| {
            PipelineError::new(
                "FW_NODE_FILE_DECOMPRESS",
                format!("decompress task failed: {err}"),
            )
        })??;

        let output = DecompressOutput {
            source_path: source_rel.clone(),
            output_dir: output_rel.clone(),
            format: "tar.gz".to_string(),
            extracted_count: entries.len(),
            entries: entries
                .into_iter()
                .map(|entry| prefixed_output_path(&output_rel, &entry))
                .collect(),
        };

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({ "decompressed": output }),
            trace: vec![format!(
                "node_kind={NODE_KIND} src={} out={} entries={}",
                source_rel, output_rel, output.extracted_count
            )],
        })
    }
}

fn list_tar_gz_entries(source_abs: &Path) -> Result<Vec<String>, PipelineError> {
    let output = Command::new("tar")
        .arg("-tzf")
        .arg(source_abs)
        .output()
        .map_err(|err| {
            PipelineError::new(
                "FW_NODE_FILE_DECOMPRESS",
                format!("failed listing archive entries: {err}"),
            )
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(PipelineError::new(
            "FW_NODE_FILE_DECOMPRESS",
            format!("tar list failed: {stderr}"),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(sanitize_rel_path)
        .filter(|entry| !entry.is_empty())
        .collect())
}

fn extract_tar_gz(source_abs: &Path, output_abs: &Path) -> Result<(), PipelineError> {
    let output = Command::new("tar")
        .arg("-xzf")
        .arg(source_abs)
        .arg("-C")
        .arg(output_abs)
        .output()
        .map_err(|err| {
            PipelineError::new(
                "FW_NODE_FILE_DECOMPRESS",
                format!("failed running tar: {err}"),
            )
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(PipelineError::new(
            "FW_NODE_FILE_DECOMPRESS",
            format!("tar extract failed: {stderr}"),
        ));
    }
    Ok(())
}

fn normalize_access(value: &str) -> &'static str {
    match value.trim() {
        "public" => "public",
        _ => "private",
    }
}

fn validate_format(format: &str) -> Result<(), PipelineError> {
    if format.trim().eq_ignore_ascii_case("tar.gz") {
        Ok(())
    } else {
        Err(PipelineError::new(
            "FW_NODE_FILE_DECOMPRESS",
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

fn sanitize_archive_stem(name: &str) -> String {
    let base = name.strip_suffix(".tar.gz").unwrap_or(name);
    let sanitized: String = base
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

fn resolve_output_dir(configured: &str, source_rel_path: &str) -> String {
    let configured = sanitize_rel_path(configured);
    if !configured.is_empty() {
        return configured;
    }
    let source_leaf = Path::new(source_rel_path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("archive.tar.gz");
    let stem = sanitize_archive_stem(source_leaf);
    format!(
        "extracted/{}",
        if stem.is_empty() { "archive" } else { &stem }
    )
}

fn prefixed_output_path(output_rel_dir: &str, leaf: &str) -> String {
    let leaf = sanitize_rel_path(leaf);
    if leaf.is_empty() {
        output_rel_dir.to_string()
    } else {
        format!("{output_rel_dir}/{leaf}")
    }
}

#[cfg(test)]
mod tests {
    use super::{resolve_output_dir, sanitize_rel_path};

    #[test]
    fn sanitizes_archive_paths() {
        assert_eq!(
            sanitize_rel_path("../private/archives/./bundle.tar.gz"),
            "private/archives/bundle.tar.gz"
        );
    }

    #[test]
    fn derives_default_extract_dir() {
        assert_eq!(
            resolve_output_dir("", "private/archives/paper-bundle.tar.gz"),
            "extracted/paper-bundle"
        );
    }
}

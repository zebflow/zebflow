//! n.fs.decompress — extract one archive into project file storage.
//!
//! First slice supports only `tar.gz`.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::file_ref::zebfs_rel_path_or_string;
use super::util::{metadata_scope, resolve_path};
use crate::pipeline::model::NodeCapability;
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType, SelectOptionDef},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

pub const NODE_KIND: &str = "n.fs.decompress";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

fn default_source_key() -> String {
    "saved.path".to_string()
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
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default)]
    pub delete_source: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            source_key: default_source_key(),
            output_dir: String::new(),
            format: default_format(),
            delete_source: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompressOutput {
    pub source_path: String,
    pub output_dir: String,
    pub root: String,
    pub format: String,
    pub entries: Vec<String>,
    pub extracted_count: usize,
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem, NodeCapability::Process],
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
                description: "Destination ZebFS object directory. Defaults to extracted/<archive>"
                    .to_string(),
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
            DslFlag {
                flag: "--delete-source".to_string(),
                config_key: "delete_source".to_string(),
                description: "Delete the source archive after successful extraction".to_string(),
                kind: DslFlagKind::Bool,
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
                    "Destination ZebFS object directory. Leave blank to use extracted/<archive>."
                        .to_string(),
                ),
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
                help: Some("Archive format to read. Current runtime supports tar.gz.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "delete_source".to_string(),
                label: "Delete Source".to_string(),
                field_type: NodeFieldType::Checkbox,
                help: Some("Delete the source archive after successful extraction.".to_string()),
                default_value: Some(json!(false)),
                ..Default::default()
            },
        ],
        layout: vec![LayoutItem::Col {
            col: vec![
                LayoutItem::Field("source_key".to_string()),
                LayoutItem::Field("output_dir".to_string()),
                LayoutItem::Field("format".to_string()),
                LayoutItem::Field("delete_source".to_string()),
            ],
        }],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Unpack an uploaded archive", "fs.decompress --output-dir imports/latest --delete-source")
                .input(serde_json::json!({ "saved": { "path": "uploads/bundle.tar.gz" } }))
                .output(serde_json::json!({ "decompressed": { "source_path": "uploads/bundle.tar.gz", "output_dir": "imports/latest", "format": "tar.gz", "extracted_count": 3, "entries": ["imports/latest/a.csv", "imports/latest/b.csv", "imports/latest/readme.md"] } })),
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
            "saved.path"
        } else {
            self.config.source_key.trim()
        };

        let source_rel = resolve_path(&input.payload, source_key)
            .map(zebfs_rel_path_or_string)
            .transpose()?
            .flatten()
            .ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_FILE_DECOMPRESS",
                    format!(
                        "source path not found at payload key '{source_key}' — chain after n.fs.save or set --source-key"
                    ),
                )
            })?;

        let source_rel = sanitize_rel_path(&source_rel);
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

        let output_rel = resolve_output_dir(&self.config.output_dir, &source_rel);
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
            // Path safety comes from the listing, before a byte is written --
            // the same order project import uses. `sanitize_rel_path` was
            // applied to the *reported* entries only, so an archive member
            // named `../../../../etc/cron.d/evil` was reported as
            // `etc/cron.d/evil` and extracted where it asked to go.
            refuse_unsafe_archive_entries(&source_abs_for_task)?;
            let entries = list_tar_gz_entries(&source_abs_for_task)?;
            extract_tar_gz(&source_abs_for_task, &output_abs_for_task)?;
            // A member may still be a symlink, which the name check cannot
            // see: `link -> /etc/passwd` is a safe name pointing anywhere. The
            // extract root is refused whole rather than partially trusted.
            refuse_extracted_symlinks(&output_abs_for_task)?;
            Ok::<Vec<String>, PipelineError>(entries)
        })
        .await
        .map_err(|err| {
            PipelineError::new(
                "FW_NODE_FILE_DECOMPRESS",
                format!("decompress task failed: {err}"),
            )
        })??;

        let full_entries: Vec<String> = entries
            .into_iter()
            .map(|entry| prefixed_output_path(&output_rel, &entry))
            .collect();

        // Root = the first top-level directory or file in the archive.
        let root = full_entries
            .first()
            .cloned()
            .unwrap_or_else(|| output_rel.clone());

        // Delete source archive after successful extraction.
        if self.config.delete_source {
            let _ = std::fs::remove_file(&source_abs);
        }

        let output = DecompressOutput {
            source_path: source_rel.clone(),
            output_dir: output_rel.clone(),
            root: root.clone(),
            format: "tar.gz".to_string(),
            extracted_count: full_entries.len(),
            entries: full_entries,
        };

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({ "decompressed": output }),
            trace: vec![format!(
                "node_kind={NODE_KIND} src={} out={} root={} entries={}",
                source_rel, output_rel, root, output.extracted_count
            )],
        })
    }
}

/// Refuses an archive whose listing names anything that does not descend from
/// the extract root: an absolute path, a `..` segment, or a backslash.
///
/// Shared with project import (`ProjectTransferService::stage_archive`) so the
/// two cannot drift; only the error type differs.
fn refuse_unsafe_archive_entries(source_abs: &Path) -> Result<(), PipelineError> {
    crate::platform::services::project_transfer::validate_archive_entry_names(source_abs).map_err(
        |err| {
            PipelineError::new(
                "FW_NODE_FILE_DECOMPRESS",
                format!("unsafe archive entry: {}", err.message),
            )
        },
    )
}

/// Refuses the extract root if anything under it is a symbolic link.
///
/// A symlink is the escape the name check cannot see: `link -> /etc/passwd` is
/// an ordinary-looking member, and every later write through it lands outside
/// the project. Detection is the shared walk project import uses; on a refusal
/// the links themselves are unlinked, because a refused extraction that leaves
/// the link on disk has not actually refused anything. Only the links are
/// removed -- the extract directory may be one the project already had, and a
/// hostile archive must not be able to make us delete it.
fn refuse_extracted_symlinks(output_abs: &Path) -> Result<(), PipelineError> {
    let Err(err) = crate::platform::services::project_transfer::refuse_symlinks(output_abs) else {
        return Ok(());
    };
    unlink_symlinks_under(output_abs);
    Err(PipelineError::new(
        "FW_NODE_FILE_DECOMPRESS",
        format!("unsafe archive entry: {}", err.message),
    ))
}

/// Removes every symbolic link under `root`, following none of them.
fn unlink_symlinks_under(root: &Path) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            let _ = std::fs::remove_file(entry.path());
        } else if file_type.is_dir() {
            unlink_symlinks_under(&entry.path());
        }
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
    use std::io::Write;
    use std::sync::Arc;

    use serde_json::json;

    use super::{
        Config, Node, NodeExecutionInput, NodeHandler, refuse_extracted_symlinks,
        refuse_unsafe_archive_entries, resolve_output_dir, sanitize_rel_path,
    };
    use crate::platform::PlatformConfig;
    use crate::platform::services::PlatformService;

    /// Appends one ustar entry. `type_flag` is `b'0'` for a file and `b'2'`
    /// for a symbolic link, whose target is the entry's link name.
    fn append_tar_entry(bytes: &mut Vec<u8>, name: &str, type_flag: u8, link: &str, body: &[u8]) {
        let mut header = [0_u8; 512];
        header[..name.len()].copy_from_slice(name.as_bytes());
        header[100..107].copy_from_slice(b"0000644");
        header[108..115].copy_from_slice(b"0000000");
        header[116..123].copy_from_slice(b"0000000");
        let size = if type_flag == b'2' { 0 } else { body.len() };
        header[124..135].copy_from_slice(format!("{size:011o}").as_bytes());
        header[136..147].copy_from_slice(b"00000000000");
        header[156] = type_flag;
        header[157..157 + link.len()].copy_from_slice(link.as_bytes());
        header[257..262].copy_from_slice(b"ustar");
        header[263..265].copy_from_slice(b"00");
        header[148..156].copy_from_slice(b"        ");
        let sum: u32 = header.iter().map(|byte| u32::from(*byte)).sum();
        header[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
        bytes.extend_from_slice(&header);
        if type_flag != b'2' {
            bytes.extend_from_slice(body);
            bytes.extend(std::iter::repeat_n(0_u8, (512 - body.len() % 512) % 512));
        }
    }

    fn write_tar_gz(path: &std::path::Path, entries: &[(&str, u8, &str, &[u8])]) {
        let mut tar = Vec::new();
        for (name, type_flag, link, body) in entries {
            append_tar_entry(&mut tar, name, *type_flag, link, body);
        }
        tar.extend(std::iter::repeat_n(0_u8, 1024));
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&tar).unwrap();
        std::fs::write(path, encoder.finish().unwrap()).unwrap();
    }

    /// S5 regression. `sanitize_rel_path` was applied to the *reported* entry
    /// list and never to the extraction, so `tar -xzf` was handed the archive
    /// with no member validation at all: a member named
    /// `../../../../etc/cron.d/evil` was written where it asked and reported as
    /// the harmless-looking `etc/cron.d/evil`. Remove the
    /// `refuse_unsafe_archive_entries` call and this passes silently.
    #[test]
    fn an_archive_member_that_climbs_out_is_refused_before_extraction() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("hostile.tar.gz");
        write_tar_gz(
            &archive,
            &[
                ("safe.txt", b'0', "", b"fine"),
                ("../../../../tmp/zebflow-escape", b'0', "", b"owned"),
            ],
        );

        let error = refuse_unsafe_archive_entries(&archive).expect_err("traversal is refused");
        assert_eq!(error.code, "FW_NODE_FILE_DECOMPRESS");
        assert!(error.message.contains("unsafe archive entry"), "{error:?}");

        // And the sanitizer alone would have hidden it.
        assert_eq!(
            sanitize_rel_path("../../../../tmp/zebflow-escape"),
            "tmp/zebflow-escape"
        );
    }

    #[test]
    fn an_absolute_archive_member_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("absolute.tar.gz");
        write_tar_gz(&archive, &[("/etc/cron.d/evil", b'0', "", b"owned")]);
        assert!(refuse_unsafe_archive_entries(&archive).is_err());
    }

    #[test]
    fn an_ordinary_archive_is_accepted() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("ok.tar.gz");
        write_tar_gz(
            &archive,
            &[
                ("bundle/", b'5', "", b""),
                ("bundle/paper.txt", b'0', "", b"content"),
            ],
        );
        refuse_unsafe_archive_entries(&archive).expect("an ordinary archive extracts");
    }

    /// A symlink member has an innocent name, so the listing check cannot see
    /// it; every later write through it lands wherever it points.
    #[test]
    fn a_symlink_member_is_refused_after_extraction_and_unlinked() {
        let dir = tempfile::tempdir().unwrap();
        let extracted = dir.path().join("extracted");
        std::fs::create_dir_all(extracted.join("nested")).unwrap();
        std::fs::write(extracted.join("kept.txt"), "pre-existing").unwrap();
        std::os::unix::fs::symlink("/etc/passwd", extracted.join("nested").join("link")).unwrap();

        let error = refuse_extracted_symlinks(&extracted).expect_err("a symlink is refused");
        assert!(error.message.contains("unsafe archive entry"), "{error:?}");
        assert!(
            !extracted.join("nested").join("link").exists()
                && std::fs::symlink_metadata(extracted.join("nested").join("link")).is_err(),
            "the link survived the refusal"
        );
        assert!(
            extracted.join("kept.txt").is_file(),
            "a hostile archive must not be able to delete what the project already had"
        );

        refuse_extracted_symlinks(&extracted).expect("a tree without links is fine");
    }

    /// S5 regression, through the node itself. `sanitize_rel_path` was applied
    /// to the *reported* entry list and never to the extraction: `tar -xzf` was
    /// handed the archive with no member validation, so a member named
    /// `../../../../etc/...` landed where it asked and was reported under a
    /// harmless-looking name. Delete the `refuse_unsafe_archive_entries` call
    /// in `execute_async` and the escape file appears.
    #[tokio::test]
    async fn the_node_refuses_an_archive_whose_members_climb_out() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut config = PlatformConfig::default();
        config.data_root = tmp.path().join("platform");
        config.default_password = "secret".to_string();
        let platform = Arc::new(PlatformService::from_config(config).expect("platform"));
        let layout = platform
            .file
            .ensure_project_layout("superadmin", "default")
            .expect("layout");

        let source_rel = "uploads/hostile.tar.gz";
        let source_abs = layout.files_dir.join(source_rel);
        std::fs::create_dir_all(source_abs.parent().expect("parent")).expect("parent");
        // Aimed three levels up from `files/extracted/hostile`, which is the
        // project directory itself -- a real place with real consequences.
        write_tar_gz(
            &source_abs,
            &[
                ("safe.txt", b'0', "", b"fine"),
                ("../../../owned.txt", b'0', "", b"owned"),
            ],
        );
        let escape = layout.files_dir.join("../../owned.txt");

        let node = Node::new(Config::default(), Arc::clone(&platform)).expect("node");
        let error = node
            .execute_async(NodeExecutionInput {
                node_id: "n-test".to_string(),
                input_pin: "in".to_string(),
                payload: json!({ "saved": { "path": source_rel } }),
                metadata: json!({
                    "owner": "superadmin",
                    "project": "default",
                    "pipeline": "test",
                    "request_id": "req-1"
                }),
                bus: None,
            })
            .await
            .expect_err("a hostile archive is refused");
        assert_eq!(error.code, "FW_NODE_FILE_DECOMPRESS");
        assert!(
            !escape.exists(),
            "an archive member escaped to {}",
            escape.display()
        );
        // Refused from the listing, so nothing at all was written -- not even
        // the members that came before the hostile one. Some `tar` builds
        // refuse a `..` member themselves, but only after extracting whatever
        // preceded it, which leaves a half-extracted tree and an error message
        // that depends on which `tar` the host happens to ship.
        assert!(
            !layout.files_dir.join("extracted/hostile/safe.txt").exists(),
            "extraction started before the archive was reviewed"
        );
        assert!(error.message.contains("unsafe archive entry"), "{error:?}");

        // A symlink member has a name the listing check cannot fault.
        let linked_rel = "uploads/linked.tar.gz";
        write_tar_gz(
            &layout.files_dir.join(linked_rel),
            &[
                ("payload/readme.txt", b'0', "", b"ordinary"),
                ("payload/passwd", b'2', "/etc/passwd", b""),
            ],
        );
        let error = node
            .execute_async(NodeExecutionInput {
                node_id: "n-test".to_string(),
                input_pin: "in".to_string(),
                payload: json!({ "saved": { "path": linked_rel } }),
                metadata: json!({
                    "owner": "superadmin",
                    "project": "default",
                    "pipeline": "test",
                    "request_id": "req-3"
                }),
                bus: None,
            })
            .await
            .expect_err("a symlink member is refused");
        assert!(error.message.contains("unsafe archive entry"), "{error:?}");
        let link = layout.files_dir.join("extracted/linked/payload/passwd");
        assert!(
            std::fs::symlink_metadata(&link).is_err(),
            "the symlink survived at {}",
            link.display()
        );

        // An ordinary archive still extracts through the same door.
        let ok_rel = "uploads/ok.tar.gz";
        write_tar_gz(
            &layout.files_dir.join(ok_rel),
            &[("bundle/paper.txt", b'0', "", b"content")],
        );
        let output = node
            .execute_async(NodeExecutionInput {
                node_id: "n-test".to_string(),
                input_pin: "in".to_string(),
                payload: json!({ "saved": { "path": ok_rel } }),
                metadata: json!({
                    "owner": "superadmin",
                    "project": "default",
                    "pipeline": "test",
                    "request_id": "req-2"
                }),
                bus: None,
            })
            .await
            .expect("an ordinary archive extracts");
        assert_eq!(output.payload["decompressed"]["extracted_count"], 1);
        assert!(
            layout
                .files_dir
                .join("extracted/ok/bundle/paper.txt")
                .is_file()
        );
    }

    #[test]
    fn sanitizes_archive_paths() {
        assert_eq!(
            sanitize_rel_path("../archives/./bundle.tar.gz"),
            "archives/bundle.tar.gz"
        );
    }

    #[test]
    fn derives_default_extract_dir() {
        assert_eq!(
            resolve_output_dir("", "archives/paper-bundle.tar.gz"),
            "extracted/paper-bundle"
        );
    }
}

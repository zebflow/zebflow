//! fs.archive.extract — extract one archive into project file storage.
//!
//! First slice supports only `tar.gz`.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::pipeline::nodes::shared::store_scratch::StoreScratch;
use crate::pipeline::nodes::shared::project_store::{OnConflict, on_conflict_flag, open_source, open_store, store_fields, store_flag, target_key};
use crate::pipeline::nodes::shared::util::{metadata_scope, resolve_path};
use crate::pipeline::model::NodeCapability;
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType, SelectOptionDef},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

pub const NODE_KIND: &str = "fs.archive.extract";
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
    /// The folder the archive opens into (default: `extracted/<archive>`).
    #[serde(default)]
    pub folder: String,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default)]
    pub delete_source: bool,
    /// The store to write to; saved explicitly at registration.
    #[serde(default)]
    pub store: Option<String>,
    /// `overwrite`, `skip` or `error` when the folder is not empty (default: error).
    #[serde(default)]
    pub on_conflict: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            source_key: default_source_key(),
            folder: String::new(),
            format: default_format(),
            store: None,
            on_conflict: None,
            delete_source: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompressOutput {
    /// The archive that was opened.
    pub source: String,
    /// The folder it opened into, in `store`.
    pub folder: String,
    pub store: String,
    pub format: String,
    /// A durable FileRef for every extracted file.
    pub files: Vec<serde_json::Value>,
    /// Whether `--delete-source` removed the archive.
    pub source_deleted: bool,
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem, NodeCapability::Process],
        title: "File Decompress".to_string(),
        description: "Extract one archive into project file storage. \
            Reads the source at `input.saved` by default — a FileRef (what `fs.save` answers) or a store path string. \
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
                        "source": { "type": "string" },
                        "folder": { "type": "string" },
                        "store": { "type": "string" },
                        "format": { "type": "string" },
                        "files": { "type": "array", "description": "A durable FileRef per extracted file" }
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
                description: "Dot-path to the source archive in the payload: a FileRef or a store path string (default: `saved`)"
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
            DslFlag {
                flag: "--folder".to_string(),
                config_key: "folder".to_string(),
                description: "The folder the archive opens into (default: extracted/<archive>)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
            DslFlag {
                flag: "--format".to_string(),
                config_key: "format".to_string(),
                description: "Archive format. First slice supports only tar.gz".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
            DslFlag {
                flag: "--delete-source".to_string(),
                config_key: "delete_source".to_string(),
                description: "Delete the source archive after successful extraction".to_string(),
                kind: DslFlagKind::Bool,
                required: false,
                ..Default::default()
            },
            store_flag(),
            on_conflict_flag(OnConflict::Error),
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
                default_value: Some(json!("saved")),
                ..Default::default()
            },
            NodeFieldDef {
                name: "folder".to_string(),
                label: "Folder".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("The folder the archive opens into (default: extracted/<archive>).".to_string()),
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
        ].into_iter().chain(store_fields(OnConflict::Error)).collect(),
        layout: vec![LayoutItem::Col {
            col: vec![
                LayoutItem::Field("source_key".to_string()),
                LayoutItem::Field("folder".to_string()),
                LayoutItem::Field("format".to_string()),
                LayoutItem::Field("delete_source".to_string()),
                LayoutItem::Field("store".to_string()),
                LayoutItem::Field("on_conflict".to_string()),
            ],
        }],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Unpack an uploaded archive", "fs.archive.extract --folder imports/latest --delete-source")
                .input(serde_json::json!({ "saved": { "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "uploads/bundle.tar.gz", "filename": "bundle.tar.gz", "mime": "application/gzip", "kind": "archive", "size": 40211, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.save", "trust": "untrusted" } }))
                .output(serde_json::json!({ "decompressed": { "source": "uploads/bundle.tar.gz", "folder": "imports/latest", "store": "local", "format": "tar.gz", "files": [{ "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "imports/latest/a.csv", "filename": "a.csv", "mime": "text/csv", "kind": "csv", "size": 120, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.archive.extract", "trust": "untrusted" }] } })),
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

        let source_value = resolve_path(&input.payload, source_key).ok_or_else(|| {
            PipelineError::new(
                "FW_NODE_FS_DECOMPRESS",
                format!("source path not found at payload key '{source_key}' — chain after n.fs.save or set --source-key"),
            )
        })?;
        let (source_store, source_rel) = open_source(&self.platform, owner, project, source_value, self.config.store.as_deref())?
            .ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_FS_DECOMPRESS",
                    format!("payload key '{source_key}' must be a FileRef or a store path string"),
                )
            })?;
        let source_rel = crate::zebfs::normalize_object_path(&source_rel)
            .map_err(|err| PipelineError::new("FW_NODE_FS_DECOMPRESS", format!("source: {}", err.message)))?;
        // Extracted files carry the archive's trust: an uploaded archive's
        // members are as untrusted as the archive.
        let trust = if crate::pipeline::nodes::shared::file_ref::is_file_ref(source_value) {
            source_value.get("trust").and_then(|value| value.as_str()).unwrap_or("untrusted").to_string()
        } else {
            "untrusted".to_string()
        };

        // tar speaks paths and the project's files live in their stores: the
        // archive is pulled into a scratch folder, extracted and checked
        // there, and only the checked files are put into this node's store.
        match source_store.fs.head(&source_rel) {
            Ok(stat) if stat.kind == crate::zebfs::ZebFsEntryKind::Object => {}
            _ => {
                return Err(PipelineError::new(
                    "FW_NODE_FS_DECOMPRESS",
                    format!("source archive not found: {source_rel}"),
                ));
            }
        }
        let folder = resolve_output_dir(&self.config.folder, &source_rel);
        let folder = target_key(Some(&folder), "", "", "FW_NODE_FS_DECOMPRESS")?;
        let store = open_store(&self.platform, owner, project, self.config.store.as_deref())?;
        let on_conflict = OnConflict::parse(self.config.on_conflict.as_deref(), OnConflict::Error, "FW_NODE_FS_DECOMPRESS")?;
        if !on_conflict.allows_tree(&store.fs, &folder, "FW_NODE_FS_DECOMPRESS")? {
            let output = DecompressOutput {
                source: source_rel.clone(),
                folder: folder.clone(),
                store: store.id.clone(),
                format: "tar.gz".to_string(),
                files: Vec::new(),
                source_deleted: false,
            };
            return Ok(NodeExecutionOutput {
                output_pins: vec![OUTPUT_PIN_OUT.to_string()],
                payload: crate::pipeline::nodes::shared::util::with_answer(&input.payload, json!({ "decompressed": output })),
                trace: vec![format!("node_kind={NODE_KIND} src={source_rel} out={folder} skipped=true")],
            });
        }
        // `overwrite` replaces the folder: files from an earlier extraction
        // that this archive does not hold would otherwise stay beside it.
        if on_conflict == OnConflict::Overwrite
            && store.fs.list(&folder).map(|entries| !entries.is_empty()).unwrap_or(false)
        {
            store
                .fs
                .delete(&folder)
                .map_err(|err| PipelineError::new("FW_NODE_FS_DECOMPRESS", format!("clear '{folder}': {err}")))?;
        }
        let scratch = StoreScratch::new("FW_NODE_FS_DECOMPRESS")?;
        let source_abs = scratch.pull(&source_store.fs, &source_rel)?;
        let output_abs = scratch.path().join(".zf-extract");
        std::fs::create_dir_all(&output_abs).map_err(|err| {
            PipelineError::new(
                "FW_NODE_FS_DECOMPRESS",
                format!("create extract dir: {err}"),
            )
        })?;

        let source_abs_for_task = source_abs.clone();
        let output_abs_for_task = output_abs.clone();
        tokio::task::spawn_blocking(move || {
            // Path safety comes from the listing, before a byte is written --
            // the same order project import uses.
            refuse_unsafe_archive_entries(&source_abs_for_task)?;
            extract_tar_gz(&source_abs_for_task, &output_abs_for_task)?;
            // A member may still be a symlink, which the name check cannot
            // see: `link -> /etc/passwd` is a safe name pointing anywhere. The
            // extract root is refused whole rather than partially trusted.
            refuse_extracted_symlinks(&output_abs_for_task)?;
            Ok::<(), PipelineError>(())
        })
        .await
        .map_err(|err| {
            PipelineError::new(
                "FW_NODE_FS_DECOMPRESS",
                format!("decompress task failed: {err}"),
            )
        })??;

        let files = scratch.push_tree_refs(&store, &output_abs, &folder, "fs.archive.extract", &trust)?;

        // The archive goes once every member is written, and a failed delete
        // fails the node.
        let source_deleted = self.config.delete_source;
        if source_deleted {
            source_store.delete_named(&self.platform, owner, project, &source_rel, "FW_NODE_FS_DECOMPRESS")?;
        }

        let count = files.len();
        let output = DecompressOutput {
            source: source_rel.clone(),
            folder: folder.clone(),
            store: store.id.clone(),
            format: "tar.gz".to_string(),
            files,
            source_deleted,
        };

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: crate::pipeline::nodes::shared::util::with_answer(&input.payload, json!({ "decompressed": output })),
            trace: vec![format!(
                "node_kind={NODE_KIND} src={source_rel} out={folder} store={} files={count}",
                store.id
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
                "FW_NODE_FS_DECOMPRESS",
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
        "FW_NODE_FS_DECOMPRESS",
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


fn extract_tar_gz(source_abs: &Path, output_abs: &Path) -> Result<(), PipelineError> {
    let output = Command::new("tar")
        .arg("-xzf")
        .arg(source_abs)
        .arg("-C")
        .arg(output_abs)
        .output()
        .map_err(|err| {
            PipelineError::new(
                "FW_NODE_FS_DECOMPRESS",
                format!("failed running tar: {err}"),
            )
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(PipelineError::new(
            "FW_NODE_FS_DECOMPRESS",
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
            "FW_NODE_FS_DECOMPRESS",
            format!(
                "unsupported archive format '{}'; first slice supports only tar.gz",
                format
            ),
        ))
    }
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
    let configured = configured.trim().trim_matches('/');
    if !configured.is_empty() {
        // Normalised (and `..` refused) by `target_key`, not quietly cleaned.
        return configured.to_string();
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


#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::sync::Arc;

    use serde_json::json;

    use super::{
        Config, Node, NodeExecutionInput, NodeHandler, refuse_extracted_symlinks,
        refuse_unsafe_archive_entries, resolve_output_dir,
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
        assert_eq!(error.code, "FW_NODE_FS_DECOMPRESS");
        assert!(error.message.contains("unsafe archive entry"), "{error:?}");
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
                payload: json!({ "saved": source_rel }),
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
        assert_eq!(error.code, "FW_NODE_FS_DECOMPRESS");
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
                payload: json!({ "saved": linked_rel }),
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
                payload: json!({ "saved": ok_rel }),
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
        assert_eq!(output.payload["decompressed"]["files"].as_array().map(Vec::len), Some(1));
        assert_eq!(output.payload["decompressed"]["files"][0]["ref"], "extracted/ok/bundle/paper.txt");
        assert!(
            layout
                .files_dir
                .join("extracted/ok/bundle/paper.txt")
                .is_file()
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

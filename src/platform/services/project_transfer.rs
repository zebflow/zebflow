//! First-class project portability helpers for export/import archives.
//!
//! The first `0.2.1` slice keeps the contract intentionally simple:
//!
//! - bundle archive: `repo/`, `data/`, `manifest.json`
//! - files archive: `files/`, `manifest.json`
//!
//! Credentials and DB connection metadata remain platform-managed and are not bundled here.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::infra::execution::placement::ProjectRuntimePlacement;
use crate::platform::adapters::file::FileAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    ProjectTransferArtifactKind, ProjectTransferManifest, now_ts, slug_segment,
};
use crate::platform::services::project_config::ZebflowJsonService;

#[derive(Default)]
struct DirectoryStats {
    file_count: u64,
    total_bytes: u64,
}

/// Export/import service for project-scoped portability archives.
pub struct ProjectTransferService {
    file: Arc<dyn FileAdapter>,
    zebflow_cfg: Arc<ZebflowJsonService>,
    artifacts_root: PathBuf,
}

impl ProjectTransferService {
    /// Create a new transfer service. Artifacts are stored under the controller data root.
    pub fn new(
        file: Arc<dyn FileAdapter>,
        zebflow_cfg: Arc<ZebflowJsonService>,
        artifacts_root: PathBuf,
    ) -> Self {
        Self {
            file,
            zebflow_cfg,
            artifacts_root,
        }
    }

    /// Return the durable directory used for one controller-tracked operation.
    pub fn operation_dir(&self, operation_id: &str) -> PathBuf {
        self.artifacts_root.join(operation_id)
    }

    /// Relative path stored in operation records for one generated artifact.
    pub fn artifact_rel_path(
        &self,
        operation_id: &str,
        kind: ProjectTransferArtifactKind,
    ) -> String {
        format!("{operation_id}/{}", kind.archive_name())
    }

    /// Absolute artifact path on disk for one controller-tracked operation.
    pub fn artifact_path(&self, operation_id: &str, kind: ProjectTransferArtifactKind) -> PathBuf {
        self.operation_dir(operation_id).join(kind.archive_name())
    }

    /// Build an export archive for one project and return the embedded manifest.
    pub fn export_project(
        &self,
        owner: &str,
        project: &str,
        kind: ProjectTransferArtifactKind,
        source_office_id: Option<&str>,
        source_controller_id: Option<&str>,
        placement: Option<ProjectRuntimePlacement>,
        output_path: &Path,
    ) -> Result<ProjectTransferManifest, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        if output_path.exists() {
            fs::remove_file(output_path)?;
        }
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let staging = output_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!(
                ".staging-export-{}-{}",
                kind.key(),
                std::process::id()
            ));
        if staging.exists() {
            fs::remove_dir_all(&staging)?;
        }
        fs::create_dir_all(&staging)?;

        let mut manifest = ProjectTransferManifest {
            schema_version: "0.3.0".to_string(),
            owner: owner.clone(),
            project: project.clone(),
            artifact_kind: kind,
            source_office_id: source_office_id.map(ToString::to_string),
            source_controller_id: source_controller_id.map(ToString::to_string),
            exported_at: now_ts(),
            runtime_profile: self.zebflow_cfg.get_runtime_profile(&owner, &project),
            placement,
            repo_file_count: 0,
            data_file_count: 0,
            files_file_count: 0,
            total_bytes: 0,
        };

        match kind {
            ProjectTransferArtifactKind::Bundle => {
                let repo_stats = copy_dir_recursive(&layout.repo_dir, &staging.join("repo"))?;
                let data_stats = copy_dir_recursive(&layout.data_dir, &staging.join("data"))?;
                manifest.repo_file_count = repo_stats.file_count;
                manifest.data_file_count = data_stats.file_count;
                manifest.total_bytes = repo_stats.total_bytes + data_stats.total_bytes;
            }
            ProjectTransferArtifactKind::Files => {
                let files_stats = copy_dir_recursive(&layout.files_dir, &staging.join("files"))?;
                manifest.files_file_count = files_stats.file_count;
                manifest.total_bytes = files_stats.total_bytes;
            }
        }

        fs::write(
            staging.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        create_tar_archive(&staging, output_path)?;
        fs::remove_dir_all(&staging)?;
        Ok(manifest)
    }

    /// Import one bundle/files archive into the current project workspace and return its manifest.
    pub fn import_project(
        &self,
        owner: &str,
        project: &str,
        kind: ProjectTransferArtifactKind,
        archive_path: &Path,
    ) -> Result<ProjectTransferManifest, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;

        let extract_dir = archive_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!(
                ".extract-import-{}-{}",
                kind.key(),
                std::process::id()
            ));
        if extract_dir.exists() {
            fs::remove_dir_all(&extract_dir)?;
        }
        fs::create_dir_all(&extract_dir)?;
        extract_tar_archive(archive_path, &extract_dir)?;

        let manifest: ProjectTransferManifest =
            serde_json::from_slice(&fs::read(extract_dir.join("manifest.json"))?)?;
        if manifest.owner != owner || manifest.project != project {
            fs::remove_dir_all(&extract_dir)?;
            return Err(PlatformError::new(
                "PROJECT_TRANSFER_SCOPE_MISMATCH",
                format!(
                    "archive belongs to {}/{} but target project is {}/{}",
                    manifest.owner, manifest.project, owner, project
                ),
            ));
        }
        if manifest.artifact_kind != kind {
            fs::remove_dir_all(&extract_dir)?;
            return Err(PlatformError::new(
                "PROJECT_TRANSFER_KIND_MISMATCH",
                format!(
                    "archive kind '{}' does not match requested '{}'",
                    manifest.artifact_kind.key(),
                    kind.key()
                ),
            ));
        }

        match kind {
            ProjectTransferArtifactKind::Bundle => {
                replace_directory(&layout.repo_dir, &extract_dir.join("repo"))?;
                replace_directory(&layout.data_dir, &extract_dir.join("data"))?;
            }
            ProjectTransferArtifactKind::Files => {
                replace_directory(&layout.files_dir, &extract_dir.join("files"))?;
            }
        }

        fs::remove_dir_all(&extract_dir)?;
        let _ = self.file.ensure_project_layout(&owner, &project)?;
        Ok(manifest)
    }

    /// Compute archive SHA-256 for operation records and download verification.
    pub fn sha256_hex(&self, path: &Path) -> Result<String, PlatformError> {
        let mut file = fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 8192];
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        Ok(format!("{:x}", hasher.finalize()))
    }
}

fn create_tar_archive(source_dir: &Path, output_path: &Path) -> Result<(), PlatformError> {
    let output = Command::new("tar")
        .arg("-cf")
        .arg(output_path)
        .arg("-C")
        .arg(source_dir)
        .arg(".")
        .output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(PlatformError::new(
        "PROJECT_TRANSFER_ARCHIVE",
        String::from_utf8_lossy(&output.stderr).trim().to_string(),
    ))
}

fn extract_tar_archive(archive_path: &Path, output_dir: &Path) -> Result<(), PlatformError> {
    let output = Command::new("tar")
        .arg("-xf")
        .arg(archive_path)
        .arg("-C")
        .arg(output_dir)
        .output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(PlatformError::new(
        "PROJECT_TRANSFER_EXTRACT",
        String::from_utf8_lossy(&output.stderr).trim().to_string(),
    ))
}

fn replace_directory(target: &Path, source: &Path) -> Result<(), PlatformError> {
    if !source.exists() {
        return Err(PlatformError::new(
            "PROJECT_TRANSFER_IMPORT",
            format!(
                "expected extracted directory '{}' is missing",
                source.display()
            ),
        ));
    }
    if target.exists() {
        fs::remove_dir_all(target)?;
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    copy_dir_recursive(source, target)?;
    Ok(())
}

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<DirectoryStats, PlatformError> {
    let mut stats = DirectoryStats::default();
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if file_type.is_dir() {
            let child = copy_dir_recursive(&source_path, &target_path)?;
            stats.file_count += child.file_count;
            stats.total_bytes += child.total_bytes;
        } else if file_type.is_file() {
            fs::copy(&source_path, &target_path)?;
            let meta = entry.metadata()?;
            stats.file_count += 1;
            stats.total_bytes += meta.len();
        }
    }
    Ok(stats)
}

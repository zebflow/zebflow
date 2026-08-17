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

use crate::contracts::kinds::ProjectBundleContract;
use crate::contracts::{ContractMetadata, read_optional_contract, write_contract};
use crate::infra::execution::placement::ProjectRuntimePlacement;
use crate::platform::adapters::file::FileAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    ProjectTransferArtifactKind, ProjectTransferManifest, now_ts, slug_segment,
};
use crate::platform::sekejap;
use crate::platform::services::project_config::ProjectConfigurationService;
use crate::platform::sqlite_schema;

#[derive(Default)]
struct DirectoryStats {
    file_count: u64,
    total_bytes: u64,
}

/// Export/import service for project-scoped portability archives.
pub struct ProjectTransferService {
    file: Arc<dyn FileAdapter>,
    zebflow_cfg: Arc<ProjectConfigurationService>,
    data_root: PathBuf,
    artifacts_root: PathBuf,
}

impl ProjectTransferService {
    /// Create a new transfer service. Artifacts are stored under the controller data root.
    pub fn new(
        file: Arc<dyn FileAdapter>,
        zebflow_cfg: Arc<ProjectConfigurationService>,
        data_root: PathBuf,
        artifacts_root: PathBuf,
    ) -> Self {
        Self {
            file,
            zebflow_cfg,
            data_root,
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
            owner: owner.clone(),
            project: project.clone(),
            artifact_kind: kind,
            source_office_id: source_office_id.map(ToString::to_string),
            source_controller_id: source_controller_id.map(ToString::to_string),
            exported_at: now_ts(),
            runtime_profile: self.zebflow_cfg.get_runtime_profile(&owner, &project)?,
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

        write_contract::<ProjectBundleContract>(
            &staging.join("manifest.json"),
            ContractMetadata::named(&project),
            manifest.clone(),
        )
        .map_err(|err| {
            PlatformError::new(
                "PROJECT_TRANSFER_MANIFEST_WRITE",
                format!("{} ({})", err, err.category()),
            )
        })?;
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

        let manifest_path = extract_dir.join("manifest.json");
        let manifest = read_optional_contract::<ProjectBundleContract>(&manifest_path)
            .map_err(|err| {
                PlatformError::new(
                    "PROJECT_TRANSFER_MANIFEST_READ",
                    format!("{} ({})", err, err.category()),
                )
            })?
            .ok_or_else(|| {
                PlatformError::new(
                    "PROJECT_TRANSFER_MANIFEST_READ",
                    "archive is missing manifest.json",
                )
            })?
            .spec;
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
                sekejap::apply_schema_from_repo(&self.data_root, &owner, &project)?;
                sqlite_schema::apply_schema_from_repo(&self.data_root, &owner, &project)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::decode_contract;
    use crate::contracts::kinds::{
        DependencyLockNodeBundleSpec, DependencyLockSource, DependencyLockSpec, NodeBundleContract,
    };
    use crate::infra::io::durable::directory_tree_sha256;
    use crate::platform::adapters::file::{FileAdapter, FilesystemFileAdapter};
    use crate::platform::services::{DependencyLockService, LibraryService};

    #[test]
    fn bundle_transfer_preserves_and_resolves_all_dependencies() {
        let root = tempfile::tempdir().unwrap();
        let source_root = root.path().join("source");
        let target_root = root.path().join("target");
        let owner = "owner";
        let project = "portable-project";

        let source_file: Arc<dyn FileAdapter> =
            Arc::new(FilesystemFileAdapter::new(source_root.join("users")));
        source_file.initialize().unwrap();
        let source_config = Arc::new(ProjectConfigurationService::new(source_root.join("users")));
        source_config
            .ensure_initialized(owner, project, "Portable Project")
            .unwrap();
        let library = Arc::new(LibraryService::from_embedded().unwrap());
        let source_lock = DependencyLockService::with_library_service(
            source_root.join("users"),
            Arc::clone(&library),
        );
        source_config
            .enable_rwe_library(owner, project, "zeb/deckgl", "full-9.x", "offline")
            .unwrap();
        let requested_libraries = source_config.get_rwe_libraries(owner, project).unwrap();
        source_lock
            .repair_rwe_libraries(owner, project, &requested_libraries)
            .unwrap();

        let source_layout = source_file.ensure_project_layout(owner, project).unwrap();
        let package_dir = source_layout.data_nodes_dir.join("openai-embedding");
        copy_dir_recursive(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("src/pipeline/nodes/bundled")
                .join("openai-embedding"),
            &package_dir,
        )
        .unwrap();
        let definition_path = package_dir.join("definition.json");
        let document =
            decode_contract::<NodeBundleContract>(&std::fs::read(&definition_path).unwrap())
                .unwrap();
        let mut definitions = document
            .spec
            .nodes
            .iter()
            .map(|node| node.kind.clone())
            .collect::<Vec<_>>();
        definitions.sort();
        let mut lock = source_lock.read(owner, project).unwrap();
        lock.nodes.bundles.insert(
            "project/openai-embedding".to_string(),
            DependencyLockNodeBundleSpec {
                version: document.spec.version,
                source: DependencyLockSource::Project,
                source_id: "project/openai-embedding".to_string(),
                entry: "nodes/openai-embedding/definition.json".to_string(),
                integrity: directory_tree_sha256(&package_dir).unwrap(),
                definitions,
            },
        );
        source_lock.write(owner, project, &lock).unwrap();
        let source_report = source_lock
            .status(owner, project, &requested_libraries)
            .unwrap();
        assert!(source_report.ok, "{source_report:?}");

        let source_transfer = ProjectTransferService::new(
            Arc::clone(&source_file),
            Arc::clone(&source_config),
            source_root.clone(),
            source_root.join("operations"),
        );
        let archive = root.path().join("portable-project.tar");
        source_transfer
            .export_project(
                owner,
                project,
                ProjectTransferArtifactKind::Bundle,
                Some("source-office"),
                None,
                None,
                &archive,
            )
            .unwrap();

        let target_file: Arc<dyn FileAdapter> =
            Arc::new(FilesystemFileAdapter::new(target_root.join("users")));
        target_file.initialize().unwrap();
        let target_config = Arc::new(ProjectConfigurationService::new(target_root.join("users")));
        target_config
            .ensure_initialized(owner, project, "Target Placeholder")
            .unwrap();
        let target_transfer = ProjectTransferService::new(
            Arc::clone(&target_file),
            Arc::clone(&target_config),
            target_root.clone(),
            target_root.join("operations"),
        );
        target_transfer
            .import_project(
                owner,
                project,
                ProjectTransferArtifactKind::Bundle,
                &archive,
            )
            .unwrap();

        let target_lock =
            DependencyLockService::with_library_service(target_root.join("users"), library);
        let target_requested = target_config.get_rwe_libraries(owner, project).unwrap();
        let report = target_lock
            .status(owner, project, &target_requested)
            .unwrap();
        assert!(report.ok, "{report:?}");
        assert_eq!(report.resolved, 2);
        assert_eq!(target_lock.read(owner, project).unwrap(), lock);

        let empty = DependencyLockSpec::default();
        assert_ne!(target_lock.read(owner, project).unwrap(), empty);
    }
}

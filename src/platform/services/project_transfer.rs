//! First-class project portability: one `ProjectBundle` archive format.
//!
//! Every export/import archive is the sealed envelope of
//! `docs/contracts/kinds/project-bundle/README.md`: `manifest.json` at the
//! archive root declares the carried classes (`repo`, `store`, `files`), one
//! tree digest per class, and the `direct.*` lock dependencies whose bytes
//! travel in `carried-dependencies/`. The legacy `bundle`/`files` artifact
//! kinds remain as route surface and select class sets
//! (`ProjectTransferArtifactKind::classes`).
//!
//! Credentials, members, policies, and DB connection metadata live in the
//! platform catalog and are not bundled here (`instance-directory.md` rule 7).

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::contracts::kinds::{
    DEPENDENCY_LOCK_FILE, DependencyLockContract, DependencyLockSource,
    PROJECT_BUNDLE_CARRIED_DEPENDENCIES_DIR, PROJECT_BUNDLE_MANIFEST_FILE,
    PROJECT_CONFIGURATION_FILE, ProjectBundleCarriedDependency, ProjectBundleClass,
    ProjectBundleContract, ProjectBundleCounts, ProjectBundleSpec, ProjectConfigurationContract,
};
use crate::contracts::{
    ContractDocument, ContractMetadata, read_optional_contract, read_optional_contract_yaml,
    write_contract, write_contract_yaml,
};
use crate::infra::io::durable::directory_tree_sha256;
use crate::platform::adapters::file::FileAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    ProjectFileLayout, ProjectTransferArtifactKind, now_ts, recovery_date_stamp, slug_segment,
};
use crate::platform::sekejap;

#[derive(Default)]
struct DirectoryStats {
    file_count: u64,
    total_bytes: u64,
}

/// One verified, extracted archive: everything import needs, before any swap.
#[derive(Debug)]
pub struct ProjectBundleStaging {
    /// Removes the staging directory on every non-crash exit path.
    _guard: tempfile::TempDir,
    /// Extraction root holding the class directories and `manifest.json`.
    pub dir: PathBuf,
    /// The decoded archive-root envelope.
    pub document: ContractDocument<ProjectBundleSpec>,
    /// Verified carried `direct.*` dependencies, ready to restore.
    carried: Vec<StagedCarriedDependency>,
}

impl ProjectBundleStaging {
    /// True when the archive declares this class.
    pub fn carries(&self, class: ProjectBundleClass) -> bool {
        self.document.spec.carries(class)
    }
}

/// One carried dependency matched to its staged bytes and lock entry.
#[derive(Debug)]
struct StagedCarriedDependency {
    name: String,
    /// Package directory relative to `data/hub/`, e.g.
    /// `rwe-libraries/zebflow.deckgl` or `nodes/my-package`.
    package_rel: String,
}

/// One class displaced by an import, recoverable until retention removes it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectImportRecoverySwap {
    /// The replaced class.
    pub class: ProjectBundleClass,
    /// Directory name under `data/recovery/` holding the displaced bytes.
    pub recovery_dir: String,
}

/// What an import did, for operation records and API responses.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectImportOutcome {
    /// The archive's `metadata.name` — provenance, recorded and shown, never
    /// a gate (`kinds/project-bundle/README.md`).
    pub provenance: String,
    /// Classes the archive carried and the import replaced.
    pub classes: Vec<ProjectBundleClass>,
    /// The archive's export timestamp.
    pub exported_at: i64,
    /// Office that produced the archive, when recorded.
    pub source_office_id: Option<String>,
    /// Per-class recovery copies; rollback is the reverse swap from these.
    pub recovery: Vec<ProjectImportRecoverySwap>,
    /// Carried `direct.*` dependencies restored into `data/hub/`.
    pub restored_dependencies: Vec<String>,
}

/// Export/import service for project-scoped portability archives.
pub struct ProjectTransferService {
    file: Arc<dyn FileAdapter>,
    data_root: PathBuf,
    artifacts_root: PathBuf,
}

impl ProjectTransferService {
    /// Create a new transfer service. Artifacts are stored under the controller data root.
    pub fn new(file: Arc<dyn FileAdapter>, data_root: PathBuf, artifacts_root: PathBuf) -> Self {
        Self {
            file,
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

    /// One unique in-flight staging directory under the EPHEMERAL tier.
    ///
    /// Staging lives at `<data-root>/tmp/transfer/{label}-{random}`, never
    /// beside the finished archives in `platform/project-operations/`: the
    /// archives an operation produces are the durable artifact, while a crash
    /// mid-operation leaves its half-copied staging here, where the next
    /// boot's ephemeral wipe removes it (`instance-directory.md` rule 3). The
    /// `TempDir` guard also removes it on every non-crash exit path.
    fn staging_dir(&self, label: &str) -> Result<tempfile::TempDir, PlatformError> {
        let staging_root = self.data_root.join("tmp").join("transfer");
        fs::create_dir_all(&staging_root)?;
        Ok(tempfile::Builder::new()
            .prefix(&format!("{label}-"))
            .tempdir_in(&staging_root)?)
    }

    /// Build an export archive carrying `classes` and return its envelope.
    pub fn export_project(
        &self,
        owner: &str,
        project: &str,
        classes: &[ProjectBundleClass],
        source_office_id: Option<&str>,
        output_path: &Path,
    ) -> Result<ContractDocument<ProjectBundleSpec>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let mut classes = classes.to_vec();
        classes.sort();
        classes.dedup();
        if output_path.exists() {
            fs::remove_file(output_path)?;
        }
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let staging_guard = self.staging_dir("export")?;
        let staging = staging_guard.path().to_path_buf();

        let mut class_digests = BTreeMap::new();
        let mut counts = ProjectBundleCounts::default();
        for class in &classes {
            let source = class_source_dir(&layout, *class);
            let staged = staging.join(class.as_str());
            let stats = copy_dir_recursive(&source, &staged)?;
            let digest = directory_tree_sha256(&staged).map_err(|error| {
                PlatformError::new(
                    "PROJECT_TRANSFER_EXPORT",
                    format!("failed hashing class '{class}': {error}"),
                )
            })?;
            class_digests.insert(*class, digest);
            match class {
                ProjectBundleClass::Repo => counts.repo = Some(stats.file_count),
                ProjectBundleClass::Store => counts.store = Some(stats.file_count),
                ProjectBundleClass::Files => counts.files = Some(stats.file_count),
            }
            counts.total_bytes += stats.total_bytes;
        }

        // Every `direct.*` lock entry travels with its bytes; `hub.*` and
        // `project` entries regenerate from the lock (`distribution.md` §5).
        // The lock lives in `repo/`, so bytes are carried exactly when the
        // repo class that explains them is carried.
        let mut carried_dependencies = Vec::new();
        if classes.contains(&ProjectBundleClass::Repo) {
            for entry in collect_direct_lock_entries(&layout)? {
                let source_dir = layout.data_hub_dir().join(&entry.package_rel);
                let staged = staging
                    .join(PROJECT_BUNDLE_CARRIED_DEPENDENCIES_DIR)
                    .join(&entry.package_rel);
                if !source_dir.is_dir() {
                    return Err(PlatformError::new(
                        "PROJECT_TRANSFER_EXPORT",
                        format!(
                            "lock entry '{}' is direct.* but its installed bytes are missing \
                             at data/hub/{}; a direct dependency cannot be fetched again, so \
                             an export without its bytes would not be whole",
                            entry.name, entry.package_rel
                        ),
                    ));
                }
                let stats = copy_dir_recursive(&source_dir, &staged)?;
                verify_carried_dependency(&staging, &entry).map_err(|error| {
                    PlatformError::new(
                        "PROJECT_TRANSFER_EXPORT",
                        format!(
                            "installed bytes for '{}' do not match zeb.lock; refusing to \
                             export an archive that lies: {}",
                            entry.name, error.message
                        ),
                    )
                })?;
                counts.total_bytes += stats.total_bytes;
                carried_dependencies.push(ProjectBundleCarriedDependency {
                    name: entry.name.clone(),
                    source: entry.source,
                    integrity: entry.integrity.clone(),
                });
            }
            carried_dependencies.sort_by(|left, right| left.name.cmp(&right.name));
        }

        let spec = ProjectBundleSpec {
            classes,
            exported_at: now_ts(),
            source_office_id: source_office_id.map(ToString::to_string),
            class_digests,
            carried_dependencies,
            counts,
        };
        let metadata = ContractMetadata::named(format!("{owner}/{project}"));
        write_contract::<ProjectBundleContract>(
            &staging.join(PROJECT_BUNDLE_MANIFEST_FILE),
            metadata.clone(),
            spec.clone(),
        )
        .map_err(|err| {
            PlatformError::new(
                "PROJECT_TRANSFER_MANIFEST_WRITE",
                format!("{} ({})", err, err.category()),
            )
        })?;
        create_tar_archive(&staging, output_path)?;
        drop(staging_guard);
        Ok(ContractDocument {
            api_version: crate::contracts::CONTRACT_API_VERSION,
            kind: crate::contracts::ContractKind::ProjectBundle.as_str(),
            metadata,
            spec,
        })
    }

    /// Extract one archive into EPHEMERAL staging and verify everything the
    /// contract requires before a project may be touched: entry path safety,
    /// the strict envelope, per-class tree digests, and carried `direct.*`
    /// dependency bytes against the staged lock.
    pub fn stage_archive(
        &self,
        archive_path: &Path,
    ) -> Result<ProjectBundleStaging, PlatformError> {
        let guard = self.staging_dir("import")?;
        let dir = guard.path().to_path_buf();

        // Path safety first, from the archive listing — before extraction, so
        // a hostile entry never lands anywhere at all.
        validate_archive_entry_names(archive_path)?;
        extract_tar_archive(archive_path, &dir)?;
        refuse_symlinks(&dir)?;

        let manifest_path = dir.join(PROJECT_BUNDLE_MANIFEST_FILE);
        let document = read_optional_contract::<ProjectBundleContract>(&manifest_path)
            .map_err(|err| {
                PlatformError::new(
                    "PROJECT_TRANSFER_MANIFEST_READ",
                    format!("{} ({})", err, err.category()),
                )
            })?
            .ok_or_else(|| {
                PlatformError::new(
                    "PROJECT_TRANSFER_MANIFEST_READ",
                    format!("archive is missing {PROJECT_BUNDLE_MANIFEST_FILE}"),
                )
            })?;

        // The manifest declares the archive; payload the manifest does not
        // declare is refused rather than silently ignored.
        for entry in fs::read_dir(&dir)? {
            let name = entry?.file_name();
            let name = name.to_string_lossy();
            let declared = name == PROJECT_BUNDLE_MANIFEST_FILE
                || (name == PROJECT_BUNDLE_CARRIED_DEPENDENCIES_DIR
                    && document.spec.carries(ProjectBundleClass::Repo))
                || ProjectBundleClass::parse(&name)
                    .is_some_and(|class| document.spec.carries(class));
            if !declared {
                return Err(PlatformError::new(
                    "PROJECT_TRANSFER_UNDECLARED",
                    format!("archive entry '{name}' is not declared by the manifest"),
                ));
            }
        }

        // Per-class digest verification: the class swaps as a unit, so it
        // verifies as a unit. An absent directory is an empty class.
        for class in &document.spec.classes {
            let staged = dir.join(class.as_str());
            fs::create_dir_all(&staged)?;
            let actual = directory_tree_sha256(&staged).map_err(|error| {
                PlatformError::new(
                    "PROJECT_TRANSFER_VERIFY",
                    format!("failed hashing staged class '{class}': {error}"),
                )
            })?;
            let expected = document
                .spec
                .class_digests
                .get(class)
                .expect("contract validation guarantees one digest per carried class");
            if &actual != expected {
                return Err(PlatformError::new(
                    "PROJECT_TRANSFER_DIGEST_MISMATCH",
                    format!(
                        "class '{class}' does not match its declared digest: expected \
                         {expected}, archive contains {actual}"
                    ),
                ));
            }
        }

        let carried = verify_staged_carried_dependencies(&dir, &document.spec)?;

        Ok(ProjectBundleStaging {
            _guard: guard,
            dir,
            document,
            carried,
        })
    }

    /// Import one archive into the current project workspace.
    ///
    /// `expected_classes`, when given, pins the legacy route surface: the
    /// archive must carry exactly that class set.
    pub fn import_project(
        &self,
        owner: &str,
        project: &str,
        expected_classes: Option<&[ProjectBundleClass]>,
        archive_path: &Path,
    ) -> Result<ProjectImportOutcome, PlatformError> {
        let staging = self.stage_archive(archive_path)?;
        if let Some(expected) = expected_classes {
            let mut expected = expected.to_vec();
            expected.sort();
            expected.dedup();
            if staging.document.spec.classes != expected {
                return Err(PlatformError::new(
                    "PROJECT_TRANSFER_KIND_MISMATCH",
                    format!(
                        "archive carries classes [{}] but this endpoint imports [{}]",
                        join_classes(&staging.document.spec.classes),
                        join_classes(&expected)
                    ),
                ));
            }
        }
        self.import_staged(owner, project, staging)
    }

    /// Replace each carried class of an existing project with the staged
    /// bytes, one atomic swap per class, displaced bytes into
    /// `data/recovery/{class}-{date}/` (BOUNDED tier).
    ///
    /// The archive's identity is provenance, never a gate: this project is
    /// what gets replaced, and the recovery copy is the safety
    /// (`kinds/project-bundle/README.md`). Classes the archive does not carry
    /// are untouched, which is also why no schema or initial-data step runs
    /// here: a carried store snapshot already contains its applied schema, and
    /// a store the archive does not carry is not this import's to change.
    /// Platform import owns fresh-store auto-initiation.
    pub fn import_staged(
        &self,
        owner: &str,
        project: &str,
        staging: ProjectBundleStaging,
    ) -> Result<ProjectImportOutcome, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let recovery_root = layout.data_recovery_dir();
        fs::create_dir_all(&recovery_root)?;

        // The archive's identity gates nothing, but zebflow.yaml and zeb.lock
        // bind metadata.name to the owning path, so a repo arriving under a
        // different project name is retargeted in staging before it swaps in —
        // the same rewrite the hub project_bundle install performs.
        if staging.carries(ProjectBundleClass::Repo) {
            retarget_staged_repo(
                &staging.dir.join(ProjectBundleClass::Repo.as_str()),
                &project,
            )?;
        }

        let mut recovery = Vec::new();
        for class in &staging.document.spec.classes {
            let target = class_source_dir(&layout, *class);
            let incoming = staging.dir.join(class.as_str());
            let recovery_dir = unique_recovery_dir_name(&recovery_root, class.as_str())?;
            swap_directory(&target, &incoming, &recovery_root.join(&recovery_dir))?;
            if *class == ProjectBundleClass::Store {
                sekejap::evict_project_pool(&self.data_root, &owner, &project);
            }
            recovery.push(ProjectImportRecoverySwap {
                class: *class,
                recovery_dir,
            });
        }

        let mut restored_dependencies = Vec::new();
        for entry in &staging.carried {
            let source = staging
                .dir
                .join(PROJECT_BUNDLE_CARRIED_DEPENDENCIES_DIR)
                .join(&entry.package_rel);
            let target = layout.data_hub_dir().join(&entry.package_rel);
            if target.exists() {
                fs::remove_dir_all(&target)?;
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            move_directory(&source, &target)?;
            restored_dependencies.push(entry.name.clone());
        }

        // The imported repository brings its own zebflow.yaml; scaffold any
        // directory the incoming layout expects but the archive left empty.
        let _ = self.file.ensure_project_layout(&owner, &project)?;
        Ok(ProjectImportOutcome {
            provenance: staging.document.metadata.name.clone(),
            classes: staging.document.spec.classes.clone(),
            exported_at: staging.document.spec.exported_at,
            source_office_id: staging.document.spec.source_office_id.clone(),
            recovery,
            restored_dependencies,
        })
    }

    /// Reverse an import's swaps: each recovery copy moves back, and the
    /// displaced imported bytes become a recovery copy of their own — the
    /// rollback is as survivable as the import was.
    pub fn rollback_import(
        &self,
        owner: &str,
        project: &str,
        entries: &[ProjectImportRecoverySwap],
    ) -> Result<Vec<ProjectImportRecoverySwap>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let recovery_root = layout.data_recovery_dir();

        // Validate everything before moving anything.
        for entry in entries {
            validate_recovery_dir_name(&entry.recovery_dir)?;
            let recovery_path = recovery_root.join(&entry.recovery_dir);
            if !recovery_path.is_dir() {
                return Err(PlatformError::new(
                    "PROJECT_TRANSFER_ROLLBACK",
                    format!(
                        "recovery copy '{}' does not exist under data/recovery/",
                        entry.recovery_dir
                    ),
                ));
            }
        }

        let mut displaced = Vec::new();
        for entry in entries {
            let target = class_source_dir(&layout, entry.class);
            let recovery_path = recovery_root.join(&entry.recovery_dir);
            let displaced_name =
                unique_recovery_dir_name(&recovery_root, &format!("{}-rolled-back", entry.class))?;
            swap_directory(
                &target,
                &recovery_path,
                &recovery_root.join(&displaced_name),
            )?;
            if entry.class == ProjectBundleClass::Store {
                sekejap::evict_project_pool(&self.data_root, &owner, &project);
            }
            displaced.push(ProjectImportRecoverySwap {
                class: entry.class,
                recovery_dir: displaced_name,
            });
        }
        let _ = self.file.ensure_project_layout(&owner, &project)?;
        Ok(displaced)
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

/// The on-disk directory one class covers. The store class is `data/store/`
/// specifically — `data/cache/` never travels and `data/hub/` never travels
/// raw (`kinds/project-bundle/README.md`).
fn class_source_dir(layout: &ProjectFileLayout, class: ProjectBundleClass) -> PathBuf {
    match class {
        ProjectBundleClass::Repo => layout.repo_dir.clone(),
        ProjectBundleClass::Store => layout.data_store_dir(),
        ProjectBundleClass::Files => layout.files_dir.clone(),
    }
}

fn join_classes(classes: &[ProjectBundleClass]) -> String {
    classes
        .iter()
        .map(|class| class.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// One `direct.*` lock entry with the paths needed to carry and verify it.
struct DirectLockEntry {
    name: String,
    source: DependencyLockSource,
    integrity: String,
    /// Package directory relative to `data/hub/` (the first two components of
    /// the lock entry path: `rwe-libraries/{package}` or `nodes/{package}`).
    package_rel: String,
    /// The lock's entry path relative to `data/hub/`.
    entry_rel: String,
    /// How `integrity` was computed: RWE libraries hash the entry file, node
    /// bundles hash the package directory tree.
    integrity_over_tree: bool,
}

/// Reads `repo/zeb.lock` and lists every `direct.*` entry. A missing lock
/// carries nothing; an unreadable lock refuses rather than exporting an
/// archive that silently drops dependencies.
fn collect_direct_lock_entries(
    layout: &ProjectFileLayout,
) -> Result<Vec<DirectLockEntry>, PlatformError> {
    let lock_path = layout.repo_dir.join(DEPENDENCY_LOCK_FILE);
    let Some(document) =
        read_optional_contract::<DependencyLockContract>(&lock_path).map_err(|err| {
            PlatformError::new(
                "PROJECT_TRANSFER_LOCK_READ",
                format!("{} ({})", err, err.category()),
            )
        })?
    else {
        return Ok(Vec::new());
    };
    let mut entries = Vec::new();
    for (name, entry) in &document.spec.rwe.libraries {
        if matches!(
            entry.source,
            DependencyLockSource::DirectFile | DependencyLockSource::DirectNpm
        ) {
            entries.push(DirectLockEntry {
                name: name.clone(),
                source: entry.source,
                integrity: entry.integrity.clone(),
                package_rel: package_rel_of(&entry.entry)?,
                entry_rel: entry.entry.clone(),
                integrity_over_tree: false,
            });
        }
    }
    for (name, entry) in &document.spec.nodes.bundles {
        if entry.source == DependencyLockSource::DirectFile {
            entries.push(DirectLockEntry {
                name: name.clone(),
                source: entry.source,
                integrity: entry.integrity.clone(),
                package_rel: package_rel_of(&entry.entry)?,
                entry_rel: entry.entry.clone(),
                integrity_over_tree: true,
            });
        }
    }
    Ok(entries)
}

/// `rwe-libraries/{package}/…` or `nodes/{package}/…` → the package directory.
fn package_rel_of(entry: &str) -> Result<String, PlatformError> {
    let mut components = entry.split('/');
    match (components.next(), components.next()) {
        (Some(namespace), Some(package))
            if !namespace.is_empty() && !package.is_empty() && components.next().is_some() =>
        {
            Ok(format!("{namespace}/{package}"))
        }
        _ => Err(PlatformError::new(
            "PROJECT_TRANSFER_LOCK_READ",
            format!("lock entry path '{entry}' does not name a package directory"),
        )),
    }
}

/// Verifies one staged carried dependency's bytes against its lock integrity.
fn verify_carried_dependency(staging: &Path, entry: &DirectLockEntry) -> Result<(), PlatformError> {
    let root = staging.join(PROJECT_BUNDLE_CARRIED_DEPENDENCIES_DIR);
    let actual = if entry.integrity_over_tree {
        directory_tree_sha256(&root.join(&entry.package_rel)).map_err(|error| {
            PlatformError::new(
                "PROJECT_TRANSFER_VERIFY",
                format!("failed hashing carried '{}': {error}", entry.name),
            )
        })?
    } else {
        let bytes = fs::read(root.join(&entry.entry_rel)).map_err(|error| {
            PlatformError::new(
                "PROJECT_TRANSFER_VERIFY",
                format!(
                    "carried '{}' is missing its entry file '{}': {error}",
                    entry.name, entry.entry_rel
                ),
            )
        })?;
        format!("sha256:{:x}", Sha256::digest(&bytes))
    };
    if actual != entry.integrity {
        return Err(PlatformError::new(
            "PROJECT_TRANSFER_DIGEST_MISMATCH",
            format!(
                "carried dependency '{}' does not match its lock integrity: expected {}, \
                 bytes are {actual}",
                entry.name, entry.integrity
            ),
        ));
    }
    Ok(())
}

/// Matches every manifest `carried_dependencies` entry to the staged repo's
/// lock and verifies the staged bytes. The lock explains the bytes, so
/// carried bytes without a carried repo are refused.
fn verify_staged_carried_dependencies(
    dir: &Path,
    spec: &ProjectBundleSpec,
) -> Result<Vec<StagedCarriedDependency>, PlatformError> {
    if spec.carried_dependencies.is_empty() {
        return Ok(Vec::new());
    }
    if !spec.carries(ProjectBundleClass::Repo) {
        return Err(PlatformError::new(
            "PROJECT_TRANSFER_CARRIED",
            "archive carries dependency bytes without the repo class whose lock explains them",
        ));
    }
    let lock_path = dir
        .join(ProjectBundleClass::Repo.as_str())
        .join(DEPENDENCY_LOCK_FILE);
    let document = read_optional_contract::<DependencyLockContract>(&lock_path)
        .map_err(|err| {
            PlatformError::new(
                "PROJECT_TRANSFER_LOCK_READ",
                format!("{} ({})", err, err.category()),
            )
        })?
        .ok_or_else(|| {
            PlatformError::new(
                "PROJECT_TRANSFER_CARRIED",
                "archive carries dependency bytes but its repo has no zeb.lock",
            )
        })?;

    let mut staged = Vec::new();
    for carried in &spec.carried_dependencies {
        let entry = if let Some(entry) = document.spec.rwe.libraries.get(&carried.name) {
            DirectLockEntry {
                name: carried.name.clone(),
                source: entry.source,
                integrity: entry.integrity.clone(),
                package_rel: package_rel_of(&entry.entry)?,
                entry_rel: entry.entry.clone(),
                integrity_over_tree: false,
            }
        } else if let Some(entry) = document.spec.nodes.bundles.get(&carried.name) {
            DirectLockEntry {
                name: carried.name.clone(),
                source: entry.source,
                integrity: entry.integrity.clone(),
                package_rel: package_rel_of(&entry.entry)?,
                entry_rel: entry.entry.clone(),
                integrity_over_tree: true,
            }
        } else {
            return Err(PlatformError::new(
                "PROJECT_TRANSFER_CARRIED",
                format!(
                    "manifest carries '{}' but the archived zeb.lock has no such entry",
                    carried.name
                ),
            ));
        };
        if entry.source != carried.source || entry.integrity != carried.integrity {
            return Err(PlatformError::new(
                "PROJECT_TRANSFER_CARRIED",
                format!(
                    "manifest and archived zeb.lock disagree about '{}'",
                    carried.name
                ),
            ));
        }
        verify_carried_dependency(dir, &entry)?;
        staged.push(StagedCarriedDependency {
            name: entry.name,
            package_rel: entry.package_rel,
        });
    }
    Ok(staged)
}

/// Rewrites the staged repo's owned documents to name the destination
/// project. Documents that do not decode stay as they are: they are the
/// user's source, and their errors surface on read exactly as they would in
/// any broken repository.
fn retarget_staged_repo(repo_dir: &Path, project: &str) -> Result<(), PlatformError> {
    let config_path = repo_dir.join(PROJECT_CONFIGURATION_FILE);
    if let Ok(Some(document)) =
        read_optional_contract_yaml::<ProjectConfigurationContract>(&config_path)
        && document.metadata.name != project
    {
        let mut metadata = document.metadata;
        metadata.name = project.to_string();
        write_contract_yaml::<ProjectConfigurationContract>(&config_path, metadata, document.spec)
            .map_err(|err| {
                PlatformError::new(
                    "PROJECT_TRANSFER_RETARGET",
                    format!(
                        "failed retargeting zebflow.yaml: {err} ({})",
                        err.category()
                    ),
                )
            })?;
    }
    let lock_path = repo_dir.join(DEPENDENCY_LOCK_FILE);
    if let Ok(Some(document)) = read_optional_contract::<DependencyLockContract>(&lock_path)
        && document.metadata.name != project
    {
        let mut metadata = document.metadata;
        metadata.name = project.to_string();
        write_contract::<DependencyLockContract>(&lock_path, metadata, document.spec).map_err(
            |err| {
                PlatformError::new(
                    "PROJECT_TRANSFER_RETARGET",
                    format!("failed retargeting zeb.lock: {err} ({})", err.category()),
                )
            },
        )?;
    }
    Ok(())
}

/// `{stem}-{date}`, suffixed `-2`, `-3`, … deterministically when a same-named
/// recovery copy already exists.
fn unique_recovery_dir_name(recovery_root: &Path, stem: &str) -> Result<String, PlatformError> {
    let base = format!("{stem}-{}", recovery_date_stamp());
    let mut candidate = base.clone();
    let mut suffix = 2usize;
    while recovery_root.join(&candidate).exists() {
        candidate = format!("{base}-{suffix}");
        suffix += 1;
        if suffix > 10_000 {
            return Err(PlatformError::new(
                "PROJECT_TRANSFER_RECOVERY",
                format!("cannot allocate a recovery directory name for '{base}'"),
            ));
        }
    }
    Ok(candidate)
}

/// A recovery reference names one directory directly under `data/recovery/`.
fn validate_recovery_dir_name(name: &str) -> Result<(), PlatformError> {
    let ok = !name.is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && name != "."
        && name != ".."
        && !name.chars().any(char::is_control);
    if !ok {
        return Err(PlatformError::new(
            "PROJECT_TRANSFER_ROLLBACK",
            format!("'{name}' is not a recovery directory name"),
        ));
    }
    Ok(())
}

/// One swap: `target` moves to `recovery_path`, `incoming` moves to `target`.
///
/// A failure after the first rename restores the displaced directory, so the
/// project never loses its current class to a half-finished swap.
fn swap_directory(
    target: &Path,
    incoming: &Path,
    recovery_path: &Path,
) -> Result<(), PlatformError> {
    if !incoming.exists() {
        fs::create_dir_all(incoming)?;
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    if !target.exists() {
        fs::create_dir_all(target)?;
    }
    fs::rename(target, recovery_path).map_err(|error| {
        PlatformError::new(
            "PROJECT_TRANSFER_SWAP",
            format!(
                "failed moving '{}' to recovery '{}': {error}",
                target.display(),
                recovery_path.display()
            ),
        )
    })?;
    if let Err(error) = move_directory(incoming, target) {
        let _ = fs::rename(recovery_path, target);
        return Err(error);
    }
    Ok(())
}

/// Moves a directory, preferring one atomic rename; a cross-device source is
/// copied and removed instead.
fn move_directory(source: &Path, target: &Path) -> Result<(), PlatformError> {
    if fs::rename(source, target).is_ok() {
        return Ok(());
    }
    copy_dir_recursive(source, target)?;
    fs::remove_dir_all(source)?;
    Ok(())
}

/// Every archive entry must descend from the extract root: no absolute path,
/// no `..` segment, no backslash. Refused from the listing, before any byte
/// is extracted.
fn validate_archive_entry_names(archive_path: &Path) -> Result<(), PlatformError> {
    let output = Command::new("tar").arg("-tf").arg(archive_path).output()?;
    if !output.status.success() {
        return Err(PlatformError::new(
            "PROJECT_TRANSFER_EXTRACT",
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    let listing = String::from_utf8_lossy(&output.stdout);
    for name in listing.lines().map(str::trim_end).filter(|l| !l.is_empty()) {
        let path = Path::new(name);
        let escapes = name.contains('\\')
            || path.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            });
        if escapes {
            return Err(PlatformError::new(
                "PROJECT_TRANSFER_PATH_ESCAPE",
                format!("archive entry '{name}' does not descend from the extract root"),
            ));
        }
    }
    Ok(())
}

/// Class digests refuse symlinks, and no export produces one, so an archive
/// containing any symlink was not produced by export and is refused whole.
fn refuse_symlinks(root: &Path) -> Result<(), PlatformError> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(PlatformError::new(
                "PROJECT_TRANSFER_PATH_ESCAPE",
                format!(
                    "archive contains symbolic link '{}'",
                    entry.path().display()
                ),
            ));
        }
        if file_type.is_dir() {
            refuse_symlinks(&entry.path())?;
        }
    }
    Ok(())
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
        DependencyLockArtifactSpec, DependencyLockNodeBundleSpec, DependencyLockSpec,
        NodeBundleContract,
    };
    use crate::platform::adapters::file::{FileAdapter, FilesystemFileAdapter};
    use crate::platform::services::DependencyLockService;
    use crate::platform::services::project_config::ProjectConfigurationService;

    struct TestInstance {
        config: Arc<ProjectConfigurationService>,
        file: Arc<dyn FileAdapter>,
        transfer: ProjectTransferService,
    }

    fn instance(root: &Path) -> TestInstance {
        let config = Arc::new(ProjectConfigurationService::new(root.join("users")));
        let file: Arc<dyn FileAdapter> = Arc::new(FilesystemFileAdapter::new(
            root.join("users"),
            Arc::clone(&config),
        ));
        file.initialize().unwrap();
        let transfer = ProjectTransferService::new(
            Arc::clone(&file),
            root.to_path_buf(),
            root.join("operations"),
        );
        TestInstance {
            config,
            file,
            transfer,
        }
    }

    /// Appends one ustar entry; enough tar to craft a hostile archive.
    fn append_tar_entry(bytes: &mut Vec<u8>, name: &str, content: &[u8]) {
        let mut header = [0_u8; 512];
        header[..name.len()].copy_from_slice(name.as_bytes());
        header[100..107].copy_from_slice(b"0000644");
        header[108..115].copy_from_slice(b"0000000");
        header[116..123].copy_from_slice(b"0000000");
        let size = format!("{:011o}", content.len());
        header[124..135].copy_from_slice(size.as_bytes());
        header[136..147].copy_from_slice(b"00000000000");
        header[156] = b'0';
        header[257..262].copy_from_slice(b"ustar");
        header[263..265].copy_from_slice(b"00");
        // Checksum is computed with the checksum field itself as spaces.
        header[148..156].copy_from_slice(b"        ");
        let sum: u32 = header.iter().map(|byte| u32::from(*byte)).sum();
        let checksum = format!("{sum:06o}\0 ");
        header[148..156].copy_from_slice(checksum.as_bytes());
        bytes.extend_from_slice(&header);
        bytes.extend_from_slice(content);
        let padding = (512 - content.len() % 512) % 512;
        bytes.extend(std::iter::repeat_n(0_u8, padding));
    }

    fn finish_tar(bytes: &mut Vec<u8>) {
        bytes.extend(std::iter::repeat_n(0_u8, 1024));
    }

    #[test]
    fn bundle_transfer_carries_direct_dependencies_and_resolves_them() {
        let root = tempfile::tempdir().unwrap();
        let source_root = root.path().join("source");
        let target_root = root.path().join("target");
        let owner = "owner";
        let project = "portable-project";

        let source = instance(&source_root);
        source
            .config
            .ensure_initialized(owner, project, "Portable Project")
            .unwrap();
        let source_lock = DependencyLockService::new(source_root.join("users"));
        source
            .config
            .enable_rwe_library(owner, project, "zeb/deckgl", "full-9.x", "hub")
            .unwrap();
        let requested_libraries = source.config.get_rwe_libraries(owner, project).unwrap();

        let source_layout = source.file.ensure_project_layout(owner, project).unwrap();
        // Materialize the installed copy a direct-file ingestion would have
        // produced: bytes under `data/hub/rwe-libraries/{package_id}/` plus a
        // `direct.file` lock entry pinning their digest — the reproducibility
        // table's "must carry bytes" row (`distribution.md` §5).
        let library_bundle = b"export const deck = 1;\n";
        let library_dir = source_layout
            .data_hub_dir()
            .join("rwe-libraries/zebflow.deckgl/0.1/runtime");
        std::fs::create_dir_all(&library_dir).unwrap();
        std::fs::write(library_dir.join("deckgl.bundle.mjs"), library_bundle).unwrap();
        source_lock
            .add_rwe_entry(
                owner,
                project,
                "zeb/deckgl",
                DependencyLockArtifactSpec {
                    version: "full-9.x".to_string(),
                    source: DependencyLockSource::DirectFile,
                    source_id: "zebflow.deckgl-0.1.1".to_string(),
                    entry: "rwe-libraries/zebflow.deckgl/0.1/runtime/deckgl.bundle.mjs".to_string(),
                    integrity: format!("sha256:{:x}", Sha256::digest(library_bundle)),
                },
            )
            .unwrap();
        source_lock
            .repair_rwe_libraries(owner, project, &requested_libraries)
            .unwrap();
        let package_dir = source_layout.data_hub_nodes_dir().join("openai-embedding");
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
            "openai-embedding".to_string(),
            DependencyLockNodeBundleSpec {
                version: document.spec.version,
                source: DependencyLockSource::DirectFile,
                source_id: "openai-embedding-local".to_string(),
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

        let archive = root.path().join("portable-project.tar");
        let exported = source
            .transfer
            .export_project(
                owner,
                project,
                ProjectTransferArtifactKind::Bundle.classes(),
                Some("source-office"),
                &archive,
            )
            .unwrap();
        assert_eq!(exported.metadata.name, "owner/portable-project");
        assert_eq!(
            exported.spec.classes,
            vec![ProjectBundleClass::Repo, ProjectBundleClass::Store]
        );
        assert_eq!(exported.spec.carried_dependencies.len(), 2);
        assert_eq!(
            exported.spec.carried_dependencies[0].name,
            "openai-embedding"
        );
        assert_eq!(exported.spec.carried_dependencies[1].name, "zeb/deckgl");
        assert!(exported.spec.counts.repo.is_some());
        assert!(exported.spec.counts.store.is_some());
        assert!(exported.spec.counts.files.is_none());

        let target = instance(&target_root);
        target
            .config
            .ensure_initialized(owner, project, "Target Placeholder")
            .unwrap();
        let outcome = target
            .transfer
            .import_project(
                owner,
                project,
                Some(ProjectTransferArtifactKind::Bundle.classes()),
                &archive,
            )
            .unwrap();
        assert_eq!(outcome.provenance, "owner/portable-project");
        assert_eq!(outcome.recovery.len(), 2);
        assert_eq!(
            outcome.restored_dependencies,
            vec!["openai-embedding".to_string(), "zeb/deckgl".to_string()]
        );

        let target_lock = DependencyLockService::new(target_root.join("users"));
        let target_requested = target.config.get_rwe_libraries(owner, project).unwrap();
        let report = target_lock
            .status(owner, project, &target_requested)
            .unwrap();
        assert!(report.ok, "{report:?}");
        assert_eq!(report.resolved, 2);
        assert_eq!(target_lock.read(owner, project).unwrap(), lock);
        assert_ne!(
            target_lock.read(owner, project).unwrap(),
            DependencyLockSpec::default()
        );
    }

    #[test]
    fn import_swaps_classes_leaves_recovery_and_rollback_reverses() {
        let root = tempfile::tempdir().unwrap();
        let owner = "owner";
        let project = "swap-project";
        let it = instance(root.path());
        it.config
            .ensure_initialized(owner, project, "Swap Project")
            .unwrap();
        let layout = it.file.ensure_project_layout(owner, project).unwrap();
        std::fs::write(layout.repo_dir.join("marker.txt"), b"original").unwrap();
        std::fs::create_dir_all(layout.data_store_dir()).unwrap();
        std::fs::write(layout.data_store_dir().join("row.txt"), b"store-original").unwrap();
        std::fs::write(layout.files_dir.join("object.txt"), b"untouched-object").unwrap();

        let archive = root.path().join("swap.tar");
        it.transfer
            .export_project(
                owner,
                project,
                ProjectTransferArtifactKind::Bundle.classes(),
                None,
                &archive,
            )
            .unwrap();

        std::fs::write(layout.repo_dir.join("marker.txt"), b"mutated").unwrap();
        let outcome = it
            .transfer
            .import_project(
                owner,
                project,
                Some(ProjectTransferArtifactKind::Bundle.classes()),
                &archive,
            )
            .unwrap();
        // The archive's bytes replaced the class...
        assert_eq!(
            std::fs::read(layout.repo_dir.join("marker.txt")).unwrap(),
            b"original"
        );
        // ...the files class was not carried and is untouched...
        assert_eq!(
            std::fs::read(layout.files_dir.join("object.txt")).unwrap(),
            b"untouched-object"
        );
        // ...and the displaced bytes exist in the recovery copy.
        let repo_swap = outcome
            .recovery
            .iter()
            .find(|swap| swap.class == ProjectBundleClass::Repo)
            .unwrap();
        let recovery_marker = layout
            .data_recovery_dir()
            .join(&repo_swap.recovery_dir)
            .join("marker.txt");
        assert_eq!(std::fs::read(&recovery_marker).unwrap(), b"mutated");

        // Rollback is the reverse swap: the recovery copy returns, the
        // imported bytes become recoverable in their own right.
        let displaced = it
            .transfer
            .rollback_import(owner, project, &outcome.recovery)
            .unwrap();
        assert_eq!(
            std::fs::read(layout.repo_dir.join("marker.txt")).unwrap(),
            b"mutated"
        );
        let rolled_back = displaced
            .iter()
            .find(|swap| swap.class == ProjectBundleClass::Repo)
            .unwrap();
        assert_eq!(
            std::fs::read(
                layout
                    .data_recovery_dir()
                    .join(&rolled_back.recovery_dir)
                    .join("marker.txt")
            )
            .unwrap(),
            b"original"
        );
    }

    #[test]
    fn tampered_class_bytes_are_refused_at_staging_and_project_untouched() {
        let root = tempfile::tempdir().unwrap();
        let owner = "owner";
        let project = "tamper-project";
        let it = instance(root.path());
        it.config
            .ensure_initialized(owner, project, "Tamper Project")
            .unwrap();
        let layout = it.file.ensure_project_layout(owner, project).unwrap();
        std::fs::write(layout.repo_dir.join("marker.txt"), b"original").unwrap();

        let archive = root.path().join("tamper.tar");
        it.transfer
            .export_project(
                owner,
                project,
                ProjectTransferArtifactKind::Bundle.classes(),
                None,
                &archive,
            )
            .unwrap();

        // Repack the archive with one flipped class byte, manifest unchanged.
        let unpack = root.path().join("unpack");
        std::fs::create_dir_all(&unpack).unwrap();
        extract_tar_archive(&archive, &unpack).unwrap();
        std::fs::write(unpack.join("repo/marker.txt"), b"tampered").unwrap();
        create_tar_archive(&unpack, &archive).unwrap();

        let error = it
            .transfer
            .import_project(
                owner,
                project,
                Some(ProjectTransferArtifactKind::Bundle.classes()),
                &archive,
            )
            .unwrap_err();
        assert_eq!(error.code, "PROJECT_TRANSFER_DIGEST_MISMATCH");
        assert_eq!(
            std::fs::read(layout.repo_dir.join("marker.txt")).unwrap(),
            b"original"
        );
        assert!(
            !layout.data_recovery_dir().exists()
                || std::fs::read_dir(layout.data_recovery_dir())
                    .unwrap()
                    .next()
                    .is_none(),
            "a refused archive must not displace anything into recovery"
        );
    }

    #[test]
    fn a_path_escape_entry_is_refused_at_staging() {
        let root = tempfile::tempdir().unwrap();
        let it = instance(root.path());
        let mut bytes = Vec::new();
        append_tar_entry(&mut bytes, "../evil.txt", b"escape");
        finish_tar(&mut bytes);
        let archive = root.path().join("evil.tar");
        std::fs::write(&archive, &bytes).unwrap();

        let error = it.transfer.stage_archive(&archive).unwrap_err();
        assert_eq!(error.code, "PROJECT_TRANSFER_PATH_ESCAPE");

        let mut absolute = Vec::new();
        append_tar_entry(&mut absolute, "/tmp/evil.txt", b"escape");
        finish_tar(&mut absolute);
        std::fs::write(&archive, &absolute).unwrap();
        let error = it.transfer.stage_archive(&archive).unwrap_err();
        assert_eq!(error.code, "PROJECT_TRANSFER_PATH_ESCAPE");
    }

    #[test]
    fn a_store_without_repo_archive_is_refused_at_staging() {
        let root = tempfile::tempdir().unwrap();
        let it = instance(root.path());
        // Handcrafted, since the canonical writer refuses to produce this.
        let staging = root.path().join("craft");
        std::fs::create_dir_all(staging.join("store")).unwrap();
        std::fs::write(staging.join("store/row.txt"), b"state").unwrap();
        let digest = directory_tree_sha256(&staging.join("store")).unwrap();
        let manifest = serde_json::json!({
            "apiVersion": "zebflow.com/v1",
            "kind": "ProjectBundle",
            "metadata": { "name": "owner/state-only" },
            "spec": {
                "classes": ["store"],
                "exported_at": 1,
                "class_digests": { "store": digest },
                "carried_dependencies": [],
                "counts": { "store": 1, "total_bytes": 5 }
            }
        });
        std::fs::write(
            staging.join(PROJECT_BUNDLE_MANIFEST_FILE),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let archive = root.path().join("store-only.tar");
        create_tar_archive(&staging, &archive).unwrap();

        let error = it.transfer.stage_archive(&archive).unwrap_err();
        assert!(
            error.message.contains("store"),
            "unexpected refusal: {error:?}"
        );
    }

    #[test]
    fn an_archive_of_the_wrong_class_set_is_refused_by_the_legacy_route_kind() {
        let root = tempfile::tempdir().unwrap();
        let owner = "owner";
        let project = "kind-project";
        let it = instance(root.path());
        it.config
            .ensure_initialized(owner, project, "Kind Project")
            .unwrap();
        let archive = root.path().join("files.tar");
        it.transfer
            .export_project(
                owner,
                project,
                ProjectTransferArtifactKind::Files.classes(),
                None,
                &archive,
            )
            .unwrap();
        let error = it
            .transfer
            .import_project(
                owner,
                project,
                Some(ProjectTransferArtifactKind::Bundle.classes()),
                &archive,
            )
            .unwrap_err();
        assert_eq!(error.code, "PROJECT_TRANSFER_KIND_MISMATCH");
    }

    #[test]
    fn undeclared_archive_payload_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let owner = "owner";
        let project = "extra-project";
        let it = instance(root.path());
        it.config
            .ensure_initialized(owner, project, "Extra Project")
            .unwrap();
        let archive = root.path().join("extra.tar");
        it.transfer
            .export_project(
                owner,
                project,
                ProjectTransferArtifactKind::Files.classes(),
                None,
                &archive,
            )
            .unwrap();
        // Add an undeclared `store/` payload beside the declared class.
        let unpack = root.path().join("unpack");
        std::fs::create_dir_all(unpack.join("store")).unwrap();
        extract_tar_archive(&archive, &unpack).unwrap();
        std::fs::create_dir_all(unpack.join("store")).unwrap();
        std::fs::write(unpack.join("store/sneak.txt"), b"x").unwrap();
        create_tar_archive(&unpack, &archive).unwrap();

        let error = it.transfer.stage_archive(&archive).unwrap_err();
        assert_eq!(error.code, "PROJECT_TRANSFER_UNDECLARED");
    }
}

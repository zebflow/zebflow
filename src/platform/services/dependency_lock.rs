//! Service for the git-tracked `repo/zeb.lock` dependency lock.
//!
//! All mutations are serialized per lock path and use durable atomic file
//! replacement. The pre-contract `{version:1,libraries:{...}}` shape is read
//! only by the explicit migration path, which keeps a recovery copy.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::contracts::kinds::INSTALLED_NODE_KIND_PREFIX;
use crate::contracts::kinds::{
    DEPENDENCY_LOCK_BACKUP_FILE, DEPENDENCY_LOCK_FILE, DependencyLockArtifactSpec,
    DependencyLockNodeBundleSpec, DependencyLockSource, DependencyLockSpec, NodeBundleContract,
    decode_dependency_lock, decode_pipeline_graph, encode_dependency_lock,
};
use crate::contracts::{ContractMetadata, decode_contract};
use crate::infra::io::durable::atomic_write;
use crate::infra::io::durable::directory_tree_sha256;
use crate::pipeline::PipelineGraph;
use crate::platform::error::PlatformError;
use crate::platform::model::{ResolvedProjectLayout, ZebflowJsonRweLibraries, slug_segment};
use crate::platform::services::project_config::ProjectConfigurationService;

/// Result of one successful legacy dependency-lock migration.
#[derive(Debug, Clone)]
pub struct DependencyLockMigration {
    pub source_format: &'static str,
    pub canonical_path: PathBuf,
    pub recovery_path: PathBuf,
}

/// Resolution state shown by APIs and Project Studio.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependencyResolutionStatus {
    Resolved,
    Missing,
    VersionMismatch,
    IntegrityMismatch,
    UnsupportedRuntime,
}

/// One dependency check result.
#[derive(Debug, Clone, Serialize)]
pub struct DependencyStatusItem {
    pub family: &'static str,
    pub name: String,
    pub status: DependencyResolutionStatus,
    pub version: String,
    pub source: String,
    pub message: String,
    pub definitions: Vec<String>,
}

/// Complete dependency state for one project.
#[derive(Debug, Clone, Serialize)]
pub struct DependencyStatusReport {
    pub ok: bool,
    pub lock_file: &'static str,
    pub resolved: usize,
    pub problems: usize,
    pub items: Vec<DependencyStatusItem>,
}

/// Reads and writes `{users_root}/{owner}/{project}/repo/zeb.lock`.
pub struct DependencyLockService {
    users_root: PathBuf,
    /// Where a project's declared layout comes from, when one is attached.
    ///
    /// Absent only for the constructors that never scan a source tree, which
    /// is why resolution falls back to the platform defaults rather than
    /// failing: a project that declares nothing resolves to them anyway.
    configs: Option<Arc<ProjectConfigurationService>>,
    update_locks: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
}

impl DependencyLockService {
    /// Creates the lock service.
    ///
    /// Every accepted lock source resolves against the project's installed
    /// copies under `data/hub/`, so no embedded-registry resolver is held.
    pub fn new(users_root: PathBuf) -> Self {
        Self {
            users_root,
            configs: None,
            update_locks: Mutex::new(HashMap::new()),
        }
    }

    /// Attaches the reader that answers where a project keeps its source.
    pub fn with_project_configs(mut self, configs: Arc<ProjectConfigurationService>) -> Self {
        self.configs = Some(configs);
        self
    }

    /// The project's source root, or the platform default when no
    /// configuration reader is attached.
    fn source_root(&self, owner: &str, project: &str) -> PathBuf {
        let source = self
            .configs
            .as_ref()
            .and_then(|configs| configs.project_layout(owner, project).ok())
            .unwrap_or_else(ResolvedProjectLayout::platform_default)
            .source;
        self.repo_path(owner, project).join(source)
    }

    fn repo_path(&self, owner: &str, project: &str) -> PathBuf {
        self.users_root
            .join(slug_segment(owner))
            .join(slug_segment(project))
            .join("repo")
    }

    fn data_path(&self, owner: &str, project: &str) -> PathBuf {
        self.users_root
            .join(slug_segment(owner))
            .join(slug_segment(project))
            .join("data")
    }

    /// Root that a node bundle's `entry` path resolves against.
    ///
    /// Bundles are materialized artifacts, so they live in the INSTALLED tier,
    /// `data/hub/`, while the lock that declares them stays in `repo/`. The
    /// recorded `entry` string (`nodes/{slug}/definition.json`) is unchanged
    /// by that split — and was unchanged again by the `data/nodes` →
    /// `data/hub/nodes` move; only the base directory it resolves against
    /// differs.
    fn node_root(&self, owner: &str, project: &str) -> PathBuf {
        self.data_path(owner, project).join("hub")
    }

    fn lock_path(&self, owner: &str, project: &str) -> PathBuf {
        self.repo_path(owner, project).join(DEPENDENCY_LOCK_FILE)
    }

    /// `.../data/recovery` — where migration safety copies live
    /// (`instance-directory.md`), never inside `repo/`.
    fn recovery_path(&self, owner: &str, project: &str) -> PathBuf {
        self.data_path(owner, project).join("recovery")
    }

    fn update_lock(&self, path: &Path) -> Arc<Mutex<()>> {
        let mut locks = self
            .update_locks
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        locks
            .entry(path.to_path_buf())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Reads `zeb.lock`, returning an empty lock only when the file is missing.
    ///
    /// Malformed, unsupported, and legacy files are rejected. Legacy files must
    /// use the explicit migration command so their original bytes are retained.
    pub fn read(&self, owner: &str, project: &str) -> Result<DependencyLockSpec, PlatformError> {
        let path = self.lock_path(owner, project);
        let lock = self.update_lock(&path);
        let _guard = lock.lock().unwrap_or_else(|error| error.into_inner());
        self.read_unlocked(&path, project)
    }

    fn read_unlocked(
        &self,
        path: &Path,
        project: &str,
    ) -> Result<DependencyLockSpec, PlatformError> {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(DependencyLockSpec::default());
            }
            Err(error) => {
                return Err(PlatformError::new(
                    "ZEB_LOCK_READ",
                    format!("failed reading '{}': {error}", path.display()),
                ));
            }
        };
        match decode_dependency_lock(&bytes) {
            Ok(document) => {
                if document.metadata.name != project {
                    return Err(PlatformError::new(
                        "ZEB_LOCK_READ",
                        format!(
                            "metadata.name '{}' must match owning project '{project}'",
                            document.metadata.name
                        ),
                    ));
                }
                Ok(document.spec)
            }
            Err(error) => {
                let suffix = if decode_pre_general_dependency_lock(&bytes).is_ok()
                    || decode_legacy_dependency_lock(&bytes).is_ok()
                {
                    "; run `zebflow project lock migrate <owner> <project>`"
                } else {
                    ""
                };
                Err(PlatformError::new(
                    "ZEB_LOCK_READ",
                    format!("{} ({}){suffix}", error, error.category()),
                ))
            }
        }
    }

    /// Validates and durably replaces `zeb.lock` as canonical JSON.
    pub fn write(
        &self,
        owner: &str,
        project: &str,
        value: &DependencyLockSpec,
    ) -> Result<(), PlatformError> {
        let path = self.lock_path(owner, project);
        let lock = self.update_lock(&path);
        let _guard = lock.lock().unwrap_or_else(|error| error.into_inner());
        self.write_unlocked(&path, project, value)
    }

    fn write_unlocked(
        &self,
        path: &Path,
        project: &str,
        value: &DependencyLockSpec,
    ) -> Result<(), PlatformError> {
        let bytes = encode_dependency_lock(ContractMetadata::named(project), value.clone())
            .map_err(|error| {
                PlatformError::new(
                    "ZEB_LOCK_WRITE",
                    format!("{} ({})", error, error.category()),
                )
            })?;
        atomic_write(path, &bytes).map_err(|error| {
            PlatformError::new(
                "ZEB_LOCK_WRITE",
                format!("failed writing '{}': {error}", path.display()),
            )
        })
    }

    /// Writes `zeb.lock` only if the file does not already exist.
    pub fn write_if_missing(
        &self,
        owner: &str,
        project: &str,
        default: &DependencyLockSpec,
    ) -> Result<(), PlatformError> {
        let path = self.lock_path(owner, project);
        let lock = self.update_lock(&path);
        let _guard = lock.lock().unwrap_or_else(|error| error.into_inner());
        if path.try_exists().map_err(PlatformError::from)? {
            return Ok(());
        }
        self.write_unlocked(&path, project, default)
    }

    /// Adds or replaces one exact library pin without losing concurrent updates.
    pub fn add_rwe_entry(
        &self,
        owner: &str,
        project: &str,
        name: &str,
        entry: DependencyLockArtifactSpec,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |value| {
            value.rwe.libraries.insert(name.to_string(), entry);
        })
    }

    /// Removes one library pin. Missing entries are already in the requested state.
    pub fn remove_rwe_entry(
        &self,
        owner: &str,
        project: &str,
        name: &str,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |value| {
            value.rwe.libraries.remove(name);
        })
    }

    /// Records one exact installed node bundle.
    pub fn upsert_node_bundle(
        &self,
        owner: &str,
        project: &str,
        name: &str,
        entry: DependencyLockNodeBundleSpec,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |value| {
            value.nodes.bundles.insert(name.to_string(), entry);
        })
    }

    /// Reconciles discovered package sources against the lock.
    ///
    /// Installed bundles live under `data/`, so nothing there is authored by
    /// hand. A digest that no longer matches the lock therefore means tampering
    /// or an interrupted write, whatever the bundle's source, and fails closed.
    ///
    /// A newly discovered package that nothing already provides is still pinned,
    /// which is how a restored project adopts bundles that arrived with it.
    pub fn record_discovered_node_bundles(
        &self,
        owner: &str,
        project: &str,
        discovered: Vec<(String, DependencyLockNodeBundleSpec)>,
    ) -> Result<(), PlatformError> {
        if discovered.is_empty() {
            return Ok(());
        }
        let path = self.lock_path(owner, project);
        let lock = self.update_lock(&path);
        let _guard = lock.lock().unwrap_or_else(|error| error.into_inner());
        let mut value = self.read_unlocked(&path, project)?;
        let mut changed = false;
        for (name, entry) in discovered {
            let provider = value
                .nodes
                .bundles
                .iter()
                .find(|(key, locked)| {
                    *key == &name
                        || locked
                            .definitions
                            .iter()
                            .any(|kind| entry.definitions.binary_search(kind).is_ok())
                })
                .map(|(key, locked)| (key.clone(), locked.clone()));

            let Some((_key, locked)) = provider else {
                value.nodes.bundles.insert(name, entry);
                changed = true;
                continue;
            };

            if locked.integrity == entry.integrity {
                continue;
            }
            // Bundles are materialized under `data/`, which is machine-owned and
            // never hand-edited, so drift means tampering or a partial write
            // regardless of where the bundle came from. Re-materializing from
            // the lock is the repair, not silently trusting what is on disk.
            return Err(PlatformError::new(
                "PLATFORM_DEPENDENCY_INTEGRITY",
                format!(
                    "node bundle '{}' no longer matches the digest recorded in zeb.lock",
                    locked.source_id
                ),
            ));
        }
        if changed {
            self.write_unlocked(&path, project, &value)?;
        }
        Ok(())
    }

    /// Removes the bundle pin that provides an explicitly uninstalled node.
    pub fn remove_node_bundle_providing(
        &self,
        owner: &str,
        project: &str,
        kind: &str,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |value| {
            value
                .nodes
                .bundles
                .retain(|_, bundle| !bundle.definitions.iter().any(|item| item == kind));
        })
    }

    /// Records installed-channel provenance for bundles written below one
    /// installation root.
    ///
    /// `bundle_id` is the canonical bundle identity — the package coordinate
    /// `publisher.package` — and becomes the lock key. `source` names the
    /// channel the bytes arrived through (`hub.local`, `hub.public`,
    /// `hub.static`, or `direct.file`), and `source_id` its coordinate.
    #[allow(clippy::too_many_arguments)]
    pub fn record_installed_node_bundles(
        &self,
        owner: &str,
        project: &str,
        install_root: &str,
        bundle_id: &str,
        source: DependencyLockSource,
        source_id: &str,
        version: &str,
    ) -> Result<(), PlatformError> {
        let normalized_root = install_root.trim_matches('/');
        let prefix = format!("{normalized_root}/");
        let path = self.lock_path(owner, project);
        let update_lock = self.update_lock(&path);
        let _guard = update_lock
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut value = self.read_unlocked(&path, project)?;
        let matching = value
            .nodes
            .bundles
            .iter()
            .filter(|(_, bundle)| {
                bundle.entry == normalized_root || bundle.entry.starts_with(&prefix)
            })
            .map(|(name, bundle)| (name.clone(), bundle.clone()))
            .collect::<Vec<_>>();
        if matching.is_empty() {
            return Err(PlatformError::new(
                "ZEB_LOCK_NODE_BUNDLE",
                format!("Hub installation root '{install_root}' contains no node bundle"),
            ));
        }
        for (old_name, _) in &matching {
            value.nodes.bundles.remove(old_name);
        }
        let multiple = matching.len() > 1;
        for (index, (_, mut bundle)) in matching.into_iter().enumerate() {
            bundle.version = version.to_string();
            bundle.source = source;
            bundle.source_id = source_id.to_string();
            // The canonical key is the package coordinate. A package carrying
            // more than one bundle manifest is keyed per manifest, which the
            // contract does not model as one identity; the suffix keeps the
            // entries distinct without inventing a namespace.
            let key = if multiple {
                format!("{bundle_id}-{}", index + 1)
            } else {
                bundle_id.to_string()
            };
            value.nodes.bundles.insert(key, bundle);
        }
        self.write_unlocked(&path, project, &value)
    }

    /// Resolves the current lock against requested libraries and project files.
    pub fn status(
        &self,
        owner: &str,
        project: &str,
        requested_libraries: &ZebflowJsonRweLibraries,
    ) -> Result<DependencyStatusReport, PlatformError> {
        let lock = self.read(owner, project)?;
        let mut items = Vec::new();

        for (name, requested) in requested_libraries {
            let Some(locked) = lock.rwe.libraries.get(name) else {
                items.push(DependencyStatusItem {
                    family: "rwe_library",
                    name: name.clone(),
                    status: DependencyResolutionStatus::Missing,
                    version: requested.version.clone(),
                    source: requested.source.clone(),
                    message: "requested library has no exact lock entry".to_string(),
                    definitions: Vec::new(),
                });
                continue;
            };
            // Every accepted source resolves identically: against the
            // installed copy at `data/hub/`, with the digest pinning identity
            // (`kinds/dependency-lock/README.md`). `source` records
            // provenance, not a resolution order, so the requested source is
            // not compared against the locked one.
            let (status, message) = if locked.version != requested.version {
                (
                    DependencyResolutionStatus::VersionMismatch,
                    "requested library resolution differs from zeb.lock".to_string(),
                )
            } else {
                let installed = self.node_root(owner, project).join(&locked.entry);
                match std::fs::read(&installed) {
                    Ok(bytes) => {
                        let digest = format!(
                            "sha256:{:x}",
                            <sha2::Sha256 as sha2::Digest>::digest(&bytes)
                        );
                        if digest == locked.integrity {
                            (
                                DependencyResolutionStatus::Resolved,
                                "exact installed library is available".to_string(),
                            )
                        } else {
                            (
                                DependencyResolutionStatus::IntegrityMismatch,
                                "installed library bytes do not match zeb.lock".to_string(),
                            )
                        }
                    }
                    Err(_) => (
                        DependencyResolutionStatus::Missing,
                        format!(
                            "installed library bytes are missing at data/hub/{}; \
                             reinstall the library from the hub",
                            locked.entry
                        ),
                    ),
                }
            };
            items.push(DependencyStatusItem {
                family: "rwe_library",
                name: name.clone(),
                status,
                version: locked.version.clone(),
                source: locked.source.as_str().to_string(),
                message,
                definitions: Vec::new(),
            });
        }

        let node_root = self.node_root(owner, project);
        let mut available = crate::pipeline::nodes::builtin_node_definitions()
            .into_iter()
            .map(|definition| definition.kind)
            .collect::<HashSet<_>>();
        available.extend(
            crate::platform::services::node_registry::NodeRegistryService::embedded_official_definitions()
                .into_iter()
                .map(|definition| definition.kind),
        );
        let mut required = BTreeSet::new();
        let source_root = self.source_root(owner, project);
        collect_project_pipeline_requirements(
            &source_root,
            &source_root,
            &mut required,
            &mut items,
        )?;
        for (name, bundle) in &lock.nodes.bundles {
            let manifest = node_root.join(&bundle.entry);
            let (status, message, inspected) = if !manifest.is_file() {
                (
                    DependencyResolutionStatus::Missing,
                    format!("locked manifest '{}' is missing", bundle.entry),
                    None,
                )
            } else {
                let package_dir = manifest.parent().ok_or_else(|| {
                    PlatformError::new("ZEB_LOCK_STATUS", "node manifest has no package directory")
                })?;
                match directory_tree_sha256(package_dir) {
                    Ok(actual) if actual == bundle.integrity => {
                        match inspect_node_bundle(&manifest, package_dir) {
                            Ok(inspected)
                                if inspected.version != bundle.version
                                    || inspected.definitions != bundle.definitions =>
                            {
                                (
                                    DependencyResolutionStatus::VersionMismatch,
                                    "installed node manifest differs from zeb.lock".to_string(),
                                    None,
                                )
                            }
                            Ok(inspected) => (
                                DependencyResolutionStatus::Resolved,
                                "exact node bundle is available".to_string(),
                                Some(inspected),
                            ),
                            Err(error) => (
                                DependencyResolutionStatus::UnsupportedRuntime,
                                error.message,
                                None,
                            ),
                        }
                    }
                    Ok(_) => (
                        DependencyResolutionStatus::IntegrityMismatch,
                        "installed node bundle bytes do not match zeb.lock".to_string(),
                        None,
                    ),
                    Err(error) => (
                        DependencyResolutionStatus::Missing,
                        format!("node bundle cannot be read: {error}"),
                        None,
                    ),
                }
            };
            if let Some(inspected) = inspected {
                available.extend(inspected.definitions);
                for graph in inspected.pipeline_graphs {
                    required.extend(graph.nodes.into_iter().map(|node| node.kind));
                }
            }
            items.push(DependencyStatusItem {
                family: "node_bundle",
                name: name.clone(),
                status,
                version: bundle.version.clone(),
                source: bundle.source.as_str().to_string(),
                message,
                definitions: bundle.definitions.clone(),
            });
        }

        for kind in required {
            if available.contains(&kind) {
                continue;
            }
            let (status, message) = if kind.starts_with(INSTALLED_NODE_KIND_PREFIX) {
                (
                    DependencyResolutionStatus::Missing,
                    "required node kind has no resolved node bundle".to_string(),
                )
            } else {
                (
                    DependencyResolutionStatus::UnsupportedRuntime,
                    "required native node kind is unavailable in this Zebflow runtime".to_string(),
                )
            };
            items.push(DependencyStatusItem {
                family: "node_kind",
                name: kind,
                status,
                version: String::new(),
                source: "unresolved".to_string(),
                message,
                definitions: Vec::new(),
            });
        }

        items.sort_by(|left, right| {
            left.family
                .cmp(right.family)
                .then_with(|| left.name.cmp(&right.name))
        });
        let resolved = items
            .iter()
            .filter(|item| item.status == DependencyResolutionStatus::Resolved)
            .count();
        let problems = items.len().saturating_sub(resolved);
        Ok(DependencyStatusReport {
            ok: problems == 0,
            lock_file: DEPENDENCY_LOCK_FILE,
            resolved,
            problems,
            items,
        })
    }

    /// Re-derives the RWE namespace from requested state and replaces it.
    ///
    /// An installed library's resolution is the installed copy the lock
    /// already pins, so repair keeps that entry and drops entries nothing
    /// requests any more. A requested library with no lock entry cannot be
    /// invented here — reinstalling the package from a hub is the way to
    /// resolve it, and that is what the error says.
    pub fn repair_rwe_libraries(
        &self,
        owner: &str,
        project: &str,
        requested_libraries: &ZebflowJsonRweLibraries,
    ) -> Result<(), PlatformError> {
        let current = self.read(owner, project)?;
        let mut resolved = std::collections::BTreeMap::new();
        let mut requested = requested_libraries.iter().collect::<Vec<_>>();
        requested.sort_by(|left, right| left.0.cmp(right.0));
        for (name, _entry) in requested {
            let Some(existing) = current.rwe.libraries.get(name) else {
                return Err(PlatformError::new(
                    "ZEB_LOCK_REPAIR_UNAVAILABLE",
                    format!(
                        "requested library '{name}' has no lock entry to keep; \
                         reinstall it from the hub"
                    ),
                ));
            };
            resolved.insert(name.clone(), existing.clone());
        }
        self.update(owner, project, |value| {
            value.rwe.libraries = resolved;
        })
    }

    /// Quarantines a `zeb.lock` the canonical reader refuses, so ordinary
    /// resolution can regenerate it from requested state.
    ///
    /// A lock carrying a dead source word is an invalid lock
    /// (`kinds/dependency-lock/README.md`): its bytes move to
    /// `data/recovery/` and an empty canonical lock takes their place —
    /// installed copies on disk are untouched, and reinstall/refresh re-pins
    /// them. A legacy-shaped lock is refused here: the explicit migration
    /// path owns those bytes. Returns the recovery path, or `None` when the
    /// lock is absent or already canonical.
    pub fn quarantine_invalid(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Option<PathBuf>, PlatformError> {
        let path = self.lock_path(owner, project);
        let lock = self.update_lock(&path);
        let _guard = lock.lock().unwrap_or_else(|error| error.into_inner());
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(PlatformError::new(
                    "ZEB_LOCK_READ",
                    format!("failed reading '{}': {error}", path.display()),
                ));
            }
        };
        if decode_dependency_lock(&bytes).is_ok() {
            return Ok(None);
        }
        if decode_pre_general_dependency_lock(&bytes).is_ok()
            || decode_legacy_dependency_lock(&bytes).is_ok()
        {
            return Err(PlatformError::new(
                "ZEB_LOCK_READ",
                "lock uses a pre-release legacy shape; run `zebflow project lock migrate` \
                 instead of regenerating",
            ));
        }
        let recovery_path = self.recovery_path(owner, project).join(format!(
            "{DEPENDENCY_LOCK_BACKUP_FILE}-invalid-{}.lock",
            crate::platform::model::recovery_date_stamp()
        ));
        if let Some(parent) = recovery_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        atomic_write(&recovery_path, &bytes).map_err(|error| {
            PlatformError::new(
                "ZEB_LOCK_READ",
                format!("failed writing recovery copy: {error}"),
            )
        })?;
        self.write_unlocked(&path, project, &DependencyLockSpec::default())?;
        Ok(Some(recovery_path))
    }

    /// Validates requested RWE libraries before compiling an affected artifact.
    pub fn validate_rwe_dependencies(
        &self,
        owner: &str,
        project: &str,
        requested_libraries: &ZebflowJsonRweLibraries,
    ) -> Result<(), PlatformError> {
        let report = self.status(owner, project, requested_libraries)?;
        let problems = report
            .items
            .iter()
            .filter(|item| {
                item.family == "rwe_library" && item.status != DependencyResolutionStatus::Resolved
            })
            .map(|item| format!("{}: {}", item.name, item.message))
            .collect::<Vec<_>>();
        if problems.is_empty() {
            Ok(())
        } else {
            Err(PlatformError::new(
                "PLATFORM_RWE_DEPENDENCY",
                format!("RWE dependencies are unresolved: {}", problems.join("; ")),
            ))
        }
    }

    /// Validates node capabilities once before pipeline activation.
    pub fn validate_pipeline_dependencies(
        &self,
        owner: &str,
        project: &str,
        graph: &PipelineGraph,
    ) -> Result<(), PlatformError> {
        let lock = self.read(owner, project)?;
        let mut available = crate::pipeline::nodes::builtin_node_definitions()
            .into_iter()
            .map(|definition| definition.kind)
            .collect::<HashSet<_>>();
        available.extend(
            crate::platform::services::node_registry::NodeRegistryService::embedded_official_definitions()
                .into_iter()
                .map(|definition| definition.kind),
        );

        let node_root = self.node_root(owner, project);
        let mut required = graph
            .nodes
            .iter()
            .map(|node| node.kind.clone())
            .collect::<BTreeSet<_>>();
        let mut checked = BTreeSet::new();
        loop {
            let Some((bundle_name, bundle)) = lock.nodes.bundles.iter().find(|(name, bundle)| {
                !checked.contains(*name)
                    && bundle
                        .definitions
                        .iter()
                        .any(|kind| required.contains(kind))
            }) else {
                break;
            };
            checked.insert(bundle_name.clone());
            let manifest = node_root.join(&bundle.entry);
            if !manifest.is_file() {
                return Err(PlatformError::new(
                    "PLATFORM_DEPENDENCY_MISSING",
                    format!(
                        "required node bundle manifest '{}' is missing",
                        bundle.entry
                    ),
                ));
            }
            let package_dir = manifest.parent().ok_or_else(|| {
                PlatformError::new("PLATFORM_DEPENDENCY_MISSING", "invalid node bundle path")
            })?;
            let actual = directory_tree_sha256(package_dir).map_err(|error| {
                PlatformError::new(
                    "PLATFORM_DEPENDENCY_MISSING",
                    format!("failed reading required node bundle: {error}"),
                )
            })?;
            if actual != bundle.integrity {
                return Err(PlatformError::new(
                    "PLATFORM_DEPENDENCY_INTEGRITY",
                    format!(
                        "required node bundle '{}' does not match zeb.lock",
                        bundle.source_id
                    ),
                ));
            }
            let inspected = inspect_node_bundle(&manifest, package_dir)?;
            if inspected.version != bundle.version || inspected.definitions != bundle.definitions {
                return Err(PlatformError::new(
                    "PLATFORM_DEPENDENCY_VERSION",
                    format!(
                        "required node bundle '{}' manifest differs from zeb.lock",
                        bundle.source_id
                    ),
                ));
            }
            available.extend(inspected.definitions);
            for nested in inspected.pipeline_graphs {
                required.extend(nested.nodes.into_iter().map(|node| node.kind));
            }
        }

        let missing = required
            .into_iter()
            .filter(|kind| !available.contains(kind))
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_DEPENDENCY_MISSING",
                format!(
                    "pipeline requires unavailable node kinds: {}",
                    missing.join(", ")
                ),
            ));
        }
        Ok(())
    }

    /// Enables one RWE library while keeping requested and resolved state aligned.
    ///
    /// The lock write occurs while both per-project mutation locks are held. If
    /// the configuration write fails, the previous lock is restored before the
    /// operation returns an error.
    pub fn enable_rwe_library(
        &self,
        configuration: &ProjectConfigurationService,
        owner: &str,
        project: &str,
        name: &str,
        entry: DependencyLockArtifactSpec,
    ) -> Result<(), PlatformError> {
        let path = self.lock_path(owner, project);
        let lock = self.update_lock(&path);
        let _guard = lock.lock().unwrap_or_else(|error| error.into_inner());
        let previous = self.read_unlocked(&path, project)?;
        let mut next = previous.clone();
        next.rwe.libraries.insert(name.to_string(), entry.clone());
        let result = configuration.update_fallible(owner, project, |config| {
            config.configs.rwe.libraries.insert(
                name.to_string(),
                crate::platform::model::ZebflowJsonRweLibraryEntry {
                    version: entry.version.clone(),
                    // Requested state records that the library is installed at
                    // an exact version; the lock keeps the provenance. Every
                    // accepted lock source resolves against the installed copy
                    // at `data/hub/rwe-libraries/`, so they all request "hub".
                    source: match entry.source {
                        DependencyLockSource::Project => {
                            return Err(PlatformError::new(
                                "ZEB_LOCK_RWE_SOURCE",
                                "'project' is not an accepted RWE library source in zebflow.com/v1",
                            ));
                        }
                        _ => "hub".to_string(),
                    },
                },
            );
            self.write_unlocked(&path, project, &next)
        });
        self.finish_coordinated_update(&path, project, &previous, result)
    }

    /// Disables one RWE library while keeping requested and resolved state aligned.
    pub fn disable_rwe_library(
        &self,
        configuration: &ProjectConfigurationService,
        owner: &str,
        project: &str,
        name: &str,
    ) -> Result<(), PlatformError> {
        let path = self.lock_path(owner, project);
        let lock = self.update_lock(&path);
        let _guard = lock.lock().unwrap_or_else(|error| error.into_inner());
        let previous = self.read_unlocked(&path, project)?;
        let mut next = previous.clone();
        next.rwe.libraries.remove(name);
        let result = configuration.update_fallible(owner, project, |config| {
            config.configs.rwe.libraries.remove(name);
            self.write_unlocked(&path, project, &next)
        });
        self.finish_coordinated_update(&path, project, &previous, result)
    }

    fn finish_coordinated_update(
        &self,
        path: &Path,
        project: &str,
        previous: &DependencyLockSpec,
        result: Result<crate::platform::model::ZebflowJson, PlatformError>,
    ) -> Result<(), PlatformError> {
        match result {
            Ok(_) => Ok(()),
            Err(operation_error) => match self.write_unlocked(path, project, previous) {
                Ok(()) => Err(operation_error),
                Err(recovery_error) => Err(PlatformError::new(
                    "ZEB_LOCK_TRANSACTION_RECOVERY",
                    format!(
                        "library update failed ({operation_error}); restoring the previous dependency lock also failed ({recovery_error})"
                    ),
                )),
            },
        }
    }

    fn update<F>(&self, owner: &str, project: &str, mutate: F) -> Result<(), PlatformError>
    where
        F: FnOnce(&mut DependencyLockSpec),
    {
        let path = self.lock_path(owner, project);
        let lock = self.update_lock(&path);
        let _guard = lock.lock().unwrap_or_else(|error| error.into_inner());
        let mut value = self.read_unlocked(&path, project)?;
        mutate(&mut value);
        self.write_unlocked(&path, project, &value)
    }

    /// Explicitly migrates the pre-contract `version: 1` lock in place.
    ///
    /// Missing digests are calculated from the registered library bytes. The
    /// original file remains at `data/recovery/zeb-lock-{date}.lock` before
    /// canonical bytes replace `repo/zeb.lock` — `data/recovery/`, not
    /// `repo/`, because a machine-generated recovery copy must never enter
    /// the git-tracked repository (`project-directory.md` §1, §5).
    pub fn migrate_legacy(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<DependencyLockMigration, PlatformError> {
        let path = self.lock_path(owner, project);
        let backup_path = self.recovery_path(owner, project).join(format!(
            "{DEPENDENCY_LOCK_BACKUP_FILE}-{}.lock",
            crate::platform::model::recovery_date_stamp()
        ));
        let lock = self.update_lock(&path);
        let _guard = lock.lock().unwrap_or_else(|error| error.into_inner());
        let bytes = std::fs::read(&path).map_err(|error| {
            PlatformError::new(
                "ZEB_LOCK_MIGRATE",
                format!("failed reading '{}': {error}", path.display()),
            )
        })?;
        if decode_dependency_lock(&bytes).is_ok() {
            return Err(PlatformError::new(
                "ZEB_LOCK_MIGRATE",
                format!("'{}' is already canonical", path.display()),
            ));
        }
        let (source_format, libraries) = match decode_pre_general_dependency_lock(&bytes) {
            Ok(previous) => {
                if previous.metadata.name != project {
                    return Err(PlatformError::new(
                        "ZEB_LOCK_MIGRATE",
                        format!(
                            "metadata.name '{}' must match owning project '{project}'",
                            previous.metadata.name
                        ),
                    ));
                }
                ("rwe_only_v1", previous.spec.libraries)
            }
            Err(previous_error) => match decode_legacy_dependency_lock(&bytes) {
                Ok(legacy) => ("legacy_1", legacy.libraries),
                Err(legacy_error) => {
                    return Err(PlatformError::new(
                        "ZEB_LOCK_MIGRATE",
                        format!(
                            "unsupported dependency lock; RWE-only v1 error: {previous_error}; legacy error: {legacy_error}"
                        ),
                    ));
                }
            },
        };
        let migrated = DependencyLockSpec::default();
        // A legacy entry named the binary's embedded registry, and `embedded`
        // is no longer a lock source (`kinds/dependency-lock/README.md`,
        // restructured 2026-08-27): the binary is only the seed, and a locked
        // library is an installed copy at `data/hub/rwe-libraries/`. Nothing
        // shipped, so a legacy entry is not converted — the lock regenerates
        // through ordinary resolution once the library is reinstalled from a
        // hub, and this migration refuses rather than writing a lock it knows
        // to be unresolvable.
        if let Some((name, _)) = libraries.into_iter().next() {
            return Err(PlatformError::new(
                "ZEB_LOCK_MIGRATE",
                format!(
                    "legacy entry '{name}' names the retired embedded source; \
                     reinstall the library from the hub and the lock regenerates"
                ),
            ));
        }

        let canonical = encode_dependency_lock(ContractMetadata::named(project), migrated.clone())
            .map_err(|error| PlatformError::new("ZEB_LOCK_MIGRATE", error.to_string()))?;
        match std::fs::read(&backup_path) {
            Ok(existing) if existing != bytes => {
                return Err(PlatformError::new(
                    "ZEB_LOCK_MIGRATE",
                    format!(
                        "recovery copy '{}' already exists with different content",
                        backup_path.display()
                    ),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if let Some(parent) = backup_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                atomic_write(&backup_path, &bytes).map_err(|error| {
                    PlatformError::new(
                        "ZEB_LOCK_MIGRATE",
                        format!("failed writing recovery copy: {error}"),
                    )
                })?;
            }
            Err(error) => {
                return Err(PlatformError::new(
                    "ZEB_LOCK_MIGRATE",
                    format!("failed reading recovery copy: {error}"),
                ));
            }
        }
        atomic_write(&path, &canonical).map_err(|error| {
            PlatformError::new(
                "ZEB_LOCK_MIGRATE",
                format!("failed writing canonical lock: {error}"),
            )
        })?;
        let reopened = self.read_unlocked(&path, project)?;
        if reopened != migrated {
            return Err(PlatformError::new(
                "ZEB_LOCK_MIGRATE",
                "canonical dependency lock did not roundtrip after migration",
            ));
        }
        Ok(DependencyLockMigration {
            source_format,
            canonical_path: path,
            recovery_path: backup_path,
        })
    }
}

struct InspectedNodeBundle {
    version: String,
    definitions: Vec<String>,
    pipeline_graphs: Vec<PipelineGraph>,
}

fn inspect_node_bundle(
    manifest_path: &Path,
    package_dir: &Path,
) -> Result<InspectedNodeBundle, PlatformError> {
    let bytes = std::fs::read(manifest_path).map_err(|error| {
        PlatformError::new(
            "PLATFORM_DEPENDENCY_MANIFEST",
            format!("failed reading '{}': {error}", manifest_path.display()),
        )
    })?;
    let mut pipeline_paths = BTreeSet::new();
    if manifest_path.file_name().and_then(|name| name.to_str()) != Some("definition.json") {
        return Err(PlatformError::new(
            "PLATFORM_DEPENDENCY_MANIFEST",
            "node bundle entry must end with definition.json",
        ));
    }
    let document = decode_contract::<NodeBundleContract>(&bytes).map_err(|error| {
        PlatformError::new(
            "PLATFORM_DEPENDENCY_MANIFEST",
            format!("{} ({})", error, error.category()),
        )
    })?;
    pipeline_paths.extend(document.spec.functions.values().cloned());
    let version = document.spec.version;
    let mut definitions = document
        .spec
        .nodes
        .into_iter()
        .map(|node| node.kind)
        .collect::<Vec<_>>();
    definitions.sort();
    definitions.dedup();

    let mut pipeline_graphs = Vec::new();
    for relative in pipeline_paths {
        let path = safe_node_bundle_path(package_dir, &relative)?;
        let source = std::fs::read(&path).map_err(|error| {
            PlatformError::new(
                "PLATFORM_DEPENDENCY_MANIFEST",
                format!(
                    "failed reading nested pipeline '{}': {error}",
                    path.display()
                ),
            )
        })?;
        let graph = decode_pipeline_graph(&source).map_err(|error| {
            PlatformError::new(
                "PLATFORM_DEPENDENCY_MANIFEST",
                format!(
                    "invalid nested pipeline '{}': {} ({})",
                    path.display(),
                    error,
                    error.category()
                ),
            )
        })?;
        pipeline_graphs.push(graph.spec);
    }
    Ok(InspectedNodeBundle {
        version,
        definitions,
        pipeline_graphs,
    })
}

fn safe_node_bundle_path(package_dir: &Path, relative: &str) -> Result<PathBuf, PlatformError> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(PlatformError::new(
            "PLATFORM_DEPENDENCY_MANIFEST",
            format!("unsafe node bundle path '{relative}'"),
        ));
    }
    Ok(package_dir.join(relative_path))
}

fn collect_project_pipeline_requirements(
    root: &Path,
    current: &Path,
    required: &mut BTreeSet<String>,
    items: &mut Vec<DependencyStatusItem>,
) -> Result<(), PlatformError> {
    let entries = match std::fs::read_dir(current) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(PlatformError::new(
                "PLATFORM_DEPENDENCY_SCAN",
                format!("failed reading '{}': {error}", current.display()),
            ));
        }
    };
    let mut entries = entries.collect::<Result<Vec<_>, _>>().map_err(|error| {
        PlatformError::new(
            "PLATFORM_DEPENDENCY_SCAN",
            format!("failed reading '{}': {error}", current.display()),
        )
    })?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| {
            PlatformError::new(
                "PLATFORM_DEPENDENCY_SCAN",
                format!("failed inspecting '{}': {error}", path.display()),
            )
        })?;
        if file_type.is_dir() {
            collect_project_pipeline_requirements(root, &path, required, items)?;
            continue;
        }
        if !file_type.is_file()
            || path.extension().and_then(|value| value.to_str()) != Some("json")
            || !path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.ends_with(".zf.json"))
        {
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let source = std::fs::read(&path).map_err(|error| {
            PlatformError::new(
                "PLATFORM_DEPENDENCY_SCAN",
                format!("failed reading pipeline '{relative}': {error}"),
            )
        })?;
        match decode_pipeline_graph(&source) {
            Ok(document) => {
                required.extend(document.spec.nodes.into_iter().map(|node| node.kind));
            }
            Err(error) => items.push(DependencyStatusItem {
                family: "pipeline_source",
                name: relative,
                status: DependencyResolutionStatus::UnsupportedRuntime,
                version: String::new(),
                source: "project".to_string(),
                message: format!(
                    "pipeline source is invalid: {} ({})",
                    error,
                    error.category()
                ),
                definitions: Vec::new(),
            }),
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyDependencyLock {
    version: u32,
    #[serde(default)]
    libraries: std::collections::BTreeMap<String, LegacyDependencyLockEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // decoded strictly so unknown shapes refuse; the values themselves are never converted
struct LegacyDependencyLockEntry {
    version: String,
    source: String,
    entry: String,
    integrity: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreGeneralDependencyLock {
    #[serde(rename = "apiVersion")]
    api_version: String,
    kind: String,
    metadata: PreGeneralDependencyLockMetadata,
    spec: PreGeneralDependencyLockSpec,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreGeneralDependencyLockMetadata {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreGeneralDependencyLockSpec {
    #[serde(default)]
    libraries: std::collections::BTreeMap<String, LegacyDependencyLockEntry>,
}

fn decode_pre_general_dependency_lock(bytes: &[u8]) -> Result<PreGeneralDependencyLock, String> {
    let value: PreGeneralDependencyLock =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    if value.api_version != crate::contracts::CONTRACT_API_VERSION {
        return Err(format!("unsupported apiVersion {}", value.api_version));
    }
    if value.kind != "DependencyLock" {
        return Err(format!("unexpected kind {}", value.kind));
    }
    Ok(value)
}

fn decode_legacy_dependency_lock(bytes: &[u8]) -> Result<LegacyDependencyLock, String> {
    let value: LegacyDependencyLock =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    if value.version != 1 {
        return Err(format!("unsupported legacy version {}", value.version));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};

    use super::*;
    use crate::pipeline::PipelineNode;

    fn entry(seed: char) -> DependencyLockArtifactSpec {
        DependencyLockArtifactSpec {
            version: "1.0.0".to_string(),
            source: DependencyLockSource::HubLocal,
            source_id: "zebflow.example@1.0.0".to_string(),
            entry: "rwe-libraries/zebflow.example/dist/main.mjs".to_string(),
            integrity: format!("sha256:{}", seed.to_string().repeat(64)),
        }
    }

    #[test]
    fn malformed_future_and_legacy_lock_files_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        let service = DependencyLockService::new(root.path().join("users"));
        let path = service.lock_path("owner", "project");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();

        std::fs::write(&path, b"not-json").unwrap();
        assert_eq!(
            service.read("owner", "project").unwrap_err().code,
            "ZEB_LOCK_READ"
        );

        std::fs::write(
            &path,
            br#"{"apiVersion":"zebflow.com/v2","kind":"DependencyLock","metadata":{"name":"project"},"spec":{"rwe":{"libraries":{}},"nodes":{"bundles":{}}}}"#,
        )
        .unwrap();
        assert_eq!(
            service.read("owner", "project").unwrap_err().code,
            "ZEB_LOCK_READ"
        );

        std::fs::write(&path, br#"{"version":1,"libraries":{}}"#).unwrap();
        assert!(
            service
                .read("owner", "project")
                .unwrap_err()
                .message
                .contains("project lock migrate")
        );
    }

    #[test]
    fn lock_write_is_strict_and_roundtrips() {
        let root = tempfile::tempdir().unwrap();
        let service = DependencyLockService::new(root.path().join("users"));
        service
            .write("owner", "project", &DependencyLockSpec::default())
            .unwrap();
        assert!(
            service
                .read("owner", "project")
                .unwrap()
                .rwe
                .libraries
                .is_empty()
        );

        let path = service.lock_path("owner", "project");
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(value["apiVersion"], "zebflow.com/v1");
        assert_eq!(value["kind"], "DependencyLock");
    }

    #[test]
    fn concurrent_updates_preserve_every_library() {
        let root = tempfile::tempdir().unwrap();
        let service = Arc::new(DependencyLockService::new(root.path().join("users")));
        service
            .write("owner", "project", &DependencyLockSpec::default())
            .unwrap();
        let barrier = Arc::new(Barrier::new(9));
        let mut threads = Vec::new();
        for index in 0..8 {
            let service = Arc::clone(&service);
            let barrier = Arc::clone(&barrier);
            threads.push(std::thread::spawn(move || {
                barrier.wait();
                service
                    .add_rwe_entry(
                        "owner",
                        "project",
                        &format!("zeb/library-{index}"),
                        entry(char::from_digit(index as u32, 16).unwrap()),
                    )
                    .unwrap();
            }));
        }
        barrier.wait();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(
            service
                .read("owner", "project")
                .unwrap()
                .rwe
                .libraries
                .len(),
            8
        );
    }

    /// A legacy lock with entries names the retired embedded source, so the
    /// migration refuses it untouched; an empty legacy lock still converts,
    /// with its original bytes kept as the recovery copy.
    #[test]
    fn legacy_migration_refuses_embedded_entries_and_converts_empty_locks() {
        let root = tempfile::tempdir().unwrap();
        let service = DependencyLockService::new(root.path().join("users"));
        let path = service.lock_path("owner", "project");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let legacy = format!(
            r#"{{"version":1,"libraries":{{"zeb/example":{{"version":"1.0.0","source":"offline","entry":"dist/main.mjs","integrity":"sha256:{}"}}}}}}"#,
            "a".repeat(64)
        );
        std::fs::write(&path, legacy.as_bytes()).unwrap();

        let error = service.migrate_legacy("owner", "project").unwrap_err();
        assert_eq!(error.code, "ZEB_LOCK_MIGRATE");
        assert!(error.message.contains("reinstall the library from the hub"));
        assert_eq!(std::fs::read(&path).unwrap(), legacy.as_bytes());

        let empty = br#"{"version":1,"libraries":{}}"#;
        std::fs::write(&path, empty).unwrap();
        let result = service.migrate_legacy("owner", "project").unwrap();
        assert_eq!(result.source_format, "legacy_1");
        assert_eq!(std::fs::read(result.recovery_path).unwrap(), empty);
        assert!(
            service
                .read("owner", "project")
                .unwrap()
                .rwe
                .libraries
                .is_empty()
        );
    }

    #[test]
    fn real_legacy_shape_with_entries_is_refused_toward_regeneration() {
        let root = tempfile::tempdir().unwrap();
        let service = DependencyLockService::new(root.path().join("users"));
        let path = service.lock_path("owner", "project");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let legacy = include_bytes!(
            "../../../tests/fixtures/contracts/dependency-lock/legacy-1-complete.json"
        );
        std::fs::write(&path, legacy).unwrap();

        let error = service.migrate_legacy("owner", "project").unwrap_err();
        assert_eq!(error.code, "ZEB_LOCK_MIGRATE");
        assert!(error.message.contains("reinstall"));
        assert_eq!(std::fs::read(&path).unwrap(), legacy);
    }

    #[test]
    fn rwe_only_envelope_with_entries_is_refused_toward_regeneration() {
        let root = tempfile::tempdir().unwrap();
        let service = DependencyLockService::new(root.path().join("users"));
        let path = service.lock_path("owner", "project");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let previous = format!(
            r#"{{"apiVersion":"zebflow.com/v1","kind":"DependencyLock","metadata":{{"name":"project"}},"spec":{{"libraries":{{"zeb/deckgl":{{"version":"full-9.x","source":"offline","entry":"0.1/runtime/deckgl.bundle.mjs","integrity":"sha256:{}"}}}}}}}}"#,
            "0".repeat(64)
        );
        std::fs::write(&path, previous.as_bytes()).unwrap();

        let migration = service.migrate_legacy("owner", "project").unwrap_err();
        assert_eq!(migration.code, "ZEB_LOCK_MIGRATE");
        assert!(migration.message.contains("reinstall"));
        assert_eq!(std::fs::read(&path).unwrap(), previous.as_bytes());

        let empty = br#"{"apiVersion":"zebflow.com/v1","kind":"DependencyLock","metadata":{"name":"project"},"spec":{"libraries":{}}}"#;
        std::fs::write(&path, empty).unwrap();
        let migration = service.migrate_legacy("owner", "project").unwrap();
        assert_eq!(migration.source_format, "rwe_only_v1");
        assert!(
            service
                .read("owner", "project")
                .unwrap()
                .rwe
                .libraries
                .is_empty()
        );
    }

    /// A canonical-shaped lock hand-edited to a dead source word refuses on
    /// read, and the quarantine path moves it aside so ordinary resolution
    /// can regenerate from requested state.
    #[test]
    fn a_lock_carrying_a_dead_source_word_is_refused_and_quarantined() {
        let root = tempfile::tempdir().unwrap();
        let service = DependencyLockService::new(root.path().join("users"));
        let path = service.lock_path("owner", "project");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let dead = format!(
            r#"{{"apiVersion":"zebflow.com/v1","kind":"DependencyLock","metadata":{{"name":"project"}},"spec":{{"rwe":{{"libraries":{{"zeb/example":{{"version":"1.0.0","source":"embedded","source_id":"zebflow.example@1.0.0","entry":"rwe-libraries/zebflow.example/dist/main.mjs","integrity":"sha256:{}"}}}}}},"nodes":{{"bundles":{{}}}}}}}}"#,
            "a".repeat(64)
        );
        std::fs::write(&path, dead.as_bytes()).unwrap();

        let error = service.read("owner", "project").unwrap_err();
        assert_eq!(error.code, "ZEB_LOCK_READ");

        let recovery = service
            .quarantine_invalid("owner", "project")
            .unwrap()
            .expect("invalid lock is quarantined");
        assert_eq!(std::fs::read(&recovery).unwrap(), dead.as_bytes());
        assert!(
            service
                .read("owner", "project")
                .unwrap()
                .rwe
                .libraries
                .is_empty()
        );
        // Already-canonical locks are left alone.
        assert!(
            service
                .quarantine_invalid("owner", "project")
                .unwrap()
                .is_none()
        );
    }

    /// The quarantine path never eats a legacy-shaped lock: those bytes
    /// belong to the explicit migration command.
    #[test]
    fn quarantine_refuses_legacy_shapes() {
        let root = tempfile::tempdir().unwrap();
        let service = DependencyLockService::new(root.path().join("users"));
        let path = service.lock_path("owner", "project");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let legacy = br#"{"version":1,"libraries":{}}"#;
        std::fs::write(&path, legacy).unwrap();

        let error = service.quarantine_invalid("owner", "project").unwrap_err();
        assert_eq!(error.code, "ZEB_LOCK_READ");
        assert_eq!(std::fs::read(&path).unwrap(), legacy);
    }

    #[test]
    fn coordinated_enablement_updates_configuration_and_lock() {
        let root = tempfile::tempdir().unwrap();
        let users = root.path().join("users");
        let configuration = ProjectConfigurationService::new(users.clone());
        configuration
            .ensure_initialized("owner", "project", "Project")
            .unwrap();
        let service = DependencyLockService::new(users);
        service
            .write("owner", "project", &DependencyLockSpec::default())
            .unwrap();

        service
            .enable_rwe_library(
                &configuration,
                "owner",
                "project",
                "zeb/example",
                entry('a'),
            )
            .unwrap();
        assert!(
            configuration
                .get_rwe_libraries("owner", "project")
                .unwrap()
                .contains_key("zeb/example")
        );
        assert!(
            service
                .read("owner", "project")
                .unwrap()
                .rwe
                .libraries
                .contains_key("zeb/example")
        );

        service
            .disable_rwe_library(&configuration, "owner", "project", "zeb/example")
            .unwrap();
        assert!(
            configuration
                .get_rwe_libraries("owner", "project")
                .unwrap()
                .is_empty()
        );
        assert!(
            service
                .read("owner", "project")
                .unwrap()
                .rwe
                .libraries
                .is_empty()
        );
    }

    #[test]
    fn failed_migration_preserves_legacy_source() {
        let root = tempfile::tempdir().unwrap();
        let service = DependencyLockService::new(root.path().join("users"));
        let path = service.lock_path("owner", "project");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let legacy = br#"{"version":1,"libraries":{"zeb/example":{"version":"1","source":"offline","entry":"dist/main.mjs","integrity":null}}}"#;
        std::fs::write(&path, legacy).unwrap();

        assert!(service.migrate_legacy("owner", "project").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), legacy);
        assert!(
            !service.recovery_path("owner", "project").exists(),
            "a refused migration writes nothing, not even the recovery directory"
        );
    }

    #[test]
    fn migration_never_overwrites_a_different_recovery_copy() {
        let root = tempfile::tempdir().unwrap();
        let service = DependencyLockService::new(root.path().join("users"));
        let path = service.lock_path("owner", "project");
        let recovery = service.recovery_path("owner", "project").join(format!(
            "{DEPENDENCY_LOCK_BACKUP_FILE}-{}.lock",
            crate::platform::model::recovery_date_stamp()
        ));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::create_dir_all(recovery.parent().unwrap()).unwrap();
        let legacy = format!(
            r#"{{"version":1,"libraries":{{"zeb/example":{{"version":"1.0.0","source":"offline","entry":"dist/main.mjs","integrity":"sha256:{}"}}}}}}"#,
            "a".repeat(64)
        );
        std::fs::write(&path, legacy.as_bytes()).unwrap();
        std::fs::write(&recovery, b"different recovery").unwrap();

        let error = service.migrate_legacy("owner", "project").unwrap_err();
        assert_eq!(error.code, "ZEB_LOCK_MIGRATE");
        assert_eq!(std::fs::read(&path).unwrap(), legacy.as_bytes());
        assert_eq!(std::fs::read(&recovery).unwrap(), b"different recovery");
    }

    #[test]
    fn failed_coordinated_update_preserves_the_previous_lock() {
        let root = tempfile::tempdir().unwrap();
        let users = root.path().join("users");
        let configuration = ProjectConfigurationService::new(users.clone());
        configuration
            .ensure_initialized("owner", "project", "Project")
            .unwrap();
        let service = DependencyLockService::new(users.clone());
        service
            .write("owner", "project", &DependencyLockSpec::default())
            .unwrap();
        std::fs::write(
            users.join("owner/project/repo/zebflow.yaml"),
            b"not: [valid",
        )
        .unwrap();

        assert!(
            service
                .enable_rwe_library(
                    &configuration,
                    "owner",
                    "project",
                    "zeb/example",
                    entry('a'),
                )
                .is_err()
        );
        assert!(
            service
                .read("owner", "project")
                .unwrap()
                .rwe
                .libraries
                .is_empty()
        );
    }

    /// A locked library resolves against its installed copy at `data/hub/`,
    /// whatever serving its provenance names, and repair keeps the entry.
    #[test]
    fn status_resolves_exact_installed_library() {
        let root = tempfile::tempdir().unwrap();
        let users = root.path().join("users");
        let service = DependencyLockService::new(users.clone());
        let mut requested = ZebflowJsonRweLibraries::new();
        requested.insert(
            "zeb/example".to_string(),
            crate::platform::model::ZebflowJsonRweLibraryEntry {
                version: "1.0.0".to_string(),
                source: "hub".to_string(),
            },
        );
        let bundle = b"export default {};\n";
        let installed = users.join("owner/project/data/hub/rwe-libraries/zebflow.example/dist");
        std::fs::create_dir_all(&installed).unwrap();
        std::fs::write(installed.join("main.mjs"), bundle).unwrap();
        let mut locked = entry('a');
        locked.integrity = format!(
            "sha256:{:x}",
            <sha2::Sha256 as sha2::Digest>::digest(bundle)
        );
        service
            .add_rwe_entry("owner", "project", "zeb/example", locked)
            .unwrap();
        service
            .repair_rwe_libraries("owner", "project", &requested)
            .unwrap();

        let report = service.status("owner", "project", &requested).unwrap();
        assert!(report.ok, "{report:?}");
        assert_eq!(report.resolved, 1);
        assert_eq!(report.problems, 0);

        // A requested library with no lock entry cannot be invented by repair.
        requested.insert(
            "zeb/absent".to_string(),
            crate::platform::model::ZebflowJsonRweLibraryEntry {
                version: "1.0.0".to_string(),
                source: "hub".to_string(),
            },
        );
        let error = service
            .repair_rwe_libraries("owner", "project", &requested)
            .unwrap_err();
        assert_eq!(error.code, "ZEB_LOCK_REPAIR_UNAVAILABLE");
    }

    #[test]
    fn status_detects_changed_and_missing_node_bundle() {
        let root = tempfile::tempdir().unwrap();
        let users = root.path().join("users");
        let service = DependencyLockService::new(users.clone());
        let package_dir = users.join("owner/project/data/hub/nodes/example");
        std::fs::create_dir_all(&package_dir).unwrap();
        std::fs::write(
            package_dir.join("definition.json"),
            br#"{
              "apiVersion":"zebflow.com/v1",
              "kind":"NodeBundle",
              "metadata":{"name":"example","version":"1.0.0"},
	              "spec":{
	                "package":"example",
	                "version":"1.0.0",
	                "title":"Example",
	                "description":"Example composite node bundle.",
	                "nodes":[{
	                  "kind":"n.x.example.thing",
	                  "title":"Example",
	                  "description":"Execute the example function.",
	                  "trigger":{"type":"webhook"},
	                  "definition":{"output_pins":["out"]}
	                }]
	              }
            }"#,
        )
        .unwrap();
        let mut lock = DependencyLockSpec::default();
        lock.nodes.bundles.insert(
            "example".to_string(),
            DependencyLockNodeBundleSpec {
                version: "1.0.0".to_string(),
                source: DependencyLockSource::Project,
                source_id: "project/example".to_string(),
                entry: "nodes/example/definition.json".to_string(),
                integrity: directory_tree_sha256(&package_dir).unwrap(),
                definitions: vec!["n.x.example.thing".to_string()],
            },
        );
        service.write("owner", "project", &lock).unwrap();
        let requested = ZebflowJsonRweLibraries::new();
        assert!(service.status("owner", "project", &requested).unwrap().ok);

        std::fs::write(package_dir.join("definition.json"), b"changed").unwrap();
        let report = service.status("owner", "project", &requested).unwrap();
        assert_eq!(
            report.items[0].status,
            DependencyResolutionStatus::IntegrityMismatch
        );

        std::fs::remove_file(package_dir.join("definition.json")).unwrap();
        let report = service.status("owner", "project", &requested).unwrap();
        assert_eq!(report.items[0].status, DependencyResolutionStatus::Missing);
    }

    #[test]
    fn activation_validation_rejects_unavailable_custom_node() {
        let root = tempfile::tempdir().unwrap();
        let service = DependencyLockService::new(root.path().join("users"));
        service
            .write("owner", "project", &DependencyLockSpec::default())
            .unwrap();
        let graph = PipelineGraph {
            id: "missing-node".to_string(),
            description: None,
            metadata: None,
            entry_nodes: vec!["custom".to_string()],
            nodes: vec![PipelineNode {
                id: "custom".to_string(),
                kind: "n.x.example.absent".to_string(),
                input_pins: Vec::new(),
                output_pins: vec!["out".to_string()],
                config: serde_json::json!({}),
            }],
            edges: Vec::new(),
        };
        let error = service
            .validate_pipeline_dependencies("owner", "project", &graph)
            .unwrap_err();
        assert_eq!(error.code, "PLATFORM_DEPENDENCY_MISSING");
    }

    #[test]
    fn status_reports_unlocked_node_kinds_required_by_saved_pipelines() {
        let root = tempfile::tempdir().unwrap();
        let users = root.path().join("users");
        let service = DependencyLockService::new(users.clone());
        service
            .write("owner", "project", &DependencyLockSpec::default())
            .unwrap();
        let pipelines = users.join("owner/project/repo/pipelines");
        std::fs::create_dir_all(&pipelines).unwrap();
        std::fs::write(
            pipelines.join("requires-custom.zf.json"),
            br#"{
              "apiVersion":"zebflow.com/v1",
              "kind":"Pipeline",
              "metadata":{"name":"requires-custom"},
              "spec":{
                "id":"requires-custom",
                "entry_nodes":["custom"],
                "nodes":[{
                  "id":"custom",
                  "kind":"n.x.example.thing",
                  "output_pins":["out"]
                }],
                "edges":[]
              }
            }"#,
        )
        .unwrap();

        let report = service
            .status("owner", "project", &ZebflowJsonRweLibraries::new())
            .unwrap();
        assert!(!report.ok);
        assert!(report.items.iter().any(|item| {
            item.family == "node_kind"
                && item.name == "n.x.example.thing"
                && item.status == DependencyResolutionStatus::Missing
        }));
    }

    fn locked_bundle(source: DependencyLockSource, digest: char) -> DependencyLockNodeBundleSpec {
        DependencyLockNodeBundleSpec {
            version: "1.0.0".to_string(),
            source,
            source_id: if source.is_hub() {
                "official.example@1.0.0".to_string()
            } else {
                "project/example".to_string()
            },
            entry: "nodes/example/definition.json".to_string(),
            integrity: format!("sha256:{}", digest.to_string().repeat(64)),
            definitions: vec!["n.x.example.thing".to_string()],
        }
    }

    /// A Hub digest is a claim about the bytes a publisher shipped, so drift is
    /// tampering and must fail closed with the previous lock intact.
    #[test]
    fn hub_bundle_digest_drift_fails_closed() {
        let root = tempfile::tempdir().unwrap();
        let service = DependencyLockService::new(root.path().join("users"));
        let mut lock = DependencyLockSpec::default();
        lock.nodes.bundles.insert(
            "official.example".to_string(),
            locked_bundle(DependencyLockSource::HubPublic, 'a'),
        );
        service.write("owner", "project", &lock).unwrap();

        let error = service
            .record_discovered_node_bundles(
                "owner",
                "project",
                vec![(
                    "example".to_string(),
                    locked_bundle(DependencyLockSource::Project, 'b'),
                )],
            )
            .expect_err("tampered hub bundle must fail closed");
        assert_eq!(error.code, "PLATFORM_DEPENDENCY_INTEGRITY");

        let after = service.read("owner", "project").unwrap();
        assert_eq!(after.nodes.bundles.len(), 1);
        assert_eq!(
            after.nodes.bundles["official.example"].integrity,
            format!("sha256:{}", "a".repeat(64)),
            "a failed reconcile must preserve the previous lock"
        );
    }

    /// Nothing under `data/` is hand-edited, so a project-sourced bundle gets
    /// the same treatment as a Hub one: drift fails closed rather than being
    /// adopted.
    #[test]
    fn project_bundle_digest_drift_also_fails_closed() {
        let root = tempfile::tempdir().unwrap();
        let service = DependencyLockService::new(root.path().join("users"));
        let mut lock = DependencyLockSpec::default();
        lock.nodes.bundles.insert(
            "example".to_string(),
            locked_bundle(DependencyLockSource::Project, 'a'),
        );
        service.write("owner", "project", &lock).unwrap();

        let error = service
            .record_discovered_node_bundles(
                "owner",
                "project",
                vec![(
                    "example".to_string(),
                    locked_bundle(DependencyLockSource::Project, 'b'),
                )],
            )
            .expect_err("project drift must fail closed");
        assert_eq!(error.code, "PLATFORM_DEPENDENCY_INTEGRITY");

        let after = service.read("owner", "project").unwrap();
        assert_eq!(after.nodes.bundles.len(), 1);
        assert_eq!(
            after.nodes.bundles["example"].integrity,
            format!("sha256:{}", "a".repeat(64)),
            "the previous lock survives"
        );
    }
}

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
use crate::platform::model::{ZebflowJsonRweLibraries, slug_segment};
use crate::platform::services::library::LibraryService;
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
    library: Option<Arc<LibraryService>>,
    update_locks: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
}

impl DependencyLockService {
    /// Creates a service without a library resolver.
    ///
    /// This is sufficient for empty locks and canonical reads. Legacy entries
    /// with missing integrity require [`Self::with_library_service`].
    pub fn new(users_root: PathBuf) -> Self {
        Self {
            users_root,
            library: None,
            update_locks: Mutex::new(HashMap::new()),
        }
    }

    /// Creates a service that can resolve and verify legacy library pins.
    pub fn with_library_service(users_root: PathBuf, library: Arc<LibraryService>) -> Self {
        Self {
            users_root,
            library: Some(library),
            update_locks: Mutex::new(HashMap::new()),
        }
    }

    fn repo_path(&self, owner: &str, project: &str) -> PathBuf {
        self.users_root
            .join(slug_segment(owner))
            .join(slug_segment(project))
            .join("repo")
    }

    /// Root that a node bundle's `entry` path resolves against.
    ///
    /// Bundles are materialized artifacts, so they live under `data/` while the
    /// lock that declares them stays in `repo/`. The recorded `entry` string is
    /// unchanged by that split; only its base directory differs.
    fn node_root(&self, owner: &str, project: &str) -> PathBuf {
        self.users_root
            .join(slug_segment(owner))
            .join(slug_segment(project))
            .join("data")
    }

    fn lock_path(&self, owner: &str, project: &str) -> PathBuf {
        self.repo_path(owner, project).join(DEPENDENCY_LOCK_FILE)
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

            let Some((key, locked)) = provider else {
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

    /// Records Hub provenance for bundles written below one installation root.
    pub fn record_hub_node_bundles(
        &self,
        owner: &str,
        project: &str,
        install_root: &str,
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
            bundle.source = DependencyLockSource::Hub;
            bundle.source_id = source_id.to_string();
            let key = if multiple {
                format!("hub/{source_id}/{}", index + 1)
            } else {
                format!("hub/{source_id}")
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
            let Some(library) = &self.library else {
                items.push(DependencyStatusItem {
                    family: "rwe_library",
                    name: name.clone(),
                    status: DependencyResolutionStatus::UnsupportedRuntime,
                    version: locked.version.clone(),
                    source: locked.source.as_str().to_string(),
                    message: "this runtime has no RWE library resolver".to_string(),
                    definitions: Vec::new(),
                });
                continue;
            };
            let expected =
                match library.resolve_lock_entry(name, &requested.version, &requested.source) {
                    Ok(value) => value,
                    Err(error) => {
                        items.push(DependencyStatusItem {
                            family: "rwe_library",
                            name: name.clone(),
                            status: DependencyResolutionStatus::UnsupportedRuntime,
                            version: locked.version.clone(),
                            source: locked.source.as_str().to_string(),
                            message: error.message,
                            definitions: Vec::new(),
                        });
                        continue;
                    }
                };
            let (status, message) = if locked.version != expected.version
                || locked.source != expected.source
                || locked.source_id != expected.source_id
                || locked.entry != expected.entry
            {
                (
                    DependencyResolutionStatus::VersionMismatch,
                    "requested library resolution differs from zeb.lock".to_string(),
                )
            } else if locked.integrity != expected.integrity {
                (
                    DependencyResolutionStatus::IntegrityMismatch,
                    "resolved library bytes do not match zeb.lock".to_string(),
                )
            } else {
                (
                    DependencyResolutionStatus::Resolved,
                    "exact embedded library is available".to_string(),
                )
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

        let repo = self.repo_path(owner, project);
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
        collect_project_pipeline_requirements(
            &repo.join("pipelines"),
            &repo.join("pipelines"),
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

    /// Re-resolves all requested RWE libraries and replaces only that namespace.
    pub fn repair_rwe_libraries(
        &self,
        owner: &str,
        project: &str,
        requested_libraries: &ZebflowJsonRweLibraries,
    ) -> Result<(), PlatformError> {
        let library = self.library.as_ref().ok_or_else(|| {
            PlatformError::new(
                "ZEB_LOCK_REPAIR_UNAVAILABLE",
                "this runtime has no RWE library resolver",
            )
        })?;
        let mut resolved = std::collections::BTreeMap::new();
        let mut requested = requested_libraries.iter().collect::<Vec<_>>();
        requested.sort_by(|left, right| left.0.cmp(right.0));
        for (name, entry) in requested {
            resolved.insert(
                name.clone(),
                library.resolve_lock_entry(name, &entry.version, &entry.source)?,
            );
        }
        self.update(owner, project, |value| {
            value.rwe.libraries = resolved;
        })
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

        let repo = self.repo_path(owner, project);
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
                    source: match entry.source {
                        DependencyLockSource::Embedded => "offline".to_string(),
                        DependencyLockSource::Hub | DependencyLockSource::Project => {
                            return Err(PlatformError::new(
                                "ZEB_LOCK_RWE_SOURCE",
                                "RWE configuration currently supports only embedded libraries",
                            ));
                        }
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
    /// original file remains at `repo/zeb.pre-v1.lock` before canonical bytes
    /// replace `repo/zeb.lock`.
    pub fn migrate_legacy(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<DependencyLockMigration, PlatformError> {
        let path = self.lock_path(owner, project);
        let backup_path = self
            .repo_path(owner, project)
            .join(DEPENDENCY_LOCK_BACKUP_FILE);
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
        let mut migrated = DependencyLockSpec::default();
        for (name, entry) in libraries {
            let resolved = match &self.library {
                Some(library) => {
                    let resolved = library.resolve_lock_entry(
                        &name,
                        entry.version.trim(),
                        entry.source.trim(),
                    )?;
                    if resolved.entry != entry.entry {
                        return Err(PlatformError::new(
                            "ZEB_LOCK_MIGRATE",
                            format!(
                                "legacy entry '{name}' points to '{}', but its registered release points to '{}'",
                                entry.entry, resolved.entry
                            ),
                        ));
                    }
                    if let Some(integrity) = entry.integrity.as_deref()
                        && !integrity.is_empty()
                        && integrity != resolved.integrity
                    {
                        return Err(PlatformError::new(
                            "ZEB_LOCK_MIGRATE",
                            format!("legacy entry '{name}' has a mismatched integrity digest"),
                        ));
                    }
                    resolved
                }
                None => DependencyLockArtifactSpec {
                    version: entry.version,
                    source: match entry.source.as_str() {
                        "offline" => DependencyLockSource::Embedded,
                        other => {
                            return Err(PlatformError::new(
                                "ZEB_LOCK_MIGRATE",
                                format!("legacy library source '{other}' cannot be resolved"),
                            ));
                        }
                    },
                    source_id: format!("zebflow/{name}"),
                    entry: entry.entry,
                    integrity: entry.integrity.filter(|value| !value.is_empty()).ok_or_else(
                        || {
                            PlatformError::new(
                                "ZEB_LOCK_MIGRATE",
                                format!(
                                    "legacy entry '{name}' has no integrity digest and no library resolver is available"
                                ),
                            )
                        },
                    )?,
                },
            };
            migrated.rwe.libraries.insert(name, resolved);
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
            source: DependencyLockSource::Embedded,
            source_id: "zebflow/zeb/example".to_string(),
            entry: "dist/main.mjs".to_string(),
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

    #[test]
    fn legacy_migration_keeps_recovery_copy() {
        let root = tempfile::tempdir().unwrap();
        let service = DependencyLockService::new(root.path().join("users"));
        let path = service.lock_path("owner", "project");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let legacy = format!(
            r#"{{"version":1,"libraries":{{"zeb/example":{{"version":"1.0.0","source":"offline","entry":"dist/main.mjs","integrity":"sha256:{}"}}}}}}"#,
            "a".repeat(64)
        );
        std::fs::write(&path, legacy.as_bytes()).unwrap();

        let result = service.migrate_legacy("owner", "project").unwrap();
        assert_eq!(result.source_format, "legacy_1");
        assert_eq!(
            std::fs::read(result.recovery_path).unwrap(),
            legacy.as_bytes()
        );
        assert_eq!(
            service
                .read("owner", "project")
                .unwrap()
                .rwe
                .libraries
                .len(),
            1
        );
    }

    #[test]
    fn real_legacy_shape_resolves_missing_integrity_from_embedded_bytes() {
        let root = tempfile::tempdir().unwrap();
        let library = Arc::new(LibraryService::from_embedded().unwrap());
        let service =
            DependencyLockService::with_library_service(root.path().join("users"), library);
        let path = service.lock_path("owner", "project");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let legacy = include_bytes!(
            "../../../tests/fixtures/contracts/dependency-lock/legacy-1-complete.json"
        );
        std::fs::write(&path, legacy).unwrap();

        service.migrate_legacy("owner", "project").unwrap();
        let migrated = service.read("owner", "project").unwrap();
        let entry = &migrated.rwe.libraries["zeb/deckgl"];
        assert!(entry.integrity.starts_with("sha256:"));
        assert_eq!(entry.integrity.len(), 71);
    }

    #[test]
    fn explicit_migration_converts_the_previous_rwe_only_envelope() {
        let root = tempfile::tempdir().unwrap();
        let library = Arc::new(LibraryService::from_embedded().unwrap());
        let service =
            DependencyLockService::with_library_service(root.path().join("users"), library);
        let path = service.lock_path("owner", "project");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let previous = format!(
            r#"{{"apiVersion":"zebflow.com/v1","kind":"DependencyLock","metadata":{{"name":"project"}},"spec":{{"libraries":{{"zeb/deckgl":{{"version":"full-9.x","source":"offline","entry":"0.1/runtime/deckgl.bundle.mjs","integrity":"sha256:{}"}}}}}}}}"#,
            "0".repeat(64)
        );
        std::fs::write(&path, previous.as_bytes()).unwrap();

        let migration = service.migrate_legacy("owner", "project").unwrap_err();
        assert_eq!(migration.code, "ZEB_LOCK_MIGRATE");
        assert!(migration.message.contains("mismatched integrity"));
        assert_eq!(std::fs::read(&path).unwrap(), previous.as_bytes());

        let resolved = service
            .library
            .as_ref()
            .unwrap()
            .resolve_lock_entry("zeb/deckgl", "full-9.x", "offline")
            .unwrap();
        let previous = format!(
            r#"{{"apiVersion":"zebflow.com/v1","kind":"DependencyLock","metadata":{{"name":"project"}},"spec":{{"libraries":{{"zeb/deckgl":{{"version":"full-9.x","source":"offline","entry":"0.1/runtime/deckgl.bundle.mjs","integrity":"{}"}}}}}}}}"#,
            resolved.integrity
        );
        std::fs::write(&path, previous.as_bytes()).unwrap();
        let migration = service.migrate_legacy("owner", "project").unwrap();
        assert_eq!(migration.source_format, "rwe_only_v1");
        let migrated = service.read("owner", "project").unwrap();
        assert_eq!(migrated.rwe.libraries["zeb/deckgl"], resolved);
        assert!(migrated.nodes.bundles.is_empty());
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
            !service
                .repo_path("owner", "project")
                .join(DEPENDENCY_LOCK_BACKUP_FILE)
                .exists()
        );
    }

    #[test]
    fn migration_never_overwrites_a_different_recovery_copy() {
        let root = tempfile::tempdir().unwrap();
        let service = DependencyLockService::new(root.path().join("users"));
        let path = service.lock_path("owner", "project");
        let recovery = service
            .repo_path("owner", "project")
            .join(DEPENDENCY_LOCK_BACKUP_FILE);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
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

    #[test]
    fn status_resolves_exact_embedded_library() {
        let root = tempfile::tempdir().unwrap();
        let library = Arc::new(LibraryService::from_embedded().unwrap());
        let service = DependencyLockService::with_library_service(
            root.path().join("users"),
            Arc::clone(&library),
        );
        let mut requested = ZebflowJsonRweLibraries::new();
        requested.insert(
            "zeb/deckgl".to_string(),
            crate::platform::model::ZebflowJsonRweLibraryEntry {
                version: "full-9.x".to_string(),
                source: "offline".to_string(),
            },
        );
        service
            .repair_rwe_libraries("owner", "project", &requested)
            .unwrap();

        let report = service.status("owner", "project", &requested).unwrap();
        assert!(report.ok);
        assert_eq!(report.resolved, 1);
        assert_eq!(report.problems, 0);
    }

    #[test]
    fn status_detects_changed_and_missing_node_bundle() {
        let root = tempfile::tempdir().unwrap();
        let users = root.path().join("users");
        let service = DependencyLockService::new(users.clone());
        let package_dir = users.join("owner/project/data/nodes/example");
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
            "project/example".to_string(),
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
            source_id: "official/example".to_string(),
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
            "hub/official/example".to_string(),
            locked_bundle(DependencyLockSource::Hub, 'a'),
        );
        service.write("owner", "project", &lock).unwrap();

        let error = service
            .record_discovered_node_bundles(
                "owner",
                "project",
                vec![(
                    "project/example".to_string(),
                    locked_bundle(DependencyLockSource::Project, 'b'),
                )],
            )
            .expect_err("tampered hub bundle must fail closed");
        assert_eq!(error.code, "PLATFORM_DEPENDENCY_INTEGRITY");

        let after = service.read("owner", "project").unwrap();
        assert_eq!(after.nodes.bundles.len(), 1);
        assert_eq!(
            after.nodes.bundles["hub/official/example"].integrity,
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
            "project/example".to_string(),
            locked_bundle(DependencyLockSource::Project, 'a'),
        );
        service.write("owner", "project", &lock).unwrap();

        let error = service
            .record_discovered_node_bundles(
                "owner",
                "project",
                vec![(
                    "project/example".to_string(),
                    locked_bundle(DependencyLockSource::Project, 'b'),
                )],
            )
            .expect_err("project drift must fail closed");
        assert_eq!(error.code, "PLATFORM_DEPENDENCY_INTEGRITY");

        let after = service.read("owner", "project").unwrap();
        assert_eq!(after.nodes.bundles.len(), 1);
        assert_eq!(
            after.nodes.bundles["project/example"].integrity,
            format!("sha256:{}", "a".repeat(64)),
            "the previous lock survives"
        );
    }
}

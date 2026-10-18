//! Service for reading and writing canonical `repo/zebflow.yaml` project configuration.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use crate::contracts::kinds::{
    LEGACY_PROJECT_CONFIGURATION_FILE, PROJECT_CONFIGURATION_BACKUP_FILE,
    PROJECT_CONFIGURATION_FILE, ProjectConfigurationContract, ProjectConfigurationSpec,
    decode_legacy_project_configuration,
};
use crate::contracts::{
    ContractMetadata, decode_contract, read_optional_contract_yaml, write_contract_yaml,
};
use crate::infra::execution::placement::ProjectRuntimeProfile;
use crate::infra::io::durable::atomic_write;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    ProjectBootstrapPlan, ResolvedProjectLayout, ZebflowJson, ZebflowJsonAssistant,
    ZebflowJsonDistributionHub, ZebflowJsonRweLibraries, ZebflowJsonRweLibraryEntry, slug_segment,
};
use crate::zebfs::FileBackend;

/// Modification time and size of `path`, or `None` when it does not exist.
/// How long a cached read is trusted without looking at the file again.
/// A write through this service drops the entry at once; this only bounds
/// how late an edit made outside the process is seen. Inside it, a request
/// reads the parse from memory and touches no disk: on some machines one
/// `stat` costs tens of microseconds and they do not run in parallel, which
/// capped a whole server near 1,700 requests a second.
pub const STAMP_TRUST: std::time::Duration = std::time::Duration::from_secs(1);

/// Raised every time a project's configuration is written or forgotten. What
/// is compiled from a configuration (a pipeline's compiled program) notes the
/// generation it was compiled at and compiles again when it moved.
static CONFIG_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The current configuration generation; see [`CONFIG_GENERATION`].
pub fn config_generation() -> u64 {
    CONFIG_GENERATION.load(std::sync::atomic::Ordering::Relaxed)
}

fn file_stamp(path: &Path) -> Option<(std::time::SystemTime, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

/// Returns true if `rel_path` matches a locked path or is inside a locked folder prefix.
pub fn is_template_path_locked(locked: &[String], rel_path: &str) -> bool {
    locked.iter().any(|p| {
        rel_path == p.as_str() || rel_path.starts_with(&format!("{}/", p.trim_end_matches('/')))
    })
}

/// Returns true if moving or deleting `rel_path` would touch a lock: the path
/// itself or a folder above it is locked, or a locked entry lies beneath it
/// (a folder taken away takes everything inside with it).
pub fn is_template_tree_locked(locked: &[String], rel_path: &str) -> bool {
    let rel = rel_path.trim_matches('/');
    if rel.is_empty() {
        return !locked.is_empty();
    }
    is_template_path_locked(locked, rel)
        || locked
            .iter()
            .any(|p| p.trim_matches('/').starts_with(&format!("{rel}/")))
}

/// One cached read of a project's declarations, and the file identity it was
/// read from.
///
/// Layout and file backend are cached together because they come from the same
/// parse of the same file: reading twice would double the cost of the hot path
/// and open a window where the two answers came from different bytes.
struct ConfigCacheEntry {
    /// `None` when the configuration file was absent at the time of the read.
    stamp: Option<(std::time::SystemTime, u64)>,
    /// When the stamp was last compared with the file.
    checked: std::time::Instant,
    layout: ResolvedProjectLayout,
    backend: FileBackend,
}

/// Reads and writes `{data_root}/users/{owner}/{project}/repo/zebflow.yaml`.
pub struct ProjectConfigurationService {
    users_root: PathBuf,
    update_locks: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
    /// Read-mostly: every request reads it several times, a write is a
    /// config change. A read lock lets those reads run side by side.
    declarations: RwLock<HashMap<PathBuf, ConfigCacheEntry>>,
    /// The parsed configuration, by file, with the file identity it was
    /// read from: every pipeline run reads it.
    parsed: RwLock<HashMap<PathBuf, (Option<(std::time::SystemTime, u64)>, std::time::Instant, Arc<ZebflowJson>)>>,
    /// The same parse by owner, then project, with when it was taken and the
    /// configuration generation it was taken at: what a request reads, with
    /// no path built. Any write or change found raises the generation.
    shared: RwLock<HashMap<String, HashMap<String, (std::time::Instant, u64, Arc<ZebflowJson>)>>>,
    /// How long a cached read is trusted: [`STAMP_TRUST`], or zero for a
    /// caller that edits the file behind the service and reads at once.
    trust: std::time::Duration,
}

impl ProjectConfigurationService {
    /// Creates service rooted at `{data_root}/users`.
    pub fn new(users_root: PathBuf) -> Self {
        Self {
            users_root,
            update_locks: Mutex::new(HashMap::new()),
            declarations: RwLock::new(HashMap::new()),
            parsed: RwLock::new(HashMap::new()),
            shared: RwLock::new(HashMap::new()),
            trust: STAMP_TRUST,
        }
    }

    /// The same service, trusting a cached read for `trust` instead.
    pub fn with_trust(mut self, trust: std::time::Duration) -> Self {
        self.trust = trust;
        self
    }

    fn repo_path(&self, owner: &str, project: &str) -> PathBuf {
        self.users_root
            .join(slug_segment(owner))
            .join(slug_segment(project))
            .join("repo")
    }

    /// `.../data/recovery` — where migration safety copies live
    /// (`project-directory.md` §5), never inside `repo/`.
    fn recovery_path(&self, owner: &str, project: &str) -> PathBuf {
        self.users_root
            .join(slug_segment(owner))
            .join(slug_segment(project))
            .join("data")
            .join("recovery")
    }

    fn config_path(&self, owner: &str, project: &str) -> PathBuf {
        self.repo_path(owner, project)
            .join(PROJECT_CONFIGURATION_FILE)
    }

    fn legacy_path(&self, owner: &str, project: &str) -> PathBuf {
        self.repo_path(owner, project)
            .join(LEGACY_PROJECT_CONFIGURATION_FILE)
    }

    /// Returns whether the canonical project configuration file exists.
    pub fn canonical_exists(&self, owner: &str, project: &str) -> bool {
        self.config_path(owner, project).is_file()
    }

    fn update_lock(&self, path: &Path) -> Arc<Mutex<()>> {
        let mut locks = self
            .update_locks
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        locks
            .entry(path.to_path_buf())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Reads `zebflow.yaml`, returning defaults only when the file is missing.
    ///
    /// Malformed and unsupported files are rejected so runtime behavior never
    /// silently changes because an authoritative project file is damaged.
    /// The parse is cached against the file's size and modification time,
    /// as [`Self::project_layout`] is: a pipeline run reads it every time.
    ///
    /// The parse is shared, not copied: every reader holds the same one.
    pub fn read_or_default(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Arc<ZebflowJson>, PlatformError> {
        let generation = config_generation();
        if let Some((at, taken_at, config)) =
            self.shared.read().unwrap_or_else(|err| err.into_inner()).get(owner).and_then(|projects| projects.get(project))
            && *taken_at == generation
            && at.elapsed() < self.trust
        {
            return Ok(config.clone());
        }
        let config = self.read_by_path(owner, project)?;
        self.shared
            .write()
            .unwrap_or_else(|err| err.into_inner())
            .entry(owner.to_string())
            .or_default()
            .insert(project.to_string(), (std::time::Instant::now(), generation, config.clone()));
        Ok(config)
    }

    /// [`Self::read_or_default`] through the cache by file.
    fn read_by_path(&self, owner: &str, project: &str) -> Result<Arc<ZebflowJson>, PlatformError> {
        let path = self.config_path(owner, project);
        if let Some((_, checked, config)) = self.parsed.read().unwrap_or_else(|err| err.into_inner()).get(&path)
            && checked.elapsed() < self.trust
        {
            return Ok(config.clone());
        }
        let stamp = file_stamp(&path);
        if let Some((was, checked, config)) = self.parsed.write().unwrap_or_else(|err| err.into_inner()).get_mut(&path)
            && *was == stamp
        {
            *checked = std::time::Instant::now();
            return Ok(config.clone());
        }
        let config = Arc::new(self.read_path_or_default(&path, &self.legacy_path(owner, project), project)?);
        // Read from disk: new, or changed behind the service.
        CONFIG_GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.parsed
            .write()
            .unwrap_or_else(|err| err.into_inner())
            .insert(path, (stamp, std::time::Instant::now(), config.clone()));
        Ok(config)
    }

    /// Effective repository layout for one project.
    ///
    /// This is on the hot path — every request that resolves a project's
    /// directories asks for it — so the parsed answer is cached against the
    /// configuration file's size and modification time. A file edited outside
    /// this process still changes both, and a write through this service drops
    /// the entry outright, so a stale layout cannot outlive the declaration it
    /// came from.
    pub fn project_layout(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ResolvedProjectLayout, PlatformError> {
        self.declarations(owner, project).map(|(layout, _)| layout)
    }

    /// Effective native file storage backend for one project.
    ///
    /// This is the declaration `src/zebfs/backend.rs` turns into an
    /// implementation. A project that declares nothing resolves to the platform
    /// default; a project that declares a word this build cannot open is
    /// refused here rather than quietly writing its bytes to the default store.
    pub fn project_file_backend(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<FileBackend, PlatformError> {
        self.declarations(owner, project)
            .map(|(_, backend)| backend)
    }

    /// Reads the cached declarations for one project, refreshing on a changed
    /// configuration file.
    fn declarations(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<(ResolvedProjectLayout, FileBackend), PlatformError> {
        let path = self.config_path(owner, project);
        {
            let cache = self
                .declarations
                .read()
                .unwrap_or_else(|err| err.into_inner());
            if let Some(entry) = cache.get(&path)
                && entry.checked.elapsed() < self.trust
            {
                return Ok((entry.layout.clone(), entry.backend));
            }
        }
        let stamp = file_stamp(&path);
        {
            let mut cache = self
                .declarations
                .write()
                .unwrap_or_else(|err| err.into_inner());
            if let Some(entry) = cache.get_mut(&path)
                && entry.stamp == stamp
            {
                entry.checked = std::time::Instant::now();
                return Ok((entry.layout.clone(), entry.backend));
            }
        }
        let config =
            self.read_path_or_default(&path, &self.legacy_path(owner, project), project)?;
        let backend = config.configs.files.effective_backend().map_err(|err| {
            PlatformError::new(
                "PROJECT_CONFIG_READ",
                format!("spec.files.backend {}", err.message),
            )
        })?;
        let layout = config.layout();
        CONFIG_GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut cache = self
            .declarations
            .write()
            .unwrap_or_else(|err| err.into_inner());
        cache.insert(
            path,
            ConfigCacheEntry {
                stamp,
                checked: std::time::Instant::now(),
                layout: layout.clone(),
                backend,
            },
        );
        Ok((layout, backend))
    }

    /// Drops what is cached for one project, for a change made around this
    /// service (an import swapping the repository in).
    pub fn forget_project(&self, owner: &str, project: &str) {
        self.forget_layout(&self.config_path(owner, project));
        if let Some(projects) = self.shared.write().unwrap_or_else(|err| err.into_inner()).get_mut(owner) {
            projects.remove(project);
        }
    }

    fn forget_layout(&self, path: &Path) {
        CONFIG_GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.declarations
            .write()
            .unwrap_or_else(|err| err.into_inner())
            .remove(path);
        self.parsed.write().unwrap_or_else(|err| err.into_inner()).remove(path);
    }

    fn read_path_or_default(
        &self,
        path: &Path,
        legacy_path: &Path,
        project: &str,
    ) -> Result<ZebflowJson, PlatformError> {
        read_optional_contract_yaml::<ProjectConfigurationContract>(path)
            .and_then(|document| {
                let Some(document) = document else {
                    if legacy_path.exists() {
                        return Err(crate::contracts::ContractError::Invalid(format!(
                            "legacy '{}' exists; run the explicit project configuration migration",
                            legacy_path.display()
                        )));
                    }
                    return Ok(ZebflowJson::default());
                };
                if document.metadata.name != project {
                    return Err(crate::contracts::ContractError::Invalid(format!(
                        "metadata.name '{}' must match owning project '{}'",
                        document.metadata.name, project
                    )));
                }
                Ok(document.spec.into())
            })
            .map_err(|err| {
                PlatformError::new(
                    "PROJECT_CONFIG_READ",
                    format!("{} ({})", err, err.category()),
                )
            })
    }

    /// Writes `zebflow.yaml` with durable atomic replacement.
    pub fn write(
        &self,
        owner: &str,
        project: &str,
        config: &ZebflowJson,
    ) -> Result<(), PlatformError> {
        let path = self.config_path(owner, project);
        let lock = self.update_lock(&path);
        let _guard = lock.lock().unwrap_or_else(|err| err.into_inner());
        self.write_unlocked(&path, project, config)
    }

    fn write_unlocked(
        &self,
        path: &Path,
        project: &str,
        config: &ZebflowJson,
    ) -> Result<(), PlatformError> {
        self.forget_layout(path);
        let written = write_contract_yaml::<ProjectConfigurationContract>(
            path,
            ContractMetadata::named(project),
            ProjectConfigurationSpec::from(config.clone()),
        )
        .map_err(|err| {
            PlatformError::new(
                "PROJECT_CONFIG_WRITE",
                format!("{} ({})", err, err.category()),
            )
        });
        // Again after: a read between the first forget and the write cached
        // the old file, and would now be trusted for STAMP_TRUST.
        self.forget_layout(path);
        written
    }

    /// Reads `zebflow.yaml`, applies a mutation, and writes it back atomically.
    pub fn update<F>(&self, owner: &str, project: &str, f: F) -> Result<ZebflowJson, PlatformError>
    where
        F: FnOnce(&mut ZebflowJson),
    {
        self.update_fallible(owner, project, |config| {
            f(config);
            Ok(())
        })
    }

    /// Runs a fallible mutation while holding the per-project configuration lock.
    ///
    /// This is restricted to platform services that coordinate `zebflow.yaml`
    /// with another durable project contract. Returning an error leaves the
    /// configuration file unchanged.
    pub(crate) fn update_fallible<F>(
        &self,
        owner: &str,
        project: &str,
        f: F,
    ) -> Result<ZebflowJson, PlatformError>
    where
        F: FnOnce(&mut ZebflowJson) -> Result<(), PlatformError>,
    {
        let path = self.config_path(owner, project);
        let legacy_path = self.legacy_path(owner, project);
        let lock = self.update_lock(&path);
        let _guard = lock.lock().unwrap_or_else(|err| err.into_inner());
        let mut cfg = self.read_path_or_default(&path, &legacy_path, project)?;
        f(&mut cfg)?;
        self.write_unlocked(&path, project, &cfg)?;
        Ok(cfg)
    }

    /// Explicitly migrates `repo/zebflow.json` into canonical `repo/zebflow.yaml`.
    ///
    /// Normal reads never invoke this path. The original JSON bytes remain in
    /// `data/recovery/zebflow-config-{date}.json` so an operator can inspect
    /// or restore them — `data/recovery/`, not `repo/`, because a
    /// machine-generated recovery copy must never enter the git-tracked
    /// repository (`project-directory.md` §1, §5).
    pub fn migrate_legacy_json(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ProjectConfigurationMigration, PlatformError> {
        let path = self.config_path(owner, project);
        let legacy_path = self.legacy_path(owner, project);
        let backup_path = self.recovery_path(owner, project).join(format!(
            "{PROJECT_CONFIGURATION_BACKUP_FILE}-{}.json",
            crate::platform::model::recovery_date_stamp()
        ));
        let lock = self.update_lock(&path);
        let _guard = lock.lock().unwrap_or_else(|err| err.into_inner());

        if path.exists() {
            return Err(PlatformError::new(
                "PROJECT_CONFIG_MIGRATE",
                format!("'{}' already exists", path.display()),
            ));
        }
        let bytes = std::fs::read(&legacy_path).map_err(|err| {
            PlatformError::new(
                "PROJECT_CONFIG_MIGRATE",
                format!("failed reading '{}': {err}", legacy_path.display()),
            )
        })?;
        let (metadata, spec, source_format) =
            match decode_contract::<ProjectConfigurationContract>(&bytes) {
                Ok(document) => (document.metadata, document.spec, "contract_json"),
                Err(_) => (
                    ContractMetadata::named(project),
                    decode_legacy_project_configuration(&bytes).map_err(|err| {
                        PlatformError::new("PROJECT_CONFIG_MIGRATE", err.to_string())
                    })?,
                    "legacy_1.0",
                ),
            };
        if metadata.name != project {
            return Err(PlatformError::new(
                "PROJECT_CONFIG_MIGRATE",
                format!(
                    "metadata.name '{}' must match owning project '{}'",
                    metadata.name, project
                ),
            ));
        }

        match std::fs::read(&backup_path) {
            Ok(existing) if existing != bytes => {
                return Err(PlatformError::new(
                    "PROJECT_CONFIG_MIGRATE",
                    format!(
                        "recovery copy '{}' already exists with different content",
                        backup_path.display()
                    ),
                ));
            }
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                if let Some(parent) = backup_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                atomic_write(&backup_path, &bytes).map_err(|err| {
                    PlatformError::new(
                        "PROJECT_CONFIG_MIGRATE",
                        format!("failed writing recovery copy: {err}"),
                    )
                })?;
            }
            Err(err) => {
                return Err(PlatformError::new(
                    "PROJECT_CONFIG_MIGRATE",
                    format!("failed reading recovery copy: {err}"),
                ));
            }
        }
        self.forget_layout(&path);
        write_contract_yaml::<ProjectConfigurationContract>(&path, metadata, spec.clone())
            .map_err(|err| PlatformError::new("PROJECT_CONFIG_MIGRATE", err.to_string()))?;
        self.forget_layout(&path);
        let reopened = read_optional_contract_yaml::<ProjectConfigurationContract>(&path)
            .map_err(|err| PlatformError::new("PROJECT_CONFIG_MIGRATE", err.to_string()))?
            .ok_or_else(|| {
                PlatformError::new(
                    "PROJECT_CONFIG_MIGRATE",
                    "canonical configuration disappeared after migration",
                )
            })?;
        if reopened.spec != spec {
            return Err(PlatformError::new(
                "PROJECT_CONFIG_MIGRATE",
                "canonical configuration did not roundtrip after migration",
            ));
        }
        std::fs::remove_file(&legacy_path).map_err(|err| {
            PlatformError::new(
                "PROJECT_CONFIG_MIGRATE",
                format!("migration succeeded but failed removing legacy source: {err}"),
            )
        })?;

        Ok(ProjectConfigurationMigration {
            source_format: source_format.to_string(),
            canonical_path: path,
            recovery_path: backup_path,
        })
    }

    /// Writes the project title to `zebflow.yaml`, preserving other fields.
    pub fn set_project_title(
        &self,
        owner: &str,
        project: &str,
        title: &str,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |cfg| {
            cfg.metadata.title = title.to_string();
        })?;
        Ok(())
    }

    /// Gets the project title from `zebflow.yaml`, falling back to the project slug.
    pub fn get_project_title(&self, owner: &str, project: &str) -> Result<String, PlatformError> {
        let cfg = self.read_or_default(owner, project)?;
        if cfg.metadata.title.trim().is_empty() {
            Ok(project.replace('-', " "))
        } else {
            Ok(cfg.metadata.title.clone())
        }
    }

    /// Returns the hub distribution contract for one project.
    pub fn get_hub_distribution(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ZebflowJsonDistributionHub, PlatformError> {
        Ok(self.read_or_default(owner, project)?.distribution.hub.clone())
    }

    pub fn set_hub_distribution(
        &self,
        owner: &str,
        project: &str,
        hub: ZebflowJsonDistributionHub,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |cfg| {
            cfg.distribution.hub = hub;
        })?;
        Ok(())
    }

    /// Returns the assistant section of `zebflow.yaml`.
    pub fn get_assistant(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ZebflowJsonAssistant, PlatformError> {
        Ok(self.read_or_default(owner, project)?.configs.assistant.clone())
    }

    /// Returns the portable runtime profile section of `zebflow.yaml`.
    pub fn get_runtime_profile(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ProjectRuntimeProfile, PlatformError> {
        Ok(self.read_or_default(owner, project)?.configs.runtime.clone())
    }

    /// Sets the portable runtime profile section of `zebflow.yaml`, preserving other fields.
    pub fn set_runtime_profile(
        &self,
        owner: &str,
        project: &str,
        runtime: ProjectRuntimeProfile,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |cfg| {
            cfg.configs.runtime = runtime;
        })?;
        Ok(())
    }

    /// Returns the bootstrap/activation plan section of `zebflow.yaml`.
    pub fn get_bootstrap(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ProjectBootstrapPlan, PlatformError> {
        Ok(self.read_or_default(owner, project)?.configs.bootstrap.clone())
    }

    /// Sets the bootstrap/activation plan section of `zebflow.yaml`, preserving other fields.
    pub fn set_bootstrap(
        &self,
        owner: &str,
        project: &str,
        bootstrap: ProjectBootstrapPlan,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |cfg| {
            cfg.configs.bootstrap = bootstrap;
        })?;
        Ok(())
    }

    /// Sets the assistant section of `zebflow.yaml`, preserving other fields.
    pub fn set_assistant(
        &self,
        owner: &str,
        project: &str,
        assistant: ZebflowJsonAssistant,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |cfg| {
            cfg.configs.assistant = assistant;
        })?;
        Ok(())
    }

    /// Returns the `rwe.libraries` map from `zebflow.yaml`.
    pub fn get_rwe_libraries(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ZebflowJsonRweLibraries, PlatformError> {
        Ok(self.read_or_default(owner, project)?.configs.rwe.libraries.clone())
    }

    /// Adds or updates one enabled library entry in `rwe.libraries`.
    pub fn enable_rwe_library(
        &self,
        owner: &str,
        project: &str,
        name: &str,
        version: &str,
        source: &str,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |cfg| {
            cfg.configs.rwe.libraries.insert(
                name.to_string(),
                ZebflowJsonRweLibraryEntry {
                    version: version.to_string(),
                    source: source.to_string(),
                },
            );
        })?;
        Ok(())
    }

    /// Removes one library entry from `rwe.libraries`. No-op if not present.
    pub fn disable_rwe_library(
        &self,
        owner: &str,
        project: &str,
        name: &str,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |cfg| {
            cfg.configs.rwe.libraries.remove(name);
        })?;
        Ok(())
    }

    /// Returns whether `rel_path` is in the locked templates list.
    pub fn is_template_locked(
        &self,
        owner: &str,
        project: &str,
        rel_path: &str,
    ) -> Result<bool, PlatformError> {
        let cfg = self.read_or_default(owner, project)?;
        Ok(is_template_path_locked(
            &cfg.configs.locks.templates,
            rel_path,
        ))
    }

    /// Returns whether moving or deleting `rel_path` would take a locked
    /// template with it — see [`is_template_tree_locked`].
    pub fn is_template_tree_locked(
        &self,
        owner: &str,
        project: &str,
        rel_path: &str,
    ) -> Result<bool, PlatformError> {
        let cfg = self.read_or_default(owner, project)?;
        Ok(is_template_tree_locked(&cfg.configs.locks.templates, rel_path))
    }

    /// Adds or removes `rel_path` from the locked templates list.
    pub fn set_template_locked(
        &self,
        owner: &str,
        project: &str,
        rel_path: &str,
        locked: bool,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |cfg| {
            let templates = &mut cfg.configs.locks.templates;
            if locked {
                if !templates.iter().any(|p| p == rel_path) {
                    templates.push(rel_path.to_string());
                }
            } else {
                templates.retain(|p| p != rel_path);
            }
        })?;
        Ok(())
    }

    /// Returns all locked template paths from `zebflow.yaml`.
    pub fn get_locked_templates(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<String>, PlatformError> {
        Ok(self
            .read_or_default(owner, project)?
            .configs
            .locks
            .templates
            .clone())
    }

    /// Initializes zebflow.yaml with defaults if it doesn't already exist.
    pub fn ensure_initialized(
        &self,
        owner: &str,
        project: &str,
        title: &str,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |cfg| {
            if cfg.metadata.title.trim().is_empty() && !title.trim().is_empty() {
                cfg.metadata.title = title.to_string();
            }
        })?;
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectConfigurationMigration {
    pub source_format: String,
    pub canonical_path: PathBuf,
    pub recovery_path: PathBuf,
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};

    use super::*;

    /// A file edited behind the service is seen once the trust window has
    /// passed, and at once after `forget_project`; inside the window the
    /// cached read answers and the disk is not touched.
    #[test]
    fn an_outside_edit_is_seen_after_the_trust_window_or_a_forget() {
        let root = tempfile::tempdir().unwrap();
        let service = ProjectConfigurationService::new(root.path().join("users"));
        let mut config = ZebflowJson::default();
        config.metadata.title = "Before".to_string();
        service.write("owner", "project", &config).unwrap();
        assert_eq!(service.read_or_default("owner", "project").unwrap().metadata.title, "Before");

        let path = service.config_path("owner", "project");
        let edited = std::fs::read_to_string(&path).unwrap().replace("Before", "After");
        std::fs::write(&path, edited).unwrap();
        assert_eq!(service.read_or_default("owner", "project").unwrap().metadata.title, "Before");
        service.forget_project("owner", "project");
        assert_eq!(service.read_or_default("owner", "project").unwrap().metadata.title, "After");
    }

    #[test]
    fn missing_config_defaults_but_malformed_and_future_configs_fail() {
        let root = tempfile::tempdir().unwrap();
        let service = ProjectConfigurationService::new(root.path().join("users")).with_trust(std::time::Duration::ZERO);
        assert!(service.read_or_default("owner", "project").is_ok());

        let path = service.config_path("owner", "project");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"spec: [").unwrap();
        assert_eq!(
            service
                .read_or_default("owner", "project")
                .unwrap_err()
                .code,
            "PROJECT_CONFIG_READ"
        );

        std::fs::write(
            &path,
            b"apiVersion: zebflow.com/v2\nkind: ProjectConfiguration\nmetadata:\n  name: project\nspec: {}\n",
        )
        .unwrap();
        assert_eq!(
            service
                .read_or_default("owner", "project")
                .unwrap_err()
                .code,
            "PROJECT_CONFIG_READ"
        );
    }

    #[test]
    fn config_write_is_strict_and_roundtrips() {
        let root = tempfile::tempdir().unwrap();
        let service = ProjectConfigurationService::new(root.path().join("users")).with_trust(std::time::Duration::ZERO);
        let mut config = ZebflowJson::default();
        config.metadata.title = "Stable".to_string();
        service.write("owner", "project", &config).unwrap();
        assert_eq!(
            service
                .read_or_default("owner", "project")
                .unwrap()
                .metadata
                .title,
            "Stable"
        );

        let path = service.config_path("owner", "project");
        let written = std::fs::read_to_string(&path).unwrap();
        let written: serde_yaml_ng::Value = serde_yaml_ng::from_str(&written).unwrap();
        assert_eq!(written["spec"]["profile"]["title"], "Stable");
        assert_eq!(
            written["spec"]["data"],
            serde_yaml_ng::Value::Mapping(Default::default())
        );
        assert!(written["spec"].get("configs").is_none());

        std::fs::write(
            &path,
            b"apiVersion: zebflow.com/v1\nkind: DependencyLock\nmetadata:\n  name: project\nspec: {}\n",
        )
        .unwrap();
        assert!(service.read_or_default("owner", "project").is_err());
        service.write("owner", "project", &config).unwrap();
        assert_eq!(
            service
                .read_or_default("owner", "project")
                .unwrap()
                .metadata
                .title,
            "Stable"
        );
    }

    #[test]
    fn legacy_json_requires_explicit_migration_and_keeps_recovery_copy() {
        let root = tempfile::tempdir().unwrap();
        let service = ProjectConfigurationService::new(root.path().join("users"));
        let legacy = service.legacy_path("owner", "project");
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::write(
            &legacy,
            br#"{
                "version":"1.0",
                "metadata":{"title":"Migrated","description":""},
                "configs":{"data":{},"pipelines":{"nodes":{}}},
                "distribution":{}
            }"#,
        )
        .unwrap();

        let err = service.read_or_default("owner", "project").unwrap_err();
        assert_eq!(err.code, "PROJECT_CONFIG_READ");
        assert!(
            err.message
                .contains("explicit project configuration migration")
        );

        let result = service.migrate_legacy_json("owner", "project").unwrap();
        assert_eq!(result.source_format, "legacy_1.0");
        assert!(!legacy.exists());
        assert!(result.canonical_path.exists());
        assert!(result.recovery_path.exists());
        assert_eq!(
            service
                .read_or_default("owner", "project")
                .unwrap()
                .metadata
                .title,
            "Migrated"
        );
    }

    #[test]
    fn failed_legacy_migration_preserves_the_source() {
        let root = tempfile::tempdir().unwrap();
        let service = ProjectConfigurationService::new(root.path().join("users"));
        let legacy = service.legacy_path("owner", "project");
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        let invalid = br#"{"version":"2.0"}"#;
        std::fs::write(&legacy, invalid).unwrap();

        assert!(service.migrate_legacy_json("owner", "project").is_err());
        assert_eq!(std::fs::read(&legacy).unwrap(), invalid);
        assert!(!service.config_path("owner", "project").exists());
        assert!(
            !service.recovery_path("owner", "project").exists(),
            "a refused migration writes nothing, not even the recovery directory"
        );
    }

    #[test]
    fn migration_never_overwrites_a_different_recovery_copy() {
        let root = tempfile::tempdir().unwrap();
        let service = ProjectConfigurationService::new(root.path().join("users"));
        let legacy = service.legacy_path("owner", "project");
        let recovery = service.recovery_path("owner", "project").join(format!(
            "{PROJECT_CONFIGURATION_BACKUP_FILE}-{}.json",
            crate::platform::model::recovery_date_stamp()
        ));
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::create_dir_all(recovery.parent().unwrap()).unwrap();
        std::fs::write(
            &legacy,
            br#"{"version":"1.0","metadata":{},"configs":{},"distribution":{}}"#,
        )
        .unwrap();
        std::fs::write(&recovery, b"previous recovery").unwrap();

        let error = service.migrate_legacy_json("owner", "project").unwrap_err();
        assert_eq!(error.code, "PROJECT_CONFIG_MIGRATE");
        assert_eq!(std::fs::read(&recovery).unwrap(), b"previous recovery");
        assert!(legacy.exists());
        assert!(!service.config_path("owner", "project").exists());
    }

    #[test]
    fn a_project_without_a_declared_layout_keeps_a_layout_free_file() {
        let root = tempfile::tempdir().unwrap();
        let service = ProjectConfigurationService::new(root.path().join("users"));
        service
            .ensure_initialized("owner", "project", "Untouched")
            .unwrap();

        let written = std::fs::read_to_string(service.config_path("owner", "project")).unwrap();
        let written: serde_yaml_ng::Value = serde_yaml_ng::from_str(&written).unwrap();
        assert!(written["spec"].get("layout").is_none());

        let config = service.read_or_default("owner", "project").unwrap();
        assert_eq!(
            config.layout(),
            crate::platform::model::ResolvedProjectLayout::platform_default()
        );
    }

    #[test]
    fn a_declared_layout_survives_an_unrelated_configuration_update() {
        let root = tempfile::tempdir().unwrap();
        let service = ProjectConfigurationService::new(root.path().join("users"));
        service
            .update("owner", "project", |cfg| {
                cfg.configs.layout.source = Some("src".to_string());
            })
            .unwrap();
        service
            .set_project_title("owner", "project", "Renamed")
            .unwrap();

        let config = service.read_or_default("owner", "project").unwrap();
        assert_eq!(config.metadata.title, "Renamed");
        assert_eq!(config.configs.layout.source.as_deref(), Some("src"));
        assert_eq!(config.layout().docs, "docs");
    }

    #[test]
    fn concurrent_updates_preserve_independent_changes() {
        let root = tempfile::tempdir().unwrap();
        let service = Arc::new(ProjectConfigurationService::new(root.path().join("users")));
        service
            .ensure_initialized("owner", "project", "Initial")
            .unwrap();
        let barrier = Arc::new(Barrier::new(3));

        let title_service = Arc::clone(&service);
        let title_barrier = Arc::clone(&barrier);
        let title = std::thread::spawn(move || {
            title_barrier.wait();
            title_service
                .set_project_title("owner", "project", "Concurrent")
                .unwrap();
        });
        let lock_service = Arc::clone(&service);
        let lock_barrier = Arc::clone(&barrier);
        let template_lock = std::thread::spawn(move || {
            lock_barrier.wait();
            lock_service
                .set_template_locked("owner", "project", "pages/system", true)
                .unwrap();
        });
        barrier.wait();
        title.join().unwrap();
        template_lock.join().unwrap();

        let config = service.read_or_default("owner", "project").unwrap();
        assert_eq!(config.metadata.title, "Concurrent");
        assert_eq!(config.configs.locks.templates, ["pages/system"]);
    }
}

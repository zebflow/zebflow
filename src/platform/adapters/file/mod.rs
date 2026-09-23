//! Swappable file-system adapters for Zebflow project assets.

pub mod selection;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fs, process::Command};

use crate::infra::io::durable::migrate_tier_entry;
use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{FileAdapterKind, ProjectFileLayout, slug_segment};
use crate::platform::services::project_config::ProjectConfigurationService;
use crate::zebfs::{FileBackend, FileStore, S3Config};

/// The credential kind that reaches a bucket.
pub const S3_CREDENTIAL_KIND: &str = "s3";

/// File adapter contract used by project service.
pub trait FileAdapter: Send + Sync {
    /// Stable adapter id.
    fn id(&self) -> &'static str;
    /// Ensure root layout exists.
    fn initialize(&self) -> Result<(), PlatformError>;
    /// Ensure one project folder tree exists and return resolved paths.
    fn ensure_project_layout(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ProjectFileLayout, PlatformError>;
    /// The credential this instance selected for a project's bucket
    /// (`data/store/files-backend.json`), read without resolving the store,
    /// so a project whose selection is wrong can still be shown and repaired.
    fn files_backend_selection(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<selection::FilesBackendSelection, PlatformError>;
    /// Persists that selection.
    fn select_files_backend(
        &self,
        owner: &str,
        project: &str,
        selection: &selection::FilesBackendSelection,
    ) -> Result<(), PlatformError>;
}

/// Filesystem adapter implementation.
pub struct FilesystemFileAdapter {
    root: PathBuf,
    /// Where a project's declared layout is read from.
    ///
    /// The adapter owns no parser of its own: `zebflow.yaml` has exactly one
    /// reader, and a second one here could accept a document the real reader
    /// refuses.
    configs: Arc<ProjectConfigurationService>,
    /// Where the credential behind an object-store backend is read from.
    ///
    /// Absent on the adapters built for tests and migrations that never
    /// open a bucket; a project declaring `s3` on such an adapter refuses
    /// rather than resolving to disk.
    credentials: Option<Arc<dyn DataAdapter>>,
}

impl FilesystemFileAdapter {
    /// Creates filesystem adapter rooted at `{data_root}/users`.
    pub fn new(root: PathBuf, configs: Arc<ProjectConfigurationService>) -> Self {
        Self {
            root,
            configs,
            credentials: None,
        }
    }

    /// Lets the adapter resolve the credential a project's bucket needs.
    pub fn with_credentials(mut self, data: Arc<dyn DataAdapter>) -> Self {
        self.credentials = Some(data);
        self
    }

    /// The bucket a project declaring `s3` keeps its files in: the credential
    /// this instance selected for it (`data/store/files-backend.json`), read
    /// into a config. Every way this can fail is named, because a project
    /// whose bytes went to a store it did not declare is worse than one that
    /// will not start.
    fn s3_store(
        &self,
        owner: &str,
        project: &str,
        store_dir: &Path,
    ) -> Result<S3Config, PlatformError> {
        let refuse = |message: String| PlatformError::new("PROJECT_FILES_BACKEND", message);
        let selection = selection::read_selection(store_dir)?;
        let Some(credential_id) = selection
            .credential_id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())
        else {
            return Err(refuse(format!(
                "spec.files.backend is '{}' but no credential is selected for it; choose an '{S3_CREDENTIAL_KIND}' credential on the Files page",
                FileBackend::S3.as_str()
            )));
        };
        let Some(data) = &self.credentials else {
            return Err(refuse(format!(
                "spec.files.backend is '{}' but this adapter has no credential store to resolve '{credential_id}' from",
                FileBackend::S3.as_str()
            )));
        };
        let credential = data
            .get_project_credential(&slug_segment(owner), &slug_segment(project), credential_id)?
            .ok_or_else(|| {
                refuse(format!(
                    "the credential '{credential_id}' selected for this project's files does not exist"
                ))
            })?;
        if credential.kind != S3_CREDENTIAL_KIND {
            return Err(refuse(format!(
                "the credential '{credential_id}' selected for this project's files is of kind '{}', not '{S3_CREDENTIAL_KIND}'",
                credential.kind
            )));
        }
        // Confidential from here on, in every diagnostic of every node it
        // reaches — the same rule the credential service applies.
        crate::platform::services::credential::register_confidential(
            owner,
            project,
            &credential.secret,
        );
        S3Config::from_credential(&credential.secret)
            .map_err(|err| refuse(format!("credential '{credential_id}': {}", err.message)))
    }

    fn project_root(&self, owner: &str, project: &str) -> PathBuf {
        self.root
            .join(slug_segment(owner))
            .join(slug_segment(project))
    }

    fn ensure_git_repo(repo_dir: &PathBuf, git_dir: &PathBuf) -> Result<(), PlatformError> {
        if git_dir.exists() {
            return Ok(());
        }
        let status = Command::new("git")
            .arg("init")
            .arg("-q")
            .arg("--initial-branch=main")
            .current_dir(repo_dir)
            .status()
            .map_err(|e| PlatformError::new("PLATFORM_GIT_INIT", e.to_string()))?;
        if status.success() {
            return Ok(());
        }
        Err(PlatformError::new(
            "PLATFORM_GIT_INIT",
            format!("git init failed with status {status}"),
        ))
    }
}

impl FileAdapter for FilesystemFileAdapter {
    fn id(&self) -> &'static str {
        "file.filesystem"
    }

    fn initialize(&self) -> Result<(), PlatformError> {
        std::fs::create_dir_all(&self.root)?;
        Ok(())
    }

    fn files_backend_selection(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<selection::FilesBackendSelection, PlatformError> {
        selection::read_selection(&self.project_root(owner, project).join("data").join("store"))
    }

    fn select_files_backend(
        &self,
        owner: &str,
        project: &str,
        selection: &selection::FilesBackendSelection,
    ) -> Result<(), PlatformError> {
        selection::write_selection(
            &self.project_root(owner, project).join("data").join("store"),
            selection,
        )
    }

    fn ensure_project_layout(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ProjectFileLayout, PlatformError> {
        let root = self.project_root(owner, project);
        let data_dir = root.join("data");
        let files_dir = root.join("files");
        let repo_dir = root.join("repo");
        let repo_git_dir = repo_dir.join(".git");
        let project_config_file =
            repo_dir.join(crate::contracts::kinds::PROJECT_CONFIGURATION_FILE);

        // The declaration takes effect here. A project that declares nothing
        // resolves to the platform defaults, and a malformed or unmigrated
        // configuration returns an error rather than being reinterpreted as
        // the defaults, because guessing here would silently relocate a
        // project's entire source tree.
        let repo_layout = self.configs.project_layout(owner, project)?;
        // Same rule as the layout above: a backend this build cannot open is
        // refused here rather than silently resolved to the default, because a
        // project whose bytes went to a store it did not declare is worse than
        // a project that will not start.
        let file_backend = self.configs.project_file_backend(owner, project)?;
        let file_store = match file_backend {
            FileBackend::Zebfs => FileStore::Local(files_dir.clone()),
            FileBackend::S3 => {
                FileStore::S3(self.s3_store(owner, project, &data_dir.join("store"))?)
            }
        };

        let resolved = ProjectFileLayout {
            root,
            data_dir,
            files_dir,
            repo_dir,
            repo_git_dir,
            project_config_file,
            repo_layout,
            file_store,
        };

        // A pre-tier project has its whole runtime cache sitting at the old
        // `data/runtime`, not just the `pipelines`/`agent_docs` children this
        // adapter used to scaffold individually — so the entire directory
        // moves in one `rename` onto its new `data/cache` home, carrying
        // those children with it. This must run before the base-dirs loop
        // below, which would otherwise create an empty `data/cache` first and
        // turn the migration into a same-path no-op.
        migrate_tier_entry(
            &resolved.data_dir.join("runtime"),
            &resolved.data_cache_dir(),
        )
        .map_err(|err| PlatformError::new("PLATFORM_DATA_TIER_MIGRATE", err.to_string()))?;

        // Same shape for the INSTALLED tier: a pre-contract project keeps its
        // node bundles at `data/nodes`, and the whole directory moves in one
        // `rename` onto `data/hub/nodes` — before the base-dirs loop below
        // scaffolds an empty `data/hub/nodes`, which would turn this into a
        // both-paths refusal. The locked `entry` strings
        // (`nodes/{slug}/definition.json`) are untouched; only the base they
        // resolve against (`data` → `data/hub`) changes with the move.
        migrate_tier_entry(
            &resolved.data_dir.join("nodes"),
            &resolved.data_hub_nodes_dir(),
        )
        .map_err(|err| PlatformError::new("PLATFORM_DATA_TIER_MIGRATE", err.to_string()))?;

        // Base dirs
        for dir in [
            &resolved.root,
            &resolved.data_dir,
            &resolved.data_store_dir(),
            &resolved.data_cache_dir(),
            &resolved.data_cache_pipelines_dir(),
            &resolved.data_cache_agent_docs_dir(),
            &resolved.data_hub_dir(),
            &resolved.data_hub_nodes_dir(),
            &resolved.data_hub_rwe_libraries_dir(),
            &resolved.data_recovery_dir(),
            &resolved.data_logs_dir(),
            &resolved.files_dir,
            // `repo/` and nothing inside it. The layout entries name where a
            // kind is looked for; they are not directories the platform makes.
            // A project starts empty, and a folder appears when something is
            // written into it -- every repository writer creates its own
            // parents. Scaffolding them here ran on every request, so a folder
            // the author deleted came back on the next page load.
            &resolved.repo_dir,
        ] {
            fs::create_dir_all(dir)?;
        }

        Self::ensure_git_repo(&resolved.repo_dir, &resolved.repo_git_dir)?;

        Ok(resolved)
    }
}

/// Builds selected file adapter.
///
/// `credentials` is the store an object-store backend's credential is read
/// from; without one, a project declaring `s3` refuses by name.
pub fn build_file_adapter(
    kind: FileAdapterKind,
    data_root: PathBuf,
    configs: Arc<ProjectConfigurationService>,
    credentials: Option<Arc<dyn DataAdapter>>,
) -> Arc<dyn FileAdapter> {
    match kind {
        FileAdapterKind::Filesystem => {
            let adapter = FilesystemFileAdapter::new(data_root.join("users"), configs);
            Arc::new(match credentials {
                Some(data) => adapter.with_credentials(data),
                None => adapter,
            })
        }
    }
}

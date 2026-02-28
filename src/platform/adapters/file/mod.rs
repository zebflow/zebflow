//! Swappable file-system adapters for Zebflow project assets.

use std::path::PathBuf;
use std::sync::Arc;
use std::{fs, process::Command};

use crate::platform::error::PlatformError;
use crate::platform::model::{FileAdapterKind, ProjectFileLayout, slug_segment};

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
}

/// Filesystem adapter implementation.
pub struct FilesystemFileAdapter {
    root: PathBuf,
}

impl FilesystemFileAdapter {
    /// Creates filesystem adapter rooted at `{data_root}/users`.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn project_root(&self, owner: &str, project: &str) -> PathBuf {
        self.root
            .join(slug_segment(owner))
            .join(slug_segment(project))
    }

    fn ensure_git_repo(app_dir: &PathBuf, git_dir: &PathBuf) -> Result<(), PlatformError> {
        if git_dir.exists() {
            return Ok(());
        }
        let status = Command::new("git")
            .arg("init")
            .arg("-q")
            .current_dir(app_dir)
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

    fn ensure_project_layout(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ProjectFileLayout, PlatformError> {
        let root = self.project_root(owner, project);
        let data_dir = root.join("data");
        let data_sekejap_dir = data_dir.join("sekejap");
        let data_sqlite_dir = data_dir.join("sqlite");
        let data_sqlite_file = data_sqlite_dir.join("project.db");
        let files_dir = root.join("files");
        let app_dir = root.join("app");
        let app_git_dir = app_dir.join(".git");
        let app_pipelines_dir = app_dir.join("pipelines");
        let app_templates_dir = app_dir.join("templates");
        let app_components_dir = app_dir.join("components");

        for dir in [
            &root,
            &data_dir,
            &data_sekejap_dir,
            &data_sqlite_dir,
            &files_dir,
            &app_dir,
            &app_pipelines_dir,
            &app_templates_dir,
            &app_components_dir,
        ] {
            fs::create_dir_all(dir)?;
        }

        if !data_sqlite_file.exists() {
            let _ = fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .open(&data_sqlite_file)?;
        }

        Self::ensure_git_repo(&app_dir, &app_git_dir)?;

        Ok(ProjectFileLayout {
            root,
            data_dir,
            data_sekejap_dir,
            data_sqlite_file,
            files_dir,
            app_dir,
            app_git_dir,
            app_pipelines_dir,
            app_templates_dir,
            app_components_dir,
        })
    }
}

/// Builds selected file adapter.
pub fn build_file_adapter(kind: FileAdapterKind, data_root: PathBuf) -> Arc<dyn FileAdapter> {
    match kind {
        FileAdapterKind::Filesystem => {
            Arc::new(FilesystemFileAdapter::new(data_root.join("users")))
        }
    }
}

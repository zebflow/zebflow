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

    fn ensure_git_repo(repo_dir: &PathBuf, git_dir: &PathBuf) -> Result<(), PlatformError> {
        if git_dir.exists() {
            return Ok(());
        }
        let status = Command::new("git")
            .arg("init")
            .arg("-q")
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

    fn ensure_project_layout(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ProjectFileLayout, PlatformError> {
        let root = self.project_root(owner, project);
        let data_dir = root.join("data");
        let data_runtime_dir = data_dir.join("runtime");
        let data_runtime_pipelines_dir = data_runtime_dir.join("pipelines");
        let data_sekejap_dir = data_dir.join("sekejap");
        let files_dir = root.join("files");
        let repo_dir = root.join("repo");
        let repo_git_dir = repo_dir.join(".git");
        let repo_pipelines_dir = repo_dir.join("pipelines");
        let repo_docs_dir = repo_dir.join("docs");
        let zebflow_json_file = repo_dir.join("zebflow.json");
        let agent_docs_dir = data_runtime_dir.join("agent_docs");

        // Base dirs
        for dir in [
            &root,
            &data_dir,
            &data_runtime_dir,
            &data_runtime_pipelines_dir,
            &data_sekejap_dir,
            &files_dir,
            &repo_dir,
            &repo_pipelines_dir,
            &repo_docs_dir,
            &agent_docs_dir,
        ] {
            fs::create_dir_all(dir)?;
        }

        // Only create assets/ and styles/ as default subdirs.
        // All other folders (automation, web, components, lib, etc.) are created
        // explicitly by the user — not auto-scaffolded on every request.
        for subdir in ["assets", "styles"] {
            fs::create_dir_all(repo_pipelines_dir.join(subdir))?;
        }

        Self::ensure_git_repo(&repo_dir, &repo_git_dir)?;

        Ok(ProjectFileLayout {
            root,
            data_dir,
            data_runtime_dir,
            data_runtime_pipelines_dir,
            data_sekejap_dir,
            files_dir,
            repo_dir,
            repo_git_dir,
            repo_pipelines_dir,
            repo_docs_dir,
            zebflow_json_file,
            agent_docs_dir,
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

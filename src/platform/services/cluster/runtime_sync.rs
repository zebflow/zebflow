//! Project runtime bundle sync orchestration service.
//!
//! This service prepares repo-owned project state so it can be materialized on a worker and also
//! knows how to apply that bundle on the receiving side. The first implementation focuses on the
//! repo working tree plus portable runtime metadata.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use crate::infra::execution::sync::{
    ProjectBootstrapPlan, ProjectBundleFile, ProjectBundleIdentity, ProjectRuntimeBundle,
};
use crate::platform::adapters::data::DataAdapter;
use crate::platform::adapters::file::FileAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    CreateProjectRequest, ProjectRuntimeMaterializationRequest, now_ts, slug_segment,
};
use crate::platform::services::{PipelineRuntimeService, ProjectService, ZebflowJsonService};

/// Product-facing runtime-sync helper.
#[derive(Clone)]
pub struct ClusterRuntimeSyncService {
    data: Arc<dyn DataAdapter>,
    file: Arc<dyn FileAdapter>,
    projects: Arc<ProjectService>,
    zebflow_cfg: Arc<ZebflowJsonService>,
    pipeline_runtime: Arc<PipelineRuntimeService>,
}

impl ClusterRuntimeSyncService {
    /// Create a new runtime-sync service.
    pub fn new(
        data: Arc<dyn DataAdapter>,
        file: Arc<dyn FileAdapter>,
        projects: Arc<ProjectService>,
        zebflow_cfg: Arc<ZebflowJsonService>,
        pipeline_runtime: Arc<PipelineRuntimeService>,
    ) -> Self {
        Self {
            data,
            file,
            projects,
            zebflow_cfg,
            pipeline_runtime,
        }
    }

    /// Build a portable runtime bundle from the current repo-owned state.
    pub fn build_bundle(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ProjectRuntimeBundle, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let cfg = self.zebflow_cfg.read_or_default(&owner, &project);
        let identity = self
            .projects
            .get_project(&owner, &project)?
            .map(|project_info| ProjectBundleIdentity {
                owner: project_info.owner,
                project: project_info.project,
                title: project_info.title,
                revision: current_git_revision(&layout.repo_dir),
            })
            .unwrap_or(ProjectBundleIdentity {
                owner: owner.clone(),
                project: project.clone(),
                title: String::new(),
                revision: current_git_revision(&layout.repo_dir),
            });

        Ok(ProjectRuntimeBundle {
            bundle_id: format!("{}-{}-{}", owner, project, now_ts()),
            identity,
            created_at: now_ts(),
            runtime_profile: cfg.runtime,
            bootstrap: cfg.bootstrap,
            repo_files: collect_repo_files(&layout.repo_dir)?,
            ..Default::default()
        })
    }

    /// Build the richer in-cluster materialization payload used for worker sync.
    pub fn build_materialization_request(
        &self,
        owner: &str,
        project: &str,
        credentials: Vec<crate::platform::model::ProjectCredential>,
        db_connections: Vec<crate::platform::model::ProjectDbConnection>,
    ) -> Result<ProjectRuntimeMaterializationRequest, PlatformError> {
        Ok(ProjectRuntimeMaterializationRequest {
            bundle: self.build_bundle(owner, project)?,
            credentials,
            db_connections,
        })
    }

    /// Rebuild local runtime metadata from the current repo working tree and auto-activate
    /// pipelines declared in `zebflow.json.bootstrap`.
    pub fn refresh_local_repo_state(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let cfg = self.zebflow_cfg.read_or_default(&owner, &project);
        reindex_project_sources(
            self.projects.as_ref(),
            &layout.repo_pipelines_dir,
            &owner,
            &project,
        )?;
        activate_bootstrap_plan(
            self.projects.as_ref(),
            self.pipeline_runtime.as_ref(),
            &owner,
            &project,
            &cfg.bootstrap,
        )?;
        self.pipeline_runtime.refresh_project(&owner, &project)?;
        Ok(())
    }

    /// Apply one portable runtime bundle onto the local node.
    pub fn apply_bundle(&self, bundle: &ProjectRuntimeBundle) -> Result<(), PlatformError> {
        let owner = slug_segment(&bundle.identity.owner);
        let project = slug_segment(&bundle.identity.project);
        let title = if bundle.identity.title.trim().is_empty() {
            project.replace('-', " ")
        } else {
            bundle.identity.title.trim().to_string()
        };
        self.projects.create_or_update_project(
            &owner,
            &CreateProjectRequest {
                project: project.clone(),
                title: Some(title),
                local_branch: None,
                runtime: Default::default(),
            },
        )?;
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        clear_repo_worktree(&layout.repo_dir)?;
        for entry in &bundle.repo_files {
            let path = sanitize_bundle_path(&layout.repo_dir, &entry.path)?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let bytes = BASE64.decode(entry.data_base64.as_bytes()).map_err(|err| {
                PlatformError::new(
                    "CLUSTER_RUNTIME_BUNDLE_DECODE",
                    format!("failed decoding bundle file '{}': {err}", entry.path),
                )
            })?;
            fs::write(path, bytes)?;
        }
        self.zebflow_cfg.update(&owner, &project, |cfg| {
            cfg.runtime = bundle.runtime_profile.clone();
            cfg.bootstrap = bundle.bootstrap.clone();
        })?;
        self.refresh_local_repo_state(&owner, &project)?;
        Ok(())
    }

    /// Apply an in-cluster materialization payload onto the local node.
    pub fn apply_materialization_request(
        &self,
        request: &ProjectRuntimeMaterializationRequest,
    ) -> Result<(), PlatformError> {
        self.apply_bundle(&request.bundle)?;
        for credential in &request.credentials {
            self.data.put_project_credential(credential)?;
        }
        for connection in &request.db_connections {
            self.data.put_project_db_connection(connection)?;
        }
        Ok(())
    }
}

fn current_git_revision(repo_dir: &Path) -> String {
    std::process::Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .unwrap_or_default()
}

fn collect_repo_files(repo_dir: &Path) -> Result<Vec<ProjectBundleFile>, PlatformError> {
    let mut files = Vec::new();
    let mut stack = vec![repo_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) => {
                return Err(PlatformError::new(
                    "CLUSTER_RUNTIME_BUNDLE_READ",
                    format!("failed reading repo directory '{}': {err}", dir.display()),
                ));
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default();
            if name == ".git" {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if !path.is_file() {
                continue;
            }
            let rel_path = path
                .strip_prefix(repo_dir)
                .map_err(|err| {
                    PlatformError::new(
                        "CLUSTER_RUNTIME_BUNDLE_PATH",
                        format!("failed deriving relative path: {err}"),
                    )
                })?
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = fs::read(&path)?;
            files.push(ProjectBundleFile {
                path: rel_path,
                data_base64: BASE64.encode(bytes),
            });
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn clear_repo_worktree(repo_dir: &Path) -> Result<(), PlatformError> {
    let entries = fs::read_dir(repo_dir).map_err(|err| {
        PlatformError::new(
            "CLUSTER_RUNTIME_BUNDLE_CLEAR",
            format!("failed reading repo root '{}': {err}", repo_dir.display()),
        )
    })?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if name == ".git" {
            continue;
        }
        if path.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

fn sanitize_bundle_path(repo_dir: &Path, rel_path: &str) -> Result<PathBuf, PlatformError> {
    let rel_path = rel_path.replace('\\', "/");
    if rel_path.is_empty() || rel_path.starts_with('/') || rel_path.contains("..") {
        return Err(PlatformError::new(
            "CLUSTER_RUNTIME_BUNDLE_PATH",
            format!("invalid bundle path '{rel_path}'"),
        ));
    }
    Ok(repo_dir.join(rel_path))
}

fn reindex_project_sources(
    projects: &ProjectService,
    repo_root: &Path,
    owner: &str,
    project: &str,
) -> Result<(), PlatformError> {
    let mut stack = vec![repo_root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)?.flatten() {
            let path = entry.path();
            let fname = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default();
            if fname.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let rel = match path.strip_prefix(repo_root) {
                Ok(value) => value.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };
            if !rel.ends_with(".zf.json") {
                continue;
            }
            let source = fs::read_to_string(&path)?;
            let file_rel_path = format!("pipelines/{rel}");
            let graph_description = serde_json::from_str::<crate::pipeline::PipelineGraph>(&source)
                .ok()
                .and_then(|graph| graph.description)
                .unwrap_or_default();
            let trigger_kind = derive_trigger_kind_from_source(&source).unwrap_or_default();
            projects.upsert_pipeline_definition(
                owner,
                project,
                &file_rel_path,
                "",
                &graph_description,
                &trigger_kind,
                &source,
            )?;
        }
    }
    Ok(())
}

fn activate_bootstrap_plan(
    projects: &ProjectService,
    pipeline_runtime: &PipelineRuntimeService,
    owner: &str,
    project: &str,
    bootstrap: &ProjectBootstrapPlan,
) -> Result<(), PlatformError> {
    if bootstrap.activate.is_empty() {
        return Ok(());
    }
    for meta in projects.list_pipeline_meta_rows(owner, project)? {
        if bootstrap
            .activate
            .iter()
            .any(|pattern| path_glob_matches(pattern, &meta.file_rel_path))
        {
            let _ = projects.activate_pipeline_definition(owner, project, &meta.file_rel_path)?;
        }
    }
    pipeline_runtime.refresh_project(owner, project)?;
    Ok(())
}

fn path_glob_matches(pattern: &str, candidate: &str) -> bool {
    let pattern_parts: Vec<&str> = pattern
        .split('/')
        .filter(|value| !value.is_empty())
        .collect();
    let candidate_parts: Vec<&str> = candidate
        .split('/')
        .filter(|value| !value.is_empty())
        .collect();
    match_path_segments(&pattern_parts, &candidate_parts)
}

fn match_path_segments(pattern: &[&str], candidate: &[&str]) -> bool {
    if pattern.is_empty() {
        return candidate.is_empty();
    }
    if pattern[0] == "**" {
        if match_path_segments(&pattern[1..], candidate) {
            return true;
        }
        return !candidate.is_empty() && match_path_segments(pattern, &candidate[1..]);
    }
    if candidate.is_empty() {
        return false;
    }
    if !match_segment(pattern[0], candidate[0]) {
        return false;
    }
    match_path_segments(&pattern[1..], &candidate[1..])
}

fn match_segment(pattern: &str, candidate: &str) -> bool {
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let candidate_chars: Vec<char> = candidate.chars().collect();
    match_segment_chars(&pattern_chars, &candidate_chars)
}

fn match_segment_chars(pattern: &[char], candidate: &[char]) -> bool {
    if pattern.is_empty() {
        return candidate.is_empty();
    }
    match pattern[0] {
        '*' => {
            if match_segment_chars(&pattern[1..], candidate) {
                return true;
            }
            !candidate.is_empty() && match_segment_chars(pattern, &candidate[1..])
        }
        '?' => !candidate.is_empty() && match_segment_chars(&pattern[1..], &candidate[1..]),
        ch => {
            !candidate.is_empty()
                && ch == candidate[0]
                && match_segment_chars(&pattern[1..], &candidate[1..])
        }
    }
}

fn derive_trigger_kind_from_source(source: &str) -> Option<String> {
    let graph = serde_json::from_str::<crate::pipeline::PipelineGraph>(source).ok()?;
    graph
        .nodes
        .iter()
        .find_map(|node| match node.kind.as_str() {
            "n.trigger.webhook" => Some("webhook".to_string()),
            "n.trigger.schedule" => Some("schedule".to_string()),
            "n.trigger.ws" => Some("ws".to_string()),
            "n.trigger.memsubscribe" => Some("memsubscribe".to_string()),
            "n.trigger.function" => Some("function".to_string()),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::path_glob_matches;

    #[test]
    fn bootstrap_globs_support_recursive_pipeline_patterns() {
        assert!(path_glob_matches(
            "pipelines/pages/**/*.zf.json",
            "pipelines/pages/demo/safety.zf.json"
        ));
        assert!(path_glob_matches(
            "pipelines/api/*.zf.json",
            "pipelines/api/auth.zf.json"
        ));
        assert!(!path_glob_matches(
            "pipelines/api/*.zf.json",
            "pipelines/api/admin/auth.zf.json"
        ));
    }
}

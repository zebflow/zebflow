//! Local runtime state refresh for one project.
//!
//! This office rebuilds its own runtime metadata from its own working tree:
//! pipeline sources under `repo/` are re-indexed, the `zebflow.yaml`
//! bootstrap plan is applied, and the pipeline runtime is reloaded.
//!
//! It is a purely local operation. Moving a project between two Zebflow
//! instances is `ProjectBundle`'s job, in either direction
//! (`docs/contracts/stability-matrix.md` row 12); the master-to-worker copier
//! that used to live beside this function is retired.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::contracts::kinds::decode_pipeline_graph;
use crate::platform::adapters::file::FileAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{ProjectBootstrapPlan, slug_segment};
use crate::platform::services::project::normalize_pipeline_glob;
use crate::platform::services::{
    PipelineRuntimeService, ProjectConfigurationService, ProjectService,
};

/// Product-facing local runtime refresh helper.
#[derive(Clone)]
pub struct ClusterRuntimeSyncService {
    file: Arc<dyn FileAdapter>,
    projects: Arc<ProjectService>,
    zebflow_cfg: Arc<ProjectConfigurationService>,
    pipeline_runtime: Arc<PipelineRuntimeService>,
}

impl ClusterRuntimeSyncService {
    /// Create a new local runtime refresh service.
    pub fn new(
        file: Arc<dyn FileAdapter>,
        projects: Arc<ProjectService>,
        zebflow_cfg: Arc<ProjectConfigurationService>,
        pipeline_runtime: Arc<PipelineRuntimeService>,
    ) -> Self {
        Self {
            file,
            projects,
            zebflow_cfg,
            pipeline_runtime,
        }
    }

    /// Rebuild local runtime metadata from the current repo working tree and auto-activate
    /// pipelines declared in `zebflow.yaml` under `spec.bootstrap`.
    pub fn refresh_local_repo_state(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let cfg = self.zebflow_cfg.read_or_default(&owner, &project)?;
        reindex_project_sources(
            self.projects.as_ref(),
            &layout.repo_source_dir(),
            &owner,
            &project,
        )?;
        activate_bootstrap_plan(
            self.projects.as_ref(),
            self.pipeline_runtime.as_ref(),
            &owner,
            &project,
            &cfg.configs.bootstrap,
        )?;
        self.pipeline_runtime.refresh_project(&owner, &project)?;
        Ok(())
    }
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
            // `rel` is already relative to the source root, which is exactly
            // what identity is. It used to be re-prefixed here.
            let file_rel_path = rel;
            let graph_description = decode_pipeline_graph(source.as_bytes())
                .ok()
                .and_then(|document| document.spec.description)
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
    // Bootstrap patterns are matched through the identity rule so a plan
    // written before identity became source-relative still selects the
    // pipelines it named.
    let layout = projects.project_layout(owner, project)?;
    let patterns: Vec<String> = bootstrap
        .activate
        .iter()
        .map(|pattern| normalize_pipeline_glob(&layout.repo_layout, pattern))
        .collect();
    for meta in projects.list_pipeline_meta_rows(owner, project)? {
        if patterns
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
    let graph = decode_pipeline_graph(source.as_bytes()).ok()?.spec;
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
    use super::{normalize_pipeline_glob, path_glob_matches};
    use crate::platform::model::ResolvedProjectLayout;

    #[test]
    fn bootstrap_globs_support_recursive_pipeline_patterns() {
        assert!(path_glob_matches(
            "pages/**/*.zf.json",
            "pages/demo/safety.zf.json"
        ));
        assert!(path_glob_matches("api/*.zf.json", "api/auth.zf.json"));
        assert!(!path_glob_matches(
            "api/*.zf.json",
            "api/admin/auth.zf.json"
        ));
    }

    #[test]
    fn a_bootstrap_plan_written_against_the_old_identity_still_selects() {
        // The tolerance only has something to strip when the project gathers
        // its source under a folder. Undeclared, the source is the repository
        // and `pipelines/` is an ordinary directory name.
        let mut layout = ResolvedProjectLayout::platform_default();
        layout.source = "pipelines".to_string();
        let pattern = normalize_pipeline_glob(&layout, "pipelines/pages/**/*.zf.json");
        assert_eq!(pattern, "pages/**/*.zf.json");
        assert!(path_glob_matches(&pattern, "pages/demo/safety.zf.json"));
    }
}

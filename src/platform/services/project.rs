//! Project management service.

use std::collections::BTreeSet;
use std::fs;
use std::sync::Arc;

use crate::platform::adapters::data::DataAdapter;
use crate::platform::adapters::file::FileAdapter;
use crate::platform::adapters::project_data::ProjectDataFactory;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    CreateProjectRequest, PipelineBreadcrumb, PipelineFolderItem, PipelineMeta,
    PipelineRegistryItem, PipelineRegistryListing, PlatformProject, ProjectFileLayout,
    normalize_virtual_path, now_ts, slug_segment,
};

/// Project service backed by swappable data + file adapters.
pub struct ProjectService {
    data: Arc<dyn DataAdapter>,
    file: Arc<dyn FileAdapter>,
    project_data: Arc<dyn ProjectDataFactory>,
}

impl ProjectService {
    /// Creates project service.
    pub fn new(
        data: Arc<dyn DataAdapter>,
        file: Arc<dyn FileAdapter>,
        project_data: Arc<dyn ProjectDataFactory>,
    ) -> Self {
        Self {
            data,
            file,
            project_data,
        }
    }

    /// Lists projects by owner.
    pub fn list_projects(&self, owner: &str) -> Result<Vec<PlatformProject>, PlatformError> {
        self.data.list_projects(owner)
    }

    /// Gets one project by owner/slug.
    pub fn get_project(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Option<PlatformProject>, PlatformError> {
        self.data.get_project(owner, project)
    }

    /// Creates or updates project metadata + required folder layout.
    pub fn create_or_update_project(
        &self,
        owner: &str,
        req: &CreateProjectRequest,
    ) -> Result<(PlatformProject, ProjectFileLayout), PlatformError> {
        let owner = slug_segment(owner);
        if owner.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_PROJECT_INVALID",
                "owner must not be empty",
            ));
        }
        let project = slug_segment(&req.project);
        if project.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_PROJECT_INVALID",
                "project must not be empty",
            ));
        }

        let now = now_ts();
        let existing = self.data.get_project(&owner, &project)?;
        let created_at = existing.as_ref().map(|p| p.created_at).unwrap_or(now);
        let title = req
            .title
            .clone()
            .unwrap_or_else(|| project.replace('-', " "));

        let record = PlatformProject {
            owner: owner.clone(),
            project: project.clone(),
            title,
            created_at,
            updated_at: now,
        };
        self.data.put_project(&record)?;
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        self.project_data.initialize_project(&layout)?;

        let metas = self.data.list_pipeline_meta(&owner, &project)?;
        if metas.is_empty() {
            self.seed_default_pipelines(&owner, &project)?;
        }
        Ok((record, layout))
    }

    /// Upserts one pipeline source file + metadata catalog entry.
    pub fn upsert_pipeline_definition(
        &self,
        owner: &str,
        project: &str,
        virtual_path: &str,
        name: &str,
        title: &str,
        description: &str,
        trigger_kind: &str,
        source: &str,
    ) -> Result<PipelineMeta, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let name = slug_segment(name);
        if owner.is_empty() || project.is_empty() || name.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_PIPELINE_INVALID",
                "owner/project/name must not be empty",
            ));
        }
        if self.data.get_project(&owner, &project)?.is_none() {
            return Err(PlatformError::new(
                "PLATFORM_PIPELINE_INVALID",
                "project not found",
            ));
        }

        let layout = self.file.ensure_project_layout(&owner, &project)?;
        self.project_data.initialize_project(&layout)?;

        let vpath = normalize_virtual_path(virtual_path);
        let (file_rel_path, file_abs_path) = self.pipeline_file_paths(&layout, &vpath, &name)?;
        if let Some(parent) = file_abs_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_abs_path, source)?;

        let now = now_ts();
        let existing = self
            .data
            .list_pipeline_meta(&owner, &project)?
            .into_iter()
            .find(|m| normalize_virtual_path(&m.virtual_path) == vpath && m.name == name);
        let created_at = existing.as_ref().map(|m| m.created_at).unwrap_or(now);
        let meta = PipelineMeta {
            owner,
            project,
            name: name.clone(),
            title: if title.trim().is_empty() {
                name.replace('-', " ")
            } else {
                title.trim().to_string()
            },
            virtual_path: vpath,
            file_rel_path,
            description: description.trim().to_string(),
            trigger_kind: if trigger_kind.trim().is_empty() {
                "webhook".to_string()
            } else {
                trigger_kind.trim().to_string()
            },
            hash: stable_hash_hex(source),
            created_at,
            updated_at: now,
        };
        self.data.put_pipeline_meta(&meta)?;
        Ok(meta)
    }

    /// Returns registry hierarchy at one virtual path.
    pub fn list_pipeline_registry(
        &self,
        owner: &str,
        project: &str,
        current_virtual_path: &str,
        base_route: &str,
    ) -> Result<PipelineRegistryListing, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let current_path = normalize_virtual_path(current_virtual_path);
        let rows = self.data.list_pipeline_meta(&owner, &project)?;

        let mut folders = BTreeSet::new();
        let mut pipelines = Vec::new();
        for m in rows {
            let vp = normalize_virtual_path(&m.virtual_path);
            if vp == current_path {
                pipelines.push(PipelineRegistryItem {
                    name: m.name,
                    title: m.title,
                    description: m.description,
                    trigger_kind: m.trigger_kind,
                    file_rel_path: m.file_rel_path,
                });
                continue;
            }
            if let Some(rem) = path_remainder(&current_path, &vp)
                && let Some(seg) = rem.split('/').next()
            {
                let seg = seg.trim();
                if !seg.is_empty() {
                    folders.insert(seg.to_string());
                }
            }
        }
        pipelines.sort_by(|a, b| a.name.cmp(&b.name));

        let folder_items = folders
            .into_iter()
            .map(|name| {
                let next = if current_path == "/" {
                    format!("/{name}")
                } else {
                    format!("{current_path}/{name}")
                };
                PipelineFolderItem {
                    name: name.clone(),
                    path: format!("{base_route}?path={next}"),
                }
            })
            .collect::<Vec<_>>();

        let mut breadcrumbs = vec![PipelineBreadcrumb {
            name: "root".to_string(),
            path: format!("{base_route}?path=/"),
            show_divider: false,
        }];
        if current_path != "/" {
            let mut accum = String::new();
            for seg in current_path.trim_start_matches('/').split('/') {
                if seg.trim().is_empty() {
                    continue;
                }
                accum.push('/');
                accum.push_str(seg);
                breadcrumbs.push(PipelineBreadcrumb {
                    name: seg.to_string(),
                    path: format!("{base_route}?path={accum}"),
                    show_divider: true,
                });
            }
        }

        Ok(PipelineRegistryListing {
            current_path,
            breadcrumbs,
            folders: folder_items,
            pipelines,
        })
    }

    fn seed_default_pipelines(&self, owner: &str, project: &str) -> Result<(), PlatformError> {
        let list_posts = r#"{
  "kind": "zebflow.pipeline",
  "version": "0.1",
  "id": "blog-list-posts",
  "metadata": {"virtual_path":"/contents/blog"},
  "nodes": []
}"#;
        let get_post = r#"{
  "kind": "zebflow.pipeline",
  "version": "0.1",
  "id": "blog-get-post",
  "metadata": {"virtual_path":"/contents/blog"},
  "nodes": []
}"#;
        let send_digest = r#"{
  "kind": "zebflow.pipeline",
  "version": "0.1",
  "id": "email-send-digest",
  "metadata": {"virtual_path":"/automation/email"},
  "nodes": []
}"#;

        let _ = self.upsert_pipeline_definition(
            owner,
            project,
            "/contents/blog",
            "list-posts",
            "List Posts",
            "Serve blog list payload",
            "webhook",
            list_posts,
        )?;
        let _ = self.upsert_pipeline_definition(
            owner,
            project,
            "/contents/blog",
            "get-post",
            "Get Post",
            "Serve post detail payload",
            "webhook",
            get_post,
        )?;
        let _ = self.upsert_pipeline_definition(
            owner,
            project,
            "/automation/email",
            "send-digest",
            "Send Digest",
            "Scheduled email digest",
            "schedule",
            send_digest,
        )?;
        Ok(())
    }

    fn pipeline_file_paths(
        &self,
        layout: &ProjectFileLayout,
        virtual_path: &str,
        name: &str,
    ) -> Result<(String, std::path::PathBuf), PlatformError> {
        let vpath = normalize_virtual_path(virtual_path);
        let filename = format!("{}.zf.json", slug_segment(name));
        let rel = if vpath == "/" {
            format!("pipelines/{filename}")
        } else {
            format!("pipelines/{}/{}", vpath.trim_start_matches('/'), filename)
        };
        let abs = layout.app_dir.join(&rel);
        if !abs.starts_with(&layout.app_dir) {
            return Err(PlatformError::new(
                "PLATFORM_PIPELINE_PATH",
                "resolved path escaped app root",
            ));
        }
        Ok((rel, abs))
    }
}

fn path_remainder(current: &str, candidate: &str) -> Option<String> {
    if current == "/" {
        let rem = candidate.trim_start_matches('/');
        if rem.is_empty() {
            None
        } else {
            Some(rem.to_string())
        }
    } else {
        let prefix = format!("{current}/");
        candidate
            .strip_prefix(&prefix)
            .map(std::string::ToString::to_string)
    }
}

fn stable_hash_hex(input: &str) -> String {
    // FNV-1a 64-bit: deterministic and lightweight for change tracking.
    let mut h: u64 = 0xcbf29ce484222325;
    for b in input.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x00000100000001B3);
    }
    format!("{h:016x}")
}

//! Project management service.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use crate::platform::adapters::data::DataAdapter;
use crate::platform::adapters::file::FileAdapter;
use crate::platform::adapters::project_data::ProjectDataFactory;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    AgentDocItem, CreateProjectRequest, PipelineBreadcrumb, PipelineFolderItem, PipelineMeta,
    PipelineRegistryItem, PipelineRegistryListing, PlatformProject, ProjectDocItem,
    ProjectFileLayout, RegistryFileItem, TemplateCreateKind, TemplateCreateRequest,
    TemplateFilePayload, TemplateGitStatusItem, TemplateMoveRequest, TemplateSaveRequest,
    TemplateTreeItem, TemplateWorkspaceListing, normalize_virtual_path, now_ts, slug_segment,
};
use crate::platform::model::ZebLock;
use crate::platform::services::project_config::ZebflowJsonService;
use crate::platform::services::zeb_lock::ZebLockService;


/// Project service backed by swappable data + file adapters.
pub struct ProjectService {
    data: Arc<dyn DataAdapter>,
    file: Arc<dyn FileAdapter>,
    project_data: Arc<dyn ProjectDataFactory>,
    zebflow_cfg: Arc<ZebflowJsonService>,
    zeb_lock: Arc<ZebLockService>,
}

impl ProjectService {
    /// Creates project service.
    pub fn new(
        data: Arc<dyn DataAdapter>,
        file: Arc<dyn FileAdapter>,
        project_data: Arc<dyn ProjectDataFactory>,
        zebflow_cfg: Arc<ZebflowJsonService>,
        zeb_lock: Arc<ZebLockService>,
    ) -> Self {
        Self {
            data,
            file,
            project_data,
            zebflow_cfg,
            zeb_lock,
        }
    }

    /// Lists projects by owner, populating title from zebflow.json.
    pub fn list_projects(&self, owner: &str) -> Result<Vec<PlatformProject>, PlatformError> {
        let mut projects = self.data.list_projects(owner)?;
        for p in &mut projects {
            p.title = self.zebflow_cfg.get_project_title(&p.owner, &p.project);
        }
        Ok(projects)
    }

    /// Gets one project by owner/slug, populating title from zebflow.json.
    pub fn get_project(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Option<PlatformProject>, PlatformError> {
        let Some(mut p) = self.data.get_project(owner, project)? else {
            return Ok(None);
        };
        p.title = self.zebflow_cfg.get_project_title(&p.owner, &p.project);
        Ok(Some(p))
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
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_string();
        let title = if title.is_empty() {
            project.replace('-', " ")
        } else {
            title
        };

        let record = PlatformProject {
            owner: owner.clone(),
            project: project.clone(),
            title: title.clone(),
            created_at,
            updated_at: now,
        };
        self.data.put_project(&record)?;
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        // Write title to zebflow.json (Layer 2)
        self.zebflow_cfg.ensure_initialized(&owner, &project, &title)?;
        // Write zeb.lock if it doesn't exist yet
        self.zeb_lock.write_if_missing(&owner, &project, &ZebLock {
            version: 1,
            ..Default::default()
        })?;
        self.project_data.initialize_project(&layout)?;
        self.ensure_default_template_workspace(&layout)?;

        Ok((record, layout))
    }

    /// Upserts one pipeline source file + metadata catalog entry.
    ///
    /// `file_rel_path` is the canonical identifier, e.g. `"pipelines/api/my-hook.zf.json"`.
    /// Name and virtual_path are derived from it automatically.
    pub fn upsert_pipeline_definition(
        &self,
        owner: &str,
        project: &str,
        file_rel_path: &str,
        title: &str,
        description: &str,
        trigger_kind: &str,
        source: &str,
    ) -> Result<PipelineMeta, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        // Normalize file_rel_path: ensure it starts with "pipelines/" and ends with ".zf.json"
        let file_rel_path = normalize_pipeline_file_rel_path(file_rel_path);
        let name = name_from_file_rel_path(&file_rel_path);
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

        let file_abs_path = self.pipeline_abs_path(&layout, &file_rel_path)?;
        if let Some(parent) = file_abs_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_abs_path, source)?;

        let vpath = virtual_path_from_file_rel_path(&file_rel_path);
        let now = now_ts();
        let existing = self.get_pipeline_meta_by_file_id(&owner, &project, &file_rel_path)?;
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
            active_hash: existing.as_ref().and_then(|m| m.active_hash.clone()),
            activated_at: existing.as_ref().and_then(|m| m.activated_at),
            created_at,
            updated_at: now,
        };
        self.data.put_pipeline_meta(&meta)?;
        Ok(meta)
    }

    /// Lists all pipeline metadata rows for one project.
    pub fn list_pipeline_meta_rows(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<PipelineMeta>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let mut rows = self.data.list_pipeline_meta(&owner, &project)?;
        // Re-derive virtual_path from file_rel_path (source of truth).
        for m in &mut rows {
            m.virtual_path = virtual_path_from_file_rel_path(&m.file_rel_path);
        }
        rows.sort_by(|a, b| a.file_rel_path.cmp(&b.file_rel_path));
        Ok(rows)
    }

    /// Returns one pipeline metadata row by stable file id (`file_rel_path`).
    pub fn get_pipeline_meta_by_file_id(
        &self,
        owner: &str,
        project: &str,
        file_id: &str,
    ) -> Result<Option<PipelineMeta>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let wanted = normalize_pipeline_file_rel_path(file_id.trim());
        let meta = self
            .data
            .list_pipeline_meta(&owner, &project)?
            .into_iter()
            .find(|m| normalize_pipeline_file_rel_path(&m.file_rel_path) == wanted)
            .map(|mut m| {
                // Re-derive virtual_path from file_rel_path (source of truth).
                m.virtual_path = virtual_path_from_file_rel_path(&m.file_rel_path);
                m
            });
        Ok(meta)
    }

    /// Reads current working-tree source for one pipeline file.
    pub fn read_pipeline_source(
        &self,
        owner: &str,
        project: &str,
        file_rel_path: &str,
    ) -> Result<String, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let abs = layout.repo_dir.join(file_rel_path);
        if !abs.starts_with(&layout.repo_dir) {
            return Err(PlatformError::new(
                "PLATFORM_PIPELINE_PATH",
                "resolved pipeline path escaped app root",
            ));
        }
        if !abs.is_file() {
            return Err(PlatformError::new(
                "PLATFORM_PIPELINE_MISSING",
                format!("pipeline file '{}' not found", file_rel_path),
            ));
        }
        Ok(fs::read_to_string(abs)?)
    }

    /// Promotes the current working-tree pipeline source to the production runtime snapshot.
    pub fn activate_pipeline_definition(
        &self,
        owner: &str,
        project: &str,
        file_rel_path: &str,
    ) -> Result<PipelineMeta, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let Some(mut meta) = self.get_pipeline_meta_by_file_id(&owner, &project, file_rel_path)? else {
            return Err(PlatformError::new(
                "PLATFORM_PIPELINE_MISSING",
                format!("pipeline '{}' not found", file_rel_path),
            ));
        };
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let source = self.read_pipeline_source(&owner, &project, &meta.file_rel_path)?;
        let current_hash = stable_hash_hex(&source);
        let snapshot_path = self.runtime_pipeline_snapshot_path(
            &layout,
            &meta.file_rel_path,
            &current_hash,
        )?;
        if let Some(parent) = snapshot_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&snapshot_path, source)?;

        meta.hash = current_hash.clone();
        meta.active_hash = Some(current_hash);
        meta.activated_at = Some(now_ts());
        meta.updated_at = now_ts();
        self.data.put_pipeline_meta(&meta)?;
        Ok(meta)
    }

    /// Removes one pipeline from the production runtime set while leaving the working tree intact.
    pub fn deactivate_pipeline_definition(
        &self,
        owner: &str,
        project: &str,
        file_rel_path: &str,
    ) -> Result<PipelineMeta, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let Some(mut meta) = self.get_pipeline_meta_by_file_id(&owner, &project, file_rel_path)? else {
            return Err(PlatformError::new(
                "PLATFORM_PIPELINE_MISSING",
                "pipeline not found",
            ));
        };
        meta.active_hash = None;
        meta.activated_at = None;
        meta.updated_at = now_ts();
        self.data.put_pipeline_meta(&meta)?;
        Ok(meta)
    }

    /// Lists active production pipeline metadata for one project.
    pub fn list_active_pipeline_meta(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<PipelineMeta>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let mut rows = self
            .data
            .list_pipeline_meta(&owner, &project)?
            .into_iter()
            .filter(|m| m.active_hash.as_deref().is_some())
            .map(|mut m| {
                m.virtual_path = virtual_path_from_file_rel_path(&m.file_rel_path);
                m
            })
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| a.file_rel_path.cmp(&b.file_rel_path));
        Ok(rows)
    }

    /// Reads the active runtime snapshot source for one active pipeline.
    pub fn read_active_pipeline_source(
        &self,
        owner: &str,
        project: &str,
        meta: &PipelineMeta,
    ) -> Result<String, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let active_hash = meta.active_hash.as_deref().ok_or_else(|| {
            PlatformError::new(
                "PLATFORM_PIPELINE_NOT_ACTIVE",
                format!("pipeline '{}' is not active", meta.name),
            )
        })?;
        let snapshot_path = self.runtime_pipeline_snapshot_path(
            &layout,
            &meta.file_rel_path,
            active_hash,
        )?;
        if !snapshot_path.is_file() {
            return Err(PlatformError::new(
                "PLATFORM_PIPELINE_ACTIVE_SNAPSHOT_MISSING",
                format!("active snapshot missing for '{}'", meta.name),
            ));
        }
        Ok(fs::read_to_string(snapshot_path)?)
    }

    /// Returns registry hierarchy at one virtual path.
    pub fn list_pipeline_registry(
        &self,
        owner: &str,
        project: &str,
        current_virtual_path: &str,
        base_route: &str,
        editor_base: &str,
    ) -> Result<PipelineRegistryListing, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let current_path = normalize_virtual_path(current_virtual_path);
        let layout = self.file.ensure_project_layout(&owner, &project)?;

        // ── 1. Pipeline metadata rows ────────────────────────────────────────
        let rows = self.data.list_pipeline_meta(&owner, &project)?;
        let mut folders: BTreeSet<String> = BTreeSet::new();
        let mut pipelines = Vec::new();
        for m in rows {
            // Derive virtual_path from file_rel_path (source of truth).
            let vp = virtual_path_from_file_rel_path(&m.file_rel_path);
            if vp == current_path {
                let is_active = m.active_hash.as_deref().map(|h| !h.is_empty() && h == m.hash).unwrap_or(false);
                let has_draft = m.active_hash.as_deref().map(|h| !h.is_empty() && h != m.hash).unwrap_or(false);
                pipelines.push(PipelineRegistryItem {
                    name: m.name,
                    title: m.title,
                    description: m.description,
                    trigger_kind: m.trigger_kind,
                    file_rel_path: m.file_rel_path,
                    is_active,
                    has_draft,
                    git_status: None,
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

        // ── 2. Physical subdirs and files ────────────────────────────────────
        // Map virtual path → physical dir: virtual "/" → repo/pipelines/, "/pages" → repo/pipelines/pages/
        let phys_dir = if current_path == "/" {
            layout.repo_pipelines_dir.clone()
        } else {
            layout.repo_pipelines_dir.join(current_path.trim_start_matches('/'))
        };

        let mut files: Vec<RegistryFileItem> = Vec::new();

        if phys_dir.is_dir() {
            if let Ok(rd) = std::fs::read_dir(&phys_dir) {
                for entry in rd.flatten() {
                    let ft = match entry.file_type() { Ok(ft) => ft, Err(_) => continue };
                    let fname = entry.file_name().to_string_lossy().into_owned();
                    if fname.starts_with('.') { continue; }

                    if ft.is_dir() {
                        // Add physical subdir if not already derived from pipeline meta
                        folders.insert(fname);
                    } else if ft.is_file() {
                        let kind = if fname.ends_with(".tsx") {
                            "template"
                        } else if fname.ends_with(".ts") {
                            "script"
                        } else if fname.ends_with(".css") {
                            "style"
                        } else {
                            continue; // skip .zf.json and other files here
                        };
                        // rel_path relative to repo/ root (used for git status)
                        let rel_path = if current_path == "/" {
                            format!("pipelines/{fname}")
                        } else {
                            format!("pipelines{current_path}/{fname}")
                        };
                        // template_path relative to repo/pipelines/ (for template editor ?file=)
                        let template_path = if current_path == "/" {
                            fname.clone()
                        } else {
                            format!("{}/{fname}", current_path.trim_start_matches('/'))
                        };
                        let scope_param = current_path.trim_start_matches('/');
                        let edit_href = format!(
                            "/projects/{owner}/{project}/editor?type=template&path={scope_param}&file={template_path}"
                        );
                        files.push(RegistryFileItem {
                            name: fname,
                            rel_path,
                            kind: kind.to_string(),
                            edit_href,
                            git_status: None,
                        });
                    }
                }
            }
        }
        files.sort_by(|a, b| a.name.cmp(&b.name));

        // ── 3. Folder items — normal first, special pinned at bottom ─────────
        // Special folders appear at the bottom in fixed order: docs → styles → assets
        const SPECIAL: &[&str] = &["docs", "assets", "styles"];
        const SPECIAL_ORDER: &[&str] = &["docs", "styles", "assets"];
        let mut normal_folders = Vec::new();
        let mut special_folders = Vec::new();
        for name in folders {
            let next = if current_path == "/" {
                format!("/{name}")
            } else {
                format!("{current_path}/{name}")
            };
            let item = PipelineFolderItem {
                name: name.clone(),
                path: format!("{base_route}?path={next}"),
                is_special: SPECIAL.contains(&name.as_str()),
            };
            if item.is_special {
                special_folders.push(item);
            } else {
                normal_folders.push(item);
            }
        }
        normal_folders.sort_by(|a, b| a.name.cmp(&b.name));
        special_folders.sort_by(|a, b| {
            let ai = SPECIAL_ORDER.iter().position(|&s| s == a.name).unwrap_or(99);
            let bi = SPECIAL_ORDER.iter().position(|&s| s == b.name).unwrap_or(99);
            ai.cmp(&bi)
        });
        normal_folders.extend(special_folders);
        let folder_items = normal_folders;

        // ── 4. Breadcrumbs ───────────────────────────────────────────────────
        let mut breadcrumbs = vec![PipelineBreadcrumb {
            name: "root".to_string(),
            path: format!("{base_route}?path=/"),
            show_divider: false,
        }];
        if current_path != "/" {
            let mut accum = String::new();
            for seg in current_path.trim_start_matches('/').split('/') {
                if seg.trim().is_empty() { continue; }
                accum.push('/');
                accum.push_str(seg);
                breadcrumbs.push(PipelineBreadcrumb {
                    name: seg.to_string(),
                    path: format!("{base_route}?path={accum}"),
                    show_divider: true,
                });
            }
        }

        let _ = editor_base; // available for future use
        Ok(PipelineRegistryListing {
            current_path,
            breadcrumbs,
            folders: folder_items,
            pipelines,
            files,
        })
    }

    /// Returns the current template workspace tree for one project.
    pub fn list_template_workspace(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<TemplateWorkspaceListing, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        self.ensure_default_template_workspace(&layout)?;

        let mut items = Vec::new();
        let mut default_file = None;
        walk_template_tree(
            &layout.repo_pipelines_dir,
            &layout.repo_pipelines_dir,
            0,
            &mut items,
            &mut default_file,
        )?;

        Ok(TemplateWorkspaceListing {
            default_file,
            items,
        })
    }

    /// Lists `.tsx` template files in the project, optionally scoped to a sub-path.
    ///
    /// Returns items with `file_kind` of `"page"` or `"component"` (only `.tsx` files).
    /// `path` is a relative sub-path under the template root; `"/"` means all files.
    pub fn list_template_pages(
        &self,
        owner: &str,
        project: &str,
        path: Option<&str>,
    ) -> Result<Vec<TemplateTreeItem>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        self.ensure_default_template_workspace(&layout)?;

        let root = &layout.repo_pipelines_dir;
        let search_root = if let Some(p) = path.filter(|p| !p.is_empty() && p != &"/") {
            let sub = p.trim_start_matches('/');
            let candidate = root.join(sub);
            if candidate.starts_with(root) && candidate.is_dir() {
                candidate
            } else {
                root.clone()
            }
        } else {
            root.clone()
        };

        let mut items = Vec::new();
        collect_tsx_files(root, &search_root, &mut items)?;
        Ok(items)
    }

    /// Returns the filesystem path of the project's template root directory.
    pub fn get_project_template_root(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<PathBuf, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        Ok(layout.repo_pipelines_dir)
    }

    /// Reads one template workspace file by relative path under `app/templates`.
    pub fn read_template_file(
        &self,
        owner: &str,
        project: &str,
        rel_path: &str,
    ) -> Result<String, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        self.ensure_default_template_workspace(&layout)?;

        let (rel, abs) = resolve_template_entry(&layout.repo_pipelines_dir, rel_path)?;
        if !abs.is_file() {
            return Err(PlatformError::new(
                "PLATFORM_TEMPLATE_MISSING",
                format!("template file '{}' not found", rel),
            ));
        }
        fs::read_to_string(&abs).map_err(PlatformError::from)
    }

    /// Reads one template file with editor metadata.
    pub fn read_template_payload(
        &self,
        owner: &str,
        project: &str,
        rel_path: &str,
    ) -> Result<TemplateFilePayload, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        self.ensure_default_template_workspace(&layout)?;

        let (rel, abs) = resolve_template_entry(&layout.repo_pipelines_dir, rel_path)?;
        if !abs.is_file() {
            return Err(PlatformError::new(
                "PLATFORM_TEMPLATE_MISSING",
                format!("template file '{}' not found", rel),
            ));
        }
        let content = fs::read_to_string(&abs)?;
        Ok(template_payload_from_content(&rel, &content))
    }

    /// Saves one template file under `app/templates`.
    pub fn write_template_file(
        &self,
        owner: &str,
        project: &str,
        req: &TemplateSaveRequest,
    ) -> Result<TemplateFilePayload, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        self.ensure_default_template_workspace(&layout)?;

        let (rel, abs) = resolve_template_entry(&layout.repo_pipelines_dir, &req.rel_path)?;
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&abs, &req.content)?;
        Ok(template_payload_from_content(&rel, &req.content))
    }

    /// Creates one controlled template entry.
    pub fn create_template_entry(
        &self,
        owner: &str,
        project: &str,
        req: &TemplateCreateRequest,
    ) -> Result<TemplateFilePayload, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        self.ensure_default_template_workspace(&layout)?;

        let parent_rel =
            normalize_template_folder_rel_path(req.parent_rel_path.as_deref().unwrap_or_default());
        let parent_rel = default_template_parent(&req.kind, &parent_rel);
        let parent_abs = layout.repo_pipelines_dir.join(&parent_rel);
        if !parent_abs.starts_with(&layout.repo_pipelines_dir) {
            return Err(PlatformError::new(
                "PLATFORM_TEMPLATE_PATH",
                "resolved template parent escaped template root",
            ));
        }
        fs::create_dir_all(&parent_abs)?;

        if req.kind == TemplateCreateKind::Folder {
            let folder_name = slug_segment(&req.name);
            if folder_name.is_empty() {
                return Err(PlatformError::new(
                    "PLATFORM_TEMPLATE_CREATE",
                    "folder name must not be empty",
                ));
            }
            let rel = if parent_rel.is_empty() {
                folder_name
            } else {
                format!("{parent_rel}/{folder_name}")
            };
            let abs = layout.repo_pipelines_dir.join(&rel);
            if abs.exists() {
                return Err(PlatformError::new(
                    "PLATFORM_TEMPLATE_CREATE",
                    format!("template folder '{}' already exists", rel),
                ));
            }
            fs::create_dir_all(&abs)?;
            return Ok(TemplateFilePayload {
                rel_path: rel.clone(),
                name: rel.rsplit('/').next().unwrap_or("folder").to_string(),
                file_kind: "folder".to_string(),
                content: String::new(),
                line_count: 0,
                is_protected: template_entry_is_protected(&rel, true),
            });
        }

        let (filename, scaffold) = scaffold_template_entry(&req.kind, &req.name)?;
        let rel = if parent_rel.is_empty() {
            filename
        } else {
            format!("{parent_rel}/{filename}")
        };
        let abs = layout.repo_pipelines_dir.join(&rel);
        if !abs.starts_with(&layout.repo_pipelines_dir) {
            return Err(PlatformError::new(
                "PLATFORM_TEMPLATE_PATH",
                "resolved template path escaped template root",
            ));
        }
        if abs.exists() {
            return Err(PlatformError::new(
                "PLATFORM_TEMPLATE_CREATE",
                format!("template '{}' already exists", rel),
            ));
        }
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&abs, &scaffold)?;
        Ok(template_payload_from_content(&rel, &scaffold))
    }

    /// Deletes one template file or folder.
    pub fn delete_template_entry(
        &self,
        owner: &str,
        project: &str,
        rel_path: &str,
    ) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        self.ensure_default_template_workspace(&layout)?;

        let (rel, abs) = resolve_template_entry(&layout.repo_pipelines_dir, rel_path)?;
        if !abs.exists() {
            return Err(PlatformError::new(
                "PLATFORM_TEMPLATE_MISSING",
                format!("template entry '{}' not found", rel),
            ));
        }
        if template_entry_is_protected(&rel, abs.is_dir()) {
            return Err(PlatformError::new(
                "PLATFORM_TEMPLATE_DELETE",
                format!("protected template entry '{}' cannot be deleted", rel),
            ));
        }
        if abs.is_dir() {
            fs::remove_dir_all(&abs)?;
        } else {
            fs::remove_file(&abs)?;
        }
        Ok(())
    }

    /// Moves one template file or folder into another folder.
    pub fn move_template_entry(
        &self,
        owner: &str,
        project: &str,
        req: &TemplateMoveRequest,
    ) -> Result<String, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        self.ensure_default_template_workspace(&layout)?;

        let (from_rel, from_abs) =
            resolve_template_entry(&layout.repo_pipelines_dir, &req.from_rel_path)?;
        if !from_abs.exists() {
            return Err(PlatformError::new(
                "PLATFORM_TEMPLATE_MISSING",
                format!("template entry '{}' not found", from_rel),
            ));
        }
        let parent_rel = normalize_template_folder_rel_path(&req.to_parent_rel_path);
        let parent_abs = layout.repo_pipelines_dir.join(&parent_rel);
        if !parent_abs.starts_with(&layout.repo_pipelines_dir) {
            return Err(PlatformError::new(
                "PLATFORM_TEMPLATE_PATH",
                "resolved move target escaped template root",
            ));
        }
        if !parent_abs.exists() || !parent_abs.is_dir() {
            return Err(PlatformError::new(
                "PLATFORM_TEMPLATE_MOVE",
                format!("target folder '{}' not found", parent_rel),
            ));
        }
        let name = from_abs
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or_else(|| PlatformError::new("PLATFORM_TEMPLATE_MOVE", "invalid source filename"))?
            .to_string();
        let to_abs = parent_abs.join(&name);
        let to_rel = if parent_rel.is_empty() {
            name
        } else {
            format!("{parent_rel}/{name}")
        };
        if from_abs == to_abs {
            return Ok(to_rel);
        }
        if to_abs.exists() {
            return Err(PlatformError::new(
                "PLATFORM_TEMPLATE_MOVE",
                format!("target '{}' already exists", to_rel),
            ));
        }
        fs::rename(&from_abs, &to_abs)?;
        Ok(to_rel)
    }

    /// Returns git status rows for files under `app/templates`.
    pub fn list_template_git_status(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<TemplateGitStatusItem>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        self.ensure_default_template_workspace(&layout)?;

        let output = Command::new("git")
            .arg("-C")
            .arg(&layout.repo_dir)
            .arg("status")
            .arg("--porcelain=v1")
            .arg("--untracked-files=all")
            .arg("--")
            .arg("templates")
            .output()
            .map_err(|err| PlatformError::new("PLATFORM_TEMPLATE_GIT", err.to_string()))?;
        if !output.status.success() {
            return Err(PlatformError::new(
                "PLATFORM_TEMPLATE_GIT",
                format!("git status failed with status {}", output.status),
            ));
        }

        let mut items = Vec::new();
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            if line.len() < 4 {
                continue;
            }
            let xy = &line[..2];
            let raw_path = line[3..].trim();
            let rel = if let Some((_, dest)) = raw_path.split_once(" -> ") {
                dest.trim().to_string()
            } else {
                raw_path.to_string()
            };
            let rel = rel.strip_prefix("templates/").unwrap_or(&rel).to_string();
            let code = if xy == "??" {
                "??".to_string()
            } else {
                let trimmed = xy.trim().replace(' ', "");
                if trimmed.is_empty() {
                    "M".to_string()
                } else {
                    trimmed
                }
            };
            if !rel.is_empty() {
                items.push(TemplateGitStatusItem {
                    rel_path: rel,
                    code,
                });
            }
        }
        items.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        Ok(items)
    }

    /// Returns git status rows for files under `repo/` (covers pipelines, templates, etc.).
    pub fn list_repo_git_status(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<TemplateGitStatusItem>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;

        let output = Command::new("git")
            .arg("-C")
            .arg(&layout.repo_dir)
            .arg("status")
            .arg("--porcelain=v1")
            .arg("--untracked-files=all")
            .output()
            .map_err(|err| PlatformError::new("PLATFORM_REPO_GIT", err.to_string()))?;
        if !output.status.success() {
            return Err(PlatformError::new(
                "PLATFORM_REPO_GIT",
                format!("git status failed with status {}", output.status),
            ));
        }

        let mut items = Vec::new();
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            if line.len() < 4 {
                continue;
            }
            let xy = &line[..2];
            let raw_path = line[3..].trim();
            let rel = if let Some((_, dest)) = raw_path.split_once(" -> ") {
                dest.trim().to_string()
            } else {
                raw_path.to_string()
            };
            let code = if xy == "??" {
                "??".to_string()
            } else {
                let trimmed = xy.trim().replace(' ', "");
                if trimmed.is_empty() {
                    "M".to_string()
                } else {
                    trimmed
                }
            };
            if !rel.is_empty() {
                items.push(TemplateGitStatusItem {
                    rel_path: rel,
                    code,
                });
            }
        }
        items.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        Ok(items)
    }

    /// Deletes one pipeline — removes the source file, platform metadata, and any active runtime snapshot.
    pub fn delete_pipeline(
        &self,
        owner: &str,
        project: &str,
        file_rel_path: &str,
    ) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let wanted = file_rel_path.trim().replace('\\', "/");

        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let abs = layout.repo_dir.join(&wanted);
        if !abs.starts_with(&layout.repo_dir) {
            return Err(PlatformError::new(
                "PLATFORM_PIPELINE_PATH",
                "resolved pipeline path escaped repo root",
            ));
        }

        // Remove source file from disk.
        if abs.is_file() {
            fs::remove_file(&abs).map_err(|e| PlatformError::new("PLATFORM_PIPELINE_DELETE", e.to_string()))?;
        }

        // Find and remove metadata from platform catalog.
        let meta = self
            .data
            .list_pipeline_meta(&owner, &project)?
            .into_iter()
            .find(|m| m.file_rel_path.trim().replace('\\', "/") == wanted);
        if let Some(m) = meta {
            self.data.delete_pipeline_meta(&m.owner, &m.project, &m.file_rel_path)?;
        }

        Ok(())
    }

    fn ensure_default_template_workspace(
        &self,
        layout: &ProjectFileLayout,
    ) -> Result<(), PlatformError> {
        // Subdirs are created by ensure_project_layout. Only scaffold default files here.
        let pages_dir = layout.repo_pipelines_dir.join("pages");
        let styles_dir = layout.repo_pipelines_dir.join("styles");
        fs::create_dir_all(&pages_dir)?;
        fs::create_dir_all(&styles_dir)?;

        let main_css = styles_dir.join("main.css");
        if !main_css.exists() {
            fs::write(
                &main_css,
                r#":root {
  --zf-color-bg: #020617;
  --zf-color-panel: #0f172a;
  --zf-color-text: #e2e8f0;
  --zf-color-accent: #ff5c00;
  --zf-color-accent-alt: #005b9a;
}
"#,
            )?;
        }

        let home_page = pages_dir.join("home.tsx");
        if !home_page.exists() {
            fs::write(
                &home_page,
                r#"export const page = {
  head: {
    title: "Home",
    description: "Default Zebflow page"
  },
  html: {
    lang: "en"
  },
  body: {
    className: "min-h-screen bg-slate-950 text-slate-100 font-sans"
  },
  navigation: "history"
};

export const app = {};

export default function Page(input) {
  return (
    <Page>
      <main className="p-6">
        <h1 className="text-3xl font-black">Home</h1>
      </main>
    </Page>
  );
}
"#,
            )?;
        }

        Ok(())
    }

    /// Returns the absolute filesystem path for a pipeline given its `file_rel_path`.
    fn pipeline_abs_path(
        &self,
        layout: &ProjectFileLayout,
        file_rel_path: &str,
    ) -> Result<PathBuf, PlatformError> {
        let abs = layout.repo_dir.join(file_rel_path);
        if !abs.starts_with(&layout.repo_dir) {
            return Err(PlatformError::new(
                "PLATFORM_PIPELINE_PATH",
                "resolved path escaped repo root",
            ));
        }
        Ok(abs)
    }

    /// Returns the runtime snapshot path for an active pipeline.
    ///
    /// Uses `file_rel_path` as the stable identifier:
    /// `pipelines/api/foo.zf.json` + `abc123` → `<runtime>/api/foo.abc123.zf.json`
    fn runtime_pipeline_snapshot_path(
        &self,
        layout: &ProjectFileLayout,
        file_rel_path: &str,
        hash: &str,
    ) -> Result<PathBuf, PlatformError> {
        // Strip "pipelines/" prefix to get the sub-path, then replace .zf.json with .{hash}.zf.json
        let sub = file_rel_path
            .trim_start_matches("pipelines/")
            .trim_start_matches('/');
        let snapshot_name = if let Some(stem) = sub.strip_suffix(".zf.json") {
            format!("{stem}.{}.zf.json", slug_segment(hash))
        } else if let Some(stem) = sub.strip_suffix(".json") {
            format!("{stem}.{}.json", slug_segment(hash))
        } else {
            format!("{sub}.{}", slug_segment(hash))
        };
        let abs = layout.data_runtime_pipelines_dir.join(&snapshot_name);
        if !abs.starts_with(&layout.data_runtime_pipelines_dir) {
            return Err(PlatformError::new(
                "PLATFORM_PIPELINE_PATH",
                "resolved runtime pipeline snapshot escaped runtime root",
            ));
        }
        Ok(abs)
    }

    /// Lists project doc files under app/docs (ERD, README.md, AGENTS.md, use cases, etc.).
    pub fn list_project_docs(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectDocItem>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let mut items = Vec::new();
        walk_docs_tree(&layout.repo_docs_dir, &layout.repo_docs_dir, &mut items)?;
        Ok(items)
    }

    /// Reads one project doc file by path under app/docs.
    pub fn read_project_doc(
        &self,
        owner: &str,
        project: &str,
        rel_path: &str,
    ) -> Result<String, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let (_rel, abs) = resolve_doc_path(&layout.repo_docs_dir, rel_path)?;
        if !abs.is_file() {
            return Err(PlatformError::new(
                "PLATFORM_DOC_MISSING",
                format!("doc file '{}' not found", rel_path),
            ));
        }
        fs::read_to_string(&abs).map_err(PlatformError::from)
    }

    /// Creates or updates one project doc file by path under `app/docs`.
    pub fn upsert_project_doc(
        &self,
        owner: &str,
        project: &str,
        rel_path: &str,
        content: &str,
    ) -> Result<ProjectDocItem, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let (rel, abs) = resolve_doc_path(&layout.repo_docs_dir, rel_path)?;

        if rel.ends_with('/') {
            return Err(PlatformError::new(
                "PLATFORM_DOC_PATH",
                "doc path must point to a file",
            ));
        }
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&abs, content)?;

        let name = abs
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("doc")
            .to_string();
        Ok(ProjectDocItem {
            path: rel,
            name,
            kind: "file".to_string(),
        })
    }

    /// Lists the three agent doc files (AGENTS.md, SOUL.md, MEMORY.md), creating defaults if absent.
    pub fn list_agent_docs(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<AgentDocItem>, PlatformError> {
        self.ensure_agent_docs_defaults(owner, project)?;
        Ok(vec![
            AgentDocItem { name: "AGENTS.md".to_string(), user_editable: true },
            AgentDocItem { name: "SOUL.md".to_string(), user_editable: true },
            AgentDocItem { name: "MEMORY.md".to_string(), user_editable: false },
        ])
    }

    /// Reads one agent doc file (AGENTS.md, SOUL.md, or MEMORY.md).
    pub fn read_agent_doc(
        &self,
        owner: &str,
        project: &str,
        name: &str,
    ) -> Result<String, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let safe_name = Self::validate_agent_doc_name(name)?;
        let path = layout.agent_docs_dir.join(safe_name);
        if path.is_file() {
            fs::read_to_string(&path).map_err(PlatformError::from)
        } else {
            Ok(String::new())
        }
    }

    /// Creates or replaces one agent doc file. All three names are valid.
    pub fn upsert_agent_doc(
        &self,
        owner: &str,
        project: &str,
        name: &str,
        content: &str,
    ) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let safe_name = Self::validate_agent_doc_name(name)?;
        let path = layout.agent_docs_dir.join(safe_name);
        fs::write(&path, content).map_err(PlatformError::from)
    }

    /// Creates default agent doc files if they don't exist yet.
    pub fn ensure_agent_docs_defaults(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let defaults: &[(&str, &str)] = &[
            ("AGENTS.md", AGENTS_MD_DEFAULT),
            ("SOUL.md", SOUL_MD_DEFAULT),
            ("MEMORY.md", MEMORY_MD_DEFAULT),
        ];
        for (name, content) in defaults {
            let path = layout.agent_docs_dir.join(name);
            if !path.exists() {
                fs::write(&path, content)?;
            }
        }
        Ok(())
    }

    fn validate_agent_doc_name(name: &str) -> Result<&str, PlatformError> {
        match name {
            "AGENTS.md" | "SOUL.md" | "MEMORY.md" => Ok(name),
            _ => Err(PlatformError::new(
                "PLATFORM_AGENT_DOC_INVALID",
                format!("agent doc name must be AGENTS.md, SOUL.md, or MEMORY.md; got '{name}'"),
            )),
        }
    }
}

const AGENTS_MD_DEFAULT: &str = "# Agents\n\nDescribe AI agents for this project, their roles, \
tools they are authorized to use, and any important constraints.\n";

const SOUL_MD_DEFAULT: &str = "# Soul\n\nDescribe the assistant's personality, communication style, \
and tone for this project.\n";

const MEMORY_MD_DEFAULT: &str = "# Memory\n\n_(This file is managed by the assistant. \
It records important project information discovered during conversations.)_\n";

fn walk_docs_tree(
    root: &Path,
    current: &Path,
    items: &mut Vec<ProjectDocItem>,
) -> Result<(), PlatformError> {
    if !current.exists() {
        return Ok(());
    }
    let mut entries = fs::read_dir(current)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| {
        let a_is_dir = a.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        let b_is_dir = b.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let rel = path
            .strip_prefix(root)
            .map_err(|_| PlatformError::new("PLATFORM_DOC_PATH", "invalid doc path"))?
            .to_string_lossy()
            .replace('\\', "/");
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            items.push(ProjectDocItem {
                path: rel.clone(),
                name: name.clone(),
                kind: "folder".to_string(),
            });
            walk_docs_tree(root, &path, items)?;
        } else {
            items.push(ProjectDocItem {
                path: rel,
                name,
                kind: "file".to_string(),
            });
        }
    }
    Ok(())
}

fn resolve_doc_path(root: &Path, rel_path: &str) -> Result<(String, PathBuf), PlatformError> {
    let normalized = rel_path
        .trim()
        .replace('\\', "/")
        .trim_matches('/')
        .to_string();
    if normalized.contains("..") {
        return Err(PlatformError::new(
            "PLATFORM_DOC_PATH",
            "doc path must not contain ..",
        ));
    }
    if normalized.is_empty() {
        return Err(PlatformError::new(
            "PLATFORM_DOC_PATH",
            "doc path must not be empty",
        ));
    }
    let abs = root.join(&normalized);
    if !abs.starts_with(root) {
        return Err(PlatformError::new(
            "PLATFORM_DOC_PATH",
            "resolved doc path escaped docs root",
        ));
    }
    Ok((normalized, abs))
}

/// Recursively collects `.tsx` files under `current`, returning `TemplateTreeItem` entries.
/// `file_kind` is `"page"` for paths containing `/pages/`, else `"component"`.
fn collect_tsx_files(
    root: &Path,
    current: &Path,
    items: &mut Vec<TemplateTreeItem>,
) -> Result<(), PlatformError> {
    let mut entries = fs::read_dir(current)?
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_tsx_files(root, &path, items)?;
        } else if file_type.is_file() {
            if path.extension().and_then(std::ffi::OsStr::to_str) != Some("tsx") {
                continue;
            }
            let rel = path
                .strip_prefix(root)
                .map_err(|_| PlatformError::new("PLATFORM_TEMPLATE_PATH", "invalid template path"))?
                .to_string_lossy()
                .replace('\\', "/");
            let file_kind = template_file_kind(&path);
            items.push(TemplateTreeItem {
                name: entry.file_name().to_string_lossy().to_string(),
                rel_path: rel,
                kind: "file".to_string(),
                depth: 0,
                file_kind,
                is_protected: false,
            });
        }
    }
    Ok(())
}

fn walk_template_tree(
    root: &Path,
    current: &Path,
    depth: usize,
    items: &mut Vec<TemplateTreeItem>,
    default_file: &mut Option<String>,
) -> Result<(), PlatformError> {
    let mut entries = fs::read_dir(current)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .collect::<Vec<_>>();

    entries.sort_by(|a, b| {
        let a_is_dir = a.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        let b_is_dir = b.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    for entry in entries {
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .map_err(|_| PlatformError::new("PLATFORM_TEMPLATE_PATH", "invalid template path"))?
            .to_string_lossy()
            .replace('\\', "/");
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            items.push(TemplateTreeItem {
                name: entry.file_name().to_string_lossy().to_string(),
                rel_path: rel.clone(),
                kind: "folder".to_string(),
                depth,
                file_kind: "folder".to_string(),
                is_protected: template_entry_is_protected(&rel, true),
            });
            walk_template_tree(root, &path, depth + 1, items, default_file)?;
        } else if file_type.is_file() {
            let file_kind = template_file_kind(&path);
            if default_file.is_none() && file_kind != "other" {
                *default_file = Some(rel.clone());
            }
            items.push(TemplateTreeItem {
                name: entry.file_name().to_string_lossy().to_string(),
                rel_path: rel.clone(),
                kind: "file".to_string(),
                depth,
                file_kind,
                is_protected: template_entry_is_protected(&rel, false),
            });
        }
    }

    Ok(())
}

fn template_file_kind(path: &Path) -> String {
    match path.extension().and_then(std::ffi::OsStr::to_str) {
        Some("tsx") => {
            let rel = path.to_string_lossy();
            if rel.contains("/pages/") {
                "page".to_string()
            } else {
                "component".to_string()
            }
        }
        Some("ts") => "script".to_string(),
        Some("css") => "style".to_string(),
        _ => "other".to_string(),
    }
}

fn normalize_template_rel_path(raw: &str) -> String {
    raw.split('/')
        .map(str::trim)
        .filter(|seg| !seg.is_empty() && *seg != "." && *seg != "..")
        .map(slug_preserving_extension)
        .filter(|seg| !seg.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

fn normalize_template_folder_rel_path(raw: &str) -> String {
    raw.split('/')
        .map(str::trim)
        .filter(|seg| !seg.is_empty() && *seg != "." && *seg != "..")
        .map(slug_segment)
        .filter(|seg| !seg.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

fn slug_preserving_extension(raw: &str) -> String {
    let mut parts = raw.rsplitn(2, '.').collect::<Vec<_>>();
    parts.reverse();
    if parts.len() == 2 {
        let stem = slug_segment(parts[0]);
        let ext = parts[1].trim().to_ascii_lowercase();
        if stem.is_empty() || ext.is_empty() {
            String::new()
        } else {
            format!("{stem}.{ext}")
        }
    } else {
        slug_segment(raw)
    }
}

fn resolve_template_entry(root: &Path, rel_path: &str) -> Result<(String, PathBuf), PlatformError> {
    let normalized =
        if rel_path.ends_with(".tsx") || rel_path.ends_with(".ts") || rel_path.ends_with(".css") {
            normalize_template_rel_path(rel_path)
        } else {
            normalize_template_folder_rel_path(rel_path)
        };
    if normalized.is_empty() {
        return Err(PlatformError::new(
            "PLATFORM_TEMPLATE_PATH",
            "template path must not be empty",
        ));
    }
    let abs = root.join(&normalized);
    if !abs.starts_with(root) {
        return Err(PlatformError::new(
            "PLATFORM_TEMPLATE_PATH",
            "resolved template path escaped template root",
        ));
    }
    Ok((normalized, abs))
}

fn default_template_parent(kind: &TemplateCreateKind, requested_parent: &str) -> String {
    if !requested_parent.is_empty() {
        return requested_parent.to_string();
    }
    match kind {
        TemplateCreateKind::Page => "pages".to_string(),
        TemplateCreateKind::Component => "components".to_string(),
        TemplateCreateKind::Script => "scripts".to_string(),
        TemplateCreateKind::Folder => String::new(),
    }
}

fn scaffold_template_entry(
    kind: &TemplateCreateKind,
    raw_name: &str,
) -> Result<(String, String), PlatformError> {
    let base = slug_segment(raw_name);
    if base.is_empty() {
        return Err(PlatformError::new(
            "PLATFORM_TEMPLATE_CREATE",
            "template name must not be empty",
        ));
    }

    match kind {
        TemplateCreateKind::Page => {
            let title = humanize_slug(&base);
            let component_name = page_component_name(&base);
            let filename = format!("{base}.tsx");
            let content = format!(
                "export const page = {{\n  head: {{\n    title: \"{title}\",\n    description: \"{title} page\"\n  }},\n  html: {{\n    lang: \"en\"\n  }},\n  body: {{\n    className: \"min-h-screen bg-slate-950 text-slate-100 font-sans\"\n  }},\n  navigation: \"history\"\n}};\n\nexport const app = {{}};\n\nexport default function {component_name}(input) {{\n  return (\n    <Page>\n      <main className=\"p-6\">\n        <h1 className=\"text-3xl font-black\">{title}</h1>\n      </main>\n    </Page>\n  );\n}}\n"
            );
            Ok((filename, content))
        }
        TemplateCreateKind::Component => {
            let component_name = component_name(&base);
            let filename = format!("{base}.tsx");
            let content = format!(
                "export default function {component_name}(props) {{\n  return (\n    <div>\n      <span>{component_name}</span>\n    </div>\n  );\n}}\n"
            );
            Ok((filename, content))
        }
        TemplateCreateKind::Script => {
            let filename = format!("{base}.ts");
            let export_name = script_export_name(&base);
            let content = format!("export function {export_name}() {{\n  return null;\n}}\n");
            Ok((filename, content))
        }
        TemplateCreateKind::Folder => Err(PlatformError::new(
            "PLATFORM_TEMPLATE_CREATE",
            "folder creation does not use file scaffolds",
        )),
    }
}

fn template_payload_from_content(rel_path: &str, content: &str) -> TemplateFilePayload {
    TemplateFilePayload {
        rel_path: rel_path.to_string(),
        name: rel_path.rsplit('/').next().unwrap_or(rel_path).to_string(),
        file_kind: template_file_kind(Path::new(rel_path)),
        content: content.to_string(),
        line_count: content.lines().count().max(1),
        is_protected: template_entry_is_protected(rel_path, false),
    }
}

fn template_entry_is_protected(rel_path: &str, is_dir: bool) -> bool {
    match rel_path {
        "styles" | "scripts" => is_dir,
        "styles/main.css" => true,
        _ => false,
    }
}

fn humanize_slug(raw: &str) -> String {
    raw.split('-')
        .filter(|seg| !seg.is_empty())
        .map(capitalize_ascii)
        .collect::<Vec<_>>()
        .join(" ")
}

fn component_name(raw: &str) -> String {
    let mut out = String::new();
    for part in raw.split('-').filter(|seg| !seg.is_empty()) {
        out.push_str(&capitalize_ascii(part));
    }
    if out.is_empty() {
        "Component".to_string()
    } else {
        out
    }
}

fn page_component_name(raw: &str) -> String {
    let base = component_name(raw);
    if base.ends_with("Page") {
        base
    } else {
        format!("{base}Page")
    }
}

fn script_export_name(raw: &str) -> String {
    let mut parts = raw.split('-').filter(|seg| !seg.is_empty());
    let first = parts.next().unwrap_or("script").to_string();
    let mut out = first;
    for part in parts {
        out.push_str(&capitalize_ascii(part));
    }
    out
}

fn capitalize_ascii(raw: &str) -> String {
    let mut chars = raw.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

/// Normalizes a pipeline file_rel_path:
/// - Ensures it starts with "pipelines/"
/// - Ensures it ends with ".zf.json"
/// - Strips leading "/" if present
pub fn normalize_pipeline_file_rel_path(raw: &str) -> String {
    let s = raw.trim().trim_start_matches('/');
    // Ensure starts with "pipelines/"
    let s = if s.starts_with("pipelines/") {
        s.to_string()
    } else {
        format!("pipelines/{s}")
    };
    // Ensure ends with ".zf.json"
    if s.ends_with(".zf.json") {
        s
    } else if s.ends_with(".json") {
        s
    } else {
        format!("{s}.zf.json")
    }
}

/// Derives the virtual_path from a file_rel_path.
/// "pipelines/api/foo.zf.json" → "/api"
/// "pipelines/foo.zf.json"     → "/"
pub fn virtual_path_from_file_rel_path(file_rel_path: &str) -> String {
    let stripped = file_rel_path
        .trim_start_matches("pipelines/")
        .trim_start_matches('/');
    match stripped.rfind('/') {
        Some(pos) => format!("/{}", &stripped[..pos]),
        None => "/".to_string(),
    }
}

/// Derives the pipeline name (slug) from a file_rel_path.
/// "pipelines/api/foo.zf.json" → "foo"
pub fn name_from_file_rel_path(file_rel_path: &str) -> String {
    std::path::Path::new(file_rel_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.trim_end_matches(".zf"))
        .unwrap_or(file_rel_path)
        .to_string()
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

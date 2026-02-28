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
    CreateProjectRequest, PipelineBreadcrumb, PipelineFolderItem, PipelineMeta,
    PipelineRegistryItem, PipelineRegistryListing, PlatformProject, ProjectFileLayout,
    TemplateCreateKind, TemplateCreateRequest, TemplateFilePayload, TemplateGitStatusItem,
    TemplateMoveRequest, TemplateSaveRequest, TemplateTreeItem, TemplateWorkspaceListing,
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
        self.ensure_default_template_workspace(&layout)?;

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
            &layout.app_templates_dir,
            &layout.app_templates_dir,
            0,
            &mut items,
            &mut default_file,
        )?;

        Ok(TemplateWorkspaceListing {
            default_file,
            items,
        })
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

        let (rel, abs) = resolve_template_entry(&layout.app_templates_dir, rel_path)?;
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

        let (rel, abs) = resolve_template_entry(&layout.app_templates_dir, rel_path)?;
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

        let (rel, abs) = resolve_template_entry(&layout.app_templates_dir, &req.rel_path)?;
        if !abs.is_file() {
            return Err(PlatformError::new(
                "PLATFORM_TEMPLATE_MISSING",
                format!("template file '{}' not found", rel),
            ));
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

        let parent_rel = normalize_template_folder_rel_path(
            req.parent_rel_path.as_deref().unwrap_or_default(),
        );
        let parent_rel = default_template_parent(&req.kind, &parent_rel);
        let parent_abs = layout.app_templates_dir.join(&parent_rel);
        if !parent_abs.starts_with(&layout.app_templates_dir) {
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
            let abs = layout.app_templates_dir.join(&rel);
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
        let abs = layout.app_templates_dir.join(&rel);
        if !abs.starts_with(&layout.app_templates_dir) {
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

        let (rel, abs) = resolve_template_entry(&layout.app_templates_dir, rel_path)?;
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

        let (from_rel, from_abs) = resolve_template_entry(&layout.app_templates_dir, &req.from_rel_path)?;
        if !from_abs.exists() {
            return Err(PlatformError::new(
                "PLATFORM_TEMPLATE_MISSING",
                format!("template entry '{}' not found", from_rel),
            ));
        }
        let parent_rel = normalize_template_folder_rel_path(&req.to_parent_rel_path);
        let parent_abs = layout.app_templates_dir.join(&parent_rel);
        if !parent_abs.starts_with(&layout.app_templates_dir) {
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
            .arg(&layout.app_dir)
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
                items.push(TemplateGitStatusItem { rel_path: rel, code });
            }
        }
        items.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        Ok(items)
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

    fn ensure_default_template_workspace(
        &self,
        layout: &ProjectFileLayout,
    ) -> Result<(), PlatformError> {
        let pages_dir = layout.app_templates_dir.join("pages");
        let components_dir = layout.app_templates_dir.join("components");
        let styles_dir = layout.app_templates_dir.join("styles");
        let scripts_dir = layout.app_templates_dir.join("scripts");

        for dir in [&pages_dir, &components_dir, &styles_dir, &scripts_dir] {
            fs::create_dir_all(dir)?;
        }

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

fn resolve_template_entry(
    root: &Path,
    rel_path: &str,
) -> Result<(String, PathBuf), PlatformError> {
    let normalized = if rel_path.ends_with(".tsx")
        || rel_path.ends_with(".ts")
        || rel_path.ends_with(".css")
    {
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
            let content = format!(
                "export function {export_name}() {{\n  return null;\n}}\n"
            );
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

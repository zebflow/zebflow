//! Project management service.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::contracts::kinds::DependencyLockSpec;
use crate::contracts::kinds::{
    decode_pipeline_graph, encode_pipeline_graph, validate_pipeline_activation,
};
use crate::infra::io::durable::{atomic_write, durable_remove_file};
use crate::pipeline::PipelineGraph;
use crate::platform::adapters::data::DataAdapter;
use crate::platform::adapters::file::FileAdapter;
use crate::platform::adapters::project_data::ProjectDataFactory;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    AgentDocItem, CreateProjectRequest, HubAuthority, LEGACY_PIPELINE_IDENTITY_ROOT,
    PIPELINE_DEFINITION_EXTENSION, PIPELINE_IDENTITY_BACKUP_FILE, PipelineBreadcrumb,
    PipelineFolderItem, PipelineMeta, PipelineRegistryItem, PipelineRegistryListing,
    PlatformProject, ProjectDocItem, ProjectDocMoveRequest, ProjectFileLayout, RegistryFileItem,
    ResolvedProjectLayout, TemplateCreateKind, TemplateCreateRequest, TemplateFilePayload,
    TemplateGitStatusItem, TemplateMoveRequest, TemplateSaveRequest, TemplateTreeItem,
    TemplateWorkspaceListing, normalize_virtual_path, now_ts, recovery_date_stamp, slug_segment,
};
use crate::platform::services::dependency_lock::DependencyLockService;
use crate::platform::services::project_config::ProjectConfigurationService;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWebhookTrigger {
    pub node_id: String,
    pub path: String,
    pub method: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct WebhookPathConflict {
    pub path: String,
    pub method: String,
    pub pipeline_name: String,
    pub file_rel_path: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ProjectGitHealth {
    pub state: String,
    pub repo_path: String,
    pub git_dir_path: String,
    pub git_dir_exists: bool,
    pub is_work_tree: bool,
    pub head_exists: bool,
    pub config_exists: bool,
    pub objects_exists: bool,
    pub refs_exists: bool,
    pub branch: String,
    pub last_error: String,
    pub recommended_action: String,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectGitRepairMode {
    Repair,
    Reinitialize,
    Reset,
}

pub fn canonical_webhook_method(raw: Option<&str>) -> String {
    raw.unwrap_or("POST").trim().to_ascii_uppercase()
}

pub fn canonical_webhook_path(raw: Option<&str>) -> String {
    let path = raw.unwrap_or("/").trim();
    let path = if path.is_empty() { "/" } else { path };
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

pub fn derive_trigger_kind_from_source(source: &str) -> Option<String> {
    let graph = decode_pipeline_graph(source.as_bytes()).ok()?.spec;
    let entry_ids: std::collections::HashSet<&str> = if !graph.entry_nodes.is_empty() {
        graph.entry_nodes.iter().map(|s| s.as_str()).collect()
    } else {
        let targets: std::collections::HashSet<&str> =
            graph.edges.iter().map(|e| e.to_node.as_str()).collect();
        graph
            .nodes
            .iter()
            .filter(|n| !targets.contains(n.id.as_str()))
            .map(|n| n.id.as_str())
            .collect()
    };
    graph
        .nodes
        .iter()
        .filter(|n| entry_ids.contains(n.id.as_str()))
        .find_map(|n| {
            let canonical = canonical_pipeline_node_kind(&n.kind);
            canonical
                .strip_prefix("n.trigger.")
                .map(|suffix| suffix.to_string())
        })
}

fn pipeline_source_is_locked(source: &str) -> bool {
    decode_pipeline_graph(source.as_bytes())
        .ok()
        .and_then(|document| document.spec.metadata)
        .is_some_and(|metadata| metadata.locked)
}

pub fn webhook_triggers_from_graph(graph: &PipelineGraph) -> Vec<ProjectWebhookTrigger> {
    graph
        .nodes
        .iter()
        .filter(|node| canonical_pipeline_node_kind(&node.kind) == "n.trigger.webhook")
        .map(|node| ProjectWebhookTrigger {
            node_id: node.id.clone(),
            path: canonical_webhook_path(
                node.config.get("path").and_then(serde_json::Value::as_str),
            ),
            method: canonical_webhook_method(
                node.config
                    .get("method")
                    .and_then(serde_json::Value::as_str),
            ),
        })
        .collect()
}

pub fn webhook_triggers_from_source(source: &str) -> Option<Vec<ProjectWebhookTrigger>> {
    let graph = decode_pipeline_graph(source.as_bytes()).ok()?.spec;
    Some(webhook_triggers_from_graph(&graph))
}

pub fn first_webhook_trigger_from_source(source: &str) -> Option<(String, String)> {
    webhook_triggers_from_source(source)?
        .into_iter()
        .next()
        .map(|trigger| (trigger.path, trigger.method))
}

fn parse_and_validate_pipeline_source(source: &str) -> Result<PipelineGraph, PlatformError> {
    let graph = parse_pipeline_source(source)?;
    validate_pipeline_activation(&graph)
        .map_err(|err| PlatformError::new("FW_EMPTY_GRAPH", err.to_string()))?;
    Ok(graph)
}

fn parse_and_validate_pipeline_source_for_save(
    source: &str,
) -> Result<PipelineGraph, PlatformError> {
    parse_pipeline_source(source)
}

fn parse_pipeline_source(source: &str) -> Result<PipelineGraph, PlatformError> {
    decode_pipeline_graph(source.as_bytes())
        .map_err(|err| {
            let code = err.violation_code().unwrap_or("PLATFORM_PIPELINE_PARSE");
            PlatformError::new(
                code,
                format!("failed parsing pipeline source: {err} ({})", err.category()),
            )
        })
        .map(|document| document.spec)
}

fn canonical_pipeline_node_kind(kind: &str) -> &str {
    if let Some(stripped) = kind.strip_prefix("x.n.") {
        return match stripped {
            "trigger.webhook" => "n.trigger.webhook",
            "trigger.schedule" => "n.trigger.schedule",
            "trigger.manual" => "n.trigger.manual",
            _ => kind,
        };
    }
    kind
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), PlatformError> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ft.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else if ft.is_file() {
            fs::copy(&entry.path(), &target)?;
        }
    }
    Ok(())
}

fn init_git_repo(repo_dir: &Path) -> Result<(), PlatformError> {
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

/// Project service backed by swappable data + file adapters.
pub struct ProjectService {
    data: Arc<dyn DataAdapter>,
    file: Arc<dyn FileAdapter>,
    project_data: Arc<dyn ProjectDataFactory>,
    zebflow_cfg: Arc<ProjectConfigurationService>,
    dependency_lock: Arc<DependencyLockService>,
    #[cfg(test)]
    fail_next_pipeline_meta_write: std::sync::atomic::AtomicBool,
}

impl ProjectService {
    fn new_project_id() -> String {
        format!("prj_{}", Uuid::new_v4().simple())
    }

    /// Creates project service.
    pub fn new(
        data: Arc<dyn DataAdapter>,
        file: Arc<dyn FileAdapter>,
        project_data: Arc<dyn ProjectDataFactory>,
        zebflow_cfg: Arc<ProjectConfigurationService>,
        dependency_lock: Arc<DependencyLockService>,
    ) -> Self {
        Self {
            data,
            file,
            project_data,
            zebflow_cfg,
            dependency_lock,
            #[cfg(test)]
            fail_next_pipeline_meta_write: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// The configuration reader/writer this service resolves layouts through,
    /// for callers that must update `zebflow.yaml` and `zeb.lock` together.
    pub fn configuration_service(&self) -> Arc<ProjectConfigurationService> {
        self.zebflow_cfg.clone()
    }

    fn put_pipeline_meta(&self, meta: &PipelineMeta) -> Result<(), PlatformError> {
        #[cfg(test)]
        if self
            .fail_next_pipeline_meta_write
            .swap(false, std::sync::atomic::Ordering::SeqCst)
        {
            return Err(PlatformError::new(
                "PLATFORM_PIPELINE_META_INJECTED",
                "injected pipeline metadata write failure",
            ));
        }
        self.data.put_pipeline_meta(meta)
    }

    fn ensure_pipeline_editable(
        &self,
        owner: &str,
        project: &str,
        file_rel_path: &str,
        action: &str,
    ) -> Result<(), PlatformError> {
        let Some(meta) = self.get_pipeline_meta_by_file_id(owner, project, file_rel_path)? else {
            return Ok(());
        };
        let source = self.read_pipeline_source(owner, project, &meta.file_rel_path)?;
        if pipeline_source_is_locked(&source) {
            return Err(PlatformError::new(
                "PLATFORM_PIPELINE_LOCKED",
                format!(
                    "pipeline '{}' is locked and cannot be {}",
                    meta.file_rel_path, action
                ),
            ));
        }
        Ok(())
    }

    fn ensure_template_editable(
        &self,
        owner: &str,
        project: &str,
        rel_path: &str,
        action: &str,
    ) -> Result<(), PlatformError> {
        if self
            .zebflow_cfg
            .is_template_locked(owner, project, rel_path)?
        {
            return Err(PlatformError::new(
                "PLATFORM_TEMPLATE_LOCKED",
                format!("template '{}' is locked and cannot be {}", rel_path, action),
            ));
        }
        Ok(())
    }

    /// Lists projects by owner, populating title from zebflow.yaml.
    pub fn list_projects(&self, owner: &str) -> Result<Vec<PlatformProject>, PlatformError> {
        let mut projects = self.data.list_projects(owner)?;
        for p in &mut projects {
            p.title = self.zebflow_cfg.get_project_title(&p.owner, &p.project)?;
        }
        Ok(projects)
    }

    /// Gets one project by owner/slug, populating title from zebflow.yaml.
    pub fn get_project(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Option<PlatformProject>, PlatformError> {
        let Some(mut p) = self.data.get_project(owner, project)? else {
            return Ok(None);
        };
        p.title = self.zebflow_cfg.get_project_title(&p.owner, &p.project)?;
        Ok(Some(p))
    }

    /// Returns the ensured filesystem layout for one project.
    pub fn project_layout(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ProjectFileLayout, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        self.file.ensure_project_layout(&owner, &project)
    }

    /// Rewrites persisted pipeline ids from repository-relative to
    /// source-relative, once.
    ///
    /// Reads already tolerate the old form, so this is not what makes an
    /// existing project work; it is what stops every read from having to
    /// forgive the stored bytes. Running it twice converts nothing the second
    /// time.
    ///
    /// Nothing durable changes before the whole conversion is known to be
    /// safe: an ambiguous or colliding id refuses the migration outright
    /// rather than picking one of the two readings.
    pub fn migrate_pipeline_identity(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<PipelineIdentityMigration, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        if self.data.get_project(&owner, &project)?.is_none() {
            return Err(PlatformError::new(
                "PIPELINE_IDENTITY_MIGRATE",
                format!("project '{owner}/{project}' not found"),
            ));
        }
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let source_dir = layout.repo_source_dir();
        let rows = self.data.list_pipeline_meta(&owner, &project)?;

        let mut rewrites = Vec::new();
        let mut unchanged = 0usize;
        let mut claimed: BTreeMap<String, String> = BTreeMap::new();
        for row in &rows {
            let from = row.file_rel_path.trim().replace('\\', "/");
            let to = strip_legacy_identity_root(&from);
            if to == from {
                unchanged += 1;
            } else {
                // The legacy root is only removable when it is not also a real
                // directory inside this project's source tree. A project whose
                // source is `src` may genuinely keep `src/pipelines/...`, and
                // renaming that would move a file the owner never asked to
                // move.
                if source_dir.join(&from).is_file() {
                    return Err(PlatformError::new(
                        "PIPELINE_IDENTITY_MIGRATE",
                        format!(
                            "'{from}' also exists at '{}' inside the source root, so removing its \
                             '{LEGACY_PIPELINE_IDENTITY_ROOT}/' prefix would name a different file",
                            source_dir.join(&from).display()
                        ),
                    ));
                }
                rewrites.push(PipelineIdentityRewrite {
                    from: from.clone(),
                    to: to.clone(),
                });
            }
            if let Some(other) = claimed.insert(to.clone(), from.clone()) {
                return Err(PlatformError::new(
                    "PIPELINE_IDENTITY_MIGRATE",
                    format!("'{other}' and '{from}' both become '{to}'"),
                ));
            }
        }

        let mut config = self.zebflow_cfg.read_or_default(&owner, &project)?;
        let bootstrap: Vec<String> = config
            .configs
            .bootstrap
            .activate
            .iter()
            .map(|pattern| strip_legacy_identity_root(pattern.trim()))
            .collect();
        let bootstrap_changed = bootstrap != config.configs.bootstrap.activate;

        if rewrites.is_empty() && !bootstrap_changed {
            return Ok(PipelineIdentityMigration {
                rewrites,
                already_canonical: unchanged,
                bootstrap_activate: Vec::new(),
                recovery_path: None,
            });
        }

        // The recovery copy is written before the first row moves, so an
        // interrupted migration leaves a readable record of what the ids
        // were. It lives in `data/recovery/`, not `repo/`: a
        // machine-generated recovery copy must never enter the git-tracked
        // repository (`project-directory.md` §1, §5).
        let recovery_path = layout.data_recovery_dir().join(format!(
            "{PIPELINE_IDENTITY_BACKUP_FILE}-{}.json",
            recovery_date_stamp()
        ));
        let record = serde_json::json!({
            "source": layout.repo_layout.source,
            "pipelines": rewrites
                .iter()
                .map(|rewrite| serde_json::json!({ "from": rewrite.from, "to": rewrite.to }))
                .collect::<Vec<_>>(),
            "bootstrap_activate": config.configs.bootstrap.activate,
        });
        let bytes = serde_json::to_vec_pretty(&record)
            .map_err(|error| PlatformError::new("PIPELINE_IDENTITY_MIGRATE", error.to_string()))?;
        match std::fs::read(&recovery_path) {
            Ok(existing) if existing != bytes => {
                return Err(PlatformError::new(
                    "PIPELINE_IDENTITY_MIGRATE",
                    format!(
                        "recovery copy '{}' already exists with different content",
                        recovery_path.display()
                    ),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if let Some(parent) = recovery_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                atomic_write(&recovery_path, &bytes)?;
            }
            Err(error) => {
                return Err(PlatformError::new(
                    "PIPELINE_IDENTITY_MIGRATE",
                    format!("failed reading recovery copy: {error}"),
                ));
            }
        }

        for row in &rows {
            let from = row.file_rel_path.trim().replace('\\', "/");
            let Some(rewrite) = rewrites.iter().find(|rewrite| rewrite.from == from) else {
                continue;
            };
            let mut moved = row.clone();
            moved.file_rel_path = rewrite.to.clone();
            moved.virtual_path = virtual_path_from_file_rel_path(&rewrite.to);
            self.put_pipeline_meta(&moved)?;
            self.data
                .delete_pipeline_meta(&row.owner, &row.project, &row.file_rel_path)?;
        }
        if bootstrap_changed {
            config.configs.bootstrap.activate = bootstrap.clone();
            self.zebflow_cfg
                .set_bootstrap(&owner, &project, config.configs.bootstrap.clone())?;
        }

        Ok(PipelineIdentityMigration {
            rewrites,
            already_canonical: unchanged,
            bootstrap_activate: if bootstrap_changed {
                bootstrap
            } else {
                Vec::new()
            },
            recovery_path: Some(recovery_path),
        })
    }

    /// Canonical identity of the pipeline `raw` names in this project.
    ///
    /// Callers outside this service reach identity through here rather than
    /// through the free function, so none of them has to know that resolving a
    /// name requires the project's declared layout.
    pub fn pipeline_identity(
        &self,
        owner: &str,
        project: &str,
        raw: &str,
    ) -> Result<String, PlatformError> {
        let layout = self.project_layout(owner, project)?;
        Ok(normalize_pipeline_file_rel_path(&layout.repo_layout, raw))
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
        let owner_user_id = self
            .data
            .get_user_auth(&owner)?
            .map(|user| user.profile.user_id)
            .ok_or_else(|| PlatformError::new("PLATFORM_USER_NOT_FOUND", "owner user not found"))?;
        let created_at = existing.as_ref().map(|p| p.created_at).unwrap_or(now);
        let title = req.title.as_deref().unwrap_or("").trim().to_string();
        let title = if title.is_empty() {
            project.replace('-', " ")
        } else {
            title
        };

        let record = PlatformProject {
            project_id: existing
                .as_ref()
                .map(|p| p.project_id.clone())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(Self::new_project_id),
            owner_user_id: existing
                .as_ref()
                .map(|p| p.owner_user_id.clone())
                .filter(|v| !v.is_empty())
                .unwrap_or(owner_user_id),
            owner: owner.clone(),
            project: project.clone(),
            title: title.clone(),
            created_at,
            updated_at: now,
        };
        self.data.put_project(&record)?;
        self.data.put_hub_authority(&HubAuthority {
            authority_id: format!("mka_{}", record.project_id),
            host_project_id: record.project_id.clone(),
            owner: owner.clone(),
            project: project.clone(),
            enabled: false,
            public_base_url: String::new(),
            created_at,
            updated_at: now,
        })?;
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        // Rename initial "main" branch if caller requested a different name.
        // Safe even with no commits — git branch -m works on an unborn branch.
        if let Some(ref branch) = req.local_branch {
            let branch = branch.trim();
            if !branch.is_empty() && branch != "main" {
                let _ = Command::new("git")
                    .arg("-C")
                    .arg(&layout.repo_dir)
                    .arg("branch")
                    .arg("-m")
                    .arg("main")
                    .arg(branch)
                    .output();
            }
        }
        // Write title to portable zebflow.yaml configuration.
        self.zebflow_cfg
            .ensure_initialized(&owner, &project, &title)?;
        // Write zeb.lock if it doesn't exist yet
        self.dependency_lock
            .write_if_missing(&owner, &project, &DependencyLockSpec::default())?;
        self.project_data.initialize_project(&layout)?;
        self.ensure_default_template_workspace(&layout)?;

        Ok((record, layout))
    }

    /// Transfer project ownership from one user to another within the same office.
    ///
    /// Re-keys all catalog records and moves the project directory tree on disk.
    /// Used for sovereignty recovery after controller dissolution (the "caravanserai" scenario).
    pub fn transfer_project_owner(
        &self,
        old_owner: &str,
        project: &str,
        new_owner: &str,
    ) -> Result<(), PlatformError> {
        let old_owner = slug_segment(old_owner);
        let new_owner_slug = slug_segment(new_owner);
        let project = slug_segment(project);

        if old_owner == new_owner_slug {
            return Err(PlatformError::new(
                "PLATFORM_TRANSFER_INVALID",
                "source and target owners are the same",
            ));
        }
        if old_owner.is_empty() || new_owner_slug.is_empty() || project.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_TRANSFER_INVALID",
                "owner and project must not be empty",
            ));
        }

        // Verify old project exists.
        let old_project = self
            .data
            .get_project(&old_owner, &project)?
            .ok_or_else(|| {
                PlatformError::new("PLATFORM_TRANSFER_NOT_FOUND", "source project not found")
            })?;

        // Verify new_owner is a real user.
        let new_user = self.data.get_user_auth(&new_owner_slug)?.ok_or_else(|| {
            PlatformError::new("PLATFORM_TRANSFER_INVALID", "target owner user not found")
        })?;

        // Verify no conflict — project must not already exist under new_owner.
        if self.data.get_project(&new_owner_slug, &project)?.is_some() {
            return Err(PlatformError::new(
                "PLATFORM_TRANSFER_CONFLICT",
                "a project with this slug already exists under the target owner",
            ));
        }

        let new_owner_user_id = &new_user.profile.user_id;

        // 1. Re-key all catalog records in a single transaction.
        self.data.transfer_project_owner(
            &old_owner,
            &project,
            &new_owner_slug,
            new_owner_user_id,
        )?;

        // 2. Move files on disk.
        let old_layout = self.file.ensure_project_layout(&old_owner, &project)?;
        // Ensure the new owner's directory exists.
        let new_layout = self.file.ensure_project_layout(&new_owner_slug, &project)?;

        if old_layout.root.exists() {
            // Remove the (empty) new layout directory that ensure_project_layout just created,
            // then rename the old directory into its place.
            let _ = fs::remove_dir_all(&new_layout.root);
            if let Err(rename_err) = fs::rename(&old_layout.root, &new_layout.root) {
                // Rename failed — attempt cross-device copy then remove.
                if copy_dir_recursive(&old_layout.root, &new_layout.root).is_ok() {
                    let _ = fs::remove_dir_all(&old_layout.root);
                } else {
                    // File move completely failed — reverse the DB transfer.
                    let old_user_id = &old_project.owner_user_id;
                    let _ = self.data.transfer_project_owner(
                        &new_owner_slug,
                        &project,
                        &old_owner,
                        old_user_id,
                    );
                    return Err(PlatformError::new(
                        "PLATFORM_TRANSFER_FS",
                        format!("failed to move project files: {rename_err}"),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Upserts one pipeline source file + metadata catalog entry.
    ///
    /// `file_rel_path` is the canonical identifier, e.g. `"api/my-hook.zf.json"`,
    /// relative to the project's declared source root.
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
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let file_rel_path = normalize_pipeline_file_rel_path(&layout.repo_layout, file_rel_path);
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
        self.ensure_pipeline_editable(&owner, &project, &file_rel_path, "edited")?;
        let graph = parse_and_validate_pipeline_source_for_save(source)?;
        let canonical_source = encode_pipeline_graph(graph)
            .and_then(|bytes| {
                String::from_utf8(bytes)
                    .map_err(|error| crate::contracts::ContractError::invalid(error.to_string()))
            })
            .map_err(|error| {
                PlatformError::new(
                    "PLATFORM_PIPELINE_SERIALIZE",
                    format!("failed serializing canonical pipeline source: {error}"),
                )
            })?;

        self.project_data.initialize_project(&layout)?;
        self.ensure_webhook_paths_available(&owner, &project, &canonical_source, &file_rel_path)?;

        let file_abs_path = self.pipeline_abs_path(&layout, &file_rel_path)?;
        let previous_source = fs::read(&file_abs_path).ok();
        let existing = self.get_pipeline_meta_by_file_id(&owner, &project, &file_rel_path)?;
        if let Some(parent) = file_abs_path.parent() {
            fs::create_dir_all(parent)?;
        }
        atomic_write(&file_abs_path, canonical_source.as_bytes())?;

        let vpath = virtual_path_from_file_rel_path(&file_rel_path);
        let now = now_ts();
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
            hash: stable_hash_hex(&canonical_source),
            active_hash: existing.as_ref().and_then(|m| m.active_hash.clone()),
            activated_at: existing.as_ref().and_then(|m| m.activated_at),
            created_at,
            updated_at: now,
        };
        if let Err(error) = self.put_pipeline_meta(&meta) {
            let rollback = match previous_source {
                Some(previous) => atomic_write(&file_abs_path, &previous),
                None => durable_remove_file(&file_abs_path),
            };
            if let Err(rollback_error) = rollback {
                return Err(PlatformError::new(
                    "PLATFORM_PIPELINE_RECOVERY",
                    format!(
                        "pipeline metadata update failed: {}; source rollback also failed: {}",
                        error.message, rollback_error
                    ),
                ));
            }
            return Err(error);
        }
        Ok(meta)
    }

    pub fn check_webhook_path_conflict(
        &self,
        owner: &str,
        project: &str,
        graph: &PipelineGraph,
        self_file_rel_path: &str,
    ) -> Result<Vec<WebhookPathConflict>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let self_file_rel_path =
            normalize_pipeline_file_rel_path(&layout.repo_layout, self_file_rel_path);
        let wanted = webhook_triggers_from_graph(graph);
        if wanted.is_empty() {
            return Ok(Vec::new());
        }

        // Rows arrive through `list_pipeline_meta_rows`, which already reads
        // them through the identity rule.
        let rows = self.list_pipeline_meta_rows(&owner, &project)?;
        let mut conflicts = Vec::new();
        for meta in rows {
            if meta.file_rel_path == self_file_rel_path {
                continue;
            }
            let Ok(source) = self.read_pipeline_source(&owner, &project, &meta.file_rel_path)
            else {
                continue;
            };
            let Some(existing) = webhook_triggers_from_source(&source) else {
                continue;
            };
            for trigger in existing {
                for candidate in &wanted {
                    if candidate.method == trigger.method && candidate.path == trigger.path {
                        conflicts.push(WebhookPathConflict {
                            path: trigger.path.clone(),
                            method: trigger.method.clone(),
                            pipeline_name: meta.name.clone(),
                            file_rel_path: meta.file_rel_path.clone(),
                        });
                    }
                }
            }
        }
        Ok(conflicts)
    }

    pub fn ensure_webhook_paths_available(
        &self,
        owner: &str,
        project: &str,
        source: &str,
        self_file_rel_path: &str,
    ) -> Result<(), PlatformError> {
        let graph = parse_and_validate_pipeline_source_for_save(source)?;
        let conflicts =
            self.check_webhook_path_conflict(owner, project, &graph, self_file_rel_path)?;
        if conflicts.is_empty() {
            return Ok(());
        }
        let first = &conflicts[0];
        Err(PlatformError::new(
            "PLATFORM_PIPELINE_WEBHOOK_CONFLICT",
            format!(
                "{} {} is already registered by pipeline '{}'",
                first.method, first.path, first.pipeline_name
            ),
        ))
    }

    /// Lists all pipeline metadata rows for one project.
    pub fn list_pipeline_meta_rows(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<PipelineMeta>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let source_dir = layout.repo_source_dir();
        let mut rows: Vec<PipelineMeta> = self
            .data
            .list_pipeline_meta(&owner, &project)?
            .into_iter()
            .map(|m| adopt_pipeline_meta(&layout.repo_layout, m))
            .filter(|m| source_dir.join(&m.file_rel_path).is_file())
            .collect();
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
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let wanted = normalize_pipeline_file_rel_path(&layout.repo_layout, file_id);
        let meta = self
            .data
            .list_pipeline_meta(&owner, &project)?
            .into_iter()
            .map(|m| adopt_pipeline_meta(&layout.repo_layout, m))
            .find(|m| m.file_rel_path == wanted);
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
        let abs = self.pipeline_abs_path(&layout, file_rel_path)?;
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
        self.ensure_pipeline_editable(&owner, &project, file_rel_path, "activated")?;
        let Some(mut meta) = self.get_pipeline_meta_by_file_id(&owner, &project, file_rel_path)?
        else {
            return Err(PlatformError::new(
                "PLATFORM_PIPELINE_MISSING",
                format!("pipeline '{}' not found", file_rel_path),
            ));
        };
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let source = self.read_pipeline_source(&owner, &project, &meta.file_rel_path)?;
        let graph = parse_and_validate_pipeline_source(&source)?;
        self.dependency_lock
            .validate_pipeline_dependencies(&owner, &project, &graph)?;
        if graph
            .nodes
            .iter()
            .any(|node| node.kind.starts_with("n.web."))
        {
            let requested = self.zebflow_cfg.get_rwe_libraries(&owner, &project)?;
            self.dependency_lock
                .validate_rwe_dependencies(&owner, &project, &requested)?;
        }
        self.ensure_webhook_paths_available(&owner, &project, &source, &meta.file_rel_path)?;
        let current_hash = stable_hash_hex(&source);
        let previous_meta = meta.clone();
        meta.hash = current_hash.clone();
        meta.active_hash = Some(current_hash.clone());
        meta.activated_at = Some(now_ts());
        meta.updated_at = now_ts();

        // Compile before changing durable active state. The runtime registry
        // uses this same constructor after commit, so an invalid candidate can
        // never replace the last executable snapshot.
        crate::platform::services::pipeline_runtime::CompiledPipeline::from_active_meta(
            &meta, &source, None,
        )?;

        let snapshot_path =
            self.runtime_pipeline_snapshot_path(&layout, &meta.file_rel_path, &current_hash)?;
        atomic_write(&snapshot_path, source.as_bytes())?;

        if let Err(error) = self.put_pipeline_meta(&meta) {
            durable_remove_file(&snapshot_path).map_err(|rollback_error| {
                PlatformError::new(
                    "PLATFORM_PIPELINE_RECOVERY",
                    format!(
                        "pipeline activation metadata update failed: {}; candidate snapshot cleanup also failed: {}",
                        error.message, rollback_error
                    ),
                )
            })?;
            return Err(error);
        }
        if let Err(error) = self.remove_runtime_pipeline_snapshots(
            &layout,
            &meta.file_rel_path,
            Some(&current_hash),
        ) {
            // The metadata pointer and its snapshot are already committed. Keep
            // the new active state and leave stale snapshots for later cleanup.
            eprintln!(
                "pipeline activation cleanup warning for '{}': {} (previous active hash: {:?})",
                meta.file_rel_path, error.message, previous_meta.active_hash
            );
        }
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
        self.ensure_pipeline_editable(&owner, &project, file_rel_path, "deactivated")?;
        let Some(mut meta) = self.get_pipeline_meta_by_file_id(&owner, &project, file_rel_path)?
        else {
            return Err(PlatformError::new(
                "PLATFORM_PIPELINE_MISSING",
                "pipeline not found",
            ));
        };
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        meta.active_hash = None;
        meta.activated_at = None;
        meta.updated_at = now_ts();
        self.put_pipeline_meta(&meta)?;
        self.remove_runtime_pipeline_snapshots(&layout, &meta.file_rel_path, None)?;
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
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let mut rows = self
            .data
            .list_pipeline_meta(&owner, &project)?
            .into_iter()
            .filter(|m| m.active_hash.as_deref().is_some())
            .map(|m| adopt_pipeline_meta(&layout.repo_layout, m))
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
        let snapshot_path =
            self.runtime_pipeline_snapshot_path(&layout, &meta.file_rel_path, active_hash)?;
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
            let m = adopt_pipeline_meta(&layout.repo_layout, m);
            let vp = m.virtual_path.clone();
            if vp == current_path {
                let is_active = m
                    .active_hash
                    .as_deref()
                    .map(|h| !h.is_empty() && h == m.hash)
                    .unwrap_or(false);
                let has_draft = m
                    .active_hash
                    .as_deref()
                    .map(|h| !h.is_empty() && h != m.hash)
                    .unwrap_or(false);
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
            layout.repo_source_dir()
        } else {
            layout
                .repo_source_dir()
                .join(current_path.trim_start_matches('/'))
        };

        let mut files: Vec<RegistryFileItem> = Vec::new();

        if phys_dir.is_dir() {
            if let Ok(rd) = std::fs::read_dir(&phys_dir) {
                for entry in rd.flatten() {
                    let ft = match entry.file_type() {
                        Ok(ft) => ft,
                        Err(_) => continue,
                    };
                    let fname = entry.file_name().to_string_lossy().into_owned();
                    if fname.starts_with('.') {
                        continue;
                    }

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
                            layout.repo_layout.source_rel(&fname)
                        } else {
                            layout.repo_layout.source_rel(&format!(
                                "{}/{fname}",
                                current_path.trim_start_matches('/')
                            ))
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
            let ai = SPECIAL_ORDER
                .iter()
                .position(|&s| s == a.name)
                .unwrap_or(99);
            let bi = SPECIAL_ORDER
                .iter()
                .position(|&s| s == b.name)
                .unwrap_or(99);
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

        let mut items = Vec::new();
        let mut default_file = None;
        walk_template_tree(
            &layout.repo_source_dir(),
            &layout.repo_source_dir(),
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

        let root = &layout.repo_source_dir();
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
        Ok(layout.repo_source_dir())
    }

    /// Resolve the absolute filesystem path for a template file given its `rel_path`.
    /// Returns `Err` if the path is invalid or escapes the template root.
    /// Used by cache eviction code to map a relative path → absolute path.
    pub fn resolve_template_abs_path(
        &self,
        owner: &str,
        project: &str,
        rel_path: &str,
    ) -> Result<PathBuf, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let (_, abs) = resolve_template_entry(&layout.repo_source_dir(), rel_path)?;
        // Canonicalize so the path format matches what the RWE compiler stores in
        // dependency_paths (the compiler uses fs::canonicalize via canonical_or_current).
        // Without this, a relative data_root like ".zebflow-platform-data" causes a
        // path mismatch and eviction never fires.
        Ok(std::fs::canonicalize(&abs).unwrap_or(abs))
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

        let (rel, abs) = resolve_template_entry(&layout.repo_source_dir(), rel_path)?;
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

        let (rel, abs) = resolve_template_entry(&layout.repo_source_dir(), rel_path)?;
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
        self.ensure_template_editable(&owner, &project, &req.rel_path, "edited")?;
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        self.ensure_default_template_workspace(&layout)?;

        let (rel, abs) = resolve_template_entry(&layout.repo_source_dir(), &req.rel_path)?;
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&abs, &req.content)?;
        Ok(template_payload_from_content(&rel, &req.content))
    }

    /// Search template files for a pattern. Returns (rel_path, line_number, block) tuples.
    /// Optional `glob` filters which files to search (e.g. "pages/*.tsx", "**/*.tsx").
    /// `context` lines before and after each match are included in the block.
    pub fn search_template_files(
        &self,
        owner: &str,
        project: &str,
        pattern: &str,
        glob: Option<&str>,
        context: usize,
    ) -> Result<Vec<(String, usize, String)>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let root = &layout.repo_source_dir();

        let pattern_lower = pattern.to_lowercase();
        let mut matches: Vec<(String, usize, String)> = Vec::new();

        let mut all_files: Vec<(String, std::path::PathBuf)> = Vec::new();
        collect_all_files(root, root, &mut all_files);

        for (rel, abs) in &all_files {
            // Template search must never include pipeline graph files.
            if rel.ends_with(".zf.json") {
                continue;
            }
            if let Some(g) = glob {
                if !template_glob_matches(g, rel) {
                    continue;
                }
            }
            let content = match fs::read_to_string(abs) {
                Ok(c) => c,
                Err(_) => continue,
            };
            if context == 0 {
                for (line_idx, line) in content.lines().enumerate() {
                    if line.to_lowercase().contains(&pattern_lower) {
                        matches.push((rel.clone(), line_idx + 1, line.to_string()));
                    }
                }
            } else {
                let all_lines: Vec<&str> = content.lines().collect();
                for (line_idx, line) in all_lines.iter().enumerate() {
                    if line.to_lowercase().contains(&pattern_lower) {
                        let start = line_idx.saturating_sub(context);
                        let end = (line_idx + context + 1).min(all_lines.len());
                        let block = all_lines[start..end].join("\n");
                        matches.push((rel.clone(), line_idx + 1, block));
                    }
                }
            }
        }

        Ok(matches)
    }

    /// Search pipeline `.zf.json` files for a pattern. Returns (rel_path, line_number, block) tuples.
    /// Optional `glob` filters which files to search.
    /// `context` lines before and after each match are included in the block.
    pub fn search_pipeline_files(
        &self,
        owner: &str,
        project: &str,
        pattern: &str,
        glob: Option<&str>,
        context: usize,
    ) -> Result<Vec<(String, usize, String)>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let root = &layout.repo_source_dir();

        let pattern_lower = pattern.to_lowercase();
        let mut matches: Vec<(String, usize, String)> = Vec::new();

        let mut all_files: Vec<(String, std::path::PathBuf)> = Vec::new();
        collect_all_files(root, root, &mut all_files);

        for (rel, abs) in &all_files {
            if !rel.ends_with(".zf.json") {
                continue;
            }
            if let Some(g) = glob {
                if !template_glob_matches(g, rel) {
                    continue;
                }
            }
            let content = match fs::read_to_string(abs) {
                Ok(c) => c,
                Err(_) => continue,
            };
            if context == 0 {
                for (line_idx, line) in content.lines().enumerate() {
                    if line.to_lowercase().contains(&pattern_lower) {
                        matches.push((rel.clone(), line_idx + 1, line.to_string()));
                    }
                }
            } else {
                let all_lines: Vec<&str> = content.lines().collect();
                for (line_idx, line) in all_lines.iter().enumerate() {
                    if line.to_lowercase().contains(&pattern_lower) {
                        let start = line_idx.saturating_sub(context);
                        let end = (line_idx + context + 1).min(all_lines.len());
                        let block = all_lines[start..end].join("\n");
                        matches.push((rel.clone(), line_idx + 1, block));
                    }
                }
            }
        }

        Ok(matches)
    }

    /// Surgical string replacement in a template file.
    /// Fails if old_string not found or appears more than once.
    pub fn edit_template_file(
        &self,
        owner: &str,
        project: &str,
        rel_path: &str,
        old_string: &str,
        new_string: &str,
    ) -> Result<usize, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        self.ensure_template_editable(&owner, &project, rel_path, "edited")?;
        let layout = self.file.ensure_project_layout(&owner, &project)?;

        let (rel, abs) = resolve_template_entry(&layout.repo_source_dir(), rel_path)?;
        if !abs.is_file() {
            return Err(PlatformError::new(
                "PLATFORM_TEMPLATE_MISSING",
                format!("template file '{}' not found", rel),
            ));
        }

        let content = fs::read_to_string(&abs)?;
        let count = content.matches(old_string).count();

        if count == 0 {
            return Err(PlatformError::new(
                "PLATFORM_TEMPLATE_EDIT",
                "old_string not found in file",
            ));
        }
        if count > 1 {
            return Err(PlatformError::new(
                "PLATFORM_TEMPLATE_EDIT",
                format!(
                    "old_string matches {} times — provide more context to make it unique",
                    count
                ),
            ));
        }

        let line_number = content
            .lines()
            .enumerate()
            .find(|(_, line)| line.contains(old_string))
            .map(|(i, _)| i + 1)
            .unwrap_or(0);

        let new_content = content.replacen(old_string, new_string, 1);
        fs::write(&abs, &new_content)?;

        Ok(line_number)
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
        if !parent_rel.is_empty() {
            self.ensure_template_editable(&owner, &project, &parent_rel, "modified")?;
        }
        let parent_abs = layout.repo_source_dir().join(&parent_rel);
        if !parent_abs.starts_with(&layout.repo_source_dir()) {
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
            let abs = layout.repo_source_dir().join(&rel);
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
        self.ensure_template_editable(&owner, &project, &rel, "created")?;
        let abs = layout.repo_source_dir().join(&rel);
        if !abs.starts_with(&layout.repo_source_dir()) {
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
        self.ensure_template_editable(&owner, &project, rel_path, "deleted")?;
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        self.ensure_default_template_workspace(&layout)?;

        let (rel, abs) = resolve_template_entry(&layout.repo_source_dir(), rel_path)?;
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
        self.ensure_template_editable(&owner, &project, &req.from_rel_path, "moved")?;
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        self.ensure_default_template_workspace(&layout)?;

        let (from_rel, from_abs) =
            resolve_template_entry(&layout.repo_source_dir(), &req.from_rel_path)?;
        if !from_abs.exists() {
            return Err(PlatformError::new(
                "PLATFORM_TEMPLATE_MISSING",
                format!("template entry '{}' not found", from_rel),
            ));
        }
        let parent_rel = normalize_template_folder_rel_path(&req.to_parent_rel_path);
        if !parent_rel.is_empty() {
            self.ensure_template_editable(&owner, &project, &parent_rel, "modified")?;
        }
        let parent_abs = layout.repo_source_dir().join(&parent_rel);
        if !parent_abs.starts_with(&layout.repo_source_dir()) {
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

    /// Returns the current health of the project's Git metadata.
    pub fn get_repo_git_health(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ProjectGitHealth, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let git_dir = &layout.repo_git_dir;
        let head_exists = git_dir.join("HEAD").is_file();
        let config_exists = git_dir.join("config").is_file();
        let objects_exists = git_dir.join("objects").is_dir();
        let refs_exists = git_dir.join("refs").is_dir();
        let rev_parse = Command::new("git")
            .arg("-C")
            .arg(&layout.repo_dir)
            .arg("rev-parse")
            .arg("--is-inside-work-tree")
            .output();
        let (is_work_tree, last_error) = match rev_parse {
            Ok(output) if output.status.success() => {
                let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
                (value == "true", String::new())
            }
            Ok(output) => (
                false,
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ),
            Err(err) => (false, err.to_string()),
        };
        let branch = if is_work_tree {
            self.get_repo_git_branch(&owner, &project)
                .unwrap_or_default()
        } else {
            String::new()
        };
        let (state, recommended_action) = if !git_dir.exists() {
            ("missing".to_string(), "repair".to_string())
        } else if is_work_tree {
            if branch.is_empty() {
                ("healthy".to_string(), "none".to_string())
            } else {
                ("healthy".to_string(), "none".to_string())
            }
        } else if head_exists || config_exists || objects_exists || refs_exists {
            ("broken".to_string(), "repair".to_string())
        } else {
            ("broken".to_string(), "reinitialize".to_string())
        };
        Ok(ProjectGitHealth {
            state,
            repo_path: layout.repo_dir.display().to_string(),
            git_dir_path: git_dir.display().to_string(),
            git_dir_exists: git_dir.exists(),
            is_work_tree,
            head_exists,
            config_exists,
            objects_exists,
            refs_exists,
            branch,
            last_error,
            recommended_action,
        })
    }

    /// Repairs or rebuilds project Git metadata while preserving the worktree.
    pub fn repair_repo_git(
        &self,
        owner: &str,
        project: &str,
        mode: ProjectGitRepairMode,
    ) -> Result<ProjectGitHealth, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let health = self.get_repo_git_health(&owner, &project)?;
        if health.state == "healthy" && mode == ProjectGitRepairMode::Repair {
            return Ok(health);
        }

        match mode {
            ProjectGitRepairMode::Repair => {
                init_git_repo(&layout.repo_dir)?;
            }
            ProjectGitRepairMode::Reinitialize | ProjectGitRepairMode::Reset => {
                if layout.repo_git_dir.exists() {
                    let backup = layout.root.join(format!("repo.git.broken.{}", now_ts()));
                    fs::rename(&layout.repo_git_dir, backup)?;
                }
                init_git_repo(&layout.repo_dir)?;
            }
        }

        self.get_repo_git_health(&owner, &project)
    }

    /// Returns the current local branch name for the project's git repo.
    /// Falls back to an empty string if git is not initialized or has no commits yet.
    pub fn get_repo_git_branch(&self, owner: &str, project: &str) -> Result<String, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let out = Command::new("git")
            .arg("-C")
            .arg(&layout.repo_dir)
            .arg("rev-parse")
            .arg("--abbrev-ref")
            .arg("HEAD")
            .output()
            .map_err(|e| PlatformError::new("PLATFORM_REPO_GIT", e.to_string()))?;
        if !out.status.success() {
            return Ok(String::new());
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// Lists all local branch names for the project's git repo.
    /// Returns an empty vec if the repo has no commits yet.
    pub fn list_repo_git_local_branches(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<String>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let out = Command::new("git")
            .arg("-C")
            .arg(&layout.repo_dir)
            .arg("branch")
            .arg("--list")
            .arg("--format=%(refname:short)")
            .output()
            .map_err(|e| PlatformError::new("PLATFORM_REPO_GIT", e.to_string()))?;
        if !out.status.success() {
            return Ok(vec![]);
        }
        let names: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        Ok(names)
    }

    /// Checks out (or creates) a local git branch.
    /// `create = true` → `git checkout -b <branch>` (fails if branch already exists).
    /// `create = false` → `git checkout <branch>` (fails if branch doesn't exist).
    pub fn checkout_repo_git_branch(
        &self,
        owner: &str,
        project: &str,
        branch: &str,
        create: bool,
    ) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(&layout.repo_dir).arg("checkout");
        if create {
            cmd.arg("-b");
        }
        cmd.arg(branch);
        let out = cmd
            .output()
            .map_err(|e| PlatformError::new("PLATFORM_REPO_GIT", e.to_string()))?;
        if !out.status.success() {
            return Err(PlatformError::new(
                "PLATFORM_REPO_GIT_CHECKOUT",
                String::from_utf8_lossy(&out.stderr).trim().to_string(),
            ));
        }
        Ok(())
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
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let wanted = normalize_pipeline_file_rel_path(&layout.repo_layout, file_rel_path);
        self.ensure_pipeline_editable(&owner, &project, &wanted, "deleted")?;

        let abs = self.pipeline_abs_path(&layout, &wanted)?;

        // Remove source file from disk.
        if abs.is_file() {
            fs::remove_file(&abs)
                .map_err(|e| PlatformError::new("PLATFORM_PIPELINE_DELETE", e.to_string()))?;
        }

        // The stored row may still carry the source root, so it is matched
        // through the identity rule but deleted by the key it was written with.
        let meta = self
            .data
            .list_pipeline_meta(&owner, &project)?
            .into_iter()
            .find(|m| {
                normalize_pipeline_file_rel_path(&layout.repo_layout, &m.file_rel_path) == wanted
            });
        if let Some(m) = meta {
            self.data
                .delete_pipeline_meta(&m.owner, &m.project, &m.file_rel_path)?;
        }

        Ok(())
    }

    fn ensure_default_template_workspace(
        &self,
        layout: &ProjectFileLayout,
    ) -> Result<(), PlatformError> {
        // Subdirs are created by ensure_project_layout. Only scaffold default files here.
        let pages_dir = layout.repo_source_dir().join("pages");
        let styles_dir = layout.repo_source_dir().join("styles");
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

        // Scaffold shared/ directories for cross-module shared code.
        // @/shared/ui, @/shared/layout, @/shared/lib — import path: @/shared/ui/button
        for subdir in ["shared/ui", "shared/layout", "shared/lib"] {
            let dir = layout.repo_source_dir().join(subdir);
            fs::create_dir_all(&dir)?;
            let gitkeep = dir.join(".gitkeep");
            if !gitkeep.exists() {
                fs::write(&gitkeep, "")?;
            }
        }

        Ok(())
    }

    /// Returns the absolute filesystem path for a pipeline given its `file_rel_path`.
    fn pipeline_abs_path(
        &self,
        layout: &ProjectFileLayout,
        file_rel_path: &str,
    ) -> Result<PathBuf, PlatformError> {
        // Identity is resolved here, so a caller may hand in a stored row that
        // still carries the source root and still reach the same file.
        let identity = normalize_pipeline_file_rel_path(&layout.repo_layout, file_rel_path);
        let source_dir = layout.repo_source_dir();
        let abs = source_dir.join(&identity);
        if !abs.starts_with(&source_dir) {
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
    /// `api/foo.zf.json` + `abc123` -> `<runtime>/api/foo.abc123.zf.json`
    fn runtime_pipeline_snapshot_path(
        &self,
        layout: &ProjectFileLayout,
        file_rel_path: &str,
        hash: &str,
    ) -> Result<PathBuf, PlatformError> {
        // Strip the source root to get the sub-path, then replace .zf.json
        // with .{hash}.zf.json
        let sub = strip_source_root(layout, file_rel_path);
        let snapshot_name = if let Some(stem) = sub.strip_suffix(".zf.json") {
            format!("{stem}.{}.zf.json", slug_segment(hash))
        } else if let Some(stem) = sub.strip_suffix(".json") {
            format!("{stem}.{}.json", slug_segment(hash))
        } else {
            format!("{sub}.{}", slug_segment(hash))
        };
        let abs = layout.data_cache_pipelines_dir().join(&snapshot_name);
        if !abs.starts_with(layout.data_cache_pipelines_dir()) {
            return Err(PlatformError::new(
                "PLATFORM_PIPELINE_PATH",
                "resolved runtime pipeline snapshot escaped runtime root",
            ));
        }
        Ok(abs)
    }

    /// Removes stale runtime snapshots for one logical pipeline file.
    ///
    /// `keep_hash`, when provided, preserves that one exact active snapshot and
    /// removes all other historical hashes for the same `file_rel_path`.
    fn remove_runtime_pipeline_snapshots(
        &self,
        layout: &ProjectFileLayout,
        file_rel_path: &str,
        keep_hash: Option<&str>,
    ) -> Result<(), PlatformError> {
        let sub = strip_source_root(layout, file_rel_path);
        let snapshot_prefix = if let Some(stem) = sub.strip_suffix(".zf.json") {
            format!("{stem}.")
        } else if let Some(stem) = sub.strip_suffix(".json") {
            format!("{stem}.")
        } else {
            format!("{sub}.")
        };
        let runtime_root = layout.data_cache_pipelines_dir();
        let parent = runtime_root.join(
            Path::new(sub)
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_default(),
        );
        if !parent.exists() {
            return Ok(());
        }

        let keep_name = keep_hash.map(|hash| {
            if let Some(stem) = sub.strip_suffix(".zf.json") {
                format!("{stem}.{}.zf.json", slug_segment(hash))
            } else if let Some(stem) = sub.strip_suffix(".json") {
                format!("{stem}.{}.json", slug_segment(hash))
            } else {
                format!("{sub}.{}", slug_segment(hash))
            }
        });

        for entry in fs::read_dir(&parent)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Ok(rel) = path.strip_prefix(&runtime_root) else {
                continue;
            };
            let rel = rel.to_string_lossy().replace('\\', "/");
            if !rel.starts_with(&snapshot_prefix) {
                continue;
            }
            if let Some(keep_name) = &keep_name
                && rel == *keep_name
            {
                continue;
            }
            durable_remove_file(&path)?;
        }
        Ok(())
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
        walk_docs_tree(&layout.repo_docs_dir(), &layout.repo_docs_dir(), &mut items)?;
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
        let (_rel, abs) = resolve_doc_path(&layout.repo_docs_dir(), rel_path)?;
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
        let (rel, abs) = resolve_doc_path(&layout.repo_docs_dir(), rel_path)?;

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

    /// Creates one project docs folder under `repo/docs`.
    pub fn create_project_doc_folder(
        &self,
        owner: &str,
        project: &str,
        rel_path: &str,
    ) -> Result<ProjectDocItem, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let (rel, abs) = resolve_doc_folder_path(&layout.repo_docs_dir(), rel_path, false)?;
        fs::create_dir_all(&abs)?;
        let name = abs
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("docs")
            .to_string();
        Ok(ProjectDocItem {
            path: rel,
            name,
            kind: "folder".to_string(),
        })
    }

    /// Moves one project docs file or folder into another docs folder.
    pub fn move_project_doc_entry(
        &self,
        owner: &str,
        project: &str,
        req: &ProjectDocMoveRequest,
    ) -> Result<String, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;

        let (from_rel, from_abs) = resolve_doc_path(&layout.repo_docs_dir(), &req.from_path)?;
        if !from_abs.exists() {
            return Err(PlatformError::new(
                "PLATFORM_DOC_MISSING",
                format!("doc entry '{}' not found", from_rel),
            ));
        }
        let (parent_rel, parent_abs) =
            resolve_doc_folder_path(&layout.repo_docs_dir(), &req.to_parent_path, true)?;
        if !parent_abs.exists() || !parent_abs.is_dir() {
            return Err(PlatformError::new(
                "PLATFORM_DOC_MOVE",
                format!("target folder '{}' not found", parent_rel),
            ));
        }
        if from_abs.is_dir() && parent_abs.starts_with(&from_abs) {
            return Err(PlatformError::new(
                "PLATFORM_DOC_MOVE",
                "cannot move a folder into itself",
            ));
        }
        let name = from_abs
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or_else(|| PlatformError::new("PLATFORM_DOC_MOVE", "invalid source filename"))?
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
                "PLATFORM_DOC_MOVE",
                format!("target '{}' already exists", to_rel),
            ));
        }
        fs::rename(&from_abs, &to_abs)?;
        Ok(to_rel)
    }

    /// Deletes one project doc file by path under `repo/docs`.
    pub fn delete_project_doc(
        &self,
        owner: &str,
        project: &str,
        rel_path: &str,
    ) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let (_rel, abs) = resolve_doc_path(&layout.repo_docs_dir(), rel_path)?;
        if !abs.is_file() {
            return Err(PlatformError::new(
                "PLATFORM_DOC_MISSING",
                format!("doc file '{}' not found", rel_path),
            ));
        }
        fs::remove_file(&abs).map_err(PlatformError::from)
    }

    /// Deletes one project docs file or folder by path under `repo/docs`.
    pub fn delete_project_doc_entry(
        &self,
        owner: &str,
        project: &str,
        rel_path: &str,
    ) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        let (_rel, abs) = resolve_doc_path(&layout.repo_docs_dir(), rel_path)?;
        if !abs.exists() {
            return Err(PlatformError::new(
                "PLATFORM_DOC_MISSING",
                format!("doc entry '{}' not found", rel_path),
            ));
        }
        if abs.is_dir() {
            fs::remove_dir_all(&abs)?;
        } else {
            fs::remove_file(&abs)?;
        }
        Ok(())
    }

    /// Lists the three agent doc files (AGENTS.md, SOUL.md, MEMORY.md), creating defaults if absent.
    pub fn list_agent_docs(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<AgentDocItem>, PlatformError> {
        self.ensure_agent_docs_defaults(owner, project)?;
        Ok(vec![
            AgentDocItem {
                name: "AGENTS.md".to_string(),
                user_editable: true,
            },
            AgentDocItem {
                name: "SOUL.md".to_string(),
                user_editable: true,
            },
            AgentDocItem {
                name: "MEMORY.md".to_string(),
                user_editable: false,
            },
        ])
    }

    /// Reads one agent doc file (AGENTS.md, SOUL.md, or MEMORY.md).
    ///
    /// MEMORY reads from its `store`-tier per-user home; an absent memory
    /// file reads as the default text without scaffolding a file — the store
    /// tier only gains a file once the assistant actually writes memory.
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
        let path = if safe_name == "MEMORY.md" {
            Self::assistant_memory_file(&layout, &owner)?
        } else {
            layout.data_cache_agent_docs_dir().join(safe_name)
        };
        if path.is_file() {
            fs::read_to_string(&path).map_err(PlatformError::from)
        } else if safe_name == "MEMORY.md" {
            Ok(MEMORY_MD_DEFAULT.to_string())
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
        let path = if safe_name == "MEMORY.md" {
            let path = Self::assistant_memory_file(&layout, &owner)?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            path
        } else {
            layout.data_cache_agent_docs_dir().join(safe_name)
        };
        fs::write(&path, content).map_err(PlatformError::from)
    }

    /// Resolves one user's assistant memory file at its `store`-tier home,
    /// migrating a pre-tier project-wide `data/cache/agent_docs/MEMORY.md`
    /// the first time it is touched. A file present at both paths is refused
    /// rather than guessed, per `migrate_tier_entry`.
    ///
    /// `{user_id}` is the owner slug: every memory writer today — the
    /// assistant chat tools and MCP `docs_agent_write` — is scoped by the
    /// project owner, with no finer acting-user identity threaded into the
    /// write path. The pre-move project-wide MEMORY.md therefore migrates to
    /// the project owner's file, the only honest attribution available.
    fn assistant_memory_file(
        layout: &crate::platform::model::ProjectFileLayout,
        user_id: &str,
    ) -> Result<PathBuf, PlatformError> {
        let target = layout.data_store_assistant_memory_file(user_id);
        crate::infra::io::durable::migrate_tier_entry(
            &layout.data_cache_agent_docs_dir().join("MEMORY.md"),
            &target,
        )
        .map_err(|err| PlatformError::new("PLATFORM_MEMORY_TIER_MIGRATE", err.to_string()))?;
        Ok(target)
    }

    /// Creates default agent doc files if they don't exist yet.
    ///
    /// MEMORY is deliberately absent here: its home is the `store` tier
    /// (`data/store/assistant/{user_id}/memory.md`), created lazily on first
    /// write. Scaffolding a MEMORY.md into `agent_docs/` would collide with
    /// the tier migration as a both-paths refusal.
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
        ];
        for (name, content) in defaults {
            let path = layout.data_cache_agent_docs_dir().join(name);
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
    resolve_doc_folder_path(root, rel_path, false)
}

fn resolve_doc_folder_path(
    root: &Path,
    rel_path: &str,
    allow_empty: bool,
) -> Result<(String, PathBuf), PlatformError> {
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
    if normalized.is_empty() && !allow_empty {
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
    let mut entries = fs::read_dir(current)?.collect::<Result<Vec<_>, _>>()?;
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

/// Recursively collect all files under `dir` as (rel_path, abs_path) pairs.
fn collect_all_files(root: &Path, dir: &Path, out: &mut Vec<(String, std::path::PathBuf)>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_all_files(root, &path, out);
        } else if let Ok(rel) = path.strip_prefix(root) {
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            out.push((rel_str, path));
        }
    }
}

/// Glob match against a pipeline identity. Exposed publicly for bulk operations.
///
/// The pattern goes through [`normalize_pipeline_glob`] first, so a pattern
/// that names the source root and one that does not select the same pipelines.
pub fn pipeline_glob_matches(layout: &ResolvedProjectLayout, pattern: &str, path: &str) -> bool {
    template_glob_matches(&normalize_pipeline_glob(layout, pattern), path)
}

pub fn template_glob_matches(pattern: &str, path: &str) -> bool {
    if pattern.is_empty() {
        return true;
    }
    // Split on `**` first
    let segments: Vec<&str> = pattern.split("**").collect();
    if segments.len() == 1 {
        // No **, use single-star match (no slash crossing)
        glob_star_match(pattern, path)
    } else {
        // Must match first segment at start, last at end, rest anywhere
        let mut remaining = path;
        for (i, seg) in segments.iter().enumerate() {
            if seg.is_empty() {
                continue;
            }
            if i == 0 {
                if !glob_star_match_prefix(seg, remaining) {
                    return false;
                }
                remaining = &remaining[seg.trim_matches('*').len().min(remaining.len())..];
            } else if i == segments.len() - 1 {
                if !remaining.ends_with(seg.trim_matches('*')) {
                    return false;
                }
            } else if let Some(pos) = remaining.find(seg.trim_matches('*')) {
                remaining = &remaining[pos + seg.trim_matches('*').len()..];
            } else {
                return false;
            }
        }
        true
    }
}

fn glob_star_match(pattern: &str, path: &str) -> bool {
    // Split pattern and path by '/', match each segment (single * = any non-slash chars, *.ext supported)
    let pat_parts: Vec<&str> = pattern.split('/').collect();
    let path_parts: Vec<&str> = path.split('/').collect();
    if pat_parts.len() != path_parts.len() {
        return false;
    }
    pat_parts
        .iter()
        .zip(path_parts.iter())
        .all(|(p, s)| glob_segment_matches(p, s))
}

fn glob_segment_matches(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if !pattern.contains('*') {
        return pattern == value;
    }
    // Handle patterns like "*.tsx", "foo*", "foo*.tsx"
    let mut remaining = value;
    let mut parts = pattern.splitn(2, '*');
    let prefix = parts.next().unwrap_or("");
    let suffix = parts.next().unwrap_or("");
    if !remaining.starts_with(prefix) {
        return false;
    }
    remaining = &remaining[prefix.len()..];
    if !suffix.is_empty() && !remaining.ends_with(suffix) {
        return false;
    }
    true
}

fn glob_star_match_prefix(pattern: &str, path: &str) -> bool {
    let trimmed = pattern.trim_end_matches('/').trim_end_matches('*');
    path.starts_with(trimmed)
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

/// `file_rel_path` with the project's source root removed.
///
/// Runtime snapshot names are built from what is left, so a path outside the
/// source root keeps its own shape rather than being rewritten into one.
fn strip_source_root<'a>(layout: &ProjectFileLayout, file_rel_path: &'a str) -> &'a str {
    layout
        .repo_layout
        .strip_source(file_rel_path)
        .unwrap_or(file_rel_path)
        .trim_start_matches('/')
}

/// One persisted pipeline id and what the migration converts it to.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PipelineIdentityRewrite {
    pub from: String,
    pub to: String,
}

/// What one run of the pipeline identity migration did.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PipelineIdentityMigration {
    pub rewrites: Vec<PipelineIdentityRewrite>,
    /// Rows that already named a pipeline the way identity does now.
    pub already_canonical: usize,
    /// The rewritten `spec.bootstrap.activate` list, empty when unchanged.
    pub bootstrap_activate: Vec<String>,
    /// Absent when there was nothing to convert, so nothing was written.
    pub recovery_path: Option<PathBuf>,
}

/// `raw` with the pre-source-relative root removed, if it carried one.
fn strip_legacy_identity_root(raw: &str) -> String {
    let cleaned = raw.trim().replace('\\', "/");
    let rooted = cleaned.trim_start_matches('/');
    rooted
        .strip_prefix(LEGACY_PIPELINE_IDENTITY_ROOT)
        .and_then(|rest| rest.strip_prefix('/'))
        .unwrap_or(rooted)
        .to_string()
}

/// One stored pipeline row, read through the identity rule.
///
/// Rows written before identity became source-relative still carry the source
/// root. Reading them through the rule is what lets an existing project keep
/// working with no user action; the migration only rewrites the durable bytes
/// so they stop needing this.
fn adopt_pipeline_meta(layout: &ResolvedProjectLayout, mut meta: PipelineMeta) -> PipelineMeta {
    meta.file_rel_path = normalize_pipeline_file_rel_path(layout, &meta.file_rel_path);
    meta.virtual_path = virtual_path_from_file_rel_path(&meta.file_rel_path);
    meta
}

/// Normalizes a pipeline `file_rel_path` into its canonical identity.
///
/// Identity is source-relative: it is the path *inside* the project's source
/// root, and the root itself lives only in `zebflow.yaml`. A project that
/// moves its source therefore keeps every pipeline id it already had.
///
/// A caller may hand in a bare name, an already-canonical id, or a path that
/// still carries the source root; all three name the same pipeline. Accepting
/// the rooted form is what lets a project written before this change keep
/// working without being migrated first, and is why the DSL, MCP, and API can
/// all keep spelling a pipeline `pipelines/api/foo` on a default layout.
pub fn normalize_pipeline_file_rel_path(layout: &ResolvedProjectLayout, raw: &str) -> String {
    let cleaned = raw.trim().replace('\\', "/");
    let rooted = cleaned.trim_start_matches('/');
    let rel = layout.strip_source(rooted).unwrap_or(rooted);
    if rel.ends_with(PIPELINE_DEFINITION_EXTENSION) {
        return rel.to_string();
    }
    // A plain `.json` tail used to be left alone, which minted an identity the
    // `.zf.json` discovery rule then refused to see. The extension is replaced
    // rather than appended so identity and discovery cannot disagree.
    let stem = rel.strip_suffix(".json").unwrap_or(rel);
    format!("{stem}{PIPELINE_DEFINITION_EXTENSION}")
}

/// Normalizes a glob that selects pipelines by identity.
///
/// Patterns are written by people and stored in `spec.bootstrap.activate`, so
/// the same tolerance identity has applies here: a pattern that still names
/// the source root selects the same pipelines as one that does not.
pub fn normalize_pipeline_glob(layout: &ResolvedProjectLayout, pattern: &str) -> String {
    let cleaned = pattern.trim().replace('\\', "/");
    let rooted = cleaned.trim_start_matches('/');
    layout.strip_source(rooted).unwrap_or(rooted).to_string()
}

/// Derives the virtual_path from a source-relative `file_rel_path`.
/// `"api/foo.zf.json"` -> `"/api"`
/// `"foo.zf.json"`     -> `"/"`
pub fn virtual_path_from_file_rel_path(file_rel_path: &str) -> String {
    let stripped = file_rel_path.trim_start_matches('/');
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
    format!("{:x}", Sha256::digest(input.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::adapters::data::build_data_adapter;
    use crate::platform::adapters::file::{FileAdapter, FilesystemFileAdapter};
    use crate::platform::adapters::project_data::build_project_data_factory;
    use crate::platform::model::{
        CreateProjectRequest, DataAdapterKind, PlatformUser, PlatformUserLocalAuth,
        ProjectRuntimeSelectionRequest, StoredUser,
    };
    use crate::platform::services::dependency_lock::DependencyLockService;
    use crate::platform::services::project_config::ProjectConfigurationService;
    use std::sync::Arc;

    fn make_service(root: &Path) -> ProjectService {
        let data = build_data_adapter(DataAdapterKind::Sqlite, root).expect("sqlite adapter");
        let configs = Arc::new(ProjectConfigurationService::new(root.join("users")));
        let file = Arc::new(FilesystemFileAdapter::new(
            root.join("users"),
            Arc::clone(&configs),
        ));
        file.initialize().expect("file adapter init");
        let now = now_ts();
        let seeded_user_id = "usr_test_superadmin".to_string();
        data.put_user(&StoredUser {
            profile: PlatformUser {
                user_id: seeded_user_id.clone(),
                owner: "superadmin".to_string(),
                role: "superadmin".to_string(),
                git_name: "Superadmin".to_string(),
                git_email: String::new(),
                created_at: now,
                updated_at: now,
            },
            auth: PlatformUserLocalAuth {
                user_id: seeded_user_id,
                password_hash: String::new(),
                password_alg: "sha256".to_string(),
                password_updated_at: now,
                credential_state: crate::platform::model::CREDENTIAL_STATE_CHOSEN.to_string(),
            },
        })
        .expect("seed test owner user");
        let project_data = build_project_data_factory(root);
        let dependency_lock = Arc::new(DependencyLockService::new(root.join("users")));
        ProjectService::new(data, file, project_data, configs, dependency_lock)
    }

    fn create_default_project(svc: &ProjectService) {
        svc.create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "default".to_string(),
                title: Some("Default".to_string()),
                local_branch: None,
                runtime: ProjectRuntimeSelectionRequest::default(),
            },
        )
        .expect("create project");
    }

    fn invalid_logic_match_router_source() -> &'static str {
        r#"{
  "apiVersion":"zebflow.com/v1",
  "kind":"Pipeline",
  "metadata":{"name":"invalid-router"},
  "spec":{
  "id":"invalid-router",
  "entry_nodes":["trigger_webhook"],
  "nodes":[
    {"id":"trigger_webhook","kind":"n.trigger.webhook","input_pins":[],"output_pins":["out"],"config":{"path":"/router","method":"POST"}},
    {"id":"kind_route","kind":"n.logic.match","input_pins":["in"],"output_pins":["csv","geojson","archive","default"],"config":{"expression":"$input.input_kind","cases":["csv","geojson","archive"],"default":"default"}},
    {"id":"csv_branch","kind":"n.web.response","input_pins":["in"],"output_pins":["out"],"config":{}}
  ],
  "edges":[
    {"from_node":"trigger_webhook","from_pin":"out","to_node":"kind_route","to_pin":"in"},
    {"from_node":"kind_route","from_pin":"out","to_node":"csv_branch","to_pin":"in"}
  ]}
}"#
    }

    fn valid_logic_match_router_source() -> &'static str {
        r#"{
  "apiVersion":"zebflow.com/v1",
  "kind":"Pipeline",
  "metadata":{"name":"valid-router"},
  "spec":{
  "id":"valid-router",
  "entry_nodes":["trigger_webhook"],
  "nodes":[
    {"id":"trigger_webhook","kind":"n.trigger.webhook","input_pins":[],"output_pins":["out"],"config":{"path":"/router","method":"POST"}},
    {"id":"kind_route","kind":"n.logic.match","input_pins":["in"],"output_pins":["csv","geojson","archive","default"],"config":{"expression":"$input.input_kind","cases":["csv","geojson","archive"],"default":"default"}},
    {"id":"csv_branch","kind":"n.web.response","input_pins":["in"],"output_pins":["out"],"config":{}}
  ],
  "edges":[
    {"from_node":"trigger_webhook","from_pin":"out","to_node":"kind_route","to_pin":"in"},
    {"from_node":"kind_route","from_pin":"csv","to_node":"csv_branch","to_pin":"in"}
  ]}
}"#
    }

    #[test]
    fn upsert_pipeline_rejects_invalid_logic_match_edge_pin() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let svc = make_service(tmp.path());
        create_default_project(&svc);

        let err = svc
            .upsert_pipeline_definition(
                "superadmin",
                "default",
                "pipelines/api/router.zf.json",
                "Router",
                "",
                "webhook",
                invalid_logic_match_router_source(),
            )
            .expect_err("invalid graph should be rejected before save");

        assert_eq!(err.code, "FW_EDGE_FROM_PIN");
        assert!(err.message.contains("kind_route"));
        assert!(err.message.contains("'out'"));
    }

    #[test]
    fn activate_pipeline_rejects_invalid_saved_logic_match_edge_pin() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let svc = make_service(tmp.path());
        create_default_project(&svc);

        let file_rel_path = "pipelines/api/router.zf.json";
        svc.upsert_pipeline_definition(
            "superadmin",
            "default",
            file_rel_path,
            "Router",
            "",
            "webhook",
            valid_logic_match_router_source(),
        )
        .expect("valid graph should save");

        let layout = svc
            .file
            .ensure_project_layout("superadmin", "default")
            .expect("layout");
        let abs = layout.repo_dir.join(file_rel_path);
        std::fs::write(abs, invalid_logic_match_router_source()).expect("write invalid source");

        let err = svc
            .activate_pipeline_definition("superadmin", "default", file_rel_path)
            .expect_err("invalid saved graph should be rejected before activation");

        assert_eq!(err.code, "FW_EDGE_FROM_PIN");
        assert!(err.message.contains("kind_route"));
        assert!(err.message.contains("'out'"));
    }

    #[test]
    fn activate_replaces_old_runtime_snapshot_for_same_pipeline_file() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let svc = make_service(tmp.path());
        svc.create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "default".to_string(),
                title: Some("Default".to_string()),
                local_branch: None,
                runtime: ProjectRuntimeSelectionRequest::default(),
            },
        )
        .expect("create project");

        let file_rel_path = "pipelines/pages/home.zf.json";
        let source_a = r#"{
  "apiVersion":"zebflow.com/v1",
  "kind":"Pipeline",
  "metadata":{"name":"pipeline-canvas"},
  "spec":{
  "id":"pipeline-canvas",
  "entry_nodes":["trigger_webhook"],
  "nodes":[
    {"id":"trigger_webhook","kind":"n.trigger.webhook","input_pins":[],"output_pins":["out"],"config":{"path":"/a"}},
    {"id":"web-response","kind":"n.web.response","input_pins":["in"],"output_pins":["out"],"config":{"template":"pages/home/home.tsx"}}
  ],
  "edges":[{"from_node":"trigger_webhook","from_pin":"out","to_node":"web-response","to_pin":"in"}]}
}"#;
        let source_b = source_a.replace(r#""/a""#, r#""/b""#);

        svc.upsert_pipeline_definition(
            "superadmin",
            "default",
            file_rel_path,
            "Home",
            "",
            "webhook",
            source_a,
        )
        .expect("upsert a");
        let meta_a = svc
            .activate_pipeline_definition("superadmin", "default", file_rel_path)
            .expect("activate a");

        svc.upsert_pipeline_definition(
            "superadmin",
            "default",
            file_rel_path,
            "Home",
            "",
            "webhook",
            &source_b,
        )
        .expect("upsert b");
        let meta_b = svc
            .activate_pipeline_definition("superadmin", "default", file_rel_path)
            .expect("activate b");

        let layout = svc
            .file
            .ensure_project_layout("superadmin", "default")
            .expect("layout");
        let runtime_pages = layout.data_cache_pipelines_dir().join("pages");
        let files = std::fs::read_dir(&runtime_pages)
            .expect("runtime pages dir")
            .map(|entry| {
                entry
                    .expect("dir entry")
                    .file_name()
                    .to_string_lossy()
                    .to_string()
            })
            .filter(|name| name.starts_with("home."))
            .collect::<Vec<_>>();

        assert_eq!(
            files.len(),
            1,
            "expected exactly one runtime snapshot, got {files:?}"
        );
        assert!(
            files[0].contains(meta_b.active_hash.as_deref().unwrap_or("")),
            "remaining snapshot should be the newest active hash"
        );
        assert!(
            !files[0].contains(meta_a.active_hash.as_deref().unwrap_or("")),
            "old snapshot hash should not remain"
        );
    }

    #[test]
    fn failed_pipeline_metadata_write_restores_previous_source() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let svc = make_service(tmp.path());
        create_default_project(&svc);
        let file_rel_path = "pipelines/api/router.zf.json";

        svc.upsert_pipeline_definition(
            "superadmin",
            "default",
            file_rel_path,
            "Router",
            "",
            "webhook",
            valid_logic_match_router_source(),
        )
        .expect("initial save");
        let previous = svc
            .read_pipeline_source("superadmin", "default", file_rel_path)
            .expect("previous source");

        svc.fail_next_pipeline_meta_write
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let changed = valid_logic_match_router_source().replace("/router", "/changed");
        let error = svc
            .upsert_pipeline_definition(
                "superadmin",
                "default",
                file_rel_path,
                "Router",
                "",
                "webhook",
                &changed,
            )
            .expect_err("metadata failure");
        assert_eq!(error.code, "PLATFORM_PIPELINE_META_INJECTED");
        assert_eq!(
            svc.read_pipeline_source("superadmin", "default", file_rel_path)
                .expect("restored source"),
            previous
        );
    }

    #[test]
    fn failed_activation_commit_keeps_previous_active_snapshot() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let svc = make_service(tmp.path());
        create_default_project(&svc);
        let file_rel_path = "pipelines/api/router.zf.json";

        svc.upsert_pipeline_definition(
            "superadmin",
            "default",
            file_rel_path,
            "Router",
            "",
            "webhook",
            valid_logic_match_router_source(),
        )
        .expect("initial save");
        let active = svc
            .activate_pipeline_definition("superadmin", "default", file_rel_path)
            .expect("initial activation");
        let old_hash = active.active_hash.clone().expect("old active hash");

        let changed = valid_logic_match_router_source().replace("/router", "/changed");
        svc.upsert_pipeline_definition(
            "superadmin",
            "default",
            file_rel_path,
            "Router",
            "",
            "webhook",
            &changed,
        )
        .expect("changed draft");
        let changed_source = svc
            .read_pipeline_source("superadmin", "default", file_rel_path)
            .expect("changed canonical source");
        let candidate_hash = stable_hash_hex(&changed_source);

        svc.fail_next_pipeline_meta_write
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let error = svc
            .activate_pipeline_definition("superadmin", "default", file_rel_path)
            .expect_err("activation metadata failure");
        assert_eq!(error.code, "PLATFORM_PIPELINE_META_INJECTED");

        let current = svc
            .get_pipeline_meta_by_file_id("superadmin", "default", file_rel_path)
            .expect("metadata read")
            .expect("metadata");
        assert_eq!(current.active_hash.as_deref(), Some(old_hash.as_str()));

        let layout = svc
            .file
            .ensure_project_layout("superadmin", "default")
            .expect("layout");
        let old_snapshot = svc
            .runtime_pipeline_snapshot_path(&layout, file_rel_path, &old_hash)
            .expect("old snapshot");
        let candidate_snapshot = svc
            .runtime_pipeline_snapshot_path(&layout, file_rel_path, &candidate_hash)
            .expect("candidate snapshot");
        assert!(old_snapshot.is_file());
        assert!(!candidate_snapshot.exists());
    }

    #[test]
    fn activation_preflight_rejects_missing_required_node_config_before_snapshot_write() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let svc = make_service(tmp.path());
        create_default_project(&svc);
        let file_rel_path = "pipelines/functions/invalid-query.zf.json";
        let source = r#"{
          "apiVersion":"zebflow.com/v1",
          "kind":"Pipeline",
          "metadata":{"name":"invalid-query"},
          "spec":{
            "id":"invalid-query",
            "entry_nodes":["function"],
            "nodes":[
              {"id":"function","kind":"n.trigger.function","input_pins":[],"output_pins":["out"],"config":{}}
            ],
            "edges":[]
          }
        }"#;
        svc.upsert_pipeline_definition(
            "superadmin",
            "default",
            file_rel_path,
            "Invalid query",
            "",
            "manual",
            source,
        )
        .expect("structurally valid draft");

        let error = svc
            .activate_pipeline_definition("superadmin", "default", file_rel_path)
            .expect_err("invalid node config must fail preflight");
        assert_eq!(error.code, "PIPELINE_NODE_CONFIG_VIOLATION");

        let meta = svc
            .get_pipeline_meta_by_file_id("superadmin", "default", file_rel_path)
            .expect("metadata read")
            .expect("metadata");
        assert!(meta.active_hash.is_none());
        let layout = svc
            .file
            .ensure_project_layout("superadmin", "default")
            .expect("layout");
        assert!(!layout.data_cache_pipelines_dir().join("functions").exists());
    }

    #[test]
    fn deactivate_removes_runtime_snapshot_for_pipeline_file() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let svc = make_service(tmp.path());
        svc.create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "default".to_string(),
                title: Some("Default".to_string()),
                local_branch: None,
                runtime: ProjectRuntimeSelectionRequest::default(),
            },
        )
        .expect("create project");

        let file_rel_path = "pipelines/pages/home.zf.json";
        let source = r#"{
  "apiVersion":"zebflow.com/v1",
  "kind":"Pipeline",
  "metadata":{"name":"pipeline-canvas"},
  "spec":{
  "id":"pipeline-canvas",
  "entry_nodes":["trigger_webhook"],
  "nodes":[
    {"id":"trigger_webhook","kind":"n.trigger.webhook","input_pins":[],"output_pins":["out"],"config":{"path":"/a"}},
    {"id":"web-response","kind":"n.web.response","input_pins":["in"],"output_pins":["out"],"config":{"template":"pages/home/home.tsx"}}
  ],
  "edges":[{"from_node":"trigger_webhook","from_pin":"out","to_node":"web-response","to_pin":"in"}]}
}"#;

        svc.upsert_pipeline_definition(
            "superadmin",
            "default",
            file_rel_path,
            "Home",
            "",
            "webhook",
            source,
        )
        .expect("upsert");
        let meta = svc
            .activate_pipeline_definition("superadmin", "default", file_rel_path)
            .expect("activate");
        svc.deactivate_pipeline_definition("superadmin", "default", file_rel_path)
            .expect("deactivate");

        let layout = svc
            .file
            .ensure_project_layout("superadmin", "default")
            .expect("layout");
        let active_path = svc
            .runtime_pipeline_snapshot_path(
                &layout,
                file_rel_path,
                meta.active_hash.as_deref().unwrap_or_default(),
            )
            .expect("snapshot path");
        assert!(
            !active_path.exists(),
            "runtime snapshot should be removed on deactivate"
        );
    }

    #[test]
    fn webhook_conflict_checks_all_saved_pipeline_definitions() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let svc = make_service(tmp.path());
        svc.create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "default".to_string(),
                title: Some("Default".to_string()),
                local_branch: None,
                runtime: ProjectRuntimeSelectionRequest::default(),
            },
        )
        .expect("create project");

        let source = r#"{
  "apiVersion":"zebflow.com/v1",
  "kind":"Pipeline",
  "metadata":{"name":"pipeline-canvas"},
  "spec":{
  "id":"pipeline-canvas",
  "entry_nodes":["trigger_webhook"],
  "nodes":[
    {"id":"trigger_webhook","kind":"n.trigger.webhook","input_pins":[],"output_pins":["out"],"config":{"path":"/same","method":"GET"}},
    {"id":"web-response","kind":"n.web.response","input_pins":["in"],"output_pins":["out"],"config":{"template":"pages/home/home.tsx"}}
  ],
  "edges":[{"from_node":"trigger_webhook","from_pin":"out","to_node":"web-response","to_pin":"in"}]}
}"#;

        svc.upsert_pipeline_definition(
            "superadmin",
            "default",
            "pipelines/pages/a.zf.json",
            "A",
            "",
            "webhook",
            source,
        )
        .expect("save first");

        let graph = decode_pipeline_graph(source.as_bytes())
            .expect("parse graph")
            .spec;
        let conflicts = svc
            .check_webhook_path_conflict(
                "superadmin",
                "default",
                &graph,
                "pipelines/pages/b.zf.json",
            )
            .expect("check conflict");
        assert_eq!(conflicts.len(), 1);
        // Identity is source-relative: the root lives in the layout, not the id.
        assert_eq!(conflicts[0].file_rel_path, "pages/a.zf.json");
        assert_eq!(conflicts[0].method, "GET");
        assert_eq!(conflicts[0].path, "/same");
    }

    #[test]
    fn upsert_rejects_duplicate_webhook_path_from_saved_pipeline() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let svc = make_service(tmp.path());
        svc.create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "default".to_string(),
                title: Some("Default".to_string()),
                local_branch: None,
                runtime: ProjectRuntimeSelectionRequest::default(),
            },
        )
        .expect("create project");

        let source = r#"{
  "apiVersion":"zebflow.com/v1",
  "kind":"Pipeline",
  "metadata":{"name":"pipeline-canvas"},
  "spec":{
  "id":"pipeline-canvas",
  "entry_nodes":["trigger_webhook"],
  "nodes":[
    {"id":"trigger_webhook","kind":"n.trigger.webhook","input_pins":[],"output_pins":["out"],"config":{"path":"/same","method":"POST"}},
    {"id":"web-response","kind":"n.web.response","input_pins":["in"],"output_pins":["out"],"config":{"template":"pages/home/home.tsx"}}
  ],
  "edges":[{"from_node":"trigger_webhook","from_pin":"out","to_node":"web-response","to_pin":"in"}]}
}"#;

        svc.upsert_pipeline_definition(
            "superadmin",
            "default",
            "pipelines/pages/a.zf.json",
            "A",
            "",
            "webhook",
            source,
        )
        .expect("save first");
        let err = svc
            .upsert_pipeline_definition(
                "superadmin",
                "default",
                "pipelines/pages/b.zf.json",
                "B",
                "",
                "webhook",
                source,
            )
            .expect_err("second save should conflict");
        assert_eq!(err.code, "PLATFORM_PIPELINE_WEBHOOK_CONFLICT");
        assert!(err.message.contains("POST /same"));
    }

    #[test]
    fn activate_rejects_duplicate_webhook_path_from_legacy_saved_pipeline() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let svc = make_service(tmp.path());
        svc.create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "default".to_string(),
                title: Some("Default".to_string()),
                local_branch: None,
                runtime: ProjectRuntimeSelectionRequest::default(),
            },
        )
        .expect("create project");

        let source = r#"{
  "apiVersion":"zebflow.com/v1",
  "kind":"Pipeline",
  "metadata":{"name":"pipeline-canvas"},
  "spec":{
  "id":"pipeline-canvas",
  "entry_nodes":["trigger_webhook"],
  "nodes":[
    {"id":"trigger_webhook","kind":"n.trigger.webhook","input_pins":[],"output_pins":["out"],"config":{"path":"/same","method":"POST"}},
    {"id":"web-response","kind":"n.web.response","input_pins":["in"],"output_pins":["out"],"config":{"template":"pages/home/home.tsx"}}
  ],
  "edges":[{"from_node":"trigger_webhook","from_pin":"out","to_node":"web-response","to_pin":"in"}]}
}"#;

        let meta_a = svc
            .upsert_pipeline_definition(
                "superadmin",
                "default",
                "pipelines/pages/a.zf.json",
                "A",
                "",
                "webhook",
                source,
            )
            .expect("save first");

        let layout = svc
            .file
            .ensure_project_layout("superadmin", "default")
            .expect("layout");
        let legacy_file = "pipelines/pages/b.zf.json";
        let legacy_abs = svc
            .pipeline_abs_path(&layout, legacy_file)
            .expect("legacy abs");
        if let Some(parent) = legacy_abs.parent() {
            std::fs::create_dir_all(parent).expect("mkdirs");
        }
        std::fs::write(&legacy_abs, source).expect("write legacy source");

        let now = now_ts();
        svc.data
            .put_pipeline_meta(&PipelineMeta {
                owner: "superadmin".to_string(),
                project: "default".to_string(),
                name: "b".to_string(),
                title: "B".to_string(),
                virtual_path: virtual_path_from_file_rel_path(legacy_file),
                file_rel_path: legacy_file.to_string(),
                description: "".to_string(),
                trigger_kind: "webhook".to_string(),
                hash: stable_hash_hex(source),
                active_hash: None,
                created_at: now,
                updated_at: now,
                activated_at: None,
            })
            .expect("insert legacy meta");

        let err = svc
            .activate_pipeline_definition("superadmin", "default", &meta_a.file_rel_path)
            .expect_err("activate should conflict against legacy saved duplicate");
        assert_eq!(err.code, "PLATFORM_PIPELINE_WEBHOOK_CONFLICT");
        assert!(err.message.contains("POST /same"));
    }

    #[test]
    fn locked_pipeline_rejects_edit_activate_and_delete() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let svc = make_service(tmp.path());
        svc.create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "default".to_string(),
                title: Some("Default".to_string()),
                local_branch: None,
                runtime: ProjectRuntimeSelectionRequest::default(),
            },
        )
        .expect("create project");

        let file_rel_path = "pipelines/pages/home.zf.json";
        let locked_source = r#"{
  "apiVersion":"zebflow.com/v1",
  "kind":"Pipeline",
  "metadata":{"name":"pipeline-canvas"},
  "spec":{
  "metadata":{"locked":true},
  "id":"pipeline-canvas",
  "entry_nodes":["trigger_webhook"],
  "nodes":[
    {"id":"trigger_webhook","kind":"n.trigger.webhook","input_pins":[],"output_pins":["out"],"config":{"path":"/locked"}},
    {"id":"web-response","kind":"n.web.response","input_pins":["in"],"output_pins":["out"],"config":{"template":"pages/home/home.tsx"}}
  ],
  "edges":[{"from_node":"trigger_webhook","from_pin":"out","to_node":"web-response","to_pin":"in"}]}
}"#;

        svc.upsert_pipeline_definition(
            "superadmin",
            "default",
            file_rel_path,
            "Home",
            "",
            "webhook",
            locked_source,
        )
        .expect("initial locked save");

        let err = svc
            .upsert_pipeline_definition(
                "superadmin",
                "default",
                file_rel_path,
                "Home",
                "",
                "webhook",
                &locked_source.replace("/locked", "/changed"),
            )
            .expect_err("locked pipeline edit should fail");
        assert_eq!(err.code, "PLATFORM_PIPELINE_LOCKED");

        let err = svc
            .activate_pipeline_definition("superadmin", "default", file_rel_path)
            .expect_err("locked pipeline activate should fail");
        assert_eq!(err.code, "PLATFORM_PIPELINE_LOCKED");

        let err = svc
            .delete_pipeline("superadmin", "default", file_rel_path)
            .expect_err("locked pipeline delete should fail");
        assert_eq!(err.code, "PLATFORM_PIPELINE_LOCKED");
    }

    #[test]
    fn locked_template_rejects_write_create_move_delete_and_edit() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let svc = make_service(tmp.path());
        svc.create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "default".to_string(),
                title: Some("Default".to_string()),
                local_branch: None,
                runtime: ProjectRuntimeSelectionRequest::default(),
            },
        )
        .expect("create project");

        let req = TemplateCreateRequest {
            kind: TemplateCreateKind::Page,
            name: "locked-page".to_string(),
            parent_rel_path: Some("pages".to_string()),
        };
        let payload = svc
            .create_template_entry("superadmin", "default", &req)
            .expect("create template");

        svc.zebflow_cfg
            .set_template_locked("superadmin", "default", &payload.rel_path, true)
            .expect("lock template");

        let err = svc
            .write_template_file(
                "superadmin",
                "default",
                &TemplateSaveRequest {
                    rel_path: payload.rel_path.clone(),
                    content: "changed".to_string(),
                },
            )
            .expect_err("locked template write should fail");
        assert_eq!(err.code, "PLATFORM_TEMPLATE_LOCKED");

        let err = svc
            .edit_template_file(
                "superadmin",
                "default",
                &payload.rel_path,
                "export",
                "import",
            )
            .expect_err("locked template edit should fail");
        assert_eq!(err.code, "PLATFORM_TEMPLATE_LOCKED");

        let err = svc
            .move_template_entry(
                "superadmin",
                "default",
                &TemplateMoveRequest {
                    from_rel_path: payload.rel_path.clone(),
                    to_parent_rel_path: "components".to_string(),
                },
            )
            .expect_err("locked template move should fail");
        assert_eq!(err.code, "PLATFORM_TEMPLATE_LOCKED");

        let err = svc
            .delete_template_entry("superadmin", "default", &payload.rel_path)
            .expect_err("locked template delete should fail");
        assert_eq!(err.code, "PLATFORM_TEMPLATE_LOCKED");

        svc.zebflow_cfg
            .set_template_locked("superadmin", "default", "pages", true)
            .expect("lock pages folder");
        let err = svc
            .create_template_entry(
                "superadmin",
                "default",
                &TemplateCreateRequest {
                    kind: TemplateCreateKind::Page,
                    name: "blocked-child".to_string(),
                    parent_rel_path: Some("pages".to_string()),
                },
            )
            .expect_err("create inside locked folder should fail");
        assert_eq!(err.code, "PLATFORM_TEMPLATE_LOCKED");
    }

    #[test]
    fn pipeline_source_rejects_missing_and_future_contract_versions() {
        let missing = r#"{
            "kind":"Pipeline",
            "metadata":{"name":"missing-version"},
            "spec":{"id":"missing-version","entry_nodes":[],"nodes":[],"edges":[]}
        }"#;
        let err = parse_and_validate_pipeline_source_for_save(missing).unwrap_err();
        assert_eq!(err.code, "PLATFORM_PIPELINE_PARSE");
        assert!(
            err.message
                .contains("missing required root field 'apiVersion'")
        );

        let future = r#"{
            "apiVersion":"zebflow.com/v2",
            "kind":"Pipeline",
            "metadata":{"name":"future-version"},
            "spec":{"id":"future-version","entry_nodes":[],"nodes":[],"edges":[]}
        }"#;
        let err = parse_and_validate_pipeline_source_for_save(future).unwrap_err();
        assert_eq!(err.code, "PLATFORM_PIPELINE_PARSE");
        assert!(err.message.contains("unsupported apiVersion"));
    }

    fn webhook_pipeline_source(id: &str, path: &str) -> String {
        format!(
            r#"{{
  "apiVersion":"zebflow.com/v1",
  "kind":"Pipeline",
  "metadata":{{"name":"{id}"}},
  "spec":{{
  "id":"{id}",
  "entry_nodes":["wh"],
  "nodes":[{{"id":"wh","kind":"n.trigger.webhook","input_pins":[],"output_pins":["out"],"config":{{"path":"{path}","method":"POST"}}}}],
  "edges":[]}}
}}"#
        )
    }

    #[test]
    fn a_project_that_declares_nothing_keeps_the_directories_it_already_had() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let svc = make_service(tmp.path());
        create_default_project(&svc);

        let layout = svc.project_layout("superadmin", "default").expect("layout");
        assert_eq!(layout.repo_layout.source, "pipelines");
        assert_eq!(layout.repo_layout.assets, "pipelines/assets");
        assert_eq!(layout.repo_source_dir(), layout.repo_dir.join("pipelines"));

        let meta = svc
            .upsert_pipeline_definition(
                "superadmin",
                "default",
                "api/router",
                "Router",
                "",
                "webhook",
                &webhook_pipeline_source("router", "/router"),
            )
            .expect("save");
        assert_eq!(meta.file_rel_path, "api/router.zf.json");
        assert!(
            layout
                .repo_dir
                .join("pipelines/api/router.zf.json")
                .is_file()
        );
    }

    #[test]
    fn a_declared_source_root_moves_registration_discovery_and_the_template_root() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let svc = make_service(tmp.path());
        create_default_project(&svc);
        svc.zebflow_cfg
            .update("superadmin", "default", |cfg| {
                cfg.configs.layout.source = Some("src/app".to_string());
            })
            .expect("declare layout");

        let layout = svc.project_layout("superadmin", "default").expect("layout");
        assert_eq!(layout.repo_source_dir(), layout.repo_dir.join("src/app"));
        // Assets follow the source they default inside, rather than staying in
        // the tree the project moved out of.
        assert_eq!(
            layout.repo_assets_dir(),
            layout.repo_dir.join("src/app/assets")
        );
        assert_eq!(
            svc.get_project_template_root("superadmin", "default")
                .expect("template root"),
            layout.repo_dir.join("src/app")
        );

        let meta = svc
            .upsert_pipeline_definition(
                "superadmin",
                "default",
                "blog/feed",
                "Feed",
                "",
                "webhook",
                &webhook_pipeline_source("feed", "/feed"),
            )
            .expect("save");
        // Identity carries no root segment, so it is the same string this
        // pipeline would have under any other declared source.
        assert_eq!(meta.file_rel_path, "blog/feed.zf.json");
        assert!(layout.repo_dir.join("src/app/blog/feed.zf.json").is_file());
        assert!(!layout.repo_dir.join("pipelines/blog/feed.zf.json").exists());

        let rows = svc
            .list_pipeline_meta_rows("superadmin", "default")
            .expect("rows");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].file_rel_path, "blog/feed.zf.json");
        assert_eq!(rows[0].virtual_path, "/blog");
        assert!(
            svc.read_pipeline_source("superadmin", "default", "blog/feed.zf.json")
                .expect("read")
                .contains("/feed")
        );
        svc.activate_pipeline_definition("superadmin", "default", "blog/feed.zf.json")
            .expect("activate");
    }

    #[test]
    fn migrating_pipeline_identity_converts_stored_ids_once() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let svc = make_service(tmp.path());
        create_default_project(&svc);

        let layout = svc.project_layout("superadmin", "default").expect("layout");
        let source = webhook_pipeline_source("legacy", "/legacy");
        let legacy_id = "pipelines/api/legacy.zf.json";
        let abs = svc
            .pipeline_abs_path(&layout, legacy_id)
            .expect("legacy abs");
        std::fs::create_dir_all(abs.parent().expect("parent")).expect("mkdirs");
        std::fs::write(&abs, &source).expect("write legacy source");
        let now = now_ts();
        svc.data
            .put_pipeline_meta(&PipelineMeta {
                owner: "superadmin".to_string(),
                project: "default".to_string(),
                name: "legacy".to_string(),
                title: "Legacy".to_string(),
                virtual_path: "/pipelines/api".to_string(),
                file_rel_path: legacy_id.to_string(),
                description: String::new(),
                trigger_kind: "webhook".to_string(),
                hash: stable_hash_hex(&source),
                active_hash: None,
                created_at: now,
                updated_at: now,
                activated_at: None,
            })
            .expect("insert legacy meta");
        svc.zebflow_cfg
            .set_bootstrap(
                "superadmin",
                "default",
                crate::platform::model::ProjectBootstrapPlan {
                    activate: vec!["pipelines/api/*.zf.json".to_string()],
                },
            )
            .expect("bootstrap");

        // Before migrating, the stored row already reads as the new identity:
        // an existing project keeps working with no user action.
        let rows = svc
            .list_pipeline_meta_rows("superadmin", "default")
            .expect("rows");
        assert_eq!(rows[0].file_rel_path, "api/legacy.zf.json");
        assert_eq!(
            svc.data
                .list_pipeline_meta("superadmin", "default")
                .expect("raw rows")[0]
                .file_rel_path,
            legacy_id,
            "the stored bytes are untouched until the migration runs"
        );

        let report = svc
            .migrate_pipeline_identity("superadmin", "default")
            .expect("migrate");
        assert_eq!(report.rewrites.len(), 1);
        assert_eq!(report.rewrites[0].from, legacy_id);
        assert_eq!(report.rewrites[0].to, "api/legacy.zf.json");
        assert_eq!(report.bootstrap_activate, vec!["api/*.zf.json".to_string()]);
        assert!(report.recovery_path.expect("recovery path").is_file());
        let stored = svc
            .data
            .list_pipeline_meta("superadmin", "default")
            .expect("raw rows");
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].file_rel_path, "api/legacy.zf.json");
        assert_eq!(stored[0].virtual_path, "/api");

        let again = svc
            .migrate_pipeline_identity("superadmin", "default")
            .expect("migrate again");
        assert!(again.rewrites.is_empty());
        assert_eq!(again.already_canonical, 1);
        assert!(
            again.recovery_path.is_none(),
            "a second run has nothing to convert and writes nothing"
        );
    }

    #[test]
    fn migrating_refuses_two_ids_that_would_become_one() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let svc = make_service(tmp.path());
        create_default_project(&svc);
        let source = webhook_pipeline_source("dup", "/dup");
        let now = now_ts();
        for id in ["pipelines/api/dup.zf.json", "api/dup.zf.json"] {
            svc.data
                .put_pipeline_meta(&PipelineMeta {
                    owner: "superadmin".to_string(),
                    project: "default".to_string(),
                    name: "dup".to_string(),
                    title: "Dup".to_string(),
                    virtual_path: "/api".to_string(),
                    file_rel_path: id.to_string(),
                    description: String::new(),
                    trigger_kind: "webhook".to_string(),
                    hash: stable_hash_hex(&source),
                    active_hash: None,
                    created_at: now,
                    updated_at: now,
                    activated_at: None,
                })
                .expect("insert row");
        }
        let err = svc
            .migrate_pipeline_identity("superadmin", "default")
            .expect_err("collision must refuse");
        assert_eq!(err.code, "PIPELINE_IDENTITY_MIGRATE");
        assert!(err.message.contains("both become"));
        let recovery_dir = svc
            .project_layout("superadmin", "default")
            .expect("layout")
            .data_recovery_dir();
        assert!(
            !recovery_dir.is_dir() || std::fs::read_dir(&recovery_dir).unwrap().next().is_none(),
            "a refused migration writes nothing"
        );
    }
}

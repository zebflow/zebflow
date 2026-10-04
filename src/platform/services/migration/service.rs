//! The migration run against a project through the platform's own
//! services: pipelines through the pipeline boundary, pages and the archive
//! through the repository file boundary. Nothing here opens a file itself.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;
use serde_json::{Value, json};

use super::plan::{
    ARCHIVE_PREFIX, Checked, FileKind, PipelineSource, Plan, PlanInputs, Status, archive_path, plan_sources,
};
use super::report;
use crate::platform::error::PlatformError;
use crate::platform::services::PlatformService;

/// The journal an apply keeps beside the archive: which pipelines were
/// active before it, so a second run (or a run after a failure) activates
/// the same ones.
pub const JOURNAL_FILE: &str = "archive/0.10/migration.json";
/// The readable record of every apply.
pub const REPORT_FILE: &str = "archive/0.10/MIGRATION.md";

/// Turning a pipeline on and off with everything that goes with it (the
/// runtime registry, schedules, subscriptions, socket clients, lifecycle
/// hooks). The web layer implements it — it owns those — and attaches it to
/// the project service; there is no lesser implementation, so an apply
/// switches pipelines the same way through every door.
#[async_trait]
pub trait PipelineSwitch: Send + Sync {
    async fn activate(&self, owner: &str, project: &str, file_rel_path: &str) -> Result<(), PlatformError>;
    async fn deactivate(&self, owner: &str, project: &str, file_rel_path: &str) -> Result<(), PlatformError>;
}

/// What an apply did.
#[derive(Debug, Clone, Serialize)]
pub struct ApplyReport {
    pub fingerprint: String,
    pub ok: bool,
    pub done: Vec<String>,
    pub failed: Vec<String>,
}

pub struct MigrationService {
    platform: Arc<PlatformService>,
}

impl MigrationService {
    pub fn new(platform: Arc<PlatformService>) -> Self {
        Self { platform }
    }

    fn read_text(&self, owner: &str, project: &str, repo_path: &str) -> Option<String> {
        self.platform.projects.read_repo_file_text(owner, project, repo_path).ok()
    }

    fn journal(&self, owner: &str, project: &str) -> BTreeMap<String, bool> {
        self.read_text(owner, project, JOURNAL_FILE)
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .and_then(|doc| doc.get("pipelines").cloned())
            .and_then(|p| p.as_object().cloned())
            .map(|map| {
                map.into_iter()
                    .filter_map(|(k, v)| v.get("was_active").and_then(Value::as_bool).map(|a| (k, a)))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Plans the migration of every pipeline of `owner/project` and every
    /// page a 0.10 pipeline renders. Reads only; writes nothing.
    pub fn plan_project(&self, owner: &str, project: &str) -> Result<Plan, PlatformError> {
        let projects = &self.platform.projects;
        let layout = projects.project_layout(owner, project)?;
        let source_root = layout.repo_layout.source.clone();
        let journal = self.journal(owner, project);
        let mut pipelines = Vec::new();
        for meta in projects.list_pipeline_meta_rows(owner, project)? {
            let repo_path = layout.repo_layout.source_rel(&meta.file_rel_path);
            if crate::platform::model::is_migration_archive_path(&repo_path) {
                continue;
            }
            let live = projects.read_pipeline_source(owner, project, &meta.file_rel_path)?;
            pipelines.push(PipelineSource {
                archived: self.read_text(owner, project, &archive_path(&repo_path)),
                repo_path,
                live,
                active: meta.active_hash.is_some(),
                draft: meta.active_hash.as_deref().is_some_and(|active| active != meta.hash),
                was_active: journal.get(&meta.file_rel_path).copied(),
                title: meta.title.clone(),
                description: meta.description.clone(),
                trigger_kind: meta.trigger_kind.clone(),
                file_rel_path: meta.file_rel_path,
            });
        }
        let credential_kinds = self
            .platform
            .credentials
            .list_project_credentials(owner, project)
            .map(|items| items.into_iter().map(|c| (c.credential_id, c.kind)).collect())
            .unwrap_or_default();
        let read_page = |repo_path: &str| {
            (self.read_text(owner, project, repo_path), self.read_text(owner, project, &archive_path(repo_path)))
        };
        let check = |file_rel_path: &str, doc: &Value| -> Result<Checked, String> {
            let source = serde_json::to_string_pretty(doc).map_err(|e| e.to_string())?;
            let (canonical, check) = projects
                .preview_pipeline_save(owner, project, file_rel_path, &source)
                .map_err(|e| e.message)?;
            Ok(Checked {
                canonical,
                refusals: check.refusals.iter().map(|p| p.to_string()).collect(),
                warnings: check.warnings.iter().map(|p| p.to_string()).collect(),
            })
        };
        let inputs = PlanInputs {
            owner,
            project,
            source_root: &source_root,
            credential_kinds,
            read_page: &read_page,
            check: &check,
        };
        Ok(plan_sources(&inputs, &pipelines))
    }

    /// Applies the plan whose fingerprint is `fingerprint`: refused when the
    /// project changed since (the fingerprint differs) or the plan is not
    /// ready. Running it again after it succeeded changes nothing.
    pub async fn apply_plan(&self, owner: &str, project: &str, fingerprint: &str) -> Result<ApplyReport, PlatformError> {
        let Some(switch) = self.platform.projects.pipeline_switch() else {
            return Err(PlatformError::new(
                "PLATFORM_MIGRATION_NO_RUNTIME",
                "the migration switches pipelines through the running web layer, and none is attached",
            ));
        };
        let switch = switch.as_ref();
        let plan = self.plan_project(owner, project)?;
        if plan.fingerprint != fingerprint.trim() {
            return Err(PlatformError::new(
                "PLATFORM_MIGRATION_STALE",
                format!(
                    "the project changed since that plan (its fingerprint is now {}); read the plan again and apply that one",
                    plan.fingerprint
                ),
            ));
        }
        if !plan.ready {
            return Err(PlatformError::new(
                "PLATFORM_MIGRATION_BLOCKED",
                format!(
                    "the plan has {} unresolved item(s) and {} refused check(s); resolve them first",
                    plan.counts.unresolved, plan.counts.refusals
                ),
            ));
        }
        let projects = &self.platform.projects;
        let mut done = Vec::new();
        let mut failed = Vec::new();

        // 1. The journal: what was active before anything is touched.
        let mut journal = self.journal(owner, project);
        let mut journal_changed = false;
        for file in plan.files.iter().filter(|f| f.kind == FileKind::Pipeline && f.status == Status::Rewrite) {
            let key = file.file_rel_path.clone().unwrap_or_default();
            if !journal.contains_key(&key) {
                journal.insert(key, file.active);
                journal_changed = true;
            }
        }
        if journal_changed {
            let doc = json!({
                "migration": "0.10 → 0.11",
                "pipelines": journal.iter().map(|(k, v)| (k.clone(), json!({ "was_active": v }))).collect::<serde_json::Map<_, _>>(),
            });
            projects.write_repo_file(owner, project, JOURNAL_FILE, &serde_json::to_string_pretty(&doc).unwrap_or_default())?;
        }

        // 2. Pipelines: off, archived, replaced.
        for file in plan.files.iter().filter(|f| f.kind == FileKind::Pipeline && f.status == Status::Rewrite) {
            let frp = file.file_rel_path.clone().unwrap_or_default();
            let currently_active = projects
                .get_pipeline_meta_by_file_id(owner, project, &frp)?
                .is_some_and(|m| m.active_hash.is_some());
            if currently_active && let Err(e) = switch.deactivate(owner, project, &frp).await {
                failed.push(format!("deactivate pipeline `{}`: {}", file.path, e.message));
                continue;
            }
            let archived = archive_path(&file.path);
            if self.read_text(owner, project, &archived).is_none()
                && let Err(e) = projects.write_repo_file(owner, project, &archived, &file.original)
            {
                failed.push(format!("archive pipeline `{}`: {}", file.path, e.message));
                continue;
            }
            match projects.upsert_pipeline_definition(
                owner,
                project,
                &frp,
                &file.title,
                &file.description,
                &file.trigger_kind,
                &file.new_text,
            ) {
                Ok(_) => done.push(format!("pipeline `{}` archived to `{archived}` and replaced by its 0.11 rewrite", file.path)),
                Err(e) => failed.push(format!("register the rewrite of `{}`: {}", file.path, e.message)),
            }
        }

        // 3. Pages: archived, replaced.
        for file in plan.files.iter().filter(|f| f.kind == FileKind::Page && f.status == Status::Rewrite) {
            let archived = archive_path(&file.path);
            if self.read_text(owner, project, &archived).is_none()
                && let Err(e) = projects.write_repo_file(owner, project, &archived, &file.original)
            {
                failed.push(format!("archive page `{}`: {}", file.path, e.message));
                continue;
            }
            match projects.write_repo_file(owner, project, &file.path, &file.new_text) {
                Ok(_) => done.push(format!("page `{}` archived to `{archived}` and rewritten", file.path)),
                Err(e) => failed.push(format!("write page `{}`: {}", file.path, e.message)),
            }
        }

        // 4. What was active is active again: functions first, so their
        // callers find them.
        let mut to_activate: Vec<_> = plan
            .files
            .iter()
            .filter(|f| f.kind == FileKind::Pipeline)
            .filter(|f| (f.status == Status::Rewrite && f.active) || f.needs_activation)
            .collect();
        to_activate.sort_by_key(|f| !f.function);
        for file in to_activate {
            let frp = file.file_rel_path.clone().unwrap_or_default();
            if failed.iter().any(|f| f.contains(&format!("`{}`", file.path))) {
                continue;
            }
            match switch.activate(owner, project, &frp).await {
                Ok(()) => done.push(format!("pipeline `{}` activated", file.path)),
                Err(e) => failed.push(format!("activate pipeline `{}`: {}", file.path, e.message)),
            }
        }

        // 5. The record.
        if !done.is_empty() || !failed.is_empty() {
            let at = chrono::Utc::now().format("%Y-%m-%d %H:%M UTC").to_string();
            let mut record = self.read_text(owner, project, REPORT_FILE).unwrap_or_else(|| {
                format!(
                    "# 0.10 → 0.11 migration\n\nThe 0.10 originals of every file the migration rewrote are kept under `{ARCHIVE_PREFIX}/`, \
                     at the path they had. Nothing under `archive/` is loaded as a pipeline. Delete the folder when the 0.11 project is verified.\n"
                )
            });
            record.push_str(&report::apply_markdown(&plan, &done, &failed, &at));
            projects.write_repo_file(owner, project, REPORT_FILE, &record)?;
        }
        Ok(ApplyReport { fingerprint: plan.fingerprint, ok: failed.is_empty(), done, failed })
    }
}

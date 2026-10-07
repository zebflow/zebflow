//! The 0.11 migration as part of an upgrade. An office upgraded from 0.10
//! holds pipelines 0.11 does not know; left alone their routes never
//! register and every site they serve answers 404, silently. So at server
//! start every project whose plan has work is either migrated (the plan is
//! ready and holds nothing for the owner to review) or named: one loud log
//! line, and a notice on the platform home and the project's dashboard
//! until the plan is applied.
//!
//! The apply is the owner's apply (`POST …/migration/0.11/apply`), so a
//! second start finds nothing to do.

use std::collections::BTreeMap;
use std::sync::RwLock;

use serde::Serialize;

use super::plan::{FileKind, Plan, Status};
use super::service::{ApplyReport, MigrationService};
use crate::platform::error::PlatformError;

/// A project the startup did not migrate: what the home and the dashboard
/// show, and the line the startup logged.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Notice {
    pub owner: String,
    pub project: String,
    /// What the owner has to look at: unresolved items, refused checks,
    /// review items, and steps of an apply that failed.
    pub items: usize,
    /// The readable plan.
    pub plan_href: String,
    /// The line logged at startup.
    pub line: String,
}

/// The notices of this process, by project. Set by the startup, cleared by
/// any apply that succeeds.
#[derive(Debug, Default)]
pub struct MigrationNotices {
    by_project: RwLock<BTreeMap<(String, String), Notice>>,
}

impl MigrationNotices {
    pub fn get(&self, owner: &str, project: &str) -> Option<Notice> {
        self.by_project.read().ok()?.get(&(owner.to_string(), project.to_string())).cloned()
    }

    pub fn set(&self, notice: Notice) {
        if let Ok(mut map) = self.by_project.write() {
            map.insert((notice.owner.clone(), notice.project.clone()), notice);
        }
    }

    pub fn clear(&self, owner: &str, project: &str) {
        if let Ok(mut map) = self.by_project.write() {
            map.remove(&(owner.to_string(), project.to_string()));
        }
    }
}

/// What the startup did with one project.
#[derive(Debug, Clone)]
pub enum StartupOutcome {
    /// Nothing of 0.10 is left.
    Nothing,
    /// Migrated; its pipelines are active as the originals were.
    Applied(ApplyReport),
    /// Not applied, or applied with failures: named for the owner.
    Named(Notice),
}

/// The plan has something to do: a file to rewrite, or a migrated pipeline
/// to switch on again.
pub fn has_work(plan: &Plan) -> bool {
    plan.files.iter().any(|f| f.status == Status::Rewrite || (f.kind == FileKind::Pipeline && f.needs_activation))
}

fn plan_href(owner: &str, project: &str) -> String {
    format!("/api/projects/{owner}/{project}/migration/0.11/plan?format=markdown")
}

fn blocked_notice(plan: &Plan) -> Notice {
    let c = &plan.counts;
    let mut blocks = Vec::new();
    if c.unresolved > 0 {
        blocks.push(format!("{} unresolved item(s)", c.unresolved));
    }
    if c.refusals > 0 {
        blocks.push(format!("{} refused check(s)", c.refusals));
    }
    if c.review > 0 {
        blocks.push(format!("{} review item(s)", c.review));
    }
    let href = plan_href(&plan.owner, &plan.project);
    Notice {
        owner: plan.owner.clone(),
        project: plan.project.clone(),
        items: c.unresolved + c.refusals + c.review,
        line: format!(
            "0.11 migration NOT applied: {}/{} — {} pipeline(s) and {} page(s) still 0.10; blocked by {}; read the plan at {href}",
            plan.owner,
            plan.project,
            c.pipelines_to_rewrite,
            c.pages_to_rewrite,
            blocks.join(", ")
        ),
        plan_href: href,
    }
}

impl MigrationService {
    /// Migrates `owner/project` if its plan is ready and holds nothing for
    /// review; otherwise names it. Sets or clears the project's notice.
    pub async fn migrate_at_start(&self, owner: &str, project: &str) -> Result<StartupOutcome, PlatformError> {
        let notices = &self.platform().migration_notices;
        let plan = self.plan_project(owner, project)?;
        if !has_work(&plan) {
            notices.clear(owner, project);
            return Ok(StartupOutcome::Nothing);
        }
        if !plan.ready || plan.counts.review > 0 {
            let notice = blocked_notice(&plan);
            notices.set(notice.clone());
            return Ok(StartupOutcome::Named(notice));
        }
        let report = self.apply_plan(owner, project, &plan.fingerprint).await?;
        if report.ok {
            return Ok(StartupOutcome::Applied(report));
        }
        let href = plan_href(owner, project);
        let notice = Notice {
            owner: owner.to_string(),
            project: project.to_string(),
            items: report.failed.len(),
            line: format!(
                "0.11 migration applied with {} failure(s): {owner}/{project} — {}; the record is archive/0.10/MIGRATION.md",
                report.failed.len(),
                report.failed.join("; ")
            ),
            plan_href: href,
        };
        notices.set(notice.clone());
        Ok(StartupOutcome::Named(notice))
    }

    /// Whether a named project still needs its migration, without applying
    /// anything: a project whose owner finished it by hand loses its notice.
    pub fn refresh_notice(&self, owner: &str, project: &str) -> Option<Notice> {
        let notices = &self.platform().migration_notices;
        notices.get(owner, project)?;
        match self.plan_project(owner, project) {
            Ok(plan) if !has_work(&plan) => {
                notices.clear(owner, project);
                None
            }
            Ok(plan) if plan.ready && plan.counts.review == 0 => notices.get(owner, project),
            Ok(plan) => {
                let notice = blocked_notice(&plan);
                notices.set(notice.clone());
                Some(notice)
            }
            Err(_) => notices.get(owner, project),
        }
    }
}

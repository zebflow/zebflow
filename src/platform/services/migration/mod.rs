//! Moving a 0.10 project to 0.11: every pipeline rewritten for the 0.11
//! kinds, flags and answers, every page a 0.10 pipeline renders rewritten
//! for the payload it now receives, the originals archived under
//! `archive/0.10/`.
//!
//! - [`rewrite_pipeline`] — one 0.10 pipeline document → its 0.11 rewrite,
//!   what changed, behaviour notes, and what cannot be mapped with
//!   certainty (reported, never guessed).
//! - [`rewrite_page`] — one TSX page, against the response nodes that
//!   render it.
//! - [`MigrationService::plan_project`] — the whole project, each rewritten
//!   pipeline put through the save-time check; a readable report and a
//!   fingerprint.
//! - [`MigrationService::apply_plan`] — that plan applied: pipelines off,
//!   archived, replaced, on again; pages archived and replaced; a record in
//!   `archive/0.10/MIGRATION.md`. Idempotent.
//! - [`startup`] — the same apply at server start, for a project whose plan
//!   is ready and holds nothing to review; any other is logged and noticed.
//!
//! The 0.10 behaviour modelled is the `v0.10.12` release's (what the offices
//! run). The table of kinds is in [`kinds`].

mod diff;
pub mod expr;
pub mod graph;
pub mod kinds;
mod node;
pub mod page;
pub mod pipeline;
pub mod plan;
mod refs;
mod report;
mod service;
pub mod startup;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

pub use page::{PageContext, PageRewrite, Renderer, rewrite_page};
pub use pipeline::{Item, Rewrite, is_old_document, rewrite_pipeline};
pub use plan::{ARCHIVE_PREFIX, Checked, FileKind, PageSource, PipelineSource, Plan, PlanFile, PlanInputs, Status, plan_sources};
pub use service::{ApplyReport, JOURNAL_FILE, MigrationService, PipelineSwitch, REPORT_FILE};
pub use startup::{MigrationNotices, Notice, StartupOutcome};

use graph::OldOutput;

/// What a rewrite knows about the project beyond the one pipeline.
#[derive(Debug, Clone, Default)]
pub struct RewriteContext {
    /// For each 0.10 function pipeline (by name), what its caller received
    /// and where that is in 0.11.
    pub functions: BTreeMap<String, OldOutput>,
    /// Credential kinds by id (a provider 0.10 read from its credential).
    pub credential_kinds: BTreeMap<String, String>,
}

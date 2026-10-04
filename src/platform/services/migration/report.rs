//! The plan and the apply written for a person to read.

use super::plan::{FileKind, Plan, PlanFile, Status};

fn kind_word(file: &PlanFile) -> &'static str {
    match file.kind {
        FileKind::Pipeline => "pipeline",
        FileKind::Page => "page",
    }
}

fn items(out: &mut String, title: &str, items: &[super::pipeline::Item]) {
    if items.is_empty() {
        return;
    }
    out.push_str(&format!("{title}:\n"));
    for item in items {
        if item.node.is_empty() {
            out.push_str(&format!("- {}\n", item.text));
        } else {
            out.push_str(&format!("- node `{}`: {}\n", item.node, item.text));
        }
    }
}

/// The plan as Markdown: a summary, then every file that changes or blocks.
pub fn plan_markdown(plan: &Plan) -> String {
    let c = &plan.counts;
    let mut out = format!("# 0.11 migration plan — {}/{}\n\n", plan.owner, plan.project);
    out.push_str(&format!("Fingerprint `{}` · ", plan.fingerprint));
    if plan.ready {
        out.push_str("**ready to apply**\n\n");
    } else {
        out.push_str(&format!(
            "**not ready**: {} file(s) blocked — {} unresolved item(s), {} refused check(s)\n\n",
            c.blocked_files, c.unresolved, c.refusals
        ));
    }
    out.push_str(&format!(
        "Pipelines: {} to rewrite, {} already migrated, {} already 0.11. Pages: {} to rewrite, {} already migrated. \
         Check warnings: {}. Behaviour notes: {}. To review: {}.\n\n",
        c.pipelines_to_rewrite, c.pipelines_done, c.pipelines_unchanged, c.pages_to_rewrite, c.pages_done, c.warnings, c.notes, c.review
    ));
    out.push_str(
        "Apply deactivates each pipeline below, copies it and each page below to `archive/0.10/<same path>`, \
         registers the rewritten pipeline at its own path, writes the rewritten page, and activates again what was active.\n\
         An unresolved item blocks the apply: write that file's 0.11 version yourself (register it at the same path) \
         and the plan lists it as already 0.11.\n",
    );
    for file in &plan.files {
        if file.status == Status::Unchanged && !file.blocked() {
            continue;
        }
        let state = match file.status {
            Status::Rewrite => "rewrite",
            Status::Done => {
                if file.needs_activation {
                    "already migrated — to activate"
                } else {
                    "already migrated"
                }
            }
            Status::Unchanged => "unchanged",
        };
        out.push_str(&format!("\n## {} `{}` — {state}", kind_word(file), file.path));
        if file.kind == FileKind::Pipeline && file.status == Status::Rewrite {
            out.push_str(if file.active { " (active: re-activated after)" } else { " (inactive: stays inactive)" });
        }
        out.push_str("\n\n");
        items(&mut out, "Unresolved (blocks the apply)", &file.unresolved);
        items(&mut out, "Review (does not block the apply)", &file.review);
        if let Some(check) = &file.check {
            if check.refusals.is_empty() {
                out.push_str(&format!("Save-time check: passed, {} warning(s)\n", check.warnings.len()));
            } else {
                out.push_str(&format!("Save-time check: **refused** ({} problem(s))\n", check.refusals.len()));
                for refusal in &check.refusals {
                    out.push_str(&format!("- {refusal}\n"));
                }
            }
            for warning in &check.warnings {
                out.push_str(&format!("- warning: {warning}\n"));
            }
        }
        items(&mut out, "Changes", &file.changes);
        items(&mut out, "Behaviour notes", &file.notes);
        if !file.diff.is_empty() {
            out.push_str("\n```diff\n");
            out.push_str(&file.diff);
            out.push_str("```\n");
        }
    }
    out
}

/// One apply, as a section of `archive/0.10/MIGRATION.md`.
pub fn apply_markdown(plan: &Plan, done: &[String], failed: &[String], at: &str) -> String {
    let mut out = format!("\n## Apply {at} — plan `{}`\n\n", plan.fingerprint);
    if done.is_empty() && failed.is_empty() {
        out.push_str("Nothing to do: every file was already migrated.\n");
        return out;
    }
    for line in done {
        out.push_str(&format!("- {line}\n"));
    }
    for line in failed {
        out.push_str(&format!("- **failed**: {line}\n"));
    }
    out
}

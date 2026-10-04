//! A project's migration planned from its files alone: every pipeline and
//! every page a 0.10 pipeline renders, rewritten, checked and diffed. The
//! platform service feeds it through its own services; nothing here touches
//! storage.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::RewriteContext;
use super::diff::unified;
use super::graph::OldOutput;
use super::kinds;
use super::page::{PageContext, Renderer, rewrite_component, rewrite_page};
use super::pipeline::{Item, is_old_document, old_graph, rewrite_pipeline};

/// Where originals are kept, repository-relative: `archive/0.10/<path>`.
pub const ARCHIVE_PREFIX: &str = "archive/0.10";

pub fn archive_path(repo_path: &str) -> String {
    format!("{ARCHIVE_PREFIX}/{}", repo_path.trim_start_matches('/'))
}

/// One pipeline as the project holds it.
#[derive(Debug, Clone)]
pub struct PipelineSource {
    /// Source-relative identity (`api/posts.zf.json`).
    pub file_rel_path: String,
    /// Repository-relative path.
    pub repo_path: String,
    pub live: String,
    /// The archived original, when an earlier apply wrote one.
    pub archived: Option<String>,
    pub active: bool,
    pub title: String,
    pub description: String,
    pub trigger_kind: String,
    /// The working tree is not what is active (a draft never activated).
    pub draft: bool,
    /// The journal says it was active before the migration.
    pub was_active: Option<bool>,
}

/// One page as the project holds it.
#[derive(Debug, Clone)]
pub struct PageSource {
    pub repo_path: String,
    pub live: Option<String>,
    pub archived: Option<String>,
}

/// What a save of a rewritten pipeline would make of it.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Checked {
    /// The source as the save writes it.
    #[serde(skip)]
    pub canonical: String,
    pub refusals: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    Pipeline,
    Page,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// A 0.10 file the apply rewrites.
    Rewrite,
    /// Already rewritten by an earlier apply.
    Done,
    /// Born 0.11, or a page nothing changed in.
    Unchanged,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanFile {
    pub kind: FileKind,
    /// Repository-relative path.
    pub path: String,
    /// Source-relative identity, for a pipeline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_rel_path: Option<String>,
    pub status: Status,
    /// A pipeline active before the migration, activated again after it.
    pub active: bool,
    /// A migrated pipeline the journal says was active and is not now.
    pub needs_activation: bool,
    pub changes: Vec<Item>,
    pub notes: Vec<Item>,
    pub unresolved: Vec<Item>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check: Option<Checked>,
    pub diff: String,
    #[serde(skip)]
    pub original: String,
    #[serde(skip)]
    pub new_text: String,
    #[serde(skip)]
    pub title: String,
    #[serde(skip)]
    pub description: String,
    #[serde(skip)]
    pub trigger_kind: String,
    /// The pipeline is a function another one calls, activated first.
    #[serde(skip)]
    pub function: bool,
}

impl PlanFile {
    pub fn blocked(&self) -> bool {
        !self.unresolved.is_empty() || self.check.as_ref().is_some_and(|c| !c.refusals.is_empty())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Plan {
    pub owner: String,
    pub project: String,
    /// Changes when anything the apply would do changes; the apply refuses a
    /// plan whose fingerprint is not the current one.
    pub fingerprint: String,
    /// No unresolved item and no refused check anywhere.
    pub ready: bool,
    pub counts: Counts,
    pub files: Vec<PlanFile>,
    /// The plan as a readable report.
    pub report: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Counts {
    pub pipelines: usize,
    pub pipelines_to_rewrite: usize,
    pub pipelines_done: usize,
    pub pipelines_unchanged: usize,
    pub pages_to_rewrite: usize,
    pub pages_done: usize,
    pub blocked_files: usize,
    pub unresolved: usize,
    pub refusals: usize,
    pub warnings: usize,
    pub notes: usize,
}

/// The inputs of a plan besides the files.
pub struct PlanInputs<'a> {
    pub owner: &'a str,
    pub project: &'a str,
    /// The source root, repository-relative ("" for the repository root).
    pub source_root: &'a str,
    pub credential_kinds: BTreeMap<String, String>,
    /// Reads a page by repository path: (live, archived).
    pub read_page: &'a dyn Fn(&str) -> (Option<String>, Option<String>),
    /// What a save of a rewritten document would make of it.
    pub check: &'a dyn Fn(&str, &Value) -> Result<Checked, String>,
}

fn source_rel(root: &str, rest: &str) -> String {
    let rest = rest.trim_start_matches('/');
    if root.is_empty() { rest.to_string() } else { format!("{}/{rest}", root.trim_end_matches('/')) }
}

fn pretty(text: &str) -> String {
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|v| serde_json::to_string_pretty(&v).ok())
        .unwrap_or_else(|| text.to_string())
}

/// Plans the migration of `pipelines` (and of the pages they render).
pub fn plan_sources(inputs: &PlanInputs<'_>, pipelines: &[PipelineSource]) -> Plan {
    // Originals: the archived copy once an apply has written one.
    let originals: Vec<(&PipelineSource, Option<Value>)> = pipelines
        .iter()
        .map(|p| {
            let text = p.archived.as_deref().unwrap_or(&p.live);
            (p, serde_json::from_str::<Value>(text).ok())
        })
        .collect();

    let mut context = RewriteContext { functions: BTreeMap::new(), credential_kinds: inputs.credential_kinds.clone() };
    let mut functions: BTreeSet<String> = BTreeSet::new();
    for (source, doc) in &originals {
        let Some(doc) = doc else { continue };
        if !is_old_document(doc) {
            continue;
        }
        let name = crate::platform::services::project::name_from_file_rel_path(&source.file_rel_path);
        if let Some(model) = function_model(doc, &context) {
            functions.insert(source.file_rel_path.clone());
            context.functions.insert(name, model);
        }
    }

    let mut files = Vec::new();
    let mut page_renderers: BTreeMap<String, Vec<Renderer>> = BTreeMap::new();
    let mut eleven_renders: BTreeSet<String> = BTreeSet::new();
    for (source, doc) in &originals {
        let mut file = PlanFile {
            kind: FileKind::Pipeline,
            path: source.repo_path.clone(),
            file_rel_path: Some(source.file_rel_path.clone()),
            status: Status::Unchanged,
            active: source.was_active.unwrap_or(source.active),
            needs_activation: false,
            changes: Vec::new(),
            notes: Vec::new(),
            unresolved: Vec::new(),
            check: None,
            diff: String::new(),
            original: source.archived.clone().unwrap_or_else(|| source.live.clone()),
            new_text: String::new(),
            title: source.title.clone(),
            description: source.description.clone(),
            trigger_kind: source.trigger_kind.clone(),
            function: functions.contains(&source.file_rel_path),
        };
        let Some(doc) = doc else {
            file.unresolved.push(Item::new("", "the file is not JSON"));
            files.push(file);
            continue;
        };
        let live_doc = serde_json::from_str::<Value>(&source.live).ok();
        let live_is_old = live_doc.as_ref().is_some_and(is_old_document);
        if !is_old_document(doc) {
            // Born 0.11: its templates are read as 0.11 already.
            for node in doc.pointer("/spec/nodes").and_then(Value::as_array).into_iter().flatten() {
                if let Some(t) = node.pointer("/config/template").and_then(Value::as_str) {
                    eleven_renders.insert(source_rel(inputs.source_root, t));
                }
            }
            files.push(file);
            continue;
        }
        let rewrite = rewrite_pipeline(doc, &context);
        if let Ok(graph) = old_graph(doc, &context) {
            let graph = Arc::new(graph);
            for (node_id, template) in &rewrite.templates {
                if let Some(node) = graph.index(node_id) {
                    page_renderers
                        .entry(source_rel(inputs.source_root, template))
                        .or_default()
                        .push(Renderer { pipeline: source.file_rel_path.clone(), graph: graph.clone(), node });
                }
            }
        }
        file.changes = rewrite.changes;
        file.notes = rewrite.notes;
        file.unresolved = rewrite.unresolved;
        if source.draft && live_is_old {
            file.unresolved.push(Item::new(
                "",
                "the working tree holds changes that were never activated; the rewrite would put them live — activate or revert them first",
            ));
        }
        if let Some(new_doc) = &rewrite.new_json {
            match (inputs.check)(&source.file_rel_path, new_doc) {
                Ok(checked) => {
                    file.new_text = checked.canonical.clone();
                    file.check = Some(checked);
                }
                Err(why) => file.unresolved.push(Item::new("", format!("the rewritten pipeline does not load: {why}"))),
            }
        }
        if file.new_text.is_empty() {
            file.new_text = rewrite.new_json.as_ref().and_then(|v| serde_json::to_string_pretty(v).ok()).unwrap_or_default();
        }
        file.diff = unified(&pretty(&file.original), &pretty(&file.new_text));
        if live_is_old {
            file.status = Status::Rewrite;
            if source.archived.as_deref().is_some_and(|a| a != source.live) {
                file.unresolved.push(Item::new("", "an archived copy exists and differs from the 0.10 file; compare them by hand"));
            }
        } else {
            file.status = Status::Done;
            file.needs_activation = source.was_active == Some(true) && !source.active;
            file.changes.clear();
            file.notes.clear();
            file.unresolved.clear();
            file.check = None;
            file.diff.clear();
        }
        files.push(file);
    }

    // The components pages import read the same payload through the
    // `input` / `ctx` globals: each is rewritten against every renderer of
    // every page that imports it.
    let mut views: Vec<(String, Vec<Renderer>, bool)> =
        page_renderers.iter().map(|(path, r)| (path.clone(), r.clone(), true)).collect();
    let mut component_renderers: BTreeMap<String, Vec<Renderer>> = BTreeMap::new();
    let mut frontier: Vec<(String, Vec<Renderer>)> = page_renderers.iter().map(|(p, r)| (p.clone(), r.clone())).collect();
    let mut seen: BTreeSet<String> = page_renderers.keys().cloned().collect();
    while let Some((path, renderers)) = frontier.pop() {
        let (live, archived) = (inputs.read_page)(&path);
        let Some(text) = archived.or(live) else { continue };
        let source_path = path.strip_prefix(&format!("{}/", inputs.source_root.trim_end_matches('/'))).unwrap_or(&path);
        let own_dir = source_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        for import in super::expr::local_imports(&text, own_dir) {
            let candidates = [import.clone(), format!("{import}.tsx"), format!("{import}.ts"), format!("{import}/index.tsx")];
            let Some(found) = candidates
                .iter()
                .map(|c| source_rel(inputs.source_root, c))
                .find(|c| (c.ends_with(".tsx") || c.ends_with(".ts")) && (inputs.read_page)(c).0.is_some())
            else {
                continue;
            };
            let entry = component_renderers.entry(found.clone()).or_default();
            let before = entry.len();
            for r in &renderers {
                if !entry.iter().any(|e| e.pipeline == r.pipeline && e.node == r.node) {
                    entry.push(r.clone());
                }
            }
            // Again whenever it gained a renderer, so what it imports does too.
            if seen.insert(found.clone()) || entry.len() > before {
                let entry = entry.clone();
                frontier.push((found, entry));
            }
        }
    }
    views.extend(component_renderers.into_iter().filter(|(p, _)| !page_renderers.contains_key(p)).map(|(p, r)| (p, r, false)));

    for (repo_path, renderers, is_page) in &views {
        let (live, archived) = (inputs.read_page)(repo_path);
        let Some(original) = archived.clone().or_else(|| live.clone()) else {
            continue;
        };
        let context = PageContext { renderers: renderers.clone() };
        let rewrite = if *is_page { rewrite_page(&original, &context) } else { rewrite_component(&original, &context) };
        let new_text = rewrite.new_tsx.clone().unwrap_or_else(|| original.clone());
        let mut file = PlanFile {
            kind: FileKind::Page,
            path: repo_path.clone(),
            file_rel_path: None,
            status: Status::Unchanged,
            active: false,
            needs_activation: false,
            changes: rewrite.changes,
            notes: rewrite.notes,
            unresolved: rewrite.unresolved,
            check: None,
            diff: unified(&original, &new_text),
            original: original.clone(),
            new_text: new_text.clone(),
            title: String::new(),
            description: String::new(),
            trigger_kind: String::new(),
            function: false,
        };
        let current = live.clone().unwrap_or_default();
        if new_text == original && file.unresolved.is_empty() {
            file.status = Status::Unchanged;
        } else if archived.is_some() && current == new_text {
            file.status = Status::Done;
            file.changes.clear();
            file.notes.clear();
            file.diff.clear();
        } else {
            file.status = Status::Rewrite;
            if archived.is_some() && current != original {
                file.unresolved.push(Item::new("", "the page changed after it was archived; compare it with the archived copy by hand"));
            }
            if eleven_renders.contains(repo_path) {
                file.unresolved.push(Item::new("", "a 0.11 pipeline renders this page too, and reads it as it is"));
            }
        }
        if file.status != Status::Unchanged || !file.unresolved.is_empty() {
            files.push(file);
        }
    }
    finish(inputs.owner, inputs.project, files)
}

/// What a 0.10 function pipeline answered its caller, as a `function.call`
/// output model.
fn function_model(doc: &Value, context: &RewriteContext) -> Option<OldOutput> {
    let nodes = doc.pointer("/spec/nodes")?.as_array()?;
    if !nodes.iter().any(|n| n.get("kind").and_then(Value::as_str) == Some("n.trigger.function")) {
        return None;
    }
    let graph = old_graph(doc, context).ok()?;
    let sinks: Vec<usize> =
        (0..graph.nodes.len()).filter(|&i| !graph.edges.iter().any(|e| e.from == i)).collect();
    // What each place the function can end at answered; a function whose
    // branches all end the same way (each a script) answered the same way.
    let mut models = Vec::new();
    for &sink in &sinks {
        let mut node = sink;
        let mut found = None;
        for _ in 0..graph.nodes.len() {
            match &graph.outputs[node] {
                OldOutput::Pass => {
                    let [edge] = graph.incoming[node].as_slice() else { break };
                    node = graph.edges[*edge].from;
                }
                output => {
                    found = Some(kinds::function_result(output, graph.nouns[node].as_deref()));
                    break;
                }
            }
        }
        models.push(found);
    }
    let model = match models.first() {
        Some(Some(first)) if models.iter().all(|m| m.as_ref() == Some(first)) => Some(first.clone()),
        // Branches that each end in a script: what was returned is one of
        // their returns, under `result` either way.
        Some(Some(_)) if models.iter().all(|m| matches!(m, Some(OldOutput::Replace { whole: Some(w), .. }) if w == &["result".to_string()])) => {
            let mut keys: Vec<super::graph::Mapping> = Vec::new();
            let mut open = false;
            for model in models.iter().flatten() {
                if let OldOutput::Replace { keys: k, rest, .. } = model {
                    open |= *rest == super::graph::Rest::Open;
                    for key in k {
                        if !keys.iter().any(|existing| existing.old == key.old) {
                            keys.push(key.clone());
                        }
                    }
                }
            }
            Some(OldOutput::Replace {
                keys,
                whole: Some(vec!["result".to_string()]),
                rest: if open { super::graph::Rest::Open } else { super::graph::Rest::Dead },
            })
        }
        _ => None,
    };
    Some(model.unwrap_or_else(|| {
        OldOutput::unknown("the function ends at nodes that answered differently, so what it returned cannot be said")
    }))
}

fn finish(owner: &str, project: &str, mut files: Vec<PlanFile>) -> Plan {
    files.sort_by(|a, b| (a.kind as u8, &a.path).cmp(&(b.kind as u8, &b.path)));
    let mut counts = Counts::default();
    let mut hasher = Sha256::new();
    for file in &files {
        match (file.kind, file.status) {
            (FileKind::Pipeline, status) => {
                counts.pipelines += 1;
                match status {
                    Status::Rewrite => counts.pipelines_to_rewrite += 1,
                    Status::Done => counts.pipelines_done += 1,
                    Status::Unchanged => counts.pipelines_unchanged += 1,
                }
            }
            (FileKind::Page, Status::Rewrite) => counts.pages_to_rewrite += 1,
            (FileKind::Page, Status::Done) => counts.pages_done += 1,
            _ => {}
        }
        if file.blocked() {
            counts.blocked_files += 1;
        }
        counts.unresolved += file.unresolved.len();
        counts.notes += file.notes.len();
        if let Some(check) = &file.check {
            counts.refusals += check.refusals.len();
            counts.warnings += check.warnings.len();
        }
        if file.status == Status::Rewrite || file.needs_activation {
            for part in [file.path.as_str(), file.original.as_str(), file.new_text.as_str(), if file.active { "1" } else { "0" }] {
                hasher.update((part.len() as u64).to_le_bytes());
                hasher.update(part.as_bytes());
            }
        }
    }
    let fingerprint = format!("{:x}", hasher.finalize())[..16].to_string();
    let ready = counts.blocked_files == 0;
    let mut plan = Plan { owner: owner.to_string(), project: project.to_string(), fingerprint, ready, counts, files, report: String::new() };
    plan.report = super::report::plan_markdown(&plan);
    plan
}

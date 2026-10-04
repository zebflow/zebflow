//! A TSX page rewritten for what its `input` is in 0.11.
//!
//! A page's `input` (its first parameter, the `input` and `ctx` globals) is
//! the payload at the response node that renders it, plus `route`,
//! `params`, `query`, `search`, `headers` and `auth` from the request when
//! the payload has no key of that name — the same in both versions. What
//! changed is the payload: `input.rows` after a Postgres query is
//! `input.query.rows`. A page is rewritten against every response node that
//! renders it; they must agree.

use std::ops::Range;
use std::sync::Arc;

use super::expr::{self, Root, Wrap};
use super::graph::OldGraph;
use super::pipeline::Item;

/// The keys the renderer copies from the request when the payload lacks them.
const INJECTED: &[&str] = &["route", "params", "query", "search", "headers", "auth"];
/// Of those, the ones a webhook answers under `webhook` in 0.11.
const IN_WEBHOOK: &[&str] = &["params", "query", "headers", "auth"];

/// One response node that renders the page.
#[derive(Clone)]
pub struct Renderer {
    pub pipeline: String,
    pub graph: Arc<OldGraph>,
    pub node: usize,
}

/// The response nodes that render one page.
#[derive(Clone, Default)]
pub struct PageContext {
    pub renderers: Vec<Renderer>,
}

/// What `rewrite_page` made of a page.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct PageRewrite {
    /// The rewritten source; `None` when nothing changed.
    pub new_tsx: Option<String>,
    pub changes: Vec<Item>,
    pub notes: Vec<Item>,
    pub unresolved: Vec<Item>,
}

/// Rewrites `old_tsx` for the 0.11 payload its renderers hand it.
pub fn rewrite_page(old_tsx: &str, context: &PageContext) -> PageRewrite {
    rewrite_view(old_tsx, context, Wrap::Page)
}

/// Rewrites a component a page imports: its reads of the `input` and `ctx`
/// globals are the page's payload.
pub fn rewrite_component(old_tsx: &str, context: &PageContext) -> PageRewrite {
    rewrite_view(old_tsx, context, Wrap::Component)
}

fn rewrite_view(old_tsx: &str, context: &PageContext, wrap: Wrap) -> PageRewrite {
    let mut out = PageRewrite::default();
    if context.renderers.is_empty() {
        return out;
    }
    let scan = match expr::scan(old_tsx, wrap) {
        Ok(scan) => scan,
        Err(why) => {
            out.unresolved.push(Item::new("", format!("the page {why}")));
            return out;
        }
    };
    for key in &scan.destructured {
        let placed: Vec<Result<Placement, String>> =
            context.renderers.iter().map(|r| place(r, &[key.clone()], false, "input")).collect();
        if placed.iter().any(|p| !matches!(p, Ok(Placement::Injected | Placement::Dead))) {
            out.unresolved.push(Item::new(
                "",
                format!("the page destructures `{key}` from its props; 0.11 holds it elsewhere — read it from input"),
            ));
        }
    }
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();
    for reference in &scan.references {
        if reference.root != Root::Input {
            continue;
        }
        let head = &old_tsx[reference.head.clone()];
        let mut placements = Vec::new();
        let mut failed = None;
        for renderer in &context.renderers {
            match place(renderer, &reference.path, reference.optional, &reference.root_text) {
                Ok(placement) => placements.push(placement),
                Err(why) => {
                    failed = Some(format!("`{head}` (rendered by {}): {why}", renderer.pipeline));
                    break;
                }
            }
        }
        if let Some(why) = failed {
            out.unresolved.push(Item::new("", why));
            continue;
        }
        match agree(&placements, &reference.root_text, &reference.path, reference.optional, &context.renderers) {
            Some(Some(text)) if text != head => {
                out.changes.push(Item::new("", format!("`{head}` → `{text}`")));
                edits.push((reference.head.clone(), text));
            }
            Some(_) => {}
            None => out.unresolved.push(Item::new("", format!("`{head}` is held differently by the pipelines that render this page"))),
        }
    }
    for bare in &scan.bare {
        if bare.whole {
            continue;
        }
        out.unresolved.push(Item::new("", format!("`{}` reads the payload by a computed key", bare.text)));
    }
    if !edits.is_empty() {
        out.new_tsx = Some(expr::apply_edits(old_tsx, edits));
    }
    out
}

/// Where one renderer puts a key the page reads.
#[derive(Debug, Clone, PartialEq)]
enum Placement {
    /// Read unchanged: the renderer still fills it from the request.
    Injected,
    /// Read nothing in 0.10 and reads nothing in 0.11.
    Dead,
    /// Read here in 0.11.
    At(String),
}

/// One text for a key every renderer can read it by: the same place, or —
/// for a page several triggers render (a webhook and an error handler) — the
/// first of places under different trigger answers, only one of which any
/// payload holds. `Some(None)`: the page reads it as written.
fn agree(placements: &[Placement], root: &str, path: &[String], optional: bool, renderers: &[Renderer]) -> Option<Option<String>> {
    let first = placements.first()?;
    if placements.iter().all(|p| p == first) {
        return Some(match first {
            Placement::At(text) => Some(text.clone()),
            _ => None,
        });
    }
    // The request's own key, named explicitly under `webhook`.
    let explicit = |renderer: &Renderer| -> Option<String> {
        renderer.graph.nodes.iter().any(|n| n.kind == "n.trigger.webhook").then(|| {
            let mut segments = vec!["webhook".to_string()];
            segments.extend(path.iter().cloned());
            if optional { super::graph::optional_path(root, &segments) } else { super::graph::member_path(root, &segments) }
        })
    };
    let mut alternatives: Vec<(String, String)> = Vec::new();
    for (placement, renderer) in placements.iter().zip(renderers) {
        let text = match placement {
            Placement::At(text) => text.clone(),
            Placement::Injected => explicit(renderer)?,
            Placement::Dead => continue,
        };
        let after = text.strip_prefix(root)?.trim_start_matches('?').trim_start_matches('.');
        let noun = after.split(['.', '?', '[']).next()?.to_string();
        if !alternatives.iter().any(|(t, _)| *t == text) {
            alternatives.push((text, noun));
        }
    }
    if alternatives.len() == 1 && !placements.contains(&Placement::Dead) {
        return Some(Some(alternatives.remove(0).0));
    }
    let mut nouns: Vec<&str> = Vec::new();
    for (_, noun) in &alternatives {
        if nouns.contains(&noun.as_str()) {
            return None;
        }
        nouns.push(noun);
    }
    // Each payload holds at most one of the answers, and a renderer that
    // read nothing holds none of them.
    for (placement, renderer) in placements.iter().zip(renderers) {
        let upstream = renderer.graph.ancestors(renderer.node);
        let present = nouns
            .iter()
            .filter(|n| upstream.iter().any(|&a| renderer.graph.nouns[a].as_deref() == Some(**n)))
            .count();
        if present > 1 || (*placement == Placement::Dead && present > 0) {
            return None;
        }
    }
    let texts: Vec<String> = alternatives
        .iter()
        .map(|(text, _)| {
            let after = text.strip_prefix(root).unwrap_or(text).trim_start_matches('?').trim_start_matches('.');
            let segments: Vec<String> = after.split('.').map(|s| s.trim_end_matches('?').to_string()).collect();
            super::graph::optional_path(root, &segments)
        })
        .collect();
    Some(Some(format!("({})", texts.join(" ?? "))))
}

/// `input.webhook.<path>` when the renderer's pipeline starts at a webhook.
fn explicit_webhook(graph: &OldGraph, path: &[String], optional: bool, root: &str) -> Option<String> {
    if !IN_WEBHOOK.contains(&path.first()?.as_str()) || !graph.nodes.iter().any(|n| n.kind == "n.trigger.webhook") {
        return None;
    }
    let mut segments = vec!["webhook".to_string()];
    segments.extend(path.iter().cloned());
    Some(if optional { super::graph::optional_path(root, &segments) } else { super::graph::member_path(root, &segments) })
}

/// Where `path` of the page's payload is in 0.11, at one renderer:
/// `Ok(None)` when the page reads it unchanged.
fn place(renderer: &Renderer, path: &[String], optional: bool, root: &str) -> Result<Placement, String> {
    let graph = &renderer.graph;
    let node = renderer.node;
    let Some(first) = path.first() else { return Ok(Placement::Injected) };
    match graph.resolve_input(node, path) {
        Ok(origin)
            if IN_WEBHOOK.contains(&first.as_str())
                && graph.nodes[origin.producer].kind == "n.trigger.webhook"
                && !graph.answered_upstream(node, first) =>
        {
            // The renderer copies the request's own `params` / `query` /
            // `headers` / `auth` in when nothing answers that key: the
            // page reads them unchanged.
            Ok(Placement::Injected)
        }
        Ok(origin) => {
            if INJECTED.contains(&first.as_str()) && graph.nodes[origin.producer].kind == "n.script" {
                // Where the script returned no `query`, 0.10 filled it from
                // the request.
                let producer = origin.producer;
                let source = graph.nodes[producer].config.get("source").and_then(|v| v.as_str()).unwrap_or("");
                match super::expr::returned(source) {
                    Some(r) if r.always.contains(first.as_str()) => {}
                    Some(r) if r.spreads_input && !r.any.contains(first.as_str()) => {
                        match graph.resolve_input(producer, &path[..1]) {
                            Ok(_) => {}
                            Err(f) if f.dead => {
                                return Ok(if graph.answered_upstream(node, first) {
                                    match explicit_webhook(graph, path, optional, root) {
                                        Some(text) => Placement::At(text),
                                        None => return Err(format!("`{first}` came from the request in 0.10; in 0.11 an upstream node answers `{first}` first")),
                                    }
                                } else {
                                    Placement::Injected
                                });
                            }
                            Err(f) => return Err(f.why),
                        }
                    }
                    _ => {
                        return Err(format!(
                            "script `{}` may not return `{first}`, and where it did not, 0.10 filled `{first}` from the request",
                            graph.nodes[producer].id
                        ));
                    }
                }
            }
            if graph.shadowed(&origin, None) {
                return Err(format!(
                    "a node between `{}` and the response answers `{}` too, and a page cannot read $nodes",
                    graph.nodes[origin.producer].id,
                    origin.first().unwrap_or_default()
                ));
            }
            if let super::graph::Target::Gone(why) = &origin.base {
                return Err(why.clone());
            }
            Ok(Placement::At(origin.render_with(root, optional)))
        }
        Err(failure) if failure.dead => {
            let answered = graph.answered_upstream(node, first);
            if INJECTED.contains(&first.as_str()) {
                // The renderer filled it from the request in 0.10.
                if !answered {
                    return Ok(Placement::Injected);
                }
                if IN_WEBHOOK.contains(&first.as_str()) && graph.nodes.iter().any(|n| n.kind == "n.trigger.webhook") {
                    let mut segments = vec!["webhook".to_string()];
                    segments.extend(path.iter().cloned());
                    let text = if optional {
                        super::graph::optional_path(root, &segments)
                    } else {
                        super::graph::member_path(root, &segments)
                    };
                    return Ok(Placement::At(text));
                }
                return Err(format!("`{first}` came from the request in 0.10; in 0.11 an upstream node answers `{first}` first"));
            }
            if answered {
                Err(format!("read nothing in 0.10, but an upstream node answers `{first}` in 0.11"))
            } else {
                Ok(Placement::Dead)
            }
        }
        Err(failure) => Err(failure.why),
    }
}

//! Rewriting the payload references in one piece of text — a `{{ }}`
//! template, a bare expression, a script body — read at one node of a 0.10
//! graph.

use std::ops::Range;

use super::expr::{self, Root, Wrap};
use super::graph::{OldGraph, Origin, member_path};

/// A text rewritten, with what changed and what could not be.
#[derive(Debug, Clone, Default)]
pub struct Rewritten {
    pub text: String,
    pub changes: Vec<String>,
    pub notes: Vec<String>,
    pub unresolved: Vec<String>,
}

impl Rewritten {
    fn absorb(&mut self, other: Rewritten) {
        self.changes.extend(other.changes);
        self.notes.extend(other.notes);
        self.unresolved.extend(other.unresolved);
    }
}

/// How a reference is read where it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// A pipeline node: `$nodes` is available, `input` is its payload.
    Node,
    /// A node whose `$input` meant something else in 0.10 (`logic.reduce
    /// --init-expr`): any payload reference is reported.
    ChangedScope,
}

/// Every `{{ … }}` of a config string rewritten, as read at node `reader`.
pub fn rewrite_template(graph: &OldGraph, reader: usize, text: &str) -> Rewritten {
    let mut out = Rewritten { text: text.to_string(), ..Default::default() };
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();
    for range in expr::template_expressions(text) {
        let inner = &text[range.clone()];
        let rewritten = rewrite_code(graph, reader, inner, Wrap::Expression, Scope::Node);
        if rewritten.text != inner {
            edits.push((range, rewritten.text.clone()));
        }
        out.absorb(rewritten);
    }
    out.text = expr::apply_edits(text, edits);
    out
}

/// One expression or script body rewritten, as read at node `reader`.
pub fn rewrite_code(graph: &OldGraph, reader: usize, source: &str, wrap: Wrap, scope: Scope) -> Rewritten {
    let mut out = Rewritten { text: source.to_string(), ..Default::default() };
    let scan = match expr::scan(source, wrap) {
        Ok(scan) => scan,
        Err(why) => {
            if source.contains("input") || source.contains("$nodes") {
                out.unresolved.push(format!("`{}` {why}, so its payload references cannot be read", short(source)));
            }
            return out;
        }
    };
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();
    // A script that reads its payload as a whole (`{ ...input }`,
    // `input[key]`) is given the 0.10 payload back: one line rebuilds it from
    // where each key went, and the code reads it as it always did.
    let shim = wrap == Wrap::FunctionBody && scan.bare.iter().any(|b| b.input);
    if shim {
        match graph.whole_input_expr(reader, "input", Some("ctx.nodes")) {
            Ok((whole, dropped)) => {
                out.changes.push(format!("the script reads its payload as a whole: `input = {whole};` rebuilds the 0.10 payload first"));
                if !dropped.is_empty() {
                    out.notes.push(format!("the rebuilt payload leaves out {} (no 0.11 equivalent)", dropped.join(", ")));
                }
                edits.push((0..0, format!("input = {whole};\n")));
            }
            Err(why) => {
                out.unresolved.push(format!(
                    "the script reads its payload as a whole (`{}`), and the 0.10 payload cannot be rebuilt: {why}",
                    short(&scan.bare.iter().find(|b| b.input).map(|b| b.text.clone()).unwrap_or_default())
                ));
                return out;
            }
        }
    }
    for reference in &scan.references {
        if shim && reference.root == Root::Input {
            continue;
        }
        let head_text = &source[reference.head.clone()];
        if scope == Scope::ChangedScope && reference.root == Root::Input {
            out.unresolved.push(format!(
                "`{head_text}` in --init-expr read the first item in 0.10; 0.11 gives --initial the payload the loop received"
            ));
            continue;
        }
        let nodes_base = if wrap == Wrap::FunctionBody { "ctx.nodes" } else { "$nodes" };
        match place(graph, reader, &reference.root, &reference.root_text, &reference.path, reference.optional, nodes_base) {
            Placed::Same => {}
            Placed::New(text) => {
                if text != head_text {
                    out.changes.push(format!("`{head_text}` → `{text}`"));
                    edits.push((reference.head.clone(), text));
                }
            }
            Placed::Dead(why) => out.notes.push(format!("`{head_text}` read nothing in 0.10 and reads nothing in 0.11 ({why})")),
            Placed::Unresolved(why) => out.unresolved.push(format!("`{head_text}`: {why}")),
        }
    }
    for bare in &scan.bare {
        if shim && bare.input {
            continue;
        }
        if bare.whole && bare.input && scope == Scope::Node {
            match graph.whole_input_expr(reader, &bare.text, Some("$nodes")) {
                Ok((text, dropped)) => {
                    out.changes.push(format!("`{}` → `{text}`", bare.text));
                    if !dropped.is_empty() {
                        out.notes.push(format!("`{}` leaves out {} (no 0.11 equivalent)", bare.text, dropped.join(", ")));
                    }
                    edits.push((bare.at.clone(), text));
                }
                Err(why) => out.unresolved.push(format!(
                    "`{}` reads the whole payload, and the 0.10 payload cannot be rebuilt in 0.11: {why}",
                    bare.text
                )),
            }
        } else {
            out.unresolved.push(format!("`{}` reads the payload by a computed key, which cannot be traced", short(&bare.text)));
        }
    }
    out.text = expr::apply_edits(source, edits);
    out
}

enum Placed {
    /// Nothing to change.
    Same,
    New(String),
    /// Read nothing in 0.10 and still reads nothing.
    Dead(String),
    Unresolved(String),
}

/// Where a reference read at `reader` is in 0.11.
fn place(graph: &OldGraph, reader: usize, root: &Root, root_text: &str, path: &[String], optional: bool, nodes_base: &str) -> Placed {
    match root {
        Root::Input => {
            if path.is_empty() {
                return Placed::Same;
            }
            match graph.resolve_input(reader, path) {
                Ok(origin) => {
                    if nodes_base == "$nodes" && graph.ambiguous(reader, &origin) {
                        // Two upstream nodes answer the key: name the one meant.
                        let own = member_path(nodes_base, &[graph.nodes[origin.producer].id.clone()]);
                        return Placed::New(origin.render_with(&own, optional));
                    }
                    Placed::New(render(graph, &origin, root_text, None, optional, nodes_base))
                }
                Err(failure) if failure.dead => {
                    if graph.answered_upstream(reader, &path[0]) {
                        Placed::Unresolved(format!(
                            "read nothing in 0.10, but an upstream node answers `{}` in 0.11",
                            path[0]
                        ))
                    } else {
                        Placed::Dead(failure.why)
                    }
                }
                Err(failure) => Placed::Unresolved(failure.why),
            }
        }
        Root::Nodes(id) => {
            let Some(node) = graph.index(id) else {
                return Placed::Unresolved(format!("no node `{id}` in this pipeline"));
            };
            let base = root_text.to_string();
            // A run input's value was `$nodes.<id>` itself; it is now under
            // its name.
            if graph.nodes[node].kind.starts_with("n.input.") {
                let Some(name) = graph.nodes[node].config.get("name").and_then(|v| v.as_str()) else {
                    return Placed::Unresolved(format!("run input `{id}` has no name"));
                };
                let mut segments = vec![id.clone()];
                segments.extend(super::graph::split(name));
                segments.extend(path.iter().cloned());
                return Placed::New(if optional {
                    super::graph::optional_path(&member_path(&base, &segments[..1]), &segments[1..])
                } else {
                    member_path(&base, &segments)
                });
            }
            if path.is_empty() {
                let own = member_path(&base, &[id.clone()]);
                return match graph.whole_output_expr(node, &own, Some(&base)) {
                    Ok((text, _)) => Placed::New(text),
                    Err(why) => Placed::Unresolved(format!("`{own}` reads node `{id}`'s whole 0.10 answer, which cannot be rebuilt: {why}")),
                };
            }
            let resolution = graph.resolve_output(node, path);
            match resolution {
                Ok(origin) => {
                    let producer_root = if graph.shadowed(&origin, Some(node)) {
                        member_path(&base, &[graph.nodes[origin.producer].id.clone()])
                    } else {
                        member_path(&base, &[id.clone()])
                    };
                    Placed::New(origin.render_with(&producer_root, optional))
                }
                Err(failure) if failure.dead => Placed::Dead(failure.why),
                Err(failure) => Placed::Unresolved(failure.why),
            }
        }
    }
}

/// An origin written from a reader's `input` (or `$nodes.<id>` when a node
/// between answers the same key in 0.11).
fn render(graph: &OldGraph, origin: &Origin, root_text: &str, also: Option<usize>, optional: bool, nodes_base: &str) -> String {
    if graph.shadowed(origin, also) {
        origin.render_with(&member_path(nodes_base, &[graph.nodes[origin.producer].id.clone()]), optional)
    } else {
        origin.render_with(root_text, optional)
    }
}

fn short(text: &str) -> String {
    let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() > 80 {
        format!("{}…", one_line.chars().take(80).collect::<String>())
    } else {
        one_line
    }
}

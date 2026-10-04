//! A 0.10 graph read the way the 0.10 engine ran it, so that every
//! `input.<key>` can be traced to the node whose answer held that key.
//!
//! A 0.10 node passed its payload on, replaced it with its own answer, or
//! merged keys into it (the per-kind model is `kinds`). A 0.11 node always
//! keeps the payload and adds one key, its noun. A reference is therefore
//! resolved by walking upstream from the reader to the node that produced
//! the key in 0.10, and rewritten to where that node's answer sits in 0.11
//! (`rows` after a Postgres query → `query.rows`).

use std::collections::BTreeSet;

use serde_json::Value;

/// Where an old key (or key path) of a node's 0.10 answer is in 0.11.
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    /// A path inside the producer's 0.11 output payload.
    Path(Vec<String>),
    /// A JavaScript expression computing the old value; `@` stands for the
    /// producer's payload (`input` or `$nodes.<id>`).
    Expr(String),
    /// The old key has no 0.11 equivalent; the reason says why.
    Gone(String),
}

/// One old key path and where it went.
#[derive(Debug, Clone, PartialEq)]
pub struct Mapping {
    pub old: Vec<String>,
    pub new: Target,
}

/// What a replaced payload held besides the mapped keys.
#[derive(Debug, Clone, PartialEq)]
pub enum Rest {
    /// Nothing: any other key read nothing in 0.10.
    Dead,
    /// Anything: any other key `k` is `whole.k` (a script's return value).
    Open,
    /// More than the mapped keys, with no 0.11 equivalent.
    Gone(String),
}

/// What a 0.10 node did to the payload it passed on.
#[derive(Debug, Clone, PartialEq)]
pub enum OldOutput {
    /// Passed the payload on unchanged.
    Pass,
    /// Replaced the payload with its own answer. `keys` maps the answer's
    /// keys; `whole` is the 0.11 path equal to the whole 0.10 answer.
    Replace { keys: Vec<Mapping>, whole: Option<Vec<String>>, rest: Rest },
    /// Added `keys` and kept the rest of the payload.
    Merge { keys: Vec<Mapping> },
    /// The engine's failure on an `error` pin. 0.10 delivered
    /// `{ input: <the failing node's input>, error, __zf_retry }`; 0.11
    /// delivers the failing node's input kept, plus its key answering
    /// `{ ok: false, error: { code, message } }` (`node-conventions.md` §6).
    /// `input.<k>` is read at the failing node's input, `error.<k>` under
    /// its key.
    ErrorEnvelope,
    /// Cannot be modelled; a reference through it is not rewritten.
    Unknown(String),
}

impl OldOutput {
    /// `keys`: `(old dotted path, new dotted path)`; a new path starting
    /// with `=` is an expression over `@`.
    pub fn replace(keys: &[(&str, &str)], whole: Option<&str>) -> Self {
        OldOutput::Replace { keys: mappings(keys), whole: whole.map(split), rest: Rest::Dead }
    }

    /// A replace whose answer holds more than `keys`, none of it mapped.
    pub fn replace_more(keys: &[(&str, &str)], why: &str) -> Self {
        OldOutput::Replace { keys: mappings(keys), whole: None, rest: Rest::Gone(why.to_string()) }
    }

    pub fn open(whole: &str) -> Self {
        OldOutput::Replace { keys: Vec::new(), whole: Some(split(whole)), rest: Rest::Open }
    }

    pub fn merge(keys: &[(&str, &str)]) -> Self {
        OldOutput::Merge { keys: mappings(keys) }
    }

    pub fn unknown(why: &str) -> Self {
        OldOutput::Unknown(why.to_string())
    }
}

/// `(old, new)`: `new` is a dotted 0.11 path, `=<expression over @>`, or
/// `!<why>` for a key with no 0.11 equivalent.
pub fn mappings(keys: &[(&str, &str)]) -> Vec<Mapping> {
    keys.iter()
        .map(|(old, new)| Mapping {
            old: split(old),
            new: if let Some(expr) = new.strip_prefix('=') {
                Target::Expr(expr.to_string())
            } else if let Some(why) = new.strip_prefix('!') {
                Target::Gone(why.to_string())
            } else {
                Target::Path(split(new))
            },
        })
        .collect()
}

pub fn split(path: &str) -> Vec<String> {
    path.split('.').filter(|s| !s.is_empty()).map(str::to_string).collect()
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: usize,
    pub from_pin: String,
    pub to: usize,
    pub to_pin: String,
}

/// One node of the 0.10 graph.
#[derive(Debug, Clone)]
pub struct OldNode {
    pub id: String,
    pub kind: String,
    pub config: Value,
}

/// Where a key read at some point of the graph came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Origin {
    /// The node whose 0.10 answer held the key.
    pub producer: usize,
    /// Where the matched part of the old path is in 0.11.
    pub base: Target,
    /// The rest of the old path, read the same way inside it.
    pub rest: Vec<String>,
    /// The nodes between the producer and the reader, which in 0.11 each
    /// add their own key on top.
    pub via: BTreeSet<usize>,
}

impl Origin {
    /// The 0.11 path when the origin is a plain path.
    pub fn path(&self) -> Option<Vec<String>> {
        match &self.base {
            Target::Path(base) => {
                let mut out = base.clone();
                out.extend(self.rest.iter().cloned());
                Some(out)
            }
            Target::Expr(_) | Target::Gone(_) => None,
        }
    }

    /// The origin written from `root` (`input`, `$nodes.q`).
    pub fn render(&self, root: &str) -> String {
        self.render_with(root, false)
    }

    /// The origin written from `root`, every step `?.` when `optional`.
    pub fn render_with(&self, root: &str, optional: bool) -> String {
        let path = |root: &str, segments: &[String]| {
            if optional { optional_path(root, segments) } else { member_path(root, segments) }
        };
        match &self.base {
            Target::Path(_) | Target::Gone(_) => path(root, &self.path().unwrap_or_default()),
            Target::Expr(expr) => path(&format!("({})", expr.replace('@', root)), &self.rest),
        }
    }

    /// The first key the origin reads in 0.11, for the shadow check.
    pub fn first(&self) -> Option<String> {
        match &self.base {
            Target::Path(base) => base.first().or(self.rest.first()).cloned(),
            Target::Expr(expr) => {
                // `@.result.error` reads `result`.
                let after = expr.split("@.").nth(1)?;
                Some(after.split(|c: char| !(c.is_alphanumeric() || c == '_')).next()?.to_string())
            }
            Target::Gone(_) => None,
        }
    }
}

/// Why a reference could not be resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct Failure {
    /// The key was not in the 0.10 payload at all: the reference read
    /// nothing then.
    pub dead: bool,
    pub why: String,
}

pub type Resolution = Result<Origin, Failure>;

fn fail(why: String) -> Failure {
    Failure { dead: false, why }
}

/// How a whole-payload expression is written: the reader's payload, the
/// name `$nodes` has where it is written, and the reader's ancestors.
struct Whole<'a> {
    root: &'a str,
    nodes_base: Option<&'a str>,
    ancestors: &'a BTreeSet<usize>,
}

pub struct OldGraph {
    pub nodes: Vec<OldNode>,
    pub edges: Vec<Edge>,
    pub incoming: Vec<Vec<usize>>,
    pub outputs: Vec<OldOutput>,
    /// The key each node adds in 0.11 (`None` for a control node).
    pub nouns: Vec<Option<String>>,
    /// What each node's `error` pin carried in 0.10, when it can be said.
    pub error_outputs: Vec<OldOutput>,
    /// The key each node's failure is delivered under in 0.11
    /// (`nodes::failure_key`); `None` for a kind 0.11 does not have.
    pub failure_keys: Vec<Option<String>>,
}

impl OldGraph {
    pub fn index(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    /// Where `path` of the payload node `reader` receives came from.
    pub fn resolve_input(&self, reader: usize, path: &[String]) -> Resolution {
        self.resolve_input_seen(reader, path, &mut BTreeSet::new())
    }

    fn resolve_input_seen(&self, reader: usize, path: &[String], seen: &mut BTreeSet<usize>) -> Resolution {
        let key = path.join(".");
        let incoming = &self.incoming[reader];
        if incoming.is_empty() {
            return Err(fail(format!("`{key}` is read by node `{}`, which nothing feeds", self.nodes[reader].id)));
        }
        let mut found: Option<Origin> = None;
        let mut dead: Option<Failure> = None;
        for &edge_index in incoming {
            let edge = &self.edges[edge_index];
            let output = if edge.from_pin == "error" {
                self.error_outputs[edge.from].clone()
            } else {
                self.outputs[edge.from].clone()
            };
            let origin = match self.resolve_through(edge.from, &output, path, seen) {
                Ok(origin) => origin,
                Err(failure) if failure.dead => {
                    dead = Some(failure);
                    continue;
                }
                Err(failure) => return Err(failure),
            };
            match &mut found {
                None => found = Some(origin),
                Some(existing) if existing.producer == origin.producer && existing.base == origin.base && existing.rest == origin.rest => {
                    existing.via.extend(origin.via);
                }
                Some(existing) => {
                    return Err(fail(format!(
                        "`{key}` reaches node `{}` from two branches that hold it differently (`{}` and `{}`)",
                        self.nodes[reader].id, self.nodes[existing.producer].id, self.nodes[origin.producer].id
                    )));
                }
            }
        }
        match (found, dead) {
            (Some(origin), None) => Ok(origin),
            (Some(origin), Some(_)) => Err(fail(format!(
                "`{key}` reaches node `{}` on one branch (from `{}`) and is absent on another",
                self.nodes[reader].id, self.nodes[origin.producer].id
            ))),
            (None, Some(failure)) => Err(failure),
            (None, None) => Err(fail(format!("`{key}` has no source"))),
        }
    }

    /// Where `path` of the payload node `node` passed on came from.
    pub fn resolve_output(&self, node: usize, path: &[String]) -> Resolution {
        let output = self.outputs[node].clone();
        self.resolve_through(node, &output, path, &mut BTreeSet::new())
    }

    fn resolve_through(&self, node: usize, output: &OldOutput, path: &[String], seen: &mut BTreeSet<usize>) -> Resolution {
        let key = path.join(".");
        if !seen.insert(node) {
            return Err(fail(format!("`{key}` is read around a cycle through node `{}`", self.nodes[node].id)));
        }
        let matched = |keys: &[Mapping]| {
            keys.iter()
                .filter(|m| !m.old.is_empty() && path.starts_with(&m.old))
                .max_by_key(|m| m.old.len())
                .map(|m| Origin {
                    producer: node,
                    base: m.new.clone(),
                    rest: path[m.old.len()..].to_vec(),
                    via: BTreeSet::new(),
                })
        };
        let gone = |origin: Origin| -> Resolution {
            match &origin.base {
                Target::Gone(why) => Err(fail(format!(
                    "`{key}` (from node `{}`, {}) has no 0.11 equivalent: {why}",
                    self.nodes[node].id, self.nodes[node].kind
                ))),
                _ => Ok(origin),
            }
        };
        let result = match output {
            OldOutput::Replace { keys, whole, rest } => match (matched(keys), rest) {
                (Some(origin), _) => gone(origin),
                (None, Rest::Open) if whole.is_some() => Ok(Origin {
                    producer: node,
                    base: Target::Path(whole.clone().unwrap_or_default()),
                    rest: path.to_vec(),
                    via: BTreeSet::new(),
                }),
                (None, Rest::Gone(why)) => Err(fail(format!(
                    "`{key}` was in what node `{}` ({}) answered in 0.10 and has no 0.11 equivalent: {why}",
                    self.nodes[node].id, self.nodes[node].kind
                ))),
                (None, _) => Err(Failure {
                    dead: true,
                    why: format!(
                        "`{key}` is not in what node `{}` ({}) answered in 0.10 — the reference read nothing then",
                        self.nodes[node].id, self.nodes[node].kind
                    ),
                }),
            },
            OldOutput::Merge { keys } => match matched(keys) {
                Some(origin) => gone(origin),
                None => self.resolve_input_seen(node, path, seen).map(|mut origin| {
                    origin.via.insert(node);
                    origin
                }),
            },
            OldOutput::Pass => self.resolve_input_seen(node, path, seen).map(|mut origin| {
                origin.via.insert(node);
                origin
            }),
            OldOutput::ErrorEnvelope => match path.split_first() {
                // The failing node's input is the payload itself in 0.11,
                // under the failing node's own key.
                Some((first, rest)) if first == "input" && !rest.is_empty() => {
                    self.resolve_input_seen(node, rest, seen).map(|mut inner| {
                        inner.via.insert(node);
                        inner
                    })
                }
                Some((first, rest)) if first == "error" => match self.failure_keys[node].as_deref() {
                    Some(failure) => match rest.first().map(String::as_str) {
                        None | Some("code" | "message") => {
                            let mut base = vec![failure.to_string(), "error".to_string()];
                            base.extend(rest.iter().cloned());
                            Ok(Origin { producer: node, base: Target::Path(base), rest: Vec::new(), via: BTreeSet::new() })
                        }
                        Some(other) => Err(fail(format!(
                            "`{key}`: a failure answers only `code` and `message` in 0.11, not `{other}` — the run record names the failing node"
                        ))),
                    },
                    None => Err(fail(format!("`{key}` on the error pin of `{}`, a kind 0.11 does not have", self.nodes[node].id))),
                },
                Some((first, _)) if first == "__zf_retry" => Ok(Origin {
                    producer: node,
                    base: Target::Path(path.to_vec()),
                    rest: Vec::new(),
                    via: BTreeSet::new(),
                }),
                Some((first, _)) if first == "input" => Err(fail(format!(
                    "`{key}`: the failing node's whole input is the payload itself in 0.11, beside the failure under its key"
                ))),
                _ => Err(Failure {
                    dead: true,
                    why: format!("`{key}` is not in the failure envelope of node `{}`", self.nodes[node].id),
                }),
            },
            OldOutput::Unknown(why) => Err(fail(format!(
                "`{key}` passes through node `{}` ({}), whose 0.10 answer cannot be mapped: {why}",
                self.nodes[node].id, self.nodes[node].kind
            ))),
        };
        seen.remove(&node);
        result
    }

    /// The whole payload node `reader` receives, when it is exactly one
    /// node's whole 0.10 answer (`{{ input }}` after a script).
    pub fn resolve_whole_input(&self, reader: usize) -> Resolution {
        let [edge_index] = self.incoming[reader].as_slice() else {
            return Err(fail(format!("the whole payload of node `{}` cannot be traced to one node", self.nodes[reader].id)));
        };
        let edge = &self.edges[*edge_index];
        self.resolve_whole_from(edge.from, edge.from_pin == "error", reader)
    }

    /// The whole payload node `node` passed on, as one 0.11 path.
    pub fn resolve_whole_output(&self, node: usize) -> Resolution {
        self.resolve_whole_from(node, false, node)
    }

    fn resolve_whole_from(&self, start: usize, error_pin: bool, reader: usize) -> Resolution {
        let mut via = BTreeSet::new();
        let mut node = start;
        let mut output = if error_pin { self.error_outputs[node].clone() } else { self.outputs[node].clone() };
        loop {
            match &output {
                OldOutput::Replace { whole: Some(whole), .. } => {
                    return Ok(Origin { producer: node, base: Target::Path(whole.clone()), rest: Vec::new(), via });
                }
                OldOutput::Pass => {
                    let [next] = self.incoming[node].as_slice() else { break };
                    via.insert(node);
                    let edge = &self.edges[*next];
                    if via.contains(&edge.from) {
                        break;
                    }
                    node = edge.from;
                    output = if edge.from_pin == "error" {
                        self.error_outputs[node].clone()
                    } else {
                        self.outputs[node].clone()
                    };
                }
                _ => break,
            }
        }
        Err(fail(format!(
            "the whole payload at node `{}` was more than one node's 0.10 answer",
            self.nodes[reader].id
        )))
    }

    /// The whole payload node `reader` receives in 0.10, as a 0.11
    /// expression: `input.webhook.body`-style reads of where each old key
    /// went, composed the way 0.10 composed them (`{ ...upstream, key }`
    /// for a merge). `root` names the reader's payload (`input`);
    /// `nodes_base` (`$nodes`, `ctx.nodes`) names an answer a later node
    /// hides, `None` where nothing can. Also answers the old keys that have
    /// no 0.11 equivalent and are left out.
    pub fn whole_input_expr(&self, reader: usize, root: &str, nodes_base: Option<&str>) -> Result<(String, Vec<String>), String> {
        let mut dropped = Vec::new();
        let ancestors = self.ancestors(reader);
        let expr = self.whole_in(reader, BTreeSet::new(), &Whole { root, nodes_base, ancestors: &ancestors }, &mut dropped, 0)?;
        Ok((expr, dropped))
    }

    /// The whole payload node `node` passed on in 0.10, written from its
    /// 0.11 output (`$nodes.<id>`), as [`Self::whole_input_expr`] does.
    pub fn whole_output_expr(&self, node: usize, root: &str, nodes_base: Option<&str>) -> Result<(String, Vec<String>), String> {
        let mut dropped = Vec::new();
        let mut via = BTreeSet::new();
        via.insert(node);
        let mut ancestors = self.ancestors(node);
        ancestors.insert(node);
        let expr = self.whole_out(node, via, &Whole { root, nodes_base, ancestors: &ancestors }, &mut dropped, 0)?;
        Ok((expr, dropped))
    }

    fn whole_in(&self, reader: usize, via: BTreeSet<usize>, w: &Whole<'_>, dropped: &mut Vec<String>, depth: usize) -> Result<String, String> {
        if depth > self.nodes.len() {
            return Err("the payload goes round a cycle".to_string());
        }
        let [edge_index] = self.incoming[reader].as_slice() else {
            return Err(format!("node `{}` receives more than one payload", self.nodes[reader].id));
        };
        let edge = &self.edges[*edge_index];
        if edge.from_pin == "error" {
            return Err(format!("node `{}` receives a failure envelope", self.nodes[reader].id));
        }
        self.whole_out(edge.from, via, w, dropped, depth + 1)
    }

    /// Where one target of `node`'s answer is read from, seen from the
    /// reader of the whole payload.
    fn whole_at(&self, node: usize, target: &Target, via: &BTreeSet<usize>, w: &Whole<'_>) -> Result<String, String> {
        let origin = Origin { producer: node, base: target.clone(), rest: Vec::new(), via: via.clone() };
        let first = origin.first().unwrap_or_default();
        let hidden = self.shadowed(&origin, None);
        let ambiguous = w.nodes_base == Some("$nodes")
            && w.ancestors.iter().filter(|&&n| self.nouns[n].as_deref() == Some(first.as_str())).count() > 1;
        if hidden || ambiguous {
            match w.nodes_base {
                Some(base) => Ok(origin.render(&member_path(base, &[self.nodes[node].id.clone()]))),
                None => Err(format!("a later node answers `{first}` too, and this code cannot read $nodes")),
            }
        } else {
            Ok(origin.render(w.root))
        }
    }

    /// An object literal holding `keys` (old paths → targets); with `base`,
    /// every level spreads what the base held there.
    fn whole_literal(
        &self,
        node: usize,
        keys: &[Mapping],
        base: Option<&str>,
        via: &BTreeSet<usize>,
        w: &Whole<'_>,
        dropped: &mut Vec<String>,
        prefix: &str,
    ) -> Result<String, String> {
        let mut firsts: Vec<String> = Vec::new();
        for key in keys {
            if let Some(first) = key.old.first()
                && !firsts.contains(first)
            {
                firsts.push(first.clone());
            }
        }
        let mut fields = Vec::new();
        for first in firsts {
            let direct = keys.iter().find(|k| k.old.len() == 1 && &k.old[0] == &first);
            let nested: Vec<Mapping> = keys
                .iter()
                .filter(|k| k.old.len() > 1 && &k.old[0] == &first)
                .map(|k| Mapping { old: k.old[1..].to_vec(), new: k.new.clone() })
                .collect();
            let shown = if prefix.is_empty() { first.clone() } else { format!("{prefix}.{first}") };
            match direct.map(|d| &d.new) {
                Some(Target::Gone(_)) if nested.iter().all(|n| matches!(n.new, Target::Gone(_))) => dropped.push(shown),
                Some(target @ (Target::Path(_) | Target::Expr(_))) => {
                    for n in nested.iter().filter(|n| matches!(n.new, Target::Gone(_))) {
                        dropped.push(format!("{shown}.{}", n.old.join(".")));
                    }
                    fields.push(format!("{}: {}", js_key(&first), self.whole_at(node, target, via, w)?));
                }
                _ => {
                    let inner_base = base.map(|b| member_path(b, &[first.clone()]));
                    let inner = self.whole_literal(node, &nested, inner_base.as_deref(), via, w, dropped, &shown)?;
                    fields.push(format!("{}: {inner}", js_key(&first)));
                }
            }
        }
        Ok(match base {
            Some(base) if fields.is_empty() => base.to_string(),
            Some(base) => format!("({{ ...{base}, {} }})", fields.join(", ")),
            None => format!("({{ {} }})", fields.join(", ")),
        })
    }

    fn whole_out(&self, node: usize, via: BTreeSet<usize>, w: &Whole<'_>, dropped: &mut Vec<String>, depth: usize) -> Result<String, String> {
        match &self.outputs[node] {
            OldOutput::Pass => {
                let mut via = via.clone();
                via.insert(node);
                self.whole_in(node, via, w, dropped, depth + 1)
            }
            OldOutput::Replace { whole: Some(whole), .. } => self.whole_at(node, &Target::Path(whole.clone()), &via, w),
            OldOutput::Replace { keys, whole: None, rest: Rest::Dead } => self.whole_literal(node, keys, None, &via, w, dropped, ""),
            OldOutput::Replace { rest: Rest::Gone(why), .. } => Err(format!("node `{}`: {why}", self.nodes[node].id)),
            OldOutput::Replace { .. } => Err(format!("node `{}` answered an open set of keys", self.nodes[node].id)),
            OldOutput::Merge { keys } => {
                let mut inner_via = via.clone();
                inner_via.insert(node);
                let upstream = self.whole_in(node, inner_via, w, dropped, depth + 1)?;
                self.whole_literal(node, keys, Some(&upstream), &via, w, dropped, "")
            }
            OldOutput::ErrorEnvelope => Err("a failure envelope".to_string()),
            OldOutput::Unknown(why) => Err(format!("node `{}` ({}): {why}", self.nodes[node].id, self.nodes[node].kind)),
        }
    }

    /// Whether a node between the producer and the reader (or `also`, the
    /// node a `$nodes.<id>` reads) adds in 0.11 the key the origin starts
    /// with — then that key would read the later node's answer.
    pub fn shadowed(&self, origin: &Origin, also: Option<usize>) -> bool {
        let Some(first) = origin.first() else { return false };
        origin
            .via
            .iter()
            .copied()
            .chain(also)
            .filter(|&n| n != origin.producer)
            .any(|n| self.nouns[n].as_deref() == Some(first.as_str()))
    }

    /// Whether, at `reader`, more than one upstream node answers the key the
    /// origin starts with in 0.11 — `input.<key>` then reads the later one
    /// in the text, and the check asks for `$nodes.<id>` instead.
    pub fn ambiguous(&self, reader: usize, origin: &Origin) -> bool {
        let Some(first) = origin.first() else { return false };
        self.ancestors(reader).into_iter().filter(|&n| self.nouns[n].as_deref() == Some(first.as_str())).count() > 1
    }

    /// Every node with a path to `node`.
    pub fn ancestors(&self, node: usize) -> BTreeSet<usize> {
        let mut out = BTreeSet::new();
        let mut stack = vec![node];
        while let Some(at) = stack.pop() {
            for &edge_index in &self.incoming[at] {
                let from = self.edges[edge_index].from;
                if out.insert(from) {
                    stack.push(from);
                }
            }
        }
        out
    }

    /// Whether some node upstream of `reader` answers `key` in 0.11.
    pub fn answered_upstream(&self, reader: usize, key: &str) -> bool {
        self.ancestors(reader).into_iter().any(|n| self.nouns[n].as_deref() == Some(key))
    }

    /// Whether the incoming edges of `node` come from branches only one of
    /// which runs: for every two of them some branching node (or a node's
    /// `out` against its `error`) is passed through on different pins by
    /// every path that reaches them.
    pub fn exclusive_join(&self, node: usize, branching: &dyn Fn(&str) -> bool) -> bool {
        let incoming = &self.incoming[node];
        let requirements: Vec<BTreeSet<(usize, String)>> =
            incoming.iter().map(|&e| self.requirements(e, branching)).collect();
        for i in 0..incoming.len() {
            for j in i + 1..incoming.len() {
                let apart = requirements[i].iter().any(|(b, p)| requirements[j].iter().any(|(c, q)| b == c && p != q));
                if !apart {
                    return false;
                }
            }
        }
        true
    }

    /// The `(branching node, pin)` every path from an entry to edge `edge`
    /// passes through.
    fn requirements(&self, edge: usize, branching: &dyn Fn(&str) -> bool) -> BTreeSet<(usize, String)> {
        let mut out = BTreeSet::new();
        let live: Vec<usize> = self.incoming.iter().flatten().copied().collect();
        for (b, node) in self.nodes.iter().enumerate() {
            let routes_errors = live.iter().any(|&e| self.edges[e].from == b && self.edges[e].from_pin == "error");
            if !(branching(&node.kind) || routes_errors) {
                continue;
            }
            let pins: BTreeSet<String> =
                live.iter().filter(|&&e| self.edges[e].from == b).map(|&e| self.edges[e].from_pin.clone()).collect();
            for pin in pins {
                if !self.reachable_without(edge, b, &pin) {
                    out.insert((b, pin));
                }
            }
        }
        out
    }

    /// Whether `edge` can still be reached from an entry with the edges
    /// leaving `node` on `pin` taken away.
    fn reachable_without(&self, edge: usize, node: usize, pin: &str) -> bool {
        let cut = |e: usize| self.edges[e].from == node && self.edges[e].from_pin == pin;
        if cut(edge) {
            return false;
        }
        let mut seen = vec![false; self.nodes.len()];
        let mut stack: Vec<usize> = (0..self.nodes.len()).filter(|&n| self.incoming[n].is_empty()).collect();
        for &n in &stack {
            seen[n] = true;
        }
        while let Some(at) = stack.pop() {
            for (e, edge_ref) in self.edges.iter().enumerate() {
                if edge_ref.from != at || cut(e) || !self.incoming[edge_ref.to].contains(&e) {
                    continue;
                }
                if !seen[edge_ref.to] {
                    seen[edge_ref.to] = true;
                    stack.push(edge_ref.to);
                }
            }
        }
        seen[self.edges[edge].from]
    }

    /// The node ids on a cycle, if the graph has one.
    pub fn cycle(&self) -> Option<Vec<String>> {
        fn visit(g: &OldGraph, at: usize, state: &mut [u8], stack: &mut Vec<usize>) -> Option<Vec<String>> {
            state[at] = 1;
            stack.push(at);
            for edge in g.edges.iter().filter(|e| e.from == at) {
                match state[edge.to] {
                    1 => {
                        let start = stack.iter().position(|&s| s == edge.to).unwrap_or(0);
                        return Some(stack[start..].iter().map(|&i| g.nodes[i].id.clone()).collect());
                    }
                    0 => {
                        if let Some(found) = visit(g, edge.to, state, stack) {
                            return Some(found);
                        }
                    }
                    _ => {}
                }
            }
            stack.pop();
            state[at] = 2;
            None
        }
        let mut state = vec![0u8; self.nodes.len()];
        let mut stack = Vec::new();
        (0..self.nodes.len()).find_map(|start| {
            if state[start] == 0 { visit(self, start, &mut state, &mut stack) } else { None }
        })
    }
}

/// `segments` written as a JavaScript member path after `root`
/// (`input.query.rows`, `$nodes["geo-convert"].dataset`).
pub fn member_path(root: &str, segments: &[String]) -> String {
    let mut out = root.to_string();
    for segment in segments {
        if is_identifier(segment) {
            out.push('.');
            out.push_str(segment);
        } else if !segment.is_empty() && segment.bytes().all(|b| b.is_ascii_digit()) {
            out.push_str(&format!("[{segment}]"));
        } else {
            out.push_str(&format!("[{}]", serde_json::to_string(segment).unwrap_or_default()));
        }
    }
    out
}

/// `segments` after `root` with optional access at every step.
pub fn optional_path(root: &str, segments: &[String]) -> String {
    let mut out = root.to_string();
    for segment in segments {
        if is_identifier(segment) {
            out.push_str("?.");
            out.push_str(segment);
        } else if !segment.is_empty() && segment.bytes().all(|b| b.is_ascii_digit()) {
            out.push_str(&format!("?.[{segment}]"));
        } else {
            out.push_str(&format!("?.[{}]", serde_json::to_string(segment).unwrap_or_default()));
        }
    }
    out
}

pub fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some(c) if c == '_' || c == '$' || c.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c == '$' || c.is_ascii_alphanumeric())
}

/// An object key as written in a JavaScript literal.
pub fn js_key(name: &str) -> String {
    if is_identifier(name) { name.to_string() } else { serde_json::to_string(name).unwrap_or_default() }
}

/// `$nodes.<id>` as written for a node id.
pub fn nodes_root(id: &str) -> String {
    member_path("$nodes", &[id.to_string()])
}

//! References to keys no upstream node answers (`node-conventions.md` §4).
//!
//! Every node adds one key — its noun — and keeps the rest (§6), so the keys
//! a node's `input` holds are known before anything runs: what its upstream
//! nodes answer, along the edges that reach it. A `logic.foreach` item run
//! starts from `{ item, index, count }` (plus the payload under
//! `--keep-input`); a loop's close runs on the payload the foreach received.
//! A node whose answer cannot be known here (a run input named by an
//! expression, a kind outside the catalogue, a fragment's first node) makes
//! everything after it open, and an open payload is never judged.
//!
//! These are warnings for now: `pipeline_check` reports them, a save does
//! not refuse them yet.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde_json::Value;

use super::Problem;
use super::suggest::did_you_mean;
use crate::pipeline::model::{PipelineGraph, PipelineNode};
use crate::pipeline::nodes::answer_key;
use crate::pipeline::nodes::basic::logic;
use crate::pipeline::NodeDefinition;

/// The pin every node has, where the engine delivers its failure.
const ERROR_PIN: &str = "error";

/// The keys a payload holds, each with the nodes that answered it; `open`
/// when some of it cannot be known; `failed`, the nodes whose `:error` path
/// every way here runs on — their `$nodes.<id>` is null here.
#[derive(Debug, Clone, Default)]
struct Keys {
    keys: BTreeMap<String, BTreeSet<usize>>,
    open: bool,
    failed: BTreeSet<usize>,
}

impl Keys {
    fn merge(&mut self, other: &Keys) {
        self.open |= other.open;
        for (key, from) in &other.keys {
            self.keys.entry(key.clone()).or_default().extend(from);
        }
    }
}

/// What a node adds to the payload: `Ok(Some(key))`, `Ok(None)` for a node
/// that passes its payload on, `Err(())` when it cannot be known.
fn adds(node: &PipelineNode, def: Option<&NodeDefinition>) -> Result<Option<String>, ()> {
    let kind = node.kind.as_str();
    if def.is_none() && !kind.starts_with("x.") {
        return Err(());
    }
    if kind.starts_with("input.") {
        return match node.config.get("name").and_then(Value::as_str).map(str::trim) {
            Some(name) if !name.is_empty() && !name.contains("{{") => Ok(Some(name.to_string())),
            _ => Err(()),
        };
    }
    match answer_key(kind) {
        Some(key) => Ok(Some(key)),
        None if kind.starts_with("logic.") || kind == "web.response.send" => Ok(None),
        None => Err(()),
    }
}

/// The key a node's failure arrives under on its `:error` pin.
fn failure_key_of(node: &PipelineNode) -> String {
    crate::pipeline::nodes::failure_key(&node.kind, node.config.get("name").and_then(Value::as_str))
}

/// A reference found in a node's config: the flag holding it, and what it reads.
struct Read {
    flag: String,
    text: String,
    /// `None` for `input.<key>`, the node id for `$nodes.<id>.<key>`.
    node: Option<String>,
    key: String,
}

fn reads_in(text: &str) -> Vec<(String, Option<String>, String)> {
    static READS: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = READS.get_or_init(|| {
        regex::Regex::new(r"(\$nodes\s*\??\.\s*([A-Za-z_][\w]*)|input)\s*\??\.\s*([A-Za-z_$][\w$]*)").expect("reads")
    });
    let mut out = Vec::new();
    for caps in re.captures_iter(text) {
        let whole = caps.get(0).expect("match");
        let before = text[..whole.start()].chars().last();
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '$') {
            continue;
        }
        out.push((whole.as_str().to_string(), caps.get(2).map(|m| m.as_str().to_string()), caps[3].to_string()));
    }
    out
}

/// The `{{ … }}` spans of a text.
fn expressions(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else { break };
        out.push(&after[..end]);
        rest = &after[end + 2..];
    }
    out
}

fn collect_strings<'a>(value: &'a Value, out: &mut Vec<&'a str>) {
    match value {
        Value::String(s) => out.push(s),
        Value::Array(items) => items.iter().for_each(|v| collect_strings(v, out)),
        Value::Object(map) => map.values().for_each(|v| collect_strings(v, out)),
        _ => {}
    }
}

/// Every `input.<key>` and `$nodes.<id>.<key>` a node's config reads: in
/// its `{{ }}` values, and in the whole of a flag whose value is an
/// expression (`logic.if --when`, `logic.foreach --from`).
fn reads(node: &PipelineNode, def: Option<&NodeDefinition>) -> Vec<Read> {
    let Some(map) = node.config.as_object() else { return Vec::new() };
    let mut out = Vec::new();
    for (key, value) in map {
        if matches!(key.as_str(), "ui" | "preview" | "title") {
            continue;
        }
        let flag = def.and_then(|d| d.dsl_flags.iter().find(|f| &f.config_key == key));
        let name = flag.map(|f| f.flag.clone()).unwrap_or_else(|| format!("--{}", key.replace('_', "-")));
        let whole = flag.is_some_and(|f| f.value == "expression");
        let mut strings = Vec::new();
        collect_strings(value, &mut strings);
        for s in strings {
            let spans = if whole && !s.contains("{{") { vec![s] } else { expressions(s) };
            for span in spans {
                for (text, node_id, read_key) in reads_in(span) {
                    out.push(Read { flag: name.clone(), text, node: node_id, key: read_key });
                }
            }
        }
    }
    out
}

/// Reference problems of a graph, as warnings: a key no upstream node
/// answers (with the likeliest key it meant), and an `input.<key>` two
/// upstream nodes answer. A graph whose flow does not hold reports nothing
/// here; the flow check says why.
pub fn check_references(graph: &PipelineGraph, defs: &[NodeDefinition]) -> Vec<Problem> {
    let Ok(loops) = crate::pipeline::engines::basic::flow_loops(graph) else { return Vec::new() };
    let n = graph.nodes.len();
    let index: HashMap<&str, usize> = graph.nodes.iter().enumerate().map(|(i, node)| (node.id.as_str(), i)).collect();
    let def_of: Vec<Option<&NodeDefinition>> =
        graph.nodes.iter().map(|node| defs.iter().find(|d| d.kind == node.kind)).collect();

    // Counted edges: a retry's re-entry is never one.
    let mut incoming: Vec<Vec<(usize, &str)>> = vec![Vec::new(); n];
    for edge in &graph.edges {
        let (Some(&from), Some(&to)) = (index.get(edge.from_node.as_str()), index.get(edge.to_node.as_str())) else {
            continue;
        };
        if graph.nodes[from].kind == logic::retry::NODE_KIND && edge.from_pin == logic::retry::OUTPUT_PIN_RETRY {
            continue;
        }
        incoming[to].push((from, edge.from_pin.as_str()));
    }
    let Some(order) = topological(n, &incoming) else { return Vec::new() };

    let mut keys_in: Vec<Keys> = vec![Keys::default(); n];
    let mut keys_out: Vec<Keys> = vec![Keys::default(); n];
    for &i in &order {
        let node = &graph.nodes[i];
        let mut payload = Keys::default();
        if let Some(foreach) = loops.closes.get(&node.id).and_then(|f| index.get(f.as_str())) {
            payload = keys_in[*foreach].clone();
        } else if incoming[i].is_empty() {
            payload.open = !node.kind.starts_with("trigger.");
        } else {
            // A node failed on every way here only when each incoming
            // payload says so.
            let mut failed: Option<BTreeSet<usize>> = None;
            let mut meet = |set: &BTreeSet<usize>| {
                failed = Some(match failed.take() {
                    None => set.clone(),
                    Some(have) => have.intersection(set).copied().collect(),
                });
            };
            for &(from, pin) in &incoming[i] {
                if graph.nodes[from].kind == logic::foreach_::NODE_KIND && pin == logic::foreach_::OUTPUT_PIN_ITEM {
                    let mut item = Keys::default();
                    for key in ["item", "index", "count"] {
                        item.keys.entry(key.to_string()).or_default().insert(from);
                    }
                    if graph.nodes[from].config.get("keep_input").and_then(Value::as_bool).unwrap_or(false) {
                        item.merge(&keys_in[from]);
                    }
                    meet(&keys_in[from].failed);
                    payload.merge(&item);
                } else if pin == ERROR_PIN {
                    // A failure the engine delivers (§6): the payload the
                    // failing node received, kept, plus its key answering
                    // `{ ok: false, error }`. Its delivery replaces that
                    // key, so `input.<key>` is the failure alone, and
                    // `$nodes.<failing node>` is null from here.
                    let mut failure = keys_in[from].clone();
                    failure.keys.insert(failure_key_of(&graph.nodes[from]), BTreeSet::from([from]));
                    failure.failed.insert(from);
                    meet(&failure.failed);
                    payload.merge(&failure);
                } else {
                    meet(&keys_out[from].failed);
                    payload.merge(&keys_out[from]);
                }
            }
            payload.failed = failed.unwrap_or_default();
        }
        let mut out = payload.clone();
        match adds(node, def_of[i]) {
            Ok(Some(key)) => {
                out.keys.entry(key).or_default().insert(i);
            }
            Ok(None) => {}
            Err(()) => out.open = true,
        }
        keys_in[i] = payload;
        keys_out[i] = out;
    }

    let mut problems = Vec::new();
    for &i in &order {
        let node = &graph.nodes[i];
        for read in reads(node, def_of[i]) {
            if read.key.starts_with("__") {
                continue;
            }
            let message = match &read.node {
                None => input_problem(graph, &keys_in[i], i, &incoming[i], &def_of, &read),
                Some(id) => {
                    let Some(&at) = index.get(id.as_str()) else { continue };
                    if keys_in[i].failed.contains(&at) {
                        Some(failed_node_problem(graph, at, &read))
                    } else {
                        nodes_problem(graph, &keys_out[at], at, &read)
                    }
                }
            };
            if let Some(message) = message {
                let problem = Problem::node(&node.id, &node.kind, Some(&read.flag), message);
                // The same read written twice in one node is one warning.
                if !problems.contains(&problem) {
                    problems.push(problem);
                }
            }
        }
    }
    problems
}

fn input_problem(
    graph: &PipelineGraph,
    payload: &Keys,
    at: usize,
    incoming: &[(usize, &str)],
    def_of: &[Option<&NodeDefinition>],
    read: &Read,
) -> Option<String> {
    let id = graph.nodes[at].id.as_str();
    let key = read.key.as_str();
    if let Some(from) = payload.keys.get(key) {
        if from.len() < 2 {
            return None;
        }
        let paths: Vec<String> = from.iter().map(|&n| format!("`$nodes.{}.{key}`", graph.nodes[n].id)).collect();
        return Some(format!(
            "`{}` — {} upstream nodes answer `{key}` ({}); `input.{key}` holds the one later in the text. Name the one you mean with `$nodes`",
            read.text,
            from.len(),
            paths.join(" and ")
        ));
    }
    if payload.open {
        return None;
    }
    let mut message = format!("`{}` — no node upstream of `{id}` answers `{key}`", read.text);
    if let Some(better) = suggest_failure(graph, incoming, key) {
        message.push_str(&format!("; a failure arrives under the failing node's key — did you mean `{better}`?"));
    } else if let Some(better) = suggest_input(payload, incoming, def_of, key) {
        message.push_str(&format!("; did you mean `{better}`?"));
    } else if payload.keys.is_empty() {
        message.push_str("; its input is empty");
    }
    Some(message)
}

/// `input.error` read after an `:error` edge: the failure is under the
/// failing node's key, `input.<key>.error`.
fn suggest_failure(graph: &PipelineGraph, incoming: &[(usize, &str)], key: &str) -> Option<String> {
    if key != "error" {
        return None;
    }
    let from = incoming.iter().rev().find(|(_, pin)| *pin == ERROR_PIN).map(|&(from, _)| from)?;
    Some(format!("input.{}.error", failure_key_of(&graph.nodes[from])))
}

/// The likeliest key an `input.<key>` meant: the key nested in an upstream
/// answer (`input.body` → `input.webhook.body`), a near spelling, else the
/// answer of the node feeding this one.
fn suggest_input(
    payload: &Keys,
    incoming: &[(usize, &str)],
    def_of: &[Option<&NodeDefinition>],
    key: &str,
) -> Option<String> {
    let mut providers: Vec<(usize, &str)> =
        payload.keys.iter().flat_map(|(k, from)| from.iter().map(move |&n| (n, k.as_str()))).collect();
    providers.sort_by(|a, b| b.0.cmp(&a.0));
    for &(n, answer) in &providers {
        let nested = def_of[n]
            .and_then(|def| def.output_schema.pointer(&format!("/properties/{answer}/properties/{key}")))
            .is_some();
        if nested {
            return Some(format!("input.{answer}.{key}"));
        }
    }
    let names: Vec<&str> = payload.keys.keys().map(String::as_str).collect();
    if let Some(near) = did_you_mean(key, &names) {
        return Some(format!("input.{near}"));
    }
    // The node feeding this one: its answer is what a reader usually wants.
    let feeder = incoming.iter().map(|&(from, _)| from).max()?;
    providers
        .iter()
        .find(|(n, _)| *n == feeder)
        .or_else(|| providers.first())
        .map(|(_, answer)| format!("input.{answer}"))
}

/// `$nodes.<id>…` read on `<id>`'s own `:error` path: the node answered
/// nothing, so it is null there; its failure is in the payload, under its key.
fn failed_node_problem(graph: &PipelineGraph, at: usize, read: &Read) -> String {
    let id = graph.nodes[at].id.as_str();
    let key = failure_key_of(&graph.nodes[at]);
    let mut message = format!("`{}` — on the `:error` path of `{id}`, `$nodes.{id}` is null; read `input.{key}.error`", read.text);
    // `$nodes.<id>.<key>…` meant the failure under the same key.
    if read.key == key {
        message.push_str(&format!(" — did you mean `input.{key}`?"));
    }
    message
}

fn nodes_problem(graph: &PipelineGraph, answered: &Keys, at: usize, read: &Read) -> Option<String> {
    let key = read.key.as_str();
    if answered.open || answered.keys.contains_key(key) {
        return None;
    }
    let id = graph.nodes[at].id.as_str();
    let own: Option<&str> = answered.keys.iter().filter(|(_, from)| from.contains(&at)).map(|(k, _)| k.as_str()).next();
    let names: Vec<&str> = answered.keys.keys().map(String::as_str).collect();
    let better = did_you_mean(key, &names)
        .map(|near| format!("$nodes.{id}.{near}"))
        .or_else(|| own.map(|own| format!("$nodes.{id}.{own}.{key}")));
    let mut message = format!("`{}` — `{id}` answers no `{key}`", read.text);
    if let Some(own) = own {
        message.push_str(&format!(" (its answer is `{own}`)"));
    }
    if let Some(better) = better {
        message.push_str(&format!("; did you mean `{better}`?"));
    }
    Some(message)
}

/// Kahn's order over the counted edges, ties in text order; `None` for a
/// cycle (the flow check refuses it).
fn topological(n: usize, incoming: &[Vec<(usize, &str)>]) -> Option<Vec<usize>> {
    let mut remaining: Vec<usize> = incoming.iter().map(Vec::len).collect();
    let mut outgoing: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (to, list) in incoming.iter().enumerate() {
        for &(from, _) in list {
            outgoing[from].push(to);
        }
    }
    let mut ready: BTreeSet<usize> = (0..n).filter(|&i| remaining[i] == 0).collect();
    let mut order = Vec::with_capacity(n);
    while let Some(i) = ready.pop_first() {
        order.push(i);
        for &to in &outgoing[i] {
            remaining[to] -= 1;
            if remaining[to] == 0 {
                ready.insert(to);
            }
        }
    }
    (order.len() == n).then_some(order)
}

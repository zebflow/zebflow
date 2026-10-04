//! One 0.10 pipeline document rewritten as a 0.11 one.

use std::collections::BTreeSet;

use serde_json::{Map, Value, json};

use super::RewriteContext;
use super::expr::Wrap;
use super::graph::{Edge, OldGraph, OldNode, OldOutput};
use super::kinds::{self, Code};
use super::node::NodeRewrite;
use super::refs::{self, Scope};

/// One line of a rewrite, tied to the node it is about (empty for the whole
/// pipeline).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Item {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub node: String,
    pub text: String,
}

impl Item {
    pub fn new(node: &str, text: impl Into<String>) -> Self {
        Self { node: node.to_string(), text: text.into() }
    }
}

/// What `rewrite_pipeline` made of a document.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Rewrite {
    /// The 0.11 document; `None` when the input was not a 0.10 pipeline.
    pub new_json: Option<Value>,
    pub changes: Vec<Item>,
    /// Behaviour that differs in 0.11 and cannot be written away.
    pub notes: Vec<Item>,
    /// What the rewrite could not map with certainty.
    pub unresolved: Vec<Item>,
    /// What the owner should look at: the rewrite is safe as it stands, but
    /// answers less than 0.10 did. Never blocks the apply.
    pub review: Vec<Item>,
    /// The templates its response nodes render, by node id.
    pub templates: Vec<(String, String)>,
}

/// Whether a pipeline document is a 0.10 one: some node kind still starts
/// `n.` (every stored 0.10 kind did; no 0.11 kind does).
pub fn is_old_document(doc: &Value) -> bool {
    nodes_of(doc).iter().any(|n| n.get("kind").and_then(Value::as_str).is_some_and(kinds::is_old_kind))
}

fn nodes_of(doc: &Value) -> Vec<Value> {
    doc.pointer("/spec/nodes").and_then(Value::as_array).cloned().unwrap_or_default()
}

/// The 0.10 graph of a document, every node modelled.
pub fn old_graph(doc: &Value, context: &RewriteContext) -> Result<OldGraph, String> {
    let raw_nodes = doc.pointer("/spec/nodes").and_then(Value::as_array).ok_or("the document has no spec.nodes")?;
    let raw_edges = doc.pointer("/spec/edges").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut nodes = Vec::new();
    for raw in raw_nodes {
        let id = raw.get("id").and_then(Value::as_str).ok_or("a node has no id")?.to_string();
        let kind = raw.get("kind").and_then(Value::as_str).ok_or("a node has no kind")?.to_string();
        let config = raw.get("config").cloned().unwrap_or_else(|| json!({}));
        nodes.push(OldNode { id, kind, config });
    }
    let index = |id: &str| nodes.iter().position(|n| n.id == id);
    let mut edges = Vec::new();
    for raw in &raw_edges {
        let get = |k: &str| raw.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
        let (Some(from), Some(to)) = (index(&get("from_node")), index(&get("to_node"))) else {
            return Err(format!("an edge joins a node that does not exist ({} → {})", get("from_node"), get("to_node")));
        };
        edges.push(Edge { from, from_pin: get("from_pin"), to, to_pin: get("to_pin") });
    }
    let mut incoming = vec![Vec::new(); nodes.len()];
    for (i, edge) in edges.iter().enumerate() {
        if !is_reentry(&nodes, edge) {
            incoming[edge.to].push(i);
        }
    }
    let mut outputs = Vec::new();
    let mut error_outputs = Vec::new();
    let mut nouns = Vec::new();
    let mut failure_keys = Vec::new();
    for node in &nodes {
        let config = node.config.as_object().cloned().unwrap_or_default();
        match kinds::rule(&node.kind) {
            Some(rule) => {
                outputs.push((rule.output)(&config, context));
                error_outputs.push((rule.error_output)(&config));
                let new_kind = rule.kind_for(&node.kind, &config);
                nouns.push(noun_of(&new_kind, &config));
                failure_keys.push(Some(crate::pipeline::nodes::failure_key(
                    &new_kind,
                    config.get("name").and_then(Value::as_str),
                )));
            }
            None => {
                outputs.push(OldOutput::unknown("a kind 0.11 does not have"));
                error_outputs.push(OldOutput::unknown("a kind 0.11 does not have"));
                nouns.push(None);
                failure_keys.push(None);
            }
        }
    }
    Ok(OldGraph { nodes, edges, incoming, outputs, nouns, error_outputs, failure_keys })
}

/// A `logic.retry` edge back into the node it re-runs.
fn is_reentry(nodes: &[OldNode], edge: &Edge) -> bool {
    nodes[edge.from].kind == "n.logic.retry" && edge.from_pin == "retry"
}

/// The key a 0.11 kind adds to the payload.
fn noun_of(kind: &str, config: &Map<String, Value>) -> Option<String> {
    if kind.starts_with("input.") {
        return config.get("name").and_then(Value::as_str).and_then(|n| n.split('.').next()).map(str::to_string);
    }
    crate::pipeline::nodes::answer_key(kind)
}

const BRANCHING: &[&str] = &["n.logic.if", "n.logic.match", "n.logic.retry", "n.crypto", "n.auth.token.verify"];

/// Rewrites one 0.10 pipeline document as a 0.11 one.
pub fn rewrite_pipeline(old_json: &Value, context: &RewriteContext) -> Rewrite {
    let mut out = Rewrite::default();
    if !is_old_document(old_json) {
        return out;
    }
    let graph = match old_graph(old_json, context) {
        Ok(graph) => graph,
        Err(why) => {
            out.unresolved.push(Item::new("", why));
            return out;
        }
    };
    flow_checks(&graph, &mut out);

    let mut new_nodes = Vec::new();
    let mut pins: Vec<Vec<(String, String)>> = Vec::new();
    let raw_nodes = nodes_of(old_json);
    for (index, raw) in raw_nodes.iter().enumerate() {
        let id = graph.nodes[index].id.clone();
        let kind = graph.nodes[index].kind.clone();
        let Some(rule) = kinds::rule(&kind) else {
            let why = if kind.starts_with("n.x.") {
                format!("{kind} is a kind installed from a package; its 0.11 name (x.<package>.<noun>.<verb>) is the package author's choice — reinstall the 0.11 package and rewrite this node by hand")
            } else if kinds::is_old_kind(&kind) {
                format!("{kind} has no 0.11 equivalent")
            } else {
                format!("{kind} is not a 0.10 kind; a pipeline mixing 0.10 and 0.11 kinds is rewritten by hand")
            };
            out.unresolved.push(Item::new(&id, why));
            new_nodes.push(raw.clone());
            pins.push(Vec::new());
            continue;
        };
        let mut config = graph.nodes[index].config.as_object().cloned().unwrap_or_default();
        rewrite_expressions(&graph, index, &rule.code_keys, &mut config, &id, &mut out);
        let mut n = NodeRewrite::new(&graph, context, index, config);
        kinds::engine_keys(&mut n);
        (rule.rewrite)(&mut n);
        n.leftovers();
        out.changes.extend(n.changes.iter().map(|t| Item::new(&id, t.clone())));
        out.notes.extend(n.notes.iter().map(|t| Item::new(&id, t.clone())));
        out.unresolved.extend(n.unresolved.iter().map(|t| Item::new(&id, t.clone())));
        if n.kind == "web.response.send"
            && let Some(template) = n.config.get("template").and_then(Value::as_str)
        {
            out.templates.push((id.clone(), template.to_string()));
        }
        let mut node = raw.clone();
        if let Some(object) = node.as_object_mut() {
            object.insert("kind".to_string(), Value::String(n.kind.clone()));
            object.insert("config".to_string(), Value::Object(n.config.clone()));
        }
        new_nodes.push(node);
        pins.push(n.pins.clone());
    }

    let mut new_edges = Vec::new();
    for raw in old_json.pointer("/spec/edges").and_then(Value::as_array).cloned().unwrap_or_default() {
        let mut edge = raw.clone();
        let from = raw.get("from_node").and_then(Value::as_str).unwrap_or_default();
        let pin = raw.get("from_pin").and_then(Value::as_str).unwrap_or_default();
        if let Some(index) = graph.index(from)
            && let Some((_, new_pin)) = pins[index].iter().find(|(old, _)| old == pin)
        {
            edge["from_pin"] = Value::String(new_pin.clone());
        }
        new_edges.push(edge);
    }

    answer_unwired_failures(&graph, &mut new_nodes, &mut new_edges, &mut out);
    review_unanswered_routes(&graph, &mut out);

    let mut doc = old_json.clone();
    doc["spec"]["nodes"] = Value::Array(new_nodes);
    doc["spec"]["edges"] = Value::Array(new_edges);
    out.new_json = Some(doc);
    out
}

/// The `{{ }}` and code in one node's config, rewritten as read there.
fn rewrite_expressions(
    graph: &OldGraph,
    index: usize,
    code_keys: &[(&str, Code)],
    config: &mut Map<String, Value>,
    id: &str,
    out: &mut Rewrite,
) {
    let keys: Vec<String> = config.keys().cloned().collect();
    for key in keys {
        if matches!(key.as_str(), "ui" | "title" | "preview" | "markup") {
            continue;
        }
        let code = code_keys.iter().find(|(k, _)| *k == key).map(|(_, c)| *c);
        let value = config.get(&key).cloned().unwrap_or(Value::Null);
        let new_value = match code {
            Some(Code::Expression) => value.as_str().map(|s| code_text(graph, index, s, Wrap::Expression, Scope::Node, &key, id, out)),
            Some(Code::ChangedScope) => {
                value.as_str().map(|s| code_text(graph, index, s, Wrap::Expression, Scope::ChangedScope, &key, id, out))
            }
            Some(Code::Body) => value.as_str().map(|s| code_text(graph, index, s, Wrap::FunctionBody, Scope::Node, &key, id, out)),
            Some(Code::ExpressionMap) => value.as_object().map(|map| {
                let mut new_map = Map::new();
                for (k, v) in map {
                    let text = v.as_str().map(|s| code_text(graph, index, s, Wrap::Expression, Scope::Node, &key, id, out));
                    new_map.insert(k.clone(), text.unwrap_or_else(|| v.clone()));
                }
                Value::Object(new_map)
            }),
            Some(Code::Foreign) => {
                if value.as_str().is_some_and(|s| s.contains("input.") || s.contains("$nodes")) {
                    out.notes.push(Item::new(id, format!("--{key} runs in the browser and is left as written")));
                }
                None
            }
            None => Some(rewrite_value(graph, index, &value, &key, id, out)),
        };
        if let Some(new_value) = new_value {
            config.insert(key, new_value);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn code_text(graph: &OldGraph, index: usize, text: &str, wrap: Wrap, scope: Scope, key: &str, id: &str, out: &mut Rewrite) -> Value {
    // A `{{ }}` written into a code flag is still a template.
    let rewritten = if text.contains("{{") && wrap == Wrap::Expression {
        refs::rewrite_template(graph, index, text)
    } else {
        refs::rewrite_code(graph, index, text, wrap, scope)
    };
    record(rewritten.clone(), key, id, out);
    Value::String(rewritten.text)
}

fn rewrite_value(graph: &OldGraph, index: usize, value: &Value, key: &str, id: &str, out: &mut Rewrite) -> Value {
    match value {
        Value::String(text) if text.contains("{{") => {
            let rewritten = refs::rewrite_template(graph, index, text);
            record(rewritten.clone(), key, id, out);
            Value::String(rewritten.text)
        }
        Value::Array(items) => Value::Array(items.iter().map(|v| rewrite_value(graph, index, v, key, id, out)).collect()),
        Value::Object(map) => {
            Value::Object(map.iter().map(|(k, v)| (k.clone(), rewrite_value(graph, index, v, key, id, out))).collect())
        }
        other => other.clone(),
    }
}

fn record(rewritten: refs::Rewritten, key: &str, id: &str, out: &mut Rewrite) {
    out.changes.extend(rewritten.changes.into_iter().map(|t| Item::new(id, format!("{t} in --{}", key.replace('_', "-")))));
    out.notes.extend(rewritten.notes.into_iter().map(|t| Item::new(id, format!("{t} in --{}", key.replace('_', "-")))));
    out.unresolved
        .extend(rewritten.unresolved.into_iter().map(|t| Item::new(id, format!("{t} (in --{})", key.replace('_', "-")))));
}

/// What the 0.11 flow rules run differently: a stray cycle, and a node two
/// branches feed (0.10 ran it once per arriving payload).
fn flow_checks(graph: &OldGraph, out: &mut Rewrite) {
    if let Some(cycle) = cycle_without_reentry(graph) {
        out.unresolved.push(Item::new(
            "",
            format!("the nodes {} form a loop; 0.11 allows only a logic.retry re-entry", cycle.join(" → ")),
        ));
    }
    let branching = |kind: &str| BRANCHING.contains(&kind);
    for node in 0..graph.nodes.len() {
        if graph.incoming[node].len() < 2 || graph.nodes[node].kind == "n.logic.collect" {
            continue;
        }
        if !graph.exclusive_join(node, &branching) {
            out.unresolved.push(Item::new(
                &graph.nodes[node].id,
                "two branches that can both run feed this node; 0.10 ran it once for each, 0.11 runs it once with both payloads merged",
            ));
        }
    }
    // Nodes after a response saw only its envelope in 0.10.
    for (i, node) in graph.nodes.iter().enumerate() {
        if node.kind == "n.web.response" && graph.edges.iter().any(|e| e.from == i && e.from_pin != "error") {
            out.unresolved.push(Item::new(
                &node.id,
                "nodes run after this response: in 0.10 they saw only the response envelope and the last one decided the answer; in 0.11 the response answers at once",
            ));
        }
    }
}

fn cycle_without_reentry(graph: &OldGraph) -> Option<Vec<String>> {
    let edges: Vec<&Edge> = graph.edges.iter().filter(|e| !is_reentry(&graph.nodes, e)).collect();
    fn visit(g: &OldGraph, edges: &[&Edge], at: usize, state: &mut [u8], stack: &mut Vec<usize>) -> Option<Vec<String>> {
        state[at] = 1;
        stack.push(at);
        for edge in edges.iter().filter(|e| e.from == at) {
            match state[edge.to] {
                1 => {
                    let start = stack.iter().position(|&s| s == edge.to).unwrap_or(0);
                    return Some(stack[start..].iter().map(|&i| g.nodes[i].id.clone()).collect());
                }
                0 => {
                    if let Some(found) = visit(g, edges, edge.to, state, stack) {
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
    let mut state = vec![0u8; graph.nodes.len()];
    let mut stack = Vec::new();
    (0..graph.nodes.len()).find_map(|start| if state[start] == 0 { visit(graph, &edges, start, &mut state, &mut stack) } else { None })
}

/// The node whose answer is the whole payload `node` passed on, through
/// nodes that passed it on.
fn whole_producer(graph: &OldGraph, mut node: usize) -> Option<usize> {
    for _ in 0..graph.nodes.len() {
        match &graph.outputs[node] {
            OldOutput::Pass => {
                let [edge] = graph.incoming[node].as_slice() else { return None };
                node = graph.edges[*edge].from;
            }
            OldOutput::Replace { .. } => return Some(node),
            _ => return None,
        }
    }
    None
}

/// A node id not yet in `nodes`: `base`, else `base2`, `base3`, ….
fn fresh_id(nodes: &[Value], base: &str) -> String {
    let used: BTreeSet<&str> = nodes.iter().filter_map(|n| n.get("id").and_then(Value::as_str)).collect();
    let mut id = base.to_string();
    let mut k = 1;
    while used.contains(id.as_str()) {
        k += 1;
        id = format!("{base}{k}");
    }
    id
}

/// A 0.10 node that delivered its own failure (`function.call`, the
/// composites) carried on past it when its `error` pin was unwired, and the
/// webhook answered the payload. 0.11 fails the node there. In a webhook
/// pipeline the failure stays visible to the caller without the payload:
/// the node's `:error` is wired to a `web.response.send --status 500`
/// answering `{ ok: false, error: { code } }` and nothing else — no message:
/// a raw message can carry internals, and the route's `--errors` governs
/// detail.
fn answer_unwired_failures(graph: &OldGraph, nodes: &mut Vec<Value>, edges: &mut Vec<Value>, out: &mut Rewrite) {
    let webhook = graph.nodes.iter().any(|n| n.kind == "n.trigger.webhook");
    for (i, node) in graph.nodes.iter().enumerate() {
        let self_routed = kinds::rule(&node.kind).is_some_and(|rule| rule.self_routed);
        if !self_routed || graph.edges.iter().any(|e| e.from == i && e.from_pin == "error") {
            continue;
        }
        if !webhook {
            out.notes.push(Item::new(
                &node.id,
                "0.10 carried on past a failure here with nothing wired to `error`; 0.11 fails the run at this node",
            ));
            continue;
        }
        let id = fresh_id(nodes, "failed");
        let key = graph.failure_keys[i].as_deref().unwrap_or("result");
        let body = format!("{{{{ {{ ok: false, error: {{ code: input.{key}.error.code }} }} }}}}");
        nodes.push(json!({
            "id": id,
            "kind": "web.response.send",
            "input_pins": ["in"],
            "output_pins": ["out"],
            "config": { "status": 500, "body": body },
        }));
        edges.push(json!({ "from_node": node.id, "from_pin": "error", "to_node": id, "to_pin": "in" }));
        out.changes.push(Item::new(
            &id,
            format!(
                "added web.response.send --status 500 --body \"{body}\" on `{}`'s :error: 0.10 carried on past a failed call and answered the run's value; 0.11 answers the failure's code and nothing else",
                node.id
            ),
        ));
    }
}

/// A 0.10 webhook answered the payload of whichever path ended without a
/// response node — request body, credentials read into it, generated
/// secrets. 0.11 never does (`node-conventions.md` §4): such a path answers
/// `204` with no body. Nothing is added to reproduce the old answer; each
/// such path is for the owner to review, with the keys its payload holds so
/// they can choose what the caller needs.
fn review_unanswered_routes(graph: &OldGraph, out: &mut Rewrite) {
    if !graph.nodes.iter().any(|n| n.kind == "n.trigger.webhook") {
        return;
    }
    let responds = |i: usize| graph.nodes[i].kind == "n.web.response";
    let sinks: Vec<usize> = (0..graph.nodes.len())
        .filter(|&i| !graph.edges.iter().any(|e| e.from == i && !is_reentry(&graph.nodes, e)))
        .filter(|&i| !responds(i) && !graph.ancestors(i).iter().any(|&a| responds(a)))
        .collect();
    for sink in sinks {
        // The 0.10 response instructions a script could return: the route
        // set a status, a cookie or a page from them, which 0.11 does only
        // through web.response.send.
        let producer = whole_producer(graph, sink);
        if let Some(producer) = producer
            && graph.nodes[producer].kind == "n.script"
        {
            let source = graph.nodes[producer].config.get("source").and_then(Value::as_str).unwrap_or("");
            if let Some(word) = ["_status", "_set_cookie", "html"].into_iter().find(|w| source.contains(w)) {
                out.unresolved.push(Item::new(
                    &graph.nodes[producer].id,
                    format!("the script may answer `{word}`, which the 0.10 webhook read as a response instruction; answer with web.response.send instead"),
                ));
                continue;
            }
        }
        let mut keys: BTreeSet<String> = graph.ancestors(sink).iter().filter_map(|&a| graph.nouns[a].clone()).collect();
        keys.extend(graph.nouns[sink].clone());
        let keys = if keys.is_empty() {
            "none known".to_string()
        } else {
            keys.iter().map(|k| format!("`{k}`")).collect::<Vec<_>>().join(", ")
        };
        // What the run's value was: a node that replaced the payload
        // answered its own value, else the payload as this path carried it.
        let value = match producer {
            Some(p) => format!("`{}`'s answer, which replaced the payload", graph.nodes[p].id),
            None => "the payload this path carried".to_string(),
        };
        out.review.push(Item::new(
            &graph.nodes[sink].id,
            format!(
                "0.10 answered the run's value here ({value}); 0.11 answers 204 — add web.response.send with what the caller needs (the payload's keys: {keys})"
            ),
        ));
    }
}

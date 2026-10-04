//! A route that can end without answering (`node-conventions.md` §4,
//! `published-mcp.md`): a webhook run that reaches no answering node
//! (`web.response.send`, `auth.oauth.approve` — [`answering_kind`])
//! answers `204` with no body, and a published MCP tool's run an empty
//! result — never the run's value. Each path from a `trigger.webhook` or a
//! `trigger.mcp` that ends at a sink (a node with no outgoing edge) without
//! passing an answering node is a warning, one per sink — an `:error` handler
//! included, since its edge is a path like any other. A warning, not a
//! refusal: a route that only acknowledges is legitimate.

use std::collections::BTreeSet;

use super::Problem;
use crate::pipeline::model::PipelineGraph;
use crate::pipeline::nodes::basic::{answering_kind, trigger};

/// Every sink some path from a `trigger.webhook` or a `trigger.mcp` reaches
/// without an answering node, in graph order.
pub fn unanswered_routes(graph: &PipelineGraph) -> Vec<Problem> {
    let mut problems = sinks_without_answer(graph, trigger::webhook::NODE_KIND, |sink| {
        format!("this route never answers a body (204) on the path ending at `{sink}` — end it in web.response.send to answer the caller")
    });
    for problem in sinks_without_answer(graph, trigger::mcp_trigger::NODE_KIND, |sink| {
        format!("this tool never answers a result on the path ending at `{sink}` — end it in web.response.send to answer the agent")
    }) {
        if !problems.contains(&problem) {
            problems.push(problem);
        }
    }
    problems
}

fn sinks_without_answer(graph: &PipelineGraph, trigger_kind: &str, message: impl Fn(&str) -> String) -> Vec<Problem> {
    let answers = |id: &str| graph.nodes.iter().any(|n| n.id == id && answering_kind(&n.kind));
    let mut seen = BTreeSet::new();
    let mut stack: Vec<&str> =
        graph.nodes.iter().filter(|n| n.kind == trigger_kind).map(|n| n.id.as_str()).collect();
    // Walk every edge, the `:error` ones too, and stop at an answering node:
    // whatever runs after it runs after the caller has the answer.
    while let Some(id) = stack.pop() {
        if !seen.insert(id) || answers(id) {
            continue;
        }
        stack.extend(graph.edges.iter().filter(|e| e.from_node == id).map(|e| e.to_node.as_str()));
    }
    graph
        .nodes
        .iter()
        .filter(|n| seen.contains(n.id.as_str()) && !answers(&n.id))
        .filter(|n| !graph.edges.iter().any(|e| e.from_node == n.id))
        .map(|n| Problem::node(&n.id, &n.kind, None, message(&n.id)))
        .collect()
}

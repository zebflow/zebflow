//! The run's flow, read off the graph once (`node-conventions.md` §4).
//!
//! Which edges count towards a node's readiness, which edges re-enter the
//! node a `logic.retry` retries, what each `logic.foreach` loop runs per item
//! and which node closes it, and whether every `$nodes` reference points
//! upstream. Everything wrong here is refused at activation, so a run never
//! waits for an edge that cannot arrive.

use std::collections::{HashMap, HashSet, VecDeque};

use super::NodesAccess;
use crate::pipeline::model::{PipelineError, PipelineGraph};
use crate::pipeline::nodes::basic::logic;
use crate::pipeline::nodes::check::suggest::did_you_mean;

/// The pin every node has, where a failure goes when it is wired.
pub(super) const ERROR_PIN: &str = "error";
/// A cycle that is not a `logic.retry` re-entry.
pub(super) const CODE_CYCLE: &str = "FW_PIPELINE_CYCLE";
/// A loop body wired across its boundary.
pub(super) const CODE_LOOP: &str = "FW_PIPELINE_LOOP";
/// A `$nodes` reference to a node no edge path leads from.
pub(super) const CODE_UPSTREAM: &str = "FW_NODES_SCOPE_UPSTREAM";

/// One edge, by node position (the node's line in the DSL text).
#[derive(Debug)]
pub(super) struct Edge {
    pub from: usize,
    pub from_pin: String,
    pub to: usize,
    pub to_pin: String,
    /// From a `logic.retry`'s `retry` pin back to a node upstream of it:
    /// never counted towards readiness; delivering it runs that node again.
    pub reentry: bool,
}

/// One `logic.foreach` loop.
#[derive(Debug, Default)]
pub(super) struct Loop {
    /// The nodes that run once per item at this loop's own level, in text
    /// order (an inner loop's body belongs to the inner loop).
    pub body: Vec<usize>,
    /// The `logic.reduce` / `logic.collect` nodes that close it.
    pub closes: Vec<usize>,
}

#[derive(Debug)]
pub(super) struct FlowPlan {
    pub index: HashMap<String, usize>,
    pub edges: Vec<Edge>,
    /// Counted (not re-entry) edges into each node, in text order of their
    /// source node.
    pub incoming: Vec<Vec<usize>>,
    /// Every edge out of each node.
    pub outgoing: Vec<Vec<usize>>,
    pub entries: Vec<usize>,
    /// Keyed by the `logic.foreach` node.
    pub loops: HashMap<usize, Loop>,
    /// The loop (its foreach) whose body holds a node; `None` at the top.
    pub owner: Vec<Option<usize>>,
    /// The loop (its foreach) a node closes.
    pub closes: Vec<Option<usize>>,
    /// No outgoing edge at all.
    pub sink: Vec<bool>,
}

impl FlowPlan {
    pub fn build(
        graph: &PipelineGraph,
        access: &HashMap<String, NodesAccess>,
    ) -> Result<Self, PipelineError> {
        let n = graph.nodes.len();
        let index: HashMap<String, usize> = graph
            .nodes
            .iter()
            .enumerate()
            .map(|(i, node)| (node.id.clone(), i))
            .collect();
        let kind = |i: usize| graph.nodes[i].kind.as_str();
        let mut edges = Vec::with_capacity(graph.edges.len());
        for edge in &graph.edges {
            let find = |id: &str| {
                index.get(id).copied().ok_or_else(|| {
                    PipelineError::new("FW_EXEC_EDGE", format!("edge names node '{id}', which is not in the graph"))
                })
            };
            edges.push(Edge {
                from: find(&edge.from_node)?,
                from_pin: edge.from_pin.clone(),
                to: find(&edge.to_node)?,
                to_pin: edge.to_pin.clone(),
                reentry: false,
            });
        }

        // Re-entry: a retry's `retry` edge to a node that reaches the retry
        // without it (the retried node, or the start of a polled stretch).
        let candidate = |e: &Edge| kind(e.from) == logic::retry::NODE_KIND && e.from_pin == logic::retry::OUTPUT_PIN_RETRY;
        let mut forward: Vec<Vec<usize>> = vec![Vec::new(); n];
        for e in edges.iter().filter(|e| !candidate(e)) {
            forward[e.from].push(e.to);
        }
        for e in edges.iter_mut() {
            if candidate(e) {
                e.reentry = e.to == e.from || reaches(&forward, e.to, e.from);
            }
        }

        let mut incoming = vec![Vec::new(); n];
        let mut outgoing = vec![Vec::new(); n];
        for (i, e) in edges.iter().enumerate() {
            outgoing[e.from].push(i);
            if !e.reentry {
                incoming[e.to].push(i);
            }
        }
        for list in &mut incoming {
            list.sort_by_key(|&i| (edges[i].from, i));
        }
        refuse_cycles(graph, &edges)?;

        let entries: Vec<usize> = if graph.entry_nodes.is_empty() {
            (0..n).filter(|&i| incoming[i].is_empty()).collect()
        } else {
            graph.entry_nodes.iter().filter_map(|id| index.get(id).copied()).collect()
        };

        let mut plan = FlowPlan {
            index,
            edges,
            incoming,
            outgoing,
            entries,
            loops: HashMap::new(),
            owner: vec![None; n],
            closes: vec![None; n],
            sink: vec![false; n],
        };
        for i in 0..n {
            plan.sink[i] = plan.outgoing[i].is_empty();
        }
        plan.find_loops(graph)?;
        plan.refuse_loop_crossings(graph)?;
        plan.refuse_downstream_references(graph, access)?;
        Ok(plan)
    }

    /// Every node reachable from `start` over counted edges, `start` included.
    pub fn downstream_of(&self, start: usize) -> HashSet<usize> {
        let mut seen = HashSet::from([start]);
        let mut queue = VecDeque::from([start]);
        while let Some(at) = queue.pop_front() {
            for &e in &self.outgoing[at] {
                let edge = &self.edges[e];
                if !edge.reentry && seen.insert(edge.to) {
                    queue.push_back(edge.to);
                }
            }
        }
        seen
    }

    /// Each foreach's body: every node reachable from its `item` edges
    /// before the reduce or collect that closes it. An inner foreach opens
    /// a level and its close shuts it again, so the walk carries the depth.
    fn find_loops(&mut self, graph: &PipelineGraph) -> Result<(), PipelineError> {
        let kind = |i: usize| graph.nodes[i].kind.as_str();
        let is_close_kind = |i: usize| matches!(kind(i), logic::reduce::NODE_KIND | logic::collect::NODE_KIND);
        let id = |i: usize| graph.nodes[i].id.as_str();
        for foreach in (0..graph.nodes.len()).filter(|&i| kind(i) == logic::foreach_::NODE_KIND) {
            let mut lp = Loop::default();
            let mut depth_of: HashMap<usize, usize> = HashMap::new();
            let mut stack: Vec<(usize, usize)> = self.outgoing[foreach]
                .iter()
                .map(|&e| &self.edges[e])
                .filter(|e| e.from_pin == logic::foreach_::OUTPUT_PIN_ITEM)
                .map(|e| (e.to, 0))
                .collect();
            while let Some((node, depth)) = stack.pop() {
                let (level, next_depth) = if is_close_kind(node) {
                    if depth == 0 {
                        if !lp.closes.contains(&node) {
                            if let Some(other) = self.closes[node] {
                                return Err(PipelineError::new(
                                    CODE_LOOP,
                                    format!("'{}' closes two loops, '{}' and '{}'; give each loop its own reduce or collect", id(node), id(other), id(foreach)),
                                ));
                            }
                            self.closes[node] = Some(foreach);
                            lp.closes.push(node);
                        }
                        continue;
                    }
                    (depth - 1, depth - 1)
                } else {
                    (depth, depth)
                };
                match depth_of.get(&node) {
                    Some(&seen) if seen == level => continue,
                    Some(_) => {
                        return Err(PipelineError::new(
                            CODE_LOOP,
                            format!("'{}' is reached both inside and outside an inner loop of '{}'; a node belongs to one loop level", id(node), id(foreach)),
                        ));
                    }
                    None => {
                        depth_of.insert(node, level);
                    }
                }
                if level == 0 {
                    if let Some(other) = self.owner[node].filter(|&o| o != foreach) {
                        return Err(PipelineError::new(
                            CODE_LOOP,
                            format!("'{}' is in the body of two loops, '{}' and '{}'", id(node), id(other), id(foreach)),
                        ));
                    }
                    self.owner[node] = Some(foreach);
                    lp.body.push(node);
                }
                for &e in &self.outgoing[node] {
                    let edge = &self.edges[e];
                    if edge.reentry {
                        continue;
                    }
                    let d = if kind(node) == logic::foreach_::NODE_KIND && edge.from_pin == logic::foreach_::OUTPUT_PIN_ITEM {
                        next_depth + 1
                    } else {
                        next_depth
                    };
                    stack.push((edge.to, d));
                }
            }
            lp.body.sort_unstable();
            lp.closes.sort_unstable();
            self.loops.insert(foreach, lp);
        }
        Ok(())
    }

    /// A loop body is entered only through its foreach's `item` pin and
    /// left only into its close; a close takes edges only from its loop.
    fn refuse_loop_crossings(&self, graph: &PipelineGraph) -> Result<(), PipelineError> {
        let id = |i: usize| graph.nodes[i].id.as_str();
        for edge in &self.edges {
            let (s, t) = (edge.from, edge.to);
            let through_item = edge.from_pin == logic::foreach_::OUTPUT_PIN_ITEM && self.loops.contains_key(&s);
            if through_item && (self.owner[t] == Some(s) || self.closes[t] == Some(s)) {
                continue;
            }
            if let Some(f) = self.closes[t] {
                if self.owner[s] == Some(f) {
                    continue;
                }
                return Err(PipelineError::new(
                    CODE_LOOP,
                    format!("'{}' closes the loop of '{}' and takes an edge from '{}', which is outside that loop; a close gathers its loop's items only", id(t), id(f), id(s)),
                ));
            }
            if self.owner[s] == self.owner[t] {
                continue;
            }
            if let Some(f) = self.owner[s] {
                return Err(PipelineError::new(
                    CODE_LOOP,
                    format!("'{}' runs once per item of the loop of '{}'; its edge to '{}' leaves the loop other than through the loop's close (a logic.reduce or logic.collect after the body)", id(s), id(f), id(t)),
                ));
            }
            let f = self.owner[t].unwrap_or(t);
            if self.closes[s] == Some(f) {
                return Err(PipelineError::new(
                    CODE_LOOP,
                    format!("'{}' is reached both from inside the loop of '{}' and after its close '{}'; a node inside a loop leads out of it only through the close", id(t), id(f), id(s)),
                ));
            }
            return Err(PipelineError::new(
                CODE_LOOP,
                format!("'{}' runs once per item of the loop of '{}'; the edge from '{}' enters it from outside the loop", id(t), id(f), id(s)),
            ));
        }
        Ok(())
    }

    /// `$nodes.<id>` names a node some edge path leads from — what has
    /// finished or been skipped by the time the reader runs.
    fn refuse_downstream_references(
        &self,
        graph: &PipelineGraph,
        access: &HashMap<String, NodesAccess>,
    ) -> Result<(), PipelineError> {
        let mut backward: Vec<Vec<usize>> = vec![Vec::new(); graph.nodes.len()];
        for e in &self.edges {
            backward[e.to].push(e.from);
        }
        for (i, node) in graph.nodes.iter().enumerate() {
            let Some(NodesAccess::Exact(ids)) = access.get(&node.id) else {
                continue;
            };
            let mut ids: Vec<&String> = ids.iter().collect();
            ids.sort();
            let mut upstream: Option<HashSet<usize>> = None;
            for target in ids {
                if node.kind == logic::retry::NODE_KIND && *target == node.id {
                    continue;
                }
                let upstream = upstream.get_or_insert_with(|| ancestors(&backward, i));
                if self.index.get(target).is_some_and(|t| upstream.contains(t)) {
                    continue;
                }
                let names: Vec<&str> = {
                    let mut names: Vec<&str> = upstream.iter().map(|&a| graph.nodes[a].id.as_str()).collect();
                    names.sort_unstable();
                    names
                };
                let what = if self.index.contains_key(target) {
                    format!("no edge path leads from '{target}' to '{}', so it has not run when '{}' runs", node.id, node.id)
                } else {
                    format!("there is no node '{target}'")
                };
                let hint = match did_you_mean(target, &names) {
                    Some(near) => format!(" — did you mean $nodes.{near}?"),
                    None if names.is_empty() => String::new(),
                    None => format!(" (upstream of it: {})", names.join(", ")),
                };
                return Err(PipelineError::new(
                    CODE_UPSTREAM,
                    format!("node '{}' reads $nodes.{target}, but {what}{hint}", node.id),
                ));
            }
        }
        Ok(())
    }
}

/// Whether `to` is reachable from `from` over `forward`.
fn reaches(forward: &[Vec<usize>], from: usize, to: usize) -> bool {
    let mut seen = HashSet::from([from]);
    let mut stack = vec![from];
    while let Some(at) = stack.pop() {
        if at == to {
            return true;
        }
        for &next in &forward[at] {
            if seen.insert(next) {
                stack.push(next);
            }
        }
    }
    false
}

/// Every node an edge path leads from (re-entry edges included: a poll
/// stretch reads the retry node's count from the round before).
fn ancestors(backward: &[Vec<usize>], node: usize) -> HashSet<usize> {
    let mut seen = HashSet::new();
    let mut stack = vec![node];
    while let Some(at) = stack.pop() {
        for &prev in &backward[at] {
            if seen.insert(prev) {
                stack.push(prev);
            }
        }
    }
    seen
}

/// Refuses a cycle over counted edges, naming it.
fn refuse_cycles(graph: &PipelineGraph, edges: &[Edge]) -> Result<(), PipelineError> {
    let n = graph.nodes.len();
    let mut forward: Vec<Vec<usize>> = vec![Vec::new(); n];
    for e in edges.iter().filter(|e| !e.reentry) {
        forward[e.from].push(e.to);
    }
    // 0 unvisited, 1 on the path, 2 done.
    let mut mark = vec![0u8; n];
    for start in 0..n {
        if mark[start] != 0 {
            continue;
        }
        let mut path: Vec<(usize, usize)> = vec![(start, 0)];
        mark[start] = 1;
        while let Some(top) = path.len().checked_sub(1) {
            let (at, next) = path[top];
            if let Some(&to) = forward[at].get(next) {
                path[top].1 += 1;
                match mark[to] {
                    0 => {
                        mark[to] = 1;
                        path.push((to, 0));
                    }
                    1 => {
                        let from = path.iter().position(|(p, _)| *p == to).unwrap_or(0);
                        let mut names: Vec<&str> = path[from..].iter().map(|(p, _)| graph.nodes[*p].id.as_str()).collect();
                        names.push(graph.nodes[to].id.as_str());
                        return Err(PipelineError::new(
                            CODE_CYCLE,
                            format!(
                                "pipeline '{}' has a cycle: {}. A node runs once, when everything wired into it has answered, so a cycle would wait for itself; only the edge from a logic.retry's :retry pin back to the node it retries may point back",
                                graph.id,
                                names.join(" → ")
                            ),
                        ));
                    }
                    _ => {}
                }
            } else {
                mark[at] = 2;
                path.pop();
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::shell::parser::build_pipeline_graph;

    fn plan(dsl: &str) -> Result<FlowPlan, PipelineError> {
        let graph = build_pipeline_graph("flow", dsl).expect("graph");
        let retention = super::super::build_nodes_retention_plan(&graph)?;
        FlowPlan::build(&graph, &retention.consumer_access)
    }

    #[test]
    fn a_retry_edge_back_is_reentry_and_any_other_cycle_is_refused_naming_it() {
        let ok = plan("[a] trigger.manual\n[b] javascript.script.run -- \"return 1;\"\n[r] logic.retry --max-attempts 2\n[a] -> [b]\n[b]:error -> [r]\n[r]:retry -> [b]\n").expect("plan");
        let reentry: Vec<bool> = ok.edges.iter().map(|e| e.reentry).collect();
        assert_eq!(reentry, [false, false, true]);
        assert!(ok.incoming[1].len() == 1, "the re-entry edge does not count towards b's readiness");

        let err = plan("[t] trigger.manual\n[a] javascript.script.run -- \"return 1;\"\n[b] javascript.script.run -- \"return 2;\"\n[t] -> [a]\n[a] -> [b]\n[b] -> [a]\n").expect_err("refused");
        assert_eq!(err.code, CODE_CYCLE);
        assert!(err.message.contains("a → b → a"), "{}", err.message);
    }

    #[test]
    fn a_loop_body_and_its_close_are_found_through_nested_loops() {
        let p = plan(
            "[a] trigger.manual\n[f] logic.foreach --from \"input.manual.rows\"\n[x] javascript.script.run -- \"return 1;\"\n[g] logic.foreach --from \"input.item\"\n[y] javascript.script.run -- \"return 2;\"\n[ri] logic.reduce --initial 0 --step \"$acc + 1\"\n[ro] logic.reduce --initial 0 --step \"$acc + 1\"\n[a] -> [f]\n[f]:item -> [x]\n[x] -> [g]\n[g]:item -> [y]\n[y] -> [ri]\n[ri] -> [ro]\n",
        )
        .expect("plan");
        let f = p.index["f"];
        let g = p.index["g"];
        assert_eq!(p.loops[&f].body, [p.index["x"], g, p.index["ri"]]);
        assert_eq!(p.loops[&f].closes, [p.index["ro"]]);
        assert_eq!(p.loops[&g].body, [p.index["y"]]);
        assert_eq!(p.loops[&g].closes, [p.index["ri"]]);
        assert_eq!(p.owner[p.index["ro"]], None);
    }

    #[test]
    fn an_edge_leaving_a_loop_body_is_refused() {
        // A failure handler inside the body is part of the body…
        plan("[a] trigger.manual\n[f] logic.foreach --from \"input.manual.rows\"\n[x] javascript.script.run -- \"return 1;\"\n[h] javascript.script.run -- \"return 2;\"\n[r] logic.reduce --initial 0 --step \"$acc + 1\"\n[a] -> [f]\n[f]:item -> [x]\n[x] -> [r]\n[x]:error -> [h]\n").expect("a body sink");
        // …but an edge to a node after the close leaves the loop.
        let err = plan("[a] trigger.manual\n[f] logic.foreach --from \"input.manual.rows\"\n[x] javascript.script.run -- \"return 1;\"\n[r] logic.reduce --initial 0 --step \"$acc + 1\"\n[after] javascript.script.run -- \"return 2;\"\n[a] -> [f]\n[f]:item -> [x]\n[x] -> [r]\n[r] -> [after]\n[x]:error -> [after]\n").expect_err("refused");
        assert_eq!(err.code, CODE_LOOP);
        assert!(err.message.contains("'after'") && err.message.contains("after its close 'r'"), "{}", err.message);
        // And a retry inside the body may not re-enter a node before the loop.
        let err = plan("[a] trigger.manual\n[p] javascript.script.run -- \"return 1;\"\n[f] logic.foreach --from \"input.script\"\n[x] javascript.script.run -- \"return 1;\"\n[w] logic.retry --max-attempts 2\n[r] logic.reduce --initial 0 --step \"$acc + 1\"\n[a] -> [p]\n[p] -> [f]\n[f]:item -> [x]\n[x]:error -> [w]\n[w]:retry -> [p]\n[x] -> [r]\n").expect_err("refused");
        assert_eq!(err.code, CODE_LOOP);
        assert!(err.message.contains("'w'") && err.message.contains("leaves the loop"), "{}", err.message);
    }

    #[test]
    fn a_reference_to_a_node_not_upstream_is_refused_with_a_near_name() {
        let err = plan("[a] trigger.manual\n[fetch] javascript.script.run -- \"return 1;\"\n[b] javascript.script.run -- \"return 2;\"\n[c] crypto.base64.encode --text \"{{ $nodes.fetsh.script }}\"\n[a] -> [fetch]\n[fetch] -> [c]\n[a] -> [b]\n").expect_err("refused");
        assert_eq!(err.code, CODE_UPSTREAM);
        assert!(err.message.contains("did you mean $nodes.fetch"), "{}", err.message);

        let err = plan("[a] trigger.manual\n[b] javascript.script.run -- \"return 1;\"\n[c] crypto.base64.encode --text \"{{ $nodes.b.script }}\"\n[a] -> [b]\n[a] -> [c]\n").expect_err("a sibling branch is not upstream");
        assert_eq!(err.code, CODE_UPSTREAM);
        assert!(err.message.contains("no edge path leads from 'b' to 'c'"), "{}", err.message);
    }
}

#[cfg(test)]
mod bundle_tests {
    /// Every function pipeline shipped in an official bundle passes the
    /// flow checks activation runs: no stray cycle, loops wired through
    /// their close, every `$nodes` reference upstream.
    #[test]
    fn every_shipped_function_pipeline_passes_the_flow_checks() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/pipeline/nodes/bundled");
        let mut checked = 0;
        for bundle in std::fs::read_dir(&root).expect("bundled dir").flatten() {
            let Ok(functions) = std::fs::read_dir(bundle.path().join("functions")) else { continue };
            for file in functions.flatten() {
                let path = file.path();
                if !path.to_string_lossy().ends_with(".zf.json") {
                    continue;
                }
                let bytes = std::fs::read(&path).expect("read");
                let graph = crate::contracts::kinds::decode_pipeline_graph(&bytes).expect("decode").spec;
                if let Err(err) = super::super::validate_flow(&graph) {
                    panic!("{}: {} {}", path.strip_prefix(&root).unwrap_or(&path).display(), err.code, err.message);
                }
                checked += 1;
            }
        }
        assert!(checked > 0);
    }
}

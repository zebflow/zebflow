//! Which node runs when (`node-conventions.md` §4, Flow).
//!
//! Every edge is *pending*, *delivered* (with a payload) or *skipped*. A node
//! is ready when every edge into it has delivered or been skipped: with at
//! least one delivery it runs once, its input the delivered payloads merged
//! in DSL text order of their sources; with none it is skipped, and so is
//! every edge out of it. When a node finishes, the edges from the pins it
//! emitted deliver and every other edge out of it is skipped.
//!
//! A `logic.foreach` runs its body once per item, each item in a frame of
//! its own — its own edge states and its own `$nodes` — one frame finished
//! before the next starts; the reduce or collect that closes the loop runs
//! once after the last frame, over what the frames delivered into it. The
//! edge from a `logic.retry` back to the node it retries is a re-entry: it
//! never counts towards readiness, and delivering it runs that node again
//! with everything downstream of it in the same frame.
//!
//! A node that could run always runs before a skip is decided, so a branch
//! a retry is about to take round again is never marked skipped first.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use futures::future::BoxFuture;
use serde_json::{Map, Value, json};

use super::flow::FlowPlan;
use super::{BasicPipelineEngine, NodesAccess, NodesRetentionPlan, emit_lifecycle, strip_private_markers};
use crate::pipeline::model::{
    ExecutionBus, NodeTraceEntry, PipelineContext, PipelineError, PipelineGraph, PipelineNode,
    PipelineOutput, Responder,
};
use crate::pipeline::nodes::NodeExecutionOutput;
use crate::pipeline::nodes::basic::logic;
use crate::pipeline::trace_capture::TraceCapture;

/// How a node that ran came out of it.
pub(super) enum Ran {
    /// It answered; each output names the pins it emits on.
    Answered(Vec<NodeExecutionOutput>),
    /// It failed and its `:error` pin is wired: the payload that pin delivers.
    Routed(Value),
}

/// What a node runs with.
pub(super) struct Delivery {
    pub payload: Value,
    pub input_pin: String,
    /// `{ item, index, count }` of the loop frame it runs in.
    pub series: Option<Value>,
    /// For the close of a loop: the items the loop's frames delivered.
    pub loop_items: Option<Vec<Value>>,
}

/// Everything one run accumulates, across every frame.
pub(super) struct RunState<'a> {
    pub ctx: &'a PipelineContext,
    pub graph: &'a PipelineGraph,
    pub bus: Option<Arc<ExecutionBus>>,
    pub run_started: std::time::Instant,
    pub project_timeout_secs: u64,
    pub trace_capture: TraceCapture,
    pub retention: NodesRetentionPlan,
    pub responder: Option<Responder>,
    pub trace: Vec<String>,
    pub node_trace: Vec<NodeTraceEntry>,
    pub response: Option<Value>,
    /// The answers `$nodes` can see: the top frame, then one per loop frame
    /// open now. A closed frame is dropped, and its body's answers with it.
    pub frames: Vec<HashMap<usize, Value>>,
    /// Each node's latest answer (the last output's payload), for `value`.
    pub last_answer: Vec<Option<Value>>,
    pub last_in_time: Option<Value>,
}

impl RunState<'_> {
    /// A node's answer as `$nodes` sees it from the current frame.
    pub fn answer_of(&self, node: usize) -> Option<&Value> {
        self.frames.iter().rev().find_map(|frame| frame.get(&node))
    }

    /// Keeps a node's answer in the current frame when some node reads it.
    pub fn store_answer(&mut self, node: &PipelineNode, idx: usize, value: Value) {
        if self.retention.retained_nodes.contains(&node.id)
            && let Some(frame) = self.frames.last_mut()
        {
            frame.insert(idx, value);
        }
    }

    /// `$nodes` for one node: each node it references, its answer or `null`
    /// (skipped, not run yet, or inside a loop that has closed).
    pub fn nodes_scope(&self, plan: &FlowPlan, node: &PipelineNode) -> Value {
        let Some(NodesAccess::Exact(ids)) = self.retention.consumer_access.get(&node.id) else {
            return json!({});
        };
        let mut scope = Map::new();
        for id in ids {
            let answer = plan.index.get(id).and_then(|&i| self.answer_of(i)).cloned();
            scope.insert(id.clone(), answer.unwrap_or(Value::Null));
        }
        Value::Object(scope)
    }

    /// A node that never ran: recorded `skipped`, announced, and `null` to
    /// every `$nodes` reader from now on.
    fn record_skip(&mut self, idx: usize) {
        let node = &self.graph.nodes[idx];
        if let Some(frame) = self.frames.last_mut() {
            frame.remove(&idx);
        }
        self.last_answer[idx] = None;
        self.node_trace.push(NodeTraceEntry {
            node_id: node.id.clone(),
            node_kind: node.kind.clone(),
            config: None,
            duration_ms: 0,
            input: Value::Null,
            output: Value::Null,
            error: None,
            status: "skipped".to_string(),
            error_code: None,
            preview_snapshot: None,
        });
        emit_lifecycle(
            &self.bus,
            "node_skipped",
            format!("{} {} skipped", node.id, node.kind),
            Some((&node.id, &node.kind)),
            None,
            &self.run_started,
        );
    }

    /// The run's result. `value` is the answer of the last sink that ran (a
    /// node with no outgoing edge), in DSL text order; when none ran, the
    /// last answer in time.
    pub fn finish(self, plan: &FlowPlan) -> PipelineOutput {
        let value = (0..self.last_answer.len())
            .rev()
            .filter(|&i| plan.sink[i])
            .find_map(|i| self.last_answer[i].clone())
            .or(self.last_in_time)
            .unwrap_or(Value::Null);
        PipelineOutput {
            value: strip_private_markers(value),
            response: self.response.map(strip_private_markers),
            trace: self.trace,
            node_trace: self.node_trace,
        }
    }
}

#[derive(Clone)]
enum EdgeState {
    Delivered(Value),
    Skipped,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NodeState {
    Done,
    Skipped,
}

/// How a loop came out, for the nodes that close it.
enum LoopOutcome {
    /// The foreach answered: the payload it received, and per close the
    /// items its frames delivered.
    Ran { base: Value, items: HashMap<usize, Vec<Value>> },
    /// The foreach was skipped or failed: its closes are skipped.
    Skipped,
}

/// One frame: the top of the run, or one item of a loop.
pub(super) struct ScopeRun {
    members: Vec<usize>,
    member: HashSet<usize>,
    series: Option<Value>,
    /// Entry nodes and the envelope the run started with (top frame only).
    seeds: HashMap<usize, Value>,
    /// Absent: pending.
    edges: HashMap<usize, EdgeState>,
    /// Absent: pending.
    nodes: HashMap<usize, NodeState>,
    loops: HashMap<usize, LoopOutcome>,
    /// Ready to run; a re-entry carries its own payload and pin.
    run_q: VecDeque<(usize, Option<(Value, String)>)>,
    /// Ready with nothing delivered; decided only when nothing can run.
    skip_q: VecDeque<usize>,
}

impl ScopeRun {
    fn new(members: Vec<usize>, series: Option<Value>) -> Self {
        Self {
            member: members.iter().copied().collect(),
            members,
            series,
            seeds: HashMap::new(),
            edges: HashMap::new(),
            nodes: HashMap::new(),
            loops: HashMap::new(),
            run_q: VecDeque::new(),
            skip_q: VecDeque::new(),
        }
    }

    /// The top frame: every node outside a loop body, the entry nodes
    /// seeded with the envelope the run started with.
    pub fn top(plan: &FlowPlan, graph: &PipelineGraph, input: &Value) -> Self {
        let members = (0..graph.nodes.len()).filter(|&i| plan.owner[i].is_none()).collect();
        let mut scope = Self::new(members, None);
        for &entry in &plan.entries {
            scope.seeds.insert(entry, input.clone());
        }
        scope
    }

    fn pending(&self, node: usize) -> bool {
        !self.nodes.contains_key(&node)
    }

    /// `None` while an input is pending; else whether anything was delivered.
    fn resolved(&self, plan: &FlowPlan, node: usize) -> Option<bool> {
        if let Some(foreach) = plan.closes[node] {
            return match self.loops.get(&foreach)? {
                LoopOutcome::Ran { .. } => Some(true),
                LoopOutcome::Skipped => Some(false),
            };
        }
        let mut delivered = self.seeds.contains_key(&node);
        for edge in &plan.incoming[node] {
            match self.edges.get(edge)? {
                EdgeState::Delivered(_) => delivered = true,
                EdgeState::Skipped => {}
            }
        }
        Some(delivered)
    }

    /// Queues a member whose inputs have all resolved.
    fn check(&mut self, plan: &FlowPlan, node: usize) {
        if !self.member.contains(&node) || !self.pending(node) {
            return;
        }
        match self.resolved(plan, node) {
            Some(true) => self.run_q.push_back((node, None)),
            Some(false) => self.skip_q.push_back(node),
            None => {}
        }
    }

    fn set_edge(&mut self, plan: &FlowPlan, edge: usize, state: EdgeState) {
        self.edges.insert(edge, state);
        self.check(plan, plan.edges[edge].to);
    }

    fn set_loop(&mut self, plan: &FlowPlan, foreach: usize, outcome: LoopOutcome) {
        self.loops.insert(foreach, outcome);
        if let Some(lp) = plan.loops.get(&foreach) {
            for &close in &lp.closes {
                self.check(plan, close);
            }
        }
    }

    /// What a ready node runs with: the delivered payloads merged in text
    /// order (one delivery unchanged); a `logic.collect` join gets the list;
    /// a loop's close the payload its foreach received and the items.
    fn delivery(&self, plan: &FlowPlan, graph: &PipelineGraph, node: usize) -> Delivery {
        let first_pin = || graph.nodes[node].input_pins.first().cloned().unwrap_or_default();
        if let Some(foreach) = plan.closes[node] {
            let (payload, items) = match self.loops.get(&foreach) {
                Some(LoopOutcome::Ran { base, items }) => {
                    (base.clone(), items.get(&node).cloned().unwrap_or_default())
                }
                _ => (Value::Null, Vec::new()),
            };
            let input_pin = plan.incoming[node].first().map(|&e| plan.edges[e].to_pin.clone()).unwrap_or_else(first_pin);
            return Delivery { payload, input_pin, series: self.series.clone(), loop_items: Some(items) };
        }
        let mut delivered: Vec<Value> = Vec::new();
        let mut input_pin = None;
        if let Some(seed) = self.seeds.get(&node) {
            delivered.push(seed.clone());
        }
        for &edge in &plan.incoming[node] {
            if let Some(EdgeState::Delivered(payload)) = self.edges.get(&edge) {
                delivered.push(payload.clone());
                input_pin.get_or_insert_with(|| plan.edges[edge].to_pin.clone());
            }
        }
        let payload = if graph.nodes[node].kind == logic::collect::NODE_KIND {
            Value::Array(delivered)
        } else {
            merge(delivered)
        };
        Delivery { payload, input_pin: input_pin.unwrap_or_else(first_pin), series: self.series.clone(), loop_items: None }
    }

    /// A re-entry edge delivered: the node it reaches, and everything
    /// downstream of it in this frame, go back to pending, and that node
    /// runs again with the re-entry's payload. Answers stay — a poll
    /// stretch reads the retry node's count from the round before.
    fn reenter(&mut self, plan: &FlowPlan, target: usize, payload: Value, pin: String) {
        for node in plan.downstream_of(target) {
            if !self.member.contains(&node) {
                continue;
            }
            self.nodes.remove(&node);
            self.loops.remove(&node);
            for edge in &plan.outgoing[node] {
                self.edges.remove(edge);
            }
        }
        self.run_q.push_back((target, Some((payload, pin))));
    }
}

/// Payloads merged in order: a key from a later one wins; one payload is
/// passed on unchanged.
fn merge(mut payloads: Vec<Value>) -> Value {
    if payloads.len() == 1 {
        return payloads.pop().unwrap_or(Value::Null);
    }
    let mut merged = Value::Object(Map::new());
    for payload in payloads {
        match (&mut merged, payload) {
            (Value::Object(into), Value::Object(from)) => into.extend(from),
            (_, other) => merged = other,
        }
    }
    merged
}

impl BasicPipelineEngine {
    /// Runs one frame to its end: every member answered or skipped.
    pub(super) fn run_scope<'s, 'r: 's>(
        &'s self,
        graph: &'s PipelineGraph,
        plan: &'s FlowPlan,
        scope: &'s mut ScopeRun,
        run: &'s mut RunState<'r>,
    ) -> BoxFuture<'s, Result<(), PipelineError>> {
        Box::pin(async move {
            for node in scope.members.clone() {
                scope.check(plan, node);
            }
            loop {
                if let Some((node, reentry)) = scope.run_q.pop_front() {
                    if !scope.pending(node) {
                        continue;
                    }
                    let delivery = match reentry {
                        Some((payload, input_pin)) => Delivery { payload, input_pin, series: scope.series.clone(), loop_items: None },
                        None if scope.resolved(plan, node) == Some(true) => scope.delivery(plan, graph, node),
                        None => continue,
                    };
                    self.step(graph, plan, scope, run, node, delivery).await?;
                    continue;
                }
                if let Some(node) = scope.skip_q.pop_front() {
                    if scope.pending(node) && scope.resolved(plan, node) == Some(false) {
                        skip(plan, scope, run, node);
                    }
                    continue;
                }
                break;
            }
            Ok(())
        })
    }

    /// Runs one ready node and settles the edges out of it.
    async fn step(
        &self,
        graph: &PipelineGraph,
        plan: &FlowPlan,
        scope: &mut ScopeRun,
        run: &mut RunState<'_>,
        node: usize,
        delivery: Delivery,
    ) -> Result<(), PipelineError> {
        let base = plan.loops.contains_key(&node).then(|| delivery.payload.clone());
        let ran = self.run_node(graph, plan, node, delivery, run).await?;
        scope.nodes.insert(node, NodeState::Done);
        match ran {
            Ran::Routed(payload) => {
                for &edge in &plan.outgoing[node] {
                    let state = if plan.edges[edge].from_pin == super::flow::ERROR_PIN {
                        EdgeState::Delivered(payload.clone())
                    } else {
                        EdgeState::Skipped
                    };
                    if !plan.edges[edge].reentry {
                        scope.set_edge(plan, edge, state);
                    }
                }
                if base.is_some() {
                    scope.set_loop(plan, node, LoopOutcome::Skipped);
                }
            }
            Ran::Answered(outs) => {
                if let Some(base) = base {
                    let items = self.run_loop(graph, plan, node, &outs, run).await?;
                    for &edge in &plan.outgoing[node] {
                        if plan.edges[edge].from_pin != logic::foreach_::OUTPUT_PIN_ITEM {
                            scope.set_edge(plan, edge, EdgeState::Skipped);
                        }
                    }
                    scope.set_loop(plan, node, LoopOutcome::Ran { base, items });
                    return Ok(());
                }
                let mut by_pin: HashMap<&str, &Value> = HashMap::new();
                for out in &outs {
                    for pin in &out.output_pins {
                        by_pin.insert(pin.as_str(), &out.payload);
                    }
                }
                let mut reentry = None;
                for &edge in &plan.outgoing[node] {
                    let e = &plan.edges[edge];
                    match (by_pin.get(e.from_pin.as_str()), e.reentry) {
                        (Some(payload), true) => reentry = Some((e.to, (*payload).clone(), e.to_pin.clone())),
                        (None, true) => {}
                        (Some(payload), false) => scope.set_edge(plan, edge, EdgeState::Delivered((*payload).clone())),
                        (None, false) => scope.set_edge(plan, edge, EdgeState::Skipped),
                    }
                }
                if let Some((target, payload, pin)) = reentry {
                    scope.reenter(plan, target, payload, pin);
                }
            }
        }
        Ok(())
    }

    /// The frames of one loop, in item order, each run to its end before the
    /// next; per close, what each frame delivered into it (an item whose
    /// edges into the close were all skipped is left out).
    async fn run_loop(
        &self,
        graph: &PipelineGraph,
        plan: &FlowPlan,
        foreach: usize,
        outs: &[NodeExecutionOutput],
        run: &mut RunState<'_>,
    ) -> Result<HashMap<usize, Vec<Value>>, PipelineError> {
        let Some(lp) = plan.loops.get(&foreach) else {
            return Ok(HashMap::new());
        };
        let mut items: HashMap<usize, Vec<Value>> = lp.closes.iter().map(|&c| (c, Vec::new())).collect();
        for out in outs {
            let field = |name: &str| out.payload.get(name).cloned().unwrap_or(Value::Null);
            let series = json!({ "item": field("item"), "index": field("index"), "count": field("count") });
            let mut frame = ScopeRun::new(lp.body.clone(), Some(series));
            let emitted = out.output_pins.iter().any(|p| p == logic::foreach_::OUTPUT_PIN_ITEM);
            for &edge in &plan.outgoing[foreach] {
                if plan.edges[edge].from_pin == logic::foreach_::OUTPUT_PIN_ITEM {
                    let state = if emitted { EdgeState::Delivered(out.payload.clone()) } else { EdgeState::Skipped };
                    frame.edges.insert(edge, state);
                }
            }
            run.frames.push(HashMap::new());
            let result = self.run_scope(graph, plan, &mut frame, run).await;
            run.frames.pop();
            result?;
            for &close in &lp.closes {
                let delivered: Vec<Value> = plan.incoming[close]
                    .iter()
                    .filter_map(|edge| match frame.edges.get(edge) {
                        Some(EdgeState::Delivered(payload)) => Some(payload.clone()),
                        _ => None,
                    })
                    .collect();
                if !delivered.is_empty()
                    && let Some(list) = items.get_mut(&close)
                {
                    list.push(merge(delivered));
                }
            }
        }
        Ok(items)
    }
}

/// A member with every input skipped: recorded, and every edge out of it
/// skipped (a foreach's closes with it).
fn skip(plan: &FlowPlan, scope: &mut ScopeRun, run: &mut RunState<'_>, node: usize) {
    scope.nodes.insert(node, NodeState::Skipped);
    run.record_skip(node);
    for &edge in &plan.outgoing[node] {
        if !plan.edges[edge].reentry {
            scope.set_edge(plan, edge, EdgeState::Skipped);
        }
    }
    if plan.loops.contains_key(&node) {
        scope.set_loop(plan, node, LoopOutcome::Skipped);
    }
}

//! The compiled lane: a pipeline that is one straight line, compiled once
//! and then only run.
//!
//! The interpreter prepares every run from the graph: it plans the flow,
//! builds each node from its config, finds and resolves each `{{ }}`, and
//! schedules the nodes through queues and edge states. For a pipeline whose
//! nodes form one line from its trigger, none of that changes between runs,
//! so [`Program::compile`] does it once: the flow plan, every node without
//! `{{ }}` built, every config with `{{ }}` compiled into templates
//! ([`CompiledConfig`]). [`BasicPipelineEngine::run_compiled`] then walks
//! the line, handing each node's output to the next.
//!
//! What a node does and what the run records is the interpreter's own code:
//! each step runs through `run_node`, `execute_dispatch` and `finish_node`,
//! and a node with nothing delivered is recorded by `record_skip`. The
//! compiled lane only skips the preparation, so a run answers, records and
//! fails the same in both lanes; `tests` holds the two to that.
//!
//! A program lives in the [`CompiledLane`] of an active pipeline. It is
//! compiled on the pipeline's first run (with that run's engine, which
//! carries the project's stores) and compiled again after any project
//! configuration changed. A pipeline that is not one straight line has no
//! program, and runs in the interpreter.

use std::sync::Arc;

use arc_swap::ArcSwapOption;

use super::{BasicPipelineEngine, NodeDispatch, NodesRetentionPlan, flow, logic, scheduler};
use crate::pipeline::expr::CompiledConfig;
use crate::pipeline::model::ExecuteOptions;
use crate::pipeline::{PipelineContext, PipelineError, PipelineGraph, PipelineOutput};

/// Where an active pipeline keeps its program, and the configuration
/// generation it was compiled at.
#[derive(Default)]
pub struct CompiledLane(ArcSwapOption<Compiled>);

struct Compiled {
    generation: u64,
    /// `None`: the pipeline is not one straight line; it runs in the
    /// interpreter, and that is remembered until the generation moves.
    program: Option<Arc<Program>>,
}

impl std::fmt::Debug for CompiledLane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = match self.0.load().as_deref() {
            None => "not compiled",
            Some(Compiled { program: Some(_), .. }) => "compiled",
            Some(Compiled { program: None, .. }) => "interpreted",
        };
        f.write_str(state)
    }
}

impl CompiledLane {
    /// The program for `graph`, compiled now when there is none for the
    /// current configuration generation.
    pub(super) fn program(&self, engine: &BasicPipelineEngine, graph: &PipelineGraph) -> Option<Arc<Program>> {
        let generation = crate::platform::services::project_config::config_generation();
        if let Some(compiled) = self.0.load().as_deref()
            && compiled.generation == generation
        {
            return compiled.program.clone();
        }
        let program = Program::compile(engine, graph).map(Arc::new);
        self.0.store(Some(Arc::new(Compiled { generation, program: program.clone() })));
        program
    }

    /// Whether the last compile made a program; `None` before the first run.
    pub fn is_compiled(&self) -> Option<bool> {
        self.0.load().as_deref().map(|compiled| compiled.program.is_some())
    }
}

/// One straight-line pipeline, compiled.
pub(super) struct Program {
    plan: flow::FlowPlan,
    retention: NodesRetentionPlan,
    /// In run order, from the trigger to the end of the line.
    steps: Vec<Step>,
}

/// One node of the line.
pub(super) struct Step {
    idx: usize,
    prepared: Prepared,
}

enum Prepared {
    /// No `{{ }}`: built once, from the config as stored.
    Ready(NodeDispatch),
    /// `{{ }}` compiled; built each run from the filled config.
    Templated(CompiledConfig),
    /// Built each run as the interpreter builds it: a node whose build reads
    /// what can change between runs (a page's template markup), or one that
    /// is not known to be safe to share across runs.
    EachRun,
}

impl Step {
    pub(super) fn ready(&self) -> Option<&NodeDispatch> {
        match &self.prepared {
            Prepared::Ready(dispatch) => Some(dispatch),
            _ => None,
        }
    }

    pub(super) fn templates(&self) -> Option<&CompiledConfig> {
        match &self.prepared {
            Prepared::Templated(templates) => Some(templates),
            _ => None,
        }
    }
}

/// Kinds whose answer is not the one payload they were delivered: a
/// collect gathers what several edges delivered, and the loop and retry
/// nodes steer the scheduler.
fn needs_the_scheduler(kind: &str) -> bool {
    kind == logic::collect::NODE_KIND
        || kind == logic::reduce::NODE_KIND
        || kind == logic::foreach_::NODE_KIND
        || kind == logic::retry::NODE_KIND
}

/// A built node that holds only its config and the project's shared
/// services, so one build serves every run, concurrently.
fn shareable(dispatch: &NodeDispatch) -> bool {
    matches!(
        dispatch,
        NodeDispatch::Webhook(_)
            | NodeDispatch::Manual(_)
            | NodeDispatch::WsTrigger(_)
            | NodeDispatch::WebResponse(_)
            | NodeDispatch::Kv(_)
            | NodeDispatch::WsState(_)
            | NodeDispatch::WsRoom(_)
            | NodeDispatch::Script(_)
    )
}

impl Program {
    /// `None` unless the graph is one straight line the scheduler adds
    /// nothing to: one entry, every node on the line, each joined to the
    /// next by one edge, no loop, no `:error` edge, no re-entry.
    fn compile(engine: &BasicPipelineEngine, graph: &PipelineGraph) -> Option<Program> {
        let (retention, plan) = engine.flow_plan(graph).ok()?;
        if !plan.loops.is_empty()
            || plan.entries.len() != 1
            || plan.edges.iter().any(|edge| edge.reentry || edge.from_pin == flow::ERROR_PIN)
            || graph.nodes.iter().any(|node| needs_the_scheduler(&node.kind))
        {
            return None;
        }
        let mut order = vec![plan.entries[0]];
        loop {
            let at = *order.last()?;
            match plan.outgoing[at].as_slice() {
                [] => break,
                [edge] => {
                    let next = plan.edges[*edge].to;
                    if plan.incoming[next].len() != 1 || order.contains(&next) {
                        return None;
                    }
                    order.push(next);
                }
                _ => return None,
            }
        }
        if order.len() != graph.nodes.len() {
            return None;
        }
        let mut steps = Vec::with_capacity(order.len());
        for idx in order {
            let node = &graph.nodes[idx];
            let prepared = match CompiledConfig::of(&node.config) {
                Some(templates) => Prepared::Templated(templates),
                None => match engine.build_node(node) {
                    Ok(dispatch) if shareable(&dispatch) => Prepared::Ready(dispatch),
                    Ok(_) => Prepared::EachRun,
                    // A node that does not build fails the same way at run
                    // time; the interpreter reports it.
                    Err(_) => return None,
                },
            };
            steps.push(Step { idx, prepared });
        }
        Some(Program { plan, retention, steps })
    }
}

impl BasicPipelineEngine {
    /// One run of a compiled pipeline: the line walked from the trigger,
    /// each node's output delivered to the next as the scheduler would
    /// deliver it on that edge, and every node after one that delivered
    /// nothing recorded as skipped.
    pub(super) async fn run_compiled(
        &self,
        program: &Program,
        graph: &PipelineGraph,
        ctx: &PipelineContext,
        options: &ExecuteOptions,
    ) -> Result<PipelineOutput, PipelineError> {
        let mut run = self.run_state(graph, ctx, options, program.retention.clone())?;
        let entry = program.steps.first().map(|step| step.idx).unwrap_or_default();
        let mut delivery = Some(scheduler::Delivery {
            payload: ctx.input.clone(),
            input_pin: graph.nodes[entry].input_pins.first().cloned().unwrap_or_default(),
            series: None,
            loop_items: None,
        });
        for step in &program.steps {
            let Some(given) = delivery.take() else {
                run.record_skip(step.idx);
                continue;
            };
            let ran = self.run_node(graph, &program.plan, step.idx, given, &mut run, Some(step)).await?;
            let scheduler::Ran::Answered(outs) = ran else {
                // A routed failure needs an `:error` edge, and a compiled
                // line has none.
                unreachable!("a compiled pipeline has no :error edge to route to");
            };
            if let Some(&edge) = program.plan.outgoing[step.idx].first() {
                let edge = &program.plan.edges[edge];
                // The last output on the edge's pin, as the scheduler takes it.
                delivery = outs
                    .into_iter()
                    .rev()
                    .find(|out| out.output_pins.iter().any(|pin| *pin == edge.from_pin))
                    .map(|out| scheduler::Delivery {
                        payload: out.payload,
                        input_pin: edge.to_pin.clone(),
                        series: None,
                        loop_items: None,
                    });
            }
        }
        Ok(run.finish(&program.plan))
    }
}

#[cfg(test)]
mod tests {
    //! Each pipeline runs in both lanes, on fresh stores, and the two must
    //! agree on everything a caller or a reader of the record sees: the
    //! value, the response sent, the record of every node (all but its
    //! duration), and a failure's code, message and node.

    use std::sync::Arc;

    use serde_json::{Value, json};

    use super::CompiledLane;
    use crate::infra::io::state::{DynStateBus, MemStateBus};
    use crate::pipeline::engines::basic::BasicPipelineEngine;
    use crate::pipeline::interface::PipelineEngine;
    use crate::pipeline::model::{ExecuteOptions, Responder};
    use crate::pipeline::PipelineContext;
    use crate::platform::shell::parser::build_pipeline_graph;

    /// Everything a run shows, durations taken out.
    fn shown(result: Result<crate::pipeline::PipelineOutput, crate::pipeline::PipelineError>, sent: Option<Value>) -> Value {
        let scrub = |trace: &Vec<crate::pipeline::model::NodeTraceEntry>| {
            let mut v = serde_json::to_value(trace).expect("trace");
            for entry in v.as_array_mut().into_iter().flatten() {
                entry["duration_ms"] = json!(0);
            }
            v
        };
        let run = match result {
            Ok(out) => json!({ "ok": true, "value": out.value, "response": out.response, "trace": out.trace, "nodes": scrub(&out.node_trace) }),
            Err(e) => json!({ "ok": false, "code": e.code, "message": e.message, "node": e.node_id, "nodes": scrub(&e.node_trace) }),
        };
        json!({ "run": run, "sent": sent })
    }

    async fn run_once(engine: &BasicPipelineEngine, graph: &crate::pipeline::PipelineGraph, input: &Value, lane: Option<Arc<CompiledLane>>) -> Value {
        let (responder, mut answer) = Responder::channel();
        let options = ExecuteOptions { responder: Some(responder), lane, ..Default::default() };
        let ctx = PipelineContext {
            owner: "test".into(),
            project: "test".into(),
            pipeline: "lane-test".into(),
            request_id: "req-lane".into(),
            route: "/lane".into(),
            input: input.clone(),
            trigger: None,
            placeholder: None,
        };
        let result = engine.execute_with_options_async(graph, &ctx, &options).await;
        shown(result, answer.try_recv().ok())
    }

    /// Runs `dsl` interpreted, then compiled twice (the second run on the
    /// program the first compiled), and returns whether it compiled.
    async fn same_in_both_lanes(dsl: &str, input: Value) -> bool {
        let graph = build_pipeline_graph("lane-test", dsl).expect("graph");
        let fresh = || {
            let bus: DynStateBus = Arc::new(MemStateBus::new());
            BasicPipelineEngine::default().with_state_bus(bus)
        };
        let interpreted = run_once(&fresh(), &graph, &input, None).await;
        let lane = Arc::new(CompiledLane::default());
        let engine = fresh();
        let compiled = run_once(&engine, &graph, &input, Some(lane.clone())).await;
        assert_eq!(compiled, interpreted, "first compiled run differs for:\n{dsl}");
        let again = run_once(&engine, &graph, &input, Some(lane.clone())).await;
        let interpreted_again = {
            // The interpreter's second run on its own store, for nodes whose
            // answer depends on what the first run wrote.
            let engine = fresh();
            let _ = run_once(&engine, &graph, &input, None).await;
            run_once(&engine, &graph, &input, None).await
        };
        assert_eq!(again, interpreted_again, "second compiled run differs for:\n{dsl}");
        lane.is_compiled().expect("the lane was asked")
    }

    #[tokio::test]
    async fn a_straight_line_answers_and_records_the_same_in_both_lanes() {
        let input = json!({ "id": 7, "name": "héllo", "lat": -6.2 });
        for dsl in [
            r#"| trigger.manual | web.response.send --body "{{ input.manual.id }}""#,
            r#"| trigger.manual | web.response.send --body ok"#,
            r#"| trigger.manual | javascript.script.run -- "return { x: input.manual.id * 2 };" | kv.entry.put --key "/a/{{ input.manual.id }}" --value "{{ input.manual }}" | web.response.send --body "{{ input.script }}""#,
            // A script the native compiler leaves to the sandbox.
            r#"| trigger.manual | javascript.script.run -- "const a = input.manual.id; return { a, n: input.manual.name.toUpperCase() };""#,
            // An expression only Deno answers, inside a compiled line.
            r#"| trigger.manual | web.response.send --body "{{ [1, 2, 3].map(x => x * input.manual.id) }}""#,
            // Two answering nodes: the second answers nobody.
            r#"| trigger.manual | web.response.send --body first | web.response.send --body second"#,
        ] {
            assert!(same_in_both_lanes(dsl, input.clone()).await, "should compile:\n{dsl}");
        }
    }

    /// An answer with nodes after it reaches the caller before they run, in
    /// both lanes: the run yields once after answering, so the HTTP ingress,
    /// which polls the run itself, answers at once and lets the rest go on.
    #[tokio::test]
    async fn an_answer_with_nodes_after_it_is_handed_over_before_they_run() {
        use std::future::Future;
        let dsl = r#"| trigger.manual | web.response.send --body ok | kv.entry.put --key after --value done"#;
        let graph = build_pipeline_graph("lane-test", dsl).expect("graph");
        for lane in [None, Some(Arc::new(CompiledLane::default()))] {
            let bus: DynStateBus = Arc::new(MemStateBus::new());
            let engine = BasicPipelineEngine::default().with_state_bus(bus.clone());
            let (responder, mut answer) = Responder::channel();
            let options = ExecuteOptions { responder: Some(responder), lane: lane.clone(), ..Default::default() };
            let ctx = PipelineContext {
                owner: "test".into(),
                project: "test".into(),
                pipeline: "lane-test".into(),
                request_id: "req-early".into(),
                route: "/early".into(),
                input: json!({}),
                trigger: None,
                placeholder: None,
            };
            let compiled = lane.is_some();
            let mut run = Box::pin(engine.execute_with_options_async(&graph, &ctx, &options));
            let waker = futures::task::noop_waker();
            let mut cx = std::task::Context::from_waker(&waker);
            assert!(run.as_mut().poll(&mut cx).is_pending(), "compiled={compiled}: the run yields after answering");
            assert!(answer.try_recv().is_ok(), "compiled={compiled}: the answer is handed over");
            assert_eq!(bus.get("test", "test", "after").expect("read"), None, "compiled={compiled}: the node after the answer has not run");
            run.await.expect("the run ends");
            assert_eq!(bus.get("test", "test", "after").expect("read"), Some(json!("done")), "compiled={compiled}: it runs after");
        }
    }

    #[tokio::test]
    async fn a_failure_fails_the_same_in_both_lanes() {
        let input = json!({ "id": 7 });
        for dsl in [
            r#"| trigger.manual | javascript.script.run -- "throw new Error('boom');" | web.response.send --body ok"#,
            r#"| trigger.manual | web.response.send --body "{{ input.missing.deep }}""#,
            // A key that comes out empty: the build fails in the run.
            r#"| trigger.manual | kv.entry.put --key "{{ input.manual.nothing ?? '' }}" --value 1"#,
        ] {
            assert!(same_in_both_lanes(dsl, input.clone()).await, "should compile:\n{dsl}");
        }
        // A node that cannot be built at all is the interpreter's to report,
        // the same way.
        assert!(!same_in_both_lanes(r#"| trigger.manual | kv.entry.put --key "" --value 1"#, input).await);
    }

    #[tokio::test]
    async fn nodes_references_and_skips_are_the_same_in_both_lanes() {
        let input = json!({ "id": 7 });
        let nodes = "[a] trigger.manual\n[b] javascript.script.run -- \"return { x: 1 };\"\n[c] web.response.send --body \"{{ $nodes.b.script.x }}\"\n[a] -> [b]\n[b] -> [c]";
        assert!(same_in_both_lanes(nodes, input.clone()).await);
        // `false` delivers nothing on `true`: the rest of the line is skipped.
        let skipped = "[a] trigger.manual\n[b] logic.if --when \"input.manual.id > 100\"\n[c] web.response.send --body yes\n[d] kv.entry.put --key /k --value 1\n[a] -> [b]\n[b]:true -> [c]\n[c] -> [d]";
        assert!(same_in_both_lanes(skipped, input.clone()).await);
    }

    #[tokio::test]
    async fn a_webhook_secret_is_struck_out_the_same_in_both_lanes() {
        let input = json!({ "body": { "user": "a", "password": "<password>" }, "query": {}, "params": {}, "method": "POST", "path": "/x" });
        let dsl = r#"| trigger.webhook --route /x --method POST | javascript.script.run -- "return { echo: input.webhook.body };" | web.response.send --body "{{ input.script }}""#;
        assert!(same_in_both_lanes(dsl, input).await);
    }

    #[tokio::test]
    async fn what_is_not_one_straight_line_runs_in_the_interpreter() {
        let input = json!({ "id": 7 });
        let fork = "[a] trigger.manual\n[b] web.response.send --body one\n[c] kv.entry.put --key /k --value 1\n[a] -> [b]\n[a] -> [c]";
        assert!(!same_in_both_lanes(fork, input.clone()).await);
        let routed = "[a] trigger.manual\n[b] javascript.script.run -- \"throw new Error('x');\"\n[c] web.response.send --body handled\n[a] -> [b]\n[b]:error -> [c]";
        assert!(!same_in_both_lanes(routed, input).await);
    }
}

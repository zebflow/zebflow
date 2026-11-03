//! A run must leave nothing behind in its runtime's heap.
//!
//! Each run used to compile a new setup script with that run's input and
//! `ctx` written into its source. V8 keeps every compiled script in its
//! compilation cache, so each request kept about 5 KiB: a script reading
//! `ctx.trigger` grew a process from 785 MB to 2,474 MB, and every
//! `logic.if` did the same.
//!
//! These tests drive the real nodes. A recorder stands in for the engine and
//! keeps what the node hands over; the test then runs exactly that on a
//! runtime of its own, the way a pool worker does, where it can read the heap.

use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

use super::super::engine::script_ctx;
use super::*;
use crate::language::{
    CompileOptions, CompiledDenoSandboxScript, CompiledProgram, DenoSandboxEngine, ExecutionContext, ExecutionOutput,
    LanguageEngine, LanguageError, ModuleSource, ProgramIr,
};
use crate::pipeline::nodes::{NodeExecutionInput, NodeHandler};

const RUNS: usize = 20_000;
const SAMPLE_EVERY: usize = 500;
/// The leak kept about 5,600 bytes per run here; a runtime that keeps
/// nothing stays far under this.
const MAX_KEPT_PER_RUN: f64 = 1024.0;

/// Compiles as the sandbox engine does, and keeps what a node hands over for
/// a run instead of running it.
#[derive(Default)]
struct Recorder {
    inner: DenoSandboxEngine,
    last: Mutex<Option<(CompiledDenoSandboxScript, Value, Value)>>,
}

impl LanguageEngine for Recorder {
    fn id(&self) -> &'static str {
        "test.recorder"
    }
    fn parse(&self, module: &ModuleSource) -> Result<ProgramIr, LanguageError> {
        self.inner.parse(module)
    }
    fn compile(&self, ir: &ProgramIr, options: &CompileOptions) -> Result<CompiledProgram, LanguageError> {
        self.inner.compile(ir, options)
    }
    fn run(&self, compiled: &CompiledProgram, input: Value, ctx: &ExecutionContext) -> Result<ExecutionOutput, LanguageError> {
        let script = serde_json::from_slice(&compiled.artifact).expect("sandbox artifact");
        *self.last.lock().expect("lock") = Some((script, input, script_ctx(ctx)));
        Ok(ExecutionOutput { value: Value::Bool(true), trace: Vec::new() })
    }
}

/// A webhook request as a pipeline hands it over, different on every
/// request the way a real one is.
fn trigger(i: usize) -> Value {
    json!({
        "kind": "webhook",
        "method": "GET",
        "path": "/trig",
        "request_id": format!("req-{i:08}"),
        "query": { "page": i.to_string() },
        "params": {},
        "headers": {
            "host": "site-a.example.com",
            "user-agent": "load-test/1.0",
            "accept": "application/json, text/plain, */*",
            "accept-language": "en-US,en;q=0.9",
            "x-forwarded-for": "203.0.113.10",
        },
        "body": null,
    })
}

fn request(i: usize) -> NodeExecutionInput {
    let payload = json!({ "webhook": trigger(i) });
    NodeExecutionInput {
        node_id: "b".to_string(),
        input_pin: "in".to_string(),
        payload: payload.clone(),
        metadata: json!({
            "project": "demo",
            "pipeline": "mem/trig",
            "request_id": format!("req-{i:08}"),
            "trigger": trigger(i),
            "nodes": { "a": payload },
        }),
        bus: None,
    }
}

/// Runs `node` `RUNS` times, each on a different request, and answers the
/// heap in use every `SAMPLE_EVERY` runs.
fn heap_through(node: &dyn NodeHandler, recorder: &Recorder, expect: Value) -> Vec<usize> {
    let tokio_rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("tokio");
    tokio_rt.block_on(async {
        let (mut js_rt, _, _) = build_runtime();
        let mut samples = Vec::new();
        for i in 0..RUNS {
            node.execute_async(request(i)).await.expect("node");
            let (script, input, ctx) = recorder.last.lock().expect("lock").take().expect("the node ran the sandbox");
            let work = ScriptWork { fn_source: script.fn_source, config: script.resolved_config, input, ctx };
            let out = execute_script(&mut js_rt, work).await.expect("run");
            assert_eq!(out, expect, "run {i}");
            if i % SAMPLE_EVERY == SAMPLE_EVERY - 1 {
                samples.push(js_rt.v8_isolate().get_heap_statistics().used_heap_size());
            }
        }
        samples
    })
}

/// Bytes kept per run: the least heap seen in the last quarter of the runs
/// against the least seen in the first. The least of a stretch is the heap
/// just after a collection, which is what the runtime actually holds.
fn kept_per_run(samples: &[usize]) -> f64 {
    let quarter = samples.len() / 4;
    let early = samples[..quarter].iter().min().copied().unwrap_or(0);
    let late = samples[samples.len() - quarter..].iter().min().copied().unwrap_or(0);
    let runs_between = (samples.len() - quarter) * SAMPLE_EVERY;
    late.saturating_sub(early) as f64 / runs_between as f64
}

fn assert_flat(what: &str, samples: &[usize]) {
    let kept = kept_per_run(samples);
    let mib: Vec<usize> = samples.iter().map(|b| b >> 20).collect();
    eprintln!("{what}: {kept:.0} bytes kept per run; heap MiB every {SAMPLE_EVERY} runs {mib:?}");
    assert!(kept < MAX_KEPT_PER_RUN, "{what} kept {kept:.0} bytes per run; heap MiB {mib:?}");
}

#[test]
fn a_script_reading_ctx_trigger_keeps_nothing_per_run() {
    let recorder = Arc::new(Recorder::default());
    let node = crate::pipeline::nodes::shared::script::Node::build(
        "b",
        &json!({ "source": "return { m: ctx.trigger.method };" }),
        &crate::pipeline::nodes::basic::javascript::LANGUAGE,
        recorder.clone(),
    )
    .expect("node");
    let samples = heap_through(&node, &recorder, json!({ "m": "GET" }));
    assert_flat("javascript.script.run reading ctx.trigger", &samples);
}

#[test]
fn logic_if_keeps_nothing_per_run() {
    use crate::pipeline::nodes::basic::logic::if_;
    let recorder = Arc::new(Recorder::default());
    let node = if_::Node::new("b", if_::Config { when: "true".to_string() }, recorder.clone()).expect("node");
    let samples = heap_through(&node, &recorder, json!(true));
    assert_flat("logic.if", &samples);
}

/// The setup takes the run's input and ctx once; a script that reaches for
/// them again gets null, never another run's.
#[test]
fn a_script_cannot_take_the_run_arguments_again() {
    let tokio_rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("tokio");
    tokio_rt.block_on(async {
        let (mut js_rt, _, _) = build_runtime();
        let script = DenoSandboxEngine::default()
            .compile_script(r#"return { again: globalThis["__zebflow_" + "run_args"](), m: ctx.trigger.method };"#, None)
            .expect("compile");
        let work = ScriptWork {
            fn_source: script.fn_source,
            config: script.resolved_config,
            input: json!({}),
            ctx: json!({ "trigger": trigger(1) }),
        };
        let out = execute_script(&mut js_rt, work).await.expect("run");
        assert_eq!(out, json!({ "again": null, "m": "GET" }));
    });
}

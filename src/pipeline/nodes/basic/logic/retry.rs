//! `n.logic.retry` — bounded retry control, fed two ways.
//!
//! **The error road.** The engine routes a node failure into `:error` edges
//! when they exist. `n.logic.retry` consumes that failure envelope
//! (`{ input, error, __zf_retry }`) and either emits `retry` with the original
//! upstream input plus updated retry metadata, or `failed` when the attempt
//! budget is exhausted. A `refused`-class error goes straight to `failed`:
//! running it again cannot help.
//!
//! **The verdict road.** A payload arriving on `in` from an ordinary edge is a
//! verdict, not a failure: `retry: true` on it (or `--when "<expr>"` true —
//! the same JavaScript over `input` that `logic.if --expr` reads) fires
//! `retry` with that payload, as many times as `--max-attempts` allows, then
//! `failed`; a false verdict passes the payload through on `done`. This is
//! what lets a poll loop wait without throwing — a wait is not an error, and
//! the canvas should never paint it red.
//!
//! Attempts are counted from `__zf_retry.attempt`: on the payload when the
//! author (or the engine's envelope) carried it, and from this node's own last
//! output through `$nodes.<self>` — the engine keeps a retry node in its own
//! scope for exactly this — so a loop whose payload is replaced on the way
//! round (`http.request` answers with a fresh body) still counts 1, 2, 3.
//!
//! **Two budgets, whichever runs out first.** `--max-attempts` is the count;
//! `--max-elapsed-ms` is wall time from the first attempt this node saw
//! (`__zf_retry.first_at`, ms epoch). The wait between attempts is
//! `--delay-ms`, grown by `--backoff` each attempt (a factor: 1 is a fixed
//! delay, 2 doubles it) and capped by `--max-delay-ms`; the wait just taken
//! is stamped as `__zf_retry.next_delay_ms`. A wait that would end past the
//! elapsed budget is not taken: `failed` fires at once, and
//! `__zf_retry.reason` / `__zf_retry.message` say which budget ran out.

use async_trait::async_trait;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};

use crate::language::{
    COMPILE_TARGET_BACKEND, CompileOptions, CompiledProgram, LanguageEngine, ModuleSource,
    SourceKind,
};
use crate::pipeline::expr::build_expression_scope_input;
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "n.logic.retry";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_RETRY: &str = "retry";
pub const OUTPUT_PIN_FAILED: &str = "failed";
pub const OUTPUT_PIN_DONE: &str = "done";
const RETRY_STATE_KEY: &str = "__zf_retry";
/// The verdict key read when `--when` is not given.
const VERDICT_KEY: &str = "retry";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Retry".to_string(),
        description: "Retry a node, fed either way. **From an `:error` pin** (`[b]:error -> [r]`): it receives the failure \
             envelope and fires `retry` with the original input (wire `[r]:retry -> [b]`) until `--max-attempts` is spent, \
             then `failed` with the last error; a `refused`-class error goes straight to `failed`. **From an ordinary edge** \
             (`[check] -> [r]`): the payload is a verdict — `retry: true` on it, or `--when \"<expr>\"` true (JavaScript over \
             `input`, as `logic.if --expr`), fires `retry` with that payload; false passes it through on `done`; the budget \
             spent fires `failed`. A poll loop waits this way without throwing, so the canvas shows a wait, never a failure. \
             `--delay-ms` waits between attempts; `--backoff 2` doubles that wait each attempt, `--max-delay-ms` caps the \
             grown wait, and `--max-elapsed-ms` is a wall-time budget from the first attempt — whichever budget runs out \
             first fires `failed`, and `__zf_retry.reason` names it. Attempts are counted in `__zf_retry.attempt` \
             (`$nodes.<r>.__zf_retry`); a retry loop without a `failed` edge swallows the give-up."
            .to_string(),
        input_schema: serde_json::json!({ "type": "object" }),
        output_schema: serde_json::json!({ "type": "object" }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![
            OUTPUT_PIN_RETRY.to_string(),
            OUTPUT_PIN_FAILED.to_string(),
            OUTPUT_PIN_DONE.to_string(),
        ],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag {
                flag: "--max-attempts".to_string(),
                config_key: "max_attempts".to_string(),
                description: "Maximum total attempts before routing to failed.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--delay-ms".to_string(),
                config_key: "delay_ms".to_string(),
                description: "Optional delay before retrying; the first wait when `--backoff` grows it.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--backoff".to_string(),
                config_key: "backoff".to_string(),
                description: "Factor the delay grows by each attempt: 1 (default) is a fixed delay, 2 doubles it (500, 1000, 2000, …)."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--max-delay-ms".to_string(),
                config_key: "max_delay_ms".to_string(),
                description: "Cap for a delay grown by `--backoff`.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--max-elapsed-ms".to_string(),
                config_key: "max_elapsed_ms".to_string(),
                description: "Wall-time budget from the first attempt; when spent (or the next wait would overrun it) `failed` fires even with attempts left."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--when".to_string(),
                config_key: "when".to_string(),
                description: "Verdict expression for a payload from an ordinary edge (JavaScript over `input`, like `logic.if --expr`); true retries, false is `done`. Without it the payload's `retry: true` is the verdict."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType};
            vec![
                NodeFieldDef {
                    name: "max_attempts".to_string(),
                    label: "Max Attempts".to_string(),
                    field_type: NodeFieldType::Text,
                    help: Some(
                        "Maximum total attempts including the original failed try.".to_string(),
                    ),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "delay_ms".to_string(),
                    label: "Delay (ms)".to_string(),
                    field_type: NodeFieldType::Text,
                    help: Some("Optional delay before emitting the retry path; the first wait when a backoff grows it.".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "backoff".to_string(),
                    label: "Backoff factor".to_string(),
                    field_type: NodeFieldType::Text,
                    help: Some("The delay grows by this factor each attempt. 1 (default) is fixed; 2 doubles it.".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "max_delay_ms".to_string(),
                    label: "Max delay (ms)".to_string(),
                    field_type: NodeFieldType::Text,
                    help: Some("Cap for a delay grown by the backoff.".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "max_elapsed_ms".to_string(),
                    label: "Max elapsed (ms)".to_string(),
                    field_type: NodeFieldType::Text,
                    help: Some("Wall-time budget from the first attempt; when spent, failed fires even with attempts left.".to_string()),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "when".to_string(),
                    label: "Retry when".to_string(),
                    field_type: NodeFieldType::Textarea,
                    rows: Some(3),
                    help: Some(
                        "Verdict for a payload from an ordinary edge: JS over `input`, true retries, false is done. Empty: the payload's `retry: true`."
                            .to_string(),
                    ),
                    ..Default::default()
                },
            ]
        },
        layout: vec![
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("max_attempts".to_string()),
                    LayoutItem::Field("delay_ms".to_string()),
                ],
            },
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("backoff".to_string()),
                    LayoutItem::Field("max_delay_ms".to_string()),
                    LayoutItem::Field("max_elapsed_ms".to_string()),
                ],
            },
            LayoutItem::Field("when".to_string()),
        ],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Three attempts at a flaky API", "logic.retry --max-attempts 3 --delay-ms 500")
                .note("Graph: `[b] http.request …`, `[b]:error -> [r]`, `[r]:retry -> [b]`, `[r]:failed -> [e] web.response --status 502`."),
            crate::pipeline::model::NodeExample::dsl("A rate-limited API", "logic.retry --max-attempts 6 --delay-ms 500 --backoff 2 --max-delay-ms 8000")
                .note("The waits grow 500, 1000, 2000, 4000, 8000 ms — the cap holds the last one. Add `--max-elapsed-ms 30000` to give up on wall time instead of on the count; `__zf_retry.reason` on the `failed` payload says which budget ran out."),
            crate::pipeline::model::NodeExample::dsl("Poll until a job is ready", "logic.retry --max-attempts 40 --delay-ms 5000")
                .input(serde_json::json!({ "retry": true, "state": "processing" }))
                .note("Graph: `[poll] http.request …`, `[check] script -- return { ...input, retry: d.status !== 'success', url: d.videoURL }`, `[poll] -> [check]`, `[check] -> [wait]`, `[wait]:retry -> [poll]`, `[wait]:done -> [download]`, `[wait]:failed -> [gaveup]`. The check never throws; `retry: true` is the wait, `retry: false` goes on through `done`."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(deserialize_with = "count_or_string")]
    pub max_attempts: usize,
    #[serde(default, deserialize_with = "millis_or_string")]
    pub delay_ms: Option<u64>,
    /// Factor the delay grows by each attempt; 1 (or absent) is a fixed delay.
    #[serde(default, deserialize_with = "factor_or_string")]
    pub backoff: Option<f64>,
    /// Cap for a delay grown by `backoff`.
    #[serde(default, deserialize_with = "millis_or_string")]
    pub max_delay_ms: Option<u64>,
    /// Wall-time budget from the first attempt, in ms.
    #[serde(default, deserialize_with = "millis_or_string")]
    pub max_elapsed_ms: Option<u64>,
    /// The verdict expression for the verdict road; `retry: true` when absent.
    #[serde(default)]
    pub when: Option<String>,
}

/// A number typed in the dialog arrives as a string; from the DSL it is a
/// number. Both spell the same setting.
fn millis_or_string<'de, D: Deserializer<'de>>(de: D) -> Result<Option<u64>, D::Error> {
    let raw = Value::deserialize(de)?;
    Ok(match raw {
        Value::Number(n) => n.as_u64().or_else(|| n.as_f64().filter(|f| *f >= 0.0).map(|f| f as u64)),
        Value::String(s) => {
            let s = s.trim();
            if s.is_empty() { None } else { s.parse::<u64>().ok() }
        }
        _ => None,
    })
}

fn factor_or_string<'de, D: Deserializer<'de>>(de: D) -> Result<Option<f64>, D::Error> {
    let raw = Value::deserialize(de)?;
    Ok(match raw {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => {
            let s = s.trim();
            if s.is_empty() { None } else { s.parse::<f64>().ok() }
        }
        _ => None,
    })
}

fn count_or_string<'de, D: Deserializer<'de>>(de: D) -> Result<usize, D::Error> {
    let raw = Value::deserialize(de)?;
    let parsed = match &raw {
        Value::Number(n) => n.as_u64().map(|n| n as usize),
        Value::String(s) => s.trim().parse::<usize>().ok(),
        _ => None,
    };
    parsed.ok_or_else(|| serde::de::Error::custom(format!("max_attempts must be a whole number, got {raw}")))
}

impl Config {
    /// The wait after attempt `attempt` (1-based): `delay_ms × backoff^(attempt-1)`,
    /// capped by `max_delay_ms`. 0 without a delay.
    pub fn delay_after(&self, attempt: usize) -> u64 {
        let Some(base) = self.delay_ms.filter(|d| *d > 0) else {
            return 0;
        };
        let factor = self.backoff.filter(|f| f.is_finite() && *f > 1.0);
        let grown = match factor {
            Some(f) => {
                let exp = attempt.saturating_sub(1) as i32;
                let v = (base as f64) * f.powi(exp);
                if v.is_finite() && v < u64::MAX as f64 { v.round() as u64 } else { u64::MAX }
            }
            None => base,
        };
        match self.max_delay_ms {
            Some(cap) => grown.min(cap),
            None => grown,
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Which budget ended a loop, for the `failed` payload and the trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GaveUp {
    Attempts,
    Elapsed,
}

impl GaveUp {
    fn reason(self) -> &'static str {
        match self {
            GaveUp::Attempts => "max_attempts",
            GaveUp::Elapsed => "max_elapsed_ms",
        }
    }
}

pub struct Node {
    config: Config,
    when: Option<CompiledProgram>,
    language: std::sync::Arc<dyn LanguageEngine>,
}

impl Node {
    pub fn new(
        node_id: &str,
        config: Config,
        language: std::sync::Arc<dyn LanguageEngine>,
    ) -> Result<Self, PipelineError> {
        if config.max_attempts == 0 {
            return Err(PipelineError::new(
                "FW_NODE_LOGIC_RETRY_CONFIG",
                "max_attempts must be greater than 0",
            ));
        }
        if let Some(factor) = config.backoff {
            if !factor.is_finite() || factor < 1.0 {
                return Err(PipelineError::new(
                    "FW_NODE_LOGIC_RETRY_CONFIG",
                    format!("--backoff must be a factor of 1 or more (got {factor}); 1 is a fixed delay, 2 doubles it"),
                ));
            }
        }
        let when = match config.when.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(expression) => Some(compile_when(node_id, expression, language.as_ref())?),
            None => None,
        };
        Ok(Self {
            config,
            when,
            language,
        })
    }
}

/// `--when` compiled the way `logic.if --expr` is: `input` is the payload,
/// `$trigger` and `$nodes` are in scope, the answer is `Boolean(...)`.
fn compile_when(
    node_id: &str,
    expression: &str,
    language: &dyn LanguageEngine,
) -> Result<CompiledProgram, PipelineError> {
    let source = format!(
        "var __scope = input;\n\
         var input = __scope.$input;\n\
         var $input = input;\n\
         var $item = __scope.$item;\n\
         var $index = __scope.$index;\n\
         var $count = __scope.$count;\n\
         var $trigger = __scope.$trigger || null;\n\
         var $nodes = __scope.$nodes || {{}};\n\
         return Boolean({expression});"
    );
    let module = ModuleSource {
        id: format!("logic.retry:{node_id}"),
        source_path: None,
        kind: SourceKind::Tsx,
        code: source,
    };
    let ir = language.parse(&module).map_err(|e| {
        PipelineError::new(
            "FW_NODE_LOGIC_RETRY_WHEN_PARSE",
            format!("node '{node_id}': --when: {e}"),
        )
    })?;
    language
        .compile(
            &ir,
            &CompileOptions {
                target: COMPILE_TARGET_BACKEND.to_string(),
                optimize_level: 1,
                emit_trace_hints: false,
            },
        )
        .map_err(|e| {
            PipelineError::new(
                "FW_NODE_LOGIC_RETRY_WHEN_COMPILE",
                format!("node '{node_id}': --when: {e}"),
            )
        })
}

fn retry_state(payload: &Value) -> Option<&Value> {
    payload.get(RETRY_STATE_KEY)
}

fn attempt_of(state: Option<&Value>) -> Option<usize> {
    state
        .and_then(|value| value.get("attempt"))
        .and_then(Value::as_u64)
        .map(|n| n as usize)
}

/// The engine's failure envelope, and nothing else: `input` beside an
/// `error` with a registered `code`, under retry state the engine stamped
/// with the failing node. A verdict payload may carry its own `error` (a
/// provider's answer, say); it is not this.
fn is_failure_envelope(payload: &Value) -> bool {
    payload.get("input").is_some()
        && payload
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(Value::as_str)
            .is_some()
        && retry_state(payload)
            .and_then(|s| s.get("failing_node_id"))
            .is_some()
}

/// The retry state carried into this run, wherever it rode: on the payload
/// itself (verdict road), on the failing node's input inside the engine's
/// envelope (error road — that input is what this node emitted last round),
/// or on this node's own last output in `$nodes.<self>`.
struct PriorState {
    attempt: usize,
    first_at: Option<u64>,
}

fn prior_state(input: &NodeExecutionInput) -> PriorState {
    let payload = &input.payload;
    let candidates = [
        retry_state(payload),
        payload.get("input").and_then(retry_state),
        input
            .metadata
            .get("nodes")
            .and_then(|nodes| nodes.get(&input.node_id))
            .and_then(retry_state),
    ];
    let attempt = candidates.iter().filter_map(|s| attempt_of(*s)).max().unwrap_or(0);
    let first_at = candidates
        .iter()
        .filter_map(|s| s.and_then(|v| v.get("first_at")).and_then(Value::as_u64))
        .min();
    PriorState { attempt, first_at }
}

/// The state this node stamps on what it emits. `failing_node_id` and kind
/// are kept from the engine's envelope on the error road, so the record
/// still says whose failure was retried.
struct StateStamp<'a> {
    attempt: usize,
    max_attempts: usize,
    first_at: u64,
    next_delay_ms: Option<u64>,
    gave_up: Option<(GaveUp, &'a str)>,
    engine: Option<&'a Value>,
}

impl StateStamp<'_> {
    fn to_value(&self) -> Value {
        let mut state = serde_json::Map::new();
        if let Some(Value::Object(engine)) = self.engine {
            for key in ["failing_node_id", "failing_node_kind"] {
                if let Some(v) = engine.get(key) {
                    state.insert(key.to_string(), v.clone());
                }
            }
        }
        state.insert("attempt".to_string(), json!(self.attempt));
        state.insert("max_attempts".to_string(), json!(self.max_attempts));
        state.insert("first_at".to_string(), json!(self.first_at));
        if let Some(delay) = self.next_delay_ms {
            state.insert("next_delay_ms".to_string(), json!(delay));
        }
        if let Some((why, message)) = self.gave_up {
            state.insert("reason".to_string(), json!(why.reason()));
            state.insert("message".to_string(), json!(message));
        }
        Value::Object(state)
    }

    /// The payload with the state on it; a non-object payload is wrapped.
    fn stamp(&self, payload: Value) -> Value {
        let state = self.to_value();
        match payload {
            Value::Object(mut map) => {
                map.insert(RETRY_STATE_KEY.to_string(), state);
                Value::Object(map)
            }
            other => json!({ "input": other, RETRY_STATE_KEY: state }),
        }
    }
}

/// The original input out of the engine's envelope, for the `retry` pin.
fn original_input(payload: &Value) -> Result<Value, PipelineError> {
    payload.get("input").cloned().ok_or_else(|| {
        PipelineError::new(
            "FW_NODE_LOGIC_RETRY_INPUT",
            "retry input payload missing `input`",
        )
    })
}

/// What the node decided for attempt `attempt`: wait `Some(delay)` and go
/// round again, or give up for the budget named.
enum Decision {
    Retry { delay_ms: u64 },
    GiveUp(GaveUp, String),
}

fn decide(config: &Config, attempt: usize, first_at: u64, now: u64) -> Decision {
    let elapsed = now.saturating_sub(first_at);
    if let Some(budget) = config.max_elapsed_ms {
        if elapsed >= budget {
            return Decision::GiveUp(
                GaveUp::Elapsed,
                format!(
                    "retry gave up: --max-elapsed-ms {budget} spent after {attempt} attempt(s) ({elapsed} ms elapsed)"
                ),
            );
        }
    }
    if attempt >= config.max_attempts {
        return Decision::GiveUp(
            GaveUp::Attempts,
            format!(
                "retry gave up: --max-attempts {} spent ({elapsed} ms elapsed)",
                config.max_attempts
            ),
        );
    }
    let delay_ms = config.delay_after(attempt);
    if let Some(budget) = config.max_elapsed_ms {
        if elapsed.saturating_add(delay_ms) > budget {
            return Decision::GiveUp(
                GaveUp::Elapsed,
                format!(
                    "retry gave up: --max-elapsed-ms {budget} would be overrun by the next wait of {delay_ms} ms after {attempt} attempt(s) ({elapsed} ms elapsed)"
                ),
            );
        }
    }
    Decision::Retry { delay_ms }
}

impl Node {
    async fn verdict(&self, input: &NodeExecutionInput) -> Result<bool, PipelineError> {
        let Some(compiled) = &self.when else {
            return Ok(input.payload.get(VERDICT_KEY) == Some(&Value::Bool(true)));
        };
        let metadata = &input.metadata;
        let text = |key: &str| {
            metadata
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let out = self
            .language
            .run(
                compiled,
                build_expression_scope_input(&input.payload, metadata),
                &crate::language::ExecutionContext {
                    project: text("project"),
                    pipeline: text("pipeline"),
                    request_id: text("request_id"),
                    trigger: metadata.get("trigger").cloned().unwrap_or(Value::Null),
                    metadata: metadata.clone(),
                },
            )
            .map_err(|e| {
                PipelineError::new(
                    "FW_NODE_LOGIC_RETRY_WHEN_RUN",
                    format!("node '{}': --when: {e}", input.node_id),
                )
            })?;
        Ok(match &out.value {
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
            Value::String(s) => !s.is_empty(),
            _ => true,
        })
    }

    /// One decision for both roads: wait and go round, or give up naming the
    /// budget. `payload` is what `retry` carries (the original input on the
    /// error road, the verdict payload itself on the verdict road);
    /// `failed_payload` is what `failed` carries.
    async fn retry_or_fail(
        &self,
        attempt: usize,
        prior: &PriorState,
        engine_state: Option<&Value>,
        payload: Value,
        failed_payload: Value,
        mut trace: Vec<String>,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let now = now_ms();
        let first_at = prior.first_at.unwrap_or(now);
        let mut stamp = StateStamp {
            attempt,
            max_attempts: self.config.max_attempts,
            first_at,
            next_delay_ms: None,
            gave_up: None,
            engine: engine_state,
        };
        match decide(&self.config, attempt, first_at, now) {
            Decision::Retry { delay_ms } => {
                trace.push(format!("next attempt in {delay_ms} ms"));
                stamp.next_delay_ms = Some(delay_ms);
                if delay_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                }
                Ok(NodeExecutionOutput {
                    output_pins: vec![OUTPUT_PIN_RETRY.to_string()],
                    payload: stamp.stamp(payload),
                    trace,
                })
            }
            Decision::GiveUp(why, message) => {
                trace.push(message.clone());
                stamp.gave_up = Some((why, &message));
                Ok(NodeExecutionOutput {
                    output_pins: vec![OUTPUT_PIN_FAILED.to_string()],
                    payload: stamp.stamp(failed_payload),
                    trace,
                })
            }
        }
    }

    /// The error road: the engine's envelope after a node failed.
    async fn after_error(
        &self,
        input: NodeExecutionInput,
        mut trace: Vec<String>,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let prior = prior_state(&input);
        // The engine already counted this failure; its stamp wins over what
        // this node remembers, which is one behind.
        let attempt = attempt_of(retry_state(&input.payload))
            .unwrap_or(1)
            .max(prior.attempt);
        trace.push("road=error".to_string());
        trace.push(format!("attempt={attempt}"));

        // A refusal is the caller's fault — a bad address, a wrong credential
        // kind, config that cannot work. Running it again changes nothing, so
        // the attempt budget is not spent on it: straight to the failed pin,
        // with the reason in the trace. Only `failed`-class errors (the world's
        // fault — a relay down, a timeout) are worth another try.
        let error_code = input
            .payload
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(Value::as_str);
        if let Some(code) = error_code {
            use crate::pipeline::error_class::{ErrorClass, class_of};
            if class_of(code) == ErrorClass::Refused {
                trace.push(format!("refused: {code} — retrying cannot help"));
                return Ok(NodeExecutionOutput {
                    output_pins: vec![OUTPUT_PIN_FAILED.to_string()],
                    payload: input.payload,
                    trace,
                });
            }
        }

        let original = original_input(&input.payload)?;
        let engine_state = retry_state(&input.payload).cloned();
        self.retry_or_fail(
            attempt,
            &prior,
            engine_state.as_ref(),
            original,
            input.payload,
            trace,
        )
        .await
    }

    /// The verdict road: a payload from an ordinary edge that says whether
    /// to go round again.
    async fn after_verdict(
        &self,
        input: NodeExecutionInput,
        mut trace: Vec<String>,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        // Counted from the payload when it carried the state, and from this
        // node's own last output otherwise — the higher wins, so a payload
        // replaced on the way round does not reset the loop.
        let prior = prior_state(&input);
        let attempt = prior.attempt + 1;
        trace.push("road=verdict".to_string());
        trace.push(format!("attempt={attempt}"));

        let again = self.verdict(&input).await?;
        trace.push(format!("verdict={}", if again { "retry" } else { "done" }));
        if !again {
            let stamp = StateStamp {
                attempt: attempt.saturating_sub(1),
                max_attempts: self.config.max_attempts,
                first_at: prior.first_at.unwrap_or_else(now_ms),
                next_delay_ms: None,
                gave_up: None,
                engine: None,
            };
            return Ok(NodeExecutionOutput {
                output_pins: vec![OUTPUT_PIN_DONE.to_string()],
                payload: stamp.stamp(input.payload),
                trace,
            });
        }
        let payload = input.payload;
        self.retry_or_fail(attempt, &prior, None, payload.clone(), payload, trace)
            .await
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        NODE_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[INPUT_PIN_IN]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_RETRY, OUTPUT_PIN_FAILED, OUTPUT_PIN_DONE]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let trace = vec![
            format!("node_kind={NODE_KIND}"),
            format!("max_attempts={}", self.config.max_attempts),
        ];
        if is_failure_envelope(&input.payload) {
            self.after_error(input, trace).await
        } else {
            self.after_verdict(input, trace).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn node(max_attempts: usize, when: Option<&str>) -> Node {
        node_with(Config {
            max_attempts,
            delay_ms: None,
            backoff: None,
            max_delay_ms: None,
            max_elapsed_ms: None,
            when: when.map(str::to_string),
        })
    }

    fn node_with(config: Config) -> Node {
        Node::new(
            "r",
            config,
            std::sync::Arc::new(crate::language::DenoSandboxEngine::default()),
        )
        .expect("node")
    }

    fn budgets(max_attempts: usize, delay_ms: u64, backoff: f64, max_delay_ms: Option<u64>, max_elapsed_ms: Option<u64>) -> Config {
        Config {
            max_attempts,
            delay_ms: Some(delay_ms),
            backoff: Some(backoff),
            max_delay_ms,
            max_elapsed_ms,
            when: None,
        }
    }

    fn run(node: &Node, payload: serde_json::Value) -> NodeExecutionOutput {
        run_with(node, payload, json!({}))
    }

    fn run_with(node: &Node, payload: serde_json::Value, metadata: serde_json::Value) -> NodeExecutionOutput {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(node.execute_async(NodeExecutionInput {
                node_id: "r".to_string(),
                input_pin: INPUT_PIN_IN.to_string(),
                payload,
                metadata,
                bus: None,
            }))
            .expect("retry node never errors")
    }

    /// A relay being down is worth another try.
    #[test]
    fn a_failed_class_error_is_retried() {
        let out = run(&node(3, None), json!({
            "input": { "email": "sari@example.test" },
            "error": { "code": "FW_NODE_MAIL_SEND", "message": "relay unreachable" },
            "__zf_retry": { "attempt": 1, "failing_node_id": "m" }
        }));
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_RETRY.to_string()]);
        assert_eq!(out.payload["email"], "sari@example.test");
        assert_eq!(out.payload["__zf_retry"]["attempt"], 1);
    }

    /// A bad address is the caller's fault. Retrying cannot help, so the
    /// attempt budget is not spent discovering that three times.
    #[test]
    fn a_refused_class_error_goes_straight_to_failed() {
        let out = run(&node(3, None), json!({
            "input": { "email": "not an address" },
            "error": { "code": "FW_NODE_MAIL_ADDRESS", "message": "not a mailbox" },
            "__zf_retry": { "attempt": 1, "failing_node_id": "m" }
        }));
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_FAILED.to_string()]);
        assert!(
            out.trace.iter().any(|t| t.contains("retrying cannot help")),
            "{:?}",
            out.trace
        );
    }

    /// The verdict road: `retry: true` fires `retry` with the payload as it
    /// came, the attempt counted on it.
    #[test]
    fn a_true_verdict_retries_with_the_original_payload_and_counts() {
        let out = run(&node(5, None), json!({ "retry": true, "state": "processing", "url": null }));
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_RETRY.to_string()]);
        assert_eq!(out.payload["state"], "processing");
        assert_eq!(out.payload["retry"], true);
        assert_eq!(out.payload["__zf_retry"]["attempt"], 1);
        assert_eq!(out.payload["__zf_retry"]["max_attempts"], 5);
        assert!(out.trace.iter().any(|t| t == "road=verdict"), "{:?}", out.trace);
    }

    /// The count survives a payload that forgot it: the node's own last
    /// output is in `$nodes.<self>`, and the higher count wins.
    #[test]
    fn the_count_comes_from_the_nodes_own_last_output_when_the_payload_lost_it() {
        let out = run_with(
            &node(5, None),
            json!({ "retry": true }),
            json!({ "nodes": { "r": { "retry": true, "__zf_retry": { "attempt": 3, "max_attempts": 5 } } } }),
        );
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_RETRY.to_string()]);
        assert_eq!(out.payload["__zf_retry"]["attempt"], 4);
    }

    /// A false verdict is `done`, with the payload passed through.
    #[test]
    fn a_false_verdict_is_done_with_the_payload() {
        let out = run(&node(5, None), json!({ "retry": false, "state": "success", "url": "https://x/v.mp4" }));
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_DONE.to_string()]);
        assert_eq!(out.payload["url"], "https://x/v.mp4");
        assert_eq!(out.payload["state"], "success");
    }

    /// The budget spent on a true verdict is `failed`, the payload kept.
    #[test]
    fn a_true_verdict_with_the_budget_spent_is_failed() {
        let out = run(&node(3, None), json!({ "retry": true, "state": "processing", "__zf_retry": { "attempt": 2 } }));
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_FAILED.to_string()]);
        assert_eq!(out.payload["state"], "processing");
        assert_eq!(out.payload["__zf_retry"]["attempt"], 3);
        assert_eq!(out.payload["__zf_retry"]["reason"], "max_attempts");
        assert!(
            out.trace.iter().any(|t| t.contains("--max-attempts 3 spent")),
            "{:?}",
            out.trace
        );
    }

    /// `--when` reads the payload as `input`, the way `logic.if --expr` does.
    #[test]
    fn when_is_a_logic_if_expression_over_input() {
        let node = node(5, Some("input.status !== 'success'"));
        let waiting = run(&node, json!({ "status": "processing" }));
        assert_eq!(waiting.output_pins, vec![OUTPUT_PIN_RETRY.to_string()]);
        let ready = run(&node, json!({ "status": "success" }));
        assert_eq!(ready.output_pins, vec![OUTPUT_PIN_DONE.to_string()]);
    }

    /// A verdict payload that happens to carry an `error` object (a
    /// provider's answer) is still a verdict, not the engine's envelope.
    #[test]
    fn a_verdict_payload_with_its_own_error_key_is_not_an_envelope() {
        let out = run(&node(5, None), json!({ "retry": false, "state": "error", "error": { "message": "provider said no" } }));
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_DONE.to_string()]);
    }
    /// The rate-limited-API example: 500 ms doubled each attempt, held at
    /// 8000 — the waits after attempts 1..6.
    #[test]
    fn the_backoff_sequence_doubles_and_the_cap_holds() {
        let config = budgets(6, 500, 2.0, Some(8000), None);
        let waits: Vec<u64> = (1..=6).map(|attempt| config.delay_after(attempt)).collect();
        assert_eq!(waits, vec![500, 1000, 2000, 4000, 8000, 8000]);
        // No backoff (or 1) is the fixed delay it always was; no delay is 0.
        assert_eq!(budgets(6, 500, 1.0, None, None).delay_after(4), 500);
        assert_eq!(
            Config { delay_ms: None, ..budgets(6, 500, 2.0, None, None) }.delay_after(4),
            0
        );
    }

    /// The wait the node took is stamped on the payload as `next_delay_ms`,
    /// and `first_at` is set once and carried round unchanged.
    #[test]
    fn the_stamp_carries_first_at_and_the_delay_taken() {
        let node = node_with(budgets(6, 10, 2.0, Some(30), None));
        let first = run(&node, json!({ "retry": true }));
        assert_eq!(first.output_pins, vec![OUTPUT_PIN_RETRY.to_string()]);
        let first_at = first.payload["__zf_retry"]["first_at"].as_u64().expect("first_at");
        assert!(first_at > 0);
        assert_eq!(first.payload["__zf_retry"]["next_delay_ms"], 10);

        let second = run(&node, first.payload.clone());
        assert_eq!(second.payload["__zf_retry"]["attempt"], 2);
        assert_eq!(second.payload["__zf_retry"]["first_at"], first_at);
        assert_eq!(second.payload["__zf_retry"]["next_delay_ms"], 20);

        let third = run(&node, second.payload.clone());
        assert_eq!(third.payload["__zf_retry"]["next_delay_ms"], 30, "capped by --max-delay-ms");
    }

    /// The error road carries `first_at` too: the failing node's input inside
    /// the envelope is what this node emitted last round.
    #[test]
    fn the_error_road_reads_first_at_from_the_envelopes_input_and_keeps_the_failing_node() {
        let node = node_with(budgets(5, 0, 1.0, None, None));
        let out = run(&node, json!({
            "input": { "email": "x@example.test", "__zf_retry": { "attempt": 1, "first_at": 1000 } },
            "error": { "code": "FW_NODE_MAIL_SEND", "message": "relay unreachable" },
            "__zf_retry": { "attempt": 2, "failing_node_id": "m", "failing_node_kind": "n.mail.send" }
        }));
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_RETRY.to_string()]);
        assert_eq!(out.payload["__zf_retry"]["attempt"], 2);
        assert_eq!(out.payload["__zf_retry"]["first_at"], 1000);
        assert_eq!(out.payload["__zf_retry"]["failing_node_id"], "m");
        assert_eq!(out.payload["__zf_retry"]["max_attempts"], 5);
    }

    /// The elapsed budget spent fires `failed` with the elapsed message even
    /// though attempts remain.
    #[test]
    fn the_elapsed_budget_spent_is_failed_with_the_elapsed_message() {
        let node = node_with(budgets(40, 0, 1.0, None, Some(5000)));
        let long_ago = now_ms() - 10_000;
        let out = run(&node, json!({ "retry": true, "__zf_retry": { "attempt": 2, "first_at": long_ago } }));
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_FAILED.to_string()]);
        assert_eq!(out.payload["__zf_retry"]["reason"], "max_elapsed_ms");
        let message = out.payload["__zf_retry"]["message"].as_str().unwrap_or_default();
        assert!(message.contains("--max-elapsed-ms 5000 spent"), "{message}");
        assert!(message.contains("3 attempt(s)"), "{message}");
        assert!(out.trace.iter().any(|t| t == message), "{:?}", out.trace);
    }

    /// A wait that would end past the elapsed budget is not taken: `failed`
    /// at once, naming the wait.
    #[test]
    fn a_wait_that_would_overrun_the_elapsed_budget_is_not_taken() {
        // 5000 ms budget, 4000 ms wait, and 2000 ms already gone.
        let node = node_with(budgets(40, 4000, 1.0, None, Some(5000)));
        let out = run(&node, json!({ "retry": true, "__zf_retry": { "attempt": 1, "first_at": now_ms() - 2000 } }));
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_FAILED.to_string()]);
        assert_eq!(out.payload["__zf_retry"]["reason"], "max_elapsed_ms");
        let message = out.payload["__zf_retry"]["message"].as_str().unwrap_or_default();
        assert!(message.contains("overrun by the next wait of 4000 ms"), "{message}");
    }

    /// Both budgets together: whichever runs out first names itself.
    #[test]
    fn both_budgets_together_whichever_first() {
        // Attempts run out first: elapsed is fine, the count is spent.
        let node = node_with(budgets(3, 0, 1.0, None, Some(60_000)));
        let out = run(&node, json!({ "retry": true, "__zf_retry": { "attempt": 2, "first_at": now_ms() } }));
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_FAILED.to_string()]);
        assert_eq!(out.payload["__zf_retry"]["reason"], "max_attempts");

        // Elapsed runs out first: attempts remain, the clock is spent.
        let node = node_with(budgets(3, 0, 1.0, None, Some(100)));
        let out = run(&node, json!({ "retry": true, "__zf_retry": { "attempt": 1, "first_at": now_ms() - 500 } }));
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_FAILED.to_string()]);
        assert_eq!(out.payload["__zf_retry"]["reason"], "max_elapsed_ms");

        // Neither: round again.
        let node = node_with(budgets(3, 0, 1.0, None, Some(60_000)));
        let out = run(&node, json!({ "retry": true, "__zf_retry": { "attempt": 1, "first_at": now_ms() } }));
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_RETRY.to_string()]);
    }

    /// A backoff under 1 would shrink the wait; it is refused at build time.
    #[test]
    fn a_backoff_under_one_is_refused() {
        let err = Node::new(
            "r",
            budgets(3, 100, 0.5, None, None),
            std::sync::Arc::new(crate::language::DenoSandboxEngine::default()),
        )
        .err()
        .expect("refused");
        assert_eq!(err.code, "FW_NODE_LOGIC_RETRY_CONFIG");
        assert!(err.message.contains("--backoff"), "{}", err.message);
    }
    /// The dialog saves a number as a string; the DSL sends a number. Both
    /// spell the same budgets.
    #[test]
    fn the_budgets_read_the_dialog_spelling_too() {
        let config: Config = serde_json::from_value(json!({
            "max_attempts": "6", "delay_ms": "500", "backoff": "2", "max_delay_ms": "8000", "max_elapsed_ms": ""
        }))
        .expect("dialog spelling");
        assert_eq!(config.max_attempts, 6);
        assert_eq!(config.delay_ms, Some(500));
        assert_eq!(config.backoff, Some(2.0));
        assert_eq!(config.max_delay_ms, Some(8000));
        assert_eq!(config.max_elapsed_ms, None);
        let config: Config = serde_json::from_value(json!({ "max_attempts": 6, "delay_ms": 500, "backoff": 2 })).expect("dsl spelling");
        assert_eq!(config.delay_after(3), 2000);
    }
}

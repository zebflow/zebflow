//! Control nodes. Their expressions are raw JavaScript in both versions;
//! only the flag names and two answers changed.

use serde_json::Value;

use super::{Code, Config, Rule};
use crate::platform::services::migration::RewriteContext;
use crate::platform::services::migration::graph::OldOutput;
use crate::platform::services::migration::node::NodeRewrite;

pub fn rule(kind: &str) -> Option<Rule> {
    Some(match kind {
        "n.logic.if" => Rule::new("logic.if", super::pass, logic_if).code(&[("expression", Code::Expression)]),
        "n.logic.match" => Rule::new("logic.match", super::pass, logic_match).code(&[("expression", Code::Expression)]),
        "n.logic.foreach" => Rule::new("logic.foreach", foreach_output, foreach).code(&[("items_expr", Code::Expression)]),
        "n.logic.reduce" => Rule::new("logic.reduce", reduce_output, reduce)
            .code(&[("init_expr", Code::ChangedScope), ("step_expr", Code::Expression)]),
        "n.logic.collect" => Rule::new("logic.collect", collect_output, collect),
        "n.logic.retry" => Rule::new("logic.retry", super::pass, retry).code(&[("when", Code::Expression)]),
        _ => return None,
    })
}

fn logic_if(n: &mut NodeRewrite<'_>) {
    n.kind("logic.if");
    n.rename("expression", "when");
}

fn logic_match(n: &mut NodeRewrite<'_>) {
    n.kind("logic.match");
    n.rename("expression", "from");
    n.keep("cases");
    n.keep("default");
}

fn foreach_output(config: &Config, _: &RewriteContext) -> OldOutput {
    let keys = [("item", "item"), ("index", "index"), ("count", "count")];
    if config.get("keep_input").and_then(Value::as_bool) == Some(true) {
        OldOutput::merge(&keys)
    } else {
        OldOutput::replace(&keys, None)
    }
}

fn foreach(n: &mut NodeRewrite<'_>) {
    n.kind("logic.foreach");
    n.rename("items_expr", "from");
    match n.take_str("dispatch").as_deref() {
        None | Some("seq") | Some("sequential") => {}
        Some(other) => n.unresolved(format!("dispatch `{other}` has no 0.11 equivalent (only sequential)")),
    }
    n.rename("chunk_size", "batch_size");
    n.keep("keep_input");
}

fn reduce_output(_: &Config, _: &RewriteContext) -> OldOutput {
    // 0.10 replaced the payload with the last accumulator.
    OldOutput::open("reduce")
}

fn reduce(n: &mut NodeRewrite<'_>) {
    n.kind("logic.reduce");
    n.rename("init_expr", "initial");
    n.rename("step_expr", "step");
    n.behaviour("for an empty list 0.10 never ran the reduce (nothing after it ran); 0.11 answers --initial once");
}

fn collect_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::unknown("0.10 answered one key per upstream node id; 0.11 merges the payloads and adds collect — read each with $nodes.<id>")
}

fn collect(n: &mut NodeRewrite<'_>) {
    n.kind("logic.collect");
}

fn retry(n: &mut NodeRewrite<'_>) {
    n.kind("logic.retry");
    n.keep("max_attempts");
    n.keep("backoff");
    n.keep("when");
    n.duration("delay_ms", "delay", "ms");
    n.duration("max_delay_ms", "max_delay", "ms");
    n.duration("max_elapsed_ms", "max_elapsed", "ms");
    if n.config.contains_key("max_elapsed") {
        n.behaviour("__zf_retry.reason reads max_elapsed, not max_elapsed_ms, when the time budget runs out");
    }
}

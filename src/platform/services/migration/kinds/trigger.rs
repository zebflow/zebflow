//! Triggers, run inputs, function calls and the concept stand-in. A 0.10
//! trigger passed its envelope on flat; a 0.11 trigger answers it under its
//! source (`input.body` → `input.webhook.body`).

use serde_json::{Value, json};

use super::{Config, Rule, words};
use crate::platform::services::migration::RewriteContext;
use crate::platform::services::migration::expr::whole_expression;
use crate::platform::services::migration::graph::{Mapping, OldOutput, Target, split};
use crate::platform::services::migration::node::NodeRewrite;

pub fn rule(kind: &str) -> Option<Rule> {
    if let Some(input) = kind.strip_prefix("n.input.") {
        return INPUT_KINDS.contains(&input).then(|| Rule::new("input", input_output, input_node).unprefixed());
    }
    Some(match kind {
        "n.trigger.webhook" => Rule::new("trigger.webhook", webhook_output, webhook),
        "n.trigger.schedule" => Rule::new("trigger.schedule", schedule_output, schedule),
        "n.trigger.manual" => Rule::new("trigger.manual", manual_output, manual),
        "n.trigger.function" => Rule::new("trigger.function", function_output, function_trigger),
        "n.trigger.mcp" => Rule::new("trigger.mcp", mcp_output, mcp),
        "n.trigger.weberror" => Rule::new("trigger.error", weberror_output, weberror),
        "n.trigger.ws" => Rule::new("trigger.room", room_output, room),
        "n.trigger.ws.client" => Rule::new("trigger.socket", socket_output, socket),
        "n.trigger.kv.subscribe" => Rule::new("trigger.topic", topic_output, topic),
        "n.function.call" => Rule::new("function.result.call", call_output, call).errors(call_error),
        "n.concept" => Rule::new("logic.concept", super::pass, concept),
        _ => return None,
    })
}

const INPUT_KINDS: &[&str] = &["text", "number", "boolean", "json", "file", "files", "image", "audio", "video"];

// ── webhook and friends ──────────────────────────────────────────────────────

fn webhook_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::replace(
        &[
            ("body", "webhook.body"),
            ("query", "webhook.query"),
            ("params", "webhook.params"),
            ("path", "webhook.path"),
            ("method", "webhook.method"),
            ("files", "webhook.files"),
            ("auth", "webhook.auth"),
        ],
        None,
    )
}

/// The auth guard `trigger.webhook` and `trigger.room` share.
fn guard(n: &mut NodeRewrite<'_>) {
    n.rename("auth_type", "auth");
    n.rename("auth_credential", "credential_id");
    if let Some(roles) = n.take("auth_required_role") {
        match words(&roles) {
            Some(list) if !list.is_empty() => {
                n.note(format!("auth_required_role {roles} → role {list:?}"));
                n.set("role", json!(list));
            }
            Some(_) => {}
            None => n.unresolved(format!("auth_required_role {roles} is not a list of roles")),
        }
    }
}

fn webhook(n: &mut NodeRewrite<'_>) {
    n.kind("trigger.webhook");
    n.rename("path", "route");
    n.keep("method");
    guard(n);
    n.keep("auth_optional");
    n.keep("errors");
}

fn schedule_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::replace(
        &[("fired_at", "schedule.fired_at"), ("node_id", "schedule.node_id"), ("trigger", "='schedule'")],
        None,
    )
}

fn schedule(n: &mut NodeRewrite<'_>) {
    n.kind("trigger.schedule");
    n.keep("cron");
    n.keep("timezone");
}

fn manual_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::open("manual")
}

fn manual(n: &mut NodeRewrite<'_>) {
    n.kind("trigger.manual");
}

fn function_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::open("function")
}

fn function_trigger(n: &mut NodeRewrite<'_>) {
    n.kind("trigger.function");
    n.keep("description");
    n.keep("examples");
    n.rename("output_schema", "result_schema");
    let params = n.take("params");
    match n.take("input_schema") {
        Some(schema) => n.set("schema", schema),
        None => {
            // 0.10 built the schema from `params` when no schema was given.
            let props = match params {
                Some(Value::String(text)) => serde_json::from_str::<Value>(&text).ok(),
                other => other,
            };
            if let Some(Value::Object(props)) = props {
                let required: Vec<Value> = props
                    .iter()
                    .filter(|(_, p)| p.get("required").and_then(Value::as_bool) == Some(true))
                    .map(|(k, _)| Value::String(k.clone()))
                    .collect();
                n.set("schema", json!({ "type": "object", "properties": props, "required": required }));
                n.note("params → schema (the schema 0.10 built from them)");
            }
        }
    }
}

fn mcp_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::replace(&[("tool_name", "mcp.tool_name"), ("arguments", "mcp.arguments")], Some("mcp"))
}

/// A 0.10 MCP tool was listed on the project's dev MCP; a 0.11 one is
/// published on an app route the owner names, behind the `--auth` they
/// choose (`published-mcp.md`). Neither can be guessed: written on the 0.10
/// node as `route` and `auth` (with `credential_id`, `role`), they carry
/// over; missing, the node is unresolved.
fn mcp(n: &mut NodeRewrite<'_>) {
    n.kind("trigger.mcp");
    n.rename("tool_name", "name");
    n.rename("tool_description", "description");
    if let Some(Value::String(params)) = n.take("parameters") {
        // 0.10's `name:type,…`, every one required.
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();
        for pair in params.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            let (name, kind) = pair.split_once(':').map(|(n, k)| (n.trim(), k.trim())).unwrap_or((pair, "string"));
            let kind = if ["string", "number", "integer", "boolean", "object", "array"].contains(&kind) { kind } else { "string" };
            properties.insert(name.to_string(), json!({ "type": kind }));
            required.push(Value::String(name.to_string()));
        }
        n.note(format!("parameters {params} → schema"));
        n.set("schema", json!({ "type": "object", "properties": properties, "required": required }));
    }
    let had_route = n.has("route");
    let had_auth = n.has("auth");
    n.keep("route");
    n.keep("auth");
    n.keep("credential_id");
    n.keep("role");
    if !had_route || !had_auth {
        n.unresolved(
            "0.10 listed this tool on the project's dev MCP; 0.11 publishes it only on a route of the mcp surface, behind an auth the owner chooses. \
             Set `route` (e.g. `/tools`) and `auth` (none, jwt or api_key, with `credential_id`) on this node, or remove the pipeline, then plan again",
        );
    }
}

fn weberror_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::open("error")
}

fn weberror(n: &mut NodeRewrite<'_>) {
    n.kind("trigger.error");
    match n.take("code") {
        None => {}
        Some(Value::Number(code)) => n.set("status", Value::Number(code)),
        Some(Value::String(code)) if code.trim().parse::<u16>().is_ok() => n.set("status", json!(code.trim().parse::<u16>().unwrap_or(0))),
        Some(Value::String(code)) if code.trim() == "*" => n.note("code * dropped (no --status catches every error)"),
        // A class of statuses, `4xx` or `5xx`, is kept as written.
        Some(Value::String(code)) if matches!(code.trim().to_ascii_lowercase().as_str(), "4xx" | "5xx") => {
            n.set("status", Value::String(code.trim().to_ascii_lowercase()))
        }
        Some(other) => n.unresolved(format!("code {other} is not a status (404) or a class of them (4xx, 5xx)")),
    }
}

fn room_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::replace(
        &[
            ("room_id", "room.room_id"),
            ("session_id", "room.session_id"),
            ("event", "room.event"),
            ("payload", "room.payload"),
            ("auth", "room.auth"),
        ],
        None,
    )
}

fn room(n: &mut NodeRewrite<'_>) {
    n.kind("trigger.room");
    n.keep("room");
    n.keep("event");
    guard(n);
}

fn socket_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::replace(
        &[
            ("url", "socket.url"),
            ("node_id", "socket.node_id"),
            ("message", "socket.message"),
            ("trigger", "='ws_client'"),
        ],
        None,
    )
}

fn socket(n: &mut NodeRewrite<'_>) {
    n.kind("trigger.socket");
    n.keep("url");
    n.keep("credential_id");
    let reconnect = n.take_bool("reconnect").unwrap_or(true);
    let attempts = n.take("max_reconnect_attempts").and_then(|v| v.as_u64().or_else(|| v.as_str()?.trim().parse().ok())).unwrap_or(0);
    if !reconnect {
        n.set("max_attempts", json!(0));
        n.note("reconnect false → max_attempts 0");
    } else if attempts > 0 {
        n.set("max_attempts", json!(attempts));
        n.note(format!("max_reconnect_attempts {attempts} → max_attempts {attempts}"));
    }
    n.duration("reconnect_delay_ms", "delay", "ms");
    n.duration("heartbeat_interval_ms", "heartbeat", "ms");
    n.rename("message_format", "parse");
}

fn topic_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::replace(
        &[
            ("channel", "topic.topic"),
            ("node_id", "topic.node_id"),
            ("message", "topic.message"),
            ("trigger", "='kv.subscribe'"),
        ],
        None,
    )
}

fn topic(n: &mut NodeRewrite<'_>) {
    n.kind("trigger.topic");
    n.rename("channel", "topic");
}

// ── run inputs ───────────────────────────────────────────────────────────────

fn input_output(config: &Config, _: &RewriteContext) -> OldOutput {
    let Some(name) = config.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty() && !n.contains("{{")) else {
        return OldOutput::unknown("a run input named at run time");
    };
    // 0.10 left the value where it was sent (and wrote a default there);
    // 0.11 answers it at its name.
    let mut keys = Vec::new();
    for slot in ["body", "files"] {
        let mut old = vec![slot.to_string()];
        old.extend(split(name));
        keys.push(Mapping { old, new: Target::Path(split(name)) });
    }
    OldOutput::Merge { keys }
}

fn input_node(n: &mut NodeRewrite<'_>) {
    let kind = n.old_kind.trim_start_matches("n.").to_string();
    n.kind(&kind);
    for key in ["name", "label", "optional", "default", "accept", "min", "max"] {
        n.keep(key);
    }
}

// ── function calls ───────────────────────────────────────────────────────────

fn call_output(config: &Config, context: &RewriteContext) -> OldOutput {
    let Some(slug) = config.get("function").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()) else {
        return OldOutput::unknown("no function is named");
    };
    match context.functions.get(slug) {
        Some(model) => model.clone(),
        None => OldOutput::unknown("the function it calls is not a 0.10 function pipeline of this project, so what it returned cannot be said"),
    }
}

fn call_error(_: &Config) -> OldOutput {
    // v0.10.12 answered `{ error: "CODE: message" }` on its error pin; 0.11
    // fails the node, and the engine delivers the payload kept plus
    // `result: { ok: false, error: { code, message } }`.
    OldOutput::replace(&[("error", "=`${@.result.error.code}: ${@.result.error.message}`")], None)
}

fn call(n: &mut NodeRewrite<'_>) {
    n.kind("function.result.call");
    n.keep("function");
    match n.take("input") {
        None => {
            // 0.10 sent the whole payload; 0.11 sends nothing by default.
            if let Some(expr) = n.implicit_whole() {
                n.note(format!("the whole payload sent → argument {expr}"));
                n.set("argument", Value::String(expr));
            }
        }
        Some(Value::String(text)) if whole_expression(&text).is_some() => n.set("argument", Value::String(text)),
        Some(Value::String(text)) => match serde_json::from_str::<Value>(&text) {
            Ok(Value::Object(map)) => n.set("argument", Value::Object(map)),
            _ => n.unresolved(format!("input `{text}` is not an object; 0.11 sends arguments as key=value")),
        },
        Some(Value::Object(map)) => n.set("argument", Value::Object(map)),
        Some(other) => n.unresolved(format!("input {other} is not an object; 0.11 sends arguments as key=value")),
    }
}

fn concept(n: &mut NodeRewrite<'_>) {
    n.kind("logic.concept");
    n.keep("text");
}

/// The result model of a function pipeline: what a 0.10 caller received
/// (the function's last payload) mapped to `result` in 0.11, which holds
/// the function's last node's answer. Exact only where the last node's
/// whole 0.10 answer is its 0.11 noun (a script's return value).
pub fn function_result(last: &OldOutput, noun: Option<&str>) -> OldOutput {
    let Some(noun) = noun else {
        return OldOutput::unknown("the function's last node answers no key in 0.11");
    };
    let retarget = |path: &[String]| -> Option<Vec<String>> {
        let (first, rest) = path.split_first()?;
        (first == noun).then(|| std::iter::once("result".to_string()).chain(rest.iter().cloned()).collect())
    };
    match last {
        OldOutput::Replace { keys, whole: Some(whole), rest } => {
            let Some(whole) = retarget(whole) else {
                return OldOutput::unknown("the function's last answer does not sit under its 0.11 key");
            };
            let mut mapped = Vec::new();
            for key in keys {
                let new = match &key.new {
                    Target::Path(path) => match retarget(path) {
                        Some(path) => Target::Path(path),
                        None => return OldOutput::unknown("the function's last answer does not sit under its 0.11 key"),
                    },
                    other => other.clone(),
                };
                mapped.push(Mapping { old: key.old.clone(), new });
            }
            OldOutput::Replace { keys: mapped, whole: Some(whole), rest: rest.clone() }
        }
        _ => OldOutput::unknown("the function's last node did not replace the payload with one answer in 0.10"),
    }
}

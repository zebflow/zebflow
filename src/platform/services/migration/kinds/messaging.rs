//! Websockets, crypto, the key-value store, tokens, scripts, the browser,
//! HTTP and mail.

use serde_json::{Map, Value, json};

use super::{Code, Config, Rule};
use crate::platform::services::migration::RewriteContext;
use crate::platform::services::migration::graph::{Mapping, OldOutput, Target, split};
use crate::platform::services::migration::node::NodeRewrite;

pub fn rule(kind: &str) -> Option<Rule> {
    Some(match kind {
        "n.ws.emit" => Rule::new("ws.message.send", super::pass, ws_emit),
        "n.ws.client.send" => Rule::new("ws.message.send", super::pass, ws_client_send),
        "n.ws.sync_state" => Rule::by(sync_state_kind, super::pass, ws_sync_state),
        "n.crypto" => Rule::by(crypto_kind, crypto_output, crypto),
        "n.kv.set" => Rule::new("kv.entry.put", super::pass, kv_set),
        "n.kv.get" => Rule::new("kv.entry.get", kv_get_output, kv_get),
        "n.kv.del" => Rule::new("kv.entry.delete", super::pass, kv_del),
        "n.kv.exists" => Rule::new("kv.entry.head", kv_exists_output, kv_exists),
        "n.kv.incr" => Rule::new("kv.entry.increment", kv_incr_output, kv_incr),
        "n.kv.expire" => Rule::new("kv.entry.expire", super::pass, kv_expire),
        "n.kv.publish" => Rule::new("kv.message.publish", super::pass, kv_publish),
        "n.auth.token.create" => Rule::new("auth.token.create", token_create_output, token_create),
        "n.auth.token.verify" => Rule::new("auth.token.verify", token_verify_output, token_verify),
        "n.script" => Rule::by(script_kind, script_output, script).code(&[("source", Code::Body)]),
        "n.browser.run" => Rule::new("browser.page.run", browser_output, browser).code(&[("code", Code::Foreign)]),
        "n.http.request" => {
            Rule::new("http.response.fetch", http_output, http).code(&[("request_bindings", Code::ExpressionMap)])
        }
        "n.mail.send" => Rule::new("mail.message.send", mail_output, mail),
        _ => return None,
    })
}

// ── websockets ───────────────────────────────────────────────────────────────

/// The room (and session) a 0.10 ws node read from the payload; 0.11 reads
/// them from the room trigger's envelope, so nothing is written when the
/// payload's came from that trigger.
fn room_and_session(n: &mut NodeRewrite<'_>, session_too: bool) {
    let keys: &[&str] = if session_too { &["room_id", "session_id"] } else { &["room_id"] };
    for key in keys {
        if *key == "room_id" && n.has("room") {
            continue;
        }
        match n.graph.resolve_input(n.index, &[key.to_string()]) {
            Ok(origin) if n.graph.nodes[origin.producer].kind == "n.trigger.ws" => {}
            Ok(_) if *key == "room_id" => {
                if let Ok(Some(expr)) = n.locate(&[key]) {
                    n.set("room", Value::String(format!("{{{{ {expr} }}}}")));
                    n.note(format!("the room read from input.room_id → room {{{{ {expr} }}}}"));
                }
            }
            Ok(origin) => n.unresolved(format!(
                "0.10 read session_id from the payload (node `{}` sets it); 0.11 reads it from the room trigger",
                n.graph.nodes[origin.producer].id
            )),
            Err(failure) if failure.dead => {}
            Err(failure) => n.unresolved(format!("the room this node sends to cannot be traced: {}", failure.why)),
        }
    }
}

/// The body a 0.10 ws node sent when none was set: the payload's own
/// `payload` field when it has one, else the whole payload.
fn implicit_ws_body(n: &mut NodeRewrite<'_>, key: &str) {
    match n.locate(&["payload"]) {
        Ok(Some(expr)) => {
            let expr = format!("{{{{ {expr} }}}}");
            n.note(format!("the body read from input.payload → {key} {expr}"));
            n.set(key, Value::String(expr));
        }
        Ok(None) => {
            if let Some(expr) = n.implicit_whole() {
                n.note(format!("the whole payload sent → {key} {expr}"));
                n.set(key, Value::String(expr));
            }
        }
        Err(why) => n.unresolved(format!("the body this node sent cannot be traced: {why}")),
    }
}

fn ws_emit(n: &mut NodeRewrite<'_>) {
    n.kind("ws.message.send");
    n.keep("event");
    n.keep("room");
    let to = n.take_str("to");
    let session = matches!(to.as_deref(), Some("session" | "others"));
    if let Some(to) = to {
        n.set("recipient", Value::String(to));
    }
    room_and_session(n, session);
    match n.take("payload") {
        Some(body) => n.set("body", body),
        None => implicit_ws_body(n, "body"),
    }
}

fn ws_client_send(n: &mut NodeRewrite<'_>) {
    n.kind("ws.message.send");
    n.keep("connection");
    match n.take("message") {
        Some(body) => n.set("body", body),
        None => {
            if let Some(expr) = n.implicit_whole() {
                n.note(format!("the whole payload sent → body {expr}"));
                n.set("body", Value::String(expr));
            }
        }
    }
}

fn sync_state_kind(config: &Config) -> String {
    match config.get("op").and_then(Value::as_str).unwrap_or("") {
        "merge" => "ws.state.update",
        "delete" => "ws.state.delete",
        _ => "ws.state.put",
    }
    .to_string()
}

fn ws_sync_state(n: &mut NodeRewrite<'_>) {
    let op = n.take_str("op").unwrap_or_else(|| "set".to_string());
    let kind = match op.as_str() {
        "set" => "ws.state.put",
        "merge" => "ws.state.update",
        "delete" => "ws.state.delete",
        other => {
            n.unresolved(format!("op `{other}` is not set, merge or delete"));
            "ws.state.put"
        }
    };
    n.kind(kind);
    n.keep("room");
    room_and_session(n, false);
    let path = n.take_str("path").or_else(|| n.take_str("state_key")).unwrap_or_else(|| "/".to_string());
    match placeholders(n, &path) {
        Some(key) => {
            if key != path {
                n.note(format!("key {path} → {key}"));
            }
            n.set("key", Value::String(key));
        }
        None => return,
    }
    if kind != "ws.state.delete" {
        match n.take("value") {
            Some(value) => n.set("value", value),
            None => implicit_ws_body(n, "value"),
        }
    }
    if n.take_bool("silent") == Some(true) {
        n.set("batch", Value::Bool(true));
        n.note("silent → batch");
    }
}

/// `{name}` placeholders (filled from the payload's top-level fields in
/// 0.10) as `{{ … }}` expressions.
fn placeholders(n: &mut NodeRewrite<'_>, path: &str) -> Option<String> {
    let mut out = String::new();
    let mut rest = path;
    while let Some(open) = rest.find('{') {
        if rest[open..].starts_with("{{") {
            let close = rest[open..].find("}}").map(|c| open + c + 2).unwrap_or(rest.len());
            out.push_str(&rest[..close]);
            rest = &rest[close..];
            continue;
        }
        let Some(close) = rest[open..].find('}') else { break };
        let name = &rest[open + 1..open + close];
        out.push_str(&rest[..open]);
        match n.locate(&[name]) {
            Ok(Some(expr)) => out.push_str(&format!("{{{{ {expr} }}}}")),
            Ok(None) => {
                n.unresolved(format!("the key placeholder {{{name}}} reads input.{name}, which no upstream node answers"));
                return None;
            }
            Err(why) => {
                n.unresolved(format!("the key placeholder {{{name}}} cannot be traced: {why}"));
                return None;
            }
        }
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    Some(out)
}

// ── crypto ───────────────────────────────────────────────────────────────────

fn crypto_kind(config: &Config) -> String {
    match config.get("op").and_then(Value::as_str).unwrap_or("") {
        "bcrypt_hash" | "argon2_hash" => "crypto.password.hash",
        "bcrypt_verify" | "argon2_verify" => "crypto.password.verify",
        "sha256" | "sha512" => "crypto.digest.create",
        "hmac_sha256" => "crypto.signature.sign",
        "base64_encode" => "crypto.base64.encode",
        "base64_decode" => "crypto.base64.decode",
        "random_hex" => "crypto.random.generate",
        _ => "crypto.digest.create",
    }
    .to_string()
}

fn crypto_output(config: &Config, _: &RewriteContext) -> OldOutput {
    let result = match config.get("op").and_then(Value::as_str).unwrap_or("") {
        "bcrypt_hash" | "argon2_hash" => "password.hash",
        "sha256" | "sha512" => "digest.value",
        "hmac_sha256" => "signature.value",
        "base64_encode" => "base64.value",
        "base64_decode" => "base64.text",
        "random_hex" => "random.value",
        _ => return OldOutput::Pass,
    };
    OldOutput::merge(&[("result", result)])
}

fn crypto(n: &mut NodeRewrite<'_>) {
    let op = n.take_str("op").unwrap_or_default();
    let kind = crypto_kind(&Map::from_iter([("op".to_string(), json!(op))]));
    n.kind(&kind);
    // The value: `--input` (the tag) or `--value` (79c7149); empty read
    // the payload's `input`.
    let value = match n.take("input").or_else(|| n.take("value")) {
        Some(value) => Some(value),
        None if op == "random_hex" => None,
        None => n.implicit(&["input"]).map(Value::String),
    };
    let role = match op.as_str() {
        "bcrypt_hash" | "argon2_hash" | "bcrypt_verify" | "argon2_verify" | "base64_decode" => "from",
        _ => "text",
    };
    if let Some(value) = value {
        n.set(role, value);
    }
    match op.as_str() {
        "bcrypt_hash" => {
            n.set("algorithm", json!("bcrypt"));
            n.keep("cost");
        }
        "argon2_hash" => {
            n.set("algorithm", json!("argon2"));
            n.take("cost");
        }
        "bcrypt_verify" | "argon2_verify" => {
            let hash = match n.take("hash") {
                Some(hash) => Some(hash),
                None => n.implicit(&["hash"]).map(Value::String),
            };
            if let Some(hash) = hash {
                n.set("hash", hash);
            }
            n.take("cost");
        }
        "sha256" => n.set("algorithm", json!("sha256")),
        "sha512" => n.set("algorithm", json!("sha512")),
        "hmac_sha256" => {
            n.take("key");
            n.unresolved("hmac_sha256 signed with a key given in the pipeline; 0.11 signs with an hmac credential — create one holding that key and set --credential");
        }
        "random_hex" => {
            let bytes = n.take("length").and_then(|v| v.as_u64()).unwrap_or(32);
            if bytes > 1024 {
                n.unresolved(format!("length {bytes} bytes is above the 0.11 ceiling of 1KiB"));
            } else {
                n.set("size", json!(format!("{bytes}B")));
                n.note(format!("length {bytes} → size {bytes}B"));
            }
        }
        "base64_encode" | "base64_decode" => {}
        other => n.unresolved(format!("op `{other}` does not exist")),
    }
    for key in ["cost", "length", "key", "hash"] {
        if n.take(key).is_some() {
            n.note(format!("{key} dropped (op {op} did not read it)"));
        }
    }
}

// ── key-value ────────────────────────────────────────────────────────────────

fn kv_common(n: &mut NodeRewrite<'_>, kind: &str) {
    n.kind(kind);
    n.keep("key");
    n.keep("durable");
}

fn ttl(n: &mut NodeRewrite<'_>) {
    match n.peek("ttl").cloned() {
        Some(Value::Number(number)) if number.as_u64() == Some(0) => {
            n.take("ttl");
            n.note("ttl 0 dropped (it meant no expiry)");
        }
        _ => n.duration("ttl", "ttl", "s"),
    }
}

fn kv_set(n: &mut NodeRewrite<'_>) {
    kv_common(n, "kv.entry.put");
    ttl(n);
    match n.take("value") {
        Some(value) => n.set("value", value),
        None => {
            if let Some(expr) = n.implicit_whole() {
                n.note(format!("the whole payload stored → value {expr}"));
                n.set("value", Value::String(expr));
            }
        }
    }
}

/// The key a 0.10 kv read answered under: `--out-key`, else the key itself.
fn answered_key(config: &Config, fallback_exists: bool) -> Option<String> {
    let text = |k: &str| config.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    if let Some(out) = text("out_key") {
        return Some(out);
    }
    if fallback_exists {
        return Some("exists".to_string());
    }
    text("key").filter(|k| !k.contains("{{"))
}

fn kv_answer(config: &Config, fallback_exists: bool, new: &str) -> OldOutput {
    match answered_key(config, fallback_exists) {
        Some(key) => OldOutput::Merge { keys: vec![Mapping { old: vec![key], new: Target::Path(split(new)) }] },
        None => OldOutput::unknown("the value was stored under a key computed at run time"),
    }
}

fn kv_get_output(config: &Config, _: &RewriteContext) -> OldOutput {
    kv_answer(config, false, "entry.value")
}

fn kv_exists_output(config: &Config, _: &RewriteContext) -> OldOutput {
    kv_answer(config, true, "entry.exists")
}

fn kv_incr_output(config: &Config, _: &RewriteContext) -> OldOutput {
    kv_answer(config, false, "entry.value")
}

fn kv_get(n: &mut NodeRewrite<'_>) {
    kv_common(n, "kv.entry.get");
    n.keep("default");
    if n.take("out_key").is_some() {
        n.note("out_key dropped (0.11 answers entry.value)");
    }
}

fn kv_del(n: &mut NodeRewrite<'_>) {
    kv_common(n, "kv.entry.delete");
}

fn kv_exists(n: &mut NodeRewrite<'_>) {
    kv_common(n, "kv.entry.head");
    if n.take("out_key").is_some() {
        n.note("out_key dropped (0.11 answers entry.exists)");
    }
}

fn kv_incr(n: &mut NodeRewrite<'_>) {
    kv_common(n, "kv.entry.increment");
    n.keep("amount");
    if n.take("out_key").is_some() {
        n.note("out_key dropped (0.11 answers entry.value)");
    }
}

fn kv_expire(n: &mut NodeRewrite<'_>) {
    kv_common(n, "kv.entry.expire");
    ttl(n);
}

fn kv_publish(n: &mut NodeRewrite<'_>) {
    n.kind("kv.message.publish");
    n.rename("channel", "topic");
    match n.take("payload") {
        Some(body) => n.set("body", body),
        None => {
            if let Some(expr) = n.implicit_whole() {
                n.note(format!("the whole payload published → body {expr}"));
                n.set("body", Value::String(expr));
            }
        }
    }
}

// ── tokens ───────────────────────────────────────────────────────────────────

fn token_create_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::replace(
        &[
            ("access_token", "token.access_token"),
            ("token_type", "token.token_type"),
            ("expires_in", "token.expires_in"),
            ("profile", "token.profile"),
        ],
        Some("token"),
    )
}

fn token_create(n: &mut NodeRewrite<'_>) {
    n.kind("auth.token.create");
    for key in ["credential_id", "claims", "issuer", "audience"] {
        n.keep(key);
    }
    n.duration("expires_in", "ttl", "s");
}

fn token_verify_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::replace(
        &[("claims", "token.claims"), ("sub", "=@.token.claims.sub"), ("reason", "token.reason")],
        None,
    )
}

fn token_verify(n: &mut NodeRewrite<'_>) {
    n.kind("auth.token.verify");
    for key in ["credential_id", "issuer", "audience"] {
        n.keep(key);
    }
    n.rename("token", "from");
}

// ── scripts ──────────────────────────────────────────────────────────────────

fn script_kind(config: &Config) -> String {
    match config.get("language").and_then(Value::as_str).unwrap_or("").trim() {
        "ts" | "typescript" => "typescript.script.run",
        _ => "javascript.script.run",
    }
    .to_string()
}

fn script_output(config: &Config, _: &RewriteContext) -> OldOutput {
    // A script whose every return is an object literal answered exactly
    // those keys; any other key read nothing.
    // `return { ...input, x }` kept the payload and added x.
    let source = config.get("source").and_then(Value::as_str).unwrap_or("return input;");
    let returned = crate::platform::services::migration::expr::returned(source);
    let keys = |any: &std::collections::BTreeSet<String>| -> Vec<Mapping> {
        any.iter().map(|k| Mapping { old: vec![k.clone()], new: Target::Path(vec!["script".to_string(), k.clone()]) }).collect()
    };
    // A script that spreads its payload reads it whole, so its rewrite
    // rebuilds the 0.10 payload first and what it returns is that payload
    // with its keys on top: every old key `k` is `script.k`.
    match returned {
        Some(r) if !r.spreads_input => OldOutput::Replace {
            keys: keys(&r.any),
            whole: Some(vec!["script".to_string()]),
            rest: crate::platform::services::migration::graph::Rest::Dead,
        },
        _ => OldOutput::open("script"),
    }
}

fn script(n: &mut NodeRewrite<'_>) {
    let language = n.take_str("language").unwrap_or_default();
    let kind = match language.trim() {
        "ts" | "typescript" => "typescript.script.run",
        "" | "js" | "javascript" => "javascript.script.run",
        other => {
            n.unresolved(format!("language `{other}` is not js or ts"));
            "javascript.script.run"
        }
    };
    n.kind(kind);
    if n.take("source_expr").is_some() {
        n.unresolved("source_expr computed the script's code at run time; 0.11 runs fixed code only");
    }
    match n.take("source") {
        Some(source) => n.set("source", source),
        None => {
            n.set("source", json!("return input;"));
            n.note("source = return input; (the 0.10 default, written out)");
        }
    }
}

// ── browser ──────────────────────────────────────────────────────────────────

fn browser_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::open("page")
}

/// A per-node timeout in milliseconds (clamped by the 0.10 node) as the
/// engine's own `--timeout`, unless the pipeline set that already.
fn node_timeout(n: &mut NodeRewrite<'_>, default_ms: u64, min_ms: u64, max_ms: u64) {
    let given = n.take("timeout_ms");
    if n.config.contains_key("timeout") {
        if given.is_some() {
            n.note("timeout_ms dropped (the engine timeout was already set)");
        }
        return;
    }
    let ms = match &given {
        None => default_ms,
        Some(value) => match value.as_u64().or_else(|| value.as_str().and_then(|s| s.trim().parse().ok())) {
            Some(ms) => ms.clamp(min_ms, max_ms),
            None => {
                n.unresolved(format!("timeout_ms {value} is not a number of milliseconds"));
                return;
            }
        },
    };
    if ms < 1000 {
        n.behaviour(format!("a {ms}ms timeout is below the 0.11 floor of 1s; it is 1s"));
    }
    let ms = ms.max(1000);
    let text = if ms % 1000 == 0 { format!("{}s", ms / 1000) } else { format!("{ms}ms") };
    n.note(format!("timeout = {text} (the node's own timeout in 0.10)"));
    n.set("timeout", Value::String(text));
}

fn browser(n: &mut NodeRewrite<'_>) {
    n.kind("browser.page.run");
    n.keep("credential_id");
    n.keep("code");
    node_timeout(n, 60_000, 1_000, 300_000);
}

// ── http ─────────────────────────────────────────────────────────────────────

fn http_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::replace(
        &[
            ("request", "response.request"),
            ("request.timeout_ms", "!0.11 does not echo the timeout"),
            ("response", "response"),
        ],
        None,
    )
}

fn http(n: &mut NodeRewrite<'_>) {
    n.kind("http.response.fetch");
    for key in ["credential_id", "url", "method", "headers", "body"] {
        n.keep(key);
    }
    n.rename("response_type", "parse");
    n.rename("body_type", "format");
    if let Some(bindings) = n.take("request_bindings") {
        match bindings {
            Value::Object(map) => {
                let wrapped: Map<String, Value> = map
                    .into_iter()
                    .map(|(k, v)| match v {
                        Value::String(expr) if !expr.contains("{{") => (k, Value::String(format!("{{{{ {} }}}}", expr.trim()))),
                        other => (k, other),
                    })
                    .collect();
                n.note(format!("request_bindings → argument {}", Value::Object(wrapped.clone())));
                n.set("argument", Value::Object(wrapped));
            }
            other => n.unresolved(format!("request_bindings {other} is not a map")),
        }
    }
    node_timeout(n, 10_000, 100, 120_000);
}

// ── mail ─────────────────────────────────────────────────────────────────────

fn mail_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::replace(
        &[
            ("sent", "message.sent"),
            ("attached", "message.attached"),
            ("subject", "message.subject"),
            ("to", "!0.11 answers message.recipient, a list"),
        ],
        None,
    )
}

fn mail(n: &mut NodeRewrite<'_>) {
    n.kind("mail.message.send");
    for key in ["credential_id", "subject", "text", "html", "reply_to"] {
        n.keep(key);
    }
    n.rename("to", "recipient");
    n.rename("from", "sender");
    if let Some(attach) = n.take("attach") {
        match attach {
            Value::Object(map) => {
                let mut files = Vec::new();
                for (name, source) in map {
                    let base = source.as_str().map(|s| s.rsplit('/').next().unwrap_or(s).to_string());
                    if base.as_deref() == Some(name.as_str()) {
                        files.push(source);
                    } else {
                        n.unresolved(format!(
                            "attachment `{name}` was sent under a name of its own; 0.11 sends a file under its stored name"
                        ));
                    }
                }
                if !files.is_empty() {
                    n.set("file", Value::Array(files));
                    n.note("attach → file");
                }
            }
            other => n.unresolved(format!("attach {other} is not a map of name to file")),
        }
    }
    n.rename("embed", "inline");
}

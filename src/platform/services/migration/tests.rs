//! The rewriter against 0.10 shapes, one mapping family at a time. Every
//! rewritten pipeline that should be clean is put through the save-time
//! check the platform runs.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{Value, json};

use super::graph::OldOutput;
use super::page::{PageContext, Renderer, rewrite_page};
use super::pipeline::{old_graph, rewrite_pipeline};
use super::{Rewrite, RewriteContext};

/// A 0.10 pipeline document: `(id, kind, config)` nodes and
/// `(from, pin, to)` edges.
fn doc(nodes: &[(&str, &str, Value)], edges: &[(&str, &str, &str)]) -> Value {
    json!({
        "apiVersion": "zebflow.com/v1",
        "kind": "Pipeline",
        "metadata": { "name": "fixture" },
        "spec": {
            "id": "fixture",
            "nodes": nodes.iter().map(|(id, kind, config)| {
                let mut pins: Vec<&str> = edges.iter().filter(|(from, _, _)| from == id).map(|(_, pin, _)| *pin).collect();
                pins.dedup();
                if pins.is_empty() {
                    pins.push("out");
                }
                json!({
                    "id": id,
                    "kind": kind,
                    "config": config,
                    "input_pins": if kind.starts_with("n.trigger.") && *kind != "n.trigger.weberror" && *kind != "n.trigger.ws" { json!([]) } else { json!(["in"]) },
                    "output_pins": pins,
                })
            }).collect::<Vec<_>>(),
            "edges": edges.iter().map(|(from, pin, to)| json!({
                "from_node": from, "from_pin": pin, "to_node": to, "to_pin": "in",
            })).collect::<Vec<_>>(),
        }
    })
}

fn chain(nodes: &[(&str, &str, Value)]) -> Value {
    let edges: Vec<(&str, &str, &str)> = nodes.windows(2).map(|w| (w[0].0, "out", w[1].0)).collect();
    doc(nodes, &edges)
}

fn rewrite(doc: &Value) -> Rewrite {
    rewrite_pipeline(doc, &RewriteContext::default())
}

fn config<'a>(rewrite: &'a Rewrite, id: &str) -> &'a Value {
    let nodes = rewrite.new_json.as_ref().expect("rewritten").pointer("/spec/nodes").expect("nodes");
    let node = nodes.as_array().unwrap().iter().find(|n| n["id"] == id).expect("node");
    &node["config"]
}

fn kind<'a>(rewrite: &'a Rewrite, id: &str) -> &'a str {
    let nodes = rewrite.new_json.as_ref().expect("rewritten").pointer("/spec/nodes").expect("nodes");
    nodes.as_array().unwrap().iter().find(|n| n["id"] == id).expect("node")["kind"].as_str().unwrap()
}

/// The rewrite has nothing unresolved and the save-time check refuses
/// nothing and warns about nothing.
fn assert_clean(rewrite: &Rewrite) {
    assert!(rewrite.unresolved.is_empty(), "unresolved: {:#?}", rewrite.unresolved);
    let doc = rewrite.new_json.as_ref().expect("rewritten");
    let graph = crate::contracts::kinds::decode_pipeline_graph(&serde_json::to_vec(doc).unwrap()).expect("decodes").spec;
    let mut defs = crate::pipeline::nodes::builtin_node_definitions();
    defs.extend(crate::platform::services::node_registry::NodeRegistryService::embedded_official_definitions());
    let check = crate::pipeline::nodes::check::check_pipeline(
        &graph,
        &defs,
        crate::pipeline::nodes::check::Catalogue::Complete,
        &crate::pipeline::nodes::check::no_credentials,
    );
    assert!(check.refusals.is_empty(), "refused: {:#?}\n{}", check.refusals, serde_json::to_string_pretty(doc).unwrap());
    assert!(check.warnings.is_empty(), "warnings: {:#?}\n{}", check.warnings, serde_json::to_string_pretty(doc).unwrap());
}

fn unresolved_mentions(rewrite: &Rewrite, words: &str) -> bool {
    rewrite.unresolved.iter().any(|u| u.text.contains(words))
}

// ── triggers and the payload paths after them ──────────────────────────────

#[test]
fn a_webhook_envelope_moves_under_webhook() {
    let old = chain(&[
        ("hook", "n.trigger.webhook", json!({ "path": "/hello", "method": "POST", "auth_type": "jwt", "auth_credential": "jwt-main", "auth_required_role": "admin,editor" })),
        ("greet", "n.script", json!({ "source": "return { greeting: 'Hi ' + input.body.name, id: input.params.id };" })),
        ("answer", "n.web.response", json!({ "body": "{{ { message: input.greeting } }}" })),
    ]);
    let out = rewrite(&old);
    assert_eq!(kind(&out, "hook"), "trigger.webhook");
    assert_eq!(config(&out, "hook")["route"], "/hello");
    assert_eq!(config(&out, "hook")["auth"], "jwt");
    assert_eq!(config(&out, "hook")["credential_id"], "jwt-main");
    assert_eq!(config(&out, "hook")["role"], json!(["admin", "editor"]));
    assert_eq!(kind(&out, "greet"), "javascript.script.run");
    assert_eq!(
        config(&out, "greet")["source"],
        "return { greeting: 'Hi ' + input.webhook.body.name, id: input.webhook.params.id };"
    );
    assert_eq!(config(&out, "answer")["body"], "{{ { message: input.script.greeting } }}");
    assert_clean(&out);
}

#[test]
fn every_trigger_answers_under_its_source() {
    let schedule = rewrite(&chain(&[
        ("tick", "n.trigger.schedule", json!({ "cron": "0 * * * *" })),
        ("log", "n.script", json!({ "source": "return { at: input.fired_at, by: input.trigger };" })),
    ]));
    assert_eq!(config(&schedule, "log")["source"], "return { at: input.schedule.fired_at, by: ('schedule') };");
    assert_clean(&schedule);

    let function = rewrite(&chain(&[
        ("fn", "n.trigger.function", json!({ "description": "Find a user.", "params": { "email": { "type": "string", "required": true } } })),
        ("find", "n.script", json!({ "source": "return { email: input.email };" })),
    ]));
    assert_eq!(config(&function, "fn")["schema"]["required"], json!(["email"]));
    assert_eq!(config(&function, "find")["source"], "return { email: input.function.email };");
    assert_clean(&function);

    let error = rewrite(&chain(&[
        ("err", "n.trigger.weberror", json!({ "code": "404" })),
        ("page", "n.web.response", json!({ "status": 404, "body": "{{ { path: input.original_path } }}" })),
    ]));
    assert_eq!(kind(&error, "err"), "trigger.error");
    assert_eq!(config(&error, "err")["status"], 404);
    assert_eq!(config(&error, "page")["body"], "{{ { path: input.error.original_path } }}");
    assert_clean(&error);

    let socket = rewrite(&chain(&[
        ("sock", "n.trigger.ws.client", json!({ "url": "wss://feed.example.com/live", "reconnect": false, "reconnect_delay_ms": 2500, "message_format": "json" })),
        ("keep", "n.script", json!({ "source": "return { m: input.message };" })),
    ]));
    assert_eq!(config(&socket, "sock")["max_attempts"], 0);
    assert_eq!(config(&socket, "sock")["delay"], "2500ms");
    assert_eq!(config(&socket, "sock")["parse"], "json");
    assert_eq!(config(&socket, "keep")["source"], "return { m: input.socket.message };");
    assert_clean(&socket);
}

#[test]
fn a_run_input_answers_at_its_name() {
    let out = rewrite(&chain(&[
        ("start", "n.trigger.manual", json!({})),
        ("title", "n.input.text", json!({ "name": "title", "default": "Untitled" })),
        ("use", "n.script", json!({ "source": "return { a: input.body.title, b: ctx.nodes.title };" })),
    ]));
    assert_eq!(kind(&out, "title"), "input.text");
    assert_eq!(config(&out, "use")["source"], "return { a: input.title, b: ctx.nodes.title.title };");
    assert_clean(&out);
}

// ── data ─────────────────────────────────────────────────────────────────────

#[test]
fn a_query_binds_by_position_writes_only_when_told_and_answers_under_query() {
    let out = rewrite(&chain(&[
        ("hook", "n.trigger.webhook", json!({ "path": "/posts", "method": "POST" })),
        ("ins", "n.pg.query", json!({ "credential_id": "pg-main", "params": "{{ [input.body.title, input.params.id] }}", "query": "INSERT INTO posts (title, owner) VALUES ($1, $2) RETURNING id" })),
        ("one", "n.pg.query", json!({ "credential_id": "pg-main", "params": "{{ input.rows[0].id }}", "query": "SELECT * FROM posts WHERE id = $1" })),
        ("answer", "n.web.response", json!({ "body": "{{ { post: input.rows[0], first: $nodes.ins.rows[0].id } }}" })),
    ]));
    let ins = config(&out, "ins");
    assert_eq!(kind(&out, "ins"), "postgres.query.run");
    assert_eq!(ins["write"], true);
    assert_eq!(ins["param"], json!({ "1": "{{ input.webhook.body.title }}", "2": "{{ input.webhook.params.id }}" }));
    assert_eq!(ins["limit"], 5000);
    let one = config(&out, "one");
    assert!(one.get("write").is_none(), "a SELECT runs read-only");
    assert_eq!(one["param"], json!({ "1": "{{ input.query.rows[0].id }}" }));
    // Two upstream nodes answer `query`: the reference names the one meant.
    assert_eq!(config(&out, "answer")["body"], "{{ { post: $nodes.one.query.rows[0], first: $nodes.ins.query.rows[0].id } }}");
    assert!(out.notes.iter().any(|n| n.text.contains("at most 5000")));
    assert_clean(&out);
}

#[test]
fn sqlite_mutate_is_query_run_with_write_and_sekejap_keeps_its_write_default() {
    let out = rewrite(&chain(&[
        ("hook", "n.trigger.webhook", json!({ "path": "/note", "method": "POST" })),
        ("save", "n.sqlite.mutate", json!({ "sql": "INSERT INTO notes (body) VALUES (?1)", "params": ["{{ input.body.text }}"] })),
        ("graph", "n.sekejap.query", json!({ "query": "INSERT INTO tags (name) VALUES ($1)", "params": "{{ input.affected_rows }}" })),
        ("answer", "n.web.response", json!({ "body": "{{ { changed: input.affected_rows } }}" })),
    ]));
    assert_eq!(kind(&out, "save"), "sqlite.query.run");
    assert_eq!(config(&out, "save")["write"], true);
    assert_eq!(config(&out, "save")["query"], "INSERT INTO notes (body) VALUES (?1)");
    assert_eq!(config(&out, "save")["param"], json!({ "1": "{{ input.webhook.body.text }}" }));
    assert_eq!(config(&out, "graph")["write"], true);
    assert_eq!(config(&out, "graph")["param"], json!({ "1": "{{ input.query.rows_affected }}" }));
    assert_eq!(config(&out, "answer")["body"], "{{ { changed: $nodes.graph.query.rows_affected } }}");
    assert_clean(&out);
}

#[test]
fn a_list_built_at_run_time_cannot_be_split_into_positions() {
    let out = rewrite(&chain(&[
        ("hook", "n.trigger.webhook", json!({ "path": "/q", "method": "GET" })),
        ("q", "n.pg.query", json!({ "credential_id": "pg-main", "params": "{{ input.query.ids.map(Number) }}", "query": "SELECT 1" })),
    ]));
    assert!(unresolved_mentions(&out, "cannot be told before it runs"), "{:#?}", out.unresolved);
}

// ── files ────────────────────────────────────────────────────────────────────

#[test]
fn an_upload_is_saved_from_the_webhook_and_its_thumbnail_from_the_saved_file() {
    let out = rewrite(&chain(&[
        ("hook", "n.trigger.webhook", json!({ "path": "/upload", "method": "POST" })),
        ("save", "n.fs.save", json!({ "field": "photo", "folder": "uploads", "allowed_kinds": ["images"], "max_size_mb": 5 })),
        ("thumb", "n.fs.image.thumbnail", json!({ "width": 200, "height": 200, "fit": "cover", "format": "jpg", "quality": 150 })),
        ("answer", "n.web.response", json!({ "body": "{{ { original: input.saved.ref, thumbnail: input.thumbnail.ref } }}" })),
    ]));
    assert_eq!(kind(&out, "save"), "fs.file.put");
    let save = config(&out, "save");
    assert_eq!(save["from"], "{{ input.webhook.files.photo }}");
    assert_eq!(save["accept"], json!(["image"]));
    assert_eq!(save["max_size"], "5MiB");
    let thumb = config(&out, "thumb");
    assert_eq!(thumb["from"], "{{ input.file }}");
    assert_eq!(thumb["quality"], 100, "0.10 clamped the quality");
    assert_eq!(config(&out, "answer")["body"], "{{ { original: input.file.ref, thumbnail: input.image.ref } }}");
    assert_clean(&out);
}

#[test]
fn object_operations_take_their_subject_explicitly() {
    let out = rewrite(&chain(&[
        ("start", "n.trigger.manual", json!({})),
        ("put", "n.fs.put", json!({ "text": "hello", "filename": "hello.txt" })),
        ("list", "n.fs.list", json!({})),
        ("drop", "n.fs.delete", json!({ "path": "tmp/old" })),
        ("end", "n.script", json!({ "source": "return { n: input.fs.count };" })),
    ]));
    assert_eq!(config(&out, "put")["folder"], "files", "n.fs.put wrote under files/");
    assert_eq!(config(&out, "list")["from"], "/");
    assert_eq!(config(&out, "drop")["from"], "tmp/old");
    assert_eq!(config(&out, "drop")["recursive"], true);
    // `fs` came from the delete (the nearest), whose answer has no count.
    assert!(unresolved_mentions(&out, "no 0.11 equivalent") || !out.unresolved.is_empty(), "{:#?}", out.unresolved);
}

// ── the response ─────────────────────────────────────────────────────────────

#[test]
fn a_response_writes_out_its_cookie_redirect_and_body() {
    let out = rewrite(&chain(&[
        ("hook", "n.trigger.webhook", json!({ "path": "/login", "method": "POST" })),
        ("token", "n.auth.token.create", json!({ "credential_id": "jwt-main", "expires_in": 3600, "claims": { "sub": "{{ input.body.user }}" } })),
        ("go", "n.web.response", json!({ "location": "/home", "set_cookie": "name=session,value={{ input.access_token }},max-age=3600" })),
    ]));
    let go = config(&out, "go");
    assert_eq!(go["status"], 302);
    assert_eq!(go["headers"]["Location"], "/home");
    assert_eq!(
        go["headers"]["Set-Cookie"],
        "session={{ input.token.access_token }}; Path=/; Max-Age=3600; SameSite=Lax; HttpOnly"
    );
    assert_eq!(config(&out, "token")["ttl"], "3600s");
    assert_eq!(config(&out, "token")["claims"]["sub"], "{{ input.webhook.body.user }}");
    assert_clean(&out);

    let text = rewrite(&chain(&[
        ("hook", "n.trigger.webhook", json!({ "path": "/t", "method": "GET" })),
        ("say", "n.web.response", json!({ "message": "plain words", "headers": { "X-Kind": "note" } })),
    ]));
    assert_eq!(config(&text, "say")["body"], "plain words");
    assert_clean(&text);

    let whole = rewrite(&chain(&[
        ("hook", "n.trigger.webhook", json!({ "path": "/w", "method": "GET" })),
        ("shape", "n.script", json!({ "source": "return { ok: true, n: 1 };" })),
        ("all", "n.web.response", json!({})),
    ]));
    assert_eq!(config(&whole, "all")["body"], "{{ input.script }}", "0.10 answered the whole payload");
    assert_clean(&whole);

    let quoted = rewrite(&chain(&[
        ("hook", "n.trigger.webhook", json!({ "path": "/s", "method": "GET" })),
        ("say", "n.web.response", json!({ "body": "just text" })),
    ]));
    assert_eq!(config(&quoted, "say")["body"], "\"just text\"", "0.10 answered a string body as JSON");
    assert_eq!(config(&quoted, "say")["headers"]["Content-Type"], "application/json");
}

#[test]
fn a_webhook_without_a_response_answers_204_and_is_left_for_review() {
    let out = rewrite(&chain(&[
        ("hook", "n.trigger.webhook", json!({ "path": "/api/ping", "method": "GET" })),
        ("pong", "n.script", json!({ "source": "return { ok: true, at: input.query.at };" })),
    ]));
    // Nothing reproduces the 0.10 payload dump: no response node is added.
    let nodes = out.new_json.as_ref().unwrap().pointer("/spec/nodes").unwrap().as_array().unwrap().clone();
    assert!(nodes.iter().all(|n| n["kind"] != "web.response.send"), "{nodes:#?}");
    assert!(out.unresolved.is_empty(), "{:#?}", out.unresolved);
    assert_eq!(out.review.len(), 1, "{:#?}", out.review);
    assert_eq!(out.review[0].node, "pong");
    assert_eq!(
        out.review[0].text,
        "0.10 answered the run's value here (`pong`'s answer, which replaced the payload); 0.11 answers 204 — add web.response.send with what the caller needs (the payload's keys: `script`, `webhook`)"
    );

    // A last node that added its key and kept the rest: the value was the
    // payload as the path carried it, not one node's answer.
    let kept = rewrite(&chain(&[
        ("hook", "n.trigger.webhook", json!({ "path": "/api/convert", "method": "POST" })),
        ("conv", "n.table.convert", json!({ "from": "uploads/demo.csv", "to_format": "json" })),
    ]));
    assert_eq!(kept.review.len(), 1, "{:#?}", kept.review);
    assert!(
        kept.review[0].text.starts_with("0.10 answered the run's value here (the payload this path carried); 0.11 answers 204"),
        "{:#?}",
        kept.review
    );

    // Each branch that ends without one is its own item; one that answers is not.
    let branches = rewrite(&doc(
        &[
            ("hook", "n.trigger.webhook", json!({ "path": "/api/flag", "method": "POST" })),
            ("check", "n.logic.if", json!({ "expr": "input.body.ok" })),
            ("yes", "n.web.response", json!({ "body": "{{ { ok: true } }}" })),
            ("no", "n.script", json!({ "source": "return { seen: true };" })),
        ],
        &[("hook", "out", "check"), ("check", "true", "yes"), ("check", "false", "no")],
    ));
    assert_eq!(branches.review.iter().map(|r| r.node.as_str()).collect::<Vec<_>>(), ["no"], "{:#?}", branches.review);

    // A schedule answers nobody: nothing to review.
    let tick = rewrite(&chain(&[
        ("tick", "n.trigger.schedule", json!({ "cron": "0 * * * *" })),
        ("log", "n.script", json!({ "source": "return { at: input.fired_at };" })),
    ]));
    assert!(tick.review.is_empty(), "{:#?}", tick.review);
}

#[test]
fn a_script_returning_0_10_response_instructions_is_unresolved() {
    let out = rewrite(&chain(&[
        ("hook", "n.trigger.webhook", json!({ "path": "/login", "method": "POST" })),
        ("go", "n.script", json!({ "source": "return { _status: 303, _set_cookie: { name: 's', value: 'v' } };" })),
    ]));
    assert!(unresolved_mentions(&out, "`_status`"), "{:#?}", out.unresolved);
    assert!(out.review.is_empty(), "{:#?}", out.review);
}

// ── scripts ──────────────────────────────────────────────────────────────────

#[test]
fn a_script_that_reads_its_payload_whole_gets_the_0_10_payload_back() {
    let out = rewrite(&chain(&[
        ("hook", "n.trigger.webhook", json!({ "path": "/echo", "method": "POST" })),
        ("echo", "n.script", json!({ "source": "return { ...input, seen: true };" })),
        ("answer", "n.web.response", json!({ "body": "{{ { body: input.body, seen: input.seen } }}" })),
    ]));
    let source = config(&out, "echo")["source"].as_str().unwrap().to_string();
    assert!(
        source.starts_with("input = ({ body: input.webhook.body, query: input.webhook.query, params: input.webhook.params, path: input.webhook.path, method: input.webhook.method, files: input.webhook.files, auth: input.webhook.auth });\n"),
        "{source}"
    );
    assert!(source.ends_with("return { ...input, seen: true };"));
    assert_eq!(config(&out, "answer")["body"], "{{ { body: input.script.body, seen: input.script.seen } }}");
    assert_clean(&out);
}

#[test]
fn a_script_that_computes_its_code_cannot_move() {
    let out = rewrite(&chain(&[
        ("start", "n.trigger.manual", json!({})),
        ("dyn", "n.script", json!({ "source_expr": "input.code" })),
    ]));
    assert!(unresolved_mentions(&out, "source_expr"), "{:#?}", out.unresolved);
}

// ── function calls ───────────────────────────────────────────────────────────

#[test]
fn a_call_sends_the_payload_0_10_sent_and_reads_the_result_where_0_11_puts_it() {
    let callee = chain(&[
        ("fn", "n.trigger.function", json!({ "description": "Greets." })),
        ("greet", "n.script", json!({ "source": "return { user: { name: input.name } };" })),
    ]);
    let mut context = RewriteContext::default();
    let graph = old_graph(&callee, &context).unwrap();
    let last = graph.outputs[1].clone();
    context.functions.insert("greet".into(), super::kinds::function_result(&last, Some("script")));
    let caller = chain(&[
        ("hook", "n.trigger.webhook", json!({ "path": "/greet", "method": "POST" })),
        ("prep", "n.script", json!({ "source": "return { name: input.body.name };" })),
        ("call", "n.function.call", json!({ "function": "greet" })),
        ("answer", "n.web.response", json!({ "body": "{{ { hello: input.user.name } }}" })),
    ]);
    let out = rewrite_pipeline(&caller, &context);
    assert_eq!(kind(&out, "call"), "function.result.call");
    assert_eq!(config(&out, "call")["argument"], "{{ input.script }}");
    assert_eq!(config(&out, "answer")["body"], "{{ { hello: input.result.user.name } }}");
    assert_clean(&out);
    // An unknown function leaves what it returned unknown.
    let blind = rewrite(&caller);
    assert!(unresolved_mentions(&blind, "not a 0.10 function pipeline"), "{:#?}", blind.unresolved);
}

/// A handler on an ordinary node's `error` pin read the 0.10 envelope
/// (`input.error`, `input.input`); 0.11 delivers the payload kept plus the
/// failing node's key, so `error` moves under that key and `input.<k>` is
/// read where the failing node received it.
#[test]
fn a_handler_reading_the_old_failure_envelope_reads_the_failing_nodes_key() {
    let out = rewrite(&doc(
        &[
            ("hook", "n.trigger.webhook", json!({ "path": "/save", "method": "POST" })),
            ("s", "n.script", json!({ "source": "return { saved: true };" })),
            ("ok", "n.web.response", json!({ "body": "{{ { saved: input.saved } }}" })),
            ("oops", "n.web.response", json!({
                "status": 500,
                "body": "{{ { code: input.error.code, why: input.error.message, name: input.input.body.name, attempt: input.__zf_retry.attempt } }}"
            })),
        ],
        &[("hook", "out", "s"), ("s", "out", "ok"), ("s", "error", "oops")],
    ));
    assert_eq!(
        config(&out, "oops")["body"],
        "{{ { code: input.script.error.code, why: input.script.error.message, name: input.webhook.body.name, attempt: input.__zf_retry.attempt } }}"
    );
    assert!(out.unresolved.is_empty(), "{:#?}", out.unresolved);

    // Which node failed is the run record's, not the answer's.
    let gone = rewrite(&doc(
        &[
            ("hook", "n.trigger.webhook", json!({ "path": "/save", "method": "POST" })),
            ("s", "n.script", json!({ "source": "return { saved: true };" })),
            ("oops", "n.web.response", json!({ "status": 500, "body": "{{ { at: input.error.node_id } }}" })),
        ],
        &[("hook", "out", "s"), ("s", "error", "oops")],
    ));
    assert!(gone.unresolved.iter().any(|u| u.text.contains("the run record names the failing node")), "{:#?}", gone.unresolved);
}

#[test]
fn an_unwired_failed_call_in_a_webhook_answers_its_failure_and_nothing_else() {
    let callee = chain(&[
        ("fn", "n.trigger.function", json!({ "description": "Greets." })),
        ("greet", "n.script", json!({ "source": "return { user: { name: input.name } };" })),
    ]);
    let mut context = RewriteContext::default();
    let graph = old_graph(&callee, &context).unwrap();
    context.functions.insert("greet".into(), super::kinds::function_result(&graph.outputs[1], Some("script")));
    let caller = chain(&[
        ("hook", "n.trigger.webhook", json!({ "path": "/greet", "method": "POST" })),
        ("call", "n.function.call", json!({ "function": "greet", "input": "{{ { name: input.body.name } }}" })),
        ("answer", "n.web.response", json!({ "body": "{{ { hello: input.user.name } }}" })),
    ]);
    let out = rewrite_pipeline(&caller, &context);
    assert_eq!(kind(&out, "failed"), "web.response.send");
    assert_eq!(config(&out, "failed")["status"], 500);
    assert_eq!(
        config(&out, "failed")["body"],
        "{{ { ok: false, error: { code: input.result.error.code } } }}",
        "the call's error code, under its noun, and nothing of the payload — no raw message"
    );
    let edges = out.new_json.as_ref().unwrap().pointer("/spec/edges").unwrap().as_array().unwrap().clone();
    assert!(
        edges.iter().any(|e| e["from_node"] == "call" && e["from_pin"] == "error" && e["to_node"] == "failed"),
        "{edges:#?}"
    );
    assert!(out.changes.iter().any(|c| c.node == "failed" && c.text.contains("on `call`'s :error")), "{:#?}", out.changes);
    assert!(out.review.is_empty(), "the call path answers: {:#?}", out.review);
    assert_clean(&out);

    // A wired error pin is the author's handler, kept; it reads the failure
    // where 0.11 delivers it.
    let wired = rewrite_pipeline(
        &doc(
            &[
                ("hook", "n.trigger.webhook", json!({ "path": "/greet", "method": "POST" })),
                ("call", "n.function.call", json!({ "function": "greet", "input": "{{ { name: input.body.name } }}" })),
                ("answer", "n.web.response", json!({ "body": "{{ { hello: input.user.name } }}" })),
                ("oops", "n.web.response", json!({ "status": 502, "body": "{{ { failed: input.error } }}" })),
            ],
            &[("hook", "out", "call"), ("call", "out", "answer"), ("call", "error", "oops")],
        ),
        &context,
    );
    let nodes = wired.new_json.as_ref().unwrap().pointer("/spec/nodes").unwrap().as_array().unwrap().clone();
    assert!(nodes.iter().all(|n| n["id"] != "failed"), "{nodes:#?}");
    assert_eq!(config(&wired, "oops")["body"], "{{ { failed: (`${input.result.error.code}: ${input.result.error.message}`) } }}");
    assert_clean(&wired);

    // Outside a webhook nobody waits on an answer: the run fails there, noted.
    let job = rewrite_pipeline(
        &chain(&[
            ("tick", "n.trigger.schedule", json!({ "cron": "0 * * * *" })),
            ("call", "n.function.call", json!({ "function": "greet", "input": "{{ { name: 'x' } }}" })),
        ]),
        &context,
    );
    assert!(job.notes.iter().any(|n| n.node == "call" && n.text.contains("0.11 fails the run")), "{:#?}", job.notes);
}

#[test]
fn an_error_page_keeps_its_status_class_as_written() {
    for class in ["4xx", "5xx"] {
        let out = rewrite(&chain(&[
            ("err", "n.trigger.weberror", json!({ "code": class })),
            ("page", "n.web.response", json!({ "status": 500, "body": "{{ { path: input.original_path } }}" })),
        ]));
        assert_eq!(config(&out, "err")["status"], class);
        assert_clean(&out);
    }
}

// ── logic ────────────────────────────────────────────────────────────────────

#[test]
fn control_nodes_keep_their_expressions_on_the_new_paths() {
    let out = doc(
        &[
            ("hook", "n.trigger.webhook", json!({ "path": "/check", "method": "POST" })),
            ("gate", "n.logic.if", json!({ "expression": "input.body.age >= 18" })),
            ("yes", "n.web.response", json!({ "body": "{{ { ok: true } }}" })),
            ("no", "n.web.response", json!({ "status": 403, "message": "too young" })),
        ],
        &[("hook", "out", "gate"), ("gate", "true", "yes"), ("gate", "false", "no")],
    );
    let out = rewrite(&out);
    assert_eq!(config(&out, "gate")["when"], "input.webhook.body.age >= 18");
    assert_clean(&out);

    let loop_doc = doc(
        &[
            ("start", "n.trigger.manual", json!({})),
            ("each", "n.logic.foreach", json!({ "items_expr": "input.items", "dispatch": "seq", "chunk_size": 2 })),
            ("sum", "n.logic.reduce", json!({ "init_expr": "0", "step_expr": "$acc + $input.item.length" })),
            ("end", "n.script", json!({ "source": "return { total: input };" })),
        ],
        &[("start", "out", "each"), ("each", "item", "sum"), ("sum", "out", "end")],
    );
    let out = rewrite(&loop_doc);
    assert_eq!(config(&out, "each")["from"], "input.manual.items");
    assert_eq!(config(&out, "each")["batch_size"], 2);
    assert!(config(&out, "each").get("dispatch").is_none());
    assert_eq!(config(&out, "sum")["step"], "$acc + $input.item.length");
    assert_eq!(config(&out, "end")["source"], "input = input.reduce;\nreturn { total: input };");
    assert!(out.notes.iter().any(|n| n.text.contains("empty list")));
    assert_clean(&out);
}

#[test]
fn two_branches_that_both_run_into_one_node_are_reported() {
    let joined = doc(
        &[
            ("start", "n.trigger.manual", json!({})),
            ("a", "n.script", json!({ "source": "return { a: 1 };" })),
            ("b", "n.script", json!({ "source": "return { b: 2 };" })),
            ("join", "n.script", json!({ "source": "return {};" })),
        ],
        &[("start", "out", "a"), ("start", "out", "b"), ("a", "out", "join"), ("b", "out", "join")],
    );
    assert!(unresolved_mentions(&rewrite(&joined), "both run"));
    // Branches of one logic.if meet without being a join.
    let exclusive = doc(
        &[
            ("start", "n.trigger.manual", json!({})),
            ("gate", "n.logic.if", json!({ "expression": "input.x" })),
            ("a", "n.script", json!({ "source": "return { v: 1 };" })),
            ("b", "n.script", json!({ "source": "return { v: 2 };" })),
            ("end", "n.script", json!({ "source": "return { v: input.v };" })),
        ],
        &[("start", "out", "gate"), ("gate", "true", "a"), ("gate", "false", "b"), ("a", "out", "end"), ("b", "out", "end")],
    );
    let out = rewrite(&exclusive);
    assert!(!unresolved_mentions(&out, "both run"), "{:#?}", out.unresolved);
}

// ── key-value, crypto, websockets, mail, http ────────────────────────────────

#[test]
fn smaller_families_map_their_flags_and_answers() {
    let out = rewrite(&chain(&[
        ("hook", "n.trigger.webhook", json!({ "path": "/kv", "method": "POST" })),
        ("hash", "n.crypto", json!({ "op": "bcrypt_hash", "input": "{{ input.body.password }}", "cost": 10 })),
        ("put", "n.kv.set", json!({ "key": "pw", "ttl": 600 })),
        ("get", "n.kv.get", json!({ "key": "pw", "out_key": "stored" })),
        ("call", "n.http.request", json!({ "url": "https://api.example.com/hook", "method": "POST", "body": "{{ { hash: input.result } }}", "timeout_ms": 5000 })),
        ("answer", "n.web.response", json!({ "body": "{{ { stored: $nodes.get.stored, status: input.response.status } }}" })),
    ]));
    assert_eq!(kind(&out, "hash"), "crypto.password.hash");
    assert_eq!(config(&out, "hash")["from"], "{{ input.webhook.body.password }}");
    assert_eq!(config(&out, "hash")["algorithm"], "bcrypt");
    assert_eq!(config(&out, "put")["ttl"], "600s");
    assert!(config(&out, "put")["value"].as_str().unwrap().starts_with("{{ ({ ...({ body: input.webhook.body"), "{}", config(&out, "put")["value"]);
    assert_eq!(config(&out, "call")["body"], "{{ { hash: input.password.hash } }}");
    assert_eq!(config(&out, "call")["timeout"], "5s");
    assert_eq!(config(&out, "answer")["body"], "{{ { stored: $nodes.get.entry.value, status: input.response.status } }}");
    assert_clean(&out);

    let hmac = rewrite(&chain(&[
        ("start", "n.trigger.manual", json!({})),
        ("sign", "n.crypto", json!({ "op": "hmac_sha256", "input": "x", "key": "k" })),
    ]));
    assert!(unresolved_mentions(&hmac, "hmac credential"));
}

#[test]
fn a_room_sends_and_keeps_state_with_keys_written_as_expressions() {
    let out = rewrite(&chain(&[
        ("room", "n.trigger.ws", json!({ "room": "lobby" })),
        ("state", "n.ws.sync_state", json!({ "op": "merge", "path": "/players/{session_id}", "value": "{{ { at: input.payload.x } }}", "silent": true })),
        ("say", "n.ws.emit", json!({ "event": "moved", "to": "others" })),
    ]));
    assert_eq!(kind(&out, "state"), "ws.state.update");
    assert_eq!(config(&out, "state")["key"], "/players/{{ input.room.session_id }}");
    assert_eq!(config(&out, "state")["value"], "{{ { at: input.room.payload.x } }}");
    assert_eq!(config(&out, "state")["batch"], true);
    assert_eq!(kind(&out, "say"), "ws.message.send");
    assert_eq!(config(&out, "say")["recipient"], "others");
    assert_eq!(config(&out, "say")["body"], "{{ input.room.payload }}");
    assert_clean(&out);
}

// ── composites and providers ─────────────────────────────────────────────────

#[test]
fn an_agent_takes_its_provider_from_its_credential() {
    let old = chain(&[
        ("hook", "n.trigger.webhook", json!({ "path": "/ask", "method": "POST" })),
        ("ask", "n.ai.agent", json!({ "credential_id": "llm", "prompt": "{{ input.body.q }}", "output_mode": "final_only" })),
        ("answer", "n.web.response", json!({ "body": "{{ { a: input.response } }}" })),
    ]);
    let mut context = RewriteContext::default();
    context.credential_kinds.insert("llm".into(), "openrouter".into());
    let out = rewrite_pipeline(&old, &context);
    assert_eq!(kind(&out, "ask"), "ai.text.generate");
    assert_eq!(config(&out, "ask")["provider"], "openrouter");
    assert_eq!(config(&out, "ask")["answer_only"], true);
    assert_eq!(config(&out, "answer")["body"], "{{ { a: input.text.value } }}");
    assert_clean(&out);
    assert!(unresolved_mentions(&rewrite(&old), "not in this project"));
}

#[test]
fn a_telegram_send_takes_what_it_read_from_the_payload_as_flags() {
    let out = rewrite(&chain(&[
        ("bot", "n.telegram.trigger", json!({ "bot_credential_id": "tg", "allowed_updates": "message" })),
        ("reply", "n.script", json!({ "source": "return { chat_id: input.chat_id, text: 'You said: ' + input.text };" })),
        ("send", "n.telegram.send", json!({ "bot_credential_id": "tg" })),
    ]));
    assert_eq!(kind(&out, "bot"), "trigger.telegram");
    assert_eq!(config(&out, "bot")["event"], json!(["message"]));
    assert_eq!(config(&out, "reply")["source"], "return { chat_id: input.telegram.chat_id, text: 'You said: ' + input.telegram.text };");
    assert_eq!(config(&out, "send")["recipient"], "{{ input.script.chat_id }}");
    assert_eq!(config(&out, "send")["text"], "{{ input.script.text }}");
    assert_clean(&out);
}

#[test]
fn what_has_no_0_11_equivalent_is_reported_never_guessed() {
    let out = rewrite(&chain(&[
        ("start", "n.trigger.manual", json!({})),
        ("pkg", "n.x.acme.thing", json!({})),
        ("speak", "n.ai.tts", json!({ "text": "hi" })),
    ]));
    assert!(unresolved_mentions(&out, "installed from a package"));
    assert!(unresolved_mentions(&out, "--return both"));
    // A key the 0.10.12 node never read is dropped, as 0.10 ignored it.
    let legacy = rewrite(&chain(&[
        ("start", "n.trigger.manual", json!({})),
        ("call", "n.function.call", json!({ "function": "f", "input": "{{ { a: 1 } }}", "input_value": "{{ input }}" })),
    ]));
    assert!(legacy.changes.iter().any(|c| c.text.contains("input_value") && c.text.contains("did not read")));
}

#[test]
fn a_0_11_document_is_left_alone() {
    let current = json!({ "apiVersion": "zebflow.com/v1", "kind": "Pipeline", "metadata": { "name": "x" },
        "spec": { "id": "x", "nodes": [{ "id": "a", "kind": "trigger.manual", "config": {} }], "edges": [] } });
    let out = rewrite(&current);
    assert!(out.new_json.is_none());
    assert!(out.changes.is_empty() && out.unresolved.is_empty());
}

// ── pages ────────────────────────────────────────────────────────────────────

#[test]
fn a_page_reads_where_its_pipeline_now_puts_each_key() {
    let old = chain(&[
        ("hook", "n.trigger.webhook", json!({ "path": "/posts/:id", "method": "GET" })),
        ("q", "n.pg.query", json!({ "credential_id": "pg-main", "params": "{{ input.params.id }}", "query": "SELECT * FROM posts WHERE id = $1" })),
        ("page", "n.web.response", json!({ "template": "pages/post.tsx" })),
    ]);
    let graph = Arc::new(old_graph(&old, &RewriteContext::default()).unwrap());
    let context = PageContext { renderers: vec![Renderer { pipeline: "posts".into(), graph, node: 2 }] };
    let page = "export default function Page(input) {\n  const row = input?.rows?.[0];\n  return <main><h1>{row?.title}</h1><p>{input.params.id}</p><p>{ctx.query.page}</p></main>;\n}\n";
    let out = rewrite_page(page, &context);
    assert!(out.unresolved.is_empty(), "{:#?}", out.unresolved);
    let new = out.new_tsx.expect("changed");
    assert!(new.contains("const row = input?.query?.rows?.[0];"), "{new}");
    // The request's own keys still come in where nothing answers them…
    assert!(new.contains("{input.params.id}"), "{new}");
    // …but `query` is the query node's answer now: the request's is named.
    assert!(new.contains("{ctx.webhook.query.page}"), "{new}");
}

#[test]
fn pages_rendered_by_different_triggers_read_whichever_answer_their_payload_holds() {
    let hook = Arc::new(old_graph(&chain(&[
        ("hook", "n.trigger.webhook", json!({ "path": "/missing", "method": "GET" })),
        ("page", "n.web.response", json!({ "template": "pages/oops.tsx" })),
    ]), &RewriteContext::default()).unwrap());
    let errors = Arc::new(old_graph(&chain(&[
        ("err", "n.trigger.weberror", json!({ "code": "404" })),
        ("page", "n.web.response", json!({ "template": "pages/oops.tsx" })),
    ]), &RewriteContext::default()).unwrap());
    let context = PageContext {
        renderers: vec![
            Renderer { pipeline: "missing".into(), graph: hook, node: 1 },
            Renderer { pipeline: "errors".into(), graph: errors, node: 1 },
        ],
    };
    let out = rewrite_page("export default function Oops(input) { return <p>{input.path}</p>; }", &context);
    assert!(out.unresolved.is_empty(), "{:#?}", out.unresolved);
    assert_eq!(
        out.new_tsx.as_deref(),
        Some("export default function Oops(input) { return <p>{(input?.webhook?.path ?? input?.error?.path)}</p>; }")
    );
}

#[test]
fn a_component_a_page_imports_reads_the_page_payload_through_the_globals() {
    let old = chain(&[
        ("hook", "n.trigger.webhook", json!({ "path": "/list", "method": "GET" })),
        ("q", "n.pg.query", json!({ "credential_id": "pg-main", "query": "SELECT 1" })),
        ("page", "n.web.response", json!({ "template": "pages/list.tsx" })),
    ]);
    let graph = Arc::new(old_graph(&old, &RewriteContext::default()).unwrap());
    let context = PageContext { renderers: vec![Renderer { pipeline: "list".into(), graph, node: 2 }] };
    // Its own `input` prop is its own; the globals are the page's payload.
    let component = "export function Count({ input }) { return <b>{input.n}{ctx.rows.length}</b>; }";
    let out = super::page::rewrite_component(component, &context);
    assert!(out.unresolved.is_empty(), "{:#?}", out.unresolved);
    assert_eq!(out.new_tsx.as_deref(), Some("export function Count({ input }) { return <b>{input.n}{ctx.query.rows.length}</b>; }"));
    assert_eq!(
        super::expr::local_imports("import { A } from \"@/components/a\";\nimport B from \"../shared/b.tsx\";\nimport { h } from \"zeb/react\";", "pages/admin"),
        vec!["components/a".to_string(), "pages/shared/b.tsx".to_string()]
    );
}

// ── the model ────────────────────────────────────────────────────────────────

#[test]
fn every_rule_names_a_0_11_kind_the_catalogue_has() {
    let mut defs: Vec<String> = crate::pipeline::nodes::builtin_node_definitions().into_iter().map(|d| d.kind).collect();
    defs.extend(crate::platform::services::node_registry::NodeRegistryService::embedded_official_definitions().into_iter().map(|d| d.kind));
    for (old, _) in super::kinds::known::KNOWN {
        let rule = super::kinds::rule(old).unwrap_or_else(|| panic!("no rule for {old}"));
        let config = serde_json::Map::new();
        let new = rule.kind_for(old, &config);
        assert!(defs.contains(&new), "{old} → {new}, which 0.11 does not have");
    }
    let _: BTreeMap<String, OldOutput> = BTreeMap::new();
}

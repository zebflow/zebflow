//! Each save-time refusal with its message, a `{{ }}` value left to the
//! run, and the reference warnings `pipeline_check` adds.

use serde_json::{Value, json};

use super::*;
use crate::pipeline::model::PipelineNode;

fn defs() -> Vec<NodeDefinition> {
    crate::pipeline::nodes::builtin_node_definitions()
}

fn def(kind: &str) -> NodeDefinition {
    defs().into_iter().find(|d| d.kind == kind).expect("kind")
}

fn build(body: &str) -> PipelineGraph {
    crate::platform::shell::parser::build_pipeline_graph_with_definitions("check-test", body, &defs()).expect("builds")
}

fn check(body: &str) -> PipelineCheck {
    check_pipeline(&build(body), &defs(), Catalogue::Complete, &no_credentials)
}

fn refusals(body: &str) -> Vec<String> {
    check(body).refusals.iter().map(ToString::to_string).collect()
}

fn warnings(body: &str) -> Vec<String> {
    let found = check(body);
    assert!(found.refusals.is_empty(), "{:?}", found.refusals);
    found.warnings.iter().map(ToString::to_string).collect()
}

fn config_problems(kind: &str, config: Value) -> Vec<String> {
    check_node_config(&def(kind), &config).into_iter().map(|p| p.message).collect()
}

fn has(found: &[String], needle: &str) -> bool {
    found.iter().any(|p| p.contains(needle))
}

const UPLOAD: &str = "| trigger.webhook --route /up --method POST";

#[test]
fn a_word_outside_a_closed_choice_is_refused_naming_the_words() {
    let found = refusals(&format!("{UPLOAD} | fs.image.thumbnail --from \"{{{{ input.webhook.files.photo }}}}\" --format jpeg"));
    assert_eq!(found, vec!["node `n1`: fs.image.thumbnail --format 'jpeg' must be one of jpg, png, webp".to_string()]);
    // Each item of a repeated flag is a word of its own.
    let found = refusals(&format!("{UPLOAD} | fs.file.put --from \"{{{{ input.webhook.files.photo }}}}\" --accept image --accept movie"));
    assert!(has(&found, "fs.file.put --accept 'movie' must be one of image, pdf"), "{found:?}");
    // The run reads a word without regard to case; so does the save.
    assert!(refusals(&format!("{UPLOAD} | fs.image.thumbnail --from \"{{{{ input.webhook.files.photo }}}}\" --format PNG")).is_empty());
}

#[test]
fn a_key_the_kind_does_not_declare_is_refused() {
    let found = config_problems("fs.image.thumbnail", json!({ "from": "a.png", "source_key": "files.photo" }));
    assert!(has(&found, "fs.image.thumbnail takes no --source-key (config key `source_key`)"), "{found:?}");
    assert!(has(&found, "— it takes --from"), "{found:?}");
    let found = config_problems("fs.image.thumbnail", json!({ "form": "a.png" }));
    assert!(has(&found, "did you mean --from?"), "{found:?}");
    // The presentation keys every kind carries are not unknown.
    assert!(config_problems("fs.image.thumbnail", json!({ "from": "a.png", "title": "Thumb", "ui": { "x": 1 }, "preview": { "out": { "as": "image" } } })).is_empty());
}

#[test]
fn a_missing_required_flag_is_refused() {
    assert_eq!(config_problems("fs.image.thumbnail", json!({})), vec!["fs.image.thumbnail needs --from".to_string()]);
    assert!(has(&config_problems("fs.image.thumbnail", json!({ "from": "  " })), "needs --from"));
}

#[test]
fn numbers_durations_and_sizes_must_parse() {
    let found = config_problems("fs.image.thumbnail", json!({ "from": "a.png", "width": "wide" }));
    assert_eq!(found, vec!["fs.image.thumbnail --width 'wide' is not a number".to_string()]);
    let found = config_problems("kv.entry.put", json!({ "key": "k", "value": "v", "ttl": 30 }));
    assert!(has(&found, "kv.entry.put --ttl"), "{found:?}");
    let found = config_problems("fs.file.put", json!({ "from": "a.png", "max_size": "ten megabytes" }));
    assert!(has(&found, "fs.file.put --max-size"), "{found:?}");
    assert!(config_problems("fs.file.put", json!({ "from": "a.png", "max_size": "10MB" })).is_empty());
    // The engine's own --timeout: a duration from 1s to 1h.
    let found = config_problems("kv.entry.get", json!({ "key": "k", "timeout": "2h" }));
    assert_eq!(found, vec!["kv.entry.get --timeout '2h' must be a duration from 1s to 1h, such as 30s, 2m or 1h".to_string()]);
    assert!(config_problems("kv.entry.get", json!({ "key": "k", "timeout": "30s" })).is_empty());
}

#[test]
fn a_value_still_to_resolve_is_left_to_the_run() {
    let found = refusals(&format!(
        "{UPLOAD} | fs.image.thumbnail --from \"{{{{ input.webhook.files.photo }}}}\" --format \"{{{{ input.webhook.body.format }}}}\" --width \"{{{{ input.webhook.body.w }}}}\" --timeout \"{{{{ input.webhook.body.t }}}}\""
    ));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_literal_only_flag_refuses_an_expression() {
    let found = config_problems("sqlite.query.run", json!({ "query": "SELECT 1", "write": "{{ input.w }}" }));
    assert_eq!(found, vec!["sqlite.query.run --write is literal, never {{ }}".to_string()]);
    let found = config_problems("ai.audio.generate", json!({ "provider": "piper", "credential_id": "c", "text": "hi", "return": "{{ input.r }}" }));
    assert!(has(&found, "ai.audio.generate --return is literal, never {{ }}; write one of inline, file"), "{found:?}");
    let found = config_problems("ai.text.generate", json!({ "provider": "{{ input.p }}", "credential_id": "c", "prompt": "hi" }));
    assert!(has(&found, "ai.text.generate --provider is literal"), "{found:?}");
    // One problem for the provider, not a second from its profile.
    assert_eq!(found.len(), 1, "{found:?}");
}

#[test]
fn a_provider_profile_is_held_at_save() {
    let found = config_problems("ai.text.generate", json!({ "provider": "openai", "credential_id": "c", "prompt": "hi", "option": { "nonsense": "1" } }));
    assert!(has(&found, "ai.text.generate --option 'nonsense' is not a setting of --provider openai"), "{found:?}");
    let found = config_problems("ai.text.generate", json!({ "provider": "acme", "credential_id": "c", "prompt": "hi" }));
    assert!(has(&found, "--provider 'acme' must be one of openai, openrouter"), "{found:?}");
}

#[test]
fn a_credential_of_another_providers_kind_is_refused_when_it_exists() {
    let graph = build("| trigger.manual | ai.text.generate --provider openai --credential main --prompt hi");
    let kind = |id: &str| (id == "main").then(|| "smtp".to_string());
    let found: Vec<String> = check_graph_nodes(&graph, &defs(), Catalogue::Complete, &kind).iter().map(ToString::to_string).collect();
    assert!(has(&found, "credential 'main' is kind 'smtp'; --provider openai takes a credential of kind openai"), "{found:?}");
    // A credential that does not exist is the run's to refuse.
    assert!(check_graph_nodes(&graph, &defs(), Catalogue::Complete, &no_credentials).is_empty());
}

#[test]
fn an_unknown_kind_is_refused_with_the_kinds_it_likely_meant() {
    let mut graph = build("| trigger.manual | javascript.script.run -- \"return 1\"");
    graph.nodes.push(PipelineNode { id: "gen".into(), kind: "static.page.generate".into(), ..graph.nodes[1].clone() });
    graph.nodes[2].config = json!({});
    let found: Vec<String> = check_graph_nodes(&graph, &defs(), Catalogue::Complete, &no_credentials).iter().map(ToString::to_string).collect();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("node `gen`: unknown node kind `static.page.generate`; did you mean"), "{found:?}");
    assert!(found[0].contains("`web.site.generate`"), "{found:?}");
    // An installed kind is left alone when only the official kinds are known.
    graph.nodes[2].kind = "x.acme.invoice.create".into();
    assert!(check_graph_nodes(&graph, &defs(), Catalogue::Official, &no_credentials).is_empty());
    assert_eq!(check_graph_nodes(&graph, &defs(), Catalogue::Complete, &no_credentials).len(), 1);
}

#[test]
fn the_flow_rules_are_refusals_too() {
    let found = refusals(
        "[a] trigger.manual\n[b] javascript.script.run -- \"return 1\"\n[c] javascript.script.run -- \"return 2\"\n[a] -> [b]\n[b] -> [c]\n[c] -> [b]\n",
    );
    assert!(has(&found, "has a cycle"), "{found:?}");
}

#[test]
fn every_problem_is_listed_at_once() {
    let found = refusals(&format!("{UPLOAD} | fs.image.thumbnail --format jpeg --fit stretch --width wide"));
    assert_eq!(found.len(), 4, "missing --from, two words, one number: {found:?}");
    let message = check(&format!("{UPLOAD} | fs.image.thumbnail --from x.png --format jpeg")).refusal_message("api/up.zf.json");
    assert_eq!(
        message,
        "pipeline 'api/up.zf.json' is refused — 1 problem:\n- node `n1`: fs.image.thumbnail --format 'jpeg' must be one of jpg, png, webp"
    );
}

// ── References (warnings) ────────────────────────────────────────────────────

#[test]
fn a_pipeline_reading_what_its_nodes_answer_is_clean() {
    let found = check(&format!(
        "{UPLOAD} | javascript.script.run -- \"return 1\" | web.response.send --body \"{{{{ {{ n: input.script, b: input.webhook.body, s: $nodes.n1.script, w: $nodes.n1.webhook }} }}}}\""
    ));
    assert!(found.is_clean(), "{found:?}");
}

#[test]
fn a_mistyped_answer_key_names_the_key_it_meant() {
    let found = warnings(&format!("{UPLOAD} | javascript.script.run -- \"return 1\" | web.response.send --body \"{{{{ input.result }}}}\""));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("`input.result` — no node upstream of `n2` answers `result`; did you mean `input.script`?"), "{found:?}");
    let found = warnings(&format!("{UPLOAD} | web.response.send --body \"{{{{ input.body.name }}}}\""));
    assert!(has(&found, "did you mean `input.webhook.body`?"), "{found:?}");
    let found = warnings(&format!("{UPLOAD} | javascript.script.run -- \"return 1\" | web.response.send --body \"{{{{ input.scirpt }}}}\""));
    assert!(has(&found, "did you mean `input.script`?"), "{found:?}");
    let found = warnings(&format!("{UPLOAD} | sqlite.query.run -- \"SELECT 1\" | web.response.send --body \"{{{{ $nodes.n1.rows }}}}\""));
    assert!(has(&found, "`$nodes.n1.rows` — `n1` answers no `rows` (its answer is `query`); did you mean `$nodes.n1.query.rows`?"), "{found:?}");
}

#[test]
fn an_input_key_two_upstream_nodes_answer_names_both() {
    let found = warnings(&format!(
        "{UPLOAD} | sqlite.query.run -- \"SELECT 1\" | sqlite.query.run -- \"SELECT 2\" | web.response.send --body \"{{{{ input.query.rows }}}}\""
    ));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("`$nodes.n1.query` and `$nodes.n2.query`"), "{found:?}");
}

#[test]
fn a_loop_item_and_its_close_answer_what_they_say() {
    let body = "[a] trigger.manual\n\
                [f] logic.foreach --from \"input.manual.body.items\"\n\
                [b] crypto.base64.encode --text \"{{ input.item }}\"\n\
                [c] logic.collect\n\
                [o] web.response.send --body \"{{ { all: input.collect, asked: input.manual } }}\"\n\
                [a] -> [f]\n[f]:item -> [b]\n[b] -> [c]\n[c] -> [o]\n";
    let found = check(body);
    assert!(found.is_clean(), "{found:?}");
    // The upstream payload is not in an item run unless kept.
    let found = warnings(&body.replace("input.item", "input.manual"));
    assert!(has(&found, "no node upstream of `b` answers `manual`"), "{found:?}");
}

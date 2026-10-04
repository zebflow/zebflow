//! AI nodes and the official composites (embedding, Telegram). A 0.10
//! composite read the payload it was given; a 0.11 composite receives only
//! its flags, so every value it read becomes an explicit flag.

use serde_json::{Value, json};

use super::{Config, Rule, words};
use crate::platform::services::migration::RewriteContext;
use crate::platform::services::migration::graph::OldOutput;
use crate::platform::services::migration::node::NodeRewrite;

pub fn rule(kind: &str) -> Option<Rule> {
    Some(match kind {
        "n.ai.agent" => Rule::new("ai.text.generate", agent_output, agent),
        "n.ai.tts" => Rule::new("ai.audio.generate", tts_output, tts),
        "n.ai.embedding" => Rule::new("ai.embedding.generate", embedding_output, embedding).errors(embedding_error),
        "n.telegram.trigger" => Rule::new("trigger.telegram", telegram_trigger_output, telegram_trigger),
        "n.telegram.send" | "n.telegram.send.photo" | "n.telegram.send.document" => {
            Rule::new("telegram.message.send", telegram_output, telegram_send).errors(telegram_error)
        }
        "n.telegram.edit" => Rule::new("telegram.message.edit", telegram_output, telegram_edit).errors(telegram_error),
        _ => return None,
    })
}

/// `--provider` for a kind that took its provider from the credential.
fn provider_from_credential(n: &mut NodeRewrite<'_>, providers: &[&str]) {
    let id = n.peek("credential_id").and_then(Value::as_str).map(str::to_string);
    let Some(id) = id else {
        n.unresolved("no credential is set, so the provider 0.10 took from it cannot be read");
        return;
    };
    if id.contains("{{") {
        n.unresolved(format!("credential `{id}` is chosen at run time; 0.11 needs --provider written down"));
        return;
    }
    match n.context.credential_kinds.get(&id) {
        Some(kind) if providers.contains(&kind.as_str()) => {
            n.set("provider", Value::String(kind.clone()));
            n.note(format!("provider = {kind} (the kind of credential {id}, which 0.10 read at run time)"));
        }
        Some(kind) => n.unresolved(format!("credential {id} is a {kind} credential; 0.11 takes {}", providers.join(" or "))),
        None => n.unresolved(format!("credential {id} is not in this project, so the provider cannot be read from it")),
    }
}

// ── ai ───────────────────────────────────────────────────────────────────────

fn agent_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::replace(
        &[
            ("response", "text.value"),
            ("verified", "text.verified"),
            ("data", "text.data"),
            ("tools_called", "text.tools_called"),
            ("iterations", "text.iterations"),
            ("budget_exhausted", "text.budget_exhausted"),
            ("chain", "text.chain"),
            ("tool_events", "text.tool_events"),
            ("metrics", "text.metrics"),
        ],
        None,
    )
}

fn agent(n: &mut NodeRewrite<'_>) {
    n.kind("ai.text.generate");
    provider_from_credential(n, &["openai", "openrouter"]);
    for key in ["credential_id", "model", "system_prompt", "budget", "schema", "max_repairs"] {
        n.keep(key);
    }
    match n.take("prompt") {
        Some(prompt) => n.set("prompt", prompt),
        None => {
            // 0.10 took the first of message, body, text, query that the
            // payload held, when it was a string.
            let mut parts = Vec::new();
            for key in ["message", "body", "text", "query"] {
                match n.locate(&[key]) {
                    Ok(Some(expr)) => parts.push(expr),
                    Ok(None) => {}
                    Err(why) => {
                        n.unresolved(format!("the prompt 0.10 read from input.{key} cannot be traced: {why}"));
                        return;
                    }
                }
            }
            if parts.is_empty() {
                n.unresolved("no --prompt, and the payload keys 0.10 fell back to (message, body, text, query) are answered by no upstream node");
            } else {
                let expr = format!(
                    "{{{{ ((v) => typeof v === 'string' ? v : undefined)([{}].find((v) => v !== undefined)) }}}}",
                    parts.join(", ")
                );
                n.note(format!("the prompt 0.10 read from the payload → prompt {expr}"));
                n.set("prompt", Value::String(expr));
            }
        }
    }
    if let Some(tools) = n.take("tools") {
        match words(&tools) {
            Some(list) if !list.is_empty() => n.set("tool", json!(list)),
            Some(_) => {}
            None => n.unresolved(format!("tools {tools} is not a list of names")),
        }
    }
    if let Some(mode) = n.take_str("output_mode") {
        if mode == "final_only" {
            n.set("answer_only", Value::Bool(true));
            n.note("output_mode final_only → answer_only");
        }
    }
    n.rename("verify_function", "verify");
}

fn tts_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::replace(
        &[
            ("audio", "audio"),
            ("audio.bytes", "audio.size"),
            ("audio.credential_id", "!0.11 does not echo the credential"),
            ("audio_blob_base64", "audio.base64"),
            ("word_timings", "audio.word_timings"),
            ("lipsync", "audio.lipsync"),
        ],
        None,
    )
}

fn tts(n: &mut NodeRewrite<'_>) {
    n.kind("ai.audio.generate");
    let provider = n.take_str("provider").unwrap_or_else(|| "piper".to_string()).trim().to_ascii_lowercase();
    n.set("provider", Value::String(provider));
    for key in ["credential_id", "text", "folder", "filename", "path", "store", "on_conflict", "speed"] {
        n.keep(key);
    }
    n.rename("speaker", "voice");
    match n.take_str("return_mode").or_else(|| n.take_str("return")).as_deref() {
        Some("file") => n.set("return", json!("file")),
        Some("blob") => n.set("return", json!("inline")),
        None | Some("both") => n.unresolved(
            "--return both (the 0.10 default) wrote the file and answered its bytes; 0.11 does one or the other — choose file or inline",
        ),
        Some(other) => n.unresolved(format!("--return {other} is not file, blob or both")),
    }
    let mut option = serde_json::Map::new();
    if let Some(volume) = n.take("volume") {
        if volume.as_f64() != Some(1.0) {
            option.insert("volume".to_string(), volume);
        }
    }
    if let Some(mode) = n.take_str("lipsync_mode").or_else(|| n.take_str("lipsync")) {
        let canonical = match mode.as_str() {
            "word_to_vowel" => "basic",
            "timed" => "timed_words",
            "audio" => "audio_guided",
            "segmented" => "audio_segmented",
            other => other,
        };
        if canonical != "none" {
            option.insert("lipsync".to_string(), Value::String(canonical.to_string()));
        }
    }
    if !option.is_empty() {
        n.note(format!("volume / lipsync → option {}", Value::Object(option.clone())));
        n.set("option", Value::Object(option));
    }
}

fn embedding_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::replace(
        &[
            ("embeddings", "embedding.vectors"),
            ("model", "embedding.model"),
            ("usage", "embedding.usage"),
            ("error", "!0.11 sends a failed call to the error pin"),
            ("raw", "!0.11 sends a failed call to the error pin"),
        ],
        None,
    )
}

fn embedding_error(_: &Config) -> OldOutput {
    OldOutput::replace(&[("error", "=`${@.embedding.error.code}: ${@.embedding.error.message}`")], None)
}

fn embedding(n: &mut NodeRewrite<'_>) {
    n.kind("ai.embedding.generate");
    provider_from_credential(n, &["openai", "openrouter"]);
    n.keep("credential_id");
    n.keep("model");
    // The composite read its text from the payload it was given: by the
    // `input_expr` mini-language (`input.a`, `input.a || input.b`,
    // `[input.a, input.b]` — each also plain JavaScript), else
    // `input.text || input.input`.
    let old = n.take_str("input_expr").map(|t| t.trim().to_string()).unwrap_or_else(|| "input.text || input.input".to_string());
    if let Some(expr) = n.old_expression(&old) {
        n.note(format!("the text the composite read from the payload → text {{{{ {expr} }}}}"));
        n.set("text", Value::String(format!("{{{{ {expr} }}}}")));
    }
    n.behaviour("a failed embedding call goes to the error pin in 0.11; 0.10 answered { error, raw } on out");
}

// ── telegram ─────────────────────────────────────────────────────────────────

fn telegram_trigger_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::open("telegram")
}

const TELEGRAM_UPDATES: &[&str] = &[
    "message", "edited_message", "channel_post", "edited_channel_post", "business_connection", "business_message",
    "edited_business_message", "deleted_business_messages", "message_reaction", "message_reaction_count", "inline_query",
    "chosen_inline_result", "callback_query", "shipping_query", "pre_checkout_query", "purchased_paid_media", "poll",
    "poll_answer", "my_chat_member", "chat_member", "chat_join_request", "chat_boost", "removed_chat_boost",
];

fn telegram_trigger(n: &mut NodeRewrite<'_>) {
    n.kind("trigger.telegram");
    n.rename("bot_credential_id", "credential_id");
    if let Some(updates) = n.take("allowed_updates") {
        match words(&updates) {
            Some(list) if list.iter().all(|w| TELEGRAM_UPDATES.contains(&w.as_str())) => {
                if !list.is_empty() {
                    n.set("event", json!(list));
                    n.note(format!("allowed_updates {updates} → event {list:?}"));
                }
            }
            _ => n.unresolved(format!("allowed_updates {updates} holds a word Telegram does not send")),
        }
    }
    n.behaviour("0.11 checks Telegram's secret header on every call; the secret is set when the pipeline is activated");
}

fn telegram_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::replace_more(
        &[
            ("response.body.result", "message.telegram"),
            ("response.body.result.message_id", "message.id"),
        ],
        "0.11 answers message: { id, recipient, sent_at, telegram }; the request echo and the raw HTTP response are gone",
    )
}

fn telegram_error(_: &Config) -> OldOutput {
    OldOutput::replace(&[("error", "=`${@.message.error.code}: ${@.message.error.message}`")], None)
}

/// A value the composite read from the payload, as a flag.
fn read(n: &mut NodeRewrite<'_>, key: &str, flag: &str, required: bool) -> bool {
    match n.locate(&[key]) {
        Ok(Some(expr)) => {
            let value = format!("{{{{ {expr} }}}}");
            n.note(format!("input.{key} the composite read → {flag} {value}"));
            n.set(flag, Value::String(value));
            true
        }
        Ok(None) if required => {
            n.unresolved(format!("the 0.10 composite read input.{key}, which no upstream node answers; 0.11 needs --{flag}"));
            false
        }
        Ok(None) => false,
        Err(why) => {
            n.unresolved(format!("input.{key} the composite read cannot be traced: {why}"));
            false
        }
    }
}

/// Values that 0.11 takes only as closed words or switches: 0.10 sent
/// whatever the payload held, so a present one cannot be carried.
fn refuse_runtime(n: &mut NodeRewrite<'_>, key: &str, flag: &str) {
    match n.locate(&[key]) {
        Ok(None) => {}
        Ok(Some(_)) => n.unresolved(format!(
            "input.{key} sent Telegram a value chosen at run time; 0.11 takes --{flag} as a written word"
        )),
        Err(why) => n.unresolved(format!("input.{key} cannot be traced: {why}")),
    }
}

fn telegram_send(n: &mut NodeRewrite<'_>) {
    n.kind("telegram.message.send");
    n.rename("bot_credential_id", "credential_id");
    read(n, "chat_id", "recipient", true);
    match n.old_kind.as_str() {
        "n.telegram.send.photo" => {
            read(n, "photo", "image", true);
            if !read(n, "caption", "text", false) {
                n.unresolved("a photo without a caption: 0.11 always needs --text");
            }
        }
        "n.telegram.send.document" => {
            read(n, "document", "file", true);
            if !read(n, "caption", "text", false) {
                n.unresolved("a document without a caption: 0.11 always needs --text");
            }
        }
        _ => {
            read(n, "text", "text", true);
            read(n, "reply_to_message_id", "in_reply_to", false);
        }
    }
    refuse_runtime(n, "parse_mode", "format");
    refuse_runtime(n, "disable_notification", "silent");
}

fn telegram_edit(n: &mut NodeRewrite<'_>) {
    n.kind("telegram.message.edit");
    n.rename("bot_credential_id", "credential_id");
    read(n, "chat_id", "recipient", true);
    read(n, "message_id", "id", true);
    read(n, "text", "text", true);
    refuse_runtime(n, "parse_mode", "format");
}

//! Pure parser for the Pipeline DSL command language.
//!
//! `DslFlag.config_key` is the single source of truth for flag→config mapping.
//! Every flag used in DSL must be declared in the node's `dsl_flags`. Undeclared
//! flags are a parse error — no auto-rule, no fallback.

use std::collections::HashMap;

use serde_json::{Map, Value, json};

use crate::pipeline::auto_tidy_pipeline_graph;
use crate::pipeline::model::{
    DslFlag, DslFlagKind, NodeDefinition, PipelineEdge, PipelineGraph, PipelineNode,
};
use crate::pipeline::nodes::builtin_node_definitions;

/// Parsed DSL command verb ready for execution.
#[derive(Debug, Clone)]
pub enum DslVerb {
    /// `get <resource> [--path <p>] [--status <s>] [--filter <f>]`
    Get {
        resource: String,
        path: Option<String>,
        filter: Option<String>,
        status: Option<String>,
    },
    /// `describe <kind> <name> [--compact]`
    Describe {
        kind: String,
        name: String,
        compact: bool,
    },
    /// `read <kind> <name>`
    Read { kind: String, name: String },
    /// `write <kind> <name> [body after --]`
    Write {
        kind: String,
        name: String,
        body: Option<String>,
    },
    /// `delete <kind> <name>`
    Delete { kind: String, name: String },
    /// `activate pipeline <file_rel_path>`
    Activate { file_rel_path: String },
    /// `deactivate pipeline <file_rel_path>`
    Deactivate { file_rel_path: String },
    /// `execute pipeline <file_rel_path> [--input <json>]`
    Execute { file_rel_path: String, input: Value },
    /// `register <file_rel_path> [--title <t>] [--description <d>] [--as-json] [| ...]`
    Register {
        file_rel_path: String,
        title: String,
        description: String,
        as_json: bool,
        body: String,
    },
    /// `patch pipeline <file_rel_path> node <id> [flags...]`
    Patch {
        file_rel_path: String,
        node_id: String,
        flags: HashMap<String, Value>,
        body: Option<String>,
    },
    /// `run [--dry-run] [| ...]`
    Run { body: String, dry_run: bool },
    /// `git <subcommand> [args...] [-- <body>]`
    Git {
        subcommand: String,
        args: Vec<String>,
        body: Option<String>,
    },
    /// `node help <kind>`
    NodeHelp { kind: String },
    /// Credential write blocked
    CredentialBlocked { reason: String },
    /// Unknown verb
    Unknown { raw: String },
}

/// Tokenize a DSL string respecting single and double quoted strings.
pub fn tokenize(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut in_backtick = false;

    for ch in s.chars() {
        match ch {
            '\'' if !in_double && !in_backtick => {
                in_single = !in_single;
            }
            '"' if !in_single && !in_backtick => {
                in_double = !in_double;
            }
            '`' if !in_single && !in_double => {
                in_backtick = !in_backtick;
                current.push(ch);
            }
            ' ' | '\t' | '\n' | '\r' if !in_single && !in_double && !in_backtick => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// Split DSL string into individual commands.
/// Joins `\` line continuations and splits on `&&` outside quotes/backticks.
pub fn split_commands(dsl: &str) -> Vec<String> {
    let joined = dsl.replace("\\\r\n", "\n").replace("\\\n", "\n");
    let bytes = joined.as_bytes();
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_backtick = false;
    let mut in_opaque_body = false;
    let mut i = 0;
    while i < bytes.len() {
        if !in_single
            && !in_double
            && !in_backtick
            && !in_opaque_body
            && bytes[i] == b'-'
            && i + 1 < bytes.len()
            && bytes[i + 1] == b'-'
            && is_standalone_token(bytes, i, i + 2)
            && command_accepts_opaque_body(&joined[start..i])
        {
            in_opaque_body = true;
            i += 2;
            continue;
        }

        match bytes[i] {
            b'\'' if !in_opaque_body && !in_double && !in_backtick => in_single = !in_single,
            b'"' if !in_opaque_body && !in_single && !in_backtick => in_double = !in_double,
            b'`' if !in_opaque_body && !in_single && !in_double => in_backtick = !in_backtick,
            b'&' if !in_opaque_body
                && !in_single
                && !in_double
                && !in_backtick
                && i + 1 < bytes.len()
                && bytes[i + 1] == b'&' =>
            {
                let seg = joined[start..i].trim();
                if !seg.is_empty() {
                    segments.push(seg.to_string());
                }
                i += 2;
                start = i;
                in_opaque_body = false;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    let last = joined[start..].trim();
    if !last.is_empty() {
        segments.push(last.to_string());
    }
    segments
}

fn is_standalone_token(bytes: &[u8], start: usize, end: usize) -> bool {
    let before_ok = start == 0 || bytes[start - 1].is_ascii_whitespace();
    let after_ok = end >= bytes.len() || bytes[end].is_ascii_whitespace();
    before_ok && after_ok
}

fn command_accepts_opaque_body(prefix: &str) -> bool {
    let tokens = tokenize(prefix);
    let Some(verb) = tokens.first().map(|s| s.to_lowercase()) else {
        return false;
    };
    matches!(
        verb.as_str(),
        "patch" | "register" | "reg" | "run" | "write" | "create" | "git"
    )
}

/// Expand short node kind alias to full qualified kind.
pub fn expand_kind(short: &str) -> Option<&'static str> {
    match short {
        "trigger.webhook" | "n.trigger.webhook" => Some("n.trigger.webhook"),
        "trigger.schedule" | "n.trigger.schedule" => Some("n.trigger.schedule"),
        "trigger.manual" | "n.trigger.manual" => Some("n.trigger.manual"),
        "pg.query" | "n.pg.query" => Some("n.pg.query"),
        "sekejap.insert" | "n.sekejap.insert" => Some("n.sekejap.insert"),
        "sekejap.query" | "n.sekejap.query" => Some("n.sekejap.query"),
        "sqlite.query" | "n.sqlite.query" => Some("n.sqlite.query"),
        "sqlite.mutate" | "n.sqlite.mutate" => Some("n.sqlite.mutate"),
        "table.convert" | "n.table.convert" => Some("n.table.convert"),
        "table.query" | "n.table.query" => Some("n.table.query"),
        "script" | "n.script" => Some("n.script"),
        "web.response" | "n.web.response" => Some("n.web.response"),
        "web.static.generate" | "n.web.static.generate" => Some("n.web.static.generate"),
        "web.docs.generate" | "n.web.docs.generate" => Some("n.web.docs.generate"),
        "http.request" | "n.http.request" => Some("n.http.request"),
        "zebtune" | "n.zebtune" => Some("n.ai.agent"),
        "logic.if" | "n.logic.if" => Some("n.logic.if"),
        "logic.match" | "n.logic.match" => Some("n.logic.match"),
        "logic.collect" | "n.logic.collect" => Some("n.logic.collect"),
        "logic.foreach" | "n.logic.foreach" => Some("n.logic.foreach"),
        "logic.reduce" | "n.logic.reduce" => Some("n.logic.reduce"),
        "logic.retry" | "n.logic.retry" => Some("n.logic.retry"),
        "trigger.ws" | "n.trigger.ws" => Some("n.trigger.ws"),
        "ws.emit" | "n.ws.emit" => Some("n.ws.emit"),
        "ws.sync_state" | "n.ws.sync_state" => Some("n.ws.sync_state"),
        "auth.token.create" | "n.auth.token.create" => Some("n.auth.token.create"),
        "crypto" | "n.crypto" => Some("n.crypto"),
        "ai.agent" | "n.ai.agent" => Some("n.ai.agent"),
        "ai.tts" | "n.ai.tts" => Some("n.ai.tts"),
        "browser.run" | "n.browser.run" => Some("n.browser.run"),
        "trigger.weberror" | "n.trigger.weberror" => Some("n.trigger.weberror"),
        "trigger.function" | "n.trigger.function" => Some("n.trigger.function"),
        "function.call" | "n.function.call" => Some("n.function.call"),
        "fs.save" | "n.fs.save" => Some("n.fs.save"),
        "fs.compress" | "n.fs.compress" => Some("n.fs.compress"),
        "fs.decompress" | "n.fs.decompress" => Some("n.fs.decompress"),
        "fs.pdf.convert" | "n.fs.pdf.convert" => Some("n.fs.pdf.convert"),
        "fs.thumbnail" | "n.fs.thumbnail" => Some("n.fs.thumbnail"),
        "fs.list" | "n.fs.list" => Some("n.fs.list"),
        "fs.head" | "n.fs.head" => Some("n.fs.head"),
        "fs.get" | "n.fs.get" => Some("n.fs.get"),
        "fs.put" | "n.fs.put" => Some("n.fs.put"),
        "fs.delete" | "n.fs.delete" => Some("n.fs.delete"),
        "fs.copy" | "n.fs.copy" => Some("n.fs.copy"),
        "fs.move" | "n.fs.move" => Some("n.fs.move"),
        "fs.mkdir" | "n.fs.mkdir" => Some("n.fs.mkdir"),
        "kv.set" | "n.kv.set" => Some("n.kv.set"),
        "kv.get" | "n.kv.get" => Some("n.kv.get"),
        "kv.del" | "n.kv.del" => Some("n.kv.del"),
        "kv.exists" | "n.kv.exists" => Some("n.kv.exists"),
        "kv.expire" | "n.kv.expire" => Some("n.kv.expire"),
        "kv.incr" | "n.kv.incr" => Some("n.kv.incr"),
        "kv.publish" | "n.kv.publish" => Some("n.kv.publish"),
        "trigger.kv.subscribe" | "n.trigger.kv.subscribe" => Some("n.trigger.kv.subscribe"),
        "geo.inspect" | "n.geo.inspect" => Some("n.geo.inspect"),
        "geo.convert" | "n.geo.convert" => Some("n.geo.convert"),
        "ms.publish" | "n.ms.publish" => Some("n.ms.publish"),
        "ms.unpublish" | "n.ms.unpublish" => Some("n.ms.unpublish"),
        "ms.get" | "n.ms.get" => Some("n.ms.get"),
        "ms.list" | "n.ms.list" => Some("n.ms.list"),
        "trigger.ws.client" | "n.trigger.ws.client" => Some("n.trigger.ws.client"),
        "ws.client.send" | "n.ws.client.send" => Some("n.ws.client.send"),
        _ => None,
    }
}

/// Default input/output pins per node kind.
pub fn default_pins(kind: &str) -> (Vec<String>, Vec<String>) {
    match kind {
        "n.trigger.webhook" | "n.trigger.schedule" | "n.trigger.manual" | "n.trigger.function" => {
            (vec![], vec!["out".to_string()])
        }
        "n.pg.query" | "n.sekejap.query" | "n.sekejap.insert" | "n.sqlite.query"
        | "n.table.convert" | "n.table.query" | "n.script" | "n.http.request"
        | "n.logic.collect" | "n.ai.tts" => (vec!["in".to_string()], vec!["out".to_string()]),
        "n.logic.foreach" => (vec!["in".to_string()], vec!["item".to_string()]),
        "n.logic.reduce" => (vec!["in".to_string()], vec!["out".to_string()]),
        "n.logic.retry" => (
            vec!["in".to_string()],
            vec!["retry".to_string(), "failed".to_string()],
        ),
        "n.logic.if" => (
            vec!["in".to_string()],
            vec!["true".to_string(), "false".to_string()],
        ),
        // n.logic.match: output pins are dynamic (set per-instance from cases config).
        // Return just ["default"] as the fallback; actual pins are set after config is parsed.
        "n.logic.match" => (vec!["in".to_string()], vec!["default".to_string()]),
        "n.web.response" => (vec!["in".to_string()], vec!["out".to_string()]),
        "n.trigger.ws" | "n.trigger.ws.client" => (vec![], vec!["out".to_string()]),
        "n.ws.emit" | "n.ws.sync_state" | "n.ws.client.send" => {
            (vec!["in".to_string()], vec!["out".to_string()])
        }
        "n.function.call" => (
            vec!["in".to_string()],
            vec!["out".to_string(), "error".to_string()],
        ),
        _ => (vec!["in".to_string()], vec!["out".to_string()]),
    }
}

/// Strips matching outer `"..."` or `'...'` from a string.
/// Returns the byte offset of the first top-level `|` in `raw` (not inside quotes).
/// Used to extract pipe bodies from raw command strings without losing quote context.
fn find_first_pipe_in_raw(raw: &str) -> Option<usize> {
    let mut in_single = false;
    let mut in_double = false;
    let mut in_backtick = false;
    for (i, ch) in raw.char_indices() {
        match ch {
            '\'' if !in_double && !in_backtick => in_single = !in_single,
            '"' if !in_single && !in_backtick => in_double = !in_double,
            '`' if !in_single && !in_double => in_backtick = !in_backtick,
            '|' if !in_single && !in_double && !in_backtick => return Some(i),
            _ => {}
        }
    }
    None
}

fn find_first_graph_marker_in_raw(raw: &str) -> Option<usize> {
    let mut in_single = false;
    let mut in_double = false;
    let mut in_backtick = false;
    for (i, ch) in raw.char_indices() {
        match ch {
            '\'' if !in_double && !in_backtick => in_single = !in_single,
            '"' if !in_single && !in_backtick => in_double = !in_double,
            '`' if !in_single && !in_double => in_backtick = !in_backtick,
            '[' if !in_single && !in_double && !in_backtick => return Some(i),
            _ => {}
        }
    }
    None
}

fn extract_pipeline_body(raw: &str) -> String {
    match (
        find_first_pipe_in_raw(raw),
        find_first_graph_marker_in_raw(raw),
    ) {
        (Some(pipe), Some(graph)) => raw[pipe.min(graph)..].to_string(),
        (Some(pipe), None) => raw[pipe..].to_string(),
        (None, Some(graph)) => raw[graph..].to_string(),
        (None, None) => String::new(),
    }
}

fn strip_outer_quotes(s: &str) -> &str {
    if s.len() >= 2
        && ((s.starts_with('"') && s.ends_with('"'))
            || (s.starts_with('\'') && s.ends_with('\''))
            || (s.starts_with('`') && s.ends_with('`')))
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Extracts the raw body substring after ` -- ` in a segment string.
/// Strips outer quotes if the entire body is quoted.
fn extract_raw_body_from(raw: &str) -> Option<String> {
    find_body_delimiter(raw)
        .map(|body_start| {
            let after = raw[body_start..].trim();
            strip_outer_quotes(after).to_string()
        })
        .filter(|s| !s.is_empty())
}

fn find_body_delimiter(raw: &str) -> Option<usize> {
    let bytes = raw.as_bytes();
    let mut in_single = false;
    let mut in_double = false;
    let mut in_backtick = false;
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'\'' if !in_double && !in_backtick => in_single = !in_single,
            b'"' if !in_single && !in_backtick => in_double = !in_double,
            b'`' if !in_single && !in_double => in_backtick = !in_backtick,
            b'-' if !in_single
                && !in_double
                && !in_backtick
                && i + 1 < bytes.len()
                && bytes[i + 1] == b'-'
                && is_standalone_token(bytes, i, i + 2) =>
            {
                let mut body_start = i + 2;
                while body_start < bytes.len() && bytes[body_start].is_ascii_whitespace() {
                    body_start += 1;
                }
                return Some(body_start);
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Coerce a DSL flag string value to the appropriate JSON type.
/// "true"/"false" → bool, integer strings → i64, float strings → f64, else string.
fn coerce_scalar_value(s: &str) -> Value {
    match s {
        "true" => json!(true),
        "false" => json!(false),
        _ => {
            if let Ok(n) = s.parse::<i64>() {
                json!(n)
            } else if let Ok(f) = s.parse::<f64>() {
                json!(f)
            } else {
                json!(s)
            }
        }
    }
}

/// Parse flag→config key mapping and body from token list after node kind.
///
/// Every `--flag` must be declared in the node's `dsl_flags`. The `config_key`
/// from the matching `DslFlag` entry is the authoritative destination key.
/// Unknown flags (not declared in `dsl_flags`) are a hard parse error.
pub fn parse_node_config(
    tokens: &[String],
    raw: &str,
    dsl_flags: &[DslFlag],
) -> Result<(Value, Option<String>), String> {
    let mut config = serde_json::Map::new();
    let mut list_modes = HashMap::<String, ListFlagMode>::new();
    let mut body: Option<String> = None;
    let mut i = 0;

    while i < tokens.len() {
        let t = &tokens[i];

        if t == "--" {
            body = extract_raw_body_from(raw);
            break;
        }

        if let Some(key) = t.strip_prefix("--") {
            let flag_str = format!("--{key}");
            let dsl_flag = dsl_flags
                .iter()
                .find(|f| f.flag == flag_str)
                .ok_or_else(|| {
                    format!("unknown flag `--{key}` — not declared in this node's dsl_flags")
                })?;

            match dsl_flag.kind {
                DslFlagKind::Scalar => {
                    let val = tokens.get(i + 1).cloned().unwrap_or_default();
                    config.insert(dsl_flag.config_key.clone(), coerce_scalar_value(&val));
                    i += 2;
                }
                DslFlagKind::CommaSeparatedList | DslFlagKind::RepeatedList => {
                    let val = tokens.get(i + 1).cloned().unwrap_or_default();
                    push_list_flag_value(&mut config, &mut list_modes, dsl_flag, &val)?;
                    i += 2;
                }
                DslFlagKind::Bool => {
                    config.insert(dsl_flag.config_key.clone(), json!(true));
                    i += 1;
                }
                DslFlagKind::KeyValuePairs => {
                    let raw = tokens.get(i + 1).cloned().unwrap_or_default();
                    let (k, v) = if let Some(eq) = raw.find('=') {
                        (raw[..eq].trim().to_string(), raw[eq + 1..].to_string())
                    } else {
                        (raw.trim().to_string(), String::new())
                    };
                    let entry = config
                        .entry(dsl_flag.config_key.clone())
                        .or_insert_with(|| Value::Object(serde_json::Map::new()));
                    if let Value::Object(m) = entry {
                        m.insert(k, json!(v));
                    }
                    i += 2;
                }
                DslFlagKind::SchemaField => {
                    let spec = tokens.get(i + 1).cloned().unwrap_or_default();
                    if spec.trim().is_empty() {
                        return Err(format!(
                            "schema field flag `{}` requires name:type",
                            dsl_flag.flag
                        ));
                    }
                    let desc = tokens.get(i + 2).filter(|v| !v.starts_with("--")).cloned();
                    push_schema_field(&mut config, &dsl_flag.config_key, &spec, desc.as_deref())?;
                    i += if desc.is_some() { 3 } else { 2 };
                }
            }
        } else {
            i += 1;
        }
    }

    Ok((Value::Object(config), body))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListFlagMode {
    Compact,
    Repeated,
}

fn push_list_flag_value(
    config: &mut serde_json::Map<String, Value>,
    list_modes: &mut HashMap<String, ListFlagMode>,
    flag: &DslFlag,
    value: &str,
) -> Result<(), String> {
    let has_comma = value.contains(',');
    let next_mode = if has_comma {
        ListFlagMode::Compact
    } else {
        ListFlagMode::Repeated
    };
    let key = flag.config_key.clone();
    if let Some(existing) = list_modes.get(&key) {
        if *existing != next_mode || has_comma {
            return Err(format!(
                "list flag `{}` must use one style per node command: either `{0} a,b,c` or `{0} a {0} b`, not both",
                flag.flag
            ));
        }
    } else {
        list_modes.insert(key.clone(), next_mode);
    }

    let values = if has_comma {
        value
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| json!(s))
            .collect::<Vec<_>>()
    } else if value.trim().is_empty() {
        Vec::new()
    } else {
        vec![json!(value)]
    };

    let entry = config
        .entry(key)
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Value::Array(existing) = entry {
        existing.extend(values);
    }
    Ok(())
}

fn push_schema_field(
    config: &mut Map<String, Value>,
    config_key: &str,
    spec: &str,
    description: Option<&str>,
) -> Result<(), String> {
    let (name, property, required) = schema_field_from_spec(spec, description)?;
    let entry = config
        .entry(config_key.to_string())
        .or_insert_with(|| json!({ "type": "object", "properties": {}, "required": [] }));
    if !entry.is_object() {
        *entry = json!({ "type": "object", "properties": {}, "required": [] });
    }
    let schema = entry.as_object_mut().expect("schema object");
    schema.insert("type".to_string(), json!("object"));
    let properties = schema
        .entry("properties".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !properties.is_object() {
        *properties = Value::Object(Map::new());
    }
    properties
        .as_object_mut()
        .expect("properties object")
        .insert(name.clone(), property);
    if required {
        let required_values = schema
            .entry("required".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        if !required_values.is_array() {
            *required_values = Value::Array(Vec::new());
        }
        let arr = required_values.as_array_mut().expect("required array");
        if !arr.iter().any(|v| v.as_str() == Some(name.as_str())) {
            arr.push(json!(name));
        }
    }
    Ok(())
}

fn schema_field_from_spec(
    spec: &str,
    description: Option<&str>,
) -> Result<(String, Value, bool), String> {
    let (raw_name, raw_type) = spec
        .split_once(':')
        .ok_or_else(|| format!("schema field `{spec}` must use name:type syntax"))?;
    let name = raw_name.trim();
    if name.is_empty() {
        return Err(format!("schema field `{spec}` has empty name"));
    }
    let mut type_spec = raw_type.trim();
    let required = type_spec.ends_with('!');
    if required {
        type_spec = type_spec.trim_end_matches('!').trim();
    }
    let mut property = schema_property_for_type(type_spec)?;
    if let Some(desc) = description.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(obj) = property.as_object_mut() {
            obj.insert("description".to_string(), json!(desc));
        }
    }
    Ok((name.to_string(), property, required))
}

fn schema_property_for_type(type_spec: &str) -> Result<Value, String> {
    let trimmed = type_spec.trim();
    if trimmed.is_empty() {
        return Err("schema field type must not be empty".to_string());
    }
    if let Some(item) = trimmed.strip_suffix("[]") {
        return Ok(json!({
            "type": "array",
            "items": schema_property_for_type(item)?
        }));
    }
    match trimmed {
        "string" | "number" | "integer" | "boolean" | "object" | "array" => {
            Ok(json!({ "type": trimmed }))
        }
        "json" | "any" => Ok(json!({})),
        "file" | "bytes" | "blob" => Ok(json!({
            "type": "object",
            "x-zebflow-type": trimmed
        })),
        other => Err(format!(
            "unsupported schema field type `{other}`; use string, number, integer, boolean, object, array, any, json, file, bytes, blob, or []"
        )),
    }
}

/// Parse flags for patch operations without DslFlag validation.
/// Used only by `parse_patch` where node kind is not known at parse time.
/// Validation against DslFlags happens later in the executor.
fn parse_flags_for_patch(tokens: &[String], cmd: &str) -> (HashMap<String, Value>, Option<String>) {
    let mut flags: HashMap<String, Value> = HashMap::new();
    let mut body: Option<String> = None;
    let mut i = 0;

    while i < tokens.len() {
        let t = &tokens[i];
        if t == "--" {
            body = extract_raw_body_from(cmd);
            break;
        }
        if let Some(key) = t.strip_prefix("--") {
            let val = tokens.get(i + 1).cloned().unwrap_or_default();
            let config_key = key.replace('-', "_");
            let new_val = coerce_scalar_value(&val);
            // Accumulate repeated flags as an array to preserve all occurrences
            // (e.g. --claim sub=$.id --claim name=$.fullname:public)
            match flags.entry(config_key) {
                std::collections::hash_map::Entry::Occupied(mut e) => match e.get_mut() {
                    Value::Array(arr) => arr.push(new_val),
                    existing => {
                        let prev = existing.clone();
                        *existing = Value::Array(vec![prev, new_val]);
                    }
                },
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(new_val);
                }
            }
            i += 2;
        } else {
            i += 1;
        }
    }

    (flags, body)
}

/// Parse `register <file_rel_path> [--title t] [--description d] [--as-json] <body>`
fn parse_register(tokens: &[String], cmd: &str) -> DslVerb {
    let file_rel_path = tokens.get(1).cloned().unwrap_or_default();
    let mut title = String::new();
    let mut description = String::new();
    let mut as_json = false;
    let mut i = 2;

    while i < tokens.len() {
        match tokens[i].as_str() {
            "--title" => {
                title = tokens.get(i + 1).cloned().unwrap_or_default();
                i += 2;
            }
            "--description" => {
                description = tokens.get(i + 1).cloned().unwrap_or_default();
                i += 2;
            }
            "--as-json" => {
                as_json = true;
                i += 1;
            }
            _ => break,
        }
    }

    // Extract body from raw string to preserve quoted values (e.g. --cron "* * * * *").
    // Pipe mode bodies start with `|`; graph mode bodies start with `[`.
    let body = extract_pipeline_body(cmd);

    DslVerb::Register {
        file_rel_path,
        title,
        description,
        as_json,
        body,
    }
}

/// Parse `patch pipeline <file_rel_path> node <id> [flags] [-- body]`
fn parse_patch(tokens: &[String], cmd: &str) -> DslVerb {
    let file_rel_path = tokens.get(2).cloned().unwrap_or_default();
    let node_id = tokens.get(4).cloned().unwrap_or_default();
    let flag_tokens = if tokens.len() > 5 {
        tokens[5..].to_vec()
    } else {
        vec![]
    };
    let (flags, body) = parse_flags_for_patch(&flag_tokens, cmd);
    DslVerb::Patch {
        file_rel_path,
        node_id,
        flags,
        body,
    }
}

fn extract_flag(tokens: &[String], flag: &str) -> Option<String> {
    let pos = tokens.iter().position(|t| t == flag)?;
    tokens.get(pos + 1).cloned()
}

fn extract_body(tokens: &[String]) -> Option<String> {
    let pos = tokens.iter().position(|t| t == "--")?;
    let rest = tokens[pos + 1..].join(" ");
    if rest.is_empty() { None } else { Some(rest) }
}

/// Parse one command string into a `DslVerb`.
pub fn parse_one_command(cmd: &str) -> DslVerb {
    let tokens = tokenize(cmd);
    if tokens.is_empty() {
        return DslVerb::Unknown {
            raw: cmd.to_string(),
        };
    }

    match tokens[0].to_lowercase().as_str() {
        "get" => {
            let resource = tokens.get(1).cloned().unwrap_or_default().to_lowercase();
            let path = extract_flag(&tokens, "--path");
            let filter = extract_flag(&tokens, "--filter");
            let status = extract_flag(&tokens, "--status");
            DslVerb::Get { resource, path, filter, status }
        }
        "describe" => {
            let kind = tokens.get(1).cloned().unwrap_or_default().to_lowercase();
            let name = tokens.get(2).cloned().unwrap_or_default();
            let compact = tokens.iter().any(|t| t == "--compact");
            DslVerb::Describe { kind, name, compact }
        }
        "read" => {
            let kind = tokens.get(1).cloned().unwrap_or_default().to_lowercase();
            let name = tokens.get(2).cloned().unwrap_or_default();
            DslVerb::Read { kind, name }
        }
        "write" | "create" => {
            let kind = tokens.get(1).cloned().unwrap_or_default().to_lowercase();
            let name = tokens.get(2).cloned().unwrap_or_default();
            let body = extract_body(&tokens);
            DslVerb::Write { kind, name, body }
        }
        "delete" | "rm" | "remove" => {
            let kind = tokens.get(1).cloned().unwrap_or_default().to_lowercase();
            let name = tokens.get(2).cloned().unwrap_or_default();
            DslVerb::Delete { kind, name }
        }
        "activate" => {
            // Accept both "activate <path>" and "activate pipeline <path>"
            let mut idx = 1;
            if tokens.get(1).map(|s| s.as_str()) == Some("pipeline") { idx = 2; }
            let file_rel_path = tokens.get(idx).cloned().unwrap_or_default();
            DslVerb::Activate { file_rel_path }
        }
        "deactivate" => {
            // Accept both "deactivate <path>" and "deactivate pipeline <path>"
            let mut idx = 1;
            if tokens.get(1).map(|s| s.as_str()) == Some("pipeline") { idx = 2; }
            let file_rel_path = tokens.get(idx).cloned().unwrap_or_default();
            DslVerb::Deactivate { file_rel_path }
        }
        "execute" | "exec" => {
            // Accept both "execute <path>" and "execute pipeline <path>"
            let mut idx = 1;
            if tokens.get(1).map(|s| s.as_str()) == Some("pipeline") { idx = 2; }
            let file_rel_path = tokens.get(idx).cloned().unwrap_or_default();
            let input_str = extract_flag(&tokens, "--input").unwrap_or_default();
            let input = serde_json::from_str(&input_str).unwrap_or(json!({}));
            DslVerb::Execute { file_rel_path, input }
        }
        "register" | "reg" => parse_register(&tokens, cmd),
        "patch" => parse_patch(&tokens, cmd),
        "run" => {
            let dry_run = tokens.iter().any(|t| t == "--dry-run");
            // Extract body from raw string to preserve quoted values.
            let body = extract_pipeline_body(cmd);
            DslVerb::Run { body, dry_run }
        }
        "git" => {
            let subcommand = tokens.get(1).cloned().unwrap_or_default().to_lowercase();
            let dash_pos = tokens.iter().position(|t| t == "--");
            let args = if let Some(pos) = dash_pos {
                tokens[2..pos].to_vec()
            } else {
                tokens[2..].to_vec()
            };
            let body = dash_pos.and_then(|pos| {
                let b = tokens[pos + 1..].join(" ");
                if b.is_empty() { None } else { Some(b) }
            });
            DslVerb::Git { subcommand, args, body }
        }
        "node" => {
            let sub = tokens.get(1).cloned().unwrap_or_default().to_lowercase();
            if sub == "help" {
                let kind = tokens.get(2).cloned().unwrap_or_default();
                DslVerb::NodeHelp { kind }
            } else {
                DslVerb::Unknown { raw: cmd.to_string() }
            }
        }
        "credential" | "credentials" | "secret" | "secrets"
        | "set-secret" | "set-credential" | "set-env" => {
            DslVerb::CredentialBlocked {
                reason: "Credential writes are blocked via DSL for security. Use the Credentials UI at /projects/{owner}/{project}/credentials".to_string(),
            }
        }
        _ => DslVerb::Unknown { raw: cmd.to_string() },
    }
}

/// Build a `PipelineGraph` from pipe (`|`) or graph (`[id] ->`) notation.
pub fn build_pipeline_graph(id: &str, body: &str) -> Result<PipelineGraph, String> {
    let all_defs = builtin_node_definitions();
    build_pipeline_graph_with_definitions(id, body, &all_defs)
}

/// Build a `PipelineGraph` using a caller-supplied node catalog.
///
/// Platform-facing callers should pass the merged native + composite registry so DSL flags
/// declared by installed/embedded composites are parsed the same way native node flags are.
pub fn build_pipeline_graph_with_definitions(
    id: &str,
    body: &str,
    definitions: &[NodeDefinition],
) -> Result<PipelineGraph, String> {
    let body = body.trim();
    if body.is_empty() {
        return Err("Pipeline body is empty".to_string());
    }
    // Detect graph mode: body contains `[` and `] ->` (with space before arrow)
    if body.contains('[') && body.contains("] ->") {
        build_graph_mode(id, body, definitions)
    } else {
        build_pipe_mode(id, body, definitions)
    }
}

/// Split a pipe-notation body into segments, ignoring `|` inside quotes/backticks.
fn split_pipe_segments(body: &str) -> Vec<&str> {
    let bytes = body.as_bytes();
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_backtick = false;
    let mut in_opaque_body = false;

    for (byte_pos, ch) in body.char_indices() {
        if !in_opaque_body
            && !in_single
            && !in_double
            && !in_backtick
            && ch == '-'
            && byte_pos + 1 < bytes.len()
            && bytes[byte_pos + 1] == b'-'
            && is_standalone_token(bytes, byte_pos, byte_pos + 2)
        {
            in_opaque_body = true;
            continue;
        }

        match ch {
            '\'' if !in_opaque_body && !in_double && !in_backtick => in_single = !in_single,
            '"' if !in_opaque_body && !in_single && !in_backtick => in_double = !in_double,
            '`' if !in_opaque_body && !in_single && !in_double => in_backtick = !in_backtick,
            '|' if !in_single
                && !in_double
                && !in_backtick
                && (!in_opaque_body || pipe_starts_node_segment(body, byte_pos)) =>
            {
                let seg = body[start..byte_pos].trim();
                if !seg.is_empty() {
                    segments.push(seg);
                }
                start = byte_pos + ch.len_utf8();
                in_opaque_body = false;
            }
            _ => {}
        }
    }
    let last = body[start..].trim();
    if !last.is_empty() {
        segments.push(last);
    }
    segments
}

fn pipe_starts_node_segment(body: &str, pipe_pos: usize) -> bool {
    let bytes = body.as_bytes();
    if pipe_pos > 0 && bytes[pipe_pos - 1] == b'|' {
        return false;
    }
    if pipe_pos + 1 < bytes.len() && bytes[pipe_pos + 1] == b'|' {
        return false;
    }

    let after = body[pipe_pos + 1..].trim_start();
    let Some(raw_kind) = after.split_whitespace().next() else {
        return false;
    };
    let kind = raw_kind.trim_matches(|ch: char| ch == ';' || ch == ',' || ch == ')');
    expand_kind(kind).is_some() || kind.starts_with("n.c.") || kind.starts_with("c.")
}

/// Build pipeline from graph notation: `[label] node_kind --flags...\n[from] -> [to]`
fn build_graph_mode(
    id: &str,
    body: &str,
    definitions: &[NodeDefinition],
) -> Result<PipelineGraph, String> {
    let mut nodes: Vec<PipelineNode> = Vec::new();
    let mut edges: Vec<PipelineEdge> = Vec::new();

    for statement in graph_mode_statements(body) {
        if is_graph_edge_statement(&statement) {
            // Edge declaration: [from]:pin -> [to]:pin  or  [from] -> [to]
            parse_graph_edge(&statement, &mut edges)?;
        } else {
            // Node declaration: [label] node_kind --flags...
            parse_graph_node(&statement, &mut nodes, definitions)?;
        }
    }

    // Entry nodes = nodes with no incoming edges
    let to_nodes: std::collections::HashSet<&str> =
        edges.iter().map(|e| e.to_node.as_str()).collect();
    let entry_nodes: Vec<String> = nodes
        .iter()
        .filter(|n| !to_nodes.contains(n.id.as_str()))
        .map(|n| n.id.clone())
        .collect();

    let mut graph = PipelineGraph {
        kind: "zebflow.pipeline".to_string(),
        version: "0.1".to_string(),
        id: id.to_string(),
        description: None,
        metadata: None,
        entry_nodes,
        nodes,
        edges,
    };
    auto_tidy_pipeline_graph(&mut graph);
    Ok(graph)
}

fn graph_mode_statements(body: &str) -> Vec<String> {
    let joined = body.replace("\\\r\n", " ").replace("\\\n", " ");
    let mut statements = Vec::new();
    let mut current: Option<String> = None;

    for line in joined.lines().map(str::trim).filter(|s| !s.is_empty()) {
        if is_graph_structural_statement(line) {
            if let Some(statement) = current.take() {
                statements.push(statement);
            }
            current = Some(line.to_string());
        } else if let Some(statement) = current.as_mut() {
            statement.push('\n');
            statement.push_str(line);
        }
    }

    if let Some(statement) = current {
        statements.push(statement);
    }
    statements
}

fn is_graph_structural_statement(line: &str) -> bool {
    is_graph_edge_statement(line) || is_graph_node_statement(line)
}

fn is_graph_edge_statement(line: &str) -> bool {
    let Some(arrow_pos) = line.find("->") else {
        return false;
    };
    graph_endpoint_is_complete(&line[..arrow_pos])
        && graph_endpoint_is_complete(&line[arrow_pos + 2..])
}

fn graph_endpoint_is_complete(value: &str) -> bool {
    let value = value.trim();
    let Some(inner) = value.strip_prefix('[') else {
        return false;
    };
    let Some((label, rest)) = inner.split_once(']') else {
        return false;
    };
    if label.trim().is_empty() {
        return false;
    }
    let rest = rest.trim();
    if rest.is_empty() {
        return true;
    }
    let Some(pin) = rest.strip_prefix(':') else {
        return false;
    };
    let pin = pin.trim();
    !pin.is_empty() && !pin.chars().any(char::is_whitespace)
}

fn is_graph_node_statement(line: &str) -> bool {
    let Some(inner) = line.strip_prefix('[') else {
        return false;
    };
    let Some((label, rest)) = inner.split_once(']') else {
        return false;
    };
    if label.trim().is_empty() {
        return false;
    }
    let tokens = tokenize(rest.trim());
    let Some(raw_kind) = tokens.first().map(String::as_str) else {
        return false;
    };
    expand_kind(raw_kind).is_some() || raw_kind.starts_with("n.c.") || raw_kind.starts_with("c.")
}

#[cfg(test)]
mod tests {
    use crate::pipeline::model::{DslFlag, DslFlagKind, NodeDefinition, PipelineNode};
    use serde_json::json;

    use super::{
        DslVerb, build_pipeline_graph, build_pipeline_graph_with_definitions,
        node_to_segment_no_body, parse_one_command, split_commands,
    };

    #[test]
    fn logic_match_cases_accept_compact_list_form() {
        let dsl = r#"
[a] trigger.manual
[b] logic.match --expr "$input.type" --cases create,update,delete --default unknown

[a] -> [b]
"#;

        let graph = build_pipeline_graph("parser-match-cases-test", dsl).expect("graph");
        let node = graph
            .nodes
            .iter()
            .find(|node| node.id == "b")
            .expect("match node");

        assert_eq!(
            node.config.get("cases"),
            Some(&json!(["create", "update", "delete"]))
        );
        assert_eq!(
            node.output_pins,
            vec![
                "create".to_string(),
                "update".to_string(),
                "delete".to_string(),
                "unknown".to_string()
            ]
        );
    }

    #[test]
    fn logic_match_cases_accept_repeated_list_form() {
        let dsl = r#"
[a] trigger.manual
[b] logic.match --expr "$input.type" --cases create --cases update --cases delete --default unknown

[a] -> [b]
"#;

        let graph = build_pipeline_graph("parser-match-cases-repeated-test", dsl).expect("graph");
        let node = graph
            .nodes
            .iter()
            .find(|node| node.id == "b")
            .expect("match node");

        assert_eq!(
            node.config.get("cases"),
            Some(&json!(["create", "update", "delete"]))
        );
    }

    #[test]
    fn list_flags_reject_mixed_compact_and_repeated_forms() {
        let dsl = r#"
[a] trigger.manual
[b] logic.match --expr "$input.type" --cases create,update --cases delete --default unknown

[a] -> [b]
"#;

        let err = build_pipeline_graph("parser-match-cases-mixed-test", dsl)
            .expect_err("mixed list syntax must fail");
        assert!(err.contains("must use one style"));
    }

    #[test]
    fn repeated_list_reconstructs_source_binding_objects() {
        let node = PipelineNode {
            id: "b".to_string(),
            kind: "n.table.query".to_string(),
            config: json!({
                "sources": [
                    { "source": "datasets/posts.csv", "alias": "posts" },
                    { "source": "$input.rows", "alias": "rows" }
                ],
                "query": "select * from posts",
                "to_json": true
            }),
            input_pins: vec!["in".to_string()],
            output_pins: vec!["out".to_string()],
        };

        let segment = node_to_segment_no_body(&node);
        assert!(segment.contains("--from \"datasets/posts.csv as posts\""));
        assert!(segment.contains("--from \"$input.rows as rows\""));
        assert!(segment.contains("--to-json"));
    }

    #[test]
    fn split_commands_preserves_graph_mode_lines_for_run() {
        let dsl = r#"
run \
  [a] trigger.manual \
  [b] script -- "return { ok: true };" \
  [a] -> [b]
"#;

        let commands = split_commands(dsl);
        assert_eq!(commands.len(), 1);

        let verb = parse_one_command(&commands[0]);
        match verb {
            DslVerb::Run { body, dry_run } => {
                assert!(!dry_run);
                let graph = build_pipeline_graph("graph-run-test", &body).expect("graph");
                assert_eq!(graph.nodes.len(), 2);
                assert_eq!(graph.edges.len(), 1);
            }
            other => panic!("expected run verb, got {other:?}"),
        }
    }

    #[test]
    fn split_commands_respects_quotes_around_ampersand() {
        let dsl = r#"register pipelines/test [trigger] trigger.manual
[echo] script -- "if (a && b) { return 1; }"
[trigger] -> [echo]"#;
        let commands = split_commands(dsl);
        assert_eq!(commands.len(), 1, "&& inside double quotes must not split");

        let dsl2 = r#"register pipelines/test [t] trigger.manual
[s] script -- `${a && b}`
[t] -> [s]"#;
        let commands2 = split_commands(dsl2);
        assert_eq!(commands2.len(), 1, "&& inside backticks must not split");

        let dsl3 = "get pipelines && activate pipelines/foo";
        let commands3 = split_commands(dsl3);
        assert_eq!(commands3.len(), 2, "&& outside quotes must still split");
    }

    #[test]
    fn register_graph_body_prefers_graph_marker_before_js_pipe() {
        let dsl = r#"register pipelines/e1/rename [a] trigger.manual
[b] script -- const left = input.old_name;
const ok = /old|new/.test(left) || left === "legacy|name";
return { ok };
[a] -> [b]"#;

        let commands = split_commands(dsl);
        assert_eq!(commands.len(), 1);
        let verb = parse_one_command(&commands[0]);
        match verb {
            DslVerb::Register { body, .. } => {
                assert!(
                    body.trim_start().starts_with("[a]"),
                    "graph body must start at the first graph marker, not at a later JS pipe"
                );
                let graph = build_pipeline_graph("register-graph-js-pipe", &body).expect("graph");
                assert_eq!(graph.nodes.len(), 2);
                assert_eq!(graph.edges.len(), 1);
                let script = graph
                    .nodes
                    .iter()
                    .find(|node| node.id == "b")
                    .expect("script node");
                let source = script
                    .config
                    .get("source")
                    .and_then(|value| value.as_str())
                    .expect("source");
                assert!(source.contains("/old|new/"));
                assert!(source.contains("legacy|name"));
                assert!(source.contains("||"));
            }
            other => panic!("expected register verb, got {other:?}"),
        }
    }

    #[test]
    fn pipe_mode_long_script_body_keeps_downstream_nodes() {
        let dsl = r#"
| trigger.manual
| script -- const ok = !!(input.left || input.right);
const label = "left|right";
return { ok, label };
| script -- return { downstream: input.ok, label: input.label };
"#;

        let graph = build_pipeline_graph("pipe-long-script-body", dsl).expect("graph");
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.nodes[1].kind, "n.script");
        assert_eq!(graph.nodes[2].kind, "n.script");
        let first_source = graph.nodes[1]
            .config
            .get("source")
            .and_then(|value| value.as_str())
            .expect("first source");
        assert!(first_source.contains("input.left || input.right"));
        assert!(first_source.contains("left|right"));
        let second_source = graph.nodes[2]
            .config
            .get("source")
            .and_then(|value| value.as_str())
            .expect("second source");
        assert!(second_source.contains("downstream"));
    }

    #[test]
    fn graph_mode_multiline_script_body_keeps_edges() {
        let dsl = r#"
[a] trigger.manual
[b] script -- const values = [1, 2, 3];
[1, 2, 3].map((value) => value + 1);
return { values };
[a] -> [b]
"#;

        let graph = build_pipeline_graph("graph-multiline-script-body", dsl).expect("graph");
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        let source = graph
            .nodes
            .iter()
            .find(|node| node.id == "b")
            .expect("script node")
            .config
            .get("source")
            .and_then(|value| value.as_str())
            .expect("source");
        assert!(source.contains("[1, 2, 3].map"));
        assert!(source.contains("return { values };"));
    }

    #[test]
    fn registry_definitions_parse_composite_dsl_flags() {
        let mut definitions = crate::pipeline::nodes::builtin_node_definitions();
        definitions.push(NodeDefinition {
            kind: "n.c.ai.embedding".to_string(),
            title: "AI Embedding".to_string(),
            description: "Composite embedding node.".to_string(),
            input_pins: vec!["in".to_string()],
            output_pins: vec!["out".to_string(), "error".to_string()],
            dsl_flags: vec![
                DslFlag {
                    flag: "--credential".to_string(),
                    config_key: "credential_id".to_string(),
                    description: "OpenAI-compatible credential.".to_string(),
                    kind: DslFlagKind::Scalar,
                    required: true,
                },
                DslFlag {
                    flag: "--model".to_string(),
                    config_key: "model".to_string(),
                    description: "Embedding model.".to_string(),
                    kind: DslFlagKind::Scalar,
                    required: false,
                },
                DslFlag {
                    flag: "--input-expr".to_string(),
                    config_key: "input_expr".to_string(),
                    description: "Text expression.".to_string(),
                    kind: DslFlagKind::Scalar,
                    required: false,
                },
            ],
            ..Default::default()
        });

        let graph = build_pipeline_graph_with_definitions(
            "composite-embedding-dsl",
            r#"
| trigger.manual
| n.c.ai.embedding --credential qwen-embed --model text-embedding-v4 --input-expr input.text
"#,
            &definitions,
        )
        .expect("graph");
        let node = graph
            .nodes
            .iter()
            .find(|node| node.kind == "n.c.ai.embedding")
            .expect("composite node");
        assert_eq!(node.config["credential_id"], json!("qwen-embed"));
        assert_eq!(node.config["model"], json!("text-embedding-v4"));
        assert_eq!(node.config["input_expr"], json!("input.text"));
    }

    #[test]
    fn patch_script_body_is_opaque_to_shell_separators() {
        let source = r#"const lat = row.Stop_lat ?? row.stop_lat;
const lon = row.Stop_long ?? row.stop_long;
const ok = !!((lat || lon) && /Stop_lat|Stop_long/.test("Stop_lat|Stop_long"));
return { ok, label: "lat|lon", pair: `${lat || ""}|${lon || ""}` };"#;
        let dsl = format!(
            "patch pipeline pipelines/goveyes/functions/e1/ingestion-tools/e1-tool-ing-transform-tabular-spatial.zf.json node n1 -- {source}"
        );

        let commands = split_commands(&dsl);
        assert_eq!(
            commands.len(),
            1,
            "operators inside a patch body must not become command separators"
        );

        let verb = parse_one_command(&commands[0]);
        match verb {
            DslVerb::Patch {
                file_rel_path,
                node_id,
                body,
                ..
            } => {
                assert_eq!(
                    file_rel_path,
                    "pipelines/goveyes/functions/e1/ingestion-tools/e1-tool-ing-transform-tabular-spatial.zf.json"
                );
                assert_eq!(node_id, "n1");
                let body = body.expect("patch body");
                assert!(body.contains("lat || lon"));
                assert!(body.contains("/Stop_lat|Stop_long/"));
                assert!(body.contains("\"lat|lon\""));
                assert!(body.contains("`${lat || \"\"}|${lon || \"\"}`"));
            }
            other => panic!("expected patch verb, got {other:?}"),
        }
    }

    #[test]
    fn function_trigger_schema_field_flags_build_json_schema() {
        let dsl = r#"
[fn] trigger.function --title "Inspect CSV" --description "Reads a CSV." --input source:file! "CSV file reference." --input options:any "Provider options." --output ok:boolean! "Whether it worked." --output columns:string[] "Detected columns."
[fn] -> [done]
[done] script -- return input;
"#;

        let graph = build_pipeline_graph("function-schema-field-test", dsl).expect("graph");
        let node = graph.nodes.iter().find(|node| node.id == "fn").expect("fn");

        assert_eq!(node.config["description"], json!("Reads a CSV."));
        assert_eq!(node.config["input_schema"]["type"], json!("object"));
        assert_eq!(node.config["input_schema"]["required"], json!(["source"]));
        assert_eq!(
            node.config["input_schema"]["properties"]["source"]["x-zebflow-type"],
            json!("file")
        );
        assert_eq!(
            node.config["input_schema"]["properties"]["options"],
            json!({"description": "Provider options."})
        );
        assert_eq!(node.config["output_schema"]["required"], json!(["ok"]));
        assert_eq!(
            node.config["output_schema"]["properties"]["columns"]["items"]["type"],
            json!("string")
        );
    }
}

fn parse_graph_node(
    line: &str,
    nodes: &mut Vec<PipelineNode>,
    definitions: &[NodeDefinition],
) -> Result<(), String> {
    let rest = line
        .strip_prefix('[')
        .ok_or("expected '[' at start of node declaration")?;
    let (label, rest) = rest
        .split_once(']')
        .ok_or("expected ']' in node declaration")?;
    let label = label.trim();
    if label.is_empty() {
        return Err("node label must not be empty".to_string());
    }
    let rest = rest.trim();
    let tokens = tokenize(rest);
    if tokens.is_empty() {
        return Err(format!("node '[{label}]' has no kind"));
    }
    let raw_kind = &tokens[0];
    let composite_kind: String;
    let full_kind = match expand_kind(raw_kind) {
        Some(k) => k,
        None if raw_kind.starts_with("n.c.") => {
            composite_kind = raw_kind.to_string();
            &composite_kind
        }
        None if raw_kind.starts_with("c.") => {
            composite_kind = format!("n.{raw_kind}");
            &composite_kind
        }
        None => return Err(format!("Unknown node kind: '{raw_kind}'")),
    };
    let (input_pins, mut output_pins) = default_pins(full_kind);
    let dsl_flags = definitions
        .iter()
        .find(|d| d.kind == full_kind)
        .map(|d| d.dsl_flags.as_slice())
        .unwrap_or(&[]);
    let (mut config, body_val) = parse_node_config(&tokens[1..], rest, dsl_flags)?;
    if let Some(bval) = body_val {
        let body_key = match full_kind {
            "n.pg.query" => "query",
            "n.sekejap.query" => "query",
            "n.sqlite.query" => "query",
            "n.sqlite.mutate" => "query",
            "n.table.query" => "query",
            "n.script" => "source",
            "n.logic.match" | "n.logic.if" => "expression",
            "n.browser.run" => "code",
            _ => "body",
        };
        if let Value::Object(ref mut map) = config {
            map.insert(body_key.to_string(), json!(bval));
        }
    }
    // For logic.match, output pins are dynamic: the declared cases + the default pin.
    if full_kind == "n.logic.match" {
        if let Value::Object(ref map) = config {
            let cases: Vec<String> = map
                .get("cases")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let default_pin = map
                .get("default")
                .and_then(|v| v.as_str())
                .unwrap_or("default")
                .to_string();
            let mut pins = cases;
            if !pins.contains(&default_pin) {
                pins.push(default_pin);
            }
            if !pins.is_empty() {
                output_pins = pins;
            }
        }
    }
    nodes.push(PipelineNode {
        id: label.to_string(),
        kind: full_kind.to_string(),
        input_pins,
        output_pins,
        config,
    });
    Ok(())
}

fn parse_graph_edge(line: &str, edges: &mut Vec<PipelineEdge>) -> Result<(), String> {
    // Format: [from]:pin -> [to]:pin  or  [from] -> [to]
    let arrow_pos = line.find("->").ok_or("expected '->' in edge declaration")?;
    let from_part = line[..arrow_pos].trim();
    let to_part = line[arrow_pos + 2..].trim();

    let (from_node, from_pin) = parse_node_pin_part(from_part, "out")?;
    let (to_node, to_pin) = parse_node_pin_part(to_part, "in")?;

    edges.push(PipelineEdge {
        from_node,
        from_pin,
        to_node,
        to_pin,
    });
    Ok(())
}

/// Parse `[label]` or `[label]:pin` into `(node_id, pin)`. `default_pin` used when no `:pin`.
fn parse_node_pin_part(s: &str, default_pin: &str) -> Result<(String, String), String> {
    let s = s.trim();
    let inner = s
        .strip_prefix('[')
        .ok_or_else(|| format!("expected '[' in edge endpoint: '{s}'"))?;
    let (label, rest) = inner
        .split_once(']')
        .ok_or_else(|| format!("expected ']' in edge endpoint: '{s}'"))?;
    let label = label.trim().to_string();
    let pin = rest
        .trim()
        .strip_prefix(':')
        .map(|p| p.trim().to_string())
        .unwrap_or_else(|| default_pin.to_string());
    Ok((label, pin))
}

// ─── DSL Reconstruction (inverse of build_pipeline_graph) ───────────────────

/// Reconstruct DSL text from a compiled `PipelineGraph`.
///
/// Emits pipe mode (`| node1\n| node2`) for simple linear chains where every
/// edge uses default "out"→"in" pins and no node has more than one in/out edge.
/// Emits graph mode (`[label] kind ...\n[a] -> [b]`) for all other topologies.
pub fn graph_to_dsl(graph: &PipelineGraph) -> String {
    if is_linear_graph(graph) {
        graph_to_pipe_mode(graph)
    } else {
        graph_to_graph_mode(graph)
    }
}

/// True iff the graph is a simple linear chain — no branching, no merging,
/// all edges on default "out"→"in" pins.
fn is_linear_graph(graph: &PipelineGraph) -> bool {
    let mut in_count: HashMap<&str, usize> = HashMap::new();
    let mut out_count: HashMap<&str, usize> = HashMap::new();

    for edge in &graph.edges {
        if edge.from_pin != "out" || edge.to_pin != "in" {
            return false;
        }
        *in_count.entry(edge.to_node.as_str()).or_insert(0) += 1;
        *out_count.entry(edge.from_node.as_str()).or_insert(0) += 1;
    }

    graph.nodes.iter().all(|n| {
        *in_count.get(n.id.as_str()).unwrap_or(&0) <= 1
            && *out_count.get(n.id.as_str()).unwrap_or(&0) <= 1
    })
}

/// Reconstruct the DSL segment for one node: `kind --flag val... -- body`
///
/// Uses `dsl_flags` as the authoritative reverse-mapping (config_key → flag).
/// Fields not declared in `dsl_flags` (e.g. `title`, `params_path`) are omitted.
fn node_to_segment(node: &PipelineNode) -> String {
    let all_defs = builtin_node_definitions();
    let dsl_flags = all_defs
        .iter()
        .find(|d| d.kind == node.kind)
        .map(|d| d.dsl_flags.as_slice())
        .unwrap_or(&[]);

    // Strip "n." prefix for cleaner output; expand_kind accepts both forms.
    let kind = node.kind.strip_prefix("n.").unwrap_or(&node.kind);
    let mut parts = vec![kind.to_string()];

    for flag in dsl_flags {
        let Some(val) = node.config.get(&flag.config_key) else {
            continue;
        };
        match &flag.kind {
            DslFlagKind::Bool => {
                if val.as_bool().unwrap_or(false) {
                    parts.push(flag.flag.clone());
                }
            }
            DslFlagKind::CommaSeparatedList => {
                if let Some(arr) = val.as_array() {
                    let csv = arr
                        .iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(",");
                    if !csv.is_empty() {
                        parts.push(flag.flag.clone());
                        parts.push(csv);
                    }
                }
            }
            DslFlagKind::RepeatedList => {
                if let Some(arr) = val.as_array() {
                    for item in arr.iter().filter_map(repeated_list_item_to_string) {
                        if item.is_empty() {
                            continue;
                        }
                        parts.push(flag.flag.clone());
                        parts.push(quote_dsl_arg(&item));
                    }
                }
            }
            DslFlagKind::Scalar => {
                let s = match val {
                    Value::String(s) if !s.is_empty() => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => continue,
                };
                parts.push(flag.flag.clone());
                // Quote values containing spaces.
                parts.push(quote_dsl_arg(&s));
            }
            DslFlagKind::KeyValuePairs => {
                if let Some(map) = val.as_object() {
                    for (k, v) in map {
                        let v_str = match v {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        parts.push(flag.flag.clone());
                        parts.push(format!("{}={}", k, v_str));
                    }
                }
            }
            DslFlagKind::SchemaField => {
                if let Ok(s) = serde_json::to_string(val) {
                    parts.push(
                        flag.flag
                            .replace("--input", "--input-schema")
                            .replace("--output", "--output-schema"),
                    );
                    parts.push(quote_dsl_arg(&s));
                }
            }
        }
    }

    // Body (SQL / script source / generic body) — stored under a kind-specific key.
    let body_key = match node.kind.as_str() {
        "n.pg.query" => "query",
        "n.sekejap.query" => "query",
        "n.sqlite.query" => "query",
        "n.sqlite.mutate" => "query",
        "n.table.query" => "query",
        "n.script" => "source",
        "n.logic.match" | "n.logic.if" => "expression",
        _ => "body",
    };
    if let Some(body) = node.config.get(body_key).and_then(|v| v.as_str()) {
        let body = body.trim();
        if !body.is_empty() {
            // Collapse internal newlines so the segment stays on one line.
            let inline: String = body
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            parts.push("--".to_string());
            parts.push(inline);
        }
    }

    parts.join(" ")
}

/// Like `node_to_segment` but omits the `-- body` portion.
/// Used by compact describe to show flags without long SQL/script bodies.
pub fn node_to_segment_no_body(node: &PipelineNode) -> String {
    let all_defs = builtin_node_definitions();
    let dsl_flags = all_defs
        .iter()
        .find(|d| d.kind == node.kind)
        .map(|d| d.dsl_flags.as_slice())
        .unwrap_or(&[]);

    let kind = node.kind.strip_prefix("n.").unwrap_or(&node.kind);
    let mut parts = vec![kind.to_string()];

    for flag in dsl_flags {
        // Skip body-typed flags (their config_key matches the body key for this node kind)
        let body_key = match node.kind.as_str() {
            "n.pg.query" => "query",
            "n.sekejap.query" => "query",
            "n.sqlite.query" => "query",
            "n.sqlite.mutate" => "query",
            "n.table.query" => "query",
            "n.script" => "source",
            "n.logic.match" | "n.logic.if" => "expression",
            _ => "body",
        };
        if flag.config_key == body_key {
            continue;
        }
        let Some(val) = node.config.get(&flag.config_key) else {
            continue;
        };
        match &flag.kind {
            DslFlagKind::Bool => {
                if val.as_bool().unwrap_or(false) {
                    parts.push(flag.flag.clone());
                }
            }
            DslFlagKind::CommaSeparatedList => {
                if let Some(arr) = val.as_array() {
                    let csv = arr
                        .iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(",");
                    if !csv.is_empty() {
                        parts.push(flag.flag.clone());
                        parts.push(csv);
                    }
                }
            }
            DslFlagKind::RepeatedList => {
                if let Some(arr) = val.as_array() {
                    for item in arr.iter().filter_map(repeated_list_item_to_string) {
                        if item.is_empty() {
                            continue;
                        }
                        parts.push(flag.flag.clone());
                        parts.push(quote_dsl_arg(&item));
                    }
                }
            }
            DslFlagKind::Scalar => {
                let s = match val {
                    Value::String(s) if !s.is_empty() => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => continue,
                };
                parts.push(flag.flag.clone());
                parts.push(quote_dsl_arg(&s));
            }
            DslFlagKind::KeyValuePairs => {
                if let Some(map) = val.as_object() {
                    for (k, v) in map {
                        let v_str = match v {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        parts.push(flag.flag.clone());
                        parts.push(format!("{}={}", k, v_str));
                    }
                }
            }
            DslFlagKind::SchemaField => {
                if let Ok(s) = serde_json::to_string(val) {
                    parts.push(
                        flag.flag
                            .replace("--input", "--input-schema")
                            .replace("--output", "--output-schema"),
                    );
                    parts.push(quote_dsl_arg(&s));
                }
            }
        }
    }

    parts.join(" ")
}

fn quote_dsl_arg(value: &str) -> String {
    if value.chars().any(char::is_whitespace) || value.contains('"') {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

fn repeated_list_item_to_string(value: &Value) -> Option<String> {
    if let Some(s) = value.as_str() {
        return Some(s.to_string());
    }
    let obj = value.as_object()?;
    let source = obj.get("source").and_then(Value::as_str)?.trim();
    let alias = obj.get("alias").and_then(Value::as_str)?.trim();
    if source.is_empty() || alias.is_empty() {
        return None;
    }
    Some(format!("{source} as {alias}"))
}

fn graph_to_pipe_mode(graph: &PipelineGraph) -> String {
    let node_map: HashMap<&str, &PipelineNode> =
        graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let next_map: HashMap<&str, &str> = graph
        .edges
        .iter()
        .map(|e| (e.from_node.as_str(), e.to_node.as_str()))
        .collect();

    let mut ordered: Vec<&PipelineNode> = Vec::new();
    if let Some(first_id) = graph.entry_nodes.first() {
        let mut cur = first_id.as_str();
        let mut visited = std::collections::HashSet::new();
        loop {
            if !visited.insert(cur) {
                break; // cycle guard
            }
            if let Some(&node) = node_map.get(cur) {
                ordered.push(node);
            }
            match next_map.get(cur) {
                Some(&next) => cur = next,
                None => break,
            }
        }
    }

    ordered
        .iter()
        .map(|n| format!("| {}", node_to_segment(n)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn graph_to_graph_mode(graph: &PipelineGraph) -> String {
    let mut lines: Vec<String> = Vec::new();

    for node in &graph.nodes {
        lines.push(format!("[{}] {}", node.id, node_to_segment(node)));
    }

    if !graph.edges.is_empty() {
        lines.push(String::new());
        for edge in &graph.edges {
            if edge.from_pin == "out" && edge.to_pin == "in" {
                lines.push(format!("[{}] -> [{}]", edge.from_node, edge.to_node));
            } else {
                lines.push(format!(
                    "[{}]:{} -> [{}]:{}",
                    edge.from_node, edge.from_pin, edge.to_node, edge.to_pin
                ));
            }
        }
    }

    lines.join("\n")
}

// ─── Pipe mode builder ───────────────────────────────────────────────────────

/// Build pipeline from pipe-notation: `trigger.webhook --path /test | pg.query --credential main`
fn build_pipe_mode(
    id: &str,
    body: &str,
    definitions: &[NodeDefinition],
) -> Result<PipelineGraph, String> {
    // Strip leading `|` if present
    let body = body.trim_start_matches('|').trim();
    let segments: Vec<&str> = split_pipe_segments(body);

    if segments.is_empty() {
        return Err("No nodes in pipeline body".to_string());
    }

    let mut nodes: Vec<PipelineNode> = Vec::new();
    let mut edges: Vec<PipelineEdge> = Vec::new();

    // Check if first node is a trigger kind
    let first_tokens = tokenize(segments[0]);
    let first_raw_kind = first_tokens.first().map(|s| s.as_str()).unwrap_or("");
    let first_full_kind = expand_kind(first_raw_kind).unwrap_or(first_raw_kind);
    let has_trigger_first = first_full_kind.starts_with("n.trigger.");

    // Auto-prepend trigger.manual if first node is not a trigger
    if !has_trigger_first {
        nodes.push(PipelineNode {
            id: "trigger".to_string(),
            kind: "n.trigger.manual".to_string(),
            input_pins: vec![],
            output_pins: vec!["out".to_string()],
            config: json!({}),
        });
    }

    for (idx, segment) in segments.iter().enumerate() {
        let seg_tokens = tokenize(segment);
        if seg_tokens.is_empty() {
            continue;
        }

        let raw_kind = &seg_tokens[0];
        let composite_kind: String;
        let full_kind = match expand_kind(raw_kind) {
            Some(k) => k,
            None if raw_kind.starts_with("n.c.") => {
                composite_kind = raw_kind.to_string();
                &composite_kind
            }
            None if raw_kind.starts_with("c.") => {
                composite_kind = format!("n.{raw_kind}");
                &composite_kind
            }
            None => return Err(format!("Unknown node kind: '{raw_kind}'")),
        };

        let node_id = format!("n{idx}");
        let (input_pins, mut output_pins) = default_pins(full_kind);
        let dsl_flags = definitions
            .iter()
            .find(|d| d.kind == full_kind)
            .map(|d| d.dsl_flags.as_slice())
            .unwrap_or(&[]);
        let (mut config, body_val) = parse_node_config(&seg_tokens[1..], segment, dsl_flags)?;

        // Set body using kind-appropriate key
        if let Some(bval) = body_val {
            let body_key = match full_kind {
                "n.pg.query" => "query",
                "n.sekejap.query" => "query",
                "n.sqlite.query" => "query",
                "n.sqlite.mutate" => "query",
                "n.table.query" => "query",
                "n.script" => "source",
                "n.browser.run" => "code",
                _ => "body",
            };
            if let Value::Object(ref mut map) = config {
                map.insert(body_key.to_string(), json!(bval));
            }
        }

        // For logic.match, output pins are dynamic: the declared cases + the default pin.
        if full_kind == "n.logic.match" {
            if let Value::Object(ref map) = config {
                let cases: Vec<String> = map
                    .get("cases")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                let default_pin = map
                    .get("default")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default")
                    .to_string();
                let mut pins = cases;
                if !pins.contains(&default_pin) {
                    pins.push(default_pin);
                }
                if !pins.is_empty() {
                    output_pins = pins;
                }
            }
        }

        // Determine from_node for edge
        let from_node = if idx == 0 {
            if !has_trigger_first {
                // Edge from auto-prepended trigger to first real node
                Some("trigger".to_string())
            } else {
                None // First node is the trigger — no incoming edge
            }
        } else {
            Some(format!("n{}", idx - 1))
        };

        if let Some(from_id) = from_node {
            edges.push(PipelineEdge {
                from_node: from_id,
                from_pin: "out".to_string(),
                to_node: node_id.clone(),
                to_pin: "in".to_string(),
            });
        }

        nodes.push(PipelineNode {
            id: node_id,
            kind: full_kind.to_string(),
            input_pins,
            output_pins,
            config,
        });
    }

    let entry_nodes = nodes
        .first()
        .map(|n| vec![n.id.clone()])
        .unwrap_or_default();

    let mut graph = PipelineGraph {
        kind: "zebflow.pipeline".to_string(),
        version: "0.1".to_string(),
        id: id.to_string(),
        description: None,
        metadata: None,
        entry_nodes,
        nodes,
        edges,
    };
    auto_tidy_pipeline_graph(&mut graph);
    Ok(graph)
}

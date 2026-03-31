//! Pure parser for the Pipeline DSL command language.
//!
//! `DslFlag.config_key` is the single source of truth for flag→config mapping.
//! Every flag used in DSL must be declared in the node's `dsl_flags`. Undeclared
//! flags are a parse error — no auto-rule, no fallback.

use std::collections::HashMap;

use serde_json::{Value, json};

use crate::pipeline::model::{DslFlag, DslFlagKind, PipelineEdge, PipelineGraph, PipelineNode};
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
    /// `describe <kind> <name>`
    Describe { kind: String, name: String },
    /// `read <kind> <name>`
    Read { kind: String, name: String },
    /// `write <kind> <name> [body after --]`
    Write { kind: String, name: String, body: Option<String> },
    /// `delete <kind> <name>`
    Delete { kind: String, name: String },
    /// `activate pipeline <file_rel_path>`
    Activate { file_rel_path: String },
    /// `deactivate pipeline <file_rel_path>`
    Deactivate { file_rel_path: String },
    /// `execute pipeline <file_rel_path> [--input <json>]`
    Execute { file_rel_path: String, input: Value },
    /// `register <file_rel_path> [--title <t>] [--as-json] [| ...]`
    Register {
        file_rel_path: String,
        title: String,
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
    Git { subcommand: String, args: Vec<String>, body: Option<String> },
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

    for ch in s.chars() {
        match ch {
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            ' ' | '\t' | '\n' | '\r' if !in_single && !in_double => {
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
/// Joins `\` line continuations and splits on `&&`.
pub fn split_commands(dsl: &str) -> Vec<String> {
    let joined = dsl.replace("\\\n", " ").replace("\\\r\n", " ");
    joined
        .split("&&")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Expand short node kind alias to full qualified kind.
pub fn expand_kind(short: &str) -> Option<&'static str> {
    match short {
        "trigger.webhook" | "n.trigger.webhook" => Some("n.trigger.webhook"),
        "trigger.schedule" | "n.trigger.schedule" => Some("n.trigger.schedule"),
        "trigger.manual" | "n.trigger.manual" => Some("n.trigger.manual"),
        "pg.query" | "n.pg.query" => Some("n.pg.query"),
        "script" | "n.script" => Some("n.script"),
        "web.response" | "n.web.response" => Some("n.web.response"),
        "http.request" | "n.http.request" => Some("n.http.request"),
        "sekejap.query" | "n.sekejap.query" => Some("n.sekejap.query"),
        // Backward-compat alias — old pipelines using n.sjtable.query still work
        "sjtable.query" | "n.sjtable.query" => Some("n.sekejap.query"),
        "fanout" | "n.fanout" | "logic.branch" | "n.logic.branch" => Some("n.logic.branch"),
        "zebtune" | "n.zebtune" => Some("n.zebtune"),
        "logic.if" | "n.logic.if" => Some("n.logic.if"),
        "logic.switch" | "n.logic.switch" => Some("n.logic.switch"),
        "logic.merge" | "n.logic.merge" => Some("n.logic.merge"),
        "trigger.ws" | "n.trigger.ws" => Some("n.trigger.ws"),
        "ws.emit" | "n.ws.emit" => Some("n.ws.emit"),
        "ws.sync_state" | "n.ws.sync_state" => Some("n.ws.sync_state"),
        "auth.token.create" | "n.auth.token.create" => Some("n.auth.token.create"),
        "crypto" | "n.crypto" => Some("n.crypto"),
        "ai.agent" | "n.ai.agent" => Some("n.ai.agent"),
        "browser.run" | "n.browser.run" => Some("n.browser.run"),
        "trigger.weberror" | "n.trigger.weberror" => Some("n.trigger.weberror"),
        "trigger.function" | "n.trigger.function" => Some("n.trigger.function"),
        "function.call" | "n.function.call" => Some("n.function.call"),
        _ => None,
    }
}

/// Default input/output pins per node kind.
pub fn default_pins(kind: &str) -> (Vec<String>, Vec<String>) {
    match kind {
        "n.trigger.webhook" | "n.trigger.schedule" | "n.trigger.manual" | "n.trigger.function" => {
            (vec![], vec!["out".to_string()])
        }
        "n.pg.query" | "n.sekejap.query" | "n.sjtable.query" | "n.script" | "n.http.request"
        | "n.zebtune" | "n.logic.branch" | "n.logic.merge" => {
            (vec!["in".to_string()], vec!["out".to_string()])
        }
        "n.logic.if" => (
            vec!["in".to_string()],
            vec!["true".to_string(), "false".to_string()],
        ),
        // n.logic.switch: output pins are dynamic (set per-instance from cases config).
        // Return just ["default"] as the fallback; actual pins are set after config is parsed.
        "n.logic.switch" => (
            vec!["in".to_string()],
            vec!["default".to_string()],
        ),
        "n.web.response" => (vec!["in".to_string()], vec!["out".to_string()]),
        "n.trigger.ws" => (vec![], vec!["out".to_string()]),
        "n.ws.emit" | "n.ws.sync_state" => (vec!["in".to_string()], vec!["out".to_string()]),
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
    for (i, ch) in raw.char_indices() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '|' if !in_single && !in_double => return Some(i),
            _ => {}
        }
    }
    None
}

fn strip_outer_quotes(s: &str) -> &str {
    if s.len() >= 2
        && ((s.starts_with('"') && s.ends_with('"'))
            || (s.starts_with('\'') && s.ends_with('\'')))
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Extracts the raw body substring after ` -- ` in a segment string.
/// Strips outer quotes if the entire body is quoted.
fn extract_raw_body_from(raw: &str) -> Option<String> {
    raw.find(" -- ").map(|pos| {
        let after = raw[pos + 4..].trim();
        strip_outer_quotes(after).to_string()
    }).filter(|s| !s.is_empty())
}

/// Coerce a DSL flag string value to the appropriate JSON type.
/// "true"/"false" → bool, integer strings → i64, float strings → f64, else string.
fn coerce_scalar_value(s: &str) -> Value {
    match s {
        "true" => json!(true),
        "false" => json!(false),
        _ => {
            if let Ok(n) = s.parse::<i64>() { json!(n) }
            else if let Ok(f) = s.parse::<f64>() { json!(f) }
            else { json!(s) }
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
            let dsl_flag = dsl_flags.iter().find(|f| f.flag == flag_str).ok_or_else(|| {
                format!(
                    "unknown flag `--{key}` — not declared in this node's dsl_flags"
                )
            })?;

            match dsl_flag.kind {
                DslFlagKind::Scalar => {
                    let val = tokens.get(i + 1).cloned().unwrap_or_default();
                    config.insert(dsl_flag.config_key.clone(), coerce_scalar_value(&val));
                    i += 2;
                }
                DslFlagKind::CommaSeparatedList => {
                    let val = tokens.get(i + 1).cloned().unwrap_or_default();
                    let arr: Vec<Value> =
                        val.split(',').map(|s| json!(s.trim())).collect();
                    config.insert(dsl_flag.config_key.clone(), Value::Array(arr));
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
            }
        } else {
            i += 1;
        }
    }

    Ok((Value::Object(config), body))
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
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    match e.get_mut() {
                        Value::Array(arr) => arr.push(new_val),
                        existing => {
                            let prev = existing.clone();
                            *existing = Value::Array(vec![prev, new_val]);
                        }
                    }
                }
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

/// Parse `register <file_rel_path> [--title t] [--as-json] <body>`
fn parse_register(tokens: &[String], cmd: &str) -> DslVerb {
    let file_rel_path = tokens.get(1).cloned().unwrap_or_default();
    let mut title = String::new();
    let mut as_json = false;
    let mut i = 2;

    while i < tokens.len() {
        match tokens[i].as_str() {
            "--title" => {
                title = tokens.get(i + 1).cloned().unwrap_or_default();
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
    let body = match find_first_pipe_in_raw(cmd) {
        Some(pos) => cmd[pos..].to_string(),
        None => cmd.find('[').map(|pos| cmd[pos..].to_string()).unwrap_or_default(),
    };

    DslVerb::Register { file_rel_path, title, as_json, body }
}

/// Parse `patch pipeline <file_rel_path> node <id> [flags] [-- body]`
fn parse_patch(tokens: &[String], cmd: &str) -> DslVerb {
    let file_rel_path = tokens.get(2).cloned().unwrap_or_default();
    let node_id = tokens.get(4).cloned().unwrap_or_default();
    let flag_tokens = if tokens.len() > 5 { tokens[5..].to_vec() } else { vec![] };
    let (flags, body) = parse_flags_for_patch(&flag_tokens, cmd);
    DslVerb::Patch { file_rel_path, node_id, flags, body }
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
        return DslVerb::Unknown { raw: cmd.to_string() };
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
            DslVerb::Describe { kind, name }
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
            let file_rel_path = tokens.get(2).cloned().unwrap_or_default();
            DslVerb::Activate { file_rel_path }
        }
        "deactivate" => {
            let file_rel_path = tokens.get(2).cloned().unwrap_or_default();
            DslVerb::Deactivate { file_rel_path }
        }
        "execute" | "exec" => {
            let file_rel_path = tokens.get(2).cloned().unwrap_or_default();
            let input_str = extract_flag(&tokens, "--input").unwrap_or_default();
            let input = serde_json::from_str(&input_str).unwrap_or(json!({}));
            DslVerb::Execute { file_rel_path, input }
        }
        "register" | "reg" => parse_register(&tokens, cmd),
        "patch" => parse_patch(&tokens, cmd),
        "run" => {
            let dry_run = tokens.iter().any(|t| t == "--dry-run");
            // Extract body from raw string to preserve quoted values.
            let body = match find_first_pipe_in_raw(cmd) {
                Some(pos) => cmd[pos..].to_string(),
                None => String::new(),
            };
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
    let body = body.trim();
    if body.is_empty() {
        return Err("Pipeline body is empty".to_string());
    }
    // Detect graph mode: body contains `[` and `] ->` (with space before arrow)
    if body.contains('[') && body.contains("] ->") {
        build_graph_mode(id, body)
    } else {
        build_pipe_mode(id, body)
    }
}

/// Split a pipe-notation body into segments, ignoring `|` inside single/double quotes.
fn split_pipe_segments(body: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut in_single = false;
    let mut in_double = false;

    for (byte_pos, ch) in body.char_indices() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '|' if !in_single && !in_double => {
                let seg = body[start..byte_pos].trim();
                if !seg.is_empty() {
                    segments.push(seg);
                }
                start = byte_pos + ch.len_utf8();
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

/// Build pipeline from graph notation: `[label] node_kind --flags...\n[from] -> [to]`
fn build_graph_mode(id: &str, body: &str) -> Result<PipelineGraph, String> {
    let mut nodes: Vec<PipelineNode> = Vec::new();
    let mut edges: Vec<PipelineEdge> = Vec::new();

    // Join line continuations then split on newlines
    let joined = body.replace("\\\n", " ").replace("\\\r\n", " ");
    let lines: Vec<&str> = joined
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    for line in &lines {
        if !line.starts_with('[') {
            continue;
        }
        if line.contains("->") {
            // Edge declaration: [from]:pin -> [to]:pin  or  [from] -> [to]
            parse_graph_edge(line, &mut edges)?;
        } else {
            // Node declaration: [label] node_kind --flags...
            parse_graph_node(line, &mut nodes)?;
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

    Ok(PipelineGraph {
        kind: "zebflow.pipeline".to_string(),
        version: "0.1".to_string(),
        id: id.to_string(),
        entry_nodes,
        nodes,
        edges,
    })
}

fn parse_graph_node(line: &str, nodes: &mut Vec<PipelineNode>) -> Result<(), String> {
    let rest = line.strip_prefix('[').ok_or("expected '[' at start of node declaration")?;
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
    let full_kind =
        expand_kind(raw_kind).ok_or_else(|| format!("Unknown node kind: '{raw_kind}'"))?;
    let (input_pins, mut output_pins) = default_pins(full_kind);
    let all_defs = builtin_node_definitions();
    let dsl_flags = all_defs
        .iter()
        .find(|d| d.kind == full_kind)
        .map(|d| d.dsl_flags.as_slice())
        .unwrap_or(&[]);
    let (mut config, body_val) = parse_node_config(&tokens[1..], rest, dsl_flags)?;
    if let Some(bval) = body_val {
        let body_key = match full_kind {
            "n.pg.query" => "query",
            "n.script" => "source",
            "n.logic.switch" | "n.logic.if" | "n.logic.branch" => "expression",
            "n.sekejap.query" | "n.sekejap.mutate" => "sql",
            _ => "body",
        };
        if let Value::Object(ref mut map) = config {
            map.insert(body_key.to_string(), json!(bval));
        }
    }
    // For logic.switch, output pins are dynamic: the declared cases + the default pin.
    if full_kind == "n.logic.switch" {
        if let Value::Object(ref map) = config {
            let cases: Vec<String> = map.get("cases")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let default_pin = map.get("default")
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
            DslFlagKind::Scalar => {
                let s = match val {
                    Value::String(s) if !s.is_empty() => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => continue,
                };
                parts.push(flag.flag.clone());
                // Quote values containing spaces.
                if s.contains(' ') {
                    parts.push(format!("\"{}\"", s.replace('"', "\\\"")));
                } else {
                    parts.push(s);
                }
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
        }
    }

    // Body (SQL / script source / generic body) — stored under a kind-specific key.
    let body_key = match node.kind.as_str() {
        "n.pg.query" => "query",
        "n.script" => "source",
        "n.logic.switch" | "n.logic.if" | "n.logic.branch" => "expression",
        "n.sekejap.query" | "n.sjtable.query" | "n.sekejap.mutate" | "n.sjtable.mutate" => "sql",
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
fn build_pipe_mode(id: &str, body: &str) -> Result<PipelineGraph, String> {
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

    let all_defs = builtin_node_definitions();

    for (idx, segment) in segments.iter().enumerate() {
        let seg_tokens = tokenize(segment);
        if seg_tokens.is_empty() {
            continue;
        }

        let raw_kind = &seg_tokens[0];
        let full_kind = expand_kind(raw_kind)
            .ok_or_else(|| format!("Unknown node kind: '{raw_kind}'"))?;

        let node_id = format!("n{idx}");
        let (input_pins, mut output_pins) = default_pins(full_kind);
        let dsl_flags = all_defs
            .iter()
            .find(|d| d.kind == full_kind)
            .map(|d| d.dsl_flags.as_slice())
            .unwrap_or(&[]);
        let (mut config, body_val) = parse_node_config(&seg_tokens[1..], segment, dsl_flags)?;

        // Set body using kind-appropriate key
        if let Some(bval) = body_val {
            let body_key = match full_kind {
                "n.pg.query" => "query",
                "n.script" => "source",
                "n.sekejap.query" | "n.sekejap.mutate" => "sql",
                _ => "body",
            };
            if let Value::Object(ref mut map) = config {
                map.insert(body_key.to_string(), json!(bval));
            }
        }

        // For logic.switch, output pins are dynamic: the declared cases + the default pin.
        if full_kind == "n.logic.switch" {
            if let Value::Object(ref map) = config {
                let cases: Vec<String> = map.get("cases")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default();
                let default_pin = map.get("default")
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

    let entry_nodes = nodes.first().map(|n| vec![n.id.clone()]).unwrap_or_default();

    Ok(PipelineGraph {
        kind: "zebflow.pipeline".to_string(),
        version: "0.1".to_string(),
        id: id.to_string(),
        entry_nodes,
        nodes,
        edges,
    })
}

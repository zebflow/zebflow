//! Pure parser for the Pipeline DSL command language.
//!
//! No I/O, no platform imports — only strings in, `DslVerb` out.

use std::collections::HashMap;

use serde_json::{Value, json};

use crate::pipeline::model::{PipelineEdge, PipelineGraph, PipelineNode};

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
    /// `activate pipeline <name>`
    Activate { name: String },
    /// `deactivate pipeline <name>`
    Deactivate { name: String },
    /// `execute pipeline <name> [--input <json>]`
    Execute { name: String, input: Value },
    /// `register <name> [--path <p>] [--title <t>] [--as-json] [| ...]`
    Register {
        name: String,
        path: String,
        title: String,
        as_json: bool,
        body: String,
    },
    /// `patch pipeline <name> node <id> [flags...]`
    Patch {
        name: String,
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
            ' ' | '\t' if !in_single && !in_double => {
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
        "web.render" | "n.web.render" => Some("n.web.render"),
        "http.request" | "n.http.request" => Some("n.http.request"),
        "sjtable.query" | "n.sjtable.query" => Some("n.sjtable.query"),
        "fanout" | "n.fanout" | "logic.branch" | "n.logic.branch" => Some("n.logic.branch"),
        "zebtune" | "n.zebtune" => Some("n.zebtune"),
        "logic.if" | "n.logic.if" => Some("n.logic.if"),
        "logic.switch" | "n.logic.switch" => Some("n.logic.switch"),
        "logic.merge" | "n.logic.merge" => Some("n.logic.merge"),
        _ => None,
    }
}

/// Default input/output pins per node kind.
pub fn default_pins(kind: &str) -> (Vec<String>, Vec<String>) {
    match kind {
        "n.trigger.webhook" | "n.trigger.schedule" | "n.trigger.manual" => {
            (vec![], vec!["out".to_string()])
        }
        "n.pg.query" | "n.sjtable.query" | "n.script" | "n.http.request"
        | "n.zebtune" | "n.logic.if" | "n.logic.switch" | "n.logic.branch"
        | "n.logic.merge" => {
            (vec!["in".to_string()], vec!["out".to_string()])
        }
        "n.web.render" => (vec!["in".to_string()], vec![]),
        _ => (vec!["in".to_string()], vec!["out".to_string()]),
    }
}

/// Strips matching outer `"..."` or `'...'` from a string.
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

/// Parse flag→config key mapping and body from token list after node kind.
pub fn parse_node_config(tokens: &[String], raw: &str) -> (Value, Option<String>) {
    let mut config = serde_json::Map::new();
    let mut body: Option<String> = None;
    let mut i = 0;

    while i < tokens.len() {
        let t = &tokens[i];

        if t == "--" {
            // Extract body from the raw segment string to preserve quotes/whitespace.
            body = extract_raw_body_from(raw);
            break;
        }

        if let Some(key) = t.strip_prefix("--") {
            let val = tokens.get(i + 1).cloned().unwrap_or_default();
            match key {
                "path" => { config.insert("path".to_string(), json!(val)); i += 2; }
                "method" => { config.insert("method".to_string(), json!(val)); i += 2; }
                "cron" => { config.insert("cron".to_string(), json!(val)); i += 2; }
                "timezone" => { config.insert("timezone".to_string(), json!(val)); i += 2; }
                "template" => { config.insert("template".to_string(), json!(val)); i += 2; }
                "route" => { config.insert("route".to_string(), json!(val)); i += 2; }
                "credential" => { config.insert("credential_id".to_string(), json!(val)); i += 2; }
                "url" => { config.insert("url".to_string(), json!(val)); i += 2; }
                "lang" => { config.insert("language".to_string(), json!(val)); i += 2; }
                "table" => { config.insert("table".to_string(), json!(val)); i += 2; }
                "op" => { config.insert("operation".to_string(), json!(val)); i += 2; }
                "expr" => { config.insert("expression".to_string(), json!(val)); i += 2; }
                "cases" => {
                    let arr: Vec<Value> = val.split(',').map(|s| json!(s.trim())).collect();
                    config.insert("cases".to_string(), Value::Array(arr));
                    i += 2;
                }
                "default" => { config.insert("default".to_string(), json!(val)); i += 2; }
                "fanout" => {
                    let branches: Vec<Value> = val.split(',').map(|s| json!(s.trim())).collect();
                    config.insert("branches".to_string(), Value::Array(branches));
                    config.insert("mode".to_string(), json!("fanout"));
                    i += 2;
                }
                "strategy" => { config.insert("strategy".to_string(), json!(val)); i += 2; }
                "budget" => { config.insert("step_budget".to_string(), json!(val)); i += 2; }
                _ => { i += 1; }
            }
        } else {
            i += 1;
        }
    }

    (Value::Object(config), body)
}

/// Parse `register <name> [--path p] [--title t] [--as-json] <body>`
fn parse_register(tokens: &[String]) -> DslVerb {
    let name = tokens.get(1).cloned().unwrap_or_default();
    let mut path = "/".to_string();
    let mut title = String::new();
    let mut as_json = false;
    let mut body_start = tokens.len();
    let mut i = 2;

    while i < tokens.len() {
        match tokens[i].as_str() {
            "--path" => {
                path = tokens.get(i + 1).cloned().unwrap_or("/".to_string());
                i += 2;
            }
            "--title" => {
                title = tokens.get(i + 1).cloned().unwrap_or_default();
                i += 2;
            }
            "--as-json" => {
                as_json = true;
                i += 1;
            }
            _ => {
                body_start = i;
                break;
            }
        }
    }

    let body = if body_start < tokens.len() {
        tokens[body_start..].join(" ")
    } else {
        String::new()
    };

    DslVerb::Register { name, path, title, as_json, body }
}

/// Parse `patch pipeline <name> node <id> [flags] [-- body]`
fn parse_patch(tokens: &[String], cmd: &str) -> DslVerb {
    let name = tokens.get(2).cloned().unwrap_or_default();
    let node_id = tokens.get(4).cloned().unwrap_or_default();
    let flag_tokens = if tokens.len() > 5 { tokens[5..].to_vec() } else { vec![] };
    let (flags_val, body) = parse_node_config(&flag_tokens, cmd);
    let flags: HashMap<String, Value> = if let Value::Object(map) = flags_val {
        map.into_iter().collect()
    } else {
        HashMap::new()
    };
    DslVerb::Patch { name, node_id, flags, body }
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
            let name = tokens.get(2).cloned().unwrap_or_default();
            DslVerb::Activate { name }
        }
        "deactivate" => {
            let name = tokens.get(2).cloned().unwrap_or_default();
            DslVerb::Deactivate { name }
        }
        "execute" | "exec" => {
            let name = tokens.get(2).cloned().unwrap_or_default();
            let input_str = extract_flag(&tokens, "--input").unwrap_or_default();
            let input = serde_json::from_str(&input_str).unwrap_or(json!({}));
            DslVerb::Execute { name, input }
        }
        "register" | "reg" => parse_register(&tokens),
        "patch" => parse_patch(&tokens, cmd),
        "run" => {
            let dry_run = tokens.iter().any(|t| t == "--dry-run");
            // body is everything after the first `|`
            let pipe_pos = tokens.iter().position(|t| t == "|").unwrap_or(tokens.len());
            let body = if pipe_pos < tokens.len() {
                tokens[pipe_pos..].join(" ")
            } else {
                String::new()
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

/// Build a `PipelineGraph` from pipe (`|`) or graph (`[id]->`) notation.
pub fn build_pipeline_graph(id: &str, body: &str) -> Result<PipelineGraph, String> {
    let body = body.trim();
    if body.is_empty() {
        return Err("Pipeline body is empty".to_string());
    }
    // Detect mode by presence of graph notation
    if body.contains("[") && body.contains("]->") {
        build_pipe_mode(id, body) // fallback: graph mode uses same parse for now
    } else {
        build_pipe_mode(id, body)
    }
}

/// Build pipeline from pipe-notation: `trigger.webhook --path /test | pg.query --credential main`
fn build_pipe_mode(id: &str, body: &str) -> Result<PipelineGraph, String> {
    // Strip leading `|` if present
    let body = body.trim_start_matches('|').trim();
    let segments: Vec<&str> = body.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

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
        let full_kind = expand_kind(raw_kind)
            .ok_or_else(|| format!("Unknown node kind: '{raw_kind}'"))?;

        let node_id = format!("n{idx}");
        let (input_pins, output_pins) = default_pins(full_kind);
        let (mut config, body_val) = parse_node_config(&seg_tokens[1..], segment);

        // Set body using kind-appropriate key
        if let Some(bval) = body_val {
            let body_key = match full_kind {
                "n.pg.query" => "query",
                "n.script" => "source",
                _ => "body",
            };
            if let Value::Object(ref mut map) = config {
                map.insert(body_key.to_string(), json!(bval));
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

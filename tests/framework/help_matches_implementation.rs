//! Help describes the implementation. It does not define it.
//!
//! `pipeline/nodes` is generated from `builtin_node_definitions()` and cannot
//! lie. The 55 hand-written files can say anything, and nothing checked them —
//! so `__status`, `__redirect`, `__body` and `__notfound` were taught as
//! "special script output keys" across ten files, with zero lines of
//! implementation anywhere.
//!
//! That is not a stale document. It was never connected to anything. And the
//! help is compiled into the binary and fed to the assistant *in full* through
//! `format_for_system_prompt()`, so the agent that writes pipelines for users
//! was instructed to emit guards that silently do nothing.
//!
//! This test is the connection: a marker the help teaches must exist in the
//! code, or the build fails naming both.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_tree(root: &Path, ext: &[&str], skip: &[&str]) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let display = path.to_string_lossy().to_string();
            if skip.iter().any(|s| display.contains(s)) {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| ext.contains(&e))
            {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    out.push((path, text));
                }
            }
        }
    }
    out
}

/// Every `__marker` the help teaches must appear somewhere in the code.
///
/// A marker is a promise to the reader — and to the assistant, which receives
/// every one of these files as instructions — that returning that key from a
/// script does something. If no code mentions it, the promise is false.
#[test]
fn every_marker_the_help_teaches_exists_in_the_code() {
    let root = repo_root();
    let help = read_tree(&root.join("src/platform/help"), &["md"], &[]);
    assert!(!help.is_empty(), "no help files found");

    // Everything the code knows about, from Rust, JS and TSX alike.
    let code: String = read_tree(
        &root.join("src"),
        &["rs", "js", "tsx", "ts"],
        &["src/platform/help"],
    )
    .into_iter()
    .map(|(_, text)| text)
    .collect::<Vec<_>>()
    .join("\n");

    let mut phantoms: BTreeSet<String> = BTreeSet::new();
    for (path, text) in &help {
        let bytes = text.as_bytes();
        let mut i = 0;
        while let Some(pos) = text[i..].find("__") {
            let start = i + pos;
            let mut end = start + 2;
            while end < bytes.len()
                && (bytes[end].is_ascii_lowercase() || bytes[end] == b'_' || bytes[end].is_ascii_digit())
            {
                end += 1;
            }
            let marker = &text[start..end];
            // Two underscores and at least three more characters: a marker, not
            // markdown emphasis or a rule of dashes.
            if marker.len() > 4 && !code.contains(marker) {
                phantoms.insert(format!(
                    "{marker}  ({})",
                    path.strip_prefix(&root).unwrap_or(path).display()
                ));
            }
            i = end.max(start + 2);
        }
    }

    assert!(
        phantoms.is_empty(),
        "help teaches markers that no code implements — either build them or \
         stop teaching them:\n  {}",
        phantoms.into_iter().collect::<Vec<_>>().join("\n  ")
    );
}

/// Every pipeline the help writes out in full must build.
///
/// A ```zf block that begins with `register` is not an illustration, it is the
/// thing the reader will paste. It goes through the same parser the API uses:
/// short kind → full kind, every flag against the node's definition, every
/// `[a]:pin -> [b]` against the node's real output pins. A flag that was
/// renamed, a pin that never existed, a node kind spelt from memory — each
/// fails here naming the file and the parser's own message.
///
/// Fragments (`| trigger.webhook …` without `register`, prose in parentheses)
//// Every pipeline the help writes down must build.
///
/// Every fenced block, whatever its language tag, is scanned for `register`
/// and `run` commands and for bare pipe-mode bodies, and each is fed to the
/// real parser and graph builder. Unknown nodes, undeclared flags (`--params`
/// on a node that has none, `--route`, `--table`), unquoted expressions and
/// dangling edges all fail here, naming the file. Twelve example recipes
/// carried flags no node had declared for weeks because only ```` ```zf ````
/// `register` blocks were checked; prose tables and plain fences were not.
///
/// Fences containing `…` or a `<placeholder>` are illustrations and are left
/// alone; a line that is only a comment (`# …`) is skipped.
#[test]
fn every_pipeline_the_help_registers_builds() {
    use zebflow::platform::shell::parser::{DslVerb, build_pipeline_graph, parse_one_command, split_commands};

    let root = repo_root();
    let mut docs = read_tree(&root.join("src/platform/help"), &["md"], &[]);
    docs.extend(read_tree(&root.join("blessed/skills"), &["md"], &[]));
    docs.extend(read_tree(&root.join("blessed/skill-extras"), &["md"], &[]));

    let is_illustration = |body: &str| body.contains('…') || body.contains("...") || body.contains('<') && body.contains('>');

    let mut failures = Vec::new();
    let mut built = 0usize;
    for (path, text) in &docs {
        let rel = path.strip_prefix(&root).unwrap_or(path).display().to_string();
        let mut in_block = false;
        let mut block = String::new();
        for line in text.lines() {
            if line.trim_start().starts_with("```") {
                if in_block {
                    let body_lines: Vec<&str> = block
                        .lines()
                        .filter(|l| !l.trim_start().starts_with('#'))
                        .collect();
                    let block_text = body_lines.join("\n");
                    let first = block_text.trim_start().to_lowercase();
                    let candidate = first.starts_with("register ")
                        || first.starts_with("run ")
                        || first.starts_with('|');
                    if candidate && !is_illustration(&block_text) {
                        let commands = if first.starts_with('|') {
                            vec![format!("run {block_text}")]
                        } else {
                            split_commands(&block_text)
                        };
                        for cmd in commands {
                            let lower = cmd.trim_start().to_lowercase();
                            let (id, body) = match parse_one_command(&cmd) {
                                DslVerb::Register { file_rel_path, body, .. } => (file_rel_path, body),
                                DslVerb::Run { body, .. } => ("run".to_string(), body),
                                _ if lower.starts_with("register ") || lower.starts_with("run ") => {
                                    failures.push(format!("{rel}: did not parse as register/run: {}", cmd.lines().next().unwrap_or("")));
                                    continue;
                                }
                                _ => continue,
                            };
                            if body.trim().is_empty() {
                                continue;
                            }
                            match build_pipeline_graph(&id, &body) {
                                Ok(_) => built += 1,
                                Err(err) => failures.push(format!("{rel}: {id}: {err}")),
                            }
                        }
                    }
                    block.clear();
                    in_block = false;
                } else {
                    in_block = true;
                }
                continue;
            }
            if in_block {
                block.push_str(line);
                block.push('\n');
            }
        }
    }

    assert!(built > 0, "no pipeline blocks found in help — the scan is broken");
    assert!(
        failures.is_empty(),
        "help writes pipelines the parser refuses — fix the help or the parser:\n  {}",
        failures.join("\n  ")
    );
}

/// Every MCP tool the help names exists in the handler.
///
/// `platform/agent.md` is the instructions text every MCP client receives, and
/// the repository skill points agents at the same tools. For weeks they taught
/// `template_write`, `docs_project_read`, `run_db_query` and
/// `execute_pipeline_dsl` — none of which the handler had. An agent that calls
/// a tool from the docs and gets "unknown tool" back has no way to tell a typo
/// from a lie; this makes it a build failure instead.
#[test]
fn every_mcp_tool_the_help_names_exists() {
    let root = repo_root();
    let handler = std::fs::read_to_string(root.join("src/platform/mcp/handler.rs")).expect("handler");
    let mut tools = BTreeSet::new();
    let mut rest = handler.as_str();
    while let Some(at) = rest.find("#[tool(") {
        rest = &rest[at..];
        let Some(fn_at) = rest.find("async fn ") else { break };
        let name: String = rest[fn_at + 9..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        tools.insert(name);
        rest = &rest[fn_at + 9..];
    }
    assert!(tools.contains("start_here") && tools.contains("pipeline_register"), "tool scan is broken: {tools:?}");

    let domains = ["pipeline_", "file_", "template_", "docs_", "connection_", "credential_", "help_", "list_ui", "install_ui", "git_", "move_", "start_", "run_", "read_", "write_", "list_", "describe_"];
    // Field, flag and event names that share a prefix with a tool domain but
    // are not tools: `start_url` is a web-manifest field; `run_start` and
    // `run_done` are the SSE lifecycle kinds a run streams.
    let not_tools = ["file_rel_path", "credential_id", "pipeline_node_timeout_secs", "git_name", "git_email", "pipeline_bundle", "template_bundle", "list_style", "file_ref", "read_only", "execute_async", "start_url", "run_start", "run_done"];

    let mut docs = read_tree(&root.join("src/platform/help"), &["md"], &[]);
    docs.extend(read_tree(&root.join("skills"), &["md"], &[]));
    docs.extend(read_tree(&root.join("blessed/skills"), &["md"], &[]));
    docs.extend(read_tree(&root.join("blessed/skill-extras"), &["md"], &[]));
    let mut failures = Vec::new();
    for (path, text) in &docs {
        let rel = path.strip_prefix(&root).unwrap_or(path).display().to_string();
        for (line_no, line) in text.lines().enumerate() {
            let mut cursor = line;
            while let Some(open) = cursor.find('`') {
                let after = &cursor[open + 1..];
                let Some(close) = after.find('`') else { break };
                let token = &after[..close];
                cursor = &after[close + 1..];
                let name: &str = token
                    .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .next()
                    .unwrap_or("");
                if name.is_empty() || !name.contains('_') || name.contains(char::is_uppercase) {
                    continue;
                }
                if !domains.iter().any(|d| name.starts_with(d)) || not_tools.contains(&name) {
                    continue;
                }
                // Only a bare identifier, or one followed by a call/argument, reads as a tool.
                let next = token[name.len()..].chars().next();
                if !matches!(next, None | Some(' ') | Some('(')) {
                    continue;
                }
                if !tools.contains(name) {
                    failures.push(format!("{rel}:{} names `{name}` — no such MCP tool", line_no + 1));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "help names MCP tools the handler does not have:\n  {}",
        failures.join("\n  ")
    );
}

/// Every ui primitive the help imports exists, and exports what the import
/// names. `design-system.md` used to teach `SelectTrigger`, `DialogTrigger`,
/// `{ Card, CardHeader } from "@/components/ui/card"` and a `code-editor` —
/// none of which any file provided. The assistant copied them into pages.
#[test]
fn every_ui_import_the_help_teaches_resolves() {
    let root = repo_root();
    let ui = root.join("src/platform/web/templates/components/ui");
    let help = read_tree(&root.join("src/platform/help"), &["md"], &[]);

    let mut failures = Vec::new();
    for (path, text) in &help {
        let rel = path.strip_prefix(&root).unwrap_or(path).display().to_string();
        for line in text.lines() {
            let Some(rest) = line.trim_start().strip_prefix("import ") else { continue };
            let Some((names, from)) = rest.split_once(" from ") else { continue };
            // `from "@/components/ui/button";   // a trailing comment` — take the quoted part.
            let quoted = from.split('"').nth(1).unwrap_or("");
            let Some(module) = quoted.strip_prefix("@/components/ui/") else { continue };
            let file = ui.join(format!("{module}.tsx"));
            let Ok(source) = std::fs::read_to_string(&file) else {
                failures.push(format!("{rel}: `@/components/ui/{module}` — no such file"));
                continue;
            };
            let names = names.trim();
            if let Some(list) = names.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
                for name in list.split(',').map(str::trim).filter(|n| !n.is_empty()) {
                    let name = name.split_whitespace().next().unwrap_or(name);
                    let exported = source.contains(&format!("export function {name}"))
                        || source.contains(&format!("export const {name}"))
                        || source.contains(&format!("export default function {name}"))
                        || source.contains(&format!("export {{ {name}"))
                        || source.contains(&format!(", {name} }}"))
                        || source.contains(&format!(", {name},"));
                    if !exported {
                        failures.push(format!("{rel}: `{name}` is not exported by ui/{module}.tsx"));
                    }
                }
            } else if !source.contains("export default") {
                failures.push(format!("{rel}: ui/{module}.tsx has no default export for `{names}`"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "help imports ui primitives that do not exist as written:\n  {}",
        failures.join("\n  ")
    );
}

/// Every name the help imports from `zeb/react` is one `zeb/react` exports.
/// `tv()` was taught as "a global" in four files; nothing installed it.
#[test]
fn every_zeb_react_import_the_help_teaches_is_exported() {
    let root = repo_root();
    let help = read_tree(&root.join("src/platform/help"), &["md"], &[]);
    let exports = zebflow::rwe::core::zeb_react::EXPORTS;
    let mut failures = Vec::new();
    for (path, text) in &help {
        let rel = path.strip_prefix(&root).unwrap_or(path).display().to_string();
        for line in text.lines() {
            let Some(rest) = line.trim_start().strip_prefix("import ") else { continue };
            let Some((names, from)) = rest.split_once(" from ") else { continue };
            if from.split('"').nth(1) != Some("zeb/react") {
                continue;
            }
            let Some(list) = names.trim().strip_prefix('{').and_then(|s| s.strip_suffix('}')) else { continue };
            for name in list.split(',').map(str::trim).filter(|n| !n.is_empty()) {
                let name = name.split_whitespace().next().unwrap_or(name);
                if !exports.contains(&name) {
                    failures.push(format!("{rel}: `{name}` is not exported by zeb/react"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "help imports names zeb/react does not export:\n  {}", failures.join("\n  "));
}

/// Every node definition carries what a reader needs to use it without
/// opening the source: a description that says what comes out, an output
/// schema unless it is a trigger, and at least one example whose DSL line
/// builds against the node's own declared flags.
///
/// `help(topic="pipeline/nodes/<kind>")` is generated from the definition and
/// nothing else, so for an agent the definition *is* the node. Two dogfood
/// rounds found every stumble at a node whose definition was one sentence:
/// `--params` unseen, the output shape unstated, the DDL dialect unmentioned.
#[test]
fn every_node_definition_documents_itself_with_a_building_example() {
    use zebflow::platform::shell::parser::build_pipeline_graph;

    let mut failures = Vec::new();
    for def in zebflow::pipeline::nodes::builtin_node_definitions() {
        let kind = def.kind.as_str();
        let is_trigger = def.input_pins.is_empty();
        if def.description.trim().len() < 120 {
            failures.push(format!("{kind}: description is {} chars; say what it needs, what comes out, and the usual mistake", def.description.trim().len()));
        }
        if !is_trigger && def.output_schema.is_null() {
            failures.push(format!("{kind}: no output_schema"));
        }
        if def.examples.is_empty() {
            failures.push(format!("{kind}: no example"));
        }
        for example in &def.examples {
            let dsl = example.dsl.trim();
            if dsl.is_empty() {
                failures.push(format!("{kind}: example `{}` has no dsl", example.title));
                continue;
            }
            let short = kind.strip_prefix("n.").unwrap_or(kind);
            let first = dsl.split_whitespace().next().unwrap_or("");
            if first != short && first != kind {
                failures.push(format!("{kind}: example dsl starts with `{first}`, not the node"));
            }
            let body = if is_trigger { format!("| {dsl}") } else { format!("| trigger.function | {dsl}") };
            if let Err(err) = build_pipeline_graph("example", &body) {
                failures.push(format!("{kind}: example `{}` does not build: {err}", example.title));
            }
        }
    }
    assert!(failures.is_empty(), "node definitions that do not document themselves:\n  {}", failures.join("\n  "));
}

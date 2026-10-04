//! Every guide an agent reads first speaks the node grammar the code has.
//!
//! The node pages are generated from the definitions and cannot drift. The
//! rest — the help pages, every `SKILL.md`, the repository skill's
//! references, the MCP instructions and tool texts, the assistant's tool
//! texts and `start_here` — is written by hand, and a hand-written example
//! outlives the flag it names. These tests read every one of them:
//!
//! - every DSL example names a real kind, only flags that kind declares, and
//!   for a closed choice only a word the flag lists; a complete example also
//!   builds through the real parser;
//! - no text reads an `input.<key>` no node answers any more (`input.rows`
//!   is `input.query.rows`, `input.body` is `input.webhook.body`, …) or
//!   reaches into `$trigger.webhook` (`$trigger` is the envelope itself);
//! - no hand-written page copies a node's flag table — a page embeds
//!   `<!-- node-flags:<kind> -->`, which renders it from the definition.
//!
//! An example that is deliberately partial opts out with
//! `<!-- dsl-lint: skip -->` on the line before its fence. Keep those rare.

use crate::pipeline::NodeDefinition;
use crate::pipeline::model::DslFlagKind;
use crate::platform::shell::parser::{build_pipeline_graph_with_definitions, tokenize};

/// Root keys no 0.11 node answers. Each was an answer, or a merge of the
/// request into the root, before every node answered under its noun.
const RETIRED_KEYS: &[&str] = &[
    "body", "rows", "columns", "row_count", "rows_affected", "saved", "thumbnail", "access_token", "found", "valid", "hash",
];

const SKIP_MARKER: &str = "<!-- dsl-lint: skip -->";

/// One text an agent reads: where it lives, and whether it is markdown
/// (fences and inline code) or a plain string (quoted examples).
pub(super) struct Guide {
    pub name: String,
    pub text: String,
    pub markdown: bool,
}

pub(super) fn guides() -> Vec<Guide> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut out: Vec<Guide> = super::HELP
        .iter()
        .map(|n| Guide { name: format!("src/platform/help/{}.md", n.path), text: n.content.to_string(), markdown: true })
        .collect();
    for dir in ["blessed/skills", "blessed/skill-extras", "skills/zebflow"] {
        let mut stack = vec![root.join(dir)];
        while let Some(path) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&path) else { continue };
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|e| e == "md") {
                    let text = std::fs::read_to_string(&p).unwrap_or_default();
                    let name = p.strip_prefix(root).unwrap_or(&p).display().to_string();
                    out.push(Guide { name, text, markdown: true });
                }
            }
        }
    }
    for (name, text) in crate::platform::mcp::agent_texts() {
        out.push(Guide { name, text, markdown: false });
    }
    for tool in crate::platform::services::assistant_tools::AssistantPlatformTools::tool_defs() {
        let mut text = tool.description.clone();
        collect_descriptions(&tool.parameters, &mut text);
        out.push(Guide { name: format!("assistant tool {}", tool.name), text, markdown: false });
    }
    let count = crate::pipeline::nodes::builtin_node_definitions().len();
    let start_here = crate::platform::services::ops::start_here_way_in(count) + crate::platform::services::ops::START_HERE_LOOP;
    out.push(Guide { name: "start_here".into(), text: start_here, markdown: true });
    let context7: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("context7.json")).unwrap_or_default()).unwrap_or_default();
    let rules = context7["rules"].as_array().map(|r| r.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("\n"));
    out.push(Guide { name: "context7.json rules".into(), text: rules.unwrap_or_default(), markdown: false });
    out
}

fn collect_descriptions(value: &serde_json::Value, out: &mut String) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, v) in map {
                match (key.as_str(), v) {
                    ("description", serde_json::Value::String(text)) => {
                        out.push('\n');
                        out.push_str(text);
                    }
                    _ => collect_descriptions(v, out),
                }
            }
        }
        serde_json::Value::Array(items) => items.iter().for_each(|v| collect_descriptions(v, out)),
        _ => {}
    }
}

/// A DSL example found in a guide: its line, its text, and whether it is a
/// whole body (from a fence or a `body="…"`) that must also build.
pub(super) struct Example {
    pub line: usize,
    pub text: String,
    pub whole: bool,
}

/// `fs.file.put`, `n.fs.save`, `x.acme.invoice.create` — a word shaped like a kind.
fn kind_shaped(word: &str) -> bool {
    let mut parts = word.split('.');
    let first = parts.next().unwrap_or("");
    let rest: Vec<&str> = parts.collect();
    !rest.is_empty()
        && std::iter::once(first).chain(rest.iter().copied()).all(|p| {
            !p.is_empty() && p.starts_with(|c: char| c.is_ascii_lowercase()) && p.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        })
}

/// A word a reader would take for a node kind: a known kind, or a dotted
/// word under a known family or the retired `n.` prefix.
fn names_a_node(word: &str, defs: &[NodeDefinition]) -> bool {
    defs.iter().any(|d| d.kind == word)
        || (kind_shaped(word) && {
            let family = word.split('.').next().unwrap_or("");
            family == "n" || (family != "input" && defs.iter().any(|d| d.kind.split('.').next() == Some(family)))
        })
}

/// Whether a one-line text reads as DSL: `| kind …`, `kind --flag …`, or a
/// console `register … | …`.
fn reads_as_dsl(text: &str, defs: &[NodeDefinition]) -> bool {
    let t = text.trim();
    let mut words = t.trim_start_matches('|').split_whitespace();
    let first = words.next().unwrap_or("");
    let second = words.next().unwrap_or("");
    (t.starts_with('|') && names_a_node(first, defs))
        || (names_a_node(first, defs) && second.starts_with("--"))
        || (t.starts_with("register ") && t.contains(" | "))
}

/// The DSL inside `body="…"` (an MCP call written in prose), unescaped.
fn body_attributes(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(at) = rest.find("body=\"") {
        let after = &rest[at + 6..];
        let mut value = String::new();
        let mut chars = after.chars();
        let mut closed = false;
        while let Some(c) = chars.next() {
            match c {
                '\\' => value.extend(chars.next()),
                '"' => {
                    closed = true;
                    break;
                }
                c => value.push(c),
            }
        }
        if closed && value.trim_start().starts_with('|') {
            out.push(value);
        }
        rest = chars.as_str();
    }
    out
}

/// Spans of a line delimited by backticks, or — in a plain string — by
/// single or double quotes (a `\"` inside double quotes is kept).
fn delimited_spans(line: &str, quotes: bool) -> Vec<String> {
    let mut spans = Vec::new();
    let delimiters: &[char] = if quotes { &['`', '\'', '"'] } else { &['`'] };
    for &d in delimiters {
        let mut open: Option<String> = None;
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            match (&mut open, c) {
                (Some(span), '\\') if d == '"' => {
                    if let Some(next) = chars.next() {
                        span.push(next);
                    }
                }
                (Some(span), c) if c == d => {
                    spans.push(std::mem::take(span));
                    open = None;
                }
                (Some(span), c) => span.push(c),
                (None, c) if c == d => open = Some(String::new()),
                (None, _) => {}
            }
        }
    }
    spans
}

/// Every DSL example in one guide.
pub(super) fn examples(guide: &Guide, defs: &[NodeDefinition]) -> Vec<Example> {
    let mut out = Vec::new();
    let lines: Vec<&str> = guide.text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if guide.markdown && line.trim_start().starts_with("```") {
            let skip = i > 0 && lines[i - 1].contains(SKIP_MARKER);
            let start = i + 1;
            let mut end = start;
            while end < lines.len() && !lines[end].trim_start().starts_with("```") {
                end += 1;
            }
            if !skip {
                fence_examples(&lines[start..end.min(lines.len())], start + 1, defs, &mut out);
            }
            i = end + 1;
            continue;
        }
        for body in body_attributes(line) {
            out.push(Example { line: i + 1, text: body, whole: true });
        }
        for span in delimited_spans(line, !guide.markdown) {
            if reads_as_dsl(&span, defs) && !span.contains("body=\"") {
                out.push(Example { line: i + 1, text: span, whole: false });
            }
        }
        i += 1;
    }
    out
}

/// The examples in one fence: a run of `|` lines (with indented
/// continuations) is one pipe body, a run of `[id] …` lines and edges is one
/// graph body, and a line that starts with a kind and a flag is one node.
fn fence_examples(block: &[&str], first_line: usize, defs: &[NodeDefinition], out: &mut Vec<Example>) {
    let mut current: Option<(usize, String, bool)> = None; // (line, text, graph)
    let flush = |current: &mut Option<(usize, String, bool)>, out: &mut Vec<Example>| {
        if let Some((line, text, _)) = current.take() {
            out.push(Example { line, text, whole: true });
        }
    };
    for (offset, raw) in block.iter().enumerate() {
        let line_no = first_line + offset;
        let line = raw.trim();
        for body in body_attributes(raw) {
            out.push(Example { line: line_no, text: body, whole: true });
        }
        let graph_line = graph_label(line).is_some();
        if line.starts_with('|') || graph_line {
            let graph = graph_line;
            match &mut current {
                Some((_, text, g)) if *g == graph => {
                    text.push('\n');
                    text.push_str(line);
                }
                _ => {
                    flush(&mut current, out);
                    current = Some((line_no, line.to_string(), graph));
                }
            }
            continue;
        }
        // A graph body runs over blank lines and over the lines of a quoted
        // multi-line body; a pipe body takes indented lines.
        if let Some((_, text, true)) = &mut current {
            let open_quote = text.chars().filter(|c| *c == '"').count() % 2 == 1;
            if line.is_empty() || open_quote {
                text.push('\n');
                text.push_str(raw);
                continue;
            }
        }
        let continuation = (raw.starts_with(' ') || raw.starts_with('\t')) && !line.is_empty();
        if let (true, Some((_, text, false))) = (continuation, &mut current) {
            text.push(' ');
            text.push_str(line);
            continue;
        }
        flush(&mut current, out);
        if let Some(rest) = line.strip_prefix("register ") {
            if let Some(at) = rest.find(" | ") {
                out.push(Example { line: line_no, text: rest[at + 1..].to_string(), whole: true });
            }
        } else if reads_as_dsl(line, defs) && !line.starts_with('|') {
            out.push(Example { line: line_no, text: format!("| {line}"), whole: false });
        }
    }
    flush(&mut current, out);
}

/// `[id]` at the start of a graph-mode line, and the rest after it.
fn graph_label(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix('[')?;
    let (label, after) = rest.split_once(']')?;
    let is_label = !label.is_empty() && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    (is_label && (after.is_empty() || after.starts_with([' ', ':']))).then_some(after)
}

/// The node statements of an example: graph lines as they are (edges and
/// notes dropped), a pipe body split at each `|` that starts a node. A
/// console `register <path> … | …` line starts at its first node.
fn statements(text: &str, defs: &[NodeDefinition]) -> Vec<String> {
    let text = match text.strip_prefix("register ").and_then(|rest| rest.find(" | ").map(|at| &rest[at + 1..])) {
        Some(body) => body,
        None => text,
    };
    if text.lines().any(|l| graph_label(l).is_some()) {
        return text
            .lines()
            .filter_map(graph_label)
            .map(str::trim)
            .filter(|rest| !rest.starts_with("->") && !rest.starts_with(':'))
            .map(str::to_string)
            .collect();
    }
    let mut out = Vec::new();
    let mut current = String::new();
    let (mut single, mut double, mut in_body) = (false, false, false);
    let chars: Vec<char> = text.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '\'' if !double && !in_body => single = !single,
            '"' if !single && !in_body => double = !double,
            '-' if !single && !double && !in_body && chars.get(i + 1) == Some(&'-')
                && chars.get(i + 2).is_none_or(|n| n.is_whitespace() || *n == '"')
                && (i == 0 || chars[i - 1].is_whitespace()) =>
            {
                in_body = true;
            }
            '|' if !single && !double && chars.get(i + 1) != Some(&'|') && (i == 0 || chars[i - 1] != '|') => {
                let next: String = chars[i + 1..].iter().collect();
                let word = next.split_whitespace().next().unwrap_or("");
                if !in_body || names_a_node(word, defs) || word == "note" {
                    if !current.trim().is_empty() {
                        out.push(current.trim().to_string());
                    }
                    current.clear();
                    in_body = false;
                    continue;
                }
            }
            _ => {}
        }
        current.push(c);
    }
    if !current.trim().is_empty() {
        out.push(current.trim().to_string());
    }
    out
}

fn is_sketch(text: &str) -> bool {
    text.contains('…') || text.contains("...") || text.contains('→') || (text.contains('<') && text.contains('>'))
}

/// What is wrong with one example: an unknown kind, a flag the kind does not
/// declare, a choice word it does not list; for a complete example, the
/// parser's own refusal.
pub(super) fn check_example(example: &Example, defs: &[NodeDefinition]) -> Vec<String> {
    let mut problems = Vec::new();
    for statement in statements(&example.text, defs) {
        let head = match statement.find(" -- ").or_else(|| statement.strip_suffix(" --").map(|s| s.len())) {
            Some(at) => &statement[..at],
            None => statement.as_str(),
        };
        let tokens = tokenize(head);
        let Some(kind) = tokens.first() else { continue };
        if kind == "note" || kind.starts_with("x.") || is_sketch(kind) {
            continue;
        }
        let Some(def) = defs.iter().find(|d| &d.kind == kind) else {
            problems.push(format!("unknown kind `{kind}`"));
            continue;
        };
        for (i, token) in tokens.iter().enumerate().skip(1) {
            if !token.starts_with("--") || token.len() < 3 {
                continue;
            }
            let name = token.split('=').next().unwrap_or(token);
            let Some(flag) = def.dsl_flags.iter().find(|f| f.flag == name) else {
                problems.push(format!("`{kind}` declares no `{name}`"));
                continue;
            };
            let closed = !flag.choices.is_empty() && matches!(flag.kind, DslFlagKind::Scalar | DslFlagKind::RepeatedList);
            if let (true, Some(value)) = (closed, tokens.get(i + 1)) {
                let literal = !value.starts_with("--") && !value.contains("{{") && !is_sketch(value) && !value.contains('|');
                if literal && !flag.choices.iter().any(|c| c == value) {
                    problems.push(format!("`{kind} {name} {value}` — `{name}` takes {}", flag.choices.join("|")));
                }
            }
        }
    }
    if problems.is_empty() && example.whole && !is_sketch(&example.text) {
        match build_pipeline_graph_with_definitions("guide-lint", &example.text, defs) {
            Err(err) => problems.push(format!("does not build: {err}")),
            // The flow activation refuses (`node-conventions.md` §4): a
            // stray cycle, a loop wired across its boundary, a `$nodes`
            // reference to a node not upstream. Only a whole pipeline — one
            // whose text writes its trigger — is judged; a fragment reads
            // nodes its text leaves out.
            Ok(graph) if writes_a_trigger(&example.text) => {
                if let Err(err) = crate::pipeline::engines::basic::validate_flow(&graph) {
                    problems.push(format!("would be refused at activation: {}", err.message));
                }
            }
            Ok(_) => {}
        }
    }
    problems
}

/// Whether an example writes its own trigger node (`| trigger.…`,
/// `[a] trigger.…`), i.e. is a whole pipeline rather than a fragment. The
/// trigger [`check_definition_examples`] puts before a one-node example
/// (`| trigger.function | <node>`) is not the example's own.
fn writes_a_trigger(text: &str) -> bool {
    !text.starts_with(DEFINITION_WRAPPER) && text.lines().any(|line| {
        let line = line.trim_start();
        let line = line.strip_prefix('|').map(str::trim_start).unwrap_or(line);
        let line = match line.strip_prefix('[') {
            Some(rest) => rest.split_once(']').map_or(line, |(_, kind)| kind.trim_start()),
            None => line,
        };
        line.starts_with("trigger.") || line.contains("| trigger.")
    })
}

/// `input.rows`, `$nodes.n1.body`, `ctx.nodes.q.saved`, `$trigger.webhook` —
/// reads of keys no node answers.
pub(super) fn retired_reads(line: &str) -> Vec<String> {
    static READS: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    static ENVELOPE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let reads = READS.get_or_init(|| {
        regex::Regex::new(r"(\$?input|\$nodes\.[A-Za-z_][\w-]*|ctx\.nodes\.[A-Za-z_][\w-]*)\??\.([a-z_]+)\b").unwrap()
    });
    let mut out = Vec::new();
    for caps in reads.captures_iter(line) {
        let whole = caps.get(0).unwrap();
        let before = line[..whole.start()].chars().last();
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '$') {
            continue;
        }
        if RETIRED_KEYS.contains(&&caps[2]) {
            out.push(format!("`{}` — no node answers `{}` at the root", whole.as_str(), &caps[2]));
        }
    }
    let envelope = ENVELOPE.get_or_init(|| regex::Regex::new(r"(\$trigger|ctx\.trigger)\??\.webhook\b").unwrap());
    for m in envelope.find_iter(line) {
        out.push(format!("`{}` — `$trigger` is the envelope itself (`$trigger.body`)", m.as_str()));
    }
    out
}

/// A kind named in backticks (`` `fs.file.put` ``) that the catalogue does
/// not have — a typo, a retired `n.` kind, a kind renamed in 0.11.
pub(super) fn unknown_kind_names(line: &str, defs: &[NodeDefinition]) -> Vec<String> {
    delimited_spans(line, false)
        .into_iter()
        .filter(|span| names_a_node(span, defs) && !defs.iter().any(|d| &d.kind == span) && !span.starts_with("x."))
        .map(|span| format!("unknown kind `{span}`"))
        .collect()
}

/// Request fields a pre-0.11 trigger merged into the root of the payload. A
/// trigger answers under its source now (`{ "webhook": { "body": … } }`).
const ENVELOPE_KEYS: &[&str] = &["body", "files", "params", "headers", "method"];

/// The key a node adds, read from its kind — or, for a run input, from its
/// `--name` (the first bare word, or the flag).
fn example_answer_key(def: &NodeDefinition, dsl: &str) -> Option<String> {
    if def.kind.starts_with("input.") {
        let tokens = tokenize(dsl);
        if let Some(at) = tokens.iter().position(|t| t == "--name") {
            return tokens.get(at + 1).cloned();
        }
        return tokens.get(1).filter(|t| !t.starts_with("--")).cloned();
    }
    crate::pipeline::nodes::answer_key(&def.kind)
}

/// What is wrong with one definition's examples: its DSL against the
/// catalogue (as any guide example), its description's inline DSL and reads,
/// and its input/output JSON against the one-answer-key rule.
/// The trigger a one-node definition example is checked behind.
const DEFINITION_WRAPPER: &str = "| trigger.function | ";

pub(super) fn check_definition_examples(def: &NodeDefinition, defs: &[NodeDefinition]) -> Vec<String> {
    let mut problems = Vec::new();
    let is_trigger = def.input_pins.is_empty();
    for (n, example) in def.examples.iter().enumerate() {
        let label = if example.title.is_empty() { format!("example {}", n + 1) } else { format!("example `{}`", example.title) };
        let dsl = example.dsl.trim();
        if !dsl.is_empty() {
            let text = if is_trigger || dsl.starts_with('|') || dsl.starts_with('[') {
                if dsl.starts_with('|') || dsl.starts_with('[') { dsl.to_string() } else { format!("| {dsl}") }
            } else {
                format!("{DEFINITION_WRAPPER}{dsl}")
            };
            let whole = Example { line: 0, text, whole: true };
            for problem in check_example(&whole, defs) {
                problems.push(format!("{label}: {problem}"));
            }
            for problem in retired_reads(dsl) {
                problems.push(format!("{label}: {problem}"));
            }
        }
        let note = Guide { name: String::new(), text: example.description.clone(), markdown: true };
        for inline in examples(&note, defs) {
            for problem in check_example(&inline, defs) {
                problems.push(format!("{label} note: {problem}"));
            }
        }
        for line in example.description.lines() {
            for problem in retired_reads(line).into_iter().chain(unknown_kind_names(line, defs)) {
                problems.push(format!("{label} note: {problem}"));
            }
        }
        let key = example_answer_key(def, dsl);
        for (side, value) in [("input", &example.input), ("output", &example.output)] {
            let Some(root) = value.as_object() else { continue };
            // A trigger's example input is the request it receives, not a payload.
            if side == "input" && is_trigger {
                continue;
            }
            for k in root.keys() {
                if ENVELOPE_KEYS.contains(&k.as_str()) {
                    problems.push(format!(
                        "{label}: {side} has `{k}` at the root — the pre-0.11 envelope; a trigger answers under its source (`{{ \"webhook\": {{ \"{k}\": … }} }}`)"
                    ));
                } else if RETIRED_KEYS.contains(&k.as_str()) && key.as_deref() != Some(k.as_str()) {
                    problems.push(format!("{label}: {side} has `{k}` at the root — no node answers it there"));
                }
            }
        }
        if let (Some(key), Some(out)) = (key.as_deref(), example.output.as_object()) {
            if !out.contains_key(key) {
                problems.push(format!("{label}: output lacks the answer key `{key}`"));
            }
            if let Some(input) = example.input.as_object().filter(|_| !is_trigger) {
                let added: Vec<&String> = out.keys().filter(|k| !input.contains_key(*k) && k.as_str() != key).collect();
                if !added.is_empty() {
                    problems.push(format!("{label}: output adds {added:?} besides its answer key `{key}`"));
                }
                let dropped: Vec<&String> = input.keys().filter(|k| !out.contains_key(*k)).collect();
                if !dropped.is_empty() {
                    problems.push(format!("{label}: output drops {dropped:?} — a node keeps the rest of the payload"));
                }
            }
        }
    }
    let mut prose = def.description.clone();
    for flag in &def.dsl_flags {
        prose.push('\n');
        prose.push_str(&flag.description);
    }
    for line in prose.lines() {
        for problem in retired_reads(line).into_iter().chain(unknown_kind_names(line, defs)) {
            problems.push(format!("description: {problem}"));
        }
    }
    let described = Guide { name: String::new(), text: prose, markdown: true };
    for inline in examples(&described, defs) {
        for problem in check_example(&inline, defs) {
            problems.push(format!("description: {problem}"));
        }
    }
    problems
}

/// A markdown table row whose first cell is a flag: a copied flag table.
pub(super) fn copies_a_flag_row(line: &str) -> bool {
    line.trim_start().strip_prefix('|').is_some_and(|rest| rest.trim_start().starts_with("`--"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalogue() -> Vec<NodeDefinition> {
        super::super::official_node_definitions_for_help()
    }

    /// Every DSL example in every guide parses against the live catalogue,
    /// and no guide reads a retired root key.
    #[test]
    fn every_guide_example_speaks_the_live_grammar() {
        let defs = catalogue();
        let mut failures = Vec::new();
        let mut checked = 0usize;
        for guide in guides() {
            for example in examples(&guide, &defs) {
                checked += 1;
                for problem in check_example(&example, &defs) {
                    failures.push(format!("{}:{}: {problem}\n    {}", guide.name, example.line, example.text.replace('\n', "\n    ")));
                }
            }
            for (i, line) in guide.text.lines().enumerate() {
                for problem in retired_reads(line).into_iter().chain(unknown_kind_names(line, &defs)) {
                    failures.push(format!("{}:{}: {problem}", guide.name, i + 1));
                }
            }
        }
        assert!(failures.is_empty(), "guides teach what the catalogue does not have:\n{}", failures.join("\n"));
        assert!(checked >= 200, "the lint found only {checked} examples — the extractor is broken, not the guides");
    }

    /// Every official definition's examples — what `help("pipeline/nodes/<kind>")`
    /// shows — speak the grammar: their DSL parses, their JSON is a 0.11
    /// payload with the node's one answer key.
    #[test]
    fn every_definition_example_speaks_the_live_grammar() {
        let defs = catalogue();
        let mut failures = Vec::new();
        let mut checked = 0usize;
        for def in &defs {
            checked += def.examples.len();
            for problem in check_definition_examples(def, &defs) {
                failures.push(format!("{}: {problem}", def.kind));
            }
        }
        assert!(failures.is_empty(), "node definitions whose examples are stale:\n{}", failures.join("\n"));
        assert!(checked >= defs.len(), "only {checked} examples for {} kinds — the reader is broken", defs.len());
    }

    /// A hand-written page never carries a node's flag table; it embeds
    /// `<!-- node-flags:<kind> -->` or points to `help("pipeline/nodes/<kind>")`.
    #[test]
    fn no_hand_written_guide_copies_a_flag_table() {
        let mut failures = Vec::new();
        for guide in guides() {
            for (i, line) in guide.text.lines().enumerate() {
                if copies_a_flag_row(line) {
                    failures.push(format!("{}:{}: {}", guide.name, i + 1, line.trim()));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "hand-written flag tables — embed <!-- node-flags:<kind> --> instead:\n{}",
            failures.join("\n")
        );
    }

    /// The lint itself: each kind of stale example is caught, a current one
    /// passes, and an opted-out fence is left alone.
    #[test]
    fn the_lint_catches_each_kind_of_stale_example() {
        let defs = catalogue();
        let text = [
            "```",
            "| trigger.webhook --route /a --method POST",
            "| fs.file.put --source-key files.photo",
            "```",
            "Inline: `n.fs.save --folder x` and `fs.image.thumbnail --from \"{{ input.file }}\" --fit stretch`.",
            "Rows: `input.rows`, `$trigger.webhook.body`.",
            "<!-- dsl-lint: skip -->",
            "```",
            "| fs.file.put --source-key files.photo",
            "```",
            "Current: `fs.file.put --from \"{{ input.webhook.files.photo }}\" --accept image` and `input.query.rows`.",
            "| `--from` | a copied row |",
        ]
        .join("\n");
        let guide = Guide { name: "sample".into(), text, markdown: true };
        let found: Vec<String> = examples(&guide, &defs).iter().flat_map(|e| check_example(e, &defs)).collect();
        assert!(found.iter().any(|p| p.contains("declares no `--source-key`")), "{found:?}");
        assert!(found.iter().any(|p| p.contains("unknown kind `n.fs.save`")), "{found:?}");
        assert!(found.iter().any(|p| p.contains("--fit stretch")), "{found:?}");
        assert_eq!(found.len(), 3, "the skipped fence and the current example add nothing: {found:?}");
        let retired: Vec<String> = guide.text.lines().flat_map(retired_reads).collect();
        assert_eq!(retired.len(), 2, "{retired:?}");
        assert_eq!(guide.text.lines().filter(|l| copies_a_flag_row(l)).count(), 1);
    }
}

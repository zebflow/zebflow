//! Components in Markdown, without a code path through authored text.
//!
//! Every docs tool lets an author reach for something a paragraph cannot do:
//! an admonition, a set of tabs, a grid of cards, one install command per
//! package manager. Docusaurus reaches it through MDX, which means arbitrary
//! JSX inside content — authored text becoming code the page runs. A docs
//! folder is often the one part of a project an outsider may write, so that
//! is not a door this generator opens.
//!
//! The form here is the fenced directive (`:::note`, `:::tab`, `:::cards`,
//! `:::code-group`), parsed at build time into a closed set of blocks. A
//! directive cannot express anything that is not in this file, and the page
//! receives data, never markup: each block is rendered by a component in the
//! site's own template, so a project restyles every admonition by editing one
//! file.
//!
//! The author's Markdown is still Markdown. A block of kind `markdown` holds
//! its text and is rendered by the same renderer as the rest of the page, on
//! the server and again in the browser, so nothing here can make the two
//! passes disagree.

use serde::{Deserialize, Serialize};

/// One piece of a page: either Markdown, or one of the components a directive
/// can name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DocBlock {
    /// Markdown, rendered by the page's Markdown renderer.
    Markdown { text: String },
    /// `:::note`, `:::tip`, `:::info`, `:::warning`, `:::danger`,
    /// `:::caution` — a callout holding blocks of its own.
    Admonition {
        variant: String,
        title: Option<String>,
        blocks: Vec<DocBlock>,
    },
    /// Labelled panes, one shown at a time. Written as consecutive `:::tab`
    /// directives, optionally wrapped in a `:::tabs` that groups nothing more
    /// than they group themselves.
    Tabs { panes: Vec<DocTabPane> },
    /// `:::cards` — a link list rendered as a grid. The content is an
    /// ordinary Markdown list, so a reader of the source, and the build's own
    /// link check, both see plain links.
    Cards { items: Vec<DocCard> },
    /// `:::code-group` — one fenced block per language or package manager,
    /// behind a tab bar.
    CodeGroup { tabs: Vec<DocCodeTab> },
    /// A fenced block at the left margin, with the furniture a reader
    /// expects around one: the filename from the fence's `title="…"`, the
    /// lines its `{3,7-9}` names, and a copy button. A fence indented inside
    /// a list item is part of that list and stays in its Markdown.
    Code {
        language: String,
        title: Option<String>,
        diff: bool,
        lines: Vec<DocCodeLine>,
    },
    /// A directive this file does not know. Its content is still the author's,
    /// so it is carried through as blocks and rendered plainly — never
    /// dropped, and never turned into markup on the strength of its name.
    Unknown { name: String, blocks: Vec<DocBlock> },
    /// A call to a component the site's template declares:
    /// `<PotoPlayer src="stories/intro.poto" controls />` on a line of its
    /// own. This is deliberately not MDX. The name selects from the
    /// template's own registry and nothing else; a name that is not in it
    /// renders as text. Props are literal attributes — a string, a number or
    /// a flag — so there is no expression to evaluate and no children to run.
    Component {
        name: String,
        props: std::collections::BTreeMap<String, serde_json::Value>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocTabPane {
    pub label: String,
    pub blocks: Vec<DocBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocCard {
    pub title: String,
    pub href: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocCodeTab {
    pub label: String,
    pub language: String,
    pub diff: bool,
    pub lines: Vec<DocCodeLine>,
}

/// One line of a code block.
///
/// `text` is the author's source and nothing else, so joining a block's lines
/// gives exactly what the copy button puts on the clipboard. The `+` of an
/// added line lives in `mark`, which is why a reader can copy a diff and get
/// code rather than a patch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocCodeLine {
    pub text: String,
    /// `add`, `remove` or `meta` in a diff; nothing in any other language.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark: Option<String>,
    /// Named by the fence's `{3,7-9}`.
    #[serde(default, skip_serializing_if = "is_not_highlighted")]
    pub highlight: bool,
}

fn is_not_highlighted(value: &bool) -> bool {
    !*value
}

/// What a fence's info string says: `ts title="server.ts" {3,7-9}`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FenceInfo {
    pub language: String,
    pub title: Option<String>,
    pub highlight: Vec<usize>,
}

/// The widest `{from-to}` taken as a highlight. Past this it is a typo, and
/// expanding it would cost a line of memory per number.
const HIGHLIGHT_RANGE_MAX: usize = 500;

/// The admonition names. A directive outside this set, and outside the
/// structural names below, is [`DocBlock::Unknown`].
const ADMONITIONS: [&str; 6] = ["note", "tip", "info", "warning", "danger", "caution"];

/// A page's Markdown as blocks. Text with no directive in it answers one
/// `markdown` block, which is the common case and costs one allocation.
pub fn parse(markdown: &str) -> Vec<DocBlock> {
    group(parse_pieces(markdown))
}

/// A parsed directive before its siblings are known: a pane has to see the
/// ones next to it to become a tab set.
enum Piece {
    Block(DocBlock),
    Pane(DocTabPane),
}

/// Consecutive panes become one `Tabs`; everything else keeps its place. A
/// lone `:::tab` is a one-pane tab set rather than an error, and a `:::tabs`
/// wrapper has nothing left to do, so it splices its children in.
fn group(pieces: Vec<Piece>) -> Vec<DocBlock> {
    let mut out: Vec<DocBlock> = Vec::new();
    let mut panes: Vec<DocTabPane> = Vec::new();
    for piece in pieces {
        match piece {
            Piece::Pane(pane) => panes.push(pane),
            Piece::Block(block) => {
                if !panes.is_empty() {
                    out.push(DocBlock::Tabs { panes: std::mem::take(&mut panes) });
                }
                out.push(block);
            }
        }
    }
    if !panes.is_empty() {
        out.push(DocBlock::Tabs { panes });
    }
    out
}

fn parse_pieces(markdown: &str) -> Vec<Piece> {
    let lines: Vec<&str> = markdown.lines().collect();
    let mut out: Vec<Piece> = Vec::new();
    let mut text: Vec<&str> = Vec::new();
    let mut i = 0;
    let mut fence: Option<usize> = None;

    while i < lines.len() {
        let line = lines[i];
        // A directive inside a fenced block is the author showing one, not
        // writing one.
        if let Some(open) = fence {
            if closes_fence(line, open) {
                fence = None;
            }
            text.push(line);
            i += 1;
            continue;
        }
        if let Some(width) = opens_fence(line) {
            // A fence indented inside a list item belongs to the list that
            // holds it, so it stays in that Markdown and is rendered by the
            // Markdown renderer. One at the left margin is a block of its
            // own and gets the furniture.
            if !line.starts_with([' ', '\t']) {
                let (block, after) = code_block(line, &lines, i + 1, width);
                flush(&mut text, &mut out);
                out.push(Piece::Block(block));
                i = after;
                continue;
            }
            fence = Some(width);
            text.push(line);
            i += 1;
            continue;
        }

        if let Some(call) = component_call(line) {
            flush(&mut text, &mut out);
            out.push(Piece::Block(call));
            i += 1;
            continue;
        }

        match directive_open(line) {
            Some((_colons, name, argument)) => {
                let (content, after) = directive_body(&lines, i + 1);
                flush(&mut text, &mut out);
                out.extend(directive(&name, &argument, &content));
                i = after;
            }
            None => {
                text.push(line);
                i += 1;
            }
        }
    }
    flush(&mut text, &mut out);
    out
}

fn flush(text: &mut Vec<&str>, out: &mut Vec<Piece>) {
    let joined = text.join("\n");
    text.clear();
    if !joined.trim().is_empty() {
        out.push(Piece::Block(DocBlock::Markdown { text: joined.trim_matches('\n').to_string() }));
    }
}

/// `<Name attr="value" flag />` on a line of its own.
///
/// The name has to start with a capital, which is what separates a component
/// call from the HTML an author may write. Only a self-closing tag is a call:
/// with no children there is nothing to evaluate. An attribute holding `{`
/// is an expression, and this file does not evaluate expressions, so the line
/// is left as the text the author typed.
fn component_call(line: &str) -> Option<DocBlock> {
    let trimmed = line.trim();
    let body = trimmed.strip_prefix('<')?.strip_suffix("/>")?;
    if body.contains('<') || body.contains('{') || body.contains('}') {
        return None;
    }
    let name_len = body.find(char::is_whitespace).unwrap_or(body.len());
    let name = &body[..name_len];
    if !name.starts_with(|c: char| c.is_ascii_uppercase()) || !name.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    let props = component_props(body[name_len..].trim())?;
    Some(DocBlock::Component { name: name.to_string(), props })
}

/// The attributes of a call: `key="value"`, `key='value'`, or a bare `key`
/// meaning true. A value that reads as a number or as `true`/`false` is one,
/// because a component that asks for a height wants 420 and not "420".
fn component_props(text: &str) -> Option<std::collections::BTreeMap<String, serde_json::Value>> {
    let mut props = std::collections::BTreeMap::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '-' || chars[i] == '_') {
            i += 1;
        }
        if i == start {
            return None;
        }
        let key: String = chars[start..i].iter().collect();
        if chars.get(i) != Some(&'=') {
            props.insert(key, serde_json::Value::Bool(true));
            continue;
        }
        i += 1;
        let quote = *chars.get(i)?;
        if quote != '"' && quote != '\'' {
            return None;
        }
        i += 1;
        let from = i;
        while i < chars.len() && chars[i] != quote {
            i += 1;
        }
        if i >= chars.len() {
            return None;
        }
        let raw: String = chars[from..i].iter().collect();
        i += 1;
        props.insert(key, literal(&raw));
    }
    Some(props)
}

fn literal(raw: &str) -> serde_json::Value {
    match raw {
        "true" => return serde_json::Value::Bool(true),
        "false" => return serde_json::Value::Bool(false),
        _ => {}
    }
    if let Ok(number) = raw.parse::<f64>()
        && let Some(number) = serde_json::Number::from_f64(number)
    {
        return serde_json::Value::Number(number);
    }
    serde_json::Value::String(raw.to_string())
}

/// `:::name argument` — the colon count, the name, and whatever follows it.
fn directive_open(line: &str) -> Option<(usize, String, String)> {
    let trimmed = line.trim();
    let colons = trimmed.chars().take_while(|c| *c == ':').count();
    if colons < 3 {
        return None;
    }
    let rest = trimmed[colons..].trim();
    let mut chars = rest.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    let name_len = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let name = &rest[..name_len];
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }
    Some((colons, name.to_ascii_lowercase(), rest[name_len..].trim().to_string()))
}

/// Whether the line closes a directive: colons and nothing else.
fn directive_close(line: &str) -> Option<usize> {
    let trimmed = line.trim();
    let colons = trimmed.chars().take_while(|c| *c == ':').count();
    (colons >= 3 && colons == trimmed.chars().count()).then_some(colons)
}

/// The lines a directive holds, and the line after its close. An unclosed
/// directive holds the rest of its parent's content: a typo costs an author
/// the wrong shape, never the build.
fn directive_body(lines: &[&str], start: usize) -> (Vec<String>, usize) {
    let mut depth = 1usize;
    let mut body = Vec::new();
    let mut fence: Option<usize> = None;
    let mut i = start;
    while i < lines.len() {
        let line = lines[i];
        if let Some(open) = fence {
            if closes_fence(line, open) {
                fence = None;
            }
            body.push(line.to_string());
            i += 1;
            continue;
        }
        if let Some(width) = opens_fence(line) {
            fence = Some(width);
            body.push(line.to_string());
            i += 1;
            continue;
        }
        if directive_close(line).is_some() {
            depth -= 1;
            if depth == 0 {
                return (body, i + 1);
            }
            body.push(line.to_string());
            i += 1;
            continue;
        }
        if directive_open(line).is_some() {
            depth += 1;
        }
        body.push(line.to_string());
        i += 1;
    }
    (body, lines.len())
}

/// One directive, by name.
fn directive(name: &str, argument: &str, content: &[String]) -> Vec<Piece> {
    let body = content.join("\n");
    match name {
        "tab" => vec![Piece::Pane(DocTabPane {
            label: if argument.is_empty() { "Tab".to_string() } else { strip_quotes(argument) },
            blocks: parse(&body),
        })],
        // The wrapper groups nothing the panes do not group themselves, so it
        // adds no node of its own.
        "tabs" => parse_pieces(&body),
        "cards" => vec![Piece::Block(DocBlock::Cards { items: cards(&body) })],
        "code-group" | "codegroup" => vec![Piece::Block(DocBlock::CodeGroup { tabs: code_tabs(&body) })],
        other if ADMONITIONS.contains(&other) => vec![Piece::Block(DocBlock::Admonition {
            variant: other.to_string(),
            title: (!argument.is_empty()).then(|| strip_quotes(argument)),
            blocks: parse(&body),
        })],
        other => vec![Piece::Block(DocBlock::Unknown { name: other.to_string(), blocks: parse(&body) })],
    }
}

/// `:::cards` content: one top-level Markdown list item per card,
/// `- [Title](href) — description`. A line with no link is a card with a
/// title and no address rather than a dropped line.
fn cards(body: &str) -> Vec<DocCard> {
    let mut items: Vec<DocCard> = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim_start();
        // A continuation line belongs to the card above it.
        let bullet = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* "));
        let Some(rest) = bullet else {
            if let Some(last) = items.last_mut()
                && !trimmed.is_empty()
            {
                if !last.description.is_empty() {
                    last.description.push(' ');
                }
                last.description.push_str(trimmed);
            }
            continue;
        };
        let rest = rest.trim();
        match link_at_start(rest) {
            Some((title, href, tail)) => {
                items.push(DocCard { title, href: Some(href), description: card_description(tail) })
            }
            // A card with no link is a door that is not built yet. It still
            // reads as a title and a line about it.
            None => {
                let (title, description) = match rest.find([':', '\u{2014}', '\u{2013}']) {
                    Some(at) => (rest[..at].trim().to_string(), card_description(&rest[at..])),
                    None => (rest.to_string(), String::new()),
                };
                items.push(DocCard { title, href: None, description })
            }
        }
    }
    items
}

/// `[Title](href)` at the start of a card line, and what follows it.
fn link_at_start(text: &str) -> Option<(String, String, &str)> {
    let rest = text.strip_prefix('[')?;
    let close = rest.find("](")?;
    let title = &rest[..close];
    let after = &rest[close + 2..];
    let end = after.find(')')?;
    let href = after[..end].split_whitespace().next().unwrap_or("");
    Some((title.trim().to_string(), href.to_string(), &after[end + 1..]))
}

/// The words after a card's link, without the dash or colon that joined them.
fn card_description(tail: &str) -> String {
    tail.trim().trim_start_matches(['—', '–', '-', ':']).trim().to_string()
}

/// `:::code-group` content: every fenced block in it, in order. The label is
/// the fence's `title="…"`, else its `[name]`, else the language.
fn code_tabs(body: &str) -> Vec<DocCodeTab> {
    let lines: Vec<&str> = body.lines().collect();
    let mut tabs = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let Some(width) = opens_fence(lines[i]) else {
            i += 1;
            continue;
        };
        let (info, body, after) = fenced(lines[i], &lines, i + 1, width);
        i = after;
        let label = info.title.clone().unwrap_or_else(|| {
            if info.language.is_empty() { "Code".to_string() } else { info.language.clone() }
        });
        let (diff, code) = code_lines(&info.language, &info.highlight, &body);
        tabs.push(DocCodeTab { label, language: info.language, diff, lines: code });
    }
    tabs
}

/// A fenced block as its own [`DocBlock::Code`], from the line after the
/// opening fence. Answers where the page continues.
fn code_block(opening: &str, lines: &[&str], start: usize, width: usize) -> (DocBlock, usize) {
    let (info, body, after) = fenced(opening, lines, start, width);
    let (diff, code) = code_lines(&info.language, &info.highlight, &body);
    (
        DocBlock::Code { language: info.language, title: info.title, diff, lines: code },
        after,
    )
}

/// The info string and the body of one fence. An unclosed fence runs to the
/// end of the page, which is what a Markdown renderer does with one, so the
/// text an author typed is never dropped for a missing line.
fn fenced<'a>(
    opening: &str,
    lines: &[&'a str],
    start: usize,
    width: usize,
) -> (FenceInfo, Vec<&'a str>, usize) {
    let info = fence_info(opening.trim().trim_start_matches('`').trim());
    let mut body: Vec<&'a str> = Vec::new();
    let mut i = start;
    while i < lines.len() && !closes_fence(lines[i], width) {
        body.push(lines[i]);
        i += 1;
    }
    (info, body, (i + 1).min(lines.len()))
}

/// The lines of a fenced block, with what a reader sees kept apart from what
/// a reader copies.
fn code_lines(language: &str, highlight: &[usize], body: &[&str]) -> (bool, Vec<DocCodeLine>) {
    let diff = language.eq_ignore_ascii_case("diff");
    let lines = body
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let (mark, text) = if diff { diff_mark(line) } else { (None, (*line).to_string()) };
            DocCodeLine { text, mark, highlight: highlight.contains(&(index + 1)) }
        })
        .collect();
    (diff, lines)
}

/// A diff line's marker, taken out of the text. A `+++`/`---` file header and
/// an `@@` hunk header are neither added nor removed code, so they are marked
/// as what they are and keep their characters.
fn diff_mark(line: &str) -> (Option<String>, String) {
    if line.starts_with("+++") || line.starts_with("---") || line.starts_with("@@") {
        return (Some("meta".to_string()), line.to_string());
    }
    match line.as_bytes().first() {
        // The leading space of a context line is the diff's own column, not
        // the author's indentation.
        Some(b'+') => (Some("add".to_string()), line[1..].to_string()),
        Some(b'-') => (Some("remove".to_string()), line[1..].to_string()),
        Some(b' ') => (None, line[1..].to_string()),
        _ => (None, line.to_string()),
    }
}

/// A fence's info string: `ts title="server.ts" {3,7-9}`, `sh [npm]`, or
/// just a language. Anything it does not understand is ignored rather than
/// refused — a fence is the author's code, and a typo in its info string must
/// not cost them the block.
pub fn fence_info(info: &str) -> FenceInfo {
    let language = info.split_whitespace().next().unwrap_or("").to_string();
    let rest = &info[language.len()..];
    FenceInfo { language, title: fence_title(rest), highlight: highlight_lines(rest) }
}

fn fence_title(rest: &str) -> Option<String> {
    if let Some(at) = rest.find("title=") {
        let value = rest[at + "title=".len()..].trim();
        let label = match value.strip_prefix(['"', '\'']) {
            Some(quoted) => quoted.split(['"', '\'']).next().unwrap_or("").to_string(),
            None => value.split_whitespace().next().unwrap_or("").to_string(),
        };
        if !label.is_empty() {
            return Some(label);
        }
    }
    if let Some(open) = rest.find('[')
        && let Some(close) = rest[open..].find(']')
    {
        let label = rest[open + 1..open + close].trim().to_string();
        if !label.is_empty() {
            return Some(label);
        }
    }
    None
}

/// `{3,7-9}` as the line numbers it names, sorted and without repeats. A
/// range runs low to high however it is written, and a number past the end of
/// the block is simply never matched.
fn highlight_lines(rest: &str) -> Vec<usize> {
    let Some(open) = rest.find('{') else {
        return Vec::new();
    };
    let Some(close) = rest[open..].find('}') else {
        return Vec::new();
    };
    let mut out: Vec<usize> = Vec::new();
    for part in rest[open + 1..open + close].split(',') {
        let part = part.trim();
        match part.split_once('-') {
            Some((from, to)) => {
                if let (Ok(from), Ok(to)) =
                    (from.trim().parse::<usize>(), to.trim().parse::<usize>())
                {
                    let (low, high) = if from <= to { (from, to) } else { (to, from) };
                    if high - low <= HIGHLIGHT_RANGE_MAX {
                        out.extend(low..=high);
                    }
                }
            }
            None => {
                if let Ok(number) = part.parse::<usize>() {
                    out.push(number);
                }
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// The backtick count of a fence this line opens, if it opens one.
fn opens_fence(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    let width = trimmed.chars().take_while(|c| *c == '`').count();
    (width >= 3).then_some(width)
}

/// Whether the line closes a fence opened with `width` backticks.
fn closes_fence(line: &str, width: usize) -> bool {
    let trimmed = line.trim();
    let count = trimmed.chars().take_while(|c| *c == '`').count();
    count >= width && trimmed.chars().skip(count).all(char::is_whitespace)
}

fn strip_quotes(text: &str) -> String {
    let trimmed = text.trim();
    match trimmed.strip_prefix(['"', '\'']) {
        Some(rest) => rest.trim_end_matches(['"', '\'']).to_string(),
        None => trimmed.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the copy button copies: the lines' own text, nothing around it.
    fn code_text(lines: &[DocCodeLine]) -> String {
        lines.iter().map(|line| line.text.as_str()).collect::<Vec<_>>().join("\n")
    }

    fn markdown_text(block: &DocBlock) -> &str {
        match block {
            DocBlock::Markdown { text } => text,
            other => panic!("expected markdown, got {other:?}"),
        }
    }

    #[test]
    fn a_page_without_a_directive_is_one_markdown_block() {
        let blocks = parse("# Title\n\nA paragraph.\n");
        assert_eq!(blocks.len(), 1);
        assert_eq!(markdown_text(&blocks[0]), "# Title\n\nA paragraph.");
    }

    #[test]
    fn an_admonition_takes_its_title_from_the_opening_line() {
        let blocks = parse("Before.\n\n:::warning Read this first\nBe careful.\n:::\n\nAfter.\n");
        assert_eq!(blocks.len(), 3, "{blocks:?}");
        match &blocks[1] {
            DocBlock::Admonition { variant, title, blocks } => {
                assert_eq!(variant, "warning");
                assert_eq!(title.as_deref(), Some("Read this first"));
                assert_eq!(markdown_text(&blocks[0]), "Be careful.");
            }
            other => panic!("expected an admonition, got {other:?}"),
        }
        assert_eq!(markdown_text(&blocks[2]), "After.");
    }

    /// The outer directive uses more colons than the one it holds, which is
    /// how every tool with this syntax nests.
    #[test]
    fn an_admonition_nests_inside_an_admonition() {
        let blocks = parse("::::danger\nOuter.\n\n:::note\nInner.\n:::\n::::\n");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            DocBlock::Admonition { variant, blocks, .. } => {
                assert_eq!(variant, "danger");
                assert_eq!(blocks.len(), 2, "{blocks:?}");
                assert!(matches!(blocks[1], DocBlock::Admonition { .. }));
            }
            other => panic!("expected an admonition, got {other:?}"),
        }
    }

    #[test]
    fn consecutive_panes_become_one_tab_set() {
        let blocks = parse(":::tab Linux\nApt.\n:::\n:::tab macOS\nBrew.\n:::\n");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            DocBlock::Tabs { panes } => {
                assert_eq!(panes.len(), 2);
                assert_eq!(panes[0].label, "Linux");
                assert_eq!(panes[1].label, "macOS");
                assert_eq!(markdown_text(&panes[1].blocks[0]), "Brew.");
            }
            other => panic!("expected tabs, got {other:?}"),
        }
    }

    /// The wrapper is for the author's eye; it must not change the shape.
    #[test]
    fn a_tabs_wrapper_holds_the_same_panes() {
        let wrapped = parse("::::tabs\n:::tab One\nA.\n:::\n:::tab Two\nB.\n:::\n::::\n");
        let bare = parse(":::tab One\nA.\n:::\n:::tab Two\nB.\n:::\n");
        assert_eq!(wrapped, bare);
    }

    #[test]
    fn a_card_list_keeps_its_links_and_descriptions() {
        let blocks = parse(":::cards\n- [Getting started](./start.md) — install and run\n- [Concepts](./concepts.md): the model\n- Coming soon — one day\n:::\n");
        match &blocks[0] {
            DocBlock::Cards { items } => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0].title, "Getting started");
                assert_eq!(items[0].href.as_deref(), Some("./start.md"));
                assert_eq!(items[0].description, "install and run");
                assert_eq!(items[1].description, "the model");
                assert_eq!(items[2].href, None);
                assert_eq!(items[2].title, "Coming soon");
                assert_eq!(items[2].description, "one day");
            }
            other => panic!("expected cards, got {other:?}"),
        }
    }

    #[test]
    fn a_code_group_labels_each_block() {
        let blocks = parse(":::code-group\n```sh title=\"npm\"\nnpm i\n```\n```sh [pnpm]\npnpm i\n```\n```rust\nfn main() {}\n```\n:::\n");
        match &blocks[0] {
            DocBlock::CodeGroup { tabs } => {
                assert_eq!(tabs.len(), 3, "{tabs:?}");
                assert_eq!(tabs[0].label, "npm");
                assert_eq!(tabs[0].language, "sh");
                assert_eq!(code_text(&tabs[0].lines), "npm i");
                assert_eq!(tabs[1].label, "pnpm");
                assert_eq!(tabs[2].label, "rust");
            }
            other => panic!("expected a code group, got {other:?}"),
        }
    }

    /// A directive shown in a code block is an author writing about the
    /// syntax, and must survive as the text they typed.
    #[test]
    fn a_directive_inside_a_fence_stays_text() {
        let source = "Write it like this:\n\n```md\n:::note\nhello\n:::\n```\n";
        let blocks = parse(source);
        assert_eq!(blocks.len(), 2, "{blocks:?}");
        match &blocks[1] {
            DocBlock::Code { language, lines, .. } => {
                assert_eq!(language, "md");
                assert_eq!(code_text(lines), ":::note\nhello\n:::");
            }
            other => panic!("expected a code block, got {other:?}"),
        }
    }

    /// The whole point of the furniture: a fence at the left margin is a
    /// block the site's own component draws, so the filename, the highlighted
    /// lines and the copy button are the site's and not the renderer's.
    #[test]
    fn a_fence_becomes_a_code_block_with_its_filename_and_highlights() {
        let blocks = parse("Before.\n\n```ts title=\"server.ts\" {2,4-5}\none\ntwo\nthree\nfour\nfive\n```\n\nAfter.\n");
        assert_eq!(blocks.len(), 3, "{blocks:?}");
        match &blocks[1] {
            DocBlock::Code { language, title, diff, lines } => {
                assert_eq!(language, "ts");
                assert_eq!(title.as_deref(), Some("server.ts"));
                assert!(!diff);
                let marked: Vec<bool> = lines.iter().map(|line| line.highlight).collect();
                assert_eq!(marked, vec![false, true, false, true, true]);
            }
            other => panic!("expected a code block, got {other:?}"),
        }
        assert_eq!(markdown_text(&blocks[2]), "After.");
    }

    /// A fence inside a list item is part of the list: pulling it out would
    /// end the list at the fence and start a new one after it.
    #[test]
    fn an_indented_fence_stays_in_its_list() {
        let blocks = parse("1. Install it:\n\n   ```sh\n   npm i\n   ```\n\n2. Run it.\n");
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        let text = markdown_text(&blocks[0]);
        assert!(text.contains("npm i"), "{text}");
        assert!(text.contains("2. Run it."), "{text}");
    }

    /// A diff's `+` and `-` are marks, never text. The page draws them in a
    /// column of its own, and leaves them out of what it copies — which it
    /// can only do because the build took them out of the line.
    #[test]
    fn a_diff_keeps_its_signs_out_of_the_line_text() {
        let blocks = parse("```diff\n--- a/server.ts\n@@ -1,2 +1,2 @@\n-const port = 80;\n+const port = 8080;\n const host = \"localhost\";\n```\n");
        match &blocks[0] {
            DocBlock::Code { diff, lines, .. } => {
                assert!(diff);
                let marks: Vec<Option<&str>> =
                    lines.iter().map(|line| line.mark.as_deref()).collect();
                assert_eq!(
                    marks,
                    vec![Some("meta"), Some("meta"), Some("remove"), Some("add"), None]
                );
                // The lines as the page receives them: no sign in any of
                // them, whatever the page then chooses to copy.
                assert_eq!(
                    code_text(lines),
                    "--- a/server.ts\n@@ -1,2 +1,2 @@\nconst port = 80;\nconst port = 8080;\nconst host = \"localhost\";"
                );
            }
            other => panic!("expected a code block, got {other:?}"),
        }
    }

    /// A fence's info string is the author's, and a mistake in it costs a
    /// highlight, never the block.
    #[test]
    fn a_broken_info_string_still_answers_a_block() {
        for info in ["ts {", "ts {abc}", "ts {0}", "ts {9-3}", "ts {1-1000000000}", ""] {
            let source = format!("```{info}\none\ntwo\nthree\n```\n");
            match &parse(&source)[0] {
                DocBlock::Code { lines, .. } => {
                    assert_eq!(code_text(lines), "one\ntwo\nthree", "{info}");
                }
                other => panic!("{info} should still be a code block, got {other:?}"),
            }
        }
        // `{9-3}` is a range written backwards, which is still lines 3 to 9.
        match &parse("```ts {9-3}\n1\n2\n3\n4\n```\n")[0] {
            DocBlock::Code { lines, .. } => {
                let marked: Vec<bool> = lines.iter().map(|line| line.highlight).collect();
                assert_eq!(marked, vec![false, false, true, true]);
            }
            other => panic!("expected a code block, got {other:?}"),
        }
    }

    /// An unclosed fence is the author's text, all of it.
    #[test]
    fn an_unclosed_fence_does_not_lose_the_rest_of_the_page() {
        let blocks = parse("```sh\nnpm i\nstill here\n");
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        match &blocks[0] {
            DocBlock::Code { lines, .. } => assert_eq!(code_text(lines), "npm i\nstill here"),
            other => panic!("expected a code block, got {other:?}"),
        }
    }

    /// Naming a component does not conjure one: an unknown name carries its
    /// content through to be rendered plainly.
    #[test]
    fn an_unknown_directive_keeps_its_content() {
        let blocks = parse(":::script\nalert(1)\n:::\n");
        match &blocks[0] {
            DocBlock::Unknown { name, blocks } => {
                assert_eq!(name, "script");
                assert_eq!(markdown_text(&blocks[0]), "alert(1)");
            }
            other => panic!("expected an unknown block, got {other:?}"),
        }
    }

    #[test]
    fn a_component_call_becomes_a_block_with_its_props() {
        let blocks = parse("Before.\n\n<PotoPlayer src=\"stories/intro.poto\" controls height=\"420\" />\n\nAfter.\n");
        assert_eq!(blocks.len(), 3, "{blocks:?}");
        match &blocks[1] {
            DocBlock::Component { name, props } => {
                assert_eq!(name, "PotoPlayer");
                assert_eq!(props["src"], serde_json::json!("stories/intro.poto"));
                assert_eq!(props["controls"], serde_json::json!(true));
                // A component that asks for a height wants a number.
                assert_eq!(props["height"], serde_json::json!(420.0));
            }
            other => panic!("expected a component, got {other:?}"),
        }
    }

    /// The things that would make this MDX are the things it refuses.
    #[test]
    fn a_tag_that_is_not_a_literal_call_stays_text() {
        for line in [
            "<PotoPlayer src={story} />",      // an expression
            "<PotoPlayer src=\"a\"></PotoPlayer>", // children
            "<div class=\"x\" />",              // not a component name
            "<potoPlayer src=\"a\" />",          // not a component name
        ] {
            let blocks = parse(line);
            assert!(
                matches!(blocks.as_slice(), [DocBlock::Markdown { .. }]),
                "{line} should stay text, got {blocks:?}"
            );
        }
    }

    #[test]
    fn an_unclosed_directive_does_not_lose_the_rest_of_the_page() {
        let blocks = parse(":::note\nStill here.\n\nAnd this too.\n");
        match &blocks[0] {
            DocBlock::Admonition { blocks, .. } => {
                assert!(markdown_text(&blocks[0]).contains("And this too."));
            }
            other => panic!("expected an admonition, got {other:?}"),
        }
    }
}

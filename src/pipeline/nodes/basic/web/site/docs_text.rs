//! The text work of a docs site: front matter and `_meta.yaml`, headings
//! and their ids, plain text for search, and the head tags a page's SEO
//! rewrites. Pure functions over strings.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PageFrontmatter {
    pub title: Option<String>,
    pub description: Option<String>,
    pub order: Option<i64>,
    pub keywords: Vec<String>,
    pub canonical: Option<String>,
    pub noindex: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FolderMeta {
    pub title: Option<String>,
    pub order: Option<i64>,
    pub collapsed: Option<bool>,
    pub nav: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocHeading {
    pub level: u8,
    pub id: String,
    pub text: String,
}

pub(super) fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub(super) fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(super) fn replace_tag_content(input: &str, tag: &str, value: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    if let Some(start) = input.find(&open)
        && let Some(end_rel) = input[start + open.len()..].find(&close)
    {
        let end = start + open.len() + end_rel;
        let mut out = String::with_capacity(input.len() + value.len());
        out.push_str(&input[..start + open.len()]);
        out.push_str(value);
        out.push_str(&input[end..]);
        return out;
    }
    insert_before_head_end(input, &format!("{open}{value}{close}"))
}

pub(super) fn replace_or_insert_meta(input: &str, attr_name: &str, attr_value: &str, content: &str) -> String {
    let needle = format!("<meta {attr_name}=\"{attr_value}\"");
    if let Some(start) = input.find(&needle)
        && let Some(end_rel) = input[start..].find('>')
    {
        let end = start + end_rel + 1;
        let replacement = format!("<meta {attr_name}=\"{attr_value}\" content=\"{content}\">");
        let mut out = String::with_capacity(input.len() + replacement.len());
        out.push_str(&input[..start]);
        out.push_str(&replacement);
        out.push_str(&input[end..]);
        return out;
    }
    insert_before_head_end(
        input,
        &format!("<meta {attr_name}=\"{attr_value}\" content=\"{content}\">"),
    )
}

pub(super) fn upsert_link_rel(input: &str, rel: &str, href: &str) -> String {
    let needle = format!("<link rel=\"{rel}\"");
    if let Some(start) = input.find(&needle)
        && let Some(end_rel) = input[start..].find('>')
    {
        let end = start + end_rel + 1;
        let replacement = format!("<link rel=\"{rel}\" href=\"{}\">", html_escape(href));
        let mut out = String::with_capacity(input.len() + replacement.len());
        out.push_str(&input[..start]);
        out.push_str(&replacement);
        out.push_str(&input[end..]);
        return out;
    }
    insert_before_head_end(
        input,
        &format!("<link rel=\"{rel}\" href=\"{}\">", html_escape(href)),
    )
}

pub(super) fn insert_before_head_end(input: &str, snippet: &str) -> String {
    if let Some(pos) = input.find("</head>") {
        let mut out = String::with_capacity(input.len() + snippet.len());
        out.push_str(&input[..pos]);
        out.push_str(snippet);
        out.push_str(&input[pos..]);
        out
    } else {
        format!("{snippet}{input}")
    }
}

pub(super) fn split_frontmatter(raw: &str) -> (PageFrontmatter, String) {
    let Some(rest) = raw.strip_prefix("---\n") else {
        return (PageFrontmatter::default(), raw.to_string());
    };
    let Some(end) = rest.find("\n---\n") else {
        return (PageFrontmatter::default(), raw.to_string());
    };
    let frontmatter_raw = &rest[..end];
    let body = rest[end + "\n---\n".len()..].to_string();
    (parse_page_frontmatter(frontmatter_raw), body)
}

pub(super) fn parse_page_frontmatter(raw: &str) -> PageFrontmatter {
    let map = parse_simple_yaml_map(raw);
    PageFrontmatter {
        title: map_string(&map, "title"),
        description: map_string(&map, "description"),
        order: map_i64(&map, "order"),
        keywords: map_string_list(&map, "keywords"),
        canonical: map_string(&map, "canonical"),
        noindex: map_bool(&map, "noindex"),
    }
}

pub(super) fn parse_folder_meta(raw: &str) -> FolderMeta {
    let map = parse_simple_yaml_map(raw);
    FolderMeta {
        title: map_string(&map, "title"),
        order: map_i64(&map, "order"),
        collapsed: map_optional_bool(&map, "collapsed"),
        nav: map_string_list(&map, "nav"),
    }
}

pub(super) fn parse_simple_yaml_map(raw: &str) -> Map<String, Value> {
    let mut out = Map::new();
    let mut current_list_key: Option<String> = None;
    let mut current_list = Vec::new();

    let flush_list =
        |out: &mut Map<String, Value>, key: &mut Option<String>, list: &mut Vec<Value>| {
            if let Some(current_key) = key.take() {
                out.insert(current_key, Value::Array(std::mem::take(list)));
            }
        };

    for line in raw.lines() {
        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() || trimmed.trim_start().starts_with('#') {
            continue;
        }
        let start_trimmed = trimmed.trim_start();
        if let Some(item) = start_trimmed.strip_prefix("- ") {
            if current_list_key.is_some() {
                current_list.push(parse_scalar(item.trim()));
            }
            continue;
        }
        flush_list(&mut out, &mut current_list_key, &mut current_list);
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim().to_string();
        let value = value.trim();
        if value.is_empty() {
            current_list_key = Some(key);
            current_list = Vec::new();
        } else {
            out.insert(key, parse_scalar(value));
        }
    }
    flush_list(&mut out, &mut current_list_key, &mut current_list);
    out
}

pub(super) fn parse_scalar(raw: &str) -> Value {
    let trimmed = raw.trim().trim_matches('"').trim_matches('\'');
    if trimmed.eq_ignore_ascii_case("true") {
        Value::Bool(true)
    } else if trimmed.eq_ignore_ascii_case("false") {
        Value::Bool(false)
    } else if let Ok(number) = trimmed.parse::<i64>() {
        Value::Number(number.into())
    } else {
        Value::String(trimmed.to_string())
    }
}

pub(super) fn map_string(map: &Map<String, Value>, key: &str) -> Option<String> {
    map.get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

pub(super) fn map_i64(map: &Map<String, Value>, key: &str) -> Option<i64> {
    map.get(key).and_then(Value::as_i64)
}

pub(super) fn map_bool(map: &Map<String, Value>, key: &str) -> bool {
    map.get(key).and_then(Value::as_bool).unwrap_or(false)
}

pub(super) fn map_optional_bool(map: &Map<String, Value>, key: &str) -> Option<bool> {
    map.get(key).and_then(Value::as_bool)
}

pub(super) fn map_string_list(map: &Map<String, Value>, key: &str) -> Vec<String> {
    match map.get(key) {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect(),
        Some(Value::String(value)) => vec![value.clone()],
        _ => Vec::new(),
    }
}

pub(super) fn extract_headings(markdown: &str) -> Vec<DocHeading> {
    let mut out = Vec::new();
    let mut seen = HashMap::new();
    for line in markdown.lines() {
        let trimmed = line.trim_start();
        let level = trimmed.chars().take_while(|ch| *ch == '#').count();
        if !(1..=6).contains(&level) {
            continue;
        }
        let Some(rest) = trimmed.get(level..) else {
            continue;
        };
        let text = rest.trim();
        if text.is_empty() {
            continue;
        }
        let base = slugify(text);
        let id = unique_slug(base, &mut seen);
        out.push(DocHeading {
            level: level as u8,
            id,
            text: strip_inline_markdown(text),
        });
    }
    out
}

pub(super) fn strip_inline_markdown(input: &str) -> String {
    input
        .chars()
        .filter(|ch| !matches!(ch, '*' | '_' | '`' | '[' | ']' | '(' | ')' | '#' | '!'))
        .collect::<String>()
        .trim()
        .to_string()
}

pub(super) fn first_paragraph(markdown: &str) -> String {
    let mut lines = Vec::new();
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !lines.is_empty() {
                break;
            }
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        lines.push(trimmed);
    }
    lines.join(" ")
}

pub(super) fn excerpt_for_search(markdown: &str) -> String {
    plain_text_from_markdown(markdown)
        .split_whitespace()
        .take(48)
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn plain_text_from_markdown(markdown: &str) -> String {
    markdown
        .lines()
        .map(|line| {
            line.trim()
                .trim_start_matches('#')
                .trim_start_matches('-')
                .trim_start_matches('*')
                .trim_start_matches('>')
                .trim()
                .replace('`', "")
                .replace('[', "")
                .replace(']', "")
                .replace('(', "")
                .replace(')', "")
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn titleize_segment(raw: &str) -> String {
    raw.split(['-', '_', ' '])
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => format!(
                    "{}{}",
                    first.to_uppercase().collect::<String>(),
                    chars.as_str().to_lowercase()
                ),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn slugify(raw: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true;
    for ch in raw.chars().flat_map(|ch| ch.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "section".to_string()
    } else {
        trimmed
    }
}

pub(super) fn unique_slug(base: String, seen: &mut HashMap<String, usize>) -> String {
    match seen.get_mut(&base) {
        Some(count) => {
            *count += 1;
            format!("{base}-{}", *count)
        }
        None => {
            seen.insert(base.clone(), 0);
            base
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_headings, parse_folder_meta, split_frontmatter};

    #[test]
    fn parses_frontmatter_and_markdown_body() {
        let raw = "---\ntitle: Query Basics\ndescription: Learn query flow\norder: 20\nkeywords:\n  - sekejap\n  - query\nnoindex: true\n---\n# Heading\n\nHello";
        let (fm, body) = split_frontmatter(raw);
        assert_eq!(fm.title.as_deref(), Some("Query Basics"));
        assert_eq!(fm.description.as_deref(), Some("Learn query flow"));
        assert_eq!(fm.order, Some(20));
        assert_eq!(
            fm.keywords,
            vec!["sekejap".to_string(), "query".to_string()]
        );
        assert!(fm.noindex);
        assert!(body.contains("# Heading"));
    }

    #[test]
    fn parses_folder_meta_lists() {
        let meta = parse_folder_meta("title: Basic\ncollapsed: true\nnav:\n  - index\n  - query\n");
        assert_eq!(meta.title.as_deref(), Some("Basic"));
        assert_eq!(meta.collapsed, Some(true));
        assert_eq!(meta.nav, vec!["index".to_string(), "query".to_string()]);
    }

    #[test]
    fn extracts_heading_ids() {
        let headings = extract_headings("# Intro\n## Query\n## Query\n");
        assert_eq!(headings.len(), 3);
        assert_eq!(headings[1].id, "query");
        assert_eq!(headings[2].id, "query-1");
    }
}

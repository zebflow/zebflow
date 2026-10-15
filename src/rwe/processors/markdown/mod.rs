//! Markdown processor for Zebflow RWE compile stage.
//!
//! This processor converts `<markdown>...</markdown>` blocks into HTML.
//! It is designed as an opt-in compile feature via `ReactiveWebOptions.processors`.

use std::collections::HashMap;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use pulldown_cmark::{CowStr, Event, LinkType, Options, Parser, Tag, html};

/// Converts `<markdown>...</markdown>` blocks into HTML fragments.
///
/// Unsupported patterns are left untouched. If a closing `</markdown>` is
/// missing, the original source from that point is preserved.
pub fn process_markdown(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut cursor = 0usize;
    while let Some(start_rel) = input[cursor..].find("<markdown>") {
        let start = cursor + start_rel;
        out.push_str(&input[cursor..start]);
        let body_start = start + "<markdown>".len();
        let Some(end_rel) = input[body_start..].find("</markdown>") else {
            out.push_str(&input[start..]);
            return out;
        };
        let body_end = body_start + end_rel;
        let rendered = render_markdown_fragment(&input[body_start..body_end]);
        out.push_str(&rendered);
        cursor = body_end + "</markdown>".len();
    }
    out.push_str(&input[cursor..]);
    out
}

/// Processes `<div data-rwe-md="base64...">` placeholders emitted by the `<Markdown>` component.
///
/// The component HTML-escapes content, so we use base64 to safely carry raw markdown
/// through the JSX render pipeline. This processor decodes and renders each placeholder.
pub fn process_rwe_md_placeholders(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut cursor = 0usize;
    let needle = "data-rwe-md=\"";
    while let Some(attr_rel) = input[cursor..].find(needle) {
        // Find the opening tag that contains this attribute — backtrack to nearest `<`
        let attr_pos = cursor + attr_rel;
        let Some(tag_start_rel) = input[cursor..attr_pos].rfind('<') else {
            out.push_str(&input[cursor..attr_pos + needle.len()]);
            cursor = attr_pos + needle.len();
            continue;
        };
        let tag_start = cursor + tag_start_rel;

        // Extract base64 value between the quotes
        let val_start = attr_pos + needle.len();
        let Some(val_end_rel) = input[val_start..].find('"') else {
            out.push_str(&input[cursor..val_start]);
            cursor = val_start;
            continue;
        };
        let val_end = val_start + val_end_rel;
        let encoded = &input[val_start..val_end];

        // Find the closing tag (self-closing or </div>)
        let after_attr = val_end + 1; // skip closing quote
        let tag_end = if let Some(sc) = input[after_attr..].find("/>") {
            after_attr + sc + "/>".len()
        } else if let Some(cl) = input[after_attr..].find('>') {
            // Find corresponding </div>
            let inner_start = after_attr + cl + 1;
            if let Some(close_rel) = input[inner_start..].find("</div>") {
                inner_start + close_rel + "</div>".len()
            } else {
                after_attr + cl + 1
            }
        } else {
            after_attr
        };

        // Emit everything before the tag
        out.push_str(&input[cursor..tag_start]);

        // Decode and render
        let md_text = BASE64
            .decode(encoded)
            .ok()
            .and_then(|b| String::from_utf8(b).ok())
            .unwrap_or_default();
        let rendered_html = render_markdown_fragment(&md_text);

        // Extract class from the placeholder div (if any)
        let tag_snippet = &input[tag_start..val_end];
        let extra_class = extract_class_attr(tag_snippet).unwrap_or_default();
        let class_attr = if extra_class.is_empty() {
            "prose".to_string()
        } else {
            format!("prose {extra_class}")
        };

        out.push_str(&format!(
            "<div class=\"{class_attr}\">{rendered_html}</div>"
        ));
        cursor = tag_end;
    }
    out.push_str(&input[cursor..]);
    out
}

fn extract_class_attr(tag_snippet: &str) -> Option<String> {
    let needle = "class=\"";
    let start = tag_snippet.find(needle)? + needle.len();
    let end = tag_snippet[start..].find('"')?;
    let raw = &tag_snippet[start..start + end];
    // Filter out the placeholder class we set in the component and reject quotes/angle brackets
    let filtered = raw
        .split_whitespace()
        .filter(|c| *c != "rwe-md-placeholder" && !c.contains(['"', '\'', '<', '>']))
        .collect::<Vec<_>>()
        .join(" ");
    Some(filtered)
}

/// Whether a markdown link or image destination may reach the page as written.
///
/// A destination without a scheme (`guide/intro`, `?page=2`, `#top`, `/docs`)
/// is relative to the page and kept. A scheme is kept only when it is http,
/// https, mailto or tel. `//host` and `/\\host` are refused: a browser reads
/// both as another host. The check runs on the destination as a browser parses
/// it — surrounding control characters trimmed and tabs and newlines removed —
/// so `java\tscript:` cannot slip past it. pulldown-cmark hands an email
/// autolink over as the bare address and adds `mailto:` itself, so it is
/// checked with that prefix.
fn safe_markdown_url(link_type: LinkType, url: &str) -> bool {
    let as_browser_reads: String = url
        .trim_matches(|c: char| c <= ' ')
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    if link_type == LinkType::Email {
        return !as_browser_reads.is_empty()
            && safe_markdown_url(LinkType::Inline, &format!("mailto:{as_browser_reads}"));
    }
    if as_browser_reads.is_empty() {
        return false;
    }
    let mut leading = as_browser_reads.chars();
    if matches!(
        (leading.next(), leading.next()),
        (Some('/' | '\\'), Some('/' | '\\'))
    ) {
        return false;
    }
    match as_browser_reads.find([':', '/', '?', '#']) {
        Some(at) if as_browser_reads[at..].starts_with(':') => {
            let scheme = as_browser_reads[..at].to_ascii_lowercase();
            matches!(scheme.as_str(), "http" | "https" | "mailto" | "tel")
        }
        _ => true,
    }
}

pub fn render_markdown_fragment(md: &str) -> String {
    let stripped = strip_script_blocks(md);
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    let parser = Parser::new_ext(&stripped, options).map(|event| match event {
        Event::Html(raw) | Event::InlineHtml(raw) => Event::Text(raw),
        Event::Start(Tag::Link { link_type, dest_url, title, id }) if !safe_markdown_url(link_type, &dest_url) => {
            Event::Start(Tag::Link { link_type, dest_url: CowStr::Borrowed("#"), title, id })
        }
        Event::Start(Tag::Image { link_type, dest_url, title, id }) if !safe_markdown_url(link_type, &dest_url) => {
            Event::Start(Tag::Image { link_type, dest_url: CowStr::Borrowed("#"), title, id })
        }
        other => other,
    });
    let mut out = String::new();
    html::push_html(&mut out, parser);
    add_heading_ids(&strip_script_blocks(&out))
}

fn strip_script_blocks(input: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let mut out = String::with_capacity(input.len());
    let mut cursor = 0usize;

    while let Some(start_rel) = lower[cursor..].find("<script") {
        let start = cursor + start_rel;
        out.push_str(&input[cursor..start]);
        let Some(end_rel) = lower[start..].find("</script>") else {
            return out;
        };
        cursor = start + end_rel + "</script>".len();
    }

    out.push_str(&input[cursor..]);
    out
}

fn add_heading_ids(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 64);
    let mut cursor = 0usize;
    let mut seen: HashMap<String, usize> = HashMap::new();

    while let Some(start_rel) = input[cursor..].find("<h") {
        let start = cursor + start_rel;
        out.push_str(&input[cursor..start]);

        let level_pos = start + 2;
        let Some(level) = input
            .get(level_pos..level_pos + 1)
            .and_then(|s| s.chars().next())
        else {
            out.push_str(&input[start..]);
            return out;
        };
        if !('1'..='6').contains(&level) {
            out.push_str("<h");
            cursor = level_pos;
            continue;
        }

        let Some(tag_end_rel) = input[level_pos + 1..].find('>') else {
            out.push_str(&input[start..]);
            return out;
        };
        let tag_end = level_pos + 1 + tag_end_rel;
        let close_tag = format!("</h{level}>");
        let Some(close_rel) = input[tag_end + 1..].find(&close_tag) else {
            out.push_str(&input[start..]);
            return out;
        };
        let close_start = tag_end + 1 + close_rel;
        let inner = &input[tag_end + 1..close_start];
        let text = strip_html_tags(inner);
        let base_id = slugify_heading(&text);
        let heading_id = unique_slug(base_id, &mut seen);

        let opening = &input[start..=tag_end];
        if opening.contains(" id=") {
            out.push_str(opening);
        } else {
            out.push_str(&format!("<h{level} id=\"{heading_id}\">"));
        }
        out.push_str(inner);
        out.push_str(&close_tag);
        cursor = close_start + close_tag.len();
    }

    out.push_str(&input[cursor..]);
    out
}

fn strip_html_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

fn slugify_heading(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true;
    for ch in input.chars().flat_map(|ch| ch.to_lowercase()) {
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

fn unique_slug(base: String, seen: &mut HashMap<String, usize>) -> String {
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
    use super::render_markdown_fragment;

    #[test]
    fn markdown_keeps_relative_and_email_links_and_refuses_unsafe_schemes() {
        let html = render_markdown_fragment(
            "[guide](guide/intro) [next](?page=2) [top](#top) [docs](/docs) \
             [site](https://example.com/a) [call](tel:+15550100) <someone@example.com>",
        );
        for kept in [
            "href=\"guide/intro\"",
            "href=\"?page=2\"",
            "href=\"#top\"",
            "href=\"/docs\"",
            "href=\"https://example.com/a\"",
            "href=\"tel:+15550100\"",
            "href=\"mailto:someone@example.com\"",
        ] {
            assert!(html.contains(kept), "{kept} missing from {html}");
        }

        let html = render_markdown_fragment(
            "[a](javascript:alert(1)) [b](JavaScript:alert(1)) [c](data:text/html,x) \
             [d](//example.com/x) [e](/\\example.com/x) [f](<java\tscript:alert(1)>) \
             [g](vbscript:x) ![h](javascript:alert(1))",
        );
        assert!(!html.to_ascii_lowercase().contains("script:"), "{html}");
        assert!(!html.contains("data:"), "{html}");
        assert!(!html.contains("example.com"), "{html}");
        assert_eq!(html.matches("href=\"#\"").count(), 7, "{html}");
        assert!(html.contains("src=\"#\""), "{html}");
    }
}

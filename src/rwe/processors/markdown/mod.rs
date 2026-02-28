//! Markdown processor for Zebflow RWE compile stage.
//!
//! This processor converts `<markdown>...</markdown>` blocks into HTML.
//! It is designed as an opt-in compile feature via `ReactiveWebOptions.processors`.

use pulldown_cmark::{Options, Parser, html};

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

fn render_markdown_fragment(md: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    let parser = Parser::new_ext(md, options);
    let mut out = String::new();
    html::push_html(&mut out, parser);
    strip_script_blocks(&out)
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

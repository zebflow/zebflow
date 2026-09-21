//! Text that wraps. SVG 2 gives a `<text>` a width with `inline-size`;
//! resvg does not implement it, so the node breaks the lines before resvg
//! sees the file: the element becomes one `<tspan>` per line, each at the
//! element's `x`, advancing by the line height.
//!
//! Each candidate line is measured by usvg itself — a one-element SVG with
//! the same family, weight, size and letter-spacing as the final render, its
//! ink width read back — so a line that measures as fitting renders as
//! fitting. Font properties are read from the element and its ancestors
//! (attributes, then `style="…"`); a `<style>` block is not consulted.
//! `<tspan>` children of a wrapped element are flattened into its text.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;

use roxmltree::{Document, Node};

use super::ConvertError;
use super::fonts::FontSet;

const DEFAULT_FONT_SIZE: f32 = 16.0;
const DEFAULT_LINE_HEIGHT: f32 = 1.2;

pub struct TextMeasurer<'a> {
    opt: usvg::Options<'a>,
    widths: RefCell<HashMap<(String, String, u16, u32, u32), f32>>,
    pub calls: std::cell::Cell<usize>,
}

impl<'a> TextMeasurer<'a> {
    pub fn new(fonts: &FontSet) -> Self {
        Self { opt: fonts.options(), widths: RefCell::new(HashMap::new()), calls: std::cell::Cell::new(0) }
    }

    /// Ink width of `text` as usvg lays it out; 0 for blank input.
    pub fn width(&self, text: &str, family: &str, weight: u16, size: f32, letter_spacing: f32) -> f32 {
        if text.trim().is_empty() {
            return 0.0;
        }
        let key = (text.to_string(), family.to_string(), weight, size.to_bits(), letter_spacing.to_bits());
        if let Some(w) = self.widths.borrow().get(&key) {
            return *w;
        }
        self.calls.set(self.calls.get() + 1);
        let mut svg = String::with_capacity(text.len() + 160);
        let _ = write!(svg, r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><text x="0" y="0" font-family=""#);
        escape_attr_into(&mut svg, family);
        let _ = write!(svg, r#"" font-weight="{weight}" font-size="{size}" letter-spacing="{letter_spacing}">"#);
        escape_into(&mut svg, text);
        svg.push_str("</text></svg>");
        let w = match usvg::Tree::from_str(&svg, &self.opt) {
            Ok(tree) => tree.root().bounding_box().width(),
            Err(_) => 0.0,
        };
        self.widths.borrow_mut().insert(key, w);
        w
    }
}

/// Greedy word wrap. A word wider than `max_w` gets its own line.
pub fn wrap(text: &str, max_w: f32, measure: impl Fn(&str) -> f32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut cur = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
            if cur.is_empty() || measure(&candidate) <= max_w {
                cur = candidate;
            } else {
                lines.push(std::mem::take(&mut cur));
                cur = word.to_string();
            }
        }
        if !cur.is_empty() || paragraph.trim().is_empty() && !lines.is_empty() {
            lines.push(cur);
        }
    }
    lines
}

/// A property as the element sees it: its own attribute, else its own
/// `style`, else the nearest ancestor's.
fn inherited(node: Node<'_, '_>, name: &str) -> Option<String> {
    for n in node.ancestors().filter(|n| n.is_element()) {
        if let Some(v) = n.attribute(name) {
            return Some(v.trim().to_string());
        }
        if let Some(style) = n.attribute("style") {
            for decl in style.split(';') {
                if let Some((k, v)) = decl.split_once(':') {
                    if k.trim().eq_ignore_ascii_case(name) {
                        return Some(v.trim().to_string());
                    }
                }
            }
        }
    }
    None
}

/// The element's own value of a property, attribute or `style`, not inherited.
fn own(node: Node<'_, '_>, name: &str) -> Option<String> {
    if let Some(v) = node.attribute(name) {
        return Some(v.trim().to_string());
    }
    node.attribute("style").and_then(|style| {
        style.split(';').find_map(|decl| {
            let (k, v) = decl.split_once(':')?;
            k.trim().eq_ignore_ascii_case(name).then(|| v.trim().to_string())
        })
    })
}

/// `40`, `40px`, `30pt` → pixels.
fn length_px(raw: &str) -> Option<f32> {
    let raw = raw.trim();
    if let Some(v) = raw.strip_suffix("px") {
        return v.trim().parse().ok();
    }
    if let Some(v) = raw.strip_suffix("pt") {
        return v.trim().parse::<f32>().ok().map(|pt| pt * 4.0 / 3.0);
    }
    raw.parse().ok()
}

fn weight_of(raw: &str) -> u16 {
    match raw.trim().to_ascii_lowercase().as_str() {
        "bold" | "bolder" => 700,
        "normal" => 400,
        "lighter" => 300,
        other => other.parse().unwrap_or(400),
    }
}

/// Every `font-family` the document names must be one the project has, so
/// no text silently falls back to another face.
pub fn check_families(doc: &Document<'_>, fonts: &FontSet) -> Result<(), ConvertError> {
    for node in doc.descendants().filter(|n| n.is_element()) {
        if let Some(stack) = own(node, "font-family") {
            fonts.resolve_stack(&stack)?;
        }
    }
    Ok(())
}

/// The document with every `<text inline-size="…">` broken into lines.
/// Answers the input unchanged when nothing declares a width.
pub fn wrap_inline_size(doc: &Document<'_>, fonts: &FontSet) -> Result<String, ConvertError> {
    let source = doc.input_text();
    let m = TextMeasurer::new(fonts);
    let mut edits: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    for node in doc.descendants().filter(|n| n.is_element() && n.tag_name().name() == "text") {
        let Some(max_w) = own(node, "inline-size").and_then(|v| length_px(&v)).filter(|w| *w > 0.0) else {
            continue;
        };
        let text: String = node.descendants().filter(|n| n.is_text()).filter_map(|n| n.text()).collect();
        if text.trim().is_empty() {
            continue;
        }
        let family = fonts.resolve_stack(&inherited(node, "font-family").unwrap_or_else(|| fonts.default_family().to_string()))?;
        let weight = inherited(node, "font-weight").map(|w| weight_of(&w)).unwrap_or(400);
        let size = inherited(node, "font-size").and_then(|s| length_px(&s)).unwrap_or(DEFAULT_FONT_SIZE).max(1.0);
        let letter_spacing = inherited(node, "letter-spacing").and_then(|s| length_px(&s)).unwrap_or(0.0);
        let line_height = match inherited(node, "line-height") {
            Some(raw) if raw.ends_with("px") => length_px(&raw).unwrap_or(size * DEFAULT_LINE_HEIGHT),
            Some(raw) => raw.trim().parse::<f32>().map(|r| r * size).unwrap_or(size * DEFAULT_LINE_HEIGHT),
            None => size * DEFAULT_LINE_HEIGHT,
        };
        let lines = wrap(&text, max_w, |t| m.width(t, family, weight, size, letter_spacing));
        let x = node.attribute("x").and_then(|x| x.split_whitespace().next()).unwrap_or("0").to_string();

        let mut out = String::with_capacity(text.len() + 128);
        out.push_str("<text");
        for attr in node.attributes() {
            let name = attr.name();
            if name == "inline-size" {
                continue;
            }
            out.push(' ');
            if attr.namespace() == Some(roxmltree::NS_XML_URI) {
                out.push_str("xml:");
            }
            out.push_str(name);
            let value = if name == "style" {
                attr.value()
                    .split(';')
                    .filter(|d| !d.split_once(':').is_some_and(|(k, _)| k.trim().eq_ignore_ascii_case("inline-size")))
                    .collect::<Vec<_>>()
                    .join(";")
            } else {
                attr.value().to_string()
            };
            out.push_str("=\"");
            escape_attr_into(&mut out, &value);
            out.push('"');
        }
        out.push('>');
        for (i, line) in lines.iter().enumerate() {
            let dy = if i == 0 { 0.0 } else { line_height };
            let _ = write!(out, r#"<tspan x="{x}" dy="{dy}">"#);
            escape_into(&mut out, line);
            out.push_str("</tspan>");
        }
        out.push_str("</text>");
        edits.push((node.range(), out));
    }
    if edits.is_empty() {
        return Ok(source.to_string());
    }
    edits.sort_by_key(|(r, _)| r.start);
    let mut result = String::with_capacity(source.len() + edits.len() * 64);
    let mut cursor = 0;
    for (range, replacement) in edits {
        if range.start < cursor {
            // A <text> inside a <text> is not SVG; keep the outer edit.
            continue;
        }
        result.push_str(&source[cursor..range.start]);
        result.push_str(&replacement);
        cursor = range.end;
    }
    result.push_str(&source[cursor..]);
    Ok(result)
}

pub fn escape_into(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
}

pub fn escape_attr_into(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_size_becomes_tspans_that_each_fit_the_width() {
        let fonts = FontSet::bundled();
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="1080" height="600"><g font-family="Inter" font-weight="800"><text x="540" y="200" font-size="80" text-anchor="middle" inline-size="900" fill="#fff">a chill trance night with slow builds, warm pads and a room that never rushes</text></g><text x="10" y="500">left alone</text></svg>"##;
        let doc = Document::parse(svg).unwrap();
        let out = wrap_inline_size(&doc, &fonts).unwrap();
        assert!(!out.contains("inline-size"), "{out}");
        assert!(out.contains(r#"<tspan x="540" dy="0">"#) && out.contains(r#"<tspan x="540" dy="96">"#), "{out}");
        assert!(out.contains(r#"<text x="10" y="500">left alone</text>"#));
        let m = TextMeasurer::new(&fonts);
        let family = fonts.resolve("Inter").unwrap();
        let lines: Vec<&str> = out.split("<tspan").skip(1).map(|s| s.split_once('>').unwrap().1.split("</tspan>").next().unwrap()).collect();
        assert!(lines.len() >= 2, "{lines:?}");
        for line in lines {
            assert!(m.width(line, family, 800, 80.0, 0.0) <= 900.0, "{line}");
        }
        // The result is still an SVG usvg accepts.
        assert!(usvg::Tree::from_str(&out, &fonts.options()).is_ok());
    }

    #[test]
    fn style_declared_properties_and_a_pixel_line_height_are_read_and_inline_size_is_dropped_from_style() {
        let fonts = FontSet::bundled();
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="200"><text x="20 30" y="40" style="font-size: 20px; inline-size: 100px; line-height: 30px; font-family: sans-serif">one two three four five six seven eight</text></svg>"#;
        let doc = Document::parse(svg).unwrap();
        let out = wrap_inline_size(&doc, &fonts).unwrap();
        assert!(out.contains(r#"style="font-size: 20px; line-height: 30px; font-family: sans-serif""#), "{out}");
        assert!(out.contains(r#"<tspan x="20" dy="30">"#), "{out}");
    }

    #[test]
    fn a_document_without_inline_size_is_answered_unchanged_and_unknown_families_are_refused() {
        let fonts = FontSet::bundled();
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><text font-family="Inter">x</text></svg>"#;
        let doc = Document::parse(svg).unwrap();
        assert_eq!(wrap_inline_size(&doc, &fonts).unwrap(), svg);
        check_families(&doc, &fonts).unwrap();
        let bad = Document::parse(r#"<svg xmlns="http://www.w3.org/2000/svg"><g style="font-family: Fraunces"><text>x</text></g></svg>"#).unwrap();
        assert!(check_families(&bad, &fonts).unwrap_err().message.contains("Fraunces"));
    }
}

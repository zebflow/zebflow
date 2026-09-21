//! Text, made drawable: families resolved and lines wrapped, before resvg
//! sees the file.
//!
//! **Families.** usvg draws a `<text>` whose `font-family` it cannot find
//! as nothing — it never falls back for a *named* family, only for an
//! element with none. So every `font-family` (attribute or `style`) is
//! rewritten to the name the font database registers (`Inter` →
//! `Inter 24pt`), a stack to its first known family, and a family nobody
//! has is refused with the list. See [`FontSet::resolve_stack`].
//!
//! **Wrapping.** SVG 2 gives a `<text>` a width with `inline-size`; resvg
//! does not implement it, so the element becomes one `<tspan>` per line,
//! each at the element's `x`, advancing by the line height. Each candidate
//! line is measured by usvg itself — a one-element SVG with the same family,
//! weight, size and letter-spacing as the final render, its ink width read
//! back — so a line that measures as fitting renders as fitting. Font
//! properties are read from the element and its ancestors (attributes, then
//! `style="…"`); a `<style>` block is not consulted. `<tspan>` children of a
//! wrapped element are flattened into its text.
//!
//! **Shrinking.** `data-fit="shrink"` on a `<text>` that has `inline-size`
//! steps the font size down (2 px at a time, never below `data-min-size`,
//! default 8) until the text fits in `data-max-lines` lines (default 1) of
//! that width — a name on a certificate, a headline that must stay on one
//! line. The rebuilt element carries the size it landed at. `data-*`
//! attributes are SVG 2's own extension point, so no namespace declaration
//! is needed and a file that carries them is still plain SVG.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::ops::Range;

use roxmltree::{Document, Node};

use super::ConvertError;
use super::fonts::FontSet;

const DEFAULT_FONT_SIZE: f32 = 16.0;
const DEFAULT_LINE_HEIGHT: f32 = 1.2;
const DEFAULT_MIN_SIZE: f32 = 8.0;
const SHRINK_STEP: f32 = 2.0;
/// Attributes the node consumes; they do not reach resvg.
const CONSUMED: &[&str] = &["inline-size", "data-fit", "data-min-size", "data-max-lines"];

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

/// One `name: value` declaration of a `style` attribute, if present.
fn style_decl(style: &str, name: &str) -> Option<String> {
    style.split(';').find_map(|decl| {
        let (k, v) = decl.split_once(':')?;
        k.trim().eq_ignore_ascii_case(name).then(|| v.trim().to_string())
    })
}

/// The `style` with one declaration replaced (`Some`) or dropped (`None`).
fn style_with(style: &str, name: &str, value: Option<&str>) -> String {
    style
        .split(';')
        .filter(|d| !d.trim().is_empty())
        .filter_map(|d| match d.split_once(':') {
            Some((k, _)) if k.trim().eq_ignore_ascii_case(name) => value.map(|v| format!("{}: {v}", k.trim())),
            _ => Some(d.trim().to_string()),
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// The element's own value of a property, attribute or `style`, not inherited.
fn own(node: Node<'_, '_>, name: &str) -> Option<String> {
    if let Some(v) = node.attribute(name) {
        return Some(v.trim().to_string());
    }
    node.attribute("style").and_then(|style| style_decl(style, name))
}

/// A property as the element sees it: its own, else the nearest ancestor's.
fn inherited(node: Node<'_, '_>, name: &str) -> Option<String> {
    node.ancestors().filter(|n| n.is_element()).find_map(|n| own(n, name))
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

/// The document made drawable: every `font-family` resolved to a family the
/// database registers (an unknown one refused), every `<text
/// inline-size="…">` broken into lines. Answers the input unchanged when
/// nothing needs either.
pub fn prepare(doc: &Document<'_>, fonts: &FontSet) -> Result<String, ConvertError> {
    let source = doc.input_text();
    let m = TextMeasurer::new(fonts);
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();

    for node in doc.descendants().filter(|n| n.is_element()) {
        // 1. Families, on every element that names one.
        for attr in node.attributes() {
            if attr.name() == "font-family" {
                let mut v = String::new();
                escape_attr_into(&mut v, fonts.resolve_stack(attr.value())?);
                edits.push((attr.range_value(), v));
            } else if attr.name() == "style" {
                if let Some(stack) = style_decl(attr.value(), "font-family") {
                    let mut v = String::new();
                    escape_attr_into(&mut v, &style_with(attr.value(), "font-family", Some(fonts.resolve_stack(&stack)?)));
                    edits.push((attr.range_value(), v));
                }
            }
        }

        // 2. Wrapping, on a <text> with a width.
        if node.tag_name().name() != "text" {
            continue;
        }
        let Some(max_w) = own(node, "inline-size").and_then(|v| length_px(&v)).filter(|w| *w > 0.0) else {
            continue;
        };
        let text: String = node.descendants().filter(|n| n.is_text()).filter_map(|n| n.text()).collect();
        if text.trim().is_empty() {
            continue;
        }
        let family = fonts.resolve_stack(&inherited(node, "font-family").unwrap_or_else(|| fonts.default_family().to_string()))?;
        let weight = inherited(node, "font-weight").map(|w| weight_of(&w)).unwrap_or(400);
        let declared = inherited(node, "font-size").and_then(|s| length_px(&s)).unwrap_or(DEFAULT_FONT_SIZE).max(1.0);
        let letter_spacing = inherited(node, "letter-spacing").and_then(|s| length_px(&s)).unwrap_or(0.0);
        let shrink = own(node, "data-fit").is_some_and(|f| f.eq_ignore_ascii_case("shrink"));
        let (size, lines) = if shrink {
            let min = own(node, "data-min-size").and_then(|v| length_px(&v)).unwrap_or(DEFAULT_MIN_SIZE).max(1.0);
            let max_lines = own(node, "data-max-lines").and_then(|v| v.trim().parse::<usize>().ok()).unwrap_or(1).max(1);
            let mut size = declared;
            loop {
                let ls = letter_spacing * size / declared;
                let lines = wrap(&text, max_w, |t| m.width(t, family, weight, size, ls));
                let fits = lines.len() <= max_lines && lines.iter().all(|l| m.width(l, family, weight, size, ls) <= max_w);
                if fits || size - SHRINK_STEP < min {
                    break (size, lines);
                }
                size -= SHRINK_STEP;
            }
        } else {
            (declared, wrap(&text, max_w, |t| m.width(t, family, weight, declared, letter_spacing)))
        };
        let line_height = match inherited(node, "line-height") {
            Some(raw) if raw.ends_with("px") => length_px(&raw).unwrap_or(size * DEFAULT_LINE_HEIGHT),
            Some(raw) => raw.trim().parse::<f32>().map(|r| r * size).unwrap_or(size * DEFAULT_LINE_HEIGHT),
            None => size * DEFAULT_LINE_HEIGHT,
        };
        let x = node.attribute("x").and_then(|x| x.split_whitespace().next()).unwrap_or("0").to_string();

        let mut out = String::with_capacity(text.len() + 128);
        out.push_str("<text");
        let mut wrote_size = false;
        for attr in node.attributes() {
            let name = attr.name();
            if CONSUMED.contains(&name) {
                continue;
            }
            out.push(' ');
            if attr.namespace() == Some(roxmltree::NS_XML_URI) {
                out.push_str("xml:");
            }
            out.push_str(name);
            let value = match name {
                "font-family" => family.to_string(),
                "font-size" if shrink => {
                    wrote_size = true;
                    size.to_string()
                }
                "style" => {
                    let mut s = style_with(attr.value(), "inline-size", None);
                    if style_decl(&s, "font-family").is_some() {
                        s = style_with(&s, "font-family", Some(family));
                    }
                    if shrink && style_decl(&s, "font-size").is_some() {
                        wrote_size = true;
                        s = style_with(&s, "font-size", Some(&size.to_string()));
                    }
                    s
                }
                _ => attr.value().to_string(),
            };
            out.push_str("=\"");
            escape_attr_into(&mut out, &value);
            out.push('"');
        }
        if shrink && !wrote_size {
            let _ = write!(out, r#" font-size="{size}""#);
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
    // Earliest first; an edit inside an element already replaced whole (its
    // own font-family, rewritten in the rebuilt tag) is dropped.
    edits.sort_by(|(a, _), (b, _)| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));
    let mut result = String::with_capacity(source.len() + edits.len() * 64);
    let mut cursor = 0;
    for (range, replacement) in edits {
        if range.start < cursor {
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
    fn inline_size_becomes_tspans_that_each_fit_the_width_and_the_family_is_the_registered_name() {
        let fonts = FontSet::bundled();
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="1080" height="600"><g font-family="Inter" font-weight="800"><text x="540" y="200" font-size="80" text-anchor="middle" inline-size="900" fill="#fff">a chill trance night with slow builds, warm pads and a room that never rushes</text></g><text x="10" y="500" font-family="Inter, sans-serif">left alone</text></svg>"##;
        let doc = Document::parse(svg).unwrap();
        let out = prepare(&doc, &fonts).unwrap();
        assert!(!out.contains("inline-size"), "{out}");
        assert!(out.contains(r#"<g font-family="Inter 24pt" font-weight="800">"#), "{out}");
        assert!(out.contains(r#"<tspan x="540" dy="0">"#) && out.contains(r#"<tspan x="540" dy="96">"#), "{out}");
        assert!(out.contains(r#"<text x="10" y="500" font-family="Inter 24pt">left alone</text>"#), "{out}");
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
    fn style_declared_properties_are_read_rewritten_and_inline_size_dropped_from_style() {
        let fonts = FontSet::bundled();
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="200"><text x="20 30" y="40" style="font-size: 20px; inline-size: 100px; line-height: 30px; font-family: sans-serif">one two three four five six seven eight</text><rect style="fill:#000; font-family: 'Inter'"/></svg>"#;
        let doc = Document::parse(svg).unwrap();
        let out = prepare(&doc, &fonts).unwrap();
        assert!(out.contains(r#"style="font-size: 20px; line-height: 30px; font-family: Inter 24pt""#), "{out}");
        assert!(out.contains(r#"<tspan x="20" dy="30">"#), "{out}");
        assert!(out.contains(r#"<rect style="fill:#000; font-family: Inter 24pt"/>"#), "{out}");
    }

    #[test]
    fn data_fit_shrink_lands_a_long_name_on_one_line_and_writes_the_size_it_landed_at() {
        let fonts = FontSet::bundled();
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="1000" height="200"><text x="500" y="100" font-family="Inter" font-weight="600" font-size="96" text-anchor="middle" inline-size="800" data-fit="shrink" data-min-size="20">Alexandra Josephine Montgomery Whitfield</text></svg>"#;
        let doc = Document::parse(svg).unwrap();
        let out = prepare(&doc, &fonts).unwrap();
        assert!(!out.contains("data-fit") && !out.contains("inline-size"), "{out}");
        assert_eq!(out.matches("<tspan").count(), 1, "one line: {out}");
        let size: f32 = out.split("font-size=\"").nth(1).unwrap().split('"').next().unwrap().parse().unwrap();
        assert!(size < 96.0 && size >= 20.0, "size {size}");
        let m = TextMeasurer::new(&fonts);
        assert!(m.width("Alexandra Josephine Montgomery Whitfield", fonts.resolve("Inter").unwrap(), 600, size, 0.0) <= 800.0);
        assert!(m.width("Alexandra Josephine Montgomery Whitfield", fonts.resolve("Inter").unwrap(), 600, size + SHRINK_STEP, 0.0) > 800.0, "one step larger would not fit");
        // Short text keeps its size; two lines allowed shrinks less.
        let short = prepare(&Document::parse(r#"<svg xmlns="http://www.w3.org/2000/svg"><text font-size="96" inline-size="800" data-fit="shrink">Ana</text></svg>"#).unwrap(), &fonts).unwrap();
        assert!(short.contains(r#"font-size="96""#), "{short}");
        let two = prepare(&Document::parse(r#"<svg xmlns="http://www.w3.org/2000/svg"><text font-size="96" inline-size="800" data-fit="shrink" data-max-lines="2">Alexandra Josephine Montgomery Whitfield</text></svg>"#).unwrap(), &fonts).unwrap();
        let size2: f32 = two.split("font-size=\"").nth(1).unwrap().split('"').next().unwrap().parse().unwrap();
        assert!(size2 > size, "two lines shrink less: {size2} vs {size}");
    }

    #[test]
    fn a_document_naming_no_family_is_answered_unchanged_and_an_unknown_family_is_refused() {
        let fonts = FontSet::bundled();
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><text>x</text></svg>"#;
        let doc = Document::parse(svg).unwrap();
        assert_eq!(prepare(&doc, &fonts).unwrap(), svg);
        let bad = Document::parse(r#"<svg xmlns="http://www.w3.org/2000/svg"><g style="font-family: Fraunces"><text>x</text></g></svg>"#).unwrap();
        assert!(prepare(&bad, &fonts).unwrap_err().message.contains("Fraunces"));
        let bad = Document::parse(r#"<svg xmlns="http://www.w3.org/2000/svg"><text font-family="Georgia, serif">x</text></svg>"#).unwrap();
        assert!(prepare(&bad, &fonts).unwrap_err().message.contains("Georgia"));
    }
}

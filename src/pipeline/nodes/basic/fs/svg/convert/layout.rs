//! The layout report: where every text and picture landed, and what
//! collides — so a script or an agent can judge a composition without
//! looking at pixels.
//!
//! Read off the parsed tree after wrapping and shrinking, in canvas pixels:
//! each `<text>` (its first words, its box, how many lines), each `<image>`
//! (numbered in document order — usvg keeps no ids), every pair of boxes
//! that overlap with the overlap area, and every box that leaves the canvas.
//!
//! A text box is the line box, ascent plus descent, so two lines of one
//! headline at tight leading overlap by a sliver; a text–text overlap
//! thinner than a quarter of the smaller box's height is adjacent lines,
//! not a collision, and is not reported. A picture's box is the rectangle
//! actually drawn after `preserveAspectRatio`, not the declared one.

use serde_json::{Value, json};

#[derive(Debug, Clone)]
struct Item {
    label: String,
    kind: &'static str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    lines: usize,
}

fn collect(group: &usvg::Group, out: &mut Vec<Item>) {
    for node in group.children() {
        match node {
            usvg::Node::Group(g) => collect(g, out),
            usvg::Node::Text(t) => {
                let text: String = t.chunks().iter().map(|c| c.text()).collect::<Vec<_>>().join(" ");
                let words: Vec<&str> = text.split_whitespace().collect();
                let first: String = words.iter().take(4).cloned().collect::<Vec<_>>().join(" ");
                let label = format!("text \"{first}\"");
                let b = t.abs_bounding_box();
                // One chunk per line after wrapping: the node writes one <tspan x dy> per line.
                let lines = t.chunks().len().max(1);
                out.push(Item { label, kind: "text", x: b.x(), y: b.y(), w: b.width(), h: b.height(), lines });
            }
            usvg::Node::Image(i) => {
                let label = format!("image {}", out.iter().filter(|it| it.kind == "image").count() + 1);
                // The drawn rectangle: the picture's pixel size through the transform
                // usvg built from the placement and preserveAspectRatio.
                let s = i.size();
                let b = tiny_skia::Rect::from_xywh(0.0, 0.0, s.width(), s.height())
                    .and_then(|r| r.transform(i.abs_transform()))
                    .unwrap_or(i.bounding_box());
                out.push(Item { label, kind: "image", x: b.x(), y: b.y(), w: b.width(), h: b.height(), lines: 0 });
            }
            usvg::Node::Path(_) => {}
        }
    }
}

fn round(v: f32) -> f64 {
    (f64::from(v) * 10.0).round() / 10.0
}

/// The report for one parsed tree.
pub fn report(tree: &usvg::Tree) -> Value {
    let (cw, ch) = (tree.size().width(), tree.size().height());
    let mut items = Vec::new();
    collect(tree.root(), &mut items);

    let boxed = |it: &Item| json!({ "x": round(it.x), "y": round(it.y), "w": round(it.w), "h": round(it.h) });
    let texts: Vec<Value> = items
        .iter()
        .filter(|it| it.kind == "text")
        .map(|it| json!({ "label": it.label, "box": boxed(it), "lines": it.lines }))
        .collect();
    let images: Vec<Value> = items.iter().filter(|it| it.kind == "image").map(|it| json!({ "label": it.label, "box": boxed(it) })).collect();

    let mut overlaps = Vec::new();
    for (i, a) in items.iter().enumerate() {
        for b in items.iter().skip(i + 1) {
            let ox = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
            let oy = (a.y + a.h).min(b.y + b.h) - a.y.max(b.y);
            let leading = if a.kind == "text" && b.kind == "text" { 0.25 * a.h.min(b.h) } else { 0.0 };
            if ox > 0.5 && oy > 0.5 + leading {
                overlaps.push(json!({ "a": a.label, "b": b.label, "area": round(ox * oy), "w": round(ox), "h": round(oy) }));
            }
        }
    }
    let outside: Vec<Value> = items
        .iter()
        .filter(|it| it.x < -0.5 || it.y < -0.5 || it.x + it.w > cw + 0.5 || it.y + it.h > ch + 0.5)
        .map(|it| {
            let by = [
                (it.x < -0.5, "left"),
                (it.y < -0.5, "top"),
                (it.x + it.w > cw + 0.5, "right"),
                (it.y + it.h > ch + 0.5, "bottom"),
            ]
            .iter()
            .filter(|(c, _)| *c)
            .map(|(_, s)| *s)
            .collect::<Vec<_>>();
            json!({ "label": it.label, "by": by })
        })
        .collect();

    json!({
        "canvas": { "w": round(cw), "h": round(ch) },
        "texts": texts,
        "images": images,
        "overlaps": overlaps,
        "outside": outside,
        "ok": overlaps.is_empty() && outside.is_empty()
    })
}

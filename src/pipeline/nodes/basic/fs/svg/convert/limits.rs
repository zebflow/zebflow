//! What this node will not spend.
//!
//! The caps elsewhere bound *memory*: a source may be 512 KB, a canvas 40
//! megapixels, a picture 10 MB. These bound *time*, which is the harder one,
//! because nothing here can interrupt work already started. The node renders
//! on the thread that called it, so a file that takes a minute holds that
//! thread for a minute and no engine timeout can take it back. The only
//! defence is to refuse the work before beginning it.
//!
//! Two things are bounded, and both were measured rather than guessed.
//!
//! **Nesting**, because every walker over the tree recurses — roxmltree's own
//! parser first, then usvg's converter, resvg's renderer, the layout report
//! and the tree's destructor. A file 4000 groups deep overflowed the stack
//! and aborted the process, taking the whole server with it rather than
//! failing one node.
//!
//! **Filters**, because they are the one input whose cost is not bounded by
//! the canvas. A filter draws into a *region*, and that region is sized
//! relative to the element, so a rectangle covering a 1080×1350 page with a
//! 300% region is filtering thirteen megapixels. Measured on one machine,
//! rendering a page of solid colour:
//!
//! | file | time |
//! |---|---|
//! | no filter | 0.4 s |
//! | turbulence, 8 octaves, 100% region | 6.4 s |
//! | blur σ=500, 300% region | 9.0 s |
//! | turbulence, 4 octaves, 300% region | 19.9 s |
//! | turbulence, 8 octaves, 300% region | 32.3 s |
//!
//! The region dominates. Cutting it from 300% to 100% cost five times less;
//! halving the octaves only 1.6 times.
//!
//! But no single attribute is the answer, and the first version of these
//! caps proved it: four separate limits, each respected, and a file that
//! still cost 54 seconds — worse than the bomb — because ten cheap
//! primitives over a wide region multiply exactly as well as one expensive
//! one. Cost is a product of three things, so the budget is on the product:
//!
//! ```text
//! score = megapixels of page  x  (region / 100)^2  x  sum of primitive weights
//! ```
//!
//! One unit is about a third of a second of one core. The page has to be in
//! it: the same blur on a 4000x4000 canvas costs eleven times what it costs
//! on a poster, and a cap counting only the region and the chain would wave
//! it through.
//!
//! Every cap **refuses and says so**, naming the attribute and the limit. It
//! would be easy to clamp instead and render something, but that is a silent
//! change to what an author asked for, and they would never learn why their
//! glow looks different.

use roxmltree::{Document, Node};

use super::ConvertError;

/// How deeply elements may nest.
///
/// Real drawings nest tens of levels; a hundred is already generous.
pub const MAX_DEPTH: usize = 100;

/// How large a filter region may be, as a percentage of the element it
/// filters. The SVG default is 120%; a generous drop shadow wants about 200%.
pub const MAX_FILTER_REGION_PERCENT: f32 = 250.0;

/// Octaves of `feTurbulence`. Each one doubles the sampling.
pub const MAX_TURBULENCE_OCTAVES: u32 = 4;

/// The largest `stdDeviation` a blur may use, in user units.
pub const MAX_BLUR_DEVIATION: f32 = 100.0;

/// The most filter primitives one `<filter>` may chain.
pub const MAX_FILTER_PRIMITIVES: usize = 12;

/// What one filter may cost.
///
/// One unit is a single pass over one megapixel, and measures at roughly a
/// third of a second of one core. Thirty is about eleven seconds for the
/// very worst file the caps admit, which is under the engine's own node
/// timeout and five times better than the fifty-four seconds the first
/// version of these caps still allowed.
pub const MAX_FILTER_SCORE: f32 = 30.0;

/// What a whole document may cost. One filter at its limit is allowed; five
/// ordinary ones together are not, because every filtered element pays its
/// filter's price again.
pub const MAX_DOCUMENT_FILTER_SCORE: f32 = 70.0;

/// What one primitive costs, relative to a single pass over the region.
///
/// Measured, not assumed. The caps that came before this were four separate
/// limits, and a file that respected every one of them still cost 54 seconds
/// because ten cheap primitives over a wide region multiply just as well as
/// one expensive primitive does. Cost is a product, so the budget is on the
/// product.
fn primitive_weight(node: &Node<'_, '_>) -> f32 {
    match node.tag_name().name() {
        "feTurbulence" => node
            .attribute("numOctaves")
            .and_then(|v| v.trim().parse::<f32>().ok())
            .unwrap_or(1.0)
            .clamp(1.0, 32.0),
        "feGaussianBlur" | "feDropShadow" | "feImage" | "feDisplacementMap" | "feConvolveMatrix" => 3.0,
        "feMorphology" | "feSpecularLighting" | "feDiffuseLighting" => 2.0,
        _ => 1.0,
    }
}

fn find_from(haystack: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    if from >= haystack.len() {
        return None;
    }
    haystack[from..].windows(needle.len()).position(|w| w == needle).map(|i| i + from)
}

/// Refuses a source that nests elements deeper than [`MAX_DEPTH`].
///
/// This reads the **text**, before any parser sees it, because roxmltree's
/// parser is itself recursive: a check after parsing would never run.
/// Comments, CDATA, processing instructions and declarations are skipped,
/// quoted attribute values are stepped over so a `>` inside one does not end
/// a tag, and a self-closing tag opens and closes in one step.
pub fn check_depth(svg: &str) -> Result<(), ConvertError> {
    let b = svg.as_bytes();
    let mut i = 0usize;
    let mut depth: usize = 0;
    while i < b.len() {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        let rest = &b[i..];
        if rest.starts_with(b"<!--") {
            match find_from(b, i + 4, b"-->") {
                Some(j) => {
                    i = j + 3;
                    continue;
                }
                None => break,
            }
        }
        if rest.starts_with(b"<![CDATA[") {
            match find_from(b, i + 9, b"]]>") {
                Some(j) => {
                    i = j + 3;
                    continue;
                }
                None => break,
            }
        }
        let declaration = rest.starts_with(b"<?") || rest.starts_with(b"<!");
        let closing = rest.starts_with(b"</");

        let mut j = i + 1;
        let mut quote = 0u8;
        let mut self_closing = false;
        while j < b.len() {
            let c = b[j];
            if quote != 0 {
                if c == quote {
                    quote = 0;
                }
            } else if c == b'"' || c == b'\'' {
                quote = c;
            } else if c == b'>' {
                self_closing = b[j - 1] == b'/';
                break;
            }
            j += 1;
        }
        if !declaration {
            if closing {
                depth = depth.saturating_sub(1);
            } else {
                depth += 1;
                if depth > MAX_DEPTH {
                    return Err(ConvertError::source(format!(
                        "the svg nests elements more than {MAX_DEPTH} deep; flatten the groups"
                    )));
                }
                if self_closing {
                    depth -= 1;
                }
            }
        }
        i = j + 1;
    }
    Ok(())
}

/// `"300%"` → 300, `"3"` → 300 (a bare number in objectBoundingBox units is
/// a fraction of the element), and anything unparseable → `None`.
fn region_percent(raw: &str, user_space: bool, canvas: f32) -> Option<f32> {
    let raw = raw.trim();
    if let Some(v) = raw.strip_suffix('%') {
        return v.trim().parse::<f32>().ok().map(f32::abs);
    }
    let v = raw.parse::<f32>().ok()?.abs();
    if user_space {
        // An absolute length: what fraction of the canvas does it cover?
        if canvas <= 0.0 { None } else { Some(v / canvas * 100.0) }
    } else {
        Some(v * 100.0)
    }
}

/// Refuses filters that would cost unbounded time.
///
/// `canvas` is the page in user units, used only to read a filter region
/// given in absolute lengths rather than percentages.
pub fn check_filters(doc: &Document<'_>, canvas: (f32, f32)) -> Result<(), ConvertError> {
    let mut total = 0.0_f32;
    for filter in doc.descendants().filter(|n| n.is_element() && n.tag_name().name() == "filter") {
        let user_space = filter.attribute("filterUnits") == Some("userSpaceOnUse");
        let mut region = 1.0_f32;
        for (attr, canvas_side, axis) in [("width", canvas.0, "width"), ("height", canvas.1, "height")] {
            if let Some(raw) = filter.attribute(attr) {
                if let Some(percent) = region_percent(raw, user_space, canvas_side) {
                    if percent > MAX_FILTER_REGION_PERCENT {
                        return Err(ConvertError::source(format!(
                            "filter{} asks for a {axis} of {raw}, about {percent:.0}% of what it filters; the limit is {MAX_FILTER_REGION_PERCENT:.0}%. \
                             A filter region is rendered in full before it is clipped, so a large one costs time no timeout can reclaim — \
                             shrink the region, or apply the filter to a smaller element",
                            named(&filter)
                        )));
                    }
                    region = region.max(percent / 100.0);
                }
            }
        }

        let primitives: Vec<Node<'_, '_>> = filter
            .children()
            .filter(|n| n.is_element() && n.tag_name().name().starts_with("fe"))
            .collect();
        if primitives.len() > MAX_FILTER_PRIMITIVES {
            return Err(ConvertError::source(format!(
                "filter{} chains {} primitives; the limit is {MAX_FILTER_PRIMITIVES}",
                named(&filter),
                primitives.len()
            )));
        }

        for p in &primitives {
            match p.tag_name().name() {
                "feTurbulence" => {
                    let octaves = p
                        .attribute("numOctaves")
                        .and_then(|v| v.trim().parse::<f32>().ok())
                        .map(|v| v.max(0.0) as u32)
                        .unwrap_or(1);
                    if octaves > MAX_TURBULENCE_OCTAVES {
                        return Err(ConvertError::source(format!(
                            "filter{} uses feTurbulence with {octaves} octaves; the limit is {MAX_TURBULENCE_OCTAVES}. \
                             Each octave doubles the sampling and past four the difference is hard to see",
                            named(&filter)
                        )));
                    }
                }
                "feGaussianBlur" | "feDropShadow" => {
                    if let Some(raw) = p.attribute("stdDeviation") {
                        let worst = raw
                            .split(|c: char| c == ',' || c.is_whitespace())
                            .filter(|s| !s.is_empty())
                            .filter_map(|s| s.parse::<f32>().ok())
                            .fold(0.0_f32, |a, b| a.max(b.abs()));
                        if worst > MAX_BLUR_DEVIATION {
                            return Err(ConvertError::source(format!(
                                "filter{} blurs with stdDeviation {worst}; the limit is {MAX_BLUR_DEVIATION}",
                                named(&filter)
                            )));
                        }
                    }
                }
                _ => {}
            }
        }

        // The product. Three things multiply and all three must be in it:
        // the page's own pixels, the square of the region drawn around each
        // filtered element, and one pass per primitive. Leaving the page out
        // was a hole — the same blur on a 4000x4000 canvas costs eleven
        // times what it costs on a poster, and would have passed a cap that
        // counted only the last two.
        let work: f32 = primitives.iter().map(primitive_weight).sum();
        let megapixels = (canvas.0.max(1.0) * canvas.1.max(1.0) / 1_000_000.0).max(0.01);
        let score = megapixels * region * region * work.max(1.0);
        if score > MAX_FILTER_SCORE {
            return Err(ConvertError::source(format!(
                "filter{} costs about {score:.0} units and the limit is {MAX_FILTER_SCORE:.0}: {:.1} megapixels of page, \
                 a region {:.0}% wide covering {:.1}x the area, and {} primitive(s) each crossing it. \
                 Narrow the region, shorten the chain, or draw a smaller page",
                named(&filter),
                megapixels,
                region * 100.0,
                region * region,
                primitives.len()
            )));
        }
        total += score;
    }

    if total > MAX_DOCUMENT_FILTER_SCORE {
        return Err(ConvertError::source(format!(
            "the filters in this svg cost about {total:.0} units together and the limit is {MAX_DOCUMENT_FILTER_SCORE:.0}; \
             every filtered element pays its filter's price again"
        )));
    }
    Ok(())
}

/// ` #hero`, or nothing when the element has no id, so a refusal can point
/// at the filter without inventing a name for it.
fn named(filter: &Node<'_, '_>) -> String {
    match filter.attribute("id") {
        Some(id) if !id.is_empty() => format!(" #{id}"),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(svg: &str) -> Document<'_> {
        Document::parse(svg).expect("fixture parses")
    }

    #[test]
    fn nesting_is_counted_off_the_text_before_any_parser_sees_it() {
        // roxmltree's own parser overflows the stack on this, so the check
        // must come first. It answers in milliseconds.
        let deep = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\">{}<rect/>{}</svg>",
            "<g>".repeat(4000),
            "</g>".repeat(4000)
        );
        let started = std::time::Instant::now();
        assert!(check_depth(&deep).unwrap_err().message.contains("deep"));
        assert!(started.elapsed().as_millis() < 500, "{:?}", started.elapsed());

        // `<svg>` is itself depth 1, so MAX_DEPTH - 2 groups put the `<rect/>`
        // exactly at the limit. One more group is one too many.
        let at = format!("<svg>{}<rect/>{}</svg>", "<g>".repeat(MAX_DEPTH - 2), "</g>".repeat(MAX_DEPTH - 2));
        assert!(check_depth(&at).is_ok());
        let past = format!("<svg>{}<rect/>{}</svg>", "<g>".repeat(MAX_DEPTH - 1), "</g>".repeat(MAX_DEPTH - 1));
        assert!(check_depth(&past).is_err());

        // Self-closing tags do not accumulate depth, however many there are.
        let flat = format!("<svg>{}</svg>", "<rect/>".repeat(5000));
        assert!(check_depth(&flat).is_ok());

        // A `>` inside an attribute, a comment, CDATA and a declaration are
        // not tags: none of them counts as nesting.
        let tricky = "<?xml version=\"1.0\"?><!DOCTYPE svg><svg><!-- <g><g><g> --><text data-x=\"a>b\">x</text><![CDATA[<g><g>]]></svg>";
        assert!(check_depth(tricky).is_ok());
    }

    #[test]
    fn an_ordinary_drop_shadow_passes_untouched() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="1080" height="1350">
            <filter id="shadow" x="-20%" y="-20%" width="140%" height="140%">
                <feDropShadow dx="0" dy="12" stdDeviation="8"/>
            </filter>
            <filter id="glow"><feGaussianBlur stdDeviation="30"/><feTurbulence numOctaves="3"/></filter>
            <rect width="1080" height="1350" filter="url(#shadow)"/>
        </svg>"##;
        assert!(check_filters(&doc(svg), (1080.0, 1350.0)).is_ok());
    }

    #[test]
    fn the_region_is_capped_and_the_refusal_names_the_filter_and_the_limit() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="1080" height="1350">
            <filter id="bomb" x="-100%" y="-100%" width="300%" height="300%">
                <feTurbulence baseFrequency="0.002" numOctaves="8"/>
            </filter>
            <rect width="1080" height="1350" filter="url(#bomb)"/>
        </svg>"##;
        let err = check_filters(&doc(svg), (1080.0, 1350.0)).unwrap_err();
        assert_eq!(err.kind, super::super::ConvertErrorKind::Source);
        assert!(err.message.contains("#bomb"), "{}", err.message);
        assert!(err.message.contains("300%") && err.message.contains("250%"), "{}", err.message);
    }

    #[test]
    fn octaves_a_huge_blur_and_a_long_chain_are_each_refused_by_name() {
        let with = |body: &str| {
            format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><filter id="f">{body}</filter></svg>"##
            )
        };
        let svg = with(r##"<feTurbulence numOctaves="8"/>"##);
        assert!(check_filters(&doc(&svg), (100.0, 100.0)).unwrap_err().message.contains("octaves"));

        let svg = with(r##"<feGaussianBlur stdDeviation="500"/>"##);
        assert!(check_filters(&doc(&svg), (100.0, 100.0)).unwrap_err().message.contains("stdDeviation"));

        // A two-axis deviation is judged by its larger side.
        let svg = with(r##"<feGaussianBlur stdDeviation="2 400"/>"##);
        assert!(check_filters(&doc(&svg), (100.0, 100.0)).unwrap_err().message.contains("stdDeviation"));

        let chain = "<feGaussianBlur stdDeviation=\"1\"/>".repeat(MAX_FILTER_PRIMITIVES + 1);
        assert!(check_filters(&doc(&with(&chain)), (100.0, 100.0)).unwrap_err().message.contains("primitives"));
    }

    #[test]
    fn a_region_in_absolute_units_is_measured_against_the_canvas() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="1000" height="1000">
            <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="5000" height="5000">
                <feGaussianBlur stdDeviation="4"/>
            </filter>
        </svg>"##;
        let err = check_filters(&doc(svg), (1000.0, 1000.0)).unwrap_err();
        assert!(err.message.contains("500%"), "{}", err.message);

        // The same absolute numbers on a page that size are an ordinary
        // 100% region — but 25 megapixels of page is not ordinary, and the
        // score says so. This is the hole the per-element cap had.
        let err = check_filters(&doc(svg), (5000.0, 5000.0)).unwrap_err();
        assert!(err.message.contains("megapixels"), "{}", err.message);
    }

    #[test]
    fn the_page_is_part_of_the_price() {
        let blur = r##"<svg xmlns="http://www.w3.org/2000/svg">
            <filter id="f" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="20"/></filter>
        </svg>"##;
        // A poster affords a wide glow.
        assert!(check_filters(&doc(blur), (1080.0, 1350.0)).is_ok());
        // An icon affords far more.
        assert!(check_filters(&doc(blur), (128.0, 128.0)).is_ok());
        // A wall-sized canvas affords none of it, though nothing about the
        // filter changed.
        let err = check_filters(&doc(blur), (4000.0, 4000.0)).unwrap_err();
        assert!(err.message.contains("costs about"), "{}", err.message);
    }
}

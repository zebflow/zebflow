//! The conversion end to end: SVG text in, pixels out, every refusal named.

pub(crate) mod test_support {
    use std::io::Cursor;

    /// A solid PNG of one colour, for fixtures — no network, no files.
    pub fn solid_png(w: u32, h: u32, rgb: [u8; 3]) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(w, h, image::Rgb(rgb));
        let mut out = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img).write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }
}

mod convert_end_to_end {
    use std::collections::HashMap;
    use std::sync::Arc;

    use super::super::*;
    use super::test_support;

    fn resolver_with(files: Vec<(Source, Vec<u8>)>) -> Resolver {
        Resolver::new(Arc::new(MemoryStore(files.into_iter().collect::<HashMap<_, _>>())))
    }

    fn png_size(bytes: &[u8]) -> (u32, u32) {
        image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().unwrap().into_dimensions().unwrap()
    }

    const POSTER: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="1080" height="1350">
  <defs><linearGradient id="bg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#012169"/><stop offset="1" stop-color="#274d9b"/></linearGradient></defs>
  <rect width="1080" height="1350" fill="url(#bg)"/>
  <text x="540" y="500" font-family="Inter" font-weight="800" font-size="120" letter-spacing="4" fill="#ffffff" text-anchor="middle" inline-size="918">RESEARCH SHOWCASE NIGHT</text>
  <line x1="400" y1="820" x2="680" y2="820" stroke="#c8102e" stroke-width="3"/>
  <text x="540" y="900" font-family="Inter" font-size="40" fill="#ffffff" text-anchor="middle">Thu 6 Nov 2026 · 7pm</text>
</svg>"##;

    #[test]
    fn a_portrait_svg_renders_to_a_png_of_its_own_size_in_every_format() {
        let fonts = FontSet::bundled();
        let resolver = resolver_with(vec![]);
        let started = std::time::Instant::now();
        let out = convert(POSTER, &fonts, &resolver, &Target::default(), OutputFormat::Png, 82).expect("converts");
        eprintln!("poster 1080x1350: {:?} ({:?})", started.elapsed(), out.timing);
        assert_eq!((out.width, out.height), (1080, 1350));
        assert_eq!(png_size(&out.bytes), (1080, 1350));
        let jpg = convert(POSTER, &fonts, &resolver, &Target::default(), OutputFormat::Jpg, 80).unwrap();
        assert!(jpg.bytes.starts_with(&[0xFF, 0xD8]));
        let webp = convert(POSTER, &fonts, &resolver, &Target::default(), OutputFormat::Webp, 80).unwrap();
        assert!(&webp.bytes[8..12] == b"WEBP");
    }

    #[test]
    fn width_height_and_fit_size_the_canvas() {
        let fonts = FontSet::bundled();
        let resolver = resolver_with(vec![]);
        let at = |w, h, fit| convert(POSTER, &fonts, &resolver, &Target { width: w, height: h, fit }, OutputFormat::Png, 82).unwrap();
        assert_eq!(png_size(&at(Some(540), None, Fit::Cover).bytes), (540, 675));
        assert_eq!(png_size(&at(Some(500), Some(500), Fit::Contain).bytes), (400, 500));
        assert_eq!(png_size(&at(Some(500), Some(500), Fit::Fill).bytes), (500, 500));
        let cover = at(Some(500), Some(500), Fit::Cover);
        assert_eq!(png_size(&cover.bytes), (500, 500));
        // Cover fills the canvas: the corners carry the gradient, not transparency.
        let img = image::load_from_memory(&cover.bytes).unwrap().to_rgba8();
        assert_eq!(img.get_pixel(1, 1).0[3], 255);
        assert_eq!(img.get_pixel(498, 498).0[3], 255);
    }

    #[test]
    fn a_picture_from_the_store_draws_and_a_url_or_an_escape_is_refused_by_href() {
        let fonts = FontSet::bundled();
        let png = test_support::solid_png(4, 4, [0, 0, 255]);
        let resolver = resolver_with(vec![(Source::Store("sandbox/photos/hero.png".into()), png)]);
        let svg = |href: &str| {
            format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200"><image href="{href}" x="0" y="0" width="200" height="100" preserveAspectRatio="xMidYMid slice"/></svg>"#
            )
        };
        let out = convert(&svg("sandbox/photos/hero.png"), &fonts, &resolver, &Target::default(), OutputFormat::Png, 82).unwrap();
        let img = image::load_from_memory(&out.bytes).unwrap().to_rgba8();
        assert_eq!(img.get_pixel(100, 50).0, [0, 0, 255, 255]);
        assert_eq!(img.get_pixel(100, 150).0[3], 0, "below the picture is transparent");
        for (href, needle) in [
            ("https://cdn.test/a.png", "URL"),
            ("data:image/png;base64,AAAA", "data URI"),
            ("../../etc/passwd", "inside the store"),
            ("sandbox/photos/missing.png", "not found"),
        ] {
            let err = convert(&svg(href), &fonts, &resolver, &Target::default(), OutputFormat::Png, 82).unwrap_err();
            assert_eq!(err.kind, ConvertErrorKind::Source, "{href}");
            assert!(err.message.contains(needle), "{href}: {}", err.message);
        }
    }

    #[test]
    fn a_repo_static_svg_draws_as_a_picture_with_the_same_fonts() {
        let fonts = FontSet::bundled();
        let logo = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 20"><rect width="100" height="20" fill="#ff0000"/></svg>"##;
        let resolver = resolver_with(vec![(Source::Repo("static/pwa/logo.svg".into()), logo.as_bytes().to_vec())]);
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200"><image href="repo://static/pwa/logo.svg" x="0" y="0" width="200" height="40"/></svg>"#;
        let out = convert(svg, &fonts, &resolver, &Target::default(), OutputFormat::Png, 82).unwrap();
        let img = image::load_from_memory(&out.bytes).unwrap().to_rgba8();
        assert_eq!(img.get_pixel(100, 20).0, [255, 0, 0, 255]);
    }

    #[test]
    fn an_unknown_font_a_non_svg_and_a_sizeless_svg_are_refused_before_anything_is_drawn() {
        let fonts = FontSet::bundled();
        let resolver = resolver_with(vec![]);
        let err = convert(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><text font-family="Fraunces">x</text></svg>"#,
            &fonts, &resolver, &Target::default(), OutputFormat::Png, 82,
        )
        .unwrap_err();
        assert_eq!(err.kind, ConvertErrorKind::Font);
        assert!(err.message.contains("Fraunces") && err.message.contains("Inter"), "{}", err.message);
        assert!(convert("{\"not\": \"svg\"}", &fonts, &resolver, &Target::default(), OutputFormat::Png, 82).unwrap_err().message.contains("not an SVG"));
        assert!(convert("<svg xmlns=\"http://www.w3.org/2000/svg\"><rect/>", &fonts, &resolver, &Target::default(), OutputFormat::Png, 82).unwrap_err().message.contains("parse"));
        // No width, height or viewBox: usvg sizes the canvas by the content's box.
        let out = convert("<svg xmlns=\"http://www.w3.org/2000/svg\"><rect width=\"1\" height=\"1\"/></svg>", &fonts, &resolver, &Target::default(), OutputFormat::Png, 82).unwrap();
        assert_eq!((out.width, out.height), (1, 1));
    }
}

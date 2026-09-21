//! Where the pictures in an SVG come from — the project only — and the one
//! resolver that hands their bytes to resvg.
//!
//! An `<image href>` names a store object (`sandbox/posters/photos/venue.jpg`,
//! or the same with `zebfs://` in front) or a repository file under
//! `static/` (`repo://static/brand/logo.svg`). A URL, a `data:` URI, an
//! absolute path or a `..` segment is refused before anything is drawn, and
//! usvg asks [`Resolver`] for every href at render time from the same cache,
//! so a render never reads anything the check did not.

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Arc, Mutex};

use usvg::ImageKind;

use super::ConvertError;

/// A source file may be this big.
pub const MAX_SOURCE_BYTES: usize = 10 * 1024 * 1024;
/// A decoded raster may have this many pixels.
pub const MAX_SOURCE_PIXELS: u64 = 40_000_000;
/// The SVG being converted, or one drawn as a picture, may be this big.
pub const MAX_SVG_BYTES: usize = 512 * 1024;

const REPO_STATIC_PREFIX: &str = "static/";
const XLINK_NS: &str = "http://www.w3.org/1999/xlink";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Source {
    /// A repo path under `static/` — a brand asset.
    Repo(String),
    /// A project store path — an upload, a generated file.
    Store(String),
}

impl Source {
    /// One `href`, read. `Ok(None)` is a local reference (`#id`) that names
    /// no file; `Err` is a picture the node will not fetch, and says why.
    pub fn parse_href(href: &str) -> Result<Option<Self>, ConvertError> {
        let href = href.trim();
        if href.is_empty() || href.starts_with('#') {
            return Ok(None);
        }
        let refuse = |why: &str| {
            ConvertError::source(format!(
                "picture '{}' {why} — a picture is a store path (sandbox/photos/a.jpg) or a repository file under static/ (repo://static/logo.svg)",
                shorten(href)
            ))
        };
        if href.starts_with("data:") {
            return Err(refuse("is a data URI; store the file and name its path"));
        }
        if let Some((scheme, rest)) = href.split_once("://") {
            return match scheme {
                "repo" if rest.starts_with(REPO_STATIC_PREFIX) && clean_path(rest) => Ok(Some(Source::Repo(rest.to_string()))),
                "repo" => Err(refuse("names a repository file outside static/")),
                "zebfs" if clean_path(rest) => Ok(Some(Source::Store(rest.to_string()))),
                "zebfs" => Err(refuse("must stay inside the store: no `..`, no absolute path")),
                _ => Err(refuse("is a URL; fetch it with http.request --response-type bytes, fs.save it, then name the store path")),
            };
        }
        if href.starts_with("//") {
            return Err(refuse("is a URL"));
        }
        let path = href.trim_start_matches("./");
        if !clean_path(path) {
            return Err(refuse("must stay inside the store: no `..`, no absolute path"));
        }
        Ok(Some(Source::Store(path.to_string())))
    }

    pub fn is_svg(&self) -> bool {
        let path = match self {
            Source::Repo(p) | Source::Store(p) => p,
        };
        path.rsplit('.').next().is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
    }

    pub fn describe(&self) -> String {
        match self {
            Source::Repo(p) => format!("repo file {p}"),
            Source::Store(p) => format!("store object {p}"),
        }
    }
}

fn shorten(href: &str) -> String {
    if href.chars().count() > 60 { format!("{}…", href.chars().take(60).collect::<String>()) } else { href.to_string() }
}

/// No `..`, no leading `/`, no drive, no backslash, no empty segment.
fn clean_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && path.split('/').all(|seg| !seg.is_empty() && seg != "..")
}

/// Every `href` (or `xlink:href`) on an `<image>` in the document, in order.
pub fn image_hrefs(doc: &roxmltree::Document<'_>) -> Vec<String> {
    doc.descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "image")
        .filter_map(|n| n.attribute("href").or_else(|| n.attribute((XLINK_NS, "href"))).map(str::to_string))
        .collect()
}

/// The project's bytes behind a [`Source`]. The node hands the resolver one
/// backed by the platform; tests hand it a map.
pub trait SourceStore: Send + Sync {
    fn read(&self, source: &Source) -> Result<Vec<u8>, ConvertError>;
}

/// A store over an in-memory map, for tests and the schema examples.
pub struct MemoryStore(pub HashMap<Source, Vec<u8>>);

impl SourceStore for MemoryStore {
    fn read(&self, source: &Source) -> Result<Vec<u8>, ConvertError> {
        self.0.get(source).cloned().ok_or_else(|| ConvertError::source(format!("{} not found", source.describe())))
    }
}

/// What the resolver learned about one source.
#[derive(Debug, Clone)]
pub enum Asset {
    Raster { bytes: Arc<Vec<u8>>, format: image::ImageFormat, width: u32, height: u32 },
    Svg { bytes: Arc<Vec<u8>> },
}

/// The one gate between an SVG and the project's bytes. `prefetch` runs
/// before the render so a refusal names the href; `image_kind` answers usvg
/// at render time from the same cache.
pub struct Resolver {
    store: Arc<dyn SourceStore>,
    cache: Mutex<HashMap<Source, Asset>>,
}

impl Resolver {
    pub fn new(store: Arc<dyn SourceStore>) -> Self {
        Self { store, cache: Mutex::new(HashMap::new()) }
    }

    /// Reads, checks and caches one source.
    pub fn prefetch(&self, source: &Source) -> Result<Asset, ConvertError> {
        if let Some(hit) = self.cache.lock().unwrap_or_else(|e| e.into_inner()).get(source) {
            return Ok(hit.clone());
        }
        let bytes = self.store.read(source)?;
        if bytes.len() > MAX_SOURCE_BYTES {
            return Err(ConvertError::source(format!(
                "{} is {} bytes; the limit is {MAX_SOURCE_BYTES}",
                source.describe(),
                bytes.len()
            )));
        }
        let asset = if source.is_svg() || looks_like_svg(&bytes) {
            if bytes.len() > MAX_SVG_BYTES {
                return Err(ConvertError::source(format!(
                    "{} is {} bytes; an SVG picture may be {MAX_SVG_BYTES}",
                    source.describe(),
                    bytes.len()
                )));
            }
            Asset::Svg { bytes: Arc::new(bytes) }
        } else {
            let reader = image::ImageReader::new(Cursor::new(&bytes))
                .with_guessed_format()
                .map_err(|e| ConvertError::source(format!("{}: {e}", source.describe())))?;
            let format = reader
                .format()
                .ok_or_else(|| ConvertError::source(format!("{} is not a PNG, JPEG, WebP, GIF or SVG", source.describe())))?;
            if !matches!(
                format,
                image::ImageFormat::Png | image::ImageFormat::Jpeg | image::ImageFormat::WebP | image::ImageFormat::Gif
            ) {
                return Err(ConvertError::source(format!(
                    "{} is {format:?}; use PNG, JPEG, WebP, GIF or SVG",
                    source.describe()
                )));
            }
            let (width, height) =
                reader.into_dimensions().map_err(|e| ConvertError::source(format!("{}: {e}", source.describe())))?;
            if u64::from(width) * u64::from(height) > MAX_SOURCE_PIXELS {
                return Err(ConvertError::source(format!(
                    "{} is {width}×{height}; a raster may have {MAX_SOURCE_PIXELS} pixels",
                    source.describe()
                )));
            }
            Asset::Raster { bytes: Arc::new(bytes), format, width, height }
        };
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).insert(source.clone(), asset.clone());
        Ok(asset)
    }

    /// usvg's `resolve_string`: the project's paths only, from the cache only.
    pub fn image_kind(&self, href: &str, opt: &usvg::Options) -> Option<ImageKind> {
        let source = Source::parse_href(href).ok().flatten()?;
        let asset = self.cache.lock().unwrap_or_else(|e| e.into_inner()).get(&source).cloned()?;
        match asset {
            Asset::Raster { bytes, format, .. } => Some(match format {
                image::ImageFormat::Png => ImageKind::PNG(bytes),
                image::ImageFormat::Jpeg => ImageKind::JPEG(bytes),
                image::ImageFormat::WebP => ImageKind::WEBP(bytes),
                image::ImageFormat::Gif => ImageKind::GIF(bytes),
                _ => return None,
            }),
            Asset::Svg { bytes } => {
                // A nested picture shapes text with the same fonts and may
                // load no pictures of its own: the tree stops here.
                let sub = usvg::Options {
                    fontdb: opt.fontdb.clone(),
                    font_family: opt.font_family.clone(),
                    image_href_resolver: usvg::ImageHrefResolver {
                        resolve_data: Box::new(|_, _, _| None),
                        resolve_string: Box::new(|_, _| None),
                    },
                    ..Default::default()
                };
                usvg::Tree::from_data(&bytes, &sub).ok().map(ImageKind::SVG)
            }
        }
    }

    /// The `usvg::Options` for a render: the font set's, with this resolver
    /// answering every href and `data:` URIs refused.
    pub fn options<'a>(&'a self, base: usvg::Options<'a>) -> usvg::Options<'a> {
        usvg::Options {
            image_href_resolver: usvg::ImageHrefResolver {
                resolve_data: Box::new(|_, _, _| None),
                resolve_string: Box::new(move |href, opt| self.image_kind(href, opt)),
            },
            ..base
        }
    }
}

/// Is this XML with an `<svg>` in it? Markup that opens with the element
/// itself, an XML declaration, a comment or a doctype all count — real files
/// from a drawing program lead with any of the four — while JSON, plain text
/// and a raster's magic bytes do not.
/// A doctype with no internal subset, removed; one with a subset, refused.
///
/// roxmltree refuses every DTD, which is the right instinct — an internal
/// subset is the only place an XML entity can be declared, and entities are
/// how an SVG reads `/etc/passwd` or expands to a gigabyte. But the standard
/// SVG 1.1 doctype every drawing program writes declares nothing at all:
///
/// ```text
/// <!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "…/svg11.dtd">
/// ```
///
/// Refusing that would reject most files saved by Illustrator or Inkscape.
/// So a doctype carrying `[` is refused by name, and one without is cut out
/// before the parser sees it. The external identifier is never fetched: it
/// is dropped, not resolved.
pub fn strip_safe_doctype(svg: &str) -> Result<std::borrow::Cow<'_, str>, ConvertError> {
    let start = match svg.find("<!DOCTYPE") {
        Some(i) => i,
        None => return Ok(std::borrow::Cow::Borrowed(svg)),
    };
    // Only a doctype in the prologue counts; after the root element it is
    // not a doctype at all, and the parser will say so.
    if svg[..start].contains("<svg") {
        return Ok(std::borrow::Cow::Borrowed(svg));
    }
    let mut quote: Option<char> = None;
    for (offset, ch) in svg[start..].char_indices() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"') | (None, '\'') => quote = Some(ch),
            (None, '[') => {
                return Err(ConvertError::source(
                    "the svg declares an internal DTD subset, which is where XML entities live; remove the <!DOCTYPE …[…]> block (a plain SVG 1.1 doctype is fine)",
                ));
            }
            (None, '>') => {
                let mut out = String::with_capacity(svg.len());
                out.push_str(&svg[..start]);
                out.push_str(&svg[start + offset + 1..]);
                return Ok(std::borrow::Cow::Owned(out));
            }
            _ => {}
        }
    }
    Err(ConvertError::source("the svg has an unterminated <!DOCTYPE"))
}

pub fn looks_like_svg(bytes: &[u8]) -> bool {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(8192)]);
    let head = head.trim_start();
    head.starts_with('<') && head.contains("<svg")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_doctype_is_cut_out_and_one_with_an_internal_subset_is_refused() {
        let illustrator = "<?xml version=\"1.0\"?>\n<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\">\n<svg xmlns=\"http://www.w3.org/2000/svg\"/>";
        let out = strip_safe_doctype(illustrator).unwrap();
        assert!(!out.contains("DOCTYPE") && out.contains("<svg"), "{out}");
        // A `>` inside the quoted public identifier does not end it early.
        let tricky = "<!DOCTYPE svg PUBLIC \"a>b\" \"c>d\"><svg/>";
        assert_eq!(strip_safe_doctype(tricky).unwrap(), "<svg/>");
        // An internal subset is refused by name, whatever it holds.
        for bad in [
            "<!DOCTYPE svg [<!ENTITY x SYSTEM \"file:///etc/passwd\">]><svg/>",
            "<!DOCTYPE svg [ ]><svg/>",
        ] {
            assert!(strip_safe_doctype(bad).unwrap_err().message.contains("internal DTD subset"), "{bad}");
        }
        // No doctype: borrowed, untouched.
        assert_eq!(strip_safe_doctype("<svg/>").unwrap(), "<svg/>");
        // The word inside the drawing is not a prologue doctype.
        let inner = "<svg><text>&lt;!DOCTYPE svg [x]&gt;</text></svg>";
        assert_eq!(strip_safe_doctype(inner).unwrap(), inner);
    }

    #[test]
    fn the_four_ways_a_real_svg_file_opens_are_all_recognised() {
        for good in [
            "<svg xmlns=\"http://www.w3.org/2000/svg\"/>",
            "<?xml version=\"1.0\"?><svg/>",
            "<!-- Generator: Illustrator --><svg/>",
            "<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\"><svg/>",
            "\n  <svg/>",
        ] {
            assert!(looks_like_svg(good.as_bytes()), "{good}");
        }
        for bad in ["{\"svg\": \"x\"}", "plain text", "<html><body/></html>", "\u{89}PNG\r\n"] {
            assert!(!looks_like_svg(bad.as_bytes()), "{bad}");
        }
    }

    #[test]
    fn a_store_path_a_zebfs_href_and_a_repo_static_file_parse_and_the_rest_is_refused() {
        assert_eq!(Source::parse_href("sandbox/photos/a.jpg").unwrap(), Some(Source::Store("sandbox/photos/a.jpg".into())));
        assert_eq!(Source::parse_href("./generated/a.png").unwrap(), Some(Source::Store("generated/a.png".into())));
        assert_eq!(Source::parse_href("zebfs://up/b.jpg").unwrap(), Some(Source::Store("up/b.jpg".into())));
        assert_eq!(Source::parse_href("repo://static/pwa/logo.svg").unwrap(), Some(Source::Repo("static/pwa/logo.svg".into())));
        assert_eq!(Source::parse_href("#grad").unwrap(), None);
        for (bad, needle) in [
            ("https://x.test/a.png", "URL"),
            ("data:image/png;base64,AAAA", "data URI"),
            ("../secret.png", "inside the store"),
            ("/etc/passwd", "inside the store"),
            ("zebfs://../x", "inside the store"),
            ("repo://pages/home.tsx", "outside static/"),
            ("file:///etc/passwd", "URL"),
        ] {
            let err = Source::parse_href(bad).unwrap_err();
            assert!(err.message.contains(needle), "{bad}: {}", err.message);
        }
    }

    #[test]
    fn image_hrefs_are_read_off_the_tree_including_xlink() {
        let doc = roxmltree::Document::parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><image href="a.png"/><g><image xlink:href="b.jpg"/></g><use href="#x"/></svg>"##,
        )
        .unwrap();
        assert_eq!(image_hrefs(&doc), vec!["a.png".to_string(), "b.jpg".to_string()]);
    }

    #[test]
    fn the_resolver_caps_and_types_what_it_reads() {
        let png = super::super::test_support::solid_png(4, 4, [255, 0, 0]);
        let mut map = HashMap::new();
        map.insert(Source::Store("a.png".into()), png);
        map.insert(Source::Store("big.bin".into()), vec![0u8; MAX_SOURCE_BYTES + 1]);
        map.insert(Source::Store("junk.png".into()), b"not an image".to_vec());
        map.insert(Source::Repo("static/l.svg".into()), b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 4 4'><rect width='4' height='4'/></svg>".to_vec());
        let resolver = Resolver::new(Arc::new(MemoryStore(map)));
        assert!(matches!(resolver.prefetch(&Source::Store("a.png".into())).unwrap(), Asset::Raster { width: 4, height: 4, .. }));
        assert!(resolver.prefetch(&Source::Store("big.bin".into())).unwrap_err().message.contains("limit"));
        assert!(resolver.prefetch(&Source::Store("junk.png".into())).unwrap_err().message.contains("not a PNG"));
        assert!(matches!(resolver.prefetch(&Source::Repo("static/l.svg".into())).unwrap(), Asset::Svg { .. }));
        let fonts = super::super::FontSet::bundled();
        let opt = fonts.options();
        assert!(matches!(resolver.image_kind("a.png", &opt), Some(ImageKind::PNG(_))));
        assert!(matches!(resolver.image_kind("zebfs://a.png", &opt), Some(ImageKind::PNG(_))));
        assert!(matches!(resolver.image_kind("repo://static/l.svg", &opt), Some(ImageKind::SVG(_))));
        assert!(resolver.image_kind("never-prefetched.png", &opt).is_none());
        assert!(resolver.image_kind("https://x.test/a.png", &opt).is_none());
    }
}

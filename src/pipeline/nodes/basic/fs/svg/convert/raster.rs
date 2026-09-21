//! SVG → pixels → bytes. resvg draws into a tiny-skia pixmap sized by the
//! SVG's own `width`/`height` or by `--width`/`--height` with `--fit`; the
//! `image` crate encodes PNG, JPEG or WebP.

use std::io::Cursor;
use std::time::Instant;

use image::{ImageEncoder, RgbaImage};

use super::ConvertError;
use super::fonts::FontSet;
use super::sources::Resolver;

/// A canvas side may be this long.
pub const MAX_SIDE: u32 = 8192;
/// A canvas may have this many pixels.
pub const MAX_PIXELS: u64 = 40_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Png,
    Jpg,
    Webp,
}

impl OutputFormat {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpg),
            "webp" => Some(Self::Webp),
            _ => None,
        }
    }
    pub fn extension(self) -> &'static str {
        match self { Self::Png => "png", Self::Jpg => "jpg", Self::Webp => "webp" }
    }
    pub fn mime(self) -> &'static str {
        match self { Self::Png => "image/png", Self::Jpg => "image/jpeg", Self::Webp => "image/webp" }
    }
}

/// How `--width`×`--height` is reached from the SVG's own size — the same
/// three words as `fs.image.thumbnail`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fit {
    /// Scale to fill the box and crop the middle.
    #[default]
    Cover,
    /// Scale to fit inside the box; the canvas is the scaled size.
    Contain,
    /// Stretch to the box exactly.
    Fill,
}

impl Fit {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "cover" => Some(Self::Cover),
            "contain" => Some(Self::Contain),
            "fill" => Some(Self::Fill),
            _ => None,
        }
    }
}

/// The size asked for: neither = the SVG's own; one = the other follows the
/// SVG's proportions; both = by `fit`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Target {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fit: Fit,
}

/// Where the time went, in milliseconds.
#[derive(Debug, Clone, Copy, Default)]
pub struct Timing {
    pub parse_ms: u64,
    pub raster_ms: u64,
    pub encode_ms: u64,
}

pub struct Rendered {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub timing: Timing,
}

impl std::fmt::Debug for Rendered {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rendered").field("bytes", &self.bytes.len()).field("width", &self.width).field("height", &self.height).field("timing", &self.timing).finish()
    }
}

/// Canvas size and the transform that puts the SVG on it.
fn place(natural: (f32, f32), target: &Target) -> Result<(u32, u32, tiny_skia::Transform), ConvertError> {
    let (nw, nh) = natural;
    if nw <= 0.0 || nh <= 0.0 {
        return Err(ConvertError::source("the svg has no size: give the <svg> a width and height or a viewBox"));
    }
    let round = |v: f32| (v.round() as u32).max(1);
    let (w, h, t) = match (target.width, target.height) {
        (None, None) => (round(nw), round(nh), tiny_skia::Transform::identity()),
        (Some(w), None) => {
            let s = w as f32 / nw;
            (w, round(nh * s), tiny_skia::Transform::from_scale(s, s))
        }
        (None, Some(h)) => {
            let s = h as f32 / nh;
            (round(nw * s), h, tiny_skia::Transform::from_scale(s, s))
        }
        (Some(w), Some(h)) => match target.fit {
            Fit::Fill => (w, h, tiny_skia::Transform::from_scale(w as f32 / nw, h as f32 / nh)),
            Fit::Contain => {
                let s = (w as f32 / nw).min(h as f32 / nh);
                (round(nw * s), round(nh * s), tiny_skia::Transform::from_scale(s, s))
            }
            Fit::Cover => {
                let s = (w as f32 / nw).max(h as f32 / nh);
                let tx = (w as f32 - nw * s) / 2.0;
                let ty = (h as f32 - nh * s) / 2.0;
                (w, h, tiny_skia::Transform::from_row(s, 0.0, 0.0, s, tx, ty))
            }
        },
    };
    if w > MAX_SIDE || h > MAX_SIDE || u64::from(w) * u64::from(h) > MAX_PIXELS {
        return Err(ConvertError::source(format!(
            "a {w}×{h} canvas is over the cap ({MAX_SIDE} px a side, {MAX_PIXELS} pixels)"
        )));
    }
    Ok((w, h, t))
}

/// Rasterises `svg` and encodes it. `quality` applies to JPEG (1–100); PNG
/// and WebP are lossless.
pub fn render(
    svg: &str,
    fonts: &FontSet,
    resolver: &Resolver,
    target: &Target,
    format: OutputFormat,
    quality: u8,
) -> Result<Rendered, ConvertError> {
    let t = Instant::now();
    let opt = resolver.options(fonts.options());
    let tree = usvg::Tree::from_str(svg, &opt).map_err(|e| ConvertError::source(format!("svg does not parse: {e}")))?;
    let parse_ms = t.elapsed().as_millis() as u64;
    let (width, height, transform) = place((tree.size().width(), tree.size().height()), target)?;

    let t = Instant::now();
    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| ConvertError::raster(format!("cannot allocate a {width}×{height} canvas")))?;
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let raster_ms = t.elapsed().as_millis() as u64;

    let t = Instant::now();
    // tiny-skia pixels are premultiplied RGBA; encoders want straight alpha.
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for px in pixmap.pixels() {
        let c = px.demultiply();
        rgba.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    let img = RgbaImage::from_raw(width, height, rgba).ok_or_else(|| ConvertError::raster("pixel buffer size mismatch"))?;
    let mut out = Cursor::new(Vec::new());
    match format {
        OutputFormat::Png => {
            image::codecs::png::PngEncoder::new_with_quality(&mut out, image::codecs::png::CompressionType::Fast, image::codecs::png::FilterType::Adaptive)
                .write_image(img.as_raw(), width, height, image::ExtendedColorType::Rgba8)
                .map_err(|e| ConvertError::raster(format!("png encode: {e}")))?;
        }
        OutputFormat::Jpg => {
            let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality.clamp(1, 100))
                .write_image(rgb.as_raw(), width, height, image::ExtendedColorType::Rgb8)
                .map_err(|e| ConvertError::raster(format!("jpeg encode: {e}")))?;
        }
        OutputFormat::Webp => {
            image::codecs::webp::WebPEncoder::new_lossless(&mut out)
                .write_image(img.as_raw(), width, height, image::ExtendedColorType::Rgba8)
                .map_err(|e| ConvertError::raster(format!("webp encode: {e}")))?;
        }
    }
    let encode_ms = t.elapsed().as_millis() as u64;
    Ok(Rendered { bytes: out.into_inner(), width, height, timing: Timing { parse_ms, raster_ms, encode_ms } })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_canvas_follows_the_svg_one_side_or_the_fit() {
        let t = |w, h, fit| Target { width: w, height: h, fit };
        assert_eq!(place((1080.0, 1350.0), &t(None, None, Fit::Cover)).map(|(w, h, _)| (w, h)).unwrap(), (1080, 1350));
        assert_eq!(place((1080.0, 1350.0), &t(Some(540), None, Fit::Cover)).map(|(w, h, _)| (w, h)).unwrap(), (540, 675));
        assert_eq!(place((1080.0, 1350.0), &t(None, Some(270), Fit::Cover)).map(|(w, h, _)| (w, h)).unwrap(), (216, 270));
        assert_eq!(place((1080.0, 1350.0), &t(Some(500), Some(500), Fit::Contain)).map(|(w, h, _)| (w, h)).unwrap(), (400, 500));
        assert_eq!(place((1080.0, 1350.0), &t(Some(500), Some(500), Fit::Fill)).map(|(w, h, _)| (w, h)).unwrap(), (500, 500));
        let (w, h, tr) = place((1080.0, 1350.0), &t(Some(500), Some(500), Fit::Cover)).unwrap();
        assert_eq!((w, h), (500, 500));
        assert!((tr.sx - 500.0 / 1080.0).abs() < 1e-5 && tr.ty < 0.0, "{tr:?}");
        assert!(place((0.0, 10.0), &t(None, None, Fit::Cover)).is_err());
        assert!(place((10.0, 10.0), &t(Some(9000), Some(9000), Fit::Fill)).unwrap_err().message.contains("cap"));
    }
}

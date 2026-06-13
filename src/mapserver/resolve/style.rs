//! Layer style system for tile rendering.
//!
//! Provides a simple per-layer style with CSS color support and
//! zoom-dependent adjustments for stroke width and point radius.

use serde::{Deserialize, Serialize};

/// Serializable style configuration stored in layer registry JSON.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayerStyleConfig {
    #[serde(default = "default_fill")]
    pub fill: String,
    #[serde(default = "default_stroke")]
    pub stroke: String,
    #[serde(default = "default_stroke_width")]
    pub stroke_width: f32,
    #[serde(default = "default_point_radius")]
    pub point_radius: f32,
    #[serde(default = "default_point_color")]
    pub point_color: String,
}

fn default_fill() -> String {
    "rgba(65,105,225,128)".to_string()
}
fn default_stroke() -> String {
    "#1E3CA0DC".to_string()
}
fn default_stroke_width() -> f32 {
    1.0
}
fn default_point_radius() -> f32 {
    4.0
}
fn default_point_color() -> String {
    "#DC3232C8".to_string()
}

impl Default for LayerStyleConfig {
    fn default() -> Self {
        Self {
            fill: default_fill(),
            stroke: default_stroke(),
            stroke_width: default_stroke_width(),
            point_radius: default_point_radius(),
            point_color: default_point_color(),
        }
    }
}

/// Render-time style with resolved RGBA byte values.
#[derive(Debug, Clone)]
pub struct LayerStyle {
    pub fill_color: Option<[u8; 4]>,
    pub stroke_color: [u8; 4],
    pub stroke_width: f32,
    pub point_radius: f32,
    pub point_color: [u8; 4],
}

impl Default for LayerStyle {
    fn default() -> Self {
        Self {
            fill_color: Some([65, 105, 225, 128]),
            stroke_color: [30, 60, 160, 220],
            stroke_width: 1.0,
            point_radius: 4.0,
            point_color: [220, 50, 50, 200],
        }
    }
}

impl LayerStyle {
    pub fn from_config(config: &LayerStyleConfig) -> Self {
        Self {
            fill_color: parse_css_color(&config.fill),
            stroke_color: parse_css_color(&config.stroke).unwrap_or([30, 60, 160, 220]),
            stroke_width: config.stroke_width.max(0.0),
            point_radius: config.point_radius.max(0.5),
            point_color: parse_css_color(&config.point_color).unwrap_or([220, 50, 50, 200]),
        }
    }
}

/// Convert a `LayerStyleConfig` (from manifest) into a render-time `LayerStyle`
/// with zoom-dependent adjustments.
pub fn resolve_layer_style(
    style_config: Option<&LayerStyleConfig>,
    zoom: Option<u8>,
) -> LayerStyle {
    let base = style_config
        .map(LayerStyle::from_config)
        .unwrap_or_default();

    let z = zoom.unwrap_or(10);
    LayerStyle {
        stroke_width: if z <= 6 {
            0.5
        } else if z <= 12 {
            base.stroke_width
        } else {
            base.stroke_width * 1.5
        },
        point_radius: if z <= 8 {
            2.0
        } else if z <= 12 {
            base.point_radius
        } else {
            base.point_radius * 1.5
        },
        ..base
    }
}

/// Parse a CSS color string to RGBA bytes.
///
/// Supports:
/// - `#RGB` / `#RGBA` / `#RRGGBB` / `#RRGGBBAA`
/// - `rgb(r,g,b)` / `rgba(r,g,b,a)`
pub fn parse_css_color(s: &str) -> Option<[u8; 4]> {
    let s = s.trim();
    if s.starts_with('#') {
        parse_hex_color(s)
    } else if s.starts_with("rgba(") || s.starts_with("rgb(") {
        parse_rgb_color(s)
    } else {
        None
    }
}

fn parse_hex_color(s: &str) -> Option<[u8; 4]> {
    let hex = s.trim_start_matches('#');
    match hex.len() {
        3 => {
            let r = hex_byte(hex.as_bytes()[0])? * 17;
            let g = hex_byte(hex.as_bytes()[1])? * 17;
            let b = hex_byte(hex.as_bytes()[2])? * 17;
            Some([r, g, b, 255])
        }
        4 => {
            let r = hex_byte(hex.as_bytes()[0])? * 17;
            let g = hex_byte(hex.as_bytes()[1])? * 17;
            let b = hex_byte(hex.as_bytes()[2])? * 17;
            let a = hex_byte(hex.as_bytes()[3])? * 17;
            Some([r, g, b, a])
        }
        6 => {
            let r = hex_pair(hex.as_bytes()[0], hex.as_bytes()[1])?;
            let g = hex_pair(hex.as_bytes()[2], hex.as_bytes()[3])?;
            let b = hex_pair(hex.as_bytes()[4], hex.as_bytes()[5])?;
            Some([r, g, b, 255])
        }
        8 => {
            let r = hex_pair(hex.as_bytes()[0], hex.as_bytes()[1])?;
            let g = hex_pair(hex.as_bytes()[2], hex.as_bytes()[3])?;
            let b = hex_pair(hex.as_bytes()[4], hex.as_bytes()[5])?;
            let a = hex_pair(hex.as_bytes()[6], hex.as_bytes()[7])?;
            Some([r, g, b, a])
        }
        _ => None,
    }
}

fn hex_byte(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn hex_pair(hi: u8, lo: u8) -> Option<u8> {
    Some(hex_byte(hi)? * 16 + hex_byte(lo)?)
}

fn parse_rgb_color(s: &str) -> Option<[u8; 4]> {
    let inner = s
        .trim_start_matches("rgba(")
        .trim_start_matches("rgb(")
        .trim_end_matches(')');
    let parts: Vec<&str> = inner.split(',').collect();
    if parts.len() < 3 {
        return None;
    }
    let r = parts[0].trim().parse::<u8>().ok()?;
    let g = parts[1].trim().parse::<u8>().ok()?;
    let b = parts[2].trim().parse::<u8>().ok()?;
    let a = if parts.len() >= 4 {
        let a_str = parts[3].trim();
        // Support both 0-255 integer and 0.0-1.0 float
        if a_str.contains('.') {
            let f: f32 = a_str.parse().ok()?;
            (f * 255.0).round() as u8
        } else {
            a_str.parse::<u8>().ok()?
        }
    } else {
        255
    };
    Some([r, g, b, a])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_6() {
        assert_eq!(parse_css_color("#FF0000"), Some([255, 0, 0, 255]));
        assert_eq!(parse_css_color("#1E3CA0"), Some([30, 60, 160, 255]));
    }

    #[test]
    fn parse_hex_8() {
        assert_eq!(parse_css_color("#FF000080"), Some([255, 0, 0, 128]));
    }

    #[test]
    fn parse_hex_3() {
        assert_eq!(parse_css_color("#F00"), Some([255, 0, 0, 255]));
    }

    #[test]
    fn parse_rgba_int() {
        assert_eq!(
            parse_css_color("rgba(65,105,225,128)"),
            Some([65, 105, 225, 128])
        );
    }

    #[test]
    fn parse_rgba_float() {
        assert_eq!(
            parse_css_color("rgba(65,105,225,0.5)"),
            Some([65, 105, 225, 128])
        );
    }

    #[test]
    fn parse_rgb() {
        assert_eq!(parse_css_color("rgb(255,0,0)"), Some([255, 0, 0, 255]));
    }

    #[test]
    fn default_style_resolves() {
        let style = resolve_layer_style(None, Some(10));
        assert!(style.fill_color.is_some());
        assert!(style.stroke_width > 0.0);
    }

    #[test]
    fn low_zoom_shrinks_strokes() {
        let config = LayerStyleConfig::default();
        let style = resolve_layer_style(Some(&config), Some(4));
        assert_eq!(style.stroke_width, 0.5);
        assert_eq!(style.point_radius, 2.0);
    }

    #[test]
    fn high_zoom_grows_strokes() {
        let config = LayerStyleConfig {
            stroke_width: 2.0,
            point_radius: 5.0,
            ..Default::default()
        };
        let style = resolve_layer_style(Some(&config), Some(14));
        assert_eq!(style.stroke_width, 3.0);
        assert_eq!(style.point_radius, 7.5);
    }
}

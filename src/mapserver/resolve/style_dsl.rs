//! Style DSL engine for data-driven map styling.
//!
//! Provides a compact URL-friendly notation for per-feature styling:
//!   - Simple:     `f:4169E180;s:1E3CA0;sw:1.5`
//!   - Unique val: `uv(road_type)` or `uv(road_type):highway=s:FF0000,local=s:888`
//!   - Class brk:  `cb(population,5,YlOrRd)` or `cb(pop):~1k=f:FFFFCC,~5k=f:FEB24C`
//!   - Graduated:  `gs(magnitude)` or `gs(magnitude):0~10=pr:3~20`
//!
//! The same DSL string works as a URL `?style=` param and as `--style` in publish.

use super::style::{parse_css_color, LayerStyle, LayerStyleConfig};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// ── Per-feature rendering instruction ────────────────────────────────────────

/// Resolved style for a single feature. Mirrors `LayerStyle` but scoped to one feature.
#[derive(Debug, Clone)]
pub struct FeatureStyle {
    pub fill_color: Option<[u8; 4]>,
    pub stroke_color: [u8; 4],
    pub stroke_width: f32,
    pub point_radius: f32,
    pub point_color: [u8; 4],
}

impl Default for FeatureStyle {
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

impl From<&LayerStyle> for FeatureStyle {
    fn from(ls: &LayerStyle) -> Self {
        Self {
            fill_color: ls.fill_color,
            stroke_color: ls.stroke_color,
            stroke_width: ls.stroke_width,
            point_radius: ls.point_radius,
            point_color: ls.point_color,
        }
    }
}

// ── Attribute value ──────────────────────────────────────────────────────────

/// A value extracted from a feature's attribute field for style classification.
#[derive(Debug, Clone, PartialEq)]
pub enum StyleValue {
    Str(String),
    F64(f64),
    I64(i64),
    Null,
}

impl StyleValue {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::F64(v) => Some(*v),
            Self::I64(v) => Some(*v as f64),
            Self::Str(s) => s.parse::<f64>().ok(),
            Self::Null => None,
        }
    }

    fn as_string_key(&self) -> String {
        match self {
            Self::Str(s) => s.clone(),
            Self::F64(v) => {
                if *v == (*v as i64) as f64 {
                    format!("{}", *v as i64)
                } else {
                    format!("{v}")
                }
            }
            Self::I64(v) => format!("{v}"),
            Self::Null => String::new(),
        }
    }
}

// ── Parsed DSL expression (AST) ─────────────────────────────────────────────

/// Property overrides parsed from shortkey notation.
#[derive(Debug, Clone, Default)]
pub struct PropMap {
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub stroke_width: Option<f32>,
    pub point_radius: Option<f32>,
    pub point_color: Option<String>,
    pub opacity: Option<f32>,
}

/// Parsed DSL expression. May contain auto markers that need data resolution.
#[derive(Debug, Clone)]
pub enum StyleDefinition {
    /// Static style: `f:4169E1;s:1E3CA0;sw:1.5`
    Simple(PropMap),
    /// Unique value: `uv(field)` or `uv(field,palette)` or `uv(field):val=props,...`
    UniqueValue {
        field: String,
        palette: Option<String>,
        manual_classes: Option<Vec<(String, PropMap)>>,
        overrides: PropMap,
    },
    /// Class breaks: `cb(field)` or `cb(field,N,ramp)` or `cb(field):~max=props,...`
    ClassBreaks {
        field: String,
        num_classes: Option<u8>,
        ramp: Option<String>,
        manual_breaks: Option<Vec<(f64, PropMap)>>,
        overrides: PropMap,
    },
    /// Graduated symbol: `gs(field)` or `gs(field):min~max=pr:min~max`
    GraduatedSymbol {
        field: String,
        value_range: Option<(f64, f64)>,
        radius_range: Option<(f32, f32)>,
        color: Option<String>,
        overrides: PropMap,
    },
}

impl StyleDefinition {
    pub fn needs_stats(&self) -> bool {
        matches!(
            self,
            StyleDefinition::ClassBreaks { manual_breaks: None, .. }
                | StyleDefinition::GraduatedSymbol { value_range: None, .. }
        )
    }

    pub fn needs_distinct(&self) -> bool {
        matches!(
            self,
            StyleDefinition::UniqueValue { manual_classes: None, .. }
        )
    }

    pub fn field_name(&self) -> Option<&str> {
        match self {
            StyleDefinition::Simple(_) => None,
            StyleDefinition::UniqueValue { field, .. } => Some(field),
            StyleDefinition::ClassBreaks { field, .. } => Some(field),
            StyleDefinition::GraduatedSymbol { field, .. } => Some(field),
        }
    }
}

// ── Data info for auto-resolution ────────────────────────────────────────────

/// Min/max statistics for a numeric column (from parquet row group metadata).
#[derive(Debug, Clone)]
pub struct FieldStats {
    pub min: f64,
    pub max: f64,
}

/// Distinct values for a categorical column.
#[derive(Debug, Clone)]
pub struct FieldDistinct {
    pub values: Vec<String>,
}

// ── Resolved style (concrete, ready for rendering) ──────────────────────────

/// Fully resolved style. No auto markers remain.
#[derive(Debug, Clone)]
pub enum ResolvedStyle {
    /// Every feature gets the same style.
    Uniform(FeatureStyle),
    /// Categorical: field value → FeatureStyle.
    UniqueValue {
        field: String,
        classes: Vec<(String, FeatureStyle)>,
        fallback: FeatureStyle,
    },
    /// Numeric breaks: sorted ascending thresholds.
    ClassBreaks {
        field: String,
        breaks: Vec<(f64, FeatureStyle)>,
        fallback: FeatureStyle,
    },
    /// Graduated symbol: linear interpolation of point radius.
    GraduatedSymbol {
        field: String,
        value_min: f64,
        value_max: f64,
        radius_min: f32,
        radius_max: f32,
        base_style: FeatureStyle,
    },
}

impl ResolvedStyle {
    pub fn is_uniform(&self) -> bool {
        matches!(self, Self::Uniform(_))
    }

    pub fn field_name(&self) -> Option<&str> {
        match self {
            Self::Uniform(_) => None,
            Self::UniqueValue { field, .. } => Some(field),
            Self::ClassBreaks { field, .. } => Some(field),
            Self::GraduatedSymbol { field, .. } => Some(field),
        }
    }

    /// Resolve a feature's style from its attribute value.
    pub fn style_for_value(&self, value: &StyleValue) -> FeatureStyle {
        match self {
            Self::Uniform(s) => s.clone(),
            Self::UniqueValue {
                classes, fallback, ..
            } => {
                let key = value.as_string_key();
                classes
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, s)| s.clone())
                    .unwrap_or_else(|| fallback.clone())
            }
            Self::ClassBreaks {
                breaks, fallback, ..
            } => {
                let Some(v) = value.as_f64() else {
                    return fallback.clone();
                };
                for (upper, style) in breaks {
                    if v <= *upper {
                        return style.clone();
                    }
                }
                fallback.clone()
            }
            Self::GraduatedSymbol {
                value_min,
                value_max,
                radius_min,
                radius_max,
                base_style,
                ..
            } => {
                let Some(v) = value.as_f64() else {
                    return base_style.clone();
                };
                let range = value_max - value_min;
                let t = if range.abs() < 1e-12 {
                    0.5
                } else {
                    ((v - value_min) / range).clamp(0.0, 1.0) as f32
                };
                let radius = radius_min + t * (radius_max - radius_min);
                let mut s = base_style.clone();
                s.point_radius = radius;
                s
            }
        }
    }
}

// ── Color palettes (categorical) ─────────────────────────────────────────────

const CATEGORY10: [[u8; 4]; 10] = [
    [31, 119, 180, 200],
    [255, 127, 14, 200],
    [44, 160, 44, 200],
    [214, 39, 40, 200],
    [148, 103, 189, 200],
    [140, 86, 75, 200],
    [227, 119, 194, 200],
    [127, 127, 127, 200],
    [188, 189, 34, 200],
    [23, 190, 207, 200],
];

const PASTEL: [[u8; 4]; 10] = [
    [166, 206, 227, 200],
    [178, 223, 138, 200],
    [251, 154, 153, 200],
    [253, 191, 111, 200],
    [202, 178, 214, 200],
    [255, 255, 153, 200],
    [190, 186, 218, 200],
    [251, 180, 174, 200],
    [204, 235, 197, 200],
    [222, 203, 228, 200],
];

const BOLD: [[u8; 4]; 10] = [
    [228, 26, 28, 200],
    [55, 126, 184, 200],
    [77, 175, 74, 200],
    [152, 78, 163, 200],
    [255, 127, 0, 200],
    [166, 86, 40, 200],
    [247, 129, 191, 200],
    [153, 153, 153, 200],
    [255, 255, 51, 200],
    [26, 152, 80, 200],
];

const DARK: [[u8; 4]; 10] = [
    [27, 158, 119, 200],
    [217, 95, 2, 200],
    [117, 112, 179, 200],
    [231, 41, 138, 200],
    [102, 166, 30, 200],
    [230, 171, 2, 200],
    [166, 118, 29, 200],
    [102, 102, 102, 200],
    [0, 109, 44, 200],
    [49, 54, 149, 200],
];

const MUTED: [[u8; 4]; 10] = [
    [136, 189, 220, 200],
    [255, 188, 121, 200],
    [117, 181, 120, 200],
    [255, 157, 166, 200],
    [186, 176, 212, 200],
    [209, 176, 134, 200],
    [243, 181, 213, 200],
    [186, 186, 186, 200],
    [214, 214, 148, 200],
    [145, 210, 222, 200],
];

pub fn categorical_palette(name: &str) -> Option<&'static [[u8; 4]]> {
    match name {
        "category10" | "default" | "" => Some(&CATEGORY10),
        "pastel" => Some(&PASTEL),
        "bold" => Some(&BOLD),
        "dark" => Some(&DARK),
        "muted" => Some(&MUTED),
        _ => None,
    }
}

// ── Sequential ramps ─────────────────────────────────────────────────────────

const RAMP_YLORRD: [[u8; 4]; 8] = [
    [255, 255, 204, 200],
    [255, 237, 160, 200],
    [254, 217, 118, 200],
    [254, 178, 76, 200],
    [253, 141, 60, 200],
    [252, 78, 42, 200],
    [227, 26, 28, 200],
    [177, 0, 38, 200],
];

const RAMP_BLUES: [[u8; 4]; 8] = [
    [239, 243, 255, 200],
    [198, 219, 239, 200],
    [158, 202, 225, 200],
    [107, 174, 214, 200],
    [66, 146, 198, 200],
    [33, 113, 181, 200],
    [8, 69, 148, 200],
    [8, 48, 107, 200],
];

const RAMP_VIRIDIS: [[u8; 4]; 8] = [
    [68, 1, 84, 200],
    [72, 36, 117, 200],
    [56, 88, 140, 200],
    [39, 130, 142, 200],
    [31, 168, 118, 200],
    [77, 195, 79, 200],
    [163, 218, 55, 200],
    [253, 231, 37, 200],
];

const RAMP_RDYLGN: [[u8; 4]; 8] = [
    [215, 48, 39, 200],
    [244, 109, 67, 200],
    [253, 174, 97, 200],
    [254, 224, 139, 200],
    [217, 239, 139, 200],
    [166, 217, 106, 200],
    [102, 189, 99, 200],
    [26, 152, 80, 200],
];

const RAMP_SPECTRAL: [[u8; 4]; 8] = [
    [213, 62, 79, 200],
    [244, 109, 67, 200],
    [253, 174, 97, 200],
    [254, 224, 139, 200],
    [230, 245, 152, 200],
    [171, 221, 164, 200],
    [102, 194, 165, 200],
    [50, 136, 189, 200],
];

const RAMP_MAGMA: [[u8; 4]; 8] = [
    [0, 0, 4, 200],
    [28, 16, 68, 200],
    [79, 18, 123, 200],
    [136, 34, 106, 200],
    [186, 54, 85, 200],
    [227, 89, 51, 200],
    [249, 149, 65, 200],
    [252, 253, 191, 200],
];

pub fn sequential_ramp(name: &str) -> Option<&'static [[u8; 4]]> {
    match name.to_ascii_lowercase().as_str() {
        "ylorrd" | "default" | "" => Some(&RAMP_YLORRD),
        "blues" => Some(&RAMP_BLUES),
        "viridis" => Some(&RAMP_VIRIDIS),
        "rdylgn" => Some(&RAMP_RDYLGN),
        "spectral" => Some(&RAMP_SPECTRAL),
        "magma" => Some(&RAMP_MAGMA),
        _ => None,
    }
}

/// Interpolate N colors from a ramp.
pub fn interpolate_ramp(ramp: &[[u8; 4]], n: usize) -> Vec<[u8; 4]> {
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![ramp[ramp.len() / 2]];
    }
    let step = (ramp.len() - 1) as f64 / (n - 1) as f64;
    (0..n)
        .map(|i| {
            let t = i as f64 * step;
            let lo = t.floor() as usize;
            let hi = (lo + 1).min(ramp.len() - 1);
            let frac = (t - lo as f64) as f32;
            lerp_color(ramp[lo], ramp[hi], frac)
        })
        .collect()
}

/// Parse custom gradient "color1:color2" and interpolate N steps.
pub fn custom_gradient(spec: &str, n: usize) -> Result<Vec<[u8; 4]>, String> {
    let parts: Vec<&str> = spec.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(format!("custom gradient needs two colors separated by ':', got: '{spec}'"));
    }
    let c1 = parse_hex_short(parts[0])
        .ok_or_else(|| format!("invalid gradient start color: '{}'", parts[0]))?;
    let c2 = parse_hex_short(parts[1])
        .ok_or_else(|| format!("invalid gradient end color: '{}'", parts[1]))?;
    Ok(interpolate_ramp(&[c1, c2], n))
}

fn lerp_color(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    [
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t).round() as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t).round() as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t).round() as u8,
        (a[3] as f32 + (b[3] as f32 - a[3] as f32) * t).round() as u8,
    ]
}

// ── DSL Parser ───────────────────────────────────────────────────────────────

/// Parse a style DSL string into a StyleDefinition.
pub fn parse_style_dsl(input: &str) -> Result<StyleDefinition, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("empty style expression".into());
    }
    if input.starts_with("uv(") {
        parse_unique_value(input)
    } else if input.starts_with("cb(") {
        parse_class_breaks(input)
    } else if input.starts_with("gs(") {
        parse_graduated_symbol(input)
    } else {
        parse_simple_style(input)
    }
}

/// Split `uv(field,palette):val=props` into `("field,palette", Some("val=props"), "overrides")`
fn split_func_parts(input: &str) -> Result<(&str, Option<&str>, &str), String> {
    let open = input.find('(').ok_or("missing '('")?;
    let close = input.find(')').ok_or("missing ')'")?;
    let args = &input[open + 1..close];
    let rest = input[close + 1..].trim();
    if rest.is_empty() {
        return Ok((args, None, ""));
    }
    if let Some(stripped) = rest.strip_prefix(':') {
        // Could have manual classes/breaks, potentially followed by ;overrides
        Ok((args, Some(stripped.trim()), ""))
    } else if rest.starts_with(';') {
        // Global overrides only
        Ok((args, None, rest))
    } else {
        Err(format!("unexpected content after ')': '{rest}'"))
    }
}

fn parse_unique_value(input: &str) -> Result<StyleDefinition, String> {
    let (args, manual_part, override_part) = split_func_parts(input)?;
    let arg_parts: Vec<&str> = args.splitn(2, ',').collect();
    let field = arg_parts[0].trim().to_string();
    if field.is_empty() {
        return Err("uv() requires a field name".into());
    }
    let palette = arg_parts.get(1).map(|s| s.trim().to_string());

    let manual_classes = if let Some(manual) = manual_part {
        // Split on ; to separate classes from overrides
        let (classes_str, overrides_str) = split_classes_and_overrides(manual);
        let _ = overrides_str; // overrides handled below
        let mut classes = Vec::new();
        for class_spec in classes_str.split(',') {
            let class_spec = class_spec.trim();
            if class_spec.is_empty() {
                continue;
            }
            let (value, props_str) = class_spec
                .split_once('=')
                .ok_or_else(|| format!("invalid class spec (expected 'value=props'): '{class_spec}'"))?;
            let props = parse_prop_assignments(props_str)?;
            classes.push((value.trim().to_string(), props));
        }
        Some(classes)
    } else {
        None
    };

    let overrides = if !override_part.is_empty() {
        parse_prop_map(override_part)?
    } else if manual_part.is_some() {
        let (_, ov) = split_classes_and_overrides(manual_part.unwrap());
        if !ov.is_empty() {
            parse_prop_map(ov)?
        } else {
            PropMap::default()
        }
    } else {
        PropMap::default()
    };

    Ok(StyleDefinition::UniqueValue {
        field,
        palette,
        manual_classes,
        overrides,
    })
}

fn parse_class_breaks(input: &str) -> Result<StyleDefinition, String> {
    let (args, manual_part, override_part) = split_func_parts(input)?;
    let arg_parts: Vec<&str> = args.splitn(3, ',').collect();
    let field = arg_parts[0].trim().to_string();
    if field.is_empty() {
        return Err("cb() requires a field name".into());
    }
    let num_classes = arg_parts
        .get(1)
        .and_then(|s| s.trim().parse::<u8>().ok());
    let ramp = arg_parts.get(2).map(|s| s.trim().to_string());

    let manual_breaks = if let Some(manual) = manual_part {
        let (breaks_str, _overrides_str) = split_classes_and_overrides(manual);
        let mut breaks = Vec::new();
        for break_spec in breaks_str.split(',') {
            let break_spec = break_spec.trim();
            if break_spec.is_empty() {
                continue;
            }
            if break_spec.starts_with("+=") || break_spec == "+" {
                // Catch-all: no upper bound, use f64::MAX
                let props_str = break_spec.strip_prefix("+=").unwrap_or("");
                let props = if props_str.is_empty() {
                    PropMap::default()
                } else {
                    parse_prop_assignments(props_str)?
                };
                breaks.push((f64::MAX, props));
            } else if let Some(rest) = break_spec.strip_prefix('~') {
                let (max_str, props_str) = rest
                    .split_once('=')
                    .ok_or_else(|| format!("invalid break spec: '{break_spec}'"))?;
                let max_val = parse_number_with_suffix(max_str.trim())?;
                let props = parse_prop_assignments(props_str)?;
                breaks.push((max_val, props));
            } else {
                return Err(format!("break spec must start with '~' or '+': '{break_spec}'"));
            }
        }
        Some(breaks)
    } else {
        None
    };

    let overrides = if !override_part.is_empty() {
        parse_prop_map(override_part)?
    } else if manual_part.is_some() {
        let (_, ov) = split_classes_and_overrides(manual_part.unwrap());
        if !ov.is_empty() {
            parse_prop_map(ov)?
        } else {
            PropMap::default()
        }
    } else {
        PropMap::default()
    };

    Ok(StyleDefinition::ClassBreaks {
        field,
        num_classes,
        ramp,
        manual_breaks,
        overrides,
    })
}

fn parse_graduated_symbol(input: &str) -> Result<StyleDefinition, String> {
    let (args, _manual_part, _override_part) = split_func_parts(input)?;
    let arg_parts: Vec<&str> = args.splitn(2, ',').collect();
    let field = arg_parts[0].trim().to_string();
    if field.is_empty() {
        return Err("gs() requires a field name".into());
    }

    Ok(StyleDefinition::GraduatedSymbol {
        field,
        value_range: None,
        radius_range: None,
        color: arg_parts.get(1).map(|s| s.trim().to_string()),
        overrides: PropMap::default(),
    })
}

fn parse_simple_style(input: &str) -> Result<StyleDefinition, String> {
    let props = parse_prop_map(input)?;
    Ok(StyleDefinition::Simple(props))
}

/// Parse semicolon-separated shortkey:value pairs.
/// Input: `f:4169E1;s:1E3CA0;sw:1.5`
fn parse_prop_map(input: &str) -> Result<PropMap, String> {
    let mut pm = PropMap::default();
    for part in input.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (key, val) = part
            .split_once(':')
            .ok_or_else(|| format!("invalid property (expected 'key:value'): '{part}'"))?;
        apply_prop(&mut pm, key.trim(), val.trim())?;
    }
    Ok(pm)
}

/// Parse plus-separated shortkey=value pairs within a class.
/// Input: `f:4169E1+s:1E3CA0+sw:1.5`  or `s:FF0000`
fn parse_prop_assignments(input: &str) -> Result<PropMap, String> {
    let mut pm = PropMap::default();
    // Support both + and ; as separators within class assignments
    for part in input.split('+') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (key, val) = part
            .split_once(':')
            .ok_or_else(|| format!("invalid property (expected 'key:value'): '{part}'"))?;
        apply_prop(&mut pm, key.trim(), val.trim())?;
    }
    Ok(pm)
}

fn apply_prop(pm: &mut PropMap, key: &str, val: &str) -> Result<(), String> {
    match key {
        "f" | "fill" => pm.fill = Some(val.to_string()),
        "s" | "stroke" => pm.stroke = Some(val.to_string()),
        "sw" => pm.stroke_width = Some(val.parse::<f32>().map_err(|_| format!("invalid sw: '{val}'"))?),
        "pr" => pm.point_radius = Some(val.parse::<f32>().map_err(|_| format!("invalid pr: '{val}'"))?),
        "pc" => pm.point_color = Some(val.to_string()),
        "o" | "opacity" => pm.opacity = Some(val.parse::<f32>().map_err(|_| format!("invalid opacity: '{val}'"))?),
        _ => return Err(format!("unknown style property: '{key}'")),
    }
    Ok(())
}

/// Parse number with optional suffixes: `5k` → 5000, `1.5m` → 1_500_000.
pub fn parse_number_with_suffix(s: &str) -> Result<f64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty number".into());
    }
    let (num_str, multiplier) = if let Some(n) = s.strip_suffix('k') {
        (n, 1_000.0)
    } else if let Some(n) = s.strip_suffix('K') {
        (n, 1_000.0)
    } else if let Some(n) = s.strip_suffix('m') {
        (n, 1_000_000.0)
    } else if let Some(n) = s.strip_suffix('M') {
        (n, 1_000_000.0)
    } else {
        (s, 1.0)
    };
    let num: f64 = num_str
        .parse()
        .map_err(|_| format!("invalid number: '{s}'"))?;
    Ok(num * multiplier)
}

/// Split manual class specs from trailing override properties.
/// In `40=s:0C0,60=s:FA0;sw:2`, the classes are `40=s:0C0,60=s:FA0` and overrides are `;sw:2`.
fn split_classes_and_overrides(input: &str) -> (&str, &str) {
    // Find the last semicolon that's followed by a property key (not inside a class assignment)
    // Simple heuristic: find first ';' that's followed by a known property key
    for (i, _) in input.match_indices(';') {
        let after = input[i + 1..].trim_start();
        if after.starts_with("f:")
            || after.starts_with("s:")
            || after.starts_with("sw:")
            || after.starts_with("pr:")
            || after.starts_with("pc:")
            || after.starts_with("o:")
            || after.starts_with("fill:")
            || after.starts_with("stroke:")
            || after.starts_with("opacity:")
        {
            return (&input[..i], &input[i..]);
        }
    }
    (input, "")
}

/// Parse a hex color without the # prefix. Supports 3, 4, 6, 8 hex digits.
fn parse_hex_short(s: &str) -> Option<[u8; 4]> {
    let s = s.trim().trim_start_matches('#');
    if s.is_empty() {
        return None;
    }
    parse_css_color(&format!("#{s}"))
}

/// Resolve a hex color from a PropMap fill/stroke/color string.
/// Supports both `#RRGGBB` and plain `RRGGBB` (DSL shorthand without #).
fn resolve_color(s: &str) -> Option<[u8; 4]> {
    let s = s.trim();
    if s.starts_with('#') || s.starts_with("rgb") {
        parse_css_color(s)
    } else {
        parse_hex_short(s)
    }
}

// ── Resolution (auto → concrete) ────────────────────────────────────────────

/// Resolve a parsed StyleDefinition into a concrete ResolvedStyle.
pub fn resolve_style(
    def: &StyleDefinition,
    stats: Option<&FieldStats>,
    distinct: Option<&FieldDistinct>,
    zoom: Option<u8>,
) -> Result<ResolvedStyle, String> {
    match def {
        StyleDefinition::Simple(props) => {
            let mut fs = FeatureStyle::default();
            apply_props_to_feature_style(&mut fs, props);
            apply_zoom_scaling(&mut fs, zoom);
            Ok(ResolvedStyle::Uniform(fs))
        }
        StyleDefinition::UniqueValue {
            field,
            palette,
            manual_classes,
            overrides,
        } => {
            if let Some(manual) = manual_classes {
                let mut base = FeatureStyle::default();
                apply_props_to_feature_style(&mut base, overrides);
                apply_zoom_scaling(&mut base, zoom);
                let classes: Vec<(String, FeatureStyle)> = manual
                    .iter()
                    .map(|(val, props)| {
                        let mut fs = base.clone();
                        apply_props_to_feature_style(&mut fs, props);
                        (val.clone(), fs)
                    })
                    .collect();
                Ok(ResolvedStyle::UniqueValue {
                    field: field.clone(),
                    classes,
                    fallback: base,
                })
            } else {
                let distinct = distinct
                    .ok_or("unique value style requires distinct values from data")?;
                let pal_name = palette.as_deref().unwrap_or("default");
                let colors = categorical_palette(pal_name)
                    .ok_or_else(|| format!("unknown palette: '{pal_name}'"))?;
                let mut base = FeatureStyle::default();
                apply_props_to_feature_style(&mut base, overrides);
                apply_zoom_scaling(&mut base, zoom);
                let classes: Vec<(String, FeatureStyle)> = distinct
                    .values
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let c = colors[i % colors.len()];
                        let mut fs = base.clone();
                        fs.fill_color = Some(c);
                        fs.stroke_color = darken(c, 0.6);
                        fs.point_color = c;
                        (v.clone(), fs)
                    })
                    .collect();
                Ok(ResolvedStyle::UniqueValue {
                    field: field.clone(),
                    classes,
                    fallback: base,
                })
            }
        }
        StyleDefinition::ClassBreaks {
            field,
            num_classes,
            ramp,
            manual_breaks,
            overrides,
        } => {
            if let Some(manual) = manual_breaks {
                let mut base = FeatureStyle::default();
                apply_props_to_feature_style(&mut base, overrides);
                apply_zoom_scaling(&mut base, zoom);
                let breaks: Vec<(f64, FeatureStyle)> = manual
                    .iter()
                    .map(|(max_val, props)| {
                        let mut fs = base.clone();
                        apply_props_to_feature_style(&mut fs, props);
                        (*max_val, fs)
                    })
                    .collect();
                Ok(ResolvedStyle::ClassBreaks {
                    field: field.clone(),
                    breaks,
                    fallback: base,
                })
            } else {
                let stats =
                    stats.ok_or("class breaks style requires min/max stats from data")?;
                let n = num_classes.unwrap_or(5) as usize;
                let colors = resolve_ramp_colors(ramp.as_deref(), n)?;
                let range = stats.max - stats.min;
                let step = if range.abs() < 1e-12 {
                    1.0
                } else {
                    range / n as f64
                };
                let mut base = FeatureStyle::default();
                apply_props_to_feature_style(&mut base, overrides);
                apply_zoom_scaling(&mut base, zoom);
                let breaks: Vec<(f64, FeatureStyle)> = (0..n)
                    .map(|i| {
                        let upper = if i == n - 1 {
                            f64::MAX
                        } else {
                            stats.min + (i + 1) as f64 * step
                        };
                        let mut fs = base.clone();
                        fs.fill_color = Some(colors[i]);
                        fs.stroke_color = darken(colors[i], 0.6);
                        fs.point_color = colors[i];
                        (upper, fs)
                    })
                    .collect();
                Ok(ResolvedStyle::ClassBreaks {
                    field: field.clone(),
                    breaks,
                    fallback: base,
                })
            }
        }
        StyleDefinition::GraduatedSymbol {
            field,
            value_range,
            radius_range,
            color,
            overrides,
        } => {
            let (vmin, vmax) = value_range.unwrap_or_else(|| {
                stats
                    .map(|s| (s.min, s.max))
                    .unwrap_or((0.0, 100.0))
            });
            let (rmin, rmax) = radius_range.unwrap_or((3.0, 16.0));
            let mut base = FeatureStyle::default();
            apply_props_to_feature_style(&mut base, overrides);
            if let Some(c) = color {
                if let Some(rgba) = resolve_color(c) {
                    base.point_color = rgba;
                    base.fill_color = Some(rgba);
                }
            }
            apply_zoom_scaling(&mut base, zoom);
            Ok(ResolvedStyle::GraduatedSymbol {
                field: field.clone(),
                value_min: vmin,
                value_max: vmax,
                radius_min: rmin,
                radius_max: rmax,
                base_style: base,
            })
        }
    }
}

fn resolve_ramp_colors(ramp_spec: Option<&str>, n: usize) -> Result<Vec<[u8; 4]>, String> {
    match ramp_spec {
        None => Ok(interpolate_ramp(sequential_ramp("default").unwrap(), n)),
        Some(spec) if spec.len() <= 7 && spec.contains(':') => custom_gradient(spec, n),
        Some(name) => {
            let ramp = sequential_ramp(name)
                .ok_or_else(|| format!("unknown color ramp: '{name}'"))?;
            Ok(interpolate_ramp(ramp, n))
        }
    }
}

fn apply_props_to_feature_style(fs: &mut FeatureStyle, props: &PropMap) {
    if let Some(ref fill) = props.fill {
        fs.fill_color = resolve_color(fill);
    }
    if let Some(ref stroke) = props.stroke {
        if let Some(c) = resolve_color(stroke) {
            fs.stroke_color = c;
        }
    }
    if let Some(sw) = props.stroke_width {
        fs.stroke_width = sw;
    }
    if let Some(pr) = props.point_radius {
        fs.point_radius = pr;
    }
    if let Some(ref pc) = props.point_color {
        if let Some(c) = resolve_color(pc) {
            fs.point_color = c;
        }
    }
    if let Some(o) = props.opacity {
        let alpha = (o * 255.0).round() as u8;
        if let Some(ref mut fc) = fs.fill_color {
            fc[3] = alpha;
        }
        fs.stroke_color[3] = alpha;
        fs.point_color[3] = alpha;
    }
}

fn apply_zoom_scaling(fs: &mut FeatureStyle, zoom: Option<u8>) {
    let z = zoom.unwrap_or(10);
    if z <= 6 {
        fs.stroke_width = 0.5;
        fs.point_radius = 2.0;
    } else if z >= 13 {
        fs.stroke_width *= 1.5;
        fs.point_radius *= 1.5;
    }
}

/// Darken a color by a factor (0.0 = black, 1.0 = unchanged).
fn darken(c: [u8; 4], factor: f32) -> [u8; 4] {
    [
        (c[0] as f32 * factor).round() as u8,
        (c[1] as f32 * factor).round() as u8,
        (c[2] as f32 * factor).round() as u8,
        c[3],
    ]
}

// ── Cache support ────────────────────────────────────────────────────────────

/// Compute a stable hash of a DSL string for tile cache keys.
pub fn style_hash(dsl: &str) -> u64 {
    let mut h = DefaultHasher::new();
    dsl.hash(&mut h);
    h.finish()
}

// ── Backward compatibility ───────────────────────────────────────────────────

/// Discriminator for style stored in layer manifest.
pub enum StyleSource {
    /// New DSL string (e.g., "cb(population,5,YlOrRd)")
    Dsl(StyleDefinition),
    /// Old JSON object (LayerStyleConfig format)
    LegacyJson,
}

/// Try to parse a style value from the layer manifest.
/// JSON string → DSL, JSON object → legacy config.
pub fn parse_style_value(style_json: &serde_json::Value) -> Result<StyleSource, String> {
    if let Some(dsl_str) = style_json.as_str() {
        Ok(StyleSource::Dsl(parse_style_dsl(dsl_str)?))
    } else if style_json.is_object() {
        Ok(StyleSource::LegacyJson)
    } else {
        Err("style must be a DSL string or a JSON object".into())
    }
}

/// Convert old-format LayerStyleConfig JSON into a ResolvedStyle.
pub fn legacy_style_to_resolved(
    style_json: &serde_json::Value,
    zoom: Option<u8>,
) -> ResolvedStyle {
    if let Ok(config) = serde_json::from_value::<LayerStyleConfig>(style_json.clone()) {
        let ls = super::style::resolve_layer_style(Some(&config), zoom);
        ResolvedStyle::Uniform(FeatureStyle::from(&ls))
    } else {
        ResolvedStyle::Uniform(FeatureStyle::default())
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_fill_stroke() {
        let def = parse_style_dsl("f:FF0000;s:00FF00;sw:2.5").unwrap();
        match def {
            StyleDefinition::Simple(props) => {
                assert_eq!(props.fill.as_deref(), Some("FF0000"));
                assert_eq!(props.stroke.as_deref(), Some("00FF00"));
                assert_eq!(props.stroke_width, Some(2.5));
            }
            _ => panic!("expected Simple"),
        }
    }

    #[test]
    fn parse_simple_all_props() {
        let def = parse_style_dsl("f:4169E180;s:1E3CA0;sw:1.5;pr:6;pc:DC3232;o:0.8").unwrap();
        match def {
            StyleDefinition::Simple(props) => {
                assert_eq!(props.fill.as_deref(), Some("4169E180"));
                assert_eq!(props.point_radius, Some(6.0));
                assert_eq!(props.point_color.as_deref(), Some("DC3232"));
                assert_eq!(props.opacity, Some(0.8));
            }
            _ => panic!("expected Simple"),
        }
    }

    #[test]
    fn parse_uv_auto() {
        let def = parse_style_dsl("uv(road_type)").unwrap();
        match def {
            StyleDefinition::UniqueValue {
                field,
                palette,
                manual_classes,
                ..
            } => {
                assert_eq!(field, "road_type");
                assert!(palette.is_none());
                assert!(manual_classes.is_none());
            }
            _ => panic!("expected UniqueValue"),
        }
    }

    #[test]
    fn parse_uv_with_palette() {
        let def = parse_style_dsl("uv(road_type,pastel)").unwrap();
        match def {
            StyleDefinition::UniqueValue { field, palette, .. } => {
                assert_eq!(field, "road_type");
                assert_eq!(palette.as_deref(), Some("pastel"));
            }
            _ => panic!("expected UniqueValue"),
        }
    }

    #[test]
    fn parse_uv_manual() {
        let def = parse_style_dsl("uv(speed_limit):40=s:22CC22,60=s:FFAA00,100=s:FF0000").unwrap();
        match def {
            StyleDefinition::UniqueValue {
                field,
                manual_classes,
                ..
            } => {
                assert_eq!(field, "speed_limit");
                let classes = manual_classes.unwrap();
                assert_eq!(classes.len(), 3);
                assert_eq!(classes[0].0, "40");
                assert_eq!(classes[0].1.stroke.as_deref(), Some("22CC22"));
                assert_eq!(classes[2].0, "100");
            }
            _ => panic!("expected UniqueValue"),
        }
    }

    #[test]
    fn parse_cb_auto() {
        let def = parse_style_dsl("cb(population)").unwrap();
        match def {
            StyleDefinition::ClassBreaks {
                field,
                num_classes,
                ramp,
                manual_breaks,
                ..
            } => {
                assert_eq!(field, "population");
                assert!(num_classes.is_none());
                assert!(ramp.is_none());
                assert!(manual_breaks.is_none());
            }
            _ => panic!("expected ClassBreaks"),
        }
    }

    #[test]
    fn parse_cb_auto_with_count_and_ramp() {
        let def = parse_style_dsl("cb(population,7,Blues)").unwrap();
        match def {
            StyleDefinition::ClassBreaks {
                field,
                num_classes,
                ramp,
                ..
            } => {
                assert_eq!(field, "population");
                assert_eq!(num_classes, Some(7));
                assert_eq!(ramp.as_deref(), Some("Blues"));
            }
            _ => panic!("expected ClassBreaks"),
        }
    }

    #[test]
    fn parse_cb_manual() {
        let def = parse_style_dsl("cb(pop):~1k=f:FFFFCC,~5k=f:FEB24C,+=f:BD0026").unwrap();
        match def {
            StyleDefinition::ClassBreaks {
                field,
                manual_breaks,
                ..
            } => {
                assert_eq!(field, "pop");
                let breaks = manual_breaks.unwrap();
                assert_eq!(breaks.len(), 3);
                assert_eq!(breaks[0].0, 1000.0);
                assert_eq!(breaks[1].0, 5000.0);
                assert_eq!(breaks[2].0, f64::MAX);
            }
            _ => panic!("expected ClassBreaks"),
        }
    }

    #[test]
    fn parse_gs_auto() {
        let def = parse_style_dsl("gs(magnitude)").unwrap();
        match def {
            StyleDefinition::GraduatedSymbol { field, .. } => {
                assert_eq!(field, "magnitude");
            }
            _ => panic!("expected GraduatedSymbol"),
        }
    }

    #[test]
    fn parse_number_suffix_k() {
        assert_eq!(parse_number_with_suffix("5k").unwrap(), 5000.0);
        assert_eq!(parse_number_with_suffix("1.5K").unwrap(), 1500.0);
        assert_eq!(parse_number_with_suffix("2m").unwrap(), 2_000_000.0);
        assert_eq!(parse_number_with_suffix("42").unwrap(), 42.0);
        assert_eq!(parse_number_with_suffix("0.5").unwrap(), 0.5);
    }

    #[test]
    fn interpolate_ramp_correct_count() {
        let colors = interpolate_ramp(&RAMP_YLORRD, 5);
        assert_eq!(colors.len(), 5);
        // First should be close to first ramp stop
        assert_eq!(colors[0], RAMP_YLORRD[0]);
        // Last should be close to last ramp stop
        assert_eq!(colors[4], RAMP_YLORRD[7]);
    }

    #[test]
    fn interpolate_ramp_single() {
        let colors = interpolate_ramp(&RAMP_BLUES, 1);
        assert_eq!(colors.len(), 1);
        // Should be the middle color
        assert_eq!(colors[0], RAMP_BLUES[4]);
    }

    #[test]
    fn custom_gradient_two_colors() {
        let colors = custom_gradient("FFF:000", 3).unwrap();
        assert_eq!(colors.len(), 3);
        // First = white
        assert_eq!(colors[0], [255, 255, 255, 255]);
        // Last = black
        assert_eq!(colors[2], [0, 0, 0, 255]);
        // Middle ≈ gray
        assert_eq!(colors[1], [128, 128, 128, 255]);
    }

    #[test]
    fn resolve_simple_style() {
        let def = parse_style_dsl("f:FF0000;sw:3").unwrap();
        let resolved = resolve_style(&def, None, None, Some(10)).unwrap();
        match resolved {
            ResolvedStyle::Uniform(fs) => {
                assert_eq!(fs.fill_color, Some([255, 0, 0, 255]));
                assert_eq!(fs.stroke_width, 3.0);
            }
            _ => panic!("expected Uniform"),
        }
    }

    #[test]
    fn resolve_cb_auto_with_stats() {
        let def = parse_style_dsl("cb(pop,3,YlOrRd)").unwrap();
        let stats = FieldStats {
            min: 0.0,
            max: 300.0,
        };
        let resolved = resolve_style(&def, Some(&stats), None, Some(10)).unwrap();
        match resolved {
            ResolvedStyle::ClassBreaks { breaks, .. } => {
                assert_eq!(breaks.len(), 3);
                // First break at ~100
                assert!((breaks[0].0 - 100.0).abs() < 0.01);
                // Second break at ~200
                assert!((breaks[1].0 - 200.0).abs() < 0.01);
                // Third break = MAX (catch-all)
                assert_eq!(breaks[2].0, f64::MAX);
            }
            _ => panic!("expected ClassBreaks"),
        }
    }

    #[test]
    fn resolve_uv_auto_with_distinct() {
        let def = parse_style_dsl("uv(state)").unwrap();
        let distinct = FieldDistinct {
            values: vec!["VIC".into(), "NSW".into(), "QLD".into()],
        };
        let resolved = resolve_style(&def, None, Some(&distinct), None).unwrap();
        match resolved {
            ResolvedStyle::UniqueValue { classes, .. } => {
                assert_eq!(classes.len(), 3);
                assert_eq!(classes[0].0, "VIC");
                assert_eq!(classes[1].0, "NSW");
                // Each should have different fill colors (from category10 palette)
                assert_ne!(classes[0].1.fill_color, classes[1].1.fill_color);
            }
            _ => panic!("expected UniqueValue"),
        }
    }

    #[test]
    fn style_for_value_class_breaks() {
        let resolved = ResolvedStyle::ClassBreaks {
            field: "x".into(),
            breaks: vec![
                (10.0, FeatureStyle {
                    fill_color: Some([255, 0, 0, 255]),
                    ..Default::default()
                }),
                (20.0, FeatureStyle {
                    fill_color: Some([0, 255, 0, 255]),
                    ..Default::default()
                }),
                (f64::MAX, FeatureStyle {
                    fill_color: Some([0, 0, 255, 255]),
                    ..Default::default()
                }),
            ],
            fallback: FeatureStyle::default(),
        };

        let s1 = resolved.style_for_value(&StyleValue::F64(5.0));
        assert_eq!(s1.fill_color, Some([255, 0, 0, 255]));

        let s2 = resolved.style_for_value(&StyleValue::F64(15.0));
        assert_eq!(s2.fill_color, Some([0, 255, 0, 255]));

        let s3 = resolved.style_for_value(&StyleValue::F64(25.0));
        assert_eq!(s3.fill_color, Some([0, 0, 255, 255]));
    }

    #[test]
    fn style_for_value_unique() {
        let resolved = ResolvedStyle::UniqueValue {
            field: "type".into(),
            classes: vec![
                ("A".into(), FeatureStyle {
                    fill_color: Some([255, 0, 0, 255]),
                    ..Default::default()
                }),
                ("B".into(), FeatureStyle {
                    fill_color: Some([0, 255, 0, 255]),
                    ..Default::default()
                }),
            ],
            fallback: FeatureStyle {
                fill_color: Some([128, 128, 128, 255]),
                ..Default::default()
            },
        };

        let sa = resolved.style_for_value(&StyleValue::Str("A".into()));
        assert_eq!(sa.fill_color, Some([255, 0, 0, 255]));

        let sb = resolved.style_for_value(&StyleValue::Str("B".into()));
        assert_eq!(sb.fill_color, Some([0, 255, 0, 255]));

        // Unknown value gets fallback
        let sc = resolved.style_for_value(&StyleValue::Str("C".into()));
        assert_eq!(sc.fill_color, Some([128, 128, 128, 255]));
    }

    #[test]
    fn style_for_value_graduated() {
        let resolved = ResolvedStyle::GraduatedSymbol {
            field: "mag".into(),
            value_min: 0.0,
            value_max: 10.0,
            radius_min: 2.0,
            radius_max: 20.0,
            base_style: FeatureStyle::default(),
        };

        let s0 = resolved.style_for_value(&StyleValue::F64(0.0));
        assert_eq!(s0.point_radius, 2.0);

        let s10 = resolved.style_for_value(&StyleValue::F64(10.0));
        assert_eq!(s10.point_radius, 20.0);

        let s5 = resolved.style_for_value(&StyleValue::F64(5.0));
        assert!((s5.point_radius - 11.0).abs() < 0.01);
    }

    #[test]
    fn style_hash_stable() {
        let h1 = style_hash("cb(population,5,YlOrRd)");
        let h2 = style_hash("cb(population,5,YlOrRd)");
        assert_eq!(h1, h2);

        let h3 = style_hash("cb(population,5,Blues)");
        assert_ne!(h1, h3);
    }

    #[test]
    fn legacy_json_to_resolved() {
        let json = serde_json::json!({
            "fill": "rgba(255,0,0,128)",
            "stroke": "#000000",
            "stroke_width": 2.0,
        });
        let resolved = legacy_style_to_resolved(&json, Some(10));
        match resolved {
            ResolvedStyle::Uniform(fs) => {
                assert_eq!(fs.fill_color, Some([255, 0, 0, 128]));
                assert_eq!(fs.stroke_width, 2.0);
            }
            _ => panic!("expected Uniform"),
        }
    }

    #[test]
    fn parse_style_value_dsl_string() {
        let json = serde_json::json!("cb(population,5,YlOrRd)");
        match parse_style_value(&json).unwrap() {
            StyleSource::Dsl(def) => {
                assert!(matches!(def, StyleDefinition::ClassBreaks { .. }));
            }
            _ => panic!("expected Dsl"),
        }
    }

    #[test]
    fn parse_style_value_legacy_object() {
        let json = serde_json::json!({"fill": "#FF0000"});
        match parse_style_value(&json).unwrap() {
            StyleSource::LegacyJson => {}
            _ => panic!("expected LegacyJson"),
        }
    }

    #[test]
    fn needs_stats_and_distinct() {
        let cb = parse_style_dsl("cb(pop,5)").unwrap();
        assert!(cb.needs_stats());
        assert!(!cb.needs_distinct());

        let uv = parse_style_dsl("uv(type)").unwrap();
        assert!(!uv.needs_stats());
        assert!(uv.needs_distinct());

        let simple = parse_style_dsl("f:FF0000").unwrap();
        assert!(!simple.needs_stats());
        assert!(!simple.needs_distinct());

        let cb_manual = parse_style_dsl("cb(pop):~100=f:FFF,+=f:000").unwrap();
        assert!(!cb_manual.needs_stats());

        let uv_manual = parse_style_dsl("uv(x):a=f:FFF,b=f:000").unwrap();
        assert!(!uv_manual.needs_distinct());
    }

    #[test]
    fn field_name_extraction() {
        assert_eq!(parse_style_dsl("cb(pop,5)").unwrap().field_name(), Some("pop"));
        assert_eq!(parse_style_dsl("uv(type)").unwrap().field_name(), Some("type"));
        assert_eq!(parse_style_dsl("gs(mag)").unwrap().field_name(), Some("mag"));
        assert_eq!(parse_style_dsl("f:FF0000").unwrap().field_name(), None);
    }

    #[test]
    fn parse_error_cases() {
        assert!(parse_style_dsl("").is_err());
        assert!(parse_style_dsl("uv()").is_err());
        assert!(parse_style_dsl("cb()").is_err());
        assert!(parse_style_dsl("gs()").is_err());
    }
}

//! Closed choices and ceilings every node shares (`node-conventions.md` §2).
//!
//! A choice flag accepts only its listed words; a size a node produces has a
//! maximum. Both are refused under the calling node's code, so a pipeline
//! learns which node and which flag, and nothing falls back to a default
//! word or an unbounded allocation.

use crate::pipeline::PipelineError;

/// A raster side may be this long, for every node that draws or resizes.
pub const MAX_RASTER_SIDE: u32 = 8192;
/// A raster may have this many pixels.
pub const MAX_RASTER_PIXELS: u64 = 40_000_000;

/// One of `words`, or `default` when the flag is empty. Any other word is
/// refused — never mapped to a default.
pub fn choice<'a>(
    value: &str,
    words: &[&'a str],
    default: &'a str,
    flag: &str,
    code: &'static str,
) -> Result<&'a str, PipelineError> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(default);
    }
    words
        .iter()
        .find(|word| word.eq_ignore_ascii_case(value))
        .copied()
        .ok_or_else(|| PipelineError::new(code, format!("{flag} '{value}' must be one of {}", words.join(", "))))
}

/// A raster of `width` × `height`, refused over the shared ceiling.
pub fn raster(width: u32, height: u32, code: &'static str) -> Result<(u32, u32), PipelineError> {
    if width == 0 || height == 0 {
        return Err(PipelineError::new(code, "a raster needs a width and a height above 0"));
    }
    if width > MAX_RASTER_SIDE || height > MAX_RASTER_SIDE || u64::from(width) * u64::from(height) > MAX_RASTER_PIXELS {
        return Err(PipelineError::new(
            code,
            format!("a {width}×{height} raster is over the ceiling ({MAX_RASTER_SIDE} px a side, {MAX_RASTER_PIXELS} pixels)"),
        ));
    }
    Ok((width, height))
}

/// A whole-number flag as the DSL or the editor sends it — a number, or the
/// number as text; unset or empty is `None`. Anything else is refused.
pub fn whole(value: &serde_json::Value, flag: &str, code: &'static str) -> Result<Option<u32>, PipelineError> {
    use serde_json::Value;
    let bad = || PipelineError::new(code, format!("{flag} '{value}' is not a whole number"));
    match value {
        Value::Null => Ok(None),
        Value::String(s) if s.trim().is_empty() => Ok(None),
        Value::String(s) => s.trim().parse::<u32>().map(Some).map_err(|_| bad()),
        Value::Number(n) => n.as_u64().and_then(|n| u32::try_from(n).ok()).map(Some).ok_or_else(bad),
        _ => Err(bad()),
    }
}

/// A number inside `min..=max`, refused outside it.
pub fn within<T: PartialOrd + std::fmt::Display + Copy>(
    value: T,
    min: T,
    max: T,
    flag: &str,
    code: &'static str,
) -> Result<T, PipelineError> {
    if value < min || value > max {
        return Err(PipelineError::new(code, format!("{flag} {value} must be between {min} and {max}")));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{choice, raster, whole, within};

    #[test]
    fn a_choice_is_closed_and_a_size_has_a_ceiling() {
        assert_eq!(choice("", &["cover", "contain"], "cover", "--fit", "T").unwrap(), "cover");
        assert_eq!(choice("Contain", &["cover", "contain"], "cover", "--fit", "T").unwrap(), "contain");
        assert!(choice("stretch", &["cover", "contain"], "cover", "--fit", "T").is_err());
        assert!(raster(8192, 4096, "T").is_ok());
        assert!(raster(8193, 10, "T").is_err());
        assert!(raster(8000, 8000, "T").is_err(), "64M pixels is over 40M");
        assert!(raster(0, 10, "T").is_err());
        assert_eq!(within(300, 36, 600, "--dpi", "T").unwrap(), 300);
        assert!(within(1200, 36, 600, "--dpi", "T").is_err());
        assert_eq!(whole(&serde_json::json!("256"), "--width", "T").unwrap(), Some(256));
        assert_eq!(whole(&serde_json::json!(80), "--width", "T").unwrap(), Some(80));
        assert_eq!(whole(&serde_json::json!(" "), "--width", "T").unwrap(), None);
        assert!(whole(&serde_json::json!("wide"), "--width", "T").is_err());
        assert!(whole(&serde_json::json!(-1), "--width", "T").is_err());
    }
}

//! Durations and sizes in flag values (`node-conventions.md` §3).
//!
//! A unit travels in the value, never in the flag's name: `--timeout 30s`,
//! `--max-size 10MB`. This is the one parser every node reads them with, so
//! `10MB` means the same bytes on every node. A bare number is refused: it
//! names no unit, and a guessed one is how `--timeout 30` meant thirty
//! milliseconds on one node and thirty seconds on the next.

use std::time::Duration;

use crate::pipeline::PipelineError;

/// Duration units, longest suffix first so `ms` is not read as `m`.
const DURATION_UNITS: &[(&str, u64)] = &[
    ("ms", 1),
    ("s", 1_000),
    ("m", 60_000),
    ("h", 3_600_000),
    ("d", 86_400_000),
];

/// Size units: decimal (×1000) and binary (×1024), longest suffix first.
const SIZE_UNITS: &[(&str, u64)] = &[
    ("KiB", 1 << 10),
    ("MiB", 1 << 20),
    ("GiB", 1 << 30),
    ("KB", 1_000),
    ("MB", 1_000_000),
    ("GB", 1_000_000_000),
    ("B", 1),
];

/// `500ms`, `30s`, `5m`, `1h`, `7d`; a decimal amount is allowed (`1.5s`).
pub fn duration(value: &str, flag: &str, code: &'static str) -> Result<Duration, PipelineError> {
    let millis = amount(value, DURATION_UNITS, flag, code, "a duration such as 500ms, 30s, 5m, 1h or 7d")?;
    Ok(Duration::from_millis(millis))
}

/// `512B`, `10KB`, `10MB`, `1GB` (×1000) or `64KiB`, `10MiB`, `1GiB` (×1024).
pub fn size(value: &str, flag: &str, code: &'static str) -> Result<u64, PipelineError> {
    amount(value, SIZE_UNITS, flag, code, "a size such as 512B, 10MB or 10MiB")
}

fn amount(
    value: &str,
    units: &[(&str, u64)],
    flag: &str,
    code: &'static str,
    expected: &str,
) -> Result<u64, PipelineError> {
    let value = value.trim();
    let refuse = || PipelineError::new(code, format!("{flag} '{value}' must be {expected}"));
    let (number, factor) = units
        .iter()
        .filter(|(suffix, _)| value.ends_with(suffix))
        .max_by_key(|(suffix, _)| suffix.len())
        .map(|(suffix, factor)| (value[..value.len() - suffix.len()].trim(), *factor))
        .ok_or_else(refuse)?;
    if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit() || byte == b'.') {
        return Err(refuse());
    }
    let number: f64 = number.parse().map_err(|_| refuse())?;
    let total = number * factor as f64;
    if !total.is_finite() || total < 0.0 || total > u64::MAX as f64 {
        return Err(refuse());
    }
    Ok(total.round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CODE: &str = "TEST_UNIT";

    #[test]
    fn durations_read_every_unit() {
        assert_eq!(duration("500ms", "--timeout", CODE).unwrap(), Duration::from_millis(500));
        assert_eq!(duration("30s", "--timeout", CODE).unwrap(), Duration::from_secs(30));
        assert_eq!(duration("5m", "--timeout", CODE).unwrap(), Duration::from_secs(300));
        assert_eq!(duration("1h", "--timeout", CODE).unwrap(), Duration::from_secs(3600));
        assert_eq!(duration("7d", "--ttl", CODE).unwrap(), Duration::from_secs(7 * 86_400));
        assert_eq!(duration("1.5s", "--timeout", CODE).unwrap(), Duration::from_millis(1500));
    }

    #[test]
    fn sizes_read_decimal_and_binary_units() {
        assert_eq!(size("512B", "--max-size", CODE).unwrap(), 512);
        assert_eq!(size("10KB", "--max-size", CODE).unwrap(), 10_000);
        assert_eq!(size("10MB", "--max-size", CODE).unwrap(), 10_000_000);
        assert_eq!(size("1GB", "--max-size", CODE).unwrap(), 1_000_000_000);
        assert_eq!(size("64KiB", "--max-size", CODE).unwrap(), 65_536);
        assert_eq!(size("10MiB", "--max-size", CODE).unwrap(), 10 * 1_048_576);
        assert_eq!(size("1GiB", "--max-size", CODE).unwrap(), 1_073_741_824);
    }

    #[test]
    fn a_value_without_a_unit_is_refused() {
        for bad in ["30", "", "s", "-5s", "10 parsecs", "1e3s", "10mb"] {
            let err = duration(bad, "--timeout", CODE).err().or_else(|| size(bad, "--max-size", CODE).err());
            assert!(err.is_some(), "'{bad}' must be refused");
        }
        let err = size("10", "--max-size", CODE).unwrap_err();
        assert!(err.message.contains("--max-size '10' must be a size"), "{}", err.message);
    }
}

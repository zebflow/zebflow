//! Statistics of a GeoJSON file layer, computed when asked.
//!
//! A GeoJSON file is scanned only up to [`MAX_SCAN_BYTES`], so the work is
//! small; the answer is kept in memory until the file changes. Every number
//! here is exact.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::Value;

use super::compute::{HISTOGRAM_BINS, TOP_VALUES, TOP_VALUES_MAX_DISTINCT};
use super::saved::{ColumnStats, Fingerprint, Histogram, LayerStats, now_rfc3339};

/// GeoJSON files larger than this are not scanned.
pub const MAX_SCAN_BYTES: u64 = 50 * 1024 * 1024;
/// How many files' statistics are kept; past it the cache starts over.
const CACHE_ENTRIES: usize = 256;

/// A computed answer and when it was computed, or why there is none.
pub type Answer = Arc<Result<(LayerStats, String), String>>;

/// The statistics of the GeoJSON file at `source`, from the cache while the
/// file is unchanged.
pub fn stats(source: &Path) -> Answer {
    let Some(fingerprint) = Fingerprint::of(source) else {
        return Arc::new(Err("the layer's file cannot be read".to_string()));
    };
    let key = format!(
        "{}\n{}\n{}",
        source.display(),
        fingerprint.size,
        fingerprint.tail_sha256
    );
    if let Some(hit) = cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&key)
        .cloned()
    {
        return hit;
    }
    let answer: Answer = Arc::new(if fingerprint.size > MAX_SCAN_BYTES {
        Err(
            "statistics are not computed for a GeoJSON file over 50 MB; publish it as GeoParquet"
                .to_string(),
        )
    } else {
        compute(source).map(|stats| (stats, now_rfc3339()))
    });
    let mut cache = cache().lock().unwrap_or_else(|e| e.into_inner());
    if cache.len() >= CACHE_ENTRIES {
        cache.clear();
    }
    cache.insert(key, answer.clone());
    answer
}

fn cache() -> &'static Mutex<HashMap<String, Answer>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Answer>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// One property's running counts.
#[derive(Default)]
struct Accum {
    data_type: String,
    non_null: u64,
    min: Option<f64>,
    max: Option<f64>,
    /// Each distinct value (as text) with its count.
    values: HashMap<String, u64>,
}

fn compute(source: &Path) -> Result<LayerStats, String> {
    let mut row_count = 0u64;
    let mut extent: Option<[f64; 4]> = None;
    let mut order: Vec<String> = Vec::new();
    let mut accum: HashMap<String, Accum> = HashMap::new();

    crate::mapserver::infra::geojson_stream::stream_feature_collection_from_path(
        source,
        |feature| {
            row_count += 1;
            if let Some(bbox) = feature
                .get("geometry")
                .and_then(crate::mapserver::infra::bbox::geometry_bbox)
            {
                extent = Some(match extent {
                    None => bbox,
                    Some(e) => [
                        e[0].min(bbox[0]),
                        e[1].min(bbox[1]),
                        e[2].max(bbox[2]),
                        e[3].max(bbox[3]),
                    ],
                });
            }
            let Some(props) = feature.get("properties").and_then(Value::as_object) else {
                return Ok(());
            };
            for (key, value) in props {
                if !accum.contains_key(key) {
                    order.push(key.clone());
                }
                let acc = accum.entry(key.clone()).or_default();
                if value.is_null() {
                    continue;
                }
                acc.non_null += 1;
                merge_type(&mut acc.data_type, value);
                if let Some(n) = value.as_f64().filter(|n| n.is_finite()) {
                    acc.min = Some(acc.min.map_or(n, |m| m.min(n)));
                    acc.max = Some(acc.max.map_or(n, |m| m.max(n)));
                }
                let text = value
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| value.to_string());
                *acc.values.entry(text).or_insert(0) += 1;
            }
            Ok(())
        },
    )?;

    let columns = order
        .into_iter()
        .filter_map(|name| {
            let acc = accum.remove(&name)?;
            Some(column_stats(name, acc, row_count))
        })
        .collect();
    Ok(LayerStats {
        row_count,
        extent,
        columns,
    })
}

fn column_stats(name: String, acc: Accum, row_count: u64) -> ColumnStats {
    let numeric = matches!(acc.data_type.as_str(), "Int64" | "Float64");
    let histogram = match (numeric, acc.min, acc.max) {
        (true, Some(lo), Some(hi)) => Some(histogram(&acc.values, lo, hi)),
        _ => None,
    };
    let cardinality = acc.values.len() as u64;
    let (top_values, top_values_skipped) = if cardinality <= TOP_VALUES_MAX_DISTINCT {
        let mut pairs: Vec<(String, u64)> = acc.values.into_iter().collect();
        pairs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        pairs.truncate(TOP_VALUES);
        (pairs, None)
    } else {
        (Vec::new(), Some("too_many_distinct".to_string()))
    };
    ColumnStats {
        name,
        data_type: if acc.data_type.is_empty() {
            "Null".to_string()
        } else {
            acc.data_type
        },
        null_count: row_count.saturating_sub(acc.non_null),
        cardinality,
        cardinality_exact: true,
        min: if numeric { acc.min } else { None },
        max: if numeric { acc.max } else { None },
        top_values,
        top_values_skipped,
        histogram,
    }
}

/// The type a property's values share: one JSON type throughout, numbers of
/// both kinds as `Float64`, anything else mixed as `Utf8`.
fn merge_type(current: &mut String, value: &Value) {
    let this = if value.is_boolean() {
        "Boolean"
    } else if value.is_i64() || value.is_u64() {
        "Int64"
    } else if value.is_f64() {
        "Float64"
    } else {
        "Utf8"
    };
    if current.is_empty() {
        *current = this.to_string();
    } else if current != this {
        let numbers = matches!(
            (current.as_str(), this),
            ("Int64", "Float64") | ("Float64", "Int64")
        );
        *current = if numbers { "Float64" } else { "Utf8" }.to_string();
    }
}

/// Equal-width bins from `lo` to `hi`, counted from each distinct value's count.
fn histogram(values: &HashMap<String, u64>, lo: f64, hi: f64) -> Histogram {
    let mut counts = vec![0u64; HISTOGRAM_BINS];
    let width = (hi - lo) / HISTOGRAM_BINS as f64;
    for (text, n) in values {
        let Some(v) = text.parse::<f64>().ok().filter(|v| v.is_finite()) else {
            continue;
        };
        let bin = if width > 0.0 && width.is_finite() {
            ((v - lo) / width).floor() as i64
        } else {
            0
        };
        counts[usize::try_from(bin.max(0))
            .unwrap_or(0)
            .min(HISTOGRAM_BINS - 1)] += n;
    }
    Histogram {
        min: lo,
        max: hi,
        counts,
    }
}

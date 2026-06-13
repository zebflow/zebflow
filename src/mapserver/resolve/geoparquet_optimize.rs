//! GeoParquet spatial optimizer.
//!
//! Reads a GeoParquet file, adds bbox covering columns (`xmin`, `ymin`, `xmax`, `ymax`),
//! sorts rows by Hilbert curve index for spatial locality, and writes an optimized
//! parquet file with small row groups (~10K) and ZSTD compression.
//!
//! After optimization, DataFusion's `pushdown_filters` can prune 90%+ of row groups
//! via column statistics on the bbox columns, dramatically reducing query time for
//! spatial tile queries.

use std::collections::HashMap;
use std::path::Path;

use datafusion::arrow::array::*;
use datafusion::arrow::datatypes::{DataType, Schema};
use datafusion::arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde::{Deserialize, Serialize};

/// Per-column statistics computed during optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnStats {
    pub name: String,
    pub data_type: String,
    pub null_count: usize,
    pub cardinality: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub top_values: Vec<(String, usize)>,
}

/// Sidecar stats file format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnStatsReport {
    pub row_count: usize,
    pub columns: Vec<ColumnStats>,
}

/// Statistics reported after optimization.
pub struct OptimizeReport {
    pub rows: usize,
    pub row_groups: usize,
    pub extent: [f64; 4], // xmin, ymin, xmax, ymax
    pub source_bytes: u64,
    pub dest_bytes: u64,
    pub geom_col: String,
    pub column_stats: Vec<ColumnStats>,
}

/// Optimize a GeoParquet file for spatial tile queries.
///
/// Delegates spatial optimization (bbox columns, Hilbert sort, ZSTD compression,
/// small row groups) to `geonative_geoparquet::optimize()`, then computes column
/// statistics from the optimized output.
pub fn optimize_geoparquet(source: &Path, dest: &Path) -> Result<OptimizeReport, String> {
    let source_bytes = std::fs::metadata(source).map(|m| m.len()).unwrap_or(0);

    // Core optimization: geonative handles bbox columns, Hilbert sort, ZSTD, row groups
    let gn_report = geonative_geoparquet::optimize(
        source,
        dest,
        geonative_geoparquet::OptimizeOptions { batch_size: 10_000 },
    )
    .map_err(|e| format!("optimize failed: {e}"))?;

    // Column stats + extent: computed from the optimized file
    let (column_stats, geom_col, extent) = compute_column_stats_from_file(dest)?;

    let num_rows = gn_report.features as usize;
    let row_groups = (num_rows + 9_999) / 10_000;

    Ok(OptimizeReport {
        rows: num_rows,
        row_groups,
        extent,
        source_bytes,
        dest_bytes: gn_report.output_bytes,
        geom_col,
        column_stats,
    })
}

/// Read an optimized parquet file and compute column stats + extent.
fn compute_column_stats_from_file(
    path: &Path,
) -> Result<(Vec<ColumnStats>, String, [f64; 4]), String> {
    use datafusion::arrow::compute::concat_batches;

    let file =
        std::fs::File::open(path).map_err(|e| format!("failed to open optimized parquet: {e}"))?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| format!("failed to create parquet reader: {e}"))?
        .with_batch_size(65536)
        .build()
        .map_err(|e| format!("failed to build parquet reader: {e}"))?;

    let schema = reader.schema().clone();
    let batches: Vec<RecordBatch> = reader
        .collect::<Result<_, _>>()
        .map_err(|e| format!("failed to read parquet batches: {e}"))?;

    if batches.is_empty() {
        return Err("optimized parquet has no data".to_string());
    }

    let combined = concat_batches(&schema, &batches)
        .map_err(|e| format!("failed to concatenate batches: {e}"))?;

    let geom_idx =
        detect_geom_column_index(&schema).ok_or("no geometry column found in optimized file")?;
    let geom_col = schema.field(geom_idx).name().clone();
    let column_stats = compute_column_stats(&combined, geom_idx);

    // Read extent from bbox columns (always present after optimization)
    let extent = read_extent_from_bbox_columns(&combined);

    Ok((column_stats, geom_col, extent))
}

/// Read the dataset extent from the xmin/ymin/xmax/ymax bbox columns.
fn read_extent_from_bbox_columns(batch: &RecordBatch) -> [f64; 4] {
    let schema = batch.schema();
    let mut extent = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];

    let xmin_idx = schema.column_with_name("xmin").map(|(i, _)| i);
    let ymin_idx = schema.column_with_name("ymin").map(|(i, _)| i);
    let xmax_idx = schema.column_with_name("xmax").map(|(i, _)| i);
    let ymax_idx = schema.column_with_name("ymax").map(|(i, _)| i);

    let (Some(xi), Some(yi), Some(xa), Some(ya)) = (xmin_idx, ymin_idx, xmax_idx, ymax_idx) else {
        return extent;
    };

    let xmin_arr = batch.column(xi).as_any().downcast_ref::<Float64Array>();
    let ymin_arr = batch.column(yi).as_any().downcast_ref::<Float64Array>();
    let xmax_arr = batch.column(xa).as_any().downcast_ref::<Float64Array>();
    let ymax_arr = batch.column(ya).as_any().downcast_ref::<Float64Array>();

    let (Some(xmn), Some(ymn), Some(xmx), Some(ymx)) = (xmin_arr, ymin_arr, xmax_arr, ymax_arr)
    else {
        return extent;
    };

    for i in 0..batch.num_rows() {
        if xmn.is_null(i) {
            continue;
        }
        let x0 = xmn.value(i);
        let y0 = ymn.value(i);
        let x1 = xmx.value(i);
        let y1 = ymx.value(i);
        if x0 < extent[0] {
            extent[0] = x0;
        }
        if y0 < extent[1] {
            extent[1] = y0;
        }
        if x1 > extent[2] {
            extent[2] = x1;
        }
        if y1 > extent[3] {
            extent[3] = y1;
        }
    }
    extent
}

// ── Column statistics ─────────────────────────────────────────────────

const MAX_DISTINCT_TRACK: usize = 1000;
const MAX_TOP_VALUES: usize = 50;
const TOP_VALUES_CARDINALITY_LIMIT: usize = 500;

/// Compute per-column statistics from a combined RecordBatch.
/// Skips geometry columns and bbox covering columns.
pub fn compute_column_stats(combined: &RecordBatch, geom_idx: usize) -> Vec<ColumnStats> {
    let schema = combined.schema();
    let skip_names: &[&str] = &[
        "xmin", "ymin", "xmax", "ymax", "min_x", "min_y", "max_x", "max_y",
    ];
    let num_rows = combined.num_rows();
    let mut stats = Vec::new();

    for (col_idx, field) in schema.fields().iter().enumerate() {
        if col_idx == geom_idx {
            continue;
        }
        let name = field.name().as_str();
        if skip_names.contains(&name) {
            continue;
        }
        if GEOM_NAMES.contains(&name) {
            continue;
        }
        // Skip binary columns (likely geometry)
        if matches!(field.data_type(), DataType::Binary | DataType::LargeBinary) {
            continue;
        }

        let col = combined.column(col_idx);
        let data_type = format!("{:?}", field.data_type());
        let null_count = col.null_count();

        let mut value_counts: HashMap<String, usize> = HashMap::new();
        let mut cardinality_capped = false;
        let mut min_val: Option<f64> = None;
        let mut max_val: Option<f64> = None;

        match field.data_type() {
            DataType::Utf8 => {
                if let Some(arr) = col.as_any().downcast_ref::<StringArray>() {
                    for i in 0..num_rows {
                        if arr.is_null(i) {
                            continue;
                        }
                        let v = arr.value(i);
                        if !cardinality_capped {
                            let entry = value_counts.entry(v.to_string()).or_insert(0);
                            *entry += 1;
                            if value_counts.len() > MAX_DISTINCT_TRACK {
                                cardinality_capped = true;
                            }
                        } else {
                            if let Some(entry) = value_counts.get_mut(v) {
                                *entry += 1;
                            }
                        }
                    }
                }
            }
            DataType::Int64 => {
                if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                    for i in 0..num_rows {
                        if arr.is_null(i) {
                            continue;
                        }
                        let v = arr.value(i);
                        let fv = v as f64;
                        min_val = Some(min_val.map_or(fv, |m: f64| m.min(fv)));
                        max_val = Some(max_val.map_or(fv, |m: f64| m.max(fv)));
                        if !cardinality_capped {
                            let key = v.to_string();
                            let entry = value_counts.entry(key).or_insert(0);
                            *entry += 1;
                            if value_counts.len() > MAX_DISTINCT_TRACK {
                                cardinality_capped = true;
                            }
                        }
                    }
                }
            }
            DataType::Int32 => {
                if let Some(arr) = col.as_any().downcast_ref::<Int32Array>() {
                    for i in 0..num_rows {
                        if arr.is_null(i) {
                            continue;
                        }
                        let v = arr.value(i);
                        let fv = v as f64;
                        min_val = Some(min_val.map_or(fv, |m: f64| m.min(fv)));
                        max_val = Some(max_val.map_or(fv, |m: f64| m.max(fv)));
                        if !cardinality_capped {
                            let key = v.to_string();
                            let entry = value_counts.entry(key).or_insert(0);
                            *entry += 1;
                            if value_counts.len() > MAX_DISTINCT_TRACK {
                                cardinality_capped = true;
                            }
                        }
                    }
                }
            }
            DataType::Float64 => {
                if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                    for i in 0..num_rows {
                        if arr.is_null(i) {
                            continue;
                        }
                        let v = arr.value(i);
                        if v.is_finite() {
                            min_val = Some(min_val.map_or(v, |m: f64| m.min(v)));
                            max_val = Some(max_val.map_or(v, |m: f64| m.max(v)));
                        }
                        if !cardinality_capped {
                            let key = if v == (v as i64) as f64 {
                                format!("{}", v as i64)
                            } else {
                                format!("{v}")
                            };
                            let entry = value_counts.entry(key).or_insert(0);
                            *entry += 1;
                            if value_counts.len() > MAX_DISTINCT_TRACK {
                                cardinality_capped = true;
                            }
                        }
                    }
                }
            }
            DataType::Float32 => {
                if let Some(arr) = col.as_any().downcast_ref::<Float32Array>() {
                    for i in 0..num_rows {
                        if arr.is_null(i) {
                            continue;
                        }
                        let v = arr.value(i) as f64;
                        if v.is_finite() {
                            min_val = Some(min_val.map_or(v, |m: f64| m.min(v)));
                            max_val = Some(max_val.map_or(v, |m: f64| m.max(v)));
                        }
                        if !cardinality_capped {
                            let key = format!("{v}");
                            let entry = value_counts.entry(key).or_insert(0);
                            *entry += 1;
                            if value_counts.len() > MAX_DISTINCT_TRACK {
                                cardinality_capped = true;
                            }
                        }
                    }
                }
            }
            DataType::Boolean => {
                if let Some(arr) = col.as_any().downcast_ref::<BooleanArray>() {
                    for i in 0..num_rows {
                        if arr.is_null(i) {
                            continue;
                        }
                        let key = if arr.value(i) { "true" } else { "false" };
                        let entry = value_counts.entry(key.to_string()).or_insert(0);
                        *entry += 1;
                    }
                }
            }
            _ => continue, // Skip unsupported column types
        }

        let cardinality = value_counts.len();

        // Build top_values for low-cardinality columns
        let top_values = if cardinality <= TOP_VALUES_CARDINALITY_LIMIT && !cardinality_capped {
            let mut pairs: Vec<(String, usize)> = value_counts.into_iter().collect();
            pairs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            pairs.truncate(MAX_TOP_VALUES);
            pairs
        } else {
            Vec::new()
        };

        stats.push(ColumnStats {
            name: name.to_string(),
            data_type,
            null_count,
            cardinality,
            min: min_val,
            max: max_val,
            top_values,
        });
    }

    stats
}

// ── Geometry column detection ─────────────────────────────────────────

const GEOM_NAMES: &[&str] = &["geometry", "geom", "wkb_geometry", "the_geom", "shape"];

fn detect_geom_column_index(schema: &Schema) -> Option<usize> {
    // Try known names first
    for name in GEOM_NAMES {
        if let Some((idx, _)) = schema.column_with_name(name) {
            return Some(idx);
        }
    }
    // Fall back to first Binary/LargeBinary column
    for (idx, field) in schema.fields().iter().enumerate() {
        if matches!(field.data_type(), DataType::Binary | DataType::LargeBinary) {
            return Some(idx);
        }
    }
    None
}

// ── GeoJSON geometry → WKB encoder (via geonative) ────────────────────

/// Encode a GeoJSON geometry object to WKB binary.
/// Delegates to geonative_geojson → geonative_core WKB codec.
pub fn geojson_geometry_to_wkb(geometry: &serde_json::Value) -> Option<Vec<u8>> {
    let ir = geonative_geojson::geometry::from_json(geometry).ok()?;
    Some(ir.to_wkb())
}

// ── Already-optimized checker ─────────────────────────────────────────

/// Check if a parquet file already has spatial optimization columns
/// (xmin, ymin, xmax, ymax as Float64) and a geometry column.
pub fn is_already_optimized(path: &Path) -> bool {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let builder = match ParquetRecordBatchReaderBuilder::try_new(file) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let schema = builder.schema();

    // Check for all four bbox columns as Float64
    let bbox_names = ["xmin", "ymin", "xmax", "ymax"];
    let has_bbox = bbox_names.iter().all(|name| {
        schema
            .column_with_name(name)
            .map(|(_, f)| matches!(f.data_type(), DataType::Float64))
            .unwrap_or(false)
    });
    if !has_bbox {
        return false;
    }

    // Check for a geometry column
    detect_geom_column_index(schema.as_ref()).is_some()
}

// ── GeoJSON → GeoParquet converter ────────────────────────────────────

/// Report from converting a GeoJSON file to GeoParquet.
pub struct ConvertReport {
    pub feature_count: usize,
    pub column_count: usize,
    pub dest_bytes: u64,
}

/// Convert a GeoJSON FeatureCollection file to a raw GeoParquet file.
///
/// Delegates to `geonative_convert::convert()`. The output is NOT spatially
/// optimized — call `optimize_geoparquet` on the output for bbox columns,
/// Hilbert sort, and small row groups.
pub fn convert_geojson_to_geoparquet(source: &Path, dest: &Path) -> Result<ConvertReport, String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create output directory: {e}"))?;
    }

    let stats =
        geonative_convert::convert(source, dest, geonative_convert::ConvertOptions::default())
            .map_err(|e| format!("convert failed: {e}"))?;

    Ok(ConvertReport {
        feature_count: stats.features as usize,
        column_count: 0,
        dest_bytes: stats.output_bytes,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── GeoJSON → WKB round-trip tests (via geonative) ────────────────

    #[test]
    fn geojson_to_wkb_point_roundtrip() {
        let geojson = serde_json::json!({"type": "Point", "coordinates": [144.9, -37.8]});
        let wkb = geojson_geometry_to_wkb(&geojson).unwrap();
        let bbox = geonative_core::bbox_from_bytes(&wkb).unwrap();
        assert!((bbox[0] - 144.9).abs() < 1e-10);
        assert!((bbox[1] - (-37.8)).abs() < 1e-10);
    }

    #[test]
    fn geojson_to_wkb_polygon_roundtrip() {
        let geojson = serde_json::json!({
            "type": "Polygon",
            "coordinates": [[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0], [0.0, 0.0]]]
        });
        let wkb = geojson_geometry_to_wkb(&geojson).unwrap();
        let bbox = geonative_core::bbox_from_bytes(&wkb).unwrap();
        assert!((bbox[0]).abs() < 1e-10);
        assert!((bbox[1]).abs() < 1e-10);
        assert!((bbox[2] - 10.0).abs() < 1e-10);
        assert!((bbox[3] - 10.0).abs() < 1e-10);
    }

    #[test]
    fn geojson_to_wkb_multipolygon_roundtrip() {
        let geojson = serde_json::json!({
            "type": "MultiPolygon",
            "coordinates": [
                [[[0.0, 0.0], [5.0, 0.0], [5.0, 5.0], [0.0, 5.0], [0.0, 0.0]]],
                [[[10.0, 10.0], [20.0, 10.0], [20.0, 20.0], [10.0, 20.0], [10.0, 10.0]]]
            ]
        });
        let wkb = geojson_geometry_to_wkb(&geojson).unwrap();
        let bbox = geonative_core::bbox_from_bytes(&wkb).unwrap();
        assert!((bbox[0]).abs() < 1e-10);
        assert!((bbox[1]).abs() < 1e-10);
        assert!((bbox[2] - 20.0).abs() < 1e-10);
        assert!((bbox[3] - 20.0).abs() < 1e-10);
    }

    #[test]
    fn geojson_to_wkb_null_returns_none() {
        assert!(geojson_geometry_to_wkb(&serde_json::Value::Null).is_none());
    }

    // ── convert_geojson_to_geoparquet test ─────────────────────────────

    #[test]
    fn convert_geojson_to_geoparquet_basic() {
        let tmp = tempfile::tempdir().expect("tmp");
        let geojson_path = tmp.path().join("test.geojson");
        let parquet_path = tmp.path().join("test.parquet");

        std::fs::write(
            &geojson_path,
            r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "properties": {"name": "A", "value": 42},
                    "geometry": {"type": "Point", "coordinates": [144.9, -37.8]}
                },
                {
                    "type": "Feature",
                    "properties": {"name": "B", "value": 99},
                    "geometry": {"type": "Point", "coordinates": [145.0, -37.7]}
                }
            ]
        }"#,
        )
        .expect("write geojson");

        let report = convert_geojson_to_geoparquet(&geojson_path, &parquet_path).unwrap();
        assert_eq!(report.feature_count, 2);
        assert!(report.dest_bytes > 0);
        assert!(parquet_path.exists());

        // Verify it's a valid parquet with a geometry column
        let file = std::fs::File::open(&parquet_path).unwrap();
        let builder = ParquetRecordBatchReaderBuilder::try_new(file).unwrap();
        let schema = builder.schema();
        assert!(schema.column_with_name("geometry").is_some());
    }

    // ── is_already_optimized test ──────────────────────────────────────

    #[test]
    fn convert_produces_optimizable_parquet() {
        // geonative_convert produces parquet with bbox columns by default,
        // so convert output is detected as already optimized.
        let tmp = tempfile::tempdir().expect("tmp");
        let geojson_path = tmp.path().join("test.geojson");
        let parquet_path = tmp.path().join("test.parquet");

        std::fs::write(
            &geojson_path,
            r#"{
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": {"name": "A"},
                "geometry": {"type": "Point", "coordinates": [144.9, -37.8]}
            }]
        }"#,
        )
        .expect("write geojson");

        convert_geojson_to_geoparquet(&geojson_path, &parquet_path).unwrap();
        // geonative's converter includes bbox columns, so it's detected as optimized
        assert!(is_already_optimized(&parquet_path));
    }

    #[test]
    fn is_already_optimized_true_after_optimize() {
        let tmp = tempfile::tempdir().expect("tmp");
        let geojson_path = tmp.path().join("test.geojson");
        let raw_path = tmp.path().join("test.parquet");
        let opt_path = tmp.path().join("test.spatial.parquet");

        std::fs::write(
            &geojson_path,
            r#"{
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": {"name": "A"},
                "geometry": {"type": "Point", "coordinates": [144.9, -37.8]}
            }]
        }"#,
        )
        .expect("write geojson");

        convert_geojson_to_geoparquet(&geojson_path, &raw_path).unwrap();
        optimize_geoparquet(&raw_path, &opt_path).unwrap();
        assert!(is_already_optimized(&opt_path));
    }
}

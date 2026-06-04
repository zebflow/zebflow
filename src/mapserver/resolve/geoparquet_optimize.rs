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
use std::sync::Arc;

use datafusion::arrow::array::*;
use datafusion::arrow::compute::{concat_batches, sort_to_indices, take};
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::arrow::record_batch::RecordBatch;
use serde::{Deserialize, Serialize};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ArrowWriter;
use parquet::basic::{Compression, ZstdLevel};
use parquet::file::properties::WriterProperties;

const ROW_GROUP_SIZE: usize = 10_000;
const HILBERT_ORDER: u32 = 65536; // 2^16 — 16-bit Hilbert curve resolution

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
/// Reads the source file, computes per-feature bounding boxes, sorts by Hilbert
/// curve index, adds bbox covering columns, and writes an optimized parquet file
/// with small row groups and ZSTD compression.
pub fn optimize_geoparquet(source: &Path, dest: &Path) -> Result<OptimizeReport, String> {
    let source_bytes = std::fs::metadata(source)
        .map(|m| m.len())
        .unwrap_or(0);

    // 1. Read all record batches
    let file = std::fs::File::open(source)
        .map_err(|e| format!("failed to open source parquet: {e}"))?;
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
        return Err("source parquet has no data".to_string());
    }

    // 2. Concatenate into one batch
    let combined = concat_batches(&schema, &batches)
        .map_err(|e| format!("failed to concatenate batches: {e}"))?;
    let num_rows = combined.num_rows();
    drop(batches); // free memory

    // 3. Find geometry column
    let geom_idx = detect_geom_column_index(&schema)
        .ok_or("no geometry column found (expected Binary/LargeBinary named geometry/geom/wkb_geometry/the_geom/shape)")?;
    let geom_col_name = schema.field(geom_idx).name().clone();
    let geom_array = combined.column(geom_idx);

    // 3b. Compute column statistics (zero extra IO — data already in memory)
    let column_stats = compute_column_stats(&combined, geom_idx);

    // 4. Compute bbox per row
    let (xmin_arr, ymin_arr, xmax_arr, ymax_arr) = compute_bboxes(geom_array, num_rows);

    // 5. Compute data extent
    let extent = compute_extent(&xmin_arr, &ymin_arr, &xmax_arr, &ymax_arr);

    // 6. Compute Hilbert indices from bbox centroids
    let hilbert_arr = compute_hilbert_indices(
        &xmin_arr, &ymin_arr, &xmax_arr, &ymax_arr, &extent,
    );

    // 7. Sort by Hilbert index
    let sort_indices = sort_to_indices(&hilbert_arr, None, None)
        .map_err(|e| format!("sort failed: {e}"))?;

    // 8. Build output: reorder existing columns + append bbox columns
    let mut new_fields: Vec<Arc<Field>> = schema.fields().iter().cloned().collect();
    // Only add bbox columns if they don't already exist
    let has_xmin = schema.column_with_name("xmin").is_some();
    if !has_xmin {
        new_fields.push(Arc::new(Field::new("xmin", DataType::Float64, true)));
        new_fields.push(Arc::new(Field::new("ymin", DataType::Float64, true)));
        new_fields.push(Arc::new(Field::new("xmax", DataType::Float64, true)));
        new_fields.push(Arc::new(Field::new("ymax", DataType::Float64, true)));
    }
    let new_schema = Arc::new(Schema::new(new_fields));

    let mut new_columns: Vec<ArrayRef> = Vec::with_capacity(combined.num_columns() + 4);
    for col in combined.columns() {
        new_columns.push(
            take(col, &sort_indices, None)
                .map_err(|e| format!("take failed: {e}"))?,
        );
    }
    if !has_xmin {
        new_columns.push(take(&xmin_arr, &sort_indices, None).map_err(|e| format!("take xmin: {e}"))?);
        new_columns.push(take(&ymin_arr, &sort_indices, None).map_err(|e| format!("take ymin: {e}"))?);
        new_columns.push(take(&xmax_arr, &sort_indices, None).map_err(|e| format!("take xmax: {e}"))?);
        new_columns.push(take(&ymax_arr, &sort_indices, None).map_err(|e| format!("take ymax: {e}"))?);
    }

    let sorted_batch = RecordBatch::try_new(new_schema.clone(), new_columns)
        .map_err(|e| format!("failed to create sorted batch: {e}"))?;

    // 9. Write optimized parquet with small row groups + ZSTD
    let props = WriterProperties::builder()
        .set_compression(Compression::ZSTD(ZstdLevel::try_new(3).unwrap()))
        .set_max_row_group_row_count(Some(ROW_GROUP_SIZE))
        .build();

    let out_file = std::fs::File::create(dest)
        .map_err(|e| format!("failed to create output file: {e}"))?;
    let mut writer = ArrowWriter::try_new(out_file, new_schema, Some(props))
        .map_err(|e| format!("failed to create parquet writer: {e}"))?;

    // Write in row-group-sized chunks
    let total = sorted_batch.num_rows();
    let mut offset = 0;
    while offset < total {
        let len = (total - offset).min(ROW_GROUP_SIZE);
        let chunk = sorted_batch.slice(offset, len);
        writer.write(&chunk).map_err(|e| format!("write failed: {e}"))?;
        offset += len;
    }

    let _metadata = writer.close().map_err(|e| format!("close failed: {e}"))?;

    let dest_bytes = std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
    let row_groups = (num_rows + ROW_GROUP_SIZE - 1) / ROW_GROUP_SIZE;

    Ok(OptimizeReport {
        rows: num_rows,
        row_groups,
        extent,
        source_bytes,
        dest_bytes,
        geom_col: geom_col_name,
        column_stats,
    })
}

// ── Column statistics ─────────────────────────────────────────────────

const MAX_DISTINCT_TRACK: usize = 1000;
const MAX_TOP_VALUES: usize = 50;
const TOP_VALUES_CARDINALITY_LIMIT: usize = 500;

/// Compute per-column statistics from a combined RecordBatch.
/// Skips geometry columns and bbox covering columns.
pub fn compute_column_stats(combined: &RecordBatch, geom_idx: usize) -> Vec<ColumnStats> {
    let schema = combined.schema();
    let skip_names: &[&str] = &["xmin", "ymin", "xmax", "ymax", "min_x", "min_y", "max_x", "max_y"];
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
                        if arr.is_null(i) { continue; }
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
                        if arr.is_null(i) { continue; }
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
                        if arr.is_null(i) { continue; }
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
                        if arr.is_null(i) { continue; }
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
                        if arr.is_null(i) { continue; }
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
                        if arr.is_null(i) { continue; }
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

// ── BBox extraction from WKB ──────────────────────────────────────────

fn compute_bboxes(
    geom_array: &ArrayRef,
    num_rows: usize,
) -> (Float64Array, Float64Array, Float64Array, Float64Array) {
    let mut xmin_builder = Float64Builder::with_capacity(num_rows);
    let mut ymin_builder = Float64Builder::with_capacity(num_rows);
    let mut xmax_builder = Float64Builder::with_capacity(num_rows);
    let mut ymax_builder = Float64Builder::with_capacity(num_rows);

    for i in 0..num_rows {
        if geom_array.is_null(i) {
            xmin_builder.append_null();
            ymin_builder.append_null();
            xmax_builder.append_null();
            ymax_builder.append_null();
            continue;
        }

        let wkb = get_binary_value(geom_array.as_ref(), i);
        match wkb.and_then(|b| wkb_bbox(b)) {
            Some(bbox) => {
                xmin_builder.append_value(bbox[0]);
                ymin_builder.append_value(bbox[1]);
                xmax_builder.append_value(bbox[2]);
                ymax_builder.append_value(bbox[3]);
            }
            None => {
                xmin_builder.append_null();
                ymin_builder.append_null();
                xmax_builder.append_null();
                ymax_builder.append_null();
            }
        }
    }

    (
        xmin_builder.finish(),
        ymin_builder.finish(),
        xmax_builder.finish(),
        ymax_builder.finish(),
    )
}

fn get_binary_value<'a>(array: &'a dyn Array, idx: usize) -> Option<&'a [u8]> {
    if let Some(arr) = array.as_any().downcast_ref::<BinaryArray>() {
        Some(arr.value(idx))
    } else if let Some(arr) = array.as_any().downcast_ref::<LargeBinaryArray>() {
        Some(arr.value(idx))
    } else {
        None
    }
}

/// Extract bounding box from WKB geometry.
/// Returns `[xmin, ymin, xmax, ymax]`.
fn wkb_bbox(wkb: &[u8]) -> Option<[f64; 4]> {
    if wkb.len() < 5 {
        return None;
    }
    let is_le = wkb[0] == 1;
    let geom_type = read_u32(wkb, 1, is_le)?;
    let mut bbox = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    wkb_collect_coords(wkb, 5, is_le, geom_type, &mut bbox)?;
    if bbox[0] <= bbox[2] && bbox[1] <= bbox[3] {
        Some(bbox)
    } else {
        None
    }
}

/// Recursively walk WKB structure and update bbox with all coordinate pairs.
fn wkb_collect_coords(
    wkb: &[u8],
    mut cursor: usize,
    is_le: bool,
    geom_type: u32,
    bbox: &mut [f64; 4],
) -> Option<usize> {
    match geom_type {
        1 => {
            // Point
            let x = read_f64(wkb, cursor, is_le)?;
            let y = read_f64(wkb, cursor + 8, is_le)?;
            update_bbox(bbox, x, y);
            Some(cursor + 16)
        }
        2 => {
            // LineString
            let n = read_u32(wkb, cursor, is_le)? as usize;
            cursor += 4;
            for _ in 0..n {
                let x = read_f64(wkb, cursor, is_le)?;
                let y = read_f64(wkb, cursor + 8, is_le)?;
                update_bbox(bbox, x, y);
                cursor += 16;
            }
            Some(cursor)
        }
        3 => {
            // Polygon
            let n_rings = read_u32(wkb, cursor, is_le)? as usize;
            cursor += 4;
            for _ in 0..n_rings {
                let n_pts = read_u32(wkb, cursor, is_le)? as usize;
                cursor += 4;
                for _ in 0..n_pts {
                    let x = read_f64(wkb, cursor, is_le)?;
                    let y = read_f64(wkb, cursor + 8, is_le)?;
                    update_bbox(bbox, x, y);
                    cursor += 16;
                }
            }
            Some(cursor)
        }
        4 | 5 | 6 => {
            // MultiPoint (4), MultiLineString (5), MultiPolygon (6)
            let n = read_u32(wkb, cursor, is_le)? as usize;
            cursor += 4;
            for _ in 0..n {
                if cursor + 5 > wkb.len() {
                    return None;
                }
                let inner_le = wkb[cursor] == 1;
                let inner_type = read_u32(wkb, cursor + 1, inner_le)?;
                cursor += 5;
                cursor = wkb_collect_coords(wkb, cursor, inner_le, inner_type, bbox)?;
            }
            Some(cursor)
        }
        _ => None, // Unknown geometry type
    }
}

#[inline]
fn update_bbox(bbox: &mut [f64; 4], x: f64, y: f64) {
    if x < bbox[0] { bbox[0] = x; }
    if y < bbox[1] { bbox[1] = y; }
    if x > bbox[2] { bbox[2] = x; }
    if y > bbox[3] { bbox[3] = y; }
}

fn read_f64(wkb: &[u8], offset: usize, is_le: bool) -> Option<f64> {
    if offset + 8 > wkb.len() {
        return None;
    }
    let bytes: [u8; 8] = wkb[offset..offset + 8].try_into().ok()?;
    Some(if is_le { f64::from_le_bytes(bytes) } else { f64::from_be_bytes(bytes) })
}

fn read_u32(wkb: &[u8], offset: usize, is_le: bool) -> Option<u32> {
    if offset + 4 > wkb.len() {
        return None;
    }
    let bytes: [u8; 4] = wkb[offset..offset + 4].try_into().ok()?;
    Some(if is_le { u32::from_le_bytes(bytes) } else { u32::from_be_bytes(bytes) })
}

// ── Data extent ───────────────────────────────────────────────────────

fn compute_extent(
    xmin: &Float64Array,
    ymin: &Float64Array,
    xmax: &Float64Array,
    ymax: &Float64Array,
) -> [f64; 4] {
    let mut extent = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for i in 0..xmin.len() {
        if xmin.is_null(i) {
            continue;
        }
        let x0 = xmin.value(i);
        let y0 = ymin.value(i);
        let x1 = xmax.value(i);
        let y1 = ymax.value(i);
        if x0 < extent[0] { extent[0] = x0; }
        if y0 < extent[1] { extent[1] = y0; }
        if x1 > extent[2] { extent[2] = x1; }
        if y1 > extent[3] { extent[3] = y1; }
    }
    extent
}

// ── Hilbert curve ─────────────────────────────────────────────────────

fn compute_hilbert_indices(
    xmin: &Float64Array,
    ymin: &Float64Array,
    xmax: &Float64Array,
    ymax: &Float64Array,
    extent: &[f64; 4],
) -> UInt64Array {
    let dx = extent[2] - extent[0];
    let dy = extent[3] - extent[1];
    let n = HILBERT_ORDER;
    let max_val = (n - 1) as f64;

    let mut builder = UInt64Builder::with_capacity(xmin.len());
    for i in 0..xmin.len() {
        if xmin.is_null(i) {
            builder.append_value(0);
            continue;
        }
        // Use bbox centroid for Hilbert index
        let cx = (xmin.value(i) + xmax.value(i)) / 2.0;
        let cy = (ymin.value(i) + ymax.value(i)) / 2.0;

        let nx = if dx > 0.0 {
            (((cx - extent[0]) / dx) * max_val).round() as u32
        } else {
            0
        };
        let ny = if dy > 0.0 {
            (((cy - extent[1]) / dy) * max_val).round() as u32
        } else {
            0
        };

        builder.append_value(hilbert_xy2d(n, nx.min(n - 1), ny.min(n - 1)) as u64);
    }
    builder.finish()
}

/// Convert (x, y) coordinates to Hilbert curve distance.
///
/// Standard algorithm from Wikipedia / "Hilbert curve" article.
/// `n` must be a power of 2.
fn hilbert_xy2d(n: u32, mut x: u32, mut y: u32) -> u32 {
    let mut d = 0u32;
    let mut s = n / 2;
    while s > 0 {
        let rx = if (x & s) > 0 { 1u32 } else { 0 };
        let ry = if (y & s) > 0 { 1u32 } else { 0 };
        d += s * s * ((3 * rx) ^ ry);
        // Rotate quadrant
        if ry == 0 {
            if rx == 1 {
                x = s.wrapping_mul(2).wrapping_sub(1).wrapping_sub(x);
                y = s.wrapping_mul(2).wrapping_sub(1).wrapping_sub(y);
            }
            std::mem::swap(&mut x, &mut y);
        }
        s /= 2;
    }
    d
}

// ── GeoJSON geometry → WKB encoder ────────────────────────────────────

/// Encode a GeoJSON geometry object to WKB binary (little-endian).
/// Inverse of `wkb_to_geojson()` in geoparquet.rs.
pub fn geojson_geometry_to_wkb(geometry: &serde_json::Value) -> Option<Vec<u8>> {
    let geom_type = geometry.get("type")?.as_str()?;
    let coords = geometry.get("coordinates")?;
    let mut buf = Vec::new();
    match geom_type {
        "Point" => {
            buf.push(1u8); // LE
            buf.extend(&1u32.to_le_bytes());
            write_wkb_point(&mut buf, coords)?;
        }
        "LineString" => {
            buf.push(1u8);
            buf.extend(&2u32.to_le_bytes());
            write_wkb_linestring(&mut buf, coords)?;
        }
        "Polygon" => {
            buf.push(1u8);
            buf.extend(&3u32.to_le_bytes());
            write_wkb_polygon(&mut buf, coords)?;
        }
        "MultiPoint" => {
            let points = coords.as_array()?;
            buf.push(1u8);
            buf.extend(&4u32.to_le_bytes());
            buf.extend(&(points.len() as u32).to_le_bytes());
            for pt in points {
                buf.push(1u8);
                buf.extend(&1u32.to_le_bytes());
                write_wkb_point(&mut buf, pt)?;
            }
        }
        "MultiLineString" => {
            let lines = coords.as_array()?;
            buf.push(1u8);
            buf.extend(&5u32.to_le_bytes());
            buf.extend(&(lines.len() as u32).to_le_bytes());
            for line in lines {
                buf.push(1u8);
                buf.extend(&2u32.to_le_bytes());
                write_wkb_linestring(&mut buf, line)?;
            }
        }
        "MultiPolygon" => {
            let polys = coords.as_array()?;
            buf.push(1u8);
            buf.extend(&6u32.to_le_bytes());
            buf.extend(&(polys.len() as u32).to_le_bytes());
            for poly in polys {
                buf.push(1u8);
                buf.extend(&3u32.to_le_bytes());
                write_wkb_polygon(&mut buf, poly)?;
            }
        }
        _ => return None,
    }
    Some(buf)
}

fn write_wkb_point(buf: &mut Vec<u8>, coords: &serde_json::Value) -> Option<()> {
    let arr = coords.as_array()?;
    if arr.len() < 2 { return None; }
    let x = arr[0].as_f64()?;
    let y = arr[1].as_f64()?;
    buf.extend(&x.to_le_bytes());
    buf.extend(&y.to_le_bytes());
    Some(())
}

fn write_wkb_linestring(buf: &mut Vec<u8>, coords: &serde_json::Value) -> Option<()> {
    let points = coords.as_array()?;
    buf.extend(&(points.len() as u32).to_le_bytes());
    for pt in points {
        write_wkb_point(buf, pt)?;
    }
    Some(())
}

fn write_wkb_polygon(buf: &mut Vec<u8>, coords: &serde_json::Value) -> Option<()> {
    let rings = coords.as_array()?;
    buf.extend(&(rings.len() as u32).to_le_bytes());
    for ring in rings {
        write_wkb_linestring(buf, ring)?;
    }
    Some(())
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
        schema.column_with_name(name)
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
/// The output parquet has property columns + a `geometry` Binary column
/// containing WKB. It is NOT spatially optimized — call `optimize_geoparquet`
/// on the output for bbox columns, Hilbert sort, and small row groups.
pub fn convert_geojson_to_geoparquet(source: &Path, dest: &Path) -> Result<ConvertReport, String> {
    use std::collections::{HashMap, HashSet};

    // 1. Collect all features
    let mut features: Vec<serde_json::Value> = Vec::new();
    crate::mapserver::infra::geojson_stream::stream_feature_collection_from_path(
        source,
        |feature| {
            features.push(feature);
            Ok(())
        },
    )?;

    if features.is_empty() {
        return Err("GeoJSON has no features".to_string());
    }

    // 2. Infer property schema: scan all features for property keys and types
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum ColKind { Bool, Int, Float, Str }

    let mut col_kinds: HashMap<String, ColKind> = HashMap::new();
    let mut col_order: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for feature in &features {
        let Some(props) = feature.get("properties").and_then(|v| v.as_object()) else {
            continue;
        };
        for (key, val) in props {
            if !seen.contains(key) {
                col_order.push(key.clone());
                seen.insert(key.clone());
            }
            if val.is_null() {
                continue;
            }
            let val_kind = if val.is_boolean() {
                ColKind::Bool
            } else if val.is_i64() || val.is_u64() {
                ColKind::Int
            } else if val.is_f64() {
                ColKind::Float
            } else {
                ColKind::Str
            };
            let entry = col_kinds.entry(key.clone()).or_insert(val_kind);
            // Promote: Int + Float → Float, anything + Str → Str
            if *entry != val_kind {
                if (*entry == ColKind::Int && val_kind == ColKind::Float)
                    || (*entry == ColKind::Float && val_kind == ColKind::Int)
                {
                    *entry = ColKind::Float;
                } else if *entry != ColKind::Str && val_kind != ColKind::Str {
                    *entry = ColKind::Str;
                } else {
                    *entry = ColKind::Str;
                }
            }
        }
    }

    // 3. Build Arrow schema
    let mut fields: Vec<Arc<Field>> = Vec::new();
    for key in &col_order {
        let dt = match col_kinds.get(key).unwrap_or(&ColKind::Str) {
            ColKind::Bool => DataType::Boolean,
            ColKind::Int => DataType::Int64,
            ColKind::Float => DataType::Float64,
            ColKind::Str => DataType::Utf8,
        };
        fields.push(Arc::new(Field::new(key, dt, true)));
    }
    fields.push(Arc::new(Field::new("geometry", DataType::Binary, true)));
    let schema = Arc::new(Schema::new(fields));
    let column_count = schema.fields().len();

    // 4. Build Arrow arrays
    let num = features.len();

    // Property column builders
    enum Builder {
        Bool(BooleanBuilder),
        Int(Int64Builder),
        Float(Float64Builder),
        Str(StringBuilder),
    }

    let mut builders: Vec<Builder> = col_order.iter().map(|key| {
        match col_kinds.get(key).unwrap_or(&ColKind::Str) {
            ColKind::Bool => Builder::Bool(BooleanBuilder::with_capacity(num)),
            ColKind::Int => Builder::Int(Int64Builder::with_capacity(num)),
            ColKind::Float => Builder::Float(Float64Builder::with_capacity(num)),
            ColKind::Str => Builder::Str(StringBuilder::with_capacity(num, num * 16)),
        }
    }).collect();

    let mut geom_builder = BinaryBuilder::with_capacity(num, num * 64);

    for feature in &features {
        let props = feature.get("properties").and_then(|v| v.as_object());

        // Property columns
        for (i, key) in col_order.iter().enumerate() {
            let val = props.and_then(|p| p.get(key));
            match &mut builders[i] {
                Builder::Bool(b) => match val {
                    Some(v) if v.is_boolean() => b.append_value(v.as_bool().unwrap()),
                    Some(v) if !v.is_null() => {
                        // Try coerce string "true"/"false"
                        if let Some(s) = v.as_str() {
                            b.append_value(s.eq_ignore_ascii_case("true"));
                        } else {
                            b.append_null();
                        }
                    }
                    _ => b.append_null(),
                },
                Builder::Int(b) => match val {
                    Some(v) if v.is_i64() => b.append_value(v.as_i64().unwrap()),
                    Some(v) if v.is_u64() => b.append_value(v.as_u64().unwrap() as i64),
                    Some(v) if v.is_f64() => b.append_value(v.as_f64().unwrap() as i64),
                    _ => b.append_null(),
                },
                Builder::Float(b) => match val {
                    Some(v) if v.is_number() => b.append_value(v.as_f64().unwrap()),
                    _ => b.append_null(),
                },
                Builder::Str(b) => match val {
                    Some(v) if v.is_string() => b.append_value(v.as_str().unwrap()),
                    Some(v) if !v.is_null() => b.append_value(v.to_string()),
                    _ => b.append_null(),
                },
            }
        }

        // Geometry column
        let geom = feature.get("geometry");
        match geom.and_then(|g| if g.is_null() { None } else { geojson_geometry_to_wkb(g) }) {
            Some(wkb) => geom_builder.append_value(&wkb),
            None => geom_builder.append_null(),
        }
    }

    // Finish arrays
    let mut arrays: Vec<ArrayRef> = builders.into_iter().map(|b| -> ArrayRef {
        match b {
            Builder::Bool(mut b) => Arc::new(b.finish()),
            Builder::Int(mut b) => Arc::new(b.finish()),
            Builder::Float(mut b) => Arc::new(b.finish()),
            Builder::Str(mut b) => Arc::new(b.finish()),
        }
    }).collect();
    arrays.push(Arc::new(geom_builder.finish()));

    let batch = RecordBatch::try_new(schema.clone(), arrays)
        .map_err(|e| format!("failed to create record batch: {e}"))?;

    // 5. Write to Parquet
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create output directory: {e}"))?;
    }
    let out_file = std::fs::File::create(dest)
        .map_err(|e| format!("failed to create output file: {e}"))?;
    let props = WriterProperties::builder()
        .set_compression(Compression::ZSTD(ZstdLevel::try_new(3).unwrap()))
        .build();
    let mut writer = ArrowWriter::try_new(out_file, schema, Some(props))
        .map_err(|e| format!("failed to create parquet writer: {e}"))?;
    writer.write(&batch).map_err(|e| format!("write failed: {e}"))?;
    writer.close().map_err(|e| format!("close failed: {e}"))?;

    let dest_bytes = std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0);

    Ok(ConvertReport {
        feature_count: num,
        column_count,
        dest_bytes,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wkb_bbox_point() {
        let mut wkb = vec![1u8]; // LE
        wkb.extend(&1u32.to_le_bytes()); // Point
        wkb.extend(&144.9f64.to_le_bytes());
        wkb.extend(&(-37.8f64).to_le_bytes());
        let bbox = wkb_bbox(&wkb).unwrap();
        assert!((bbox[0] - 144.9).abs() < 1e-10);
        assert!((bbox[1] - (-37.8)).abs() < 1e-10);
        assert!((bbox[2] - 144.9).abs() < 1e-10);
        assert!((bbox[3] - (-37.8)).abs() < 1e-10);
    }

    #[test]
    fn wkb_bbox_linestring() {
        let mut wkb = vec![1u8]; // LE
        wkb.extend(&2u32.to_le_bytes()); // LineString
        wkb.extend(&3u32.to_le_bytes()); // 3 points
        for (x, y) in [(1.0, 2.0), (3.0, 4.0), (5.0, 1.0)] {
            wkb.extend(&(x as f64).to_le_bytes());
            wkb.extend(&(y as f64).to_le_bytes());
        }
        let bbox = wkb_bbox(&wkb).unwrap();
        assert!((bbox[0] - 1.0).abs() < 1e-10); // xmin
        assert!((bbox[1] - 1.0).abs() < 1e-10); // ymin
        assert!((bbox[2] - 5.0).abs() < 1e-10); // xmax
        assert!((bbox[3] - 4.0).abs() < 1e-10); // ymax
    }

    #[test]
    fn wkb_bbox_polygon() {
        let mut wkb = vec![1u8]; // LE
        wkb.extend(&3u32.to_le_bytes()); // Polygon
        wkb.extend(&1u32.to_le_bytes()); // 1 ring
        wkb.extend(&5u32.to_le_bytes()); // 5 points (closed)
        for (x, y) in [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0), (0.0, 0.0)] {
            wkb.extend(&(x as f64).to_le_bytes());
            wkb.extend(&(y as f64).to_le_bytes());
        }
        let bbox = wkb_bbox(&wkb).unwrap();
        assert!((bbox[0]).abs() < 1e-10);
        assert!((bbox[1]).abs() < 1e-10);
        assert!((bbox[2] - 10.0).abs() < 1e-10);
        assert!((bbox[3] - 10.0).abs() < 1e-10);
    }

    #[test]
    fn wkb_bbox_multipolygon() {
        // MultiPolygon with 2 polygons
        let mut wkb = vec![1u8]; // LE
        wkb.extend(&6u32.to_le_bytes()); // MultiPolygon
        wkb.extend(&2u32.to_le_bytes()); // 2 polygons

        // Polygon 1: (0,0)-(5,5)
        wkb.push(1u8);
        wkb.extend(&3u32.to_le_bytes());
        wkb.extend(&1u32.to_le_bytes()); // 1 ring
        wkb.extend(&5u32.to_le_bytes()); // 5 points
        for (x, y) in [(0.0, 0.0), (5.0, 0.0), (5.0, 5.0), (0.0, 5.0), (0.0, 0.0)] {
            wkb.extend(&(x as f64).to_le_bytes());
            wkb.extend(&(y as f64).to_le_bytes());
        }

        // Polygon 2: (10,10)-(20,20)
        wkb.push(1u8);
        wkb.extend(&3u32.to_le_bytes());
        wkb.extend(&1u32.to_le_bytes());
        wkb.extend(&5u32.to_le_bytes());
        for (x, y) in [(10.0, 10.0), (20.0, 10.0), (20.0, 20.0), (10.0, 20.0), (10.0, 10.0)] {
            wkb.extend(&(x as f64).to_le_bytes());
            wkb.extend(&(y as f64).to_le_bytes());
        }

        let bbox = wkb_bbox(&wkb).unwrap();
        assert!((bbox[0]).abs() < 1e-10);     // xmin = 0
        assert!((bbox[1]).abs() < 1e-10);     // ymin = 0
        assert!((bbox[2] - 20.0).abs() < 1e-10); // xmax = 20
        assert!((bbox[3] - 20.0).abs() < 1e-10); // ymax = 20
    }

    #[test]
    fn hilbert_basic_ordering() {
        // Points along a line should produce monotonic Hilbert indices
        let d0 = hilbert_xy2d(16, 0, 0);
        let d1 = hilbert_xy2d(16, 1, 0);
        let d_far = hilbert_xy2d(16, 15, 15);
        // d0 and d1 should be close, d_far should be different
        assert!(d0 != d_far);
        assert!(d1 != d_far);
        // Total range should be 0..256 for 16x16
        assert!(d0 < 256);
        assert!(d_far < 256);
    }

    #[test]
    fn hilbert_spatial_locality() {
        // Adjacent cells should have closer Hilbert indices than distant cells
        let n = 256u32;
        let d_near = hilbert_xy2d(n, 100, 100);
        let d_adjacent = hilbert_xy2d(n, 101, 100);
        let d_far = hilbert_xy2d(n, 200, 200);

        let dist_near = (d_near as i64 - d_adjacent as i64).unsigned_abs();
        let dist_far = (d_near as i64 - d_far as i64).unsigned_abs();
        // Adjacent cells should generally be closer on the curve than distant cells
        assert!(dist_near < dist_far);
    }

    #[test]
    fn wkb_bbox_empty_returns_none() {
        assert!(wkb_bbox(&[]).is_none());
        assert!(wkb_bbox(&[0, 0, 0]).is_none());
    }

    // ── GeoJSON → WKB round-trip tests ────────────────────────────────

    #[test]
    fn geojson_to_wkb_point_roundtrip() {
        let geojson = serde_json::json!({"type": "Point", "coordinates": [144.9, -37.8]});
        let wkb = geojson_geometry_to_wkb(&geojson).unwrap();
        let bbox = wkb_bbox(&wkb).unwrap();
        assert!((bbox[0] - 144.9).abs() < 1e-10);
        assert!((bbox[1] - (-37.8)).abs() < 1e-10);
    }

    #[test]
    fn geojson_to_wkb_linestring_roundtrip() {
        let geojson = serde_json::json!({
            "type": "LineString",
            "coordinates": [[1.0, 2.0], [3.0, 4.0], [5.0, 1.0]]
        });
        let wkb = geojson_geometry_to_wkb(&geojson).unwrap();
        let bbox = wkb_bbox(&wkb).unwrap();
        assert!((bbox[0] - 1.0).abs() < 1e-10);
        assert!((bbox[1] - 1.0).abs() < 1e-10);
        assert!((bbox[2] - 5.0).abs() < 1e-10);
        assert!((bbox[3] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn geojson_to_wkb_polygon_roundtrip() {
        let geojson = serde_json::json!({
            "type": "Polygon",
            "coordinates": [[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0], [0.0, 0.0]]]
        });
        let wkb = geojson_geometry_to_wkb(&geojson).unwrap();
        let bbox = wkb_bbox(&wkb).unwrap();
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
        let bbox = wkb_bbox(&wkb).unwrap();
        assert!((bbox[0]).abs() < 1e-10);
        assert!((bbox[1]).abs() < 1e-10);
        assert!((bbox[2] - 20.0).abs() < 1e-10);
        assert!((bbox[3] - 20.0).abs() < 1e-10);
    }

    #[test]
    fn geojson_to_wkb_multilinestring_roundtrip() {
        let geojson = serde_json::json!({
            "type": "MultiLineString",
            "coordinates": [
                [[1.0, 1.0], [2.0, 2.0]],
                [[3.0, 3.0], [4.0, 4.0]]
            ]
        });
        let wkb = geojson_geometry_to_wkb(&geojson).unwrap();
        let bbox = wkb_bbox(&wkb).unwrap();
        assert!((bbox[0] - 1.0).abs() < 1e-10);
        assert!((bbox[1] - 1.0).abs() < 1e-10);
        assert!((bbox[2] - 4.0).abs() < 1e-10);
        assert!((bbox[3] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn geojson_to_wkb_multipoint_roundtrip() {
        let geojson = serde_json::json!({
            "type": "MultiPoint",
            "coordinates": [[1.0, 2.0], [3.0, 4.0]]
        });
        let wkb = geojson_geometry_to_wkb(&geojson).unwrap();
        let bbox = wkb_bbox(&wkb).unwrap();
        assert!((bbox[0] - 1.0).abs() < 1e-10);
        assert!((bbox[1] - 2.0).abs() < 1e-10);
        assert!((bbox[2] - 3.0).abs() < 1e-10);
        assert!((bbox[3] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn geojson_to_wkb_null_returns_none() {
        assert!(geojson_geometry_to_wkb(&serde_json::Value::Null).is_none());
    }

    #[test]
    fn geojson_to_wkb_unsupported_type_returns_none() {
        let geojson = serde_json::json!({"type": "GeometryCollection", "geometries": []});
        assert!(geojson_geometry_to_wkb(&geojson).is_none());
    }

    // ── convert_geojson_to_geoparquet test ─────────────────────────────

    #[test]
    fn convert_geojson_to_geoparquet_basic() {
        let tmp = tempfile::tempdir().expect("tmp");
        let geojson_path = tmp.path().join("test.geojson");
        let parquet_path = tmp.path().join("test.parquet");

        std::fs::write(&geojson_path, r#"{
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
        }"#).expect("write geojson");

        let report = convert_geojson_to_geoparquet(&geojson_path, &parquet_path).unwrap();
        assert_eq!(report.feature_count, 2);
        assert!(report.dest_bytes > 0);
        assert!(parquet_path.exists());

        // Verify it's a valid parquet with a geometry column
        let file = std::fs::File::open(&parquet_path).unwrap();
        let builder = ParquetRecordBatchReaderBuilder::try_new(file).unwrap();
        let schema = builder.schema();
        assert!(schema.column_with_name("geometry").is_some());
        assert!(schema.column_with_name("name").is_some());
        assert!(schema.column_with_name("value").is_some());
    }

    // ── is_already_optimized test ──────────────────────────────────────

    #[test]
    fn is_already_optimized_false_for_raw_parquet() {
        let tmp = tempfile::tempdir().expect("tmp");
        let geojson_path = tmp.path().join("test.geojson");
        let parquet_path = tmp.path().join("test.parquet");

        std::fs::write(&geojson_path, r#"{
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": {"name": "A"},
                "geometry": {"type": "Point", "coordinates": [144.9, -37.8]}
            }]
        }"#).expect("write geojson");

        convert_geojson_to_geoparquet(&geojson_path, &parquet_path).unwrap();
        assert!(!is_already_optimized(&parquet_path));
    }

    #[test]
    fn is_already_optimized_true_after_optimize() {
        let tmp = tempfile::tempdir().expect("tmp");
        let geojson_path = tmp.path().join("test.geojson");
        let raw_path = tmp.path().join("test.parquet");
        let opt_path = tmp.path().join("test.spatial.parquet");

        std::fs::write(&geojson_path, r#"{
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": {"name": "A"},
                "geometry": {"type": "Point", "coordinates": [144.9, -37.8]}
            }]
        }"#).expect("write geojson");

        convert_geojson_to_geoparquet(&geojson_path, &raw_path).unwrap();
        optimize_geoparquet(&raw_path, &opt_path).unwrap();
        assert!(is_already_optimized(&opt_path));
    }
}

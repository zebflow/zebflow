//! Direct GeoParquet → PNG tile renderer.
//!
//! Bypasses DataFusion entirely for tile rendering. Reads parquet directly
//! with `ProjectionMask` (geometry + bbox columns only), streams WKB bytes
//! straight to tiny-skia PathBuilder with zero intermediate JSON allocation.
//!
//! For a z14 tile with ~100 features: ~20ms vs ~110ms via DataFusion.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use datafusion::arrow::array::*;
use datafusion::arrow::datatypes::{DataType, SchemaRef};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ProjectionMask;
use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform,
};

use super::style::LayerStyle;

// ── Cached file metadata ──────────────────────────────────────────────

struct CachedParquetMeta {
    schema: SchemaRef,
    geom_col_idx: usize,
    /// Indices of [xmin, ymin, xmax, ymax] bbox columns (if optimized).
    bbox_col_indices: Option<[usize; 4]>,
    file_mtime: u64,
    file_size: u64,
}

fn meta_cache() -> &'static Mutex<HashMap<String, CachedParquetMeta>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CachedParquetMeta>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn file_version(path: &Path) -> (u64, u64) {
    let Ok(meta) = std::fs::metadata(path) else {
        return (0, 0);
    };
    let mtime = meta
        .modified()
        .ok()
        .and_then(|ts| ts.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|v| v.as_secs())
        .unwrap_or(0);
    (mtime, meta.len())
}

const GEOM_NAMES: &[&str] = &["geometry", "geom", "wkb_geometry", "the_geom", "shape"];

fn detect_geom_col(schema: &SchemaRef) -> Option<usize> {
    // Check by name first
    for name in GEOM_NAMES {
        if let Some((idx, _)) = schema.column_with_name(name) {
            return Some(idx);
        }
    }
    // Fallback: first Binary/LargeBinary column
    for (idx, field) in schema.fields().iter().enumerate() {
        if matches!(field.data_type(), DataType::Binary | DataType::LargeBinary) {
            return Some(idx);
        }
    }
    None
}

fn detect_bbox_cols(schema: &SchemaRef) -> Option<[usize; 4]> {
    let names = [
        ["xmin", "ymin", "xmax", "ymax"],
        ["min_x", "min_y", "max_x", "max_y"],
    ];
    for group in &names {
        let indices: Vec<_> = group
            .iter()
            .filter_map(|name| {
                let (idx, field) = schema.column_with_name(name)?;
                if matches!(field.data_type(), DataType::Float64) {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();
        if indices.len() == 4 {
            return Some([indices[0], indices[1], indices[2], indices[3]]);
        }
    }
    None
}

fn get_or_create_meta(source_path: &Path) -> Result<(SchemaRef, usize, Option<[usize; 4]>), String> {
    let key = source_path.to_string_lossy().to_string();
    let (mtime, size) = file_version(source_path);

    {
        let cache = meta_cache().lock().map_err(|e| format!("meta cache lock: {e}"))?;
        if let Some(entry) = cache.get(&key) {
            if entry.file_mtime == mtime && entry.file_size == size {
                return Ok((entry.schema.clone(), entry.geom_col_idx, entry.bbox_col_indices));
            }
        }
    }

    // Build metadata from file
    let file = std::fs::File::open(source_path)
        .map_err(|e| format!("failed to open parquet: {e}"))?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| format!("failed to read parquet metadata: {e}"))?;
    let schema = builder.schema().clone();

    let geom_idx = detect_geom_col(&schema)
        .ok_or("no geometry column found in parquet")?;
    let bbox_indices = detect_bbox_cols(&schema);

    let mut cache = meta_cache().lock().map_err(|e| format!("meta cache lock: {e}"))?;
    cache.insert(
        key,
        CachedParquetMeta {
            schema: schema.clone(),
            geom_col_idx: geom_idx,
            bbox_col_indices: bbox_indices,
            file_mtime: mtime,
            file_size: size,
        },
    );

    Ok((schema, geom_idx, bbox_indices))
}

// ── Main entry point ──────────────────────────────────────────────────

/// Render a tile directly from GeoParquet — no DataFusion, no JSON.
///
/// Returns PNG bytes. Reads only geometry + bbox columns using ProjectionMask.
/// Streams WKB bytes directly to tiny-skia PathBuilder.
pub fn render_tile_direct(
    source_path: &Path,
    bbox: [f64; 4],
    width: u32,
    height: u32,
    zoom: Option<u8>,
    style: &LayerStyle,
    resolved: Option<&super::style_dsl::ResolvedStyle>,
    filter: Option<&super::filter_dsl::FilterExpr>,
    limit: usize,
) -> Result<Vec<u8>, String> {
    let (schema, geom_idx, bbox_indices) = get_or_create_meta(source_path)?;

    // Require bbox columns for correct spatial filtering.
    // Without them we can't filter rows by tile extent — fall back to DataFusion.
    if bbox_indices.is_none() {
        return Err("no bbox columns — falling back to DataFusion".into());
    }

    // Open file and build reader with projection mask (geometry + bbox + optional style field)
    let file = std::fs::File::open(source_path)
        .map_err(|e| format!("failed to open parquet: {e}"))?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| format!("failed to read parquet: {e}"))?;

    // Detect style field parquet leaf index for projection mask
    let style_field_pq_idx = resolved
        .and_then(|r| r.field_name())
        .and_then(|fname| {
            let ps = builder.parquet_schema();
            (0..ps.num_columns()).find(|&i| ps.column(i).name() == fname)
        });

    // Detect filter field parquet leaf indices for projection mask
    let filter_field_pq_indices: Vec<usize> = filter
        .map(|f| {
            super::filter_dsl::filter_field_names(f)
                .iter()
                .filter_map(|fname| {
                    let ps = builder.parquet_schema();
                    (0..ps.num_columns()).find(|&i| ps.column(i).name() == *fname)
                })
                .collect()
        })
        .unwrap_or_default();

    // Row group pruning: skip row groups whose bbox stats don't intersect the tile
    let selected_row_groups = prune_row_groups(&builder, bbox_indices, &bbox);

    let projection = build_projection_mask(builder.parquet_schema(), &schema, geom_idx, bbox_indices, style_field_pq_idx, &filter_field_pq_indices);
    let mut b = builder.with_projection(projection).with_batch_size(8192);
    if let Some(ref rgs) = selected_row_groups {
        b = b.with_row_groups(rgs.clone());
    }
    let reader = b.build().map_err(|e| format!("failed to build reader: {e}"))?;

    render_batches(reader, &schema, geom_idx, bbox_indices, &bbox, width, height, zoom, style, resolved, filter, limit)
}

/// Prune row groups using column statistics on bbox columns.
/// Returns None if pruning is not possible (no bbox columns or no stats).
fn prune_row_groups(
    builder: &ParquetRecordBatchReaderBuilder<std::fs::File>,
    bbox_indices: Option<[usize; 4]>,
    tile_bbox: &[f64; 4],
) -> Option<Vec<usize>> {
    let _ = bbox_indices?; // Need bbox columns to exist
    let metadata = builder.metadata();
    let num_rg = metadata.num_row_groups();

    // Find parquet leaf column indices for xmin/ymin/xmax/ymax by path.
    // Arrow indices don't match parquet leaf indices when struct columns exist.
    let parquet_schema = builder.parquet_schema();
    let leaf_names = ["xmin", "ymin", "xmax", "ymax"];
    let mut parquet_col_indices = [None; 4];
    for i in 0..parquet_schema.num_columns() {
        let col_desc = parquet_schema.column(i);
        let col_name = col_desc.name();
        for (j, name) in leaf_names.iter().enumerate() {
            if col_name == *name {
                parquet_col_indices[j] = Some(i);
            }
        }
    }

    let pi = [
        parquet_col_indices[0]?,
        parquet_col_indices[1]?,
        parquet_col_indices[2]?,
        parquet_col_indices[3]?,
    ];

    let mut selected = Vec::new();

    for rg_idx in 0..num_rg {
        let rg_meta = metadata.row_group(rg_idx);

        let xmin_stats = get_f64_col_stats(rg_meta, pi[0]);
        let ymin_stats = get_f64_col_stats(rg_meta, pi[1]);
        let xmax_stats = get_f64_col_stats(rg_meta, pi[2]);
        let ymax_stats = get_f64_col_stats(rg_meta, pi[3]);

        let pruned = match (xmin_stats, ymin_stats, xmax_stats, ymax_stats) {
            (Some((xmin_min, _)), Some((ymin_min, _)), Some((_, xmax_max)), Some((_, ymax_max))) => {
                // Row group definitely doesn't intersect if:
                xmin_min > tile_bbox[2]  // all features start right of tile
                    || xmax_max < tile_bbox[0]  // all features end left of tile
                    || ymin_min > tile_bbox[3]  // all features start above tile
                    || ymax_max < tile_bbox[1]  // all features end below tile
            }
            _ => false, // No stats — can't prune, include it
        };

        if !pruned {
            selected.push(rg_idx);
        }
    }

    Some(selected)
}

/// Extract (min, max) f64 statistics from a row group column.
fn get_f64_col_stats(
    rg_meta: &parquet::file::metadata::RowGroupMetaData,
    col_idx: usize,
) -> Option<(f64, f64)> {
    if col_idx >= rg_meta.num_columns() {
        return None;
    }
    let col = rg_meta.column(col_idx);
    let stats = col.statistics()?;
    use parquet::file::statistics::Statistics;
    match stats {
        Statistics::Double(s) => Some((*s.min_opt()?, *s.max_opt()?)),
        _ => None,
    }
}

// ── Data resolution for style DSL ─────────────────────────────────────

/// Get min/max statistics for a named column from parquet row group metadata.
/// Zero data scan — reads parquet footer metadata only.
pub fn get_field_stats(
    source_path: &Path,
    field_name: &str,
) -> Option<super::style_dsl::FieldStats> {
    let file = std::fs::File::open(source_path).ok()?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file).ok()?;
    let metadata = builder.metadata().clone();
    let parquet_schema = builder.parquet_schema();

    // Find the parquet leaf column index
    let col_idx = (0..parquet_schema.num_columns())
        .find(|&i| parquet_schema.column(i).name() == field_name)?;

    let mut global_min = f64::MAX;
    let mut global_max = f64::MIN;
    let mut found_any = false;

    for rg_idx in 0..metadata.num_row_groups() {
        if let Some((min, max)) = get_f64_col_stats(metadata.row_group(rg_idx), col_idx) {
            global_min = global_min.min(min);
            global_max = global_max.max(max);
            found_any = true;
        }
        // Also try i64 stats and convert
        if !found_any {
            if col_idx < metadata.row_group(rg_idx).num_columns() {
                let col = metadata.row_group(rg_idx).column(col_idx);
                if let Some(stats) = col.statistics() {
                    use parquet::file::statistics::Statistics;
                    match stats {
                        Statistics::Int64(s) => {
                            if let (Some(&mn), Some(&mx)) = (s.min_opt(), s.max_opt()) {
                                global_min = global_min.min(mn as f64);
                                global_max = global_max.max(mx as f64);
                                found_any = true;
                            }
                        }
                        Statistics::Int32(s) => {
                            if let (Some(&mn), Some(&mx)) = (s.min_opt(), s.max_opt()) {
                                global_min = global_min.min(mn as f64);
                                global_max = global_max.max(mx as f64);
                                found_any = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    if found_any && global_min <= global_max {
        Some(super::style_dsl::FieldStats {
            min: global_min,
            max: global_max,
        })
    } else {
        None
    }
}

/// Collect distinct string values for a named column.
/// Samples at most 4 row groups to keep it fast.
pub fn get_field_distinct(
    source_path: &Path,
    field_name: &str,
    max_values: usize,
) -> Option<super::style_dsl::FieldDistinct> {
    let file = std::fs::File::open(source_path).ok()?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file).ok()?;
    let schema = builder.schema().clone();

    let (_col_idx, _field) = schema.column_with_name(field_name)?;

    // Find the parquet leaf index for this column
    let parquet_schema = builder.parquet_schema();
    let pq_col_idx = (0..parquet_schema.num_columns())
        .find(|&i| parquet_schema.column(i).name() == field_name)?;

    let projection = ProjectionMask::leaves(parquet_schema, vec![pq_col_idx]);

    let max_rgs = 4;
    let num_rgs = builder.metadata().num_row_groups();
    let selected_rgs: Vec<usize> = (0..num_rgs.min(max_rgs)).collect();

    let reader = builder
        .with_projection(projection)
        .with_row_groups(selected_rgs)
        .with_batch_size(4096)
        .build()
        .ok()?;

    let mut distinct = std::collections::HashSet::new();
    for batch_result in reader {
        let batch = batch_result.ok()?;
        if batch.num_columns() == 0 {
            continue;
        }
        let col = batch.column(0);
        if let Some(arr) = col.as_any().downcast_ref::<StringArray>() {
            for i in 0..arr.len() {
                if !arr.is_null(i) {
                    distinct.insert(arr.value(i).to_string());
                }
            }
        } else if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
            for i in 0..arr.len() {
                if !arr.is_null(i) {
                    distinct.insert(arr.value(i).to_string());
                }
            }
        } else if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
            for i in 0..arr.len() {
                if !arr.is_null(i) {
                    let v = arr.value(i);
                    if v == (v as i64) as f64 {
                        distinct.insert(format!("{}", v as i64));
                    } else {
                        distinct.insert(format!("{v}"));
                    }
                }
            }
        } else if let Some(arr) = col.as_any().downcast_ref::<Int32Array>() {
            for i in 0..arr.len() {
                if !arr.is_null(i) {
                    distinct.insert(arr.value(i).to_string());
                }
            }
        }
        if distinct.len() >= max_values {
            break;
        }
    }

    let mut values: Vec<String> = distinct.into_iter().collect();
    values.sort();
    values.truncate(max_values);
    Some(super::style_dsl::FieldDistinct { values })
}

// ── Projection mask building ─────────────────────────────────────────

fn build_projection_mask(
    parquet_schema: &parquet::schema::types::SchemaDescriptor,
    _arrow_schema: &SchemaRef,
    geom_idx: usize,
    bbox_indices: Option<[usize; 4]>,
    style_field_idx: Option<usize>,
    extra_indices: &[usize],
) -> ProjectionMask {
    let mut indices = vec![geom_idx];
    if let Some(bi) = bbox_indices {
        indices.extend_from_slice(&bi);
    }
    if let Some(si) = style_field_idx {
        indices.push(si);
    }
    indices.extend_from_slice(extra_indices);
    indices.sort();
    indices.dedup();
    ProjectionMask::leaves(parquet_schema, indices)
}

fn render_batches(
    reader: parquet::arrow::arrow_reader::ParquetRecordBatchReader,
    full_schema: &SchemaRef,
    geom_idx: usize,
    bbox_indices: Option<[usize; 4]>,
    bbox: &[f64; 4],
    width: u32,
    height: u32,
    _zoom: Option<u8>,
    style: &LayerStyle,
    resolved: Option<&super::style_dsl::ResolvedStyle>,
    filter: Option<&super::filter_dsl::FilterExpr>,
    limit: usize,
) -> Result<Vec<u8>, String> {
    let w = width.min(4096).max(1);
    let h = height.min(4096).max(1);
    let mut pixmap = Pixmap::new(w, h).ok_or("failed to create pixmap")?;

    // Prepare default paints (used when not per-feature styling)
    let fill_paint = style.fill_color.map(|c| {
        let mut p = Paint::default();
        p.set_color(Color::from_rgba8(c[0], c[1], c[2], c[3]));
        p.anti_alias = true;
        p
    });

    let mut stroke_paint = Paint::default();
    stroke_paint.set_color(Color::from_rgba8(
        style.stroke_color[0],
        style.stroke_color[1],
        style.stroke_color[2],
        style.stroke_color[3],
    ));
    stroke_paint.anti_alias = true;

    let stroke_def = {
        let mut s = Stroke::default();
        s.width = style.stroke_width;
        s.line_cap = LineCap::Round;
        s.line_join = LineJoin::Round;
        s
    };

    let mut point_paint = Paint::default();
    point_paint.set_color(Color::from_rgba8(
        style.point_color[0],
        style.point_color[1],
        style.point_color[2],
        style.point_color[3],
    ));
    point_paint.anti_alias = true;

    let transform = Transform::identity();

    // Per-feature styling setup
    let use_per_feature = resolved.map_or(false, |r| !r.is_uniform());
    let style_field_name = if use_per_feature {
        resolved.and_then(|r| r.field_name())
    } else {
        None
    };

    // Map projected column indices back to batch column positions.
    // When ProjectionMask is applied, the batch only contains the projected columns
    // in their original schema order. We need to find them by name.
    let batch_geom_name = full_schema.field(geom_idx).name().clone();
    let batch_bbox_names: Option<[String; 4]> = bbox_indices.map(|bi| {
        [
            full_schema.field(bi[0]).name().clone(),
            full_schema.field(bi[1]).name().clone(),
            full_schema.field(bi[2]).name().clone(),
            full_schema.field(bi[3]).name().clone(),
        ]
    });

    let tile_dx = bbox[2] - bbox[0];
    let tile_dy = bbox[3] - bbox[1];
    // Pixel snap threshold: skip vertices closer than this in pixel space.
    // At z6 (tile ~3° wide), a 256px tile has ~85px/degree. A speed zone
    // road segment might have vertices every 10m (~0.0001°) = 0.008px apart.
    // Snapping to 0.5px eliminates >90% of vertices at overview zoom.
    let px_snap = 0.5f32;

    // Pixel saturation bitmap for low-zoom point deduplication.
    // At low zoom, most pixel positions are painted multiple times by sub-pixel
    // points. A bitmap tracks which pixels have been drawn, and once ≥95% are
    // filled we stop reading entirely — typically cutting 10M-row point layers
    // from 500K iterations to ~30K.
    let use_saturation = tile_dx > 0.01; // roughly z12 and below
    let bitmap_len = (w as usize) * (h as usize);
    let mut pixel_bitmap: Vec<u8> = if use_saturation {
        vec![0u8; bitmap_len / 8 + 1]
    } else {
        Vec::new()
    };
    let mut pixels_set: u32 = 0;
    let saturation_threshold = (bitmap_len as f64 * 0.95) as u32;
    let mut saturated = false;
    let mut total_rows_processed: u32 = 0;

    let mut count = 0usize;

    for batch_result in reader {
        let batch = batch_result.map_err(|e| format!("batch read error: {e}"))?;
        let batch_schema = batch.schema();

        // Find geometry column in projected batch
        let bg_idx = batch_schema
            .index_of(&batch_geom_name)
            .map_err(|_| "geometry column not in projected batch")?;

        // Find bbox columns in projected batch
        let bb_indices: Option<[usize; 4]> = batch_bbox_names.as_ref().and_then(|names| {
            let i0 = batch_schema.index_of(&names[0]).ok()?;
            let i1 = batch_schema.index_of(&names[1]).ok()?;
            let i2 = batch_schema.index_of(&names[2]).ok()?;
            let i3 = batch_schema.index_of(&names[3]).ok()?;
            Some([i0, i1, i2, i3])
        });

        let geom_col = batch.column(bg_idx);

        // Get bbox arrays
        let bbox_arrays: Option<[&Float64Array; 4]> = bb_indices.map(|bi| {
            [
                batch.column(bi[0]).as_any().downcast_ref::<Float64Array>().unwrap(),
                batch.column(bi[1]).as_any().downcast_ref::<Float64Array>().unwrap(),
                batch.column(bi[2]).as_any().downcast_ref::<Float64Array>().unwrap(),
                batch.column(bi[3]).as_any().downcast_ref::<Float64Array>().unwrap(),
            ]
        });

        // Find style field column in projected batch (for per-feature styling)
        let style_col: Option<&dyn datafusion::arrow::array::Array> = style_field_name
            .and_then(|fname| batch_schema.index_of(fname).ok())
            .map(|idx| batch.column(idx).as_ref());
        let batch_per_feature = use_per_feature && style_col.is_some();

        // Build filter column cache for this batch (once per batch)
        let filter_col_cache = filter.map(|f| {
            super::filter_dsl::build_filter_column_cache(f, &batch_schema)
        });

        for row in 0..batch.num_rows() {
            if count >= limit || saturated {
                break;
            }

            // Periodic saturation check (every 4096 rows)
            if use_saturation {
                total_rows_processed += 1;
                if total_rows_processed & 0xFFF == 0 && pixels_set >= saturation_threshold {
                    saturated = true;
                    break;
                }
            }

            // Fast bbox pre-filter
            if let Some(ref ba) = bbox_arrays {
                let fxmin = ba[0].value(row);
                let fymin = ba[1].value(row);
                let fxmax = ba[2].value(row);
                let fymax = ba[3].value(row);
                if !bbox_overlaps(fxmin, fymin, fxmax, fymax, bbox) {
                    continue;
                }

                // Attribute filter (check before sub-pixel skip to avoid counting filtered rows)
                if let (Some(f), Some(cc)) = (filter, &filter_col_cache) {
                    if !super::filter_dsl::matches_arrow_row(f, &batch, row, cc) {
                        continue;
                    }
                }

                // Sub-pixel skip: if feature bbox is <1px in both dimensions, render as dot
                let feat_px_w = ((fxmax - fxmin) / tile_dx * w as f64).abs();
                let feat_px_h = ((fymax - fymin) / tile_dy * h as f64).abs();
                if feat_px_w < 1.0 && feat_px_h < 1.0 {
                    let cx = ((fxmin + fxmax) * 0.5 - bbox[0]) / tile_dx * w as f64;
                    let cy = (bbox[3] - (fymin + fymax) * 0.5) / tile_dy * h as f64;

                    // Pixel saturation: skip if this pixel was already painted
                    if use_saturation {
                        let px = (cx as u32).min(w - 1);
                        let py = (cy as u32).min(h - 1);
                        let bit_idx = (py * w + px) as usize;
                        if bit_idx < bitmap_len {
                            let byte_idx = bit_idx >> 3;
                            let bit_mask = 1u8 << (bit_idx & 7);
                            if pixel_bitmap[byte_idx] & bit_mask != 0 {
                                // Already painted — skip
                                continue;
                            }
                            pixel_bitmap[byte_idx] |= bit_mask;
                            pixels_set += 1;
                        }
                    }

                    if batch_per_feature {
                        if let (Some(r), Some(col)) = (resolved, style_col) {
                            let sv = extract_style_value(col, row);
                            let fs = r.style_for_value(&sv);
                            let mut pp = Paint::default();
                            pp.set_color(Color::from_rgba8(
                                fs.point_color[0], fs.point_color[1],
                                fs.point_color[2], fs.point_color[3],
                            ));
                            pp.anti_alias = true;
                            draw_point(&mut pixmap, cx as f32, cy as f32, &pp, 1.0, transform);
                        }
                    } else {
                        draw_point(&mut pixmap, cx as f32, cy as f32, &point_paint, 1.0, transform);
                    }
                    count += 1;
                    continue;
                }
            }

            // Get WKB bytes directly — zero copy from Arrow buffer
            let wkb = get_binary_bytes(geom_col, row);
            let Some(wkb) = wkb else { continue };
            if wkb.len() < 5 {
                continue;
            }

            // Parse WKB → build path → render directly onto pixmap
            if batch_per_feature {
                if let (Some(r), Some(col)) = (resolved, style_col) {
                    let sv = extract_style_value(col, row);
                    let fs = r.style_for_value(&sv);
                    let (ff, fsp, fsd, fpp, fpr) = paints_from_feature_style(fs);
                    render_wkb_to_pixmap(
                        wkb, bbox, w, h, &mut pixmap,
                        ff.as_ref(), &fsp, &fsd, &fpp, fpr, transform, px_snap,
                    );
                }
            } else {
                render_wkb_to_pixmap(
                    wkb, bbox, w, h, &mut pixmap,
                    fill_paint.as_ref(), &stroke_paint, &stroke_def,
                    &point_paint, style.point_radius, transform, px_snap,
                );
            }

            count += 1;
        }

        if count >= limit || saturated {
            break;
        }
    }

    pixmap
        .encode_png()
        .map_err(|e| format!("png encode failed: {e}"))
}

// ── WKB → Pixmap rendering ───────────────────────────────────────────

/// Get binary bytes from an Arrow array (handles Binary and LargeBinary).
#[inline]
fn get_binary_bytes<'a>(col: &'a dyn datafusion::arrow::array::Array, idx: usize) -> Option<&'a [u8]> {
    if col.is_null(idx) {
        return None;
    }
    if let Some(arr) = col.as_any().downcast_ref::<BinaryArray>() {
        return Some(arr.value(idx));
    }
    if let Some(arr) = col.as_any().downcast_ref::<LargeBinaryArray>() {
        return Some(arr.value(idx));
    }
    None
}

#[inline(always)]
fn bbox_overlaps(xmin: f64, ymin: f64, xmax: f64, ymax: f64, tile: &[f64; 4]) -> bool {
    xmin <= tile[2] && xmax >= tile[0] && ymin <= tile[3] && ymax >= tile[1]
}

/// Convert geographic coordinate to pixel position within tile.
#[inline(always)]
fn geo_px(lon: f64, lat: f64, bbox: &[f64; 4], w: u32, h: u32) -> (f32, f32) {
    let dx = bbox[2] - bbox[0];
    let dy = bbox[3] - bbox[1];
    let px = ((lon - bbox[0]) / dx * w as f64) as f32;
    let py = ((bbox[3] - lat) / dy * h as f64) as f32; // Y flipped
    (px, py)
}

/// Render a WKB geometry directly onto a Pixmap — no intermediate allocations.
fn render_wkb_to_pixmap(
    wkb: &[u8],
    bbox: &[f64; 4],
    w: u32,
    h: u32,
    pixmap: &mut Pixmap,
    fill_paint: Option<&Paint>,
    stroke_paint: &Paint,
    stroke_def: &Stroke,
    point_paint: &Paint,
    point_radius: f32,
    transform: Transform,
    px_snap: f32,
) {
    let is_le = wkb[0] == 1;
    let geom_type = if is_le {
        u32::from_le_bytes([wkb[1], wkb[2], wkb[3], wkb[4]])
    } else {
        u32::from_be_bytes([wkb[1], wkb[2], wkb[3], wkb[4]])
    };

    let mut cursor = 5usize;

    match geom_type {
        1 => {
            // Point
            if let Some((x, y)) = read_f64_pair(wkb, cursor, is_le) {
                let (px, py) = geo_px(x, y, bbox, w, h);
                draw_point(pixmap, px, py, point_paint, point_radius, transform);
            }
        }
        2 => {
            // LineString
            let mut pb = PathBuilder::new();
            if build_linestring_path(&mut pb, wkb, &mut cursor, is_le, bbox, w, h, px_snap) {
                if let Some(path) = pb.finish() {
                    pixmap.stroke_path(&path, stroke_paint, stroke_def, transform, None);
                }
            }
        }
        3 => {
            // Polygon
            let mut pb = PathBuilder::new();
            if build_polygon_path(&mut pb, wkb, &mut cursor, is_le, bbox, w, h, px_snap) {
                if let Some(path) = pb.finish() {
                    if let Some(fp) = fill_paint {
                        pixmap.fill_path(&path, fp, FillRule::EvenOdd, transform, None);
                    }
                    if stroke_def.width > 0.0 {
                        pixmap.stroke_path(&path, stroke_paint, stroke_def, transform, None);
                    }
                }
            }
        }
        4 => {
            // MultiPoint
            if let Some(num) = wkb_read_u32(wkb, cursor, is_le) {
                cursor += 4;
                for _ in 0..num {
                    if cursor + 21 > wkb.len() { break; }
                    let inner_le = wkb[cursor] == 1;
                    cursor += 5;
                    if let Some((x, y)) = read_f64_pair(wkb, cursor, inner_le) {
                        cursor += 16;
                        let (px, py) = geo_px(x, y, bbox, w, h);
                        draw_point(pixmap, px, py, point_paint, point_radius, transform);
                    }
                }
            }
        }
        5 => {
            // MultiLineString
            if let Some(num) = wkb_read_u32(wkb, cursor, is_le) {
                cursor += 4;
                for _ in 0..num {
                    if cursor + 5 > wkb.len() { break; }
                    let inner_le = wkb[cursor] == 1;
                    cursor += 5;
                    let mut pb = PathBuilder::new();
                    if build_linestring_path(&mut pb, wkb, &mut cursor, inner_le, bbox, w, h, px_snap) {
                        if let Some(path) = pb.finish() {
                            pixmap.stroke_path(&path, stroke_paint, stroke_def, transform, None);
                        }
                    }
                }
            }
        }
        6 => {
            // MultiPolygon
            if let Some(num) = wkb_read_u32(wkb, cursor, is_le) {
                cursor += 4;
                for _ in 0..num {
                    if cursor + 5 > wkb.len() { break; }
                    let inner_le = wkb[cursor] == 1;
                    cursor += 5;
                    let mut pb = PathBuilder::new();
                    if build_polygon_path(&mut pb, wkb, &mut cursor, inner_le, bbox, w, h, px_snap) {
                        if let Some(path) = pb.finish() {
                            if let Some(fp) = fill_paint {
                                pixmap.fill_path(&path, fp, FillRule::EvenOdd, transform, None);
                            }
                            if stroke_def.width > 0.0 {
                                pixmap.stroke_path(&path, stroke_paint, stroke_def, transform, None);
                            }
                        }
                    }
                }
            }
        }
        _ => {} // Skip unknown geometry types
    }
}

// ── WKB parsing helpers ──────────────────────────────────────────────

#[inline(always)]
fn wkb_read_f64(wkb: &[u8], offset: usize, is_le: bool) -> Option<f64> {
    if offset + 8 > wkb.len() { return None; }
    let bytes: [u8; 8] = wkb[offset..offset + 8].try_into().ok()?;
    Some(if is_le { f64::from_le_bytes(bytes) } else { f64::from_be_bytes(bytes) })
}

#[inline(always)]
fn wkb_read_u32(wkb: &[u8], offset: usize, is_le: bool) -> Option<u32> {
    if offset + 4 > wkb.len() { return None; }
    let bytes: [u8; 4] = wkb[offset..offset + 4].try_into().ok()?;
    Some(if is_le { u32::from_le_bytes(bytes) } else { u32::from_be_bytes(bytes) })
}

#[inline(always)]
fn read_f64_pair(wkb: &[u8], offset: usize, is_le: bool) -> Option<(f64, f64)> {
    let x = wkb_read_f64(wkb, offset, is_le)?;
    let y = wkb_read_f64(wkb, offset + 8, is_le)?;
    Some((x, y))
}

/// Build a linestring path from WKB bytes. Advances cursor past the linestring.
/// Vertices closer than `snap` pixels to the previous emitted vertex are skipped.
fn build_linestring_path(
    pb: &mut PathBuilder,
    wkb: &[u8],
    cursor: &mut usize,
    is_le: bool,
    bbox: &[f64; 4],
    w: u32,
    h: u32,
    snap: f32,
) -> bool {
    let Some(num_points) = wkb_read_u32(wkb, *cursor, is_le) else { return false };
    *cursor += 4;
    let mut started = false;
    let mut last_px = 0.0f32;
    let mut last_py = 0.0f32;
    let snap_sq = snap * snap;
    for i in 0..num_points {
        let Some((x, y)) = read_f64_pair(wkb, *cursor, is_le) else { return started };
        *cursor += 16;
        let (px, py) = geo_px(x, y, bbox, w, h);
        if !started {
            pb.move_to(px, py);
            last_px = px;
            last_py = py;
            started = true;
        } else {
            let dx = px - last_px;
            let dy = py - last_py;
            // Skip intermediate vertices within snap threshold, but always emit last vertex
            if dx * dx + dy * dy < snap_sq && i + 1 != num_points {
                continue;
            }
            pb.line_to(px, py);
            last_px = px;
            last_py = py;
        }
    }
    started
}

/// Build a polygon path from WKB bytes. Advances cursor past the polygon.
/// Vertices closer than `snap` pixels to the previous emitted vertex are skipped.
fn build_polygon_path(
    pb: &mut PathBuilder,
    wkb: &[u8],
    cursor: &mut usize,
    is_le: bool,
    bbox: &[f64; 4],
    w: u32,
    h: u32,
    snap: f32,
) -> bool {
    let Some(num_rings) = wkb_read_u32(wkb, *cursor, is_le) else { return false };
    *cursor += 4;
    let snap_sq = snap * snap;
    let mut any = false;
    for _ in 0..num_rings {
        let Some(num_points) = wkb_read_u32(wkb, *cursor, is_le) else { return any };
        *cursor += 4;
        let mut started = false;
        let mut last_px = 0.0f32;
        let mut last_py = 0.0f32;
        for i in 0..num_points {
            let Some((x, y)) = read_f64_pair(wkb, *cursor, is_le) else { return any };
            *cursor += 16;
            let (px, py) = geo_px(x, y, bbox, w, h);
            if !started {
                pb.move_to(px, py);
                last_px = px;
                last_py = py;
                started = true;
            } else {
                let dx = px - last_px;
                let dy = py - last_py;
                if dx * dx + dy * dy < snap_sq && i + 1 != num_points {
                    continue;
                }
                pb.line_to(px, py);
                last_px = px;
                last_py = py;
            }
        }
        if started {
            pb.close();
            any = true;
        }
    }
    any
}

/// Draw a point as a filled circle.
fn draw_point(
    pixmap: &mut Pixmap,
    cx: f32,
    cy: f32,
    paint: &Paint,
    radius: f32,
    transform: Transform,
) {
    let rect = tiny_skia::Rect::from_xywh(cx - radius, cy - radius, radius * 2.0, radius * 2.0);
    let Some(rect) = rect else { return };
    let Some(path) = PathBuilder::from_oval(rect) else { return };
    pixmap.fill_path(&path, paint, FillRule::Winding, transform, None);
}

// ── Per-feature styling helpers ───────────────────────────────────────

/// Extract a StyleValue from an Arrow array column at the given row.
fn extract_style_value(
    col: &dyn datafusion::arrow::array::Array,
    row: usize,
) -> super::style_dsl::StyleValue {
    use super::style_dsl::StyleValue;
    if col.is_null(row) {
        return StyleValue::Null;
    }
    if let Some(arr) = col.as_any().downcast_ref::<StringArray>() {
        return StyleValue::Str(arr.value(row).to_string());
    }
    if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
        return StyleValue::F64(arr.value(row));
    }
    if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
        return StyleValue::I64(arr.value(row));
    }
    if let Some(arr) = col.as_any().downcast_ref::<Int32Array>() {
        return StyleValue::I64(arr.value(row) as i64);
    }
    if let Some(arr) = col.as_any().downcast_ref::<Float32Array>() {
        return StyleValue::F64(arr.value(row) as f64);
    }
    StyleValue::Null
}

/// Build Paint objects from a FeatureStyle for per-feature rendering.
fn paints_from_feature_style(
    fs: super::style_dsl::FeatureStyle,
) -> (Option<Paint<'static>>, Paint<'static>, Stroke, Paint<'static>, f32) {
    let fill = fs.fill_color.map(|c| {
        let mut p = Paint::default();
        p.set_color(Color::from_rgba8(c[0], c[1], c[2], c[3]));
        p.anti_alias = true;
        p
    });
    let mut stroke_p = Paint::default();
    stroke_p.set_color(Color::from_rgba8(
        fs.stroke_color[0], fs.stroke_color[1],
        fs.stroke_color[2], fs.stroke_color[3],
    ));
    stroke_p.anti_alias = true;
    let stroke_d = {
        let mut s = Stroke::default();
        s.width = fs.stroke_width;
        s.line_cap = LineCap::Round;
        s.line_join = LineJoin::Round;
        s
    };
    let mut point_p = Paint::default();
    point_p.set_color(Color::from_rgba8(
        fs.point_color[0], fs.point_color[1],
        fs.point_color[2], fs.point_color[3],
    ));
    point_p.anti_alias = true;
    (fill, stroke_p, stroke_d, point_p, fs.point_radius)
}

// ── Direct GeoParquet → MVT ──────────────────────────────────────────

/// Render MVT tile directly from GeoParquet — no DataFusion, no JSON.
///
/// Returns gzip-compressed MVT bytes. Reads only geometry + bbox + property
/// columns using ProjectionMask.
pub fn render_mvt_direct(
    source_path: &Path,
    bbox: [f64; 4],
    layer_name: &str,
    zoom: Option<u8>,
    filter: Option<&super::filter_dsl::FilterExpr>,
    allowed_properties: &[String],
    limit: usize,
) -> Result<Vec<u8>, String> {
    let (schema, geom_idx, bbox_indices) = get_or_create_meta(source_path)?;

    if bbox_indices.is_none() {
        return Err("no bbox columns — falling back to DataFusion".into());
    }

    let file = std::fs::File::open(source_path)
        .map_err(|e| format!("failed to open parquet: {e}"))?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| format!("failed to read parquet: {e}"))?;

    // Detect filter field parquet leaf indices
    let filter_field_pq_indices: Vec<usize> = filter
        .map(|f| {
            super::filter_dsl::filter_field_names(f)
                .iter()
                .filter_map(|fname| {
                    let ps = builder.parquet_schema();
                    (0..ps.num_columns()).find(|&i| ps.column(i).name() == *fname)
                })
                .collect()
        })
        .unwrap_or_default();

    // Find parquet leaf indices for property columns
    let prop_pq_indices: Vec<usize> = if allowed_properties.is_empty() {
        // Include all non-geometry, non-bbox columns
        let ps = builder.parquet_schema();
        let skip_names: &[&str] = &[
            "xmin", "ymin", "xmax", "ymax", "min_x", "min_y", "max_x", "max_y",
        ];
        (0..ps.num_columns())
            .filter(|&i| {
                let col_desc = ps.column(i);
                let name = col_desc.name();
                !GEOM_NAMES.contains(&name) && !skip_names.contains(&name)
            })
            .collect()
    } else {
        let ps = builder.parquet_schema();
        allowed_properties
            .iter()
            .filter_map(|pname| {
                (0..ps.num_columns()).find(|&i| ps.column(i).name() == pname.as_str())
            })
            .collect()
    };

    // Row group pruning
    let selected_row_groups = prune_row_groups(&builder, bbox_indices, &bbox);

    // Build projection mask including geometry, bbox, filter fields, and property columns
    let mut all_extra = filter_field_pq_indices.clone();
    all_extra.extend_from_slice(&prop_pq_indices);
    let projection = build_projection_mask(
        builder.parquet_schema(),
        &schema,
        geom_idx,
        bbox_indices,
        None,
        &all_extra,
    );
    let mut b = builder.with_projection(projection).with_batch_size(8192);
    if let Some(ref rgs) = selected_row_groups {
        b = b.with_row_groups(rgs.clone());
    }
    let reader = b.build().map_err(|e| format!("failed to build reader: {e}"))?;

    mvt_encode_batches(
        reader,
        &schema,
        geom_idx,
        bbox_indices,
        &bbox,
        layer_name,
        zoom,
        filter,
        allowed_properties,
        limit,
    )
}

/// Encode parquet record batches directly to gzipped MVT.
fn mvt_encode_batches(
    reader: parquet::arrow::arrow_reader::ParquetRecordBatchReader,
    full_schema: &SchemaRef,
    geom_idx: usize,
    bbox_indices: Option<[usize; 4]>,
    bbox: &[f64; 4],
    layer_name: &str,
    _zoom: Option<u8>,
    filter: Option<&super::filter_dsl::FilterExpr>,
    allowed_properties: &[String],
    limit: usize,
) -> Result<Vec<u8>, String> {
    use super::mvt::{encode_wkb_feature, MvtValue};

    let batch_geom_name = full_schema.field(geom_idx).name().clone();
    let batch_bbox_names: Option<[String; 4]> = bbox_indices.map(|bi| {
        [
            full_schema.field(bi[0]).name().clone(),
            full_schema.field(bi[1]).name().clone(),
            full_schema.field(bi[2]).name().clone(),
            full_schema.field(bi[3]).name().clone(),
        ]
    });

    let mut tags = super::mvt::TagTable::new();
    let mut cursor = (0i32, 0i32);
    let mut feature_id = 1u64;
    let mut encoded_features = Vec::new();
    let mut count = 0usize;

    for batch_result in reader {
        let batch = batch_result.map_err(|e| format!("batch read error: {e}"))?;
        let batch_schema = batch.schema();

        let bg_idx = batch_schema
            .index_of(&batch_geom_name)
            .map_err(|_| "geometry column not in projected batch")?;

        let bb_indices: Option<[usize; 4]> = batch_bbox_names.as_ref().and_then(|names| {
            let i0 = batch_schema.index_of(&names[0]).ok()?;
            let i1 = batch_schema.index_of(&names[1]).ok()?;
            let i2 = batch_schema.index_of(&names[2]).ok()?;
            let i3 = batch_schema.index_of(&names[3]).ok()?;
            Some([i0, i1, i2, i3])
        });

        let geom_col = batch.column(bg_idx);

        let bbox_arrays: Option<[&Float64Array; 4]> = bb_indices.map(|bi| {
            [
                batch.column(bi[0]).as_any().downcast_ref::<Float64Array>().unwrap(),
                batch.column(bi[1]).as_any().downcast_ref::<Float64Array>().unwrap(),
                batch.column(bi[2]).as_any().downcast_ref::<Float64Array>().unwrap(),
                batch.column(bi[3]).as_any().downcast_ref::<Float64Array>().unwrap(),
            ]
        });

        // Identify property columns in the projected batch
        let prop_cols: Vec<(String, usize)> = batch_schema
            .fields()
            .iter()
            .enumerate()
            .filter(|(idx, field)| {
                let name = field.name().as_str();
                // Skip geometry and bbox columns
                if *idx == bg_idx {
                    return false;
                }
                if let Some(ref bi) = bb_indices {
                    if bi.contains(idx) {
                        return false;
                    }
                }
                if GEOM_NAMES.contains(&name) {
                    return false;
                }
                if ["xmin", "ymin", "xmax", "ymax", "min_x", "min_y", "max_x", "max_y"]
                    .contains(&name)
                {
                    return false;
                }
                if !allowed_properties.is_empty() {
                    return allowed_properties.iter().any(|p| p == name);
                }
                true
            })
            .map(|(idx, field)| (field.name().clone(), idx))
            .collect();

        let filter_col_cache = filter.map(|f| {
            super::filter_dsl::build_filter_column_cache(f, &batch_schema)
        });

        for row in 0..batch.num_rows() {
            if count >= limit {
                break;
            }

            // Fast bbox pre-filter
            if let Some(ref ba) = bbox_arrays {
                let fxmin = ba[0].value(row);
                let fymin = ba[1].value(row);
                let fxmax = ba[2].value(row);
                let fymax = ba[3].value(row);
                if !bbox_overlaps(fxmin, fymin, fxmax, fymax, bbox) {
                    continue;
                }

                if let (Some(f), Some(cc)) = (filter, &filter_col_cache) {
                    if !super::filter_dsl::matches_arrow_row(f, &batch, row, cc) {
                        continue;
                    }
                }
            }

            let wkb = get_binary_bytes(geom_col, row);
            let Some(wkb) = wkb else { continue };
            if wkb.len() < 5 {
                continue;
            }

            // Extract properties for this row
            let props: Vec<(&str, MvtValue)> = prop_cols
                .iter()
                .filter_map(|(name, idx)| {
                    let col = batch.column(*idx);
                    arrow_value_to_mvt(col, row).map(|v| (name.as_str(), v))
                })
                .collect();

            if let Some(feat_bytes) = encode_wkb_feature(
                wkb, &props, bbox, 4096, &mut tags, &mut cursor, &mut feature_id,
            ) {
                encoded_features.push(feat_bytes);
            }

            count += 1;
        }

        if count >= limit {
            break;
        }
    }

    let raw = super::mvt::assemble_tile(layer_name, &tags, &encoded_features);
    Ok(super::mvt::gzip_compress(&raw))
}

/// Extract an MVT-compatible value from an Arrow array column at a given row.
fn arrow_value_to_mvt(
    col: &dyn datafusion::arrow::array::Array,
    row: usize,
) -> Option<super::mvt::MvtValue> {
    use super::mvt::MvtValue;
    if col.is_null(row) {
        return None;
    }
    if let Some(arr) = col.as_any().downcast_ref::<StringArray>() {
        return Some(MvtValue::Str(arr.value(row).to_string()));
    }
    if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
        return Some(MvtValue::F64(arr.value(row)));
    }
    if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
        return Some(MvtValue::I64(arr.value(row)));
    }
    if let Some(arr) = col.as_any().downcast_ref::<Int32Array>() {
        return Some(MvtValue::I64(arr.value(row) as i64));
    }
    if let Some(arr) = col.as_any().downcast_ref::<Float32Array>() {
        return Some(MvtValue::F64(arr.value(row) as f64));
    }
    if let Some(arr) = col.as_any().downcast_ref::<BooleanArray>() {
        return Some(MvtValue::Bool(arr.value(row)));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bbox_overlaps_intersecting() {
        assert!(bbox_overlaps(1.0, 1.0, 3.0, 3.0, &[2.0, 2.0, 4.0, 4.0]));
    }

    #[test]
    fn bbox_overlaps_disjoint() {
        assert!(!bbox_overlaps(1.0, 1.0, 2.0, 2.0, &[3.0, 3.0, 4.0, 4.0]));
    }

    #[test]
    fn bbox_overlaps_containing() {
        assert!(bbox_overlaps(0.0, 0.0, 10.0, 10.0, &[2.0, 2.0, 4.0, 4.0]));
    }

    #[test]
    fn geo_px_center() {
        let (px, py) = geo_px(144.5, -37.5, &[144.0, -38.0, 145.0, -37.0], 256, 256);
        assert!((px - 128.0).abs() < 1.0);
        assert!((py - 128.0).abs() < 1.0);
    }

    #[test]
    fn geo_px_corners() {
        let bbox = [144.0, -38.0, 145.0, -37.0];
        let (px, py) = geo_px(144.0, -37.0, &bbox, 256, 256); // top-left
        assert!(px.abs() < 0.1);
        assert!(py.abs() < 0.1);

        let (px, py) = geo_px(145.0, -38.0, &bbox, 256, 256); // bottom-right
        assert!((px - 256.0).abs() < 0.1);
        assert!((py - 256.0).abs() < 0.1);
    }

    #[test]
    fn render_wkb_point_onto_pixmap() {
        let mut wkb = vec![1u8]; // LE
        wkb.extend(&1u32.to_le_bytes()); // Point
        wkb.extend(&144.5f64.to_le_bytes());
        wkb.extend(&(-37.5f64).to_le_bytes());

        let mut pixmap = Pixmap::new(256, 256).unwrap();
        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba8(255, 0, 0, 255));
        let stroke = Stroke::default();
        let transform = Transform::identity();

        render_wkb_to_pixmap(
            &wkb,
            &[144.0, -38.0, 145.0, -37.0],
            256, 256,
            &mut pixmap,
            None,
            &paint,
            &stroke,
            &paint,
            4.0,
            transform,
            0.5,
        );
        // Should have drawn something (non-empty pixmap)
        assert!(pixmap.data().iter().any(|&b| b != 0));
    }

    #[test]
    fn render_wkb_polygon_onto_pixmap() {
        let mut wkb = vec![1u8]; // LE
        wkb.extend(&3u32.to_le_bytes()); // Polygon
        wkb.extend(&1u32.to_le_bytes()); // 1 ring
        wkb.extend(&5u32.to_le_bytes()); // 5 points
        for &(x, y) in &[
            (144.2, -37.8),
            (144.8, -37.8),
            (144.8, -37.2),
            (144.2, -37.2),
            (144.2, -37.8),
        ] {
            wkb.extend(&(x as f64).to_le_bytes());
            wkb.extend(&(y as f64).to_le_bytes());
        }

        let mut pixmap = Pixmap::new(256, 256).unwrap();
        let mut fill = Paint::default();
        fill.set_color(Color::from_rgba8(0, 0, 255, 128));
        let mut stroke_p = Paint::default();
        stroke_p.set_color(Color::from_rgba8(0, 0, 0, 255));
        let stroke = Stroke::default();

        render_wkb_to_pixmap(
            &wkb,
            &[144.0, -38.0, 145.0, -37.0],
            256, 256,
            &mut pixmap,
            Some(&fill),
            &stroke_p,
            &stroke,
            &stroke_p,
            4.0,
            Transform::identity(),
            0.5,
        );
        assert!(pixmap.data().iter().any(|&b| b != 0));
    }

    #[test]
    fn build_linestring_path_valid() {
        let mut wkb = Vec::new();
        wkb.extend(&3u32.to_le_bytes()); // 3 points
        for &(x, y) in &[(0.0f64, 0.0f64), (1.0, 1.0), (2.0, 0.0)] {
            wkb.extend(&x.to_le_bytes());
            wkb.extend(&y.to_le_bytes());
        }
        let mut pb = PathBuilder::new();
        let mut cursor = 0;
        let ok = build_linestring_path(
            &mut pb, &wkb, &mut cursor, true, &[0.0, 0.0, 2.0, 1.0], 256, 256, 0.5,
        );
        assert!(ok);
        assert!(pb.finish().is_some());
    }
}

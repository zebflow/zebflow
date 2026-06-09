//! GeoParquet resolution via DataFusion + GeoDataFusion.
//!
//! Queries a GeoParquet file using spatial SQL: `ST_Intersects(geometry, ST_MakeEnvelope(...))`
//! with DataFusion for query execution and GeoDataFusion for ST_* functions.
//!
//! **Performance**: SessionContext + geometry column detection are cached per parquet file
//! in a module-level pool. Cloning a SessionContext is cheap (Arc bump on internal state).
//! File mtime+size are checked on every request to detect republished data.

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use datafusion::prelude::{ParquetReadOptions, SessionConfig, SessionContext};
use serde_json::{Value, json};

use crate::mapserver::publish::manifest::PublishedLayerManifest;

use super::{ResolveRequest, ResolveResponse};

// ── SessionContext pool ──────────────────────────────────────────────
// Uses tokio::sync::Mutex so the lock can be held across await points.
// This serializes context creation per parquet file, preventing the
// thundering herd problem where N concurrent tile requests would each
// create their own SessionContext.

struct BboxCols {
    xmin: String,
    ymin: String,
    xmax: String,
    ymax: String,
}

struct ContextEntry {
    ctx: SessionContext,
    geom_col: String,
    bbox_cols: Option<BboxCols>,
    file_mtime: u64,
    file_size: u64,
}

fn context_pool() -> &'static tokio::sync::Mutex<HashMap<String, ContextEntry>> {
    static POOL: OnceLock<tokio::sync::Mutex<HashMap<String, ContextEntry>>> = OnceLock::new();
    POOL.get_or_init(|| tokio::sync::Mutex::new(HashMap::new()))
}

/// Get file mtime (seconds since epoch) and size for staleness checks.
fn file_version(path: &Path) -> Option<(u64, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some((mtime, meta.len()))
}

/// Resolve features from a GeoParquet file using DataFusion spatial queries.
pub fn resolve_from_geoparquet(
    manifest: &PublishedLayerManifest,
    request: &ResolveRequest,
) -> Result<ResolveResponse, String> {
    if manifest.bbox_required && request.bbox.is_none() {
        return Err("bbox is required for this layer".to_string());
    }

    let source_path = Path::new(&manifest.source_ref);
    if !source_path.exists() {
        return Err(format!(
            "geoparquet source file not found: {}",
            source_path.display()
        ));
    }

    // DataFusion is async; use block_in_place since we're called from an async context
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(resolve_async(manifest, request, source_path))
    })
}

async fn resolve_async(
    manifest: &PublishedLayerManifest,
    request: &ResolveRequest,
    source_path: &Path,
) -> Result<ResolveResponse, String> {
    let pool_key = source_path.to_string_lossy().to_string();
    let current_version = file_version(source_path).unwrap_or((0, 0));

    // Hold the async lock during both lookup and (if needed) creation.
    // This serializes context creation per file — the first request creates
    // the context, subsequent concurrent requests wait and then clone it.
    let (ctx, geom_col, bbox_cols) = {
        let mut pool = context_pool().lock().await;

        let needs_create = pool
            .get(&pool_key)
            .map(|entry| {
                entry.file_mtime != current_version.0 || entry.file_size != current_version.1
            })
            .unwrap_or(true);

        if needs_create {
            let config = SessionConfig::new()
                .set_bool("datafusion.execution.parquet.pushdown_filters", true)
                .set_bool("datafusion.execution.parquet.reorder_filters", true);
            let ctx = SessionContext::new_with_config(config);
            geodatafusion::register(&ctx);

            let path_str = source_path.to_string_lossy();
            ctx.register_parquet("layer", &path_str, ParquetReadOptions::default())
                .await
                .map_err(|e| format!("failed to register parquet: {e}"))?;

            let geom_col = detect_geometry_column(&ctx).await?;
            let bbox_cols = detect_bbox_columns(&ctx).await;

            pool.insert(
                pool_key.clone(),
                ContextEntry {
                    ctx: ctx.clone(),
                    geom_col: geom_col.clone(),
                    bbox_cols: bbox_cols.as_ref().map(|b| BboxCols {
                        xmin: b.xmin.clone(),
                        ymin: b.ymin.clone(),
                        xmax: b.xmax.clone(),
                        ymax: b.ymax.clone(),
                    }),
                    file_mtime: current_version.0,
                    file_size: current_version.1,
                },
            );
            (ctx, geom_col, bbox_cols)
        } else {
            let entry = pool.get(&pool_key).unwrap();
            let bbox = entry.bbox_cols.as_ref().map(|b| BboxCols {
                xmin: b.xmin.clone(),
                ymin: b.ymin.clone(),
                xmax: b.xmax.clone(),
                ymax: b.ymax.clone(),
            });
            (entry.ctx.clone(), entry.geom_col.clone(), bbox)
        }
    };

    // Use the caller-provided limit directly if set (tile rendering passes
    // a tile_limit that is not capped by max_features — same as GeoServer WMS).
    // Only fall back to / cap at max_features when no explicit limit is given.
    let hard_limit = match request.limit {
        Some(l) => l,
        None => manifest.max_features,
    };

    // Build property selection
    let select_cols = if manifest.allowed_properties.is_empty() {
        "*".to_string()
    } else {
        let mut cols: Vec<String> = manifest
            .allowed_properties
            .iter()
            .map(|p| format!("\"{}\"", p.replace('"', "\"\"")))
            .collect();
        cols.push(format!("\"{}\"", geom_col.replace('"', "\"\"")));
        cols.join(", ")
    };

    // Build WHERE clause for spatial + attribute filters.
    // When bbox covering columns exist (from optimized parquet), prepend a
    // float-comparison pre-filter that DataFusion can pushdown to the parquet
    // reader for row group pruning. ST_Intersects runs as a precise filter
    // on the candidate rows that survive the pre-filter.
    let where_clause = {
        let mut conditions = Vec::new();

        // Spatial conditions
        if let Some(bbox) = request.bbox {
            let (min_x, min_y, max_x, max_y) = (bbox[0], bbox[1], bbox[2], bbox[3]);

            // Pre-filter: bbox covering columns (pushdown-friendly float comparisons)
            if let Some(ref bc) = bbox_cols {
                conditions.push(format!(
                    "\"{}\" <= {max_x} AND \"{}\" >= {min_x} AND \"{}\" <= {max_y} AND \"{}\" >= {min_y}",
                    bc.xmin, bc.xmax, bc.ymin, bc.ymax
                ));
            }

            // Precise filter: spatial intersection
            conditions.push(format!(
                "ST_Intersects(\"{geom_col}\", ST_GeomFromText('POLYGON(({min_x} {min_y}, {max_x} {min_y}, {max_x} {max_y}, {min_x} {max_y}, {min_x} {min_y}))'))"
            ));
        }

        // Attribute filter
        if let Some(ref filter_str) = request.filter {
            if let Ok(filter) = super::filter_dsl::parse_filter(filter_str) {
                conditions.push(super::filter_dsl::filter_to_sql(&filter));
            }
        }

        if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        }
    };

    let sql = format!(
        "SELECT {select_cols} FROM layer {where_clause} LIMIT {hard_limit}"
    );

    let df = ctx
        .sql(&sql)
        .await
        .map_err(|e| format!("geoparquet query failed: {e}"))?;

    let batches = df
        .collect()
        .await
        .map_err(|e| format!("geoparquet collect failed: {e}"))?;

    // Convert to GeoJSON features
    let mut features: Vec<Value> = Vec::new();
    for batch in &batches {
        let geom_col_idx = batch
            .schema()
            .index_of(&geom_col)
            .map_err(|e| format!("geometry column not found in results: {e}"))?;

        for row_idx in 0..batch.num_rows() {
            let mut properties = serde_json::Map::new();
            let mut geometry = Value::Null;

            for (col_idx, field) in batch.schema().fields().iter().enumerate() {
                let col = batch.column(col_idx);
                let value = arrow_value_at(col, row_idx);

                if col_idx == geom_col_idx {
                    // Try to parse geometry — it could be WKB binary, WKT text, or GeoJSON
                    geometry = parse_geometry_value(&value);
                } else {
                    let name = field.name().clone();
                    // Skip internal bbox covering columns from output
                    if matches!(name.as_str(), "xmin" | "ymin" | "xmax" | "ymax") {
                        continue;
                    }
                    properties.insert(name, value);
                }
            }

            features.push(json!({
                "type": "Feature",
                "properties": Value::Object(properties),
                "geometry": geometry,
            }));
        }
    }

    let count = features.len();
    Ok(ResolveResponse {
        layer: manifest.layer_id.clone(),
        count,
        truncated: false, // We used LIMIT so can't know total
        features,
    })
}

/// Detect the geometry column name from the registered table schema.
async fn detect_geometry_column(ctx: &SessionContext) -> Result<String, String> {
    // Check GeoParquet metadata via SQL schema inspection
    let df = ctx
        .sql("SELECT * FROM layer LIMIT 0")
        .await
        .map_err(|e| format!("schema inspection failed: {e}"))?;

    let schema = df.schema();

    // Common geometry column names
    const GEOM_NAMES: &[&str] = &["geometry", "geom", "wkb_geometry", "the_geom", "shape"];

    for name in GEOM_NAMES {
        if schema.field_with_unqualified_name(name).is_ok() {
            return Ok(name.to_string());
        }
    }

    // Fall back to first Binary/LargeBinary column (likely WKB geometry)
    for field in schema.fields() {
        let dt = field.data_type();
        if matches!(
            dt,
            datafusion::arrow::datatypes::DataType::Binary
                | datafusion::arrow::datatypes::DataType::LargeBinary
        ) {
            return Ok(field.name().clone());
        }
    }

    Err("no geometry column found in geoparquet file; \
         expected one of: geometry, geom, wkb_geometry, the_geom, shape"
        .to_string())
}

/// Detect bbox covering columns (xmin, ymin, xmax, ymax) in the schema.
/// These are added by the spatial optimizer and enable row group pruning.
async fn detect_bbox_columns(ctx: &SessionContext) -> Option<BboxCols> {
    let df = ctx.sql("SELECT * FROM layer LIMIT 0").await.ok()?;
    let schema = df.schema();

    // Check for all four bbox columns as Float64
    let names = [
        ("xmin", "ymin", "xmax", "ymax"),
        ("min_x", "min_y", "max_x", "max_y"),
        ("bbox_xmin", "bbox_ymin", "bbox_xmax", "bbox_ymax"),
    ];

    for (xmin, ymin, xmax, ymax) in names {
        let has_all = [xmin, ymin, xmax, ymax].iter().all(|name| {
            schema
                .field_with_unqualified_name(name)
                .ok()
                .map(|f| matches!(f.data_type(), datafusion::arrow::datatypes::DataType::Float64))
                .unwrap_or(false)
        });
        if has_all {
            return Some(BboxCols {
                xmin: xmin.to_string(),
                ymin: ymin.to_string(),
                xmax: xmax.to_string(),
                ymax: ymax.to_string(),
            });
        }
    }

    None
}

/// Extract a scalar value from an Arrow array at a given row index.
fn arrow_value_at(col: &dyn datafusion::arrow::array::Array, idx: usize) -> Value {
    use datafusion::arrow::array::*;
    use datafusion::arrow::datatypes::DataType;

    if col.is_null(idx) {
        return Value::Null;
    }

    match col.data_type() {
        DataType::Boolean => {
            let arr = col.as_any().downcast_ref::<BooleanArray>().unwrap();
            json!(arr.value(idx))
        }
        DataType::Int8 => json!(col.as_any().downcast_ref::<Int8Array>().unwrap().value(idx)),
        DataType::Int16 => json!(col.as_any().downcast_ref::<Int16Array>().unwrap().value(idx)),
        DataType::Int32 => json!(col.as_any().downcast_ref::<Int32Array>().unwrap().value(idx)),
        DataType::Int64 => json!(col.as_any().downcast_ref::<Int64Array>().unwrap().value(idx)),
        DataType::UInt8 => json!(col.as_any().downcast_ref::<UInt8Array>().unwrap().value(idx)),
        DataType::UInt16 => json!(col.as_any().downcast_ref::<UInt16Array>().unwrap().value(idx)),
        DataType::UInt32 => json!(col.as_any().downcast_ref::<UInt32Array>().unwrap().value(idx)),
        DataType::UInt64 => json!(col.as_any().downcast_ref::<UInt64Array>().unwrap().value(idx)),
        DataType::Float32 => {
            let v = col
                .as_any()
                .downcast_ref::<Float32Array>()
                .unwrap()
                .value(idx);
            json!(v)
        }
        DataType::Float64 => {
            let v = col
                .as_any()
                .downcast_ref::<Float64Array>()
                .unwrap()
                .value(idx);
            json!(v)
        }
        DataType::Utf8 => {
            let arr = col.as_any().downcast_ref::<StringArray>().unwrap();
            json!(arr.value(idx))
        }
        DataType::LargeUtf8 => {
            let arr = col.as_any().downcast_ref::<LargeStringArray>().unwrap();
            json!(arr.value(idx))
        }
        DataType::Binary => {
            let arr = col.as_any().downcast_ref::<BinaryArray>().unwrap();
            // Return raw bytes as base64 — geometry parsing is handled separately
            Value::String(base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                arr.value(idx),
            ))
        }
        DataType::LargeBinary => {
            let arr = col.as_any().downcast_ref::<LargeBinaryArray>().unwrap();
            Value::String(base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                arr.value(idx),
            ))
        }
        _ => {
            // Fallback: use Display
            let arr = datafusion::arrow::util::display::ArrayFormatter::try_new(
                col,
                &datafusion::arrow::util::display::FormatOptions::default(),
            );
            match arr {
                Ok(formatter) => json!(formatter.value(idx).to_string()),
                Err(_) => Value::Null,
            }
        }
    }
}

/// Parse a geometry value from Arrow output into GeoJSON geometry.
///
/// Handles (in order):
/// - GeoJSON string
/// - Hex-encoded WKB (common from GeoDataFusion / PostGIS output)
/// - Base64-encoded WKB (from raw binary columns)
/// - WKT text (fallback: client can handle it)
fn parse_geometry_value(value: &Value) -> Value {
    match value {
        Value::String(s) => {
            // Try GeoJSON first
            if s.starts_with('{') {
                if let Ok(parsed) = serde_json::from_str::<Value>(s) {
                    if parsed.get("type").is_some() {
                        return parsed;
                    }
                }
            }
            // Try hex-encoded WKB (geodatafusion outputs geometry as hex strings)
            if s.len() >= 10 && s.chars().all(|c| c.is_ascii_hexdigit()) {
                if let Some(bytes) = hex_decode(s) {
                    if let Some(geojson) = wkb_to_geojson(&bytes) {
                        return geojson;
                    }
                }
            }
            // Try base64-encoded WKB
            if let Ok(bytes) = base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                s,
            ) {
                if let Some(geojson) = wkb_to_geojson(&bytes) {
                    return geojson;
                }
            }
            // Return as-is (possibly WKT) — client can handle it
            json!({"type": "GeometryFromWKT", "wkt": s})
        }
        _ => Value::Null,
    }
}

/// Decode a hex string to bytes.
fn hex_decode(hex: &str) -> Option<Vec<u8>> {
    if hex.len() % 2 != 0 {
        return None;
    }
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for chunk in hex.as_bytes().chunks(2) {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        bytes.push((hi << 4) | lo);
    }
    Some(bytes)
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Convert WKB (Well-Known Binary) to GeoJSON geometry.
///
/// Delegates to geonative_core for WKB parsing and geonative_geojson for
/// GeoJSON serialization.
fn wkb_to_geojson(wkb: &[u8]) -> Option<Value> {
    let geom = geonative_core::Geometry::from_wkb(wkb).ok()?;
    Some(geonative_geojson::geometry::to_json(&geom))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wkb_point_to_geojson() {
        // WKB LE Point(1.0, 2.0)
        let mut wkb = vec![1u8]; // LE
        wkb.extend(&1u32.to_le_bytes()); // Point type
        wkb.extend(&1.0f64.to_le_bytes());
        wkb.extend(&2.0f64.to_le_bytes());

        let result = wkb_to_geojson(&wkb).unwrap();
        assert_eq!(result["type"], "Point");
        assert_eq!(result["coordinates"][0], 1.0);
        assert_eq!(result["coordinates"][1], 2.0);
    }

    #[test]
    fn wkb_polygon_to_geojson() {
        // WKB LE Polygon with one ring: 4 points forming a square
        let mut wkb = vec![1u8]; // LE
        wkb.extend(&3u32.to_le_bytes()); // Polygon type
        wkb.extend(&1u32.to_le_bytes()); // 1 ring
        wkb.extend(&5u32.to_le_bytes()); // 5 points (closed)
        for &(x, y) in &[
            (0.0f64, 0.0f64),
            (1.0, 0.0),
            (1.0, 1.0),
            (0.0, 1.0),
            (0.0, 0.0),
        ] {
            wkb.extend(&x.to_le_bytes());
            wkb.extend(&y.to_le_bytes());
        }

        let result = wkb_to_geojson(&wkb).unwrap();
        assert_eq!(result["type"], "Polygon");
        let ring = result["coordinates"][0].as_array().unwrap();
        assert_eq!(ring.len(), 5);
    }
}

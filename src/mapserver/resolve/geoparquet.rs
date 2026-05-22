//! GeoParquet resolution via DataFusion + GeoDataFusion.
//!
//! Queries a GeoParquet file using spatial SQL: `ST_Intersects(geometry, ST_MakeEnvelope(...))`
//! with DataFusion for query execution and GeoDataFusion for ST_* functions.

use std::path::Path;

use datafusion::prelude::{ParquetReadOptions, SessionContext};
use serde_json::{Value, json};

use crate::mapserver::publish::manifest::PublishedLayerManifest;

use super::{ResolveRequest, ResolveResponse};

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
    let ctx = SessionContext::new();
    geodatafusion::register(&ctx);

    // Register parquet file as a table
    let path_str = source_path.to_string_lossy();
    ctx.register_parquet("layer", &path_str, ParquetReadOptions::default())
        .await
        .map_err(|e| format!("failed to register parquet: {e}"))?;

    // Detect geometry column name
    let geom_col = detect_geometry_column(&ctx).await?;

    let hard_limit = request
        .limit
        .unwrap_or(manifest.max_features)
        .min(manifest.max_features);

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

    // Build WHERE clause for spatial filter
    // geodatafusion 0.4.0 does not have ST_MakeEnvelope — use ST_GeomFromText with a WKT polygon
    let where_clause = if let Some(bbox) = request.bbox {
        let (min_x, min_y, max_x, max_y) = (bbox[0], bbox[1], bbox[2], bbox[3]);
        format!(
            "WHERE ST_Intersects(\"{geom_col}\", ST_GeomFromText('POLYGON(({min_x} {min_y}, {max_x} {min_y}, {max_x} {max_y}, {min_x} {max_y}, {min_x} {min_y}))'))"
        )
    } else {
        String::new()
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
/// Supports the common geometry types used in GeoParquet:
/// Point, LineString, Polygon, MultiPoint, MultiLineString, MultiPolygon.
fn wkb_to_geojson(wkb: &[u8]) -> Option<Value> {
    if wkb.len() < 5 {
        return None;
    }

    let byte_order = wkb[0];
    let is_le = byte_order == 1;

    let geom_type = if is_le {
        u32::from_le_bytes([wkb[1], wkb[2], wkb[3], wkb[4]])
    } else {
        u32::from_be_bytes([wkb[1], wkb[2], wkb[3], wkb[4]])
    };

    let mut cursor = 5usize;

    match geom_type {
        1 => {
            // Point
            let (x, y, new_cursor) = read_point(wkb, cursor, is_le)?;
            let _ = new_cursor;
            Some(json!({"type": "Point", "coordinates": [x, y]}))
        }
        2 => {
            // LineString
            let (coords, _) = read_linestring(wkb, cursor, is_le)?;
            Some(json!({"type": "LineString", "coordinates": coords}))
        }
        3 => {
            // Polygon
            let (rings, _) = read_polygon(wkb, cursor, is_le)?;
            Some(json!({"type": "Polygon", "coordinates": rings}))
        }
        4 => {
            // MultiPoint
            let num = read_u32(wkb, cursor, is_le)?;
            cursor += 4;
            let mut points = Vec::new();
            for _ in 0..num {
                if cursor + 5 > wkb.len() {
                    return None;
                }
                let inner_le = wkb[cursor] == 1;
                cursor += 5; // skip byte_order + geom_type
                let (x, y, c) = read_point(wkb, cursor, inner_le)?;
                cursor = c;
                points.push(json!([x, y]));
            }
            Some(json!({"type": "MultiPoint", "coordinates": points}))
        }
        5 => {
            // MultiLineString
            let num = read_u32(wkb, cursor, is_le)?;
            cursor += 4;
            let mut lines = Vec::new();
            for _ in 0..num {
                if cursor + 5 > wkb.len() {
                    return None;
                }
                let inner_le = wkb[cursor] == 1;
                cursor += 5;
                let (coords, c) = read_linestring(wkb, cursor, inner_le)?;
                cursor = c;
                lines.push(coords);
            }
            Some(json!({"type": "MultiLineString", "coordinates": lines}))
        }
        6 => {
            // MultiPolygon
            let num = read_u32(wkb, cursor, is_le)?;
            cursor += 4;
            let mut polys = Vec::new();
            for _ in 0..num {
                if cursor + 5 > wkb.len() {
                    return None;
                }
                let inner_le = wkb[cursor] == 1;
                cursor += 5;
                let (rings, c) = read_polygon(wkb, cursor, inner_le)?;
                cursor = c;
                polys.push(rings);
            }
            Some(json!({"type": "MultiPolygon", "coordinates": polys}))
        }
        _ => None,
    }
}

fn read_f64(wkb: &[u8], offset: usize, is_le: bool) -> Option<f64> {
    if offset + 8 > wkb.len() {
        return None;
    }
    let bytes: [u8; 8] = wkb[offset..offset + 8].try_into().ok()?;
    Some(if is_le {
        f64::from_le_bytes(bytes)
    } else {
        f64::from_be_bytes(bytes)
    })
}

fn read_u32(wkb: &[u8], offset: usize, is_le: bool) -> Option<u32> {
    if offset + 4 > wkb.len() {
        return None;
    }
    let bytes: [u8; 4] = wkb[offset..offset + 4].try_into().ok()?;
    Some(if is_le {
        u32::from_le_bytes(bytes)
    } else {
        u32::from_be_bytes(bytes)
    })
}

fn read_point(wkb: &[u8], offset: usize, is_le: bool) -> Option<(f64, f64, usize)> {
    let x = read_f64(wkb, offset, is_le)?;
    let y = read_f64(wkb, offset + 8, is_le)?;
    Some((x, y, offset + 16))
}

fn read_linestring(wkb: &[u8], offset: usize, is_le: bool) -> Option<(Vec<Value>, usize)> {
    let num = read_u32(wkb, offset, is_le)?;
    let mut cursor = offset + 4;
    let mut coords = Vec::new();
    for _ in 0..num {
        let (x, y, c) = read_point(wkb, cursor, is_le)?;
        cursor = c;
        coords.push(json!([x, y]));
    }
    Some((coords, cursor))
}

fn read_polygon(wkb: &[u8], offset: usize, is_le: bool) -> Option<(Vec<Vec<Value>>, usize)> {
    let num_rings = read_u32(wkb, offset, is_le)?;
    let mut cursor = offset + 4;
    let mut rings = Vec::new();
    for _ in 0..num_rings {
        let (ring, c) = read_linestring(wkb, cursor, is_le)?;
        cursor = c;
        rings.push(ring);
    }
    Some((rings, cursor))
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

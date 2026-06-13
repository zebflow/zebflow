//! MVT (Mapbox Vector Tile) encoder.
//!
//! Implements the [MVT 2.1 specification](https://github.com/mapbox/vector-tile-spec/tree/master/2.1)
//! with zero external protobuf dependencies. The wire format is encoded inline.
//!
//! Two encoding paths:
//! - `features_to_mvt` — from resolved GeoJSON features (`Vec<Value>`)
//! - `encode_wkb_feature` — from raw WKB bytes (for direct GeoParquet path)

use std::collections::HashMap;
use std::io::Write;

// ── Protobuf wire format ────────────────────────────────────────────────────

/// Encode a varint (LEB128).
fn encode_varint(buf: &mut Vec<u8>, mut val: u64) {
    loop {
        let byte = (val & 0x7F) as u8;
        val >>= 7;
        if val == 0 {
            buf.push(byte);
            return;
        }
        buf.push(byte | 0x80);
    }
}

/// Encode a protobuf field tag.
fn encode_tag(buf: &mut Vec<u8>, field: u32, wire_type: u8) {
    encode_varint(buf, ((field as u64) << 3) | wire_type as u64);
}

/// Encode a length-delimited field (wire type 2).
fn encode_bytes(buf: &mut Vec<u8>, field: u32, data: &[u8]) {
    encode_tag(buf, field, 2);
    encode_varint(buf, data.len() as u64);
    buf.extend_from_slice(data);
}

/// Encode a string field.
fn encode_string(buf: &mut Vec<u8>, field: u32, s: &str) {
    encode_bytes(buf, field, s.as_bytes());
}

/// Encode a varint field (wire type 0).
fn encode_varint_field(buf: &mut Vec<u8>, field: u32, val: u64) {
    encode_tag(buf, field, 0);
    encode_varint(buf, val);
}

/// Encode a double field (wire type 1, fixed64).
fn encode_double_field(buf: &mut Vec<u8>, field: u32, val: f64) {
    encode_tag(buf, field, 1);
    buf.extend_from_slice(&val.to_le_bytes());
}

/// Encode a sint64 field (wire type 0, zigzag-encoded).
fn encode_sint64_field(buf: &mut Vec<u8>, field: u32, val: i64) {
    encode_tag(buf, field, 0);
    encode_varint(buf, zigzag64(val));
}

/// Encode packed repeated uint32 field (wire type 2).
fn encode_packed_uint32(buf: &mut Vec<u8>, field: u32, vals: &[u32]) {
    if vals.is_empty() {
        return;
    }
    let mut inner = Vec::new();
    for &v in vals {
        encode_varint(&mut inner, v as u64);
    }
    encode_bytes(buf, field, &inner);
}

/// Zigzag encode i32 → u32.
fn zigzag(v: i32) -> u32 {
    ((v << 1) ^ (v >> 31)) as u32
}

/// Zigzag encode i64 → u64.
fn zigzag64(v: i64) -> u64 {
    ((v << 1) ^ (v >> 63)) as u64
}

// ── MVT geometry commands ───────────────────────────────────────────────────

const CMD_MOVE_TO: u32 = 1;
const CMD_LINE_TO: u32 = 2;
const CMD_CLOSE_PATH: u32 = 7;

/// Build a command integer: (id & 0x7) | (count << 3).
fn command_integer(id: u32, count: u32) -> u32 {
    (id & 0x7) | (count << 3)
}

/// MVT geometry type constants (matches GeomType enum in the spec).
const GEOM_POINT: u32 = 1;
const GEOM_LINESTRING: u32 = 2;
const GEOM_POLYGON: u32 = 3;

/// Encode a single point as MVT commands. Returns (geom_type, commands).
fn encode_point_geom(x: i32, y: i32, cursor: &mut (i32, i32)) -> Vec<u32> {
    let dx = x - cursor.0;
    let dy = y - cursor.1;
    *cursor = (x, y);
    vec![command_integer(CMD_MOVE_TO, 1), zigzag(dx), zigzag(dy)]
}

/// Encode multiple points (MultiPoint) as MVT commands.
fn encode_multi_point_geom(points: &[(i32, i32)], cursor: &mut (i32, i32)) -> Vec<u32> {
    if points.is_empty() {
        return vec![];
    }
    let mut cmds = Vec::with_capacity(1 + points.len() * 2);
    cmds.push(command_integer(CMD_MOVE_TO, points.len() as u32));
    for &(x, y) in points {
        let dx = x - cursor.0;
        let dy = y - cursor.1;
        *cursor = (x, y);
        cmds.push(zigzag(dx));
        cmds.push(zigzag(dy));
    }
    cmds
}

/// Encode a linestring as MVT commands.
fn encode_linestring_geom(coords: &[(i32, i32)], cursor: &mut (i32, i32)) -> Vec<u32> {
    if coords.len() < 2 {
        return vec![];
    }
    let mut cmds = Vec::with_capacity(3 + (coords.len() - 1) * 2);
    // MoveTo first point
    let (x0, y0) = coords[0];
    let dx = x0 - cursor.0;
    let dy = y0 - cursor.1;
    *cursor = (x0, y0);
    cmds.push(command_integer(CMD_MOVE_TO, 1));
    cmds.push(zigzag(dx));
    cmds.push(zigzag(dy));
    // LineTo remaining points
    cmds.push(command_integer(CMD_LINE_TO, (coords.len() - 1) as u32));
    for &(x, y) in &coords[1..] {
        let dx = x - cursor.0;
        let dy = y - cursor.1;
        *cursor = (x, y);
        cmds.push(zigzag(dx));
        cmds.push(zigzag(dy));
    }
    cmds
}

/// Encode a polygon (exterior ring + holes) as MVT commands.
/// Each ring: MoveTo + LineTo + ClosePath.
fn encode_polygon_geom(rings: &[Vec<(i32, i32)>], cursor: &mut (i32, i32)) -> Vec<u32> {
    let mut cmds = Vec::new();
    for ring in rings {
        // Need at least 3 unique points for a valid ring (last point = first, so 4 coords min)
        if ring.len() < 3 {
            continue;
        }
        // Drop the closing point if it duplicates the first (MVT uses ClosePath instead)
        let coords = if ring.len() >= 2 && ring.first() == ring.last() {
            &ring[..ring.len() - 1]
        } else {
            ring.as_slice()
        };
        if coords.len() < 3 {
            continue;
        }
        // MoveTo first point
        let (x0, y0) = coords[0];
        let dx = x0 - cursor.0;
        let dy = y0 - cursor.1;
        *cursor = (x0, y0);
        cmds.push(command_integer(CMD_MOVE_TO, 1));
        cmds.push(zigzag(dx));
        cmds.push(zigzag(dy));
        // LineTo remaining points
        cmds.push(command_integer(CMD_LINE_TO, (coords.len() - 1) as u32));
        for &(x, y) in &coords[1..] {
            let dx = x - cursor.0;
            let dy = y - cursor.1;
            *cursor = (x, y);
            cmds.push(zigzag(dx));
            cmds.push(zigzag(dy));
        }
        // ClosePath
        cmds.push(command_integer(CMD_CLOSE_PATH, 1));
    }
    cmds
}

// ── Coordinate projection ───────────────────────────────────────────────────

/// Default MVT extent (coordinate space per tile).
const MVT_EXTENT: u32 = 4096;

/// Project WGS84 lon/lat to MVT tile coordinates [0..extent].
fn geo_to_mvt(lon: f64, lat: f64, bbox: &[f64; 4], extent: u32) -> (i32, i32) {
    let dx = bbox[2] - bbox[0];
    let dy = bbox[3] - bbox[1];
    if dx.abs() < 1e-12 || dy.abs() < 1e-12 {
        return (0, 0);
    }
    let x = ((lon - bbox[0]) / dx * extent as f64).round() as i32;
    let y = ((bbox[3] - lat) / dy * extent as f64).round() as i32; // Y flipped
    (x, y)
}

// ── Tag (key/value) table ───────────────────────────────────────────────────

/// MVT property value types.
#[derive(Debug, Clone)]
pub(crate) enum MvtValue {
    Str(String),
    F64(f64),
    I64(i64),
    Bool(bool),
}

/// Hashable key for value deduplication.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum MvtValueKey {
    Str(String),
    F64Bits(u64),
    I64(i64),
    Bool(bool),
}

impl MvtValue {
    fn to_key(&self) -> MvtValueKey {
        match self {
            MvtValue::Str(s) => MvtValueKey::Str(s.clone()),
            MvtValue::F64(v) => MvtValueKey::F64Bits(v.to_bits()),
            MvtValue::I64(v) => MvtValueKey::I64(*v),
            MvtValue::Bool(v) => MvtValueKey::Bool(*v),
        }
    }

    /// Encode as protobuf Value message bytes.
    fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        match self {
            MvtValue::Str(s) => encode_string(&mut buf, 1, s),
            MvtValue::F64(v) => encode_double_field(&mut buf, 3, *v),
            MvtValue::I64(v) => encode_sint64_field(&mut buf, 6, *v),
            MvtValue::Bool(v) => encode_varint_field(&mut buf, 7, *v as u64),
        }
        buf
    }
}

/// Interned key/value tables for a layer's tags.
pub(crate) struct TagTable {
    keys: Vec<String>,
    values: Vec<MvtValue>,
    key_index: HashMap<String, u32>,
    value_index: HashMap<MvtValueKey, u32>,
}

impl TagTable {
    pub(crate) fn new() -> Self {
        Self {
            keys: Vec::new(),
            values: Vec::new(),
            key_index: HashMap::new(),
            value_index: HashMap::new(),
        }
    }

    fn intern_key(&mut self, key: &str) -> u32 {
        if let Some(&idx) = self.key_index.get(key) {
            return idx;
        }
        let idx = self.keys.len() as u32;
        self.keys.push(key.to_string());
        self.key_index.insert(key.to_string(), idx);
        idx
    }

    fn intern_value(&mut self, val: MvtValue) -> u32 {
        let key = val.to_key();
        if let Some(&idx) = self.value_index.get(&key) {
            return idx;
        }
        let idx = self.values.len() as u32;
        self.values.push(val);
        self.value_index.insert(key, idx);
        idx
    }

    /// Encode all keys as `repeated string keys = 3`.
    fn encode_keys(&self, buf: &mut Vec<u8>) {
        for k in &self.keys {
            encode_string(buf, 3, k);
        }
    }

    /// Encode all values as `repeated Value values = 4`.
    fn encode_values(&self, buf: &mut Vec<u8>) {
        for v in &self.values {
            let val_bytes = v.encode();
            encode_bytes(buf, 4, &val_bytes);
        }
    }
}

// ── GeoJSON coordinate extraction ───────────────────────────────────────────

/// Extract a coordinate pair [lon, lat] from a JSON array.
fn json_coord(val: &serde_json::Value, bbox: &[f64; 4], extent: u32) -> Option<(i32, i32)> {
    let arr = val.as_array()?;
    let lon = arr.first()?.as_f64()?;
    let lat = arr.get(1)?.as_f64()?;
    Some(geo_to_mvt(lon, lat, bbox, extent))
}

/// Extract a ring of coordinates from a JSON array of arrays.
fn json_ring(coords: &serde_json::Value, bbox: &[f64; 4], extent: u32) -> Vec<(i32, i32)> {
    coords
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|c| json_coord(c, bbox, extent))
                .collect()
        })
        .unwrap_or_default()
}

// ── GeoJSON feature → MVT feature ──────────────────────────────────────────

/// Encode a GeoJSON feature into MVT feature protobuf bytes.
/// Returns None if the feature has no valid geometry.
fn encode_geojson_feature(
    feature: &serde_json::Value,
    bbox: &[f64; 4],
    extent: u32,
    tags: &mut TagTable,
    cursor: &mut (i32, i32),
    feature_id: &mut u64,
    allowed_properties: &[String],
) -> Option<Vec<u8>> {
    let geometry = feature.get("geometry")?;
    let geom_type = geometry.get("type")?.as_str()?;
    let coordinates = geometry.get("coordinates")?;

    let (mvt_type, commands) = match geom_type {
        "Point" => {
            let (x, y) = json_coord(coordinates, bbox, extent)?;
            (GEOM_POINT, encode_point_geom(x, y, cursor))
        }
        "MultiPoint" => {
            let points: Vec<(i32, i32)> = coordinates
                .as_array()?
                .iter()
                .filter_map(|c| json_coord(c, bbox, extent))
                .collect();
            if points.is_empty() {
                return None;
            }
            (GEOM_POINT, encode_multi_point_geom(&points, cursor))
        }
        "LineString" => {
            let coords = json_ring(coordinates, bbox, extent);
            if coords.len() < 2 {
                return None;
            }
            (GEOM_LINESTRING, encode_linestring_geom(&coords, cursor))
        }
        "MultiLineString" => {
            let lines: Vec<Vec<(i32, i32)>> = coordinates
                .as_array()?
                .iter()
                .map(|line| json_ring(line, bbox, extent))
                .filter(|c| c.len() >= 2)
                .collect();
            if lines.is_empty() {
                return None;
            }
            let mut cmds = Vec::new();
            for line in &lines {
                cmds.extend(encode_linestring_geom(line, cursor));
            }
            (GEOM_LINESTRING, cmds)
        }
        "Polygon" => {
            let rings: Vec<Vec<(i32, i32)>> = coordinates
                .as_array()?
                .iter()
                .map(|ring| json_ring(ring, bbox, extent))
                .collect();
            let cmds = encode_polygon_geom(&rings, cursor);
            if cmds.is_empty() {
                return None;
            }
            (GEOM_POLYGON, cmds)
        }
        "MultiPolygon" => {
            let mut all_cmds = Vec::new();
            for poly in coordinates.as_array()? {
                let rings: Vec<Vec<(i32, i32)>> = poly
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .map(|ring| json_ring(ring, bbox, extent))
                            .collect()
                    })
                    .unwrap_or_default();
                all_cmds.extend(encode_polygon_geom(&rings, cursor));
            }
            if all_cmds.is_empty() {
                return None;
            }
            (GEOM_POLYGON, all_cmds)
        }
        _ => return None,
    };

    // Build tags from properties
    let mut tag_pairs = Vec::new();
    if let Some(props) = feature.get("properties").and_then(|p| p.as_object()) {
        for (key, val) in props {
            if !allowed_properties.is_empty() && !allowed_properties.iter().any(|p| p == key) {
                continue;
            }
            let mvt_val = json_value_to_mvt(val)?;
            let ki = tags.intern_key(key);
            let vi = tags.intern_value(mvt_val);
            tag_pairs.push(ki);
            tag_pairs.push(vi);
        }
    }

    // Encode Feature message
    let id = *feature_id;
    *feature_id += 1;
    let mut feat_buf = Vec::new();
    // optional uint64 id = 1
    if id > 0 {
        encode_varint_field(&mut feat_buf, 1, id);
    }
    // repeated uint32 tags = 2 (packed)
    encode_packed_uint32(&mut feat_buf, 2, &tag_pairs);
    // optional GeomType type = 3
    encode_varint_field(&mut feat_buf, 3, mvt_type as u64);
    // repeated uint32 geometry = 4 (packed)
    encode_packed_uint32(&mut feat_buf, 4, &commands);

    Some(feat_buf)
}

/// Convert a JSON value to an MvtValue.
fn json_value_to_mvt(val: &serde_json::Value) -> Option<MvtValue> {
    match val {
        serde_json::Value::String(s) => Some(MvtValue::Str(s.clone())),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(MvtValue::I64(i))
            } else if let Some(f) = n.as_f64() {
                Some(MvtValue::F64(f))
            } else {
                None
            }
        }
        serde_json::Value::Bool(b) => Some(MvtValue::Bool(*b)),
        _ => None, // skip null, array, object
    }
}

// ── WKB feature → MVT feature ───────────────────────────────────────────────

/// Encode a WKB geometry with properties into MVT feature protobuf bytes.
pub(crate) fn encode_wkb_feature(
    wkb: &[u8],
    properties: &[(&str, MvtValue)],
    bbox: &[f64; 4],
    extent: u32,
    tags: &mut TagTable,
    cursor: &mut (i32, i32),
    feature_id: &mut u64,
) -> Option<Vec<u8>> {
    if wkb.len() < 5 {
        return None;
    }
    let le = wkb[0] == 1;
    let geom_type_raw = wkb_read_u32(wkb, 1, le);

    let (mvt_type, commands) = match geom_type_raw {
        1 => {
            // Point
            if wkb.len() < 21 {
                return None;
            }
            let lon = wkb_read_f64(wkb, 5, le);
            let lat = wkb_read_f64(wkb, 13, le);
            let (x, y) = geo_to_mvt(lon, lat, bbox, extent);
            (GEOM_POINT, encode_point_geom(x, y, cursor))
        }
        2 => {
            // LineString
            if wkb.len() < 9 {
                return None;
            }
            let n = wkb_read_u32(wkb, 5, le) as usize;
            let coords = wkb_read_coords(wkb, 9, n, le, bbox, extent);
            if coords.len() < 2 {
                return None;
            }
            (GEOM_LINESTRING, encode_linestring_geom(&coords, cursor))
        }
        3 => {
            // Polygon
            if wkb.len() < 9 {
                return None;
            }
            let num_rings = wkb_read_u32(wkb, 5, le) as usize;
            let mut offset = 9;
            let mut rings = Vec::with_capacity(num_rings);
            for _ in 0..num_rings {
                if offset + 4 > wkb.len() {
                    break;
                }
                let n = wkb_read_u32(wkb, offset, le) as usize;
                offset += 4;
                rings.push(wkb_read_coords(wkb, offset, n, le, bbox, extent));
                offset += n * 16;
            }
            let cmds = encode_polygon_geom(&rings, cursor);
            if cmds.is_empty() {
                return None;
            }
            (GEOM_POLYGON, cmds)
        }
        4 => {
            // MultiPoint
            if wkb.len() < 9 {
                return None;
            }
            let num = wkb_read_u32(wkb, 5, le) as usize;
            let mut offset = 9;
            let mut points = Vec::with_capacity(num);
            for _ in 0..num {
                if offset + 21 > wkb.len() {
                    break;
                }
                let sub_le = wkb[offset] == 1;
                let lon = wkb_read_f64(wkb, offset + 5, sub_le);
                let lat = wkb_read_f64(wkb, offset + 13, sub_le);
                points.push(geo_to_mvt(lon, lat, bbox, extent));
                offset += 21;
            }
            if points.is_empty() {
                return None;
            }
            (GEOM_POINT, encode_multi_point_geom(&points, cursor))
        }
        5 => {
            // MultiLineString
            if wkb.len() < 9 {
                return None;
            }
            let num = wkb_read_u32(wkb, 5, le) as usize;
            let mut offset = 9;
            let mut all_cmds = Vec::new();
            for _ in 0..num {
                if offset + 9 > wkb.len() {
                    break;
                }
                let sub_le = wkb[offset] == 1;
                // skip byte order + type (5 bytes)
                let n = wkb_read_u32(wkb, offset + 5, sub_le) as usize;
                offset += 9;
                let coords = wkb_read_coords(wkb, offset, n, sub_le, bbox, extent);
                if coords.len() >= 2 {
                    all_cmds.extend(encode_linestring_geom(&coords, cursor));
                }
                offset += n * 16;
            }
            if all_cmds.is_empty() {
                return None;
            }
            (GEOM_LINESTRING, all_cmds)
        }
        6 => {
            // MultiPolygon
            if wkb.len() < 9 {
                return None;
            }
            let num_polys = wkb_read_u32(wkb, 5, le) as usize;
            let mut offset = 9;
            let mut all_cmds = Vec::new();
            for _ in 0..num_polys {
                if offset + 9 > wkb.len() {
                    break;
                }
                let sub_le = wkb[offset] == 1;
                let num_rings = wkb_read_u32(wkb, offset + 5, sub_le) as usize;
                offset += 9;
                let mut rings = Vec::with_capacity(num_rings);
                for _ in 0..num_rings {
                    if offset + 4 > wkb.len() {
                        break;
                    }
                    let n = wkb_read_u32(wkb, offset, sub_le) as usize;
                    offset += 4;
                    rings.push(wkb_read_coords(wkb, offset, n, sub_le, bbox, extent));
                    offset += n * 16;
                }
                all_cmds.extend(encode_polygon_geom(&rings, cursor));
            }
            if all_cmds.is_empty() {
                return None;
            }
            (GEOM_POLYGON, all_cmds)
        }
        _ => return None,
    };

    // Build tags
    let mut tag_pairs = Vec::new();
    for (key, val) in properties {
        let ki = tags.intern_key(key);
        let vi = tags.intern_value(val.clone());
        tag_pairs.push(ki);
        tag_pairs.push(vi);
    }

    let id = *feature_id;
    *feature_id += 1;
    let mut feat_buf = Vec::new();
    if id > 0 {
        encode_varint_field(&mut feat_buf, 1, id);
    }
    encode_packed_uint32(&mut feat_buf, 2, &tag_pairs);
    encode_varint_field(&mut feat_buf, 3, mvt_type as u64);
    encode_packed_uint32(&mut feat_buf, 4, &commands);

    Some(feat_buf)
}

// ── WKB helpers ─────────────────────────────────────────────────────────────

fn wkb_read_u32(buf: &[u8], offset: usize, le: bool) -> u32 {
    let b = &buf[offset..offset + 4];
    if le {
        u32::from_le_bytes([b[0], b[1], b[2], b[3]])
    } else {
        u32::from_be_bytes([b[0], b[1], b[2], b[3]])
    }
}

fn wkb_read_f64(buf: &[u8], offset: usize, le: bool) -> f64 {
    let b = &buf[offset..offset + 8];
    if le {
        f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
    } else {
        f64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
    }
}

fn wkb_read_coords(
    buf: &[u8],
    start: usize,
    count: usize,
    le: bool,
    bbox: &[f64; 4],
    extent: u32,
) -> Vec<(i32, i32)> {
    let mut coords = Vec::with_capacity(count);
    let mut offset = start;
    for _ in 0..count {
        if offset + 16 > buf.len() {
            break;
        }
        let lon = wkb_read_f64(buf, offset, le);
        let lat = wkb_read_f64(buf, offset + 8, le);
        coords.push(geo_to_mvt(lon, lat, bbox, extent));
        offset += 16;
    }
    coords
}

// ── Tile assembly ───────────────────────────────────────────────────────────

/// Build a complete MVT tile (uncompressed) from GeoJSON features.
pub fn features_to_mvt(
    layer_name: &str,
    features: &[serde_json::Value],
    bbox: &[f64; 4],
    allowed_properties: &[String],
) -> Vec<u8> {
    let mut tags = TagTable::new();
    let mut cursor = (0i32, 0i32);
    let mut feature_id = 1u64;
    let mut encoded_features = Vec::new();

    for feature in features {
        if let Some(feat_bytes) = encode_geojson_feature(
            feature,
            bbox,
            MVT_EXTENT,
            &mut tags,
            &mut cursor,
            &mut feature_id,
            allowed_properties,
        ) {
            encoded_features.push(feat_bytes);
        }
    }

    assemble_tile(layer_name, &tags, &encoded_features)
}

/// Build a complete MVT tile, gzip-compressed.
pub fn features_to_mvt_gz(
    layer_name: &str,
    features: &[serde_json::Value],
    bbox: &[f64; 4],
    allowed_properties: &[String],
) -> Vec<u8> {
    let raw = features_to_mvt(layer_name, features, bbox, allowed_properties);
    gzip_compress(&raw)
}

/// Assemble Layer + Tile protobuf from encoded features and tag table.
pub(crate) fn assemble_tile(
    layer_name: &str,
    tags: &TagTable,
    encoded_features: &[Vec<u8>],
) -> Vec<u8> {
    // Build Layer message
    let mut layer_buf = Vec::new();

    // required uint32 version = 15
    encode_varint_field(&mut layer_buf, 15, 2);
    // required string name = 1
    encode_string(&mut layer_buf, 1, layer_name);
    // repeated Feature features = 2
    for feat in encoded_features {
        encode_bytes(&mut layer_buf, 2, feat);
    }
    // repeated string keys = 3
    tags.encode_keys(&mut layer_buf);
    // repeated Value values = 4
    tags.encode_values(&mut layer_buf);
    // optional uint32 extent = 5
    encode_varint_field(&mut layer_buf, 5, MVT_EXTENT as u64);

    // Build Tile message
    let mut tile_buf = Vec::new();
    // repeated Layer layers = 3
    encode_bytes(&mut tile_buf, 3, &layer_buf);

    tile_buf
}

/// Gzip compress bytes.
pub(crate) fn gzip_compress(data: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(data).unwrap_or_default();
    encoder.finish().unwrap_or_default()
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn varint_encoding() {
        let mut buf = Vec::new();
        encode_varint(&mut buf, 0);
        assert_eq!(buf, vec![0]);

        buf.clear();
        encode_varint(&mut buf, 1);
        assert_eq!(buf, vec![1]);

        buf.clear();
        encode_varint(&mut buf, 127);
        assert_eq!(buf, vec![127]);

        buf.clear();
        encode_varint(&mut buf, 128);
        assert_eq!(buf, vec![0x80, 0x01]);

        buf.clear();
        encode_varint(&mut buf, 300);
        assert_eq!(buf, vec![0xAC, 0x02]);
    }

    #[test]
    fn zigzag_encoding() {
        assert_eq!(zigzag(0), 0);
        assert_eq!(zigzag(-1), 1);
        assert_eq!(zigzag(1), 2);
        assert_eq!(zigzag(-2), 3);
        assert_eq!(zigzag(2), 4);
    }

    #[test]
    fn command_integer_encoding() {
        assert_eq!(command_integer(CMD_MOVE_TO, 1), 9); // (1 & 7) | (1 << 3)
        assert_eq!(command_integer(CMD_LINE_TO, 3), 26); // (2 & 7) | (3 << 3)
        assert_eq!(command_integer(CMD_CLOSE_PATH, 1), 15); // (7 & 7) | (1 << 3)
    }

    #[test]
    fn geo_to_mvt_projection() {
        let bbox = [144.0, -38.0, 145.0, -37.0];
        // Center of tile
        let (x, y) = geo_to_mvt(144.5, -37.5, &bbox, 4096);
        assert_eq!(x, 2048);
        assert_eq!(y, 2048);
        // Top-left
        let (x, y) = geo_to_mvt(144.0, -37.0, &bbox, 4096);
        assert_eq!(x, 0);
        assert_eq!(y, 0);
        // Bottom-right
        let (x, y) = geo_to_mvt(145.0, -38.0, &bbox, 4096);
        assert_eq!(x, 4096);
        assert_eq!(y, 4096);
    }

    #[test]
    fn encode_point_geometry() {
        let mut cursor = (0, 0);
        let cmds = encode_point_geom(25, 17, &mut cursor);
        assert_eq!(
            cmds,
            vec![command_integer(CMD_MOVE_TO, 1), zigzag(25), zigzag(17),]
        );
        assert_eq!(cursor, (25, 17));
    }

    #[test]
    fn encode_linestring_geometry() {
        let coords = vec![(0, 0), (10, 10), (20, 10)];
        let mut cursor = (0, 0);
        let cmds = encode_linestring_geom(&coords, &mut cursor);
        assert_eq!(cmds[0], command_integer(CMD_MOVE_TO, 1));
        assert_eq!(cmds[3], command_integer(CMD_LINE_TO, 2));
        assert_eq!(cursor, (20, 10));
    }

    #[test]
    fn encode_polygon_geometry() {
        let ring = vec![(0, 0), (10, 0), (10, 10), (0, 10), (0, 0)];
        let mut cursor = (0, 0);
        let cmds = encode_polygon_geom(&[ring], &mut cursor);
        // Should have MoveTo(1) + LineTo(3) + ClosePath(1)
        // MoveTo first point
        assert_eq!(cmds[0], command_integer(CMD_MOVE_TO, 1));
        // LineTo 3 remaining points (excluding closing point)
        assert_eq!(cmds[3], command_integer(CMD_LINE_TO, 3));
        // ClosePath
        assert_eq!(*cmds.last().unwrap(), command_integer(CMD_CLOSE_PATH, 1));
    }

    #[test]
    fn tag_table_interning() {
        let mut tags = TagTable::new();
        let ki1 = tags.intern_key("name");
        let ki2 = tags.intern_key("name");
        assert_eq!(ki1, ki2); // Same key → same index

        let ki3 = tags.intern_key("type");
        assert_ne!(ki1, ki3); // Different key → different index

        let vi1 = tags.intern_value(MvtValue::Str("road".into()));
        let vi2 = tags.intern_value(MvtValue::Str("road".into()));
        assert_eq!(vi1, vi2); // Same value → same index

        let vi3 = tags.intern_value(MvtValue::I64(42));
        assert_ne!(vi1, vi3);
    }

    #[test]
    fn features_to_mvt_point() {
        let features = vec![json!({
            "type": "Feature",
            "properties": {"name": "test", "value": 42},
            "geometry": {
                "type": "Point",
                "coordinates": [144.5, -37.5]
            }
        })];
        let bbox = [144.0, -38.0, 145.0, -37.0];
        let mvt = features_to_mvt("test_layer", &features, &bbox, &[]);
        assert!(!mvt.is_empty());
        // Should be valid protobuf: first byte is tag for layers field (3 << 3 | 2 = 26)
        assert_eq!(mvt[0], 26);
    }

    #[test]
    fn features_to_mvt_polygon() {
        let features = vec![json!({
            "type": "Feature",
            "properties": {"area": "park"},
            "geometry": {
                "type": "Polygon",
                "coordinates": [[[144.0, -37.0], [145.0, -37.0], [145.0, -38.0], [144.0, -38.0], [144.0, -37.0]]]
            }
        })];
        let bbox = [144.0, -38.0, 145.0, -37.0];
        let mvt = features_to_mvt("polygons", &features, &bbox, &[]);
        assert!(!mvt.is_empty());
    }

    #[test]
    fn features_to_mvt_empty() {
        let features: Vec<serde_json::Value> = vec![];
        let bbox = [0.0, 0.0, 1.0, 1.0];
        let mvt = features_to_mvt("empty", &features, &bbox, &[]);
        // Should still produce a valid tile with an empty layer
        assert!(!mvt.is_empty());
    }

    #[test]
    fn features_to_mvt_mixed_types() {
        let features = vec![
            json!({
                "type": "Feature",
                "properties": {"kind": "point"},
                "geometry": {"type": "Point", "coordinates": [0.5, 0.5]}
            }),
            json!({
                "type": "Feature",
                "properties": {"kind": "line"},
                "geometry": {"type": "LineString", "coordinates": [[0.0, 0.0], [1.0, 1.0]]}
            }),
        ];
        let bbox = [0.0, 0.0, 1.0, 1.0];
        let mvt = features_to_mvt("mixed", &features, &bbox, &[]);
        assert!(!mvt.is_empty());
    }

    #[test]
    fn gzip_roundtrip() {
        let features = vec![json!({
            "type": "Feature",
            "properties": {"x": 1},
            "geometry": {"type": "Point", "coordinates": [0.5, 0.5]}
        })];
        let bbox = [0.0, 0.0, 1.0, 1.0];
        let gz = features_to_mvt_gz("gz_test", &features, &bbox, &[]);
        assert!(!gz.is_empty());
        // Gzip magic bytes
        assert_eq!(gz[0], 0x1f);
        assert_eq!(gz[1], 0x8b);
        // Decompress
        use std::io::Read;
        let mut decoder = flate2::read::GzDecoder::new(&gz[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).unwrap();
        assert!(!decompressed.is_empty());
        // First byte of decompressed MVT should be layer tag
        assert_eq!(decompressed[0], 26);
    }

    #[test]
    fn property_filtering() {
        let features = vec![json!({
            "type": "Feature",
            "properties": {"name": "foo", "secret": "bar", "value": 1},
            "geometry": {"type": "Point", "coordinates": [0.5, 0.5]}
        })];
        let bbox = [0.0, 0.0, 1.0, 1.0];

        // With allowed_properties filter
        let mvt_filtered = features_to_mvt("filtered", &features, &bbox, &["name".to_string()]);
        // Without filter (all properties)
        let mvt_all = features_to_mvt("all", &features, &bbox, &[]);

        // Filtered should be smaller (fewer properties encoded)
        assert!(mvt_filtered.len() < mvt_all.len());
    }

    #[test]
    fn wkb_point_to_mvt() {
        // WKB Point: LE, type=1, x=0.5, y=0.5
        let mut wkb = vec![1u8]; // little-endian
        wkb.extend_from_slice(&1u32.to_le_bytes()); // type = Point
        wkb.extend_from_slice(&0.5f64.to_le_bytes()); // x
        wkb.extend_from_slice(&0.5f64.to_le_bytes()); // y

        let bbox = [0.0, 0.0, 1.0, 1.0];
        let mut tags = TagTable::new();
        let mut cursor = (0, 0);
        let mut fid = 1u64;
        let props = vec![("name", MvtValue::Str("test".into()))];
        let result =
            encode_wkb_feature(&wkb, &props, &bbox, 4096, &mut tags, &mut cursor, &mut fid);
        assert!(result.is_some());
    }

    #[test]
    fn encode_multipoint_geometry() {
        let points = vec![(10, 20), (30, 40)];
        let mut cursor = (0, 0);
        let cmds = encode_multi_point_geom(&points, &mut cursor);
        // Should be: MoveTo(2), dx1, dy1, dx2, dy2
        assert_eq!(cmds[0], command_integer(CMD_MOVE_TO, 2));
        assert_eq!(cmds.len(), 5); // 1 command + 2*2 params
    }
}

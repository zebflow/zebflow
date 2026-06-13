//! Tile renderer: rasterize GeoJSON features onto a PNG image.
//!
//! Uses `tiny-skia` to draw polygons, lines, and points onto a transparent
//! pixmap. The output is a PNG byte buffer ready for HTTP response.

use serde_json::Value;
use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform,
};

use super::style::LayerStyle;

// ── Z/X/Y slippy map tile → bbox conversion ──────────────────────────

/// Convert z/x/y slippy map tile coordinates to a WGS84 bounding box.
///
/// Returns `[min_lon, min_lat, max_lon, max_lat]` (EPSG:4326).
/// Delegates to geonative_tile for the standard Web Mercator tile math.
pub fn tile_to_bbox(z: u8, x: u32, y: u32) -> [f64; 4] {
    geonative_tile::TileCoord::new(z, x, y).bbox()
}

/// Request parameters for tile rendering.
#[derive(Debug, Clone)]
pub struct TileRenderRequest {
    /// Bounding box: [min_x, min_y, max_x, max_y] (EPSG:4326 lon/lat).
    pub bbox: [f64; 4],
    /// Tile width in pixels (default 256, max 1024).
    pub width: u32,
    /// Tile height in pixels (default 256, max 1024).
    pub height: u32,
    /// Optional zoom level for style adjustments.
    pub zoom: Option<u8>,
}

/// Render GeoJSON features to a PNG byte buffer.
///
/// Features are projected from lon/lat (EPSG:4326) to pixel coordinates
/// within the tile's bounding box, then rasterized with tiny-skia.
pub fn render_features_to_png(
    features: &[Value],
    request: &TileRenderRequest,
    style: &LayerStyle,
    resolved: Option<&super::style_dsl::ResolvedStyle>,
) -> Result<Vec<u8>, String> {
    let pixmap = render_features_to_pixmap(features, request, style, resolved)?;
    encode_pixmap_to_png(&pixmap)
}

/// Render GeoJSON features to a Pixmap (no PNG encoding).
pub fn render_features_to_pixmap(
    features: &[Value],
    request: &TileRenderRequest,
    style: &LayerStyle,
    resolved: Option<&super::style_dsl::ResolvedStyle>,
) -> Result<Pixmap, String> {
    let w = request.width.min(4096).max(1);
    let h = request.height.min(4096).max(1);

    let mut pixmap = Pixmap::new(w, h).ok_or_else(|| "failed to create pixmap".to_string())?;

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
    let style_field = if use_per_feature {
        resolved.and_then(|r| r.field_name())
    } else {
        None
    };

    for feature in features {
        let geom = &feature["geometry"];
        let geom_type = geom["type"].as_str().unwrap_or("");

        // Compute per-feature paints if data-driven styling is active
        let feat_paints: Option<(Option<Paint>, Paint, Stroke, Paint, f32)> =
            if let (Some(fname), Some(r)) = (style_field, resolved) {
                let sv = extract_geojson_style_value(feature, fname);
                let fs = r.style_for_value(&sv);
                Some(feature_style_paints(fs))
            } else {
                None
            };

        let (fp, sp, sd, pp, pr) = if let Some(ref fp) = feat_paints {
            (fp.0.as_ref(), &fp.1, &fp.2, &fp.3, fp.4)
        } else {
            (
                fill_paint.as_ref(),
                &stroke_paint,
                &stroke_def,
                &point_paint,
                style.point_radius,
            )
        };

        match geom_type {
            "Polygon" => {
                if let Some(coords) = geom["coordinates"].as_array() {
                    render_polygon(&mut pixmap, coords, request, fp, sp, sd, transform);
                }
            }
            "MultiPolygon" => {
                if let Some(polygons) = geom["coordinates"].as_array() {
                    for poly_coords in polygons {
                        if let Some(rings) = poly_coords.as_array() {
                            render_polygon(&mut pixmap, rings, request, fp, sp, sd, transform);
                        }
                    }
                }
            }
            "LineString" => {
                if let Some(coords) = geom["coordinates"].as_array() {
                    render_linestring(&mut pixmap, coords, request, sp, sd, transform);
                }
            }
            "MultiLineString" => {
                if let Some(lines) = geom["coordinates"].as_array() {
                    for line in lines {
                        if let Some(coords) = line.as_array() {
                            render_linestring(&mut pixmap, coords, request, sp, sd, transform);
                        }
                    }
                }
            }
            "Point" => {
                if let Some(coords) = geom["coordinates"].as_array() {
                    render_point(&mut pixmap, coords, request, pp, pr, transform);
                }
            }
            "MultiPoint" => {
                if let Some(points) = geom["coordinates"].as_array() {
                    for pt in points {
                        if let Some(coords) = pt.as_array() {
                            render_point(&mut pixmap, coords, request, pp, pr, transform);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(pixmap)
}

/// Extract a StyleValue from a GeoJSON feature's properties.
fn extract_geojson_style_value(feature: &Value, field: &str) -> super::style_dsl::StyleValue {
    use super::style_dsl::StyleValue;
    let val = &feature["properties"][field];
    match val {
        Value::String(s) => StyleValue::Str(s.clone()),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                StyleValue::I64(i)
            } else if let Some(f) = n.as_f64() {
                StyleValue::F64(f)
            } else {
                StyleValue::Null
            }
        }
        _ => StyleValue::Null,
    }
}

/// Build Paint objects from a FeatureStyle for per-feature rendering.
fn feature_style_paints(
    fs: super::style_dsl::FeatureStyle,
) -> (
    Option<Paint<'static>>,
    Paint<'static>,
    Stroke,
    Paint<'static>,
    f32,
) {
    let fill = fs.fill_color.map(|c| {
        let mut p = Paint::default();
        p.set_color(Color::from_rgba8(c[0], c[1], c[2], c[3]));
        p.anti_alias = true;
        p
    });
    let mut stroke_p = Paint::default();
    stroke_p.set_color(Color::from_rgba8(
        fs.stroke_color[0],
        fs.stroke_color[1],
        fs.stroke_color[2],
        fs.stroke_color[3],
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
        fs.point_color[0],
        fs.point_color[1],
        fs.point_color[2],
        fs.point_color[3],
    ));
    point_p.anti_alias = true;
    (fill, stroke_p, stroke_d, point_p, fs.point_radius)
}

/// Encode a Pixmap to PNG bytes.
pub fn encode_pixmap_to_png(pixmap: &Pixmap) -> Result<Vec<u8>, String> {
    pixmap
        .encode_png()
        .map_err(|e| format!("png encode failed: {e}"))
}

/// Information about a metatile grid.
#[derive(Debug, Clone)]
pub struct MetatileInfo {
    /// Metatile bounding box [min_x, min_y, max_x, max_y].
    pub bbox: [f64; 4],
    /// Width of one sub-tile in degrees.
    pub tile_w: f64,
    /// Height of one sub-tile in degrees.
    pub tile_h: f64,
    /// Grid size (4 for 4×4 metatile).
    pub grid_size: u32,
}

impl MetatileInfo {
    /// Compute the bbox of a sub-tile at (col, row) within the metatile.
    pub fn sub_tile_bbox(&self, col: u32, row: u32) -> [f64; 4] {
        let x0 = self.bbox[0] + col as f64 * self.tile_w;
        let y0 = self.bbox[3] - (row + 1) as f64 * self.tile_h; // y is flipped: row 0 = top
        [x0, y0, x0 + self.tile_w, y0 + self.tile_h]
    }

    /// Find which (col, row) a tile bbox belongs to within this metatile.
    pub fn tile_position(&self, tile_bbox: &[f64; 4]) -> (u32, u32) {
        let col = ((tile_bbox[0] - self.bbox[0]) / self.tile_w).round() as u32;
        // row 0 = top of metatile (max_y), row increases downward
        let row = ((self.bbox[3] - tile_bbox[3]) / self.tile_h).round() as u32;
        (col.min(self.grid_size - 1), row.min(self.grid_size - 1))
    }
}

/// Compute metatile info from a single tile's bbox.
///
/// Groups tiles into a deterministic 4×4 grid. All tiles that belong to the
/// same metatile will produce the same MetatileInfo.
pub fn compute_metatile_info(tile_bbox: &[f64; 4], grid_size: u32) -> MetatileInfo {
    let tile_w = tile_bbox[2] - tile_bbox[0];
    let tile_h = tile_bbox[3] - tile_bbox[1];
    let meta_w = tile_w * grid_size as f64;
    let meta_h = tile_h * grid_size as f64;
    let meta_x0 = (tile_bbox[0] / meta_w).floor() * meta_w;
    let meta_y0 = (tile_bbox[1] / meta_h).floor() * meta_h;
    MetatileInfo {
        bbox: [meta_x0, meta_y0, meta_x0 + meta_w, meta_y0 + meta_h],
        tile_w,
        tile_h,
        grid_size,
    }
}

/// Slice a metatile pixmap into grid_size × grid_size individual tile PNGs.
///
/// Returns Vec of (col, row, png_bytes) for each sub-tile.
pub fn slice_metatile(pixmap: &Pixmap, grid_size: u32) -> Result<Vec<(u32, u32, Vec<u8>)>, String> {
    let tile_w = pixmap.width() / grid_size;
    let tile_h = pixmap.height() / grid_size;
    if tile_w == 0 || tile_h == 0 {
        return Err("metatile too small to slice".to_string());
    }

    let src_data = pixmap.data();
    let src_stride = pixmap.width() as usize * 4; // 4 bytes per pixel (RGBA)
    let mut tiles = Vec::with_capacity((grid_size * grid_size) as usize);

    for row in 0..grid_size {
        for col in 0..grid_size {
            let mut sub = Pixmap::new(tile_w, tile_h)
                .ok_or_else(|| "failed to create sub-tile pixmap".to_string())?;
            let dst_data = sub.data_mut();
            let dst_stride = tile_w as usize * 4;

            let src_x = col as usize * tile_w as usize;
            let src_y = row as usize * tile_h as usize;

            for y in 0..tile_h as usize {
                let src_off = (src_y + y) * src_stride + src_x * 4;
                let dst_off = y * dst_stride;
                dst_data[dst_off..dst_off + dst_stride]
                    .copy_from_slice(&src_data[src_off..src_off + dst_stride]);
            }

            let png = sub
                .encode_png()
                .map_err(|e| format!("sub-tile png encode failed: {e}"))?;
            tiles.push((col, row, png));
        }
    }

    Ok(tiles)
}

/// Convert a geographic coordinate (lon, lat) to pixel position within the tile.
#[inline]
fn geo_to_pixel(lon: f64, lat: f64, request: &TileRenderRequest) -> (f32, f32) {
    let [min_x, min_y, max_x, max_y] = request.bbox;
    let dx = max_x - min_x;
    let dy = max_y - min_y;

    // Guard against zero-extent bbox
    let px = if dx.abs() < 1e-12 {
        request.width as f32 / 2.0
    } else {
        ((lon - min_x) / dx * request.width as f64) as f32
    };

    // Y is flipped: higher lat = lower pixel y
    let py = if dy.abs() < 1e-12 {
        request.height as f32 / 2.0
    } else {
        ((max_y - lat) / dy * request.height as f64) as f32
    };

    (px, py)
}

/// Extract (lon, lat) from a GeoJSON coordinate array [lon, lat, ...].
#[inline]
fn coord_pair(coord: &Value) -> Option<(f64, f64)> {
    let arr = coord.as_array()?;
    if arr.len() < 2 {
        return None;
    }
    Some((arr[0].as_f64()?, arr[1].as_f64()?))
}

/// Render a single polygon (array of rings, first is exterior).
fn render_polygon(
    pixmap: &mut Pixmap,
    rings: &[Value],
    request: &TileRenderRequest,
    fill_paint: Option<&Paint>,
    stroke_paint: &Paint,
    stroke_def: &Stroke,
    transform: Transform,
) {
    if rings.is_empty() {
        return;
    }

    // Build path from all rings
    let mut pb = PathBuilder::new();
    for ring_val in rings {
        let Some(ring) = ring_val.as_array() else {
            continue;
        };
        if ring.len() < 3 {
            continue;
        }

        let mut started = false;
        for coord in ring {
            let Some((lon, lat)) = coord_pair(coord) else {
                continue;
            };
            let (px, py) = geo_to_pixel(lon, lat, request);
            if !started {
                pb.move_to(px, py);
                started = true;
            } else {
                pb.line_to(px, py);
            }
        }
        if started {
            pb.close();
        }
    }

    let Some(path) = pb.finish() else {
        return;
    };

    // Fill then stroke
    if let Some(fp) = fill_paint {
        pixmap.fill_path(&path, fp, FillRule::EvenOdd, transform, None);
    }
    if stroke_def.width > 0.0 {
        pixmap.stroke_path(&path, stroke_paint, stroke_def, transform, None);
    }
}

/// Render a linestring (array of coordinate pairs).
fn render_linestring(
    pixmap: &mut Pixmap,
    coords: &[Value],
    request: &TileRenderRequest,
    stroke_paint: &Paint,
    stroke_def: &Stroke,
    transform: Transform,
) {
    if coords.len() < 2 {
        return;
    }

    let mut pb = PathBuilder::new();
    let mut started = false;
    for coord in coords {
        let Some((lon, lat)) = coord_pair(coord) else {
            continue;
        };
        let (px, py) = geo_to_pixel(lon, lat, request);
        if !started {
            pb.move_to(px, py);
            started = true;
        } else {
            pb.line_to(px, py);
        }
    }

    if let Some(path) = pb.finish() {
        pixmap.stroke_path(&path, stroke_paint, stroke_def, transform, None);
    }
}

/// Render a single point as a filled circle.
fn render_point(
    pixmap: &mut Pixmap,
    coords: &[Value],
    request: &TileRenderRequest,
    paint: &Paint,
    radius: f32,
    transform: Transform,
) {
    let Some((lon, lat)) = coord_pair(&Value::Array(coords.to_vec())) else {
        return;
    };
    let (cx, cy) = geo_to_pixel(lon, lat, request);

    // Skip points clearly outside the tile (with some margin)
    let margin = radius * 2.0;
    if cx < -margin
        || cy < -margin
        || cx > request.width as f32 + margin
        || cy > request.height as f32 + margin
    {
        return;
    }

    let rect = tiny_skia::Rect::from_xywh(cx - radius, cy - radius, radius * 2.0, radius * 2.0);
    let Some(rect) = rect else {
        return;
    };
    let Some(path) = PathBuilder::from_oval(rect) else {
        return;
    };
    pixmap.fill_path(&path, paint, FillRule::Winding, transform, None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_request() -> TileRenderRequest {
        TileRenderRequest {
            bbox: [144.0, -38.0, 145.0, -37.0],
            width: 256,
            height: 256,
            zoom: Some(10),
        }
    }

    #[test]
    fn geo_to_pixel_center() {
        let req = test_request();
        let (px, py) = geo_to_pixel(144.5, -37.5, &req);
        assert!((px - 128.0).abs() < 1.0);
        assert!((py - 128.0).abs() < 1.0);
    }

    #[test]
    fn geo_to_pixel_corners() {
        let req = test_request();
        // Top-left: min_x, max_y → pixel (0, 0)
        let (px, py) = geo_to_pixel(144.0, -37.0, &req);
        assert!(px.abs() < 0.1);
        assert!(py.abs() < 0.1);

        // Bottom-right: max_x, min_y → pixel (256, 256)
        let (px, py) = geo_to_pixel(145.0, -38.0, &req);
        assert!((px - 256.0).abs() < 0.1);
        assert!((py - 256.0).abs() < 0.1);
    }

    #[test]
    fn render_empty_features() {
        let req = test_request();
        let style = LayerStyle::default();
        let result = render_features_to_png(&[], &req, &style, None);
        assert!(result.is_ok());
        let png = result.unwrap();
        // PNG magic bytes
        assert_eq!(&png[..4], &[0x89, b'P', b'N', b'G']);
    }

    #[test]
    fn render_polygon_feature() {
        let features = vec![json!({
            "type": "Feature",
            "properties": {},
            "geometry": {
                "type": "Polygon",
                "coordinates": [[
                    [144.2, -37.8],
                    [144.8, -37.8],
                    [144.8, -37.2],
                    [144.2, -37.2],
                    [144.2, -37.8]
                ]]
            }
        })];
        let req = test_request();
        let style = LayerStyle::default();
        let result = render_features_to_png(&features, &req, &style, None);
        assert!(result.is_ok());
        let png = result.unwrap();
        // Non-trivial PNG (has actual content, should be larger than empty)
        assert!(png.len() > 100);
    }

    #[test]
    fn render_point_feature() {
        let features = vec![json!({
            "type": "Feature",
            "properties": {},
            "geometry": {
                "type": "Point",
                "coordinates": [144.5, -37.5]
            }
        })];
        let req = test_request();
        let style = LayerStyle::default();
        let result = render_features_to_png(&features, &req, &style, None);
        assert!(result.is_ok());
    }

    #[test]
    fn render_linestring_feature() {
        let features = vec![json!({
            "type": "Feature",
            "properties": {},
            "geometry": {
                "type": "LineString",
                "coordinates": [
                    [144.2, -37.8],
                    [144.5, -37.5],
                    [144.8, -37.2]
                ]
            }
        })];
        let req = test_request();
        let style = LayerStyle::default();
        let result = render_features_to_png(&features, &req, &style, None);
        assert!(result.is_ok());
    }

    #[test]
    fn render_multipolygon_feature() {
        let features = vec![json!({
            "type": "Feature",
            "properties": {},
            "geometry": {
                "type": "MultiPolygon",
                "coordinates": [
                    [[
                        [144.1, -37.9],
                        [144.4, -37.9],
                        [144.4, -37.6],
                        [144.1, -37.6],
                        [144.1, -37.9]
                    ]],
                    [[
                        [144.6, -37.4],
                        [144.9, -37.4],
                        [144.9, -37.1],
                        [144.6, -37.1],
                        [144.6, -37.4]
                    ]]
                ]
            }
        })];
        let req = test_request();
        let style = LayerStyle::default();
        let result = render_features_to_png(&features, &req, &style, None);
        assert!(result.is_ok());
    }

    #[test]
    fn render_mixed_geometry_types() {
        let features = vec![
            json!({
                "type": "Feature",
                "properties": {},
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[
                        [144.2, -37.8],
                        [144.4, -37.8],
                        [144.4, -37.6],
                        [144.2, -37.6],
                        [144.2, -37.8]
                    ]]
                }
            }),
            json!({
                "type": "Feature",
                "properties": {},
                "geometry": {
                    "type": "Point",
                    "coordinates": [144.5, -37.5]
                }
            }),
            json!({
                "type": "Feature",
                "properties": {},
                "geometry": {
                    "type": "LineString",
                    "coordinates": [
                        [144.6, -37.4],
                        [144.8, -37.2]
                    ]
                }
            }),
        ];
        let req = test_request();
        let style = LayerStyle::default();
        let result = render_features_to_png(&features, &req, &style, None);
        assert!(result.is_ok());
        let png = result.unwrap();
        assert!(png.len() > 100);
    }

    #[test]
    fn clamped_tile_dimensions() {
        let req = TileRenderRequest {
            bbox: [144.0, -38.0, 145.0, -37.0],
            width: 2048, // over max
            height: 0,   // under min
            zoom: None,
        };
        let style = LayerStyle::default();
        let result = render_features_to_png(&[], &req, &style, None);
        assert!(result.is_ok());
    }

    #[test]
    fn render_features_to_pixmap_valid() {
        let req = test_request();
        let style = LayerStyle::default();
        let pixmap = render_features_to_pixmap(&[], &req, &style, None).unwrap();
        assert_eq!(pixmap.width(), 256);
        assert_eq!(pixmap.height(), 256);
    }

    #[test]
    fn encode_pixmap_roundtrip() {
        let pixmap = Pixmap::new(64, 64).unwrap();
        let png = encode_pixmap_to_png(&pixmap).unwrap();
        assert_eq!(&png[..4], &[0x89, b'P', b'N', b'G']);
    }

    #[test]
    fn slice_metatile_produces_correct_count() {
        let pixmap = Pixmap::new(1024, 1024).unwrap();
        let tiles = slice_metatile(&pixmap, 4).unwrap();
        assert_eq!(tiles.len(), 16);
        for (col, row, png) in &tiles {
            assert!(*col < 4);
            assert!(*row < 4);
            assert_eq!(&png[..4], &[0x89, b'P', b'N', b'G']);
        }
    }

    #[test]
    fn slice_metatile_tile_dimensions() {
        // Render a colored pixmap to check slicing
        let mut pixmap = Pixmap::new(512, 512).unwrap();
        // Fill with some color so it's not empty
        pixmap.fill(Color::from_rgba8(255, 0, 0, 255));
        let tiles = slice_metatile(&pixmap, 2).unwrap();
        assert_eq!(tiles.len(), 4);
        // Each sub-tile should be 256x256 and have non-trivial size
        for (_, _, png) in &tiles {
            assert!(png.len() > 100);
        }
    }

    #[test]
    fn compute_metatile_info_deterministic() {
        // Two tiles in the same 4x4 grid should produce the same metatile
        let tile_a = [144.0, -38.0, 144.25, -37.75]; // col=0
        let tile_b = [144.25, -38.0, 144.5, -37.75]; // col=1
        let meta_a = compute_metatile_info(&tile_a, 4);
        let meta_b = compute_metatile_info(&tile_b, 4);
        assert_eq!(meta_a.bbox, meta_b.bbox);
    }

    #[test]
    fn compute_metatile_info_sub_tile_bbox() {
        let tile = [144.0, -38.0, 144.25, -37.75];
        let meta = compute_metatile_info(&tile, 4);
        // The original tile should be at some position in the metatile
        let (col, row) = meta.tile_position(&tile);
        let recovered = meta.sub_tile_bbox(col, row);
        assert!((recovered[0] - tile[0]).abs() < 1e-10);
        assert!((recovered[1] - tile[1]).abs() < 1e-10);
        assert!((recovered[2] - tile[2]).abs() < 1e-10);
        assert!((recovered[3] - tile[3]).abs() < 1e-10);
    }

    #[test]
    fn metatile_tile_position_all_positions() {
        let tile_w = 0.25;
        let tile_h = 0.25;
        // Create a metatile from the tile at (0,0)
        let first_tile = [144.0, -38.0, 144.0 + tile_w, -38.0 + tile_h];
        let meta = compute_metatile_info(&first_tile, 4);

        // Check all 16 positions produce unique (col, row) pairs
        let mut positions = std::collections::HashSet::new();
        for row in 0..4u32 {
            for col in 0..4u32 {
                let sub = meta.sub_tile_bbox(col, row);
                let (rc, rr) = meta.tile_position(&sub);
                assert_eq!(
                    (rc, rr),
                    (col, row),
                    "position mismatch for col={col} row={row}"
                );
                positions.insert((rc, rr));
            }
        }
        assert_eq!(positions.len(), 16);
    }

    // ── tile_to_bbox tests ──────────────────────────────────────────────

    #[test]
    fn tile_to_bbox_z0_covers_world() {
        let bbox = tile_to_bbox(0, 0, 0);
        assert!((bbox[0] - (-180.0)).abs() < 1e-6);
        assert!((bbox[2] - 180.0).abs() < 1e-6);
        assert!(bbox[3] > 85.0); // Mercator max lat ~85.05
        assert!(bbox[1] < -85.0);
    }

    #[test]
    fn tile_to_bbox_z1_quadrants() {
        // z=1 splits into 4 tiles
        let nw = tile_to_bbox(1, 0, 0);
        let ne = tile_to_bbox(1, 1, 0);
        assert!((nw[0] - (-180.0)).abs() < 1e-6);
        assert!((nw[2] - 0.0).abs() < 1e-6);
        assert!((ne[0] - 0.0).abs() < 1e-6);
        assert!((ne[2] - 180.0).abs() < 1e-6);
    }

    #[test]
    fn tile_to_bbox_z10_jakarta() {
        // z=10, x=815, y=529 should be near Jakarta (106.8, -6.2)
        let bbox = tile_to_bbox(10, 815, 529);
        assert!(bbox[0] > 106.0 && bbox[0] < 107.0);
        assert!(bbox[1] > -7.0 && bbox[1] < -6.0);
    }
}

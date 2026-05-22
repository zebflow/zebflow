//! Geometry simplification for mapserver responses.
//!
//! Applies Douglas-Peucker simplification to reduce coordinate density
//! at low zoom levels. This cuts response size for deck.gl and other
//! clients without visually affecting the map at the given zoom.

use serde_json::Value;

/// Zoom-dependent tolerance in degrees for Douglas-Peucker simplification.
/// Returns `None` at high zoom levels where simplification is unnecessary.
fn tolerance_for_zoom(zoom: Option<u8>) -> Option<f64> {
    match zoom {
        Some(z) if z <= 4 => Some(0.05),
        Some(z) if z <= 6 => Some(0.01),
        Some(z) if z <= 8 => Some(0.005),
        Some(z) if z <= 10 => Some(0.001),
        Some(z) if z <= 12 => Some(0.0005),
        _ => None, // zoom > 12 or unset: no simplification
    }
}

/// Simplify all features in place. Modifies geometry coordinates
/// for Polygon, MultiPolygon, LineString, and MultiLineString types.
/// Points are left unchanged.
pub fn simplify_features(features: &mut [Value], zoom: Option<u8>) {
    let Some(tolerance) = tolerance_for_zoom(zoom) else {
        return;
    };
    for feature in features.iter_mut() {
        if let Some(geometry) = feature.get_mut("geometry") {
            simplify_geometry(geometry, tolerance);
        }
    }
}

fn simplify_geometry(geometry: &mut Value, tolerance: f64) {
    let geom_type = geometry
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    match geom_type.as_str() {
        "LineString" => {
            if let Some(coords) = geometry.get("coordinates").and_then(Value::as_array).cloned() {
                let simplified = douglas_peucker(&coords, tolerance);
                geometry["coordinates"] = Value::Array(simplified);
            }
        }
        "MultiLineString" => {
            if let Some(lines) = geometry.get("coordinates").and_then(Value::as_array).cloned() {
                let simplified: Vec<Value> = lines
                    .iter()
                    .map(|line| {
                        if let Some(coords) = line.as_array() {
                            Value::Array(douglas_peucker(coords, tolerance))
                        } else {
                            line.clone()
                        }
                    })
                    .collect();
                geometry["coordinates"] = Value::Array(simplified);
            }
        }
        "Polygon" => {
            if let Some(rings) = geometry.get("coordinates").and_then(Value::as_array).cloned() {
                let simplified: Vec<Value> = rings
                    .iter()
                    .map(|ring| {
                        if let Some(coords) = ring.as_array() {
                            Value::Array(douglas_peucker_ring(coords, tolerance))
                        } else {
                            ring.clone()
                        }
                    })
                    .collect();
                geometry["coordinates"] = Value::Array(simplified);
            }
        }
        "MultiPolygon" => {
            if let Some(polys) = geometry.get("coordinates").and_then(Value::as_array).cloned() {
                let simplified: Vec<Value> = polys
                    .iter()
                    .map(|poly| {
                        if let Some(rings) = poly.as_array() {
                            let simplified_rings: Vec<Value> = rings
                                .iter()
                                .map(|ring| {
                                    if let Some(coords) = ring.as_array() {
                                        Value::Array(douglas_peucker_ring(coords, tolerance))
                                    } else {
                                        ring.clone()
                                    }
                                })
                                .collect();
                            Value::Array(simplified_rings)
                        } else {
                            poly.clone()
                        }
                    })
                    .collect();
                geometry["coordinates"] = Value::Array(simplified);
            }
        }
        // Point, MultiPoint — no simplification needed
        _ => {}
    }
}

/// Douglas-Peucker for a ring (closed polygon ring).
/// Ensures at least 4 points are preserved (minimum valid ring).
fn douglas_peucker_ring(coords: &[Value], tolerance: f64) -> Vec<Value> {
    if coords.len() <= 4 {
        return coords.to_vec();
    }
    let mut result = douglas_peucker(coords, tolerance);
    // Ensure ring closure and minimum 4 points
    if result.len() < 4 {
        return coords.to_vec();
    }
    // Ensure the ring is closed
    if let (Some(first), Some(last)) = (result.first().cloned(), result.last().cloned()) {
        if first != last {
            result.push(first);
        }
    }
    result
}

/// Douglas-Peucker line simplification.
fn douglas_peucker(coords: &[Value], tolerance: f64) -> Vec<Value> {
    if coords.len() <= 2 {
        return coords.to_vec();
    }

    // Find the point farthest from the line between first and last
    let (first_x, first_y) = coord_xy(&coords[0]);
    let (last_x, last_y) = coord_xy(coords.last().unwrap());

    let mut max_dist = 0.0f64;
    let mut max_idx = 0usize;

    for (i, coord) in coords.iter().enumerate().skip(1).take(coords.len() - 2) {
        let (px, py) = coord_xy(coord);
        let dist = perpendicular_distance(px, py, first_x, first_y, last_x, last_y);
        if dist > max_dist {
            max_dist = dist;
            max_idx = i;
        }
    }

    if max_dist > tolerance {
        // Recurse on both halves
        let mut left = douglas_peucker(&coords[..=max_idx], tolerance);
        let right = douglas_peucker(&coords[max_idx..], tolerance);
        // Remove duplicate point at the join
        left.pop();
        left.extend(right);
        left
    } else {
        // All points within tolerance — keep only endpoints
        vec![coords[0].clone(), coords.last().unwrap().clone()]
    }
}

fn coord_xy(coord: &Value) -> (f64, f64) {
    let arr = coord.as_array();
    let x = arr
        .and_then(|a| a.first())
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let y = arr
        .and_then(|a| a.get(1))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    (x, y)
}

fn perpendicular_distance(
    px: f64,
    py: f64,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-20 {
        // Line segment is essentially a point
        let ddx = px - x1;
        let ddy = py - y1;
        return (ddx * ddx + ddy * ddy).sqrt();
    }
    ((dy * px - dx * py + x2 * y1 - y2 * x1).abs()) / len_sq.sqrt()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn simplify_reduces_polygon_coordinates_at_low_zoom() {
        // A polygon with many points that should simplify at low zoom
        let coords: Vec<Value> = (0..100)
            .map(|i| {
                let angle = (i as f64) * std::f64::consts::TAU / 100.0;
                let r = 1.0 + (i as f64 * 0.001); // near-circle with tiny jitter
                json!([r * angle.cos(), r * angle.sin()])
            })
            .collect();
        let mut closed = coords.clone();
        closed.push(coords[0].clone());

        let mut features = vec![json!({
            "type": "Feature",
            "properties": {},
            "geometry": {
                "type": "Polygon",
                "coordinates": [closed]
            }
        })];

        let original_len = features[0]["geometry"]["coordinates"][0]
            .as_array()
            .unwrap()
            .len();

        simplify_features(&mut features, Some(4));

        let simplified_len = features[0]["geometry"]["coordinates"][0]
            .as_array()
            .unwrap()
            .len();

        assert!(
            simplified_len < original_len,
            "expected simplification: {simplified_len} should be < {original_len}"
        );
        // Ring must still be closed and have at least 4 points
        assert!(simplified_len >= 4);
    }

    #[test]
    fn simplify_preserves_points() {
        let mut features = vec![json!({
            "type": "Feature",
            "properties": {},
            "geometry": {
                "type": "Point",
                "coordinates": [106.8, -6.2]
            }
        })];

        let before = features[0].clone();
        simplify_features(&mut features, Some(4));
        assert_eq!(features[0], before);
    }

    #[test]
    fn simplify_skips_at_high_zoom() {
        let coords: Vec<Value> = (0..50)
            .map(|i| json!([i as f64 * 0.01, i as f64 * 0.01]))
            .collect();

        let mut features = vec![json!({
            "type": "Feature",
            "properties": {},
            "geometry": {
                "type": "LineString",
                "coordinates": coords
            }
        })];

        let before = features[0].clone();
        simplify_features(&mut features, Some(15));
        assert_eq!(features[0], before, "high zoom should not simplify");
    }

    #[test]
    fn simplify_no_zoom_skips() {
        let mut features = vec![json!({
            "type": "Feature",
            "properties": {},
            "geometry": {
                "type": "LineString",
                "coordinates": [[0, 0], [1, 1], [2, 0]]
            }
        })];

        let before = features[0].clone();
        simplify_features(&mut features, None);
        assert_eq!(features[0], before, "no zoom should not simplify");
    }

    #[test]
    fn douglas_peucker_reduces_collinear_points() {
        // Points roughly on a line — should simplify to endpoints
        let coords = vec![
            json!([0.0, 0.0]),
            json!([0.5, 0.0001]), // nearly collinear
            json!([1.0, 0.0]),
        ];
        let result = douglas_peucker(&coords, 0.001);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn douglas_peucker_keeps_significant_deviation() {
        let coords = vec![
            json!([0.0, 0.0]),
            json!([0.5, 1.0]), // significant deviation
            json!([1.0, 0.0]),
        ];
        let result = douglas_peucker(&coords, 0.001);
        assert_eq!(result.len(), 3);
    }
}

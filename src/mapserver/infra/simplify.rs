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

/// Simplify a GeoJSON geometry in place using geonative's Douglas-Peucker.
fn simplify_geometry(geometry: &mut Value, tolerance: f64) {
    // Parse GeoJSON geometry → IR, simplify, convert back
    let Ok(ir) = geonative_geojson::geometry::from_json(geometry) else {
        return;
    };
    let simplified = geonative_utils::simplify::simplify_geometry(&ir, tolerance);
    let result = geonative_geojson::geometry::to_json(&simplified);
    // Replace geometry fields in place
    if let Some(coords) = result.get("coordinates") {
        geometry["coordinates"] = coords.clone();
    }
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
    fn simplify_reduces_collinear_linestring() {
        // Points roughly on a line — should simplify to endpoints
        let mut features = vec![json!({
            "type": "Feature",
            "properties": {},
            "geometry": {
                "type": "LineString",
                "coordinates": [[0.0, 0.0], [0.5, 0.0001], [1.0, 0.0]]
            }
        })];
        simplify_features(&mut features, Some(4)); // tolerance 0.05
        let coords = features[0]["geometry"]["coordinates"].as_array().unwrap();
        assert_eq!(coords.len(), 2);
    }

    #[test]
    fn simplify_keeps_significant_deviation() {
        // Point with significant deviation — should be kept
        let mut features = vec![json!({
            "type": "Feature",
            "properties": {},
            "geometry": {
                "type": "LineString",
                "coordinates": [[0.0, 0.0], [0.5, 1.0], [1.0, 0.0]]
            }
        })];
        simplify_features(&mut features, Some(4)); // tolerance 0.05
        let coords = features[0]["geometry"]["coordinates"].as_array().unwrap();
        assert_eq!(coords.len(), 3);
    }
}

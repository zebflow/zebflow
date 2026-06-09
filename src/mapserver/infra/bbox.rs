use serde_json::Value;

pub fn normalize_bbox(mut bbox: [f64; 4]) -> Option<[f64; 4]> {
    if !bbox.iter().all(|v| v.is_finite()) {
        return None;
    }
    if bbox[0] > bbox[2] {
        bbox.swap(0, 2);
    }
    if bbox[1] > bbox[3] {
        bbox.swap(1, 3);
    }
    Some(bbox)
}

pub fn feature_intersects_bbox(feature: &Value, bbox: [f64; 4]) -> bool {
    let Some(geom) = feature.get("geometry") else {
        return false;
    };
    let Some(feature_bbox) = geometry_bbox(geom) else {
        return false;
    };
    !(feature_bbox[2] < bbox[0]
        || feature_bbox[0] > bbox[2]
        || feature_bbox[3] < bbox[1]
        || feature_bbox[1] > bbox[3])
}

pub fn geometry_bbox(geometry: &Value) -> Option<[f64; 4]> {
    let ir = geonative_geojson::geometry::from_json(geometry).ok()?;
    ir.bbox()
}

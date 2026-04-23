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
    let coords = geometry.get("coordinates")?;
    let mut state: Option<[f64; 4]> = None;
    visit_coords(coords, &mut |lon, lat| {
        state = Some(match state {
            Some([minx, miny, maxx, maxy]) => [
                minx.min(lon),
                miny.min(lat),
                maxx.max(lon),
                maxy.max(lat),
            ],
            None => [lon, lat, lon, lat],
        });
    });
    state
}

fn visit_coords(value: &Value, f: &mut impl FnMut(f64, f64)) {
    if let Some(arr) = value.as_array() {
        if arr.len() >= 2 && arr[0].is_number() && arr[1].is_number() {
            let lon = arr[0].as_f64().unwrap_or(0.0);
            let lat = arr[1].as_f64().unwrap_or(0.0);
            f(lon, lat);
            return;
        }
        for item in arr {
            visit_coords(item, f);
        }
    }
}

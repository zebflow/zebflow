use serde_json::Value;

use crate::mapserver::infra::bbox::{feature_intersects_bbox, normalize_bbox};
use crate::mapserver::infra::source::SourceAdapter;
use crate::mapserver::publish::manifest::PublishedLayerManifest;

use super::{ResolveRequest, ResolveResponse};

pub fn resolve_feature_collection_from_geojson_file(
    manifest: &PublishedLayerManifest,
    request: &ResolveRequest,
) -> Result<ResolveResponse, String> {
    let source_value = crate::mapserver::infra::source::geojson_file::GeoJsonFileSource::load(
        &manifest.source_ref,
    )?;
    resolve_feature_collection_from_value(manifest, request, source_value)
}

fn resolve_feature_collection_from_value(
    manifest: &PublishedLayerManifest,
    request: &ResolveRequest,
    source_value: Value,
) -> Result<ResolveResponse, String> {
    if manifest.bbox_required && request.bbox.is_none() {
        return Err("bbox is required for this layer".to_string());
    }
    let Some(obj) = source_value.as_object() else {
        return Err("source is not a GeoJSON object".to_string());
    };
    if obj.get("type").and_then(Value::as_str) != Some("FeatureCollection") {
        return Err("source is not a GeoJSON FeatureCollection".to_string());
    }
    let Some(features) = obj.get("features").and_then(Value::as_array) else {
        return Err("source GeoJSON missing features array".to_string());
    };
    let bbox = request.bbox.and_then(normalize_bbox);
    let hard_limit = request
        .limit
        .unwrap_or(manifest.max_features)
        .min(manifest.max_features);
    let mut out = Vec::new();
    let mut matched = 0usize;
    for feature in features {
        let intersects = bbox
            .map(|bbox| feature_intersects_bbox(feature, bbox))
            .unwrap_or(true);
        if !intersects {
            continue;
        }
        matched += 1;
        if out.len() < hard_limit {
            out.push(prune_feature_properties(
                feature.clone(),
                &manifest.allowed_properties,
            ));
        }
    }
    Ok(ResolveResponse {
        layer: manifest.layer_id.clone(),
        count: out.len(),
        truncated: matched > out.len(),
        features: out,
    })
}

fn prune_feature_properties(mut feature: Value, allowed: &[String]) -> Value {
    if allowed.is_empty() {
        return feature;
    }
    let Some(feature_obj) = feature.as_object_mut() else {
        return feature;
    };
    let Some(props) = feature_obj
        .get_mut("properties")
        .and_then(Value::as_object_mut)
    else {
        return feature;
    };
    props.retain(|key, _| allowed.iter().any(|candidate| candidate == key));
    feature
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::mapserver::publish::manifest::{PublishedLayerManifest, SourceKind};

    use super::*;

    #[test]
    fn feature_collection_bbox_filter_and_limit_work() {
        let manifest = PublishedLayerManifest {
            layer_id: "adm1".to_string(),
            path: "/adm1".to_string(),
            source_kind: SourceKind::GeoJsonFile,
            source_ref: "/tmp/adm1.geojson".to_string(),
            mode: "features".to_string(),
            min_zoom: None,
            max_zoom: None,
            bbox_required: true,
            max_features: 1,
            allowed_properties: vec!["name".to_string()],
        };
        let req = ResolveRequest {
            layer_id: "adm1".to_string(),
            bbox: Some([106.0, -7.0, 108.0, -5.0]),
            zoom: Some(6),
            limit: Some(10),
        };
        let source = json!({
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "properties": { "name": "A", "code": "1" },
                    "geometry": {
                        "type": "Polygon",
                        "coordinates": [[[106.0, -6.5], [107.0, -6.5], [107.0, -6.0], [106.0, -6.0], [106.0, -6.5]]]
                    }
                },
                {
                    "type": "Feature",
                    "properties": { "name": "B", "code": "2" },
                    "geometry": {
                        "type": "Polygon",
                        "coordinates": [[[106.5, -6.4], [107.5, -6.4], [107.5, -5.9], [106.5, -5.9], [106.5, -6.4]]]
                    }
                },
                {
                    "type": "Feature",
                    "properties": { "name": "Far", "code": "3" },
                    "geometry": {
                        "type": "Polygon",
                        "coordinates": [[[120.0, 0.0], [121.0, 0.0], [121.0, 1.0], [120.0, 1.0], [120.0, 0.0]]]
                    }
                }
            ]
        });
        let out = resolve_feature_collection_from_value(&manifest, &req, source).unwrap();
        assert_eq!(out.layer, "adm1");
        assert_eq!(out.count, 1);
        assert!(out.truncated);
        assert_eq!(
            out.features[0],
            json!({
                "type": "Feature",
                "properties": { "name": "A" },
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[106.0, -6.5], [107.0, -6.5], [107.0, -6.0], [106.0, -6.0], [106.0, -6.5]]]
                }
            })
        );
    }
}

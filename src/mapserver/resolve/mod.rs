use crate::mapserver::publish::manifest::{PublishedLayerManifest, SourceKind};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod artifact;
pub mod cache;
pub mod filter_dsl;
pub mod function_cache;
pub mod geoparquet;
pub mod geoparquet_direct;
pub mod geoparquet_optimize;
pub mod mvt;
pub mod query;
pub mod stats;
pub mod style;
pub mod style_dsl;
pub mod tile;
pub mod tile_cache;

/// Whether one source property may leave the server for this layer.
///
/// Contract `MapPublishManifest`, "What the public may see": `allowed_properties`
/// is closed by default. An empty or absent list means **geometry only, no
/// properties**, and there is deliberately no wildcard — a column added by a
/// later re-upload stays hidden until someone names it.
pub fn property_is_public(allowed: &[String], name: &str) -> bool {
    allowed.iter().any(|candidate| candidate == name)
}

/// Drops every property the layer's `allowed_properties` does not name.
///
/// Shared by every feature-serving path so one list governs all of them.
pub fn prune_feature_properties(mut feature: Value, allowed: &[String]) -> Value {
    let Some(feature_obj) = feature.as_object_mut() else {
        return feature;
    };
    let Some(props) = feature_obj
        .get_mut("properties")
        .and_then(Value::as_object_mut)
    else {
        return feature;
    };
    props.retain(|key, _| property_is_public(allowed, key));
    feature
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolveRequest {
    pub layer_id: String,
    pub bbox: Option<[f64; 4]>,
    pub zoom: Option<u8>,
    pub limit: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolveResponse {
    pub layer: String,
    pub count: usize,
    pub truncated: bool,
    pub features: Vec<Value>,
}

pub fn resolve_features(
    manifest: &PublishedLayerManifest,
    request: &ResolveRequest,
    preloaded: Option<&Value>,
) -> Result<ResolveResponse, String> {
    if let Some(zoom) = request.zoom {
        if let Some(min_zoom) = manifest.min_zoom {
            if zoom < min_zoom {
                return Ok(ResolveResponse {
                    layer: manifest.layer_id.clone(),
                    count: 0,
                    truncated: false,
                    features: Vec::new(),
                });
            }
        }
        if let Some(max_zoom) = manifest.max_zoom {
            if zoom > max_zoom {
                return Ok(ResolveResponse {
                    layer: manifest.layer_id.clone(),
                    count: 0,
                    truncated: false,
                    features: Vec::new(),
                });
            }
        }
    }
    let mut response = match manifest.source_kind {
        SourceKind::GeoJsonFile => {
            query::resolve_feature_collection_from_geojson_file(manifest, request)
        }
        SourceKind::GeoJsonArtifact => artifact::resolve_from_artifact(
            manifest,
            request,
            std::path::Path::new(&manifest.source_ref),
        ),
        SourceKind::GeoParquet => geoparquet::resolve_from_geoparquet(manifest, request),
        SourceKind::GeoJsonFunction => {
            let fc =
                preloaded.ok_or_else(|| "GeoJsonFunction: features not preloaded".to_string())?;
            query::resolve_feature_collection_from_value(manifest, request, fc.clone())
        }
    }?;

    // Post-process: simplify geometry at low zoom to reduce response size
    crate::mapserver::infra::simplify::simplify_features(&mut response.features, request.zoom);

    Ok(response)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn an_empty_allowed_list_is_geometry_only_not_a_wildcard() {
        // Contract `MapPublishManifest`, "What the public may see": `[]` or
        // absent means geometry only. There is no wildcard, deliberately, so a
        // column added by a later re-upload stays hidden until someone ticks it.
        let feature = json!({
            "type": "Feature",
            "properties": { "name": "A", "owner_phone": "+61400000000" },
            "geometry": { "type": "Point", "coordinates": [1.0, 2.0] }
        });

        let unconfigured = prune_feature_properties(feature.clone(), &[]);
        assert_eq!(unconfigured["properties"], json!({}));
        assert_eq!(unconfigured["geometry"], feature["geometry"]);

        let named = prune_feature_properties(feature.clone(), &["name".to_string()]);
        assert_eq!(named["properties"], json!({ "name": "A" }));

        assert!(!property_is_public(&[], "name"));
        assert!(property_is_public(&["name".to_string()], "name"));
        assert!(!property_is_public(&["name".to_string()], "owner_phone"));
    }
}

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

/// Returns whether `zoom` satisfies the manifest's `min_zoom` and `max_zoom` bounds.
pub fn zoom_within_bounds(manifest: &PublishedLayerManifest, zoom: Option<u8>) -> bool {
    if let Some(zoom) = zoom {
        if let Some(min_zoom) = manifest.min_zoom {
            if zoom < min_zoom {
                return false;
            }
        }
        if let Some(max_zoom) = manifest.max_zoom {
            if zoom > max_zoom {
                return false;
            }
        }
    }
    true
}

/// Validates a caller-supplied `?style=` DSL parameter against `manifest.allowed_properties`.
///
/// A caller may only colour or classify features by columns that are published
/// in `allowed_properties`; referencing a hidden column via `?style=` is refused
/// so pixel colours cannot be used as an oracle for private columns.
pub fn validate_caller_style_dsl(
    manifest: &PublishedLayerManifest,
    caller_style: Option<&str>,
) -> Result<(), String> {
    let Some(raw) = caller_style.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    let parsed = style_dsl::parse_style_dsl(raw).map_err(|e| format!("invalid style: {e}"))?;
    if let Some(field) = parsed.field_name()
        && !property_is_public(&manifest.allowed_properties, field)
    {
        return Err(format!(
            "style field '{field}' is not a published property of this layer"
        ));
    }
    Ok(())
}

pub fn resolve_features(
    manifest: &PublishedLayerManifest,
    request: &ResolveRequest,
    preloaded: Option<&Value>,
) -> Result<ResolveResponse, String> {
    if !zoom_within_bounds(manifest, request.zoom) {
        return Ok(ResolveResponse {
            layer: manifest.layer_id.clone(),
            count: 0,
            truncated: false,
            features: Vec::new(),
        });
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

    #[test]
    fn mapserver_rejects_unallowed_caller_style_field_enforces_zoom_bounds_and_keys_function_cache_by_request()
     {
        let manifest = PublishedLayerManifest {
            layer_id: "parcels".to_string(),
            path: "/parcels".to_string(),
            source_kind: SourceKind::GeoParquet,
            source_ref: "parcels.parquet".to_string(),
            mode: "features".to_string(),
            min_zoom: Some(10),
            max_zoom: Some(16),
            bbox_required: true,
            max_features: 100,
            allowed_properties: vec!["name".to_string()],
            style: None,
            filter: None,
            function_slug: None,
            cache_ttl_secs: None,
        };

        // 1. Caller ?style= referencing a hidden property is rejected, while allowed properties and uniform styles pass.
        let err = validate_caller_style_dsl(&manifest, Some("uv(secret_valuation)"))
            .expect_err("hidden style field must be rejected");
        assert!(err.contains("is not a published property"), "{err}");
        let err = validate_caller_style_dsl(&manifest, Some("cb(secret_valuation,5,YlOrRd)"))
            .expect_err("hidden class-breaks field must be rejected");
        assert!(err.contains("is not a published property"), "{err}");
        assert!(validate_caller_style_dsl(&manifest, Some("uv(name)")).is_ok());
        assert!(validate_caller_style_dsl(&manifest, Some("f:4169E180;s:1E3CA0")).is_ok());
        assert!(validate_caller_style_dsl(&manifest, None).is_ok());

        // 2. Zoom bounds check enforces min_zoom and max_zoom before direct rendering.
        assert!(!zoom_within_bounds(&manifest, Some(0)));
        assert!(!zoom_within_bounds(&manifest, Some(9)));
        assert!(zoom_within_bounds(&manifest, Some(10)));
        assert!(zoom_within_bounds(&manifest, Some(16)));
        assert!(!zoom_within_bounds(&manifest, Some(17)));

        // 3. Function cache keys vary by layer_id, bbox, zoom, filter, and limit.
        let k1 = function_cache::cache_key(
            "org",
            "demo",
            "fn_parcels",
            "parcels",
            Some([0.0, 0.0, 1.0, 1.0]),
            Some(12),
            Some("name:A"),
            100,
        );
        let k_diff_bbox = function_cache::cache_key(
            "org",
            "demo",
            "fn_parcels",
            "parcels",
            Some([1.0, 1.0, 2.0, 2.0]),
            Some(12),
            Some("name:A"),
            100,
        );
        let k_diff_filter = function_cache::cache_key(
            "org",
            "demo",
            "fn_parcels",
            "parcels",
            Some([0.0, 0.0, 1.0, 1.0]),
            Some(12),
            Some("name:B"),
            100,
        );
        let k_diff_zoom = function_cache::cache_key(
            "org",
            "demo",
            "fn_parcels",
            "parcels",
            Some([0.0, 0.0, 1.0, 1.0]),
            Some(13),
            Some("name:A"),
            100,
        );
        let k_diff_limit = function_cache::cache_key(
            "org",
            "demo",
            "fn_parcels",
            "parcels",
            Some([0.0, 0.0, 1.0, 1.0]),
            Some(12),
            Some("name:A"),
            10,
        );
        assert_ne!(k1, k_diff_bbox);
        assert_ne!(k1, k_diff_filter);
        assert_ne!(k1, k_diff_zoom);
        assert_ne!(k1, k_diff_limit);
    }
}

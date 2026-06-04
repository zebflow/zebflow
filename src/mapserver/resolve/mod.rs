use crate::mapserver::publish::manifest::{PublishedLayerManifest, SourceKind};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod artifact;
pub mod cache;
pub mod function_cache;
pub mod geoparquet;
pub mod geoparquet_direct;
pub mod geoparquet_optimize;
pub mod query;
pub mod filter_dsl;
pub mod mvt;
pub mod stats;
pub mod style;
pub mod style_dsl;
pub mod tile;
pub mod tile_cache;

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
            let fc = preloaded
                .ok_or_else(|| "GeoJsonFunction: features not preloaded".to_string())?;
            query::resolve_feature_collection_from_value(manifest, request, fc.clone())
        }
    }?;

    // Post-process: simplify geometry at low zoom to reduce response size
    crate::mapserver::infra::simplify::simplify_features(&mut response.features, request.zoom);

    Ok(response)
}

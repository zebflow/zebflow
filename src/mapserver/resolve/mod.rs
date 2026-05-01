use crate::mapserver::publish::manifest::{PublishedLayerManifest, SourceKind};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod artifact;
pub mod cache;
pub mod query;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolveRequest {
    pub layer_id: String,
    pub bbox: Option<[f64; 4]>,
    pub zoom: Option<u8>,
    pub limit: Option<usize>,
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
    match manifest.source_kind {
        SourceKind::GeoJsonFile => {
            query::resolve_feature_collection_from_geojson_file(manifest, request)
        }
        SourceKind::GeoJsonArtifact => artifact::resolve_from_artifact(
            manifest,
            request,
            std::path::Path::new(&manifest.source_ref),
        ),
    }
}

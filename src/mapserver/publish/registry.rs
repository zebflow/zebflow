use crate::mapserver::publish::manifest::{PublishedLayerManifest, SourceKind};

pub fn manifest_from_runtime(
    layer_id: String,
    path: String,
    source_kind: SourceKind,
    source_ref: String,
    mode: String,
    min_zoom: Option<u8>,
    max_zoom: Option<u8>,
    bbox_required: bool,
    max_features: usize,
    allowed_properties: Vec<String>,
    style: Option<serde_json::Value>,
    filter: Option<String>,
    function_slug: Option<String>,
    cache_ttl_secs: Option<u64>,
) -> PublishedLayerManifest {
    PublishedLayerManifest {
        layer_id,
        path,
        source_kind,
        source_ref,
        mode,
        min_zoom,
        max_zoom,
        bbox_required,
        max_features,
        allowed_properties,
        style,
        filter,
        function_slug,
        cache_ttl_secs,
    }
}

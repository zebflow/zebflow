use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    GeoJsonFile,
    GeoJsonArtifact,
    GeoParquet,
    GeoJsonFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishedLayerManifest {
    pub layer_id: String,
    pub path: String,
    pub source_kind: SourceKind,
    pub source_ref: String,
    pub mode: String,
    pub min_zoom: Option<u8>,
    pub max_zoom: Option<u8>,
    pub bbox_required: bool,
    pub max_features: usize,
    pub allowed_properties: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function_slug: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_ttl_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArtifactChunkRecord {
    pub chunk_id: String,
    pub rel_path: String,
    pub bbox: [f64; 4],
    pub item_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeoJsonArtifactManifest {
    pub version: u32,
    pub layer_id: String,
    pub source_ref: String,
    pub chunk_grid_degrees: f64,
    pub feature_count: usize,
    pub chunk_count: usize,
    pub bbox: Option<[f64; 4]>,
    pub chunks: Vec<ArtifactChunkRecord>,
}

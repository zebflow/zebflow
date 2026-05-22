use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    GeoJsonFile,
    GeoJsonArtifact,
    GeoParquet,
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

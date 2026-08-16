use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::contracts::{ContractError, ContractKind, ContractMetadata, PlatformContract};

/// One layer published through a project MapServer instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapserverLayerRecord {
    pub layer_id: String,
    pub path: String,
    pub source_path: String,
    #[serde(default)]
    pub source_kind: String,
    #[serde(default)]
    pub artifact_manifest_path: Option<String>,
    pub mode: String,
    #[serde(default)]
    pub min_zoom: Option<u8>,
    #[serde(default)]
    pub max_zoom: Option<u8>,
    pub bbox_required: bool,
    pub max_features: usize,
    pub allowed_properties: Vec<String>,
    #[serde(default)]
    pub feature_count: Option<usize>,
    #[serde(default)]
    pub chunk_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function_slug: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_ttl_secs: Option<u64>,
}

/// Canonical published layer registry for one MapServer instance.
pub struct MapPublishManifestContract;

impl PlatformContract for MapPublishManifestContract {
    type Spec = Vec<MapserverLayerRecord>;
    const KIND: ContractKind = ContractKind::MapPublishManifest;

    fn validate(_metadata: &ContractMetadata, spec: &Self::Spec) -> Result<(), ContractError> {
        let mut layer_ids = HashSet::new();
        for layer in spec {
            if layer.layer_id.trim().is_empty() {
                return Err(ContractError::invalid("map layer_id must not be empty"));
            }
            if !layer_ids.insert(layer.layer_id.as_str()) {
                return Err(ContractError::invalid(format!(
                    "duplicate map layer_id '{}'",
                    layer.layer_id
                )));
            }
        }
        Ok(())
    }
}

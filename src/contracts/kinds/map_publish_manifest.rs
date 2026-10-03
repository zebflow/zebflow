use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::contracts::{ContractError, ContractKind, ContractMetadata, PlatformContract};

/// One layer published through a project MapServer instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapserverLayerRecord {
    pub layer_id: String,
    pub path: String,
    /// The project store `source_path` is a key in, pinned at publish.
    pub store: String,
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

/// A layer id is a folder and file name in the cache and the store, so it is
/// a plain slug: 1–64 of `A-Z a-z 0-9 - _`.
pub fn valid_layer_id(id: &str) -> bool {
    (1..=64).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// `{instance}/{layer}/manifest.json` with both names plain slugs: the one
/// shape an artifact path may take below `mapserver-artifacts/`, since a
/// delete removes its folder.
pub fn valid_artifact_rest(rest: &str) -> bool {
    let parts: Vec<&str> = rest.split('/').collect();
    matches!(parts.as_slice(), [instance, layer, "manifest.json"] if valid_layer_id(instance) && valid_layer_id(layer))
}

/// Canonical published layer registry for one MapServer instance.
pub struct MapPublishManifestContract;

impl PlatformContract for MapPublishManifestContract {
    type Spec = Vec<MapserverLayerRecord>;
    const KIND: ContractKind = ContractKind::MapPublishManifest;

    fn validate(_metadata: &ContractMetadata, spec: &Self::Spec) -> Result<(), ContractError> {
        let mut layer_ids = HashSet::new();
        for layer in spec {
            if !valid_layer_id(&layer.layer_id) {
                return Err(ContractError::invalid(format!(
                    "map layer_id '{}' must be 1–64 letters, digits, '-' or '_'",
                    layer.layer_id
                )));
            }
            // The artifact path names a folder a delete removes, so it has
            // exactly one shape and never climbs out of it.
            if let Some(artifact) = &layer.artifact_manifest_path {
                let rest = artifact.strip_prefix("mapserver-artifacts/").unwrap_or("");
                if !valid_artifact_rest(rest) {
                    return Err(ContractError::invalid(format!(
                        "map layer '{}' artifact_manifest_path '{artifact}' is not mapserver-artifacts/{{instance}}/{{layer}}/manifest.json",
                        layer.layer_id
                    )));
                }
            }
            if !layer_ids.insert(layer.layer_id.as_str()) {
                return Err(ContractError::invalid(format!(
                    "duplicate map layer_id '{}'",
                    layer.layer_id
                )));
            }
            // `source_path` is a key in the layer's store. The registry
            // (`data/store/mapserver/{instance}.layers.json`) can arrive
            // already written in a project bundle, so a key climbing out of
            // the store is refused here, at the read boundary every serving
            // path passes through.
            if crate::infra::io::path::rel_path_escapes_root(&layer.source_path) {
                return Err(ContractError::invalid(format!(
                    "map layer '{}' source_path '{}' escapes the project",
                    layer.layer_id, layer.source_path
                )));
            }
        }
        Ok(())
    }
}

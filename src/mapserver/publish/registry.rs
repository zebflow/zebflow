//! The published layer registry: where it lives, what shape it holds, and the
//! only reader and writer of the file.
//!
//! One file (`files/mapserver/{instance}.layers.json`) carries one
//! `MapPublishManifest` contract document. Both publishers — the project web UI
//! and `n.ms.publish` — go through here, so neither can drift from the other.

use std::path::{Path, PathBuf};

use crate::contracts::kinds::{MapPublishManifestContract, MapserverLayerRecord};
use crate::contracts::{ContractError, ContractMetadata};
use crate::mapserver::publish::manifest::{PublishedLayerManifest, SourceKind};

/// The single MapServer instance every project starts with.
pub const DEFAULT_INSTANCE: &str = "default-mapserver";

/// Where one instance's layer registry lives inside a project's `files/` tree.
pub fn layers_manifest_path(files_dir: &Path, instance: &str) -> PathBuf {
    files_dir
        .join("mapserver")
        .join(format!("{instance}.layers.json"))
}

/// The one canonical shape of a published layer's serving path.
///
/// Contract `MapPublishManifest`: `path` is stored **without** a leading slash.
/// Requests arrive with one, and records written before the two publishers
/// agreed may hold either, so every comparison normalises both sides.
pub fn normalize_layer_path(path: &str) -> &str {
    path.trim().trim_start_matches('/').trim_end_matches('/')
}

/// Reads one instance's registry. A missing file is an empty registry.
///
/// Paths are normalised on the way out, so a record written before the
/// publishers agreed reads back in the contract's shape and the next write
/// persists it. No separate migration step exists or is needed.
pub fn read_layers(path: &Path) -> Result<Vec<MapserverLayerRecord>, ContractError> {
    let mut items =
        match crate::contracts::read_optional_contract::<MapPublishManifestContract>(path) {
            Ok(document) => document.map(|value| value.spec).unwrap_or_default(),
            Err(err) => match read_pre_contract_array(path) {
                Some(items) => items,
                None => return Err(err),
            },
        };
    for item in &mut items {
        item.path = normalize_layer_path(&item.path).to_string();
    }
    Ok(items)
}

/// Reads a registry written before both publishers shared this module.
///
/// `n.ms.publish` used to write a bare JSON array here while the web side wrote
/// the contract envelope, so a project could hold a file neither the serving
/// path nor the other publisher could read. Accepting it once — and only the
/// bare array, with the one field that had drifted out of the record dropped —
/// turns that into a repair: the next write puts the file in the contract's
/// shape. Anything else still refuses, as the contract says it must.
fn read_pre_contract_array(path: &Path) -> Option<Vec<MapserverLayerRecord>> {
    let bytes = std::fs::read(path).ok()?;
    let mut entries: Vec<serde_json::Value> = serde_json::from_slice(&bytes).ok()?;
    for entry in &mut entries {
        if let Some(object) = entry.as_object_mut() {
            object.remove("column_stats_path");
        }
    }
    serde_json::from_value(serde_json::Value::Array(entries)).ok()
}

/// Durably writes one instance's registry as a canonical contract document.
pub fn write_layers(
    path: &Path,
    instance: &str,
    items: &[MapserverLayerRecord],
) -> Result<(), ContractError> {
    let mut items = items.to_vec();
    for item in &mut items {
        item.path = normalize_layer_path(&item.path).to_string();
    }
    crate::contracts::write_contract::<MapPublishManifestContract>(
        path,
        ContractMetadata::named(instance),
        items,
    )
}

#[allow(clippy::too_many_arguments)]
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

#[cfg(test)]
mod tests {
    use super::*;

    fn record(path: &str) -> MapserverLayerRecord {
        MapserverLayerRecord {
            layer_id: "roads".to_string(),
            path: path.to_string(),
            source_path: "mapserver/roads.geojson".to_string(),
            source_kind: "geojson_file".to_string(),
            artifact_manifest_path: None,
            mode: "features".to_string(),
            min_zoom: None,
            max_zoom: None,
            bbox_required: false,
            max_features: 1000,
            allowed_properties: vec!["name".to_string()],
            feature_count: None,
            chunk_count: None,
            style: None,
            filter: None,
            function_slug: None,
            cache_ttl_secs: None,
        }
    }

    #[test]
    fn the_registry_holds_one_path_shape_and_one_document_format() {
        let tmp = tempfile::tempdir().expect("tmp");
        let path = layers_manifest_path(tmp.path(), DEFAULT_INSTANCE);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");

        // Contract `MapPublishManifest`: `path` is stored without a leading
        // slash. A writer that adds one is how a published layer fails to serve.
        write_layers(&path, DEFAULT_INSTANCE, &[record("/roads")]).expect("write");
        let raw = std::fs::read_to_string(&path).expect("read raw");
        assert!(
            !raw.contains("\"/roads\""),
            "stored with a leading slash: {raw}"
        );

        // One document format: the contract envelope, not a bare array.
        let value: serde_json::Value = serde_json::from_str(&raw).expect("json");
        assert_eq!(value["kind"], serde_json::json!("MapPublishManifest"));
        assert_eq!(value["spec"][0]["path"], serde_json::json!("roads"));

        let items = read_layers(&path).expect("read");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].path, "roads");

        // A record written before the publishers agreed still reads back in the
        // contract's shape; the next write persists it.
        crate::contracts::write_contract::<MapPublishManifestContract>(
            &path,
            ContractMetadata::named(DEFAULT_INSTANCE),
            vec![record("/roads")],
        )
        .expect("legacy write");
        let items = read_layers(&path).expect("read legacy");
        assert_eq!(items[0].path, "roads");
    }

    #[test]
    fn a_registry_written_before_the_publishers_shared_this_module_is_repaired() {
        let tmp = tempfile::tempdir().expect("tmp");
        let path = layers_manifest_path(tmp.path(), DEFAULT_INSTANCE);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");

        // What `n.ms.publish` used to write: a bare array, carrying the
        // `column_stats_path` field the contract's record never had, in a file
        // the web side and the serving path could not read at all.
        let mut legacy = serde_json::to_value(record("roads")).expect("record json");
        legacy["column_stats_path"] =
            serde_json::json!("mapserver/.optimized/roads.spatial.stats.json");
        std::fs::write(
            &path,
            serde_json::to_string(&serde_json::json!([legacy])).expect("legacy json"),
        )
        .expect("legacy write");

        let items = read_layers(&path).expect("legacy read");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].layer_id, "roads");

        // Writing it back puts the file in the contract's shape.
        write_layers(&path, DEFAULT_INSTANCE, &items).expect("rewrite");
        let raw = std::fs::read_to_string(&path).expect("read raw");
        assert!(raw.contains("MapPublishManifest"));
        assert!(!raw.contains("column_stats_path"));

        // A file that is neither shape still refuses.
        std::fs::write(&path, b"{\"apiVersion\":\"zebflow.com/v1\"}").expect("bad write");
        assert!(read_layers(&path).is_err());
    }

    #[test]
    fn a_request_path_and_a_stored_path_meet_in_one_shape() {
        assert_eq!(normalize_layer_path("/roads/"), "roads");
        assert_eq!(normalize_layer_path("roads"), "roads");
        assert_eq!(normalize_layer_path(" /layers/roads "), "layers/roads");
    }
}

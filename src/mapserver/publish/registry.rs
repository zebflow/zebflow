//! The published layer registry: where it lives, what shape it holds, and the
//! only reader and writer of the file.
//!
//! One file (`data/store/mapserver/{instance}.layers.json`, store tier — never
//! a user object) carries one `MapPublishManifest` contract document. Both publishers — the project web UI
//! and `mapserver.layer.publish` — go through here, so neither can drift from the other.

use std::path::{Path, PathBuf};

use crate::contracts::kinds::{MapPublishManifestContract, MapserverLayerRecord};
use crate::contracts::{ContractError, ContractMetadata};
use crate::mapserver::publish::manifest::{PublishedLayerManifest, SourceKind};

/// The single MapServer instance every project starts with.
pub const DEFAULT_INSTANCE: &str = "default-mapserver";

/// Where one instance's layer registry lives in a project's `data/store/`.
pub fn layers_manifest_path(store_dir: &Path, instance: &str) -> PathBuf {
    store_dir
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

/// Reads one instance's registry. A missing file is an empty registry; paths
/// are normalised on the way out.
pub fn read_layers(path: &Path) -> Result<Vec<MapserverLayerRecord>, ContractError> {
    let mut items = crate::contracts::read_optional_contract::<MapPublishManifestContract>(path)?
        .map(|value| value.spec)
        .unwrap_or_default();
    for item in &mut items {
        item.path = normalize_layer_path(&item.path).to_string();
    }
    Ok(items)
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

    /// A record holding a `source_path` that climbs out of the store is
    /// refused on read, whoever wrote the file.
    #[test]
    fn a_layer_reaching_outside_the_project_is_refused_on_read() {
        let dir = tempfile::tempdir().unwrap();
        let enveloped = dir.path().join("enveloped.layers.json");
        std::fs::write(
            &enveloped,
            serde_json::json!({
                "apiVersion": "zebflow.com/v1",
                "kind": "MapPublishManifest",
                "metadata": { "name": "alice/demo" },
                "spec": [{
                    "layer_id": "roads", "path": "roads", "store": "local",
                    "source_path": "../../../../etc/passwd",
                    "source_kind": "geojson_file", "mode": "features",
                    "min_zoom": 0, "max_zoom": 14, "bbox_required": true,
                    "max_features": 1000, "allowed_properties": []
                }]
            })
            .to_string(),
        )
        .unwrap();
        let err = read_layers(&enveloped).expect_err("an escaping source_path is refused");
        assert!(err.to_string().contains("escapes the project"), "{err}");
    }

    /// A delete removes the folder an artifact path names and joins the layer
    /// id into cache and store names, so both have one shape (finding
    /// astra-map-layer-deletion-permits-traversal-to).
    #[test]
    fn a_layer_cannot_name_a_folder_outside_its_own() {
        let dir = tempfile::tempdir().unwrap();
        let path = layers_manifest_path(dir.path(), DEFAULT_INSTANCE);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        for (layer_id, artifact) in [
            ("roads", Some("../../../../home")),
            ("roads", Some("mapserver-artifacts/../../x/manifest.json")),
            ("roads", Some("mapserver/.artifacts/roads/manifest.json")),
            ("roads", Some("mapserver-artifacts/default-mapserver//manifest.json")),
            ("roads", Some("mapserver-artifacts/default-mapserver/roads/other.json")),
            ("../roads", None),
            ("ro ads", None),
        ] {
            let mut item = record("roads");
            item.layer_id = layer_id.to_string();
            item.artifact_manifest_path = artifact.map(str::to_string);
            assert!(write_layers(&path, DEFAULT_INSTANCE, &[item]).is_err(), "{layer_id} {artifact:?}");
        }
        let mut ok = record("roads");
        ok.artifact_manifest_path = Some("mapserver-artifacts/default-mapserver/roads/manifest.json".into());
        write_layers(&path, DEFAULT_INSTANCE, &[ok]).unwrap();
    }

    fn record(path: &str) -> MapserverLayerRecord {
        MapserverLayerRecord {
            layer_id: "roads".to_string(),
            path: path.to_string(),
            store: "local".to_string(),
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
    fn a_request_path_and_a_stored_path_meet_in_one_shape() {
        assert_eq!(normalize_layer_path("/roads/"), "roads");
        assert_eq!(normalize_layer_path("roads"), "roads");
        assert_eq!(normalize_layer_path(" /layers/roads "), "layers/roads");
    }
}

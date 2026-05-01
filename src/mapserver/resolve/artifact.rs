use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::mapserver::infra::bbox::normalize_bbox;
use crate::mapserver::publish::manifest::{GeoJsonArtifactManifest, PublishedLayerManifest};

use super::{ResolveRequest, ResolveResponse};

#[derive(Debug, Deserialize)]
struct ChunkItemRecord {
    bbox: [f64; 4],
    feature: Value,
}

pub fn resolve_from_artifact(
    manifest: &PublishedLayerManifest,
    request: &ResolveRequest,
    artifact_manifest_path: &Path,
) -> Result<ResolveResponse, String> {
    if manifest.bbox_required && request.bbox.is_none() {
        return Err("bbox is required for this layer".to_string());
    }
    let artifact = read_artifact_manifest(artifact_manifest_path)?;
    let bbox = request.bbox.and_then(normalize_bbox);
    let hard_limit = request
        .limit
        .unwrap_or(manifest.max_features)
        .min(manifest.max_features);
    let artifact_root = artifact_manifest_path
        .parent()
        .ok_or_else(|| "artifact manifest missing parent directory".to_string())?;
    let mut out = Vec::new();
    let mut truncated = false;
    for chunk in artifact.chunks.iter().filter(|chunk| {
        bbox.map(|needle| intersects_bbox(chunk.bbox, needle))
            .unwrap_or(true)
    }) {
        let chunk_path = artifact_root.join(&chunk.rel_path);
        let file = File::open(&chunk_path)
            .map_err(|err| format!("failed reading chunk {}: {err}", chunk_path.display()))?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line.map_err(|err| {
                format!("failed reading chunk line {}: {err}", chunk_path.display())
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let item = serde_json::from_str::<ChunkItemRecord>(&line)
                .map_err(|err| format!("failed parsing chunk line: {err}"))?;
            if bbox
                .map(|needle| intersects_bbox(item.bbox, needle))
                .unwrap_or(true)
            {
                if out.len() < hard_limit {
                    out.push(prune_feature_properties(
                        item.feature,
                        &manifest.allowed_properties,
                    ));
                } else {
                    truncated = true;
                    break;
                }
            }
        }
        if truncated {
            break;
        }
    }
    Ok(ResolveResponse {
        layer: manifest.layer_id.clone(),
        count: out.len(),
        truncated,
        features: out,
    })
}

fn read_artifact_manifest(path: &Path) -> Result<GeoJsonArtifactManifest, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|err| format!("failed reading artifact manifest {}: {err}", path.display()))?;
    serde_json::from_str::<GeoJsonArtifactManifest>(&raw)
        .map_err(|err| format!("failed parsing artifact manifest {}: {err}", path.display()))
}

fn intersects_bbox(a: [f64; 4], b: [f64; 4]) -> bool {
    !(a[2] < b[0] || a[0] > b[2] || a[3] < b[1] || a[1] > b[3])
}

fn prune_feature_properties(mut feature: Value, allowed: &[String]) -> Value {
    if allowed.is_empty() {
        return feature;
    }
    let Some(feature_obj) = feature.as_object_mut() else {
        return feature;
    };
    let Some(props) = feature_obj
        .get_mut("properties")
        .and_then(Value::as_object_mut)
    else {
        return feature;
    };
    props.retain(|key, _| allowed.iter().any(|candidate| candidate == key));
    feature
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::PathBuf;

    use serde_json::json;
    use sysinfo::{ProcessesToUpdate, System};

    use crate::mapserver::publish::build::build_geojson_artifact;
    use crate::mapserver::publish::manifest::{
        ArtifactChunkRecord, GeoJsonArtifactManifest, SourceKind,
    };

    use super::*;

    #[test]
    fn resolve_from_artifact_filters_bbox_and_limit() {
        let tmp = tempfile::tempdir().expect("tmp");
        let chunk_dir = tmp.path().join("chunks");
        fs::create_dir_all(&chunk_dir).expect("chunk dir");
        fs::write(
            chunk_dir.join("g0_0.ndjson"),
            format!(
                "{}\n{}\n",
                serde_json::to_string(&json!({
                    "bbox": [106.0, -6.5, 107.0, -6.0],
                    "feature": {
                        "type": "Feature",
                        "properties": { "name": "A", "code": "1" },
                        "geometry": {
                            "type": "Polygon",
                            "coordinates": [[[106.0, -6.5], [107.0, -6.5], [107.0, -6.0], [106.0, -6.0], [106.0, -6.5]]]
                        }
                    }
                }))
                .unwrap(),
                serde_json::to_string(&json!({
                    "bbox": [106.5, -6.4, 107.5, -5.9],
                    "feature": {
                        "type": "Feature",
                        "properties": { "name": "B", "code": "2" },
                        "geometry": {
                            "type": "Polygon",
                            "coordinates": [[[106.5, -6.4], [107.5, -6.4], [107.5, -5.9], [106.5, -5.9], [106.5, -6.4]]]
                        }
                    }
                }))
                .unwrap(),
            ),
        )
        .expect("chunk write");
        fs::write(
            tmp.path().join("manifest.json"),
            serde_json::to_string(&GeoJsonArtifactManifest {
                version: 1,
                layer_id: "adm1".to_string(),
                source_ref: "sample".to_string(),
                chunk_grid_degrees: 1.0,
                feature_count: 2,
                chunk_count: 1,
                bbox: Some([106.0, -6.5, 107.5, -5.9]),
                chunks: vec![ArtifactChunkRecord {
                    chunk_id: "g0_0".to_string(),
                    rel_path: "chunks/g0_0.ndjson".to_string(),
                    bbox: [106.0, -6.5, 107.5, -5.9],
                    item_count: 2,
                }],
            })
            .unwrap(),
        )
        .expect("manifest write");

        let manifest = PublishedLayerManifest {
            layer_id: "adm1".to_string(),
            path: "/adm1".to_string(),
            source_kind: SourceKind::GeoJsonArtifact,
            source_ref: tmp.path().join("manifest.json").display().to_string(),
            mode: "features".to_string(),
            min_zoom: None,
            max_zoom: None,
            bbox_required: true,
            max_features: 1,
            allowed_properties: vec!["name".to_string()],
        };
        let req = ResolveRequest {
            layer_id: "adm1".to_string(),
            bbox: Some([106.0, -7.0, 108.0, -5.0]),
            zoom: Some(6),
            limit: Some(10),
        };
        let out = resolve_from_artifact(&manifest, &req, &tmp.path().join("manifest.json"))
            .expect("resolve");
        assert_eq!(out.count, 1);
        assert!(out.truncated);
        assert_eq!(out.features[0]["properties"], json!({"name": "A"}));
    }

    #[test]
    fn mapserver_artifact_villages_memory_benchmark() {
        let Ok(source) = env::var("MAPSERVER_BENCH_SOURCE") else {
            eprintln!("MAPSERVER_BENCH_SOURCE not set; skipping benchmark");
            return;
        };
        let source = PathBuf::from(source);
        if !source.exists() {
            eprintln!("MAPSERVER_BENCH_SOURCE missing; skipping benchmark");
            return;
        }

        let tmp = tempfile::tempdir().expect("tmp");
        let artifact_dir = tmp.path().join("artifact");
        let mut system = System::new();
        let pid = sysinfo::get_current_pid().expect("pid");

        let before = process_memory_kib(&mut system, pid);
        let build = build_geojson_artifact(
            &source,
            "adm4_villages",
            &artifact_dir,
            "private/mapserver/.artifacts/default-mapserver/adm4_villages",
        )
        .expect("build artifact");
        let after_build = process_memory_kib(&mut system, pid);

        let manifest = PublishedLayerManifest {
            layer_id: "adm4_villages".to_string(),
            path: "/layers/adm4-villages".to_string(),
            source_kind: SourceKind::GeoJsonArtifact,
            source_ref: build.manifest_abs_path.display().to_string(),
            mode: "features".to_string(),
            min_zoom: None,
            max_zoom: None,
            bbox_required: true,
            max_features: 50,
            allowed_properties: vec![],
        };
        let req = ResolveRequest {
            layer_id: "adm4_villages".to_string(),
            bbox: Some([107.50, -7.10, 107.80, -6.80]),
            zoom: Some(11),
            limit: Some(50),
        };
        let resolved =
            resolve_from_artifact(&manifest, &req, &build.manifest_abs_path).expect("resolve");
        let after_query = process_memory_kib(&mut system, pid);

        eprintln!(
            "MAPSERVER_BENCH before_kib={before} after_build_kib={after_build} after_query_kib={after_query} chunks={} features={} returned={} truncated={}",
            build.chunk_count, build.feature_count, resolved.count, resolved.truncated
        );
    }

    fn process_memory_kib(system: &mut System, pid: sysinfo::Pid) -> u64 {
        system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
        system.process(pid).map(|p| p.memory()).unwrap_or(0) / 1024
    }
}

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::mapserver::infra::bbox::{geometry_bbox, normalize_bbox};
use crate::mapserver::infra::geojson_stream::stream_feature_collection_from_path;

use super::manifest::{ArtifactChunkRecord, GeoJsonArtifactManifest};

const CHUNK_GRID_DEGREES: f64 = 1.0;
const MAX_OPEN_CHUNK_WRITERS: usize = 64;
const MAX_CHUNK_BYTES: u64 = 4 * 1024 * 1024;
const MAX_CHUNK_ITEMS: usize = 4_000;
const MAX_SPLIT_DEPTH: usize = 8;

#[derive(Debug, Clone)]
pub struct ArtifactBuildOutput {
    pub manifest_abs_path: PathBuf,
    pub manifest_rel_path: String,
    pub feature_count: usize,
    pub chunk_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChunkItemRecord {
    bbox: [f64; 4],
    feature: Value,
}

#[derive(Debug)]
struct ChunkMeta {
    bbox: [f64; 4],
    item_count: usize,
}

#[derive(Debug)]
struct WriterEntry {
    writer: BufWriter<File>,
    last_used_tick: u64,
}

struct ChunkWriterPool {
    chunks_dir: PathBuf,
    max_open: usize,
    tick: u64,
    open: HashMap<String, WriterEntry>,
}

impl ChunkWriterPool {
    fn new(chunks_dir: PathBuf, max_open: usize) -> Self {
        Self {
            chunks_dir,
            max_open,
            tick: 0,
            open: HashMap::new(),
        }
    }

    fn write_line(&mut self, chunk_id: &str, line: &str) -> Result<(), String> {
        self.tick += 1;
        if !self.open.contains_key(chunk_id) {
            if self.open.len() >= self.max_open {
                self.evict_one()?;
            }
            let path = self.chunks_dir.join(format!("{chunk_id}.ndjson"));
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .map_err(|err| format!("failed opening chunk file {}: {err}", path.display()))?;
            self.open.insert(
                chunk_id.to_string(),
                WriterEntry {
                    writer: BufWriter::new(file),
                    last_used_tick: self.tick,
                },
            );
        }
        if let Some(entry) = self.open.get_mut(chunk_id) {
            entry.last_used_tick = self.tick;
            entry
                .writer
                .write_all(line.as_bytes())
                .map_err(|err| format!("failed writing chunk {chunk_id}: {err}"))?;
            entry
                .writer
                .write_all(b"\n")
                .map_err(|err| format!("failed writing chunk newline {chunk_id}: {err}"))?;
        }
        Ok(())
    }

    fn evict_one(&mut self) -> Result<(), String> {
        let mut oldest_key: Option<String> = None;
        let mut oldest_tick = u64::MAX;
        for (key, entry) in &self.open {
            if entry.last_used_tick < oldest_tick {
                oldest_tick = entry.last_used_tick;
                oldest_key = Some(key.clone());
            }
        }
        let Some(key) = oldest_key else {
            return Ok(());
        };
        if let Some(mut entry) = self.open.remove(&key) {
            entry
                .writer
                .flush()
                .map_err(|err| format!("failed flushing chunk writer {key}: {err}"))?;
        }
        Ok(())
    }

    fn flush_all(mut self) -> Result<(), String> {
        for (key, mut entry) in self.open.drain() {
            entry
                .writer
                .flush()
                .map_err(|err| format!("failed flushing chunk writer {key}: {err}"))?;
        }
        Ok(())
    }
}

pub fn build_geojson_artifact(
    source_abs_path: &Path,
    layer_id: &str,
    artifact_abs_dir: &Path,
    artifact_rel_dir: &str,
) -> Result<ArtifactBuildOutput, String> {
    let parent = artifact_abs_dir
        .parent()
        .ok_or_else(|| "artifact dir missing parent".to_string())?;
    fs::create_dir_all(parent).map_err(|err| {
        format!(
            "failed creating artifact parent {}: {err}",
            parent.display()
        )
    })?;
    let tmp_dir = tempfile::tempdir_in(parent)
        .map_err(|err| format!("failed creating temp artifact dir: {err}"))?;
    let chunks_dir = tmp_dir.path().join("chunks");
    fs::create_dir_all(&chunks_dir)
        .map_err(|err| format!("failed creating chunks dir {}: {err}", chunks_dir.display()))?;

    let mut writer_pool = ChunkWriterPool::new(chunks_dir.clone(), MAX_OPEN_CHUNK_WRITERS);
    let mut chunk_meta = HashMap::<String, ChunkMeta>::new();
    let mut overall_bbox: Option<[f64; 4]> = None;
    let mut feature_count = 0usize;

    stream_feature_collection_from_path(source_abs_path, |feature| {
        if feature.get("type").and_then(Value::as_str) != Some("Feature") {
            return Err("GeoJSON features array contains a non-Feature item".to_string());
        }
        let bbox = feature
            .get("geometry")
            .and_then(geometry_bbox)
            .and_then(normalize_bbox)
            .ok_or_else(|| "feature geometry is missing or invalid".to_string())?;
        let chunk_id = chunk_id_for_bbox(bbox);
        let line = serde_json::to_string(&ChunkItemRecord {
            bbox,
            feature: feature.clone(),
        })
        .map_err(|err| format!("failed serializing chunk item: {err}"))?;
        writer_pool.write_line(&chunk_id, &line)?;
        feature_count += 1;
        overall_bbox = Some(merge_bbox(overall_bbox, bbox));
        chunk_meta
            .entry(chunk_id)
            .and_modify(|meta| {
                meta.bbox = merge_bbox(Some(meta.bbox), bbox);
                meta.item_count += 1;
            })
            .or_insert(ChunkMeta {
                bbox,
                item_count: 1,
            });
        Ok(())
    })?;

    writer_pool.flush_all()?;

    let mut chunks = chunk_meta
        .into_iter()
        .map(|(chunk_id, meta)| ArtifactChunkRecord {
            rel_path: format!("chunks/{chunk_id}.ndjson"),
            chunk_id,
            bbox: meta.bbox,
            item_count: meta.item_count,
        })
        .collect::<Vec<_>>();
    repartition_oversized_chunks(&chunks_dir, &mut chunks)?;
    chunks.sort_by(|a, b| a.chunk_id.cmp(&b.chunk_id));

    let artifact_manifest = GeoJsonArtifactManifest {
        version: 1,
        layer_id: layer_id.to_string(),
        source_ref: source_abs_path.display().to_string(),
        chunk_grid_degrees: CHUNK_GRID_DEGREES,
        feature_count,
        chunk_count: chunks.len(),
        bbox: overall_bbox,
        chunks,
    };

    let manifest_abs_path = tmp_dir.path().join("manifest.json");
    let raw = serde_json::to_string_pretty(&artifact_manifest)
        .map_err(|err| format!("failed serializing artifact manifest: {err}"))?;
    fs::write(&manifest_abs_path, raw).map_err(|err| {
        format!(
            "failed writing artifact manifest {}: {err}",
            manifest_abs_path.display()
        )
    })?;

    if artifact_abs_dir.exists() {
        fs::remove_dir_all(artifact_abs_dir).map_err(|err| {
            format!(
                "failed replacing old artifact dir {}: {err}",
                artifact_abs_dir.display()
            )
        })?;
    }
    fs::rename(tmp_dir.keep(), artifact_abs_dir).map_err(|err| {
        format!(
            "failed moving artifact into place {}: {err}",
            artifact_abs_dir.display()
        )
    })?;

    Ok(ArtifactBuildOutput {
        manifest_abs_path: artifact_abs_dir.join("manifest.json"),
        manifest_rel_path: format!("{artifact_rel_dir}/manifest.json"),
        feature_count,
        chunk_count: artifact_manifest.chunk_count,
    })
}

fn chunk_id_for_bbox(bbox: [f64; 4]) -> String {
    let cx = (bbox[0] + bbox[2]) / 2.0;
    let cy = (bbox[1] + bbox[3]) / 2.0;
    let x = ((cx + 180.0) / CHUNK_GRID_DEGREES).floor() as i32;
    let y = ((cy + 90.0) / CHUNK_GRID_DEGREES).floor() as i32;
    format!("g{x}_{y}")
}

fn merge_bbox(current: Option<[f64; 4]>, next: [f64; 4]) -> [f64; 4] {
    match current {
        Some([minx, miny, maxx, maxy]) => [
            minx.min(next[0]),
            miny.min(next[1]),
            maxx.max(next[2]),
            maxy.max(next[3]),
        ],
        None => next,
    }
}

fn repartition_oversized_chunks(
    chunks_dir: &Path,
    chunks: &mut Vec<ArtifactChunkRecord>,
) -> Result<(), String> {
    let mut next_chunks = Vec::with_capacity(chunks.len());
    for chunk in chunks.drain(..) {
        let chunk_path = chunks_dir.join(format!("{}.ndjson", chunk.chunk_id));
        let size = fs::metadata(&chunk_path)
            .map(|meta| meta.len())
            .map_err(|err| {
                format!(
                    "failed reading chunk metadata {}: {err}",
                    chunk_path.display()
                )
            })?;
        if size <= MAX_CHUNK_BYTES && chunk.item_count <= MAX_CHUNK_ITEMS {
            next_chunks.push(chunk);
            continue;
        }
        let items = read_chunk_items(&chunk_path)?;
        fs::remove_file(&chunk_path).map_err(|err| {
            format!(
                "failed removing oversized chunk {}: {err}",
                chunk_path.display()
            )
        })?;
        let split_chunks = split_chunk_records(chunks_dir, &chunk.chunk_id, items, 0)?;
        next_chunks.extend(split_chunks);
    }
    *chunks = next_chunks;
    Ok(())
}

fn read_chunk_items(path: &Path) -> Result<Vec<ChunkItemRecord>, String> {
    let file = File::open(path)
        .map_err(|err| format!("failed opening chunk {}: {err}", path.display()))?;
    let reader = BufReader::new(file);
    let mut items = Vec::new();
    for line in reader.lines() {
        let line =
            line.map_err(|err| format!("failed reading chunk line {}: {err}", path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        let item = serde_json::from_str::<ChunkItemRecord>(&line)
            .map_err(|err| format!("failed parsing chunk line {}: {err}", path.display()))?;
        items.push(item);
    }
    Ok(items)
}

fn split_chunk_records(
    chunks_dir: &Path,
    chunk_id: &str,
    items: Vec<ChunkItemRecord>,
    depth: usize,
) -> Result<Vec<ArtifactChunkRecord>, String> {
    if items.is_empty() {
        return Ok(Vec::new());
    }
    let (bbox, approx_bytes) = summarize_items(&items)?;
    if depth >= MAX_SPLIT_DEPTH
        || items.len() <= 1
        || (items.len() <= MAX_CHUNK_ITEMS && approx_bytes <= MAX_CHUNK_BYTES)
    {
        return write_chunk_records(chunks_dir, chunk_id, items, bbox);
    }

    let split_axis_x = (bbox[2] - bbox[0]) >= (bbox[3] - bbox[1]);
    let split_at = if split_axis_x {
        (bbox[0] + bbox[2]) / 2.0
    } else {
        (bbox[1] + bbox[3]) / 2.0
    };

    let mut left = Vec::new();
    let mut right = Vec::new();
    for item in items {
        let center = if split_axis_x {
            (item.bbox[0] + item.bbox[2]) / 2.0
        } else {
            (item.bbox[1] + item.bbox[3]) / 2.0
        };
        if center <= split_at {
            left.push(item);
        } else {
            right.push(item);
        }
    }

    if left.is_empty() || right.is_empty() {
        let mut items = if left.is_empty() { right } else { left };
        items.sort_by(|a, b| {
            let ac = if split_axis_x {
                (a.bbox[0] + a.bbox[2]) / 2.0
            } else {
                (a.bbox[1] + a.bbox[3]) / 2.0
            };
            let bc = if split_axis_x {
                (b.bbox[0] + b.bbox[2]) / 2.0
            } else {
                (b.bbox[1] + b.bbox[3]) / 2.0
            };
            ac.partial_cmp(&bc).unwrap_or(std::cmp::Ordering::Equal)
        });
        let mid = items.len() / 2;
        right = items.split_off(mid);
        left = items;
    }

    let mut out = Vec::new();
    out.extend(split_chunk_records(
        chunks_dir,
        &format!("{chunk_id}a"),
        left,
        depth + 1,
    )?);
    out.extend(split_chunk_records(
        chunks_dir,
        &format!("{chunk_id}b"),
        right,
        depth + 1,
    )?);
    Ok(out)
}

fn summarize_items(items: &[ChunkItemRecord]) -> Result<([f64; 4], u64), String> {
    let mut bbox: Option<[f64; 4]> = None;
    let mut approx_bytes = 0u64;
    for item in items {
        bbox = Some(merge_bbox(bbox, item.bbox));
        approx_bytes += serde_json::to_vec(item)
            .map_err(|err| format!("failed sizing chunk item: {err}"))?
            .len() as u64
            + 1;
    }
    Ok((bbox.unwrap_or([0.0, 0.0, 0.0, 0.0]), approx_bytes))
}

fn write_chunk_records(
    chunks_dir: &Path,
    chunk_id: &str,
    items: Vec<ChunkItemRecord>,
    bbox: [f64; 4],
) -> Result<Vec<ArtifactChunkRecord>, String> {
    let path = chunks_dir.join(format!("{chunk_id}.ndjson"));
    let file = File::create(&path)
        .map_err(|err| format!("failed creating chunk {}: {err}", path.display()))?;
    let mut writer = BufWriter::new(file);
    let item_count = items.len();
    for item in items {
        let line = serde_json::to_string(&item)
            .map_err(|err| format!("failed serializing split chunk item: {err}"))?;
        writer
            .write_all(line.as_bytes())
            .map_err(|err| format!("failed writing split chunk {}: {err}", path.display()))?;
        writer
            .write_all(b"\n")
            .map_err(|err| format!("failed writing split newline {}: {err}", path.display()))?;
    }
    writer
        .flush()
        .map_err(|err| format!("failed flushing split chunk {}: {err}", path.display()))?;
    Ok(vec![ArtifactChunkRecord {
        chunk_id: chunk_id.to_string(),
        rel_path: format!("chunks/{chunk_id}.ndjson"),
        bbox,
        item_count,
    }])
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;

    use super::*;

    #[test]
    fn build_geojson_artifact_creates_chunk_manifest() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let source = tmp.path().join("sample.geojson");
        fs::write(
            &source,
            serde_json::to_string(&json!({
                "type": "FeatureCollection",
                "features": [
                    {
                        "type": "Feature",
                        "properties": { "name": "A" },
                        "geometry": {
                            "type": "Point",
                            "coordinates": [106.8, -6.2]
                        }
                    },
                    {
                        "type": "Feature",
                        "properties": { "name": "B" },
                        "geometry": {
                            "type": "Point",
                            "coordinates": [107.1, -6.4]
                        }
                    }
                ]
            }))
            .expect("json"),
        )
        .expect("write source");
        let artifact_dir = tmp.path().join("artifact");
        let out = build_geojson_artifact(
            &source,
            "sample",
            &artifact_dir,
            "private/mapserver/.artifacts/sample",
        )
        .expect("build artifact");
        assert_eq!(out.feature_count, 2);
        assert!(out.chunk_count >= 1);
        assert!(artifact_dir.join("manifest.json").exists());
    }

    #[test]
    fn build_geojson_artifact_recursively_splits_oversized_chunks() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let source = tmp.path().join("dense.geojson");
        let large_label = "x".repeat(1200);
        let features = (0..5000)
            .map(|index| {
                json!({
                    "type": "Feature",
                    "properties": {
                        "name": format!("Village {index}"),
                        "label": large_label.clone(),
                    },
                    "geometry": {
                        "type": "Point",
                        "coordinates": [
                            106.8 + ((index % 50) as f64 * 0.001),
                            -6.2 - ((index / 50) as f64 * 0.001)
                        ]
                    }
                })
            })
            .collect::<Vec<_>>();
        fs::write(
            &source,
            serde_json::to_string(&json!({
                "type": "FeatureCollection",
                "features": features,
            }))
            .expect("json"),
        )
        .expect("write source");
        let artifact_dir = tmp.path().join("artifact");
        let out = build_geojson_artifact(
            &source,
            "dense",
            &artifact_dir,
            "private/mapserver/.artifacts/dense",
        )
        .expect("build artifact");
        assert!(out.chunk_count > 1);
        let raw = fs::read_to_string(artifact_dir.join("manifest.json")).expect("manifest");
        let manifest: GeoJsonArtifactManifest = serde_json::from_str(&raw).expect("parse manifest");
        assert!(
            manifest
                .chunks
                .iter()
                .all(|chunk| chunk.item_count <= MAX_CHUNK_ITEMS)
        );
    }
}

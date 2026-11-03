//! The GeoJSON reply of a map layer, written one feature at a time into a
//! reply with a fixed largest size, so one request never holds more than that.

use std::io::Write;

use serde_json::Value;

use crate::mapserver::publish::manifest::{PublishedLayerManifest, SourceKind};

use super::ResolveRequest;

/// The largest GeoJSON reply the map server sends, in bytes. `max_features`
/// counts shapes, not bytes: a few thousand detailed polygons, held whole as a
/// JSON tree and then as text, are enough to run a 2 GiB process out of
/// memory. A reply that reaches this size ends early with `"truncated":
/// true`; every shape of a layer that large is drawn as PNG or vector tiles.
pub const MAX_REPLY_BYTES: usize = 16 * 1024 * 1024;

/// The layer's features for one request as a GeoJSON FeatureCollection:
/// `features`, `layer`, `count` (features in this reply) and `truncated`
/// (features were left out, by the request's limit or by `MAX_REPLY_BYTES`).
pub fn geojson_reply(
    manifest: &PublishedLayerManifest,
    request: &ResolveRequest,
    preloaded: Option<&Value>,
) -> Result<Vec<u8>, String> {
    reply_within(manifest, request, preloaded, MAX_REPLY_BYTES)
}

fn reply_within(
    manifest: &PublishedLayerManifest,
    request: &ResolveRequest,
    preloaded: Option<&Value>,
    max_bytes: usize,
) -> Result<Vec<u8>, String> {
    let mut reply = Reply::new(max_bytes);
    if !super::zoom_within_bounds(manifest, request.zoom) {
        return Ok(reply.finish(&manifest.layer_id, false));
    }
    let truncated = match manifest.source_kind {
        // Streamed: each row is written and dropped before the next is read.
        SourceKind::GeoParquet => {
            super::geoparquet::for_each_feature(manifest, request, |mut feature, _| {
                crate::mapserver::infra::simplify::simplify_features(
                    std::slice::from_mut(&mut feature),
                    request.zoom,
                );
                reply.push(&feature)
            })?
        }
        // These sources are read whole already; the reply is still capped.
        _ => {
            let resolved = super::resolve_features(manifest, request, preloaded)?;
            let all_fit = resolved.features.iter().all(|feature| reply.push(feature));
            resolved.truncated || !all_fit
        }
    };
    Ok(reply.finish(&manifest.layer_id, truncated))
}

/// A FeatureCollection being written. Features go straight into the bytes,
/// so no JSON tree of the whole reply exists.
struct Reply {
    body: Vec<u8>,
    count: usize,
    max_bytes: usize,
}

impl Reply {
    fn new(max_bytes: usize) -> Self {
        let mut body = Vec::with_capacity(max_bytes.min(64 * 1024));
        body.extend_from_slice(br#"{"type":"FeatureCollection","features":["#);
        Reply {
            body,
            count: 0,
            max_bytes,
        }
    }

    /// Adds one feature, or adds nothing and answers false when it would take
    /// the reply past its largest size. Writing stops at that size, so a single
    /// huge feature never lands in memory as text.
    fn push(&mut self, feature: &Value) -> bool {
        let start = self.body.len();
        let mut out = Capped {
            body: &mut self.body,
            max_bytes: self.max_bytes,
        };
        let written = (self.count == 0 || out.write_all(b",").is_ok())
            && serde_json::to_writer(&mut out, feature).is_ok();
        if !written {
            self.body.truncate(start);
            return false;
        }
        self.count += 1;
        true
    }

    fn finish(mut self, layer: &str, truncated: bool) -> Vec<u8> {
        self.body.extend_from_slice(br#"],"layer":"#);
        // A string always serializes.
        let _ = serde_json::to_writer(&mut self.body, layer);
        let _ = write!(
            self.body,
            r#","count":{},"truncated":{}}}"#,
            self.count, truncated
        );
        self.body
    }
}

/// A writer that refuses to grow the reply past `max_bytes`.
struct Capped<'a> {
    body: &'a mut Vec<u8>,
    max_bytes: usize,
}

impl Write for Capped<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.body.len() + buf.len() > self.max_bytes {
            return Err(std::io::Error::other("the reply is at its largest size"));
        }
        self.body.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn square(at: f64, points: usize) -> Value {
        let ring: Vec<Value> = (0..=points)
            .map(|i| {
                let a = i as f64 / points as f64 * std::f64::consts::TAU;
                json!([at + a.cos() * 0.001, 1.0 + a.sin() * 0.001])
            })
            .collect();
        json!({"type": "Feature", "properties": {}, "geometry": {"type": "Polygon", "coordinates": [ring]}})
    }

    #[test]
    fn a_reply_stops_before_its_largest_size_and_says_so() {
        let mut reply = Reply::new(20_000);
        let mut added = 0;
        for i in 0..100 {
            if !reply.push(&square(1.0 + i as f64 * 0.01, 100)) {
                break;
            }
            added += 1;
        }
        assert!(added > 0 && added < 100, "added {added}");
        let body = reply.finish("layer-a", true);
        assert!(body.len() <= 20_000 + 100);
        let parsed: Value = serde_json::from_slice(&body).expect("the reply stays valid JSON");
        assert_eq!(parsed["type"], "FeatureCollection");
        assert_eq!(parsed["features"].as_array().unwrap().len(), added);
        assert_eq!(parsed["count"], added);
        assert_eq!(parsed["truncated"], true);
        assert_eq!(parsed["layer"], "layer-a");
    }

    #[test]
    fn a_feature_larger_than_the_reply_is_left_out_without_being_written_whole() {
        let mut reply = Reply::new(10_000);
        assert!(reply.push(&square(1.0, 10)));
        assert!(!reply.push(&square(2.0, 100_000)));
        assert!(
            reply.body.capacity() < 1_000_000,
            "capacity {}",
            reply.body.capacity()
        );
        let parsed: Value = serde_json::from_slice(&reply.finish("layer-a", true)).unwrap();
        assert_eq!(parsed["count"], 1);
    }

    /// An optimized GeoParquet layer of `shapes` detailed polygons.
    fn detailed_layer(dir: &std::path::Path, shapes: usize, points: usize) -> std::path::PathBuf {
        let features: Vec<Value> = (0..shapes)
            .map(|i| {
                let mut f = square(1.0 + i as f64 * 0.01, points);
                f["properties"] = json!({ "name": format!("area {i}") });
                f
            })
            .collect();
        let source = dir.join("areas.geojson");
        std::fs::write(
            &source,
            serde_json::to_vec(&json!({"type": "FeatureCollection", "features": features}))
                .unwrap(),
        )
        .unwrap();
        let raw = dir.join("areas.raw.parquet");
        let optimized = dir.join("areas.spatial.parquet");
        super::super::geoparquet_optimize::convert_geojson_to_geoparquet(&source, &raw).unwrap();
        super::super::geoparquet_optimize::optimize_geoparquet(&raw, &optimized).unwrap();
        optimized
    }

    fn manifest_for(source: &std::path::Path) -> PublishedLayerManifest {
        PublishedLayerManifest {
            layer_id: "areas".to_string(),
            path: "areas".to_string(),
            source_kind: SourceKind::GeoParquet,
            source_ref: source.display().to_string(),
            mode: "features".to_string(),
            min_zoom: None,
            max_zoom: None,
            bbox_required: false,
            max_features: 2000,
            allowed_properties: vec!["name".to_string()],
            style: None,
            filter: None,
            function_slug: None,
            cache_ttl_secs: None,
        }
    }

    fn request(limit: usize) -> ResolveRequest {
        ResolveRequest {
            layer_id: "areas".to_string(),
            bbox: Some([0.0, 0.0, 2.0, 2.0]),
            zoom: None,
            limit: Some(limit),
            filter: None,
        }
    }

    // The GeoParquet path drives DataFusion with `block_in_place`, which needs
    // a multi-thread runtime.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_layer_of_detailed_shapes_answers_within_the_largest_size() {
        let tmp = tempfile::tempdir().unwrap();
        // 40 polygons of 5000 points: about 7 MB as GeoJSON text.
        let manifest = manifest_for(&detailed_layer(tmp.path(), 40, 5000));

        let capped = reply_within(&manifest, &request(2000), None, 1024 * 1024).unwrap();
        assert!(
            capped.len() <= 1024 * 1024 + 200,
            "reply of {} bytes",
            capped.len()
        );
        let capped: Value = serde_json::from_slice(&capped).expect("valid JSON");
        let count = capped["count"].as_u64().unwrap();
        assert!(count > 0 && count < 40, "count {count}");
        assert_eq!(capped["features"].as_array().unwrap().len() as u64, count);
        assert_eq!(capped["truncated"], true);
        assert_eq!(
            capped["features"][0]["properties"]["name"]
                .as_str()
                .map(|n| n.starts_with("area ")),
            Some(true)
        );

        // The request's limit also says truncated, now that one extra row is read.
        let limited: Value = serde_json::from_slice(
            &reply_within(&manifest, &request(10), None, usize::MAX).unwrap(),
        )
        .unwrap();
        assert_eq!(limited["count"], 10);
        assert_eq!(limited["truncated"], true);

        let whole: Value = serde_json::from_slice(
            &reply_within(&manifest, &request(2000), None, usize::MAX).unwrap(),
        )
        .unwrap();
        assert_eq!(whole["count"], 40);
        assert_eq!(whole["truncated"], false);
    }

    #[test]
    fn an_empty_reply_is_a_valid_feature_collection() {
        let parsed: Value =
            serde_json::from_slice(&Reply::new(1024).finish("a\"b", false)).unwrap();
        assert_eq!(parsed["features"], json!([]));
        assert_eq!(parsed["count"], 0);
        assert_eq!(parsed["truncated"], false);
        assert_eq!(parsed["layer"], "a\"b");
    }
}

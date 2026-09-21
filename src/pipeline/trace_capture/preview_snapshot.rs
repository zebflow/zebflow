//! A small picture of a temporary image, kept in the record so the canvas
//! can still show it after the run's files are gone.
//!
//! When a node declares `--preview image` (or `--preview-in image`) and the
//! value that preview would draw is a **temporary** image FileRef, the engine
//! writes `preview_snapshot` on the node's trace entry — never inside the
//! payload: `{ slot, mime, width, height, data_base64 }`, longest side
//! [`SNAPSHOT_MAX_SIDE`] px, JPEG at [`SNAPSHOT_QUALITY`], at most
//! [`SNAPSHOT_MAX_BYTES`]; over that, `{ slot, snapshot_skipped: "too large" }`.
//! A durable file needs none: the Studio reads it from the store.
//!
//! JPEG rather than WebP: the `image` crate in this tree encodes WebP
//! lossless only, and a lossless 540 px poster with a gradient is not 64 KB.

use std::io::Cursor;
use std::sync::Arc;

use base64::Engine as _;
use image::GenericImageView;
use serde_json::{Value, json};

use crate::pipeline::nodes::shared::file_ref::{LIFECYCLE_TEMPORARY, file_ref_lifecycle, is_file_ref, read_file_ref_bytes};
use crate::platform::services::PlatformService;

pub const SNAPSHOT_MAX_SIDE: u32 = 540;
pub const SNAPSHOT_MAX_BYTES: usize = 64 * 1024;
pub const SNAPSHOT_QUALITY: u8 = 70;
pub const SNAPSHOT_MIME: &str = "image/jpeg";
/// How deep the no-path search looks, the same as the editor's.
const AUTO_PICK_DEPTH: usize = 3;
/// The trigger envelope's own keys: searched last, like the editor does.
const ENVELOPE_KEYS: &[&str] = &["body", "files"];

/// A `{ as, path? }` preview cell from `config.preview`.
fn declared_image_cell<'a>(config: &'a Value, slot: &str) -> Option<&'a Value> {
    let cell = config.get("preview")?.get(slot)?;
    let as_kind = cell.get("as")?.as_str()?.trim().to_ascii_lowercase();
    (as_kind == "image").then_some(cell)
}

fn is_image_ref(value: &Value) -> bool {
    is_file_ref(value)
        && (value.get("mime").and_then(Value::as_str).is_some_and(|m| m.starts_with("image/"))
            || value.get("kind").and_then(Value::as_str) == Some("image"))
}

fn follow_path<'a>(payload: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = payload;
    for segment in path.trim().split('.').filter(|s| !s.is_empty()) {
        current = match current {
            Value::Object(map) => map.get(segment)?,
            Value::Array(items) => items.get(segment.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(current)
}

/// The value an image preview of `payload` would draw: the declared path,
/// or the first image FileRef found the way the editor finds it — the node's
/// own product before the envelope, a durable file before a temporary one.
pub fn chosen_image_value<'a>(payload: &'a Value, cell: &Value) -> Option<&'a Value> {
    if let Some(path) = cell.get("path").and_then(Value::as_str).map(str::trim).filter(|p| !p.is_empty()) {
        return follow_path(payload, path);
    }
    fn walk<'a>(value: &'a Value, depth: usize, skip_temporary: bool) -> Option<&'a Value> {
        if is_image_ref(value) {
            if skip_temporary && file_ref_lifecycle(value) == Some(LIFECYCLE_TEMPORARY) {
                return None;
            }
            return Some(value);
        }
        if depth >= AUTO_PICK_DEPTH {
            return None;
        }
        match value {
            Value::Array(items) => items.iter().find_map(|v| walk(v, depth + 1, skip_temporary)),
            Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort_by_key(|k| ENVELOPE_KEYS.contains(&k.as_str()));
                keys.into_iter().find_map(|k| walk(&map[k], depth + 1, skip_temporary))
            }
            _ => None,
        }
    }
    walk(payload, 0, true).or_else(|| walk(payload, 0, false))
}

/// The snapshot object for one temporary image FileRef, or the reason it was skipped.
pub fn snapshot_of(platform: &Arc<PlatformService>, owner: &str, project: &str, slot: &str, file_ref: &Value) -> Value {
    let bytes = match read_file_ref_bytes(platform, owner, project, file_ref) {
        Ok(b) => b,
        Err(e) => return json!({ "slot": slot, "snapshot_skipped": format!("unreadable: {}", e.message) }),
    };
    match encode(&bytes) {
        Ok((jpeg, w, h)) if jpeg.len() <= SNAPSHOT_MAX_BYTES => json!({
            "slot": slot,
            "mime": SNAPSHOT_MIME,
            "width": w,
            "height": h,
            "data_base64": base64::engine::general_purpose::STANDARD.encode(&jpeg),
        }),
        Ok(_) => json!({ "slot": slot, "snapshot_skipped": "too large" }),
        Err(why) => json!({ "slot": slot, "snapshot_skipped": why }),
    }
}

/// Decode with a bomb guard, shrink to the longest side, encode JPEG.
fn encode(bytes: &[u8]) -> Result<(Vec<u8>, u32, u32), String> {
    let mut reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format().map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16_000);
    limits.max_image_height = Some(16_000);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let img = reader.decode().map_err(|e| format!("undecodable: {e}"))?;
    let small = if img.width().max(img.height()) > SNAPSHOT_MAX_SIDE { img.thumbnail(SNAPSHOT_MAX_SIDE, SNAPSHOT_MAX_SIDE) } else { img };
    let (w, h) = small.dimensions();
    let rgb = small.to_rgb8();
    let mut out = Cursor::new(Vec::new());
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, SNAPSHOT_QUALITY)
        .encode(rgb.as_raw(), w, h, image::ExtendedColorType::Rgb8)
        .map_err(|e| format!("encode: {e}"))?;
    Ok((out.into_inner(), w, h))
}

/// What the engine stores on the entry for this node, if anything: the
/// declared `out` half first, else `in`; only for a temporary image FileRef.
pub fn snapshot_for(platform: Option<&Arc<PlatformService>>, owner: &str, project: &str, config: &Value, input: &Value, output: &Value) -> Option<Value> {
    let platform = platform?;
    for (slot, payload) in [("out", output), ("in", input)] {
        let Some(cell) = declared_image_cell(config, slot) else { continue };
        let Some(value) = chosen_image_value(payload, cell) else { continue };
        if is_image_ref(value) && file_ref_lifecycle(value) == Some(LIFECYCLE_TEMPORARY) {
            return Some(snapshot_of(platform, owner, project, slot, value));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_ref(lifecycle: &str, mime: &str) -> Value {
        let kind = mime.split('/').next().unwrap_or("binary");
        json!({ "__zf_type": "file_ref", "backend": "zebfs", "ref": "tmp/runs/r/files/a.png", "filename": "a.png", "mime": mime, "kind": kind,
            "size": 1, "sha256": format!("sha256:{}", "0".repeat(64)), "lifecycle": lifecycle, "origin": "fs.svg.convert", "trust": "generated" })
    }

    #[test]
    fn the_chosen_value_follows_the_path_or_picks_like_the_editor() {
        let temp = file_ref("temporary", "image/png");
        let durable = file_ref("durable", "image/png");
        let payload = json!({ "body": { "photo": temp.clone() }, "image": temp.clone(), "saved": durable.clone(), "note": "x" });
        assert_eq!(chosen_image_value(&payload, &json!({ "as": "image", "path": "image" })), Some(&temp));
        // No path: a durable file first, wherever it sits.
        assert_eq!(chosen_image_value(&payload, &json!({ "as": "image" })), Some(&durable));
        // Only temporaries: the node's own product before the envelope.
        let payload = json!({ "body": { "photo": temp.clone() }, "image": temp.clone() });
        assert!(chosen_image_value(&payload, &json!({ "as": "image" })).is_some());
        assert_eq!(chosen_image_value(&json!({ "n": 1 }), &json!({ "as": "image" })), None);
        assert_eq!(chosen_image_value(&json!({ "clip": file_ref("temporary", "video/mp4") }), &json!({ "as": "image" })), None);
    }

    #[test]
    fn a_snapshot_is_a_small_jpeg_and_a_huge_one_is_skipped() {
        let png = crate::pipeline::nodes::basic::fs::svg::convert::test_support::solid_png(1080, 1350, [1, 33, 105]);
        let (jpeg, w, h) = encode(&png).unwrap();
        assert_eq!((w, h), (432, 540));
        assert!(jpeg.len() <= SNAPSHOT_MAX_BYTES, "{}", jpeg.len());
        assert!(jpeg.starts_with(&[0xFF, 0xD8]));
        assert!(encode(b"not an image").unwrap_err().contains("undecodable"));
    }

    #[test]
    fn nothing_is_recorded_without_a_declared_image_preview_or_for_a_durable_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut config = crate::platform::model::PlatformConfig::default();
        config.data_root = tmp.keep().join("platform");
        config.default_password = "secret".to_string();
        let platform = Arc::new(PlatformService::from_config(config).expect("platform"));
        let out = json!({ "image": file_ref("durable", "image/png") });
        assert!(snapshot_for(Some(&platform), "o", "p", &json!({ "preview": { "out": { "as": "image" } } }), &Value::Null, &out).is_none());
        let out = json!({ "image": file_ref("temporary", "image/png") });
        assert!(snapshot_for(Some(&platform), "o", "p", &json!({}), &Value::Null, &out).is_none());
        assert!(snapshot_for(Some(&platform), "o", "p", &json!({ "preview": { "out": { "as": "json" } } }), &Value::Null, &out).is_none());
        // Declared and temporary but the bytes are gone: skipped, with the reason, not an error.
        let snap = snapshot_for(Some(&platform), "o", "p", &json!({ "preview": { "out": { "as": "image" } } }), &Value::Null, &out).unwrap();
        assert_eq!(snap["slot"], json!("out"));
        assert!(snap["snapshot_skipped"].as_str().unwrap().starts_with("unreadable"));
    }
}

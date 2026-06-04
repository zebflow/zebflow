//! Tile-specific response cache.
//!
//! Similar to `cache.rs` but tuned for PNG tile responses:
//! - Longer TTL (600s) since tiles are more stable than feature queries
//! - More entries (512) since many tiles are requested per layer
//! - No gzip: PNG is already compressed, stored as raw bytes
//! - Max body: 256KB per tile

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Write as _;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const TILE_CACHE_TTL: Duration = Duration::from_secs(600);
const TILE_CACHE_MAX_ENTRIES: usize = 2048;
const TILE_CACHE_MAX_BODY_BYTES: usize = 256 * 1024;

#[derive(Clone)]
struct TileCacheEntry {
    body: Vec<u8>,
    inserted_at: Instant,
    source_ref: String,
    source_version: String,
}

struct TileCacheState {
    entries: HashMap<String, TileCacheEntry>,
    order: VecDeque<String>,
}

fn tile_cache() -> &'static Mutex<TileCacheState> {
    static CACHE: OnceLock<Mutex<TileCacheState>> = OnceLock::new();
    CACHE.get_or_init(|| {
        Mutex::new(TileCacheState {
            entries: HashMap::new(),
            order: VecDeque::new(),
        })
    })
}

/// Build a cache key for a tile request.
pub fn tile_cache_key(
    layer_id: &str,
    bbox: &[f64; 4],
    width: u32,
    height: u32,
    zoom: Option<u8>,
    source_ref: &str,
    style_dsl: Option<&str>,
    filter: Option<&str>,
    format: &str,
) -> String {
    let sv = source_version(source_ref);
    let z = zoom
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".to_string());
    let mut key = String::with_capacity(128);
    let _ = write!(
        key,
        "tile:{layer_id}:{:.6},{:.6},{:.6},{:.6}:{width}x{height}:z={z}:src={sv}:fmt={format}",
        bbox[0], bbox[1], bbox[2], bbox[3]
    );
    if let Some(dsl) = style_dsl {
        let hash = super::style_dsl::style_hash(dsl);
        let _ = write!(key, ":sty={hash:016x}");
    }
    if let Some(f) = filter {
        let hash = super::filter_dsl::filter_hash(f);
        let _ = write!(key, ":flt={hash:016x}");
    }
    key
}

/// Get a cached tile PNG. Returns `None` on miss or expiry.
pub fn get_tile(key: &str) -> Option<Vec<u8>> {
    let mut cache = tile_cache().lock().ok()?;
    let entry = cache.entries.get(key)?;
    if entry.inserted_at.elapsed() <= TILE_CACHE_TTL {
        return Some(entry.body.clone());
    }
    // TTL expired — check if source file is unchanged
    let current_version = source_version(&entry.source_ref);
    if current_version == entry.source_version {
        let body = entry.body.clone();
        if let Some(entry) = cache.entries.get_mut(key) {
            entry.inserted_at = Instant::now();
        }
        return Some(body);
    }
    // Source changed — evict
    cache.entries.remove(key);
    None
}

/// Store a tile PNG in the cache.
pub fn put_tile(key: String, body: Vec<u8>, source_ref: &str) {
    if body.len() > TILE_CACHE_MAX_BODY_BYTES {
        return;
    }
    let Ok(mut cache) = tile_cache().lock() else {
        return;
    };
    cache.entries.insert(
        key.clone(),
        TileCacheEntry {
            body,
            inserted_at: Instant::now(),
            source_ref: source_ref.to_string(),
            source_version: source_version(source_ref),
        },
    );
    cache.order.push_back(key);
    evict(&mut cache);
}

fn evict(cache: &mut TileCacheState) {
    while cache.entries.len() > TILE_CACHE_MAX_ENTRIES {
        let Some(oldest) = cache.order.pop_front() else {
            break;
        };
        cache.entries.remove(&oldest);
    }
}

fn source_version(source_ref: &str) -> String {
    let path = Path::new(source_ref);
    let Ok(meta) = std::fs::metadata(path) else {
        return "missing".to_string();
    };
    let modified = meta
        .modified()
        .ok()
        .and_then(|ts| ts.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|v| v.as_secs())
        .unwrap_or(0);
    format!("{}:{}", meta.len(), modified)
}

// ── Metatile pixmap cache ──────────────────────────────────────────────
//
// Caches rendered metatile pixmap data so concurrent tile requests can
// slice from the same rendered image without re-querying or re-rendering.

const METATILE_PIXMAP_TTL: Duration = Duration::from_secs(30);
const METATILE_PIXMAP_MAX_ENTRIES: usize = 16;

struct MetatilePixmapEntry {
    /// Raw RGBA pixel data (width × height × 4 bytes).
    data: Vec<u8>,
    width: u32,
    height: u32,
    /// The metatile's geographic bounding box.
    meta_bbox: [f64; 4],
    inserted_at: Instant,
}

struct MetatilePixmapCache {
    entries: HashMap<String, MetatilePixmapEntry>,
    order: VecDeque<String>,
    /// Metatiles currently being rendered (in-flight tracking).
    inflight: HashSet<String>,
}

fn metatile_cache() -> &'static Mutex<MetatilePixmapCache> {
    static CACHE: OnceLock<Mutex<MetatilePixmapCache>> = OnceLock::new();
    CACHE.get_or_init(|| {
        Mutex::new(MetatilePixmapCache {
            entries: HashMap::new(),
            order: VecDeque::new(),
            inflight: HashSet::new(),
        })
    })
}

/// Try to claim a metatile for rendering. Returns `true` if this caller should
/// render the metatile. Returns `false` if another request is already rendering it.
pub fn try_claim_metatile(meta_key: &str) -> bool {
    let Ok(mut cache) = metatile_cache().lock() else {
        return false;
    };
    cache.inflight.insert(meta_key.to_string())
}

/// Release a metatile claim after rendering is complete.
pub fn release_metatile(meta_key: &str) {
    if let Ok(mut cache) = metatile_cache().lock() {
        cache.inflight.remove(meta_key);
    }
}

/// Store a rendered metatile pixmap.
pub fn put_metatile_pixmap(
    meta_key: String,
    data: Vec<u8>,
    width: u32,
    height: u32,
    meta_bbox: [f64; 4],
) {
    let Ok(mut cache) = metatile_cache().lock() else {
        return;
    };
    cache.entries.insert(
        meta_key.clone(),
        MetatilePixmapEntry {
            data,
            width,
            height,
            meta_bbox,
            inserted_at: Instant::now(),
        },
    );
    cache.order.push_back(meta_key);
    // Evict old entries
    while cache.entries.len() > METATILE_PIXMAP_MAX_ENTRIES {
        let Some(oldest) = cache.order.pop_front() else {
            break;
        };
        cache.entries.remove(&oldest);
    }
}

/// Slice a single tile from a cached metatile pixmap.
/// Returns PNG bytes for the tile at `tile_bbox` within the metatile.
pub fn get_metatile_tile(
    meta_key: &str,
    tile_bbox: &[f64; 4],
    tile_w_px: u32,
    tile_h_px: u32,
) -> Option<Vec<u8>> {
    let cache = metatile_cache().lock().ok()?;
    let entry = cache.entries.get(meta_key)?;
    if entry.inserted_at.elapsed() > METATILE_PIXMAP_TTL {
        return None;
    }

    let meta = &entry.meta_bbox;
    let meta_w = meta[2] - meta[0];
    let meta_h = meta[3] - meta[1];
    if meta_w.abs() < 1e-12 || meta_h.abs() < 1e-12 {
        return None;
    }

    // Compute pixel offset of the tile within the metatile
    let px_x = ((tile_bbox[0] - meta[0]) / meta_w * entry.width as f64).round() as i64;
    // Y is flipped: higher lat = lower pixel y
    let px_y = ((meta[3] - tile_bbox[3]) / meta_h * entry.height as f64).round() as i64;

    if px_x < 0 || px_y < 0 {
        return None;
    }
    let px_x = px_x as u32;
    let px_y = px_y as u32;

    if px_x + tile_w_px > entry.width || px_y + tile_h_px > entry.height {
        return None;
    }

    // Slice pixel data
    let mut sub = tiny_skia::Pixmap::new(tile_w_px, tile_h_px)?;
    let dst_data = sub.data_mut();
    let src_stride = entry.width as usize * 4;
    let dst_stride = tile_w_px as usize * 4;

    for y in 0..tile_h_px as usize {
        let src_off = (px_y as usize + y) * src_stride + px_x as usize * 4;
        let dst_off = y * dst_stride;
        if src_off + dst_stride > entry.data.len() {
            return None;
        }
        dst_data[dst_off..dst_off + dst_stride]
            .copy_from_slice(&entry.data[src_off..src_off + dst_stride]);
    }

    sub.encode_png().ok()
}

/// Build a metatile key for in-flight tracking and pixmap cache.
pub fn metatile_cache_key(
    layer_id: &str,
    meta_bbox: &[f64; 4],
    zoom: Option<u8>,
    source_ref: &str,
    style_dsl: Option<&str>,
    filter: Option<&str>,
) -> String {
    let sv = source_version(source_ref);
    let z = zoom
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".to_string());
    let mut key = String::with_capacity(128);
    let _ = write!(
        key,
        "meta:{layer_id}:{:.6},{:.6},{:.6},{:.6}:z={z}:src={sv}",
        meta_bbox[0], meta_bbox[1], meta_bbox[2], meta_bbox[3]
    );
    if let Some(dsl) = style_dsl {
        let hash = super::style_dsl::style_hash(dsl);
        let _ = write!(key, ":sty={hash:016x}");
    }
    if let Some(f) = filter {
        let hash = super::filter_dsl::filter_hash(f);
        let _ = write!(key, ":flt={hash:016x}");
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_cache_key_format() {
        let key = tile_cache_key("lga", &[144.0, -38.0, 145.0, -37.0], 256, 256, Some(10), "/tmp/test.parquet", None, None, "png");
        assert!(key.starts_with("tile:lga:"));
        assert!(key.contains("256x256"));
        assert!(key.contains("z=10"));
        assert!(key.contains("fmt=png"));
    }

    #[test]
    fn tile_cache_miss_on_empty() {
        assert!(get_tile("nonexistent-key-xyz").is_none());
    }
}

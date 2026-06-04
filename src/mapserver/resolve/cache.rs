use std::collections::{HashMap, VecDeque};
use std::fmt::Write as _;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;

use crate::mapserver::publish::manifest::PublishedLayerManifest;

use super::ResolveRequest;

const RESPONSE_CACHE_TTL: Duration = Duration::from_secs(300);
const RESPONSE_CACHE_MAX_ENTRIES: usize = 64;
const RESPONSE_CACHE_MAX_BODY_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone)]
pub struct ResponseCacheEntry {
    pub body_gzip: Vec<u8>,
    inserted_at: Instant,
    source_ref: String,
    source_version: String,
}

struct ResponseCacheState {
    entries: HashMap<String, ResponseCacheEntry>,
    order: VecDeque<String>,
}

fn response_cache() -> &'static Mutex<ResponseCacheState> {
    static CACHE: OnceLock<Mutex<ResponseCacheState>> = OnceLock::new();
    CACHE.get_or_init(|| {
        Mutex::new(ResponseCacheState {
            entries: HashMap::new(),
            order: VecDeque::new(),
        })
    })
}

pub fn cache_key(request: &ResolveRequest) -> String {
    let mut key = format!("{}:", request.layer_id);
    if let Some(bbox) = request.bbox {
        let _ = write!(
            key,
            "{:.4},{:.4},{:.4},{:.4}",
            bbox[0], bbox[1], bbox[2], bbox[3]
        );
    } else {
        key.push('*');
    }
    let _ = write!(
        key,
        ":z={}:l={}",
        request
            .zoom
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string()),
        request
            .limit
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    if let Some(ref filter) = request.filter {
        let hash = super::filter_dsl::filter_hash(filter);
        let _ = write!(key, ":flt={hash:016x}");
    }
    key
}

pub fn response_cache_key(manifest: &PublishedLayerManifest, request: &ResolveRequest) -> String {
    let mut key = cache_key(request);
    let source_version = source_version(&manifest.source_ref);
    let _ = write!(
        key,
        ":src={}:bbox={}:max={}:props={}",
        source_version,
        if manifest.bbox_required { "1" } else { "0" },
        manifest.max_features,
        manifest.allowed_properties.join(",")
    );
    key
}

pub fn get_response_bytes(key: &str) -> Option<Vec<u8>> {
    let mut cache = response_cache().lock().ok()?;
    let entry = cache.entries.get(key)?;
    if entry.inserted_at.elapsed() <= RESPONSE_CACHE_TTL {
        return decode_gzip(&entry.body_gzip).ok();
    }
    // TTL expired — revalidate by checking source file mtime.
    // The cache key already embeds source_version, so if the file changed
    // this key would never be looked up. But if the same key is requeried
    // and the file hasn't changed, we can extend the entry's lifetime.
    let current_version = source_version(&entry.source_ref);
    if current_version == entry.source_version {
        // Source unchanged — refresh the entry
        let body = decode_gzip(&entry.body_gzip).ok();
        if let Some(entry) = cache.entries.get_mut(key) {
            entry.inserted_at = Instant::now();
        }
        return body;
    }
    // Source changed — evict stale entry
    cache.entries.remove(key);
    None
}

pub fn put_response_bytes(key: String, body: Vec<u8>, source_ref: &str) {
    if body.len() > RESPONSE_CACHE_MAX_BODY_BYTES {
        return;
    }
    let Ok(body_gzip) = encode_gzip(&body) else {
        return;
    };
    let Ok(mut cache) = response_cache().lock() else {
        return;
    };
    cache.entries.insert(
        key.clone(),
        ResponseCacheEntry {
            body_gzip,
            inserted_at: Instant::now(),
            source_ref: source_ref.to_string(),
            source_version: source_version(source_ref),
        },
    );
    cache.order.push_back(key);
    evict_response_cache(&mut cache);
}

fn evict_response_cache(cache: &mut ResponseCacheState) {
    while cache.entries.len() > RESPONSE_CACHE_MAX_ENTRIES {
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

fn encode_gzip(input: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder
        .write_all(input)
        .map_err(|err| format!("failed writing gzip body: {err}"))?;
    encoder
        .finish()
        .map_err(|err| format!("failed finishing gzip body: {err}"))
}

fn decode_gzip(input: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = GzDecoder::new(input);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|err| format!("failed decoding gzip body: {err}"))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gzip_roundtrip_cache_body() {
        let body = vec![b'x'; 32 * 1024];
        let zipped = encode_gzip(&body).expect("encode");
        let unzipped = decode_gzip(&zipped).expect("decode");
        assert_eq!(body, unzipped);
        assert!(zipped.len() < body.len());
    }
}

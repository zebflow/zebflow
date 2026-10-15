//! In-memory TTL cache for GeoJsonFunction pipeline results.
//!
//! Prevents re-executing the function pipeline for every tile request on the
//! same layer. Multiple concurrent tile requests (e.g. a map loading 20 tiles)
//! result in one function call + N-1 cache hits.

use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::Value;

const DEFAULT_TTL_SECS: u64 = 60;
const MAX_ENTRIES: usize = 32;

struct CachedResult {
    value: Value,
    inserted_at: Instant,
    ttl: Duration,
}

struct CacheState {
    entries: HashMap<String, CachedResult>,
    order: VecDeque<String>,
}

fn cache() -> &'static Mutex<CacheState> {
    static CACHE: OnceLock<Mutex<CacheState>> = OnceLock::new();
    CACHE.get_or_init(|| {
        Mutex::new(CacheState {
            entries: HashMap::new(),
            order: VecDeque::new(),
        })
    })
}

/// Build cache key for a function layer scoped by request parameters so one
/// caller's query parameters cannot poison the cached GeoJSON for another caller.
/// `format` is part of the key because the function receives it and may answer
/// differently for a PNG tile, an MVT tile and a GeoJSON download. The free-text
/// filter stays last so no other part can be forged through it.
#[allow(clippy::too_many_arguments)]
pub fn cache_key(
    owner: &str,
    project: &str,
    slug: &str,
    layer_id: &str,
    bbox: Option<[f64; 4]>,
    zoom: Option<u8>,
    format: &str,
    filter: Option<&str>,
    limit: usize,
) -> String {
    let bbox_part = match bbox {
        Some([minx, miny, maxx, maxy]) => {
            format!("{}:{}:{}:{}", minx.to_bits(), miny.to_bits(), maxx.to_bits(), maxy.to_bits())
        }
        None => "none".to_string(),
    };
    let zoom_part = zoom
        .map(|z| z.to_string())
        .unwrap_or_else(|| "none".to_string());
    let filter_part = filter.unwrap_or("");
    format!(
        "{owner}:{project}:{slug}:layer={layer_id}:bbox={bbox_part}:zoom={zoom_part}:format={format}:limit={limit}:filter={filter_part}"
    )
}

/// Return cached function result if present and not expired.
pub fn get_cached(key: &str) -> Option<Value> {
    let mut state = cache().lock().ok()?;
    let entry = state.entries.get(key)?;
    if entry.inserted_at.elapsed() <= entry.ttl {
        Some(entry.value.clone())
    } else {
        state.entries.remove(key);
        None
    }
}

/// Store a function result in the cache with a given TTL (in seconds).
pub fn put_cached(key: &str, value: &Value, ttl_secs: u64) {
    let ttl = Duration::from_secs(if ttl_secs == 0 {
        DEFAULT_TTL_SECS
    } else {
        ttl_secs
    });
    let Ok(mut state) = cache().lock() else {
        return;
    };
    state.entries.insert(
        key.to_string(),
        CachedResult {
            value: value.clone(),
            inserted_at: Instant::now(),
            ttl,
        },
    );
    state.order.push_back(key.to_string());
    // Evict oldest entries when over capacity.
    while state.entries.len() > MAX_ENTRIES {
        let Some(oldest) = state.order.pop_front() else {
            break;
        };
        state.entries.remove(&oldest);
    }
}

/// Invalidate a specific cache entry or all entries matching a layer/function prefix.
pub fn invalidate(key: &str) {
    if let Ok(mut state) = cache().lock() {
        let prefix = format!("{key}:");
        state
            .entries
            .retain(|existing, _| existing != key && !existing.starts_with(&prefix));
        state
            .order
            .retain(|existing| existing != key && !existing.starts_with(&prefix));
    }
}

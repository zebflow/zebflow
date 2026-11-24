//! Gzip for text responses, when the browser accepts it.
//!
//! A page, its scripts and the `zeb/*` libraries are text that shrinks to a
//! third or less, and an office without a CDN in front of it sent them full
//! size. This compresses on the way out:
//!
//! - only a complete `200` body whose size is known (a stream, server-sent
//!   events and a websocket upgrade pass through untouched), between
//!   [`MIN_BYTES`] and [`MAX_BYTES`];
//! - only text-like types (HTML, JavaScript, CSS, JSON, SVG, XML, wasm);
//! - never a response that is already encoded or answers a `Range`.
//!
//! A response the server marks `immutable` (a script addressed by its hash) is
//! the same bytes forever at its address, and one with a strong `ETag` (a
//! library file, named by its content hash) is the same bytes wherever that
//! tag goes: both are compressed once and kept, up to [`CACHE_BYTES`] in all.

use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::sync::{LazyLock, Mutex};

use axum::body::{Body, Bytes};
use axum::extract::Request;
use axum::http::header::{ACCEPT_ENCODING, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, ETAG, VARY};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use axum::body::HttpBody as _;

/// Smaller bodies gain less than the header costs.
const MIN_BYTES: u64 = 1024;
/// Larger bodies are left alone rather than held in memory to compress.
const MAX_BYTES: u64 = 16 * 1024 * 1024;
/// The compressed immutable responses kept, in total.
const CACHE_BYTES: usize = 64 * 1024 * 1024;

/// A strong validator for `bytes`: a short content hash. Embedded files are
/// hashed once (by address and length); other bytes each time.
pub fn etag_of(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    static KNOWN: LazyLock<Mutex<HashMap<(usize, usize), String>>> = LazyLock::new(Default::default);
    let key = (bytes.as_ptr() as usize, bytes.len());
    let embedded = bytes.len() > 4096;
    if embedded && let Some(found) = KNOWN.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
        return found.clone();
    }
    let tag = format!("\"{}\"", &hex::encode(Sha256::digest(bytes))[..20]);
    if embedded {
        let mut known = KNOWN.lock().unwrap_or_else(|e| e.into_inner());
        if known.len() > 4096 {
            known.clear();
        }
        known.insert(key, tag.clone());
    }
    tag
}

/// Whether `if_none_match` names `etag`, compared weakly as RFC 9110 asks for `If-None-Match`.
fn matches_etag(if_none_match: &str, etag: &str) -> bool {
    let bare = |tag: &str| tag.trim().trim_start_matches("W/").to_string();
    let wanted = bare(etag);
    if_none_match.split(',').any(|tag| tag.trim() == "*" || bare(tag) == wanted)
}

pub async fn compress(request: Request, next: Next) -> Response {
    let if_none_match = request.headers().get(axum::http::header::IF_NONE_MATCH).and_then(|v| v.to_str().ok()).map(str::to_string);
    let accepts = request
        .headers()
        .get(ACCEPT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .is_some_and(accepts_gzip);
    let key = format!("{}{}", request.headers().get("host").and_then(|h| h.to_str().ok()).unwrap_or(""), request.uri());
    let response = next.run(request).await;
    // The browser already holds this exact answer: say so instead of sending it.
    if response.status() == StatusCode::OK
        && let (Some(wanted), Some(etag)) = (if_none_match.as_deref(), response.headers().get(ETAG).and_then(|v| v.to_str().ok()))
        && matches_etag(wanted, etag)
    {
        let mut not_modified = Response::new(Body::empty());
        *not_modified.status_mut() = StatusCode::NOT_MODIFIED;
        for name in [ETAG, CACHE_CONTROL, VARY] {
            if let Some(value) = response.headers().get(&name) {
                not_modified.headers_mut().insert(name, value.clone());
            }
        }
        return not_modified;
    }
    if !accepts || !compressible(&response) {
        return response;
    }
    let immutable = response
        .headers()
        .get(CACHE_CONTROL)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains("immutable"));
    // The same bytes are compressed once: an immutable answer is the same
    // bytes at its address forever, and a strong validator names its bytes
    // wherever they are served (a library file carries a content hash).
    let strong = response
        .headers()
        .get(ETAG)
        .and_then(|value| value.to_str().ok())
        .filter(|etag| !etag.starts_with("W/"))
        .map(|etag| format!("etag:{etag}"));
    let kept_as = if immutable { Some(key) } else { strong };
    let (mut parts, body) = response.into_parts();
    if let Some(kept_as) = kept_as.as_deref()
        && let Some(found) = cached(kept_as)
    {
        return finish(parts, found);
    }
    let Ok(bytes) = axum::body::to_bytes(body, MAX_BYTES as usize).await else {
        // The size was checked before; a body that still overran is answered as an error, not truncated.
        parts.status = StatusCode::INTERNAL_SERVER_ERROR;
        return Response::from_parts(parts, Body::empty());
    };
    let thorough = kept_as.is_some();
    let packed = match tokio::task::spawn_blocking(move || gzip(&bytes, thorough).map(|packed| (packed, bytes))).await {
        Ok(Ok(found)) => found,
        _ => return Response::from_parts(parts, Body::empty()),
    };
    let (packed, original) = packed;
    // Not worth it: send what was asked for as it was.
    if packed.len() as u64 >= original.len() as u64 {
        return Response::from_parts(parts, Body::from(original));
    }
    let packed = Bytes::from(packed);
    if let Some(kept_as) = kept_as {
        remember(kept_as, packed.clone());
    }
    finish(parts, packed)
}

fn finish(mut parts: axum::http::response::Parts, packed: Bytes) -> Response {
    parts.headers.insert(CONTENT_ENCODING, HeaderValue::from_static("gzip"));
    parts.headers.remove(CONTENT_LENGTH);
    parts.headers.append(VARY, HeaderValue::from_static("accept-encoding"));
    // Another encoding is another representation: a strong validator no longer names it.
    if let Some(etag) = parts.headers.get(ETAG).and_then(|v| v.to_str().ok()).map(str::to_string)
        && !etag.starts_with("W/")
        && let Ok(weak) = HeaderValue::from_str(&format!("W/{etag}"))
    {
        parts.headers.insert(ETAG, weak);
    }
    Response::from_parts(parts, Body::from(packed))
}

fn accepts_gzip(header: &str) -> bool {
    header.split(',').any(|part| {
        let mut pieces = part.trim().split(';');
        let name = pieces.next().unwrap_or("").trim();
        let refused = pieces.any(|param| matches!(param.trim().replace(' ', "").as_str(), "q=0" | "q=0.0" | "q=0.00" | "q=0.000"));
        (name.eq_ignore_ascii_case("gzip") || name == "*") && !refused
    })
}

fn compressible(response: &Response) -> bool {
    if response.status() != StatusCode::OK {
        return false;
    }
    let headers = response.headers();
    if headers.contains_key(CONTENT_ENCODING) || headers.contains_key(CONTENT_RANGE) {
        return false;
    }
    let Some(size) = response.body().size_hint().exact() else { return false };
    if !(MIN_BYTES..=MAX_BYTES).contains(&size) {
        return false;
    }
    let kind = headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("").to_ascii_lowercase();
    let kind = kind.split(';').next().unwrap_or("").trim().to_string();
    (kind.starts_with("text/") && kind != "text/event-stream")
        || matches!(
            kind.as_str(),
            "application/javascript"
                | "application/json"
                | "application/xml"
                | "application/wasm"
                | "application/manifest+json"
                | "application/geo+json"
                | "image/svg+xml"
        )
}

fn gzip(bytes: &[u8], thorough: bool) -> std::io::Result<Vec<u8>> {
    // An immutable file is compressed once and sent many times: take the time.
    let level = if thorough { flate2::Compression::best() } else { flate2::Compression::new(5) };
    let mut encoder = flate2::write::GzEncoder::new(Vec::with_capacity(bytes.len() / 3), level);
    encoder.write_all(bytes)?;
    encoder.finish()
}

struct Kept {
    by_key: HashMap<String, Bytes>,
    order: VecDeque<String>,
    bytes: usize,
}

static KEPT: LazyLock<Mutex<Kept>> = LazyLock::new(|| Mutex::new(Kept { by_key: HashMap::new(), order: VecDeque::new(), bytes: 0 }));

fn cached(key: &str) -> Option<Bytes> {
    KEPT.lock().unwrap_or_else(|e| e.into_inner()).by_key.get(key).cloned()
}

fn remember(key: String, packed: Bytes) {
    if packed.len() > CACHE_BYTES / 4 {
        return;
    }
    let mut kept = KEPT.lock().unwrap_or_else(|e| e.into_inner());
    if kept.by_key.contains_key(&key) {
        return;
    }
    kept.bytes += packed.len();
    kept.order.push_back(key.clone());
    kept.by_key.insert(key, packed);
    while kept.bytes > CACHE_BYTES {
        let Some(oldest) = kept.order.pop_front() else { break };
        if let Some(gone) = kept.by_key.remove(&oldest) {
            kept.bytes -= gone.len();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_validator_matches_weakly() {
        assert!(matches_etag("W/\"abc\"", "\"abc\""));
        assert!(matches_etag("\"x\", \"abc\"", "W/\"abc\""));
        assert!(matches_etag("*", "\"abc\""));
        assert!(!matches_etag("\"abd\"", "\"abc\""));
        let big = vec![7u8; 10_000];
        assert_eq!(etag_of(&big), etag_of(&big));
        assert_ne!(etag_of(&big), etag_of(&big[1..]));
    }

    #[test]
    fn gzip_is_accepted_unless_refused() {
        assert!(accepts_gzip("gzip, deflate, br"));
        assert!(accepts_gzip("br;q=1.0, gzip;q=0.8"));
        assert!(accepts_gzip("*"));
        assert!(!accepts_gzip("gzip;q=0"));
        assert!(!accepts_gzip("br, deflate"));
    }

    #[test]
    fn text_shrinks_and_reads_back() {
        let text = "export const a = 1;\n".repeat(500);
        let packed = gzip(text.as_bytes(), false).unwrap();
        assert!(packed.len() < text.len() / 5);
        let mut back = String::new();
        std::io::Read::read_to_string(&mut flate2::read::GzDecoder::new(&packed[..]), &mut back).unwrap();
        assert_eq!(back, text);
    }
}

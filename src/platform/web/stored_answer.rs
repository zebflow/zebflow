//! The HTTP half of a stored-file answer (`web.response.send --file <FileRef>`
//! or `--path`): the node decided the store, key, type, name and validator
//! (`pipeline/nodes/basic/web/response/stored.rs`); this streams the object
//! from its store and speaks the parts of HTTP a file needs — `HEAD`, one
//! `Range` (206, 416, `If-Range`), `If-None-Match` / `If-Modified-Since`
//! (304), `ETag`, `Last-Modified`, `Accept-Ranges`.
//!
//! Bytes move in 64 KiB chunks from a blocking reader through a channel, so a
//! file of any size costs a fixed buffer, never its size, and never base64.
//! The object is opened before the answer starts: a file deleted between the
//! node and the answer is a 404, and a local file opened stays readable even
//! if its run's temporary files are swept while it streams.

use std::io::Read;

use axum::body::{Body, Bytes};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::Value;

use super::PlatformAppState;

const CHUNK: usize = 64 * 1024;

/// What the caller asked, as far as a file answer cares.
#[derive(Clone, Copy)]
pub(super) struct Asked<'a> {
    pub method: &'a Method,
    pub headers: &'a HeaderMap,
}

/// The facts the node put under `stored`.
struct Stored {
    store: String,
    key: String,
    size: u64,
    modified: Option<i64>,
    etag: String,
}

impl Stored {
    fn read(value: &Value) -> Option<Self> {
        Some(Self {
            store: value.get("store")?.as_str()?.to_string(),
            key: value.get("key")?.as_str()?.to_string(),
            size: value.get("size")?.as_u64()?,
            modified: value.get("modified").and_then(Value::as_i64),
            etag: value.get("etag")?.as_str()?.to_string(),
        })
    }

    fn last_modified(&self) -> Option<String> {
        let secs = self.modified?;
        chrono::DateTime::<chrono::Utc>::from_timestamp(secs, 0).map(|t| t.format("%a, %d %b %Y %H:%M:%S GMT").to_string())
    }
}

/// The answer for a `stored` envelope. `status` is the author's `--status`
/// (`None`: 200); `declared` the envelope's headers, applied as written.
/// Conditional and range requests apply only to a plain 200 answer.
pub(super) async fn stored_response(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    stored: &Value,
    status: Option<StatusCode>,
    declared: &[(String, String)],
    asked: Option<Asked<'_>>,
) -> Response {
    let Some(stored) = Stored::read(stored) else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "a stored answer without its store, key, size and validator").into_response();
    };
    let get = Method::GET;
    let method = asked.map(|a| a.method).unwrap_or(&get);
    let empty = HeaderMap::new();
    let request = asked.map(|a| a.headers).unwrap_or(&empty);
    let plain = status.is_none_or(|s| s == StatusCode::OK) && (method == Method::GET || method == Method::HEAD);
    let last_modified = stored.last_modified();

    let mut answer_status = status.unwrap_or(StatusCode::OK);
    let (mut offset, mut len) = (0u64, stored.size);
    if plain {
        if not_modified(request, &stored) {
            return validators_only(StatusCode::NOT_MODIFIED, &stored, last_modified.as_deref(), declared);
        }
        if let Some(range) = text(request, header::RANGE)
            && if_range_holds(request, &stored, last_modified.as_deref())
        {
            match parse_range(range, stored.size) {
                RangeAsk::Ignore => {}
                RangeAsk::Bytes(start, end) => {
                    answer_status = StatusCode::PARTIAL_CONTENT;
                    (offset, len) = (start, end - start + 1);
                }
                RangeAsk::Unsatisfiable => {
                    let mut resp = validators_only(StatusCode::RANGE_NOT_SATISFIABLE, &stored, last_modified.as_deref(), declared);
                    if let Ok(value) = HeaderValue::from_str(&format!("bytes */{}", stored.size)) {
                        resp.headers_mut().insert(header::CONTENT_RANGE, value);
                    }
                    return resp;
                }
            }
        }
    }

    let body = if method == Method::HEAD {
        Body::empty()
    } else {
        match open(state, owner, project, &stored, offset, len).await {
            Ok(reader) => stream(reader),
            Err(code) if code == "ZEBFS_NOT_FOUND" => return StatusCode::NOT_FOUND.into_response(),
            Err(code) => {
                eprintln!("⚠ stored answer {owner}/{project}: '{}' in store '{}' could not be opened ({code})", stored.key, stored.store);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
    };
    let mut resp = Response::new(body);
    *resp.status_mut() = answer_status;
    super::apply_zf_extras(&mut resp, declared);
    let headers = resp.headers_mut();
    put_validators(headers, &stored, last_modified.as_deref());
    headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    headers.insert(header::CONTENT_LENGTH, HeaderValue::from(len));
    if answer_status == StatusCode::PARTIAL_CONTENT
        && let Ok(value) = HeaderValue::from_str(&format!("bytes {offset}-{}/{}", offset + len - 1, stored.size))
    {
        headers.insert(header::CONTENT_RANGE, value);
    }
    super::harden_byte_headers(headers);
    resp
}

/// A 304 or 416: the validators and the caching rule, no body.
fn validators_only(status: StatusCode, stored: &Stored, last_modified: Option<&str>, declared: &[(String, String)]) -> Response {
    let mut resp = Response::new(Body::empty());
    *resp.status_mut() = status;
    let keep: Vec<(String, String)> = declared
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("cache-control") || name.eq_ignore_ascii_case("vary"))
        .cloned()
        .collect();
    super::apply_zf_extras(&mut resp, &keep);
    put_validators(resp.headers_mut(), stored, last_modified);
    resp
}

fn put_validators(headers: &mut HeaderMap, stored: &Stored, last_modified: Option<&str>) {
    if let Ok(value) = HeaderValue::from_str(&stored.etag) {
        headers.insert(header::ETAG, value);
    }
    if let Some(value) = last_modified.and_then(|v| HeaderValue::from_str(v).ok()) {
        headers.insert(header::LAST_MODIFIED, value);
    }
}

fn text(headers: &HeaderMap, name: header::HeaderName) -> Option<&str> {
    headers.get(name).and_then(|v| v.to_str().ok()).map(str::trim).filter(|v| !v.is_empty())
}

/// `If-None-Match` by weak comparison (RFC 9110 §13.1.2); without it,
/// `If-Modified-Since` against the object's time.
fn not_modified(request: &HeaderMap, stored: &Stored) -> bool {
    if let Some(tags) = text(request, header::IF_NONE_MATCH) {
        let ours = opaque(&stored.etag);
        return tags.split(',').map(str::trim).any(|tag| tag == "*" || opaque(tag) == ours);
    }
    match (text(request, header::IF_MODIFIED_SINCE).and_then(parse_http_date), stored.modified) {
        (Some(since), Some(modified)) => modified <= since,
        _ => false,
    }
}

/// `If-Range` holds when it names this object exactly: a strong tag equal to
/// ours, or the very `Last-Modified` we send. Otherwise the whole file goes.
fn if_range_holds(request: &HeaderMap, stored: &Stored, last_modified: Option<&str>) -> bool {
    match text(request, header::IF_RANGE) {
        None => true,
        Some(tag) if tag.starts_with('"') => !stored.etag.starts_with("W/") && tag == stored.etag,
        Some(tag) if tag.starts_with("W/") => false,
        Some(date) => last_modified == Some(date),
    }
}

fn opaque(tag: &str) -> &str {
    tag.trim().trim_start_matches("W/")
}

fn parse_http_date(value: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc2822(value).ok().map(|t| t.timestamp())
}

#[derive(Debug, PartialEq)]
enum RangeAsk {
    /// Not one range we serve (malformed, another unit, several ranges): the
    /// whole file, as RFC 9110 §14.2 allows.
    Ignore,
    /// First and last byte, inclusive.
    Bytes(u64, u64),
    Unsatisfiable,
}

fn parse_range(value: &str, size: u64) -> RangeAsk {
    let Some(spec) = value.trim().strip_prefix("bytes=") else { return RangeAsk::Ignore };
    if spec.contains(',') {
        return RangeAsk::Ignore;
    }
    let Some((first, last)) = spec.trim().split_once('-') else { return RangeAsk::Ignore };
    let (first, last) = (first.trim(), last.trim());
    let number = |s: &str| if s.bytes().all(|b| b.is_ascii_digit()) { s.parse::<u64>().ok() } else { None };
    match (first.is_empty(), last.is_empty()) {
        (true, true) => RangeAsk::Ignore,
        // The last n bytes.
        (true, false) => match number(last) {
            None => RangeAsk::Ignore,
            Some(0) => RangeAsk::Unsatisfiable,
            Some(_) if size == 0 => RangeAsk::Unsatisfiable,
            Some(n) => RangeAsk::Bytes(size.saturating_sub(n), size - 1),
        },
        (false, _) => {
            let Some(start) = number(first) else { return RangeAsk::Ignore };
            let end = if last.is_empty() { Some(u64::MAX) } else { number(last) };
            match end {
                None => RangeAsk::Ignore,
                Some(end) if end < start => RangeAsk::Ignore,
                Some(_) if start >= size => RangeAsk::Unsatisfiable,
                Some(end) => RangeAsk::Bytes(start, end.min(size - 1)),
            }
        }
    }
}

/// The object opened at `offset` for `len` bytes, off the async threads.
async fn open(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    stored: &Stored,
    offset: u64,
    len: u64,
) -> Result<Box<dyn Read + Send>, &'static str> {
    let platform = state.platform.clone();
    let (owner, project, store, key) = (owner.to_string(), project.to_string(), stored.store.clone(), stored.key.clone());
    tokio::task::spawn_blocking(move || {
        let opened = platform.file.open_store(&owner, &project, Some(&store)).map_err(|err| err.code)?;
        opened.fs.open_range(&key, offset, len).map_err(|err| err.code)
    })
    .await
    .unwrap_or(Err("ZEBFS_IO"))
}

/// A blocking reader as a response body, a chunk at a time. A caller that
/// goes away stops the reading.
fn stream(mut reader: Box<dyn Read + Send>) -> Body {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, std::io::Error>>(4);
    tokio::task::spawn_blocking(move || {
        let mut buf = vec![0u8; CHUNK];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.blocking_send(Ok(Bytes::copy_from_slice(&buf[..n]))).is_err() {
                        break;
                    }
                }
                Err(err) => {
                    let _ = tx.blocking_send(Err(err));
                    break;
                }
            }
        }
    });
    Body::from_stream(futures::stream::unfold(rx, |mut rx| async move { rx.recv().await.map(|item| (item, rx)) }))
}

#[cfg(test)]
mod tests {
    use super::{RangeAsk, parse_range};

    #[test]
    fn one_byte_range_is_served_and_anything_else_is_the_whole_file_or_416() {
        assert_eq!(parse_range("bytes=0-99", 1000), RangeAsk::Bytes(0, 99));
        assert_eq!(parse_range("bytes=900-", 1000), RangeAsk::Bytes(900, 999));
        assert_eq!(parse_range("bytes=900-5000", 1000), RangeAsk::Bytes(900, 999));
        assert_eq!(parse_range("bytes=-100", 1000), RangeAsk::Bytes(900, 999));
        assert_eq!(parse_range("bytes=-5000", 1000), RangeAsk::Bytes(0, 999));
        assert_eq!(parse_range("bytes=1000-", 1000), RangeAsk::Unsatisfiable);
        assert_eq!(parse_range("bytes=-0", 1000), RangeAsk::Unsatisfiable);
        assert_eq!(parse_range("bytes=0-", 0), RangeAsk::Unsatisfiable);
        for ignored in ["bytes=0-1,5-6", "items=0-1", "bytes=5-1", "bytes=a-b", "bytes=-", "bytes=+1-2"] {
            assert_eq!(parse_range(ignored, 1000), RangeAsk::Ignore, "{ignored}");
        }
    }
}

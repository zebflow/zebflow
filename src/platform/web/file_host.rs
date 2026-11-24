//! The ZebFS gateway: the only door through which a browser reads a
//! project's stored files without signing in (`docs/contracts/kinds/zebfs-acl`).
//!
//! Two kinds of answer, both decided by the project's exposure rules and
//! nothing else — no folder name, no session cookie:
//!
//! - **The file host** (`<project>.<owner>.fs.localhost` on a dev machine)
//!   serves `public_read` and `public_execute` paths, always inert: the bytes
//!   may be used by a page, never run as one.
//! - **A `serve` origin** answers the one `public_execute` folder that names
//!   it, at `/`, with scripts running. That host answers nothing else.
//!
//! Both read a file's size and date first, then stream only the bytes the
//! answer carries: the whole file, or the one range asked for (206), which
//! is what lets a browser play a video and skip ahead in it. A file is never
//! held in memory.

use std::time::UNIX_EPOCH;

use axum::body::Body;
use axum::http::header::{
    ACCEPT_RANGES, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH,
    LAST_MODIFIED, RANGE,
};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};

use super::stored_answer::{RangeAsk, if_range_holds, parse_range, stream_opened};
use super::{PlatformAppState, content_type_for_path, harden_byte_response, internal_error};
use crate::platform::error::PlatformError;
use crate::zebfs::acl::{serve_mount_path, serve_origin_host, serve_origin_port};
use crate::zebfs::{ZebFs, ZebFsAccess, ZebFsEntryKind, ZebFsStat, normalize_object_path};

fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "There is nothing at this address.").into_response()
}

/// A path with a segment starting with `.` names something no visitor asks
/// for — the `.zebflow-static-site.json` manifest a generator writes names
/// the site's template paths — so it answers like a missing file. Only
/// `/.well-known/…` is a visitor's path (RFC 8615). Read after decoding, so
/// `%2e` is a dot too.
fn hidden_path(decoded: &str) -> bool {
    decoded
        .trim_start_matches('/')
        .split('/')
        .enumerate()
        .any(|(index, segment)| segment.starts_with('.') && !(index == 0 && segment == ".well-known"))
}

/// What a reader may reuse without asking, and for how long.
///
/// `no-cache` is not "do not store": it stores the file and asks before
/// reusing it. With the validator below, an unchanged file costs a 304 and no
/// body, and a rebuilt one is never served from a reader's cache.
///
/// The header this replaced was `public, max-age=300` with no validator, so
/// for five minutes after a build every page, every search chunk and every
/// asset could be the previous build's — at addresses that never change. No
/// path here is content-addressed, so there is nothing that may be held
/// longer: a library's path carries its minor version (`zeb/markdown/0.1/…`)
/// and that is rebuilt in place, so `immutable` would pin a reader to a
/// superseded bundle until the cache evicted it. Long-lived caching wants
/// hashed file names, not a longer guess.
const REVALIDATE: HeaderValue = HeaderValue::from_static("no-cache");

/// A validator for one stored object: its size and the second it was written.
///
/// Weak, because two writes inside one second are indistinguishable, and that
/// is the trade every static file server makes rather than hashing the bytes
/// of every response. It changes whenever a build rewrites the file, which is
/// what this has to catch.
fn object_validator(stat: &crate::zebfs::ZebFsStat) -> Option<(HeaderValue, Option<HeaderValue>)> {
    let modified = stat.modified?;
    let seconds = modified.duration_since(UNIX_EPOCH).ok()?.as_secs();
    let etag = HeaderValue::from_str(&format!("W/\"{}-{}\"", stat.size, seconds)).ok()?;
    Some((etag, http_date(seconds).and_then(|date| HeaderValue::from_str(&date).ok())))
}

/// `Sun, 06 Nov 1994 08:49:37 GMT` — the one date format HTTP requires
/// (RFC 9110 §5.6.7). Civil-date arithmetic, as the sitemap's `lastmod` does,
/// so no date crate is pulled in for one line.
fn http_date(seconds: u64) -> Option<String> {
    const DAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] =
        ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let days = (seconds / 86_400) as i64;
    let time = seconds % 86_400;
    let weekday = DAYS[(days.rem_euclid(7)) as usize];

    // civil_from_days (Howard Hinnant), shifted to a 1 March year start.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    Some(format!(
        "{weekday}, {day:02} {} {year:04} {:02}:{:02}:{:02} GMT",
        MONTHS[(month - 1) as usize],
        time / 3_600,
        (time % 3_600) / 60,
        time % 60,
    ))
}

/// Whether the reader already holds this exact version.
///
/// `If-None-Match` wins over `If-Modified-Since` when both are sent, which is
/// what RFC 9110 §13.1.3 requires: the tag is the precise answer and the date
/// only has one-second resolution.
fn still_fresh(request: &HeaderMap, etag: &HeaderValue, last_modified: Option<&HeaderValue>) -> bool {
    if let Some(sent) = request.get(IF_NONE_MATCH).and_then(|value| value.to_str().ok()) {
        return sent.split(',').any(|candidate| {
            let candidate = candidate.trim();
            candidate == "*" || candidate.as_bytes() == etag.as_bytes()
        });
    }
    match (request.get(IF_MODIFIED_SINCE), last_modified) {
        (Some(sent), Some(known)) => sent.as_bytes() == known.as_bytes(),
        _ => false,
    }
}

/// The answer for one stored file, decided from its size and date before any
/// byte is read: unchanged (304), the one range asked for (206), a range past
/// its end (416), or the whole file. `body` gives the bytes from an offset for
/// a length.
fn object_response(rel: &str, stat: &ZebFsStat, request: &HeaderMap, body: impl FnOnce(u64, u64) -> Body) -> Response {
    let validator = object_validator(stat);
    if let Some((etag, last_modified)) = validator.as_ref()
        && still_fresh(request, etag, last_modified.as_ref())
    {
        let mut resp = Response::new(Body::empty());
        *resp.status_mut() = StatusCode::NOT_MODIFIED;
        write_cache_headers(resp.headers_mut(), validator.as_ref());
        return resp;
    }

    let size = stat.size;
    let (mut status, mut offset, mut len) = (StatusCode::OK, 0, size);
    let etag = validator.as_ref().and_then(|(etag, _)| etag.to_str().ok()).unwrap_or("");
    let last_modified = validator.as_ref().and_then(|(_, date)| date.as_ref()).and_then(|date| date.to_str().ok());
    if let Some(range) = request.get(RANGE).and_then(|value| value.to_str().ok())
        && if_range_holds(request, etag, last_modified)
    {
        match parse_range(range, size) {
            RangeAsk::Ignore => {}
            RangeAsk::Bytes(start, end) => (status, offset, len) = (StatusCode::PARTIAL_CONTENT, start, end - start + 1),
            RangeAsk::Unsatisfiable => {
                let mut resp = Response::new(Body::empty());
                *resp.status_mut() = StatusCode::RANGE_NOT_SATISFIABLE;
                write_cache_headers(resp.headers_mut(), validator.as_ref());
                if let Ok(value) = HeaderValue::from_str(&format!("bytes */{size}")) {
                    resp.headers_mut().insert(CONTENT_RANGE, value);
                }
                return resp;
            }
        }
    }

    let mut resp = Response::new(body(offset, len));
    *resp.status_mut() = status;
    let headers = resp.headers_mut();
    if let Ok(value) = HeaderValue::from_str(content_type_for_path(std::path::Path::new(rel))) {
        headers.insert(CONTENT_TYPE, value);
    }
    write_cache_headers(headers, validator.as_ref());
    headers.insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    headers.insert(CONTENT_LENGTH, HeaderValue::from(len));
    if status == StatusCode::PARTIAL_CONTENT
        && let Ok(value) = HeaderValue::from_str(&format!("bytes {offset}-{}/{size}", offset + len - 1))
    {
        headers.insert(CONTENT_RANGE, value);
    }
    resp
}

/// `len` bytes of `rel` from `offset`, streamed from the store.
fn stored_body(zebfs: &ZebFs, rel: &str) -> impl FnOnce(u64, u64) -> Body {
    let (zebfs, rel) = (zebfs.clone(), rel.to_string());
    move |offset, len| {
        stream_opened(move || {
            zebfs.open_range(&rel, offset, len).map_err(|err| std::io::Error::other(format!("{}: {}", err.code, err.message)))
        })
    }
}

/// The stat of a stored file; a folder or a missing path is `None`.
fn file_stat(zebfs: &ZebFs, rel: &str) -> Result<Option<ZebFsStat>, crate::zebfs::ZebFsError> {
    match zebfs.head(rel) {
        Ok(stat) if stat.kind == ZebFsEntryKind::Object => Ok(Some(stat)),
        Ok(_) => Ok(None),
        Err(err) if err.code == "ZEBFS_NOT_FOUND" || err.code == "ZEBFS_INVALID_PATH" => Ok(None),
        Err(err) => Err(err),
    }
}

fn write_cache_headers(headers: &mut HeaderMap, validator: Option<&(HeaderValue, Option<HeaderValue>)>) {
    headers.insert(CACHE_CONTROL, REVALIDATE);
    if let Some((etag, last_modified)) = validator {
        headers.insert(ETAG, etag.clone());
        if let Some(last_modified) = last_modified {
            headers.insert(LAST_MODIFIED, last_modified.clone());
        }
    }
}

/// A request on the project's file host. An unexposed path answers exactly
/// like a missing one, so the host never confirms that a private file exists.
pub(super) async fn file_host_response(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    path: &str,
    request: &HeaderMap,
) -> Response {
    let decoded = percent_decode(path);
    if hidden_path(&decoded) {
        return not_found();
    }
    let Ok(rel) = normalize_object_path(&decoded) else {
        return not_found();
    };
    if !super::project_exists_or_on_disk(state, owner, project) {
        return not_found();
    }
    let layout = match state.platform.file.ensure_project_layout(owner, project) {
        Ok(layout) => layout,
        Err(err) => return internal_error(err),
    };
    match crate::platform::services::zebfs_acl::effective_access(&layout.data_store_dir(), &rel) {
        Ok(access) if access.is_exposed() => {}
        Ok(_) => return not_found(),
        Err(err) => return internal_error(PlatformError::new(err.code, err.message)),
    }
    let zebfs = layout.open_files();
    match file_stat(&zebfs, &rel) {
        Ok(Some(stat)) => harden_byte_response(object_response(&rel, &stat, request, stored_body(&zebfs, &rel))).await,
        Ok(None) => not_found(),
        Err(err) => internal_error(PlatformError::new(err.code, err.message)),
    }
}

/// A request on one of the project's hosts that a `public_execute` folder
/// answers, or `None` when no folder claims it and the project's pages do.
///
/// - A `serve` entry with no mount path gives its folder the whole host:
///   every path answers from it.
/// - Entries with a mount path (`https://example.com/docs/`) answer those
///   paths and leave every other path to the pages: the longest mount wins,
///   `/docs` redirects to `/docs/`, and a missing file is a 404, never a page.
///
/// GET and HEAD only. An unreadable rules document means everything is
/// private (kinds/zebfs-acl §Resolution): no folder answers, the pages do.
#[allow(clippy::too_many_arguments)]
pub(super) fn execute_site_response(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    request_host: &str,
    request_port: Option<&str>,
    method: &Method,
    path: &str,
    query: Option<&str>,
    request: &HeaderMap,
) -> Option<Response> {
    if !super::project_exists_or_on_disk(state, owner, project) {
        return None;
    }
    let layout = state.platform.file.ensure_project_layout(owner, project).ok()?;
    let manifest = crate::platform::services::zebfs_acl::read(&layout.data_store_dir()).ok()?;
    let mut claims: Vec<(&str, &str)> = manifest
        .rules
        .iter()
        .filter(|(_, rule)| rule.access == ZebFsAccess::PublicExecute)
        .flat_map(|(folder, rule)| rule.serve.iter().map(move |entry| (folder.as_str(), entry.as_str())))
        .filter(|(_, entry)| origin_matches(entry, request_host, request_port))
        .map(|(folder, entry)| (folder, serve_mount_path(entry)))
        .collect();
    if claims.is_empty() {
        return None;
    }
    // Only a request the folder answers is held to GET and HEAD: a host with
    // a mounted path still takes its pages' other methods everywhere else.
    let readable = method == Method::GET || method == Method::HEAD;
    let decoded = percent_decode(path);
    let (folder, mount) = match claims.iter().find(|(_, mount)| *mount == "/") {
        Some(&whole_host) => {
            if !readable {
                return Some(StatusCode::METHOD_NOT_ALLOWED.into_response());
            }
            whole_host
        }
        None => {
            claims.sort_by_key(|(_, mount)| std::cmp::Reverse(mount.len()));
            let under = |path: &str, mount: &str| path.starts_with(mount) || path == mount.trim_end_matches('/');
            let Some(&(folder, mount)) = claims.iter().find(|(_, mount)| under(&decoded, mount)) else {
                // `//learn/x` or `/./learn/x` reads as the mount once its empty
                // and `.` segments go: refused, never handed to the pages.
                if ambiguous_path(path, &decoded) && claims.iter().any(|(_, mount)| under(&squeezed(&decoded), mount)) {
                    return Some((StatusCode::BAD_REQUEST, "This address can be read more than one way.").into_response());
                }
                return None;
            };
            if !readable {
                return Some(StatusCode::METHOD_NOT_ALLOWED.into_response());
            }
            if ambiguous_path(path, &decoded) {
                return Some((StatusCode::BAD_REQUEST, "This address can be read more than one way.").into_response());
            }
            // A worker installed from a mounted folder would keep answering in
            // visitors' browsers after the folder changes, on the site's own
            // origin. Mounted folders serve pages, not workers.
            if request.contains_key("service-worker") {
                return Some(not_found());
            }
            if decoded == mount.trim_end_matches('/') {
                // The folder's pages link relative to the mount, so its root
                // is the path with the slash.
                let location = match query.filter(|q| !q.is_empty()) {
                    Some(q) => format!("{mount}?{q}"),
                    None => mount.to_string(),
                };
                return Some(axum::response::Redirect::permanent(&location).into_response());
            }
            (folder, mount)
        }
    };

    let request_rel = decoded.get(mount.len()..).unwrap_or("").trim_start_matches('/').to_string();
    if hidden_path(&request_rel) {
        return Some(not_found());
    }
    let zebfs = layout.open_files();
    // A folder asked for without its last `/` is redirected to it, as static
    // hosts do: its page names its scripts and assets relative to itself
    // (`./_assets/…`), and from `/docs/guide` they would resolve one folder up.
    let folder_page = (!request_rel.is_empty() && !request_rel.ends_with('/'))
        .then(|| format!("{folder}/{}/index.html", request_rel.trim_end_matches('/')));
    for candidate in site_candidates(folder, &request_rel) {
        let Ok(rel) = normalize_object_path(&candidate) else {
            continue;
        };
        // A nested rule may make part of the folder private again; the
        // longest match decides, here as everywhere, and it must still be
        // served at this address.
        match manifest.effective_rule(&rel) {
            Ok(Some((_, rule)))
                if rule.access == ZebFsAccess::PublicExecute
                    && rule
                        .serve
                        .iter()
                        .any(|entry| origin_matches(entry, request_host, request_port) && serve_mount_path(entry) == mount) => {}
            _ => continue,
        }
        if folder_page.as_deref().and_then(|page| normalize_object_path(page).ok()).as_deref() == Some(rel.as_str()) && zebfs.head(&rel).is_ok() {
            let location = match query.filter(|q| !q.is_empty()) {
                Some(q) => format!("{}/?{q}", path.trim_end_matches('/')),
                None => format!("{}/", path.trim_end_matches('/')),
            };
            return Some(axum::response::Redirect::permanent(&location).into_response());
        }
        if let Ok(Some(stat)) = file_stat(&zebfs, &rel) {
            let mut resp = object_response(&rel, &stat, request, stored_body(&zebfs, &rel));
            resp.headers_mut().insert(
                "x-content-type-options",
                HeaderValue::from_static("nosniff"),
            );
            return Some(resp);
        }
    }
    Some(not_found())
}

/// `decoded` without empty or `.` segments, keeping a trailing `/`.
fn squeezed(decoded: &str) -> String {
    let segments: Vec<&str> = decoded.split('/').filter(|s| !s.is_empty() && *s != ".").collect();
    let tail = if decoded.ends_with('/') && !segments.is_empty() { "/" } else { "" };
    format!("/{}{tail}", segments.join("/"))
}

/// An encoded `/`, `\` or NUL, a doubled `/`, or a `.` or `..` segment: a path
/// the mount and the store could read two ways. Refused, never guessed.
fn ambiguous_path(raw: &str, decoded: &str) -> bool {
    let lower = raw.to_ascii_lowercase();
    lower.contains("%2f")
        || lower.contains("%5c")
        || lower.contains("%00")
        || decoded.contains('\\')
        || decoded.contains('\0')
        || decoded.contains("//")
        || decoded.split('/').any(|segment| segment == "." || segment == "..")
}

fn site_candidates(folder: &str, request_rel: &str) -> Vec<String> {
    let base = if request_rel.is_empty() {
        folder.to_string()
    } else {
        format!("{folder}/{}", request_rel.trim_end_matches('/'))
    };
    let mut out = Vec::new();
    if !request_rel.is_empty() && !request_rel.ends_with('/') {
        out.push(base.clone());
    }
    out.push(format!("{base}/index.html"));
    if !request_rel.is_empty() && !request_rel.ends_with('/') {
        out.push(format!("{base}.html"));
    }
    out
}

/// The host the request was routed by against one normal-form origin: the
/// host must match, and so must the port when the origin names one.
fn origin_matches(origin: &str, request_host: &str, request_port: Option<&str>) -> bool {
    if serve_origin_host(origin) != request_host {
        return false;
    }
    match serve_origin_port(origin) {
        Some(port) => request_port == Some(port),
        None => true,
    }
}

fn percent_decode(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        // Read on bytes: a `%` before a multi-byte character is not an
        // escape, and slicing the str there would split the character.
        if bytes[i] == b'%'
            && i + 3 <= bytes.len()
            && let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3])
            && let Ok(value) = u8::from_str_radix(hex, 16)
        {
            out.push(value);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::{
        file_stat, hidden_path, http_date, object_response, object_validator, origin_matches, percent_decode,
        site_candidates, still_fresh,
    };
    use axum::body::Body;
    use axum::http::header::{
        ACCEPT_RANGES, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_RANGE, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, IF_RANGE,
        LAST_MODIFIED, RANGE,
    };
    use axum::http::{HeaderMap, HeaderValue, StatusCode};
    use std::time::{Duration, UNIX_EPOCH};

    /// A body cut from `bytes`, as the store would stream it.
    fn from_bytes(bytes: &'static [u8]) -> impl FnOnce(u64, u64) -> Body {
        move |offset, len| Body::from(bytes[offset as usize..(offset + len) as usize].to_vec())
    }

    async fn body_of(response: axum::response::Response) -> Vec<u8> {
        axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("body").to_vec()
    }

    fn asking(pairs: &[(axum::http::HeaderName, &'static str)]) -> HeaderMap {
        let mut request = HeaderMap::new();
        for (name, value) in pairs {
            request.insert(name.clone(), HeaderValue::from_static(value));
        }
        request
    }

    fn stat(size: u64, seconds: u64) -> crate::zebfs::ZebFsStat {
        crate::zebfs::ZebFsStat {
            path: "site/index.html".to_string(),
            size,
            modified: Some(UNIX_EPOCH + Duration::from_secs(seconds)),
            kind: crate::zebfs::ZebFsEntryKind::Object,
        }
    }

    fn header(response: &axum::response::Response, name: axum::http::HeaderName) -> String {
        response.headers().get(name).and_then(|v| v.to_str().ok()).unwrap_or_default().to_string()
    }

    #[test]
    fn a_dot_segment_is_hidden_except_well_known() {
        assert!(hidden_path("/.zebflow-static-site.json"));
        assert!(hidden_path("/docs/.zebflow-static-site.json"));
        assert!(hidden_path(&percent_decode("/%2Ezebflow-static-site.json")));
        assert!(hidden_path("/a/.git/config"));
        assert!(hidden_path("/.well-known/.hidden"));
        assert!(!hidden_path("/.well-known/security.txt"));
        assert!(!hidden_path("/docs/v1.2/index.html"));
        assert!(!hidden_path("/"));
        assert!(hidden_path(&percent_decode("/a/%2e")), "an escape at the very end decodes too");
        assert_eq!(percent_decode("/caf%C3%A9/%é"), "/café/%é", "a % before a multi-byte character is kept");
    }

    #[test]
    fn a_site_path_tries_the_object_then_its_index_then_html() {
        assert_eq!(site_candidates("site", ""), vec!["site/index.html"]);
        assert_eq!(
            site_candidates("site", "artists/k"),
            vec!["site/artists/k", "site/artists/k/index.html", "site/artists/k.html"]
        );
        assert_eq!(site_candidates("site", "docs/"), vec!["site/docs/index.html"]);
    }

    #[test]
    fn origins_match_on_host_and_on_port_when_named() {
        assert!(origin_matches("https://site.example/", "site.example", None));
        assert!(origin_matches("https://site.example/", "site.example", Some("443")));
        assert!(!origin_matches("https://site.example/", "www.site.example", None));
        assert!(origin_matches("http://d.o.localhost:10610/", "d.o.localhost", Some("10610")));
        assert!(!origin_matches("http://d.o.localhost:10610/", "d.o.localhost", Some("8080")));
    }

    /// A page at an address that never changes has to be checked before it is
    /// reused, or a build is invisible to anyone who read the site before it.
    #[test]
    fn a_served_file_carries_a_validator_and_asks_before_it_is_reused() {
        const PAGE: [u8; 1234] = [b' '; 1234];
        let stat = stat(1234, 1_762_000_000);
        let response = object_response("site/index.html", &stat, &HeaderMap::new(), from_bytes(&PAGE));
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(header(&response, CACHE_CONTROL), "no-cache");
        assert_eq!(header(&response, ETAG), "W/\"1234-1762000000\"");
        assert!(!header(&response, LAST_MODIFIED).is_empty());
    }

    #[test]
    fn a_reader_holding_this_version_is_told_so_and_sent_no_body() {
        let stat = stat(1234, 1_762_000_000);
        let mut request = HeaderMap::new();
        request.insert(IF_NONE_MATCH, HeaderValue::from_static("W/\"1234-1762000000\""));
        let response = object_response("site/index.html", &stat, &request, from_bytes(b"<html></html>"));
        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(header(&response, ETAG), "W/\"1234-1762000000\"");
    }

    /// The whole point: after a build the reader's copy stops matching.
    #[test]
    fn a_rebuilt_file_does_not_match_the_tag_the_reader_holds() {
        let before = object_validator(&stat(1234, 1_762_000_000)).expect("validator").0;
        let after = object_validator(&stat(1200, 1_762_000_900)).expect("validator").0;
        assert_ne!(before, after);
        let mut request = HeaderMap::new();
        request.insert(IF_NONE_MATCH, before.clone());
        assert!(!still_fresh(&request, &after, None));
        assert!(still_fresh(&request, &before, None));
    }

    /// A tag is exact and a date is not, so the tag decides when both are sent
    /// (RFC 9110 13.1.3).
    #[test]
    fn a_tag_decides_over_a_date() {
        let (etag, last_modified) = object_validator(&stat(10, 1_762_000_000)).expect("validator");
        let last_modified = last_modified.expect("a date");
        let mut request = HeaderMap::new();
        request.insert(IF_MODIFIED_SINCE, last_modified.clone());
        request.insert(IF_NONE_MATCH, HeaderValue::from_static("W/\"999-1\""));
        assert!(!still_fresh(&request, &etag, Some(&last_modified)));
    }

    const CLIP: &[u8] = b"0123456789abcdefghij";

    fn clip_stat() -> crate::zebfs::ZebFsStat {
        crate::zebfs::ZebFsStat { path: "wiki/clip.mp4".to_string(), ..stat(CLIP.len() as u64, 1_762_000_000) }
    }

    /// A video plays on iPhone Safari, and skips ahead anywhere, only when a
    /// range is answered with exactly those bytes and a 206.
    #[tokio::test]
    async fn a_range_is_answered_with_those_bytes_and_206() {
        for (asked, content_range, bytes) in [
            ("bytes=0-1", "bytes 0-1/20", &b"01"[..]),
            ("bytes=10-", "bytes 10-19/20", &b"abcdefghij"[..]),
            ("bytes=-3", "bytes 17-19/20", &b"hij"[..]),
            ("bytes=15-999", "bytes 15-19/20", &b"fghij"[..]),
        ] {
            let request = asking(&[(RANGE, asked)]);
            let response = object_response("wiki/clip.mp4", &clip_stat(), &request, from_bytes(CLIP));
            assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT, "{asked}");
            assert_eq!(header(&response, CONTENT_RANGE), content_range, "{asked}");
            assert_eq!(header(&response, CONTENT_LENGTH), bytes.len().to_string(), "{asked}");
            assert_eq!(header(&response, ACCEPT_RANGES), "bytes");
            assert_eq!(header(&response, ETAG), "W/\"20-1762000000\"");
            assert_eq!(body_of(response).await, bytes, "{asked}");
        }
    }

    #[tokio::test]
    async fn no_range_is_the_whole_file_and_says_ranges_are_accepted() {
        let response = object_response("wiki/clip.mp4", &clip_stat(), &HeaderMap::new(), from_bytes(CLIP));
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(header(&response, ACCEPT_RANGES), "bytes");
        assert_eq!(header(&response, CONTENT_LENGTH), "20");
        assert!(header(&response, CONTENT_RANGE).is_empty());
        assert_eq!(body_of(response).await, CLIP);
    }

    #[test]
    fn a_range_past_the_end_is_416_naming_the_size() {
        let request = asking(&[(RANGE, "bytes=20-")]);
        let response = object_response("wiki/clip.mp4", &clip_stat(), &request, from_bytes(CLIP));
        assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);
        assert_eq!(header(&response, CONTENT_RANGE), "bytes */20");
    }

    /// RFC 9110 14.2: a range we do not serve (several ranges, another unit)
    /// may be answered with the whole file.
    #[tokio::test]
    async fn several_ranges_or_another_unit_get_the_whole_file() {
        for asked in ["bytes=0-1,5-6", "items=0-1"] {
            let request = asking(&[(RANGE, asked)]);
            let response = object_response("wiki/clip.mp4", &clip_stat(), &request, from_bytes(CLIP));
            assert_eq!(response.status(), StatusCode::OK, "{asked}");
            assert_eq!(body_of(response).await, CLIP, "{asked}");
        }
    }

    /// `If-Range` resumes only the exact version: our tag is weak, so a tag
    /// never qualifies; our very `Last-Modified` does.
    #[tokio::test]
    async fn if_range_resumes_only_on_our_exact_date() {
        let response = object_response(
            "wiki/clip.mp4",
            &clip_stat(),
            &asking(&[(RANGE, "bytes=0-1"), (IF_RANGE, "W/\"20-1762000000\"")]),
            from_bytes(CLIP),
        );
        assert_eq!(response.status(), StatusCode::OK);
        let date = header(
            &object_response("wiki/clip.mp4", &clip_stat(), &HeaderMap::new(), from_bytes(CLIP)),
            LAST_MODIFIED,
        );
        let mut request = asking(&[(RANGE, "bytes=0-1")]);
        request.insert(IF_RANGE, HeaderValue::from_str(&date).expect("date"));
        let response = object_response("wiki/clip.mp4", &clip_stat(), &request, from_bytes(CLIP));
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        request.insert(IF_RANGE, HeaderValue::from_static("Thu, 01 Jan 1970 00:00:00 GMT"));
        let response = object_response("wiki/clip.mp4", &clip_stat(), &request, from_bytes(CLIP));
        assert_eq!(response.status(), StatusCode::OK);
    }

    /// An unchanged file is a 304 even when a range is asked: the reader's
    /// copy is already the whole thing.
    #[test]
    fn an_unchanged_file_is_304_before_any_range() {
        let request = asking(&[(RANGE, "bytes=0-1"), (IF_NONE_MATCH, "W/\"20-1762000000\"")]);
        let response = object_response("wiki/clip.mp4", &clip_stat(), &request, from_bytes(CLIP));
        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    }

    /// The stat comes first now, so a folder must not pass for an empty file.
    #[test]
    fn only_a_stored_file_has_a_stat_to_serve() {
        let dir = tempfile::tempdir().expect("tempdir");
        let zebfs = crate::zebfs::ZebFs::Local(crate::zebfs::LocalZebFs::new(dir.path().to_path_buf()));
        zebfs.put("wiki/clip.mp4", CLIP).expect("put");
        assert_eq!(file_stat(&zebfs, "wiki/clip.mp4").expect("stat").map(|s| s.size), Some(20));
        assert!(file_stat(&zebfs, "wiki").expect("stat").is_none(), "a folder");
        assert!(file_stat(&zebfs, "wiki/none.mp4").expect("stat").is_none(), "a missing file");
    }

    #[test]
    fn a_date_is_written_the_one_way_http_accepts() {
        assert_eq!(http_date(784_111_777).as_deref(), Some("Sun, 06 Nov 1994 08:49:37 GMT"));
        assert_eq!(http_date(0).as_deref(), Some("Thu, 01 Jan 1970 00:00:00 GMT"));
    }

    #[test]
    fn encoded_paths_decode_before_matching() {
        assert_eq!(percent_decode("/a%20b.png"), "/a b.png");
        assert_eq!(percent_decode("/%2e%2e/x"), "/../x");
    }
}

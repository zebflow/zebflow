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

use std::time::UNIX_EPOCH;

use axum::body::Body;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

use super::{PlatformAppState, content_type_for_path, harden_byte_response, internal_error};
use crate::platform::error::PlatformError;
use crate::zebfs::acl::serve_origin_host;
use crate::zebfs::{ZebFsAccess, normalize_object_path};

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

fn object_response(rel: &str, bytes: Vec<u8>, stat: &crate::zebfs::ZebFsStat, request: &HeaderMap) -> Response {
    let validator = object_validator(stat);
    if let Some((etag, last_modified)) = validator.as_ref()
        && still_fresh(request, etag, last_modified.as_ref())
    {
        let mut resp = Response::new(Body::empty());
        *resp.status_mut() = StatusCode::NOT_MODIFIED;
        write_cache_headers(resp.headers_mut(), validator.as_ref());
        return resp;
    }

    let mut resp = Response::new(Body::from(bytes));
    *resp.status_mut() = StatusCode::OK;
    if let Ok(value) = HeaderValue::from_str(content_type_for_path(std::path::Path::new(rel))) {
        resp.headers_mut().insert(CONTENT_TYPE, value);
    }
    write_cache_headers(resp.headers_mut(), validator.as_ref());
    resp
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
    let layout = match state.platform.file.ensure_project_layout(owner, project) {
        Ok(layout) => layout,
        Err(err) => return internal_error(err),
    };
    match crate::platform::services::zebfs_acl::effective_access(&layout.data_store_dir(), &rel) {
        Ok(access) if access.is_exposed() => {}
        Ok(_) => return not_found(),
        Err(err) => return internal_error(PlatformError::new(err.code, err.message)),
    }
    match layout.open_files().get(&rel) {
        Ok(object) => harden_byte_response(object_response(&rel, object.bytes, &object.stat, request)).await,
        Err(err) if err.code == "ZEBFS_NOT_FOUND" || err.code == "ZEBFS_INVALID_PATH" => not_found(),
        Err(err) => internal_error(PlatformError::new(err.code, err.message)),
    }
}

/// A request on one of the project's hosts: the `public_execute` folder whose
/// `serve` names this host answers it, or `None` when no rule names it.
pub(super) fn execute_site_response(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    request_host: &str,
    request_port: Option<&str>,
    path: &str,
    request: &HeaderMap,
) -> Option<Response> {
    let layout = state.platform.file.ensure_project_layout(owner, project).ok()?;
    let store_dir = layout.data_store_dir();
    let manifest = crate::platform::services::zebfs_acl::read(&store_dir).ok()?;
    let folder = manifest.rules.iter().find_map(|(folder, rule)| {
        (rule.access == ZebFsAccess::PublicExecute
            && rule.serve.iter().any(|origin| origin_matches(origin, request_host, request_port)))
        .then(|| folder.clone())
    })?;

    let request_rel = percent_decode(path);
    if hidden_path(&request_rel) {
        return Some(not_found());
    }
    let request_rel = request_rel.trim_start_matches('/');
    let zebfs = layout.open_files();
    for candidate in site_candidates(&folder, request_rel) {
        let Ok(rel) = normalize_object_path(&candidate) else {
            continue;
        };
        // A nested rule may make part of the folder private again; the
        // longest match decides, here as everywhere.
        match manifest.effective_rule(&rel) {
            Ok(Some((_, rule)))
                if rule.access == ZebFsAccess::PublicExecute
                    && rule.serve.iter().any(|origin| origin_matches(origin, request_host, request_port)) => {}
            _ => continue,
        }
        if let Ok(object) = zebfs.get(&rel) {
            let mut resp = object_response(&rel, object.bytes, &object.stat, request);
            resp.headers_mut().insert(
                "x-content-type-options",
                HeaderValue::from_static("nosniff"),
            );
            return Some(resp);
        }
    }
    Some(not_found())
}

/// The objects a site path may name, in order: the exact object, then the
/// folder's `index.html`, then `<path>.html`.
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
    let authority = origin
        .split_once("://")
        .map(|(_, rest)| rest.trim_end_matches('/'))
        .unwrap_or(origin);
    match authority.rsplit_once(':') {
        Some((_, port)) => request_port == Some(port),
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
        hidden_path, http_date, object_response, object_validator, origin_matches, percent_decode,
        site_candidates, still_fresh,
    };
    use axum::http::header::{CACHE_CONTROL, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
    use axum::http::{HeaderMap, HeaderValue, StatusCode};
    use std::time::{Duration, UNIX_EPOCH};

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
        let stat = stat(1234, 1_762_000_000);
        let response = object_response("site/index.html", b"<html></html>".to_vec(), &stat, &HeaderMap::new());
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
        let response = object_response("site/index.html", b"<html></html>".to_vec(), &stat, &request);
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

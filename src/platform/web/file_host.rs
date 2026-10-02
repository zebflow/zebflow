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

use axum::body::Body;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

use super::{PlatformAppState, content_type_for_path, harden_byte_response, internal_error};
use crate::platform::error::PlatformError;
use crate::zebfs::acl::serve_origin_host;
use crate::zebfs::{ZebFsAccess, normalize_object_path};

fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "There is nothing at this address.").into_response()
}

fn object_response(rel: &str, bytes: Vec<u8>) -> Response {
    let mut resp = Response::new(Body::from(bytes));
    *resp.status_mut() = StatusCode::OK;
    if let Ok(value) = HeaderValue::from_str(content_type_for_path(std::path::Path::new(rel))) {
        resp.headers_mut().insert(CONTENT_TYPE, value);
    }
    resp.headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("public, max-age=300"));
    resp
}

/// A request on the project's file host. An unexposed path answers exactly
/// like a missing one, so the host never confirms that a private file exists.
pub(super) async fn file_host_response(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    path: &str,
) -> Response {
    let Ok(rel) = normalize_object_path(&percent_decode(path)) else {
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
        Ok(object) => harden_byte_response(object_response(&rel, object.bytes)).await,
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
            let mut resp = object_response(&rel, object.bytes);
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
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(value) = u8::from_str_radix(&path[i + 1..i + 3], 16)
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
    use super::{origin_matches, percent_decode, site_candidates};

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

    #[test]
    fn encoded_paths_decode_before_matching() {
        assert_eq!(percent_decode("/a%20b.png"), "/a b.png");
        assert_eq!(percent_decode("/%2e%2e/x"), "/../x");
    }
}

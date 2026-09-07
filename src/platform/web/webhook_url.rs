//! Preserve the browser URL across public and worker webhook ingress.
//!
//! Axum's extracted tail is decoded and is suitable for pipeline matching. URL
//! transport and SSR need the raw path instead, including escaped delimiters
//! and trailing slashes. Only the transport prefix changes on a worker hop.

use axum::http::Uri;

use crate::platform::error::PlatformError;

const PUBLIC_PREFIX: &str = "/wh/";
const WORKER_PREFIX: &str = "/api/internal/runtime/webhook/";

/// Browser-facing pathname for a public or authenticated worker request.
pub(super) fn pathname(uri: &Uri) -> String {
    match uri.path().strip_prefix(WORKER_PREFIX) {
        Some(suffix) => format!("{PUBLIC_PREFIX}{suffix}"),
        None => uri.path().to_string(),
    }
}

/// Map webhook transport without decoding or trimming the path or query.
pub(super) fn worker_path_and_query(uri: &Uri) -> Result<String, PlatformError> {
    let suffix = uri
        .path()
        .strip_prefix(PUBLIC_PREFIX)
        .or_else(|| uri.path().strip_prefix(WORKER_PREFIX))
        .ok_or_else(|| {
            PlatformError::new("CLUSTER_WORKER_PROXY", "request is not a webhook URL")
        })?;
    let mut path = format!("{WORKER_PREFIX}{suffix}");
    if let Some(query) = uri.query() {
        path.push('?');
        path.push_str(query);
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_round_trip_preserves_raw_browser_path_and_query() {
        for raw in [
            "/wh/owner/project",
            "/wh/owner/project/",
            "/wh/owner/project/a%3Fb%23c%2Fd/?tag=one&tag=two&q=caf%C3%A9+tea",
            "/wh/owner/project/a//b/?",
        ] {
            let public: Uri = raw.parse().unwrap();
            let forwarded: Uri = worker_path_and_query(&public).unwrap().parse().unwrap();
            assert_eq!(pathname(&public), public.path());
            assert_eq!(pathname(&forwarded), public.path());
            assert_eq!(forwarded.query(), public.query());
            assert_eq!(
                worker_path_and_query(&forwarded).unwrap(),
                forwarded.to_string()
            );
        }
    }

    #[test]
    fn forwarding_refuses_non_webhook_paths() {
        assert!(worker_path_and_query(&"/projects/owner/project".parse().unwrap()).is_err());
    }
}

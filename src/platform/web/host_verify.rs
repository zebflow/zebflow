//! Settings → Addressing → Verify, without a header that names the project.
//!
//! A public response never says which owner and project answered it. To prove
//! a host reaches *this* project, Verify mints a one-time token for the
//! project and requests `/_verify/{token}` through the host; the addressing
//! gate answers `204` only when the host resolved to the project the token
//! was minted for. `/_verify` alone answers `204` on any project host — the
//! check a generated proxy config ends with: the request reached Zebflow with
//! its Host header intact. `/_` is reserved on project hosts, so no app route
//! can collide.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

pub(super) const PREFIX: &str = "/_verify";
/// How long a minted token answers; Verify uses it at once.
const LIFETIME: Duration = Duration::from_secs(60);

static TOKENS: Mutex<Option<HashMap<String, (String, String, Instant)>>> = Mutex::new(None);

/// A token that proves `owner/project` once, within [`LIFETIME`].
pub(super) fn mint(owner: &str, project: &str) -> String {
    let token = uuid::Uuid::new_v4().simple().to_string();
    let now = Instant::now();
    let mut guard = TOKENS.lock().unwrap_or_else(|e| e.into_inner());
    let tokens = guard.get_or_insert_with(HashMap::new);
    tokens.retain(|_, (_, _, at)| now.duration_since(*at) < LIFETIME);
    tokens.insert(token.clone(), (owner.to_string(), project.to_string(), now));
    token
}

/// The gate's answer to `/_verify[/{token}]` on a host of `owner/project`:
/// 204 for the bare probe or a live token of this project (used up), 404
/// with `x-zebflow-verify: other` for anything else.
pub(super) fn answer(path: &str, owner: &str, project: &str) -> Response {
    let rest = path.strip_prefix(PREFIX).unwrap_or_default().trim_matches('/');
    if rest.is_empty() {
        return StatusCode::NO_CONTENT.into_response();
    }
    let mut guard = TOKENS.lock().unwrap_or_else(|e| e.into_inner());
    let found = guard.as_mut().and_then(|tokens| tokens.remove(rest));
    match found {
        Some((o, p, at)) if o == owner && p == project && at.elapsed() < LIFETIME => StatusCode::NO_CONTENT.into_response(),
        _ => {
            let mut resp = StatusCode::NOT_FOUND.into_response();
            resp.headers_mut().insert("x-zebflow-verify", HeaderValue::from_static("other"));
            resp
        }
    }
}

/// Whether a path is the probe.
pub(super) fn is_probe(path: &str) -> bool {
    path == PREFIX || path.starts_with(&format!("{PREFIX}/"))
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::{answer, mint};

    #[test]
    fn a_token_proves_its_own_project_once() {
        let token = mint("site-a", "demo");
        assert_eq!(answer(&format!("/_verify/{token}"), "site-a", "catalog").status(), StatusCode::NOT_FOUND, "another project");
        let token = mint("site-a", "demo");
        assert_eq!(answer(&format!("/_verify/{token}"), "site-a", "demo").status(), StatusCode::NO_CONTENT);
        assert_eq!(answer(&format!("/_verify/{token}"), "site-a", "demo").status(), StatusCode::NOT_FOUND, "used up");
        assert_eq!(answer("/_verify", "site-a", "demo").status(), StatusCode::NO_CONTENT);
        assert_eq!(answer("/_verify/unknown", "site-a", "demo").status(), StatusCode::NOT_FOUND);
    }
}

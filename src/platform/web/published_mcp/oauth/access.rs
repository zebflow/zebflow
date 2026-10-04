//! An `oauth` route's access tokens: the app's JWTs (RFC 9068 shape),
//! signed with the route's `--credential` and good for that one route.
//!
//! Claims are the approved app token's — `sub`, `roles` and whatever the
//! app signed — without its own `iat exp nbf iss aud jti _zf_public`, plus
//! `iss` and `aud` (both the connect URL), `iat`, `exp` (an hour), `jti`,
//! `client_id` and `scope`. The header says `typ: at+jwt`. Because `aud` is
//! required here and refused by a webhook's `--auth jwt`, an access token
//! never opens a `jwt` webhook, and the app's own session token (no `aud`,
//! no `at+jwt`) never opens an `oauth` route.

use jsonwebtoken::{Header, Validation};
use serde_json::{Map, Value, json};

use crate::pipeline::nodes::basic::auth::sign;
use crate::platform::services::published_oauth::{ACCESS_TTL, now_unix};

/// The claims of the app's token that the access token does not carry over.
const DROPPED: [&str; 7] = ["iat", "exp", "nbf", "iss", "aud", "jti", "_zf_public"];
const TYP: &str = "at+jwt";

/// Signs an access token for `claims` (the approval's snapshot).
pub(super) fn mint(secret: &Value, claims: &Value, issuer: &str, client_id: &str, scope: &str) -> Result<String, String> {
    let (algorithm, key) = sign::encoding_key(secret).map_err(|e| e.to_string())?;
    let mut out: Map<String, Value> = claims
        .as_object()
        .map(|m| m.iter().filter(|(k, _)| !DROPPED.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
    let now = now_unix();
    for (key, value) in [
        ("iss", json!(issuer)),
        ("aud", json!(issuer)),
        ("iat", json!(now)),
        ("exp", json!(now + ACCESS_TTL as i64)),
        ("jti", json!(uuid::Uuid::new_v4().simple().to_string())),
        ("client_id", json!(client_id)),
        ("scope", json!(scope)),
    ] {
        out.insert(key.to_string(), value);
    }
    let mut header = Header::new(algorithm);
    header.typ = Some(TYP.to_string());
    jsonwebtoken::encode(&header, &Value::Object(out), &key).map_err(|e| e.to_string())
}

/// Why an access token was refused.
#[derive(Debug)]
pub(in crate::platform::web) enum AccessRefusal {
    /// Missing, malformed, expired, or for another route: 401.
    Token(String),
    /// The credential is unusable: the operator's problem.
    Key(String),
}

/// The claims of a valid access token for `resource`: the credential's
/// algorithm and signature, `exp` (and `nbf`), `aud == resource`,
/// `iss == resource`, `typ == at+jwt`.
pub(in crate::platform::web) fn verify_access(secret: &Value, token: &str, resource: &str) -> Result<Value, AccessRefusal> {
    let (algorithm, key) = sign::decoding_key(secret).map_err(|e| AccessRefusal::Key(e.to_string()))?;
    let header = jsonwebtoken::decode_header(token).map_err(|e| AccessRefusal::Token(format!("not a JWT: {e}")))?;
    if !header.typ.as_deref().is_some_and(|typ| typ.eq_ignore_ascii_case(TYP) || typ.eq_ignore_ascii_case("application/at+jwt")) {
        return Err(AccessRefusal::Token("not an access token of this server (typ is not at+jwt)".to_string()));
    }
    let mut validation = Validation::new(algorithm);
    validation.validate_exp = true;
    validation.validate_nbf = true;
    validation.set_audience(&[resource]);
    validation.set_issuer(&[resource]);
    validation.set_required_spec_claims(&["exp", "aud", "iss"]);
    jsonwebtoken::decode::<Value>(token, &key, &validation)
        .map(|data| data.claims)
        .map_err(|e| AccessRefusal::Token(format!("JWT invalid: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: &str = "http://library.site-a.localhost/_mcp/library";

    /// One generated signing secret for the whole test run.
    fn secret() -> Value {
        static SECRET: std::sync::OnceLock<String> = std::sync::OnceLock::new();
        json!({ "algorithm": "HS256", "secret": SECRET.get_or_init(|| uuid::Uuid::new_v4().to_string()) })
    }

    #[test]
    fn an_access_token_opens_its_route_only() {
        let app = json!({ "sub": "reader-1", "roles": ["reader"], "aud": "app", "exp": 1, "iat": 1, "_zf_public": ["roles"] });
        let token = mint(&secret(), &app, R, "zfc_aaaaaaaaaaaaaaaaaaaaaaaaaa", "mcp").unwrap();
        let claims = verify_access(&secret(), &token, R).unwrap();
        assert_eq!(claims["sub"], "reader-1");
        assert_eq!(claims["roles"], json!(["reader"]));
        assert_eq!(claims["aud"], R);
        assert!(claims.get("_zf_public").is_none());
        assert!(matches!(verify_access(&secret(), &token, "http://library.site-a.localhost/_mcp/other"), Err(AccessRefusal::Token(_))));
        assert!(matches!(verify_access(&json!({ "secret": uuid::Uuid::new_v4().to_string() }), &token, R), Err(AccessRefusal::Token(_))));
    }

    #[test]
    fn the_apps_own_token_is_not_an_access_token() {
        let (algorithm, key) = sign::encoding_key(&secret()).unwrap();
        let now = now_unix();
        let app = jsonwebtoken::encode(&Header::new(algorithm), &json!({ "sub": "reader-1", "aud": R, "iss": R, "exp": now + 60 }), &key).unwrap();
        assert!(matches!(verify_access(&secret(), &app, R), Err(AccessRefusal::Token(_))), "no at+jwt, no entry");
    }
}

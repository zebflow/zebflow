//! The pure rules of a published route's OAuth: secrets, PKCE, redirect
//! URIs. No storage, no clock — each one a function a test can pin.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use reqwest::Url;
use ring::rand::{SecureRandom, SystemRandom};
use sha2::{Digest, Sha256};

/// Redirect URIs one client may register.
pub const MAX_REDIRECT_URIS: usize = 10;
/// The longest `client_name` kept.
pub const MAX_CLIENT_NAME: usize = 100;

/// Lowercase hex SHA-256 — how every secret is stored (the key is the hash).
pub fn sha256_hex(text: &str) -> String {
    Sha256::digest(text.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// The first 16 hex of the route's SHA-256: its folder in the project's store.
pub fn route_hash(route: &str) -> String {
    sha256_hex(route)[..16].to_string()
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    SystemRandom::new().fill(&mut bytes).expect("the system's random source");
    bytes
}

/// A bearer secret (ticket, code, refresh token): 32 random bytes, base64url.
pub fn new_secret() -> String {
    URL_SAFE_NO_PAD.encode(random_bytes::<32>())
}

/// A registered client's id: `zfc_` and 26 lowercase base32 characters
/// (128 random bits).
pub fn new_client_id() -> String {
    const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";
    let bytes = random_bytes::<16>();
    let mut bits: u32 = 0;
    let mut held = 0;
    let mut out = String::from("zfc_");
    for byte in bytes {
        bits = (bits << 8) | u32::from(byte);
        held += 8;
        while held >= 5 {
            held -= 5;
            out.push(ALPHABET[((bits >> held) & 31) as usize] as char);
        }
    }
    if held > 0 {
        out.push(ALPHABET[((bits << (5 - held)) & 31) as usize] as char);
    }
    out
}

/// `true` for a well-formed id this server issued.
pub fn is_issued_client_id(id: &str) -> bool {
    id.strip_prefix("zfc_").is_some_and(|rest| rest.len() == 26 && rest.bytes().all(|b| b.is_ascii_lowercase() || (b'2'..=b'7').contains(&b)))
}

/// An S256 `code_challenge`: 43 base64url characters (RFC 7636 §4.2).
pub fn valid_challenge(challenge: &str) -> bool {
    challenge.len() == 43 && challenge.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// RFC 7636 §4.6: `BASE64URL(SHA256(verifier)) == challenge`, the verifier
/// 43–128 unreserved characters.
pub fn pkce_matches(verifier: &str, challenge: &str) -> bool {
    let shaped = (43..=128).contains(&verifier.len())
        && verifier.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~'));
    shaped && URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())) == challenge
}

fn is_loopback_host(url: &Url) -> bool {
    matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
}

/// A redirect URI a client may register: absolute `https`, or `http` on a
/// loopback host (a native app's local listener); never a fragment.
pub fn acceptable_redirect(uri: &str) -> Result<(), String> {
    let url = Url::parse(uri).map_err(|e| format!("redirect URI '{uri}' is not a URL: {e}"))?;
    if url.fragment().is_some() {
        return Err(format!("redirect URI '{uri}' has a fragment"));
    }
    match url.scheme() {
        "https" if url.host_str().is_some() => Ok(()),
        "http" if is_loopback_host(&url) => Ok(()),
        _ => Err(format!("redirect URI '{uri}' must be https, or http on localhost / 127.0.0.1 / [::1]")),
    }
}

/// Whether `presented` is `registered`: the same string, or — for a loopback
/// `http` URI — the same but for the port (RFC 8252 §7.3: a native app
/// listens on whatever port the system gave it).
pub fn redirect_matches(registered: &str, presented: &str) -> bool {
    if registered == presented {
        return true;
    }
    let (Ok(mut a), Ok(mut b)) = (Url::parse(registered), Url::parse(presented)) else {
        return false;
    };
    if a.scheme() != "http" || b.scheme() != "http" || !is_loopback_host(&a) || !is_loopback_host(&b) {
        return false;
    }
    let _ = a.set_port(None);
    let _ = b.set_port(None);
    a == b
}

/// The host a person is told they are sending their sign-in to.
pub fn redirect_host(uri: &str) -> String {
    Url::parse(uri).ok().and_then(|u| u.host_str().map(str::to_string)).unwrap_or_default()
}

/// `uri` with these query parameters added, the ones it has kept.
pub fn with_query(uri: &str, pairs: &[(&str, &str)]) -> String {
    match Url::parse(uri) {
        Ok(mut url) => {
            {
                let mut query = url.query_pairs_mut();
                for (key, value) in pairs {
                    query.append_pair(key, value);
                }
            }
            url.to_string()
        }
        Err(_) => uri.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_matches_the_rfc_7636_appendix_b_vector() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        assert!(valid_challenge(challenge));
        assert!(pkce_matches(verifier, challenge));
        assert!(!pkce_matches("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXx", challenge));
        assert!(!pkce_matches("short", challenge), "a verifier is 43 to 128 characters");
        assert!(!valid_challenge("plain-text"));
    }

    #[test]
    fn redirects_match_exactly_but_a_loopback_port() {
        assert!(redirect_matches("https://client.example/cb", "https://client.example/cb"));
        assert!(!redirect_matches("https://client.example/cb", "https://client.example/cb/"));
        assert!(!redirect_matches("https://client.example/cb", "https://client.example:8443/cb"), "an https port is part of the URI");
        assert!(redirect_matches("http://127.0.0.1/callback", "http://127.0.0.1:53121/callback"));
        assert!(redirect_matches("http://localhost:7777/callback", "http://localhost:40123/callback"));
        assert!(!redirect_matches("http://localhost/callback", "http://localhost:4000/other"));
        assert!(!redirect_matches("http://localhost/callback", "http://127.0.0.1:4000/callback"), "another host is another URI");
    }

    #[test]
    fn only_https_or_loopback_http_without_a_fragment_registers() {
        assert!(acceptable_redirect("https://client.example/cb").is_ok());
        assert!(acceptable_redirect("http://localhost:7777/oauth/callback").is_ok());
        assert!(acceptable_redirect("http://[::1]/cb").is_ok());
        assert!(acceptable_redirect("http://client.example/cb").is_err());
        assert!(acceptable_redirect("https://client.example/cb#x").is_err());
        assert!(acceptable_redirect("javascript:alert(1)").is_err());
    }

    #[test]
    fn ids_and_secrets_are_shaped_and_fresh() {
        let id = new_client_id();
        assert!(is_issued_client_id(&id), "{id}");
        assert_ne!(id, new_client_id());
        assert_eq!(new_secret().len(), 43);
        assert_eq!(route_hash("/library").len(), 16);
        assert_eq!(redirect_host("https://client.example:8443/cb"), "client.example");
        assert_eq!(with_query("https://client.example/cb?a=1", &[("code", "c d")]), "https://client.example/cb?a=1&code=c+d");
    }
}

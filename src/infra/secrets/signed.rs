//! Signed tokens: a JSON payload and its HMAC-SHA256, both base64url,
//! joined by a dot (`<payload>.<signature>`).
//!
//! The key is a per-process 32-byte secret the caller holds and never writes
//! down, so a token proves only that this process issued it: the OAuth state
//! a Studio sign-in carries through the provider, and a room ticket
//! (`docs/contracts/rooms.md`). Expiry, single use and what the payload must
//! name are the caller's rules; this module only seals and checks.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// A fresh random key for one process.
pub fn process_key() -> [u8; 32] {
    use rand::RngExt as _;
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    bytes
}

/// `payload` signed with `key`.
pub fn sign(key: &[u8; 32], payload: &Value) -> String {
    let body = URL_SAFE_NO_PAD.encode(payload.to_string().as_bytes());
    let mut mac = HmacSha256::new_from_slice(key).expect("hmac accepts any key length");
    mac.update(body.as_bytes());
    let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    format!("{body}.{signature}")
}

/// The payload of a token `key` signed; an error naming what is wrong
/// otherwise. The signature is checked before the payload is read.
pub fn verify(key: &[u8; 32], token: &str) -> Result<Value, String> {
    let (body, signature) = token.split_once('.').ok_or("invalid token format")?;
    let signature = URL_SAFE_NO_PAD.decode(signature).map_err(|e| format!("bad signature encoding: {e}"))?;
    let mut mac = HmacSha256::new_from_slice(key).expect("hmac accepts any key length");
    mac.update(body.as_bytes());
    mac.verify_slice(&signature).map_err(|_| "invalid signature".to_string())?;
    let bytes = URL_SAFE_NO_PAD.decode(body).map_err(|e| format!("bad payload encoding: {e}"))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("bad payload json: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_token_verifies_with_its_key_only_and_unchanged() {
        let key = process_key();
        let token = sign(&key, &json!({ "room": "lobby", "n": 1 }));
        assert_eq!(verify(&key, &token).expect("valid"), json!({ "room": "lobby", "n": 1 }));
        assert_eq!(verify(&process_key(), &token).unwrap_err(), "invalid signature", "another process's key");
        let (body, signature) = token.split_once('.').unwrap();
        let forged = URL_SAFE_NO_PAD.encode(json!({ "room": "admin", "n": 1 }).to_string());
        assert!(verify(&key, &format!("{forged}.{signature}")).is_err(), "a changed payload");
        assert!(verify(&key, body).is_err(), "no signature");
    }
}

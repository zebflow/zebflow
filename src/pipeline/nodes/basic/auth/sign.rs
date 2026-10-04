//! The keys of a `jwt_signing_key` credential, read one way by everything
//! that signs or verifies the app's tokens: `auth.token.create`,
//! `auth.token.verify`'s neighbours, a webhook's `--auth jwt`,
//! `auth.oauth.approve` and a published MCP route's `--auth oauth`.
//!
//! The credential's secret holds `algorithm` (HS256/384/512 or RS256/384/512,
//! default HS256) and `secret` for HMAC, `private_key` (and `public_key`) as
//! PEM for RSA.

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Validation};
use serde_json::Value;

/// Why a credential's key could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyError {
    /// `algorithm` names none of the six.
    Algorithm(String),
    /// An HMAC credential without `secret`.
    SecretMissing,
    /// An RSA credential without the PEM this use needs.
    KeyMissing(&'static str),
    /// The PEM does not parse.
    KeyInvalid(String),
}

impl std::fmt::Display for KeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyError::Algorithm(name) => write!(f, "unsupported JWT algorithm '{name}'"),
            KeyError::SecretMissing => write!(f, "jwt_signing_key credential missing 'secret' field"),
            KeyError::KeyMissing(field) => write!(f, "jwt_signing_key credential missing '{field}' field"),
            KeyError::KeyInvalid(reason) => write!(f, "jwt_signing_key key is invalid: {reason}"),
        }
    }
}

/// The credential's algorithm, read case-insensitively; HS256 when unset.
pub fn algorithm(secret: &Value) -> Result<Algorithm, KeyError> {
    let name = secret.get("algorithm").and_then(Value::as_str).unwrap_or("HS256");
    match name.trim().to_ascii_uppercase().as_str() {
        "HS256" => Ok(Algorithm::HS256),
        "HS384" => Ok(Algorithm::HS384),
        "HS512" => Ok(Algorithm::HS512),
        "RS256" => Ok(Algorithm::RS256),
        "RS384" => Ok(Algorithm::RS384),
        "RS512" => Ok(Algorithm::RS512),
        _ => Err(KeyError::Algorithm(name.to_string())),
    }
}

fn is_hmac(algorithm: Algorithm) -> bool {
    matches!(algorithm, Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512)
}

fn hmac_secret(secret: &Value) -> Result<&str, KeyError> {
    secret.get("secret").and_then(Value::as_str).ok_or(KeyError::SecretMissing)
}

/// The key that signs: the HMAC secret, or the RSA private key.
pub fn encoding_key(secret: &Value) -> Result<(Algorithm, EncodingKey), KeyError> {
    let algorithm = algorithm(secret)?;
    if is_hmac(algorithm) {
        return Ok((algorithm, EncodingKey::from_secret(hmac_secret(secret)?.as_bytes())));
    }
    let pem = secret.get("private_key").and_then(Value::as_str).ok_or(KeyError::KeyMissing("private_key"))?;
    let key = EncodingKey::from_rsa_pem(pem.as_bytes()).map_err(|e| KeyError::KeyInvalid(e.to_string()))?;
    Ok((algorithm, key))
}

/// The key that verifies: the HMAC secret, or the RSA public key (the
/// private key when no public one is stored).
pub fn decoding_key(secret: &Value) -> Result<(Algorithm, DecodingKey), KeyError> {
    let algorithm = algorithm(secret)?;
    if is_hmac(algorithm) {
        return Ok((algorithm, DecodingKey::from_secret(hmac_secret(secret)?.as_bytes())));
    }
    let pem = secret
        .get("public_key")
        .or_else(|| secret.get("private_key"))
        .and_then(Value::as_str)
        .ok_or(KeyError::KeyMissing("public_key"))?;
    let key = DecodingKey::from_rsa_pem(pem.as_bytes()).map_err(|e| KeyError::KeyInvalid(e.to_string()))?;
    Ok((algorithm, key))
}

/// Why a token did not verify.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    /// The credential itself is unusable — the operator's to fix.
    Key(KeyError),
    /// The token: bad signature, expired, malformed.
    Token(String),
}

/// An app token as `auth.token.create` signed it: the signature with the
/// credential's algorithm and `exp` (and `nbf` when present). Its `aud`, if
/// any, is not this check's business — `auth.oauth.approve` reads who
/// signed in, not whom the token was for.
pub fn verify_app_token(secret: &Value, token: &str) -> Result<Value, VerifyError> {
    let (algorithm, key) = decoding_key(secret).map_err(VerifyError::Key)?;
    let mut validation = Validation::new(algorithm);
    validation.validate_exp = true;
    validation.validate_nbf = true;
    validation.validate_aud = false;
    jsonwebtoken::decode::<Value>(token, &key, &validation)
        .map(|data| data.claims)
        .map_err(|e| VerifyError::Token(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_algorithm_is_read_one_way() {
        assert_eq!(algorithm(&json!({})).unwrap(), Algorithm::HS256);
        assert_eq!(algorithm(&json!({ "algorithm": "hs512" })).unwrap(), Algorithm::HS512);
        assert_eq!(algorithm(&json!({ "algorithm": "ES256" })), Err(KeyError::Algorithm("ES256".into())));
        assert!(matches!(encoding_key(&json!({ "algorithm": "HS256" })), Err(KeyError::SecretMissing)));
        assert!(matches!(decoding_key(&json!({ "algorithm": "RS256" })), Err(KeyError::KeyMissing("public_key"))));
    }

    #[test]
    fn an_app_token_verifies_whatever_its_audience() {
        let secret = json!({ "algorithm": "HS256", "secret": uuid::Uuid::new_v4().to_string() });
        let (algorithm, key) = encoding_key(&secret).unwrap();
        let now = chrono::Utc::now().timestamp();
        let sign = |claims: Value| jsonwebtoken::encode(&jsonwebtoken::Header::new(algorithm), &claims, &key).unwrap();
        let claims = verify_app_token(&secret, &sign(json!({ "sub": "u1", "aud": "app", "exp": now + 60 }))).unwrap();
        assert_eq!(claims["sub"], "u1");
        assert!(matches!(verify_app_token(&secret, &sign(json!({ "sub": "u1", "exp": now - 120 }))), Err(VerifyError::Token(_))));
        let other = json!({ "secret": uuid::Uuid::new_v4().to_string() });
        assert!(matches!(verify_app_token(&other, &sign(json!({ "exp": now + 60 }))), Err(VerifyError::Token(_))));
    }
}

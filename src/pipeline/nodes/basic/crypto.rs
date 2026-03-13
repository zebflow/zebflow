//! `n.crypto` — cryptographic primitives for pipeline security.
//!
//! A single multipurpose node covering the most common cryptographic
//! operations needed in authentication and data-integrity pipelines.
//!
//! # Operations
//!
//! | `--op` | Description | Output pin |
//! |---|---|---|
//! | `sha256` | SHA-256 hex digest of input | `out` |
//! | `sha512` | SHA-512 hex digest of input | `out` |
//! | `bcrypt_hash` | bcrypt password hash | `out` |
//! | `bcrypt_verify` | Compare plaintext against bcrypt hash | `true` / `false` |
//! | `argon2_hash` | Argon2id password hash (PHC string format) | `out` |
//! | `argon2_verify` | Compare plaintext against Argon2 hash | `true` / `false` |
//! | `hmac_sha256` | HMAC-SHA256 hex of input using `key` | `out` |
//! | `base64_encode` | Standard Base64 encode | `out` |
//! | `base64_decode` | Standard Base64 decode to UTF-8 | `out` |
//! | `random_hex` | Cryptographically random hex string | `out` |
//!
//! # Config flags
//!
//! | Flag | Type | Default | Description |
//! |---|---|---|---|
//! | `--op` | string | *(required)* | Operation name (see table above) |
//! | `--input-path` | string | `""` → `payload.input` | JSON pointer for primary input |
//! | `--hash-path` | string | `""` → `payload.hash` | JSON pointer for stored hash (verify ops) |
//! | `--key-path` | string | `""` → `payload.key` | JSON pointer for HMAC secret key |
//! | `--cost` | integer | `12` | bcrypt cost factor (4–31) |
//! | `--length` | integer | `32` | Random byte count for `random_hex` |
//!
//! # Payload extraction
//!
//! Each op reads its inputs from the flowing payload via JSON pointer paths.
//! If a path flag is empty, a well-known field name is used as fallback:
//!
//! | Op | Primary input field | Secondary input field |
//! |---|---|---|
//! | `sha256`, `sha512` | `payload.input` | — |
//! | `bcrypt_hash`, `argon2_hash` | `payload.input` | — |
//! | `bcrypt_verify`, `argon2_verify` | `payload.input` (plaintext) | `payload.hash` (stored hash) |
//! | `hmac_sha256` | `payload.input` (message) | `payload.key` (secret key) |
//! | `base64_encode`, `base64_decode` | `payload.input` | — |
//! | `random_hex` | *(none — generates fresh bytes)* | — |
//!
//! # Output pins
//!
//! Hash / encode / decode operations write `result` into the payload and
//! emit to the `out` pin.
//!
//! Verify operations (`bcrypt_verify`, `argon2_verify`) route to the `true`
//! or `false` pin, forwarding the original payload unchanged — no extra
//! `n.logic.if` node needed.
//!
//! # Example pipelines
//!
//! **User registration — hash a password:**
//! ```text
//! | n.trigger.webhook --path /auth/register --method POST
//! | n.crypto --op bcrypt_hash
//! | n.pg.query --credential main-db -- "INSERT INTO users (email, pw_hash) VALUES ($.email, $.result)"
//! ```
//!
//! **User login — verify password and issue JWT:**
//! ```text
//! | n.trigger.webhook --path /auth/login --method POST
//! | n.pg.query --credential main-db -- "SELECT pw_hash AS hash FROM users WHERE email = $.email"
//! | n.crypto --op bcrypt_verify
//! | [true]  → n.auth.token.create --credential jwt-key
//! | [false] → n.script -- "return { _status: 401, error: 'Invalid credentials' }"
//! ```
//!
//! **Webhook signature check (HMAC-SHA256):**
//! ```text
//! | n.trigger.webhook --path /webhooks/github --method POST
//! | n.crypto --op hmac_sha256 --key-path /webhook_secret
//! | n.logic.if -- "payload.result === payload.expected_sig"
//! ```
//!
//! **Generate a secure session token:**
//! ```text
//! | n.trigger.webhook --path /auth/session --method POST
//! | n.crypto --op random_hex --length 32
//! | n.pg.query --credential main-db -- "INSERT INTO sessions (token, user_id) VALUES ($.result, $.user_id)"
//! ```

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256, Sha512};

use crate::pipeline::{
    FrameworkError, NodeDefinition,
    nodes::{FrameworkNode, NodeExecutionInput, NodeExecutionOutput},
};

pub const NODE_KIND: &str = "n.crypto";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";
const OUTPUT_PIN_TRUE: &str = "true";
const OUTPUT_PIN_FALSE: &str = "false";

/// Return the [`NodeDefinition`] for `n.crypto`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Crypto".to_string(),
        description: "Cryptographic primitives: hash, verify, HMAC, base64, random. \
            Use --op to select the operation. Hash/encode ops write { result } to the \
            payload and emit to the 'out' pin. Verify ops (bcrypt_verify, argon2_verify) \
            route to 'true' or 'false' without an extra n.logic.if. \
            Input defaults to payload.input; override with --input-path."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Primary input value (password, message, etc.)."
                },
                "hash": {
                    "type": "string",
                    "description": "Stored hash for verify operations."
                },
                "key": {
                    "type": "string",
                    "description": "HMAC secret key."
                }
            }
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "result": {
                    "type": "string",
                    "description": "Computed value (hex digest, base64 string, bcrypt/argon2 hash, etc.)."
                }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![
            OUTPUT_PIN_OUT.to_string(),
            OUTPUT_PIN_TRUE.to_string(),
            OUTPUT_PIN_FALSE.to_string(),
        ],
        script_available: false,
        script_bridge: None,
        ai_tool: Default::default(),
    }
}

/// Configuration for `n.crypto`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Cryptographic operation to perform (required).
    ///
    /// Valid values: `sha256`, `sha512`, `bcrypt_hash`, `bcrypt_verify`,
    /// `argon2_hash`, `argon2_verify`, `hmac_sha256`, `base64_encode`,
    /// `base64_decode`, `random_hex`.
    pub op: String,

    /// JSON pointer into the payload for the primary input value.
    ///
    /// Empty (default) → reads `payload["input"]`.
    /// Example: `"/body"` — use `payload.body` as the input.
    #[serde(default)]
    pub input_path: String,

    /// JSON pointer into the payload for the stored hash (verify ops only).
    ///
    /// Empty (default) → reads `payload["hash"]`.
    #[serde(default)]
    pub hash_path: String,

    /// JSON pointer into the payload for the HMAC secret key (`hmac_sha256` only).
    ///
    /// Empty (default) → reads `payload["key"]`.
    #[serde(default)]
    pub key_path: String,

    /// bcrypt cost factor (default `12`, range 4–31).
    ///
    /// Higher cost = slower hash = harder to brute-force.
    /// Cost 12 takes ~200–400 ms on modern hardware.
    #[serde(default)]
    pub cost: Option<u32>,

    /// Number of random bytes for `random_hex` (default `32`).
    ///
    /// Output hex string length = `length * 2`.
    /// Example: `length = 32` → 64-character hex string.
    #[serde(default)]
    pub length: Option<u32>,
}

/// `n.crypto` node instance.
pub struct Node {
    config: Config,
}

impl Node {
    pub fn new(config: Config) -> Result<Self, FrameworkError> {
        const VALID_OPS: &[&str] = &[
            "sha256",
            "sha512",
            "bcrypt_hash",
            "bcrypt_verify",
            "argon2_hash",
            "argon2_verify",
            "hmac_sha256",
            "base64_encode",
            "base64_decode",
            "random_hex",
        ];
        if config.op.is_empty() {
            return Err(FrameworkError::new(
                "FW_NODE_CRYPTO_CONFIG",
                "n.crypto: --op must be specified",
            ));
        }
        if !VALID_OPS.contains(&config.op.as_str()) {
            return Err(FrameworkError::new(
                "FW_NODE_CRYPTO_OP",
                format!(
                    "n.crypto: unknown op '{}'. Valid ops: {}",
                    config.op,
                    VALID_OPS.join(", ")
                ),
            ));
        }
        Ok(Self { config })
    }
}

// ── Payload helpers ──────────────────────────────────────────────────────────

/// Extract a string value from the payload using a JSON pointer path.
///
/// If `path` is empty, falls back to `payload[fallback_key]`.
/// Returns `""` if neither is found or the value is not a string.
fn extract_str<'a>(payload: &'a Value, path: &str, fallback_key: &str) -> &'a str {
    if !path.is_empty() {
        let ptr = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        };
        return payload.pointer(&ptr).and_then(Value::as_str).unwrap_or_default();
    }
    payload.get(fallback_key).and_then(Value::as_str).unwrap_or_default()
}

/// Clone `payload`, insert `"result"` field, and return the modified object.
fn with_result(payload: Value, result: impl Into<Value>) -> Value {
    let mut out = payload;
    if let Value::Object(ref mut map) = out {
        map.insert("result".to_string(), result.into());
    }
    out
}

// ── FrameworkNode impl ───────────────────────────────────────────────────────

#[async_trait]
impl FrameworkNode for Node {
    fn kind(&self) -> &'static str {
        NODE_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[INPUT_PIN_IN]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT, OUTPUT_PIN_TRUE, OUTPUT_PIN_FALSE]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, FrameworkError> {
        let payload = input.payload;

        match self.config.op.as_str() {
            // ── sha256 ────────────────────────────────────────────────────────
            "sha256" => {
                let input_val =
                    extract_str(&payload, &self.config.input_path, "input").to_string();
                let mut h = Sha256::new();
                h.update(input_val.as_bytes());
                let result = hex::encode(h.finalize());
                Ok(NodeExecutionOutput {
                    output_pins: vec![OUTPUT_PIN_OUT.to_string()],
                    payload: with_result(payload, result),
                    trace: vec!["n.crypto: sha256".to_string()],
                })
            }

            // ── sha512 ────────────────────────────────────────────────────────
            "sha512" => {
                let input_val =
                    extract_str(&payload, &self.config.input_path, "input").to_string();
                let mut h = Sha512::new();
                h.update(input_val.as_bytes());
                let result = hex::encode(h.finalize());
                Ok(NodeExecutionOutput {
                    output_pins: vec![OUTPUT_PIN_OUT.to_string()],
                    payload: with_result(payload, result),
                    trace: vec!["n.crypto: sha512".to_string()],
                })
            }

            // ── bcrypt_hash ───────────────────────────────────────────────────
            "bcrypt_hash" => {
                let input_val =
                    extract_str(&payload, &self.config.input_path, "input").to_string();
                let cost = self.config.cost.unwrap_or(12);
                let result = tokio::task::spawn_blocking(move || {
                    bcrypt::hash(&input_val, cost).map_err(|e| e.to_string())
                })
                .await
                .map_err(|e| FrameworkError::new("FW_NODE_CRYPTO_SPAWN", e.to_string()))?
                .map_err(|e| FrameworkError::new("FW_NODE_CRYPTO_BCRYPT_HASH", e))?;
                Ok(NodeExecutionOutput {
                    output_pins: vec![OUTPUT_PIN_OUT.to_string()],
                    payload: with_result(payload, result),
                    trace: vec![format!("n.crypto: bcrypt_hash cost={cost}")],
                })
            }

            // ── bcrypt_verify ─────────────────────────────────────────────────
            "bcrypt_verify" => {
                let plaintext =
                    extract_str(&payload, &self.config.input_path, "input").to_string();
                let stored =
                    extract_str(&payload, &self.config.hash_path, "hash").to_string();
                let is_valid = tokio::task::spawn_blocking(move || {
                    bcrypt::verify(&plaintext, &stored).unwrap_or(false)
                })
                .await
                .map_err(|e| FrameworkError::new("FW_NODE_CRYPTO_SPAWN", e.to_string()))?;
                let pin = if is_valid { OUTPUT_PIN_TRUE } else { OUTPUT_PIN_FALSE };
                Ok(NodeExecutionOutput {
                    output_pins: vec![pin.to_string()],
                    payload,
                    trace: vec![format!("n.crypto: bcrypt_verify result={is_valid}")],
                })
            }

            // ── argon2_hash ───────────────────────────────────────────────────
            "argon2_hash" => {
                let input_val =
                    extract_str(&payload, &self.config.input_path, "input").to_string();
                let result = tokio::task::spawn_blocking(move || {
                    use argon2::{
                        Argon2,
                        password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
                    };
                    let salt = SaltString::generate(&mut OsRng);
                    Argon2::default()
                        .hash_password(input_val.as_bytes(), &salt)
                        .map(|h| h.to_string())
                        .map_err(|e| e.to_string())
                })
                .await
                .map_err(|e| FrameworkError::new("FW_NODE_CRYPTO_SPAWN", e.to_string()))?
                .map_err(|e| FrameworkError::new("FW_NODE_CRYPTO_ARGON2_HASH", e))?;
                Ok(NodeExecutionOutput {
                    output_pins: vec![OUTPUT_PIN_OUT.to_string()],
                    payload: with_result(payload, result),
                    trace: vec!["n.crypto: argon2_hash".to_string()],
                })
            }

            // ── argon2_verify ─────────────────────────────────────────────────
            "argon2_verify" => {
                let plaintext =
                    extract_str(&payload, &self.config.input_path, "input").to_string();
                let stored =
                    extract_str(&payload, &self.config.hash_path, "hash").to_string();
                let is_valid = tokio::task::spawn_blocking(move || {
                    use argon2::{
                        Argon2,
                        password_hash::{PasswordHash, PasswordVerifier},
                    };
                    PasswordHash::new(&stored)
                        .map(|h| Argon2::default().verify_password(plaintext.as_bytes(), &h).is_ok())
                        .unwrap_or(false)
                })
                .await
                .map_err(|e| FrameworkError::new("FW_NODE_CRYPTO_SPAWN", e.to_string()))?;
                let pin = if is_valid { OUTPUT_PIN_TRUE } else { OUTPUT_PIN_FALSE };
                Ok(NodeExecutionOutput {
                    output_pins: vec![pin.to_string()],
                    payload,
                    trace: vec![format!("n.crypto: argon2_verify result={is_valid}")],
                })
            }

            // ── hmac_sha256 ───────────────────────────────────────────────────
            "hmac_sha256" => {
                let input_val =
                    extract_str(&payload, &self.config.input_path, "input").to_string();
                let key_val =
                    extract_str(&payload, &self.config.key_path, "key").to_string();
                type HmacSha256 = Hmac<Sha256>;
                let mut mac = HmacSha256::new_from_slice(key_val.as_bytes())
                    .map_err(|e| FrameworkError::new("FW_NODE_CRYPTO_HMAC_KEY", e.to_string()))?;
                mac.update(input_val.as_bytes());
                let result = hex::encode(mac.finalize().into_bytes());
                Ok(NodeExecutionOutput {
                    output_pins: vec![OUTPUT_PIN_OUT.to_string()],
                    payload: with_result(payload, result),
                    trace: vec!["n.crypto: hmac_sha256".to_string()],
                })
            }

            // ── base64_encode ─────────────────────────────────────────────────
            "base64_encode" => {
                let input_val =
                    extract_str(&payload, &self.config.input_path, "input").to_string();
                let result = general_purpose::STANDARD.encode(input_val.as_bytes());
                Ok(NodeExecutionOutput {
                    output_pins: vec![OUTPUT_PIN_OUT.to_string()],
                    payload: with_result(payload, result),
                    trace: vec!["n.crypto: base64_encode".to_string()],
                })
            }

            // ── base64_decode ─────────────────────────────────────────────────
            "base64_decode" => {
                let input_val =
                    extract_str(&payload, &self.config.input_path, "input").to_string();
                let bytes = general_purpose::STANDARD.decode(input_val.as_bytes()).map_err(|e| {
                    FrameworkError::new("FW_NODE_CRYPTO_BASE64_DECODE", e.to_string())
                })?;
                let result = String::from_utf8(bytes).map_err(|e| {
                    FrameworkError::new("FW_NODE_CRYPTO_BASE64_UTF8", e.to_string())
                })?;
                Ok(NodeExecutionOutput {
                    output_pins: vec![OUTPUT_PIN_OUT.to_string()],
                    payload: with_result(payload, result),
                    trace: vec!["n.crypto: base64_decode".to_string()],
                })
            }

            // ── random_hex ────────────────────────────────────────────────────
            "random_hex" => {
                let length = self.config.length.unwrap_or(32) as usize;
                let result = tokio::task::spawn_blocking(move || {
                    use rand::RngExt;
                    let mut rng = rand::rng();
                    let bytes: Vec<u8> = (0..length).map(|_| rng.random::<u8>()).collect();
                    hex::encode(&bytes)
                })
                .await
                .map_err(|e| FrameworkError::new("FW_NODE_CRYPTO_SPAWN", e.to_string()))?;
                Ok(NodeExecutionOutput {
                    output_pins: vec![OUTPUT_PIN_OUT.to_string()],
                    payload: with_result(payload, result),
                    trace: vec![format!("n.crypto: random_hex length={length}")],
                })
            }

            // ── unknown op (validation should have caught this) ───────────────
            other => Err(FrameworkError::new(
                "FW_NODE_CRYPTO_OP",
                format!("n.crypto: unknown op '{other}'"),
            )),
        }
    }
}

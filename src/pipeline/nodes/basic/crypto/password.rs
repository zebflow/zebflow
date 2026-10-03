//! `crypto.password.hash` and `crypto.password.verify` — a password stored
//! as a salted hash and checked against it.
//!
//! The hash is argon2id (PHC string, the default) or bcrypt (`--cost`,
//! 4–31, default 12). The verifier reads the algorithm from the hash itself
//! (`$argon2…`, `$2a$` / `$2b$` / `$2y$`) and refuses any other format. A
//! wrong password is an answer, not an error: `password: { valid: false }`
//! on the `false` pin. An empty password or hash is refused, so a login whose
//! lookup found no user goes to `:error`, never to `true`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{INPUT_PIN_IN, OUTPUT_PIN_FALSE, OUTPUT_PIN_OUT, OUTPUT_PIN_TRUE, answer, choice_flag, field, flag, required, select};
use crate::pipeline::model::{LayoutItem, NodeExample};
use crate::pipeline::nodes::shared::limits::{choice, within};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const HASH_KIND: &str = "crypto.password.hash";
pub const VERIFY_KIND: &str = "crypto.password.verify";

/// The hasher failed or could not be scheduled: the world's side.
pub const HASH_CODE: &str = "FW_NODE_CRYPTO_PASSWORD_HASH";
/// A flag set wrong: an unknown algorithm, a cost out of range or with argon2.
pub const HASH_CONFIG_CODE: &str = "FW_NODE_CRYPTO_PASSWORD_HASH_CONFIG";
/// `--from` is empty.
pub const HASH_EMPTY_CODE: &str = "FW_NODE_CRYPTO_PASSWORD_HASH_EMPTY";
/// The verifier could not be scheduled: the world's side.
pub const VERIFY_CODE: &str = "FW_NODE_CRYPTO_PASSWORD_VERIFY";
/// A flag that is not text.
pub const VERIFY_CONFIG_CODE: &str = "FW_NODE_CRYPTO_PASSWORD_VERIFY_CONFIG";
/// `--from` or `--hash` is empty.
pub const VERIFY_EMPTY_CODE: &str = "FW_NODE_CRYPTO_PASSWORD_VERIFY_EMPTY";
/// `--hash` is not an argon2 or bcrypt hash.
pub const VERIFY_FORMAT_CODE: &str = "FW_NODE_CRYPTO_PASSWORD_VERIFY_FORMAT";

const ALGORITHMS: &[&str] = &["argon2", "bcrypt"];
const DEFAULT_COST: u32 = 12;
const BCRYPT_PREFIXES: &[&str] = &["$2a$", "$2b$", "$2y$"];

pub fn hash_definition() -> NodeDefinition {
    NodeDefinition {
        kind: HASH_KIND.to_string(),
        title: "Hash Password".to_string(),
        description: "Hash the password in `--from` for storage: `--algorithm argon2` (default, argon2id) or `bcrypt` with `--cost` 4–31 \
            (default 12; argon2 refuses it). Adds `password: { hash, algorithm }` and keeps the rest of the payload. An empty `--from` is refused."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `password` is added." }),
        output_schema: json!({ "type": "object", "properties": { "password": { "type": "object", "properties": {
            "hash": { "type": "string", "description": "PHC string (argon2) or $2b$ string (bcrypt)" },
            "algorithm": { "type": "string", "enum": ALGORITHMS }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![
            crate::pipeline::model::DslFlag { required: true, secret: true, ..flag("--from", "from", "The password to hash. Usually a {{ }} expression.", "text") },
            choice_flag("--algorithm", "algorithm", "argon2 (default) or bcrypt.", ALGORITHMS),
            flag("--cost", "cost", "bcrypt cost 4-31 (default 12); argon2 refuses it.", "number"),
        ],
        fields: vec![
            field("from", "Password", "The password to hash: a literal or {{ expr }}."),
            select("algorithm", "Algorithm", "argon2id for new systems; bcrypt to match an existing store.", ALGORITHMS),
            field("cost", "Cost (bcrypt)", "4-31, default 12. Each step doubles the work."),
        ],
        layout: vec![
            LayoutItem::Field("from".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("algorithm".to_string()), LayoutItem::Field("cost".to_string())] },
        ],
        examples: vec![
            NodeExample::dsl("Hash a password at registration", r#"crypto.password.hash --from "{{ $trigger.body.password }}""#)
                .output(json!({ "password": { "hash": "$argon2id$v=19$m=19456,t=2,p=1$…", "algorithm": "argon2" } }))
                .note("Store `input.password.hash`; the rest of the payload is still there for the INSERT."),
        ],
        ..Default::default()
    }
}

pub fn verify_definition() -> NodeDefinition {
    NodeDefinition {
        kind: VERIFY_KIND.to_string(),
        title: "Verify Password".to_string(),
        description: "Check the password in `--from` against the stored `--hash` (argon2 or bcrypt, read from the hash). \
            A match answers `password: { valid: true }` on the `true` pin, a mismatch `{ valid: false }` on `false`; the rest of the payload is kept. \
            An empty `--from` or `--hash` — a login whose lookup found no user — is refused and goes to `:error`, never to `true`."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `password` is added." }),
        output_schema: json!({ "type": "object", "properties": { "password": { "type": "object", "properties": {
            "valid": { "type": "boolean" }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_TRUE.to_string(), OUTPUT_PIN_FALSE.to_string()],
        dsl_flags: vec![
            crate::pipeline::model::DslFlag { required: true, secret: true, ..flag("--from", "from", "The password to check. Usually a {{ }} expression.", "text") },
            crate::pipeline::model::DslFlag { required: true, secret: true, ..flag("--hash", "hash", "The stored hash (argon2 or bcrypt).", "text") },
        ],
        fields: vec![
            field("from", "Password", "The password to check: a literal or {{ expr }}."),
            field("hash", "Stored hash", "The hash from the user record, e.g. {{ input.query.rows[0]?.password_hash }}."),
        ],
        layout: vec![LayoutItem::Field("from".to_string()), LayoutItem::Field("hash".to_string())],
        examples: vec![
            NodeExample::dsl(
                "Check a password at login",
                r#"crypto.password.verify --from "{{ $trigger.body.password }}" --hash "{{ input.query.rows[0]?.password_hash }}""#,
            )
            .output(json!({ "query": { "rows": [{ "id": 7, "password_hash": "$argon2id$…" }] }, "password": { "valid": true } }))
            .note("Fires `true` or `false`; wire `false` to a 401. No user means an empty hash, which goes to `:error`."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HashConfig {
    /// The password.
    #[serde(default)]
    pub from: Value,
    /// `argon2` (default) or `bcrypt`.
    #[serde(default)]
    pub algorithm: String,
    /// bcrypt cost, 4–31.
    #[serde(default)]
    pub cost: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VerifyConfig {
    /// The password.
    #[serde(default)]
    pub from: Value,
    /// The stored hash.
    #[serde(default)]
    pub hash: Value,
}

/// `crypto.password.hash`, its flags checked.
pub struct Hash {
    password: String,
    algorithm: &'static str,
    cost: u32,
}

impl Hash {
    pub fn new(config: HashConfig) -> Result<Self, PipelineError> {
        let algorithm = choice(&config.algorithm, ALGORITHMS, "argon2", "--algorithm", HASH_CONFIG_CODE)?;
        let cost = match cost_of(&config.cost)? {
            Some(_) if algorithm == "argon2" => {
                return Err(PipelineError::new(HASH_CONFIG_CODE, "--cost applies to bcrypt only; argon2 refuses it"));
            }
            Some(cost) => within(cost, 4, 31, "--cost", HASH_CONFIG_CODE)?,
            None => DEFAULT_COST,
        };
        let password = required(&config.from, "--from", HASH_CONFIG_CODE, HASH_EMPTY_CODE)?;
        Ok(Self { password, algorithm, cost })
    }
}

/// `--cost` as a whole number, `None` when unset.
fn cost_of(value: &Value) -> Result<Option<u32>, PipelineError> {
    let bad = || PipelineError::new(HASH_CONFIG_CODE, format!("--cost '{value}' is not a whole number"));
    match value {
        Value::Null => Ok(None),
        Value::String(s) if s.trim().is_empty() => Ok(None),
        Value::String(s) => s.trim().parse::<u32>().map(Some).map_err(|_| bad()),
        Value::Number(n) => n.as_u64().and_then(|n| u32::try_from(n).ok()).map(Some).ok_or_else(bad),
        _ => Err(bad()),
    }
}

#[async_trait]
impl NodeHandler for Hash {
    fn kind(&self) -> &'static str {
        HASH_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[INPUT_PIN_IN]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        let (password, algorithm, cost) = (self.password.clone(), self.algorithm, self.cost);
        let hash = tokio::task::spawn_blocking(move || match algorithm {
            "bcrypt" => bcrypt::hash(&password, cost).map_err(|e| e.to_string()),
            _ => {
                use argon2::{
                    Argon2,
                    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
                };
                let salt = SaltString::generate(&mut OsRng);
                Argon2::default().hash_password(password.as_bytes(), &salt).map(|h| h.to_string()).map_err(|e| e.to_string())
            }
        })
        .await
        .map_err(|e| PipelineError::new(HASH_CODE, e.to_string()))?
        .map_err(|e| PipelineError::new(HASH_CODE, e))?;
        Ok(answer(
            OUTPUT_PIN_OUT,
            &input.payload,
            json!({ "password": { "hash": hash, "algorithm": algorithm } }),
            format!("node_kind={HASH_KIND} algorithm={algorithm}"),
        ))
    }
}

/// `crypto.password.verify`, its flags checked.
pub struct Verify {
    password: String,
    hash: String,
}

impl Verify {
    pub fn new(config: VerifyConfig) -> Result<Self, PipelineError> {
        let password = required(&config.from, "--from", VERIFY_CONFIG_CODE, VERIFY_EMPTY_CODE)?;
        let hash = required(&config.hash, "--hash", VERIFY_CONFIG_CODE, VERIFY_EMPTY_CODE)?;
        algorithm_of(&hash)?;
        Ok(Self { password, hash })
    }
}

/// The algorithm a stored hash names, from its prefix.
fn algorithm_of(hash: &str) -> Result<&'static str, PipelineError> {
    if hash.starts_with("$argon2") {
        Ok("argon2")
    } else if BCRYPT_PREFIXES.iter().any(|prefix| hash.starts_with(prefix)) {
        Ok("bcrypt")
    } else {
        Err(PipelineError::new(VERIFY_FORMAT_CODE, "--hash is not an argon2 ($argon2…) or bcrypt ($2a$, $2b$, $2y$) hash"))
    }
}

/// `Ok(valid)`, or `Err` when the hash is malformed past its prefix.
fn check(password: &str, hash: &str) -> Result<bool, String> {
    match algorithm_of(hash).map_err(|e| e.message)? {
        "bcrypt" => bcrypt::verify(password, hash).map_err(|e| e.to_string()),
        _ => {
            use argon2::{
                Argon2,
                password_hash::{Error, PasswordHash, PasswordVerifier},
            };
            let parsed = PasswordHash::new(hash).map_err(|e| e.to_string())?;
            if parsed.hash.is_none() || parsed.salt.is_none() {
                return Err("it carries no salt or no hash".to_string());
            }
            match Argon2::default().verify_password(password.as_bytes(), &parsed) {
                Ok(()) => Ok(true),
                Err(Error::Password) => Ok(false),
                Err(other) => Err(other.to_string()),
            }
        }
    }
}

#[async_trait]
impl NodeHandler for Verify {
    fn kind(&self) -> &'static str {
        VERIFY_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[INPUT_PIN_IN]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_TRUE, OUTPUT_PIN_FALSE]
    }

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        let (password, hash) = (self.password.clone(), self.hash.clone());
        let valid = tokio::task::spawn_blocking(move || check(&password, &hash))
            .await
            .map_err(|e| PipelineError::new(VERIFY_CODE, e.to_string()))?
            .map_err(|e| PipelineError::new(VERIFY_FORMAT_CODE, format!("--hash is malformed: {e}")))?;
        Ok(answer(
            if valid { OUTPUT_PIN_TRUE } else { OUTPUT_PIN_FALSE },
            &input.payload,
            json!({ "password": { "valid": valid } }),
            format!("node_kind={VERIFY_KIND} valid={valid}"),
        ))
    }
}

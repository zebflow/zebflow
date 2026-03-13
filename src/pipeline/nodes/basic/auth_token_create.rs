/// JWT token creation node — signs claims using a stored `jwt_signing_key` credential.
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::pipeline::{
    FrameworkError, NodeDefinition,
    nodes::{FrameworkNode, NodeExecutionInput, NodeExecutionOutput},
};
use crate::platform::services::CredentialService;

use super::util::metadata_scope;

pub const NODE_KIND: &str = "n.auth.token.create";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Create Auth Token".to_string(),
        description: "Signs a JWT access token from input data using a stored jwt_signing_key credential. Supports HS256 and RS256 algorithms.".to_string(),
        input_schema: json!({
            "type": "object",
            "description": "Input payload for claim extraction."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "access_token": { "type": "string" },
                "token_type": { "type": "string" },
                "expires_in": { "type": "integer" },
                "profile": { "type": "object" }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        ai_tool: Default::default(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// ID of the `jwt_signing_key` credential.
    pub credential_id: String,
    /// Token lifetime in seconds (default 900).
    #[serde(default)]
    pub expires_in: Option<i64>,
    /// Map of claim_name → JSON pointer path (`$.field`) or literal value.
    #[serde(default)]
    pub claims: Map<String, Value>,
    /// Optional JWT issuer (`iss`).
    #[serde(default)]
    pub issuer: Option<String>,
    /// Optional JWT audience (`aud`).
    #[serde(default)]
    pub audience: Option<String>,
}

pub struct Node {
    config: Config,
    credentials: Arc<CredentialService>,
}

impl Node {
    pub fn new(config: Config, credentials: Arc<CredentialService>) -> Result<Self, FrameworkError> {
        if config.credential_id.trim().is_empty() {
            return Err(FrameworkError::new(
                "FW_NODE_AUTH_TOKEN_CONFIG",
                "config.credential_id must not be empty",
            ));
        }
        Ok(Self { config, credentials })
    }
}

/// Resolves a claim value: strings starting with `$.` are treated as JSON pointer paths
/// into the input payload; everything else is used as a literal.
fn resolve_claim(val: &Value, payload: &Value) -> Value {
    if let Value::String(expr) = val {
        if let Some(pointer) = expr.strip_prefix("$.") {
            let ptr = format!("/{}", pointer.replace('.', "/"));
            return payload.pointer(&ptr).cloned().unwrap_or(Value::Null);
        }
    }
    val.clone()
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[async_trait]
impl FrameworkNode for Node {
    fn kind(&self) -> &'static str {
        NODE_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[INPUT_PIN_IN]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, FrameworkError> {
        let (owner, project, _pipeline, _request_id) = metadata_scope(&input.metadata)?;

        // --- Credential ---
        let credential = self
            .credentials
            .get_project_credential(owner, project, &self.config.credential_id)
            .map_err(|err| FrameworkError::new("FW_NODE_AUTH_TOKEN_CREDENTIAL", err.to_string()))?
            .ok_or_else(|| {
                FrameworkError::new(
                    "FW_NODE_AUTH_TOKEN_CREDENTIAL_MISSING",
                    format!("credential '{}' not found", self.config.credential_id),
                )
            })?;

        if credential.kind != "jwt_signing_key" {
            return Err(FrameworkError::new(
                "FW_NODE_AUTH_TOKEN_CREDENTIAL_KIND",
                format!(
                    "credential '{}' is kind '{}', expected 'jwt_signing_key'",
                    credential.credential_id, credential.kind
                ),
            ));
        }

        let algorithm_str = credential
            .secret
            .get("algorithm")
            .and_then(|v| v.as_str())
            .unwrap_or("HS256");

        let algorithm = match algorithm_str {
            "HS256" | "hs256" => Algorithm::HS256,
            "HS384" | "hs384" => Algorithm::HS384,
            "HS512" | "hs512" => Algorithm::HS512,
            "RS256" | "rs256" => Algorithm::RS256,
            "RS384" | "rs384" => Algorithm::RS384,
            "RS512" | "rs512" => Algorithm::RS512,
            other => {
                return Err(FrameworkError::new(
                    "FW_NODE_AUTH_TOKEN_ALGORITHM",
                    format!("unsupported JWT algorithm '{}'", other),
                ))
            }
        };

        // --- Build claims from input payload ---
        let mut claims_map = Map::new();
        for (key, val) in &self.config.claims {
            claims_map.insert(key.clone(), resolve_claim(val, &input.payload));
        }

        // Copy profile before adding standard JWT fields
        let profile = Value::Object(claims_map.clone());

        // Add standard JWT claims
        let now = now_unix();
        let expires_in = self.config.expires_in.unwrap_or(900);
        claims_map.insert("iat".to_string(), json!(now));
        claims_map.insert("exp".to_string(), json!(now + expires_in));
        if let Some(iss) = &self.config.issuer {
            claims_map.insert("iss".to_string(), json!(iss));
        }
        if let Some(aud) = &self.config.audience {
            claims_map.insert("aud".to_string(), json!(aud));
        }

        // --- Sign JWT ---
        let header = Header::new(algorithm);
        let claims_val = Value::Object(claims_map);

        let token = match algorithm {
            Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
                let secret = credential
                    .secret
                    .get("secret")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        FrameworkError::new(
                            "FW_NODE_AUTH_TOKEN_SECRET_MISSING",
                            "jwt_signing_key credential missing 'secret' field",
                        )
                    })?;
                jsonwebtoken::encode(
                    &header,
                    &claims_val,
                    &EncodingKey::from_secret(secret.as_bytes()),
                )
                .map_err(|err| {
                    FrameworkError::new("FW_NODE_AUTH_TOKEN_SIGN", err.to_string())
                })?
            }
            Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => {
                let pem = credential
                    .secret
                    .get("private_key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        FrameworkError::new(
                            "FW_NODE_AUTH_TOKEN_KEY_MISSING",
                            "jwt_signing_key credential missing 'private_key' field",
                        )
                    })?;
                let key = EncodingKey::from_rsa_pem(pem.as_bytes()).map_err(|err| {
                    FrameworkError::new("FW_NODE_AUTH_TOKEN_KEY_INVALID", err.to_string())
                })?;
                jsonwebtoken::encode(&header, &claims_val, &key).map_err(|err| {
                    FrameworkError::new("FW_NODE_AUTH_TOKEN_SIGN", err.to_string())
                })?
            }
            _ => {
                return Err(FrameworkError::new(
                    "FW_NODE_AUTH_TOKEN_ALGORITHM",
                    "unsupported JWT algorithm variant",
                ))
            }
        };

        let output = json!({
            "access_token": token,
            "token_type": "bearer",
            "expires_in": expires_in,
            "profile": profile,
        });

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: output,
            trace: vec![format!(
                "n.auth.token.create: signed {} token, exp +{}s",
                algorithm_str, expires_in
            )],
        })
    }
}

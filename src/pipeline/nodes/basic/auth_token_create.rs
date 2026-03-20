/// JWT token creation node — signs claims using a stored `jwt_signing_key` credential.
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::pipeline::{
    PipelineError, NodeDefinition,
    nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput},
};
use crate::platform::services::CredentialService;

use crate::pipeline::model::LayoutItem;
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
        config_schema: serde_json::json!({
        "type": "object",
        "properties": {
            "credential_id": { "type": "string", "description": "ID of the jwt_signing_key credential." },
            "expires_in": { "type": "integer", "description": "Token lifetime in seconds (default 900)." },
            "set_cookie": { "type": "boolean", "description": "When true, sets the token as an HttpOnly cookie in the response." },
            "cookie_name": { "type": "string", "description": "Cookie name (default: zebflow_session)." },
            "claims": { "type": "object", "description": "Map of claim_name → JSON pointer path ($.field) or literal value." },
            "issuer": { "type": "string" },
            "audience": { "type": "string" }
        }
    }),
        dsl_flags: vec![
            crate::pipeline::model::DslFlag {
                flag: "--credential".to_string(),
                config_key: "credential_id".to_string(),
                description: "ID of the jwt_signing_key credential used to sign the token.".to_string(),
                kind: crate::pipeline::model::DslFlagKind::Scalar,
                required: true,
            },
            crate::pipeline::model::DslFlag {
                flag: "--expires-in".to_string(),
                config_key: "expires_in".to_string(),
                description: "Token lifetime in seconds (default 900).".to_string(),
                kind: crate::pipeline::model::DslFlagKind::Scalar,
                required: false,
            },
            crate::pipeline::model::DslFlag {
                flag: "--set-cookie".to_string(),
                config_key: "set_cookie".to_string(),
                description: "When true, instructs the webhook ingress to set the token as an HttpOnly cookie (name controlled by --cookie-name).".to_string(),
                kind: crate::pipeline::model::DslFlagKind::Scalar,
                required: false,
            },
            crate::pipeline::model::DslFlag {
                flag: "--cookie-name".to_string(),
                config_key: "cookie_name".to_string(),
                description: "Cookie name to use when --set-cookie is true (default: zebflow_session).".to_string(),
                kind: crate::pipeline::model::DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType, NodeFieldDataSource, SelectOptionDef};
            vec![
                NodeFieldDef { name: "title".to_string(), label: "Title".to_string(), field_type: NodeFieldType::Text, help: Some("Override display title for this node.".to_string()), ..Default::default() },
                NodeFieldDef { name: "credential_id".to_string(), label: "Signing Credential".to_string(), field_type: NodeFieldType::Select, data_source: Some(NodeFieldDataSource::CredentialsJwt), help: Some("JWT signing key credential (kind: jwt_signing_key).".to_string()), ..Default::default() },
                NodeFieldDef { name: "algorithm".to_string(), label: "Algorithm".to_string(), field_type: NodeFieldType::Select, options: vec![
                    SelectOptionDef { value: "HS256".to_string(), label: "HS256 — HMAC-SHA256 (symmetric)".to_string() },
                    SelectOptionDef { value: "HS384".to_string(), label: "HS384 — HMAC-SHA384 (symmetric)".to_string() },
                    SelectOptionDef { value: "HS512".to_string(), label: "HS512 — HMAC-SHA512 (symmetric)".to_string() },
                    SelectOptionDef { value: "RS256".to_string(), label: "RS256 — RSA-PKCS1v15-SHA256 (asymmetric)".to_string() },
                    SelectOptionDef { value: "RS384".to_string(), label: "RS384 — RSA-PKCS1v15-SHA384 (asymmetric)".to_string() },
                    SelectOptionDef { value: "RS512".to_string(), label: "RS512 — RSA-PKCS1v15-SHA512 (asymmetric)".to_string() },
                    SelectOptionDef { value: "ES256".to_string(), label: "ES256 — ECDSA P-256 (asymmetric)".to_string() },
                    SelectOptionDef { value: "ES384".to_string(), label: "ES384 — ECDSA P-384 (asymmetric)".to_string() },
                ], ..Default::default() },
                NodeFieldDef { name: "expires_in".to_string(), label: "Expires In (seconds)".to_string(), field_type: NodeFieldType::Text, ..Default::default() },
                NodeFieldDef { name: "claims".to_string(), label: "Static Claims (JSON)".to_string(), field_type: NodeFieldType::Textarea, rows: Some(5), ..Default::default() },
                NodeFieldDef { name: "issuer".to_string(), label: "Issuer (iss)".to_string(), field_type: NodeFieldType::Text, ..Default::default() },
                NodeFieldDef { name: "audience".to_string(), label: "Audience (aud)".to_string(), field_type: NodeFieldType::Text, ..Default::default() },
            ]
        },
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("title".to_string()), LayoutItem::Field("algorithm".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("credential_id".to_string()), LayoutItem::Field("expires_in".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("issuer".to_string()), LayoutItem::Field("audience".to_string())] },
            LayoutItem::Field("claims".to_string()),
        ],
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
    /// When true, instruct the webhook ingress to set the token as an HttpOnly cookie.
    #[serde(default)]
    pub set_cookie: bool,
    /// Cookie name when `set_cookie` is true (default: `zebflow_session`).
    #[serde(default)]
    pub cookie_name: Option<String>,
}

pub struct Node {
    config: Config,
    credentials: Arc<CredentialService>,
}

impl Node {
    pub fn new(config: Config, credentials: Arc<CredentialService>) -> Result<Self, PipelineError> {
        if config.credential_id.trim().is_empty() {
            return Err(PipelineError::new(
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
impl NodeHandler for Node {
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
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let (owner, project, _pipeline, _request_id) = metadata_scope(&input.metadata)?;

        // --- Credential ---
        let credential = self
            .credentials
            .get_project_credential(owner, project, &self.config.credential_id)
            .map_err(|err| PipelineError::new("FW_NODE_AUTH_TOKEN_CREDENTIAL", err.to_string()))?
            .ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_AUTH_TOKEN_CREDENTIAL_MISSING",
                    format!("credential '{}' not found", self.config.credential_id),
                )
            })?;

        if credential.kind != "jwt_signing_key" {
            return Err(PipelineError::new(
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
                return Err(PipelineError::new(
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
                        PipelineError::new(
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
                    PipelineError::new("FW_NODE_AUTH_TOKEN_SIGN", err.to_string())
                })?
            }
            Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => {
                let pem = credential
                    .secret
                    .get("private_key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        PipelineError::new(
                            "FW_NODE_AUTH_TOKEN_KEY_MISSING",
                            "jwt_signing_key credential missing 'private_key' field",
                        )
                    })?;
                let key = EncodingKey::from_rsa_pem(pem.as_bytes()).map_err(|err| {
                    PipelineError::new("FW_NODE_AUTH_TOKEN_KEY_INVALID", err.to_string())
                })?;
                jsonwebtoken::encode(&header, &claims_val, &key).map_err(|err| {
                    PipelineError::new("FW_NODE_AUTH_TOKEN_SIGN", err.to_string())
                })?
            }
            _ => {
                return Err(PipelineError::new(
                    "FW_NODE_AUTH_TOKEN_ALGORITHM",
                    "unsupported JWT algorithm variant",
                ))
            }
        };

        let mut output = json!({
            "access_token": token,
            "token_type": "bearer",
            "expires_in": expires_in,
            "profile": profile,
        });

        // Inject _set_cookie directive for the webhook ingress to pick up.
        if self.config.set_cookie {
            let name = self.config.cookie_name.as_deref().unwrap_or("zebflow_session");
            if let Value::Object(ref mut map) = output {
                map.insert("_set_cookie".to_string(), json!({
                    "name": name,
                    "value": token,
                    "max_age": expires_in,
                    "http_only": true,
                    "same_site": "Lax",
                    "path": "/"
                }));
            }
        }

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

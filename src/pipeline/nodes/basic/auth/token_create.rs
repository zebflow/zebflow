/// JWT token creation node — signs claims using a stored `jwt_signing_key` credential.
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::pipeline::model::NodeCapability;
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::CredentialService;

use crate::pipeline::nodes::shared::util::metadata_scope;
use crate::pipeline::model::LayoutItem;

pub const NODE_KIND: &str = "auth.token.create";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        // The minted session token. Rule 3 by value cannot see it — it never
        // came out of the credential store, it was just created here — and the
        // name list only catches it because `access_token` happens to be on it.
        // Declared so it stays masked even if that list changes.
        secret_paths: vec!["/access_token".to_string()],
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Credential],
        title: "Create Auth Token".to_string(),
        description: "Signs a JWT access token from input data using a stored jwt_signing_key credential. Supports HS256 and RS256 algorithms. A claim whose name ends in `:public` (e.g. `--claim \"name:public={{ input.fullname }}\"`) is the only kind exposed in the browser via `ctx.auth`; all others remain server-only. The value keeps the type its expression gives.".to_string(),
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
            "claims": { "type": "object", "description": "Map of claim_name → value. A value is a literal or {{ expr }}. End the name with `:public` (`roles:public`) to expose the claim in the browser. Claims without it are signed into the JWT but never reach the browser DOM." },
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
                ..Default::default()
            },
            crate::pipeline::model::DslFlag {
                flag: "--expires-in".to_string(),
                config_key: "expires_in".to_string(),
                description: "Token lifetime in seconds (default 900).".to_string(),
                kind: crate::pipeline::model::DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
            crate::pipeline::model::DslFlag {
                flag: "--claim".to_string(),
                config_key: "claims".to_string(),
                description: "Map a JWT claim from the input payload. Repeat for each claim. Format: claim_name={{ expr }} or claim_name=literal. End the name with :public to expose the claim in the browser via ctx.auth (e.g. --claim \"name:public={{ input.fullname }}\"); the value keeps its type. Claims without :public are signed but never reach the browser DOM. e.g. --claim \"sub={{ input.id }}\" --claim \"name:public={{ input.fullname }}\"".to_string(),
                kind: crate::pipeline::model::DslFlagKind::KeyValuePairs,
                required: false,
                ..Default::default()
            },
            crate::pipeline::model::DslFlag {
                flag: "--issuer".to_string(),
                config_key: "issuer".to_string(),
                description: "JWT issuer claim (iss).".to_string(),
                kind: crate::pipeline::model::DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
            crate::pipeline::model::DslFlag {
                flag: "--audience".to_string(),
                config_key: "audience".to_string(),
                description: "JWT audience claim (aud).".to_string(),
                kind: crate::pipeline::model::DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType, NodeFieldDataSource};
            vec![
                NodeFieldDef { name: "credential_id".to_string(), label: "Signing Credential".to_string(), field_type: NodeFieldType::Select, data_source: Some(NodeFieldDataSource::CredentialsJwt), help: Some("JWT signing key credential (kind: jwt_signing_key). Algorithm is determined by the credential.".to_string()), ..Default::default() },
                NodeFieldDef { name: "expires_in".to_string(), label: "Expires In (seconds)".to_string(), field_type: NodeFieldType::Text, placeholder: Some("900".to_string()), help: Some("Token lifetime in seconds. Defaults to 900 when omitted.".to_string()), ..Default::default() },
                NodeFieldDef { name: "issuer".to_string(), label: "Issuer (iss)".to_string(), field_type: NodeFieldType::Text, help: Some("Optional JWT issuer claim written as iss.".to_string()), ..Default::default() },
                NodeFieldDef { name: "audience".to_string(), label: "Audience (aud)".to_string(), field_type: NodeFieldType::Text, help: Some("Optional JWT audience claim written as aud.".to_string()), ..Default::default() },
                NodeFieldDef { name: "claims".to_string(), label: "Claims".to_string(), field_type: NodeFieldType::ClaimsPairs, help: Some("Map claim name → literal or {{ expr }}. Toggle \"Public\" to expose that claim in the browser via ctx.auth. Private claims (no toggle) are signed into the JWT but never reach the browser DOM.".to_string()), ..Default::default() },
            ]
        },
        layout: vec![
            LayoutItem::Field("expires_in".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("credential_id".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("issuer".to_string()), LayoutItem::Field("audience".to_string())] },
            LayoutItem::Field("claims".to_string()),
        ],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Mint a session token after login", r#"auth.token.create --credential jwt_main --expires-in 86400 --claim "sub={{ input.query.rows[0]._key }}" --claim "name:public={{ input.query.rows[0].name }}" --claim "roles:public={{ input.query.rows[0].roles }}""#)
                .output(serde_json::json!({ "access_token": "eyJhbGciOiJIUzI1NiJ9…", "token_type": "bearer", "expires_in": 86400, "profile": { "name": "Ana", "roles": ["editor"] } }))
                .note("Then `web.response.send --status 303 --header \"Location=/home\" --header \"Set-Cookie=zebflow_session={{ input.access_token }}; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly\"` (sent as written; add `; Secure` behind HTTPS). `roles` must be an array for `--auth-required-role`."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// ID of the `jwt_signing_key` credential.
    pub credential_id: String,
    /// Token lifetime in seconds (default 900).
    #[serde(default)]
    pub expires_in: Option<i64>,
    /// Map of claim_name → literal or `{{ expr }}` (resolved engine-side).
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
    pub fn new(config: Config, credentials: Arc<CredentialService>) -> Result<Self, PipelineError> {
        if config.credential_id.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_AUTH_TOKEN_CONFIG",
                "config.credential_id must not be empty",
            ));
        }
        Ok(Self {
            config,
            credentials,
        })
    }
}

/// The claims to sign, each with whether the browser may see it.
///
/// Visibility rides on the claim's name — `roles:public={{ input.roles }}` —
/// so the value stays a whole `{{ expr }}`, which the engine resolves to its
/// native type before this node runs: a list stays a list, `null` stays null,
/// an 18-digit NIP stays the text it was. The marker used to sit after the
/// value (`{{ input.roles }}:public`); that turned every public claim into a
/// sentence the engine stringified, and the node had to guess the type back.
/// The old place is refused, not read, so nothing guesses again.
fn claim_entries(claims: &Map<String, Value>) -> Result<Vec<(String, Value, bool)>, PipelineError> {
    let mut out: Vec<(String, Value, bool)> = Vec::new();
    for (key, val) in claims {
        if let Value::String(s) = val {
            if s.trim_end().ends_with(":public") {
                return Err(PipelineError::new(
                    "FW_NODE_AUTH_CLAIM_PUBLIC_ON_VALUE",
                    format!(
                        "claim '{key}': `:public` belongs on the name, not the value — \
                         write --claim \"{key}:public=…\""
                    ),
                ));
            }
        }
        let (name, public) = match key.strip_suffix(":public") {
            Some(name) => (name.trim().to_string(), true),
            None => (key.trim().to_string(), false),
        };
        if name.is_empty() {
            return Err(PipelineError::new("FW_NODE_AUTH_CLAIM_NAME", "a claim needs a name before `:public`"));
        }
        if out.iter().any(|(n, _, _)| n == &name) {
            return Err(PipelineError::new(
                "FW_NODE_AUTH_CLAIM_NAME",
                format!("claim '{name}' is given twice"),
            ));
        }
        out.push((name, val.clone(), public));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::claim_entries;
    use serde_json::{Map, Value, json};

    fn claims(v: Value) -> Map<String, Value> {
        v.as_object().cloned().unwrap()
    }

    #[test]
    fn a_public_claim_keeps_the_type_its_expression_gave() {
        // What the engine hands over after resolving whole `{{ }}` values.
        let got = claim_entries(&claims(json!({
            "roles:public": ["lecturer", "manager"],
            "identifier:public": "123456789012345678",
            "original_player_id:public": null,
            "level:public": 3,
            "nickname:public": "true",
        })))
        .unwrap();
        let find = |n: &str| got.iter().find(|(k, _, _)| k == n).cloned().unwrap();
        assert_eq!(find("roles"), ("roles".into(), json!(["lecturer", "manager"]), true));
        assert_eq!(find("identifier").1, json!("123456789012345678"));
        assert_eq!(find("original_player_id").1, Value::Null);
        assert_eq!(find("level").1, json!(3));
        assert_eq!(find("nickname").1, json!("true"), "a nickname that reads like a boolean stays text");
    }

    #[test]
    fn a_claim_without_the_marker_is_private() {
        let got = claim_entries(&claims(json!({ "highest_unit_id": "u-1" }))).unwrap();
        assert_eq!(got, vec![("highest_unit_id".into(), json!("u-1"), false)]);
    }

    #[test]
    fn the_marker_after_the_value_is_refused_with_the_fix() {
        let err = claim_entries(&claims(json!({ "roles": "[\"admin\"]:public" }))).unwrap_err();
        assert_eq!(err.code, "FW_NODE_AUTH_CLAIM_PUBLIC_ON_VALUE");
        assert!(err.message.contains("roles:public="), "the error shows where it goes: {}", err.message);
    }

    #[test]
    fn one_claim_given_public_and_private_is_refused() {
        let err = claim_entries(&claims(json!({ "roles": [], "roles:public": [] }))).unwrap_err();
        assert_eq!(err.code, "FW_NODE_AUTH_CLAIM_NAME");
    }
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
                ));
            }
        };

        // --- Build claims from input payload ---
        let mut claims_map = Map::new();
        let mut public_keys: Vec<String> = Vec::new();
        for (name, value, public) in claim_entries(&self.config.claims)? {
            if public {
                public_keys.push(name.clone());
            }
            claims_map.insert(name, value);
        }
        // Embed public claim list into the JWT so web.response.send can filter at render time.
        if !public_keys.is_empty() {
            claims_map.insert(
                "_zf_public".to_string(),
                Value::Array(
                    public_keys
                        .iter()
                        .map(|k| Value::String(k.clone()))
                        .collect(),
                ),
            );
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
                .map_err(|err| PipelineError::new("FW_NODE_AUTH_TOKEN_SIGN", err.to_string()))?
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
                jsonwebtoken::encode(&header, &claims_val, &key)
                    .map_err(|err| PipelineError::new("FW_NODE_AUTH_TOKEN_SIGN", err.to_string()))?
            }
            _ => {
                return Err(PipelineError::new(
                    "FW_NODE_AUTH_TOKEN_ALGORITHM",
                    "unsupported JWT algorithm variant",
                ));
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
            payload: crate::pipeline::nodes::shared::util::with_answer(&input.payload, output),
            trace: vec![format!(
                "auth.token.create: signed {} token, exp +{}s",
                algorithm_str, expires_in
            )],
        })
    }
}

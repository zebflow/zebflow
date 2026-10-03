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
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem};

pub const NODE_KIND: &str = "auth.token.create";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        // The minted session token. Rule 3 by value cannot see it — it never
        // came out of the credential store, it was just created here.
        secret_paths: vec!["/token/access_token".to_string()],
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Credential],
        title: "Create Auth Token".to_string(),
        description: "Signs a JWT with a stored `jwt_signing_key` credential (HS256/384/512 or RS256/384/512, as the credential says), \
            valid for `--ttl` (a duration, default 15m). Adds `token: { access_token, token_type, expires_in, profile }` and keeps the \
            rest of the payload; `expires_in` is in seconds, as OAuth writes it. A claim whose name ends in `:public` \
            (`--claim \"name:public={{ input.fullname }}\"`) is the only kind exposed in the browser via `ctx.auth`; all others stay \
            server-only. A claim's value keeps the type its expression gives."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `token` is added." }),
        output_schema: json!({ "type": "object", "properties": { "token": { "type": "object", "properties": {
            "access_token": { "type": "string" },
            "token_type": { "type": "string" },
            "expires_in": { "type": "integer", "description": "Seconds until it expires." },
            "profile": { "type": "object", "description": "The claims signed in, without iat/exp/iss/aud." }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag { required: true, ..flag("--credential", "credential_id", "The jwt_signing_key credential that signs the token.", "text") },
            flag("--ttl", "ttl", "How long the token is valid, e.g. 15m, 1h or 1d (default 15m).", "duration"),
            DslFlag {
                kind: DslFlagKind::KeyValuePairs,
                ..flag(
                    "--claim",
                    "claims",
                    "A claim to sign, repeated: name=literal or name={{ expr }} (the value keeps its type). End the name with :public to expose it in the browser via ctx.auth, e.g. --claim \"name:public={{ input.fullname }}\"; the rest are signed but never reach the browser.",
                    "expression",
                )
            },
            flag("--issuer", "issuer", "The iss claim.", "text"),
            flag("--audience", "audience", "The aud claim.", "text"),
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType, NodeFieldDataSource};
            vec![
                NodeFieldDef { name: "credential_id".to_string(), label: "Signing Credential".to_string(), field_type: NodeFieldType::Select, data_source: Some(NodeFieldDataSource::CredentialsJwt), help: Some("JWT signing key credential (kind: jwt_signing_key). Algorithm is determined by the credential.".to_string()), ..Default::default() },
                NodeFieldDef { name: "ttl".to_string(), label: "TTL".to_string(), field_type: NodeFieldType::Text, placeholder: Some("15m".to_string()), help: Some("How long the token is valid, e.g. 15m, 1h or 1d. Defaults to 15m.".to_string()), ..Default::default() },
                NodeFieldDef { name: "issuer".to_string(), label: "Issuer (iss)".to_string(), field_type: NodeFieldType::Text, help: Some("Optional JWT issuer claim written as iss.".to_string()), ..Default::default() },
                NodeFieldDef { name: "audience".to_string(), label: "Audience (aud)".to_string(), field_type: NodeFieldType::Text, help: Some("Optional JWT audience claim written as aud.".to_string()), ..Default::default() },
                NodeFieldDef { name: "claims".to_string(), label: "Claims".to_string(), field_type: NodeFieldType::ClaimsPairs, help: Some("Map claim name → literal or {{ expr }}. Toggle \"Public\" to expose that claim in the browser via ctx.auth. Private claims (no toggle) are signed into the JWT but never reach the browser DOM.".to_string()), ..Default::default() },
            ]
        },
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("credential_id".to_string()), LayoutItem::Field("ttl".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("issuer".to_string()), LayoutItem::Field("audience".to_string())] },
            LayoutItem::Field("claims".to_string()),
        ],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Mint a session token after login", r#"auth.token.create --credential jwt_main --ttl 1d --claim "sub={{ input.query.rows[0]._key }}" --claim "name:public={{ input.query.rows[0].name }}" --claim "roles:public={{ input.query.rows[0].roles }}""#)
                .output(serde_json::json!({ "token": { "access_token": "eyJhbGciOiJIUzI1NiJ9…", "token_type": "bearer", "expires_in": 86400, "profile": { "name": "Ana", "roles": ["editor"] } } }))
                .note("Then `web.response.send --status 303 --header \"Location=/home\" --header \"Set-Cookie=zebflow_session={{ input.token.access_token }}; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly\"` (sent as written; add `; Secure` behind HTTPS). `roles` must be an array for a trigger's `--role`."),
        ],
        ..Default::default()
    }
}

/// A scalar flag with its 0.11 metadata.
fn flag(name: &str, key: &str, description: &str, value: &str) -> DslFlag {
    DslFlag {
        flag: name.to_string(),
        config_key: key.to_string(),
        description: description.to_string(),
        kind: DslFlagKind::Scalar,
        value: value.to_string(),
        ..Default::default()
    }
}

/// The lifetime when `--ttl` is not given.
const DEFAULT_TTL_SECS: u64 = 15 * 60;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// ID of the `jwt_signing_key` credential.
    pub credential_id: String,
    /// How long the token is valid: a duration (default 15m).
    #[serde(default)]
    pub ttl: Value,
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
    /// `--ttl` in seconds.
    ttl: u64,
    credentials: Arc<CredentialService>,
}

impl Node {
    pub fn new(config: Config, credentials: Arc<CredentialService>) -> Result<Self, PipelineError> {
        if config.credential_id.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_AUTH_TOKEN_CREATE_CONFIG",
                "--credential is empty; it needs the jwt_signing_key credential's id",
            ));
        }
        let ttl = match &config.ttl {
            Value::Null => DEFAULT_TTL_SECS,
            Value::String(text) if text.trim().is_empty() => DEFAULT_TTL_SECS,
            Value::String(text) => {
                let duration = crate::pipeline::nodes::shared::units::duration(text, "--ttl", "FW_NODE_AUTH_TOKEN_CREATE_CONFIG")?;
                if duration.as_secs() == 0 {
                    return Err(PipelineError::new("FW_NODE_AUTH_TOKEN_CREATE_CONFIG", format!("--ttl '{text}' must be at least 1s")));
                }
                duration.as_secs()
            }
            other => {
                return Err(PipelineError::new(
                    "FW_NODE_AUTH_TOKEN_CREATE_CONFIG",
                    format!("--ttl {other} must be a duration such as 15m, 1h or 1d"),
                ));
            }
        };
        Ok(Self {
            config,
            ttl,
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
                    "FW_NODE_AUTH_TOKEN_CREATE_CLAIM_PUBLIC_ON_VALUE",
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
            return Err(PipelineError::new("FW_NODE_AUTH_TOKEN_CREATE_CLAIM_NAME", "a claim needs a name before `:public`"));
        }
        if out.iter().any(|(n, _, _)| n == &name) {
            return Err(PipelineError::new(
                "FW_NODE_AUTH_TOKEN_CREATE_CLAIM_NAME",
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
        assert_eq!(err.code, "FW_NODE_AUTH_TOKEN_CREATE_CLAIM_PUBLIC_ON_VALUE");
        assert!(err.message.contains("roles:public="), "the error shows where it goes: {}", err.message);
    }

    /// `--ttl` is a duration; the answer is `token`, the payload kept, and the
    /// minted token is masked where it sits.
    #[tokio::test]
    async fn the_token_answers_under_token_and_lives_for_ttl() {
        use crate::pipeline::nodes::{NodeExecutionInput, NodeHandler};
        let platform = crate::pipeline::nodes::shared::test_platform::test_platform();
        platform
            .credentials
            .upsert_project_credential(
                "superadmin",
                "default",
                &crate::platform::model::UpsertProjectCredentialRequest {
                    credential_id: "sign".to_string(),
                    title: "Signing key".to_string(),
                    kind: "jwt_signing_key".to_string(),
                    secret: json!({ "algorithm": "HS256", "secret": uuid::Uuid::new_v4().to_string() }),
                    notes: String::new(),
                },
            )
            .expect("credential");
        let config: super::Config = serde_json::from_value(json!({
            "credential_id": "sign", "ttl": "1h", "claims": { "sub": "u_1", "name:public": "Ana" }
        }))
        .unwrap();
        let node = super::Node::new(config, platform.credentials.clone()).expect("node");
        let out = node
            .execute_async(NodeExecutionInput {
                node_id: "t".to_string(),
                input_pin: "in".to_string(),
                payload: json!({ "kept": 1 }),
                metadata: json!({ "owner": "superadmin", "project": "default", "pipeline": "t", "request_id": "r" }),
                bus: None,
            })
            .await
            .expect("signs");
        assert_eq!(out.payload["kept"], 1);
        assert_eq!(out.payload["token"]["expires_in"], 3600);
        assert_eq!(out.payload["token"]["token_type"], "bearer");
        assert_eq!(out.payload["token"]["profile"]["name"], "Ana");
        assert!(out.payload["token"]["access_token"].as_str().is_some_and(|t| t.split('.').count() == 3));
        assert_eq!(super::definition().secret_paths, vec!["/token/access_token".to_string()]);

        let fifteen: super::Config = serde_json::from_value(json!({ "credential_id": "sign" })).unwrap();
        assert_eq!(super::Node::new(fifteen, platform.credentials.clone()).unwrap().ttl, 900, "default 15m");
        for bad in [json!("900"), json!(900), json!("0s"), json!("soon")] {
            let config: super::Config = serde_json::from_value(json!({ "credential_id": "sign", "ttl": bad })).unwrap();
            let err = super::Node::new(config, platform.credentials.clone()).err().expect("refused");
            assert_eq!(err.code, "FW_NODE_AUTH_TOKEN_CREATE_CONFIG", "{bad}");
        }
    }

    #[test]
    fn one_claim_given_public_and_private_is_refused() {
        let err = claim_entries(&claims(json!({ "roles": [], "roles:public": [] }))).unwrap_err();
        assert_eq!(err.code, "FW_NODE_AUTH_TOKEN_CREATE_CLAIM_NAME");
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
            .map_err(|err| PipelineError::new("FW_NODE_AUTH_TOKEN_CREATE_CREDENTIAL", err.to_string()))?
            .ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_AUTH_TOKEN_CREATE_CREDENTIAL_MISSING",
                    format!("credential '{}' not found", self.config.credential_id),
                )
            })?;

        if credential.kind != "jwt_signing_key" {
            return Err(PipelineError::new(
                "FW_NODE_AUTH_TOKEN_CREATE_CREDENTIAL_KIND",
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
                    "FW_NODE_AUTH_TOKEN_CREATE_ALGORITHM",
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
        let expires_in = self.ttl as i64;
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
                            "FW_NODE_AUTH_TOKEN_CREATE_SECRET_MISSING",
                            "jwt_signing_key credential missing 'secret' field",
                        )
                    })?;
                jsonwebtoken::encode(
                    &header,
                    &claims_val,
                    &EncodingKey::from_secret(secret.as_bytes()),
                )
                .map_err(|err| PipelineError::new("FW_NODE_AUTH_TOKEN_CREATE_SIGN", err.to_string()))?
            }
            Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => {
                let pem = credential
                    .secret
                    .get("private_key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        PipelineError::new(
                            "FW_NODE_AUTH_TOKEN_CREATE_KEY_MISSING",
                            "jwt_signing_key credential missing 'private_key' field",
                        )
                    })?;
                let key = EncodingKey::from_rsa_pem(pem.as_bytes()).map_err(|err| {
                    PipelineError::new("FW_NODE_AUTH_TOKEN_CREATE_KEY_INVALID", err.to_string())
                })?;
                jsonwebtoken::encode(&header, &claims_val, &key)
                    .map_err(|err| PipelineError::new("FW_NODE_AUTH_TOKEN_CREATE_SIGN", err.to_string()))?
            }
            _ => {
                return Err(PipelineError::new(
                    "FW_NODE_AUTH_TOKEN_CREATE_ALGORITHM",
                    "unsupported JWT algorithm variant",
                ));
            }
        };

        let output = json!({ "token": {
            "access_token": token,
            "token_type": "bearer",
            "expires_in": expires_in,
            "profile": profile,
        } });

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

//! JWT verification — checks a token against a stored `jwt_signing_key`.
//!
//! # Why this is a node and not three lines of script
//!
//! The platform could sign a token and not check one, which left every auth
//! pipeline to verify in `javascript.script.run`: split the JWT, HMAC the halves, compare
//! the result. That is where `alg: none` acceptance, algorithm confusion, and
//! non-constant-time comparison come from — each an ordinary mistake to make
//! and an invisible one to review. The sandbox has no constant-time compare to
//! offer even to an author who knows to want one.
//!
//! So verification is a node. It names the algorithm from the credential rather
//! than trusting the token's own header, requires `exp`, and answers on two
//! pins so a pipeline branches on the outcome instead of inspecting a flag it
//! might forget to check.

use std::sync::Arc;

use async_trait::async_trait;
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::model::{
    DslFlag, DslFlagKind, LayoutItem, NodeFieldDataSource, NodeFieldDef, NodeFieldType,
};
use crate::pipeline::model::NodeCapability;
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::CredentialService;

use crate::pipeline::nodes::shared::util::metadata_scope;

pub const NODE_KIND: &str = "auth.token.verify";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_VALID: &str = "valid";
const OUTPUT_PIN_INVALID: &str = "invalid";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Process],
        title: "Verify Token".to_string(),
        description: "Checks a JWT against a `jwt_signing_key` credential. It checks the JWT in `--from` — a password-reset or e-mail-confirmation link (`$trigger.query.token`), a token \
            posted by another system (`$trigger.body.token`) — against a `jwt_signing_key` credential. Adds `token: { valid, claims }` \
            on the `valid` pin, or `token: { valid: false, reason }` on `invalid`, and keeps the rest of the payload. The algorithm \
            comes from the credential, never from the token's header, so `alg: none` is refused; a token without `exp` is invalid. \
            To protect a route with the session cookie or a bearer header do not use this: put `--auth jwt --credential <id>` on \
            the trigger and read `$trigger.auth`."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `token` is added." }),
        output_schema: json!({ "type": "object", "properties": { "token": { "type": "object", "properties": {
            "valid": { "type": "boolean" },
            "claims": { "type": "object", "description": "The verified claims (valid pin)." },
            "reason": { "type": "string", "description": "Which check failed (invalid pin)." }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![
            OUTPUT_PIN_VALID.to_string(),
            OUTPUT_PIN_INVALID.to_string(),
        ],
        dsl_flags: vec![
            DslFlag { required: true, ..flag("--credential", "credential_id", "The jwt_signing_key credential that signed it.", "text") },
            // The token is a bearer credential — holding it is being the user
            // — and it never came from the credential store, so rule 3 by
            // value cannot see it: the run record masks the flag instead.
            DslFlag {
                required: true,
                secret: true,
                ..flag("--from", "from", "The token to check — usually {{ $trigger.query.token }} or {{ $trigger.body.token }}.", "text")
            },
            flag("--issuer", "issuer", "Require this iss claim.", "text"),
            flag("--audience", "audience", "Require this aud claim.", "text"),
        ],
        fields: vec![
            NodeFieldDef {
                name: "credential_id".to_string(),
                label: "Signing credential".to_string(),
                field_type: NodeFieldType::Select,
                data_source: Some(NodeFieldDataSource::CredentialsJwt),
                help: Some("The `jwt_signing_key` that signed this token.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "from".to_string(),
                label: "Token".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("The token to check, e.g. {{ $trigger.query.token }}.".to_string()),
                span: Some("full".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "issuer".to_string(),
                label: "Issuer".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Optional. Require this `iss`.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "audience".to_string(),
                label: "Audience".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Optional. Require this `aud`.".to_string()),
                ..Default::default()
            },
        ],
        layout: vec![
            LayoutItem::Field("credential_id".to_string()),
            LayoutItem::Field("from".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("issuer".to_string()), LayoutItem::Field("audience".to_string())] },
        ],
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Confirm an e-mail link", r#"auth.token.verify --credential jwt_main --from "{{ $trigger.query.token }}" --audience email-confirm"#)
                .output(serde_json::json!({ "token": { "valid": true, "claims": { "sub": "u_1", "aud": "email-confirm", "exp": 1789000000 } } }))
                .note("`valid` → mark the user confirmed by `input.token.claims.sub`; `invalid` → a page saying the link expired."),
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub credential_id: String,
    /// The token to check.
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub issuer: String,
    #[serde(default)]
    pub audience: String,
}

pub struct Node {
    config: Config,
    credentials: Arc<CredentialService>,
}

impl Node {
    pub fn new(config: Config, credentials: Arc<CredentialService>) -> Result<Self, PipelineError> {
        if config.credential_id.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_AUTH_TOKEN_VERIFY_CONFIG",
                "--credential is empty; it needs the jwt_signing_key credential's id",
            ));
        }
        Ok(Self {
            config,
            credentials,
        })
    }
}

/// A refusal is an answer, not a failure: an absent or expired token is the
/// ordinary case for a logged-out visitor, and a pipeline routes on it. Only a
/// misconfiguration — no credential, wrong kind — is an error.
fn invalid(payload: &Value, reason: &str) -> NodeExecutionOutput {
    NodeExecutionOutput {
        output_pins: vec![OUTPUT_PIN_INVALID.to_string()],
        payload: crate::pipeline::nodes::shared::util::with_answer(payload, json!({ "token": { "valid": false, "reason": reason } })),
        trace: vec![format!("node_kind={NODE_KIND}"), format!("invalid={reason}")],
    }
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
        &[OUTPUT_PIN_VALID, OUTPUT_PIN_INVALID]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let (owner, project, _pipeline, _request_id) = metadata_scope(&input.metadata)?;

        let credential = self
            .credentials
            .get_project_credential(owner, project, &self.config.credential_id)
            .map_err(|err| PipelineError::new("FW_NODE_AUTH_TOKEN_VERIFY_CREDENTIAL", err.to_string()))?
            .ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_AUTH_TOKEN_VERIFY_CREDENTIAL_MISSING",
                    format!("credential '{}' not found", self.config.credential_id),
                )
            })?;
        if credential.kind != "jwt_signing_key" {
            return Err(PipelineError::new(
                "FW_NODE_AUTH_TOKEN_VERIFY_CREDENTIAL_KIND",
                format!(
                    "credential '{}' is kind '{}', expected 'jwt_signing_key'",
                    credential.credential_id, credential.kind
                ),
            ));
        }

        // An empty token is a logged-out visitor, not a broken pipeline.
        let token = self.config.from.trim();
        if token.is_empty() {
            return Ok(invalid(&input.payload, "no token presented"));
        }

        // The algorithm is the credential's, never the token's. Reading `alg`
        // from the token is what lets an attacker pick `none`, or hand an
        // RS256 verifier an HS256 token signed with the public key.
        let algorithm_str = credential
            .secret
            .get("algorithm")
            .and_then(Value::as_str)
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
                    "FW_NODE_AUTH_TOKEN_VERIFY_ALGORITHM",
                    format!("unsupported JWT algorithm '{other}'"),
                ));
            }
        };

        let key = match algorithm {
            Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
                let secret = credential
                    .secret
                    .get("secret")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        PipelineError::new(
                            "FW_NODE_AUTH_TOKEN_VERIFY_SECRET_MISSING",
                            "jwt_signing_key credential missing 'secret' field",
                        )
                    })?;
                DecodingKey::from_secret(secret.as_bytes())
            }
            _ => {
                // An RSA token is verified with the public half; the private
                // key is for signing and is not what a verifier needs.
                let pem = credential
                    .secret
                    .get("public_key")
                    .or_else(|| credential.secret.get("private_key"))
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        PipelineError::new(
                            "FW_NODE_AUTH_TOKEN_VERIFY_SECRET_MISSING",
                            "jwt_signing_key credential missing 'public_key' for an RSA algorithm",
                        )
                    })?;
                DecodingKey::from_rsa_pem(pem.as_bytes()).map_err(|err| {
                    PipelineError::new("FW_NODE_AUTH_TOKEN_VERIFY_KEY", err.to_string())
                })?
            }
        };

        let mut validation = Validation::new(algorithm);
        // A session token without an expiry is a session that never ends.
        validation.required_spec_claims = ["exp"].iter().map(|s| s.to_string()).collect();
        if !self.config.issuer.trim().is_empty() {
            validation.set_issuer(&[self.config.issuer.trim()]);
        }
        if !self.config.audience.trim().is_empty() {
            validation.set_audience(&[self.config.audience.trim()]);
        }

        match jsonwebtoken::decode::<Value>(token, &key, &validation) {
            Ok(data) => {
                Ok(NodeExecutionOutput {
                    output_pins: vec![OUTPUT_PIN_VALID.to_string()],
                    payload: crate::pipeline::nodes::shared::util::with_answer(
                        &input.payload,
                        json!({ "token": { "valid": true, "claims": data.claims } }),
                    ),
                    trace: vec![format!("node_kind={NODE_KIND}"), "valid=true".to_string()],
                })
            }
            // The reason is the library's, and it names the check that failed —
            // expired, bad signature, wrong issuer — without echoing the token.
            Err(err) => Ok(invalid(&input.payload, &err.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::model::UpsertProjectCredentialRequest;
    use crate::pipeline::nodes::NodeExecutionInput;

    const KEY: &str = "test-signing-key-not-a-real-one-0123456789";

    fn platform(_tag: &str) -> crate::pipeline::nodes::shared::test_platform::TestPlatform {
        let p = crate::pipeline::nodes::shared::test_platform::test_platform();
        p.credentials
            .upsert_project_credential(
                "superadmin",
                "default",
                &UpsertProjectCredentialRequest {
                    credential_id: "sign".to_string(),
                    title: "Signing key".to_string(),
                    kind: "jwt_signing_key".to_string(),
                    secret: json!({ "algorithm": "HS256", "secret": KEY }),
                    notes: String::new(),
                },
            )
            .expect("credential");
        p
    }

    fn meta() -> Value {
        json!({ "owner": "superadmin", "project": "default", "pipeline": "t", "request_id": "r" })
    }

    async fn verify(p: &crate::pipeline::nodes::shared::test_platform::TestPlatform, token: &str) -> NodeExecutionOutput {
        Node::new(
            Config {
                credential_id: "sign".to_string(),
                from: token.to_string(),
                ..Default::default()
            },
            p.credentials.clone(),
        )
        .expect("node")
        .execute_async(NodeExecutionInput {
            node_id: "verify".to_string(),
            input_pin: INPUT_PIN_IN.to_string(),
            payload: json!({ "kept": 1 }),
            metadata: meta(),
            bus: None,
        })
        .await
        .expect("verify runs")
    }

    fn sign(claims: Value) -> String {
        jsonwebtoken::encode(
            &jsonwebtoken::Header::new(Algorithm::HS256),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(KEY.as_bytes()),
        )
        .expect("sign")
    }

    fn later() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            + 3600
    }

    #[tokio::test]
    async fn a_token_this_key_signed_is_valid() {
        let p = platform("ok");
        let out = verify(&p, &sign(json!({ "sub": "a@b.c", "exp": later() }))).await;
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_VALID.to_string()]);
        assert_eq!(out.payload["token"]["valid"], true);
        assert_eq!(out.payload["token"]["claims"]["sub"], "a@b.c");
        assert_eq!(out.payload["kept"], 1, "the payload is kept");
    }

    /// The attack this node exists to stop. A forged token is `alg: none` with
    /// an empty signature — trivially made, and accepted by any verifier that
    /// reads the algorithm out of the token it is checking.
    #[tokio::test]
    async fn an_alg_none_token_is_refused() {
        let p = platform("algnone");
        let header = b64(
            br#"{"alg":"none","typ":"JWT"}"#,
        );
        let claims = b64(
            format!(r#"{{"sub":"attacker@evil.test","exp":{}}}"#, later()).as_bytes(),
        );
        let forged = format!("{header}.{claims}.");
        let out = verify(&p, &forged).await;
        assert_eq!(
            out.output_pins,
            vec![OUTPUT_PIN_INVALID.to_string()],
            "alg:none must never verify"
        );
    }

    /// Signed with a different key — the ordinary forgery.
    #[tokio::test]
    async fn a_token_signed_with_another_key_is_refused() {
        let p = platform("wrongkey");
        let forged = jsonwebtoken::encode(
            &jsonwebtoken::Header::new(Algorithm::HS256),
            &json!({ "sub": "attacker@evil.test", "exp": later() }),
            &jsonwebtoken::EncodingKey::from_secret(b"a different key entirely"),
        )
        .expect("sign");
        let out = verify(&p, &forged).await;
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_INVALID.to_string()]);
    }

    #[tokio::test]
    async fn an_expired_token_is_refused() {
        let p = platform("expired");
        let out = verify(&p, &sign(json!({ "sub": "a@b.c", "exp": 1_000_000_000i64 }))).await;
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_INVALID.to_string()]);
        assert!(
            out.payload["token"]["reason"].as_str().unwrap_or_default().contains("Expired"),
            "reason should name the check that failed: {:?}",
            out.payload["token"]["reason"]
        );
    }

    /// A session that never ends is not a session.
    #[tokio::test]
    async fn a_token_without_an_expiry_is_refused() {
        let p = platform("noexp");
        let out = verify(&p, &sign(json!({ "sub": "a@b.c" }))).await;
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_INVALID.to_string()]);
    }

    /// A logged-out visitor is the ordinary case, not a broken pipeline.
    #[tokio::test]
    async fn no_token_routes_to_invalid_rather_than_failing() {
        let p = platform("empty");
        let out = verify(&p, "").await;
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_INVALID.to_string()]);
        assert_eq!(out.payload["token"]["reason"], "no token presented");
    }

    /// The token is a bearer credential — holding it is being the user — and it
    /// never came from the credential store, so rule 3 by value cannot see it:
    /// `--from` is declared secret, and the run record masks it.
    #[test]
    fn the_token_flag_is_declared_secret() {
        let def = definition();
        let from = def.dsl_flags.iter().find(|f| f.flag == "--from").expect("--from");
        assert!(from.secret && from.required);
        assert!(!def.dsl_flags.iter().any(|f| f.flag == "--token"), "--token is retired");
    }

    pub(super) fn b64(bytes: &[u8]) -> String {
        const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut out = String::new();
        for chunk in bytes.chunks(3) {
            let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
            let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
            out.push(A[(n >> 18) as usize & 63] as char);
            out.push(A[(n >> 12) as usize & 63] as char);
            if chunk.len() > 1 { out.push(A[(n >> 6) as usize & 63] as char); }
            if chunk.len() > 2 { out.push(A[n as usize & 63] as char); }
        }
        out
    }
}

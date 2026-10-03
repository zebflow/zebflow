//! `crypto.signature.sign` and `crypto.signature.verify` — HMAC-SHA256 of
//! `--text`, keyed by the `secret` of an `hmac` credential named by
//! `--credential`. The key never appears in a flag or the payload.
//!
//! Signing answers `signature: { algorithm, value }` (lowercase hex).
//! Verifying compares `--signature` (hex, either case) in constant time and
//! answers `signature: { valid }` on the `true` or `false` pin; a signature
//! that is not hex is simply not valid.

use std::sync::Arc;

use async_trait::async_trait;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::Sha256;

use super::{INPUT_PIN_IN, OUTPUT_PIN_FALSE, OUTPUT_PIN_OUT, OUTPUT_PIN_TRUE, answer, choice_flag, field, flag, required, text_of};
use crate::pipeline::model::{DslFlag, LayoutItem, NodeCapability, NodeExample, NodeFieldDataSource, NodeFieldDef, NodeFieldType};
use crate::pipeline::nodes::shared::{limits::choice, util::metadata_scope};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::CredentialService;

pub const SIGN_KIND: &str = "crypto.signature.sign";
pub const VERIFY_KIND: &str = "crypto.signature.verify";

/// The credential store could not be read: the world's side.
pub const SIGN_CODE: &str = "FW_NODE_CRYPTO_SIGNATURE_SIGN";
pub const SIGN_CONFIG_CODE: &str = "FW_NODE_CRYPTO_SIGNATURE_SIGN_CONFIG";
pub const SIGN_EMPTY_CODE: &str = "FW_NODE_CRYPTO_SIGNATURE_SIGN_EMPTY";
/// The credential is missing, not `hmac`, or has no secret.
pub const SIGN_CREDENTIAL_CODE: &str = "FW_NODE_CRYPTO_SIGNATURE_SIGN_CREDENTIAL";
pub const VERIFY_CODE: &str = "FW_NODE_CRYPTO_SIGNATURE_VERIFY";
pub const VERIFY_CONFIG_CODE: &str = "FW_NODE_CRYPTO_SIGNATURE_VERIFY_CONFIG";
pub const VERIFY_EMPTY_CODE: &str = "FW_NODE_CRYPTO_SIGNATURE_VERIFY_EMPTY";
pub const VERIFY_CREDENTIAL_CODE: &str = "FW_NODE_CRYPTO_SIGNATURE_VERIFY_CREDENTIAL";

const ALGORITHMS: &[&str] = &["hmac-sha256"];
/// The credential kind that holds the key.
pub const CREDENTIAL_KIND: &str = "hmac";

/// One kind's codes, so the shared steps raise the calling kind's.
struct Codes {
    world: &'static str,
    config: &'static str,
    empty: &'static str,
    credential: &'static str,
}

const SIGN: Codes = Codes { world: SIGN_CODE, config: SIGN_CONFIG_CODE, empty: SIGN_EMPTY_CODE, credential: SIGN_CREDENTIAL_CODE };
const VERIFY: Codes = Codes { world: VERIFY_CODE, config: VERIFY_CONFIG_CODE, empty: VERIFY_EMPTY_CODE, credential: VERIFY_CREDENTIAL_CODE };

fn shared_flags() -> Vec<DslFlag> {
    vec![
        DslFlag { required: true, ..flag("--text", "text", "The text to sign (its UTF-8 bytes), e.g. a raw request body.", "text") },
        DslFlag { required: true, ..flag("--credential", "credential_id", "Id of the hmac credential whose secret is the key.", "text") },
        choice_flag("--algorithm", "algorithm", "hmac-sha256 (default, the only one).", ALGORITHMS),
    ]
}

fn shared_fields() -> Vec<NodeFieldDef> {
    vec![
        field("text", "Text", "The text to sign: a literal or {{ expr }}."),
        NodeFieldDef {
            field_type: NodeFieldType::Select,
            data_source: Some(NodeFieldDataSource::CredentialsByKind(CREDENTIAL_KIND.to_string())),
            ..field("credential_id", "Credential", "An hmac credential; its secret is the key.")
        },
    ]
}

fn definition(kind: &str, title: &str, description: &str, answer: Value, verify: bool) -> NodeDefinition {
    let mut flags = shared_flags();
    let mut fields = shared_fields();
    let mut layout = vec![LayoutItem::Field("text".to_string()), LayoutItem::Field("credential_id".to_string())];
    if verify {
        flags.push(DslFlag { required: true, ..flag("--signature", "signature", "The signature to check, hex.", "text") });
        fields.push(field("signature", "Signature", "The hex signature to check, e.g. from a request header."));
        layout.push(LayoutItem::Field("signature".to_string()));
    }
    NodeDefinition {
        kind: kind.to_string(),
        capabilities: vec![NodeCapability::Credential],
        title: title.to_string(),
        description: description.to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `signature` is added." }),
        output_schema: json!({ "type": "object", "properties": { "signature": { "type": "object", "properties": answer } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: if verify {
            vec![OUTPUT_PIN_TRUE.to_string(), OUTPUT_PIN_FALSE.to_string()]
        } else {
            vec![OUTPUT_PIN_OUT.to_string()]
        },
        dsl_flags: flags,
        fields,
        layout,
        ..Default::default()
    }
}

pub fn sign_definition() -> NodeDefinition {
    NodeDefinition {
        examples: vec![
            NodeExample::dsl("Sign an outgoing callback", r#"crypto.signature.sign --text "{{ input.body_text }}" --credential partner-hmac"#)
                .output(json!({ "signature": { "algorithm": "hmac-sha256", "value": "5d41402abc4b2a76b9719d911017c592…" } })),
        ],
        ..definition(
            SIGN_KIND,
            "Sign",
            "HMAC-SHA256 of `--text`, keyed by the secret of the `hmac` credential `--credential` (an id from credential_list, never a value). \
             Adds `signature: { algorithm, value }` (lowercase hex) and keeps the rest of the payload. An empty `--text` is refused.",
            json!({ "algorithm": { "type": "string", "enum": ALGORITHMS }, "value": { "type": "string", "description": "Lowercase hex" } }),
            false,
        )
    }
}

pub fn verify_definition() -> NodeDefinition {
    NodeDefinition {
        examples: vec![
            NodeExample::dsl(
                "Check a webhook signature",
                r#"crypto.signature.verify --text "{{ input.body_text }}" --credential partner-hmac --signature "{{ input.headers['x-signature'] }}""#,
            )
            .output(json!({ "signature": { "valid": true } }))
            .note("Fires `true` or `false`; wire `false` to a 401."),
        ],
        ..definition(
            VERIFY_KIND,
            "Verify Signature",
            "Check the hex `--signature` against the HMAC-SHA256 of `--text`, keyed by the secret of the `hmac` credential `--credential`, \
             in constant time. A match answers `signature: { valid: true }` on the `true` pin, anything else `{ valid: false }` on `false`; \
             the rest of the payload is kept. An empty `--text` or `--signature` is refused.",
            json!({ "valid": { "type": "boolean" } }),
            true,
        )
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub text: Value,
    /// Id of the `hmac` credential.
    #[serde(default)]
    pub credential_id: Value,
    /// `hmac-sha256` (default).
    #[serde(default)]
    pub algorithm: String,
    /// The hex signature to check (verify only).
    #[serde(default)]
    pub signature: Value,
}

/// The flags both kinds share, checked.
struct Plan {
    text: String,
    credential_id: String,
    credentials: Arc<CredentialService>,
}

impl Plan {
    fn new(config: &Config, credentials: Arc<CredentialService>, codes: &Codes) -> Result<Self, PipelineError> {
        choice(&config.algorithm, ALGORITHMS, "hmac-sha256", "--algorithm", codes.config)?;
        let credential_id = text_of(&config.credential_id, "--credential", codes.config)?.trim().to_string();
        if credential_id.is_empty() {
            return Err(PipelineError::new(codes.empty, "--credential is empty; it needs a credential id"));
        }
        let text = required(&config.text, "--text", codes.config, codes.empty)?;
        Ok(Self { text, credential_id, credentials })
    }

    /// The MAC of the text, keyed by the credential's secret, resolved the
    /// way every credential node resolves its own: by id, in the run's project.
    fn mac(&self, metadata: &Value, codes: &Codes) -> Result<Hmac<Sha256>, PipelineError> {
        let (owner, project, _, _) = metadata_scope(metadata)?;
        let credential = self
            .credentials
            .get_project_credential(owner, project, &self.credential_id)
            .map_err(|err| PipelineError::new(codes.world, err.to_string()))?
            .ok_or_else(|| PipelineError::new(codes.credential, format!("credential '{}' not found", self.credential_id)))?;
        if credential.kind != CREDENTIAL_KIND {
            return Err(PipelineError::new(
                codes.credential,
                format!("credential '{}' is '{}' not '{CREDENTIAL_KIND}'", credential.credential_id, credential.kind),
            ));
        }
        let secret = credential.secret.get("secret").and_then(Value::as_str).unwrap_or_default();
        if secret.is_empty() {
            return Err(PipelineError::new(codes.credential, format!("credential '{}' has no secret", credential.credential_id)));
        }
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(|e| PipelineError::new(codes.credential, e.to_string()))?;
        mac.update(self.text.as_bytes());
        Ok(mac)
    }
}

/// `crypto.signature.sign`.
pub struct Sign {
    plan: Plan,
}

impl Sign {
    pub fn new(config: Config, credentials: Arc<CredentialService>) -> Result<Self, PipelineError> {
        Ok(Self { plan: Plan::new(&config, credentials, &SIGN)? })
    }
}

/// `crypto.signature.verify`.
pub struct Verify {
    plan: Plan,
    signature: String,
}

impl Verify {
    pub fn new(config: Config, credentials: Arc<CredentialService>) -> Result<Self, PipelineError> {
        let plan = Plan::new(&config, credentials, &VERIFY)?;
        let signature = required(&config.signature, "--signature", VERIFY.config, VERIFY.empty)?.trim().to_string();
        Ok(Self { plan, signature })
    }
}

#[async_trait]
impl NodeHandler for Sign {
    fn kind(&self) -> &'static str {
        SIGN_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[INPUT_PIN_IN]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        let value = hex::encode(self.plan.mac(&input.metadata, &SIGN)?.finalize().into_bytes());
        Ok(answer(
            OUTPUT_PIN_OUT,
            &input.payload,
            json!({ "signature": { "algorithm": ALGORITHMS[0], "value": value } }),
            format!("node_kind={SIGN_KIND} credential={}", self.plan.credential_id),
        ))
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
        let mac = self.plan.mac(&input.metadata, &VERIFY)?;
        // `verify_slice` compares in constant time.
        let valid = hex::decode(&self.signature).is_ok_and(|expected| mac.verify_slice(&expected).is_ok());
        Ok(answer(
            if valid { OUTPUT_PIN_TRUE } else { OUTPUT_PIN_FALSE },
            &input.payload,
            json!({ "signature": { "valid": valid } }),
            format!("node_kind={VERIFY_KIND} credential={} valid={valid}", self.plan.credential_id),
        ))
    }
}

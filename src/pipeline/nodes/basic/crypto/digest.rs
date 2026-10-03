//! `crypto.digest.create` — the SHA-256 or SHA-512 digest of `--text`, as
//! lowercase hex. The answer is `digest: { algorithm, value }`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256, Sha512};

use super::{INPUT_PIN_IN, OUTPUT_PIN_OUT, answer, choice_flag, field, flag, required, select};
use crate::pipeline::model::{DslFlag, LayoutItem, NodeExample};
use crate::pipeline::nodes::shared::limits::choice;
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "crypto.digest.create";
/// A flag set wrong: an unknown algorithm, or `--text` that is not text.
pub const CONFIG_CODE: &str = "FW_NODE_CRYPTO_DIGEST_CREATE_CONFIG";
/// `--text` is empty.
pub const EMPTY_CODE: &str = "FW_NODE_CRYPTO_DIGEST_CREATE_EMPTY";

const ALGORITHMS: &[&str] = &["sha256", "sha512"];

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Digest".to_string(),
        description: "The digest of `--text` with `--algorithm sha256` (default) or `sha512`, as lowercase hex. \
            Adds `digest: { algorithm, value }` and keeps the rest of the payload. An empty `--text` is refused. \
            A digest is not a password hash: store passwords with `crypto.password.hash`."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `digest` is added." }),
        output_schema: json!({ "type": "object", "properties": { "digest": { "type": "object", "properties": {
            "algorithm": { "type": "string", "enum": ALGORITHMS },
            "value": { "type": "string", "description": "Lowercase hex" }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![
            DslFlag { required: true, ..flag("--text", "text", "The text to digest (its UTF-8 bytes).", "text") },
            choice_flag("--algorithm", "algorithm", "sha256 (default) or sha512.", ALGORITHMS),
        ],
        fields: vec![
            field("text", "Text", "The text to digest: a literal or {{ expr }}."),
            select("algorithm", "Algorithm", "SHA-256 or SHA-512.", ALGORITHMS),
        ],
        layout: vec![LayoutItem::Field("text".to_string()), LayoutItem::Field("algorithm".to_string())],
        examples: vec![
            NodeExample::dsl("A content fingerprint", r#"crypto.digest.create --text "{{ input.body.content }}""#)
                .output(json!({ "digest": { "algorithm": "sha256", "value": "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824" } })),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub text: Value,
    /// `sha256` (default) or `sha512`.
    #[serde(default)]
    pub algorithm: String,
}

pub struct Node {
    text: String,
    algorithm: &'static str,
}

impl Node {
    pub fn new(config: Config) -> Result<Self, PipelineError> {
        let algorithm = choice(&config.algorithm, ALGORITHMS, "sha256", "--algorithm", CONFIG_CODE)?;
        let text = required(&config.text, "--text", CONFIG_CODE, EMPTY_CODE)?;
        Ok(Self { text, algorithm })
    }
}

/// The hex digest of `text`.
pub fn digest(algorithm: &str, text: &str) -> String {
    match algorithm {
        "sha512" => hex::encode(Sha512::digest(text.as_bytes())),
        _ => hex::encode(Sha256::digest(text.as_bytes())),
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
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        let value = digest(self.algorithm, &self.text);
        Ok(answer(
            OUTPUT_PIN_OUT,
            &input.payload,
            json!({ "digest": { "algorithm": self.algorithm, "value": value } }),
            format!("node_kind={NODE_KIND} algorithm={}", self.algorithm),
        ))
    }
}

//! `crypto.base64.encode` and `crypto.base64.decode` — standard Base64
//! (RFC 4648, padded) of UTF-8 text. Encoding answers `base64: { value }`;
//! decoding answers `base64: { text }` and refuses a string that is not
//! Base64 or does not decode to UTF-8 text.

use ::base64::{Engine as _, engine::general_purpose::STANDARD};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{INPUT_PIN_IN, OUTPUT_PIN_OUT, answer, field, flag, required};
use crate::pipeline::model::{DslFlag, LayoutItem, NodeExample};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const ENCODE_KIND: &str = "crypto.base64.encode";
pub const DECODE_KIND: &str = "crypto.base64.decode";

/// `--text` is not text.
pub const ENCODE_CONFIG_CODE: &str = "FW_NODE_CRYPTO_BASE64_ENCODE_CONFIG";
/// `--text` is empty.
pub const ENCODE_EMPTY_CODE: &str = "FW_NODE_CRYPTO_BASE64_ENCODE_EMPTY";
/// `--from` is not text.
pub const DECODE_CONFIG_CODE: &str = "FW_NODE_CRYPTO_BASE64_DECODE_CONFIG";
/// `--from` is empty.
pub const DECODE_EMPTY_CODE: &str = "FW_NODE_CRYPTO_BASE64_DECODE_EMPTY";
/// `--from` is not Base64, or its bytes are not UTF-8 text.
pub const DECODE_INVALID_CODE: &str = "FW_NODE_CRYPTO_BASE64_DECODE_INVALID";

pub fn encode_definition() -> NodeDefinition {
    NodeDefinition {
        kind: ENCODE_KIND.to_string(),
        title: "Base64 Encode".to_string(),
        description: "Standard Base64 (padded) of the UTF-8 bytes of `--text`. Adds `base64: { value }` and keeps the rest of the payload. \
            An empty `--text` is refused. Base64 is an encoding, not a secret: anyone can decode it."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `base64` is added." }),
        output_schema: json!({ "type": "object", "properties": { "base64": { "type": "object", "properties": {
            "value": { "type": "string" }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![DslFlag { required: true, ..flag("--text", "text", "The text to encode.", "text") }],
        fields: vec![field("text", "Text", "The text to encode: a literal or {{ expr }}.")],
        layout: vec![LayoutItem::Field("text".to_string())],
        examples: vec![
            NodeExample::dsl("A Basic auth value", r#"crypto.base64.encode --text "{{ input.user + ':' + input.token }}""#)
                .output(json!({ "base64": { "value": "ZGVtbzpzZWNyZXQ=" } })),
        ],
        ..Default::default()
    }
}

pub fn decode_definition() -> NodeDefinition {
    NodeDefinition {
        kind: DECODE_KIND.to_string(),
        title: "Base64 Decode".to_string(),
        description: "Decode the standard Base64 string in `--from` to UTF-8 text. Adds `base64: { text }` and keeps the rest of the payload. \
            An empty `--from`, a string that is not Base64, or bytes that are not UTF-8 text are refused."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `base64` is added." }),
        output_schema: json!({ "type": "object", "properties": { "base64": { "type": "object", "properties": {
            "text": { "type": "string" }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![DslFlag { required: true, ..flag("--from", "from", "The Base64 string to decode.", "text") }],
        fields: vec![field("from", "Base64", "The Base64 string: a literal or {{ expr }}.")],
        layout: vec![LayoutItem::Field("from".to_string())],
        examples: vec![
            NodeExample::dsl("Read an encoded field", r#"crypto.base64.decode --from "{{ $trigger.body.data }}""#)
                .input(json!({ "body": { "data": "aGVsbG8=" } }))
                .output(json!({ "body": { "data": "aGVsbG8=" }, "base64": { "text": "hello" } })),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EncodeConfig {
    #[serde(default)]
    pub text: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecodeConfig {
    #[serde(default)]
    pub from: Value,
}

pub struct Encode {
    text: String,
}

impl Encode {
    pub fn new(config: EncodeConfig) -> Result<Self, PipelineError> {
        Ok(Self { text: required(&config.text, "--text", ENCODE_CONFIG_CODE, ENCODE_EMPTY_CODE)? })
    }
}

pub struct Decode {
    from: String,
}

impl Decode {
    pub fn new(config: DecodeConfig) -> Result<Self, PipelineError> {
        Ok(Self { from: required(&config.from, "--from", DECODE_CONFIG_CODE, DECODE_EMPTY_CODE)? })
    }
}

#[async_trait]
impl NodeHandler for Encode {
    fn kind(&self) -> &'static str {
        ENCODE_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[INPUT_PIN_IN]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        let value = STANDARD.encode(self.text.as_bytes());
        Ok(answer(OUTPUT_PIN_OUT, &input.payload, json!({ "base64": { "value": value } }), format!("node_kind={ENCODE_KIND}")))
    }
}

#[async_trait]
impl NodeHandler for Decode {
    fn kind(&self) -> &'static str {
        DECODE_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[INPUT_PIN_IN]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        let bytes = STANDARD
            .decode(self.from.trim().as_bytes())
            .map_err(|e| PipelineError::new(DECODE_INVALID_CODE, format!("--from is not Base64: {e}")))?;
        let text = String::from_utf8(bytes)
            .map_err(|e| PipelineError::new(DECODE_INVALID_CODE, format!("--from does not decode to UTF-8 text: {e}")))?;
        Ok(answer(OUTPUT_PIN_OUT, &input.payload, json!({ "base64": { "text": text } }), format!("node_kind={DECODE_KIND}")))
    }
}

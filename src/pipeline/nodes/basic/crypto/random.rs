//! `crypto.random.generate` — `--size` cryptographically random bytes
//! (default `32B`, at most `1KiB`), as `--encoding hex` (default) or
//! `base64`. The answer is `random: { value }`.

use ::base64::{Engine as _, engine::general_purpose::STANDARD};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{INPUT_PIN_IN, OUTPUT_PIN_OUT, answer, choice_flag, field, flag, select, text_of};
use crate::pipeline::model::{LayoutItem, NodeExample};
use crate::pipeline::nodes::shared::{limits::choice, units};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "crypto.random.generate";
/// A flag set wrong: a size without a unit, zero or over the ceiling, or an unknown encoding.
pub const CONFIG_CODE: &str = "FW_NODE_CRYPTO_RANDOM_GENERATE_CONFIG";

const ENCODINGS: &[&str] = &["hex", "base64"];
const DEFAULT_SIZE: &str = "32B";
/// The ceiling on `--size`: 1KiB.
pub const MAX_BYTES: u64 = 1024;

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Random".to_string(),
        description: "Cryptographically random bytes: `--size` (default 32B, at most 1KiB) as `--encoding hex` (default) or `base64`. \
            For tokens, OAuth `state`, nonces and salts. Adds `random: { value }` and keeps the rest of the payload."
            .to_string(),
        // A generated token is often a session or reset token: masked in the
        // run record, never in what the next node receives.
        secret_paths: vec!["/random/value".to_string()],
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `random` is added." }),
        output_schema: json!({ "type": "object", "properties": { "random": { "type": "object", "properties": {
            "value": { "type": "string", "description": "The bytes, hex or base64" }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![
            flag("--size", "size", "How many random bytes, with a unit: 16B, 32B (default), at most 1KiB.", "size"),
            choice_flag("--encoding", "encoding", "hex (default) or base64.", ENCODINGS),
        ],
        fields: vec![
            field("size", "Size", "Random bytes with a unit, e.g. 32B (default); at most 1KiB."),
            select("encoding", "Encoding", "hex: two characters a byte; base64: shorter.", ENCODINGS),
        ],
        layout: vec![LayoutItem::Row { row: vec![LayoutItem::Field("size".to_string()), LayoutItem::Field("encoding".to_string())] }],
        examples: vec![
            NodeExample::dsl("An OAuth state", "crypto.random.generate --size 16B")
                .output(json!({ "random": { "value": "9f2c41d07be35a86c1e0f4a2b9d3e781" } })),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// A size with a unit (`32B`).
    #[serde(default)]
    pub size: Value,
    /// `hex` (default) or `base64`.
    #[serde(default)]
    pub encoding: String,
}

pub struct Node {
    bytes: usize,
    encoding: &'static str,
}

impl Node {
    pub fn new(config: Config) -> Result<Self, PipelineError> {
        let size = text_of(&config.size, "--size", CONFIG_CODE)?;
        let size = if size.trim().is_empty() { DEFAULT_SIZE.to_string() } else { size };
        let bytes = units::size(&size, "--size", CONFIG_CODE)?;
        // An empty token is not a token, and every size has a ceiling.
        if bytes == 0 || bytes > MAX_BYTES {
            return Err(PipelineError::new(CONFIG_CODE, format!("--size {size} must be between 1B and 1KiB")));
        }
        let encoding = choice(&config.encoding, ENCODINGS, "hex", "--encoding", CONFIG_CODE)?;
        Ok(Self { bytes: bytes as usize, encoding })
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
        let bytes: Vec<u8> = {
            use rand::RngExt;
            let mut rng = rand::rng();
            (0..self.bytes).map(|_| rng.random::<u8>()).collect()
        };
        let value = match self.encoding {
            "base64" => STANDARD.encode(&bytes),
            _ => hex::encode(&bytes),
        };
        Ok(answer(
            OUTPUT_PIN_OUT,
            &input.payload,
            json!({ "random": { "value": value } }),
            format!("node_kind={NODE_KIND} bytes={} encoding={}", self.bytes, self.encoding),
        ))
    }
}

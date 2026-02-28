//! Schedule trigger node.

use serde::{Deserialize, Serialize};

use crate::framework::{FrameworkError, nodes::{FrameworkNode, NodeExecutionInput, NodeExecutionOutput}};

pub const NODE_KIND: &str = "x.n.trigger.schedule";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub cron: String,
    #[serde(default)]
    pub timezone: String,
}

pub struct Node {
    config: Config,
}

impl Node {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl FrameworkNode for Node {
    fn kind(&self) -> &'static str { NODE_KIND }
    fn input_pins(&self) -> &'static [&'static str] { &[INPUT_PIN_IN] }
    fn output_pins(&self) -> &'static [&'static str] { &[OUTPUT_PIN_OUT] }

    fn execute(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, FrameworkError> {
        if input.input_pin != INPUT_PIN_IN {
            return Err(FrameworkError::new(
                "FW_NODE_TRIGGER_SCHEDULE_INPUT_PIN",
                format!("unsupported input pin '{}'", input.input_pin),
            ));
        }
        Ok(NodeExecutionOutput {
            output_pin: OUTPUT_PIN_OUT.to_string(),
            payload: input.payload,
            trace: vec![
                format!("node_kind={NODE_KIND}"),
                format!("cron={}", self.config.cron),
                format!("timezone={}", self.config.timezone),
            ],
        })
    }
}

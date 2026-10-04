//! `trigger.socket` — connect to an external WebSocket server and fire the pipeline on each message.
//!
//! This is a **passthrough trigger node** — it does not do any work at execution time.
//! The real connection happens in [`crate::infra::ws_client::WsClientManager`],
//! which spawns a background tokio task when the pipeline is activated.
//!
//! When a message arrives from the external WS server, the background task fires
//! this pipeline, and the trigger answers one key, `socket`:
//!
//! ```json
//! {
//!   "socket": {
//!     "node_id": "<node-id>",
//!     "url": "wss://...",
//!     "message": { ... }
//!   }
//! }
//! ```
//!
//! Downstream nodes read the received data at `input.socket.message` (or
//! `$trigger.message`).
//!
//! # Reconnecting
//!
//! A socket reconnects with capped exponential backoff starting at `--delay`
//! (default `5s`). `--max-attempts` bounds it: omitted = unlimited, `0` =
//! never reconnect, `N` = at most N reconnects.
//!
//! # Example
//!
//! ```text
//! | trigger.socket --url wss://stream.example.com/feed
//! | ws.message.send --room dashboard --event feed --body "{{ input.socket.message }}"
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::model::NodeCapability;
use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType, SelectOptionDef};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "trigger.socket";
const OUTPUT_PIN_OUT: &str = "out";
/// The key this trigger answers under.
pub const ANSWER_KEY: &str = "socket";
/// Raised when a socket's flags are refused at activation.
pub const CONFIG_CODE: &str = "FW_NODE_TRIGGER_SOCKET_CONFIG";
/// `--parse`: how a received message is read.
pub const PARSE_WORDS: [&str; 2] = ["json", "text"];
/// The first reconnect waits this long unless `--delay` says otherwise.
pub const DEFAULT_DELAY: &str = "5s";
/// A ping goes out this often unless `--heartbeat` says otherwise.
pub const DEFAULT_HEARTBEAT: &str = "30s";

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

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Network, NodeCapability::Credential],
        title: "Socket Trigger".to_string(),
        description: "Runs the pipeline for every message from an outbound WebSocket. It keeps an outbound WebSocket connection to another server open while the pipeline is active and runs the pipeline \
            for every message it receives — price feeds, a broker, another Zebflow. `--url` is `ws://` or `wss://`; `--credential` \
            supplies auth if the server needs it. A socket reconnects with backoff starting at `--delay` (default 5s); `--max-attempts` \
            bounds it (omitted = unlimited, 0 = never reconnect, N = at most N reconnects). Answers one key, `socket`: \
            `{ url, message, node_id }` — the message is `input.socket.message` (`$trigger.message`), parsed \
            JSON with `--parse json` (default), a string with `text`. To send back on the same connection use \
            `ws.message.send --connection <this node id>`. Not for browsers: that is `trigger.room`."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "socket": {
                    "type": "object",
                    "properties": {
                        "url": { "type": "string" },
                        "node_id": { "type": "string" },
                        "message": { "description": "Received WebSocket message payload." }
                    }
                }
            }
        }),
        input_pins: vec![],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({
            "type": "object",
            "required": ["url"],
            "properties": {
                "url": { "type": "string", "description": "WebSocket server URL (ws:// or wss://)." },
                "credential_id": { "type": "string", "description": "Credential for auth headers/tokens." },
                "delay": { "type": "string", "description": "First reconnect delay, a duration (5s); backoff doubles it up to 60s. Default: 5s." },
                "max_attempts": { "type": "integer", "description": "Reconnects allowed: omitted = unlimited, 0 = never reconnect, N = at most N." },
                "heartbeat": { "type": "string", "description": "Ping interval, a duration (30s). Default: 30s." },
                "parse": { "type": "string", "enum": PARSE_WORDS, "description": "How a message is read: json (default) or text." }
            }
        }),
        dsl_flags: vec![
            DslFlag { required: true, ..flag("--url", "url", "WebSocket server URL (ws:// or wss://).", "text") },
            flag("--credential", "credential_id", "Credential id for auth headers/tokens.", "text"),
            flag("--delay", "delay", "First reconnect delay (`5s`, `500ms`); backoff doubles it up to 60s. Default: 5s.", "duration"),
            flag("--max-attempts", "max_attempts", "Reconnects allowed: omitted = unlimited, 0 = never reconnect, N = at most N.", "number"),
            flag("--heartbeat", "heartbeat", "Ping interval (`30s`). Default: 30s.", "duration"),
            DslFlag {
                choices: PARSE_WORDS.iter().map(|w| w.to_string()).collect(),
                ..flag("--parse", "parse", "How a received message is read: json (default) or text.", "text")
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "url".to_string(),
                label: "URL".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("WebSocket server URL (ws:// or wss://).".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "credential_id".to_string(),
                label: "Credential".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Credential ID for auth headers/tokens.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "delay".to_string(),
                label: "Reconnect delay".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some(DEFAULT_DELAY.to_string()),
                help: Some("First reconnect delay, a duration (5s); backoff doubles it up to 60s.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "max_attempts".to_string(),
                label: "Max attempts".to_string(),
                field_type: NodeFieldType::Number,
                help: Some("Reconnects allowed. Empty: unlimited; 0: never reconnect.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "heartbeat".to_string(),
                label: "Heartbeat".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some(DEFAULT_HEARTBEAT.to_string()),
                help: Some("Ping interval, a duration (30s).".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "parse".to_string(),
                label: "Parse".to_string(),
                field_type: NodeFieldType::Select,
                options: PARSE_WORDS.iter().map(|w| SelectOptionDef { value: w.to_string(), label: w.to_string() }).collect(),
                help: Some("How a received message is read: json or text.".to_string()),
                ..Default::default()
            },
        ],
        layout: vec![],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Follow a price feed", r#"trigger.socket --url wss://feed.example.com/ticks --parse json --delay 2s --max-attempts 10"#)
                .output(serde_json::json!({ "socket": { "url": "wss://feed.example.com/ticks", "message": { "symbol": "AUDUSD", "bid": 0.6512 }, "node_id": "n0" } })),
        ],
        ..Default::default()
    }
}

/// The node's config as stored: durations stay text until [`Settings::of`]
/// reads them.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub credential_id: String,
    #[serde(default)]
    pub delay: Value,
    #[serde(default)]
    pub max_attempts: Value,
    #[serde(default)]
    pub heartbeat: Value,
    #[serde(default)]
    pub parse: String,
}

/// What the connection manager runs with, read from the flags once, at
/// activation: a bad duration or an unknown `--parse` word refuses the
/// pipeline there rather than at the first message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub delay_ms: u64,
    /// `None`: unlimited. `Some(0)`: never reconnect.
    pub max_attempts: Option<u64>,
    pub heartbeat_ms: u64,
    pub parse: String,
}

impl Settings {
    pub fn of(config: &Value) -> Result<Self, PipelineError> {
        let config: Config = serde_json::from_value(config.clone())
            .map_err(|err| PipelineError::new(CONFIG_CODE, err.to_string()))?;
        let duration_ms = |value: &Value, flag: &str, default: &str| -> Result<u64, PipelineError> {
            let text = match value {
                Value::Null => default.to_string(),
                Value::String(s) if s.trim().is_empty() => default.to_string(),
                Value::String(s) => s.trim().to_string(),
                other => other.to_string(),
            };
            let duration = crate::pipeline::nodes::shared::units::duration(&text, flag, CONFIG_CODE)?;
            Ok(duration.as_millis().max(1) as u64)
        };
        let max_attempts = match &config.max_attempts {
            Value::Null => None,
            Value::String(s) if s.trim().is_empty() => None,
            Value::Number(n) => Some(n.as_u64().ok_or_else(|| {
                PipelineError::new(CONFIG_CODE, format!("--max-attempts must be a whole number of 0 or more, got {n}"))
            })?),
            Value::String(s) => Some(s.trim().parse::<u64>().map_err(|_| {
                PipelineError::new(CONFIG_CODE, format!("--max-attempts must be a whole number of 0 or more, got '{s}'"))
            })?),
            other => {
                return Err(PipelineError::new(CONFIG_CODE, format!("--max-attempts must be a number, got {other}")));
            }
        };
        let parse = crate::pipeline::nodes::shared::limits::choice(&config.parse, &PARSE_WORDS, "json", "--parse", CONFIG_CODE)?;
        Ok(Self {
            delay_ms: duration_ms(&config.delay, "--delay", DEFAULT_DELAY)?,
            max_attempts,
            heartbeat_ms: duration_ms(&config.heartbeat, "--heartbeat", DEFAULT_HEARTBEAT)?,
            parse: parse.to_string(),
        })
    }
}

pub struct Node {
    #[allow(dead_code)]
    config: Config,
}

impl Node {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        NODE_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        // The envelope was injected by the WsClientManager background task.
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: super::answer_under(ANSWER_KEY, input.payload),
            trace: vec![format!("trigger.socket: url={}", self.config.url)],
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Settings;

    #[test]
    fn durations_and_attempts_read_as_written() {
        let s = Settings::of(&json!({ "url": "wss://feed.example.com", "delay": "2s", "heartbeat": "500ms", "max_attempts": 3 })).unwrap();
        assert_eq!(s, Settings { delay_ms: 2000, max_attempts: Some(3), heartbeat_ms: 500, parse: "json".into() });
        let s = Settings::of(&json!({ "url": "wss://feed.example.com" })).unwrap();
        assert_eq!((s.delay_ms, s.max_attempts, s.heartbeat_ms), (5000, None, 30000), "omitted: 5s, unlimited, 30s");
        let s = Settings::of(&json!({ "url": "wss://feed.example.com", "max_attempts": 0, "parse": "text" })).unwrap();
        assert_eq!((s.max_attempts, s.parse.as_str()), (Some(0), "text"), "0 = never reconnect");
    }

    #[test]
    fn a_bad_duration_or_word_is_refused() {
        for bad in [json!({ "delay": "5 parsecs" }), json!({ "heartbeat": "soon" }), json!({ "parse": "xml" }), json!({ "max_attempts": -1 })] {
            let err = Settings::of(&bad).unwrap_err();
            assert_eq!(err.code, super::CONFIG_CODE, "{bad}");
        }
    }
}

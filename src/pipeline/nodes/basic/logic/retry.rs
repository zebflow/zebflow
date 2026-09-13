//! `n.logic.retry` — bounded retry control for a failing upstream node.
//!
//! The engine routes a node failure into `:error` edges when they exist. `n.logic.retry`
//! consumes that failure envelope and either:
//!
//! - emits `retry` with the original upstream input plus updated retry metadata
//! - emits `failed` when the attempt budget is exhausted

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "n.logic.retry";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_RETRY: &str = "retry";
pub const OUTPUT_PIN_FAILED: &str = "failed";
const RETRY_STATE_KEY: &str = "__zf_retry";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Retry".to_string(),
        description: "Retry a failed node. Wire it from the failing node's `:error` pin (`[b]:error -> [r]`); it receives the failure \
             envelope and fires `retry` with the original input (wire `[r]:retry -> [b]` to run it again) until `--max-attempts` \
             is spent, then fires `failed` with the last error. `--delay-ms` waits between attempts. Only nodes with an `error` \
             pin, or any node in graph mode with an `:error` edge, can feed it; a retry loop without a `failed` edge swallows the error."
            .to_string(),
        input_schema: serde_json::json!({ "type": "object" }),
        output_schema: serde_json::json!({ "type": "object" }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_RETRY.to_string(), OUTPUT_PIN_FAILED.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag {
                flag: "--max-attempts".to_string(),
                config_key: "max_attempts".to_string(),
                description: "Maximum total attempts before routing to failed.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--delay-ms".to_string(),
                config_key: "delay_ms".to_string(),
                description: "Optional delay before retrying.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType};
            vec![
                NodeFieldDef {
                    name: "max_attempts".to_string(),
                    label: "Max Attempts".to_string(),
                    field_type: NodeFieldType::Text,
                    help: Some(
                        "Maximum total attempts including the original failed try.".to_string(),
                    ),
                    ..Default::default()
                },
                NodeFieldDef {
                    name: "delay_ms".to_string(),
                    label: "Delay (ms)".to_string(),
                    field_type: NodeFieldType::Text,
                    help: Some("Optional delay before emitting the retry path.".to_string()),
                    ..Default::default()
                },
            ]
        },
        layout: vec![LayoutItem::Row {
            row: vec![
                LayoutItem::Field("max_attempts".to_string()),
                LayoutItem::Field("delay_ms".to_string()),
            ],
        }],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Three attempts at a flaky API", "logic.retry --max-attempts 3 --delay-ms 500")
                .note("Graph: `[b] http.request …`, `[b]:error -> [r]`, `[r]:retry -> [b]`, `[r]:failed -> [e] web.response --status 502`."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub max_attempts: usize,
    #[serde(default)]
    pub delay_ms: Option<u64>,
}

pub struct Node {
    config: Config,
}

impl Node {
    pub fn new(config: Config) -> Result<Self, PipelineError> {
        if config.max_attempts == 0 {
            return Err(PipelineError::new(
                "FW_NODE_LOGIC_RETRY_CONFIG",
                "max_attempts must be greater than 0",
            ));
        }
        Ok(Self { config })
    }
}

fn retry_state(payload: &Value) -> Option<&Value> {
    payload.get(RETRY_STATE_KEY)
}

fn restore_retry_payload(payload: &Value) -> Result<Value, PipelineError> {
    let original = payload.get("input").cloned().ok_or_else(|| {
        PipelineError::new(
            "FW_NODE_LOGIC_RETRY_INPUT",
            "retry input payload missing `input`",
        )
    })?;
    let state = retry_state(payload).cloned().ok_or_else(|| {
        PipelineError::new(
            "FW_NODE_LOGIC_RETRY_INPUT",
            "retry input payload missing internal retry state",
        )
    })?;

    Ok(match original {
        Value::Object(mut map) => {
            map.insert(RETRY_STATE_KEY.to_string(), state);
            Value::Object(map)
        }
        other => json!({
            "input": other,
            RETRY_STATE_KEY: state,
        }),
    })
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
        &[OUTPUT_PIN_RETRY, OUTPUT_PIN_FAILED]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let attempt = retry_state(&input.payload)
            .and_then(|value| value.get("attempt"))
            .and_then(Value::as_u64)
            .unwrap_or(1) as usize;

        let mut trace = vec![
            format!("node_kind={NODE_KIND}"),
            format!("attempt={attempt}"),
            format!("max_attempts={}", self.config.max_attempts),
        ];

        // A refusal is the caller's fault — a bad address, a wrong credential
        // kind, config that cannot work. Running it again changes nothing, so
        // the attempt budget is not spent on it: straight to the failed pin,
        // with the reason in the trace. Only `failed`-class errors (the world's
        // fault — a relay down, a timeout) are worth another try.
        let error_code = input
            .payload
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(Value::as_str);
        if let Some(code) = error_code {
            use crate::pipeline::error_class::{ErrorClass, class_of};
            if class_of(code) == ErrorClass::Refused {
                trace.push(format!("refused: {code} — retrying cannot help"));
                return Ok(NodeExecutionOutput {
                    output_pins: vec![OUTPUT_PIN_FAILED.to_string()],
                    payload: input.payload,
                    trace,
                });
            }
        }

        if attempt < self.config.max_attempts {
            if let Some(delay_ms) = self.config.delay_ms.filter(|delay| *delay > 0) {
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
            return Ok(NodeExecutionOutput {
                output_pins: vec![OUTPUT_PIN_RETRY.to_string()],
                payload: restore_retry_payload(&input.payload)?,
                trace,
            });
        }

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_FAILED.to_string()],
            payload: input.payload,
            trace,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(node: &Node, payload: serde_json::Value) -> NodeExecutionOutput {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(node.execute_async(NodeExecutionInput {
                node_id: "r".to_string(),
                input_pin: INPUT_PIN_IN.to_string(),
                payload,
                metadata: json!({}),
                bus: None,
            }))
            .expect("retry node never errors")
    }

    /// A relay being down is worth another try.
    #[test]
    fn a_failed_class_error_is_retried() {
        let node = Node::new(Config { max_attempts: 3, delay_ms: None }).expect("node");
        let out = run(&node, json!({
            "input": { "email": "sari@example.test" },
            "error": { "code": "FW_NODE_MAIL_SEND", "message": "relay unreachable" },
            "__zf_retry": { "attempt": 1 }
        }));
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_RETRY.to_string()]);
    }

    /// A bad address is the caller's fault. Retrying cannot help, so the
    /// attempt budget is not spent discovering that three times.
    #[test]
    fn a_refused_class_error_goes_straight_to_failed() {
        let node = Node::new(Config { max_attempts: 3, delay_ms: None }).expect("node");
        let out = run(&node, json!({
            "input": { "email": "not an address" },
            "error": { "code": "FW_NODE_MAIL_ADDRESS", "message": "not a mailbox" },
            "__zf_retry": { "attempt": 1 }
        }));
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_FAILED.to_string()]);
        assert!(
            out.trace.iter().any(|t| t.contains("retrying cannot help")),
            "{:?}",
            out.trace
        );
    }
}

//! Schedule trigger node.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "n.trigger.schedule";
pub const OUTPUT_PIN_OUT: &str = "out";

/// Unified node-definition metadata for `n.trigger.schedule`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Schedule Trigger".to_string(),
        description: "Starts the pipeline on a cron schedule once it is active: `--cron` is five fields (`0 7 * * *` = 07:00 daily), \
            `--timezone` an IANA name (default UTC). The payload is `{ trigger: \"schedule\", fired_at: <RFC 3339>, node_id }` — \
            there is no `body`, no request; anything the job needs it reads from the database or KV. A scheduled pipeline must not \
            end in a page (`web.response --template`); it ends in a write, a mail, or a bare `web.response` summary. Runs show under \
            `pipeline_get_invocations` with trigger `schedule`; the Studio's Schedules tab lists them."
            .to_string(),
        input_schema: serde_json::json!({
            "type":"object",
            "description":"Schedule tick payload."
        }),
        output_schema: serde_json::json!({
            "type":"object",
            "description":"Unmodified tick payload for downstream nodes."
        }),
        input_pins: vec![],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag {
                flag: "--cron".to_string(),
                config_key: "cron".to_string(),
                description:
                    "Cron expression (e.g. '* * * * *' = every minute, '0 * * * *' = hourly)."
                        .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--timezone".to_string(),
                config_key: "timezone".to_string(),
                description: "IANA timezone, e.g. UTC or Asia/Jakarta.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "cron".to_string(),
                label: "Cron".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Cron expression for schedule trigger.".to_string()),
                default_value: Some(serde_json::json!("*/5 * * * *")),
                ..Default::default()
            },
            NodeFieldDef {
                name: "timezone".to_string(),
                label: "Timezone".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("IANA timezone, for example UTC or Asia/Jakarta.".to_string()),
                default_value: Some(serde_json::json!("UTC")),
                ..Default::default()
            },
        ],
        layout: vec![LayoutItem::Row {
            row: vec![
                LayoutItem::Field("cron".to_string()),
                LayoutItem::Field("timezone".to_string()),
            ],
        }],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Daily digest at 07:00 Melbourne time", r#"trigger.schedule --cron "0 7 * * *" --timezone Australia/Melbourne"#)
                .output(serde_json::json!({ "trigger": "schedule", "fired_at": "2026-09-13T21:00:00+00:00", "node_id": "n0" })),
            crate::pipeline::model::NodeExample::dsl("Every 15 minutes", r#"trigger.schedule --cron "*/15 * * * *""#),
        ],
        ..Default::default()
    }
}

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
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: input.payload,
            trace: vec![
                format!("node_kind={NODE_KIND}"),
                format!("cron={}", self.config.cron),
                format!("timezone={}", self.config.timezone),
            ],
        })
    }
}

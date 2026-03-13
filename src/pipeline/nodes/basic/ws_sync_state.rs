//! `n.ws.sync_state` — patch a WebSocket room's shared state tree.
//!
//! This node mutates the shared state of a room and — depending on the
//! `--silent` flag — either broadcasts the change immediately or defers it to
//! the room's 33 ms tick loop.
//!
//! # Config flags
//!
//! | Flag | Type | Default | Description |
//! |---|---|---|---|
//! | `--op` | `"set"` \| `"merge"` \| `"delete"` | `"set"` | State mutation type |
//! | `--path` | string | `""` (root) | JSON-pointer path; supports `{key}` placeholders |
//! | `--value-path` | string | `""` | JSON pointer into the **payload** to extract the value; empty = whole payload |
//! | `--room` | string | `""` | Static room id override; if empty, `room_id` is read from the payload |
//! | `--silent` | bool | `false` | Batch the update via the tick loop instead of broadcasting immediately |
//!
//! # Dynamic paths
//!
//! `--path` supports `{key}` placeholders resolved from the flowing payload:
//!
//! ```text
//! --path /players/{session_id}         → /players/abc123
//! --path /places/house/{user_id}       → /places/house/u42
//! --path /rooms/{room_type}/npcs/{id}  → /rooms/arena/npcs/boss1
//! ```
//!
//! See [`crate::infra::transport::ws::path::interpolate_path`] for full semantics.
//!
//! # Choosing `--silent`
//!
//! | Use case | Recommended |
//! |---|---|
//! | Chat message, score update, door state | `PatchState` (immediate, default) |
//! | 3D position at 30 fps, sensor stream | `PatchStateSilent` (`--silent`) |
//!
//! With `--silent`, 600 mutations/s from 20 players produce only 30 broadcasts/s
//! (one full-state snapshot per tick).
//!
//! # Room resolution
//!
//! 1. If `--room` is set, use `{owner}/{project}/{room}` as the room key.
//! 2. Otherwise read `room_id` from `input.payload.room_id` (injected by `n.trigger.ws`).
//!
//! Returns an error only if no room id can be resolved; a missing room (no
//! clients have ever joined) is silently skipped.
//!
//! # Payload passthrough
//!
//! The node forwards the unchanged incoming payload to its `out` pin so
//! downstream nodes can continue processing.
//!
//! # Example pipelines
//!
//! **Multiplayer position (batched at 30 fps):**
//! ```text
//! | n.trigger.ws --event move
//! | n.ws.sync_state --op merge --path /players/{session_id} --silent
//! ```
//!
//! **Chat message (immediate):**
//! ```text
//! | n.trigger.ws --event chat
//! | n.ws.sync_state --op set --path /last_message
//! ```
//!
//! **AI agent updating global state from a scheduled job:**
//! ```text
//! | n.trigger.schedule --cron "*/5 * * * *"
//! | n.script -- "return { weather: 'rainy', temp: 18 }"
//! | n.ws.sync_state --op merge --path /world --room lobby
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::{
    PipelineError, NodeDefinition,
    nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput},
};
use crate::pipeline::model::{DslFlag, DslFlagKind};
use crate::infra::transport::ws::{RoomCmd, StateOp, WsHub, interpolate_path};

pub const NODE_KIND: &str = "n.ws.sync_state";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

/// Return the [`NodeDefinition`] for `n.ws.sync_state`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "WS Sync State".to_string(),
        description: "Patches the shared state of a WebSocket room and optionally broadcasts \
            the update. Supports dynamic paths like /players/{session_id}. \
            Use --silent for high-frequency updates (30 fps); the room tick loop \
            will batch them. Use --room to target a room from server-side pipelines \
            (scheduled jobs, webhooks) where no WS client is the trigger."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({ "type": "object" }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({
            "type": "object",
            "properties": {
                "op": {
                    "type": "string",
                    "enum": ["set", "merge", "delete"],
                    "description": "State mutation type. set = replace value at path. merge = shallow-merge object. delete = remove key. Default: set."
                },
                "path": {
                    "type": "string",
                    "description": "JSON-pointer destination path. Supports {key} placeholders resolved from payload. Examples: /counter, /players/{session_id}. Empty = root."
                },
                "value_path": {
                    "type": "string",
                    "description": "JSON pointer into the incoming payload to extract the value to write. Empty = use the whole payload (or payload.payload if present)."
                },
                "room": {
                    "type": "string",
                    "description": "Static room id override. Required for server-initiated pipelines (scheduled jobs, webhooks) where no WS client is the trigger."
                },
                "silent": {
                    "type": "boolean",
                    "description": "If true, accumulate via the 33ms tick loop instead of broadcasting immediately. Use for high-frequency streams (>=10 Hz). Default: false."
                }
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--op".to_string(),
                config_key: "op".to_string(),
                description: "State mutation: set (replace), merge (shallow-merge), delete. Default: set.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--path".to_string(),
                config_key: "path".to_string(),
                description: "JSON-pointer destination. Supports {key} placeholders from payload. Example: /players/{session_id}.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--value-path".to_string(),
                config_key: "value_path".to_string(),
                description: "JSON pointer into the payload to extract the value. Empty = whole payload.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--room".to_string(),
                config_key: "room".to_string(),
                description: "Static room id for server-initiated pipelines without a WS trigger.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--silent".to_string(),
                config_key: "silent".to_string(),
                description: "Batch via 33ms tick loop instead of immediate broadcast. Recommended for >=10 Hz update streams.".to_string(),
                kind: DslFlagKind::Bool,
                required: false,
            },
        ],
        ai_tool: Default::default(),
    }
}

/// Configuration for `n.ws.sync_state`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// State mutation type: `"set"` (default), `"merge"`, or `"delete"`.
    ///
    /// - `set`    — replace the value at `path`.
    /// - `merge`  — shallow-merge an object into the target; works at any path depth.
    /// - `delete` — remove the key at `path` from its parent.
    #[serde(default)]
    pub op: String,

    /// JSON-pointer destination path, with optional `{key}` placeholders.
    ///
    /// Examples: `""` (root), `"/counter"`, `"/players/{session_id}"`,
    /// `"/places/house/{user_id}"`.
    ///
    /// Placeholders are resolved from top-level string fields of the incoming
    /// payload via [`interpolate_path`].
    #[serde(default)]
    pub path: String,

    /// JSON pointer into the **payload** to extract the value to write.
    ///
    /// Empty (default) — use the entire incoming payload as the value.
    /// Example: `"/position"` — write only `payload.position` into `path`.
    #[serde(default)]
    pub value_path: String,

    /// Static room id override.
    ///
    /// When set, the node targets `{owner}/{project}/{room}` regardless of
    /// `payload.room_id`.  Required for server-initiated pipelines (scheduled
    /// jobs, webhooks) that have no WS trigger context.
    ///
    /// Example: `"lobby"`, `"places/hall"`.
    #[serde(default)]
    pub room: String,

    /// If `true`, accumulate the mutation silently and let the 33 ms tick loop
    /// flush a single `state_patch` broadcast.
    ///
    /// Use for high-frequency streams (≥ 10 Hz positional updates).
    /// Default `false` — broadcast immediately.
    #[serde(default)]
    pub silent: bool,
}

/// `n.ws.sync_state` node instance.
pub struct Node {
    config: Config,
    ws_hub: Arc<WsHub>,
}

impl Node {
    pub fn new(config: Config, ws_hub: Arc<WsHub>) -> Result<Self, PipelineError> {
        Ok(Self { config, ws_hub })
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

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let owner = input
            .metadata
            .get("owner")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let project = input
            .metadata
            .get("project")
            .and_then(Value::as_str)
            .unwrap_or_default();

        // Room resolution: static --room flag takes precedence over payload.room_id.
        let room_id = if !self.config.room.is_empty() {
            self.config.room.clone()
        } else {
            input
                .payload
                .get("room_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };

        if room_id.is_empty() {
            return Err(PipelineError::new(
                "FW_WS_SYNC_STATE_NO_ROOM",
                "n.ws.sync_state: room_id missing — set --room or ensure n.trigger.ws is upstream",
            ));
        }

        let room_key = format!("{}/{}/{}", owner, project, room_id);

        // Resolve dynamic path placeholders from the payload.
        let resolved_path = interpolate_path(&self.config.path, &input.payload);

        let op = match self.config.op.as_str() {
            "merge" => StateOp::Merge,
            "delete" => StateOp::Delete,
            _ => StateOp::Set,
        };

        if let Some(room) = self.ws_hub.get_room(&room_key) {
            let value = if matches!(op, StateOp::Delete) {
                None
            } else {
                let base = if self.config.value_path.is_empty() {
                    // No value_path — use the whole payload (or its inner .payload if present).
                    input
                        .payload
                        .get("payload")
                        .cloned()
                        .unwrap_or_else(|| input.payload.clone())
                } else {
                    let ptr = if self.config.value_path.starts_with('/') {
                        self.config.value_path.clone()
                    } else {
                        format!("/{}", self.config.value_path)
                    };
                    input.payload.pointer(&ptr).cloned().unwrap_or(Value::Null)
                };
                Some(base)
            };

            let cmd = if self.config.silent {
                RoomCmd::PatchStateSilent {
                    op,
                    path: resolved_path.clone(),
                    value,
                }
            } else {
                RoomCmd::PatchState {
                    op,
                    path: resolved_path.clone(),
                    value,
                }
            };

            room.send_cmd(cmd);
        }

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: input.payload,
            trace: vec![format!(
                "n.ws.sync_state: op={} path={} silent={}",
                self.config.op, resolved_path, self.config.silent
            )],
        })
    }
}

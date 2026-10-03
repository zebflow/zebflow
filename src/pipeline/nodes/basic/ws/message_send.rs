//! `ws.message.send` — send `--body` to a room's sockets or on an outgoing
//! client socket.
//!
//! ```text
//! ws.message.send (--room ROOM | --connection CONN) --body VALUE [--event NAME]
//!                 [--recipient all|session|others]
//! ```
//!
//! - **`--room`** reaches the sockets joined to a room of this project as
//!   `{ "type": "event", "event": <--event>, "payload": <--body> }`. Without
//!   `--room` or `--connection` the room is the one of the `trigger.room` that
//!   started the run. `--recipient` (`all` default, `session`, `others`) is
//!   enforced by the server; `session` and `others` need the triggering
//!   session, so they work only under `trigger.room`. A room nobody has joined
//!   is not an error: the answer says `sent: false`.
//! - **`--connection`** sends on the socket a `trigger.socket` node holds open:
//!   its node id (`n0`), or `pipeline/path:node_id` for another pipeline's. A
//!   text body goes as is, anything else as JSON text. A client socket has no
//!   event name or recipients, so `--event` and `--recipient` are refused.
//!
//! Both `--room` and `--connection` is refused. The answer is
//! `message: { sent, room, event }` for a room and
//! `message: { sent, connection }` for a connection; the payload is kept.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{INPUT_PIN_IN, OUTPUT_PIN_OUT, answer, field, flag, room_key, room_of, text_of};
use crate::infra::transport::ws::{EmitTarget, RoomCmd, WsHub};
use crate::infra::ws_client::WsClientManager;
use crate::pipeline::model::{DslFlag, LayoutItem, NodeCapability, NodeExample, NodeFieldType, SelectOptionDef};
use crate::pipeline::nodes::shared::limits::choice;
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "ws.message.send";
/// A flag set wrong: both targets, an unknown recipient, or a room-only flag with `--connection`.
pub const CONFIG_CODE: &str = "FW_NODE_WS_MESSAGE_SEND_CONFIG";
/// `--body` missing or resolved to nothing.
pub const EMPTY_CODE: &str = "FW_NODE_WS_MESSAGE_SEND_EMPTY";
/// No target: no `--room`, no `--connection` and no `trigger.room` upstream.
pub const ROOM_CODE: &str = "FW_NODE_WS_MESSAGE_SEND_ROOM";
/// `--recipient session|others` without a triggering session.
pub const SESSION_CODE: &str = "FW_NODE_WS_MESSAGE_SEND_SESSION";
/// The client socket is not open (or the name matches more than one).
pub const CONNECTION_CODE: &str = "FW_NODE_WS_MESSAGE_SEND_CONNECTION";
/// This engine has no room hub or no client socket manager.
pub const UNAVAILABLE_CODE: &str = "FW_NODE_WS_MESSAGE_SEND_UNAVAILABLE";

const RECIPIENTS: &[&str] = &["all", "session", "others"];
const DEFAULT_EVENT: &str = "event";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        // `--connection` reaches a host outside the platform.
        capabilities: vec![NodeCapability::Network],
        title: "WS Message Send".to_string(),
        description: "Sends `--body` over a WebSocket. `--room` reaches the sockets joined to a room of this project as an \
            `event` message named `--event` (default `event`); without `--room` the room is the one of the `trigger.room` \
            that started the run. `--recipient` all (default), session (the triggering socket only) or others (every socket \
            but that one). `--connection` instead sends on the socket a `trigger.socket` node holds open (its node id, or \
            `pipeline/path:node_id`); it takes no `--event` or `--recipient`. Give one of `--room` and `--connection`, not \
            both. Adds `message: { sent, room, event }` (or `{ sent, connection }`) and keeps the rest of the payload; \
            `sent` is false when nobody has joined the room. Does not change room state — that is `ws.state.*`."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `message` is added." }),
        output_schema: json!({ "type": "object", "properties": { "message": { "type": "object", "properties": {
            "sent": { "type": "boolean", "description": "Whether the message was handed to a room or a socket" },
            "room": { "type": "string", "description": "The room, for a room send" },
            "event": { "type": "string", "description": "The event name, for a room send" },
            "connection": { "type": "string", "description": "The connection, for a client socket send" }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![
            flag("--room", "room", "The room to send to; default: the room of the trigger.room that started the run.", "text", false),
            flag("--connection", "connection", "The trigger.socket node whose open socket to send on: its id, or pipeline/path:node_id.", "text", false),
            flag("--body", "body", "What to send — a literal or {{ expr }}.", "json", true),
            flag("--event", "event", "The event name clients receive (rooms only). Default: event.", "text", false),
            DslFlag {
                choices: RECIPIENTS.iter().map(|w| w.to_string()).collect(),
                ..flag(
                    "--recipient",
                    "recipient",
                    "Rooms only: all (default), session (the triggering socket) or others (every socket but that one).",
                    "",
                    false,
                )
            },
        ],
        fields: vec![
            field("room", "Room", "The room to send to; empty: the room of the trigger.room that started the run."),
            field("connection", "Connection", "Instead of a room: the trigger.socket node whose socket to send on."),
            field("body", "Body", "What to send — a literal or {{ expr }}."),
            field("event", "Event", "The event name clients receive (rooms only). Default: event."),
            crate::pipeline::model::NodeFieldDef {
                field_type: NodeFieldType::Select,
                options: RECIPIENTS
                    .iter()
                    .map(|w| SelectOptionDef { value: w.to_string(), label: w.to_string() })
                    .collect(),
                // No default: the form must not write a recipient a
                // `--connection` send would refuse; empty means all.
                ..field("recipient", "Recipient", "Rooms only: all (empty), session (the triggering socket) or others.")
            },
        ],
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("room".to_string()), LayoutItem::Field("connection".to_string())] },
            LayoutItem::Field("body".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("event".to_string()), LayoutItem::Field("recipient".to_string())] },
        ],
        examples: vec![
            NodeExample::dsl(
                "Broadcast a chat line to the room",
                r#"ws.message.send --event chat.message --body "{{ { from: input.session_id, text: input.payload.text } }}""#,
            )
            .output(json!({ "message": { "sent": true, "room": "lobby", "event": "chat.message" } }))
            .note("After `trigger.room`, the room is the one the event came from."),
            NodeExample::dsl(
                "Push from a schedule to a fixed room",
                r#"ws.message.send --room dashboard --event stats.tick --body "{{ { online: input.online } }}""#,
            ),
            NodeExample::dsl(
                "Subscribe after connecting",
                r#"ws.message.send --connection n0 --body "{{ { op: 'subscribe', symbols: ['AUDUSD'] } }}""#,
            )
            .output(json!({ "message": { "sent": true, "connection": "n0" } }))
            .note("`n0` is the `trigger.socket` node that holds the socket open."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// The room; empty: the room of the `trigger.room` that started the run.
    #[serde(default)]
    pub room: Value,
    /// The `trigger.socket` node whose socket to send on.
    #[serde(default)]
    pub connection: Value,
    /// What to send.
    #[serde(default)]
    pub body: Value,
    /// The event name (rooms only).
    #[serde(default)]
    pub event: Value,
    /// `all` (default), `session` or `others` (rooms only).
    #[serde(default)]
    pub recipient: String,
}

enum Target {
    Room {
        /// `--room`; empty means the trigger's room.
        room: String,
        event: String,
        recipient: &'static str,
        hub: Arc<WsHub>,
    },
    Connection {
        connection: String,
        manager: Arc<WsClientManager>,
    },
}

pub struct Node {
    target: Target,
    body: Value,
}

impl Node {
    pub fn new(
        config: Config,
        hub: Option<Arc<WsHub>>,
        manager: Option<Arc<WsClientManager>>,
    ) -> Result<Self, PipelineError> {
        let room = text_of(&config.room, "--room", CONFIG_CODE)?;
        let connection = text_of(&config.connection, "--connection", CONFIG_CODE)?;
        let event = text_of(&config.event, "--event", CONFIG_CODE)?;
        if config.body.is_null() || config.body.as_str().is_some_and(str::is_empty) {
            return Err(PipelineError::new(EMPTY_CODE, "--body is empty; it needs a value"));
        }
        if !room.is_empty() && !connection.is_empty() {
            return Err(PipelineError::new(CONFIG_CODE, "give --room or --connection, not both"));
        }
        let target = if connection.is_empty() {
            let recipient = choice(&config.recipient, RECIPIENTS, "all", "--recipient", CONFIG_CODE)?;
            let hub = hub.ok_or_else(|| PipelineError::new(UNAVAILABLE_CODE, "ws hub is not configured on this framework engine"))?;
            let event = if event.is_empty() { DEFAULT_EVENT.to_string() } else { event };
            Target::Room { room, event, recipient, hub }
        } else {
            if !event.is_empty() {
                return Err(PipelineError::new(CONFIG_CODE, "--event names a room event; a client socket sends --body as is"));
            }
            if !config.recipient.trim().is_empty() {
                return Err(PipelineError::new(CONFIG_CODE, "--recipient chooses sockets in a room; a client socket has one peer"));
            }
            let manager = manager.ok_or_else(|| {
                PipelineError::new(UNAVAILABLE_CODE, "ws client manager is not configured on this framework engine")
            })?;
            Target::Connection { connection, manager }
        };
        Ok(Self { target, body: config.body })
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
        match &self.target {
            Target::Room { room, event, recipient, hub } => {
                let room = room_of(room, &input.payload);
                if room.is_empty() {
                    return Err(PipelineError::new(
                        ROOM_CODE,
                        "no room: give --room or --connection, or start the run from trigger.room",
                    ));
                }
                let session = input.payload.get("session_id").and_then(Value::as_str).unwrap_or_default();
                let to = match *recipient {
                    "all" => EmitTarget::All,
                    other => {
                        if session.is_empty() {
                            return Err(PipelineError::new(
                                SESSION_CODE,
                                format!("--recipient {other} needs the triggering session; start the run from trigger.room"),
                            ));
                        }
                        if other == "session" { EmitTarget::Session(session.to_string()) } else { EmitTarget::Others(session.to_string()) }
                    }
                };
                // A room nobody has joined has no sockets to reach.
                let sent = match hub.get_room(&room_key(&input.metadata, &room)) {
                    Some(handle) => {
                        handle.send_cmd(RoomCmd::Emit { event: event.clone(), payload: self.body.clone(), to });
                        true
                    }
                    None => false,
                };
                Ok(answer(
                    &input.payload,
                    json!({ "message": { "sent": sent, "room": room, "event": event } }),
                    format!("node_kind={NODE_KIND} room={room} event={event} recipient={recipient} sent={sent}"),
                ))
            }
            Target::Connection { connection, manager } => {
                let owner = input.metadata.get("owner").and_then(Value::as_str).unwrap_or_default();
                let project = input.metadata.get("project").and_then(Value::as_str).unwrap_or_default();
                let text = match &self.body {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                manager
                    .send_to_connection(owner, project, connection, text)
                    .await
                    .map_err(|err| PipelineError::new(CONNECTION_CODE, err))?;
                Ok(answer(
                    &input.payload,
                    json!({ "message": { "sent": true, "connection": connection } }),
                    format!("node_kind={NODE_KIND} connection={connection} sent=true"),
                ))
            }
        }
    }
}

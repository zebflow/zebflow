//! `ws.ticket.create` — a pass for one user into one room
//! (`docs/contracts/rooms.md` § Opening a room and letting people in).
//!
//! ```text
//! ws.ticket.create --room NAME --user SUB [--ttl DURATION]
//! ```
//!
//! Signed by this process, usable once, valid for `--ttl` (default 60s, at
//! most 10m). The client joins with `?ticket=`. Refused for a room that is
//! not open (`FW_NODE_WS_ROOM_NOT_OPEN`) and for a full one. Answers
//! `ticket: { room, ticket, expires_at }`.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{INPUT_PIN_IN, OUTPUT_PIN_OUT, answer, duration_flag, exact_room, field, flag, project_of, refused, rfc3339, text_of};
use crate::infra::transport::ws::ticket::{DEFAULT_TTL, MAX_TTL, MIN_TTL};
use crate::infra::transport::ws::{HubRefusal, WsHub};
use crate::pipeline::model::{LayoutItem, NodeExample};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "ws.ticket.create";
/// A flag that does not read, or a pattern for `--room`.
pub const CONFIG_CODE: &str = "FW_NODE_WS_TICKET_CREATE_CONFIG";
/// `--room` or `--user` missing or resolved to nothing.
pub const EMPTY_CODE: &str = "FW_NODE_WS_TICKET_CREATE_EMPTY";
/// The room is at its `--max-sessions`.
pub const FULL_CODE: &str = "FW_NODE_WS_TICKET_CREATE_FULL";
/// This engine has no room hub.
pub const UNAVAILABLE_CODE: &str = "FW_NODE_WS_TICKET_CREATE_UNAVAILABLE";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "WS Ticket Create".to_string(),
        description: "A pass for `--user` into the open room `--room`: signed by the platform, usable once, valid for `--ttl` \
            (default 60s, at most 10m). The client connects to `/ws/{owner}/{project}/rooms/{room}?ticket=…`. Issue it from a \
            webhook that checks the app's own data first — the room queries nothing. A `--private` room admits only ticket \
            holders. The ticket's user must be the identity the room's join rule proves; under `--auth none` it becomes the \
            session's identity. Refused for a room that is not open (`FW_NODE_WS_ROOM_NOT_OPEN`) and for a full one. Adds \
            `ticket: { room, ticket, expires_at }` and keeps the rest of the payload."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `ticket` is added." }),
        output_schema: json!({ "type": "object", "properties": { "ticket": { "type": "object", "properties": {
            "room": { "type": "string" },
            "ticket": { "type": "string", "description": "Presented as ?ticket= when the client connects" },
            "expires_at": { "type": "string" }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![
            flag("--room", "room", "The open room the ticket admits to (an exact name).", "text", true),
            flag("--user", "user", "Who the ticket admits: the identity (sub) the session will have.", "text", true),
            flag("--ttl", "ttl", "How long the ticket can be used (default 60s, at most 10m).", "duration", false),
        ],
        fields: vec![
            field("room", "Room", "The open room the ticket admits to."),
            field("user", "User", "Who it admits: {{ $trigger.auth.sub }}."),
            field("ttl", "TTL", "Default 60s, at most 10m."),
        ],
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("room".to_string()), LayoutItem::Field("user".to_string())] },
            LayoutItem::Field("ttl".to_string()),
        ],
        examples: vec![
            NodeExample::dsl("Let a player into a match", r#"ws.ticket.create --room "match-{{ $trigger.params.id }}" --user "{{ $trigger.auth.sub }}""#)
                .output(json!({ "ticket": { "room": "match-7", "ticket": "eyJr…", "expires_at": "2026-10-16T08:01:00Z" } }))
                .note("After `trigger.webhook --route /api/matches/:id/ticket --auth jwt` and a check that the caller belongs in match 7."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub room: Value,
    #[serde(default)]
    pub user: Value,
    #[serde(default)]
    pub ttl: Value,
}

pub struct Node {
    room: String,
    user: String,
    ttl: std::time::Duration,
    hub: Arc<WsHub>,
}

impl Node {
    pub fn new(config: Config, hub: Arc<WsHub>) -> Result<Self, PipelineError> {
        let room = text_of(&config.room, "--room", CONFIG_CODE)?;
        let user = text_of(&config.user, "--user", CONFIG_CODE)?;
        if room.is_empty() || user.is_empty() {
            let flag = if room.is_empty() { "--room" } else { "--user" };
            return Err(PipelineError::new(EMPTY_CODE, format!("{flag} is empty; a ticket admits one user into one room")));
        }
        let room = exact_room(&room, "--room", CONFIG_CODE)?;
        let ttl = duration_flag(&config.ttl, "--ttl", CONFIG_CODE, MIN_TTL, MAX_TTL)?.unwrap_or(DEFAULT_TTL);
        Ok(Self { room, user, ttl, hub })
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
        let (owner, project) = project_of(&input.metadata);
        let ticket = self.hub.ticket(owner, project, &self.room, &self.user, self.ttl).map_err(|refusal| match refusal {
            HubRefusal::Full(_) => PipelineError::new(FULL_CODE, refusal.to_string()),
            other => refused(other, CONFIG_CODE),
        })?;
        Ok(answer(
            &input.payload,
            json!({ "ticket": { "room": self.room, "ticket": ticket.token, "expires_at": rfc3339(ticket.expires_at) } }),
            format!("node_kind={NODE_KIND} room={}", self.room),
        ))
    }
}

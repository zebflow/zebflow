//! Built-in trigger nodes.
//!
//! Triggers are entry nodes: they have no input pins and they create the first
//! payload for a pipeline run. For the general node authoring contract, read
//! `src/pipeline/nodes/mod.rs`; for registration, read
//! `src/pipeline/nodes/basic/mod.rs`.
//!
//! Ingress payloads must be stable enough for downstream nodes and frontend
//! generated clients to depend on. `trigger.webhook` uses this shape:
//!
//! - `input.body`: user-submitted JSON, form fields, multipart text fields, or
//!   `null` for empty requests.
//! - `input.query`, `input.params`, `input.path`, `input.method`: request context.
//! - `input.files.<field>`: uploaded files as FileRef metadata. Repeated file
//!   field names, `field[]`, and `field[0]` are represented as arrays under
//!   `input.files.<field>`.
//!
//! Keep trigger-specific details in each trigger module, and keep shared file byte
//! rules in `src/pipeline/nodes/shared/file_ref.rs`.
//!
//! `trigger.ws` lives with the WebSocket family (`basic/ws/trigger.rs`).

use crate::pipeline::NodeDefinition;

pub mod function;
pub mod kv_subscribe;
pub mod manual;
pub mod mcp_trigger;
pub mod schedule;
pub mod weberror;
pub mod webhook;
pub mod ws_client;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![
        function::definition(),
        kv_subscribe::definition(),
        webhook::definition(),
        schedule::definition(),
        manual::definition(),
        mcp_trigger::definition(),
        ws_client::definition(),
        weberror::definition(),
    ]
}

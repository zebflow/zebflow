//! `ws.*` — WebSocket rooms: `ws.emit`, `ws.sync_state`, `ws.client_send`,
//! and the trigger that starts a pipeline from a room event (`trigger.ws`,
//! whose code lives with the family it serves).

use crate::pipeline::NodeDefinition;

pub mod client_send;
pub mod emit;
pub mod sync_state;
pub mod trigger;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![
        trigger::definition(),
        client_send::definition(),
        sync_state::definition(),
        emit::definition(),
    ]
}

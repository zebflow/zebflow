//! WebSocket engine — real-time room management, state sync, and event broadcast.
//!
//! # Module structure
//!
//! | Module | Responsibility |
//! |---|---|
//! | [`room`] | Room actor: shared state, broadcast channel, tick-based flush |
//! | [`path`] | Dynamic path interpolation (`{session_id}`, `{user_id}`, …) |
//! | [`WsHub`] | Central registry of all active rooms (held in `PlatformService`) |
//!
//! # Room key format
//!
//! Rooms are keyed by `"{owner}/{project}/{room_id}"`, e.g.
//! `"alice/myapp/lobby"` or `"alice/myapp/places/hall"`.
//! The `room_id` portion is taken from the WS URL and may contain slashes.
//!
//! # Quick start for pipeline authors
//!
//! ```text
//! trigger.room   --event move
//! n.ws.sync_state --op merge --state-key /players/{session_id} --silent
//! ```
//!
//! This accumulates positional updates and broadcasts them at ≈30 fps via
//! the room tick loop — see [`room::RoomCmd::PatchStateSilent`] for details.

pub mod path;
pub mod room;

pub use path::interpolate_path;
pub use room::{EmitTarget, RoomBroadcast, RoomCmd, RoomHandle, SessionGuard, StateOp};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Central WebSocket hub — owns the registry of all active rooms.
///
/// Rooms are created lazily by the first [`WsHub::join_room`] and disposed by
/// the [`WsHub::leave_room`] that removes their last session. Both hold the
/// registry lock, so a join can never pick up a room that is being disposed.  This struct is cheap to clone (all state is behind
/// an `Arc`).
///
/// Held as `pub ws_hub: Arc<WsHub>` in [`crate::platform::services::PlatformService`].
#[derive(Clone)]
pub struct WsHub {
    rooms: Arc<Mutex<HashMap<String, Arc<RoomHandle>>>>,
}

impl WsHub {
    /// Create an empty hub.
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Join `room_key` as a new session: return the room (spawned if absent)
    /// and the session's guard. Hand the guard back to [`WsHub::leave_room`].
    pub fn join_room(&self, room_key: &str) -> (Arc<RoomHandle>, SessionGuard) {
        let mut rooms = self.rooms.lock().unwrap_or_else(|e| e.into_inner());
        let room = rooms
            .entry(room_key.to_string())
            .or_insert_with(RoomHandle::spawn)
            .clone();
        let guard = room.join_session();
        (room, guard)
    }

    /// Return an existing room without creating one.
    ///
    /// Called by `n.ws.sync_state` and `n.ws.emit` when they need a room
    /// that must have been created by a prior WS connection.  Returns `None`
    /// if no client has ever joined the room.
    pub fn get_room(&self, room_key: &str) -> Option<Arc<RoomHandle>> {
        self.rooms
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(room_key)
            .cloned()
    }

    /// List all room keys currently tracked by the hub.
    pub fn list_rooms(&self) -> Vec<String> {
        self.rooms
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect()
    }

    /// Leave a room: drop the session's guard and, when it was the last
    /// session, stop the room actor and forget its state.
    ///
    /// The guard is dropped *before* the count is read — reading it while the
    /// leaving session still held its guard is why rooms used to live forever.
    pub fn leave_room(&self, room_key: &str, guard: SessionGuard) {
        let mut rooms = self.rooms.lock().unwrap_or_else(|e| e.into_inner());
        drop(guard);
        if rooms.get(room_key).map(|room| room.session_count() == 0).unwrap_or(false) {
            if let Some(room) = rooms.remove(room_key) {
                room.send_cmd(RoomCmd::Shutdown);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_last_session_leaving_disposes_the_room() {
        let hub = WsHub::new();
        let (_room, a) = hub.join_room("o/p/lobby");
        let (_room, b) = hub.join_room("o/p/lobby");
        hub.leave_room("o/p/lobby", a);
        assert_eq!(hub.list_rooms(), vec!["o/p/lobby".to_string()], "one session is still there");
        hub.leave_room("o/p/lobby", b);
        assert!(hub.list_rooms().is_empty(), "the last leave disposes the room");
    }

    #[tokio::test]
    async fn a_room_rejoined_after_disposal_starts_empty() {
        let hub = WsHub::new();
        let (room, a) = hub.join_room("o/p/lobby");
        room.send_cmd(RoomCmd::PatchState { op: StateOp::Set, path: "/players/x".into(), value: Some(serde_json::json!({ "n": 1 })) });
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        assert_eq!(room.get_state()["players"]["x"]["n"], 1);
        hub.leave_room("o/p/lobby", a);
        let (fresh, _b) = hub.join_room("o/p/lobby");
        assert_eq!(fresh.get_state(), serde_json::json!({}), "no ghost state survives disposal");
    }
}

impl Default for WsHub {
    fn default() -> Self {
        Self::new()
    }
}

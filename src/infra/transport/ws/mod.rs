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
//! n.trigger.ws   --event move
//! n.ws.sync_state --op merge --path /players/{session_id} --silent
//! ```
//!
//! This accumulates positional updates and broadcasts them at ≈30 fps via
//! the room tick loop — see [`room::RoomCmd::PatchStateSilent`] for details.

pub mod path;
pub mod room;

pub use path::interpolate_path;
pub use room::{EmitTarget, RoomCmd, RoomHandle, SessionGuard, StateOp};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Central WebSocket hub — owns the registry of all active rooms.
///
/// Rooms are created lazily on first connection and cleaned up when the last
/// session disconnects.  This struct is cheap to clone (all state is behind
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

    /// Return an existing room or spawn a new one.
    ///
    /// Called by the WS route handler on every new connection.
    pub fn get_or_create_room(&self, room_key: &str) -> Arc<RoomHandle> {
        let mut rooms = self.rooms.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(room) = rooms.get(room_key) {
            return room.clone();
        }
        let room = RoomHandle::spawn();
        rooms.insert(room_key.to_string(), room.clone());
        room
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

    /// Remove a room from the registry.
    ///
    /// Called by the WS route handler when a session disconnects.  The room
    /// is only removed if its session count has reached zero (i.e. all
    /// clients have left).
    pub fn remove_room(&self, room_key: &str) {
        let mut rooms = self.rooms.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(room) = rooms.get(room_key) {
            if room.session_count() == 0 {
                room.send_cmd(RoomCmd::Shutdown);
                rooms.remove(room_key);
            }
        }
    }
}

impl Default for WsHub {
    fn default() -> Self {
        Self::new()
    }
}

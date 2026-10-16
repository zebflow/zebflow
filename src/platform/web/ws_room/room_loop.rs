//! One room's loop: it takes the room's queue in arrival order and handles
//! one thing at a time (`rooms.md` § Events and ticks).
//!
//! The first session to join starts it; it ends when the last session has
//! left and every event left is handled. While it runs it holds the room's
//! turn, so a loop started meanwhile waits and two never run at once.

use std::sync::Arc;

use super::events::{Resolved, RoomCtx, run_event};
use crate::infra::transport::ws::{RoomHandle, RoomQueue};

/// Run the room until its last session is gone and its queue is empty.
pub(super) async fn run_room(ctx: RoomCtx, room: Arc<RoomHandle>, mut queue: RoomQueue) {
    let _turn = room.turn().await;
    let mut resolved = Resolved::of(&ctx, &room);
    loop {
        if !resolved.is_current(&room) {
            resolved = Resolved::of(&ctx, &room);
        }
        tokio::select! {
            next = queue.recv() => match next {
                Some(event) => run_event(&ctx, &room, &resolved, event).await,
                None => break,
            },
            // A pipeline was activated or deactivated: resolve again.
            _ = room.changed() => {}
        }
    }
}

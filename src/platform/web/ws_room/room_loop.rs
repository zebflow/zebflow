//! One room's loop: it takes the room's queue in arrival order and handles
//! one thing at a time (`rooms.md` § Events and ticks).
//!
//! The first session to join starts it; it ends when the last session has
//! left and every event left is handled, a tick room's waiting inputs
//! included. While it runs it holds the room's turn, so a loop started
//! meanwhile waits and two never run at once. In a
//! tick room ([`tick`](super::tick)) client events wait for the tick, while
//! `$connect` and `$disconnect` still run their own triggers — a
//! `$disconnect` after the tick that takes its session's last inputs.

use std::sync::Arc;

use super::events::{Resolved, RoomCtx, run_event};
use super::tick::{Ticker, until};
use crate::infra::transport::ws::{RoomHandle, RoomQueue};

/// Run the room until its last session is gone and its queue is empty.
pub(super) async fn run_room(ctx: RoomCtx, room: Arc<RoomHandle>, mut queue: RoomQueue) {
    let _turn = room.turn().await;
    let mut resolved = Resolved::of(&ctx, &room);
    let mut ticker = Ticker::new(room.tick());
    loop {
        if !resolved.is_current(&room) {
            resolved = Resolved::of(&ctx, &room);
            for event in ticker.retime(room.tick()) {
                run_event(&ctx, &room, &resolved, event).await;
            }
        }
        let due = ticker.due_at();
        tokio::select! {
            next = queue.recv() => match next {
                Some(event) if ticker.is_on() && !event.event.starts_with('$') => ticker.take(event, &room),
                Some(event) => {
                    if let Err(event) = ticker.hold(event) {
                        run_event(&ctx, &room, &resolved, event).await;
                    }
                }
                // The last session left. Inputs still waiting get their tick,
                // and the `$disconnect` held for it runs after.
                None => {
                    if ticker.has_waiting() {
                        until(ticker.due_at()).await;
                        for event in ticker.run(&ctx, &room, &resolved).await {
                            run_event(&ctx, &room, &resolved, event).await;
                        }
                    }
                    break;
                }
            },
            _ = until(due) => {
                for event in ticker.run(&ctx, &room, &resolved).await {
                    run_event(&ctx, &room, &resolved, event).await;
                }
            }
            // A pipeline was activated or deactivated: resolve again.
            _ = room.changed() => {}
        }
    }
}

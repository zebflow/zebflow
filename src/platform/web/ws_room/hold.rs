//! Keeping a dropped session's seat (`docs/contracts/rooms.md` § Leaving,
//! dropping and closing).
//!
//! In a room opened with `--hold`, a socket that ends without a close frame
//! keeps its seat: it still counts towards `--max-sessions`, keeps its id
//! and tags, and is sent nothing. `$away` is raised on the room's queue,
//! after every event the socket sent. Then, whichever comes first:
//!
//! - a socket resumes it with the resume token its last `joined` gave
//!   (usable once, only during the hold) and takes the seat back: `joined`
//!   with its whole view and a new token, then `$back`;
//! - the hold ends: `$disconnect { reason: "timeout" }`;
//! - `ws.session.close` or `ws.room.close` closes it: `$disconnect` with
//!   their usual reason.

use std::sync::Arc;
use std::time::Duration;

use super::*;
use crate::infra::transport::ws::{Kick, RoomHandle, Seat};

/// Keep the seat of the session whose socket dropped, for `hold`, for the
/// resume token whose nonce is `nonce`.
pub(super) fn keep(ctx: &SessionCtx, room: &Arc<RoomHandle>, seat: Seat, nonce: String, user: Option<String>, hold: Duration) {
    room.detach(&ctx.session_id);
    // Queued behind every event the socket sent, like a `$disconnect`.
    seat.queue.raise(AWAY_EVENT, json!({}));
    let hub = ctx.state.platform.ws_hub.clone();
    let (owner, project, room_id, session) = (ctx.owner.clone(), ctx.project.clone(), ctx.room_id.clone(), ctx.session_id.clone());
    let asked = hub.hold(&owner, &project, &room_id, &session, &nonce, user);
    lane().spawn(async move {
        let end = wait(seat, asked, hold, || hub.release(&owner, &project, &room_id, &session, &nonce)).await;
        if let Some((seat, reason)) = end {
            seat.queue.raise(DISCONNECT_EVENT, json!({ "reason": reason }));
        }
    });
}

/// How a hold ended.
enum Ended {
    Due,
    Kicked(Kick),
    Asked(Option<tokio::sync::oneshot::Sender<Seat>>),
}

/// Wait out the hold. Answers the seat and the `$disconnect` reason when it
/// ends unused; `None` when a resume took it. `release` ends the hold unless
/// a resume took it first.
async fn wait(
    mut seat: Seat,
    mut asked: tokio::sync::oneshot::Receiver<tokio::sync::oneshot::Sender<Seat>>,
    hold: Duration,
    release: impl Fn() -> bool,
) -> Option<(Seat, &'static str)> {
    let deadline = tokio::time::sleep(hold);
    tokio::pin!(deadline);
    let mut due = false;
    loop {
        let ended = tokio::select! {
            _ = &mut deadline, if !due => Ended::Due,
            Some(kick) = seat.kicks.recv() => Ended::Kicked(kick),
            reply = &mut asked => Ended::Asked(reply.ok()),
        };
        match ended {
            Ended::Due if release() => return Some((seat, "timeout")),
            // A resume took the seat as the hold ended: hand it over.
            Ended::Due => due = true,
            Ended::Kicked(kick) => {
                release();
                return Some((seat, kick.reason));
            }
            Ended::Asked(Some(reply)) => {
                // A resumed socket that went away meanwhile drops it: the
                // seat ends with it.
                if let Err(seat) = reply.send(seat) {
                    return Some((seat, "dropped"));
                }
                return None;
            }
            Ended::Asked(None) => return Some((seat, "dropped")),
        }
    }
}

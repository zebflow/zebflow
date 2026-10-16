//! A room's one queue (`docs/contracts/rooms.md` § Events and ticks).
//!
//! **A room handles one thing at a time, in arrival order across all its
//! sessions.** Every session of a room pushes into the same queue — its
//! client events and the `$connect` / `$disconnect` the server raises for
//! it — and one room loop takes them off in order. Two sessions' events
//! never run at once in one room, so a pipeline that reads state and writes
//! it back needs no lock.
//!
//! A session may have at most [`MAX_PENDING_PER_SESSION`] events waiting to
//! run; [`SessionQueue::push`] refuses the next one and the socket closes
//! with `1008`. An event counts as waiting until it has run (or, in a tick
//! room, until the tick that took it ends, or it was dropped), so the room's
//! memory is bounded by its sessions.
//!
//! The queue lives as long as one of its sessions does: the room holds only
//! a weak sender, so the loop ends when the last session's [`SessionQueue`]
//! is dropped and every event left is handled. The next join starts a new
//! loop, which first waits for the old one's turn to end
//! ([`super::RoomHandle::turn`]).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;
use tokio::sync::mpsc;

/// Most events one session may have waiting to run (`rooms.md`: more closes
/// the session with `1008`).
pub const MAX_PENDING_PER_SESSION: usize = 256;

/// One thing a room handles: a session's event, or a `$` event the server
/// raised for it.
pub struct RoomEvent {
    /// The session it came from.
    pub session: String,
    /// The event name (`move`, `$connect`, …).
    pub event: String,
    /// The event body.
    pub payload: Value,
    /// What the join rule proved for the session (`input.room.auth`).
    pub auth: Value,
    pending: Arc<AtomicUsize>,
}

impl Drop for RoomEvent {
    fn drop(&mut self) {
        self.pending.fetch_sub(1, Ordering::AcqRel);
    }
}

/// The room loop's end of the queue.
pub type RoomQueue = mpsc::UnboundedReceiver<RoomEvent>;

/// One session's way into its room's queue. Dropping it is the session
/// saying it will send nothing more.
pub struct SessionQueue {
    tx: mpsc::UnboundedSender<RoomEvent>,
    session: String,
    auth: Value,
    pending: Arc<AtomicUsize>,
}

impl SessionQueue {
    pub(super) fn new(tx: mpsc::UnboundedSender<RoomEvent>, session: &str, auth: Value) -> Self {
        Self { tx, session: session.to_string(), auth, pending: Arc::new(AtomicUsize::new(0)) }
    }

    /// Queue a client event; `false` when this session already has
    /// [`MAX_PENDING_PER_SESSION`] waiting, or the room loop is gone.
    pub fn push(&self, event: String, payload: Value) -> bool {
        if self.pending.load(Ordering::Acquire) >= MAX_PENDING_PER_SESSION {
            return false;
        }
        self.send(event, payload)
    }

    /// Queue an event the server raises (`$connect`, `$disconnect`); never
    /// refused for the cap, so a flooding session still gets its `$disconnect`.
    pub fn raise(&self, event: &str, payload: Value) {
        self.send(event.to_string(), payload);
    }

    /// Events of this session waiting to run.
    pub fn pending(&self) -> usize {
        self.pending.load(Ordering::Acquire)
    }

    fn send(&self, event: String, payload: Value) -> bool {
        self.pending.fetch_add(1, Ordering::AcqRel);
        let item = RoomEvent {
            session: self.session.clone(),
            event,
            payload,
            auth: self.auth.clone(),
            pending: self.pending.clone(),
        };
        // A refused send drops the event, which gives its slot back.
        self.tx.send(item).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn queue() -> (SessionQueue, RoomQueue) {
        let (tx, rx) = mpsc::unbounded_channel();
        (SessionQueue::new(tx, "s1", json!({ "sub": "u-17" })), rx)
    }

    #[test]
    fn a_session_may_have_256_events_waiting_and_no_more() {
        let (session, _rx) = queue();
        for i in 0..MAX_PENDING_PER_SESSION {
            assert!(session.push("move".into(), json!({ "i": i })), "event {i} fits");
        }
        assert!(!session.push("move".into(), json!({})), "one past the cap is refused, not queued without limit");
        session.raise("$disconnect", json!({ "reason": "flood" }));
        assert_eq!(session.pending(), MAX_PENDING_PER_SESSION + 1, "the server's own event is never refused");
    }

    #[test]
    fn an_event_waits_until_it_is_handled() {
        let (session, mut rx) = queue();
        assert!(session.push("bid".into(), json!({ "n": 1 })));
        let event = rx.try_recv().expect("queued");
        assert_eq!((event.session.as_str(), event.event.as_str(), &event.auth), ("s1", "bid", &json!({ "sub": "u-17" })));
        assert_eq!(session.pending(), 1, "taken off the queue is not yet run");
        drop(event);
        assert_eq!(session.pending(), 0);
    }
}

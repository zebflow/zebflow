//! A tick room's ticks (`rooms.md` § Events and ticks).
//!
//! In a tick room every client event waits for the next tick: at most
//! `--max-inputs` per session, the rest dropped and counted
//! (`inputs_dropped`). Each tick runs the room's `$tick` pipelines once with
//! `input.room: { room_id, event: "$tick", tick, dt_ms, inputs }`, every
//! waiting input in arrival order; an empty tick runs too, since timers and
//! respawns live in state. Ticks never overlap: one that outlasts `--every`
//! delays the next and counts a `tick_overrun`. What the tick changed and
//! sent goes out once, when it ends: one `patch { tick, ops, events }` per
//! session.
//!
//! `$connect` and `$disconnect` run their own triggers as they arrive — but a
//! `$disconnect` whose session still has inputs waiting runs right after the
//! tick that takes them, so it always comes after every event its socket
//! sent and a late move cannot bring back a player who left.

use std::collections::HashMap;

use serde_json::json;
use tokio::time::Instant;

use super::events::{Resolved, RoomCtx, run_pipeline};
use crate::infra::transport::ws::{Counter, RoomEvent, RoomHandle, TickSpec};
use crate::pipeline::nodes::basic::ws::trigger::TICK_EVENT;

/// The inputs waiting for the next tick, and when it is due.
#[derive(Default)]
pub(super) struct Ticker {
    spec: Option<TickSpec>,
    next_at: Option<Instant>,
    last_start: Option<Instant>,
    inputs: Vec<RoomEvent>,
    per_session: HashMap<String, u32>,
    /// `$` events held until the inputs before them have had their tick.
    after_tick: Vec<RoomEvent>,
}

impl Ticker {
    pub(super) fn new(spec: Option<TickSpec>) -> Self {
        let mut ticker = Self::default();
        ticker.retime(spec);
        ticker
    }

    /// Whether client events wait for a tick.
    pub(super) fn is_on(&self) -> bool {
        self.spec.is_some()
    }

    /// Whether inputs wait for the next tick.
    pub(super) fn has_waiting(&self) -> bool {
        !self.inputs.is_empty()
    }

    /// When the next tick is due; `None` in an event room.
    pub(super) fn due_at(&self) -> Option<Instant> {
        self.next_at
    }

    /// The room's tick as the active set now declares it. Inputs that waited
    /// for a tick the room no longer has come back, to run as events.
    pub(super) fn retime(&mut self, spec: Option<TickSpec>) -> Vec<RoomEvent> {
        if spec == self.spec {
            return Vec::new();
        }
        self.spec = spec;
        match spec {
            Some(spec) => {
                self.next_at = Some(self.last_start.unwrap_or_else(Instant::now) + spec.every());
                Vec::new()
            }
            None => {
                self.next_at = None;
                self.last_start = None;
                self.per_session.clear();
                let mut back = std::mem::take(&mut self.inputs);
                back.append(&mut self.after_tick);
                back
            }
        }
    }

    /// A client event waits for the next tick, unless its session already
    /// has `--max-inputs` waiting.
    pub(super) fn take(&mut self, event: RoomEvent, room: &RoomHandle) {
        let max = self.spec.map_or(0, |spec| spec.max_inputs);
        let waiting = self.per_session.entry(event.session.clone()).or_insert(0);
        if *waiting >= max {
            room.count(Counter::InputsDropped);
            return;
        }
        *waiting += 1;
        self.inputs.push(event);
    }

    /// Hold a `$` event of a session that has inputs waiting until after
    /// the tick that takes them; `Err` gives it back to run now.
    pub(super) fn hold(&mut self, event: RoomEvent) -> Result<(), RoomEvent> {
        if self.per_session.get(&event.session).is_some_and(|n| *n > 0) {
            self.after_tick.push(event);
            return Ok(());
        }
        Err(event)
    }

    /// Run one tick, then send the state it changed. Answers the `$` events
    /// held for it, to run now.
    pub(super) async fn run(&mut self, ctx: &RoomCtx, room: &RoomHandle, resolved: &Resolved) -> Vec<RoomEvent> {
        let Some(spec) = self.spec else { return Vec::new() };
        let start = Instant::now();
        let dt_ms = self.last_start.map_or(spec.every_ms, |last| start.duration_since(last).as_millis() as u64);
        self.last_start = Some(start);
        room.count(Counter::Ticks);
        let taken = std::mem::take(&mut self.inputs);
        self.per_session.clear();
        let inputs: Vec<serde_json::Value> = taken
            .iter()
            .map(|e| json!({ "session": e.session, "event": e.event, "payload": e.payload, "auth": e.auth }))
            .collect();
        let tick = room.counter(Counter::Ticks);
        let input = json!({
            "room_id": ctx.room_id,
            "event": TICK_EVENT,
            "tick": tick,
            "dt_ms": dt_ms,
            "inputs": inputs,
        });
        room.begin_tick();
        for compiled in resolved.for_event(&ctx.room_id, TICK_EVENT) {
            run_pipeline(ctx, room, resolved, compiled, input.clone(), "tick").await;
        }
        // Handled: each session's inputs stop counting as waiting.
        drop(taken);
        room.end_tick(tick);
        if start.elapsed() > spec.every() {
            room.count(Counter::TickOverruns);
            self.next_at = Some(Instant::now());
        } else {
            self.next_at = Some(start + spec.every());
        }
        std::mem::take(&mut self.after_tick)
    }
}

/// Resolves at `at`; never, without one.
pub(super) async fn until(at: Option<Instant>) {
    match at {
        Some(at) => tokio::time::sleep_until(at).await,
        None => std::future::pending().await,
    }
}

//! One room: its settings, its sessions, its shared state and its broadcast
//! channel (`docs/contracts/rooms.md`).
//!
//! A room is a value the hub ([`super::WsHub`]) registers. While it has
//! sessions, one room loop takes its queue ([`super::queue`]) in order; with
//! none, nothing runs for it. A state change is applied by the caller, under
//! the state lock, and sent from there, so two writes are sent in the order
//! they were made; a `--batch` write marks the state dirty and one
//! short-lived task sends it 33 ms later, at most once a tick. How a send
//! reaches each session is [`super::delivery`].
//!
//! The room also carries what its loop must notice: [`RoomHandle::epoch`]
//! rises whenever the project's active `trigger.room` set changes, so the
//! loop resolves the room's pipelines again then — and only then — and
//! [`RoomHandle::tick`] says whether a `--event $tick` trigger makes it a
//! tick room. In a tick room every state change waits: it goes out once per
//! tick, after the tick pipeline ends ([`RoomHandle::flush`]).
//!
//! # Wire protocol `zebflow.rooms.v1` (server → client JSON)
//!
//! | `type` | Additional fields | When |
//! |---|---|---|
//! | `"joined"` | `session_id`, `room`, `state`, `resume?` | Sent by the socket handler on connect |
//! | `"patch"` | `ops`; in a tick room also `tick`, `events` | After a state change (at once, on the batch tick, or once per tick) |
//! | `"event"` | `event`, `payload` | After [`RoomHandle::emit`], outside a tick |
//! | `"resync"` | `state` | Sent by the socket handler when a session fell behind |
//!
//! Delivery is decided on the server: a frame is queued only for the
//! sessions its [`EmitTarget`] selects.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tokio::sync::{Notify, mpsc};

use super::delivery::{self, Attached, Delivery, Member};
use super::queue::{RoomEvent, RoomQueue, SessionQueue};

/// How long a `--batch` change waits before it is sent (≈ 30 fps).
pub const BATCH_TICK: Duration = Duration::from_millis(33);

/// The state cap of a room that names none (`--max-state`).
pub const DEFAULT_MAX_STATE: u64 = 1_000_000;

/// How long an opened room may sit empty before it closes (`--idle`).
pub const DEFAULT_IDLE: Duration = Duration::from_secs(600);

/// State mutation applied to the room's state tree at a JSON pointer.
#[derive(Debug, Clone, Copy)]
pub enum StateOp {
    /// Replace the value at the path, creating the objects above it.
    Set,
    /// Shallow-merge an object into the object at the path.
    Merge,
    /// Remove the key at the path.
    Delete,
}

/// Which sessions receive a message.
#[derive(Debug, Clone)]
pub enum EmitTarget {
    /// Every session in the room.
    All,
    /// The one session with this id.
    Session(String),
    /// Every session but this one.
    Others(String),
    /// Every session of this identity (`user:<sub>`).
    User(String),
}

/// Whether a room was declared by an exact `trigger.room --room` or opened
/// by `ws.room.open` under a pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomKind {
    Fixed,
    Opened,
}

impl RoomKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RoomKind::Fixed => "fixed",
            RoomKind::Opened => "opened",
        }
    }
}

/// What makes a room a tick room: an active `trigger.room --event $tick
/// --every <ms> [--max-inputs <n>]` that matches it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TickSpec {
    /// `--every`, 16 ms to 1 s.
    pub every_ms: u64,
    /// `--max-inputs`: per session, the most inputs that wait for one tick.
    pub max_inputs: u32,
}

impl TickSpec {
    /// `--every` as a duration.
    pub fn every(&self) -> Duration {
        Duration::from_millis(self.every_ms)
    }
}

/// What `ws.room.open` sets; a fixed room has the defaults.
#[derive(Debug, Clone)]
pub struct RoomSettings {
    /// Only ticket holders may join (`--private`).
    pub private: bool,
    /// The most sessions at once (`--max-sessions`); `None` is no cap.
    pub max_sessions: Option<u32>,
    /// A small JSON object `ws.room.list` shows (`--meta`).
    pub meta: Value,
    /// How long the room may sit empty before it closes (`--idle`).
    pub idle: Duration,
    /// How long a dropped session's seat is kept (`--hold`); a fixed room has none.
    pub hold: Option<Duration>,
    /// The most bytes of state (`--max-state`); a write past it is refused.
    pub max_state: u64,
    /// The pipeline that opened the room, or that declares a fixed one.
    pub opened_by: String,
}

impl RoomSettings {
    /// A fixed room: public, uncapped, the default state cap.
    pub fn fixed(declared_by: &str) -> Self {
        Self {
            private: false,
            max_sessions: None,
            meta: json!({}),
            idle: DEFAULT_IDLE,
            hold: None,
            max_state: DEFAULT_MAX_STATE,
            opened_by: declared_by.to_string(),
        }
    }
}

/// A counter of [`RoomHandle`]; they only rise.
#[derive(Debug, Clone, Copy)]
pub enum Counter {
    MessagesIn,
    MessagesOut,
    PipelineRuns,
    PipelineErrors,
    Resyncs,
    FloodCloses,
    Ticks,
    TickOverruns,
    InputsDropped,
}

#[derive(Default)]
struct Counters([AtomicU64; 9]);

/// Why a socket is closed by the server, sent as its close frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kick {
    /// The WebSocket close code.
    pub code: u16,
    /// The `$disconnect` reason, also the close frame's reason.
    pub reason: &'static str,
}

/// `ws.room.close`: `4410`, `$disconnect { reason: "room_closed" }`.
pub const KICK_ROOM_CLOSED: Kick = Kick { code: 4410, reason: "room_closed" };
/// `ws.session.close`: `4409`, `$disconnect { reason: "closed" }`.
pub const KICK_CLOSED: Kick = Kick { code: 4409, reason: "closed" };

/// A state write past the room's `--max-state`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateFull {
    pub limit: u64,
    pub would_be: u64,
}

/// One room. Shared by its sessions and the nodes that reach it.
pub struct RoomHandle {
    name: String,
    kind: RoomKind,
    settings: RoomSettings,
    /// Changes every time a room of this name is opened; a ticket names it.
    instance: String,
    opened_at: String,
    state: RwLock<Value>,
    state_bytes: AtomicU64,
    dirty: AtomicBool,
    /// Lock order: `delivery`, then `state`, then `members`.
    delivery: Mutex<Delivery>,
    members: Mutex<HashMap<String, Member>>,
    /// Set while a tick's pipelines run: what they send waits for the tick's frame.
    ticking: AtomicBool,
    counters: Counters,
    last_activity_ms: AtomicI64,
    empty_since: Mutex<Option<Instant>>,
    /// The room's queue, held weakly: its sessions hold it alive.
    queue: Mutex<Option<mpsc::WeakUnboundedSender<RoomEvent>>>,
    /// Held by the room loop that is running; the next one waits for it.
    turn: tokio::sync::Mutex<()>,
    /// Rises when the project's active `trigger.room` set changes.
    epoch: AtomicU64,
    /// Set when a `--event $tick` trigger matches the room.
    tick: RwLock<Option<TickSpec>>,
    /// Wakes the room loop when [`Self::epoch`] rose.
    changed: Notify,
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn rfc3339(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .unwrap_or_default()
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

impl RoomHandle {
    /// A new, empty room.
    pub fn new(name: &str, kind: RoomKind, settings: RoomSettings) -> Arc<Self> {
        let now = now_ms();
        Arc::new(Self {
            name: name.to_string(),
            kind,
            settings,
            instance: uuid::Uuid::new_v4().simple().to_string(),
            opened_at: rfc3339(now),
            state: RwLock::new(json!({})),
            state_bytes: AtomicU64::new(2),
            dirty: AtomicBool::new(false),
            delivery: Mutex::new(Delivery::new()),
            members: Mutex::new(HashMap::new()),
            ticking: AtomicBool::new(false),
            counters: Counters::default(),
            last_activity_ms: AtomicI64::new(now),
            empty_since: Mutex::new(Some(Instant::now())),
            queue: Mutex::new(None),
            turn: tokio::sync::Mutex::new(()),
            epoch: AtomicU64::new(0),
            tick: RwLock::new(None),
            changed: Notify::new(),
        })
    }

    /// One session's way into the room's queue. The second value is the
    /// queue's receiving end when no loop is taking it — the caller starts
    /// one with it.
    pub fn session_queue(&self, session_id: &str, auth: Value) -> (SessionQueue, Option<RoomQueue>) {
        let mut slot = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(tx) = slot.as_ref().and_then(|weak| weak.upgrade()) {
            return (SessionQueue::new(tx, session_id, auth), None);
        }
        let (tx, rx) = mpsc::unbounded_channel();
        *slot = Some(tx.downgrade());
        (SessionQueue::new(tx, session_id, auth), Some(rx))
    }

    /// The room loop's turn: a loop holds it while it runs, so a loop started
    /// while the last one still drains waits, and the room never handles two
    /// things at once.
    pub async fn turn(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.turn.lock().await
    }

    /// Rises each time the project's active `trigger.room` set changes.
    pub fn epoch(&self) -> u64 {
        self.epoch.load(Ordering::Acquire)
    }

    /// Resolves once [`Self::epoch`] has risen since the last call.
    pub async fn changed(&self) {
        self.changed.notified().await
    }

    /// The project's active `trigger.room` set changed; `tick` is what it
    /// now says of this room.
    pub(super) fn redeclared(&self, tick: Option<TickSpec>) {
        *self.tick.write().unwrap_or_else(|e| e.into_inner()) = tick;
        self.epoch.fetch_add(1, Ordering::AcqRel);
        self.changed.notify_one();
    }

    /// The tick, when this is a tick room.
    pub fn tick(&self) -> Option<TickSpec> {
        *self.tick.read().unwrap_or_else(|e| e.into_inner())
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn kind(&self) -> RoomKind {
        self.kind
    }
    pub fn settings(&self) -> &RoomSettings {
        &self.settings
    }
    pub fn instance(&self) -> &str {
        &self.instance
    }

    fn delivery(&self) -> std::sync::MutexGuard<'_, Delivery> {
        self.delivery.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn members(&self) -> std::sync::MutexGuard<'_, HashMap<String, Member>> {
        self.members.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Send a named event to the sessions `to` selects: an `event` frame at
    /// once, or, while a tick runs, a part of the tick's frame.
    pub fn emit(&self, event: &str, payload: Value, to: EmitTarget) {
        let mut delivery = self.delivery();
        if self.ticking.load(Ordering::Acquire) {
            delivery.hold_for_tick(event, payload, to);
        } else {
            delivery.send_event(&delivery::event_frame(event, &payload), &to, &mut self.members());
        }
        drop(delivery);
        self.touch();
    }

    /// Give the socket of the seated session `session_id` its frames and its
    /// whole view; `None` when it holds no seat.
    pub fn attach(&self, session_id: &str) -> Option<Attached> {
        let mut delivery = self.delivery();
        let mut members = self.members();
        members.get_mut(session_id).map(|member| delivery.attach(member))
    }

    /// The seat stays, but no socket holds it: nothing is sent to it.
    pub fn detach(&self, session_id: &str) {
        let _delivery = self.delivery();
        if let Some(member) = self.members().get_mut(session_id) {
            delivery::detach(member);
        }
    }

    /// Whether the session fell more than a backlog behind.
    pub fn is_behind(&self, session_id: &str) -> bool {
        self.members().get(session_id).is_some_and(delivery::is_behind)
    }

    /// A behind session's whole view, from which its next patch follows;
    /// its socket has dropped every frame it had queued.
    pub fn resync(&self, session_id: &str) -> Option<Value> {
        let mut delivery = self.delivery();
        let mut members = self.members();
        let view = members.get_mut(session_id).map(|member| delivery.resync(member))?;
        self.count(Counter::Resyncs);
        Some(view)
    }

    /// The room's state now.
    pub fn get_state(&self) -> Value {
        self.state.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// The size of the state as last measured, in bytes of JSON.
    pub fn state_bytes(&self) -> u64 {
        self.state_bytes.load(Ordering::Relaxed)
    }

    /// Change the state at `path`. At once, every session is sent the whole
    /// state; with `batch`, it is sent on the next 33 ms tick; in a tick room,
    /// when the tick ends ([`Self::flush`]). A change that would take the
    /// state past `--max-state` is undone and refused.
    pub fn apply(self: &Arc<Self>, op: StateOp, path: &str, value: Option<Value>, batch: bool) -> Result<(), StateFull> {
        let mut state = self.state.write().unwrap_or_else(|e| e.into_inner());
        let saved = json_ptr_get(&state, path).cloned();
        let before = saved.as_ref().map(json_len).unwrap_or(0);
        apply_state_op(&mut state, op, path, value);
        let limit = self.settings.max_state;
        let undo = |state: &mut Value| match &saved {
            Some(old) => json_ptr_set(state, path, old.clone()),
            None => json_ptr_delete(state, path),
        };
        let tick_room = self.tick().is_some();
        if batch || tick_room {
            let after = json_ptr_get(&state, path).map(json_len).unwrap_or(0);
            let estimate = (self.state_bytes() + after).saturating_sub(before);
            if estimate > limit {
                undo(&mut state);
                return Err(StateFull { limit, would_be: estimate });
            }
            self.state_bytes.store(estimate, Ordering::Relaxed);
            drop(state);
            self.touch();
            // A tick room's loop sends it when the tick ends.
            if !self.dirty.swap(true, Ordering::AcqRel) && !tick_room {
                self.schedule_flush();
            }
            return Ok(());
        }
        let text = state.to_string();
        if text.len() as u64 > limit {
            undo(&mut state);
            return Err(StateFull { limit, would_be: text.len() as u64 });
        }
        self.state_bytes.store(text.len() as u64, Ordering::Relaxed);
        drop(state);
        // A batch change still waiting goes out with this one.
        self.dirty.store(true, Ordering::Release);
        self.send_changes(None);
        self.touch();
        Ok(())
    }

    /// Send a batch on the fast lane, [`BATCH_TICK`] from now.
    fn schedule_flush(self: &Arc<Self>) {
        let room = self.clone();
        super::lane().spawn(async move {
            tokio::time::sleep(BATCH_TICK).await;
            room.flush();
        });
    }

    /// Send what changed if a change is waiting (a batch's 33 ms).
    pub fn flush(&self) {
        self.send_changes(None);
    }

    /// A tick's pipelines start: what they send waits for its frame.
    pub fn begin_tick(&self) {
        self.ticking.store(true, Ordering::Release);
    }

    /// A tick ended: one frame per session carries the state it changed and
    /// the events it sent.
    pub fn end_tick(&self, tick: u64) {
        self.ticking.store(false, Ordering::Release);
        self.send_changes(Some(tick));
    }

    /// Send every session what changed since the last send.
    fn send_changes(&self, tick: Option<u64>) {
        let mut delivery = self.delivery();
        let dirty = self.dirty.swap(false, Ordering::AcqRel);
        if !dirty && (tick.is_none() || !delivery.has_tick_events()) {
            return;
        }
        let state = self.state.read().unwrap_or_else(|e| e.into_inner()).clone();
        if dirty {
            self.state_bytes.store(json_len(&state), Ordering::Relaxed);
        }
        delivery.send_changes(state, &mut self.members(), tick);
    }

    /// Forget the state (a fixed room whose last session left).
    fn reset_state(&self) {
        let mut delivery = self.delivery();
        let mut state = self.state.write().unwrap_or_else(|e| e.into_inner());
        *state = json!({});
        delivery.reset();
        self.state_bytes.store(2, Ordering::Relaxed);
        self.dirty.store(false, Ordering::Release);
    }

    /// Add one to a counter.
    pub fn count(&self, counter: Counter) {
        self.counters.0[counter as usize].fetch_add(1, Ordering::Relaxed);
    }

    /// A counter's reading.
    pub fn counter(&self, counter: Counter) -> u64 {
        self.counters.0[counter as usize].load(Ordering::Relaxed)
    }

    /// Mark the room active now.
    pub fn touch(&self) {
        self.last_activity_ms.store(now_ms(), Ordering::Relaxed);
    }

    /// Sessions joined now.
    pub fn session_count(&self) -> usize {
        self.members.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// Whether another session may join.
    pub(super) fn has_seat(&self) -> bool {
        self.settings.max_sessions.is_none_or(|max| self.session_count() < max as usize)
    }

    pub(super) fn add_member(&self, session_id: &str, user: Option<String>, kick: mpsc::UnboundedSender<Kick>) {
        self.members().insert(session_id.to_string(), Member::new(user, kick));
        *self.empty_since.lock().unwrap_or_else(|e| e.into_inner()) = None;
        self.touch();
    }

    /// Remove a session; true when the room is empty after it.
    pub(super) fn remove_member(&self, session_id: &str) -> bool {
        let empty = {
            let mut members = self.members.lock().unwrap_or_else(|e| e.into_inner());
            members.remove(session_id);
            members.is_empty()
        };
        self.touch();
        if empty {
            *self.empty_since.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
            if self.kind == RoomKind::Fixed {
                self.reset_state();
            }
        }
        empty
    }

    /// Whether an opened room has sat empty past its `--idle` at `now`.
    pub(super) fn idle_at(&self, now: Instant) -> bool {
        self.kind == RoomKind::Opened
            && self
                .empty_since
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .is_some_and(|since| now.saturating_duration_since(since) >= self.settings.idle)
    }

    /// Close the sessions `which` selects with `kick`; how many.
    pub(super) fn kick(&self, which: &EmitTarget, kick: Kick) -> usize {
        let members = self.members.lock().unwrap_or_else(|e| e.into_inner());
        let mut closed = 0;
        for (id, member) in members.iter() {
            if member.is_targeted(which, id) && member.kick.send(kick).is_ok() {
                closed += 1;
            }
        }
        closed
    }

    /// The identities joined, each once, sorted.
    pub fn member_users(&self) -> Vec<String> {
        let members = self.members.lock().unwrap_or_else(|e| e.into_inner());
        let mut users: Vec<String> = members.values().filter_map(|m| m.user.clone()).collect();
        users.sort();
        users.dedup();
        users
    }

    /// This room as `ws.room.list` shows it.
    pub fn describe(&self, members: bool) -> Value {
        let s = &self.settings;
        let mut entry = json!({
            "room": self.name,
            "kind": self.kind.as_str(),
            "private": s.private,
            "mode": if self.tick().is_some() { "tick" } else { "event" },
            "meta": s.meta,
            "sessions": self.session_count(),
            "max_sessions": s.max_sessions,
            "opened_at": self.opened_at,
            "opened_by": s.opened_by,
            "last_activity_at": rfc3339(self.last_activity_ms.load(Ordering::Relaxed)),
            "state_bytes": self.state_bytes(),
            "counters": {
                "messages_in": self.counter(Counter::MessagesIn),
                "messages_out": self.counter(Counter::MessagesOut),
                "ticks": self.counter(Counter::Ticks),
                "tick_overruns": self.counter(Counter::TickOverruns),
                "inputs_dropped": self.counter(Counter::InputsDropped),
                "pipeline_runs": self.counter(Counter::PipelineRuns),
                "pipeline_errors": self.counter(Counter::PipelineErrors),
                "resyncs": self.counter(Counter::Resyncs),
                "flood_closes": self.counter(Counter::FloodCloses),
            },
        });
        if members {
            entry["members"] = json!(self.member_users());
        }
        entry
    }
}

fn json_len(value: &Value) -> u64 {
    serde_json::to_vec(value).map(|v| v.len() as u64).unwrap_or(0)
}

// ---- State helpers ---------------------------------------------------------

fn segments(path: &str) -> Vec<&str> {
    path.trim_start_matches('/').split('/').filter(|s| !s.is_empty()).collect()
}

fn json_ptr_get<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = root;
    for part in segments(path) {
        cur = cur.as_object()?.get(part)?;
    }
    Some(cur)
}

/// Apply a [`StateOp`] mutation to the state tree.
fn apply_state_op(state: &mut Value, op: StateOp, path: &str, value: Option<Value>) {
    match op {
        StateOp::Set => json_ptr_set(state, path, value.unwrap_or(Value::Null)),
        StateOp::Merge => {
            if let Some(Value::Object(patch)) = value {
                json_ptr_merge(state, path, patch);
            }
        }
        StateOp::Delete => json_ptr_delete(state, path),
    }
}

/// Set `value` at `path`, creating intermediate objects as needed.
fn json_ptr_set(root: &mut Value, path: &str, value: Value) {
    let parts = segments(path);
    let Some((last, above)) = parts.split_last() else {
        *root = value;
        return;
    };
    let mut cur = root;
    for part in above {
        let Value::Object(map) = cur else { return };
        cur = map.entry(part.to_string()).or_insert_with(|| json!({}));
    }
    if let Value::Object(map) = cur {
        map.insert((*last).to_string(), value);
    }
}

/// Shallow-merge `patch` into the object at `path`, creating it if absent.
fn json_ptr_merge(root: &mut Value, path: &str, patch: serde_json::Map<String, Value>) {
    let mut cur = root;
    for part in segments(path) {
        let Value::Object(map) = cur else { return };
        cur = map.entry(part.to_string()).or_insert_with(|| json!({}));
    }
    if let Value::Object(target) = cur {
        target.extend(patch);
    }
}

/// Remove the key at `path` from its parent object.
fn json_ptr_delete(root: &mut Value, path: &str) {
    let parts = segments(path);
    let Some((last, above)) = parts.split_last() else { return };
    let mut cur = root;
    for part in above {
        match cur.as_object_mut().and_then(|map| map.get_mut(*part)) {
            Some(next) => cur = next,
            None => return,
        }
    }
    if let Value::Object(map) = cur {
        map.remove(*last);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(max_state: u64) -> Arc<RoomHandle> {
        RoomHandle::new("lobby", RoomKind::Opened, RoomSettings { max_state, ..RoomSettings::fixed("t") })
    }

    /// Seat `id` signed in as `user` and attach a socket to it.
    pub(crate) fn seat(room: &RoomHandle, id: &str, user: Option<&str>) -> (Attached, mpsc::UnboundedReceiver<Kick>) {
        let (tx, rx) = mpsc::unbounded_channel();
        room.add_member(id, user.map(str::to_string), tx);
        (room.attach(id).expect("seated"), rx)
    }

    /// The next frame queued for a session, read.
    pub(crate) fn frame(attached: &mut Attached) -> Option<Value> {
        attached.frames.try_recv().ok().map(|text| serde_json::from_str(&text).expect("json"))
    }

    #[test]
    fn an_emit_reaches_only_the_sessions_its_target_selects() {
        let room = room(DEFAULT_MAX_STATE);
        let (mut s1, _k1) = seat(&room, "s1", Some("u-17"));
        let (mut s2, _k2) = seat(&room, "s2", Some("u-18"));
        room.emit("hi", json!({ "a": 1 }), EmitTarget::Session("s1".into()));
        assert_eq!(frame(&mut s1), Some(json!({ "type": "event", "event": "hi", "payload": { "a": 1 } })));
        assert_eq!(frame(&mut s2), None);
        room.emit("bye", json!({}), EmitTarget::Others("s1".into()));
        assert_eq!((frame(&mut s1), frame(&mut s2).map(|f| f["event"].clone())), (None, Some(json!("bye"))));
        room.emit("dm", json!({}), EmitTarget::User("u-17".into()));
        assert_eq!((frame(&mut s1).map(|f| f["event"].clone()), frame(&mut s2)), (Some(json!("dm")), None));
    }

    #[tokio::test]
    async fn a_change_sends_one_small_patch_at_once_and_a_batch_on_the_tick() {
        let room = room(DEFAULT_MAX_STATE);
        room.apply(StateOp::Set, "/players", Some(json!({ "s1": { "x": 0, "y": 0 }, "s2": { "x": 9, "y": 9 } })), false).unwrap();
        let (mut s1, _k) = seat(&room, "s1", None);
        assert_eq!(s1.view, json!({ "players": { "s1": { "x": 0, "y": 0 }, "s2": { "x": 9, "y": 9 } } }), "attach gives the whole view");
        room.apply(StateOp::Merge, "/players/s1", Some(json!({ "x": 1 })), false).unwrap();
        assert_eq!(
            frame(&mut s1),
            Some(json!({ "type": "patch", "ops": [ { "op": "set", "path": ["players", "s1", "x"], "value": 1 } ] })),
            "only the changed leaf, not the state"
        );
        room.apply(StateOp::Set, "/b", Some(json!(2)), true).unwrap();
        room.apply(StateOp::Delete, "/players/s2", None, true).unwrap();
        assert_eq!(frame(&mut s1), None, "a batch waits for the tick");
        tokio::time::sleep(BATCH_TICK * 3).await;
        let tick = frame(&mut s1).expect("one send");
        assert_eq!(tick["ops"], json!([ { "op": "del", "path": ["players", "s2"] }, { "op": "set", "path": ["b"], "value": 2 } ]), "two batch changes, one patch");
        assert_eq!(frame(&mut s1), None);
        room.apply(StateOp::Set, "/b", Some(json!(2)), false).unwrap();
        assert_eq!(frame(&mut s1), None, "a write that changes nothing sends nothing");
    }

    #[test]
    fn a_session_that_falls_behind_is_skipped_until_it_resyncs() {
        let room = room(DEFAULT_MAX_STATE);
        let (mut slow, _k) = seat(&room, "slow", None);
        for i in 0..delivery::SESSION_BACKLOG + 5 {
            room.apply(StateOp::Set, "/n", Some(json!(i)), false).unwrap();
        }
        assert!(room.is_behind("slow"));
        while slow.frames.try_recv().is_ok() {}
        assert_eq!(room.resync("slow"), Some(json!({ "n": delivery::SESSION_BACKLOG + 4 })), "the whole view as last sent");
        assert!(!room.is_behind("slow"));
        room.apply(StateOp::Set, "/n", Some(json!(0)), false).unwrap();
        assert_eq!(frame(&mut slow).unwrap()["ops"], json!([ { "op": "set", "path": ["n"], "value": 0 } ]), "patches follow the resync");
        assert_eq!(room.counter(Counter::Resyncs), 1);
    }

    #[test]
    fn a_detached_seat_is_sent_nothing_and_attaching_again_gives_the_whole_view() {
        let room = room(DEFAULT_MAX_STATE);
        let (mut first, _k) = seat(&room, "s1", None);
        room.detach("s1");
        room.apply(StateOp::Set, "/a", Some(json!(1)), false).unwrap();
        room.emit("hi", json!({}), EmitTarget::All);
        assert_eq!(frame(&mut first), None);
        assert_eq!(room.session_count(), 1, "the seat stays");
        let mut again = room.attach("s1").expect("seated");
        assert_eq!(again.view, json!({ "a": 1 }));
        assert_eq!(frame(&mut again), None);
    }

    #[test]
    fn a_write_past_max_state_is_refused_and_undone() {
        let room = room(40);
        room.apply(StateOp::Set, "/name", Some(json!("ok")), false).expect("fits");
        let big = json!("x".repeat(64));
        for batch in [false, true] {
            let err = room.apply(StateOp::Set, "/name", Some(big.clone()), batch).expect_err("past the cap");
            assert_eq!(err.limit, 40);
            assert_eq!(room.get_state(), json!({ "name": "ok" }), "batch={batch}: the old value stays");
            let err = room.apply(StateOp::Set, "/other", Some(big.clone()), batch).expect_err("a new key too");
            assert!(err.would_be > 40);
            assert_eq!(room.get_state(), json!({ "name": "ok" }), "batch={batch}: the new key is removed");
        }
        room.apply(StateOp::Delete, "/name", None, false).expect("a delete fits");
        assert_eq!(room.state_bytes(), 2);
    }

    #[test]
    fn put_merge_and_delete_change_the_tree() {
        let room = room(DEFAULT_MAX_STATE);
        room.apply(StateOp::Set, "/players/s1", Some(json!({ "x": 0, "y": 0 })), false).unwrap();
        room.apply(StateOp::Merge, "/players/s1", Some(json!({ "x": 5 })), false).unwrap();
        assert_eq!(room.get_state(), json!({ "players": { "s1": { "x": 5, "y": 0 } } }));
        room.apply(StateOp::Delete, "/players/s1", None, false).unwrap();
        assert_eq!(room.get_state(), json!({ "players": {} }));
    }

    #[tokio::test]
    async fn a_tick_sends_one_frame_with_its_ops_and_its_events() {
        let room = room(DEFAULT_MAX_STATE);
        room.redeclared(Some(TickSpec { every_ms: 50, max_inputs: 30 }));
        let (mut s1, _k1) = seat(&room, "s1", None);
        let (mut s2, _k2) = seat(&room, "s2", None);
        room.apply(StateOp::Set, "/a", Some(json!(1)), false).unwrap();
        room.begin_tick();
        room.apply(StateOp::Set, "/b", Some(json!(2)), true).unwrap();
        room.emit("hit", json!({ "by": "s1" }), EmitTarget::All);
        room.emit("you", json!({}), EmitTarget::Session("s2".into()));
        tokio::time::sleep(BATCH_TICK * 2).await;
        assert_eq!(frame(&mut s1), None, "nothing goes out during the tick, not even a batch or an event");
        assert_eq!(room.get_state(), json!({ "a": 1, "b": 2 }), "the state itself changed at once");
        room.end_tick(7);
        let ops = json!([ { "op": "set", "path": ["a"], "value": 1 }, { "op": "set", "path": ["b"], "value": 2 } ]);
        assert_eq!(
            frame(&mut s1),
            Some(json!({ "type": "patch", "tick": 7, "ops": ops, "events": [ { "event": "hit", "payload": { "by": "s1" } } ] }))
        );
        assert_eq!(frame(&mut s2).unwrap()["events"], json!([ { "event": "hit", "payload": { "by": "s1" } }, { "event": "you", "payload": {} } ]));
        assert_eq!((frame(&mut s1), frame(&mut s2)), (None, None), "one frame per tick");
        room.begin_tick();
        room.end_tick(8);
        assert_eq!(frame(&mut s1), None, "a tick that changed nothing sends nothing");
        room.emit("between", json!({}), EmitTarget::All);
        assert_eq!(frame(&mut s1).unwrap()["type"], "event", "outside a tick an event goes out as an event frame");
        assert_eq!(room.describe(false)["mode"], "tick");
        room.redeclared(None);
        assert_eq!(room.describe(false)["mode"], "event");
        room.apply(StateOp::Set, "/a", Some(json!(3)), false).unwrap();
        assert!(frame(&mut s1).is_some(), "an event room sends at once again");
    }

    #[test]
    fn a_fixed_room_forgets_its_state_with_its_last_session() {
        let fixed = RoomHandle::new("lobby", RoomKind::Fixed, RoomSettings::fixed("t"));
        let (tx, _rx) = mpsc::unbounded_channel();
        fixed.add_member("s1", None, tx);
        fixed.apply(StateOp::Set, "/a", Some(json!(1)), false).unwrap();
        assert!(fixed.remove_member("s1"));
        assert_eq!(fixed.get_state(), json!({}));
    }
}

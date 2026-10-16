//! One room: its settings, its sessions, its shared state and its broadcast
//! channel (`docs/contracts/rooms.md`).
//!
//! A room is a value the hub ([`super::WsHub`]) registers. While it has
//! sessions, one room loop takes its queue ([`super::queue`]) in order; with
//! none, nothing runs for it. A state change is applied by the caller, under
//! the state lock, and broadcast from there, so two writes are sent in the
//! order they were made; a `--batch` write marks the state dirty and one
//! short-lived task sends it 33 ms later, at most once a tick.
//!
//! The room also carries what its loop must notice: [`RoomHandle::epoch`]
//! rises whenever the project's active `trigger.room` set changes, so the
//! loop resolves the room's pipelines again then — and only then.
//!
//! # Wire protocol (server → client JSON)
//!
//! | `type` | Additional fields | When |
//! |---|---|---|
//! | `"joined"` | `session_id`, `room`, `state` | Sent by the socket handler on connect |
//! | `"state_patch"` | `state` | After a state change (at once, or on the batch tick) |
//! | `"event"` | `event`, `payload` | After [`RoomHandle::emit`] |
//! | `"resync"` | `state` | Sent by the socket handler when a session fell behind |
//!
//! Delivery is decided on the server: every [`RoomBroadcast`] carries its
//! [`EmitTarget`], and each session's socket loop forwards only what
//! [`RoomBroadcast::is_for`] admits.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tokio::sync::{Notify, broadcast, mpsc};

use super::queue::{RoomEvent, RoomQueue, SessionQueue};

/// Capacity of the per-room broadcast channel. A session that falls further
/// behind is sent `resync` with the whole state.
pub const BROADCAST_CAPACITY: usize = 256;

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

/// One message on a room's broadcast channel: the JSON text and who it is for.
#[derive(Debug, Clone)]
pub struct RoomBroadcast {
    /// Serialised JSON, shared by every receiver.
    pub text: Arc<str>,
    /// Which sessions may receive it.
    pub to: EmitTarget,
}

impl RoomBroadcast {
    /// Whether the session `session_id`, signed in as `user`, may receive this.
    pub fn is_for(&self, session_id: &str, user: Option<&str>) -> bool {
        match &self.to {
            EmitTarget::All => true,
            EmitTarget::Session(id) => id == session_id,
            EmitTarget::Others(id) => id != session_id,
            EmitTarget::User(sub) => user == Some(sub.as_str()),
        }
    }
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
    /// How long a dropped session's seat is kept (`--hold`); stored, not yet enforced.
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
}

#[derive(Default)]
struct Counters([AtomicU64; 6]);

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

struct Member {
    user: Option<String>,
    kick: mpsc::UnboundedSender<Kick>,
}

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
    broadcast_tx: broadcast::Sender<RoomBroadcast>,
    members: Mutex<HashMap<String, Member>>,
    counters: Counters,
    last_activity_ms: AtomicI64,
    empty_since: Mutex<Option<Instant>>,
    /// The room's queue, held weakly: its sessions hold it alive.
    queue: Mutex<Option<mpsc::WeakUnboundedSender<RoomEvent>>>,
    /// Held by the room loop that is running; the next one waits for it.
    turn: tokio::sync::Mutex<()>,
    /// Rises when the project's active `trigger.room` set changes.
    epoch: AtomicU64,
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
        let (broadcast_tx, _) = broadcast::channel(BROADCAST_CAPACITY);
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
            broadcast_tx,
            members: Mutex::new(HashMap::new()),
            counters: Counters::default(),
            last_activity_ms: AtomicI64::new(now),
            empty_since: Mutex::new(Some(Instant::now())),
            queue: Mutex::new(None),
            turn: tokio::sync::Mutex::new(()),
            epoch: AtomicU64::new(0),
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

    /// The project's active `trigger.room` set changed.
    pub(super) fn redeclared(&self) {
        self.epoch.fetch_add(1, Ordering::AcqRel);
        self.changed.notify_one();
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

    /// Subscribe to this room's broadcasts.
    pub fn subscribe(&self) -> broadcast::Receiver<RoomBroadcast> {
        self.broadcast_tx.subscribe()
    }

    /// Send a named event to the sessions `to` selects.
    pub fn emit(&self, event: &str, payload: Value, to: EmitTarget) {
        let msg = json!({ "type": "event", "event": event, "payload": payload });
        let _ = self.broadcast_tx.send(RoomBroadcast { text: msg.to_string().into(), to });
        self.touch();
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
    /// state; with `batch`, it is sent on the next 33 ms tick. A change that
    /// would take the state past `--max-state` is undone and refused.
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
        if batch {
            let after = json_ptr_get(&state, path).map(json_len).unwrap_or(0);
            let estimate = (self.state_bytes() + after).saturating_sub(before);
            if estimate > limit {
                undo(&mut state);
                return Err(StateFull { limit, would_be: estimate });
            }
            self.state_bytes.store(estimate, Ordering::Relaxed);
            drop(state);
            self.touch();
            if !self.dirty.swap(true, Ordering::AcqRel) {
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
        // A batch change still waiting goes out with this one.
        self.dirty.store(false, Ordering::Release);
        let _ = self.broadcast_tx.send(state_patch(&text));
        drop(state);
        self.touch();
        Ok(())
    }

    fn schedule_flush(self: &Arc<Self>) {
        match tokio::runtime::Handle::try_current() {
            Ok(runtime) => {
                let room = self.clone();
                runtime.spawn(async move {
                    tokio::time::sleep(BATCH_TICK).await;
                    room.flush();
                });
            }
            Err(_) => self.flush(),
        }
    }

    /// Send the state if a batch change is waiting.
    fn flush(&self) {
        let state = self.state.read().unwrap_or_else(|e| e.into_inner());
        if self.dirty.swap(false, Ordering::AcqRel) {
            let text = state.to_string();
            self.state_bytes.store(text.len() as u64, Ordering::Relaxed);
            let _ = self.broadcast_tx.send(state_patch(&text));
        }
    }

    /// Forget the state (a fixed room whose last session left).
    fn reset_state(&self) {
        let mut state = self.state.write().unwrap_or_else(|e| e.into_inner());
        *state = json!({});
        self.state_bytes.store(2, Ordering::Relaxed);
        self.dirty.store(false, Ordering::Release);
    }

    /// Add one to a counter.
    pub fn count(&self, counter: Counter) {
        self.counters.0[counter as usize].fetch_add(1, Ordering::Relaxed);
    }

    fn counter(&self, counter: Counter) -> u64 {
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
        self.members
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(session_id.to_string(), Member { user, kick });
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
            let hit = match which {
                EmitTarget::All => true,
                EmitTarget::Session(s) => s == id,
                EmitTarget::Others(s) => s != id,
                EmitTarget::User(sub) => member.user.as_deref() == Some(sub.as_str()),
            };
            if hit && member.kick.send(kick).is_ok() {
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
            "mode": "event",
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
                "ticks": 0,
                "tick_overruns": 0,
                "inputs_dropped": 0,
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

fn state_patch(state_text: &str) -> RoomBroadcast {
    RoomBroadcast {
        text: format!(r#"{{"type":"state_patch","state":{state_text}}}"#).into(),
        to: EmitTarget::All,
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

    #[test]
    fn a_target_selects_sessions_and_identities() {
        let b = |to| RoomBroadcast { text: "{}".into(), to };
        assert!(b(EmitTarget::Session("s1".into())).is_for("s1", None));
        assert!(!b(EmitTarget::Session("s1".into())).is_for("s2", None));
        assert!(!b(EmitTarget::Others("s1".into())).is_for("s1", None));
        assert!(b(EmitTarget::Others("s1".into())).is_for("s2", None));
        assert!(b(EmitTarget::User("u-17".into())).is_for("s9", Some("u-17")));
        assert!(!b(EmitTarget::User("u-17".into())).is_for("s9", Some("u-18")));
        assert!(!b(EmitTarget::User("u-17".into())).is_for("s9", None));
    }

    #[tokio::test]
    async fn an_emit_reaches_subscribers_with_its_target() {
        let room = room(DEFAULT_MAX_STATE);
        let mut rx = room.subscribe();
        room.emit("hi", json!({ "a": 1 }), EmitTarget::Session("s1".into()));
        let got = rx.recv().await.expect("broadcast");
        assert!(got.is_for("s1", None) && !got.is_for("s2", None));
        let v: Value = serde_json::from_str(&got.text).unwrap();
        assert_eq!(v, json!({ "type": "event", "event": "hi", "payload": { "a": 1 } }));
    }

    #[tokio::test]
    async fn a_change_is_sent_at_once_and_a_batch_on_the_tick() {
        let room = room(DEFAULT_MAX_STATE);
        let mut rx = room.subscribe();
        room.apply(StateOp::Set, "/a", Some(json!(1)), false).unwrap();
        let now: Value = serde_json::from_str(&rx.recv().await.unwrap().text).unwrap();
        assert_eq!(now, json!({ "type": "state_patch", "state": { "a": 1 } }));
        room.apply(StateOp::Set, "/b", Some(json!(2)), true).unwrap();
        room.apply(StateOp::Set, "/c", Some(json!(3)), true).unwrap();
        assert!(rx.try_recv().is_err(), "a batch waits for the tick");
        let tick: Value = serde_json::from_str(&rx.recv().await.unwrap().text).unwrap();
        assert_eq!(tick["state"], json!({ "a": 1, "b": 2, "c": 3 }), "two batch changes, one send");
        assert!(rx.try_recv().is_err());
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

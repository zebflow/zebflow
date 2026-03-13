//! Room actor — maintains shared state, broadcast channel, and tick loop.
//!
//! Each room runs a single Tokio task ([`run_room`]) that processes
//! [`RoomCmd`] messages and drives a 33 ms tick loop for batched state
//! broadcasts.  The actor is spawned via [`RoomHandle::spawn`] and accessed
//! through the returned [`Arc<RoomHandle>`].
//!
//! # Architecture
//!
//! ```text
//! Pipeline nodes / WS sessions
//!         │
//!         │  mpsc::UnboundedSender<RoomCmd>
//!         ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │  run_room  (Tokio task)                                     │
//! │                                                             │
//! │  PatchState      ──► mutate state ──► broadcast immediately │
//! │  PatchStateSilent──► mutate state ──► set dirty flag        │
//! │  Emit            ──► broadcast event immediately            │
//! │  Shutdown        ──► break loop                             │
//! │                                                             │
//! │  Tick (33 ms)    ──► if dirty: broadcast state_patch        │
//! └─────────────────────────────────────────────────────────────┘
//!         │
//!         │  broadcast::Sender<String>  (JSON)
//!         ▼
//!    All subscribed WS sessions
//! ```
//!
//! # Immediate vs tick-batched state updates
//!
//! | [`RoomCmd`] variant | Broadcast behaviour |
//! |---|---|
//! | [`RoomCmd::PatchState`] | Mutates state + broadcasts `state_patch` immediately |
//! | [`RoomCmd::PatchStateSilent`] | Mutates state + sets dirty flag — **no broadcast** |
//! | Tick (33 ms) | Broadcasts `state_patch` once if dirty flag is set, then clears it |
//!
//! **Use `PatchState`** for low-frequency, event-driven updates (chat messages,
//! score changes, door opens).
//!
//! **Use `PatchStateSilent`** for high-frequency streams such as 3D positional
//! updates.  At 30 fps × 20 players = 600 mutations/s, only 30 broadcasts/s
//! are produced — one compact full-state snapshot per tick.
//!
//! # Wire protocol (server → client JSON)
//!
//! | `type` | Additional fields | When |
//! |---|---|---|
//! | `"joined"` | `session_id`, `state` | Emitted once by the WS route handler on connect |
//! | `"state_patch"` | `state` | After any state mutation (immediate or tick) |
//! | `"event"` | `event`, `payload`, `to`, `target_session` | After [`RoomCmd::Emit`] |
//!
//! Clients receiving an `"event"` message should check `to` and
//! `target_session` before acting, since all sessions receive the same
//! broadcast stream.

use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use serde_json::{Value, json};
use tokio::sync::{broadcast, mpsc};
use tokio::time::{Duration, interval};

/// Capacity of the per-room broadcast channel.
///
/// If a slow subscriber falls more than this many messages behind, it will
/// receive a [`broadcast::error::RecvError::Lagged`] error and should
/// re-sync by requesting the current state snapshot.
pub const BROADCAST_CAPACITY: usize = 256;

/// Interval between tick-loop state-flush broadcasts (≈ 30 fps).
const TICK_INTERVAL_MS: u64 = 33;

// ---- Command types ---------------------------------------------------------

/// State mutation operation applied to the shared room state tree.
///
/// All operations accept a JSON-pointer `path` (e.g. `"/players/abc123"`)
/// that is resolved after dynamic interpolation by [`crate::ws::path::interpolate_path`].
#[derive(Debug, Clone)]
pub enum StateOp {
    /// Replace the value at `path` with the supplied value.
    ///
    /// Creates intermediate objects if the path does not exist.
    Set,

    /// Shallow-merge an object into the value at `path`.
    ///
    /// The target and the incoming value must both be JSON objects.
    /// If `path` is empty or `"/"`, merges at the root.
    /// Useful for updating individual fields of a sub-object without
    /// replacing the whole thing:
    ///
    /// ```text
    /// state  = { "players": { "abc": { "x": 0, "y": 0, "anim": "idle" } } }
    /// Merge("/players/abc", { "x": 5, "anim": "walk" })
    /// result = { "players": { "abc": { "x": 5, "y": 0, "anim": "walk" } } }
    /// ```
    Merge,

    /// Remove the key at `path` from its parent object.
    Delete,
}

/// Emit target — controls which connected sessions receive an event message.
///
/// All sessions subscribe to the same broadcast channel.  Clients are
/// responsible for filtering by the `to` / `target_session` fields in the
/// received JSON.
#[derive(Debug, Clone)]
pub enum EmitTarget {
    /// Broadcast to all connected sessions (no filtering).
    All,

    /// Deliver only to the session with the given `session_id`.
    ///
    /// Other sessions will receive the message but the client SDK should
    /// discard it.
    Session(String),

    /// Deliver to all sessions *except* the one with the given `session_id`.
    ///
    /// Useful for propagating a client's own action to peers without echoing
    /// it back to the originator (e.g. player movement in a multiplayer game).
    Others(String),
}

/// Commands processed asynchronously by the room actor task.
///
/// Send via [`RoomHandle::send_cmd`] (fire-and-forget, never blocks).
#[derive(Debug)]
pub enum RoomCmd {
    /// Mutate the shared room state and **immediately** broadcast a
    /// `state_patch` message to all subscribers.
    ///
    /// Use for low-frequency events (chat, score, game lifecycle).
    PatchState {
        /// The mutation type (`Set`, `Merge`, `Delete`).
        op: StateOp,
        /// JSON pointer path, already interpolated (see [`crate::ws::path`]).
        path: String,
        /// New value (ignored for `Delete`).
        value: Option<Value>,
    },

    /// Mutate the shared room state **silently** — no broadcast is sent.
    ///
    /// A dirty flag is set instead.  The room's 33 ms tick loop will flush
    /// the accumulated state to all subscribers at most once per tick.
    ///
    /// Use for high-frequency streams (≥ 10 fps positional updates, sensor
    /// readings) to avoid flooding client JS event loops.
    PatchStateSilent {
        /// The mutation type (`Set`, `Merge`, `Delete`).
        op: StateOp,
        /// JSON pointer path, already interpolated (see [`crate::ws::path`]).
        path: String,
        /// New value (ignored for `Delete`).
        value: Option<Value>,
    },

    /// Broadcast a named event to one or more sessions.
    ///
    /// Does not modify shared state.  Useful for transient notifications such
    /// as chat messages, AI narration, or game events that clients should not
    /// persist.
    Emit {
        /// Application-level event name, e.g. `"chat"`, `"ai_update"`.
        event: String,
        /// Arbitrary event payload.
        payload: Value,
        /// Which sessions should act on this event.
        to: EmitTarget,
    },

    /// Signal the room actor to stop its loop and release resources.
    ///
    /// Called by [`WsHub::remove_room`] when the last session disconnects.
    Shutdown,
}

// ---- Session guard ---------------------------------------------------------

/// RAII guard that decrements the room's session count when dropped.
///
/// Returned by [`RoomHandle::join_session`].  When the last guard is dropped,
/// [`WsHub::remove_room`] will remove the room from the registry.
pub struct SessionGuard {
    count: Arc<AtomicUsize>,
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        self.count.fetch_sub(1, Ordering::Relaxed);
    }
}

// ---- Room handle -----------------------------------------------------------

/// Handle to a running room actor — shared cheaply across WS sessions via `Arc`.
///
/// All methods are **synchronous and non-blocking**.  State reads use a
/// [`std::sync::RwLock`] that is never held across an `await` point, so reads
/// from async contexts are safe without `await`.
pub struct RoomHandle {
    cmd_tx: mpsc::UnboundedSender<RoomCmd>,
    broadcast_tx: broadcast::Sender<String>,
    session_count: Arc<AtomicUsize>,
    /// Shared room state — readable without going through the actor.
    state: Arc<RwLock<Value>>,
}

impl RoomHandle {
    /// Spawn a new room actor and return its handle.
    ///
    /// The actor task runs until [`RoomCmd::Shutdown`] is received or the
    /// last [`mpsc`] sender is dropped.
    pub fn spawn() -> Arc<Self> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (broadcast_tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        let session_count = Arc::new(AtomicUsize::new(0));
        let state = Arc::new(RwLock::new(json!({})));
        // dirty is local to the actor task only — not exposed on the handle.
        let dirty = Arc::new(AtomicBool::new(false));

        let handle = Arc::new(Self {
            cmd_tx,
            broadcast_tx: broadcast_tx.clone(),
            session_count: session_count.clone(),
            state: state.clone(),
        });

        tokio::spawn(run_room(cmd_rx, broadcast_tx, state, dirty));
        handle
    }

    /// Subscribe to broadcasts from this room.
    ///
    /// The receiver will receive all `state_patch` and `event` messages as
    /// JSON strings.  If the subscriber falls behind by more than
    /// [`BROADCAST_CAPACITY`] messages, it receives a
    /// [`broadcast::error::RecvError::Lagged`] error.
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.broadcast_tx.subscribe()
    }

    /// Send a command to the room actor (fire-and-forget, never blocks).
    ///
    /// The command is queued in an unbounded MPSC channel.  Errors (channel
    /// closed) are silently ignored — this happens only after [`Shutdown`].
    pub fn send_cmd(&self, cmd: RoomCmd) {
        let _ = self.cmd_tx.send(cmd);
    }

    /// Get a synchronous snapshot of the current room state.
    ///
    /// Acquires a read lock on the shared state — safe to call from both sync
    /// and async contexts since the lock is never held across an `await`.
    pub fn get_state(&self) -> Value {
        self.state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Register a new session and return a guard that auto-decrements on drop.
    ///
    /// Drop the returned [`SessionGuard`] when the WS connection closes.
    pub fn join_session(&self) -> SessionGuard {
        self.session_count.fetch_add(1, Ordering::Relaxed);
        SessionGuard {
            count: self.session_count.clone(),
        }
    }

    /// Current number of active WS sessions connected to this room.
    pub fn session_count(&self) -> usize {
        self.session_count.load(Ordering::Relaxed)
    }
}

// ---- Room actor ------------------------------------------------------------

/// Room actor loop.
///
/// Processes [`RoomCmd`] messages and drives the 33 ms tick loop.
async fn run_room(
    mut cmd_rx: mpsc::UnboundedReceiver<RoomCmd>,
    broadcast_tx: broadcast::Sender<String>,
    state: Arc<RwLock<Value>>,
    dirty: Arc<AtomicBool>,
) {
    let mut tick = interval(Duration::from_millis(TICK_INTERVAL_MS));

    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => {
                match cmd {
                    None | Some(RoomCmd::Shutdown) => break,

                    Some(RoomCmd::PatchState { op, path, value }) => {
                        // Mutate under write lock, release before broadcast.
                        {
                            let mut w = state.write().unwrap_or_else(|e| e.into_inner());
                            apply_state_op(&mut w, &op, &path, value);
                        }
                        let snapshot = state.read().unwrap_or_else(|e| e.into_inner()).clone();
                        let msg = json!({ "type": "state_patch", "state": snapshot });
                        let _ = broadcast_tx.send(msg.to_string());
                    }

                    Some(RoomCmd::PatchStateSilent { op, path, value }) => {
                        // Mutate only — tick loop will flush.
                        let mut w = state.write().unwrap_or_else(|e| e.into_inner());
                        apply_state_op(&mut w, &op, &path, value);
                        dirty.store(true, Ordering::Relaxed);
                    }

                    Some(RoomCmd::Emit { event, payload, to }) => {
                        let (to_label, target_session) = match &to {
                            EmitTarget::All => ("all", None),
                            EmitTarget::Session(id) => ("session", Some(id.clone())),
                            EmitTarget::Others(id) => ("others", Some(id.clone())),
                        };
                        let msg = json!({
                            "type": "event",
                            "event": event,
                            "payload": payload,
                            "to": to_label,
                            "target_session": target_session,
                        });
                        let _ = broadcast_tx.send(msg.to_string());
                    }
                }
            }

            _ = tick.tick() => {
                // Flush accumulated silent mutations once per tick (~30 fps).
                if dirty.swap(false, Ordering::Relaxed) {
                    let snapshot = state.read().unwrap_or_else(|e| e.into_inner()).clone();
                    let msg = json!({ "type": "state_patch", "state": snapshot });
                    let _ = broadcast_tx.send(msg.to_string());
                }
            }
        }
    }
}

// ---- State helpers ---------------------------------------------------------

/// Apply a [`StateOp`] mutation to the state tree.
fn apply_state_op(state: &mut Value, op: &StateOp, path: &str, value: Option<Value>) {
    match op {
        StateOp::Set => {
            if path.is_empty() || path == "/" {
                if let Some(v) = value {
                    *state = v;
                }
            } else {
                json_ptr_set(state, path, value.unwrap_or(Value::Null));
            }
        }
        StateOp::Merge => {
            if let Some(Value::Object(patch)) = value {
                if path.is_empty() || path == "/" {
                    // Merge at root.
                    if let Value::Object(current) = state {
                        for (k, v) in patch {
                            current.insert(k, v);
                        }
                    }
                } else {
                    // Navigate to target path and merge there.
                    json_ptr_merge(state, path, patch);
                }
            }
        }
        StateOp::Delete => {
            json_ptr_delete(state, path);
        }
    }
}

/// Set `value` at `path` (JSON pointer), creating intermediate objects as needed.
fn json_ptr_set(root: &mut Value, path: &str, value: Value) {
    let parts: Vec<&str> = path
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        *root = value;
        return;
    }
    let mut cur = root;
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            if let Value::Object(map) = cur {
                map.insert((*part).to_string(), value);
            }
            return;
        }
        if let Value::Object(map) = cur {
            cur = map
                .entry(part.to_string())
                .or_insert_with(|| json!({}));
        } else {
            return;
        }
    }
}

/// Shallow-merge `patch` into the object at `path`, creating it if absent.
fn json_ptr_merge(root: &mut Value, path: &str, patch: serde_json::Map<String, Value>) {
    let parts: Vec<&str> = path
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        if let Value::Object(current) = root {
            for (k, v) in patch {
                current.insert(k, v);
            }
        }
        return;
    }
    let mut cur = root;
    for part in &parts {
        if let Value::Object(map) = cur {
            cur = map
                .entry(part.to_string())
                .or_insert_with(|| json!({}));
        } else {
            return;
        }
    }
    if let Value::Object(target) = cur {
        for (k, v) in patch {
            target.insert(k, v);
        }
    }
}

/// Remove the key at `path` from its parent object.
fn json_ptr_delete(root: &mut Value, path: &str) {
    let parts: Vec<&str> = path
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        return;
    }
    let mut cur = root;
    for part in &parts[..parts.len() - 1] {
        if let Value::Object(map) = cur {
            if let Some(next) = map.get_mut(*part) {
                cur = next;
            } else {
                return;
            }
        } else {
            return;
        }
    }
    if let Value::Object(map) = cur {
        map.remove(*parts.last().unwrap());
    }
}

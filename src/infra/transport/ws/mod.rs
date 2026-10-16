//! WebSocket rooms: the registry of every project's rooms
//! (`docs/contracts/rooms.md`).
//!
//! | Module | Responsibility |
//! |---|---|
//! | [`pattern`] | room names and the patterns `trigger.room --room` declares |
//! | [`queue`] | a room's one queue: every session's events, in arrival order |
//! | [`room`] | one room: settings, sessions, state, broadcast, counters |
//! | [`ticket`] | signed, single-use passes into one room |
//! | [`WsHub`] | which rooms exist, and every way one opens, is joined or closes |
//!
//! **Every room exists because the server made it.** A fixed room exists
//! while an active `trigger.room` names it exactly ([`WsHub::declare`]); an
//! opened room exists from [`WsHub::open`] until [`WsHub::close`], or until
//! it has sat empty for its `--idle`. Joining a room that does not exist is
//! refused; no join creates one.
//!
//! Rooms are keyed `"{owner}/{project}/{room}"`.

pub mod pattern;
pub mod queue;
pub mod room;
pub mod ticket;

pub use pattern::RoomPattern;
pub use queue::{MAX_PENDING_PER_SESSION, RoomEvent, RoomQueue, SessionQueue};
pub use room::{
    Counter, EmitTarget, KICK_CLOSED, KICK_ROOM_CLOSED, Kick, RoomBroadcast, RoomHandle, RoomKind, RoomSettings,
    StateFull, StateOp,
};
pub use ticket::{Ticket, TicketPass};

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tokio::sync::mpsc;

/// How often empty opened rooms are checked against their `--idle`.
pub const IDLE_SWEEP: Duration = Duration::from_secs(1);

/// Why the hub refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HubRefusal {
    /// An exact room that is not open.
    NotOpen(String),
    /// The room is at its `--max-sessions`.
    Full(String),
    /// `open`: no active `trigger.room` pattern matches the name.
    Undeclared(String),
    /// `open`: the name is a fixed room's.
    Fixed(String),
    /// `open`: a room of that name is open.
    Taken(String),
    /// A name or a selection that does not read.
    BadName(String),
    /// The ticket was spent between reading and joining.
    TicketSpent,
}

impl std::fmt::Display for HubRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotOpen(room) => write!(f, "room '{room}' is not open"),
            Self::Full(room) => write!(f, "room '{room}' is full (--max-sessions)"),
            Self::Undeclared(room) => {
                write!(f, "no active trigger.room pattern matches '{room}': declare one (e.g. --room \"match-:id\") first")
            }
            Self::Fixed(room) => write!(f, "'{room}' is a fixed room (an exact trigger.room --room); it is always open"),
            Self::Taken(room) => write!(f, "room '{room}' is already open"),
            Self::BadName(why) => f.write_str(why),
            Self::TicketSpent => f.write_str("the ticket was already used"),
        }
    }
}

struct Inner {
    /// Lock order: `declared` before `rooms`.
    declared: Mutex<HashMap<String, Vec<(RoomPattern, String)>>>,
    rooms: Mutex<HashMap<String, Arc<RoomHandle>>>,
    tickets: ticket::TicketBook,
    sweeping: AtomicBool,
}

/// Every project's rooms. Cheap to clone; held as `PlatformService::ws_hub`.
#[derive(Clone)]
pub struct WsHub {
    inner: Arc<Inner>,
}

/// One session's seat; leaving is dropping it.
pub struct SessionGuard {
    hub: Weak<Inner>,
    key: String,
    session_id: String,
    instance: String,
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        if let Some(inner) = self.hub.upgrade() {
            let rooms = inner.rooms.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(room) = rooms.get(&self.key).filter(|r| r.instance() == self.instance) {
                room.remove_member(&self.session_id);
            }
        }
    }
}

fn project_key(owner: &str, project: &str) -> String {
    format!("{owner}/{project}")
}

/// The hub's key for a room of a project.
pub fn room_key(owner: &str, project: &str, room: &str) -> String {
    format!("{owner}/{project}/{room}")
}

impl Default for WsHub {
    fn default() -> Self {
        Self::new()
    }
}

impl WsHub {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                declared: Mutex::new(HashMap::new()),
                rooms: Mutex::new(HashMap::new()),
                tickets: ticket::TicketBook::default(),
                sweeping: AtomicBool::new(false),
            }),
        }
    }

    fn rooms(&self) -> std::sync::MutexGuard<'_, HashMap<String, Arc<RoomHandle>>> {
        self.inner.rooms.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The rooms a project's active `trigger.room` nodes declare, as
    /// `(--room, pipeline)`. A fixed room appears with its trigger and closes
    /// (`4410`) when no active trigger names it any more.
    pub fn declare(&self, owner: &str, project: &str, rooms: &[(String, String)]) {
        let parsed: Vec<(RoomPattern, String)> = rooms
            .iter()
            .filter_map(|(room, pipeline)| RoomPattern::parse(room).ok().map(|p| (p, pipeline.clone())))
            .collect();
        let mut declared = self.inner.declared.lock().unwrap_or_else(|e| e.into_inner());
        let mut registry = self.rooms();
        let fixed: HashMap<&str, &str> = parsed
            .iter()
            .filter_map(|(p, pipeline)| match p {
                RoomPattern::Exact(name) => Some((name.as_str(), pipeline.as_str())),
                _ => None,
            })
            .collect();
        let prefix = format!("{}/", project_key(owner, project));
        registry.retain(|key, room| {
            let gone = key.starts_with(&prefix) && room.kind() == RoomKind::Fixed && !fixed.contains_key(room.name());
            if gone {
                room.kick(&EmitTarget::All, KICK_ROOM_CLOSED);
            }
            !gone
        });
        for (name, pipeline) in &fixed {
            registry
                .entry(room_key(owner, project, name))
                .or_insert_with(|| RoomHandle::new(name, RoomKind::Fixed, RoomSettings::fixed(pipeline)));
        }
        // Every room of the project resolves its pipelines again.
        for (_, room) in registry.iter().filter(|(key, _)| key.starts_with(&prefix)) {
            room.redeclared();
        }
        drop(registry);
        if parsed.is_empty() {
            declared.remove(&project_key(owner, project));
        } else {
            declared.insert(project_key(owner, project), parsed);
        }
    }

    /// Whether `name` is a fixed room, a name an active pattern declares, or neither.
    pub fn declares(&self, owner: &str, project: &str, name: &str) -> Option<RoomKind> {
        let declared = self.inner.declared.lock().unwrap_or_else(|e| e.into_inner());
        let patterns = declared.get(&project_key(owner, project))?;
        if patterns.iter().any(|(p, _)| !p.is_pattern() && p.matches(name)) {
            return Some(RoomKind::Fixed);
        }
        patterns.iter().any(|(p, _)| p.is_pattern() && p.matches(name)).then_some(RoomKind::Opened)
    }

    /// Open the room `name` under an active pattern.
    pub fn open(&self, owner: &str, project: &str, name: &str, settings: RoomSettings) -> Result<Arc<RoomHandle>, HubRefusal> {
        if !pattern::valid_name(name) {
            return Err(HubRefusal::BadName(pattern::name_problem(name)));
        }
        match self.declares(owner, project, name) {
            Some(RoomKind::Fixed) => return Err(HubRefusal::Fixed(name.to_string())),
            None => return Err(HubRefusal::Undeclared(name.to_string())),
            Some(RoomKind::Opened) => {}
        }
        let room = {
            let mut rooms = self.rooms();
            let key = room_key(owner, project, name);
            if rooms.contains_key(&key) {
                return Err(HubRefusal::Taken(name.to_string()));
            }
            let room = RoomHandle::new(name, RoomKind::Opened, settings);
            rooms.insert(key, room.clone());
            room
        };
        self.ensure_sweeper();
        Ok(room)
    }

    /// The open room `name`, exactly.
    pub fn room(&self, owner: &str, project: &str, name: &str) -> Option<Arc<RoomHandle>> {
        self.rooms().get(&room_key(owner, project, name)).cloned()
    }

    /// The rooms a selection reaches: an exact name (which must be open), a
    /// pattern or `*` (every open room it matches, maybe none).
    pub fn select(&self, owner: &str, project: &str, selection: &str) -> Result<Vec<Arc<RoomHandle>>, HubRefusal> {
        let selection = selection.trim();
        if selection != "*" {
            match RoomPattern::parse(selection).map_err(HubRefusal::BadName)? {
                RoomPattern::Exact(name) => {
                    return self.room(owner, project, &name).map(|r| vec![r]).ok_or(HubRefusal::NotOpen(name));
                }
                RoomPattern::Wild { .. } => {}
            }
        }
        let prefix = format!("{}/", project_key(owner, project));
        let mut found: Vec<Arc<RoomHandle>> = self
            .rooms()
            .iter()
            .filter(|(key, room)| key.starts_with(&prefix) && pattern::selects(selection, room.name()))
            .map(|(_, room)| room.clone())
            .collect();
        found.sort_by(|a, b| a.name().cmp(b.name()));
        Ok(found)
    }

    /// Close the rooms a selection reaches: every session gets `4410`, the
    /// state is dropped. A fixed room comes back at once, empty. Answers each
    /// room closed with the sessions it held.
    pub fn close(&self, owner: &str, project: &str, selection: &str) -> Result<Vec<(String, usize)>, HubRefusal> {
        let selected = self.select(owner, project, selection)?;
        let mut rooms = self.rooms();
        let mut closed = Vec::new();
        for room in selected {
            let key = room_key(owner, project, room.name());
            if !rooms.get(&key).is_some_and(|r| Arc::ptr_eq(r, &room)) {
                continue;
            }
            let sessions = room.kick(&EmitTarget::All, KICK_ROOM_CLOSED);
            if room.kind() == RoomKind::Fixed {
                rooms.insert(key, RoomHandle::new(room.name(), RoomKind::Fixed, room.settings().clone()));
            } else {
                rooms.remove(&key);
            }
            closed.push((room.name().to_string(), sessions));
        }
        Ok(closed)
    }

    /// [`WsHub::close`] as `ws.room.close`, the API and the MCP answer it:
    /// `{ room, closed: [names], sessions }`.
    pub fn close_report(&self, owner: &str, project: &str, selection: &str) -> Result<Value, HubRefusal> {
        let closed = self.close(owner, project, selection)?;
        let sessions: usize = closed.iter().map(|(_, n)| n).sum();
        let names: Vec<&str> = closed.iter().map(|(name, _)| name.as_str()).collect();
        Ok(json!({ "room": selection, "closed": names, "sessions": sessions }))
    }

    /// Close the sessions `which` selects (a session id or a user) in the
    /// rooms a selection reaches, with `4409`. Answers (rooms, sessions).
    pub fn close_sessions(&self, owner: &str, project: &str, selection: &str, which: &EmitTarget) -> Result<(usize, usize), HubRefusal> {
        let rooms = self.select(owner, project, selection)?;
        let sessions = rooms.iter().map(|room| room.kick(which, KICK_CLOSED)).sum();
        Ok((rooms.len(), sessions))
    }

    /// `ws.room.list`: the open rooms a selection reaches, and the totals.
    pub fn list(&self, owner: &str, project: &str, selection: &str, members: bool) -> Result<Value, HubRefusal> {
        let selection = if selection.trim().is_empty() { "*" } else { selection };
        let rooms = match self.select(owner, project, selection) {
            Err(HubRefusal::NotOpen(_)) => Vec::new(),
            other => other?,
        };
        let sessions: usize = rooms.iter().map(|r| r.session_count()).sum();
        let entries: Vec<Value> = rooms.iter().map(|r| r.describe(members)).collect();
        Ok(json!({ "rooms": entries, "total": { "rooms": rooms.len(), "sessions": sessions } }))
    }

    /// A ticket for `user` into the open room `name`; refused for a full room.
    pub fn ticket(&self, owner: &str, project: &str, name: &str, user: &str, ttl: Duration) -> Result<Ticket, HubRefusal> {
        let room = self.room(owner, project, name).ok_or_else(|| HubRefusal::NotOpen(name.to_string()))?;
        if !room.has_seat() {
            return Err(HubRefusal::Full(name.to_string()));
        }
        Ok(self.inner.tickets.issue(&room_key(owner, project, name), room.instance(), user, ttl))
    }

    /// The pass a ticket gives into the open room `name`, unspent.
    pub fn read_ticket(&self, owner: &str, project: &str, name: &str, token: &str) -> Result<TicketPass, String> {
        let room = self.room(owner, project, name).ok_or_else(|| format!("room '{name}' is not open"))?;
        self.inner.tickets.read(token, &room_key(owner, project, name), room.instance())
    }

    /// Seat one session in the open room `name` as `user`, spending `pass`.
    /// Its socket learns of a server close on the returned receiver.
    pub fn join(
        &self,
        owner: &str,
        project: &str,
        name: &str,
        session_id: &str,
        user: Option<String>,
        pass: Option<&TicketPass>,
    ) -> Result<(Arc<RoomHandle>, SessionGuard, mpsc::UnboundedReceiver<Kick>), HubRefusal> {
        let key = room_key(owner, project, name);
        let rooms = self.rooms();
        let room = rooms.get(&key).cloned().ok_or_else(|| HubRefusal::NotOpen(name.to_string()))?;
        if !room.has_seat() {
            return Err(HubRefusal::Full(name.to_string()));
        }
        if let Some(pass) = pass {
            if pass.instance != room.instance() {
                return Err(HubRefusal::NotOpen(name.to_string()));
            }
            if !self.inner.tickets.spend(pass) {
                return Err(HubRefusal::TicketSpent);
            }
        }
        let (kick_tx, kick_rx) = mpsc::unbounded_channel();
        room.add_member(session_id, user, kick_tx);
        drop(rooms);
        let guard = SessionGuard {
            hub: Arc::downgrade(&self.inner),
            key,
            session_id: session_id.to_string(),
            instance: room.instance().to_string(),
        };
        Ok((room, guard, kick_rx))
    }

    /// Close every opened room that has sat empty past its `--idle` at `now`.
    pub fn sweep_idle(&self, now: Instant) -> usize {
        let mut rooms = self.rooms();
        let before = rooms.len();
        rooms.retain(|_, room| !room.idle_at(now));
        before - rooms.len()
    }

    fn ensure_sweeper(&self) {
        let Ok(runtime) = tokio::runtime::Handle::try_current() else { return };
        if self.inner.sweeping.swap(true, Ordering::AcqRel) {
            return;
        }
        let weak = Arc::downgrade(&self.inner);
        runtime.spawn(async move {
            let mut every = tokio::time::interval(IDLE_SWEEP);
            loop {
                every.tick().await;
                let Some(inner) = weak.upgrade() else { break };
                WsHub { inner }.sweep_idle(Instant::now());
            }
        });
    }

    /// Every room key the hub holds.
    pub fn list_rooms(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.rooms().keys().cloned().collect();
        keys.sort();
        keys
    }
}

#[cfg(test)]
mod tests;

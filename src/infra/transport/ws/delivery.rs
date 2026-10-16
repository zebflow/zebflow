//! How frames reach a room's sessions (`docs/contracts/rooms.md` § Clients
//! and upgrades, § Limits).
//!
//! Each session has its own queue of encoded frames, at most
//! [`SESSION_BACKLOG`] long, which its socket drains. A room never sends a
//! state: it remembers the state and its `--visible-to` rules as of its last
//! send ([`Delivery::flushed`]) and, at each send, gives every session a
//! `patch` of only what changed in **its view** — the view its tags had at
//! its last frame against the one they have now ([`super::view`]). Each
//! distinct view is cut once per send, and sessions that are sent the same
//! thing share one encoding, and so the same bytes.
//!
//! A session whose queue is full is **behind**: it is skipped from then on
//! and its socket is woken; the socket drops what is queued and asks for
//! `resync` ([`super::RoomHandle::resync`]), the whole view as of the last
//! send, from which the next `patch` follows. Events it missed are lost.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use serde_json::{Value, json};
use tokio::sync::{Notify, mpsc};

use super::room::{EmitTarget, Kick};
use super::view::{ViewKey, Visibility, diff};

/// How many frames a session may fall behind before it is sent `resync`.
pub const SESSION_BACKLOG: usize = 256;

/// One encoded frame, shared by every session it goes to.
pub type Frame = Arc<str>;

/// What a socket gets when it attaches to its seat.
pub struct Attached {
    /// The session's frames, in order.
    pub frames: mpsc::Receiver<Frame>,
    /// Woken when the session fell behind.
    pub wake: Arc<Notify>,
    /// The session's whole view: what `joined` carries.
    pub view: Value,
}

/// One session's seat in a room.
pub(super) struct Member {
    pub user: Option<String>,
    pub kick: mpsc::UnboundedSender<Kick>,
    /// Where its frames go; `None` while no socket holds the seat.
    out: Option<mpsc::Sender<Frame>>,
    behind: bool,
    wake: Arc<Notify>,
    /// Its tags (`ws.session.tag`): which `--visible-to` paths it sees.
    pub tags: BTreeSet<String>,
    /// Its tags as of the last frame it was sent: its view then.
    sent_tags: BTreeSet<String>,
}

impl Member {
    pub fn new(user: Option<String>, kick: mpsc::UnboundedSender<Kick>) -> Self {
        Self { user, kick, out: None, behind: false, wake: Arc::new(Notify::new()), tags: BTreeSet::new(), sent_tags: BTreeSet::new() }
    }

    /// Whether a socket holds the seat and is not behind.
    fn live(&self) -> bool {
        self.out.is_some() && !self.behind
    }

    /// Queue a frame; a full queue makes the session behind.
    fn send(&mut self, frame: &Frame) {
        let Some(out) = self.out.as_ref().filter(|_| !self.behind) else { return };
        match out.try_send(frame.clone()) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                self.behind = true;
                self.wake.notify_one();
            }
            Err(mpsc::error::TrySendError::Closed(_)) => self.out = None,
        }
    }

    /// Whether `target` selects this session, `id`.
    pub fn is_targeted(&self, target: &EmitTarget, id: &str) -> bool {
        match target {
            EmitTarget::All => true,
            EmitTarget::Session(s) => s == id,
            EmitTarget::Others(s) => s != id,
            EmitTarget::User(sub) => self.user.as_deref() == Some(sub.as_str()),
            EmitTarget::Tag(tag) => self.tags.contains(tag),
        }
    }
}

/// An event a tick sent, waiting for the tick's frame.
pub(super) struct TickEvent {
    body: Value,
    to: EmitTarget,
}

/// The room's sending side: the state as of the last send, and the events
/// of the tick that is running.
pub(super) struct Delivery {
    /// The state as of the last send: every session's last frame left it
    /// holding its view of this.
    flushed: Value,
    /// The `--visible-to` rules as of the last send.
    flushed_vis: Visibility,
    tick_events: Vec<TickEvent>,
}

/// `event { event, payload }`.
pub(super) fn event_frame(event: &str, payload: &Value) -> Frame {
    json!({ "type": "event", "event": event, "payload": payload }).to_string().into()
}

impl Delivery {
    pub fn new() -> Self {
        Self { flushed: json!({}), flushed_vis: Visibility::default(), tick_events: Vec::new() }
    }

    /// Forget the state (a fixed room's last session left).
    pub fn reset(&mut self) {
        self.flushed = json!({});
        self.flushed_vis = Visibility::default();
    }

    /// A session's whole view as of the last send; from it its next patch follows.
    fn whole_view(&self, member: &mut Member) -> Value {
        member.sent_tags = member.tags.clone();
        self.flushed_vis.view(&self.flushed, &self.flushed_vis.key(&member.tags))
    }

    /// Hold an event for the frame of the tick that sent it.
    pub fn hold_for_tick(&mut self, event: &str, payload: Value, to: EmitTarget) {
        self.tick_events.push(TickEvent { body: json!({ "event": event, "payload": payload }), to });
    }

    /// Whether a running tick sent events.
    pub fn has_tick_events(&self) -> bool {
        !self.tick_events.is_empty()
    }

    /// Give a socket its seat: a fresh queue and the whole view.
    pub fn attach(&mut self, member: &mut Member) -> Attached {
        let (tx, frames) = mpsc::channel(SESSION_BACKLOG);
        member.out = Some(tx);
        member.behind = false;
        Attached { frames, wake: member.wake.clone(), view: self.whole_view(member) }
    }

    /// The whole view of a session that fell behind, from which its next
    /// patch follows.
    pub fn resync(&mut self, member: &mut Member) -> Value {
        member.behind = false;
        self.whole_view(member)
    }

    /// Send every live session what changed in its view since the last
    /// send: `patch { ops }`, or in a tick room `patch { tick, ops, events }`
    /// carrying the tick's events too. A session that has nothing new is
    /// sent nothing.
    pub fn send_changes(&mut self, state: Value, vis: Visibility, members: &mut HashMap<String, Member>, tick: Option<u64>) {
        // Events wait only for the frame of the tick that sent them.
        let events = if tick.is_some() { std::mem::take(&mut self.tick_events) } else { Vec::new() };
        let (old_state, old_vis) = (std::mem::replace(&mut self.flushed, state), std::mem::replace(&mut self.flushed_vis, vis));
        let (new_state, new_vis) = (&self.flushed, &self.flushed_vis);
        // Each distinct view cut once, each change of view diffed once, each
        // distinct frame encoded once.
        let mut old_views: HashMap<ViewKey, Value> = HashMap::new();
        let mut new_views: HashMap<ViewKey, Value> = HashMap::new();
        let mut changes: HashMap<(ViewKey, ViewKey), Vec<Value>> = HashMap::new();
        let mut frames: HashMap<(ViewKey, ViewKey, Vec<usize>), Option<Frame>> = HashMap::new();
        for (id, member) in members.iter_mut().filter(|(_, m)| m.live()) {
            let was = old_vis.key(&member.sent_tags);
            let is = new_vis.key(&member.tags);
            member.sent_tags = member.tags.clone();
            let mine: Vec<usize> = events.iter().enumerate().filter(|(_, e)| member.is_targeted(&e.to, id)).map(|(i, _)| i).collect();
            let frame = frames.entry((was.clone(), is.clone(), mine)).or_insert_with_key(|(was, is, mine)| {
                let ops = changes.entry((was.clone(), is.clone())).or_insert_with(|| {
                    let before = old_views.entry(was.clone()).or_insert_with(|| old_vis.view(&old_state, was));
                    let after = new_views.entry(is.clone()).or_insert_with(|| new_vis.view(new_state, is));
                    diff(before, after)
                });
                if ops.is_empty() && mine.is_empty() {
                    return None;
                }
                Some(patch_frame(tick, ops, mine.iter().map(|i| &events[*i].body)))
            });
            if let Some(frame) = frame {
                member.send(frame);
            }
        }
    }

    /// Send an event frame to the live sessions `to` selects.
    pub fn send_event(&self, frame: &Frame, to: &EmitTarget, members: &mut HashMap<String, Member>) {
        for (id, member) in members.iter_mut() {
            if member.is_targeted(to, id) {
                member.send(frame);
            }
        }
    }
}

/// `patch { tick?, ops, events? }`: `tick` and `events` only in a tick room.
fn patch_frame<'a>(tick: Option<u64>, ops: &[Value], events: impl Iterator<Item = &'a Value>) -> Frame {
    let mut frame = json!({ "type": "patch", "ops": ops });
    if let Some(tick) = tick {
        frame["tick"] = json!(tick);
        frame["events"] = Value::Array(events.cloned().collect());
    }
    frame.to_string().into()
}

/// Detach a socket from its seat: the seat stays, nothing is sent to it.
pub(super) fn detach(member: &mut Member) {
    member.out = None;
    member.behind = false;
}

/// Whether a socket holds the seat.
pub(super) fn is_attached(member: &Member) -> bool {
    member.out.is_some()
}

/// Whether the session fell behind.
pub(super) fn is_behind(member: &Member) -> bool {
    member.behind
}

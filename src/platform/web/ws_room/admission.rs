//! Who may join a room (`docs/contracts/rooms.md` § The join rule).
//!
//! In order, each refusal closing the socket before anything of the room is
//! sent:
//!
//! 1. no active `trigger.room` declares the room → `4404`;
//! 2. the room's join rule (`--auth` / `--credential` / `--role`) refuses the
//!    visitor's headers → `4401`;
//! 3. the room is not open (a pattern nobody opened) → `4404`;
//! 4. a ticket that does not verify for this room, a `--private` room without
//!    one, or a ticket for another user than the one the rule proved → `4401`;
//! 5. the room is at its `--max-sessions` → `4429`.
//!
//! The rule is checked once, here. Its claims — or, where the rule proves no
//! identity, `{ sub }` from the ticket — ride on every event as
//! `input.room.auth`.
//!
//! A socket resuming a held seat (`?resume=`) passes the same rule, needs no
//! ticket and takes no new seat; the identity the rule proves, if any, must
//! be the held session's ([`admit_resume`]).

use super::*;
use crate::infra::transport::ws::{HubRefusal, RoomPattern, TicketPass};
use crate::platform::services::pipeline_runtime::WsTriggerSpec;

/// A refusal: the close code and its reason.
pub(super) type Refusal = (u16, &'static str);

pub(super) const CLOSE_NOT_OPEN: Refusal = (4404, "room not open");
pub(super) const CLOSE_UNAUTHORIZED: Refusal = (4401, "unauthorized");
pub(super) const CLOSE_FULL: Refusal = (4429, "room full");

/// What a socket was admitted as.
pub(super) struct Admitted {
    /// `input.room.auth`: the rule's claims, `{ sub }` from a ticket, or null.
    pub auth: Value,
    /// The identity `--recipient user:<sub>` and `ws.session.close --user` reach.
    pub user: Option<String>,
    /// The ticket to spend at join.
    pub pass: Option<TicketPass>,
    /// When the login expires (the claims' `exp`), closing with `4408`.
    pub expires_at: Option<i64>,
}

/// Whether a `trigger.room --room t_room` declares `room`.
pub(super) fn declares(t_room: &str, room: &str) -> bool {
    RoomPattern::parse(t_room).is_ok_and(|p| p.matches(room))
}

/// Whether a `trigger.room` with `t_room` / `t_event` handles `event` in `room`.
/// Reserved `$` events reach only triggers that name them.
pub(super) fn trigger_matches(t_room: &str, t_event: &str, room: &str, event: &str) -> bool {
    let event_match = t_event == event || (t_event.is_empty() && !event.starts_with('$'));
    event_match && declares(t_room, room)
}

/// What a socket resuming a held seat was admitted as.
pub(super) struct Resumed {
    pub resume: crate::infra::transport::ws::Resume,
    pub expires_at: Option<i64>,
}

/// The claims the room's join rule proves for this connection; a refusal
/// when it refuses or no active `trigger.room` declares the room.
fn rule(ctx: &SessionCtx) -> Result<Value, Refusal> {
    let pipelines = ctx.state.platform.pipeline_runtime.list_project(&ctx.owner, &ctx.project);
    let mut rules: Vec<&WsTriggerSpec> = Vec::new();
    for spec in pipelines.iter().flat_map(|p| p.ws_triggers.iter()) {
        if declares(&spec.room, &ctx.room_id) && !rules.iter().any(|r| r.join_rule() == spec.join_rule()) {
            rules.push(spec);
        }
    }
    if rules.is_empty() {
        return Err(CLOSE_NOT_OPEN);
    }
    // Activation keeps one rule per room; should two differ, both must pass.
    let mut claims = Value::Null;
    for spec in &rules {
        match verify_rule(ctx, spec) {
            Ok(Some(proved)) if claims.is_null() => claims = proved,
            Ok(_) => {}
            Err(()) => return Err(CLOSE_UNAUTHORIZED),
        }
    }
    Ok(claims)
}

/// Decide whether this connection takes back the seat `token` holds.
pub(super) fn admit_resume(ctx: &SessionCtx, token: &str) -> Result<Resumed, Refusal> {
    let claims = rule(ctx)?;
    let proved = claims.get("sub").and_then(Value::as_str);
    let hub = &ctx.state.platform.ws_hub;
    let resume = hub.resume(&ctx.owner, &ctx.project, &ctx.room_id, token, proved).map_err(|refusal| match refusal {
        HubRefusal::NotOpen(_) => CLOSE_NOT_OPEN,
        _ => CLOSE_UNAUTHORIZED,
    })?;
    Ok(Resumed { resume, expires_at: claims.get("exp").and_then(Value::as_i64) })
}

/// Decide whether this connection joins its room.
pub(super) fn admit(ctx: &SessionCtx, ticket: Option<&str>) -> Result<Admitted, Refusal> {
    let claims = rule(ctx)?;
    let hub = &ctx.state.platform.ws_hub;
    let room = hub.room(&ctx.owner, &ctx.project, &ctx.room_id).ok_or(CLOSE_NOT_OPEN)?;
    let pass = match ticket.filter(|t| !t.is_empty()) {
        Some(token) => Some(hub.read_ticket(&ctx.owner, &ctx.project, &ctx.room_id, token).map_err(|_| CLOSE_UNAUTHORIZED)?),
        None if room.settings().private => return Err(CLOSE_UNAUTHORIZED),
        None => None,
    };
    let proved_sub = claims.get("sub").and_then(Value::as_str).map(str::to_string);
    let user = match (&proved_sub, &pass) {
        (Some(sub), Some(pass)) if *sub != pass.user => return Err(CLOSE_UNAUTHORIZED),
        (Some(sub), _) => Some(sub.clone()),
        (None, Some(pass)) => Some(pass.user.clone()),
        (None, None) => None,
    };
    let auth = match (&proved_sub, &user) {
        (None, Some(sub)) if claims.is_null() => json!({ "sub": sub }),
        _ => claims.clone(),
    };
    let expires_at = claims.get("exp").and_then(Value::as_i64);
    Ok(Admitted { auth, user, pass, expires_at })
}

/// The close a hub refusal at join becomes.
pub(super) fn join_refusal(refusal: &HubRefusal) -> Refusal {
    match refusal {
        HubRefusal::Full(_) => CLOSE_FULL,
        HubRefusal::TicketSpent => CLOSE_UNAUTHORIZED,
        _ => CLOSE_NOT_OPEN,
    }
}

/// The rule's own check, from the visitor's headers, on every office: a
/// connection a controller forwards carries the visitor's headers, and the
/// controller's call header proves only who forwarded it.
fn verify_rule(ctx: &SessionCtx, spec: &WsTriggerSpec) -> Result<Option<Value>, ()> {
    if spec.join_rule().0 == "none" {
        return Ok(None);
    }
    verify_webhook_auth(
        &ctx.headers,
        &axum::body::Bytes::new(), // WS messages have no body to sign
        &spec.auth_type,
        &spec.auth_credential,
        &spec.auth_required_role,
        &ctx.state.platform.credentials,
        &ctx.owner,
        &ctx.project,
    )
    .map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_events_reach_only_triggers_that_name_them() {
        assert!(trigger_matches("lobby", "$disconnect", "lobby", "$disconnect"));
        assert!(!trigger_matches("lobby", "", "lobby", "$disconnect"), "a catch-all event trigger must not see lifecycle events");
        assert!(trigger_matches("lobby", "", "lobby", "move"));
        assert!(!trigger_matches("arena", "move", "lobby", "move"));
        assert!(trigger_matches("match-*", "move", "match-42", "move"));
        assert!(!trigger_matches("match-*", "move", "match-", "move"));
        assert!(trigger_matches("thread-:id", "chat", "thread-7", "chat"));
        assert!(!trigger_matches("*", "chat", "thread-7", "chat"), "`*` alone declares no room");
    }
}

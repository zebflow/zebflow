//! The hub's rules: which rooms exist, who may join, and how rooms close.

use super::*;

const O: &str = "demo";
const P: &str = "site-a";

fn hub_declaring(rooms: &[&str]) -> WsHub {
    let hub = WsHub::new();
    let declared: Vec<(String, String)> = rooms.iter().map(|r| (r.to_string(), "pipelines/rooms".to_string())).collect();
    hub.declare(O, P, &declared);
    hub
}

fn opened(max_sessions: Option<u32>) -> RoomSettings {
    RoomSettings { max_sessions, ..RoomSettings::fixed("pipelines/open") }
}

#[test]
fn a_fixed_room_exists_while_declared_and_closes_when_not() {
    let hub = hub_declaring(&["lobby", "match-:id"]);
    assert_eq!(hub.list_rooms(), vec!["demo/site-a/lobby".to_string()], "a pattern declares no room by itself");
    let (_room, _guard, mut kicks) = hub.join(O, P, "lobby", "s1", None, None).expect("a fixed room is open");
    hub.declare(O, P, &[("match-:id".into(), "pipelines/rooms".into())]);
    assert!(hub.list_rooms().is_empty(), "no active trigger names lobby any more");
    assert_eq!(kicks.try_recv().unwrap(), KICK_ROOM_CLOSED);
}

#[test]
fn a_join_never_creates_a_room() {
    let hub = hub_declaring(&["match-:id"]);
    assert_eq!(hub.join(O, P, "match-1", "s1", None, None).err(), Some(HubRefusal::NotOpen("match-1".into())));
    assert!(hub.list_rooms().is_empty());
    hub.open(O, P, "match-1", opened(None)).expect("open");
    assert!(hub.join(O, P, "match-1", "s1", None, None).is_ok(), "an opened room admits");
}

#[test]
fn open_needs_a_pattern_and_refuses_fixed_taken_and_bad_names() {
    let hub = hub_declaring(&["lobby", "match-:id"]);
    assert_eq!(hub.open(O, P, "zone-1", opened(None)).err(), Some(HubRefusal::Undeclared("zone-1".into())));
    assert_eq!(hub.open(O, P, "lobby", opened(None)).err(), Some(HubRefusal::Fixed("lobby".into())));
    assert!(hub.open(O, P, "match-1", opened(None)).is_ok());
    assert_eq!(hub.open(O, P, "match-1", opened(None)).err(), Some(HubRefusal::Taken("match-1".into())));
    assert!(matches!(hub.open(O, P, "match-*", opened(None)), Err(HubRefusal::BadName(_))));
    assert!(matches!(hub.open("demo", "site-b", "match-2", opened(None)), Err(HubRefusal::Undeclared(_))), "another project's pattern");
}

#[test]
fn max_sessions_caps_joins_and_tickets() {
    let hub = hub_declaring(&["match-:id"]);
    hub.open(O, P, "match-1", opened(Some(1))).unwrap();
    let seat = hub.join(O, P, "match-1", "s1", None, None).expect("first seat");
    assert_eq!(hub.join(O, P, "match-1", "s2", None, None).err(), Some(HubRefusal::Full("match-1".into())));
    assert_eq!(hub.ticket(O, P, "match-1", "u-17", ticket::DEFAULT_TTL).err(), Some(HubRefusal::Full("match-1".into())));
    drop(seat);
    assert!(hub.ticket(O, P, "match-1", "u-17", ticket::DEFAULT_TTL).is_ok(), "a seat is free again");
    assert!(hub.join(O, P, "match-1", "s2", None, None).is_ok());
}

#[test]
fn a_ticket_is_spent_by_the_join_it_admits() {
    let hub = hub_declaring(&["match-:id"]);
    hub.open(O, P, "match-1", opened(None)).unwrap();
    assert_eq!(hub.ticket(O, P, "match-2", "u-17", ticket::DEFAULT_TTL).err(), Some(HubRefusal::NotOpen("match-2".into())));
    let ticket = hub.ticket(O, P, "match-1", "u-17", ticket::DEFAULT_TTL).unwrap();
    let pass = hub.read_ticket(O, P, "match-1", &ticket.token).expect("valid");
    let _seat = hub.join(O, P, "match-1", "s1", Some(pass.user.clone()), Some(&pass)).expect("admitted");
    assert_eq!(hub.join(O, P, "match-1", "s2", None, Some(&pass)).err(), Some(HubRefusal::TicketSpent));
    assert!(hub.read_ticket(O, P, "match-1", &ticket.token).is_err());
}

#[test]
fn close_kicks_drops_state_and_answers_each_room() {
    let hub = hub_declaring(&["lobby", "match-:id"]);
    hub.open(O, P, "match-1", opened(None)).unwrap();
    hub.open(O, P, "match-2", opened(None)).unwrap();
    let (_r, _g, mut kicked) = hub.join(O, P, "match-1", "s1", None, None).unwrap();
    assert_eq!(hub.close(O, P, "match-9").err(), Some(HubRefusal::NotOpen("match-9".into())));
    let closed = hub.close(O, P, "match-*").unwrap();
    assert_eq!(closed, vec![("match-1".to_string(), 1), ("match-2".to_string(), 0)]);
    assert_eq!(kicked.try_recv().unwrap(), KICK_ROOM_CLOSED);
    assert_eq!(hub.list_rooms(), vec!["demo/site-a/lobby".to_string()]);
    assert!(hub.close(O, P, "match-*").unwrap().is_empty(), "a pattern reaching nothing is no error");

    // A fixed room closes and is back at once, empty.
    let lobby = hub.room(O, P, "lobby").unwrap();
    let (_r, _g, _k) = hub.join(O, P, "lobby", "s2", None, None).unwrap();
    lobby.apply(StateOp::Set, "/a", Some(json!(1)), false).unwrap();
    assert_eq!(hub.close(O, P, "lobby").unwrap(), vec![("lobby".to_string(), 1)]);
    let fresh = hub.room(O, P, "lobby").expect("still declared");
    assert!(!Arc::ptr_eq(&lobby, &fresh));
    assert_eq!(fresh.get_state(), json!({}));
}

#[test]
fn session_close_reaches_a_session_or_every_session_of_a_user() {
    let hub = hub_declaring(&["lobby"]);
    let (_r1, _g1, mut a) = hub.join(O, P, "lobby", "s1", Some("u-17".into()), None).unwrap();
    let (_r2, _g2, mut b) = hub.join(O, P, "lobby", "s2", Some("u-17".into()), None).unwrap();
    let (_r3, _g3, mut c) = hub.join(O, P, "lobby", "s3", Some("u-18".into()), None).unwrap();
    assert_eq!(hub.close_sessions(O, P, "lobby", &EmitTarget::User("u-17".into())).unwrap(), (1, 2));
    assert_eq!((a.try_recv().ok(), b.try_recv().ok(), c.try_recv().ok()), (Some(KICK_CLOSED), Some(KICK_CLOSED), None));
    assert_eq!(hub.close_sessions(O, P, "lobby", &EmitTarget::Session("s3".into())).unwrap(), (1, 1));
    assert_eq!(c.try_recv().ok(), Some(KICK_CLOSED));
}

#[test]
fn an_opened_room_closes_after_sitting_empty_for_its_idle() {
    let hub = hub_declaring(&["match-:id"]);
    hub.open(O, P, "match-1", RoomSettings { idle: Duration::from_secs(5), ..opened(None) }).unwrap();
    let seat = hub.join(O, P, "match-1", "s1", None, None).unwrap();
    let later = Instant::now() + Duration::from_secs(10);
    assert_eq!(hub.sweep_idle(later), 0, "a room with a session is not idle");
    drop(seat);
    assert_eq!(hub.sweep_idle(Instant::now()), 0, "not empty long enough");
    assert_eq!(hub.sweep_idle(Instant::now() + Duration::from_secs(6)), 1);
    assert!(hub.list_rooms().is_empty());
}

#[test]
fn the_last_leave_keeps_an_opened_room_and_its_state() {
    let hub = hub_declaring(&["match-:id"]);
    let room = hub.open(O, P, "match-1", opened(None)).unwrap();
    let seat = hub.join(O, P, "match-1", "s1", None, None).unwrap();
    room.apply(StateOp::Set, "/score", Some(json!(3)), false).unwrap();
    drop(seat);
    assert_eq!(hub.room(O, P, "match-1").unwrap().get_state(), json!({ "score": 3 }), "an opened room keeps state until it closes");
}

#[test]
fn list_answers_the_contracts_shape_with_members_on_request() {
    let hub = hub_declaring(&["lobby", "match-:id"]);
    hub.open(O, P, "match-1", RoomSettings { meta: json!({ "mode": "ranked" }), ..opened(Some(4)) }).unwrap();
    let (_r, _g, _k) = hub.join(O, P, "match-1", "s1", Some("u-17".into()), None).unwrap();
    let all = hub.list(O, P, "*", false).unwrap();
    assert_eq!(all["total"], json!({ "rooms": 2, "sessions": 1 }));
    let entry = &all["rooms"][1];
    assert_eq!(entry["room"], "match-1");
    assert_eq!(entry["kind"], "opened");
    assert_eq!(entry["max_sessions"], 4);
    assert_eq!(entry["meta"], json!({ "mode": "ranked" }));
    assert!(entry.get("members").is_none(), "no identities without --members");
    for counter in ["messages_in", "messages_out", "ticks", "tick_overruns", "inputs_dropped", "pipeline_runs", "pipeline_errors", "resyncs", "flood_closes"] {
        assert_eq!(entry["counters"][counter], 0, "{counter}");
    }
    let some = hub.list(O, P, "match-*", true).unwrap();
    assert_eq!(some["total"]["rooms"], 1);
    assert_eq!(some["rooms"][0]["members"], json!(["u-17"]));
    assert_eq!(hub.list(O, P, "match-9", false).unwrap()["total"]["rooms"], 0, "an exact room not open lists nothing");
}

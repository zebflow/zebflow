//! How a room runs its pipelines (`docs/contracts/rooms.md` § Events and
//! ticks, § The fast lane): one thing at a time per room, the pipelines
//! resolved again when the active set changes, tick rooms, and room work on
//! threads of its own that write no run record.

use futures::SinkExt;
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;

use super::published_mcp::{OWNER, PROJECT, app_at, dsl, login, publish, send, temp_dir};
use super::ws_rooms::{Socket, connect, hook, next_of, serve};

async fn say(socket: &mut Socket, event: &str, payload: Value) {
    socket.send(Message::Text(json!({ "event": event, "payload": payload }).to_string().into())).await.expect("send");
}

/// One room as `GET …/ws/rooms` lists it.
async fn listed(app: &axum::Router, cookie: &str, room: &str) -> Value {
    let (status, list) = send(app, cookie, "GET", &format!("/api/projects/{OWNER}/{PROJECT}/ws/rooms?room={room}"), Value::Null).await;
    assert_eq!(status.as_u16(), 200, "{list}");
    list["rooms"][0].clone()
}

/// The next frame of event `name`, failing on any other event first.
async fn next_named(socket: &mut Socket, name: &str) -> Value {
    let frame = next_of(socket, "event").await;
    assert_eq!(frame["event"], name, "{frame}");
    frame["payload"].clone()
}

/// The next `event` frame's name.
async fn next_event(socket: &mut Socket) -> String {
    next_of(socket, "event").await["event"].as_str().unwrap_or_default().to_string()
}

/// Send ten bids as `who` and answer every list the room sent back.
async fn bid_ten(mut socket: Socket, who: &str) -> Vec<Value> {
    for i in 0..10 {
        say(&mut socket, "bid", json!({ "n": format!("{who}{i}") })).await;
    }
    let mut lists = Vec::new();
    for _ in 0..10 {
        lists.push(next_of(&mut socket, "event").await["payload"].clone());
    }
    lists
}

/// Two sessions bid at once into one room. Each bid reads the list, waits,
/// and writes it back: run concurrently, one bid would overwrite the other.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_sessions_read_modify_write_never_interleave() {
    let root = temp_dir("ws-room-order");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    publish(
        &app,
        &cookie,
        "auction-bid",
        r#"| trigger.room --room auction --event bid | kv.entry.get --key auction:bids | javascript.script.run -- "const list = (input.entry && input.entry.value) || []; const until = Date.now() + 15; while (Date.now() < until) {} return list.concat([input.room.payload.n]);" | kv.entry.put --key auction:bids --value "{{ input.script }}" | ws.message.send --event bid.done --recipient session --body "{{ input.script }}""#,
    )
    .await;
    let address = serve(app, &cookie).await;
    let mut a = connect(address, "auction", &[]).await;
    let mut b = connect(address, "auction", &[]).await;
    next_of(&mut a, "joined").await;
    next_of(&mut b, "joined").await;

    let (from_a, from_b) = tokio::join!(bid_ten(a, "a"), bid_ten(b, "b"));
    let longest = from_a.iter().chain(&from_b).max_by_key(|list| list.as_array().map_or(0, Vec::len)).expect("lists");
    let bids: Vec<&str> = longest.as_array().expect("a list").iter().filter_map(Value::as_str).collect();
    assert_eq!(bids.len(), 20, "no bid lost to another session's write: {bids:?}");
    for who in ["a", "b"] {
        let own: Vec<&str> = bids.iter().copied().filter(|n| n.starts_with(who)).collect();
        let sent: Vec<String> = (0..10).map(|i| format!("{who}{i}")).collect();
        assert_eq!(own, sent, "each session's bids keep their order");
    }
}

#[tokio::test]
async fn activation_and_deactivation_change_what_an_open_room_runs() {
    let root = temp_dir("ws-room-resolve");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    publish(&app, &cookie, "ping-a", r#"| trigger.room --room lobby --event ping | ws.message.send --event a --recipient session --body 1"#).await;
    let address = serve(app.clone(), &cookie).await;
    let mut socket = connect(address, "lobby", &[]).await;
    next_of(&mut socket, "joined").await;
    say(&mut socket, "ping", json!({})).await;
    assert_eq!(next_event(&mut socket).await, "a");

    publish(&app, &cookie, "ping-b", r#"| trigger.room --room lobby --event ping | ws.message.send --event b --recipient session --body 1"#).await;
    say(&mut socket, "ping", json!({})).await;
    assert_eq!([next_event(&mut socket).await, next_event(&mut socket).await], ["a", "b"], "an activation reaches a room already running");

    let answer = dsl(&app, &cookie, "deactivate pipeline pipelines/tests/ping-a.zf.json").await;
    assert_eq!(answer["ok"], json!(true), "{answer}");
    say(&mut socket, "ping", json!({})).await;
    say(&mut socket, "ping", json!({})).await;
    assert_eq!([next_event(&mut socket).await, next_event(&mut socket).await], ["b", "b"], "a deactivated pipeline runs no more");
}

#[tokio::test]
async fn a_tick_gets_every_input_of_its_interval_in_arrival_order_up_to_max_inputs_before_a_disconnect() {
    let root = temp_dir("ws-room-tick");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    publish(&app, &cookie, "arena-tick", r#"| trigger.room --room arena --event $tick --every 1s --max-inputs 2 | ws.message.send --event tick --body "{{ input.room }}""#).await;
    publish(&app, &cookie, "arena-move", r#"| trigger.room --room arena --event move | ws.message.send --event moved --body 1"#).await;
    publish(&app, &cookie, "arena-gone", r#"| trigger.room --room arena --event $disconnect | ws.message.send --event gone --body "{{ input.room.session_id }}""#).await;
    let address = serve(app.clone(), &cookie).await;
    let mut a = connect(address, "arena", &[]).await;
    let mut b = connect(address, "arena", &[]).await;
    let a_id = next_of(&mut a, "joined").await["session_id"].clone();
    let b_id = next_of(&mut b, "joined").await["session_id"].clone();

    let first = next_named(&mut a, "tick").await;
    assert_eq!(first["inputs"], json!([]), "an empty tick still runs: {first}");
    assert_eq!((first["room_id"].as_str(), first["event"].as_str()), (Some("arena"), Some("$tick")));
    for i in 1..=3 {
        say(&mut a, "move", json!({ "i": i })).await;
    }
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    say(&mut b, "move", json!({ "i": 9 })).await;
    b.close(None).await.expect("close");

    let second = next_named(&mut a, "tick").await;
    let inputs = |who: &Value, i: u64| json!({ "session": who, "event": "move", "payload": { "i": i }, "auth": null });
    assert_eq!(second["inputs"], json!([inputs(&a_id, 1), inputs(&a_id, 2), inputs(&b_id, 9)]), "in arrival order; a's third is past --max-inputs 2");
    assert_eq!(second["tick"].as_u64(), first["tick"].as_u64().map(|t| t + 1));
    let dt = second["dt_ms"].as_u64().expect("dt_ms");
    assert!((900..1500).contains(&dt), "dt_ms {dt}");
    assert_eq!(next_named(&mut a, "gone").await, b_id, "b's $disconnect waited for the tick that took b's last move");

    let room = listed(&app, &cookie, "arena").await;
    assert_eq!(room["mode"], "tick", "{room}");
    assert_eq!(room["counters"]["inputs_dropped"], 1, "{room}");
    assert!(room["counters"]["ticks"].as_u64() >= Some(2), "{room}");
}

/// The last player leaves right after a move: the room's loop must not end
/// before the tick takes that move and the `$disconnect` runs after it, or
/// the player stays in the room's state for good.
#[tokio::test]
async fn the_last_session_s_move_gets_its_tick_and_its_disconnect_runs_after() {
    let root = temp_dir("ws-room-last-tick");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    let append = |entry: &str| format!(r#"kv.entry.get --key den:log | javascript.script.run -- "const list = (input.entry && input.entry.value) || []; {entry}" | kv.entry.put --key den:log --value "{{{{ input.script }}}}""#);
    publish(&app, &cookie, "den-tick", &format!("| trigger.room --room den --event $tick --every 200ms | {}", append("return input.room.inputs.length ? list.concat(['tick:' + input.room.inputs.length]) : list;"))).await;
    publish(&app, &cookie, "den-gone", &format!("| trigger.room --room den --event $disconnect | {}", append("return list.concat(['gone']);"))).await;
    publish(&app, &cookie, "den-log", r#"| trigger.webhook --route /den-log --method POST | kv.entry.get --key den:log | web.response.send --body "{{ input.entry.value }}""#).await;
    let address = serve(app.clone(), &cookie).await;
    let mut socket = connect(address, "den", &[]).await;
    next_of(&mut socket, "joined").await;
    say(&mut socket, "move", json!({ "x": 1 })).await;
    socket.close(None).await.expect("close");

    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    assert_eq!(hook(&app, "/den-log", json!({})).await, json!(["tick:1", "gone"]), "the move had its tick, then the $disconnect ran");
}

#[tokio::test]
async fn a_tick_that_outlasts_every_delays_the_next_and_counts_an_overrun() {
    let root = temp_dir("ws-room-overrun");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    publish(
        &app,
        &cookie,
        "forge-tick",
        r#"| trigger.room --room forge --event $tick --every 50ms | javascript.script.run -- "const s = Date.now(); let w = 0; while (Date.now() < s + 120) { w += new Array(200000).fill(7).join('').length; } return { s, e: Date.now() };" | ws.message.send --event tick --body "{{ { tick: input.room.tick, s: input.script.s, e: input.script.e } }}""#,
    )
    .await;
    let address = serve(app.clone(), &cookie).await;
    let mut socket = connect(address, "forge", &[]).await;
    next_of(&mut socket, "joined").await;
    let mut ticks = Vec::new();
    for _ in 0..4 {
        ticks.push(next_named(&mut socket, "tick").await);
    }
    for pair in ticks.windows(2) {
        assert_eq!(pair[1]["tick"].as_u64(), pair[0]["tick"].as_u64().map(|t| t + 1), "{pair:?}");
        assert!(pair[1]["s"].as_i64() >= pair[0]["e"].as_i64(), "a tick starts after the last one ended: {pair:?}");
    }
    let counters = listed(&app, &cookie, "forge").await["counters"].clone();
    assert!(counters["tick_overruns"].as_u64() >= Some(3), "{counters}");
}

#[tokio::test]
async fn a_tick_trigger_turns_an_open_room_into_a_tick_room_and_back() {
    let root = temp_dir("ws-room-mode");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    publish(&app, &cookie, "hall-chat", r#"| trigger.room --room hall --event chat | ws.message.send --event said --recipient session --body 1"#).await;
    let address = serve(app.clone(), &cookie).await;
    let mut socket = connect(address, "hall", &[]).await;
    next_of(&mut socket, "joined").await;
    assert_eq!(listed(&app, &cookie, "hall").await["mode"], "event");

    publish(&app, &cookie, "hall-tick", r#"| trigger.room --room hall --event $tick --every 100ms | ws.message.send --event tick --body "{{ input.room.inputs }}""#).await;
    assert_eq!(listed(&app, &cookie, "hall").await["mode"], "tick");
    say(&mut socket, "chat", json!({ "text": "hi" })).await;
    let inputs = loop {
        let inputs = next_named(&mut socket, "tick").await;
        if inputs != json!([]) {
            break inputs;
        }
    };
    assert_eq!(inputs[0]["payload"], json!({ "text": "hi" }), "the chat waited for the tick, its own trigger did not run");

    let answer = dsl(&app, &cookie, "deactivate pipeline pipelines/tests/hall-tick.zf.json").await;
    assert_eq!(answer["ok"], json!(true), "{answer}");
    assert_eq!(listed(&app, &cookie, "hall").await["mode"], "event");
    say(&mut socket, "chat", json!({ "text": "again" })).await;
    while next_event(&mut socket).await != "said" {}
}

#[tokio::test]
async fn activation_refuses_two_ticks_for_one_room_and_every_without_a_tick() {
    let root = temp_dir("ws-room-tick-rules");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    publish(&app, &cookie, "arenas", r#"| trigger.room --room "arena-*" --event $tick --every 50ms | ws.message.send --body 1"#).await;
    publish(&app, &cookie, "arenas-too", r#"| trigger.room --room "arena-:id" --event $tick --every 50ms | ws.message.send --body 2"#).await;
    for (name, body, why) in [
        ("arena-slow", r#"| trigger.room --room arena-1 --event $tick --every 100ms | ws.message.send --body 1"#, "one tick"),
        ("lobby-every", r#"| trigger.room --room lobby --event move --every 50ms | ws.message.send --body 1"#, "belong to --event $tick"),
        ("lobby-tick", r#"| trigger.room --room lobby --event $tick | ws.message.send --body 1"#, "needs --every"),
    ] {
        let registered = dsl(&app, &cookie, &format!("register pipelines/tests/{name} -- {body}")).await;
        let activated = dsl(&app, &cookie, &format!("activate pipeline pipelines/tests/{name}.zf.json")).await;
        assert_ne!(activated["ok"], json!(true), "{name}: {activated}");
        let text = format!("{registered} {activated}");
        assert!(text.contains(why), "{name}: {text}");
    }
}

/// The room's `ticks` counter now.
async fn ticks(app: &axum::Router, cookie: &str, room: &str) -> u64 {
    listed(app, cookie, room).await["counters"]["ticks"].as_u64().expect("ticks")
}

/// One thread serves HTTP here, so a webhook that keeps it busy stops
/// everything on it: the room must not be on it.
#[tokio::test]
async fn a_slow_webhook_does_not_delay_a_tick() {
    let root = temp_dir("ws-room-lane");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    publish(&app, &cookie, "clock-tick", r#"| trigger.room --room clock --event $tick --every 20ms | ws.state.put --key /tick --value "{{ input.room.tick }}""#).await;
    publish(
        &app,
        &cookie,
        "slow",
        r#"| trigger.webhook --route /slow --method POST | javascript.script.run -- "const s = Date.now(); let w = 0; while (Date.now() < s + 600) { w += new Array(200000).fill(7).join('').length; } return Date.now() - s;" | web.response.send --body "{{ input.script }}""#,
    )
    .await;
    let address = serve(app.clone(), &cookie).await;
    let mut socket = connect(address, "clock", &[]).await;
    next_of(&mut socket, "joined").await;
    next_of(&mut socket, "state_patch").await;

    let before = ticks(&app, &cookie, "clock").await;
    let started = std::time::Instant::now();
    let held = hook(&app, "/slow", json!({})).await;
    let elapsed = started.elapsed().as_millis() as u64;
    let after = ticks(&app, &cookie, "clock").await;
    assert!(held.as_u64() >= Some(500), "the webhook held its thread: {held}");
    // About 30 at 20 ms; none at all when the room shares the HTTP thread.
    assert!(after - before >= 5, "{} ticks in {elapsed} ms of a busy HTTP thread", after - before);
}

/// `GET …/pipelines/invocations` for one pipeline.
async fn records(app: &axum::Router, cookie: &str, name: &str) -> Value {
    let uri = format!("/api/projects/{OWNER}/{PROJECT}/pipelines/invocations?pipeline=pipelines/tests/{name}.zf.json");
    let (status, answer) = send(app, cookie, "GET", &uri, Value::Null).await;
    assert_eq!(status.as_u16(), 200, "{answer}");
    answer
}

#[tokio::test]
async fn room_runs_write_no_run_record_and_a_failure_is_recorded_once_a_minute() {
    let root = temp_dir("ws-room-records");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    publish(&app, &cookie, "room-ok", r#"| trigger.room --room lobby --event ok | ws.message.send --event fine --recipient session --body 1"#).await;
    publish(&app, &cookie, "room-fail", r#"| trigger.room --room lobby --event boom | javascript.script.run -- "throw new Error('no luck');""#).await;
    let address = serve(app.clone(), &cookie).await;
    let mut socket = connect(address, "lobby", &[]).await;
    next_of(&mut socket, "joined").await;
    for event in ["ok", "boom", "boom", "boom", "ok"] {
        say(&mut socket, event, json!({})).await;
    }
    // One at a time, in order: the second answer comes after every boom ran.
    next_named(&mut socket, "fine").await;
    next_named(&mut socket, "fine").await;

    assert_eq!(records(&app, &cookie, "room-ok").await["count"], 0, "a room run writes no run record");
    let failed = records(&app, &cookie, "room-fail").await;
    assert_eq!(failed["count"], 1, "three failures within a minute, one record: {failed}");
    assert_eq!((failed["entries"][0]["status"].as_str(), failed["entries"][0]["trigger"].as_str()), (Some("error"), Some("room")));
    let counters = listed(&app, &cookie, "lobby").await["counters"].clone();
    assert_eq!((counters["pipeline_runs"].as_u64(), counters["pipeline_errors"].as_u64()), (Some(5), Some(3)), "{counters}");
}

//! End-to-end cost of a webhook request and a room message, each against a
//! floor: plain axum doing the same work with no pipeline. Ignored
//! diagnostics, not timing assertions (the 100k study in the ledger). Run in
//! a release build:
//! `cargo test --release --test platform perf_ -- --ignored --nocapture --test-threads 1`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::{Body, to_bytes};
use axum::extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade};
use axum::http::Request;
use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;
use tower::ServiceExt;

use super::published_mcp::{OWNER, PROJECT, app_at, login, publish, send, temp_dir};
use super::ws_rooms::{Socket, connect, next_of, serve};

/// Webhook requests in flight at once.
const IN_FLIGHT: usize = 32;
const REQUESTS: usize = 4000;

/// `REQUESTS` POSTs of a small JSON body, `IN_FLIGHT` at a time, straight
/// into the router (no TCP): requests per second and p50 / p99.
async fn hammer(app: &axum::Router, uri: &str) {
    let latencies = Arc::new(Mutex::new(Vec::with_capacity(REQUESTS)));
    let start = Instant::now();
    let tasks: Vec<_> = (0..IN_FLIGHT)
        .map(|t| {
            let (app, uri, latencies) = (app.clone(), uri.to_string(), latencies.clone());
            tokio::spawn(async move {
                for n in 0..REQUESTS / IN_FLIGHT {
                    let body = json!({ "id": t * REQUESTS + n, "lat": -6.2, "lng": 106.8 }).to_string();
                    let request = Request::builder().uri(&uri).method("POST").header("content-type", "application/json").body(Body::from(body)).expect("request");
                    let began = Instant::now();
                    let response = app.clone().oneshot(request).await.expect("response");
                    let status = response.status();
                    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body");
                    assert!(status.is_success(), "{uri}: {status} {}", String::from_utf8_lossy(&bytes));
                    latencies.lock().unwrap().push(began.elapsed());
                }
            })
        })
        .collect();
    for task in tasks {
        task.await.expect("task");
    }
    let took = start.elapsed();
    let mut l = latencies.lock().unwrap().clone();
    l.sort();
    let at = |q: f64| l[((l.len() as f64 * q) as usize).min(l.len() - 1)].as_secs_f64() * 1e3;
    eprintln!(
        "perf-wh {uri:<40} {:>9.0} req/s  p50 {:>6.2} ms  p99 {:>6.2} ms  ({} in flight, debug_assertions={})",
        REQUESTS as f64 / took.as_secs_f64(),
        at(0.5),
        at(0.99),
        IN_FLIGHT,
        cfg!(debug_assertions)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore = "manual webhook cost measurement"]
async fn perf_webhook_against_plain_axum() {
    let map = Arc::new(Mutex::new(HashMap::<u64, Value>::new()));
    let floor = axum::Router::new().route(
        "/floor",
        axum::routing::post(move |axum::Json(body): axum::Json<Value>| {
            let map = map.clone();
            async move {
                let id = body["id"].as_u64().unwrap_or_default();
                map.lock().unwrap().insert(id, body.clone());
                axum::Json(json!({ "x": id * 2 }))
            }
        }),
    );
    hammer(&floor, "/floor").await;

    let root = temp_dir("perf-webhook");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    publish(&app, &cookie, "wh-one", r#"| trigger.webhook --route /bench/one --method POST | web.response.send --body "{{ input.webhook.body.id }}""#).await;
    publish(
        &app,
        &cookie,
        "wh-three",
        r#"| trigger.webhook --route /bench/three --method POST | javascript.script.run -- "return { x: input.webhook.body.id * 2 };" | kv.entry.put --key "/a/{{ input.webhook.body.id }}" --value "{{ input.webhook.body }}" | web.response.send --body "{{ input.script }}""#,
    )
    .await;
    for route in ["/bench/one", "/bench/one", "/bench/three"] {
        hammer(&app, &format!("/wh/{OWNER}/{PROJECT}{route}")).await;
    }
}

/// Room sessions and the messages each sends.
const SESSIONS: usize = 100;
const EACH: usize = 100;

/// `SESSIONS` sockets each send `EACH` messages as fast as they can; the
/// time until `processed` reports all of them.
async fn flood(address: std::net::SocketAddr, room: &str, processed: impl Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = u64> + Send>>) {
    let mut sockets: Vec<Socket> = Vec::new();
    for _ in 0..SESSIONS {
        let mut socket = connect(address, room, &[]).await;
        next_of(&mut socket, "joined").await;
        sockets.push(socket);
    }
    let total = (SESSIONS * EACH) as u64;
    let base = processed().await;
    let start = Instant::now();
    let senders: Vec<_> = sockets
        .into_iter()
        .enumerate()
        .map(|(s, mut socket)| {
            tokio::spawn(async move {
                for n in 0..EACH {
                    let text = json!({ "event": "gps", "payload": { "id": s * EACH + n, "lat": -6.2, "lng": 106.8 } }).to_string();
                    socket.send(Message::Text(text.into())).await.expect("send");
                }
                // Keep reading so the server never sees a stalled session.
                while let Ok(Some(_)) = tokio::time::timeout(Duration::from_secs(30), socket.next()).await {}
            })
        })
        .collect();
    loop {
        let done = processed().await - base;
        if done >= total {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(120), "{room}: only {done} of {total} processed");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let took = start.elapsed();
    eprintln!(
        "perf-ws {room:<34} {:>9.0} msg/s  ({SESSIONS} sessions × {EACH}, {:.0} ms, debug_assertions={})",
        total as f64 / took.as_secs_f64(),
        took.as_secs_f64() * 1e3,
        cfg!(debug_assertions)
    );
    for sender in senders {
        sender.abort();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore = "manual room cost measurement"]
async fn perf_room_against_plain_axum_socket() {
    // Floor: parse each message and keep the latest per id, nothing else.
    let count = Arc::new(AtomicU64::new(0));
    let latest = Arc::new(Mutex::new(HashMap::<u64, Value>::new()));
    let (c, m) = (count.clone(), latest.clone());
    let floor = axum::Router::new().route(
        "/ws/{owner}/{project}/rooms/{room}",
        axum::routing::get(move |upgrade: WebSocketUpgrade| {
            let (count, latest) = (c.clone(), m.clone());
            async move {
                upgrade.protocols(["zebflow.rooms.v1"]).on_upgrade(move |mut socket: WebSocket| async move {
                    let _ = socket.send(AxumMessage::Text(json!({ "type": "joined", "state": {} }).to_string().into())).await;
                    while let Some(Ok(AxumMessage::Text(text))) = socket.recv().await {
                        let v: Value = serde_json::from_str(text.as_str()).expect("json");
                        let id = v["payload"]["id"].as_u64().unwrap_or_default();
                        latest.lock().unwrap().insert(id, v["payload"].clone());
                        count.fetch_add(1, Ordering::Relaxed);
                    }
                })
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let floor_address = listener.local_addr().expect("address");
    tokio::spawn(async move { axum::serve(listener, floor).await });
    flood(floor_address, "floor", || {
        let count = count.clone();
        Box::pin(async move { count.load(Ordering::Relaxed) })
    })
    .await;

    let root = temp_dir("perf-room");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    // What the room keeps is visible to nobody, so this measures the run, not the fanout.
    publish(&app, &cookie, "ev-room", r#"| trigger.room --room ev --event gps | ws.state.put --key "/pos/{{ input.room.payload.id }}" --value "{{ input.room.payload }}" --visible-to ops"#).await;
    // The same run on a state that stays at 100 keys: what the run costs
    // when the state is small.
    publish(&app, &cookie, "ev-small", r#"| trigger.room --room ev-small --event gps | ws.state.put --key "/pos/{{ input.room.payload.id % 100 }}" --value "{{ input.room.payload }}" --visible-to ops"#).await;
    publish(
        &app,
        &cookie,
        "tick-room",
        r#"| trigger.room --room tk --event $tick --every 50ms --max-inputs 256 --state /n | javascript.script.run -- "const pos = {}; for (const i of input.room.inputs) pos['d' + i.payload.id] = i.payload; return { pos, n: (input.room.state.n || 0) + input.room.inputs.length };" | ws.state.update --key /pos --value "{{ input.script.pos }}" --visible-to ops | ws.state.put --key /n --value "{{ input.script.n }}""#,
    )
    .await;
    let (app2, cookie2) = (app.clone(), cookie.clone());
    let address = serve(app, &cookie).await;
    let counter = move |room: &'static str, name: &'static str| {
        let (app, cookie) = (app2.clone(), cookie2.clone());
        move || {
            let (app, cookie) = (app.clone(), cookie.clone());
            Box::pin(async move {
                let (_, list) = send(&app, &cookie, "GET", &format!("/api/projects/{OWNER}/{PROJECT}/ws/rooms?room={room}"), Value::Null).await;
                list["rooms"][0]["counters"][name].as_u64().unwrap_or_default()
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = u64> + Send>>
        }
    };
    flood(address, "ev", counter("ev", "pipeline_runs")).await;
    flood(address, "ev-small", counter("ev-small", "pipeline_runs")).await;
    // A tick run takes every waiting input and adds how many to /n, which a
    // watcher reads from the patches.
    let mut watcher = connect(address, "tk", &[]).await;
    next_of(&mut watcher, "joined").await;
    let seen = Arc::new(AtomicU64::new(0));
    let s = seen.clone();
    tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = watcher.next().await {
            let frame: Value = serde_json::from_str(text.as_str()).expect("json");
            for op in frame["ops"].as_array().into_iter().flatten() {
                if op["path"] == json!(["n"]) {
                    s.store(op["value"].as_u64().unwrap_or_default(), Ordering::Relaxed);
                }
            }
        }
    });
    flood(address, "tk", move || {
        let seen = seen.clone();
        Box::pin(async move { seen.load(Ordering::Relaxed) })
    })
    .await;
}

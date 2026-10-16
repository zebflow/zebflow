//! `zeb/rooms` in a page: the compiler recognises the import and loads the
//! bundle in the browser, while the server render gets the stub — a room that
//! is still connecting and sends nothing.

use serde_json::json;
use zebflow::language::NoopLanguageEngine;
use zebflow::rwe::{
    ReactiveWebEngine, ReactiveWebOptions, RenderContext, RweReactiveWebEngine, TemplateSource,
};

#[test]
fn a_page_using_rooms_renders_connecting_on_the_server_and_loads_the_bundle() {
    let engine = RweReactiveWebEngine;
    let language = NoopLanguageEngine;
    let source = TemplateSource {
        id: "zeb-rooms-page".into(),
        source_path: None,
        markup: r#"
import { useRoom, joinRoom } from "zeb/rooms";

export default function Match() {
  const { state, status, send, room } = useRoom("match-7", { ticket: "/api/matches/7/ticket", select: (s) => s.players });
  const plain = joinRoom("lobby");
  return <main>
    <p id="status">{status}</p>
    <p id="state">{state === undefined ? "none" : "some"}</p>
    <p id="send">{String(send("move", { x: 1 }))}</p>
    <p id="room">{room === null ? "null" : "room"}</p>
    <p id="plain">{plain.status}:{String(plain.send("move", {}))}</p>
  </main>;
}
"#
        .into(),
    };
    let compiled = engine
        .compile_template(&source, &language, &ReactiveWebOptions::default())
        .expect("compile a page importing zeb/rooms");
    let context = RenderContext {
        route: "/match".into(),
        request_id: "zeb-rooms-page".into(),
        metadata: json!({}),
        enabled_libraries: vec![],
    };
    let rendered = engine.render(&compiled, json!({}), &language, &context).expect("render");
    assert!(!rendered.html.contains("RWE component error"), "{}", rendered.html);
    for expected in [
        "<p id=\"status\">connecting</p>",
        "<p id=\"state\">none</p>",
        "<p id=\"send\">false</p>",
        "<p id=\"room\">null</p>",
        "<p id=\"plain\">connecting:false</p>",
    ] {
        assert!(rendered.html.contains(expected), "{expected} in {}", rendered.html);
    }
    let scripts = rendered.compiled_scripts.iter().map(|script| script.content.as_str()).collect::<Vec<_>>().join("\n");
    assert!(
        scripts.contains("/assets/libraries/zeb/rooms/0.1/runtime/rooms.bundle.mjs"),
        "the browser loads the bundle"
    );
}

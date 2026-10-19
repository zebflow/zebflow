//! Runtime `zeb/*` libraries are bound per import, never installed as globals.
//!
//! The browser files each bundle under `globalThis.__zebLibs`; the server files
//! its stand-ins under the same keys. A page reads what it imported from there,
//! so the markup the server renders is the same, and a name the page did not
//! import is not in scope.

use serde_json::json;
use zebflow::language::NoopLanguageEngine;
use zebflow::rwe::{
    ReactiveWebEngine, ReactiveWebOptions, RenderContext, RenderOutput, RweReactiveWebEngine,
    TemplateSource,
};

fn render(markup: &str) -> Result<RenderOutput, String> {
    let engine = RweReactiveWebEngine;
    let language = NoopLanguageEngine;
    let source = TemplateSource {
        id: "zeb-libraries".into(),
        source_path: None,
        markup: markup.into(),
    };
    let compiled = engine
        .compile_template(&source, &language, &ReactiveWebOptions::default())
        .map_err(|error| format!("compile: {error:?}"))?;
    let context = RenderContext {
        route: "/libraries".into(),
        request_id: "zeb-libraries".into(),
        metadata: json!({}),
        enabled_libraries: vec![],
    };
    engine
        .render(&compiled, json!({}), &language, &context)
        .map_err(|error| format!("render: {error:?}"))
}

fn scripts(rendered: &RenderOutput) -> String {
    rendered
        .compiled_scripts
        .iter()
        .map(|script| script.content.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_page_renders_the_placeholders_of_the_libraries_it_imports() {
    let rendered = render(
        r##"
import { ThreeScene } from "zeb/threejs";
import { Markdown } from "zeb/markdown";
import { useDebounce } from "zeb/use";

export default function Page() {
  const shown = useDebounce("held", 200);
  const text = "# Hi";
  return <main>
    <ThreeScene id="scene" />
    <Markdown content={text} />
    <p id="debounced">{shown}</p>
  </main>;
}
"##,
    )
    .expect("render a page importing runtime libraries");

    assert!(!rendered.html.contains("RWE component error"), "{}", rendered.html);
    for expected in [
        // Byte for byte what the stub drew before libraries were bound per import.
        "<div data-zeb-lib=\"threejs\" data-zeb-wrapper=\"ThreeScene\" data-config=\"{}\" id=\"scene\" class=\"w-full h-full\"></div>",
        // The Markdown stand-in's placeholder, which the engine expands into
        // the rendered document before the page leaves.
        "<div class=\"prose\"><h1 id=\"hi\">Hi</h1>",
        "<p id=\"debounced\">held</p>",
    ] {
        assert!(rendered.html.contains(expected), "{expected} in {}", rendered.html);
    }
}

#[test]
fn renamed_and_namespace_imports_bind_the_library_that_was_imported() {
    let rendered = render(
        r##"
import { ThreeScene as Stage } from "zeb/threejs";
import * as potoru from "zeb/potoru";

export default function Page() {
  const { PotoSnapshot } = potoru;
  return <main>
    <Stage id="stage" />
    <PotoSnapshot id="snap" src="/_files/stories/intro.poto" />
    <p id="has-player">{String(typeof potoru.PotoPlayer)}</p>
  </main>;
}
"##,
    )
    .expect("render a page with an alias and a namespace import");

    assert!(!rendered.html.contains("RWE component error"), "{}", rendered.html);
    assert!(rendered.html.contains("data-zeb-wrapper=\"ThreeScene\""), "{}", rendered.html);
    assert!(rendered.html.contains("data-zeb-wrapper=\"PotoSnapshot\""), "{}", rendered.html);
    assert!(rendered.html.contains("<p id=\"has-player\">function</p>"), "{}", rendered.html);
}

#[test]
fn a_library_with_nothing_to_draw_on_the_server_still_imports() {
    let rendered = render(
        r##"
import { createDocument } from "zeb/pdf";
import { createEditor } from "zeb/prosemirror";

export default function Page() {
  return <p id="kept">{typeof createDocument}:{typeof createEditor}</p>;
}
"##,
    )
    .expect("render a page naming exports the server has no stand-in for");

    assert!(!rendered.html.contains("RWE component error"), "{}", rendered.html);
    assert!(rendered.html.contains("<p id=\"kept\">undefined:undefined</p>"), "{}", rendered.html);
}

/// The stand-ins are entries of their library, not globals: a page that names
/// one without importing it cannot reach it.
#[test]
fn a_library_name_that_was_not_imported_is_not_in_scope() {
    let outcome = render(
        r##"
export default function Page() {
  return <main><ThreeScene id="scene" /></main>;
}
"##,
    );
    match outcome {
        Err(_) => {}
        Ok(rendered) => {
            assert!(
                rendered.html.contains("RWE component error"),
                "ThreeScene was reachable without an import: {}",
                rendered.html
            );
            assert!(!rendered.html.contains("data-zeb-lib=\"threejs\""), "{}", rendered.html);
        }
    }
}

#[test]
fn the_browser_files_the_libraries_and_installs_none_of_their_names() {
    let rendered = render(
        r##"
import { mountThreeScene } from "zeb/threejs";
import { renderMarkdown } from "zeb/markdown";

export default function Page() {
  return <p id="fns">{typeof mountThreeScene}:{typeof renderMarkdown}</p>;
}
"##,
    )
    .expect("render");

    let scripts = scripts(&rendered);
    assert!(
        scripts.contains(
            "globalThis.__zebLibs[\"zeb/threejs\"] = await import('/assets/libraries/zeb/threejs/0.1/runtime/threejs.bundle.mjs');"
        ),
        "the browser loads the bundle into the registry: {scripts}"
    );
    assert!(scripts.contains("globalThis.__zebLibs[\"zeb/markdown\"]"), "{scripts}");
    assert!(!scripts.contains("Object.assign(globalThis"), "no library is spread onto globalThis");
    // The page module is imported after the whole preamble has run.
    let registry = scripts.rfind("globalThis.__zebLibs[\"zeb/markdown\"]").expect("registry");
    let page = scripts.find("await import('data:text/javascript;base64,").expect("page module");
    assert!(registry < page, "libraries load before the page module runs");
}

/// aslnut has hundreds of names a page calls while it renders to make the
/// options and elements it hands `<World>`. On the server each one is an inert
/// stand-in, and `<World>` renders exactly the box the wrapper's `World` renders
/// before it has a canvas, so hydration matches.
#[test]
fn a_page_calls_zeb_world_while_it_renders_and_gets_the_world_box() {
    let rendered = render(
        r##"
import { useMemo } from "zeb/react";
import { World, earth, plane, layer, data, scale } from "zeb/world";

export default function Page() {
  const places = useMemo(() => data.geojson("/places.geojson", { id: "id" }), []);
  const kind = useMemo(() => scale.category((d) => String(d.props.kind)), []);
  const options = useMemo(() => ({ body: earth({ imagery: [], terrain: [] }), flat: plane() }), []);
  const asked = layer({ id: "places", data: places, points: { size: 10, color: kind } });
  return (
    <World className="h-64 w-full" style={{ height: "300px" }} options={options} elements={[asked]}>
      <p id="overlay">on top</p>
    </World>
  );
}
"##,
    )
    .expect("render a page that calls zeb/world while it renders");

    assert!(!rendered.html.contains("RWE component error"), "{}", rendered.html);
    assert!(
        rendered.html.contains(
            "<div class=\"h-64 w-full\" style=\"position:relative;height:300px\"><div style=\"position:absolute;inset:0\"></div><p id=\"overlay\">on top</p></div>"
        ),
        "the box, its canvas slot, then the overlays: {}",
        rendered.html
    );
    let page = scripts(&rendered);
    assert!(
        page.contains(
            "globalThis.__zebLibs[\"zeb/world\"] = await import('/assets/libraries/zeb/world/0.1/runtime/world.bundle.mjs');"
        ),
        "the browser loads the bundle into the registry: {page}"
    );
}

#[test]
fn the_world_hooks_answer_no_world_and_a_stand_in_cannot_hang_an_await() {
    let rendered = render(
        r##"
import { useWorld, useElement, useWorldEvent, earth, imagery, data, theme } from "zeb/world";
import * as world from "zeb/world";

export default function Page() {
  useWorldEvent("tap", () => {});
  const made = earth({ imagery: [imagery.esri()] });
  const rows = data.rows([{ id: "a", lon: 0, lat: 0 }]);
  return <main>
    <p id="world">{String(useWorld())}</p>
    <p id="element">{String(useElement("places"))}</p>
    <p id="then">{typeof made.then}:{typeof rows.then}:{typeof world.then}</p>
    <p id="iterable">{typeof made[Symbol.iterator]}:{typeof made[Symbol.toPrimitive]}:{typeof made.toJSON}</p>
    <p id="called">{typeof made.anything.at.all()}:{typeof new theme.dark()}:{typeof world.shape}</p>
    <p id="json">{JSON.stringify({ made })}</p>
  </main>;
}
"##,
    )
    .expect("render a page that uses the zeb/world hooks and stand-ins");

    assert!(!rendered.html.contains("RWE component error"), "{}", rendered.html);
    for expected in [
        "<p id=\"world\">null</p>",
        "<p id=\"element\">null</p>",
        // `then` must be undefined, or an `await` of the value waits for ever.
        "<p id=\"then\">undefined:undefined:undefined</p>",
        "<p id=\"iterable\">undefined:undefined:undefined</p>",
        "<p id=\"called\">function:function:function</p>",
        "<p id=\"json\">{}</p>",
    ] {
        assert!(rendered.html.contains(expected), "{expected} in {}", rendered.html);
    }
}

/// The library is one package: every file the entry names answers beside it,
/// and the notices Apache-2.0 asks us to carry are in it.
#[test]
fn the_world_package_carries_every_file_its_entry_names() {
    let packages = zebflow::platform::blessed::blessed_packages().expect("blessed packages");
    let world = packages
        .iter()
        .find(|package| package.package_id == "zebflow.world")
        .expect("zeb/world is a blessed package");
    let file = |path: &str| {
        world
            .files
            .iter()
            .find(|file| file.rel_path == path)
            .unwrap_or_else(|| panic!("zebflow.world carries {path}"))
            .bytes
    };
    for notice in ["LICENSE", "NOTICE", "DATA-LICENCES.md", "THIRD-PARTY-NOTICES.txt"] {
        assert!(!file(notice).is_empty(), "{notice} is not empty");
    }
    let mut named = 0;
    for runtime in world.files.iter().filter(|file| {
        file.rel_path.starts_with("0.1/runtime/") && file.rel_path.ends_with(".mjs")
    }) {
        let text = String::from_utf8_lossy(runtime.bytes);
        for (at, _) in text.match_indices("./") {
            let name: String = text[at + 2..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
                .collect();
            if name.ends_with(".mjs") {
                named += 1;
                file(&format!("0.1/runtime/{name}"));
            }
        }
    }
    assert!(named >= 3, "the entry names its chunks and its tile worker: {named}");
    let entry = String::from_utf8_lossy(file("0.1/runtime/world.bundle.mjs"));
    assert!(entry.contains("globalThis.__zebReact"), "the entry reads Zeb React, not React");
    assert!(!entry.contains("from\"react\"") && !entry.contains("from'react'"), "no bare react import");
}

/// The page the browser suite serves (`tests/e2e/specs/rwe-world.spec.ts`,
/// read from there so the two cannot drift) renders on the server without a
/// component error, in the box the spec expects.
#[test]
fn the_world_page_of_the_browser_suite_renders_on_the_server() {
    let spec = std::fs::read_to_string("tests/e2e/specs/rwe-world.spec.ts").expect("the world spec");
    let start = spec.find("const SOURCE = `").expect("the spec holds the page") + "const SOURCE = `".len();
    let source = &spec[start..start + spec[start..].find("`;").expect("the page ends")];
    let rendered = render(source).expect("render the world page of the browser suite");

    assert!(!rendered.html.contains("RWE component error"), "{}", rendered.html);
    assert!(
        rendered.html.contains(
            "<div id=\"stage\"><div style=\"position:relative;width:480px;height:320px\"><div style=\"position:absolute;inset:0\"></div>"
        ),
        "{}",
        rendered.html
    );
    assert!(rendered.html.contains("cool:shown"), "{}", rendered.html);
}

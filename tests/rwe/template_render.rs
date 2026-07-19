use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use zebflow::language::NoopLanguageEngine;
use zebflow::rwe::{
    ComponentOptions, ReactiveWebEngine, ReactiveWebOptions, RenderContext, RweReactiveWebEngine,
    TemplateOptions, TemplateSource,
};

fn render_context(route: &str) -> RenderContext {
    RenderContext {
        route: route.to_string(),
        request_id: format!("req-{}", route.trim_matches('/').replace('/', "-")),
        metadata: json!({}),
        enabled_libraries: Vec::new(),
    }
}

fn demo_component_options() -> ReactiveWebOptions {
    let mut registry = BTreeMap::new();
    registry.insert(
        "BlogHeader".to_string(),
        include_str!("../../src/rwe/demo/templates/components/blog-header.tsx").to_string(),
    );
    registry.insert(
        "BlogHero".to_string(),
        include_str!("../../src/rwe/demo/templates/components/blog-hero.tsx").to_string(),
    );
    registry.insert(
        "TreeA".to_string(),
        include_str!("../../src/rwe/demo/templates/components/tree-a.tsx").to_string(),
    );
    registry.insert(
        "TreeB".to_string(),
        include_str!("../../src/rwe/demo/templates/components/tree-b.tsx").to_string(),
    );
    registry.insert(
        "TreeC".to_string(),
        include_str!("../../src/rwe/demo/templates/components/tree-c.tsx").to_string(),
    );
    registry.insert(
        "TreeD".to_string(),
        include_str!("../../src/rwe/demo/templates/components/tree-d.tsx").to_string(),
    );
    registry.insert(
        "TreeF".to_string(),
        include_str!("../../src/rwe/demo/templates/components/tree-f.tsx").to_string(),
    );

    ReactiveWebOptions {
        components: ComponentOptions {
            registry,
            strict: true,
        },
        processors: vec!["tailwind".to_string(), "markdown".to_string()],
        ..Default::default()
    }
}

fn temp_fixture_root(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("unix time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("zebflow-{name}-{nonce}"));
    fs::create_dir_all(&root).expect("create temp fixture root");
    root
}

#[test]
fn template_compile_and_render_supports_get_page_contract() {
    let engine = RweReactiveWebEngine;
    let language = NoopLanguageEngine;
    let template = TemplateSource {
        id: "page.dynamic-head".to_string(),
        source_path: None,
        markup: r#"
export const page = {
  html: { lang: "en" },
  body: { className: "bg-black text-white" },
  navigation: "history",
};

export function getPage(input) {
  return {
    head: {
      title: `${input.artist} — ${input.song}`,
      description: `Lyrics for ${input.song}`,
    },
    body: { className: "bg-slate-950 text-slate-100" },
  };
}

export default function Page(input) {
  return (
    <Page>
      <main>
        <h1>{input.song}</h1>
        <p>{input.artist}</p>
      </main>
    </Page>
  );
}
"#
        .to_string(),
    };

    let compiled = engine
        .compile_template(&template, &language, &ReactiveWebOptions::default())
        .expect("compile getPage template");
    assert_eq!(compiled.engine_id, "rwe");
    assert!(compiled.engine_payload.is_some());

    let rendered = engine
        .render(
            &compiled,
            json!({ "artist": "Aurora", "song": "Runaway" }),
            &language,
            &render_context("/lyrics/runaway"),
        )
        .expect("render getPage template");

    assert!(rendered.html.contains("Runaway"));
    assert!(rendered.html.contains("Aurora"));
}

#[test]
fn template_render_supports_blog_home_demo() {
    let engine = RweReactiveWebEngine;
    let language = NoopLanguageEngine;
    let template = TemplateSource {
        id: "demo.blog-home".to_string(),
        source_path: None,
        markup: include_str!("../../src/rwe/demo/templates/pages/blog-home.tsx").to_string(),
    };

    let compiled = engine
        .compile_template(&template, &language, &ReactiveWebOptions::default())
        .expect("compile blog home");

    let rendered = engine
        .render(
            &compiled,
            json!({
                "seo": {
                    "title": "Zebflow Blog",
                    "description": "Automation engineering posts",
                    "canonical": "https://example.com/blog"
                },
                "blog": {
                    "title": "Zebflow Engineering",
                    "tagline": "Build observable systems fast"
                },
                "posts": [
                    { "id": 1, "title": "Post A", "excerpt": "A excerpt", "url": "/blog/post-a" },
                    { "id": 2, "title": "Post B", "excerpt": "B excerpt", "url": "/blog/post-b" },
                    { "id": 3, "title": "Post C", "excerpt": "C excerpt", "url": "/blog/post-c" }
                ]
            }),
            &language,
            &render_context("/blog"),
        )
        .expect("render blog home");

    assert!(rendered.html.contains("Zebflow Engineering"));
    assert!(rendered.html.contains("Build observable systems fast"));
    assert!(rendered.html.contains("/blog/post-a"));
}

#[test]
fn template_render_supports_component_registry_composed_blog() {
    let engine = RweReactiveWebEngine;
    let language = NoopLanguageEngine;
    let template = TemplateSource {
        id: "demo.blog-home-composed".to_string(),
        source_path: None,
        markup: include_str!("../../src/rwe/demo/templates/pages/blog-home-composed.tsx")
            .to_string(),
    };

    let compiled = engine
        .compile_template(&template, &language, &demo_component_options())
        .expect("compile composed blog");

    let rendered = engine
        .render(
            &compiled,
            json!({
                "seo": { "title": "Composed Blog" },
                "blog": { "title": "Zebflow", "tagline": "Composable web" },
                "hero": { "title": "Hero", "subtitle": "Sub" },
                "posts": [{ "title": "A" }]
            }),
            &language,
            &render_context("/blog/composed"),
        )
        .expect("render composed blog");

    assert!(rendered.html.contains("Zebflow"));
    assert!(rendered.html.contains("Composable web"));
    assert!(rendered.html.contains("Hero"));
}

#[test]
fn template_render_supports_state_sharing_demo() {
    let engine = RweReactiveWebEngine;
    let language = NoopLanguageEngine;
    let template = TemplateSource {
        id: "demo.state-sharing".to_string(),
        source_path: None,
        markup: include_str!("../../src/rwe/demo/templates/pages/state-sharing-composed.tsx")
            .to_string(),
    };

    let compiled = engine
        .compile_template(&template, &language, &demo_component_options())
        .expect("compile state-sharing demo");

    let rendered = engine
        .render(
            &compiled,
            json!({ "shared": { "seed": 33 } }),
            &language,
            &render_context("/state-sharing"),
        )
        .expect("render state-sharing demo");

    assert!(!rendered.html.trim().is_empty());
    assert!(!rendered.compiled_scripts.is_empty());
}

#[test]
fn template_render_supports_list_hydration_demo() {
    let engine = RweReactiveWebEngine;
    let language = NoopLanguageEngine;
    let template = TemplateSource {
        id: "demo.list-hydration".to_string(),
        source_path: None,
        markup: include_str!("../../src/rwe/demo/templates/pages/list-hydration.tsx").to_string(),
    };

    let compiled = engine
        .compile_template(&template, &language, &ReactiveWebOptions::default())
        .expect("compile list hydration demo");

    let rendered = engine
        .render(
            &compiled,
            json!({
                "items": [
                    { "id": 11, "title": "Alpha" },
                    { "id": 12, "title": "Beta" }
                ]
            }),
            &language,
            &render_context("/list-hydration"),
        )
        .expect("render list hydration demo");

    assert!(rendered.html.contains("Keyed List + Hydration Islands"));
    assert!(rendered.html.contains("Alpha (#11)"));
    assert!(rendered.html.contains("Beta (#12)"));
}

#[test]
fn template_compile_resolves_imports_from_template_root() {
    let engine = RweReactiveWebEngine;
    let language = NoopLanguageEngine;
    let root = temp_fixture_root("rwe-import-root");
    let shared_dir = root.join("shared/ui");
    let pages_dir = root.join("pages");
    fs::create_dir_all(&shared_dir).expect("create shared dir");
    fs::create_dir_all(&pages_dir).expect("create pages dir");

    let component_path = shared_dir.join("button.tsx");
    fs::write(
        &component_path,
        r#"
export default function Button(props) {
  return <button className="rounded bg-zinc-900 text-white px-3 py-2">{props.label}</button>;
}
"#,
    )
    .expect("write button component");

    let page_path = pages_dir.join("example.tsx");
    let markup = r#"
import Button from "@/shared/ui/button";

export const page = {
  head: { title: "Import Works" },
  html: { lang: "en" },
  navigation: "history",
};

export default function Page(input) {
  return (
    <Page>
      <main>
        <h1>Imported Button</h1>
        <Button label={input.label} />
      </main>
    </Page>
  );
}
"#;
    fs::write(&page_path, markup).expect("write page");

    let compiled = engine
        .compile_template(
            &TemplateSource {
                id: "import.root.example".to_string(),
                source_path: Some(page_path.clone()),
                markup: markup.to_string(),
            },
            &language,
            &ReactiveWebOptions {
                templates: TemplateOptions {
                    template_root: Some(root),
                    style_entries: Vec::new(),
                },
                processors: vec!["tailwind".to_string()],
                ..Default::default()
            },
        )
        .expect("compile import-root example");

    let rendered = engine
        .render(
            &compiled,
            json!({ "label": "Import Works" }),
            &language,
            &render_context("/imports"),
        )
        .expect("render import-root example");

    assert!(rendered.html.contains("Imported Button"));
    assert!(rendered.html.contains("Import Works"));
}

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use zebflow::language::{
    COMPILE_TARGET_FRONTEND, CompileOptions, CompiledProgram, ExecutionContext, ExecutionOutput,
    LanguageEngine, LanguageError, ModuleSource, ProgramIr, SourceKind,
};
use zebflow::rwe::{
    ComponentOptions, NoopReactiveWebEngine, ReactiveMode, ReactiveWebEngine, ReactiveWebOptions,
    RenderContext, ResourceAllowList, RuntimeMode, StyleEngineMode, TemplateOptions,
    TemplateSource,
};

#[test]
fn template_compile_applies_tailwind_allowlist_and_reactive_scan() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let template = TemplateSource {
        id: "page.main".to_string(),
        source_path: None,
        markup: include_str!("fixtures/page.tsx").to_string(),
    };
    let options = ReactiveWebOptions {
        style_engine: StyleEngineMode::TailwindLike,
        reactive_mode: ReactiveMode::Bindings,
        runtime_mode: RuntimeMode::Prod,
        allow_list: ResourceAllowList {
            css: vec!["https://assets.safe/*".to_string()],
            scripts: vec!["https://cdn.safe/*".to_string()],
            urls: Vec::new(),
        },
        load_scripts: vec!["https://cdn.safe/runtime.js".to_string()],
        ..Default::default()
    };

    let compiled = engine
        .compile_template(&template, &language, &options)
        .expect("compile template");

    assert!(compiled.html_ir.contains("<style data-rwe-tw>"));
    assert!(!compiled.html_ir.contains("application/zebflow+json"));
    assert!(compiled.html_ir.contains("https://assets.safe/app.css"));
    assert!(!compiled.html_ir.contains("https://blocked.bad/evil.css"));
    assert!(compiled.html_ir.contains("https://cdn.safe/runtime.js"));
    assert!(!compiled.html_ir.contains("https://blocked.bad/evil.js"));
    assert!(compiled.compiled_logic.is_some());
    assert!(compiled.runtime_bundle.source.contains("dispatch"));
    assert!(compiled.runtime_bundle.source.contains("runMemos"));
    assert!(compiled.runtime_bundle.source.contains("runEffects"));
    let artifact = compiled
        .compiled_logic
        .as_ref()
        .expect("compiled logic")
        .artifact
        .clone();
    let decoded: Value = serde_json::from_slice(&artifact).expect("decode compiled artifact");
    let source = decoded
        .get("source")
        .and_then(Value::as_str)
        .expect("script source in compiled artifact");
    assert!(source.contains("function counterDelta"));
    assert!(source.contains("\"counter.inc\""));
    assert!(
        compiled
            .control_script_source
            .as_deref()
            .unwrap_or("")
            .contains("effect")
    );

    assert!(
        compiled
            .reactive_bindings
            .iter()
            .any(|b| b.kind == "event.click" && b.key == "counter.inc")
    );
    assert!(
        compiled
            .reactive_bindings
            .iter()
            .any(|b| b.kind == "bind.text" && b.key == "client.title")
    );
    assert!(
        compiled
            .reactive_bindings
            .iter()
            .any(|b| b.kind == "bind.show" && b.key == "client.showInfo")
    );
    assert!(
        compiled
            .reactive_bindings
            .iter()
            .any(|b| b.kind == "bind.hide" && b.key == "client.showInfo")
    );
    assert_eq!(compiled.runtime_bundle.name, "rwe-runtime.js");
}

#[test]
fn template_render_injects_runtime_and_forwards_language_patch() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let template = TemplateSource {
        id: "page.main".to_string(),
        source_path: None,
        markup: include_str!("fixtures/page.tsx").to_string(),
    };
    let options = ReactiveWebOptions {
        runtime_mode: RuntimeMode::Dev,
        language: zebflow::rwe::LanguageOptions {
            run_patch: Some(json!({
                "allowList": { "externalFetchHosts": ["openai.com"] },
                "timeoutMs": 1000
            })),
        },
        ..Default::default()
    };

    let compiled = engine
        .compile_template(&template, &language, &options)
        .expect("compile template");
    let out = engine
        .render(
            &compiled,
            json!({ "title": "Hello" }),
            &language,
            &RenderContext {
                route: "/home".to_string(),
                request_id: "req-1".to_string(),
                metadata: json!({}),
            },
        )
        .expect("render template");

    assert!(out.html.contains("data-rwe-runtime=\"rwe-runtime-dev.js\""));
    assert!(out.html.contains("window.__ZEBFLOW_RWE_BINDINGS__"));
    assert!(out.html.contains("window.__ZEBFLOW_RWE__.mount"));
    assert!(out.html.contains("new Function('input','metadata'"));

    let meta = out
        .hydration_payload
        .get("metadata")
        .cloned()
        .unwrap_or(Value::Null);
    assert!(meta.get("languageRunPatch").is_some());
    assert_eq!(
        out.hydration_payload
            .get("input")
            .and_then(Value::as_object)
            .and_then(|o| o.get("title"))
            .and_then(Value::as_str),
        Some("Hello")
    );
}

#[test]
fn template_compile_supports_application_zebflow_json_script_payload() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let template = TemplateSource {
        id: "page.program".to_string(),
        source_path: None,
        markup: r#"
<html><body>
  <h1>Program Payload</h1>
  <script type="application/zebflow+json">
    { "state": { "client": { "count": 1 } }, "actions": {} }
  </script>
</body></html>
"#
        .to_string(),
    };

    let compiled = engine
        .compile_template(&template, &language, &ReactiveWebOptions::default())
        .expect("compile template with application/zebflow+json");

    assert!(compiled.compiled_logic.is_some());
    assert!(!compiled.html_ir.contains("application/zebflow+json"));
    assert!(
        compiled
            .control_script_source
            .as_deref()
            .unwrap_or("")
            .contains("\"count\":1")
    );

    let rendered = engine
        .render(
            &compiled,
            json!({}),
            &language,
            &RenderContext {
                route: "/json".to_string(),
                request_id: "req-json".to_string(),
                metadata: json!({}),
            },
        )
        .expect("render template with application/zebflow+json");
    assert!(rendered.html.contains("window.__ZEBFLOW_RWE__.mount"));
}

#[test]
fn template_compile_supports_convention_todo_and_list_render_examples() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let options = ReactiveWebOptions::default();

    let todo = TemplateSource {
        id: "convention.todo".to_string(),
        source_path: None,
        markup: include_str!("../../docs/conventions/templates/pages/todo.tsx").to_string(),
    };
    let todo_compiled = engine
        .compile_template(&todo, &language, &options)
        .expect("compile todo.tsx");
    assert!(todo_compiled.compiled_logic.is_some());
    assert!(
        todo_compiled
            .control_script_source
            .as_deref()
            .unwrap_or("")
            .contains("\"todo.add\"")
    );

    let list = TemplateSource {
        id: "convention.list_render".to_string(),
        source_path: None,
        markup: include_str!("../../docs/conventions/templates/pages/list-render.tsx").to_string(),
    };
    let list_compiled = engine
        .compile_template(&list, &language, &options)
        .expect("compile list-render.tsx");
    assert!(list_compiled.compiled_logic.is_some());
    assert!(
        list_compiled
            .reactive_bindings
            .iter()
            .any(|b| b.kind == "bind.text" && b.key == "list.rows.0")
    );
}

#[test]
fn template_render_supports_ssr_blog_templates() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let options = ReactiveWebOptions::default();

    let blog_home = TemplateSource {
        id: "convention.blog_home".to_string(),
        source_path: None,
        markup: include_str!("../../docs/conventions/templates/pages/blog-home.tsx").to_string(),
    };
    let compiled_home = engine
        .compile_template(&blog_home, &language, &options)
        .expect("compile blog-home.tsx");
    let rendered_home = engine
        .render(
            &compiled_home,
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
                    { "title": "Post A", "excerpt": "A excerpt", "url": "/blog/post-a" },
                    { "title": "Post B", "excerpt": "B excerpt", "url": "/blog/post-b" },
                    { "title": "Post C", "excerpt": "C excerpt", "url": "/blog/post-c" }
                ]
            }),
            &language,
            &RenderContext {
                route: "/blog".to_string(),
                request_id: "req-blog-home".to_string(),
                metadata: json!({}),
            },
        )
        .expect("render blog-home.tsx");
    assert!(rendered_home.html.contains("<title>Zebflow Blog</title>"));
    assert!(rendered_home.html.contains("<h1>Zebflow Engineering</h1>"));
    assert!(rendered_home.html.contains("href=\"/blog/post-a\""));

    let blog_post = TemplateSource {
        id: "convention.blog_post".to_string(),
        source_path: None,
        markup: include_str!("../../docs/conventions/templates/pages/blog-post.tsx").to_string(),
    };
    let compiled_post = engine
        .compile_template(&blog_post, &language, &options)
        .expect("compile blog-post.tsx");
    let rendered_post = engine
        .render(
            &compiled_post,
            json!({
                "post": {
                    "seoTitle": "Post A | Zebflow",
                    "seoDescription": "A deep-dive about RWE",
                    "url": "https://example.com/blog/post-a",
                    "title": "Post A",
                    "author": "Mala",
                    "publishedAt": "2026-02-26",
                    "summary": "Intro paragraph",
                    "body": ["Line 1", "Line 2", "Line 3"]
                }
            }),
            &language,
            &RenderContext {
                route: "/blog/post-a".to_string(),
                request_id: "req-blog-post".to_string(),
                metadata: json!({}),
            },
        )
        .expect("render blog-post.tsx");
    assert!(
        rendered_post
            .html
            .contains("<title>Post A | Zebflow</title>")
    );
    assert!(rendered_post.html.contains("<h1>Post A</h1>"));
    assert!(rendered_post.html.contains("By Mala"));
}

#[test]
fn template_render_supports_boolean_jshow_and_jhide_expressions() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let template = TemplateSource {
        id: "page.boolean_visibility".to_string(),
        source_path: None,
        markup: r#"
export const page = {
  head: { title: "Visibility" },
  html: { lang: "en" },
  body: { className: "bg-slate-950" },
  navigation: "history",
};

export default function Page(input) {
  return (
    <Page>
      <section id="shown" jShow="input.flags.show && !input.flags.hide">Shown</section>
      <section id="hidden" jHide="input.flags.show || input.flags.hide">Hidden</section>
    </Page>
  );
}
"#
        .to_string(),
    };

    let compiled = engine
        .compile_template(&template, &language, &ReactiveWebOptions::default())
        .expect("compile boolean visibility template");

    let rendered = engine
        .render(
            &compiled,
            json!({
                "flags": {
                    "show": true,
                    "hide": false
                }
            }),
            &language,
            &RenderContext {
                route: "/visibility".to_string(),
                request_id: "req-visibility".to_string(),
                metadata: json!({}),
            },
        )
        .expect("render boolean visibility template");

    assert!(rendered.html.contains("<section id=\"shown\" j-show=\"input.flags.show && !input.flags.hide\">Shown</section>"));
    assert!(rendered.html.contains("<section id=\"hidden\" j-hide=\"input.flags.show || input.flags.hide\" hidden>Hidden</section>"));
}

#[test]
fn template_compile_strips_jsx_comments_from_markup() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let template = TemplateSource {
        id: "page.jsx_comments".to_string(),
        source_path: None,
        markup: r##"
export const page = {
  head: { title: "Comments" },
  html: { lang: "en" },
  body: { className: "bg-slate-950" },
  navigation: "history",
};

export default function Page(input) {
  return (
    <Page>
      <div>Before</div>
      {/* <span className="should-not-render">ghost</span> */}
      <div>After</div>
    </Page>
  );
}
"##
        .to_string(),
    };

    let compiled = engine
        .compile_template(&template, &language, &ReactiveWebOptions::default())
        .expect("compile jsx comment template");

    assert!(compiled.html_ir.contains("<div>Before</div>"));
    assert!(compiled.html_ir.contains("<div>After</div>"));
    assert!(!compiled.html_ir.contains("should-not-render"));
    assert!(!compiled.html_ir.contains("ghost"));
}

#[test]
fn template_compile_supports_ts_line_and_block_comments_around_structure() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let template = TemplateSource {
        id: "page.comment_variants".to_string(),
        source_path: None,
        markup: r#"
// top-level line comment
export const page = {
  /* object comment */
  head: { title: "Comment Variants" },
  html: { lang: "en" },
  body: { className: "bg-slate-950" },
  navigation: "history",
};

export const app = {
  // app comment
  state: {
    count: 1,
  },
  actions: {
    /* block comment inside object */
  },
};

export default function Page(input) {
  return (
    <Page>
      {/* jsx comment */}
      <div>{input.label}</div>
    </Page>
  );
}
"#
        .to_string(),
    };

    let compiled = engine
        .compile_template(&template, &language, &ReactiveWebOptions::default())
        .expect("compile comment variants template");

    assert!(compiled.html_ir.contains("{{input.label}}"));
    assert!(!compiled.html_ir.contains("jsx comment"));
    assert!(
        compiled
            .control_script_source
            .as_deref()
            .unwrap_or("")
            .contains("count")
    );
}

#[test]
fn template_render_ssr_for_loop_handles_nested_same_tag_descendants() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let template = TemplateSource {
        id: "page.for_nested_same_tag".to_string(),
        source_path: None,
        markup: r#"
export const page = {
  head: { title: "Nested Loop" },
  html: { lang: "en" },
  body: { className: "bg-slate-950" },
  navigation: "history",
};

export default function Page(input) {
  return (
    <Page>
      <div className="path">
        <span jFor="crumb in input.breadcrumbs" className="crumb">
          <span jShow="crumb.show_divider" className="divider">/</span>
          <a href="{crumb.path}" className="label">{crumb.name}</a>
        </span>
      </div>
    </Page>
  );
}
"#
        .to_string(),
    };

    let compiled = engine
        .compile_template(&template, &language, &ReactiveWebOptions::default())
        .expect("compile nested same-tag loop template");

    let rendered = engine
        .render(
            &compiled,
            json!({
                "breadcrumbs": [
                    { "name": "root", "path": "/", "show_divider": false },
                    { "name": "automation", "path": "/automation", "show_divider": true },
                    { "name": "email", "path": "/automation/email", "show_divider": true }
                ]
            }),
            &language,
            &RenderContext {
                route: "/pipelines".to_string(),
                request_id: "req-loop-nested".to_string(),
                metadata: json!({}),
            },
        )
        .expect("render nested same-tag loop template");

    assert!(rendered.html.contains(">root</a>"));
    assert!(rendered.html.contains(">automation</a>"));
    assert!(rendered.html.contains(">email</a>"));
    assert!(rendered.html.contains("class=\"divider\""));
    assert!(rendered.html.contains(">/</span>"));
    assert!(rendered.html.contains("href=\"/\""));
    assert!(rendered.html.contains("href=\"/automation\""));
    assert!(rendered.html.contains("href=\"/automation/email\""));
}

#[test]
fn template_compile_supports_component_registry_in_compile_stage() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;

    let mut registry = BTreeMap::new();
    registry.insert(
        "BlogHeader".to_string(),
        include_str!("../../docs/conventions/templates/components/blog-header.tsx").to_string(),
    );
    registry.insert(
        "BlogHero".to_string(),
        include_str!("../../docs/conventions/templates/components/blog-hero.tsx").to_string(),
    );

    let options = ReactiveWebOptions {
        components: ComponentOptions {
            registry,
            strict: true,
        },
        ..Default::default()
    };

    let template = TemplateSource {
        id: "convention.blog_home_composed".to_string(),
        source_path: None,
        markup: include_str!("../../docs/conventions/templates/pages/blog-home-composed.tsx")
            .to_string(),
    };
    let compiled = engine
        .compile_template(&template, &language, &options)
        .expect("compile componentized blog template");
    assert!(!compiled.html_ir.contains("<BlogHeader"));
    assert!(compiled.html_ir.contains("{{input.blog.title}}"));
    assert!(
        compiled
            .diagnostics
            .iter()
            .any(|d| d.code == "RWE_COMPONENT_RESOLVED")
    );

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
            &RenderContext {
                route: "/blog".to_string(),
                request_id: "req-blog-composed".to_string(),
                metadata: json!({}),
            },
        )
        .expect("render componentized blog template");
    assert!(rendered.html.contains("Zebflow"));
    assert!(rendered.html.contains("Composable web"));
}

#[test]
fn template_compile_supports_component_level_hydration_modes() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;

    let mut registry = BTreeMap::new();
    registry.insert(
        "Button".to_string(),
        r#"
export const app = {};

export default function Page(input) {
  return (
    <button className="px-3 py-2 rounded-md bg-gray-900 text-white">
      <span>{props.label}</span>
    </button>
  );
}
"#
        .to_string(),
    );

    let options = ReactiveWebOptions {
        components: ComponentOptions {
            registry,
            strict: true,
        },
        ..Default::default()
    };

    let template = TemplateSource {
        id: "page.component_hydrate".to_string(),
        source_path: None,
        markup: r#"
export const app = {};

export default function Page(input) {
  return (
    <html>
      <body>
        <Button label="Open" hydrate="interaction" />
      </body>
    </html>
  );
}
"#
        .to_string(),
    };

    let compiled = engine
        .compile_template(&template, &language, &options)
        .expect("compile component hydration template");
    assert!(compiled.html_ir.contains("data-rwe-component=\"Button\""));
    assert!(compiled.html_ir.contains("hydrate=\"interaction\""));
    assert!(
        compiled
            .reactive_bindings
            .iter()
            .any(|b| b.kind == "hydrate.mode" && b.key == "interaction")
    );
    assert!(
        compiled
            .diagnostics
            .iter()
            .any(|d| d.code == "RWE_COMPONENT_HYDRATE")
    );
}

#[test]
fn template_compile_resolves_component_imports_from_template_root() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let template_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/platform/web/templates");
    let source_path = template_root.join("pages/platform-login.tsx");
    let markup = std::fs::read_to_string(&source_path).expect("read platform-login.tsx");

    let compiled = engine
        .compile_template(
            &TemplateSource {
                id: "platform.login".to_string(),
                source_path: Some(source_path),
                markup,
            },
            &language,
            &ReactiveWebOptions {
                templates: TemplateOptions {
                    template_root: Some(template_root),
                    style_entries: Vec::new(),
                },
                processors: vec!["tailwind".to_string()],
                ..Default::default()
            },
        )
        .expect("compile platform login with explicit imports");

    assert!(!compiled.html_ir.contains("<Button"));
    assert!(!compiled.html_ir.contains("<Page>"));
    assert!(compiled.html_ir.contains("<html lang=\"en\">"));
    assert!(
        compiled
            .html_ir
            .contains("<title>{{input.seo.title}}</title>")
    );
    assert!(compiled.html_ir.contains("Secure Access"));
    assert!(
        compiled
            .diagnostics
            .iter()
            .any(|d| d.code == "RWE_COMPONENT_RESOLVED")
    );
}

#[test]
fn template_compile_auto_discovers_template_root_style_entries() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let root = temp_fixture_root("template-style-defaults");
    let styles_dir = root.join("styles");
    let pages_dir = root.join("pages");
    fs::create_dir_all(&styles_dir).expect("create styles dir");
    fs::create_dir_all(&pages_dir).expect("create pages dir");
    fs::write(
        styles_dir.join("main.css"),
        ":root { --zf-theme-accent: #dc2626; }\n[data-theme=\"marketing\"] { --zf-theme-accent: #0f766e; }\n",
    )
    .expect("write main css");
    let source_path = pages_dir.join("landing.tsx");
    let markup = r#"export const page = {
  head: { title: "Landing" },
  html: { lang: "en" },
  body: { className: "bg-zinc-50 text-gray-900" }
};

export default function Page(input) {
  return (
    <Page>
      <main className="px-6 py-10">
        <h1>Landing</h1>
      </main>
    </Page>
  );
}
"#;
    fs::write(&source_path, markup).expect("write page");

    let compiled = engine
        .compile_template(
            &TemplateSource {
                id: "theme.defaults".to_string(),
                source_path: Some(source_path),
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
        .expect("compile template with discovered styles");

    assert!(compiled.html_ir.contains("--zf-theme-accent: #dc2626"));
    assert!(
        compiled
            .html_ir
            .contains("data-rwe-template-style=\"styles/main.css\"")
    );
    assert!(compiled.html_ir.contains("[data-theme=\"marketing\"]"));
    assert!(compiled.html_ir.contains("<style data-rwe-tw>"));
    assert!(
        compiled
            .diagnostics
            .iter()
            .any(|d| d.code == "RWE_TEMPLATE_STYLE_INCLUDED"
                && d.message.contains("styles/main.css"))
    );
}

#[test]
fn template_compile_explicit_template_style_entries_fail_when_missing() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let root = temp_fixture_root("template-style-missing");
    let pages_dir = root.join("pages");
    fs::create_dir_all(&pages_dir).expect("create pages dir");
    let source_path = pages_dir.join("landing.tsx");
    let markup = r#"export const page = {
  head: { title: "Landing" },
  html: { lang: "en" },
  body: { className: "bg-white text-gray-900" }
};

export default function Page(input) {
  return <Page><main>Landing</main></Page>;
}
"#;
    fs::write(&source_path, markup).expect("write page");

    let err = engine
        .compile_template(
            &TemplateSource {
                id: "theme.explicit_missing".to_string(),
                source_path: Some(source_path),
                markup: markup.to_string(),
            },
            &language,
            &ReactiveWebOptions {
                templates: TemplateOptions {
                    template_root: Some(root),
                    style_entries: vec!["styles/brand.css".to_string()],
                },
                ..Default::default()
            },
        )
        .expect_err("missing explicit style entry should fail");

    assert!(err.code.contains("RWE_TEMPLATE_STYLE_MISSING"));
}

#[test]
fn template_compile_and_render_supports_nested_component_state_sharing_tree() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;

    let mut registry = BTreeMap::new();
    registry.insert(
        "TreeA".to_string(),
        include_str!("../../docs/conventions/templates/components/tree-a.tsx").to_string(),
    );
    registry.insert(
        "TreeB".to_string(),
        include_str!("../../docs/conventions/templates/components/tree-b.tsx").to_string(),
    );
    registry.insert(
        "TreeC".to_string(),
        include_str!("../../docs/conventions/templates/components/tree-c.tsx").to_string(),
    );
    registry.insert(
        "TreeD".to_string(),
        include_str!("../../docs/conventions/templates/components/tree-d.tsx").to_string(),
    );
    registry.insert(
        "TreeF".to_string(),
        include_str!("../../docs/conventions/templates/components/tree-f.tsx").to_string(),
    );

    let options = ReactiveWebOptions {
        components: ComponentOptions {
            registry,
            strict: true,
        },
        ..Default::default()
    };

    let template = TemplateSource {
        id: "convention.state_sharing_composed".to_string(),
        source_path: None,
        markup: include_str!("../../docs/conventions/templates/pages/state-sharing-composed.tsx")
            .to_string(),
    };
    let compiled = engine
        .compile_template(&template, &language, &options)
        .expect("compile state-sharing-composed.tsx");
    assert!(!compiled.html_ir.contains("<TreeA"));
    assert!(compiled.html_ir.contains("@click=\"tree.c.inc\""));
    assert!(compiled.html_ir.contains("@click=\"tree.f.reset\""));
    assert!(
        compiled
            .reactive_bindings
            .iter()
            .any(|b| b.kind == "bind.text" && b.key == "shared.value")
    );
    assert!(
        compiled
            .control_script_source
            .as_deref()
            .unwrap_or("")
            .contains("\"tree.c.inc\"")
    );

    let rendered = engine
        .render(
            &compiled,
            json!({
                "shared": { "seed": 7 }
            }),
            &language,
            &RenderContext {
                route: "/state-sharing".to_string(),
                request_id: "req-state-sharing".to_string(),
                metadata: json!({}),
            },
        )
        .expect("render state-sharing-composed.tsx");
    assert!(rendered.html.contains("SSR seed value: 7"));
    assert!(rendered.html.contains("F reads shared value:"));
    assert!(rendered.html.contains("new Function('input','metadata'"));
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
fn template_render_supports_j_for_keyed_and_hydration_islands() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let template = TemplateSource {
        id: "convention.list_hydration".to_string(),
        source_path: None,
        markup: include_str!("../../docs/conventions/templates/pages/list-hydration.tsx").to_string(),
    };

    let compiled = engine
        .compile_template(&template, &language, &ReactiveWebOptions::default())
        .expect("compile list-hydration.tsx");
    assert!(
        compiled
            .reactive_bindings
            .iter()
            .any(|b| b.kind == "bind.for" && b.key == "item in input.items")
    );
    assert!(
        compiled
            .reactive_bindings
            .iter()
            .any(|b| b.kind == "bind.for.key" && b.key == "item.id")
    );
    assert!(
        compiled
            .reactive_bindings
            .iter()
            .any(|b| b.kind == "hydrate.mode" && b.key == "interaction")
    );
    assert!(compiled.runtime_bundle.source.contains("initForBlocks"));
    assert!(compiled.runtime_bundle.source.contains("renderForBlocks"));
    assert!(
        compiled
            .runtime_bundle
            .source
            .contains("initHydrationIslands")
    );

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
            &RenderContext {
                route: "/list-hydration".to_string(),
                request_id: "req-list-hydration".to_string(),
                metadata: json!({}),
            },
        )
        .expect("render list-hydration.tsx");
    assert!(rendered.html.contains("data-rwe-for-template=\"1\""));
    assert!(rendered.html.contains("data-rwe-for-seeded="));
    assert!(rendered.html.contains("Alpha (#11)"));
    assert!(rendered.html.contains("Beta (#12)"));
    assert!(rendered.html.contains("hydrate=\"interaction\""));
}

#[test]
fn template_render_resolves_j_for_placeholders_in_tag_attributes() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let template = TemplateSource {
        id: "page.jfor.attr".to_string(),
        source_path: None,
        markup: r#"
<!doctype html>
<html>
  <body>
    <a j-for="item in input.items" href="/projects/{{item.owner}}/{{item.project}}">
      {{item.project}}
    </a>
  </body>
</html>
"#
        .to_string(),
    };

    let compiled = engine
        .compile_template(&template, &language, &ReactiveWebOptions::default())
        .expect("compile j-for attribute template");

    let rendered = engine
        .render(
            &compiled,
            json!({
                "items": [
                    { "owner": "superadmin", "project": "default" }
                ]
            }),
            &language,
            &RenderContext {
                route: "/home".to_string(),
                request_id: "req-jfor-attr".to_string(),
                metadata: json!({}),
            },
        )
        .expect("render j-for attribute template");

    assert!(
        rendered
            .html
            .contains("href=\"/projects/superadmin/default\"")
    );
    assert!(!rendered.html.contains("href=\"/projects//\""));
}

#[test]
fn template_compile_tailwind_like_supports_variants_and_arbitrary_values() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let template = TemplateSource {
        id: "page.tailwind.rich".to_string(),
        source_path: None,
        markup: r#"
<!doctype html>
<html>
  <head><title>Tailwind Rich</title></head>
  <body>
    <button class="md:px-4 hover:bg-cyan-400 w-[24rem] text-sm">Click</button>
  </body>
</html>
"#
        .to_string(),
    };

    let compiled = engine
        .compile_template(&template, &language, &ReactiveWebOptions::default())
        .expect("compile rich tailwind-like template");

    assert!(compiled.html_ir.contains("<style data-rwe-tw>"));
    assert!(compiled.html_ir.contains("@media (min-width: 768px)"));
    assert!(compiled.html_ir.contains(".hover\\:bg-cyan-400:hover{"));
    assert!(compiled.html_ir.contains(".w-\\[24rem\\]{width:24rem;}"));
    assert!(compiled.html_ir.contains(".text-sm{font-size:0.875rem;}"));
}

#[test]
fn template_compile_tailwind_font_black_maps_to_weight_not_family() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let template = TemplateSource {
        id: "page.font_black".to_string(),
        source_path: None,
        markup: r#"
export const page = {
  head: { title: "Fonts" },
  html: { lang: "en" },
  body: { className: "font-sans" }
};

export default function Page(input) {
  return (
    <Page>
      <main>
        <h1 className="text-4xl font-black">Heading</h1>
      </main>
    </Page>
  );
}
"#
        .to_string(),
    };

    let compiled = engine
        .compile_template(
            &template,
            &language,
            &ReactiveWebOptions {
                processors: vec!["tailwind".to_string()],
                ..Default::default()
            },
        )
        .expect("compile font-black template");

    assert!(compiled.html_ir.contains("font-weight:900;"));
    assert!(!compiled.html_ir.contains("--zebflow-font-black"));
}

#[test]
fn template_compile_processors_can_enable_tailwind_and_markdown_explicitly() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let template = TemplateSource {
        id: "page.processor.explicit".to_string(),
        source_path: None,
        markup: r#"
<!doctype html>
<html>
  <head><title>Processors</title></head>
  <body class="px-4">
    <markdown># Hello Processor</markdown>
  </body>
</html>
"#
        .to_string(),
    };

    let options = ReactiveWebOptions {
        style_engine: StyleEngineMode::Off,
        processors: vec!["tailwind".to_string(), "markdown".to_string()],
        ..Default::default()
    };
    let compiled = engine
        .compile_template(&template, &language, &options)
        .expect("compile with explicit processors");

    assert!(compiled.html_ir.contains("<style data-rwe-tw>"));
    assert!(
        compiled
            .html_ir
            .contains(".px-4{padding-left:1rem;padding-right:1rem;}")
    );
    assert!(compiled.html_ir.contains("<h1>Hello Processor</h1>"));
    assert!(!compiled.html_ir.contains("<markdown>"));
}

#[test]
fn template_compile_processors_explicit_list_overrides_legacy_style_engine_flag() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let template = TemplateSource {
        id: "page.processor.override".to_string(),
        source_path: None,
        markup: r#"
<!doctype html>
<html>
  <head><title>Processors</title></head>
  <body class="px-4">
    <markdown>**Bold**</markdown>
  </body>
</html>
"#
        .to_string(),
    };

    let options = ReactiveWebOptions {
        // style_engine defaults to TailwindLike; explicit processors should override it.
        processors: vec!["markdown".to_string()],
        ..Default::default()
    };
    let compiled = engine
        .compile_template(&template, &language, &options)
        .expect("compile with markdown-only processor list");

    assert!(!compiled.html_ir.contains("<style data-rwe-tw>"));
    assert!(compiled.html_ir.contains("<p><strong>Bold</strong></p>"));
}

#[test]
fn template_compile_tailwind_processor_injects_preflight_baseline() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let template = TemplateSource {
        id: "page.processor.preflight".to_string(),
        source_path: None,
        markup: r#"
<!doctype html>
<html>
  <head><title>Preflight</title></head>
  <body>
    <h1>Preflight Check</h1>
  </body>
</html>
"#
        .to_string(),
    };
    let options = ReactiveWebOptions {
        style_engine: StyleEngineMode::Off,
        processors: vec!["tailwind".to_string()],
        ..Default::default()
    };
    let compiled = engine
        .compile_template(&template, &language, &options)
        .expect("compile with tailwind preflight");

    assert!(compiled.html_ir.contains("<style data-rwe-tw>"));
    assert!(compiled.html_ir.contains("box-sizing: border-box"));
    assert!(compiled.html_ir.contains("list-style: none"));
    assert!(!compiled.html_ir.contains("--theme("));
}

#[test]
fn template_compile_warns_for_dynamic_class_placeholders() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let template = TemplateSource {
        id: "page.dynamic.nav".to_string(),
        source_path: None,
        markup: r#"
<!doctype html>
<html>
  <head><title>Dynamic Classes</title></head>
  <body>
    <a class="{{input.nav.class}}">Nav Item</a>
  </body>
</html>
"#
        .to_string(),
    };

    let compiled = engine
        .compile_template(&template, &language, &ReactiveWebOptions::default())
        .expect("compile template with dynamic classes");
    // Compile stage cannot resolve dynamic class payloads yet.
    assert!(!compiled.html_ir.contains(".py-2{"));
    assert!(compiled.needs_runtime_tailwind_rebuild);
    assert!(
        compiled
            .diagnostics
            .iter()
            .any(|d| d.code == "RWE_TAILWIND_DYNAMIC_CLASS_WARN")
    );

    let rendered = engine
        .render(
            &compiled,
            json!({
                "nav": {
                    "class": "group w-full flex items-center gap-3 px-3 py-2 rounded-md bg-gray-900 text-white text-sm font-medium"
                }
            }),
            &language,
            &RenderContext {
                route: "/projects/superadmin/default/design".to_string(),
                request_id: "req-dynamic-tailwind".to_string(),
                metadata: json!({}),
            },
        )
        .expect("render template with dynamic classes");

    assert!(!rendered.html.contains(".py-2{"));
    assert!(
        !rendered
            .html
            .contains(".rounded-md{border-radius:0.375rem;}")
    );
    assert!(
        rendered
            .html
            .contains("class=\"group w-full flex items-center gap-3 px-3 py-2 rounded-md bg-gray-900 text-white text-sm font-medium\"")
    );
}

#[test]
fn template_compile_tw_variants_on_parent_scope_compiles_exact_dynamic_tokens_once() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let template = TemplateSource {
        id: "page.dynamic.nav.with.variants".to_string(),
        source_path: None,
        markup: r#"
<!doctype html>
<html>
  <head><title>Dynamic Classes With Variants</title></head>
  <body>
    <div tw-variants="py-2 rounded-md bg-gray-900 text-white">
      <a class="{{input.nav.class}}">Nav Item</a>
    </div>
  </body>
</html>
"#
        .to_string(),
    };

    let compiled = engine
        .compile_template(&template, &language, &ReactiveWebOptions::default())
        .expect("compile template with parent tw-variants");

    assert!(!compiled.needs_runtime_tailwind_rebuild);
    assert!(
        !compiled
            .diagnostics
            .iter()
            .any(|d| d.code == "RWE_TAILWIND_DYNAMIC_CLASS_WARN")
    );
    assert!(compiled.tailwind_variant_patterns.is_empty());
    assert!(
        compiled
            .tailwind_variant_exact_tokens
            .contains(&"py-2".to_string())
    );
    assert!(
        compiled
            .tailwind_variant_exact_tokens
            .contains(&"rounded-md".to_string())
    );

    assert!(compiled.html_ir.contains(".py-2{"));
    assert!(
        compiled
            .html_ir
            .contains(".rounded-md{border-radius:0.375rem;}")
    );
    assert!(compiled.html_ir.contains(".bg-gray-900{"));
    assert!(compiled.html_ir.contains(".text-white{"));
}

#[test]
fn template_compile_tw_variants_wildcards_enable_runtime_transform_bundle() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let template = TemplateSource {
        id: "page.dynamic.nav.wildcards".to_string(),
        source_path: None,
        markup: r#"
<!doctype html>
<html>
  <head><title>Dynamic Classes With Wildcards</title></head>
  <body>
    <div tw-variants="tw(bg-[*] text-[*])">
      <a class="{{input.nav.class}}">Nav Item</a>
    </div>
  </body>
</html>
"#
        .to_string(),
    };

    let compiled = engine
        .compile_template(&template, &language, &ReactiveWebOptions::default())
        .expect("compile template with wildcard tw-variants");

    assert!(compiled.needs_runtime_tailwind_rebuild);
    assert!(
        compiled
            .tailwind_variant_patterns
            .contains(&"bg-[*]".to_string())
    );
    assert!(
        compiled
            .tailwind_variant_patterns
            .contains(&"text-[*]".to_string())
    );
    assert!(
        compiled
            .html_ir
            .contains(".tw-bg-dyn{background-color:var(--tw-bg);}")
    );
    assert!(
        compiled
            .html_ir
            .contains(".tw-text-dyn{color:var(--tw-text);}")
    );
    assert!(
        !compiled
            .diagnostics
            .iter()
            .any(|d| d.code == "RWE_TAILWIND_DYNAMIC_CLASS_WARN")
    );

    let rendered = engine
        .render(
            &compiled,
            json!({
                "nav": {
                    "class": "bg-[#22c55e] text-[#111827] px-2 py-1 rounded"
                }
            }),
            &language,
            &RenderContext {
                route: "/projects/superadmin/default/design".to_string(),
                request_id: "req-dynamic-tailwind-variants".to_string(),
                metadata: json!({}),
            },
        )
        .expect("render template with wildcard tw-variants");

    assert!(rendered.html.contains("window.__ZEBFLOW_TW_DYN__"));
    assert!(rendered.html.contains("\"bg-[*]\""));
    assert!(rendered.html.contains("\"text-[*]\""));
}

#[test]
fn template_typed_class_notation_generates_tokens_and_resolves_allowed_option() {
    let engine = NoopReactiveWebEngine;
    let language = EchoLanguageEngine;
    let template = TemplateSource {
        id: "page.typed.class.notation".to_string(),
        source_path: None,
        markup: r#"
<!doctype html>
<html>
  <head><title>Typed Class Notation</title></head>
  <body>
    <div class="px-3 py-2 @{input.nav.design|[bg-gray-900 text-white]|[bg-gray-100 text-gray-700 hover:bg-gray-200]|default=[bg-gray-100 text-gray-700 hover:bg-gray-200]}">Menu</div>
    <div class="bg-@{input.intent|red|blue|green|default=blue}">Intent</div>
  </body>
</html>
"#
        .to_string(),
    };

    let compiled = engine
        .compile_template(&template, &language, &ReactiveWebOptions::default())
        .expect("compile template with typed class notation");

    assert!(
        !compiled
            .diagnostics
            .iter()
            .any(|d| d.code == "RWE_TAILWIND_DYNAMIC_CLASS_WARN")
    );
    assert!(compiled.html_ir.contains(".bg-gray-900{"));
    assert!(compiled.html_ir.contains(".bg-gray-100{"));
    assert!(compiled.html_ir.contains(".hover\\:bg-gray-200:hover{"));
    assert!(compiled.html_ir.contains(".bg-red{"));
    assert!(compiled.html_ir.contains(".bg-blue{"));
    assert!(compiled.html_ir.contains(".bg-green{"));

    let rendered = engine
        .render(
            &compiled,
            json!({
                "nav": {
                    "design": "bg-gray-100 text-gray-700 hover:bg-gray-200"
                },
                "intent": "unknown"
            }),
            &language,
            &RenderContext {
                route: "/projects/superadmin/default/design".to_string(),
                request_id: "req-typed-class".to_string(),
                metadata: json!({}),
            },
        )
        .expect("render template with typed class notation");

    assert!(
        rendered
            .html
            .contains("class=\"px-3 py-2 bg-gray-100 text-gray-700 hover:bg-gray-200\"")
    );
    assert!(rendered.html.contains("class=\"bg-blue\""));
}

struct EchoLanguageEngine;

impl LanguageEngine for EchoLanguageEngine {
    fn id(&self) -> &'static str {
        "language.echo.test"
    }

    fn parse(&self, module: &ModuleSource) -> Result<ProgramIr, LanguageError> {
        Ok(ProgramIr {
            source_id: module.id.clone(),
            kind: SourceKind::Tsx,
            body: json!({ "source": module.code }),
        })
    }

    fn compile(
        &self,
        ir: &ProgramIr,
        options: &CompileOptions,
    ) -> Result<CompiledProgram, LanguageError> {
        Ok(CompiledProgram {
            engine_id: self.id().to_string(),
            source_id: ir.source_id.clone(),
            artifact: serde_json::to_vec(&ir.body).map_err(|e| {
                LanguageError::new(
                    "ECHO_COMPILE",
                    format!("encode error for '{}': {e}", ir.source_id),
                )
            })?,
            metadata: json!({
                "target": options.target,
                "isFrontend": options.target == COMPILE_TARGET_FRONTEND,
            }),
        })
    }

    fn run(
        &self,
        _compiled: &CompiledProgram,
        input: Value,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionOutput, LanguageError> {
        Ok(ExecutionOutput {
            value: json!({
                "input": input,
                "metadata": ctx.metadata,
            }),
            trace: vec!["engine=language.echo.test".to_string()],
        })
    }
}

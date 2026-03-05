//! Axum demo integration for RWE templates.
//!
//! This module provides a ready-to-run router so templates can be viewed in a
//! browser during development.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use axum::Router;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Html;
use axum::routing::get;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::language::{LanguageEngine, NoopLanguageEngine};
use crate::rwe::{
    CompiledTemplate, ComponentOptions, ReactiveWebEngine, ReactiveWebOptions, RenderContext,
    ResourceAllowList, TemplateSource, resolve_engine_or_default,
};

const LUCIDE_SCRIPT_URL: &str = "https://unpkg.com/lucide@0.469.0/dist/umd/lucide.min.js";

/// Shared state for Axum demo routes.
#[derive(Clone)]
pub struct DemoAppState {
    rwe: Arc<dyn ReactiveWebEngine>,
    language: Arc<dyn LanguageEngine>,
    pages: Arc<BTreeMap<&'static str, CompiledTemplate>>,
    page_compile_us: Arc<BTreeMap<&'static str, u128>>,
}

#[derive(Debug, Deserialize)]
struct SeedQuery {
    seed: Option<i64>,
}

/// Builds a demo router with precompiled TSX pages.
pub fn build_demo_router() -> Result<Router, String> {
    let state = build_demo_state()?;
    Ok(Router::new()
        .route("/", get(route_home))
        .route("/rwe/comprehensive", get(route_rwe_comprehensive))
        .route("/rwe/dashboard", get(route_rwe_dashboard))
        .route("/rwe/frontpage", get(route_rwe_frontpage))
        .route("/recycling", get(route_recycling))
        .route("/showcase", get(route_showcase))
        .route("/todo", get(route_todo))
        .route("/list-hydration", get(route_list_hydration))
        .route("/state-sharing", get(route_state_sharing))
        .route("/blog", get(route_blog_home))
        .route("/blog/post-a", get(route_blog_post))
        .route("/blog/composed", get(route_blog_composed))
        .route("/rwe/lab", get(route_rwe_lab))
        .with_state(state))
}

fn build_demo_state() -> Result<DemoAppState, String> {
    let rwe_engine_id = std::env::var("ZEBFLOW_RWE_DEMO_ENGINE_ID").ok();
    let rwe: Arc<dyn ReactiveWebEngine> = resolve_engine_or_default(rwe_engine_id.as_deref());
    let language: Arc<dyn LanguageEngine> = Arc::new(NoopLanguageEngine);

    let options = options_with_components();
    let mut pages = BTreeMap::new();
    let mut page_compile_us = BTreeMap::new();
    let mut insert_compiled_page = |key: &'static str,
                                    page_id: &str,
                                    markup: &str,
                                    options: ReactiveWebOptions|
     -> Result<(), String> {
        let started = Instant::now();
        let compiled = compile_page(rwe.as_ref(), language.as_ref(), page_id, markup, options)?;
        page_compile_us.insert(key, started.elapsed().as_micros());
        pages.insert(key, compiled);
        Ok(())
    };

    insert_compiled_page(
        "home",
        "page.home",
        include_str!("demo/templates/pages/rwe-counter.tsx"),
        options.clone(),
    )?;
    insert_compiled_page(
        "recycling-nature",
        "page.recycling-nature",
        include_str!("demo/templates/pages/recycling-nature.tsx"),
        options.clone(),
    )?;
    insert_compiled_page(
        "zebflow-showcase",
        "page.zebflow-showcase",
        include_str!("demo/templates/pages/zebflow-showcase.tsx"),
        options.clone(),
    )?;
    insert_compiled_page(
        "todo",
        "page.todo",
        include_str!("demo/templates/pages/rwe-todo.tsx"),
        options.clone(),
    )?;
    insert_compiled_page(
        "rwe-comprehensive",
        "page.rwe-comprehensive",
        include_str!("demo/templates/pages/rwe-comprehensive.tsx"),
        options.clone(),
    )?;
    insert_compiled_page(
        "rwe-dashboard",
        "page.rwe-dashboard",
        include_str!("demo/templates/pages/rwe-dashboard.tsx"),
        options.clone(),
    )?;
    insert_compiled_page(
        "rwe-frontpage",
        "page.rwe-frontpage",
        include_str!("demo/templates/pages/rwe-frontpage.tsx"),
        options.clone(),
    )?;
    insert_compiled_page(
        "list-hydration",
        "page.list-hydration",
        include_str!("demo/templates/pages/list-hydration.tsx"),
        options.clone(),
    )?;
    insert_compiled_page(
        "state-sharing",
        "page.state-sharing",
        include_str!("demo/templates/pages/state-sharing-composed.tsx"),
        options.clone(),
    )?;
    insert_compiled_page(
        "blog-home",
        "page.blog-home",
        include_str!("demo/templates/pages/blog-home.tsx"),
        options.clone(),
    )?;
    insert_compiled_page(
        "blog-post",
        "page.blog-post",
        include_str!("demo/templates/pages/blog-post.tsx"),
        options.clone(),
    )?;
    insert_compiled_page(
        "blog-composed",
        "page.blog-composed",
        include_str!("demo/templates/pages/blog-home-composed.tsx"),
        options.clone(),
    )?;
    if let Err(err) = insert_compiled_page(
        "rwe-lab",
        "page.rwe-lab",
        include_str!("demo/templates/pages/rwe-lab.tsx"),
        options,
    ) {
        eprintln!("skip optional demo page 'rwe-lab': {err}");
    }

    Ok(DemoAppState {
        rwe,
        language,
        pages: Arc::new(pages),
        page_compile_us: Arc::new(page_compile_us),
    })
}

fn options_with_components() -> ReactiveWebOptions {
    let mut registry = BTreeMap::new();
    registry.insert(
        "BlogHeader".to_string(),
        include_str!("demo/templates/components/blog-header.tsx").to_string(),
    );
    registry.insert(
        "BlogHero".to_string(),
        include_str!("demo/templates/components/blog-hero.tsx").to_string(),
    );
    registry.insert(
        "TreeA".to_string(),
        include_str!("demo/templates/components/tree-a.tsx").to_string(),
    );
    registry.insert(
        "TreeB".to_string(),
        include_str!("demo/templates/components/tree-b.tsx").to_string(),
    );
    registry.insert(
        "TreeC".to_string(),
        include_str!("demo/templates/components/tree-c.tsx").to_string(),
    );
    registry.insert(
        "TreeD".to_string(),
        include_str!("demo/templates/components/tree-d.tsx").to_string(),
    );
    registry.insert(
        "TreeF".to_string(),
        include_str!("demo/templates/components/tree-f.tsx").to_string(),
    );

    ReactiveWebOptions {
        components: ComponentOptions {
            registry,
            strict: true,
        },
        allow_list: ResourceAllowList {
            scripts: vec![LUCIDE_SCRIPT_URL.to_string()],
            ..Default::default()
        },
        load_scripts: vec![LUCIDE_SCRIPT_URL.to_string()],
        processors: vec!["tailwind".to_string(), "markdown".to_string()],
        ..Default::default()
    }
}

fn compile_page(
    rwe: &dyn ReactiveWebEngine,
    language: &dyn LanguageEngine,
    id: &str,
    markup: &str,
    options: ReactiveWebOptions,
) -> Result<CompiledTemplate, String> {
    rwe.compile_template(
        &TemplateSource {
            id: id.to_string(),
            source_path: None,
            markup: markup.to_string(),
        },
        language,
        &options,
    )
    .map_err(|e| format!("compile failed for '{id}': {e}"))
}

fn render_page(
    state: &DemoAppState,
    page: &'static str,
    route: &str,
    input: Value,
) -> Result<String, String> {
    let compiled = state
        .pages
        .get(page)
        .ok_or_else(|| format!("compiled page '{page}' not found"))?;
    let compile_us = *state.page_compile_us.get(page).unwrap_or(&0u128);
    let render_started = Instant::now();
    let out = state
        .rwe
        .render(
            compiled,
            input,
            state.language.as_ref(),
            &RenderContext {
                route: route.to_string(),
                request_id: format!("req-{page}"),
                metadata: json!({ "demo": true }),
            },
        )
        .map_err(|e| format!("render failed for '{page}': {e}"))?;
    let render_us = render_started.elapsed().as_micros();
    Ok(compose_demo_document(out, page, route, compile_us, render_us))
}

fn compose_demo_document(
    out: crate::rwe::RenderOutput,
    page: &str,
    route: &str,
    compile_us: u128,
    render_us: u128,
) -> String {
    let mut html = String::new();
    html.push_str("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">");

    if let Some(css) = out.hydration_payload.get("css").and_then(|v| v.as_str())
        && !css.trim().is_empty()
    {
        html.push_str("<style>");
        html.push_str(css);
        html.push_str("</style>");
    }

    html.push_str("</head><body>");
    html.push_str(&out.html);

    for script in out.compiled_scripts {
        if script.content.trim().is_empty() {
            continue;
        }
        html.push_str("<script type=\"module\">");
        html.push_str(&script.content.replace("</script>", "<\\/script>"));
        html.push_str("</script>");
    }

    html.push_str(&format!(
        "<aside id=\"rwe-demo-timing\" style=\"position:fixed;right:12px;bottom:12px;z-index:99999;padding:8px 10px;border:1px solid #3f3f46;border-radius:8px;background:rgba(9,9,11,.88);color:#d4d4d8;font:11px/1.35 ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,monospace;max-width:360px\">\
            <div style=\"color:#a1a1aa\">{}</div>\
            <div style=\"margin-top:2px\">route: {}</div>\
            <div style=\"margin-top:4px\">compile: <strong>{}us</strong></div>\
            <div>render: <strong>{}us</strong></div>\
        </aside>",
        page, route, compile_us, render_us
    ));

    html.push_str("</body></html>");
    html
}

async fn route_home(
    State(state): State<DemoAppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    render_page(
        &state,
        "home",
        "/",
        json!({
            "initialCount": 0
        }),
    )
    .map(Html)
    .map_err(internal_error)
}

async fn route_rwe_comprehensive(
    State(state): State<DemoAppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    render_page(
        &state,
        "rwe-comprehensive",
        "/rwe/comprehensive",
        json!({
            "seedCount": 3
        }),
    )
    .map(Html)
    .map_err(internal_error)
}

async fn route_rwe_dashboard(
    State(state): State<DemoAppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    render_page(
        &state,
        "rwe-dashboard",
        "/rwe/dashboard",
        json!({
            "data": {
                "users": 15240,
                "revenue": 241820,
                "failures": 5,
                "latencyP95": 211,
                "pipelines": [
                    { "name": "ingest-users", "success": 99 },
                    { "name": "build-marts", "success": 97 },
                    { "name": "stream-events", "success": 94 },
                    { "name": "sync-crm", "success": 89 }
                ]
            }
        }),
    )
    .map(Html)
    .map_err(internal_error)
}

async fn route_rwe_frontpage(
    State(state): State<DemoAppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    render_page(
        &state,
        "rwe-frontpage",
        "/rwe/frontpage",
        json!({
            "pricing": {
                "monthly": 39,
                "yearly": 390
            }
        }),
    )
    .map(Html)
    .map_err(internal_error)
}

async fn route_todo(
    State(state): State<DemoAppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    render_page(
        &state,
        "todo",
        "/todo",
        json!({
            "total": 2,
            "items": [
                { "title": "Review pipeline contracts" },
                { "title": "Add regression tests" }
            ]
        }),
    )
        .map(Html)
        .map_err(internal_error)
}

async fn route_showcase(
    State(state): State<DemoAppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    render_page(
        &state,
        "zebflow-showcase",
        "/showcase",
        json!({
            "seo": {
                "title": "Zebflow - Deploy Once, Evolve Safely",
                "description": "Tiny automation engine with reactive web templates, pipelines, and secure script runtime."
            }
        }),
    )
    .map(Html)
    .map_err(internal_error)
}

async fn route_recycling(
    State(state): State<DemoAppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    render_page(
        &state,
        "recycling-nature",
        "/recycling",
        json!({
            "seo": {
                "title": "Recycling for Living Cities",
                "description": "A nature-inspired recycling guide with measurable local impact."
            },
            "hero": {
                "title": "Recycle Better, Restore Nature Faster",
                "subtitle": "A practical community model that reduces landfill pressure, protects rivers, and turns daily habits into visible environmental gains."
            },
            "metrics": {
                "plasticKg": 1840,
                "compostKg": 760,
                "actions": 1294
            },
            "recycleTips": [
                {
                    "id": 1,
                    "title": "Sort at the source",
                    "detail": "Keep paper, plastic, metal, and organic waste separated before disposal."
                },
                {
                    "id": 2,
                    "title": "Clean recyclables quickly",
                    "detail": "Rinse food residue from containers to avoid contamination in batch processing."
                },
                {
                    "id": 3,
                    "title": "Compost kitchen scraps",
                    "detail": "Turn fruit peels and coffee grounds into soil support for local urban gardens."
                }
            ]
        }),
    )
    .map(Html)
    .map_err(internal_error)
}

async fn route_list_hydration(
    State(state): State<DemoAppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    render_page(
        &state,
        "list-hydration",
        "/list-hydration",
        json!({
            "items": [
                { "id": 101, "title": "Alpha" },
                { "id": 102, "title": "Beta" },
                { "id": 103, "title": "Gamma" }
            ]
        }),
    )
    .map(Html)
    .map_err(internal_error)
}

async fn route_state_sharing(
    State(state): State<DemoAppState>,
    Query(query): Query<SeedQuery>,
) -> Result<Html<String>, (StatusCode, String)> {
    let seed = query.seed.unwrap_or(7);
    render_page(
        &state,
        "state-sharing",
        "/state-sharing",
        json!({
            "shared": { "seed": seed }
        }),
    )
    .map(Html)
    .map_err(internal_error)
}

async fn route_blog_home(
    State(state): State<DemoAppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    render_page(
        &state,
        "blog-home",
        "/blog",
        json!({
            "seo": {
                "title": "Zebflow Blog",
                "description": "Automation engineering posts",
                "canonical": "http://127.0.0.1:8787/blog"
            },
            "blog": {
                "title": "Zebflow Engineering",
                "tagline": "Build observable systems fast"
            },
            "posts": [
                { "id": 1, "title": "Post A", "excerpt": "A excerpt", "url": "/blog/post-a" },
                { "id": 2, "title": "Post B", "excerpt": "B excerpt", "url": "/blog/post-a" },
                { "id": 3, "title": "Post C", "excerpt": "C excerpt", "url": "/blog/post-a" }
            ]
        }),
    )
    .map(Html)
    .map_err(internal_error)
}

async fn route_blog_post(
    State(state): State<DemoAppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    render_page(
        &state,
        "blog-post",
        "/blog/post-a",
        json!({
            "post": {
                "seoTitle": "Post A | Zebflow",
                "seoDescription": "A deep-dive about RWE",
                "url": "http://127.0.0.1:8787/blog/post-a",
                "title": "Post A",
                "author": "Mala",
                "publishedAt": "2026-02-26",
                "summary": "Intro paragraph",
                "body": ["Line 1", "Line 2", "Line 3"]
            }
        }),
    )
    .map(Html)
    .map_err(internal_error)
}

async fn route_blog_composed(
    State(state): State<DemoAppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    render_page(
        &state,
        "blog-composed",
        "/blog/composed",
        json!({
            "seo": { "title": "Composed Blog" },
            "blog": { "title": "Zebflow", "tagline": "Composable web" },
            "hero": { "title": "Hero", "subtitle": "Sub" },
            "posts": [{ "title": "A" }]
        }),
    )
    .map(Html)
    .map_err(internal_error)
}

async fn route_rwe_lab(
    State(state): State<DemoAppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    render_page(
        &state,
        "rwe-lab",
        "/rwe/lab",
        json!({}),
    )
    .map(Html)
    .map_err(internal_error)
}

fn internal_error(msg: String) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, msg)
}

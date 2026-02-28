//! Axum demo integration for RWE templates.
//!
//! This module provides a ready-to-run router so templates can be viewed in a
//! browser during development.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::Router;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Html;
use axum::routing::get;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::language::{LanguageEngine, NoopLanguageEngine};
use crate::rwe::{
    CompiledTemplate, ComponentOptions, NoopReactiveWebEngine, ReactiveWebEngine,
    ReactiveWebOptions, RenderContext, ResourceAllowList, TemplateSource,
};

const LUCIDE_SCRIPT_URL: &str = "https://unpkg.com/lucide@0.469.0/dist/umd/lucide.min.js";

/// Shared state for Axum demo routes.
#[derive(Clone)]
pub struct DemoAppState {
    rwe: Arc<dyn ReactiveWebEngine>,
    language: Arc<dyn LanguageEngine>,
    pages: Arc<BTreeMap<&'static str, CompiledTemplate>>,
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
        .route("/recycling", get(route_recycling))
        .route("/showcase", get(route_showcase))
        .route("/todo", get(route_todo))
        .route("/list-hydration", get(route_list_hydration))
        .route("/state-sharing", get(route_state_sharing))
        .route("/blog", get(route_blog_home))
        .route("/blog/post-a", get(route_blog_post))
        .route("/blog/composed", get(route_blog_composed))
        .with_state(state))
}

fn build_demo_state() -> Result<DemoAppState, String> {
    let rwe: Arc<dyn ReactiveWebEngine> = Arc::new(NoopReactiveWebEngine);
    let language: Arc<dyn LanguageEngine> = Arc::new(NoopLanguageEngine);

    let options = options_with_components();
    let mut pages = BTreeMap::new();
    pages.insert(
        "home",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "page.home",
            include_str!("../../conventions/templates/pages/home.tsx"),
            options.clone(),
        )?,
    );
    pages.insert(
        "recycling-nature",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "page.recycling-nature",
            include_str!("../../conventions/templates/pages/recycling-nature.tsx"),
            options.clone(),
        )?,
    );
    pages.insert(
        "zebflow-showcase",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "page.zebflow-showcase",
            include_str!("../../conventions/templates/pages/zebflow-showcase.tsx"),
            options.clone(),
        )?,
    );
    pages.insert(
        "todo",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "page.todo",
            include_str!("../../conventions/templates/pages/todo.tsx"),
            options.clone(),
        )?,
    );
    pages.insert(
        "list-hydration",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "page.list-hydration",
            include_str!("../../conventions/templates/pages/list-hydration.tsx"),
            options.clone(),
        )?,
    );
    pages.insert(
        "state-sharing",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "page.state-sharing",
            include_str!("../../conventions/templates/pages/state-sharing-composed.tsx"),
            options.clone(),
        )?,
    );
    pages.insert(
        "blog-home",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "page.blog-home",
            include_str!("../../conventions/templates/pages/blog-home.tsx"),
            options.clone(),
        )?,
    );
    pages.insert(
        "blog-post",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "page.blog-post",
            include_str!("../../conventions/templates/pages/blog-post.tsx"),
            options.clone(),
        )?,
    );
    pages.insert(
        "blog-composed",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "page.blog-composed",
            include_str!("../../conventions/templates/pages/blog-home-composed.tsx"),
            options,
        )?,
    );

    Ok(DemoAppState {
        rwe,
        language,
        pages: Arc::new(pages),
    })
}

fn options_with_components() -> ReactiveWebOptions {
    let mut registry = BTreeMap::new();
    registry.insert(
        "BlogHeader".to_string(),
        include_str!("../../conventions/templates/components/blog-header.tsx").to_string(),
    );
    registry.insert(
        "BlogHero".to_string(),
        include_str!("../../conventions/templates/components/blog-hero.tsx").to_string(),
    );
    registry.insert(
        "TreeA".to_string(),
        include_str!("../../conventions/templates/components/tree-a.tsx").to_string(),
    );
    registry.insert(
        "TreeB".to_string(),
        include_str!("../../conventions/templates/components/tree-b.tsx").to_string(),
    );
    registry.insert(
        "TreeC".to_string(),
        include_str!("../../conventions/templates/components/tree-c.tsx").to_string(),
    );
    registry.insert(
        "TreeD".to_string(),
        include_str!("../../conventions/templates/components/tree-d.tsx").to_string(),
    );
    registry.insert(
        "TreeF".to_string(),
        include_str!("../../conventions/templates/components/tree-f.tsx").to_string(),
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
    Ok(out.html)
}

async fn route_home(
    State(state): State<DemoAppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    render_page(
        &state,
        "home",
        "/",
        json!({
            "home": {
                "title": "Zebflow Home",
                "description": "Axum demo page."
            }
        }),
    )
    .map(Html)
    .map_err(internal_error)
}

async fn route_todo(
    State(state): State<DemoAppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    render_page(&state, "todo", "/todo", json!({}))
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

fn internal_error(msg: String) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, msg)
}

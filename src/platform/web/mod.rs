//! Axum web layer for Zebflow platform flows, rendered via RWE templates.

use std::fs;
use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Form, Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header::CONTENT_TYPE, header::SET_COOKIE};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::language::{LanguageEngine, NoopLanguageEngine};
use crate::platform::error::PlatformError;
use crate::platform::model::{CreateProjectRequest, CreateUserRequest, LoginRequest};
use crate::platform::services::PlatformService;
use crate::rwe::{
    CompiledTemplate, NoopReactiveWebEngine, ReactiveWebEngine, ReactiveWebOptions, RenderContext,
    TemplateOptions, TemplateSource,
};

const BRAND_LOGO_SVG: &[u8] = include_bytes!("../../../docs/conventions/assets/branding/logo.svg");
const BRAND_LOGO_PNG: &[u8] = include_bytes!("../../../docs/conventions/assets/branding/logo.png");

/// Shared frontend render bundle (compiled templates + engines).
#[derive(Clone)]
struct PlatformFrontend {
    rwe: Arc<dyn ReactiveWebEngine>,
    language: Arc<dyn LanguageEngine>,
    pages: Arc<std::collections::BTreeMap<&'static str, CompiledTemplate>>,
}

/// Shared app state used by platform routes.
#[derive(Clone)]
pub struct PlatformAppState {
    /// Platform service graph.
    pub platform: Arc<PlatformService>,
    frontend: PlatformFrontend,
}

/// Builds Zebflow platform router.
pub fn router(platform: Arc<PlatformService>) -> Router {
    let frontend = build_frontend().unwrap_or_else(|err| {
        panic!("failed building platform frontend templates: {err}");
    });

    Router::new()
        .route("/", get(root_redirect))
        .route("/assets/branding/{asset}", get(branding_asset))
        .route("/login", get(login_page).post(login_submit))
        .route("/logout", post(logout_submit))
        .route("/home", get(home_page))
        .route("/home/projects/create", post(home_create_project_submit))
        .route("/projects/{owner}/{project}", get(project_root_page))
        .route(
            "/projects/{owner}/{project}/pipelines/{tab}",
            get(project_pipelines_page),
        )
        .route("/projects/{owner}/{project}/build", get(project_build_root_page))
        .route(
            "/projects/{owner}/{project}/build/{tab}",
            get(project_build_page),
        )
        .route("/projects/{owner}/{project}/studio", get(project_studio_redirect_page))
        .route("/projects/{owner}/{project}/studio/{tab}", get(project_studio_tab_redirect_page))
        .route(
            "/projects/{owner}/{project}/design",
            get(project_design_page),
        )
        .route(
            "/projects/{owner}/{project}/dashboard",
            get(project_dashboard_page),
        )
        .route(
            "/projects/{owner}/{project}/credentials",
            get(project_credentials_page),
        )
        .route(
            "/projects/{owner}/{project}/tables/connections",
            get(project_tables_connections_page),
        )
        .route(
            "/projects/{owner}/{project}/tables/connections/{connection}",
            get(project_table_connection_page),
        )
        .route("/projects/{owner}/{project}/files", get(project_files_page))
        .route("/projects/{owner}/{project}/todo", get(project_todo_page))
        .route(
            "/projects/{owner}/{project}/settings",
            get(project_settings_page),
        )
        .route("/api/meta", get(api_meta))
        .route("/api/users", get(api_list_users).post(api_create_user))
        .route(
            "/api/users/{owner}/projects",
            get(api_list_projects).post(api_create_project),
        )
        .with_state(PlatformAppState { platform, frontend })
}

fn build_frontend() -> Result<PlatformFrontend, PlatformError> {
    let rwe: Arc<dyn ReactiveWebEngine> = Arc::new(NoopReactiveWebEngine);
    let language: Arc<dyn LanguageEngine> = Arc::new(NoopLanguageEngine);
    let template_root = platform_template_root();

    let options = ReactiveWebOptions {
        templates: TemplateOptions {
            template_root: Some(template_root.clone()),
        style_entries: Vec::new(),
        },
        processors: vec!["tailwind".to_string()],
        ..Default::default()
    };

    let mut pages = std::collections::BTreeMap::new();

    pages.insert(
        "platform-login",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "platform.login",
            &template_root,
            "pages/platform-login.tsx",
            options.clone(),
        )?,
    );

    pages.insert(
        "platform-home",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "platform.home",
            &template_root,
            "pages/platform-home.tsx",
            options.clone(),
        )?,
    );

    pages.insert(
        "platform-project-pipelines",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "platform.project.pipelines",
            &template_root,
            "pages/platform-project-pipelines.tsx",
            options.clone(),
        )?,
    );

    pages.insert(
        "platform-project-section",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "platform.project.section",
            &template_root,
            "pages/platform-project-section.tsx",
            options.clone(),
        )?,
    );

    pages.insert(
        "platform-project-studio",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "platform.project.studio",
            &template_root,
            "pages/platform-project-studio.tsx",
            options.clone(),
        )?,
    );

    pages.insert(
        "platform-project-tables",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "platform.project.tables",
            &template_root,
            "pages/platform-project-tables.tsx",
            options.clone(),
        )?,
    );

    pages.insert(
        "platform-project-table-connection",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "platform.project.table_connection",
            &template_root,
            "pages/platform-project-table-connection.tsx",
            options,
        )?,
    );

    Ok(PlatformFrontend {
        rwe,
        language,
        pages: Arc::new(pages),
    })
}

fn compile_page(
    rwe: &dyn ReactiveWebEngine,
    language: &dyn LanguageEngine,
    id: &str,
    template_root: &FsPath,
    relative_path: &str,
    options: ReactiveWebOptions,
) -> Result<CompiledTemplate, PlatformError> {
    let page_path = template_root.join(relative_path);
    let markup = fs::read_to_string(&page_path).map_err(|err| {
        PlatformError::new(
            "PLATFORM_RWE_SOURCE_READ",
            format!("failed reading '{}': {err}", page_path.display()),
        )
    })?;
    rwe.compile_template(
        &TemplateSource {
            id: id.to_string(),
            source_path: Some(page_path),
            markup,
        },
        language,
        &options,
    )
    .map_err(|e| PlatformError::new("PLATFORM_RWE_COMPILE", e.to_string()))
}

fn platform_template_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/platform/web/templates")
}

fn render_page(
    state: &PlatformAppState,
    page: &'static str,
    route: &str,
    input: Value,
) -> Result<String, PlatformError> {
    let compiled = state
        .frontend
        .pages
        .get(page)
        .ok_or_else(|| PlatformError::new("PLATFORM_RWE_PAGE_MISSING", page))?;

    let out = state
        .frontend
        .rwe
        .render(
            compiled,
            input,
            state.frontend.language.as_ref(),
            &RenderContext {
                route: route.to_string(),
                request_id: format!("zebflow-{page}"),
                metadata: json!({"zebflow": true}),
            },
        )
        .map_err(|e| PlatformError::new("PLATFORM_RWE_RENDER", e.to_string()))?;

    Ok(out.html)
}

async fn root_redirect() -> Redirect {
    Redirect::to("/login")
}

#[derive(Debug, Clone, Default, Deserialize)]
struct PipelineRegistryQuery {
    path: Option<String>,
}

async fn branding_asset(Path(asset): Path<String>) -> Response {
    match asset.as_str() {
        "logo.svg" => asset_response("image/svg+xml; charset=utf-8", BRAND_LOGO_SVG),
        "logo.png" => asset_response("image/png", BRAND_LOGO_PNG),
        _ => (StatusCode::NOT_FOUND, "asset not found").into_response(),
    }
}

fn asset_response(content_type: &'static str, bytes: &[u8]) -> Response {
    let mut resp = Response::new(Body::from(bytes.to_vec()));
    *resp.status_mut() = StatusCode::OK;
    if let Ok(v) = HeaderValue::from_str(content_type) {
        resp.headers_mut().insert(CONTENT_TYPE, v);
    }
    resp
}

async fn login_page(State(state): State<PlatformAppState>) -> Response {
    match render_login_page(&state, None, StatusCode::OK) {
        Ok(resp) => resp,
        Err(err) => internal_error(err),
    }
}

fn render_login_page(
    state: &PlatformAppState,
    error: Option<&str>,
    status: StatusCode,
) -> Result<Response, PlatformError> {
    let html = render_page(
        state,
        "platform-login",
        "/login",
        json!({
            "seo": {
                "title": "Zebflow Platform Login",
                "description": "Login page for Zebflow platform"
            },
            "error": error.unwrap_or(""),
            "default_identifier": state.platform.config.default_owner,
        }),
    )?;
    Ok((status, Html(html)).into_response())
}

async fn login_submit(
    State(state): State<PlatformAppState>,
    Form(req): Form<LoginRequest>,
) -> Response {
    match state.platform.auth.login(&req.identifier, &req.password) {
        Ok(Some(session)) => {
            let mut resp = Redirect::to("/home").into_response();
            let cookie = format!(
                "zebflow_session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=86400",
                session.owner
            );
            if let Ok(v) = HeaderValue::from_str(&cookie) {
                resp.headers_mut().insert(SET_COOKIE, v);
            }
            resp
        }
        Ok(None) => {
            match render_login_page(
                &state,
                Some("invalid credentials"),
                StatusCode::UNAUTHORIZED,
            ) {
                Ok(resp) => resp,
                Err(err) => internal_error(err),
            }
        }
        Err(err) => internal_error(err),
    }
}

async fn logout_submit() -> Response {
    let mut resp = Redirect::to("/login").into_response();
    if let Ok(v) =
        HeaderValue::from_str("zebflow_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")
    {
        resp.headers_mut().insert(SET_COOKIE, v);
    }
    resp
}

async fn home_page(State(state): State<PlatformAppState>, headers: HeaderMap) -> Response {
    let Some(owner) = session_owner(&headers) else {
        return Redirect::to("/login").into_response();
    };

    match state.platform.projects.list_projects(&owner) {
        Ok(items) => {
            let projects = items
                .into_iter()
                .map(|item| {
                    let item_owner = if item.owner.trim().is_empty() {
                        owner.clone()
                    } else {
                        item.owner.clone()
                    };
                    json!({
                        "owner": item_owner,
                        "project": item.project,
                        "title": item.title,
                        "path": format!("/projects/{}/{}", item_owner, item.project),
                    })
                })
                .collect::<Vec<_>>();
            match render_page(
                &state,
                "platform-home",
                "/home",
                json!({
                    "seo": {
                        "title": "Zebflow Platform Home",
                        "description": "Project list"
                    },
                    "owner": owner,
                    "projects": projects,
                }),
            ) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Err(err) => internal_error(err),
    }
}

async fn home_create_project_submit(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Form(req): Form<CreateProjectRequest>,
) -> Response {
    let Some(owner) = session_owner(&headers) else {
        return Redirect::to("/login").into_response();
    };

    match state
        .platform
        .projects
        .create_or_update_project(&owner, &req)
    {
        Ok(_) => Redirect::to("/home").into_response(),
        Err(err) => internal_error(err),
    }
}

async fn project_root_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Query(query): Query<PipelineRegistryQuery>,
) -> Response {
    render_project_pipelines_with_tab(
        state,
        headers,
        owner,
        project,
        "registry",
        query.path.as_deref().unwrap_or("/"),
    )
    .await
}

async fn project_pipelines_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, tab)): Path<(String, String, String)>,
    Query(query): Query<PipelineRegistryQuery>,
) -> Response {
    if tab == "editor" {
        return Redirect::to(&format!("/projects/{owner}/{project}/build/templates")).into_response();
    }

    render_project_pipelines_with_tab(
        state,
        headers,
        owner,
        project,
        &tab,
        query.path.as_deref().unwrap_or("/"),
    )
    .await
}

async fn project_build_root_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    render_project_build_with_tab(state, headers, owner, project, "templates").await
}

async fn project_build_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, tab)): Path<(String, String, String)>,
) -> Response {
    render_project_build_with_tab(state, headers, owner, project, &tab).await
}

async fn project_studio_redirect_page(
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    Redirect::to(&format!("/projects/{owner}/{project}/build/templates")).into_response()
}

async fn project_studio_tab_redirect_page(
    Path((owner, project, tab)): Path<(String, String, String)>,
) -> Response {
    Redirect::to(&format!("/projects/{owner}/{project}/build/{tab}")).into_response()
}

async fn render_project_pipelines_with_tab(
    state: PlatformAppState,
    headers: HeaderMap,
    owner: String,
    project: String,
    tab: &str,
    registry_path: &str,
) -> Response {
    let Some(session_owner) = session_owner(&headers) else {
        return Redirect::to("/login").into_response();
    };
    if session_owner != owner {
        return (StatusCode::FORBIDDEN, Html("forbidden".to_string())).into_response();
    }

    let is_registry = tab == "registry";
    let tab_payload = if is_registry {
        Some((
            "registry",
            "Pipeline Registry",
            "Browse pipelines by project path.",
            Vec::new(),
        ))
    } else {
        pipeline_tab_payload(tab)
    };
    let Some((tab_key, tab_title, tab_desc, items)) = tab_payload else {
        return (
            StatusCode::NOT_FOUND,
            Html("pipeline tab not found".to_string()),
        )
            .into_response();
    };

    match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => {
            let nav = nav_classes(&owner, &project, "pipelines", Some(tab_key));
            let route = format!("/projects/{owner}/{project}/pipelines/{tab_key}");
            let registry = if is_registry {
                match state.platform.projects.list_pipeline_registry(
                    &owner,
                    &project,
                    registry_path,
                    &route,
                ) {
                    Ok(listing) => json!({
                        "current_path": listing.current_path,
                        "breadcrumbs": listing.breadcrumbs,
                        "folders": listing.folders,
                        "pipelines": listing.pipelines,
                        "has_folders": !listing.folders.is_empty(),
                        "has_pipelines": !listing.pipelines.is_empty(),
                    }),
                    Err(err) => return internal_error(err),
                }
            } else {
                json!({
                    "current_path": "/",
                    "breadcrumbs": [],
                    "folders": [],
                    "pipelines": []
                })
            };
            let input = json!({
                "seo": {
                    "title": format!("{} - Pipelines", info.title),
                    "description": "Pipeline management"
                },
                "owner": info.owner,
                "project": info.project,
                "title": info.title,
                "project_href": format!("/projects/{owner}/{project}"),
                "current_menu": format!("Pipelines / {tab_title}"),
                "page_title": tab_title,
                "page_subtitle": tab_desc,
                "pipeline_items": items,
                "is_registry": is_registry,
                "is_non_registry": !is_registry,
                "registry": registry,
                "nav": nav,
            });

            match render_page(&state, "platform-project-pipelines", &route, input) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn render_project_build_with_tab(
    state: PlatformAppState,
    headers: HeaderMap,
    owner: String,
    project: String,
    tab: &str,
) -> Response {
    let Some(session_owner) = session_owner(&headers) else {
        return Redirect::to("/login").into_response();
    };
    if session_owner != owner {
        return (StatusCode::FORBIDDEN, Html("forbidden".to_string())).into_response();
    }

    let Some((tab_key, tab_title, tab_desc, action_label, items)) = build_tab_payload(tab) else {
        return (
            StatusCode::NOT_FOUND,
            Html("build tab not found".to_string()),
        )
            .into_response();
    };

    match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => {
            let nav = nav_classes(&owner, &project, "build", Some(tab_key));
            let route = format!("/projects/{owner}/{project}/build/{tab_key}");
            let input = json!({
                "seo": {
                    "title": format!("{} - {}", info.title, tab_title),
                    "description": tab_desc
                },
                "owner": info.owner,
                "project": info.project,
                "title": info.title,
                "project_href": format!("/projects/{owner}/{project}"),
                "current_menu": format!("Build / {tab_title}"),
                "page_title": tab_title,
                "page_subtitle": tab_desc,
                "items": items,
                "primary_action": {
                    "href": route,
                    "label": action_label
                },
                "nav": nav,
            });
            match render_page(&state, "platform-project-studio", &route, input) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn project_design_page(
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    Redirect::to(&format!("/projects/{owner}/{project}/build/templates")).into_response()
}

async fn project_dashboard_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    render_section_page(
        state,
        headers,
        owner,
        project,
        "dashboard",
        "Dashboard",
        "Observe runtime health, pipeline throughput, and execution traces.",
        vec![
            json!({"title":"Pipeline Throughput","description":"Run volume and status in real-time."}),
            json!({"title":"Execution Latency","description":"P50/P95 duration across workflows."}),
        ],
    )
    .await
}

async fn project_credentials_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    render_section_page(
        state,
        headers,
        owner,
        project,
        "credentials",
        "Credentials",
        "Store and manage API keys, DB credentials, and runtime secrets.",
        vec![
            json!({"title":"LLM Providers","description":"OpenAI, OpenRouter, Anthropic connectors."}),
            json!({"title":"Database Auth","description":"Postgres, MySQL, analytics endpoints."}),
        ],
    )
    .await
}

async fn project_tables_connections_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    let Some(session_owner) = session_owner(&headers) else {
        return Redirect::to("/login").into_response();
    };
    if session_owner != owner {
        return (StatusCode::FORBIDDEN, Html("forbidden".to_string())).into_response();
    }

    match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => {
            let nav = nav_classes(&owner, &project, "tables", Some("connections"));
            let route = format!("/projects/{owner}/{project}/tables/connections");
            let input = json!({
                "seo": {
                    "title": format!("{} - Tables", info.title),
                    "description": "Connections and table browser"
                },
                "owner": info.owner,
                "project": info.project,
                "title": info.title,
                "project_href": format!("/projects/{owner}/{project}"),
                "connections": [
                    {
                        "id": "main_pg",
                        "name": "Main Postgres",
                        "driver": "postgres",
                        "path": format!("/projects/{owner}/{project}/tables/connections/main_pg")
                    },
                    {
                        "id": "warehouse",
                        "name": "Warehouse",
                        "driver": "postgres",
                        "path": format!("/projects/{owner}/{project}/tables/connections/warehouse")
                    }
                ],
                "nav": nav,
            });
            match render_page(&state, "platform-project-tables", &route, input) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn project_table_connection_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection)): Path<(String, String, String)>,
) -> Response {
    let Some(session_owner) = session_owner(&headers) else {
        return Redirect::to("/login").into_response();
    };
    if session_owner != owner {
        return (StatusCode::FORBIDDEN, Html("forbidden".to_string())).into_response();
    }

    match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => {
            let nav = nav_classes(&owner, &project, "tables", Some("connections"));
            let route = format!("/projects/{owner}/{project}/tables/connections/{connection}");
            let input = json!({
                "seo": {
                    "title": format!("{} - Tables {}", info.title, connection),
                    "description": "Connection table browser"
                },
                "owner": info.owner,
                "project": info.project,
                "title": info.title,
                "project_href": format!("/projects/{owner}/{project}"),
                "connection": connection,
                "tables": [
                    {"name":"users","rows": 1240, "updated":"2026-02-26"},
                    {"name":"projects","rows": 230, "updated":"2026-02-26"},
                    {"name":"pipelines","rows": 842, "updated":"2026-02-26"},
                    {"name":"run_logs","rows": 12031, "updated":"2026-02-26"}
                ],
                "nav": nav,
            });
            match render_page(&state, "platform-project-table-connection", &route, input) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn project_files_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    render_section_page(
        state,
        headers,
        owner,
        project,
        "files",
        "Files",
        "Git-sync friendly project files and assets.",
        vec![
            json!({"title":"File Browser","description":"Browse templates, scripts, and static assets."}),
            json!({"title":"Git Sync","description":"Track and sync project files with git repositories."}),
        ],
    )
    .await
}

async fn project_todo_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    render_section_page(
        state,
        headers,
        owner,
        project,
        "todo",
        "Todo",
        "Collaborative notes and task lists for project delivery.",
        vec![
            json!({"title":"Backlog","description":"Track pending improvements and fixes."}),
            json!({"title":"Sprint Tasks","description":"Focus tasks tied to current release cycle."}),
        ],
    )
    .await
}

async fn project_settings_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    render_section_page(
        state,
        headers,
        owner,
        project,
        "settings",
        "Settings",
        "Project policies, adapters, and runtime defaults.",
        vec![
            json!({"title":"Runtime Policy","description":"Timeout, retries, and execution policy."}),
            json!({"title":"Environment","description":"Project-level variables and secrets policy."}),
        ],
    )
    .await
}

async fn render_section_page(
    state: PlatformAppState,
    headers: HeaderMap,
    owner: String,
    project: String,
    section_key: &str,
    section_title: &str,
    section_desc: &str,
    cards: Vec<Value>,
) -> Response {
    let Some(session_owner) = session_owner(&headers) else {
        return Redirect::to("/login").into_response();
    };
    if session_owner != owner {
        return (StatusCode::FORBIDDEN, Html("forbidden".to_string())).into_response();
    }

    match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => {
            let nav = nav_classes(&owner, &project, section_key, None);
            let route = format!("/projects/{owner}/{project}/{section_key}");
            let input = json!({
                "seo": {
                    "title": format!("{} - {}", info.title, section_title),
                    "description": section_desc
                },
                "owner": info.owner,
                "project": info.project,
                "title": info.title,
                "project_href": format!("/projects/{owner}/{project}"),
                "current_menu": section_title,
                "page_title": section_title,
                "page_subtitle": section_desc,
                "cards": cards,
                "nav": nav,
            });
            match render_page(&state, "platform-project-section", &route, input) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response(),
        Err(err) => internal_error(err),
    }
}

fn pipeline_tab_payload(
    tab: &str,
) -> Option<(&'static str, &'static str, &'static str, Vec<Value>)> {
    match tab {
        "webhooks" => Some((
            "webhooks",
            "Webhook Pipelines",
            "Inbound HTTP triggers mapped to project pipelines.",
            vec![
                json!({"name":"POST /orders","description":"Trigger order fulfillment"}),
                json!({"name":"POST /users","description":"Trigger onboarding flow"}),
            ],
        )),
        "schedules" => Some((
            "schedules",
            "Schedule Pipelines",
            "Cron-based and interval-based recurring jobs.",
            vec![
                json!({"name":"0 * * * *","description":"Hourly analytics rollup"}),
                json!({"name":"*/5 * * * *","description":"5-minute health sweep"}),
            ],
        )),
        "functions" => Some((
            "functions",
            "Function Pipelines",
            "Callable in-house functions for reuse across workflows.",
            vec![
                json!({"name":"fn.normalize_event","description":"Normalize event schema"}),
                json!({"name":"fn.score_customer","description":"Compute customer score"}),
            ],
        )),
        _ => None,
    }
}

fn build_tab_payload(
    tab: &str,
) -> Option<(&'static str, &'static str, &'static str, &'static str, Vec<Value>)> {
    match tab {
        "templates" => Some((
            "templates",
            "Templates",
            "Route-bound TSX pages, shared components, and template-root styles for the current project.",
            "Open Templates",
            vec![
                json!({"title":"Page Routes","description":"Server-rendered TSX page entrypoints mapped by the project web layer."}),
                json!({"title":"Shared Components","description":"Reusable TSX modules imported from the template root."}),
                json!({"title":"Theme Styles","description":"main.css defines project typography, theme tokens, and global surfaces."}),
            ],
        )),
        "assets" => Some((
            "assets",
            "Assets",
            "Project-owned images, icons, brand media, and static resources shipped with the web runtime.",
            "Manage Assets",
            vec![
                json!({"title":"Brand Media","description":"Logos, illustrations, and identity assets consumed by templates."}),
                json!({"title":"Static Resources","description":"Images, downloads, and shared files served with the project."}),
                json!({"title":"Design Inputs","description":"Reference material kept close to the shipped frontend surface."}),
            ],
        )),
        "schema" => Some((
            "schema",
            "Schema",
            "Structured project definitions such as ERD, app design documents, use-case maps, and data contracts.",
            "Open Schema",
            vec![
                json!({"title":"App Design Docs","description":"High-level system and interaction definitions for the project."}),
                json!({"title":"ERD & Data Models","description":"Database structure and cross-entity relationships."}),
                json!({"title":"Use Cases & Concepts","description":"Implementation notes, use cases, and conceptual design artifacts."}),
            ],
        )),
        _ => None,
    }
}

fn nav_classes(owner: &str, project: &str, main: &str, pipeline_sub: Option<&str>) -> Value {
    let pipelines_base = format!("/projects/{owner}/{project}/pipelines");

    json!({
        "title": "Project Menu",
        "links": {
            "pipelines_registry": format!("{pipelines_base}/registry?path=/"),
            "pipelines_webhooks": format!("{pipelines_base}/webhooks"),
            "pipelines_schedules": format!("{pipelines_base}/schedules"),
            "pipelines_functions": format!("{pipelines_base}/functions"),
            "build_templates": format!("/projects/{owner}/{project}/build/templates"),
            "build_assets": format!("/projects/{owner}/{project}/build/assets"),
            "build_schema": format!("/projects/{owner}/{project}/build/schema"),
            "dashboard": format!("/projects/{owner}/{project}/dashboard"),
            "credentials": format!("/projects/{owner}/{project}/credentials"),
            "tables_connections": format!("/projects/{owner}/{project}/tables/connections"),
            "files": format!("/projects/{owner}/{project}/files"),
            "todo": format!("/projects/{owner}/{project}/todo"),
            "settings": format!("/projects/{owner}/{project}/settings"),
        },
        "classes": {
            "pipelines": if main == "pipelines" { "is-active" } else { "" },
            "build": if main == "build" { "is-active" } else { "" },
            "dashboard": if main == "dashboard" { "is-active" } else { "" },
            "credentials": if main == "credentials" { "is-active" } else { "" },
            "tables": if main == "tables" { "is-active" } else { "" },
            "files": if main == "files" { "is-active" } else { "" },
            "todo": if main == "todo" { "is-active" } else { "" },
            "settings": if main == "settings" { "is-active" } else { "" },
            "pipeline_registry": if pipeline_sub == Some("registry") { "is-active" } else { "" },
            "pipeline_webhooks": if pipeline_sub == Some("webhooks") { "is-active" } else { "" },
            "pipeline_schedules": if pipeline_sub == Some("schedules") { "is-active" } else { "" },
            "pipeline_functions": if pipeline_sub == Some("functions") { "is-active" } else { "" },
            "build_templates": if main == "build" && pipeline_sub == Some("templates") { "is-active" } else { "" },
            "build_assets": if main == "build" && pipeline_sub == Some("assets") { "is-active" } else { "" },
            "build_schema": if main == "build" && pipeline_sub == Some("schema") { "is-active" } else { "" },
            "table_connections": if main == "tables" { "is-active" } else { "" },
        }
    })
}

async fn api_meta(State(state): State<PlatformAppState>) -> Response {
    Json(json!({
        "ok": true,
        "data_adapter": state.platform.data.id(),
        "file_adapter": state.platform.file.id(),
        "project_data_factory": state.platform.project_data.id(),
        "project_data_engines": state.platform.project_data.enabled_engines(),
    }))
    .into_response()
}

async fn api_list_users(State(state): State<PlatformAppState>) -> Response {
    match state.platform.users.list_users() {
        Ok(items) => Json(json!({"ok": true, "items": items})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_create_user(
    State(state): State<PlatformAppState>,
    Json(req): Json<CreateUserRequest>,
) -> Response {
    match state.platform.users.create_or_update_user(&req) {
        Ok(user) => Json(json!({"ok": true, "user": user})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_list_projects(
    State(state): State<PlatformAppState>,
    Path(owner): Path<String>,
) -> Response {
    match state.platform.projects.list_projects(&owner) {
        Ok(items) => Json(json!({"ok": true, "items": items})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_create_project(
    State(state): State<PlatformAppState>,
    Path(owner): Path<String>,
    Json(req): Json<CreateProjectRequest>,
) -> Response {
    match state
        .platform
        .projects
        .create_or_update_project(&owner, &req)
    {
        Ok((project, layout)) => {
            Json(json!({"ok": true, "project": project, "layout": layout})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

fn session_owner(headers: &HeaderMap) -> Option<String> {
    let cookie = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    cookie.split(';').map(str::trim).find_map(|part| {
        part.strip_prefix("zebflow_session=")
            .map(ToString::to_string)
    })
}

fn internal_error(err: PlatformError) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
    )
        .into_response()
}

//! Axum web layer for Zebflow platform flows, rendered via RWE templates.

mod embedded;

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
use crate::platform::model::{
    CreateProjectRequest, CreateUserRequest, LoginRequest, ProjectAccessSubject,
    ProjectCapability, TemplateCompileRequest, TemplateCompileResponse, TemplateCreateRequest,
    TemplateDiagnostic, TemplateMoveRequest, TemplateSaveRequest,
};
use crate::platform::services::PlatformService;
use crate::rwe::{
    CompiledTemplate, NoopReactiveWebEngine, ReactiveWebEngine, ReactiveWebOptions, RenderContext,
    TemplateOptions, TemplateSource,
};
use embedded::{PLATFORM_TEMPLATE_ASSETS, platform_library_asset};

const BRAND_LOGO_SVG: &[u8] = include_bytes!("../../../docs/conventions/assets/branding/logo.svg");
const BRAND_LOGO_PNG: &[u8] = include_bytes!("../../../docs/conventions/assets/branding/logo.png");
const PLATFORM_TEMPLATE_EDITOR_JS: &str = include_str!("runtime/template-editor.mjs");

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
    let frontend = build_frontend(&platform.config.data_root).unwrap_or_else(|err| {
        panic!("failed building platform frontend templates: {err}");
    });

    Router::new()
        .route("/", get(root_redirect))
        .route("/assets/branding/{asset}", get(branding_asset))
        .route("/assets/platform/{asset}", get(platform_asset))
        .route("/assets/libraries/{*path}", get(library_asset))
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
        .route(
            "/projects/{owner}/{project}/settings/web-libraries",
            get(project_settings_web_libraries_page),
        )
        .route(
            "/projects/{owner}/{project}/settings/nodes",
            get(project_settings_nodes_page),
        )
        .route("/api/meta", get(api_meta))
        .route("/api/users", get(api_list_users).post(api_create_user))
        .route(
            "/api/users/{owner}/projects",
            get(api_list_projects).post(api_create_project),
        )
        .route(
            "/api/projects/{owner}/{project}/templates/workspace",
            get(api_template_workspace),
        )
        .route(
            "/api/projects/{owner}/{project}/templates/file",
            get(api_template_file).put(api_template_save).delete(api_template_delete),
        )
        .route(
            "/api/projects/{owner}/{project}/templates/create",
            post(api_template_create),
        )
        .route(
            "/api/projects/{owner}/{project}/templates/move",
            post(api_template_move),
        )
        .route(
            "/api/projects/{owner}/{project}/templates/git-status",
            get(api_template_git_status),
        )
        .route(
            "/api/projects/{owner}/{project}/templates/diagnostics",
            post(api_template_diagnostics),
        )
        .with_state(PlatformAppState { platform, frontend })
}

fn build_frontend(data_root: &FsPath) -> Result<PlatformFrontend, PlatformError> {
    let rwe: Arc<dyn ReactiveWebEngine> = Arc::new(NoopReactiveWebEngine);
    let language: Arc<dyn LanguageEngine> = Arc::new(NoopLanguageEngine);
    let template_root = materialize_platform_template_root(data_root)?;

    let options = ReactiveWebOptions {
        load_scripts: vec!["/assets/platform/*".to_string()],
        allow_list: crate::rwe::ResourceAllowList {
            scripts: vec!["/assets/platform/*".to_string()],
            urls: vec!["/assets/platform/*".to_string()],
            ..Default::default()
        },
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
        "platform-project-build-templates",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "platform.project.build.templates",
            &template_root,
            "pages/platform-project-build-templates.tsx",
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

fn materialize_platform_template_root(data_root: &FsPath) -> Result<PathBuf, PlatformError> {
    let root = data_root.join("platform").join("embedded").join("templates");
    for asset in PLATFORM_TEMPLATE_ASSETS {
        let full = root.join(asset.path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(full, asset.bytes)?;
    }
    Ok(root)
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

#[derive(Debug, Clone, Default, Deserialize)]
struct TemplateWorkspaceQuery {
    file: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TemplatePathQuery {
    path: Option<String>,
}

async fn branding_asset(Path(asset): Path<String>) -> Response {
    match asset.as_str() {
        "logo.svg" => asset_response("image/svg+xml; charset=utf-8", BRAND_LOGO_SVG),
        "logo.png" => asset_response("image/png", BRAND_LOGO_PNG),
        _ => (StatusCode::NOT_FOUND, "asset not found").into_response(),
    }
}

async fn platform_asset(Path(asset): Path<String>) -> Response {
    match asset.as_str() {
        "template-editor.mjs" => {
            asset_response("text/javascript; charset=utf-8", PLATFORM_TEMPLATE_EDITOR_JS.as_bytes())
        }
        _ => (StatusCode::NOT_FOUND, "asset not found").into_response(),
    }
}

async fn library_asset(Path(path): Path<String>) -> Response {
    let normalized = path.trim_start_matches('/').replace('\\', "/");
    match platform_library_asset(&normalized) {
        Some(bytes) => asset_response(content_type_for_path(FsPath::new(&normalized)), bytes),
        None => (StatusCode::NOT_FOUND, "asset not found").into_response(),
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
    Query(query): Query<TemplateWorkspaceQuery>,
) -> Response {
    render_project_build_with_tab(state, headers, owner, project, "templates", query).await
}

async fn project_build_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, tab)): Path<(String, String, String)>,
    Query(query): Query<TemplateWorkspaceQuery>,
) -> Response {
    render_project_build_with_tab(state, headers, owner, project, &tab, query).await
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
    if let Err(response) = require_project_page_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return response;
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
    query: TemplateWorkspaceQuery,
) -> Response {
    let capability = if tab == "templates" {
        ProjectCapability::TemplatesRead
    } else {
        ProjectCapability::ProjectRead
    };
    if let Err(response) = require_project_page_capability(
        &state,
        &headers,
        &owner,
        &project,
        capability,
    ) {
        return response;
    }

    if tab == "templates" {
        return render_project_build_templates(state, owner, project, query).await;
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

async fn render_project_build_templates(
    state: PlatformAppState,
    owner: String,
    project: String,
    query: TemplateWorkspaceQuery,
) -> Response {
    match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => {
            let workspace = match state.platform.projects.list_template_workspace(&owner, &project) {
                Ok(workspace) => workspace,
                Err(err) => return internal_error(err),
            };

            let selected_rel = query
                .file
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .map(str::to_string)
                .or_else(|| workspace.default_file.clone());

            let selected_file = selected_rel
                .as_deref()
                .map(|rel| state.platform.projects.read_template_payload(&owner, &project, rel))
                .transpose();
            let selected_file = match selected_file {
                Ok(file) => file,
                Err(err) => return internal_error(err),
            };

            let selected_file = selected_file.unwrap_or_else(|| crate::platform::model::TemplateFilePayload {
                rel_path: "pages/home.tsx".to_string(),
                name: "home.tsx".to_string(),
                file_kind: "page".to_string(),
                content: String::new(),
                line_count: 1,
                is_protected: false,
            });
            let selected_rel = selected_file.rel_path.clone();
            let selected_name = selected_file.name.clone();
            let selected_kind = selected_file.file_kind.clone();
            let selected_lines = selected_file.line_count;
            let selected_content = selected_file.content.clone();

            let template_items = workspace
                .items
                .into_iter()
                .map(|item| {
                    let href = if item.kind == "file" {
                        format!(
                            "/projects/{owner}/{project}/build/templates?file={}",
                            item.rel_path
                        )
                    } else {
                        String::new()
                    };
                    json!({
                        "name": item.name,
                        "rel_path": item.rel_path,
                        "kind": item.kind,
                        "file_kind": item.file_kind,
                        "is_protected": item.is_protected,
                        "indent_px": 12 + (item.depth * 14),
                        "href": href,
                        "is_file": item.kind == "file",
                        "is_folder": item.kind == "folder",
                        "is_page": item.file_kind == "page",
                        "is_component": item.file_kind == "component",
                        "is_script": item.file_kind == "script",
                        "is_style": item.file_kind == "style",
                        "is_selected": item.rel_path == selected_rel,
                        "classes": if item.rel_path == selected_rel { "is-selected" } else { "" },
                    })
                })
                .collect::<Vec<_>>();

            let nav = nav_classes(&owner, &project, "build", Some("templates"));
            let route = format!("/projects/{owner}/{project}/build/templates");
            let input = json!({
                "seo": {
                    "title": format!("{} - Templates", info.title),
                    "description": "Project template workspace"
                },
                "owner": info.owner,
                "project": info.project,
                "title": info.title,
                "project_href": format!("/projects/{owner}/{project}"),
                "current_menu": "Build / Templates",
                "nav": nav,
                "workspace": {
                    "items": template_items,
                    "api": {
                        "workspace": format!("/api/projects/{owner}/{project}/templates/workspace"),
                        "file": format!("/api/projects/{owner}/{project}/templates/file"),
                        "save": format!("/api/projects/{owner}/{project}/templates/file"),
                        "create": format!("/api/projects/{owner}/{project}/templates/create"),
                        "move": format!("/api/projects/{owner}/{project}/templates/move"),
                        "delete": format!("/api/projects/{owner}/{project}/templates/file"),
                        "git_status": format!("/api/projects/{owner}/{project}/templates/git-status"),
                        "diagnostics": format!("/api/projects/{owner}/{project}/templates/diagnostics"),
                    },
                    "selected_file": {
                        "name": selected_name,
                        "rel_path": selected_rel,
                        "file_kind": selected_kind,
                        "content": selected_content,
                        "line_count": selected_lines,
                        "is_protected": selected_file.is_protected,
                    },
                    "codemirror": {
                        "runtime_src": "/assets/libraries/zeb/codemirror/0.1/runtime/codemirror.bundle.mjs",
                        "package_label": "zeb/codemirror@0.1",
                    }
                },
            });

            match render_page(&state, "platform-project-build-templates", &route, input) {
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
        ProjectCapability::ProjectRead,
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
        ProjectCapability::ProjectRead,
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
    if let Err(response) = require_project_page_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
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
    if let Err(response) = require_project_page_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
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
        ProjectCapability::FilesRead,
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
        ProjectCapability::ProjectRead,
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
    let web_libraries_href = format!("/projects/{owner}/{project}/settings/web-libraries");
    let nodes_href = format!("/projects/{owner}/{project}/settings/nodes");
    let settings_href = format!("/projects/{owner}/{project}/settings");
    render_section_page(
        state,
        headers,
        owner,
        project,
        "settings",
        ProjectCapability::SettingsRead,
        "Settings",
        "Project policies, adapters, and runtime defaults.",
        vec![
            json!({
                "title":"Web Library Manager",
                "description":"Install and pin Zeb Libraries for templates, editor autocomplete, and compile-time runtime assets.",
                "href": web_libraries_href,
                "tag":"Web"
            }),
            json!({
                "title":"Node Manager",
                "description":"Manage runtime nodes, extension packages, and future hot-reloadable execution capabilities.",
                "href": nodes_href,
                "tag":"Runtime"
            }),
            json!({
                "title":"Runtime Policy",
                "description":"Timeout, retries, and execution policy.",
                "href": settings_href.clone(),
                "tag":"Core"
            }),
            json!({
                "title":"Environment",
                "description":"Project-level variables and secrets policy.",
                "href": settings_href,
                "tag":"Core"
            }),
        ],
    )
    .await
}

async fn project_settings_web_libraries_page(
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
        ProjectCapability::SettingsRead,
        "Web Library Manager",
        "Install Zeb Libraries, pin versions into the project, and feed editor autocomplete plus compile-time runtime assets.",
        vec![
            json!({
                "title":"zeb/codemirror",
                "description":"Platform-managed editor dependency. Bundled for the Build > Templates workspace and reusable later in project templates.",
                "href":"#",
                "tag":"Official"
            }),
            json!({
                "title":"Project Libraries",
                "description":"Project-owned library state should live under app/libraries with versions pinned in app/libraries.lock.json.",
                "href":"#",
                "tag":"Contract"
            }),
        ],
    )
    .await
}

async fn project_settings_nodes_page(
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
        ProjectCapability::SettingsRead,
        "Node Manager",
        "Manage runtime node packages, execution extensions, and future hot-reloadable node capabilities.",
        vec![
            json!({
                "title":"Official Nodes",
                "description":"Verified runtime node packages distributed with or for Zebflow.",
                "href":"#",
                "tag":"Official"
            }),
            json!({
                "title":"Community and Custom Nodes",
                "description":"Future install lanes for project-specific extensions with stricter trust and permission policy.",
                "href":"#",
                "tag":"Future"
            }),
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
    capability: ProjectCapability,
    section_title: &str,
    section_desc: &str,
    cards: Vec<Value>,
) -> Response {
    if let Err(response) =
        require_project_page_capability(&state, &headers, &owner, &project, capability)
    {
        return response;
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

fn content_type_for_path(path: &FsPath) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("mjs") | Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml; charset=utf-8",
        Some("png") => "image/png",
        _ => "application/octet-stream",
    }
}

fn template_kind_from_rel(rel: &str) -> &'static str {
    if rel.ends_with(".css") {
        "style"
    } else if rel.ends_with(".ts") {
        "script"
    } else if rel.contains("/pages/") || rel.starts_with("pages/") {
        "page"
    } else {
        "component"
    }
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

async fn api_template_workspace(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesRead,
    ) {
        return response;
    }
    match state.platform.projects.list_template_workspace(&owner, &project) {
        Ok(workspace) => Json(workspace).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_template_file(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Query(query): Query<TemplatePathQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesRead,
    ) {
        return response;
    }
    let Some(path) = query.path.as_deref() else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"missing path"})),
        )
            .into_response();
    };
    match state.platform.projects.read_template_payload(&owner, &project, path) {
        Ok(file) => Json(file).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_template_save(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<TemplateSaveRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesWrite,
    ) {
        return response;
    }
    match state.platform.projects.write_template_file(&owner, &project, &req) {
        Ok(file) => Json(file).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_template_create(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<TemplateCreateRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesCreate,
    ) {
        return response;
    }
    match state.platform.projects.create_template_entry(&owner, &project, &req) {
        Ok(file) => Json(file).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_template_move(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<TemplateMoveRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesMove,
    ) {
        return response;
    }
    match state.platform.projects.move_template_entry(&owner, &project, &req) {
        Ok(rel_path) => Json(json!({ "rel_path": rel_path })).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_template_delete(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Query(query): Query<TemplatePathQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesDelete,
    ) {
        return response;
    }
    let Some(path) = query.path.as_deref() else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"missing path"})),
        )
            .into_response();
    };
    match state.platform.projects.delete_template_entry(&owner, &project, path) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_template_git_status(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesRead,
    ) {
        return response;
    }
    match state.platform.projects.list_template_git_status(&owner, &project) {
        Ok(items) => Json(items).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_template_diagnostics(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<TemplateCompileRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesDiagnostics,
    ) {
        return response;
    }

    let owner = crate::platform::model::slug_segment(&owner);
    let project = crate::platform::model::slug_segment(&project);
    let layout = match state.platform.file.ensure_project_layout(&owner, &project) {
        Ok(layout) => layout,
        Err(err) => return internal_error(err),
    };

    let response = compile_template_buffer(&state, &layout.app_templates_dir, &req);
    Json(response).into_response()
}

fn session_owner(headers: &HeaderMap) -> Option<String> {
    let cookie = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    cookie.split(';').map(str::trim).find_map(|part| {
        part.strip_prefix("zebflow_session=")
            .map(ToString::to_string)
    })
}

fn require_project_page_capability(
    state: &PlatformAppState,
    headers: &HeaderMap,
    owner: &str,
    project: &str,
    capability: ProjectCapability,
) -> Result<ProjectAccessSubject, Response> {
    let Some(session_owner) = session_owner(headers) else {
        return Err(Redirect::to("/login").into_response());
    };
    let subject = ProjectAccessSubject::user(&session_owner);
    match state
        .platform
        .authz
        .ensure_project_capability(&subject, owner, project, capability)
    {
        Ok(()) => Ok(subject),
        Err(err) if err.code == "PLATFORM_PROJECT_MISSING" => {
            Err((StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response())
        }
        Err(err) if err.code == "PLATFORM_AUTHZ_FORBIDDEN" => {
            Err((StatusCode::FORBIDDEN, Html("forbidden".to_string())).into_response())
        }
        Err(err) => Err(internal_error(err)),
    }
}

fn require_project_api_capability(
    state: &PlatformAppState,
    headers: &HeaderMap,
    owner: &str,
    project: &str,
    capability: ProjectCapability,
) -> Result<ProjectAccessSubject, Response> {
    let Some(session_owner) = session_owner(headers) else {
        return Err(StatusCode::UNAUTHORIZED.into_response());
    };
    let subject = ProjectAccessSubject::user(&session_owner);
    match state
        .platform
        .authz
        .ensure_project_capability(&subject, owner, project, capability)
    {
        Ok(()) => Ok(subject),
        Err(err) if err.code == "PLATFORM_PROJECT_MISSING" => {
            Err(StatusCode::NOT_FOUND.into_response())
        }
        Err(err) if err.code == "PLATFORM_AUTHZ_FORBIDDEN" => {
            Err(StatusCode::FORBIDDEN.into_response())
        }
        Err(err) => Err(internal_error(err)),
    }
}

fn compile_template_buffer(
    state: &PlatformAppState,
    template_root: &FsPath,
    req: &TemplateCompileRequest,
) -> TemplateCompileResponse {
    let rel = req.rel_path.trim();
    if rel.is_empty() {
        return TemplateCompileResponse {
            ok: false,
            diagnostics: vec![TemplateDiagnostic {
                code: "template_path_missing".to_string(),
                message: "template path must not be empty".to_string(),
                severity: "error".to_string(),
                from: Some(0),
                to: Some(1),
            }],
        };
    }

    let kind = template_kind_from_rel(rel);
    if kind == "script" || kind == "style" {
        return TemplateCompileResponse {
            ok: true,
            diagnostics: Vec::new(),
        };
    }

    let options = ReactiveWebOptions {
        load_scripts: vec!["/assets/platform/*".to_string()],
        allow_list: crate::rwe::ResourceAllowList {
            scripts: vec!["/assets/platform/*".to_string()],
            urls: vec!["/assets/platform/*".to_string()],
            ..Default::default()
        },
        templates: TemplateOptions {
            template_root: Some(template_root.to_path_buf()),
            style_entries: Vec::new(),
        },
        processors: vec!["tailwind".to_string()],
        ..Default::default()
    };

    let source = TemplateSource {
        id: format!("platform.editor.{}", rel.replace('/', ".")),
        source_path: Some(template_root.join(rel)),
        markup: req.content.clone(),
    };

    match state
        .frontend
        .rwe
        .compile_template(&source, state.frontend.language.as_ref(), &options)
    {
        Ok(compiled) => TemplateCompileResponse {
            ok: true,
            diagnostics: compiled
                .diagnostics
                .into_iter()
                .map(|diag| TemplateDiagnostic {
                    code: diag.code,
                    message: diag.message,
                    severity: "warning".to_string(),
                    from: None,
                    to: None,
                })
                .collect(),
        },
        Err(err) => TemplateCompileResponse {
            ok: false,
            diagnostics: vec![TemplateDiagnostic {
                code: err.code.to_string(),
                message: err.message,
                severity: "error".to_string(),
                from: Some(0),
                to: Some(1),
            }],
        },
    }
}

fn internal_error(err: PlatformError) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
    )
        .into_response()
}

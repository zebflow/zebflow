//! Axum web layer for Zebflow platform flows, rendered via RWE templates.

mod embedded;

use std::collections::{BTreeSet, VecDeque};
use std::convert::Infallible;
use std::fs;
use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::body::{Body, Bytes};
use axum::extract::{Form, Path, Query, State};
use axum::http::{
    HeaderMap, HeaderValue, Method, StatusCode, Uri, header::CACHE_CONTROL, header::CONTENT_TYPE,
    header::SET_COOKIE,
};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{any, get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};
use swc_common::{FileName, SourceMap, sync::Lrc};
use swc_ecma_ast::{Callee, Expr, ModuleDecl, ModuleItem};
use swc_ecma_parser::{Parser, StringInput, Syntax, TsSyntax, lexer::Lexer};

use crate::automaton::assistant_config::load_project_assistant_llm;
use crate::framework::{BasicFrameworkEngine, FrameworkContext, FrameworkEngine, PipelineGraph};
use crate::language::{DenoSandboxEngine, LanguageEngine, NoopLanguageEngine};
use crate::platform::error::PlatformError;
use crate::platform::model::{
    CreateProjectRequest, CreateSimpleTableRequest, CreateUserRequest,
    DescribeProjectDbConnectionRequest, ExecutePipelineRequest, LoginRequest,
    McpSessionCreateRequest, PipelineExecuteTrigger, PipelineLocateRequest, ProjectAccessSubject,
    ProjectCapability, QueryProjectDbConnectionRequest, SimpleTableQueryRequest,
    TemplateCompileRequest, TemplateCompileResponse, TemplateCreateRequest, TemplateDiagnostic,
    TemplateMoveRequest, TemplateSaveRequest, TestProjectDbConnectionRequest,
    UpsertPipelineDefinitionRequest, UpsertProjectAssistantConfigRequest,
    UpsertProjectCredentialRequest, UpsertProjectDbConnectionRequest, UpsertProjectDocRequest,
    UpsertSimpleTableRowRequest,
};
use crate::platform::services::PlatformService;
use crate::rwe::{
    CompiledScript, CompiledTemplate, ReactiveWebEngine, ReactiveWebOptions, RenderContext,
    RenderScriptCache, ScriptCacheConfig, TemplateOptions, TemplateSource,
    resolve_engine_or_default,
};
use embedded::{PLATFORM_TEMPLATE_ASSETS, platform_library_asset};

const BRAND_LOGO_SVG: &[u8] = include_bytes!("../../../docs/conventions/assets/branding/logo.svg");
const BRAND_LOGO_PNG: &[u8] = include_bytes!("../../../docs/conventions/assets/branding/logo.png");
const RWE_ROUTER_JS: &str = include_str!("rwe_router.js");
const PLATFORM_MAIN_CSS: &str = include_str!("templates/styles/main.css");
const PLATFORM_DB_SUITE_CSS: &str = include_str!("templates/styles/db-suite.css");
const PLATFORM_DB_CONNECTIONS_CSS: &str = include_str!("templates/styles/db-connections.css");

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
    render_script_cache: Option<Arc<RenderScriptCache>>,
}

/// Builds Zebflow platform router.
pub fn router(platform: Arc<PlatformService>) -> Router {
    let frontend = build_frontend(&platform.config.data_root).unwrap_or_else(|err| {
        panic!("failed building platform frontend templates: {err}");
    });
    let render_script_cache = build_render_script_cache(&platform.config.data_root);

    let mcp_service = crate::platform::mcp::build_mcp_service(platform.clone());

    Router::new()
        .route("/", get(root_redirect))
        .route("/assets/branding/{asset}", get(branding_asset))
        .route("/assets/platform/rwe-router.js", get(rwe_router_js_handler))
        .route("/assets/platform/{asset}", get(platform_asset))
        .route("/assets/rwe/scripts/{hash}", get(rwe_script_asset))
        .route(
            "/assets/{owner}/{project}/rwe/scripts/{hash}",
            get(project_rwe_script_asset),
        )
        .route("/assets/{owner}/{project}/{*path}", get(project_asset))
        .route("/assets/libraries/{*path}", get(library_asset))
        .route("/login", get(login_page).post(login_submit))
        .route("/logout", post(logout_submit))
        .route("/home", get(home_page))
        .route("/docs/node", get(docs_node_contract))
        .route("/docs/operation", get(docs_operation_contract))
        .route("/home/projects/create", post(home_create_project_submit))
        .route("/projects/{owner}/{project}", get(project_root_page))
        .route(
            "/projects/{owner}/{project}/pipelines/{tab}",
            get(project_pipelines_page),
        )
        .route(
            "/projects/{owner}/{project}/build",
            get(project_build_root_page),
        )
        .route(
            "/projects/{owner}/{project}/build/{tab}",
            get(project_build_page),
        )
        .route(
            "/projects/{owner}/{project}/studio",
            get(project_studio_redirect_page),
        )
        .route(
            "/projects/{owner}/{project}/studio/{tab}",
            get(project_studio_tab_redirect_page),
        )
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
            "/projects/{owner}/{project}/db/connections",
            get(project_db_connections_page),
        )
        .route(
            "/projects/{owner}/{project}/db/{db_kind}/{connection}",
            get(project_db_suite_redirect_page),
        )
        .route(
            "/projects/{owner}/{project}/db/{db_kind}/{connection}/{tab}",
            get(project_db_suite_page),
        )
        .route(
            "/projects/{owner}/{project}/tables/connections",
            get(project_tables_connections_legacy_redirect_page),
        )
        .route(
            "/projects/{owner}/{project}/tables/connections/{connection}",
            get(project_table_connection_legacy_redirect_page),
        )
        .route("/projects/{owner}/{project}/files", get(project_files_page))
        .route("/projects/{owner}/{project}/todo", get(project_todo_page))
        .route(
            "/projects/{owner}/{project}/settings",
            get(project_settings_page),
        )
        .route(
            "/projects/{owner}/{project}/settings/{tab}",
            get(project_settings_tab_page),
        )
        .route(
            "/projects/{owner}/{project}/settings/web-libraries",
            get(project_settings_web_libraries_legacy_redirect_page),
        )
        .route("/api/meta", get(api_meta))
        .route("/api/admin/db/collections", get(api_admin_db_list_collections))
        .route("/api/admin/db/query", post(api_admin_db_query))
        .route("/api/admin/db/node/{slug}", get(api_admin_db_get_node).delete(api_admin_db_delete_node))
        .route(
            "/api/projects/{owner}/{project}/nodes",
            get(api_list_node_definitions),
        )
        .route("/api/users", get(api_list_users).post(api_create_user))
        .route(
            "/api/users/{owner}/projects",
            get(api_list_projects).post(api_create_project),
        )
        .route(
            "/api/projects/{owner}/{project}/pipelines/registry",
            get(api_pipeline_registry),
        )
        .route(
            "/api/projects/{owner}/{project}/pipelines",
            get(api_list_pipelines),
        )
        .route(
            "/api/projects/{owner}/{project}/pipelines/by-id",
            get(api_get_pipeline_by_id),
        )
        .route(
            "/api/projects/{owner}/{project}/pipelines/definition",
            post(api_upsert_pipeline_definition),
        )
        .route(
            "/api/projects/{owner}/{project}/pipelines/activate",
            post(api_activate_pipeline_definition),
        )
        .route(
            "/api/projects/{owner}/{project}/pipelines/deactivate",
            post(api_deactivate_pipeline_definition),
        )
        .route(
            "/api/projects/{owner}/{project}/pipelines/execute",
            post(api_execute_pipeline),
        )
        .route(
            "/api/projects/{owner}/{project}/pipelines/hits",
            get(api_pipeline_hits),
        )
        .route(
            "/api/projects/{owner}/{project}/templates/workspace",
            get(api_template_workspace),
        )
        .route(
            "/api/projects/{owner}/{project}/templates/file",
            get(api_template_file)
                .put(api_template_save)
                .delete(api_template_delete),
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
        .route(
            "/api/projects/{owner}/{project}/credentials",
            get(api_list_credentials).post(api_upsert_credential),
        )
        .route(
            "/api/projects/{owner}/{project}/credentials/{credential_id}",
            get(api_get_credential)
                .put(api_upsert_credential_by_path)
                .delete(api_delete_credential),
        )
        .route(
            "/api/projects/{owner}/{project}/assistant/config",
            get(api_get_project_assistant_config).put(api_upsert_project_assistant_config),
        )
        .route(
            "/api/projects/{owner}/{project}/assistant/chat",
            post(api_project_assistant_chat),
        )
        .route(
            "/api/projects/{owner}/{project}/db/connections",
            get(api_list_db_connections).post(api_upsert_db_connection),
        )
        .route(
            "/api/projects/{owner}/{project}/db/connections/{connection_id}/describe",
            get(api_describe_db_connection),
        )
        .route(
            "/api/projects/{owner}/{project}/db/connections/{connection_id}/schemas",
            get(api_list_db_connection_schemas),
        )
        .route(
            "/api/projects/{owner}/{project}/db/connections/{connection_id}/tables",
            get(api_list_db_connection_tables),
        )
        .route(
            "/api/projects/{owner}/{project}/db/connections/{connection_id}/functions",
            get(api_list_db_connection_functions),
        )
        .route(
            "/api/projects/{owner}/{project}/db/connections/{connection_id}/query",
            post(api_query_db_connection),
        )
        .route(
            "/api/projects/{owner}/{project}/db/connections/{connection_id}/table-preview",
            get(api_preview_db_connection_table),
        )
        .route(
            "/api/projects/{owner}/{project}/db/connections/{connection_slug}",
            get(api_get_db_connection)
                .put(api_upsert_db_connection_by_path)
                .delete(api_delete_db_connection),
        )
        .route(
            "/api/projects/{owner}/{project}/db/connections/test",
            post(api_test_db_connection),
        )
        .route(
            "/api/projects/{owner}/{project}/docs",
            get(api_list_project_docs).post(api_upsert_project_doc),
        )
        .route(
            "/api/projects/{owner}/{project}/docs/file",
            get(api_read_project_doc).put(api_upsert_project_doc_file),
        )
        .route(
            "/api/projects/{owner}/{project}/agent-docs",
            get(api_list_agent_docs),
        )
        .route(
            "/api/projects/{owner}/{project}/agent-docs/file",
            get(api_read_agent_doc).put(api_upsert_agent_doc_file),
        )
        .route(
            "/api/projects/{owner}/{project}/tables",
            get(api_list_simple_tables).post(api_create_simple_table),
        )
        .route(
            "/api/projects/{owner}/{project}/tables/{table}",
            get(api_get_simple_table).delete(api_delete_simple_table),
        )
        .route(
            "/api/projects/{owner}/{project}/tables/rows",
            post(api_upsert_simple_table_row),
        )
        .route(
            "/api/projects/{owner}/{project}/tables/query",
            post(api_query_simple_table_rows),
        )
        .route(
            "/api/projects/{owner}/{project}/mcp/session",
            get(api_get_mcp_session)
                .post(api_create_mcp_session)
                .delete(api_revoke_mcp_session),
        )
        .route(
            "/api/projects/{owner}/{project}/assets/prepare",
            post(api_prepare_project_assets),
        )
        .nest("/api/projects/{owner}/{project}/mcp", mcp_service)
        .route("/wh/{owner}/{project}", any(public_webhook_ingress_root))
        .route("/wh/{owner}/{project}/{*tail}", any(public_webhook_ingress))
        .with_state(PlatformAppState {
            platform,
            frontend,
            render_script_cache,
        })
}

fn build_render_script_cache(data_root: &FsPath) -> Option<Arc<RenderScriptCache>> {
    let root = data_root.join("platform").join("rwe-script-cache");
    let cfg = ScriptCacheConfig::new(root, 8 * 1024 * 1024);
    match RenderScriptCache::new(cfg) {
        Ok(cache) => Some(Arc::new(cache)),
        Err(err) => {
            eprintln!("warning: failed creating RWE script cache: {err}");
            None
        }
    }
}

fn build_frontend(data_root: &FsPath) -> Result<PlatformFrontend, PlatformError> {
    let rwe_engine_id = std::env::var("ZEBFLOW_PLATFORM_RWE_ENGINE_ID").ok();
    let rwe: Arc<dyn ReactiveWebEngine> = resolve_engine_or_default(rwe_engine_id.as_deref());
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
        "platform-project-pipelines-registry",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "platform.project.pipelines.registry",
            &template_root,
            "pages/platform-project-pipelines-registry.tsx",
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
        "platform-project-settings",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "platform.project.settings",
            &template_root,
            "pages/platform-project-settings.tsx",
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
        "platform-project-docs",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "platform.project.docs",
            &template_root,
            "pages/platform-project-docs.tsx",
            options.clone(),
        )?,
    );

    pages.insert(
        "platform-project-credentials",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "platform.project.credentials",
            &template_root,
            "pages/platform-project-credentials.tsx",
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
            options.clone(),
        )?,
    );

    pages.insert(
        "platform-project-table-connection-postgresql",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "platform.project.table_connection.postgresql",
            &template_root,
            "pages/platform-project-table-connection-postgresql.tsx",
            options.clone(),
        )?,
    );

    pages.insert(
        "platform-project-table-connection-sjtable",
        compile_page(
            rwe.as_ref(),
            language.as_ref(),
            "platform.project.table_connection.sjtable",
            &template_root,
            "pages/platform-project-table-connection-sjtable.tsx",
            options.clone(),
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
    let root = data_root
        .join("platform")
        .join("embedded")
        .join("templates");
    for asset in PLATFORM_TEMPLATE_ASSETS {
        let full = root.join(asset.path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(full, asset.bytes)?;
    }
    rewrite_platform_template_alias_imports(&root)?;
    Ok(root)
}

fn rewrite_platform_template_alias_imports(root: &FsPath) -> Result<(), PlatformError> {
    fn visit(dir: &FsPath, out: &mut Vec<PathBuf>) -> Result<(), PlatformError> {
        for entry in fs::read_dir(dir).map_err(|e| {
            PlatformError::new(
                "PLATFORM_TEMPLATE_READ_DIR",
                format!("failed reading '{}': {e}", dir.display()),
            )
        })? {
            let entry = entry.map_err(|e| {
                PlatformError::new(
                    "PLATFORM_TEMPLATE_READ_DIR",
                    format!("failed reading dir entry in '{}': {e}", dir.display()),
                )
            })?;
            let path = entry.path();
            if path.is_dir() {
                visit(&path, out)?;
                continue;
            }
            if let Some(ext) = path.extension().and_then(|v| v.to_str())
                && matches!(ext, "tsx" | "ts" | "jsx" | "js")
            {
                out.push(path);
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    visit(root, &mut files)?;

    for file in files {
        let source = fs::read_to_string(&file).map_err(|e| {
            PlatformError::new(
                "PLATFORM_TEMPLATE_READ",
                format!("failed reading '{}': {e}", file.display()),
            )
        })?;
        let normalized = normalize_legacy_attr_values_in_source(&source);
        let rewritten = rewrite_alias_imports_for_source(&normalized, root)?;
        if rewritten != source {
            fs::write(&file, rewritten).map_err(|e| {
                PlatformError::new(
                    "PLATFORM_TEMPLATE_WRITE",
                    format!("failed writing '{}': {e}", file.display()),
                )
            })?;
        }
    }
    Ok(())
}

fn normalize_legacy_attr_values_in_source(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut i = 0usize;
    let mut out = String::with_capacity(source.len());
    while i < bytes.len() {
        if bytes[i] == b'<' {
            let Some(tag_end) = find_tag_end_for_rewrite(source, i) else {
                out.push_str(&source[i..]);
                break;
            };
            let tag_src = &source[i..=tag_end];
            out.push_str(&convert_legacy_attr_expr_in_tag_source(tag_src));
            i = tag_end + 1;
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn find_tag_end_for_rewrite(source: &str, tag_start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut i = tag_start;
    let mut in_quote: Option<u8> = None;
    while i < bytes.len() {
        let b = bytes[i];
        match in_quote {
            Some(q) if b == q => in_quote = None,
            None if b == b'\'' || b == b'"' => in_quote = Some(b),
            None if b == b'>' => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

fn convert_legacy_attr_expr_in_tag_source(tag_source: &str) -> String {
    if tag_source.starts_with("</") || tag_source.starts_with("<!") || tag_source.starts_with("<?")
    {
        return tag_source.to_string();
    }
    let bytes = tag_source.as_bytes();
    let mut i = 0usize;
    let mut out = String::with_capacity(tag_source.len());

    while i < bytes.len() {
        if bytes[i] == b'=' && i + 1 < bytes.len() && bytes[i + 1] == b'"' {
            let value_start = i + 2;
            let mut j = value_start;
            while j < bytes.len() && bytes[j] != b'"' {
                j += 1;
            }
            if j >= bytes.len() {
                out.push_str(&tag_source[i..]);
                break;
            }
            let value = &tag_source[value_start..j];
            let trimmed = value.trim();
            if trimmed.starts_with('{') && trimmed.ends_with('}') && trimmed.len() >= 2 {
                let inner = trimmed[1..trimmed.len() - 1].trim();
                out.push('=');
                out.push('{');
                out.push_str(inner);
                out.push('}');
            } else {
                out.push('=');
                out.push('"');
                out.push_str(value);
                out.push('"');
            }
            i = j + 1;
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn rewrite_alias_imports_for_source(source: &str, root: &FsPath) -> Result<String, PlatformError> {
    let mut out = source.to_string();
    out = rewrite_alias_import_variant(&out, root, '"')?;
    out = rewrite_alias_import_variant(&out, root, '\'')?;
    Ok(out)
}

fn rewrite_alias_import_variant(
    source: &str,
    root: &FsPath,
    quote: char,
) -> Result<String, PlatformError> {
    let marker = format!("from {}", quote);
    let mut out = source.to_string();
    let mut cursor = 0usize;
    while let Some(rel_idx) = out[cursor..].find(marker.as_str()) {
        let idx = cursor + rel_idx;
        let spec_start = idx + marker.len();
        let Some(end_rel) = out[spec_start..].find(quote) else {
            break;
        };
        let spec_end = spec_start + end_rel;
        let rel = &out[spec_start..spec_end];
        if !rel.starts_with("@/") {
            cursor = spec_end + 1;
            continue;
        }
        let resolved = resolve_platform_alias_import(root, rel.trim_start_matches("@/"))?;
        out.replace_range(spec_start..spec_end, &resolved);
        cursor = spec_start + resolved.len();
    }
    Ok(out)
}

fn resolve_platform_alias_import(root: &FsPath, rel: &str) -> Result<String, PlatformError> {
    let base = root.join(rel);
    if base.exists() {
        let abs = fs::canonicalize(&base).unwrap_or(base);
        return Ok(abs.to_string_lossy().to_string());
    }
    for ext in [".tsx", ".ts", ".jsx", ".js"] {
        let candidate = PathBuf::from(format!("{}{}", base.display(), ext));
        if candidate.exists() {
            let abs = fs::canonicalize(&candidate).unwrap_or(candidate);
            return Ok(abs.to_string_lossy().to_string());
        }
    }
    for index in ["index.tsx", "index.ts", "index.jsx", "index.js"] {
        let candidate = base.join(index);
        if candidate.exists() {
            let abs = fs::canonicalize(&candidate).unwrap_or(candidate);
            return Ok(abs.to_string_lossy().to_string());
        }
    }
    Ok(base.to_string_lossy().to_string())
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

    let mut html = out.html;

    // Ensure UTF-8 meta is present early in the document so browsers don't
    // fall back to Latin-1 when no DOCTYPE or explicit encoding is in the fragment.
    html = ensure_meta_charset(html);

    if let Some(css) = out.hydration_payload.get("css").and_then(Value::as_str)
        && !css.trim().is_empty()
    {
        let style_block = format!("<style data-rwe-tw>{css}</style>");
        if let Some(pos) = html.find("</head>") {
            html.insert_str(pos, &style_block);
        } else {
            html = format!("{style_block}{html}");
        }
    }

    // All project workspace pages depend on shared shell/design CSS.
    if route.starts_with("/projects/") {
        html = ensure_stylesheet_link(html, "/assets/platform/main.css");
    }
    // DB suite pages require dedicated layout rules + devicons.
    if html.contains("data-db-suite=\"true\"") {
        html = ensure_stylesheet_link(html, "/assets/platform/db-suite.css");
        html = ensure_stylesheet_link(
            html,
            "/assets/libraries/zeb/devicons/0.1/runtime/devicons.css",
        );
    }

    Ok(externalize_rwe_scripts(
        state,
        html.as_str(),
        &out.compiled_scripts,
        None,
    ))
}

fn ensure_meta_charset(mut html: String) -> String {
    if html.contains("<meta charset") || html.contains("<meta http-equiv=\"Content-Type\"") {
        return html;
    }
    let tag = "<meta charset=\"utf-8\">";
    if let Some(pos) = html.find("<head>") {
        html.insert_str(pos + "<head>".len(), tag);
    } else if let Some(pos) = html.find("</head>") {
        html.insert_str(pos, tag);
    } else {
        html = format!("{tag}{html}");
    }
    html
}

fn ensure_stylesheet_link(mut html: String, href: &str) -> String {
    if html.contains(href) {
        return html;
    }
    let link = format!("<link rel=\"stylesheet\" href=\"{href}\">");
    if let Some(pos) = html.find("</head>") {
        html.insert_str(pos, &link);
        return html;
    }
    format!("{link}{html}")
}

fn externalize_rwe_scripts(
    state: &PlatformAppState,
    html: &str,
    compiled_scripts: &[CompiledScript],
    project_scope: Option<(&str, &str)>,
) -> String {
    let Some(cache) = &state.render_script_cache else {
        return html.to_string();
    };
    if compiled_scripts.is_empty() {
        return html.to_string();
    }

    let mut script_tags = String::new();
    for script in compiled_scripts {
        if script.content_hash.trim().is_empty() {
            continue;
        }
        let store_res = match project_scope {
            Some((owner, project)) => cache.store_scoped(owner, project, script),
            None => cache.store(script),
        };
        if store_res.is_err() {
            continue;
        }
        let role = if script.id == "runtime" {
            "runtime"
        } else {
            "page"
        };
        let src = match project_scope {
            Some((owner, project)) => {
                format!(
                    "/assets/{owner}/{project}/rwe/scripts/{}",
                    script.content_hash
                )
            }
            None => format!("/assets/rwe/scripts/{}", script.content_hash),
        };
        script_tags.push_str(&format!(
            "<script type=\"module\" defer data-rwe-external=\"{}\" src=\"{}\"></script>",
            role, src
        ));
    }
    if script_tags.is_empty() {
        return html.to_string();
    }

    let stripped = strip_inline_runtime_bundle(html);
    inject_before_body_end(&stripped, &script_tags)
}

fn strip_inline_runtime_bundle(html: &str) -> String {
    let marker = "<script data-rwe-runtime=";
    let Some(start) = html.find(marker) else {
        return html.to_string();
    };
    let Some(end_rel) = html[start..].find("</script>") else {
        return html.to_string();
    };
    let end = start + end_rel + "</script>".len();
    let mut out = String::with_capacity(html.len());
    out.push_str(&html[..start]);
    out.push_str(&html[end..]);
    out
}

fn inject_before_body_end(html: &str, chunk: &str) -> String {
    if let Some(idx) = html.rfind("</body>") {
        let mut out = String::with_capacity(html.len() + chunk.len());
        out.push_str(&html[..idx]);
        out.push_str(chunk);
        out.push_str(&html[idx..]);
        out
    } else {
        let mut out = String::with_capacity(html.len() + chunk.len());
        out.push_str(html);
        out.push_str(chunk);
        out
    }
}

async fn root_redirect() -> Redirect {
    Redirect::to("/login")
}

#[derive(Debug, Clone, Default, Deserialize)]
struct PipelineRegistryQuery {
    path: Option<String>,
    scope: Option<String>,
    id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PipelineRegistryScope {
    Path,
    Project,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct PipelineListQuery {
    path: Option<String>,
    recursive: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct PipelineByIdQuery {
    id: Option<String>,
    include_source: Option<bool>,
    include_active_source: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TemplateWorkspaceQuery {
    file: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TemplatePathQuery {
    path: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct DocPathQuery {
    path: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct DbSuiteQuery {
    table: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct DbDescribeQuery {
    scope: Option<String>,
    schema: Option<String>,
    include_system: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct DbObjectListQuery {
    schema: Option<String>,
    include_system: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct DbTablePreviewQuery {
    table: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct PrepareProjectAssetsRequest {
    library: String,
    version: String,
    entries: Vec<String>,
}

impl Default for PrepareProjectAssetsRequest {
    fn default() -> Self {
        Self {
            library: "zeb/threejs".to_string(),
            version: "0.1".to_string(),
            entries: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct AssistantChatRequest {
    message: String,
    history: Vec<AssistantChatMessage>,
    use_high_model: bool,
    current_page: Option<String>,
    client_time: Option<String>,
}

impl Default for AssistantChatRequest {
    fn default() -> Self {
        Self {
            message: String::new(),
            history: Vec::new(),
            use_high_model: false,
            current_page: None,
            client_time: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct AssistantChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ProjectAssetChunkItem {
    chunk_id: String,
    module_count: usize,
    modules: Vec<String>,
    path: String,
    url: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ProjectAssetEntryItem {
    entry: String,
    path: String,
    url: String,
    chunks: Vec<ProjectAssetChunkItem>,
    imports: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ProjectAssetLibraryItem {
    library: String,
    version: String,
    entries: Vec<ProjectAssetEntryItem>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ProjectAssetManifest {
    schema_version: String,
    owner: String,
    project: String,
    generated_at: i64,
    strategy: String,
    libraries: Vec<ProjectAssetLibraryItem>,
}

async fn branding_asset(Path(asset): Path<String>) -> Response {
    match asset.as_str() {
        "logo.svg" => asset_response("image/svg+xml; charset=utf-8", BRAND_LOGO_SVG),
        "logo.png" => asset_response("image/png", BRAND_LOGO_PNG),
        _ => (StatusCode::NOT_FOUND, "asset not found").into_response(),
    }
}

async fn rwe_router_js_handler() -> Response {
    asset_response("application/javascript; charset=utf-8", RWE_ROUTER_JS.as_bytes())
}

async fn platform_asset(Path(asset): Path<String>) -> Response {
    match asset.as_str() {
        "main.css" => {
            asset_response("text/css; charset=utf-8", PLATFORM_MAIN_CSS.as_bytes())
        }
        "db-suite.css" => {
            asset_response("text/css; charset=utf-8", PLATFORM_DB_SUITE_CSS.as_bytes())
        }
        "db-connections.css" => asset_response(
            "text/css; charset=utf-8",
            PLATFORM_DB_CONNECTIONS_CSS.as_bytes(),
        ),
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

async fn rwe_script_asset(
    State(state): State<PlatformAppState>,
    Path(hash): Path<String>,
) -> Response {
    if !hash
        .bytes()
        .all(|ch| ch.is_ascii_hexdigit() || ch == b'-' || ch == b'_')
    {
        return (StatusCode::BAD_REQUEST, "invalid script hash").into_response();
    }
    let Some(cache) = &state.render_script_cache else {
        return (StatusCode::NOT_FOUND, "script cache unavailable").into_response();
    };
    let Some(content) = cache.get(&hash).ok().flatten() else {
        return (StatusCode::NOT_FOUND, "script not found").into_response();
    };

    let mut resp = Response::new(Body::from(content));
    *resp.status_mut() = StatusCode::OK;
    resp.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/javascript; charset=utf-8"),
    );
    resp.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    resp
}

async fn project_rwe_script_asset(
    State(state): State<PlatformAppState>,
    Path((owner, project, hash)): Path<(String, String, String)>,
) -> Response {
    if !hash
        .bytes()
        .all(|ch| ch.is_ascii_hexdigit() || ch == b'-' || ch == b'_')
    {
        return (StatusCode::BAD_REQUEST, "invalid script hash").into_response();
    }
    let valid_segment = |value: &str| {
        value
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_')
    };
    if !valid_segment(&owner) || !valid_segment(&project) {
        return (StatusCode::BAD_REQUEST, "invalid project scope").into_response();
    }
    let Some(cache) = &state.render_script_cache else {
        return (StatusCode::NOT_FOUND, "script cache unavailable").into_response();
    };

    let scoped = cache.get_scoped(&owner, &project, &hash).ok().flatten();
    let global = cache.get(&hash).ok().flatten();
    let Some(content) = scoped.or(global) else {
        return (StatusCode::NOT_FOUND, "script not found").into_response();
    };

    let mut resp = Response::new(Body::from(content));
    *resp.status_mut() = StatusCode::OK;
    resp.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/javascript; charset=utf-8"),
    );
    resp.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    resp
}

async fn project_asset(
    State(state): State<PlatformAppState>,
    Path((owner, project, path)): Path<(String, String, String)>,
) -> Response {
    let valid_segment = |value: &str| {
        value
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_')
    };
    if !valid_segment(&owner) || !valid_segment(&project) {
        return (StatusCode::BAD_REQUEST, "invalid project scope").into_response();
    }

    let normalized = path.trim_start_matches('/').replace('\\', "/");
    if normalized.is_empty() || normalized.contains("..") {
        return (StatusCode::BAD_REQUEST, "invalid asset path").into_response();
    }

    let layout = match state.platform.file.ensure_project_layout(&owner, &project) {
        Ok(layout) => layout,
        Err(err) => return internal_error(err),
    };
    let root = layout.data_runtime_dir.join("web-assets");
    let abs = root.join(&normalized);
    if !abs.starts_with(&root) {
        return (StatusCode::BAD_REQUEST, "invalid asset path").into_response();
    }
    if !abs.is_file() {
        return (StatusCode::NOT_FOUND, "asset not found").into_response();
    }
    let bytes = match std::fs::read(&abs) {
        Ok(bytes) => bytes,
        Err(err) => {
            return internal_error(PlatformError::new("PLATFORM_ASSET_READ", err.to_string()));
        }
    };
    let mut resp = Response::new(Body::from(bytes));
    *resp.status_mut() = StatusCode::OK;
    if let Ok(v) = HeaderValue::from_str(content_type_for_path(&abs)) {
        resp.headers_mut().insert(CONTENT_TYPE, v);
    }
    resp.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    resp
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
        query.path.as_deref(),
        None,
    )
    .await
}

async fn project_pipelines_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, tab)): Path<(String, String, String)>,
    Query(query): Query<PipelineRegistryQuery>,
) -> Response {
    render_project_pipelines_with_tab(
        state,
        headers,
        owner,
        project,
        &tab,
        query.path.as_deref(),
        query.id.as_deref(),
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

async fn project_studio_redirect_page(Path((owner, project)): Path<(String, String)>) -> Response {
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
    registry_path: Option<&str>,
    editor_id: Option<&str>,
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
    let is_editor = tab == "editor";
    let tab_payload = if is_registry {
        Some((
            "registry",
            "Pipeline Registry",
            "Browse pipelines by project path.",
            Vec::new(),
        ))
    } else if is_editor {
        Some((
            "editor",
            "Pipeline Editor",
            "Create and edit pipeline graph + node configuration.",
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
            let editor_base = format!("/projects/{owner}/{project}/pipelines/editor");
            let route_base = format!("/projects/{owner}/{project}/pipelines/registry");
            let pipeline_items = if is_registry || is_editor {
                items
            } else {
                let trigger_filter = match tab_key {
                    "webhooks" => Some("webhook"),
                    "schedules" => Some("schedule"),
                    "manual" => Some("manual"),
                    "functions" => Some("function"),
                    _ => None,
                };
                match state
                    .platform
                    .projects
                    .list_pipeline_meta_rows(&owner, &project)
                {
                    Ok(rows) => rows
                        .into_iter()
                        .filter(|meta| {
                            trigger_filter
                                .map(|wanted| meta.trigger_kind.eq_ignore_ascii_case(wanted))
                                .unwrap_or(true)
                        })
                        .map(|meta| {
                            let file_rel_path = meta.file_rel_path.clone();
                            let virtual_path = crate::platform::model::normalize_virtual_path(
                                &meta.virtual_path,
                            );
                            let (webhook_path, webhook_method) = if tab_key == "webhooks" {
                                match state.platform.projects.read_pipeline_source(
                                    &owner,
                                    &project,
                                    &file_rel_path,
                                ) {
                                    Ok(source) => webhook_trigger_from_pipeline_source(&source)
                                        .map_or((None, None), |(path, method)| {
                                            (Some(path), Some(method))
                                        }),
                                    Err(_) => (None, None),
                                }
                            } else {
                                (None, None)
                            };
                            json!({
                                "name": meta.name,
                                "title": meta.title,
                                "description": meta.description,
                                "trigger_kind": meta.trigger_kind,
                                "virtual_path": virtual_path,
                                "file_rel_path": file_rel_path,
                                "editor_href": format!("{editor_base}?path={virtual_path}&id={file_rel_path}"),
                                "webhook_path": webhook_path.unwrap_or_else(|| "/".to_string()),
                                "webhook_method": webhook_method.unwrap_or_else(|| "GET".to_string()),
                            })
                        })
                        .collect::<Vec<_>>(),
                    Err(err) => return internal_error(err),
                }
            };

            let registry = if is_registry {
                let current_registry_path = registry_path.unwrap_or("/");
                match state.platform.projects.list_pipeline_registry(
                    &owner,
                    &project,
                    current_registry_path,
                    &route_base,
                ) {
                    Ok(listing) => {
                        let current_path = listing.current_path;
                        let breadcrumbs = listing.breadcrumbs;
                        let folders = listing.folders;
                        let pipelines = listing
                            .pipelines
                            .into_iter()
                            .map(|item| {
                                let file_id = item.file_rel_path.clone();
                                json!({
                                    "name": item.name,
                                    "title": item.title,
                                    "description": item.description,
                                    "trigger_kind": item.trigger_kind,
                                    "file_rel_path": item.file_rel_path,
                                    "edit_href": format!("{editor_base}?path={current_path}&id={file_id}")
                                })
                            })
                            .collect::<Vec<_>>();
                        let has_folders = !folders.is_empty();
                        let has_pipelines = !pipelines.is_empty();
                        json!({
                            "current_path": current_path,
                            "editor_href": format!("{editor_base}?path={current_path}"),
                            "breadcrumbs": breadcrumbs,
                            "folders": folders,
                            "pipelines": pipelines,
                            "has_folders": has_folders,
                            "has_pipelines": has_pipelines,
                        })
                    }
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

            let editor_payload = if is_editor {
                let all_rows = match state
                    .platform
                    .projects
                    .list_pipeline_meta_rows(&owner, &project)
                {
                    Ok(rows) => rows,
                    Err(err) => return internal_error(err),
                };

                let wanted_id = editor_id
                    .map(str::trim)
                    .filter(|raw| !raw.is_empty())
                    .map(str::to_string)
                    .or_else(|| all_rows.first().map(|meta| meta.file_rel_path.clone()));

                let selected_any = wanted_id
                    .as_deref()
                    .and_then(|id| all_rows.iter().find(|row| row.file_rel_path == id))
                    .cloned()
                    .or_else(|| all_rows.first().cloned());

                let scope_path = registry_path
                    .map(crate::platform::model::normalize_virtual_path)
                    .unwrap_or_else(|| {
                        selected_any
                            .as_ref()
                            .map(|meta| {
                                crate::platform::model::normalize_virtual_path(&meta.virtual_path)
                            })
                            .unwrap_or_else(|| "/".to_string())
                    });

                let rows = all_rows
                    .iter()
                    .filter(|meta| {
                        crate::platform::model::normalize_virtual_path(&meta.virtual_path)
                            == scope_path
                    })
                    .cloned()
                    .collect::<Vec<_>>();

                let selected = wanted_id
                    .as_deref()
                    .and_then(|id| rows.iter().find(|row| row.file_rel_path == id))
                    .cloned()
                    .or_else(|| rows.first().cloned());
                let selected_id_effective =
                    selected.as_ref().map(|meta| meta.file_rel_path.clone());

                let mut lock_map = std::collections::HashMap::new();
                for meta in &rows {
                    let locked = match state.platform.projects.read_pipeline_source(
                        &owner,
                        &project,
                        &meta.file_rel_path,
                    ) {
                        Ok(source) => pipeline_source_is_locked(&source),
                        Err(_) => false,
                    };
                    lock_map.insert(meta.file_rel_path.clone(), locked);
                }

                let (source, graph_json, parse_error, hit_stats, selected_locked) =
                    if let Some(meta) = &selected {
                        let source = match state.platform.projects.read_pipeline_source(
                            &owner,
                            &project,
                            &meta.file_rel_path,
                        ) {
                            Ok(source) => source,
                            Err(err) => return internal_error(err),
                        };
                        let (graph_json, parse_error) = match serde_json::from_str::<Value>(&source)
                        {
                            Ok(value) => (value, Value::Null),
                            Err(err) => (
                                Value::Null,
                                Value::String(format!("pipeline JSON parse error: {err}")),
                            ),
                        };
                        let stats =
                            state
                                .platform
                                .pipeline_hits
                                .get(&owner, &project, &meta.file_rel_path);
                        let locked = lock_map
                            .get(&meta.file_rel_path)
                            .copied()
                            .unwrap_or_else(|| pipeline_source_is_locked(&source));
                        (
                            Value::String(source),
                            graph_json,
                            parse_error,
                            json!(stats),
                            Value::Bool(locked),
                        )
                    } else {
                        (
                            Value::Null,
                            Value::Null,
                            Value::Null,
                            Value::Null,
                            Value::Bool(false),
                        )
                    };

                let node_catalog = crate::framework::nodes::builtin_node_definitions()
                    .into_iter()
                    .map(|def| {
                        json!({
                            "kind": def.kind,
                            "title": def.title,
                            "description": def.description,
                            "input_pins": def.input_pins,
                            "output_pins": def.output_pins,
                            "input_schema": def.input_schema,
                            "output_schema": def.output_schema
                        })
                    })
                    .collect::<Vec<_>>();

                let mut folder_counts = std::collections::BTreeMap::<String, usize>::new();
                for meta in &all_rows {
                    let vpath = crate::platform::model::normalize_virtual_path(&meta.virtual_path);
                    *folder_counts.entry(vpath).or_insert(0) += 1;
                }
                let scope_folders = folder_counts
                    .into_iter()
                    .map(|(vpath, count)| {
                        json!({
                            "virtual_path": vpath,
                            "count": count,
                            "href": format!("{editor_base}?path={vpath}")
                        })
                    })
                    .collect::<Vec<_>>();

                let mut scope_hierarchy = vec![json!({
                    "name": "root",
                    "virtual_path": "/",
                    "href": format!("{editor_base}?path=/")
                })];
                if scope_path != "/" {
                    let mut accum = String::new();
                    for seg in scope_path.trim_start_matches('/').split('/') {
                        if seg.trim().is_empty() {
                            continue;
                        }
                        accum.push('/');
                        accum.push_str(seg);
                        scope_hierarchy.push(json!({
                            "name": seg,
                            "virtual_path": accum,
                            "href": format!("{editor_base}?path={accum}")
                        }));
                    }
                }

                let pipelines = rows
                    .iter()
                    .map(|meta| {
                        let file_id = meta.file_rel_path.clone();
                        let is_active = meta
                            .active_hash
                            .as_deref()
                            .map(|hash| hash == meta.hash)
                            .unwrap_or(false);
                        let has_draft = meta
                            .active_hash
                            .as_deref()
                            .map(|hash| hash != meta.hash)
                            .unwrap_or(false);
                        let locked = lock_map.get(&file_id).copied().unwrap_or(false);
                        json!({
                            "id": file_id,
                            "name": meta.name,
                            "title": meta.title,
                            "description": meta.description,
                            "trigger_kind": meta.trigger_kind,
                            "virtual_path": meta.virtual_path,
                            "file_rel_path": meta.file_rel_path,
                            "is_active": is_active,
                            "has_draft": has_draft,
                            "is_locked": locked,
                            "status_label": if is_active { "active" } else if has_draft { "draft" } else { "inactive" },
                            "editor_href": format!("{editor_base}?path={scope_path}&id={file_id}")
                        })
                    })
                    .collect::<Vec<_>>();

                json!({
                    "scope_path": scope_path,
                    "scope_hierarchy": scope_hierarchy,
                    "scope_folders": scope_folders,
                    "selected_id": selected_id_effective,
                    "selected_locked": selected_locked,
                    "selected_meta": selected,
                    "selected_source": source,
                    "selected_graph": graph_json,
                    "parse_error": parse_error,
                    "hits": hit_stats,
                    "pipelines": pipelines,
                    "nodes": node_catalog,
                    "api": {
                        "registry": format!("/api/projects/{owner}/{project}/pipelines/registry"),
                        "list": format!("/api/projects/{owner}/{project}/pipelines"),
                        "by_id": format!("/api/projects/{owner}/{project}/pipelines/by-id"),
                        "definition": format!("/api/projects/{owner}/{project}/pipelines/definition"),
                        "activate": format!("/api/projects/{owner}/{project}/pipelines/activate"),
                        "deactivate": format!("/api/projects/{owner}/{project}/pipelines/deactivate"),
                        "hits": format!("/api/projects/{owner}/{project}/pipelines/hits"),
                        "nodes": format!("/api/projects/{owner}/{project}/nodes"),
                        "credentials": format!("/api/projects/{owner}/{project}/credentials"),
                        "templates_workspace": format!("/api/projects/{owner}/{project}/templates/workspace"),
                        "template_file": format!("/api/projects/{owner}/{project}/templates/file"),
                        "template_save": format!("/api/projects/{owner}/{project}/templates/save"),
                    },
                    "graphui": {
                        "runtime_src": "/assets/libraries/zeb/graphui/0.1/runtime/graphui.bundle.mjs",
                        "package_label": "zeb/graphui@0.1"
                    }
                })
            } else {
                Value::Null
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
                "pipeline_items": pipeline_items,
                "is_registry": is_registry,
                "is_editor": is_editor,
                "is_non_registry": !is_registry,
                "is_webhooks": tab_key == "webhooks",
                "registry": registry,
                "editor": editor_payload,
                "nav": nav,
            });

            let page_id = if is_registry {
                "platform-project-pipelines-registry"
            } else {
                "platform-project-pipelines"
            };
            match render_page(&state, page_id, &route, input) {
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
    if tab == "schema" {
        return Redirect::to(&format!("/projects/{owner}/{project}/build/docs")).into_response();
    }

    let capability = if tab == "templates" {
        ProjectCapability::TemplatesRead
    } else {
        ProjectCapability::ProjectRead
    };
    if let Err(response) =
        require_project_page_capability(&state, &headers, &owner, &project, capability)
    {
        return response;
    }

    if tab == "templates" {
        return render_project_build_templates(state, owner, project, query).await;
    }

    if tab == "docs" {
        return render_project_build_docs(state, owner, project, query).await;
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
            let workspace = match state
                .platform
                .projects
                .list_template_workspace(&owner, &project)
            {
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
                .map(|rel| {
                    state
                        .platform
                        .projects
                        .read_template_payload(&owner, &project, rel)
                })
                .transpose();
            let selected_file = match selected_file {
                Ok(file) => file,
                Err(err) => return internal_error(err),
            };

            let selected_file =
                selected_file.unwrap_or_else(|| crate::platform::model::TemplateFilePayload {
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

async fn render_project_build_docs(
    state: PlatformAppState,
    owner: String,
    project: String,
    query: TemplateWorkspaceQuery,
) -> Response {
    match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => {
            let docs = match state.platform.projects.list_project_docs(&owner, &project) {
                Ok(items) => items,
                Err(err) => return internal_error(err),
            };

            let selected_path = query
                .file
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .or_else(|| {
                    docs.iter()
                        .find(|item| item.kind == "file")
                        .map(|item| item.path.clone())
                });

            let selected_content = selected_path.as_deref().and_then(|path| {
                state
                    .platform
                    .projects
                    .read_project_doc(&owner, &project, path)
                    .ok()
            });

            let file_count = docs.iter().filter(|item| item.kind == "file").count();
            let folder_count = docs.iter().filter(|item| item.kind == "folder").count();

            let doc_items = docs
                .iter()
                .map(|item| {
                    json!({
                        "path": item.path,
                        "name": item.name,
                        "kind": item.kind,
                        "href": format!("/projects/{owner}/{project}/build/docs?file={}", item.path)
                    })
                })
                .collect::<Vec<_>>();

            let summary_items = vec![
                json!({
                    "title":"Doc Files",
                    "description": format!("{file_count} file(s) available in app/docs"),
                }),
                json!({
                    "title":"Folders",
                    "description": format!("{folder_count} folder(s) inside app/docs"),
                }),
                json!({
                    "title":"Operation Context",
                    "description":"Use list_project_docs/read_project_doc/create_project_doc from API, MCP, or assistant layers.",
                }),
            ];

            let nav = nav_classes(&owner, &project, "build", Some("docs"));
            let route = format!("/projects/{owner}/{project}/build/docs");
            let input = json!({
                "seo": {
                    "title": format!("{} - Docs", info.title),
                    "description": "Project docs and schema-like design context"
                },
                "owner": info.owner,
                "project": info.project,
                "title": info.title,
                "project_href": format!("/projects/{owner}/{project}"),
                "current_menu": "Build / Docs",
                "page_title": "Docs",
                "page_subtitle": "Project docs under app/docs (ERD, README, AGENTS, use cases).",
                "items": summary_items,
                "primary_action": {
                    "href": route,
                    "label": "Open Docs"
                },
                "docs": {
                    "enabled": true,
                    "items": doc_items,
                    "selected_path": selected_path,
                    "selected_content": selected_content.unwrap_or_default(),
                    "api": {
                        "list": format!("/api/projects/{owner}/{project}/docs"),
                        "read": format!("/api/projects/{owner}/{project}/docs/file"),
                        "create": format!("/api/projects/{owner}/{project}/docs"),
                        "agent_list": format!("/api/projects/{owner}/{project}/agent-docs"),
                        "agent_read": format!("/api/projects/{owner}/{project}/agent-docs/file"),
                        "agent_save": format!("/api/projects/{owner}/{project}/agent-docs/file"),
                    }
                },
                "nav": nav,
            });
            match render_page(&state, "platform-project-docs", &route, input) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn project_design_page(Path((owner, project)): Path<(String, String)>) -> Response {
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
    if let Err(response) = require_project_page_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::CredentialsRead,
    ) {
        return response;
    }

    match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => {
            let nav = nav_classes(&owner, &project, "credentials", None);
            let route = format!("/projects/{owner}/{project}/credentials");
            let input = json!({
                "seo": {
                    "title": format!("{} - Credentials", info.title),
                    "description": "Credential catalog and secret payload management"
                },
                "owner": info.owner,
                "project": info.project,
                "title": info.title,
                "project_href": format!("/projects/{owner}/{project}"),
                "credentials": {
                    "api": {
                        "list": format!("/api/projects/{owner}/{project}/credentials"),
                        "item_base": format!("/api/projects/{owner}/{project}/credentials"),
                    }
                },
                "nav": nav,
            });
            match render_page(&state, "platform-project-credentials", &route, input) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn project_db_connections_page(
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
            let nav = nav_classes(&owner, &project, "databases", Some("connections"));
            let route = format!("/projects/{owner}/{project}/db/connections");
            let connections = match state
                .platform
                .db_connections
                .list_project_connections(&owner, &project)
            {
                Ok(items) => items,
                Err(err) => return internal_error(err),
            };
            let connection_cards = connections
                .iter()
                .map(|item| {
                    json!({
                        "connection_id": item.connection_id,
                        "slug": item.connection_slug,
                        "name": item.connection_label,
                        "kind": item.database_kind,
                        "icon_class": db_connection_icon_class(&item.database_kind),
                        "credential_id": item.credential_id,
                        "updated_at": item.updated_at,
                        "description": if item.database_kind == "sjtable" {
                            "Project-local SekejapDB used by Simple Tables and runtime data."
                        } else {
                            "Credential-backed external database connection."
                        },
                        "path": format!(
                            "/projects/{owner}/{project}/db/{}/{}/tables",
                            item.database_kind,
                            item.connection_slug
                        )
                    })
                })
                .collect::<Vec<_>>();
            let input = json!({
                "seo": {
                    "title": format!("{} - Databases", info.title),
                    "description": "Project database connections"
                },
                "owner": info.owner,
                "project": info.project,
                "title": info.title,
                "project_href": format!("/projects/{owner}/{project}"),
                "connections": connection_cards,
                "db_connections": {
                    "api": {
                        "list": format!("/api/projects/{owner}/{project}/db/connections"),
                        "item_base": format!("/api/projects/{owner}/{project}/db/connections"),
                        "test": format!("/api/projects/{owner}/{project}/db/connections/test"),
                        "credentials_list": format!("/api/projects/{owner}/{project}/credentials"),
                    }
                },
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

async fn project_db_suite_redirect_page(
    Path((owner, project, db_kind, connection)): Path<(String, String, String, String)>,
) -> Response {
    Redirect::to(&format!(
        "/projects/{owner}/{project}/db/{db_kind}/{connection}/tables"
    ))
    .into_response()
}

async fn project_db_suite_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, db_kind, connection, tab)): Path<(
        String,
        String,
        String,
        String,
        String,
    )>,
    Query(query): Query<DbSuiteQuery>,
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

    let tab_key = match tab.as_str() {
        "tables" | "query" | "schema" | "mart" => tab,
        _ => {
            return (StatusCode::NOT_FOUND, Html("db tab not found".to_string())).into_response();
        }
    };

    match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => {
            let Some(connection_info) = (match state.platform.db_connections.get_project_connection(
                &owner,
                &project,
                &connection,
            ) {
                Ok(item) => item,
                Err(err) => return internal_error(err),
            }) else {
                return (
                    StatusCode::NOT_FOUND,
                    Html("db connection not found".to_string()),
                )
                    .into_response();
            };
            if connection_info.database_kind != db_kind {
                return (
                    StatusCode::NOT_FOUND,
                    Html("db connection not found".to_string()),
                )
                    .into_response();
            }
            let nav = nav_classes(&owner, &project, "databases", Some("connections"));
            let route = format!("/projects/{owner}/{project}/db/{db_kind}/{connection}/{tab_key}");
            let table_page_key = match connection_info.database_kind.as_str() {
                "postgresql" => "platform-project-table-connection-postgresql",
                "sjtable" => "platform-project-table-connection-sjtable",
                _ => "platform-project-table-connection",
            };

            let requested = query.table.unwrap_or_default();
            let selected_table = requested.trim().to_lowercase().replace(
                |c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != '.',
                "-",
            );
            let query_example = if connection_info.database_kind == "sjtable" {
                format!(
                    "{{\n  \"pipeline\": [\n    {{ \"op\": \"collection\", \"name\": \"sjtable__{}\" }},\n    {{ \"op\": \"take\", \"n\": 200 }}\n  ]\n}}",
                    selected_table.split('.').next_back().unwrap_or("your_table")
                )
            } else {
                "-- Write SQL and click Run Query.".to_string()
            };

            let table_query = if selected_table.is_empty() {
                String::new()
            } else {
                format!("?table={selected_table}")
            };
            let base = format!("/projects/{owner}/{project}/db/{db_kind}/{connection}");
            let suite_tabs = vec![
                json!({
                    "label": "Tables",
                    "href": format!("{base}/tables{table_query}"),
                    "classes": if tab_key == "tables" { "is-active" } else { "" },
                }),
                json!({
                    "label": "Query",
                    "href": format!("{base}/query{table_query}"),
                    "classes": if tab_key == "query" { "is-active" } else { "" },
                }),
                json!({
                    "label": "Schema",
                    "href": format!("{base}/schema{table_query}"),
                    "classes": if tab_key == "schema" { "is-active" } else { "" },
                }),
                json!({
                    "label": "Mart",
                    "href": format!("{base}/mart{table_query}"),
                    "classes": if tab_key == "mart" { "is-active" } else { "" },
                }),
            ];

            let input = json!({
                "seo": {
                    "title": format!("{} - DB {} / {}", info.title, db_kind, connection),
                    "description": "Database suite"
                },
                "owner": info.owner,
                "project": info.project,
                "title": info.title,
                "project_href": format!("/projects/{owner}/{project}"),
                "connection": {
                    "id": connection_info.connection_id,
                    "name": connection_info.connection_label,
                    "kind": db_kind,
                    "slug": connection,
                    "icon_class": db_connection_icon_class(&connection_info.database_kind),
                    "credential_id": connection_info.credential_id,
                },
                "db_runtime_api": {
                    "describe": format!("/api/projects/{owner}/{project}/db/connections/{}/describe", connection_info.connection_id),
                    "schemas": format!("/api/projects/{owner}/{project}/db/connections/{}/schemas", connection_info.connection_id),
                    "tables": format!("/api/projects/{owner}/{project}/db/connections/{}/tables", connection_info.connection_id),
                    "functions": format!("/api/projects/{owner}/{project}/db/connections/{}/functions", connection_info.connection_id),
                    "preview": format!("/api/projects/{owner}/{project}/db/connections/{}/table-preview", connection_info.connection_id),
                    "query": format!("/api/projects/{owner}/{project}/db/connections/{}/query", connection_info.connection_id),
                },
                "suite_tabs": suite_tabs,
                "object_groups": Vec::<Value>::new(),
                "tables": Vec::<Value>::new(),
                "table_summary": "Loading tables...",
                "preview": {
                    "columns": Vec::<String>::new(),
                    "rows": Vec::<Vec<String>>::new(),
                    "empty": true,
                },
                "query_example": query_example,
                "schema_text": "{}",
                "tab_flags": {
                    "tables": tab_key == "tables",
                    "query": tab_key == "query",
                    "schema": tab_key == "schema",
                    "mart": tab_key == "mart",
                },
                "nav": nav,
            });
            match render_page(&state, table_page_key, &route, input) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn project_tables_connections_legacy_redirect_page(
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    Redirect::to(&format!("/projects/{owner}/{project}/db/connections")).into_response()
}

async fn project_table_connection_legacy_redirect_page(
    Path((owner, project, connection)): Path<(String, String, String)>,
) -> Response {
    Redirect::to(&format!(
        "/projects/{owner}/{project}/db/sjtable/{connection}/tables"
    ))
    .into_response()
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
    render_settings_tab_page(state, headers, owner, project, "general".to_string()).await
}

async fn project_settings_tab_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, tab)): Path<(String, String, String)>,
) -> Response {
    render_settings_tab_page(state, headers, owner, project, tab).await
}

async fn project_settings_web_libraries_legacy_redirect_page(
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    Redirect::to(&format!("/projects/{owner}/{project}/settings/libraries")).into_response()
}

async fn render_settings_tab_page(
    state: PlatformAppState,
    headers: HeaderMap,
    owner: String,
    project: String,
    raw_tab: String,
) -> Response {
    if let Err(response) = require_project_page_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsRead,
    ) {
        return response;
    }

    let tab = normalize_settings_tab(&raw_tab);
    let tab_title = settings_tab_title(tab);
    let tab_subtitle = settings_tab_subtitle(tab);

    match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => {
            let nav = nav_classes(&owner, &project, "settings", None);
            let route = if tab == "general" {
                format!("/projects/{owner}/{project}/settings")
            } else {
                format!("/projects/{owner}/{project}/settings/{tab}")
            };

            let tabs = settings_tab_items(&owner, &project, tab);
            let general_cards = settings_general_cards(&owner, &project);
            let policy_cards = settings_policy_cards();
            let library_cards = settings_library_cards();
            let node_cards = settings_node_cards();

            let assistant_config = match state
                .platform
                .assistant_configs
                .get_project_assistant_config(&owner, &project)
            {
                Ok(config) => config,
                Err(err) => return internal_error(err),
            };

            let assistant_credentials = match state
                .platform
                .credentials
                .list_project_credentials(&owner, &project)
            {
                Ok(items) => items
                    .into_iter()
                    .filter(|item| item.kind == "openai")
                    .map(|item| {
                        json!({
                            "credential_id": item.credential_id,
                            "title": item.title,
                            "kind": item.kind
                        })
                    })
                    .collect::<Vec<_>>(),
                Err(err) => return internal_error(err),
            };

            let mcp_session = state
                .platform
                .mcp_sessions
                .get_for_project(&owner, &project);

            let input = json!({
                "seo": {
                    "title": format!("{} - Settings / {}", info.title, tab_title),
                    "description": tab_subtitle
                },
                "owner": info.owner,
                "project": info.project,
                "title": info.title,
                "project_href": format!("/projects/{owner}/{project}"),
                "current_menu": "Settings",
                "settings_tabs": tabs,
                "active_tab": tab,
                "tab_flags": {
                    "general": tab == "general",
                    "policy": tab == "policy",
                    "automatons": tab == "automatons",
                    "libraries": tab == "libraries",
                    "nodes": tab == "nodes"
                },
                "page_title": tab_title,
                "page_subtitle": tab_subtitle,
                "cards_general": general_cards,
                "cards_policy": policy_cards,
                "cards_libraries": library_cards,
                "cards_nodes": node_cards,
                "assistant": {
                    "api": {
                        "config": format!("/api/projects/{owner}/{project}/assistant/config")
                    },
                    "config": assistant_config,
                    "credentials": assistant_credentials
                },
                "mcp": {
                    "active": mcp_session.is_some(),
                    "status_label": if mcp_session.is_some() { "active" } else { "inactive" },
                    "capabilities": mcp_session
                        .as_ref()
                        .map(|session| session.capabilities.iter().map(|cap| cap.key()).collect::<Vec<_>>())
                        .unwrap_or_default()
                },
                "nav": nav,
            });
            match render_page(&state, "platform-project-settings", &route, input) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response(),
        Err(err) => internal_error(err),
    }
}

fn normalize_settings_tab(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "" | "general" => "general",
        "policy" => "policy",
        "automatons" => "automatons",
        "libraries" => "libraries",
        "nodes" => "nodes",
        _ => "general",
    }
}

fn settings_tab_title(tab: &str) -> &'static str {
    match tab {
        "policy" => "Policy",
        "automatons" => "Automatons",
        "libraries" => "Libraries",
        "nodes" => "Nodes",
        _ => "General",
    }
}

fn settings_tab_subtitle(tab: &str) -> &'static str {
    match tab {
        "policy" => "Capability boundaries, runtime constraints, and session controls.",
        "automatons" => "Assistant and automation runtime configuration per project.",
        "libraries" => "Installed web libraries and runtime package contracts.",
        "nodes" => "Live node contracts and script/tool availability.",
        _ => "Core project defaults and shared runtime switches.",
    }
}

fn settings_tab_items(owner: &str, project: &str, active: &str) -> Vec<Value> {
    let base = format!("/projects/{owner}/{project}/settings");
    let entries = [
        ("general", "General"),
        ("policy", "Policy"),
        ("automatons", "Automatons"),
        ("libraries", "Libraries"),
        ("nodes", "Nodes"),
    ];
    entries
        .iter()
        .map(|(key, label)| {
            json!({
                "key": *key,
                "label": *label,
                "href": if *key == "general" { base.clone() } else { format!("{base}/{key}") },
                "classes": if *key == active { "is-active" } else { "" }
            })
        })
        .collect::<Vec<_>>()
}

fn settings_general_cards(owner: &str, project: &str) -> Vec<Value> {
    vec![
        json!({
            "title":"Runtime Defaults",
            "description":"Project-wide defaults for execution cadence, retries, and production activation flow.",
            "href": format!("/projects/{owner}/{project}/settings/automatons"),
            "tag":"Core"
        }),
        json!({
            "title":"Connected Services",
            "description":"Credentials, DB connections, and tool-facing service contract usage.",
            "href": format!("/projects/{owner}/{project}/settings/libraries"),
            "tag":"Core"
        }),
    ]
}

fn settings_policy_cards() -> Vec<Value> {
    vec![
        json!({
            "title":"Capability Gate",
            "description":"Subject capability checks enforced across REST, MCP, and assistant channels.",
            "href":"#",
            "tag":"Access"
        }),
        json!({
            "title":"Request Boundary",
            "description":"Input validation, payload size bounds, and deterministic error contracts.",
            "href":"#",
            "tag":"Runtime"
        }),
        json!({
            "title":"Session Scope",
            "description":"Project-scoped session constraints for remote control and internal assistant execution.",
            "href":"#",
            "tag":"Session"
        }),
    ]
}

fn settings_library_cards() -> Vec<Value> {
    vec![
        json!({
            "title":"zeb/codemirror",
            "description":"Platform-managed editor dependency for template and query editing.",
            "href":"#",
            "tag":"Official"
        }),
        json!({
            "title":"zeb/graphui",
            "description":"Graph canvas package for pipeline visualization and editing.",
            "href":"#",
            "tag":"Official"
        }),
        json!({
            "title":"zeb/threejs",
            "description":"Three.js bridge runtime and reusable TSX wrapper modules.",
            "href":"#",
            "tag":"Official"
        }),
        json!({
            "title":"zeb/deckgl",
            "description":"Deck.gl bridge runtime for geospatial data surfaces.",
            "href":"#",
            "tag":"Official"
        }),
        json!({
            "title":"zeb/d3",
            "description":"D3 runtime bridge and chart wrapper catalog.",
            "href":"#",
            "tag":"Official"
        }),
    ]
}

fn settings_node_cards() -> Vec<Value> {
    let mut cards = crate::framework::nodes::builtin_node_definitions()
        .into_iter()
        .map(|def| {
            let script_tag = if def.script_available {
                "script:on"
            } else {
                "script:off"
            };
            let ai_tag = if def.ai_tool.registered {
                "ai:tool"
            } else {
                "ai:off"
            };
            json!({
                "title": format!("{} ({})", def.title, def.kind),
                "description": def.description,
                "href":"#",
                "tag": format!("{script_tag} · {ai_tag}")
            })
        })
        .collect::<Vec<_>>();
    if cards.is_empty() {
        cards.push(json!({
            "title":"No registered nodes",
            "description":"Node registry is empty.",
            "href":"#",
            "tag":"runtime"
        }));
    }
    cards
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
            Vec::new(),
        )),
        "schedules" => Some((
            "schedules",
            "Schedule Pipelines",
            "Cron-based and interval-based recurring jobs.",
            Vec::new(),
        )),
        "manual" => Some((
            "manual",
            "Manual Pipelines",
            "Pipelines triggered explicitly from API/UI manual execute requests.",
            Vec::new(),
        )),
        "functions" => Some((
            "functions",
            "Function Pipelines",
            "Callable in-house functions for reuse across workflows.",
            Vec::new(),
        )),
        _ => None,
    }
}

fn build_tab_payload(
    tab: &str,
) -> Option<(
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Vec<Value>,
)> {
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
        "docs" | "schema" => Some((
            "docs",
            "Docs",
            "Structured project docs such as ERD, app design documents, use-case maps, and data contracts.",
            "Open Docs",
            vec![
                json!({"title":"App Design Docs","description":"High-level system and interaction definitions for the project."}),
                json!({"title":"ERD & Data Models","description":"Database structure and cross-entity relationships."}),
                json!({"title":"Use Cases & Concepts","description":"Implementation notes, use cases, and conceptual design artifacts."}),
            ],
        )),
        _ => None,
    }
}

fn project_nav_map(owner: &str, project: &str) -> String {
    let b = format!("/projects/{owner}/{project}");
    let pb = format!("{b}/pipelines");
    format!(
        "  - Pipelines › Registry: {pb}/registry?path=/\n\
           - Pipelines › Webhooks: {pb}/webhooks\n\
           - Pipelines › Schedules: {pb}/schedules\n\
           - Pipelines › Manual: {pb}/manual\n\
           - Pipelines › Functions: {pb}/functions\n\
           - Build › Templates: {b}/build/templates\n\
           - Build › Assets: {b}/build/assets\n\
           - Build › Docs: {b}/build/docs\n\
           - Dashboard: {b}/dashboard\n\
           - Credentials: {b}/credentials\n\
           - Databases / Tables (lists all connections): {b}/db/connections\n\
           - Files: {b}/files\n\
           - Todo: {b}/todo\n\
           - Settings: {b}/settings\n\
         \n\
         DB connection sub-pages (substitute actual db_kind and connection_id):\n\
           - {b}/db/{{db_kind}}/{{connection_id}}/tables  — browse tables\n\
           - {b}/db/{{db_kind}}/{{connection_id}}/query   — run SQL / query UI\n\
           - {b}/db/{{db_kind}}/{{connection_id}}/schema  — schema explorer"
    )
}

fn nav_classes(owner: &str, project: &str, main: &str, pipeline_sub: Option<&str>) -> Value {
    let pipelines_base = format!("/projects/{owner}/{project}/pipelines");

    json!({
        "title": "Project Menu",
        "links": {
            "pipelines_registry": format!("{pipelines_base}/registry?path=/"),
            "pipelines_editor": format!("{pipelines_base}/editor"),
            "pipelines_webhooks": format!("{pipelines_base}/webhooks"),
            "pipelines_schedules": format!("{pipelines_base}/schedules"),
            "pipelines_manual": format!("{pipelines_base}/manual"),
            "pipelines_functions": format!("{pipelines_base}/functions"),
            "build_templates": format!("/projects/{owner}/{project}/build/templates"),
            "build_assets": format!("/projects/{owner}/{project}/build/assets"),
            "build_docs": format!("/projects/{owner}/{project}/build/docs"),
            "build_schema": format!("/projects/{owner}/{project}/build/docs"),
            "dashboard": format!("/projects/{owner}/{project}/dashboard"),
            "credentials": format!("/projects/{owner}/{project}/credentials"),
            "db_connections": format!("/projects/{owner}/{project}/db/connections"),
            "tables_connections": format!("/projects/{owner}/{project}/db/connections"),
            "files": format!("/projects/{owner}/{project}/files"),
            "todo": format!("/projects/{owner}/{project}/todo"),
            "settings": format!("/projects/{owner}/{project}/settings"),
        },
        "classes": {
            "pipelines": if main == "pipelines" { "is-active" } else { "" },
            "build": if main == "build" { "is-active" } else { "" },
            "dashboard": if main == "dashboard" { "is-active" } else { "" },
            "credentials": if main == "credentials" { "is-active" } else { "" },
            "databases": if main == "databases" { "is-active" } else { "" },
            "tables": if main == "databases" { "is-active" } else { "" },
            "files": if main == "files" { "is-active" } else { "" },
            "todo": if main == "todo" { "is-active" } else { "" },
            "settings": if main == "settings" { "is-active" } else { "" },
            "pipeline_registry": if pipeline_sub == Some("registry") { "is-active" } else { "" },
            "pipeline_editor": if pipeline_sub == Some("editor") { "is-active" } else { "" },
            "pipeline_webhooks": if pipeline_sub == Some("webhooks") { "is-active" } else { "" },
            "pipeline_schedules": if pipeline_sub == Some("schedules") { "is-active" } else { "" },
            "pipeline_manual": if pipeline_sub == Some("manual") { "is-active" } else { "" },
            "pipeline_functions": if pipeline_sub == Some("functions") { "is-active" } else { "" },
            "build_templates": if main == "build" && pipeline_sub == Some("templates") { "is-active" } else { "" },
            "build_assets": if main == "build" && pipeline_sub == Some("assets") { "is-active" } else { "" },
            "build_docs": if main == "build" && (pipeline_sub == Some("docs") || pipeline_sub == Some("schema")) { "is-active" } else { "" },
            "build_schema": if main == "build" && (pipeline_sub == Some("docs") || pipeline_sub == Some("schema")) { "is-active" } else { "" },
            "db_connections": if main == "databases" { "is-active" } else { "" },
            "table_connections": if main == "databases" { "is-active" } else { "" },
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

fn db_connection_icon_class(database_kind: &str) -> &'static str {
    match database_kind {
        "postgresql" => "devicon-postgresql-plain colored",
        "mysql" => "devicon-mysql-plain colored",
        "sqlite" => "devicon-sqlite-plain colored",
        "redis" => "devicon-redis-plain colored",
        "mongodb" => "devicon-mongodb-plain colored",
        "qdrant" => "devicon-vectorlogozone-plain",
        "sjtable" => "zf-icon-sjtable",
        _ => "zf-icon-default-db",
    }
}

fn project_web_assets_root(layout: &crate::platform::model::ProjectFileLayout) -> PathBuf {
    layout.data_runtime_dir.join("web-assets")
}

#[derive(Debug, Clone, Copy)]
struct ProjectAssetLibrarySpec {
    library: &'static str,
    version: &'static str,
    default_entry: &'static str,
    vendor_rel_paths: &'static [&'static str],
    npm_deps: &'static [(&'static str, &'static str)],
    detect_markers: &'static [&'static str],
}

impl ProjectAssetLibrarySpec {
    fn library_root(self, layout: &crate::platform::model::ProjectFileLayout) -> PathBuf {
        let mut root = layout.app_dir.join("libraries");
        for segment in self.library.split('/') {
            root = root.join(segment);
        }
        root.join(self.version)
    }
}

const PROJECT_ASSET_LIBRARY_SPECS: &[ProjectAssetLibrarySpec] = &[
    ProjectAssetLibrarySpec {
        library: "zeb/threejs",
        version: "0.1",
        default_entry: "runtime/threejs.bundle.mjs",
        vendor_rel_paths: &[
            "library.json",
            "exports.json",
            "keywords.json",
            "runtime/threejs.bundle.mjs",
            "runtime/threejs.global.js",
            "wrappers/ThreeScene.tsx",
        ],
        npm_deps: &[("three", "0.183.2")],
        detect_markers: &[
            "/assets/libraries/zeb/threejs/",
            "zeb/threejs",
            "'three'",
            "\"three\"",
        ],
    },
    ProjectAssetLibrarySpec {
        library: "zeb/deckgl",
        version: "0.1",
        default_entry: "runtime/deckgl.bundle.mjs",
        vendor_rel_paths: &[
            "library.json",
            "exports.json",
            "keywords.json",
            "runtime/deckgl.bundle.mjs",
            "wrappers/DeckMap.tsx",
        ],
        npm_deps: &[
            ("deck.gl", "9.2.10"),
            ("@deck.gl/core", "9.2.10"),
            ("@deck.gl/layers", "9.2.10"),
        ],
        detect_markers: &[
            "/assets/libraries/zeb/deckgl/",
            "zeb/deckgl",
            "'deck.gl'",
            "\"deck.gl\"",
            "@deck.gl/",
        ],
    },
    ProjectAssetLibrarySpec {
        library: "zeb/d3",
        version: "0.1",
        default_entry: "runtime/d3.bundle.mjs",
        vendor_rel_paths: &[
            "library.json",
            "exports.json",
            "keywords.json",
            "runtime/d3.bundle.mjs",
            "wrappers/D3Bars.tsx",
        ],
        npm_deps: &[("d3", "7.9.0")],
        detect_markers: &["/assets/libraries/zeb/d3/", "zeb/d3", "'d3'", "\"d3\""],
    },
    ProjectAssetLibrarySpec {
        library: "zeb/devicons",
        version: "0.1",
        default_entry: "runtime/devicons.bundle.mjs",
        vendor_rel_paths: &[
            "library.json",
            "exports.json",
            "keywords.json",
            "runtime/devicons.bundle.mjs",
            "runtime/devicons.css",
        ],
        npm_deps: &[],
        detect_markers: &[
            "/assets/libraries/zeb/devicons/",
            "zeb/devicons",
            "devicon-",
            "zf-devicon",
        ],
    },
    ProjectAssetLibrarySpec {
        library: "zeb/threejs-vrm",
        version: "0.1",
        default_entry: "runtime/threejs-vrm.bundle.mjs",
        vendor_rel_paths: &[
            "library.json",
            "exports.json",
            "keywords.json",
            "runtime/threejs-vrm.bundle.mjs",
            "wrappers/VrmViewer.tsx",
        ],
        npm_deps: &[("three", "0.183.2"), ("@pixiv/three-vrm", "3.5.0")],
        detect_markers: &[
            "/assets/libraries/zeb/threejs-vrm/",
            "zeb/threejs-vrm",
            "@pixiv/three-vrm",
            "GLTFLoader",
        ],
    },
];

fn resolve_project_asset_library_spec(
    library: &str,
    version: &str,
) -> Option<ProjectAssetLibrarySpec> {
    PROJECT_ASSET_LIBRARY_SPECS
        .iter()
        .copied()
        .find(|spec| spec.library == library && spec.version == version)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct InstalledNpmDependency {
    package: String,
    version: String,
    store_path: String,
    index_path: String,
    linked_path: String,
}

fn npm_store_root(data_root: &FsPath) -> PathBuf {
    data_root.join("mounted").join("npm-store")
}

fn encode_package_for_path(package: &str) -> String {
    package
        .replace('@', "_at_")
        .replace('/', "__")
        .replace('\\', "__")
}

fn run_process_capture_stdout(cmd: &mut std::process::Command) -> Result<String, PlatformError> {
    let output = cmd.output().map_err(|err| {
        PlatformError::new(
            "PLATFORM_LIBRARY_PROCESS",
            format!("failed spawning process: {err}"),
        )
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(PlatformError::new(
            "PLATFORM_LIBRARY_PROCESS",
            format!(
                "process exited with status {} stdout='{}' stderr='{}'",
                output.status, stdout, stderr
            ),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn ensure_npm_packaged_dependency(
    store_root: &FsPath,
    package: &str,
    version: &str,
) -> Result<(PathBuf, PathBuf), PlatformError> {
    let encoded = encode_package_for_path(package);
    let package_dir = store_root
        .join("packages")
        .join(&encoded)
        .join(version)
        .join("package");
    let index_path = store_root
        .join("indexes")
        .join(format!("{encoded}@{version}.exports.json"));
    if package_dir.is_dir() && index_path.is_file() {
        return Ok((package_dir, index_path));
    }

    std::fs::create_dir_all(store_root.join("packages").join(&encoded).join(version))?;
    std::fs::create_dir_all(store_root.join("tarballs"))?;
    std::fs::create_dir_all(store_root.join("indexes"))?;
    std::fs::create_dir_all(store_root.join("tmp"))?;

    let spec = format!("{package}@{version}");
    let npm_path = std::process::Command::new("npm")
        .arg("--version")
        .output()
        .map_err(|err| {
            PlatformError::new(
                "PLATFORM_LIBRARY_NPM_MISSING",
                format!("npm is required but unavailable: {err}"),
            )
        })?;
    if !npm_path.status.success() {
        return Err(PlatformError::new(
            "PLATFORM_LIBRARY_NPM_MISSING",
            "npm command is unavailable",
        ));
    }

    let pack_stdout = run_process_capture_stdout(
        std::process::Command::new("npm")
            .arg("pack")
            .arg(&spec)
            .arg("--pack-destination")
            .arg(store_root.join("tarballs")),
    )?;
    let tarball_name = pack_stdout
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .next_back()
        .ok_or_else(|| {
            PlatformError::new(
                "PLATFORM_LIBRARY_NPM_PACK",
                format!("npm pack produced empty output for '{spec}'"),
            )
        })?;
    let tarball = store_root.join("tarballs").join(tarball_name);
    if !tarball.is_file() {
        return Err(PlatformError::new(
            "PLATFORM_LIBRARY_NPM_PACK",
            format!(
                "npm pack output tarball missing for '{}' (expected '{}')",
                spec,
                tarball.display()
            ),
        ));
    }

    let tmp_extract_dir = store_root.join("tmp").join(format!(
        "{}-{}-{}",
        encoded,
        version,
        crate::platform::model::now_ts()
    ));
    if tmp_extract_dir.exists() {
        std::fs::remove_dir_all(&tmp_extract_dir)?;
    }
    std::fs::create_dir_all(&tmp_extract_dir)?;
    let _ = run_process_capture_stdout(
        std::process::Command::new("tar")
            .arg("-xzf")
            .arg(&tarball)
            .arg("-C")
            .arg(&tmp_extract_dir),
    )?;
    let extracted_package = tmp_extract_dir.join("package");
    if !extracted_package.is_dir() {
        return Err(PlatformError::new(
            "PLATFORM_LIBRARY_NPM_PACK",
            format!(
                "tar extraction for '{}' missing 'package/' folder at '{}'",
                spec,
                extracted_package.display()
            ),
        ));
    }

    if package_dir.exists() {
        std::fs::remove_dir_all(&package_dir)?;
    }
    if let Some(parent) = package_dir.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(&extracted_package, &package_dir)?;
    let _ = std::fs::remove_dir_all(&tmp_extract_dir);

    build_package_declaration_index(&package_dir, &index_path, package, version)?;
    Ok((package_dir, index_path))
}

fn collect_files_recursively_with_exts(
    root: &FsPath,
    exts: &[&str],
    out: &mut Vec<PathBuf>,
) -> Result<(), PlatformError> {
    if !root.exists() {
        return Ok(());
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let ft = entry.file_type()?;
            if ft.is_symlink() {
                continue;
            }
            if ft.is_dir() {
                stack.push(path);
                continue;
            }
            if ft.is_file() {
                let ext = path
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or("");
                let filename = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("");
                if exts.iter().any(|item| {
                    item.eq_ignore_ascii_case(ext)
                        || filename
                            .to_ascii_lowercase()
                            .ends_with(&item.to_ascii_lowercase())
                }) {
                    out.push(path);
                }
            }
        }
    }
    Ok(())
}

fn extract_export_symbols_from_ts_declaration(source: &str) -> BTreeSet<String> {
    let mut symbols = BTreeSet::new();
    let keywords = [
        "const",
        "function",
        "class",
        "interface",
        "type",
        "enum",
        "namespace",
        "let",
        "var",
    ];
    for line in source.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("export ") {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("export {") {
            let block = rest.split('}').next().unwrap_or_default();
            for item in block.split(',') {
                let name = item.trim();
                if name.is_empty() {
                    continue;
                }
                let alias = name.split_whitespace().next().unwrap_or_default();
                if !alias.is_empty() {
                    symbols.insert(alias.to_string());
                }
            }
            continue;
        }
        for keyword in keywords {
            let needle = format!("export {keyword} ");
            if let Some(rest) = trimmed.strip_prefix(&needle) {
                let ident: String = rest
                    .chars()
                    .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '$')
                    .collect();
                if !ident.is_empty() {
                    symbols.insert(ident);
                }
            }
            let declare_needle = format!("export declare {keyword} ");
            if let Some(rest) = trimmed.strip_prefix(&declare_needle) {
                let ident: String = rest
                    .chars()
                    .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '$')
                    .collect();
                if !ident.is_empty() {
                    symbols.insert(ident);
                }
            }
        }
    }
    symbols
}

fn build_package_declaration_index(
    package_dir: &FsPath,
    index_path: &FsPath,
    package: &str,
    version: &str,
) -> Result<(), PlatformError> {
    let mut declaration_files = Vec::new();
    collect_files_recursively_with_exts(
        package_dir,
        &["d.ts", "d.mts", "d.cts"],
        &mut declaration_files,
    )?;
    declaration_files.sort();
    let mut all_symbols = BTreeSet::new();
    let mut files_meta = Vec::new();
    for file in declaration_files.iter().take(600) {
        let rel = file
            .strip_prefix(package_dir)
            .unwrap_or(file)
            .to_string_lossy()
            .replace('\\', "/");
        let source = std::fs::read_to_string(file).unwrap_or_default();
        let symbols = extract_export_symbols_from_ts_declaration(&source);
        for symbol in &symbols {
            all_symbols.insert(symbol.clone());
        }
        files_meta.push(json!({
            "path": rel,
            "export_count": symbols.len()
        }));
    }
    if let Some(parent) = index_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(
        index_path,
        serde_json::to_vec_pretty(&json!({
            "schema_version": "0.1",
            "package": package,
            "version": version,
            "generated_at": crate::platform::model::now_ts(),
            "total_declaration_files": declaration_files.len(),
            "total_exports": all_symbols.len(),
            "exports": all_symbols.into_iter().collect::<Vec<_>>(),
            "files": files_meta,
        }))?,
    )?;
    Ok(())
}

fn link_dependency_into_project_node_modules(
    layout: &crate::platform::model::ProjectFileLayout,
    package: &str,
    package_dir: &FsPath,
) -> Result<PathBuf, PlatformError> {
    let node_modules = layout.app_dir.join("node_modules");
    std::fs::create_dir_all(&node_modules)?;
    let mut dest = node_modules.clone();
    for segment in package.split('/') {
        dest = dest.join(segment);
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if dest.exists() || std::fs::symlink_metadata(&dest).is_ok() {
        if dest.is_dir() && !std::fs::symlink_metadata(&dest)?.file_type().is_symlink() {
            std::fs::remove_dir_all(&dest)?;
        } else {
            std::fs::remove_file(&dest)?;
        }
    }
    let target = std::fs::canonicalize(package_dir).unwrap_or_else(|_| package_dir.to_path_buf());
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, &dest).map_err(|err| {
        PlatformError::new(
            "PLATFORM_LIBRARY_LINK",
            format!(
                "failed linking '{}' -> '{}': {err}",
                dest.display(),
                target.display()
            ),
        )
    })?;
    #[cfg(not(unix))]
    {
        return Err(PlatformError::new(
            "PLATFORM_LIBRARY_LINK",
            "symlink-based node_modules linking is only implemented for unix targets",
        ));
    }
    Ok(dest)
}

fn update_project_libraries_lock(
    layout: &crate::platform::model::ProjectFileLayout,
    spec: ProjectAssetLibrarySpec,
    installed_deps: &[InstalledNpmDependency],
) -> Result<(), PlatformError> {
    let lock_path = layout.app_dir.join("libraries.lock.json");
    let mut root = if lock_path.is_file() {
        let raw = std::fs::read_to_string(&lock_path).unwrap_or_default();
        serde_json::from_str::<serde_json::Value>(&raw).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    let libraries_obj = root
        .as_object_mut()
        .map(|obj| obj.entry("libraries").or_insert_with(|| json!({})))
        .and_then(|value| value.as_object_mut())
        .ok_or_else(|| {
            PlatformError::new(
                "PLATFORM_LIBRARY_LOCK",
                "libraries.lock.json has invalid shape",
            )
        })?;

    let key = format!("{}@{}", spec.library, spec.version);
    libraries_obj.insert(
        key,
        json!({
            "library": spec.library,
            "version": spec.version,
            "updated_at": crate::platform::model::now_ts(),
            "npm_deps": installed_deps,
        }),
    );

    root["schema_version"] = json!("0.1");
    root["updated_at"] = json!(crate::platform::model::now_ts());

    std::fs::write(lock_path, serde_json::to_vec_pretty(&root)?)?;
    Ok(())
}

fn ensure_library_npm_dependencies(
    data_root: &FsPath,
    layout: &crate::platform::model::ProjectFileLayout,
    spec: ProjectAssetLibrarySpec,
) -> Result<Vec<InstalledNpmDependency>, PlatformError> {
    let store_root = npm_store_root(data_root);
    std::fs::create_dir_all(&store_root)?;
    let mut installed = Vec::new();
    for (package, version) in spec.npm_deps {
        let (package_dir, index_path) =
            ensure_npm_packaged_dependency(&store_root, package, version)?;
        let linked_path = link_dependency_into_project_node_modules(layout, package, &package_dir)?;
        installed.push(InstalledNpmDependency {
            package: (*package).to_string(),
            version: (*version).to_string(),
            store_path: package_dir.display().to_string(),
            index_path: index_path.display().to_string(),
            linked_path: linked_path.display().to_string(),
        });
    }
    update_project_libraries_lock(layout, spec, &installed)?;
    Ok(installed)
}

fn detect_required_project_library_specs(
    templates_root: &FsPath,
) -> Result<Vec<ProjectAssetLibrarySpec>, PlatformError> {
    let mut files = Vec::new();
    collect_files_recursively_with_exts(
        templates_root,
        &["tsx", "ts", "jsx", "js", "mjs"],
        &mut files,
    )?;
    let mut detected = BTreeSet::new();
    for path in files {
        let source = match std::fs::read_to_string(&path) {
            Ok(source) => source,
            Err(_) => continue,
        };
        for spec in PROJECT_ASSET_LIBRARY_SPECS {
            if spec
                .detect_markers
                .iter()
                .any(|marker| source.contains(marker))
            {
                detected.insert((spec.library, spec.version));
            }
        }
        if let Ok(imports) = parse_module_imports_with_swc(&path, &source) {
            for import in imports {
                if import == "three" {
                    detected.insert(("zeb/threejs", "0.1"));
                }
                if import == "d3" || import.starts_with("d3-") {
                    detected.insert(("zeb/d3", "0.1"));
                }
                if import == "@pixiv/three-vrm" {
                    detected.insert(("zeb/threejs-vrm", "0.1"));
                }
                if import == "deck.gl" || import.starts_with("@deck.gl/") {
                    detected.insert(("zeb/deckgl", "0.1"));
                }
            }
        }
    }
    Ok(detected
        .into_iter()
        .filter_map(|(library, version)| resolve_project_asset_library_spec(library, version))
        .collect())
}

fn trigger_project_asset_prepare_on_template_save(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    layout: &crate::platform::model::ProjectFileLayout,
) -> Result<(), PlatformError> {
    let detected_specs = detect_required_project_library_specs(&layout.app_templates_dir)?;
    for spec in detected_specs {
        let _ = prepare_project_assets_manifest(
            owner,
            project,
            &state.platform.config.data_root,
            layout,
            PrepareProjectAssetsRequest {
                library: spec.library.to_string(),
                version: spec.version.to_string(),
                entries: Vec::new(),
            },
        )?;
    }
    Ok(())
}

fn prepare_project_assets_manifest(
    owner: &str,
    project: &str,
    data_root: &FsPath,
    layout: &crate::platform::model::ProjectFileLayout,
    req: PrepareProjectAssetsRequest,
) -> Result<ProjectAssetManifest, PlatformError> {
    let library = req.library.trim().to_string();
    let version = req.version.trim().to_string();
    let Some(spec) = resolve_project_asset_library_spec(&library, &version) else {
        let supported = PROJECT_ASSET_LIBRARY_SPECS
            .iter()
            .map(|item| format!("{}@{}", item.library, item.version))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(PlatformError::new(
            "PLATFORM_ASSET_PREPARE_UNSUPPORTED",
            format!(
                "library '{}' version '{}' is unsupported. supported: {supported}",
                library, version
            ),
        ));
    };
    let entries = if req.entries.is_empty() {
        vec![spec.default_entry.to_string()]
    } else {
        req.entries
    };

    // Install flow: resolve deps/versions, download+extract into mounted npm store,
    // build declaration/export index, and link into project app/node_modules.
    let _installed_deps = ensure_library_npm_dependencies(data_root, layout, spec)?;

    // Scaffold step: vendor curated library bridge assets into project workspace.
    let library_root = spec.library_root(layout);
    materialize_vendor_library(spec, &library_root)?;

    // Scaffold step: build simple chunk graph from vendored entry modules via SWC import parsing.
    let assets_root = project_web_assets_root(layout);
    let rwe_assets_root = assets_root.join("rwe");
    std::fs::create_dir_all(rwe_assets_root.join("chunks"))?;

    let mut entry_items = Vec::new();
    for entry in entries {
        let rel = entry.trim().replace('\\', "/");
        if rel.is_empty() || rel.contains("..") {
            return Err(PlatformError::new(
                "PLATFORM_ASSET_ENTRY_INVALID",
                "entry path is invalid",
            ));
        }
        let entry_abs = library_root.join(&rel);
        if !entry_abs.is_file() {
            return Err(PlatformError::new(
                "PLATFORM_ASSET_ENTRY_MISSING",
                format!("entry '{}' not found in vendored library", rel),
            ));
        }

        let graph = build_module_graph_with_swc(&entry_abs, &library_root)?;
        let mut chunk_source = String::new();
        for module_rel in &graph.modules {
            let module_abs = library_root.join(module_rel);
            let source = std::fs::read_to_string(&module_abs)?;
            chunk_source.push_str(&format!("// module: {module_rel}\n{source}\n"));
        }
        let chunk_source = compact_chunk_javascript(&chunk_source);
        let chunk_id = stable_fnv64_hex(&chunk_source);
        let chunk_rel = format!("rwe/chunks/{chunk_id}.mjs");
        let chunk_abs = assets_root.join(&chunk_rel);
        if let Some(parent) = chunk_abs.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&chunk_abs, chunk_source)?;

        let chunk_url = format!("/assets/{owner}/{project}/{chunk_rel}");
        let entry_item = ProjectAssetEntryItem {
            entry: rel.clone(),
            path: format!("app/libraries/{}/{}/{rel}", spec.library, spec.version),
            url: chunk_url.clone(),
            chunks: vec![ProjectAssetChunkItem {
                chunk_id,
                module_count: graph.modules.len(),
                modules: graph.modules.clone(),
                path: format!("data/runtime/web-assets/{chunk_rel}"),
                url: chunk_url,
            }],
            imports: graph.imports,
        };
        entry_items.push(entry_item);
    }

    let manifest_abs = assets_root.join("rwe").join("manifest.json");
    let mut manifest = match std::fs::read(&manifest_abs) {
        Ok(bytes) => {
            serde_json::from_slice::<ProjectAssetManifest>(&bytes).unwrap_or(ProjectAssetManifest {
                schema_version: "0.1".to_string(),
                owner: owner.to_string(),
                project: project.to_string(),
                generated_at: crate::platform::model::now_ts(),
                strategy: "swc-import-graph-scaffold".to_string(),
                libraries: Vec::new(),
            })
        }
        Err(_) => ProjectAssetManifest {
            schema_version: "0.1".to_string(),
            owner: owner.to_string(),
            project: project.to_string(),
            generated_at: crate::platform::model::now_ts(),
            strategy: "swc-import-graph-scaffold".to_string(),
            libraries: Vec::new(),
        },
    };
    manifest.schema_version = "0.1".to_string();
    manifest.owner = owner.to_string();
    manifest.project = project.to_string();
    manifest.generated_at = crate::platform::model::now_ts();
    manifest.strategy = "swc-import-graph-scaffold".to_string();
    if let Some(item) = manifest
        .libraries
        .iter_mut()
        .find(|item| item.library == library && item.version == version)
    {
        item.entries = entry_items;
    } else {
        manifest.libraries.push(ProjectAssetLibraryItem {
            library,
            version,
            entries: entry_items,
        });
    }
    manifest
        .libraries
        .sort_by(|a, b| (&a.library, &a.version).cmp(&(&b.library, &b.version)));
    if let Some(parent) = manifest_abs.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&manifest_abs, serde_json::to_vec_pretty(&manifest)?)?;
    Ok(manifest)
}

fn materialize_vendor_library(
    spec: ProjectAssetLibrarySpec,
    library_root: &FsPath,
) -> Result<(), PlatformError> {
    for rel in spec.vendor_rel_paths {
        let catalog = format!("{}/{}/{}", spec.library, spec.version, rel);
        let Some(bytes) = platform_library_asset(&catalog) else {
            return Err(PlatformError::new(
                "PLATFORM_ASSET_VENDOR_SOURCE_MISSING",
                format!("embedded library asset '{catalog}' not found"),
            ));
        };
        let abs = library_root.join(rel);
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(abs, bytes)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
struct ModuleGraphSummary {
    modules: Vec<String>,
    imports: Vec<String>,
}

fn build_module_graph_with_swc(
    entry_abs: &FsPath,
    root: &FsPath,
) -> Result<ModuleGraphSummary, PlatformError> {
    let mut queue = VecDeque::new();
    let mut seen = BTreeSet::new();
    let mut ordered_modules = Vec::new();
    let mut imports = BTreeSet::new();

    queue.push_back(entry_abs.to_path_buf());
    while let Some(module_abs) = queue.pop_front() {
        let Ok(rel) = module_abs.strip_prefix(root) else {
            continue;
        };
        let rel_norm = rel.to_string_lossy().replace('\\', "/");
        if !seen.insert(rel_norm.clone()) {
            continue;
        }
        ordered_modules.push(rel_norm.clone());
        let source = std::fs::read_to_string(&module_abs)?;
        let specs = parse_module_imports_with_swc(&module_abs, &source)?;
        for spec in specs {
            if spec.starts_with("./") || spec.starts_with("../") {
                if let Some(next) = resolve_relative_module(&module_abs, &spec) {
                    queue.push_back(next);
                }
            } else {
                imports.insert(spec);
            }
        }
    }

    Ok(ModuleGraphSummary {
        modules: ordered_modules,
        imports: imports.into_iter().collect(),
    })
}

fn parse_module_imports_with_swc(
    path: &FsPath,
    source: &str,
) -> Result<Vec<String>, PlatformError> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(
        FileName::Real(path.to_path_buf()).into(),
        source.to_string(),
    );
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx: true,
            decorators: true,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let module = parser.parse_module().map_err(|err| {
        PlatformError::new(
            "PLATFORM_ASSET_SWC_PARSE",
            format!("failed parsing '{}': {err:?}", path.display()),
        )
    })?;

    let mut imports = Vec::new();
    for item in module.body {
        match item {
            ModuleItem::ModuleDecl(decl) => match decl {
                ModuleDecl::Import(import) => {
                    imports.push(import.src.value.to_string_lossy().to_string())
                }
                ModuleDecl::ExportAll(export) => {
                    imports.push(export.src.value.to_string_lossy().to_string())
                }
                ModuleDecl::ExportNamed(export) => {
                    if let Some(src) = export.src {
                        imports.push(src.value.to_string_lossy().to_string());
                    }
                }
                _ => {}
            },
            ModuleItem::Stmt(stmt) => {
                // Include simple dynamic `import("...")` calls in import graph.
                if let swc_ecma_ast::Stmt::Expr(expr_stmt) = stmt
                    && let Expr::Call(call) = *expr_stmt.expr
                    && matches!(call.callee, Callee::Import(_))
                    && let Some(first) = call.args.first()
                    && let Expr::Lit(swc_ecma_ast::Lit::Str(s)) = &*first.expr
                {
                    imports.push(s.value.to_string_lossy().to_string());
                }
            }
        }
    }
    Ok(imports)
}

fn resolve_relative_module(base_abs: &FsPath, spec: &str) -> Option<PathBuf> {
    let parent = base_abs.parent()?;
    let stem = parent.join(spec);
    let candidates = [
        stem.clone(),
        stem.with_extension("mjs"),
        stem.with_extension("js"),
        stem.with_extension("ts"),
        stem.with_extension("tsx"),
        stem.join("index.mjs"),
        stem.join("index.js"),
        stem.join("index.ts"),
        stem.join("index.tsx"),
    ];
    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn stable_fnv64_hex(input: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in input.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x00000100000001B3);
    }
    format!("{h:016x}")
}

fn compact_chunk_javascript(source: &str) -> String {
    let mut out = String::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("// module:") {
            continue;
        }
        out.push_str(trimmed);
        out.push('\n');
    }
    out
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

/// Public, machine-readable node contract extracted from built-in node registry.
async fn docs_node_contract() -> Response {
    let items = crate::framework::nodes::builtin_node_definitions()
        .into_iter()
        .map(crate::framework::NodeContractItem::from)
        .collect::<Vec<_>>();
    Json(crate::framework::NodeContractDocument {
        ok: true,
        schema_version: "0.1",
        source: "framework::nodes::builtin_node_definitions",
        items,
    })
    .into_response()
}

/// Public, machine-readable operation contract extracted from `platform::operations`.
async fn docs_operation_contract() -> Response {
    Json(crate::platform::operations::OperationContractDocument {
        ok: true,
        schema_version: "0.1",
        source: "platform::operations::OPERATIONS",
        items: crate::platform::operations::operation_contract_items(),
    })
    .into_response()
}

async fn api_list_node_definitions(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsRead,
    ) {
        return response;
    }
    Json(json!({
        "ok": true,
        "items": crate::framework::nodes::builtin_node_definitions()
    }))
    .into_response()
}

// ── Admin DB endpoints ──────────────────────────────────────────────────────

fn require_superadmin(state: &PlatformAppState, headers: &HeaderMap) -> Result<(), Response> {
    let Some(owner) = session_owner(headers) else {
        return Err(StatusCode::UNAUTHORIZED.into_response());
    };
    let is_superadmin = state
        .platform
        .users
        .get_user(&owner)
        .ok()
        .flatten()
        .map(|u| u.role == "superadmin")
        .unwrap_or(false);
    if !is_superadmin {
        return Err(StatusCode::FORBIDDEN.into_response());
    }
    Ok(())
}

async fn api_admin_db_list_collections(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
) -> Response {
    if let Err(r) = require_superadmin(&state, &headers) {
        return r;
    }
    match state.platform.data.admin_list_collections() {
        Ok(collections) => Json(json!({
            "ok": true,
            "collections": collections.into_iter().map(|(name, count)| json!({"name": name, "count": count})).collect::<Vec<_>>()
        }))
        .into_response(),
        Err(err) => internal_error(err),
    }
}

#[derive(serde::Deserialize)]
struct AdminDbQueryRequest {
    pipeline: serde_json::Value,
}

async fn api_admin_db_query(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Json(req): Json<AdminDbQueryRequest>,
) -> Response {
    if let Err(r) = require_superadmin(&state, &headers) {
        return r;
    }
    let q = json!({"pipeline": req.pipeline}).to_string();
    match state.platform.data.admin_raw_query(&q) {
        Ok(rows) => {
            let count = rows.len();
            Json(json!({"ok": true, "rows": rows, "count": count})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_admin_db_get_node(
    State(state): State<PlatformAppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Err(r) = require_superadmin(&state, &headers) {
        return r;
    }
    match state.platform.data.admin_get_node(&slug) {
        Ok(Some(raw)) => {
            let v: serde_json::Value =
                serde_json::from_str(&raw).unwrap_or(serde_json::Value::String(raw));
            Json(json!({"ok": true, "slug": slug, "node": v})).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "error": {"code": "NODE_NOT_FOUND", "message": format!("Node not found: {slug}")}})),
        )
            .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_admin_db_delete_node(
    State(state): State<PlatformAppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Err(r) = require_superadmin(&state, &headers) {
        return r;
    }
    match state.platform.data.admin_delete_node(&slug) {
        Ok(deleted) => {
            Json(json!({"ok": true, "deleted": deleted, "slug": slug})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

// ────────────────────────────────────────────────────────────────────────────

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

async fn api_prepare_project_assets(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<PrepareProjectAssetsRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::LibrariesInstall,
    ) {
        return response;
    }

    let owner_slug = crate::platform::model::slug_segment(&owner);
    let project_slug = crate::platform::model::slug_segment(&project);
    if owner_slug.is_empty() || project_slug.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code":"PLATFORM_ASSET_SCOPE_INVALID","message":"owner/project must not be empty"}})),
        )
            .into_response();
    }

    let layout = match state
        .platform
        .file
        .ensure_project_layout(&owner_slug, &project_slug)
    {
        Ok(layout) => layout,
        Err(err) => return internal_error(err),
    };

    match prepare_project_assets_manifest(
        &owner_slug,
        &project_slug,
        &state.platform.config.data_root,
        &layout,
        req,
    ) {
        Ok(manifest) => Json(json!({
            "ok": true,
            "manifest": manifest,
            "manifest_url": format!("/assets/{owner_slug}/{project_slug}/rwe/manifest.json")
        }))
        .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_pipeline_registry(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Query(query): Query<PipelineRegistryQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return response;
    }
    let scope = match resolve_pipeline_registry_scope(&query) {
        Ok(scope) => scope,
        Err(response) => return response,
    };
    let base_route = format!("/projects/{owner}/{project}/pipelines/registry");
    match scope {
        PipelineRegistryScope::Path => {
            let current_path = query.path.as_deref().unwrap_or("/");
            match state.platform.projects.list_pipeline_registry(
                &owner,
                &project,
                current_path,
                &base_route,
            ) {
                Ok(listing) => {
                    Json(json!({"ok": true, "scope": "path", "listing": listing})).into_response()
                }
                Err(err) => internal_error(err),
            }
        }
        PipelineRegistryScope::Project => match state
            .platform
            .projects
            .list_pipeline_meta_rows(&owner, &project)
        {
            Ok(rows) => {
                let items = rows
                    .into_iter()
                    .map(|meta| {
                        let file_rel_path = meta.file_rel_path.clone();
                        json!({
                            "id": file_rel_path,
                            "name": meta.name,
                            "title": meta.title,
                            "description": meta.description,
                            "trigger_kind": meta.trigger_kind,
                            "virtual_path": meta.virtual_path,
                            "file_rel_path": meta.file_rel_path
                        })
                    })
                    .collect::<Vec<_>>();
                let count = items.len();
                Json(json!({
                    "ok": true,
                    "scope": "project",
                    "items": items,
                    "count": count
                }))
                .into_response()
            }
            Err(err) => internal_error(err),
        },
    }
}

async fn api_list_pipelines(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Query(query): Query<PipelineListQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return response;
    }
    let base_path =
        crate::platform::model::normalize_virtual_path(query.path.as_deref().unwrap_or("/"));
    let recursive = query.recursive.unwrap_or(false);
    match state
        .platform
        .projects
        .list_pipeline_meta_rows(&owner, &project)
    {
        Ok(rows) => {
            let items = rows
                .into_iter()
                .filter(|meta| pipeline_path_matches(&base_path, &meta.virtual_path, recursive))
                .map(|meta| {
                    json!({
                        "id": meta.file_rel_path,
                        "meta": meta
                    })
                })
                .collect::<Vec<_>>();
            Json(json!({
                "ok": true,
                "path": base_path,
                "recursive": recursive,
                "items": items
            }))
            .into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_get_pipeline_by_id(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Query(query): Query<PipelineByIdQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return response;
    }
    let Some(file_id) = query
        .id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    else {
        return (
            StatusCode::BAD_REQUEST,
            Json(
                json!({"ok": false, "error": {"code":"PLATFORM_PIPELINE_ID_MISSING", "message":"query.id is required"}}),
            ),
        )
            .into_response();
    };

    let meta = match state
        .platform
        .projects
        .get_pipeline_meta_by_file_id(&owner, &project, file_id)
    {
        Ok(Some(meta)) => meta,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(
                    json!({"ok": false, "error": {"code":"PLATFORM_PIPELINE_MISSING", "message":"pipeline not found"}}),
                ),
            )
                .into_response();
        }
        Err(err) => return internal_error(err),
    };

    let include_source = query.include_source.unwrap_or(true);
    let include_active_source = query.include_active_source.unwrap_or(false);
    let source = if include_source {
        match state
            .platform
            .projects
            .read_pipeline_source(&owner, &project, &meta.file_rel_path)
        {
            Ok(source) => Some(source),
            Err(err) => return internal_error(err),
        }
    } else {
        None
    };
    let active_source = if include_active_source {
        match state
            .platform
            .projects
            .read_active_pipeline_source(&owner, &project, &meta)
        {
            Ok(source) => Some(source),
            Err(err) if err.code == "PLATFORM_PIPELINE_NOT_ACTIVE" => None,
            Err(err) => return internal_error(err),
        }
    } else {
        None
    };
    let locked = if let Some(source_text) = source.as_deref() {
        pipeline_source_is_locked(source_text)
    } else {
        match state
            .platform
            .projects
            .read_pipeline_source(&owner, &project, &meta.file_rel_path)
        {
            Ok(source_text) => pipeline_source_is_locked(&source_text),
            Err(_) => false,
        }
    };

    Json(json!({
        "ok": true,
        "id": meta.file_rel_path,
        "meta": meta,
        "locked": locked,
        "source": source,
        "active_source": active_source,
        "hits": state
            .platform
            .pipeline_hits
            .get(&owner, &project, &meta.file_rel_path)
    }))
    .into_response()
}

async fn api_upsert_pipeline_definition(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<UpsertPipelineDefinitionRequest>,
) -> Response {
    // Milestone 1: allow direct pipeline creation even without authenticated session.
    if session_owner(&headers).is_some()
        && let Err(response) = require_project_api_capability(
            &state,
            &headers,
            &owner,
            &project,
            ProjectCapability::PipelinesWrite,
        )
    {
        return response;
    }

    let existing_meta = match state.platform.projects.get_pipeline_meta(
        &owner,
        &project,
        &req.virtual_path,
        &req.name,
    ) {
        Ok(meta) => meta,
        Err(err) => return internal_error(err),
    };
    if let Some(meta) = existing_meta {
        let locked = match state.platform.projects.read_pipeline_source(
            &owner,
            &project,
            &meta.file_rel_path,
        ) {
            Ok(source) => pipeline_source_is_locked(&source),
            Err(_) => false,
        };
        if locked {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "ok": false,
                    "error": {
                        "code": "PLATFORM_PIPELINE_LOCKED",
                        "message": "pipeline is locked and cannot be edited"
                    }
                })),
            )
                .into_response();
        }
    }

    match state.platform.projects.upsert_pipeline_definition(
        &owner,
        &project,
        &req.virtual_path,
        &req.name,
        &req.title,
        &req.description,
        &req.trigger_kind,
        &req.source,
    ) {
        Ok(meta) => Json(json!({"ok": true, "meta": meta})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_activate_pipeline_definition(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<PipelineLocateRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }

    match state.platform.projects.activate_pipeline_definition(
        &owner,
        &project,
        &req.virtual_path,
        &req.name,
    ) {
        Ok(meta) => {
            if let Err(err) = state.platform.pipeline_runtime.refresh_pipeline(
                &owner,
                &project,
                &req.virtual_path,
                &req.name,
            ) {
                return internal_error(err);
            }
            Json(json!({"ok": true, "meta": meta})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_deactivate_pipeline_definition(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<PipelineLocateRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }

    match state.platform.projects.deactivate_pipeline_definition(
        &owner,
        &project,
        &req.virtual_path,
        &req.name,
    ) {
        Ok(meta) => {
            if let Err(err) = state.platform.pipeline_runtime.refresh_pipeline(
                &owner,
                &project,
                &req.virtual_path,
                &req.name,
            ) {
                return internal_error(err);
            }
            Json(json!({"ok": true, "meta": meta})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_execute_pipeline(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<ExecutePipelineRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesExecute,
    ) {
        return response;
    }

    let meta = match state.platform.projects.get_pipeline_meta(
        &owner,
        &project,
        &req.virtual_path,
        &req.name,
    ) {
        Ok(Some(meta)) => meta,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(
                    json!({"ok": false, "error": {"code":"PLATFORM_PIPELINE_MISSING", "message":"pipeline not found"}}),
                ),
            )
                .into_response();
        }
        Err(err) => return internal_error(err),
    };

    let source = match state
        .platform
        .projects
        .read_active_pipeline_source(&owner, &project, &meta)
    {
        Ok(source) => source,
        Err(err) if err.code == "PLATFORM_PIPELINE_NOT_ACTIVE" => {
            state.platform.pipeline_hits.record_failure(
                &owner,
                &project,
                &meta.file_rel_path,
                "api.execute",
                err.code,
                "pipeline must be activated before execution",
            );
            return (
                StatusCode::CONFLICT,
                Json(
                    json!({"ok": false, "error": {"code": err.code, "message":"pipeline must be activated before execution"}}),
                ),
            )
                .into_response();
        }
        Err(err) => return internal_error(err),
    };

    let mut graph: PipelineGraph = match serde_json::from_str(&source) {
        Ok(graph) => graph,
        Err(err) => {
            state.platform.pipeline_hits.record_failure(
                &owner,
                &project,
                &meta.file_rel_path,
                "api.execute",
                "PLATFORM_PIPELINE_PARSE",
                &err.to_string(),
            );
            return (
                StatusCode::BAD_REQUEST,
                Json(
                    json!({"ok": false, "error": {"code":"PLATFORM_PIPELINE_PARSE", "message": err.to_string()}}),
                ),
            )
                .into_response();
        }
    };
    if let Err(err) = hydrate_web_render_markup_from_templates(&state, &owner, &project, &mut graph)
    {
        state.platform.pipeline_hits.record_failure(
            &owner,
            &project,
            &meta.file_rel_path,
            "api.execute",
            err.code,
            &err.message,
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
        )
            .into_response();
    }

    if let Err(message) = validate_execute_trigger(&graph, &req) {
        state.platform.pipeline_hits.record_failure(
            &owner,
            &project,
            &meta.file_rel_path,
            "api.execute",
            "PLATFORM_PIPELINE_TRIGGER_MISMATCH",
            &message,
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(
                json!({"ok": false, "error": {"code":"PLATFORM_PIPELINE_TRIGGER_MISMATCH", "message": message}}),
            ),
        )
            .into_response();
    }

    let request_id = format!(
        "pipeline-exec-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    let credentials = state.platform.credentials.clone();
    let simple_tables = state.platform.simple_tables.clone();
    let graph_for_run = graph.clone();
    let ctx = FrameworkContext {
        owner: owner.clone(),
        project: project.clone(),
        pipeline: graph.id.clone(),
        request_id: request_id.clone(),
        input: req.input.clone(),
    };
    let engine = BasicFrameworkEngine::new(
        Arc::new(DenoSandboxEngine::default()),
        state.frontend.rwe.clone(),
        Some(credentials),
        Some(simple_tables),
    );
    match engine.execute_async(&graph_for_run, &ctx).await {
        Ok(output) => {
            state
                .platform
                .pipeline_hits
                .record_success(&owner, &project, &meta.file_rel_path);
            Json(json!({
                "ok": true,
                "meta": meta,
                "output": output.value,
                "trace": output.trace
            }))
            .into_response()
        }
        Err(err) => {
            state.platform.pipeline_hits.record_failure(
                &owner,
                &project,
                &meta.file_rel_path,
                "api.execute",
                err.code,
                &err.message,
            );
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
            )
                .into_response()
        }
    }
}

async fn api_pipeline_hits(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return response;
    }

    match state
        .platform
        .projects
        .list_pipeline_meta_rows(&owner, &project)
    {
        Ok(rows) => {
            let items = rows
                .into_iter()
                .map(|meta| {
                    let file_id = meta.file_rel_path.clone();
                    json!({
                        "id": file_id,
                        "name": meta.name,
                        "virtual_path": meta.virtual_path,
                        "file_rel_path": meta.file_rel_path,
                        "stats": state
                            .platform
                            .pipeline_hits
                            .get(&owner, &project, &meta.file_rel_path)
                    })
                })
                .collect::<Vec<_>>();
            Json(json!({
                "ok": true,
                "items": items,
                "count": items.len()
            }))
            .into_response()
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
    match state
        .platform
        .projects
        .list_template_workspace(&owner, &project)
    {
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
    match state
        .platform
        .projects
        .read_template_payload(&owner, &project, path)
    {
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
    match state
        .platform
        .projects
        .write_template_file(&owner, &project, &req)
    {
        Ok(file) => {
            let owner_slug = crate::platform::model::slug_segment(&owner);
            let project_slug = crate::platform::model::slug_segment(&project);
            if let Ok(layout) = state
                .platform
                .file
                .ensure_project_layout(&owner_slug, &project_slug)
            {
                if let Err(err) = trigger_project_asset_prepare_on_template_save(
                    &state,
                    &owner_slug,
                    &project_slug,
                    &layout,
                ) {
                    eprintln!(
                        "warning: template save asset prepare failed for {}/{}: {} ({})",
                        owner_slug, project_slug, err.code, err.message
                    );
                }
            }
            Json(file).into_response()
        }
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
    match state
        .platform
        .projects
        .create_template_entry(&owner, &project, &req)
    {
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
    match state
        .platform
        .projects
        .move_template_entry(&owner, &project, &req)
    {
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
    match state
        .platform
        .projects
        .delete_template_entry(&owner, &project, path)
    {
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
    match state
        .platform
        .projects
        .list_template_git_status(&owner, &project)
    {
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

async fn api_list_credentials(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::CredentialsRead,
    ) {
        return response;
    }
    match state
        .platform
        .credentials
        .list_project_credentials(&owner, &project)
    {
        Ok(items) => Json(json!({"ok": true, "items": items})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_get_credential(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, credential_id)): Path<(String, String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::CredentialsRead,
    ) {
        return response;
    }
    match state
        .platform
        .credentials
        .get_project_credential(&owner, &project, &credential_id)
    {
        Ok(Some(credential)) => Json(json!({"ok": true, "credential": credential})).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"ok": false, "error": {"code":"PLATFORM_CREDENTIAL_MISSING","message":"credential not found"}}))).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_upsert_credential(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<UpsertProjectCredentialRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::CredentialsWrite,
    ) {
        return response;
    }
    match state
        .platform
        .credentials
        .upsert_project_credential(&owner, &project, &req)
    {
        Ok(credential) => Json(json!({"ok": true, "credential": credential})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_upsert_credential_by_path(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, credential_id)): Path<(String, String, String)>,
    Json(mut req): Json<UpsertProjectCredentialRequest>,
) -> Response {
    req.credential_id = credential_id;
    api_upsert_credential(State(state), headers, Path((owner, project)), Json(req)).await
}

async fn api_delete_credential(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, credential_id)): Path<(String, String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::CredentialsWrite,
    ) {
        return response;
    }
    match state
        .platform
        .credentials
        .delete_project_credential(&owner, &project, &credential_id)
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_get_project_assistant_config(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsRead,
    ) {
        return response;
    }
    match state
        .platform
        .assistant_configs
        .get_project_assistant_config(&owner, &project)
    {
        Ok(config) => Json(json!({"ok": true, "config": config})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_upsert_project_assistant_config(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<UpsertProjectAssistantConfigRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsWrite,
    ) {
        return response;
    }
    match state
        .platform
        .assistant_configs
        .upsert_project_assistant_config(&owner, &project, &req)
    {
        Ok(config) => Json(json!({"ok": true, "config": config})).into_response(),
        Err(err) => internal_error(err),
    }
}

/// Load up to `max_pairs * 2` chat messages from the project's runtime data dir.
fn load_chat_history(
    file: &Arc<dyn crate::platform::adapters::file::FileAdapter>,
    owner: &str,
    project: &str,
) -> Vec<Value> {
    let layout = match file.ensure_project_layout(owner, project) {
        Ok(l) => l,
        Err(_) => return vec![],
    };
    let path = layout.data_runtime_dir.join("chat_history.json");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    serde_json::from_str::<Vec<Value>>(&content).unwrap_or_default()
}

/// Append a user+assistant exchange and persist to disk, keeping the last `max_pairs` pairs.
fn save_chat_history(
    file: &Arc<dyn crate::platform::adapters::file::FileAdapter>,
    owner: &str,
    project: &str,
    user_msg: &str,
    assistant_msg: &str,
    max_pairs: usize,
) {
    let layout = match file.ensure_project_layout(owner, project) {
        Ok(l) => l,
        Err(_) => return,
    };
    let path = layout.data_runtime_dir.join("chat_history.json");
    let mut history: Vec<Value> =
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default();
    history.push(json!({"role": "user", "content": user_msg}));
    history.push(json!({"role": "assistant", "content": assistant_msg}));
    // Keep last max_pairs pairs = max_pairs * 2 messages
    let keep = max_pairs * 2;
    if history.len() > keep {
        history.drain(0..history.len() - keep);
    }
    if let Ok(json) = serde_json::to_string(&history) {
        std::fs::create_dir_all(&layout.data_runtime_dir).ok();
        std::fs::write(&path, json).ok();
    }
}

async fn api_project_assistant_chat(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<AssistantChatRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::ProjectRead,
    ) {
        return response;
    }

    let message = req.message.trim().to_string();
    if message.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": {
                    "code": "ASSISTANT_MESSAGE_INVALID",
                    "message": "message must not be empty"
                }
            })),
        )
            .into_response();
    }

    let bundle = match load_project_assistant_llm(state.platform.data.as_ref(), &owner, &project) {
        Ok(bundle) => bundle,
        Err(err) => {
            let status = match err.code {
                "ASSISTANT_NOT_CONFIGURED"
                | "ASSISTANT_DISABLED"
                | "ASSISTANT_NO_LLM"
                | "ASSISTANT_CREDENTIAL_MISSING"
                | "ASSISTANT_CREDENTIAL_INVALID" => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            return (
                status,
                Json(json!({
                    "ok": false,
                    "error": { "code": err.code, "message": err.message }
                })),
            )
                .into_response();
        }
    };

    let tools = crate::platform::services::AssistantPlatformTools::new(
        state.platform.clone(),
        &owner,
        &project,
    );

    let tool_defs = crate::platform::services::AssistantPlatformTools::tool_defs();

    let mut messages: Vec<Value> = Vec::new();
    {
        let skills = crate::platform::skills::all_skills();
        let skills_text = crate::platform::skills::format_skills_for_system_prompt(skills);
        let page_context = req.current_page
            .as_deref()
            .filter(|p| !p.is_empty())
            .map(|p| format!("\nCurrently viewing: {p}"))
            .unwrap_or_default();
        let time_context = req.client_time
            .as_deref()
            .filter(|t| !t.is_empty())
            .map(|t| format!("\nUser local time: {t}"))
            .unwrap_or_default();
        let nav_map = project_nav_map(&owner, &project);

        // Load agent docs for system prompt
        state.platform.projects.ensure_agent_docs_defaults(&owner, &project).ok();
        let memory = state.platform.projects.read_agent_doc(&owner, &project, "MEMORY.md").unwrap_or_default();
        let soul   = state.platform.projects.read_agent_doc(&owner, &project, "SOUL.md").unwrap_or_default();
        let agents = state.platform.projects.read_agent_doc(&owner, &project, "AGENTS.md").unwrap_or_default();
        let readme = state.platform.projects.read_project_doc(&owner, &project, "README.md").unwrap_or_default();
        let readme_section = if readme.trim().is_empty() {
            String::new()
        } else {
            format!("\n\n## Project README\n{readme}")
        };

        let system = format!(
            "You are the Zebflow project assistant.\n\
             Project: {owner}/{project}{page_context}{time_context}\n\n\
             ## Your Memory\n{memory}\n\n\
             ## Your Soul\n{soul}\n\n\
             ## Project Context\n{agents}{readme_section}\n\n\
             ## Behavior\n\
             When you discover important project information (architecture, decisions, patterns, \
             data models), check Your Memory above. If it is not already recorded, call \
             `update_memory` with the full updated MEMORY.md content.\n\n\
             ## Navigation Rules\n\
             Use `navigate_to` with these exact URLs.\n\
             ALWAYS navigate first — never use tools to fetch data that a page already shows.\n\
             \n\
             For database queries:\n\
             - Always call `describe_db_connection` first to see the schema tree.\n\
             - Then call `get_table_columns` for EACH table you will touch — NEVER guess column names.\n\
             - Use `run_db_query` to execute SQL and get results (also shows in the browser UI for the user).\n\
             Do NOT use `query_table` for interactive queries — use `run_db_query` instead.\n\
             \n\
             Available pages:\n\
             {nav_map}\n\n\
             ## Zebflow Knowledge\n\n{skills_text}"
        );
        messages.push(json!({"role": "system", "content": system}));
    }

    // Server-side history takes precedence; fall back to client-sent history if empty.
    let server_history = load_chat_history(&state.platform.file, &owner, &project);
    let history_source: Box<dyn Iterator<Item = Value>> = if server_history.is_empty() {
        // Fall back to client-sent history (first session or no persistence yet)
        Box::new(
            req.history
                .into_iter()
                .filter(|item| !item.content.trim().is_empty())
                .filter_map(|item| match item.role.as_str() {
                    "user" | "assistant" => Some(json!({"role": item.role, "content": item.content.trim()})),
                    _ => None,
                })
                .take(20),
        )
    } else {
        Box::new(server_history.into_iter().take(20))
    };
    for item in history_source {
        messages.push(item);
    }
    messages.push(json!({"role": "user", "content": message}));

    let llm = if req.use_high_model {
        bundle.high.clone()
    } else {
        bundle.general.clone()
    };

    let max_steps = bundle.max_steps;
    let chat_history_pairs_for_save = bundle.chat_history_pairs;
    let model_tier = if req.use_high_model { "high" } else { "general" };

    // Channel for streaming step events to SSE
    let (step_tx, mut step_rx) =
        tokio::sync::mpsc::unbounded_channel::<AssistantStepEvent>();

    // Spawn the agentic loop as a background task
    let loop_task = tokio::spawn(async move {
        run_assistant_loop(llm, &tools, tool_defs, messages, max_steps, &step_tx).await
    });

    // Collect all SSE events: start + step events + message + done
    let owner_clone = owner.clone();
    let project_clone = project.clone();
    let file_for_history = state.platform.file.clone();
    let message_for_history = message.clone();

    let sse_stream = async_stream::stream! {
        // start event
        yield Ok::<Event, Infallible>(
            Event::default().event("start").data(
                json!({
                    "ok": true,
                    "owner": owner_clone,
                    "project": project_clone,
                    "model_tier": model_tier,
                    "max_steps": max_steps,
                })
                .to_string(),
            ),
        );

        // Drain step events while loop is running
        let mut budget_exhausted = false;
        let mut steps_taken: u32 = 0;
        let final_content;

        loop {
            match step_rx.recv().await {
                Some(AssistantStepEvent::ToolCall { step, tool, args, thought }) => {
                    steps_taken = step;
                    yield Ok(Event::default().event("tool_call").data(
                        json!({
                            "step": step,
                            "tool": tool,
                            "args": args,
                            "thought": thought,
                        }).to_string()
                    ));
                }
                Some(AssistantStepEvent::ToolResult { step, tool, result_preview }) => {
                    steps_taken = step;
                    yield Ok(Event::default().event("tool_result").data(
                        json!({
                            "step": step,
                            "tool": tool,
                            "result_preview": result_preview,
                        }).to_string()
                    ));
                }
                Some(AssistantStepEvent::Navigate { url, label }) => {
                    yield Ok(Event::default().event("navigate").data(
                        json!({ "url": url, "label": label }).to_string()
                    ));
                }
                Some(AssistantStepEvent::FillInput { selector, value, submit }) => {
                    yield Ok(Event::default().event("fill_input").data(
                        json!({ "selector": selector, "value": value, "submit": submit }).to_string()
                    ));
                }
                Some(AssistantStepEvent::InteractionSequence { id, label, steps }) => {
                    yield Ok(Event::default().event("interaction_sequence").data(
                        json!({ "id": id, "label": label, "steps": steps }).to_string()
                    ));
                }
                Some(AssistantStepEvent::BudgetExhausted) => {
                    budget_exhausted = true;
                }
                Some(AssistantStepEvent::Done(content)) => {
                    final_content = content;
                    break;
                }
                None => {
                    // Channel closed (loop task ended without Done — shouldn't happen)
                    final_content = String::new();
                    break;
                }
            }
        }

        // Await loop task to make sure it's fully done
        let _ = loop_task.await;

        // Persist this exchange to server-side chat history (last 10 pairs)
        if !final_content.is_empty() {
            save_chat_history(
                &file_for_history,
                &owner_clone,
                &project_clone,
                &message_for_history,
                &final_content,
                chat_history_pairs_for_save as usize,
            );
        }

        let content_html = crate::rwe::processors::markdown::render_markdown_fragment(&final_content);
        yield Ok(Event::default().event("message").data(
            json!({"role":"assistant","content": final_content, "content_html": content_html}).to_string()
        ));
        yield Ok(Event::default().event("done").data(
            json!({"ok": true, "steps_taken": steps_taken, "budget_exhausted": budget_exhausted}).to_string()
        ));
    };

    Sse::new(sse_stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response()
}

/// Events emitted by the assistant agentic loop.
enum AssistantStepEvent {
    ToolCall {
        step: u32,
        tool: String,
        args: Value,
        thought: String,
    },
    ToolResult {
        step: u32,
        tool: String,
        result_preview: String,
    },
    Navigate {
        url: String,
        label: String,
    },
    FillInput {
        selector: String,
        value: String,
        submit: bool,
    },
    InteractionSequence {
        id: String,
        label: String,
        steps: Value,
    },
    BudgetExhausted,
    Done(String),
}

async fn run_assistant_loop(
    llm: std::sync::Arc<dyn crate::automaton::llm_interface::LlmCall>,
    tools: &crate::platform::services::AssistantPlatformTools,
    tool_defs: Vec<crate::automaton::llm_interface::ToolDef>,
    mut messages: Vec<Value>,
    max_steps: u32,
    step_tx: &tokio::sync::mpsc::UnboundedSender<AssistantStepEvent>,
) -> String {
    use crate::automaton::llm_interface::CallResult;

    for step in 1..=max_steps {
        let result = match llm.call_with_tools(messages.clone(), &tool_defs).await {
            Ok(r) => r,
            Err(err) => {
                let content = format!("(LLM error: {err})");
                let _ = step_tx.send(AssistantStepEvent::Done(content.clone()));
                return content;
            }
        };

        match result {
            CallResult::Text(content) => {
                let _ = step_tx.send(AssistantStepEvent::Done(content.clone()));
                return content;
            }
            CallResult::ToolCalls(calls) => {
                // Append the assistant's tool_calls message to history
                let tool_calls_json: Vec<Value> = calls
                    .iter()
                    .map(|tc| {
                        json!({
                            "id": tc.id,
                            "type": "function",
                            "function": { "name": tc.name, "arguments": tc.arguments }
                        })
                    })
                    .collect();
                messages.push(json!({
                    "role": "assistant",
                    "content": null,
                    "tool_calls": tool_calls_json
                }));

                // Execute each call and append tool result messages
                for tc in &calls {
                    let args: Value =
                        serde_json::from_str(&tc.arguments).unwrap_or(json!({}));

                    let _ = step_tx.send(AssistantStepEvent::ToolCall {
                        step,
                        tool: tc.name.clone(),
                        args: args.clone(),
                        thought: String::new(),
                    });

                    let tool_result = tools.run_async(&tc.name, &args).await;
                    let result_str = tool_result.text.clone();

                    // Side-effect events for interactive tools
                    if let Some(seq) = tool_result.interaction {
                        // Interaction sequence takes priority over navigate/fill events
                        let id = seq["id"].as_str().unwrap_or(&tc.id).to_string();
                        let label = seq["label"].as_str().unwrap_or(&tc.name).to_string();
                        let steps = seq["steps"].clone();
                        let _ = step_tx.send(AssistantStepEvent::InteractionSequence {
                            id,
                            label,
                            steps,
                        });
                    } else if tc.name == "navigate_to" {
                        if let Some(url) = args["url"].as_str() {
                            let label = args["label"].as_str().unwrap_or(url).to_string();
                            let _ = step_tx.send(AssistantStepEvent::Navigate {
                                url: url.to_string(),
                                label,
                            });
                        }
                    } else if tc.name == "fill_input" {
                        if let Some(selector) = args["selector"].as_str() {
                            let value = args["value"].as_str().unwrap_or("").to_string();
                            let submit = args["submit"].as_bool().unwrap_or(false);
                            let _ = step_tx.send(AssistantStepEvent::FillInput {
                                selector: selector.to_string(),
                                value,
                                submit,
                            });
                        }
                    }

                    let result_preview = result_str.chars().take(500).collect::<String>();

                    let _ = step_tx.send(AssistantStepEvent::ToolResult {
                        step,
                        tool: tc.name.clone(),
                        result_preview,
                    });

                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": tc.id,
                        "content": result_str
                    }));
                }
            }
        }
    }

    // Budget exhausted
    let _ = step_tx.send(AssistantStepEvent::BudgetExhausted);
    let content = "(max steps reached)".to_string();
    let _ = step_tx.send(AssistantStepEvent::Done(content.clone()));
    content
}

async fn api_list_db_connections(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    match state
        .platform
        .db_connections
        .list_project_connections(&owner, &project)
    {
        Ok(items) => Json(json!({"ok": true, "items": items})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_get_db_connection(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_slug)): Path<(String, String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    match state
        .platform
        .db_connections
        .get_project_connection(&owner, &project, &connection_slug)
    {
        Ok(Some(connection)) => Json(json!({"ok": true, "connection": connection})).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "error": {"code":"PLATFORM_DB_CONNECTION_MISSING","message":"db connection not found"}})),
        )
            .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_upsert_db_connection(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<UpsertProjectDbConnectionRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesWrite,
    ) {
        return response;
    }
    match state
        .platform
        .db_connections
        .upsert_project_connection(&owner, &project, &req)
    {
        Ok(connection) => Json(json!({"ok": true, "connection": connection})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_upsert_db_connection_by_path(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_slug)): Path<(String, String, String)>,
    Json(mut req): Json<UpsertProjectDbConnectionRequest>,
) -> Response {
    req.connection_slug = connection_slug;
    api_upsert_db_connection(State(state), headers, Path((owner, project)), Json(req)).await
}

async fn api_delete_db_connection(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_slug)): Path<(String, String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesWrite,
    ) {
        return response;
    }
    match state.platform.db_connections.delete_project_connection(
        &owner,
        &project,
        &connection_slug,
    ) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_test_db_connection(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<TestProjectDbConnectionRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    match state
        .platform
        .db_connections
        .test_project_connection(&owner, &project, &req)
        .await
    {
        Ok(result) => Json(json!({"ok": true, "result": result})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_describe_db_connection(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_id)): Path<(String, String, String)>,
    Query(query): Query<DbDescribeQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    let req = DescribeProjectDbConnectionRequest {
        scope: query.scope,
        schema: query.schema,
        include_system: query.include_system,
    };
    match state
        .platform
        .db_runtime
        .describe_connection(&owner, &project, &connection_id, &req)
        .await
    {
        Ok(result) => Json(json!({"ok": true, "result": result})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_list_db_connection_schemas(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_id)): Path<(String, String, String)>,
    Query(query): Query<DbObjectListQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    let req = DescribeProjectDbConnectionRequest {
        scope: Some("schemas".to_string()),
        schema: query.schema,
        include_system: query.include_system,
    };
    match state
        .platform
        .db_runtime
        .describe_connection(&owner, &project, &connection_id, &req)
        .await
    {
        Ok(result) => Json(json!({"ok": true, "result": result})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_list_db_connection_tables(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_id)): Path<(String, String, String)>,
    Query(query): Query<DbObjectListQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    let req = DescribeProjectDbConnectionRequest {
        scope: Some("tables".to_string()),
        schema: query.schema,
        include_system: query.include_system,
    };
    match state
        .platform
        .db_runtime
        .describe_connection(&owner, &project, &connection_id, &req)
        .await
    {
        Ok(result) => Json(json!({"ok": true, "result": result})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_list_db_connection_functions(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_id)): Path<(String, String, String)>,
    Query(query): Query<DbObjectListQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    let req = DescribeProjectDbConnectionRequest {
        scope: Some("functions".to_string()),
        schema: query.schema,
        include_system: query.include_system,
    };
    match state
        .platform
        .db_runtime
        .describe_connection(&owner, &project, &connection_id, &req)
        .await
    {
        Ok(result) => Json(json!({"ok": true, "result": result})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_preview_db_connection_table(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_id)): Path<(String, String, String)>,
    Query(query): Query<DbTablePreviewQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    let table = query.table.unwrap_or_default();
    if table.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code":"PLATFORM_DB_QUERY_INVALID","message":"query.table is required"}})),
        )
            .into_response();
    }
    let req = QueryProjectDbConnectionRequest {
        table: Some(table.split('.').next_back().unwrap_or(&table).to_string()),
        sql: format!(
            "SELECT * FROM {} LIMIT {}",
            quote_sql_identifier_path(&table),
            query.limit.unwrap_or(120).clamp(1, 5000)
        ),
        limit: Some(query.limit.unwrap_or(120).clamp(1, 5000)),
        read_only: Some(true),
        ..Default::default()
    };
    match state
        .platform
        .db_runtime
        .query_connection(&owner, &project, &connection_id, &req)
        .await
    {
        Ok(result) => Json(json!({"ok": true, "result": result})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_query_db_connection(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_id)): Path<(String, String, String)>,
    Json(req): Json<QueryProjectDbConnectionRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    match state
        .platform
        .db_runtime
        .query_connection(&owner, &project, &connection_id, &req)
        .await
    {
        Ok(result) => Json(json!({"ok": true, "result": result})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_list_project_docs(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::ProjectRead,
    ) {
        return response;
    }

    match state.platform.projects.list_project_docs(&owner, &project) {
        Ok(items) => Json(json!({"ok": true, "items": items})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_read_project_doc(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Query(query): Query<DocPathQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::ProjectRead,
    ) {
        return response;
    }
    let Some(path) = query
        .path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code":"PLATFORM_DOC_PATH","message":"missing docs path"}})),
        )
            .into_response();
    };

    match state
        .platform
        .projects
        .read_project_doc(&owner, &project, path)
    {
        Ok(content) => Json(json!({
            "ok": true,
            "doc": {
                "path": path,
                "content": content
            }
        }))
        .into_response(),
        Err(err) if err.code == "PLATFORM_DOC_MISSING" => (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
        )
            .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_upsert_project_doc(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<UpsertProjectDocRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::FilesWrite,
    ) {
        return response;
    }

    match state
        .platform
        .projects
        .upsert_project_doc(&owner, &project, &req.path, &req.content)
    {
        Ok(item) => Json(json!({"ok": true, "doc": item})).into_response(),
        Err(err) if err.code == "PLATFORM_DOC_PATH" => (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
        )
            .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_upsert_project_doc_file(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Query(query): Query<DocPathQuery>,
    body: Bytes,
) -> Response {
    let Some(path) = query
        .path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code":"PLATFORM_DOC_PATH","message":"missing docs path"}})),
        )
            .into_response();
    };
    let req = UpsertProjectDocRequest {
        path: path.to_string(),
        content: String::from_utf8(body.to_vec()).unwrap_or_default(),
    };
    api_upsert_project_doc(State(state), headers, Path((owner, project)), Json(req)).await
}

async fn api_list_agent_docs(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::ProjectRead,
    ) {
        return response;
    }
    match state.platform.projects.list_agent_docs(&owner, &project) {
        Ok(items) => Json(json!({"ok": true, "items": items})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_read_agent_doc(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Query(query): Query<DocPathQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::ProjectRead,
    ) {
        return response;
    }
    let Some(name) = query.path.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code":"PLATFORM_AGENT_DOC_INVALID","message":"missing name query param"}})),
        )
            .into_response();
    };
    match state.platform.projects.read_agent_doc(&owner, &project, name) {
        Ok(content) => Json(json!({"ok": true, "doc": {"name": name, "content": content}})).into_response(),
        Err(err) if err.code == "PLATFORM_AGENT_DOC_INVALID" => (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
        )
            .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_upsert_agent_doc_file(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Query(query): Query<DocPathQuery>,
    body: Bytes,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::FilesWrite,
    ) {
        return response;
    }
    let Some(name) = query.path.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code":"PLATFORM_AGENT_DOC_INVALID","message":"missing name query param"}})),
        )
            .into_response();
    };
    // Only user-editable docs can be written via REST (not MEMORY.md)
    if name == "MEMORY.md" {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"ok": false, "error": {"code":"PLATFORM_AGENT_DOC_READONLY","message":"MEMORY.md is managed by the assistant and cannot be written via REST"}})),
        )
            .into_response();
    }
    let content = String::from_utf8(body.to_vec()).unwrap_or_default();
    match state.platform.projects.upsert_agent_doc(&owner, &project, name, &content) {
        Ok(_) => Json(json!({"ok": true, "name": name})).into_response(),
        Err(err) if err.code == "PLATFORM_AGENT_DOC_INVALID" => (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
        )
            .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_list_simple_tables(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    match state.platform.simple_tables.list_tables(&owner, &project) {
        Ok(items) => Json(json!({"ok": true, "items": items})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_create_simple_table(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<CreateSimpleTableRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesWrite,
    ) {
        return response;
    }
    match state
        .platform
        .simple_tables
        .create_table(&owner, &project, &req)
    {
        Ok(table) => Json(json!({"ok": true, "table": table})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_get_simple_table(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, table)): Path<(String, String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    match state.platform.simple_tables.get_table(&owner, &project, &table) {
        Ok(Some(table)) => Json(json!({"ok": true, "table": table})).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"ok": false, "error": {"code":"PLATFORM_SIMPLE_TABLE_MISSING","message":"simple table not found"}}))).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_delete_simple_table(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, table)): Path<(String, String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesWrite,
    ) {
        return response;
    }
    match state
        .platform
        .simple_tables
        .delete_table(&owner, &project, &table)
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_upsert_simple_table_row(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<UpsertSimpleTableRowRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesWrite,
    ) {
        return response;
    }
    match state
        .platform
        .simple_tables
        .upsert_row(&owner, &project, &req)
    {
        Ok(row) => Json(json!({"ok": true, "row": row})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_query_simple_table_rows(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<SimpleTableQueryRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    match state
        .platform
        .simple_tables
        .query_rows(&owner, &project, &req)
    {
        Ok(result) => Json(json!({"ok": true, "result": result})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn public_webhook_ingress(
    State(state): State<PlatformAppState>,
    Path((owner, project, tail)): Path<(String, String, String)>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let owner = crate::platform::model::slug_segment(&owner);
    let project = crate::platform::model::slug_segment(&project);
    let path = format!("/{}", tail.trim_start_matches('/'));
    let method_key = method.as_str().to_ascii_uppercase();

    struct Candidate {
        compiled: crate::platform::services::pipeline_runtime::CompiledPipeline,
        path_params: serde_json::Map<String, Value>,
        static_segments: usize,
        dynamic_segments: usize,
        total_segments: usize,
    }

    let mut candidates = Vec::<Candidate>::new();
    for compiled in state
        .platform
        .pipeline_runtime
        .list_project(&owner, &project)
    {
        for trigger in &compiled.webhook_triggers {
            if !trigger.method.eq_ignore_ascii_case(&method_key) {
                continue;
            }
            let Some(path_match) = match_webhook_path(&trigger.path, &path) else {
                continue;
            };
            candidates.push(Candidate {
                compiled: compiled.clone(),
                path_params: path_match.params,
                static_segments: path_match.static_segments,
                dynamic_segments: path_match.dynamic_segments,
                total_segments: path_match.total_segments,
            });
        }
    }

    candidates.sort_by(|a, b| {
        b.static_segments
            .cmp(&a.static_segments)
            .then(a.dynamic_segments.cmp(&b.dynamic_segments))
            .then(b.total_segments.cmp(&a.total_segments))
            .then(a.compiled.file_rel_path.cmp(&b.compiled.file_rel_path))
    });

    let Some(selected) = candidates.into_iter().next() else {
        return (StatusCode::NOT_FOUND, Html("not found".to_string())).into_response();
    };
    let mut graph = selected.compiled.graph.clone();
    if let Err(err) = hydrate_web_render_markup_from_templates(&state, &owner, &project, &mut graph)
    {
        state.platform.pipeline_hits.record_failure(
            &owner,
            &project,
            &selected.compiled.file_rel_path,
            "webhook.ingress",
            err.code,
            &err.message,
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
        )
            .into_response();
    }

    let request_id = format!(
        "webhook-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    let input =
        build_webhook_ingress_input(&method, &uri, &headers, &body, &path, &selected.path_params);

    let credentials = state.platform.credentials.clone();
    let simple_tables = state.platform.simple_tables.clone();
    let graph_for_run = graph.clone();
    let ctx = FrameworkContext {
        owner: owner.clone(),
        project: project.clone(),
        pipeline: graph.id.clone(),
        request_id: request_id.clone(),
        input: input.clone(),
    };
    let file_rel_path = selected.compiled.file_rel_path.clone();
    let engine = BasicFrameworkEngine::new(
        Arc::new(DenoSandboxEngine::default()),
        state.frontend.rwe.clone(),
        Some(credentials),
        Some(simple_tables),
    );
    let output = match engine.execute_async(&graph_for_run, &ctx).await {
        Ok(output) => output,
        Err(err) => {
            state.platform.pipeline_hits.record_failure(
                &owner,
                &project,
                &file_rel_path,
                "webhook.ingress",
                err.code,
                &err.message,
            );
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
            )
                .into_response();
        }
    };
    state
        .platform
        .pipeline_hits
        .record_success(&owner, &project, &file_rel_path);

    if let Some(html) = output.value.get("html").and_then(Value::as_str) {
        let scripts = output
            .value
            .get("compiled_scripts")
            .cloned()
            .and_then(|value| serde_json::from_value::<Vec<CompiledScript>>(value).ok())
            .unwrap_or_default();
        let externalized =
            externalize_rwe_scripts(&state, html, &scripts, Some((&owner, &project)));
        return Html(externalized).into_response();
    }

    if let Some(code) = output.value.get("status").and_then(Value::as_u64) {
        let status = StatusCode::from_u16(code as u16).unwrap_or(StatusCode::OK);
        if let Some(body_value) = output.value.get("body") {
            if let Some(text) = body_value.as_str() {
                return (status, text.to_string()).into_response();
            }
            return (status, Json(body_value.clone())).into_response();
        }
        return (status, Json(json!({"ok": true}))).into_response();
    }

    Json(json!({"ok": true, "output": output.value, "trace": output.trace})).into_response()
}

async fn public_webhook_ingress_root(
    State(state): State<PlatformAppState>,
    Path((owner, project)): Path<(String, String)>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    public_webhook_ingress(
        State(state),
        Path((owner, project, String::new())),
        method,
        uri,
        headers,
        body,
    )
    .await
}

fn build_webhook_ingress_input(
    method: &Method,
    uri: &Uri,
    headers: &HeaderMap,
    body: &Bytes,
    path: &str,
    path_params: &serde_json::Map<String, Value>,
) -> Value {
    let query = parse_query_to_json(uri.query());
    let params = Value::Object(path_params.clone());
    let method_value = method.as_str().to_string();

    let attach_context = |obj: &mut serde_json::Map<String, Value>| {
        if let Value::Object(path_map) = &params {
            for (key, value) in path_map {
                obj.entry(key.clone()).or_insert_with(|| value.clone());
            }
        }
        obj.insert("query".to_string(), query.clone());
        obj.insert("params".to_string(), params.clone());
        obj.insert("path".to_string(), Value::String(path.to_string()));
        obj.insert("method".to_string(), Value::String(method_value.clone()));
        obj.insert(
            "ctx".to_string(),
            json!({
                "path": path,
                "method": method.as_str(),
                "query": query,
                "params": params,
            }),
        );
    };

    if method == Method::GET && body.is_empty() {
        let mut obj = match query.clone() {
            Value::Object(map) => map,
            _ => serde_json::Map::new(),
        };
        attach_context(&mut obj);
        return Value::Object(obj);
    }

    if body.is_empty() {
        let mut obj = serde_json::Map::new();
        attach_context(&mut obj);
        return Value::Object(obj);
    }

    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    if content_type.contains("application/json")
        && let Ok(parsed) = serde_json::from_slice::<Value>(body)
    {
        if let Value::Object(mut obj) = parsed {
            attach_context(&mut obj);
            return Value::Object(obj);
        }
        return json!({
            "body": parsed,
            "query": query,
            "params": params,
            "path": path,
            "method": method.as_str(),
            "ctx": {
                "path": path,
                "method": method.as_str(),
                "query": query,
                "params": params
            }
        });
    }

    let body_text = String::from_utf8_lossy(body).to_string();
    json!({
        "body": body_text,
        "query": query,
        "params": params,
        "path": path,
        "method": method.as_str(),
        "ctx": {
            "path": path,
            "method": method.as_str(),
            "query": query,
            "params": params
        }
    })
}

fn parse_query_to_json(raw_query: Option<&str>) -> Value {
    let mut map = serde_json::Map::new();
    let Some(raw_query) = raw_query else {
        return Value::Object(map);
    };
    for pair in raw_query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let mut split = pair.splitn(2, '=');
        let key = split.next().unwrap_or_default().trim();
        if key.is_empty() {
            continue;
        }
        let value = split.next().unwrap_or_default().trim();
        map.insert(key.to_string(), Value::String(value.to_string()));
    }
    Value::Object(map)
}

fn quote_sql_identifier_path(raw: &str) -> String {
    raw.split('.')
        .filter(|part| !part.trim().is_empty())
        .map(|part| format!("\"{}\"", part.trim().replace('\"', "\"\"")))
        .collect::<Vec<_>>()
        .join(".")
}

#[derive(Debug)]
struct WebhookPathMatch {
    params: serde_json::Map<String, Value>,
    static_segments: usize,
    dynamic_segments: usize,
    total_segments: usize,
}

fn match_webhook_path(pattern: &str, actual: &str) -> Option<WebhookPathMatch> {
    let normalized_pattern = normalize_webhook_path(pattern);
    let normalized_actual = normalize_webhook_path(actual);

    let pattern_segments = split_webhook_segments(&normalized_pattern);
    let actual_segments = split_webhook_segments(&normalized_actual);
    if pattern_segments.len() != actual_segments.len() {
        return None;
    }

    let mut params = serde_json::Map::new();
    let mut static_segments = 0usize;
    let mut dynamic_segments = 0usize;

    for (pattern_seg, actual_seg) in pattern_segments.iter().zip(actual_segments.iter()) {
        if let Some(name) = path_param_name(pattern_seg) {
            dynamic_segments += 1;
            params.insert(name.to_string(), Value::String((*actual_seg).to_string()));
            continue;
        }
        if pattern_seg == actual_seg {
            static_segments += 1;
            continue;
        }
        return None;
    }

    Some(WebhookPathMatch {
        params,
        static_segments,
        dynamic_segments,
        total_segments: pattern_segments.len(),
    })
}

fn normalize_webhook_path(raw: &str) -> String {
    let raw = raw.trim();
    if raw.is_empty() || raw == "/" {
        return "/".to_string();
    }
    let mut out = String::from("/");
    out.push_str(raw.trim_matches('/'));
    out
}

fn split_webhook_segments(path: &str) -> Vec<&str> {
    if path == "/" {
        return Vec::new();
    }
    path.trim_start_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
}

fn path_param_name(segment: &str) -> Option<&str> {
    let name = segment.strip_prefix('{')?.strip_suffix('}')?;
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    if name.contains('/') || name.contains('{') || name.contains('}') {
        return None;
    }
    Some(name)
}

fn pipeline_path_matches(base_path: &str, candidate_path: &str, recursive: bool) -> bool {
    let base = crate::platform::model::normalize_virtual_path(base_path);
    let candidate = crate::platform::model::normalize_virtual_path(candidate_path);
    if !recursive {
        return candidate == base;
    }
    if base == "/" {
        return true;
    }
    candidate == base || candidate.starts_with(&(base + "/"))
}

fn pipeline_source_is_locked(source: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(source) else {
        return false;
    };
    value
        .get("metadata")
        .and_then(|metadata| metadata.get("locked"))
        .and_then(Value::as_bool)
        .or_else(|| value.get("locked").and_then(Value::as_bool))
        .unwrap_or(false)
}

fn resolve_pipeline_registry_scope(
    query: &PipelineRegistryQuery,
) -> Result<PipelineRegistryScope, Response> {
    match query
        .scope
        .as_deref()
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("path") => Ok(PipelineRegistryScope::Path),
        Some("project") => Ok(PipelineRegistryScope::Project),
        Some(_) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": {
                    "code": "PLATFORM_PIPELINE_REGISTRY_SCOPE_INVALID",
                    "message": "query.scope must be 'path' or 'project'"
                }
            })),
        )
            .into_response()),
        None => {
            if query.path.is_some() {
                Ok(PipelineRegistryScope::Path)
            } else {
                Ok(PipelineRegistryScope::Project)
            }
        }
    }
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

fn webhook_trigger_from_pipeline_source(source: &str) -> Option<(String, String)> {
    let graph = serde_json::from_str::<PipelineGraph>(source).ok()?;
    let node = graph
        .nodes
        .iter()
        .find(|node| canonical_pipeline_node_kind(&node.kind) == "n.trigger.webhook")?;
    let path = node
        .config
        .get("path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(ToString::to_string)?;
    let method = node
        .config
        .get("method")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|method| !method.is_empty())
        .unwrap_or("POST")
        .to_ascii_uppercase();
    Some((path, method))
}

fn canonical_pipeline_node_kind(kind: &str) -> &str {
    if let Some(stripped) = kind.strip_prefix("x.n.") {
        return match stripped {
            "trigger.webhook" => "n.trigger.webhook",
            "trigger.schedule" => "n.trigger.schedule",
            "trigger.manual" => "n.trigger.manual",
            _ => kind,
        };
    }
    kind
}

fn validate_execute_trigger(
    graph: &PipelineGraph,
    req: &ExecutePipelineRequest,
) -> Result<(), String> {
    match req.trigger {
        PipelineExecuteTrigger::Webhook => {
            let wanted_path = req.webhook_path.as_deref().unwrap_or("/").trim();
            let wanted_method = req
                .webhook_method
                .as_deref()
                .unwrap_or("POST")
                .trim()
                .to_uppercase();
            let matched = graph.nodes.iter().any(|node| {
                if canonical_pipeline_node_kind(&node.kind) != "n.trigger.webhook" {
                    return false;
                }
                let node_path = node
                    .config
                    .get("path")
                    .and_then(Value::as_str)
                    .unwrap_or("/")
                    .trim();
                let node_method = node
                    .config
                    .get("method")
                    .and_then(Value::as_str)
                    .unwrap_or("POST")
                    .trim()
                    .to_uppercase();
                node_path == wanted_path && node_method == wanted_method
            });
            if matched {
                Ok(())
            } else {
                Err(format!(
                    "no webhook trigger matched path='{}' method='{}'",
                    wanted_path, wanted_method
                ))
            }
        }
        PipelineExecuteTrigger::Schedule => {
            let wanted_cron = req.schedule_cron.as_deref().map(str::trim);
            let matched = graph.nodes.iter().any(|node| {
                if canonical_pipeline_node_kind(&node.kind) != "n.trigger.schedule" {
                    return false;
                }
                match wanted_cron {
                    Some(cron) => {
                        node.config
                            .get("cron")
                            .and_then(Value::as_str)
                            .map(str::trim)
                            == Some(cron)
                    }
                    None => true,
                }
            });
            if matched {
                Ok(())
            } else if let Some(cron) = wanted_cron {
                Err(format!("no schedule trigger matched cron='{}'", cron))
            } else {
                Err("pipeline has no schedule trigger".to_string())
            }
        }
        PipelineExecuteTrigger::Manual => {
            let matched = graph
                .nodes
                .iter()
                .any(|node| canonical_pipeline_node_kind(&node.kind) == "n.trigger.manual");
            if matched {
                Ok(())
            } else {
                Err("pipeline has no manual trigger".to_string())
            }
        }
    }
}

fn hydrate_web_render_markup_from_templates(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    graph: &mut PipelineGraph,
) -> Result<(), PlatformError> {
    for node in &mut graph.nodes {
        if node.kind != "n.web.render" {
            continue;
        }
        let has_markup = node
            .config
            .get("markup")
            .and_then(Value::as_str)
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if has_markup {
            continue;
        }

        let template_rel = node
            .config
            .get("template_path")
            .or_else(|| node.config.get("template_rel_path"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let Some(template_rel) = template_rel else {
            continue;
        };

        let markup = state
            .platform
            .projects
            .read_template_file(owner, project, template_rel)?;
        if let Some(map) = node.config.as_object_mut() {
            map.insert("markup".to_string(), Value::String(markup));
        }
    }
    Ok(())
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

async fn api_get_mcp_session(
    State(state): State<PlatformAppState>,
    Path((owner, project)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::McpSessionCreate,
    ) {
        return resp;
    }

    match state
        .platform
        .mcp_sessions
        .get_for_project(&owner, &project)
    {
        Some(session) => Json(json!({
            "ok": true,
            "session": {
                "active": true,
                "capabilities": session.capabilities.iter().map(|c| c.key()).collect::<Vec<_>>(),
            }
        }))
        .into_response(),
        None => Json(json!({
            "ok": true,
            "session": {
                "active": false,
                "capabilities": Vec::<String>::new(),
            }
        }))
        .into_response(),
    }
}

async fn api_create_mcp_session(
    State(state): State<PlatformAppState>,
    Path((owner, project)): Path<(String, String)>,
    headers: HeaderMap,
    Json(req): Json<McpSessionCreateRequest>,
) -> Response {
    if let Err(resp) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::McpSessionCreate,
    ) {
        return resp;
    }

    let capabilities: Vec<ProjectCapability> = req
        .capabilities
        .iter()
        .filter_map(|key| ProjectCapability::from_key(key))
        .collect();

    if capabilities.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": {"code": "INVALID_REQUEST", "message": "At least one valid capability must be specified"}
            })),
        )
            .into_response();
    }

    let base_url = std::env::var("ZEBFLOW_PLATFORM_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:10610".to_string());

    match state
        .platform
        .mcp_sessions
        .create(&owner, &project, capabilities, &base_url, req.auto_reset_seconds)
    {
        Ok(response) => Json(json!({"ok": true, "session": response})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_revoke_mcp_session(
    State(state): State<PlatformAppState>,
    Path((owner, project)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::McpSessionRevoke,
    ) {
        return resp;
    }

    match state
        .platform
        .mcp_sessions
        .revoke_for_project(&owner, &project)
    {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(err) => internal_error(err),
    }
}

fn internal_error(err: PlatformError) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
    )
        .into_response()
}

//! The project's architecture as the runtime holds it: the active pipelines
//! as they run, and for each page one of them answers with, the files that
//! page's compile read. Read from the pipeline runtime and the page cache,
//! never from the repository, so it shows what is live.
//!
//! | Route | Guard | Answers |
//! |---|---|---|
//! | `GET /api/projects/{o}/{p}/architecture` | `pipelines.read` | `{ pipelines, pages }` |
//!
//! A page lists its files once it is compiled — by its first request, or by
//! a save of a file it uses. Until then it answers `compiled: false`.

use super::*;
use std::collections::BTreeSet;

/// `GET …/architecture`: what this project runs, as the runtime holds it.
pub(super) async fn api_architecture(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(&state, &headers, &owner, &project, ProjectCapability::PipelinesRead) {
        return response;
    }
    let (owner, project) = (slug_segment(&owner), slug_segment(&project));
    let repo = match state.platform.projects.resolve_repo_root_abs(&owner, &project) {
        Ok(repo) => repo,
        Err(err) => return internal_error(err),
    };
    let mut pages = BTreeSet::new();
    let pipelines: Vec<Value> = state
        .platform
        .pipeline_runtime
        .list_project(&owner, &project)
        .iter()
        .map(|pipeline| {
            let mut linked = BTreeSet::new();
            let mut credentials = BTreeSet::new();
            let nodes: Vec<Value> = pipeline
                .graph
                .nodes
                .iter()
                .map(|node| {
                    let mut config = node.config.clone();
                    if let Some(map) = config.as_object_mut() {
                        map.remove("ui");
                    }
                    if node.kind == "web.response.send"
                        && let Some(template) = node.config.get("template").and_then(Value::as_str)
                    {
                        linked.insert(template.trim_start_matches('/').to_string());
                    }
                    if let Some(credential) = node.config.get("credential_id").and_then(Value::as_str) {
                        credentials.insert(credential.to_string());
                    }
                    json!({ "id": node.id, "kind": node.kind, "config": config })
                })
                .collect();
            pages.extend(linked.iter().cloned());
            json!({
                "file": pipeline.file_rel_path,
                "active_hash": pipeline.active_hash,
                "routes": pipeline.webhook_triggers.iter().map(|t| json!({ "method": t.method, "path": t.path })).collect::<Vec<_>>(),
                "rooms": pipeline.ws_triggers.iter().map(|t| t.room.clone()).collect::<Vec<_>>(),
                "nodes": nodes,
                "edges": pipeline.graph.edges.iter().map(|e| json!({ "from": e.from_node, "to": e.to_node })).collect::<Vec<_>>(),
                "pages": linked,
                "credentials": credentials,
            })
        })
        .collect();
    let pages: Vec<Value> = pages.into_iter().map(|page| page_view(&state, &repo, &page)).collect();
    Json(json!({ "pipelines": pipelines, "pages": pages })).into_response()
}

/// One page and the repository files its compile read, itself excluded;
/// files from outside the repository (installed libraries) are counted.
fn page_view(state: &PlatformAppState, repo: &FsPath, page: &str) -> Value {
    let entry = fs::canonicalize(repo.join(page)).unwrap_or_else(|_| repo.join(page));
    let Some(read) = crate::pipeline::engines::basic::compiled_page_files(&state.template_cache, &entry.to_string_lossy()) else {
        return json!({ "page": page, "compiled": false, "files": [], "outside_repo": 0 });
    };
    let mut files = BTreeSet::new();
    let mut outside = 0;
    for path in read {
        match FsPath::new(&path).strip_prefix(repo) {
            Ok(rel) if FsPath::new(&path) != entry => {
                files.insert(rel.to_string_lossy().into_owned());
            }
            Ok(_) => {}
            Err(_) => outside += 1,
        }
    }
    json!({ "page": page, "compiled": true, "files": files, "outside_repo": outside })
}

//! `web.site.generate` through real pipelines, read back where a visitor
//! reads it: the folder's `serve` origin (`kinds/zebfs-acl`, `public_execute`)
//! and the project's file host (`addressing.md` §2b, inert).
//!
//! Page mode (`--template … --path …`): a page is written, served, shares one
//! `_assets/` with its siblings, is `unchanged` on a second identical run, and
//! `--on-conflict skip|error` leave a changed page as it was. Docs mode
//! (`--from`): Markdown under `docs/<folder>/` becomes pages in `_meta.yaml`
//! and front-matter order, a search index, and — once the folder has a serve
//! origin — a sitemap and canonical links; the template is scaffolded when
//! missing. The manifest and every dot path answer 404 on both hosts
//! (`/.well-known/` is covered by `published_mcp::a_dot_path_is_never_served_except_well_known`).
//!
//! Every resource goes through its API: templates and Markdown through the
//! repository API, pipelines through the DSL endpoint, exposure through the
//! Files access API. Nothing is written into the data root directly.

use std::fs;

use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use zebflow::pipeline::nodes::basic::web::site;
use zebflow::platform::{PlatformConfig, build_router};

const OWNER: &str = "superadmin";
const PROJECT: &str = "default";
/// The project's automatic dev host, used as a folder's `serve` origin.
const SITE_HOST: &str = "default.superadmin.localhost";
const SITE_ORIGIN: &str = "http://default.superadmin.localhost";
/// The project's automatic file host.
const FILE_HOST: &str = "default.superadmin.fs.localhost";

struct TestDir(std::path::PathBuf);

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn temp_dir(name: &str) -> TestDir {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
    TestDir(std::env::temp_dir().join(format!("zebflow-platform-{name}-{now}")))
}

async fn app_at(root: &TestDir) -> axum::Router {
    let mut config = PlatformConfig::default();
    config.data_root = root.0.clone();
    config.default_password = "test-pass".to_string();
    build_router(config).await.expect("platform router")
}

async fn login(app: &axum::Router) -> String {
    let form = serde_urlencoded::to_string([("identifier", "superadmin"), ("password", "test-pass")]).expect("form");
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/login")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(form))
                .expect("request"),
        )
        .await
        .expect("login");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    response.headers().get(header::SET_COOKIE).expect("cookie").to_str().expect("text").split(';').next().unwrap().to_string()
}

async fn text_of(response: axum::response::Response) -> (StatusCode, HeaderMap, String) {
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body");
    (status, headers, String::from_utf8_lossy(&bytes).to_string())
}

fn parse(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| Value::String(text.to_string()))
}

async fn send(app: &axum::Router, cookie: &str, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
    let request = Request::builder()
        .uri(uri)
        .method(method)
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request");
    let (status, _, text) = text_of(app.clone().oneshot(request).await.expect("response")).await;
    (status, parse(&text))
}

fn api(path: &str) -> String {
    format!("/api/projects/{OWNER}/{PROJECT}{path}")
}

/// Writes one repository file through the repository API.
async fn put_repo_file(app: &axum::Router, cookie: &str, path: &str, content: &str) {
    let request = Request::builder()
        .uri(api(&format!("/repo/file?path={path}")))
        .method("PUT")
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from(content.to_string()))
        .expect("request");
    let (status, _, text) = text_of(app.clone().oneshot(request).await.expect("response")).await;
    assert_eq!(status, StatusCode::OK, "PUT {path}: {text}");
}

async fn repo_file_status(app: &axum::Router, cookie: &str, path: &str) -> (StatusCode, String) {
    let request = Request::builder()
        .uri(api(&format!("/repo/file?path={path}")))
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .expect("request");
    let (status, _, text) = text_of(app.clone().oneshot(request).await.expect("response")).await;
    (status, text)
}

/// A stored object's bytes as the Studio reads them (signed in).
async fn store_object(app: &axum::Router, cookie: &str, key: &str) -> (StatusCode, String) {
    let request = Request::builder()
        .uri(api(&format!("/files/object?ref={key}")))
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .expect("request");
    let (status, _, text) = text_of(app.clone().oneshot(request).await.expect("response")).await;
    (status, text)
}

/// A visitor's GET on `host`: no cookie.
async fn visit(app: &axum::Router, host: &str, uri: &str) -> (StatusCode, HeaderMap, String) {
    let request = Request::builder().uri(uri).header(header::HOST, host).body(Body::empty()).expect("request");
    text_of(app.clone().oneshot(request).await.expect("response")).await
}

async fn dsl(app: &axum::Router, cookie: &str, line: &str) -> Value {
    send(app, cookie, "POST", &api("/pipelines/dsl"), json!({ "dsl": line })).await.1
}

async fn publish(app: &axum::Router, cookie: &str, name: &str, body: &str) {
    for line in [format!("register pipelines/tests/{name} -- {body}"), format!("activate pipeline pipelines/tests/{name}.zf.json")] {
        let answer = dsl(app, cookie, &line).await;
        assert_eq!(answer["ok"], json!(true), "{line}: {answer}");
    }
}

async fn execute(app: &axum::Router, cookie: &str, name: &str, input: Value) -> (StatusCode, Value) {
    send(
        app,
        cookie,
        "POST",
        &api("/pipelines/execute"),
        json!({ "file_rel_path": format!("pipelines/tests/{name}.zf.json"), "trigger": "manual", "input": input }),
    )
    .await
}

/// Runs and answers `output.site`, panicking on a failed run.
async fn generate(app: &axum::Router, cookie: &str, name: &str, input: Value) -> Value {
    let (status, body) = execute(app, cookie, name, input).await;
    assert_eq!(status, StatusCode::OK, "{name}: {body}");
    body["output"]["site"].clone()
}

/// Serves `folder` as a site on the dev host (`public_execute`).
async fn serve_folder(app: &axum::Router, cookie: &str, folder: &str) {
    let (status, body) = send(
        app,
        cookie,
        "PUT",
        &api("/files/access"),
        json!({ "path": folder, "access": "public_execute", "scope": "prefix", "serve": [SITE_ORIGIN] }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

const ALBUM_TEMPLATE: &str = r#"export const page = { head: { title: "Demo album" } };

export default function AlbumPage(input) {
  return (
    <Page>
      <main>
        <img src="/assets/branding/logo.svg" alt="logo" />
        <h1>{input.manual.title}</h1>
        <p>{input.manual.note}</p>
      </main>
    </Page>
  );
}
"#;

async fn album_site(app: &axum::Router, cookie: &str) {
    put_repo_file(app, cookie, "pages/album.tsx", ALBUM_TEMPLATE).await;
    publish(app, cookie, "site-page", r#"| trigger.manual | web.site.generate --template pages/album.tsx --path "{{ input.manual.path }}""#).await;
}

/// A page is written under `site/`, answers `site` and keeps the payload,
/// opens on its folder's serve origin with its shared `_assets/`, is inert
/// on the file host, and an identical second run rewrites nothing.
#[tokio::test]
async fn a_generated_page_is_served_where_its_folder_is_served() {
    let root = temp_dir("site-generate-page");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    album_site(&app, &cookie).await;

    let home_input = json!({ "path": "index.html", "title": "Demo Records", "note": "Welcome to site-a." });
    let (status, body) = execute(&app, &cookie, "site-page", home_input.clone()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let mut keys: Vec<&String> = body["output"].as_object().expect("output").keys().collect();
    keys.sort();
    // The run's value keeps the manual trigger's envelope as `manual`.
    assert_eq!(keys, vec!["manual", "site"], "one key added, the rest kept: {body}");
    let home = &body["output"]["site"];
    assert_eq!(home["mode"], json!("page"), "{home}");
    assert_eq!(home["status"], json!("written"), "{home}");
    assert_eq!(home["path"], json!("site/index.html"), "{home}");
    assert_eq!(home["route"], json!("/"), "{home}");
    assert_eq!(home["folder"], json!("site"), "{home}");
    assert_eq!(home["template"], json!("pages/album.tsx"), "{home}");
    assert_eq!(home["manifest_path"], json!("site/.zebflow-static-site.json"), "{home}");
    assert_eq!(home["file"]["__zf_type"], json!("file_ref"), "{home}");
    assert_eq!(home["file"]["ref"], json!("site/index.html"), "{home}");
    assert!(home["store"].is_string(), "the store is named: {home}");
    assert_eq!(home["origin"], json!(null), "no serve rule yet, so no origin: {home}");

    let album = generate(&app, &cookie, "site-page", json!({ "path": "albums/demo-album.html", "title": "Demo Album One", "note": "Ten demo tracks." })).await;
    assert_eq!(album["status"], json!("written"), "{album}");
    assert_eq!(album["route"], json!("/albums/demo-album.html"), "{album}");
    assert_eq!(album["asset_group"], home["asset_group"], "one template, one asset group");

    // Private until served: the site host does not answer the folder yet.
    assert_eq!(visit(&app, SITE_HOST, "/albums/demo-album.html").await.0, StatusCode::NOT_FOUND);

    serve_folder(&app, &cookie, "site").await;
    let (status, _, html) = visit(&app, SITE_HOST, "/").await;
    assert_eq!(status, StatusCode::OK, "{html}");
    assert!(html.contains("Demo Records") && html.contains("Welcome to site-a."), "{html}");
    assert!(!html.contains("RWE component error"), "{html}");
    assert!(html.contains("\"./_assets/branding/logo.svg\""), "a root page links its assets relatively: {html}");

    for uri in ["/albums/demo-album.html", "/albums/demo-album"] {
        let (status, _, page) = visit(&app, SITE_HOST, uri).await;
        assert_eq!(status, StatusCode::OK, "{uri}");
        assert!(page.contains("Demo Album One"), "{uri}: {page}");
        assert!(page.contains("../_assets/branding/logo.svg"), "a nested page reaches the same _assets/: {page}");
    }
    let (status, headers, svg) = visit(&app, SITE_HOST, "/_assets/branding/logo.svg").await;
    assert_eq!(status, StatusCode::OK);
    assert!(svg.contains("<svg"), "{svg}");
    assert!(headers.get(header::CONTENT_TYPE).is_some_and(|v| v.to_str().unwrap_or("").starts_with("image/svg")));

    // The manifest lists the shared asset once, for both pages.
    let (status, manifest) = store_object(&app, &cookie, "site/.zebflow-static-site.json").await;
    assert_eq!(status, StatusCode::OK, "the Studio's signed-in object read answers a dot file: {manifest}");
    let manifest = parse(&manifest);
    let pages: Vec<&str> = manifest["pages"].as_array().expect("pages").iter().filter_map(|p| p["path"].as_str()).collect();
    assert_eq!(pages, vec!["albums/demo-album.html", "index.html"], "{manifest}");
    let logos = manifest["assets"].as_array().expect("assets").iter().filter(|a| a["path"] == json!("_assets/branding/logo.svg")).count();
    assert_eq!(logos, 1, "{manifest}");

    // The manifest and any dot path are never served, encoded or not.
    for uri in ["/.zebflow-static-site.json", "/%2Ezebflow-static-site.json", "/%2ezebflow-static-site.json", "/_assets/.zebflow-static-site.json"] {
        assert_eq!(visit(&app, SITE_HOST, uri).await.0, StatusCode::NOT_FOUND, "{uri}");
    }

    // The file host answers the same object inert: sandboxed, downloaded.
    let (status, headers, _) = visit(&app, FILE_HOST, "/site/index.html").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers.get(header::CONTENT_SECURITY_POLICY).and_then(|v| v.to_str().ok()), Some("sandbox"));
    assert!(headers.get(header::CONTENT_DISPOSITION).and_then(|v| v.to_str().ok()).is_some_and(|v| v.starts_with("attachment")));
    assert_eq!(visit(&app, FILE_HOST, "/site/.zebflow-static-site.json").await.0, StatusCode::NOT_FOUND);

    // A second identical run writes nothing.
    let again = generate(&app, &cookie, "site-page", home_input).await;
    assert_eq!(again["status"], json!("unchanged"), "{again}");
    assert_eq!(again["origin"], json!(SITE_ORIGIN), "the origin comes from the serve rule: {again}");
}

/// A changed page: `skip` leaves it and says so, `error` fails the run with
/// the conflict code and leaves it, identical content is never a conflict,
/// and the default overwrites.
#[tokio::test]
async fn on_conflict_skip_and_error_leave_a_changed_page_as_it_was() {
    let root = temp_dir("site-generate-conflict");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    album_site(&app, &cookie).await;
    for (name, policy) in [("site-page-skip", "skip"), ("site-page-error", "error")] {
        publish(
            &app,
            &cookie,
            name,
            &format!(r#"| trigger.manual | web.site.generate --template pages/album.tsx --path "{{{{ input.manual.path }}}}" --on-conflict {policy}"#),
        )
        .await;
    }
    let first = json!({ "path": "index.html", "title": "Version one", "note": "demo" });
    let second = json!({ "path": "index.html", "title": "Version two", "note": "demo" });
    assert_eq!(generate(&app, &cookie, "site-page", first.clone()).await["status"], json!("written"));

    let skipped = generate(&app, &cookie, "site-page-skip", second.clone()).await;
    assert_eq!(skipped["status"], json!("skipped"), "{skipped}");
    let (_, html) = store_object(&app, &cookie, "site/index.html").await;
    assert!(html.contains("Version one") && !html.contains("Version two"), "skip left the page: {html}");

    let (status, refused) = execute(&app, &cookie, "site-page-error", second.clone()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    // The execute API answers the node's own code, not a wrapper.
    assert_eq!(refused["error"]["code"], json!(site::CODE_CONFLICT), "{refused}");
    let (_, html) = store_object(&app, &cookie, "site/index.html").await;
    assert!(html.contains("Version one"), "error left the page: {html}");

    let same = generate(&app, &cookie, "site-page-error", first).await;
    assert_eq!(same["status"], json!("unchanged"), "identical content is no conflict: {same}");

    let overwritten = generate(&app, &cookie, "site-page", second).await;
    assert_eq!(overwritten["status"], json!("written"), "{overwritten}");
    let (_, html) = store_object(&app, &cookie, "site/index.html").await;
    assert!(html.contains("Version two"), "{html}");

    // Skip and error are decided before anything is written: a template that
    // links an asset the site lacks leaves no asset and no manifest entry.
    put_repo_file(&app, &cookie, "pages/album-v2.tsx", &ALBUM_TEMPLATE.replace("logo.svg", "logo.png")).await;
    for (name, policy) in [("site-v2-skip", "skip"), ("site-v2-error", "error")] {
        publish(
            &app,
            &cookie,
            name,
            &format!(r#"| trigger.manual | web.site.generate --template pages/album-v2.tsx --path "{{{{ input.manual.path }}}}" --on-conflict {policy}"#),
        )
        .await;
    }
    let third = json!({ "path": "index.html", "title": "Version three", "note": "demo" });
    assert_eq!(generate(&app, &cookie, "site-v2-skip", third.clone()).await["status"], json!("skipped"));
    let (status, refused) = execute(&app, &cookie, "site-v2-error", third).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert_eq!(store_object(&app, &cookie, "site/_assets/branding/logo.png").await.0, StatusCode::NOT_FOUND, "no asset written");
    let (_, manifest) = store_object(&app, &cookie, "site/.zebflow-static-site.json").await;
    let templates: Vec<String> = parse(&manifest)["templates"].as_array().expect("templates").iter().filter_map(|t| t["template"].as_str().map(str::to_string)).collect();
    assert_eq!(templates, vec!["pages/album.tsx"], "no manifest entry for a page not written: {manifest}");
}

/// A page generated from a webhook embeds its input for hydration, so the
/// request that triggered it — the caller's address, browser, referrer,
/// cookie and Authorization header — must not be part of that input: the
/// page renders from the payload without the trigger's `headers`, `cookies`
/// and `auth` (`web.site.generate`'s description; `addressing.md` §0).
#[tokio::test]
async fn a_page_generated_from_a_request_does_not_publish_the_request() {
    let root = temp_dir("site-generate-request");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    put_repo_file(
        &app,
        &cookie,
        "pages/note.tsx",
        "export default function Note(input) {\n  return (<Page><main><h1>{input.webhook.body.title}</h1></main></Page>);\n}\n",
    )
    .await;
    publish(
        &app,
        &cookie,
        "site-from-request",
        r#"| trigger.webhook --route /notes/publish --method POST | web.site.generate --template pages/note.tsx --path notes/latest.html | web.response.send --body "{{ { path: input.site.path, status: input.site.status } }}""#,
    )
    .await;
    let bearer = "Bearer demo-request-token-not-for-pages";
    let request = Request::builder()
        .uri(format!("/wh/{OWNER}/{PROJECT}/notes/publish"))
        .method("POST")
        .header(header::COOKIE, &cookie)
        .header(header::AUTHORIZATION, bearer)
        .header("x-forwarded-for", "203.0.113.10")
        .header(header::USER_AGENT, "demo-agent/1.0")
        .header(header::REFERER, "https://example.com/private-referrer")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "title": "A demo note" }).to_string()))
        .expect("request");
    let (status, _, answer) = text_of(app.clone().oneshot(request).await.expect("response")).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(parse(&answer)["path"], json!("site/notes/latest.html"), "{answer}");

    let (status, html) = store_object(&app, &cookie, "site/notes/latest.html").await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("A demo note"), "{html}");
    let session = cookie.split_once('=').map(|(_, value)| value).unwrap_or(&cookie);
    assert!(!html.contains(session), "the generated page carries the requester's session cookie");
    assert!(!html.contains("demo-request-token-not-for-pages"), "the generated page carries the request's Authorization header");
    for (what, value) in [("address", "203.0.113.10"), ("browser", "demo-agent/1.0"), ("referrer", "private-referrer")] {
        assert!(!html.contains(value), "the generated page carries the caller's {what}");
    }
    // The body the author rendered is the page's; it is still there to hydrate.
    assert!(html.contains("\"title\":\"A demo note\""), "{html}");
}

/// A small handbook: a root page, two folders whose `_meta.yaml` order
/// contradicts their alphabetical order, and two pages whose front-matter
/// order does the same.
async fn handbook(app: &axum::Router, cookie: &str) {
    for (path, content) in [
        ("docs/handbook/_meta.yaml", "title: Demo Handbook\n"),
        ("docs/handbook/index.md", "# Welcome\n\nThe demo handbook home.\n"),
        ("docs/handbook/setup/_meta.yaml", "title: Setup\norder: 1\n"),
        ("docs/handbook/setup/install.md", "---\ntitle: Install\norder: 1\n---\n# Install\n\nInstall the demo app.\n"),
        ("docs/handbook/setup/configure.md", "---\ntitle: Configure\norder: 2\n---\n# Configure\n\nConfigure site-a.\n"),
        ("docs/handbook/advanced/_meta.yaml", "title: Advanced\norder: 2\n"),
        ("docs/handbook/advanced/tuning.md", "---\ntitle: Tuning\n---\n# Tuning\n\nTune the demo cache.\n"),
    ] {
        put_repo_file(app, cookie, path, content).await;
    }
    publish(app, cookie, "docs-build", "| trigger.manual | web.site.generate --from handbook --folder handbook-site").await;
}

fn search_hrefs(index: &str) -> Vec<String> {
    parse(index).as_array().expect("search index").iter().filter_map(|e| e["href"].as_str().map(str::to_string)).collect()
}

/// The sidebar is the server-rendered `<nav>` between "Contents" and `<main`.
fn sidebar(html: &str) -> &str {
    let start = html.find("Contents").expect("the scaffold's sidebar heading");
    let end = html[start..].find("<main").map(|i| start + i).unwrap_or(html.len());
    &html[start..end]
}

/// Markdown to a served docs site: pages, the scaffolded template, the
/// sidebar and the search index in `_meta.yaml` / front-matter order, and the
/// sitemap and canonical links once the folder has a serve origin.
#[tokio::test]
async fn a_docs_folder_becomes_a_served_site_in_meta_order() {
    let root = temp_dir("site-generate-docs");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    handbook(&app, &cookie).await;
    assert_eq!(repo_file_status(&app, &cookie, site::DEFAULT_DOCS_TEMPLATE).await.0, StatusCode::NOT_FOUND, "no template before the first build");

    let first = generate(&app, &cookie, "docs-build", json!({})).await;
    assert_eq!(first["mode"], json!("docs"), "{first}");
    assert_eq!(first["status"], json!("ok"), "{first}");
    assert_eq!(first["name"], json!("Demo Handbook"), "the root _meta.yaml names the site: {first}");
    assert_eq!(first["template"], json!(site::DEFAULT_DOCS_TEMPLATE), "{first}");
    assert_eq!(first["from"], json!("handbook"), "{first}");
    assert_eq!(first["folder"], json!("handbook-site"), "{first}");
    assert_eq!(first["page_count"], json!(4), "{first}");
    assert_eq!(first["search_index_path"], json!("handbook-site/search-index.json"), "{first}");
    assert_eq!(first["sitemap_path"], json!(null), "no serve origin, no sitemap: {first}");

    let (status, template) = repo_file_status(&app, &cookie, site::DEFAULT_DOCS_TEMPLATE).await;
    assert_eq!(status, StatusCode::OK, "the scaffold was written");
    assert!(template.contains("DocsTemplate"), "{template}");

    serve_folder(&app, &cookie, "handbook-site").await;
    let second = generate(&app, &cookie, "docs-build", json!({})).await;
    assert_eq!(second["origin"], json!(SITE_ORIGIN), "{second}");
    assert_eq!(second["sitemap_path"], json!("handbook-site/sitemap.xml"), "{second}");
    let third = generate(&app, &cookie, "docs-build", json!({})).await;
    assert_eq!(third["generated_files"], json!(0), "an unchanged build rewrites nothing: {third}");

    let (status, _, home) = visit(&app, SITE_HOST, "/").await;
    assert_eq!(status, StatusCode::OK, "{home}");
    assert!(home.contains("<title>Welcome | Demo Handbook</title>"), "{home}");
    assert!(!home.contains("RWE component error"), "{home}");

    let (status, _, install) = visit(&app, SITE_HOST, "/setup/install/").await;
    assert_eq!(status, StatusCode::OK, "{install}");
    assert!(install.contains("<title>Install | Demo Handbook</title>"), "{install}");
    // zeb/markdown renders on the server, so the body is in the HTML.
    assert!(install.contains("Install the demo app."), "{install}");
    assert!(install.contains("http://default.superadmin.localhost/setup/install/"), "canonical link: {install}");

    // `_meta.yaml` orders the folders, front matter the pages — not titles.
    let nav = sidebar(&install);
    let at = |needle: &str| nav.find(needle).unwrap_or_else(|| panic!("{needle} in the sidebar: {nav}"));
    assert!(at("Setup") < at("Advanced"), "folder order from _meta.yaml: {nav}");
    assert!(at("Install") < at("Configure"), "page order from front matter: {nav}");

    let (status, _, index) = visit(&app, SITE_HOST, "/search-index.json").await;
    assert_eq!(status, StatusCode::OK, "{index}");
    assert_eq!(search_hrefs(&index), vec!["/", "/setup/install/", "/setup/configure/", "/advanced/tuning/"], "reading order: {index}");

    let (status, _, sitemap) = visit(&app, SITE_HOST, "/sitemap.xml").await;
    assert_eq!(status, StatusCode::OK, "{sitemap}");
    for route in ["/", "/setup/install/", "/setup/configure/", "/advanced/tuning/"] {
        assert!(sitemap.contains(&format!("<loc>{SITE_ORIGIN}{route}</loc>")), "{route} in {sitemap}");
    }
    assert_eq!(visit(&app, SITE_HOST, "/.zebflow-static-site.json").await.0, StatusCode::NOT_FOUND);
}

/// A removed or renamed Markdown page leaves the search index, the sitemap
/// and the manifest on the next build.
#[tokio::test]
async fn a_removed_or_renamed_page_leaves_the_index_sitemap_and_manifest() {
    let root = temp_dir("site-generate-docs-removed");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    handbook(&app, &cookie).await;
    serve_folder(&app, &cookie, "handbook-site").await;
    generate(&app, &cookie, "docs-build", json!({})).await;

    let (status, body) = send(&app, &cookie, "DELETE", &api("/repo/file?path=docs/handbook/advanced/tuning.md"), json!(null)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = send(
        &app,
        &cookie,
        "POST",
        &api("/repo/move"),
        json!({ "from_path": "docs/handbook/setup/configure.md", "to_path": "docs/handbook/setup/settings.md" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let rebuilt = generate(&app, &cookie, "docs-build", json!({})).await;
    assert_eq!(rebuilt["page_count"], json!(3), "{rebuilt}");

    let (_, _, index) = visit(&app, SITE_HOST, "/search-index.json").await;
    assert_eq!(search_hrefs(&index), vec!["/", "/setup/install/", "/setup/settings/"], "{index}");
    let (_, _, sitemap) = visit(&app, SITE_HOST, "/sitemap.xml").await;
    assert!(!sitemap.contains("/advanced/tuning/") && !sitemap.contains("/setup/configure/"), "{sitemap}");
    let (status, _, renamed) = visit(&app, SITE_HOST, "/setup/settings/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(renamed.contains("Configure site-a."), "{renamed}");

    let (status, manifest) = store_object(&app, &cookie, "handbook-site/.zebflow-static-site.json").await;
    assert_eq!(status, StatusCode::OK, "{manifest}");
    let pages: Vec<String> = parse(&manifest)["pages"].as_array().expect("pages").iter().filter_map(|p| p["path"].as_str().map(str::to_string)).collect();
    assert_eq!(pages, vec!["index.html", "setup/install/index.html", "setup/settings/index.html"], "{manifest}");
}

/// A page no longer generated is removed: its HTML leaves the store with its
/// manifest record, so the site stops serving it. Also after a template edit,
/// which changes the asset group the build records.
#[tokio::test]
async fn a_removed_page_stops_being_served() {
    let root = temp_dir("site-generate-docs-stale");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    handbook(&app, &cookie).await;
    serve_folder(&app, &cookie, "handbook-site").await;
    generate(&app, &cookie, "docs-build", json!({})).await;
    assert_eq!(visit(&app, SITE_HOST, "/advanced/tuning/").await.0, StatusCode::OK);
    let (status, body) = send(&app, &cookie, "DELETE", &api("/repo/file?path=docs/handbook/advanced/tuning.md"), json!(null)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    generate(&app, &cookie, "docs-build", json!({})).await;
    assert_eq!(visit(&app, SITE_HOST, "/advanced/tuning/").await.0, StatusCode::NOT_FOUND);
    assert_eq!(store_object(&app, &cookie, "handbook-site/advanced/tuning/index.html").await.0, StatusCode::NOT_FOUND);

    // Edit the template, then remove another page: still pruned.
    let (_, template) = repo_file_status(&app, &cookie, site::DEFAULT_DOCS_TEMPLATE).await;
    let edited = parse(&template)["file"]["content"].as_str().expect("template content").replace("Contents", "On this site");
    put_repo_file(&app, &cookie, site::DEFAULT_DOCS_TEMPLATE, &edited).await;
    let (status, body) = send(&app, &cookie, "DELETE", &api("/repo/file?path=docs/handbook/setup/configure.md"), json!(null)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let rebuilt = generate(&app, &cookie, "docs-build", json!({})).await;
    assert_eq!(rebuilt["page_count"], json!(2), "{rebuilt}");
    assert_eq!(visit(&app, SITE_HOST, "/setup/configure/").await.0, StatusCode::NOT_FOUND);
    let (status, _, install) = visit(&app, SITE_HOST, "/setup/install/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(install.contains("On this site"), "the edited template rebuilt the kept pages: {install}");
    let (_, manifest) = store_object(&app, &cookie, "handbook-site/.zebflow-static-site.json").await;
    let manifest = parse(&manifest);
    let pages: Vec<&str> = manifest["pages"].as_array().expect("pages").iter().filter_map(|p| p["path"].as_str()).collect();
    assert_eq!(pages, vec!["index.html", "setup/install/index.html"], "{manifest}");
    let groups: std::collections::BTreeSet<&str> =
        manifest["assets"].as_array().expect("assets").iter().filter_map(|a| a["asset_group"].as_str()).collect();
    assert_eq!(groups.len(), 1, "no asset is left under the template's earlier group: {manifest}");
}

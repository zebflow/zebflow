//! `web.response.send` answering a stored file, through real pipelines on a
//! real platform router: the protected-download pattern — `trigger.webhook
//! --auth jwt` → a `sekejap.query.run` ownership check → `fs.file.get
//! --return file` → `web.response.send --file "{{ input.file }}"` — and the
//! HTTP a file needs: `Range` (206, 416, `If-Range`), `HEAD`, `ETag` /
//! `If-None-Match` → 304, `Last-Modified`, a download name, a 20 MB file
//! streamed, a missing file as 404 with the reason kept on the run, and no
//! key that leaves the project's stores.

use std::fs;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

use zebflow::platform::{PlatformConfig, build_router};

const OWNER: &str = "superadmin";
const PROJECT: &str = "default";
const SIGNING_SECRET: &str = "stored-answer-test-signing-secret-0123456789";

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

async fn send(app: &axum::Router, cookie: &str, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
    let request = Request::builder()
        .uri(uri)
        .method(method)
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request");
    let response = app.clone().oneshot(request).await.expect("response");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body");
    (status, serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).to_string())))
}

fn api(path: &str) -> String {
    format!("/api/projects/{OWNER}/{PROJECT}{path}")
}

async fn publish(app: &axum::Router, cookie: &str, name: &str, body: &str) {
    let register = if body.trim_start().starts_with('[') {
        format!("register pipelines/tests/{name}\n{}", body.trim())
    } else {
        format!("register pipelines/tests/{name} -- {body}")
    };
    for line in [register, format!("activate pipeline pipelines/tests/{name}.zf.json")] {
        let (_, answer) = send(app, cookie, "POST", &api("/pipelines/dsl"), json!({ "dsl": line })).await;
        assert_eq!(answer["ok"], json!(true), "{line}: {answer}");
    }
}

/// One file into the project store through the Files API, at `folder/name`.
async fn upload(app: &axum::Router, cookie: &str, folder: &str, name: &str, bytes: &[u8]) {
    let boundary = "zfstoredanswerboundary";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(api(&format!("/files/upload?path={folder}")))
                .method("POST")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, format!("multipart/form-data; boundary={boundary}"))
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("upload");
    assert_eq!(response.status(), StatusCode::OK, "upload {folder}/{name}");
}

fn token(sub: &str) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
    let claims = json!({ "sub": sub, "iat": now, "exp": now + 600 });
    jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &jsonwebtoken::EncodingKey::from_secret(SIGNING_SECRET.as_bytes()))
        .expect("token")
}

struct Answer {
    status: StatusCode,
    headers: axum::http::HeaderMap,
    body: Vec<u8>,
}

impl Answer {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }
}

/// A visitor's request to a webhook route, with these headers.
async fn visit(app: &axum::Router, method: &str, path: &str, headers: &[(&str, String)]) -> Answer {
    let mut request = Request::builder().uri(format!("/wh/{OWNER}/{PROJECT}{path}")).method(method);
    for (name, value) in headers {
        request = request.header(*name, value);
    }
    let response = app.clone().oneshot(request.body(Body::empty()).expect("request")).await.expect("response");
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body").to_vec();
    Answer { status, headers, body }
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// The fixture: a signing key, a sekejap `documents` table naming each
/// file's owner, two files, and the download route.
async fn documents(app: &axum::Router, cookie: &str, big: &[u8]) {
    let (status, body) = send(
        app,
        cookie,
        "POST",
        &api("/credentials"),
        json!({ "credential_id": "docs-key", "title": "Member tokens", "kind": "jwt_signing_key", "notes": "",
            "secret": { "algorithm": "HS256", "secret": SIGNING_SECRET } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = send(
        app,
        cookie,
        "POST",
        &api("/db/connections"),
        json!({ "connection_slug": "docs-store", "connection_label": "Documents", "database_kind": "sekejap", "config": {} }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let connection = body["connection"]["connection_id"].as_str().expect("connection_id").to_string();
    let (status, body) = send(
        app,
        cookie,
        "POST",
        &api(&format!("/db/connections/{connection}/query")),
        json!({ "sql": "CREATE TABLE documents (_key TEXT PRIMARY KEY, owner TEXT NOT NULL, path TEXT NOT NULL)", "read_only": false }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    upload(app, cookie, "docs", "report-a.pdf", b"%PDF-1.7 report of member a").await;
    upload(app, cookie, "docs", "big.bin", big).await;

    publish(app, cookie, "documents-seed", r#"| trigger.manual | sekejap.record.create --table documents --record "{{ input.manual.items }}""#).await;
    let (status, seeded) = send(
        app,
        cookie,
        "POST",
        &api("/pipelines/execute"),
        json!({ "file_rel_path": "pipelines/tests/documents-seed.zf.json", "trigger": "manual", "input": { "items": [
            { "key": "d1", "fields": { "owner": "member-a", "path": "docs/report-a.pdf" } },
            { "key": "d2", "fields": { "owner": "member-a", "path": "docs/big.bin" } }
        ] } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{seeded}");

    // The owner's pattern. The query is the ownership check: a row only
    // when the signed-in member owns the document.
    publish(
        app,
        cookie,
        "documents-download",
        r#"
[hook] trigger.webhook --route /documents/:id --method GET --auth jwt --credential docs-key
[own] sekejap.query.run --param "1={{ input.webhook.params.id }}" --param "2={{ input.webhook.auth.sub }}" -- "SELECT _key, path FROM documents WHERE _key = $1 AND owner = $2"
[mine] logic.if --when "input.query.rows.length > 0"
[get] fs.file.get --from "{{ input.query.rows[0].path }}" --return file
[send] web.response.send --file "{{ input.file }}"
[deny] web.response.send --status 404 --body "not found"
[hook] -> [own]
[own] -> [mine]
[mine]:true -> [get]
[get] -> [send]
[mine]:false -> [deny]
"#,
    )
    .await;
}

/// The protected-download pattern, end to end: the owner gets the bytes,
/// another member 404, no token 401; Range, HEAD and 304 on the same route;
/// a 20 MB file arrives whole and as streamed ranges.
#[tokio::test]
async fn a_members_file_is_answered_to_its_owner_only_with_range_head_and_304() {
    let root = temp_dir("stored-answer-owner");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    let big: Vec<u8> = (0..20 * 1024 * 1024u32).map(|i| (i.wrapping_mul(2654435761) >> 13) as u8).collect();
    documents(&app, &cookie, &big).await;
    let owner = [("authorization", format!("Bearer {}", token("member-a")))];
    let other = [("authorization", format!("Bearer {}", token("member-b")))];

    let mine = visit(&app, "GET", "/documents/d1", &owner).await;
    assert_eq!(mine.status, StatusCode::OK, "{}", String::from_utf8_lossy(&mine.body));
    assert_eq!(sha256(&mine.body), sha256(b"%PDF-1.7 report of member a"));
    assert_eq!(mine.header("content-type"), Some("application/pdf"));
    assert_eq!(mine.header("content-disposition"), Some("inline; filename=\"report-a.pdf\""));
    assert_eq!(mine.header("accept-ranges"), Some("bytes"));
    assert_eq!(mine.header("cache-control"), Some("private, no-cache"));
    assert_eq!(mine.header("x-content-type-options"), Some("nosniff"));
    let etag = mine.header("etag").expect("an ETag").to_string();
    assert!(etag.starts_with('"') && etag.len() == 66, "the FileRef's digest, strong: {etag}");
    let last_modified = mine.header("last-modified").expect("Last-Modified").to_string();

    let theirs = visit(&app, "GET", "/documents/d1", &other).await;
    assert_eq!(theirs.status, StatusCode::NOT_FOUND, "another member learns nothing");

    // The short form `help("pipeline/web")` teaches: the key straight from the
    // ownership query; no row is `null`, and null is 404, never the payload.
    publish(&app, &cookie, "documents-short", r#"| trigger.webhook --route /mine/:id --method GET --auth jwt --credential docs-key | sekejap.query.run --param "1={{ input.webhook.params.id }}" --param "2={{ input.webhook.auth.sub }}" -- "SELECT path FROM documents WHERE _key = $1 AND owner = $2" | web.response.send --path "{{ input.query.rows[0]?.path ?? null }}""#).await;
    let short = visit(&app, "GET", "/mine/d1", &owner).await;
    assert_eq!((short.status, sha256(&short.body)), (StatusCode::OK, sha256(b"%PDF-1.7 report of member a")));
    let short = visit(&app, "GET", "/mine/d1", &[other[0].clone(), ("accept".into(), "application/json".to_string())]).await;
    assert_eq!(short.status, StatusCode::NOT_FOUND, "{}", String::from_utf8_lossy(&short.body));
    assert!(!String::from_utf8_lossy(&short.body).contains("docs/"), "no row, no payload: {}", String::from_utf8_lossy(&short.body));
    assert!(!String::from_utf8_lossy(&theirs.body).contains("member a"));
    assert_eq!(visit(&app, "GET", "/documents/d1", &[]).await.status, StatusCode::UNAUTHORIZED);

    // HEAD: the GET's headers, no body, and the object never opened.
    let head = visit(&app, "HEAD", "/documents/d1", &owner).await;
    assert_eq!(head.status, StatusCode::OK);
    assert!(head.body.is_empty());
    assert_eq!(head.header("content-length"), Some("27"));
    assert_eq!(head.header("etag"), Some(etag.as_str()));

    // Conditional: the same validator is a 304 with no body.
    let again = visit(&app, "GET", "/documents/d1", &[owner[0].clone(), ("if-none-match", etag.clone())]).await;
    assert_eq!(again.status, StatusCode::NOT_MODIFIED);
    assert!(again.body.is_empty());
    assert_eq!(again.header("etag"), Some(etag.as_str()));
    let since = visit(&app, "GET", "/documents/d1", &[owner[0].clone(), ("if-modified-since", last_modified.clone())]).await;
    assert_eq!(since.status, StatusCode::NOT_MODIFIED);
    let changed = visit(&app, "GET", "/documents/d1", &[owner[0].clone(), ("if-none-match", "\"another\"".to_string())]).await;
    assert_eq!(changed.status, StatusCode::OK);

    // Range: one range is a 206; past the end is a 416; a stale If-Range is the whole file.
    let part = visit(&app, "GET", "/documents/d1", &[owner[0].clone(), ("range", "bytes=0-7".to_string())]).await;
    assert_eq!(part.status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(part.body, b"%PDF-1.7");
    assert_eq!(part.header("content-range"), Some("bytes 0-7/27"));
    let tail = visit(&app, "GET", "/documents/d1", &[owner[0].clone(), ("range", "bytes=-8".to_string())]).await;
    assert_eq!(tail.body, b"member a");
    let past = visit(&app, "GET", "/documents/d1", &[owner[0].clone(), ("range", "bytes=500-".to_string())]).await;
    assert_eq!(past.status, StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(past.header("content-range"), Some("bytes */27"));
    let stale = visit(&app, "GET", "/documents/d1", &[owner[0].clone(), ("range", "bytes=0-7".to_string()), ("if-range", "\"old\"".to_string())]).await;
    assert_eq!((stale.status, stale.body.len()), (StatusCode::OK, 27));
    let fresh = visit(&app, "GET", "/documents/d1", &[owner[0].clone(), ("range", "bytes=0-7".to_string()), ("if-range", etag.clone())]).await;
    assert_eq!(fresh.status, StatusCode::PARTIAL_CONTENT);

    // 20 MB: whole, and a range from its middle, byte for byte.
    let whole = visit(&app, "GET", "/documents/d2", &owner).await;
    assert_eq!(whole.status, StatusCode::OK);
    assert_eq!(whole.header("content-length"), Some(big.len().to_string().as_str()));
    assert_eq!(sha256(&whole.body), sha256(&big), "the 20 MB file arrives whole");
    let middle = visit(&app, "GET", "/documents/d2", &[owner[0].clone(), ("range", "bytes=10485760-11534335".to_string())]).await;
    assert_eq!(middle.status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(sha256(&middle.body), sha256(&big[10_485_760..11_534_336]));

}

/// A key from the request never leaves the project's stores: `..`, an
/// absolute path, the store's own metadata and a missing file are all 404,
/// and a `--file` that resolved to nothing is a 404 too, never the payload.
#[tokio::test]
async fn a_stored_answer_never_leaves_the_stores_and_nothing_is_a_404() {
    let root = temp_dir("stored-answer-containment");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    upload(&app, &cookie, "public", "note.txt", b"a public note").await;
    fs::write(root.0.join("outside.txt"), b"secret outside the stores").expect("outside file");
    publish(&app, &cookie, "raw", r#"| trigger.webhook --route /raw --method GET | web.response.send --path "{{ input.webhook.query.key }}" --filename "{{ input.webhook.query.name ?? 'download.txt' }}""#).await;
    publish(&app, &cookie, "nothing", r#"| trigger.webhook --route /nothing --method GET | web.response.send --file "{{ input.webhook.query.missing ?? null }}""#).await;

    let note = visit(&app, "GET", "/raw?key=public/note.txt&name=n%C3%A9.txt", &[]).await;
    assert_eq!(note.status, StatusCode::OK);
    assert_eq!(note.body, b"a public note");
    assert_eq!(note.header("content-disposition"), Some("attachment; filename=\"n_.txt\"; filename*=UTF-8''n%C3%A9.txt"));
    assert!(note.header("etag").is_some_and(|tag| tag.starts_with("W/\"")), "a bare key's validator is weak");

    for key in [
        "../../outside.txt",
        "public/../../outside.txt",
        "%2Fetc%2Fhosts",
        "..%2F..%2Foutside.txt",
        ".zebfs/acl.json",
        "public/none.txt",
        "public",
    ] {
        let answer = visit(&app, "GET", &format!("/raw?key={key}"), &[("accept", "application/json".to_string())]).await;
        assert_eq!(answer.status, StatusCode::NOT_FOUND, "{key}: {}", String::from_utf8_lossy(&answer.body));
        assert!(!String::from_utf8_lossy(&answer.body).contains("secret outside"), "{key}");
    }
    // A FileRef whose object is gone by the time the answer runs: 404, and
    // the run keeps why.
    upload(&app, &cookie, "public", "gone.txt", b"soon deleted").await;
    publish(&app, &cookie, "gone", r#"| trigger.webhook --route /gone --method GET | fs.file.get --from public/gone.txt --return file | fs.file.delete --from public/gone.txt | web.response.send --file "{{ $nodes.n1.file }}""#).await;
    let gone = visit(&app, "GET", "/gone", &[]).await;
    assert_eq!(gone.status, StatusCode::NOT_FOUND, "{}", String::from_utf8_lossy(&gone.body));
    let (_, runs) = send(&app, &cookie, "GET", &api("/pipelines/invocations?pipeline=pipelines/tests/gone.zf.json"), json!({})).await;
    let runs_text = runs.to_string();
    assert!(
        runs_text.contains("FW_NODE_WEB_RESPONSE_SEND_NOT_FOUND") || runs_text.contains("public/gone.txt' is not in store"),
        "the run record keeps the reason: {runs_text}"
    );

    let nothing = visit(&app, "GET", "/nothing", &[("accept", "application/json".to_string())]).await;
    assert_eq!(nothing.status, StatusCode::NOT_FOUND, "{}", String::from_utf8_lossy(&nothing.body));
    assert!(!String::from_utf8_lossy(&nothing.body).contains("webhook"), "the payload is never the answer");
}

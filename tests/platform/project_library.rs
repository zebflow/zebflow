//! `rwe-library-manifest/README.md` § Project libraries: a maintainer uploads
//! one `.tar.gz`, Zebflow scans and checks every file, installs it for this
//! project only, a page imports `site-a-lib/world`, and every file is checked
//! again before it is served.

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

use super::published_mcp::{OWNER, PROJECT, app_at, login, publish, temp_dir};

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

/// A minimal ustar `.tar.gz` of plain files.
fn tar_gz(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (path, bytes) in files {
        let mut header = [0u8; 512];
        header[..path.len()].copy_from_slice(path.as_bytes());
        header[100..107].copy_from_slice(b"0000644");
        header[124..135].copy_from_slice(format!("{:011o}", bytes.len()).as_bytes());
        header[136..147].copy_from_slice(b"00000000000");
        header[156] = b'0';
        header[257..262].copy_from_slice(b"ustar");
        header[148..156].copy_from_slice(b"        ");
        let sum: u32 = header.iter().map(|b| *b as u32).sum();
        header[148..155].copy_from_slice(format!("{sum:06o}\0").as_bytes());
        out.extend_from_slice(&header);
        out.extend_from_slice(bytes);
        out.resize(out.len().div_ceil(512) * 512, 0);
    }
    out.resize(out.len() + 1024, 0);
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    std::io::Write::write_all(&mut encoder, &out).unwrap();
    encoder.finish().unwrap()
}

/// A library as zeb-bundler writes it: `world` and an original-path entry,
/// a shared chunk, a worker and a data file. `world_code` lets a test plant
/// what the scan must refuse.
fn library(name: &str, version: &str, world_code: &str) -> Vec<u8> {
    let v = version;
    let files: Vec<(String, Vec<u8>)> = vec![
        (format!("{v}/runtime/chunk-1.mjs"), b"export const shared = 41;\n".to_vec()),
        (format!("{v}/runtime/world.mjs"), world_code.as_bytes().to_vec()),
        (format!("{v}/runtime/three/addons/Orbit.mjs"), b"import{shared}from\"../../chunk-1.mjs\";export class Orbit{constructor(){this.n=shared}}\n".to_vec()),
        (format!("{v}/runtime/geometry.worker.mjs"), b"self.onmessage=(e)=>self.postMessage(e.data*2);\n".to_vec()),
        (format!("{v}/data/terrain.bin"), vec![1, 2, 3, 4]),
        (format!("{v}/types/world.d.ts"), b"export declare function makeWorld(): number;\n".to_vec()),
    ];
    let record = |path: &str| {
        let bytes = &files.iter().find(|(p, _)| p == path).unwrap().1;
        json!({ "size_bytes": bytes.len(), "integrity": digest(bytes) })
    };
    let world = format!("{v}/runtime/world.mjs");
    let manifest = json!({
        "apiVersion": "zebflow.com/v1",
        "kind": "RweLibraryManifest",
        "metadata": { "name": name },
        "spec": {
            "name": name,
            "description": "test library",
            "exports": ["Orbit", "makeWorld"],
            "versions": { v: {
                "entry": world,
                "size_bytes": record(&world)["size_bytes"],
                "integrity": record(&world)["integrity"],
                "source": "offline",
                "package_version": v,
                "entries": {
                    "world": { "file": world, "exports": ["makeWorld"], "types": format!("{v}/types/world.d.ts") },
                    "three/addons/Orbit.js": { "file": format!("{v}/runtime/three/addons/Orbit.mjs"), "exports": ["Orbit"] }
                },
                "workers": { "geometry": format!("{v}/runtime/geometry.worker.mjs") },
                "files": files.iter().map(|(p, _)| (p.clone(), record(p))).collect::<serde_json::Map<String, Value>>()
            } },
            "license": { "spdx": "MIT", "holders": ["Site A"], "notices": ["THIRD-PARTY-NOTICES.txt"] }
        }
    });
    let mut all: Vec<(&str, Vec<u8>)> = files.iter().map(|(p, b)| (p.as_str(), b.clone())).collect();
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
    all.push(("manifest.json", manifest_bytes));
    all.push(("THIRD-PARTY-NOTICES.txt", b"MIT\n".to_vec()));
    tar_gz(&all)
}

const WORLD: &str = "import{shared}from\"./chunk-1.mjs\";export const makeWorld=()=>shared+1;export const workerUrl=new URL(\"./geometry.worker.mjs\",import.meta.url).href;\n";

async fn raw(app: &axum::Router, cookie: &str, method: &str, uri: &str, content_type: &str, body: Vec<u8>) -> (StatusCode, String, Vec<u8>) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .method(method)
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, content_type)
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    let kind = response.headers().get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or_default().to_string();
    (status, kind, to_bytes(response.into_body(), usize::MAX).await.expect("body").to_vec())
}

async fn upload(app: &axum::Router, cookie: &str, archive: Vec<u8>) -> (StatusCode, Value) {
    let (status, _, body) = raw(app, cookie, "POST", &format!("/api/projects/{OWNER}/{PROJECT}/rwe/libraries/upload"), "application/gzip", archive).await;
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

#[tokio::test]
async fn an_uploaded_library_is_checked_installed_imported_and_served() {
    let root = temp_dir("project-library");
    let app = app_at(&root).await;
    let cookie = login(&app).await;

    let (status, answer) = upload(&app, &cookie, library("site-a-lib", "0.1.0", WORLD)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["library"]["name"], "site-a-lib");
    assert_eq!(answer["library"]["entries"], json!(["three/addons/Orbit.js", "world"]));

    // The same upload again changes nothing; it is not an error.
    let (status, answer) = upload(&app, &cookie, library("site-a-lib", "0.1.0", WORLD)).await;
    assert_eq!((status, answer["library"]["changed"].clone()), (StatusCode::OK, json!(false)), "{answer}");

    // Listed with what pages may import, and locked as an upload.
    let (_, _, listed) = raw(&app, &cookie, "GET", &format!("/api/projects/{OWNER}/{PROJECT}/rwe/libraries"), "application/json", Vec::new()).await;
    let listed: Value = serde_json::from_slice(&listed).unwrap();
    let mine = listed.as_array().unwrap().iter().find(|item| item["name"] == "site-a-lib").cloned().expect("listed");
    assert_eq!(mine["imports"], json!(["site-a-lib/three/addons/Orbit.js", "site-a-lib/world"]));
    let lock = std::fs::read_to_string(root.0.join("users/superadmin/default/repo/zeb.lock")).expect("lock");
    assert!(lock.contains("site-a-lib") && lock.contains("direct.file"), "{lock}");

    // A page imports it like a zeb/* library.
    let page = "import { makeWorld } from \"site-a-lib/world\";\nimport { Orbit } from \"site-a-lib/three/addons/Orbit.js\";\nexport default function Page() {\n  return <main>lib page</main>;\n}\n";
    let (status, _, _) = raw(&app, &cookie, "PUT", &format!("/api/projects/{OWNER}/{PROJECT}/repo/file?path=pages/lib.tsx"), "text/plain", page.as_bytes().to_vec()).await;
    assert_eq!(status, StatusCode::OK);
    publish(&app, &cookie, "lib-page", "| trigger.webhook --route /lib-page --method GET | web.response.send --template pages/lib.tsx").await;
    let (status, _, html) = raw(&app, &cookie, "GET", &format!("/wh/{OWNER}/{PROJECT}/lib-page"), "text/html", Vec::new()).await;
    let html = String::from_utf8_lossy(&html).to_string();
    assert_eq!(status, StatusCode::OK, "{html}");
    assert!(!html.contains("RWE component error"), "{html}");
    assert!(html.contains("lib page"), "{html}");
    // The page's script loads each entry it imports, from the project's own path.
    let script_src = html.split("data-rwe-external=\"page\" src=\"").nth(1).and_then(|rest| rest.split('"').next()).expect("page script");
    let (status, _, script) = raw(&app, &cookie, "GET", script_src, "", Vec::new()).await;
    let script = String::from_utf8_lossy(&script).to_string();
    assert_eq!(status, StatusCode::OK);
    for key in ["site-a-lib/world", "site-a-lib/three/addons/Orbit.js"] {
        assert!(script.contains(&format!("__zebLibs[\"{key}\"] = await import(")), "{key} is loaded: {script}");
    }
    assert!(script.contains("_rwe/lib/site-a-lib/0.1.0/runtime/world.mjs"), "from the project's own path: {script}");

    // Every listed file is served from the project's path, checked; nothing else is.
    let base = format!("/static/{OWNER}/{PROJECT}/_rwe/lib/site-a-lib/0.1.0");
    for (file, kind) in [("runtime/world.mjs", "javascript"), ("runtime/chunk-1.mjs", "javascript"), ("runtime/geometry.worker.mjs", "javascript"), ("data/terrain.bin", "")] {
        let (status, content_type, _) = raw(&app, &cookie, "GET", &format!("{base}/{file}"), "", Vec::new()).await;
        assert_eq!(status, StatusCode::OK, "{file}");
        assert!(content_type.contains(kind), "{file}: {content_type}");
    }
    let (status, _, _) = raw(&app, &cookie, "GET", &format!("/static/{OWNER}/{PROJECT}/_rwe/lib/site-a-lib/manifest.json"), "", Vec::new()).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "the manifest is not a library file");

    // A file changed on disk after the upload is refused, not served.
    let installed = root.0.join("users/superadmin/default/data/hub/rwe-libraries/project.site-a-lib/0.1.0/runtime/chunk-1.mjs");
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&installed, b"export const shared = 666;\n").expect("tamper");
    let (status, _, _) = raw(&app, &cookie, "GET", &format!("{base}/runtime/chunk-1.mjs"), "", Vec::new()).await;
    assert_eq!(status, StatusCode::CONFLICT);

    // A higher version replaces it; anything else is refused whole.
    let (status, answer) = upload(&app, &cookie, library("site-a-lib", "0.2.0", WORLD)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    for (archive, word) in [
        (library("site-a-lib", "0.1.0", WORLD), "only with a higher version"),
        (library("site-a-lib", "0.3.0", "export const makeWorld = () => eval(\"1\");\n"), "security scan"),
        (library("site-a-lib", "0.3.0", "export const makeWorld = () => import(\"https://cdn.example.com/x.mjs\");\n"), "another site"),
        (library("zeb-lib", "0.1.0", WORLD), "not a project library name"),
        (tar_gz(&[("manifest.json", b"{}".to_vec())]), "not a valid library manifest"),
        (b"not an archive".to_vec(), "not a .tar.gz"),
    ] {
        let (status, answer) = upload(&app, &cookie, archive).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{word}: {answer}");
        assert!(answer["error"]["message"].as_str().unwrap_or_default().contains(word), "{word}: {answer}");
    }

    // A subpath the library does not have is refused by name when a page compiles.
    let bad = "import { nope } from \"site-a-lib/nope\";\nexport default function Page() { return <main>x</main>; }\n";
    let (status, _, _) = raw(&app, &cookie, "PUT", &format!("/api/projects/{OWNER}/{PROJECT}/repo/file?path=pages/bad.tsx"), "text/plain", bad.as_bytes().to_vec()).await;
    assert_eq!(status, StatusCode::OK);
    publish(&app, &cookie, "bad-page", "| trigger.webhook --route /bad-page --method GET --errors show | web.response.send --template pages/bad.tsx").await;
    let (_, _, body) = raw(&app, &cookie, "GET", &format!("/wh/{OWNER}/{PROJECT}/bad-page"), "text/html", Vec::new()).await;
    let body = String::from_utf8_lossy(&body);
    assert!(body.contains("not an entry of the project library site-a-lib"), "{body}");
}

/// A library recipe lives whole in the repository: its `bun.lock` is
/// accepted, and its own source is not checked as a page template (a bare
/// `addEventListener` is fine in library code), while the same file outside a
/// recipe folder still is.
#[tokio::test]
async fn a_recipe_lives_whole_in_the_repository() {
    let root = temp_dir("project-library-recipe");
    let mut config = zebflow::platform::PlatformConfig::default();
    config.data_root = root.0.clone();
    config.default_password = "test-pass".to_string();
    let platform = std::sync::Arc::new(zebflow::platform::PlatformService::from_config(config).expect("platform"));
    let ops = zebflow::platform::services::ops::PlatformOps::new(platform.clone(), OWNER, PROJECT);
    let recipe = "apiVersion: zebflow.com/v1\nkind: LibraryRecipe\nmetadata:\n  name: site-a-lib\nspec:\n  version: 0.1.0\n  entries:\n    world: src/world.ts\n";
    let source = "export function start() {\n  addEventListener(\"resize\", () => {});\n}\n";
    for (path, content) in [("libraries/site-a-lib/library.yaml", recipe), ("libraries/site-a-lib/bun.lock", "{}\n")] {
        let written = ops.file_write(path, content);
        assert!(!written.text.starts_with("Error"), "{path}: {}", written.text);
    }
    let inside = ops.file_write("libraries/site-a-lib/src/world.ts", source);
    assert!(!inside.text.contains("template_problems"), "recipe source: {}", inside.text);
    let outside = ops.file_write("components/world.ts", source);
    assert!(outside.text.contains("template_problems"), "a component is still checked: {}", outside.text);
    let stray = ops.file_write("notes/other.lock", "x");
    assert!(stray.text.starts_with("Error"), "only bun.lock, not every .lock: {}", stray.text);
}

/// One-entry library standing in for a built-in one: `{v}/runtime/main.mjs`
/// passes on `parts.mjs`, which exports `names`.
fn override_library(name: &str, version: &str, names: &[String]) -> Vec<u8> {
    let v = version;
    let declared: Vec<String> = names.iter().filter(|n| n.as_str() != "default").map(|n| format!("export const {n} = 1;")).collect();
    let mut parts = declared.join("\n");
    if names.iter().any(|n| n == "default") {
        parts.push_str("\nexport default 1;");
    }
    let files: Vec<(String, Vec<u8>)> = vec![
        (format!("{v}/runtime/main.mjs"), b"export * from \"./parts.mjs\";\nexport const addedByOverride = 2;\n".to_vec()),
        (format!("{v}/runtime/parts.mjs"), format!("{parts}\n").into_bytes()),
    ];
    let record = |path: &str| {
        let bytes = &files.iter().find(|(p, _)| p == path).unwrap().1;
        json!({ "size_bytes": bytes.len(), "integrity": digest(bytes) })
    };
    let main = format!("{v}/runtime/main.mjs");
    let manifest = json!({
        "apiVersion": "zebflow.com/v1",
        "kind": "RweLibraryManifest",
        "metadata": { "name": name },
        "spec": {
            "name": name,
            "description": "an override",
            "exports": names.iter().filter(|n| n.as_str() != "default").collect::<Vec<_>>(),
            "versions": { v: {
                "entry": main,
                "size_bytes": record(&main)["size_bytes"],
                "integrity": record(&main)["integrity"],
                "source": "offline",
                "package_version": v,
                "entries": { "rooms": { "file": main, "exports": names.iter().filter(|n| n.as_str() != "default").collect::<Vec<_>>() } },
                "files": files.iter().map(|(p, _)| (p.clone(), record(p))).collect::<serde_json::Map<String, Value>>()
            } },
            "license": { "spdx": "MIT", "holders": ["Site A"], "notices": ["THIRD-PARTY-NOTICES.txt"] }
        }
    });
    let mut all: Vec<(&str, Vec<u8>)> = files.iter().map(|(p, b)| (p.as_str(), b.clone())).collect();
    all.push(("manifest.json", serde_json::to_vec_pretty(&manifest).unwrap()));
    all.push(("THIRD-PARTY-NOTICES.txt", b"MIT\n".to_vec()));
    tar_gz(&all)
}

/// The page script of a published page, as the browser fetches it.
async fn page_script(app: &axum::Router, cookie: &str, route: &str) -> String {
    let (status, _, html) = raw(app, cookie, "GET", &format!("/wh/{OWNER}/{PROJECT}/{route}"), "text/html", Vec::new()).await;
    let html = String::from_utf8_lossy(&html).to_string();
    assert_eq!(status, StatusCode::OK, "{html}");
    assert!(!html.contains("RWE component error"), "{html}");
    let src = html.split("data-rwe-external=\"page\" src=\"").nth(1).and_then(|rest| rest.split('"').next()).expect("page script").to_string();
    let (_, _, script) = raw(app, cookie, "GET", &src, "", Vec::new()).await;
    String::from_utf8_lossy(&script).to_string()
}

/// `rwe-library-manifest` § Overrides: an uploaded library stands in for a
/// built-in one in this project's pages only when it exports every name the
/// built-in one does; removing it brings the built-in one back.
#[tokio::test]
async fn an_override_replaces_a_built_in_library_only_with_its_whole_interface() {
    let root = temp_dir("project-library-override");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    let names: Vec<String> = zebflow::platform::services::project_library::overrides::built_in_names("zeb/rooms")
        .expect("built-in names")
        .into_iter()
        .collect();
    let some_name = names.iter().find(|n| n.as_str() != "default").expect("a named export").clone();
    let page = format!("import {{ {some_name} }} from \"zeb/rooms\";\nexport default function Page() {{\n  return <main>rooms page</main>;\n}}\n");
    let (status, _, _) = raw(&app, &cookie, "PUT", &format!("/api/projects/{OWNER}/{PROJECT}/repo/file?path=pages/rooms.tsx"), "text/plain", page.into_bytes()).await;
    assert_eq!(status, StatusCode::OK);
    publish(&app, &cookie, "rooms-page", "| trigger.webhook --route /rooms-page --method GET | web.response.send --template pages/rooms.tsx").await;
    let built_in = "zeb/rooms/0.1/runtime/rooms.bundle.mjs";
    assert!(page_script(&app, &cookie, "rooms-page").await.contains(built_in));

    let upload_as = |archive: Vec<u8>| {
        let (app, cookie) = (app.clone(), cookie.clone());
        async move {
            let uri = format!("/api/projects/{OWNER}/{PROJECT}/rwe/libraries/upload?override=zeb/rooms");
            let (status, _, body) = raw(&app, &cookie, "POST", &uri, "application/gzip", archive).await;
            (status, serde_json::from_slice::<Value>(&body).unwrap_or(Value::Null))
        }
    };

    // One name short: refused, naming it, and nothing is installed.
    let short: Vec<String> = names.iter().filter(|n| **n != some_name).cloned().collect();
    let (status, answer) = upload_as(override_library("rooms-short", "0.1.0", &short)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{answer}");
    let message = answer["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("does not export 1 of the") && message.contains(&some_name), "{message}");
    let lock = std::fs::read_to_string(root.0.join("users/superadmin/default/repo/zeb.lock")).unwrap_or_default();
    assert!(!lock.contains("rooms-short"), "nothing installed: {lock}");

    // Every name, plus one: installed, checked, and the page loads it instead.
    let (status, answer) = upload_as(override_library("rooms-next", "0.1.0", &names)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["override"]["library"], "zeb/rooms");
    assert_eq!(answer["override"]["with"], "rooms-next");
    assert_eq!(answer["override"]["built_in_names"], json!(names.len()));
    assert_eq!(answer["override"]["added_names"], json!(1));
    assert_eq!(answer["override"]["files_checked"], json!(["pages/rooms.tsx"]));
    let (_, _, listed) = raw(&app, &cookie, "GET", &format!("/api/projects/{OWNER}/{PROJECT}/rwe/libraries"), "application/json", Vec::new()).await;
    let listed: Value = serde_json::from_slice(&listed).unwrap();
    let rooms = listed.as_array().unwrap().iter().find(|item| item["name"] == "zeb/rooms").cloned().expect("listed");
    assert_eq!(rooms["overridden_by"], json!({"library": "rooms-next", "entry": "rooms"}));
    let script = page_script(&app, &cookie, "rooms-page").await;
    assert!(script.contains("_rwe/lib/rooms-next/0.1.0/runtime/main.mjs") && !script.contains(built_in), "{script}");

    // A new version that drops a name would break the override: refused.
    let uri = format!("/api/projects/{OWNER}/{PROJECT}/rwe/libraries/upload");
    let (status, _, body) = raw(&app, &cookie, "POST", &uri, "application/gzip", override_library("rooms-next", "0.2.0", &short)).await;
    let answer: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{answer}");
    assert!(answer["error"]["message"].as_str().unwrap_or_default().contains("would break the override of zeb/rooms"), "{answer}");

    // Removed: the built-in library loads again.
    let override_uri = format!("/api/projects/{OWNER}/{PROJECT}/rwe/libraries/override");
    let (status, _, body) = raw(&app, &cookie, "DELETE", &format!("{override_uri}?library=zeb/rooms"), "", Vec::new()).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert!(page_script(&app, &cookie, "rooms-page").await.contains(built_in));

    // Set again on the installed library; removing the library drops it.
    let set = json!({"library": "zeb/rooms", "with": "rooms-next"}).to_string().into_bytes();
    let (status, _, body) = raw(&app, &cookie, "POST", &override_uri, "application/json", set).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert!(page_script(&app, &cookie, "rooms-page").await.contains("rooms-next"));
    let remove = format!("/api/projects/{OWNER}/{PROJECT}/rwe/libraries/remove?name=rooms-next");
    let (status, _, body) = raw(&app, &cookie, "DELETE", &remove, "", Vec::new()).await;
    let answer: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    assert_eq!((status, answer["overrides_removed"].clone()), (StatusCode::OK, json!(["zeb/rooms"])), "{answer}");
    assert!(page_script(&app, &cookie, "rooms-page").await.contains(built_in));

    // Only a built-in bundle library can be replaced.
    let set = json!({"library": "zeb/react", "with": "rooms-next"}).to_string().into_bytes();
    let (status, _, body) = raw(&app, &cookie, "POST", &override_uri, "application/json", set).await;
    let answer: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{answer}");
    assert!(answer["error"]["message"].as_str().unwrap_or_default().contains("not a built-in library an override can replace"), "{answer}");
}

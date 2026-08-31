//! Credential encryption, exercised where an operator meets it.
//!
//! `docs/contracts/kinds/credential/README.md` states four rules this file
//! turns into assertions: the whole secret half is ciphertext at rest, the
//! ciphertext names the key that made it, a missing key refuses to start rather
//! than regenerating, and rotation installs a new key without re-encrypting
//! anything.
//!
//! The at-rest assertion deliberately scans **every file under the data root**
//! rather than the one column. A secret that leaked into a WAL frame, a
//! journal, a log, or a cache would satisfy a column check and fail the thing
//! the contract protects: "a copied database — a backup, a support bundle, a
//! snapshot, a misplaced volume".

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use zebflow::platform::{PlatformConfig, PlatformService};

const SECRET: &str = "correct-horse-battery-staple-9271";
const HOST: &str = "db.internal.example";

fn temp_test_dir(name: &str) -> PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("zebflow-credential-{name}-{now}"))
}

async fn response_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body bytes");
    serde_json::from_slice(&body).expect("json body")
}

async fn login(app: &axum::Router) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/login")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("identifier=superadmin&password=test-pass"))
                .expect("request"),
        )
        .await
        .expect("login response");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    response
        .headers()
        .get(header::SET_COOKIE)
        .expect("set-cookie")
        .to_str()
        .expect("cookie")
        .to_string()
}

async fn call(app: &axum::Router, method: &str, uri: &str, cookie: &str, body: Value) -> Value {
    let request = Request::builder()
        .uri(uri)
        .method(method)
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "application/json");
    let response = app
        .clone()
        .oneshot(
            request
                .body(if body.is_null() {
                    Body::empty()
                } else {
                    Body::from(body.to_string())
                })
                .expect("request"),
        )
        .await
        .expect("response");
    assert!(
        response.status().is_success(),
        "{method} {uri} -> {}",
        response.status()
    );
    response_json(response).await
}

/// Every regular file under `root`, so nothing is scanned by name.
fn all_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                found.push(path);
            }
        }
    }
    found
}

fn files_containing(root: &Path, needle: &str) -> Vec<PathBuf> {
    all_files(root)
        .into_iter()
        .filter(|path| {
            std::fs::read(path)
                .map(|bytes| {
                    bytes
                        .windows(needle.len())
                        .any(|window| window == needle.as_bytes())
                })
                .unwrap_or(false)
        })
        .collect()
}

#[tokio::test]
async fn a_credential_saved_through_the_api_appears_nowhere_in_the_data_root_as_plaintext() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("at-rest");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform"));
    let app = zebflow::platform::web::router(platform.clone()).await;
    let cookie = login(&app).await;

    call(
        &app,
        "POST",
        "/api/projects/superadmin/default/credentials",
        &cookie,
        json!({
            "credential_id": "prod-db",
            "title": "Production database",
            "kind": "postgres",
            "secret": {"host": HOST, "user": "app", "password": SECRET},
            "notes": "billing account 4"
        }),
    )
    .await;

    // (a) The secret half is unreadable, and not only in its own column: the
    // whole tree is scanned, WAL frames and all.
    let leaks = files_containing(&data_root, SECRET);
    assert!(
        leaks.is_empty(),
        "the secret is on disk in the clear: {leaks:?}"
    );
    // The public-looking half went with it — the contract encrypts the whole
    // secret blob "regardless of what the type declared".
    let leaks = files_containing(&data_root, HOST);
    assert!(
        leaks.is_empty(),
        "a declared-public value leaked: {leaks:?}"
    );
    // And something *is* stored: this test would pass on an empty database.
    assert!(
        !files_containing(&data_root, "zfc1:1:").is_empty(),
        "no ciphertext was written at all"
    );
    // The notes are not the secret half, and are stored as they are.
    assert!(!files_containing(&data_root, "billing account 4").is_empty());

    // (b) The same credential reads back through the API, whole.
    let read = call(
        &app,
        "GET",
        "/api/projects/superadmin/default/credentials/prod-db",
        &cookie,
        Value::Null,
    )
    .await;
    assert_eq!(read["credential"]["secret"]["password"], json!(SECRET));
    assert_eq!(read["credential"]["secret"]["host"], json!(HOST));
    assert_eq!(read["credential"]["kind"], json!("postgres"));

    // The listing still says a secret is there without carrying one.
    let listed = call(
        &app,
        "GET",
        "/api/projects/superadmin/default/credentials",
        &cookie,
        Value::Null,
    )
    .await;
    let items = listed["items"].as_array().expect("credentials array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["has_secret"], json!(true));
    assert!(
        !listed.to_string().contains(SECRET),
        "the list response carried the secret"
    );

    // (e) The instance key is not a world-readable file.
    let key_path = data_root.join("platform/credential-key");
    assert!(key_path.is_file());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&key_path)
            .expect("meta")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
    // …and it is not in the database beside the ciphertext.
    let key_text = std::fs::read_to_string(&key_path).expect("key file");
    let key_body = key_text.trim().to_string();
    let elsewhere: Vec<_> = files_containing(&data_root, &key_body)
        .into_iter()
        .filter(|path| path != &key_path)
        .collect();
    assert!(
        elsewhere.is_empty(),
        "the instance key is stored beside the ciphertext: {elsewhere:?}"
    );

    let _ = std::fs::remove_dir_all(&data_root);
}

#[tokio::test]
async fn rotation_and_rekey_are_online_and_leave_every_credential_readable() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("rotate");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform"));
    let app = zebflow::platform::web::router(platform.clone()).await;
    let cookie = login(&app).await;

    let save = |id: &'static str, value: &'static str| {
        let app = app.clone();
        let cookie = cookie.clone();
        async move {
            call(
                &app,
                "POST",
                "/api/projects/superadmin/default/credentials",
                &cookie,
                json!({
                    "credential_id": id,
                    "title": id,
                    "kind": "api_key",
                    "secret": {"key": value}
                }),
            )
            .await
        }
    };

    save("first", "sealed-under-one").await;

    let keyring = call(
        &app,
        "GET",
        "/api/admin/credentials/keyring",
        &cookie,
        Value::Null,
    )
    .await;
    assert_eq!(keyring["keyring"]["current_key_id"], json!(1));
    assert_eq!(keyring["keyring"]["source"], json!("file"));
    assert_eq!(
        keyring["keyring"]["generations"].as_array().unwrap().len(),
        1
    );

    // (d) Rotation installs a new generation and re-encrypts nothing.
    let rotated = call(
        &app,
        "POST",
        "/api/admin/credentials/rotate",
        &cookie,
        Value::Null,
    )
    .await;
    assert_eq!(rotated["keyring"]["current_key_id"], json!(2));
    let generations = rotated["keyring"]["generations"]
        .as_array()
        .expect("generations");
    assert_eq!(generations.len(), 2, "older keys stay for reads");

    save("second", "sealed-under-two").await;

    // New writes name the new key; the old envelope is untouched and readable.
    assert!(!files_containing(&data_root, "zfc1:1:").is_empty());
    assert!(!files_containing(&data_root, "zfc1:2:").is_empty());
    for (id, expected) in [
        ("first", "sealed-under-one"),
        ("second", "sealed-under-two"),
    ] {
        let read = call(
            &app,
            "GET",
            &format!("/api/projects/superadmin/default/credentials/{id}"),
            &cookie,
            Value::Null,
        )
        .await;
        assert_eq!(read["credential"]["secret"]["key"], json!(expected));
    }

    // The sweep is the separate operation that retires a generation.
    let sweep = call(
        &app,
        "POST",
        "/api/admin/credentials/reencrypt",
        &cookie,
        Value::Null,
    )
    .await;
    assert_eq!(sweep["sweep"]["key_id"], json!(2));
    assert_eq!(sweep["sweep"]["examined"], json!(2));
    assert_eq!(sweep["sweep"]["rewritten"], json!(1));
    // The sweep rebuilds the catalog, so the superseded envelope stops being in
    // the file rather than merely stopping being referenced — SQLite keeps a
    // freed page's bytes until it reuses them, and `strings catalog.db` is the
    // reader this whole feature exists to defeat.
    assert_eq!(sweep["sweep"]["compacted"], json!(true));
    assert!(
        files_containing(&data_root, "zfc1:1:").is_empty(),
        "after the sweep nothing names the retired generation"
    );

    // Rekey replaces the instance key and touches no credential.
    let key_path = data_root.join("platform/credential-key");
    let before = std::fs::read_to_string(&key_path).expect("key file");
    call(
        &app,
        "POST",
        "/api/admin/credentials/rekey",
        &cookie,
        Value::Null,
    )
    .await;
    let after = std::fs::read_to_string(&key_path).expect("key file");
    assert_ne!(before, after);
    let read = call(
        &app,
        "GET",
        "/api/projects/superadmin/default/credentials/first",
        &cookie,
        Value::Null,
    )
    .await;
    assert_eq!(
        read["credential"]["secret"]["key"],
        json!("sealed-under-one")
    );

    // Nothing here is reachable without being superadmin.
    for uri in [
        "/api/admin/credentials/keyring",
        "/api/admin/credentials/rotate",
        "/api/admin/credentials/rekey",
        "/api/admin/credentials/reencrypt",
    ] {
        let method = if uri.ends_with("keyring") {
            "GET"
        } else {
            "POST"
        };
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .method(method)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{uri}");
    }

    let _ = std::fs::remove_dir_all(&data_root);
}

/// (c) The refusal, from a whole platform rather than from an adapter.
#[tokio::test]
async fn an_instance_restarted_without_its_key_refuses_to_start() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("refuse");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();
    {
        let platform = Arc::new(PlatformService::from_config(config).expect("platform"));
        let app = zebflow::platform::web::router(platform.clone()).await;
        let cookie = login(&app).await;
        call(
            &app,
            "POST",
            "/api/projects/superadmin/default/credentials",
            &cookie,
            json!({
                "credential_id": "prod-db",
                "title": "Production database",
                "kind": "postgres",
                "secret": {"password": SECRET}
            }),
        )
        .await;
    }

    let key_path = data_root.join("platform/credential-key");
    let saved = std::fs::read_to_string(&key_path).expect("key file");
    std::fs::remove_file(&key_path).expect("remove key");

    let mut restart = PlatformConfig::default();
    restart.data_root = data_root.clone();
    restart.default_password = "test-pass".to_string();
    let err = PlatformService::from_config(restart)
        .err()
        .expect("a keyless instance must refuse to start");
    assert_eq!(err.code, "PLATFORM_CREDENTIAL_KEY_MISSING");
    assert!(err.message.contains("cannot be regenerated"), "{err:?}");
    assert!(
        err.message.contains("ZEBFLOW_CREDENTIAL_KEY"),
        "the refusal must name both ways back: {err:?}"
    );
    assert!(!key_path.exists(), "a refusal must never write a new key");

    // An altered key is refused too, and just as loudly.
    std::fs::write(
        &key_path,
        "zfk1:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    )
    .expect("write");
    let mut altered = PlatformConfig::default();
    altered.data_root = data_root.clone();
    altered.default_password = "test-pass".to_string();
    let err = PlatformService::from_config(altered)
        .err()
        .expect("a wrong key must refuse to start");
    assert_eq!(err.code, "PLATFORM_CREDENTIAL_KEY_UNUSABLE");

    // Restoring the key restores the instance, credential intact.
    std::fs::write(&key_path, saved).expect("restore");
    let mut restored = PlatformConfig::default();
    restored.data_root = data_root.clone();
    restored.default_password = "test-pass".to_string();
    let platform = Arc::new(PlatformService::from_config(restored).expect("restored"));
    let app = zebflow::platform::web::router(platform.clone()).await;
    let cookie = login(&app).await;
    let read = call(
        &app,
        "GET",
        "/api/projects/superadmin/default/credentials/prod-db",
        &cookie,
        Value::Null,
    )
    .await;
    assert_eq!(read["credential"]["secret"]["password"], json!(SECRET));

    let _ = std::fs::remove_dir_all(&data_root);
}

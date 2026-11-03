//! `/stats` of a GeoParquet layer through a real router: it answers
//! `computing` until the background computation has saved its result in the
//! layer's cache folder, then serves that result, and starts over when the
//! layer's file changes. The public view shows only `allowed_properties`.

use std::fs;
use std::path::Path;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use zebflow::contracts::kinds::MapserverLayerRecord;
use zebflow::mapserver::publish::registry;
use zebflow::mapserver::resolve::geoparquet_optimize::{
    convert_geojson_to_geoparquet, optimize_geoparquet,
};
use zebflow::platform::{PlatformConfig, build_router};

struct TestDir(std::path::PathBuf);

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn temp_dir(name: &str) -> TestDir {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    TestDir(std::env::temp_dir().join(format!("zebflow-platform-{name}-{now}")))
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
        .expect("login");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .expect("cookie")
        .to_str()
        .expect("text");
    cookie.split(';').next().unwrap_or_default().to_string()
}

async fn get(app: &axum::Router, uri: &str, cookie: Option<&str>) -> (StatusCode, Value) {
    let mut request = Request::builder().uri(uri);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).expect("request"))
        .await
        .expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

/// An optimized GeoParquet copy of `rows` invented sites: `suburb` north,
/// south or east by `i % 3`, and `floors` from 1 to 4.
fn write_layer(scratch: &Path, dest: &Path, rows: usize) {
    let features: Vec<Value> = (0..rows)
        .map(|i| {
            let suburb = ["north", "south", "east"][i % 3];
            json!({
                "type": "Feature",
                "properties": { "suburb": suburb, "floors": 1 + i % 4 },
                "geometry": { "type": "Point", "coordinates": [100.0 + i as f64 * 0.001, -5.0] }
            })
        })
        .collect();
    let geojson = scratch.join("sites.geojson");
    let raw = scratch.join("sites.raw.parquet");
    fs::write(
        &geojson,
        json!({ "type": "FeatureCollection", "features": features }).to_string(),
    )
    .expect("geojson");
    convert_geojson_to_geoparquet(&geojson, &raw).expect("convert");
    fs::create_dir_all(dest.parent().expect("folder")).expect("optimized folder");
    optimize_geoparquet(&raw, dest).expect("optimize");
}

/// Asks the public `/stats` until it is no longer computing.
async fn until_ready(app: &axum::Router) -> Value {
    for _ in 0..300 {
        let (status, body) = get(app, "/ms/superadmin/default/sites/stats", None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        if body["state"] != "computing" {
            return body;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("statistics were not ready within 30 s");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_geoparquet_layers_stats_are_computed_once_and_then_read() {
    let root = temp_dir("mapserver-stats");
    let mut config = PlatformConfig::default();
    config.data_root = root.0.clone();
    config.default_password = "test-pass".to_string();
    let app = build_router(config).await.expect("router");
    let cookie = login(&app).await;

    let project = root.0.join("users").join("superadmin").join("default");
    let optimized = project
        .join("files")
        .join("mapserver")
        .join(".optimized")
        .join("sites.spatial.parquet");
    let scratch = root.0.join("scratch");
    fs::create_dir_all(&scratch).expect("scratch");
    write_layer(&scratch, &optimized, 300);

    let record = MapserverLayerRecord {
        layer_id: "sites".to_string(),
        path: "sites".to_string(),
        store: "local".to_string(),
        source_path: "mapserver/.optimized/sites.spatial.parquet".to_string(),
        source_kind: "geoparquet".to_string(),
        artifact_manifest_path: None,
        mode: "features".to_string(),
        min_zoom: None,
        max_zoom: None,
        bbox_required: false,
        max_features: 100,
        allowed_properties: vec!["suburb".to_string()],
        feature_count: Some(300),
        chunk_count: None,
        style: None,
        filter: None,
        function_slug: None,
        cache_ttl_secs: None,
    };
    let registry_file = registry::layers_manifest_path(
        &project.join("data").join("store"),
        registry::DEFAULT_INSTANCE,
    );
    fs::create_dir_all(registry_file.parent().expect("folder")).expect("registry folder");
    registry::write_layers(&registry_file, registry::DEFAULT_INSTANCE, &[record])
        .expect("registry");

    let addressing = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/settings/addressing")
                .method("PUT")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "data": { "hosts": [], "routes": [], "disabled": ["fs"] } })
                        .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("addressing");
    assert_eq!(
        addressing.status(),
        StatusCode::OK,
        "switch the ms surface on"
    );

    // Nothing is saved yet: the first answer says so, with the footer's row
    // count, and queues the computation.
    let saved = project
        .join("data")
        .join("cache")
        .join("mapserver-artifacts")
        .join(registry::DEFAULT_INSTANCE)
        .join("sites")
        .join("stats.json");
    assert!(!saved.exists());
    let (status, first) = get(&app, "/ms/superadmin/default/sites/stats", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["state"], "computing", "{first}");
    assert_eq!(first["row_count"], 300);
    assert_eq!(first["columns"], json!([]));

    let ready = until_ready(&app).await;
    assert_eq!(ready["state"], "ready", "{ready}");
    assert!(
        ready["computed_at"]
            .as_str()
            .is_some_and(|at| at.ends_with('Z')),
        "{ready}"
    );
    assert_eq!(ready["row_count"], 300);
    assert!(ready["extent"].is_array(), "{ready}");
    assert!(ready.get("allowed_properties").is_none());
    let columns = ready["columns"].as_array().expect("columns");
    assert_eq!(
        columns.len(),
        1,
        "the public sees only the allowed column: {ready}"
    );
    assert_eq!(columns[0]["name"], "suburb");
    assert_eq!(
        columns[0]["top_values"],
        json!([["east", 100], ["north", 100], ["south", 100]])
    );
    assert!(
        saved.exists(),
        "the result is saved in the layer's cache folder"
    );

    let (status, operator) = get(
        &app,
        "/api/projects/superadmin/default/mapserver/default-mapserver/layers/sites/stats",
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(operator["state"], "ready");
    assert_eq!(operator["allowed_properties"], json!(["suburb"]));
    let floors = operator["columns"]
        .as_array()
        .expect("columns")
        .iter()
        .find(|c| c["name"] == "floors")
        .expect("the operator sees every column");
    assert_eq!(
        (floors["min"].as_f64(), floors["max"].as_f64()),
        (Some(1.0), Some(4.0))
    );
    let counts = floors["histogram"]["counts"].as_array().expect("histogram");
    assert_eq!(counts.iter().filter_map(Value::as_u64).sum::<u64>(), 300);

    // A changed file is computing again until its own result is saved.
    write_layer(&scratch, &optimized, 450);
    let (_, changed) = get(&app, "/ms/superadmin/default/sites/stats", None).await;
    assert_eq!(changed["state"], "computing", "{changed}");
    assert_eq!(changed["row_count"], 450);
    let fresh = until_ready(&app).await;
    assert_eq!(fresh["state"], "ready");
    assert_eq!(fresh["row_count"], 450);

    let (status, _) = get(&app, "/ms/superadmin/default/nowhere/stats", None).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "an unknown layer is the only 404"
    );
}

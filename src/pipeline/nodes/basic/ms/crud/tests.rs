use std::sync::Arc;

use serde_json::{Value, json};

use super::*;

/// A platform on a temporary data root, removed when the guard drops.
fn platform() -> (tempfile::TempDir, Arc<PlatformService>) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut config = crate::platform::model::PlatformConfig::default();
    config.data_root = tmp.path().join("platform");
    (tmp, Arc::new(PlatformService::from_config(config).expect("platform")))
}

async fn run(platform: &Arc<PlatformService>, kind: &str, config: Value) -> Result<Value, PipelineError> {
    let config: Config = serde_json::from_value(config).expect("config");
    let node = Node::new(config, platform.clone(), Operation::from_kind(kind).expect("an ms kind"))?;
    let kept = json!({ "keep": 1 });
    let out = node
        .execute_async(NodeExecutionInput {
            node_id: "n0".to_string(),
            input_pin: "in".to_string(),
            payload: kept,
            metadata: json!({ "owner": "demo", "project": "demo", "pipeline": "test", "request_id": "r1" }),
            bus: None,
        })
        .await?;
    Ok(out.payload)
}

fn settings(config: Value) -> Result<PublishSettings, PipelineError> {
    serde_json::from_value::<Config>(config).expect("config").publish_settings()
}

const ROADS: &str = r#"{"type":"FeatureCollection","features":[
  {"type":"Feature","properties":{"name":"Main St","lanes":2,"owner":"council"},"geometry":{"type":"LineString","coordinates":[[144.96,-37.81],[144.97,-37.82]]}},
  {"type":"Feature","properties":{"name":"High St","lanes":4,"owner":"state"},"geometry":{"type":"LineString","coordinates":[[144.98,-37.80],[144.99,-37.79]]}}
]}"#;

#[test]
fn publish_flags_read_in_their_0_11_words() {
    let graph = crate::platform::shell::parser::build_pipeline_graph(
        "t",
        "[a] trigger.manual\n[b] ms.layer.publish --name roads --route roads --from data/roads.geojson --parse geojson --field name --field lanes --max-items 200 --min-zoom 4 --max-zoom 12 --stroke-width 2.5\n[a] -> [b]\n",
    )
    .expect("graph");
    let config: Config = serde_json::from_value(graph.nodes[1].config.clone()).expect("config");
    let got = config.publish_settings().expect("settings");
    assert_eq!(got.parse, Some("geojson"));
    assert_eq!(got.fields, vec!["name".to_string(), "lanes".to_string()]);
    assert_eq!(got.max_items, 200);
    assert_eq!((got.min_zoom, got.max_zoom), (Some(4), Some(12)));
    assert_eq!(got.stroke_width, Some(2.5));
    assert_eq!(got.ttl_secs, None);

    let flags: Vec<String> = publish_definition().dsl_flags.into_iter().map(|f| f.flag).collect();
    for gone in ["--source-kind", "--max-features", "--allowed-properties", "--cache-ttl"] {
        assert!(!flags.iter().any(|f| f == gone), "{gone} is gone: {flags:?}");
    }
}

#[test]
fn publish_refuses_what_it_cannot_mean() {
    let base = json!({ "name": "roads", "route": "roads", "from": "data/roads.geojson" });
    let with = |extra: Value| {
        let mut config = base.clone();
        config.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        settings(config)
    };
    assert_eq!(settings(base.clone()).unwrap().max_items, DEFAULT_MAX_ITEMS as usize);
    for (extra, words) in [
        (json!({ "parse": "shapefile" }), "--parse"),
        (json!({ "max_items": 0 }), "--max-items"),
        (json!({ "max_items": 50_001 }), "--max-items"),
        (json!({ "field": "name,lanes" }), "repeat --field"),
        (json!({ "min_zoom": 12, "max_zoom": 4 }), "above"),
        (json!({ "max_zoom": 25 }), "--max-zoom"),
        (json!({ "stroke_width": "wide" }), "--stroke-width"),
        (json!({ "ttl": "30s" }), "needs --function"),
        (json!({ "function": "geo/live" }), "two sources"),
        (json!({ "build_artifact": true }), "--skip-optimize"),
    ] {
        let err = with(extra.clone()).expect_err(&extra.to_string());
        assert_eq!(err.code, PUBLISH_CONFIG_CODE, "{extra}");
        assert!(err.message.contains(words), "{extra}: {}", err.message);
    }
    assert!(settings(json!({ "name": "roads", "route": "roads" })).unwrap_err().message.contains("--from is required"));

    let function = settings(json!({ "name": "live", "route": "live", "function": "geo/live", "ttl": "5m" })).unwrap();
    assert_eq!(function.ttl_secs, Some(300));
    assert!(settings(json!({ "name": "live", "route": "live", "function": "geo/live", "ttl": "30" })).is_err(), "a bare number names no unit");
    assert!(settings(json!({ "name": "live", "route": "live", "function": "geo/live", "ttl": "8d" })).is_err(), "over the ceiling");
}

/// `ref` is opaque to every node but its own backend
/// (`kinds/file-ref/README.md`): a remote handle is never joined to a local path.
#[test]
fn from_refuses_a_foreign_backend_ref() {
    let from = json!({
        "__zf_type": "file_ref", "backend": "gdrive", "store": "x",
        "ref": "1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms", "filename": "roads.geojson",
        "mime": "application/geo+json", "kind": "geojson", "size": 1,
        "sha256": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "lifecycle": "durable", "origin": "webhook", "trust": "untrusted"
    });
    assert_eq!(zebfs_rel_path_or_string(&from).unwrap_err().code, "FW_FILE_REF_BACKEND");
}

#[tokio::test]
async fn publish_get_list_unpublish_answer_layer_and_keep_the_payload() {
    let (_tmp, p) = platform();
    let store = project_store::open_store(&p, "demo", "demo", None).expect("store");
    store.fs.put("data/roads.geojson", ROADS.as_bytes()).expect("put");

    let published = run(&p, PUBLISH_KIND, json!({ "name": "roads", "route": "/roads/", "from": "data/roads.geojson", "field": ["name"] }))
        .await
        .expect("publish");
    assert_eq!(published["keep"], 1, "the payload is kept");
    let layer = &published["layer"];
    assert_eq!(layer["name"], "roads");
    assert_eq!(layer["route"], "roads");
    assert_eq!(layer["source_kind"], "geoparquet");
    assert_eq!(layer["source"], "mapserver/.optimized/roads.spatial.parquet");
    assert_eq!(layer["fields"], json!(["name"]));
    assert_eq!(layer["max_items"], DEFAULT_MAX_ITEMS);
    assert_eq!(layer["feature_count"], 2);
    assert_eq!(layer["optimization"]["applied"], true);
    assert!(layer.get("serving").is_some_and(Value::is_boolean));
    assert!(store.fs.head("mapserver/.optimized/roads.spatial.parquet").is_ok(), "the optimized copy is in the store");

    let got = run(&p, GET_KIND, json!({ "name": "roads" })).await.expect("get");
    assert_eq!(got["layer"]["found"], true);
    assert_eq!(got["layer"]["source_kind"], "geoparquet");
    let missing = run(&p, GET_KIND, json!({ "name": "rivers" })).await.expect("get");
    assert_eq!(missing["layer"], json!({ "found": false, "name": "rivers" }));

    let listed = run(&p, LIST_KIND, json!({})).await.expect("list");
    assert_eq!(listed["layer"]["count"], 1);
    assert_eq!(listed["layer"]["items"][0]["name"], "roads");

    let removed = run(&p, UNPUBLISH_KIND, json!({ "name": "roads" })).await.expect("unpublish");
    assert_eq!(removed["layer"], json!({ "name": "roads", "removed": true }));
    assert!(store.fs.head("mapserver/.optimized/roads.spatial.parquet").is_err(), "the optimized copy goes with it");
    assert!(store.fs.head("data/roads.geojson").is_ok(), "the source stays");
    let again = run(&p, UNPUBLISH_KIND, json!({ "name": "roads" })).await.expect("unpublish");
    assert_eq!(again["layer"]["removed"], false);
    assert_eq!(run(&p, LIST_KIND, json!({})).await.expect("list")["layer"]["count"], 0);
}

#[tokio::test]
async fn skip_optimize_serves_the_source_and_a_function_layer_keeps_its_ttl() {
    let (_tmp, p) = platform();
    let store = project_store::open_store(&p, "demo", "demo", None).expect("store");
    store.fs.put("data/roads.json", ROADS.as_bytes()).expect("put");
    store.fs.put("data/roads.txt", ROADS.as_bytes()).expect("put");

    let as_is = run(&p, PUBLISH_KIND, json!({ "name": "raw", "route": "raw", "from": "data/roads.json", "skip_optimize": true }))
        .await
        .expect("publish");
    assert_eq!(as_is["layer"]["source"], "data/roads.json");
    assert_eq!(as_is["layer"]["source_kind"], "geojson_file");
    assert!(as_is["layer"].get("optimization").is_none());

    let chunked = run(
        &p,
        PUBLISH_KIND,
        json!({ "name": "chunked", "route": "chunked", "from": "data/roads.json", "skip_optimize": true, "build_artifact": true }),
    )
    .await
    .expect("publish");
    assert_eq!(chunked["layer"]["source_kind"], "geojson_artifact");
    assert_eq!(chunked["layer"]["feature_count"], 2);

    let live = run(&p, PUBLISH_KIND, json!({ "name": "live", "route": "live", "function": "geo/live", "ttl": "30s", "bbox_optional": true }))
        .await
        .expect("publish");
    assert_eq!(live["layer"]["source_kind"], "geojson_function");
    assert_eq!(live["layer"]["function"], "geo/live");
    assert_eq!(live["layer"]["ttl"], "30s");
    assert_eq!(live["layer"]["bbox_required"], false);

    let unknown = run(&p, PUBLISH_KIND, json!({ "name": "odd", "route": "odd", "from": "data/roads.txt" })).await.unwrap_err();
    assert_eq!(unknown.code, PUBLISH_CONFIG_CODE);
    assert!(unknown.message.contains("--parse"), "{}", unknown.message);
}

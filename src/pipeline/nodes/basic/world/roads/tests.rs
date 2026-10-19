//! `world.roads.bake` on a small invented place: a seed in the store, a pack out.
//!
//! The place is synthetic. A grid of ground 800 m on a side, sloping 1 % east
//! and 0.5 % south, and four street lines over a 600 m box near the origin
//! (0°, 0°): two straight streets that cross, a street that bends and meets
//! the cross street from the side. The baker keeps its design 30 m inside the
//! grid, so the ground reaches 100 m past the streets on every side.

use super::bake::*;
use crate::pipeline::nodes::shared::project_store::{NodeStore, open_store};
use crate::pipeline::nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler};
use crate::pipeline::{PipelineError, nodes::shared::test_platform::test_platform};
use crate::platform::services::PlatformService;
use serde_json::{Value, json};
use std::sync::Arc;

const SEED: &str = "world/seed";
const PACK: &str = "world/packs/roads/site-a";
/// Metres per degree of longitude at the equator, and per degree of latitude,
/// as the baker takes them.
const M_PER_LON: f64 = 111_320.0;
const M_PER_LAT: f64 = 110_574.0;

fn platform() -> crate::pipeline::nodes::shared::test_platform::TestPlatform {
    test_platform()
}

fn store(platform: &Arc<PlatformService>) -> NodeStore {
    open_store(platform, "demo", "demo", None).expect("store")
}

/// A place metre (east, south) as the seed's [lon, lat].
fn lonlat(x: f64, z: f64) -> Value {
    json!([x / M_PER_LON, -z / M_PER_LAT])
}

fn street(osm_id: u64, highway: &str, name: &str, nodes: &[u64], line: &[(f64, f64)]) -> Value {
    json!({
        "type": "Feature",
        "geometry": { "type": "LineString", "coordinates": line.iter().map(|(x, z)| lonlat(*x, *z)).collect::<Vec<_>>() },
        "properties": { "osm_id": osm_id, "nodes": nodes, "highway": highway, "name": name, "surface": "asphalt" }
    })
}

fn roads() -> String {
    json!({
        "type": "FeatureCollection",
        "attribution": "Invented streets for a test; real packs credit OpenStreetMap contributors (ODbL)",
        "bbox": [-100.0 / M_PER_LON, -700.0 / M_PER_LAT, 700.0 / M_PER_LON, 100.0 / M_PER_LAT],
        "features": [
            street(1001, "residential", "East Street", &[1, 2, 3], &[(0.0, 300.0), (300.0, 300.0), (600.0, 300.0)]),
            street(1002, "residential", "Cross Street", &[10, 2, 30, 11], &[(300.0, 0.0), (300.0, 300.0), (300.0, 440.0), (300.0, 600.0)]),
            street(1003, "tertiary", "Bend Road", &[20, 21, 22, 30], &[(40.0, 520.0), (150.0, 520.0), (230.0, 470.0), (300.0, 440.0)]),
        ]
    })
    .to_string()
}

const NX: usize = 81;
const STEP: f64 = 10.0;
const GRID_START: f64 = -100.0;

fn ground_json(nx: Value, step: Value) -> String {
    json!({ "origin": { "lon": 0.0, "lat": 0.0 }, "nx": nx, "nz": NX, "x0": GRID_START, "z0": GRID_START, "step": step, "source": "Invented ground for a test" }).to_string()
}

fn ground_f32() -> Vec<u8> {
    let mut bytes = Vec::with_capacity(NX * NX * 4);
    for j in 0..NX {
        for i in 0..NX {
            let (x, z) = (GRID_START + i as f64 * STEP, GRID_START + j as f64 * STEP);
            bytes.extend(((20.0 + 0.01 * x + 0.005 * z) as f32).to_le_bytes());
        }
    }
    bytes
}

/// The seed in the store, under `dir`, as `site-a`.
fn put_seed(p: &Arc<PlatformService>, dir: &str) {
    let s = store(p);
    s.fs.put(&format!("{dir}/site-a.geojson"), roads().as_bytes()).unwrap();
    s.fs.put(&format!("{dir}/site-a.ground.json"), ground_json(json!(NX), json!(STEP)).as_bytes()).unwrap();
    s.fs.put(&format!("{dir}/site-a.ground.f32"), &ground_f32()).unwrap();
}

async fn run_full(p: &Arc<PlatformService>, config: Value, payload: Value) -> Result<NodeExecutionOutput, PipelineError> {
    let node = Node::new(serde_json::from_value(config).expect("config"), p.clone())?;
    node.execute_async(NodeExecutionInput {
        node_id: "n0".to_string(),
        input_pin: "in".to_string(),
        payload,
        metadata: json!({ "owner": "demo", "project": "demo", "pipeline": "test", "request_id": "r1" }),
        bus: None,
    })
    .await
}

async fn run(p: &Arc<PlatformService>, config: Value, payload: Value) -> Result<Value, PipelineError> {
    Ok(run_full(p, config, payload).await?.payload)
}

fn config() -> Value {
    json!({ "from": SEED, "place": "site-a", "folder": PACK })
}

fn read_json(p: &Arc<PlatformService>, key: &str) -> Value {
    serde_json::from_slice(&store(p).read_capped(key, "T").unwrap()).unwrap()
}

#[tokio::test]
async fn a_seed_in_the_store_is_a_pack_in_the_store() {
    let p = platform();
    put_seed(&p, SEED);
    let started = std::time::Instant::now();
    let out = run_full(&p, config(), json!({ "keep": { "me": true } })).await.unwrap();
    let elapsed = started.elapsed().as_secs_f64();
    let payload = out.payload;

    // One key added, the rest kept.
    let mut keys: Vec<&str> = payload.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, vec!["keep", "roads"]);
    assert_eq!(payload["keep"], json!({ "me": true }));

    let roads = &payload["roads"];
    assert_eq!(roads["folder"], PACK);
    let items = roads["items"].as_array().unwrap();
    assert_eq!(roads["count"].as_u64(), Some(items.len() as u64));
    let mut total = 0u64;
    for item in items {
        crate::pipeline::nodes::shared::file_ref::validate_file_ref(item).expect("a contract FileRef");
        let key = item["ref"].as_str().unwrap();
        assert!(key.starts_with(&format!("{PACK}/")) && key.ends_with(".json"), "only plain .json files: {key}");
        assert_eq!((item["origin"].as_str(), item["trust"].as_str()), (Some(NODE_KIND), Some("generated")));
        total += item["size"].as_u64().unwrap();
    }
    // No `.json.gz` was stored, beside the index or anywhere else in the pack.
    for gz in ["index.json.gz", "clamp.json.gz", "terrain.json.gz", "walk.json.gz"] {
        assert!(store(&p).fs.head(&format!("{PACK}/{gz}")).is_err(), "{gz} was stored");
    }

    // The index is the pack's entry: v 2, a tile list, the sources it credits.
    let index_ref = &roads["index"];
    assert_eq!(index_ref["ref"], format!("{PACK}/index.json"));
    assert!(items.iter().any(|item| item == index_ref));
    let index = read_json(&p, &format!("{PACK}/index.json"));
    assert_eq!(index["v"], 2);
    assert_eq!(index["size"], 500.0);
    let tiles = index["tiles"].as_array().unwrap();
    assert!(!tiles.is_empty(), "the pack has tiles");
    assert!(index["sources"]["roads"]["licence"].as_str().unwrap().contains("ODbL"));
    for tile in tiles {
        let key = format!("{PACK}/{}_{}.json", tile[0], tile[1]);
        assert!(items.iter().any(|item| item["ref"] == key), "{key} is an item");
        let doc = read_json(&p, &key);
        assert_eq!((doc["v"].as_u64(), doc["ix"].clone(), doc["iz"].clone()), (Some(3), tile[0].clone(), tile[1].clone()));
        assert!(!doc["numeric"].as_array().unwrap().is_empty(), "{key} holds pieces");
    }
    assert!(items.iter().any(|item| item["ref"] == format!("{PACK}/clamp.json")));

    // The report is the baker's summary, and agrees with the pack.
    let report = &roads["report"];
    assert_eq!(report["place"], "site-a");
    assert!(report["streets"].as_u64().unwrap() >= 3, "{report}");
    assert_eq!(report["tiles"].as_u64(), Some(tiles.len() as u64));

    let trace = out.trace.join(" ");
    assert!(trace.contains(&format!("files={}", items.len())) && trace.contains("bake_s="), "{trace}");
    eprintln!("world.roads.bake test place: {} files, {} tiles, {total} bytes in the pack, {elapsed:.2} s through the node", items.len(), tiles.len());
}

#[tokio::test]
async fn a_missing_required_file_is_named() {
    let p = platform();
    put_seed(&p, SEED);
    for gone in ["site-a.ground.f32", "site-a.ground.json", "site-a.geojson"] {
        let s = store(&p);
        let name = format!("{SEED}/{gone}");
        let bytes = s.read_capped(&format!("{SEED}/{gone}"), "T").unwrap();
        s.fs.delete(&format!("{SEED}/{gone}")).unwrap();
        let err = run(&p, config(), json!({})).await.unwrap_err();
        assert_eq!(err.code, "FW_NODE_WORLD_ROADS_BAKE_SOURCE", "{gone}");
        assert!(err.message.contains(&name) && err.message.contains("missing"), "{}", err.message);
        s.fs.put(&format!("{SEED}/{gone}"), &bytes).unwrap();
    }
    // Nothing was written by a refused bake, and an absent folder is named too.
    assert!(store(&p).fs.head(&format!("{PACK}/index.json")).is_err());
    let err = run(&p, json!({ "from": "world/none", "place": "site-a" }), json!({})).await.unwrap_err();
    assert!(err.message.contains("world/none/site-a.geojson"), "{}", err.message);
    for bad in [json!({}), json!({ "from": "" }), json!({ "from": 7 }), json!({ "from": { "name": "x" } })] {
        let mut config = bad.clone();
        config["place"] = json!("site-a");
        assert_eq!(run(&p, config, json!({})).await.unwrap_err().code, "FW_NODE_WORLD_ROADS_BAKE_SOURCE", "{bad}");
    }
}

#[tokio::test]
async fn a_place_is_a_plain_slug() {
    let p = platform();
    put_seed(&p, SEED);
    let long = "a".repeat(65);
    for bad in ["", "  ", "../site-a", "site/a", "site a", "site.a", "Site\u{e9}", long.as_str()] {
        let err = run(&p, json!({ "from": SEED, "place": bad }), json!({})).await.unwrap_err();
        assert_eq!(err.code, CONFIG_CODE, "{bad:?}");
        assert!(err.message.contains("--place"), "{}", err.message);
    }
    assert_eq!(run(&p, json!({ "from": SEED, "place": "site-a", "folder": "../out" }), json!({})).await.unwrap_err().code, CONFIG_CODE);
    assert_eq!(run(&p, json!({ "from": SEED, "place": "site-a", "folder": "/out" }), json!({})).await.unwrap_err().code, CONFIG_CODE);
    assert_eq!(run(&p, json!({ "from": "../seed", "place": "site-a" }), json!({})).await.unwrap_err().code, "FW_NODE_WORLD_ROADS_BAKE_SOURCE");
    assert_eq!(run(&p, json!({ "from": SEED, "place": "site-a", "on_conflict": "replace" }), json!({})).await.unwrap_err().code, CONFIG_CODE);
}

#[tokio::test]
async fn a_folder_that_is_not_empty_follows_on_conflict() {
    let p = platform();
    put_seed(&p, SEED);
    // The default folder is world/roads/<place>; a tree answer names the seed folder too.
    let first = run(&p, json!({ "from": { "folder": SEED }, "place": "site-a" }), json!({})).await.unwrap();
    assert_eq!(first["roads"]["folder"], "world/roads/site-a");
    let count = first["roads"]["count"].as_u64().unwrap();
    assert!(count > 0);

    let again = run(&p, json!({ "from": SEED, "place": "site-a" }), json!({})).await.unwrap_err();
    assert!(again.message.contains("not empty"), "{}", again.message);

    let skipped = run(&p, json!({ "from": SEED, "place": "site-a", "on_conflict": "skip" }), json!({ "keep": 1 })).await.unwrap();
    assert_eq!(skipped["keep"], 1);
    assert_eq!((skipped["roads"]["count"].as_u64(), skipped["roads"]["items"].as_array().map(Vec::len)), (Some(0), Some(0)));
    assert_eq!(skipped["roads"]["index"]["ref"], "world/roads/site-a/index.json");
    assert!(skipped["roads"]["report"].is_null());

    let over = run(&p, json!({ "from": SEED, "place": "site-a", "on_conflict": "overwrite" }), json!({})).await.unwrap();
    assert_eq!(over["roads"]["count"].as_u64(), Some(count));
}

#[tokio::test]
async fn rules_override_the_baker_field_by_field() {
    let p = platform();
    put_seed(&p, SEED);
    let s = store(&p);
    s.fs.put("world/rules/wide.json", br#"{ "verge_width": [0.0, 6.0, 6.0, 6.0, 5.5, 6.5, 3.5, 0.0], "kerb_height": 0.12 }"#).unwrap();
    let ok = run(&p, json!({ "from": SEED, "place": "site-a", "folder": "world/wide", "rules": "world/rules/wide.json" }), json!({})).await.unwrap();
    assert!(ok["roads"]["count"].as_u64().unwrap() > 0);
    // The wider verge is in the design: the same seed, a different verge area (a local street's carriageway is fixed).
    let plain = run(&p, json!({ "from": SEED, "place": "site-a", "folder": "world/plain" }), json!({})).await.unwrap();
    assert_ne!(ok["roads"]["report"]["mesh"]["verge_m2"], plain["roads"]["report"]["mesh"]["verge_m2"]);

    s.fs.put("world/rules/bad.json", br#"{ "kerb_height": "tall" }"#).unwrap();
    s.fs.put("world/rules/not-json.json", b"lane = 3").unwrap();
    for bad in ["world/rules/bad.json", "world/rules/not-json.json"] {
        let err = run(&p, json!({ "from": SEED, "place": "site-a", "folder": "world/never", "rules": bad }), json!({})).await.unwrap_err();
        assert_eq!(err.code, "FW_NODE_WORLD_ROADS_BAKE_SEED", "{bad}");
        assert!(err.message.contains(bad) && err.message.contains("rules"), "{}", err.message);
    }
    let missing = run(&p, json!({ "from": SEED, "place": "site-a", "folder": "world/never", "rules": "world/rules/none.json" }), json!({})).await.unwrap_err();
    assert_eq!(missing.code, "FW_NODE_WORLD_ROADS_BAKE_SOURCE");
    assert!(store(&p).fs.head("world/never/index.json").is_err());
}

/// A seed whose ground header is wrong: refused under the seed code, naming the header file.
async fn refused(p: &Arc<PlatformService>, what: &str) -> String {
    let err = run(p, config(), json!({})).await.unwrap_err();
    assert_eq!(err.code, "FW_NODE_WORLD_ROADS_BAKE_SEED", "{what}: {}", err.message);
    assert!(err.message.contains("site-a.ground.json"), "{}", err.message);
    err.message
}

#[tokio::test]
async fn a_ground_the_baker_cannot_cover_is_refused_before_it_runs() {
    let p = platform();
    put_seed(&p, SEED);
    let s = store(&p);
    let header = format!("{SEED}/site-a.ground.json");
    s.fs.put(&header, ground_json(json!(0), json!(STEP)).as_bytes()).unwrap();
    refused(&p, "no cells").await;
    s.fs.put(&header, ground_json(json!(NX), json!(1.0e9)).as_bytes()).unwrap();
    assert!(refused(&p, "a grid across the planet").await.contains("in parts"));
    s.fs.put(&header, ground_json(json!(NX), json!(-4.0)).as_bytes()).unwrap();
    refused(&p, "a step below nought").await;
    s.fs.put(&header, ground_json(json!(NX * 2), json!(STEP)).as_bytes()).unwrap();
    assert!(refused(&p, "a grid the file does not fill").await.contains("holds"));
    s.fs.put(&header, b"{ not json").unwrap();
    refused(&p, "not json").await;
}

#[test]
fn the_signature_takes_from_and_place() {
    assert_eq!(
        crate::pipeline::nodes::node_signature(&definition()),
        "world.roads.bake --from TEXT --place TEXT [--rules JSON] [--store TEXT] [--folder TEXT] [--on-conflict error|skip|overwrite] → roads"
    );
}

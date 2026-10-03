use std::sync::Arc;

use serde_json::{Value, json};

use super::*;
use crate::pipeline::nodes::shared::project_store::open_store;

fn platform() -> crate::pipeline::nodes::shared::test_platform::TestPlatform {
    crate::pipeline::nodes::shared::test_platform::test_platform()
}

async fn run(platform: &Arc<PlatformService>, config: Value, payload: Value) -> Result<Value, PipelineError> {
    let config: Config = serde_json::from_value(config).expect("config");
    let node = Node::new(config, platform.clone())?;
    let out = node
        .execute_async(NodeExecutionInput {
            node_id: "n0".to_string(),
            input_pin: "in".to_string(),
            payload,
            metadata: json!({ "owner": "demo", "project": "demo", "pipeline": "test", "request_id": "r1" }),
            bus: None,
        })
        .await?;
    Ok(out.payload)
}

fn stored(platform: &Arc<PlatformService>, key: &str) -> Vec<u8> {
    open_store(platform, "demo", "demo", None).unwrap().read_capped(key, "T").unwrap()
}

#[test]
fn the_signature_is_one_kind_with_a_symbology() {
    assert_eq!(
        crate::pipeline::nodes::node_signature(&definition()),
        "fs.barcode.render --text TEXT [--symbology qr|code128] [--format svg|png] [--width N] [--height N] [--margin N] \
         [--color TEXT] [--background TEXT] [--ecc L|M|Q|H] [--store TEXT] [--folder TEXT] [--filename TEXT] [--path TEXT] \
         [--on-conflict error|skip|overwrite] → barcode"
    );
}

/// The answer is one key, `barcode`: a contract FileRef with what was drawn,
/// beside the payload it was given.
#[tokio::test]
async fn a_qr_code_answers_barcode_and_keeps_the_payload() {
    let p = platform();
    let input = json!({ "number": "CERT-2026-0412" });
    let out = run(&p, json!({ "text": "https://example.com/c/CERT-2026-0412", "folder": "certificates/qr", "filename": "CERT-2026-0412" }), input).await.unwrap();
    let mut keys: Vec<&str> = out.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, vec!["barcode", "number"]);
    let barcode = &out["barcode"];
    crate::pipeline::nodes::shared::file_ref::validate_file_ref(barcode).expect("a contract FileRef");
    assert_eq!(barcode["ref"], "certificates/qr/CERT-2026-0412.svg");
    assert_eq!((barcode["origin"].as_str(), barcode["trust"].as_str(), barcode["mime"].as_str()), (Some(NODE_KIND), Some("generated"), Some("image/svg+xml")));
    assert_eq!((barcode["symbology"].as_str(), barcode["format"].as_str()), (Some("qr"), Some("svg")));
    assert_eq!(barcode["width"], barcode["height"], "a QR Code is square");
    assert!(barcode["width"].as_u64().unwrap() <= 256);
    assert!(barcode.get("svg").is_none() && barcode.get("url").is_none());
    assert!(stored(&p, "certificates/qr/CERT-2026-0412.svg").starts_with(b"<svg"));
}

/// The same node draws Code 128; a number given as `--text` is its digits,
/// and `--height` is the bar height inside the quiet zone.
#[tokio::test]
async fn code128_is_the_same_node_with_a_bar_height() {
    let p = platform();
    let out = run(&p, json!({ "symbology": "code128", "text": 12345678, "format": "png", "height": 50, "margin": 0, "width": 300 }), json!({})).await.unwrap();
    let barcode = &out["barcode"];
    assert_eq!((barcode["symbology"].as_str(), barcode["format"].as_str(), barcode["mime"].as_str()), (Some("code128"), Some("png"), Some("image/png")));
    assert_eq!(barcode["height"], 50);
    let key = barcode["ref"].as_str().unwrap();
    assert!(key.starts_with("barcodes/") && key.ends_with(".png"), "{key}");
    assert!(stored(&p, key).starts_with(b"\x89PNG"));
    let text = run(&p, json!({ "symbology": "code128", "text": "T-000481" }), json!({})).await.unwrap();
    assert_eq!(text["barcode"]["format"], "svg");
}

#[tokio::test]
async fn a_flag_the_symbology_does_not_take_is_refused() {
    let p = platform();
    let ecc = run(&p, json!({ "symbology": "code128", "text": "T-1", "ecc": "H" }), json!({})).await.unwrap_err();
    assert_eq!(ecc.code, CONFIG_CODE);
    assert!(ecc.message.contains("--ecc"), "{}", ecc.message);
    let height = run(&p, json!({ "text": "hello", "height": 80 }), json!({})).await.unwrap_err();
    assert_eq!(height.code, CONFIG_CODE);
    assert!(height.message.contains("--height"), "{}", height.message);
    assert!(run(&p, json!({ "text": "hello", "ecc": "H" }), json!({})).await.is_ok());
}

#[tokio::test]
async fn every_choice_is_closed() {
    let p = platform();
    let symbology = run(&p, json!({ "text": "x", "symbology": "ean13" }), json!({})).await.unwrap_err();
    assert_eq!(symbology.code, CONFIG_CODE);
    assert!(symbology.message.contains("qr, code128"), "{}", symbology.message);
    for config in [json!({ "text": "x", "format": "jpg" }), json!({ "text": "x", "ecc": "low" }), json!({ "text": "x", "color": "red" }), json!({ "text": "x", "width": "wide" }), json!({ "text": "x", "margin": 17 })] {
        assert_eq!(run(&p, config.clone(), json!({})).await.unwrap_err().code, CONFIG_CODE, "{config}");
    }
}

#[tokio::test]
async fn the_text_must_be_something_the_code_can_carry() {
    let p = platform();
    assert_eq!(run(&p, json!({ "text": "" }), json!({})).await.unwrap_err().code, TEXT_CODE);
    let ascii = run(&p, json!({ "symbology": "code128", "text": "Café" }), json!({})).await.unwrap_err();
    assert_eq!(ascii.code, TEXT_CODE);
    assert!(ascii.message.contains("'é'"), "{}", ascii.message);
    assert_eq!(run(&p, json!({ "text": "x".repeat(3000) }), json!({})).await.unwrap_err().code, TEXT_CODE);
    // Each Code 128 symbol is 11 modules: 800 letters cannot fit 8192 px.
    assert_eq!(run(&p, json!({ "symbology": "code128", "text": "A".repeat(800) }), json!({})).await.unwrap_err().code, SIZE_CODE);
}

#[tokio::test]
async fn a_named_file_that_exists_is_an_error_unless_on_conflict_says_otherwise() {
    let p = platform();
    let config = json!({ "text": "a", "path": "codes/a.svg" });
    run(&p, config.clone(), json!({})).await.unwrap();
    assert_eq!(run(&p, config.clone(), json!({})).await.unwrap_err().code, CODE);
    let skipped = run(&p, json!({ "text": "b", "path": "codes/a.svg", "on_conflict": "skip" }), json!({})).await.unwrap();
    assert_eq!(skipped["barcode"]["ref"], "codes/a.svg");
    run(&p, json!({ "text": "b", "path": "codes/a.svg", "on_conflict": "overwrite" }), json!({})).await.unwrap();
}

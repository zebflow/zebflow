//! A write that must not overwrite a change it has not seen (`expect_hash`
//! with `file_stat`), and a page checked without saving (`template_check`),
//! as an agent reaches them over MCP.

use std::sync::Arc;

use serde_json::{Value, json};
use zebflow::platform::{PlatformConfig, PlatformService};

use super::mcp_delete::{TestDir, login, mcp_call, mcp_token};

fn answer(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| panic!("json answer: {text}"))
}

#[tokio::test]
async fn a_write_names_the_version_it_read_and_a_page_is_checked_unsaved() {
    let root = TestDir(std::env::temp_dir().join(format!(
        "zebflow-platform-mcp-versions-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    )));
    let mut config = PlatformConfig::default();
    config.data_root = root.0.clone();
    config.default_password = "test-pass".to_string();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform"));
    let app = zebflow::platform::web::router(platform.clone()).await;
    let cookie = login(&app).await;
    let token = mcp_token(&app, &cookie, &["templates.read", "templates.write", "pipelines.read", "pipelines.write"]).await;
    let card = "components/card.tsx";

    let missing = answer(&mcp_call(&app, &token, "file_stat", json!({ "rel_path": card })).await.expect("stat"));
    assert_eq!(missing["exists"], false);

    let body = "export default function Card() {\n  return <div>card</div>;\n}\n";
    mcp_call(&app, &token, "file_write", json!({ "rel_path": card, "content": body, "expect_hash": "none" })).await.expect("a new file");
    let again = mcp_call(&app, &token, "file_write", json!({ "rel_path": card, "content": body, "expect_hash": "none" })).await;
    assert!(again.as_ref().is_err_and(|e| e.contains("changed since")), "a file that exists is not new: {again:?}");

    let first = answer(&mcp_call(&app, &token, "file_stat", json!({ "rel_path": card })).await.expect("stat"));
    let read = first["hash"].as_str().expect("hash").to_string();
    assert!(read.starts_with("sha256:"), "{first}");
    mcp_call(&app, &token, "file_edit", json!({ "rel_path": card, "old_string": "card</div>", "new_string": "a card</div>", "expect_hash": read }))
        .await
        .expect("the version it read");
    let stale = mcp_call(&app, &token, "file_write", json!({ "rel_path": card, "content": body, "expect_hash": read })).await;
    let stale = stale.expect_err("the version it read is gone");
    assert!(stale.contains("changed since") && stale.contains("it is now sha256:"), "{stale}");
    let kept = mcp_call(&app, &token, "file_read", json!({ "rel_path": card })).await.expect("read");
    assert!(kept.contains("a card"), "the refused write wrote nothing: {kept}");

    let good = "import Card from \"@/components/card\";\nexport default function Page() {\n  return <main><Card /></main>;\n}\n";
    let checked = answer(&mcp_call(&app, &token, "template_check", json!({ "rel_path": "pages/good.tsx", "content": good })).await.expect("check"));
    assert_eq!(checked["ok"], true, "{checked}");
    let broken = good.replace("@/components/card", "@/components/missing");
    let checked = answer(&mcp_call(&app, &token, "template_check", json!({ "rel_path": "pages/broken.tsx", "content": broken })).await.expect("check"));
    assert_eq!(checked["ok"], false, "{checked}");
    let missing_page = mcp_call(&app, &token, "file_stat", json!({ "rel_path": "pages/good.tsx" })).await.expect("stat");
    assert_eq!(answer(&missing_page)["exists"], false, "a check saves nothing");

    // A component's private name may match a name another file in its tree
    // exports: a page inlines the component and renames its private names.
    let helper = "export async function signOut() { return true; }\nexport function label() { return \"x\"; }\n";
    let shell = "import { label } from \"@/scripts/session-helper\";\nasync function signOut() { return false; }\nexport default function Shell() {\n  return <button onClick={() => signOut()}>{label()}</button>;\n}\n";
    mcp_call(&app, &token, "file_write", json!({ "rel_path": "scripts/session-helper.ts", "content": helper })).await.expect("helper");
    mcp_call(&app, &token, "file_write", json!({ "rel_path": "components/shell.tsx", "content": shell })).await.expect("shell");
    let checked = answer(&mcp_call(&app, &token, "template_check", json!({ "rel_path": "components/shell.tsx" })).await.expect("check"));
    assert_eq!(checked["ok"], true, "a component's private name is not the page's: {checked}");
    let as_page = answer(&mcp_call(&app, &token, "template_check", json!({ "rel_path": "pages/shell.tsx", "content": shell })).await.expect("check"));
    assert_eq!(as_page["ok"], false, "a page's own names are not renamed, so there the clash is real: {as_page}");
    assert!(as_page.to_string().contains("RWE_BUNDLE_NAME_COLLISION") || as_page.to_string().contains("declared by both"), "{as_page}");
    let unsaved = shell.replace("@/scripts/session-helper", "@/scripts/nowhere");
    let checked = answer(&mcp_call(&app, &token, "template_check", json!({ "rel_path": "components/shell.tsx", "content": unsaved })).await.expect("check"));
    assert_eq!(checked["ok"], false, "the unsaved content is what is checked: {checked}");

    let pipeline = json!({ "file_rel_path": "pipelines/tests/versioned", "body": "| trigger.webhook --route /versioned --method GET | web.response.send --body ok", "expect_hash": "none" });
    mcp_call(&app, &token, "pipeline_register", pipeline.clone()).await.expect("a new pipeline");
    let twice = mcp_call(&app, &token, "pipeline_register", pipeline).await;
    assert!(
        twice.as_ref().is_err_and(|e| e.contains("changed since")) || twice.as_ref().is_ok_and(|t| t.contains("changed since")),
        "a pipeline that exists is not new: {twice:?}"
    );
}

use std::sync::Arc;

use serde_json::{Value, json};

use super::*;

fn platform() -> crate::pipeline::nodes::shared::test_platform::TestPlatform {
    crate::pipeline::nodes::shared::test_platform::test_platform()
}

fn store(platform: &Arc<PlatformService>) -> NodeStore {
    open_store(platform, "demo", "demo", None).expect("store")
}

async fn run(platform: &Arc<PlatformService>, kind: &str, config: Value, payload: Value) -> Result<Value, PipelineError> {
    let config: Config = serde_json::from_value(config).expect("config");
    let node = Node::new(config, platform.clone(), Operation::for_kind(kind).expect("a kind here"))?;
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

/// The payload's keys, sorted: the answer is one key beside what came in.
fn keys(payload: &Value) -> Vec<String> {
    let mut keys: Vec<String> = payload.as_object().expect("an object").keys().cloned().collect();
    keys.sort();
    keys
}

#[tokio::test]
async fn every_file_node_needs_from() {
    let p = platform();
    for kind in [GET_NODE_KIND, HEAD_NODE_KIND, DELETE_NODE_KIND, COPY_NODE_KIND, MOVE_NODE_KIND] {
        let op = Operation::for_kind(kind).unwrap();
        let none = run(&p, kind, json!({ "folder": "x" }), json!({})).await.unwrap_err();
        assert_eq!(none.code, op.source_code(), "{kind}");
        assert!(none.message.contains("--from is required"), "{kind}: {}", none.message);
        let list = run(&p, kind, json!({ "from": ["a.txt"] }), json!({})).await.unwrap_err();
        assert_eq!(list.code, op.source_code(), "{kind}");
        assert_eq!(run(&p, kind, json!({ "from": { "name": "x" } }), json!({})).await.unwrap_err().code, op.source_code());
        assert_eq!(run(&p, kind, json!({ "from": "../x.txt" }), json!({})).await.unwrap_err().code, op.source_code());
    }
}

#[tokio::test]
async fn get_and_head_answer_file_and_keep_the_payload() {
    let p = platform();
    store(&p).fs.put("docs/notes.md", b"# Notes").unwrap();
    let out = run(&p, GET_NODE_KIND, json!({ "from": "docs/notes.md" }), json!({ "keep": 1 })).await.unwrap();
    assert_eq!(keys(&out), vec!["file", "keep"]);
    assert_eq!((out["file"]["path"].as_str(), out["file"]["content"].as_str()), (Some("docs/notes.md"), Some("# Notes")));
    assert!(out["file"]["base64"].is_null());

    let out = run(&p, GET_NODE_KIND, json!({ "from": "docs/notes.md", "encoding": "base64" }), json!({})).await.unwrap();
    assert_eq!(out["file"]["base64"], "IyBOb3Rlcw==");
    let bad = run(&p, GET_NODE_KIND, json!({ "from": "docs/notes.md", "encoding": "hex" }), json!({})).await.unwrap_err();
    assert_eq!(bad.code, "FW_NODE_FS_FILE_GET_CONFIG");

    store(&p).fs.put("bin/a.bin", &[0xff, 0xfe]).unwrap();
    assert_eq!(run(&p, GET_NODE_KIND, json!({ "from": "bin/a.bin" }), json!({})).await.unwrap_err().code, GET_UTF8_CODE);

    // A FileRef is read from the store it names.
    let file = store(&p).file_ref("docs/notes.md", "notes.md", "text/markdown", b"# Notes", "fs.file.put", "generated");
    let out = run(&p, HEAD_NODE_KIND, json!({ "from": file }), json!({ "file": file })).await.unwrap();
    assert_eq!(keys(&out), vec!["file"]);
    assert_eq!((out["file"]["path"].as_str(), out["file"]["size"].as_u64()), (Some("docs/notes.md"), Some(7)));
    assert_eq!(run(&p, HEAD_NODE_KIND, json!({ "from": "docs/none.md" }), json!({})).await.unwrap_err().code, "FW_NODE_FS_FILE_HEAD");
}

#[tokio::test]
async fn delete_answers_the_ref_it_removed() {
    let p = platform();
    store(&p).fs.put("uploads/a.txt", b"a").unwrap();
    let out = run(&p, DELETE_NODE_KIND, json!({ "from": "uploads/a.txt" }), json!({ "keep": true })).await.unwrap();
    assert_eq!(keys(&out), vec!["file", "keep"]);
    assert_eq!(out["file"], json!({ "ref": "uploads/a.txt", "deleted": true }));
    assert!(store(&p).fs.head("uploads/a.txt").is_err());
}

#[tokio::test]
async fn a_folder_is_deleted_only_with_recursive() {
    let p = platform();
    store(&p).fs.put("exports/2026/a.csv", b"a").unwrap();
    store(&p).fs.put("exports/2026/b.csv", b"b").unwrap();
    let refused = run(&p, DELETE_NODE_KIND, json!({ "from": "exports/2026" }), json!({})).await.unwrap_err();
    assert_eq!(refused.code, DELETE_FOLDER_CODE);
    assert!(refused.message.contains("--recursive"), "{}", refused.message);
    assert!(store(&p).fs.head("exports/2026/a.csv").is_ok(), "nothing was removed");

    // A FileRef names one file, whatever the switch says.
    let mut file = store(&p).stored_ref("exports/2026/a.csv", "fs.file.put", "generated", "T").unwrap();
    file["ref"] = json!("exports/2026");
    let by_ref = run(&p, DELETE_NODE_KIND, json!({ "from": file, "recursive": true }), json!({})).await.unwrap_err();
    assert_eq!(by_ref.code, DELETE_FOLDER_CODE);

    // A file is a file with or without the switch.
    let one = run(&p, DELETE_NODE_KIND, json!({ "from": "exports/2026/b.csv", "recursive": true }), json!({})).await.unwrap();
    assert_eq!(one["file"], json!({ "ref": "exports/2026/b.csv", "deleted": true }));
    assert!(store(&p).fs.head("exports/2026/a.csv").is_ok());

    let out = run(&p, DELETE_NODE_KIND, json!({ "from": "exports/2026", "recursive": true }), json!({ "keep": 1 })).await.unwrap();
    assert_eq!(keys(&out), vec!["file", "keep"]);
    assert_eq!(out["file"], json!({ "ref": "exports/2026", "deleted": true, "recursive": true }));
    assert!(store(&p).fs.head("exports/2026").is_err());
}

#[tokio::test]
async fn copy_and_move_answer_the_stored_file() {
    let p = platform();
    store(&p).fs.put("inbox/a.csv", b"a,b\n").unwrap();
    let input = json!({ "body": { "note": "x" } });
    let out = run(&p, COPY_NODE_KIND, json!({ "from": "inbox/a.csv", "folder": "copies" }), input.clone()).await.unwrap();
    assert_eq!(keys(&out), vec!["body", "file"]);
    crate::pipeline::nodes::shared::file_ref::validate_file_ref(&out["file"]).expect("a contract FileRef");
    assert_eq!((out["file"]["ref"].as_str(), out["file"]["origin"].as_str()), (Some("copies/a.csv"), Some(COPY_NODE_KIND)));

    // A destination that exists is an error unless told; a skip answers it as it is.
    let again = run(&p, COPY_NODE_KIND, json!({ "from": "inbox/a.csv", "folder": "copies" }), json!({})).await.unwrap_err();
    assert!(again.message.contains("already exists"), "{}", again.message);
    let moved = run(&p, MOVE_NODE_KIND, json!({ "from": "inbox/a.csv", "path": "copies/a.csv", "on_conflict": "skip" }), json!({})).await.unwrap();
    assert_eq!(moved["file"]["origin"], MOVE_NODE_KIND);
    assert!(store(&p).fs.head("inbox/a.csv").is_ok(), "a skipped move keeps its source");

    let file = store(&p).stored_ref("inbox/a.csv", "fs.file.put", "generated", "T").unwrap();
    let out = run(&p, MOVE_NODE_KIND, json!({ "from": file, "folder": "archive" }), input).await.unwrap();
    assert_eq!((out["file"]["ref"].as_str(), out["body"]["note"].as_str()), (Some("archive/a.csv"), Some("x")));
    assert!(store(&p).fs.head("inbox/a.csv").is_err());
    let same = run(&p, COPY_NODE_KIND, json!({ "from": "archive/a.csv" }), json!({})).await.unwrap_err();
    assert_eq!(same.code, "FW_NODE_FS_FILE_COPY_CONFIG");
}

#[tokio::test]
async fn folder_list_reads_from_and_answers_folder() {
    let p = platform();
    store(&p).fs.put("uploads/a.jpg", b"a").unwrap();
    store(&p).fs.put("uploads/b.jpg", b"b").unwrap();
    for from in [json!(null), json!(""), json!(1)] {
        let err = run(&p, LIST_NODE_KIND, json!({ "from": from }), json!({})).await.unwrap_err();
        assert_eq!(err.code, "FW_NODE_FS_FOLDER_LIST_SOURCE");
    }
    let out = run(&p, LIST_NODE_KIND, json!({ "from": "uploads" }), json!({ "keep": true })).await.unwrap();
    assert_eq!(keys(&out), vec!["folder", "keep"]);
    assert_eq!((out["folder"]["path"].as_str(), out["folder"]["count"].as_u64()), (Some("uploads"), Some(2)));
    assert_eq!(out["folder"]["items"][0]["path"], "uploads/a.jpg");
    let root = run(&p, LIST_NODE_KIND, json!({ "from": "/" }), json!({})).await.unwrap();
    assert_eq!(root["folder"]["path"], "");
    assert!(root["folder"]["items"].as_array().unwrap().iter().any(|item| item["path"] == "uploads"));
}

#[tokio::test]
async fn folder_create_takes_folder_and_says_whether_it_made_it() {
    let p = platform();
    let none = run(&p, MKDIR_NODE_KIND, json!({ "from": "uploads/2026" }), json!({})).await.unwrap_err();
    assert_eq!(none.code, "FW_NODE_FS_FOLDER_CREATE_CONFIG");
    let out = run(&p, MKDIR_NODE_KIND, json!({ "folder": "uploads/2026" }), json!({ "keep": true })).await.unwrap();
    assert_eq!(keys(&out), vec!["folder", "keep"]);
    assert_eq!(out["folder"], json!({ "path": "uploads/2026", "created": true }));
    let again = run(&p, MKDIR_NODE_KIND, json!({ "folder": "uploads/2026" }), json!({})).await.unwrap();
    assert_eq!(again["folder"]["created"], false);
    assert_eq!(run(&p, MKDIR_NODE_KIND, json!({ "folder": "../up" }), json!({})).await.unwrap_err().code, "FW_NODE_FS_FOLDER_CREATE_CONFIG");
}

#[test]
fn the_signatures_name_from_and_the_answer() {
    let sig = |def: NodeDefinition| crate::pipeline::nodes::node_signature(&def);
    assert_eq!(sig(get_definition()), "fs.file.get --from FILE [--encoding text|base64] [--store TEXT] → file");
    assert_eq!(sig(head_definition()), "fs.file.head --from FILE [--store TEXT] → file");
    assert_eq!(sig(delete_definition()), "fs.file.delete --from FILE [--recursive] [--store TEXT] → file");
    assert_eq!(
        sig(copy_definition()),
        "fs.file.copy --from FILE [--store TEXT] [--folder TEXT] [--filename TEXT] [--path TEXT] [--on-conflict error|skip|overwrite] → file"
    );
    assert_eq!(sig(list_definition()), "fs.folder.list --from TEXT [--store TEXT] → folder");
    assert_eq!(sig(mkdir_definition()), "fs.folder.create --folder TEXT [--store TEXT] → folder");
}

//! `trigger.repo` — run when files under a path of the project's repository
//! change.
//!
//! A **passthrough trigger**, like `trigger.topic`: the watching happens in
//! [`crate::infra::repo_watch::RepoWatcher`], which listens to the change
//! record from the moment the pipeline is activated. It waits for a quiet
//! moment, so a burst of saves is one run, and answers one key, `repo`:
//!
//! ```json
//! {
//!   "repo": {
//!     "path": "docs",
//!     "changes": [{ "action": "edit", "path": "docs/start.md", "person": "superadmin", "via": "studio" }],
//!     "node_id": "n0"
//!   }
//! }
//! ```
//!
//! A change the pipeline itself made never starts it again.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::change_log::RepoChange;

pub const NODE_KIND: &str = "trigger.repo";
const OUTPUT_PIN_OUT: &str = "out";
/// The key this trigger answers under.
pub const ANSWER_KEY: &str = "repo";

/// How long the repository must be quiet before a run starts.
pub const QUIET_MS: u64 = 2_000;
/// The longest a run waits for quiet while changes keep coming.
pub const MAX_WAIT_MS: u64 = 30_000;
/// Changes listed in one run's answer; the rest are counted.
pub const MAX_CHANGES: usize = 500;

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Repository Trigger".to_string(),
        description: "Runs when files under `--path` of the project's repository change (created, edited, moved or deleted, by a person in \
            Zebflow Studio, an MCP agent, a git sync or an import), once the pipeline is active. It waits until the repository has been quiet for 2 seconds, so \
            a burst of saves is one run (at most 30 seconds of waiting). `--path` is a folder or a file, repository-relative (`docs`, \
            `content/blog`, `data/catalog.csv`); `/` is the whole repository; `docs` does not match `docs-old`. Answers one key, `repo`: \
            `{ path, changes: [{ action, path, to_path?, person, via }], dropped, node_id }` (`input.repo.changes`). A change the pipeline \
            itself made never starts it again. Typical use: `| trigger.repo --path docs | web.site.generate --from docs …` rebuilds a docs \
            site on every edit. Runs show under `pipeline_get_invocations` with trigger `trigger.repo`."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "repo": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "The `--path` this trigger watches, without slashes at either end." },
                        "changes": { "type": "array", "items": { "type": "object", "properties": {
                            "action": { "type": "string", "description": "`create`, `edit`, `delete`, `move`, or a step such as `git-sync`." },
                            "path": { "type": "string" },
                            "to_path": { "type": "string" },
                            "person": { "type": "string" },
                            "via": { "type": "string" }
                        } } },
                        "dropped": { "type": "integer", "description": "Changes beyond the 500 listed." },
                        "node_id": { "type": "string" }
                    }
                }
            }
        }),
        input_pins: vec![],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": { "type": "string", "description": "The repository folder or file to watch; `/` is the whole repository." },
            }
        }),
        dsl_flags: vec![DslFlag {
            flag: "--path".to_string(),
            config_key: "path".to_string(),
            description: "The repository folder or file to watch (`docs`, `data/catalog.csv`); `/` is the whole repository.".to_string(),
            kind: DslFlagKind::Scalar,
            required: true,
            value: "text".to_string(),
            ..Default::default()
        }],
        fields: vec![NodeFieldDef {
            name: "path".to_string(),
            label: "Path".to_string(),
            field_type: NodeFieldType::Text,
            help: Some("The repository folder or file to watch; / is the whole repository.".to_string()),
            ..Default::default()
        }],
        layout: vec![],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Rebuild a docs site on every edit", "trigger.repo --path docs")
                .output(json!({ "repo": { "path": "docs", "changes": [{ "action": "edit", "path": "docs/start.md", "person": "superadmin", "via": "studio" }], "dropped": 0, "node_id": "n0" } }))
                .note("Then `| web.site.generate --from docs --folder sites/docs`."),
        ],
        ..Default::default()
    }
}

/// `--path` as the watcher compares it: repository-relative, no slashes at
/// either end, empty for the whole repository. Refused when it could name
/// something outside the repository.
pub fn watched_path(raw: &str) -> Result<String, String> {
    let path = raw.trim().replace('\\', "/");
    if path.is_empty() {
        return Err("--path is required: a repository folder or file, or / for the whole repository".to_string());
    }
    let path = path.trim_matches('/').to_string();
    if path.split('/').any(|part| part == ".." || part == "." || part.is_empty() && !path.is_empty()) || path.contains(':') {
        return Err(format!("--path '{raw}' must be a plain repository path such as docs or content/blog"));
    }
    Ok(path)
}

/// The envelope a run starts with: the watched path and every change in the
/// burst, the oldest first.
pub fn envelope(path: &str, node_id: &str, changes: &[RepoChange]) -> serde_json::Value {
    let listed: Vec<serde_json::Value> = changes
        .iter()
        .take(MAX_CHANGES)
        .map(|change| {
            let mut entry = json!({ "action": change.action, "path": change.path, "person": change.person, "via": change.via });
            if let Some(to) = &change.to_path {
                entry["to_path"] = json!(to);
            }
            entry
        })
        .collect();
    json!({
        "path": path,
        "changes": listed,
        "dropped": changes.len().saturating_sub(MAX_CHANGES),
        "node_id": node_id,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub path: String,
}

pub struct Node {
    config: Config,
}

impl Node {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        NODE_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: super::answer_under(ANSWER_KEY, input.payload),
            trace: vec![format!("trigger.repo: path={}", self.config.path)],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(action: &str, path: &str, to: Option<&str>) -> RepoChange {
        RepoChange {
            owner: "superadmin".into(),
            project: "site-a".into(),
            action: action.into(),
            path: path.into(),
            to_path: to.map(str::to_string),
            person: "superadmin".into(),
            via: "studio".into(),
            run: None,
        }
    }

    #[test]
    fn a_path_is_repository_relative_and_slash_is_everything() {
        assert_eq!(watched_path("docs/").unwrap(), "docs");
        assert_eq!(watched_path("/content/blog").unwrap(), "content/blog");
        assert_eq!(watched_path("/").unwrap(), "");
        for bad in ["", "  ", "../etc", "docs/../..", "docs//x", "C:/x", "./docs"] {
            assert!(watched_path(bad).is_err(), "{bad:?} is refused");
        }
    }

    #[test]
    fn a_folder_is_not_a_prefix_of_its_neighbour() {
        assert!(change("edit", "docs/a.md", None).touches("docs"));
        assert!(change("edit", "docs", None).touches("docs"));
        assert!(!change("edit", "docs-old/a.md", None).touches("docs"));
        assert!(change("move", "drafts/a.md", Some("docs/a.md")).touches("docs"), "moved in");
        assert!(change("move", "docs/a.md", Some("drafts/a.md")).touches("docs"), "moved out");
        assert!(change("git-sync", "", None).touches("docs"), "a step over the whole repository");
        assert!(change("edit", "pages/x.tsx", None).touches(""), "/ watches everything");
    }

    #[tokio::test]
    async fn the_answer_is_repo_with_every_change() {
        let changes = [change("edit", "docs/a.md", None), change("move", "docs/b.md", Some("docs/c.md"))];
        let out = Node::new(Config { path: "docs".into() })
            .execute_async(NodeExecutionInput {
                node_id: "t".into(),
                input_pin: String::new(),
                payload: envelope("docs", "t", &changes),
                metadata: json!({}),
                bus: None,
            })
            .await
            .expect("answer");
        assert_eq!(out.payload["repo"]["changes"][1], json!({ "action": "move", "path": "docs/b.md", "to_path": "docs/c.md", "person": "superadmin", "via": "studio" }));
        assert_eq!(out.payload["repo"]["path"], "docs");
        assert_eq!(out.payload["repo"]["dropped"], 0);
    }
}

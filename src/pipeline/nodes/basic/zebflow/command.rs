//! `zebflow.command.run` — runs Zebflow's own operations from a pipeline,
//! inside the fence a `zebflow_control` credential draws
//! (`kinds/credential` § zebflow_control).
//!
//! # Pipeline position
//!
//! After a webhook and its login check: a downstream app sends commands, this
//! node carries them out — writing files, registering and switching on
//! pipelines — and never outside the fence. The fence lives in the credential,
//! which only a maintainer in Studio makes or changes, so nothing this node
//! does can widen it.
//!
//! # DSL
//! ```text
//! | zebflow.command.run --credential zf_modules --command "{{ input.webhook.body }}" --as "{{ input.webhook.auth.sub }}"
//! ```
//!
//! # Input/output
//! - **`--command`:** one `{ tool, arguments }` or a list of them, in the
//!   shape the MCP tools take (`file_write`, `pipeline_register`, …).
//! - **Output `out`:** the payload plus `command: { ok, results }`, one result
//!   per command in order.
//! - **Failure:** a refused or failed command fails the node with its index,
//!   tool and reason; the commands before it stay done.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::model::{
    DslFlag, DslFlagKind, NodeCapability, NodeExample, NodeFieldDef, NodeFieldType, PipelineGraph,
};
use crate::pipeline::nodes::shared::util::{metadata_scope, with_answer};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;
use crate::platform::services::change_log::{self, ChangeActor};
use crate::platform::services::ops::{FileListOptions, OpsResult, PlatformOps};

pub const NODE_KIND: &str = "zebflow.command.run";
/// A command failed while it ran.
pub const CODE: &str = "FW_NODE_ZEBFLOW_COMMAND_RUN";
/// The node or its credential is set up wrong.
pub const CONFIG_CODE: &str = "FW_NODE_ZEBFLOW_COMMAND_RUN_CONFIG";
/// A command reached past the fence.
pub const REFUSED_CODE: &str = "FW_NODE_ZEBFLOW_COMMAND_RUN_REFUSED";
/// The credential kind that holds the fence.
pub const CREDENTIAL_KIND: &str = "zebflow_control";
/// The most commands one run carries.
const MAX_COMMANDS: usize = 50;

/// The operations a fence may open, by the names the MCP tools carry.
pub const TOOLS: &[&str] = &[
    "file_list",
    "file_read",
    "file_write",
    "file_edit",
    "file_delete",
    "file_stat",
    "template_check",
    "pipeline_get",
    "pipeline_register",
    "pipeline_activate",
    "pipeline_deactivate",
    "pipeline_delete",
    "credential_list",
    "credential_create",
    "credential_update",
    "credential_delete",
];

/// What a fence in deny mode refuses when it names nothing itself: the
/// project's shared stores and functions — its built-in databases, key-value
/// store, stored files, file tables and function pipelines — none of which has
/// a fence of its own. A made pipeline keeps its data in a delegated database.
pub const DEFAULT_DENY: &[&str] = &["sekejap.*", "sqlite.*", "kv.*", "fs.*", "table.*", "function.*"];

/// The family no made pipeline may use, whatever the fence says: a made
/// pipeline that could run commands would carry the fence's power past it.
const NEVER_FAMILY: &str = "zebflow";

/// Entry nodes a made pipeline may start with. Not `trigger.function`: a
/// function's name is project-wide, so a made one could stand in for one the
/// app's own pipelines call.
const ALLOWED_TRIGGERS: &[&str] = &["trigger.webhook", "trigger.schedule", "trigger.manual", "trigger.repo"];

/// The extension of a pipeline file, which only the pipeline operations touch.
const PIPELINE_FILE: &str = ".zf.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// The `zebflow_control` credential that draws the fence.
    #[serde(default)]
    pub credential_id: String,
    /// One `{ tool, arguments }`, a list of them, or JSON text of either.
    #[serde(default)]
    pub command: Value,
    /// Who asked — recorded in the change record beside the credential.
    #[serde(default, rename = "as")]
    pub as_person: Value,
}

/// How a fence's `nodes` list reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodesMode {
    /// Every node except those listed (or [`DEFAULT_DENY`]).
    Deny,
    /// Only the nodes listed.
    Allow,
}

/// Whether a kind is named by a pattern: the kind itself, or `family.*`.
fn pattern_names(pattern: &str, kind: &str) -> bool {
    match pattern.strip_suffix(".*") {
        Some(family) => kind.split('.').next() == Some(family),
        None => pattern == kind,
    }
}

/// What a `zebflow_control` credential allows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fence {
    /// The fence credential's own id: what its door's credentials are marked
    /// as managed by.
    pub id: String,
    /// Repository folder, ending in `/`: files and pipelines live only here.
    pub scope: String,
    /// Route prefix, ending in `/`: a made webhook answers only under it.
    pub routes: String,
    /// The operations allowed, from [`TOOLS`].
    pub tools: Vec<String>,
    /// Credentials a made pipeline may use, by id. Their secrets never show.
    pub delegate: Vec<String>,
    /// Whether `nodes` lists what a made pipeline may not use, or the only
    /// nodes it may use.
    pub nodes_mode: NodesMode,
    /// Kinds (`kv.entry.get`) or families (`kv.*`); in deny mode an empty list
    /// means [`DEFAULT_DENY`].
    pub nodes: Vec<String>,
    /// Credential kinds the door may create; empty means none.
    pub manage_kinds: Vec<String>,
    /// How many credentials the door may manage at once; `None` is no limit.
    pub manage_limit: Option<u32>,
    /// How the change record names this fence.
    pub record_as: String,
    /// The credentials this door manages now, read when it runs; never from
    /// the secret.
    pub managed: Vec<String>,
}

fn config_error(message: impl Into<String>) -> PipelineError {
    PipelineError::new(CONFIG_CODE, message)
}

/// A repository-relative path with `.` and empty segments removed; `None`
/// when it is absolute or climbs out with `..`.
fn clean_path(raw: &str) -> Option<String> {
    let raw = raw.trim().replace('\\', "/");
    if raw.starts_with('/') {
        return None;
    }
    let mut parts = Vec::new();
    for segment in raw.split('/') {
        match segment {
            "" | "." => {}
            ".." => return None,
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

fn text_field(secret: &Value, key: &str) -> String {
    secret.get(key).and_then(Value::as_str).map(str::trim).unwrap_or_default().to_string()
}

/// A list field: an array of strings, or text split on commas and new lines.
fn list_field(secret: &Value, key: &str) -> Vec<String> {
    let items: Vec<String> = match secret.get(key) {
        Some(Value::Array(items)) => items.iter().filter_map(Value::as_str).map(str::to_string).collect(),
        Some(Value::String(text)) => text.split([',', '\n']).map(str::to_string).collect(),
        _ => Vec::new(),
    };
    items.into_iter().map(|item| item.trim().to_string()).filter(|item| !item.is_empty()).collect()
}

impl Fence {
    /// Reads a fence from a credential's secret, refusing one that does not
    /// draw a real boundary: no folder, the repository root, a route prefix
    /// that is the whole site, or an operation that does not exist.
    pub fn from_secret(credential_id: &str, secret: &Value) -> Result<Self, PipelineError> {
        let scope = clean_path(&text_field(secret, "scope"))
            .filter(|scope| !scope.is_empty())
            .ok_or_else(|| config_error(format!("credential '{credential_id}': `scope` must be a folder inside the repository, such as modules/")))?;
        let routes = text_field(secret, "routes");
        let routes = routes.trim_end_matches('/');
        if !routes.starts_with('/') || routes.len() < 2 {
            return Err(config_error(format!("credential '{credential_id}': `routes` must be a route prefix such as /m/, not the whole site")));
        }
        let tools = list_field(secret, "tools");
        if tools.is_empty() {
            return Err(config_error(format!("credential '{credential_id}': `tools` lists no operation")));
        }
        if let Some(unknown) = tools.iter().find(|tool| !TOOLS.contains(&tool.as_str())) {
            return Err(config_error(format!(
                "credential '{credential_id}': `{unknown}` is not an operation a fence can open ({})",
                TOOLS.join(", ")
            )));
        }
        let nodes_mode = match text_field(secret, "nodes_mode").as_str() {
            "" | "deny" => NodesMode::Deny,
            "allow" => NodesMode::Allow,
            other => return Err(config_error(format!("credential '{credential_id}': `nodes_mode` is deny or allow, not '{other}'"))),
        };
        let nodes = list_field(secret, "nodes");
        let catalogue = crate::pipeline::nodes::basic::builtin_node_definitions();
        if let Some(unknown) = nodes.iter().find(|pattern| !catalogue.iter().any(|def| pattern_names(pattern, &def.kind))) {
            return Err(config_error(format!("credential '{credential_id}': `{unknown}` names no node; write a kind (kv.entry.get) or a family (kv.*)")));
        }
        if nodes_mode == NodesMode::Allow && nodes.is_empty() {
            return Err(config_error(format!("credential '{credential_id}': `nodes_mode: allow` needs `nodes`, the nodes a made pipeline may use")));
        }
        let manage_kinds = list_field(secret, "manage_kinds");
        let kinds = crate::platform::services::credential::builtin_credential_types();
        if let Some(kind) = manage_kinds.iter().find(|kind| kind.as_str() == CREDENTIAL_KIND || !kinds.iter().any(|def| &def.kind == *kind)) {
            let why = if kind == CREDENTIAL_KIND { "a door may never create another fence" } else { "it is not a credential kind" };
            return Err(config_error(format!("credential '{credential_id}': `manage_kinds` names '{kind}'; {why}")));
        }
        if tools.iter().any(|tool| tool == "credential_create") && manage_kinds.is_empty() {
            return Err(config_error(format!("credential '{credential_id}': `credential_create` needs `manage_kinds`, the kinds the door may create")));
        }
        let manage_limit = match secret.get("manage_limit") {
            None | Some(Value::Null) => None,
            Some(Value::String(text)) if text.trim().is_empty() => None,
            Some(value) => Some(
                value
                    .as_u64()
                    .or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))
                    .filter(|limit| (1..=10_000).contains(limit))
                    .ok_or_else(|| config_error(format!("credential '{credential_id}': `manage_limit` is a whole number from 1 to 10000, or empty for no limit")))?
                    as u32,
            ),
        };
        let record_as = text_field(secret, "record_as");
        Ok(Self {
            id: credential_id.to_string(),
            scope: format!("{scope}/"),
            routes: format!("{routes}/"),
            tools,
            delegate: list_field(secret, "delegate"),
            nodes_mode,
            nodes,
            manage_kinds,
            manage_limit,
            record_as: if record_as.is_empty() { credential_id.to_string() } else { record_as },
            managed: Vec::new(),
        })
    }

    /// Whether a repository-relative path lies inside the fence.
    pub fn holds(&self, repo_path: &str) -> bool {
        clean_path(repo_path).is_some_and(|path| format!("{path}/").starts_with(&self.scope))
    }

    /// Whether a webhook route lies under the fence's route prefix. A route
    /// carrying anything a router might decode or cut (`%`, `\`, `?`, `#`,
    /// spaces, control characters) or a `.`/`..` segment is outside, whatever
    /// it starts with.
    fn holds_route(&self, route: &str) -> bool {
        let route = route.trim();
        if route.chars().any(|c| matches!(c, '%' | '\\' | '?' | '#') || c.is_whitespace() || c.is_control()) {
            return false;
        }
        if route.split('/').any(|segment| segment == ".." || segment == ".") {
            return false;
        }
        let route = if route.starts_with('/') { route.to_string() } else { format!("/{route}") };
        format!("{}/", route.trim_end_matches('/')).starts_with(&self.routes)
    }
}

/// The config keys a kind's own definition marks as naming a credential: a
/// field whose options are credentials, or a flag spelled with "credential".
fn credential_keys(kind: &str, catalogue: &[NodeDefinition]) -> Vec<String> {
    use crate::pipeline::model::NodeFieldDataSource as S;
    let Some(def) = catalogue.iter().find(|def| def.kind == kind) else { return Vec::new() };
    let mut keys: Vec<String> = def
        .fields
        .iter()
        .filter(|field| {
            matches!(
                field.data_source,
                Some(
                    S::CredentialsAll
                        | S::CredentialsPostgres
                        | S::CredentialsJwt
                        | S::CredentialsBrowser
                        | S::CredentialsOpenAi
                        | S::CredentialsSecureRequest
                        | S::CredentialsHttpAuth
                        | S::CredentialsWebhookAuth
                        | S::CredentialsByKind(_)
                )
            )
        })
        .map(|field| field.name.clone())
        .collect();
    keys.extend(def.dsl_flags.iter().filter(|flag| flag.flag.contains("credential")).map(|flag| flag.config_key.clone()));
    keys
}

/// Every credential a node's config names: the values under the keys its
/// definition marks, and any value under a key spelled with "credential",
/// however deep and in whatever case — each string in it, or in a list.
fn credential_refs(config: &Value, marked: &[String]) -> Vec<String> {
    fn strings(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::String(text) if !text.trim().is_empty() => out.push(text.trim().to_string()),
            Value::Array(items) => items.iter().for_each(|item| strings(item, out)),
            Value::Object(map) => map.values().for_each(|item| strings(item, out)),
            _ => {}
        }
    }
    fn walk(value: &Value, marked: &[String], top: bool, out: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                for (key, inner) in map {
                    if key.to_ascii_lowercase().contains("credential") || (top && marked.contains(key)) {
                        strings(inner, out);
                    } else {
                        walk(inner, marked, false, out);
                    }
                }
            }
            Value::Array(items) => items.iter().for_each(|item| walk(item, marked, false, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(config, marked, true, &mut out);
    out
}

/// What a pipeline made through the node may contain: allowed entry nodes, the
/// nodes the fence's list lets through and never `zebflow.*`, webhook routes
/// under the prefix, delegated credentials only.
pub fn check_graph(graph: &PipelineGraph, fence: &Fence) -> Result<(), String> {
    let catalogue = crate::pipeline::nodes::basic::builtin_node_definitions();
    for (index, node) in graph.nodes.iter().enumerate() {
        let who = node_name(index, &node.id);
        let kind = node.kind.as_str();
        let family = kind.split('.').next().unwrap_or_default();
        if !catalogue.iter().any(|def| def.kind == kind) {
            return Err(format!("{who} is '{kind}', which may not be used: it is not in the catalogue exactly as written"));
        }
        if family == NEVER_FAMILY {
            return Err(format!("{who} is {kind}; a made pipeline may never use {NEVER_FAMILY}.*"));
        }
        if family == "trigger" && !ALLOWED_TRIGGERS.contains(&kind) {
            return Err(format!("{who} is {kind}; a made pipeline starts with {}", ALLOWED_TRIGGERS.join(", ")));
        }
        // In allow mode a trigger is listed like any node; in deny mode the
        // list governs the nodes after it.
        if family != "trigger" || fence.nodes_mode == NodesMode::Allow {
            match fence.nodes_mode {
                NodesMode::Deny => {
                    let denied: Vec<&str> = if fence.nodes.is_empty() {
                        DEFAULT_DENY.to_vec()
                    } else {
                        fence.nodes.iter().map(String::as_str).collect()
                    };
                    if let Some(pattern) = denied.iter().find(|pattern| pattern_names(pattern, kind)) {
                        return Err(format!("{who} is {kind}; this fence denies {pattern}"));
                    }
                }
                NodesMode::Allow => {
                    if !fence.nodes.iter().any(|pattern| pattern_names(pattern, kind)) {
                        return Err(format!("{who} is {kind}; this fence allows only {}", fence.nodes.join(", ")));
                    }
                }
            }
        }
        if kind == "trigger.webhook" {
            let route = node.config.get("route").and_then(Value::as_str).unwrap_or_default();
            if !fence.holds_route(route) {
                return Err(format!("{who} answers at '{route}', outside {}", fence.routes));
            }
        }
        // A made pipeline watches its own folder: what changes elsewhere in the
        // repository is not the fence's to react to.
        if kind == crate::pipeline::nodes::basic::trigger::repo::NODE_KIND {
            let raw = node.config.get("path").and_then(Value::as_str).unwrap_or_default();
            let path = crate::pipeline::nodes::basic::trigger::repo::watched_path(raw).map_err(|why| format!("{who}: {why}"))?;
            if path.is_empty() || !fence.holds(&path) {
                return Err(format!("{who} watches '{raw}', outside {}", fence.scope));
            }
        }
        for id in credential_refs(&node.config, &credential_keys(kind, &catalogue)) {
            if id.contains("{{") {
                return Err(format!("{who} names its credential with an expression; a made pipeline names a delegated credential as written"));
            }
            if !fence.delegate.contains(&id) && !fence.managed.contains(&id) {
                return Err(format!("{who} uses credential '{id}', which is not delegated to this fence"));
            }
        }
    }
    Ok(())
}

/// How a refusal names a node: its place in the pipeline as written, and the
/// author's label when there is one (pipe mode labels nodes `n0`, `n1`, …).
fn node_name(index: usize, id: &str) -> String {
    if id == format!("n{index}") { format!("node {}", index + 1) } else { format!("node {} ('{id}')", index + 1) }
}

/// The commands `--command` carries, each a tool and its arguments.
fn parse_commands(value: &Value) -> Result<Vec<(String, Value)>, PipelineError> {
    let value = match value {
        Value::String(text) if text.trim().is_empty() => return Err(config_error("--command is empty")),
        Value::String(text) => serde_json::from_str(text.trim()).map_err(|err| config_error(format!("--command is not JSON: {err}")))?,
        Value::Null => return Err(config_error("--command is empty")),
        other => other.clone(),
    };
    let items = match value {
        Value::Array(items) => items,
        one => vec![one],
    };
    if items.is_empty() || items.len() > MAX_COMMANDS {
        return Err(config_error(format!("--command carries {} commands; one run takes 1 to {MAX_COMMANDS}", items.len())));
    }
    items
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            let tool = item.get("tool").and_then(Value::as_str).map(str::trim).unwrap_or_default().to_string();
            if tool.is_empty() {
                return Err(config_error(format!("command {index} names no `tool`")));
            }
            let arguments = item.get("arguments").cloned().unwrap_or_else(|| json!({}));
            if !arguments.is_object() {
                return Err(config_error(format!("command {index} ({tool}): `arguments` must be an object")));
            }
            Ok((tool, arguments))
        })
        .collect()
}

fn arg<'a>(arguments: &'a Value, key: &str) -> Result<&'a str, String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("`{key}` is required"))
}

fn opt<'a>(arguments: &'a Value, key: &str) -> Option<&'a str> {
    arguments.get(key).and_then(Value::as_str).filter(|value| !value.trim().is_empty())
}

fn number(arguments: &Value, key: &str) -> Option<u32> {
    arguments.get(key).and_then(Value::as_u64).map(|n| n.min(u64::from(u32::MAX)) as u32)
}

/// An operation's text answer as a value: JSON when it is JSON, else the text.
fn answered(result: OpsResult) -> Result<Value, String> {
    if result.text.starts_with("Error:") {
        return Err(result.text);
    }
    Ok(serde_json::from_str(&result.text).unwrap_or(Value::String(result.text)))
}

pub struct Node {
    config: Config,
    platform: Arc<PlatformService>,
}

impl Node {
    pub fn new(config: Config, platform: Arc<PlatformService>) -> Result<Self, PipelineError> {
        if config.credential_id.trim().is_empty() {
            return Err(config_error("--credential is empty; it needs a zebflow_control credential's id"));
        }
        Ok(Self { config, platform })
    }

    /// A pipeline's identity, cleaned, and its repository-relative path. An
    /// identity with `..`, a leading `/` or a backslash is refused rather than
    /// repaired, and the cleaned one is what the operations receive.
    fn pipeline_path(&self, owner: &str, project: &str, file_rel_path: &str) -> Result<(String, String), String> {
        if file_rel_path.contains('\\') {
            return Err(format!("pipeline '{file_rel_path}': a path uses /, not \\"));
        }
        let cleaned = clean_path(file_rel_path).filter(|path| !path.is_empty()).ok_or_else(|| format!("pipeline '{file_rel_path}' is not a plain path inside the project"))?;
        let layout = self.platform.projects.project_layout(owner, project).map_err(|err| err.message)?;
        let identity = crate::platform::services::project::normalize_pipeline_file_rel_path(&layout.repo_layout, &cleaned);
        let source = layout.repo_layout.source.trim_matches('/');
        let repo_path = if source.is_empty() { identity.clone() } else { format!("{source}/{identity}") };
        Ok((identity, repo_path))
    }

    /// No link inside the fence's folder leads out of it: the nearest part of
    /// the path that exists must resolve inside the folder, and the folder
    /// itself must not be a link.
    fn really_inside(&self, owner: &str, project: &str, fence: &Fence, path: &str) -> Result<(), String> {
        let layout = self.platform.projects.project_layout(owner, project).map_err(|err| err.message)?;
        let repo = layout.repo_dir.canonicalize().map_err(|err| err.to_string())?;
        let scope_dir = repo.join(fence.scope.trim_end_matches('/'));
        if !scope_dir.exists() {
            return Ok(());
        }
        let real_scope = scope_dir.canonicalize().map_err(|err| err.to_string())?;
        if real_scope != scope_dir {
            return Err(format!("the folder {} is a link; a fence's folder must be a real folder", fence.scope));
        }
        let mut probe = repo.join(path);
        while !probe.exists() {
            if !probe.pop() {
                break;
            }
        }
        let real = probe.canonicalize().map_err(|err| err.to_string())?;
        if real.starts_with(&real_scope) {
            Ok(())
        } else {
            Err(format!("'{path}' leads outside {} through a link", fence.scope))
        }
    }

    /// A file path inside the fence, cleaned. Pipeline files are refused: they
    /// change only through the pipeline operations, which check them.
    fn fenced_file(&self, fence: &Fence, owner: &str, project: &str, arguments: &Value, key: &str) -> Result<String, String> {
        let raw = arg(arguments, key)?;
        let path = clean_path(raw).filter(|path| fence.holds(path)).ok_or_else(|| format!("'{raw}' is outside {}", fence.scope))?;
        if path.ends_with(PIPELINE_FILE) {
            return Err(format!("'{path}' is a pipeline file; pipelines change through pipeline_register and pipeline_delete"));
        }
        self.really_inside(owner, project, fence, &path)?;
        Ok(path)
    }

    fn fenced_pipeline(&self, fence: &Fence, owner: &str, project: &str, arguments: &Value) -> Result<String, String> {
        let file_rel_path = arg(arguments, "file_rel_path")?;
        let (identity, repo_path) = self.pipeline_path(owner, project, file_rel_path)?;
        if !fence.holds(&repo_path) {
            return Err(format!("pipeline '{file_rel_path}' is outside {}", fence.scope));
        }
        self.really_inside(owner, project, fence, &repo_path)?;
        Ok(identity)
    }

    /// The saved pipeline, read back and checked: what will run is what was
    /// checked, whatever the register command's own parsing made of the body.
    fn check_saved(&self, fence: &Fence, owner: &str, project: &str, identity: &str) -> Result<(), String> {
        let source = self.platform.projects.read_pipeline_source(owner, project, identity).map_err(|err| err.to_string())?;
        let graph = crate::contracts::kinds::decode_pipeline_graph(source.as_bytes())
            .map_err(|err| err.to_string())?
            .spec;
        check_graph(&graph, fence)
    }

    /// Runs one command inside the fence. The error is the refusal or the
    /// operation's own message.
    async fn run_one(&self, fence: &Fence, owner: &str, project: &str, tool: &str, arguments: &Value) -> Result<Value, String> {
        if !fence.tools.iter().any(|allowed| allowed == tool) {
            return Err(format!("`{tool}` is not opened by this fence"));
        }
        let ops = PlatformOps::new(self.platform.clone(), owner, project);
        match tool {
            "file_list" => {
                let glob = format!("{}**", fence.scope);
                answered(ops.file_list(FileListOptions {
                    query: opt(arguments, "query"),
                    glob: Some(&glob),
                    kind: opt(arguments, "kind"),
                    limit: number(arguments, "limit"),
                    format: None,
                }))
            }
            "file_read" => {
                let path = self.fenced_file(fence, owner, project, arguments, "rel_path")?;
                answered(ops.file_read(&path, number(arguments, "offset"), number(arguments, "limit")))
            }
            "file_write" => {
                let path = self.fenced_file(fence, owner, project, arguments, "rel_path")?;
                let content = arguments.get("content").and_then(Value::as_str).ok_or("`content` is required")?;
                // The app sent the content; answering it back doubles a
                // large file for nothing.
                answered(ops.write_if_unchanged(&path, opt(arguments, "expect_hash"), || async { ops.file_write(&path, content) }).await)?;
                Ok(json!({ "ok": true, "rel_path": path, "bytes": content.len() }))
            }
            "file_edit" => {
                let path = self.fenced_file(fence, owner, project, arguments, "rel_path")?;
                let old_string = arg(arguments, "old_string")?;
                let new_string = arguments.get("new_string").and_then(Value::as_str).unwrap_or_default();
                answered(ops.write_if_unchanged(&path, opt(arguments, "expect_hash"), || async { ops.file_edit(&path, old_string, new_string) }).await)
            }
            "file_stat" => {
                // Read-only, so a pipeline file inside the fence answers too:
                // its hash is what pipeline_register's expect_hash names.
                let raw = arg(arguments, "rel_path")?;
                let path = clean_path(raw).filter(|path| fence.holds(path)).ok_or_else(|| format!("'{raw}' is outside {}", fence.scope))?;
                self.really_inside(owner, project, fence, &path)?;
                answered(ops.file_stat(&path))
            }
            "template_check" => {
                let path = self.fenced_file(fence, owner, project, arguments, "rel_path")?;
                let content = arguments.get("content").and_then(Value::as_str);
                answered(ops.template_check(&path, content))
            }
            "file_delete" => {
                let path = self.fenced_file(fence, owner, project, arguments, "rel_path")?;
                if format!("{path}/") == fence.scope {
                    return Err(format!("{} is the fence's own folder and stays", fence.scope));
                }
                self.platform
                    .projects
                    .delete_repo_file_or_empty_folder(owner, project, &path)
                    .map(|()| json!({ "ok": true, "rel_path": path }))
                    .map_err(|err| err.to_string())
            }
            "pipeline_get" => {
                let identity = self.fenced_pipeline(fence, owner, project, arguments)?;
                answered(ops.pipeline_get(&identity, None))
            }
            "pipeline_register" => {
                let identity = self.fenced_pipeline(fence, owner, project, arguments)?;
                let body = arg(arguments, "body")?;
                let graph = crate::platform::shell::parser::build_pipeline_graph("check", body)?;
                check_graph(&graph, fence)?;
                let (_, repo_path) = self.pipeline_path(owner, project, arg(arguments, "file_rel_path")?)?;
                let register = || ops.pipeline_register(body, Some(&identity), None, None, opt(arguments, "title"), opt(arguments, "description"));
                let answer = answered(ops.write_if_unchanged(&repo_path, opt(arguments, "expect_hash"), register).await)?;
                if let Err(reason) = self.check_saved(fence, owner, project, &identity) {
                    // Saved, but not what was checked: take it back out.
                    let _ = ops.pipeline_delete(&identity).await;
                    return Err(format!("the saved pipeline failed the check and was removed: {reason}"));
                }
                Ok(answer)
            }
            "pipeline_activate" => {
                // Checked again as it is switched on, from what is saved.
                let identity = self.fenced_pipeline(fence, owner, project, arguments)?;
                self.check_saved(fence, owner, project, &identity)?;
                let answer = answered(ops.pipeline_activate(&identity).await)?;
                let live = self
                    .platform
                    .projects
                    .get_pipeline_meta_by_file_id(owner, project, &identity)
                    .ok()
                    .flatten()
                    .is_some_and(|meta| meta.active_hash.is_some());
                if live {
                    Ok(answer)
                } else {
                    Err(format!("pipeline '{identity}' did not switch on: {answer}"))
                }
            }
            "pipeline_deactivate" => {
                let identity = self.fenced_pipeline(fence, owner, project, arguments)?;
                answered(ops.pipeline_deactivate(&identity).await)
            }
            "pipeline_delete" => {
                let identity = self.fenced_pipeline(fence, owner, project, arguments)?;
                ops.pipeline_delete(&identity).await
            }
            "credential_list" => super::managed::list(&self.platform, owner, project, fence),
            "credential_create" => super::managed::create(&self.platform, owner, project, fence, arguments),
            "credential_update" => super::managed::update(&self.platform, owner, project, fence, arguments),
            "credential_delete" => super::managed::delete(&self.platform, owner, project, fence, arguments),
            other => Err(format!("`{other}` is not an operation this node runs")),
        }
    }
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Process],
        title: "Run Zebflow Commands".to_string(),
        description: "Runs Zebflow's own operations — write and read files, register, switch on, switch off and delete pipelines — \
            inside the fence a `zebflow_control` credential draws: one folder, one route prefix, the operations it opens and the \
            credentials it delegates. A downstream app's webhook hands it `--command`, one `{ tool, arguments }` or a list, in the \
            shape the MCP tools take. A pipeline it makes may start only with a webhook, schedule or manual trigger, may \
            answer only under the route prefix, may use only delegated credentials named as written, and never uses `zebflow.*`; which other nodes it \
            may use is the fence's `nodes` list, in deny mode (by default the project's shared stores: `sekejap`, `sqlite`, `kv`, `fs`, \
            `table`, `function`) or allow mode (only those listed). `out` adds `command: { ok, results }`. A refused \
            or failed command fails the node, naming its index, tool and reason; the commands before it stay done. Every change \
            is recorded as `control · <fence>` and the `--as` person."
            .to_string(),
        input_pins: vec!["in".to_string()],
        output_pins: vec!["out".to_string()],
        output_schema: json!({
            "description": "The payload plus `command`: `ok` and one result per command, in order.",
            "properties": { "command": { "properties": { "ok": { "type": "boolean" }, "results": { "type": "array" } } } }
        }),
        config_schema: json!({
            "type": "object",
            "required": ["credential_id", "command"],
            "properties": {
                "credential_id": { "type": "string", "description": "The zebflow_control credential that draws the fence." },
                "command": { "description": "One { tool, arguments } or a list of them." },
                "as": { "description": "Who asked, for the change record." }
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--credential".to_string(),
                config_key: "credential_id".to_string(),
                description: "The zebflow_control credential that draws the fence.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
                value: "text".to_string(),
                ..Default::default()
            },
            DslFlag {
                flag: "--command".to_string(),
                config_key: "command".to_string(),
                description: "One { tool, arguments } or a list of them: \"{{ input.webhook.body }}\".".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
                value: "expression".to_string(),
                ..Default::default()
            },
            DslFlag {
                flag: "--as".to_string(),
                config_key: "as".to_string(),
                description: "Who asked, recorded beside the fence in the change record: \"{{ input.webhook.auth.sub }}\".".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                value: "expression".to_string(),
                ..Default::default()
            },
        ],
        fields: vec![
            NodeFieldDef { name: "credential_id".to_string(), label: "Fence".to_string(), field_type: NodeFieldType::Text, help: Some("The zebflow_control credential's id.".to_string()), ..Default::default() },
            NodeFieldDef { name: "command".to_string(), label: "Command".to_string(), field_type: NodeFieldType::Text, help: Some("One { tool, arguments } or a list, usually {{ input.webhook.body }}.".to_string()), ..Default::default() },
            NodeFieldDef { name: "as".to_string(), label: "As".to_string(), field_type: NodeFieldType::Text, help: Some("Who asked, for the change record.".to_string()), ..Default::default() },
        ],
        examples: vec![
            NodeExample::dsl(
                "An admin-only door for a downstream app",
                r#"zebflow.command.run --credential zf_modules --command "{{ input.webhook.body }}" --as "{{ input.webhook.auth.sub }}""#,
            )
            .output(json!({ "command": { "ok": true, "results": [{ "tool": "pipeline_activate", "ok": true, "answer": "activated modules/rain/jobs/hourly.zf.json" }] } }))
            .note("Behind `trigger.webhook --auth jwt` with an admin role. The body is `{ \"tool\": \"pipeline_activate\", \"arguments\": { \"file_rel_path\": \"modules/rain/jobs/hourly.zf.json\" } }`."),
        ],
        ..Default::default()
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        NODE_KIND
    }

    fn input_pins(&self) -> &'static [&'static str] {
        &["in"]
    }

    fn output_pins(&self) -> &'static [&'static str] {
        &["out"]
    }

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        let (owner, project, _pipeline, _request_id) = metadata_scope(&input.metadata)?;
        let (owner, project) = (owner.to_string(), project.to_string());
        let credential = self
            .platform
            .credentials
            .get_project_credential(&owner, &project, &self.config.credential_id)
            .map_err(|err| PipelineError::new(CODE, err.message))?
            .ok_or_else(|| config_error(format!("credential '{}' not found", self.config.credential_id)))?;
        if credential.kind != CREDENTIAL_KIND {
            return Err(config_error(format!(
                "credential '{}' is a {} credential, not {CREDENTIAL_KIND}",
                credential.credential_id, credential.kind
            )));
        }
        let mut fence = Fence::from_secret(&credential.credential_id, &credential.secret)?;
        fence.managed = super::managed::managed_ids(&self.platform, &owner, &project, &fence).map_err(|err| PipelineError::new(CODE, err))?;
        let commands = parse_commands(&self.config.command)?;
        let person = match &self.config.as_person {
            Value::String(text) => text.trim().chars().filter(|c| !c.is_control()).take(80).collect(),
            Value::Number(number) => number.to_string(),
            _ => String::new(),
        };
        let actor = ChangeActor {
            person,
            via: format!("control · {}", fence.record_as),
            run: change_log::carried().and_then(|outer| outer.run),
        };

        let results = change_log::scope(actor, async {
            let mut results = Vec::new();
            for (index, (tool, arguments)) in commands.iter().enumerate() {
                match self.run_one(&fence, &owner, &project, tool, arguments).await {
                    Ok(answer) => results.push(json!({ "tool": tool, "ok": true, "answer": answer })),
                    Err(reason) => {
                        let code = if ["outside", "not opened", "may never", "denies", "allows only", "not delegated", "starts with", "pipeline file", "a link", "own folder", "not a plain path", "expression", "not in the catalogue", "failed the check", "not managed by", "door manages", "may never create", "not an outside address", "its limit", "already exists", "delegated to this door", "plain repository path", "--path is required", "changed since that version"].iter().any(|word| reason.contains(word)) {
                            REFUSED_CODE
                        } else {
                            CODE
                        };
                        return Err(PipelineError::new(code, format!("command {index} ({tool}): {reason}")));
                    }
                }
            }
            Ok(results)
        })
        .await?;
        Ok(NodeExecutionOutput {
            output_pins: vec!["out".to_string()],
            payload: with_answer(input.payload, json!({ "command": { "ok": true, "results": results } })),
            trace: vec![format!("zebflow.command.run: {} command(s) inside {}", commands.len(), fence.scope)],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fence() -> Fence {
        Fence::from_secret(
            "zf_modules",
            &json!({ "scope": "modules/", "routes": "/m", "tools": ["file_write", "pipeline_register"], "delegate": "pg_modules, rain_key" }),
        )
        .expect("fence")
    }

    #[test]
    fn a_fence_must_draw_a_real_boundary() {
        let f = fence();
        assert_eq!((f.scope.as_str(), f.routes.as_str(), f.record_as.as_str()), ("modules/", "/m/", "zf_modules"));
        assert_eq!(f.delegate, vec!["pg_modules", "rain_key"], "a list from text");
        for bad in [
            json!({ "scope": "", "routes": "/m", "tools": ["file_write"] }),
            json!({ "scope": "/", "routes": "/m", "tools": ["file_write"] }),
            json!({ "scope": "../other", "routes": "/m", "tools": ["file_write"] }),
            json!({ "scope": "modules", "routes": "/", "tools": ["file_write"] }),
            json!({ "scope": "modules", "routes": "/m", "tools": [] }),
            json!({ "scope": "modules", "routes": "/m", "tools": ["credential_rotate"] }),
            json!({ "scope": "modules", "routes": "/m", "tools": ["credential_create"] }),
            json!({ "scope": "modules", "routes": "/m", "tools": ["file_read"], "manage_kinds": ["zebflow_control"] }),
            json!({ "scope": "modules", "routes": "/m", "tools": ["file_read"], "manage_kinds": ["no_such_kind"] }),
            json!({ "scope": "modules", "routes": "/m", "tools": ["file_read"], "manage_limit": 0 }),
        ] {
            assert!(Fence::from_secret("x", &bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_path_is_inside_only_under_the_scope() {
        let f = fence();
        assert!(f.holds("modules/rain/panel.tsx"));
        assert!(f.holds("./modules//rain/x.tsx"));
        assert!(!f.holds("modules-old/x.tsx"), "a neighbour sharing the prefix is outside");
        assert!(!f.holds("modules/../pages/home.tsx"), "no climbing out");
        assert!(!f.holds("/modules/x.tsx"));
        assert!(!f.holds("pages/home.tsx"));
        assert!(f.holds_route("/m/rain/latest") && f.holds_route("m/rain"));
        assert!(!f.holds_route("/mail") && !f.holds_route("/admin") && !f.holds_route("/m/../admin"));
    }

    fn graph(dsl: &str) -> PipelineGraph {
        crate::platform::shell::parser::build_pipeline_graph("t", dsl).expect("graph")
    }

    #[test]
    fn a_made_pipeline_stays_inside_the_fence() {
        let f = fence();
        check_graph(&graph("| trigger.webhook --route /m/rain/latest | postgres.query.run --credential pg_modules -- \"SELECT 1\" | web.response.send --body ok"), &f)
            .expect("a delegated credential under the prefix");
        check_graph(&graph("| trigger.schedule --cron \"0 * * * *\" | http.response.fetch --url https://api.example.com/x"), &f).expect("a schedule");
        check_graph(&graph("| trigger.repo --path modules/rain | http.response.fetch --url https://api.example.com/x"), &f).expect("a watch inside the folder");
        let refusals = [
            ("| trigger.webhook --route /admin/x | web.response.send --body ok", "outside /m/"),
            ("| trigger.webhook --route /m/x | postgres.query.run --credential main_pg -- \"SELECT 1\"", "not delegated"),
            ("| trigger.webhook --route /m/x | kv.entry.get --key login", "denies kv.*"),
            ("| trigger.webhook --route /m/x | zebflow.command.run --credential zf_modules --command \"{{ input.webhook.body }}\"", "may never use zebflow"),
            ("| trigger.webhook --route /m/x | function.result.call --function main-job", "denies function.*"),
            ("| trigger.mcp --route /tools --name t --description d", "starts with"),
            ("| trigger.repo --path pages", "outside modules/"),
            ("| trigger.repo --path /", "outside modules/"),
            ("| trigger.repo --path modules-old/x", "outside modules/"),
            ("| trigger.repo --path modules/../pages", "plain repository path"),
        ];
        for (dsl, why) in refusals {
            let err = check_graph(&graph(dsl), &f).expect_err(dsl);
            assert!(err.contains(why), "{dsl}: {err}");
        }
    }

    fn fence_with(nodes_mode: &str, nodes: Value) -> Result<Fence, PipelineError> {
        Fence::from_secret("zf", &json!({ "scope": "modules/", "routes": "/m", "tools": ["file_write"], "delegate": ["pg_modules"], "nodes_mode": nodes_mode, "nodes": nodes }))
    }

    #[test]
    fn the_node_list_denies_or_allows_and_never_lets_zebflow_through() {
        // Deny mode with its own list replaces the default.
        let deny = fence_with("deny", json!(["http.response.fetch"])).expect("deny fence");
        assert!(check_graph(&graph("| trigger.webhook --route /m/x | kv.entry.get --key a"), &deny).is_ok(), "kv is not on this list");
        let err = check_graph(&graph("| trigger.webhook --route /m/x | http.response.fetch --url https://api.example.com"), &deny).unwrap_err();
        assert!(err.contains("denies http.response.fetch"), "{err}");

        // Allow mode: only what is listed, triggers by their own rule.
        let allow = fence_with("allow", json!(["trigger.webhook", "postgres.*", "web.response.send"])).expect("allow fence");
        check_graph(&graph("| trigger.webhook --route /m/x | postgres.query.run --credential pg_modules -- \"SELECT 1\" | web.response.send --body ok"), &allow)
            .expect("listed nodes");
        let err = check_graph(&graph("| trigger.webhook --route /m/x | http.response.fetch --url https://api.example.com"), &allow).unwrap_err();
        assert!(err.contains("allows only trigger.webhook, postgres.*, web.response.send"), "{err}");
        // In allow mode a trigger is listed like any node.
        let err = check_graph(&graph("| trigger.schedule --cron \"0 * * * *\" | web.response.send --body ok"), &allow).unwrap_err();
        assert!(err.contains("is trigger.schedule; this fence allows only"), "{err}");

        // zebflow.* is never allowed, not even when listed.
        let listed = fence_with("allow", json!(["trigger.webhook", "zebflow.*", "web.*"])).expect("fence");
        let err = check_graph(&graph("| trigger.webhook --route /m/x | zebflow.command.run --credential zf --command x"), &listed).unwrap_err();
        assert!(err.contains("may never use zebflow"), "{err}");

        // A typo, an unknown mode, or allow with nothing listed is refused.
        assert!(fence_with("deny", json!(["kv.entry.gett"])).is_err());
        assert!(fence_with("deny", json!(["nosuch.*"])).is_err());
        assert!(fence_with("maybe", json!([])).is_err());
        assert!(fence_with("allow", json!([])).is_err());
    }

    /// A graph straight from JSON, as a planted pipeline file would hold it.
    fn raw_graph(nodes: Value) -> PipelineGraph {
        serde_json::from_value(json!({ "id": "raw", "nodes": nodes, "edges": [] })).expect("graph")
    }

    fn node(kind: &str, config: Value) -> Value {
        json!({ "id": "n", "kind": kind, "input_pins": ["in"], "output_pins": ["out"], "config": config })
    }

    #[test]
    fn credentials_are_found_however_they_are_named_and_never_by_expression() {
        let f = fence();
        for config in [
            json!({ "auth": { "credential_id": "main_pg" } }),
            json!({ "credentials": ["main_pg"] }),
            json!({ "CredentialId": "main_pg" }),
            json!({ "deep": [{ "x": { "secret_credential": "main_pg" } }] }),
        ] {
            let err = check_graph(&raw_graph(json!([node("http.response.fetch", config.clone())])), &f).expect_err(&config.to_string());
            assert!(err.contains("'main_pg', which is not delegated"), "{config}: {err}");
        }
        let err = check_graph(&raw_graph(json!([node("postgres.query.run", json!({ "credential_id": "{{ input.webhook.body.cred }}" }))])), &f).unwrap_err();
        assert!(err.contains("expression"), "{err}");
        check_graph(&raw_graph(json!([node("postgres.query.run", json!({ "credential_id": "pg_modules" }))])), &f).expect("delegated, as written");
    }

    #[test]
    fn routes_and_kinds_that_could_be_read_two_ways_are_refused() {
        let f = fence();
        for route in ["/m/%2e%2e/admin", "/m/x?y=1", "/m/a b", "/m/./x", "/m/../admin", "/m\\x", "/m/x#y"] {
            assert!(!f.holds_route(route), "{route}");
        }
        assert!(f.holds_route("/m/rain/latest") && f.holds_route("/m/rain-2/v1.json"));
        for kind in ["KV.entry.get", "Zebflow.command.run", " kv.entry.get", "nosuch.thing.run"] {
            let err = check_graph(&raw_graph(json!([node(kind, json!({}))])), &f).unwrap_err();
            assert!(err.contains("not in the catalogue"), "{kind}: {err}");
        }
        let err = check_graph(&raw_graph(json!([node("trigger.function", json!({}))])), &f).unwrap_err();
        assert!(err.contains("starts with"), "a function trigger: {err}");
    }

    #[test]
    fn commands_are_one_or_a_list_of_tool_and_arguments() {
        assert_eq!(parse_commands(&json!({ "tool": "file_read", "arguments": { "rel_path": "modules/a" } })).unwrap().len(), 1);
        assert_eq!(parse_commands(&json!([{ "tool": "a" }, { "tool": "b" }])).unwrap().len(), 2);
        assert_eq!(parse_commands(&json!("{\"tool\":\"a\"}")).unwrap()[0].0, "a", "JSON text");
        assert!(parse_commands(&json!({ "arguments": {} })).is_err(), "no tool");
        assert!(parse_commands(&json!({ "tool": "a", "arguments": [] })).is_err(), "arguments not an object");
        assert!(parse_commands(&Value::Null).is_err());
        assert!(parse_commands(&json!(vec![json!({ "tool": "a" }); MAX_COMMANDS + 1])).is_err());
    }
}

//! Platform-aware tools for the project assistant agentic loop.
//!
//! Thin adapter over `PlatformOps`. Converts `OpsResult` → `ToolRunResult`,
//! honoring the `navigate` field for browser redirection.

use std::sync::Arc;

use serde_json::{Value, json};

use crate::automaton::infra::llm_interface::ToolDef;
use crate::platform::services::{PlatformOps, PlatformService};

/// Result of executing a tool — text answer plus optional browser interaction sequence.
pub struct ToolRunResult {
    /// Human-readable result shown in the chat tool bubble.
    pub text: String,
    /// If present, emitted as `interaction_sequence` SSE event for browser automation.
    pub interaction: Option<Value>,
    /// If present, browser navigates to this URL after the tool call.
    pub navigate: Option<String>,
}

impl ToolRunResult {
    pub fn ok(s: impl Into<String>) -> Self {
        Self { text: s.into(), interaction: None, navigate: None }
    }
    pub fn ok_navigate(s: impl Into<String>, url: impl Into<String>) -> Self {
        Self { text: s.into(), interaction: None, navigate: Some(url.into()) }
    }
    pub fn err(s: impl Into<String>) -> Self {
        Self { text: format!("Error: {}", s.into()), interaction: None, navigate: None }
    }
}

/// Platform-aware tool runner for the project assistant.
pub struct AssistantPlatformTools {
    platform: Arc<PlatformService>,
    owner: String,
    project: String,
}

impl AssistantPlatformTools {
    pub fn new(platform: Arc<PlatformService>, owner: &str, project: &str) -> Self {
        Self {
            platform,
            owner: owner.to_string(),
            project: project.to_string(),
        }
    }

    /// Tool definitions in OpenAI function calling schema format.
    pub fn tool_defs() -> Vec<ToolDef> {
        vec![
            // ── Orientation ────────────────────────────────────────────────────
            ToolDef {
                name: "start_here".to_string(),
                description: "Call this first. Returns project overview: git status, pipeline count, \
                    docs list, DB connections, template tree, and AGENTS.md. \
                    Use at the start of every session to orient yourself.".to_string(),
                parameters: json!({ "type": "object", "properties": {} }),
            },
            // ── Help / Knowledge ───────────────────────────────────────────────
            ToolDef {
                name: "help".to_string(),
                description: "Hierarchical docs browser. No topic = full index with all available paths. \
                    Paths: 'pipeline' (DSL + web patterns), 'pipeline/dsl', 'pipeline/authoring', \
                    'pipeline/web', 'pipeline/nodes' (live node catalog), 'pipeline/nodes/{kind}' (one node), \
                    'pipeline/examples' (index), 'pipeline/examples/{slug}' (full recipe), \
                    'web' (TSX pages + imports), 'web/hooks', 'web/tailwind', 'web/design-system', 'web/libraries', \
                    'tool' (Tool.time/arr/stat/geo globals), \
                    'db' (database overview), 'db/sekejap' (SekejapQL), \
                    'platform' (overview), 'platform/agent', 'platform/api', 'platform/operations', 'platform/workflow'. \
                    Call help() before writing pipelines or templates.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "topic": { "type": "string", "description": "Help path to load. Omit for the full index." }
                    }
                }),
            },
            ToolDef {
                name: "help_search".to_string(),
                description: "Search Zebflow docs. Returns matching chunks from pipeline docs, \
                    web template docs, node catalog, and all help files. \
                    Use for any concept, node name, DSL flag, or syntax question.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["query"],
                    "properties": {
                        "query": { "type": "string", "description": "Search query — node name, DSL flag, concept, or keyword." }
                    }
                }),
            },
            // ── Pipelines ──────────────────────────────────────────────────────
            ToolDef {
                name: "pipeline_list".to_string(),
                description: "List all pipelines in the project with metadata (name, title, trigger_kind, active status).".to_string(),
                parameters: json!({ "type": "object", "properties": {} }),
            },
            ToolDef {
                name: "pipeline_get".to_string(),
                description: "Get a specific pipeline definition by file-relative path.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["file_rel_path"],
                    "properties": {
                        "file_rel_path": { "type": "string", "description": "File-relative path of the pipeline (e.g. 'pipelines/api/blog-home.zf.json')." }
                    }
                }),
            },
            ToolDef {
                name: "pipeline_register".to_string(),
                description: "Register (create or update) a pipeline by pipe-chained node body. \
                    Body format: '| trigger.webhook --path /x | pg.query --credential db -- \"SQL\"'. \
                    After registering, call pipeline_activate to make it live.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["body"],
                    "properties": {
                        "file_rel_path": { "type": "string", "description": "File-relative path under repo/ (e.g. 'pipelines/api/blog-home'). Preferred over name+path." },
                        "name": { "type": "string", "description": "Pipeline name slug. Used when file_rel_path is not set." },
                        "path": { "type": "string", "description": "Virtual path for grouping (e.g. '/pages', '/api'). Defaults to '/'." },
                        "title": { "type": "string", "description": "Optional human-readable display title." },
                        "body": { "type": "string", "description": "Pipeline body: pipe-chained nodes starting with |." }
                    }
                }),
            },
            ToolDef {
                name: "pipeline_describe".to_string(),
                description: "Inspect a pipeline — returns its nodes, edges, status, and hit stats. \
                    Node IDs from this output are required for pipeline_patch.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["file_rel_path"],
                    "properties": {
                        "file_rel_path": { "type": "string", "description": "File-relative path of the pipeline." }
                    }
                }),
            },
            ToolDef {
                name: "pipeline_patch".to_string(),
                description: "Patch one node in a saved pipeline without rewriting the full graph. \
                    Call pipeline_describe first to get node IDs. \
                    Pipeline status becomes stale after patching — call pipeline_activate to make it live again.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["file_rel_path", "node_id"],
                    "properties": {
                        "file_rel_path": { "type": "string", "description": "File-relative path of the pipeline." },
                        "node_id": { "type": "string", "description": "Node ID to patch (e.g. 'n0', 'b', 'trigger')." },
                        "flags": { "type": "string", "description": "Space-separated --flag value pairs (e.g. '--credential new-db --path /updated')." },
                        "body": { "type": "string", "description": "Body content for the node (SQL for pg.query, JS for script nodes)." }
                    }
                }),
            },
            ToolDef {
                name: "pipeline_activate".to_string(),
                description: "Activate a pipeline — makes it live so it can serve traffic and be executed. \
                    Must be called after pipeline_register or after patching.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["file_rel_path"],
                    "properties": {
                        "file_rel_path": { "type": "string", "description": "File-relative path of the pipeline to activate." }
                    }
                }),
            },
            ToolDef {
                name: "pipeline_deactivate".to_string(),
                description: "Deactivate a pipeline — takes it offline. \
                    Pipeline source is retained and can be re-activated later.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["file_rel_path"],
                    "properties": {
                        "file_rel_path": { "type": "string", "description": "File-relative path of the pipeline to deactivate." }
                    }
                }),
            },
            ToolDef {
                name: "pipeline_execute".to_string(),
                description: "Execute a registered active pipeline. Records execution hits. \
                    Pipeline must be activated first.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["file_rel_path"],
                    "properties": {
                        "file_rel_path": { "type": "string", "description": "File-relative path of the pipeline to execute." },
                        "input": { "type": "string", "description": "Optional JSON input payload string (e.g. '{\"order_id\": 42}')." }
                    }
                }),
            },
            ToolDef {
                name: "pipeline_run".to_string(),
                description: "Run a pipe-chained node body EPHEMERALLY — not saved, not logged, no hit recording. \
                    Use this to test queries, explore data, or prototype before registering. \
                    Example body: '| pg.query --credential main-db -- \"SELECT count(*) FROM users\"'. \
                    Auto-prepends trigger.manual if no trigger node specified.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["body"],
                    "properties": {
                        "body": { "type": "string", "description": "Pipe-chained node body starting with |." },
                        "input": { "description": "Optional JSON input payload (object or JSON string)." }
                    }
                }),
            },
            // ── Templates ──────────────────────────────────────────────────────
            ToolDef {
                name: "template_list".to_string(),
                description: "List all template files in the project workspace.".to_string(),
                parameters: json!({ "type": "object", "properties": {} }),
            },
            ToolDef {
                name: "template_get".to_string(),
                description: "Read a specific template file's full content.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["rel_path"],
                    "properties": {
                        "rel_path": { "type": "string", "description": "Relative path to the template file (e.g. 'pages/home.tsx')." }
                    }
                }),
            },
            ToolDef {
                name: "template_create".to_string(),
                description: "Create a new template file with scaffolding. \
                    Kind must be one of: page (pages/*.tsx), component (components/*.tsx), \
                    script (scripts/*.ts), folder. \
                    Returns the scaffolded content — use template_write to customise it after.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["kind", "name"],
                    "properties": {
                        "kind": { "type": "string", "description": "Kind: 'page', 'component', 'script', or 'folder'." },
                        "name": { "type": "string", "description": "Base name for the file (e.g. 'blog-home', 'user-card')." },
                        "parent_rel_path": { "type": "string", "description": "Optional parent folder path under templates/ (e.g. 'components/ui')." }
                    }
                }),
            },
            ToolDef {
                name: "template_write".to_string(),
                description: "Write (create or overwrite) a template file. \
                    Path is relative to templates/ (e.g. 'pages/blog-home.tsx'). \
                    Use help_web_engine for TSX conventions before writing.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["rel_path", "content"],
                    "properties": {
                        "rel_path": { "type": "string", "description": "Relative path under templates/ (e.g. 'pages/blog-home.tsx')." },
                        "content": { "type": "string", "description": "Full file content to write." }
                    }
                }),
            },
            // ── Project Docs ───────────────────────────────────────────────────
            ToolDef {
                name: "docs_project_list".to_string(),
                description: "List project doc files (ERD, README.md, architecture docs) under repo/docs/.".to_string(),
                parameters: json!({ "type": "object", "properties": {} }),
            },
            ToolDef {
                name: "docs_project_read".to_string(),
                description: "Read one project doc by path (e.g. README.md, architecture.md).".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["path"],
                    "properties": {
                        "path": { "type": "string", "description": "Relative path to the doc file under repo/docs (e.g. 'README.md')." }
                    }
                }),
            },
            ToolDef {
                name: "docs_project_write".to_string(),
                description: "Write (create or update) a project doc file under repo/docs/. \
                    Use for specs, architecture docs, ERDs, API contracts, README, CHANGELOG. \
                    Always commit after writing with git_command.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["path", "content"],
                    "properties": {
                        "path": { "type": "string", "description": "Relative path under repo/docs/ (e.g. 'README.md', 'architecture.md')." },
                        "content": { "type": "string", "description": "Full file content to write." }
                    }
                }),
            },
            // ── Agent Docs ─────────────────────────────────────────────────────
            ToolDef {
                name: "docs_agent_list".to_string(),
                description: "List the three agent doc files: AGENTS.md (project instructions), \
                    SOUL.md (agent personality), MEMORY.md (persistent memory across sessions). \
                    Always read AGENTS.md first when starting work on a project.".to_string(),
                parameters: json!({ "type": "object", "properties": {} }),
            },
            ToolDef {
                name: "docs_agent_read".to_string(),
                description: "Read one agent doc: AGENTS.md (project instructions), \
                    SOUL.md (agent personality), or MEMORY.md (persistent memory). \
                    Read AGENTS.md at the start of every session.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["name"],
                    "properties": {
                        "name": { "type": "string", "description": "Agent doc name: 'AGENTS.md', 'SOUL.md', or 'MEMORY.md'." }
                    }
                }),
            },
            ToolDef {
                name: "docs_agent_write".to_string(),
                description: "Write an agent doc. AGENTS.md: project-specific instructions. \
                    SOUL.md: agent personality and tone. \
                    MEMORY.md: persistent notes the agent writes to remember things across sessions.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["name", "content"],
                    "properties": {
                        "name": { "type": "string", "description": "Agent doc name: 'AGENTS.md', 'SOUL.md', or 'MEMORY.md'." },
                        "content": { "type": "string", "description": "Full file content to write." }
                    }
                }),
            },
            // ── Connections & Credentials ──────────────────────────────────────
            ToolDef {
                name: "connection_list".to_string(),
                description: "List all DB connections for this project — returns slug, label, and kind \
                    (postgres, mysql, sekejap). Use the slug with connection_describe and in --credential flags.".to_string(),
                parameters: json!({ "type": "object", "properties": {} }),
            },
            ToolDef {
                name: "connection_describe".to_string(),
                description: "Describe a DB connection's schema — tables, columns, types, constraints. \
                    Use scope='tables' for a quick overview, or omit scope for the full tree. \
                    Always run this before writing SQL queries. \
                    Use table='schema.table' (e.g. table='public.users') for column detail.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["slug"],
                    "properties": {
                        "slug": { "type": "string", "description": "Connection slug (e.g. 'main-db', 'default')." },
                        "scope": { "type": "string", "description": "Scope: 'tables', 'schemas', or omit for full tree." },
                        "schema": { "type": "string", "description": "Filter to a specific schema name (e.g. 'public')." },
                        "table": { "type": "string", "description": "Filter to a specific table for column detail (e.g. 'public.users')." }
                    }
                }),
            },
            ToolDef {
                name: "credential_list".to_string(),
                description: "List credentials for this project — returns id, title, and kind only. \
                    Values are never exposed. Use the id in pipeline nodes that require authentication.".to_string(),
                parameters: json!({ "type": "object", "properties": {} }),
            },
            // ── Git ────────────────────────────────────────────────────────────
            ToolDef {
                name: "git_command".to_string(),
                description: "Run a git command on the project repository. \
                    Allowed subcommands: status, log, diff, add, commit. \
                    Destructive operations (reset, rebase, force, checkout) are blocked. \
                    Always commit after registering or patching pipelines.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["subcommand"],
                    "properties": {
                        "subcommand": { "type": "string", "description": "Git subcommand: status, log, diff, add, commit." },
                        "args": { "type": "string", "description": "Additional arguments as a space-separated string." },
                        "message": { "type": "string", "description": "Commit message — only used when subcommand is 'commit'." }
                    }
                }),
            },
            // ── UI Catalog ─────────────────────────────────────────────────────
            ToolDef {
                name: "list_ui_catalog".to_string(),
                description: "List all available shadcn-compatible Zeb React UI components \
                    that can be installed into shared/ui/. Returns name, category, description, \
                    filename, and whether each component is already installed.".to_string(),
                parameters: json!({ "type": "object", "properties": {} }),
            },
            ToolDef {
                name: "install_ui_components".to_string(),
                description: "Install shadcn-compatible UI components into shared/ui/. \
                    Pass names like ['button','card','dialog']. \
                    Set overwrite=true to replace existing files.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["names"],
                    "properties": {
                        "names": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Component names to install, e.g. ['button', 'card', 'dialog']."
                        },
                        "overwrite": { "type": "boolean", "description": "If true, overwrite existing files. Default: false." }
                    }
                }),
            },
        ]
    }

    /// Execute a named tool, delegating all logic to `PlatformOps`.
    pub async fn run_async(&self, name: &str, args: &Value) -> ToolRunResult {
        let ops = PlatformOps::new(self.platform.clone(), &self.owner, &self.project);

        let result = match name {
            // ── Orientation ────────────────────────────────────────────────────
            "start_here" => ops.start_here().await,

            // ── Help / Knowledge ───────────────────────────────────────────────
            "help" => ops.help(args.get("topic").and_then(|v| v.as_str()).unwrap_or("")),
            "help_search" => ops.help_search(args["query"].as_str().unwrap_or("")),

            // ── Pipelines ──────────────────────────────────────────────────────
            "pipeline_list" => ops.pipeline_list(),
            "pipeline_get" => ops.pipeline_get(args["file_rel_path"].as_str().unwrap_or("")),
            "pipeline_register" => {
                ops.pipeline_register(
                    args["body"].as_str().unwrap_or(""),
                    args.get("file_rel_path").and_then(|v| v.as_str()),
                    args.get("name").and_then(|v| v.as_str()),
                    args.get("path").and_then(|v| v.as_str()),
                    args.get("title").and_then(|v| v.as_str()),
                ).await
            }
            "pipeline_describe" => {
                ops.pipeline_describe(args["file_rel_path"].as_str().unwrap_or("")).await
            }
            "pipeline_patch" => {
                ops.pipeline_patch(
                    args["file_rel_path"].as_str().unwrap_or(""),
                    args["node_id"].as_str().unwrap_or(""),
                    args.get("flags").and_then(|v| v.as_str()),
                    args.get("body").and_then(|v| v.as_str()),
                ).await
            }
            "pipeline_activate" => {
                ops.pipeline_activate(args["file_rel_path"].as_str().unwrap_or("")).await
            }
            "pipeline_deactivate" => {
                ops.pipeline_deactivate(args["file_rel_path"].as_str().unwrap_or("")).await
            }
            "pipeline_execute" => {
                ops.pipeline_execute(
                    args["file_rel_path"].as_str().unwrap_or(""),
                    args.get("input").and_then(|v| v.as_str()),
                ).await
            }
            "pipeline_run" => {
                let input = args.get("input").and_then(|v| match v {
                    Value::Object(_) | Value::Array(_) => Some(v.clone()),
                    Value::String(s) if !s.is_empty() => serde_json::from_str(s).ok(),
                    _ => None,
                });
                ops.pipeline_run(args["body"].as_str().unwrap_or(""), input).await
            }

            // ── Templates ──────────────────────────────────────────────────────
            "template_list" => ops.template_list(),
            "template_get" => ops.template_get(args["rel_path"].as_str().unwrap_or("")),
            "template_create" => ops.template_create(
                args["kind"].as_str().unwrap_or(""),
                args["name"].as_str().unwrap_or(""),
                args.get("parent_rel_path").and_then(|v| v.as_str()),
            ),
            "template_write" => ops.template_write(
                args["rel_path"].as_str().unwrap_or(""),
                args["content"].as_str().unwrap_or(""),
            ),

            // ── Project Docs ───────────────────────────────────────────────────
            "docs_project_list" => ops.docs_project_list(),
            "docs_project_read" => ops.docs_project_read(args["path"].as_str().unwrap_or("")),
            "docs_project_write" => ops.docs_project_write(
                args["path"].as_str().unwrap_or(""),
                args["content"].as_str().unwrap_or(""),
            ),

            // ── Agent Docs ─────────────────────────────────────────────────────
            "docs_agent_list" => ops.docs_agent_list(),
            "docs_agent_read" => ops.docs_agent_read(args["name"].as_str().unwrap_or("")),
            "docs_agent_write" => ops.docs_agent_write(
                args["name"].as_str().unwrap_or(""),
                args["content"].as_str().unwrap_or(""),
            ),

            // ── Connections & Credentials ──────────────────────────────────────
            "connection_list" => ops.connection_list(),
            "connection_describe" => {
                ops.connection_describe(
                    args["slug"].as_str().unwrap_or(""),
                    args.get("scope").and_then(|v| v.as_str()),
                    args.get("schema").and_then(|v| v.as_str()),
                    args.get("table").and_then(|v| v.as_str()),
                ).await
            }
            "credential_list" => ops.credential_list(),

            // ── Git ────────────────────────────────────────────────────────────
            "git_command" => {
                ops.git_command(
                    args["subcommand"].as_str().unwrap_or(""),
                    args.get("args").and_then(|v| v.as_str()),
                    args.get("message").and_then(|v| v.as_str()),
                ).await
            }

            // ── UI Catalog ─────────────────────────────────────────────────────
            "list_ui_catalog" => ops.list_ui_catalog(),
            "install_ui_components" => {
                let names: Vec<String> = args["names"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default();
                let overwrite = args.get("overwrite").and_then(|v| v.as_bool());
                ops.install_ui_components(names, overwrite)
            }

            _ => return ToolRunResult::err(format!("Unknown tool: '{name}'")),
        };

        // Convert OpsResult → ToolRunResult (navigate honored)
        if let Some(url) = result.navigate {
            ToolRunResult::ok_navigate(result.text, url)
        } else {
            ToolRunResult { text: result.text, interaction: None, navigate: None }
        }
    }
}

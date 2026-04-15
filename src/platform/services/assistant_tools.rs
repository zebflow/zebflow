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
        Self {
            text: s.into(),
            interaction: None,
            navigate: None,
        }
    }
    pub fn ok_navigate(s: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            text: s.into(),
            interaction: None,
            navigate: Some(url.into()),
        }
    }
    pub fn err(s: impl Into<String>) -> Self {
        Self {
            text: format!("Error: {}", s.into()),
            interaction: None,
            navigate: None,
        }
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
                    'db' (database overview), \
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
                name: "pipeline_search".to_string(),
                description: "Search pipeline .zf.json files for a pattern. Returns file:line matches. \
                    Optional glob filters files. Use output_mode='files_with_matches' for file paths only.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["pattern"],
                    "properties": {
                        "pattern": { "type": "string", "description": "Case-insensitive substring to search for." },
                        "glob": { "type": "string", "description": "Optional glob to filter files (e.g. 'pipelines/api/*.zf.json')." },
                        "context": { "type": "integer", "description": "Lines of context before/after each match (default 0)." },
                        "head_limit": { "type": "integer", "description": "Limit output to first N entries." },
                        "output_mode": { "type": "string", "description": "'content' (default) or 'files_with_matches' for file paths only." }
                    }
                }),
            },
            ToolDef {
                name: "pipeline_get".to_string(),
                description: "Get a specific pipeline by file-relative path. \
                    Use node_id to return just one node instead of the full graph.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["file_rel_path"],
                    "properties": {
                        "file_rel_path": { "type": "string", "description": "File-relative path of the pipeline (e.g. 'pipelines/api/blog-home.zf.json')." },
                        "node_id": { "type": "string", "description": "Optional node ID to return just that node. Accepts opaque ID, kind, or kind[index]." }
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
                    node_id accepts: opaque ID (e.g. 'n0'), node kind (e.g. 'trigger.webhook', 'pg.query'), \
                    or kind+index (e.g. 'pg.query[1]') when multiple nodes share the same kind. \
                    Pipeline status becomes stale after patching — call pipeline_activate to make it live again.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["file_rel_path", "node_id"],
                    "properties": {
                        "file_rel_path": { "type": "string", "description": "File-relative path of the pipeline." },
                        "node_id": { "type": "string", "description": "Node ID, kind, or kind+index (e.g. 'n0', 'trigger.webhook', 'pg.query[1]')." },
                        "flags": { "type": "string", "description": "Space-separated --flag value pairs (e.g. '--credential new-db --path /updated')." },
                        "body": { "type": "string", "description": "Body content for the node (SQL for pg.query, JS for script nodes)." }
                    }
                }),
            },
            ToolDef {
                name: "pipeline_activate".to_string(),
                description: "Activate a pipeline or bulk-activate many pipelines. \
                    Must be called after pipeline_register or after patching. \
                    Use glob to activate all matching pipelines at once (e.g. 'pipelines/modules/**').".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "file_rel_path": { "type": "string", "description": "File-relative path of a single pipeline to activate. Ignored when glob is set." },
                        "glob": { "type": "string", "description": "Glob pattern to bulk-activate matching pipelines (e.g. 'pipelines/modules/manage/**')." }
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
                    Pipeline must be activated first. \
                    For function pipelines (n.trigger.function) pass `input` to test with real data; \
                    without it the pipeline receives an empty payload.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["file_rel_path"],
                    "properties": {
                        "file_rel_path": { "type": "string", "description": "File-relative path of the pipeline to execute." },
                        "input": { "description": "Optional JSON input payload (object or JSON string). For function pipelines this becomes the `input` parameter inside the pipeline." }
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
            ToolDef {
                name: "pipeline_get_invocations".to_string(),
                description: "Get recent execution history for a pipeline. Returns stored invocations \
                    with timestamp, duration, status (ok/error), trigger source, error message, \
                    and per-node trace. Use this to inspect past runs, debug failures on scheduled \
                    pipelines, or verify that a pipeline is executing correctly.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["file_rel_path"],
                    "properties": {
                        "file_rel_path": { "type": "string", "description": "File-relative path of the pipeline (e.g. 'pipelines/api/blog-home.zf.json')." }
                    }
                }),
            },
            // ── Templates ──────────────────────────────────────────────────────
            ToolDef {
                name: "template_list".to_string(),
                description: "List template files in the project workspace. Use glob to filter.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "glob": { "type": "string", "description": "Optional glob to filter (e.g. 'pages/*.tsx'). Omit for all files." }
                    }
                }),
            },
            ToolDef {
                name: "template_get".to_string(),
                description: "Read a template file. Use offset/limit to read a range of lines.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["rel_path"],
                    "properties": {
                        "rel_path": { "type": "string", "description": "Relative path to the template file (e.g. 'pages/home.tsx')." },
                        "offset": { "type": "integer", "description": "1-based starting line number (optional)." },
                        "limit": { "type": "integer", "description": "Number of lines to return (optional)." }
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
            ToolDef {
                name: "template_search".to_string(),
                description: "Search template files for a pattern. Returns file:line matches. \
                    Optional glob filters files. Use output_mode='files_with_matches' for file paths only.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["pattern"],
                    "properties": {
                        "pattern": { "type": "string", "description": "Case-insensitive substring to search for." },
                        "glob": { "type": "string", "description": "Optional glob to filter files (e.g. 'pages/*.tsx')." },
                        "context": { "type": "integer", "description": "Lines of context before/after each match (default 0)." },
                        "head_limit": { "type": "integer", "description": "Limit output to first N entries." },
                        "output_mode": { "type": "string", "description": "'content' (default) or 'files_with_matches' for file paths only." }
                    }
                }),
            },
            ToolDef {
                name: "template_edit".to_string(),
                description: "Surgical string replacement in a template file. \
                    No need to read the full file first — just provide old_string and new_string. \
                    Fails if old_string not found or matches more than once.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["rel_path", "old_string", "new_string"],
                    "properties": {
                        "rel_path": { "type": "string", "description": "Relative path to the template file." },
                        "old_string": { "type": "string", "description": "Exact string to replace. Must match exactly once." },
                        "new_string": { "type": "string", "description": "Replacement string." }
                    }
                }),
            },
            ToolDef {
                name: "template_outline".to_string(),
                description: "Parse a template file and return its structural outline: imports, exports, \
                    functions, classes, types, interfaces — with line numbers. \
                    Much cheaper than template_get for understanding file structure.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["rel_path"],
                    "properties": {
                        "rel_path": { "type": "string", "description": "Relative path to the template file." }
                    }
                }),
            },
            ToolDef {
                name: "template_deps".to_string(),
                description: "Show a template's dependency graph: what it imports and \
                    which other templates import it. Use to understand component relationships.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["rel_path"],
                    "properties": {
                        "rel_path": { "type": "string", "description": "Relative path to the template file." }
                    }
                }),
            },
            ToolDef {
                name: "template_batch_edit".to_string(),
                description: "Apply multiple edits across one or more template files in a single call. \
                    Each edit has rel_path, old_string, new_string. Fails fast on first error.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["edits"],
                    "properties": {
                        "edits": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "required": ["rel_path", "old_string", "new_string"],
                                "properties": {
                                    "rel_path": { "type": "string" },
                                    "old_string": { "type": "string" },
                                    "new_string": { "type": "string" }
                                }
                            },
                            "description": "Array of edits to apply."
                        }
                    }
                }),
            },
            ToolDef {
                name: "move_resource".to_string(),
                description: "Rename or reorganize a pipeline or template file. \
                    Domain detected from path: .zf.json = pipeline, anything else = template. \
                    For pipelines: deactivate → move → re-activate lifecycle is automatic. \
                    Parent folders created automatically. No cross-domain moves.".to_string(),
                parameters: json!({
                    "type": "object",
                    "required": ["from_path", "to_path"],
                    "properties": {
                        "from_path": { "type": "string", "description": "Source path. Pipeline: file_rel_path e.g. 'pipelines/api/old.zf.json'. Template: rel_path e.g. 'pages/old.tsx'." },
                        "to_path": { "type": "string", "description": "Destination path. Same domain as from_path. Parent folders created automatically." }
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
                    (postgres, mysql, sqlite). Use the slug with connection_describe and in --credential flags.".to_string(),
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
            "pipeline_get" => ops.pipeline_get(
                args["file_rel_path"].as_str().unwrap_or(""),
                args.get("node_id").and_then(|v| v.as_str()),
            ),
            "pipeline_register" => {
                ops.pipeline_register(
                    args["body"].as_str().unwrap_or(""),
                    args.get("file_rel_path").and_then(|v| v.as_str()),
                    args.get("name").and_then(|v| v.as_str()),
                    args.get("path").and_then(|v| v.as_str()),
                    args.get("title").and_then(|v| v.as_str()),
                    args.get("description").and_then(|v| v.as_str()),
                )
                .await
            }
            "pipeline_describe" => {
                let compact = args
                    .get("compact")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                ops.pipeline_describe(args["file_rel_path"].as_str().unwrap_or(""), compact)
                    .await
            }
            "pipeline_patch" => {
                ops.pipeline_patch(
                    args["file_rel_path"].as_str().unwrap_or(""),
                    args["node_id"].as_str().unwrap_or(""),
                    args.get("flags").and_then(|v| v.as_str()),
                    args.get("body").and_then(|v| v.as_str()),
                )
                .await
            }
            "pipeline_activate" => {
                if let Some(glob) = args
                    .get("glob")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                {
                    ops.pipeline_activate_glob(glob).await
                } else {
                    ops.pipeline_activate(args["file_rel_path"].as_str().unwrap_or(""))
                        .await
                }
            }
            "pipeline_deactivate" => {
                ops.pipeline_deactivate(args["file_rel_path"].as_str().unwrap_or(""))
                    .await
            }
            "pipeline_execute" => {
                // Accept input as either a JSON string or a JSON object/array.
                // Normalize to a JSON string so ops.pipeline_execute can embed it in the DSL.
                let input_str = args.get("input").and_then(|v| match v {
                    Value::String(s) if !s.is_empty() => Some(s.clone()),
                    Value::Null => None,
                    other => serde_json::to_string(other).ok(),
                });
                ops.pipeline_execute(
                    args["file_rel_path"].as_str().unwrap_or(""),
                    input_str.as_deref(),
                )
                .await
            }
            "pipeline_run" => {
                let input = args.get("input").and_then(|v| match v {
                    Value::Object(_) | Value::Array(_) => Some(v.clone()),
                    Value::String(s) if !s.is_empty() => serde_json::from_str(s).ok(),
                    _ => None,
                });
                ops.pipeline_run(args["body"].as_str().unwrap_or(""), input)
                    .await
            }
            "pipeline_get_invocations" => {
                ops.pipeline_get_invocations(args["file_rel_path"].as_str().unwrap_or(""))
            }

            // ── Templates ──────────────────────────────────────────────────────
            "template_list" => ops.template_list(args.get("glob").and_then(|v| v.as_str())),
            "template_get" => ops.template_get(
                args["rel_path"].as_str().unwrap_or(""),
                args.get("offset")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u32),
                args.get("limit").and_then(|v| v.as_u64()).map(|n| n as u32),
            ),
            "template_create" => ops.template_create(
                args["kind"].as_str().unwrap_or(""),
                args["name"].as_str().unwrap_or(""),
                args.get("parent_rel_path").and_then(|v| v.as_str()),
            ),
            "template_write" => ops.template_write(
                args["rel_path"].as_str().unwrap_or(""),
                args["content"].as_str().unwrap_or(""),
            ),
            "template_search" => ops.template_search(
                args["pattern"].as_str().unwrap_or(""),
                args.get("glob").and_then(|v| v.as_str()),
                args.get("context").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                args.get("head_limit")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u32),
                args.get("output_mode").and_then(|v| v.as_str()),
            ),
            "pipeline_search" => ops.pipeline_search(
                args["pattern"].as_str().unwrap_or(""),
                args.get("glob").and_then(|v| v.as_str()),
                args.get("context").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                args.get("head_limit")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u32),
                args.get("output_mode").and_then(|v| v.as_str()),
            ),
            "template_outline" => ops.template_outline(args["rel_path"].as_str().unwrap_or("")),
            "template_deps" => ops.template_deps(args["rel_path"].as_str().unwrap_or("")),
            "template_batch_edit" => {
                let edits: Vec<(String, String, String)> = args
                    .get("edits")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|e| {
                                Some((
                                    e.get("rel_path")?.as_str()?.to_string(),
                                    e.get("old_string")?.as_str()?.to_string(),
                                    e.get("new_string")?.as_str()?.to_string(),
                                ))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                ops.template_batch_edit(&edits)
            }
            "template_edit" => ops.template_edit(
                args["rel_path"].as_str().unwrap_or(""),
                args["old_string"].as_str().unwrap_or(""),
                args["new_string"].as_str().unwrap_or(""),
            ),
            "move_resource" => {
                ops.move_resource(
                    args["from_path"].as_str().unwrap_or(""),
                    args["to_path"].as_str().unwrap_or(""),
                )
                .await
            }

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
                )
                .await
            }
            "credential_list" => ops.credential_list(),

            // ── Git ────────────────────────────────────────────────────────────
            "git_command" => {
                ops.git_command(
                    args["subcommand"].as_str().unwrap_or(""),
                    args.get("args").and_then(|v| v.as_str()),
                    args.get("message").and_then(|v| v.as_str()),
                )
                .await
            }

            // ── UI Catalog ─────────────────────────────────────────────────────
            "list_ui_catalog" => ops.list_ui_catalog(),
            "install_ui_components" => {
                let names: Vec<String> = args["names"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
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
            ToolRunResult {
                text: result.text,
                interaction: None,
                navigate: None,
            }
        }
    }
}

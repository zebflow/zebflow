//! Pipeline node interfaces and built-in node implementations.
//!
//! This module is the **single source of truth** for how nodes are authored in Zebflow.
//! Every built-in node lives under [`basic`].  This doc is the living specification —
//! read it before creating or modifying any node.
//!
//! ---
//!
//! # Payload Contract — Replace, Not Merge
//!
//! **Every node replaces the payload.** This is the universal rule.
//!
//! When a node produces output, it returns a **fresh JSON object** containing only
//! its own result. The upstream payload is discarded. This makes data flow explicit
//! and predictable — you always know exactly what each node outputs.
//!
//! ## Node categories
//!
//! | Category | Behavior | Examples |
//! |----------|----------|---------|
//! | **Trigger** | Constructs the initial payload from the incoming request | `trigger.webhook`, `trigger.schedule`, `trigger.manual` |
//! | **Transform** | Replaces payload with its result | `pg.query` → `{ rows }`, `script` → return value, `crypto` → `{ result }` |
//! | **Side-effect** | Performs action, passes payload through unchanged | `mem.set`, `mem.del`, `ws.emit`, `mem.publish` |
//! | **Routing** | Directs to output pins, passes payload through unchanged | `logic.if`, `logic.branch`, `logic.switch` |
//! | **Terminal** | Produces response envelope | `web.response` → `{ __zf_response }` |
//!
//! ## Accessing upstream data
//!
//! Since every node replaces the payload, use these references in `{{ expr }}`
//! config values to reach upstream data:
//!
//! | Reference | Description |
//! |-----------|-------------|
//! | `$input` | Current node's input (previous node's output) |
//! | `$trigger.auth` | JWT claims from the original request (immutable) |
//! | `$trigger.params` | URL path params (immutable) |
//! | `$trigger.query` | Query string params (immutable) |
//! | `$nodes.<id>.<field>` | Output of any completed upstream node by graph ID |
//!
//! `$trigger` is the **safe, immutable** reference — it never changes regardless of
//! what transformations happen in the pipeline. `$nodes.<id>` lets you reach across
//! the graph to any earlier node's output.
//!
//! ## Output key conventions
//!
//! | Node kind | Output shape |
//! |-----------|-------------|
//! | `pg.query` (SELECT) | `{ rows: [...] }` |
//! | `pg.query` (INSERT/UPDATE/DELETE) | `{ affected_rows: N }` |
//! | `sekejap.query` | `{ rows: [...] }` |
//! | `file.save` | `{ saved: { path, url, ... } }` |
//! | `img.thumbnail` | `{ thumbnail: { path, url, ... } }` |
//! | `crypto` (hash/encode) | `{ result: "..." }` |
//! | `mem.get` | `{ [out_key]: value }` |
//! | `mem.incr` | `{ [out_key]: number }` |
//! | `mem.exists` | `{ [out_key]: boolean }` |
//! | `http.request` | `{ request: {...}, response: {...} }` |
//! | `script` | Whatever the user returns |
//! | `auth.token.create` | `{ access_token, token_type, expires_in, ... }` |
//!
//! Nodes that process **multiple records** always use `rows` as the key:
//! `{ rows: [...] }`. This applies to `pg.query`, `sqlite.query`, `sekejap.query`.
//!
//! ---
//!
//! # Node Anatomy
//!
//! A complete node module exposes exactly four things:
//!
//! ```text
//! my_node.rs
//! ├── pub fn definition() -> NodeDefinition   ← kind-level contract (schemas, flags, docs)
//! ├── pub struct Config { ... }               ← typed config with JsonSchema derive
//! ├── pub struct Node { ... }                 ← runtime instance
//! └── impl NodeHandler for Node               ← execution logic
//! ```
//!
//! Then register in `basic/mod.rs → builtin_node_definitions()`.  That's it.
//!
//! ---
//!
//! # 1. `definition()` — the node contract
//!
//! `definition()` is the **most important function in a node module**.  It populates
//! [`NodeDefinition`](crate::pipeline::NodeDefinition) — the kind-level contract that
//! drives everything: UI dialogs, DSL parsing, help text, LLM context, API docs.
//!
//! ```rust,ignore
//! pub fn definition() -> NodeDefinition {
//!     NodeDefinition {
//!         kind: NODE_KIND.to_string(),           // "n.category.action"
//!         title: "Human Title".to_string(),      // shown in UI catalogs
//!         description: "...".to_string(),        // shown in UI + fed to LLM
//!         config_schema: serde_json::json!({ ... }),  // or schemars::schema_for!(Config)
//!         input_schema:  serde_json::json!({ ... }),  // payload shape consumed
//!         output_schema: serde_json::json!({ ... }),  // payload shape produced
//!         input_pins:  vec!["in".to_string()],   // empty for trigger nodes
//!         output_pins: vec!["out".to_string()],  // ["out","error"], ["true","false"], etc.
//!         dsl_flags: vec![ ... ],                // see §4 below
//!         script_available: false,               // see §5 below
//!         script_bridge: None,
//!         ai_tool: Default::default(),           // see §6 below
//!     }
//! }
//! ```
//!
//! ## Module-level doc = node documentation
//!
//! Write the module `//!` doc block as if it IS the node's reference page.  Include:
//!
//! - One-line summary of what the node does.
//! - **Pipeline position** — is it a trigger? terminal? middle? never standalone?
//! - **User-facing config table** — only fields the user/agent sets. Never list
//!   platform-injected fields (markup, route, etc.).
//! - **Config schema** — the JSON Schema block so LLMs can read it directly.
//! - **Studio UI hint** — ASCII mockup of how the node dialog should look.
//! - **Input/output payload examples** — what flows in, what flows out.
//! - **DSL examples** — show the pipe chain with all flags.
//! - **Project-level settings** if the node is affected by `zebflow.json` settings.
//!
//! The module doc becomes the `description` field at `/docs/node` — **no separate
//! markdown file needed**.  The source IS the docs.
//!
//! ---
//!
//! # 2. Config struct
//!
//! ```rust,ignore
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize, Default)]
//! pub struct Config {
//!     /// Required: path to the TSX page file relative to templates/.
//!     pub template_path: String,
//!     /// Optional: comma-separated external script URLs.
//!     #[serde(default)]
//!     pub load_scripts: String,
//! }
//! ```
//!
//! Rules:
//! - Field names are `snake_case` — they are the config JSON keys.
//! - Required fields have no `#[serde(default)]`.
//! - Optional fields have `#[serde(default)]` and use `Option<T>` or a defaultable type.
//! - Write a doc comment on every field — it becomes the `description` in `config_schema`.
//! - Platform-injected fields (markup, route, internal ids) live in the struct but must
//!   NOT appear in `dsl_flags` or `config_schema.required`.
//!
//! ---
//!
//! # 3. Config schema
//!
//! Populate `NodeDefinition::config_schema` by hand with a JSON Schema object:
//!
//! ```rust,ignore
//! config_schema: serde_json::json!({
//!     "type": "object",
//!     "required": ["template_path"],
//!     "properties": {
//!         "template_path": {
//!             "type": "string",
//!             "description": "TSX file relative to templates/. Example: pages/blog-home."
//!         },
//!         "load_scripts": {
//!             "type": "string",
//!             "description": "Comma-separated external script URLs. Each must match allow_list."
//!         }
//!     }
//! }),
//! ```
//!
//! Consumers of `config_schema`:
//! - **Studio UI** — renders form fields from `properties` without any TypeScript hardcoding.
//! - **Register validator** — checks incoming node configs before saving to disk.
//! - **LLM agent** — reads schema to know what to put in node config when authoring pipelines.
//! - **`/docs/node`** — exposes schema in the API contract document.
//!
//! ---
//!
//! # 4. DSL flags
//!
//! Every user-facing config field gets a [`DslFlag`](crate::pipeline::model::DslFlag) entry.
//!
//! ```rust,ignore
//! use crate::pipeline::model::{DslFlag, DslFlagKind};
//!
//! dsl_flags: vec![
//!     DslFlag {
//!         flag: "--template-path".to_string(),
//!         config_key: "template_path".to_string(),
//!         description: "TSX page file relative to templates/, e.g. pages/blog-home.".to_string(),
//!         kind: DslFlagKind::Scalar,
//!         required: true,
//!     },
//!     DslFlag {
//!         flag: "--load-scripts".to_string(),
//!         config_key: "load_scripts".to_string(),
//!         description: "Comma-separated external script URLs. Each must match allow_list.".to_string(),
//!         kind: DslFlagKind::CommaSeparatedList,
//!         required: false,
//!     },
//! ],
//! ```
//!
//! ## Universal parsing rule
//!
//! The DSL parser applies: `--flag-name` → `flag_name` (replace `-` with `_`) for ALL flags,
//! declared or not.  `dsl_flags` exists only for documentation, help text, required-flag
//! validation, and LLM context.  **New nodes work in the DSL without touching `parser.rs`.**
//!
//! ## Flag kind reference
//!
//! | `DslFlagKind` | DSL syntax | Config result |
//! |---|---|---|
//! | `Scalar` | `--key value` | `"value"` |
//! | `CommaSeparatedList` | `--key a,b,c` | `["a","b","c"]` |
//! | `Bool` | `--silent` (no value) | `true` |
//!
//! Body content (after ` -- `) is always captured separately as the node body string.
//!
//! ## Naming convention
//!
//! Flag name = `--kebab-case` of the `snake_case` config key.  Always.  No abbreviations,
//! no aliases.  `--credential-id` not `--cred`.  `--template-path` not `--template`.
//! This makes flags derivable from the schema property names — LLMs can infer them.
//!
//! ---
//!
//! # 5. Script bridge — callable from `n.script`
//!
//! Set `script_available: true` and provide a `NodeScriptBridge` to expose the node
//! as a global function inside the Deno sandbox of `n.script`:
//!
//! ```rust,ignore
//! script_available: true,
//! script_bridge: Some(NodeScriptBridge {
//!     name: "n.pg.query".to_string(),
//!     enabled: true,
//! }),
//! ```
//!
//! Pipeline authors can then call it from script:
//!
//! ```js
//! // inside n.script body
//! const rows = await n.pg.query({ credential_id: "main-db", query: "SELECT * FROM posts" });
//! return { posts: rows };
//! ```
//!
//! Only expose nodes whose config is safe to construct dynamically.  Nodes that mutate
//! state or emit side effects should be careful here.
//!
//! ---
//!
//! # 6. AI tool — invokable by LLM agents
//!
//! Set `ai_tool.registered: true` to register the node as a callable tool for Zebtune
//! and other LLM agents running inside the platform:
//!
//! ```rust,ignore
//! ai_tool: NodeAiToolDefinition {
//!     registered: true,
//!     tool_name: "query_database".to_string(),
//!     tool_description: "Run a SQL query against a project database connection and return rows.".to_string(),
//!     tool_input_schema: serde_json::json!({
//!         "type": "object",
//!         "required": ["credential_id", "query"],
//!         "properties": {
//!             "credential_id": { "type": "string", "description": "Database connection slug." },
//!             "query":         { "type": "string", "description": "SQL query to execute." }
//!         }
//!     }),
//! },
//! ```
//!
//! `tool_input_schema` is what the LLM sees — it may differ from `config_schema` if the
//! tool accepts a simpler or different interface than the full node config.
//!
//! ---
//!
//! # 7. `NodeHandler` implementation
//!
//! ```rust,ignore
//! use async_trait::async_trait;
//! use crate::pipeline::nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput};
//! use crate::pipeline::PipelineError;
//!
//! pub struct Node { config: Config }
//!
//! impl Node {
//!     pub fn new(config: Config) -> Self { Self { config } }
//! }
//!
//! #[async_trait]
//! impl NodeHandler for Node {
//!     fn kind(&self) -> &'static str { NODE_KIND }
//!
//!     fn input_pins(&self)  -> &'static [&'static str] { &["in"] }
//!     fn output_pins(&self) -> &'static [&'static str] { &["out"] }
//!
//!     async fn execute_async(
//!         &self,
//!         input: NodeExecutionInput,
//!     ) -> Result<NodeExecutionOutput, PipelineError> {
//!         // Read config: self.config.*
//!         // Read upstream payload: input.payload
//!         // Read owner/project: input.metadata.get("owner")
//!         // Read request id: input.metadata.get("request_id")
//!         Ok(NodeExecutionOutput {
//!             output_pins: vec!["out".to_string()],
//!             payload: serde_json::json!({ "result": "..." }),
//!             trace: vec![format!("n.my.node: done")],
//!         })
//!     }
//! }
//! ```
//!
//! Rules:
//! - Always include at least one `trace` entry — it shows in pipeline run logs.
//! - Emit `PipelineError::new("FW_MY_NODE_CODE", "message")` for recoverable errors.
//! - Use `output_pins` to control which downstream edges fire — only listed pins propagate.
//! - For conditional branching, emit `vec!["true"]` or `vec!["false"]` selectively.
//!
//! ---
//!
//! # 8. Registration
//!
//! Add your `definition()` call to `src/pipeline/nodes/basic/mod.rs`:
//!
//! ```rust,ignore
//! pub fn builtin_node_definitions() -> Vec<NodeDefinition> {
//!     vec![
//!         // ... existing nodes ...
//!         my_node::definition(),
//!     ]
//! }
//! ```
//!
//! And add the dispatch arm in `src/pipeline/engines/basic.rs` inside the `match node.kind`
//! block so the engine knows how to build and execute your node instance.
//!
//! ---
//!
//! # Complete example
//!
//! ```rust,ignore
//! //! `n.example.echo` — replaces the payload with `{ echo_tag }` from config.
//! //!
//! //! # Pipeline position
//! //! Middleware node. Always between a trigger and a terminal.
//! //!
//! //! # User-facing config
//! //! | Field | Type | Required | Description |
//! //! |---|---|---|---|
//! //! | `tag` | string | ✓ | Label output as `echo_tag` |
//! //!
//! //! # DSL
//! //! ```text
//! //! | n.trigger.webhook --path /ping
//! //! | n.example.echo --tag hello
//! //! ```
//!
//! use async_trait::async_trait;
//! use serde::{Deserialize, Serialize};
//! use serde_json::json;
//! use crate::pipeline::{PipelineError, NodeDefinition};
//! use crate::pipeline::model::{DslFlag, DslFlagKind};
//! use crate::pipeline::nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput};
//!
//! pub const NODE_KIND: &str = "n.example.echo";
//!
//! pub fn definition() -> NodeDefinition {
//!     NodeDefinition {
//!         kind: NODE_KIND.to_string(),
//!         title: "Echo".to_string(),
//!         description: "Replaces the payload with { echo_tag } from config.".to_string(),
//!         config_schema: json!({
//!             "type": "object",
//!             "required": ["tag"],
//!             "properties": {
//!                 "tag": { "type": "string", "description": "Value to output as echo_tag." }
//!             }
//!         }),
//!         input_schema:  json!({ "type": "object" }),
//!         output_schema: json!({ "type": "object", "properties": { "echo_tag": { "type": "string" } } }),
//!         input_pins:  vec!["in".to_string()],
//!         output_pins: vec!["out".to_string()],
//!         dsl_flags: vec![
//!             DslFlag {
//!                 flag: "--tag".to_string(),
//!                 config_key: "tag".to_string(),
//!                 description: "Value to output as echo_tag.".to_string(),
//!                 kind: DslFlagKind::Scalar,
//!                 required: true,
//!             },
//!         ],
//!         script_available: false,
//!         script_bridge: None,
//!         ai_tool: Default::default(),
//!     }
//! }
//!
//! #[derive(Debug, Clone, Serialize, Deserialize, Default)]
//! pub struct Config { pub tag: String }
//!
//! pub struct Node { config: Config }
//! impl Node { pub fn new(config: Config) -> Self { Self { config } } }
//!
//! #[async_trait]
//! impl NodeHandler for Node {
//!     fn kind(&self) -> &'static str { NODE_KIND }
//!     fn input_pins(&self)  -> &'static [&'static str] { &["in"] }
//!     fn output_pins(&self) -> &'static [&'static str] { &["out"] }
//!     async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
//!         // Replace payload — never merge into input.payload
//!         Ok(NodeExecutionOutput {
//!             output_pins: vec!["out".to_string()],
//!             payload: json!({ "echo_tag": self.config.tag }),
//!             trace: vec![format!("n.example.echo: tag={}", self.config.tag)],
//!         })
//!     }
//! }
//! ```

pub mod basic;
mod interface;

pub use interface::{NodeHandler, NodeExecutionInput, NodeExecutionOutput};

use crate::pipeline::model::{DslFlagKind, NodeDefinition};

/// Returns all built-in node definitions.
pub fn builtin_node_definitions() -> Vec<crate::pipeline::NodeDefinition> {
    basic::builtin_node_definitions()
}

fn format_node_definition_markdown(def: &NodeDefinition) -> String {
    let mut s = String::new();
    s.push_str(&format!("### `{}` — {}\n\n", def.kind, def.title));
    s.push_str(def.description.trim());
    s.push_str("\n\n");
    let ip = if def.input_pins.is_empty() {
        "*(none — trigger / entry)*".to_string()
    } else {
        format!("`{}`", def.input_pins.join("`, `"))
    };
    let op = if def.output_pins.is_empty() {
        "*(dynamic / graph-defined)*".to_string()
    } else {
        format!("`{}`", def.output_pins.join("`, `"))
    };
    s.push_str(&format!("- **Input pins:** {ip}\n- **Output pins:** {op}\n\n"));
    if !def.dsl_flags.is_empty() {
        s.push_str("| DSL flag | Config key | Required | Kind | Description |\n");
        s.push_str("|----------|------------|----------|------|-------------|\n");
        for f in &def.dsl_flags {
            let req = if f.required { "yes" } else { "no" };
            let kind_s = match f.kind {
                DslFlagKind::Scalar => "scalar",
                DslFlagKind::CommaSeparatedList => "comma-list",
                DslFlagKind::Bool => "bool",
                DslFlagKind::KeyValuePairs => "key-value-pairs",
            };
            let desc = f.description.replace('|', "\\|");
            s.push_str(&format!(
                "| `{}` | `{}` | {} | {} | {} |\n",
                f.flag, f.config_key, req, kind_s, desc
            ));
        }
        s.push_str("\n");
    }
    if !def.input_schema.is_null() {
        s.push_str("**Input payload (schema):**\n```json\n");
        if let Ok(pretty) = serde_json::to_string_pretty(&def.input_schema) {
            s.push_str(&pretty);
        }
        s.push_str("\n```\n\n");
    }
    if !def.output_schema.is_null() {
        s.push_str("**Output payload (schema):**\n```json\n");
        if let Ok(pretty) = serde_json::to_string_pretty(&def.output_schema) {
            s.push_str(&pretty);
        }
        s.push_str("\n```\n\n");
    }
    s.push_str(&format!(
        "**MCP:** `help_nodes` with `kind=\"{}\"` for this section only.\n\n",
        def.kind
    ));
    s
}

fn kind_query_matches_def(def: &NodeDefinition, query: &str) -> bool {
    let q = query.trim();
    if q.is_empty() {
        return false;
    }
    let kn = def.kind.to_lowercase();
    let qn = q.to_lowercase();
    if kn == qn {
        return true;
    }
    if qn.starts_with("n.") {
        return kn == qn;
    }
    if kn == format!("n.{qn}") {
        return true;
    }
    kn.strip_prefix("n.").is_some_and(|tail| tail == qn)
}

/// One node section — same source as [`builtin_node_definitions`].
pub fn node_markdown_by_kind_query(query: &str) -> Option<String> {
    basic::builtin_node_definitions()
        .into_iter()
        .find(|d| kind_query_matches_def(d, query))
        .map(|d| format_node_definition_markdown(&d))
}

/// Full catalog for `help_pipeline` / `help_nodes` — generated from Rust `definition()`, not hand-written markdown.
pub fn builtin_nodes_markdown_reference() -> String {
    let mut s = String::from(
        "## Node kinds (live — from `builtin_node_definitions()`)\n\n\
         This block matches the pipeline editor / node API: titles, descriptions, pins, DSL flags, and input/output schemas.\n\n\
         - **Full catalog:** `help_nodes` with no `kind` (same as this section).\n\
         - **One kind:** `help_nodes` with `kind=\"n.script\"` (or `script`, `trigger.webhook`, etc.).\n\n\
         ---\n\n",
    );
    for def in basic::builtin_node_definitions() {
        s.push_str(&format_node_definition_markdown(&def));
        s.push_str("---\n\n");
    }
    s
}

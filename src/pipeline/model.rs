//! Pipeline domain model — the complete type system for graph execution, node contracts, and
//! runtime context.
//!
//! # Overview
//!
//! A **pipeline** is a directed graph of **nodes** connected by **edges**.
//! Each edge links one node's output pin to another node's input pin.  The engine performs a
//! BFS queue traversal, executing nodes in arrival order and forwarding the output
//! **payload** (a `serde_json::Value`) downstream through the edges.
//! Cycles are permitted — a node can be revisited when an edge points back to an earlier node
//! (e.g. a conditional loop via `n.logic.if`'s `false` pin).
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  PipelineGraph                                                  │
//! │                                                                 │
//! │  ┌───────────────────┐    edge: n0/out → n1/in                 │
//! │  │ PipelineNode (n0) │ ──────────────────────────────────────► │
//! │  │  kind: n.trigger  │                                         │
//! │  │  config: { path } │    ┌─────────────────────┐             │
//! │  └───────────────────┘    │ PipelineNode (n1)   │             │
//! │                           │  kind: n.pg.query   │             │
//! │                           │  config: { query }  │             │
//! │                           └──────────┬──────────┘             │
//! │                                      │ edge: n1/out → n2/in   │
//! │                                      ▼                        │
//! │                           ┌─────────────────────┐             │
//! │                           │ PipelineNode (n2)   │             │
//! │                           │  kind: n.web.render │             │
//! │                           │  config: { tmpl }   │             │
//! │                           └─────────────────────┘             │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Three Separate Concerns
//!
//! This module contains three groups of types that must not be confused:
//!
//! | Group | Types | Purpose |
//! |-------|-------|---------|
//! | **Graph** | [`PipelineGraph`], [`PipelineNode`], [`PipelineEdge`] | Persisted pipeline definition — what gets saved to `*.zf.json` and loaded at runtime |
//! | **Definition** | [`NodeDefinition`], [`NodeScriptBridge`], [`NodeAiToolDefinition`] | Static contract describing what a node kind *is* — its schemas, pins, and capabilities |
//! | **Runtime** | [`PipelineContext`], [`PipelineOutput`], [`PipelineError`], [`ExecuteOptions`] | Values produced and consumed during a single pipeline execution run |
//!
//! # Node Definition vs Node Instance
//!
//! **[`NodeDefinition`]** is the *kind-level* contract — it is the same for every instance of
//! `n.pg.query` across all pipelines. It is declared once per node kind via a `definition()`
//! function in each node module (e.g. `pg_query::definition()`), registered into a catalog,
//! and served at `/docs/node` via [`NodeContractDocument`].
//!
//! **[`PipelineNode`]** is a *graph-level instance* — a specific occurrence of a node kind
//! inside one pipeline, identified by a unique `id`, carrying its own `config` blob.  Many
//! `PipelineNode` instances can share the same `kind` string but have different configs.
//!
//! ```text
//!                  NodeDefinition (kind = "n.pg.query")
//!                  ├── config_schema: { credential_id: required, query: required, ... }
//!                  ├── input_schema:  { type: object }
//!                  ├── output_schema: { rows: array } | { affected_rows: integer }
//!                  ├── input_pins:    ["in"]
//!                  └── output_pins:  ["out"]
//!
//!                           ↑ validates config against config_schema at register time
//!
//!   PipelineNode { id: "n1", kind: "n.pg.query", config: { credential_id: "pg-main", query: "SELECT ..." } }
//!   PipelineNode { id: "n4", kind: "n.pg.query", config: { credential_id: "pg-replica", query: "SELECT ..." } }
//! ```
//!
//! # Config Schema
//!
//! [`NodeDefinition::config_schema`] is the single source of truth for what static
//! configuration a node kind accepts.  It is a JSON Schema object derived from each node's
//! typed `Config` struct via `schemars::schema_for!` (available through the `rmcp::schemars`
//! re-export).
//!
//! Adding `#[derive(JsonSchema)]` to a node's `Config` struct and calling
//! `serde_json::to_value(schemars::schema_for!(Config))` in its `definition()` function
//! provides:
//!
//! 1. **Validation at register time** — `config` blobs on incoming [`PipelineNode`]s can be
//!    checked against the schema before a pipeline is saved to disk.  Bad config fails fast
//!    rather than at execution time.
//! 2. **UI dialog auto-generation** — the pipeline editor reads `config_schema.properties`
//!    to render the correct field types, labels, and required markers without hardcoding them
//!    in TypeScript.
//! 3. **DSL flag derivation** — the DSL parser can map `--flag` names to `config` JSON keys
//!    by reading schema property names, eliminating hardcoded mapping tables.
//! 4. **API docs** — `config_schema` is included in [`NodeContractItem`] and surfaced at
//!    `/docs/node` so external tooling can discover node capabilities programmatically.
//!
//! # Pin System
//!
//! Nodes communicate through named **pins**.  An output pin on one node connects to an input
//! pin on another via a [`PipelineEdge`].  The flowing value between pins is a
//! `serde_json::Value` (the **payload**).
//!
//! ```text
//!   NodeA { output_pins: ["out", "error"] }
//!       │
//!       │  PipelineEdge { from_node: "A", from_pin: "out", to_node: "B", to_pin: "in" }
//!       ▼
//!   NodeB { input_pins: ["in"] }
//! ```
//!
//! Pin names are declared in [`NodeDefinition`] (the kind-level contract) and instantiated in
//! [`PipelineNode`] (the graph instance).  Trigger nodes have no input pins.  Terminal nodes
//! need not have output pins — their payload becomes the HTTP response.
//!
//! # Authoring a New Node
//!
//! Every node module must expose:
//!
//! 1. A `pub fn definition() -> NodeDefinition` that populates all contract fields.
//! 2. A `pub struct Config` with `#[derive(Serialize, Deserialize, JsonSchema)]` documenting
//!    accepted config fields via doc comments.
//! 3. A `pub struct Node` that implements [`crate::pipeline::nodes::NodeHandler`].
//! 4. Register the `definition()` call in `pipeline/nodes/basic/mod.rs →
//!    builtin_node_definitions()`.
//!
//! ```rust,ignore
//! // my_node.rs
//! use schemars::JsonSchema;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
//! pub struct Config {
//!     /// Required: the thing the node needs.
//!     pub my_field: String,
//!     /// Optional override.
//!     #[serde(default)]
//!     pub optional_field: Option<String>,
//! }
//!
//! pub fn definition() -> NodeDefinition {
//!     NodeDefinition {
//!         kind: "n.my.node".to_string(),
//!         title: "My Node".to_string(),
//!         description: "Does the thing.".to_string(),
//!         config_schema: serde_json::to_value(schemars::schema_for!(Config)).unwrap_or_default(),
//!         input_schema: serde_json::json!({ "type": "object" }),
//!         output_schema: serde_json::json!({ "type": "object", "properties": { "result": { "type": "string" } } }),
//!         input_pins: vec!["in".to_string()],
//!         output_pins: vec!["out".to_string()],
//!         ..Default::default()
//!     }
//! }
//! ```
//!
//! # Contract Document
//!
//! At startup, [`crate::pipeline::nodes::basic::builtin_node_definitions()`] returns all
//! [`NodeDefinition`]s.  The platform converts them to [`NodeContractItem`]s via
//! `From<NodeDefinition>` and assembles a [`NodeContractDocument`] served at `/docs/node`.
//! This endpoint is the programmatic API for everything that needs to know about node
//! capabilities: UI, DSL tooling, LLM assistant, external integrations.
//!
//! # Payload Flow
//!
//! During execution, the engine feeds an initial payload (from the HTTP request) into the
//! trigger node.  Each node receives an **input payload** (the output of the upstream node
//! that fired its input pin) and produces an **output payload** forwarded downstream.
//! [`NodeDefinition::input_schema`] and [`NodeDefinition::output_schema`] document these
//! payload shapes — they describe the *flowing data*, not the static config.
//!
//! The trigger node is special: it receives the raw request payload and passes it through
//! unchanged (or enriched).  Downstream nodes transform the payload until a terminal node
//! (e.g. `n.web.render`) produces the final HTTP response.

use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// How a DSL flag value is interpreted by the parser.
///
/// The universal transformation rule `--flag-name` → `flag_name` (replace `-` with `_`)
/// applies to ALL flags automatically.  `DslFlagKind` controls only how the *value* is parsed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DslFlagKind {
    /// Single string value consumed from the next token.
    ///
    /// `--template-path pages/blog-home` → `config["template_path"] = "pages/blog-home"`
    #[default]
    Scalar,
    /// Comma-separated string split into a JSON array.
    ///
    /// `--load-scripts https://a.com,https://b.com` → `config["load_scripts"] = ["https://a.com","https://b.com"]`
    CommaSeparatedList,
    /// Presence flag — no value consumed; sets config key to `true`.
    ///
    /// `--silent` → `config["silent"] = true`
    Bool,
}

/// One DSL flag declaration owned by a [`NodeDefinition`].
///
/// Each node declares the DSL flags it accepts.  The parser uses these for:
/// - **Help text**: `pipeline --help n.web.render` lists all flags with descriptions.
/// - **Required validation**: flags with `required: true` must be present when registering.
/// - **LLM context**: surfaced in `/docs/node` so agents know exactly what flags to emit.
///
/// The parser does NOT need this list to parse — it applies the universal rule
/// `--flag-name` → `flag_name` for any undeclared flag too.  Declarations exist for
/// documentation and validation only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DslFlag {
    /// CLI flag name including `--` prefix (e.g. `"--template-path"`).
    ///
    /// Convention: always `--kebab-case` matching the `snake_case` config key
    /// via the universal rule (replace `-` with `_`).
    pub flag: String,
    /// Config JSON key this flag writes to (e.g. `"template_path"`).
    ///
    /// Must match the corresponding field in the node's `Config` struct exactly.
    pub config_key: String,
    /// Human-readable description for help text and LLM context.
    ///
    /// Write this as a single sentence starting with a verb:
    /// "Path to the TSX page file relative to templates/."
    pub description: String,
    /// How the flag value is parsed.
    pub kind: DslFlagKind,
    /// Whether this flag must be present for the node to be valid.
    pub required: bool,
}

/// A complete pipeline graph ready for execution.
///
/// Persisted as `repo/pipelines/<name>.zf.json`.  Loaded by
/// [`crate::platform::services::PipelineRuntimeService`] at server startup and on demand.
///
/// # Execution entry point
///
/// The engine starts traversal from [`entry_nodes`](PipelineGraph::entry_nodes).  If the
/// list is empty, the engine falls back to nodes that have no incoming edges (i.e. nodes
/// with no input pins — trigger nodes).
///
/// # JSON shape
///
/// ```json
/// {
///   "kind": "zebflow.pipeline",
///   "version": "0.1",
///   "id": "blog-home",
///   "nodes": [ ... ],
///   "edges": [ ... ]
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineGraph {
    /// Marker for the graph format; always `"zebflow.pipeline"`.
    #[serde(default = "default_pipeline_kind")]
    pub kind: String,
    /// Graph schema version; always `"0.1"` for now.
    #[serde(default = "default_pipeline_version")]
    pub version: String,
    /// Unique pipeline id — matches the filename stem of the `.zf.json` file.
    pub id: String,
    /// Explicit entry node ids.  If empty the engine uses nodes with no incoming edges.
    #[serde(default)]
    pub entry_nodes: Vec<String>,
    /// All nodes in this graph.  Order does not matter — the engine performs topological
    /// sort at execution time.
    pub nodes: Vec<PipelineNode>,
    /// All directed edges connecting output pins to input pins.
    pub edges: Vec<PipelineEdge>,
}

/// One node instance inside a [`PipelineGraph`].
///
/// This is the *instance* — not the kind-level contract.  Many nodes with the same `kind`
/// can exist in the same graph or across different graphs, each with different `config`.
///
/// # Config vs Definition
///
/// `config` is a raw `serde_json::Value` blob.  The engine deserializes it into the node
/// kind's typed `Config` struct at build/compile time.  The shape is defined by
/// [`NodeDefinition::config_schema`].
///
/// # Pins
///
/// `input_pins` and `output_pins` are stored per instance to allow the graph serialization
/// format to be self-describing.  They must be consistent with what
/// [`NodeDefinition::input_pins`] and [`NodeDefinition::output_pins`] declare for the kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineNode {
    /// Unique node id within this graph (e.g. `"n0"`, `"trigger"`, `"render_blog"`).
    pub id: String,
    /// Kind identifier — must match a registered node kind (e.g. `"n.web.render"`).
    pub kind: String,
    /// Input pin names for this instance.  Alias: `"inputs"` in JSON.
    #[serde(default, alias = "inputs")]
    pub input_pins: Vec<String>,
    /// Output pin names for this instance.  Alias: `"outputs"` in JSON.
    #[serde(default, alias = "outputs")]
    pub output_pins: Vec<String>,
    /// Node-specific configuration blob.  Shape is defined by the kind's
    /// [`NodeDefinition::config_schema`].  Validated at register time; deserialized into
    /// a typed `Config` struct at engine build time.
    #[serde(default)]
    pub config: Value,
}

/// A directed connection from one node's output pin to another node's input pin.
///
/// Together, all edges in a [`PipelineGraph`] form the DAG.  The engine follows edges
/// to determine which node fires next and what payload it receives.
///
/// # Example
///
/// ```json
/// { "from_node": "trigger", "from_pin": "out", "to_node": "query", "to_pin": "in" }
/// ```
///
/// The `from`/`to` aliases allow a shorter JSON representation in `.zf.json` files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineEdge {
    /// Source node id.  Alias: `"from"` in JSON.
    #[serde(alias = "from")]
    pub from_node: String,
    /// Output pin name on the source node (e.g. `"out"`, `"true"`, `"error"`).
    pub from_pin: String,
    /// Target node id.  Alias: `"to"` in JSON.
    #[serde(alias = "to")]
    pub to_node: String,
    /// Input pin name on the target node (almost always `"in"`).
    pub to_pin: String,
}

/// Declares that this node kind can be invoked from inside an `n.script` node.
///
/// When `enabled = true` the language engine exposes the node's capability as a global
/// function under `bridge_name` inside the Deno sandbox.  Pipeline authors can call it
/// directly from JavaScript without adding a separate graph node.
///
/// # Example
///
/// ```js
/// // inside n.script source
/// const rows = await n.pg.query({ credential_id: "main-db", query: "SELECT * FROM posts" });
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct NodeScriptBridge {
    /// Function name exposed in the script sandbox (e.g. `"n.pg.query"`).
    pub name: String,
    /// Whether this bridge is active in the current runtime.  Disabled bridges are visible
    /// in the contract document but not callable from scripts.
    #[serde(default)]
    pub enabled: bool,
}

/// Registers this node kind as a callable AI tool for LLM-based agents.
///
/// When `registered = true`, the assistant can invoke this node's capability by name
/// during an agentic pipeline run (e.g. Zebtune calling `n.pg.query` to answer a question).
/// `tool_input_schema` is a JSON Schema object the LLM uses to form valid tool arguments.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NodeAiToolDefinition {
    /// Whether this node is exposed as an AI tool.
    #[serde(default)]
    pub registered: bool,
    /// Stable tool name/id used by the LLM (e.g. `"query_database"`).
    #[serde(default)]
    pub tool_name: String,
    /// Human-readable description fed to the LLM to explain what the tool does.
    #[serde(default)]
    pub tool_description: String,
    /// JSON Schema for the tool's input arguments as seen by the LLM.
    #[serde(default)]
    pub tool_input_schema: Value,
}

/// The canonical contract for one node kind.
///
/// **One `NodeDefinition` per node kind** — not per instance.  Every node module exposes a
/// `pub fn definition() -> NodeDefinition` and registers it via
/// `builtin_node_definitions()`.
///
/// # Three Schemas
///
/// A `NodeDefinition` carries three independent JSON Schema objects.  Understanding the
/// difference is critical:
///
/// | Field | Describes | Populated by |
/// |-------|-----------|--------------|
/// | `config_schema` | Static fields in the node's `Config` struct | `schemars::schema_for!(Config)` |
/// | `input_schema`  | Payload shape this node *consumes* from upstream | Hand-written in `definition()` |
/// | `output_schema` | Payload shape this node *produces* downstream   | Hand-written in `definition()` |
///
/// `config_schema` is the authoritative source for what can be put in
/// [`PipelineNode::config`].  Downstream consumers (UI dialog renderer, DSL parser, register
/// validator) all read this schema rather than hardcoding field knowledge.
///
/// `input_schema` and `output_schema` document the *flowing payload* — the `serde_json::Value`
/// passed between nodes at runtime.  They are informational for now (not enforced at runtime)
/// but drive UI tooltips and LLM context.
///
/// # Pin Declarations
///
/// `input_pins` and `output_pins` list the pin names this kind exposes.  Trigger nodes have
/// empty `input_pins`.  Logic nodes like `n.logic.branch` have dynamic `output_pins` (empty
/// here, populated per instance in the graph).
///
/// # Usage Dimensions
///
/// A node kind can be usable in three ways simultaneously:
/// 1. As a **graph node** — always true for any registered kind.
/// 2. As a **script bridge** — callable from `n.script` Deno sandbox (see [`NodeScriptBridge`]).
/// 3. As an **AI tool** — invokable by LLM agents (see [`NodeAiToolDefinition`]).
///
/// The [`NodeUsageMatrix`] in [`NodeContractItem`] captures all three dimensions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NodeDefinition {
    /// Stable kind id — must be unique across all registered nodes.
    /// Convention: `n.<category>.<action>` (e.g. `n.pg.query`, `n.web.render`).
    pub kind: String,
    /// Short display title for UI catalogs and tooltips (e.g. `"Postgres Query"`).
    pub title: String,
    /// Full behavior description for docs, UI dialogs, and LLM context.
    pub description: String,
    /// JSON Schema for the node's static `Config` struct.
    ///
    /// **Populate this via `schemars::schema_for!(Config)`** (available as
    /// `rmcp::schemars::schema_for!`).  Adding `#[derive(JsonSchema)]` to the `Config`
    /// struct and annotating fields with doc comments produces a schema with `required`,
    /// `properties`, `description`, and `type` automatically.
    ///
    /// Consumers:
    /// - **Validation**: `register_pipeline` checks each [`PipelineNode::config`] against
    ///   this schema before saving — fail fast before execution.
    /// - **UI**: The pipeline editor reads `properties` to render form fields without any
    ///   TypeScript-side hardcoding.
    /// - **DSL**: The parser maps `--flag` names to config keys using property names.
    /// - **API**: Exposed in [`NodeContractItem`] at `/docs/node`.
    ///
    /// Defaults to `Value::Null` (no schema declared yet) — treated as "accept anything".
    #[serde(default)]
    pub config_schema: Value,
    /// JSON Schema for the payload this node *reads* from the upstream output.
    ///
    /// Informational — not enforced at runtime.  Documents what fields the node expects
    /// to find in `input.payload` so pipeline authors know what upstream nodes must produce.
    #[serde(default)]
    pub input_schema: Value,
    /// JSON Schema for the payload this node *writes* to its output pins.
    ///
    /// Informational — not enforced at runtime.  Documents what fields downstream nodes
    /// can rely on after this node runs.
    #[serde(default)]
    pub output_schema: Value,
    /// Input pin names declared by this kind.  Empty for trigger nodes (no upstream).
    /// Almost always `["in"]` for processing nodes.
    #[serde(default)]
    pub input_pins: Vec<String>,
    /// Output pin names declared by this kind.  Common values: `["out"]`, `["out", "error"]`,
    /// `["true", "false"]`.  Empty for dynamic-pin nodes like `n.logic.branch` — pins are
    /// defined per instance in the graph.
    #[serde(default)]
    pub output_pins: Vec<String>,
    /// Whether this node kind can be called from inside an `n.script` Deno sandbox.
    #[serde(default)]
    pub script_available: bool,
    /// Script bridge metadata.  Only meaningful when `script_available = true`.
    #[serde(default)]
    pub script_bridge: Option<NodeScriptBridge>,
    /// AI tool registration.  Only meaningful when `ai_tool.registered = true`.
    #[serde(default)]
    pub ai_tool: NodeAiToolDefinition,
    /// DSL flag declarations for this node kind.
    ///
    /// Each entry documents one `--flag-name` → `config_key` mapping.  The parser applies
    /// the universal rule (`-` → `_`) for any flag not listed here — declarations exist
    /// for help text, required-flag validation, and LLM context surfaced at `/docs/node`.
    ///
    /// # Convention
    ///
    /// Declare every user-facing flag.  Internal/platform-injected fields (e.g. `markup`,
    /// `route`) must NOT appear here — they are not settable by users or agents.
    #[serde(default)]
    pub dsl_flags: Vec<DslFlag>,
}

/// Script bridge capability as exposed in a [`NodeUsageMatrix`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NodeScriptUsageContract {
    /// Whether this node can be called from `n.script`.
    pub available: bool,
    /// Function name exposed in the Deno sandbox.
    #[serde(default)]
    pub bridge_name: String,
    /// Whether the bridge is active in the current runtime build.
    #[serde(default)]
    pub enabled: bool,
}

/// AI tool capability as exposed in a [`NodeUsageMatrix`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NodeToolUsageContract {
    /// Whether this node is registered as an AI tool.
    pub registered: bool,
    /// Tool name/id.
    #[serde(default)]
    pub tool_name: String,
    /// Description fed to the LLM.
    #[serde(default)]
    pub tool_description: String,
    /// JSON Schema for tool arguments as seen by the LLM.
    #[serde(default)]
    pub tool_input_schema: Value,
}

/// Summary of where and how a node kind can be used.
///
/// A node is always usable as a pipeline graph node.  The matrix additionally records
/// whether it is callable from script and/or invokable by AI agents.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NodeUsageMatrix {
    /// Always `true` — every registered kind is available as a graph node.
    pub pipeline_node: bool,
    /// Script bridge availability.
    pub script_usage: NodeScriptUsageContract,
    /// AI tool availability.
    pub tool_usage: NodeToolUsageContract,
}

/// One entry in the node contract document, ready for API serialization.
///
/// Produced from a [`NodeDefinition`] via `From<NodeDefinition> for NodeContractItem`.
/// Carries all three schemas plus the [`NodeUsageMatrix`].
///
/// Served inside [`NodeContractDocument`] at `GET /docs/node`.  External consumers
/// (pipeline editor UI, DSL tooling, LLM assistant, external integrations) use this
/// document to discover node capabilities without reading Rust source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NodeContractItem {
    /// Stable kind id.
    pub kind: String,
    /// Display title.
    pub title: String,
    /// Full behavior description.
    pub description: String,
    /// JSON Schema for the node's static config.  See [`NodeDefinition::config_schema`].
    #[serde(default)]
    pub config_schema: Value,
    /// JSON Schema for the consumed payload.
    #[serde(default)]
    pub input_schema: Value,
    /// JSON Schema for the produced payload.
    #[serde(default)]
    pub output_schema: Value,
    /// Declared input pin names.
    #[serde(default)]
    pub input_pins: Vec<String>,
    /// Declared output pin names.
    #[serde(default)]
    pub output_pins: Vec<String>,
    /// Where this node kind can be used.
    pub usage_matrix: NodeUsageMatrix,
    /// DSL flag declarations — same as [`NodeDefinition::dsl_flags`].
    ///
    /// Surfaced here so API consumers (UI, LLM, external tooling) can discover
    /// exactly what `--flags` a node accepts without reading Rust source.
    #[serde(default)]
    pub dsl_flags: Vec<DslFlag>,
}

/// Root response envelope for `GET /docs/node`.
///
/// Contains all registered node contracts in a single document.  Clients should check
/// `ok = true` before processing `items`.  `schema_version` can be used to detect
/// breaking changes to the contract format.
///
/// # Assembling the document
///
/// ```rust,ignore
/// let items: Vec<NodeContractItem> = builtin_node_definitions()
///     .into_iter()
///     .map(NodeContractItem::from)
///     .collect();
/// let doc = NodeContractDocument {
///     ok: true,
///     schema_version: "1",
///     source: "builtin",
///     items,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NodeContractDocument {
    /// `true` when the document was assembled successfully.
    pub ok: bool,
    /// Monotonic version of the contract schema format.  Increment on breaking changes.
    pub schema_version: &'static str,
    /// Source tag for traceability (e.g. `"builtin"`).
    pub source: &'static str,
    /// All node contract entries, sorted by kind.
    #[serde(default)]
    pub items: Vec<NodeContractItem>,
}

/// Converts a kind-level [`NodeDefinition`] into an API-facing [`NodeContractItem`].
///
/// This is the main path for building the `/docs/node` response.  It flattens the
/// `script_bridge` and `ai_tool` metadata into the flat [`NodeUsageMatrix`] structure.
impl From<NodeDefinition> for NodeContractItem {
    fn from(value: NodeDefinition) -> Self {
        let (bridge_name, bridge_enabled) = value
            .script_bridge
            .as_ref()
            .map(|bridge| (bridge.name.clone(), bridge.enabled))
            .unwrap_or_else(|| (String::new(), false));
        Self {
            kind: value.kind,
            title: value.title,
            description: value.description,
            config_schema: value.config_schema,
            input_schema: value.input_schema,
            output_schema: value.output_schema,
            input_pins: value.input_pins,
            output_pins: value.output_pins,
            dsl_flags: value.dsl_flags,
            usage_matrix: NodeUsageMatrix {
                pipeline_node: true,
                script_usage: NodeScriptUsageContract {
                    available: value.script_available,
                    bridge_name,
                    enabled: bridge_enabled,
                },
                tool_usage: NodeToolUsageContract {
                    registered: value.ai_tool.registered,
                    tool_name: value.ai_tool.tool_name,
                    tool_description: value.ai_tool.tool_description,
                    tool_input_schema: value.ai_tool.tool_input_schema,
                },
            },
        }
    }
}

/// A streaming step event emitted by long-running nodes (e.g. `n.ai.zebtune`).
///
/// Sent through [`ExecuteOptions::step_tx`] so the platform can forward individual
/// reasoning steps to the client via SSE before the final output is ready.
///
/// `step` is a short machine-readable tag (e.g. `"thinking"`, `"tool_call"`,
/// `"tool_result"`, `"final"`).  `at` is a wall-clock timestamp or elapsed time string.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepEvent {
    /// Short machine-readable step tag.
    pub step: String,
    /// Human-readable description of what happened at this step.
    pub description: String,
    /// Timestamp or elapsed time string (e.g. `"00:00:03"`).
    pub at: String,
}

/// Per-execution options passed to the pipeline engine.
///
/// Currently only carries an optional SSE step channel.  Pass `Default::default()` for
/// standard non-streaming execution.
#[derive(Debug, Default)]
pub struct ExecuteOptions {
    /// When set, nodes that support streaming (e.g. `n.ai.zebtune`) send each step
    /// event here.  The platform can forward these to the HTTP client via SSE while the
    /// pipeline is still running.
    pub step_tx: Option<tokio::sync::mpsc::UnboundedSender<StepEvent>>,
}

/// Immutable execution context passed to every node during a pipeline run.
///
/// Contains the tenant/project scope and a unique request id for tracing.  The `input`
/// field carries the original trigger payload — nodes should read their flowing payload
/// from `NodeExecutionInput::payload`, not from here.
///
/// `route` carries the inbound HTTP request path for webhook-triggered pipelines.
/// It is injected into every node's `metadata` under the key `"route"` so that
/// downstream nodes — in particular `n.web.render` — can access the real request
/// path without reading it from a potentially-transformed payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineContext {
    /// Owner / tenant id (e.g. `"superadmin"`).
    pub owner: String,
    /// Project id (e.g. `"default"`).
    pub project: String,
    /// Pipeline id matching [`PipelineGraph::id`].
    pub pipeline: String,
    /// Unique id for this execution run — used in trace entries and logs.
    pub request_id: String,
    /// Inbound HTTP request path for webhook-triggered pipelines (e.g. `"/blog/my-post"`).
    ///
    /// Populated by the webhook handler from the matched request URI.  Empty string for
    /// non-webhook triggers (schedule, WS, manual).  Injected into node metadata as
    /// `"route"` so `n.web.render` can build a correct [`crate::rwe::RenderContext`]
    /// without requiring a `route` field on the node config itself.
    #[serde(default)]
    pub route: String,
    /// The raw trigger payload that started this pipeline run.  Passed into the entry
    /// node and forwarded downstream through edges.
    pub input: Value,
}

/// Final output of a completed pipeline execution.
///
/// `value` is the payload produced by the last (terminal) node.  For `n.web.render` this
/// contains `{ html, compiled_scripts, hydration_payload }`.  For all other terminal nodes
/// it is whatever the node returned.
///
/// `trace` is a flat ordered list of trace strings emitted by each node in execution order,
/// useful for debugging slow or failed runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineOutput {
    /// Final output payload from the terminal node.
    pub value: Value,
    /// Ordered trace entries from all nodes that executed during this run.
    pub trace: Vec<String>,
}

/// A typed error produced at any point during pipeline execution.
///
/// `code` is a stable SCREAMING_SNAKE_CASE string prefixed by the node kind
/// (e.g. `"FW_NODE_PG_CONFIG"`, `"FW_NODE_WEB_RENDER_COMPILE"`).  It is safe to
/// match on in tests and error handlers.  `message` is the human-readable detail.
///
/// # Stable error code conventions
///
/// | Prefix | Meaning |
/// |--------|---------|
/// | `FW_NODE_<KIND>_CONFIG` | Node config validation failed at build time |
/// | `FW_NODE_<KIND>_COMPILE` | Node compile/build phase failed |
/// | `FW_NODE_<KIND>_RUN` | Node execution failed at runtime |
/// | `FW_NODE_<KIND>_CREDENTIAL*` | Credential lookup or kind mismatch |
/// | `FW_NODE_<KIND>_INPUT_PIN` | Unexpected input pin name |
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineError {
    /// Stable error code.  Never changes across versions for a given failure mode.
    pub code: &'static str,
    /// Human-readable error detail.
    pub message: String,
}

impl PipelineError {
    /// Constructs a new error with a stable code and a descriptive message.
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl Display for PipelineError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for PipelineError {}

fn default_pipeline_kind() -> String {
    "zebflow.pipeline".to_string()
}

fn default_pipeline_version() -> String {
    "0.1".to_string()
}

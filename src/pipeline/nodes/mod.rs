//! Pipeline node interfaces and built-in node implementations.
//!
//! This module is the **single source of truth** for how nodes are authored in Zebflow.
//! Every built-in node lives under [`basic`], in the folder of its DSL family:
//! `fs.file.put` is `basic/fs/put.rs`, `kv.entry.get` is `basic/kv/get.rs`, and a
//! family with one node keeps it in that folder's `mod.rs`. Helpers several
//! families use live in [`shared`]. This doc is the living specification —
//! read it before creating or modifying any node.
//!
//! ---
//!
//! # Node Anatomy
//!
//! A complete node module exposes exactly four things:
//!
//! ```text
//! basic/<family>/<node>.rs
//! ├── pub fn definition() -> NodeDefinition   ← kind-level contract (schemas, flags, docs)
//! ├── pub struct Config { ... }               ← typed config with JsonSchema derive
//! ├── pub struct Node { ... }                 ← runtime instance
//! └── impl NodeHandler for Node               ← execution logic
//! ```
//!
//! Then add it to the family's `definitions()` in `basic/<family>/mod.rs`, which
//! `basic/mod.rs → builtin_node_definitions()` reads.  That's it.
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
//!         kind: NODE_KIND.to_string(),           // "family.noun.verb"
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
//! - **Project-level settings** if the node is affected by `zebflow.yaml` settings.
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
//!     /// Optional: external script URLs (`--script`, repeated).
//!     #[serde(default)]
//!     pub scripts: Vec<String>,
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
//!         "scripts": {
//!             "type": "array",
//!             "description": "External script URLs. Each must match allow_list."
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
//!, ..Default::default()     },
//!     DslFlag {
//!         flag: "--script".to_string(),
//!         config_key: "scripts".to_string(),
//!         description: "An external script URL; repeat for several. Each must match allow_list.".to_string(),
//!         kind: DslFlagKind::RepeatedList,
//!         required: false,
//!, ..Default::default()     },
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
//! | `RepeatedList` | `--key a --key b` (a comma stays inside its value) | `["a","b"]` |
//! | `Bool` | `--batch` (no value) | `true` |
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
//! # 5. Script bridge — declared, and built by nothing
//!
//! Write `script_available: false` and `script_bridge: None`.  Every node does.
//!
//! A script cannot reach a node handler.  The `n` object handed to a `javascript.script.run` body
//! is assembled by `build_capabilities_expr` in `language/engines/deno_sandbox/pool.rs`
//! from pure `time` and `math` helpers; the sandbox exposes two host ops, neither of
//! which dispatches a node, and hides `Deno.core` from user code.  Declaring `true`
//! would grant nothing and would put a false "script access" claim in the node
//! catalog, which is why no definition does.
//!
//! The two fields survive because `NodeDefinition` is a frozen `zebflow.com/v1`
//! contract; see `docs/contracts/kinds/node-definition/README.md`.  Building the bridge
//! is a feature, and the egress rules in `docs/contracts/confinement.md` §3 have to
//! cover it before it exists.
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
//!             trace: vec![format!("example.thing.run: done")],
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
//! Declare the file and add your `definition()` call to the family list in
//! `src/pipeline/nodes/basic/<family>/mod.rs` (a new family is a new folder,
//! declared in `basic/mod.rs` and appended to `family_definitions()` there):
//!
//! ```rust,ignore
//! pub mod my_node;
//!
//! pub fn definitions() -> Vec<NodeDefinition> {
//!     vec![
//!         // ... the family's other nodes ...
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
//! //! `example.message.echo` — passes the upstream payload through unchanged with a tag.
//! //!
//! //! # Pipeline position
//! //! Middleware node. Always between a trigger and a terminal.
//! //!
//! //! # User-facing config
//! //! | Field | Type | Required | Description |
//! //! |---|---|---|---|
//! //! | `tag` | string | ✓ | Label injected into the payload as `echo_tag` |
//! //!
//! //! # DSL
//! //! ```text
//! //! | trigger.webhook --route /ping
//! //! | example.message.echo --tag hello
//! //! ```
//!
//! use async_trait::async_trait;
//! use serde::{Deserialize, Serialize};
//! use serde_json::json;
//! use crate::pipeline::{PipelineError, NodeDefinition};
//! use crate::pipeline::model::{DslFlag, DslFlagKind};
//! use crate::pipeline::nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput};
//!
//! pub const NODE_KIND: &str = "example.message.echo";
//!
//! pub fn definition() -> NodeDefinition {
//!     NodeDefinition {
//!         kind: NODE_KIND.to_string(),
//!         title: "Echo".to_string(),
//!         description: "Passes payload through, injecting echo_tag from config.".to_string(),
//!         config_schema: json!({
//!             "type": "object",
//!             "required": ["tag"],
//!             "properties": {
//!                 "tag": { "type": "string", "description": "Label to inject as echo_tag." }
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
//!                 description: "Label injected into the output payload as echo_tag.".to_string(),
//!                 kind: DslFlagKind::Scalar,
//!                 required: true,
//!, ..Default::default()             },
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
//!         let mut payload = input.payload;
//!         payload["echo_tag"] = json!(self.config.tag.clone());
//!         Ok(NodeExecutionOutput {
//!             output_pins: vec!["out".to_string()],
//!             payload,
//!             trace: vec![format!("example.message.echo: tag={}", self.config.tag)],
//!         })
//!     }
//! }
//! ```

pub mod basic;
mod conventions;
mod interface;
pub mod shared;

pub use interface::{NodeExecutionInput, NodeExecutionOutput, NodeHandler};

use std::collections::HashSet;

use crate::pipeline::model::{
    DslFlagKind, LayoutItem, NodeCapability, NodeDefinition, NodeFieldType,
};

/// Returns all built-in node definitions.
pub fn builtin_node_definitions() -> Vec<crate::pipeline::NodeDefinition> {
    basic::builtin_node_definitions()
}

/// What every native node kind can reach, keyed by kind.
///
/// This is the compiled-in half of the answer the package safety review needs,
/// and it is a constant of the build: nothing installed can add to it, remove
/// from it, or disagree with it. Bundle-provided kinds are resolved separately,
/// by deriving them from the nodes they compose.
///
/// A kind with an empty set is present and empty, never absent. "This node
/// reaches nothing" and "nobody asked this node" are different answers and the
/// review reports them differently.
pub fn native_node_capabilities()
-> &'static std::collections::BTreeMap<String, std::collections::BTreeSet<NodeCapability>> {
    static NATIVE: std::sync::LazyLock<
        std::collections::BTreeMap<String, std::collections::BTreeSet<NodeCapability>>,
    > = std::sync::LazyLock::new(|| {
        builtin_node_definitions()
            .into_iter()
            .map(|definition| {
                (
                    definition.kind,
                    definition.capabilities.into_iter().collect(),
                )
            })
            .collect()
    });
    &NATIVE
}

fn validate_layout_item(
    item: &LayoutItem,
    field_names: &HashSet<String>,
    errors: &mut Vec<String>,
) {
    match item {
        LayoutItem::Field(name) => {
            if !field_names.contains(name) {
                errors.push(format!("layout references unknown field '{}'", name));
            }
        }
        LayoutItem::Row { row } => {
            if row.is_empty() {
                errors.push("layout row must not be empty".to_string());
            }
            for child in row {
                validate_layout_item(child, field_names, errors);
            }
        }
        LayoutItem::Col { col } => {
            if col.is_empty() {
                errors.push("layout col must not be empty".to_string());
            }
            for child in col {
                validate_layout_item(child, field_names, errors);
            }
        }
    }
}

/// Validates that a node definition is complete enough to serve as API and docs source.
pub fn validate_node_definition_contract(def: &NodeDefinition) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let kind = def.kind.trim();
    if kind.is_empty() {
        errors.push("kind must not be empty".to_string());
    } else if kind.split('.').count() < 2 || kind.split('.').any(|segment| segment.is_empty()) {
        errors.push(format!("kind '{}' must be dotted segments, e.g. 'fs.image.thumbnail'", kind));
    }
    if def.title.trim().is_empty() {
        errors.push("title must not be empty".to_string());
    }
    if def.description.trim().is_empty() {
        errors.push("description must not be empty".to_string());
    }

    for (pin_role, pins) in [
        ("input", def.input_pins.as_slice()),
        ("output", def.output_pins.as_slice()),
    ] {
        for pin in pins {
            let trimmed = pin.trim();
            if trimmed.is_empty() {
                errors.push(format!("{} pin must not be empty", pin_role));
            } else if trimmed != pin || trimmed.contains(char::is_whitespace) {
                errors.push(format!(
                    "{} pin '{}' must be a compact token",
                    pin_role, pin
                ));
            }
        }
    }

    let mut flag_keys = HashSet::new();
    for flag in &def.dsl_flags {
        if !flag.flag.starts_with("--") || flag.flag.trim() != flag.flag {
            errors.push(format!(
                "DSL flag '{}' must start with -- and have no surrounding space",
                flag.flag
            ));
        }
        if flag.config_key.trim().is_empty() {
            errors.push(format!(
                "DSL flag '{}' config_key must not be empty",
                flag.flag
            ));
        } else {
            flag_keys.insert(flag.config_key.clone());
        }
        if flag.description.trim().is_empty() {
            errors.push(format!(
                "DSL flag '{}' description must not be empty",
                flag.flag
            ));
        }
    }

    let mut field_names = HashSet::new();
    for field in &def.fields {
        if field.name.trim().is_empty() {
            errors.push("field name must not be empty".to_string());
            continue;
        }
        if !field_names.insert(field.name.clone()) {
            errors.push(format!("field '{}' is declared more than once", field.name));
        }
        if field.label.trim().is_empty() {
            errors.push(format!("field '{}' label must not be empty", field.name));
        }
        if !matches!(field.field_type, NodeFieldType::Section)
            && field.help.as_deref().unwrap_or("").trim().is_empty()
        {
            errors.push(format!("field '{}' help must not be empty", field.name));
        }
    }

    for item in &def.layout {
        validate_layout_item(item, &field_names, &mut errors);
    }

    if let Some(properties) = def
        .config_schema
        .get("properties")
        .and_then(|value| value.as_object())
    {
        for key in properties.keys() {
            if key == "title" || key == crate::pipeline::model::NODE_TIMEOUT_KEY {
                continue;
            }
            if !field_names.contains(key) && !flag_keys.contains(key) {
                errors.push(format!(
                    "config property '{}' must be documented by a field or DSL flag",
                    key
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// The one-line signature a node's definition generates
/// (`node-conventions.md` §1–§4): its short kind, every declared flag with its
/// value type, choices and cardinality — optional flags in brackets — and the
/// key it answers. The presentation flags every kind takes are left out.
pub fn node_signature(def: &NodeDefinition) -> String {
    let common: Vec<String> = crate::pipeline::model::engine_common_dsl_flags()
        .into_iter()
        .map(|flag| flag.flag)
        .collect();
    let kind = def.kind.as_str();
    let mut out = kind.to_string();
    for flag in def.dsl_flags.iter().filter(|flag| !common.contains(&flag.flag)) {
        out.push(' ');
        out.push_str(&flag_token(flag, &flag.choices, flag.required));
    }
    if let Some(key) = answer_key(kind) {
        out.push_str(&format!(" → {key}"));
    }
    out
}

/// What a value is, as a signature writes it: its words, or its type.
fn value_token(value: &str, choices: &[String]) -> String {
    if !choices.is_empty() {
        return choices.join("|");
    }
    match value {
        "" => "VALUE".to_string(),
        "number" => "N".to_string(),
        "expression" => "EXPR".to_string(),
        other => other.trim_start_matches("file:").to_ascii_uppercase(),
    }
}

/// One flag of a signature, bracketed when optional.
fn flag_token(flag: &crate::pipeline::model::DslFlag, choices: &[String], required: bool) -> String {
    let value = value_token(&flag.value, choices);
    let token = match flag.kind {
        DslFlagKind::Bool => flag.flag.clone(),
        DslFlagKind::KeyValuePairs => format!("{} KEY={value}…", flag.flag),
        DslFlagKind::RepeatedList => format!("{} {value}…", flag.flag),
        _ => format!("{} {value}", flag.flag),
    };
    if required { token } else { format!("[{token}]") }
}

/// One provider's signature (`node-conventions.md` §11), generated from the
/// kind's definition and that provider's profile: the shared roles, the
/// provider-specific roles it takes, its narrower choices and models, and
/// each `--option` key it takes in place of the open `--option`. `None` for
/// a provider the kind has no profile for.
pub fn provider_signature(def: &NodeDefinition, provider: &str) -> Option<String> {
    use crate::pipeline::nodes::shared::profile::{MODEL_FLAG, OPTION_FLAG, PROVIDER_FLAG, gated_roles};
    let profile = def.profiles.iter().find(|p| p.provider == provider)?;
    let gated = gated_roles(def);
    let common: Vec<String> = crate::pipeline::model::engine_common_dsl_flags()
        .into_iter()
        .map(|flag| flag.flag)
        .collect();
    let mut out = def.kind.clone();
    for flag in def.dsl_flags.iter().filter(|flag| !common.contains(&flag.flag)) {
        let name = flag.flag.as_str();
        let token = if name == PROVIDER_FLAG {
            format!("{PROVIDER_FLAG} {provider}")
        } else if name == OPTION_FLAG {
            let options: Vec<String> = profile
                .options
                .iter()
                .map(|option| format!("[{OPTION_FLAG} {}={}]", option.key, value_token(&option.value, &option.choices)))
                .collect();
            if options.is_empty() {
                continue;
            }
            options.join(" ")
        } else if gated.contains(name) {
            let Some(role) = profile.roles.get(name) else { continue };
            let mut token = flag_token(flag, &flag.choices, role.required);
            if let Some(max) = role.max_repeat {
                token = token.replacen('…', &format!("…(≤{max})"), 1);
            }
            token
        } else {
            let choices = match (name, profile.choices.get(name)) {
                (_, Some(words)) => words.clone(),
                (MODEL_FLAG, None) if !profile.models.is_empty() => profile.models.clone(),
                _ => flag.choices.clone(),
            };
            flag_token(flag, &choices, flag.required)
        };
        out.push(' ');
        out.push_str(&token);
    }
    if let Some(key) = answer_key(&def.kind) {
        out.push_str(&format!(" → {key}"));
    }
    Some(out)
}

/// Every provider's signature and what it adds, as a help section; empty for
/// a kind without profiles.
pub fn format_provider_profiles_markdown(def: &NodeDefinition) -> String {
    if def.profiles.is_empty() {
        return String::new();
    }
    let mut s = String::from("**Per provider** (`--provider` picks one; each line is generated from its profile):\n\n```\n");
    for profile in &def.profiles {
        if let Some(signature) = provider_signature(def, &profile.provider) {
            s.push_str(&signature);
            s.push('\n');
        }
    }
    s.push_str("```\n\n");
    for profile in &def.profiles {
        let kinds = if profile.credential_kinds.is_empty() {
            "any kind".to_string()
        } else {
            profile.credential_kinds.iter().map(|k| format!("`{k}`")).collect::<Vec<_>>().join(" or ")
        };
        s.push_str(&format!("- `{}` — `--credential` of kind {kinds}", profile.provider));
        if !profile.models.is_empty() {
            s.push_str(&format!("; `--model` {}", profile.models.iter().map(|m| format!("`{m}`")).collect::<Vec<_>>().join(" ")));
        }
        s.push('\n');
        for option in &profile.options {
            s.push_str(&format!("  - `--option {}=…` — {}\n", option.key, option.description.trim()));
        }
    }
    s.push('\n');
    s
}

/// The key a kind answers under (`node-conventions.md` §1, §6): an acting
/// node's noun (a custom composite's too: `x.<package>.<noun>.<verb>`), an
/// entry node's source, and the two closing loop nodes' verbs (`reduce`,
/// `collect`); the other control nodes and run inputs (whose key is their
/// `--name`) have none fixed.
pub fn answer_key(kind: &str) -> Option<String> {
    let parts: Vec<&str> = kind.split('.').collect();
    match parts.as_slice() {
        ["trigger", source, ..] => Some((*source).to_string()),
        ["logic", verb @ ("reduce" | "collect")] => Some((*verb).to_string()),
        ["logic", ..] | ["input", ..] => None,
        // It answers the caller and passes its payload on (§4, §6).
        ["web", "response", "send"] => None,
        ["x", _package, noun, _verb] => Some((*noun).to_string()),
        ["x", ..] => None,
        [_, noun, _] => Some((*noun).to_string()),
        _ => None,
    }
}

pub fn format_node_definition_markdown(def: &NodeDefinition) -> String {
    let mut s = String::new();
    s.push_str(&format!("### `{}` — {}\n\n", def.kind, def.title));
    s.push_str(&format!("```\n{}\n```\n\n", node_signature(def)));
    s.push_str(&format_provider_profiles_markdown(def));
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
    s.push_str(&format!(
        "- **Input pins:** {ip}\n- **Output pins:** {op}\n\n"
    ));
    if !def.dsl_flags.is_empty() {
        s.push_str("| DSL flag | Config key | Required | Kind | Description |\n");
        s.push_str("|----------|------------|----------|------|-------------|\n");
        for f in &def.dsl_flags {
            let req = if f.required { "yes" } else { "no" };
            let kind_s = match f.kind {
                DslFlagKind::Scalar => "scalar",
                DslFlagKind::RepeatedList => "repeated-list",
                DslFlagKind::Bool => "bool",
                DslFlagKind::KeyValuePairs => "key-value-pairs",
                DslFlagKind::SchemaField => "schema-field",
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
    for example in &def.examples {
        let title = if example.title.is_empty() { "Example" } else { example.title.as_str() };
        s.push_str(&format!("**{title}:**\n"));
        if !example.dsl.is_empty() {
            s.push_str(&format!("```\n| {}\n```\n", example.dsl));
        }
        if !example.description.is_empty() {
            s.push_str(example.description.trim());
            s.push_str("\n");
        }
        if !example.input.is_null() || !example.output.is_null() {
            s.push_str("```json\n");
            if !example.input.is_null() {
                s.push_str(&format!("// input\n{}\n", example.input));
            }
            if !example.output.is_null() {
                s.push_str(&format!("// output\n{}\n", example.output));
            }
            s.push_str("```\n");
        }
        s.push_str("\n");
    }
    s.push_str(&format!(
        "**MCP:** `help_nodes` with `kind=\"{}\"` for this section only.\n\n",
        def.kind
    ));
    s
}

pub fn kind_query_matches_def(def: &NodeDefinition, query: &str) -> bool {
    let q = query.trim();
    if q.is_empty() {
        return false;
    }
    def.kind.eq_ignore_ascii_case(q)
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
         - **One kind:** `help_nodes` with `kind=\"javascript.script.run\"` (or `trigger.webhook`, etc.).\n\n\
         ---\n\n",
    );
    for def in basic::builtin_node_definitions() {
        s.push_str(&format_node_definition_markdown(&def));
        s.push_str("---\n\n");
    }
    s
}

#[cfg(test)]
mod tests {
    /// A composite node is a pipeline in JSON, so a setting its inner node no
    /// longer reads still parses and is silently ignored. That is how the
    /// embedding node went on sending empty requests for nine days after
    /// `http.response.fetch` renamed `body_path` to `body`. Every node in every
    /// bundled function must use only settings its kind declares.
    #[test]
    fn every_composite_node_uses_only_settings_its_inner_nodes_read() {
        use std::collections::{BTreeMap, BTreeSet};
        let known: BTreeMap<String, BTreeSet<String>> = super::builtin_node_definitions()
            .into_iter()
            .map(|def| {
                let mut keys: BTreeSet<String> = def.dsl_flags.iter().map(|f| f.config_key.clone()).collect();
                keys.extend(def.fields.iter().map(|f| f.name.clone()));
                if let Some(props) = def.config_schema.get("properties").and_then(|p| p.as_object()) {
                    keys.extend(props.keys().cloned());
                }
                keys.insert("ui".to_string()); // the editor's canvas position
                (def.kind, keys)
            })
            .collect();
        let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut stack = vec![crate_dir.join("src/pipeline/nodes/bundled")];
        let (mut checked, mut problems) = (0, Vec::new());
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).expect("read bundled dir").flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if !path.to_string_lossy().ends_with(".zf.json") {
                    continue;
                }
                let shown = path.strip_prefix(crate_dir).unwrap_or(&path).display().to_string();
                let doc: serde_json::Value =
                    serde_json::from_str(&std::fs::read_to_string(&path).expect("read bundle")).expect("bundle is JSON");
                let spec = doc.get("spec").unwrap_or(&doc);
                for node in spec["nodes"].as_array().into_iter().flatten() {
                    checked += 1;
                    let kind = node["kind"].as_str().unwrap_or_default();
                    let Some(keys) = known.get(kind) else {
                        problems.push(format!("{shown}#{}: no built-in node '{kind}'", node["id"]));
                        continue;
                    };
                    for key in node["config"].as_object().into_iter().flat_map(|c| c.keys()) {
                        if !keys.contains(key) {
                            problems.push(format!("{shown}#{}: {kind} does not read '{key}'", node["id"]));
                        }
                    }
                }
            }
        }
        assert!(checked > 0, "no bundled node found; the walk is looking in the wrong place");
        assert!(problems.is_empty(), "stale composite-node settings:\n{}", problems.join("\n"));
    }

    /// On 2026-09-09 every `--…-path` / `--…-expr` flag that wanted a value
    /// became a flag that takes the value. Descriptions that still named the
    /// old flags taught agents a pipeline the parser refuses. A flag of that
    /// shape named anywhere in a node's definition must exist on some node.
    #[test]
    fn no_node_definition_names_a_retired_path_or_expr_flag() {
        let defs = super::builtin_node_definitions();
        let declared: std::collections::BTreeSet<&str> =
            defs.iter().flat_map(|d| d.dsl_flags.iter().map(|f| f.flag.as_str())).collect();
        let mut stale = Vec::new();
        for def in &defs {
            let text = serde_json::to_string(def).expect("definition serialises");
            let bytes = text.as_bytes();
            let mut at = 0;
            while let Some(pos) = text[at..].find("--") {
                let start = at + pos;
                let mut end = start + 2;
                while end < bytes.len() && (bytes[end].is_ascii_lowercase() || bytes[end].is_ascii_digit() || bytes[end] == b'-') {
                    end += 1;
                }
                let flag = &text[start..end];
                let preceded_by_word = start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'-');
                if !preceded_by_word && (flag.ends_with("-path") || flag.ends_with("-expr")) && !declared.contains(flag) {
                    stale.push(format!("{}: {flag}", def.kind));
                }
                at = end.max(start + 2);
            }
        }
        stale.sort();
        stale.dedup();
        assert!(stale.is_empty(), "definitions naming a flag no node has:\n{}", stale.join("\n"));
    }

    #[test]
    fn native_node_definitions_have_complete_contracts() {
        let mut failures = Vec::new();
        for def in super::builtin_node_definitions() {
            if let Err(error) = crate::contracts::kinds::validate_normalized_node_definition(&def) {
                failures.push(format!("{}: {}", def.kind, error));
            }
        }
        assert!(
            failures.is_empty(),
            "incomplete node definitions:\n{}",
            failures.join("\n")
        );
    }

    /// The canvas-preview flags are injected into every node, the two table
    /// nodes included: they keep no row sample of their own, so `--preview`
    /// means one thing everywhere.
    #[test]
    fn every_node_takes_the_canvas_preview_flags() {
        let defs = super::builtin_node_definitions();
        let flags_of = |kind: &str| {
            defs.iter()
                .find(|d| d.kind == kind)
                .unwrap_or_else(|| panic!("{kind} missing"))
                .dsl_flags
                .iter()
                .map(|f| (f.flag.clone(), f.config_key.clone()))
                .collect::<Vec<_>>()
        };

        for kind in ["table.query.run", "table.data.convert", "javascript.script.run"] {
            let flags = flags_of(kind);
            assert!(
                flags.iter().any(|(flag, key)| flag == "--preview" && key == "preview.out"),
                "{kind} takes --preview: {flags:?}"
            );
            assert!(
                flags.iter().any(|(flag, key)| flag == "--preview-in" && key == "preview.in"),
                "{kind} takes --preview-in: {flags:?}"
            );
            assert!(
                !flags.iter().any(|(_, key)| key == "preview"),
                "{kind} no longer stores anything flat at `preview`: {flags:?}"
            );
        }
        for kind in ["table.query.run", "table.data.convert"] {
            let flags = flags_of(kind);
            assert!(
                !flags.iter().any(|(flag, _)| flag == "--preview-rows"),
                "{kind} has no row sample of its own: {flags:?}"
            );
        }
    }

    /// The source layout is the catalogue's family list: one folder per DSL
    /// family under `basic/`, one file per node, nothing loose beside
    /// `basic/mod.rs`. A family with one node keeps it in that folder's
    /// `mod.rs`; helpers several families share live in `nodes/shared/`.
    #[test]
    fn every_node_file_lives_in_its_family_folder() {
        let basic = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/pipeline/nodes/basic");
        let families: std::collections::BTreeSet<String> = super::builtin_node_definitions()
            .iter()
            .map(|def| def.kind.split('.').next().unwrap_or_default().to_string())
            .collect();

        let mut failures = Vec::new();
        let mut folders = std::collections::BTreeSet::new();
        for entry in std::fs::read_dir(&basic).expect("basic/ is readable") {
            let entry = entry.expect("entry");
            let name = entry.file_name().to_string_lossy().to_string();
            if entry.path().is_dir() {
                if !families.contains(&name) {
                    failures.push(format!(
                        "basic/{name}/ is not a DSL family of the catalogue (families: {})",
                        families.iter().cloned().collect::<Vec<_>>().join(", ")
                    ));
                }
                folders.insert(name);
            } else if name != "mod.rs" {
                failures.push(format!(
                    "basic/{name} sits beside mod.rs; a node file is basic/<family>/<node>.rs"
                ));
            }
        }
        for family in &families {
            if !folders.contains(family) {
                failures.push(format!("family `{family}` has nodes in the catalogue but no basic/{family}/ folder"));
            }
        }
        assert!(failures.is_empty(), "node layout:\n{}", failures.join("\n"));
    }

    /// `help("pipeline/nodes/<kind>")` prints each provider's own signature,
    /// generated from its profile: the openai line narrows `--model` to its
    /// models, the openrouter line takes any.
    #[test]
    fn a_kind_with_profiles_prints_each_providers_signature() {
        let def = crate::platform::services::node_registry::NodeRegistryService::embedded_official_definitions()
            .into_iter()
            .find(|d| d.kind == "ai.embedding.generate")
            .expect("the embedding composite ships");
        let help = crate::platform::help::get_help("pipeline/nodes/ai.embedding.generate").expect("help");
        assert!(help.contains("**Per provider**"), "{help}");
        assert!(help.contains("ai.embedding.generate --provider openai --credential TEXT [--model text-embedding-3-small|text-embedding-3-large|text-embedding-ada-002] --text TEXT… → embedding"), "{help}");
        assert!(help.contains("ai.embedding.generate --provider openrouter --credential TEXT [--model TEXT] --text TEXT… → embedding"), "{help}");
        assert!(help.contains("- `openai` — `--credential` of kind `openai`"), "{help}");
        assert_eq!(super::provider_signature(&def, "cohere"), None);
        let tts = crate::platform::help::get_help("pipeline/nodes/ai.audio.generate").expect("help");
        assert!(tts.contains("[--option volume=N] [--option lipsync=none|basic|timed_words|audio_guided|audio_segmented]"), "{tts}");
        assert!(tts.contains("  - `--option volume=…` — Loudness multiplier"), "{tts}");
    }

    #[test]
    fn embedded_official_node_definitions_have_complete_contracts() {
        let mut failures = Vec::new();
        for def in crate::platform::services::node_registry::NodeRegistryService::embedded_official_definitions() {
            if let Err(error) = crate::contracts::kinds::validate_normalized_node_definition(&def) {
                failures.push(format!("{}: {}", def.kind, error));
            }
        }
        assert!(
            failures.is_empty(),
            "incomplete embedded official node definitions:\n{}",
            failures.join("\n")
        );
    }
}

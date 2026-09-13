//! Pipeline DSL executor — executes parsed verbs using platform services.

use std::collections::{HashMap, HashSet};
use crate::platform::model::RepoTreeScope;
use std::sync::Arc;

use serde_json::{Value, json};

use super::parser::{
    DslVerb, build_pipeline_graph_with_definitions, parse_one_command, split_commands,
};
use super::{DslLine, DslOutput};
use crate::contracts::kinds::{decode_pipeline_graph, encode_pipeline_graph};
use crate::platform::services::PlatformService;

/// Executor for Pipeline DSL commands.
pub struct DslExecutor {
    platform: Arc<PlatformService>,
    owner: String,
    project: String,
}

impl DslExecutor {
    pub fn new(platform: Arc<PlatformService>, owner: &str, project: &str) -> Self {
        Self {
            platform,
            owner: owner.to_string(),
            project: project.to_string(),
        }
    }

    fn node_definitions(&self) -> Vec<crate::pipeline::NodeDefinition> {
        let mut definitions = self
            .platform
            .node_registry
            .merged_definitions(&self.owner, &self.project);
        // Nodes the project describes but cannot run still resolve here, so a
        // pipeline referencing one reports that its package is missing rather
        // than that the kind is unknown. The kind is not unknown; the
        // implementation is absent, and those need different answers.
        definitions.extend(
            self.platform
                .node_registry
                .unavailable_interface_definitions(&self.owner, &self.project),
        );
        definitions.sort_by(|a, b| a.kind.cmp(&b.kind));
        definitions.dedup_by(|a, b| a.kind == b.kind);
        definitions
    }

    /// Execute a run body with an optional initial JSON payload.
    pub async fn execute_run_with_input(
        &self,
        body: &str,
        input: Option<serde_json::Value>,
    ) -> DslOutput {
        self.cmd_run(body, false, input).await
    }

    /// Execute a full DSL string (may contain multiple `&&`-chained commands).
    pub async fn execute_dsl(&self, dsl: &str) -> DslOutput {
        let commands = split_commands(dsl);
        if commands.is_empty() {
            return DslOutput::err("Empty command");
        }
        let mut parsed = Vec::new();
        for cmd in commands {
            let cmd = cmd.trim();
            if cmd.is_empty() {
                continue;
            }
            let verb = parse_one_command(cmd);
            if let DslVerb::Unknown { raw } = &verb {
                let verb_word = raw.split_whitespace().next().unwrap_or("?");
                return DslOutput::err(format!(
                    "Unknown command: '{}'. Type help for available commands.",
                    verb_word
                ));
            }
            parsed.push(verb);
        }

        let mut combined = DslOutput::new_ok();
        for verb in parsed {
            let result = self.execute_verb(verb).await;
            if !result.ok {
                combined.ok = false;
            }
            combined.extend(result.lines);
        }
        combined
    }

    async fn execute_verb(&self, verb: DslVerb) -> DslOutput {
        match verb {
            DslVerb::Get {
                resource,
                path,
                filter,
                status,
            } => {
                self.cmd_get(
                    &resource,
                    path.as_deref(),
                    filter.as_deref(),
                    status.as_deref(),
                )
                .await
            }
            DslVerb::Describe {
                kind,
                name,
                compact,
            } => self.cmd_describe(&kind, &name, compact).await,
            DslVerb::Read { kind, name } => self.cmd_describe(&kind, &name, false).await,
            DslVerb::Activate { file_rel_path } => self.cmd_activate(&file_rel_path).await,
            DslVerb::Deactivate { file_rel_path } => self.cmd_deactivate(&file_rel_path).await,
            DslVerb::Execute {
                file_rel_path,
                input,
            } => self.cmd_execute(&file_rel_path, input).await,
            DslVerb::Register {
                file_rel_path,
                title,
                description,
                as_json,
                body,
            } => {
                self.cmd_register(&file_rel_path, &title, &description, as_json, &body)
                    .await
            }
            DslVerb::Patch {
                file_rel_path,
                node_id,
                flags,
                body,
            } => {
                self.cmd_patch(&file_rel_path, &node_id, flags, body.as_deref())
                    .await
            }
            DslVerb::Run { body, dry_run } => self.cmd_run(&body, dry_run, None).await,
            DslVerb::Delete { kind, name } => self.cmd_delete(&kind, &name).await,
            DslVerb::Git {
                subcommand,
                args,
                body,
            } => self.cmd_git(&subcommand, args, body.as_deref()).await,
            DslVerb::NodeHelp { kind } => self.cmd_node_help(&kind),
            DslVerb::CredentialBlocked { reason } => self.cmd_credential_blocked(&reason),
            DslVerb::Write { .. } => DslOutput::err("write/create is not yet implemented via DSL"),
            DslVerb::Unknown { raw } => {
                let verb_word = raw.split_whitespace().next().unwrap_or("?");
                DslOutput::err(format!(
                    "Unknown command: '{}'. Type help for available commands.",
                    verb_word
                ))
            }
        }
    }

    async fn cmd_get(
        &self,
        resource: &str,
        _path: Option<&str>,
        _filter: Option<&str>,
        _status: Option<&str>,
    ) -> DslOutput {
        match resource {
            "pipelines" | "pipeline" => {
                match self
                    .platform
                    .projects
                    .list_pipeline_meta_rows(&self.owner, &self.project)
                {
                    Ok(rows) => {
                        let mut out = DslOutput::new_ok();
                        if rows.is_empty() {
                            out.push(DslLine::muted("(no pipelines)"));
                            return out;
                        }
                        out.push(DslLine::muted(format!(
                            "{:<26} {:<12} {:<8} {}",
                            "NAME", "TRIGGER", "STATUS", "PATH"
                        )));
                        out.push(DslLine::muted(format!(
                            "{:-<26} {:-<12} {:-<8} {}",
                            "", "", "", "----"
                        )));
                        for m in &rows {
                            let name = truncate(&m.name, 26);
                            let trigger = truncate(&m.trigger_kind, 12);
                            let status = crate::platform::services::ops::pipeline_status(m);
                            out.push(DslLine::info(format!(
                                "{:<26} {:<12} {:<8} {}",
                                name, trigger, status, m.virtual_path
                            )));
                        }
                        out
                    }
                    Err(e) => DslOutput::err(format!("Error: {}", e.message)),
                }
            }
            "nodes" | "node" => {
                let defs = crate::pipeline::nodes::builtin_node_definitions();
                let mut out = DslOutput::new_ok();
                out.push(DslLine::muted(format!("{:<30} {}", "KIND", "DESCRIPTION")));
                out.push(DslLine::muted(format!("{:-<30} {}", "", "------------")));
                for def in &defs {
                    out.push(DslLine::info(format!(
                        "{:<30} {}",
                        def.kind,
                        truncate(&def.description, 60)
                    )));
                }
                out
            }
            "connections" | "connection" => {
                match self
                    .platform
                    .db_connections
                    .list_project_connections(&self.owner, &self.project)
                {
                    Ok(conns) => {
                        let mut out = DslOutput::new_ok();
                        if conns.is_empty() {
                            out.push(DslLine::muted("(no connections)"));
                            return out;
                        }
                        out.push(DslLine::muted(format!(
                            "{:<24} {:<16} {}",
                            "SLUG", "KIND", "LABEL"
                        )));
                        for c in &conns {
                            out.push(DslLine::info(format!(
                                "{:<24} {:<16} {}",
                                c.connection_slug, c.database_kind, c.connection_label
                            )));
                        }
                        out
                    }
                    Err(e) => DslOutput::err(format!("Error: {}", e.message)),
                }
            }
            "credentials" | "credential" => {
                match self
                    .platform
                    .credentials
                    .list_project_credentials(&self.owner, &self.project)
                {
                    Ok(creds) => {
                        let mut out = DslOutput::new_ok();
                        if creds.is_empty() {
                            out.push(DslLine::muted("(no credentials)"));
                            return out;
                        }
                        for c in &creds {
                            out.push(DslLine::info(format!("{} ({})", c.credential_id, c.kind)));
                        }
                        out
                    }
                    Err(e) => DslOutput::err(format!("Error: {}", e.message)),
                }
            }
            "templates" | "template" => {
                match self
                    .platform
                    .projects
                    .list_repo_tree(&self.owner, &self.project, &RepoTreeScope::all())
                {
                    Ok(ws) => {
                        let mut out = DslOutput::new_ok();
                        if ws.items.is_empty() {
                            out.push(DslLine::muted("(no templates)"));
                            return out;
                        }
                        for item in &ws.items {
                            if item.kind == "file" {
                                out.push(DslLine::info(item.rel_path.clone()));
                            }
                        }
                        out
                    }
                    Err(e) => DslOutput::err(format!("Error: {}", e.message)),
                }
            }
            "docs" | "doc" => {
                match self
                    .platform
                    .projects
                    .list_repo_tree(&self.owner, &self.project, &RepoTreeScope::all())
                {
                    Ok(listing) => {
                        let mut out = DslOutput::new_ok();
                        let docs = listing
                            .items
                            .iter()
                            .filter(|item| item.file_kind == "doc")
                            .collect::<Vec<_>>();
                        if docs.is_empty() {
                            out.push(DslLine::muted("(no docs)"));
                            return out;
                        }
                        for item in docs {
                            out.push(DslLine::info(item.rel_path.clone()));
                        }
                        out
                    }
                    Err(e) => DslOutput::err(format!("Error: {}", e.message)),
                }
            }
            other => DslOutput::err(format!(
                "get: unknown resource '{}'. Try: pipelines, nodes, connections, credentials, templates, docs",
                other
            )),
        }
    }

    async fn cmd_describe(&self, kind: &str, name: &str, compact: bool) -> DslOutput {
        match kind {
            "pipeline" | "pipelines" => self.describe_pipeline(name, compact).await,
            "connection" | "connections" => self.describe_connection(name).await,
            "node" | "nodes" => self.describe_node(name),
            other => DslOutput::err(format!(
                "describe: unknown kind '{}'. Try: pipeline, connection, node",
                other
            )),
        }
    }

    async fn describe_pipeline(&self, file_rel_path: &str, compact: bool) -> DslOutput {
        let Some(meta) = (match self.platform.projects.get_pipeline_meta_by_file_id(
            &self.owner,
            &self.project,
            file_rel_path,
        ) {
            Ok(m) => m,
            Err(e) => return DslOutput::err(format!("Error: {}", e.message)),
        }) else {
            return DslOutput::err(format!("Pipeline '{file_rel_path}' not found"));
        };

        let mut out = DslOutput::new_ok();
        let status = crate::platform::services::ops::pipeline_status(&meta);
        let hash_short = meta.hash.chars().take(8).collect::<String>();

        let hits = self
            .platform
            .pipeline_hits
            .get(&self.owner, &self.project, &meta.file_rel_path);

        if compact {
            // Compact header: single line
            out.push(DslLine::info(format!(
                "{}  [{}]  hash:{}  hits:{}/{}",
                meta.file_rel_path, status, hash_short, hits.success_count, hits.failed_count
            )));
        } else {
            out.push(DslLine::info(format!("pipeline: {}", meta.file_rel_path)));
            out.push(DslLine::info(format!(
                "status: {}  hash: {}  hits: {} ok / {} failed",
                status, hash_short, hits.success_count, hits.failed_count
            )));
        }

        // Reconstruct DSL from the stored graph
        if let Ok(source) = self.platform.projects.read_pipeline_source(
            &self.owner,
            &self.project,
            &meta.file_rel_path,
        ) {
            if let Ok(graph) = decode_pipeline_graph(source.as_bytes()).map(|value| value.spec) {
                if compact {
                    // Compact: one line per node, no body content
                    for node in &graph.nodes {
                        let seg = crate::platform::shell::parser::node_to_segment_no_body(node);
                        out.push(DslLine::info(format!("  {:6}  {}", node.id, seg)));
                    }
                } else {
                    let dsl = crate::platform::shell::parser::graph_to_dsl(&graph);
                    out.push(DslLine::blank());
                    for line in dsl.lines() {
                        out.push(DslLine::info(line.to_string()));
                    }

                    // Node id reference for pipeline_patch
                    if !graph.nodes.is_empty() {
                        let ids = graph
                            .nodes
                            .iter()
                            .map(|n| {
                                format!(
                                    "{}({})",
                                    n.id,
                                    n.kind.strip_prefix("n.").unwrap_or(&n.kind)
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("  ");
                        out.push(DslLine::blank());
                        out.push(DslLine::muted(format!("node ids (for patch): {}", ids)));
                    }
                }
            }
        }

        out
    }

    async fn describe_connection(&self, name: &str) -> DslOutput {
        let conns = match self
            .platform
            .db_connections
            .list_project_connections(&self.owner, &self.project)
        {
            Ok(c) => c,
            Err(e) => return DslOutput::err(format!("Error: {}", e.message)),
        };
        let Some(conn) = conns.iter().find(|c| c.connection_slug == name) else {
            return DslOutput::err(format!("Connection '{name}' not found"));
        };

        let mut out = DslOutput::new_ok();
        out.push(DslLine::info(format!(
            "connection: {}  kind: {}  label: {}",
            conn.connection_slug, conn.database_kind, conn.connection_label
        )));
        if let Some(cred_id) = &conn.credential_id {
            out.push(DslLine::muted(format!("  credential: {}", cred_id)));
        }
        out
    }

    /// Refreshes the interfaces this project carries for third-party nodes.
    ///
    /// Failing to write an interface must not fail a pipeline save: the pipeline
    /// is the user's work, the interface is a derived convenience.
    fn sync_node_interfaces(&self) {
        if let Err(error) = self
            .platform
            .node_registry
            .sync_project_node_interfaces(&self.owner, &self.project)
        {
            eprintln!(
                "node_interfaces: sync failed for {}/{}: {}",
                self.owner, self.project, error.message
            );
        }
    }

    fn describe_node(&self, kind: &str) -> DslOutput {
        let defs = crate::pipeline::nodes::builtin_node_definitions();
        let full_kind = crate::platform::shell::parser::expand_kind(kind).unwrap_or(kind);
        let Some(def) = defs.iter().find(|d| d.kind == full_kind) else {
            return DslOutput::err(format!(
                "Node kind '{kind}' not found. Use 'get nodes' for list."
            ));
        };

        let mut out = DslOutput::new_ok();
        out.push(DslLine::info(format!(
            "kind: {}  — {}",
            def.kind, def.description
        )));
        out.push(DslLine::muted(format!(
            "  inputs: {}  outputs: {}",
            def.input_pins.join(", "),
            def.output_pins.join(", ")
        )));
        out
    }

    async fn cmd_register(
        &self,
        file_rel_path: &str,
        title: &str,
        description: &str,
        as_json: bool,
        body: &str,
    ) -> DslOutput {
        if file_rel_path.is_empty() {
            return DslOutput::err(
                "register: pipeline file_rel_path is required (e.g. api/my-pipe)",
            );
        }
        if body.is_empty() {
            return DslOutput::err(
                "register: pipeline body is required. \
                 Example: register api/my-pipe | trigger.webhook --path /api | pg.query --credential main-db",
            );
        }

        let name = crate::platform::services::project::name_from_file_rel_path(file_rel_path);
        let node_definitions = self.node_definitions();
        let graph_source = if as_json {
            body.to_string()
        } else {
            match build_pipeline_graph_with_definitions(&name, body, &node_definitions) {
                Ok(mut graph) => {
                    if !description.trim().is_empty() {
                        graph.description = Some(description.trim().to_string());
                    }
                    match encode_pipeline_graph(graph).and_then(|bytes| {
                        String::from_utf8(bytes).map_err(|err| {
                            crate::contracts::ContractError::invalid(err.to_string())
                        })
                    }) {
                        Ok(source) => source,
                        Err(e) => return DslOutput::err(format!("Serialize error: {e}")),
                    }
                }
                Err(e) => return DslOutput::err(format!("Parse error: {e}")),
            }
        };

        // Validate JSON
        let graph = match decode_pipeline_graph(graph_source.as_bytes()) {
            Ok(document) => document.spec,
            Err(e) => return DslOutput::err(format!("Invalid pipeline JSON: {e}")),
        };

        // Webhook conflict check: reject if any active pipeline already owns the same path.
        let self_file_rel_path = self
            .platform
            .projects
            .get_pipeline_meta_by_file_id(&self.owner, &self.project, file_rel_path)
            .ok()
            .flatten()
            .map(|m| m.file_rel_path)
            .unwrap_or_default();

        match self.platform.projects.check_webhook_path_conflict(
            &self.owner,
            &self.project,
            &graph,
            &self_file_rel_path,
        ) {
            Ok(conflicts) if !conflicts.is_empty() => {
                let c = &conflicts[0];
                return DslOutput::err(format!(
                    "Webhook conflict: {} {} is already registered by pipeline '{}' ({})",
                    c.method, c.path, c.pipeline_name, c.file_rel_path
                ));
            }
            Ok(_) => {}
            Err(err) => return DslOutput::err(format!("Webhook validation failed: {err}")),
        }

        let trigger_kind = graph
            .nodes
            .first()
            .map(|n| {
                if n.kind.contains("webhook") {
                    "webhook"
                } else if n.kind.contains("schedule") {
                    "schedule"
                } else if n.kind.contains("function") {
                    "function"
                } else {
                    "manual"
                }
            })
            .unwrap_or("manual");

        let display_title = if title.is_empty() { &name } else { title };
        // --description flag takes precedence; fall back to description embedded in graph JSON
        let effective_description = if !description.trim().is_empty() {
            description.trim().to_string()
        } else {
            graph.description.as_deref().unwrap_or("").to_string()
        };

        match self.platform.projects.upsert_pipeline_definition(
            &self.owner,
            &self.project,
            file_rel_path,
            display_title,
            &effective_description,
            trigger_kind,
            &graph_source,
        ) {
            Ok(meta) => {
                // The set of third-party nodes this project depends on may have
                // changed, so refresh the interfaces it carries in repo/nodes.
                self.sync_node_interfaces();
                let mut out = DslOutput::new_ok();
                out.push(DslLine::success(format!(
                    "Pipeline '{}' registered ({} nodes). Use 'activate pipeline {}' to make it live.",
                    meta.file_rel_path,
                    graph.nodes.len(),
                    meta.file_rel_path
                )));
                // Emit non-fatal warnings for unknown config keys (likely flag typos).
                for w in validate_graph_flags(&graph, &node_definitions) {
                    out.push(DslLine::muted(format!("⚠ {}", w)));
                }
                out
            }
            Err(e) => DslOutput::err(format!("Error: {}", e.message)),
        }
    }

    async fn cmd_patch(
        &self,
        file_rel_path: &str,
        node_id: &str,
        flags: HashMap<String, Value>,
        body: Option<&str>,
    ) -> DslOutput {
        if file_rel_path.is_empty() || node_id.is_empty() {
            return DslOutput::err(
                "patch: usage: patch pipeline <file_rel_path> node <id> [--flag value...]",
            );
        }

        let meta = match self.platform.projects.get_pipeline_meta_by_file_id(
            &self.owner,
            &self.project,
            file_rel_path,
        ) {
            Ok(Some(m)) => m,
            Ok(None) => return DslOutput::err(format!("Pipeline '{file_rel_path}' not found")),
            Err(e) => return DslOutput::err(format!("Error: {}", e.message)),
        };

        let source = match self.platform.projects.read_pipeline_source(
            &self.owner,
            &self.project,
            &meta.file_rel_path,
        ) {
            Ok(s) => s,
            Err(e) => return DslOutput::err(format!("Error reading pipeline: {}", e.message)),
        };

        let mut graph = match decode_pipeline_graph(source.as_bytes()) {
            Ok(document) => document.spec,
            Err(e) => return DslOutput::err(format!("Parse error: {e}")),
        };

        // Find node index: first try exact ID, then kind-based addressing
        let node_idx = if let Some(i) = graph.nodes.iter().position(|n| n.id == node_id) {
            i
        } else {
            let (kind_ref, explicit_idx) = parse_node_kind_ref(node_id);
            let norm_ref = kind_ref.strip_prefix("n.").unwrap_or(&kind_ref);
            let candidates: Vec<usize> = graph
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.kind.strip_prefix("n.").unwrap_or(&n.kind) == norm_ref)
                .map(|(i, _)| i)
                .collect();
            match (candidates.len(), explicit_idx) {
                (0, _) => {
                    return DslOutput::err(format!(
                        "Node '{node_id}' not found in pipeline '{file_rel_path}'"
                    ));
                }
                (1, None) | (1, Some(0)) => candidates[0],
                (_, Some(idx)) if idx < candidates.len() => candidates[idx],
                (_, Some(idx)) => {
                    return DslOutput::err(format!(
                        "Kind '{kind_ref}' has {} node(s) — index {idx} is out of range",
                        candidates.len()
                    ));
                }
                (n, None) => {
                    return DslOutput::err(format!(
                        "Kind '{kind_ref}' matches {n} nodes — use {kind_ref}[0], {kind_ref}[1], etc. to specify which"
                    ));
                }
            }
        };
        let node = &mut graph.nodes[node_idx];
        let node_kind = node.kind.clone();

        // Build raw_flag_key → config_key maps for all DSL flag kinds so that flags like
        // --credential (raw key "credential") correctly write to config_key "credential_id",
        // --claim → "claims" (KeyValuePairs), --roles → ["a","b"] (CommaSeparatedList), etc.
        let (kv_config_keys, comma_list_config_keys, scalar_config_keys): (
            std::collections::HashMap<String, String>,
            std::collections::HashMap<String, String>,
            std::collections::HashMap<String, String>,
        ) = {
            let defs = crate::pipeline::nodes::builtin_node_definitions();
            let node_def = defs.iter().find(|d| d.kind == node_kind);
            let flag_key = |f: &crate::pipeline::model::DslFlag| {
                f.flag.trim_start_matches("--").replace('-', "_")
            };
            let kv = node_def
                .map(|d| {
                    d.dsl_flags
                        .iter()
                        .filter(|f| f.kind == crate::pipeline::model::DslFlagKind::KeyValuePairs)
                        .map(|f| (flag_key(f), f.config_key.clone()))
                        .collect()
                })
                .unwrap_or_default();
            let csv = node_def
                .map(|d| {
                    d.dsl_flags
                        .iter()
                        .filter(|f| {
                            matches!(
                                f.kind,
                                crate::pipeline::model::DslFlagKind::CommaSeparatedList
                                    | crate::pipeline::model::DslFlagKind::RepeatedList
                            )
                        })
                        .map(|f| (flag_key(f), f.config_key.clone()))
                        .collect()
                })
                .unwrap_or_default();
            // Scalar and Bool flags: flag name may differ from config key (e.g. --credential → credential_id).
            let scalar = node_def
                .map(|d| {
                    d.dsl_flags
                        .iter()
                        .filter(|f| {
                            matches!(
                                f.kind,
                                crate::pipeline::model::DslFlagKind::Scalar
                                    | crate::pipeline::model::DslFlagKind::Bool
                            )
                        })
                        .map(|f| (flag_key(f), f.config_key.clone()))
                        .collect()
                })
                .unwrap_or_default();
            (kv, csv, scalar)
        };

        if let Value::Object(ref mut cfg) = node.config {
            for (k, v) in &flags {
                if let Some(config_key) = kv_config_keys.get(k) {
                    // KeyValuePairs: parse each "key=value" string and merge into the map.
                    // v may be a single string or an array (when flag repeated multiple times).
                    let items: Vec<&str> = match v {
                        Value::Array(arr) => arr.iter().filter_map(|x| x.as_str()).collect(),
                        Value::String(s) => vec![s.as_str()],
                        _ => vec![],
                    };
                    let entry = cfg
                        .entry(config_key.clone())
                        .or_insert_with(|| Value::Object(serde_json::Map::new()));
                    if let Value::Object(m) = entry {
                        for pair in items {
                            let (pk, pv) = if let Some(eq) = pair.find('=') {
                                (&pair[..eq], &pair[eq + 1..])
                            } else {
                                (pair, "")
                            };
                            m.insert(pk.to_string(), json!(pv));
                        }
                    }
                } else if let Some(config_key) = comma_list_config_keys.get(k) {
                    // CommaSeparatedList: split "a,b,c" → ["a","b","c"].
                    // parse_flags_for_patch stored it as a raw string via coerce_scalar_value.
                    let csv = match v {
                        Value::String(s) => s.as_str(),
                        _ => {
                            // Already an array (or unexpected type) — store as-is.
                            cfg.insert(config_key.clone(), v.clone());
                            continue;
                        }
                    };
                    let arr: Vec<Value> = csv.split(',').map(|s| json!(s.trim())).collect();
                    cfg.insert(config_key.clone(), Value::Array(arr));
                } else {
                    // Scalar/Bool: map flag key → config_key (e.g. "credential" → "credential_id").
                    // Fall back to the raw key for unknown/custom config keys.
                    let config_key = scalar_config_keys
                        .get(k)
                        .cloned()
                        .unwrap_or_else(|| k.clone());
                    cfg.insert(config_key, v.clone());
                }
            }
            if let Some(body_val) = body {
                let body_key = if node.kind.contains("pg.query") {
                    "query"
                } else if node.kind.contains("script") {
                    "source"
                } else {
                    "body"
                };
                cfg.insert(body_key.to_string(), json!(body_val));
            }
        }

        let new_source = match encode_pipeline_graph(graph.clone()).and_then(|bytes| {
            String::from_utf8(bytes)
                .map_err(|err| crate::contracts::ContractError::invalid(err.to_string()))
        }) {
            Ok(source) => source,
            Err(e) => return DslOutput::err(format!("Serialize error: {e}")),
        };

        let trigger_kind = graph
            .nodes
            .first()
            .map(|n| {
                if n.kind.contains("webhook") {
                    "webhook"
                } else if n.kind.contains("schedule") {
                    "schedule"
                } else if n.kind.contains("function") {
                    "function"
                } else {
                    "manual"
                }
            })
            .unwrap_or("manual");

        match self.platform.projects.upsert_pipeline_definition(
            &self.owner,
            &self.project,
            &meta.file_rel_path,
            &meta.title,
            &meta.description,
            trigger_kind,
            &new_source,
        ) {
            Ok(_) => {
                self.sync_node_interfaces();
                let mut out = DslOutput::new_ok();
                out.push(DslLine::success(format!(
                    "Node '{node_id}' in pipeline '{file_rel_path}' updated."
                )));
                out
            }
            Err(e) => DslOutput::err(format!("Error: {}", e.message)),
        }
    }

    async fn cmd_activate(&self, file_rel_path: &str) -> DslOutput {
        if file_rel_path.is_empty() {
            return DslOutput::err("activate: pipeline file_rel_path is required");
        }

        match self.platform.projects.activate_pipeline_definition(
            &self.owner,
            &self.project,
            file_rel_path,
        ) {
            Ok(meta) => {
                let _ = self.platform.pipeline_runtime.refresh_pipeline(
                    &self.owner,
                    &self.project,
                    file_rel_path,
                );
                let mut out = DslOutput::new_ok();
                out.push(DslLine::success(format!(
                    "Pipeline '{}' activated.",
                    meta.file_rel_path
                )));
                out
            }
            Err(e) => DslOutput::err(format!("Error: {}", e.message)),
        }
    }

    async fn cmd_deactivate(&self, file_rel_path: &str) -> DslOutput {
        if file_rel_path.is_empty() {
            return DslOutput::err("deactivate: pipeline file_rel_path is required");
        }

        match self.platform.projects.deactivate_pipeline_definition(
            &self.owner,
            &self.project,
            file_rel_path,
        ) {
            Ok(meta) => {
                let _ = self.platform.pipeline_runtime.refresh_pipeline(
                    &self.owner,
                    &self.project,
                    file_rel_path,
                );
                let mut out = DslOutput::new_ok();
                out.push(DslLine::success(format!(
                    "Pipeline '{}' deactivated.",
                    meta.file_rel_path
                )));
                out
            }
            Err(e) => DslOutput::err(format!("Error: {}", e.message)),
        }
    }

    async fn cmd_execute(&self, file_rel_path: &str, input: Value) -> DslOutput {
        if file_rel_path.is_empty() {
            return DslOutput::err("execute: pipeline file_rel_path is required");
        }

        let meta = match self.platform.projects.get_pipeline_meta_by_file_id(
            &self.owner,
            &self.project,
            file_rel_path,
        ) {
            Ok(Some(m)) => m,
            Ok(None) => return DslOutput::err(format!("Pipeline '{file_rel_path}' not found")),
            Err(e) => return DslOutput::err(format!("Error: {}", e.message)),
        };
        if meta.active_hash.is_none() {
            return DslOutput::err(format!(
                "Pipeline '{file_rel_path}' is not active. Use 'activate pipeline {file_rel_path}' first."
            ));
        }

        let source = match self.platform.projects.read_active_pipeline_source(
            &self.owner,
            &self.project,
            &meta,
        ) {
            Ok(s) => s,
            Err(e) => return DslOutput::err(format!("Error: {}", e.message)),
        };

        let graph = match decode_pipeline_graph(source.as_bytes()) {
            Ok(document) => document.spec,
            Err(e) => return DslOutput::err(format!("Parse error: {e}")),
        };

        let ctx = crate::pipeline::PipelineContext {
            owner: self.owner.clone(),
            project: self.project.clone(),
            pipeline: graph.id.clone(),
            request_id: format!(
                "dsl-exec-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ),
            route: Default::default(),
            input,
            trigger: None,
            placeholder: None,
        };

        let engine = crate::pipeline::BasicPipelineEngine::new(
            Arc::new(self.platform.project_sandbox(&self.owner, &self.project)),
            crate::rwe::resolve_engine_or_default(None),
            Some(self.platform.credentials.clone()),
        )
        .with_platform(self.platform.clone())
        .with_ws_hub(self.platform.ws_hub.clone())
        .with_state_bus(self.platform.state_bus.clone())
        .with_data_root(self.platform.config.data_root.clone())
        .with_project_layout(
            self.platform
                .projects
                .project_layout(&self.owner, &self.project)
                .ok(),
        );

        use crate::pipeline::PipelineEngine;
        match engine.execute_async(&graph, &ctx).await {
            Ok(output) => {
                self.platform.pipeline_hits.record_success(
                    &self.owner,
                    &self.project,
                    &meta.file_rel_path,
                );
                let mut out = DslOutput::new_ok();
                out.push(DslLine::success(format!(
                    "Pipeline '{}' executed.",
                    meta.file_rel_path
                )));
                let result_str = serde_json::to_string_pretty(&output.value)
                    .unwrap_or_else(|_| output.value.to_string());
                for line in result_str.lines().take(20) {
                    out.push(DslLine::info(line.to_string()));
                }
                if !output.node_trace.is_empty() {
                    let total_ms: u64 = output.node_trace.iter().map(|e| e.duration_ms).sum();
                    out.push(DslLine::muted(format!(
                        "--- node trace ({} nodes, {}ms total) ---",
                        output.node_trace.len(),
                        total_ms
                    )));
                    for entry in &output.node_trace {
                        let marker = if entry.error.is_none() { "✓" } else { "✗" };
                        let err_part = entry
                            .error
                            .as_deref()
                            .map(|e| format!("  → {}", e))
                            .unwrap_or_default();
                        out.push(DslLine::info(format!(
                            "  {}  {}  ({})  {}ms{}",
                            marker, entry.node_id, entry.node_kind, entry.duration_ms, err_part
                        )));
                    }
                }
                out
            }
            Err(e) => {
                self.platform.pipeline_hits.record_failure(
                    &self.owner,
                    &self.project,
                    &meta.file_rel_path,
                    "dsl.execute",
                    e.code,
                    &e.message,
                );
                let node_ctx = match (&e.node_id, &e.node_kind) {
                    (Some(id), Some(kind)) => format!(" [node {} ({})]", id, kind),
                    _ => String::new(),
                };
                DslOutput::err(format!(
                    "Execution error: {} — {}{}",
                    e.code, e.message, node_ctx
                ))
            }
        }
    }

    async fn cmd_run(
        &self,
        body: &str,
        dry_run: bool,
        initial_input: Option<serde_json::Value>,
    ) -> DslOutput {
        if body.is_empty() {
            return DslOutput::err(
                "run: pipeline body is required. Example: run | trigger.manual | script -- return { ok: true };",
            );
        }

        let node_definitions = self.node_definitions();
        match build_pipeline_graph_with_definitions("ephemeral", body, &node_definitions) {
            Ok(graph) => {
                if dry_run {
                    let mut out = DslOutput::new_ok();
                    out.push(DslLine::info("Dry run — parsed pipeline graph:"));
                    match serde_json::to_string_pretty(&graph) {
                        Ok(s) => {
                            for line in s.lines() {
                                out.push(DslLine::muted(line.to_string()));
                            }
                        }
                        Err(e) => out.push(DslLine::error(format!("Serialize error: {e}"))),
                    }
                    return out;
                }

                let ctx = crate::pipeline::PipelineContext {
                    owner: self.owner.clone(),
                    project: self.project.clone(),
                    pipeline: graph.id.clone(),
                    request_id: format!(
                        "dsl-run-{}",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis()
                    ),
                    route: Default::default(),
                    input: initial_input.unwrap_or_else(|| json!({})),
                    trigger: None,
                    placeholder: None,
                };

                let engine = crate::pipeline::BasicPipelineEngine::new(
                    Arc::new(self.platform.project_sandbox(&self.owner, &self.project)),
                    crate::rwe::resolve_engine_or_default(None),
                    Some(self.platform.credentials.clone()),
                )
                .with_platform(self.platform.clone())
                .with_ws_hub(self.platform.ws_hub.clone())
                .with_state_bus(self.platform.state_bus.clone())
                .with_data_root(self.platform.config.data_root.clone())
                .with_project_layout(
                    self.platform
                        .projects
                        .project_layout(&self.owner, &self.project)
                        .ok(),
                );

                use crate::pipeline::PipelineEngine;
                match engine.execute_async(&graph, &ctx).await {
                    Ok(output) => {
                        let mut out = DslOutput::new_ok();
                        out.push(DslLine::success(format!(
                            "Ephemeral pipeline executed ({} nodes).",
                            graph.nodes.len()
                        )));
                        let result_str = serde_json::to_string_pretty(&output.value)
                            .unwrap_or_else(|_| output.value.to_string());
                        for line in result_str.lines().take(20) {
                            out.push(DslLine::info(line.to_string()));
                        }
                        if !output.node_trace.is_empty() {
                            let total_ms: u64 =
                                output.node_trace.iter().map(|e| e.duration_ms).sum();
                            out.push(DslLine::muted(format!(
                                "--- node trace ({} nodes, {}ms total) ---",
                                output.node_trace.len(),
                                total_ms
                            )));
                            for entry in &output.node_trace {
                                let marker = if entry.error.is_none() { "✓" } else { "✗" };
                                let err_part = entry
                                    .error
                                    .as_deref()
                                    .map(|e| format!("  → {}", e))
                                    .unwrap_or_default();
                                out.push(DslLine::info(format!(
                                    "  {}  {}  ({})  {}ms{}",
                                    marker,
                                    entry.node_id,
                                    entry.node_kind,
                                    entry.duration_ms,
                                    err_part
                                )));
                            }
                        }
                        out
                    }
                    Err(e) => {
                        let node_ctx = match (&e.node_id, &e.node_kind) {
                            (Some(id), Some(kind)) => format!(" [node {} ({})]", id, kind),
                            _ => String::new(),
                        };
                        DslOutput::err(format!("Run error: {} — {}{}", e.code, e.message, node_ctx))
                    }
                }
            }
            Err(e) => DslOutput::err(format!("Parse error: {e}")),
        }
    }

    async fn cmd_delete(&self, kind: &str, name: &str) -> DslOutput {
        DslOutput::err(format!(
            "delete: not supported via DSL for safety. \
             Use the UI to delete {} '{}'. \
             You can use 'deactivate pipeline {}' to take it offline.",
            kind, name, name
        ))
    }

    async fn cmd_git(&self, subcommand: &str, args: Vec<String>, body: Option<&str>) -> DslOutput {
        let allowed = ["status", "log", "diff", "add", "commit"];
        if !allowed.contains(&subcommand) {
            return DslOutput::err(format!(
                "git: '{}' is not allowed. Allowed subcommands: status, log, diff, add, commit",
                subcommand
            ));
        }
        // The subcommand was the only thing ever checked, and git's own flags
        // are a general-purpose file interface: `git diff --no-index
        // /etc/passwd /dev/null` prints any file the server can read straight
        // into the tool result, and `--output=<path>` writes one. `current_dir`
        // bounds relative resolution and nothing else. So arguments are
        // reviewed too, per subcommand.
        if let Err(reason) = review_git_arguments(subcommand, &args) {
            return DslOutput::err(format!("git: {reason}"));
        }

        let layout = match self
            .platform
            .file
            .ensure_project_layout(&self.owner, &self.project)
        {
            Ok(l) => l,
            Err(e) => return DslOutput::err(format!("Error: {}", e.message)),
        };

        let mut cmd = std::process::Command::new("git");

        // On commit: inject committer identity from user profile.
        // git global options (-c) must come BEFORE the subcommand.
        if subcommand == "commit" {
            // Fetch stored git identity (best-effort; fall back to owner slug).
            let (git_name, git_email) = self
                .platform
                .users
                .get_user(&self.owner)
                .ok()
                .flatten()
                .map(|u| {
                    let name = if u.git_name.is_empty() {
                        u.owner.clone()
                    } else {
                        u.git_name
                    };
                    let email = if u.git_email.is_empty() {
                        format!("{}@zebflow.local", u.owner)
                    } else {
                        u.git_email
                    };
                    (name, email)
                })
                .unwrap_or_else(|| (self.owner.clone(), format!("{}@zebflow.local", self.owner)));

            // -c flags before subcommand (git global options).
            cmd.arg("-c").arg(format!("user.name={git_name}"));
            cmd.arg("-c").arg(format!("user.email={git_email}"));
        }

        cmd.arg(subcommand);

        if subcommand == "commit" {
            // Filter out any -c user.name / -c user.email the LLM may have injected.
            let mut skip_next = false;
            for arg in &args {
                if skip_next {
                    skip_next = false;
                    continue;
                }
                if arg == "-c" {
                    skip_next = true;
                    continue;
                }
                if arg.starts_with("user.name=") || arg.starts_with("user.email=") {
                    continue;
                }
                cmd.arg(arg);
            }
        } else {
            for arg in &args {
                cmd.arg(arg);
            }
        }

        // For commit with body: use body as commit message.
        // Strip any Co-authored-by lines injected by AI agents before committing.
        if subcommand == "commit" {
            if let Some(msg) = body {
                let cleaned: String = msg
                    .lines()
                    .filter(|l| !l.trim_start().to_lowercase().starts_with("co-authored-by:"))
                    .collect::<Vec<_>>()
                    .join("\n")
                    .trim_end()
                    .to_string();
                cmd.arg("-m").arg(cleaned);
            }
        }

        cmd.current_dir(&layout.repo_dir);

        match cmd.output() {
            Ok(output) => {
                let mut out = DslOutput::new_ok();
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                for line in stdout.lines() {
                    out.push(DslLine::info(line.to_string()));
                }
                if !stderr.is_empty() {
                    for line in stderr.lines() {
                        out.push(DslLine::muted(line.to_string()));
                    }
                }
                if stdout.is_empty() && stderr.is_empty() {
                    out.push(DslLine::muted("(no output)"));
                }
                if !output.status.success() {
                    out.ok = false;
                }
                out
            }
            Err(e) => DslOutput::err(format!("git error: {e}")),
        }
    }

    fn cmd_node_help(&self, kind: &str) -> DslOutput {
        if kind.is_empty() {
            return DslOutput::err(
                "node help: kind is required. Example: node help trigger.webhook",
            );
        }
        self.describe_node(kind)
    }

    fn cmd_credential_blocked(&self, reason: &str) -> DslOutput {
        let mut out = DslOutput::new_ok();
        out.push(DslLine::error(reason.to_string()));
        out.push(DslLine::muted(
            "Use the Credentials UI at /projects/{owner}/{project}/credentials to manage secrets.",
        ));
        out
    }
}

/// Parse a node kind reference with optional index suffix.
/// "pg.query" → ("pg.query", None)
/// "pg.query[1]" → ("pg.query", Some(1))
fn parse_node_kind_ref(s: &str) -> (String, Option<usize>) {
    if let Some(bracket_pos) = s.rfind('[') {
        if s.ends_with(']') {
            let kind = s[..bracket_pos].to_string();
            let idx_str = &s[bracket_pos + 1..s.len() - 1];
            if let Ok(idx) = idx_str.parse::<usize>() {
                return (kind, Some(idx));
            }
        }
    }
    (s.to_string(), None)
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

/// Config keys that are valid on any node and should not trigger unknown-key warnings.
const GLOBAL_CONFIG_KEYS: &[&str] = &[
    "title",
    "path",
    "method",
    "route",
    "credential_id",
    "sql",
    "query",
    "source",
    "body",
    "markup",
    "template_path",
    "template_id",
    "credential_id_expr",
    "query_expr",
    "params_path",
    "params_expr",
    "room",
    "event",
    "ui",
];

/// Validate node config keys against declared DSL flags for each node kind.
/// Returns a list of warning strings for unknown config keys (likely typos).
fn validate_graph_flags(
    graph: &crate::pipeline::PipelineGraph,
    definitions: &[crate::pipeline::NodeDefinition],
) -> Vec<String> {
    let defs: HashMap<String, &crate::pipeline::NodeDefinition> =
        definitions.iter().map(|d| (d.kind.clone(), d)).collect();
    let mut warnings = vec![];
    for node in &graph.nodes {
        let Some(def) = defs.get(&node.kind) else {
            continue;
        };
        // Only check nodes that have declared flags; skip nodes with empty flag list.
        if def.dsl_flags.is_empty() {
            continue;
        }
        let known_keys: HashSet<&str> = def
            .dsl_flags
            .iter()
            .map(|f| f.config_key.as_str())
            .collect();

        let global_keys: HashSet<&str> = GLOBAL_CONFIG_KEYS.iter().copied().collect();

        if let Some(obj) = node.config.as_object() {
            for key in obj.keys() {
                if global_keys.contains(key.as_str()) {
                    continue;
                }
                if !known_keys.contains(key.as_str()) {
                    warnings.push(format!(
                        "node {} ({}): unknown config key '{}' — check flag spelling. Known: {}",
                        node.id,
                        node.kind,
                        key,
                        known_keys.iter().cloned().collect::<Vec<_>>().join(", ")
                    ));
                }
            }
        }
    }
    warnings
}

#[allow(dead_code)]
fn summarize_config(config: &Value) -> String {
    let Some(map) = config.as_object() else {
        return String::new();
    };
    let parts: Vec<String> = map
        .iter()
        .take(3)
        .map(|(k, v)| {
            let val_str = match v {
                Value::String(s) => {
                    if s.len() > 30 {
                        format!("{}…", &s[..29])
                    } else {
                        s.clone()
                    }
                }
                other => other.to_string(),
            };
            format!("{}={}", k, val_str)
        })
        .collect();
    parts.join(" ")
}

// ── Git argument policy ───────────────────────────────────────────────────────

/// Flags each allowed git subcommand may be given, in exact form.
///
/// The list is an allowlist, not a denylist, because git's flag surface grows
/// with every release and a denylist written today is wrong at the next
/// upgrade. Anything absent is refused with the flag named, so a caller learns
/// what to ask for instead of guessing.
fn git_exact_flags(subcommand: &str) -> &'static [&'static str] {
    match subcommand {
        "status" => &[
            "-s",
            "--short",
            "--long",
            "-b",
            "--branch",
            "--show-stash",
            "--porcelain",
            "-u",
            "--untracked-files",
            "--ignored",
            "-z",
            "--no-renames",
            "--renames",
            "-v",
            "--verbose",
            "--",
        ],
        "log" => &[
            "--oneline",
            "--stat",
            "--shortstat",
            "--numstat",
            "--name-only",
            "--name-status",
            "--graph",
            "--decorate",
            "--no-decorate",
            "--all",
            "--reverse",
            "--merges",
            "--no-merges",
            "--first-parent",
            "--follow",
            "-p",
            "--patch",
            "--no-patch",
            "--abbrev-commit",
            "--no-abbrev-commit",
            "--relative-date",
            "--topo-order",
            "--date-order",
            "--author-date-order",
            "--no-color",
            "-z",
            "--",
        ],
        "diff" => &[
            "--stat",
            "--shortstat",
            "--numstat",
            "--name-only",
            "--name-status",
            "--summary",
            "--check",
            "--cached",
            "--staged",
            "-p",
            "--patch",
            "--no-patch",
            "--raw",
            "-w",
            "--ignore-all-space",
            "-b",
            "--ignore-space-change",
            "--ignore-blank-lines",
            "--no-color",
            "--text",
            "-z",
            "--",
        ],
        "add" => &[
            "-A",
            "--all",
            "-u",
            "--update",
            "-n",
            "--dry-run",
            "-f",
            "--force",
            "-N",
            "--intent-to-add",
            "--ignore-removal",
            "--no-ignore-removal",
            "-v",
            "--verbose",
            "--",
        ],
        "commit" => &[
            "-a",
            "--all",
            "--amend",
            "--allow-empty",
            "--allow-empty-message",
            "--no-verify",
            "-q",
            "--quiet",
            "-v",
            "--verbose",
            "--",
        ],
        _ => &[],
    }
}

/// Flags that carry a value, allowed only in the joined `--flag=value` form.
///
/// The joined form is required so the value can be reviewed with the flag it
/// belongs to. A separated `--grep pattern` would otherwise leave `pattern`
/// looking like a positional argument and be reviewed under the wrong rule.
fn git_valued_flag_prefixes(subcommand: &str) -> &'static [&'static str] {
    match subcommand {
        "status" => &["--porcelain=", "--untracked-files=", "--ignored=", "-u"],
        "log" => &[
            "--max-count=",
            "--skip=",
            "--since=",
            "--until=",
            "--after=",
            "--before=",
            "--author=",
            "--committer=",
            "--grep=",
            "--decorate=",
            "--date=",
            "--pretty=",
            "--format=",
            "--abbrev=",
            "--unified=",
            "--find-renames=",
        ],
        "diff" => &[
            "--unified=",
            "--find-renames=",
            "--find-copies=",
            "--diff-filter=",
            "--word-diff=",
            "--abbrev=",
            "--src-prefix=",
            "--dst-prefix=",
        ],
        "add" => &["--chmod="],
        "commit" => &["--message="],
        _ => &[],
    }
}

/// Short flags whose value is glued on, in `-n5` / `-U3` shape.
fn git_glued_numeric_flags(subcommand: &str) -> &'static [char] {
    match subcommand {
        "log" => &['n'],
        "diff" => &['U'],
        _ => &[],
    }
}

/// Reviews one git invocation's arguments.
///
/// Three rules, in order:
///
/// 1. A value that a flag carries must not be a path out of the repository —
///    `--pretty=` and friends are format strings, but `--find-renames=` and the
///    pathspec forms are not, and one rule for all of them is cheaper to keep
///    right than a per-flag exception list.
/// 2. Every flag must be named in the subcommand's allowlist. This is what
///    stops `--no-index`, `--output=`, `--file=`, `--ext-diff`, `-c`,
///    `--git-dir=`, and every future equivalent.
/// 3. Every positional argument must stay inside the repository: no absolute
///    path, no `..` path segment, no null byte. `..` is refused as a *segment*
///    rather than as a substring, so `main..HEAD` and `origin/main...HEAD`
///    still name revision ranges while `../../etc/passwd` does not.
///
/// `-m` is special-cased for commit: its value is a message, not a path, and it
/// is the one flag the DSL passes in separated form.
pub(crate) fn review_git_arguments(subcommand: &str, args: &[String]) -> Result<(), String> {
    let exact = git_exact_flags(subcommand);
    let valued = git_valued_flag_prefixes(subcommand);
    let glued = git_glued_numeric_flags(subcommand);

    let mut expect_message_value = false;
    let mut pathspec_only = false;

    for arg in args {
        if arg.contains('\0') {
            return Err("argument contains a null byte".to_string());
        }
        if expect_message_value {
            expect_message_value = false;
            continue;
        }
        if pathspec_only || !arg.starts_with('-') || arg == "-" {
            review_git_positional(arg)?;
            continue;
        }
        if arg == "--" {
            pathspec_only = true;
            continue;
        }
        if subcommand == "commit" && arg == "-m" {
            expect_message_value = true;
            continue;
        }
        if subcommand == "commit" && arg.starts_with("-m") {
            continue;
        }
        if exact.contains(&arg.as_str()) {
            continue;
        }
        if let Some(prefix) = valued
            .iter()
            .find(|prefix| arg.starts_with(**prefix) && arg.len() > prefix.len())
        {
            let value = &arg[prefix.len()..];
            review_git_flag_value(arg, value)?;
            continue;
        }
        // `-5`, `-n5`, `-U3`: a count, not a path.
        let short = &arg[1..];
        let numeric_tail = short
            .strip_prefix(|ch: char| glued.contains(&ch))
            .unwrap_or(if subcommand == "log" { short } else { "" });
        if !numeric_tail.is_empty() && numeric_tail.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }
        return Err(format!(
            "'{arg}' is not an allowed option for 'git {subcommand}'. \
             Allowed: {}",
            allowed_summary(exact, valued)
        ));
    }
    Ok(())
}

fn allowed_summary(exact: &[&str], valued: &[&str]) -> String {
    let mut names: Vec<String> = exact
        .iter()
        .filter(|flag| **flag != "--")
        .map(|flag| (*flag).to_string())
        .collect();
    names.extend(valued.iter().map(|prefix| format!("{prefix}<value>")));
    names.join(", ")
}

/// A flag's value may not name a path outside the repository. A format string
/// is not a path, so only the escaping shapes are refused rather than every
/// value containing a slash.
fn review_git_flag_value(flag: &str, value: &str) -> Result<(), String> {
    if git_path_escapes(value) {
        return Err(format!(
            "the value given to '{}' leaves the repository",
            flag.split('=').next().unwrap_or(flag)
        ));
    }
    Ok(())
}

fn review_git_positional(arg: &str) -> Result<(), String> {
    if git_path_escapes(arg) {
        return Err(format!(
            "'{arg}' leaves the repository; git may only read and write paths inside it"
        ));
    }
    Ok(())
}

/// Whether a token names something outside the repository directory.
///
/// Absolute paths and Windows drive prefixes are out by construction. `..` is
/// judged per path segment: `a/../b` escapes, `main..HEAD` is a revision range
/// and does not.
fn git_path_escapes(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    let unified = token.replace('\\', "/");
    if unified.starts_with('/') || unified.starts_with("~/") || unified == "~" {
        return true;
    }
    if unified.len() >= 2
        && unified.as_bytes()[1] == b':'
        && unified.as_bytes()[0].is_ascii_alphabetic()
    {
        return true;
    }
    unified.split('/').any(|segment| segment == "..")
}

#[cfg(test)]
mod git_policy_tests {
    use super::review_git_arguments;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|arg| (*arg).to_string()).collect()
    }

    /// S6 regression. The subcommand was the only thing checked and the
    /// remaining arguments were passed to `git` verbatim, so git's own flags
    /// became a file-read and file-write primitive for anything the server
    /// process could reach. `current_dir(repo_dir)` bounds relative resolution
    /// and nothing more. Delete the `review_git_arguments` call in `cmd_git`
    /// and each of these reaches the real git.
    #[test]
    fn git_flags_cannot_be_turned_into_a_file_interface() {
        let attacks: &[(&str, &[&str])] = &[
            // Reads any file the server can read, straight into the result.
            ("diff", &["--no-index", "/etc/passwd", "/dev/null"]),
            (
                "diff",
                &["--no-index", "../../../../etc/passwd", "/dev/null"],
            ),
            // Writes anywhere.
            ("diff", &["--output=/tmp/evil"]),
            ("log", &["--output=/tmp/evil"]),
            // Reads a file into the commit message, then `git log` reads it back.
            ("commit", &["-F", "/etc/passwd"]),
            ("commit", &["--file=/etc/passwd"]),
            ("add", &["--pathspec-from-file=/etc/passwd"]),
            // Runs a program.
            ("diff", &["--ext-diff"]),
            ("log", &["--ext-diff"]),
            // Repoints git at another repository.
            ("status", &["--git-dir=/srv/other/.git"]),
            ("status", &["--work-tree=/"]),
            ("log", &["-c", "core.pager=sh -c 'id'"]),
            // Plain path traversal in a pathspec.
            ("add", &["../../../../etc/cron.d/evil"]),
            ("add", &["/etc/cron.d/evil"]),
            ("diff", &["--", "../../secrets"]),
            ("status", &["~/.ssh/id_rsa"]),
        ];
        for (subcommand, argv) in attacks {
            assert!(
                review_git_arguments(subcommand, &args(argv)).is_err(),
                "git {subcommand} {argv:?} was allowed"
            );
        }
    }

    /// The policy has to leave real usage alone, or it will be worked around.
    #[test]
    fn ordinary_git_usage_still_passes() {
        let allowed: &[(&str, &[&str])] = &[
            ("status", &[]),
            ("status", &["--short", "--branch"]),
            ("status", &["--porcelain=v1"]),
            ("log", &["--oneline", "-10"]),
            ("log", &["-n5", "--stat"]),
            ("log", &["--pretty=format:%h %s", "--date=short"]),
            ("log", &["--since=2024-01-01", "--author=alice"]),
            ("log", &["main..HEAD"]),
            ("log", &["origin/main...HEAD", "--oneline"]),
            ("log", &["--", "src/pages/index.tsx"]),
            ("diff", &["--stat"]),
            ("diff", &["--cached", "-U3"]),
            ("diff", &["HEAD~1", "HEAD"]),
            ("diff", &["--", "src/pipelines"]),
            ("add", &["."]),
            ("add", &["-A"]),
            (
                "add",
                &["src/pages/index.tsx", "src/pipelines/api/foo.zf.json"],
            ),
            ("commit", &["-m", "feat: a thing"]),
            ("commit", &["--amend", "--allow-empty"]),
            ("commit", &["-a", "-q"]),
        ];
        for (subcommand, argv) in allowed {
            review_git_arguments(subcommand, &args(argv))
                .unwrap_or_else(|error| panic!("git {subcommand} {argv:?} refused: {error}"));
        }
    }

    /// The policy has to be *reached*, not merely defined. This goes through
    /// the real `cmd_git`, which is the one path both the DSL console and the
    /// MCP `git_command` tool take.
    #[tokio::test]
    async fn the_executor_refuses_a_git_invocation_that_reads_a_file_off_the_host() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut config = crate::platform::PlatformConfig::default();
        config.data_root = tmp.path().join("platform");
        config.default_password = "secret".to_string();
        let platform = std::sync::Arc::new(
            crate::platform::services::PlatformService::from_config(config).expect("platform"),
        );
        platform
            .file
            .ensure_project_layout("superadmin", "default")
            .expect("layout");
        let executor = super::DslExecutor::new(platform, "superadmin", "default");

        let output = executor
            .execute_dsl("git diff --no-index /etc/hosts /dev/null")
            .await;
        let text = output
            .lines
            .iter()
            .map(|line| line.text.clone())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("--no-index"), "{text}");
        let host_file = std::fs::read_to_string("/etc/hosts").unwrap_or_default();
        let leaked = host_file
            .lines()
            .find(|line| line.contains("localhost") && !line.trim_start().starts_with('#'));
        if let Some(leaked) = leaked {
            assert!(
                !text.contains(leaked.trim()),
                "a host file line reached the tool result: {text}"
            );
        }

        // The same door still runs ordinary git.
        let status = executor.execute_dsl("git status --short").await;
        assert!(
            !status
                .lines
                .iter()
                .any(|line| line.text.contains("is not an allowed option")),
            "{:?}",
            status.lines
        );
    }

    #[test]
    fn a_refusal_names_the_flag_and_what_is_allowed() {
        let error = review_git_arguments("diff", &args(&["--no-index"])).unwrap_err();
        assert!(error.contains("--no-index"), "{error}");
        assert!(error.contains("--stat"), "{error}");
    }
}

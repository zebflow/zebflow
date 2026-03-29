//! PlatformOps — canonical implementations of all platform tools.
//!
//! Both `AssistantPlatformTools` and `ZebflowMcpHandler` delegate to this struct.
//! Neither should contain business logic of their own.

use std::sync::Arc;

use serde_json::{Value, json};

use crate::platform::model::{
    DescribeProjectDbConnectionRequest, TemplateCreateKind, TemplateCreateRequest, TemplateSaveRequest,
};
use crate::platform::services::PlatformService;

/// The result of a platform operation.
pub struct OpsResult {
    pub text: String,
    /// If present, the browser should navigate to this URL after the tool call.
    pub navigate: Option<String>,
}

impl OpsResult {
    pub fn ok(text: impl Into<String>) -> Self {
        Self { text: text.into(), navigate: None }
    }
    pub fn ok_nav(text: impl Into<String>, url: impl Into<String>) -> Self {
        Self { text: text.into(), navigate: Some(url.into()) }
    }
    pub fn err(msg: impl Into<String>) -> Self {
        Self { text: format!("Error: {}", msg.into()), navigate: None }
    }
}

/// Canonical implementation of all 33 platform tools.
pub struct PlatformOps {
    pub platform: Arc<PlatformService>,
    pub owner: String,
    pub project: String,
}

impl PlatformOps {
    pub fn new(platform: Arc<PlatformService>, owner: &str, project: &str) -> Self {
        Self {
            platform,
            owner: owner.to_string(),
            project: project.to_string(),
        }
    }
}

// ── Orientation ───────────────────────────────────────────────────────────────

impl PlatformOps {
    pub async fn start_here(&self) -> OpsResult {
        let owner = &self.owner;
        let project = &self.project;
        let mut out = String::new();

        // ── Overview ──────────────────────────────────────────────────────────
        out.push_str(&format!(
            "# Overview\n\
             Zebflow is a pipeline-based platform: HTTP/WS/schedule triggers chain through\n\
             nodes (query, script, AI, render) to produce APIs, pages, automations, and\n\
             real-time systems — all authored as .zf.json pipelines + TSX templates.\n\n\
             → Project current state:  ## The Project\n\
             → Platform how-to:        ## Zebflow Docs\n\n\
             Webhook URL pattern: `/wh/{owner}/{project}{{path}}`\n"
        ));

        // ── The Project ───────────────────────────────────────────────────────
        out.push_str(&format!("\n---\n\n## The Project: {owner}/{project}\n"));

        // AGENTS.md
        out.push_str("\n### AGENTS.md\n");
        match self.platform.projects.read_agent_doc(owner, project, "AGENTS.md") {
            Ok(content) => out.push_str(&content),
            Err(_) => out.push_str("(not found — create one to set project rules for all agents)"),
        }

        // Docs
        out.push_str("\n\n### Docs  [repo/docs/]\n");
        out.push_str("Read before building. Update as the project evolves.\n\n");
        match self.platform.projects.list_project_docs(owner, project) {
            Ok(docs) if !docs.is_empty() => {
                for d in &docs {
                    out.push_str(&format!("  {} → `docs_project_read(\"{}\")`\n", d.path, d.path));
                }
            }
            Ok(_) => {
                out.push_str(
                    "(none yet)\n\
                     Interview the user first — what to build, DB schema, first pages/endpoints, auth needs?\n\
                     Then write: `docs_project_write path=\"REQUIREMENTS.md\" content=...`"
                );
            }
            Err(e) => out.push_str(&format!("(error: {e})")),
        }

        // Pipelines
        out.push_str("\n\n### Pipelines\n");
        match self.platform.projects.list_pipeline_meta_rows(owner, project) {
            Ok(pipelines) if !pipelines.is_empty() => {
                let active = pipelines.iter().filter(|p| p.active_hash.is_some()).count();
                let draft = pipelines.len() - active;
                out.push_str(&format!("[{active} active, {draft} draft]\n\n"));
                for p in &pipelines {
                    out.push_str(&format!(
                        "  {} [{}] → `pipeline_describe file_rel_path=\"{}\"`\n",
                        p.file_rel_path,
                        if p.active_hash.is_some() { "active" } else { "draft" },
                        p.file_rel_path
                    ));
                }
            }
            Ok(_) => out.push_str("(none yet — use `pipeline_register` to create one)\n"),
            Err(e) => out.push_str(&format!("(error: {e})\n")),
        }

        // Templates
        out.push_str("\n### Templates  [repo/templates/]\n");
        match self.platform.projects.list_template_workspace(owner, project) {
            Ok(workspace) => {
                let files: Vec<_> = workspace.items.iter()
                    .filter(|i| i.kind == "file")
                    .collect();
                if files.is_empty() {
                    out.push_str("(none yet — use `template_create` to scaffold)\n");
                } else {
                    for item in files.iter().take(30) {
                        out.push_str(&format!(
                            "  {} → `template_get rel_path=\"{}\"`\n",
                            item.rel_path, item.rel_path
                        ));
                    }
                    if files.len() > 30 {
                        out.push_str(&format!("  ... ({} more)\n", files.len() - 30));
                    }
                }
            }
            Err(e) => out.push_str(&format!("(error: {e})\n")),
        }

        // Connections & Credentials
        out.push_str("\n### Connections & Credentials\n");
        match self.platform.db_connections.list_project_connections(owner, project) {
            Ok(items) if !items.is_empty() => {
                for c in &items {
                    out.push_str(&format!(
                        "  {} ({}) → `connection_describe slug=\"{}\" scope=tables`\n",
                        c.connection_slug, c.database_kind, c.connection_slug
                    ));
                }
            }
            Ok(_) => out.push_str("  Connections: (none — add via UI Settings → Connections)\n"),
            Err(e) => out.push_str(&format!("  Connections: (error: {e})\n")),
        }
        out.push_str("  Sekejap (embedded DB, always available) — `help(\"db/sekejap\")`\n");
        match self.platform.credentials.list_project_credentials(owner, project) {
            Ok(items) if !items.is_empty() => {
                out.push_str("  Credentials: ");
                let creds: Vec<String> = items.iter().map(|c| format!("{} ({})", c.title, c.kind)).collect();
                out.push_str(&creds.join(", "));
                out.push('\n');
            }
            Ok(_) => out.push_str("  Credentials: (none)\n"),
            Err(_) => {}
        }

        // ── Zebflow Docs ──────────────────────────────────────────────────────
        out.push_str("\n---\n\n## Zebflow Docs\n");

        // Pipeline Examples — auto from HELP array
        out.push_str("\n### Pipeline Examples\n");
        out.push_str("Full DSL recipe: `help(\"pipeline/examples/<slug>\")`\n\n");
        let examples: Vec<_> = crate::platform::help::HELP
            .iter()
            .filter(|n| n.path.starts_with("pipeline/examples/"))
            .collect();
        if examples.is_empty() {
            out.push_str("(none)\n");
        } else {
            for ex in &examples {
                let slug = ex.path.trim_start_matches("pipeline/examples/");
                out.push_str(&format!("  {:<32} — {}\n", slug, ex.excerpt));
            }
        }

        // Built-in Nodes — auto from definitions
        let node_defs = crate::pipeline::nodes::builtin_node_definitions();
        out.push_str(&format!("\n### Built-in Nodes  ({} total)\n", node_defs.len()));
        out.push_str("Full flags + schema: `help(\"pipeline/nodes/<kind>\")`\n\n");
        for d in &node_defs {
            out.push_str(&format!("  {:<28} — {}\n", d.kind, d.title));
        }

        // Reference
        out.push_str(
            "\n### Reference\n\
             `help()`                       — full docs index\n\
             `help(\"pipeline/dsl\")`          — DSL syntax\n\
             `help(\"pipeline/nodes\")`        — all nodes with every flag\n\
             `help(\"web\")`                   — TSX templates\n\
             `help_search(\"query\")`          — search all docs + nodes\n"
        );

        // ── Agent Memory ──────────────────────────────────────────────────────
        out.push_str(
            "\n---\n\n## Agent Memory\n\
             `docs_agent_read name=MEMORY.md`                      — read notes from previous sessions\n\
             `docs_agent_write name=MEMORY.md content=<notes>`     — save what you discover\n\n\
             Save: schema decisions, patterns that work, project-specific discoveries, user preferences.\n\
             Update at end of every session.\n"
        );

        OpsResult::ok(out)
    }
}

// ── Help / Knowledge ──────────────────────────────────────────────────────────

impl PlatformOps {
    /// Unified help browser. No topic = root index. Hierarchical paths:
    /// "pipeline", "pipeline/dsl", "pipeline/nodes", "pipeline/nodes/{kind}",
    /// "web", "web/hooks", "tool", "db", "db/sekejap", "platform", etc.
    pub fn help(&self, topic: &str) -> OpsResult {
        match crate::platform::help::get_help(topic) {
            Ok(content) => OpsResult::ok(content),
            Err(msg) => OpsResult::err(msg),
        }
    }

    pub fn help_search(&self, query: &str) -> OpsResult {
        let query_lower = query.to_lowercase();
        let terms: Vec<String> = query_lower
            .split_whitespace()
            .filter(|t| t.len() >= 2)
            .map(|t| t.to_string())
            .collect();

        if terms.is_empty() {
            return OpsResult::ok(
                "Provide search terms. Example: help_search(\"webhook credential\")".to_string(),
            );
        }

        // Search corpus: static HELP files + dynamic node catalog
        let all = crate::platform::help::all_searchable_content();

        struct DocMatch {
            path: String,
            title: String,
            term_coverage: usize, // distinct query terms found anywhere in doc
            chunks: Vec<String>,  // matched lines with context
        }

        let context_lines = 3usize;
        let mut doc_matches: Vec<DocMatch> = Vec::new();

        for (path, title, content) in &all {
            let lines: Vec<&str> = content.lines().collect();
            let mut chunks: Vec<String> = Vec::new();
            let mut last_end = 0usize;
            let mut i = 0;

            while i < lines.len() {
                let line_lower = lines[i].to_lowercase();
                // Any term matching this line → include with context
                let hit = terms.iter().any(|t| line_lower.contains(t.as_str()));
                if hit {
                    let start = i.saturating_sub(context_lines).max(last_end);
                    let end = (i + context_lines + 1).min(lines.len());
                    chunks.push(lines[start..end].join("\n"));
                    last_end = end;
                    i = end;
                } else {
                    i += 1;
                }
            }

            if !chunks.is_empty() {
                let doc_lower = content.to_lowercase();
                let term_coverage = terms.iter().filter(|t| doc_lower.contains(t.as_str())).count();
                doc_matches.push(DocMatch { path: path.clone(), title: title.clone(), term_coverage, chunks });
            }
        }

        if doc_matches.is_empty() {
            return OpsResult::ok(format!(
                "No results for '{}'. Try broader terms or call help() for the full index.",
                query
            ));
        }

        // Sort: most term coverage first (docs matching more of your query terms rank higher)
        doc_matches.sort_by(|a, b| b.term_coverage.cmp(&a.term_coverage).then(b.chunks.len().cmp(&a.chunks.len())));

        let mut out = format!(
            "## Search: `{}` — {} document(s) matched\n\n",
            query, doc_matches.len()
        );
        let max_chars = 8000usize;
        let mut total = out.len();
        let mut shown = 0usize;

        for dm in &doc_matches {
            if total >= max_chars { break; }
            let header = format!(
                "### `{}` — {} ({}/{} terms)\n",
                dm.path, dm.title, dm.term_coverage, terms.len()
            );
            out.push_str(&header);
            total += header.len();
            for chunk in &dm.chunks {
                if total >= max_chars { break; }
                let block = format!("```\n{}\n```\n\n", chunk);
                total += block.len();
                out.push_str(&block);
            }
            out.push_str("---\n\n");
            shown += 1;
        }

        if shown < doc_matches.len() {
            out.push_str(&format!(
                "*{} more result(s) not shown — narrow your query or call `help(\"path\")` directly.*\n",
                doc_matches.len() - shown
            ));
        }

        OpsResult::ok(out)
    }
}

// ── Pipelines ─────────────────────────────────────────────────────────────────

impl PlatformOps {
    pub fn pipeline_list(&self) -> OpsResult {
        match self.platform.projects.list_pipeline_meta_rows(&self.owner, &self.project) {
            Ok(pipelines) => OpsResult::ok(
                serde_json::to_string_pretty(&json!({ "pipelines": pipelines, "count": pipelines.len() }))
                    .unwrap_or_default()
            ),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub fn pipeline_get(&self, file_rel_path: &str) -> OpsResult {
        match self.platform.projects.get_pipeline_meta_by_file_id(&self.owner, &self.project, file_rel_path) {
            Err(e) => OpsResult::err(e.to_string()),
            Ok(None) => OpsResult::err(format!("Pipeline '{file_rel_path}' not found")),
            Ok(Some(meta)) => {
                match self.platform.projects.read_pipeline_source(&self.owner, &self.project, &meta.file_rel_path) {
                    Ok(source) => OpsResult::ok(
                        serde_json::to_string_pretty(&json!({ "meta": meta, "source": source }))
                            .unwrap_or_default()
                    ),
                    Err(e) => OpsResult::err(e.to_string()),
                }
            }
        }
    }

    pub async fn pipeline_register(
        &self,
        body: &str,
        file_rel_path: Option<&str>,
        name: Option<&str>,
        path: Option<&str>,
        title: Option<&str>,
    ) -> OpsResult {
        let frp = match file_rel_path {
            Some(frp) => frp.to_string(),
            None => {
                let n = name.unwrap_or_default();
                let raw_path = path.unwrap_or("/");
                let vpath = raw_path.trim_matches('/');
                if vpath.is_empty() {
                    format!("pipelines/{n}")
                } else {
                    format!("pipelines/{vpath}/{n}")
                }
            }
        };
        let mut dsl = format!("register {frp}");
        if let Some(t) = title {
            dsl.push_str(&format!(" --title \"{t}\""));
        }
        dsl.push(' ');
        dsl.push_str(body);

        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(), &self.owner, &self.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        let text = output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
        let nav = format!("/projects/{}/{}/pipelines/registry?path=/", self.owner, self.project);
        OpsResult::ok_nav(text, nav)
    }

    pub async fn pipeline_describe(&self, file_rel_path: &str) -> OpsResult {
        let dsl = format!("describe pipeline {file_rel_path}");
        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(), &self.owner, &self.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        OpsResult::ok(output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n"))
    }

    pub async fn pipeline_patch(
        &self,
        file_rel_path: &str,
        node_id: &str,
        flags: Option<&str>,
        body: Option<&str>,
    ) -> OpsResult {
        let mut dsl = format!("patch pipeline {file_rel_path} node {node_id}");
        if let Some(f) = flags {
            dsl.push(' ');
            dsl.push_str(f);
        }
        if let Some(b) = body {
            dsl.push_str(&format!(" -- {b}"));
        }
        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(), &self.owner, &self.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        OpsResult::ok(output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n"))
    }

    pub async fn pipeline_activate(&self, file_rel_path: &str) -> OpsResult {
        let dsl = format!("activate pipeline {file_rel_path}");
        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(), &self.owner, &self.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        let text = output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
        let nav = format!("/projects/{}/{}/pipelines/registry?path=/", self.owner, self.project);
        OpsResult::ok_nav(text, nav)
    }

    pub async fn pipeline_deactivate(&self, file_rel_path: &str) -> OpsResult {
        let dsl = format!("deactivate pipeline {file_rel_path}");
        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(), &self.owner, &self.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        OpsResult::ok(output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n"))
    }

    pub async fn pipeline_execute(&self, file_rel_path: &str, input: Option<&str>) -> OpsResult {
        let mut dsl = format!("execute pipeline {file_rel_path}");
        if let Some(i) = input {
            dsl.push_str(&format!(" --input {i}"));
        }
        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(), &self.owner, &self.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        OpsResult::ok(output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n"))
    }

    pub async fn pipeline_run(&self, body: &str, input: Option<Value>) -> OpsResult {
        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(), &self.owner, &self.project,
        );
        let output = executor.execute_run_with_input(body, input).await;
        OpsResult::ok(output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n"))
    }
}

// ── Templates ─────────────────────────────────────────────────────────────────

impl PlatformOps {
    pub fn template_list(&self) -> OpsResult {
        match self.platform.projects.list_template_workspace(&self.owner, &self.project) {
            Ok(workspace) => OpsResult::ok(serde_json::to_string_pretty(&workspace).unwrap_or_default()),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub fn template_get(&self, rel_path: &str) -> OpsResult {
        match self.platform.projects.read_template_file(&self.owner, &self.project, rel_path) {
            Ok(content) => OpsResult::ok(content),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub fn template_create(&self, kind: &str, name: &str, parent_rel_path: Option<&str>) -> OpsResult {
        let kind = match kind {
            "page" => TemplateCreateKind::Page,
            "component" => TemplateCreateKind::Component,
            "script" => TemplateCreateKind::Script,
            "folder" => TemplateCreateKind::Folder,
            other => return OpsResult::err(format!(
                "Invalid kind '{other}'. Must be: page, component, script, folder"
            )),
        };
        let req = TemplateCreateRequest {
            kind,
            name: name.to_string(),
            parent_rel_path: parent_rel_path.map(|s| s.to_string()),
        };
        match self.platform.projects.create_template_entry(&self.owner, &self.project, &req) {
            Ok(payload) => {
                let nav = format!("/projects/{}/{}/files", self.owner, self.project);
                OpsResult::ok_nav(serde_json::to_string_pretty(&payload).unwrap_or_default(), nav)
            }
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub fn template_write(&self, rel_path: &str, content: &str) -> OpsResult {
        let req = TemplateSaveRequest {
            rel_path: rel_path.to_string(),
            content: content.to_string(),
        };
        match self.platform.projects.write_template_file(&self.owner, &self.project, &req) {
            Ok(payload) => {
                let nav = format!("/projects/{}/{}/files", self.owner, self.project);
                OpsResult::ok_nav(serde_json::to_string_pretty(&payload).unwrap_or_default(), nav)
            }
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub fn template_search(&self, pattern: &str, glob: Option<&str>, context: usize) -> OpsResult {
        if pattern.trim().is_empty() {
            return OpsResult::err("pattern must not be empty");
        }
        match self.platform.projects.search_template_files(&self.owner, &self.project, pattern, glob, context) {
            Err(e) => OpsResult::err(e.to_string()),
            Ok(matches) if matches.is_empty() => OpsResult::ok(format!(
                "No matches for '{}' in templates{}.",
                pattern,
                glob.map(|g| format!(" (glob: {g})")).unwrap_or_default()
            )),
            Ok(matches) => {
                let mut out = format!("{} match(es) for '{}':\n\n", matches.len(), pattern);
                for (rel, line_no, block) in &matches {
                    if context == 0 {
                        out.push_str(&format!("{}:{}: {}\n", rel, line_no, block.trim()));
                    } else {
                        out.push_str(&format!("{}:{}:\n```\n{}\n```\n\n", rel, line_no, block));
                    }
                }
                OpsResult::ok(out)
            }
        }
    }

    pub fn pipeline_search(&self, pattern: &str, glob: Option<&str>, context: usize) -> OpsResult {
        if pattern.trim().is_empty() {
            return OpsResult::err("pattern must not be empty");
        }
        match self.platform.projects.search_pipeline_files(&self.owner, &self.project, pattern, glob, context) {
            Err(e) => OpsResult::err(e.to_string()),
            Ok(matches) if matches.is_empty() => OpsResult::ok(format!(
                "No matches for '{}' in pipelines{}.",
                pattern,
                glob.map(|g| format!(" (glob: {g})")).unwrap_or_default()
            )),
            Ok(matches) => {
                let mut out = format!("{} match(es) for '{}':\n\n", matches.len(), pattern);
                for (rel, line_no, block) in &matches {
                    if context == 0 {
                        out.push_str(&format!("{}:{}: {}\n", rel, line_no, block.trim()));
                    } else {
                        out.push_str(&format!("{}:{}:\n```\n{}\n```\n\n", rel, line_no, block));
                    }
                }
                OpsResult::ok(out)
            }
        }
    }

    pub fn template_edit(&self, rel_path: &str, old_string: &str, new_string: &str) -> OpsResult {
        if old_string.is_empty() {
            return OpsResult::err("old_string must not be empty");
        }
        match self.platform.projects.edit_template_file(&self.owner, &self.project, rel_path, old_string, new_string) {
            Ok(line_no) => OpsResult::ok(format!(
                "Replaced at line {} in {}.", line_no, rel_path
            )),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    /// Rename or reorganize a pipeline or template file.
    ///
    /// Domain is detected automatically:
    /// - `.zf.json` extension (or `pipelines/` prefix) → pipeline domain
    /// - anything else → template domain
    ///
    /// Cross-domain moves (pipeline ↔ template) are rejected.
    /// For pipelines: deactivate → move → re-activate lifecycle is handled automatically.
    /// Parent folders are created automatically.
    pub async fn move_resource(&self, from_path: &str, to_path: &str) -> OpsResult {
        let from_path = from_path.trim();
        let to_path = to_path.trim();

        if from_path.is_empty() || to_path.is_empty() {
            return OpsResult::err("from_path and to_path must not be empty");
        }

        let from_is_pipeline = pipeline_path_heuristic(from_path);
        let to_is_pipeline = pipeline_path_heuristic(to_path);

        if from_is_pipeline != to_is_pipeline {
            return OpsResult::err(
                "Cross-domain move not supported. \
                 Pipeline paths end with .zf.json; template paths do not. \
                 Cannot mix the two in a single move.",
            );
        }

        if from_is_pipeline {
            self.move_pipeline(from_path, to_path).await
        } else {
            self.move_template(from_path, to_path)
        }
    }

    async fn move_pipeline(&self, from_path: &str, to_path: &str) -> OpsResult {
        use crate::platform::services::project::normalize_pipeline_file_rel_path;
        let owner = &self.owner;
        let project = &self.project;
        let projects = &self.platform.projects;
        let runtime = &self.platform.pipeline_runtime;

        let from = normalize_pipeline_file_rel_path(from_path);
        let to = normalize_pipeline_file_rel_path(to_path);

        if from == to {
            return OpsResult::err("from_path and to_path resolve to the same pipeline");
        }

        // Check destination doesn't already exist
        match projects.get_pipeline_meta_by_file_id(owner, project, &to) {
            Err(e) => return OpsResult::err(e.to_string()),
            Ok(Some(_)) => return OpsResult::err(format!("Destination '{}' already exists", to)),
            Ok(None) => {}
        }

        // Load source metadata
        let meta = match projects.get_pipeline_meta_by_file_id(owner, project, &from) {
            Err(e) => return OpsResult::err(e.to_string()),
            Ok(None) => return OpsResult::err(format!("Pipeline '{}' not found", from)),
            Ok(Some(m)) => m,
        };
        let was_active = meta.active_hash.is_some();

        // Read source JSON
        let source = match projects.read_pipeline_source(owner, project, &from) {
            Err(e) => return OpsResult::err(e.to_string()),
            Ok(s) => s,
        };

        // Patch the "id" field to the new file_rel_path
        let new_source = match patch_pipeline_id_field(&source, &to) {
            Ok(s) => s,
            Err(e) => return OpsResult::err(format!("Failed to patch pipeline id: {e}")),
        };

        // Register at new path (creates file + DB entry, not yet active)
        if let Err(e) = projects.upsert_pipeline_definition(
            owner, project, &to,
            &meta.title, &meta.description, &meta.trigger_kind,
            &new_source,
        ) {
            return OpsResult::err(format!("Failed to register at new path: {e}"));
        }

        // Re-activate at new path if was active before
        if was_active {
            if let Err(e) = projects.activate_pipeline_definition(owner, project, &to) {
                return OpsResult::err(format!("Failed to activate at new path: {e}"));
            }
            let _ = runtime.refresh_pipeline(owner, project, &to);
        }

        // Remove old pipeline (file + DB metadata)
        if let Err(e) = projects.delete_pipeline(owner, project, &from) {
            return OpsResult::err(format!("Failed to delete old pipeline: {e}"));
        }

        // Evict old path from runtime (it was already deleted from DB so refresh_pipeline would fail)
        runtime.evict(owner, project, &from);

        OpsResult::ok(format!(
            "Moved pipeline {} → {}{}",
            from, to,
            if was_active { " (re-activated at new path)" } else { " (draft)" }
        ))
    }

    fn move_template(&self, from_path: &str, to_path: &str) -> OpsResult {
        let layout = match self.platform.file.ensure_project_layout(&self.owner, &self.project) {
            Err(e) => return OpsResult::err(e.to_string()),
            Ok(l) => l,
        };

        let root = &layout.repo_pipelines_dir;
        let from_abs = root.join(from_path);
        let to_abs = root.join(to_path);

        if !from_abs.starts_with(root) || !to_abs.starts_with(root) {
            return OpsResult::err("Path escapes template root");
        }

        if !from_abs.is_file() {
            return OpsResult::err(format!("Template '{}' not found", from_path));
        }

        if to_abs.exists() {
            return OpsResult::err(format!("Destination '{}' already exists", to_path));
        }

        if let Some(parent) = to_abs.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return OpsResult::err(format!("Failed to create parent dirs: {e}"));
            }
        }

        if let Err(e) = std::fs::rename(&from_abs, &to_abs) {
            return OpsResult::err(format!("Failed to move file: {e}"));
        }

        OpsResult::ok(format!("Moved template {} → {}", from_path, to_path))
    }
}

/// Returns true if the path looks like a pipeline file.
fn pipeline_path_heuristic(path: &str) -> bool {
    path.ends_with(".zf.json") || path.starts_with("pipelines/")
}

/// Parses JSON source, sets the `"id"` field to `new_file_rel_path`, returns pretty-printed JSON.
fn patch_pipeline_id_field(source: &str, new_file_rel_path: &str) -> Result<String, String> {
    let mut obj: serde_json::Value =
        serde_json::from_str(source).map_err(|e| format!("invalid JSON: {e}"))?;
    if let Some(map) = obj.as_object_mut() {
        map.insert(
            "id".to_string(),
            serde_json::Value::String(new_file_rel_path.to_string()),
        );
    }
    serde_json::to_string_pretty(&obj).map_err(|e| format!("serialize error: {e}"))
}

// ── Project Docs ──────────────────────────────────────────────────────────────

impl PlatformOps {
    pub fn docs_project_list(&self) -> OpsResult {
        match self.platform.projects.list_project_docs(&self.owner, &self.project) {
            Ok(docs) => OpsResult::ok(
                serde_json::to_string_pretty(&json!({ "docs": docs, "count": docs.len() }))
                    .unwrap_or_default()
            ),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub fn docs_project_read(&self, path: &str) -> OpsResult {
        match self.platform.projects.read_project_doc(&self.owner, &self.project, path) {
            Ok(content) => OpsResult::ok(content),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub fn docs_project_write(&self, path: &str, content: &str) -> OpsResult {
        match self.platform.projects.upsert_project_doc(&self.owner, &self.project, path, content) {
            Ok(doc) => OpsResult::ok(serde_json::to_string_pretty(&doc).unwrap_or_default()),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }
}

// ── Agent Docs ────────────────────────────────────────────────────────────────

impl PlatformOps {
    pub fn docs_agent_list(&self) -> OpsResult {
        match self.platform.projects.list_agent_docs(&self.owner, &self.project) {
            Ok(docs) => OpsResult::ok(serde_json::to_string_pretty(&docs).unwrap_or_default()),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub fn docs_agent_read(&self, name: &str) -> OpsResult {
        match self.platform.projects.read_agent_doc(&self.owner, &self.project, name) {
            Ok(content) => OpsResult::ok(content),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub fn docs_agent_write(&self, name: &str, content: &str) -> OpsResult {
        match self.platform.projects.upsert_agent_doc(&self.owner, &self.project, name, content) {
            Ok(()) => OpsResult::ok(format!("{name} written successfully.")),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }
}

// ── Connections & Credentials ─────────────────────────────────────────────────

impl PlatformOps {
    pub fn connection_list(&self) -> OpsResult {
        match self.platform.db_connections.list_project_connections(&self.owner, &self.project) {
            Ok(items) => OpsResult::ok(
                serde_json::to_string_pretty(&json!({
                    "connections": items.iter().map(|c| json!({
                        "slug": c.connection_slug,
                        "label": c.connection_label,
                        "kind": c.database_kind,
                    })).collect::<Vec<_>>(),
                    "count": items.len(),
                })).unwrap_or_default()
            ),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub async fn connection_describe(
        &self,
        slug: &str,
        scope: Option<&str>,
        schema: Option<&str>,
        table: Option<&str>,
    ) -> OpsResult {
        let conn = match self.platform.db_connections.get_project_connection(&self.owner, &self.project, slug) {
            Err(e) => return OpsResult::err(e.to_string()),
            Ok(None) => return OpsResult::err(format!("Connection '{slug}' not found")),
            Ok(Some(c)) => c,
        };

        let req = DescribeProjectDbConnectionRequest {
            scope: if table.is_some() {
                Some("columns".to_string())
            } else {
                scope.map(|s| s.to_string())
            },
            schema: schema.map(|s| s.to_string()),
            table: table.map(|s| s.to_string()),
            include_system: Some(false),
        };

        match self.platform.db_runtime.describe_connection(&self.owner, &self.project, &conn.connection_id, &req).await {
            Ok(result) => OpsResult::ok(format_describe_for_llm(&result)),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub fn credential_list(&self) -> OpsResult {
        match self.platform.credentials.list_project_credentials(&self.owner, &self.project) {
            Ok(items) => OpsResult::ok(
                serde_json::to_string_pretty(&json!({
                    "credentials": items.iter().map(|c| {
                        let mut entry = json!({
                            "id": c.credential_id,
                            "title": c.title,
                            "kind": c.kind,
                            "notes": c.notes,
                        });
                        if !c.auth_roles.is_empty() {
                            entry["auth_roles"] = json!(c.auth_roles);
                        }
                        entry
                    }).collect::<Vec<_>>(),
                    "count": items.len(),
                })).unwrap_or_default()
            ),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }
}

// ── Git ───────────────────────────────────────────────────────────────────────

impl PlatformOps {
    pub async fn git_command(&self, subcommand: &str, args: Option<&str>, message: Option<&str>) -> OpsResult {
        let mut dsl = format!("git {subcommand}");
        if let Some(a) = args {
            dsl.push(' ');
            dsl.push_str(a);
        }
        if let Some(m) = message {
            dsl.push_str(&format!(" -- {m}"));
        }
        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(), &self.owner, &self.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        OpsResult::ok(output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n"))
    }
}

// ── UI Catalog ────────────────────────────────────────────────────────────────

impl PlatformOps {
    pub fn list_ui_catalog(&self) -> OpsResult {
        match self.platform.file.ensure_project_layout(&self.owner, &self.project) {
            Ok(layout) => {
                let shared_ui_dir = layout.repo_pipelines_dir.join("shared").join("ui");
                let entries = crate::platform::catalog::CatalogService::list_ui_with_presence(&shared_ui_dir);
                OpsResult::ok(serde_json::to_string_pretty(&entries).unwrap_or_default())
            }
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub fn install_ui_components(&self, names: Vec<String>, overwrite: Option<bool>) -> OpsResult {
        match self.platform.file.ensure_project_layout(&self.owner, &self.project) {
            Ok(layout) => {
                let shared_ui_dir = layout.repo_pipelines_dir.join("shared").join("ui");
                match crate::platform::catalog::CatalogService::install_ui(
                    &names,
                    &shared_ui_dir,
                    overwrite.unwrap_or(false),
                ) {
                    Ok(report) => OpsResult::ok(serde_json::to_string_pretty(&report).unwrap_or_default()),
                    Err(e) => OpsResult::err(e),
                }
            }
            Err(e) => OpsResult::err(e.to_string()),
        }
    }
}

// ── DB describe formatter ─────────────────────────────────────────────────────

/// Format a DB describe result as compact LLM-readable text.
///
/// Output example:
/// ```
/// # mydb (postgresql) — scope: tables
///
/// public.users
/// - id: uuid, PK, default:uuid_generate_v4()
/// - role_id: uuid, NOT NULL, → public.roles.id
/// ```
fn format_describe_for_llm(
    result: &crate::platform::model::ProjectDbConnectionDescribeResult,
) -> String {
    use crate::platform::model::DbObjectNode;

    fn format_table(node: &DbObjectNode) -> String {
        let mut out = String::new();
        let schema = node.schema.as_deref().unwrap_or("public");
        out.push_str(&format!("\n{}.{}\n", schema, node.name));

        if let Some(cols) = node.meta.get("columns").and_then(|v| v.as_array()) {
            for col in cols {
                let name = col["name"].as_str().unwrap_or("?");
                let typ = col["type"].as_str().unwrap_or("?");
                let nullable = col["nullable"].as_bool().unwrap_or(true);
                let is_pk = col.get("pk").and_then(|v| v.as_bool()).unwrap_or(false);
                let fk = col.get("fk");
                let default = col.get("default").and_then(|v| v.as_str());

                let mut parts: Vec<String> = vec![typ.to_string()];

                if is_pk {
                    parts.push("PK".to_string());
                } else if !nullable {
                    parts.push("NOT NULL".to_string());
                }

                if let Some(fk) = fk {
                    let rs = fk["schema"].as_str().unwrap_or("?");
                    let rt = fk["table"].as_str().unwrap_or("?");
                    let rc = fk["column"].as_str().unwrap_or("?");
                    parts.push(format!("→ {rs}.{rt}.{rc}"));
                }

                if let Some(d) = default {
                    let truncated = if d.len() > 48 {
                        format!("{}…", &d[..48])
                    } else {
                        d.to_string()
                    };
                    parts.push(format!("default:{truncated}"));
                }

                out.push_str(&format!("- {name}: {}\n", parts.join(", ")));
            }
        }
        out
    }

    let mut out = format!(
        "# {} ({}) — scope: {}\n",
        result.connection_slug, result.database_kind, result.scope
    );

    for node in &result.nodes {
        match node.kind.as_str() {
            "schema" => {
                if !node.children.is_empty() {
                    for child in &node.children {
                        if child.kind == "table" {
                            out.push_str(&format_table(child));
                        } else if child.kind == "function" {
                            let s = child.schema.as_deref().unwrap_or("public");
                            out.push_str(&format!("\nfn {s}.{}\n", child.name));
                        }
                    }
                }
            }
            "table" => {
                out.push_str(&format_table(node));
            }
            "function" => {
                let s = node.schema.as_deref().unwrap_or("public");
                out.push_str(&format!("\nfn {s}.{}\n", node.name));
            }
            _ => {}
        }
    }

    out
}

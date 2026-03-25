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

        let mut sections: Vec<String> = Vec::new();

        sections.push(format!(
            "# Zebflow Platform Overview\n\
             Project: {owner}/{project}\n\n\
             Zebflow is a pipeline-based reactive web platform.\n\
             - **Pipelines**: chain of nodes that handle HTTP, WebSocket, or schedule triggers\n\
             - **Web pages (TSX)**: server renders HTML from project templates, optional browser hydration\n\
             - **Nodes**: trigger.webhook, pg.query, sekejap.query, script, web.render, ws.emit, ...\n\
             - **Credentials**: stored secrets referenced by slug in pipeline --credential flags\n\
             - **DB Connections**: named database connections (postgres, sekejap) with schema inspect",
        ));

        // Project git repo section
        {
            let git_section = match self.platform.file.ensure_project_layout(owner, project) {
                Ok(layout) => {
                    let git_status = std::process::Command::new("git")
                        .args(["status", "--short"])
                        .current_dir(&layout.repo_dir)
                        .output()
                        .ok()
                        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                        .unwrap_or_default();

                    let status_block = if git_status.is_empty() {
                        "  (working tree clean — nothing to commit)".to_string()
                    } else {
                        git_status.lines().map(|l| format!("  {l}")).collect::<Vec<_>>().join("\n")
                    };

                    let has_remote = std::process::Command::new("git")
                        .args(["remote", "get-url", "origin"])
                        .current_dir(&layout.repo_dir)
                        .output()
                        .map(|o| o.status.success())
                        .unwrap_or(false);

                    let remote_warning = if has_remote {
                        String::new()
                    } else {
                        "⚠️ **No remote configured** — changes are only tracked locally. \
                         Ask the user to set a remote so work is backed up:\n\
                         `git remote add origin <url>` then `git push -u origin main`\n\n"
                            .to_string()
                    };

                    format!(
                        "## Project Repository\n\
                         \n\
                         All project files are git-tracked — pipelines, templates, docs, assets, styles, \
                         and `zebflow.json`. **Always commit after making changes** so every edit is recorded.\n\
                         \n\
                         ```\n\
                         git_command subcommand=add args=\".\"\n\
                         git_command subcommand=commit message=\"describe what you built\"\n\
                         ```\n\
                         \n\
                         {remote_warning}\
                         **Current status:**\n\
                         {status_block}",
                    )
                }
                Err(_) => "## Project Git Repo\n(could not read repo layout)".to_string(),
            };
            sections.push(git_section);
        }

        // Project docs list
        match self.platform.projects.list_project_docs(owner, project) {
            Ok(docs) if !docs.is_empty() => {
                sections.push(format!(
                    "## Project Docs (repo/docs/)\n{}",
                    docs.iter().map(|d| format!("- {} → docs_project_read(\"{}\")", d.path, d.path)).collect::<Vec<_>>().join("\n")
                ));
            }
            Ok(_) => {
                sections.push("## Project Docs\n(none yet — use docs_project_write to create)".to_string());
            }
            Err(e) => {
                sections.push(format!("## Project Docs\n(error: {e})"));
            }
        }

        // AGENTS.md content
        match self.platform.projects.read_agent_doc(owner, project, "AGENTS.md") {
            Ok(content) => {
                sections.push(format!("## AGENTS.md (Project Instructions)\n{content}"));
            }
            Err(_) => {
                sections.push("## AGENTS.md\n(not found — create with docs_agent_write name=AGENTS.md)".to_string());
            }
        }

        // DB connections
        match self.platform.db_connections.list_project_connections(owner, project) {
            Ok(items) if !items.is_empty() => {
                let lines: Vec<String> = items.iter().map(|c| {
                    format!("- {} ({}) — kind: {}", c.connection_slug, c.connection_label, c.database_kind)
                }).collect();

                let has_schema_doc = self.platform.projects
                    .list_project_docs(owner, project)
                    .ok()
                    .map(|docs| docs.iter().any(|d| {
                        let p = d.path.to_lowercase();
                        p.contains("db_schema") || p.contains("db-schema") || p.contains("schema.md")
                    }))
                    .unwrap_or(false);

                let schema_hint = if has_schema_doc {
                    "  → Schema doc exists in docs/ — read it before writing queries.".to_string()
                } else {
                    "  → No schema doc yet. Run `connection_describe slug=<slug> scope=tables` on each \
                     connection, then save the output to `docs/db_schema.md` via `docs_project_write`. \
                     This gives you and future agents a persistent, token-efficient schema reference."
                        .to_string()
                };

                sections.push(format!(
                    "## DB Connections\n{}\n\n{schema_hint}",
                    lines.join("\n"),
                ));
            }
            Ok(_) => {
                sections.push("## DB Connections\n(none configured)".to_string());
            }
            Err(e) => {
                sections.push(format!("## DB Connections\n(error: {e})"));
            }
        }

        // Sekejap block
        sections.push(
            "## Sekejap (Embedded Database)\n\
             Zebflow's built-in multi-model database — graph, vector, spatial, full-text, vague temporal.\n\
             Suitable for: blog posts, user tables, AI memory, vector embeddings, event graphs, RAG indexes.\n\
             Workflow: create table in UI (Tables page) → use `n.sekejap.query` in pipelines.\n\
             Node: `n.sekejap.query --table <name> --op query|upsert`\n\
             Collections use internal prefix `sjtable__` (e.g. table \"posts\" → collection \"sjtable__posts\").\n\
             No external connection needed — scoped to project automatically."
                .to_string(),
        );

        // Template tree
        match self.platform.projects.list_template_workspace(owner, project) {
            Ok(workspace) => {
                sections.push(format!(
                    "## Template Tree\n{}",
                    serde_json::to_string_pretty(&workspace).unwrap_or_else(|_| "(parse error)".to_string())
                ));
            }
            Err(e) => {
                sections.push(format!("## Templates\n(error: {e})"));
            }
        }

        // Agent orientation
        sections.push(
            "## Agent Orientation — Read Before Acting\n\
             \n\
             Assess the project state from the sections above:\n\
             \n\
             **If the project is NEW or SPARSE** (no AGENTS.md, no docs, no pipelines, no templates):\n\
             → Do NOT start building immediately.\n\
             → Interview the user first. Ask:\n\
               1. What are you building? Describe the domain, data model, and key user flows.\n\
               2. Do you have an existing database? If yes, share the schema or connection slug.\n\
               3. What are the first 2-3 pages or API endpoints you need?\n\
               4. Any auth requirements? (login, JWT, roles?)\n\
             \n\
             **If DB connections exist but no schema doc**:\n\
             → Run `connection_describe` on each connection and save to `docs/db_schema.md`\n\
               before writing any SQL queries.\n\
             \n\
             **If AGENTS.md exists**:\n\
             → Follow those instructions. They override everything else.\n\
             \n\
             **If the user's request doesn't match the project's available data/connections**:\n\
             → Point out the mismatch and ask what they actually have before proceeding.\n\
             \n\
             Only proceed to build when you have enough context to do it correctly."
                .to_string(),
        );

        // Help pointers — auto-generated from HELP array
        let help_index = crate::platform::help::help_root_index();

        sections.push(format!(
            "## Next: Help Tools\n\
             {help_index}\n\n\
             - `pipeline_list` — see existing pipelines\n\
             - `docs_agent_read name=MEMORY.md` — read previous session notes",
        ));

        OpsResult::ok(sections.join("\n\n---\n\n"))
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
        let terms: Vec<&str> = query_lower.split_whitespace().collect();
        let all = crate::platform::help::all_help_text();

        let mut results: Vec<String> = Vec::new();
        let context_lines = 5usize;
        let max_chars = 3000usize;
        let mut total_chars = 0usize;

        'outer: for (path, title, content) in &all {
            let lines: Vec<&str> = content.lines().collect();
            let mut i = 0;
            while i < lines.len() {
                let line_lower = lines[i].to_lowercase();
                let matches = terms.iter().all(|t| line_lower.contains(t));
                if matches {
                    let start = i.saturating_sub(context_lines);
                    let end = (i + context_lines + 1).min(lines.len());
                    let chunk = lines[start..end].join("\n");
                    let entry = format!("### Match in `{}` ({})\n{}\n", path, title, chunk);
                    total_chars += entry.len();
                    results.push(entry);
                    i = end;
                    if total_chars >= max_chars { break 'outer; }
                } else {
                    i += 1;
                }
            }
        }

        if results.is_empty() {
            OpsResult::ok(format!("No results for '{query}'. Try different terms or call help() for the full index."))
        } else {
            OpsResult::ok(results.join("\n---\n\n"))
        }
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
                    "credentials": items.iter().map(|c| json!({
                        "id": c.credential_id,
                        "title": c.title,
                        "kind": c.kind,
                        "notes": c.notes,
                    })).collect::<Vec<_>>(),
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

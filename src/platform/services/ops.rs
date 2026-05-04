//! PlatformOps — canonical implementations of all platform tools.
//!
//! Both `AssistantPlatformTools` and `ZebflowMcpHandler` delegate to this struct.
//! Neither should contain business logic of their own.

use std::sync::Arc;

use serde::Serialize;
use serde_json::{Value, json};

use crate::platform::model::{
    DescribeProjectDbConnectionRequest, TemplateCreateKind, TemplateCreateRequest,
    TemplateSaveRequest,
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
        Self {
            text: text.into(),
            navigate: None,
        }
    }
    pub fn ok_nav(text: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            navigate: Some(url.into()),
        }
    }
    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            text: format!("Error: {}", msg.into()),
            navigate: None,
        }
    }
}

/// Canonical implementation of all 33 platform tools.
pub struct PlatformOps {
    pub platform: Arc<PlatformService>,
    pub owner: String,
    pub project: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectHelpSection {
    pub id: String,
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub children: Vec<ProjectHelpSection>,
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
    pub fn help_dialog_sections(&self) -> Vec<ProjectHelpSection> {
        let owner = &self.owner;
        let project = &self.project;
        let node_count = crate::pipeline::nodes::builtin_node_definitions().len();
        let example_count = crate::platform::help::HELP
            .iter()
            .filter(|n| n.path.starts_with("pipeline/examples/"))
            .count();

        let start_here = format!(
            "# Start Here\n\n\
             Zebflow turns pipeline triggers into APIs, pages, and automations.\n\n\
             - Project: `{owner}/{project}`\n\
             - Webhook base: `/wh/{owner}/{project}{{path}}`\n\
             - Built-in nodes: `{node_count}`\n\
             - Pipeline examples: `{example_count}`\n\n\
             ## Core References\n\n\
             - Node catalog: `help(\"pipeline/nodes\")`\n\
             - Pipeline DSL: `help(\"pipeline/dsl\")`\n\
             - TSX templates: `help(\"web\")`\n\
             - Pipeline examples: `help(\"pipeline/examples\")`\n\
             - Search docs: `help_search(\"query\")`\n\n\
             ## Efficient Development\n\n\
             - Debug a pipeline immediately with `pipeline_execute file_rel_path=\"...\" input={{...}}`.\n\
             - Inspect the full graph first with `pipeline_describe file_rel_path=\"...\"`.\n\
             - Patch one node with `pipeline_patch`, then `pipeline_activate` to make it live.\n\
             - Use `pipeline_get_invocations file_rel_path=\"...\"` for webhook or scheduled runs.\n\
             - Search before creating: `pipeline_search` and `template_search`.\n\n\
             ## Template Cache Note\n\n\
             If template changes do not show after agent-side edits, clear the RWE template cache. UI saves already do this automatically.\n"
        );

        fn doc(path: &str) -> String {
            crate::platform::help::get_help(path).unwrap_or_else(|err| {
                format!(
                    "# Missing Help\n\nRequested `{path}` but failed to load it.\n\n```\n{err}\n```"
                )
            })
        }

        fn join_docs(paths: &[&str]) -> String {
            paths
                .iter()
                .map(|path| doc(path))
                .collect::<Vec<_>>()
                .join("\n\n---\n\n")
        }

        fn section(
            id: &str,
            title: &str,
            content: String,
            children: Vec<ProjectHelpSection>,
        ) -> ProjectHelpSection {
            ProjectHelpSection {
                id: id.to_string(),
                title: title.to_string(),
                content,
                children,
            }
        }

        vec![
            section(
                "start",
                "Start",
                doc("guide"),
                vec![
                    section("start-here", "Start Here", start_here, vec![]),
                    section(
                        "zebflow-overview",
                        "Zebflow Overview",
                        doc("guide/overview"),
                        vec![],
                    ),
                    section(
                        "pipelines-and-templates",
                        "Pipelines + Templates",
                        join_docs(&["guide/pipelines-templates", "pipeline/index"]),
                        vec![],
                    ),
                    section(
                        "simple-blog",
                        "Simple Blog",
                        doc("guide/simple-blog"),
                        vec![],
                    ),
                ],
            ),
            section(
                "build-web-apps",
                "Build Web Apps",
                join_docs(&[
                    "guide/react-and-libraries",
                    "guide/gallery",
                    "guide/project-management",
                ]),
                vec![
                    section(
                        "react-and-internal-libraries",
                        "React + Internal Libraries",
                        join_docs(&["guide/react-and-libraries", "web/index"]),
                        vec![],
                    ),
                    section(
                        "gallery-and-examples",
                        "Gallery + Source Examples",
                        join_docs(&["guide/gallery", "pipeline/examples"]),
                        vec![],
                    ),
                    section(
                        "project-management",
                        "Project Management",
                        join_docs(&["guide/project-management", "platform/operations"]),
                        vec![],
                    ),
                ],
            ),
            section(
                "databases",
                "Databases",
                join_docs(&["db/index"]),
                vec![
                    section("sekejap", "Sekejap", join_docs(&["db/sekejap"]), vec![]),
                    section("sqlite", "SQLite", doc("guide/sqlite"), vec![]),
                    section("mapserver", "MapServer", doc("guide/mapserver"), vec![]),
                ],
            ),
            section(
                "marketplace",
                "Marketplace",
                doc("guide/marketplace"),
                vec![
                    section(
                        "frontend-libraries",
                        "Frontend Libraries",
                        doc("guide/marketplace/frontend-libraries"),
                        vec![
                            section("zeb-use", "zeb/use", doc("web/use"), vec![]),
                            section("zeb-deckgl", "zeb/deckgl", doc("web/deckgl"), vec![]),
                            section("zeb-pdf", "zeb/pdf", doc("web/pdf"), vec![]),
                            section("zeb-markdown", "zeb/markdown", doc("web/markdown"), vec![]),
                        ],
                    ),
                    section(
                        "marketplace-nodes",
                        "Nodes",
                        doc("guide/marketplace/nodes"),
                        vec![section(
                            "node-catalog",
                            "Node Catalog",
                            doc("pipeline/nodes"),
                            vec![],
                        )],
                    ),
                    section("packs", "Packs", doc("guide/marketplace/packs"), vec![]),
                    section(
                        "marketplace-how-it-works",
                        "How Marketplace Works",
                        doc("guide/marketplace/how-it-works"),
                        vec![],
                    ),
                ],
            ),
            section(
                "agentic-use",
                "Agentic Use",
                doc("guide/agentic"),
                vec![
                    section(
                        "agent-node-and-tools",
                        "Agent Node + Tools",
                        join_docs(&["guide/agentic", "platform/agent"]),
                        vec![],
                    ),
                    section(
                        "operator-console",
                        "Operator Console",
                        doc("guide/agentic/operator-console"),
                        vec![],
                    ),
                    section(
                        "mcp-by-project",
                        "MCP by Project",
                        join_docs(&["guide/agentic/mcp-by-project", "platform/agent"]),
                        vec![],
                    ),
                ],
            ),
            section(
                "federated-offices",
                "Federated Offices",
                doc("guide/federated-offices"),
                vec![],
            ),
        ]
    }

    pub async fn start_here(&self) -> OpsResult {
        let owner = &self.owner;
        let project = &self.project;
        let mut out = String::new();

        out.push_str(&format!(
            "# Zebflow MCP Start Here\n\n\
             Project scope: `{owner}/{project}`\n\
             Webhook base: `/wh/{owner}/{project}{{path}}`\n\
             Mental model: pipelines connect triggers to nodes for APIs, pages, automations, and jobs.\n\n\
             ## First Moves\n\
             1. Read the embedded AGENTS.md and MEMORY.md below.\n\
             2. For pipelines, call `pipeline_list`, then `pipeline_describe file_rel_path=\"...\" compact=true`.\n\
             3. For templates, call `template_list`, then `template_outline rel_path=\"...\"` before `template_get`.\n\
             4. For SQL, call `connection_list`, then `connection_describe slug=\"...\" scope=\"tables\"` before writing queries.\n\
             5. For syntax, call `help topic=\"pipeline/dsl\"`, `help topic=\"web\"`, or `help_search query=\"...\"`.\n"
        ));

        out.push_str("\n---\n\n## Project Instructions: AGENTS.md\n");
        match self
            .platform
            .projects
            .read_agent_doc(owner, project, "AGENTS.md")
        {
            Ok(content) => out.push_str(&content),
            Err(_) => out
                .push_str("(none — create with `docs_agent_write name=\"AGENTS.md\" content=...`)"),
        }

        if let Ok(soul) = self
            .platform
            .projects
            .read_agent_doc(owner, project, "SOUL.md")
        {
            if !soul
                .trim_start()
                .starts_with("# Soul\n\nDescribe the assistant")
                && soul.len() > 60
            {
                out.push_str("\n\n## Personality (SOUL.md)\n");
                out.push_str(&soul);
            }
        }

        out.push_str("\n\n## Project Memory: MEMORY.md\n");
        match self
            .platform
            .projects
            .read_agent_doc(owner, project, "MEMORY.md")
        {
            Ok(mem) => {
                let is_default =
                    mem.contains("_(This file is managed by the assistant.") && mem.len() < 300;
                if is_default {
                    out.push_str("(empty — write discoveries here: `docs_agent_write name=\"MEMORY.md\" content=...`)");
                } else {
                    out.push_str(&mem);
                }
            }
            Err(_) => out.push_str("(none)"),
        }

        out.push_str("\n\n---\n\n## Project Docs\n");
        match self.platform.projects.list_project_docs(owner, project) {
            Ok(docs) if !docs.is_empty() => {
                for d in &docs {
                    out.push_str(&format!(
                        "  {} -> `docs_project_read path=\"{}\"`\n",
                        d.path, d.path
                    ));
                }
            }
            Ok(_) => {
                out.push_str(
                    "(none — interview the user: what to build, DB schema, auth needs?)\n",
                );
                out.push_str("Then: `docs_project_write path=\"REQUIREMENTS.md\" content=...`\n");
            }
            Err(e) => out.push_str(&format!("(error: {e})\n")),
        }

        out.push_str("\n---\n\n## Live Project Inventory\n");

        match self
            .platform
            .projects
            .list_pipeline_meta_rows(owner, project)
        {
            Ok(ps) if !ps.is_empty() => {
                let active = ps.iter().filter(|p| p.active_hash.is_some()).count();
                let draft = ps.len() - active;
                out.push_str(&format!(
                    "\n### Pipelines [{active} active, {draft} draft]\n"
                ));
                for p in ps.iter().take(60) {
                    let status = if p.active_hash.is_some() {
                        "active"
                    } else {
                        "draft"
                    };
                    let trigger = if !p.trigger_kind.is_empty() {
                        format!(" | {}", p.trigger_kind)
                    } else {
                        String::new()
                    };
                    out.push_str(&format!("  {} [{status}{trigger}]\n", p.file_rel_path));
                }
                if ps.len() > 60 {
                    out.push_str(&format!("  ... ({} more)\n", ps.len() - 60));
                }
                out.push_str(
                    "  -> `pipeline_describe file_rel_path=\"...\" compact=true` for node IDs and key config\n",
                );
            }
            Ok(_) => {
                out.push_str("\n### Pipelines\n  (none — use `pipeline_register` to create)\n")
            }
            Err(e) => out.push_str(&format!("\n### Pipelines\n  (error: {e})\n")),
        }

        match self
            .platform
            .projects
            .list_template_workspace(owner, project)
        {
            Ok(workspace) => {
                let files: Vec<_> = workspace
                    .items
                    .iter()
                    .filter(|i| i.kind == "file")
                    .collect();
                out.push_str(&format!("\n### Templates [{} files]\n", files.len()));
                if files.is_empty() {
                    out.push_str("  (none — use `template_create` to scaffold)\n");
                } else {
                    for item in files.iter().take(40) {
                        let tag = template_type_tag(&item.rel_path);
                        out.push_str(&format!("  [{}] {}\n", tag, item.rel_path));
                    }
                    if files.len() > 40 {
                        out.push_str(&format!("  ... ({} more)\n", files.len() - 40));
                    }
                    out.push_str("  -> `template_outline rel_path=\"...\"` first, then `template_get rel_path=\"...\"` when content is needed\n");
                }
            }
            Err(e) => out.push_str(&format!("\n### Templates\n  (error: {e})\n")),
        }

        out.push_str("\n### Connections & Credentials\n");
        match self
            .platform
            .db_connections
            .list_project_connections(owner, project)
        {
            Ok(items) if !items.is_empty() => {
                for c in &items {
                    out.push_str(&format!(
                        "  {} ({}) -> `connection_describe slug=\"{}\" scope=\"tables\"`\n",
                        c.connection_slug, c.database_kind, c.connection_slug
                    ));
                }
            }
            Ok(_) => out.push_str("  (none — add via UI Settings → Connections)\n"),
            Err(e) => out.push_str(&format!("  (error: {e})\n")),
        }
        match self
            .platform
            .credentials
            .list_project_credentials(owner, project)
        {
            Ok(items) if !items.is_empty() => {
                let creds: Vec<String> = items
                    .iter()
                    .map(|c| format!("{} ({})", c.title, c.kind))
                    .collect();
                out.push_str(&format!("  Credentials: {}\n", creds.join(", ")));
            }
            Ok(_) => out.push_str("  Credentials: (none)\n"),
            Err(_) => {}
        }

        let git = self.git_command("log", Some("--oneline -8"), None).await;
        if !git.text.starts_with("Error") && !git.text.trim().is_empty() {
            out.push_str("\n---\n\n## Recent Git Activity\n");
            for line in git.text.lines().take(8) {
                out.push_str(&format!("  {line}\n"));
            }
        }

        let node_count = crate::pipeline::nodes::builtin_node_definitions().len();
        let example_count = crate::platform::help::HELP
            .iter()
            .filter(|n| n.path.starts_with("pipeline/examples/"))
            .count();
        out.push_str(&format!(
            "\n---\n\n## MCP Tool Map\n\
             Pipeline DSL: `help topic=\"pipeline/dsl\"`; node catalog: `help topic=\"pipeline/nodes\"` ({node_count} built-in nodes)\n\
             Web templates: `help topic=\"web\"`; examples: `help topic=\"pipeline/examples\"` ({example_count} recipes)\n\
             Search docs: `help_search query=\"...\"`\n\
             Read/write project docs: `docs_project_list`, `docs_project_read`, `docs_project_write`\n\
             Read/write agent memory: `docs_agent_read name=\"MEMORY.md\"`, `docs_agent_write name=\"MEMORY.md\" content=...`\n\
             Inspect code cheaply: `template_outline`, `template_deps`; edit with `template_edit` or `template_batch_edit`\n\
             Full agent workflow: `help topic=\"platform/workflow\"`\n"
        ));

        out.push_str(
            "\n## Operational Rules\n\
             - Before patching a pipeline, call `pipeline_describe` and use the returned node IDs.\n\
             - After `pipeline_register` or `pipeline_patch`, call `pipeline_activate` before expecting traffic to use it.\n\
             - When testing function pipelines, pass an explicit `input` object.\n\
             - After meaningful work, update `MEMORY.md` with durable project facts.\n"
        );

        OpsResult::ok(out)
    }
}

// ── Help / Knowledge ──────────────────────────────────────────────────────────

impl PlatformOps {
    /// Unified help browser. No topic = root index. Hierarchical paths:
    /// "pipeline", "pipeline/dsl", "pipeline/nodes", "pipeline/nodes/{kind}",
    /// "web", "web/hooks", "tool", "db", "platform", etc.
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
                let term_coverage = terms
                    .iter()
                    .filter(|t| doc_lower.contains(t.as_str()))
                    .count();
                doc_matches.push(DocMatch {
                    path: path.clone(),
                    title: title.clone(),
                    term_coverage,
                    chunks,
                });
            }
        }

        if doc_matches.is_empty() {
            return OpsResult::ok(format!(
                "No results for '{}'. Try broader terms or call help() for the full index.",
                query
            ));
        }

        // Sort: most term coverage first (docs matching more of your query terms rank higher)
        doc_matches.sort_by(|a, b| {
            b.term_coverage
                .cmp(&a.term_coverage)
                .then(b.chunks.len().cmp(&a.chunks.len()))
        });

        let mut out = format!(
            "## Search: `{}` — {} document(s) matched\n\n",
            query,
            doc_matches.len()
        );
        let max_chars = 8000usize;
        let mut total = out.len();
        let mut shown = 0usize;

        for dm in &doc_matches {
            if total >= max_chars {
                break;
            }
            let header = format!(
                "### `{}` — {} ({}/{} terms)\n",
                dm.path,
                dm.title,
                dm.term_coverage,
                terms.len()
            );
            out.push_str(&header);
            total += header.len();
            for chunk in &dm.chunks {
                if total >= max_chars {
                    break;
                }
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
        match self
            .platform
            .projects
            .list_pipeline_meta_rows(&self.owner, &self.project)
        {
            Ok(pipelines) => OpsResult::ok(
                serde_json::to_string_pretty(
                    &json!({ "pipelines": pipelines, "count": pipelines.len() }),
                )
                .unwrap_or_default(),
            ),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub fn pipeline_get(&self, file_rel_path: &str, node_id: Option<&str>) -> OpsResult {
        let meta_opt = self
            .platform
            .projects
            .get_pipeline_meta_by_file_id(&self.owner, &self.project, file_rel_path)
            .ok()
            .flatten();

        // Exact match found — use it.
        if let Some(meta) = meta_opt {
            return match self.platform.projects.read_pipeline_source(
                &self.owner,
                &self.project,
                &meta.file_rel_path,
            ) {
                Ok(source) => {
                    // If node_id filter is set, extract just that node.
                    if let Some(nid) = node_id.filter(|s| !s.is_empty()) {
                        return extract_pipeline_node(&source, nid, &meta.file_rel_path);
                    }
                    OpsResult::ok(
                        serde_json::to_string_pretty(&json!({ "meta": meta, "source": source }))
                            .unwrap_or_default(),
                    )
                }
                Err(e) => OpsResult::err(e.to_string()),
            };
        }

        // Fuzzy fallback: substring match on file_rel_path across all catalog entries.
        let rows = match self
            .platform
            .projects
            .list_pipeline_meta_rows(&self.owner, &self.project)
        {
            Ok(r) => r,
            Err(e) => return OpsResult::err(e.to_string()),
        };
        let needle = file_rel_path.to_lowercase();
        let candidates: Vec<String> = rows
            .iter()
            .filter(|m| m.file_rel_path.to_lowercase().contains(&needle))
            .map(|m| m.file_rel_path.clone())
            .collect();
        match candidates.len() {
            0 => OpsResult::err(format!("Pipeline '{file_rel_path}' not found")),
            1 => self.pipeline_get(&candidates[0], node_id),
            _ => OpsResult::err(format!(
                "Ambiguous: '{}' matches {} pipelines — use exact path:\n{}",
                file_rel_path,
                candidates.len(),
                candidates
                    .iter()
                    .map(|p| format!("  {p}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            )),
        }
    }

    pub async fn pipeline_register(
        &self,
        body: &str,
        file_rel_path: Option<&str>,
        name: Option<&str>,
        path: Option<&str>,
        title: Option<&str>,
        description: Option<&str>,
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
        if let Some(d) = description.filter(|d| !d.trim().is_empty()) {
            dsl.push_str(&format!(" --description \"{d}\""));
        }
        dsl.push(' ');
        dsl.push_str(body);

        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &self.owner,
            &self.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        let text = output
            .lines
            .iter()
            .map(|l| l.text.clone())
            .collect::<Vec<_>>()
            .join("\n");
        let nav = format!(
            "/projects/{}/{}/pipelines/registry?path=/",
            self.owner, self.project
        );
        OpsResult::ok_nav(text, nav)
    }

    pub async fn pipeline_describe(&self, file_rel_path: &str, compact: bool) -> OpsResult {
        let mut dsl = format!("describe pipeline {file_rel_path}");
        if compact {
            dsl.push_str(" --compact");
        }
        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &self.owner,
            &self.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        OpsResult::ok(
            output
                .lines
                .iter()
                .map(|l| l.text.clone())
                .collect::<Vec<_>>()
                .join("\n"),
        )
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
            self.platform.clone(),
            &self.owner,
            &self.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        OpsResult::ok(
            output
                .lines
                .iter()
                .map(|l| l.text.clone())
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }

    pub async fn pipeline_activate(&self, file_rel_path: &str) -> OpsResult {
        let dsl = format!("activate pipeline {file_rel_path}");
        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &self.owner,
            &self.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        let text = output
            .lines
            .iter()
            .map(|l| l.text.clone())
            .collect::<Vec<_>>()
            .join("\n");
        let nav = format!(
            "/projects/{}/{}/pipelines/registry?path=/",
            self.owner, self.project
        );
        OpsResult::ok_nav(text, nav)
    }

    /// Activate all pipelines whose `file_rel_path` matches the given glob pattern.
    /// Reports per-pipeline success/fail and a summary count.
    pub async fn pipeline_activate_glob(&self, glob: &str) -> OpsResult {
        if glob.trim().is_empty() {
            return OpsResult::err("glob must not be empty");
        }
        let rows = match self
            .platform
            .projects
            .list_pipeline_meta_rows(&self.owner, &self.project)
        {
            Ok(r) => r,
            Err(e) => return OpsResult::err(e.to_string()),
        };

        // Reuse the same glob matcher used by template_search / pipeline_search.
        let matching: Vec<String> = rows
            .iter()
            .filter(|m| {
                crate::platform::services::project::pipeline_glob_matches(glob, &m.file_rel_path)
            })
            .map(|m| m.file_rel_path.clone())
            .collect();

        if matching.is_empty() {
            return OpsResult::err(format!("No pipelines match glob '{glob}'"));
        }

        let mut ok_count = 0usize;
        let mut fail_count = 0usize;
        let mut lines = Vec::new();

        for frp in &matching {
            match self.platform.projects.activate_pipeline_definition(
                &self.owner,
                &self.project,
                frp,
            ) {
                Ok(_) => {
                    let _ = self.platform.pipeline_runtime.refresh_pipeline(
                        &self.owner,
                        &self.project,
                        frp,
                    );
                    ok_count += 1;
                    lines.push(format!("  ✓ {frp}"));
                }
                Err(e) => {
                    fail_count += 1;
                    lines.push(format!("  ✗ {frp}  — {}", e.message));
                }
            }
        }

        let summary = format!(
            "Activated {ok_count}/{} pipeline(s) matching '{glob}'{fail}.",
            matching.len(),
            fail = if fail_count > 0 {
                format!(" ({fail_count} failed)")
            } else {
                String::new()
            }
        );
        let nav = format!(
            "/projects/{}/{}/pipelines/registry?path=/",
            self.owner, self.project
        );
        OpsResult::ok_nav(format!("{summary}\n{}", lines.join("\n")), nav)
    }

    pub async fn pipeline_deactivate(&self, file_rel_path: &str) -> OpsResult {
        let dsl = format!("deactivate pipeline {file_rel_path}");
        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &self.owner,
            &self.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        OpsResult::ok(
            output
                .lines
                .iter()
                .map(|l| l.text.clone())
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }

    pub async fn pipeline_execute(&self, file_rel_path: &str, input: Option<&str>) -> OpsResult {
        let mut dsl = format!("execute pipeline {file_rel_path}");
        if let Some(i) = input {
            // Wrap in single quotes so the DSL tokenizer treats JSON objects (which may
            // contain spaces) as a single token. JSON does not use single quotes so this
            // is safe. The tokenizer strips the outer single quotes before parsing.
            dsl.push_str(&format!(" --input '{i}'"));
        }
        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &self.owner,
            &self.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        OpsResult::ok(
            output
                .lines
                .iter()
                .map(|l| l.text.clone())
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }

    pub async fn pipeline_run(&self, body: &str, input: Option<Value>) -> OpsResult {
        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &self.owner,
            &self.project,
        );
        let output = executor.execute_run_with_input(body, input).await;
        OpsResult::ok(
            output
                .lines
                .iter()
                .map(|l| l.text.clone())
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }

    pub fn pipeline_get_invocations(&self, file_rel_path: &str) -> OpsResult {
        match self.platform.data.get_pipeline_invocations(
            &self.owner,
            &self.project,
            file_rel_path,
            None,
        ) {
            Ok(entries) if entries.is_empty() => OpsResult::ok(format!(
                "No invocations recorded for '{}'.\n\nNote: invocations are stored per pipeline run. Try running or executing the pipeline first.",
                file_rel_path
            )),
            Ok(entries) => {
                let mut out = format!("{} — {} invocation(s)\n", file_rel_path, entries.len());
                for (i, inv) in entries.iter().enumerate() {
                    let ts = chrono::DateTime::from_timestamp(inv.at, 0)
                        .map(|dt: chrono::DateTime<chrono::Utc>| {
                            dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
                        })
                        .unwrap_or_else(|| inv.at.to_string());
                    out.push_str(&format!(
                        "\n[{}] {} | {} | {} | {}ms\n",
                        i + 1,
                        ts,
                        inv.status,
                        inv.trigger,
                        inv.duration_ms
                    ));
                    if let Some(ref err) = inv.error {
                        out.push_str(&format!("    ERROR: {}\n", err));
                    }
                    for entry in &inv.trace {
                        let marker = if entry.error.is_none() { "✓" } else { "✗" };
                        let err_part = entry
                            .error
                            .as_deref()
                            .map(|e| format!("  → {}", e))
                            .unwrap_or_default();
                        out.push_str(&format!(
                            "    {}  {}  ({})  {}ms{}\n",
                            marker, entry.node_id, entry.node_kind, entry.duration_ms, err_part
                        ));
                    }
                }
                OpsResult::ok(out)
            }
            Err(e) => OpsResult::err(format!("Error: {}", e.message)),
        }
    }
}

// ── Templates ─────────────────────────────────────────────────────────────────

impl PlatformOps {
    pub fn template_list(&self, glob: Option<&str>) -> OpsResult {
        match self
            .platform
            .projects
            .list_template_workspace(&self.owner, &self.project)
        {
            Ok(workspace) => {
                if let Some(g) = glob.filter(|s| !s.is_empty()) {
                    // Filter items by glob and prune empty folders.
                    let filtered_items: Vec<_> = workspace
                        .items
                        .iter()
                        .filter(|item| {
                            if item.kind == "folder" {
                                return false;
                            }
                            crate::platform::services::project::template_glob_matches(
                                g,
                                &item.rel_path,
                            )
                        })
                        .cloned()
                        .collect();
                    let result = json!({
                        "items": filtered_items,
                        "count": filtered_items.len(),
                        "glob": g,
                    });
                    OpsResult::ok(serde_json::to_string_pretty(&result).unwrap_or_default())
                } else {
                    OpsResult::ok(serde_json::to_string_pretty(&workspace).unwrap_or_default())
                }
            }
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub fn template_get(
        &self,
        rel_path: &str,
        offset: Option<u32>,
        limit: Option<u32>,
    ) -> OpsResult {
        // Resolve content — try exact match first, then fuzzy fallback.
        let (resolved_path, content) =
            match self
                .platform
                .projects
                .read_template_file(&self.owner, &self.project, rel_path)
            {
                Ok(content) => (rel_path.to_string(), content),
                Err(_) => {
                    // Fuzzy fallback
                    let listing = match self
                        .platform
                        .projects
                        .list_template_workspace(&self.owner, &self.project)
                    {
                        Ok(l) => l,
                        Err(e) => return OpsResult::err(e.to_string()),
                    };
                    let needle = rel_path.to_lowercase();
                    let candidates: Vec<String> = listing
                        .items
                        .iter()
                        .filter(|item| {
                            item.kind != "folder" && item.rel_path.to_lowercase().contains(&needle)
                        })
                        .map(|item| item.rel_path.clone())
                        .collect();
                    match candidates.len() {
                        0 => return OpsResult::err(format!("Template '{rel_path}' not found")),
                        1 => match self.platform.projects.read_template_file(
                            &self.owner,
                            &self.project,
                            &candidates[0],
                        ) {
                            Ok(content) => (candidates[0].clone(), content),
                            Err(e) => return OpsResult::err(e.to_string()),
                        },
                        _ => {
                            return OpsResult::err(format!(
                                "Ambiguous: '{}' matches {} templates — use exact path:\n{}",
                                rel_path,
                                candidates.len(),
                                candidates
                                    .iter()
                                    .map(|p| format!("  {p}"))
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            ));
                        }
                    }
                }
            };

        // If offset/limit provided, return a line-numbered slice.
        if offset.is_some() || limit.is_some() {
            let lines: Vec<&str> = content.lines().collect();
            let total = lines.len();
            let start = offset.unwrap_or(1).max(1) as usize; // 1-based
            let count = limit.unwrap_or(total as u32) as usize;
            let start_idx = (start - 1).min(total);
            let end_idx = (start_idx + count).min(total);
            let slice = &lines[start_idx..end_idx];

            let mut out = format!(
                "# {} (lines {}-{} of {})\n",
                resolved_path,
                start_idx + 1,
                end_idx,
                total
            );
            for (i, line) in slice.iter().enumerate() {
                out.push_str(&format!("{}| {}\n", start_idx + i + 1, line));
            }
            return OpsResult::ok(out);
        }

        // Full content (existing behavior).
        if resolved_path != rel_path {
            OpsResult::ok(format!("// resolved: {}\n{content}", resolved_path))
        } else {
            OpsResult::ok(content)
        }
    }

    pub fn template_outline(&self, rel_path: &str) -> OpsResult {
        let content =
            match self
                .platform
                .projects
                .read_template_file(&self.owner, &self.project, rel_path)
            {
                Ok(c) => c,
                Err(e) => return OpsResult::err(e.to_string()),
            };
        let result =
            crate::platform::services::tsx_outline::extract_outline(&content, Some(rel_path));
        OpsResult::ok(crate::platform::services::tsx_outline::format_outline(
            rel_path, &result,
        ))
    }

    pub fn template_deps(&self, rel_path: &str) -> OpsResult {
        let content =
            match self
                .platform
                .projects
                .read_template_file(&self.owner, &self.project, rel_path)
            {
                Ok(c) => c,
                Err(e) => return OpsResult::err(e.to_string()),
            };

        // Forward deps: what this file imports
        let import_sources =
            crate::platform::services::tsx_outline::extract_import_sources(&content);

        let mut out = format!(
            "# {} — dependency graph\n\n## Imports ({})\n",
            rel_path,
            import_sources.len()
        );
        for src in &import_sources {
            out.push_str(&format!("  {}\n", src));
        }

        // Reverse deps: which files import this one
        // Build patterns that would reference this file
        let base = rel_path.trim_end_matches(".tsx").trim_end_matches(".ts");
        let patterns: Vec<String> = vec![
            format!("@/{}", rel_path),
            format!("@/{}", base),
            format!("\"{}\"", rel_path),
            format!("\"{}\"", base),
        ];

        let workspace = match self
            .platform
            .projects
            .list_template_workspace(&self.owner, &self.project)
        {
            Ok(w) => w,
            Err(_) => return OpsResult::ok(out),
        };

        let mut importers: Vec<String> = Vec::new();
        for item in &workspace.items {
            if item.kind == "folder" || item.rel_path == rel_path {
                continue;
            }
            let file_content = match self.platform.projects.read_template_file(
                &self.owner,
                &self.project,
                &item.rel_path,
            ) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let lower = file_content.to_lowercase();
            for pat in &patterns {
                if lower.contains(&pat.to_lowercase()) {
                    importers.push(item.rel_path.clone());
                    break;
                }
            }
        }

        out.push_str(&format!(
            "\n## Imported by ({} file{})\n",
            importers.len(),
            if importers.len() == 1 { "" } else { "s" }
        ));
        if importers.is_empty() {
            out.push_str("  (none found)\n");
        } else {
            for imp in &importers {
                out.push_str(&format!("  {}\n", imp));
            }
        }

        OpsResult::ok(out)
    }

    pub fn template_batch_edit(&self, edits: &[(String, String, String)]) -> OpsResult {
        if edits.is_empty() {
            return OpsResult::err("edits list must not be empty");
        }
        let mut results: Vec<String> = Vec::new();
        for (i, (rel_path, old_string, new_string)) in edits.iter().enumerate() {
            if old_string.is_empty() {
                results.push(format!("[{}] {} — SKIP: old_string empty", i + 1, rel_path));
                continue;
            }
            match self.platform.projects.edit_template_file(
                &self.owner,
                &self.project,
                rel_path,
                old_string,
                new_string,
            ) {
                Ok(line_no) => {
                    results.push(format!("[{}] {} line {} — ok", i + 1, rel_path, line_no));
                }
                Err(e) => {
                    results.push(format!("[{}] {} — ERROR: {}", i + 1, rel_path, e));
                    // Fail fast: stop on first error.
                    break;
                }
            }
        }
        OpsResult::ok(results.join("\n"))
    }

    pub fn template_create(
        &self,
        kind: &str,
        name: &str,
        parent_rel_path: Option<&str>,
    ) -> OpsResult {
        let kind = match kind {
            "page" => TemplateCreateKind::Page,
            "component" => TemplateCreateKind::Component,
            "script" => TemplateCreateKind::Script,
            "folder" => TemplateCreateKind::Folder,
            other => {
                return OpsResult::err(format!(
                    "Invalid kind '{other}'. Must be: page, component, script, folder"
                ));
            }
        };
        let req = TemplateCreateRequest {
            kind,
            name: name.to_string(),
            parent_rel_path: parent_rel_path.map(|s| s.to_string()),
        };
        match self
            .platform
            .projects
            .create_template_entry(&self.owner, &self.project, &req)
        {
            Ok(payload) => {
                let nav = format!("/projects/{}/{}/files", self.owner, self.project);
                OpsResult::ok_nav(
                    serde_json::to_string_pretty(&payload).unwrap_or_default(),
                    nav,
                )
            }
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub fn template_write(&self, rel_path: &str, content: &str) -> OpsResult {
        let req = TemplateSaveRequest {
            rel_path: rel_path.to_string(),
            content: content.to_string(),
        };
        match self
            .platform
            .projects
            .write_template_file(&self.owner, &self.project, &req)
        {
            Ok(payload) => {
                let nav = format!("/projects/{}/{}/files", self.owner, self.project);
                OpsResult::ok_nav(
                    serde_json::to_string_pretty(&payload).unwrap_or_default(),
                    nav,
                )
            }
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub fn template_search(
        &self,
        pattern: &str,
        glob: Option<&str>,
        context: usize,
        head_limit: Option<u32>,
        output_mode: Option<&str>,
    ) -> OpsResult {
        if pattern.trim().is_empty() {
            return OpsResult::err("pattern must not be empty");
        }
        match self.platform.projects.search_template_files(
            &self.owner,
            &self.project,
            pattern,
            glob,
            context,
        ) {
            Err(e) => OpsResult::err(e.to_string()),
            Ok(matches) if matches.is_empty() => OpsResult::ok(format!(
                "No matches for '{}' in templates{}.",
                pattern,
                glob.map(|g| format!(" (glob: {g})")).unwrap_or_default()
            )),
            Ok(matches) => format_search_results(&matches, pattern, head_limit, output_mode),
        }
    }

    pub fn pipeline_search(
        &self,
        pattern: &str,
        glob: Option<&str>,
        context: usize,
        head_limit: Option<u32>,
        output_mode: Option<&str>,
    ) -> OpsResult {
        if pattern.trim().is_empty() {
            return OpsResult::err("pattern must not be empty");
        }
        match self.platform.projects.search_pipeline_files(
            &self.owner,
            &self.project,
            pattern,
            glob,
            context,
        ) {
            Err(e) => OpsResult::err(e.to_string()),
            Ok(matches) if matches.is_empty() => OpsResult::ok(format!(
                "No matches for '{}' in pipelines{}.",
                pattern,
                glob.map(|g| format!(" (glob: {g})")).unwrap_or_default()
            )),
            Ok(matches) => format_search_results(&matches, pattern, head_limit, output_mode),
        }
    }

    pub fn template_edit(&self, rel_path: &str, old_string: &str, new_string: &str) -> OpsResult {
        if old_string.is_empty() {
            return OpsResult::err("old_string must not be empty");
        }
        match self.platform.projects.edit_template_file(
            &self.owner,
            &self.project,
            rel_path,
            old_string,
            new_string,
        ) {
            Ok(line_no) => OpsResult::ok(format!("Replaced at line {} in {}.", line_no, rel_path)),
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
            owner,
            project,
            &to,
            &meta.title,
            &meta.description,
            &meta.trigger_kind,
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
            from,
            to,
            if was_active {
                " (re-activated at new path)"
            } else {
                " (draft)"
            }
        ))
    }

    fn move_template(&self, from_path: &str, to_path: &str) -> OpsResult {
        let layout = match self
            .platform
            .file
            .ensure_project_layout(&self.owner, &self.project)
        {
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

/// Single-letter type tag for a template file based on its path prefix.
/// P=page, C=component, L=layout, S=script/behavior, F=other file.
fn template_type_tag(rel_path: &str) -> &'static str {
    if rel_path.starts_with("pages/") {
        "P"
    } else if rel_path.starts_with("components/") {
        "C"
    } else if rel_path.starts_with("layout/") {
        "L"
    } else if rel_path.starts_with("scripts/") || rel_path.starts_with("behavior/") {
        "S"
    } else {
        "F"
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
        match self
            .platform
            .projects
            .list_project_docs(&self.owner, &self.project)
        {
            Ok(docs) => OpsResult::ok(
                serde_json::to_string_pretty(&json!({ "docs": docs, "count": docs.len() }))
                    .unwrap_or_default(),
            ),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub fn docs_project_read(&self, path: &str) -> OpsResult {
        match self
            .platform
            .projects
            .read_project_doc(&self.owner, &self.project, path)
        {
            Ok(content) => OpsResult::ok(content),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub fn docs_project_write(&self, path: &str, content: &str) -> OpsResult {
        match self
            .platform
            .projects
            .upsert_project_doc(&self.owner, &self.project, path, content)
        {
            Ok(doc) => OpsResult::ok(serde_json::to_string_pretty(&doc).unwrap_or_default()),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }
}

// ── Agent Docs ────────────────────────────────────────────────────────────────

impl PlatformOps {
    pub fn docs_agent_list(&self) -> OpsResult {
        match self
            .platform
            .projects
            .list_agent_docs(&self.owner, &self.project)
        {
            Ok(docs) => OpsResult::ok(serde_json::to_string_pretty(&docs).unwrap_or_default()),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub fn docs_agent_read(&self, name: &str) -> OpsResult {
        match self
            .platform
            .projects
            .read_agent_doc(&self.owner, &self.project, name)
        {
            Ok(content) => OpsResult::ok(content),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub fn docs_agent_write(&self, name: &str, content: &str) -> OpsResult {
        match self
            .platform
            .projects
            .upsert_agent_doc(&self.owner, &self.project, name, content)
        {
            Ok(()) => OpsResult::ok(format!("{name} written successfully.")),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }
}

// ── Connections & Credentials ─────────────────────────────────────────────────

impl PlatformOps {
    pub fn connection_list(&self) -> OpsResult {
        match self
            .platform
            .db_connections
            .list_project_connections(&self.owner, &self.project)
        {
            Ok(items) => OpsResult::ok(
                serde_json::to_string_pretty(&json!({
                    "connections": items.iter().map(|c| json!({
                        "slug": c.connection_slug,
                        "label": c.connection_label,
                        "kind": c.database_kind,
                    })).collect::<Vec<_>>(),
                    "count": items.len(),
                }))
                .unwrap_or_default(),
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
        let conn = match self.platform.db_connections.get_project_connection(
            &self.owner,
            &self.project,
            slug,
        ) {
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

        match self
            .platform
            .db_runtime
            .describe_connection(&self.owner, &self.project, &conn.connection_id, &req)
            .await
        {
            Ok(result) => OpsResult::ok(format_describe_for_llm(&result)),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub fn credential_list(&self) -> OpsResult {
        match self
            .platform
            .credentials
            .list_project_credentials(&self.owner, &self.project)
        {
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
                }))
                .unwrap_or_default(),
            ),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }
}

// ── Git ───────────────────────────────────────────────────────────────────────

impl PlatformOps {
    pub async fn git_command(
        &self,
        subcommand: &str,
        args: Option<&str>,
        message: Option<&str>,
    ) -> OpsResult {
        let mut dsl = format!("git {subcommand}");
        if let Some(a) = args {
            dsl.push(' ');
            dsl.push_str(a);
        }
        if let Some(m) = message {
            dsl.push_str(&format!(" -- {m}"));
        }
        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &self.owner,
            &self.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        OpsResult::ok(
            output
                .lines
                .iter()
                .map(|l| l.text.clone())
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }
}

// ── UI Catalog ────────────────────────────────────────────────────────────────

impl PlatformOps {
    pub fn list_ui_catalog(&self) -> OpsResult {
        match self
            .platform
            .file
            .ensure_project_layout(&self.owner, &self.project)
        {
            Ok(layout) => {
                let shared_ui_dir = layout.repo_pipelines_dir.join("shared").join("ui");
                let entries =
                    crate::platform::catalog::CatalogService::list_ui_with_presence(&shared_ui_dir);
                OpsResult::ok(serde_json::to_string_pretty(&entries).unwrap_or_default())
            }
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    pub fn install_ui_components(&self, names: Vec<String>, overwrite: Option<bool>) -> OpsResult {
        match self
            .platform
            .file
            .ensure_project_layout(&self.owner, &self.project)
        {
            Ok(layout) => {
                let shared_ui_dir = layout.repo_pipelines_dir.join("shared").join("ui");
                match crate::platform::catalog::CatalogService::install_ui(
                    &names,
                    &shared_ui_dir,
                    overwrite.unwrap_or(false),
                ) {
                    Ok(report) => {
                        OpsResult::ok(serde_json::to_string_pretty(&report).unwrap_or_default())
                    }
                    Err(e) => OpsResult::err(e),
                }
            }
            Err(e) => OpsResult::err(e.to_string()),
        }
    }
}

// ── Search result formatter ────────────────────────────────────────────────────

/// Format search matches with optional head_limit and output_mode.
fn format_search_results(
    matches: &[(String, usize, String)],
    pattern: &str,
    head_limit: Option<u32>,
    output_mode: Option<&str>,
) -> OpsResult {
    let mode = output_mode.unwrap_or("content");
    let limit = head_limit.map(|n| n as usize).unwrap_or(usize::MAX);

    match mode {
        "files_with_matches" => {
            // Deduplicate by file path.
            let mut seen = std::collections::HashSet::new();
            let mut files: Vec<&str> = Vec::new();
            for (rel, _, _) in matches {
                if seen.insert(rel.as_str()) {
                    files.push(rel);
                }
            }
            let total = files.len();
            let shown: Vec<&&str> = files.iter().take(limit).collect();
            let mut out = format!("{} file(s) match '{}':\n", total, pattern);
            for f in &shown {
                out.push_str(&format!("  {}\n", f));
            }
            if shown.len() < total {
                out.push_str(&format!("  ... ({} more)\n", total - shown.len()));
            }
            OpsResult::ok(out)
        }
        _ => {
            // "content" mode — existing behavior with optional limit.
            let total = matches.len();
            let capped: Vec<&(String, usize, String)> = matches.iter().take(limit).collect();
            let mut out = format!("{} match(es) for '{}'", total, pattern);
            if capped.len() < total {
                out.push_str(&format!(" (showing first {})", capped.len()));
            }
            out.push_str(":\n\n");
            for (rel, line_no, block) in &capped {
                if block.contains('\n') {
                    out.push_str(&format!("{}:{}:\n```\n{}\n```\n\n", rel, line_no, block));
                } else {
                    out.push_str(&format!("{}:{}: {}\n", rel, line_no, block.trim()));
                }
            }
            if capped.len() < total {
                out.push_str(&format!(
                    "... {} more match(es) not shown.\n",
                    total - capped.len()
                ));
            }
            OpsResult::ok(out)
        }
    }
}

// ── Pipeline node extractor ───────────────────────────────────────────────────

/// Extract a single node from a pipeline JSON source by ID, kind, or kind[index].
fn extract_pipeline_node(source: &str, node_id: &str, file_rel_path: &str) -> OpsResult {
    let graph: Value = match serde_json::from_str(source) {
        Ok(v) => v,
        Err(e) => return OpsResult::err(format!("Invalid pipeline JSON: {e}")),
    };

    let nodes = match graph.get("nodes").and_then(|n| n.as_array()) {
        Some(n) => n,
        None => return OpsResult::err("Pipeline has no 'nodes' array"),
    };

    // Parse node_id: could be "n0", "trigger.webhook", "pg.query[1]"
    let (kind_filter, index_filter) = if node_id.contains('[') {
        // kind[index] form
        let parts: Vec<&str> = node_id.splitn(2, '[').collect();
        let kind = parts[0];
        let idx: usize = parts
            .get(1)
            .and_then(|s| s.trim_end_matches(']').parse().ok())
            .unwrap_or(0);
        (Some(kind.to_string()), Some(idx))
    } else if node_id.contains('.') || node_id.contains(':') {
        // Looks like a kind (e.g. "trigger.webhook", "pg.query")
        (Some(node_id.to_string()), None)
    } else {
        // Opaque ID
        (None, None)
    };

    let mut found: Option<&Value> = None;

    if let Some(ref kind) = kind_filter {
        let mut kind_matches: Vec<&Value> = Vec::new();
        for node in nodes {
            let nk = node.get("kind").and_then(|v| v.as_str()).unwrap_or("");
            if nk == kind {
                kind_matches.push(node);
            }
        }
        if let Some(idx) = index_filter {
            found = kind_matches.get(idx).copied();
        } else if kind_matches.len() == 1 {
            found = Some(kind_matches[0]);
        } else if kind_matches.len() > 1 {
            return OpsResult::err(format!(
                "Multiple nodes match kind '{}' — use {}[0], {}[1], etc.\n{} matches found.",
                kind,
                kind,
                kind,
                kind_matches.len()
            ));
        }
    } else {
        // Opaque ID match
        for node in nodes {
            let nid = node.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if nid == node_id {
                found = Some(node);
                break;
            }
        }
    }

    match found {
        Some(node) => OpsResult::ok(serde_json::to_string_pretty(node).unwrap_or_default()),
        None => OpsResult::err(format!("Node '{}' not found in {}", node_id, file_rel_path)),
    }
}

// ── DB describe formatter ─────────────────────────────────────────────────────

/// Format a DB describe result as compact LLM-readable text.
///
/// Output example:
/// ```text
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

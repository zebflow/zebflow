//! PlatformOps — canonical implementations of all platform tools.
//!
//! Both `AssistantPlatformTools` and `ZebflowMcpHandler` delegate to this struct.
//! Neither should contain business logic of their own.

use std::sync::Arc;

use serde::Serialize;
use serde_json::{Value, json};

use crate::contracts::kinds::decode_pipeline_graph;
use crate::platform::model::{
    DescribeProjectDbConnectionRequest,
    PIPELINE_DEFINITION_EXTENSION,
    PipelineMeta,
    RepoTreeScope,
    ResolvedProjectLayout,
    TemplateSaveRequest,
    TemplateTreeItem,
};
use crate::platform::services::PlatformService;
use crate::platform::services::project::name_from_file_rel_path;

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

#[derive(Debug, Clone, Default)]
pub struct PipelineListOptions<'a> {
    pub query: Option<&'a str>,
    pub glob: Option<&'a str>,
    pub status: Option<&'a str>,
    pub trigger_kind: Option<&'a str>,
    pub limit: Option<u32>,
    pub format: Option<&'a str>,
}

#[derive(Debug, Clone, Default)]
pub struct FileListOptions<'a> {
    pub query: Option<&'a str>,
    pub glob: Option<&'a str>,
    pub kind: Option<&'a str>,
    pub limit: Option<u32>,
    pub format: Option<&'a str>,
}

#[derive(Debug, Clone, Default)]
struct TemplateSemanticMeta {
    title: String,
    description: String,
    keywords: Vec<String>,
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

fn official_node_count() -> usize {
    crate::pipeline::nodes::builtin_node_definitions().len()
        + crate::platform::services::node_registry::NodeRegistryService::embedded_official_definitions().len()
}

// ── Orientation ───────────────────────────────────────────────────────────────

impl PlatformOps {
    pub fn help_dialog_sections(&self) -> Vec<ProjectHelpSection> {
        let owner = &self.owner;
        let project = &self.project;
        let node_count = official_node_count();
        let example_count = crate::platform::help::HELP
            .iter()
            .filter(|n| n.path.starts_with("pipeline/examples/"))
            .count();

        let start_here = format!(
            "# Start Here\n\n\
             Zebflow turns pipeline triggers into APIs, pages, and automations.\n\n\
             - Project: `{owner}/{project}`\n\
             - Site: `http://{project}.{owner}.localhost:<port>/` (every route you build, links and redirects written as `/path`); neutral form `/wh/{owner}/{project}{{path}}`. Fetch any route with `route_fetch path=\"{{path}}\"`\n\
             - Official nodes: `{node_count}`\n\
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
             - Search before creating: `pipeline_search` and `file_search`.\n\n\
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
                "hub",
                "Hub",
                doc("guide/hub"),
                vec![
                    section(
                        "frontend-libraries",
                        "Frontend Libraries",
                        doc("guide/hub/frontend-libraries"),
                        vec![
                            section("zeb-use", "zeb/use", doc("web/use"), vec![]),
                            section("zeb-deckgl", "zeb/deckgl", doc("web/deckgl"), vec![]),
                            section("zeb-pdf", "zeb/pdf", doc("web/pdf"), vec![]),
                            section("zeb-markdown", "zeb/markdown", doc("web/markdown"), vec![]),
                        ],
                    ),
                    section(
                        "hub-nodes",
                        "Nodes",
                        doc("guide/hub/nodes"),
                        vec![section(
                            "node-catalog",
                            "Node Catalog",
                            doc("pipeline/nodes"),
                            vec![],
                        )],
                    ),
                    section("packs", "Packages", doc("guide/hub/packs"), vec![]),
                    section(
                        "hub-how-it-works",
                        "How Hub Works",
                        doc("guide/hub/how-it-works"),
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
            section(
                "credential-encryption",
                "Credential Encryption",
                doc("guide/credential-encryption"),
                vec![],
            ),
            section(
                "sending-mail",
                "Sending Mail",
                doc("guide/sending-mail"),
                vec![],
            ),
        ]
    }

    /// What an agent reads first. The shape was chosen by an ablation on
    /// haiku, sonnet and grok (research/zebflow/ablation/2026-09-18-start-here):
    /// a frame that separates the platform from the project, the project's
    /// files in reading order, a task→material table instead of a flat skill
    /// list, and counts instead of inventories — because an inventory dump
    /// taught the weakest model what this project has and nothing about what
    /// Zebflow is, and a table gave it a direction to open next.
    pub async fn start_here(&self) -> OpsResult {
        let owner = &self.owner;
        let project = &self.project;
        let mut out = String::new();

        let (title, description) = self
            .platform
            .zebflow_cfg
            .read_or_default(owner, project)
            .map(|c| (c.metadata.title.trim().to_string(), c.metadata.description.trim().to_string()))
            .unwrap_or_default();
        let title = if title.is_empty() { project.to_string() } else { title };
        let about = if description.is_empty() { String::new() } else { format!(" — {description}") };

        out.push_str(&format!(
            "# Start here\n\n\
             You are an agent doing **full-stack development of one project, \"{title}\"{about}, on Zebflow, a unified full-stack runtime.**\n\n\
             **Zebflow** is the platform: one running instance, many projects. It provides the runtime (pipelines of nodes, TSX pages on `zeb/react` + `zeb/ui`, databases, files, auth), the node catalogue, the `zeb/*` libraries and the skills. \
             **The project** `{owner}/{project}` is one site built on it, with its own repository, store, databases, credentials and hosts. Every tool in this session is scoped to this project. \
             Project knowledge is in the project's files; Zebflow knowledge is in `help` and the skills. Do not confuse the two.\n\n\
             Site: `http://{project}.{owner}.localhost:<port>/` — write links and redirects as `/path`; neutral form `/wh/{owner}/{project}{{path}}`.\n\n\
             ## Step 1 — the project (now)\n"
        ));

        // The project's own documents, brief first when there is one.
        let tree = self.platform.projects.list_repo_tree(owner, project, &RepoTreeScope::all());
        let mut docs: Vec<String> = tree
            .as_ref()
            .map(|l| l.items.iter().filter(|i| i.file_kind == "doc").map(|i| i.rel_path.clone()).collect())
            .unwrap_or_default();
        docs.sort_by_key(|d| {
            let l = d.to_ascii_lowercase();
            (!(l.ends_with("docs/brief.md") || l == "docs/brief.md"), !l.starts_with("docs/"), l)
        });
        if docs.is_empty() {
            out.push_str("- No project documents yet — interview the user (what to build, for whom, data, auth), then `file_write rel_path=\"docs/brief.md\" content=…`.\n");
        } else {
            for (i, d) in docs.iter().take(8).enumerate() {
                let hint = if i == 0 { " — what to build and for whom; read first" } else { "" };
                out.push_str(&format!("- `file_read rel_path=\"{d}\"`{hint}\n"));
            }
            if docs.len() > 8 {
                out.push_str(&format!("- … {} more: `file_list glob=\"docs/**\"`\n", docs.len() - 8));
            }
        }
        out.push_str("- AGENTS.md (the project's rules) and MEMORY.md (what earlier agents learned) are embedded below — read both.\n");

        // What exists: counts and the one tool that expands each. Never the list.
        let (active, stale, draft, total) = match self.platform.projects.list_pipeline_meta_rows(owner, project) {
            Ok(ps) => {
                let a = ps.iter().filter(|p| pipeline_status(p) == "active").count();
                let st = ps.iter().filter(|p| pipeline_status(p) == "stale").count();
                (a, st, ps.len() - a - st, ps.len())
            }
            Err(_) => (0, 0, 0, 0),
        };
        let files = tree.as_ref().map(|l| l.items.iter().filter(|i| i.kind == "file").count()).unwrap_or(0);
        let connections: Vec<String> = self
            .platform
            .db_connections
            .list_project_connections(owner, project)
            .map(|items| items.iter().map(|c| format!("`{}` ({})", c.connection_slug, c.database_kind)).collect())
            .unwrap_or_default();
        let credentials: Vec<String> = self
            .platform
            .credentials
            .list_project_credentials(owner, project)
            .map(|items| items.iter().map(|c| format!("{} ({})", c.title, c.kind)).collect())
            .unwrap_or_default();
        if total == 0 && files == 0 {
            out.push_str("- Nothing exists yet: an empty project — `pipeline_register` creates the first route, `file_write` the first page.\n");
        } else {
            out.push_str(&format!(
                "- What exists: {total} pipelines ({active} active, {stale} stale, {draft} draft) → `pipeline_list`, then `pipeline_describe file_rel_path=\"…\" compact=true`; \
                 {files} files → `file_list`, then `file_outline rel_path=\"…\"` before `file_read`.\n"
            ));
        }
        if !connections.is_empty() {
            out.push_str(&format!(
                "- Databases: {} → `connection_describe slug=\"…\" scope=\"tables\"` before any SQL.",
                connections.join(", ")
            ));
            if !credentials.is_empty() {
                out.push_str(&format!(" Credentials: {}.", credentials.join(", ")));
            }
            out.push('\n');
        }
        // A repository with no commits answers "fatal: … does not have any
        // commits yet" on stdout; that is not a commit list.
        let git = self.git_command("log", Some("--oneline -3"), None).await;
        let git_text = git.text.trim();
        if !git_text.is_empty() && !git_text.starts_with("Error") && !git_text.starts_with("fatal:") {
            let last: Vec<&str> = git_text.lines().take(3).collect();
            out.push_str(&format!("- Last commits: {}\n", last.join(" · ")));
        }

        // Zebflow knowledge as a direction table: one row per kind of task,
        // read only when the task is that kind.
        let project_skills: Vec<String> = self
            .platform
            .projects
            .project_layout(owner, project)
            .map(|layout| {
                crate::platform::skills::list(&layout.repo_source_dir())
                    .into_iter()
                    .filter(|s| s.source == crate::platform::skills::SkillSource::Project)
                    .map(|s| format!("`{}` — {}", s.name, s.description))
                    .collect()
            })
            .unwrap_or_default();
        let node_count = official_node_count();
        out.push_str(&format!(
            "\n## Step 2 — Zebflow, only what this task needs\n\
             | If the task is… | open |\n|---|---|\n\
             | any task, first time this session | `skill_read name=\"zebflow-basic\"` |\n\
             | creating a new file or module | `skill_read name=\"zebflow-engineering\"` |\n\
             | a route, an API, a form POST, a job | `help topic=\"pipeline/dsl\"` then `skill_read name=\"zebflow-pipeline\"` |\n\
             | a page or component | `help topic=\"web\"` then `skill_read name=\"zebflow-rwe\"` |\n\
             | a screen with forms, tables, dialogs | `skill_read name=\"zebflow-ui\"` |\n\
             | a query, a table, a migration | `connection_describe` then `skill_read name=\"zebflow-data\"` |\n\
             | sign-in, roles, a protected route | `skill_read name=\"zebflow-auth\"` |\n\
             | uploads, images, rich text | `skill_read name=\"zebflow-files-editor\"` |\n\
             | which node does X ({node_count} kinds, one line each) | `help topic=\"pipeline/nodes\"` |\n\
             | one node's flags | `help topic=\"pipeline/nodes/<short name>\"` |\n\
             | where a file or URL lives, hosts, surfaces | `help topic=\"platform\"` |\n\
             | proving a route or page works | `skill_read name=\"zebflow-verify\"` |\n\
             | adding a Hub package | `skill_read name=\"zebflow-hub\"` |\n\
             | anything else | `help_search query=\"…\"` |\n"
        ));
        if !project_skills.is_empty() {
            out.push_str("\nThis project's own skills override the blessed ones:\n");
            for s in &project_skills {
                out.push_str(&format!("- {s}\n"));
            }
        }

        out.push_str(
            "\n## Step 3 — the loop\n\
             Orient (`pipeline_describe … compact=true`, `file_outline`, `connection_describe`) → \
             Build through the tools only (`pipeline_register` for DSL, `file_write` for TSX, then `pipeline_activate`; never the filesystem, never a direct edit of a live route) → \
             Verify (`route_fetch path=\"…\"`: status, `rwe_component_errors`, body — a 200 with a component error is not done) → \
             Record (`docs_agent_write name=\"MEMORY.md\"` with durable, non-obvious facts; never commit for the user).\n",
        );

        out.push_str("\n---\n\n## AGENTS.md\n");
        match self.platform.projects.read_agent_doc(owner, project, "AGENTS.md") {
            Ok(content) => out.push_str(&content),
            Err(_) => out.push_str("(none — create with `docs_agent_write name=\"AGENTS.md\" content=…`)"),
        }
        if let Ok(soul) = self.platform.projects.read_agent_doc(owner, project, "SOUL.md") {
            if !soul.trim_start().starts_with("# Soul\n\nDescribe the assistant") && soul.len() > 60 {
                out.push_str("\n\n## Personality (SOUL.md)\n");
                out.push_str(&soul);
            }
        }
        out.push_str("\n\n## MEMORY.md\n");
        match self.platform.projects.read_agent_doc(owner, project, "MEMORY.md") {
            Ok(mem) => {
                let is_default = mem.contains("_(This file is managed by the assistant.") && mem.len() < 300;
                if is_default {
                    out.push_str("(empty — write discoveries here: `docs_agent_write name=\"MEMORY.md\" content=…`)");
                } else {
                    out.push_str(&mem);
                }
            }
            Err(_) => out.push_str("(none)"),
        }

        out.push_str(
            "\n\n## Operational rules\n\
             - Before patching a pipeline, `pipeline_describe` and use the returned node ids.\n\
             - After `pipeline_register` or `pipeline_patch`, `pipeline_activate` before expecting traffic.\n\
             - When testing function pipelines, pass an explicit `input` object.\n\
             - After meaningful work, update `MEMORY.md` with durable project facts.\n",
        );

        OpsResult::ok(out)
    }}

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
    pub fn pipeline_list(&self, options: PipelineListOptions<'_>) -> OpsResult {
        match self
            .platform
            .projects
            .list_pipeline_meta_rows(&self.owner, &self.project)
        {
            Ok(pipelines) => self.format_pipeline_list(pipelines, options),
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    fn format_pipeline_list(
        &self,
        pipelines: Vec<PipelineMeta>,
        options: PipelineListOptions<'_>,
    ) -> OpsResult {
        let format = options.format.unwrap_or("compact");
        if format == "json" {
            let filtered = filter_pipeline_rows(self, pipelines, &options, false);
            return OpsResult::ok(
                serde_json::to_string_pretty(
                    &json!({ "pipelines": filtered, "count": filtered.len() }),
                )
                .unwrap_or_default(),
            );
        }

        let mut rows = filter_pipeline_rows(self, pipelines, &options, true);
        let total = rows.len();
        let default_limit = if format == "compact" { 80 } else { usize::MAX };
        let limit = options
            .limit
            .map(|n| n as usize)
            .unwrap_or(default_limit)
            .max(1);
        if rows.len() > limit {
            rows.truncate(limit);
        }

        let mut out = format!("count: {total}");
        if rows.len() < total {
            out.push_str(&format!(" (showing {})", rows.len()));
        }
        out.push('\n');

        match format {
            "compact" => {
                for (meta, trigger_summary) in rows {
                    out.push_str(&format!(
                        "{} | {} | {} | {}\n",
                        meta.file_rel_path,
                        trigger_summary,
                        pipeline_status(&meta),
                        pipeline_purpose(&meta)
                    ));
                }
                OpsResult::ok(out)
            }
            "tree" => {
                for (meta, trigger_summary) in rows {
                    let depth = meta.file_rel_path.matches('/').count();
                    out.push_str(&format!(
                        "{}{} | {} | {} | {}\n",
                        "  ".repeat(depth),
                        meta.file_rel_path,
                        trigger_summary,
                        pipeline_status(&meta),
                        pipeline_purpose(&meta)
                    ));
                }
                OpsResult::ok(out)
            }
            other => OpsResult::err(format!(
                "pipeline_list format must be compact, json, or tree; got '{other}'"
            )),
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
                // Identity is source-relative, so a minted path carries no
                // root segment; the root lives in the project's layout.
                let vpath = raw_path.trim_matches('/');
                if vpath.is_empty() {
                    n.to_string()
                } else {
                    format!("{vpath}/{n}")
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
        // The page that shows what was just registered, with it selected.
        let nav = crate::platform::interaction::patterns::pipeline_editor_url(
            &self.owner,
            &self.project,
            &frp,
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
        target: &str,
        node_id: &str,
        flags: Option<&str>,
        body: Option<&str>,
    ) -> OpsResult {
        let target = if target.trim().is_empty() { "node" } else { target.trim() };
        let mut dsl = format!("patch pipeline {file_rel_path} {target} {node_id}");
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
        let nav = crate::platform::interaction::patterns::pipeline_editor_url(
            &self.owner,
            &self.project,
            file_rel_path,
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

        let layout = match self
            .platform
            .projects
            .project_layout(&self.owner, &self.project)
        {
            Ok(value) => value,
            Err(e) => return OpsResult::err(e.to_string()),
        };
        // Reuse the same glob matcher used by file_search / pipeline_search.
        let matching: Vec<String> = rows
            .iter()
            .filter(|m| {
                crate::platform::services::project::pipeline_glob_matches(
                    &layout.repo_layout,
                    glob,
                    &m.file_rel_path,
                )
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
    pub fn file_list(&self, options: FileListOptions<'_>) -> OpsResult {
        match self
            .platform
            .projects
            .list_repo_tree(&self.owner, &self.project, &RepoTreeScope::all())
        {
            Ok(workspace) => {
                let format = options.format.unwrap_or("compact");
                if format == "json" {
                    if let Some(g) = options.glob.filter(|s| !s.is_empty()) {
                        let filtered_items: Vec<_> = workspace
                            .items
                            .iter()
                            .filter(|item| {
                                item.kind != "folder"
                                    && crate::platform::services::project::template_glob_matches(
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
                        return OpsResult::ok(
                            serde_json::to_string_pretty(&result).unwrap_or_default(),
                        );
                    }
                    return OpsResult::ok(
                        serde_json::to_string_pretty(&workspace).unwrap_or_default(),
                    );
                }

                self.format_file_list(workspace.items, options)
            }
            Err(e) => OpsResult::err(e.to_string()),
        }
    }

    fn format_file_list(
        &self,
        items: Vec<TemplateTreeItem>,
        options: FileListOptions<'_>,
    ) -> OpsResult {
        let format = options.format.unwrap_or("compact");
        let mut rows = filter_template_rows(self, items, &options);
        let total = rows.len();
        let default_limit = if format == "compact" { 120 } else { usize::MAX };
        let limit = options
            .limit
            .map(|n| n as usize)
            .unwrap_or(default_limit)
            .max(1);
        if rows.len() > limit {
            rows.truncate(limit);
        }

        let mut out = format!("count: {total}");
        if rows.len() < total {
            out.push_str(&format!(" (showing {})", rows.len()));
        }
        out.push('\n');

        match format {
            "compact" => {
                for (item, meta) in rows {
                    out.push_str(&format!(
                        "{} | {} | {} | {}\n",
                        item.rel_path, item.file_kind, meta.title, meta.description
                    ));
                }
                OpsResult::ok(out)
            }
            "tree" => {
                for (item, meta) in rows {
                    out.push_str(&format!(
                        "{}{} | {} | {} | {}\n",
                        "  ".repeat(item.depth),
                        item.rel_path,
                        item.file_kind,
                        meta.title,
                        meta.description
                    ));
                }
                OpsResult::ok(out)
            }
            other => OpsResult::err(format!(
                "file_list format must be compact, json, or tree; got '{other}'"
            )),
        }
    }

    pub fn file_read(&self, rel_path: &str, offset: Option<u32>, limit: Option<u32>) -> OpsResult {
        // Resolve content — try exact match first, then fuzzy fallback.
        let (resolved_path, content) =
            match self
                .platform
                .projects
                .read_repo_file_text(&self.owner, &self.project, rel_path)
            {
                Ok(content) => (rel_path.to_string(), content),
                Err(_) => {
                    // Fuzzy fallback
                    let listing = match self
                        .platform
                        .projects
                        .list_repo_tree(&self.owner, &self.project, &RepoTreeScope::all())
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
                        1 => match self.platform.projects.read_repo_file_text(
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

    pub fn file_outline(&self, rel_path: &str) -> OpsResult {
        let content =
            match self
                .platform
                .projects
                .read_repo_file_text(&self.owner, &self.project, rel_path)
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

    pub fn file_deps(&self, rel_path: &str) -> OpsResult {
        let content =
            match self
                .platform
                .projects
                .read_repo_file_text(&self.owner, &self.project, rel_path)
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
            .list_repo_tree(&self.owner, &self.project, &RepoTreeScope::all())
        {
            Ok(w) => w,
            Err(_) => return OpsResult::ok(out),
        };

        let mut importers: Vec<String> = Vec::new();
        for item in &workspace.items {
            if item.kind == "folder" || item.rel_path == rel_path {
                continue;
            }
            let file_content = match self.platform.projects.read_repo_file_text(
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

    pub fn file_batch_edit(&self, edits: &[(String, String, String)]) -> OpsResult {
        if edits.is_empty() {
            return OpsResult::err("edits list must not be empty");
        }
        let mut results: Vec<String> = Vec::new();
        for (i, (rel_path, old_string, new_string)) in edits.iter().enumerate() {
            if old_string.is_empty() {
                results.push(format!("[{}] {} — SKIP: old_string empty", i + 1, rel_path));
                continue;
            }
            match self.platform.projects.edit_repo_file(
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

    /// Creates one file or folder at a path, with a starter body chosen by kind.
    ///
    /// The kind decides the extension and the first few lines; it does not
    /// decide *where* the file lands. The road this replaces sent a page to
    /// `{source}/pages` whatever folder the caller named.
    pub fn file_create(&self, kind: &str, name: &str, parent_rel_path: Option<&str>) -> OpsResult {
        let parent = parent_rel_path.unwrap_or("").trim_matches('/');
        let join = |leaf: &str| {
            if parent.is_empty() {
                leaf.to_string()
            } else {
                format!("{parent}/{leaf}")
            }
        };

        if kind == "folder" {
            return match self.platform.projects.create_repo_folder(
                &self.owner,
                &self.project,
                &join(name),
            ) {
                Ok(rel) => OpsResult::ok(format!("Created folder {rel}")),
                Err(e) => OpsResult::err(e.to_string()),
            };
        }

        let (extension, body) = match kind {
            "page" | "component" => (
                "tsx",
                format!("export default function {name}() {{\n  return <div />;\n}}\n"),
            ),
            "script" => ("ts", "export {};\n".to_string()),
            "doc" => ("md", format!("# {name}\n")),
            "style" => ("css", String::new()),
            other => {
                return OpsResult::err(format!(
                    "Invalid kind '{other}'. Must be: page, component, script, style, doc, folder"
                ));
            }
        };
        let leaf = if name.ends_with(&format!(".{extension}")) {
            name.to_string()
        } else {
            format!("{name}.{extension}")
        };
        match self.platform.projects.write_repo_file(
            &self.owner,
            &self.project,
            &join(&leaf),
            &body,
        ) {
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

    pub fn file_write(&self, rel_path: &str, content: &str) -> OpsResult {
        let req = TemplateSaveRequest {
            rel_path: rel_path.to_string(),
            content: content.to_string(),
        };
        match self.platform.projects.write_repo_file(
            &self.owner,
            &self.project,
            &req.rel_path,
            &req.content,
        ) {
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

    pub fn file_search(
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
        match self
            .platform
            .projects
            .search_repo_files(&self.owner, &self.project, pattern, context)
        {
            Err(e) => OpsResult::err(e.to_string()),
            Ok(matches) if matches.is_empty() => OpsResult::ok(format!(
                "No matches for '{}' in the repository{}.",
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

    pub fn file_edit(&self, rel_path: &str, old_string: &str, new_string: &str) -> OpsResult {
        if old_string.is_empty() {
            return OpsResult::err("old_string must not be empty");
        }
        match self.platform.projects.edit_repo_file(
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

        let layout = match self
            .platform
            .file
            .ensure_project_layout(&self.owner, &self.project)
        {
            Err(e) => return OpsResult::err(e.to_string()),
            Ok(l) => l,
        };
        let from_is_pipeline = pipeline_path_heuristic(&layout.repo_layout, from_path);
        let to_is_pipeline = pipeline_path_heuristic(&layout.repo_layout, to_path);

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
        let owner = &self.owner;
        let project = &self.project;
        let projects = &self.platform.projects;
        let runtime = &self.platform.pipeline_runtime;

        let from = match projects.pipeline_identity(owner, project, from_path) {
            Ok(value) => value,
            Err(error) => return OpsResult::err(error.to_string()),
        };
        let to = match projects.pipeline_identity(owner, project, to_path) {
            Ok(value) => value,
            Err(error) => return OpsResult::err(error.to_string()),
        };

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

        // Rename the pipeline inside its own source before re-registering it.
        let new_source = match rename_pipeline_in_source(&source, &to) {
            Ok(s) => s,
            Err(e) => return OpsResult::err(format!("Failed to patch pipeline id: {e}")),
        };

        // Withdraw the old registration before writing the new one. The moved
        // pipeline keeps its own webhook triggers, and
        // `check_webhook_path_conflict` exempts only the file being written, so
        // while both paths are registered the pipeline collides with itself.
        // Registering first and deleting after made every rename of a webhook
        // pipeline fail on `PLATFORM_PIPELINE_WEBHOOK_CONFLICT`.
        if let Err(e) = projects.delete_pipeline(owner, project, &from) {
            return OpsResult::err(format!("Failed to delete old pipeline: {e}"));
        }
        // The DB row is gone, so `refresh_pipeline` on the old path would fail.
        runtime.evict(owner, project, &from);

        // Register at the new path (creates file + DB entry, not yet active).
        // Nothing holds the pipeline at this point, so a refusal here has to put
        // the original back rather than leave the project without it.
        if let Err(e) = projects.upsert_pipeline_definition(
            owner,
            project,
            &to,
            &meta.title,
            &meta.description,
            &meta.trigger_kind,
            &new_source,
        ) {
            let restored = projects.upsert_pipeline_definition(
                owner,
                project,
                &from,
                &meta.title,
                &meta.description,
                &meta.trigger_kind,
                &source,
            );
            if restored.is_ok() && was_active {
                let _ = projects.activate_pipeline_definition(owner, project, &from);
                let _ = runtime.refresh_pipeline(owner, project, &from);
            }
            return OpsResult::err(match restored {
                Ok(_) => format!("Failed to register at new path: {e}"),
                Err(restore_error) => format!(
                    "Failed to register at new path: {e}. \
                     The original at '{from}' could not be restored either: {restore_error}"
                ),
            });
        }

        // Re-activate at new path if was active before
        if was_active {
            if let Err(e) = projects.activate_pipeline_definition(owner, project, &to) {
                return OpsResult::err(format!("Failed to activate at new path: {e}"));
            }
            let _ = runtime.refresh_pipeline(owner, project, &to);
        }

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

        let root = &layout.repo_source_dir();
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
///
/// Deliberately wider than the layout's own pipeline test: a caller naming a
/// destination inside the source root but without the extension is still
/// asking to move a pipeline, and the two halves must both be satisfiable for
/// the move to be refused as cross-domain.
fn pipeline_path_heuristic(layout: &ResolvedProjectLayout, path: &str) -> bool {
    path.ends_with(PIPELINE_DEFINITION_EXTENSION) || layout.is_in_source(path)
}

/// Parses JSON source, sets the `"id"` field to `new_file_rel_path`, returns pretty-printed JSON.
/// Points a pipeline's own source at its new path.
///
/// A pipeline source is a contract envelope, so the identifier lives at
/// `spec.id` and not at the root. Two things follow, and both were wrong when
/// this wrote a root `"id"`: the envelope refuses unknown root fields, so every
/// rename failed to re-register; and `spec.id` is an identifier
/// (`validate_identifier`, `pipeline.rs`), so it takes the pipeline's name and
/// never the `pipelines/…/x.zf.json` path.
///
/// `metadata.name` moves with it. The contract requires the two to agree, and
/// the source is decoded before it is re-encoded, so leaving the metadata on
/// the old name only trades one refusal for another.
fn rename_pipeline_in_source(source: &str, new_file_rel_path: &str) -> Result<String, String> {
    let name = name_from_file_rel_path(new_file_rel_path);
    let mut envelope: serde_json::Value =
        serde_json::from_str(source).map_err(|e| format!("invalid JSON: {e}"))?;

    let spec = envelope
        .get_mut("spec")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| "pipeline source has no spec object".to_string())?;
    spec.insert("id".to_string(), serde_json::Value::String(name.clone()));

    let metadata = envelope
        .get_mut("metadata")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| "pipeline source has no metadata object".to_string())?;
    metadata.insert("name".to_string(), serde_json::Value::String(name));

    serde_json::to_string_pretty(&envelope).map_err(|e| format!("serialize error: {e}"))
}

#[cfg(test)]
mod rename_tests {
    use super::*;

    fn source(name: &str) -> String {
        serde_json::json!({
            "apiVersion": "zebflow.com/v1",
            "kind": "Pipeline",
            "metadata": { "name": name },
            "spec": {
                "id": name,
                "entry_nodes": ["n0"],
                "nodes": [{
                    "id": "n0",
                    "kind": "n.trigger.webhook",
                    "input_pins": [],
                    "output_pins": ["out"],
                    "config": { "path": "/hook", "method": "POST" }
                }],
                "edges": []
            }
        })
        .to_string()
    }

    /// The renamed source has to survive the same decode the writer performs.
    /// Before this, the rename wrote a root `"id"` and every move died on
    /// `unknown root field 'id'`.
    #[test]
    fn renamed_source_still_decodes_as_a_pipeline() {
        let renamed =
            rename_pipeline_in_source(&source("before"), "pipelines/team/after.zf.json").unwrap();

        let document = decode_pipeline_graph(renamed.as_bytes()).expect("renamed source decodes");
        assert_eq!(document.spec.id, "after");

        let value: serde_json::Value = serde_json::from_str(&renamed).unwrap();
        assert_eq!(value["metadata"]["name"], "after");
        // The identifier is the name, never the path it was addressed by.
        assert!(value.get("id").is_none());
    }

    #[test]
    fn rename_refuses_a_source_that_is_not_an_envelope() {
        let err = rename_pipeline_in_source(r#"{"id":"bare","nodes":[]}"#, "pipelines/x.zf.json")
            .unwrap_err();
        assert!(err.contains("no spec object"), "{err}");
    }
}

// ── Project Docs ──────────────────────────────────────────────────────────────

impl PlatformOps {}

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

// ── Theme ─────────────────────────────────────────────────────────────────────

impl PlatformOps {
    /// A complete, contrast-checked theme from a seed and a mood — the
    /// `brand-system` skill's engine (`crate::platform::theme::generate`).
    /// The answer is JSON: tokens for both modes, the CSS blocks to paste,
    /// the mood's type and geometry, the contrast table, and notes.
    pub fn theme_generate(&self, seed: Option<String>, mood: Option<String>) -> OpsResult {
        use crate::platform::theme::generate::{generate, Mood};
        let mood_raw = mood.unwrap_or_else(|| "calm".to_string());
        let Some(mood) = Mood::parse(&mood_raw) else {
            return OpsResult::err(format!(
                "unknown mood `{mood_raw}`; one of {}",
                Mood::ALL.iter().map(|m| m.key()).collect::<Vec<_>>().join(", ")
            ));
        };
        let seed = seed
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| mood.default_seed().to_string());
        match generate(&seed, mood) {
            Ok(mut theme) => {
                // A project that holds the pairing under static/fonts/ gets
                // @font-face rules for its own files instead of the link.
                if let Ok(files) = self.platform.fonts.project_font_files(&self.owner, &self.project) {
                    theme.fonts_css = crate::platform::theme::generate::font_face_css(&theme.style, &files, &self.owner, &self.project);
                    if theme.fonts_css.is_some() {
                        theme.notes.push("the display and text families are in static/fonts/: paste `fonts_css` and skip the Google Fonts link".to_string());
                    }
                }
                OpsResult::ok(serde_json::to_string_pretty(&theme).unwrap_or_default())
            }
            Err(message) => OpsResult::err(message),
        }
    }
}

// ── Route fetch ───────────────────────────────────────────────────────────────

/// What `route_fetch` answers with: enough to judge a route without a browser.
#[derive(Debug, Serialize)]
pub struct RouteFetchReport {
    pub url: String,
    pub status: u16,
    pub content_type: String,
    pub location: String,
    pub set_cookie: Vec<String>,
    pub length: usize,
    pub rwe_component_errors: Vec<String>,
    /// What search and answer engines will read, as numbers and booleans —
    /// present for HTML responses (`docs/contracts/discoverability.md` §4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seo: Option<Value>,
    /// Whether a phone would offer to install this page: the manifest it links,
    /// the worker that manifest names, and Chrome's checklist as booleans —
    /// present when the page links a manifest (`discoverability.md` §6).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pwa: Option<Value>,
    pub body: String,
    pub truncated: bool,
}

/// The manifest `href` a page links, if any.
pub fn manifest_href(html: &str) -> Option<String> {
    let re = regex::Regex::new(r#"(?is)<link\s+[^>]*rel=["']manifest["'][^>]*href=["']([^"']*)["']"#).unwrap();
    re.captures(html).and_then(|c| c.get(1)).map(|m| m.as_str().trim().to_string()).filter(|h| !h.is_empty())
}

/// Chrome's installability checklist read off a manifest document, without a
/// browser. `worker` is what fetching the manifest's `serviceworker.src` gave
/// back: `(status, content_type, service_worker_allowed)`. Pure, so a test can
/// hand it a manifest and see every reason.
pub fn manifest_facts(manifest: &Value, worker: Option<(u16, String, String)>) -> Value {
    let s = |k: &str| manifest.get(k).and_then(Value::as_str).map(str::to_string);
    let name = s("name").filter(|v| !v.trim().is_empty());
    let short_name = s("short_name").filter(|v| !v.trim().is_empty());
    let start_url = s("start_url");
    let scope = s("scope").unwrap_or_else(|| "/".to_string());
    let display = s("display").unwrap_or_else(|| "browser".to_string());
    let icons: Vec<Value> = manifest.get("icons").and_then(Value::as_array).cloned().unwrap_or_default();
    let icon_px = |icon: &Value| -> u32 {
        icon.get("sizes").and_then(Value::as_str).unwrap_or("").split('x').next().and_then(|w| w.trim().parse().ok()).unwrap_or(0)
    };
    let has_192 = icons.iter().any(|i| icon_px(i) >= 192);
    let has_512 = icons.iter().any(|i| icon_px(i) >= 512);
    let has_maskable = icons.iter().any(|i| i.get("purpose").and_then(Value::as_str).is_some_and(|p| p.contains("maskable")));
    let start_in_scope = start_url.as_deref().is_some_and(|u| u.starts_with(scope.as_str()));
    let display_ok = matches!(display.as_str(), "standalone" | "fullscreen" | "minimal-ui");

    // The manifest may name its worker (`serviceworker.src`, the old draft
    // member); otherwise the convention is `sw.js` at the scope root.
    let (worker_src, assumed) = match manifest.get("serviceworker").and_then(|w| w.get("src")).and_then(Value::as_str) {
        Some(src) => (Some(src.to_string()), false),
        None => (Some(format!("{}sw.js", if scope.ends_with('/') { scope.clone() } else { format!("{scope}/") })), true),
    };
    let worker_report = match (&worker_src, &worker) {
        (Some(src), Some((status, content_type, allowed))) => {
            let is_js = content_type.contains("javascript") || content_type.contains("ecmascript");
            // The worker controls its own directory, widened by Service-Worker-Allowed.
            let own_dir = match src.rfind('/') { Some(i) => src[..=i].to_string(), None => "/".to_string() };
            let effective = if !allowed.is_empty() { allowed.clone() } else { own_dir };
            let covers = start_url.as_deref().is_some_and(|u| u.starts_with(effective.as_str()));
            json!({ "src": src, "assumed": assumed, "status": status, "content_type": content_type, "service_worker_allowed": allowed, "effective_scope": effective, "is_javascript": is_js, "covers_start_url": covers, "ok": *status == 200 && is_js && covers })
        }
        (Some(src), None) => json!({ "src": src, "assumed": assumed, "ok": false, "error": "worker could not be fetched" }),
        (None, _) => json!({ "src": null, "ok": false }),
    };
    let worker_ok = worker_report.get("ok").and_then(Value::as_bool).unwrap_or(false);

    let mut reasons = Vec::new();
    if name.is_none() && short_name.is_none() { reasons.push("manifest has neither name nor short_name"); }
    if start_url.is_none() { reasons.push("manifest has no start_url"); }
    else if !start_in_scope { reasons.push("start_url is outside scope"); }
    if !display_ok { reasons.push("display must be standalone, fullscreen or minimal-ui"); }
    if !has_192 { reasons.push("no icon of at least 192×192"); }
    if !has_512 { reasons.push("no icon of at least 512×512"); }
    if !worker_ok { reasons.push("no service worker controlling start_url"); }

    json!({
        "name": name, "short_name": short_name, "id": s("id"), "start_url": start_url, "scope": scope, "display": display,
        "theme_color": s("theme_color"), "background_color": s("background_color"),
        "icons": { "count": icons.len(), "has_192": has_192, "has_512": has_512, "has_maskable": has_maskable },
        "worker": worker_report,
        "installable": reasons.is_empty(),
        "reasons": reasons,
    })
}

/// Read the machine-facing facts off a rendered page. This is deliberately a
/// scan, not a parse: the builder compares `h1_count == 1` and
/// `og.absolute == true` against the skills' rules instead of judging HTML.
pub fn seo_facts(html: &str) -> Value {
    use regex::Regex;
    fn first<'a>(re: &Regex, html: &'a str) -> Option<String> {
        re.captures(html).and_then(|c| c.get(1)).map(|m| m.as_str().trim().to_string())
    }
    let title_re = Regex::new(r"(?is)<title[^>]*>(.*?)</title>").unwrap();
    let meta_name = |name: &str| Regex::new(&format!(r#"(?is)<meta\s+[^>]*name=["']{name}["'][^>]*content=["']([^"']*)["']"#)).unwrap();
    let meta_prop = |prop: &str| Regex::new(&format!(r#"(?is)<meta\s+[^>]*property=["']{prop}["'][^>]*content=["']([^"']*)["']"#)).unwrap();
    let canonical_re = Regex::new(r#"(?is)<link\s+[^>]*rel=["']canonical["'][^>]*href=["']([^"']*)["']"#).unwrap();
    let hreflang_re = Regex::new(r#"(?is)<link\s+[^>]*hreflang=["']([^"']*)["']"#).unwrap();
    let h1_re = Regex::new(r"(?is)<h1[\s>]").unwrap();
    let lang_re = Regex::new(r#"(?is)<html[^>]*\slang=["']([^"']*)["']"#).unwrap();
    let ld_re = Regex::new(r#"(?is)<script[^>]*type=["']application/ld\+json["'][^>]*>(.*?)</script>"#).unwrap();
    let img_re = Regex::new(r"(?is)<img\b[^>]*>").unwrap();
    let alt_re = Regex::new(r"(?is)\salt\s*=").unwrap();

    let og_image = first(&meta_prop("og:image"), html);
    let absolute = |u: &Option<String>| u.as_deref().map(|u| u.starts_with("http://") || u.starts_with("https://")).unwrap_or(false);
    let mut jsonld_types: Vec<String> = Vec::new();
    for cap in ld_re.captures_iter(html) {
        let raw = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let text = raw.replace("\\u003c", "<").replace("\\u003e", ">").replace("\\u0026", "&");
        if let Ok(value) = serde_json::from_str::<Value>(&text) {
            fn collect(v: &Value, out: &mut Vec<String>) {
                match v {
                    Value::Array(items) => items.iter().for_each(|i| collect(i, out)),
                    Value::Object(map) => {
                        if let Some(t) = map.get("@type") {
                            match t {
                                Value::String(s) => out.push(s.clone()),
                                Value::Array(ts) => ts.iter().filter_map(|x| x.as_str()).for_each(|s| out.push(s.to_string())),
                                _ => {}
                            }
                        }
                        if let Some(graph) = map.get("@graph") {
                            collect(graph, out);
                        }
                    }
                    _ => {}
                }
            }
            collect(&value, &mut jsonld_types);
        } else {
            jsonld_types.push("INVALID_JSON".to_string());
        }
    }
    let description = first(&meta_name("description"), html);
    let canonical = first(&canonical_re, html);
    serde_json::json!({
        "title": first(&title_re, html),
        "description_chars": description.as_ref().map(|d| d.chars().count()).unwrap_or(0),
        "canonical": canonical,
        "canonical_absolute": absolute(&canonical),
        "h1_count": h1_re.find_iter(html).count(),
        "lang": first(&lang_re, html),
        "robots": first(&meta_name("robots"), html),
        "og": {
            "title": first(&meta_prop("og:title"), html).is_some(),
            "description": first(&meta_prop("og:description"), html).is_some(),
            "image": og_image,
            "absolute": absolute(&og_image),
            "url": first(&meta_prop("og:url"), html),
            "type": first(&meta_prop("og:type"), html),
        },
        "twitter_card": first(&meta_name("twitter:card"), html),
        "jsonld_types": jsonld_types,
        "hreflang": hreflang_re.captures_iter(html).filter_map(|c| c.get(1)).map(|m| m.as_str().to_string()).collect::<Vec<_>>(),
        "images_without_alt": img_re.find_iter(html).filter(|m| !alt_re.is_match(m.as_str())).count(),
    })
}

impl PlatformOps {
    /// Fetch one of this project's own routes over loopback, the way a browser
    /// or a curl would: the request goes through the real webhook ingress, so
    /// auth, cookies, redirects and rendering all happen. The report carries
    /// the status, the headers a verifier reads, the body (capped) and every
    /// `<!-- RWE component error: … -->` found in it — the one line a 200 hides.
    ///
    /// This exists because an agent that only has MCP had no way to fetch
    /// what it built; every rung of the model ladder stopped at "cannot verify".
    pub async fn route_fetch(
        &self,
        path: String,
        method: Option<String>,
        body: Option<Value>,
        form: Option<serde_json::Map<String, Value>>,
        headers: Option<serde_json::Map<String, Value>>,
        cookie: Option<String>,
        follow_redirects: Option<bool>,
        max_body_chars: Option<usize>,
    ) -> OpsResult {
        let path = path.trim();
        if path.starts_with("http://") || path.starts_with("https://") {
            return OpsResult::err("route_fetch takes a project path such as /book or /api/slots?date=…, not a full URL — it only reaches this project's own routes");
        }
        let path = if path.starts_with('/') { path.to_string() } else { format!("/{path}") };
        // The request goes to loopback with the project's dev host as `Host`,
        // so it is routed exactly as a browser on that host would be — root-
        // relative links and `Location: /admin` included (`addressing.md` §4).
        // With the port, as a browser on this machine would send it, so the
        // absolute URLs the page emits are the ones a person can click.
        let dev_host = format!(
            "{}:{}",
            crate::platform::services::addressing::AddressingService::dev_host(&self.owner, &self.project),
            crate::platform::boot::configured_port()
        );
        let url = format!("{}{}", crate::platform::boot::local_instance_url(), path);
        let method = method.unwrap_or_else(|| "GET".to_string()).to_ascii_uppercase();
        let method = match reqwest::Method::from_bytes(method.as_bytes()) {
            Ok(m) => m,
            Err(_) => return OpsResult::err(format!("unknown method {method}")),
        };
        let policy = if follow_redirects.unwrap_or(false) {
            reqwest::redirect::Policy::limited(5)
        } else {
            reqwest::redirect::Policy::none()
        };
        let client = match reqwest::Client::builder()
            .redirect(policy)
            .timeout(std::time::Duration::from_secs(60))
            .build()
        {
            Ok(c) => c,
            Err(e) => return OpsResult::err(e.to_string()),
        };
        let mut request = client.request(method, &url).header(reqwest::header::HOST, dev_host.clone());
        if let Some(headers) = headers {
            for (k, v) in headers {
                let value = match v {
                    Value::String(s) => s,
                    other => other.to_string(),
                };
                request = request.header(k, value);
            }
        }
        if let Some(cookie) = cookie.filter(|c| !c.trim().is_empty()) {
            request = request.header(reqwest::header::COOKIE, cookie.trim().to_string());
        }
        if let Some(form) = form {
            let pairs: Vec<(String, String)> = form
                .into_iter()
                .map(|(k, v)| (k, match v { Value::String(s) => s, other => other.to_string() }))
                .collect();
            request = request.form(&pairs);
        } else if let Some(body) = body {
            request = match body {
                Value::String(s) => request.body(s),
                other => request.json(&other),
            };
        }
        let response = match request.send().await {
            Ok(r) => r,
            Err(e) => return OpsResult::err(format!("fetch failed: {e}")),
        };
        let status = response.status().as_u16();
        let header = |name: reqwest::header::HeaderName| {
            response
                .headers()
                .get(&name)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string()
        };
        let content_type = header(reqwest::header::CONTENT_TYPE);
        let location = header(reqwest::header::LOCATION);
        let set_cookie: Vec<String> = response
            .headers()
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(str::to_string))
            .collect();
        let text = response.text().await.unwrap_or_default();
        let length = text.len();
        let rwe_component_errors: Vec<String> = text
            .match_indices("<!-- RWE component error:")
            .map(|(i, _)| {
                let rest = &text[i..];
                let end = rest.find("-->").unwrap_or(rest.len().min(300));
                rest[..end].trim().to_string()
            })
            .collect();
        let seo = if content_type.starts_with("text/html") { Some(seo_facts(&text)) } else { None };
        // A page that links a manifest is asking to be installable; fetch the
        // manifest and the worker it names on the same host and report Chrome's
        // checklist, so an agent sees "installable: false, reasons: […]" instead
        // of a phone that silently never offers the button.
        let pwa = match manifest_href(&text) {
            Some(href) if content_type.starts_with("text/html") => {
                let same_host = |p: &str| format!("{}{}", crate::platform::boot::local_instance_url(), if p.starts_with('/') { p.to_string() } else { format!("/{p}") });
                let manifest_resp = client.get(same_host(&href)).header(reqwest::header::HOST, dev_host.clone()).send().await;
                match manifest_resp {
                    Ok(r) => {
                        let m_status = r.status().as_u16();
                        let m_type = r.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
                        let m_text = r.text().await.unwrap_or_default();
                        match serde_json::from_str::<Value>(&m_text) {
                            Ok(manifest) if m_status == 200 => {
                                let scope = manifest.get("scope").and_then(Value::as_str).unwrap_or("/");
                                let worker_src = manifest.get("serviceworker").and_then(|w| w.get("src")).and_then(Value::as_str).map(str::to_string)
                                    .unwrap_or_else(|| format!("{}sw.js", if scope.ends_with('/') { scope.to_string() } else { format!("{scope}/") }));
                                let worker = match client.get(same_host(&worker_src)).header(reqwest::header::HOST, dev_host.clone()).send().await {
                                    Ok(w) => {
                                        let h = |n: &str| w.headers().get(n).and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
                                        Some((w.status().as_u16(), h("content-type"), h("service-worker-allowed")))
                                    }
                                    Err(_) => None,
                                };
                                let mut facts = manifest_facts(&manifest, worker);
                                facts["href"] = json!(href);
                                facts["content_type"] = json!(m_type);
                                Some(facts)
                            }
                            _ => Some(json!({ "href": href, "status": m_status, "content_type": m_type, "installable": false, "reasons": ["manifest did not answer 200 with JSON"] })),
                        }
                    }
                    Err(e) => Some(json!({ "href": href, "installable": false, "reasons": [format!("manifest fetch failed: {e}")] })),
                }
            }
            _ => None,
        };
        let cap = max_body_chars.unwrap_or(6000).clamp(200, 60_000);
        let (body, truncated) = if text.chars().count() > cap {
            (text.chars().take(cap).collect::<String>(), true)
        } else {
            (text, false)
        };
        let report = RouteFetchReport {
            url: format!("http://{dev_host}{path}"),
            status,
            content_type,
            location,
            set_cookie,
            length,
            rwe_component_errors,
            seo,
            pwa,
            body,
            truncated,
        };
        OpsResult::ok(serde_json::to_string_pretty(&report).unwrap_or_default())
    }
}

#[cfg(test)]
mod pwa_facts_tests {
    use super::{manifest_facts, manifest_href};
    use serde_json::json;

    #[test]
    fn a_complete_manifest_with_a_root_worker_is_installable() {
        let m = json!({ "name": "RESEARCHSITE", "start_url": "/", "scope": "/", "display": "standalone",
            "icons": [{ "src": "/_files/pwa/icon-192.png", "sizes": "192x192" }, { "src": "/_files/pwa/icon-512.png", "sizes": "512x512", "purpose": "maskable" }],
            "serviceworker": { "src": "/sw.js", "scope": "/" } });
        let f = manifest_facts(&m, Some((200, "text/javascript; charset=utf-8".into(), "/".into())));
        assert_eq!(f["installable"], true, "{f}");
        assert_eq!(f["icons"]["has_maskable"], true);
        assert_eq!(f["worker"]["effective_scope"], "/");
    }

    #[test]
    fn a_worker_served_under_files_controls_nothing_unless_the_header_widens_it() {
        let m = json!({ "name": "X", "start_url": "/", "display": "standalone",
            "icons": [{ "sizes": "192x192" }, { "sizes": "512x512" }], "serviceworker": { "src": "/_files/sw.js" } });
        let narrow = manifest_facts(&m, Some((200, "text/javascript".into(), String::new())));
        assert_eq!(narrow["installable"], false);
        assert_eq!(narrow["worker"]["effective_scope"], "/_files/");
        assert!(narrow["reasons"].as_array().unwrap().iter().any(|r| r.as_str().unwrap().contains("service worker")));
        let widened = manifest_facts(&m, Some((200, "text/javascript".into(), "/".into())));
        assert_eq!(widened["installable"], true);
    }

    #[test]
    fn a_manifest_that_names_no_worker_is_checked_at_sw_js_under_its_scope() {
        let mut m = json!({ "name": "My RESEARCHSITE", "start_url": "/member/", "scope": "/member/", "display": "standalone",
            "icons": [{ "sizes": "192x192" }, { "sizes": "512x512" }] });
        let f = manifest_facts(&m, Some((200, "text/javascript".into(), String::new())));
        assert_eq!(f["worker"]["src"], "/member/sw.js");
        assert_eq!(f["worker"]["assumed"], true);
        assert_eq!(f["installable"], true, "{f}");
        // `/member` without the slash is outside `/member/` — the browser's rule, and the
        // one mistake every second app makes, so the report has to say it.
        m["start_url"] = json!("/member");
        let f = manifest_facts(&m, Some((200, "text/javascript".into(), String::new())));
        assert_eq!(f["worker"]["covers_start_url"], false);
        assert_eq!(f["installable"], false);
    }

    #[test]
    fn every_missing_piece_is_a_named_reason() {
        let f = manifest_facts(&json!({ "start_url": "/app", "scope": "/member", "display": "browser", "icons": [] }), None);
        let reasons: Vec<&str> = f["reasons"].as_array().unwrap().iter().map(|r| r.as_str().unwrap()).collect();
        assert!(reasons.iter().any(|r| r.contains("name")));
        assert!(reasons.iter().any(|r| r.contains("outside scope")));
        assert!(reasons.iter().any(|r| r.contains("display")));
        assert!(reasons.iter().any(|r| r.contains("192")));
        assert!(reasons.iter().any(|r| r.contains("512")));
        assert!(reasons.iter().any(|r| r.contains("service worker")));
    }

    #[test]
    fn the_manifest_link_is_read_off_the_page() {
        assert_eq!(manifest_href(r#"<head><link rel="manifest" href="/manifest.webmanifest"><title>x</title></head>"#).as_deref(), Some("/manifest.webmanifest"));
        assert_eq!(manifest_href("<head><title>x</title></head>"), None);
    }
}

#[cfg(test)]
mod seo_facts_tests {
    use super::seo_facts;

    #[test]
    fn a_page_is_read_as_numbers_and_booleans() {
        let html = r#"<!DOCTYPE html><html lang="en"><head><title>Jane Doe — RESEARCHSITE</title>
            <meta name="description" content="Maternal health in eastern Indonesia.">
            <link rel="canonical" href="https://research.example/researchers/jane-doe">
            <meta property="og:title" content="Jane Doe"><meta property="og:description" content="x">
            <meta property="og:image" content="/files/photos/jane.webp"><meta name="twitter:card" content="summary_large_image">
            <link rel="alternate" hreflang="en" href="https://research.example/r"><link rel="alternate" hreflang="id" href="https://research.example/id/r">
            <script type="application/ld+json">{"@context":"https://schema.org","@type":"Person","name":"Jane \u003c/script\u003e Doe"}</script>
            <script type="application/ld+json">[{"@type":"BreadcrumbList"},{"@graph":[{"@type":"WebSite"}]}]</script>
            </head><body><h1>Jane</h1><h1>Twice</h1><img src="a.png"><img src="b.png" alt=""></body></html>"#;
        let facts = seo_facts(html);
        assert_eq!(facts["title"], "Jane Doe — RESEARCHSITE");
        assert_eq!(facts["description_chars"], 37);
        assert_eq!(facts["canonical_absolute"], true);
        assert_eq!(facts["h1_count"], 2);
        assert_eq!(facts["og"]["absolute"], false, "a relative og:image is the finding");
        assert_eq!(facts["twitter_card"], "summary_large_image");
        assert_eq!(facts["jsonld_types"], serde_json::json!(["Person", "BreadcrumbList", "WebSite"]));
        assert_eq!(facts["hreflang"], serde_json::json!(["en", "id"]));
        assert_eq!(facts["images_without_alt"], 1);
        assert_eq!(facts["lang"], "en");
    }
}

// ── Hub ───────────────────────────────────────────────────────────────────────

impl PlatformOps {
    /// The shelf, as an agent reads it: one line per package, filtered by a
    /// word in the id, title, description or tags, and by kind. This is what
    /// `hub_add` can install; remote repositories are the Studio's business.
    pub fn hub_search(&self, query: Option<String>, kind: Option<String>) -> OpsResult {
        let needle = query.unwrap_or_default().trim().to_lowercase();
        let kind = kind.unwrap_or_default().trim().to_lowercase();
        let packages = match self.platform.hub.installable_packages(&self.owner) {
            Ok(items) => items,
            Err(err) => return OpsResult::err(err.to_string()),
        };
        let rows: Vec<Value> = packages
            .into_iter()
            .filter(|(package, _)| kind.is_empty() || package.asset_kind == kind)
            .filter(|(package, _)| {
                needle.is_empty()
                    || package.package_id.to_lowercase().contains(&needle)
                    || package.title.to_lowercase().contains(&needle)
                    || package.description.to_lowercase().contains(&needle)
                    || package.tags.iter().any(|tag| tag.to_lowercase().contains(&needle))
            })
            .map(|(package, latest_version)| {
                json!({
                    "package_id": package.package_id,
                    "asset_kind": package.asset_kind,
                    "title": package.title,
                    "description": package.description,
                    "tags": package.tags,
                    "latest_version": latest_version,
                    "publisher": package.publisher_display_name,
                })
            })
            .collect();
        if rows.is_empty() {
            return OpsResult::ok(
                "No package matches. `hub_search` with no arguments lists everything this project can add.",
            );
        }
        OpsResult::ok(serde_json::to_string_pretty(&rows).unwrap_or_default())
    }

    fn hub_version_or_latest(&self, package_id: &str, version: Option<String>) -> Result<String, String> {
        if let Some(version) = version.map(|v| v.trim().to_string()).filter(|v| !v.is_empty()) {
            return Ok(version);
        }
        match self.platform.hub.latest_live_asset_version(package_id) {
            Ok(Some(version)) => Ok(version),
            Ok(None) => Err(format!(
                "`{package_id}` has no installable release on this shelf; `hub_search` lists what does"
            )),
            Err(err) => Err(err.to_string()),
        }
    }

    /// What an add would do, before it does it: files added and overwritten,
    /// pipelines registered, nodes, credentials, network and public endpoints.
    pub fn hub_review(&self, package_id: String, version: Option<String>, target_folder: Option<String>) -> OpsResult {
        let version = match self.hub_version_or_latest(&package_id, version) {
            Ok(version) => version,
            Err(message) => return OpsResult::err(message),
        };
        let target_folder = target_folder.unwrap_or_default().trim().to_string();
        match self.platform.hub.review_asset_install(
            &self.owner,
            &self.project,
            &package_id,
            &version,
            &target_folder,
        ) {
            Ok(review) => OpsResult::ok(serde_json::to_string_pretty(&review).unwrap_or_default()),
            Err(err) => OpsResult::err(err.to_string()),
        }
    }

    /// Add a shelf package to this project — the same door as the Studio's
    /// Add button. Where it lands depends on the kind: a skill at
    /// `skills/<name>/`, a library under `shared/`, a bundle under
    /// `target_folder`. The result says what was written and registered.
    pub fn hub_add(&self, package_id: String, version: Option<String>, target_folder: Option<String>) -> OpsResult {
        let version = match self.hub_version_or_latest(&package_id, version) {
            Ok(version) => version,
            Err(message) => return OpsResult::err(message),
        };
        let target_folder = target_folder.unwrap_or_default().trim().to_string();
        match self.platform.hub.install_asset(
            &self.owner,
            &self.project,
            &package_id,
            &version,
            &target_folder,
        ) {
            Ok(result) => OpsResult::ok(serde_json::to_string_pretty(&result).unwrap_or_default()),
            Err(err) => OpsResult::err(err.to_string()),
        }
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
                let shared_ui_dir = layout.repo_source_dir().join("shared").join("ui");
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
                let shared_ui_dir = layout.repo_source_dir().join("shared").join("ui");
                match crate::platform::catalog::CatalogService::install_ui_reviewed(
                    &layout.repo_layout,
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

fn filter_pipeline_rows(
    ops: &PlatformOps,
    pipelines: Vec<PipelineMeta>,
    options: &PipelineListOptions<'_>,
    include_trigger_summary: bool,
) -> Vec<(PipelineMeta, String)> {
    let query = options
        .query
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase);
    let status_filter = options
        .status
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("all"))
        .map(str::to_lowercase);
    let trigger_filter = options
        .trigger_kind
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("all"))
        .map(normalize_trigger_filter);
    let glob = options
        .glob
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|glob| {
            ops.platform
                .projects
                .project_layout(&ops.owner, &ops.project)
                .ok()
                .map(|layout| {
                    crate::platform::services::project::normalize_pipeline_glob(
                        &layout.repo_layout,
                        glob,
                    )
                })
        });

    pipelines
        .into_iter()
        .filter(|meta| {
            glob.as_deref()
                .map(|glob| {
                    crate::platform::services::project::template_glob_matches(
                        glob,
                        &meta.file_rel_path,
                    )
                })
                .unwrap_or(true)
        })
        .filter(|meta| {
            status_filter
                .as_deref()
                .map(|wanted| pipeline_status(meta).eq_ignore_ascii_case(wanted))
                .unwrap_or(true)
        })
        .filter(|meta| {
            trigger_filter
                .as_deref()
                .map(|wanted| normalize_trigger_filter(&meta.trigger_kind) == wanted)
                .unwrap_or(true)
        })
        .filter_map(|meta| {
            let trigger_summary = if include_trigger_summary || query.is_some() {
                pipeline_trigger_summary(ops, &meta)
            } else {
                meta.trigger_kind.clone()
            };
            let matches_query = query
                .as_deref()
                .map(|needle| {
                    [
                        meta.file_rel_path.as_str(),
                        meta.name.as_str(),
                        meta.title.as_str(),
                        meta.description.as_str(),
                        meta.trigger_kind.as_str(),
                        trigger_summary.as_str(),
                    ]
                    .iter()
                    .any(|value| value.to_lowercase().contains(needle))
                })
                .unwrap_or(true);
            matches_query.then_some((meta, trigger_summary))
        })
        .collect()
}

fn filter_template_rows(
    ops: &PlatformOps,
    items: Vec<TemplateTreeItem>,
    options: &FileListOptions<'_>,
) -> Vec<(TemplateTreeItem, TemplateSemanticMeta)> {
    let query = options
        .query
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase);
    let kind_filter = options
        .kind
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("all"))
        .map(str::to_lowercase);

    items
        .into_iter()
        .filter(|item| item.kind != "folder")
        .filter(|item| {
            options
                .glob
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|glob| {
                    crate::platform::services::project::template_glob_matches(glob, &item.rel_path)
                })
                .unwrap_or(true)
        })
        .filter(|item| {
            kind_filter
                .as_deref()
                .map(|wanted| item.file_kind.eq_ignore_ascii_case(wanted))
                .unwrap_or(true)
        })
        .filter_map(|item| {
            let meta = read_template_semantic_meta(ops, &item.rel_path);
            let keyword_text = meta.keywords.join(" ");
            let matches_query = query
                .as_deref()
                .map(|needle| {
                    [
                        item.rel_path.as_str(),
                        item.name.as_str(),
                        item.file_kind.as_str(),
                        meta.title.as_str(),
                        meta.description.as_str(),
                        keyword_text.as_str(),
                    ]
                    .iter()
                    .any(|value| value.to_lowercase().contains(needle))
                })
                .unwrap_or(true);
            matches_query.then_some((item, meta))
        })
        .collect()
}

/// `active` serves the working tree; `stale` serves an older snapshot because
/// the file changed after activation (register or patch) and nobody promoted
/// it; `draft` serves nothing. An agent that patched a live pipeline sees
/// `stale` and knows the next step is `pipeline_activate`.
pub(crate) fn pipeline_status(meta: &PipelineMeta) -> &'static str {
    match meta.active_hash.as_deref() {
        Some(active) if active == meta.hash => "active",
        Some(_) => "stale",
        None => "draft",
    }
}

fn pipeline_purpose(meta: &PipelineMeta) -> &str {
    let description = meta.description.trim();
    if !description.is_empty() {
        return description;
    }
    meta.title.trim()
}

fn normalize_trigger_filter(value: &str) -> String {
    let lower = value.trim().to_lowercase();
    lower
        .strip_prefix("n.trigger.")
        .or_else(|| lower.strip_prefix("trigger."))
        .unwrap_or(&lower)
        .to_string()
}

fn pipeline_trigger_summary(ops: &PlatformOps, meta: &PipelineMeta) -> String {
    let fallback = if meta.trigger_kind.trim().is_empty() {
        "n.trigger.unknown".to_string()
    } else if meta.trigger_kind.starts_with("n.trigger.") {
        meta.trigger_kind.clone()
    } else {
        format!("n.trigger.{}", meta.trigger_kind)
    };

    let source = match ops.platform.projects.read_pipeline_source(
        &ops.owner,
        &ops.project,
        &meta.file_rel_path,
    ) {
        Ok(source) => source,
        Err(_) => return fallback,
    };
    let graph = match decode_pipeline_graph(source.as_bytes()) {
        Ok(document) => document.spec,
        Err(_) => return fallback,
    };
    let Some(node) = graph
        .nodes
        .iter()
        .find(|node| node.kind.starts_with("n.trigger."))
    else {
        return fallback;
    };

    match node.kind.as_str() {
        "n.trigger.webhook" => {
            let method = node
                .config
                .get("method")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("GET");
            let path = node
                .config
                .get("path")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("/");
            format!("n.trigger.webhook {} {}", method.to_uppercase(), path)
        }
        "n.trigger.schedule" => {
            let cron = node
                .config
                .get("cron")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("* * * * *");
            format!("n.trigger.schedule {cron}")
        }
        "n.trigger.memsubscribe" => {
            let channel = node
                .config
                .get("channel")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("*");
            format!("n.trigger.memsubscribe {channel}")
        }
        "n.trigger.mcp" => {
            let tool_name = node
                .config
                .get("tool_name")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(&meta.name);
            format!("n.trigger.mcp {tool_name}")
        }
        "n.trigger.function" => format!("n.trigger.function {}", meta.name),
        other => other.to_string(),
    }
}

fn read_template_semantic_meta(ops: &PlatformOps, rel_path: &str) -> TemplateSemanticMeta {
    let Ok(content) = ops
        .platform
        .projects
        .read_repo_file_text(&ops.owner, &ops.project, rel_path)
    else {
        return TemplateSemanticMeta::default();
    };
    parse_template_semantic_meta(&content)
}

fn parse_template_semantic_meta(content: &str) -> TemplateSemanticMeta {
    let trimmed = content.trim_start();
    let Some(after_open) = trimmed.strip_prefix("/*") else {
        return TemplateSemanticMeta::default();
    };
    let Some(end) = after_open.find("*/") else {
        return TemplateSemanticMeta::default();
    };
    let block = &after_open[..end];
    let mut in_zebflow = false;
    let mut in_keywords = false;
    let mut meta = TemplateSemanticMeta::default();

    for raw_line in block.lines() {
        let line = raw_line.trim().trim_start_matches('*').trim();
        if line == "zebflow:" {
            in_zebflow = true;
            in_keywords = false;
            continue;
        }
        if !in_zebflow || line.is_empty() {
            continue;
        }
        if let Some(value) = line.strip_prefix("title:") {
            meta.title = unquote_meta_value(value.trim());
            in_keywords = false;
        } else if let Some(value) = line.strip_prefix("description:") {
            meta.description = unquote_meta_value(value.trim());
            in_keywords = false;
        } else if line == "keywords:" {
            in_keywords = true;
        } else if in_keywords && line.starts_with('-') {
            meta.keywords
                .push(unquote_meta_value(line.trim_start_matches('-').trim()));
        } else if !line.starts_with(' ') && !line.starts_with('-') {
            in_keywords = false;
        }
    }

    meta
}

fn unquote_meta_value(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

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

// ── Skills ───────────────────────────────────────────────────────────────────

impl PlatformOps {
    /// Tier 1: every skill this project's agent sees, name and description.
    pub fn skill_list(&self) -> OpsResult {
        let layout = match self.platform.projects.project_layout(&self.owner, &self.project) {
            Ok(layout) => layout,
            Err(e) => return OpsResult::err(e.to_string()),
        };
        let skills = crate::platform::skills::list(&layout.repo_source_dir());
        if skills.is_empty() {
            return OpsResult::ok("No skills. A project adds one at skills/<name>/SKILL.md.".to_string());
        }
        let mut out = String::from(
            "Skills — read one with `skill_read name=\"<name>\"` when its trigger matches your task; \
             a reference file beside it with `skill_read name=\"<name>\" path=\"references/<file>\"`.\n\n",
        );
        out.push_str(&crate::platform::skills::render_listing(&skills));
        OpsResult::ok(out)
    }

    /// Tier 2 and 3: the `SKILL.md` body, or a file beside it.
    pub fn skill_read(&self, name: &str, path: Option<&str>) -> OpsResult {
        let layout = match self.platform.projects.project_layout(&self.owner, &self.project) {
            Ok(layout) => layout,
            Err(e) => return OpsResult::err(e.to_string()),
        };
        let path = path.unwrap_or("");
        match crate::platform::skills::read(&layout.repo_source_dir(), name, path) {
            Some((source, text)) => {
                let where_ = match source {
                    crate::platform::skills::SkillSource::Project => format!("skills/{name}/"),
                    crate::platform::skills::SkillSource::Blessed => "blessed".to_string(),
                };
                let file = if path.is_empty() { "SKILL.md" } else { path };
                OpsResult::ok(format!("<!-- skill {name} · {file} · {where_} -->\n{text}"))
            }
            None => {
                let known = crate::platform::skills::list(&layout.repo_source_dir())
                    .into_iter()
                    .map(|s| s.name)
                    .collect::<Vec<_>>()
                    .join(", ");
                OpsResult::err(format!(
                    "No skill '{name}'{}. Known skills: {known}. `skill_list` shows them with descriptions.",
                    if path.is_empty() { String::new() } else { format!(" file '{path}'") }
                ))
            }
        }
    }
}

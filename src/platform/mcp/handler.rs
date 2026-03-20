//! Zebflow MCP handler exposing project-scoped tools.

use std::sync::Arc;

use axum::http;
use rmcp::handler::server::{ServerHandler, tool::Extension};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ServerCapabilities, ServerInfo};
use rmcp::schemars::JsonSchema;
use rmcp::{ErrorData as McpError, schemars, tool, tool_handler, tool_router};
use serde_json::json;

use crate::platform::model::{
    DescribeProjectDbConnectionRequest, McpSession, ProjectAccessSubject, TemplateCreateKind,
    TemplateCreateRequest, TemplateSaveRequest, mcp_tool_capability,
};
use crate::platform::services::PlatformService;

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineGetParams {
    /// File-relative path of the pipeline (e.g. "pipelines/my-pipeline.zf.json").
    file_rel_path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct TemplateGetParams {
    /// Relative path to the template file (e.g. "pages/home.tsx").
    rel_path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct DocsProjectReadParams {
    /// Relative path to the doc file under repo/docs (e.g. "README.md").
    path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct SkillReadParams {
    /// Skill name to read (e.g. "pipeline-authoring", "rwe-templates").
    name: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct TemplateWriteParams {
    /// Relative path under templates/ (e.g. "pages/blog-home.tsx", "components/ui/card.tsx").
    rel_path: String,
    /// Full file content to write.
    content: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct TemplateCreateParams {
    /// Kind of entry to create: "page", "component", "script", or "folder".
    kind: String,
    /// Base name for the file or folder (e.g. "blog-home", "user-card").
    name: String,
    /// Optional parent folder path under templates/ (e.g. "components/ui").
    parent_rel_path: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct DocsProjectWriteParams {
    /// Relative path under repo/docs/ (e.g. "README.md", "architecture.md", "erd.md").
    path: String,
    /// Full file content to write.
    content: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct DocsAgentReadParams {
    /// Agent doc name: "AGENTS.md", "SOUL.md", or "MEMORY.md".
    name: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct DocsAgentWriteParams {
    /// Agent doc name: "AGENTS.md" (project instructions), "SOUL.md" (agent personality),
    /// or "MEMORY.md" (persistent memory across sessions).
    name: String,
    /// Full file content to write.
    content: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineRegisterParams {
    /// File-relative path of the pipeline under repo/ (e.g. "pipelines/api/blog-home").
    /// The .zf.json extension is added automatically if omitted.
    /// Preferred over name+path. If omitted, derived from name and path fields.
    #[serde(default)]
    file_rel_path: Option<String>,
    /// Pipeline name slug (e.g. "blog-home", "process-order"). Used when file_rel_path is not set.
    #[serde(default)]
    name: Option<String>,
    /// Virtual path for grouping (e.g. "/pages", "/api", "/jobs"). Defaults to "/". Used when file_rel_path is not set.
    #[serde(default)]
    path: Option<String>,
    /// Optional human-readable display title.
    title: Option<String>,
    /// Pipeline body: pipe-chained nodes starting with |.
    /// Example: "| trigger.webhook --path /blog --method GET | pg.query --credential main-db -- \"SELECT * FROM posts\""
    /// Use help_pipeline or skill_read with name "pipeline-dsl" for the full node catalog and syntax.
    body: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineDescribeParams {
    /// File-relative path of the pipeline (e.g. "pipelines/api/blog-home.zf.json").
    /// Also accepted as "name" for backward compatibility.
    #[serde(alias = "name")]
    file_rel_path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelinePatchParams {
    /// File-relative path of the pipeline (e.g. "pipelines/api/blog-home.zf.json").
    /// Also accepted as "name" for backward compatibility.
    #[serde(alias = "name")]
    file_rel_path: String,
    /// Node ID to patch — get IDs from pipeline_describe output (e.g. "n0", "b", "trigger").
    node_id: String,
    /// Space-separated --flag value pairs to update in the node config.
    /// Example: "--credential new-db --path /updated"
    flags: Option<String>,
    /// Body content for the node (SQL for pg.query, JS source for script nodes).
    body: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineActivateParams {
    /// File-relative path of the pipeline to activate (e.g. "pipelines/api/blog-home.zf.json").
    /// Also accepted as "name" for backward compatibility.
    #[serde(alias = "name")]
    file_rel_path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineDeactivateParams {
    /// File-relative path of the pipeline to deactivate (e.g. "pipelines/api/blog-home.zf.json").
    /// Also accepted as "name" for backward compatibility.
    #[serde(alias = "name")]
    file_rel_path: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineExecuteParams {
    /// File-relative path of the registered active pipeline to execute (e.g. "pipelines/api/blog-home.zf.json").
    /// Also accepted as "name" for backward compatibility.
    #[serde(alias = "name")]
    file_rel_path: String,
    /// Optional JSON input payload string (e.g. "{\"order_id\": 42}").
    input: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct PipelineRunParams {
    /// Pipe-chained node body to execute inline — NOT saved, NOT logged.
    /// Starts with | followed by nodes: "| pg.query --credential main-db -- \"SELECT count(*) FROM users\""
    /// Auto-prepends trigger.manual if no trigger node is specified.
    /// Use this for testing queries, one-off scripts, or data exploration.
    body: String,
}

#[derive(serde::Deserialize, JsonSchema)]
struct GitCommandParams {
    /// Git subcommand: status, log, diff, add, commit
    subcommand: String,
    /// Additional arguments as a space-separated string (e.g. "path/to/file" for add/diff, "--limit 10" for log).
    args: Option<String>,
    /// Commit message — only used when subcommand is "commit".
    message: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct ConnectionDescribeParams {
    /// Connection slug — get slugs from connection_list (e.g. "main-db", "default").
    slug: String,
    /// Scope to inspect: "tables", "schemas", "functions", or omit for full tree.
    scope: Option<String>,
    /// Filter to a specific schema name (e.g. "public"). Only meaningful with scope="tables".
    schema: Option<String>,
    /// Filter to a specific table for column-level detail.
    /// Format: "schema.table" (e.g. "academic.staff") or just "table" for public schema.
    /// Use scope="tables" first to discover table names, then table=<name> for columns.
    table: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct HelpExamplesParams {
    /// Archetype slug to load (e.g. "blog-with-admin", "forum-with-chat").
    /// Omit to list all available archetypes.
    slug: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct HelpNodesParams {
    /// Node kind to look up (e.g. "n.sekejap.query", "script", "trigger.webhook").
    /// Omit to return the full node catalog.
    kind: Option<String>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct HelpSearchParams {
    /// Search query — node name, DSL flag, concept, or keyword.
    query: String,
}

/// Zebflow MCP handler with project-scoped tools.
///
/// Sessions are injected via HTTP request extensions by the middleware layer.
/// Tools access the session via `Extension<http::request::Parts>` and extract
/// the `McpSession` from `parts.extensions`.
#[derive(Clone)]
pub struct ZebflowMcpHandler {
    platform: Arc<PlatformService>,
    tool_router: rmcp::handler::server::tool::ToolRouter<Self>,
}

#[tool_router]
impl ZebflowMcpHandler {
    pub fn new(platform: Arc<PlatformService>) -> Self {
        Self {
            platform,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Call this first. Returns Zebflow platform overview, project name, \
        project docs list, AGENTS.md content (if exists), DB connections, template tree, \
        and which help tools to call next. Your orientation before building anything.")]
    async fn start_here(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;

        let mut sections: Vec<String> = Vec::new();

        // Static overview
        sections.push(format!(
            "# Zebflow Platform Overview\n\
             Project: {}/{}\n\n\
             Zebflow is a pipeline-based reactive web platform.\n\
             - **Pipelines**: chain of nodes that handle HTTP, WebSocket, or schedule triggers\n\
             - **RWE Engine**: compiles TSX templates server-side, hydrates client-side\n\
             - **Nodes**: trigger.webhook, pg.query, sekejap.query, script, web.render, ws.emit, ...\n\
             - **Credentials**: stored secrets referenced by slug in pipeline --credential flags\n\
             - **DB Connections**: named database connections (postgres, sekejap) with schema inspect",
            session.owner, session.project
        ));

        // Project git repo section
        {
            let git_section = match self.platform.file.ensure_project_layout(&session.owner, &session.project) {
                Ok(layout) => {
                    // Run git status --short inside repo_dir
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
                        git_status.lines().map(|l| format!("  {}", l)).collect::<Vec<_>>().join("\n")
                    };

                    // Check if a remote is configured
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
                         {}\
                         **Current status:**\n\
                         {}",
                        remote_warning,
                        status_block
                    )
                }
                Err(_) => "## Project Git Repo\n(could not read repo layout)".to_string(),
            };
            sections.push(git_section);
        }

        // Project docs list
        match self.platform.projects.list_project_docs(&session.owner, &session.project) {
            Ok(docs) if !docs.is_empty() => {
                sections.push(format!(
                    "## Project Docs (repo/docs/)\n{}",
                    docs.iter().map(|d| format!("- {}", d.path)).collect::<Vec<_>>().join("\n")
                ));
            }
            Ok(_) => {
                sections.push("## Project Docs\n(none yet — use docs_project_write to create)".to_string());
            }
            Err(e) => {
                sections.push(format!("## Project Docs\n(error: {})", e));
            }
        }

        // AGENTS.md content if it exists
        match self.platform.projects.read_agent_doc(&session.owner, &session.project, "AGENTS.md") {
            Ok(content) => {
                sections.push(format!("## AGENTS.md (Project Instructions)\n{}", content));
            }
            Err(_) => {
                sections.push("## AGENTS.md\n(not found — create with docs_agent_write name=AGENTS.md)".to_string());
            }
        }

        // DB connections
        match self.platform.db_connections.list_project_connections(&session.owner, &session.project) {
            Ok(items) if !items.is_empty() => {
                let lines: Vec<String> = items.iter().map(|c| {
                    format!("- {} ({}) — kind: {}", c.connection_slug, c.connection_label, c.database_kind)
                }).collect();

                // Check if db_schema.md already exists in project docs
                let has_schema_doc = self.platform.projects
                    .list_project_docs(&session.owner, &session.project)
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
                    "## DB Connections\n{}\n\n{}",
                    lines.join("\n"),
                    schema_hint
                ));
            }
            Ok(_) => {
                sections.push("## DB Connections\n(none configured)".to_string());
            }
            Err(e) => {
                sections.push(format!("## DB Connections\n(error: {})", e));
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
        match self.platform.projects.list_template_workspace(&session.owner, &session.project) {
            Ok(workspace) => {
                sections.push(format!(
                    "## Template Tree\n{}",
                    serde_json::to_string_pretty(&workspace).unwrap_or_else(|_| "(parse error)".to_string())
                ));
            }
            Err(e) => {
                sections.push(format!("## Templates\n(error: {})", e));
            }
        }

        // Agent orientation — interview direction
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

        // Help pointers — with dynamic examples index
        let examples_index = crate::platform::skills::all_examples()
            .iter()
            .map(|e| format!("  - `{}` — {}", e.slug, e.title))
            .collect::<Vec<_>>()
            .join("\n");

        sections.push(format!(
            "## Next: Help Tools\n\
             - `help_pipeline` — DSL syntax, pipe mode, web patterns (GET page, POST API, auth gate, redirect)\n\
             - `help_rwe` — TSX templates, SSR, useState, passing pipeline data via input object\n\
             - `help_examples slug=<slug>` — full DSL recipe. Available slugs:\n{}\n\
             - `help_nodes` — node catalog (all kinds, flags, input/output)\n\
             - `help_search query=<term>` — search across all skill docs\n\
             - `skill_list` — list all reference skill documents\n\
             - `pipeline_list` — see existing pipelines\n\
             - `docs_agent_read name=MEMORY.md` — read previous session notes",
            examples_index
        ));

        Ok(CallToolResult::success(vec![Content::text(sections.join("\n\n---\n\n"))]))
    }

    #[tool(description = "Pipeline system guide — DSL syntax, pipe mode, how to register \
        and activate, common fullstack web patterns (GET page, POST API, auth gate, redirect, cron). \
        Call this before writing your first pipeline.")]
    async fn help_pipeline(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "help_pipeline")?;

        match crate::platform::skills::get_skill("help-pipeline") {
            Some(skill) => Ok(CallToolResult::success(vec![Content::text(skill.content)])),
            None => Err(McpError::internal_error("help-pipeline skill not found", None)),
        }
    }

    #[tool(description = "Reactive Web Engine guide — TSX templates, SSR + hydration, \
        passing pipeline data into templates via the input object, useState, hooks, \
        n.web.render node. Call this before writing TSX templates.")]
    async fn help_rwe(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "help_rwe")?;

        let mut content = String::new();
        if let Some(skill) = crate::platform::skills::get_skill("rwe-templates") {
            content.push_str(skill.content);
        }
        content.push_str("\n\n---\n\n");
        if let Some(skill) = crate::platform::skills::get_skill("pipeline-dsl-rwe") {
            content.push_str(skill.content);
        }

        Ok(CallToolResult::success(vec![Content::text(content)]))
    }

    #[tool(description = "Project archetypes with full DSL recipes. Without slug: lists 6 available \
        archetypes (blog, forum, game, scheduling, scraping, auth). With slug: returns full pipeline \
        architecture, DSL bodies, and nodes used for that archetype.")]
    async fn help_examples(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<HelpExamplesParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "help_examples")?;

        match params.slug {
            None => {
                let examples = crate::platform::skills::all_examples();
                let listing = examples
                    .iter()
                    .map(|e| format!("- **{}** (`{}`) — {}", e.title, e.slug, e.description))
                    .collect::<Vec<_>>()
                    .join("\n");
                let text = format!(
                    "# Available Archetypes\n\nCall `help_examples slug=<slug>` to load full DSL recipe.\n\n{}",
                    listing
                );
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            Some(slug) => match crate::platform::skills::get_example(&slug) {
                Some(example) => Ok(CallToolResult::success(vec![Content::text(example.content)])),
                None => {
                    let available: Vec<&str> = crate::platform::skills::all_examples()
                        .iter()
                        .map(|e| e.slug)
                        .collect();
                    Err(McpError::invalid_params(
                        format!(
                            "Archetype '{}' not found. Available: {}",
                            slug,
                            available.join(", ")
                        ),
                        None,
                    ))
                }
            },
        }
    }

    #[tool(description = "Node reference catalog. Without kind: returns the full node catalog \
        with all node types, flags, and examples. With kind: returns the section for that specific \
        node (e.g. kind='script', kind='n.sekejap.query', kind='trigger.webhook').")]
    async fn help_nodes(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<HelpNodesParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "help_nodes")?;

        let catalog = match crate::platform::skills::get_skill("pipeline-nodes") {
            Some(s) => s.content,
            None => return Err(McpError::internal_error("pipeline-nodes skill not found", None)),
        };

        match params.kind {
            None => Ok(CallToolResult::success(vec![Content::text(catalog)])),
            Some(kind) => {
                // Search for the section matching this node kind in the catalog
                let kind_lower = kind.to_lowercase();
                let normalized = kind_lower.trim_start_matches("n.").trim_start_matches("n_");
                let mut result = String::new();
                let mut in_section = false;
                let mut found = false;

                for line in catalog.lines() {
                    if line.starts_with("### ") {
                        let header = line[4..].to_lowercase();
                        let matches = header.contains(normalized)
                            || header.contains(&kind_lower)
                            || normalized.split('.').any(|part| header.contains(part));
                        if in_section && found {
                            break;
                        }
                        in_section = matches;
                        if matches {
                            found = true;
                        }
                    }
                    if in_section {
                        result.push_str(line);
                        result.push('\n');
                    }
                }

                if found {
                    Ok(CallToolResult::success(vec![Content::text(result)]))
                } else {
                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "Node '{}' not found in catalog. Returning full catalog:\n\n{}",
                        kind, catalog
                    ))]))
                }
            }
        }
    }

    #[tool(description = "Search Zebflow docs. Returns matching chunks from pipeline docs, \
        RWE docs, node catalog, and skill files. Use for any concept, node name, DSL flag, \
        or syntax question. Example: query='jwt', query='sekejap upsert', query='web.render'.")]
    async fn help_search(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<HelpSearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "help_search")?;

        let query_lower = params.query.to_lowercase();
        let terms: Vec<&str> = query_lower.split_whitespace().collect();
        let skills = crate::platform::skills::all_skills();

        let mut results: Vec<String> = Vec::new();
        let context_lines = 5usize;
        let max_chars = 3000usize;
        let mut total_chars = 0usize;

        'outer: for skill in skills {
            let lines: Vec<&str> = skill.content.lines().collect();
            let mut i = 0;
            while i < lines.len() {
                let line_lower = lines[i].to_lowercase();
                let matches = terms.iter().all(|t| line_lower.contains(t));
                if matches {
                    let start = i.saturating_sub(context_lines);
                    let end = (i + context_lines + 1).min(lines.len());
                    let chunk = lines[start..end].join("\n");
                    let header = format!("### Match in `{}` ({})\n", skill.name, skill.title);
                    let entry = format!("{}{}\n", header, chunk);
                    total_chars += entry.len();
                    results.push(entry);
                    i = end;
                    if total_chars >= max_chars {
                        break 'outer;
                    }
                } else {
                    i += 1;
                }
            }
        }

        if results.is_empty() {
            Ok(CallToolResult::success(vec![Content::text(format!(
                "No results for '{}'. Try different terms or use `skill_list` to browse all skills.",
                params.query
            ))]))
        } else {
            Ok(CallToolResult::success(vec![Content::text(results.join("\n---\n\n"))]))
        }
    }

    #[tool(description = "List all pipelines in the project")]
    async fn pipeline_list(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_list")?;

        match self
            .platform
            .projects
            .list_pipeline_meta_rows(&session.owner, &session.project)
        {
            Ok(pipelines) => {
                let content = json!({
                    "pipelines": pipelines,
                    "count": pipelines.len()
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&content).unwrap(),
                )]))
            }
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(description = "List all templates in the project workspace")]
    async fn template_list(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "template_list")?;

        match self
            .platform
            .projects
            .list_template_workspace(&session.owner, &session.project)
        {
            Ok(workspace) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&workspace).unwrap(),
            )])),
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(description = "Get a specific pipeline by file-relative path")]
    async fn pipeline_get(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineGetParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_get")?;

        let meta = self
            .platform
            .projects
            .get_pipeline_meta_by_file_id(&session.owner, &session.project, &params.file_rel_path)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let Some(meta) = meta else {
            return Err(McpError::invalid_params(
                format!("Pipeline '{}' not found", params.file_rel_path),
                None,
            ));
        };
        let source = self
            .platform
            .projects
            .read_pipeline_source(&session.owner, &session.project, &meta.file_rel_path)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let content = json!({ "meta": meta, "source": source });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&content).unwrap(),
        )]))
    }

    #[tool(description = "Get a specific template by relative path")]
    async fn template_get(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<TemplateGetParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "template_get")?;

        match self
            .platform
            .projects
            .read_template_file(&session.owner, &session.project, &params.rel_path)
        {
            Ok(content) => Ok(CallToolResult::success(vec![Content::text(content)])),
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(
        description = "Create a new template file with scaffolding. \
                       Kind must be one of: page (pages/*.tsx), component (components/*.tsx), \
                       script (scripts/*.ts), folder. \
                       Returns the scaffolded content — use template_write to customise it after."
    )]
    async fn template_create(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<TemplateCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "template_create")?;

        let kind = match params.kind.as_str() {
            "page" => TemplateCreateKind::Page,
            "component" => TemplateCreateKind::Component,
            "script" => TemplateCreateKind::Script,
            "folder" => TemplateCreateKind::Folder,
            other => return Err(McpError::invalid_params(
                format!("Invalid kind '{}'. Must be: page, component, script, folder", other),
                None,
            )),
        };

        let req = TemplateCreateRequest {
            kind,
            name: params.name,
            parent_rel_path: params.parent_rel_path,
        };

        match self.platform.projects.create_template_entry(&session.owner, &session.project, &req) {
            Ok(payload) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&payload).unwrap(),
            )])),
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(
        description = "Write (create or overwrite) a template file. \
                       Use template_create first to scaffold with boilerplate, then template_write to fill in content. \
                       Path is relative to templates/ (e.g. 'pages/blog-home.tsx', 'components/ui/card.tsx'). \
                       Use help_rwe or skill_read 'rwe-templates' for TSX conventions before writing."
    )]
    async fn template_write(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<TemplateWriteParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "template_write")?;

        let req = TemplateSaveRequest {
            rel_path: params.rel_path,
            content: params.content,
        };

        match self.platform.projects.write_template_file(&session.owner, &session.project, &req) {
            Ok(payload) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&payload).unwrap(),
            )])),
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(
        description = "Write (create or update) a project doc file under repo/docs/. \
                       Use for specs, architecture docs, ERDs, API contracts, README, CHANGELOG. \
                       These files are git-synced. Always commit after writing with git_command."
    )]
    async fn docs_project_write(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<DocsProjectWriteParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "docs_project_write")?;

        match self.platform.projects.upsert_project_doc(
            &session.owner,
            &session.project,
            &params.path,
            &params.content,
        ) {
            Ok(doc) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&doc).unwrap(),
            )])),
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(
        description = "List the three agent doc files: AGENTS.md (project instructions for agents), \
                       SOUL.md (agent personality and behavior config), MEMORY.md (persistent memory across sessions). \
                       Always read AGENTS.md first when starting work on a project."
    )]
    async fn docs_agent_list(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "docs_agent_list")?;

        match self.platform.projects.list_agent_docs(&session.owner, &session.project) {
            Ok(docs) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&docs).unwrap(),
            )])),
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(
        description = "Read one agent doc: AGENTS.md (project instructions), \
                       SOUL.md (agent personality), or MEMORY.md (persistent memory). \
                       Read AGENTS.md at the start of every session to understand the project."
    )]
    async fn docs_agent_read(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<DocsAgentReadParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "docs_agent_read")?;

        match self.platform.projects.read_agent_doc(&session.owner, &session.project, &params.name) {
            Ok(content) => Ok(CallToolResult::success(vec![Content::text(content)])),
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(
        description = "Write an agent doc. AGENTS.md: project-specific instructions for all agents. \
                       SOUL.md: agent personality, tone, and behavioral config. \
                       MEMORY.md: persistent notes the agent writes to remember things across sessions. \
                       Agents should update MEMORY.md after completing significant work."
    )]
    async fn docs_agent_write(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<DocsAgentWriteParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "docs_agent_write")?;

        match self.platform.projects.upsert_agent_doc(
            &session.owner,
            &session.project,
            &params.name,
            &params.content,
        ) {
            Ok(()) => Ok(CallToolResult::success(vec![Content::text(
                format!("{} written successfully.", params.name),
            )])),
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(description = "List all DB connections for this project — returns slug, label, and kind (postgres, mysql, sekejap). Use the slug with connection_describe and in --credential flags.")]
    async fn connection_list(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "connection_list")?;

        match self
            .platform
            .db_connections
            .list_project_connections(&session.owner, &session.project)
        {
            Ok(items) => {
                let content = json!({
                    "connections": items.iter().map(|c| json!({
                        "slug": c.connection_slug,
                        "label": c.connection_label,
                        "kind": c.database_kind,
                    })).collect::<Vec<_>>(),
                    "count": items.len(),
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&content).unwrap(),
                )]))
            }
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(description = "Describe a DB connection's schema — tables, columns, types, constraints. Use scope='tables' for a quick overview, scope='schemas' to list schemas, or omit scope for the full tree. Always run this before writing SQL queries. Use table='schema.table' (e.g. table='academic.staff') to get column detail for a specific table.")]
    async fn connection_describe(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<ConnectionDescribeParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "connection_describe")?;

        let conn = self
            .platform
            .db_connections
            .get_project_connection(&session.owner, &session.project, &params.slug)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .ok_or_else(|| {
                McpError::invalid_params(
                    format!("Connection '{}' not found", params.slug),
                    None,
                )
            })?;

        let req = DescribeProjectDbConnectionRequest {
            scope: if params.table.is_some() {
                Some("columns".to_string())
            } else {
                params.scope
            },
            schema: params.schema,
            table: params.table,
            include_system: Some(false),
        };

        match self
            .platform
            .db_runtime
            .describe_connection(&session.owner, &session.project, &conn.connection_id, &req)
            .await
        {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(
                format_describe_for_llm(&result),
            )])),
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(description = "List credentials for this project — returns id, title, and kind only. Values are never exposed. Use the id in pipeline nodes that require authentication.")]
    async fn credential_list(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "credential_list")?;

        match self
            .platform
            .credentials
            .list_project_credentials(&session.owner, &session.project)
        {
            Ok(items) => {
                let content = json!({
                    "credentials": items.iter().map(|c| json!({
                        "id": c.credential_id,
                        "title": c.title,
                        "kind": c.kind,
                        "notes": c.notes,
                    })).collect::<Vec<_>>(),
                    "count": items.len(),
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&content).unwrap(),
                )]))
            }
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(description = "List project doc files (ERD, README.md, architecture docs, use cases) under repo/docs")]
    async fn docs_project_list(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "docs_project_list")?;

        match self
            .platform
            .projects
            .list_project_docs(&session.owner, &session.project)
        {
            Ok(docs) => {
                let content = json!({ "docs": docs, "count": docs.len() });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&content).unwrap(),
                )]))
            }
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(description = "Read one project doc by path (e.g. README.md, AGENTS.md, architecture.md)")]
    async fn docs_project_read(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<DocsProjectReadParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "docs_project_read")?;

        match self
            .platform
            .projects
            .read_project_doc(&session.owner, &session.project, &params.path)
        {
            Ok(content) => Ok(CallToolResult::success(vec![Content::text(content)])),
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }

    #[tool(description = "List all available Zebflow platform skills (operational knowledge docs)")]
    async fn skill_list(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "skill_list")?;

        let skills = crate::platform::skills::all_skills();
        let items: Vec<_> = skills
            .iter()
            .map(|s| {
                json!({
                    "name": s.name,
                    "title": s.title,
                    "summary": s.summary(),
                })
            })
            .collect();
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&json!({"skills": items, "count": items.len()})).unwrap(),
        )]))
    }

    #[tool(description = "Read the full content of a Zebflow platform skill by name. \
        Use skill_list to see available skill names. Prefer help_pipeline, help_rwe, help_nodes \
        for guided docs — use skill_read for raw reference material.")]
    async fn skill_read(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<SkillReadParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "skill_read")?;

        match crate::platform::skills::get_skill(&params.name) {
            Some(skill) => Ok(CallToolResult::success(vec![Content::text(skill.content)])),
            None => Err(McpError::invalid_params(
                format!(
                    "Skill '{}' not found. Use skill_list to see available skills.",
                    params.name
                ),
                None,
            )),
        }
    }

    #[tool(
        description = "Register (create or update) a pipeline by name and pipe-chained node body. \
                       Body format: '| trigger.webhook --path /x | pg.query --credential db -- \"SQL\"'. \
                       After registering, call pipeline_activate to make it live. \
                       Use help_pipeline or skill_read with name 'pipeline-dsl' for the full node catalog and syntax."
    )]
    async fn pipeline_register(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineRegisterParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_register")?;

        let file_rel_path = match params.file_rel_path {
            Some(frp) => frp,
            None => {
                let name = params.name.unwrap_or_default();
                let raw_path = params.path.unwrap_or_else(|| "/".to_string());
                let vpath = raw_path.trim_matches('/');
                if vpath.is_empty() {
                    format!("pipelines/{}", name)
                } else {
                    format!("pipelines/{}/{}", vpath, name)
                }
            }
        };
        let mut dsl = format!("register {}", file_rel_path);
        if let Some(title) = &params.title {
            dsl.push_str(&format!(" --title \"{}\"", title));
        }
        dsl.push(' ');
        dsl.push_str(&params.body);

        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &session.owner,
            &session.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        let formatted = output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
        Ok(CallToolResult::success(vec![Content::text(formatted)]))
    }

    #[tool(
        description = "Inspect a pipeline — returns its nodes, edges, status, and hit stats. \
                       Node IDs from this output are required for pipeline_patch."
    )]
    async fn pipeline_describe(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineDescribeParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_describe")?;

        let dsl = format!("describe pipeline {}", params.file_rel_path);
        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &session.owner,
            &session.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        let formatted = output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
        Ok(CallToolResult::success(vec![Content::text(formatted)]))
    }

    #[tool(
        description = "Patch one node in a saved pipeline without rewriting the full graph. \
                       Call pipeline_describe first to get node IDs. \
                       Pipeline status becomes stale after patching — call pipeline_activate to make it live again."
    )]
    async fn pipeline_patch(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelinePatchParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_patch")?;

        let mut dsl = format!("patch pipeline {} node {}", params.file_rel_path, params.node_id);
        if let Some(flags) = &params.flags {
            dsl.push(' ');
            dsl.push_str(flags);
        }
        if let Some(body) = &params.body {
            dsl.push_str(&format!(" -- {}", body));
        }

        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &session.owner,
            &session.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        let formatted = output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
        Ok(CallToolResult::success(vec![Content::text(formatted)]))
    }

    #[tool(
        description = "Activate a pipeline — makes it live so it can serve traffic and be executed. \
                       Must be called after pipeline_register or after patching. \
                       A pipeline must be active before pipeline_execute will run it."
    )]
    async fn pipeline_activate(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineActivateParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_activate")?;

        let dsl = format!("activate pipeline {}", params.file_rel_path);
        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &session.owner,
            &session.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        let formatted = output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
        Ok(CallToolResult::success(vec![Content::text(formatted)]))
    }

    #[tool(
        description = "Deactivate a pipeline — takes it offline. Traffic stops being served. \
                       Pipeline source is retained and can be re-activated later."
    )]
    async fn pipeline_deactivate(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineDeactivateParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_deactivate")?;

        let dsl = format!("deactivate pipeline {}", params.file_rel_path);
        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &session.owner,
            &session.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        let formatted = output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
        Ok(CallToolResult::success(vec![Content::text(formatted)]))
    }

    #[tool(
        description = "Execute a registered active pipeline by name. Records execution hits. \
                       Pipeline must be activated first — use pipeline_activate if status is draft or stale. \
                       Use pipeline_list to see pipeline names and activation status."
    )]
    async fn pipeline_execute(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineExecuteParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_execute")?;

        let mut dsl = format!("execute pipeline {}", params.file_rel_path);
        if let Some(input) = &params.input {
            dsl.push_str(&format!(" --input {}", input));
        }

        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &session.owner,
            &session.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        let formatted = output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
        Ok(CallToolResult::success(vec![Content::text(formatted)]))
    }

    #[tool(
        description = "Run a pipe-chained node body EPHEMERALLY — not saved, not logged, no hit recording. \
                       Use this to test queries, explore data, or prototype before registering. \
                       Example body: '| pg.query --credential main-db -- \"SELECT count(*) FROM users\"'. \
                       Auto-prepends trigger.manual if no trigger node specified."
    )]
    async fn pipeline_run(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<PipelineRunParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "pipeline_run")?;

        let dsl = format!("run {}", params.body);
        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &session.owner,
            &session.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        let formatted = output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
        Ok(CallToolResult::success(vec![Content::text(formatted)]))
    }

    #[tool(
        description = "Run a git command on the project repository. \
                       Allowed subcommands: status, log, diff, add, commit. \
                       Destructive operations (reset, rebase, force, checkout) are blocked. \
                       Always commit after registering or patching pipelines."
    )]
    async fn git_command(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(params): Parameters<GitCommandParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.get_session_from_http_parts(&parts)?;
        self.check_tool_capability(&session, "git_command")?;

        let mut dsl = format!("git {}", params.subcommand);
        if let Some(args) = &params.args {
            dsl.push(' ');
            dsl.push_str(args);
        }
        if let Some(msg) = &params.message {
            dsl.push_str(&format!(" -- {}", msg));
        }

        let executor = crate::platform::shell::executor::DslExecutor::new(
            self.platform.clone(),
            &session.owner,
            &session.project,
        );
        let output = executor.execute_dsl(&dsl).await;
        let formatted = output.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
        Ok(CallToolResult::success(vec![Content::text(formatted)]))
    }

    fn get_session_from_http_parts(
        &self,
        parts: &http::request::Parts,
    ) -> Result<McpSession, McpError> {
        parts
            .extensions
            .get::<McpSession>()
            .cloned()
            .ok_or_else(|| {
                McpError::invalid_params(
                    "No active MCP session; ensure Authorization header is set with valid session token",
                    None,
                )
            })
    }

    fn check_tool_capability(&self, session: &McpSession, tool_name: &str) -> Result<(), McpError> {
        let required_capability = mcp_tool_capability(tool_name).ok_or_else(|| {
            McpError::invalid_params(format!("Unknown tool '{}'", tool_name), None)
        })?;

        let subject = ProjectAccessSubject::mcp_session(&session.token);
        match self.platform.authz.ensure_project_capability(
            &subject,
            &session.owner,
            &session.project,
            required_capability,
        ) {
            Ok(()) => Ok(()),
            Err(_) => Err(McpError::invalid_params(
                format!(
                    "Tool '{}' requires capability '{}' which is not allowed in this session",
                    tool_name,
                    required_capability.key()
                ),
                None,
            )),
        }
    }
}

#[tool_handler]
impl ServerHandler for ZebflowMcpHandler {
    fn get_info(&self) -> ServerInfo {
        let instructions = crate::platform::skills::get_skill("agent-core")
            .map(|s| s.content.to_string())
            .unwrap_or_else(|| {
                "Zebflow project management tools. Call start_here first. \
                 Use help_pipeline, help_rwe, help_examples, help_nodes for guided docs. \
                 Use skill_list then skill_read for reference material."
                    .to_string()
            });
        ServerInfo {
            instructions: Some(instructions.into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

/// Format a DB describe result as compact LLM-readable text.
///
/// Output example:
/// ```
/// # openobe-uinsgd (postgresql) — scope: tables
///
/// academic.academic_period
/// - academic_period_id: uuid, PK, default:uuid_generate_v4()
/// - academic_year: smallint, NOT NULL
/// - unit_id: uuid, NOT NULL, → academic.academic_unit.unit_id
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
                    parts.push(format!("→ {}.{}.{}", rs, rt, rc));
                }

                if let Some(d) = default {
                    // Truncate long defaults (e.g. huge jsonb literals)
                    let truncated = if d.len() > 48 {
                        format!("{}…", &d[..48])
                    } else {
                        d.to_string()
                    };
                    parts.push(format!("default:{}", truncated));
                }

                out.push_str(&format!("- {}: {}\n", name, parts.join(", ")));
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
                            out.push_str(&format!("\nfn {}.{}\n", s, child.name));
                        }
                    }
                }
            }
            "table" => {
                out.push_str(&format_table(node));
            }
            "function" => {
                let s = node.schema.as_deref().unwrap_or("public");
                out.push_str(&format!("\nfn {}.{}\n", s, node.name));
            }
            _ => {}
        }
    }

    out
}

/// Build the MCP service with token validation using rmcp's StreamableHttpService.
pub fn build_mcp_service<S: Clone + Send + Sync + 'static>(
    platform: Arc<PlatformService>,
) -> axum::Router<S> {
    use axum::middleware;
    use rmcp::transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    };
    use tokio_util::sync::CancellationToken;

    let session_manager = Arc::new(LocalSessionManager::default());
    let cancellation_token = CancellationToken::new();

    let config = StreamableHttpServerConfig {
        sse_keep_alive: Some(std::time::Duration::from_secs(30)),
        sse_retry: Some(std::time::Duration::from_secs(5)),
        stateful_mode: false,
        json_response: true,
        cancellation_token: cancellation_token.clone(),
    };

    let platform_for_factory = platform.clone();
    let service = StreamableHttpService::new(
        move || {
            let platform = platform_for_factory.clone();
            let handler = ZebflowMcpHandler::new(platform);
            Ok(handler)
        },
        session_manager,
        config,
    );

    let platform_for_middleware = platform.clone();
    axum::Router::new()
        .route_service("/", service)
        .layer(middleware::from_fn(
            move |mut req: axum::extract::Request, next: middleware::Next| {
                let platform = platform_for_middleware.clone();
                async move {
                    let token = req
                        .headers()
                        .get("authorization")
                        .and_then(|h| h.to_str().ok())
                        .and_then(|s| s.strip_prefix("Bearer "))
                        .unwrap_or("");

                    if !token.is_empty() {
                        if let Some(session) = platform.mcp_sessions.lookup(token) {
                            req.extensions_mut().insert(session);
                        }
                    }

                    next.run(req).await
                }
            },
        ))
}
